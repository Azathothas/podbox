//! podbox-gate: the record gate as a binary.
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
const VERBATIM_PREFIXES: [&str; 3] = ["docs/conventions/", "docs/methodology/", "docs/security/"];

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
const EXIT_CODE_HOME: &str = "crates/podbox-probe/src/exit.rs";

// Each preloaded object holds to the interpose ceiling declared once in
// the interpose build script, per libc, in the committed readings file.
// The export-set comparison lives in the binary that performs it.
const INTERPOSE_BUILD: &str = "scripts/build-interpose.sh";
const INTERPOSE_BUILD_SRC: &str = "crates/podbox-gate/src/interpose_build.rs";
const INTERPOSE_SIZES: &str = "experiments/results/bloat-interpose.txt";

// Where the dev check lives, for the third-state check. T-1555 ported the
// shell driver to the gate crate; the check reads the Rust source, which
// carries the same step-status contract the script did.
const DEVCHECK: &str = "crates/podbox-gate/src/dev.rs";

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
const PARITY_RS: &str = "crates/podbox-cli/src/parity.rs";
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
    "crates/podbox-cli/src",
    "crates/podbox-probe/src",
    "crates/podbox-image/src",
    "crates/podbox-enter/src",
    "crates/podbox-complete/src",
];

const CITE_PREFIXES: [&str; 6] = [
    "references",
    "crates",
    "experiments",
    "scripts",
    "docs",
    "TODO",
];

const UNQUALIFIED_IMAGES: [&str; 15] = [
    "voidlinux/voidlinux-musl",
    "rockylinux/rockylinux",
    "opensuse/leap",
    "library/",
    "golang:",
    "alpine",
    "debian",
    "ubuntu",
    "archlinux",
    "fedora",
    "almalinux",
    "rocky-minimal",
    "rocky",
    "opensuse-leap",
    "voidlinux-musl",
];

const RULE_ANCHORS: [&str; 4] = [
    "Post-task cleanup, after every task",
    "gc --job <id> --apply",
    "gc --apply",
    "experiments/results/",
];

const CLEANUP_FIELDS: [&str; 4] = ["containers", "guest_dirs", "host_dirs", "sessions"];

fn rows_of(verb: &str) -> &str {
    for (alias, under) in PARITY_ROWS_OF.iter().chain(PARITY_ROWS_OF_TAIL.iter()) {
        if *alias == verb {
            return under;
        }
    }
    verb
}

struct Gate {
    errors: Vec<String>,
    seen: BTreeMap<&'static str, u64>,
}

impl Gate {
    fn new() -> Gate {
        let keys = [
            "rows",
            "entries",
            "fields",
            "counts",
            "corpus",
            "todo_citations",
            "todo_links",
            "crossrefs",
            "tree_citations",
            "tree_links",
            "bare_citations",
            "size_ceiling",
            "experiment_numbers",
            "ci_components",
            "exit_codes",
            "prove_registry",
            "closure_records",
            "interpose_sizes",
            "interpose_exports",
            "devcheck_third_state",
            "prove_flags",
            "parity_notes",
            "ascii_output",
            "post_task_cleanup",
            "perf_budget",
        ];
        let mut seen = BTreeMap::new();
        for key in keys {
            seen.insert(key, 0);
        }
        Gate {
            errors: Vec::new(),
            seen: BTreeMap::new(),
        }
        .with_seen(seen)
    }

    fn with_seen(mut self, seen: BTreeMap<&'static str, u64>) -> Gate {
        self.seen = seen;
        self
    }

    fn err(&mut self, where_: &str, msg: String) {
        self.errors.push(format!("{where_}: {msg}"));
    }

    fn bump(&mut self, key: &'static str) {
        *self.seen.entry(key).or_insert(0) += 1;
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn read_text(path: &Path) -> Result<String, String> {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Ok(normalize_newlines(&text)),
            Err(_) => Err("not readable".to_string()),
        },
        Err(_) => Err("not readable".to_string()),
    }
}

fn count_lines_bytes(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    let mut count = data.iter().filter(|b| **b == b'\n').count();
    if data[data.len() - 1] != b'\n' {
        count += 1;
    }
    count
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// Advance one character, so byte-stepping scanners never split one.
fn step_at(line: &str, at: usize) -> usize {
    if at >= line.len() {
        return at + 1;
    }
    at + line[at..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

fn is_task_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 6
        && bytes[0] == b'T'
        && bytes[1] == b'-'
        && bytes[2..6].iter().all(|b| b.is_ascii_digit())
}

// Single-quote a string the way the reference output quotes one.
fn quote_repr(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c < '\u{20}' || c == '\u{7f}') => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

// Format a float the way the reference output formats a ceiling.
fn format_ceiling(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let exp = value.abs().log10().floor() as i32;
    if (-4..15).contains(&exp) {
        let decimals = (5 - exp).max(0) as usize;
        let raw = format!("{:.*}", decimals, value);
        let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
        if trimmed.is_empty() || trimmed == "-0" {
            return "0".to_string();
        }
        return trimmed.to_string();
    }
    let mantissa = value / 10_f64.powi(exp);
    let raw = format!("{mantissa:.5}");
    let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}e{exp:+03}")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

// A citation the gate resolves. Anchored at a top-level directory of the
// repository so that prose mentioning a bare name in the abstract is not
// read as a path.
struct Cite {
    cited: String,
    start: usize,
    end: Option<usize>,
}

fn match_cite_at(line: &str, at: usize) -> Option<(Cite, usize)> {
    let bytes = line.as_bytes();
    if at > 0 {
        let prev = bytes[at - 1];
        if is_word_byte(prev) || prev == b'/' || prev == b'.' || prev == b'-' {
            return None;
        }
    }
    let mut prefix_end = None;
    for prefix in CITE_PREFIXES {
        if line[at..].starts_with(prefix) && line[at + prefix.len()..].starts_with('/') {
            prefix_end = Some(at + prefix.len() + 1);
            break;
        }
    }
    let path_start = prefix_end?;
    let mut run = path_start;
    while run < bytes.len() {
        let b = bytes[run];
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'@' | b'/' | b'-') {
            run += 1;
        } else {
            break;
        }
    }
    if run >= bytes.len() || bytes[run] != b':' || run == path_start {
        return None;
    }
    let colon = run;
    let mut digits = colon + 1;
    while digits < bytes.len() && bytes[digits].is_ascii_digit() {
        digits += 1;
    }
    if digits == colon + 1 {
        return None;
    }
    let mut end_digits = digits;
    let mut range_end = None;
    if end_digits < bytes.len() && bytes[end_digits] == b'-' {
        let mut tail = end_digits + 1;
        while tail < bytes.len() && bytes[tail].is_ascii_digit() {
            tail += 1;
        }
        if tail > end_digits + 1 {
            range_end = Some(line[end_digits + 1..tail].parse::<usize>().unwrap_or(0));
            end_digits = tail;
        }
    }
    if end_digits < bytes.len() {
        let next = bytes[end_digits];
        if is_word_byte(next) || next == b'.' || next == b'-' {
            return None;
        }
    }
    let cited = line[at..colon].to_string();
    let start = line[colon + 1..digits].parse::<usize>().unwrap_or(0);
    Some((
        Cite {
            cited,
            start,
            end: range_end,
        },
        end_digits,
    ))
}

fn find_cites(line: &str) -> Vec<Cite> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < line.len() {
        if let Some((cite, end)) = match_cite_at(line, at) {
            out.push(cite);
            at = end.max(at + 1);
        } else {
            at = step_at(line, at);
        }
    }
    out
}

// A path named in backticks with no line number. The shape the first
// defect took, and a path-and-line matcher cannot see it.
fn find_bares(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at = step_at(line, at);
            continue;
        }
        let mut found = None;
        for prefix in CITE_PREFIXES {
            let head = format!("`{prefix}/");
            if line[at..].starts_with(&head) {
                let mut run = at + head.len();
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric()
                        || matches!(b, b'.' | b'_' | b'+' | b'@' | b'/' | b'-')
                    {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run < bytes.len() && bytes[run] == b'`' {
                    found = Some((line[at + 1..run].to_string(), run + 1));
                }
                break;
            }
        }
        if let Some((cited, next)) = found {
            out.push(cited);
            at = next;
        } else {
            at = step_at(line, at);
        }
    }
    out
}

struct Link {
    href: String,
}

fn find_links(line: &str) -> Vec<Link> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'[' {
            at = step_at(line, at);
            continue;
        }
        let Some(close) = line[at..].find(']') else {
            break;
        };
        let paren = at + close + 1;
        if paren >= bytes.len() || bytes[paren] != b'(' {
            at = step_at(line, at);
            continue;
        }
        let rest = &line[paren + 1..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let segment = &rest[..end];
        if segment.is_empty() || segment.starts_with(')') || segment.starts_with('#') {
            at = paren + 1;
            continue;
        }
        let href = segment.split('#').next().unwrap_or("").trim();
        if href.is_empty() {
            at = paren + 1;
            continue;
        }
        out.push(Link {
            href: href.to_string(),
        });
        at = paren + 1 + end + 1;
    }
    out
}

// Nothing under these may be tracked. The results directory is the
// exception and is the evidence, so it is deliberately not a scratch
// prefix: every scratch directory is a dot directory under experiments.
fn is_scratch(rel: &str) -> bool {
    if let Some(rest) = rel.strip_prefix("experiments/.") {
        return !rest.is_empty() && !rest.starts_with('/') && rest.contains('/');
    }
    if rel.starts_with("target/") {
        return true;
    }
    if let Some(rest) = rel.strip_prefix("crates/") {
        if let Some(slash) = rest.find('/') {
            if &rest[slash..] == "/target/" {
                return true;
            }
            if rest[slash..].starts_with("/target/") {
                return true;
            }
        }
        return false;
    }
    rel.contains("/__pycache__/")
}

// Split text into lines the way the reference reader splits them: a
// trailing newline ends the last line rather than starting a new one.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') && parts.last().is_some_and(|last| last.is_empty()) {
        parts.pop();
    }
    parts
}

// A whole-line declaration of the form NAME digits, blank-separated.
fn match_spaced_decl(line: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(name)?.strip_prefix(' ')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(rest.to_string())
}

// A whole-line declaration of the form NAME=digits, multiline search.
fn match_decl_line(line: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(name)?.strip_prefix('=')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(rest.to_string())
}

fn search_decl(text: &str, name: &str) -> Option<String> {
    split_lines(text)
        .iter()
        .find_map(|line| match_decl_line(line, name))
}

fn search_spaced_decl(text: &str, name: &str) -> Option<String> {
    split_lines(text)
        .iter()
        .find_map(|line| match_spaced_decl(line, name))
}

struct Experiment {
    num: String,
    name: String,
}

fn match_experiment_at(text: &str, at: usize) -> Option<(Experiment, usize)> {
    let rest = text.get(at..)?;
    let after = rest.strip_prefix("experiments/")?;
    let mut digits = 0;
    while after
        .as_bytes()
        .get(digits)
        .is_some_and(|b| b.is_ascii_digit())
    {
        digits += 1;
    }
    if digits == 0 || after.as_bytes().get(digits) != Some(&b'-') {
        return None;
    }
    let num = after[..digits].to_string();
    let name_start = digits + 1;
    let name_bytes = after.as_bytes();
    let mut run = name_start;
    while run < name_bytes.len() {
        let b = name_bytes[run];
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-') {
            run += 1;
        } else {
            break;
        }
    }
    let mut split = run;
    while split > name_start {
        if after[split..].starts_with(".sh") {
            let name = after[name_start..split].to_string();
            if !name.is_empty() {
                let end = at + "experiments/".len() + split + ".sh".len();
                return Some((Experiment { num, name }, end));
            }
            break;
        }
        split -= 1;
    }
    None
}

