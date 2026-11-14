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
//! A hard link inside the image stays a hard link inside the tree: the
//! names share one inode of this container's own tree, never an inode of
//! the image's tree or of another container. busybox:musl holds one binary
//! under about 400 names; a copy of each name took 500 MB.
//!
//! ⚠ What the copy does not keep: the extended attributes and the times of
//! the directories. A FIFO, a socket and a device node are not copied; the
//! extraction of a rootless store makes none of them as such.

use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
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
    /// Names that are a hard link to a file of the tree copied before.
    pub linked: u64,
    /// The wall time of the fill.
    pub ms: u128,
    /// TODO/image.md T-1781: the tree is the spare of the image, renamed
    /// into place; the counts are those of its fill.
    pub spare: bool,
    /// The first copy of each `(dev, ino)` of the image with more than one
    /// link.
    links: std::collections::HashMap<(u64, u64), PathBuf>,
}

impl Fill {
    /// The form of the fill, as the banner names it.
    pub fn form(&self) -> &'static str {
        if self.spare {
            return "the spare tree";
        }
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
    sweep(store);
    // The copy is made under a name of this process and renamed when it is
    // whole: a process killed during the copy leaves `.fill-PID-ID`, which
    // the next fill removes, and never a tree that a record could name.
    let partial = trees(store).join(format!("{FILLING}{}-{id}", std::process::id()));
    let dir = trees(store).join(&id);
    fs::create_dir(&partial)?;
    let started = Instant::now();
    let mut fill = Fill::default();
    let done = copy_dir(&src, &partial.join("rootfs"), &mut fill)
        .and_then(|_| match (sidecar.is_file(), sidecar.file_name()) {
            (true, Some(name)) => fs::copy(&sidecar, partial.join(name)).map(|_| ()),
            _ => Ok(()),
        })
        .and_then(|_| fs::rename(&partial, &dir));
    if let Err(e) = done {
        let _ = open_dirs(&partial);
        let _ = pod_extract::remove_tree(&partial);
        return Err(e);
    }
    fill.rootfs = dir.join("rootfs");
    fill.dir = dir;
    fill.ms = started.elapsed().as_millis();
    Ok(fill)
}

/// The prefix of a tree that a process is filling.
pub const FILLING: &str = ".fill-";

/// Remove each partial tree whose process is gone: a run that was killed
/// during its copy. A partial tree of a live process stays. The count of
/// the removed trees.
pub fn sweep(store: &Store) -> usize {
    let Ok(rd) = fs::read_dir(trees(store)) else {
        return 0;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|s| s.strip_prefix(FILLING))
            .and_then(|s| s.split('-').next())
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == std::process::id() || Path::new(&format!("/proc/{pid}")).exists() {
            continue;
        }
        let p = e.path();
        if open_dirs(&p).is_ok() && pod_extract::remove_tree(&p).is_ok() {
            n += 1;
        }
    }
    n
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
        Some(dir) => {
            // A directory of the image without the owner's write bit (0555
            // is common) refuses the removal of its entries to a user that
            // is not root (CI run 391).
            open_dirs(&dir)?;
            pod_extract::remove_tree(&dir).map_err(|e| io::Error::other(e.to_string()))
        }
        None => Ok(false),
    }
}

/// `O_PATH | O_NOFOLLOW | O_DIRECTORY`: a handle of the directory itself
/// that never follows a link.
#[cfg(target_arch = "x86_64")]
const DIR_HANDLE: i32 = 0o10_000_000 | 0o400_000 | 0o200_000;
#[cfg(not(target_arch = "x86_64"))]
const DIR_HANDLE: i32 = 0o10_000_000 | 0o100_000 | 0o40_000; // aarch64, the generic table

/// Add the owner's `rwx` to `dir` and to each directory under it. Each
/// step goes through `/proc/self/fd/N` of an `O_PATH` handle opened with
/// `O_NOFOLLOW`, so a link that the payload put in the tree is never
/// followed out of it, also when it takes the place of a directory during
/// the walk.
fn open_dirs(dir: &Path) -> io::Result<()> {
    let h = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(DIR_HANDLE)
        .open(dir)
    {
        Ok(h) => h,
        // A link, or an entry that is gone: nothing to open.
        Err(e) if matches!(e.raw_os_error(), Some(20 | 40 | 2)) => return Ok(()),
        Err(e) => return Err(e),
    };
    let at = PathBuf::from(format!("/proc/self/fd/{}", h.as_raw_fd()));
    let mode = fs::metadata(&at)?.permissions().mode() & 0o7777;
    if mode & 0o700 != 0o700 {
        fs::set_permissions(&at, fs::Permissions::from_mode(mode | 0o700))?;
    }
    for entry in fs::read_dir(&at)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            open_dirs(&at.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Run `f` with the owner's bits `bits` added to the mode of `path` where
/// the owner lacks them, and the mode restored after. The extraction gives
/// the files to the user that runs podvm, with the image's modes: a file of
/// mode 0000 (`/etc/shadow` of some images) is podvm's own and still
/// cannot be opened by a user that is not root.
fn with_owner_bits<T>(path: &Path, bits: u32, f: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    let mode = fs::symlink_metadata(path)?.permissions().mode() & 0o7777;
    if mode & bits == bits {
        return f();
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode | bits))?;
    let r = f();
    let back = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    let v = r?;
    back?;
    Ok(v)
}

fn copy_dir(src: &Path, dst: &Path, fill: &mut Fill) -> io::Result<()> {
    let mode = fs::symlink_metadata(src)?.permissions().mode() & 0o7777;
    with_owner_bits(src, 0o500, || copy_dir_readable(src, dst, fill, mode))
}

fn copy_dir_readable(src: &Path, dst: &Path, fill: &mut Fill, mode: u32) -> io::Result<()> {
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
        } else if kind.is_file() && meta.nlink() > 1 {
            let key = (meta.dev(), meta.ino());
            if let Some(first) = fill.links.get(&key) {
                fs::hard_link(first, &to)?;
                fill.linked += 1;
            } else {
                with_owner_bits(&from, 0o400, || copy_file(&from, &to, &meta, fill))?;
                fill.links.insert(key, to);
            }
        } else if kind.is_file() {
            with_owner_bits(&from, 0o400, || copy_file(&from, &to, &meta, fill))?;
        } else {
            fill.skipped += 1;
        }
    }
    // The mode of the source as it was before `with_owner_bits`.
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

