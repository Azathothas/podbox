//! TODO/cli.md T-1695: `--label` reaches the container record. `ps
//! --filter label=KEY[=VALUE]` matches it, and `inspect` prints `.Labels`.
//!
//! Hermetic: the image is a one-file rootfs that `podvm import` records
//! (a static program), and a foreground `run` leaves its record.

use std::path::{Path, PathBuf};
use std::process::Command;

fn podvm(store: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_podvm"))
        // TODO/gate.md T-1859: no detached spare-tree fill writes into the
        // store after the test removed it.
        .env("PODVM_SPARE_TREE", "0")
        .args(args)
        .env("PODVM_STORE", store)
        .output()
        .expect("podvm runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A ustar archive of one executable file, in pure std.
fn write_ustar(path: &Path, name: &str, data: &[u8]) {
    assert!(data.len() < 8 << 30, "ustar size field: under 8 GiB");
    let mut hdr = [0u8; 512];
    hdr[..name.len()].copy_from_slice(name.as_bytes());
    hdr[100..108].copy_from_slice(b"0000755\0");
    hdr[108..116].copy_from_slice(b"0000000\0");
    hdr[116..124].copy_from_slice(b"0000000\0");
    hdr[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
    hdr[136..148].copy_from_slice(b"00000000000\0");
    hdr[148..156].copy_from_slice(b"        ");
    hdr[156] = b'0';
    hdr[257..262].copy_from_slice(b"ustar");
    hdr[263..265].copy_from_slice(b"00");
    let sum: u32 = hdr.iter().map(|b| u32::from(*b)).sum();
    hdr[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut tar = hdr.to_vec();
    tar.extend_from_slice(data);
    tar.resize(tar.len() + (512 - data.len() % 512) % 512, 0);
    tar.resize(tar.len() + 1024, 0);
    std::fs::write(path, tar).unwrap();
}

/// A store with one imported image and one created container that has the
/// labels `job=7` and `tier=x`. Its id.
fn labelled(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("podvm-labels-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tar = dir.join("r.tar");
    // A static program, which runs on each rung: a `#!` script needs an
    // interpreter in the image, which a one-file image has not (CI denies
    // chroot). The test binary itself is a static musl program; `--list`
    // prints the names of its tests and exits 0.
    let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    write_ustar(&tar, "hello", &me);
    let store = dir.join("store");
    let (rc, _, err) = podvm(&store, &["import", tar.to_str().unwrap(), "lab/one:1"]);
    assert_eq!(rc, 0, "import: {err}");
    // A foreground run that leaves its record: where chroot is denied, a
    // static program has no detached driver, so `create` refuses it there.
    let (rc, _, err) = podvm(
        &store,
        &[
            "run",
            "--name",
            "lab",
            "--label",
            "job=7",
            "--label=tier=x",
            "lab/one:1",
            "/hello",
            "--list",
        ],
    );
    assert_eq!(rc, 0, "run: {err}");
    let (rc, out, err) = podvm(
        &store,
        &["ps", "-a", "-q", "--no-trunc", "--filter", "name=lab"],
    );
    assert_eq!(rc, 0, "ps: {err}");
    (store, out.trim().to_string())
}

#[test]
fn label_reaches_ps_filter() {
    let (store, id) = labelled("ps");
    let ps = |f: &str| podvm(&store, &["ps", "-a", "-q", "--no-trunc", "--filter", f]).1;
    assert_eq!(ps("label=job=7").trim(), id, "KEY=VALUE matches");
    assert_eq!(ps("label=tier").trim(), id, "KEY matches any value");
    assert!(
        ps("label=job=8").trim().is_empty(),
        "another value does not match"
    );
    assert!(
        ps("label=nope").trim().is_empty(),
        "another key does not match"
    );
    let both = podvm(
        &store,
        &[
            "ps",
            "-a",
            "-q",
            "--no-trunc",
            "--filter",
            "label=job=7",
            "--filter",
            "label=tier=x",
        ],
    )
    .1;
    assert_eq!(both.trim(), id, "each filter must match");
    let _ = std::fs::remove_dir_all(store.parent().unwrap());
}

#[test]
fn label_reaches_inspect() {
    let (store, id) = labelled("inspect");
    let (rc, out, err) = podvm(&store, &["inspect", "--format", "{{.Labels}}", &id]);
    assert_eq!(rc, 0, "inspect: {err}");
    assert_eq!(out.trim(), "job=7,tier=x");
    let (rc, _, err) = podvm(&store, &["create", "--label", "=v", "lab/one:1", "/hello"]);
    assert_eq!(rc, 125, "a label with no key is a flag error: {err}");
    assert!(err.contains("--label takes KEY=VALUE"), "{err}");
    let _ = std::fs::remove_dir_all(store.parent().unwrap());
}
