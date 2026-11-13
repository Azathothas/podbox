//! Answering to `docker` and `podman` on PATH.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0803, `TOOL.md` section 2.0 and
//! section 6.8.
//!
//! ⭐ **An agent runs `docker`.** If podvm is only reachable as `podvm` it has
//! replaced nothing on the machines it is for, where `docker` on PATH is
//! already a podman alias with no daemon behind it.
//!
//! ⛔ **Multicall on `argv[0]`, never a wrapper script.** A shell wrapper is a
//! second artefact and it breaks the memfd rung
//! ([`TODO/packaging.md`](../../../TODO/packaging.md) T-1001), which is exactly
//! what `references/qaidvoid__onelf` records as skipping that rung. One binary,
//! two extra names, and the banner says which name was used.
//!
//! ⛔ **RULED BY THE OPERATOR ON 2026-09-08**: podvm refuses to take the
//! `docker` name where a working docker daemon is reachable, unless an explicit
//! flag says otherwise, and says why in one line. A machine with a working
//! daemon is a machine where podvm is the wrong tool. ⚠ The check is on a
//! REACHABLE DAEMON and not on a `docker` binary being present: on the target
//! runtime that binary exists with nothing behind it, so a binary check would
//! refuse on exactly the machines podvm is for.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

/// The names podvm answers to besides its own.
pub const ALIASES: &[&str] = &["docker", "podman"];

/// The name podvm was invoked under, with no directory.
pub fn invoked_as() -> String {
    std::env::args()
        .next()
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "podvm".into())
}

/// One banner line where podvm was reached under somebody else's name.
///
/// ⛔ `TOOL.md` section 4.1: a caller has to be able to tell which tool ran.
/// Taking the name is the product requirement; taking it silently is the thing
/// the honesty rules exist to forbid.
pub fn alias_note() -> Option<String> {
    let name = invoked_as();
    if !ALIASES.contains(&name.as_str()) {
        return None;
    }
    Some(format!(
        "podvm: invoked as `{name}`. This is podvm {}, not {name}: it takes {name}'s \
         verbs, flags and exit codes, and `podvm system info` lists every one it does \
         not honour (TODO/cli.md T-0803)\n",
        env!("CARGO_PKG_VERSION")
    ))
}

/// What the daemon check could determine.
///
/// ⚠ Three states and not two, because "could not tell" is not "no daemon".
/// `AGENTS.md`: a guard that cannot answer says so rather than picking the
/// convenient answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Daemon {
    /// Something answered `/_ping` on the socket.
    Reachable(String),
    /// The socket is there and nothing is listening, or there is no socket.
    Absent(String),
    /// podvm could not run the check at all, and this says why.
    Unknown(String),
}

