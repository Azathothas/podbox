use super::*;

/// # Safety
/// The payload's own contract for `mkdtemp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkdtemp(template: *mut c_char) -> *mut c_char {
    let Some(f) = next_mkdtemp() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let null: *mut c_char = core::ptr::null_mut();
    let r = unsafe { on_template(b"mkdtemp", template, null, |p| !p.is_null(), |t| f(t)) };
    // T-1699: the real call answers the mapped copy, which dies with the
    // stack frame. The caller gets its own template, as libc answers.
    if r.is_null() {
        r
    } else {
        template
    }
}

/// `tmpfile`, which takes no path and forwards unchanged.
///
/// ⛔ `execl`, `execlp`, `execle` and `execlpe` are NOT here, and the absence
/// is a language wall rather than an omission: they are C-variadic, and
/// stable Rust cannot define a C-variadic function (rust-lang/rust#44930).
/// A payload calling one reaches the kernel untranslated, which behaves as
/// without the object where no table is set and misses the rewrite where one
/// is.
///
/// # Safety
/// The payload's own contract for `tmpfile(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tmpfile() -> *mut c_void {
    let Some(f) = next_tmpfile() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    unsafe { f() }
}

/// # Safety
/// The payload's own contract for `tmpfile64(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tmpfile64() -> *mut c_void {
    let Some(f) = next_tmpfile64() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    unsafe { f() }
}

// ----------------------------------------------- T-0711: the identity calls
//
// ⭐ Two behaviours behind one symbol each, chosen by the environment.
// Without `PODVM_IDENTITY` every setter below calls through and names the
// failure: that is the honest default the operator ruled. With it, the call
// tries the real one first and records a refusal as success, which is the
// fakeroot behaviour the flag exists to turn on. The getters answer the
// record under the flag and forward without it.

/// Without the flag: the real call's answer, and a named diagnostic where
/// it fails.
///
/// ⛔ The `write` inside `say::line` can itself move `errno`, so the number
/// is saved across it. Reporting the wrong errno would send the reader to
/// the wrong wall.
unsafe fn honest(what: &[u8], rc: c_int) -> c_int {
    if rc == 0 {
        return 0;
    }
    let e = errno();
    // T-0711: the refusal names the call and the errno, number and --
    // where the number is one of the wall's usual -- name. A payload
    // deciding whether a failed drop is fatal reads `EPERM` faster than
    // `1`, and a bare number where the table has no name stays a number
    // rather than a guess.
    say::line(&[
        what,
        b" failed with errno ",
        say::Num::new(e.unsigned_abs() as u64).as_bytes(),
        errno_name(e),
        b"; podvm runs the payload as uid 0 and does not change it \
          (TODO/interpose.md T-0711)",
    ]);
    set_errno(e);
    rc
}

/// The wall's usual errno names, T-0711. Kernel UAPI numbers, identical
/// on every Linux architecture this object loads on. Anything else
/// answers empty, and the caller prints the number alone.
pub(super) fn errno_name(e: c_int) -> &'static [u8] {
    match e {
        1 => b" (EPERM)",
        2 => b" (ENOENT)",
        13 => b" (EACCES)",
        22 => b" (EINVAL)",
        30 => b" (EROFS)",
        38 => b" (ENOSYS)",
        _ => b"",
    }
}

/// An invalid id stays invalid: it is the caller's mistake, not the wall,
/// so the real call answers it without the record.
fn valid(id: u32) -> bool {
    id != identity::NO_ID
}

/// # Safety
/// The payload's own contract for `setuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setuid(uid: c_uint) -> c_int {
    let Some(f) = next_setuid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setuid", f(uid)) };
    }
    if !valid(uid) {
        return unsafe { f(uid) };
    }
    unsafe { identity::attempt(b"setuid", || f(uid), || identity::set_user(uid, uid, uid)) }
}

/// # Safety
/// The payload's own contract for `setgid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgid(gid: c_uint) -> c_int {
    let Some(f) = next_setgid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setgid", f(gid)) };
    }
    if !valid(gid) {
        return unsafe { f(gid) };
    }
    unsafe { identity::attempt(b"setgid", || f(gid), || identity::set_group(gid, gid, gid)) }
}

/// # Safety
/// The payload's own contract for `seteuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seteuid(euid: c_uint) -> c_int {
    let Some(f) = next_seteuid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"seteuid", f(euid)) };
    }
    if !valid(euid) {
        return unsafe { f(euid) };
    }
    unsafe {
        identity::attempt(
            b"seteuid",
            || f(euid),
            || identity::set_user(identity::NO_ID, euid, identity::NO_ID),
        )
    }
}

/// # Safety
/// The payload's own contract for `setegid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setegid(egid: c_uint) -> c_int {
    let Some(f) = next_setegid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setegid", f(egid)) };
    }
    if !valid(egid) {
        return unsafe { f(egid) };
    }
    unsafe {
        identity::attempt(
            b"setegid",
            || f(egid),
            || identity::set_group(identity::NO_ID, egid, identity::NO_ID),
        )
    }
}

/// # Safety
/// The payload's own contract for `setreuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setreuid(ruid: c_int, euid: c_int) -> c_int {
    let Some(f) = next_setreuid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setreuid", f(ruid, euid)) };
    }
    let (r, e) = (ruid as u32, euid as u32);
    if !valid(r) && !valid(e) {
        return unsafe { f(ruid, euid) };
    }
    // ⭐ Saved follows the (new) effective id, which is the kernel's own
    // rule: a `-1` effective leaves the effective where the record has it,
    // so the saved lands there too rather than where the real id went.
    let s = if valid(e) { e } else { identity::euid() };
    unsafe {
        identity::attempt(
            b"setreuid",
            || f(ruid, euid),
            || identity::set_user(r, e, s),
        )
    }
}

