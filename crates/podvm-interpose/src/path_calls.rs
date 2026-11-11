use super::*;

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_current_dir_name() -> *mut c_char {
    let Some(f) = next_get_current_dir_name() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f() };
    }
    let r = unsafe { f() };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { reverse_alloc(r as *mut c_void) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn realpath(path: *const c_char, resolved: *mut c_char) -> *mut c_char {
    let Some(f) = next_realpath() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let mut exe = [0u8; crate::map::OUT];
    let path = unsafe { exe_input(path, &mut exe) };
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"realpath", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(p, resolved) } as *mut c_char;
    }
    let path = p;
    if resolved.is_null() {
        let r = unsafe { f(path, core::ptr::null_mut()) };
        if r.is_null() {
            return core::ptr::null_mut();
        }
        return unsafe { reverse_alloc(r) };
    }
    // The caller sized the buffer at PATH_MAX: POSIX gives the call no size
    // argument, so glibc assumes that scale, and OUT matches it. A virtual
    // name past it answers `ERANGE` rather than overflowing.
    let mut tmp = [0u8; crate::map::OUT];
    let r = unsafe { f(path, tmp.as_mut_ptr() as *mut c_char) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    let real_n = crate::map::strnlen(tmp.as_ptr() as *const c_char, crate::map::OUT);
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, tmp.as_ptr(), real_n, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    let (src, n) = if v < 0 {
        (tmp.as_ptr(), real_n)
    } else {
        (vbuf.as_ptr(), v as usize)
    };
    unsafe { copy_fit(src, n, resolved, crate::map::OUT) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn canonicalize_file_name(path: *const c_char) -> *mut c_char {
    let Some(f) = next_canonicalize() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let mut exe = [0u8; crate::map::OUT];
    let path = unsafe { exe_input(path, &mut exe) };
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe {
        crate::map::prepare(
            b"canonicalize_file_name",
            crate::map::AT_FDCWD,
            path,
            &mut buf,
        )
    };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(p) } as *mut c_char;
    }
    let r = unsafe { f(p) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { reverse_alloc(r) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlink(path: *const c_char, buf: *mut c_char, n: usize) -> isize {
    let fail: isize = -1;
    let Some(f) = crate::map::next_readlink() else {
        set_errno(EINVAL);
        return fail;
    };
    // ⭐ T-1409. Loader-family entries answer the payload-visible exe from
    // the guest path even where the real call succeeds (host `/proc`
    // mounted, the real readlink naming the loader). Gated on the CLI-set
    // flag; declined where no guest path rides, falling through to the real
    // call. The substitution says nothing here: the CLI names it once on
    // the banner.
    if unsafe { crate::procfs::exe_override_hit(path) }
        && let Some(e) = unsafe { proc_readlink_exe(buf, n) }
    {
        return e;
    }
    let mut ibuf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"readlink", crate::map::AT_FDCWD, path, &mut ibuf) };
    if p.is_null() {
        return fail;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        let rc = unsafe { f(p, buf, n) };
        // T-0413: where no procfs is mounted the real call fails `ENOENT`,
        // and only there does the emulation answer.
        if rc < 0
            && errno() == crate::procfs::ENOENT
            && let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) }
        {
            return e;
        }
        return rc;
    }
    let rc = unsafe { f(p, buf, n) };
    if rc < 0 {
        // T-0413, as above. `path` is the caller's own spelling only where
        // the table left it untouched (`p` is then that same pointer): a
        // deliberate mapping of `/dev/fd` wins over this emulation, and
        // the `/proc` leaves never rewrite (T-0707 excludes them).
        if p == path
            && errno() == crate::procfs::ENOENT
            && let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) }
        {
            return e;
        }
        return rc;
    }
    // The answer is bytes, not a string: no NUL terminates it.
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, buf as *const u8, rc as usize, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return fail;
    }
    if v < 0 {
        return rc;
    }
    if v as usize > n {
        set_errno(ERANGE);
        return fail;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(vbuf.as_ptr(), buf as *mut u8, v as usize);
    }
    v
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    n: usize,
) -> isize {
    let fail: isize = -1;
    let Some(f) = next_readlinkat() else {
        set_errno(EINVAL);
        return fail;
    };
    // ⭐ T-1409, as in `readlink`: the guest exe answers even where the real
    // call succeeds, gated on the CLI-set flag.
    if unsafe { crate::procfs::exe_override_hit(path) }
        && let Some(e) = unsafe { proc_readlink_exe(buf, n) }
    {
        return e;
    }
    let mut ibuf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"readlinkat", dirfd, path, &mut ibuf) };
    if p.is_null() {
        return fail;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        let rc = unsafe { f(dirfd, p, buf, n) };
        // T-0413, as in `readlink`: only `ENOENT` answers.
        if rc < 0
            && errno() == crate::procfs::ENOENT
            && let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) }
        {
            return e;
        }
        return rc;
    }
    let rc = unsafe { f(dirfd, p, buf, n) };
    if rc < 0 {
        // T-0413, as in `readlink`: untouched spellings only.
        if p == path
            && errno() == crate::procfs::ENOENT
            && let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) }
        {
            return e;
        }
        return rc;
    }
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, buf as *const u8, rc as usize, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return fail;
    }
    if v < 0 {
        return rc;
    }
    if v as usize > n {
        set_errno(ERANGE);
        return fail;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(vbuf.as_ptr(), buf as *mut u8, v as usize);
    }
    v
}

