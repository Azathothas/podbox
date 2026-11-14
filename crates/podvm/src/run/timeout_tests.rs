use super::*;

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// TODO/cli.md T-1655. `--timeout` takes whole seconds over 0, in both
/// spellings, and carries them to the run; 0, a fraction and a word are
/// flag errors.
#[test]
fn the_timeout_takes_whole_seconds_over_zero() {
    assert_eq!(
        parse("run", &v(&["--timeout", "2", "img"]))
            .unwrap()
            .timeout,
        Some(2)
    );
    assert_eq!(
        parse("run", &v(&["--timeout=30", "img"])).unwrap().timeout,
        Some(30)
    );
    assert_eq!(parse("run", &v(&["img"])).unwrap().timeout, None);
    for bad in ["0", "1.5", "two", "-1", ""] {
        assert_eq!(
            parse("run", &v(&["--timeout", bad, "img"])).unwrap_err(),
            EXIT_FLAG_ERROR,
            "{bad:?}"
        );
    }
}

/// TODO/cli.md T-1655. `--ulimit` and `--security-opt no-new-privileges`
/// reach the record of the run; a bad limit and each other security option
/// are flag errors.
#[test]
fn the_limits_of_the_hardening_line_parse() {
    let o = parse(
        "run",
        &v(&[
            "--ulimit",
            "nofile=64",
            "--ulimit=fsize=1024:2048",
            "--security-opt",
            "no-new-privileges",
            "img",
        ]),
    )
    .unwrap();
    assert_eq!(o.hardening.ulimits, v(&["nofile=64", "fsize=1024:2048"]));
    assert!(o.hardening.no_new_privs);
    assert_eq!(o.hardening.limits().len(), 2);
    let o = parse("run", &v(&["--security-opt=no-new-privileges:true", "img"])).unwrap();
    assert!(o.hardening.no_new_privs);
    assert!(
        parse("run", &v(&["--cap-drop", "ALL", "img"]))
            .unwrap()
            .hardening
            .no_new_privs
    );
    assert!(
        parse("run", &v(&["--cap-drop=all", "img"]))
            .unwrap()
            .hardening
            .no_new_privs
    );
    assert_eq!(
        parse("run", &v(&["--cap-drop", "CHOWN", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse("run", &v(&["--cap-add", "NET_ADMIN", "img"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert!(!parse("run", &v(&["img"])).unwrap().hardening.no_new_privs);
    for bad in [
        &["--ulimit", "files=1", "img"][..],
        &["--ulimit", "nofile=2:1", "img"][..],
        &["--security-opt", "seccomp=unconfined", "img"][..],
        &["--security-opt", "label=disable", "img"][..],
    ] {
        assert_eq!(
            parse("run", &v(bad)).unwrap_err(),
            EXIT_FLAG_ERROR,
            "{bad:?}"
        );
    }
}

/// TODO/cli.md T-1827: the last value of a resource wins, `-1` takes the
/// hard limit that podvm has, and a hard value over it is refused with
/// both numbers.
#[test]
fn ulimit_plan_keeps_the_last_value_and_resolves_minus_one() {
    use super::parse::values::ulimit_plan;
    let own = |_: u64| Some(1_048_576);
    let got = ulimit_plan(&v(&["nofile=64", "nproc=10", "nofile=128"]), &own).unwrap();
    assert_eq!(got, v(&["nproc=10", "nofile=128"]));
    let got = ulimit_plan(&v(&["nofile=-1"]), &own).unwrap();
    assert_eq!(got, v(&["nofile=1048576"]));
    let e = ulimit_plan(&v(&["nofile=64:1048577"]), &own).unwrap_err();
    assert!(e.contains("1048577") && e.contains("1048576"), "{e}");
    let unlimited = |_: u64| None;
    let got = ulimit_plan(&v(&["core=-1"]), &unlimited).unwrap();
    assert_eq!(got, v(&["core=-1"]));
}
