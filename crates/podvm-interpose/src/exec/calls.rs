//! The exec entry points, T-0703 and T-1646.
//!
//! Each one takes [`route`]: a `Pass` calls the real function as before,
//! with the mapped path; a `Launch` calls the real `execve` (or the real
//! `posix_spawn`) with the new vectors. The `p` variants find the name on
//! `PATH` themselves on the userland rung, so their launch needs no second
//! search: the real search would read the host's directories.

use super::*;
use crate::set_errno;
use core::ffi::c_void;

unsafe extern "C" {
    static environ: *const *const c_char;
}

/// # Safety
/// The payload's own contract for `execve(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let Some(f) = crate::next_execve() else {
        set_errno(EINVAL);
        return -1;
    };
    let (mut s, mut buf) = (Scratch::new(), [0u8; OUT]);
    match unsafe {
        route(
            b"execve", AT_FDCWD, path, argv, envp, false, &mut s, &mut buf,
        )
    } {
        Route::Pass(p) => unsafe { f(p, argv, envp) },
        Route::Fail => -1,
        Route::Launch(l) => unsafe { f(l.path, l.argv, l.envp) },
    }
}

/// The shared body of `execv`, `execvp` and `execvpe`: a launch takes the
/// real `execve`, a pass takes `pass`.
unsafe fn exec_env(
    what: &[u8],
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    search: bool,
    pass: impl FnOnce(*const c_char) -> c_int,
) -> c_int {
    let Some(f) = crate::next_execve() else {
        set_errno(EINVAL);
        return -1;
    };
    let (mut s, mut buf) = (Scratch::new(), [0u8; OUT]);
    match unsafe { route(what, AT_FDCWD, path, argv, envp, search, &mut s, &mut buf) } {
        Route::Pass(p) => pass(p),
        Route::Fail => -1,
        Route::Launch(l) => unsafe { f(l.path, l.argv, l.envp) },
    }
}

/// # Safety
/// The payload's own contract for `execv(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    let Some(f) = crate::next_execv() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { exec_env(b"execv", path, argv, environ, false, |p| f(p, argv)) }
}

/// # Safety
/// The payload's own contract for `execvp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execvp(path: *const c_char, argv: *const *const c_char) -> c_int {
    let Some(f) = crate::next_execvp() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { exec_env(b"execvp", path, argv, environ, true, |p| f(p, argv)) }
}

/// # Safety
/// The payload's own contract for `execvpe(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execvpe(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let Some(f) = crate::next_execvpe() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { exec_env(b"execvpe", path, argv, envp, true, |p| f(p, argv, envp)) }
}

/// `execveat(2)`, T-1646 and T-1703. `AT_EMPTY_PATH` with an empty path
/// (the form of `fexecve`) takes the route of the descriptor's file;
/// `AT_SYMLINK_NOFOLLOW` refuses a link with `ELOOP` and then takes the
/// route of the name. Another flag reaches the real call, which answers
/// `EINVAL` for it.
///
/// # Safety
/// The payload's own contract for `execveat(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execveat(
    dirfd: c_int,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    flags: c_int,
) -> c_int {
    use super::descriptor::{AT_EMPTY_PATH, AT_SYMLINK_NOFOLLOW};
    let f = real_execveat;
    let Some(exec) = crate::next_execve() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; OUT];
    let mut s = Scratch::new();
    if flags & !(AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW) != 0 {
        let p = unsafe { crate::map::prepare(b"execveat", dirfd, path, &mut buf) };
        if p.is_null() {
            return -1;
        }
        return unsafe { f(dirfd, p, argv, envp, flags) };
    }
    if flags & AT_EMPTY_PATH != 0 && !path.is_null() && unsafe { *path } == 0 {
        let mut n = super::descriptor::Names::new();
        return match unsafe {
            super::descriptor::route_fd(b"fexecve", dirfd, argv, envp, &mut s, &mut n)
        } {
            Route::Pass(_) => unsafe { f(dirfd, path, argv, envp, flags) },
            Route::Fail => -1,
            Route::Launch(l) => {
                let r = unsafe { exec(l.path, l.argv, l.envp) };
                n.exec_failed();
                r
            }
        };
    }
    if flags & AT_SYMLINK_NOFOLLOW != 0
        && unsafe { super::descriptor::is_link(dirfd, path, &mut buf) }
    {
        set_errno(ELOOP);
        return -1;
    }
    match unsafe {
        route(
            b"execveat",
            dirfd,
            path,
            argv,
            envp,
            false,
            &mut s,
            &mut buf,
        )
    } {
        Route::Pass(p) => unsafe { f(dirfd, p, argv, envp, flags) },
        Route::Fail => -1,
        Route::Launch(l) => unsafe { exec(l.path, l.argv, l.envp) },
    }
}

