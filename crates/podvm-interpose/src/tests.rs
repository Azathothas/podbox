use super::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

unsafe extern "C" {
    fn dlopen(name: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, sym: *const c_char) -> *mut c_void;
}

const RTLD_NOW: c_int = 2;
const RTLD_DEFAULT: *mut c_void = core::ptr::null_mut();

/// The real four-argument shape. Every payload calls it this way; the
/// dynamic symbol carries no arity, so a wrapper that declares fewer is
/// still entered with all four registers set.
type FchmodatFn = unsafe extern "C" fn(c_int, *const c_char, c_uint, c_int) -> c_int;

fn lookup<F>(handle: *mut c_void, name: &str) -> F {
    let sym = CString::new(name).expect("symbol name");
    let f = unsafe { dlsym(handle, sym.as_ptr()) };
    assert!(!f.is_null(), "dlsym {name}");
    unsafe { std::mem::transmute_copy::<*mut c_void, F>(&f) }
}

/// libc's own `fchmodat`: the oracle the interposed entry point agrees
/// with on every flags value. `libc.so.6` first, musl's name after it,
/// because the test runs on whichever libc the toolchain used.
fn libc_fchmodat() -> FchmodatFn {
    for lib in ["libc.so.6", "libc.musl-x86_64.so.1"] {
        let name = CString::new(lib).expect("library name");
        let h = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
        if !h.is_null() {
            return lookup(h, "fchmodat");
        }
    }
    panic!("no libc to compare against");
}

/// This object's own interposed entry point, looked up the way the
/// loader looks it up: the test binary defines the `#[no_mangle]`
/// symbol itself, so it wins over libc's without any preload.
fn under_test() -> FchmodatFn {
    lookup(RTLD_DEFAULT, "fchmodat")
}

fn call(f: FchmodatFn, path: &CString, flags: c_int) -> (c_int, c_int) {
    clear_errno();
    let rc = unsafe { f(AT_FDCWD, path.as_ptr(), 0o777, flags) };
    (rc, errno())
}

