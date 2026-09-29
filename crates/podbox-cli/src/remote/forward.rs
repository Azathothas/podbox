//! `podbox remote ssh forward`: one fixed target with no relay.
//!
//! Moves this process's standard descriptors to a target the lane's own
//! `proxy` binary dials, which is what makes this arm a valid ssh
//! `ProxyCommand` for a directly reachable SSH server. This arm owns no
//! flag spellings at all: everything past `forward` passes through
//! verbatim and the binary's validation is the one validation, so the
//! two cannot drift.

use super::{resolve_helper, run_child, SSH_USAGE};

/// Split the arm's argv into the target words for the proxy binary.
/// `Ok(None)` is `-h/--help`.
///
/// Help is first-word-only on this arm, unlike its siblings: everything
/// past `forward` is the target, and a later `-h` may belong to the
/// target's own command (`forward exec prog -h` forwards that flag, it
/// does not print this usage).
fn forward_argv(args: &[String]) -> Result<Option<Vec<String>>, i32> {
    let rest = if args.first().map(String::as_str) == Some("--") {
        &args[1..]
    } else {
        args
    };
    if rest.first().map(String::as_str) == Some("-h")
        || rest.first().map(String::as_str) == Some("--help")
    {
        print!("{SSH_USAGE}");
        return Ok(None);
    }
    if rest.is_empty() {
        eprintln!("podbox remote ssh forward: needs a target: tcp <host> <port>, unix <path>, or exec <prog> [args...]\n{SSH_USAGE}");
        return Err(podbox_image::error::EXIT_FLAG_ERROR);
    }
    Ok(Some(rest.to_vec()))
}

pub fn forward(args: &[String]) -> i32 {
    let words = match forward_argv(args) {
        Ok(None) => return 0,
        Ok(Some(words)) => words,
        Err(c) => return c,
    };
    let bin = match resolve_helper("proxy") {
        Ok(bin) => bin,
        Err(c) => return c,
    };
    run_child(&bin, &words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(xs: &[&str]) -> Result<Option<Vec<String>>, i32> {
        forward_argv(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn help_as_the_first_word_prints_and_exits_zero() {
        assert!(matches!(f(&["-h"]), Ok(None)));
        assert!(matches!(f(&["--help"]), Ok(None)));
    }

    #[test]
    fn a_later_help_word_belongs_to_the_target() {
        let words = f(&["exec", "prog", "-h"]).unwrap().unwrap();
        assert_eq!(words, vec!["exec", "prog", "-h"]);
    }

    #[test]
    fn no_target_is_a_flag_error() {
        assert_eq!(f(&[]).unwrap_err(), podbox_image::error::EXIT_FLAG_ERROR);
    }

    #[test]
    fn a_leading_separator_is_dropped_and_the_target_passes_through() {
        let words = f(&["--", "tcp", "h", "22"]).unwrap().unwrap();
        assert_eq!(words, vec!["tcp", "h", "22"]);
        let words = f(&["unix", "/tmp/x.sock"]).unwrap().unwrap();
        assert_eq!(words, vec!["unix", "/tmp/x.sock"]);
    }

    #[test]
    fn forward_refuses_where_no_proxy_resolves() {
        // Hermetic: the sibling is absent and PATH names nothing, so the
        // refusal names the binary rather than spawning something else.
        // (The PATH-mutating shape is refused here: tests share one
        // process environment and a global switch would race them.)
        let bin = super::super::resolve_helper_in(
            "podbox-remote-no-such-proxy-xyz",
            None,
            Some(std::ffi::OsStr::new("/podbox-remote-no-such-dir-xyz")),
        );
        assert_eq!(bin.unwrap_err(), podbox_image::error::EXIT_RUNTIME_ERROR);
    }
}
