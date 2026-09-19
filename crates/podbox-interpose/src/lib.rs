//! `TOOL.md` section 6.7: the `LD_PRELOAD` cdylib.
//!
//! ⭐ **Ownership virtualization, which is the half a path interposer does not
//! have.** [`TODO/interpose.md`](../../../TODO/interpose.md) T-0704:
//! `references/VHSgunzo__pathmap/tree/path-mapping.c:1240-1241` routes `chown`
//! through the same path-rewriting macro as every other entry point and passes
//! the ids through untouched, so a payload's `chown 0:42` fails with `EINVAL`
//! exactly as it would with no interposer loaded. That is the wall that stops
//! the most tools, and clearing it is what makes M6 worth doing.
//!
//! ⚠ **Path virtualization is T-0703, and it landed beside the ownership
//! half.** Every path-taking entry point rewrites through `map` before
//! calling through. The two halves stay separate mechanisms: with no table
//! every path call forwards unchanged, and shipping the rewrite is not a
//! claim about reverse mapping (T-0705) or the emulated operations (T-0708).
//!
//! # The four constraints, none of them optional
//!
//! This object runs inside other people's processes, so
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0701 binds every line:
//!
//! 1. ⛔ **A version script exporting only the interposed symbols.**
//!    `interpose.map` is it, and `scripts/build-interpose.sh` passes it. A
//!    default Rust cdylib exports `rust_eh_personality` and friends into every
//!    process it is loaded into;
//! 2. ⛔ **No allocation and no lock on an interposed path.** Every buffer here
//!    is on the stack, the `dlsym` cache is one `AtomicPtr` per entry point, and
//!    the memo needs no lock because `O_APPEND` makes the kernel do the
//!    serialising. ⚠ The constraint is on THIS object's allocation: forwarding
//!    re-enters the payload's allocator and that is fine and unavoidable;
//! 3. ⛔ **No `println!`.** [`say`] writes to fd 2 directly, because stdout
//!    belongs to the payload;
//! 4. ⛔ **It must survive the payload forking**, which every workload here does
//!    constantly. Nothing is registered with `atfork`, nothing is held across a
//!    fork but an `AtomicPtr` of a function address, and the memo is a file
//!    rather than memory precisely so a `fork` and an `execve` both keep it.
//!
//! # What it does to a `chown`
//!
//! ⛔ The real call is TRIED first and only its failure is swallowed, which is
//! fakeroot's shape at
//! `references/salsa-debian__fakeroot/tree/libfakeroot.c:875-907`. ⚠ And
//! fakeroot's errno test is **wrong for this runtime**:
//! `references/salsa-debian__fakeroot/tree/libfakeroot.c:903` is
//! `if(r&&(errno==EPERM)) r=0;` and there are eight such sites. Here the errno
//! is `EINVAL`, so every one of them would hand the error back to the caller.
//! podbox swallows `EPERM` **and** `EINVAL` and records which.

#![forbid(unsafe_op_in_unsafe_fn)]
// ⛔ x86_64 ONLY, and refused by name rather than built wrong. The `stat`
// offsets below are this architecture's, measured under both libcs; on i686 and
// arm they differ, and an object that wrote a uid at the wrong offset would
// corrupt whatever field is there. `scripts/build-interpose.sh` builds the two
// x86_64 targets, so this is the set that exists rather than a limitation
// invented here.
#[cfg(not(target_arch = "x86_64"))]
compile_error!(
    "podbox-interpose is x86_64-only: src/lib.rs carries this architecture's \
     `struct stat` field offsets, measured under glibc and musl, and another \
     architecture needs its own measurement (TODO/interpose.md T-0704)"
);

pub mod identity;
pub mod map;
pub mod memo;
pub mod real;
pub mod say;

use core::ffi::{c_char, c_int, c_uint, c_void};

// ------------------------------------------------------------- `struct stat`
//
// ⭐ MEASURED, not read out of a header. `experiments/105-interpose-ownership.sh`
// compiles one `offsetof` program with the host `cc` and once more through
// `scripts/zig-cc.sh` against musl, and both answer:
//
//     sizeof=144 dev=0 ino=8 mode=24 uid=28 gid=32 rdev=40 size=48
//
// ⚠ They agree on x86_64 and that is a property of this architecture rather
// than a general truth: `TODO/interpose.md` T-0702's whole premise is that
// struct layout is what a preload cannot bridge, and
// `references/pkgforge-dev__cross-libc-dlopen/tree/docs/limits.md:20` records
// `regoff_t` at 4 bytes on glibc and 8 on musl. This object is built once per
// libc anyway, so it never has to bridge them.
const ST_DEV: usize = 0;
const ST_INO: usize = 8;
const ST_UID: usize = 28;
const ST_GID: usize = 32;

// ------------------------------------------------------------ `struct statx`
//
// ⛔ **`statx` IS NOT A DUPLICATE OF `stat`, and leaving it out made the glibc
// arm answer `0:0` while the memo was written.** Measured on 2026-09-09 by
// `experiments/105-interpose-ownership.sh`: coreutils' `stat` on a glibc 2.41
// payload asks `statx(2)` and never reaches `stat`, `stat64` or `__xstat`, so
// every one of those was interposed and none of them was called. busybox's
// `stat` on musl does call `stat`, which is why the musl arm passed and the
// glibc one did not -- a one-libc test would have shipped this.
//
// ⚠ Its offsets are the KERNEL's and identical under both libcs, measured the
// same way: sizeof=256 mask=0 uid=20 gid=24 mode=28 ino=32 devmaj=136
// devmin=140.
const STX_MASK: usize = 0;
const STX_UID: usize = 20;
const STX_GID: usize = 24;
const STX_INO: usize = 32;
const STX_DEV_MAJOR: usize = 136;
const STX_DEV_MINOR: usize = 140;
const STATX_UID: u32 = 0x0000_0008;
const STATX_GID: u32 = 0x0000_0010;

/// The kernel's wide device encoding, which is what `st_dev` carries and what
/// `statx` splits into a major and a minor. ⛔ The same formula
/// `podbox_probe::sys::makedev` uses, because a memo written under one encoding
/// and looked up under another finds nothing and reads as "no memo".
fn makedev(major: u64, minor: u64) -> u64 {
    ((major & 0xfff) << 8)
        | (minor & 0xff)
        | ((major & !0xfffu64) << 32)
        | ((minor & !0xffu64) << 12)
}

