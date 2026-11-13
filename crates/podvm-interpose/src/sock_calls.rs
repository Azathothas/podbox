//! A Unix socket path takes the path table, T-1710.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-1710. A payload
//! names a Unix socket by a guest path in `sockaddr_un.sun_path`. Without
//! a rewrite the kernel resolves it on the host root: a `bind` makes the
//! socket outside the rootfs, and a `connect` reaches a host socket by
//! its guest name. Thus:
//!
//! * `bind`, `connect`, `sendto` and `sendmsg` rewrite a path address as
//!   `open` rewrites a path. `bind` makes a node, so a read-only prefix
//!   of `PODVM_RO` answers `EROFS` for it.
//! * where the host path does not fit the 108 bytes of `sun_path`, the
//!   call opens the parent directory with `O_PATH` and names the socket
//!   `/proc/self/fd/N/NAME`, then closes the descriptor.
//! * `getsockname`, `getpeername`, `accept`, `accept4`, `recvfrom` and
//!   `recvmsg` answer the guest name of a host path under a TO side.
//!
//! A socket bound through the long form keeps that name in the kernel;
//! [`names`] keeps its guest name for the answers of this process.
//!
//! An abstract name (a first byte of NUL) and an unnamed address are not
//! paths and pass unchanged. A raw system call passes the table (T-1652):
//! the interposer stays not a boundary.
//!
//! ⛔ No allocation: each buffer is on the caller's stack.

use core::ffi::{c_char, c_int, c_long, c_void};

use crate::map::{AT_FDCWD, OUT};
use crate::raw_calls::forward;

mod names;

crate::real!(pub fn next_bind = "bind"(c_int, *const c_void, u32) -> c_int);
crate::real!(pub fn next_connect = "connect"(c_int, *const c_void, u32) -> c_int);
crate::real!(pub fn next_sendto = "sendto"(c_int, *const c_void, usize, c_int, *const c_void, u32) -> isize);
crate::real!(pub fn next_sendmsg = "sendmsg"(c_int, *const c_void, c_int) -> isize);
crate::real!(pub fn next_getsockname = "getsockname"(c_int, *mut c_void, *mut u32) -> c_int);
crate::real!(pub fn next_getpeername = "getpeername"(c_int, *mut c_void, *mut u32) -> c_int);
crate::real!(pub fn next_accept = "accept"(c_int, *mut c_void, *mut u32) -> c_int);
crate::real!(pub fn next_accept4 = "accept4"(c_int, *mut c_void, *mut u32, c_int) -> c_int);
crate::real!(pub fn next_recvfrom = "recvfrom"(c_int, *mut c_void, usize, c_int, *mut c_void, *mut u32) -> isize);
crate::real!(pub fn next_recvmsg = "recvmsg"(c_int, *mut c_void, c_int) -> isize);

const AF_UNIX: u16 = 1;
/// The bytes of `sun_path`.
pub const SUN_PATH: usize = 108;
/// The offset of `sun_path`: after `sun_family`.
const PATH_AT: usize = 2;
/// `sizeof(struct sockaddr_un)`.
const SUN_LEN: usize = PATH_AT + SUN_PATH;
use crate::raw_calls::{SYS_CLOSE, SYS_OPENAT};
// T-1744: O_DIRECTORY differs on aarch64.
const O_PATH_DIR: c_long =
    (crate::oflags::O_PATH | crate::oflags::O_DIRECTORY | crate::oflags::O_CLOEXEC) as c_long;
const EINVAL: c_int = 22;
const ENAMETOOLONG: c_int = 36;

/// A rewritten address, on the caller's stack.
pub struct Addr {
    sun: [u8; SUN_LEN],
    len: u32,
    /// The parent directory of the long form, closed on drop.
    dir: c_int,
}

