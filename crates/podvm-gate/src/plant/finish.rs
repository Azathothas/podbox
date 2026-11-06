use super::*;

/// The cases from 28 on, the controls and the verdict: the last part of
/// `run`, after `continue_run`.
pub(super) fn finish_run(plant: &mut Plant, root: &Path, scratch: &str) -> i32 {
    // The plant lands in a printed string on purpose, and the octal bytes
    // travel through one variable: a literal backslash-digit run here would
    // read as a regex back reference under some tools and the mutation would
    // land nowhere. The glyph itself is built from bytes, never written out.
    let glyph = String::from_utf8(vec![0xE2u8, 0x9Bu8, 0x94u8]).unwrap_or_default();
    plant.case_plant(
        "28 a glyph in a printed string",
        "prints U+26D4",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm/src/run.rs");
            let out = sub_first_per_line(
                &text,
                "refused: do not run any COMMAND",
                &format!("refused: do not run any COMMAND {glyph}"),
            );
            write_file(root, "crates/podvm/src/run.rs", &out);
            true
        },
    );

    plant.case_plant(
        "29 a dropped cleanup procedure",
        "post-task cleanup procedure",
        scratch,
        &|| {
            let text = read_file(root, "TODO/RULES.md");
            let out = sub_first_per_line(
                &text,
                "Post-task cleanup, after every task",
                "Post-task cleanup, someday",
            );
            write_file(root, "TODO/RULES.md", &out);
            true
        },
    );

    // The lane report is external state. Supply one kept session through a
    // tool fixture and assert that the check names it. The fixture changes
    // no repo file. Two vehicles: the extensionless script runs where the OS
    // execs shebangs, and the batch file runs where it does not.
    {
        let mockbin = plant.backup.join("mockbin");
        let _ = fs::create_dir_all(&mockbin);
        let _ = fs::write(
            mockbin.join("wsl-toolkit"),
            "#!/bin/sh\n[ \"$*\" = \"--instance podvm gc --json\" ] || exit 2\nprintf '%s\\n' '{\"schema\":\"wsl-toolkit-cleanup/1\",\"dry_run\":true,\"containers\":null,\"guest_dirs\":null,\"host_dirs\":null,\"sessions\":[\"plant-session\"]}'\n",
        );
        let _ = fs::write(
            mockbin.join("wsl-toolkit.bat"),
            "@echo off\r\nif \"%*\"==\"--instance podvm gc --json\" (\r\n  echo {\"schema\":\"wsl-toolkit-cleanup/1\",\"dry_run\":true,\"containers\":null,\"guest_dirs\":null,\"host_dirs\":null,\"sessions\":[\"plant-session\"]}\r\n  exit /b 0\r\n)\r\nexit /b 2\r\n",
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                mockbin.join("wsl-toolkit"),
                fs::Permissions::from_mode(0o755),
            );
        }
        let old_path = env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ";" } else { ":" };
        let mock_env = vec![(
            "PATH".to_string(),
            format!("{}{}{}", mockbin.display(), sep, old_path),
        )];
        let name = "29b a kept lane session";
        let want = "lane job still kept: plant-session";
        match plant.run_gate(&mock_env) {
            Ok(child) => {
                let rc = child.code.unwrap_or(1);
                if rc != 0 && child.combined.contains(want) && !plant.base_out.contains(want) {
                    println!("  ok   {name:<34} red, and it named it");
                    plant.plants_caught += 1;
                } else {
                    println!("  MISS {name:<34} the gate did not name the kept session");
                    plant.plants_missed += 1;
                }
            }
            Err(e) => {
                println!("  MISS {name:<34} the gate could not run: {e}");
                plant.plants_missed += 1;
            }
        }
    }

    plant.case_plant(
        "30 a perf reading over its ceiling",
        "perf regression",
        scratch,
        &|| {
            let text = read_file(root, "experiments/results/perf-lane.txt");
            let needle = "run.repeat\tloopback\t";
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if let Some(at) = line.find(needle) {
                        let mut end = at + needle.len();
                        let bytes = line.as_bytes();
                        while end < bytes.len()
                            && (bytes[end].is_ascii_digit() || bytes[end] == b'.')
                        {
                            end += 1;
                        }
                        format!("{}99999.0{}", &line[..at + needle.len()], &line[end..])
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "experiments/results/perf-lane.txt", &out.join("\n"));
            true
        },
    );

    println!();
    plant.case_plant(
        "31 source state differs",
        "source state differs",
        scratch,
        &|| {
            append_file(root, "docs/runtime-state.md", "\nStale generated state.\n");
            true
        },
    );

    run_pins(plant, root, scratch);

    println!();
    println!("== not planted against");
    println!("  16 coverage floor    no case. Planting it means making a check examine");
    println!("                       nothing, which requires editing the gate's own");
    println!("                       matchers rather than the tree. Every other check's");
    println!("                       counter is asserted non-zero on every run instead,");
    println!("                       and a zero is reported as a failure.");
    println!();
    println!("== controls");

    plant.case_control("an ordinary prose edit", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nOne more sentence that cites nothing.\n",
        );
    });

    // A citation that resolves: this binary's own first line.
    plant.case_control("a citation that does resolve", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nThe gate is `crates/podvm-gate/src/plant.rs:1`.\n",
        );
    });

    plant.case_control("a forward reference to a result", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nIt will land in `experiments/results/not-yet-taken.txt`.\n",
        );
    });

    // A payload-side flag after the image is not the tool's flag.
    plant.case_control("a payload-side flag after the image", scratch, &|| {
        let text = read_file(root, "TODO/enter.md");
        let marker = "sh -c 'echo hi'";
        let mut out = text.clone();
        if let Some(at) = text.find(marker) {
            out.replace_range(at..at + marker.len(), "sh -c 'echo hi' --not-a-podvm-flag");
        }
        write_file(root, "TODO/enter.md", &out);
    });

    println!();
    println!("== verdict");
    println!(
        "  plants   {} caught, {} missed",
        plant.plants_caught, plant.plants_missed
    );
    println!(
        "  controls {} quiet, {} fired",
        plant.controls_quiet, plant.controls_fired
    );
    // Guard 4: these two are reported apart. A single "defects caught"
    // number adds the controls in and overstates the coverage by exactly
    // their count.
    if plant.plants_missed == 0 && plant.controls_fired == 0 {
        println!("  every check that was planted against went red with its own message.");
        0
    } else {
        println!("  see the MISS and FIRE lines above.");
        1
    }
}

// Insert the failing pin as the test's first statement, after every line
// opening that test. Each anchor below names exactly one test in its file,
// verified with grep before the change landed; the pin panics before any
// fixture matters, so the run needs no engine, no network, and no binary.
pub(super) fn pin_test(root: &Path, file: &str, anchor: &str) {
    let marker = format!("plant {file}");
    let needle = format!("fn {anchor}(");
    let text = read_file(root, file);
    let mut out = Vec::new();
    for line in text.split('\n') {
        out.push(line.to_string());
        if line.contains(&needle) {
            let indent: String = line.chars().take_while(|c| *c == ' ').collect();
            out.push(format!("{indent}    assert!(false, \"{marker}\");"));
        }
    }
    write_file(root, file, &out.join("\n"));
}

// The shared module holds no test of its own, so the pin is a test of its
// own: it lives in the file, fails with the file's name, and runs inside
// every test binary that includes the module.
pub(super) fn pin_module(root: &Path, file: &str, test: &str) {
    let marker = format!("plant {file}");
    append_file(
        root,
        file,
        &format!("\n#[test]\nfn {test}() {{\n    assert!(false, \"{marker}\");\n}}\n"),
    );
}
