//! The head of an executable file, read the way the kernel reads it, T-1646.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1646. The exec
//! family must know what the kernel would do with a file before it gives the
//! file to the kernel: a `#!` line names an interpreter, and an ELF file with
//! `PT_INTERP` names a loader. Both names are guest paths. This object takes
//! no dependency (`Cargo.toml`), so it carries its own reader. The parsers
//! here are pure over bytes, and the tests pin them; [`read_head`] opens the
//! file, and [`read_at`] is the one bounded read they take their bytes from.
//!
//! ⛔ No allocation: each buffer is the caller's.

use core::ffi::{c_char, c_int, c_void};

use super::{EACCES, EINVAL, ENOEXEC, O_RDONLY_CLOEXEC};
use crate::map::OUT;

/// The bytes of a file head that the parsers read: the kernel's own
/// `BINPRM_BUF_SIZE`, which bounds a `#!` line. A 64-bit ELF header is 64
/// bytes, so it fits.
pub const HEAD: usize = 256;

/// The size of one 64-bit program header.
pub const PHENT: usize = 56;

/// `PT_INTERP`.
const PT_INTERP: u32 = 3;

/// `EM_X86_64`. The only machine this object is built for (`lib.rs`).
const EM_X86_64: u16 = 62;

/// What the kernel does with a file, as far as the exec family must know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `#!` line.
    Script,
    /// A native 64-bit ELF executable. Its program headers are `phnum`
    /// entries at `phoff`.
    Elf { phoff: u64, phnum: usize },
    /// An ELF file of another class, byte order or machine. The kernel
    /// decides; T-1646 routes only the native machine.
    Foreign,
    /// Neither: the kernel answers `ENOEXEC`, or a `binfmt_misc` handler
    /// takes it.
    Other,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut v = [0u8; 8];
    v.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(v)
}

/// Classify the first bytes of a file. `head` holds the bytes read, no more.
pub fn kind(head: &[u8]) -> Kind {
    if head.len() >= 2 && head[0] == b'#' && head[1] == b'!' {
        return Kind::Script;
    }
    if head.len() < 4 || &head[..4] != b"\x7fELF" {
        return Kind::Other;
    }
    // The header fields this reads: class (4), data (5), type (16),
    // machine (18), phoff (32), phentsize (54), phnum (56).
    if head.len() < 64 || head[4] != 2 || head[5] != 1 || u16_at(head, 18) != EM_X86_64 {
        return Kind::Foreign;
    }
    // ET_EXEC or ET_DYN. Another type is not a program.
    let ty = u16_at(head, 16);
    if (ty != 2 && ty != 3) || u16_at(head, 54) as usize != PHENT {
        return Kind::Other;
    }
    Kind::Elf {
        phoff: u64_at(head, 32),
        phnum: u16_at(head, 56) as usize,
    }
}

/// The `PT_INTERP` entry in a table of program headers: its file offset and
/// its size. `None` is a static executable.
pub fn interp_entry(phdrs: &[u8]) -> Option<(u64, u64)> {
    let mut at = 0usize;
    while at + PHENT <= phdrs.len() {
        if u32_at(phdrs, at) == PT_INTERP {
            return Some((u64_at(phdrs, at + 8), u64_at(phdrs, at + 32)));
        }
        at += PHENT;
    }
    None
}

/// The interpreter name and the optional argument of a `#!` line, as
/// ranges into the head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shebang {
    pub interp: (usize, usize),
    pub arg: Option<(usize, usize)>,
}

/// Parse a `#!` line as `fs/binfmt_script.c` does.
///
/// The line ends at the first newline. Blanks (space and tab) before the
/// name are skipped. The name ends at a blank. The rest of the line, with
/// its outer blanks removed, is ONE argument, or none where it is empty.
/// Where the head holds no newline, the name must end at a blank inside
/// the head, or the kernel refuses with `ENOEXEC`; a long argument is cut
/// at the end of the head, as the kernel cuts it. `None` is that refusal.
/// `whole` tells that the head holds the whole file: the kernel pads a
/// short file with NUL bytes, so its end also ends the name.
pub fn shebang(head: &[u8], whole: bool) -> Option<Shebang> {
    if head.len() < 2 || head[0] != b'#' || head[1] != b'!' {
        return None;
    }
    let blank = |c: u8| c == b' ' || c == b'\t';
    let nl = head.iter().position(|&c| c == b'\n');
    let mut end = nl.unwrap_or(head.len());
    while end > 2 && blank(head[end - 1]) {
        end -= 1;
    }
    let mut at = 2usize;
    while at < end && blank(head[at]) {
        at += 1;
    }
    let start = at;
    while at < end && !blank(head[at]) && head[at] != 0 {
        at += 1;
    }
    if at == start {
        return None;
    }
    if nl.is_none() && at == head.len() && !whole {
        return None;
    }
    let interp = (start, at);
    while at < end && blank(head[at]) {
        at += 1;
    }
    let arg = (at < end).then_some((at, end));
    Some(Shebang { interp, arg })
}

