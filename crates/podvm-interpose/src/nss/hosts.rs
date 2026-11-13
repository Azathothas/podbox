//! `/etc/hosts` of the image, TODO/interpose.md T-1753.
//!
//! ⭐ The image's `/etc/hosts` answers first: `--add-host` writes it, and a
//! name that it has resolves to its address, as in a container. A name that
//! it does not have goes to libc, which asks DNS through the host's
//! `/etc/resolv.conf`.
//!
//! ⚠ libc also reads the host's `/etc/hosts` for such a name, so a name
//! that only the host's file has still resolves: the userland rung shares
//! the host's network, and the image has no resolver of its own.
//!
//! `getaddrinfo` builds no list itself: it gives libc the address of the
//! image's line as a numeric host, so the list is libc's and libc's
//! `freeaddrinfo` frees it. `ai_canonname` is then the address.

use super::*;
use core::net::IpAddr;

const AF_UNSPEC: c_int = 0;
const AF_INET: c_int = 2;
const AF_INET6: c_int = 10;
const AI_NUMERICHOST: c_int = 4;

/// `struct hostent` of glibc and musl.
#[repr(C)]
pub struct Hostent {
    name: *mut c_char,
    aliases: *mut *mut c_char,
    addrtype: c_int,
    length: c_int,
    addr_list: *mut *mut c_char,
}

/// `struct addrinfo` of glibc and musl, on x86_64 and aarch64.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AddrInfo {
    flags: c_int,
    family: c_int,
    socktype: c_int,
    protocol: c_int,
    addrlen: c_uint,
    addr: *mut c_void,
    canonname: *mut c_char,
    next: *mut AddrInfo,
}

/// The words of a line of /etc/hosts, without its comment.
fn words(line: &[u8]) -> impl Iterator<Item = &[u8]> + Clone {
    let line = line.split(|&b| b == b'#').next().unwrap_or(&[]);
    line.split(|b| b.is_ascii_whitespace())
        .filter(|w| !w.is_empty())
}

/// Call `f` with the address and the names of the first line of the image's
/// /etc/hosts that names `name` with an address of family `af`.
fn lookup<R>(name: &[u8], af: c_int, mut f: impl FnMut(IpAddr, &[u8]) -> R) -> Option<R> {
    if name.is_empty() {
        return None;
    }
    scan(b"/etc/hosts\0", |_, line| {
        let mut w = words(line);
        let addr = w.next()?;
        if !w.any(|n| n.eq_ignore_ascii_case(name)) {
            return None;
        }
        let ip: IpAddr = core::str::from_utf8(addr).ok()?.parse().ok()?;
        match (af, ip) {
            (AF_UNSPEC, _) | (AF_INET, IpAddr::V4(_)) | (AF_INET6, IpAddr::V6(_)) => {
                Some(f(ip, line))
            }
            _ => None,
        }
    })
}

/// Fill `he` with `ip` and the names of `line`, in the buffer.
fn fill(ip: IpAddr, line: &[u8], he: *mut Hostent, buf: *mut u8, len: usize) -> Result<(), c_int> {
    let mut b = Buf { p: buf, len, at: 0 };
    let mut put = || -> Option<Hostent> {
        let (octets, n, af): ([u8; 16], usize, c_int) = match ip {
            IpAddr::V4(a) => {
                let mut o = [0u8; 16];
                o[..4].copy_from_slice(&a.octets());
                (o, 4, AF_INET)
            }
            IpAddr::V6(a) => (a.octets(), 16, AF_INET6),
        };
        let addr = b.take(n, 8)?;
        unsafe { core::ptr::copy_nonoverlapping(octets.as_ptr(), addr, n) };
        let list = b.ptrs(1)?;
        unsafe { *list = addr as *mut c_char };
        let mut names = words(line).skip(1);
        let canon = b.str(names.next()?)?;
        let aliases = b.ptrs(names.clone().count())?;
        for (i, a) in names.enumerate() {
            let s = b.str(a)?;
            unsafe { *aliases.add(i) = s };
        }
        Some(Hostent {
            name: canon,
            aliases,
            addrtype: af,
            length: n as c_int,
            addr_list: list,
        })
    };
    match put() {
        Some(h) => {
            unsafe { he.write(h) };
            Ok(())
        }
        None => Err(ERANGE),
    }
}

static HE: Static<(Hostent, [u8; LINE])> = Static(UnsafeCell::new(unsafe { core::mem::zeroed() }));

