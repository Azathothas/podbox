//! TODO/enter.md T-1731: a dynamic program on the native-static rung. The
//! root holds the host's `cat` (a dynamic program of glibc), its
//! interpreter and its libraries, as `ldd` names them, and a marker that
//! the host has not. The loader maps the interpreter beside the program,
//! and each library that the interpreter opens resolves in the root.

#![cfg(target_arch = "x86_64")]

use std::path::{Path, PathBuf};
use std::process::Command;

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Each file that `ldd PROG` names with an absolute path: the libraries
/// and the interpreter.
fn needs(prog: &Path) -> Option<Vec<PathBuf>> {
    let out = Command::new("ldd").arg(prog).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut v = Vec::new();
    for line in text.lines() {
        let path = match line.split_once("=>") {
            Some((_, right)) => right.split_whitespace().next(),
            None => line.split_whitespace().next(),
        };
        if let Some(p) = path.filter(|p| p.starts_with('/')) {
            v.push(PathBuf::from(p));
        }
    }
    (!v.is_empty()).then_some(v)
}

/// A root with the dynamic `cat` of the host, or None where the host has
/// no dispatch, no `ldd` or no dynamic `cat`.
fn root() -> Option<Root> {
    if let Err(e) = podvm_dispatch::available() {
        eprintln!("SKIP: {e}");
        return None;
    }
    let cat = PathBuf::from("/bin/cat");
    let Some(libs) = needs(&cat) else {
        eprintln!("SKIP: no ldd, or /bin/cat is not dynamic");
        return None;
    };
    let d = std::env::temp_dir().join(format!("podvm-t1731-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("etc")).unwrap();
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("etc/podvm-marker"), "inroot\n").unwrap();
    std::fs::copy(&cat, d.join("bin/cat")).unwrap();
    for lib in libs {
        let to = d.join(lib.strip_prefix("/").unwrap());
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        // The file the link names, at the path that the program names.
        std::fs::copy(std::fs::canonicalize(&lib).unwrap(), &to).unwrap();
    }
    Some(Root(d))
}

#[test]
fn a_dynamic_program_of_the_root_runs_with_its_own_interpreter() {
    let Some(r) = root() else { return };
    let out = Command::new(env!("CARGO_BIN_EXE_podvm-dispatch-run"))
        .env_remove(podvm_dispatch::TIER_VAR)
        .arg(&r.0)
        // ROOT EXE ARGV0 ARGS, as `podvm-dispatch-run` takes them.
        .args(["/bin/cat", "cat", "/etc/podvm-marker"])
        .current_dir(&r.0)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    eprintln!("{:?} {}", out.status, String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.status.code(), Some(0), "the dynamic cat exits 0");
    assert_eq!(text, "inroot\n", "cat read the root's file, not the host's");
}