/// Is `name` the file name of a glibc loader older than 2.33, which does
/// not know `--argv0`?
///
/// glibc installed its loader as `ld-2.NN.so` up to 2.33, and as the
/// `PT_INTERP` name itself from 2.34. So the resolved file name tells the
/// version: `ld-2.31.so` is too old, `ld-2.33.so` and
/// `ld-linux-x86-64.so.2` take the option. The musl loader takes it too.
pub fn loader_lacks_argv0(name: &[u8]) -> bool {
    let Some(rest) = name.strip_prefix(b"ld-2.") else {
        return false;
    };
    let Some(digits) = rest.strip_suffix(b".so") else {
        return false;
    };
    if digits.is_empty() || digits.len() > 3 || !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let minor = digits.iter().fold(0u32, |v, d| v * 10 + (d - b'0') as u32);
    minor < 33
}

/// Does the guest loader named `name` (its resolved file) take
/// `--inhibit-cache`, so the host's `/etc/ld.so.cache` plays no part
/// (TODO/enter.md T-1719)? glibc 2.16 and later do: `ld-2.NN.so` with NN of
/// 16 or more, and the `ld-linux*` file of glibc 2.34 and later. musl has
/// no cache and no such option.
pub fn loader_takes_inhibit_cache(name: &[u8]) -> bool {
    if name.starts_with(b"ld-linux") {
        return true;
    }
    let Some(digits) = name
        .strip_prefix(b"ld-2.")
        .and_then(|r| r.strip_suffix(b".so"))
    else {
        return false;
    };
    !digits.is_empty()
        && digits.len() <= 3
        && digits.iter().all(u8::is_ascii_digit)
        && digits.iter().fold(0u32, |v, d| v * 10 + (d - b'0') as u32) >= 16
}

/// The final element of a path.
pub fn basename(p: &[u8]) -> &[u8] {
    match p.iter().rposition(|&c| c == b'/') {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

unsafe extern "C" {
    fn pread(fd: c_int, buf: *mut c_void, count: usize, offset: i64) -> isize;
}

/// Read up to `buf.len()` bytes of `fd` at `off`. Returns the count read,
/// which is short only at the end of the file, or `-1` with `errno` set.
/// Bounded: each turn of the loop reads at least one byte or stops.
///
/// # Safety
/// `fd` must be a descriptor the caller holds open.
pub unsafe fn read_at(fd: c_int, off: u64, buf: &mut [u8]) -> isize {
    let mut got = 0usize;
    while got < buf.len() {
        let Ok(at) = i64::try_from(off + got as u64) else {
            return got as isize;
        };
        let n = unsafe {
            pread(
                fd,
                buf[got..].as_mut_ptr() as *mut c_void,
                buf.len() - got,
                at,
            )
        };
        if n < 0 {
            if crate::errno() == EINTR {
                continue;
            }
            return -1;
        }
        if n == 0 {
            break;
        }
        got += n as usize;
    }
    got as isize
}

const EINTR: c_int = 4;

/// What the kernel does with one file.
pub enum File {
    /// A `#!` line, in the first `usize` bytes of the head.
    Script(usize),
    /// An ELF file with `PT_INTERP`; the name is in the `phdrs` buffer.
    Dynamic,
    /// The kernel runs it as it is: static, or not ELF.
    Direct,
    /// An ELF file of another class, byte order or machine (T-1703).
    Foreign,
}

/// Read the head of `host` and say what the kernel does with it.
///
/// # Safety
/// `host` must be NUL-terminated.
pub unsafe fn read_head(
    host: *const c_char,
    head: &mut [u8; HEAD],
    phdrs: &mut [u8; OUT],
) -> Result<File, c_int> {
    let (Some(open), Some(close)) = (crate::next_open(), crate::next_close()) else {
        return Err(EINVAL);
    };
    let fd = unsafe { open(host, O_RDONLY_CLOEXEC, 0) };
    if fd < 0 {
        return Err(crate::errno());
    }
    let got = unsafe { read_at(fd, 0, &mut head[..HEAD - 1]) };
    let r = if got < 0 {
        // A directory reads `EISDIR`; the kernel answers its exec `EACCES`.
        Err(EACCES)
    } else {
        let got = got as usize;
        match kind(&head[..got]) {
            Kind::Script => Ok(File::Script(got)),
            Kind::Elf { phoff, phnum } => unsafe { read_interp(fd, phoff, phnum, phdrs) }
                .map(|dynamic| if dynamic { File::Dynamic } else { File::Direct }),
            Kind::Foreign => Ok(File::Foreign),
            Kind::Other => Ok(File::Direct),
        }
    };
    unsafe { close(fd) };
    r
}

/// Read the `PT_INTERP` name into `out` with its NUL. `Ok(false)` is a
/// static executable. A table or a name that does not read whole is
/// `ENOEXEC`, as the kernel answers it.
unsafe fn read_interp(
    fd: c_int,
    phoff: u64,
    phnum: usize,
    out: &mut [u8; OUT],
) -> Result<bool, c_int> {
    let size = phnum
        .checked_mul(PHENT)
        .filter(|&n| n > 0 && n <= OUT)
        .ok_or(ENOEXEC)?;
    if unsafe { read_at(fd, phoff, &mut out[..size]) } != size as isize {
        return Err(ENOEXEC);
    }
    let Some((off, len)) = interp_entry(&out[..size]) else {
        return Ok(false);
    };
    let len = usize::try_from(len)
        .ok()
        .filter(|&n| (2..OUT).contains(&n))
        .ok_or(ENOEXEC)?;
    if unsafe { read_at(fd, off, &mut out[..len]) } != len as isize || out[len - 1] != 0 {
        return Err(ENOEXEC);
    }
    Ok(true)
}
