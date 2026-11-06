use super::*;

fn report_with_degradation() -> podvm_complete::Report {
    let mut r = podvm_complete::Report::default();
    r.fixups.push(podvm_complete::Fixup {
        entry: "T-0401",
        id: "dev-shim",
        path: "dev/null".into(),
        action: podvm_complete::Action::Created,
        detail: "a regular file".into(),
        degraded: true,
    });
    r
}

/// ⛔ Without `--strict` a degraded run is a run. The banner is the report,
/// and refusing by default would make podvm useless on the machines it
/// exists for.
#[test]
fn without_strict_nothing_is_refused() {
    let ask = Ask::default();
    let mut out = Vec::new();
    assert!(strict_refusal(
        "run",
        &ask,
        "chroot",
        &report_with_degradation(),
        &mut out,
        &StrictCtx::default()
    )
    .is_ok());
    assert!(out.is_empty());
}

/// ⭐ With `--strict=all`, all three inputs are named in one refusal
/// rather than the first one found, exactly the T-0804 behavior, and
/// every line names its class.
#[test]
fn strict_all_names_every_reason_at_once() {
    let ask = Ask {
        strict: true,
        strict_all: true,
        seen_flags: vec!["-i".into(), "--rm".into()],
        ..Ask::default()
    };
    let mut out = Vec::new();
    let e = strict_refusal(
        "run",
        &ask,
        "chroot",
        &report_with_degradation(),
        &mut out,
        &StrictCtx {
            chroot_usable: true,
            ..StrictCtx::default()
        },
    )
    .unwrap_err();
    assert_eq!(e, EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    // the Stub flag
    assert!(s.contains("-i, --interactive"), "{s}");
    // the rung
    assert!(s.contains("`chroot`"), "{s}");
    // the fixup
    assert!(s.contains("dev/null"), "{s}");
    // ⚠ and NOT the Native one.
    assert!(!s.contains("--rm is"), "{s}");
    assert!(s.contains("3 way(s)"), "{s}");
    assert!(s.contains("[safety]"), "{s}");
    assert!(s.contains("[substitution]"), "{s}");
}

/// ⭐ TODO/cli.md T-1415. Kind decides: on a chroot-denied host the
/// userland rung and the dev-shim substitutions warn and the run
/// proceeds, while a path-virtualization opt-out and a failed fixup
/// still refuse, each line naming its class.
#[test]
fn strict_classifies_safety_against_substitution() {
    let userland = podvm_probe::select::Rung::Userland.word();
    let denied = StrictCtx {
        chroot_usable: false,
        ..StrictCtx::default()
    };
    // The best available rung plus a dev-shim: warnings, exit 0.
    let ask = Ask {
        strict: true,
        ..Ask::default()
    };
    let mut out = Vec::new();
    assert!(strict_refusal(
        "run",
        &ask,
        userland,
        &report_with_degradation(),
        &mut out,
        &denied
    )
    .is_ok());
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("--strict warning [substitution]"), "{s}");
    // The same run with the pass-through opted in: refused, safety.
    let unsafe_ctx = StrictCtx {
        chroot_usable: false,
        unsafe_host_paths: true,
        ..StrictCtx::default()
    };
    let mut out = Vec::new();
    let e = strict_refusal(
        "run",
        &ask,
        userland,
        &report_with_degradation(),
        &mut out,
        &unsafe_ctx,
    )
    .unwrap_err();
    assert_eq!(e, EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("[safety]"), "{s}");
    assert!(s.contains("--unsafe-host-paths"), "{s}");
    // A fixup podvm tried and could not: refused even where the rung
    // is the best available.
    let mut failed = podvm_complete::Report::default();
    failed.fixups.push(podvm_complete::Fixup {
        entry: "T-0401",
        id: "dev-shim",
        path: "dev/null".into(),
        action: podvm_complete::Action::Failed,
        detail: "a directory".into(),
        degraded: true,
    });
    let mut out = Vec::new();
    let e = strict_refusal("run", &ask, userland, &failed, &mut out, &denied).unwrap_err();
    assert_eq!(e, EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("[safety]"), "{s}");
    // And `--strict=all` refuses the warned run too.
    let ask = Ask {
        strict: true,
        strict_all: true,
        ..Ask::default()
    };
    let mut out = Vec::new();
    let e = strict_refusal(
        "run",
        &ask,
        userland,
        &report_with_degradation(),
        &mut out,
        &denied,
    )
    .unwrap_err();
    assert_eq!(e, EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("--strict=all"), "{s}");
}

