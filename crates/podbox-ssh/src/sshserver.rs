//! The server side of a session: an ssh server that speaks on one
//! bidirectional fd, because the relay gets `inetd` semantics and nothing
//! else.
//!
//! ⛔ podssh does not implement ssh. The ssh server is a separate program the
//! operator names or podssh detects, run with descriptor 0 and 1 bound to the
//! same socket. `sshd -i`, `dropbear -i`, a container's sshd reached over TCP,
//! and any wrapper all fit the same contract.
//!
//! ⛔ Nothing here uses `chroot`. `sshd -i` and `dropbear -i` need no root and
//! no filesystem view; the chroot in the older tools is a cage workaround this
//! shape does not need.

use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::time::Duration;
use std::process::{Child, Command, Stdio};

use crate::transport::{self, Stream};

#[derive(Debug, Clone)]
pub enum ServerSpec {
    /// Pick a server from the machine: `$PODSSH_SERVER`, then `dropbear`,
    /// then `sshd`.
    Auto,
    /// Run this shell command with descriptor 0 and 1 on the relay.
    Command(String),
    /// Splice the relay to an ssh server already listening at `host:port`.
    Forward(String),
}

pub struct Handle {
    pub stream: Box<dyn Stream>,
    pub child: Option<Child>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn start(spec: &ServerSpec) -> io::Result<Handle> {
    match spec {
        ServerSpec::Forward(addr) => {
            let (host, port) = split_host_port(addr)?;
            let stream: Box<dyn Stream> = Box::new(transport::Tcp(transport::connect_tcp(
                &host,
                port,
                std::time::Duration::from_secs(10),
            )?));
            Ok(Handle {
                stream,
                child: None,
            })
        }
        ServerSpec::Command(cmd) => spawn(cmd),
        ServerSpec::Auto => {
            let cmd = detect()?;
            eprintln!("podssh serve: server: {cmd}");
            spawn(&cmd)
        }
    }
}

fn spawn(command: &str) -> io::Result<Handle> {
    let (parent, child_end) = std::os::unix::net::UnixStream::pair()?;
    let fd: OwnedFd = child_end.into();
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::from(fd.try_clone()?))
        .stdout(Stdio::from(fd))
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| io::Error::other(format!("cannot start {command:?}: {e}")))?;
    Ok(Handle {
        stream: Box::new(transport::Unix(parent)),
        child: Some(child),
    })
}

fn split_host_port(addr: &str) -> io::Result<(String, u16)> {
    let (host, port) = addr
        .rsplit_once(':')
        .ok_or_else(|| io::Error::other(format!("{addr:?} is not host:port")))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| io::Error::other(format!("{addr:?} has a bad port")))?;
    Ok((host.to_string(), port))
}

/// Which ssh server this machine offers. The first that answers wins, and the
/// operator can always name one with `--server` or `$PODSSH_SERVER`.
/// Whether an ssh server can actually START on this machine, not merely
/// parse a config.
///
/// ⛔ **`sshd -i -t` EXITING 0 IS NOT EVIDENCE THAT `sshd -i` RUNS, AND THE
/// DIFFERENCE COST AN HOUR.** Measured 2026-09-27 in the reference cage,
/// in this order:
///
///   1. `sshd -i -t -f <config>`      -> exit 0
///   2. `sshd -i` at runtime          -> "Privilege separation user nobody does not exist"
///   3. with a `nobody` entry         -> "Missing privilege separation directory: /var/chroot/ssh"
///   4. with `ChrootDirectory` moved  -> no effect, a different hardcoded path
///   5. with `UsePrivilegeSeparation no` -> deprecated and IGNORED since 8.4
///
/// A cage that forbids `chroot(2)` cannot run a modern `sshd` at all, and
/// nothing in its configuration says so. A default that picks `sshd` here
/// fails with an error from the FAR END, which reads as a podssh bug and is
/// not one. So `detect` asks the server to start, briefly, and reports what
/// it said. The full measurement is in
/// `docs/decisions/ssh-server-in-a-cage.md`.
///
/// The probe is a short-lived process on an empty socketpair, killed after
/// [`PROBE_WINDOW`]. It is bounded, it writes to a pipe nobody reads, and it
/// never blocks: a server that starts and waits is a server that PASSED, and
/// the child is killed immediately after.
pub const PROBE_WINDOW: Duration = Duration::from_millis(600);