/// The real `execveat`. musl exports none (its `fexecve` makes the system
/// call itself), so there the call goes to the kernel directly.
unsafe extern "C" fn real_execveat(
    dirfd: c_int,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    flags: c_int,
) -> c_int {
    if let Some(f) = crate::next_execveat() {
        return unsafe { f(dirfd, path, argv, envp, flags) };
    }
    let a = [
        dirfd as core::ffi::c_long,
        path as core::ffi::c_long,
        argv as core::ffi::c_long,
        envp as core::ffi::c_long,
        flags as core::ffi::c_long,
        0,
    ];
    unsafe { crate::raw_calls::forward(SYS_EXECVEAT, a) as c_int }
}

/// `SYS_execveat` on x86_64.
use crate::raw_calls::SYS_EXECVEAT;

/// `fexecve(3)`: glibc and musl call `execveat` inside libc, past the
/// preload, so podvm exports it and takes the descriptor route (T-1703).
///
/// # Safety
/// The payload's own contract for `fexecve(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fexecve(
    fd: c_int,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        execveat(
            fd,
            c"".as_ptr(),
            argv,
            envp,
            super::descriptor::AT_EMPTY_PATH,
        )
    }
}

/// The shared body of `posix_spawn` and `posix_spawnp`. These answer an
/// error number, not `-1`.
#[allow(clippy::too_many_arguments)]
unsafe fn spawn(
    what: &[u8],
    pid: *mut c_int,
    path: *const c_char,
    fa: *const c_void,
    attr: *const c_void,
    argv: *const *const c_char,
    envp: *const *const c_char,
    search: bool,
    pass: impl FnOnce(*const c_char) -> c_int,
) -> c_int {
    let Some(f) = crate::next_posix_spawn() else {
        return EINVAL;
    };
    let (mut s, mut buf) = (Scratch::new(), [0u8; OUT]);
    match unsafe { route(what, AT_FDCWD, path, argv, envp, search, &mut s, &mut buf) } {
        Route::Pass(p) => pass(p),
        Route::Fail => crate::errno(),
        Route::Launch(l) => unsafe { f(pid, l.path, fa, attr, l.argv, l.envp) },
    }
}

/// # Safety
/// The payload's own contract for `posix_spawn(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut c_int,
    path: *const c_char,
    fa: *const c_void,
    attr: *const c_void,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let Some(f) = crate::next_posix_spawn() else {
        return EINVAL;
    };
    let pass = |p| unsafe { f(pid, p, fa, attr, argv, envp) };
    unsafe { spawn(b"posix_spawn", pid, path, fa, attr, argv, envp, false, pass) }
}

/// # Safety
/// The payload's own contract for `posix_spawnp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut c_int,
    path: *const c_char,
    fa: *const c_void,
    attr: *const c_void,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let Some(f) = crate::next_posix_spawnp() else {
        return EINVAL;
    };
    let pass = |p| unsafe { f(pid, p, fa, attr, argv, envp) };
    unsafe { spawn(b"posix_spawnp", pid, path, fa, attr, argv, envp, true, pass) }
}
