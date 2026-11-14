use super::*;

pub(super) fn continue_run(
    plant: &mut Plant,
    root: &Path,
    scratch: &str,
    interpose_ceiling: &str,
) -> i32 {
    // The component is read out of the wrapper at run time, like the
    // ceilings. Naming it here would be a second declaration of the value
    // this check exists to keep in one place.
    let (cc_wrapper, cc_component) = match cc_component(root) {
        Some(pair) => pair,
        None => {
            eprintln!("SKIP: the cargo config names no readable toolchain wrapper");
            return 2;
        }
    };
    let _ = cc_wrapper;

    // The build tool's own name is never written out here, in a case title
    // or in a pattern: this harness is itself a script the check reads, so
    // that name here would read as this harness running it.
    plant.case_plant(
        "19a a build job without its toolchain",
        "does not carry",
        scratch,
        &|| {
            let text = read_file(root, ".github/workflows/gate.yml");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| strip_component_line(line, &cc_component))
                .collect();
            write_file(root, ".github/workflows/gate.yml", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "19b the same, reached by a script",
        "runs cargo (through",
        scratch,
        &|| {
            // Both direct build spellings go: the workspace musl line and
            // the gate-crate `-p` lines the wave-4 ports added. What remains
            // reaches cargo only through scripts/build-interpose.sh, which is
            // the arm this case exists to catch.
            let text = read_file(root, ".github/workflows/gate.yml");
            let out: Vec<String> = text
                .split('\n')
                .filter(|line| {
                    !line.contains("--release --target")
                        && !line.contains("cargo build --release -p")
                })
                .map(|line| strip_component_line(line, &cc_component))
                .collect();
            write_file(root, ".github/workflows/gate.yml", &out.join("\n"));
            true
        },
    );

    // The needle is assembled at run time. The check reads every crate
    // source, so a literal copy of that declaration written here would be a
    // second one the day the check's scope widens to this directory.
    let exit_decl = exit_declaration();
    plant.case_plant(
        "20 a second exit-code declaration",
        "already holds docker",
        scratch,
        &|| {
            append_file(
                root,
                "crates/podvm-supervise/src/lib.rs",
                &format!("\n{exit_decl}\n"),
            );
            true
        },
    );

    plant.case_plant(
        "21a an unqualified image in a Prove line",
        "unqualified image reference",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            // TODO/gate.md T-1672: a line of a section `## Prove`.
            let mut in_prove = false;
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if line.starts_with("## ") || line.starts_with("# T-") {
                        in_prove = line == "## Prove";
                    }
                    if in_prove && line.contains("public.ecr.aws/docker/library/alpine:3.20") {
                        line.replacen(
                            "public.ecr.aws/docker/library/alpine:3.20",
                            "alpine:latest",
                            1,
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "21b a docker.io image in a Prove line",
        "which pulls from Docker Hub",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            // TODO/gate.md T-1672: a line of a section `## Prove`.
            let mut in_prove = false;
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if line.starts_with("## ") || line.starts_with("# T-") {
                        in_prove = line == "## Prove";
                    }
                    if in_prove && line.contains("public.ecr.aws/docker/library/alpine:3.20") {
                        line.replacen(
                            "public.ecr.aws/docker/library/alpine:3.20",
                            "docker.io/library/alpine:latest",
                            1,
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // The plant is global on purpose rather than aimed at one entry: any
    // single entry named here would rot the way cases 3, 4 and 10 did.
    plant.case_plant(
        "22 a closed entry with no recorded run",
        "closes without a recorded run",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if let Some(rest) = line.strip_prefix("**Done") {
                        format!("*Done{rest}")
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "23a an interpose object over the ceiling",
        "at or over the interpose ceiling",
        scratch,
        &|| {
            set_bare_reading(
                root,
                "experiments/results/bloat-interpose.txt",
                "interpose_gnu_bytes",
                interpose_ceiling,
            );
            true
        },
    );

    plant.case_plant(
        "23b an interpose reading with no sizes",
        "carries no `interpose_gnu_bytes",
        scratch,
        &|| {
            swap_first_prefixed_line(
                root,
                "experiments/results/bloat-interpose.txt",
                "interpose_gnu_bytes ",
                "interpose_gnu_bytes_renamed ",
            );
            true
        },
    );

    // The needle is a token pair, and the plant removes one half of it.
    // T-1557 moved the comparison into the binary: the mutation breaks the
    // `nm` spelling there, which the export check reads. The old
    // build-interpose.sh target no longer carries the needle, so planting
    // there lands nowhere (guard 1 caught exactly that).
    plant.case_plant(
        "24 the export comparison removed",
        "carries no export-set comparison",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm-gate/src/interpose_build.rs");
            let out = text.replace("--defined-only", "--defined-onlx");
            write_file(root, "crates/podvm-gate/src/interpose_build.rs", &out);
            true
        },
    );

    // The arm, retargeted. A step that exits 2 then reaches no arm that
    // names it, and the holding check must go red rather than pass a gate
    // that stays silent about skips. T-1555 ported the shell driver to
    // dev.rs: the mutation moves the match's `2 =>` arm, which the devcheck
    // reads, off its status.
    plant.case_plant(
        "25 the SKIP arm retargeted",
        "has no SKIP arm",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm-gate/src/dev/steps.rs");
            let needle = "\n            2 => {";
            let Some(at) = text.find(needle) else {
                return false;
            };
            let mut out = text.to_string();
            out.replace_range(at + 1..at + needle.len(), "9 => {");
            write_file(root, "crates/podvm-gate/src/dev/steps.rs", &out);
            true
        },
    );

    plant.case_plant(
        "26a a done Prove naming a refused flag",
        // `--network` is accepted since `--network broker` (T-1665), so the
        // plant names a flag that the table still refuses.
        "Prove names `--privileged`",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(
                &text,
                "`! podvm run --network=none --rm",
                "`podvm run --privileged --rm",
            );
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26b a done Prove naming an unlisted flag",
        "Prove names `--frobnicate`",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(
                &text,
                "podvm probe --strict",
                "podvm probe --strict --frobnicate",
            );
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26c a done Prove running a refused verb",
        "refuses outright",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(&text, "podvm probe --strict", "podvm pause");
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26d a done Prove running an unknown verb",
        "is no verb",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out =
                sub_first_per_line(&text, "podvm probe --strict", "podvm frobnicate --strict");
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26e a done Prove with a bad cluster member",
        "Prove names `-Z`",
        scratch,
        &|| {
            append_file(
                root,
                "TODO/probe.md",
                "\n# T-9998: Cluster plant\n\n**Source:** plant\n**Category:** probe\n**Milestone:** none\n**Priority:** P3\n**Effort:** S\n**Status:** done\n\n## Problem\n\nplant\n\n## Premise\n\nplant\n\n## Approach\n\nplant\n\n## Decision\n\nplant\n\n## Prove\n\n`podvm ps -aZ`\n\n## Done\n\n**Done 2026-01-01.** plant.\n",
            );
            true
        },
    );

    plant.case_plant(
        "27a a parity note leaning on a shipped milestone",
        "leans on M4",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm/src/parity.rs");
            let out = sub_first_per_line(
                &text,
                "says which half failed (TODO/cli.md T-1331)",
                "needs a supervisor, which is M4",
            );
            write_file(root, "crates/podvm/src/parity.rs", &out);
            true
        },
    );

    plant.case_plant(
        "27b a parity note missing a present verb",
        "claims `prune`",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm/src/parity.rs");
            let out = sub_first_per_line(
                &text,
                "events are not implemented",
                "events, prune and df are not implemented",
            );
            write_file(root, "crates/podvm/src/parity.rs", &out);
            true
        },
    );

    plant.case_plant(
        "27c a curated flag with no parity row",
        "names `--read-only`",
        scratch,
        &|| {
            let text = read_file(root, "crates/podvm/src/parity.rs");
            let out: Vec<String> = text
                .split('\n')
                .filter(|line| !line.contains("--read-only"))
                .map(|line| line.to_string())
                .collect();
            write_file(root, "crates/podvm/src/parity.rs", &out.join("\n"));
            true
        },
    );

    // TODO/machine.md T-1879: the retired sentence of the DOS refusal.
    plant.case_plant(
        "27d a message that says podvm fetches no base",
        "a retired sentence",
        scratch,
        &|| {
            let file = "crates/podvm/src/windows/dos.rs";
            let text = read_file(root, file);
            let planted = concat!("No base is ", "fetched");
            write_file(root, file, &format!("{text}// {planted} here.\n"));
            true
        },
    );

    finish_run(plant, root, scratch)
}
