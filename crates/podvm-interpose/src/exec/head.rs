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

#[path = "parse.rs"]
mod parse;
pub use parse::*;

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
    /// The kernel runs it as it is: not ELF.
    Direct,
    /// A static ELF of this machine: the native-static rung where podvm
    /// names itself, else the kernel (TODO/enter.md T-1724).
    Static,
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
                .map(|dynamic| if dynamic { File::Dynamic } else { File::Static }),
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
