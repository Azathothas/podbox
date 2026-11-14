//! TODO/interpose.md T-1844: `place` writes through a name of its own.

use super::place;

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-place-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join(".podvm")).unwrap();
    d
}

/// A symlink at the old fixed temporary name is not followed, and two
/// places of different objects leave one whole object and no temporary
/// file.
#[test]
fn place_temp_name_follows_no_symlink_and_leaves_one_whole_object() {
    let d = scratch("t1844");
    let outside = d.join("outside");
    std::fs::write(&outside, b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, d.join(".podvm/.interpose.so.tmp")).unwrap();
    let root = d.to_str().unwrap().to_string();
    place(&root, b"glibc object").unwrap();
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep");

    let a = vec![b'a'; 1 << 20];
    let b = vec![b'b'; 1 << 20];
    let threads: Vec<_> = [a.clone(), b.clone()]
        .into_iter()
        .map(|bytes| {
            let root = root.clone();
            std::thread::spawn(move || {
                for _ in 0..20 {
                    place(&root, &bytes).unwrap();
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let got = std::fs::read(d.join(".podvm/interpose.so")).unwrap();
    assert!(got == a || got == b, "a mixed or partial object");
    let left: Vec<String> = std::fs::read_dir(d.join(".podvm"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(".interpose.so.") && n != ".interpose.so.tmp")
        .collect();
    assert!(left.is_empty(), "temporary files left: {left:?}");
    let _ = std::fs::remove_dir_all(&d);
}
