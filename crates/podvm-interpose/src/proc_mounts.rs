use super::*;

/// Serve `open` of a mount-table file, T-0413.
///
/// The fixture is generated into a `memfd_create` descriptor from live
/// topology (the payload root's own device and filesystem) plus the
/// recorded T-0708 emulated mounts, per `TOOL.md` section 10. Where the
/// memo descriptor was not handed, or the old libc has no `memfd_create`,
/// or any step fails, this declines and the real failure stands:
/// an uncounted or half-written table is worse than an absent one.
unsafe fn proc_open_mounts(kind: crate::procfs::MountFile, flags: c_int) -> Option<c_int> {
    use crate::procfs::*;
    let saved = errno();
    let fail = |e: c_int| -> Option<c_int> {
        set_errno(e);
        None
    };
    let Some(memo) = crate::memo::memo_fd() else {
        return fail(saved);
    };
    let Some(memfd_create) = next_memfd_create() else {
        return fail(saved);
    };
    let name = b"podvm-mounts\0";
    // The only flag this object ever sets; the request's own close-on-exec
    // bit is reconciled after the serve so the answer stays exact.
    const MFD_CLOEXEC: c_uint = 1;
    let m = unsafe { memfd_create(name.as_ptr() as *const c_char, MFD_CLOEXEC) };
    if m < 0 {
        return fail(saved);
    }
    let close_quiet = |m: c_int| {
        if let Some(close) = next_close() {
            unsafe { close(m) };
        }
    };
    let Some(stat) = next_stat() else {
        close_quiet(m);
        return fail(saved);
    };
    let Some(statfs) = next_statfs() else {
        close_quiet(m);
        return fail(saved);
    };
    let slash = b"/\0";
    let mut st = [0u8; STAT_LEN];
    let mut fs = [0u8; STATFS_LEN];
    if unsafe {
        stat(
            slash.as_ptr() as *const c_char,
            st.as_mut_ptr() as *mut c_void,
        )
    } != 0
        || unsafe {
            statfs(
                slash.as_ptr() as *const c_char,
                fs.as_mut_ptr() as *mut c_void,
            )
        } != 0
    {
        close_quiet(m);
        return fail(saved);
    }
    let dev = u64::from_ne_bytes(st[0..8].try_into().unwrap_or([0u8; 8]));
    let f_type = u64::from_ne_bytes(fs[0..8].try_into().unwrap_or([0u8; 8]));
    let f_flags = u64::from_ne_bytes(
        fs[STATFS_FLAGS..STATFS_FLAGS + 8]
            .try_into()
            .unwrap_or([0u8; 8]),
    );
    let rw = f_flags & ST_RDONLY == 0;
    // ⚠ One reader beside the writers: the memo scan rewinds first like
    // `memo::lookup` does, and shares its exposure to a concurrent
    // writer's append (writes are whole records under `O_APPEND`; a scan
    // interleaved with one sees whole records or a torn tail it drops).
    let mut area = [0u8; 4096];
    let mut recs: [RecMount; 64] = [RecMount {
        src_start: 0,
        src_len: 0,
        tgt_start: 0,
        tgt_len: 0,
        ro: false,
    }; 64];
    let taken = unsafe { scan_mounts(memo, &mut area, &mut recs) };
    let mut out = [0u8; FIXTURE_MAX];
    let n = match kind {
        MountFile::Mounts => render_mounts(
            Fstype::of_magic(f_type),
            rw,
            &area,
            &recs[..taken],
            &mut out,
        ),
        MountFile::MountInfo => render_mountinfo(
            major_of(dev),
            minor_of(dev),
            Fstype::of_magic(f_type),
            rw,
            &area,
            &recs[..taken],
            &mut out,
        ),
    };
    if !unsafe { write_all(m, &out[..n]) } || !unsafe { rewind(m) } {
        close_quiet(m);
        return fail(saved);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 5, 0, 0) {
        let e = errno();
        close_quiet(m);
        set_errno(e);
        return None;
    }
    // The serve is close-on-exec; the request may not have asked for it.
    // A descriptor flag, not a description one, so reconciling it touches
    // nothing shared.
    if flags & crate::procfs::O_CLOEXEC == 0 && !unsafe { set_cloexec(m, false) } {
        let e = errno();
        close_quiet(m);
        set_errno(e);
        return None;
    }
    Some(m)
}

