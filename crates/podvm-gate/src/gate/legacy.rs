//! The old view of an entry (TODO/gate.md T-1672).
//!
//! The record is in the format of podcrates' pod-todo: `# T-NNNN: Title`,
//! the header fields `**Name:** value`, and the sections `## Name`. The
//! checks of this gate read an entry in podvm's old layout: ten fields
//! `Name:` with continuation lines indented by 13 columns, then the dated
//! paragraphs. This module builds that layout from the new text in memory,
//! so each check keeps its rule; the files hold only the new format.
//!
//! The layout: the heading `### T-NNNN Title`, the six header fields without
//! Milestone, the five sections Problem to Prove as fields, then the text of
//! `## Done`, `## Blocker`, `## Start condition` and `## Correction`
//! unindented. `## Done` comes first: check 22 reads the first unindented
//! line after `Prove:` as the record of the closing run.

/// The column where the value of a field starts in the old layout.
const WIDTH: usize = 13;
/// The header fields that the old layout holds, in its order.
const HEADER: [&str; 5] = ["Source", "Category", "Priority", "Effort", "Status"];
/// The sections that are fields in the old layout, in its order.
const BODY: [&str; 5] = ["Problem", "Premise", "Approach", "Decision", "Prove"];
/// The sections after the fields, in the order of the old layout.
const AFTER: [&str; 4] = ["Done", "Blocker", "Start condition", "Correction"];

fn field(out: &mut Vec<String>, name: &str, lines: &[String]) {
    let pad = " ".repeat(WIDTH - name.len() - 1);
    let mut first = true;
    for l in lines {
        if first {
            out.push(format!("{name}:{pad}{l}"));
            first = false;
        } else if l.trim().is_empty() {
            out.push(String::new());
        } else {
            out.push(format!("{}{l}", " ".repeat(WIDTH)));
        }
    }
    if first {
        out.push(format!("{name}:{pad}-"));
    }
}

fn trimmed(lines: &[String]) -> Vec<String> {
    let start = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(start, |e| e + 1);
    lines[start..end].to_vec()
}

/// The old layout of the entry whose lines (its H1 first) are `lines`.
pub(super) fn view(id: &str, title: &str, lines: &[String]) -> String {
    let mut header: Vec<(String, Vec<String>)> = Vec::new();
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut fence = false;
    for line in lines.iter().skip(1) {
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        if !fence && let Some(name) = line.strip_prefix("## ") {
            sections.push((name.trim().to_string(), Vec::new()));
            continue;
        }
        if let Some((_, body)) = sections.last_mut() {
            body.push(line.clone());
            continue;
        }
        if let Some(rest) = line.strip_prefix("**")
            && let Some((name, value)) = rest.split_once(":**")
        {
            header.push((name.to_string(), vec![value.trim().to_string()]));
        } else if !line.trim().is_empty()
            && let Some((_, v)) = header.last_mut()
        {
            v.push(line.trim().to_string());
        }
    }
    // A field or a section that the entry does not give is no field of the
    // view: the check of the fields names it.
    let get = |set: &[(String, Vec<String>)], name: &str| {
        set.iter().find(|(n, _)| n == name).map(|(_, v)| trimmed(v))
    };
    let mut out = vec![format!("### {id} {title}"), String::new()];
    for h in HEADER {
        if let Some(v) = get(&header, h) {
            field(&mut out, h, &v);
        }
    }
    out.push(String::new());
    for b in BODY {
        if let Some(v) = get(&sections, b) {
            field(&mut out, b, &v);
        }
    }
    for a in AFTER {
        let body = get(&sections, a).unwrap_or_default();
        if !body.is_empty() {
            out.push(String::new());
            out.extend(body);
        }
    }
    out.push(String::new());
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_entry_reads_as_the_old_layout() {
        let text = "# T-0001: A title\n\n**Source:** issue 1;\n  issue 2\n**Category:** gate\n\
                    **Milestone:** none\n**Priority:** P2\n**Effort:** S\n**Status:** done\n\n\
                    ## Problem\n\nOne line.\n\n  An indented one.\n\n## Premise\n\nP.\n\n\
                    ## Approach\n\nA.\n\n## Decision\n\nD.\n\n## Prove\n\n`true` exits 0.\n\n\
                    ## Correction\n\n**Filed 2026-01-01.** F.\n\n## Done\n\n**Done 2026-01-02.** R.\n";
        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        let v = view("T-0001", "A title", &lines);
        let want = "### T-0001 A title\n\nSource:      issue 1;\n             issue 2\n\
                    Category:    gate\nPriority:    P2\nEffort:      S\nStatus:      done\n\n\
                    Problem:     One line.\n\n               An indented one.\n\
                    Premise:     P.\nApproach:    A.\nDecision:    D.\nProve:       `true` exits 0.\n\n\
                    **Done 2026-01-02.** R.\n\n**Filed 2026-01-01.** F.\n";
        assert_eq!(v, want);
    }
}
