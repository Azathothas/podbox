//! Test each descriptor and temporary-file execution method. These tests
//! require the `test-hooks` feature.
//!
//! ```sh
//! cargo test --features test-hooks
//! ```
//!
//! Hooks:
//! - `MEMFD_NG_TEST_NO_EXECVEAT` disables `execveat(2)`.
//! - `MEMFD_NG_TEST_NO_PROC` disables the `/proc/self/fd` method.
//! - `MEMFD_NG_TEST_NO_OTMPFILE` disables `O_TMPFILE`.
//! - `MEMFD_NG_TEST_NO_NAMED_STAGE` disables named-file creation.
//!
//! Each test sets `TMPDIR` to a new executable directory. The test checks this
//! directory for files that remain.

#![cfg(all(feature = "test-hooks", target_os = "linux"))]

mod common;

#[cfg(target_arch = "x86_64")]
use common::TINY_ELF_EXIT42;
use common::{stub_code, EnvGuard};
use memfd_ng::{MemFdExecutable, Stdio};

#[test]
fn rung2_proc_path_executes_when_execveat_is_refused() {
    let _g = EnvGuard::set(&[("MEMFD_NG_TEST_NO_EXECVEAT", "1")]);
    let out = MemFdExecutable::new("rung2-stub", stub_code())
        .args(["print", "via-procfd-rung"])
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"via-procfd-rung\n");
    assert!(out.status.success());
}

