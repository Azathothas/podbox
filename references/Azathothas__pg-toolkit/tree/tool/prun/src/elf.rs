//! Just enough ELF to answer three questions about a payload.
//!
//! ⭐ The launcher needs to know the class (so it picks the right library
//! tree), whether the file has a `PT_INTERP` at all (a static payload needs no
//! loader), and what that interpreter string says (a payload whose interpreter
//! was rewritten at build time is started a different way). Nothing else.
//!
//! ⛔ It reads a bounded prefix and never the whole file. A payload is
//! routinely a hundred megabytes, and the program headers are in the first
//! page of every one of them; upstream reads the file to the end for a 32-bit
//! payload and reads to the section table for a 64-bit one, which is a whole
//! file's worth of page cache to answer a question the header already has.
//!
//! SPDX-License-Identifier: 0BSD

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// PT_INTERP, from the ELF specification.
const PT_INTERP: u32 = 3;

/// How much of the file the header walk may read. A program header table
/// past this is malformed rather than large: the ABI wants it in the first
/// page, and the biggest real one this project has seen is 14 entries.
const HEADER_LIMIT: u64 = 1 << 20;

/// What the launcher learned about a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// A 32-bit payload is started against the 32-bit library tree.
    pub elf32: bool,
    /// No `PT_INTERP` means nothing to hand a loader.
    pub statik: bool,
    /// The interpreter string, without its NUL, when there is one.
    pub interp: Option<String>,
}

/// Read a payload's header. An unreadable file, or one that is not an ELF, is
/// an error rather than a default: the caller has a real path in hand and a
/// wrong answer here picks the wrong start route.
pub fn read(path: &Path) -> std::io::Result<Payload> {
    let mut f = File::open(path)?;
    let mut ident = [0u8; 64];
    let n = f.read(&mut ident)?;
    if n < 16 || &ident[..4] != b"\x7fELF" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not an ELF file",
        ));
    }
    let elf32 = ident[4] == 1;
    let le = ident[5] != 2;
    let want = if elf32 { 52 } else { 64 };
    if n < want {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "ELF header is short",
        ));
    }

    let (phoff, phentsize, phnum) = if elf32 {
        (
            u32(&ident[28..32], le) as u64,
            u16(&ident[42..44], le) as usize,
            u16(&ident[44..46], le) as usize,
        )
    } else {
        (
            u64f(&ident[32..40], le),
            u16(&ident[54..56], le) as usize,
            u16(&ident[56..58], le) as usize,
        )
    };

    let least = if elf32 { 32 } else { 56 };
    if phnum == 0 || phentsize < least || phoff == 0 {
        // ⚠ No program header table is a legitimate ELF - a relocatable object
        // or a stripped-to-nothing file - and it has no interpreter, which is
        // the answer the caller wants rather than an error.
        return Ok(Payload { elf32, statik: true, interp: None });
    }
    let span = phoff.saturating_add((phnum * phentsize) as u64);
    if span > HEADER_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "program header table is not in the first megabyte",
        ));
    }

    let mut table = vec![0u8; phnum * phentsize];
    f.seek(SeekFrom::Start(phoff))?;
    f.read_exact(&mut table)?;

    for i in 0..phnum {
        let e = &table[i * phentsize..(i + 1) * phentsize];
        let ptype = u32(&e[0..4], le);
        if ptype != PT_INTERP {
            continue;
        }
        let (off, size) = if elf32 {
            (u32(&e[4..8], le) as u64, u32(&e[16..20], le) as u64)
        } else {
            (u64f(&e[8..16], le), u64f(&e[32..40], le))
        };
        if size == 0 || size > 4096 {
            break;
        }
        // ⛔ The string is read from the FILE rather than from any buffer
        // already in hand. A tool that rewrites an interpreter to a longer one
        // appends the new string at the end of the file and points the header
        // there, so a reader that only has the first page has the old string
        // or no string at all.
        let mut buf = vec![0u8; size as usize];
        f.seek(SeekFrom::Start(off))?;
        f.read_exact(&mut buf)?;
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let s = String::from_utf8_lossy(&buf[..end]).into_owned();
        return Ok(Payload { elf32, statik: false, interp: Some(s) });
    }
    Ok(Payload { elf32, statik: true, interp: None })
}

/// Whether a file begins with a shebang. A script is started by its own
/// interpreter and never by a loader.
pub fn is_script(path: &Path) -> std::io::Result<bool> {
    let mut f = File::open(path)?;
    let mut two = [0u8; 2];
    match f.read(&mut two)? {
        2 => Ok(&two == b"#!"),
        _ => Ok(false),
    }
}

fn u16(b: &[u8], le: bool) -> u16 {
    let a = [b[0], b[1]];
    if le {
        u16::from_le_bytes(a)
    } else {
        u16::from_be_bytes(a)
    }
}

fn u32(b: &[u8], le: bool) -> u32 {
    let a = [b[0], b[1], b[2], b[3]];
    if le {
        u32::from_le_bytes(a)
    } else {
        u32::from_be_bytes(a)
    }
}

