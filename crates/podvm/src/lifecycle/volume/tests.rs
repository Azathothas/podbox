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
    assert_eq!(parse(&format!("{link}:/data")).unwrap(), want);
    assert_eq!(parse(&format!("{link}:/data:rw")).unwrap(), want);
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1653. `:ro` is refused by name until T-1655, and each
/// shape with no meaning here is refused with its reason.
#[test]
fn a_volume_spelling_podvm_cannot_serve_is_refused() {
    let d = tmp("refuse");
    let h = d.to_str().unwrap();
    let refused = |spec: &str, want: &str| {
        let e = parse(spec).unwrap_err();
        assert!(e.contains(want), "{spec}: {e}");
    };
    refused(&format!("{h}:/data:ro"), "T-1655");
    refused(&format!("{h}:/data:rw,ro"), "T-1655");
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

/// The mount of a path is the longest mount point over it, the mount on
/// top where two share a point, and an escaped point reads unescaped.
#[test]
fn the_mount_of_a_path_is_the_longest_point_over_it() {
    let mi = "\
22 1 0:21 / / rw,relatime - ext4 /dev/vda rw
30 22 0:30 / /srv rw,nosuid - tmpfs tmpfs rw
31 30 0:31 / /srv/nx rw,noexec - tmpfs tmpfs rw
32 31 0:32 / /srv/nx rw,nodev - tmpfs tmpfs rw
33 22 0:33 / /my\\040dir rw,noexec - tmpfs tmpfs rw
";
    assert_eq!(mount_options(mi, "/home/u").as_deref(), Some("rw,relatime"));
    assert_eq!(mount_options(mi, "/srv/a").as_deref(), Some("rw,nosuid"));
    assert_eq!(mount_options(mi, "/srv/nxa").as_deref(), Some("rw,nosuid"));
    assert_eq!(mount_options(mi, "/srv/nx/b").as_deref(), Some("rw,nodev"));
    assert_eq!(mount_options(mi, "/my dir/x").as_deref(), Some("rw,noexec"));
    assert_eq!(mount_options("", "/x"), None);
}

/// TODO/enter.md T-1653. The banner names each volume as a rewrite, a
/// noexec source, and a source whose mount podvm cannot read.
#[test]
fn the_banner_names_a_noexec_source() {
    let v = |h: &str| Volume {
        guest: "/data".to_string(),
        host: h.to_string(),
    };
    let mi = "1 0 0:1 / / rw - ext4 r rw\n2 1 0:2 / /nx rw,noexec - tmpfs t rw\n";
    let b = banner_with(&[v("/home/d")], Some(mi));
    assert!(b.contains("-v /home/d:/data is a path rewrite"), "{b}");
    assert!(!b.contains("noexec"), "{b}");
    let b = banner_with(&[v("/nx/d")], Some(mi));
    assert!(b.contains("/nx/d is on a noexec mount"), "{b}");
    let b = banner_with(&[v("/home/d")], None);
    assert!(b.contains("cannot tell whether a program in /data"), "{b}");
}

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| (*s).to_string()).collect()
}

fn userland(family: Family) -> Userland {
    Userland {
        family,
        loader_argv: Vec::new(),
        lib_dirs: Vec::new(),
        banner: String::new(),
        guest_argv: Vec::new(),
        gaps: Vec::new(),
    }
}

/// TODO/enter.md T-1653. The loader family serves a volume; the chroot
/// rung, the memfd family and a forced rung refuse it, and so does a
/// working directory or a first program inside the volume.
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
    assert_eq!(refuse_paths("run", &vs, &v(&["/bin/sh"]), "/"), Ok(()));
    assert_eq!(refuse_paths("run", &vs, &v(&["sh"]), "/datax"), Ok(()));
    assert_eq!(refuse_paths("run", &vs, &v(&["data/x"]), "/"), Ok(()));
    assert_eq!(refuse_paths("run", &[], &v(&["/data/x"]), "/data"), Ok(()));
    assert_eq!(refuse_paths("run", &vs, &v(&["/bin/sh"]), "/data/sub"), err);
    assert_eq!(refuse_paths("run", &vs, &v(&["/data/prog"]), "/"), err);
}
