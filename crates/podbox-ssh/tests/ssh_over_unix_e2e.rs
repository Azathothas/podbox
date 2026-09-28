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

mod common;

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{
    current_user, ensure_privsep, fresh_tmp, make_keys, require_binary, run_ssh, CLIENT_DEADLINE,
};

const ACCEPT_DEADLINE: Duration = Duration::from_secs(20);

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
    let tmp = fresh_tmp("podbox-ssh-e2e");
    ensure_privsep();
    let keys = make_keys(&keygen, &tmp);
    let config = podbox_ssh::write_sshd_config(&tmp, &keys.host, &keys.authorized).unwrap();
    Fixture {
        sock: tmp.join("ssh.sock"),
        tmp,
        config,
        user: current_user(),
        user_key: keys.user,
    }
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

fn drive(remote: &[&str]) -> std::process::Output {
    let fx = setup();
    let listener = UnixListener::bind(&fx.sock).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (server, done) = start_server(listener, fx.config.clone());
    let proxy = env!("CARGO_BIN_EXE_proxy");
    let proxy_cmd = format!("'{proxy}' unix '{}'", fx.sock.display());
    let out = run_ssh(&proxy_cmd, &fx.user, &fx.user_key, remote, None);
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
