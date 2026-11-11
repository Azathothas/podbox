//! The calls of the payload that carry a path. [`path_call`] gives the
//! result of a call it serves, or `None` for a call that runs as the
//! payload gave it.
//!
//! A call that does not follow its last link works on the parent's
//! descriptor and the last name. A call that follows works on an `O_PATH`
//! descriptor of the object: with `AT_EMPTY_PATH` where the call takes it,
//! else through `/proc/self/fd/N`.

use crate::path::{self, fd_link, parent, target, Parent, Target};
use crate::state::{EXE, PATH_MAX};
use crate::sys::{self, nr, sc, sc6, AT_EMPTY_PATH, AT_FDCWD, AT_REMOVEDIR, AT_SYMLINK_NOFOLLOW};

const AT_SYMLINK_FOLLOW: i64 = 0x400;
const IN_DONT_FOLLOW: i64 = 0x0200_0000;
/// The flags that openat2(2) takes (`VALID_OPEN_FLAGS` of the kernel).
const VALID_OPEN: i64 = 0o37777703;

fn arg(p: i64) -> Result<&'static [u8], i64> {
    if p == 0 {
        return Err(-sys::EFAULT);
    }
    // SAFETY: the payload gave p as a path; see `sys::cstr`.
    unsafe { sys::cstr(p, PATH_MAX) }.ok_or(-sys::ENAMETOOLONG)
}

/// `/proc/self/fd/N` of an `O_PATH` descriptor of the object.
struct Obj {
    fd: i64,
    link: [u8; 32],
}

impl Obj {
    fn proc(&mut self) -> i64 {
        let fd = self.fd;
        fd_link(fd, &mut self.link).as_ptr() as i64
    }
}

impl Drop for Obj {
    fn drop(&mut self) {
        sys::close(self.fd);
    }
}

/// The object of `(dirfd, path)`, or `Ok(None)` where the call keeps the
/// host's meaning.
fn object(dirfd: i64, p: i64, follow: bool) -> Result<Option<Obj>, i64> {
    let path = arg(p)?;
    let mut __g1 = crate::pool::Buf::get().ok_or(-sys::ENOMEM)?;
    let buf = __g1.buf();
    let t = target(dirfd, path, buf)?;
    if t == Target::Host {
        return Ok(None);
    }
    let fd = path::open_path(dirfd, path, t, &buf[..], follow, false);
    if fd < 0 {
        Err(fd)
    } else {
        Ok(Some(Obj { fd, link: [0; 32] }))
    }
}

/// The parent and the last name of `(dirfd, path)`: the descriptor and a
/// pointer to the name, or the payload's own dirfd and path for a host
/// target. `name` holds the name.
fn split(dirfd: i64, p: i64, name: &mut [u8; 256]) -> Result<(Parent, i64), i64> {
    let path = arg(p)?;
    let mut __g2 = crate::pool::Buf::get().ok_or(-sys::ENOMEM)?;
    let buf = __g2.buf();
    let t = target(dirfd, path, buf)?;
    if t == Target::Host {
        return Ok((
            Parent {
                fd: dirfd,
                owned: false,
            },
            p,
        ));
    }
    let par = parent(t, buf, name)?;
    Ok((par, name.as_ptr() as i64))
}

/// Run `f` on the result of `r`, or give the error.
fn or_err<T>(r: Result<T, i64>, f: impl FnOnce(T) -> i64) -> i64 {
    match r {
        Ok(v) => f(v),
        Err(e) => e,
    }
}

/// A call on the parent and the name: `n(pfd, name, a, b, c)`.
fn on_parent(n: i64, dirfd: i64, p: i64, a: i64, b: i64, c: i64) -> i64 {
    let mut name = [0u8; 256];
    or_err(split(dirfd, p, &mut name), |(par, np)| {
        // SAFETY: the name and the payload's arguments live for the call.
        unsafe { sc6(n, par.fd, np, a, b, c, 0) }
    })
}