fn is_experiment_file(rel: &str) -> Option<Experiment> {
    let (exp, end) = match_experiment_at(rel, 0)?;
    if end == rel.len() {
        Some(exp)
    } else {
        None
    }
}

fn find_experiments(text: &str) -> Vec<(Experiment, usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if let Some((exp, end)) = match_experiment_at(text, at) {
            let line = text[..at].matches('\n').count() + 1;
            out.push((exp, line, at));
            at = end.max(at + 1);
        } else {
            at = step_at(text, at);
        }
    }
    out
}

// The toolchain a cargo build needs, derived in two hops and hard-coded
// in neither. The cargo config names a program; that program says which
// bootstrap component installs it.
fn toolchain_wrappers(config: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in config.split('\n') {
        let mut pos = 0;
        let bytes = line.as_bytes();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut matched = None;
        for key in ["CC", "AR", "LD", "CXX"] {
            if line[pos..].starts_with(key) && line[pos + key.len()..].starts_with('_') {
                matched = Some(key.len());
                break;
            }
        }
        let Some(key_len) = matched else {
            continue;
        };
        let mut cursor = pos + key_len + 1;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
        {
            cursor += 1;
        }
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'{' {
            continue;
        }
        let after = &line[cursor..];
        let Some(value_at) = after.find("value") else {
            continue;
        };
        if after[..value_at].contains('}') {
            continue;
        }
        let mut val = cursor + value_at + "value".len();
        while val < bytes.len() && bytes[val].is_ascii_whitespace() {
            val += 1;
        }
        if val >= bytes.len() || bytes[val] != b'=' {
            continue;
        }
        val += 1;
        while val < bytes.len() && bytes[val].is_ascii_whitespace() {
            val += 1;
        }
        if val >= bytes.len() || bytes[val] != b'"' {
            continue;
        }
        val += 1;
        let start = val;
        while val < bytes.len() && bytes[val] != b'"' {
            val += 1;
        }
        if val >= bytes.len() {
            continue;
        }
        out.push(line[start..val].to_string());
    }
    out
}

fn wrapper_components(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("bootstrap-env.sh") {
        let mut pos = at + found + "bootstrap-env.sh".len();
        let bytes = body.as_bytes();
        if pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            let start = pos;
            if pos < bytes.len() && bytes[pos].is_ascii_lowercase() {
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_lowercase()
                        || bytes[pos].is_ascii_digit()
                        || bytes[pos] == b'-')
                {
                    pos += 1;
                }
                out.push(body[start..pos].to_string());
            }
        }
        at = at + found + 1;
    }
    out
}

fn cargo_toolchain_components(root: &Path) -> BTreeMap<String, String> {
    let mut comps = BTreeMap::new();
    let Ok(config) = read_text(&root.join(CARGO_CONFIG)) else {
        return comps;
    };
    for wrapper in toolchain_wrappers(&config) {
        let Ok(body) = read_text(&root.join(&wrapper)) else {
            continue;
        };
        for component in wrapper_components(&body) {
            comps.entry(component).or_insert(wrapper.clone());
        }
    }
    comps
}

fn is_cargo_call(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = text[at..].find("cargo") {
        let pos = at + found;
        let before_ok = pos == 0
            || (!is_word_byte(bytes[pos - 1]) && !matches!(bytes[pos - 1], b'/' | b'.' | b'-'));
        if before_ok {
            let mut cursor = pos + "cargo".len();
            let mut spaces = 0;
            while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
                cursor += 1;
                spaces += 1;
            }
            if spaces > 0 && cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() {
                return true;
            }
        }
        at = pos + 1;
    }
    false
}

fn find_repo_scripts(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'.' && bytes[at + 1] == b'/' {
            let mut matched = None;
            for prefix in ["scripts/", "experiments/"] {
                if text[at + 2..].starts_with(prefix) {
                    matched = Some(prefix.len());
                    break;
                }
            }
            if let Some(prefix_len) = matched {
                let script_start = at + 2;
                let mut run = script_start + prefix_len;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                let mut split = run;
                let mut done = false;
                while split > script_start + prefix_len {
                    if text[split..].starts_with(".sh") {
                        out.push(text[script_start..split + 3].to_string());
                        at = split + 3;
                        done = true;
                        break;
                    }
                    split -= 1;
                }
                if done {
                    continue;
                }
            }
        }
        at = step_at(text, at);
    }
    out.sort();
    out.dedup();
    out
}

fn bootstrap_components(body: &str) -> BTreeSet<String> {
    let mut have = BTreeSet::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("bootstrap-env.sh") {
        let pos = at + found + "bootstrap-env.sh".len();
        let tail: String = body[pos..]
            .chars()
            .take_while(|c| !matches!(c, '\n' | '#' | '|' | '&' | ';'))
            .collect();
        for token in tail.split_whitespace() {
            if !token.starts_with('-') {
                have.insert(token.to_string());
            }
        }
        at = pos;
    }
    have
}

struct WorkflowJob {
    name: String,
    body: String,
}

fn workflow_jobs(text: &str) -> Vec<WorkflowJob> {
    let mut jobs: Vec<WorkflowJob> = Vec::new();
    let mut name: Option<String> = None;
    let mut buf: Vec<String> = Vec::new();
    let mut in_jobs = false;
    for line in text.split('\n') {
        let stripped: String = line.trim_end_matches([' ', '\t', '\r', '\n']).to_string();
        if stripped == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.starts_with("  ") && !line[2..].starts_with(' ') && !line[2..].starts_with('\t') {
            let rest = line[2..].trim_end();
            if let Some(colon) = rest.find(':') {
                let candidate = &rest[..colon];
                if !candidate.is_empty()
                    && candidate
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    && rest[colon + 1..].trim().is_empty()
                {
                    if let Some(active) = name.take() {
                        jobs.push(WorkflowJob {
                            name: active,
                            body: buf.join("\n"),
                        });
                    }
                    name = Some(candidate.to_string());
                    buf = Vec::new();
                    continue;
                }
            }
        }
        if name.is_some() {
            buf.push(line.to_string());
        }
    }
    if let Some(active) = name {
        jobs.push(WorkflowJob {
            name: active,
            body: buf.join("\n"),
        });
    }
    jobs
}

fn uncommented(body: &str) -> String {
    body.split('\n')
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

// A docker exit-code declaration, not a use of one and not a bare number
// in a match arm or a message.
fn find_exit_decls(text: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub const ") else {
            continue;
        };
        if !rest.starts_with("EXIT_") {
            continue;
        }
        let mut pos = "EXIT_".len();
        let bytes = rest.as_bytes();
        while pos < bytes.len() && (bytes[pos].is_ascii_uppercase() || bytes[pos] == b'_') {
            pos += 1;
        }
        if pos == "EXIT_".len() {
            continue;
        }
        let name = rest[..pos].to_string();
        let mut cursor = pos;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b':' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if !rest[cursor..].starts_with("i32") {
            continue;
        }
        cursor += "i32".len();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            continue;
        }
        let value = rest[start..cursor].to_string();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b';' {
            continue;
        }
        out.push((name, value, index + 1));
    }
    out
}

fn match_unqualified_at(line: &str, at: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    if at > 0 {
        let prev = bytes[at - 1];
        if is_word_byte(prev) || matches!(prev, b'/' | b'.' | b':' | b'-') {
            return None;
        }
    }
    for image in UNQUALIFIED_IMAGES {
        let (head, tail) = if image.ends_with('/') || image.ends_with(':') {
            let mut run = at + image.len();
            if image.ends_with('/') {
                let start = run;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run == start {
                    continue;
                }
            } else {
                let start = run;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run == start {
                    continue;
                }
            }
            (image, run)
        } else {
            if !line[at..].starts_with(image) {
                continue;
            }
            (image, at + image.len())
        };
        if !line[at..].starts_with(head) {
            continue;
        }
        let mut end = tail;
        if end < bytes.len() && bytes[end] == b':' {
            let mut tag = end + 1;
            while tag < bytes.len() {
                let b = bytes[tag];
                if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                    tag += 1;
                } else {
                    break;
                }
            }
            if tag > end + 1 {
                end = tag;
            }
        }
        if end < bytes.len() {
            let next = bytes[end];
            if is_word_byte(next) || matches!(next, b'/' | b'.' | b':' | b'-') {
                continue;
            }
        }
        return Some((line[at..end].to_string(), end));
    }
    None
}

fn search_unqualified(line: &str, from: usize) -> Option<(String, usize, usize)> {
    let mut at = from;
    while at < line.len() {
        if let Some((matched, end)) = match_unqualified_at(line, at) {
            return Some((matched, at, end));
        }
        at = step_at(line, at);
    }
    None
}

fn search_docker_io(line: &str) -> Option<String> {
    if let Some(found) = line.find("docker.io/") {
        let mut end = found + "docker.io/".len();
        let bytes = line.as_bytes();
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }
        return Some(line[found..end].to_string());
    }
    None
}
struct ParityRow {
    verb: String,
    spellings: Option<String>,
    status: String,
}

struct ParityRowMatch {
    row: ParityRow,
    line: usize,
    offset: usize,
}

fn find_parity_rows(text: &str) -> Vec<ParityRowMatch> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = text[at..].find("Row") {
        let mut pos = at + found + "Row".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'{' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("verb:") {
            at = pos;
            continue;
        }
        pos += "verb:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'"' {
            at = pos;
            continue;
        }
        pos += 1;
        let verb_start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let verb = text[verb_start..pos].to_string();
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("flag:") {
            at = pos;
            continue;
        }
        pos += "flag:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let spellings;
        if text[pos..].starts_with("Option::None") {
            spellings = None;
            pos += "Option::None".len();
        } else if text[pos..].starts_with("Some(\"") {
            pos += "Some(\"".len();
            let list_start = pos;
            while pos < bytes.len() && bytes[pos] != b'"' {
                pos += 1;
            }
            if pos >= bytes.len() {
                break;
            }
            spellings = Some(text[list_start..pos].to_string());
            pos += 1;
            if !text[pos..].starts_with(')') {
                at = pos;
                continue;
            }
            pos += 1;
        } else {
            at = pos;
            continue;
        }
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("status:") {
            at = pos;
            continue;
        }
        pos += "status:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let status_start = pos;
        while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
            pos += 1;
        }
        if pos == status_start {
            at = pos;
            continue;
        }
        let status = text[status_start..pos].to_string();
        let line = text[..at + found].matches('\n').count() + 1;
        out.push(ParityRowMatch {
            row: ParityRow {
                verb,
                spellings,
                status,
            },
            line,
            offset: at + found,
        });
        at = pos;
    }
    out
}

struct ParityAdmission {
    admitted: BTreeMap<String, BTreeSet<String>>,
    refused_verbs: BTreeSet<String>,
    spellings: BTreeMap<String, BTreeSet<String>>,
}

fn parity_admission(text: &str) -> ParityAdmission {
    let mut admitted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut refused_verbs = BTreeSet::new();
    let mut spellings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row_match in find_parity_rows(text) {
        let row = row_match.row;
        if let Some(list) = row.spellings {
            for spelling in list.split(',') {
                spellings
                    .entry(rows_of(&row.verb).to_string())
                    .or_default()
                    .insert(spelling.trim().to_string());
            }
            if row.status == "NoneStatus" {
                continue;
            }
            for spelling in list.split(',') {
                admitted
                    .entry(rows_of(&row.verb).to_string())
                    .or_default()
                    .insert(spelling.trim().to_string());
            }
        } else if row.status == "NoneStatus" {
            refused_verbs.insert(row.verb.clone());
        }
    }
    ParityAdmission {
        admitted,
        refused_verbs,
        spellings,
    }
}