impl Drop for Addr {
    fn drop(&mut self) {
        if self.dir >= 0 {
            let e = crate::errno();
            unsafe { forward(SYS_CLOSE, [self.dir as c_long, 0, 0, 0, 0, 0]) };
            crate::set_errno(e);
        }
    }
}

impl Addr {
    pub fn ptr(&self) -> *const c_void {
        self.sun.as_ptr() as *const c_void
    }
    pub fn size(&self) -> u32 {
        self.len
    }
}

/// The path of a `sockaddr_un` at `addr`: `Some(bytes)` for a named Unix
/// address, `None` for any other family, an abstract name, or none.
///
/// # Safety
/// `addr` must be readable for `len` bytes.
unsafe fn unix_path<'a>(addr: *const c_void, len: u32) -> Option<&'a [u8]> {
    let len = len as usize;
    if addr.is_null() || len <= PATH_AT || len > SUN_LEN + 1 {
        return None;
    }
    let b = unsafe { core::slice::from_raw_parts(addr as *const u8, len) };
    if u16::from_ne_bytes([b[0], b[1]]) != AF_UNIX || b[PATH_AT] == 0 {
        return None;
    }
    let p = &b[PATH_AT..];
    Some(&p[..p.iter().position(|&c| c == 0).unwrap_or(p.len())])
}

/// Write `path` as a `sockaddr_un` into `a`, with its NUL.
fn put(a: &mut Addr, path: &[u8]) {
    a.sun[..PATH_AT].copy_from_slice(&AF_UNIX.to_ne_bytes());
    a.sun[PATH_AT..PATH_AT + path.len()].copy_from_slice(path);
    a.sun[PATH_AT + path.len()] = 0;
    a.len = (PATH_AT + path.len() + 1) as u32;
}

impl Addr {
    /// The path that this address names, without its NUL.
    fn path(&self) -> &[u8] {
        &self.sun[PATH_AT..self.len as usize - 1]
    }
}

/// The long form of a host path: `/proc/self/fd/N/NAME` with `N` the
/// parent directory opened with `O_PATH`. `Err` with the errno.
fn long_form(a: &mut Addr, host: &[u8]) -> Result<(), c_int> {
    let cut = host.iter().rposition(|&c| c == b'/').ok_or(ENAMETOOLONG)?;
    let name = &host[cut + 1..];
    let mut parent = [0u8; OUT];
    let pn = cut.max(1);
    parent[..pn].copy_from_slice(&host[..pn]);
    let fd = unsafe {
        forward(
            SYS_OPENAT,
            [
                AT_FDCWD as c_long,
                parent.as_ptr() as c_long,
                O_PATH_DIR,
                0,
                0,
                0,
            ],
        )
    };
    if fd < 0 {
        return Err(crate::errno());
    }
    a.dir = fd as c_int;
    let mut via = [0u8; SUN_PATH];
    let n = crate::exec::fd_path(b"/proc/self/fd/", a.dir, &mut via);
    if n + 1 + name.len() >= SUN_PATH {
        return Err(ENAMETOOLONG);
    }
    via[n] = b'/';
    via[n + 1..n + 1 + name.len()].copy_from_slice(name);
    let total = n + 1 + name.len();
    put(a, &via[..total]);
    Ok(())
}

/// Rewrite a path address for the call `what`. `Ok(None)`: pass the
/// caller's address unchanged.
///
/// # Safety
/// `addr` must be readable for `len` bytes.
pub unsafe fn rewrite(
    what: &[u8],
    addr: *const c_void,
    len: u32,
    writes: bool,
) -> Result<Option<Addr>, c_int> {
    let Some(path) = (unsafe { unix_path(addr, len) }) else {
        return Ok(None);
    };
    if unsafe { crate::map::table() }.n == 0 {
        return Ok(None);
    }
    let mut guest = [0u8; OUT];
    if path.len() >= OUT {
        return Ok(None);
    }
    guest[..path.len()].copy_from_slice(path);
    let mut buf = [0u8; OUT];
    let p = unsafe {
        crate::map::prepare_mode(
            what,
            AT_FDCWD,
            guest.as_ptr() as *const c_char,
            writes,
            &mut buf,
        )
    };
    if p.is_null() {
        return Err(crate::errno());
    }
    let n = crate::map::strnlen(p, OUT);
    let host = unsafe { core::slice::from_raw_parts(p as *const u8, n) };
    if host == path {
        return Ok(None);
    }
    let mut a = Addr {
        sun: [0; SUN_LEN],
        len: 0,
        dir: -1,
    };
    if n < SUN_PATH {
        put(&mut a, host);
    } else {
        long_form(&mut a, host)?;
    }
    Ok(Some(a))
}

