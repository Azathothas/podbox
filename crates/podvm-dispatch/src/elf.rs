//! The ELF of a static x86_64 program: the check, and the map of its
//! segments into podvm's own address space.

use crate::sys::{self, nr, sc6};

const PT_LOAD: u32 = 1;
const PT_INTERP: u32 = 3;
const PT_PHDR: u32 = 6;
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;
const MAP_PRIVATE: i64 = 0x02;
const MAP_FIXED: i64 = 0x10;
const MAP_ANON: i64 = 0x20;
const MAP_FIXED_NOREPLACE: i64 = 0x10_0000;
const PAGE: u64 = 4096;

fn u16_at(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
fn u64_at(b: &[u8], o: usize) -> u64 {
    b.get(o..o + 8).map_or(0, |s| {
        u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
    })
}

/// One program header.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ph {
    pub kind: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
}

/// The header of the file (64 bytes) and a reader of the file at an
/// offset: Ok where the file is a static ELF of x86_64 (no PT_INTERP).
/// This runs in the handler too: it uses no allocator.
pub fn static_x86_64(
    head: &[u8],
    mut read: impl FnMut(u64, &mut [u8]) -> i64,
) -> Result<(), &'static str> {
    if head.get(..4) != Some(b"\x7fELF".as_slice())
        || head.get(4) != Some(&2)
        || head.get(5) != Some(&1)
    {
        return Err("not an ELF of 64 bits, little-endian");
    }
    if u16_at(head, 18) != EM_X86_64 {
        return Err("not an ELF of x86_64");
    }
    let kind = u16_at(head, 16);
    if kind != ET_EXEC && kind != ET_DYN {
        return Err("not an executable ELF");
    }
    let (off, ent, num) = (
        u64_at(head, 32),
        u16_at(head, 54) as u64,
        u16_at(head, 56) as u64,
    );
    if ent != 56 || num == 0 || num > 64 {
        return Err("program headers of a size this loader does not take");
    }
    let mut ph = [0u8; 56];
    for i in 0..num {
        if read(off + i * ent, &mut ph) != 56 {
            return Err("a program header cannot be read");
        }
        if u32_at(&ph, 0) == PT_INTERP {
            return Err("a dynamic program (it names an interpreter)");
        }
    }
    Ok(())
}

/// The headers of a file, read with `read`.
pub fn headers(
    head: &[u8],
    mut read: impl FnMut(u64, &mut [u8]) -> i64,
) -> Result<Vec<Ph>, &'static str> {
    static_x86_64(head, &mut read)?;
    let (off, num) = (u64_at(head, 32), u16_at(head, 56) as u64);
    let mut out = Vec::new();
    let mut b = [0u8; 56];
    for i in 0..num {
        read(off + i * 56, &mut b);
        out.push(Ph {
            kind: u32_at(&b, 0),
            flags: u32_at(&b, 4),
            offset: u64_at(&b, 8),
            vaddr: u64_at(&b, 16),
            filesz: u64_at(&b, 32),
            memsz: u64_at(&b, 40),
        });
    }
    Ok(out)
}

/// A program in memory.
#[derive(Debug)]
pub struct Image {
    pub entry: u64,
    pub phdr: u64,
    pub phnum: u64,
    /// The end of its highest segment, page-aligned: the break starts here.
    pub end: u64,
}

fn prot(flags: u32) -> i64 {
    (if flags & 4 != 0 { 1 } else { 0 })
        | (if flags & 2 != 0 { 2 } else { 0 })
        | (if flags & 1 != 0 { 4 } else { 0 })
}

fn mmap(addr: u64, len: u64, prot: i64, flags: i64, fd: i64, off: u64) -> Result<u64, String> {
    // SAFETY: a map at a range that this loader reserved, or anywhere.
    let r = unsafe {
        sc6(
            nr::MMAP,
            addr as i64,
            len as i64,
            prot,
            flags,
            fd,
            off as i64,
        )
    };
    if r < 0 {
        Err(format!("mmap of {len} bytes at {addr:#x}: errno {}", -r))
    } else {
        Ok(r as u64)
    }
}