/// A call on the object through its `/proc/self/fd/N` path:
/// `n(proc, a, b, c, d)`, or with `at` first `n(AT_FDCWD, proc, a, b, c)`.
fn on_proc(n: i64, at: bool, dirfd: i64, p: i64, follow: bool, a: [i64; 4]) -> i64 {
    match object(dirfd, p, follow) {
        Err(e) => e,
        Ok(None) => pass_through(n, at, dirfd, p, a),
        // SAFETY: the proc path and the payload's arguments live for the call.
        Ok(Some(mut o)) => unsafe {
            let pp = o.proc();
            if at {
                sc6(n, AT_FDCWD, pp, a[0], a[1], a[2], a[3])
            } else {
                sc6(n, pp, a[0], a[1], a[2], a[3], 0)
            }
        },
    }
}

fn pass_through(n: i64, at: bool, dirfd: i64, p: i64, a: [i64; 4]) -> i64 {
    // SAFETY: the payload's own call.
    unsafe {
        if at {
            sc6(n, dirfd, p, a[0], a[1], a[2], a[3])
        } else {
            sc6(n, p, a[0], a[1], a[2], a[3], 0)
        }
    }
}

/// A call with `AT_EMPTY_PATH` on the object: `n(fd, "", a, b, c|EMPTY)`
/// where `flag_at` names the argument that holds the flags.
fn on_empty(n: i64, dirfd: i64, p: i64, flags: i64, a: [i64; 4], flag_at: usize) -> i64 {
    let follow = flags & AT_SYMLINK_NOFOLLOW == 0;
    match object(dirfd, p, follow) {
        Err(e) => e,
        Ok(None) => pass_through(n, true, dirfd, p, a),
        Ok(Some(o)) => {
            let mut a = a;
            a[flag_at] = (flags & !AT_SYMLINK_NOFOLLOW) | AT_EMPTY_PATH;
            // SAFETY: "" and the payload's arguments live for the call.
            unsafe { sc6(n, o.fd, c"".as_ptr() as i64, a[0], a[1], a[2], a[3]) }
        }
    }
}

fn empty_path(p: i64, flags: i64) -> bool {
    p == 0 || (flags & AT_EMPTY_PATH != 0 && matches!(arg(p), Ok(b) if b.is_empty()))
}

/// `openat` and its kin.
fn open(dirfd: i64, p: i64, flags: i64, mode: i64) -> i64 {
    let path = match arg(p) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(mut __g3) = crate::pool::Buf::get() else {
        return -sys::ENOMEM;
    };
    let buf = __g3.buf();
    let given = mode;
    let mode = if flags & (sys::O_CREAT | sys::O_TMPFILE) != 0 {
        mode & 0o7777
    } else {
        0
    };
    match target(dirfd, path, buf) {
        Err(e) => e,
        // SAFETY: the payload's own call.
        Ok(Target::Host) => unsafe { sc6(nr::OPENAT, dirfd, p, flags, given, 0, 0) },
        // SAFETY: buf and the arguments live for the call.
        Ok(Target::HostAbs(_)) => unsafe {
            sc6(nr::OPENAT, AT_FDCWD, buf.as_ptr() as i64, flags, mode, 0, 0)
        },
        Ok(Target::Root(n)) => {
            let mut f = flags & VALID_OPEN;
            if f & sys::O_PATH != 0 {
                f &= sys::O_PATH | sys::O_DIRECTORY | sys::O_NOFOLLOW | sys::O_CLOEXEC;
            }
            let fd = path::open_in(buf, n, f, mode, 0);
            // T-1657: a descriptor of the sized `/tmp`.
            if fd >= 0 && crate::tier::on() && crate::tier::under_tmp(&buf[..n]) {
                crate::tier::mark(fd, true);
            }
            fd
        }
    }
}

