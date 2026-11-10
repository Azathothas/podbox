use super::*;

/// T-1698: the destination of these tests is one level below the scratch
/// directory, so a whiteout that escapes it can only reach the sentinel
/// beside it, and never the shared temporary directory.
fn nested(s: &Scratch) -> std::path::PathBuf {
    let root = s.path().join("rootfs");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(s.path().join("sentinel"), b"keep").unwrap();
    root
}

/// ⛔ T-1698 (a security defect). `.wh...` at the layer root named `..`, and
/// its removal emptied the directory above the destination.
#[test]
fn a_whiteout_of_dotdot_at_the_root_refuses_the_layer() {
    let s = Scratch::new("whdotdot");
    let root = nested(&s);
    let l = layer(&[E::File(".wh...", b"", 0o644)]);
    match run(&root, &[l]) {
        Err(Error::Refused { entry, why, .. }) => {
            assert_eq!(entry, ".wh...");
            assert!(why.contains(".."), "{why}");
        }
        other => panic!("a whiteout of `..` was not refused: {other:?}"),
    }
    assert!(
        s.path().join("sentinel").exists(),
        "the whiteout left the destination"
    );
}

/// ⛔ T-1698. A whiteout whose directory climbs out is refused lexically, so
/// a kernel with no `openat2` never sees it.
#[test]
fn a_whiteout_that_climbs_out_refuses_the_layer() {
    let s = Scratch::new("whclimb");
    let root = nested(&s);
    for p in ["../.wh.sentinel", "a/../../.wh.sentinel", "../.wh..wh..opq"] {
        let l = layer(&[E::Raw(p, b"")]);
        match run(&root, &[l]) {
            Err(Error::Refused { entry, .. }) => assert_eq!(entry, p),
            other => panic!("{p} was not refused: {other:?}"),
        }
        assert!(
            s.path().join("sentinel").exists(),
            "{p} left the destination"
        );
    }
}

/// T-1698. A whiteout of `.` names the directory it sits in, which no layer
/// may delete by name; it refuses as an entry named `.` does.
#[test]
fn a_whiteout_of_dot_refuses_the_layer() {
    let s = Scratch::new("whdot");
    let root = nested(&s);
    let l = layer(&[E::Dir("d/", 0o755), E::File("d/.wh..", b"", 0o644)]);
    assert!(matches!(run(&root, &[l]), Err(Error::Refused { .. })));
    assert!(s.path().join("sentinel").exists());
}
