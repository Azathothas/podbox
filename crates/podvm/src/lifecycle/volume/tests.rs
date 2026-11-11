use super::*;

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-vol-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// TODO/enter.md T-1653. `HOST:GUEST` and `HOST:GUEST:rw` parse to the
/// canonical host path; a link in the host side resolves, so the reverse
/// map of the interposer matches the working directory that the kernel
/// reports.
#[test]
#[cfg(unix)]
fn a_volume_parses_to_the_canonical_host_path() {
    let d = tmp("parse");
    let real = d.join("real");
    std::fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, d.join("link")).unwrap();
    let canon = std::fs::canonicalize(&real).unwrap();
    let want = Volume {
        guest: "/data".to_string(),
        host: canon.to_str().unwrap().to_string(),
    };
    let link = d.join("link");
    let link = link.to_str().unwrap();
    assert_eq!(
        parse(&format!("{link}:/data")).unwrap(),
        (want.clone(), false)
    );
    assert_eq!(
        parse(&format!("{link}:/data:rw")).unwrap(),
        (want.clone(), false)
    );
    // TODO/cli.md T-1655: `ro` reads, served by the read-only layer.
    assert_eq!(parse(&format!("{link}:/data:ro")).unwrap(), (want, true));
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1653. Each shape with no meaning here is refused with
/// its reason.
#[test]
fn a_volume_spelling_podvm_cannot_serve_is_refused() {
    let d = tmp("refuse");
    let h = d.to_str().unwrap();
    let refused = |spec: &str, want: &str| {
        let e = parse(spec).unwrap_err();
        assert!(e.contains(want), "{spec}: {e}");
    };
    refused(&format!("{h}:/data:z"), "has nothing to act on");
    refused("/data", "anonymous volume");
    refused("cache:/data", "named volume");
    refused(&format!("{h}:data"), "is not absolute");
    refused(
        &format!("{h}/missing:/data"),
        "refuses rather than creating",
    );
    refused("/a:/b:rw:x", "HOST:GUEST[:OPTIONS]");
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1653. The banner names each volume as a rewrite, a
/// noexec source, and a source whose mount flags podvm cannot read.
#[test]
fn the_banner_names_a_noexec_source() {
    let v = |h: &str| Volume {
        guest: "/data".to_string(),
        host: h.to_string(),
    };
    let flags = |h: &str| match h {
        "/nx/d" => Some(true),
        "/home/d" => Some(false),
        _ => None,
    };
    let b = banner_with(&[v("/home/d")], flags);
    assert!(b.contains("-v /home/d:/data is a path rewrite"), "{b}");
    assert!(!b.contains("noexec"), "{b}");
    let b = banner_with(&[v("/nx/d")], flags);
    assert!(b.contains("/nx/d is on a noexec mount"), "{b}");
    let b = banner_with(&[v("/gone/d")], flags);
    assert!(b.contains("cannot tell whether a program in /data"), "{b}");
}

/// TODO/enter.md T-1653. One `statfs(2)` answers the noexec flag of a
/// source: the answer agrees with the options of the mount table where
/// both name the same mount, and a missing path has no answer.
#[test]
fn statfs_answers_the_noexec_flag() {
    assert_eq!(noexec("/podvm-no-such-path/x"), None);
    assert_eq!(noexec("/a\0b"), None);
    let Ok(table) = std::fs::read_to_string("/proc/self/mountinfo") else {
        eprintln!("skipping, /proc/self/mountinfo cannot be read");
        return;
    };
    for line in table.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (Some(point), Some(opts)) = (f.get(4), f.get(5)) else {
            continue;
        };
        // One mount at the point, with no escape in it: the answer of
        // statfs is the answer of that mount.
        if point.contains('\\')
            || table
                .lines()
                .filter(|l| l.split(' ').nth(4) == Some(*point))
                .count()
                != 1
        {
            continue;
        }
        let Some(got) = noexec(point) else {
            continue;
        };
        let want = opts.split(',').any(|o| o == "noexec");
        assert_eq!(got, want, "{point}: {opts}");
    }
}

fn userland(family: Family) -> Userland {
    Userland {
        family,
        loader_argv: Vec::new(),
        lib_dirs: Vec::new(),
        banner: String::new(),
        guest_argv: Vec::new(),
        gaps: Vec::new(),
        maps: podvm_enter::userland::Maps::of_root("/r"),
        dispatch_exe: None,
        emulator: None,
        syscall: false,
    }
}

