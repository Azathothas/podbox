//! The base of a relative path, read back in guest names, T-1643.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1643. The real
//! working directory and the target of a directory descriptor are host
//! paths. podvm starts the payload with its working directory inside the
//! rootfs, so each base already carries the TO side of a pair. The join
//! that `absolutize` makes goes to `rewrite`, and `rewrite` adds the TO
//! side. Thus the base must be a guest path before the join, or the
//! rootfs prefix comes in two times and each relative path misses.
//!
//! ⛔ No allocation and no lock: each buffer is on the caller's stack.

use super::*;

/// Read the target of `/proc/self/fd/<dirfd>` into `base`.
///
/// Returns the length, or `-1` where the link cannot be read or does not
/// name an absolute path (a socket or a pipe names `socket:[n]`, and a
/// join under that text would match no pair).
///
/// # Safety
/// `readlink` must be the real call.
pub(super) unsafe fn fd_base(
    readlink: unsafe extern "C" fn(*const c_char, *mut c_char, usize) -> isize,
    dirfd: c_int,
    base: &mut [u8; OUT],
) -> isize {
    // `/proc/self/fd/<n>` in a 64-byte stack buffer: the longest `c_int`
    // is 11 digits plus the sign.
    let mut link = [0u8; 64];
    let head = b"/proc/self/fd/";
    link[..head.len()].copy_from_slice(head);
    let mut num = [0u8; 12];
    let neg = dirfd < 0;
    let mut v = (dirfd as i64).unsigned_abs();
    let mut d = 0usize;
    loop {
        num[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
        if v == 0 {
            break;
        }
    }
    let mut at = head.len();
    if neg {
        link[at] = b'-';
        at += 1;
    }
    while d > 0 {
        d -= 1;
        link[at] = num[d];
        at += 1;
    }
    link[at] = 0;
    let r = unsafe {
        readlink(
            link.as_ptr() as *const c_char,
            base.as_mut_ptr() as *mut c_char,
            OUT - 1,
        )
    };
    if r <= 0 || r as usize >= OUT || base[0] != b'/' {
        return -1;
    }
    r
}

/// Reverse-map a host base into guest names in place, T-1643.
///
/// A base under a TO side becomes its FROM side. A base under no TO side
/// stays as it is: the interposed `getcwd` shows the same host path to the
/// payload, so the payload and the join agree. Returns the new length, or
/// `-1` where the guest name does not fit.
///
/// # Safety
/// `base` must hold `base_n` readable bytes.
pub(super) unsafe fn guest_base(t: &Table, base: &mut [u8; OUT], base_n: usize) -> isize {
    let mut guest = [0u8; OUT];
    match unsafe { unrewrite(t, base.as_ptr(), base_n, &mut guest) } {
        -1 => base_n as isize,
        n if n < 0 => -1,
        n => {
            base[..n as usize].copy_from_slice(&guest[..n as usize]);
            n
        }
    }
}

/// Normalise the absolute path in `p[..n]` in place, T-1643.
///
/// Remove each `.` component, apply each `..` to the component before it,
/// and fold each run of `/` into one, before the table match. A `..` at the
/// root stays at the root, as the kernel does under a real root. Without
/// this, `/tmp/../etc` matches the `/tmp` pair, and `/../etc` under the `/`
/// pair names the parent of the rootfs. A trailing `/`, or a last `.` or
/// `..`, keeps one trailing `/`: the kernel then still requires a
/// directory there.
///
/// ⚠ The rule is lexical. A `..` after a symbolic link goes to the parent
/// of the link, not to the parent of its target. Inside the rootfs the
/// kernel cannot resolve an absolute link target correctly in either case.
///
/// Returns the new length. The output is never longer than the input, so
/// each write lands at or before the byte that the scan reads next.
pub fn normalise(p: &mut [u8; OUT], n: usize) -> usize {
    debug_assert!(n > 0 && p[0] == b'/', "normalise takes an absolute path");
    let mut w = 1usize;
    let mut r = 0usize;
    let mut dir_tail = false;
    while r < n {
        while r < n && p[r] == b'/' {
            r += 1;
        }
        if r >= n {
            break;
        }
        let start = r;
        while r < n && p[r] != b'/' {
            r += 1;
        }
        let len = r - start;
        dir_tail = r >= n && (len <= 2 && p[start..r].iter().all(|&b| b == b'.'));
        if len == 1 && p[start] == b'.' {
            continue;
        }
        if len == 2 && p[start] == b'.' && p[start + 1] == b'.' {
            // Back to the separator before the last component.
            while w > 1 && p[w - 1] != b'/' {
                w -= 1;
            }
            if w > 1 {
                w -= 1;
            }
            continue;
        }
        if w > 1 {
            p[w] = b'/';
            w += 1;
        }
        p.copy_within(start..r, w);
        w += len;
    }
    if (dir_tail || p[n - 1] == b'/') && w > 1 {
        p[w] = b'/';
        w += 1;
    }
    w
}