/// T-1657: `unlinkat` that gives back the bytes of a file under the sized
/// `/tmp`.
fn unlink(dirfd: i64, p: i64, flags: i64) -> i64 {
    let mut freed = 0;
    if crate::tier::on()
        && flags & AT_REMOVEDIR == 0
        && let (Ok(path), Some(mut g)) = (arg(p), crate::pool::Buf::get())
    {
        let buf = g.buf();
        if let Ok(Target::Root(n)) = target(dirfd, path, buf)
            && crate::tier::under_tmp(&buf[..n])
        {
            let fd = path::open_in(buf, n, sys::O_PATH | sys::O_NOFOLLOW | sys::O_CLOEXEC, 0, 0);
            if fd >= 0 {
                freed = crate::tier::freed_by_unlink(fd);
                sys::close(fd);
            }
        }
    }
    let r = on_parent(nr::UNLINKAT, dirfd, p, flags, 0, 0);
    if r == 0 && freed > 0 {
        crate::tier::unlinked(freed);
    }
    r
}

/// `readlinkat`, with the answers of `/proc/self` in guest paths.
fn readlink(dirfd: i64, p: i64, out: i64, size: i64) -> i64 {
    let path = match arg(p) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(mut __g4) = crate::pool::Buf::get() else {
        return -sys::ENOMEM;
    };
    let buf = __g4.buf();
    let t = match target(dirfd, path, buf) {
        Ok(t) => t,
        Err(e) => return e,
    };
    match t {
        // SAFETY: the payload's own call.
        Target::Host => unsafe { sc6(nr::READLINKAT, dirfd, p, out, size, 0, 0) },
        Target::Root(_) => on_parent(nr::READLINKAT, dirfd, p, out, size, 0),
        Target::HostAbs(n) => {
            let g = &buf[..n];
            if crate::procself::is_exe(g) {
                return copy_out(EXE.get(), out, size, false);
            }
            let Some(mut __g5) = crate::pool::Buf::get() else {
                return -sys::ENOMEM;
            };
            let host = __g5.buf();
            let r = sys::readlinkat(AT_FDCWD, &buf[..=n], host);
            if r < 0 {
                return r;
            }
            let h = &host[..r as usize];
            if crate::procself::is_self_link(g) {
                let Some(mut gb) = crate::pool::Buf::get() else {
                    return -sys::ENOMEM;
                };
                let gbuf = gb.buf();
                if let Some(gn) = path::guest_of_host(h, &mut gbuf[..]) {
                    return copy_out(&gbuf[..gn], out, size, false);
                }
            }
            copy_out(h, out, size, false)
        }
    }
}

/// Copy `v` to the payload's buffer: readlink cuts at `size`; getcwd adds a
/// NUL and gives ERANGE.
fn copy_out(v: &[u8], out: i64, size: i64, nul: bool) -> i64 {
    let need = v.len() + nul as usize;
    if size < 0 || (nul && need > size as usize) {
        return -sys::ERANGE;
    }
    let n = v.len().min(size as usize);
    // SAFETY: the payload gave `out` with `size` bytes.
    unsafe {
        core::ptr::copy_nonoverlapping(v.as_ptr(), out as *mut u8, n);
        if nul {
            *(out as *mut u8).add(n) = 0;
        }
    }
    (n + nul as usize) as i64
}

fn getcwd(out: i64, size: i64) -> i64 {
    let Some(mut __g6) = crate::pool::Buf::get() else {
        return -sys::ENOMEM;
    };
    let host = __g6.buf();
    let r = sys::readlinkat(AT_FDCWD, b"/proc/self/cwd\0", host);
    if r < 0 {
        return r;
    }
    let h = &host[..r as usize];
    let Some(mut gb) = crate::pool::Buf::get() else {
        return -sys::ENOMEM;
    };
    let gbuf = gb.buf();
    match path::guest_of_host(h, &mut gbuf[..]) {
        Some(gn) => copy_out(&gbuf[..gn], out, size, true),
        None => copy_out(h, out, size, true),
    }
}

fn chdir(p: i64) -> i64 {
    match object(AT_FDCWD, p, true) {
        Err(e) => e,
        // SAFETY: the payload's own call.
        Ok(None) => unsafe { sc(nr::CHDIR, p, 0, 0) },
        // SAFETY: fchdir reads no memory.
        Ok(Some(o)) => unsafe { sc(81, o.fd, 0, 0) },
    }
}

