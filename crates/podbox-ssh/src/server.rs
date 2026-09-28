//! The server side of a session: an SSH server that speaks on one
//! bidirectional descriptor.
//!
//! ⛔ This crate does not implement SSH. The server is a separate program the
//! operator names or this module detects, run with descriptors 0 and 1 bound
//! to the same socket. `sshd -i` and `dropbear -i` both fit that contract.
//!
//! ⛔ Detection starts the server. A presence check is not a capability
//! check: a binary on `PATH` that cannot run here must not win. The probe
//! costs one window per candidate, at startup, and buys an error that names
//! this machine's problem instead of a failure the operator sees later.

use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{Error, Kind};
use crate::transport;

/// How long the probe gives a candidate server to exit. A server that exits
/// inside this window failed, and its first stderr line is the diagnosis. A
/// server that is still running passed.
pub const PROBE_WINDOW: Duration = Duration::from_millis(600);

/// A server child with the parent end of its socketpair. Dropping it kills
/// the child: a server outlives no handle to it.
pub struct ServerChild {
    pub stream: Box<dyn transport::Stream>,
    child: Option<std::process::Child>,
}

impl Drop for ServerChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Run `command` with descriptors 0 and 1 on one end of a Unix socketpair
/// and return the other end. The command runs through `/bin/sh -c`, so a
/// full server line with flags fits in one string.
pub fn spawn_stdio(command: &str) -> Result<ServerChild, Error> {
    let (parent, child_end) = std::os::unix::net::UnixStream::pair()
        .map_err(|e| Error::server("spawn server socketpair", e))?;
    let fd: OwnedFd = child_end.into();
    let stdin_fd: OwnedFd = fd
        .try_clone()
        .map_err(|e| Error::server("spawn server stdio", e))?;
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::from(stdin_fd))
        .stdout(Stdio::from(fd))
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            Error::server(
                "spawn server",
                io::Error::other(format!("cannot start {command:?}: {e}")),
            )
        })?;
    Ok(ServerChild {
        stream: Box::new(transport::Unix(parent)),
        child: Some(child),
    })
}

/// Try to start `command` once on a throwaway socketpair, and report the
/// first line it printed, or `None` if it survived the window.
///
/// ⛔ Still running is a pass, and that is the whole claim. A server that has
/// not exited inside the window started, and starting is what this probe can
/// see. Whether it speaks SSH is not this probe's question: the caller finds
/// out at key exchange, with a real client, which is a better place to find
/// out than a guess made here.
pub fn probe_server(command: &str) -> Option<String> {
    // ⛔ The parent end stays open for the whole window. A child whose peer
    // closed reads EOF instead of running, which would fail a live server.
    let (_parent, child_end) = std::os::unix::net::UnixStream::pair().ok()?;
    let fd: OwnedFd = child_end.into();
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::from(fd.try_clone().ok()?))
        .stdout(Stdio::from(fd))
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // Read stderr on a thread: a chatty server that fills the pipe would
    // otherwise block in write instead of running, which reads as a pass for
    // a server that said no.
    let taken = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    if let Some(mut err) = taken {
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = err.read_to_string(&mut buf);
            let _ = tx.send(buf);
        });
    }
    std::thread::sleep(PROBE_WINDOW);
    match child.try_wait() {
        // Exited inside the window: a failure, and its first line is the
        // diagnosis.
        Ok(Some(_)) => {
            let text = rx
                .recv_timeout(Duration::from_millis(500))
                .unwrap_or_default();
            let first = text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("exited without saying why")
                .to_string();
            Some(first)
        }
        // Still running: a pass. Kill it; it was only ever a probe.
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = rx.recv_timeout(Duration::from_millis(200));
            None
        }
        Err(e) => Some(format!("cannot wait: {e}")),
    }
}

