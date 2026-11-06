//! podvm-gate: the record gate as a binary.
//!
//! A behaviour-preserving port of the Python reader half of the work-todo
//! pair. The gate asserts, independently of the writer, that every row in
//! the index names an entry that exists, that every entry has a row, that
//! row and entry status agree, that every count is what the rows say, that
//! every entry carries all ten required fields with a Prove that is a
//! command, and that every citation, link, bare path, ceiling, experiment
//! number, toolchain component, exit-code home, Prove registry reference,
//! closure record, interpose size, export comparison, dev check arm, Prove
//! flag, parity note, printed string, cleanup procedure, perf budget and
//! generated snapshot holds.
//!
//! The reader takes no arguments, and extras are ignored. The exit code is
//! read from this process, unpiped. Exit: 0 everything agrees, 1 something
//! disagrees, 2 could not run.
//!
//! The row and entry-heading matchers live in the grammar module, shared
//! with the counter binary, with one characterization test per grammar.
//! Every other matcher below is hand-rolled over bytes, because the crate
//! carries no dependencies: each function names the expression it mirrors.
//! Two deliberate approximations, recorded rather than silent: the Python
//! matchers for digits and word characters also accept non-ASCII text, and
//! this binary reads ASCII only. Every line the gate reads is prose-rule
//! ASCII, so the two agree on every input the tree can hold. A Python run
//! that crashes on malformed input is reported here as a finding instead.

mod grammar;

use grammar::{parse_entry_heading, parse_index_row, FIELDS, PRIORITIES, STATUSES};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// The corpus and imported methodology are somebody else's text. Their
// internal citations are not this project's to hold. Project-authored
// documents beside those imported directories are checked like every
// other project file.
const CORPUS_PREFIX: &str = "references/";

// The captured trees left the live tree on 2026-11-06, by the operator's
// ruling: each reference is tracked in a GitHub issue instead. This manifest
// lists every file the trees held, with its line count, at the last commit
// that held them. A citation into `references/` resolves against it, so the
// record stays checked in a fresh or shallow clone that has no trees.
const CORPUS_MANIFEST: &str = "references/MANIFEST.tsv";
const VERBATIM_PREFIXES: [&str; 3] = ["docs/conventions/", "docs/methodology/", "docs/security/"];

// The history pages are frozen records: each describes its recorded
// revision, never the current tree, and is never edited. The rename of
// 2026-11-06 (podbox to podvm) moved every crate path they cite, so their
// citations and links are not held against today's tree.
const HISTORY_PREFIX: &str = "docs/history/";

// The imported methodology links resolve today. Holding zero is what stops
// a new dangling link from becoming accepted noise.
const KNOWN_VERBATIM_DANGLING: usize = 0;

// Files this project wrote that are neither markdown nor under the task
// directory, and that carry citations worth resolving.
const SOURCE_SUFFIXES: [&str; 6] = [".rs", ".sh", ".py", ".toml", ".yml", ".yaml"];

// The first of check 14's two exemptions, narrow on purpose. The results
// directory is where a measurement lands, so an entry whose work is to take
// that measurement cites a file that does not exist yet.
const FORWARD_REF_PREFIX: &str = "experiments/results/";

// The second and last of check 14's exemptions, per line rather than per
// file, so it cannot be turned on for a whole document by accident.
const KNOWN_ABSENT: &str = "<!-- known-absent -->";

// The one file allowed to declare the release binary's ceiling, and the
// committed evidence it is checked against. Both are read as text; nothing
// is built, so this runs on a fresh clone.
const CEILING_SCRIPT: &str = "experiments/110-bloat-delta.sh";
const BLOAT_BASELINE: &str = "experiments/results/bloat-baseline.txt";

// The toolchain a cargo build needs is declared once, in the cargo config,
// and CI carries a second declaration of it as a list of bootstrap
// components. Nothing below lists a component: the config names a wrapper,
// the wrapper names the component that installs it.
const CARGO_CONFIG: &str = ".cargo/config.toml";
const WORKFLOWS: &str = ".github/workflows/";

// Docker's exit codes are declared in exactly one file.
const EXIT_CODE_HOME: &str = "crates/podvm-probe/src/exit.rs";

// Each preloaded object holds to the interpose ceiling declared once in
// the interpose build script, per libc, in the committed readings file.
// The export-set comparison lives in the binary that performs it.
const INTERPOSE_BUILD: &str = "scripts/build-interpose.sh";
const INTERPOSE_BUILD_SRC: &str = "crates/podvm-gate/src/interpose_build.rs";
const INTERPOSE_SIZES: &str = "experiments/results/bloat-interpose.txt";

// Where the dev check lives, for the third-state check. T-1555 ported the
// shell driver to the gate crate; the check reads the Rust source, which
// carries the same step-status contract the script did.
const DEVCHECK: &str = "crates/podvm-gate/src/dev/steps.rs";

