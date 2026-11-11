//! The descriptor and no-follow forms of `execveat`, T-1703.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1703. `fexecve`
//! is `execveat(fd, "", argv, envp, AT_EMPTY_PATH)`, and the kernel then
//! reads the loader or the `#!` interpreter of the file on the host root,
//! as T-1646 found for a path. Thus the route reads the file of the
//! descriptor the same way:
//!
//! * a file with a guest name takes the route of that name, as an
//!   `execve` of it;
//! * a file with no guest name (a `memfd`, a deleted file) takes the route
//!   of `/proc/self/fd/N`. The guest loader opens that name after the
//!   exec, so the descriptor loses `FD_CLOEXEC` for the exec, and gets it
//!   back when the exec fails.
//!
//! `AT_SYMLINK_NOFOLLOW` refuses a link as its last component with
//! `ELOOP`, as the kernel does, and then takes the route of the name.
//!
//! ⛔ No allocation: each buffer is the caller's [`Names`] or [`Scratch`].

use super::*;

/// `AT_EMPTY_PATH`.
pub const AT_EMPTY_PATH: c_int = 0x1000;
/// `AT_SYMLINK_NOFOLLOW`.
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;

const F_GETFD: c_int = 1;
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;

unsafe extern "C" {
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
}

/// The names one descriptor route keeps alive until the exec, on the
/// caller's stack.
pub struct Names {
    /// The guest name of the descriptor's file.
    name: [u8; OUT],
    /// The buffer that [`route`] takes.
    buf: [u8; OUT],
    /// `/dev/fd/N`: the name that a `#!` interpreter gets, as the kernel
    /// gives it.
    file0: [u8; 32],
    /// The descriptor and its old flags, where the route cleared
    /// `FD_CLOEXEC`.
    restore: Option<(c_int, c_int)>,
}

impl Names {
    pub const fn new() -> Names {
        Names {
            name: [0; OUT],
            buf: [0; OUT],
            file0: [0; 32],
            restore: None,
        }
    }

    /// The exec failed: the descriptor gets its `FD_CLOEXEC` back.
    pub fn exec_failed(&mut self) {
        if let Some((fd, flags)) = self.restore.take() {
            let e = crate::errno();
            unsafe { fcntl(fd, F_SETFD, flags) };
            crate::set_errno(e);
        }
    }
}

/// Write `prefix` and the decimal `fd` with a NUL into `out`; the length.
pub fn fd_path(prefix: &[u8], fd: c_int, out: &mut [u8]) -> usize {
    let mut num = [0u8; 12];
    let mut v = (fd as i64).unsigned_abs();
    let mut d = 0usize;
    loop {
        num[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
        if v == 0 {
            break;
        }
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let mut at = prefix.len();
    while d > 0 {
        d -= 1;
        out[at] = num[d];
        at += 1;
    }
    out[at] = 0;
    at
}

/// The route of `fexecve` and of `execveat` with `AT_EMPTY_PATH` and an
/// empty path. A `Pass` means the earlier behaviour: the real call with
/// the caller's own arguments.
///
/// # Safety
/// `argv` and `envp` as the exec's own contract.
pub unsafe fn route_fd(
    what: &[u8],
    fd: c_int,
    argv: *const *const c_char,
    envp: *const *const c_char,
    s: &mut Scratch,
    n: &mut Names,
) -> Route {
    let t = unsafe { crate::map::table() };
    if t.n == 0 || !anchored(&t) {
        return Route::Pass(core::ptr::null());
    }
    if unsafe { crate::map::fd_guest(&t, fd, &mut n.name) } > 0 {
        let name = n.name.as_ptr() as *const c_char;
        return unsafe { route(what, AT_FDCWD, name, argv, envp, false, s, &mut n.buf) };
    }
    let flags = unsafe { fcntl(fd, F_GETFD, 0) };
    if flags < 0 {
        return Route::Fail;
    }
    fd_path(b"/proc/self/fd/", fd, &mut s.resolved);
    fd_path(b"/proc/self/fd/", fd, &mut s.host);
    fd_path(b"/dev/fd/", fd, &mut n.file0);
    if flags & FD_CLOEXEC != 0 {
        unsafe { fcntl(fd, F_SETFD, flags & !FD_CLOEXEC) };
        n.restore = Some((fd, flags));
    }
    let file0 = n.file0.as_ptr() as *const c_char;
    match unsafe { plan(what, &t, 0, file0, argv, envp, s, true) } {
        Ok(l) => Route::Launch(l),
        Err(e) => {
            crate::set_errno(e);
            n.exec_failed();
            Route::Fail
        }
    }
}

/// Is the last component of `path` (for `dirfd`) a symbolic link? The
/// kernel answers `ELOOP` for it under `AT_SYMLINK_NOFOLLOW`.
///
/// # Safety
/// `path` as the exec's own contract.
pub unsafe fn is_link(dirfd: c_int, path: *const c_char, buf: &mut [u8; OUT]) -> bool {
    let p = unsafe { crate::map::prepare_follow(b"execveat", dirfd, path, false, false, buf) };
    let Some(readlinkat) = crate::next_readlinkat() else {
        return false;
    };
    let mut one = [0u8; 1];
    !p.is_null() && unsafe { readlinkat(dirfd, p, one.as_mut_ptr() as *mut c_char, 1) } >= 0
}
