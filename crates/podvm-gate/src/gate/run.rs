use super::*;

pub(super) fn run_gate(root: &Path) -> i32 {
    let todo = root.join("TODO");
    let corpus = load_corpus(root);
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
                        if let Some(row) = rows.get(tid)
                            && status != row.status
                        {
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
            if field == "Category"
                && let Some(row) = rows.get(tid)
                && value != row.category
            {
                gate.err(
                    &where_,
                    format!(
                        "{tid} Category {} disagrees with the row's {}",
                        quote_repr(value),
                        quote_repr(&row.category)
                    ),
                );
            }
            if field == "Priority"
                && let Some(row) = rows.get(tid)
                && value != row.prio
            {
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
        if let Some(remainder) = prove_first_line(&entry.body)
            && !remainder.contains("```")
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

    let derived = check_index_counts(&mut gate, &rows, &index);
    let total = rows.len();

    check_reference_map(&mut gate, &todo, &corpus);
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
                let where_ = format!("TODO/{name}:{}", index_no + 1);
                if cite.cited.starts_with(CORPUS_PREFIX) {
                    if let Some(problem) = corpus_cite_problem(&corpus, &cite) {
                        gate.err(&where_, problem);
                    }
                    continue;
                }
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
