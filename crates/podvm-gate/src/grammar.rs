//! Row and entry-heading grammars shared by `podvm-gate` and `podvm-count`.
//!
//! This module mirrors the Python gate exactly: `ROW` and `HEADING`, with
//! the field, status, and priority vocabularies beside them. Both binaries parse `TODO/INDEX.md` rows and `### T-` entry
//! headers, so the two grammars live here once (T-1551). The port in T-1552 is
//! behaviour-preserving, and the characterization tests below are the proof
//! that it preserves these two matchers: every fixture is a verbatim line
//! from the tree, cited beside it.
//!
//! Two deliberate approximations, both recorded because a silent one would be a
//! defect. Python `\s` and `\d` match Unicode whitespace and Unicode digits;
//! this module matches ASCII whitespace and ASCII digits. Every line this
//! grammar reads is prose-rule ASCII (`docs/conventions/prose.md`), so the
//! two agree on every input the tree can hold.

/// One `TODO/INDEX.md` table row in the format of pod-todo (TODO/gate.md
/// T-1672): `| [T-NNNN](file) | Pn | effort | milestone | category | status | item |`.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexRow {
    pub id: String,
    pub file: String,
    pub priority: String,
    pub effort: String,
    pub milestone: String,
    pub category: String,
    pub status: String,
    pub item: String,
}

/// One entry heading: `# T-NNNN: <title>`.
#[derive(Debug, PartialEq, Eq)]
pub struct EntryHeading {
    pub id: String,
    pub title: String,
}

/// The ten entry fields the record gate (`podvm-gate`) requires, in order.
pub const FIELDS: [&str; 10] = [
    "Source", "Category", "Priority", "Effort", "Status", "Problem", "Premise", "Approach",
    "Decision", "Prove",
];

/// The four row and entry states the record gate admits.
pub const STATUSES: [&str; 4] = ["open", "partial", "blocked", "done"];

/// The four priorities the record gate admits.
pub const PRIORITIES: [&str; 4] = ["P0", "P1", "P2", "P3"];

fn is_task_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 6
        && bytes[0] == b'T'
        && bytes[1] == b'-'
        && bytes[2..6].iter().all(|b| b.is_ascii_digit())
}

/// Parse the `[T-NNNN](file)` link cell of an index row.
fn parse_link_cell(cell: &str) -> Option<(String, String)> {
    let cell = cell.trim();
    let rest = cell.strip_prefix('[')?;
    let close = rest.find(']')?;
    let id = &rest[..close];
    if !is_task_id(id) {
        return None;
    }
    let after = &rest[close + 1..];
    let file = after.strip_prefix('(')?.strip_suffix(')')?;
    if file.is_empty() || file.contains(')') {
        return None;
    }
    Some((id.to_string(), file.to_string()))
}

/// Parse the status cell: up to two `*` each side of a lowercase word.
///
/// The Python source is `\*{0,2}([a-z]+)\*{0,2}`: each side carries zero to
/// two stars independently, and the pair is not required to match.
fn parse_status_cell(cell: &str) -> Option<String> {
    let cell = cell.trim();
    let leading = cell.len() - cell.trim_start_matches('*').len();
    if leading > 2 {
        return None;
    }
    let rest = &cell[leading..];
    let word_len = rest
        .find(|c: char| !c.is_ascii_lowercase())
        .unwrap_or(rest.len());
    let word = &rest[..word_len];
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    let trailing = &rest[word_len..];
    if trailing.len() > 2 || !trailing.bytes().all(|b| b == b'*') {
        return None;
    }
    Some(word.to_string())
}

/// Parse one `TODO/INDEX.md` row into its cells.
///
/// The line opens with `|`, closes with `|` past optional whitespace, and
/// holds seven inner cells, as pod-todo reads them. The item cell may itself
/// hold `|` characters: the item runs to the final pipe.
pub fn parse_index_row(line: &str) -> Option<IndexRow> {
    let line = line.trim_end_matches(['\r', '\n']);
    let inner = line.strip_prefix('|')?;
    let inner = inner.trim_end();
    let inner = inner.strip_suffix('|')?;
    let cells: Vec<&str> = inner.split('|').collect();
    if cells.len() < 7 {
        return None;
    }
    let (id, file) = parse_link_cell(cells[0])?;
    let priority = cells[1].trim();
    if priority.len() != 2
        || priority.as_bytes()[0] != b'P'
        || !priority.as_bytes()[1].is_ascii_digit()
    {
        return None;
    }
    let effort = cells[2].trim();
    if !["S", "M", "L"].contains(&effort) {
        return None;
    }
    let milestone = cells[3].trim();
    if milestone.is_empty() || milestone.contains(' ') {
        return None;
    }
    let category = cells[4].trim();
    if category.is_empty()
        || !category
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b == b'-')
    {
        return None;
    }
    let status = parse_status_cell(cells[5])?;
    let item = cells[6..].join("|").trim().to_string();
    if item.is_empty() {
        return None;
    }
    Some(IndexRow {
        id,
        file,
        priority: priority.to_string(),
        effort: effort.to_string(),
        milestone: milestone.to_string(),
        category: category.to_string(),
        status,
        item,
    })
}

