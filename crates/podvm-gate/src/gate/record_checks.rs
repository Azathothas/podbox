use super::*;

pub(super) fn check_exit_codes(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
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
                     error is: `pub use podvm_probe::exit::{name};` is the \
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
        if let Some(id) = rest.get(..6)
            && is_task_id(id)
        {
            heads.push((index, id.to_string()));
        }
    }
    heads
}

pub(super) fn prove_block(lines: &[String], start: usize, end: usize) -> usize {
    let mut j = start + 1;
    while j < end && (lines[j].starts_with(' ') || lines[j].starts_with('\t')) {
        j += 1;
    }
    j
}

pub(super) fn check_prove_registry(gate: &mut Gate, todo: &Path) {
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
pub(super) fn todo_names(todo: &Path) -> Result<Vec<String>, ()> {
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

pub(super) fn check_closure_records(gate: &mut Gate, entries: &BTreeMap<String, Entry>) {
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

pub(super) fn check_interpose_sizes(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
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
            if let Some(rest) = line.strip_prefix(&head)
                && (rest == "absent"
                    || (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())))
            {
                got.insert(which.to_string(), rest.to_string());
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
pub(super) fn check_interpose_exports(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
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

pub(super) fn check_devcheck_third_state(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
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
