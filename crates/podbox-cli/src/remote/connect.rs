//! `podbox remote ssh connect`: the operator side of a relay session.
//!
//! Moves this process's standard descriptors to the pair's session, which
//! is what makes this arm a valid ssh `ProxyCommand`: ssh writes and reads
//! this process's descriptors and the lane's own `operator` binary moves
//! those bytes to the session and back. This arm owns the flag spellings
//! and passes them through verbatim, so the binary's validation is the one
//! validation and the two cannot drift.

use super::{resolve_helper, run_child, value, SSH_USAGE};

/// Every dash-flag `parse` below accepts. The parity table carries one row
/// per entry, and the cross-check in `crate::parity`'s tests holds the two
/// lists identical in both directions, so a flag cannot land in one and
/// miss the other. The match arms below must list exactly these.
pub(crate) const FLAGS: &[&str] = &["--relay", "--name", "--connect-token", "--ready-wait"];

/// `podbox remote ssh connect --relay <wss> --name <name> --connect-token
/// <token> [--ready-wait <secs>]`. `Ok(None)` is `-h/--help`.
///
/// A leading `--` is a separator and is dropped: there are no positionals
/// past it, only flags. Values are never validated here, not even
/// `--ready-wait`: the binary owns that check, and re-checking it here
/// would be the drift the module docs forbid.
fn parse(args: &[String]) -> Result<Option<Vec<String>>, i32> {
    let mut out: Vec<String> = Vec::new();
    let mut it = args.iter();
    if it.clone().next().map(String::as_str) == Some("--") {
        it.next();
    }
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{SSH_USAGE}");
                return Ok(None);
            }
            "--relay" | "--name" | "--connect-token" | "--ready-wait" => {
                let v = value(&mut it, arg, SSH_USAGE)?;
                out.push(arg.clone());
                out.push(v.clone());
            }
            other if other.starts_with('-') => {
                // FLAGS is the list this command takes; the arms above are
                // its shape. Anything outside the list never reaches an arm,
                // and anything inside it without an arm above refuses rather
                // than falling through (the parity cross-test forbids that
                // drift, and the positive tests pin every arm).
                if !FLAGS.contains(&other) {
                    eprintln!(
                        "podbox remote ssh connect: {other} is not a flag this command has\n{SSH_USAGE}"
                    );
                } else {
                    eprintln!(
                        "podbox remote ssh connect: {other} is listed but has no arm\n{SSH_USAGE}"
                    );
                }
                return Err(podbox_image::error::EXIT_FLAG_ERROR);
            }
            other => {
                eprintln!(
                    "podbox remote ssh connect: {other:?} is not a positional this command takes\n{SSH_USAGE}"
                );
                return Err(podbox_image::error::EXIT_FLAG_ERROR);
            }
        }
    }
    Ok(Some(out))
}

pub fn connect(args: &[String]) -> i32 {
    let flags = match parse(args) {
        Ok(None) => return 0,
        Ok(Some(flags)) => flags,
        Err(c) => return c,
    };
    let bin = match resolve_helper("operator") {
        Ok(bin) => bin,
        Err(c) => return c,
    };
    run_child(&bin, &flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(xs: &[&str]) -> Result<Option<Vec<String>>, i32> {
        parse(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn help_prints_and_exits_zero() {
        assert!(matches!(p(&["-h"]), Ok(None)));
        assert!(matches!(p(&["--help"]), Ok(None)));
    }

    #[test]
    fn flags_pass_through_verbatim_in_order() {
        let flags = p(&[
            "--relay",
            "wss://example.com",
            "--name",
            "n1",
            "--connect-token",
            "tok",
            "--ready-wait",
            "30",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(
            flags,
            vec![
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--connect-token",
                "tok",
                "--ready-wait",
                "30",
            ]
        );
    }

    #[test]
    fn a_leading_separator_is_dropped_and_partial_sets_pass() {
        let flags = p(&["--", "--relay", "wss://example.com"]).unwrap().unwrap();
        assert_eq!(flags, vec!["--relay", "wss://example.com"]);
    }

    #[test]
    fn values_are_never_validated_here() {
        // The binary owns the `--ready-wait` range check; re-checking it
        // here would be the drift the module docs forbid.
        let flags = p(&["--ready-wait", "soon"]).unwrap().unwrap();
        assert_eq!(flags, vec!["--ready-wait", "soon"]);
    }

    #[test]
    fn a_flag_without_its_value_and_an_unknown_flag_are_flag_errors() {
        use podbox_image::error::EXIT_FLAG_ERROR;
        assert_eq!(p(&["--relay"]).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(
            p(&["--name", "n1", "--node-token", "tok"]).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(p(&["extra"]).unwrap_err(), EXIT_FLAG_ERROR);
    }

    #[test]
    fn every_listed_flag_has_an_arm() {
        // A flag added to FLAGS without a match arm above would fall
        // into the "listed but has no arm" refusal; this test turns
        // that drift red by requiring every entry to parse clean.
        for flag in FLAGS {
            let args: Vec<String> = vec![flag.to_string(), "v".to_string()];
            assert!(
                matches!(parse(&args), Ok(Some(_))),
                "{flag} is listed but has no arm"
            );
        }
    }
}
