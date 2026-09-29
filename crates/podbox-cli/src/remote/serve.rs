//! `podbox remote ssh serve`: the agent side of a relay session.
//!
//! Registers one pair name with the relay and runs the SSH server for
//! every operator session that arrives. The server is the lane's own
//! `node` binary, which probes its server before it serves; this arm owns
//! the flag spellings and passes them through verbatim, so the binary's
//! validation is the one validation and the two cannot drift.

use super::{resolve_helper, run_child, value, SSH_USAGE};

/// Every dash-flag `parse` below accepts. The parity table carries one row
/// per entry, and the cross-check in `crate::parity`'s tests holds the two
/// lists identical in both directions, so a flag cannot land in one and
/// miss the other. The match arms below must list exactly these.
pub(crate) const FLAGS: &[&str] = &["--relay", "--name", "--node-token", "--server", "--once"];

/// `podbox remote ssh serve --relay <wss> --name <name> --node-token
/// <token> [--server <cmd>] [--once]`. `Ok(None)` is `-h/--help`.
///
/// A leading `--` is a separator and is dropped: there are no positionals
/// past it, only flags.
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
            "--relay" | "--name" | "--node-token" | "--server" => {
                let v = value(&mut it, arg, SSH_USAGE)?;
                out.push(arg.clone());
                out.push(v.clone());
            }
            "--once" => {
                out.push(arg.clone());
            }
            other if other.starts_with('-') => {
                // FLAGS is the list this command takes; the arms above are
                // its shape. Anything outside the list never reaches an arm,
                // and anything inside it without an arm above refuses rather
                // than falling through (the parity cross-test forbids that
                // drift, and the positive tests pin every arm).
                if !FLAGS.contains(&other) {
                    eprintln!(
                        "podbox remote ssh serve: {other} is not a flag this command has\n{SSH_USAGE}"
                    );
                } else {
                    eprintln!(
                        "podbox remote ssh serve: {other} is listed but has no arm\n{SSH_USAGE}"
                    );
                }
                return Err(podbox_image::error::EXIT_FLAG_ERROR);
            }
            other => {
                eprintln!(
                    "podbox remote ssh serve: {other:?} is not a positional this command takes\n{SSH_USAGE}"
                );
                return Err(podbox_image::error::EXIT_FLAG_ERROR);
            }
        }
    }
    Ok(Some(out))
}

pub fn serve(args: &[String]) -> i32 {
    let flags = match parse(args) {
        Ok(None) => return 0,
        Ok(Some(flags)) => flags,
        Err(c) => return c,
    };
    let bin = match resolve_helper("node") {
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
            "--node-token",
            "tok",
            "--server",
            "cat",
            "--once",
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
                "--node-token",
                "tok",
                "--server",
                "cat",
                "--once",
            ]
        );
    }

    #[test]
    fn a_leading_separator_is_dropped_and_partial_sets_pass() {
        let flags = p(&["--", "--relay", "wss://example.com", "--once"])
            .unwrap()
            .unwrap();
        assert_eq!(flags, vec!["--relay", "wss://example.com", "--once",]);
        let once = p(&["--once"]).unwrap().unwrap();
        assert_eq!(once, vec!["--once"]);
    }

    #[test]
    fn values_with_spaces_pass_through_whole() {
        let flags = p(&["--server", "my server --flag"]).unwrap().unwrap();
        assert_eq!(flags, vec!["--server", "my server --flag"]);
    }

    #[test]
    fn a_flag_without_its_value_and_an_unknown_flag_are_flag_errors() {
        use podbox_image::error::EXIT_FLAG_ERROR;
        assert_eq!(p(&["--relay"]).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(
            p(&["--name", "n1", "--bogus"]).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(p(&["extra"]).unwrap_err(), EXIT_FLAG_ERROR);
    }

    #[test]
    fn the_connect_side_flags_do_not_belong_here() {
        use podbox_image::error::EXIT_FLAG_ERROR;
        assert_eq!(
            p(&["--relay", "w", "--connect-token", "t"]).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            p(&["--relay", "w", "--ready-wait", "30"]).unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn every_listed_flag_has_an_arm() {
        // A flag added to FLAGS without a match arm above would fall
        // into the "listed but has no arm" refusal; this test turns
        // that drift red by requiring every entry to parse clean.
        for flag in FLAGS {
            let args: Vec<String> = if *flag == "--once" {
                vec![flag.to_string()]
            } else {
                vec![flag.to_string(), "v".to_string()]
            };
            assert!(
                matches!(parse(&args), Ok(Some(_))),
                "{flag} is listed but has no arm"
            );
        }
    }
}