fn parity_cluster_values(text: &str) -> BTreeSet<(String, char)> {
    let mut out = BTreeSet::new();
    let Some(decl) = text.find("pub const CLUSTER_VALUES") else {
        return out;
    };
    let Some(close) = text[decl..].find("];") else {
        return out;
    };
    let slice = &text[decl..decl + close];
    let bytes = slice.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'(' {
            at += 1;
            continue;
        }
        let mut pos = at + 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'"' {
            at += 1;
            continue;
        }
        pos += 1;
        let verb_start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let verb = slice[verb_start..pos].to_string();
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at += 1;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos + 2 >= bytes.len()
            || bytes[pos] != b'\''
            || !bytes[pos + 1].is_ascii_alphanumeric()
            || bytes[pos + 2] != b'\''
        {
            at += 1;
            continue;
        }
        let member = bytes[pos + 1] as char;
        pos += 3;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b')' {
            at += 1;
            continue;
        }
        out.insert((verb, member));
        at = pos + 1;
    }
    out
}

fn parity_cluster_boundary(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(decl) = text.find("const CLUSTER_BOUNDARY") else {
        return out;
    };
    let Some(close) = text[decl..].find("];") else {
        return out;
    };
    let slice = &text[decl..decl + close];
    let bytes = slice.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'"' {
            at += 1;
            continue;
        }
        let mut pos = at + 1;
        while pos < bytes.len() && bytes[pos].is_ascii_lowercase() {
            pos += 1;
        }
        if pos > at + 1 && pos < bytes.len() && bytes[pos] == b'"' {
            out.insert(slice[at + 1..pos].to_string());
            at = pos + 1;
        } else {
            at += 1;
        }
    }
    out
}

// Tokenise a shell command the way the reference port does: quoting keeps
// a template one value, and a line that does not parse that way falls back
// to a plain split, which still keeps every dash-token intact.
fn shlex_split(cmd: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for ch in cmd.chars() {
        if escaped {
            if ch != '\n' {
                cur.push(ch);
                started = true;
            }
            escaped = false;
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                cur.push(ch);
                started = true;
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            } else if ch == '\\' {
                escaped = true;
            } else {
                cur.push(ch);
                started = true;
            }
            continue;
        }
        match ch {
            '\\' => {
                escaped = true;
            }
            '\'' => {
                in_single = true;
                started = true;
            }
            '"' => {
                in_double = true;
                started = true;
            }
            ' ' | '\t' | '\r' | '\n' => {
                if started {
                    tokens.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            _ => {
                cur.push(ch);
                started = true;
            }
        }
    }
    if escaped || in_single || in_double {
        return None;
    }
    if started {
        tokens.push(cur);
    }
    Some(tokens)
}

struct ProveCommand {
    verb: String,
    flags: Vec<String>,
    bad: Option<String>,
}

fn match_prove_inv(line: &str, at: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    if !line[at..].starts_with("podbox") {
        return None;
    }
    let mut pos = at + "podbox".len();
    let mut spaces = 0;
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n') {
        pos += 1;
        spaces += 1;
    }
    if spaces == 0 {
        return None;
    }
    for head in ["image", "system"] {
        if line[pos..].starts_with(head) {
            let mut sub = pos + head.len();
            let mut sub_spaces = 0;
            while sub < bytes.len() && matches!(bytes[sub], b' ' | b'\t' | b'\r' | b'\n') {
                sub += 1;
                sub_spaces += 1;
            }
            if sub_spaces > 0 && sub < bytes.len() && bytes[sub].is_ascii_alphabetic() {
                let mut end = sub + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric()
                        || bytes[end] == b'_'
                        || bytes[end] == b'-')
                {
                    end += 1;
                }
                return Some((line[pos..end].to_string(), end));
            }
        }
    }
    if pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
        let mut end = pos + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        {
            end += 1;
        }
        return Some((line[pos..end].to_string(), end));
    }
    None
}

fn prove_term_at(tail: &str) -> usize {
    let bytes = tail.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'|' | b';' | b'`' | b'&' => return pos,
            b'$' if pos + 1 < bytes.len() && bytes[pos + 1] == b'(' => {
                return pos;
            }
            _ => pos += 1,
        }
    }
    bytes.len()
}

fn prove_commands(
    block: &[String],
    spellings: &BTreeMap<String, BTreeSet<String>>,
    cluster_values: &BTreeSet<(String, char)>,
    boundary: &BTreeSet<String>,
) -> Vec<ProveCommand> {
    let mut out = Vec::new();
    for line in block {
        let mut at = 0;
        while at < line.len() {
            let Some(found) = line[at..].find("podbox") else {
                break;
            };
            let pos = at + found;
            let Some((verb, end)) = match_prove_inv(line, pos) else {
                at = pos + 1;
                continue;
            };
            let before = line[..pos].trim_end();
            if before.ends_with('!') {
                at = end;
                continue;
            }
            let under = rows_of(&verb).to_string();
            let bounded = boundary.contains(&under);
            let tail = &line[end..];
            let term = prove_term_at(tail);
            let cmd = &tail[..term];
            let tokens = shlex_split(cmd)
                .unwrap_or_else(|| cmd.split_whitespace().map(str::to_string).collect());
            let mut flags = Vec::new();
            let mut bad = None;
            let mut prev_dash = !bounded;
            for tok in &tokens {
                if tok.starts_with('-') && tok.chars().count() > 1 {
                    let body: String = tok.chars().skip(1).collect();
                    let first = body.chars().next().unwrap_or('\0');
                    if tok.chars().count() > 2
                        && !tok.starts_with("--")
                        && first.is_alphanumeric()
                        && body.is_ascii()
                    {
                        for member in body.chars() {
                            let one = format!("-{member}");
                            if cluster_values.contains(&(under.clone(), member)) {
                                flags.push(one);
                                break;
                            }
                            let named = spellings.get(&under).is_some_and(|set| set.contains(&one));
                            if !member.is_alphanumeric() || !named {
                                bad = Some(one);
                                break;
                            }
                            flags.push(one);
                        }
                        if bad.is_some() {
                            break;
                        }
                        prev_dash = false;
                        continue;
                    }
                    let flag = tok.split('=').next().unwrap_or(tok).to_string();
                    flags.push(flag);
                    prev_dash = true;
                } else if prev_dash {
                    prev_dash = false;
                } else if bounded {
                    break;
                }
            }
            out.push(ProveCommand { verb, flags, bad });
            at = end;
        }
    }
    out
}
enum RunError {
    Spawn,
    Timeout,
}

fn run_with_timeout(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<(Option<i32>, String, String), RunError> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RunError::Spawn)?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code();
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                return Ok((code, stdout, stderr));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(RunError::Spawn),
        }
    }
}

fn which(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        for extension in ["exe", "bat", "cmd"] {
            let with_ext = dir.join(format!("{program}.{extension}"));
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }
    }
    None
}

fn tracked_files(root: &Path) -> Option<BTreeSet<String>> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "ls-files", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut files = BTreeSet::new();
    for part in output.stdout.split(|b| *b == 0) {
        if part.is_empty() {
            continue;
        }
        files.insert(String::from_utf8_lossy(part).into_owned());
    }
    Some(files)
}

fn is_ours(rel: &str) -> bool {
    if rel.starts_with(CORPUS_PREFIX) {
        return false;
    }
    for prefix in VERBATIM_PREFIXES {
        if rel.starts_with(prefix) {
            return false;
        }
    }
    true
}

fn has_source_suffix(rel: &str) -> bool {
    SOURCE_SUFFIXES.iter().any(|suffix| rel.ends_with(suffix))
}

// The first word of a status value, stars stripped, exactly as the
// reference reader takes it. A value with no word is invalid rather than
// a crash.
fn status_word(value: &str) -> Option<String> {
    let stripped = value.trim_matches('*');
    stripped.split_whitespace().next().map(str::to_string)
}

fn find_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    for line in body.split('\n') {
        let Some(rest) = line.strip_prefix(field) else {
            continue;
        };
        if !rest.starts_with(':') {
            continue;
        }
        let after = &rest[1..];
        let spaces = after.len() - after.trim_start_matches(' ').len();
        if spaces == 0 {
            continue;
        }
        let value = &after[spaces..];
        if value.is_empty() || value.starts_with(' ') {
            continue;
        }
        let _ = value;
        return Some(after.trim_start_matches(' ').trim_end());
    }
    None
}

struct Entry {
    file: String,
    line: usize,
    title: String,
    body: String,
}

fn contains_number_with_boundaries(line: &str, number: &str) -> bool {
    if number.is_empty() {
        return false;
    }
    let bytes = line.as_bytes();
    let needle = number.as_bytes();
    let mut at = 0;
    while at + needle.len() <= bytes.len() {
        if &bytes[at..at + needle.len()] == needle {
            let before_ok = at == 0 || !bytes[at - 1].is_ascii_digit();
            let after_ok =
                at + needle.len() == bytes.len() || !bytes[at + needle.len()].is_ascii_digit();
            if before_ok && after_ok {
                return true;
            }
        }
        at += 1;
    }
    false
}