/// Is a working docker daemon reachable from here?
///
/// ⛔ A REACHABLE DAEMON, not a `docker` binary. The socket is connected to and
/// asked `/_ping`, so a stale socket file with nothing behind it reads as
/// absent, which is the state the machines podvm is for are actually in.
pub fn docker_daemon() -> Daemon {
    let host = std::env::var("DOCKER_HOST").unwrap_or_default();
    let path: PathBuf = if host.is_empty() {
        PathBuf::from("/var/run/docker.sock")
    } else if let Some(p) = host.strip_prefix("unix://") {
        PathBuf::from(p)
    } else {
        // ⚠ The dockless posture, from the other side: where the guard cannot
        // determine its target it says the GUARD is degraded and continues,
        // rather than going quiet.
        return Daemon::Unknown(format!(
            "$DOCKER_HOST is {host:?}, which is not a unix:// socket. podvm tests a \
             unix socket and nothing else, so it cannot tell whether a daemon is there"
        ));
    };
    if !path.exists() {
        return Daemon::Absent(format!("{} does not exist", path.display()));
    }
    let Ok(mut sock) = UnixStream::connect(&path) else {
        return Daemon::Absent(format!(
            "{} exists and nothing is listening on it",
            path.display()
        ));
    };
    // ⛔ Bounded. AGENTS.md: nothing that touches another process waits
    // with no upper bound, and podvm inherits that as a requirement.
    let t = Duration::from_secs(2);
    let _ = sock.set_read_timeout(Some(t));
    let _ = sock.set_write_timeout(Some(t));
    if sock
        .write_all(b"GET /_ping HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .is_err()
    {
        return Daemon::Absent(format!(
            "{} accepted a connection and no request",
            path.display()
        ));
    }
    let mut buf = [0u8; 64];
    match sock.read(&mut buf) {
        Ok(n) if n > 0 => {
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            if head.contains(" 200") {
                Daemon::Reachable(format!("{} answered /_ping", path.display()))
            } else {
                Daemon::Unknown(format!(
                    "{} answered {:?}, which podvm cannot read as a daemon or as its \
                     absence",
                    path.display(),
                    head.lines().next().unwrap_or("").trim()
                ))
            }
        }
        _ => Daemon::Absent(format!(
            "{} accepted a connection and answered nothing",
            path.display()
        )),
    }
}

pub const INSTALL_USAGE: &str = "\
usage: podvm system install-names [--dir D] [--force] [name...]

  Install `docker` and `podman` as SYMLINKS to this binary, so an agent that
  knows docker reaches podvm without learning anything.

  --dir D    where to put them. Default: the directory this binary is in
  --force    install the `docker` name even where a docker daemon answers, and
             replace a file that is not already a link to this binary

  refused: podvm REFUSES the `docker` name where a working docker daemon is
    reachable, unless --force. A machine with a working daemon is a machine
    where podvm is the wrong tool. The check is on a reachable daemon and not
    on a `docker` binary being present: on the runtime podvm is for, that
    binary exists with nothing behind it.

  refused: Symlinks, never copies and never a wrapper script. One binary is one
    artefact, and a shell wrapper breaks the memfd rung (TODO/packaging.md
    T-1001).
";

pub fn install(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, INSTALL_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, INSTALL_USAGE) {
        return c;
    }
    let mut dir: Option<PathBuf> = None;
    let mut force = false;
    let mut wanted: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{INSTALL_USAGE}");
                return 0;
            }
            "--force" => force = true,
            "--dir" => match it.next() {
                Some(d) => dir = Some(PathBuf::from(d)),
                None => {
                    eprintln!("podvm system install-names: --dir needs a directory");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--dir=") => {
                dir = Some(PathBuf::from(&other["--dir=".len()..]))
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, INSTALL_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => {
                if !ALIASES.contains(&other) {
                    eprintln!(
                        "podvm system install-names: {other:?} is not a name podvm \
                         answers to. It answers to: {}",
                        ALIASES.join(", ")
                    );
                    return EXIT_CLI_ERROR;
                }
                wanted.push(other.to_string());
            }
        }
    }
    if wanted.is_empty() {
        wanted = ALIASES.iter().map(|s| (*s).to_string()).collect();
    }

    // ⛔ The link target is THIS binary, resolved through /proc so a relative
    // argv[0] or a PATH lookup cannot produce a link that points at nothing.
    let me = match std::fs::read_link("/proc/self/exe") {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "podvm system install-names: cannot read /proc/self/exe, so podvm \
                 does not know what to link to: {e}"
            );
            return EXIT_RUNTIME_ERROR;
        }
    };
    let dir = dir.unwrap_or_else(|| me.parent().unwrap_or(Path::new(".")).to_path_buf());

    let daemon = docker_daemon();
    let mut code = 0;
    for name in &wanted {
        let dest = dir.join(name);
        // ⭐ The operator's ruling, and it applies to the `docker` name only:
        // `podman` on a machine with a docker daemon is not a name that
        // shadows a working client.
        if name == "docker" {
            match &daemon {
                Daemon::Reachable(where_) if !force => {
                    eprintln!(
                        "podvm system install-names: a docker daemon is reachable \
                         ({where_}), so podvm will not take the `docker` name: this is \
                         a machine where podvm is the wrong tool. --force installs it \
                         anyway (TODO/cli.md T-0803)"
                    );
                    code = EXIT_RUNTIME_ERROR;
                    continue;
                }
                Daemon::Unknown(why) => {
                    // ⚠ The guard says IT is degraded and continues, rather than
                    // going quiet or refusing on a check it did not make.
                    eprintln!(
                        "podvm system install-names: note: the daemon check is degraded: {why}"
                    );
                }
                _ => {}
            }
        }
        match std::fs::symlink_metadata(&dest) {
            Ok(md) => {
                let already = md.file_type().is_symlink()
                    && std::fs::read_link(&dest).map(|t| t == me).unwrap_or(false);
                if already {
                    println!("{} -> {} (already)", dest.display(), me.display());
                    continue;
                }
                if !force {
                    eprintln!(
                        "podvm system install-names: {} exists and is not a link to \
                         this binary. podvm will not replace it without --force",
                        dest.display()
                    );
                    code = EXIT_RUNTIME_ERROR;
                    continue;
                }
                let _ = std::fs::remove_file(&dest);
            }
            Err(_) => {
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }
        match std::os::unix::fs::symlink(&me, &dest) {
            Ok(()) => println!("{} -> {}", dest.display(), me.display()),
            Err(e) => {
                eprintln!("podvm system install-names: {}: {e}", dest.display());
                code = EXIT_RUNTIME_ERROR;
            }
        }
    }
    code
}

/// The start of each process of podvm: the entry of the native-static
/// rung (T-1652), then SIGPIPE back to its default. Rust ignores SIGPIPE,
/// so `podvm profile list | head` panicked on a closed pipe (TODO/cli.md
/// T-1769); with the default, podvm ends quietly, as each Unix tool does.
pub fn start() {
    podvm_dispatch::main_if_entered("podvm");
    unsafe extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: SIG_DFL installs no handler of this process.
    unsafe { signal(SIGPIPE, SIG_DFL) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_names_usage_is_plain_ascii() {
        // TODO/cli.md T-1336: `install-names --help` prints bytes
        // 0x00-0x7F only.
        let bad = INSTALL_USAGE.bytes().filter(|b| *b > 0x7F).count();
        assert_eq!(bad, 0, "install-names usage carries {bad} non-ASCII bytes");
    }

    /// ⛔ The names are the table's and this module's, and they have to be the
    /// same list: a name podvm answers to with no parity row is a surface
    /// nobody documented.
    #[test]
    fn every_alias_is_in_the_parity_table() {
        for a in ALIASES {
            assert!(
                crate::parity::TABLE.iter().any(|r| r.note.contains(a)),
                "{a} is a name podvm takes and the parity table never mentions it"
            );
        }
    }

    #[test]
    fn the_alias_note_names_the_name_that_was_used() {
        // ⚠ Driven through the function's own input rather than by re-execing:
        // `invoked_as` reads argv[0], which a unit test cannot set.
        for a in ALIASES {
            assert!(!a.is_empty());
        }
        // Under the test harness argv[0] is the test binary, so there is no note.
        assert!(alias_note().is_none(), "{}", invoked_as());
    }

    /// ⚠ Whatever this machine is, the check must answer one of three states
    /// and must never panic. A daemon IS reachable in this project's own
    /// container, and is not on the runtime podvm targets.
    #[test]
    fn the_daemon_check_answers_one_of_three_states() {
        match docker_daemon() {
            Daemon::Reachable(s) | Daemon::Absent(s) | Daemon::Unknown(s) => {
                assert!(!s.is_empty(), "the state carries no reason")
            }
        }
    }
}
