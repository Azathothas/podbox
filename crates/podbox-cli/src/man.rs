//! `podbox man`: the manual, generated from the binary's own data.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-1332: help text copied into a
//! document drifts from the binary the day a flag is added, so the manual is
//! rendered here instead: the command list and the one-line summaries come
//! from the parity table, and every reference section carries the exact bytes
//! that verb's `--help` prints, captured by re-executing this binary. A new
//! verb appears by construction once it has a table row; a changed usage
//! string appears without any edit here.
//!
//! ⛔ No hand-kept verb list. [`implemented`] is the table's own implemented
//! verbs minus the argv0 aliases, and [`help_argv`] is pinned against
//! [`parity::rows_of`] by unit test, so the two cannot fork.

use std::io::{IsTerminal, Write};

use crate::parity::{self, Row, Status};

pub const MAN_USAGE: &str = "\
usage: podbox man [--no-pager] [verb]

  man            render the manual from the binary's own usage strings
                 and the parity table
  man <verb>     render only that verb's section
  --no-pager     print to stdout even on a terminal
";

/// The verbs that get a command-reference section: every implemented table
/// verb except the argv0 aliases, which are names for this binary rather
/// than commands it takes.
fn implemented() -> Vec<&'static Row> {
    parity::TABLE
        .iter()
        .filter(|r| {
            r.flag.is_none()
                && !matches!(r.status, Status::None)
                && !crate::names::ALIASES.contains(&r.verb)
        })
        .collect()
}

/// The argv that prints a verb's usage: bare for a top-level verb, through
/// the group for a subverb whose rows live under its own name (T-0808).
fn help_argv(verb: &str) -> Vec<&str> {
    match verb {
        "prune" => vec!["image", "prune"],
        "install-names" | "abi" => vec!["system", verb],
        _ => vec![verb],
    }
}

/// Capture what `podbox <argv...> --help` prints on stdout: the same bytes
/// the caller sees, so the manual cannot drift from them. Empty (a refusal,
/// which goes to stderr) is `None`.
fn help_of(verb: &str) -> Option<String> {
    let mut cmd = help_argv(verb);
    cmd.push("--help");
    let exe = std::env::current_exe().ok()?;
    let out = std::process::Command::new(exe).args(&cmd).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

/// The command-list summary: the note's first sentence. The notes are
/// written as sentences, so this is a read rather than a parse; the unit
/// test below holds it on fixed strings.
fn summary(note: &str) -> &str {
    match note.find(". ") {
        Some(i) => note[..=i].trim(),
        None => note.trim().trim_end_matches('.'),
    }
}

fn render_section(out: &mut String, row: &Row, help: &dyn Fn(&str) -> Option<String>) {
    out.push_str(&format!(
        "=== podbox {} [{}]\n",
        row.verb,
        row.status.word()
    ));
    out.push_str(row.note.trim());
    out.push('\n');
    match help(row.verb) {
        Some(text) => {
            if !text.ends_with('\n') {
                out.push_str(&text);
                out.push('\n');
            } else {
                out.push_str(&text);
            }
        }
        None => {
            out.push_str(&format!(
                "No usage to show: `podbox {}` is not a command this binary takes.\n",
                row.verb
            ));
        }
    }
    out.push('\n');
}

/// The whole manual, or one verb's section under the same header.
fn render(only: Option<&str>, help: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let verbs = implemented();
    if let Some(want) = only {
        let row = parity::TABLE
            .iter()
            .find(|r| r.flag.is_none() && r.verb == want)?;
        let mut out = String::new();
        header(&mut out);
        render_section(&mut out, row, &help);
        return Some(out);
    }
    let mut out = String::new();
    header(&mut out);
    out.push_str("COMMANDS\n");
    let width = verbs.iter().map(|r| r.verb.len()).max().unwrap_or(0);
    for row in &verbs {
        out.push_str(&format!(
            "  {:<width$} {} [{}]\n",
            row.verb,
            summary(row.note),
            row.status.word(),
            width = width
        ));
    }
    out.push('\n');
    global(&mut out);
    out.push_str("COMMAND REFERENCE\n");
    for row in &verbs {
        render_section(&mut out, row, &help);
    }
    Some(out)
}

fn header(out: &mut String) {
    out.push_str(&format!("podbox {} manual\n\n", env!("CARGO_PKG_VERSION")));
}

fn global(out: &mut String) {
    out.push_str(&format!(
        "GLOBAL\n\
         Every verb prints its own usage with -h or --help and exits 0.\n\
         Installed as docker, podman or podvm, this binary takes the same \
         verbs under any of those names.\n\
         A refusal from a verb's flag parser exits {}; a refusal after the \
         flags parsed exits {}; a payload's exit code is its own, 128+N \
         for a signal. The full table is data: podbox system info \
         --format '{{{{json .ExitCodes}}}}'.\n\
         podbox never prompts: a missing input is a refusal naming what is \
         missing.\n\
         Answers go to stdout; progress and diagnostics go to stderr.\n\n",
        podbox_probe::exit::EXIT_FLAG_ERROR,
        podbox_probe::exit::EXIT_CLI_ERROR,
    ));
}

fn run(args: &[String], help: &dyn Fn(&str) -> Option<String>) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let args: Vec<String> = match crate::parity::expand("man", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member("man", &member, MAN_USAGE);
        }
    };
    let mut no_pager = false;
    let mut want: Option<&str> = None;
    for a in &args {
        match a.as_str() {
            "--no-pager" => no_pager = true,
            "-h" | "--help" => {
                print!("{MAN_USAGE}");
                return 0;
            }
            s if s.starts_with('-') => {
                if let Err(c) = crate::parity::admit("man", s, MAN_USAGE) {
                    return c;
                }
                return crate::parity::no_arm("man", s);
            }
            s => {
                if want.is_some() {
                    eprint!("{MAN_USAGE}");
                    return podbox_image::error::EXIT_FLAG_ERROR;
                }
                want = Some(s);
            }
        }
    }
    if let Some(name) = want {
        if parity::TABLE
            .iter()
            .all(|r| r.flag.is_some() || r.verb != name)
        {
            let mut err = std::io::stderr().lock();
            let _ = writeln!(
                err,
                "podbox man: {name}: podbox has no such command, and neither does \
                 the parity table (TOOL.md section 6.8)"
            );
            return podbox_image::error::EXIT_CLI_ERROR;
        }
    }
    let text = match render(want, &help) {
        Some(t) => t,
        None => return podbox_image::error::EXIT_CLI_ERROR,
    };
    // ⭐ The pager is a display choice, never the answer: `--no-pager` and a
    // non-terminal stdout print the same bytes, and a pager that cannot
    // start falls back to stdout rather than failing the manual.
    let pager: Option<String> = if no_pager || !std::io::stdout().is_terminal() {
        None
    } else {
        Some(
            std::env::var("PAGER")
                .ok()
                .filter(|p| !p.trim().is_empty())
                .unwrap_or_else(|| "less".to_string()),
        )
    };
    match pager {
        None => {
            print!("{text}");
            0
        }
        Some(p) => {
            let mut child = match std::process::Command::new(&p)
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("podbox man: {p} could not start ({e}), printing without a pager");
                    print!("{text}");
                    return 0;
                }
            };
            let wrote = match child.stdin.take() {
                Some(mut stdin) => stdin.write_all(text.as_bytes()).is_ok(),
                None => false,
            };
            if !wrote {
                eprintln!("podbox man: {p} took no input, printing without a pager");
                print!("{text}");
                return 0;
            }
            let _ = child.wait();
            0
        }
    }
}

