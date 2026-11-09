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

/// `execveat(2)`. A call with flags (`AT_EMPTY_PATH` for `fexecve`,
/// `AT_SYMLINK_NOFOLLOW`) keeps the earlier behaviour: it names a
/// descriptor or forbids the link walk that the route makes.
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
    let Some(f) = crate::next_execveat() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; OUT];
    if flags != 0 {
        let p = unsafe { crate::map::prepare(b"execveat", dirfd, path, &mut buf) };
        if p.is_null() {
            return -1;
        }
        return unsafe { f(dirfd, p, argv, envp, flags) };
    }
    let Some(exec) = crate::next_execve() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut s = Scratch::new();
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
