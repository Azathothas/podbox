use super::*;
use podvm_enter::userland::Maps;

/// ⛔ Every declared field is built and every built field is declared.
#[test]
fn the_declared_container_fields_are_the_built_ones() {
    let d = std::env::temp_dir().join(format!("podvm-lc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let s = podvm_image::Store::open(&d).unwrap();
    let c = Container {
        id: "a".repeat(64),
        name: "n".into(),
        argv: vec!["true".into()],
        env: Vec::new(),
        working_dir: "/".into(),
        created_at: "now".into(),
        started_at: None,
        finished_at: None,
        state: State::Created,
        pid: None,
        launcher_pid: None,
        exit_code: None,
        noticed: None,
        wall_ms: None,
        end: None,
        of: podvm_supervise::table::Detail {
            image: "img".into(),
            manifest_digest: "sha256:0".into(),
            rootfs: "/tmp".into(),
            rung: "chroot".into(),
            userland_exec: Vec::new(),
            completion: vec!["created dev/null (dev-shim, T-0401)".into()],
            completion_degraded: 1,
        },
    };
    let built: Vec<&str> = container_fields(&s, &c).iter().map(|(k, _)| *k).collect();
    assert_eq!(built, CONTAINER_INSPECT_FIELDS);
    // ⛔ And a container with no exit code renders a dash, never a zero.
    let f = container_fields(&s, &c);
    assert_eq!(
        f.iter().find(|(k, _)| *k == "ExitCode").unwrap().1,
        "-".to_string()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// ⛔ A bare name means the passwd file's primary group, not the uid
/// repeated: the plant is a user whose gid differs from its uid, and
/// the old `(u, u)` answer fails it (TODO/interpose.md T-0711).
#[test]
fn a_bare_user_name_takes_its_primary_group_from_the_image() {
    let d = std::env::temp_dir().join(format!("podvm-user-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("etc")).unwrap();
    std::fs::write(
        d.join("etc/passwd"),
        "root:x:0:0:root:/root:/bin/sh\napp:x:1000:100:app::/bin/sh\n",
    )
    .unwrap();
    std::fs::write(
        d.join("etc/group"),
        "root:x:0:\napp:x:100:app:\nwheel:x:10:app\n",
    )
    .unwrap();
    let root = d.to_string_lossy().to_string();
    assert_eq!(resolve_user("run", &root, "app"), Ok((1000, 100)));
    assert_eq!(resolve_user("run", &root, "1000"), Ok((1000, 1000)));
    assert_eq!(resolve_user("run", &root, "app:wheel"), Ok((1000, 10)));
    assert_eq!(resolve_user("run", &root, "1000:100"), Ok((1000, 100)));
    assert!(resolve_user("run", &root, "nobody").is_err());
    assert!(resolve_user("run", &root, "app:nogroup").is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_ps_fields_are_the_ones_the_template_check_allows() {
    let c = Container {
        id: "b".repeat(64),
        name: "n".into(),
        argv: vec!["sleep".into(), "1".into()],
        env: Vec::new(),
        working_dir: "/".into(),
        created_at: "now".into(),
        started_at: None,
        finished_at: None,
        state: State::Running,
        pid: Some(7),
        launcher_pid: Some(6),
        exit_code: None,
        noticed: None,
        wall_ms: None,
        end: None,
        of: podvm_supervise::table::Detail {
            image: "img".into(),
            manifest_digest: "sha256:0".into(),
            rootfs: "/tmp".into(),
            rung: "chroot".into(),
            userland_exec: Vec::new(),
            completion: vec!["created dev/null (dev-shim, T-0401)".into()],
            completion_degraded: 1,
        },
    };
    let built: Vec<&str> = ps_fields(&c, false).iter().map(|(k, _)| *k).collect();
    assert_eq!(built, ps_field_names());
    assert!(format::check("{{.Status}}", &ps_field_names()).is_ok());
    assert!(format::check("{{.Nope}}", &ps_field_names()).is_err());
}

/// TODO/milestones.md T-1112. A guest that is not Linux is refused by
/// name with the missing leg, on every entry path, not pulled and failed.
#[test]
fn a_non_linux_guest_is_refused_by_name() {
    assert_eq!(
        ensure_linux_guest("run", "windows", "amd64"),
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    );
    assert_eq!(
        ensure_linux_guest("exec", "darwin", "arm64"),
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    );
    assert!(ensure_linux_guest("run", "linux", "amd64").is_ok());
    assert!(ensure_linux_guest("start", "linux", "arm64").is_ok());
}

/// ⭐ TODO/milestones.md T-1112. The guest arm exists now, and it does not
/// live on the OCI path: a Windows guest is routed to its own verb by
/// name, so a caller who asked here is told where it is rather than that
/// it does not exist. An OS with no driver is still refused outright.
#[test]
fn a_guest_that_is_not_linux_is_routed_to_its_own_verb_when_one_exists() {
    assert_eq!(guest_verb("windows"), Some("podvm windows run"));
    assert_eq!(guest_verb("darwin"), None);
    assert_eq!(guest_verb("freebsd"), None);
    // and the route is what the OCI path refuses with
    assert_eq!(
        ensure_linux_guest("run", "windows", "amd64"),
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    );
}

/// TODO/enter.md T-1411. The detached drive a record carries: the
/// loader argv where the loader family was decided, nothing everywhere
/// else, so `create` and `run -d` cannot disagree about which family
/// has a detached driver.
#[test]
fn the_record_carries_the_loader_drive_and_nothing_else() {
    let loader = Userland {
        family: podvm_enter::userland::Family::Loader,
        loader_argv: vec!["/r/lib/ld.so".to_string(), "/r/bin/sh".to_string()],
        lib_dirs: vec!["/r/lib".to_string()],
        banner: String::new(),
        guest_argv: vec!["/bin/sh".to_string()],
        gaps: Vec::new(),
        maps: Maps::of_root("/r"),
    };
    assert_eq!(
        detached_drive(Some(&loader)),
        vec!["/r/lib/ld.so".to_string(), "/r/bin/sh".to_string()]
    );
    let memfd = Userland {
        family: podvm_enter::userland::Family::Memfd,
        ..loader.clone()
    };
    assert!(detached_drive(Some(&memfd)).is_empty());
    assert!(detached_drive(None).is_empty());
}

/// TODO/enter.md T-1317. The pre-fetch tier's truth table: chroot, the
/// kernel's memfd, or an interposer object each admit; only all three
/// missing refuse. The wrapper reads the machine; this pins the
/// decision it feeds.
#[test]
fn entry_is_possible_where_any_family_may_run() {
    assert!(entry_possible(true, false, false));
    assert!(entry_possible(false, true, false));
    assert!(entry_possible(false, false, true));
    assert!(entry_possible(true, true, true));
    assert!(!entry_possible(false, false, false));
}

/// TODO/enter.md T-1317. The exact rung, decided pre-fixup: a usable
/// chroot decides chroot without reading the payload; a static
/// payload on a native image with a memfd kernel decides the memfd
/// family; a script and a foreign image refuse naming why.
#[test]
fn the_rung_decision_names_its_family_or_its_refusal() {
    fn findings(rows: Vec<(&'static str, podvm_probe::verdict::Outcome)>) -> podvm_probe::Findings {
        podvm_probe::Findings {
            rows,
            identity: podvm_probe::identity::Identity::default(),
            writable: Vec::new(),
            self_exe: String::new(),
        }
    }
    let chroot = "chroot(/tmp)";
    let ok = findings(vec![(chroot, podvm_probe::verdict::Outcome::ok())]);
    let denied = findings(vec![(
        chroot,
        podvm_probe::verdict::Outcome::denied(podvm_probe::sys::Errno(1)),
    )]);
    // Chroot usable: the payload is never read.
    let nowhere = Maps::of_root("/nowhere");
    let chosen = decide_entry("run", &nowhere, &[], &[], true, &ok, false, None);
    assert!(chosen.unwrap().is_none());
    // Static payload, native, memfd kernel: the memfd family.
    let d = std::env::temp_dir().join(format!("podvm-decide-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/prog"), static_pie()).unwrap();
    let root = d.to_string_lossy().to_string();
    let m = Maps::of_root(&root);
    let argv = ["prog".to_string()];
    let env = ["PATH=/bin".to_string()];
    // T-1648: only under the opt-in; the refusal without it is pinned in
    // `a_static_payload_refuses_without_the_opt_in` below.
    if podvm_enter::memfd::memfd_exec().runs() {
        let u = decide_entry("run", &m, &argv, &env, true, &denied, true, None)
            .unwrap()
            .expect("a static payload decides a family");
        assert_eq!(u.family, podvm_enter::userland::Family::Memfd);
    }
    // A script refuses naming the interpreter problem, at 126: found,
    // but not invocable on this rung. The message names the resolved
    // file and its `#!` line (T-1413).
    std::fs::write(d.join("bin/tool.sh"), b"#!/bin/sh\necho hi\n").unwrap();
    let argv = ["tool.sh".to_string()];
    let e = decide_entry("run", &m, &argv, &env, true, &denied, false, None).unwrap_err();
    assert_eq!(e, podvm_image::error::EXIT_CANNOT_INVOKE);
    // A name nothing resolves is 127, wherever the rung would run it.
    let argv = ["no-such-prog".to_string()];
    let e = decide_entry("run", &m, &argv, &env, true, &denied, false, None).unwrap_err();
    assert_eq!(e, podvm_image::error::EXIT_NOT_FOUND);
    // A foreign image refuses naming binfmt, at 125: podvm-side.
    let argv = ["prog".to_string()];
    let e = decide_entry("run", &m, &argv, &env, false, &denied, false, None).unwrap_err();
    assert_eq!(e, podvm_image::error::EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1648. Where chroot is denied a static payload refuses
/// at 125 without `--unsafe-host-paths`, whatever the memfd answers, and
/// the refusal names the static reason, the host paths and every route.
/// With the opt-in the same payload enters the memfd family, where a
/// memfd executes (the T-1688 probe; a sealed memfd refuses at 125 too).
#[test]
fn a_static_payload_refuses_without_the_opt_in() {
    let denied = podvm_probe::Findings {
        rows: vec![(
            "chroot(/tmp)",
            podvm_probe::verdict::Outcome::denied(podvm_probe::sys::Errno(1)),
        )],
        identity: podvm_probe::identity::Identity::default(),
        writable: Vec::new(),
        self_exe: String::new(),
    };
    let d = std::env::temp_dir().join(format!("podvm-static-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/prog"), static_pie()).unwrap();
    let root = d.to_string_lossy().to_string();
    let m = Maps::of_root(&root);
    let argv = ["prog".to_string()];
    let env = ["PATH=/bin".to_string()];
    let e = decide_entry("run", &m, &argv, &env, true, &denied, false, None).unwrap_err();
    assert_eq!(e, podvm_image::error::EXIT_RUNTIME_ERROR);
    for want in [
        "statically linked",
        "resolve on the host",
        "--unsafe-host-paths",
        "chroot(2) is permitted",
        "T-1651",
        "T-1652",
    ] {
        assert!(
            STATIC_HOST_PATHS.contains(want),
            "{want}: {STATIC_HOST_PATHS}"
        );
    }
    let opted = decide_entry("run", &m, &argv, &env, true, &denied, true, None);
    if podvm_enter::memfd::memfd_exec().runs() {
        let u = opted.unwrap().expect("the opt-in decides a family");
        assert_eq!(u.family, podvm_enter::userland::Family::Memfd);
        assert!(u.banner.contains("--unsafe-host-paths"), "{}", u.banner);
    } else {
        assert_eq!(opted.unwrap_err(), podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/cli.md T-1414. The refusal code follows the payload status:
/// `ladder::resolve_payload`'s absent sentence is 127, its escape and
/// loop sentences are 126, and anything else stays 125. Pinned here so
/// a rewording there fails here instead of silently returning 125.
#[test]
fn resolve_codes_follow_the_payload_status() {
    assert_eq!(
        resolve_code("prog names no file in the image"),
        podvm_image::error::EXIT_NOT_FOUND
    );
    assert_eq!(
        resolve_code("/x escapes the image"),
        podvm_image::error::EXIT_CANNOT_INVOKE
    );
    assert_eq!(
        resolve_code("/x has too many levels of symlinks"),
        podvm_image::error::EXIT_CANNOT_INVOKE
    );
    assert_eq!(
        resolve_code("something else went wrong"),
        podvm_image::error::EXIT_RUNTIME_ERROR
    );
}

/// A 64-bit little-endian static PIE: the smallest bytes the memfd
/// family decides on. Mirrors `podvm-enter/src/memfd.rs`'s own
/// builder, which this crate cannot reach across its test gate.
fn static_pie() -> Vec<u8> {
    let mut b = vec![0u8; 64 + 56];
    b[0..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[32..40].copy_from_slice(&64u64.to_le_bytes());
    b[54..56].copy_from_slice(&56u16.to_le_bytes());
    b[56..58].copy_from_slice(&1u16.to_le_bytes());
    b[64..68].copy_from_slice(&1u32.to_le_bytes());
    b
}

/// TODO/supervise.md T-1318. `-f`/`--follow` selects following; anything
/// else behaves as before.
#[test]
fn logs_follow_is_a_parsed_flag() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_logs(&v(&["-f", "c1"])).unwrap();
    assert!(o.follow);
    assert_eq!(o.want, "c1");
    let o = parse_logs(&v(&["--follow", "c1"])).unwrap();
    assert!(o.follow);
    assert_eq!(o.want, "c1");
    let o = parse_logs(&v(&["c1"])).unwrap();
    assert!(!o.follow);
    assert_eq!(o.want, "c1");
    assert!(parse_logs(&v(&[])).is_err());
}

/// TODO/cli.md T-1337. `--tail` takes a count in both spellings and
/// in any position; anything else is a flag error naming the value.
#[test]
fn logs_tail_takes_a_count_in_both_spellings() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_logs(&v(&["--tail", "5", "c1"])).unwrap();
    assert_eq!(o.tail, Some(5));
    let o = parse_logs(&v(&["c1", "--tail=0"])).unwrap();
    assert_eq!(o.tail, Some(0));
    let o = parse_logs(&v(&["-f", "--tail", "3", "c1"])).unwrap();
    assert!(o.follow);
    assert_eq!(o.tail, Some(3));
    let o = parse_logs(&v(&["c1"])).unwrap();
    assert_eq!(o.tail, None);
    for bad in ["--tail=x", "--tail=-1", "--tail=", "--tail", "--tail=5x"] {
        assert_eq!(
            parse_logs(&v(&[bad, "c1"])).unwrap_err(),
            EXIT_FLAG_ERROR,
            "{bad} was not refused"
        );
    }
}

/// TODO/cli.md T-1323. `cp` parses its flag and two positionals.
#[test]
fn cp_parses_recursive_and_two_paths() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_cp(&v(&["-r", "img:/a", "/b"])).unwrap();
    assert!(o.recursive);
    assert_eq!(o.a, "img:/a");
    assert_eq!(o.b, "/b");
    let o = parse_cp(&v(&["/b", "c:/a"])).unwrap();
    assert!(!o.recursive);
    assert!(parse_cp(&v(&["only"])).is_err());
    assert!(parse_cp(&v(&["a", "b", "c"])).is_err());
}

/// TODO/cli.md T-1323. A clean tree round-trips through `copy_tree`
/// with bytes intact, in both directions.
#[test]
fn copy_tree_round_trips_a_clean_tree() {
    let base = std::env::temp_dir().join(format!("podvm-cp-{}", std::process::id()));
    let src = base.join("src");
    let dst = base.join("dst");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("a"), b"aaa").unwrap();
    std::fs::write(src.join("sub").join("b"), b"bbb").unwrap();
    assert_eq!(copy_tree(&src, &dst, &src, true), 0);
    assert_eq!(std::fs::read(dst.join("a")).unwrap(), b"aaa");
    assert_eq!(std::fs::read(dst.join("sub").join("b")).unwrap(), b"bbb");
    let back = base.join("back");
    // ⚠ The inward root must exist, as a rootfs always does: the gate
    // resolves the root before it resolves anything under it.
    std::fs::create_dir_all(&back).unwrap();
    assert_eq!(copy_tree(&dst, &back, &back, false), 0);
    assert_eq!(std::fs::read(back.join("sub").join("b")).unwrap(), b"bbb");
    let _ = std::fs::remove_dir_all(&base);
}

/// TODO/cli.md T-1323. A symlink replicates as a symlink, verbatim
/// and unresolved: the target bytes are copied, so an absolute target
/// escapes nothing and no shadow bytes land. T-0305's rule, applied
/// to `cp`: distro rootfses are full of legitimate absolute links.
#[test]
fn copy_tree_replicates_a_symlink_without_resolving_it() {
    let base = std::env::temp_dir().join(format!("podvm-cp-link-{}", std::process::id()));
    let src = base.join("src");
    let dst = base.join("dst");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("good"), b"good").unwrap();
    std::os::unix::fs::symlink("/etc/shadow", src.join("evil")).unwrap();
    assert_eq!(copy_tree(&src, &dst, &src, true), 0);
    assert_eq!(std::fs::read(dst.join("good")).unwrap(), b"good");
    let meta = std::fs::symlink_metadata(dst.join("evil")).unwrap();
    assert!(meta.file_type().is_symlink(), "evil replicates as a link");
    assert_eq!(
        std::fs::read_link(dst.join("evil")).unwrap(),
        std::path::PathBuf::from("/etc/shadow")
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// TODO/cli.md T-1323. A destination through a pre-existing symlink
/// refuses the whole copy before a byte lands: the write would land
/// where the link points, not where it is named.
#[test]
fn copy_tree_refuses_a_destination_through_a_planted_symlink() {
    let base = std::env::temp_dir().join(format!("podvm-cp-plant-{}", std::process::id()));
    let src = base.join("src");
    let back = base.join("back");
    let planted = base.join("planted");
    let _ = std::fs::remove_dir_all(&base);
    // The source names sub/link/inner as real directories and files.
    std::fs::create_dir_all(src.join("sub").join("link")).unwrap();
    std::fs::write(src.join("sub").join("link").join("inner"), b"inner").unwrap();
    std::fs::write(src.join("top"), b"top").unwrap();
    // The destination already has sub/link as a symlink elsewhere.
    std::fs::create_dir_all(back.join("sub")).unwrap();
    std::fs::create_dir_all(&planted).unwrap();
    std::os::unix::fs::symlink(&planted, back.join("sub").join("link")).unwrap();
    assert_ne!(copy_tree(&src, &back, &back, false), 0);
    assert!(
        !planted.join("inner").exists(),
        "nothing lands through the link"
    );
    assert!(
        !back.join("top").exists(),
        "a refused copy writes nothing else either"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// TODO/cli.md T-1323. A single symlink out of a rootfs replicates
/// rather than refuses, even with an absolute target: the gate covers
/// the link's own path and the target stays opaque.
#[test]
fn copy_one_replicates_a_symlink_with_an_absolute_target() {
    let base = std::env::temp_dir().join(format!("podvm-cp-one-link-{}", std::process::id()));
    let root = base.join("rootfs");
    let out = base.join("out");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(root.join("etc")).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::os::unix::fs::symlink("/proc/self/mounts", root.join("etc").join("mtab")).unwrap();
    assert_eq!(
        copy_one(&root, "/etc/mtab", &out.join("mtab"), true, false),
        0
    );
    let got = out.join("mtab");
    assert!(std::fs::symlink_metadata(&got)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        std::fs::read_link(&got).unwrap(),
        std::path::PathBuf::from("/proc/self/mounts")
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// TODO/cli.md T-1323. An `image:path` of `../../x` never leaves the
/// rootfs: the same gate as a single file.
#[test]
fn copy_one_refuses_an_escape() {
    let base = std::env::temp_dir().join(format!("podvm-cp-escape-{}", std::process::id()));
    let root = base.join("rootfs");
    let out = base.join("out");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&root).unwrap();
    assert_ne!(copy_one(&root, "../../evil", &out, true, false), 0);
    assert!(!base.join("evil").exists());
    let _ = std::fs::remove_dir_all(&base);
}
