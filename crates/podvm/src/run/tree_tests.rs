use super::*;

const DIGEST: &str = "sha256:1709aa";

/// A store with an "extracted" image of one file.
fn store(name: &str) -> podvm_image::Store {
    let root = std::env::temp_dir().join(format!("podvm-t1709-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let s = podvm_image::Store::open(&root).unwrap();
    let (rootfs, _) = podvm_extract::paths(&s, DIGEST);
    std::fs::create_dir_all(rootfs.join("etc")).unwrap();
    std::fs::write(rootfs.join("etc/hosts"), "127.0.0.1 localhost\n").unwrap();
    s
}

#[test]
fn two_containers_of_one_image_have_two_trees() {
    let s = store("two");
    let a = Tree::fill("run", &s, DIGEST).unwrap();
    let b = Tree::fill("run", &s, DIGEST).unwrap();
    assert_ne!(a.rootfs(), b.rootfs());
    let (image, _) = podvm_extract::paths(&s, DIGEST);
    assert_ne!(a.rootfs(), image.to_string_lossy());
    std::fs::write(a.fill.rootfs.join("etc/marker"), "a\n").unwrap();
    std::fs::write(a.fill.rootfs.join("etc/hosts"), "10.0.0.1 svc\n").unwrap();
    assert!(
        !b.fill.rootfs.join("etc/marker").exists(),
        "b reads the write of a"
    );
    assert!(
        !image.join("etc/marker").exists(),
        "the image reads the write of a"
    );
    assert_eq!(
        std::fs::read_to_string(b.fill.rootfs.join("etc/hosts")).unwrap(),
        "127.0.0.1 localhost\n"
    );
    // A refused prepare drops its guard, and the tree goes with it.
    let dir = b.fill.dir.clone();
    drop(b);
    assert!(!dir.exists(), "the tree of a refused prepare stayed");
    a.keep();
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn rm_removes_the_container_tree() {
    let s = store("rm");
    let t = Tree::fill("create", &s, DIGEST).unwrap();
    let (rootfs, dir) = (t.rootfs(), t.fill.dir.clone());
    t.keep();
    assert!(dir.is_dir(), "keep removed the tree");
    let c = podvm_supervise::create(
        &s,
        Some("t1709"),
        "alpine",
        DIGEST,
        &rootfs,
        vec!["true".into()],
        Vec::new(),
        "/".into(),
        "chroot",
        Vec::new(),
        0,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(c.of.rootfs, rootfs);
    podvm_supervise::remove(&s, "t1709", false).unwrap();
    assert!(!dir.exists(), "rm kept the container's tree");
    let (image, _) = podvm_extract::paths(&s, DIGEST);
    assert!(
        image.join("etc/hosts").is_file(),
        "rm removed the image's tree"
    );
    let _ = std::fs::remove_dir_all(s.root());
}
