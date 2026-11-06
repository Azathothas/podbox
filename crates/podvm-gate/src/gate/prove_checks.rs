use super::*;

pub(super) fn check_prove_flags(gate: &mut Gate, entries: &BTreeMap<String, Entry>, root: &Path) {
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
                            "({tid}) Prove runs `podvm {} {}`, \
                             and `{}` is no verb podvm answers to. A \
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
                        "({tid}) Prove runs `podvm {}`, which the parity \
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
                        "({tid}) Prove names `{bad}` for `podvm {}`, \
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
                            "({tid}) Prove names `{flag}` for `podvm {}`, \
                             which the parity table does not admit: `podvm \
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
            if pos > start
                && let Ok(num) = note[start..pos].parse::<u64>()
            {
                out.push(num);
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

pub(super) fn check_parity_notes(gate: &mut Gate, entries: &BTreeMap<String, Entry>, root: &Path) {
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
pub(super) fn check_ascii_output(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
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