/// T-1311: the interposed `fchmodat` forwards `flags`. A symlink with
/// `AT_SYMLINK_NOFOLLOW` and with 0 answers exactly as libc does. The
/// test process runs without the preload, so the wrapper resolves the
/// real call through `RTLD_NEXT` and the only difference under test is
/// what the wrapper passes on.
#[test]
fn fchmodat_forwards_flags() {
    let dir = std::env::temp_dir().join(format!("podvm-fchmodat-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("f"), b"data").expect("temp file");
    std::os::unix::fs::symlink(dir.join("f"), dir.join("l")).expect("temp link");
    let link = CString::new(dir.join("l").as_os_str().as_bytes()).expect("link path");
    let real = libc_fchmodat();
    let wrapped = under_test();
    for flags in [0, AT_SYMLINK_NOFOLLOW] {
        // Adjacent and in the same order both sides: a mode-set that
        // lands changes what the next call reads.
        let want = call(real, &link, flags);
        let got = call(wrapped, &link, flags);
        assert_eq!(got, want, "flags={flags:#x}");
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// The real `realpath` shape. A NULL second argument asks libc to
/// allocate the answer, which is what libdnf passes for each repodata
/// target. The dynamic symbol carries no arity, so the return type here
/// follows the wrapper rather than libc's own declaration.
type RealpathFn = unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_void;

/// libc's own `realpath`: the oracle. A handle lookup answers the
/// default version, which is what a bare payload binds.
fn libc_realpath() -> RealpathFn {
    for lib in ["libc.so.6", "libc.musl-x86_64.so.1"] {
        let name = CString::new(lib).expect("library name");
        let h = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
        if !h.is_null() {
            return lookup(h, "realpath");
        }
    }
    panic!("no libc to compare against");
}

/// This object's own interposed entry point, looked up the way the
/// loader looks it up: the test binary defines the `#[no_mangle]`
/// symbol itself, so it wins over libc's without any preload.
fn wrapped_realpath() -> RealpathFn {
    lookup(RTLD_DEFAULT, "realpath")
}

/// libc's own `free`, for the answers both sides allocate. Neither side
/// may free with the test binary's own allocator: the bytes come from
/// libc's.
fn libc_free() -> unsafe extern "C" fn(*mut c_void) {
    for lib in ["libc.so.6", "libc.musl-x86_64.so.1"] {
        let name = CString::new(lib).expect("library name");
        let h = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
        if !h.is_null() {
            return lookup(h, "free");
        }
    }
    panic!("no libc to compare against");
}

fn call_realpath(f: RealpathFn, path: &CString) -> (*mut c_void, c_int) {
    clear_errno();
    let r = unsafe { f(path.as_ptr(), core::ptr::null_mut()) };
    (r, errno())
}

fn resolved_bytes(p: *mut c_void) -> Vec<u8> {
    assert!(!p.is_null());
    unsafe { std::ffi::CStr::from_ptr(p as *const c_char) }
        .to_bytes()
        .to_vec()
}

/// T-1309: the versioned resolver falls back to `dlsym` where the
/// named version does not exist. A version no libc defines exercises
/// the fallback on every libc, with no compatibility version needed.
/// Separate resolvers keep each lookup honest: sharing one would let
/// the second call answer from the first call's cache.
#[test]
fn versioned_lookup_falls_back_to_dlsym() {
    let plain = crate::real::Next::new();
    let want = unsafe { plain.get(c"realpath".to_bytes_with_nul()) };
    assert!(!want.is_null(), "dlsym resolves realpath");
    let versioned = crate::real::Next::new();
    let got = unsafe {
        versioned.get_versioned(
            c"realpath".to_bytes_with_nul(),
            c"GLIBC_9.9".to_bytes_with_nul(),
        )
    };
    assert_eq!(got, want, "fallback answers the same definition");
}

/// T-1309: the interposed `realpath` forwards to the default version.
/// The compatibility version answers `EINVAL` where the second argument
/// is NULL, and an unversioned `dlsym` returns it on some payload libcs
/// (measured on rocky 9), so a wrapper that resolves it breaks every
/// caller that passes NULL. The test process runs without the preload, so the wrapper
/// resolves the real call through `RTLD_NEXT` and the only difference
/// under test is which version it resolves.
#[test]
fn realpath_null_resolved_matches_libc() {
    let dir = std::env::temp_dir().join(format!("podvm-realpath-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = CString::new(dir.as_os_str().as_bytes()).expect("dir path");
    let real = libc_realpath();
    let wrapped = wrapped_realpath();
    let free = libc_free();
    // Adjacent and in the same order both sides.
    let (want_ptr, want_errno) = call_realpath(real, &path);
    let (got_ptr, got_errno) = call_realpath(wrapped, &path);
    // The oracle guards the fixture: a temp dir that does not resolve
    // proves nothing, so its failure fails the test rather than passing
    // it vacuously.
    assert!(!want_ptr.is_null(), "oracle resolves the temp dir");
    assert_eq!(got_errno, want_errno, "errno");
    assert!(!got_ptr.is_null(), "wrapped resolves the temp dir");
    assert_eq!(resolved_bytes(got_ptr), resolved_bytes(want_ptr));
    unsafe {
        free(want_ptr);
        free(got_ptr);
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// T-1409: without the CLI-set flag nothing reroutes, even the exe
/// spelling. (With the flag the wrappers serve the guest path first;
/// that half needs a real `/proc` and a guest value, so the lane drive
/// owns it and this pins the default that must not change.)
#[test]
fn exe_override_is_off_without_the_flag() {
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODVM_GUEST_EXE_FORCE") };
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODVM_GUEST_EXE") };
    assert!(!unsafe { crate::procfs::exe_override_hit(c"/proc/self/exe".as_ptr()) });
    let mut out = [0u8; crate::map::OUT];
    assert!(unsafe { stat_exe_path(&mut out) }.is_none());
}

/// T-0413: the standard stream links read back their targets, exactly
/// as the kernel reports them. Anything else is not a stream link.
#[test]
fn stdio_links_read_back_their_fd_targets() {
    assert_eq!(
        proc_stdio_target(b"/dev/stdin\0"),
        Some(&b"/proc/self/fd/0"[..])
    );
    assert_eq!(
        proc_stdio_target(b"/dev/stdout\0"),
        Some(&b"/proc/self/fd/1"[..])
    );
    assert_eq!(
        proc_stdio_target(b"/dev/stderr\0"),
        Some(&b"/proc/self/fd/2"[..])
    );
    assert_eq!(proc_stdio_target(b"/dev/stdin/\0"), None);
    assert_eq!(proc_stdio_target(b"/proc/self/fd/0\0"), None);
}

/// T-0711: the wall's usual errnos read back with their names, and
/// anything else stays a bare number rather than a guess.
#[test]
fn wall_errnos_carry_their_names() {
    assert_eq!(errno_name(1), b" (EPERM)");
    assert_eq!(errno_name(13), b" (EACCES)");
    assert_eq!(errno_name(22), b" (EINVAL)");
    assert_eq!(errno_name(38), b" (ENOSYS)");
    assert_eq!(errno_name(99), b"");
}

unsafe extern "C" {
    fn dlmopen(lmid: core::ffi::c_long, name: *const c_char, flag: c_int) -> *mut c_void;
}

/// T-1645: a name with a `/` is a path the table applies to. A bare
/// soname is a search, and a `$` token is the loader's to expand.
#[test]
fn only_a_slash_name_without_a_token_is_mapped() {
    let cases: &[(&core::ffi::CStr, bool)] = &[
        (c"/usr/lib/x.so", true),
        (c"./x.so", true),
        (c"lib/x.so", true),
        (c"libc.so.6", false),
        (c"$ORIGIN/x.so", false),
        (c"/opt/$LIB/x.so", false),
    ];
    for (name, want) in cases {
        assert_eq!(unsafe { is_mapped_name(name.as_ptr()) }, *want, "{name:?}");
    }
    assert!(!unsafe { is_mapped_name(core::ptr::null()) });
}

/// The directory and the file name of the libc this test process runs
/// on, read from `/proc/self/maps`.
fn loaded_libc() -> (String, String) {
    let maps = std::fs::read_to_string("/proc/self/maps").expect("maps");
    for line in maps.lines() {
        let Some(path) = line.split_whitespace().nth(5) else {
            continue;
        };
        let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
        if file.starts_with("libc.so") || file.starts_with("libc-") {
            return (dir.to_string(), file.to_string());
        }
    }
    panic!("no libc in /proc/self/maps");
}

/// T-1645: `dlopen` and `dlmopen` load a mapped object by its guest
/// path. The guest directory names nothing on the host, so without the
/// table each load fails; with `GUEST:REAL` set, each load answers the
/// handle of the libc already in the process, which the loader finds by
/// device and inode. The exported trampolines are what the calls enter:
/// the test binary defines `dlopen` and `dlmopen` itself.
#[test]
fn dlopen_loads_a_mapped_object_by_its_guest_path() {
    const RTLD_NOW: c_int = 2;
    const LM_ID_BASE: core::ffi::c_long = 0;
    let (dir, file) = loaded_libc();
    let guest = CString::new(format!("/podvm-t1645-guest/{file}")).expect("guest path");
    let soname = CString::new(file.clone()).expect("soname");
    // The control: a bare soname goes through unchanged and loads.
    let want = unsafe { dlopen(soname.as_ptr(), RTLD_NOW) };
    assert!(!want.is_null(), "the bare soname {file} loads");
    // Without the table the guest path names nothing.
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODVM_MAPS") };
    assert!(unsafe { dlopen(guest.as_ptr(), RTLD_NOW) }.is_null());
    // SAFETY: as above.
    unsafe { std::env::set_var("PODVM_MAPS", format!("/podvm-t1645-guest:{dir}")) };
    // A longer name first: the rewritten name lives in one buffer for
    // each thread, and the shorter name after it must not keep its tail.
    let long = CString::new(format!(
        "/podvm-t1645-guest/{file}-a-longer-name-that-is-absent"
    ))
    .expect("long path");
    assert!(unsafe { dlopen(long.as_ptr(), RTLD_NOW) }.is_null());
    let got = unsafe { dlopen(guest.as_ptr(), RTLD_NOW) };
    let got_m = unsafe { dlmopen(LM_ID_BASE, guest.as_ptr(), RTLD_NOW) };
    // SAFETY: as above.
    unsafe { std::env::remove_var("PODVM_MAPS") };
    assert_eq!(got, want, "dlopen of the guest path");
    assert_eq!(
        got_m, want,
        "dlmopen of the guest path in the base namespace"
    );
}

/// T-1699 and T-1701: the `mk*temp` family in a deep rootfs.
mod template;

// TODO/interpose.md T-1704.
mod exe_realpath;

// TODO/cli.md T-1655.
mod readonly;

// TODO/interpose.md T-1710.
mod sock;

// TODO/interpose.md T-1725.
mod links;
