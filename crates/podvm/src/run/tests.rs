use super::*;

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| (*s).to_string()).collect()
}

/// T-0413: the guest path rides beside the exec, a stale one is
/// scrubbed first, and nothing rides where nothing resolves.
#[test]
fn push_guest_exe_sets_scrubs_and_skips() {
    let d = std::env::temp_dir().join(format!("podvm-run-exe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/prog"), b"\x7fELF").unwrap();
    let root = d.to_string_lossy().to_string();
    let dirs = vec!["/bin".to_string()];
    let mut env = vec![
        "PODVM_GUEST_EXE=/stale".to_string(),
        "PATH=/bin".to_string(),
    ];
    push_guest_exe(&mut env, &root, "prog", &dirs);
    assert_eq!(
        env.iter()
            .filter(|e| e.starts_with("PODVM_GUEST_EXE="))
            .count(),
        1,
        "{env:?}"
    );
    assert!(
        env.iter().any(|e| e == "PODVM_GUEST_EXE=/bin/prog"),
        "{env:?}"
    );
    let mut env = vec!["A=1".to_string()];
    push_guest_exe(&mut env, &root, "absent", &dirs);
    assert!(
        !env.iter().any(|e| e.starts_with("PODVM_GUEST_EXE=")),
        "{env:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// ⭐ TODO/enter.md T-1407: `--unsafe-host-paths` parses, and is off
/// unless asked for.
#[test]
fn unsafe_host_paths_parses_and_defaults_off() {
    assert!(
        parse("run", &v(&["--unsafe-host-paths", "img"]))
            .unwrap()
            .unsafe_host_paths
    );
    assert!(!parse("run", &v(&["img"])).unwrap().unsafe_host_paths);
}

/// ⭐ TODO/cli.md T-1415: `--strict=all` keeps the T-0804 refusal of
/// every difference, and anything else after `=` is a flag error.
#[test]
fn strict_all_parses_and_anything_else_is_a_flag_error() {
    let o = parse("run", &v(&["--strict=all", "img"])).unwrap();
    assert!(o.ask.strict);
    assert!(o.ask.strict_all);
    let o = parse("run", &v(&["--strict", "img"])).unwrap();
    assert!(o.ask.strict);
    assert!(!o.ask.strict_all);
    assert_eq!(
        parse("run", &v(&["--strict=sometimes", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// ⭐ TODO/cli.md T-1416: `-q`/`--quiet` forces the banner off and
/// `--verbose` prints it, including bundled with `-i`. `-v` is NOT
/// among them: it stays docker's `--volume`, refused by the table.
#[test]
fn quiet_and_verbose_parse() {
    let o = parse("run", &v(&["-q", "img"])).unwrap();
    assert!(o.ask.quiet);
    assert!(!o.ask.verbose);
    let o = parse("run", &v(&["--verbose", "img"])).unwrap();
    assert!(o.ask.verbose);
    let o = parse("run", &v(&["-qi", "img"])).unwrap();
    assert!(o.ask.quiet);
    assert_eq!(
        parse("run", &v(&["-v", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// ⭐ TODO/supervise.md T-1421. The foreground record shape this verb
/// relies on: `create` writes it, a table update lands it `Exited`
/// with the code and the times, `ps -a` lists it, and `logs` reads
/// the file at its log path. Pinned here so a record drift fails
/// here instead of on the drive.
#[test]
#[cfg(unix)]
fn a_foreground_record_lands_exited_with_logs() {
    let dir = std::env::temp_dir().join(format!("podvm-fg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = podvm_image::Store::open(&dir).unwrap();
    let c = podvm_supervise::create(
        &store,
        Some("fg1"),
        "img",
        "sha256:0",
        "/tmp",
        vec!["echo".into(), "hi".into()],
        Vec::new(),
        "/".into(),
        "userland",
        Vec::new(),
        0,
        Vec::new(),
    )
    .unwrap();
    std::fs::write(podvm_supervise::table::log_path(&store, &c.id), b"hi\n").unwrap();
    let id = c.id.clone();
    podvm_supervise::table::update(&store, |t| {
        match t.containers.iter_mut().find(|x| x.id == id) {
            Some(x) => {
                x.state = podvm_supervise::table::State::Exited;
                x.exit_code = Some(0);
                x.started_at = Some("s".into());
                x.finished_at = Some("f".into());
                Ok(())
            }
            None => Err(podvm_supervise::Error("gone".into())),
        }
    })
    .unwrap();
    let list = podvm_supervise::list(&store, true).unwrap();
    let got = list
        .iter()
        .find(|x| x.name == "fg1")
        .expect("fg1 is listed");
    assert_eq!(got.status(), "Exited (0)");
    assert_eq!(
        podvm_supervise::logs(&store, "fg1").unwrap(),
        b"hi\n",
        "logs reads the recorded output"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_usage_string_is_plain_ascii() {
    // TODO/cli.md T-1336: every `--help` path prints bytes 0x00-0x7F
    // only. `run`'s usage is the shared one (`create` and `exec`
    // print it too), so holding this one holds all three.
    let text = usage("run") + &usage("create") + RUN_OPTIONS;
    let bad = text.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "run usage carries {bad} non-ASCII bytes");
}

/// ⛔ The rule a caller cannot work around if podvm gets it wrong.
#[test]
fn parsing_stops_at_the_image_name() {
    let o = parse("run", &v(&["--rm", "alpine", "ls", "-l", "--rm"])).unwrap();
    assert!(o.rm);
    assert_eq!(o.image.as_deref(), Some("alpine"));
    // ⛔ `-l` and the SECOND `--rm` belong to `ls`, not to podvm.
    assert_eq!(o.command, v(&["ls", "-l", "--rm"]));
}

#[test]
fn a_flag_needing_a_value_does_not_swallow_the_image() {
    assert_eq!(
        parse("run", &v(&["--platform"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(parse("run", &v(&["-e"])).unwrap_err(), EXIT_FLAG_ERROR);
}

#[test]
fn both_spellings_of_every_valued_flag_reach_the_same_place() {
    let a = parse(
        "run",
        &v(&["-e", "A=1", "-w", "/w", "--entrypoint", "/e", "img"]),
    )
    .unwrap();
    let b = parse(
        "run",
        &v(&["--env=A=1", "--workdir=/w", "--entrypoint=/e", "img"]),
    )
    .unwrap();
    assert_eq!(a.env, b.env);
    assert_eq!(a.workdir, b.workdir);
    assert_eq!(a.entrypoint, b.entrypoint);
    assert_eq!(a.image, b.image);
}

#[test]
fn an_unknown_pull_mode_is_invalid_input_and_not_a_silent_default() {
    assert_eq!(
        parse("run", &v(&["--pull", "sometimes", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--pull", "always", "img"])).unwrap().pull,
        "always"
    );
    assert_eq!(parse("run", &v(&["img"])).unwrap().pull, "missing");
}

/// ⭐ TODO/cli.md T-0801. Every flag the table ADMITS for this verb reaches
/// an arm of this parser. The other direction is held by construction: an
/// argument starting with `-` goes through `parity::admit` first, so an arm
/// for a flag with no row is unreachable rather than a second surface.
#[test]
fn every_flag_the_table_admits_is_handled_by_this_parser() {
    for r in crate::parity::TABLE
        .iter()
        .filter(|r| r.verb == "run" && r.flag.is_some())
    {
        if r.status == crate::parity::Status::None {
            // ⚠ A `None` row is refused BY `admit`, so reaching an arm is
            // exactly what it must not do. Asserted the other way round.
            let got = parse(
                "run",
                &v(&[
                    r.flag.unwrap().split(',').next().unwrap().trim(),
                    "img",
                    "true",
                ]),
            );
            assert_eq!(
                got.unwrap_err(),
                EXIT_FLAG_ERROR,
                "{:?} was not refused",
                r.flag
            );
            continue;
        }
        for spelling in r.flag.unwrap().split(',') {
            let f = spelling.trim();
            if f == "-h" || f == "--help" {
                assert_eq!(
                    parse("run", &v(&[f])).unwrap_err(),
                    0,
                    "{f} did not print usage"
                );
                continue;
            }
            // ⚠ THE ASSERTION IS `parity::no_arm`'s PANIC, not a code
            // comparison. It used to compare against EXIT_RUNTIME_ERROR,
            // which was the fallback arm's own code and distinguishable
            // from a legitimate refusal's 2; T-0802 measured docker and
            // made a flag error 125 as well, so that comparison started
            // asserting `125 != 125` and could no longer fail. Both shapes
            // are tried because a valued flag needs a value, a boolean one
            // does not, and one with a closed set of values (`--pull`)
            // refuses any value this test could invent: all three are
            // legitimate, and only reaching the fallback arm is not.
            for shape in [v(&[f, "V", "img", "true"]), v(&[f, "img", "true"])] {
                let _ = parse("run", &shape);
            }
        }
    }
}

#[test]
/// ⚠ docker's own code for a required argument that is not there is 1 and
/// not 125: measured, `docker run` with no image exits 1. T-0802.
fn no_image_is_a_cli_error() {
    assert_eq!(parse("run", &v(&["--rm"])).unwrap_err(), EXIT_CLI_ERROR);
}

/// ⭐ TODO/machine.md T-1302. The tier flag takes two values, and the
/// emulator arguments collect whole: one token per occurrence, and a
/// value with a space stays one token rather than splitting.
#[test]
fn the_tier_flag_takes_two_values_and_qemu_args_collect_whole() {
    assert_eq!(
        parse("run", &v(&["--podvm-tier=machine", "img"]))
            .unwrap()
            .tier,
        Some(crate::tier::Want::Machine)
    );
    assert_eq!(
        parse("run", &v(&["--podvm-tier", "chroot", "img"]))
            .unwrap()
            .tier,
        Some(crate::tier::Want::Chroot)
    );
    assert_eq!(
        parse("run", &v(&["--podvm-tier=sandbox", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--podvm-tier", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    let o = parse(
        "run",
        &v(&["--podvm-qemu-arg", "a", "--podvm-qemu-arg=b c", "img"]),
    )
    .unwrap();
    assert_eq!(o.qemu_args, v(&["a", "b c"]));
}

/// ⭐ TODO/machine.md T-1305: the memory spelling parses to bytes in both
/// forms, and anything that is not a size is a flag error.
#[test]
fn the_mem_flag_parses_to_bytes_in_both_forms() {
    assert_eq!(
        parse("run", &v(&["--podvm-mem", "512M", "img"]))
            .unwrap()
            .mem,
        Some(512 << 20)
    );
    assert_eq!(
        parse("run", &v(&["--podvm-mem=1G", "img"])).unwrap().mem,
        Some(1 << 30)
    );
    assert_eq!(
        parse("run", &v(&["--podvm-mem=bogus", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--podvm-mem", "0", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// ⭐ TODO/cli.md T-0801, issue 60: `--env-file` loads docker's
/// `KEY=VALUE` lines at the flag's position, and the stub trio
/// (`--label`, `--attach`, `--expose`) parses without reaching an arm
/// that acts on them.
#[test]
fn env_file_loads_and_stubs_parse() {
    let dir = std::env::temp_dir().join(format!("pb-envfile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.env");
    std::fs::write(
        &file,
        "# a comment\n\nA=1\nB = two words\nC=\"quoted\"\nD='single'\n",
    )
    .unwrap();
    let path = file.to_str().unwrap().to_string();
    // Both spellings land in the same place, at the flag's position:
    // a later -e wins over the file.
    let o = parse("run", &v(&["--env-file", &path, "-e", "A=9", "img"])).unwrap();
    assert_eq!(
        o.env,
        v(&["A=1", "B=two words", "C=quoted", "D=single", "A=9"])
    );
    let o = parse("run", &v(&[&format!("--env-file={path}"), "img"])).unwrap();
    assert_eq!(o.env, v(&["A=1", "B=two words", "C=quoted", "D=single"]));
    // A line without `=` is refused naming the file, never skipped.
    let bad = dir.join("bad.env");
    std::fs::write(&bad, "A=1\nNOEQUALS\n").unwrap();
    assert_eq!(
        parse("run", &v(&["--env-file", bad.to_str().unwrap(), "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    // A missing file is refused naming the path, never empty env.
    assert_eq!(
        parse(
            "run",
            &v(&[
                "--env-file",
                dir.join("absent.env").to_str().unwrap(),
                "img"
            ])
        )
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
    std::fs::remove_dir_all(&dir).ok();
    // The stub trio parses and changes nothing about the run.
    let o = parse(
        "run",
        &v(&["--label", "k=v", "--attach", "--expose", "img"]),
    )
    .unwrap();
    assert_eq!(o.image.as_deref(), Some("img"));
    assert!(o.env.is_empty());
}

/// ⭐ TODO/supervise.md T-0605, issue 56: `--log-driver` takes
/// `json-file` and nothing else. The accepted value parses in both
/// spellings; any other value, or none, is a flag error naming the
/// value rather than a silent substitution.
#[test]
fn log_driver_takes_json_file_and_nothing_else() {
    assert!(parse("run", &v(&["--log-driver", "json-file", "img"])).is_ok());
    assert!(parse("run", &v(&["--log-driver=json-file", "img"])).is_ok());
    assert_eq!(
        parse("run", &v(&["--log-driver", "syslog", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--log-driver=syslog", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--log-driver", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// ⭐ TODO/enter.md T-0501: `--device` repeats in both spellings, keeps
/// the raw spellings for `prepare`, and a malformed shape is a flag
/// error naming it, before anything is fetched.
#[test]
fn device_collects_in_both_spellings_and_refuses_shapes() {
    let o = parse(
        "run",
        &v(&[
            "--device",
            "/dev/zero:/dev/hostzero:rm",
            "--device=/dev/null",
            "img",
        ]),
    )
    .unwrap();
    assert_eq!(o.devices, v(&["/dev/zero:/dev/hostzero:rm", "/dev/null"]));
    for bad in [
        "--device=/dev/zero:/a:/b:c",
        "--device=relative:/g:r",
        "--device=/dev/zero:/g:rx",
        "--device=/dev/zero:/g:m",
        "--device=/dev/zero:/:r",
    ] {
        assert_eq!(
            parse("run", &v(&[bad, "img"])).unwrap_err(),
            EXIT_FLAG_ERROR,
            "{bad} was not refused"
        );
    }
    assert_eq!(
        parse("run", &v(&["--device", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// TODO/cli.md T-1649. The word that a refusal of an Entrypoint script
/// names: the first word of the caller, only where the Entrypoint of the
/// image runs and the caller gave a command.
#[test]
fn the_retry_word_is_the_first_word_of_a_caller_command() {
    let ep = v(&["docker-entrypoint.sh"]);
    let o = parse("run", &v(&["node:22-alpine", "node", "-e", "x"])).unwrap();
    assert_eq!(o.retry_word(Some(&ep)), Some("node"));
    assert_eq!(o.retry_word(None), None);
    assert_eq!(o.retry_word(Some(&[])), None);
    let o = parse("run", &v(&["--entrypoint", "node", "img", "-e", "x"])).unwrap();
    assert_eq!(o.retry_word(Some(&ep)), None);
    let o = parse("run", &v(&["img"])).unwrap();
    assert_eq!(o.retry_word(Some(&ep)), None);
}
