//! TODO/image.md T-1741 (issue 281): a plain tar whose end padding is past
//! the last read of the extraction imports and extracts with no digest
//! mismatch.
//!
//! The tar is the reproduction of issue 281: one file of 60 000 bytes, the
//! end-of-archive marker (it ends at byte 61 952), then zeros to a record
//! of 10 240 bytes (71 680 bytes), as GNU tar writes it. Before
//! podcrates `v0.31.0` the hash stopped at the marker, and `extract`
//! refused the layer.

use std::path::PathBuf;
use std::process::Command;

fn podvm() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_podvm"))
}

#[test]
fn a_tar_padded_to_its_record_imports_and_extracts() {
    let d = std::env::temp_dir().join(format!("podvm-t1741-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut b = tar::Builder::new(Vec::new());
    let mut h = tar::Header::new_gnu();
    h.set_size(60_000);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, "f", &vec![0u8; 60_000][..]).unwrap();
    let mut bytes = b.into_inner().unwrap();
    assert_eq!(bytes.len(), 61_952, "the marker ends at 61 952");
    bytes.resize(71_680, 0);
    let tar = d.join("a.tar");
    std::fs::write(&tar, &bytes).unwrap();
    let store = d.join("store");
    let run = |args: &[&str]| {
        Command::new(podvm())
            .args(args)
            .env("PODVM_STORE", &store)
            .output()
            .unwrap()
    };
    let i = run(&["import", tar.to_str().unwrap(), "t1741:1"]);
    assert!(i.status.success(), "{}", String::from_utf8_lossy(&i.stderr));
    let e = run(&["extract", "t1741:1"]);
    let err = String::from_utf8_lossy(&e.stderr);
    assert!(e.status.success(), "extract: {err}");
    assert!(!err.contains("digest mismatch"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}
