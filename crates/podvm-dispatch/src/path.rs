//! The guest path of a call, and the host object that it names.
//!
//! A path in the root resolves with `openat2(ROOT_FD, path,
//! RESOLVE_IN_ROOT)`, so `..` and each symlink stay in the root. A path
//! under `/proc`, `/sys` or `/dev` is the host's. A relative path joins the
//! guest path of its directory: the cwd or the descriptor, read back from
//! `/proc/self`. A directory outside the root keeps the host's meaning.

use crate::state::{self, PATH_MAX, ROOT};
use crate::sys::{self, AT_FDCWD, O_CLOEXEC, O_PATH, RESOLVE_IN_ROOT};

/// The guest prefixes that are the host's.
const HOST: &[&[u8]] = &[b"/proc", b"/sys", b"/dev"];

/// Where a path of the payload goes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// Not in the root (a directory descriptor outside it): the call keeps
    /// its own `dirfd` and path.
    Host,
    /// A host path, absolute, of `len` bytes in the buffer.
    HostAbs(usize),
    /// A guest path, absolute, of `len` bytes in the buffer.
    Root(usize),
}

/// `/proc/self/fd/N`, or `/proc/self/cwd` for `AT_FDCWD`, with its NUL.
pub fn fd_link(fd: i64, out: &mut [u8; 32]) -> &[u8] {
    if fd == AT_FDCWD {
        return b"/proc/self/cwd\0";
    }
    let head = b"/proc/self/fd/";
    out[..head.len()].copy_from_slice(head);
    let mut n = head.len();
    let mut digits = [0u8; 20];
    let mut d = 0usize;
    let mut v = fd as u64;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        d += 1;
        v /= 10;
        if v == 0 || d == digits.len() {
            break;
        }
    }
    while d > 0 && n < out.len() - 1 {
        d -= 1;
        out[n] = digits[d];
        n += 1;
    }
    out[n] = 0;
    &out[..=n]
}

/// The guest path of a host path, where it is in the root: the part after
/// the root, or `/` for the root itself.
pub fn strip_root(host: &[u8]) -> Option<&[u8]> {
    let root = ROOT.get();
    if host == root {
        return Some(b"/");
    }
    if host.len() > root.len() && host.starts_with(root) && host[root.len()] == b'/' {
        return Some(&host[root.len()..]);
    }
    None
}

/// The descriptor that a guest path resolves under, and the index in
/// `guest` where the rest begins: a volume with the longest guest prefix,
/// else the root (TODO/enter.md T-1724).
pub fn base_of(guest: &[u8]) -> (i64, usize) {
    let mut best: Option<(&state::Vol, usize)> = None;
    for v in state::vols() {
        let g = v.guest.get();
        let hit =
            guest == g || (guest.len() > g.len() && guest.starts_with(g) && guest[g.len()] == b'/');
        if hit && best.is_none_or(|(_, n)| g.len() > n) {
            best = Some((v, g.len()));
        }
    }
    match best {
        Some((v, n)) => (v.fd.load(core::sync::atomic::Ordering::Relaxed), n),
        None => (state::root_fd(), 0),
    }
}

/// `openat2` of the guest path `buf[..n]` (a NUL at n) in the root or its
/// volume, with `RESOLVE_IN_ROOT` at that directory.
pub fn open_in(buf: &[u8], n: usize, flags: i64, mode: i64, resolve: u64) -> i64 {
    let (fd, at) = base_of(&buf[..n]);
    let rest = if at >= n { &b".\0"[..] } else { &buf[at..=n] };
    sys::openat2(fd, rest, flags, mode, resolve | RESOLVE_IN_ROOT)
}

/// The guest path of a host path into `out`: under a volume's host side
/// it is the volume's guest side and the rest; in the root it is the part
/// after the root. None where the host path is in neither.
pub fn guest_of_host(host: &[u8], out: &mut [u8]) -> Option<usize> {
    let under = |h: &[u8], p: &[u8]| {
        h == p || (h.len() > p.len() && h.starts_with(p) && h[p.len()] == b'/')
    };
    let mut best: Option<(&[u8], &[u8])> = None;
    for v in state::vols() {
        let hp = v.host.get();
        if under(host, hp) && best.is_none_or(|(b, _)| hp.len() > b.len()) {
            best = Some((hp, v.guest.get()));
        }
    }
    let (prefix, rest): (&[u8], &[u8]) = match best {
        Some((hp, gp)) => (gp, &host[hp.len()..]),
        None => {
            let g = strip_root(host)?;
            (&[], g)
        }
    };
    let total = prefix.len() + rest.len();
    if total >= out.len() {
        return None;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..total].copy_from_slice(rest);
    Some(total)
}

fn under_host(guest: &[u8]) -> bool {
    HOST.iter()
        .any(|p| guest.starts_with(p) && (guest.len() == p.len() || guest[p.len()] == b'/'))
}