#[test]
fn named_rung_executes_and_parent_cleans_up_without_procfs() {
    // Disable execveat and procfs. The child then uses a named file. The parent
    // removes the file after it waits for the child.
    let good = common::exec_tmpdir("named-rung");
    let _g = EnvGuard::set(&[
        ("MEMFD_NG_TEST_NO_EXECVEAT", "1"),
        ("MEMFD_NG_TEST_NO_PROC", "1"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("named-rung-stub", stub_code())
        .args(["print", "via-named-rung"])
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"via-named-rung\n");
    assert!(out.status.success());
    common::assert_no_fallback_leftovers(&good);
}

#[test]
fn named_rung_failure_cleans_up_via_pipe_protocol() {
    // Disable procfs and execute an invalid image. The child reports the error
    // through the pipe. The parent removes the named file.
    let good = common::exec_tmpdir("named-fail");
    let _g = EnvGuard::set(&[
        ("MEMFD_NG_TEST_NO_PROC", "1"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let err = MemFdExecutable::new("doomed", b"definitely not an elf")
        .status()
        .unwrap_err();
    // The result must be ENOEXEC.
    assert_eq!(err.raw_os_error(), Some(8));
    common::assert_no_fallback_leftovers(&good);
}

#[test]
#[cfg(target_arch = "x86_64")]
fn tiny_elf_still_runs_through_every_rung() {
    // Execute the deterministic image with each method.
    let cases: &[(&str, Option<&str>)] = &[
        ("execveat", None),
        ("procfd", Some("MEMFD_NG_TEST_NO_EXECVEAT")),
        ("named", Some("MEMFD_NG_TEST_NO_PROC")),
    ];
    for (label, hook) in cases {
        let good = common::exec_tmpdir("tiny");
        let mut g = EnvGuard::set(&[("TMPDIR", good.to_str().unwrap())]);
        if let Some(h) = hook {
            g.put(h, "1");
        }
        let st = MemFdExecutable::new("tiny", TINY_ELF_EXIT42)
            .status()
            .unwrap();
        assert_eq!(st.code(), Some(42), "the ELF failed with {label}");
    }
}

#[test]
fn otmpfile_staging_serves_the_whole_ladder() {
    // Require O_TMPFILE by disabling named-file creation. Each applicable
    // execution configuration must still work.
    let good = common::exec_tmpdir("otmp");
    let mut g = EnvGuard::set(&[
        ("MEMFD_NG_TEST_NO_NAMED_STAGE", "1"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);

    // Case 1 uses memfd execution. The test hook must not affect this case.
    let out = MemFdExecutable::new("otmp-memfd", stub_code())
        .args(["print", "otmp-normal"])
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"otmp-normal\n");

    // Case 2 uses O_TMPFILE and reopens the file through /proc/self/fd.
    g.put("NO_MEMFDEXEC", "1");
    let out = MemFdExecutable::new("otmp-proc", stub_code())
        .args(["print", "otmp-via-proc-reopen"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"otmp-via-proc-reopen\n");
    assert_eq!(out.stderr, b"", "the crate must stay silent");
    g.unset("NO_MEMFDEXEC");

    // Case 3 uses O_TMPFILE and linkat without procfs. This operation requires
    // CAP_DAC_READ_SEARCH. EPERM is valid for an unprivileged user.
    g.put("MEMFD_NG_TEST_NO_EXECVEAT", "1");
    g.put("MEMFD_NG_TEST_NO_PROC", "1");
    let out = MemFdExecutable::new("otmp-norproc", stub_code())
        .args(["print", "otmp-via-linkat-dance"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output();
    match out {
        Ok(o) => assert_eq!(o.stdout, b"otmp-via-linkat-dance\n"),
        Err(e) => assert_eq!(e.raw_os_error(), Some(1) /* EPERM: hook-enforced */),
    }
    common::assert_no_fallback_leftovers(&good);
}

#[test]
fn legacy_named_staging_still_works_when_otmpfile_is_off() {
    // Disable O_TMPFILE. The named-file method must write, set the mode,
    // execute, and remove the file.
    let good = common::exec_tmpdir("legacy-named");
    let _g = EnvGuard::set(&[
        ("MEMFD_NG_TEST_NO_OTMPFILE", "1"),
        ("MEMFD_NG_TEST_NO_EXECVEAT", "1"),
        ("MEMFD_NG_TEST_NO_PROC", "1"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("legacy-named", stub_code())
        .args(["print", "legacy-named-staging"])
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"legacy-named-staging\n");
    common::assert_no_fallback_leftovers(&good);
}

#[test]
fn home_cache_rung_stages_inside_dot_cache() {
    // Skip the fixed system directories. Set the first two environment paths
    // to directories that do not exist. The fallback must use $HOME/.cache.
    let home = common::exec_tmpdir("home-cache");
    let _g = EnvGuard::set(&[
        ("NO_MEMFDEXEC", "1"),
        ("MEMFD_NG_TEST_NO_SYSDIRS", "1"),
        ("XDG_RUNTIME_DIR", "/nonexistent-mfd-h1"),
        ("TMPDIR", "/nonexistent-mfd-h2"),
        ("HOME", home.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("home-cache", stub_code())
        .args(["print", "staged-in-cache"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"staged-in-cache\n");
    assert_eq!(out.stderr, b"");
    let cache = home.join(".cache");
    assert!(cache.is_dir(), "the HOME fallback must create $HOME/.cache");
    common::assert_no_fallback_leftovers(&cache);
    let stray: Vec<_> = std::fs::read_dir(&home)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(common::FALLBACK_PREFIX)
        })
        .collect();
    assert!(stray.is_empty(), "must not stage in HOME root: {stray:?}");
}

#[test]
fn otmpfile_off_procmode_still_cleans_up_failures() {
    // A/B control with procfs: O_TMPFILE skipped, the named-create-then-unlink
    // flow must leave nothing behind on success and failure.
    let good = common::exec_tmpdir("legacy-proc");
    let _g = EnvGuard::set(&[
        ("MEMFD_NG_TEST_NO_OTMPFILE", "1"),
        ("NO_MEMFDEXEC", "1"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("legacy-proc", stub_code())
        .args(["print", "legacy-with-procfs"])
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"legacy-with-procfs\n");

    let err = MemFdExecutable::new("legacy-doomed", b"still not an elf")
        .status()
        .unwrap_err();
    assert_eq!(err.raw_os_error(), Some(8) /* ENOEXEC */);
    common::assert_no_fallback_leftovers(&good);
}