/// TODO/enter.md T-1653. The loader family serves a volume; the chroot
/// rung, the memfd family and a forced rung refuse it.
#[test]
fn only_the_loader_family_serves_a_volume() {
    let vs = [Volume {
        guest: "/data".to_string(),
        host: "/h".to_string(),
    }];
    let loader = userland(Family::Loader);
    let memfd = userland(Family::Memfd);
    let err = Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    assert_eq!(refuse_unserved("run", &vs, Some(&loader), None), Ok(()));
    assert_eq!(refuse_unserved("run", &[], None, None), Ok(()));
    assert_eq!(refuse_unserved("run", &vs, None, None), err);
    assert_eq!(refuse_unserved("run", &vs, Some(&memfd), None), err);
    assert_eq!(
        refuse_unserved(
            "run",
            &vs,
            Some(&loader),
            Some(podvm_enter::ladder::Mode::Memfd)
        ),
        err
    );
}

/// TODO/enter.md T-1653. The table of a run holds the volumes beside the
/// root pair, and a `-w` in a volume passes the check of the working
/// directory, which the image alone refuses; a volume that the table
/// cannot hold refuses at 125.
#[test]
fn a_workdir_in_a_volume_passes_through_the_table() {
    let d = tmp("workdir");
    std::fs::create_dir_all(d.join("rootfs/bin")).unwrap();
    std::fs::create_dir_all(d.join("host/sub")).unwrap();
    let root = d.join("rootfs").to_string_lossy().to_string();
    let vs = [Volume {
        guest: "/data".to_string(),
        host: d.join("host").to_string_lossy().to_string(),
    }];
    let m = maps("run", &root, &vs).unwrap();
    assert_eq!(m.volumes(), &vs[..]);
    assert!(podvm_enter::check_workdir(root.as_str(), "/data/sub", "img").is_err());
    podvm_enter::check_workdir(&m, "/data/sub", "img").unwrap();
    let bad = [Volume {
        guest: "/proc/x".to_string(),
        host: "/h".to_string(),
    }];
    assert_eq!(
        maps("run", &root, &bad).unwrap_err(),
        podvm_image::error::EXIT_RUNTIME_ERROR
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/cli.md T-1655. The read-only table: `--read-only` makes `/`
/// read-only with each writable volume as an exception; a `:ro` volume is
/// read-only; no flag gives no table, and a caller's own value is dropped.
#[test]
fn the_read_only_table_names_each_volume() {
    let v = |g: &str| Volume {
        guest: g.to_string(),
        host: "/h".to_string(),
    };
    let vols = [v("/workspace"), v("/state")];
    let ro = ["/state".to_string()];
    assert_eq!(
        read_only_table(true, &[], &vols),
        "ro:/,rw:/dev,rw:/workspace,rw:/state"
    );
    assert_eq!(
        read_only_table(true, &ro, &vols),
        "ro:/,rw:/dev,rw:/workspace,ro:/state"
    );
    assert_eq!(read_only_table(false, &ro, &vols), "ro:/state");
    let mut env = vec!["PODVM_RO=rw:/".to_string()];
    assert_eq!(
        read_only_env("run", false, &[], &vols, None, &mut env),
        Ok(())
    );
    assert!(env.is_empty(), "{env:?}");
    assert_eq!(
        read_only_env("run", true, &[], &vols, None, &mut env),
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    );
}

/// TODO/cli.md T-1726. An exec of a container takes the read-only table
/// of its run from the record, drops a caller's own value, and refuses it
/// by name on an entry with no read-only layer.
#[test]
fn an_exec_takes_the_read_only_table_of_its_container() {
    let record = vec!["A=1".to_string(), "PODVM_RO=ro:/e".to_string()];
    let mut env = vec!["PODVM_RO=rw:/".to_string(), "B=2".to_string()];
    let loader = userland(Family::Loader);
    container_read_only("exec", &record, Some(&loader), &mut env).unwrap();
    assert_eq!(env, ["B=2", "PODVM_RO=ro:/e"]);
    let mut env = vec!["B=2".to_string()];
    let memfd = userland(Family::Memfd);
    assert!(container_read_only("exec", &record, Some(&memfd), &mut env).is_err());
    assert!(container_read_only("exec", &record, None, &mut env).is_err());
    let mut env = vec!["PODVM_RO=ro:/".to_string()];
    container_read_only("exec", &["A=1".to_string()], None, &mut env).unwrap();
    assert!(
        env.is_empty(),
        "a caller's own table goes, also with none in the record"
    );
}
