//! End to end: a real `sshd -i` on an accepted Unix socket, driven by a
//! real `ssh` client through the proxy binary as `ProxyCommand`.
//!
//! ⛔ If `sshd`, `ssh`, or `ssh-keygen` is absent the test fails loud and
//! names the missing binary. A skip would read as green on a machine that
//! proved nothing.
//!
//! ⛔ Every wait here is bounded. The listener polls with a deadline, the
//! client carries `ConnectTimeout`, and the child runner kills what it
//! cannot wait for.

use std::io::Read;
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ACCEPT_DEADLINE: Duration = Duration::from_secs(20);
const CLIENT_DEADLINE: Duration = Duration::from_secs(40);

/// Find `name` on `PATH`, or panic naming it. A missing server binary is a
/// failed test, never a skipped one.
fn require_binary(name: &str) -> PathBuf {
    let found = std::env::var_os("PATH").map(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join(name))
            .find(|c| {
                c.is_file()
                    && std::fs::metadata(c)
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
            })
    });
    match found {
        Some(Some(p)) => p,
        _ => panic!("e2e needs {name:?} on PATH and it is absent; cannot prove the SSH path"),
    }
}

struct Fixture {
    tmp: PathBuf,
    sock: PathBuf,
    config: PathBuf,
    user: String,
    user_key: PathBuf,
}

fn setup() -> Fixture {
    require_binary("sshd");
    require_binary("ssh");
    let keygen = require_binary("ssh-keygen");
    // ⛔ Tests in one binary run in parallel threads. The directory carries
    // a counter beside the pid so two tests never share keys or a socket.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("podbox-ssh-e2e-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // ⛔ The directory holds host and user keys, so it is owner-only.
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();

    // Debian's sshd demands the privilege separation directory even in
    // inetd mode. Creating the standard path is what the server package
    // does; where it cannot be created the server fails loud below.
    let _ = std::fs::create_dir_all("/run/sshd");
    let host_key = tmp.join("host_key");
    run_keygen(&keygen, &host_key);
    let user_key = tmp.join("user_key");
    run_keygen(&keygen, &user_key);
    let pubkey = std::fs::read_to_string(tmp.join("user_key.pub")).unwrap();
    let authorized = tmp.join("authorized_keys");
    std::fs::write(&authorized, pubkey).unwrap();

    let config = podbox_ssh::write_sshd_config(&tmp, &host_key, &authorized).unwrap();
    let user = current_user();
    Fixture {
        sock: tmp.join("ssh.sock"),
        tmp,
        config,
        user,
        user_key,
    }
}

fn run_keygen(keygen: &Path, out: &Path) {
    let st = Command::new(keygen)
        .args(["-t", "ed25519", "-N", "", "-q", "-f"])
        .arg(out)
        .status()
        .unwrap();
    assert!(st.success(), "ssh-keygen could not mint {}", out.display());
}

fn current_user() -> String {
    if let Ok(u) = std::env::var("USER") {
        if !u.trim().is_empty() {
            return u;
        }
    }
    let out = Command::new("id").arg("-un").output().unwrap();
    assert!(out.status.success(), "cannot learn the current user");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Start one `sshd -i` on the accepted end of `listener`. Returns the server
/// thread and the sender that stops it. Accept and shutdown are both
/// bounded: the thread always returns.
fn start_server(
    listener: UnixListener,
    config: PathBuf,
) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Sender<()>) {
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + ACCEPT_DEADLINE;
        let accepted = loop {
            match listener.accept() {
                Ok((s, _)) => break Some(s),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => panic!("accept failed: {e}"),
            }
        };
        let accepted = accepted.expect("no client connected inside the accept deadline");
        let fd: OwnedFd = accepted.into();
        let stdin_fd: OwnedFd = fd.try_clone().unwrap();
        let mut child = Command::new("sshd")
            .args(["-i", "-e", "-f"])
            .arg(&config)
            .stdin(Stdio::from(stdin_fd))
            .stdout(Stdio::from(fd))
            .spawn()
            .unwrap();
        // The client session runs, then the test sends `done`. The wait for
        // it is bounded so a silent client cannot wedge the suite.
        let _ = done_rx.recv_timeout(CLIENT_DEADLINE);
        let _ = child.kill();
        let _ = child.wait();
    });
    (handle, done_tx)
}

/// Run `ssh` with the proxy as `ProxyCommand` and the given remote command.
/// Bounded: the client carries `ConnectTimeout`, and the runner kills what
/// outlives the deadline.
fn run_client(fx: &Fixture, remote: &[&str]) -> std::process::Output {
    let proxy = env!("CARGO_BIN_EXE_proxy");
    let proxy_cmd = format!("'{proxy}' unix '{}'", fx.sock.display());
    let mut cmd = Command::new("ssh");
    cmd.args(["-o", &format!("ProxyCommand={proxy_cmd}")])
        .args(["-o", "BatchMode=yes"])
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-o", "UserKnownHostsFile=/dev/null"])
        .args(["-o", "ConnectTimeout=10"])
        .args(["-o", "LogLevel=ERROR"])
        .args(["-i"])
        .arg(&fx.user_key)
        .args(["-l", &fx.user, "localhost"])
        .args(remote)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + CLIENT_DEADLINE;
    loop {
        match child.try_wait().unwrap() {
            Some(status) => {
                let mut out = Vec::new();
                let mut err = Vec::new();
                child.stdout.take().unwrap().read_to_end(&mut out).unwrap();
                child.stderr.take().unwrap().read_to_end(&mut err).unwrap();
                return std::process::Output {
                    status,
                    stdout: out,
                    stderr: err,
                };
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("ssh client outlived the client deadline");
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn drive(remote: &[&str]) -> std::process::Output {
    let fx = setup();
    let listener = UnixListener::bind(&fx.sock).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (server, done) = start_server(listener, fx.config.clone());
    let out = run_client(&fx, remote);
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&fx.tmp);
    out
}

#[test]
fn ssh_runs_a_command_through_proxy_and_socketpair() {
    // ⛔ The assertion this pins: exact stdout bytes, empty stderr, exit 0.
    // A byte more or less is a failure, not a pass with a note.
    let out = drive(&["printf", "hello-podbox-ssh"]);
    assert_eq!(
        out.stdout, b"hello-podbox-ssh",
        "stdout was {:?}, stderr was {:?}",
        out.stdout, out.stderr
    );
    assert!(out.stderr.is_empty(), "stderr was {:?}", out.stderr);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn remote_exit_code_passes_through() {
    // ⛔ The assertion this pins: a failing remote command reports its own
    // exit code through the proxy, not 0 and not the proxy's own code.
    let out = drive(&["exit", "42"]);
    assert_eq!(
        out.status.code(),
        Some(42),
        "status was {:?}, stderr was {:?}",
        out.status,
        out.stderr
    );
}

#[test]
fn missing_binary_fails_loud_by_name() {
    // ⛔ The guarantee this pins: the failure names the binary. A message
    // that says only "setup failed" sends the next reader hunting.
    let missing = "podbox-ssh-no-such-binary-xyz";
    let result = std::panic::catch_unwind(|| require_binary(missing));
    let err = result.unwrap_err();
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains(missing),
        "failure did not name the binary: {msg:?}"
    );
}