/// Read `dev` and `ino` out of a filled `struct stat`.
///
/// # Safety
/// `st` must point at a `struct stat` the real call has just filled.
unsafe fn dev_ino(st: *const c_void) -> (u64, u64) {
    let b = st as *const u8;
    unsafe {
        (
            (b.add(ST_DEV) as *const u64).read_unaligned(),
            (b.add(ST_INO) as *const u64).read_unaligned(),
        )
    }
}

/// Report the memo back, over a filled `struct stat`.
///
/// ⛔ Only the fields the payload actually asked for are overwritten: a `chown`
/// that named a gid and left the uid alone must not make podbox invent a uid.
/// A lookup past the scan ceiling leaves the real `stat` untouched: the memo
/// may hold a newer record for this file past it, so any record from before it
/// could be stale, and the real owner marked degraded is the honest answer.
///
/// # Safety
/// As [`dev_ino`].
unsafe fn report(st: *mut c_void) {
    let (dev, ino) = unsafe { dev_ino(st) };
    match memo::lookup(dev, ino) {
        memo::Lookup::Miss | memo::Lookup::BeyondCeiling => (),
        memo::Lookup::Hit(o) => {
            let b = st as *mut u8;
            unsafe {
                if o.set & memo::SET_UID != 0 {
                    (b.add(ST_UID) as *mut u32).write_unaligned(o.uid);
                }
                if o.set & memo::SET_GID != 0 {
                    (b.add(ST_GID) as *mut u32).write_unaligned(o.gid);
                }
            }
            if say::debug() {
                say::line(&[
                    b"reported the recorded owner ",
                    say::Num::new(o.uid as u64).as_bytes(),
                    b":",
                    say::Num::new(o.gid as u64).as_bytes(),
                    b" for inode ",
                    say::Num::new(ino).as_bytes(),
                ]);
            }
        }
    }
}

/// Report the memo back, over a filled `struct statx`.
///
/// Past the scan ceiling this leaves the real `statx` untouched, for `report`'s
/// reason: any record from before the ceiling could be stale.
///
/// # Safety
/// `st` must point at a `struct statx` the real call has just filled.
unsafe fn report_statx(st: *mut c_void) {
    let b = st as *mut u8;
    let (ino, major, minor) = unsafe {
        (
            (b.add(STX_INO) as *const u64).read_unaligned(),
            (b.add(STX_DEV_MAJOR) as *const u32).read_unaligned() as u64,
            (b.add(STX_DEV_MINOR) as *const u32).read_unaligned() as u64,
        )
    };
    let memo::Lookup::Hit(o) = memo::lookup(makedev(major, minor), ino) else {
        return;
    };
    unsafe {
        let mut mask = (b.add(STX_MASK) as *const u32).read_unaligned();
        if o.set & memo::SET_UID != 0 {
            (b.add(STX_UID) as *mut u32).write_unaligned(o.uid);
            // ⚠ The mask bit too: `statx` says which fields it filled, and a uid
            // podbox wrote without setting the bit is one the caller is entitled
            // to ignore.
            mask |= STATX_UID;
        }
        if o.set & memo::SET_GID != 0 {
            (b.add(STX_GID) as *mut u32).write_unaligned(o.gid);
            mask |= STATX_GID;
        }
        (b.add(STX_MASK) as *mut u32).write_unaligned(mask);
    }
}

// ------------------------------------------------------------------ the errno
//
// ⚠ `__errno_location` on glibc and musl both; it is the one name both use for
// the thread's errno slot, and reading `errno` through it is what any C caller
// does.

extern "C" {
    fn __errno_location() -> *mut c_int;
}

const EPERM: c_int = 1;
const EINVAL: c_int = 22;

fn errno() -> c_int {
    unsafe { *__errno_location() }
}

fn clear_errno() {
    unsafe { *__errno_location() = 0 }
}

pub(crate) fn set_errno(e: c_int) {
    unsafe { *__errno_location() = e }
}

/// Is this the wall podbox exists for?
///
/// ⛔ **Both `EPERM` and `EINVAL`.** fakeroot tests `EPERM` alone at eight sites
/// and every one of them would hand the error back to the caller: `chown` to an id this
/// runtime cannot map answers `EINVAL`, which is
/// [`TODO/extract.md`](../../../TODO/extract.md) T-0302's own finding.
/// ⛔ Anything else is the payload's answer and is handed back unchanged: a
/// `chown` on a read-only filesystem must still fail.
pub(crate) fn is_the_wall(e: c_int) -> bool {
    e == EPERM || e == EINVAL
}

// ------------------------------------------------------------- the real calls

