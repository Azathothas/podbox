use super::*;
use podvm_probe::verdict::Outcome;

fn findings_with(rows: Vec<(&'static str, Outcome)>) -> Findings {
    Findings {
        rows,
        ..Findings::empty()
    }
}

/// Unset and empty mean the default entry: the ladder only engages on a
/// request, and no request is the common case.
#[test]
fn no_request_means_the_default_entry() {
    assert_eq!(parse_request("").unwrap(), None);
    assert_eq!(parse_request("   ").unwrap(), None);
    assert_eq!(parse_request("memfd").unwrap(), Some(Mode::Memfd));
}

/// An unknown word refuses with the rung list and exits docker's runtime
/// error, never a guess at what was meant.
#[test]
fn an_unknown_request_refuses_with_the_list() {
    let e = parse_request("memfd2").unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("memfd2"), "{text}");
    assert!(text.contains("rundir"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
}

/// Only foreground `run` drives the ladder. Every other verb, the
/// detached form and the machine tier refuse naming what was asked, and
/// no request refuses nowhere.
#[test]
fn the_ladder_is_foreground_run_or_a_named_refusal() {
    assert!(scope_refusal("run", false, false, "").is_none());
    assert!(scope_refusal("run", false, false, "memfd").is_none());
    for (verb, detach, machine) in [
        ("exec", false, false),
        ("create", false, false),
        ("run", true, false),
        ("run", false, true),
    ] {
        let text = scope_refusal(verb, detach, machine, "memfd")
            .expect("a force outside foreground run refuses");
        assert!(text.contains("PODVM_MODE"), "{text}");
        assert!(text.contains("memfd"), "{text}");
    }
}

/// The FUSE feed is the open row beside ptmx's rule: an `Ok` open is up,
/// a denial is down, and a missing row is down rather than a promise.
#[test]
fn fuse_is_up_only_where_the_open_row_answered_ok() {
    // ⛔ Tagged, as above: never share a removable base path.
    let base = std::env::temp_dir().join(format!("podvm-ladder-base-fuse-{}", std::process::id()));
    let up = findings_with(vec![("open(/dev/fuse, O_RDWR)", Outcome::ok())]);
    assert!(availability(&up, false, false, &base).fuse);
    let down = findings_with(vec![(
        "open(/dev/fuse, O_RDWR)",
        Outcome::denied(podvm_probe::sys::EPERM),
    )]);
    assert!(!availability(&down, false, false, &base).fuse);
    assert!(!availability(&Findings::empty(), false, false, &base).fuse);
    let _ = std::fs::remove_dir_all(&base);
}

/// The tmpfs feed is the attach verdict: only a mount the payload can use
/// counts, and anything less refuses naming the mount.
#[test]
fn tmpfs_is_up_only_where_a_mount_attached() {
    // ⛔ Tagged, not pid-only: the directory-rung tests below share
    // one base path shape and remove it mid-stage under parallel
    // threads (T-0413 drive: the directory-rungs test failed
    // in-suite at `assert!(avail.rundir)` and passed alone).
    let base = std::env::temp_dir().join(format!("podvm-ladder-base-tmpfs-{}", std::process::id()));
    let attached = findings_with(vec![("move_mount(-> /tmp/mm-probe)", Outcome::ok())]);
    assert!(availability(&attached, false, false, &base).tmpfs);
    assert!(!availability(&Findings::empty(), false, false, &base).tmpfs);
    let _ = std::fs::remove_dir_all(&base);
}

/// Rundir is up where the store takes staging; the cache additionally
/// needs asking. Claiming either where its base is missing is the
/// rung that does not exist.
#[test]
fn the_directory_rungs_need_their_stage_base() {
    // ⛔ Tagged, as above: the tmpfs test beside this one removes
    // the shared shape mid-stage.
    let base =
        std::env::temp_dir().join(format!("podvm-ladder-base-dirrung-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let avail = availability(&Findings::empty(), true, true, &base);
    assert!(avail.memfd);
    assert!(avail.rundir);
    assert!(avail.cache);
    assert!(avail.cache_requested);
    let _ = std::fs::remove_dir_all(&base);
    // Without the opt-in the cache stays down while rundir holds.
    let avail = availability(&Findings::empty(), false, false, &base);
    assert!(avail.rundir);
    assert!(!avail.cache);
    let _ = std::fs::remove_dir_all(&base);
}

/// A forced cache is admitted where asked with a writable base, and
/// refused naming the opt-in where not. Fork-free: the entry itself
/// is what the drive proves.
#[test]
fn a_forced_cache_is_admitted_only_where_asked() {
    use podvm_enter::ladder as ladder_mod;
    // ⛔ Tagged, as above.
    let base = std::env::temp_dir().join(format!(
        "podvm-ladder-base-forcecache-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let avail = availability(&Findings::empty(), false, true, &base);
    assert_eq!(
        ladder_mod::choose(Some(Mode::Cache), &avail, None).unwrap(),
        Mode::Cache
    );
    let avail = availability(&Findings::empty(), false, false, &base);
    let e = ladder_mod::choose(Some(Mode::Cache), &avail, None).unwrap_err();
    assert!(format!("{e}").contains("PODVM_CACHE=1"), "{e}");
    let _ = std::fs::remove_dir_all(&base);
}

/// A forced rundir over a missing payload refuses naming the force and
/// the missing file, not a rung decision: no rung can run what nothing
/// resolved. No fork happens: the resolve precedes the staging.
#[test]
fn a_forced_rundir_over_a_missing_payload_names_it() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // ⛔ Tagged, not pid-only: two tests sharing one store path race
    // each other's removal mid-stage (T-0413 drive, two green runs
    // each failing a different rundir test).
    let store = std::env::temp_dir().join(format!("podvm-ladder-store-mp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["true".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let usable = findings_with(vec![("chroot(/tmp)", Outcome::ok())]);
    let e = enter_forced(
        &root,
        &plan,
        Mode::RunDir,
        dir.to_str().unwrap(),
        "true",
        &plan.path_dirs,
        &usable,
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=rundir was forced"), "{text}");
    assert!(text.contains("names no file"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A forced rundir stages the tree whole: the staged root carries the
/// payload, and nothing is merged into an existing tree. Fork-free:
/// the entry itself is what the drive proves.
#[test]
fn a_forced_rundir_stages_the_tree_whole() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-stage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("bin/prog"), b"\x7fELF").unwrap();
    // ⛔ Tagged, as above: this store path collided with the missing-
    // payload test's under parallel threads.
    let store = std::env::temp_dir().join(format!("podvm-ladder-store-st-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    let staged = stage_rundir(&store, dir.to_str().unwrap()).expect("staging copies");
    assert_eq!(std::fs::read(staged.join("bin/prog")).unwrap(), b"\x7fELF");
    assert!(store.join("runs").is_dir());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A forced tmpfs over a missing payload refuses naming the force
/// and the missing file, not a rung decision: no rung can run what
/// nothing resolved. No fork happens: the resolve precedes the mount.
#[test]
fn a_forced_tmpfs_over_a_missing_payload_names_it() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-t{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["true".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let usable = findings_with(vec![
        ("chroot(/tmp)", Outcome::ok()),
        ("move_mount(-> /tmp/mm-probe)", Outcome::ok()),
    ]);
    let store = store_dir("t");
    let e = enter_forced(
        &root,
        &plan,
        Mode::Tmpfs,
        dir.to_str().unwrap(),
        "true",
        &plan.path_dirs,
        &usable,
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=tmpfs was forced"), "{text}");
    assert!(text.contains("names no file"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A forced fuse over a missing payload refuses naming the force
/// and the missing file, not a rung decision: no rung can run what
/// nothing resolved. No fork happens: the resolve precedes the serve.
#[test]
fn a_forced_fuse_over_a_missing_payload_names_it() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-f{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["true".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let usable = findings_with(vec![
        ("chroot(/tmp)", Outcome::ok()),
        ("open(/dev/fuse, O_RDWR)", Outcome::ok()),
    ]);
    let store = store_dir("f");
    let e = enter_forced(
        &root,
        &plan,
        Mode::Fuse,
        dir.to_str().unwrap(),
        "true",
        &plan.path_dirs,
        &usable,
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=fuse was forced"), "{text}");
    assert!(text.contains("names no file"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// TODO/enter.md T-1317. Where chroot is denied, a forced fuse rung
/// refuses naming the force and the denial rather than the rung:
/// their mechanisms need what this host denies, and only the memfd
/// rung enters without it.
#[test]
fn a_forced_fuse_rung_without_chroot_names_the_denial() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-nc{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["true".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let denied = findings_with(vec![(
        "chroot(/tmp)",
        Outcome::denied(podvm_probe::sys::EPERM),
    )]);
    let store = store_dir("nc");
    let e = enter_forced(
        &root,
        &plan,
        Mode::Fuse,
        dir.to_str().unwrap(),
        "true",
        &plan.path_dirs,
        &denied,
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=fuse was forced"), "{text}");
    assert!(text.contains("chroot(2) is denied"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A forced memfd over a missing payload refuses naming the force and
/// the missing file, not a rung decision: no rung can run what nothing
/// resolved. No fork happens: the resolve precedes every entry.
#[test]
fn a_forced_memfd_over_a_missing_payload_names_it() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-m{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["absent".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let store = store_dir("m");
    let e = enter_forced(
        &root,
        &plan,
        Mode::Memfd,
        dir.to_str().unwrap(),
        "absent",
        &plan.path_dirs,
        &Findings::empty(),
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=memfd was forced"), "{text}");
    assert!(text.contains("names no file"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A forced memfd over a script payload refuses naming the force and the
/// route-past, rather than exec'ing down a rung the bytes cannot take.
/// No fork happens here either: the refusal precedes the entry.
#[test]
fn a_forced_memfd_over_a_script_refuses_naming_both() {
    let dir = std::env::temp_dir().join(format!("podvm-ladder-cli-s{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("bin/run.sh"), b"#!/bin/sh\necho hi\n").unwrap();
    let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
    let plan = Plan {
        argv: vec!["run.sh".to_string()],
        env: Vec::new(),
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds::default(),
        banner: String::new(),
        path_dirs: vec!["/bin".to_string()],
    };
    let mut sink = Vec::new();
    let store = store_dir("s");
    let e = enter_forced(
        &root,
        &plan,
        Mode::Memfd,
        dir.to_str().unwrap(),
        "run.sh",
        &plan.path_dirs,
        &Findings::empty(),
        &store,
        "sha256:test",
        &mut sink,
    )
    .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("PODVM_MODE=memfd was forced"), "{text}");
    assert!(text.contains("routes past"), "{text}");
    assert_eq!(e.exit_code(), podvm_enter::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&store);
}

/// A fresh store root for the staging tests, removed by the caller.
fn store_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-ladder-store-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}
