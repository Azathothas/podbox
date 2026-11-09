//! A lookup that misses on the mapped path tries the rootfs walk once,
//! T-1646.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1646 (issue 127:
//! absolute in-image link targets). A shell finds a command with `stat`, and
//! alpine's `/bin/chmod` is a link to `/bin/busybox`: the kernel follows it on
//! the host root, where it names nothing, and the shell says `not found`
//! before it calls `execve`. The `stat` and `access` shapes that follow the
//! last link call [`again`] after `ENOENT` or `ENOTDIR`: where the walk of
//! the guest path inside the rootfs gives another host path, the call goes
//! once more with it.
//!
//! ⚠ A miss is the trigger, so a link whose target also exists on the host
//! still answers for the host file. The exec itself always walks; the
//! other path calls (`open` among them) do not yet.

use super::*;

/// The walked host path of `path` into `out`, where the earlier call
/// missed and the walk names another file than `first`.
///
/// # Safety
/// `path` and `first` must be NUL-terminated within `OUT` bytes.
pub unsafe fn again(
    dirfd: c_int,
    path: *const c_char,
    first: *const c_char,
    out: &mut [u8; OUT],
) -> Option<*const c_char> {
    let e = crate::errno();
    if (e != ENOENT && e != ENOTDIR) || path.is_null() || first.is_null() {
        return None;
    }
    let t = unsafe { crate::map::table() };
    if !anchored(&t) {
        return None;
    }
    let mut guest = [0u8; OUT];
    let mut resolved = [0u8; OUT];
    let gn = unsafe { crate::map::absolutize(&t, dirfd, path, &mut guest) };
    let walked = usize::try_from(gn)
        .ok()
        .filter(|&n| n > 0)
        .and_then(|gn| unsafe { walk::resolve(&t, &guest[..gn], &mut resolved, out) }.ok());
    let fn_ = crate::map::strnlen(first, OUT);
    let first = unsafe { core::slice::from_raw_parts(first as *const u8, fn_) };
    match walked {
        Some((_, hn)) if first != &out[..hn] => Some(out.as_ptr() as *const c_char),
        // The walk reads links, which sets `errno`: the caller's miss stays.
        _ => {
            crate::set_errno(e);
            None
        }
    }
}
