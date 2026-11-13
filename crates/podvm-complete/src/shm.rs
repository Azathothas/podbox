//! `/dev/shm` of the rootfs, TODO/enter.md T-1759.
//!
//! ⭐ docker gives each container a tmpfs on `/dev/shm`, and python's
//! `multiprocessing`, `sem_open` and `shm_open` need it. podvm makes the
//! directory in the rootfs, sticky and writable by each user (mode 1777):
//! the namespace and chroot rungs run on a private copy of the rootfs, and
//! the path table of the userland rung maps `/dev/shm` into it. It is not
//! a tmpfs, so `--shm-size` stays unserved (`podvm run --help`).
//!
//! ⛔ A `dev` or `dev/shm` of the image that is a symbolic link, or not a
//! directory, is left as it is: a write through a link of the image could
//! leave the rootfs.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::{Action, Fixup};

/// The id of the row, and its entry.
pub const ID: &str = "dev-shm";
/// The row of `dev/pts`: the mount point that the namespace rung of
/// `pod-enter` binds the host's `/dev/pts` on (TODO/cli.md T-1760).
pub const PTS_ID: &str = "dev-pts";

/// Make `dev/shm` in `rootfs` where it is absent.
pub fn dev_shm(rootfs: &Path) -> Fixup {
    dev_dir(
        rootfs,
        "shm",
        ID,
        "T-1759",
        0o1777,
        "mode 1777, for shm_open and sem_open",
    )
}

/// Make `dev/pts` in `rootfs` where it is absent: with no directory there,
/// `tty` of a `run -t` found no name on the namespace rung.
pub fn dev_pts(rootfs: &Path) -> Fixup {
    dev_dir(
        rootfs,
        "pts",
        PTS_ID,
        "T-1760",
        0o755,
        "the mount point of the host's /dev/pts",
    )
}

/// Make `dev/NAME` with `mode` where it is absent.
fn dev_dir(
    rootfs: &Path,
    name: &str,
    id: &'static str,
    entry: &'static str,
    mode: u32,
    why: &str,
) -> Fixup {
    let row = |action, detail: &str| Fixup {
        entry,
        id,
        path: format!("dev/{name}"),
        action,
        detail: detail.into(),
        degraded: action == Action::Failed,
    };
    let dev = rootfs.join("dev");
    match std::fs::symlink_metadata(&dev) {
        Ok(m) if m.file_type().is_dir() => {}
        Ok(_) => return row(Action::Skipped, "the image's /dev is not a directory"),
        Err(e) => return row(Action::Skipped, &format!("the rootfs has no /dev: {e}")),
    }
    let at = dev.join(name);
    match std::fs::symlink_metadata(&at) {
        Ok(m) if m.file_type().is_dir() => return row(Action::Unchanged, ""),
        Ok(_) => {
            return row(
                Action::Skipped,
                &format!("the image's /dev/{name} is not a directory"),
            )
        }
        Err(_) => {}
    }
    let made = std::fs::create_dir(&at)
        .and_then(|()| std::fs::set_permissions(&at, std::fs::Permissions::from_mode(mode)));
    match made {
        Ok(()) => row(Action::Created, why),
        Err(e) => row(Action::Failed, &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn dev_shm_is_made_once_and_never_through_a_link() {
        let d = std::env::temp_dir().join(format!("podvm-t1759-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("a/dev")).unwrap();
        let f = dev_shm(&d.join("a"));
        assert_eq!(f.action, Action::Created, "{}", f.detail);
        let m = std::fs::metadata(d.join("a/dev/shm")).unwrap();
        assert_eq!(m.mode() & 0o7777, 0o1777);
        assert_eq!(dev_shm(&d.join("a")).action, Action::Unchanged);
        // T-1760: the mount point of /dev/pts, mode 755.
        assert_eq!(dev_pts(&d.join("a")).action, Action::Created);
        let m = std::fs::metadata(d.join("a/dev/pts")).unwrap();
        assert_eq!(m.mode() & 0o7777, 0o755);

        // A /dev of the image that is a link is left as it is.
        std::fs::create_dir_all(d.join("b")).unwrap();
        std::fs::create_dir_all(d.join("outside")).unwrap();
        std::os::unix::fs::symlink(d.join("outside"), d.join("b/dev")).unwrap();
        assert_eq!(dev_shm(&d.join("b")).action, Action::Skipped);
        assert!(!d.join("outside/shm").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