pub fn man(args: &[String]) -> i32 {
    run(args, &help_of)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub(verb: &str) -> Option<String> {
        Some(format!("usage: podbox {verb} [options]\n"))
    }

    #[test]
    fn every_implemented_verb_renders_a_section() {
        let text = render(None, &stub).expect("the full manual renders");
        let verbs = implemented();
        assert!(!verbs.is_empty(), "the manual lists no verbs");
        for row in &verbs {
            assert!(
                text.contains(&format!("=== podbox {} [{}]", row.verb, row.status.word())),
                "no section for {}",
                row.verb
            );
            assert!(
                text.contains(&format!("usage: podbox {} [options]", row.verb)),
                "no help bytes for {}",
                row.verb
            );
        }
        let sections = text.matches("=== podbox ").count();
        assert_eq!(sections, verbs.len(), "{text}");
    }

    #[test]
    fn aliases_render_no_reference_section() {
        let text = render(None, &stub).expect("the full manual renders");
        for alias in crate::names::ALIASES {
            assert!(
                !text.contains(&format!("=== podbox {alias} ")),
                "{alias} is a name for the binary, not a command section"
            );
        }
    }

    #[test]
    fn no_unimplemented_verb_gets_a_reference_section() {
        let verbs = implemented();
        assert!(
            verbs.iter().all(|r| !matches!(r.status, Status::None)),
            "a refused verb would render as a command"
        );
        assert!(verbs.iter().all(|r| r.flag.is_none()));
    }

    #[test]
    fn one_verb_renders_only_its_section() {
        let text = render(Some("ps"), &stub).expect("ps renders");
        assert!(text.contains("=== podbox ps "), "{text}");
        assert!(!text.contains("=== podbox run "), "{text}");
        assert!(render(Some("frobnicate"), &stub).is_none());
    }

    #[test]
    fn group_help_paths_match_rows_of() {
        for (single, multi) in [
            ("prune", "image prune"),
            ("install-names", "system install-names"),
            ("abi", "system abi"),
        ] {
            assert_eq!(parity::rows_of(multi), single, "{multi}");
            assert_eq!(help_argv(single).join(" "), multi, "{single}");
        }
    }

    #[test]
    fn summary_is_the_first_sentence() {
        assert_eq!(
            summary("enters a chroot, never a namespace. The rest."),
            "enters a chroot, never a namespace."
        );
        assert_eq!(summary("one artefact, no split"), "one artefact, no split");
    }

    #[test]
    fn the_manual_names_its_sources() {
        let text = render(None, &stub).expect("the full manual renders");
        assert!(text.starts_with(&format!("podbox {} manual", env!("CARGO_PKG_VERSION"))));
        assert!(text.contains("GLOBAL"));
        assert!(text.contains("COMMAND REFERENCE"));
    }
}
