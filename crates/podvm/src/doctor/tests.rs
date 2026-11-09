use super::*;

#[test]
fn doctor_usage_is_plain_ascii() {
    // TODO/cli.md T-1336: usages print on a constrained host.
    let bad = DOCTOR_USAGE.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "doctor usage carries {bad} non-ASCII bytes");
}

/// The winquick shape, held: a fail carries exactly one fix, health
/// agrees with the problem list, and the incomplete assessment
/// outranks a failed one.
#[test]
fn health_agrees_with_the_problem_list() {
    let mut d = Doctor::default();
    assert_eq!(d.exit_code(), 0);
    d.ok("emu", "qemu=ok".to_string());
    d.note("kvm", "missing under tcg".to_string());
    assert_eq!(d.exit_code(), 0);
    d.fail("accel", "denied".to_string(), "install QEMU".to_string());
    assert_eq!(d.exit_code(), 1);
    let text = d.render("tcg");
    assert!(text.contains("fail accel: denied\n"));
    assert!(text.contains("fix: install QEMU\n"));
    assert!(text.contains("profile: tcg\n"));
    assert_eq!(d.fixes.len(), 1);
    d.unrunnable("prlimit(RLIMIT_FSIZE)");
    assert_eq!(d.exit_code(), 2);
    assert!(d.render("refused").contains("unmeasured: "));
}

/// TODO/cli.md T-1337: doctor decides its arguments before it
/// probes, so `--help` prints and a bad flag refuses hermetic.
#[test]
fn doctor_help_and_bad_flag_need_no_probe() {
    assert_eq!(doctor("doctor", &["--help".to_string()]), 0);
    assert_eq!(
        doctor("doctor", &["--bogus".to_string()]),
        podvm_image::error::EXIT_FLAG_ERROR
    );
    assert_eq!(
        doctor("doctor", &["some-thing".to_string()]),
        podvm_image::error::EXIT_FLAG_ERROR
    );
}

/// TODO/image.md T-1418. The `store_exec` row answers from the
/// directory, not the process store: `yes` with a 0 on a writable
/// directory, and a refusal naming the path where nothing can probe.
#[test]
#[cfg(unix)]
fn store_exec_row_reports_yes_on_a_writable_directory() {
    let d = std::env::temp_dir().join(format!("podvm-doctor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    assert_eq!(store_exec_row_at(&d), 0);
    let _ = std::fs::remove_dir_all(&d);
    // Not a directory podvm can write a probe into: an error naming
    // the path, never a "no".
    assert_eq!(store_exec_row_at(&d.join("absent")), 2);
}

/// T-1690 (issue 214): the soft limit is the ceiling. A soft 1 GiB with
/// an infinite hard limit is judged at 1 GiB, not passed as "no ceiling".
#[test]
fn the_ceiling_is_the_soft_limit() {
    const GIB: u64 = 1 << 30;
    let ok = judge_ceiling(GIB, u64::MAX, 4 * GIB, "/s").unwrap();
    assert!(ok.contains("covers the 1.00 GiB"), "{ok}");
    assert!(ok.contains("hard limit is infinity"), "{ok}");
    let short = judge_ceiling(GIB, u64::MAX, GIB / 2, "/s").unwrap_err();
    assert!(short.contains("below the 1.00 GiB"), "{short}");
    let equal = judge_ceiling(GIB, GIB, 2 * GIB, "/s").unwrap();
    assert!(!equal.contains("hard"), "{equal}");
    let both = judge_ceiling(GIB, 2 * GIB, GIB / 2, "/s").unwrap_err();
    assert!(both.contains("hard limit is 2.00 GiB"), "{both}");
}

/// TODO/enter.md T-1688: the row `memfd_exec` reads the probe's answer.
/// A hold is ok, a denial fails with the fix where chroot is denied and
/// notes it where chroot holds, and an unmeasured probe is a note.
#[test]
fn the_memfd_exec_row_follows_the_probe() {
    use podvm_enter::memfd::MemfdExec;
    use podvm_probe::sys;
    let sealed = MemfdExec::NoMemfd {
        sysctl: Some(2),
        errno: sys::EACCES,
    };
    let mut d = Doctor::default();
    examine_memfd_exec(false, &sealed, &mut d);
    assert_eq!(d.exit_code(), 1);
    let text = d.render("tcg");
    assert!(
        text.contains("fail memfd_exec: denied by vm.memfd_noexec=2"),
        "{text}"
    );
    assert!(text.contains("fix: set vm.memfd_noexec to 0"), "{text}");

    let mut d = Doctor::default();
    examine_memfd_exec(true, &sealed, &mut d);
    assert_eq!(d.exit_code(), 0);
    let text = d.render("tcg");
    assert!(
        text.contains("note memfd_exec: denied by vm.memfd_noexec=2"),
        "{text}"
    );
    assert!(
        text.contains("only PODVM_MODE=memfd needs it; fix: set"),
        "{text}"
    );
    assert!(d.fixes.is_empty());

    let mut d = Doctor::default();
    examine_memfd_exec(false, &MemfdExec::Exec { sysctl: Some(1) }, &mut d);
    assert_eq!(d.exit_code(), 0);
    assert!(d.render("tcg").contains("ok memfd_exec: a memfd executes"));

    let mut d = Doctor::default();
    let un = MemfdExec::Unmeasured {
        step: "clone(2) for the probe child",
        why: "EAGAIN (11)".to_string(),
    };
    examine_memfd_exec(false, &un, &mut d);
    assert_eq!(d.exit_code(), 0);
    assert!(d.render("tcg").contains("note memfd_exec: not measured"));
}