/// Which SSH server this machine runs. `sshd` first, then `dropbear`: the
/// generated config below targets `sshd`, so it is tried first. Every
/// candidate is probed by starting it, and a server that says why it cannot
/// run does not win.
pub fn detect() -> Result<String, Error> {
    // ⛔ The operator's explicit choice always wins. Detection never
    // overrides a named command.
    if let Ok(s) = std::env::var("PODSSH_SERVER") {
        if !s.trim().is_empty() {
            return Ok(s);
        }
    }
    let mut refused: Vec<String> = Vec::new();
    if on_path("sshd") {
        match prepare_sshd() {
            Ok(cmd) => match probe_server(&cmd) {
                None => return Ok(cmd),
                Some(why) => refused.push(format!("sshd: {why}")),
            },
            Err(e) => refused.push(format!("sshd: {e}")),
        }
    } else {
        refused.push("sshd: not on PATH".to_string());
    }
    if on_path("dropbear") {
        match dropbear_command() {
            Some(cmd) => match probe_server(&cmd) {
                None => return Ok(cmd),
                Some(why) => refused.push(format!("dropbear: {why}")),
            },
            None => refused.push("dropbear: no host key could be created".to_string()),
        }
    } else {
        refused.push("dropbear: not on PATH".to_string());
    }
    Err(Error::new(
        Kind::Server,
        "detect server",
        format!(
            "no SSH server on PATH can run here:\n  {}\nSet PODSSH_SERVER to a command that speaks SSH on stdin/stdout.",
            refused.join("\n  ")
        ),
    ))
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|p| {
                let c = p.join(name);
                c.is_file()
                    && std::fs::metadata(&c)
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn dropbear_command() -> Option<String> {
    if let Ok(k) = std::env::var("PODSSH_DROPBEAR_HOSTKEY") {
        if !k.trim().is_empty() {
            return Some(format!("dropbear -i -E -s -g -r {}", shell_quote(&k)));
        }
    }
    for k in [
        "/etc/dropbear/dropbear_ed25519_host_key",
        "/etc/dropbear/dropbear_rsa_host_key",
    ] {
        if Path::new(k).exists() {
            return Some(format!("dropbear -i -E -s -g -r {}", shell_quote(k)));
        }
    }
    None
}

/// Build a throwaway `sshd` line: runtime dir, host key, generated config.
/// A machine without `/etc/ssh` is normal here, so every path is explicit.
fn prepare_sshd() -> Result<String, Error> {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).map_err(|e| Error::server("sshd runtime dir", e))?;
    // ⛔ The directory holds a host key at a predictable path, so it is
    // owner-only. A key in a directory anybody can write is a key somebody
    // else can replace. A directory that already exists with looser bits is
    // tightened rather than trusted.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| Error::server("sshd runtime dir", e))?;
    let host_key = std::env::var("PODSSH_HOST_KEY")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dir.join("ssh_host_ed25519_key"));
    if !host_key.exists() {
        let st = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-q", "-f"])
            .arg(&host_key)
            .status()
            .map_err(|e| Error::server("sshd host key", e))?;
        if !st.success() {
            return Err(Error::new(
                Kind::Server,
                "sshd host key",
                "ssh-keygen could not make a host key",
            ));
        }
    }
    let authorized = std::env::var("PODSSH_AUTHORIZED_KEYS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".ssh/authorized_keys"));
    if !authorized.exists() {
        return Err(Error::new(
            Kind::Server,
            "sshd authorized_keys",
            format!(
                "no authorized_keys at {}; set PODSSH_AUTHORIZED_KEYS or put a public key there",
                authorized.display()
            ),
        ));
    }
    let config = write_sshd_config(&dir, &host_key, &authorized)?;
    Ok(format!(
        "sshd -i -e -f {}",
        shell_quote(&config.display().to_string())
    ))
}

/// Write the generated `sshd` config into `dir` and return its path.
///
/// ⛔ This config is a default this crate chooses, not the operator's
/// choice. It authenticates by key only, and it never permits a root
/// password login: a root key is a decision the operator makes in their own
/// config, not one this crate makes for them.
pub fn write_sshd_config(
    dir: &Path,
    host_key: &Path,
    authorized_keys: &Path,
) -> Result<PathBuf, Error> {
    let config = dir.join("sshd_config");
    let mut f = std::fs::File::create(&config).map_err(|e| Error::server("sshd config", e))?;
    writeln!(f, "HostKey {}", host_key.display())
        .and_then(|_| writeln!(f, "AuthorizedKeysFile {}", authorized_keys.display()))
        .and_then(|_| writeln!(f, "PidFile {}", dir.join("sshd.pid").display()))
        .and_then(|_| writeln!(f, "PasswordAuthentication no"))
        .and_then(|_| writeln!(f, "KbdInteractiveAuthentication no"))
        .and_then(|_| writeln!(f, "PubkeyAuthentication yes"))
        .and_then(|_| writeln!(f, "PermitRootLogin prohibit-password"))
        .and_then(|_| writeln!(f, "UsePAM no"))
        // StrictModes stays off: the runtime directory and the authorized
        // keys file may live where the cage cannot tighten them, and a
        // server that refuses over modes answers a different question than
        // the one this config asks.
        .and_then(|_| writeln!(f, "StrictModes no"))
        .and_then(|_| writeln!(f, "PrintMotd no"))
        // The environment stays the server's own: accepting client-sent
        // variables would let a caller reach past the command.
        .and_then(|_| writeln!(f, "PermitUserEnvironment no"))
        .map_err(|e| Error::server("sshd config", e))?;
    // ⛔ The config names a private key path. It is owner-only like the
    // directory around it.
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| Error::server("sshd config", e))?;
    Ok(config)
}

