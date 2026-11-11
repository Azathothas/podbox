use super::*;
use std::os::unix::fs::MetadataExt;

const DIGEST: &str = "sha256:7709aa";

/// A store with an "extracted" image: a file, a link, a read-only
/// directory and the sidecar beside the rootfs.
fn store(name: &str) -> Store {
    let root = std::env::temp_dir().join(format!("podvm-tree-{}-{name}", std::process::id()));
    let _ = pod_extract::remove_tree(&root);
    let s = Store::open(&root).unwrap();
    let (rootfs, sidecar) = crate::paths(&s, DIGEST);
    fs::create_dir_all(rootfs.join("etc")).unwrap();
    fs::write(rootfs.join("etc/hosts"), "127.0.0.1 localhost\n").unwrap();
    std::os::unix::fs::symlink("/etc/hosts", rootfs.join("hosts-link")).unwrap();
    fs::create_dir_all(rootfs.join("ro")).unwrap();
    fs::write(rootfs.join("ro/file"), "x").unwrap();
    fs::set_permissions(rootfs.join("ro"), fs::Permissions::from_mode(0o555)).unwrap();
    fs::write(&sidecar, "{}\n").unwrap();
    s
}

fn clean(s: &Store) {
    let _ = pod_extract::remove_tree(s.root());
}

#[test]
fn a_fill_copies_files_links_modes_and_the_sidecar() {
    let s = store("copy");
    let f = fill(&s, DIGEST).unwrap();
    assert_eq!(f.rootfs, f.dir.join("rootfs"));
    assert_eq!(
        fs::read_to_string(f.rootfs.join("etc/hosts")).unwrap(),
        "127.0.0.1 localhost\n"
    );
    assert_eq!(
        fs::read_link(f.rootfs.join("hosts-link")).unwrap(),
        Path::new("/etc/hosts")
    );
    let mode = fs::metadata(f.rootfs.join("ro"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o555);
    assert!(
        f.dir.join(".meta.jsonl").is_file(),
        "the sidecar stays beside the rootfs"
    );
    assert_eq!(f.cloned + f.copied, 2);
    assert_ne!(f.form(), "empty");
    clean(&s);
}

#[test]
fn two_fills_share_no_inode_and_no_write() {
    let s = store("two");
    let a = fill(&s, DIGEST).unwrap();
    let b = fill(&s, DIGEST).unwrap();
    assert_ne!(a.dir, b.dir);
    let ia = fs::metadata(a.rootfs.join("etc/hosts")).unwrap().ino();
    let ib = fs::metadata(b.rootfs.join("etc/hosts")).unwrap().ino();
    assert_ne!(ia, ib, "two trees share an inode, so a write would cross");
    fs::write(a.rootfs.join("etc/hosts"), "10.0.0.1 svc\n").unwrap();
    assert_eq!(
        fs::read_to_string(b.rootfs.join("etc/hosts")).unwrap(),
        "127.0.0.1 localhost\n"
    );
    let (src, _) = crate::paths(&s, DIGEST);
    assert_eq!(
        fs::read_to_string(src.join("etc/hosts")).unwrap(),
        "127.0.0.1 localhost\n",
        "a write in a container tree reached the image"
    );
    clean(&s);
}

#[test]
fn only_a_tree_of_this_store_is_removed() {
    let s = store("remove");
    let f = fill(&s, DIGEST).unwrap();
    let (src, _) = crate::paths(&s, DIGEST);
    // The image's own tree, a path outside the store and a name that is
    // not an id are not container trees.
    assert!(tree_of(&s, &src.to_string_lossy()).is_none());
    assert!(tree_of(&s, "/tmp/rootfs").is_none());
    assert!(tree_of(&s, &trees(&s).join("../x/rootfs").to_string_lossy()).is_none());
    assert!(!remove(&s, &src.to_string_lossy()).unwrap());
    assert!(src.join("etc/hosts").is_file());
    assert_eq!(
        tree_of(&s, &f.rootfs.to_string_lossy()),
        Some(f.dir.clone())
    );
    assert!(remove(&s, &f.rootfs.to_string_lossy()).unwrap());
    assert!(!f.dir.exists());
    clean(&s);
}
