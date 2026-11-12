//! A reader of one regular file of a squashfs 4.0 image with xz
//! compression, TODO/machine.md T-1723.
//!
//! The pinned Alpine kernel keeps the 9p modules in `boot/modloop-virt`,
//! a squashfs image. podvm reads the few modules that a guest needs out of
//! it, with no `unsquashfs` on the host. The reader takes what that image
//! has: xz blocks, basic and extended directories and regular files, and
//! fragments. Another compressor, or a path through a symlink, is an error
//! by name.
//!
//! The layout is the one of the kernel's `fs/squashfs` (`squashfs_fs.h`):
//! a superblock of 96 bytes, metadata blocks of up to 8 KiB, each with a
//! header of two bytes (bit 15: stored uncompressed), and data blocks whose
//! size word has bit 24 set where the block is stored uncompressed.

use std::io::Read;

const MAGIC: u32 = 0x7371_7368;
const XZ: u16 = 4;
const META_MAX: usize = 8192;
const NO_FRAGMENT: u32 = 0xffff_ffff;

const BASIC_DIR: u16 = 1;
const BASIC_FILE: u16 = 2;
const EXT_DIR: u16 = 8;
const EXT_FILE: u16 = 9;

fn le16(b: &[u8], at: usize) -> Result<u16, String> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| "squashfs: a read past the end".to_string())
}

fn le32(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| "squashfs: a read past the end".to_string())
}

fn le64(b: &[u8], at: usize) -> Result<u64, String> {
    b.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("eight bytes")))
        .ok_or_else(|| "squashfs: a read past the end".to_string())
}

fn unxz(b: &[u8], max: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    xz4rust::XzReader::new(std::io::Cursor::new(b.to_vec()))
        .take(max as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("squashfs: an xz block does not decode: {e}"))?;
    if out.len() > max {
        return Err(format!("squashfs: an xz block gives more than {max} bytes"));
    }
    Ok(out)
}

/// One squashfs image in memory.
pub struct Image<'a> {
    b: &'a [u8],
    block_size: u32,
    root: u64,
    inode_table: u64,
    dir_table: u64,
    frag_table: u64,
    frag_count: u32,
}

/// One inode, as far as this reader needs it.
enum Inode {
    Dir { block: u32, offset: u16, size: u32 },
    File(File),
}

struct File {
    start: u64,
    size: u64,
    fragment: u32,
    frag_offset: u32,
    blocks: Vec<u32>,
}

impl<'a> Image<'a> {
    pub fn new(b: &'a [u8]) -> Result<Self, String> {
        if le32(b, 0)? != MAGIC {
            return Err("squashfs: no squashfs magic".into());
        }
        if le16(b, 20)? != XZ {
            return Err(format!(
                "squashfs: compressor {} is not xz, the one this reader takes",
                le16(b, 20)?
            ));
        }
        if (le16(b, 28)?, le16(b, 30)?) != (4, 0) {
            return Err("squashfs: not version 4.0".into());
        }
        Ok(Image {
            b,
            block_size: le32(b, 12)?,
            frag_count: le32(b, 16)?,
            root: le64(b, 32)?,
            inode_table: le64(b, 64)?,
            dir_table: le64(b, 72)?,
            frag_table: le64(b, 80)?,
        })
    }

    /// One metadata block at `at`: its bytes and the position after it.
    fn meta_block(&self, at: u64) -> Result<(Vec<u8>, u64), String> {
        let at = at as usize;
        let h = le16(self.b, at)?;
        let len = (h & 0x7fff) as usize;
        let raw = self
            .b
            .get(at + 2..at + 2 + len)
            .ok_or("squashfs: a metadata block past the end")?;
        let bytes = if h & 0x8000 != 0 {
            raw.to_vec()
        } else {
            unxz(raw, META_MAX)?
        };
        Ok((bytes, (at + 2 + len) as u64))
    }

    /// `len` bytes of the metadata stream that starts at the block `block`
    /// (an absolute position) and the offset `offset` in it.
    fn meta(&self, block: u64, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(offset + len);
        let mut at = block;
        while out.len() < offset + len {
            let (bytes, next) = self.meta_block(at)?;
            if bytes.is_empty() {
                return Err("squashfs: an empty metadata block".into());
            }
            out.extend_from_slice(&bytes);
            at = next;
        }
        Ok(out[offset..offset + len].to_vec())
    }

    /// The inode of the reference `r` (block << 16 | offset).
    fn inode(&self, r: u64) -> Result<Inode, String> {
        let block = self.inode_table + (r >> 16);
        let off = (r & 0xffff) as usize;
        let head = self.meta(block, off, 16)?;
        match le16(&head, 0)? {
            BASIC_DIR => {
                let d = self.meta(block, off, 32)?;
                Ok(Inode::Dir {
                    block: le32(&d, 16)?,
                    size: le16(&d, 24)? as u32,
                    offset: le16(&d, 26)?,
                })
            }
            EXT_DIR => {
                let d = self.meta(block, off, 40)?;
                Ok(Inode::Dir {
                    size: le32(&d, 20)?,
                    block: le32(&d, 24)?,
                    offset: le16(&d, 34)?,
                })
            }
            BASIC_FILE => {
                let d = self.meta(block, off, 32)?;
                let (start, fragment, frag_offset, size) = (
                    le32(&d, 16)? as u64,
                    le32(&d, 20)?,
                    le32(&d, 24)?,
                    le32(&d, 28)? as u64,
                );
                let blocks = self.block_list(block, off + 32, size, fragment)?;
                Ok(Inode::File(File {
                    start,
                    size,
                    fragment,
                    frag_offset,
                    blocks,
                }))
            }
            EXT_FILE => {
                let d = self.meta(block, off, 56)?;
                let (start, size, fragment, frag_offset) =
                    (le64(&d, 16)?, le64(&d, 24)?, le32(&d, 44)?, le32(&d, 48)?);
                let blocks = self.block_list(block, off + 56, size, fragment)?;
                Ok(Inode::File(File {
                    start,
                    size,
                    fragment,
                    frag_offset,
                    blocks,
                }))
            }
            t => Err(format!(
                "squashfs: inode type {t} is not a directory or a regular file"
            )),
        }
    }