// --------------------------------------- T-0413: `/proc/self` emulation
//
// Where no procfs is mounted the real call fails `ENOENT`, and only then do
// these answer: the wrappers below try the real call first and call in here
// on exactly that errno. Each answers `Some` where it served (a descriptor,
// or -1 with the kernel's exact errno) and `None` where it declines, and a
// decline preserves the real failure's errno. `readlink` truncates into a
// short buffer the way the kernel does; nothing here invents a byte.

/// Fill `st` with the descriptor's real metadata.
///
/// Through `next_fstat`, never the wrapper two lines down the file: the
/// wrapper would report the memo over the bytes this emulation reasons
/// about, and a memo hit is a statement about ownership, not about the
/// descriptor's type.
pub(crate) unsafe fn proc_fstat(fdno: u32, st: &mut [u8; crate::procfs::STAT_LEN]) -> bool {
    let Some(fstat) = next_fstat() else {
        return false;
    };
    unsafe { fstat(fdno as c_int, st.as_mut_ptr() as *mut c_void) == 0 }
}

/// Serve `open` of a descriptor leaf, T-0413.
pub(super) unsafe fn proc_open_fd(fdno: u32, flags: c_int) -> Option<c_int> {
    use crate::procfs::*;
    let mut st = [0u8; STAT_LEN];
    if !unsafe { proc_fstat(fdno, &mut st) } {
        // No such descriptor: the kernel's answer for the leaf is ENOENT,
        // with procfs or without it.
        set_errno(ENOENT);
        return Some(-1);
    }
    let mode = mode_of(&st);
    if mode & S_IFMT == S_IFSOCK {
        // A socket has no reopen: the kernel answers ENXIO, exactly.
        set_errno(ENXIO);
        return Some(-1);
    }
    if mode & S_IFMT != S_IFIFO {
        return None;
    }
    if unsafe { fs_magic(fdno as c_int) } != Some(PIPEFS_MAGIC) {
        // A named fifo reopens with blocking semantics no duplicate
        // reproduces: refused, never approximated. The `fstatfs` may have
        // clobbered the real failure's errno, so it is restored.
        set_errno(ENOENT);
        return None;
    }
    let Some(orig) = (unsafe { open_flags(fdno as c_int) }) else {
        set_errno(ENOENT);
        return Some(-1);
    };
    match open_answer(orig, flags) {
        OpenAnswer::Dup => {
            let fd = unsafe { dup_of(fdno as c_int, flags & O_CLOEXEC != 0) };
            if fd < 0 {
                return Some(-1);
            }
            // No tally behind it, no emulation: the duplicate closes and
            // the failure reads as the memo write's own errno.
            if !crate::emulate::tally(crate::emulate::OP_PROC, 1, 0, 0) {
                let e = errno();
                if let Some(close) = next_close() {
                    unsafe { close(fd) };
                }
                set_errno(e);
                return Some(-1);
            }
            Some(fd)
        }
        OpenAnswer::Fail(e) => {
            set_errno(e);
            Some(-1)
        }
        OpenAnswer::Pass => None,
    }
}

/// Serve `readlink` of a descriptor leaf, T-0413.
pub(super) unsafe fn proc_readlink_fd(fdno: u32, buf: *mut c_char, n: usize) -> Option<isize> {
    use crate::procfs::*;
    let mut st = [0u8; STAT_LEN];
    if !unsafe { proc_fstat(fdno, &mut st) } {
        set_errno(ENOENT);
        return Some(-1);
    }
    let mode = mode_of(&st);
    let prefix = if mode & S_IFMT == S_IFSOCK {
        SOCKET_PREFIX
    } else if mode & S_IFMT == S_IFIFO && unsafe { fs_magic(fdno as c_int) } == Some(PIPEFS_MAGIC) {
        PIPE_PREFIX
    } else {
        // A path the emulation cannot name exactly (a regular file, a
        // device, a named fifo): refused, with the real errno restored
        // past the `fstatfs` above.
        set_errno(crate::procfs::ENOENT);
        return None;
    };
    let mut link = [0u8; 32];
    let len = fd_link(prefix, ino_of(&st), &mut link);
    let take = core::cmp::min(len, n);
    unsafe {
        core::ptr::copy_nonoverlapping(link.as_ptr(), buf as *mut u8, take);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 2, 0, 0) {
        set_errno(ENOENT);
        return Some(-1);
    }
    Some(take as isize)
}

