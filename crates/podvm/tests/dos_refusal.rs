//! TODO/machine.md T-1736: `podvm windows run --guest dos` with no DOS base
//! image refuses by name, exits 125, and fetches or writes nothing. The
//! refusal is at `crates/podvm/src/windows/dos.rs` (`dos_run`).

use std::process::Command;

#[test]
fn a_dos_run_with_no_base_is_refused_by_name() {
    let home = std::env::temp_dir().join(format!("podvm-dos-refusal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let base = home.join("no-base.img");
    let out = Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(["windows", "run", "--guest", "dos", "ver"])
        .env("HOME", &home)
        .env("PODVM_DOS_BASE", &base)
        .env_remove("PODVM_STORE")
        .output()
        .expect("podvm runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(125), "{err}");
    assert!(err.contains("no DOS base image at"), "{err}");
    assert!(err.contains(&base.display().to_string()), "{err}");
    let left: Vec<_> = std::fs::read_dir(&home).unwrap().flatten().collect();
    assert!(left.is_empty(), "the refusal wrote {left:?}");
    let _ = std::fs::remove_dir_all(&home);
}
