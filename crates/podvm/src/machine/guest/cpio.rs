//! A `newc` cpio writer for the initramfs of a guest, TODO/machine.md T-1656.
//!
//! The kernel unpacks each `newc` archive of the initrd in order, and a
//! later archive may add files to an earlier one. So the base archive of
//! an image (its rootfs) is built once and cached, and each run appends a
//! small archive (`/podvm-init`, the console nodes). Each entry of the
//! image has the owner and the group that the image gives it, from the
//! sidecar of the extraction ([`Newc::owners_from`], TODO/machine.md
//! T-1776): the extraction keeps the ids of its own process, and before
//! each file of a guest was root's. Each other entry is root's. The time
//! is 0, so the same tree gives the same bytes.
//!
//! Device nodes, sockets and FIFOs of the rootfs are left out: the kernel
//! makes the console nodes from the run archive, and `devtmpfs` serves the
//! rest. A file that does not read fails the archive, by name.
//!
//! A hard link of the tree is a hard link of the archive: the first name
//! carries the data, each other name the same inode number and no data,
//! and the kernel links it. Without that, the applets of busybox:musl (one
//! binary under about 400 names) filled 500 MB and the guest ran out of
//! memory while it unpacked them.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

/// `S_IFDIR`, `S_IFREG`, `S_IFLNK` and `S_IFCHR`.
pub const DIR: u32 = 0o040_000;
pub const REG: u32 = 0o100_000;
pub const LNK: u32 = 0o120_000;
pub const CHR: u32 = 0o020_000;

/// A `newc` archive in the making.
pub struct Newc<W: Write> {
    out: W,
    ino: u32,
    written: u64,
    /// The archive inode of each `(dev, ino)` of the tree with more than
    /// one link.
    links: HashMap<(u64, u64), u32>,
    /// The owner and the group of each name of the image (T-1776).
    owners: HashMap<String, (u32, u32)>,
}

impl<W: Write> Newc<W> {
    pub fn new(out: W) -> Self {
        Newc {
            out,
            ino: 1,
            written: 0,
            links: HashMap::new(),
            owners: HashMap::new(),
        }
    }

