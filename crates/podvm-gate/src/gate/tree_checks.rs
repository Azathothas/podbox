use super::*;

pub(super) fn find_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    for line in body.split('\n') {
        let Some(rest) = line.strip_prefix(field) else {
            continue;
        };
        if !rest.starts_with(':') {
            continue;
        }
        let after = &rest[1..];
        let spaces = after.len() - after.trim_start_matches(' ').len();
        if spaces == 0 {
            continue;
        }
        let value = &after[spaces..];
        if value.is_empty() || value.starts_with(' ') {
            continue;
        }
        let _ = value;
        return Some(after.trim_start_matches(' ').trim_end());
    }
    None
}

pub(super) struct Entry {
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) title: String,
    pub(super) body: String,
}

pub(super) fn contains_number_with_boundaries(line: &str, number: &str) -> bool {
    if number.is_empty() {
        return false;
    }
    let bytes = line.as_bytes();
    let needle = number.as_bytes();
    let mut at = 0;
    while at + needle.len() <= bytes.len() {
        if &bytes[at..at + needle.len()] == needle {
            let before_ok = at == 0 || !bytes[at - 1].is_ascii_digit();
            let after_ok =
                at + needle.len() == bytes.len() || !bytes[at + needle.len()].is_ascii_digit();
            if before_ok && after_ok {
                return true;
            }
        }
        at += 1;
    }
    false
}

