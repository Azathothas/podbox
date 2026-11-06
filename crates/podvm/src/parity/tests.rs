use super::*;

#[test]
fn every_parity_note_is_plain_ascii() {
    // TODO/cli.md T-1336: the parity table is machine-readable
    // (`system info --format '{{json .Parity}}'`), so a note needing
    // a codepoint above U+007F forces every downstream parser to
    // handle bytes that carry no meaning.
    let bad: Vec<(&str, usize)> = TABLE
        .iter()
        .map(|r| (r.verb, r.note.bytes().filter(|b| *b > 0x7F).count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    assert!(bad.is_empty(), "non-ASCII parity notes: {bad:?}");
}

/// ⭐ T-0801's `Prove`, as a unit test as well as a command: the table is
/// the contract, so its size and its vocabulary are asserted here rather
/// than only in an experiment somebody has to remember to run.
#[test]
fn the_table_is_at_least_sixty_rows_and_uses_only_the_four_statuses() {
    assert!(TABLE.len() >= 60, "the table has only {} rows", TABLE.len());
    for r in TABLE {
        assert!(
            ["Native", "Degraded", "Stub", "None"].contains(&r.status.word()),
            "{} has a fifth status",
            r.verb
        );
        assert!(!r.note.is_empty(), "{} has no note", r.verb);
    }
}

/// ⛔ A `None` row exists to be a sentence, so an empty reason makes the row
/// worse than absent: the caller is told no and not why.
#[test]
fn every_unimplemented_row_says_why() {
    for r in TABLE.iter().filter(|r| r.status == Status::None) {
        assert!(
            r.note.len() > 20,
            "{} {:?} refuses without a reason a caller can act on",
            r.verb,
            r.flag
        );
    }
}

#[test]
fn a_flag_is_found_under_every_spelling_the_parser_accepts() {
    assert_eq!(flag("run", "-e").map(|r| r.status), Some(Status::Native));
    assert_eq!(flag("run", "--env").map(|r| r.status), Some(Status::Native));
    // ⚠ `--env=A=1` is one argument and the row is for the flag.
    assert_eq!(
        flag("run", "--env=A=1").map(|r| r.status),
        Some(Status::Native)
    );
    // ⚠ `--name` was a `None` row until M4 and is `Native` now, so the
    // refusal case is taken from a row that is still one rather than from a
    // name that changed meaning under the test.
    assert_eq!(
        flag("run", "--name").map(|r| r.status),
        Some(Status::Native)
    );
    assert_eq!(
        flag("run", "--privileged").map(|r| r.status),
        Some(Status::None)
    );
    assert!(flag("run", "--nope").is_none());
    // ⚠ Per verb, not global: `--entrypoint` is run's and exec has no such
    // flag, so exec must not find run's row.
    assert!(flag("exec", "--entrypoint").is_none());
}

#[test]
fn no_verb_carries_the_same_flag_twice() {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for r in TABLE {
        let Some(spellings) = r.flag else { continue };
        for s in spellings.split(',') {
            let key = (r.verb, s.trim());
            assert!(!seen.contains(&key), "{} carries {} twice", key.0, key.1);
            seen.push(key);
        }
    }
}

#[test]
fn every_flag_row_names_a_verb_the_table_also_has_a_row_for() {
    for r in TABLE.iter().filter(|r| r.flag.is_some()) {
        assert!(
            verb(r.verb).is_some(),
            "{} has flags and no verb row",
            r.verb
        );
    }
}

/// TODO/cli.md T-1331. `--filter` is `name=` and `label=` only, and
/// anything else refuses naming the two keys that exist.
#[test]
fn filter_parses_name_and_label_and_nothing_else() {
    let f = parse_filter("name=web").unwrap();
    assert_eq!(f.key, "name");
    assert_eq!(f.value, "web");
    let f = parse_filter("label=key=value").unwrap();
    assert_eq!(f.key, "label");
    assert_eq!(f.value, "key=value");
    assert!(parse_filter("status=running").is_err());
    assert!(parse_filter("name").is_err());
    assert!(parse_filter("=x").is_err());
}

/// TODO/cli.md T-1330. Docker's cluster rule exactly: value-less
/// shorts split, a value-taking member consumes the rest, an unknown
/// member refuses naming the member. Longs, lone shorts and values
/// starting with `-` pass through untouched.
#[test]
fn cluster_expands_docker_clusters() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(expand("ps", &v(&["-aq"])), Ok(v(&["-a", "-q"])));
    assert_eq!(expand("run", &v(&["-it"])), Ok(v(&["-i", "-t"])));
    // ⚠ Per verb: `-t` takes a value for `stop` and is a boolean for
    // `run`, so the same spelling expands two ways.
    assert_eq!(expand("stop", &v(&["-t5"])), Ok(v(&["-t", "5"])));
    assert_eq!(
        expand("run", &v(&["-eFOO=bar", "img"])),
        Ok(v(&["-e", "FOO=bar", "img"]))
    );
    // ⛔ Unknown member refuses naming the member, not the cluster.
    assert_eq!(expand("ps", &v(&["-aZ"])), Err("-Z".to_string()));
    assert_eq!(expand("ps", &v(&["-Zq"])), Err("-Z".to_string()));
    // Untouched: longs, lone shorts, `-`, values. A digit member is
    // an unknown member, exactly as docker reads it.
    assert_eq!(
        expand("ps", &v(&["--format", "x", "-a", "-", "img"])),
        Ok(v(&["--format", "x", "-a", "-", "img"]))
    );
    assert_eq!(expand("ps", &v(&["-a1"])), Err("-1".to_string()));
    // ⚠ Past a value-taking member the rest is opaque, braces and
    // all: the template never parses as members.
    assert_eq!(
        expand("inspect", &v(&["-f{{.Id}}", "img"])),
        Ok(v(&["-f", "{{.Id}}", "img"]))
    );
    // ⚠ The image boundary: `run` and `exec` stop expanding past
    // it, so payload flags reach the payload. Every other verb
    // reads its whole argv, and clusters expand anywhere in it.
    assert_eq!(
        expand("run", &v(&["-it", "img", "ls", "-la"])),
        Ok(v(&["-i", "-t", "img", "ls", "-la"]))
    );
    assert_eq!(
        expand("stop", &v(&["c1", "-t5"])),
        Ok(v(&["c1", "-t", "5"]))
    );
    // ⚠ `create` is served by `run`'s parser, so it expands by run's rows.
    assert_eq!(expand("create", &v(&["-it"])), Ok(v(&["-i", "-t"])));
}

/// ⛔ `CLUSTER_VALUES` is the one declaration of which shorts take
/// values. A member listed here with no table row would expand into a
/// flag `admit` then refuses as unknown, so the list is held to the
/// table it expands against.
#[test]
fn every_cluster_value_has_a_table_row() {
    for (under, c) in CLUSTER_VALUES {
        let one = format!("-{c}");
        assert!(
            flag(under, &one).is_some(),
            "{under} takes a value for `{one}` with no table row"
        );
    }
}

/// ⛔ `podvm create --no-steps` was ACCEPTED and acted on while
/// `podvm system info` listed `--no-steps` for `run` and `exec` only, so
/// the table under-described the binary and the refusal message sent a
/// caller to a row set that does not exist. Both halves are asserted here:
/// a verb that borrows another's rows resolves them, and it says whose.
#[test]
fn a_verb_served_by_another_parser_resolves_that_verbs_rows_and_says_so() {
    assert_eq!(rows_of("create"), "run");
    assert_eq!(rows_of("run"), "run", "a verb with its own rows keeps them");
    assert_eq!(rows_of("exec"), "exec");

    // The flag `create` was silently taking, now found through the table.
    assert!(
        flag("create", "--no-steps").is_some(),
        "create is served by run's parser, so run's rows must resolve for it"
    );
    assert!(
        flag("create", "--no-such-flag").is_none(),
        "borrowing rows must not admit a flag that has no row anywhere"
    );

    // ⚠ The row a reader is sent to has to name the borrowing, or the
    // caller looks for `create --no-steps` in the table and finds nothing.
    let note = verb("create").expect("create has a verb row").note;
    assert!(
        note.contains("run's flag set"),
        "the create row must say whose rows it takes, got {note:?}"
    );
}

#[test]
fn the_json_carries_every_row_and_the_status_word() {
    let doc: serde_json::Value = serde_json::from_str(&json()).unwrap();
    let arr = doc.as_array().unwrap();
    assert_eq!(arr.len(), TABLE.len());
    assert!(arr.iter().all(|r| r["status"].is_string()));
    assert!(arr
        .iter()
        .any(|r| r["verb"] == "run" && r["flag"].is_null()));
}

/// T-0808: the `image` and `system` groups dispatch to one handler per
/// subverb, so each subverb resolves the rows its flags live under and
/// the refusal names the path the caller typed.
#[test]
fn group_subverbs_resolve_the_rows_their_flags_live_under() {
    for (sub, home) in [
        ("image ls", "images"),
        ("image list", "images"),
        ("image rm", "rmi"),
        ("image remove", "rmi"),
        ("image prune", "prune"),
        ("image tag", "tag"),
        ("image inspect", "inspect"),
        ("image pull", "pull"),
        ("image extract", "extract"),
        ("info", "info"),
        ("system info", "info"),
        ("system install-names", "install-names"),
        ("system abi", "abi"),
        ("system df", "df"),
    ] {
        assert_eq!(rows_of(sub), home, "{sub} resolves under {home}");
    }
    assert!(flag("image prune", "-a").is_some());
    assert!(flag("image prune", "--no-such-flag").is_none());
    assert!(flag("system install-names", "--dir").is_some());
    assert!(flag("info", "--format").is_some());
}

/// T-1404: the SSH verbs parse their flags literally (the windows
/// precedent), so the table cannot bind them through `admit`. What
/// binds them instead is this test: every flag row for the verb names
/// a flag one of its parsers takes, and every flag the parsers take
/// has a row. A flag landing in one and missing the other goes red
/// here rather than drifting silently.
#[test]
fn ssh_flag_rows_match_their_parsers_in_both_directions() {
    use std::collections::HashSet;
    let serve: HashSet<&str> = crate::remote::serve::FLAGS.iter().copied().collect();
    let connect: HashSet<&str> = crate::remote::connect::FLAGS.iter().copied().collect();
    let both: HashSet<&str> = serve.union(&connect).copied().collect();
    let machine: HashSet<&str> = crate::machine::ssh::FLAGS.iter().copied().collect();
    // Help is structural (every parser prints usage on -h/--help)
    // rather than a member of the flag lists.
    let mut remote = both.clone();
    remote.insert("-h");
    remote.insert("--help");
    let mut mssh = machine.clone();
    mssh.insert("-h");
    mssh.insert("--help");
    for (verb, accepted) in [("remote ssh", remote), ("machine ssh", mssh)] {
        let mut rowed: HashSet<&str> = HashSet::new();
        for r in TABLE.iter().filter(|r| r.verb == verb) {
            if let Some(spellings) = r.flag {
                for s in spellings.split(',') {
                    rowed.insert(s.trim());
                }
            }
        }
        for spelled in &rowed {
            assert!(
                accepted.contains(spelled),
                "{verb} rows {spelled} but no parser takes it"
            );
        }
        for taken in &accepted {
            assert!(
                rowed.contains(taken),
                "{verb} parsers take {taken} with no table row"
            );
        }
    }
}

/// T-0808: the pre-pass refuses before any arm runs, and passes
/// listed flags and positionals through.
#[test]
fn admit_all_refuses_before_any_arm_runs() {
    let usage = "usage: podvm images";
    assert_eq!(
        admit_all("images", &["--no-such-flag".to_string()], usage),
        Some(podvm_image::error::EXIT_FLAG_ERROR)
    );
    assert_eq!(admit_all("images", &["-a".to_string()], usage), None);
    assert_eq!(
        admit_all("images", &["some-image".to_string()], usage),
        None
    );
    assert_eq!(admit_all("images", &[], usage), None);
}

/// ⭐ TODO/cli.md T-0801, issue 60: the curated docker surface stays
/// covered. Every flag in the issue's list resolves under `run` (any
/// status: honored or refused with its reason), and every verb in its
/// list has a verb row. `crates/podvm-gate/src/main.rs` check 27 holds the
/// same list from outside the binary; this holds it from inside.
#[test]
fn issue_60_curated_surface_stays_covered() {
    const FLAGS: &[&str] = &[
        "--attach",
        "--blkio-weight",
        "--cgroup-parent",
        "--cidfile",
        "--cpu-period",
        "--cpu-quota",
        "--cpu-shares",
        "--cpuset-cpus",
        "--detach-keys",
        "--device",
        "--device-cgroup-rule",
        "--disable-content-trust",
        "--dns",
        "--dns-option",
        "--dns-search",
        "--domainname",
        "--env-file",
        "--expose",
        "--gpus",
        "--group-add",
        "--health-cmd",
        "--init",
        "--ipc",
        "--isolation",
        "--label",
        "--link",
        "--log-driver",
        "--log-opt",
        "--mac-address",
        "--mount",
        "--oom-kill-disable",
        "--pid",
        "--pids-limit",
        "--read-only",
        "--runtime",
        "--security-opt",
        "--shm-size",
        "--stop-signal",
        "--stop-timeout",
        "--sysctl",
        "--tmpfs",
        "--ulimit",
        "--userns",
        "--uts",
        "--volume-driver",
        "--volumes-from",
    ];
    const VERBS: &[&str] = &[
        "manifest",
        "node",
        "plugin",
        "scan",
        "secret",
        "service",
        "stack",
        "trust",
        "checkpoint",
        "config",
    ];
    assert_eq!(FLAGS.len(), 46, "the issue lists 46 run flags");
    assert_eq!(VERBS.len(), 10, "the issue lists 10 verbs");
    for f in FLAGS {
        assert!(
            flag("run", f).is_some(),
            "curated flag {f} has no row under `run`"
        );
    }
    for v in VERBS {
        assert!(verb(v).is_some(), "curated verb {v} has no verb row");
    }
}

/// T-1527 (`325` clause 2). Every verb refuses a flag no row names, at
/// the flag-error code, before any arm runs. The live drive probes each
/// verb path with an unlisted flag; what is pinned here is the gate
/// that drive leans on, over every distinct verb in the table.
#[test]
fn every_verb_refuses_an_unlisted_flag() {
    let mut verbs: Vec<&str> = TABLE.iter().map(|r| r.verb).collect();
    verbs.sort_unstable();
    verbs.dedup();
    assert!(!verbs.is_empty());
    for v in verbs {
        assert_eq!(
            admit(v, "--no-such-flag", ""),
            Err(podvm_image::error::EXIT_FLAG_ERROR),
            "{v} admitted an unlisted flag"
        );
    }
}

/// T-1527 (`325` clause 1). Every refused flag row refuses with its own
/// note: each `None` row's first spelling is refused up front rather
/// than accepted and silently lost. The note text itself is covered by
/// `every_unimplemented_row_says_why`; the drive compares it against
/// the binary's stderr.
#[test]
fn every_refused_flag_row_refuses_its_first_spelling() {
    let mut count = 0;
    for r in TABLE.iter().filter(|r| r.status == Status::None) {
        let Some(spellings) = r.flag else {
            continue;
        };
        let first = spellings.split(',').next().unwrap().trim();
        assert_eq!(
            admit(r.verb, first, ""),
            Err(podvm_image::error::EXIT_FLAG_ERROR),
            "{} {first} was not refused up front",
            r.verb
        );
        count += 1;
    }
    assert!(count > 0, "no refused flag rows to pin");
}
