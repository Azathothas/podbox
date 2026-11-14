//! TODO/machine.md T-1736, T-1861: `podvm windows run --guest dos` with no
//! DOS base image fetches the pinned FreeDOS archive of the catalog into
//! the catalog store. When podvm cannot make that store, it refuses by
//! name, exits 125, and fetches nothing. The fetch is at
//! `crates/podvm/src/windows/pull.rs` (`dos_base`, `fetched`).

use std::process::Command;

#[test]
fn a_dos_run_that_cannot_store_its_base_is_refused_by_name() {
    let home = std::env::temp_dir().join(format!("podvm-dos-refusal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    // A file where the store's parent directory must be: the store
    // cannot exist, so no byte is fetched.
    let blocker = home.join("blocker");
    std::fs::write(&blocker, b"").unwrap();
    let base = home.join("no-base.img");
    let out = Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(["windows", "run", "--guest", "dos", "ver"])
        .env("HOME", &home)
        .env("PODVM_DOS_BASE", &base)
        .env("PODVM_WINDOWS_BASE", blocker.join("base.vhdx"))
        .env_remove("PODVM_STORE")
        .output()
        .expect("podvm runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(125), "{err}");
    assert!(
        err.contains(&blocker.join("catalog").display().to_string()),
        "{err}"
    );
    assert!(!err.contains("fetching"), "{err}");
    let left: Vec<_> = std::fs::read_dir(&home).unwrap().flatten().collect();
    assert_eq!(left.len(), 1, "the refusal wrote {left:?}");
    assert!(!base.exists());
    let _ = std::fs::remove_dir_all(&home);
}