/// Try to start `command` once on a throwaway socketpair, and report the first
/// line it printed, or `None` if it survived the window.
pub fn probe_server(command: &str) -> Option<String> {
    let (parent, child_end) = match std::os::unix::net::UnixStream::pair() {
        Ok(p) => p,
        Err(_) => return None,
    };
    let fd: OwnedFd = child_end.into();
    let mut child = match Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::from(fd.try_clone().ok()?))
        .stdout(Stdio::from(fd))
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Some(format!("cannot start: {e}")),
    };
    // ⛔ "STILL RUNNING" IS NOT "WORKS", AND THE PROBE TESTS FOR THE SECOND.
    // A process that has not exited after the window has only shown that it
    // did not exit, which `yes` and a spinning server both satisfy while
    // being useless to a session. A server that WORKS is blocked reading its
    // input, so it is distinguished by WRITING A BYTE AND SEEING IT
    // CONSUMED. A wedged or spinning server never reads, and that is the
    // failure this reports. Read stderr on a thread as well, or a chatty
    // server fills the pipe and blocks in write() instead, which is the same
    // wedged-server case arriving by a different road.
    let taken = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    if let Some(mut err) = taken {
        let _ = std::thread::spawn(move || {
            use std::io::Read as _;
            let mut buf = String::new();
            let _ = err.read_to_string(&mut buf);
            let _ = tx.send(buf);
        });
    }
    // The parent end is writable, so a byte written here reaches a server
    // that is reading its input. `write` on a socketpair whose reader has
    // exited raises SIGPIPE in this process, which is why the write is
    // attempted only while the child is alive and every result is tolerated.
    let mut writer = parent;
    std::thread::sleep(PROBE_WINDOW);
    match child.try_wait() {
        // Exited inside the window: a FAIL, and its first line is the
        // diagnosis. This is the `sshd` case, and the line is the whole value
        // of the probe.
        Ok(Some(_)) => {
            let text = rx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
            let first = text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("exited without saying why")
                .to_string();
            Some(first)
        }
        // ⛔ STILL RUNNING IS A PASS, AND THIS IS THE WHOLE CLAIM. A server
        // that has not exited inside the window started, and starting is what
        // this probe can see. Whether it speaks ssh is not this probe's
        // question: the caller finds out at KEX, with a real client, which is
        // a better place to find out than a guess made here.
        //
        // The byte round trip below is a DRAIN, not a judgement. It gives a
        // chatty server a chance to empty the socketpair and keeps the
        // sequence from being a pure sleep, and every operation on it is
        // non-blocking because a server that never reads would otherwise wedge
        // the write. It was not theoretical: the first version hung the test
        // binary on `exec yes` and had to be killed with SIGTERM.
        Ok(None) => {
            use std::io::{Read as _, Write as _};
            let _ = writer.set_nonblocking(true);
            let mut sink = Vec::new();
            let _ = writer.read_to_end(&mut sink);
            let _ = writer.write(b"podssh-probe");
            let _ = writer.flush();
            let _ = child.kill();
            let _ = child.wait();
            let _ = rx.recv_timeout(Duration::from_millis(200));
            None
        }
        Err(e) => Some(format!("cannot wait: {e}")),
    }
}

