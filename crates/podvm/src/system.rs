//! `podvm system info`, and `podvm info` as docker spells it.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0801.
//!
//! ⭐ **This verb exists so the parity table is reachable from a program.** An
//! agent that knows docker needs to be able to ask what this podvm will and
//! will not do BEFORE it runs anything, and the answer has to be data:
//!
//! ```sh
//! podvm system info --format '{{json .Parity}}' | jq -r '.[] | select(.status=="None") | .verb'
//! ```
//!
//! ⛔ **podvm has no daemon**, so docker's server half of `info` is not
//! withheld and not faked: what stands in its place is the rung this machine
//! permits, measured, and what that rung does not provide.

use crate::{format, parity};
use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podvm_probe::select::Rung;

pub const SYSTEM_USAGE: &str = "\
usage: podvm system info [--format T]
       podvm system df
       podvm system install-names [--dir D] [--force]
       podvm system abi <object> <libc>
       podvm info [--format T]

  df           disk usage: images with one row each, containers, stored
               blob bytes beside extracted rootfs bytes, and what
               `image prune` would reclaim. Accounting never fails the
               verb: an unreadable file counts nothing (TODO/cli.md
               T-1337)

  abi          may <object> be preloaded into a payload served by <libc>?
               Answered by READING both, never by loading one:
               DT_NEEDED against the libc's own SONAME, then every imported
               symbol and every imported symbol VERSION, from .dynsym and
               never .symtab (TODO/interpose.md T-0709).
               Exit 0 it may, 1 it may not and the reason is on stderr, 2 a
               file could not be read.

  Fields: .Parity .ParityRows .ExitCodes .Version .Rung .EnteredRung
          .StrictOk .Store .Banner .Platform .Kernel .Verbs .VerbsNative
          .VerbsDegraded .VerbsStub .VerbsNone

  --format T   {{.Field}} placeholders, plus `{{json .Field}}`. .Parity is
               already a JSON document, so both spellings print it.

  refused: podvm has no daemon, so there is no client/server split to report. The
    rung this machine permits stands where docker prints a server version,
    and it is measured rather than assumed.

  refused: .Rung and .EnteredRung are TWO ANSWERS and a caller needs both. .Rung is
    what this machine would permit; .EnteredRung is the sequence `podvm run`
    actually performs: the namespace rung where the probe selects it, else a
    chroot, and `userland` wherever chroot(2) is denied, whatever the
    selection, as `run` enters a no-chroot family there or refuses
    (TODO/enter.md T-1317, TODO/cli.md T-1713); `incus` where `tier = incus`
    in $store/config and its socket is there (TODO/machine.md T-1722). Reporting the first
    as if it were the second is the defect TODO/cli.md T-0804 was opened for.

  note: .ExitCodes is docker's exit-code contract as data: one object per case,
    with `case`, `code` and `what`. Measured against docker rather than read,
    by experiments/330-exit-codes.sh (TODO/cli.md T-0802).

  note: .Parity is the verb and flag parity table (TOOL.md section 6.8) as data:
    one object per row, with `verb`, `flag`, `status` and `note`. `status` is
    one of Native, Degraded, Stub, None and there is no fifth.
";

/// Fields whose value is already a JSON document, so `{{json .X}}` prints it
/// rather than quoting it. ⚠ A subset of [`FIELDS`], never a second list.
const DOCUMENTS: &[&str] = &["Parity", "ExitCodes"];

pub const FIELDS: &[&str] = &[
    "Parity",
    "ParityRows",
    // ⭐ T-0802. docker's exit codes as data, for the same reason the parity
    // table is data: a caller decides from it, and a measurement asserts
    // against it rather than against a number typed into a shell script.
    "ExitCodes",
    "Version",
    "Rung",
    // ⭐ T-0804 rule 4, as DATA. The entered rung is the one constant the
    // banner is built from wherever chroot holds, and `userland` where it
    // does not (TODO/enter.md T-1317): a script that had only `.Rung` to
    // read could print "the rung podvm uses" and name a rung podvm does not
    // enter with -- which `experiments/240-distro-sweep.sh` did.
    "EnteredRung",
    "StrictOk",
    "Store",
    "Banner",
    "Platform",
    "Kernel",
    "Verbs",
    "VerbsNative",
    "VerbsDegraded",
    "VerbsStub",
    "VerbsNone",
];