pub(super) fn check_tree(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let corpus = load_corpus(root);
    let mut dangling_verbatim = 0;
    for rel in files.iter() {
        if rel.starts_with(CORPUS_PREFIX) || rel.starts_with(HISTORY_PREFIX) {
            continue;
        }
        let ours = is_ours(rel);
        let is_md = rel.ends_with(".md");
        if !is_md && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let mut target_cache: BTreeMap<String, Option<usize>> = BTreeMap::new();
        for (index, line) in text.split('\n').enumerate() {
            let n = index + 1;
            if ours {
                for cite in find_cites(line) {
                    if rel.starts_with("TODO/") {
                        continue;
                    }
                    gate.bump("tree_citations");
                    let where_ = format!("{rel}:{n}");
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
                    if !files.contains(&cite.cited) {
                        gate.err(
                            &where_,
                            format!(
                                "cites {}, which exists here and is NOT \
                                 tracked by git, so a fresh clone does not have it",
                                cite.cited
                            ),
                        );
                        continue;
                    }
                    let count = *target_cache.entry(cite.cited.clone()).or_insert_with(|| {
                        fs::read(&target).ok().map(|data| count_lines_bytes(&data))
                    });
                    let Some(count) = count else {
                        continue;
                    };
                    let last = cite.end.unwrap_or(cite.start);
                    if last > count {
                        gate.err(
                            &where_,
                            format!(
                                "cites {}:{}, and that file has {count} lines",
                                cite.cited, cite.start
                            ),
                        );
                    }
                }
            }
            if ours && !line.contains(KNOWN_ABSENT) {
                for cited in find_bares(line) {
                    gate.bump("bare_citations");
                    if cited.starts_with(FORWARD_REF_PREFIX) {
                        continue;
                    }
                    let stem = cited.trim_end_matches('/');
                    if files.contains(stem) {
                        continue;
                    }
                    if stem.starts_with(CORPUS_PREFIX) && corpus_names(&corpus, stem) {
                        continue;
                    }
                    let stem_slash = format!("{stem}/");
                    if files.iter().any(|f| f.starts_with(&stem_slash)) {
                        continue;
                    }
                    gate.err(
                        &format!("{rel}:{n}"),
                        format!(
                            "names `{cited}`, and git tracks no such file or \
                             directory. A citation that resolves on one disk and \
                             not in a fresh clone is the defect this check exists \
                             for."
                        ),
                    );
                }
            }
            if !is_md {
                continue;
            }
            for link in find_links(line) {
                let mut href = link.href;
                if href.starts_with("http://")
                    || href.starts_with("https://")
                    || href.starts_with("mailto:")
                {
                    continue;
                }
                if let Some(hash) = href.find('#') {
                    href = href[..hash].to_string();
                }
                href = href.trim().to_string();
                if href.is_empty() {
                    continue;
                }
                let dir = Path::new(rel).parent().unwrap_or(Path::new(""));
                let target = lexical_normalize(&dir.join(&href));
                let exists = root.join(&target).exists();
                if ours {
                    if rel.starts_with("TODO/") {
                        continue;
                    }
                    gate.bump("tree_links");
                    if !exists {
                        gate.err(
                            &format!("{rel}:{n}"),
                            format!("link target {href} does not resolve"),
                        );
                    } else {
                        let target_str = target.to_string_lossy().replace('\\', "/");
                        if root.join(&target).is_file() && !files.contains(&target_str) {
                            gate.err(
                                &format!("{rel}:{n}"),
                                format!(
                                    "links to {href}, which exists here and is NOT \
                                     tracked by git"
                                ),
                            );
                        }
                    }
                } else if !exists {
                    dangling_verbatim += 1;
                }
            }
        }
    }
    if dangling_verbatim != KNOWN_VERBATIM_DANGLING {
        gate.err(
            "docs/",
            format!(
                "{dangling_verbatim} dangling link(s) in the imported \
                 methodology; expected {KNOWN_VERBATIM_DANGLING}. \
                 Reconcile the import or provide the missing target."
            ),
        );
    }
}
pub(super) fn check_size_ceiling(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    if !files.contains(CEILING_SCRIPT) {
        gate.err(
            CEILING_SCRIPT,
            "does not exist or is not tracked, so the release \
             binary has no declared ceiling. TODO/deps.md T-0910."
                .to_string(),
        );
        return;
    }
    gate.bump("size_ceiling");
    let Ok(script) = read_text(&root.join(CEILING_SCRIPT)) else {
        gate.err(
            CEILING_SCRIPT,
            "declares no `CEILING_BYTES=<n>` line. That line is \
             the ceiling's one home; without it every other file \
             naming a size is unanchored."
                .to_string(),
        );
        return;
    };
    let Some(ceiling) = search_decl(&script, "CEILING_BYTES") else {
        gate.err(
            CEILING_SCRIPT,
            "declares no `CEILING_BYTES=<n>` line. That line is \
             the ceiling's one home; without it every other file \
             naming a size is unanchored."
                .to_string(),
        );
        return;
    };

    for rel in files.iter() {
        if rel == CEILING_SCRIPT || !is_ours(rel) {
            continue;
        }
        if !rel.ends_with(".md") && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        gate.bump("size_ceiling");
        for (index, line) in text.split('\n').enumerate() {
            if contains_number_with_boundaries(line, &ceiling) {
                gate.err(
                    &format!("{}:{}", rel, index + 1),
                    format!(
                        "names the binary size ceiling {ceiling} itself. It is \
                         declared in {CEILING_SCRIPT} and nowhere else; call that \
                         script instead of copying its number."
                    ),
                );
            }
        }
    }

    if !files.contains(BLOAT_BASELINE) {
        gate.err(
            BLOAT_BASELINE,
            "is not tracked. TODO/deps.md T-0910: without a committed baseline \
             there is no `before`, and a dependency that lands without a \
             before-and-after number has not landed. Take it with \
             `./experiments/110-bloat-delta.sh baseline`."
                .to_string(),
        );
        return;
    }
    gate.bump("size_ceiling");
    let Ok(baseline) = read_text(&root.join(BLOAT_BASELINE)) else {
        gate.err(
            BLOAT_BASELINE,
            "carries no `total_bytes <n>` line, so the baseline \
             cannot be compared with anything."
                .to_string(),
        );
        return;
    };
    if search_spaced_decl(&baseline, "total_bytes").is_none() {
        gate.err(
            BLOAT_BASELINE,
            "carries no `total_bytes <n>` line, so the baseline \
             cannot be compared with anything."
                .to_string(),
        );
        return;
    }

    let limit: u64 = ceiling.parse().unwrap_or(u64::MAX);
    let mut readings: Vec<&String> = files
        .iter()
        .filter(|f| f.starts_with("experiments/results/bloat-") && f.ends_with(".txt"))
        .collect();
    readings.sort();
    for rel in readings {
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let Some(found) = search_spaced_decl(&text, "total_bytes") else {
            continue;
        };
        gate.bump("size_ceiling");
        let total: u64 = found.parse().unwrap_or(0);
        if total >= limit {
            gate.err(
                rel,
                format!(
                    "records total_bytes {total}, which is at or over the ceiling \
                     of {limit} declared in {CEILING_SCRIPT}. Raise the ceiling \
                     deliberately, with the delta that justifies it committed \
                     beside the change."
                ),
            );
        }
    }
}