/// Map the program of `fd` (whose first 64 bytes are `head`).
pub fn load(fd: i64, head: &[u8]) -> Result<Image, String> {
    let pread = |off: u64, out: &mut [u8]| {
        // SAFETY: out lives for the call.
        unsafe {
            sc6(
                nr::PREAD64,
                fd,
                out.as_mut_ptr() as i64,
                out.len() as i64,
                off as i64,
                0,
                0,
            )
        }
    };
    let phs = headers(head, pread).map_err(str::to_string)?;
    let loads: Vec<&Ph> = phs.iter().filter(|p| p.kind == PT_LOAD).collect();
    if loads.is_empty() {
        return Err("no PT_LOAD segment".into());
    }
    let lo = loads
        .iter()
        .map(|p| p.vaddr & !(PAGE - 1))
        .min()
        .unwrap_or(0);
    let hi = loads
        .iter()
        .map(|p| (p.vaddr + p.memsz + PAGE - 1) & !(PAGE - 1))
        .max()
        .unwrap_or(0);
    let fixed = u16_at(head, 16) == ET_EXEC;
    let base = if fixed {
        let at = mmap(
            lo,
            hi - lo,
            0,
            MAP_PRIVATE | MAP_ANON | MAP_FIXED_NOREPLACE,
            -1,
            0,
        )?;
        if at != lo {
            return Err(format!(
                "the program's range {lo:#x}..{hi:#x} is taken in podvm's address space"
            ));
        }
        lo
    } else {
        mmap(0, hi - lo, 0, MAP_PRIVATE | MAP_ANON, -1, 0)?
    };
    let bias = base - lo;
    for p in &loads {
        let start = (p.vaddr & !(PAGE - 1)) + bias;
        let file_end = p.vaddr + p.filesz + bias;
        let mem_end = (p.vaddr + p.memsz + bias + PAGE - 1) & !(PAGE - 1);
        let pr = prot(p.flags);
        if p.filesz > 0 {
            let len = file_end - start;
            mmap(
                start,
                len,
                pr | 2,
                MAP_PRIVATE | MAP_FIXED,
                fd,
                p.offset & !(PAGE - 1),
            )?;
        }
        let page_end = (file_end + PAGE - 1) & !(PAGE - 1);
        if p.memsz > p.filesz {
            if p.filesz > 0 && page_end > file_end {
                // SAFETY: the tail of the page that the file map holds, mapped writable above.
                unsafe {
                    core::ptr::write_bytes(file_end as *mut u8, 0, (page_end - file_end) as usize)
                };
            }
            if mem_end > page_end {
                mmap(
                    page_end,
                    mem_end - page_end,
                    pr,
                    MAP_PRIVATE | MAP_ANON | MAP_FIXED,
                    -1,
                    0,
                )?;
            }
        }
        // SAFETY: the range was mapped above.
        unsafe {
            sc6(
                nr::MPROTECT,
                start as i64,
                (page_end.max(start + PAGE) - start) as i64,
                pr,
                0,
                0,
                0,
            )
        };
    }
    let phoff = u64_at(head, 32);
    let phdr = phs
        .iter()
        .find(|p| p.kind == PT_PHDR)
        .map(|p| p.vaddr + bias)
        .or_else(|| {
            loads
                .iter()
                .find(|p| p.offset <= phoff && phoff < p.offset + p.filesz)
                .map(|p| p.vaddr + (phoff - p.offset) + bias)
        })
        .ok_or("the program headers are in no segment")?;
    let _ = sys::EINVAL;
    Ok(Image {
        entry: u64_at(head, 24) + bias,
        phdr,
        phnum: phs.len() as u64,
        end: hi + bias,
    })
}

/// `#!INTERP [ARG]` of a script, or None.
pub fn shebang(head: &[u8]) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
    let rest = head.strip_prefix(b"#!")?;
    let line = rest.split(|&c| c == b'\n').next()?;
    let line: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|c| *c == b' ' || *c == b'\t')
        .collect();
    let cut = line
        .iter()
        .position(|c| *c == b' ' || *c == b'\t')
        .unwrap_or(line.len());
    let interp = line[..cut].to_vec();
    let arg: Vec<u8> = line[cut..]
        .iter()
        .copied()
        .skip_while(|c| *c == b' ' || *c == b'\t')
        .collect();
    let arg: Vec<u8> = arg
        .iter()
        .copied()
        .rev()
        .skip_while(|c| *c == b' ' || *c == b'\t' || *c == b'\r')
        .collect::<Vec<u8>>()
        .into_iter()
        .rev()
        .collect();
    if interp.is_empty() {
        return None;
    }
    Some((interp, (!arg.is_empty()).then_some(arg)))
}