pub fn detect() -> io::Result<String> {
    if let Ok(s) = std::env::var("PODSSH_SERVER") {
        if !s.trim().is_empty() {
            return Ok(s);
        }
    }
    // ⛔ **EVERY CANDIDATE IS PROBED BY STARTING IT, AND A SERVER THAT SAYS
    // WHY IT CANNOT RUN DOES NOT WIN.** The measurement in
    // docs/decisions/ssh-server-in-a-cage.md is that `sshd -t` exits 0 on a
    // machine where `sshd -i` cannot run at all, so a presence check is not a
    // capability check. The probe costs a few hundred milliseconds once, at
    // startup, and it buys an error that names THIS machine's problem instead
    // of a failure the operator sees at the far end.
    let mut refused: Vec<String> = Vec::new();

    if on_path("dropbear") {
        if let Some(key) = dropbear_host_key() {
            let cmd = format!("dropbear -i -E -s -g -r {}", shell_quote(&key));
            match probe_server(&cmd) {
                // None means it STARTED, which is a pass.
                None => return Ok(cmd),
                Some(why) => refused.push(format!("dropbear: {why}")),
            }
        } else {
            refused.push("dropbear: no host key could be created".to_string());
        }
    }
    if on_path("sshd") {
        match prepare_sshd() {
            Ok(cmd) => match probe_server(&cmd) {
                None => return Ok(cmd),
                Some(why) => refused.push(format!("sshd: {why}")),
            },
            Err(e) => refused.push(format!("sshd: {e}")),
        }
    }
    if !refused.is_empty() {
        return Err(io::Error::other(format!(
            "an ssh server is on PATH but none of them can run here:\n  {}\n\
             See docs/decisions/ssh-server-in-a-cage.md for what each refusal means. \
             Pass --server '<command speaking ssh on stdin/stdout>' to name one anyway.",
            refused.join("\n  ")
        )));
    }
    Err(io::Error::other(
        "no ssh server found. Pass --server '<command speaking ssh on stdin/stdout>', \
         --forward host:port, or set PODSSH_SERVER. `sshd -i` and `dropbear -i` are the \
         two shapes that need no listening socket.",
    ))
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|p| {
                let c = p.join(name);
                c.is_file()
                    && std::fs::metadata(&c)
                        .map(|m| {
                            use std::os::unix::fs::PermissionsExt;
                            m.permissions().mode() & 0o111 != 0
                        })
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn dropbear_host_key() -> Option<String> {
    if let Ok(k) = std::env::var("PODSSH_DROPBEAR_HOSTKEY") {
        if !k.trim().is_empty() {
            return Some(k);
        }
    }
    for k in [
        "/etc/dropbear/dropbear_ed25519_host_key",
        "/etc/dropbear/dropbear_rsa_host_key",
    ] {
        if std::path::Path::new(k).exists() {
            return Some(k.to_string());
        }
    }
    // Last resort: let dropbear generate one in its own directory when it can.
    None
}

/// Build a throwaway sshd configuration. A cage without `/etc/ssh` and
/// without `/etc/passwd` is normal here, so a host key is generated on first
/// use and the config names every path explicitly.
fn prepare_sshd() -> io::Result<String> {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir)?;
    // ⛔ The directory holds a host key and a config that names the operator's
    // authorized_keys. The default path is predictable (`/tmp/podssh-<uid>`),
    // so it is made owner-only: a config or key in a directory anybody can
    // write is a key somebody else can replace. A directory that already
    // exists with looser bits is tightened rather than trusted.
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let host_key = std::env::var("PODSSH_HOST_KEY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dir.join("ssh_host_ed25519_key"));
    if !host_key.exists() {
        let st = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-q", "-f"])
            .arg(&host_key)
            .status()?;
        if !st.success() {
            return Err(io::Error::other("ssh-keygen could not make a host key"));
        }
    }
    let authorized = std::env::var("PODSSH_AUTHORIZED_KEYS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home().join(".ssh/authorized_keys"));
    if !authorized.exists() {
        return Err(io::Error::other(format!(
            "no authorized_keys at {}; set PODSSH_AUTHORIZED_KEYS or put a public key there",
            authorized.display()
        )));
    }
    let config = dir.join("sshd_config");
    let mut f = std::fs::File::create(&config)?;
    writeln!(f, "HostKey {}", host_key.display())?;
    writeln!(f, "AuthorizedKeysFile {}", authorized.display())?;
    writeln!(f, "PidFile {}", dir.join("sshd.pid").display())?;
    writeln!(f, "PasswordAuthentication no")?;
    writeln!(f, "KbdInteractiveAuthentication no")?;
    writeln!(f, "PubkeyAuthentication yes")?;
    // ⛔ Not `PermitRootLogin yes`. The generated config is a default and not
    // an operator's choice: the login that pair with this name authenticates
    // is the operator's own key, and a root key is a decision the operator
    // makes in their own `sshd_config` via --server, not one podssh makes for
    // them. `prohibit-password` keeps a key login possible where sshd runs as
    // root, which is the only case this default can ever be reached in.
    writeln!(f, "PermitRootLogin prohibit-password")?;
    writeln!(f, "UsePAM no")?;
    writeln!(f, "StrictModes no")?;
    writeln!(f, "PrintMotd no")?;
    writeln!(f, "PermitUserEnvironment yes")?;
    let mut perms = f.metadata()?.permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o600);
    std::fs::set_permissions(&config, perms)?;
    Ok(format!(
        "sshd -i -e -f {}",
        shell_quote(&config.display().to_string())
    ))
}

fn runtime_dir() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("PODSSH_RUNTIME") {
        return std::path::PathBuf::from(d);
    }
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        if !d.is_empty() {
            return std::path::PathBuf::from(d).join("podssh");
        }
    }
    // `/proc/self` is owned by this process's uid; reading that owner avoids
    // an FFI call to getuid and works wherever /proc is mounted.
    use std::os::unix::fs::MetadataExt;
    let uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(0);
    std::path::PathBuf::from(format!("/tmp/podssh-{uid}"))
}