fn check_tree(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let mut dangling_verbatim = 0;
    for rel in files.iter() {
        if rel.starts_with(CORPUS_PREFIX) {
            continue;
        }
        let ours = is_ours(rel);
        let is_md = rel.ends_with(".md");
        if !is_md && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let mut target_cache: BTreeMap<String, Option<usize>> = BTreeMap::new();
        for (index, line) in text.split('\n').enumerate() {
            let n = index + 1;
            if ours {
                for cite in find_cites(line) {
                    if rel.starts_with("TODO/") {
                        continue;
                    }
                    gate.bump("tree_citations");
                    let where_ = format!("{rel}:{n}");
                    let target = root.join(&cite.cited);
                    if !target.is_file() {
                        gate.err(
                            &where_,
                            format!(
                                "cites {}:{}, and that file does not exist",
                                cite.cited, cite.start
                            ),
                        );
                        continue;
                    }
                    if !files.contains(&cite.cited) {
                        gate.err(
                            &where_,
                            format!(
                                "cites {}, which exists here and is NOT \
                                 tracked by git, so a fresh clone does not have it",
                                cite.cited
                            ),
                        );
                        continue;
                    }
                    let count = *target_cache.entry(cite.cited.clone()).or_insert_with(|| {
                        fs::read(&target).ok().map(|data| count_lines_bytes(&data))
                    });
                    let Some(count) = count else {
                        continue;
                    };
                    let last = cite.end.unwrap_or(cite.start);
                    if last > count {
                        gate.err(
                            &where_,
                            format!(
                                "cites {}:{}, and that file has {count} lines",
                                cite.cited, cite.start
                            ),
                        );
                    }
                }
            }
            if ours && !line.contains(KNOWN_ABSENT) {
                for cited in find_bares(line) {
                    gate.bump("bare_citations");
                    if cited.starts_with(FORWARD_REF_PREFIX) {
                        continue;
                    }
                    let stem = cited.trim_end_matches('/');
                    if files.contains(stem) {
                        continue;
                    }
                    let stem_slash = format!("{stem}/");
                    if files.iter().any(|f| f.starts_with(&stem_slash)) {
                        continue;
                    }
                    gate.err(
                        &format!("{rel}:{n}"),
                        format!(
                            "names `{cited}`, and git tracks no such file or \
                             directory. A citation that resolves on one disk and \
                             not in a fresh clone is the defect this check exists \
                             for."
                        ),
                    );
                }
            }
            if !is_md {
                continue;
            }
            for link in find_links(line) {
                let mut href = link.href;
                if href.starts_with("http://")
                    || href.starts_with("https://")
                    || href.starts_with("mailto:")
                {
                    continue;
                }
                if let Some(hash) = href.find('#') {
                    href = href[..hash].to_string();
                }
                href = href.trim().to_string();
                if href.is_empty() {
                    continue;
                }
                let dir = Path::new(rel).parent().unwrap_or(Path::new(""));
                let target = lexical_normalize(&dir.join(&href));
                let exists = root.join(&target).exists();
                if ours {
                    if rel.starts_with("TODO/") {
                        continue;
                    }
                    gate.bump("tree_links");
                    if !exists {
                        gate.err(
                            &format!("{rel}:{n}"),
                            format!("link target {href} does not resolve"),
                        );
                    } else {
                        let target_str = target.to_string_lossy().replace('\\', "/");
                        if root.join(&target).is_file() && !files.contains(&target_str) {
                            gate.err(
                                &format!("{rel}:{n}"),
                                format!(
                                    "links to {href}, which exists here and is NOT \
                                     tracked by git"
                                ),
                            );
                        }
                    }
                } else if !exists {
                    dangling_verbatim += 1;
                }
            }
        }
    }
    if dangling_verbatim != KNOWN_VERBATIM_DANGLING {
        gate.err(
            "docs/",
            format!(
                "{dangling_verbatim} dangling link(s) in the imported \
                 methodology; expected {KNOWN_VERBATIM_DANGLING}. \
                 Reconcile the import or provide the missing target."
            ),
        );
    }
}
fn check_size_ceiling(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    if !files.contains(CEILING_SCRIPT) {
        gate.err(
            CEILING_SCRIPT,
            "does not exist or is not tracked, so the release \
             binary has no declared ceiling. TODO/deps.md T-0910."
                .to_string(),
        );
        return;
    }
    gate.bump("size_ceiling");
    let Ok(script) = read_text(&root.join(CEILING_SCRIPT)) else {
        gate.err(
            CEILING_SCRIPT,
            "declares no `CEILING_BYTES=<n>` line. That line is \
             the ceiling's one home; without it every other file \
             naming a size is unanchored."
                .to_string(),
        );
        return;
    };
    let Some(ceiling) = search_decl(&script, "CEILING_BYTES") else {
        gate.err(
            CEILING_SCRIPT,
            "declares no `CEILING_BYTES=<n>` line. That line is \
             the ceiling's one home; without it every other file \
             naming a size is unanchored."
                .to_string(),
        );
        return;
    };

    for rel in files.iter() {
        if rel == CEILING_SCRIPT || !is_ours(rel) {
            continue;
        }
        if !rel.ends_with(".md") && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        gate.bump("size_ceiling");
        for (index, line) in text.split('\n').enumerate() {
            if contains_number_with_boundaries(line, &ceiling) {
                gate.err(
                    &format!("{}:{}", rel, index + 1),
                    format!(
                        "names the binary size ceiling {ceiling} itself. It is \
                         declared in {CEILING_SCRIPT} and nowhere else; call that \
                         script instead of copying its number."
                    ),
                );
            }
        }
    }

    if !files.contains(BLOAT_BASELINE) {
        gate.err(
            BLOAT_BASELINE,
            "is not tracked. TODO/deps.md T-0910: without a committed baseline \
             there is no `before`, and a dependency that lands without a \
             before-and-after number has not landed. Take it with \
             `./experiments/110-bloat-delta.sh baseline`."
                .to_string(),
        );
        return;
    }
    gate.bump("size_ceiling");
    let Ok(baseline) = read_text(&root.join(BLOAT_BASELINE)) else {
        gate.err(
            BLOAT_BASELINE,
            "carries no `total_bytes <n>` line, so the baseline \
             cannot be compared with anything."
                .to_string(),
        );
        return;
    };
    if search_spaced_decl(&baseline, "total_bytes").is_none() {
        gate.err(
            BLOAT_BASELINE,
            "carries no `total_bytes <n>` line, so the baseline \
             cannot be compared with anything."
                .to_string(),
        );
        return;
    }

    let limit: u64 = ceiling.parse().unwrap_or(u64::MAX);
    let mut readings: Vec<&String> = files
        .iter()
        .filter(|f| f.starts_with("experiments/results/bloat-") && f.ends_with(".txt"))
        .collect();
    readings.sort();
    for rel in readings {
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let Some(found) = search_spaced_decl(&text, "total_bytes") else {
            continue;
        };
        gate.bump("size_ceiling");
        let total: u64 = found.parse().unwrap_or(0);
        if total >= limit {
            gate.err(
                rel,
                format!(
                    "records total_bytes {total}, which is at or over the ceiling \
                     of {limit} declared in {CEILING_SCRIPT}. Raise the ceiling \
                     deliberately, with the delta that justifies it committed \
                     beside the change."
                ),
            );
        }
    }
}

fn check_experiment_numbers(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let mut numbers: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut record = |num: String, name: String, where_: String| {
        gate.bump("experiment_numbers");
        numbers
            .entry(num)
            .or_default()
            .entry(name)
            .or_insert(where_);
    };
    for rel in files.iter() {
        if let Some(exp) = is_experiment_file(rel) {
            record(exp.num, exp.name, rel.clone());
        }
    }
    for rel in files.iter() {
        if !is_ours(rel) {
            continue;
        }
        if !rel.ends_with(".md") && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let mut skipped = BTreeSet::new();
        for (index, line) in split_lines(&text).into_iter().enumerate() {
            if line.contains(KNOWN_ABSENT) {
                skipped.insert(index + 1);
            }
        }
        for (exp, line, _) in find_experiments(&text) {
            if skipped.contains(&line) {
                continue;
            }
            record(exp.num, exp.name, format!("{rel}:{line}"));
        }
    }
    let mut nums: Vec<(&String, &BTreeMap<String, String>)> = numbers.iter().collect();
    nums.sort_by_key(|(num, _)| num.parse::<u64>().unwrap_or(u64::MAX));
    for (num, names) in nums {
        if names.len() < 2 {
            continue;
        }
        let listed: Vec<String> = names
            .iter()
            .map(|(name, where_)| format!("`{num}-{name}.sh` ({where_})"))
            .collect();
        gate.err(
            &format!("experiments/{num}-"),
            format!(
                "experiment number {num} carries {} names: {}. \
                 experiments/README.md rules that a number is never reused, \
                 because a citation of it has to keep meaning what it meant. \
                 Give the new one a free number. TODO/gate.md T-1205.",
                names.len(),
                listed.join(", ")
            ),
        );
    }
}
fn check_ci_components(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let required = cargo_toolchain_components(root);
    let mut workflows: Vec<&String> = files.iter().filter(|f| f.starts_with(WORKFLOWS)).collect();
    workflows.sort();
    for rel in workflows {
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let jobs = workflow_jobs(&text);
        let mut names: Vec<&WorkflowJob> = jobs.iter().collect();
        names.sort_by(|a, b| a.name.cmp(&b.name));
        for job in names {
            let body = uncommented(&job.body);
            let mut runs_cargo = is_cargo_call(&body);
            let mut via = None;
            for script in find_repo_scripts(&body) {
                if !files.contains(&script) {
                    continue;
                }
                let Ok(script_body) = read_text(&root.join(&script)) else {
                    continue;
                };
                if is_cargo_call(&uncommented(&script_body)) {
                    runs_cargo = true;
                    via = Some(script);
                    break;
                }
            }
            if !runs_cargo {
                continue;
            }
            let have = bootstrap_components(&body);
            let through = match &via {
                Some(script) if !is_cargo_call(&body) => {
                    format!(" (through `{script}`)")
                }
                _ => String::new(),
            };
            let mut comps: Vec<&String> = required.keys().collect();
            comps.sort();
            for comp in comps {
                gate.bump("ci_components");
                if have.contains(comp) {
                    continue;
                }
                gate.err(
                    rel,
                    format!(
                        "job `{}` runs cargo{through} and its \
                         `bootstrap-env.sh` list does not carry `{comp}`. \
                         `{CARGO_CONFIG}` points a cargo build at \
                         `{}`, which says it is installed by \
                         `bootstrap-env.sh {comp}`, so the job dies inside a \
                         dependency's build script rather than on its own step. \
                         TODO/gate.md T-1206.",
                        job.name, required[comp]
                    ),
                );
            }
        }
    }
}

fn check_exit_codes(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let mut sources: Vec<&String> = files
        .iter()
        .filter(|f| f.starts_with("crates/") && f.ends_with(".rs"))
        .collect();
    sources.sort();
    for rel in sources {
        if rel == EXIT_CODE_HOME {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        for (name, value, line) in find_exit_decls(&text) {
            gate.bump("exit_codes");
            gate.err(
                &format!("{rel}:{line}"),
                format!(
                    "declares `{name} = {value}` and `{EXIT_CODE_HOME}` \
                     already holds docker's exit codes. A second declaration is how \
                     two verbs of one binary came to disagree about what a flag \
                     error is: `pub use podbox_probe::exit::{name};` is the \
                     way to have it. TODO/cli.md T-0802."
                ),
            );
        }
    }
    match read_text(&root.join(EXIT_CODE_HOME)) {
        Ok(home) => {
            let count = find_exit_decls(&home).len() as u64;
            *gate.seen.entry("exit_codes").or_insert(0) += count;
        }
        Err(_) => {
            gate.err(
                EXIT_CODE_HOME,
                "is where docker's exit codes live and it is not readable. \
                 TODO/cli.md T-0802."
                    .to_string(),
            );
        }
    }
}

fn entry_heads(lines: &[String]) -> Vec<(usize, String)> {
    let mut heads = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.starts_with("### ") {
            continue;
        }
        let rest = &line["### ".len()..];
        if let Some(id) = rest.get(..6) {
            if is_task_id(id) {
                heads.push((index, id.to_string()));
            }
        }
    }
    heads
}

fn prove_block(lines: &[String], start: usize, end: usize) -> usize {
    let mut j = start + 1;
    while j < end && (lines[j].starts_with(' ') || lines[j].starts_with('\t')) {
        j += 1;
    }
    j
}

fn check_prove_registry(gate: &mut Gate, todo: &Path) {
    let Ok(names) = todo_names(todo) else {
        return;
    };
    for name in names {
        let Ok(text) = read_text(&todo.join(&name)) else {
            continue;
        };
        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        let heads = entry_heads(&lines);
        for (position, (line_no, tid)) in heads.iter().enumerate() {
            let end = heads
                .get(position + 1)
                .map(|(next, _)| *next)
                .unwrap_or(lines.len());
            let mut start = None;
            for (offset, line) in lines.iter().enumerate().take(end).skip(*line_no) {
                if line.starts_with("Prove:") {
                    start = Some(offset);
                    break;
                }
            }
            let Some(start) = start else {
                continue;
            };
            let j = prove_block(&lines, start, end);
            gate.bump("prove_registry");
            let block = &lines[start..j];
            if block
                .iter()
                .any(|line| line.contains("experiments/results/across/"))
            {
                continue;
            }
            for (offset, line) in block.iter().enumerate() {
                if line.contains(KNOWN_ABSENT) {
                    continue;
                }
                let where_ = format!("TODO/{name}:{}", start + offset + 1);
                if let Some(matched) = search_docker_io(line) {
                    gate.err(
                        &where_,
                        format!(
                            "({tid}) Prove names `{matched}`, which pulls \
                             from Docker Hub. Every Prove reference is a row \
                             of DISTRO_ROWS_M5 in \
                             `scripts/common/distro-matrix.sh`, named by its \
                             fully qualified reference. TODO/gate.md T-1209."
                        ),
                    );
                    continue;
                }
                let mut from = 0;
                while let Some((matched, matched_at, matched_end)) = search_unqualified(line, from)
                {
                    if line[..matched_at].trim_end().ends_with(".sh") {
                        from = matched_end;
                        continue;
                    }
                    gate.err(
                        &where_,
                        format!(
                            "({tid}) Prove names an unqualified image \
                             reference `{matched}`, which resolves through \
                             the engine's shortname aliases to Docker Hub. \
                             Every Prove reference is a row of DISTRO_ROWS_M5 \
                             in `scripts/common/distro-matrix.sh`, named by \
                             its fully qualified reference. \
                             TODO/gate.md T-1209."
                        ),
                    );
                    break;
                }
            }
        }
    }
}
fn todo_names(todo: &Path) -> Result<Vec<String>, ()> {
    let entries = fs::read_dir(todo).map_err(|_| ())?;
    let mut names = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".md") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn check_closure_records(gate: &mut Gate, entries: &BTreeMap<String, Entry>) {
    for (tid, entry) in entries.iter() {
        let Some(status_line) = find_field(&entry.body, "Status") else {
            continue;
        };
        let Some(word) = status_word(status_line) else {
            continue;
        };
        if word.split_whitespace().next().unwrap_or("") != "done" {
            continue;
        }
        gate.bump("closure_records");
        let where_ = format!("TODO/{}:{}", entry.file, entry.line);
        let lines: Vec<&str> = entry.body.split('\n').collect();
        let mut start = None;
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with("Prove: ") {
                start = Some(index);
                break;
            }
        }
        let Some(start) = start else {
            continue;
        };
        let mut j = start + 1;
        while j < lines.len()
            && (lines[j].trim().is_empty()
                || lines[j].starts_with(' ')
                || lines[j].starts_with('\t'))
        {
            j += 1;
        }
        if j >= lines.len() || !lines[j].starts_with("**Done") {
            gate.err(
                &where_,
                format!(
                    "({tid}) closes without a recorded run: the first unindented \
                     line after Prove does not open with `**Done`. \
                     TODO/gate.md T-1208."
                ),
            );
        }
    }
}