fn u64f(b: &[u8], le: bool) -> u64 {
    let a = [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
    if le {
        u64::from_le_bytes(a)
    } else {
        u64::from_be_bytes(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A synthetic 64-bit little-endian ELF with one PT_INTERP, built here so
    // the reader is exercised without a payload on disk.
    fn synth(interp: &str, elf32: bool) -> Vec<u8> {
        let (ehsize, phentsize) = if elf32 { (52usize, 32usize) } else { (64usize, 56usize) };
        let phoff = ehsize as u64;
        let interp_off = (ehsize + phentsize) as u64;
        let mut v = vec![0u8; interp_off as usize + interp.len() + 1];
        v[..4].copy_from_slice(b"\x7fELF");
        v[4] = if elf32 { 1 } else { 2 };
        v[5] = 1;
        if elf32 {
            v[28..32].copy_from_slice(&(phoff as u32).to_le_bytes());
            v[42..44].copy_from_slice(&(phentsize as u16).to_le_bytes());
            v[44..46].copy_from_slice(&1u16.to_le_bytes());
            let p = ehsize;
            v[p..p + 4].copy_from_slice(&PT_INTERP.to_le_bytes());
            v[p + 4..p + 8].copy_from_slice(&(interp_off as u32).to_le_bytes());
            v[p + 16..p + 20].copy_from_slice(&((interp.len() + 1) as u32).to_le_bytes());
        } else {
            v[32..40].copy_from_slice(&phoff.to_le_bytes());
            v[54..56].copy_from_slice(&(phentsize as u16).to_le_bytes());
            v[56..58].copy_from_slice(&1u16.to_le_bytes());
            let p = ehsize;
            v[p..p + 4].copy_from_slice(&PT_INTERP.to_le_bytes());
            v[p + 8..p + 16].copy_from_slice(&interp_off.to_le_bytes());
            v[p + 32..p + 40].copy_from_slice(&((interp.len() + 1) as u64).to_le_bytes());
        }
        let s = interp_off as usize;
        v[s..s + interp.len()].copy_from_slice(interp.as_bytes());
        v
    }

    // ⚠ The pid is in the name, and it is not decoration. `fs.protected_regular`
    // is 1 on an ordinary kernel, which refuses an O_CREAT open of an existing
    // file in a sticky world-writable directory when the opener does not own it.
    // A fixed name under /tmp therefore works until somebody runs this suite as a
    // second uid, and then fails as PermissionDenied for a reason that reads like
    // a broken checkout. Measured: five of these failed exactly that way after an
    // earlier run had left the files behind owned by another account.
    fn write(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn reads_a_64_bit_interpreter() {
        let p = write("prun-elf64", &synth("/lib64/ld-linux-x86-64.so.2", false));
        let got = read(&p).unwrap();
        assert!(!got.elf32);
        assert!(!got.statik);
        assert_eq!(got.interp.as_deref(), Some("/lib64/ld-linux-x86-64.so.2"));
    }

    #[test]
    fn reads_a_32_bit_interpreter() {
        let p = write("prun-elf32", &synth("/lib/ld-linux.so.2", true));
        let got = read(&p).unwrap();
        assert!(got.elf32);
        assert_eq!(got.interp.as_deref(), Some("/lib/ld-linux.so.2"));
    }

    #[test]
    fn a_file_with_no_program_headers_is_static() {
        let mut v = synth("/x", false);
        v[56..58].copy_from_slice(&0u16.to_le_bytes()); // e_phnum = 0
        let p = write("prun-elfstatic", &v);
        let got = read(&p).unwrap();
        assert!(got.statik);
        assert_eq!(got.interp, None);
    }

    #[test]
    fn a_non_elf_is_an_error_rather_than_a_default() {
        let p = write("prun-notelf", b"#!/bin/sh\necho hi\n");
        assert!(read(&p).is_err());
        assert!(is_script(&p).unwrap());
    }
}

/// PT_LOAD, from the ELF specification.
const PT_LOAD: u32 = 1;
/// PT_DYNAMIC, which carries the relocation table a static-PIE needs applying.
const PT_DYNAMIC: u32 = 2;

/// One mappable segment, in the terms `mmap` wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Where the segment goes, relative to the image's base.
    pub vaddr: u64,
    /// Bytes to take from the file.
    pub filesz: u64,
    /// Bytes the segment occupies once mapped. ⚠ Always at least `filesz`;
    /// the difference is `.bss` and has to read as zero.
    pub memsz: u64,
    /// Where the bytes are in the file.
    pub offset: u64,
    /// `PF_X`, `PF_W`, `PF_R`.
    pub flags: u32,
}

/// What a userland loader needs to map an image and hand it to its entry
/// point.
///
/// ⛔ Everything here is read from a bounded prefix and every field is
/// attacker-controlled until [`LoadPlan::check`] has bounded it.
///
/// ⛔ **It deliberately stops short of `DT_NEEDED`, and that boundary is the
/// point.** Resolving a dependency, binding a symbol or running an initialiser
/// is what `tool/runtime/binary/elfload.c` already does in 1,398 lines of shipped
/// C - and it does it by binding an undefined symbol to the libc the image
/// already links, so a `DT_NEEDED` the process satisfies is answered rather
/// than opened. Growing any of that here is a decision about which loader this
/// project has, not a field added to a struct. `docs/binary/host-dlopen.md` is
/// the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadPlan {
    /// `e_entry`, relative to the base for a PIE.
    pub entry: u64,
    /// `e_phoff`, so the auxiliary vector can point at the mapped headers.
    pub phoff: u64,
    pub phentsize: u16,
    pub phnum: u16,
    /// The mappable segments, in the order the table lists them.
    pub load: Vec<Segment>,
    /// `PT_DYNAMIC`'s virtual address, when there is one.
    pub dynamic: Option<u64>,
    /// The interpreter string, when the image names one.
    pub interp: Option<String>,
    /// True for `ET_DYN`, which is what a position-independent image is.
    pub pie: bool,
}

/// Read the load plan of the ELF64 image at `path`.
///
/// ⛔ ELF64 only, and an ELF32 image is refused by name rather than
/// mis-parsed. The auxiliary vector's entries are pointer-width and the stack
/// this feeds is built in `usize`, so a 32-bit image needs a second layout
/// rather than a cast, and nothing here has one to test against.
pub fn load_plan(path: &Path) -> std::io::Result<LoadPlan> {
    let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, m.to_string());
    let mut f = File::open(path)?;
    let mut ident = [0u8; 64];
    let n = f.read(&mut ident)?;
    if n < 64 || &ident[..4] != b"\x7fELF" {
        return Err(bad("not an ELF file, or its header is short"));
    }
    if ident[4] != 2 {
        return Err(bad("a 32-bit ELF: this loader builds a 64-bit stack and auxiliary vector"));
    }
    let le = ident[5] != 2;
    let e_type = u16(&ident[16..18], le);
    // ET_EXEC is 2 and ET_DYN is 3.
    let pie = e_type == 3;
    let entry = u64f(&ident[24..32], le);
    let phoff = u64f(&ident[32..40], le);
    let phentsize = u16(&ident[54..56], le);
    let phnum = u16(&ident[56..58], le);
    if phnum == 0 || (phentsize as usize) < 56 || phoff == 0 {
        return Err(bad("no usable program header table, so there is nothing to map"));
    }
    let span = phoff.saturating_add(phnum as u64 * phentsize as u64);
    if span > HEADER_LIMIT {
        return Err(bad("program header table is not in the first megabyte"));
    }
    let mut table = vec![0u8; phnum as usize * phentsize as usize];
    f.seek(SeekFrom::Start(phoff))?;
    f.read_exact(&mut table)?;

    let mut load = Vec::new();
    let mut dynamic = None;
    let mut interp = None;
    for i in 0..phnum as usize {
        let e = &table[i * phentsize as usize..(i + 1) * phentsize as usize];
        let ptype = u32(&e[0..4], le);
        let flags = u32(&e[4..8], le);
        let offset = u64f(&e[8..16], le);
        let vaddr = u64f(&e[16..24], le);
        let filesz = u64f(&e[32..40], le);
        let memsz = u64f(&e[40..48], le);
        match ptype {
            PT_LOAD => {
                if memsz < filesz {
                    return Err(bad("a segment whose memory size is under its file size"));
                }
                load.push(Segment { vaddr, filesz, memsz, offset, flags });
            }
            PT_DYNAMIC => dynamic = Some(vaddr),
            PT_INTERP => {
                if filesz > 0 && filesz <= 4096 {
                    let mut buf = vec![0u8; filesz as usize];
                    f.seek(SeekFrom::Start(offset))?;
                    f.read_exact(&mut buf)?;
                    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                    interp = Some(String::from_utf8_lossy(&buf[..end]).into_owned());
                }
            }
            _ => {}
        }
    }
    if load.is_empty() {
        return Err(bad("no PT_LOAD segment, so the image maps to nothing"));
    }
    Ok(LoadPlan { entry, phoff, phentsize, phnum, load, dynamic, interp, pie })
}

impl LoadPlan {
    /// The highest byte the image occupies, which is how much address space
    /// has to be reserved for it.
    pub fn span(&self) -> Option<u64> {
        self.load.iter().try_fold(0u64, |hi, s| {
            let end = s.vaddr.checked_add(s.memsz)?;
            Some(hi.max(end))
        })
    }

    /// Where the program headers land once the image is mapped, when they are
    /// inside a mapped segment.
    ///
    /// ⛔ Derived from the SEGMENTS rather than assumed to be `phoff`. The
    /// auxiliary vector's `AT_PHDR` is an address, and the file offset only
    /// equals the virtual address when a segment happens to map the front of
    /// the file at offset zero. An image whose headers are not mapped at all
    /// has no address to give, and saying so is better than giving a wrong one.
    pub fn phdr_vaddr(&self) -> Option<u64> {
        let end = self.phoff.checked_add(self.phnum as u64 * self.phentsize as u64)?;
        for s in &self.load {
            let seg_end = s.offset.checked_add(s.filesz)?;
            if self.phoff >= s.offset && end <= seg_end {
                return Some(s.vaddr + (self.phoff - s.offset));
            }
        }
        None
    }
}
