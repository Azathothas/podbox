//! The user, group and host databases of the image, TODO/interpose.md
//! T-1753.
//!
//! ⛔ **libc reads these files past the exported `open`.** glibc's
//! `nss_files` opens `/etc/passwd`, `/etc/group` and `/etc/hosts` with its
//! internal `__open_nocancel`, and musl's `getpw*` with `__fopen_rb_ca`:
//! neither call goes through a symbol this object exports, so the path
//! table never sees the path, and on the userland rung `getpwuid(1000)`
//! answered the host's user. The fix is one level up: this object exports
//! the lookups themselves and reads the image's file through the table.
//!
//! ⭐ **Where `PODVM_MAPS` is unset, each lookup is libc's own.** Where it
//! is set, the image's `/etc/passwd` and `/etc/group` are the database: a
//! name that the image does not have is not found, as in a container, and
//! no other source of `nsswitch.conf` is asked. `/etc/hosts` is asked
//! first, and a name that it does not have goes to libc (DNS);
//! [`hosts`] has the detail.
//!
//! ⚠ No allocation and no lock (`lib.rs`, constraint 2): each file is read
//! with raw system calls into stack buffers, a reentrant call fills the
//! caller's buffer, and a call without `_r` fills static storage, as libc's
//! own does (POSIX: such a call is not thread-safe). A line longer than
//! [`LINE`] bytes is skipped.

use super::*;
use crate::raw_calls::{forward, SYS_CLOSE, SYS_OPENAT};
use core::cell::UnsafeCell;
use core::ffi::c_long;
use core::sync::atomic::{AtomicUsize, Ordering};

pub(crate) mod group;
pub use group::*;
pub(crate) mod hosts;
pub use hosts::*;

#[cfg(target_arch = "x86_64")]
const SYS_READ: c_long = 0;
#[cfg(target_arch = "aarch64")]
const SYS_READ: c_long = 63;
const O_RDONLY_CLOEXEC: c_long = 0o2000000;
const EINTR: c_int = 4;

/// The longest line that a lookup reads.
pub(crate) const LINE: usize = 4096;

/// Whether the path table is set: else each lookup is libc's.
pub(crate) fn active() -> bool {
    unsafe { crate::map::table().n > 0 }
}

/// Call `f` with the index and the bytes of each line of the guest file
/// `path` (NUL-terminated), until `f` answers. `None` where no line
/// answers or the file does not open. `errno` is the caller's after.
pub(crate) fn scan<R>(path: &[u8], mut f: impl FnMut(usize, &[u8]) -> Option<R>) -> Option<R> {
    let saved = errno();
    let r = scan_inner(path, &mut f);
    set_errno(saved);
    r
}

fn scan_inner<R>(path: &[u8], f: &mut impl FnMut(usize, &[u8]) -> Option<R>) -> Option<R> {
    let mut out = [0u8; crate::map::OUT];
    let p = unsafe {
        crate::map::prepare_mode(
            b"open",
            crate::map::AT_FDCWD,
            path.as_ptr() as *const c_char,
            false,
            &mut out,
        )
    };
    if p.is_null() {
        return None;
    }
    let at = crate::map::AT_FDCWD as c_long;
    let fd = unsafe { forward(SYS_OPENAT, [at, p as c_long, O_RDONLY_CLOEXEC, 0, 0, 0]) };
    if fd < 0 {
        return None;
    }
    let mut chunk = [0u8; 4096];
    let mut line = [0u8; LINE];
    let (mut n_line, mut long, mut idx) = (0usize, false, 0usize);
    let mut found = None;
    'read: loop {
        let ptr = chunk.as_mut_ptr() as c_long;
        let n = unsafe { forward(SYS_READ, [fd, ptr, chunk.len() as c_long, 0, 0, 0]) };
        if n < 0 && errno() == EINTR {
            continue;
        }
        if n <= 0 {
            break;
        }
        for &b in &chunk[..n as usize] {
            if b == b'\n' {
                if !long && let Some(r) = f(idx, &line[..n_line]) {
                    found = Some(r);
                    break 'read;
                }
                (idx, n_line, long) = (idx + 1, 0, false);
            } else if n_line < LINE {
                line[n_line] = b;
                n_line += 1;
            } else {
                long = true;
            }
        }
    }
    if found.is_none() && n_line > 0 && !long {
        found = f(idx, &line[..n_line]);
    }
    unsafe { forward(SYS_CLOSE, [fd, 0, 0, 0, 0, 0]) };
    found
}

/// The bytes of a C string, without the NUL.
pub(crate) fn bytes<'a>(s: *const c_char) -> &'a [u8] {
    if s.is_null() {
        return &[];
    }
    unsafe { core::slice::from_raw_parts(s as *const u8, crate::map::strnlen(s, LINE)) }
}

