//! Row and entry-heading grammars shared by `podbox-gate` and `podbox-count`.
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

/// One `TODO/INDEX.md` table row: `| [T-NNNN](file) | Pn | category | status | item |`.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexRow {
    pub id: String,
    pub file: String,
    pub priority: String,
    pub category: String,
    pub status: String,
    pub item: String,
}

/// One entry heading: `### T-NNNN <title>`.
#[derive(Debug, PartialEq, Eq)]
pub struct EntryHeading {
    pub id: String,
    pub title: String,
}

/// The ten entry fields the record gate (`podbox-gate`) requires, in order.
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

/// Parse one `TODO/INDEX.md` row into its six cells.
///
/// Mirrors `ROW`: the line opens with `|`, closes with `|` past optional
/// whitespace, and holds five inner cells. The item cell may itself hold
/// `|` characters: like the lazy `(.+?)\s*\|` anchored at end of line, the
/// item runs to the final pipe.
pub fn parse_index_row(line: &str) -> Option<IndexRow> {
    let line = line.trim_end_matches(['\r', '\n']);
    let inner = line.strip_prefix('|')?;
    let inner = inner.trim_end();
    let inner = inner.strip_suffix('|')?;
    let cells: Vec<&str> = inner.split('|').collect();
    if cells.len() < 5 {
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
    let category = cells[2].trim();
    if category.is_empty()
        || !category
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b == b'-')
    {
        return None;
    }
    let status = parse_status_cell(cells[3])?;
    let item = cells[4..].join("|").trim().to_string();
    if item.is_empty() {
        return None;
    }
    Some(IndexRow {
        id,
        file,
        priority: priority.to_string(),
        category: category.to_string(),
        status,
        item,
    })
}

/// Parse one `### T-NNNN <title>` entry heading.
///
/// Mirrors `HEADING`: `###`, spaces, the id, spaces, then a title opening
/// with a non-space character. The title is kept verbatim, trailing spaces
/// included, exactly as the capture group holds them.
pub fn parse_entry_heading(line: &str) -> Option<EntryHeading> {
    let line = line.trim_end_matches(['\r', '\n']);
    let rest = line.strip_prefix("### ")?;
    let id = rest.get(..6)?;
    if !is_task_id(id) {
        return None;
    }
    // ` +`: one space minimum, then the title opens with a non-space.
    let title = rest.get(6..)?.strip_prefix(' ')?;
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
        // TODO/INDEX.md:61, verbatim.
        let line = "| [T-0101](probe.md) | P0 | probe | done | The probe set, one disposable child per probe, errno not boolean |";
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
        // TODO/INDEX.md:161, verbatim: the status cell carries `**`.
        let line = "| [T-1001](packaging.md) | P0 | packaging | **done** | A single static binary with no `PT_INTERP` |";
        let row = parse_index_row(line).expect("the bold row must parse");
        assert_eq!(row.id, "T-1001");
        assert_eq!(row.status, "done");
        assert_eq!(row.item, "A single static binary with no `PT_INTERP`");
    }

    #[test]
    fn row_item_may_hold_pipes() {
        // Mirrors `(.+?)\s*\|\s*$`: the item runs to the final pipe.
        let line = "| [T-9999](probe.md) | P3 | probe | open | A \\| B \\| C |";
        let row = parse_index_row(line).expect("pipes in the item must parse");
        assert_eq!(row.item, "A \\| B \\| C");
    }

    #[test]
    fn row_unbalanced_stars_match_like_the_regex() {
        // `\*{0,2}` applies per side with no pairing: Python accepts both.
        for status in ["**done", "done**", "*done*", "done"] {
            let line = format!("| [T-0101](probe.md) | P0 | probe | {status} | Item |");
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
            "| [T-101](probe.md) | P0 | probe | done | Item |",
            "| [T-0101]() | P0 | probe | done | Item |",
            "| [T-0101](probe.md) | PX | probe | done | Item |",
            "| [T-0101](probe.md) | P0 | Probe | done | Item |",
            "| [T-0101](probe.md) | P0 | probe | ***done*** | Item |",
            "| [T-0101](probe.md) | P0 | probe | done |  |",
            "| [T-0101](probe.md) | P0 | probe | done | Item ",
        ];
        for line in bad {
            assert!(parse_index_row(line).is_none(), "must reject: {line}");
        }
    }

    // Characterization of HEADING: verbatim headings the Python accepts.
    #[test]
    fn heading_plain() {
        // TODO/probe.md:22, verbatim.
        let line = "### T-0101 The probe set, one disposable child per probe, errno not boolean";
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
            "## T-0101 Two hashes, not three",
            "### T-101 Three id digits, not four",
            "### T-0101No space after the id",
            "### T-0101 ",
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
