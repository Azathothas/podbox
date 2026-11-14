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

/// TODO/gate.md T-1859, TODO/extract.md T-1891: the fixture holds a
/// directory of mode 0555, and `pod_extract::remove_tree` cannot remove
/// its file as a user who is not root (EACCES), so CI found nine stores
/// left. The tree is the test's own, so each directory gets owner write
/// first; the walk follows no link.
fn clean(s: &Store) {
    fn widen(dir: &std::path::Path) {
        let Ok(md) = fs::symlink_metadata(dir) else {
            return;
        };
        if !md.is_dir() {
            return;
        }
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(md.mode() | 0o700));
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            widen(&e.path());
        }
    }
    widen(s.root());
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

/// CI run 390 (a user that is not root): a file of mode 0000 and a
/// directory of mode 0000 of the image are podvm's own and are copied, and
/// the source and the copy keep the modes of the image.
#[test]
fn a_file_and_a_directory_of_mode_0000_are_copied_with_their_modes() {
    let s = store("zero");
    let (rootfs, _) = crate::paths(&s, DIGEST);
    fs::write(rootfs.join("etc/shadow"), "root:*:1::::::\n").unwrap();
    fs::set_permissions(rootfs.join("etc/shadow"), fs::Permissions::from_mode(0o0)).unwrap();
    fs::create_dir_all(rootfs.join("secret")).unwrap();
    fs::write(rootfs.join("secret/k"), "k").unwrap();
    fs::set_permissions(rootfs.join("secret"), fs::Permissions::from_mode(0o0)).unwrap();
    let f = fill(&s, DIGEST).unwrap();
    let mode = |p: &Path| fs::symlink_metadata(p).unwrap().permissions().mode() & 0o7777;
    assert_eq!(mode(&f.rootfs.join("etc/shadow")), 0);
    assert_eq!(mode(&f.rootfs.join("secret")), 0);
    assert_eq!(
        mode(&rootfs.join("etc/shadow")),
        0,
        "the source keeps its mode"
    );
    assert_eq!(mode(&rootfs.join("secret")), 0, "the source keeps its mode");
    fs::set_permissions(f.rootfs.join("secret"), fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(
        f.rootfs.join("etc/shadow"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(f.rootfs.join("secret/k")).unwrap(), "k");
    assert_eq!(
        fs::read_to_string(f.rootfs.join("etc/shadow")).unwrap(),
        "root:*:1::::::\n"
    );
    fs::set_permissions(rootfs.join("secret"), fs::Permissions::from_mode(0o700)).unwrap();
    clean(&s);
}

/// A hard link of the image stays one inode inside the container's tree,
/// and that inode is the tree's own: not the image's, not another tree's.
#[test]
fn a_hard_link_of_the_image_is_one_inode_of_the_tree() {
    let s = store("links");
    let (src, _) = crate::paths(&s, DIGEST);
    fs::create_dir_all(src.join("bin")).unwrap();
    fs::write(src.join("bin/busybox"), "applet").unwrap();
    for n in ["ls", "sh", "wget"] {
        fs::hard_link(src.join("bin/busybox"), src.join("bin").join(n)).unwrap();
    }
    let a = fill(&s, DIGEST).unwrap();
    let b = fill(&s, DIGEST).unwrap();
    assert_eq!(a.linked, 3);
    let ino = |p: &Path| fs::metadata(p).unwrap().ino();
    let first = ino(&a.rootfs.join("bin/busybox"));
    for n in ["ls", "sh", "wget"] {
        assert_eq!(ino(&a.rootfs.join("bin").join(n)), first, "{n}");
    }
    assert_ne!(first, ino(&src.join("bin/busybox")));
    assert_ne!(first, ino(&b.rootfs.join("bin/busybox")));
    clean(&s);
}

/// CI run 391 (a user that is not root): the removal of a tree opens each
/// read-only directory first, and never through a link out of the tree.
#[test]
fn a_tree_with_read_only_directories_is_removed_and_no_link_is_followed() {
    let s = store("rodirs");
    let f = fill(&s, DIGEST).unwrap();
    let outside = s.root().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o500)).unwrap();
    std::os::unix::fs::symlink(&outside, f.rootfs.join("ro-link")).unwrap();
    fs::set_permissions(f.rootfs.join("etc"), fs::Permissions::from_mode(0o555)).unwrap();
    assert!(remove(&s, &f.rootfs.to_string_lossy()).unwrap());
    assert!(!f.dir.exists());
    let mode = fs::metadata(&outside).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o500, "the walk followed the link out of the tree");
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).unwrap();
    clean(&s);
}

