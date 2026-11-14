//! TODO/cli.md T-1830: a `--tmpfs` directory ends with its run. An
//! `exec` into the container after the run sees the tree of the image at
//! that path, not a removed directory.
//!
//! The image is the host's `cat` and `tee` with the loader and libc of the
//! host, imported from a tar (as `volumes_default_rung.rs`), and a file
//! `/tmp/keep` of the tree. A host with no glibc at the paths of x86_64
//! skips the test with a line on stderr.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const FILES: &[(&str, &[&str])] = &[
    (
        "lib64/ld-linux-x86-64.so.2",
        &["/lib64/ld-linux-x86-64.so.2"],
    ),
    (
        "lib/x86_64-linux-gnu/libc.so.6",
        &[
            "/lib/x86_64-linux-gnu/libc.so.6",
            "/usr/lib/x86_64-linux-gnu/libc.so.6",
        ],
    ),
    // The glibc object of the interposer needs libdl and libgcc_s.
    (
        "lib/x86_64-linux-gnu/libdl.so.2",
        &[
            "/lib/x86_64-linux-gnu/libdl.so.2",
            "/usr/lib/x86_64-linux-gnu/libdl.so.2",
        ],
    ),
    (
        "lib/x86_64-linux-gnu/libgcc_s.so.1",
        &[
            "/lib/x86_64-linux-gnu/libgcc_s.so.1",
            "/usr/lib/x86_64-linux-gnu/libgcc_s.so.1",
        ],
    ),
    ("usr/bin/cat", &["/usr/bin/cat", "/bin/cat"]),
    ("usr/bin/tee", &["/usr/bin/tee", "/bin/tee"]),
];

/// The tar of the image, or `None` where the host lacks a file.
fn image_tar() -> Option<Vec<u8>> {
    let mut b = tar::Builder::new(Vec::new());
    for dir in ["tmp", "etc"] {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Directory);
        h.set_mode(if dir == "tmp" { 0o1777 } else { 0o755 });
        h.set_size(0);
        h.set_cksum();
        b.append_data(&mut h, format!("{dir}/"), &[][..]).ok()?;
    }
    let mut h = tar::Header::new_gnu();
    h.set_size(5);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, "tmp/keep", &b"tree\n"[..]).ok()?;
    for (inside, candidates) in FILES {
        let host = candidates.iter().map(Path::new).find(|p| p.exists())?;
        let bytes = std::fs::read(std::fs::canonicalize(host).ok()?).ok()?;
        let mut h = tar::Header::new_gnu();
        h.set_size(bytes.len() as u64);
        h.set_mode(0o755);
        h.set_cksum();
        b.append_data(&mut h, inside, &bytes[..]).ok()?;
    }
    b.into_inner().ok()
}

fn podvm(store: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_podvm"))
        // TODO/gate.md T-1859: no detached spare-tree fill writes into the
        // store after the test removed it.
        .env("PODVM_SPARE_TREE", "0")
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(stdin).unwrap();
    c.wait_with_output().unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn tmpfs_exec_after_the_run_sees_the_tree_of_the_image() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let Some(tar) = image_tar() else {
        eprintln!("T-1830: this host has no glibc at the x86_64 paths; skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("podvm-t1830-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("img.tar"), tar).unwrap();
    let store = dir.join("store");
    let tar = dir.join("img.tar");
    let i = podvm(&store, &["import", tar.to_str().unwrap(), "t1830:1"], b"");
    assert!(i.status.success(), "import: {}", text(&i.stderr));
    let r = podvm(
        &store,
        &[
            "run", "-i", "--name", "tx", "--tmpfs", "/tmp", "t1830:1", "tee", "/tmp/x",
        ],
        b"run\n",
    );
    assert!(r.status.success(), "run: {}", text(&r.stderr));
    // The file of the tree, which the tmpfs hid during the run.
    let e = podvm(&store, &["exec", "tx", "cat", "/tmp/keep"], b"");
    assert_eq!(e.status.code(), Some(0), "exec: {}", text(&e.stderr));
    assert_eq!(text(&e.stdout), "tree\n");
    // The file of the run went with its tmpfs.
    let g = podvm(&store, &["exec", "tx", "cat", "/tmp/x"], b"");
    assert_eq!(g.status.code(), Some(1), "exec: {}", text(&g.stderr));
    let _ = podvm(&store, &["rm", "-f", "tx"], b"");
    let _ = std::fs::remove_dir_all(&dir);
}
