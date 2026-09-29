//! `podbox machine`: the machine group, and its one member `ssh`.
//! `TODO/podssh.md` T-1404.
//!
//! `machine` is a different axis from `remote`: a guest podbox itself runs
//! and manages, against a machine somebody else started. That split is
//! podman's (`podman machine ssh` opens a shell in podman's own VM), and
//! the entry keeps the two apart for the same reason: folding them would
//! make one verb mean two things.
//!
//! The member `ssh` boots a disposable Linux guest from the named kernel
//! and initramfs and runs one SSH command in it, reporting the guest
//! command's exit code. The SSH server executes in the guest on its serial
//! line; the handshake itself is the probe, so no separate probe can go
//! stale beside it. Guests with no SSH endpoint never reach this arm:
//! there is no Windows or DOS flavor here, only the Linux guest the flags
//! describe.

pub(crate) mod ssh;

use podbox_image::error::EXIT_RUNTIME_ERROR;

pub const USAGE: &str = "\
usage: podbox machine <member> [options]

  ssh          open an SSH session in a Linux guest podbox boots (podman
               parity): --kernel, --initramfs and --user-key describe the
               guest, and the guest command's exit code is the process's
  -h, --help   print this usage and exit 0
";

/// The `machine` group: usage, one member, and the refusals around it.
///
/// A flag the parity table does not list never reaches an arm. A member the
/// group does not have is a caller mistake naming the member. `ssh` boots
/// the Linux guest its flags describe and reports the guest command's exit
/// code, which is docker's 125 where the run itself fails.
pub fn machine(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") | None => {
            print!("{USAGE}");
            0
        }
        Some("ssh") => ssh::ssh(&args[1..]),
        Some(first) if first.starts_with('-') => {
            // The table decides, as in every other parser: an unlisted flag
            // never reaches an arm.
            if let Err(c) = crate::parity::admit("machine", first, USAGE) {
                return c;
            }
            crate::parity::no_arm("machine", first)
        }
        Some(other) => {
            eprintln!("podbox machine: {other:?}: no such member\n{USAGE}");
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
    fn ssh_without_its_guest_flags_is_a_flag_error() {
        assert_eq!(machine(&["ssh".to_string()]), EXIT_FLAG_ERROR);
        assert_eq!(
            machine(&["ssh".to_string(), "--kernel".to_string(), "k".to_string()]),
            EXIT_FLAG_ERROR
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
}
