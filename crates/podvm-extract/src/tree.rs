//! The tree of one container (TODO/image.md T-1709).
//!
//! Each container gets its own copy of the extracted image, under
//! `<store>/trees/<id>/`: `rootfs/` and the sidecar `.meta.jsonl` beside it,
//! the same layout as the image's own tree. A write in one container is not
//! read by another, and completion edits only the container's own copy.
//!
//! The copy is made file by file. Each regular file is first cloned with
//! `FICLONE`, which shares the blocks and not the inode, where the
//! filesystem of the store supports it (btrfs, XFS). Elsewhere the kernel
//! copies the bytes (`copy_file_range` through `std::io::copy`). Never a hard
//! link: a hard link shares the inode, so a write in one container would be
//! read by the other.
//!
//! ⚠ What the copy does not keep: the extended attributes, the times of the
//! directories, and a hard link inside the image (each name gets its own
//! inode). A FIFO, a socket and a device node are not copied; the
//! extraction of a rootless store makes none of them as such.

use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;

use podvm_image::Store;

/// The directory of the container trees, under the store root.
pub const TREES: &str = "trees";

/// The cost of one fill, for the banner and for experiment 430.
#[derive(Debug, Default, Clone)]
pub struct Fill {
    /// `<store>/trees/<id>`.
    pub dir: PathBuf,
    /// `<store>/trees/<id>/rootfs`.
    pub rootfs: PathBuf,
    /// Regular files cloned with `FICLONE`.
    pub cloned: u64,
    /// Regular files that the kernel copied.
    pub copied: u64,
    /// The bytes of the regular files.
    pub bytes: u64,
    /// Entries that are not a directory, a regular file or a link.
    pub skipped: u64,
    /// The wall time of the fill.
    pub ms: u128,
}

impl Fill {
    /// The form of the fill, as the banner names it.
    pub fn form(&self) -> &'static str {
        match (self.cloned, self.copied) {
            (0, 0) => "empty",
            (_, 0) => "FICLONE",
            (0, _) => "copy",
            _ => "FICLONE and copy",
        }
    }

    /// One line for the banner.
    pub fn line(&self) -> String {
        format!(
            "podvm: the container's own rootfs: {} files, {:.1} MiB, by {}, in {} ms \
             (TODO/image.md T-1709)\n",
            self.cloned + self.copied,
            self.bytes as f64 / (1024.0 * 1024.0),
            self.form(),
            self.ms
        )
    }
}

/// `<store>/trees`.
pub fn trees(store: &Store) -> PathBuf {
    store.root().join(TREES)
}

/// A fresh id for a tree: 32 hex digits from `/dev/urandom`.
pub fn new_id() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    io::Read::read_exact(&mut fs::File::open("/dev/urandom")?, &mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Copy the extracted image `digest` into a new container tree.
///
/// The image must be extracted, and the caller holds its lock (T-0204), so
/// that no GC removes the source during the copy. A failure removes the
/// partial tree.
pub fn fill(store: &Store, digest: &str) -> io::Result<Fill> {
    let (src, sidecar) = crate::paths(store, digest);
    let id = new_id()?;
    fs::create_dir_all(trees(store))?;
    let dir = trees(store).join(&id);
    fs::create_dir(&dir)?;
    let started = Instant::now();
    let mut fill = Fill {
        rootfs: dir.join("rootfs"),
        dir: dir.clone(),
        ..Fill::default()
    };
    let done = copy_dir(&src, &fill.rootfs.clone(), &mut fill).and_then(|_| {
        match (sidecar.is_file(), sidecar.file_name()) {
            (true, Some(name)) => fs::copy(&sidecar, dir.join(name)).map(|_| ()),
            _ => Ok(()),
        }
    });
    if let Err(e) = done {
        let _ = pod_extract::remove_tree(&dir);
        return Err(e);
    }
    fill.ms = started.elapsed().as_millis();
    Ok(fill)
}

/// The tree directory of `rootfs` when it is a container tree of this
/// store (`<store>/trees/<id>/rootfs`), else `None`.
pub fn tree_of(store: &Store, rootfs: &str) -> Option<PathBuf> {
    let rootfs = Path::new(rootfs);
    if rootfs.file_name()? != "rootfs" {
        return None;
    }
    let dir = rootfs.parent()?;
    let id = dir.file_name()?.to_str()?;
    let well_formed = !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit());
    (well_formed && dir.parent()? == trees(store)).then(|| dir.to_path_buf())
}

/// Remove the container tree of `rootfs`. A rootfs that is not a container
/// tree (an image's own tree, a record of an older podvm) stays.
///
/// ⛔ The removal never follows a symlink out of the tree: the tree came
/// from a hostile archive, and the payload wrote in it.
pub fn remove(store: &Store, rootfs: &str) -> io::Result<bool> {
    match tree_of(store, rootfs) {
        Some(dir) => pod_extract::remove_tree(&dir).map_err(|e| io::Error::other(e.to_string())),
        None => Ok(false),
    }
}

fn copy_dir(src: &Path, dst: &Path, fill: &mut Fill) -> io::Result<()> {
    fs::create_dir(dst)?;
    // Owner-writable while the children are made; the image's mode last.
    fs::set_permissions(dst, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let meta = fs::symlink_metadata(&from)?;
        let kind = meta.file_type();
        if kind.is_dir() {
            copy_dir(&from, &to, fill)?;
        } else if kind.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&from)?, &to)?;
        } else if kind.is_file() {
            copy_file(&from, &to, &meta, fill)?;
        } else {
            fill.skipped += 1;
        }
    }
    let mode = fs::symlink_metadata(src)?.permissions().mode() & 0o7777;
    fs::set_permissions(dst, fs::Permissions::from_mode(mode))
}

fn copy_file(from: &Path, to: &Path, meta: &fs::Metadata, fill: &mut Fill) -> io::Result<()> {
    let mut src = fs::File::open(from)?;
    let mut dst = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(to)?;
    if clone(&dst, &src) {
        fill.cloned += 1;
    } else {
        io::copy(&mut src, &mut dst)?;
        fill.copied += 1;
    }
    fill.bytes += meta.len();
    if let Ok(t) = meta.modified() {
        let _ = dst.set_modified(t);
    }
    dst.set_permissions(fs::Permissions::from_mode(
        meta.permissions().mode() & 0o7777,
    ))
}

/// `ioctl(dst, FICLONE, src)`: true where the filesystem shared the blocks.
fn clone(dst: &fs::File, src: &fs::File) -> bool {
    const FICLONE: u64 = 0x4004_9409;
    #[cfg(target_arch = "x86_64")]
    const SYS_IOCTL: i64 = 16;
    #[cfg(not(target_arch = "x86_64"))]
    const SYS_IOCTL: i64 = 29; // aarch64 and riscv64, the generic table
    let args = [
        dst.as_raw_fd() as u64,
        FICLONE,
        src.as_raw_fd() as u64,
        0,
        0,
        0,
    ];
    // SAFETY: ioctl(2) with two descriptors that this function borrows for
    // the call; FICLONE reads no user memory.
    unsafe { pod_hostprobe::sys::sys(SYS_IOCTL, args) }.is_ok()
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