pub(super) fn check_experiment_numbers(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let mut numbers: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut record = |num: String, name: String, where_: String| {
        gate.bump("experiment_numbers");
        numbers
            .entry(num)
            .or_default()
            .entry(name)
            .or_insert(where_);
    };
    for rel in files.iter() {
        if let Some(exp) = is_experiment_file(rel) {
            record(exp.num, exp.name, rel.clone());
        }
    }
    for rel in files.iter() {
        if !is_ours(rel) {
            continue;
        }
        if !rel.ends_with(".md") && !has_source_suffix(rel) {
            continue;
        }
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let mut skipped = BTreeSet::new();
        for (index, line) in split_lines(&text).into_iter().enumerate() {
            if line.contains(KNOWN_ABSENT) {
                skipped.insert(index + 1);
            }
        }
        for (exp, line, _) in find_experiments(&text) {
            if skipped.contains(&line) {
                continue;
            }
            record(exp.num, exp.name, format!("{rel}:{line}"));
        }
    }
    let mut nums: Vec<(&String, &BTreeMap<String, String>)> = numbers.iter().collect();
    nums.sort_by_key(|(num, _)| num.parse::<u64>().unwrap_or(u64::MAX));
    for (num, names) in nums {
        if names.len() < 2 {
            continue;
        }
        let listed: Vec<String> = names
            .iter()
            .map(|(name, where_)| format!("`{num}-{name}.sh` ({where_})"))
            .collect();
        gate.err(
            &format!("experiments/{num}-"),
            format!(
                "experiment number {num} carries {} names: {}. \
                 experiments/README.md rules that a number is never reused, \
                 because a citation of it has to keep meaning what it meant. \
                 Give the new one a free number. TODO/gate.md T-1205.",
                names.len(),
                listed.join(", ")
            ),
        );
    }
}
pub(super) fn check_ci_components(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    let required = cargo_toolchain_components(root);
    let mut workflows: Vec<&String> = files.iter().filter(|f| f.starts_with(WORKFLOWS)).collect();
    workflows.sort();
    for rel in workflows {
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        let jobs = workflow_jobs(&text);
        let mut names: Vec<&WorkflowJob> = jobs.iter().collect();
        names.sort_by(|a, b| a.name.cmp(&b.name));
        for job in names {
            let body = uncommented(&job.body);
            let mut runs_cargo = is_cargo_call(&body);
            let mut via = None;
            for script in find_repo_scripts(&body) {
                if !files.contains(&script) {
                    continue;
                }
                let Ok(script_body) = read_text(&root.join(&script)) else {
                    continue;
                };
                if is_cargo_call(&uncommented(&script_body)) {
                    runs_cargo = true;
                    via = Some(script);
                    break;
                }
            }
            if !runs_cargo {
                continue;
            }
            let have = bootstrap_components(&body);
            let through = match &via {
                Some(script) if !is_cargo_call(&body) => {
                    format!(" (through `{script}`)")
                }
                _ => String::new(),
            };
            let mut comps: Vec<&String> = required.keys().collect();
            comps.sort();
            for comp in comps {
                gate.bump("ci_components");
                if have.contains(comp) {
                    continue;
                }
                gate.err(
                    rel,
                    format!(
                        "job `{}` runs cargo{through} and its \
                         `bootstrap-env.sh` list does not carry `{comp}`. \
                         `{CARGO_CONFIG}` points a cargo build at \
                         `{}`, which says it is installed by \
                         `bootstrap-env.sh {comp}`, so the job dies inside a \
                         dependency's build script rather than on its own step. \
                         TODO/gate.md T-1206.",
                        job.name, required[comp]
                    ),
                );
            }
        }
    }
}
