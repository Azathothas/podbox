//! TODO/cli.md T-1761: `-v`, `:ro`, `--tmpfs` and `--read-only` work on
//! the default rung of a host that permits chroot(2). Before, each was
//! refused at 125 with a message that named the chroot rung.
//!
//! The image is the host's `cat` and `tee` with the loader and libc of the
//! host, imported from a tar: the test needs no network. A host with no
//! glibc at the paths of x86_64 skips the test with a line on stderr.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn podvm() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_podvm"))
}

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
    for dir in ["tmp", "etc", "data"] {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Directory);
        h.set_mode(if dir == "tmp" { 0o1777 } else { 0o755 });
        h.set_size(0);
        h.set_cksum();
        b.append_data(&mut h, format!("{dir}/"), &[][..]).ok()?;
    }
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

struct Lane {
    dir: PathBuf,
    store: PathBuf,
}

impl Lane {
    fn run(&self, args: &[&str], stdin: &[u8]) -> Output {
        let mut c = Command::new(podvm())
            // TODO/gate.md T-1859: no detached spare-tree fill writes into the
            // store after the test removed it.
            .env("PODVM_SPARE_TREE", "0")
            .args(args)
            .env("PODVM_STORE", &self.store)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(stdin).unwrap();
        c.wait_with_output().unwrap()
    }
}

impl Drop for Lane {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn volumes_tmpfs_and_read_only_run_on_the_default_rung() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let Some(tar) = image_tar() else {
        eprintln!("T-1761: this host has no glibc at the x86_64 paths; skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("podvm-t1761-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let vol = dir.join("vol");
    std::fs::create_dir_all(&vol).unwrap();
    std::fs::write(vol.join("in.txt"), "invol\n").unwrap();
    std::fs::write(dir.join("img.tar"), tar).unwrap();
    let lane = Lane {
        store: dir.join("store"),
        dir,
    };
    let i = lane.run(
        &[
            "import",
            lane.dir.join("img.tar").to_str().unwrap(),
            "t1761:1",
        ],
        b"",
    );
    assert!(i.status.success(), "import: {}", text(&i.stderr));
    let v = vol.to_str().unwrap();
    let rw = format!("{v}:/data");
    let ro = format!("{v}:/data:ro");

    // A volume reads, and a write lands on the host.
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-v",
            &rw,
            "t1761:1",
            "/usr/bin/cat",
            "/data/in.txt",
        ],
        b"",
    );
    assert_eq!(text(&o.stdout), "invol\n", "-v read: {}", text(&o.stderr));
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-i",
            "-v",
            &rw,
            "t1761:1",
            "/usr/bin/tee",
            "/data/out.txt",
        ],
        b"w\n",
    );
    assert!(o.status.success(), "-v write: {}", text(&o.stderr));
    assert_eq!(std::fs::read_to_string(vol.join("out.txt")).unwrap(), "w\n");

    // `:ro` reads and refuses a write in the payload, not at 125.
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-v",
            &ro,
            "t1761:1",
            "/usr/bin/cat",
            "/data/in.txt",
        ],
        b"",
    );
    assert_eq!(text(&o.stdout), "invol\n", ":ro read: {}", text(&o.stderr));
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-i",
            "-v",
            &ro,
            "t1761:1",
            "/usr/bin/tee",
            "/data/ro.txt",
        ],
        b"x\n",
    );
    let code = o.status.code().unwrap_or(-1);
    assert!(
        code != 0 && code != 125,
        ":ro write exit {code}: {}",
        text(&o.stderr)
    );
    assert!(!vol.join("ro.txt").exists(), ":ro write reached the host");

    // `--tmpfs` is writable; `--read-only` refuses a write to the rootfs.
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-i",
            "--tmpfs",
            "/scratch",
            "t1761:1",
            "/usr/bin/tee",
            "/scratch/f",
        ],
        b"t\n",
    );
    assert!(o.status.success(), "--tmpfs: {}", text(&o.stderr));
    assert_eq!(text(&o.stdout), "t\n");
    let o = lane.run(
        &[
            "run",
            "--rm",
            "-i",
            "--read-only",
            "t1761:1",
            "/usr/bin/tee",
            "/etc/x",
        ],
        b"x\n",
    );
    let code = o.status.code().unwrap_or(-1);
    assert!(
        code != 0 && code != 125,
        "--read-only exit {code}: {}",
        text(&o.stderr)
    );
    assert!(
        text(&o.stderr).to_lowercase().contains("read-only"),
        "--read-only: {}",
        text(&o.stderr)
    );
}