/// The image-side link a `/dev` spelling resolves through.
///
/// `/proc/self` leaves are kernel-named and need no image path, so this
/// answers true for them without a call. The `/dev` spellings resolve
/// through an image link (`/dev/fd`, `/dev/stdin`, ...), and where the
/// image holds no such link the kernel answers `ENOENT`: the emulation
/// must not answer past a link the image does not have, so it checks
/// first and declines where the check fails, with the real errno
/// restored past the check itself.
unsafe fn proc_image_link_ok(path: &[u8]) -> bool {
    let link: &[u8] = if path.starts_with(b"/dev/fd/") {
        b"/dev/fd\0"
    } else if path == b"/dev/stdin\0" || path == b"/dev/stdout\0" || path == b"/dev/stderr\0" {
        path
    } else {
        return true;
    };
    let Some(lstat) = next_lstat() else {
        return false;
    };
    let mut st = [0u8; crate::procfs::STAT_LEN];
    if unsafe {
        lstat(
            link.as_ptr() as *const c_char,
            st.as_mut_ptr() as *mut c_void,
        )
    } != 0
    {
        set_errno(crate::procfs::ENOENT);
        return false;
    }
    true
}

/// What a standard stream link reads back: its target, exactly as the
/// kernel reports it on a live `/proc`. `None` for anything else.
pub(super) fn proc_stdio_target(path: &[u8]) -> Option<&'static [u8]> {
    if path == b"/dev/stdin\0" {
        Some(b"/proc/self/fd/0")
    } else if path == b"/dev/stdout\0" {
        Some(b"/proc/self/fd/1")
    } else if path == b"/dev/stderr\0" {
        Some(b"/proc/self/fd/2")
    } else {
        None
    }
}

/// Serve `open` of a `/proc/self` leaf after the real call failed, T-0413.
///
/// Only absolute spellings the matchers name, and only where the table
/// left the path untouched: a deliberate mapping wins over this
/// emulation, and the wrappers establish the untouched part by comparing
/// pointers before calling here.
pub(super) unsafe fn proc_emulate_open(
    path: *const c_char,
    flags: c_int,
    mode: c_uint,
) -> Option<c_int> {
    if path.is_null() {
        return None;
    }
    let n = crate::map::strnlen(path, crate::map::OUT);
    if n == 0 || n >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, n + 1) };
    // The `/dev` spellings resolve through an image link; the `/proc`
    // spellings are kernel-named. Nothing answers past a link the image
    // does not hold.
    if !unsafe { proc_image_link_ok(bytes) } {
        return None;
    }
    if let Some(fdno) = crate::procfs::fd_number(bytes) {
        return unsafe { proc_open_fd(fdno, flags) };
    }
    if crate::procfs::is_self_exe(bytes) {
        return unsafe { proc_open_exe(flags, mode) };
    }
    if let Some(kind) = crate::procfs::mount_file(bytes) {
        return unsafe { proc_open_mounts(kind, flags) };
    }
    None
}

/// Serve `readlink` of a `/proc/self` leaf after the real call failed,
/// T-0413. Same spelling rules as [`proc_emulate_open`].
pub(super) unsafe fn proc_emulate_readlink(
    path: *const c_char,
    buf: *mut c_char,
    n: usize,
) -> Option<isize> {
    if path.is_null() || buf.is_null() {
        return None;
    }
    let len = crate::map::strnlen(path, crate::map::OUT);
    if len == 0 || len >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, len + 1) };
    // The standard stream links read back their target, exactly as the
    // kernel reports them: they are links, not leaves.
    if let Some(target) = proc_stdio_target(bytes) {
        if !unsafe { proc_image_link_ok(bytes) } {
            return None;
        }
        let take = core::cmp::min(target.len(), n);
        unsafe {
            core::ptr::copy_nonoverlapping(target.as_ptr(), buf as *mut u8, take);
        }
        if !crate::emulate::tally(crate::emulate::OP_PROC, 2, 0, 0) {
            set_errno(crate::procfs::ENOENT);
            return Some(-1);
        }
        return Some(take as isize);
    }
    if !unsafe { proc_image_link_ok(bytes) } {
        return None;
    }
    if let Some(fdno) = crate::procfs::fd_number(bytes) {
        return unsafe { proc_readlink_fd(fdno, buf, n) };
    }
    if crate::procfs::is_self_exe(bytes) {
        return unsafe { proc_readlink_exe(buf, n) };
    }
    None
}

// --------------------------------------- the emulated operations, T-0708
//
// Four calls the runtime cannot honour, counted in the memo file beside
// the ownership records (`emulate`, `inspect` reading them back). The one
// rule across all four: no tally behind it, no emulation. Where the memo
// descriptor was not handed, each falls back to the real call and its
// honest failure instead of reporting an uncounted success.

/// Linux `O_*` for the mknod stand-in, UAPI `asm-generic/fcntl.h`.
/// Arch-generic on Linux.
pub(super) const O_WRONLY: c_int = 0o1;
pub(super) const O_CREAT: c_int = 0o100;
pub(super) const O_EXCL: c_int = 0o200;

