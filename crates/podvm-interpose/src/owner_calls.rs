use super::*;

/// `AT_SYMLINK_NOFOLLOW`, for the `lchown` half of `fchownat`.
pub(super) const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
pub(super) const AT_FDCWD: c_int = -100;

/// The shared body of every `chown`-family entry point.
///
/// ⛔ **Try the real call FIRST.** A runtime where the `chown` would have
/// succeeded must not get a memo instead of the real thing: the memo changes no
/// kernel permission check, and preferring it would make podvm weaker than the
/// bare chroot on a machine that can do the real work.
///
/// ⚠ `uid` and `gid` of `-1` mean "leave it", which is `chown(2)`'s own
/// convention, so `set` records what the payload actually asked for.
unsafe fn owned(
    real: Option<i32>,
    stat_for_ids: Option<(u64, u64)>,
    uid: c_uint,
    gid: c_uint,
    what: &[u8],
) -> c_int {
    let Some(rc) = real else {
        // ⚠ The payload's libc does not define this entry point at all, which
        // this object cannot happen upon: it was resolved through
        // `dlsym(RTLD_NEXT)` and a null there means the symbol is gone.
        say::line(&[what, b": podvm could not resolve the real call"]);
        unsafe { *__errno_location() = EINVAL };
        return -1;
    };
    if rc == 0 {
        return 0;
    }
    let e = errno();
    if !is_the_wall(e) {
        return rc;
    }
    let Some((dev, ino)) = stat_for_ids else {
        // ⛔ podvm could not learn WHICH file, so it has nothing to record and
        // hands the payload the real failure. A success reported with no memo
        // behind it is the lie this whole object exists to avoid.
        say::line(&[
            what,
            b": the call was refused and podvm could not stat the target, so it \
              has nothing to record and the error is the payload's",
        ]);
        unsafe { *__errno_location() = e };
        return -1;
    };
    let mut set = 0u32;
    if uid != c_uint::MAX {
        set |= memo::SET_UID;
    }
    if gid != c_uint::MAX {
        set |= memo::SET_GID;
    }
    let o = memo::Owner { uid, gid, set };
    if !memo::record(dev, ino, o) {
        say::line(&[
            what,
            b": podvm could not write the ownership memo, so the failure is the \
              payload's rather than being hidden",
        ]);
        unsafe { *__errno_location() = e };
        return -1;
    }
    if say::debug() {
        say::line(&[
            what,
            b": the runtime refused it with errno ",
            say::Num::new(e as u64).as_bytes(),
            b"; podvm recorded the intended owner and reported success \
              (TODO/interpose.md T-0704)",
        ]);
    }
    clear_errno();
    0
}

/// `dev` and `ino` for a path, through the REAL `stat`, so this does not recurse
/// into podvm's own interposition.
unsafe fn ids_of_path(path: *const c_char, follow: bool) -> Option<(u64, u64)> {
    let mut st = [0u8; 256];
    let f = if follow { next_stat() } else { next_lstat() };
    let rc = match f {
        Some(f) => unsafe { f(path, st.as_mut_ptr() as *mut c_void) },
        None => {
            // ⚠ glibc before 2.33 exports `stat` only as `__xstat`, so the
            // fallback is not a nicety: on such a payload `dlsym("stat")`
            // answers null. `1` is `_STAT_VER_LINUX` on x86_64.
            let g = if follow { next_xstat() } else { next_lxstat() }?;
            unsafe { g(1, path, st.as_mut_ptr() as *mut c_void) }
        }
    };
    if rc != 0 {
        return None;
    }
    Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
}

unsafe fn ids_of_fd(fd: c_int) -> Option<(u64, u64)> {
    let mut st = [0u8; 256];
    let rc = match next_fstat() {
        Some(f) => unsafe { f(fd, st.as_mut_ptr() as *mut c_void) },
        None => {
            let g = next_fxstat()?;
            unsafe { g(1, fd, st.as_mut_ptr() as *mut c_void) }
        }
    };
    if rc != 0 {
        return None;
    }
    Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
}

// ------------------------------------------------------- the exported symbols
//
// ⛔ Every one of these is in `interpose.map` and nothing else is. A symbol
// exported by accident is a symbol some other library in the payload's process
// resolves to podvm.

