//! `podbox remote`: the verbs that act across a machine boundary.
//! `TODO/podssh.md` T-1404.
//!
//! `remote` is a namespace and not a synonym for `ssh`, decided by the
//! operator on 2026-09-27 and recorded in `docs/decisions/remote-verb.md`:
//! `remote fetch`, `remote download` and `remote wget` are all "reach a
//! machine that is not this one" and each would otherwise want its own
//! top-level verb. `local` is the counterpart that makes `remote` mean
//! something, and `machine ssh` is a different axis (a guest podbox itself
//! runs) and stays separate.
//!
//! The one member today is `ssh`, brought from podbox pull request 67
//! (`66b6fa10`) onto the transport `main` already proves: `serve`
//! registers this machine with the multiplexed relay and runs the probed
//! SSH server per session, `connect` is the operator side and a valid ssh
//! `ProxyCommand`, and `forward` reaches one fixed target with no relay.
//! The relay and probe members the pull request's help advertised never
//! had arms; this group lists only the arms it has, so no help row stands
//! in for a runnable arm.
//!
//! None of this links the SSH crate: the group resolves the lane-built
//! `node`, `operator` and `proxy` binaries beside this binary or on PATH
//! and runs them with inherited descriptors. The binaries own their
//! validation; this group owns its own flag spellings and never re-checks
//! what the binary checks, so the two cannot drift.

pub(crate) mod connect;
pub(crate) mod forward;
pub(crate) mod serve;

use std::path::PathBuf;

use podbox_image::error::EXIT_RUNTIME_ERROR;

pub const USAGE: &str = "\
usage: podbox remote <member> [options]

  ssh          reach another machine: serve registers this machine with a
               relay and opens no listening socket, connect is the
               operator's ProxyCommand, forward reaches one fixed target
               with no relay
  -h, --help   print this usage and exit 0

  This group is the verbs that act ACROSS a machine boundary. A guest
  podbox itself runs is `podbox machine ssh`.
  See docs/decisions/remote-verb.md for why the group is named.
";

pub const SSH_USAGE: &str = "\
usage: podbox remote ssh <serve|connect|forward> [options]

  serve    --relay <wss> --name <name> --node-token <token>
           [--server <cmd>] [--once]
           register with the relay and run the SSH server for every
           operator session. The server is probed before it serves.
           --once exits 1 where no session completed, by the helper's
           own design.
  connect  --relay <wss> --name <name> --connect-token <token>
           [--ready-wait <secs>]
           move this process's standard descriptors to the session, for
           use as an ssh ProxyCommand. ssh reports 255 where the
           connection fails and the far command's own code otherwise.
  forward  (tcp <host> <port> | unix <path> | exec <prog> [args...])
           move this process's standard descriptors to one fixed target
           with no relay, for use as an ssh ProxyCommand.
  -h, --help   print this usage and exit 0

  Flags override the PODSSH_RELAY, PODSSH_NAME, PODSSH_NODE_TOKEN and
  PODSSH_CONNECT_TOKEN environment the helper binaries also read.
";

/// One value after a flag, or the flag error that says which one is missing.
pub(crate) fn value<'a>(
    it: &mut std::slice::Iter<'a, String>,
    flag: &str,
    usage: &str,
) -> Result<&'a String, i32> {
    it.next().ok_or_else(|| {
        eprintln!("podbox remote ssh: {flag} needs a value\n{usage}");
        podbox_image::error::EXIT_FLAG_ERROR
    })
}

/// The helper binary `name`, resolved beside this binary first and on PATH
/// after it. The PATH half is a plain first-executable match: a nearer
/// same-named file without the exec bit is skipped, not spawned, and a
/// name that resolves nowhere is a runtime refusal naming the binary and
/// how to provide it.
pub(crate) fn resolve_helper(name: &str) -> Result<PathBuf, i32> {
    let sibling: Option<PathBuf> = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(name)))
        .filter(|p| is_executable(p));
    let path = std::env::var_os("PATH");
    resolve_helper_in(name, sibling, path.as_deref())
}

