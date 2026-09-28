//! `podbox machine`: the machine group, and its one member `ssh`.
//! `TODO/podssh.md` T-1404.
//!
//! `machine` is a different axis from `remote`: a guest podbox itself runs
//! and manages, against a machine somebody else started. That split is
//! podman's (`podman machine ssh` opens a shell in podman's own VM), and
//! the entry keeps the two apart for the same reason: folding them would
//! make one verb mean two things.
//!
//! The member `ssh` refuses on every path today, and says why: no guest
//! driver carries an SSH server endpoint. Linux guests have no in-product
//! driver at all (the tier refuses naming the missing legs), and Windows
//! and DOS guests take commands through the mailbox agent rather than SSH.
//! A refusal that names the missing endpoint is the whole implementation
//! until that endpoint exists. The entry stays open on it.

use podbox_image::error::EXIT_RUNTIME_ERROR;

pub const USAGE: &str = "\
usage: podbox machine <member> [options]

  ssh          open an SSH session in a guest podbox runs (podman parity)
  -h, --help   print this usage and exit 0
";

const SSH_USAGE: &str = "\
usage: podbox machine ssh [guest] [-- command ...]

  Open an SSH session in a machine guest podbox runs, and run the command
  there. The guest's exit code is the process's own exit code.

  Refused today naming the missing endpoint: no guest driver carries an
  SSH server (TODO/podssh.md T-1404).
  -h, --help   print this usage and exit 0
";

/// The refusal `machine ssh` prints, as a value so tests read it.
///
/// `guest` is the named guest, or `None` where none was named. The reason
/// never depends on it: the endpoint is missing on every guest.
fn ssh_refusal(guest: Option<&str>) -> String {
    let who = guest.unwrap_or("no guest named");
    format!(
        "podbox machine ssh: {who}: no machine guest carries an SSH server. \
         Linux guests have no in-product driver (the machine tier refuses \
         naming the missing legs), and Windows and DOS guests take commands \
         through the mailbox agent rather than SSH. Nothing started and \
         nothing was fetched (TODO/podssh.md T-1404)"
    )
}

fn ssh(args: &[String]) -> i32 {
    let mut rest = args;
    if rest.first().map(String::as_str) == Some("--") {
        rest = &rest[1..];
    }
    match rest.first().map(String::as_str) {
        Some("-h") | Some("--help") => {
            print!("{SSH_USAGE}");
            0
        }
        guest => {
            eprintln!("{}", ssh_refusal(guest));
            EXIT_RUNTIME_ERROR
        }
    }
}

/// The `machine` group: usage, one member, and the refusals around it.
///
/// A flag the parity table does not list never reaches an arm. A member the
/// group does not have is a caller mistake naming the member. `ssh` refuses
/// with the missing endpoint, which is docker's 125.
pub fn machine(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") | None => {
            print!("{USAGE}");
            0
        }
        Some("ssh") => ssh(&args[1..]),
        Some(first) if first.starts_with('-') => {
            // The table decides, as in every other parser: an unlisted flag
            // never reaches an arm.
            if let Err(c) = crate::parity::admit("machine", first, USAGE) {
                return c;
            }
            crate::parity::no_arm("machine", first)
        }
        Some(other) => {
            eprintln!("podbox machine: {other:?}: not implemented yet");
            EXIT_RUNTIME_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use podbox_image::error::EXIT_FLAG_ERROR;

    #[test]
    fn bare_and_help_print_usage_and_exit_zero() {
        assert_eq!(machine(&[]), 0);
        assert_eq!(machine(&["-h".to_string()]), 0);
        assert_eq!(machine(&["--help".to_string()]), 0);
    }

    #[test]
    fn ssh_refuses_with_the_runtime_error_on_every_spelling() {
        assert_eq!(machine(&["ssh".to_string()]), EXIT_RUNTIME_ERROR);
        assert_eq!(
            machine(&["ssh".to_string(), "guest".to_string()]),
            EXIT_RUNTIME_ERROR
        );
        assert_eq!(
            machine(&["ssh".to_string(), "--".to_string(), "guest".to_string()]),
            EXIT_RUNTIME_ERROR
        );
    }

    #[test]
    fn ssh_help_prints_its_usage_and_exits_zero() {
        assert_eq!(machine(&["ssh".to_string(), "-h".to_string()]), 0);
        assert_eq!(machine(&["ssh".to_string(), "--help".to_string()]), 0);
    }

    #[test]
    fn an_unknown_member_is_a_runtime_error() {
        assert_eq!(machine(&["relay".to_string()]), EXIT_RUNTIME_ERROR);
    }

    #[test]
    fn an_unlisted_flag_is_a_flag_error() {
        assert_eq!(machine(&["--bogus".to_string()]), EXIT_FLAG_ERROR);
    }

    #[test]
    fn the_refusal_names_the_guest_and_the_missing_endpoint() {
        let named = ssh_refusal(Some("guest"));
        assert!(named.contains("guest"), "{named}");
        assert!(
            named.contains("no machine guest carries an SSH server"),
            "{named}"
        );
        assert!(
            named.contains("Nothing started and nothing was fetched"),
            "{named}"
        );
        let unnamed = ssh_refusal(None);
        assert!(unnamed.contains("no guest named"), "{unnamed}");
        assert!(
            unnamed.contains("no machine guest carries an SSH server"),
            "{unnamed}"
        );
    }
}