    fn block_list(&self, block: u64, off: usize, size: u64, frag: u32) -> Result<Vec<u32>, String> {
        let bs = self.block_size as u64;
        let n = if frag == NO_FRAGMENT {
            size.div_ceil(bs)
        } else {
            size / bs
        } as usize;
        let raw = self.meta(block, off, n * 4)?;
        (0..n).map(|i| le32(&raw, i * 4)).collect()
    }

    /// The inode reference of `name` in the directory, or `None`.
    fn lookup(&self, dir: &Inode, name: &str) -> Result<Option<u64>, String> {
        Ok(self
            .entries(dir, name)?
            .into_iter()
            .find(|(n, _)| n == name.as_bytes())
            .map(|(_, r)| r))
    }

    /// Each name of the directory with its inode reference.
    fn entries(&self, dir: &Inode, name: &str) -> Result<Vec<(Vec<u8>, u64)>, String> {
        let Inode::Dir {
            block,
            offset,
            size,
        } = dir
        else {
            return Err(format!(
                "squashfs: a component before {name} is not a directory"
            ));
        };
        // The size counts three bytes more than the listing (the `.` and `..`
        // of the kernel).
        let len = (*size as usize).saturating_sub(3);
        let d = self.meta(self.dir_table + *block as u64, *offset as usize, len)?;
        let mut at = 0usize;
        let mut out = Vec::new();
        while at + 12 <= d.len() {
            let count = le32(&d, at)? as usize + 1;
            let start = le32(&d, at + 4)? as u64;
            at += 12;
            for _ in 0..count {
                let off = le16(&d, at)? as u64;
                let nlen = le16(&d, at + 6)? as usize + 1;
                let n = d
                    .get(at + 8..at + 8 + nlen)
                    .ok_or("squashfs: a directory entry past the end")?;
                at += 8 + nlen;
                out.push((n.to_vec(), (start << 16) | off));
            }
        }
        Ok(out)
    }

    /// The names in the directory at `path`.
    pub fn names(&self, path: &str) -> Result<Vec<String>, String> {
        let mut node = self.inode(self.root)?;
        for c in path.split('/').filter(|c| !c.is_empty()) {
            let r = self
                .lookup(&node, c)?
                .ok_or_else(|| format!("squashfs: no {path} (no {c})"))?;
            node = self.inode(r)?;
        }
        Ok(self
            .entries(&node, path)?
            .into_iter()
            .map(|(n, _)| String::from_utf8_lossy(&n).into_owned())
            .collect())
    }

    /// The bytes of the regular file at `path` (components split by `/`).
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let mut node = self.inode(self.root)?;
        for c in path.split('/').filter(|c| !c.is_empty()) {
            let r = self
                .lookup(&node, c)?
                .ok_or_else(|| format!("squashfs: no {path} (no {c})"))?;
            node = self.inode(r)?;
        }
        match node {
            Inode::File(f) => self.data(&f),
            Inode::Dir { .. } => Err(format!("squashfs: {path} is a directory")),
        }
    }

    fn data(&self, f: &File) -> Result<Vec<u8>, String> {
        let bs = self.block_size as usize;
        let mut out = Vec::with_capacity(f.size as usize);
        let mut at = f.start as usize;
        for &w in &f.blocks {
            let len = (w & 0x00ff_ffff) as usize;
            if len == 0 {
                out.resize(out.len() + bs, 0);
                continue;
            }
            let raw = self
                .b
                .get(at..at + len)
                .ok_or("squashfs: a data block past the end")?;
            if w & (1 << 24) != 0 {
                out.extend_from_slice(raw);
            } else {
                out.extend_from_slice(&unxz(raw, bs)?);
            }
            at += len;
        }
        if f.fragment != NO_FRAGMENT {
            let frag = self.fragment(f.fragment)?;
            let tail = (f.size as usize).saturating_sub(out.len());
            let s = frag
                .get(f.frag_offset as usize..f.frag_offset as usize + tail)
                .ok_or("squashfs: a fragment shorter than its file")?;
            out.extend_from_slice(s);
        }
        out.truncate(f.size as usize);
        Ok(out)
    }

    /// The bytes of the fragment block `i`.
    fn fragment(&self, i: u32) -> Result<Vec<u8>, String> {
        if i >= self.frag_count {
            return Err(format!("squashfs: fragment {i} of {}", self.frag_count));
        }
        // The table holds the position of each metadata block of entries;
        // an entry is 16 bytes, so a block holds 512.
        let ptr = le64(self.b, self.frag_table as usize + (i as usize / 512) * 8)?;
        let e = self.meta(ptr, (i as usize % 512) * 16, 16)?;
        let (start, w) = (le64(&e, 0)? as usize, le32(&e, 8)?);
        let len = (w & 0x00ff_ffff) as usize;
        let raw = self
            .b
            .get(start..start + len)
            .ok_or("squashfs: a fragment block past the end")?;
        if w & (1 << 24) != 0 {
            Ok(raw.to_vec())
        } else {
            unxz(raw, self.block_size as usize)
        }
    }
}

#[cfg(test)]
#[path = "squashfs_tests.rs"]
mod tests;
