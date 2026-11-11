use super::*;

pub(super) fn run() -> i32 {
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SKIP: cannot locate the repository root");
            return 2;
        }
    };
    let gate = match gate_path() {
        Some(gate) => gate,
        None => {
            eprintln!("SKIP: cannot locate the gate binary beside this binary");
            return 2;
        }
    };
    if !is_executable(&gate) {
        eprintln!("SKIP: {} is not executable", gate.display());
        return 2;
    }
    if run_bounded("git", &["--version".to_string()], &root, &[], 30).is_err() {
        eprintln!("SKIP: no git");
        return 2;
    }

    // Refuse to start over staged work. The restore is safe, but a dirty
    // index means the clean baseline below is not clean, and every case then
    // compares against a tree somebody was mid-edit on.
    let file_args: Vec<String> = FILES.iter().map(|f| f.to_string()).collect();
    let mut diff_args = vec!["diff".to_string(), "--quiet".to_string(), "--".to_string()];
    diff_args.extend(file_args.clone());
    let work_dirty = run_bounded("git", &diff_args, &root, &[], 60)
        .map(|out| out.code.unwrap_or(1) != 0)
        .unwrap_or(true);
    let mut cached_args = vec![
        "diff".to_string(),
        "--cached".to_string(),
        "--quiet".to_string(),
        "--".to_string(),
    ];
    cached_args.extend(file_args);
    let index_dirty = run_bounded("git", &cached_args, &root, &[], 60)
        .map(|out| out.code.unwrap_or(1) != 0)
        .unwrap_or(true);
    if work_dirty || index_dirty {
        eprintln!("SKIP: one of the files this script plants into has uncommitted changes:");
        let mut status_args = vec![
            "status".to_string(),
            "--porcelain".to_string(),
            "--".to_string(),
        ];
        status_args.extend(FILES.iter().map(|f| f.to_string()));
        if let Ok(out) = run_bounded("git", &status_args, &root, &[], 60) {
            eprintln!("{}", out.combined.trim_end());
        }
        eprintln!("Commit or stash them. This script must own the tree while it runs.");
        return 2;
    }

    let pid = std::process::id();
    let backup = env::temp_dir().join(format!("podvm-plant-{pid}"));
    if fs::create_dir_all(&backup).is_err() {
        eprintln!("SKIP: cannot create a backup directory");
        return 2;
    }
    let scratch = "experiments/.plantscratch";

    let mut plant = Plant {
        root: root.clone(),
        gate,
        backup: backup.clone(),
        base_out: String::new(),
        plants_caught: 0,
        plants_missed: 0,
        controls_quiet: 0,
        controls_fired: 0,
        transcript: match env::var("PODVM_PLANT_TRANSCRIPT") {
            Ok(value) if !value.trim().is_empty() => {
                let path = PathBuf::from(value.trim());
                Some(if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                })
            }
            _ => None,
        },
    };
    if plant.backup_all().is_err() {
        eprintln!("SKIP: cannot back up the listed files");
        let _ = fs::remove_dir_all(&backup);
        return 2;
    }

    println!("== baseline: the gate on the clean tree");
    let (base_rc, base_out) = match plant.run_gate(&[]) {
        Ok(child) => (child.code.unwrap_or(1), child.combined),
        Err(e) => {
            eprintln!("SKIP: the gate could not run: {e}");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    println!("  exit {base_rc}");
    if base_rc != 0 {
        eprintln!("SKIP: the gate is already red. Nothing below would be interpretable.");
        for line in base_out.lines() {
            eprintln!("    {line}");
        }
        plant.restore_all(scratch);
        let _ = fs::remove_dir_all(&backup);
        return 2;
    }
    println!();
    plant.base_out = base_out;

    // The needles are assembled or read here, before the first case. The
    // gate reads this source like any other tracked file, so none of these
    // values may appear in it literally.
    let gate_source = gate_source_name();
    let past_eof = read_file(&root, &gate_source).split('\n').count() + 1;
    let bad_a = format!("{gate_source}:{past_eof}");
    let bad_b = format!("{gate_source}:{past_eof}");
    let bad_bare = bad_bare_name();
    let ceiling = match ceiling_from_file(&root, "experiments/110-bloat-delta.sh", "CEILING_BYTES")
    {
        Some(value) => value,
        None => {
            eprintln!("SKIP: experiments/110-bloat-delta.sh declares no CEILING_BYTES");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let interpose_ceiling = match ceiling_from_file(
        &root,
        "scripts/build-interpose.sh",
        "INTERPOSE_CEILING_BYTES",
    ) {
        Some(value) => value,
        None => {
            eprintln!("SKIP: scripts/build-interpose.sh declares no INTERPOSE_CEILING_BYTES");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let taken = match taken_experiment(&root) {
        Some(num) => num,
        None => {
            eprintln!("SKIP: experiments/ carries no numbered script to collide with");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let dup_plant = format!("experiments/{taken}-plantdup.sh");

    // Fresh evidence per run: truncate the transcript before the first case,
    // when the caller asked for one. The write is best-effort and happens
    // outside every case, so it never moves the hashes guard 1 compares and
    // never touches the verdict. Unset, the binary prints only the verdict
    // lines, exactly like the script.
    if let Some(path) = plant.transcript.clone() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, "");
    }

    println!("== plants");

    plant.case_plant(
        "1 row without an entry",
        "has a row and no entry",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/INDEX.md",
                "| [T-9999](probe.md) | P0 | S | none | probe | open | A row naming nothing |\n",
            );
            true
        },
    );

    plant.case_plant(
        "2 entry without a row",
        "has an entry and no row",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/probe.md",
                "\n# T-9998: An entry no row names\n\n**Status:** open\n",
            );
            true
        },
    );

    // Status-agnostic, for the same reason cases 4 and 10 are count-agnostic.
    // A plant must mutate whatever status is there.
    plant.case_plant(
        "3 status disagreement",
        "disagrees with the index",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/probe.md");
            let mut from = String::new();
            for line in text.split('\n') {
                if let Some(rest) = line.strip_prefix("**Status:** ") {
                    let word: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase())
                        .collect();
                    if !word.is_empty() {
                        from = word;
                        break;
                    }
                }
            }
            if from.is_empty() {
                return false;
            }
            let to = if from == "blocked" { "open" } else { "blocked" };
            let text = text.split('\n').collect::<Vec<_>>();
            let mut done = false;
            let out: Vec<String> = text
                .into_iter()
                .map(|line| {
                    if !done && line.starts_with(&format!("**Status:** {from}")) {
                        done = true;
                        format!(
                            "**Status:** {to}{}",
                            &line["**Status:** ".len() + from.len()..]
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(&root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // Count-agnostic. A plant must mutate whatever number is there.
    plant.case_plant(
        "4 count block is stale",
        "the totals line is not the rows",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/INDEX.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    let mut parts = line.splitn(2, " items: ");
                    if let (Some(head), Some(tail)) = (parts.next(), parts.next())
                        && !head.is_empty()
                        && head.bytes().all(|b| b.is_ascii_digit())
                        && tail.contains(" open")
                    {
                        let mut tail_parts = tail.splitn(2, ' ');
                        if let (Some(_), Some(rest)) = (tail_parts.next(), tail_parts.next()) {
                            return format!("{head} items: 999 {rest}");
                        }
                    }
                    line.to_string()
                })
                .collect();
            write_file(&root, "TODO/INDEX.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "5 a missing field",
        "has no `Decision:` field",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/probe.md");
            let mut done = false;
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    // TODO/gate.md T-1672: the section `## Decision`.
                    if !done && line == "## Decision" {
                        done = true;
                        "## Decisionx".to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(&root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // Every mention, not the first row. Each tree is named twice in the map,
    // so deleting one row leaves the other and the check never loses it.
    plant.case_plant(
        "6 a corpus tree the map lost",
        "and the map does not name it",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/reference-map.md");
            let out = text.replace("`references/qaidvoid__onelf`", "the onelf tree");
            write_file(&root, "TODO/reference-map.md", &out);
            true
        },
    );

    // The trees left the live tree on 2026-11-06. A citation into one resolves
    // only against references/MANIFEST.tsv, so a path the manifest does not
    // list must turn the gate red.
    plant.case_plant(
        "6b a corpus citation the manifest does not list",
        "MANIFEST.tsv lists no such file",
        scratch,
        &|| {
            // Assembled here, so this source line is not a citation itself.
            let missing = ["references", "qaidvoid__onelf", "tree", "no-such-file.rs"].join("/");
            append_file(
                &root,
                "TODO/probe.md",
                &format!("\nSee `{missing}:1` for this.\n"),
            );
            true
        },
    );

    plant.case_plant("7 a citation past end of file", "lines", scratch, &|| {
        append_file(
            &root,
            "TODO/probe.md",
            &format!("\nSee `{bad_a}` for this.\n"),
        );
        true
    });

    plant.case_plant(
        "8 a TODO link to nothing",
        "does not resolve",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/probe.md",
                "\nSee [nothing](no-such-file.md).\n",
            );
            true
        },
    );

    plant.case_plant(
        "9 a T-NNNN naming nothing",
        "which is not an entry",
        scratch,
        &|| {
            append_file(&root, "TODO/probe.md", "\nBlocked behind T-8888.\n");
            true
        },
    );

    plant.case_plant(
        "10 PROGRESS count is stale",
        "the count line is not the rows",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/PROGRESS.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    let mut parts = line.splitn(2, " entries: ");
                    if let (Some(head), Some(tail)) = (parts.next(), parts.next())
                        && !head.is_empty()
                        && head.bytes().all(|b| b.is_ascii_digit())
                        && tail.contains(" open")
                    {
                        let mut tail_parts = tail.splitn(2, ' ');
                        if let (Some(_), Some(rest)) = (tail_parts.next(), tail_parts.next()) {
                            return format!("{head} entries: 999 {rest}");
                        }
                    }
                    line.to_string()
                })
                .collect();
            write_file(&root, "TODO/PROGRESS.md", &out.join("\n"));
            true
        },
    );

    middle_run(
        &mut plant,
        &root,
        scratch,
        &CaseInputs {
            bad_b: &bad_b,
            bad_bare: &bad_bare,
            ceiling: &ceiling,
            taken: &taken,
            dup_plant: &dup_plant,
        },
    );
    let code = continue_run(&mut plant, &root, scratch, &interpose_ceiling);
    plant.restore_all(scratch);
    let _ = fs::remove_dir_all(&backup);
    code
}