/// TODO/image.md T-1781: the spare tree of an image, beside its own tree
/// (`<store>/rootfs/<digest>/spare/`), so that the removal of the image
/// removes it too. A run takes it with one rename in place of a copy, and
/// a detached `podvm tree-prefill` fills the next one.
pub const SPARE: &str = "spare";
/// The counts of the fill of a spare: `files bytes`, in the spare.
const SPARE_COUNTS: &str = ".counts";

/// The spare tree of the image `digest`.
pub fn spare_dir(store: &Store, digest: &str) -> PathBuf {
    let (src, _) = crate::paths(store, digest);
    src.parent().map(|p| p.join(SPARE)).unwrap_or(src)
}

/// Take the spare of `digest` as a new container tree, where there is one.
pub fn take_spare(store: &Store, digest: &str) -> Option<Fill> {
    let spare = spare_dir(store, digest);
    if !spare.join("rootfs").is_dir() {
        return None;
    }
    let started = Instant::now();
    let counts = fs::read_to_string(spare.join(SPARE_COUNTS)).unwrap_or_default();
    let mut it = counts
        .split_whitespace()
        .map(|n| n.parse::<u64>().unwrap_or(0));
    let (files, bytes) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
    fs::create_dir_all(trees(store)).ok()?;
    let dir = trees(store).join(new_id().ok()?);
    fs::rename(&spare, &dir).ok()?;
    let _ = fs::remove_file(dir.join(SPARE_COUNTS));
    Some(Fill {
        rootfs: dir.join("rootfs"),
        dir,
        copied: files,
        bytes,
        ms: started.elapsed().as_millis(),
        spare: true,
        ..Fill::default()
    })
}

/// Fill the spare of `digest` where it has none: `true` where this call
/// made it. The caller holds the image (T-0204), as for [`fill`].
pub fn prefill(store: &Store, digest: &str) -> io::Result<bool> {
    let spare = spare_dir(store, digest);
    if spare.exists() {
        return Ok(false);
    }
    let f = fill(store, digest)?;
    let counts = format!("{} {}\n", f.cloned + f.copied, f.bytes);
    let made =
        fs::write(f.dir.join(SPARE_COUNTS), counts).and_then(|()| fs::rename(&f.dir, &spare));
    match made {
        Ok(()) => Ok(true),
        Err(e) => {
            // Another process made the spare first, or the store went.
            let _ = open_dirs(&f.dir);
            let _ = pod_extract::remove_tree(&f.dir);
            if spare.exists() {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

/// TODO/image.md T-1781: the prefix of a tree that a removal put aside.
pub const TRASH: &str = ".trash-";

/// The store whose trash podvm empties at its exit, once [`discard`] put a
/// tree there.
static TRASHED: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Whether the removal of a tree goes to the background
/// (`PODVM_SPARE_TREE` is not `0`).
fn deferred() -> bool {
    std::env::var_os("PODVM_SPARE_TREE").is_none_or(|v| v != "0")
}

/// Put the container tree of `rootfs` aside with one rename, for a later
/// [`empty_trash`]: the removal of a large tree (37 405 entries of golang)
/// took seconds of each `run --rm`. With `PODVM_SPARE_TREE=0`, the same as
/// [`remove`].
pub fn discard(store: &Store, rootfs: &str) -> io::Result<bool> {
    let Some(dir) = tree_of(store, rootfs) else {
        return Ok(false);
    };
    if !deferred() {
        return remove(store, rootfs);
    }
    let aside = trees(store).join(format!("{TRASH}{}", new_id()?));
    match fs::rename(&dir, &aside) {
        Ok(()) => {
            if let Ok(mut t) = TRASHED.lock() {
                *t = Some(store.root().to_path_buf());
            }
            Ok(true)
        }
        Err(_) => remove(store, rootfs),
    }
}

/// The store whose trash this process filled, taken once.
pub fn trashed() -> Option<PathBuf> {
    TRASHED.lock().ok().and_then(|mut t| t.take())
}

/// Remove each tree of the trash; the count removed.
pub fn empty_trash(store: &Store) -> usize {
    let Ok(rd) = fs::read_dir(trees(store)) else {
        return 0;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let named = e.file_name().to_str().is_some_and(|s| s.starts_with(TRASH));
        let p = e.path();
        if named && open_dirs(&p).is_ok() && pod_extract::remove_tree(&p).is_ok() {
            n += 1;
        }
    }
    n
}
