use super::*;

pub(super) const CITE_PREFIXES: [&str; 6] = [
    "references",
    "crates",
    "experiments",
    "scripts",
    "docs",
    "TODO",
];

pub(super) const UNQUALIFIED_IMAGES: [&str; 15] = [
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

pub(super) const RULE_ANCHORS: [&str; 4] = [
    "Post-task cleanup, after every task",
    "gc --job <id> --apply",
    "gc --apply",
    "experiments/results/",
];

pub(super) const CLEANUP_FIELDS: [&str; 4] = ["containers", "guest_dirs", "host_dirs", "sessions"];

pub(super) fn rows_of(verb: &str) -> &str {
    for (alias, under) in PARITY_ROWS_OF.iter().chain(PARITY_ROWS_OF_TAIL.iter()) {
        if *alias == verb {
            return under;
        }
    }
    verb
}

pub(super) struct Gate {
    pub(super) errors: Vec<String>,
    pub(super) seen: BTreeMap<&'static str, u64>,
}

impl Gate {
    pub(super) fn new() -> Gate {
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
            "findings",
            "pod_todo",
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

    pub(super) fn err(&mut self, where_: &str, msg: String) {
        self.errors.push(format!("{where_}: {msg}"));
    }

    pub(super) fn bump(&mut self, key: &'static str) {
        *self.seen.entry(key).or_insert(0) += 1;
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(super) fn read_text(path: &Path) -> Result<String, String> {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Ok(normalize_newlines(&text)),
            Err(_) => Err("not readable".to_string()),
        },
        Err(_) => Err("not readable".to_string()),
    }
}

// Path to line count, from `CORPUS_MANIFEST`. A line is `PATH<TAB>LINES`;
// a line that starts with `#` is a comment. An unreadable manifest gives an
// empty map, and `run_gate` reports that as its own problem.
pub(super) fn load_corpus(root: &Path) -> BTreeMap<String, usize> {
    let mut corpus = BTreeMap::new();
    let Ok(text) = read_text(&root.join(CORPUS_MANIFEST)) else {
        return corpus;
    };
    for line in text.split('\n') {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((path, lines)) = line.split_once('\t')
            && let Ok(lines) = lines.trim().parse::<usize>()
        {
            corpus.insert(path.to_string(), lines);
        }
    }
    corpus
}

// True when the manifest lists `stem` as a file or as a directory.
pub(super) fn corpus_names(corpus: &BTreeMap<String, usize>, stem: &str) -> bool {
    if corpus.contains_key(stem) {
        return true;
    }
    let dir = format!("{stem}/");
    corpus
        .range(dir.clone()..)
        .next()
        .is_some_and(|(path, _)| path.starts_with(&dir))
}

// The problem with a `PATH:LINE` citation into the corpus, if any.
pub(super) fn corpus_cite_problem(corpus: &BTreeMap<String, usize>, cite: &Cite) -> Option<String> {
    let range = match cite.end {
        Some(end) => format!("{}-{end}", cite.start),
        None => format!("{}", cite.start),
    };
    match corpus.get(&cite.cited) {
        None => Some(format!(
            "cites {}:{range}, and {CORPUS_MANIFEST} lists no such file",
            cite.cited
        )),
        Some(&count) if cite.end.unwrap_or(cite.start) > count => Some(format!(
            "cites {}:{range}, and that file has {count} lines",
            cite.cited
        )),
        Some(_) => None,
    }
}

pub(super) fn count_lines_bytes(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    let mut count = data.iter().filter(|b| **b == b'\n').count();
    if data[data.len() - 1] != b'\n' {
        count += 1;
    }
    count
}

pub(super) fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// Advance one character, so byte-stepping scanners never split one.
pub(super) fn step_at(line: &str, at: usize) -> usize {
    if at >= line.len() {
        return at + 1;
    }
    at + line[at..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

// Single-quote a string the way the reference output quotes one.
pub(super) fn quote_repr(text: &str) -> String {
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
pub(super) fn format_ceiling(value: f64) -> String {
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

pub(super) fn lexical_normalize(path: &Path) -> PathBuf {
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
pub(super) struct Cite {
    pub(super) cited: String,
    pub(super) start: usize,
    pub(super) end: Option<usize>,
}

pub(super) fn match_cite_at(line: &str, at: usize) -> Option<(Cite, usize)> {
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
