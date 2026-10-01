//! podbox-count: the writer half of the work-todo pair, as a binary.
//!
//! A behaviour-preserving port of the retired todo-count script (T-1553). It
//! re-derives every count in `TODO/INDEX.md` from the rows and rewrites the
//! Counts block in place, and with `--set T-NNNN <status>` it moves one
//! entry's status in both the row and the entry so the two cannot drift.
//! Arguments are parsed by hand; `clap` was ruled out at T-0908.
//!
//! Two deliberate approximations, both untriggerable on a green tree.
//! Python `\s`, `\d`, `[a-z]` match Unicode; this module matches ASCII
//! whitespace, ASCII digits, ASCII lowercase (the prose rule holds ASCII,
//! and T-1551 records the same approximation for the shared grammar).
//! Python `repr` on a hostile `--set` status escapes backslashes and picks
//! quotes; this module wraps in single quotes unless the value holds one,
//! then double quotes, without backslash escapes.
//!
//! One intentional divergence from `grammar.rs`: the shared row parser
//! rejects an empty item cell, while the Python `ROW` accepts it (`.*`).
//! The writer mirrors the Python exactly, so `--set` and the counts move
//! precisely the rows the old script moved. The differential proof (lane,
//! `--check` plus a `--set` round-trip against the deleted script restored
//! beside the tree) pins the agreement.
//!
//! `find_root` below duplicates `main.rs`'s copy on purpose: the two
//! binaries share no library target (a `[lib]` would be a new export
//! surface), and editing the proven gate file to share twelve lines is
//! the larger change.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

const STATUSES: [&str; 4] = ["open", "partial", "blocked", "done"];
const PRIORITIES: [&str; 4] = ["P0", "P1", "P2", "P3"];

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(candidate);
        }
        candidate = candidate.join("..");
    }
    None
}

fn is_task_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 6
        && bytes[0] == b'T'
        && bytes[1] == b'-'
        && bytes[2..6].iter().all(|b| b.is_ascii_digit())
}

/// A row the Python `ROW` accepts, with the status word's byte span so
/// `--set` rewrites spacing byte-for-byte. Groups: 1 link cell through the
/// pipe before the priority, 3 priority, 4 through the pipe after the
/// category, 5 the status word with its stars, 6 the rest.
struct MatchedRow {
    id: String,
    priority: String,
    status: String,
    status_bold: bool,
    status_start: usize,
    status_end: usize,
}

fn match_row(line: &str) -> Option<MatchedRow> {
    let bytes = line.as_bytes();
    if bytes.first() != Some(&b'|') {
        return None;
    }
    // Five pipes minimum: lead, link, priority, category, status, item.
    let mut pipes = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'|' {
            pipes.push(i);
        }
    }
    if pipes.len() < 5 {
        return None;
    }
    let cell = |n: usize| &line[pipes[n] + 1..pipes[n + 1]];
    // Group 2: the [T-NNNN](file) link.
    let link = cell(0).trim();
    let rest = link.strip_prefix('[')?;
    let close = rest.find(']')?;
    let id = &rest[..close];
    if !is_task_id(id) {
        return None;
    }
    let after = rest.get(close + 1..)?;
    let file = after.strip_prefix('(')?.strip_suffix(')')?;
    if file.is_empty() || file.contains(')') {
        return None;
    }
    // Group 3: P0-P3 (Python says P\d; the tree holds P0-P3).
    let prio = cell(1).trim();
    if prio.len() != 2 || prio.as_bytes()[0] != b'P' || !prio.as_bytes()[1].is_ascii_digit() {
        return None;
    }
    // Group 4's middle: lowercase-dash category.
    let category = cell(2).trim();
    if category.is_empty()
        || !category
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b == b'-')
    {
        return None;
    }
    // Group 5: stars plus word; the span covers the trimmed cell so the
    // surrounding spaces stay exactly where they were.
    let raw_status = cell(3);
    let leading_ws = raw_status.len() - raw_status.trim_start().len();
    let trailing_ws = raw_status.len() - raw_status.trim_end().len();
    let word = &raw_status[leading_ws..raw_status.len() - trailing_ws];
    let strip_stars = word.trim_matches('*');
    let star_count = word.len() - strip_stars.len();
    if star_count > 4
        || strip_stars.is_empty()
        || !strip_stars.bytes().all(|b| b.is_ascii_lowercase())
        || word.bytes().filter(|b| *b != b'*').count() != strip_stars.len()
    {
        return None;
    }
    // Stars must sit at the edges, at most two per side, like \*{0,2}[a-z]+\*{0,2}.
    let leading_stars = word.len() - word.trim_start_matches('*').len();
    let trailing_stars = word.len() - word.trim_end_matches('*').len();
    if leading_stars > 2 || trailing_stars > 2 {
        return None;
    }
    let base = pipes[3] + 1;
    Some(MatchedRow {
        id: id.to_string(),
        priority: prio.to_string(),
        status: strip_stars.to_string(),
        status_bold: word.starts_with("**"),
        status_start: base + leading_ws,
        status_end: base + raw_status.len() - trailing_ws,
    })
}

