use super::*;

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

pub(super) fn check_document_state(gate: &mut Gate, root: &Path) {
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
pub(super) fn py_list(words: &[&str]) -> String {
    let inner: Vec<String> = words.iter().map(|word| format!("'{word}'")).collect();
    format!("[{}]", inner.join(", "))
}

pub(super) struct IndexRow {
    pub(super) target: String,
    pub(super) prio: String,
    pub(super) category: String,
    pub(super) status: String,
    pub(super) title: String,
    pub(super) line: usize,
}

pub(super) fn find_root() -> Option<PathBuf> {
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

pub(super) fn prove_first_line(body: &str) -> Option<String> {
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

pub(super) fn contains_backtick_span(text: &str) -> bool {
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

pub(super) fn starts_double_indented(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(a), Some(b), Some(c)) => {
            (a == ' ' || a == '\t') && (b == ' ' || b == '\t') && c != ' ' && c != '\t' && c != '\n'
        }
        _ => false,
    }
}

/// `TODO/reference-map.md` against the captured corpus: each tree the map
/// names is in the manifest with its PROVENANCE.md, and the manifest holds
/// no tree the map does not name.
pub(super) fn check_reference_map(gate: &mut Gate, todo: &Path, corpus: &BTreeMap<String, usize>) {
    let map_path = todo.join("reference-map.md");
    if !map_path.is_file() {
        gate.err("TODO/reference-map.md", "does not exist".to_string());
    } else if let Ok(map_text) = read_text(&map_path) {
        let mut named = BTreeSet::new();
        for line in map_text.split('\n') {
            for cited in find_bares(line) {
                if let Some(tree) = cited.strip_prefix("references/")
                    && !tree.contains('/')
                {
                    named.insert(tree.to_string());
                }
            }
        }
        if named.is_empty() {
            gate.err(
                "TODO/reference-map.md",
                "names no reference under references/".to_string(),
            );
        }
        if corpus.is_empty() {
            gate.err(
                CORPUS_MANIFEST,
                "is missing or lists no file, so no citation into references/ \
                 can resolve"
                    .to_string(),
            );
        }
        let mut named_sorted: Vec<&String> = named.iter().collect();
        named_sorted.sort();
        for tree in named_sorted {
            gate.bump("corpus");
            if !corpus_names(corpus, &format!("{CORPUS_PREFIX}{tree}")) {
                gate.err(
                    "TODO/reference-map.md",
                    format!("references/{tree} is not in {CORPUS_MANIFEST}"),
                );
            } else if !corpus.contains_key(&format!("{CORPUS_PREFIX}{tree}/PROVENANCE.md")) {
                gate.err(
                    "TODO/reference-map.md",
                    format!("references/{tree} has no PROVENANCE.md in {CORPUS_MANIFEST}"),
                );
            }
        }
        let mut in_corpus = BTreeSet::new();
        for path in corpus.keys() {
            if let Some((tree, _)) = path
                .strip_prefix(CORPUS_PREFIX)
                .and_then(|rest| rest.split_once('/'))
            {
                in_corpus.insert(tree.to_string());
            }
        }
        let mut extra: Vec<&String> = in_corpus.difference(&named).collect();
        extra.sort();
        for tree in extra {
            gate.err(
                "TODO/reference-map.md",
                format!("references/{tree} is in {CORPUS_MANIFEST} and the map does not name it"),
            );
        }
    }
}

/// The totals line, the row of each priority and the All row of
/// `TODO/INDEX.md`, each against the rows. Answers the count of each status,
/// which `PROGRESS.md`'s state line is checked against too.
pub(super) fn check_index_counts<'a>(
    gate: &mut Gate,
    rows: &'a BTreeMap<String, IndexRow>,
    index: &str,
) -> BTreeMap<&'a str, usize> {
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
        "{total} entries: {} open, {} partial, {} blocked, {} done.",
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
        "| **All** | {} | {} | {} | {} | {total} |",
        derived["open"], derived["partial"], derived["blocked"], derived["done"]
    );
    if !index.contains(&want_all) {
        gate.err(
            "TODO/INDEX.md",
            format!("the All count row is not the rows. Expected exactly:\n    {want_all}"),
        );
    }
    derived
}