/// Reverse-map the address that a call answered in place: a host path
/// under a TO side becomes its guest name, where the caller's buffer
/// takes it. `cap` is the caller's buffer size; `*lenp` the kernel's
/// length.
///
/// # Safety
/// `addr` must be writable for `cap` bytes and `lenp` valid.
pub unsafe fn answer(addr: *mut c_void, cap: u32, lenp: *mut u32) {
    if addr.is_null() || lenp.is_null() {
        return;
    }
    // A truncated answer holds a cut path: it stays as the kernel gave it.
    let len = unsafe { *lenp };
    if len > cap {
        return;
    }
    let Some(path) = (unsafe { unix_path(addr, len) }) else {
        return;
    };
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return;
    }
    let mut guest = [0u8; OUT];
    let mut kept = [0u8; SUN_PATH];
    let g = match names::find(path, &mut kept) {
        Some(n) => {
            guest[..n].copy_from_slice(&kept[..n]);
            n as isize
        }
        None => unsafe { crate::map::unrewrite(&t, path.as_ptr(), path.len(), &mut guest) },
    };
    if g <= 0 || g as usize >= SUN_PATH {
        return;
    }
    let g = g as usize;
    let full = (PATH_AT + g + 1) as u32;
    let b = addr as *mut u8;
    let room = (cap as usize).saturating_sub(PATH_AT);
    let copy = g.min(room);
    unsafe {
        core::ptr::copy_nonoverlapping(guest.as_ptr(), b.add(PATH_AT), copy);
        if copy < room {
            *b.add(PATH_AT + copy) = 0;
        }
        *lenp = full;
    }
}

macro_rules! addr_call {
    ($name:ident, $next:ident, $writes:expr) => {
        /// # Safety
        /// The payload's own contract for this socket call.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(fd: c_int, addr: *const c_void, len: u32) -> c_int {
            let Some(f) = $next() else {
                crate::set_errno(EINVAL);
                return -1;
            };
            match unsafe { rewrite(stringify!($name).as_bytes(), addr, len, $writes) } {
                Ok(None) => unsafe { f(fd, addr, len) },
                Ok(Some(a)) => unsafe { f(fd, a.ptr(), a.size()) },
                Err(e) => {
                    crate::set_errno(e);
                    -1
                }
            }
        }
    };
}

addr_call!(connect, next_connect, false);

/// `bind(2)`. A bind through the long form keeps its guest name.
///
/// # Safety
/// The payload's own contract for `bind(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(fd: c_int, addr: *const c_void, len: u32) -> c_int {
    let Some(f) = next_bind() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    match unsafe { rewrite(b"bind", addr, len, true) } {
        Ok(None) => unsafe { f(fd, addr, len) },
        Ok(Some(a)) => {
            let r = unsafe { f(fd, a.ptr(), a.size()) };
            if r == 0
                && a.dir >= 0
                && let Some(guest) = unsafe { unix_path(addr, len) }
            {
                names::keep(a.path(), guest);
            }
            r
        }
        Err(e) => {
            crate::set_errno(e);
            -1
        }
    }
}