crate::real!(fn next_gethostbyname = "gethostbyname"(*const c_char) -> *mut Hostent);
crate::real!(fn next_gethostbyname2 = "gethostbyname2"(*const c_char, c_int) -> *mut Hostent);
crate::real!(fn next_gethostbyname_r = "gethostbyname_r"(*const c_char, *mut Hostent, *mut c_char, usize, *mut *mut Hostent, *mut c_int) -> c_int);
crate::real!(fn next_gethostbyname2_r = "gethostbyname2_r"(*const c_char, c_int, *mut Hostent, *mut c_char, usize, *mut *mut Hostent, *mut c_int) -> c_int);
crate::real!(fn next_getaddrinfo = "getaddrinfo"(*const c_char, *const c_char, *const AddrInfo, *mut *mut AddrInfo) -> c_int);

/// The image's answer for a call without `_r`, in static storage.
fn static_answer(name: *const c_char, af: c_int) -> Option<*mut Hostent> {
    if !active() || !matches!(af, AF_INET | AF_INET6) {
        return None;
    }
    let s = HE.0.get();
    let (he, b) = unsafe { (&raw mut (*s).0, (&raw mut (*s).1) as *mut u8) };
    match lookup(bytes(name), af, |ip, line| fill(ip, line, he, b, LINE))? {
        Ok(()) => Some(he),
        Err(_) => None,
    }
}

/// The image's answer for a `_r` call, or `None` where the image's file
/// does not name `name`.
fn answer(
    name: *const c_char,
    af: c_int,
    he: *mut Hostent,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Hostent,
    h_errnop: *mut c_int,
) -> Option<c_int> {
    if !active() || !matches!(af, AF_INET | AF_INET6) {
        return None;
    }
    let r = lookup(bytes(name), af, |ip, line| {
        fill(ip, line, he, buf as *mut u8, len)
    })?;
    unsafe {
        *result = if r.is_ok() { he } else { core::ptr::null_mut() };
        if !h_errnop.is_null() {
            *h_errnop = 0;
        }
    }
    Some(r.err().unwrap_or(0))
}

/// # Safety
/// libc's contract for `gethostbyname`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname(name: *const c_char) -> *mut Hostent {
    if let Some(h) = static_answer(name, AF_INET) {
        return h;
    }
    next_gethostbyname().map_or(core::ptr::null_mut(), |f| unsafe { f(name) })
}

/// # Safety
/// libc's contract for `gethostbyname2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname2(name: *const c_char, af: c_int) -> *mut Hostent {
    if let Some(h) = static_answer(name, af) {
        return h;
    }
    next_gethostbyname2().map_or(core::ptr::null_mut(), |f| unsafe { f(name, af) })
}

/// # Safety
/// libc's contract for `gethostbyname_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname_r(
    name: *const c_char,
    he: *mut Hostent,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Hostent,
    h_errnop: *mut c_int,
) -> c_int {
    if let Some(r) = answer(name, AF_INET, he, buf, len, result, h_errnop) {
        return r;
    }
    next_gethostbyname_r().map_or(ENOSYS_NSS, |f| unsafe {
        f(name, he, buf, len, result, h_errnop)
    })
}

/// # Safety
/// libc's contract for `gethostbyname2_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname2_r(
    name: *const c_char,
    af: c_int,
    he: *mut Hostent,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Hostent,
    h_errnop: *mut c_int,
) -> c_int {
    if let Some(r) = answer(name, af, he, buf, len, result, h_errnop) {
        return r;
    }
    next_gethostbyname2_r().map_or(ENOSYS_NSS, |f| unsafe {
        f(name, af, he, buf, len, result, h_errnop)
    })
}

/// The text of an address with its NUL, in a stack buffer.
struct Text {
    b: [u8; 64],
    n: usize,
}

impl core::fmt::Write for Text {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.n + s.len();
        if end >= self.b.len() {
            return Err(core::fmt::Error);
        }
        self.b[self.n..end].copy_from_slice(s.as_bytes());
        self.n = end;
        Ok(())
    }
}

/// # Safety
/// libc's contract for `getaddrinfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const AddrInfo,
    res: *mut *mut AddrInfo,
) -> c_int {
    let Some(real) = next_getaddrinfo() else {
        return -11; // EAI_SYSTEM
    };
    let af = if hints.is_null() {
        AF_UNSPEC
    } else {
        unsafe { (*hints).family }
    };
    if !active() || node.is_null() || !matches!(af, AF_UNSPEC | AF_INET | AF_INET6) {
        return unsafe { real(node, service, hints, res) };
    }
    let Some(ip) = lookup(bytes(node), af, |ip, _| ip) else {
        return unsafe { real(node, service, hints, res) };
    };
    let mut t = Text { b: [0; 64], n: 0 };
    if core::fmt::write(&mut t, format_args!("{ip}")).is_err() {
        return unsafe { real(node, service, hints, res) };
    }
    t.b[t.n] = 0;
    let mut h = if hints.is_null() {
        unsafe { core::mem::zeroed::<AddrInfo>() }
    } else {
        unsafe { *hints }
    };
    h.flags |= AI_NUMERICHOST;
    unsafe { real(t.b.as_ptr() as *const c_char, service, &h, res) }
}
