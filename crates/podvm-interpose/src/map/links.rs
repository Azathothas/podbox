//! TODO/interpose.md T-1725. A path of a path call resolves its links in
//! guest names before the real call.
//!
//! ⛔ The table rewrites a path string; the kernel then follows each link
//! of the host path from the host's root. An absolute target is a guest
//! path: `/home/link -> /opt/f` of an image opened `/opt/f` of the host
//! (measured, PodUtils/podrunner#61). So the funnel walks the path in guest
//! names with the walk of the exec routes (T-1646): each directory, and the
//! last component where the call follows it. A call that does not follow
//! its last link keeps that name.

use super::*;
use crate::exec::walk;

/// The calls that do not follow the link of their last component. Each
/// other call follows it, unless its flags say no ([`prepare_follow`]).
const NOFOLLOW: &[&[u8]] = &[
    b"lstat",
    b"lstat64",
    b"__lxstat",
    b"__lxstat64",
    b"lchown",
    b"readlink",
    b"readlinkat",
    b"unlink",
    b"unlinkat",
    b"rmdir",
    b"remove",
    b"rename",
    b"renameat",
    b"renameat2",
    b"link",
    b"linkat",
    b"symlink",
    b"symlinkat",
    b"lsetxattr",
    b"lgetxattr",
    b"llistxattr",
    b"lremovexattr",
    b"lutimes",
    b"mkdir",
    b"mkdirat",
    b"mknod",
    b"mknodat",
    b"mkfifo",
    b"mkfifoat",
    b"bind",
];

/// Whether the call `what` follows the link of its last component.
pub fn follows(what: &[u8]) -> bool {
    !NOFOLLOW.contains(&what)
}

/// The host path of the absolute guest path `g` into `out` (with a NUL),
/// each link resolved in guest names: all of it where `follow`, else its
/// directory, with the last name kept. The length, or an errno.
///
/// # Safety
/// Calls the real `readlink`.
pub unsafe fn resolve_links(
    t: &Table,
    g: &[u8],
    follow: bool,
    out: &mut [u8; OUT],
) -> Result<usize, c_int> {
    let mut guest = [0u8; OUT];
    let mut end = g.len();
    while end > 1 && g[end - 1] == b'/' {
        end -= 1;
    }
    let cut = g[..end].iter().rposition(|&c| c == b'/').unwrap_or(0);
    let last = &g[cut + 1..end];
    if follow || last.is_empty() || last == b"." || last == b".." {
        let (_, hn) = unsafe { walk::resolve(t, g, &mut guest, out) }?;
        return Ok(hn);
    }
    let parent = if cut == 0 { &g[..1] } else { &g[..cut] };
    let (_, hn) = unsafe { walk::resolve(t, parent, &mut guest, out) }?;
    let mut at = hn;
    if at + 1 + last.len() >= OUT {
        return Err(ENAMETOOLONG);
    }
    if at == 0 || out[at - 1] != b'/' {
        out[at] = b'/';
        at += 1;
    }
    out[at..at + last.len()].copy_from_slice(last);
    at += last.len();
    out[at] = 0;
    Ok(at)
}