/// Whether this machine's `<store>/config` suppresses the mode banner.
///
/// ⚠ Reads `printed` where no store can be opened rather than refusing: the
/// banner's default is on, and a machine with no store has not suppressed it.
fn banner_state() -> String {
    match podvm_image::open_store() {
        Ok(s) if crate::complete::banner_quiet(&s) => format!(
            "suppressed by {}/{} ({} = quiet)",
            s.root().display(),
            crate::complete::CONFIG_FILE,
            crate::complete::BANNER_KEY
        ),
        _ => "printed on every run and exec".to_string(),
    }
}

/// `podvm system <sub>`.
pub fn system(args: &[String]) -> i32 {
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    match args.first().map(String::as_str) {
        Some("info") => info("system info", rest),
        Some("df") => df("system df", rest),
        Some("install-names") => crate::names::install("system install-names", rest),
        Some("abi") => abi("system abi", rest),
        Some("-h") | Some("--help") | None => {
            print!("{SYSTEM_USAGE}");
            0
        }
        Some(first) if first.starts_with('-') => {
            if let Err(c) = crate::parity::admit("system", first, SYSTEM_USAGE) {
                return c;
            }
            crate::parity::no_arm("system", first)
        }
        Some(other) => {
            // ⛔ The table answers, not this match arm. `podvm system df` gets
            // the reason `system` is Degraded rather than a bare "no".
            eprintln!(
                "podvm system: {other:?} is not implemented. `podvm system info` is, \
                 and `podvm system info --format '{{{{json .Parity}}}}'` lists every \
                 verb podvm has and every one it does not"
            );
            podvm_image::error::EXIT_RUNTIME_ERROR
        }
    }
}

/// `podvm system abi <object> <libc>`.
///
/// ⭐ **[`TODO/interpose.md`](../../../TODO/interpose.md) T-0709's answer, made
/// drivable.** The selection is a reader, so the way to assert it is to point
/// it at two real files and compare its prediction with what the loader then
/// does -- which is what `experiments/80-interposer-abi.sh` does, against the
/// same objects it builds for its own checks.
///
/// ⛔ Three exit codes and the third is not a failure: 0 admitted, 1 refused
/// with the reason on stderr, 2 a file podvm could not read.
pub fn abi(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, SYSTEM_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, SYSTEM_USAGE) {
        return c;
    }
    let mut paths: Vec<&str> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{SYSTEM_USAGE}");
                return 0;
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, SYSTEM_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => paths.push(other),
        }
    }
    if paths.len() != 2 {
        eprintln!("podvm system abi: two paths are required: <object> <libc>");
        eprint!("{SYSTEM_USAGE}");
        return EXIT_CLI_ERROR;
    }
    let read = |p: &str| podvm_enter::abi::Elf::read(p);
    let (object, libc) = match (read(paths[0]), read(paths[1])) {
        (Ok(o), Ok(l)) => (o, l),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("podvm system abi: {e}");
            // ⛔ 2, and it means "could not be measured". A file podvm cannot
            // read is not a file podvm has refused.
            return 2;
        }
    };
    match podvm_enter::abi::admits(&object, &libc) {
        podvm_enter::abi::Verdict::Admitted => {
            println!(
                "admitted: {} may be preloaded into a payload served by {} ({}, {} \
                 imports checked against {} definitions)",
                object.path,
                libc.path,
                libc.flavour().word(),
                object.imported.len(),
                libc.defined.len()
            );
            0
        }
        podvm_enter::abi::Verdict::Refused(why) => {
            // The reason of pod-enter is one sentence with no verdict word
            // (podcrates C335); podvm adds the label and wraps it (C338).
            eprint!("{}", wrap(&format!("refused: {why}"), 76, "  "));
            1
        }
    }
}

/// `text` in lines of at most `width` columns, cut at spaces, each line
/// after the first under `indent`. A word longer than the width stays whole.
pub(crate) fn wrap(text: &str, width: usize, indent: &str) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let lead = if out.is_empty() { "" } else { indent };
        if !line.is_empty() && lead.len() + line.len() + 1 + word.chars().count() > width {
            out.push_str(lead);
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push_str(if out.is_empty() { "" } else { indent });
        out.push_str(&line);
        out.push('\n');
    }
    out
}

mod df_info;
pub use df_info::*;

#[cfg(test)]
mod tests;