/// # Safety
/// The payload's own contract for `sendto(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendto(
    fd: c_int,
    buf: *const c_void,
    n: usize,
    flags: c_int,
    addr: *const c_void,
    len: u32,
) -> isize {
    let Some(f) = next_sendto() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    match unsafe { rewrite(b"sendto", addr, len, false) } {
        Ok(None) => unsafe { f(fd, buf, n, flags, addr, len) },
        Ok(Some(a)) => unsafe { f(fd, buf, n, flags, a.ptr(), a.size()) },
        Err(e) => {
            crate::set_errno(e);
            -1
        }
    }
}

/// `struct msghdr` of x86_64: `msg_name` at 0, `msg_namelen` at 8, 56
/// bytes in all.
#[repr(C)]
#[derive(Clone, Copy)]
struct MsgHdr {
    name: *mut c_void,
    namelen: u32,
    rest: [u64; 5],
}

/// # Safety
/// The payload's own contract for `sendmsg(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendmsg(fd: c_int, msg: *const c_void, flags: c_int) -> isize {
    let Some(f) = next_sendmsg() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    if msg.is_null() {
        return unsafe { f(fd, msg, flags) };
    }
    let mut h = unsafe { *(msg as *const MsgHdr) };
    match unsafe { rewrite(b"sendmsg", h.name, h.namelen, false) } {
        Ok(None) => unsafe { f(fd, msg, flags) },
        Ok(Some(a)) => {
            h.name = a.ptr() as *mut c_void;
            h.namelen = a.size();
            unsafe { f(fd, &h as *const MsgHdr as *const c_void, flags) }
        }
        Err(e) => {
            crate::set_errno(e);
            -1
        }
    }
}

macro_rules! name_call {
    ($name:ident, $next:ident) => {
        /// # Safety
        /// The payload's own contract for this socket call.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(fd: c_int, addr: *mut c_void, lenp: *mut u32) -> c_int {
            let Some(f) = $next() else {
                crate::set_errno(EINVAL);
                return -1;
            };
            let cap = if lenp.is_null() { 0 } else { unsafe { *lenp } };
            let r = unsafe { f(fd, addr, lenp) };
            if r >= 0 {
                unsafe { answer(addr, cap, lenp) };
            }
            r
        }
    };
}

name_call!(getsockname, next_getsockname);
name_call!(getpeername, next_getpeername);
name_call!(accept, next_accept);

/// # Safety
/// The payload's own contract for `accept4(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn accept4(
    fd: c_int,
    addr: *mut c_void,
    lenp: *mut u32,
    flags: c_int,
) -> c_int {
    let Some(f) = next_accept4() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    let cap = if lenp.is_null() { 0 } else { unsafe { *lenp } };
    let r = unsafe { f(fd, addr, lenp, flags) };
    if r >= 0 {
        unsafe { answer(addr, cap, lenp) };
    }
    r
}

/// # Safety
/// The payload's own contract for `recvfrom(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvfrom(
    fd: c_int,
    buf: *mut c_void,
    n: usize,
    flags: c_int,
    addr: *mut c_void,
    lenp: *mut u32,
) -> isize {
    let Some(f) = next_recvfrom() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    let cap = if lenp.is_null() { 0 } else { unsafe { *lenp } };
    let r = unsafe { f(fd, buf, n, flags, addr, lenp) };
    if r >= 0 {
        unsafe { answer(addr, cap, lenp) };
    }
    r
}

/// # Safety
/// The payload's own contract for `recvmsg(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvmsg(fd: c_int, msg: *mut c_void, flags: c_int) -> isize {
    let Some(f) = next_recvmsg() else {
        crate::set_errno(EINVAL);
        return -1;
    };
    let cap = if msg.is_null() {
        0
    } else {
        unsafe { (*(msg as *const MsgHdr)).namelen }
    };
    let r = unsafe { f(fd, msg, flags) };
    if r >= 0 && !msg.is_null() {
        let h = msg as *mut MsgHdr;
        unsafe { answer((*h).name, cap, &mut (*h).namelen) };
    }
    r
}