/// A decimal id, or `None`.
pub(crate) fn id(s: &[u8]) -> Option<c_uint> {
    if s.is_empty() || s.len() > 10 {
        return None;
    }
    let mut v: u64 = 0;
    for &b in s {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (b - b'0') as u64;
    }
    c_uint::try_from(v).ok()
}

/// The fields of a line split on `sep`, or `None` for a comment, an empty
/// line or a compat line (`+`, `-`) of NIS.
pub(crate) fn fields<const N: usize>(line: &[u8], sep: u8) -> Option<[&[u8]; N]> {
    if matches!(line.first(), None | Some(b'#' | b'+' | b'-')) {
        return None;
    }
    let mut out: [&[u8]; N] = [&[]; N];
    let mut it = line.splitn(N, |&b| b == sep);
    for slot in out.iter_mut() {
        *slot = it.next()?;
    }
    Some(out)
}

/// A bump allocator over the caller's buffer.
pub(crate) struct Buf {
    pub(crate) p: *mut u8,
    pub(crate) len: usize,
    pub(crate) at: usize,
}

impl Buf {
    /// `n` bytes aligned to `align`, or `None` where the buffer is full.
    pub(crate) fn take(&mut self, n: usize, align: usize) -> Option<*mut u8> {
        let start = (self.p as usize + self.at).next_multiple_of(align) - self.p as usize;
        if start.checked_add(n)? > self.len {
            return None;
        }
        self.at = start + n;
        Some(unsafe { self.p.add(start) })
    }

    /// A copy of `s` with its NUL.
    pub(crate) fn str(&mut self, s: &[u8]) -> Option<*mut c_char> {
        let d = self.take(s.len() + 1, 1)?;
        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), d, s.len());
            *d.add(s.len()) = 0;
        }
        Some(d as *mut c_char)
    }

    /// An array of `n` pointers, the last one null.
    pub(crate) fn ptrs(&mut self, n: usize) -> Option<*mut *mut c_char> {
        let size = core::mem::size_of::<*mut c_char>();
        let d = self.take((n + 1) * size, size)? as *mut *mut c_char;
        unsafe { *d.add(n) = core::ptr::null_mut() };
        Some(d)
    }
}

/// `struct passwd` of glibc and musl, on x86_64 and aarch64.
#[repr(C)]
pub struct Passwd {
    name: *mut c_char,
    passwd: *mut c_char,
    uid: c_uint,
    gid: c_uint,
    gecos: *mut c_char,
    dir: *mut c_char,
    shell: *mut c_char,
}

/// Storage of a call without `_r`.
pub(crate) struct Static<T>(pub(crate) UnsafeCell<T>);
// SAFETY: a call without `_r` is not thread-safe by its contract, as libc's.
unsafe impl<T> Sync for Static<T> {}

static PW: Static<(Passwd, [u8; LINE])> = Static(UnsafeCell::new(unsafe { core::mem::zeroed() }));
static PW_AT: AtomicUsize = AtomicUsize::new(0);

/// What a lookup matches.
#[derive(Clone, Copy)]
pub(crate) enum Key<'a> {
    Name(&'a [u8]),
    Id(c_uint),
    /// The first entry at this line or after it, for `get*ent`.
    From(usize),
}

impl Key<'_> {
    pub(crate) fn hits(&self, idx: usize, name: &[u8], id: &[u8]) -> bool {
        match *self {
            Key::Name(n) => n == name,
            Key::Id(want) => self::id(id) == Some(want),
            Key::From(at) => idx >= at,
        }
    }
}

/// Fill `pw` from the first line of the image's /etc/passwd that `key`
/// matches: `Some(Ok(line))`, `Some(Err(ERANGE))`, or `None`.
fn passwd(key: Key, pw: *mut Passwd, buf: *mut u8, len: usize) -> Option<Result<usize, c_int>> {
    scan(b"/etc/passwd\0", |idx, line| {
        let [name, pass, uid, gid, gecos, dir, shell] = fields::<7>(line, b':')?;
        if !key.hits(idx, name, uid) {
            return None;
        }
        let (uid, gid) = (id(uid)?, id(gid)?);
        let mut b = Buf { p: buf, len, at: 0 };
        let mut put = || -> Option<Passwd> {
            Some(Passwd {
                name: b.str(name)?,
                passwd: b.str(pass)?,
                uid,
                gid,
                gecos: b.str(gecos)?,
                dir: b.str(dir)?,
                shell: b.str(shell)?,
            })
        };
        Some(match put() {
            Some(p) => {
                unsafe { pw.write(p) };
                Ok(idx)
            }
            None => Err(ERANGE),
        })
    })
}