crate::real!(pub fn next_chown = "chown"(*const c_char, c_uint, c_uint) -> c_int);
crate::real!(pub fn next_lchown = "lchown"(*const c_char, c_uint, c_uint) -> c_int);
crate::real!(pub fn next_fchown = "fchown"(c_int, c_uint, c_uint) -> c_int);
crate::real!(pub fn next_fchownat = "fchownat"(c_int, *const c_char, c_uint, c_uint, c_int) -> c_int);
crate::real!(pub fn next_stat = "stat"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_lstat = "lstat"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_fstat = "fstat"(c_int, *mut c_void) -> c_int);
crate::real!(pub fn next_fstatat = "fstatat"(c_int, *const c_char, *mut c_void, c_int) -> c_int);
crate::real!(pub fn next_stat64 = "stat64"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_lstat64 = "lstat64"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_fstat64 = "fstat64"(c_int, *mut c_void) -> c_int);
crate::real!(pub fn next_fstatat64 = "fstatat64"(c_int, *const c_char, *mut c_void, c_int) -> c_int);
crate::real!(pub fn next_xstat = "__xstat"(c_int, *const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_lxstat = "__lxstat"(c_int, *const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_fxstat = "__fxstat"(c_int, c_int, *mut c_void) -> c_int);
crate::real!(pub fn next_fxstatat = "__fxstatat"(c_int, c_int, *const c_char, *mut c_void, c_int) -> c_int);
crate::real!(pub fn next_statx = "statx"(c_int, *const c_char, c_int, c_uint, *mut c_void) -> c_int);

// ------------------------------------------------------- T-0703: the paths
//
// ⭐ One declaration per interposed path-taking symbol, in the same shape as
// above. A name the payload's libc does not define resolves to null, and the
// entry point answers `EINVAL` rather than calling it: that is a payload
// probing for something absent, not a call.
//
// ⚠ `pub`, because the macro-generated `pub` entry points below name them:
// a private alias in a public signature is E0363.

pub type Filter = unsafe extern "C" fn(*const c_void) -> c_int;
pub type Compar = unsafe extern "C" fn(*const *const c_void, *const *const c_void) -> c_int;
pub type ErrFunc = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FtwFunc = unsafe extern "C" fn(*const c_char, *const c_void, c_int) -> c_int;
pub type NftwFunc =
    unsafe extern "C" fn(*const c_char, *const c_void, c_int, *const c_void) -> c_int;

crate::real!(pub fn next_open = "open"(*const c_char, c_int, c_uint) -> c_int);
crate::real!(pub fn next_open64 = "open64"(*const c_char, c_int, c_uint) -> c_int);
crate::real!(pub fn next_openat = "openat"(c_int, *const c_char, c_int, c_uint) -> c_int);
crate::real!(pub fn next_openat64 = "openat64"(c_int, *const c_char, c_int, c_uint) -> c_int);
crate::real!(pub fn next_openat2 = "openat2"(c_int, *const c_char, *const c_void, usize) -> c_int);
crate::real!(pub fn next_creat = "creat"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_creat64 = "creat64"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_execve = "execve"(*const c_char, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execv = "execv"(*const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execvp = "execvp"(*const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execvpe = "execvpe"(*const c_char, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execveat = "execveat"(c_int, *const c_char, *const *const c_char, *const *const c_char, c_int) -> c_int);
crate::real!(pub fn next_posix_spawn = "posix_spawn"(*mut c_int, *const c_char, *const c_void, *const c_void, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_posix_spawnp = "posix_spawnp"(*mut c_int, *const c_char, *const c_void, *const c_void, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_chdir = "chdir"(*const c_char) -> c_int);
crate::real!(pub fn next_opendir = "opendir"(*const c_char) -> *mut c_void);
crate::real!(pub fn next_scandir = "scandir"(*const c_char, *mut *mut c_void, Filter, Compar) -> c_int);
crate::real!(pub fn next_readlink = "readlink"(*const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_readlinkat = "readlinkat"(c_int, *const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_realpath = "realpath"(*const c_char, *mut c_char) -> *mut c_void);
crate::real!(pub fn next_canonicalize = "canonicalize_file_name"(*const c_char) -> *mut c_void);
crate::real!(pub fn next_getxattr = "getxattr"(*const c_char, *const c_char, *mut c_void, usize) -> isize);
crate::real!(pub fn next_lgetxattr = "lgetxattr"(*const c_char, *const c_char, *mut c_void, usize) -> isize);
crate::real!(pub fn next_setxattr = "setxattr"(*const c_char, *const c_char, *const c_void, usize, c_int) -> c_int);
crate::real!(pub fn next_lsetxattr = "lsetxattr"(*const c_char, *const c_char, *const c_void, usize, c_int) -> c_int);
crate::real!(pub fn next_listxattr = "listxattr"(*const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_llistxattr = "llistxattr"(*const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_removexattr = "removexattr"(*const c_char, *const c_char) -> c_int);
crate::real!(pub fn next_lremovexattr = "lremovexattr"(*const c_char, *const c_char) -> c_int);
crate::real!(pub fn next_link = "link"(*const c_char, *const c_char) -> c_int);
crate::real!(pub fn next_linkat = "linkat"(c_int, *const c_char, c_int, *const c_char, c_int) -> c_int);
crate::real!(pub fn next_symlink = "symlink"(*const c_char, *const c_char) -> c_int);
crate::real!(pub fn next_symlinkat = "symlinkat"(*const c_char, c_int, *const c_char) -> c_int);
crate::real!(pub fn next_unlink = "unlink"(*const c_char) -> c_int);
crate::real!(pub fn next_unlinkat = "unlinkat"(c_int, *const c_char, c_int) -> c_int);
crate::real!(pub fn next_rename = "rename"(*const c_char, *const c_char) -> c_int);
crate::real!(pub fn next_renameat = "renameat"(c_int, *const c_char, c_int, *const c_char) -> c_int);
crate::real!(pub fn next_renameat2 = "renameat2"(c_int, *const c_char, c_int, *const c_char, c_uint) -> c_int);
crate::real!(pub fn next_mkdir = "mkdir"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_mkdirat = "mkdirat"(c_int, *const c_char, c_uint) -> c_int);
crate::real!(pub fn next_rmdir = "rmdir"(*const c_char) -> c_int);
crate::real!(pub fn next_mkstemp = "mkstemp"(*mut c_char) -> c_int);
crate::real!(pub fn next_mkostemp = "mkostemp"(*mut c_char, c_int) -> c_int);
crate::real!(pub fn next_mkdtemp = "mkdtemp"(*mut c_char) -> *mut c_char);
crate::real!(pub fn next_access = "access"(*const c_char, c_int) -> c_int);
crate::real!(pub fn next_eaccess = "eaccess"(*const c_char, c_int) -> c_int);
crate::real!(pub fn next_euidaccess = "euidaccess"(*const c_char, c_int) -> c_int);
crate::real!(pub fn next_faccessat = "faccessat"(c_int, *const c_char, c_int, c_int) -> c_int);
crate::real!(pub fn next_chmod = "chmod"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_fchmodat = "fchmodat"(c_int, *const c_char, c_uint) -> c_int);
crate::real!(pub fn next_truncate = "truncate"(*const c_char, i64) -> c_int);
crate::real!(pub fn next_utime = "utime"(*const c_char, *const c_void) -> c_int);
crate::real!(pub fn next_utimes = "utimes"(*const c_char, *const c_void) -> c_int);
crate::real!(pub fn next_utimensat = "utimensat"(c_int, *const c_char, *const c_void, c_int) -> c_int);
crate::real!(pub fn next_statfs = "statfs"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_statvfs = "statvfs"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_remove = "remove"(*const c_char) -> c_int);
crate::real!(pub fn next_fopen = "fopen"(*const c_char, *const c_char) -> *mut c_void);
crate::real!(pub fn next_fopen64 = "fopen64"(*const c_char, *const c_char) -> *mut c_void);
crate::real!(pub fn next_tmpfile = "tmpfile"() -> *mut c_void);
crate::real!(pub fn next_tmpfile64 = "tmpfile64"() -> *mut c_void);
crate::real!(pub fn next_freopen = "freopen"(*const c_char, *const c_char, *mut c_void) -> *mut c_void);
crate::real!(pub fn next_freopen64 = "freopen64"(*const c_char, *const c_char, *mut c_void) -> *mut c_void);
crate::real!(pub fn next_glob = "glob"(*const c_char, c_int, ErrFunc, *mut c_void) -> c_int);
crate::real!(pub fn next_glob64 = "glob64"(*const c_char, c_int, ErrFunc, *mut c_void) -> c_int);
crate::real!(pub fn next_ftw = "ftw"(*const c_char, FtwFunc, c_int) -> c_int);
crate::real!(pub fn next_ftw64 = "ftw64"(*const c_char, FtwFunc, c_int) -> c_int);
crate::real!(pub fn next_nftw = "nftw"(*const c_char, NftwFunc, c_int, c_int) -> c_int);
crate::real!(pub fn next_nftw64 = "nftw64"(*const c_char, NftwFunc, c_int, c_int) -> c_int);
crate::real!(pub fn next_xstat64 = "__xstat64"(c_int, *const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_lxstat64 = "__lxstat64"(c_int, *const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_fxstatat64 = "__fxstatat64"(c_int, c_int, *const c_char, *mut c_void, c_int) -> c_int);

// -------------------------------------------------- T-0711: the identity
//
// One declaration per interposed identity call. Getters return `c_uint`;
// setters take it, except the `*re*` shapes, whose `-1` means "leave" and
// is therefore `c_int`.
crate::real!(pub fn next_setuid = "setuid"(c_uint) -> c_int);
crate::real!(pub fn next_setgid = "setgid"(c_uint) -> c_int);
crate::real!(pub fn next_seteuid = "seteuid"(c_uint) -> c_int);
crate::real!(pub fn next_setegid = "setegid"(c_uint) -> c_int);
crate::real!(pub fn next_setreuid = "setreuid"(c_int, c_int) -> c_int);
crate::real!(pub fn next_setregid = "setregid"(c_int, c_int) -> c_int);
crate::real!(pub fn next_setresuid = "setresuid"(c_int, c_int, c_int) -> c_int);
crate::real!(pub fn next_setresgid = "setresgid"(c_int, c_int, c_int) -> c_int);
crate::real!(pub fn next_setgroups = "setgroups"(usize, *const c_uint) -> c_int);
crate::real!(pub fn next_getuid = "getuid"() -> c_uint);
crate::real!(pub fn next_geteuid = "geteuid"() -> c_uint);
crate::real!(pub fn next_getgid = "getgid"() -> c_uint);
crate::real!(pub fn next_getegid = "getegid"() -> c_uint);
crate::real!(pub fn next_getgroups = "getgroups"(c_int, *mut c_uint) -> c_int);
crate::real!(pub fn next_getresuid = "getresuid"(*mut c_uint, *mut c_uint, *mut c_uint) -> c_int);
crate::real!(pub fn next_getresgid = "getresgid"(*mut c_uint, *mut c_uint, *mut c_uint) -> c_int);

/// `AT_SYMLINK_NOFOLLOW`, for the `lchown` half of `fchownat`.
const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
const AT_FDCWD: c_int = -100;

/// The shared body of every `chown`-family entry point.
///
/// ⛔ **Try the real call FIRST.** A runtime where the `chown` would have
/// succeeded must not get a memo instead of the real thing: the memo changes no
/// kernel permission check, and preferring it would make podbox weaker than the
/// bare chroot on a machine that can do the real work.
///
/// ⚠ `uid` and `gid` of `-1` mean "leave it", which is `chown(2)`'s own
/// convention, so `set` records what the payload actually asked for.
unsafe fn owned(
    real: Option<i32>,
    stat_for_ids: Option<(u64, u64)>,
    uid: c_uint,
    gid: c_uint,
    what: &[u8],
) -> c_int {
    let Some(rc) = real else {
        // ⚠ The payload's libc does not define this entry point at all, which
        // this object cannot happen upon: it was resolved through
        // `dlsym(RTLD_NEXT)` and a null there means the symbol is gone.
        say::line(&[what, b": podbox could not resolve the real call"]);
        unsafe { *__errno_location() = EINVAL };
        return -1;
    };
    if rc == 0 {
        return 0;
    }
    let e = errno();
    if !is_the_wall(e) {
        return rc;
    }
    let Some((dev, ino)) = stat_for_ids else {
        // ⛔ podbox could not learn WHICH file, so it has nothing to record and
        // hands the payload the real failure. A success reported with no memo
        // behind it is the lie this whole object exists to avoid.
        say::line(&[
            what,
            b": the call was refused and podbox could not stat the target, so it \
              has nothing to record and the error is the payload's",
        ]);
        unsafe { *__errno_location() = e };
        return -1;
    };
    let mut set = 0u32;
    if uid != c_uint::MAX {
        set |= memo::SET_UID;
    }
    if gid != c_uint::MAX {
        set |= memo::SET_GID;
    }
    let o = memo::Owner { uid, gid, set };
    if !memo::record(dev, ino, o) {
        say::line(&[
            what,
            b": podbox could not write the ownership memo, so the failure is the \
              payload's rather than being hidden",
        ]);
        unsafe { *__errno_location() = e };
        return -1;
    }
    if say::debug() {
        say::line(&[
            what,
            b": the runtime refused it with errno ",
            say::Num::new(e as u64).as_bytes(),
            b"; podbox recorded the intended owner and reported success \
              (TODO/interpose.md T-0704)",
        ]);
    }
    clear_errno();
    0
}

/// `dev` and `ino` for a path, through the REAL `stat`, so this does not recurse
/// into podbox's own interposition.
unsafe fn ids_of_path(path: *const c_char, follow: bool) -> Option<(u64, u64)> {
    let mut st = [0u8; 256];
    let f = if follow { next_stat() } else { next_lstat() };
    let rc = match f {
        Some(f) => unsafe { f(path, st.as_mut_ptr() as *mut c_void) },
        None => {
            // ⚠ glibc before 2.33 exports `stat` only as `__xstat`, so the
            // fallback is not a nicety: on such a payload `dlsym("stat")`
            // answers null. `1` is `_STAT_VER_LINUX` on x86_64.
            let g = if follow { next_xstat() } else { next_lxstat() }?;
            unsafe { g(1, path, st.as_mut_ptr() as *mut c_void) }
        }
    };
    if rc != 0 {
        return None;
    }
    Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
}

unsafe fn ids_of_fd(fd: c_int) -> Option<(u64, u64)> {
    let mut st = [0u8; 256];
    let rc = match next_fstat() {
        Some(f) => unsafe { f(fd, st.as_mut_ptr() as *mut c_void) },
        None => {
            let g = next_fxstat()?;
            unsafe { g(1, fd, st.as_mut_ptr() as *mut c_void) }
        }
    };
    if rc != 0 {
        return None;
    }
    Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
}

// ------------------------------------------------------- the exported symbols
//
// ⛔ Every one of these is in `interpose.map` and nothing else is. A symbol
// exported by accident is a symbol some other library in the payload's process
// resolves to podbox.

/// # Safety
/// The payload's own contract for `chown(2)`.
#[no_mangle]
pub unsafe extern "C" fn chown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"chown", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let ids = unsafe { ids_of_path(p, true) };
    let rc = next_chown().map(|f| unsafe { f(p, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"chown") }
}

/// # Safety
/// The payload's own contract for `lchown(2)`.
#[no_mangle]
pub unsafe extern "C" fn lchown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"lchown", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let ids = unsafe { ids_of_path(p, false) };
    let rc = next_lchown().map(|f| unsafe { f(p, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"lchown") }
}

/// # Safety
/// The payload's own contract for `fchown(2)`.
#[no_mangle]
pub unsafe extern "C" fn fchown(fd: c_int, uid: c_uint, gid: c_uint) -> c_int {
    let ids = unsafe { ids_of_fd(fd) };
    let rc = next_fchown().map(|f| unsafe { f(fd, uid, gid) });
    unsafe { owned(rc, ids, uid, gid, b"fchown") }
}

/// # Safety
/// The payload's own contract for `fchownat(2)`.
///
/// ⚠ Hand-written rather than macro-generated, which is the one shape
/// `references/VHSgunzo__pathmap/tree/path-mapping.c:1242-1256` also writes out:
/// the flags decide whether the target is the link or what it points at, and a
/// macro over the family cannot express that.
#[no_mangle]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    uid: c_uint,
    gid: c_uint,
    flags: c_int,
) -> c_int {
    // ⭐ The path goes through the table first, so a mapped `fchownat` memos
    // the file the kernel actually changed. Where the rewrite moved it, the
    // call goes with an absolute path; where it did not, the original
    // descriptor and path go through untouched.
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"fchownat", dirfd, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let (rfd, rpath) = if p == path {
        (dirfd, path)
    } else {
        (AT_FDCWD, p)
    };
    let ids = if rfd == AT_FDCWD || unsafe { *rpath } == b'/' as c_char {
        unsafe { ids_of_path(rpath, flags & AT_SYMLINK_NOFOLLOW == 0) }
    } else {
        // A relative path against a directory descriptor, rewritten above
        // where the table matched it. The real `fstatat` answers what is
        // left without podbox resolving anything further.
        let mut st = [0u8; 256];
        let rc = match next_fstatat() {
            Some(f) => unsafe { f(rfd, rpath, st.as_mut_ptr() as *mut c_void, flags) },
            None => match next_fxstatat() {
                Some(g) => unsafe { g(1, rfd, rpath, st.as_mut_ptr() as *mut c_void, flags) },
                None => -1,
            },
        };
        if rc == 0 {
            Some(unsafe { dev_ino(st.as_ptr() as *const c_void) })
        } else {
            None
        }
    };
    let rc = next_fchownat().map(|f| unsafe { f(rfd, rpath, uid, gid, flags) });
    unsafe { owned(rc, ids, uid, gid, b"fchownat") }
}

/// Every `stat`-family entry point: call the real one, then report the memo.
///
/// ⛔ A macro, so the sixteen shapes cannot drift apart. T-0703's Decision names
/// the same reason for the path families.
macro_rules! stat_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            let rc = unsafe { f($($arg),*) };
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

/// The path-taking `stat` shapes: rewrite first, then the real call, then
/// the memo. The memo is keyed by inode, so it answers for the rewritten
/// file, which is the file the kernel just stated.
macro_rules! stat_path_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident, $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            let $path = p;
            let rc = unsafe { f($($arg),*) };
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

/// The `*at` `stat` shapes: resolve the descriptor pair first. Where the
/// rewrite moved the path the call goes with an absolute one; where it did
/// not, the original descriptor and path go through untouched.
macro_rules! stat_at_entry {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* $(,)? ), $st:ident, $dirfd:ident, $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else {
                unsafe { *__errno_location() = EINVAL };
                return -1;
            };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return -1;
            }
            let ($dirfd, $path) = if p == $path {
                ($dirfd, $path)
            } else {
                (crate::map::AT_FDCWD, p)
            };
            let rc = unsafe { f($($arg),*) };
            if rc == 0 {
                unsafe { report($st) };
            }
            rc
        }
    };
}

stat_path_entry!(stat, next_stat, (path: *const c_char, st: *mut c_void), st, path);
stat_path_entry!(lstat, next_lstat, (path: *const c_char, st: *mut c_void), st, path);
stat_entry!(fstat, next_fstat, (fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    fstatat,
    next_fstatat,
    (dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path
);
// ⭐ THE `64` NAMES ARE NOT DUPLICATES, and T-0703 measured why: all four
// `libpython3.*` and `libglib-2.0.so.0` import only `stat64`, `lstat64` and
// `fstatat64`, while `libarchive` and `libdbus-1` import the plain names. Both
// sets are live in one process.
stat_path_entry!(stat64, next_stat64, (path: *const c_char, st: *mut c_void), st, path);
stat_path_entry!(lstat64, next_lstat64, (path: *const c_char, st: *mut c_void), st, path);
stat_entry!(fstat64, next_fstat64, (fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    fstatat64,
    next_fstatat64,
    (dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path
);
// ⭐ AND THE `__xstat` SHAPE, which is glibc's pre-2.33 one and which glibc 2.39
// still exports at `GLIBC_2.2.5`. The PAYLOAD's libc decides which shape its
// libraries call, not the host this object was built on.
stat_path_entry!(
    __xstat,
    next_xstat,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path
);
stat_path_entry!(
    __lxstat,
    next_lxstat,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path
);
// ⭐ AND THEIR `64` SHAPE, which the completeness command counts as reached.
// Same contract as above: define it, forward it, let the loader decide.
stat_path_entry!(
    __xstat64,
    next_xstat64,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path
);
stat_path_entry!(
    __lxstat64,
    next_lxstat64,
    (ver: c_int, path: *const c_char, st: *mut c_void),
    st,
    path
);
stat_entry!(__fxstat, next_fxstat, (ver: c_int, fd: c_int, st: *mut c_void), st);
stat_at_entry!(
    __fxstatat,
    next_fxstatat,
    (ver: c_int, dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path
);
stat_at_entry!(
    __fxstatat64,
    next_fxstatat64,
    (ver: c_int, dirfd: c_int, path: *const c_char, st: *mut c_void, flags: c_int),
    st,
    dirfd,
    path
);

/// `statx(2)`, and it is the entry point a modern `stat(1)` actually calls.
///
/// # Safety
/// The payload's own contract for `statx(2)`.
#[no_mangle]
pub unsafe extern "C" fn statx(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mask: c_uint,
    st: *mut c_void,
) -> c_int {
    let Some(f) = next_statx() else {
        unsafe { *__errno_location() = EINVAL };
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"statx", dirfd, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let (dirfd, path) = if p == path {
        (dirfd, path)
    } else {
        (crate::map::AT_FDCWD, p)
    };
    // ⛔ The caller's mask is widened to include the identity fields. A caller
    // that did not ask for `STATX_UID` gets it anyway, which `statx(2)` permits
    // -- the kernel may return more than was asked for -- and without it podbox
    // would have no uid to overwrite and no `stx_ino` to key the memo on.
    let rc = unsafe {
        f(
            dirfd,
            path,
            flags,
            mask | STATX_UID | STATX_GID | 0x0000_0100,
            st,
        )
    };
    if rc == 0 {
        unsafe { report_statx(st) };
    }
    rc
}

// ------------------------------------------- T-0703: the path entry points
//
// ⭐ One macro per return shape, so the macro-covered entry points cannot
// drift apart, which is `references/VHSgunzo__pathmap/tree/path-mapping.c:250-262`'s
// own decision. Every one funnels its path through [`map::prepare`] and calls
// the real function with what comes back. `$path` is the argument the rewrite
// applies to.
//
// ⚠ Functions taking only descriptors stay out: `fstat`, `fchdir`,
// `fexecve`, `fchmod` and their kin carry nothing to rewrite, which is
// `experiments/100-interpose-symbols.sh`'s own rule for the same absence.
//
// ⚠ What these macros deliberately do NOT do: save and restore `errno` around
// the real call. Linux leaves `errno` alone on success, and every internal
// step that can fail returns before the real call runs, so there is nothing
// to restore on the success path and the failure path carries the real
// call's own number.

macro_rules! path_int {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_nonneg {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident, $ret:ty) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
            let fail: $ret = -1 as $ret;
            let Some(f) = $real() else { set_errno(EINVAL); return fail; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return fail;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_ptr {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> *mut c_void {
            let Some(f) = $real() else { set_errno(EINVAL); return core::ptr::null_mut(); };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return core::ptr::null_mut();
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_at_int {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $dirfd:ident, $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return -1;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_at_nonneg {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $dirfd:ident, $path:ident, $ret:ty) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
            let fail: $ret = -1 as $ret;
            let Some(f) = $real() else { set_errno(EINVAL); return fail; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return fail;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

path_int!(chdir, next_chdir, (path: *const c_char), path);
path_int!(unlink, next_unlink, (path: *const c_char), path);
path_int!(rmdir, next_rmdir, (path: *const c_char), path);
path_int!(remove, next_remove, (path: *const c_char), path);
path_int!(mkdir, next_mkdir, (path: *const c_char, mode: c_uint), path);
path_int!(chmod, next_chmod, (path: *const c_char, mode: c_uint), path);
path_int!(truncate, next_truncate, (path: *const c_char, len: i64), path);
path_int!(utime, next_utime, (path: *const c_char, times: *const c_void), path);
path_int!(utimes, next_utimes, (path: *const c_char, times: *const c_void), path);
path_int!(access, next_access, (path: *const c_char, mode: c_int), path);
path_int!(eaccess, next_eaccess, (path: *const c_char, mode: c_int), path);
path_int!(euidaccess, next_euidaccess, (path: *const c_char, mode: c_int), path);
path_int!(statfs, next_statfs, (path: *const c_char, buf: *mut c_void), path);
path_int!(statvfs, next_statvfs, (path: *const c_char, buf: *mut c_void), path);
path_int!(setxattr, next_setxattr, (path: *const c_char, name: *const c_char, value: *const c_void, size: usize, flags: c_int), path);
path_int!(lsetxattr, next_lsetxattr, (path: *const c_char, name: *const c_char, value: *const c_void, size: usize, flags: c_int), path);
path_int!(removexattr, next_removexattr, (path: *const c_char, name: *const c_char), path);
path_int!(lremovexattr, next_lremovexattr, (path: *const c_char, name: *const c_char), path);
path_int!(execve, next_execve, (path: *const c_char, argv: *const *const c_char, envp: *const *const c_char), path);
path_int!(execv, next_execv, (path: *const c_char, argv: *const *const c_char), path);
path_int!(execvp, next_execvp, (path: *const c_char, argv: *const *const c_char), path);
path_int!(execvpe, next_execvpe, (path: *const c_char, argv: *const *const c_char, envp: *const *const c_char), path);
path_at_int!(execveat, next_execveat, (dirfd: c_int, path: *const c_char, argv: *const *const c_char, envp: *const *const c_char, flags: c_int), dirfd, path);
path_int!(posix_spawn, next_posix_spawn, (pid: *mut c_int, path: *const c_char, fa: *const c_void, attr: *const c_void, argv: *const *const c_char, envp: *const *const c_char), path);
path_int!(posix_spawnp, next_posix_spawnp, (pid: *mut c_int, path: *const c_char, fa: *const c_void, attr: *const c_void, argv: *const *const c_char, envp: *const *const c_char), path);
path_int!(glob, next_glob, (pattern: *const c_char, flags: c_int, errfunc: ErrFunc, pglob: *mut c_void), pattern);
path_int!(glob64, next_glob64, (pattern: *const c_char, flags: c_int, errfunc: ErrFunc, pglob: *mut c_void), pattern);
path_int!(ftw, next_ftw, (path: *const c_char, func: FtwFunc, nfds: c_int), path);
path_int!(ftw64, next_ftw64, (path: *const c_char, func: FtwFunc, nfds: c_int), path);
path_int!(nftw, next_nftw, (path: *const c_char, func: NftwFunc, nfds: c_int, flags: c_int), path);
path_int!(nftw64, next_nftw64, (path: *const c_char, func: NftwFunc, nfds: c_int, flags: c_int), path);

path_nonneg!(creat, next_creat, (path: *const c_char, mode: c_uint), path, c_int);
path_nonneg!(creat64, next_creat64, (path: *const c_char, mode: c_uint), path, c_int);
path_nonneg!(scandir, next_scandir, (dir: *const c_char, list: *mut *mut c_void, filter: Filter, compar: Compar), dir, c_int);
path_nonneg!(readlink, next_readlink, (path: *const c_char, buf: *mut c_char, n: usize), path, isize);
path_nonneg!(getxattr, next_getxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(lgetxattr, next_lgetxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(listxattr, next_listxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);
path_nonneg!(llistxattr, next_llistxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);

path_ptr!(opendir, next_opendir, (path: *const c_char), path);
path_ptr!(fopen, next_fopen, (path: *const c_char, mode: *const c_char), path);
path_ptr!(fopen64, next_fopen64, (path: *const c_char, mode: *const c_char), path);
path_ptr!(freopen, next_freopen, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path);
path_ptr!(freopen64, next_freopen64, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path);
path_ptr!(realpath, next_realpath, (path: *const c_char, resolved: *mut c_char), path);
path_ptr!(canonicalize_file_name, next_canonicalize, (path: *const c_char), path);

path_at_int!(mkdirat, next_mkdirat, (dirfd: c_int, path: *const c_char, mode: c_uint), dirfd, path);
path_at_int!(unlinkat, next_unlinkat, (dirfd: c_int, path: *const c_char, flags: c_int), dirfd, path);
path_at_int!(fchmodat, next_fchmodat, (dirfd: c_int, path: *const c_char, mode: c_uint), dirfd, path);
path_at_int!(faccessat, next_faccessat, (dirfd: c_int, path: *const c_char, mode: c_int, flags: c_int), dirfd, path);
path_at_int!(utimensat, next_utimensat, (dirfd: c_int, path: *const c_char, times: *const c_void, flags: c_int), dirfd, path);
path_at_int!(openat2, next_openat2, (dirfd: c_int, path: *const c_char, how: *const c_void, size: usize), dirfd, path);
path_at_nonneg!(readlinkat, next_readlinkat, (dirfd: c_int, path: *const c_char, buf: *mut c_char, n: usize), dirfd, path, isize);

// ------------------------------------------------- shapes no macro covers

const ERANGE: c_int = 34;
const ENAMETOOLONG: c_int = 36;

/// `open` with a fixed third argument rather than `...`.
///
/// ⭐ On x86_64 SysV the first six integer arguments travel in registers, so a
/// two-argument caller leaves whatever in the third register and a
/// three-argument one puts the mode there. Reading it only under
/// `O_CREAT|O_TMPFILE` is exact for both shapes, and stable Rust cannot
/// define C-variadic functions at all (rust-lang/rust#44930), so this is the
/// shape rather than a workaround.
macro_rules! open_fixed {
    ($name:ident, $real:ident, $path:ident, $flags:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($path: *const c_char, $flags: c_int, mode: c_uint) -> c_int {
            let Some(f) = $real() else {
                set_errno(EINVAL);
                return -1;
            };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            unsafe { f(p, $flags, mode) }
        }
    };
    ($name:ident, $real:ident, $dirfd:ident, $path:ident, $flags:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            $dirfd: c_int,
            $path: *const c_char,
            $flags: c_int,
            mode: c_uint,
        ) -> c_int {
            let Some(f) = $real() else {
                set_errno(EINVAL);
                return -1;
            };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return -1;
            }
            unsafe { f($dirfd, p, $flags, mode) }
        }
    };
}

open_fixed!(open, next_open, path, flags);
open_fixed!(open64, next_open64, path, flags);
open_fixed!(openat, next_openat, dirfd, path, flags);
open_fixed!(openat64, next_openat64, dirfd, path, flags);

/// Two paths in one call: both resolve, and either failing fails the call.
/// `linkat`, `renameat` and `renameat2` share the shape; only the names and
/// the trailing arguments differ.
macro_rules! path_dual {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $dirfd_a:ident, $path_a:ident, $dirfd_b:ident, $path_b:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut bufa = [0u8; crate::map::OUT];
            let mut bufb = [0u8; crate::map::OUT];
            let pa = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd_a, $path_a, &mut bufa)
            };
            let pb = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd_b, $path_b, &mut bufb)
            };
            if pa.is_null() || pb.is_null() {
                return -1;
            }
            let $path_a = pa;
            let $path_b = pb;
            unsafe { f($($arg),*) }
        }
    };
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path_a:ident, $path_b:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut bufa = [0u8; crate::map::OUT];
            let mut bufb = [0u8; crate::map::OUT];
            let pa = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path_a,
                    &mut bufa,
                )
            };
            let pb = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path_b,
                    &mut bufb,
                )
            };
            if pa.is_null() || pb.is_null() {
                return -1;
            }
            let $path_a = pa;
            let $path_b = pb;
            unsafe { f($($arg),*) }
        }
    };
}

path_dual!(link, next_link, (old: *const c_char, new: *const c_char), old, new);
path_dual!(rename, next_rename, (old: *const c_char, new: *const c_char), old, new);
path_dual!(linkat, next_linkat, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char, flags: c_int), olddirfd, old, newdirfd, new);
path_dual!(renameat, next_renameat, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char), olddirfd, old, newdirfd, new);
path_dual!(renameat2, next_renameat2, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char, flags: c_uint), olddirfd, old, newdirfd, new);

/// `symlink` maps the link path fully and the target literally.
///
/// ⭐ The target is link CONTENT, resolved when the link is used and relative
/// to the link's own directory, so absolutizing it now would answer about the
/// wrong directory. An absolute target matches the table as written; a
/// relative one matches only where it literally starts with a prefix. That is
/// `references/VHSgunzo__pathmap/tree/path-mapping.c:1019-1035`'s shape.
///
/// # Safety
/// The payload's own contract for `symlink(2)`.
#[no_mangle]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    let Some(f) = next_symlink() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut linkb = [0u8; crate::map::OUT];
    let mut targb = [0u8; crate::map::OUT];
    let lp = unsafe { crate::map::prepare(b"symlink", crate::map::AT_FDCWD, linkpath, &mut linkb) };
    let tp = unsafe { crate::map::prepare_literal(b"symlink", target, &mut targb) };
    if lp.is_null() || tp.is_null() {
        return -1;
    }
    unsafe { f(tp, lp) }
}

/// # Safety
/// The payload's own contract for `symlinkat(2)`.
#[no_mangle]
pub unsafe extern "C" fn symlinkat(
    target: *const c_char,
    newdirfd: c_int,
    linkpath: *const c_char,
) -> c_int {
    let Some(f) = next_symlinkat() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut linkb = [0u8; crate::map::OUT];
    let mut targb = [0u8; crate::map::OUT];
    let lp = unsafe { crate::map::prepare(b"symlinkat", newdirfd, linkpath, &mut linkb) };
    let tp = unsafe { crate::map::prepare_literal(b"symlinkat", target, &mut targb) };
    if lp.is_null() || tp.is_null() {
        return -1;
    }
    unsafe { f(tp, newdirfd, lp) }
}

/// A `mk*` template is rewritten in place, because the caller reads the
/// chosen name back out of the same buffer.
///
/// ⛔ The mapped name lands only where it fits in the bytes the template
/// already occupies. Writing past them would overflow a buffer whose size
/// the caller never states, so a longer mapping is `ERANGE` rather than a
/// truncation. That is T-0705's decision, applied early.
unsafe fn place_template(what: &[u8], template: *mut c_char, rewritten: *const c_char) -> bool {
    let mut n = 0usize;
    while n < crate::map::OUT {
        if unsafe { *(rewritten as *const u8).add(n) } == 0 {
            break;
        }
        n += 1;
    }
    let mut cap = 0usize;
    while cap < crate::map::OUT {
        if unsafe { *(template as *const u8).add(cap) } == 0 {
            break;
        }
        cap += 1;
    }
    if n > cap {
        crate::say::line(&[what, b": the mapped name does not fit the template"]);
        set_errno(ERANGE);
        return false;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(rewritten as *const u8, template as *mut u8, n + 1);
    }
    true
}

/// Rewrite `template` through the table where it matches, leaving it alone
/// where it does not. Returns the pointer to call with, or null on failure.
unsafe fn map_template(
    what: &[u8],
    template: *mut c_char,
    out: &mut [u8; crate::map::OUT],
) -> *mut c_char {
    if template.is_null() {
        return template;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return template;
    }
    let n = crate::map::strnlen(template, crate::map::OUT);
    if n >= crate::map::OUT {
        return template;
    }
    // Templates are relative to the working directory by construction.
    let mut abs = [0u8; crate::map::OUT];
    if unsafe { crate::map::absolutize(crate::map::AT_FDCWD, template, &mut abs) } < 0 {
        crate::say::line(&[what, b": podbox cannot resolve the path, so the call fails"]);
        set_errno(2);
        return core::ptr::null_mut();
    }
    let abs_n = crate::map::strnlen(abs.as_ptr() as *const c_char, crate::map::OUT);
    match unsafe { crate::map::rewrite(&t, abs.as_ptr(), abs_n, out) } {
        -1 => template,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit"]);
            set_errno(ENAMETOOLONG);
            core::ptr::null_mut()
        }
        _ => {
            if !unsafe { place_template(what, template, out.as_ptr() as *const c_char) } {
                return core::ptr::null_mut();
            }
            template
        }
    }
}

/// # Safety
/// The payload's own contract for `mkstemp(3)`.
#[no_mangle]
pub unsafe extern "C" fn mkstemp(template: *mut c_char) -> c_int {
    let Some(f) = next_mkstemp() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let t = unsafe { map_template(b"mkstemp", template, &mut buf) };
    if t.is_null() {
        return -1;
    }
    unsafe { f(t) }
}

/// # Safety
/// The payload's own contract for `mkostemp(3)`.
#[no_mangle]
pub unsafe extern "C" fn mkostemp(template: *mut c_char, flags: c_int) -> c_int {
    let Some(f) = next_mkostemp() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let t = unsafe { map_template(b"mkostemp", template, &mut buf) };
    if t.is_null() {
        return -1;
    }
    unsafe { f(t, flags) }
}

/// # Safety
/// The payload's own contract for `mkdtemp(3)`.
#[no_mangle]
pub unsafe extern "C" fn mkdtemp(template: *mut c_char) -> *mut c_char {
    let Some(f) = next_mkdtemp() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let mut buf = [0u8; crate::map::OUT];
    let t = unsafe { map_template(b"mkdtemp", template, &mut buf) };
    if t.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { f(t) }
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
#[no_mangle]
pub unsafe extern "C" fn tmpfile() -> *mut c_void {
    let Some(f) = next_tmpfile() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    unsafe { f() }
}

/// # Safety
/// The payload's own contract for `tmpfile64(3)`.
#[no_mangle]
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
// Without `PODBOX_IDENTITY` every setter below calls through and names the
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
    say::line(&[
        what,
        b" failed with errno ",
        say::Num::new(e.unsigned_abs() as u64).as_bytes(),
        b"; podbox runs the payload as uid 0 and does not change it \
          (TODO/interpose.md T-0711)",
    ]);
    set_errno(e);
    rc
}

/// An invalid id stays invalid: it is the caller's mistake, not the wall,
/// so the real call answers it without the record.
fn valid(id: u32) -> bool {
    id != identity::NO_ID
}

/// # Safety
/// The payload's own contract for `setuid(2)`.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
        #[no_mangle]
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