pub(crate) fn resolve_helper_in(
    name: &str,
    sibling: Option<PathBuf>,
    path: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, i32> {
    if let Some(beside) = sibling {
        return Ok(beside);
    }
    if let Some(found) = path_lookup_in(name, path) {
        return Ok(found);
    }
    eprintln!(
        "podbox remote ssh: no `{name}` beside this binary or on PATH. \
         Build the podbox-ssh binaries (`cargo build -p podbox-ssh --bins`) \
         and run this beside them, or put them on PATH"
    );
    Err(EXIT_RUNTIME_ERROR)
}

pub(crate) fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

fn path_lookup_in(name: &str, path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let path = path?;
    std::env::split_paths(path)
        .map(|d| d.join(name))
        .find(|p| is_executable(p))
}

/// Run `bin` with `args` and inherited descriptors, and answer with the
/// child's own status. A status with no code (killed by a signal) is
/// 128 plus the signal, the shell's own rule; a status that cannot be
/// read at all is the runtime error.
pub(crate) fn run_child(bin: &std::path::Path, args: &[String]) -> i32 {
    match std::process::Command::new(bin)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
    {
        Ok(status) => status_to_code(status),
        Err(e) => {
            eprintln!("podbox remote ssh: cannot run {}: {e}", bin.display());
            EXIT_RUNTIME_ERROR
        }
    }
}

#[cfg(unix)]
fn signal_of(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

pub(crate) fn status_to_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    if let Some(sig) = signal_of(status) {
        return 128 + sig;
    }
    eprintln!("podbox remote ssh: the helper ended with no status to report");
    EXIT_RUNTIME_ERROR
}

/// The `remote` group: usage, one member, and the refusals around it.
///
/// A flag the parity table does not list never reaches an arm. A member
/// the group does not have is a caller mistake naming the member, which
/// is docker's 125: the verb was going to run something.
pub fn remote(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") | None => {
            print!("{USAGE}");
            0
        }
        Some("ssh") => ssh(&args[1..]),
        Some(first) if first.starts_with('-') => {
            // The table decides, as in every other parser: an unlisted flag
            // never reaches an arm.
            if let Err(c) = crate::parity::admit("remote", first, USAGE) {
                return c;
            }
            crate::parity::no_arm("remote", first)
        }
        Some(other) => {
            eprintln!("podbox remote: {other:?}: no such member\n{USAGE}");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn ssh(args: &[String]) -> i32 {
    let rest = if args.first().map(String::as_str) == Some("--") {
        &args[1..]
    } else {
        args
    };
    match rest.first().map(String::as_str) {
        Some("-h") | Some("--help") | None => {
            print!("{SSH_USAGE}");
            0
        }
        // A dash-shaped word here is a flag, not a subcommand: the
        // subcommands are serve, connect and forward, so anything else
        // starting with a dash is refused as a flag rather than as a
        // command the group does not have.
        Some(other) if other.starts_with('-') => {
            eprintln!("podbox remote ssh: {other} is not a flag this command has\n{SSH_USAGE}");
            podbox_image::error::EXIT_FLAG_ERROR
        }
        Some("serve") => serve::serve(&rest[1..]),
        Some("connect") => connect::connect(&rest[1..]),
        Some("forward") => forward::forward(&rest[1..]),
        Some(other) => {
            eprintln!("podbox remote ssh: {other:?}: no such command\n{SSH_USAGE}");
            EXIT_RUNTIME_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_and_help_print_usage_and_exit_zero() {
        assert_eq!(remote(&[]), 0);
        assert_eq!(remote(&["-h".to_string()]), 0);
        assert_eq!(remote(&["--help".to_string()]), 0);
        assert_eq!(remote(&["ssh".to_string()]), 0);
        assert_eq!(remote(&["ssh".to_string(), "-h".to_string()]), 0);
        assert_eq!(remote(&["ssh".to_string(), "--help".to_string()]), 0);
    }

    #[test]
    fn an_unknown_member_names_itself_and_is_a_runtime_error() {
        assert_eq!(remote(&["relay".to_string()]), EXIT_RUNTIME_ERROR);
        assert_eq!(
            remote(&["ssh".to_string(), "relay".to_string()]),
            EXIT_RUNTIME_ERROR
        );
    }

    #[test]
    fn an_unlisted_flag_is_a_flag_error() {
        assert_eq!(
            remote(&["--bogus".to_string()]),
            podbox_image::error::EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn resolution_prefers_the_sibling_then_path_then_the_refusal() {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("podbox-remote-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sibling = dir.join("probe-helper");
        std::fs::write(&sibling, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&sibling, std::fs::Permissions::from_mode(0o755)).unwrap();
        // An executable sibling wins even where PATH would answer too.
        assert_eq!(
            resolve_helper_in("probe-helper", Some(sibling.clone()), Some(dir.as_os_str()),)
                .unwrap(),
            sibling
        );
        // Without a sibling the PATH entry answers.
        assert_eq!(
            resolve_helper_in("probe-helper", None, Some(dir.as_os_str())).unwrap(),
            sibling
        );
        // A name that resolves nowhere is the runtime refusal.
        assert_eq!(
            resolve_helper_in(
                "podbox-remote-no-such-helper-xyz",
                None,
                Some(dir.as_os_str()),
            )
            .unwrap_err(),
            EXIT_RUNTIME_ERROR
        );
        // The production wrapper filters a non-executable sibling before
        // it reaches the inside: only the refusal below can observe that
        // layer, because the sibling directory is the test binary's own.
        assert_eq!(
            resolve_helper("podbox-remote-no-such-helper-xyz").unwrap_err(),
            EXIT_RUNTIME_ERROR
        );
        // A same-named file without the exec bit on PATH is skipped, not
        // spawned: neutering the exec-bit check would resolve it.
        let noexec = dir.join("probe-helper-noexec");
        std::fs::write(&noexec, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&noexec, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            resolve_helper_in("probe-helper-noexec", None, Some(dir.as_os_str())).unwrap_err(),
            EXIT_RUNTIME_ERROR
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dash_word_after_ssh_is_a_flag_refusal_not_a_command() {
        assert_eq!(
            ssh(&["--bogus".to_string()]),
            podbox_image::error::EXIT_FLAG_ERROR
        );
        assert_eq!(
            ssh(&["--".to_string(), "--bogus".to_string()]),
            podbox_image::error::EXIT_FLAG_ERROR
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_signaled_child_maps_to_128_plus_the_signal() {
        use std::os::unix::process::ExitStatusExt;
        let term = std::process::ExitStatus::from_raw(0x0f);
        assert_eq!(term.signal(), Some(15));
        assert_eq!(status_to_code(term), 143);
        let kill = std::process::ExitStatus::from_raw(0x09);
        assert_eq!(status_to_code(kill), 137);
    }
    #[test]
    fn a_child_true_reports_zero_and_a_child_false_reports_its_code() {
        assert_eq!(run_child(std::path::Path::new("/bin/true"), &[]), 0);
        assert_eq!(run_child(std::path::Path::new("/bin/false"), &[]), 1);
        assert_eq!(
            run_child(std::path::Path::new("/podbox-remote-no-such-bin-xyz"), &[]),
            EXIT_RUNTIME_ERROR
        );
    }
}
