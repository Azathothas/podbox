//! Check 34: the findings page (TODO/gate.md T-1668, issue 189).
//!
//! `docs/findings.md` holds one table row for each defect of a tool and
//! each trait of a host that changes how a result reads. A row is
//! `| ID | TOOL | MARK | FINDING | EVIDENCE | CONDITIONS | PODVM |`:
//! ID is `F<n>`, `F<n>a`, `L<n>` (the rows of sand-net) or `P<n>` (podvm's
//! own); MARK is `observed` or `inferred`; EVIDENCE names a command in
//! backticks or a file under `experiments/results/` that exists; a row of
//! sand-net may be marked `-` and cite its scope issue, because its source
//! and its marks are gone (issue 189); CONDITIONS
//! names the host; PODVM is `re-measured`, `not reproduced`, `not
//! applicable` or `open`. A row with no command and no result file is a
//! claim with no evidence, and the gate refuses it.

use super::*;

/// The page.
pub(super) const FINDINGS: &str = "docs/findings.md";
/// The words of the MARK cell.
const MARKS: [&str; 2] = ["observed", "inferred"];
/// The words of the PODVM cell.
const STATES: [&str; 4] = ["re-measured", "not reproduced", "not applicable", "open"];

/// Whether `id` is the id of a findings row.
fn is_id(id: &str) -> bool {
    let num = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    match id.as_bytes().first() {
        Some(b'F') => {
            let rest = &id[1..];
            num(rest.strip_suffix('a').unwrap_or(rest))
        }
        Some(b'L' | b'P') => num(&id[1..]),
        _ => false,
    }
}

/// The cells of a table line, trimmed; None where the line is no row.
fn cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|c| c.trim().to_string()).collect())
}

/// Whether the evidence cell names a command or a result file of the tree.
/// A row of sand-net (`F`, `L`) may cite the scope issue that keeps its
/// text instead: its source, and the evidence in it, is gone (issue 189).
fn has_evidence(root: &Path, id: &str, cell: &str) -> Result<(), String> {
    let results: Vec<&str> = cell
        .split(|c: char| c.is_whitespace() || c == '`' || c == '(' || c == ')' || c == ',')
        .filter(|w| w.starts_with("experiments/results/"))
        .collect();
    for r in &results {
        if !root.join(r).is_file() {
            return Err(format!("names {r}, and the tree has no such file"));
        }
    }
    let issue = cell
        .split("issue ")
        .skip(1)
        .any(|rest| rest.starts_with(|c: char| c.is_ascii_digit()));
    let imported = id.starts_with(['F', 'L']);
    if !results.is_empty() || cell.matches('`').count() >= 2 || (imported && issue) {
        return Ok(());
    }
    Err(
        "names no command (in backticks) and no file under experiments/results/: \
         a finding with no evidence is a guess"
            .to_string(),
    )
}

pub(super) fn check_findings(gate: &mut Gate, root: &Path) {
    let Ok(text) = read_text(&root.join(FINDINGS)) else {
        gate.err(
            FINDINGS,
            "does not exist, and check 34 holds the findings rows of the scope \
             (TODO/gate.md T-1668)"
                .to_string(),
        );
        return;
    };
    let mut ids = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let Some(c) = cells(line) else {
            continue;
        };
        let Some(id) = c.first().filter(|id| is_id(id)) else {
            continue;
        };
        let at = format!("{FINDINGS}:{}", n + 1);
        gate.bump("findings");
        if c.len() != 7 {
            gate.err(
                &at,
                format!("row {id} has {} cells, and a findings row has 7", c.len()),
            );
            continue;
        }
        if !ids.insert(id.clone()) {
            gate.err(&at, format!("row {id} is a second row of that id"));
        }
        // A row of sand-net whose mark the scope issue lost is `-`.
        let lost = c[2] == "-" && id.starts_with(['F', 'L']);
        if !MARKS.contains(&c[2].as_str()) && !lost {
            gate.err(
                &at,
                format!("row {id} is marked {:?}: observed or inferred", c[2]),
            );
        }
        if let Err(why) = has_evidence(root, id, &c[4]) {
            gate.err(&at, format!("row {id} {why}"));
        }
        if c[5].is_empty() || c[5] == "-" {
            gate.err(
                &at,
                format!("row {id} names no host conditions: a result is a property of its host"),
            );
        }
        if !STATES.contains(&c[6].as_str()) {
            gate.err(
                &at,
                format!(
                    "row {id} has the podvm state {:?}: one of {}",
                    c[6],
                    STATES.join(", ")
                ),
            );
        }
    }
}