fn runtime_dir() -> PathBuf {
    if let Ok(d) = std::env::var("PODSSH_RUNTIME") {
        if !d.trim().is_empty() {
            return PathBuf::from(d);
        }
    }
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        if !d.trim().is_empty() {
            return PathBuf::from(d).join("podssh");
        }
    }
    PathBuf::from(format!("/tmp/podbox-ssh-{}", process_uid()))
}

fn process_uid() -> u32 {
    // `/proc/self` is owned by this process's uid; reading that owner
    // avoids a libc call and works wherever /proc is mounted.
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(0)
}

fn home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Minimal shell quoting so a path with a space survives `/bin/sh -c`.
fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-:=@,%+".contains(&b))
    {
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
    fn explicit_server_command_wins_over_detection() {
        std::env::set_var("PODSSH_SERVER", "cat");
        let got = detect().unwrap();
        std::env::remove_var("PODSSH_SERVER");
        assert_eq!(got, "cat");
    }

    #[test]
    fn generated_config_is_owner_only_and_refuses_root_passwords() {
        // ⛔ The generated config is a default this crate chooses, so what
        // it chooses is asserted rather than left to the next reader.
        let dir = std::env::temp_dir().join(format!("podbox-ssh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let keys = dir.join("authorized_keys");
        std::fs::write(&keys, "ssh-ed25519 AAAA test\n").unwrap();
        let key = dir.join("ssh_host_ed25519_key");
        std::fs::write(&key, "fake-key\n").unwrap();
        let config = write_sshd_config(&dir, &key, &keys).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "runtime dir is {mode:o}, not 700");
        let cfg = std::fs::read_to_string(&config).unwrap();
        assert!(cfg.contains("PermitRootLogin prohibit-password"), "{cfg}");
        assert!(cfg.contains("PasswordAuthentication no"), "{cfg}");
        assert!(cfg.contains("PubkeyAuthentication yes"), "{cfg}");
        assert!(cfg.contains("PermitUserEnvironment no"), "{cfg}");
        assert!(!cfg.contains("PermitRootLogin yes"), "{cfg}");
        let cmode = std::fs::metadata(&config).unwrap().permissions().mode() & 0o777;
        assert_eq!(cmode, 0o600, "sshd_config is {cmode:o}, not 600");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_stdio_carries_bytes_through_cat() {
        // `cat` stands in for an SSH server: the contract is only that it
        // reads and writes the one descriptor.
        let mut h = spawn_stdio("cat").unwrap();
        let s = &mut h.stream;
        s.write_all(b"round-trip").unwrap();
        s.flush().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut buf = [0u8; 10];
        let mut got = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while got < buf.len() && std::time::Instant::now() < deadline {
            match s.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if crate::pump::is_would_block(&e) => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("read: {e}"),
            }
        }
        assert_eq!(&buf, b"round-trip");
    }

    #[test]
    fn a_server_that_starts_passes_and_one_that_exits_fails_with_its_reason() {
        // ⛔ The discrimination this exists for: a server that waits started,
        // and a server that exits did not. A probe that cannot tell the two
        // apart is the same defect as no probe at all.
        assert_eq!(
            probe_server("cat"),
            None,
            "cat starts and waits, so it passes"
        );
        let why = probe_server("echo 'Missing privilege separation directory' >&2; exit 1")
            .expect("a server that exits must not pass");
        assert!(why.contains("privilege separation"), "got {why:?}");
        let missing =
            probe_server("podbox-ssh-no-such-command-xyz").expect("a missing server fails");
        assert!(!missing.is_empty(), "even a missing server names itself");
    }

    #[test]
    fn the_probe_is_bounded_even_when_the_server_never_exits() {
        // ⛔ A probe that can hang is a hang in every caller, and no caller
        // carries a timeout of its own.
        let start = std::time::Instant::now();
        let _ = probe_server("sleep 30");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the probe must be bounded, took {:?}",
            start.elapsed()
        );
    }
}