fn py_list(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render(status_cell_starts_bold: bool, status: &str) -> String {
    if status_cell_starts_bold {
        format!("**{status}**")
    } else {
        status.to_string()
    }
}

fn py_repr(value: &str) -> String {
    if value.contains('\'') {
        format!("\"{value}\"")
    } else {
        format!("'{value}'")
    }
}

fn read_text(path: &std::path::Path) -> Option<String> {
    String::from_utf8(std::fs::read(path).ok()?).ok()
}

fn run(argv: &[String]) -> ExitCode {
    let Some(root) = find_root() else {
        eprintln!("todo-count: cannot locate the repository root");
        return ExitCode::from(2);
    };
    let todo = root.join("TODO");
    let index_path = todo.join("INDEX.md");
    if !index_path.is_file() {
        eprintln!("todo-count: TODO/INDEX.md does not exist");
        return ExitCode::from(2);
    }

    let check_only = argv.iter().any(|a| a == "--check");
    let mut set_id: Option<String> = None;
    let mut set_status: Option<String> = None;
    if let Some(i) = argv.iter().position(|a| a == "--set") {
        if argv.len() < i + 3 {
            eprintln!("todo-count: --set needs an id and a status");
            return ExitCode::from(2);
        }
        let (id, status) = (argv[i + 1].clone(), argv[i + 2].clone());
        if !is_task_id(&id) {
            eprintln!("todo-count: {id} is not a T-NNNN id");
            return ExitCode::from(2);
        }
        if !STATUSES.contains(&status.as_str()) {
            eprintln!(
                "todo-count: {} is not one of {}",
                py_repr(&status),
                py_list(&STATUSES)
            );
            return ExitCode::from(2);
        }
        set_id = Some(id);
        set_status = Some(status);
    }

    let Some(index_text) = read_text(&index_path) else {
        eprintln!("todo-count: TODO/INDEX.md is not UTF-8");
        return ExitCode::from(1);
    };
    let mut lines: Vec<String> = index_text.split('\n').map(|l| l.to_string()).collect();
    // Python splitlines on "a\n" gives ["a"]; split('\n') gives ["a", ""].
    // The splice below re-terminates, so drop the artefact empty tail now
    // to keep line indices aligned with the script's.
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    // -- move one status, in the row and in the entry ----------------------
    if let (Some(id), Some(status)) = (set_id.as_ref(), set_status.as_ref()) {
        let mut moved = false;
        for line in lines.iter_mut() {
            if let Some(m) = match_row(line) {
                if &m.id == id {
                    line.replace_range(
                        m.status_start..m.status_end,
                        &render(m.status_bold, status),
                    );
                    moved = true;
                }
            }
        }
        if !moved {
            eprintln!("todo-count: {id} has no row in TODO/INDEX.md");
            return ExitCode::from(1);
        }
        let mut hits = 0;
        let mut names: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&todo) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".md") {
                    names.push(name);
                }
            }
        }
        names.sort();
        for name in &names {
            let path = todo.join(name);
            let Some(text) = read_text(&path) else {
                continue;
            };
            let heading = format!("### {id} ");
            if !text.split('\n').any(|l| l.starts_with(&heading)) {
                continue;
            }
            let Some(head) = text.find(&heading) else {
                continue;
            };
            let tail = &text[head + 1..];
            let next_rel = tail.find("\n### T-");
            let nxt = match next_rel {
                Some(r) => head + 1 + r,
                None => text.len(),
            };
            let body = &text[head..nxt];
            // First Status line: ^(Status: +)\*{0,2}[a-z]+\*{0,2}, stars dropped.
            let mut replaced: Option<String> = None;
            let mut offset = 0;
            for line in body.split('\n') {
                let line_len = line.len();
                if let Some(rest) = line.strip_prefix("Status:") {
                    let spaces = rest.len() - rest.trim_start_matches(' ').len();
                    if spaces > 0 {
                        let after = &rest[spaces..];
                        let word_len = after
                            .find(|c: char| c != '*' && !c.is_ascii_lowercase())
                            .unwrap_or(after.len());
                        let word = &after[..word_len];
                        let core = word.trim_matches('*');
                        let lead = word.len() - word.trim_start_matches('*').len();
                        let trail = word.len() - word.trim_end_matches('*').len();
                        if !core.is_empty()
                            && core.bytes().all(|b| b.is_ascii_lowercase())
                            && lead <= 2
                            && trail <= 2
                        {
                            // Replace the match only, not the line: the
                            // script's subn leaves whatever follows the word.
                            let match_end = offset + 7 + spaces + word_len;
                            let new_prefix = format!("Status:{}{}", &rest[..spaces], status);
                            let mut new_body = String::with_capacity(body.len());
                            new_body.push_str(&body[..offset]);
                            new_body.push_str(&new_prefix);
                            new_body.push_str(&body[match_end..]);
                            replaced = Some(new_body);
                            break;
                        }
                    }
                }
                offset += line_len + 1;
            }
            if let Some(new_body) = replaced {
                let mut new_text = String::with_capacity(text.len());
                new_text.push_str(&text[..head]);
                new_text.push_str(&new_body);
                new_text.push_str(&text[nxt..]);
                if std::fs::write(&path, new_text).is_err() {
                    eprintln!("todo-count: cannot write {}", path.display());
                    return ExitCode::from(1);
                }
                hits += 1;
            }
        }
        if hits == 0 {
            eprintln!("todo-count: {id} has a row but no entry to move");
            return ExitCode::from(1);
        }
    }

    // -- derive --------------------------------------------------------------
    let mut derived = [0usize; 4];
    let mut per_prio = [[0usize; 4]; 4];
    let mut bad = 0;
    for line in &lines {
        let Some(m) = match_row(line) else { continue };
        let s = STATUSES.iter().position(|s| *s == m.status);
        let p = PRIORITIES.iter().position(|p| *p == m.priority);
        let (Some(s), Some(p)) = (s, p) else {
            eprintln!(
                "todo-count: row {} has status {} priority {}",
                m.id,
                py_repr(&m.status),
                py_repr(&m.priority)
            );
            bad += 1;
            continue;
        };
        derived[s] += 1;
        per_prio[p][s] += 1;
    }
    if bad > 0 {
        return ExitCode::from(1);
    }
    let total: usize = derived.iter().sum();
    if total == 0 {
        eprintln!("todo-count: no rows found");
        return ExitCode::from(1);
    }

    let mut block = vec![
        format!(
            "{} items: {} open, {} partial, {} blocked, {} done.",
            total, derived[0], derived[1], derived[2], derived[3]
        ),
        String::new(),
        "Counted from the rows above by `./target/release/podbox-count` and asserted".to_string(),
        "independently by `./target/release/podbox-gate`, which is the gate. A number here"
            .to_string(),
        "that disagrees with the rows cannot reach a commit.".to_string(),
        String::new(),
        "| Priority | Open | Partial | Blocked | Done | Total |".to_string(),
        "| --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for (p, counts) in PRIORITIES.iter().zip(per_prio.iter()) {
        block.push(format!(
            "| {p} | {} | {} | {} | {} | {} |",
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            counts.iter().sum::<usize>()
        ));
    }
    block.push(format!(
        "| **All** | **{}** | **{}** | **{}** | **{}** | **{}** |",
        derived[0], derived[1], derived[2], derived[3], total
    ));

    if check_only {
        println!("{}", block.join("\n"));
        return ExitCode::from(0);
    }

    // -- splice --------------------------------------------------------------
    let Some(start) = lines.iter().position(|l| l == "## Counts") else {
        eprintln!("todo-count: TODO/INDEX.md has no `## Counts` heading");
        return ExitCode::from(1);
    };
    let mut end = start + 1;
    while end < lines.len() && !lines[end].starts_with("## ") {
        end += 1;
    }
    let mut new_lines = lines[..start + 1].to_vec();
    new_lines.push(String::new());
    new_lines.extend(block);
    new_lines.push(String::new());
    new_lines.extend_from_slice(&lines[end..]);
    let out = new_lines.join("\n").trim_end_matches('\n').to_string() + "\n";
    if std::fs::write(&index_path, out).is_err() {
        eprintln!("todo-count: cannot write TODO/INDEX.md");
        return ExitCode::from(1);
    }
    println!(
        "todo-count: wrote {total} items: {} open, {} partial, {} blocked, {} done",
        derived[0], derived[1], derived[2], derived[3]
    );
    ExitCode::from(0)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    run(&argv)
}