/// The `_r` answer of a lookup: 0 with `*result` set or null, or `ERANGE`.
pub(crate) fn answer_r<T>(
    r: Option<Result<usize, c_int>>,
    out: *mut T,
    result: *mut *mut T,
) -> c_int {
    unsafe { *result = core::ptr::null_mut() };
    match r {
        Some(Ok(_)) => {
            unsafe { *result = out };
            0
        }
        Some(Err(e)) => e,
        None => 0,
    }
}

crate::real!(fn next_getpwnam = "getpwnam"(*const c_char) -> *mut Passwd);
crate::real!(fn next_getpwuid = "getpwuid"(c_uint) -> *mut Passwd);
crate::real!(fn next_getpwnam_r = "getpwnam_r"(*const c_char, *mut Passwd, *mut c_char, usize, *mut *mut Passwd) -> c_int);
crate::real!(fn next_getpwuid_r = "getpwuid_r"(c_uint, *mut Passwd, *mut c_char, usize, *mut *mut Passwd) -> c_int);
crate::real!(fn next_getpwent = "getpwent"() -> *mut Passwd);
crate::real!(fn next_setpwent = "setpwent"() -> ());
crate::real!(fn next_endpwent = "endpwent"() -> ());

/// The `struct passwd` and buffer of the calls without `_r`.
fn pw_static() -> (*mut Passwd, *mut u8, usize) {
    let s = PW.0.get();
    unsafe { (&raw mut (*s).0, (&raw mut (*s).1) as *mut u8, LINE) }
}

/// The static entry where the lookup found one, else null.
pub(crate) fn found<T>(r: Option<Result<usize, c_int>>, t: *mut T) -> *mut T {
    match r {
        Some(Ok(_)) => t,
        Some(Err(e)) => {
            set_errno(e);
            core::ptr::null_mut()
        }
        None => core::ptr::null_mut(),
    }
}

/// # Safety
/// libc's contract for `getpwnam`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwnam(name: *const c_char) -> *mut Passwd {
    if !active() {
        return next_getpwnam().map_or(core::ptr::null_mut(), |f| unsafe { f(name) });
    }
    let (t, b, n) = pw_static();
    found(passwd(Key::Name(bytes(name)), t, b, n), t)
}

/// # Safety
/// libc's contract for `getpwuid`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwuid(uid: c_uint) -> *mut Passwd {
    if !active() {
        return next_getpwuid().map_or(core::ptr::null_mut(), |f| unsafe { f(uid) });
    }
    let (t, b, n) = pw_static();
    found(passwd(Key::Id(uid), t, b, n), t)
}

/// # Safety
/// libc's contract for `getpwnam_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwnam_r(
    name: *const c_char,
    pw: *mut Passwd,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Passwd,
) -> c_int {
    if !active() {
        return next_getpwnam_r().map_or(ENOSYS_NSS, |f| unsafe { f(name, pw, buf, len, result) });
    }
    answer_r(
        passwd(Key::Name(bytes(name)), pw, buf as *mut u8, len),
        pw,
        result,
    )
}

/// # Safety
/// libc's contract for `getpwuid_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwuid_r(
    uid: c_uint,
    pw: *mut Passwd,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Passwd,
) -> c_int {
    if !active() {
        return next_getpwuid_r().map_or(ENOSYS_NSS, |f| unsafe { f(uid, pw, buf, len, result) });
    }
    answer_r(passwd(Key::Id(uid), pw, buf as *mut u8, len), pw, result)
}

/// # Safety
/// libc's contract for `getpwent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwent() -> *mut Passwd {
    if !active() {
        return next_getpwent().map_or(core::ptr::null_mut(), |f| unsafe { f() });
    }
    let (t, b, n) = pw_static();
    let r = passwd(Key::From(PW_AT.load(Ordering::Relaxed)), t, b, n);
    if let Some(Ok(idx)) = r {
        PW_AT.store(idx + 1, Ordering::Relaxed);
    }
    found(r, t)
}

/// # Safety
/// libc's contract for `setpwent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setpwent() {
    PW_AT.store(0, Ordering::Relaxed);
    if !active()
        && let Some(f) = next_setpwent()
    {
        unsafe { f() }
    }
}

/// # Safety
/// libc's contract for `endpwent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn endpwent() {
    PW_AT.store(0, Ordering::Relaxed);
    if !active()
        && let Some(f) = next_endpwent()
    {
        unsafe { f() }
    }
}

/// The answer of a `_r` call whose libc symbol is absent.
pub(crate) const ENOSYS_NSS: c_int = 38;
