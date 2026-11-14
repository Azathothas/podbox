//! TODO/extract.md T-1813: a layer row of mode 4755, 2755 or 6755 ends
//! with that mode in the tree, as root and as uid 1000. Before podcrates
//! `v0.44.0` (podcrates@T-581) the extraction dropped `S_ISUID` and
//! `S_ISGID` of a file whose mode already had owner write.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROWS: [(&str, u32); 3] = [("bin/u", 0o4755), ("bin/g", 0o2755), ("bin/ug", 0o6755)];

fn layer(dir: &Path) -> PathBuf {
    let mut b = tar::Builder::new(Vec::new());
    for (name, mode) in ROWS {
        let mut h = tar::Header::new_gnu();
        h.set_size(3);
        h.set_mode(mode);
        h.set_cksum();
        b.append_data(&mut h, name, &b"abc"[..]).unwrap();
    }
    let tar = dir.join("a.tar");
    std::fs::write(&tar, b.into_inner().unwrap()).unwrap();
    std::fs::set_permissions(&tar, std::fs::Permissions::from_mode(0o644)).unwrap();
    tar
}

/// Import and extract as the caller of `prefix` (empty: this process),
/// and return the rootfs that `extract` prints.
fn extract(prefix: &[&str], podvm: &Path, dir: &Path, tar: &Path) -> PathBuf {
    let run = |args: &[&str]| {
        let mut argv: Vec<&str> = prefix.to_vec();
        argv.push(podvm.to_str().unwrap());
        argv.extend_from_slice(args);
        let out = Command::new(argv[0])
            .args(&argv[1..])
            .env("HOME", dir)
            .env("PODVM_STORE", dir.join("store"))
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(out.status.success(), "{args:?}: {err}");
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    run(&["import", tar.to_str().unwrap(), "t1813:1"]);
    let out = run(&["extract", "t1813:1"]);
    let line = out
        .lines()
        .find(|l| l.ends_with("/rootfs"))
        .unwrap_or_else(|| panic!("no rootfs line in {out}"));
    PathBuf::from(line)
}

fn assert_modes(rootfs: &Path, who: &str) {
    for (name, mode) in ROWS {
        let m = std::fs::metadata(rootfs.join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(m, mode, "{who}: {name} is {m:o}, the layer says {mode:o}");
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-t1813-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn setuid_and_setgid_modes_survive_extraction() {
    let podvm = PathBuf::from(env!("CARGO_BIN_EXE_podvm"));
    let d = scratch("self");
    let tar = layer(&d);
    assert_modes(&extract(&[], &podvm, &d, &tar), "this process");
    let _ = std::fs::remove_dir_all(&d);

    // uid 1000: only where this test runs as root and `setpriv` exists.
    let root = Command::new("id").arg("-u").output().ok();
    let is_root = root.is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "0");
    let setpriv = ["/usr/bin/setpriv", "/bin/setpriv"]
        .into_iter()
        .find(|p| Path::new(p).exists());
    let (true, Some(setpriv)) = (is_root, setpriv) else {
        eprintln!("SKIP: the uid 1000 half needs root and setpriv");
        return;
    };
    let d = scratch("u1000");
    std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o777)).unwrap();
    let tar = layer(&d);
    // A copy that uid 1000 can execute: the build tree can be private.
    let copy = d.join("podvm");
    std::fs::copy(&podvm, &copy).unwrap();
    std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o755)).unwrap();
    let prefix = [
        setpriv,
        "--reuid",
        "1000",
        "--regid",
        "1000",
        "--clear-groups",
    ];
    assert_modes(&extract(&prefix, &copy, &d, &tar), "uid 1000");
    let _ = std::fs::remove_dir_all(&d);
}