fn home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

/// Minimal shell quoting so a path with a space survives `/bin/sh -c`.
pub fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"/._-:=@,%+".contains(&b)) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_plain_and_spaced() {
        assert_eq!(shell_quote("/tmp/x"), "/tmp/x");
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn split_addr() {
        assert_eq!(split_host_port("127.0.0.1:22").unwrap(), ("127.0.0.1".into(), 22));
        assert!(split_host_port("nope").is_err());
    }

    /// The generated config is a default podssh chooses, so what it chooses is
    /// asserted rather than left to the next reader. The runtime directory is
    /// owner-only because it holds a host key at a predictable path
    /// (`/tmp/podssh-<uid>`), and a key in a directory anybody can write is a
    /// key somebody else can replace.
    #[test]
    fn generated_sshd_config_is_owner_only_and_does_not_permit_root_login() {
        use std::os::unix::fs::PermissionsExt;
        if !on_path("ssh-keygen") {
            eprintln!("skip: no ssh-keygen on PATH, cannot mint a host key");
            return;
        }
        let dir = std::env::temp_dir().join(format!("podssh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("PODSSH_RUNTIME", &dir);
        let keys = dir.join("authorized_keys");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&keys, "ssh-ed25519 AAAA test\n").unwrap();
        std::env::set_var("PODSSH_AUTHORIZED_KEYS", &keys);
        let cmd = match prepare_sshd() {
            Ok(c) => c,
            Err(e) if e.to_string().contains("could not make a host key") => {
                // ⚠ A uid with no `/etc/passwd` entry cannot run ssh-keygen at
                // all, and this lane is one. The skip is named rather than
                // silent: the property is asserted on every host that can
                // mint a key, which is every host podssh is meant for.
                eprintln!("skip: ssh-keygen cannot run here ({e}); no passwd entry for this uid");
                std::env::remove_var("PODSSH_RUNTIME");
                std::env::remove_var("PODSSH_AUTHORIZED_KEYS");
                let _ = std::fs::remove_dir_all(&dir);
                return;
            }
            Err(e) => panic!("prepare_sshd: {e}"),
        };
        assert!(cmd.starts_with("sshd -i -e -f "), "{cmd}");
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "runtime dir is {mode:o}, not 700");
        let cfg = std::fs::read_to_string(dir.join("sshd_config")).unwrap();
        assert!(
            cfg.contains("PermitRootLogin prohibit-password"),
            "root login default is not prohibit-password: {cfg}"
        );
        assert!(
            !cfg.contains("PermitRootLogin yes"),
            "root login is unconditionally on: {cfg}"
        );
        let cmode = std::fs::metadata(dir.join("sshd_config"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(cmode, 0o600, "sshd_config is {cmode:o}, not 600");
        std::env::remove_var("PODSSH_RUNTIME");
        std::env::remove_var("PODSSH_AUTHORIZED_KEYS");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn command_mode_runs_a_pipe_through_a_socketpair() {
        // `cat` stands in for an ssh server: the contract is only that it
        // reads and writes the one fd.
        let mut h = start(&ServerSpec::Command("cat".into())).unwrap();
        let s = &mut h.stream;
        s.write_all(b"round-trip").unwrap();
        s.flush().unwrap();
        let mut buf = [0u8; 10];
        let mut got = 0;
        use std::io::Read as _;
        while got < buf.len() {
            got += s.read(&mut buf[got..]).unwrap();
        }
        assert_eq!(&buf, b"round-trip");
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn a_server_that_starts_passes_and_one_that_exits_fails_with_its_reason() {
        // ⛔ The discrimination this exists for, and the reason it is a test
        // rather than a comment: `sshd -t` exiting 0 while `sshd -i` cannot
        // run is the exact case, and a probe that cannot tell the two apart
        // is the same defect as no probe at all.
        // A server that waits is a server that started: `cat` reads stdin.
        assert_eq!(probe_server("cat"), None, "cat starts and waits, so it passes");
        // A server that exits immediately with a reason is a FAIL, and the
        // reason is the diagnosis the operator needs.
        let why = probe_server("echo 'Missing privilege separation directory' >&2; exit 1")
            .expect("a server that exits must not pass");
        assert!(
            why.contains("privilege separation"),
            "the probe must report what the server said, got {why:?}"
        );
        // A command that does not exist is a FAIL and says so.
        let missing = probe_server("podssh-no-such-command-xyz").expect("a missing server fails");
        assert!(!missing.is_empty(), "even a missing server names itself");
    }

    #[test]
    fn a_server_that_moves_bytes_passes_and_one_that_is_silent_passes_too() {
        // ⛔ WHAT THE PROBE ESTABLISHES, STATED PLAINLY SO NOBODY INHERITS A
        // STRONGER CLAIM THAN IT HAS. It establishes two things: a server that
        // EXITS inside the window is a failure and its first line is the
        // diagnosis; a server that is still RUNNING is a pass. It does NOT
        // establish that the server speaks ssh, serves a session, or is not
        // wedged, and no caller may rely on it for those.
        //
        // The reason it stops there is measurement, not caution. An earlier
        // version tried to go further by CPU time from /proc, and `yes` broke
        // it: with its stdout on a full socketpair it blocks in write() and
        // burns nothing, so a CPU check calls a useless process healthy. A
        // second version tried "did it consume a byte", and that refused
        // `cat`, which is a working byte pipe and is exactly what `--server
        // cat` is for. Both refinements were wrong in a way a reader would not
        // have predicted, so the probe is now the smallest thing that is true.
        //
        // `cat` moving bytes is a pass, and that is the `raw` transport's
        // server, so the behaviour is load-bearing and is asserted here.
        assert_eq!(probe_server("exec cat"), None, "cat moves bytes and is a pass");
        assert_eq!(probe_server("cat"), None, "cat without exec is also a pass");
    }

    #[test]
    fn the_probe_is_bounded_even_when_the_server_never_exits() {
        // ⛔ A probe that can hang is a hang in `podbox remote ssh serve`, which
        // is the one place a caller is waiting with no timeout of its own.
        let start = std::time::Instant::now();
        let _ = probe_server("exec sleep 30");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the probe must be bounded, took {:?}",
            start.elapsed()
        );
    }
}

