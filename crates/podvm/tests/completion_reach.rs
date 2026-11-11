//! TODO/complete.md T-1735: the completion layer of the shipped binary
//! never writes out of the rootfs through a link, and it follows a link
//! that stays inside. This is experiment 85 as a test: the tests of
//! `pod-rootfs` drive one function, and this drives `podvm run` through
//! a real image, extracted into a real store and tampered with the way an
//! image could ship.
//!
//! The assertion is on the filesystem: a canary outside the rootfs is
//! written before the run and read after it. Six doors:
//!
//! * A `/etc/mtab`, B `/etc/resolv.conf`, C `/etc/hosts`, E `/etc/passwd`:
//!   each an absolute link out of the rootfs (T-0402 to T-0405).
//! * D `/etc/ssl/certs`: an absolute directory link out (T-0407). Inside
//!   the rootfs it dangles; the write lands at the rebased path, or the
//!   refusal is on the banner. A fixup that does nothing and says nothing
//!   fails (T-0804).
//! * F `/etc/apt -> /var/aptreal`: a link that stays inside, which the
//!   layer must follow (T-0407).
//!
//! The image tree is planted, and each container has its own copy of it
//! (T-1709), so the fixups are read in `<store>/trees/`. The banner is
//! quiet by default (T-1416), so the run passes `--verbose`.
//!
//! Live: it needs Linux, the registry and container entry, and it SKIPS
//! with its reason where one is missing, as `tests/detached_stdio.rs`
//! does.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const PODVM: &str = env!("CARGO_BIN_EXE_podvm");

fn run_podvm(args: &[&str], store: &Path, deadline: Duration) -> Output {
    let mut child = Command::new(PODVM)
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn podvm {args:?}: {e}"));
    let start = Instant::now();
    while child.try_wait().expect("poll podvm").is_none() {
        if start.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("podvm {args:?} outlived the {deadline:?} deadline");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.wait_with_output().expect("read podvm output")
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn first_line(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

/// The rootfs of the one container under `<store>/trees/`.
fn container_rootfs(store: &Path) -> PathBuf {
    let trees = store.join("trees");
    let mut found = Vec::new();
    for entry in fs::read_dir(&trees)
        .expect("the store has trees/")
        .flatten()
    {
        let p = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.join("etc").is_dir() {
            found.push(p);
        } else if p.join("rootfs/etc").is_dir() {
            found.push(p.join("rootfs"));
        }
    }
    assert_eq!(
        found.len(),
        1,
        "one container tree under {trees:?}: {found:?}"
    );
    found.remove(0)
}

fn rm_link(path: &Path) {
    if path.is_dir() && !path.is_symlink() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn completion_writes_stay_in_the_rootfs_and_follow_an_inside_link() {
    if !cfg!(target_os = "linux") {
        eprintln!("SKIP completion reach: container entry needs Linux");
        return;
    }
    let work = std::env::temp_dir().join(format!("podvm-completion-reach-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let store = work.join("store");
    let canary = work.join("canary");
    fs::create_dir_all(canary.join("dir")).unwrap();
    let image = std::env::var("PODVM_RUN_IMAGE")
        .unwrap_or_else(|_| "ghcr.io/pkgforge-dev/archlinux:latest".to_string());

    let pull = run_podvm(&["pull", &image], &store, Duration::from_secs(900));
    if !pull.status.success() {
        eprintln!(
            "SKIP completion reach: could not pull {image}: {}",
            text(&pull.stderr).trim()
        );
        let _ = fs::remove_dir_all(&work);
        return;
    }
    let extract = run_podvm(&["extract", &image], &store, Duration::from_secs(900));
    assert!(
        extract.status.success(),
        "extract: {}",
        text(&extract.stderr)
    );
    let stdout = text(&extract.stdout);
    let image_root = PathBuf::from(stdout.lines().last().expect("extract prints the rootfs"));
    assert!(image_root.join("etc").is_dir(), "{image_root:?}");

    // The canaries, outside the store.
    let files = ["mtab", "resolv", "hosts", "passwd"];
    for f in files {
        fs::write(canary.join(f), format!("CANARY-{f}-UNTOUCHED\n")).unwrap();
    }
    fs::write(
        canary.join("dir/ca-certificates.crt"),
        "CANARY-DIRFILE-UNTOUCHED\n",
    )
    .unwrap();

    // The six doors, planted in the image tree.
    let etc = image_root.join("etc");
    for (name, f) in [
        ("mtab", "mtab"),
        ("resolv.conf", "resolv"),
        ("hosts", "hosts"),
        ("passwd", "passwd"),
    ] {
        rm_link(&etc.join(name));
        symlink(canary.join(f), etc.join(name)).unwrap();
    }
    rm_link(&etc.join("ssl"));
    fs::create_dir_all(etc.join("ssl")).unwrap();
    symlink(canary.join("dir"), etc.join("ssl/certs")).unwrap();
    rm_link(&etc.join("apt"));
    let _ = fs::remove_dir_all(image_root.join("var/aptreal"));
    fs::create_dir_all(image_root.join("var/aptreal/apt.conf.d")).unwrap();
    symlink("/var/aptreal", etc.join("apt")).unwrap();

    let args = [
        "run",
        "--verbose",
        "--pull",
        "never",
        "--name",
        "reach",
        &image,
        "/bin/true",
    ];
    let run = run_podvm(&args, &store, Duration::from_secs(300));
    let err = text(&run.stderr);
    if !run.status.success() && err.contains("carries no interposer") {
        eprintln!(
            "SKIP completion reach: the binary has no interposer object: {}",
            err.trim()
        );
        let _ = run_podvm(&["rm", "-f", "reach"], &store, Duration::from_secs(60));
        let _ = fs::remove_dir_all(&work);
        return;
    }
    assert!(
        run.status.success(),
        "run exited {:?}: {err}",
        run.status.code()
    );

    // Each canary is intact: no write left the rootfs.
    for f in files {
        assert_eq!(
            first_line(&canary.join(f)),
            format!("CANARY-{f}-UNTOUCHED"),
            "door {f}\n{err}"
        );
    }
    assert_eq!(
        first_line(&canary.join("dir/ca-certificates.crt")),
        "CANARY-DIRFILE-UNTOUCHED",
        "door D\n{err}"
    );

    // The writes landed inside, in the container's own tree.
    let root = container_rootfs(&store);
    for name in ["mtab", "resolv.conf", "hosts", "passwd"] {
        let p = root.join("etc").join(name);
        assert!(!p.is_symlink(), "etc/{name} is still a link\n{err}");
        assert!(p.is_file(), "etc/{name} is not a regular file\n{err}");
    }
    let passwd = fs::read_to_string(root.join("etc/passwd")).unwrap();
    assert!(passwd.lines().any(|l| l.starts_with("root:")), "{passwd}");
    // F: the link that stays inside is followed.
    assert!(
        root.join("var/aptreal/apt.conf.d/99podvm").is_file(),
        "etc/apt -> /var/aptreal was not followed\n{err}"
    );
    // D: written at the rebased path, or refused on the banner.
    let rebased = root
        .join(canary.strip_prefix("/").unwrap())
        .join("dir/ca-certificates.crt");
    let reported = err.contains("complete: failed");
    assert!(
        rebased.is_file() || reported,
        "door D: podvm neither wrote {rebased:?} nor reported a refusal\n{err}"
    );

    let rm = run_podvm(&["rm", "-f", "reach"], &store, Duration::from_secs(120));
    assert!(rm.status.success(), "rm: {}", text(&rm.stderr));
    let _ = fs::remove_dir_all(&work);
}
