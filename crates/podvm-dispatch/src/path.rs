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
        let Some(guest) = strip_root(&buf[..n]) else {
            return Ok(Target::Host);
        };
        // The guest part is a suffix of buf: move it to the front.
        let (g0, glen) = (n - guest.len(), guest.len());
        buf.copy_within(g0..n, 0);
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
        Target::Root(n) => sys::openat2(state::root_fd(), &buf[..=n], flags, 0, RESOLVE_IN_ROOT),
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
        sys::openat2(state::root_fd(), &buf[..=plen], flags, 0, RESOLVE_IN_ROOT)
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