/// # Safety
/// The payload's own contract for `chown(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"chown", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let ids = unsafe { ids_of_path(p, true) };
    let rc = next_chown().map(|f| unsafe { f(p, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"chown") }
}

/// # Safety
/// The payload's own contract for `lchown(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lchown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"lchown", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let ids = unsafe { ids_of_path(p, false) };
    let rc = next_lchown().map(|f| unsafe { f(p, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"lchown") }
}

/// # Safety
/// The payload's own contract for `fchown(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchown(fd: c_int, uid: c_uint, gid: c_uint) -> c_int {
    let ids = unsafe { ids_of_fd(fd) };
    let rc = next_fchown().map(|f| unsafe { f(fd, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"fchown") }
}

/// # Safety
/// The payload's own contract for `fchownat(2)`.
///
/// ⚠ Hand-written rather than macro-generated, which is the one shape
/// `references/VHSgunzo__pathmap/tree/path-mapping.c:1242-1256` also writes out:
/// the flags decide whether the target is the link or what it points at, and a
/// macro over the family cannot express that.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    uid: c_uint,
    gid: c_uint,
    flags: c_int,
) -> c_int {
    // ⭐ The path goes through the table first, so a mapped `fchownat` memos
    // the file the kernel actually changed. Where the rewrite moved it, the
    // call goes with an absolute path; where it did not, the original
    // descriptor and path go through untouched.
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"fchownat", dirfd, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let (rfd, rpath) = if p == path {
        (dirfd, path)
    } else {
        (AT_FDCWD, p)
    };
    let ids = if rfd == AT_FDCWD || unsafe { *rpath } == b'/' as c_char {
        unsafe { ids_of_path(rpath, flags & AT_SYMLINK_NOFOLLOW == 0) }
    } else {
        // A relative path against a directory descriptor, rewritten above
        // where the table matched it. The real `fstatat` answers what is
        // left without podvm resolving anything further.
        let mut st = [0u8; 256];
        let rc = match next_fstatat() {
            Some(f) => unsafe { f(rfd, rpath, st.as_mut_ptr() as *mut c_void, flags) },
            None => match next_fxstatat() {
                Some(g) => unsafe { g(1, rfd, rpath, st.as_mut_ptr() as *mut c_void, flags) },
                None => -1,
            },
        };
        if rc == 0 {
            Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
        } else {
            None
        }
    };
    let rc = next_fchownat().map(|f| unsafe { f(rfd, rpath, uid, gid, flags) });
    unsafe { owned(rc, ids, uid, gid, b"fchownat") }
}

/// Every `stat`-family entry point: call the real one, then report the memo.
///
/// ⛔ A macro, so the sixteen shapes cannot drift apart. T-0703's Decision names
/// the same reason for the path families.
macro_rules! stat_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            let rc = unsafe { f($($arg),*) };
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

/// The path-taking `stat` shapes: rewrite first, then the real call, then
/// the memo. The memo is keyed by inode, so it answers for the rewritten
/// file, which is the file the kernel just stated.
///
/// T-1646: a shape that follows the last link (`$follow`) tries the rootfs
/// walk once where the mapped path misses ([`crate::exec::lookup::again`]).
macro_rules! stat_path_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident, $path:ident, $follow:expr) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            // ⭐ T-1409. Loader-family entries answer the payload-visible exe
            // from the guest path even where the real call succeeds (host
            // `/proc` mounted, the real stat naming the loader). Gated on
            // the CLI-set flag; declined where no guest path rides, falling
            // through to the real exe. The substitution says nothing here:
            // the CLI names it once on the banner.
            if unsafe { crate::procfs::exe_override_hit($path) } {
                let mut gout = [0u8; crate::map::OUT];
                if let Some(gp) = unsafe { stat_exe_path(&mut gout) } {
                    let $path = gp;
                    let rc = unsafe { f($($arg),*) };
                    if rc == 0 {
                        unsafe { report($st) };
                    }
                    return rc;
                }
            }
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            let guest = $path;
            let $path = p;
            let mut rc = unsafe { f($($arg),*) };
            let mut walked = [0u8; crate::map::OUT];
            if rc != 0
                && $follow
                && let Some(q) = unsafe {
                    crate::exec::lookup::again(crate::map::AT_FDCWD, guest, p, &mut walked)
                }
            {
                let $path = q;
                rc = unsafe { f($($arg),*) };
            }
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

