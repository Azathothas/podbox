//! Resolve a guest path inside the rootfs, link by link, T-1646.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1646. The table
//! maps a guest path to a host path, but the kernel follows each symbolic
//! link of that host path on the host root. An absolute link target is a
//! guest path: alpine's `/bin/ls` points at `/bin/busybox`, and Debian's
//! `/lib64/ld-linux-x86-64.so.2` points at `/lib/x86_64-linux-gnu/...`. On
//! the host these name the host's own files or nothing. So the walk reads
//! each link itself and goes on in guest names, as
//! `podvm_enter::abi::resolve_in` does for the top-level payload
//! (`loader_plan`, TODO/enter.md T-1408).
//!
//! ⛔ No allocation and no lock: each buffer is on the caller's stack.

use super::*;

/// The link count after which the walk stops with `ELOOP`, as Linux does.
const MAX_LINKS: usize = 40;

const ELOOP: c_int = 40;
const ENAMETOOLONG: c_int = 36;

/// The host path of the guest path `guest[..n]` into `host`, with a NUL.
/// `Ok(None)` is a path outside the table (the never-rewrite paths of
/// T-0707): the kernel resolves it, and it is its own host path.
fn host_of(t: &Table, guest: &[u8], host: &mut [u8; OUT]) -> Result<Option<usize>, c_int> {
    match unsafe { crate::map::rewrite(t, guest.as_ptr(), guest.len(), host) } {
        -1 => Ok(None),
        n if n < 0 => Err(ENAMETOOLONG),
        n => {
            host[n as usize] = 0;
            Ok(Some(n as usize))
        }
    }
}

/// Resolve the absolute guest path `g` inside the rootfs.
///
/// Writes the resolved guest path to `guest` and its host path to `host`,
/// each with a NUL, and returns the two lengths. Each component is checked
/// for a link on its host path; a link target goes on in guest names, so
/// an absolute target starts again at the guest root. `..` never goes above
/// the guest root. A component that is absent or not a link stays as it
/// is: the exec answers for it, as the kernel would. Under a path outside
/// the table (`/proc`) the walk follows no link: those links are the
/// kernel's own, and their text names host objects.
///
/// # Safety
/// Calls the real `readlink`.
pub unsafe fn resolve(
    t: &Table,
    g: &[u8],
    guest: &mut [u8; OUT],
    host: &mut [u8; OUT],
) -> Result<(usize, usize), c_int> {
    if g.is_empty() || g[0] != b'/' || g.len() >= OUT {
        return Err(ENAMETOOLONG);
    }
    let Some(readlink) = crate::map::next_readlink() else {
        return Err(EINVAL);
    };
    let mut pend = [0u8; OUT];
    let mut link = [0u8; OUT];
    let mut pn = g.len();
    pend[..pn].copy_from_slice(g);
    let mut pos = 0usize;
    guest[0] = b'/';
    let mut dn = 1usize;
    let mut links = 0usize;
    let mut literal = false;
    while pos < pn {
        while pos < pn && pend[pos] == b'/' {
            pos += 1;
        }
        if pos >= pn {
            break;
        }
        let start = pos;
        while pos < pn && pend[pos] != b'/' {
            pos += 1;
        }
        let len = pos - start;
        if len == 1 && pend[start] == b'.' {
            continue;
        }
        if len == 2 && pend[start] == b'.' && pend[start + 1] == b'.' {
            while dn > 1 && guest[dn - 1] != b'/' {
                dn -= 1;
            }
            if dn > 1 {
                dn -= 1;
            }
            continue;
        }
        let parent = dn;
        if dn > 1 {
            guest[dn] = b'/';
            dn += 1;
        }
        if dn + len >= OUT {
            return Err(ENAMETOOLONG);
        }
        guest[dn..dn + len].copy_from_slice(&pend[start..pos]);
        dn += len;
        if host_of(t, &guest[..dn], host)?.is_none() {
            // Outside the table: the rest goes as it is written.
            literal = true;
            break;
        }
        let r = unsafe {
            readlink(
                host.as_ptr() as *const c_char,
                link.as_mut_ptr() as *mut c_char,
                OUT - 1,
            )
        };
        if r <= 0 {
            continue;
        }
        links += 1;
        if links > MAX_LINKS {
            return Err(ELOOP);
        }
        // The target, then the components not yet walked.
        let ln = r as usize;
        let rest = pn - pos;
        if ln + rest >= OUT {
            return Err(ENAMETOOLONG);
        }
        link[ln..ln + rest].copy_from_slice(&pend[pos..pn]);
        pend[..ln + rest].copy_from_slice(&link[..ln + rest]);
        pn = ln + rest;
        pos = 0;
        dn = if pend[0] == b'/' { 1 } else { parent };
    }
    if literal && pos < pn {
        if dn + (pn - pos) >= OUT {
            return Err(ENAMETOOLONG);
        }
        guest[dn..dn + pn - pos].copy_from_slice(&pend[pos..pn]);
        dn += pn - pos;
    }
    guest[dn] = 0;
    let hn = match host_of(t, &guest[..dn], host)? {
        Some(n) => n,
        None => {
            host[..dn].copy_from_slice(&guest[..dn]);
            host[dn] = 0;
            dn
        }
    };
    Ok((dn, hn))
}