/// renameat2 and linkat: two paths.
fn two(n: i64, od: i64, op: i64, nd: i64, np: i64, flags: i64) -> i64 {
    let mut name_a = [0u8; 256];
    let mut name_b = [0u8; 256];
    if n == nr::LINKAT && flags & AT_SYMLINK_FOLLOW != 0 {
        let mut o = match object(od, op, true) {
            Err(e) => return e,
            Ok(None) => return unsafe { sc6(n, od, op, nd, np, flags, 0) },
            Ok(Some(o)) => o,
        };
        let pp = o.proc();
        return or_err(split(nd, np, &mut name_b), |(b, bn)| {
            // SAFETY: the names and the proc path live for the call.
            unsafe { sc6(n, AT_FDCWD, pp, b.fd, bn, flags, 0) }
        });
    }
    let a = match split(od, op, &mut name_a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    or_err(split(nd, np, &mut name_b), |(b, bn)| {
        // SAFETY: the names live for the call.
        unsafe { sc6(n, a.0.fd, a.1, b.fd, bn, flags, 0) }
    })
}

/// The result of a call that carries a path, or `None` for any other call.
pub fn path_call(n: i64, a: [i64; 6]) -> Option<i64> {
    let f = AT_FDCWD;
    Some(match n {
        nr::OPEN => open(f, a[0], a[1], a[2]),
        nr::CREAT => open(f, a[0], 0o1101, a[1]),
        nr::OPENAT => open(a[0], a[1], a[2], a[3]),
        nr::STAT => on_empty(nr::NEWFSTATAT, f, a[0], 0, [a[1], 0, 0, 0], 1),
        nr::LSTAT => on_empty(
            nr::NEWFSTATAT,
            f,
            a[0],
            AT_SYMLINK_NOFOLLOW,
            [a[1], 0, 0, 0],
            1,
        ),
        nr::NEWFSTATAT if empty_path(a[1], a[3]) => return None,
        nr::NEWFSTATAT if a[3] & AT_SYMLINK_NOFOLLOW != 0 => {
            on_parent(n, a[0], a[1], a[2], a[3], 0)
        }
        nr::NEWFSTATAT => on_empty(n, a[0], a[1], a[3], [a[2], a[3], 0, 0], 1),
        nr::STATX if empty_path(a[1], a[2]) => return None,
        nr::STATX if a[2] & AT_SYMLINK_NOFOLLOW != 0 => on_parent(n, a[0], a[1], a[2], a[3], a[4]),
        nr::STATX => on_empty(n, a[0], a[1], a[2], [a[2], a[3], a[4], 0], 0),
        nr::ACCESS => on_proc(nr::FACCESSAT2, true, f, a[0], true, [a[1], 0, 0, 0]),
        nr::FACCESSAT => on_proc(nr::FACCESSAT2, true, a[0], a[1], true, [a[2], 0, 0, 0]),
        nr::FACCESSAT2 if empty_path(a[1], a[3]) => return None,
        nr::FACCESSAT2 if a[3] & AT_SYMLINK_NOFOLLOW != 0 => {
            on_parent(n, a[0], a[1], a[2], a[3], 0)
        }
        nr::FACCESSAT2 => on_proc(n, true, a[0], a[1], true, [a[2], a[3], 0, 0]),
        nr::READLINK => readlink(f, a[0], a[1], a[2]),
        nr::READLINKAT => readlink(a[0], a[1], a[2], a[3]),
        nr::GETCWD => getcwd(a[0], a[1]),
        nr::CHDIR => chdir(a[0]),
        nr::MKDIR => on_parent(nr::MKDIRAT, f, a[0], a[1], 0, 0),
        nr::MKDIRAT => on_parent(n, a[0], a[1], a[2], 0, 0),
        nr::MKNOD => on_parent(nr::MKNODAT, f, a[0], a[1], a[2], 0),
        nr::MKNODAT => on_parent(n, a[0], a[1], a[2], a[3], 0),
        nr::UNLINK => unlink(f, a[0], 0),
        nr::RMDIR => on_parent(nr::UNLINKAT, f, a[0], AT_REMOVEDIR, 0, 0),
        nr::UNLINKAT => unlink(a[0], a[1], a[2]),
        nr::SYMLINK => symlink(a[0], f, a[1]),
        nr::SYMLINKAT => symlink(a[0], a[1], a[2]),
        nr::RENAME => two(nr::RENAMEAT2, f, a[0], f, a[1], 0),
        nr::RENAMEAT => two(nr::RENAMEAT2, a[0], a[1], a[2], a[3], 0),
        nr::RENAMEAT2 => two(n, a[0], a[1], a[2], a[3], a[4]),
        nr::LINK => two(nr::LINKAT, f, a[0], f, a[1], 0),
        nr::LINKAT => two(n, a[0], a[1], a[2], a[3], a[4]),
        nr::CHMOD => on_proc(nr::FCHMODAT, true, f, a[0], true, [a[1], 0, 0, 0]),
        nr::FCHMODAT => on_proc(n, true, a[0], a[1], true, [a[2], 0, 0, 0]),
        nr::FCHMODAT2 if a[3] & AT_SYMLINK_NOFOLLOW != 0 => on_parent(n, a[0], a[1], a[2], a[3], 0),
        nr::FCHMODAT2 => on_proc(nr::FCHMODAT, true, a[0], a[1], true, [a[2], 0, 0, 0]),
        nr::CHOWN => on_empty(nr::FCHOWNAT, f, a[0], 0, [a[1], a[2], 0, 0], 2),
        nr::LCHOWN => on_empty(
            nr::FCHOWNAT,
            f,
            a[0],
            AT_SYMLINK_NOFOLLOW,
            [a[1], a[2], 0, 0],
            2,
        ),
        nr::FCHOWNAT if empty_path(a[1], a[4]) => return None,
        nr::FCHOWNAT if a[4] & AT_SYMLINK_NOFOLLOW != 0 => {
            on_parent(n, a[0], a[1], a[2], a[3], a[4])
        }
        nr::FCHOWNAT => on_empty(n, a[0], a[1], a[4], [a[2], a[3], a[4], 0], 2),
        nr::TRUNCATE | nr::UTIME | nr::UTIMES | nr::STATFS => {
            on_proc(n, false, f, a[0], true, [a[1], 0, 0, 0])
        }
        nr::FUTIMESAT if a[1] == 0 => return None,
        nr::FUTIMESAT => on_proc(n, true, a[0], a[1], true, [a[2], 0, 0, 0]),
        nr::UTIMENSAT if a[1] == 0 => return None,
        nr::UTIMENSAT if a[3] & AT_SYMLINK_NOFOLLOW != 0 => on_parent(n, a[0], a[1], a[2], a[3], 0),
        nr::UTIMENSAT => on_proc(
            n,
            true,
            a[0],
            a[1],
            true,
            [a[2], a[3] & !AT_EMPTY_PATH, 0, 0],
        ),
        nr::SETXATTR | nr::GETXATTR | nr::LISTXATTR | nr::REMOVEXATTR => {
            on_proc(n, false, f, a[0], true, [a[1], a[2], a[3], a[4]])
        }
        nr::LSETXATTR | nr::LGETXATTR | nr::LLISTXATTR | nr::LREMOVEXATTR => {
            on_proc(n, false, f, a[0], false, [a[1], a[2], a[3], a[4]])
        }
        nr::INOTIFY_ADD_WATCH => {
            let fd = a[0];
            match object(f, a[1], a[2] & IN_DONT_FOLLOW == 0) {
                Err(e) => e,
                Ok(None) => return None,
                // SAFETY: the proc path lives for the call.
                Ok(Some(mut o)) => unsafe { sc(n, fd, o.proc(), a[2]) },
            }
        }
        nr::CHROOT | nr::MOUNT | nr::UMOUNT2 | nr::PIVOT_ROOT => -sys::EPERM,
        _ => return None,
    })
}

fn symlink(target_p: i64, dirfd: i64, p: i64) -> i64 {
    let mut name = [0u8; 256];
    or_err(split(dirfd, p, &mut name), |(par, np)| {
        // SAFETY: the target string is the payload's; the name lives here.
        unsafe { sc(nr::SYMLINKAT, target_p, par.fd, np) }
    })
}
