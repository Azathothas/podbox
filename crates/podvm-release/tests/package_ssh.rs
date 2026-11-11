//! TODO/packaging.md T-1737: `podvm-package-ssh` refuses a missing helper
//! and invalid ELF bytes by name, and packages the four helpers of a
//! release build. The port of the script experiments/394-ssh-package.sh
//! (gone; T-1405).
//!
//! The two refusals are hermetic. The package needs the native release
//! helpers and `podvm-release-licenses` under
//! `target/x86_64-unknown-linux-musl/release/`, and it SKIPS with its
//! reason where they are not built (`podvm-dev check` builds them first).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_podvm-package-ssh");
const HELPERS: [&str; 4] = ["node", "operator", "proxy", "shell"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-package-ssh-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn package(dir: &Path) -> Output {
    Command::new(BIN)
        .arg(dir)
        .arg("x86_64")
        .current_dir(root())
        .output()
        .expect("podvm-package-ssh runs")
}

fn executable(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_missing_helper_is_refused_by_name() {
    let d = scratch("missing");
    for h in ["operator", "proxy", "shell"] {
        executable(&d.join(h), b"x");
    }
    let out = package(&d);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("SSH-PACKAGE-FAIL missing executable: node"),
        "{err}"
    );
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn invalid_elf_bytes_are_refused_by_name() {
    let d = scratch("invalid");
    for h in HELPERS {
        executable(&d.join(h), b"invalid ELF fixture\n");
    }
    let out = package(&d);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains("SSH-PACKAGE-FAIL invalid ELF: node"), "{err}");
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn an_unknown_arch_cannot_run() {
    let d = scratch("arch");
    let out = Command::new(BIN)
        .arg(&d)
        .arg("sparc")
        .current_dir(root())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn the_release_helpers_are_packaged_with_their_notices() {
    let release = root().join("target/x86_64-unknown-linux-musl/release");
    let licenses = release.join("podvm-release-licenses");
    if let Some(h) = HELPERS.iter().find(|h| !release.join(h).is_file()) {
        eprintln!("SKIP package: the release helper {h} is not built");
        return;
    }
    if !licenses.is_file() {
        eprintln!("SKIP package: {} is not built", licenses.display());
        return;
    }
    let d = scratch("ok");
    for h in HELPERS {
        fs::copy(release.join(h), d.join(h)).unwrap();
    }
    let out = Command::new(BIN)
        .arg(&d)
        .arg("x86_64")
        .env("PODVM_LICENSES", &licenses)
        .current_dir(root())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(stdout.contains("SSH-PACKAGE-OK x86_64"), "{stdout}");
    let asset = root().join("dist/podvm-ssh-x86_64.tar.gz");
    let list = Command::new("tar")
        .arg("-tzf")
        .arg(&asset)
        .output()
        .unwrap();
    let members = String::from_utf8_lossy(&list.stdout);
    for m in [
        "./node",
        "./operator",
        "./proxy",
        "./shell",
        "./LICENSE",
        "./SSH-SHIM-LICENSE",
    ] {
        assert!(
            members.lines().any(|l| l == m),
            "{m} is not in the archive: {members}"
        );
    }
    let sum = fs::read_to_string(root().join("dist/podvm-ssh-x86_64.tar.gz.sha256")).unwrap();
    assert!(
        sum.trim_end().ends_with("  podvm-ssh-x86_64.tar.gz"),
        "{sum}"
    );
    let _ = fs::remove_dir_all(&d);
}