    /// The owners of the image from the sidecar of its extraction: one JSON
    /// object on each line, with `path`, `uid` and `gid`. A line that does
    /// not parse is left out, and its entry is root's.
    pub fn owners_from(&mut self, sidecar: &Path) -> io::Result<()> {
        let text = std::fs::read_to_string(sidecar)?;
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let id = |k: &str| v.get(k).and_then(serde_json::Value::as_u64);
            if let (Some(path), Some(uid), Some(gid)) =
                (v.get("path").and_then(|p| p.as_str()), id("uid"), id("gid"))
            {
                self.owners.insert(
                    path.trim_start_matches('/').to_string(),
                    (uid as u32, gid as u32),
                );
            }
        }
        Ok(())
    }

    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.out.write_all(bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    fn pad(&mut self) -> io::Result<()> {
        let n = (4 - (self.written % 4) as usize) % 4;
        self.put(&[0u8; 3][..n])
    }

    /// One entry: `name` without a leading `/`, the full `mode`, the data,
    /// and the device numbers of a node.
    pub fn entry(
        &mut self,
        name: &str,
        mode: u32,
        data: &[u8],
        rdev: (u32, u32),
    ) -> io::Result<()> {
        let nlink = if mode & 0o170_000 == DIR { 2 } else { 1 };
        let ino = if name == "TRAILER!!!" { 0 } else { self.ino };
        self.ino += 1;
        self.raw(name, mode, data, rdev, ino, nlink)
    }

    /// One entry with its inode number and its link count.
    fn raw(
        &mut self,
        name: &str,
        mode: u32,
        data: &[u8],
        rdev: (u32, u32),
        ino: u32,
        nlink: u32,
    ) -> io::Result<()> {
        let name = name.trim_start_matches('/');
        let (uid, gid) = self.owners.get(name).copied().unwrap_or((0, 0));
        let head = format!(
            "070701{ino:08x}{mode:08x}{:08x}{:08x}{nlink:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
            uid,
            gid,
            0,
            data.len(),
            0,
            0,
            rdev.0,
            rdev.1,
            name.len() + 1,
            0
        );
        self.put(head.as_bytes())?;
        self.put(name.as_bytes())?;
        self.put(&[0])?;
        self.pad()?;
        self.put(data)?;
        self.pad()
    }

    /// The trailer, and the writer back.
    pub fn finish(mut self) -> io::Result<W> {
        self.entry("TRAILER!!!", 0, &[], (0, 0))?;
        Ok(self.out)
    }

    /// Each directory, regular file and link under `root`, in a sorted walk,
    /// with the names relative to `root`.
    pub fn tree(&mut self, root: &Path) -> io::Result<()> {
        self.walk(root, root, "")
    }

    /// The tree under `root` at the guest path `at` (absolute), with an
    /// entry for each directory of `at` itself first. A file `root` is one
    /// entry at `at`.
    pub fn tree_at(&mut self, root: &Path, at: &str) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let at = at.trim_matches('/');
        let mut dir = String::new();
        let parts: Vec<&str> = at.split('/').filter(|c| !c.is_empty()).collect();
        for (i, c) in parts.iter().enumerate() {
            if !dir.is_empty() {
                dir.push('/');
            }
            dir.push_str(c);
            if i + 1 < parts.len() {
                self.entry(&dir, DIR | 0o755, &[], (0, 0))?;
            }
        }
        let md = std::fs::metadata(root)?;
        if md.is_file() {
            let data = std::fs::read(root)?;
            return self.entry(at, REG | (md.permissions().mode() & 0o7777), &data, (0, 0));
        }
        self.entry(at, DIR | (md.permissions().mode() & 0o7777), &[], (0, 0))?;
        self.walk(root, root, at)
    }

    fn walk(&mut self, root: &Path, dir: &Path, prefix: &str) -> io::Result<()> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let mut names: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        names.sort_by_key(|e| e.file_name());
        for e in names {
            let path = e.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let rel = if prefix.is_empty() {
                rel
            } else {
                format!("{prefix}/{rel}")
            };
            let md = std::fs::symlink_metadata(&path)?;
            let perm = md.permissions().mode() & 0o7777;
            let ft = md.file_type();
            if ft.is_dir() {
                self.entry(&rel, DIR | perm, &[], (0, 0))?;
                self.walk(root, &path, prefix)?;
            } else if ft.is_symlink() {
                let target = std::fs::read_link(&path)?;
                self.entry(
                    &rel,
                    LNK | 0o777,
                    target.to_string_lossy().as_bytes(),
                    (0, 0),
                )?;
            } else if ft.is_file() && md.nlink() > 1 {
                let key = (md.dev(), md.ino());
                if let Some(&ino) = self.links.get(&key) {
                    self.raw(&rel, REG | perm, &[], (0, 0), ino, 2)?;
                    continue;
                }
                let data = std::fs::read(&path).map_err(|err| {
                    io::Error::new(err.kind(), format!("{}: {err}", path.display()))
                })?;
                let ino = self.ino;
                self.ino += 1;
                self.links.insert(key, ino);
                self.raw(&rel, REG | perm, &data, (0, 0), ino, 2)?;
            } else if ft.is_file() {
                let data = std::fs::read(&path).map_err(|err| {
                    io::Error::new(err.kind(), format!("{}: {err}", path.display()))
                })?;
                self.entry(&rel, REG | perm, &data, (0, 0))?;
            }
        }
        Ok(())
    }
}

/// The data of the first regular member of the `newc` archive `a` whose
/// name ends with `suffix`.
pub fn member<'a>(a: &'a [u8], suffix: &str) -> Option<&'a [u8]> {
    let hex = |s: &[u8]| u32::from_str_radix(std::str::from_utf8(s).ok()?, 16).ok();
    let mut i = 0usize;
    while i + 110 <= a.len() && &a[i..i + 6] == b"070701" {
        let mode = hex(&a[i + 14..i + 22])?;
        let size = hex(&a[i + 54..i + 62])? as usize;
        let nsize = hex(&a[i + 94..i + 102])? as usize;
        let name = a.get(i + 110..i + 110 + nsize.saturating_sub(1))?;
        let start = (i + 110 + nsize + 3) & !3;
        let data = a.get(start..start + size)?;
        if mode & 0o170_000 == REG && name.ends_with(suffix.as_bytes()) {
            return Some(data);
        }
        i = (start + size + 3) & !3;
    }
    None
}
