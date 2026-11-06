use super::*;

impl Plant {
    // A T-1579 pin: the file's own test, broken on purpose. The mutation
    // inserts a failing assertion naming the file as the test's first
    // statement, so the test panics before any fixture matters and the
    // failure names the file. The run must go red naming both the test and
    // the planted message; red for any other reason is a miss, exactly like
    // a gate case that reddens on the wrong message.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn case_rust_test(
        &mut self,
        name: &str,
        file: &str,
        test: &str,
        cargo: &[String],
        extra_env: &[(String, String)],
        scratch: &str,
        mutate: &dyn Fn(),
    ) {
        let before = self.hashes();
        mutate();
        let after = self.hashes();
        if before == after {
            println!("  MISS {name:<34} the mutation did not land; the files are byte-identical");
            self.plants_missed += 1;
            self.restore_all(scratch);
            return;
        }
        let marker = format!("plant {file}");
        let (rc, out) = match run_bounded("cargo", cargo, &self.root, extra_env, CARGO_TIMEOUT_SECS)
        {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  MISS {name:<34} the test could not run: {e}");
                self.plants_missed += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        // The marker is constructed, not observed: no clean tree can carry
        // `plant <file>`, so there is no clean-tree arm to check. The test
        // name arm stays: a red run that names neither is red for another
        // reason.
        if rc != 0 && out.contains(test) && out.contains(&marker) {
            println!("  ok   {name:<34} red, and it named it");
            self.plants_caught += 1;
        } else if rc == 0 {
            println!("  MISS {name:<34} the test stayed green");
            self.plants_missed += 1;
        } else {
            println!("  MISS {name:<34} red for another reason, not this one");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.plants_missed += 1;
        }
        // The transcript is evidence, not behaviour: it is written after the
        // restore, so it never moves the hashes above, and only when the
        // caller asked for it by name.
        if let Some(path) = self.transcript.clone() {
            let mut section = format!(
                "--- {name} {file} ---\ntest: {test}\nmutation: insert \
                 `assert!(false, \"{marker}\");` into the test\ncommand: cargo {}\nexit: {rc}\noutput:\n",
                cargo.join(" ")
            );
            section.push_str(&out);
            if !out.ends_with('\n') {
                section.push('\n');
            }
            section.push_str("restored from the backup copy\n\n");
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut old = fs::read_to_string(&path).unwrap_or_default();
            old.push_str(&section);
            let _ = fs::write(&path, old);
        }
    }
}

/// The values `run` reads once and the cases 11 to 18b use.
pub(super) struct CaseInputs<'a> {
    pub(super) bad_b: &'a str,
    pub(super) bad_bare: &'a str,
    pub(super) ceiling: &'a str,
    pub(super) taken: &'a str,
    pub(super) dup_plant: &'a str,
}

/// Cases 11 to 18b of `run`, in its order: one function per half, as
/// `continue_run` is, so no function of this file passes 500 lines.
pub(super) fn middle_run(plant: &mut Plant, root: &Path, scratch: &str, inputs: &CaseInputs) {
    let CaseInputs {
        bad_b,
        bad_bare,
        ceiling,
        taken,
        dup_plant,
    } = *inputs;
    plant.case_plant("11 a tree citation past EOF", "lines", scratch, &|| {
        append_file(
            root,
            "README.md",
            &format!("\nSee `{bad_b}` for the gate.\n"),
        );
        true
    });

    plant.case_plant(
        "12 a link out of README",
        "does not resolve",
        scratch,
        &|| {
            append_file(root, "README.md", "\n[gone](docs/no-such-page.md)\n");
            true
        },
    );

    plant.case_plant(
        "13 a tenth dangling link",
        "dangling link",
        scratch,
        &|| {
            append_file(
                root,
                "docs/conventions/prose.md",
                "\n[gone](../../scripts/no-such-template-script.sh)\n",
            );
            true
        },
    );

    plant.case_plant(
        "14 a bare path naming nothing",
        "git tracks no such file",
        scratch,
        &|| {
            append_file(
                root,
                "README.md",
                &format!("\nThe work is in `{bad_bare}`.\n"),
            );
            true
        },
    );

    // Case 15 stages a file inside a scratch directory and must force-add it,
    // because the ignore rule is what stops it in normal use.
    plant.case_plant(
        "15 tracked experiment scratch",
        "build artefact or experiment scratch",
        scratch,
        &|| {
            let dir = root.join(scratch);
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(dir.join("x.bin"), b"");
            let _ = git(root, &["add", "-f", &format!("{scratch}/x.bin")]);
            true
        },
    );

    plant.case_plant(
        "17a the ceiling in a second file",
        "names the binary size ceiling",
        scratch,
        &|| {
            append_file(
                root,
                "README.md",
                &format!("\nThe release binary must stay under {ceiling} bytes.\n"),
            );
            true
        },
    );

    plant.case_plant(
        "17b a baseline over the ceiling",
        "at or over the ceiling",
        scratch,
        &|| {
            set_bare_reading(
                root,
                "experiments/results/bloat-baseline.txt",
                "total_bytes",
                ceiling,
            );
            true
        },
    );

    plant.case_plant(
        "17c a baseline with no total",
        "carries no `total_bytes <n>` line",
        scratch,
        &|| {
            swap_first_prefixed_line(
                root,
                "experiments/results/bloat-baseline.txt",
                "total_bytes ",
                "total_bytes_renamed ",
            );
            true
        },
    );

    plant.case_plant(
        "17d a shipped reading over the ceiling",
        "at or over the ceiling",
        scratch,
        &|| {
            set_bare_reading(
                root,
                "experiments/results/bloat-image.txt",
                "total_bytes",
                ceiling,
            );
            true
        },
    );

    // No backticks in 18a's planted line, deliberately: in backticks it is
    // also a bare path naming nothing, another check fires too, and the case
    // would pass on the wrong message.
    plant.case_plant(
        "18a a Prove at a taken number",
        "Give the new one a free number",
        scratch,
        &|| {
            append_file(
                root,
                "README.md",
                &format!(
                    "\nThe loop is driven by experiments/{taken}-plantdup.sh, once it exists.\n"
                ),
            );
            true
        },
    );

    plant.case_plant(
        "18b a second script on disk",
        "Give the new one a free number",
        scratch,
        &|| {
            let _ = fs::write(root.join(dup_plant), "#!/bin/sh\n# a plant\n");
            let _ = git(root, &["add", dup_plant]);
            true
        },
    );
    // 18b's staged file is removed by every restore, which finds the
    // duplicate name rather than taking it as a parameter.
}