fn check_interpose_sizes(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    if !files.contains(INTERPOSE_BUILD) {
        gate.err(
            INTERPOSE_BUILD,
            "is where the interpose ceiling lives and it is \
             not tracked. TODO/gate.md T-1207."
                .to_string(),
        );
        return;
    }
    let Ok(build) = read_text(&root.join(INTERPOSE_BUILD)) else {
        gate.err(
            INTERPOSE_BUILD,
            "declares no `INTERPOSE_CEILING_BYTES=<n>` line. \
             That line is the ceiling's one home; without it \
             every other file naming a size is unanchored."
                .to_string(),
        );
        return;
    };
    let Some(ceiling) = search_decl(&build, "INTERPOSE_CEILING_BYTES") else {
        gate.err(
            INTERPOSE_BUILD,
            "declares no `INTERPOSE_CEILING_BYTES=<n>` line. \
             That line is the ceiling's one home; without it \
             every other file naming a size is unanchored."
                .to_string(),
        );
        return;
    };
    gate.bump("interpose_sizes");

    for rel in files.iter() {
        if rel == INTERPOSE_BUILD || !is_ours(rel) {
            continue;
        }
        if !rel.ends_with(".md") && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        gate.bump("interpose_sizes");
        for (index, line) in text.split('\n').enumerate() {
            if contains_number_with_boundaries(line, &ceiling) {
                gate.err(
                    &format!("{}:{}", rel, index + 1),
                    format!(
                        "names the interpose ceiling {ceiling} itself. It is \
                         declared in {INTERPOSE_BUILD} and nowhere else."
                    ),
                );
            }
        }
    }

    if !files.contains(INTERPOSE_SIZES) {
        gate.err(
            INTERPOSE_SIZES,
            "is not tracked. TODO/gate.md T-1207 item 3: without a committed \
             per-libc reading the objects' growth hides in the binary \
             total's headroom. Take it with \
             `./experiments/110-bloat-delta.sh interpose` after \
             `./scripts/build-interpose.sh`."
                .to_string(),
        );
        return;
    }
    gate.bump("interpose_sizes");
    let Ok(text) = read_text(&root.join(INTERPOSE_SIZES)) else {
        gate.err(
            INTERPOSE_SIZES,
            "carries no `interpose_gnu_bytes <n>` line, so the \
             gnu object's size is not held."
                .to_string(),
        );
        return;
    };
    let mut got: BTreeMap<String, String> = BTreeMap::new();
    for line in text.split('\n') {
        for which in ["gnu", "musl"] {
            let head = format!("interpose_{which}_bytes ");
            if let Some(rest) = line.strip_prefix(&head) {
                if rest == "absent"
                    || (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
                {
                    got.insert(which.to_string(), rest.to_string());
                }
            }
        }
    }
    for which in ["gnu", "musl"] {
        let Some(value) = got.get(which) else {
            gate.err(
                INTERPOSE_SIZES,
                format!(
                    "carries no `interpose_{which}_bytes <n>` line, so the \
                     {which} object's size is not held."
                ),
            );
            continue;
        };
        gate.bump("interpose_sizes");
        if value == "absent" {
            gate.err(
                INTERPOSE_SIZES,
                format!(
                    "records `interpose_{which}_bytes absent`: the objects were \
                     not built where the reading was taken. Re-take it after \
                     `./scripts/build-interpose.sh`."
                ),
            );
        } else {
            let size: u64 = value.parse().unwrap_or(0);
            let limit: u64 = ceiling.parse().unwrap_or(u64::MAX);
            if size >= limit {
                gate.err(
                    INTERPOSE_SIZES,
                    format!(
                        "records interpose_{which}_bytes {value}, which is at \
                         or over the interpose ceiling of {ceiling} declared in \
                         {INTERPOSE_BUILD}."
                    ),
                );
            }
        }
    }
}
fn check_interpose_exports(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    if !files.contains(INTERPOSE_BUILD_SRC) {
        gate.err(
            INTERPOSE_BUILD_SRC,
            "is not tracked, so nothing holds the export \
             comparison. TODO/gate.md T-1207."
                .to_string(),
        );
        return;
    }
    let Ok(text) = read_text(&root.join(INTERPOSE_BUILD_SRC)) else {
        gate.err(
            INTERPOSE_BUILD_SRC,
            "is not readable. TODO/gate.md T-1207.".to_string(),
        );
        return;
    };
    gate.bump("interpose_exports");
    if !text.contains("interpose.map") || !text.contains("nm -D --defined-only") {
        gate.err(
            INTERPOSE_BUILD_SRC,
            "carries no export-set comparison against interpose.map. \
             TODO/gate.md T-1207 item 2: without it a link that stopped \
             applying the version script produces a working object and \
             exits 0."
                .to_string(),
        );
    }
}

fn check_devcheck_third_state(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    if !files.contains(DEVCHECK) {
        gate.err(DEVCHECK, "is not tracked. TODO/gate.md T-1207.".to_string());
        return;
    }
    let Ok(text) = read_text(&root.join(DEVCHECK)) else {
        gate.err(
            DEVCHECK,
            "is not readable. TODO/gate.md T-1207.".to_string(),
        );
        return;
    };
    gate.bump("devcheck_third_state");
    // The port keeps the script's contract in one `match` on the step's own
    // status: 0 passes, 2 skips, anything else fails. The names below are
    // the contract, not an implementation detail: a step that could not run
    // must read as SKIP, never as FAILED.
    let Some(head) = text.find("match step_rc") else {
        gate.err(
            DEVCHECK,
            "runs its check steps without reading each step's own status. \
             TODO/gate.md T-1207 item 4."
                .to_string(),
        );
        return;
    };
    let tail = &text[head..];
    let mut depth: i32 = 0;
    let mut end = text.len();
    for (offset, ch) in tail.char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                end = head + offset;
                break;
            }
        }
    }
    let arm = &text[head..end];
    if !arm.contains("2 =>") || !arm.contains("SKIP") {
        gate.err(
            DEVCHECK,
            "has no SKIP arm for a step that exits 2. TODO/gate.md T-1207 \
             item 4: a step that could not run must read as SKIP, never as \
             FAILED."
                .to_string(),
        );
    }
    if !arm.contains("_ =>") || !arm.contains("fail") {
        gate.err(
            DEVCHECK,
            "has no FAILED arm for a step that exits otherwise. TODO/gate.md \
             T-1207 item 4: only a real failure fails the check."
                .to_string(),
        );
    }
}

fn check_prove_flags(gate: &mut Gate, entries: &BTreeMap<String, Entry>, root: &Path) {
    let Ok(parity) = read_text(&root.join(PARITY_RS)) else {
        gate.err(
            PARITY_RS,
            "is not readable, so no Prove command can be \
             admitted. TODO/gate.md T-1325."
                .to_string(),
        );
        return;
    };
    let admission = parity_admission(&parity);
    let cluster_values = parity_cluster_values(&parity);
    let boundary = parity_cluster_boundary(&parity);
    let mut verbs: BTreeSet<String> = admission
        .admitted
        .keys()
        .chain(admission.refused_verbs.iter())
        .cloned()
        .collect();
    for (alias, under) in PARITY_ROWS_OF.iter().chain(PARITY_ROWS_OF_TAIL.iter()) {
        verbs.insert(alias.to_string());
        verbs.insert(under.to_string());
    }
    for (tid, entry) in entries.iter() {
        let Some(status_line) = find_field(&entry.body, "Status") else {
            continue;
        };
        if status_word(status_line).as_deref() != Some("done") {
            continue;
        }
        gate.bump("prove_flags");
        let lines: Vec<String> = entry.body.split('\n').map(str::to_string).collect();
        let mut start = None;
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with("Prove:") {
                start = Some(index);
                break;
            }
        }
        let Some(start) = start else {
            continue;
        };
        let end = prove_block(&lines, start, lines.len());
        let where_ = format!("TODO/{}:{}", entry.file, entry.line + start);
        for command in prove_commands(
            &lines[start..end],
            &admission.spellings,
            &cluster_values,
            &boundary,
        ) {
            if !verbs.contains(&command.verb) {
                if !command.flags.is_empty() {
                    gate.err(
                        &where_,
                        format!(
                            "({tid}) Prove runs `podbox {} {}`, \
                             and `{}` is no verb podbox answers to. A \
                             recorded acceptance that never ran is the failure \
                             this check exists to catch. TODO/gate.md T-1325.",
                            command.verb,
                            command.flags.join(" "),
                            command.verb
                        ),
                    );
                }
                continue;
            }
            let under = rows_of(&command.verb).to_string();
            if admission.refused_verbs.contains(&command.verb) {
                gate.err(
                    &where_,
                    format!(
                        "({tid}) Prove runs `podbox {}`, which the parity \
                         table refuses outright. A recorded acceptance that \
                         never ran is the failure this check exists to catch. \
                         TODO/gate.md T-1325.",
                        command.verb
                    ),
                );
                continue;
            }
            if let Some(bad) = command.bad {
                gate.err(
                    &where_,
                    format!(
                        "({tid}) Prove names `{bad}` for `podbox {}`, \
                         which has no row in the parity table: bundled shorts \
                         expand by docker's rule and the member refuses by \
                         name. TODO/gate.md T-1325.",
                        command.verb
                    ),
                );
                continue;
            }
            for flag in command.flags {
                let admitted = admission
                    .admitted
                    .get(&under)
                    .is_some_and(|set| set.contains(&flag));
                if !admitted {
                    gate.err(
                        &where_,
                        format!(
                            "({tid}) Prove names `{flag}` for `podbox {}`, \
                             which the parity table does not admit: `podbox \
                             {}` refuses it the way `admit` refuses \
                             anything with no row or with a None row. A \
                             recorded acceptance that never ran is the failure \
                             this check exists to catch. TODO/gate.md T-1325.",
                            command.verb, command.verb
                        ),
                    );
                }
            }
        }
    }
}
fn match_milestone_title(title: &str) -> Option<u64> {
    let rest = title.strip_prefix('M')?;
    let mut digits = 0;
    while rest
        .as_bytes()
        .get(digits)
        .is_some_and(|b| b.is_ascii_digit())
    {
        digits += 1;
    }
    if digits == 0 {
        return None;
    }
    let after = rest.as_bytes().get(digits);
    if after.is_some_and(|b| is_word_byte(*b)) {
        return None;
    }
    rest[..digits].parse::<u64>().ok()
}

fn milestone_blame(note: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for head in ["until M", "which is M"] {
        let mut at = 0;
        while let Some(found) = note[at..].find(head) {
            let mut pos = at + found + head.len();
            let bytes = note.as_bytes();
            let start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos > start {
                if let Ok(num) = note[start..pos].parse::<u64>() {
                    out.push(num);
                }
            }
            at = pos.max(at + 1);
        }
    }
    out
}