/// # Safety
/// The payload's own contract for `setregid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setregid(rgid: c_int, egid: c_int) -> c_int {
    let Some(f) = next_setregid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setregid", f(rgid, egid)) };
    }
    let (r, e) = (rgid as u32, egid as u32);
    if !valid(r) && !valid(e) {
        return unsafe { f(rgid, egid) };
    }
    // ⭐ As in `setreuid`: saved follows the (new) effective id.
    let s = if valid(e) { e } else { identity::egid() };
    unsafe {
        identity::attempt(
            b"setregid",
            || f(rgid, egid),
            || identity::set_group(r, e, s),
        )
    }
}

/// # Safety
/// The payload's own contract for `setresuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setresuid(ruid: c_int, euid: c_int, suid: c_int) -> c_int {
    let Some(f) = next_setresuid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setresuid", f(ruid, euid, suid)) };
    }
    let (r, e, s) = (ruid as u32, euid as u32, suid as u32);
    if !valid(r) && !valid(e) && !valid(s) {
        return unsafe { f(ruid, euid, suid) };
    }
    unsafe {
        identity::attempt(
            b"setresuid",
            || f(ruid, euid, suid),
            || identity::set_user(r, e, s),
        )
    }
}

/// # Safety
/// The payload's own contract for `setresgid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setresgid(rgid: c_int, egid: c_int, sgid: c_int) -> c_int {
    let Some(f) = next_setresgid() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setresgid", f(rgid, egid, sgid)) };
    }
    let (r, e, s) = (rgid as u32, egid as u32, sgid as u32);
    if !valid(r) && !valid(e) && !valid(s) {
        return unsafe { f(rgid, egid, sgid) };
    }
    unsafe {
        identity::attempt(
            b"setresgid",
            || f(rgid, egid, sgid),
            || identity::set_group(r, e, s),
        )
    }
}

/// # Safety
/// The payload's own contract for `setgroups(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgroups(size: usize, list: *const c_uint) -> c_int {
    let Some(f) = next_setgroups() else {
        set_errno(EINVAL);
        return -1;
    };
    if !identity::enabled() {
        return unsafe { honest(b"setgroups", f(size, list)) };
    }
    // ⛔ Past the kernel's own ceiling the size is invalid input, not a list
    // to record: `NGROUPS_MAX` is 65536 on Linux, and recording a truncated
    // count as the whole truth would corrupt every later `getgroups`.
    if size > 65536 {
        return unsafe { f(size, list) };
    }
    let mut buf = [0u32; 16];
    let n = size.min(buf.len());
    let mut i = 0usize;
    while i < n {
        buf[i] = unsafe { *list.add(i) };
        i += 1;
    }
    let count = size as u32;
    unsafe {
        identity::attempt(
            b"setgroups",
            || f(size, list),
            || {
                identity::set_groups(&buf[..n], count);
            },
        )
    }
}

/// # Safety
/// The payload's own contract for `getuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getuid() -> c_uint {
    if identity::enabled() {
        return identity::ruid();
    }
    match next_getuid() {
        Some(f) => unsafe { f() },
        None => u32::MAX,
    }
}

/// # Safety
/// The payload's own contract for `geteuid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn geteuid() -> c_uint {
    if identity::enabled() {
        return identity::euid();
    }
    match next_geteuid() {
        Some(f) => unsafe { f() },
        None => u32::MAX,
    }
}

/// # Safety
/// The payload's own contract for `getgid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgid() -> c_uint {
    if identity::enabled() {
        return identity::rgid();
    }
    match next_getgid() {
        Some(f) => unsafe { f() },
        None => u32::MAX,
    }
}

/// # Safety
/// The payload's own contract for `getegid(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getegid() -> c_uint {
    if identity::enabled() {
        return identity::egid();
    }
    match next_getegid() {
        Some(f) => unsafe { f() },
        None => u32::MAX,
    }
}

/// # Safety
/// The payload's own contract for `getgroups(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgroups(size: c_int, list: *mut c_uint) -> c_int {
    if !identity::enabled() {
        let Some(f) = next_getgroups() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(size, list) };
    }
    if size < 0 {
        // ⛔ A negative size is invalid input, and the record must not answer
        // it: the real call fails `EINVAL`, and so does this one.
        let Some(f) = next_getgroups() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(size, list) };
    }
    unsafe { identity::groups(list, size) }
}

/// Fill three out-pointers from the record, or fault exactly as the real
/// call faults on a null one.
macro_rules! getres {
    ($name:ident, $real:ident, $r:ident, $e:ident, $s:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            ruid: *mut c_uint,
            euid: *mut c_uint,
            suid: *mut c_uint,
        ) -> c_int {
            if identity::enabled() {
                if ruid.is_null() || euid.is_null() || suid.is_null() {
                    let Some(f) = $real() else {
                        set_errno(EINVAL);
                        return -1;
                    };
                    return unsafe { f(ruid, euid, suid) };
                }
                unsafe {
                    *ruid = identity::$r();
                    *euid = identity::$e();
                    *suid = identity::$s();
                }
                return 0;
            }
            let Some(f) = $real() else {
                set_errno(EINVAL);
                return -1;
            };
            unsafe { f(ruid, euid, suid) }
        }
    };
}

getres!(getresuid, next_getresuid, ruid, euid, saved_uid);
getres!(getresgid, next_getresgid, rgid, egid, saved_gid);