/// The caller-resolved guest path, validated: a leading slash, no interior
/// NUL, short enough to hand on. Anything else is no variable rather than a
/// guess at one.
pub(crate) unsafe fn proc_exe_value(out: &mut [u8; crate::map::OUT]) -> Option<usize> {
    let (p, n) = unsafe { crate::map::lookup_env(crate::procfs::GUEST_EXE_VAR) }?;
    if n == 0 || n + 1 >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(p, n) };
    if bytes.contains(&0) || !bytes.starts_with(b"/") {
        return None;
    }
    out[..n].copy_from_slice(bytes);
    out[n] = 0;
    Some(n)
}

/// The input of `realpath` and `canonicalize_file_name`: the guest name of
/// the program in `out` where `path` is `/proc/self/exe` under the
/// override, else `path`.
///
/// ⛔ T-1704. libc follows the kernel link of `/proc/self/exe`, which names
/// the image's loader that podvm started, while `readlink` answers the
/// program (T-1409). A runtime that finds its own directory with
/// `realpath` then looks beside the loader (the .NET apphost exits 154). So
/// both calls start from the name that `readlink` gives and resolve it
/// through the table: the answer is the program after the image's links.
pub(super) unsafe fn exe_input(
    path: *const c_char,
    out: &mut [u8; crate::map::OUT],
) -> *const c_char {
    if unsafe { crate::procfs::exe_override_hit(path) } && unsafe { proc_exe_value(out) }.is_some()
    {
        return out.as_ptr() as *const c_char;
    }
    path
}

/// Serve `readlink` of `/proc/self/exe`, T-0413.
pub(super) unsafe fn proc_readlink_exe(buf: *mut c_char, n: usize) -> Option<isize> {
    let mut guest = [0u8; crate::map::OUT];
    let len = unsafe { proc_exe_value(&mut guest) }?;
    let take = core::cmp::min(len, n);
    unsafe {
        core::ptr::copy_nonoverlapping(guest.as_ptr(), buf as *mut u8, take);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 3, 0, 0) {
        set_errno(crate::procfs::ENOENT);
        return Some(-1);
    }
    Some(take as isize)
}

/// Serve `open` of `/proc/self/exe`, T-0413.
///
/// The guest path goes back through the table: it names the image as the
/// payload sees it, so a deliberate mapping applies to it like any other
/// guest path. The flags ride through to the same file, which is what the
/// kernel applies them to. Counted only where the open succeeded: a failed
/// open told the payload nothing but the kernel's errno.
pub(super) unsafe fn proc_open_exe(flags: c_int, mode: c_uint) -> Option<c_int> {
    let mut guest = [0u8; crate::map::OUT];
    let _ = unsafe { proc_exe_value(&mut guest) }?;
    let mut rewritten = [0u8; crate::map::OUT];
    let p = unsafe {
        crate::map::prepare_literal(b"open", guest.as_ptr() as *const c_char, &mut rewritten)
    };
    if p.is_null() {
        return Some(-1);
    }
    let Some(open) = next_open() else {
        set_errno(EINVAL);
        return Some(-1);
    };
    let fd = unsafe { open(p, flags, mode) };
    // No tally behind it, no emulation: the open closes and the failure
    // reads as the memo write's own errno.
    if fd >= 0 && !crate::emulate::tally(crate::emulate::OP_PROC, 4, 0, 0) {
        let e = errno();
        if let Some(close) = next_close() {
            unsafe { close(fd) };
        }
        set_errno(e);
        return Some(-1);
    }
    Some(fd)
}

/// The guest exe path rewritten for a `stat` call, T-1409.
///
/// Resolves the guest value and rewrites it through the table into `out`,
/// returning the pointer to stat. `None` where the override declines (no
/// guest value, or the table moved it): the caller falls through to the
/// real exe. No allocation; `out` is the caller's stack buffer.
///
/// # Safety
/// `out` must hold `OUT` writable bytes, which is what every entry point
/// passes.
pub(super) unsafe fn stat_exe_path(out: &mut [u8; crate::map::OUT]) -> Option<*const c_char> {
    let mut guest = [0u8; crate::map::OUT];
    let _ = unsafe { proc_exe_value(&mut guest) }?;
    let p = unsafe { crate::map::prepare_literal(b"stat", guest.as_ptr() as *const c_char, out) };
    if p.is_null() {
        return None;
    }
    Some(p)
}
