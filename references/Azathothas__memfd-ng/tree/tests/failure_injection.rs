//! Test invalid input and forced failures. The cases cover directory errors,
//! empty images, NUL bytes, large argument lists, and unsafe program names.
//!
//! Each test that changes the environment holds an `EnvGuard`. The guard
//! serializes the test and restores the environment when it is dropped.

#![cfg(target_os = "linux")]

mod common;

use common::{stub_code, EnvGuard};
use memfd_ng::{MemFdExecutable, Stdio};

/// Skip a directory that does not exist and use the next candidate.
///
/// `XDG_RUNTIME_DIR` does not exist. `TMPDIR` is an executable directory.
#[test]
fn tmpfs_ladder_walks_past_dead_directories() {
    let good = common::exec_tmpdir("walk");
    let _g = EnvGuard::set(&[
        ("NO_MEMFDEXEC", "1"),
        ("XDG_RUNTIME_DIR", "/nonexistent-mfd-a"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("dead-dirs", stub_code())
        .args(["print", "walked-past-dead"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"walked-past-dead\n");
    assert_eq!(out.stderr, b"", "the fallback must not write to stderr");
    common::assert_no_fallback_leftovers(&good);
}

/// Skip a `noexec` mount and use the next directory.
///
/// `XDG_RUNTIME_DIR` points to `/proc`. `TMPDIR` points to an executable
/// directory.
#[test]
fn tmpfs_ladder_skips_a_real_noexec_mount() {
    let good = common::exec_tmpdir("noexec");
    let _g = EnvGuard::set(&[
        ("NO_MEMFDEXEC", "1"),
        ("XDG_RUNTIME_DIR", "/proc"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("noexec-ladder", stub_code())
        .args(["print", "survived-noexec"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"survived-noexec\n");
    assert_eq!(out.stderr, b"", "the fallback must not write to stderr");
    assert!(
        std::fs::read_dir("/proc")
            .unwrap()
            .filter_map(|e| e.ok())
            .all(|e| !e
                .file_name()
                .to_string_lossy()
                .starts_with(common::FALLBACK_PREFIX)),
        "the fallback used a noexec mount"
    );
    common::assert_no_fallback_leftovers(&good);
}

/// A program name with `/` and `..` must not change the fallback path.
///
/// The library creates the path from the user ID, process ID, and random
/// bytes. It does not use the program name in the path.
#[test]
fn hostile_program_name_cannot_escape_the_staging_dir() {
    let good = common::exec_tmpdir("hostile");
    let _g = EnvGuard::set(&[
        ("NO_MEMFDEXEC", "1"),
        ("XDG_RUNTIME_DIR", "/nonexistent-mfd-x"),
        ("TMPDIR", good.to_str().unwrap()),
    ]);
    // Use path traversal sequences in the program name.
    let out = MemFdExecutable::new("../../../../etc/mfd-escape", stub_code())
        .args(["print", "name-did-not-escape"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"name-did-not-escape\n");
    assert_eq!(out.stderr, b"");
    // The parent directory must not contain a path from the program name.
    let parent = good.parent().unwrap();
    assert!(
        !parent.join("etc").exists(),
        "the program name escaped the staging directory"
    );
    common::assert_no_fallback_leftovers(&good);
}

/// Use a later directory when the first two candidates do not exist.
///
/// The available directory can be `/dev/shm`, `/var/tmp`, or `$HOME/.cache`.
/// A file under the home directory must use `$HOME/.cache`.
#[test]
fn ladder_reaches_a_deep_rung_when_the_first_two_are_dead() {
    let home = common::exec_tmpdir("deep");
    let _g = EnvGuard::set(&[
        ("NO_MEMFDEXEC", "1"),
        ("XDG_RUNTIME_DIR", "/nonexistent-mfd-d1"),
        ("TMPDIR", "/nonexistent-mfd-d2"),
        ("HOME", home.to_str().unwrap()),
    ]);
    let out = MemFdExecutable::new("deep-rung", stub_code())
        .args(["print", "deep-rung-ok"])
        .stdout(Stdio::MakePipe)
        .stderr(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"deep-rung-ok\n");
    assert_eq!(out.stderr, b"");
    let stray: Vec<_> = std::fs::read_dir(&home)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(common::FALLBACK_PREFIX)
        })
        .collect();
    assert!(
        stray.is_empty(),
        "the HOME fallback must use .cache: {stray:?}"
    );
    let cache = home.join(".cache");
    if cache.is_dir() {
        common::assert_no_fallback_leftovers(&cache);
    }
}

/// An empty image must return ENOEXEC without a hang or panic.
#[test]
fn empty_image_fails_with_enoexec() {
    let _g = EnvGuard::empty();
    let err = MemFdExecutable::new("empty", b"").status().unwrap_err();
    assert_eq!(err.raw_os_error(), Some(8 /* ENOEXEC */));
    common::assert_no_fallback_leftovers(&common::tmpdir());
}

/// A NUL byte in an argument is rejected before the fork (std discipline) and
/// leaves no state behind.
#[test]
fn nul_byte_in_argument_is_rejected() {
    let _g = EnvGuard::empty();
    common::clear_stale_fallback_files(&common::tmpdir());
    let err = MemFdExecutable::new("nul-arg", stub_code())
        .arg("bad\0arg")
        .status()
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    common::assert_no_fallback_leftovers(&common::tmpdir());
}

/// One thousand arguments survive argv construction and the exec unchanged.
#[test]
fn large_argument_vectors_round_trip() {
    let _g = EnvGuard::empty();
    let args: Vec<String> = (0..1000).map(|i| format!("arg{i}")).collect();
    let mut exe = MemFdExecutable::new("many-args", stub_code());
    exe.arg("print").args(&args);
    let out = exe
        .arg("sentinel") // argv[1002]; proves nothing was dropped
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(
        out.stdout,
        format!("{} sentinel\n", args.join(" ")).into_bytes()
    );
}

/// An environment value that contains `=` round-trips unchanged through the
/// key=value reconstruction.
#[test]
fn environment_values_with_equals_signs_round_trip() {
    let _g = EnvGuard::empty();
    let out = MemFdExecutable::new("eq-env", stub_code())
        .arg("env")
        .arg("WEIRD_VALUE")
        .env("WEIRD_VALUE", "a=b=c=d")
        .stdout(Stdio::MakePipe)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"a=b=c=d\n");
}

/// `sealed(false)` after `prepare()` must drop the cached image: the next
/// spawn stages a fresh unsealed image instead of re-running the sealed one.
#[test]
fn unsealing_after_prepare_invalidates_the_cache() {
    let _g = EnvGuard::empty();
    let mut exe = MemFdExecutable::new("reseat", stub_code());
    exe.prepare().unwrap();
    assert!(exe.is_prepared() && exe.is_sealed());
    exe.sealed(false);
    assert!(!exe.is_prepared(), "sealed() must drop the cache");
    assert!(!exe.is_sealed());
    exe.prepare().unwrap();
    assert!(exe.is_prepared() && !exe.is_sealed());
    let st = exe.arg("exit").arg("0").status().unwrap();
    assert!(st.success());
}