/// Parse one `# T-NNNN: <title>` entry heading, the H1 of pod-todo.
///
/// `# `, the id, `: `, then a title opening with a non-space character.
/// The title is kept verbatim, trailing spaces included.
pub fn parse_entry_heading(line: &str) -> Option<EntryHeading> {
    let line = line.trim_end_matches(['\r', '\n']);
    let rest = line.strip_prefix("# ")?;
    let id = rest.get(..6)?;
    if !is_task_id(id) {
        return None;
    }
    let title = rest.get(6..)?.strip_prefix(": ")?;
    if title.is_empty() || title.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    Some(EntryHeading {
        id: id.to_string(),
        title: title.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Characterization of ROW: verbatim rows the Python matcher accepts.
    #[test]
    fn row_plain_status() {
        // A row of TODO/INDEX.md in the format of pod-todo.
        let line = "| [T-0101](probe.md) | P0 | M | none | probe | done | The probe set, one disposable child per probe, errno not boolean |";
        let row = parse_index_row(line).expect("the plain row must parse");
        assert_eq!(row.id, "T-0101");
        assert_eq!(row.file, "probe.md");
        assert_eq!(row.priority, "P0");
        assert_eq!(row.category, "probe");
        assert_eq!(row.status, "done");
        assert_eq!(
            row.item,
            "The probe set, one disposable child per probe, errno not boolean"
        );
    }

    #[test]
    fn row_bold_status() {
        // The status cell may carry `**`.
        let line = "| [T-1001](packaging.md) | P0 | M | none | packaging | **done** | A single static binary with no `PT_INTERP` |";
        let row = parse_index_row(line).expect("the bold row must parse");
        assert_eq!(row.id, "T-1001");
        assert_eq!(row.status, "done");
        assert_eq!(row.item, "A single static binary with no `PT_INTERP`");
    }

    #[test]
    fn row_item_may_hold_pipes() {
        // The item runs to the final pipe.
        let line = "| [T-9999](probe.md) | P3 | M | none | probe | open | A \\| B \\| C |";
        let row = parse_index_row(line).expect("pipes in the item must parse");
        assert_eq!(row.item, "A \\| B \\| C");
    }

    #[test]
    fn row_unbalanced_stars_match_like_the_regex() {
        // `\*{0,2}` applies per side with no pairing: Python accepts both.
        for status in ["**done", "done**", "*done*", "done"] {
            let line = format!("| [T-0101](probe.md) | P0 | M | none | probe | {status} | Item |");
            let row = parse_index_row(&line).expect("must parse");
            assert_eq!(row.status, "done");
        }
    }

    #[test]
    fn row_rejects_what_row_rejects() {
        // Each line fails exactly one ROW atom: short id, missing file,
        // two-letter priority, uppercase category, triple-star status,
        // empty item, missing final pipe.
        let bad = [
            "| [T-101](probe.md) | P0 | M | none | probe | done | Item |",
            "| [T-0101]() | P0 | M | none | probe | done | Item |",
            "| [T-0101](probe.md) | PX | M | none | probe | done | Item |",
            "| [T-0101](probe.md) | P0 | M | none | Probe | done | Item |",
            "| [T-0101](probe.md) | P0 | M | none | probe | ***done*** | Item |",
            "| [T-0101](probe.md) | P0 | M | none | probe | done |  |",
            "| [T-0101](probe.md) | P0 | M | none | probe | done | Item ",
            "| [T-0101](probe.md) | P0 | XL | none | probe | done | Item |",
            "| [T-0101](probe.md) | P0 | probe | done | The old row of podvm |",
        ];
        for line in bad {
            assert!(parse_index_row(line).is_none(), "must reject: {line}");
        }
    }

    // The H1 of an entry in the format of pod-todo.
    #[test]
    fn heading_plain() {
        let line = "# T-0101: The probe set, one disposable child per probe, errno not boolean";
        let head = parse_entry_heading(line).expect("the heading must parse");
        assert_eq!(head.id, "T-0101");
        assert_eq!(
            head.title,
            "The probe set, one disposable child per probe, errno not boolean"
        );
    }

    #[test]
    fn heading_rejects_what_heading_rejects() {
        let bad = [
            "## T-0101: Two hashes, not one",
            "### T-0101 The old heading of podvm",
            "# T-101: Three id digits, not four",
            "# T-0101 No colon after the id",
            "# T-0101: ",
        ];
        for line in bad {
            assert!(parse_entry_heading(line).is_none(), "must reject: {line}");
        }
    }

    // The shared vocabulary both binaries parse against.
    #[test]
    fn vocabulary_matches_the_gate() {
        assert_eq!(
            FIELDS,
            [
                "Source", "Category", "Priority", "Effort", "Status", "Problem", "Premise",
                "Approach", "Decision", "Prove",
            ]
        );
        assert_eq!(STATUSES, ["open", "partial", "blocked", "done"]);
        assert_eq!(PRIORITIES, ["P0", "P1", "P2", "P3"]);
    }
}