fn missing_claim_groups(note: &str) -> Vec<String> {
    let bytes = note.as_bytes();
    let mut groups = Vec::new();
    let mut search = 0;
    while search < note.len() {
        let mut found = None;
        let mut start = search;
        while start < note.len() {
            if !note.as_bytes()[start].is_ascii_alphabetic() {
                start = step_at(note, start);
                continue;
            }
            let mut run = start + 1;
            while run < note.len()
                && (bytes[run].is_ascii_alphanumeric()
                    || matches!(bytes[run], b'_' | b'`' | b',' | b' ' | b'/' | b'-'))
            {
                run += 1;
            }
            let mut best = None;
            for literal in [" is not implemented", " are not implemented"] {
                let mut at = start;
                while let Some(found) = note[at..run].find(literal) {
                    let lit_at = at + found;
                    if best.is_none_or(|(best_at, _)| lit_at > best_at) {
                        best = Some((lit_at, lit_at + literal.len()));
                    }
                    at = lit_at + 1;
                }
            }
            if let Some((lit_at, lit_end)) = best {
                found = Some((start, lit_at, lit_end));
                break;
            }
            start = step_at(note, start);
        }
        if let Some((start, lit_at, lit_end)) = found {
            groups.push(note[start..lit_at].to_string());
            search = lit_end;
        } else {
            break;
        }
    }
    groups
}

fn split_claim(group: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut at = 0;
    let bytes = group.as_bytes();
    let mut current = String::new();
    while at < bytes.len() {
        let delimiter = if group[at..].starts_with(',') {
            Some(1)
        } else if group[at..].starts_with(" and ") || group[at..].starts_with(" or ") {
            Some(5)
        } else if group[at..].starts_with('/') {
            Some(1)
        } else {
            None
        };
        if let Some(len) = delimiter {
            parts.push(std::mem::take(&mut current));
            at += len;
        } else {
            let ch = group[at..].chars().next().unwrap_or('\0');
            current.push(ch);
            at = step_at(group, at);
        }
    }
    parts.push(current);
    parts
}

fn find_parity_note(text: &str, from: usize) -> Option<String> {
    let brace = text[from..].find("},")?;
    let slice = &text[from..from + brace + 2];
    let marker = slice.find("note: \"")?;
    let mut pos = marker + "note: \"".len();
    let bytes = slice.as_bytes();
    let mut note = String::new();
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => return Some(note),
            b'\\' if pos + 1 < bytes.len() => {
                note.push(bytes[pos] as char);
                note.push(bytes[pos + 1] as char);
                pos += 2;
            }
            _ => {
                note.push(bytes[pos] as char);
                pos += 1;
            }
        }
    }
    None
}

fn check_parity_notes(gate: &mut Gate, entries: &BTreeMap<String, Entry>, root: &Path) {
    let Ok(text) = read_text(&root.join(PARITY_RS)) else {
        gate.err(
            PARITY_RS,
            "is not readable, so its notes cannot be held. \
             TODO/gate.md T-1325."
                .to_string(),
        );
        return;
    };
    let mut shipped = BTreeSet::new();
    for entry in entries.values() {
        if entry.file != "milestones.md" {
            continue;
        }
        let Some(num) = match_milestone_title(&entry.title) else {
            continue;
        };
        let done = entry.body.split('\n').any(|line| {
            line.starts_with("Status:")
                && line["Status:".len()..]
                    .trim_start_matches(' ')
                    .starts_with("done")
                && line["Status:".len()..]
                    .trim_start_matches(' ')
                    .get("done".len()..)
                    .is_none_or(|rest| {
                        rest.is_empty() || !rest.bytes().next().is_some_and(is_word_byte)
                    })
        });
        if done {
            shipped.insert(num);
        }
    }
    let rows = find_parity_rows(&text);
    let admission = parity_admission(&text);
    let present: BTreeSet<String> = admission.admitted.keys().cloned().collect();
    for row_match in rows.iter() {
        let row = &row_match.row;
        let line = row_match.line;
        let Some(note) = find_parity_note(&text, row_match.offset) else {
            continue;
        };
        gate.bump("parity_notes");
        let where_ = format!("{PARITY_RS}:{line}");
        let arm = row.spellings.as_deref().unwrap_or(&row.verb);
        for blamed in milestone_blame(&note) {
            if shipped.contains(&blamed) {
                gate.err(
                    &where_,
                    format!(
                        "the `{arm}` note leans on M{blamed}, which shipped \
                         (`TODO/milestones.md`): say what is \
                         missing now instead of blaming a milestone that is \
                         done. TODO/gate.md T-1325."
                    ),
                );
            }
        }
        for group in missing_claim_groups(&note) {
            for candidate in split_claim(&group) {
                let name = candidate.trim().trim_matches('`').trim().to_string();
                if present.contains(&name) {
                    gate.err(
                        &where_,
                        format!(
                            "the `{arm}` note claims `{name}` is not \
                             implemented, but the parity table carries it: a \
                             note that misses a verb rots the contract this \
                             table is. TODO/gate.md T-1325."
                        ),
                    );
                }
            }
        }
    }
    let run_spellings = admission.spellings.get("run");
    for name in CURATED_RUN_FLAGS
        .iter()
        .chain(CURATED_RUN_FLAGS_TAIL.iter())
        .chain(std::iter::once(&CURATED_RUN_FLAGS_LAST))
    {
        gate.bump("parity_notes");
        let missing = run_spellings.is_none_or(|set| !set.contains(*name));
        if missing {
            gate.err(
                PARITY_RS,
                format!(
                    "the curated docker surface (issue 60) names `{name}` for \
                     `run`, and the parity table has no row for it: a missing \
                     row cannot pass as a refusal. TODO/gate.md T-1325."
                ),
            );
        }
    }
    let mut verbs_present = BTreeSet::new();
    for row_match in rows.iter() {
        let row = &row_match.row;
        if row.spellings.as_deref().is_none_or(str::is_empty) {
            verbs_present.insert(row.verb.clone());
        }
    }
    for name in CURATED_VERBS {
        gate.bump("parity_notes");
        if !verbs_present.contains(name) {
            gate.err(
                PARITY_RS,
                format!(
                    "the curated docker surface (issue 60) names `{name}`, and \
                     the parity table has no verb row for it. TODO/gate.md \
                     T-1325."
                ),
            );
        }
    }
}
fn check_ascii_output(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let mut sources: Vec<&String> = files
        .iter()
        .filter(|f| f.ends_with(".rs") && ASCII_SCOPES.iter().any(|scope| f.starts_with(scope)))
        .collect();
    sources.sort();
    for rel in sources {
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        for (index, line) in split_lines(&text).into_iter().enumerate() {
            gate.bump("ascii_output");
            if line.trim_start().starts_with("//") {
                continue;
            }
            for ch in line.chars() {
                if ch as u32 > 0x7F {
                    gate.err(
                        &format!("{}:{}", rel, index + 1),
                        format!(
                            "prints U+{:04X}: the binary prints plain \
                             ASCII on every path, and a glyph here is \
                             unrenderable bytes in an automated caller's \
                             stream. TODO/cli.md T-1336.",
                            ch as u32
                        ),
                    );
                    break;
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

fn parse_json(text: &str) -> Option<(JsonValue, usize)> {
    let bytes = text.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }
    match bytes[pos] {
        b'n' if text[pos..].starts_with("null") => Some((JsonValue::Null, pos + 4)),
        b't' if text[pos..].starts_with("true") => Some((JsonValue::Bool(true), pos + 4)),
        b'f' if text[pos..].starts_with("false") => Some((JsonValue::Bool(false), pos + 5)),
        b'"' => {
            let mut out = String::new();
            let mut cursor = pos + 1;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'"' => {
                        return Some((JsonValue::Text(out), cursor + 1));
                    }
                    b'\\' if cursor + 1 < bytes.len() => {
                        let esc = bytes[cursor + 1];
                        match esc {
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            b'/' => out.push('/'),
                            b'n' => out.push('\n'),
                            b't' => out.push('\t'),
                            b'r' => out.push('\r'),
                            _ => {
                                out.push('\\');
                                out.push(esc as char);
                            }
                        }
                        cursor += 2;
                    }
                    _ => {
                        out.push(bytes[cursor] as char);
                        cursor += 1;
                    }
                }
            }
            None
        }
        b'[' => {
            let mut items = Vec::new();
            let mut cursor = pos + 1;
            loop {
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b']' {
                    return Some((JsonValue::Array(items), cursor + 1));
                }
                let (value, next) = parse_json(&text[cursor..])?;
                items.push(value);
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b',' {
                    cursor += 1;
                    continue;
                }
                if cursor < bytes.len() && bytes[cursor] == b']' {
                    return Some((JsonValue::Array(items), cursor + 1));
                }
                return None;
            }
        }
        b'{' => {
            let mut fields = Vec::new();
            let mut cursor = pos + 1;
            loop {
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    return Some((JsonValue::Object(fields), cursor + 1));
                }
                let (key, next) = parse_json(&text[cursor..])?;
                let JsonValue::Text(name) = key else {
                    return None;
                };
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor >= bytes.len() || bytes[cursor] != b':' {
                    return None;
                }
                cursor += 1;
                let (value, next) = parse_json(&text[cursor..])?;
                fields.push((name, value));
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b',' {
                    cursor += 1;
                    continue;
                }
                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    return Some((JsonValue::Object(fields), cursor + 1));
                }
                return None;
            }
        }
        _ => {
            let mut cursor = pos;
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && !matches!(bytes[cursor], b',' | b']' | b'}')
            {
                cursor += 1;
            }
            if cursor == pos {
                return None;
            }
            Some((JsonValue::Number(text[pos..cursor].to_string()), cursor))
        }
    }
}

fn json_field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