/// ⭐ T-0412. A step is a reason on its own, and it is counted BEFORE any of
/// them runs: podvm executing a command inside somebody else's image that
/// the caller did not write is a difference from docker, and refusing after
/// running one would be podvm acting and then declining to have acted.
#[test]
fn strict_refuses_a_run_whose_only_difference_is_a_step() {
    let mut report = podvm_complete::Report::default();
    report.steps.push(podvm_complete::Step {
        entry: "T-0412",
        id: "ca-hash-dir",
        argv: vec![
            "/usr/bin/openssl".into(),
            "rehash".into(),
            "/etc/ssl/certs".into(),
        ],
        why: "libzypp reads this directory and no CAfile at all".into(),
    });
    let ask = Ask {
        strict: true,
        ..Ask::default()
    };
    let floor = podvm_probe::select::Selection::STRICT_FLOOR.word();
    let mut out = Vec::new();
    let e =
        strict_refusal("run", &ask, floor, &report, &mut out, &StrictCtx::default()).unwrap_err();
    assert_eq!(e, EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("/usr/bin/openssl rehash /etc/ssl/certs"), "{s}");
    assert!(s.contains("1 safety-relevant way(s)"), "{s}");
}

/// ⚠ A run with nothing degraded passes `--strict`, or the flag would be a
/// refusal rather than a gate.
#[test]
fn strict_passes_a_run_with_no_degradation() {
    let ask = Ask {
        strict: true,
        seen_flags: vec!["--rm".into()],
        ..Ask::default()
    };
    let floor = podvm_probe::select::Selection::STRICT_FLOOR.word();
    let mut out = Vec::new();
    assert!(strict_refusal(
        "run",
        &ask,
        floor,
        &podvm_complete::Report::default(),
        &mut out,
        &StrictCtx::default(),
    )
    .is_ok());
}

/// ⭐ TODO/cli.md T-1416. Quiet by default, the banner under
/// `--verbose`, `-q` forcing it off, the store config still forcing
/// it off: the operator ruling reversing T-0804's command-line rule
/// in writing. Refusals never pass through here, so nothing about
/// them is asserted here.
#[test]
fn the_banner_is_quiet_by_default_and_loud_under_verbose() {
    let d = std::env::temp_dir().join(format!("podvm-cfg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::set_var("PODVM_STORE", &d) };
    let s = podvm_image::open_store().unwrap();
    // Default: quiet. The banner config key still reads as before.
    assert!(!banner_quiet(&s));
    assert!(!banner_loud(&s, &Ask::default()));
    // `--verbose` prints it.
    let verbose = Ask {
        verbose: true,
        ..Ask::default()
    };
    assert!(banner_loud(&s, &verbose));
    // `-q` forces it off, even beside `--verbose`.
    let off = Ask {
        verbose: true,
        quiet: true,
        ..Ask::default()
    };
    assert!(!banner_loud(&s, &off));
    // The store config forces it off under `--verbose` too.
    std::fs::write(s.root().join(CONFIG_FILE), "# a comment\nbanner = quiet\n").unwrap();
    assert!(banner_quiet(&s));
    assert!(!banner_loud(&s, &verbose));
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODVM_STORE") };
    let _ = std::fs::remove_dir_all(&d);
}