/// A partial tree of a process that is gone (a run killed during its copy)
/// goes at the next fill; a partial tree of a live process stays.
#[test]
fn a_partial_tree_of_a_dead_process_is_swept() {
    let s = store("sweep");
    fs::create_dir_all(trees(&s)).unwrap();
    // A pid above the kernel's maximum is never alive.
    let dead = trees(&s).join(format!("{FILLING}4194305-ab"));
    let live = trees(&s).join(format!("{FILLING}{}-cd", std::process::id()));
    fs::create_dir_all(dead.join("rootfs/ro")).unwrap();
    fs::set_permissions(dead.join("rootfs/ro"), fs::Permissions::from_mode(0o555)).unwrap();
    fs::create_dir_all(&live).unwrap();
    let f = fill(&s, DIGEST).unwrap();
    assert!(!dead.exists(), "the partial tree of a dead process stays");
    assert!(live.exists(), "the partial tree of a live process went");
    assert!(f.rootfs.join("etc/hosts").is_file());
    assert_eq!(f.dir.parent().unwrap(), trees(&s));
    clean(&s);
}

/// TODO/image.md T-1781: a spare is taken with one rename, keeps the counts
/// of its fill and the sidecar, holds no write of a tree taken before, and
/// a second prefill leaves a spare as it is.
#[test]
fn a_spare_is_taken_once_and_holds_no_write_of_a_run() {
    let s = store("spare");
    assert!(take_spare(&s, DIGEST).is_none(), "no spare yet");
    assert!(prefill(&s, DIGEST).unwrap(), "the first prefill makes it");
    assert!(!prefill(&s, DIGEST).unwrap(), "a second prefill keeps it");
    assert!(spare_dir(&s, DIGEST).join("rootfs").is_dir());
    let a = take_spare(&s, DIGEST).unwrap();
    assert!(a.spare && a.form() == "the spare tree");
    assert_eq!(a.copied + a.cloned, 2, "the counts of the fill");
    assert!(a.dir.join(".meta.jsonl").is_file());
    assert!(!a.dir.join(".counts").exists());
    assert!(tree_of(&s, a.rootfs.to_str().unwrap()).is_some());
    assert!(
        !spare_dir(&s, DIGEST).exists(),
        "the spare went to the tree"
    );
    fs::write(a.rootfs.join("etc/hosts"), "10.0.0.1 svc\n").unwrap();
    assert!(prefill(&s, DIGEST).unwrap());
    let b = take_spare(&s, DIGEST).unwrap();
    assert_eq!(
        fs::read_to_string(b.rootfs.join("etc/hosts")).unwrap(),
        "127.0.0.1 localhost\n",
        "the spare holds a write of the tree before it"
    );
    clean(&s);
}

/// TODO/image.md T-1781: a discarded tree goes aside with one rename and
/// the trash empties after; a path that is no container tree stays.
#[test]
fn a_discarded_tree_goes_aside_and_the_trash_empties() {
    let s = store("trash");
    let f = fill(&s, DIGEST).unwrap();
    assert!(discard(&s, f.rootfs.to_str().unwrap()).unwrap());
    assert!(!f.dir.exists(), "the tree is still in place");
    assert_eq!(trashed().as_deref(), Some(s.root()));
    assert_eq!(trashed(), None, "taken once");
    let (src, _) = crate::paths(&s, DIGEST);
    assert!(
        !discard(&s, src.to_str().unwrap()).unwrap(),
        "the image's own tree stays"
    );
    assert_eq!(empty_trash(&s), 1);
    assert_eq!(empty_trash(&s), 0);
    assert!(src.join("etc/hosts").is_file());
    clean(&s);
}
