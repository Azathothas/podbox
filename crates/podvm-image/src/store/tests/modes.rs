//! TODO/supervise.md T-1715. The store root keeps the container table,
//! and the table keeps each `-e` value, so a new root is mode 0700 and an
//! exposed root is reported, never changed.

use std::os::unix::fs::PermissionsExt;

use super::*;

fn fresh(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-store-mode-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn mode_of(p: &Path) -> u32 {
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

/// A store root that `open` creates is 0700 whatever the umask lets
/// through, and its parents keep the default modes.
#[test]
fn a_new_store_root_is_private() {
    let base = fresh("new");
    let root = base.join("deep").join("podvm");
    let store = Store::open(&root).unwrap();
    assert_eq!(mode_of(&root), 0o700);
    assert_eq!(store.exposed_mode(), None);
    // The parent is not podvm's: it has the mode of a directory made with
    // the default mode under the same umask, not a forced 0700.
    std::fs::create_dir(base.join("control")).unwrap();
    assert_eq!(mode_of(&base.join("deep")), mode_of(&base.join("control")));
    let _ = std::fs::remove_dir_all(&base);
}

/// A root that exists keeps its mode; `exposed_mode` names it where group
/// or other can reach the root.
#[test]
fn an_existing_open_root_is_reported_and_kept() {
    let root = fresh("open");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    let store = Store::open(&root).unwrap();
    assert_eq!(
        mode_of(&root),
        0o755,
        "open changed the mode of a root it did not create"
    );
    assert_eq!(store.exposed_mode(), Some(0o755));
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(store.exposed_mode(), None);
    let _ = std::fs::remove_dir_all(&root);
}
