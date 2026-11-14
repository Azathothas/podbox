//! TODO/image.md T-1843: `extract --force` takes the hold of the image
//! (T-0204) before it removes the extracted root. A process that holds the
//! image lock exclusively, as the GC does, keeps the root whole: the
//! removal cannot start while the lock is held.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn podvm(store: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_podvm"));
    c.env("PODVM_STORE", store).env("HOME", store);
    c
}

fn rootfs_of(out: &[u8]) -> PathBuf {
    let text = String::from_utf8_lossy(out);
    PathBuf::from(text.lines().find(|l| l.ends_with("/rootfs")).unwrap())
}

#[test]
fn extract_force_lock_is_taken_before_the_removal() {
    if Command::new("flock").arg("--version").output().is_err() {
        eprintln!("SKIP: no flock(1) on this host");
        return;
    }
    let d = std::env::temp_dir().join(format!("podvm-t1843-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let store = d.join("store");
    let mut b = tar::Builder::new(Vec::new());
    let mut h = tar::Header::new_gnu();
    h.set_size(3);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, "marker", &b"abc"[..]).unwrap();
    let tar = d.join("a.tar");
    std::fs::write(&tar, b.into_inner().unwrap()).unwrap();
    let ok = |o: std::process::Output| {
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        o
    };
    ok(podvm(&store)
        .args(["import", tar.to_str().unwrap(), "t1843:1"])
        .output()
        .unwrap());
    let first = ok(podvm(&store).args(["extract", "t1843:1"]).output().unwrap());
    let marker = rootfs_of(&first.stdout).join("marker");
    assert!(marker.is_file());

    // The GC's side: an exclusive lock on each image lock of the store.
    let locks: Vec<PathBuf> = std::fs::read_dir(store.join("locks"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        // The image lock only: `HEX.lock`, not `HEX.extract.lock` or
        // `HEX.fill.lock`, which are the locks of other steps.
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".lock"))
                .is_some_and(|stem| !stem.contains('.'))
        })
        .collect();
    assert!(!locks.is_empty(), "no image lock in the store");
    let mut gc = Command::new("flock")
        .arg("-x")
        .arg(&locks[0])
        .args(["sleep", "3"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let start = Instant::now();
    let mut force = podvm(&store)
        .args(["extract", "--force", "t1843:1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // While the exclusive lock is held, the root stays whole.
    while start.elapsed() < Duration::from_millis(1500) {
        assert!(
            marker.is_file(),
            "extract --force removed the root while the image lock was held"
        );
        if force.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = gc.wait();
    let out = force.wait_with_output().unwrap();
    if out.status.success() {
        assert!(
            start.elapsed() >= Duration::from_millis(2000),
            "extract --force ended before the lock was released"
        );
        assert!(marker.is_file(), "the root came back without its file");
    } else {
        // A hold that refuses instead of waiting: the root is still whole.
        assert!(marker.is_file());
    }
    let _ = std::fs::remove_dir_all(&d);
}