/// The target of `(dirfd, path)`; the buffer holds the absolute path with a
/// NUL. Err is `-errno`.
pub fn target(dirfd: i64, path: &[u8], buf: &mut [u8; PATH_MAX]) -> Result<Target, i64> {
    let len = if path.first() == Some(&b'/') {
        if path.len() >= PATH_MAX {
            return Err(-sys::ENAMETOOLONG);
        }
        buf[..path.len()].copy_from_slice(path);
        path.len()
    } else {
        let mut link = [0u8; 32];
        let n = sys::readlinkat(AT_FDCWD, fd_link(dirfd, &mut link), &mut buf[..]);
        if n < 0 || n as usize >= PATH_MAX {
            // A bad descriptor gives the kernel's own error in the call.
            return Ok(Target::Host);
        }
        let n = n as usize;
        let Some(mut tmp) = crate::pool::Buf::get() else {
            return Err(-sys::ENOMEM);
        };
        let t = tmp.buf();
        let Some(glen) = guest_of_host(&buf[..n], &mut t[..]) else {
            return Ok(Target::Host);
        };
        buf[..glen].copy_from_slice(&t[..glen]);
        let mut at = glen;
        if path.is_empty() {
            // AT_EMPTY_PATH: the directory itself.
        } else {
            if at + 1 + path.len() >= PATH_MAX {
                return Err(-sys::ENAMETOOLONG);
            }
            if buf[at - 1] != b'/' {
                buf[at] = b'/';
                at += 1;
            }
            buf[at..at + path.len()].copy_from_slice(path);
            at += path.len();
        }
        at
    };
    buf[len] = 0;
    Ok(if under_host(&buf[..len]) {
        Target::HostAbs(len)
    } else {
        Target::Root(len)
    })
}

/// An `O_PATH` descriptor of the target, the last link followed or not.
pub fn open_path(dirfd: i64, path: &[u8], t: Target, buf: &[u8], follow: bool, dir: bool) -> i64 {
    let mut flags = O_PATH | O_CLOEXEC;
    if !follow {
        flags |= sys::O_NOFOLLOW;
    }
    if dir {
        flags |= sys::O_DIRECTORY;
    }
    match t {
        Target::Host => {
            let Some(mut g) = crate::pool::Buf::get() else {
                return -sys::ENOMEM;
            };
            let Some(p) = with_nul(path, g.buf()) else {
                return -sys::ENAMETOOLONG;
            };
            sys::openat2(dirfd, p, flags, 0, 0)
        }
        Target::HostAbs(n) => sys::openat2(AT_FDCWD, &buf[..=n], flags, 0, 0),
        Target::Root(n) => open_in(buf, n, flags, 0, 0),
    }
}

/// `path` with a NUL in `out`.
pub fn with_nul<'a>(path: &[u8], out: &'a mut [u8; PATH_MAX]) -> Option<&'a [u8]> {
    if path.len() >= PATH_MAX {
        return None;
    }
    out[..path.len()].copy_from_slice(path);
    out[path.len()] = 0;
    Some(&out[..=path.len()])
}

/// The parent of a target and its last name: a descriptor (to close where
/// `owned`) and the name, NUL-ended, in `name`.
pub struct Parent {
    pub fd: i64,
    pub owned: bool,
}

impl Drop for Parent {
    fn drop(&mut self) {
        if self.owned && self.fd >= 0 {
            sys::close(self.fd);
        }
    }
}

/// Split `buf[..len]` at its last name: the parent's `O_PATH` descriptor,
/// and the name copied into `name`. Trailing slashes go; a name of `.` or
/// `..`, or the root, gives the path itself and `.`. The caller passes a
/// [`Target::Host`] through before it comes here.
pub fn parent(t: Target, buf: &mut [u8; PATH_MAX], name: &mut [u8; 256]) -> Result<Parent, i64> {
    let (len, in_root) = match t {
        Target::Host => return Err(-sys::EINVAL),
        Target::HostAbs(n) => (n, false),
        Target::Root(n) => (n, true),
    };
    let mut end = len;
    while end > 1 && buf[end - 1] == b'/' {
        end -= 1;
    }
    let cut = buf[..end].iter().rposition(|&c| c == b'/').unwrap_or(0);
    let last = &buf[cut + 1..end];
    if last.len() >= name.len() {
        return Err(-sys::ENAMETOOLONG);
    }
    if last.is_empty() || last == b"." || last == b".." {
        name[0] = b'.';
        name[1] = 0;
        let fd = open_path(AT_FDCWD, &[], t, &buf[..], true, false);
        return if fd < 0 {
            Err(fd)
        } else {
            Ok(Parent { fd, owned: true })
        };
    }
    name[..last.len()].copy_from_slice(last);
    name[last.len()] = 0;
    // The parent's path: up to the cut, or "/" for a name under the root.
    let plen = if cut == 0 { 1 } else { cut };
    let saved = buf[plen];
    buf[plen] = 0;
    let flags = O_PATH | O_CLOEXEC | sys::O_DIRECTORY;
    let fd = if in_root {
        open_in(&buf[..], plen, flags, 0, 0)
    } else {
        sys::openat2(AT_FDCWD, &buf[..=plen], flags, 0, 0)
    };
    buf[plen] = saved;
    if fd < 0 {
        Err(fd)
    } else {
        Ok(Parent { fd, owned: true })
    }
}
