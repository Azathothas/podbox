//! `execve` of the payload: podvm executes itself again, so the kernel
//! keeps each rule of an exec (the threads end, `O_CLOEXEC` descriptors
//! close, caught signals reset), and the new podvm loads the new program.
//!
//! The argv stays the payload's. Two variables carry the rest:
//! [`ROOT_VAR`] (the host path of the root) and [`EXE_VAR`] (the guest path
//! of the program). The working directory is the process's own.

use crate::path::{self, Target};
use crate::state::{EXE, PATH_MAX, ROOT, SELF};
use crate::sys::{self, nr, sc, sc6, AT_EMPTY_PATH, AT_FDCWD};

pub const ROOT_VAR: &[u8] = b"PODVM_DISPATCH_ROOT=";
pub const EXE_VAR: &[u8] = b"PODVM_DISPATCH_EXE=";
const X_OK: i64 = 1;
const MAP_PRIVATE_ANON: i64 = 0x22;

/// The guest path of the program into `buf`, and its length.
fn program(dirfd: i64, p: i64, flags: i64, buf: &mut [u8; PATH_MAX]) -> Result<usize, i64> {
    if p == 0 {
        return Err(-sys::EFAULT);
    }
    // SAFETY: the payload gave p as a path.
    let path = unsafe { sys::cstr(p, PATH_MAX) }.ok_or(-sys::ENAMETOOLONG)?;
    if path.is_empty() && flags & AT_EMPTY_PATH == 0 {
        return Err(-sys::ENOENT);
    }
    match path::target(dirfd, path, buf)? {
        Target::Root(n) => Ok(n),
        Target::HostAbs(n) if crate::procself::is_exe(&buf[..n]) => {
            let e = EXE.get();
            buf[..e.len()].copy_from_slice(e);
            buf[e.len()] = 0;
            Ok(e.len())
        }
        // A program outside the root would run with no dispatch.
        _ => Err(-sys::EACCES),
    }
}

/// Check the program as the kernel would: it exists, it is executable, and
/// it is a static ELF of x86_64 or a script.
fn check(buf: &[u8; PATH_MAX], n: usize) -> i64 {
    let t = Target::Root(n);
    let fd = path::open_path(AT_FDCWD, &[], t, &buf[..], true, false);
    if fd < 0 {
        return fd;
    }
    let mut link = [0u8; 32];
    let pp = path::fd_link(fd, &mut link);
    // SAFETY: the proc path lives for the call.
    let r = unsafe { sc6(nr::FACCESSAT2, AT_FDCWD, pp.as_ptr() as i64, X_OK, 0, 0, 0) };
    if r < 0 {
        sys::close(fd);
        return r;
    }
    let rd = sys::openat2(AT_FDCWD, pp, sys::O_RDONLY | sys::O_CLOEXEC, 0, 0);
    sys::close(fd);
    if rd < 0 {
        return rd;
    }
    let mut head = [0u8; 64];
    // SAFETY: head lives for the call.
    let got = unsafe { sc6(nr::PREAD64, rd, head.as_mut_ptr() as i64, 64, 0, 0, 0) };
    let mut verdict = -sys::ENOEXEC;
    if got >= 2 && head[0] == b'#' && head[1] == b'!' {
        verdict = 0;
    } else if got >= 64 {
        verdict = match crate::elf::static_x86_64(&head, |off, out| {
            // SAFETY: out lives for the call.
            unsafe {
                sc6(
                    nr::PREAD64,
                    rd,
                    out.as_mut_ptr() as i64,
                    out.len() as i64,
                    off as i64,
                    0,
                    0,
                )
            }
        }) {
            Ok(()) => 0,
            Err(_) => -sys::ENOEXEC,
        };
    }
    sys::close(rd);
    verdict
}

/// The payload's execveat(2).
pub fn execveat(dirfd: i64, p: i64, argv: i64, envp: i64, flags: i64) -> i64 {
    let Some(mut g) = crate::pool::Buf::get() else {
        return -sys::ENOMEM;
    };
    let buf = g.buf();
    let n = match program(dirfd, p, flags, buf) {
        Ok(n) => n,
        Err(e) => return e,
    };
    let c = check(buf, n);
    if c < 0 {
        return c;
    }
    // The new environment: the payload's, less our two names, and ours.
    let mut count = 0usize;
    if envp != 0 {
        // SAFETY: the payload gave envp as a NULL-ended array.
        while unsafe { *(envp as *const i64).add(count) } != 0 {
            count += 1;
        }
    }
    let root = ROOT.get();
    let strings = ROOT_VAR.len() + root.len() + 1 + EXE_VAR.len() + n + 1;
    let size = ((count + 3) * 8 + strings + 4095) & !4095;
    // SAFETY: an anonymous map for the new array; it ends with the exec.
    let mem = unsafe { sc6(nr::MMAP, 0, size as i64, 3, MAP_PRIVATE_ANON, -1, 0) };
    if mem < 0 {
        return -sys::ENOMEM;
    }
    // SAFETY: mem holds size bytes; each pointer written lies in it.
    unsafe {
        let arr = mem as *mut i64;
        let mut k = 0usize;
        for i in 0..count {
            let e = *(envp as *const i64).add(i);
            let s = sys::cstr(e, 1 << 20).unwrap_or(&[]);
            if s.starts_with(ROOT_VAR) || s.starts_with(EXE_VAR) {
                continue;
            }
            *arr.add(k) = e;
            k += 1;
        }
        let mut at = mem as *mut u8;
        at = at.add((count + 3) * 8);
        for (name, value) in [(ROOT_VAR, root), (EXE_VAR, &buf[..n])] {
            *arr.add(k) = at as i64;
            k += 1;
            core::ptr::copy_nonoverlapping(name.as_ptr(), at, name.len());
            at = at.add(name.len());
            core::ptr::copy_nonoverlapping(value.as_ptr(), at, value.len());
            at = at.add(value.len());
            *at = 0;
            at = at.add(1);
        }
        *arr.add(k) = 0;
        let r = sc(nr::EXECVE, SELF.get_z().as_ptr() as i64, argv, mem);
        sc(nr::MUNMAP, mem, size as i64, 0);
        r
    }
}