/// The `*at` `stat` shapes: resolve the descriptor pair first. Where the
/// rewrite moved the path the call goes with an absolute one; where it did
/// not, the original descriptor and path go through untouched.
///
/// T-1646: `$follow` is false where the flags hold `AT_SYMLINK_NOFOLLOW`;
/// otherwise a miss tries the rootfs walk once, as the path shapes do.
macro_rules! stat_at_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident, $dirfd:ident, $path:ident, $follow:expr) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            // ⭐ T-1409, as in the path shapes: the guest path is absolute,
            // so the call goes with `AT_FDCWD` where the override serves it.
            if unsafe { crate::procfs::exe_override_hit($path) } {
                let mut gout = [0u8; crate::map::OUT];
                if let Some(gp) = unsafe { stat_exe_path(&mut gout) } {
                    let $dirfd = crate::map::AT_FDCWD;
                    let $path = gp;
                    let rc = unsafe { f($($arg),*) };
                    if rc == 0 {
                        unsafe { report($st) };
                    }
                    return rc;
                }
            }
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return -1;
            }
            let follow = $follow;
            let (guest_fd, guest) = ($dirfd, $path);
            let ($dirfd, $path) = if p == $path {
                ($dirfd, $path)
            } else {
                (crate::map::AT_FDCWD, p)
            };
            let mut rc = unsafe { f($($arg),*) };
            let mut walked = [0u8; crate::map::OUT];
            if rc != 0
                && follow
                && let Some(q) = unsafe { crate::exec::lookup::again(guest_fd, guest, p, &mut walked) }
            {
                let ($dirfd, $path) = (crate::map::AT_FDCWD, q);
                rc = unsafe { f($($arg),*) };
            }
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

stat_path_entry!(stat, next_stat, (path: *const c_char, st: *mut c_void), st, path, true);
stat_path_entry!(lstat, next_lstat, (path: *const c_char, st: *mut c_void), st, path, false);
stat_entry!(fstat, next_fstat, (fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    fstatat,
    next_fstatat,
    (dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path,
    flags & 0x100 == 0
);
// ⭐ THE `64` NAMES ARE NOT DUPLICATES, and T-0703 measured why: all four
// `libpython3.*` and `libglib-2.0.so.0` import only `stat64`, `lstat64` and
// `fstatat64`, while `libarchive` and `libdbus-1` import the plain names. Both
// sets are live in one process.
stat_path_entry!(stat64, next_stat64, (path: *const c_char, st: *mut c_void), st, path, true);
stat_path_entry!(lstat64, next_lstat64, (path: *const c_char, st: *mut c_void), st, path, false);
stat_entry!(fstat64, next_fstat64, (fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    fstatat64,
    next_fstatat64,
    (dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path,
    flags & 0x100 == 0
);
// ⭐ AND THE `__xstat` SHAPE, which is glibc's pre-2.33 one and which glibc 2.39
// still exports at `GLIBC_2.2.5`. The PAYLOAD's libc decides which shape its
// libraries call, not the host this object was built on.
stat_path_entry!(
    __xstat,
    next_xstat,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path,
    true
);
stat_path_entry!(
    __lxstat,
    next_lxstat,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path,
    false
);
// ⭐ AND THEIR `64` SHAPE, which the completeness command counts as reached.
// Same contract as above: define it, forward it, let the loader decide.
stat_path_entry!(
    __xstat64,
    next_xstat64,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path,
    true
);
stat_path_entry!(
    __lxstat64,
    next_lxstat64,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path,
    false
);
stat_entry!(__fxstat, next_fxstat, (ver: c_int, fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    __fxstatat,
    next_fxstatat,
    (ver: c_int, dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path,
    flags & 0x100 == 0
);
stat_at_entry!(
    __fxstatat64,
    next_fxstatat64,
    (ver: c_int, dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path,
    flags & 0x100 == 0
);
