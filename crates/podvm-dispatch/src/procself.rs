//! The links of `/proc/self` whose answer is a path: the guest sees the
//! guest's path.

use crate::sys::{nr, sc};

/// The number after `/proc/`: `self`, `thread-self`, or the pid of the
/// process. The rest of the path, or None.
fn own(g: &[u8]) -> Option<&[u8]> {
    let rest = g.strip_prefix(b"/proc/")?;
    let cut = rest.iter().position(|&c| c == b'/')?;
    let (who, tail) = rest.split_at(cut);
    if who == b"self" || who == b"thread-self" {
        return Some(tail);
    }
    // SAFETY: getpid reads no memory.
    let pid = unsafe { sc(nr::GETPID, 0, 0, 0) };
    let mut v = 0i64;
    for &c in who {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v.wrapping_mul(10).wrapping_add((c - b'0') as i64);
    }
    (v == pid).then_some(tail)
}

/// `/proc/self/exe`: the payload's own guest path.
pub fn is_exe(g: &[u8]) -> bool {
    own(g) == Some(b"/exe")
}

/// `/proc/self/cwd`, `/proc/self/root` and `/proc/self/fd/N`: host paths
/// that point into the root.
pub fn is_self_link(g: &[u8]) -> bool {
    match own(g) {
        Some(t) => t == b"/cwd" || t == b"/root" || t.starts_with(b"/fd/"),
        None => false,
    }
}
