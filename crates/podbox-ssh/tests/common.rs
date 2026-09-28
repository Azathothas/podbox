//! Shared fixtures for the podbox-ssh integration tests: key material, the
//! generated sshd config inputs, and a bounded ssh runner.
//!
//! Two suites share this module: `ssh_over_unix_e2e` (unix socket through
//! the proxy) and `mux_two_client` (the multiplexed relay through node and
//! operator binaries). Helpers used by only one suite carry the fact on
//! themselves rather than growing suite-specific branches here.
//!
//! ⛔ A missing binary fails loud and names the binary. A skip would read as
//! green on a machine that proved nothing.
//!
//! ⛔ Every wait here is bounded. The client carries `ConnectTimeout`, the
//! runner kills what outlives its deadline, and stdin writers run on their
//! own thread so a full pipe cannot wedge the test.

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How long one ssh client may run before the runner kills it.
pub const CLIENT_DEADLINE: Duration = Duration::from_secs(60);

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Find `name` on `PATH`, or panic naming it.
pub fn require_binary(name: &str) -> PathBuf {
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

/// A fresh owner-only directory, unique across the parallel tests in one
/// binary: the counter beside the pid means two tests never share keys or a
/// socket.
pub fn fresh_tmp(prefix: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("{prefix}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
    tmp
}

pub struct Keys {
    /// The host key. Read by the unix-socket e2e; the relay tests mint
    /// their own server through detection, hence the allow.
    #[allow(dead_code)]
    pub host: PathBuf,
    pub user: PathBuf,
    pub authorized: PathBuf,
}

/// Mint a host key, a user key, and the authorized_keys trusting the user
/// key. Panics naming the keygen binary's failure.
pub fn make_keys(keygen: &Path, tmp: &Path) -> Keys {
    let host = tmp.join("host_key");
    let user = tmp.join("user_key");
    for out in [&host, &user] {
        let st = Command::new(keygen)
            .args(["-t", "ed25519", "-N", "", "-q", "-f"])
            .arg(out)
            .status()
            .unwrap();
        assert!(st.success(), "ssh-keygen could not mint {}", out.display());
    }
    let pubkey = std::fs::read_to_string(tmp.join("user_key.pub")).unwrap();
    let authorized = tmp.join("authorized_keys");
    std::fs::write(&authorized, pubkey).unwrap();
    Keys {
        host,
        user,
        authorized,
    }
}

pub fn current_user() -> String {
    if let Ok(u) = std::env::var("USER") {
        if !u.trim().is_empty() {
            return u;
        }
    }
    let out = Command::new("id").arg("-un").output().unwrap();
    assert!(out.status.success(), "cannot learn the current user");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Debian's sshd demands the privilege separation directory even in inetd
/// mode. Creating the standard path is what the server package does; where
/// it cannot be created the server fails loud at startup.
pub fn ensure_privsep() {
    let _ = std::fs::create_dir_all("/run/sshd");
}

/// Run `ssh` with `proxy_cmd` as `ProxyCommand` and the given remote
/// command. Stdin carries `stdin_data` when present, written on its own
/// thread so a full pipe in either direction cannot wedge the runner.
/// Bounded: the client carries `ConnectTimeout`, and the runner kills what
/// outlives the deadline instead of hanging the suite.
pub fn run_ssh(
    proxy_cmd: &str,
    user: &str,
    user_key: &Path,
    remote: &[&str],
    stdin_data: Option<Vec<u8>>,
) -> Output {
    let mut cmd = Command::new("ssh");
    cmd.args(["-o", &format!("ProxyCommand={proxy_cmd}")])
        .args(["-o", "BatchMode=yes"])
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-o", "UserKnownHostsFile=/dev/null"])
        .args(["-o", "ConnectTimeout=10"])
        .args(["-o", "LogLevel=ERROR"])
        .args(["-i"])
        .arg(user_key)
        .args(["-l", user, "localhost"])
        .args(remote)
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    if let Some(data) = stdin_data {
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
            // Dropping closes stdin: the remote `cat` sees EOF and finishes.
        });
    }
    collect(child, CLIENT_DEADLINE)
}

/// Wait for `child` until the deadline and return everything it wrote. A
/// child that outlives its deadline is killed and fails the test by name.
///
/// ⛔ Both pipes drain on their own threads from spawn: a child whose
/// output exceeds the pipe buffer would otherwise block writing it while
/// this runner waits for the exit that never comes. That exact deadlock
/// wedged the first 200 KiB relay transfer for the full deadline.
pub fn collect(mut child: Child, deadline: Duration) -> Output {
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out_handle = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let err_handle = std::thread::spawn(move || {
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        err
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().unwrap() {
            Some(status) => break status,
            None => {
                if start.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("child outlived the {:?} deadline", deadline);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    };
    // The pipes hit EOF when the child dies, so both readers always finish.
    Output {
        status,
        stdout: out_handle.join().unwrap(),
        stderr: err_handle.join().unwrap(),
    }
}

/// Wait until `cond` holds or the deadline passes. For waits on a
/// condition, never on a guessed duration. Shared module: the relay tests
/// use this, the unix e2e does not, hence the allow.
#[allow(dead_code)]
pub fn wait_for<T>(what: &str, deadline: Duration, mut cond: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(v) = cond() {
            return v;
        }
        if start.elapsed() >= deadline {
            panic!("timed out waiting for {what}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