// Render a ledger item the way the reference output renders one.
fn render_json_item(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "None".to_string(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Number(raw) => raw.clone(),
        JsonValue::Text(text) => text.clone(),
        JsonValue::Array(items) => {
            let inner: Vec<String> = items.iter().map(render_json_item).collect();
            format!("[{}]", inner.join(", "))
        }
        JsonValue::Object(fields) => {
            let inner: Vec<String> = fields
                .iter()
                .map(|(key, val)| format!("'{}': {}", key, render_json_item(val)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

fn check_post_task_cleanup(gate: &mut Gate, root: &Path) {
    let rules = root.join("TODO").join("RULES.md");
    let Ok(rules_text) = read_text(&rules) else {
        gate.err(
            "TODO/RULES.md",
            "does not exist, and check 29 holds the \
             post-task cleanup procedure it must carry"
                .to_string(),
        );
        return;
    };
    for anchor in RULE_ANCHORS {
        gate.bump("post_task_cleanup");
        if !rules_text.contains(anchor) {
            gate.err(
                "TODO/RULES.md",
                format!(
                    "names no post-task cleanup procedure: {} is gone, and \
                     storage on the lane host is a fixed allowance",
                    quote_repr(anchor)
                ),
            );
        }
    }
    let Some(tool) = which("wsl-toolkit") else {
        return;
    };
    let tool_str = tool.to_string_lossy().into_owned();
    let ran = run_with_timeout(
        &tool_str,
        &["--instance", "podbox", "gc", "--json"],
        root,
        Duration::from_millis(180000),
    );
    let (code, stdout, stderr) = match ran {
        Err(RunError::Spawn) => {
            gate.err(
                "wsl-toolkit",
                "the lane-job ledger could not be read: could not run the ledger command"
                    .to_string(),
            );
            return;
        }
        Err(RunError::Timeout) => {
            gate.err(
                "wsl-toolkit",
                "the lane-job ledger could not be read: the ledger command timed out".to_string(),
            );
            return;
        }
        Ok(done) => done,
    };
    if code != Some(0) {
        let detail: String = stderr.trim().chars().take(120).collect();
        gate.err(
            "wsl-toolkit",
            format!(
                "the lane-job ledger could not be read: exit {}: {detail}",
                code.unwrap_or(-1)
            ),
        );
        return;
    }
    let report_text = stdout;
    let Some((report, _)) = parse_json(&report_text) else {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger is not JSON: no JSON value found".to_string(),
        );
        return;
    };
    let JsonValue::Object(fields) = report else {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger has an unknown schema or is not a dry run".to_string(),
        );
        return;
    };
    let schema_ok = matches!(
        json_field(&fields, "schema"),
        Some(JsonValue::Text(text)) if text == "wsl-toolkit-cleanup/1"
    );
    let dry_ok = matches!(json_field(&fields, "dry_run"), Some(JsonValue::Bool(true)));
    if !schema_ok || !dry_ok {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger has an unknown schema or is not a dry run".to_string(),
        );
        return;
    }
    for field in CLEANUP_FIELDS {
        let items = json_field(&fields, field);
        gate.bump("post_task_cleanup");
        let Some(items) = items else {
            continue;
        };
        if *items == JsonValue::Null {
            continue;
        }
        let JsonValue::Array(items) = items else {
            gate.err(
                "wsl-toolkit",
                format!("the lane-job ledger field {field} is not a list"),
            );
            continue;
        };
        for item in items {
            gate.err(
                "wsl-toolkit",
                format!(
                    "lane job still kept: {} -- remove it \
                     with `wsl-toolkit --instance podbox gc \
                     --job <id> --apply` before the next job",
                    render_json_item(item)
                ),
            );
        }
    }
}
fn check_perf_budget(gate: &mut Gate, root: &Path) {
    let Ok(text) = read_text(&root.join(PERF_CEILINGS)) else {
        gate.err(
            PERF_CEILINGS,
            "does not exist, and check 30 holds the \
             perf budgets against the committed readings. \
             TODO/gate.md T-1338."
                .to_string(),
        );
        return;
    };
    let mut ceilings: BTreeMap<String, (String, f64, f64)> = BTreeMap::new();
    for (index, line) in text.split('\n').enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            gate.err(
                &format!("{PERF_CEILINGS}:{}", index + 1),
                format!("row does not parse: {}", line.trim()),
            );
            continue;
        }
        let metric = parts[0].trim().to_string();
        let unit = parts[1].trim().to_string();
        let ceiling: Option<f64> = parts[2].trim().parse().ok();
        let tolerance: Option<f64> = parts[3].trim().parse().ok();
        match (ceiling, tolerance) {
            (Some(ceiling), Some(tolerance)) => {
                ceilings.insert(metric, (unit, ceiling, tolerance));
            }
            _ => {
                gate.err(
                    &format!("{PERF_CEILINGS}:{}", index + 1),
                    format!("ceiling is not a number: {}", line.trim()),
                );
            }
        }
    }
    if ceilings.is_empty() {
        gate.err(
            PERF_CEILINGS,
            "carries no budgets, and an empty budget file \
             holds every reading trivially. TODO/gate.md T-1338."
                .to_string(),
        );
        return;
    }
    let mut rows = Vec::new();
    for rel in PERF_RESULTS {
        let Ok(body) = read_text(&root.join(rel)) else {
            if rel.ends_with("perf-lane.txt") || rel.ends_with("perf-seeds.tsv") {
                gate.err(
                    rel,
                    "does not exist, and check 30 compares the \
                     committed perf readings. TODO/gate.md T-1338."
                        .to_string(),
                );
            }
            continue;
        };
        for (index, line) in body.split('\n').enumerate() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 10 {
                continue;
            }
            rows.push((
                rel.to_string(),
                index + 1,
                parts[4].to_string(),
                parts[5].to_string(),
                parts[7].to_string(),
                parts[8].to_string(),
                parts[9].to_string(),
            ));
        }
    }
    for metric in ceilings.keys() {
        if !rows
            .iter()
            .any(|(_, _, _, row_metric, _, _, _)| row_metric == metric)
        {
            gate.err(
                PERF_CEILINGS,
                format!(
                    "budget `{metric}` names no committed reading, \
                     and a budget nothing measures holds vacuously. \
                     TODO/gate.md T-1338."
                ),
            );
        }
    }
    for (rel, line_no, shape, metric, value, unit, state) in rows {
        if state != "ok" {
            continue;
        }
        let Some((_, ceiling, tolerance)) = ceilings.get(&metric) else {
            continue;
        };
        let Ok(number): Result<f64, _> = value.parse() else {
            continue;
        };
        gate.bump("perf_budget");
        if number > ceiling * (1.0 + tolerance) {
            gate.err(
                &format!("{rel}:{line_no}"),
                format!(
                    "perf regression: `{metric}` reads {value}{unit} \
                     against a {} ceiling on shape {shape}. \
                     TODO/gate.md T-1338.",
                    format_ceiling(*ceiling)
                ),
            );
        }
    }
}

fn find_python() -> Option<String> {
    for candidate in ["python3", "python", "py"] {
        let Some(path) = which(candidate) else {
            continue;
        };
        let path_str = path.to_string_lossy().into_owned();
        let usable = run_with_timeout(
            &path_str,
            &["--version"],
            &env::temp_dir(),
            Duration::from_millis(15000),
        );
        if matches!(usable, Ok((Some(0), _, _))) {
            return Some(path_str);
        }
    }
    None
}

fn check_document_state(gate: &mut Gate, root: &Path) {
    gate.seen.insert("document_state", 1);
    let Some(python) = find_python() else {
        gate.err(
            RUNTIME_STATE,
            "source state could not run: no Python interpreter on PATH".to_string(),
        );
        return;
    };
    let script = root
        .join(DOCUMENT_STATE_SCRIPT)
        .to_string_lossy()
        .into_owned();
    match run_with_timeout(&python, &[&script], root, Duration::from_millis(30000)) {
        Err(RunError::Spawn) => {
            gate.err(
                RUNTIME_STATE,
                "source state could not run: could not run the state script".to_string(),
            );
        }
        Err(RunError::Timeout) => {
            gate.err(
                RUNTIME_STATE,
                "source state could not run: the state script timed out".to_string(),
            );
        }
        Ok((code, stdout, stderr)) => {
            if code != Some(0) {
                let detail = if stderr.is_empty() { stdout } else { stderr };
                gate.err(
                    RUNTIME_STATE,
                    format!("source state differs: {}", detail.trim()),
                );
            }
        }
    }
}

// Render a word list the way the reference output renders one.
fn py_list(words: &[&str]) -> String {
    let inner: Vec<String> = words.iter().map(|word| format!("'{word}'")).collect();
    format!("[{}]", inner.join(", "))
}

struct IndexRow {
    target: String,
    prio: String,
    category: String,
    status: String,
    title: String,
    line: usize,
}

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(lexical_normalize(&candidate));
        }
        candidate = candidate.join("..");
    }
    None
}

fn prove_first_line(body: &str) -> Option<String> {
    for line in body.split('\n') {
        if !line.starts_with("Prove:") {
            continue;
        }
        let after = &line["Prove:".len()..];
        if !after.starts_with(' ') {
            continue;
        }
        let spaces = after.len() - after.trim_start_matches(' ').len();
        if spaces == 0 {
            continue;
        }
        return Some(after[spaces..].to_string());
    }
    None
}

fn contains_backtick_span(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let mut pos = at + 1;
        while pos < bytes.len() && bytes[pos] != b'`' {
            pos += 1;
        }
        if pos < bytes.len() && pos > at + 1 {
            return true;
        }
        at += 1;
    }
    false
}

fn starts_double_indented(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(a), Some(b), Some(c)) => {
            (a == ' ' || a == '\t') && (b == ' ' || b == '\t') && c != ' ' && c != '\t' && c != '\n'
        }
        _ => false,
    }
}