// The perf harness writes rows; the gate compares them. The comparison
// lives here and nowhere else, so a budget and a reading cannot drift.
const PERF_CEILINGS: &str = "experiments/perf-ceilings.tsv";
const PERF_RESULTS: [&str; 3] = [
    "experiments/results/perf-lane.txt",
    "experiments/results/perf-seeds.tsv",
    "experiments/results/perf-kvm.txt",
];

// The generated snapshot and the script that renders it from source.
const RUNTIME_STATE: &str = "docs/runtime-state.md";
const DOCUMENT_STATE_SCRIPT: &str = "scripts/document-state.py";

// The parity table is the machine-readable contract. The spellings below
// mirror the runtime's boundary: one copy of the rows, and a second verb
// served by the first verb's parser.
const PARITY_RS: &str = "crates/podvm/src/parity.rs";
const PARITY_ROWS_OF: [(&str, &str); 12] = [
    ("create", "run"),
    ("image ls", "images"),
    ("image list", "images"),
    ("image rm", "rmi"),
    ("image remove", "rmi"),
    ("image prune", "prune"),
    ("image tag", "tag"),
    ("image inspect", "inspect"),
    ("image pull", "pull"),
    ("image extract", "extract"),
    ("system info", "info"),
    ("system install-names", "install-names"),
];
// Kept short: the full rows-of table continues with the abi and df verbs.
const PARITY_ROWS_OF_TAIL: [(&str, &str); 2] = [("system abi", "abi"), ("system df", "df")];

// The curated surface the parity table must cover. Every flag here must
// resolve under run and every verb must have a verb row, so a new parser
// flag cannot land without a row and a missing row cannot pass as a
// refusal.
const CURATED_RUN_FLAGS: [&str; 41] = [
    "--attach",
    "--blkio-weight",
    "--cgroup-parent",
    "--cidfile",
    "--cpu-period",
    "--cpu-quota",
    "--cpu-shares",
    "--cpuset-cpus",
    "--detach-keys",
    "--device",
    "--device-cgroup-rule",
    "--disable-content-trust",
    "--dns",
    "--dns-option",
    "--dns-search",
    "--domainname",
    "--env-file",
    "--expose",
    "--gpus",
    "--group-add",
    "--health-cmd",
    "--init",
    "--ipc",
    "--isolation",
    "--label",
    "--link",
    "--log-driver",
    "--log-opt",
    "--mac-address",
    "--mount",
    "--oom-kill-disable",
    "--pid",
    "--pids-limit",
    "--read-only",
    "--runtime",
    "--security-opt",
    "--shm-size",
    "--stop-signal",
    "--stop-timeout",
    "--sysctl",
    "--tmpfs",
];
const CURATED_RUN_FLAGS_TAIL: [&str; 4] = ["--ulimit", "--userns", "--uts", "--volume-driver"];
const CURATED_RUN_FLAGS_LAST: &str = "--volumes-from";
const CURATED_VERBS: [&str; 10] = [
    "manifest",
    "node",
    "plugin",
    "scan",
    "secret",
    "service",
    "stack",
    "trust",
    "checkpoint",
    "config",
];

// The binary prints plain ASCII on every path. Comments stay out of scope.
const ASCII_SCOPES: [&str; 5] = [
    "crates/podvm/src",
    "crates/podvm-probe/src",
    "crates/podvm-image/src",
    "crates/podvm-enter/src",
    "crates/podvm-complete/src",
];

#[path = "gate/text.rs"]
mod text;
use text::*;

#[path = "gate/cites.rs"]
mod cites;
use cites::*;

#[path = "gate/workflows.rs"]
mod workflows;
use workflows::*;

#[path = "gate/prove_parse.rs"]
mod prove_parse;
use prove_parse::*;

#[path = "gate/tree_checks.rs"]
mod tree_checks;
use tree_checks::*;

#[path = "gate/record_checks.rs"]
mod record_checks;
use record_checks::*;

#[path = "gate/prove_checks.rs"]
mod prove_checks;
use prove_checks::*;

#[path = "gate/json_checks.rs"]
mod json_checks;
use json_checks::*;

#[path = "gate/state_checks.rs"]
mod state_checks;
use state_checks::*;

#[path = "gate/run.rs"]
mod run;
use run::*;

fn main() {
    let code = match find_root() {
        None => {
            eprintln!("check-todo: cannot locate the repository root");
            2
        }
        Some(root) => {
            if !root.join("TODO").is_dir() {
                eprintln!("check-todo: TODO/ does not exist");
                2
            } else if !root.join("TODO").join("INDEX.md").is_file() {
                eprintln!("check-todo: TODO/INDEX.md does not exist");
                2
            } else {
                run_gate(&root)
            }
        }
    };
    std::process::exit(code);
}
