//! The interactive session server: supervise a shell under a line
//! discipline, with no pty anywhere.
//!
//! `shell [--shell CMD]`
//!
//! `sshd` runs this under `ForceCommand`. Standard input and output are
//! the SSH client's; the discipline echoes, edits, recalls history, and
//! delivers signal characters to the shell's process group, and exits
//! with the shell's own code. It reports the error (plus the usage line
//! for usage failures) and exits with the runtime code, so the SSH
//! client prints its own diagnosis beside it.
//!
//! ⛔ One-shot commands are refused naming `SSH_ORIGINAL_COMMAND`: a
//! session server serves sessions. Subsystem requests never reach this
//! server: the generated daemon configuration defines no subsystems, so
//! `sshd` refuses them itself. Every wait on the read side is bounded;
//! writes block until the kernel takes them, like the relay legs.

#![forbid(unsafe_code)]

use podbox_ssh::session::{detect_shell, run_session, SessionConfig};
use podbox_ssh::{Error, Kind};

const USAGE: &str = "usage: shell [--shell CMD]";

fn main() {
    std::process::exit(run(&std::env::args().skip(1).collect::<Vec<_>>()));
}

fn run(argv: &[String]) -> i32 {
    match run_inner(argv) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            if e.kind() == Kind::Usage {
                eprintln!("{USAGE}");
            }
            e.exit_code()
        }
    }
}

fn run_inner(argv: &[String]) -> Result<i32, Error> {
    let usage = || Error::new(Kind::Usage, "shell args", USAGE);
    // ⛔ A command in the environment is a caller mistake, not a second
    // way to pass one: exec requests arrive refused below, and anything
    // else in the environment is ignored rather than half-honoured.
    if let Ok(cmd) = std::env::var("SSH_ORIGINAL_COMMAND") {
        if !cmd.trim().is_empty() {
            return Err(Error::new(
                Kind::Usage,
                "shell session",
                "a session server serves sessions: exec requests name SSH_ORIGINAL_COMMAND and are refused",
            ));
        }
    }
    let shell = parse_args(argv)?;
    if shell.trim().is_empty() {
        return Err(usage());
    }
    // ⛔ Standard descriptors belong to the SSH client. A failure to take
    // them is a server-side error on this end, reported like any other.
    let mut stdio = podbox_ssh::Stdio::new().map_err(|e| Error::server("shell stdio", e))?;
    run_session(&SessionConfig { shell }, &mut stdio)
}

/// Parse the shell command line into the supervised shell command.
/// Untrusted input maps to typed usage errors; nothing here panics on
/// argument shape. Tests pin this function directly, never the runner.
fn parse_args(argv: &[String]) -> Result<String, Error> {
    let usage = || Error::new(Kind::Usage, "shell args", USAGE);
    let mut shell = detect_shell();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--shell" => {
                i += 1;
                shell = argv.get(i).ok_or_else(usage)?.clone();
            }
            _ => return Err(usage()),
        }
        i += 1;
    }
    Ok(shell)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn shell_flag_selects() {
        assert_eq!(
            parse_args(&args(&["--shell", "sh"])).unwrap(),
            "sh".to_string()
        );
        // The default is the environment or `sh`, never an empty command.
        assert!(!parse_args(&args(&[])).unwrap().trim().is_empty());
    }

    #[test]
    fn missing_and_bad_flags_are_usage_errors() {
        for argv in [
            args(&["--bogus"]),
            args(&["--shell"]),
            args(&["--shell", "sh", "extra"]),
        ] {
            // ⛔ No `unwrap_err`: the failure must stay a kind, never a
            // value that could carry a secret into a test log.
            let e = match parse_args(&argv) {
                Ok(_) => panic!("{argv:?} parsed"),
                Err(e) => e,
            };
            assert_eq!(e.kind(), Kind::Usage, "{argv:?}: {e}");
        }
    }
}
