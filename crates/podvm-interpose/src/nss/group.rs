//! The group database of the image, TODO/interpose.md T-1753: `getgr*`,
//! `getgrouplist` and `initgroups`. [`super`] has the rules.

use super::*;

/// `struct group` of glibc and musl.
#[repr(C)]
pub struct Group {
    name: *mut c_char,
    passwd: *mut c_char,
    gid: c_uint,
    mem: *mut *mut c_char,
}

static GR: Static<(Group, [u8; 4 * LINE])> =
    Static(UnsafeCell::new(unsafe { core::mem::zeroed() }));
static GR_AT: AtomicUsize = AtomicUsize::new(0);

crate::real!(fn next_getgrnam = "getgrnam"(*const c_char) -> *mut Group);
crate::real!(fn next_getgrgid = "getgrgid"(c_uint) -> *mut Group);
crate::real!(fn next_getgrnam_r = "getgrnam_r"(*const c_char, *mut Group, *mut c_char, usize, *mut *mut Group) -> c_int);
crate::real!(fn next_getgrgid_r = "getgrgid_r"(c_uint, *mut Group, *mut c_char, usize, *mut *mut Group) -> c_int);
crate::real!(fn next_getgrent = "getgrent"() -> *mut Group);
crate::real!(fn next_setgrent = "setgrent"() -> ());
crate::real!(fn next_endgrent = "endgrent"() -> ());
crate::real!(fn next_getgrouplist = "getgrouplist"(*const c_char, c_uint, *mut c_uint, *mut c_int) -> c_int);
crate::real!(fn next_initgroups = "initgroups"(*const c_char, c_uint) -> c_int);

/// The `struct group` and buffer of the calls without `_r`.
fn gr_static() -> (*mut Group, *mut u8, usize) {
    let s = GR.0.get();
    unsafe { (&raw mut (*s).0, (&raw mut (*s).1) as *mut u8, 4 * LINE) }
}

/// Fill `gr` from the first line of the image's /etc/group that `key`
/// matches.
fn group(key: Key, gr: *mut Group, buf: *mut u8, len: usize) -> Option<Result<usize, c_int>> {
    scan(b"/etc/group\0", |idx, line| {
        let [name, pass, gid, mem] = fields::<4>(line, b':')?;
        if !key.hits(idx, name, gid) {
            return None;
        }
        let gid = id(gid)?;
        let mut b = Buf { p: buf, len, at: 0 };
        let mut put = || -> Option<Group> {
            let members = mem.split(|&c| c == b',').filter(|m| !m.is_empty());
            let list = b.ptrs(members.clone().count())?;
            for (i, m) in members.enumerate() {
                let s = b.str(m)?;
                unsafe { *list.add(i) = s };
            }
            Some(Group {
                name: b.str(name)?,
                passwd: b.str(pass)?,
                gid,
                mem: list,
            })
        };
        Some(match put() {
            Some(g) => {
                unsafe { gr.write(g) };
                Ok(idx)
            }
            None => Err(ERANGE),
        })
    })
}

/// # Safety
/// libc's contract for `getgrnam`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrnam(name: *const c_char) -> *mut Group {
    if !active() {
        return next_getgrnam().map_or(core::ptr::null_mut(), |f| unsafe { f(name) });
    }
    let (t, b, n) = gr_static();
    found(group(Key::Name(bytes(name)), t, b, n), t)
}

/// # Safety
/// libc's contract for `getgrgid`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrgid(gid: c_uint) -> *mut Group {
    if !active() {
        return next_getgrgid().map_or(core::ptr::null_mut(), |f| unsafe { f(gid) });
    }
    let (t, b, n) = gr_static();
    found(group(Key::Id(gid), t, b, n), t)
}

/// # Safety
/// libc's contract for `getgrnam_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrnam_r(
    name: *const c_char,
    gr: *mut Group,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Group,
) -> c_int {
    if !active() {
        return next_getgrnam_r().map_or(ENOSYS_NSS, |f| unsafe { f(name, gr, buf, len, result) });
    }
    answer_r(
        group(Key::Name(bytes(name)), gr, buf as *mut u8, len),
        gr,
        result,
    )
}

/// # Safety
/// libc's contract for `getgrgid_r`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrgid_r(
    gid: c_uint,
    gr: *mut Group,
    buf: *mut c_char,
    len: usize,
    result: *mut *mut Group,
) -> c_int {
    if !active() {
        return next_getgrgid_r().map_or(ENOSYS_NSS, |f| unsafe { f(gid, gr, buf, len, result) });
    }
    answer_r(group(Key::Id(gid), gr, buf as *mut u8, len), gr, result)
}

/// # Safety
/// libc's contract for `getgrent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrent() -> *mut Group {
    if !active() {
        return next_getgrent().map_or(core::ptr::null_mut(), |f| unsafe { f() });
    }
    let (t, b, n) = gr_static();
    let r = group(Key::From(GR_AT.load(Ordering::Relaxed)), t, b, n);
    if let Some(Ok(idx)) = r {
        GR_AT.store(idx + 1, Ordering::Relaxed);
    }
    found(r, t)
}

/// # Safety
/// libc's contract for `setgrent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgrent() {
    GR_AT.store(0, Ordering::Relaxed);
    if !active()
        && let Some(f) = next_setgrent()
    {
        unsafe { f() }
    }
}

/// # Safety
/// libc's contract for `endgrent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn endgrent() {
    GR_AT.store(0, Ordering::Relaxed);
    if !active()
        && let Some(f) = next_endgrent()
    {
        unsafe { f() }
    }
}

/// The most groups that [`getgrouplist`] collects.
const MAX_GROUPS: usize = 256;

/// The groups of `user` in the image's /etc/group, `first` first.
fn groups_of(user: &[u8], first: c_uint, out: &mut [c_uint; MAX_GROUPS]) -> usize {
    out[0] = first;
    let mut n = 1;
    scan(b"/etc/group\0", |_, line| {
        let [_, _, gid, mem] = fields::<4>(line, b':')?;
        let gid = id(gid)?;
        if n < MAX_GROUPS
            && !out[..n].contains(&gid)
            && mem.split(|&c| c == b',').any(|m| m == user)
        {
            out[n] = gid;
            n += 1;
        }
        None::<()>
    });
    n
}

/// # Safety
/// libc's contract for `getgrouplist`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrouplist(
    user: *const c_char,
    first: c_uint,
    groups: *mut c_uint,
    ngroups: *mut c_int,
) -> c_int {
    if !active() {
        return next_getgrouplist().map_or(-1, |f| unsafe { f(user, first, groups, ngroups) });
    }
    let mut all = [0 as c_uint; MAX_GROUPS];
    let n = groups_of(bytes(user), first, &mut all);
    let room = unsafe { (*ngroups).max(0) as usize };
    let copy = n.min(room);
    unsafe {
        core::ptr::copy_nonoverlapping(all.as_ptr(), groups, copy);
        *ngroups = n as c_int;
    }
    if n > room {
        -1
    } else {
        n as c_int
    }
}

/// # Safety
/// libc's contract for `initgroups`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn initgroups(user: *const c_char, first: c_uint) -> c_int {
    if !active() {
        return next_initgroups().map_or(-1, |f| unsafe { f(user, first) });
    }
    let mut all = [0 as c_uint; MAX_GROUPS];
    let n = groups_of(bytes(user), first, &mut all);
    unsafe { crate::setgroups(n, all.as_ptr()) }
}