/// File-type bits, UAPI `linux/stat.h`. A directory is refused with the
/// real call's answer; everything else becomes the regular file the
/// specification names.
const S_IFMT: c_uint = 0o170000;
const S_IFDIR: c_uint = 0o40000;

/// Say the `CLONE_NEWNET` strip out loud, T-0708's own rule. An empty netns
/// is `ENETUNREACH` for every connection, which reads as a network outage
/// rather than as a stripped flag, so the strip is never silent.
pub(super) fn say_newnet(what: &[u8], flags: u32) {
    crate::say::line(&[
        what,
        b": stripping CLONE_NEWNET from flags=",
        crate::say::Num::new(flags as u64).as_bytes(),
        b": no new network namespace is made, so every connection uses \
          the host network (TODO/interpose.md T-0708)",
    ]);
}

/// Create the regular-file stand-in for `mknod`, T-0708.
///
/// The path is prepared, a directory is refused, `O_EXCL` answers the
/// existing file honestly, and the mode's permission bits ride through
/// `open` (which applies the umask exactly as the real call would). The
/// device is ignored: there is no node to put it in.
///
/// Returns 0, or -1 with the kernel's errno.
///
/// # Safety
/// `path` must be readable up to its NUL within `OUT` bytes.
unsafe fn mknod_make(path: *const c_char, mode: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"mknod", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    if mode & S_IFMT == S_IFDIR {
        set_errno(EPERM);
        return -1;
    }
    let Some(open) = next_open() else {
        set_errno(EINVAL);
        return -1;
    };
    let Some(close) = next_close() else {
        set_errno(EINVAL);
        return -1;
    };
    let fd = unsafe { open(p, O_WRONLY | O_CREAT | O_EXCL, mode & 0o7777) };
    if fd < 0 {
        return -1;
    }
    unsafe { close(fd) };
    // No tally behind it, no success: the tally is what counts this
    // emulation, so one that does not land (a memo write failing under a
    // full disk) unmakes the stand-in and fails with the write's own errno
    // rather than reporting a file nothing counted.
    if crate::emulate::tally(crate::emulate::OP_MKNOD, mode, 0, 0) {
        return 0;
    }
    let e = errno();
    if let Some(unlink) = next_unlink() {
        unsafe { unlink(p) };
    }
    set_errno(e);
    -1
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mknod(path: *const c_char, mode: c_uint, dev: u64) -> c_int {
    let _ = dev;
    // No tally behind it, no emulation: the real call and its honest
    // failure (this runtime refuses the node) rather than an uncounted
    // success.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_mknod() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(path, mode, dev) };
    }
    unsafe { mknod_make(path, mode) }
}

/// # Safety
/// The payload's own contract for this entry point: `__xmknod` is the
/// spelling glibc before 2.33 compiles `mknod` to, version first.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __xmknod(
    ver: c_int,
    path: *const c_char,
    mode: c_uint,
    dev: *const c_void,
) -> c_int {
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_xmknod() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(ver, path, mode, dev) };
    }
    unsafe { mknod_make(path, mode) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mount(
    source: *const c_char,
    target: *const c_char,
    fstype: *const c_char,
    flags: c_ulong,
    data: *const c_void,
) -> c_int {
    let _ = fstype;
    let _ = data;
    // No tally behind it, no emulation.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_mount() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(source, target, fstype, flags, data) };
    }
    let mut tbuf = [0u8; crate::map::OUT];
    let t = unsafe { crate::map::prepare(b"mount", crate::map::AT_FDCWD, target, &mut tbuf) };
    if t.is_null() {
        return -1;
    }
    // The source is often not a path at all (`proc`, `tmpfs`, `none`): an
    // absolute one is prepared, anything else travels as the caller said it.
    // ⛔ `sbuf` lives to the end of the function because the tally below
    // reads through `s`: returning through a narrower scope would borrow a
    // dead buffer.
    let mut sbuf = [0u8; crate::map::OUT];
    let s = if !source.is_null() && unsafe { *(source as *const u8) } == b'/' {
        let s = unsafe { crate::map::prepare(b"mount", crate::map::AT_FDCWD, source, &mut sbuf) };
        if s.is_null() {
            return -1;
        }
        s
    } else {
        source
    };
    let sn = if s.is_null() {
        0
    } else {
        crate::map::strnlen(s, crate::map::OUT)
    };
    let tn = crate::map::strnlen(t, crate::map::OUT);
    if sn >= crate::map::OUT || tn >= crate::map::OUT {
        set_errno(ENAMETOOLONG);
        return -1;
    }
    // Mount flags fit 32 bits (every `MS_*` is one), so the cast keeps them.
    if unsafe { crate::emulate::tally_mount(flags as u32, s as *const u8, sn, t as *const u8, tn) }
    {
        return 0;
    }
    let Some(f) = next_mount() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { f(source, target, fstype, flags, data) }
}