fn run_gate(root: &Path) -> i32 {
    let todo = root.join("TODO");
    let refs = root.join("references");
    let Ok(index) = read_text(&todo.join("INDEX.md")) else {
        eprintln!("check-todo: TODO/INDEX.md does not exist");
        return 2;
    };

    let mut gate = Gate::new();

    let mut rows: BTreeMap<String, IndexRow> = BTreeMap::new();
    for (index_no, line) in index.split('\n').enumerate() {
        if !line.starts_with("| [T-") {
            continue;
        }
        let Some(parsed) = parse_index_row(line) else {
            gate.err(
                &format!("TODO/INDEX.md:{}", index_no + 1),
                format!("row does not parse: {}", line.trim()),
            );
            continue;
        };
        let tid = parsed.id.clone();
        if rows.contains_key(&tid) {
            gate.err(
                &format!("TODO/INDEX.md:{}", index_no + 1),
                format!("{tid} appears twice"),
            );
        }
        if !STATUSES.contains(&parsed.status.as_str()) {
            gate.err(
                &format!("TODO/INDEX.md:{}", index_no + 1),
                format!(
                    "{tid} status {} is not one of {}",
                    quote_repr(&parsed.status),
                    py_list(&STATUSES)
                ),
            );
        }
        if !PRIORITIES.contains(&parsed.priority.as_str()) {
            gate.err(
                &format!("TODO/INDEX.md:{}", index_no + 1),
                format!(
                    "{tid} priority {} is not one of {}",
                    quote_repr(&parsed.priority),
                    py_list(&PRIORITIES)
                ),
            );
        }
        rows.insert(
            tid,
            IndexRow {
                target: parsed.file,
                prio: parsed.priority,
                category: parsed.category,
                status: parsed.status,
                title: parsed.item,
                line: index_no + 1,
            },
        );
        gate.bump("rows");
    }

    if rows.is_empty() {
        gate.err("TODO/INDEX.md", "no entry rows found".to_string());
    }

    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    let Ok(names) = todo_names(&todo) else {
        eprintln!("check-todo: TODO/ does not exist");
        return 2;
    };
    for name in &names {
        if !name.ends_with(".md")
            || ["INDEX.md", "PROGRESS.md", "RULES.md", "reference-map.md"].contains(&name.as_str())
        {
            continue;
        }
        let Ok(text) = read_text(&todo.join(name)) else {
            continue;
        };
        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        let mut heads = Vec::new();
        for (index_no, line) in lines.iter().enumerate() {
            if let Some(parsed) = parse_entry_heading(line) {
                heads.push((index_no, parsed.id, parsed.title));
            }
        }
        for (position, (line_no, tid, title)) in heads.iter().enumerate() {
            let end = heads
                .get(position + 1)
                .map(|(next, _, _)| *next)
                .unwrap_or(lines.len());
            let body = lines[*line_no..end].join("\n");
            if entries.contains_key(tid) {
                gate.err(
                    &format!("TODO/{name}:{}", line_no + 1),
                    format!("{tid} is defined twice"),
                );
            }
            entries.insert(
                tid.clone(),
                Entry {
                    file: name.clone(),
                    line: line_no + 1,
                    title: title.clone(),
                    body,
                },
            );
            gate.bump("entries");
        }
    }

    for (tid, row) in rows.iter() {
        match entries.get(tid) {
            None => {
                gate.err(
                    &format!("TODO/INDEX.md:{}", row.line),
                    format!("{tid} has a row and no entry"),
                );
            }
            Some(entry) => {
                if entry.file != row.target {
                    gate.err(
                        &format!("TODO/INDEX.md:{}", row.line),
                        format!(
                            "{tid} row links to {} and the entry is in {}",
                            row.target, entry.file
                        ),
                    );
                }
                if entry.title != row.title {
                    gate.err(
                        &format!("TODO/INDEX.md:{}", row.line),
                        format!(
                            "{tid} row title {} != entry title {}",
                            quote_repr(&row.title),
                            quote_repr(&entry.title)
                        ),
                    );
                }
            }
        }
    }
    for (tid, entry) in entries.iter() {
        if !rows.contains_key(tid) {
            gate.err(
                &format!("TODO/{}:{}", entry.file, entry.line),
                format!("{tid} has an entry and no row"),
            );
        }
    }

    for (tid, entry) in entries.iter() {
        let where_ = format!("TODO/{}:{}", entry.file, entry.line);
        for field in FIELDS {
            let Some(value) = find_field(&entry.body, field) else {
                gate.err(&where_, format!("{tid} has no `{field}:` field"));
                continue;
            };
            gate.bump("fields");
            if field == "Status" {
                match status_word(value) {
                    Some(status) if STATUSES.contains(&status.as_str()) => {
                        if let Some(row) = rows.get(tid) {
                            if status != row.status {
                                gate.err(
                                    &where_,
                                    format!(
                                        "{tid} entry status {} disagrees with the index \
                                         row's {}",
                                        quote_repr(&status),
                                        quote_repr(&row.status)
                                    ),
                                );
                            }
                        }
                    }
                    Some(status) => {
                        gate.err(
                            &where_,
                            format!(
                                "{tid} Status {} is not one of {}",
                                quote_repr(&status),
                                py_list(&STATUSES)
                            ),
                        );
                    }
                    None => {
                        gate.err(
                            &where_,
                            format!(
                                "{tid} Status {} is not one of {}",
                                quote_repr(value),
                                py_list(&STATUSES)
                            ),
                        );
                    }
                }
            }
            if field == "Category" {
                if let Some(row) = rows.get(tid) {
                    if value != row.category {
                        gate.err(
                            &where_,
                            format!(
                                "{tid} Category {} disagrees with the row's {}",
                                quote_repr(value),
                                quote_repr(&row.category)
                            ),
                        );
                    }
                }
            }
            if field == "Priority" {
                if let Some(row) = rows.get(tid) {
                    if value != row.prio {
                        gate.err(
                            &where_,
                            format!(
                                "{tid} Priority {} disagrees with the row's {}",
                                quote_repr(value),
                                quote_repr(&row.prio)
                            ),
                        );
                    }
                }
            }
        }
        if let Some(remainder) = prove_first_line(&entry.body) {
            if !remainder.contains("```")
                && !contains_backtick_span(&remainder)
                && !starts_double_indented(&remainder)
            {
                let head: String = remainder.chars().take(60).collect();
                gate.err(
                    &where_,
                    format!("{tid} Prove is prose, not a command: {}", quote_repr(&head)),
                );
            }
        }
    }

    let mut derived: BTreeMap<&str, usize> = BTreeMap::new();
    for status in STATUSES {
        derived.insert(status, 0);
    }
    let mut per_prio: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for prio in PRIORITIES {
        let mut inner = BTreeMap::new();
        for status in STATUSES {
            inner.insert(status, 0);
        }
        per_prio.insert(prio, inner);
    }
    for row in rows.values() {
        *derived.entry(row.status.as_str()).or_insert(0) += 1;
        if let Some(inner) = per_prio.get_mut(row.prio.as_str()) {
            *inner.entry(row.status.as_str()).or_insert(0) += 1;
        }
    }
    let total = rows.len();

    *gate.seen.entry("counts").or_insert(0) = 1 + PRIORITIES.len() as u64;
    let want = format!(
        "{total} items: {} open, {} partial, {} blocked, {} done.",
        derived["open"], derived["partial"], derived["blocked"], derived["done"]
    );
    if !index.contains(&want) {
        gate.err(
            "TODO/INDEX.md",
            format!("the totals line is not the rows. Expected exactly:\n    {want}"),
        );
    }
    for prio in PRIORITIES {
        let counts = &per_prio[prio];
        let prio_total: usize = counts.values().sum();
        let want_row = format!(
            "| {prio} | {} | {} | {} | {} | {prio_total} |",
            counts["open"], counts["partial"], counts["blocked"], counts["done"]
        );
        if !index.contains(&want_row) {
            gate.err(
                "TODO/INDEX.md",
                format!("the {prio} count row is not the rows. Expected exactly:\n    {want_row}"),
            );
        }
    }
    let want_all = format!(
        "| **All** | **{}** | **{}** | **{}** | **{}** | **{total}** |",
        derived["open"], derived["partial"], derived["blocked"], derived["done"]
    );
    if !index.contains(&want_all) {
        gate.err(
            "TODO/INDEX.md",
            format!("the All count row is not the rows. Expected exactly:\n    {want_all}"),
        );
    }

    let map_path = todo.join("reference-map.md");
    if !map_path.is_file() {
        gate.err("TODO/reference-map.md", "does not exist".to_string());
    } else if let Ok(map_text) = read_text(&map_path) {
        let mut named = BTreeSet::new();
        for line in map_text.split('\n') {
            for cited in find_bares(line) {
                if let Some(tree) = cited.strip_prefix("references/") {
                    if !tree.contains('/') {
                        named.insert(tree.to_string());
                    }
                }
            }
        }
        if named.is_empty() {
            gate.err(
                "TODO/reference-map.md",
                "names no reference under references/".to_string(),
            );
        }
        let mut named_sorted: Vec<&String> = named.iter().collect();
        named_sorted.sort();
        for tree in named_sorted {
            let dir = refs.join(tree);
            gate.bump("corpus");
            if !dir.is_dir() {
                gate.err(
                    "TODO/reference-map.md",
                    format!("references/{tree} does not exist"),
                );
            } else if !dir.join("PROVENANCE.md").is_file() {
                gate.err(
                    "TODO/reference-map.md",
                    format!("references/{tree} has no PROVENANCE.md"),
                );
            }
        }
        let mut on_disk = BTreeSet::new();
        if refs.is_dir() {
            if let Ok(dir_entries) = fs::read_dir(&refs) {
                for entry in dir_entries.flatten() {
                    if entry.path().is_dir() {
                        on_disk.insert(entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }
        let mut extra: Vec<&String> = on_disk.difference(&named).collect();
        extra.sort();
        for tree in extra {
            gate.err(
                "TODO/reference-map.md",
                format!("references/{tree} is on disk and the map does not name it"),
            );
        }
    }
    for name in &names {
        if !name.ends_with(".md") {
            continue;
        }
        let Ok(text) = read_text(&todo.join(name)) else {
            continue;
        };
        for (index_no, line) in text.split('\n').enumerate() {
            for cite in find_cites(line) {
                gate.bump("todo_citations");
                let target = root.join(&cite.cited);
                let where_ = format!("TODO/{name}:{}", index_no + 1);
                if !target.is_file() {
                    gate.err(
                        &where_,
                        format!(
                            "cites {}:{}, and that file does not exist",
                            cite.cited, cite.start
                        ),
                    );
                    continue;
                }
                let data = fs::read(&target).unwrap_or_default();
                let count = count_lines_bytes(&data);
                let last = cite.end.unwrap_or(cite.start);
                if last > count {
                    let range = if let Some(end) = cite.end {
                        format!("{}-{end}", cite.start)
                    } else {
                        format!("{}", cite.start)
                    };
                    gate.err(
                        &where_,
                        format!(
                            "cites {}:{range}, and that file has {count} lines",
                            cite.cited
                        ),
                    );
                }
            }
            for link in find_links(line) {
                let mut href = link.href;
                if href.starts_with("http://")
                    || href.starts_with("https://")
                    || href.starts_with("mailto:")
                {
                    continue;
                }
                gate.bump("todo_links");
                let original = href.clone();
                let first = href.split(':').next().unwrap_or("").to_string();
                href = first;
                let target = lexical_normalize(&todo.join(&href));
                if !target.exists() {
                    gate.err(
                        &format!("TODO/{name}:{}", index_no + 1),
                        format!("link target {original} does not resolve"),
                    );
                }
            }
        }
    }

    for name in &names {
        if !name.ends_with(".md") {
            continue;
        }
        let Ok(text) = read_text(&todo.join(name)) else {
            continue;
        };
        for (index_no, line) in text.split('\n').enumerate() {
            if line.starts_with("| [T-") || line.starts_with("### T-") {
                continue;
            }
            let mut mentioned = BTreeSet::new();
            let bytes = line.as_bytes();
            let mut at = 0;
            while at < line.len() {
                let is_start = bytes[at] == b'T'
                    && line.len() >= at + 6
                    && bytes[at + 1] == b'-'
                    && bytes[at + 2].is_ascii_digit()
                    && bytes[at + 3].is_ascii_digit()
                    && bytes[at + 4].is_ascii_digit()
                    && bytes[at + 5].is_ascii_digit()
                    && (at == 0 || !is_word_byte(bytes[at - 1]))
                    && (line.len() == at + 6 || !is_word_byte(bytes[at + 6]));
                if is_start {
                    mentioned.insert(line[at..at + 6].to_string());
                    at += 6;
                } else {
                    at = step_at(line, at);
                }
            }
            for tid in mentioned {
                gate.bump("crossrefs");
                if !entries.contains_key(&tid) {
                    gate.err(
                        &format!("TODO/{name}:{}", index_no + 1),
                        format!("names {tid}, which is not an entry"),
                    );
                }
            }
        }
    }

    let prog_path = todo.join("PROGRESS.md");
    if !prog_path.is_file() {
        gate.err("TODO/PROGRESS.md", "does not exist".to_string());
    } else if let Ok(prog) = read_text(&prog_path) {
        let want_prog = format!(
            "{total} entries: {} open, {} partial, {} blocked, {} done.",
            derived["open"], derived["partial"], derived["blocked"], derived["done"]
        );
        if !prog.contains(&want_prog) {
            gate.err(
                "TODO/PROGRESS.md",
                format!("the count line is not the rows. Expected exactly:\n    {want_prog}"),
            );
        }
    }

    let Some(files) = tracked_files(root) else {
        eprintln!(
            "check-todo: `git ls-files` failed, so checks 11 to 15 could not \
             run. This is not a pass."
        );
        return 2;
    };
    check_tree(&mut gate, root, &files);

    for rel in files.iter() {
        if is_scratch(rel) {
            gate.err(
                rel,
                "is a build artefact or experiment scratch and is tracked. \
                 Untrack it and make .gitignore carry a rule rather than a \
                 list of names."
                    .to_string(),
            );
        }
    }

    check_size_ceiling(&mut gate, root, &files);
    check_experiment_numbers(&mut gate, root, &files);
    check_ci_components(&mut gate, root, &files);
    check_exit_codes(&mut gate, root, &files);
    check_prove_registry(&mut gate, &todo);
    check_closure_records(&mut gate, &entries);
    check_interpose_sizes(&mut gate, root, &files);
    check_interpose_exports(&mut gate, root, &files);
    check_devcheck_third_state(&mut gate, root, &files);
    check_prove_flags(&mut gate, &entries, root);
    check_parity_notes(&mut gate, &entries, root);
    check_ascii_output(&mut gate, root, &files);
    check_post_task_cleanup(&mut gate, root);
    check_perf_budget(&mut gate, root);
    check_document_state(&mut gate, root);

    let empty: Vec<String> = gate
        .seen
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(key, _)| key.to_string())
        .collect();
    for key in &empty {
        gate.err(
            "check-todo",
            format!(
                "the {} check examined nothing. Either the tree \
                 lost every case or the check stopped matching; both \
                 are failures.",
                quote_repr(key)
            ),
        );
    }

    println!(
        "check-todo: {} rows, {} entries, {} open, {} partial, {} blocked, {} done",
        rows.len(),
        entries.len(),
        derived["open"],
        derived["partial"],
        derived["blocked"],
        derived["done"]
    );
    let coverage: Vec<String> = gate
        .seen
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect();
    println!("check-todo: coverage {}", coverage.join(" "));
    if gate.errors.is_empty() {
        println!("check-todo: ok");
        return 0;
    }
    eprintln!("check-todo: {} problem(s)", gate.errors.len());
    for error in &gate.errors {
        eprintln!("  {error}");
    }
    1
}

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
