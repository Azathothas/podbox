use super::*;

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// TODO/cli.md T-1654: `--podvm-cpus` takes 1 to 64 in both forms, and
/// `--profile` gives its flags to the one parser, before the caller's.
#[test]
fn the_cpus_flag_and_a_profile_parse() {
    assert_eq!(
        parse("run", &v(&["--podvm-cpus", "2", "img"]))
            .unwrap()
            .cpus,
        Some(2)
    );
    assert_eq!(
        parse("run", &v(&["--podvm-cpus=64", "img"])).unwrap().cpus,
        Some(64)
    );
    for bad in ["0", "65", "two"] {
        assert_eq!(
            parse("run", &v(&["--podvm-cpus", bad, "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }
    let o = parse("run", &v(&["--profile", "default", "-w", "/x", "img"])).unwrap();
    assert_eq!(o.profile.as_deref(), Some("default"));
    assert_eq!(o.workdir.as_deref(), Some("/x"));
    assert_eq!(
        parse("run", &v(&["-s", "fs.user=1", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--profile=nosuch", "img"])).unwrap_err(),
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
