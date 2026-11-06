use super::*;

/// TODO/enter.md T-1407. The maps table is set on a userland run, a
/// caller-supplied value is scrubbed first, and the banner names the
/// anchoring; `--unsafe-host-paths` opts out and the banner names the
/// opt-in instead.
#[test]
fn userland_env_anchors_paths_or_names_the_opt_out() {
    use podvm_enter::userland::{Family, MAPS_VAR};
    let env = vec![format!("{MAPS_VAR}=/:evil"), "PATH=/bin".to_string()];
    let ue = userland_env("run", "/store/rootfs", env, Family::Loader, false).unwrap();
    assert_eq!(
        ue.env
            .iter()
            .filter(|e| e.starts_with("PODVM_MAPS="))
            .count(),
        1,
        "{:?}",
        ue.env
    );
    assert!(
        ue.env.iter().any(|e| e == "PODVM_MAPS=/:/store/rootfs"),
        "{:?}",
        ue.env
    );
    assert!(!ue.unsafe_paths);
    assert!(ue.banner.contains("PODVM_MAPS"), "{}", ue.banner);
    let ue = userland_env(
        "run",
        "/store/rootfs",
        vec!["PATH=/bin".to_string()],
        Family::Memfd,
        true,
    )
    .unwrap();
    assert!(
        !ue.env.iter().any(|e| e.starts_with("PODVM_MAPS=")),
        "{:?}",
        ue.env
    );
    assert!(ue.unsafe_paths);
    assert!(ue.banner.contains("--unsafe-host-paths"), "{}", ue.banner);
    // A rootfs the table cannot spell refuses rather than rewriting
    // nothing: an empty table on this rung is a silent host write.
    assert_eq!(
        userland_env("run", "relative", vec![], Family::Loader, false).unwrap_err(),
        podvm_image::error::EXIT_RUNTIME_ERROR
    );
}

/// TODO/enter.md T-1409. `PODVM_GUEST_EXE_FORCE=1` rides only the
/// loader family beside an already-set `PODVM_GUEST_EXE`, exact `"1"`,
/// named once on the banner; a stale value is scrubbed first and the
/// memfd family never sees it.
#[test]
fn userland_env_forces_the_guest_exe_only_beside_the_loader() {
    use podvm_enter::userland::Family;
    let exe = format!("{}=/bin/prog", podvm_enter::ladder::GUEST_EXE_VAR);
    let ue = userland_env(
        "run",
        "/store/rootfs",
        vec![exe.clone(), "PODVM_GUEST_EXE_FORCE=0".to_string()],
        Family::Loader,
        false,
    )
    .unwrap();
    assert_eq!(
        ue.env
            .iter()
            .filter(|e| e.starts_with("PODVM_GUEST_EXE_FORCE="))
            .count(),
        1,
        "{:?}",
        ue.env
    );
    assert!(
        ue.env.iter().any(|e| e == "PODVM_GUEST_EXE_FORCE=1"),
        "{:?}",
        ue.env
    );
    assert!(ue.banner.contains("PODVM_GUEST_EXE"), "{}", ue.banner);
    // No guest exe set: no force, even on the loader family.
    let ue = userland_env("run", "/store/rootfs", vec![], Family::Loader, false).unwrap();
    assert!(
        !ue.env
            .iter()
            .any(|e| e.starts_with("PODVM_GUEST_EXE_FORCE=")),
        "{:?}",
        ue.env
    );
    // The memfd family never carries it, guest exe or not.
    let ue = userland_env("run", "/store/rootfs", vec![exe], Family::Memfd, false).unwrap();
    assert!(
        !ue.env
            .iter()
            .any(|e| e.starts_with("PODVM_GUEST_EXE_FORCE=")),
        "{:?}",
        ue.env
    );
}

/// TODO/image.md T-1418. The store-exec gate passes where a created
/// file executes and refuses naming the path where the probe cannot
/// run. The `no` verdict itself is the image crate's to prove; what
/// this pins is that the gate calls it and maps its answers.
#[test]
#[cfg(unix)]
fn store_exec_passes_where_a_created_file_executes() {
    let d = std::env::temp_dir().join(format!("podvm-storexec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    assert!(ensure_store_exec("run", &d).is_ok());
    assert_eq!(
        ensure_store_exec("run", &d.join("absent")),
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1411. The detached refusal names the blocking
/// call and every affected verb, so a caller learns what would lift
/// it rather than just what failed. The loader drive is named beside
/// it, because that family runs instead of refusing.
#[test]
fn the_detached_refusal_names_the_blocking_call_and_the_verbs() {
    let s = detached_userland_refusal("start", "memfd");
    assert!(s.contains("spawn_selected"), "{s}");
    assert!(s.contains("spawn_userland"), "{s}");
    assert!(s.contains("memfd"), "{s}");
    for v in ["run -d", "create", "start"] {
        assert!(s.contains(v), "{s}");
    }
}

#[test]
fn lifecycle_usage_strings_are_plain_ascii() {
    // TODO/cli.md T-1336: `ps`, `logs`, `restart` and `cp` usages
    // print on every `--help`; a glyph there is unrenderable bytes
    // in an automated caller's stream.
    let text = [PS_USAGE, LOGS_USAGE, RESTART_USAGE, CP_USAGE].concat();
    let bad = text.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "lifecycle usage carries {bad} non-ASCII bytes");
}

#[test]
fn a_container_path_is_split_from_a_plain_one() {
    assert_eq!(
        split("web:/etc/hosts"),
        Some(("web".into(), "/etc/hosts".into()))
    );
    assert_eq!(split("/etc/hosts"), None);
    // ⚠ A path with a colon in it is not a container reference.
    assert_eq!(split("/tmp/a:b"), None);
    assert_eq!(split("web:"), None);
}

#[test]
fn a_signal_is_taken_by_name_or_number_and_never_defaulted() {
    assert_eq!(signal_number("TERM"), Some(15));
    assert_eq!(signal_number("SIGKILL"), Some(9));
    assert_eq!(signal_number("9"), Some(9));
    // ⛔ Refused rather than defaulted: sending the wrong signal is not
    // something a caller can notice afterwards.
    assert_eq!(signal_number("NOPE"), None);
    assert_eq!(signal_number("0"), None);
    assert_eq!(signal_number("999"), None);
}

mod fields_and_copy;
