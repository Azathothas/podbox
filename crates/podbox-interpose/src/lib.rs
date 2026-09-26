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

pub mod device;
pub mod emulate;
pub mod identity;
pub mod map;
pub mod memo;
pub mod procfs;
pub mod real;
pub mod say;

use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

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
// T-0413: resolved with `dlsym` rather than linked, so an old libc past the
// wrapper (glibc 2.27) merely declines the emulation instead of refusing the
// whole object at load (T-1312's ceiling).
crate::real!(pub fn next_memfd_create = "memfd_create"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_creat = "creat"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_creat64 = "creat64"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_execve = "execve"(*const c_char, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execv = "execv"(*const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execvp = "execvp"(*const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execvpe = "execvpe"(*const c_char, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_execveat = "execveat"(c_int, *const c_char, *const *const c_char, *const *const c_char, c_int) -> c_int);
crate::real!(pub fn next_posix_spawn = "posix_spawn" @ "GLIBC_2.15"(*mut c_int, *const c_char, *const c_void, *const c_void, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_posix_spawnp = "posix_spawnp" @ "GLIBC_2.15"(*mut c_int, *const c_char, *const c_void, *const c_void, *const *const c_char, *const *const c_char) -> c_int);
crate::real!(pub fn next_chdir = "chdir"(*const c_char) -> c_int);
crate::real!(pub fn next_opendir = "opendir"(*const c_char) -> *mut c_void);
crate::real!(pub fn next_scandir = "scandir"(*const c_char, *mut *mut c_void, Filter, Compar) -> c_int);
crate::real!(pub fn next_readlink = "readlink"(*const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_readlinkat = "readlinkat"(c_int, *const c_char, *mut c_char, usize) -> isize);
crate::real!(pub fn next_realpath = "realpath" @ "GLIBC_2.3"(*const c_char, *mut c_char) -> *mut c_void);
crate::real!(pub fn next_canonicalize = "canonicalize_file_name"(*const c_char) -> *mut c_void);
crate::real!(pub fn next_get_current_dir_name = "get_current_dir_name"() -> *mut c_char);
crate::real!(fn next_malloc = "malloc"(usize) -> *mut c_void);
crate::real!(fn next_free = "free"(*mut c_void) -> ());
crate::real!(pub fn next_getxattr = "getxattr"(*const c_char, *const c_char, *mut c_void, usize) -> isize);
crate::real!(pub fn next_lgetxattr = "lgetxattr"(*const c_char, *const c_char, *mut c_void, usize) -> isize);
crate::real!(pub fn next_setxattr = "setxattr"(*const c_char, *const c_char, *const c_void, usize, c_int) -> c_int);
crate::real!(pub fn next_lsetxattr = "lsetxattr"(*const c_char, *const c_char, *const c_void, usize, c_int) -> c_int);
// T-0708: the emulated operations. All single-version on glibc (measured
// with objdump -T on the build host's libc: mknod@2.33, mount@2.2.5,
// unshare@2.4, clone@2.2.5, none with a compat beside the default), so
// plain `dlsym` answers each, and musl carries one version per name.
// `__xmknod` is the spelling payloads on glibc before 2.33 carry: `mknod`
// itself is new in 2.33, and older headers compile the call to `__xmknod`.
crate::real!(pub fn next_mknod = "mknod"(*const c_char, c_uint, u64) -> c_int);
crate::real!(pub fn next_xmknod = "__xmknod"(c_int, *const c_char, c_uint, *const c_void) -> c_int);
crate::real!(pub fn next_mount = "mount"(*const c_char, *const c_char, *const c_char, c_ulong, *const c_void) -> c_int);
crate::real!(pub fn next_unshare = "unshare"(c_int) -> c_int);
crate::real!(pub fn next_clone = "clone"(*mut c_void, *mut c_void, c_int, *mut c_void, *mut c_void, *mut c_void, *mut c_void) -> c_int);
crate::real!(pub fn next_close = "close"(c_int) -> c_int);
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
crate::real!(pub fn next_fchmodat = "fchmodat"(c_int, *const c_char, c_uint, c_int) -> c_int);
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
crate::real!(pub fn next_glob = "glob" @ "GLIBC_2.27"(*const c_char, c_int, ErrFunc, *mut c_void) -> c_int);
crate::real!(pub fn next_glob64 = "glob64" @ "GLIBC_2.27"(*const c_char, c_int, ErrFunc, *mut c_void) -> c_int);
crate::real!(pub fn next_ftw = "ftw"(*const c_char, FtwFunc, c_int) -> c_int);
crate::real!(pub fn next_ftw64 = "ftw64"(*const c_char, FtwFunc, c_int) -> c_int);
crate::real!(pub fn next_nftw = "nftw" @ "GLIBC_2.3.3"(*const c_char, NftwFunc, c_int, c_int) -> c_int);
crate::real!(pub fn next_nftw64 = "nftw64" @ "GLIBC_2.3.3"(*const c_char, NftwFunc, c_int, c_int) -> c_int);
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
path_nonneg!(getxattr, next_getxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(lgetxattr, next_lgetxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(listxattr, next_listxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);
path_nonneg!(llistxattr, next_llistxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);

path_ptr!(opendir, next_opendir, (path: *const c_char), path);
path_ptr!(fopen, next_fopen, (path: *const c_char, mode: *const c_char), path);
path_ptr!(fopen64, next_fopen64, (path: *const c_char, mode: *const c_char), path);
path_ptr!(freopen, next_freopen, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path);
path_ptr!(freopen64, next_freopen64, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path);

path_at_int!(mkdirat, next_mkdirat, (dirfd: c_int, path: *const c_char, mode: c_uint), dirfd, path);
path_at_int!(unlinkat, next_unlinkat, (dirfd: c_int, path: *const c_char, flags: c_int), dirfd, path);
path_at_int!(fchmodat, next_fchmodat, (dirfd: c_int, path: *const c_char, mode: c_uint, flags: c_int), dirfd, path);
path_at_int!(faccessat, next_faccessat, (dirfd: c_int, path: *const c_char, mode: c_int, flags: c_int), dirfd, path);
path_at_int!(utimensat, next_utimensat, (dirfd: c_int, path: *const c_char, times: *const c_void, flags: c_int), dirfd, path);
path_at_int!(openat2, next_openat2, (dirfd: c_int, path: *const c_char, how: *const c_void, size: usize), dirfd, path);

// ------------------------------------------- results read back in virtual names, T-0705
//
// The forward map rewrites what goes into the kernel. These six read the
// answer back out: where the real result lies under a TO prefix it is
// rewritten to the virtual FROM, so a payload comparing what it wrote
// with what it reads sees one tree. Unmatched answers travel untouched.
// Too long answers `ERANGE` rather than a truncation or a leak, which is
// the entry's ruled Decision. `readdir`'s `d_name` takes no reversal: a
// bare name carries no prefix, so there is nothing to match it against.

/// Copy `n` bytes at `src` plus a NUL into the caller's `size`-byte buffer.
///
/// Answers `ERANGE` where the buffer is too small, which is the real
/// call's own answer there and T-0705's ruled answer for a virtual name
/// that does not fit.
///
/// # Safety
/// `src` must be readable for `n` bytes, `dst` writable for `size` bytes.
unsafe fn copy_fit(src: *const u8, n: usize, dst: *mut c_char, size: usize) -> *mut c_char {
    if n + 1 > size {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst as *mut u8, n);
        *(dst as *mut u8).add(n) = 0;
    }
    dst
}

/// Copy `n` bytes at `p` into freshly allocated memory.
///
/// The allocating result shapes must hand back memory the caller frees,
/// so a reversed name needs its own allocation from the real `malloc`.
/// Returns null where `malloc` fails, with `errno` carrying its number.
///
/// # Safety
/// `p` must be readable for `n` bytes.
unsafe fn malloc_copy(p: *const u8, n: usize) -> *mut c_void {
    let Some(malloc) = next_malloc() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let q = unsafe { malloc(n + 1) };
    if q.is_null() {
        return q;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(p, q as *mut u8, n);
        *(q as *mut u8).add(n) = 0;
    }
    q
}

/// Free memory the real call allocated, where the allocator answers.
///
/// # Safety
/// `p` must be memory the real call allocated, or null.
unsafe fn free_ptr(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    if let Some(free) = next_free() {
        unsafe { free(p) };
    }
}

/// Reverse an allocated real path into an allocated virtual one.
///
/// Frees `real` on every path out. Unmatched answers the real pointer
/// itself: nothing virtual covers it, so the real path is the truth, not
/// a leak. Too long, or a real path at the buffer ceiling, answers
/// `ERANGE` rather than a truncation or a leak.
///
/// # Safety
/// `real` must be a non-null NUL-terminated string from the real call.
unsafe fn reverse_alloc(real: *mut c_void) -> *mut c_char {
    let real_n = crate::map::strnlen(real as *const c_char, crate::map::OUT);
    if real_n >= crate::map::OUT {
        unsafe { free_ptr(real) };
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, real as *const u8, real_n, &mut vbuf) };
    if v == -2 {
        unsafe { free_ptr(real) };
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    if v < 0 {
        return real as *mut c_char;
    }
    let q = unsafe { malloc_copy(vbuf.as_ptr(), v as usize) };
    unsafe { free_ptr(real) };
    q as *mut c_char
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    let Some(f) = crate::map::next_getcwd() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    // ⭐ Empty table, empty wrapper: with nothing to reverse against the
    // call goes straight through, so unmapped runs keep byte-identical
    // behaviour (and no length ceiling below the real call's own).
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(buf, size) };
    }
    let mut tmp = [0u8; crate::map::OUT];
    let r = unsafe { f(tmp.as_mut_ptr() as *mut c_char, crate::map::OUT) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    let real_n = crate::map::strnlen(tmp.as_ptr() as *const c_char, crate::map::OUT);
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, tmp.as_ptr(), real_n, &mut vbuf) };
    if buf.is_null() {
        // The glibc extension: allocate the answer.
        if v == -2 {
            set_errno(ERANGE);
            return core::ptr::null_mut();
        }
        if v < 0 {
            return unsafe { malloc_copy(tmp.as_ptr(), real_n) } as *mut c_char;
        }
        return unsafe { malloc_copy(vbuf.as_ptr(), v as usize) } as *mut c_char;
    }
    if v == -2 {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    // Unmatched answers the real path where it fits, exactly as the real
    // call would have; too small answers `ERANGE` on both sides.
    let (src, n) = if v < 0 {
        (tmp.as_ptr(), real_n)
    } else {
        (vbuf.as_ptr(), v as usize)
    };
    unsafe { copy_fit(src, n, buf, size) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn get_current_dir_name() -> *mut c_char {
    let Some(f) = next_get_current_dir_name() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f() };
    }
    let r = unsafe { f() };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { reverse_alloc(r as *mut c_void) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn realpath(path: *const c_char, resolved: *mut c_char) -> *mut c_char {
    let Some(f) = next_realpath() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"realpath", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(p, resolved) } as *mut c_char;
    }
    let path = p;
    if resolved.is_null() {
        let r = unsafe { f(path, core::ptr::null_mut()) };
        if r.is_null() {
            return core::ptr::null_mut();
        }
        return unsafe { reverse_alloc(r) };
    }
    // The caller sized the buffer at PATH_MAX: POSIX gives the call no size
    // argument, so glibc assumes that scale, and OUT matches it. A virtual
    // name past it answers `ERANGE` rather than overflowing.
    let mut tmp = [0u8; crate::map::OUT];
    let r = unsafe { f(path, tmp.as_mut_ptr() as *mut c_char) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    let real_n = crate::map::strnlen(tmp.as_ptr() as *const c_char, crate::map::OUT);
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, tmp.as_ptr(), real_n, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    let (src, n) = if v < 0 {
        (tmp.as_ptr(), real_n)
    } else {
        (vbuf.as_ptr(), v as usize)
    };
    unsafe { copy_fit(src, n, resolved, crate::map::OUT) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn canonicalize_file_name(path: *const c_char) -> *mut c_char {
    let Some(f) = next_canonicalize() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe {
        crate::map::prepare(
            b"canonicalize_file_name",
            crate::map::AT_FDCWD,
            path,
            &mut buf,
        )
    };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(p) } as *mut c_char;
    }
    let r = unsafe { f(p) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { reverse_alloc(r) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn readlink(path: *const c_char, buf: *mut c_char, n: usize) -> isize {
    let fail: isize = -1;
    let Some(f) = crate::map::next_readlink() else {
        set_errno(EINVAL);
        return fail;
    };
    let mut ibuf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"readlink", crate::map::AT_FDCWD, path, &mut ibuf) };
    if p.is_null() {
        return fail;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        let rc = unsafe { f(p, buf, n) };
        // T-0413: where no procfs is mounted the real call fails `ENOENT`,
        // and only there does the emulation answer.
        if rc < 0 && errno() == crate::procfs::ENOENT {
            if let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) } {
                return e;
            }
        }
        return rc;
    }
    let rc = unsafe { f(p, buf, n) };
    if rc < 0 {
        // T-0413, as above. `path` is the caller's own spelling only where
        // the table left it untouched (`p` is then that same pointer): a
        // deliberate mapping of `/dev/fd` wins over this emulation, and
        // the `/proc` leaves never rewrite (T-0707 excludes them).
        if p == path && errno() == crate::procfs::ENOENT {
            if let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) } {
                return e;
            }
        }
        return rc;
    }
    // The answer is bytes, not a string: no NUL terminates it.
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, buf as *const u8, rc as usize, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return fail;
    }
    if v < 0 {
        return rc;
    }
    if v as usize > n {
        set_errno(ERANGE);
        return fail;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(vbuf.as_ptr(), buf as *mut u8, v as usize);
    }
    v
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    n: usize,
) -> isize {
    let fail: isize = -1;
    let Some(f) = next_readlinkat() else {
        set_errno(EINVAL);
        return fail;
    };
    let mut ibuf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"readlinkat", dirfd, path, &mut ibuf) };
    if p.is_null() {
        return fail;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        let rc = unsafe { f(dirfd, p, buf, n) };
        // T-0413, as in `readlink`: only `ENOENT` answers.
        if rc < 0 && errno() == crate::procfs::ENOENT {
            if let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) } {
                return e;
            }
        }
        return rc;
    }
    let rc = unsafe { f(dirfd, p, buf, n) };
    if rc < 0 {
        // T-0413, as in `readlink`: untouched spellings only.
        if p == path && errno() == crate::procfs::ENOENT {
            if let Some(e) = unsafe { proc_emulate_readlink(path, buf, n) } {
                return e;
            }
        }
        return rc;
    }
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, buf as *const u8, rc as usize, &mut vbuf) };
    if v == -2 {
        set_errno(ERANGE);
        return fail;
    }
    if v < 0 {
        return rc;
    }
    if v as usize > n {
        set_errno(ERANGE);
        return fail;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(vbuf.as_ptr(), buf as *mut u8, v as usize);
    }
    v
}

// --------------------------------------- T-0413: `/proc/self` emulation
//
// Where no procfs is mounted the real call fails `ENOENT`, and only then do
// these answer: the wrappers below try the real call first and call in here
// on exactly that errno. Each answers `Some` where it served (a descriptor,
// or -1 with the kernel's exact errno) and `None` where it declines, and a
// decline preserves the real failure's errno. `readlink` truncates into a
// short buffer the way the kernel does; nothing here invents a byte.

/// Fill `st` with the descriptor's real metadata.
///
/// Through `next_fstat`, never the wrapper two lines down the file: the
/// wrapper would report the memo over the bytes this emulation reasons
/// about, and a memo hit is a statement about ownership, not about the
/// descriptor's type.
pub(crate) unsafe fn proc_fstat(fdno: u32, st: &mut [u8; crate::procfs::STAT_LEN]) -> bool {
    let Some(fstat) = next_fstat() else {
        return false;
    };
    unsafe { fstat(fdno as c_int, st.as_mut_ptr() as *mut c_void) == 0 }
}

/// Serve `open` of a descriptor leaf, T-0413.
unsafe fn proc_open_fd(fdno: u32, flags: c_int) -> Option<c_int> {
    use crate::procfs::*;
    let mut st = [0u8; STAT_LEN];
    if !unsafe { proc_fstat(fdno, &mut st) } {
        // No such descriptor: the kernel's answer for the leaf is ENOENT,
        // with procfs or without it.
        set_errno(ENOENT);
        return Some(-1);
    }
    let mode = mode_of(&st);
    if mode & S_IFMT == S_IFSOCK {
        // A socket has no reopen: the kernel answers ENXIO, exactly.
        set_errno(ENXIO);
        return Some(-1);
    }
    if mode & S_IFMT != S_IFIFO {
        return None;
    }
    if unsafe { fs_magic(fdno as c_int) } != Some(PIPEFS_MAGIC) {
        // A named fifo reopens with blocking semantics no duplicate
        // reproduces: refused, never approximated. The `fstatfs` may have
        // clobbered the real failure's errno, so it is restored.
        set_errno(ENOENT);
        return None;
    }
    let Some(orig) = (unsafe { open_flags(fdno as c_int) }) else {
        set_errno(ENOENT);
        return Some(-1);
    };
    match open_answer(orig, flags) {
        OpenAnswer::Dup => {
            let fd = unsafe { dup_of(fdno as c_int, flags & O_CLOEXEC != 0) };
            if fd < 0 {
                return Some(-1);
            }
            // No tally behind it, no emulation: the duplicate closes and
            // the failure reads as the memo write's own errno.
            if !crate::emulate::tally(crate::emulate::OP_PROC, 1, 0, 0) {
                let e = errno();
                if let Some(close) = next_close() {
                    unsafe { close(fd) };
                }
                set_errno(e);
                return Some(-1);
            }
            Some(fd)
        }
        OpenAnswer::Fail(e) => {
            set_errno(e);
            Some(-1)
        }
        OpenAnswer::Pass => None,
    }
}

/// Serve `readlink` of a descriptor leaf, T-0413.
unsafe fn proc_readlink_fd(fdno: u32, buf: *mut c_char, n: usize) -> Option<isize> {
    use crate::procfs::*;
    let mut st = [0u8; STAT_LEN];
    if !unsafe { proc_fstat(fdno, &mut st) } {
        set_errno(ENOENT);
        return Some(-1);
    }
    let mode = mode_of(&st);
    let prefix = if mode & S_IFMT == S_IFSOCK {
        SOCKET_PREFIX
    } else if mode & S_IFMT == S_IFIFO && unsafe { fs_magic(fdno as c_int) } == Some(PIPEFS_MAGIC) {
        PIPE_PREFIX
    } else {
        // A path the emulation cannot name exactly (a regular file, a
        // device, a named fifo): refused, with the real errno restored
        // past the `fstatfs` above.
        set_errno(crate::procfs::ENOENT);
        return None;
    };
    let mut link = [0u8; 32];
    let len = fd_link(prefix, ino_of(&st), &mut link);
    let take = core::cmp::min(len, n);
    unsafe {
        core::ptr::copy_nonoverlapping(link.as_ptr(), buf as *mut u8, take);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 2, 0, 0) {
        set_errno(ENOENT);
        return Some(-1);
    }
    Some(take as isize)
}

/// The caller-resolved guest path, validated: a leading slash, no interior
/// NUL, short enough to hand on. Anything else is no variable rather than a
/// guess at one.
unsafe fn proc_exe_value(out: &mut [u8; crate::map::OUT]) -> Option<usize> {
    let (p, n) = unsafe { crate::map::lookup_env(crate::procfs::GUEST_EXE_VAR) }?;
    if n == 0 || n + 1 >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(p, n) };
    if bytes.contains(&0) || !bytes.starts_with(b"/") {
        return None;
    }
    out[..n].copy_from_slice(bytes);
    out[n] = 0;
    Some(n)
}

/// Serve `readlink` of `/proc/self/exe`, T-0413.
unsafe fn proc_readlink_exe(buf: *mut c_char, n: usize) -> Option<isize> {
    let mut guest = [0u8; crate::map::OUT];
    let len = unsafe { proc_exe_value(&mut guest) }?;
    let take = core::cmp::min(len, n);
    unsafe {
        core::ptr::copy_nonoverlapping(guest.as_ptr(), buf as *mut u8, take);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 3, 0, 0) {
        set_errno(crate::procfs::ENOENT);
        return Some(-1);
    }
    Some(take as isize)
}

/// Serve `open` of `/proc/self/exe`, T-0413.
///
/// The guest path goes back through the table: it names the image as the
/// payload sees it, so a deliberate mapping applies to it like any other
/// guest path. The flags ride through to the same file, which is what the
/// kernel applies them to. Counted only where the open succeeded: a failed
/// open told the payload nothing but the kernel's errno.
unsafe fn proc_open_exe(flags: c_int, mode: c_uint) -> Option<c_int> {
    let mut guest = [0u8; crate::map::OUT];
    let _ = unsafe { proc_exe_value(&mut guest) }?;
    let mut rewritten = [0u8; crate::map::OUT];
    let p = unsafe {
        crate::map::prepare_literal(b"open", guest.as_ptr() as *const c_char, &mut rewritten)
    };
    if p.is_null() {
        return Some(-1);
    }
    let Some(open) = next_open() else {
        set_errno(EINVAL);
        return Some(-1);
    };
    let fd = unsafe { open(p, flags, mode) };
    // No tally behind it, no emulation: the open closes and the failure
    // reads as the memo write's own errno.
    if fd >= 0 && !crate::emulate::tally(crate::emulate::OP_PROC, 4, 0, 0) {
        let e = errno();
        if let Some(close) = next_close() {
            unsafe { close(fd) };
        }
        set_errno(e);
        return Some(-1);
    }
    Some(fd)
}

/// Serve `open` of a mount-table file, T-0413.
///
/// The fixture is generated into a `memfd_create` descriptor from live
/// topology (the payload root's own device and filesystem) plus the
/// recorded T-0708 emulated mounts, per `TOOL.md` section 10. Where the
/// memo descriptor was not handed, or the old libc has no `memfd_create`,
/// or any step fails, this declines and the real failure stands:
/// an uncounted or half-written table is worse than an absent one.
unsafe fn proc_open_mounts(kind: crate::procfs::MountFile, flags: c_int) -> Option<c_int> {
    use crate::procfs::*;
    let saved = errno();
    let fail = |e: c_int| -> Option<c_int> {
        set_errno(e);
        None
    };
    let Some(memo) = crate::memo::memo_fd() else {
        return fail(saved);
    };
    let Some(memfd_create) = next_memfd_create() else {
        return fail(saved);
    };
    let name = b"podbox-mounts\0";
    // The only flag this object ever sets; the request's own close-on-exec
    // bit is reconciled after the serve so the answer stays exact.
    const MFD_CLOEXEC: c_uint = 1;
    let m = unsafe { memfd_create(name.as_ptr() as *const c_char, MFD_CLOEXEC) };
    if m < 0 {
        return fail(saved);
    }
    let close_quiet = |m: c_int| {
        if let Some(close) = next_close() {
            unsafe { close(m) };
        }
    };
    let Some(stat) = next_stat() else {
        close_quiet(m);
        return fail(saved);
    };
    let Some(statfs) = next_statfs() else {
        close_quiet(m);
        return fail(saved);
    };
    let slash = b"/\0";
    let mut st = [0u8; STAT_LEN];
    let mut fs = [0u8; STATFS_LEN];
    if unsafe {
        stat(
            slash.as_ptr() as *const c_char,
            st.as_mut_ptr() as *mut c_void,
        )
    } != 0
        || unsafe {
            statfs(
                slash.as_ptr() as *const c_char,
                fs.as_mut_ptr() as *mut c_void,
            )
        } != 0
    {
        close_quiet(m);
        return fail(saved);
    }
    let dev = u64::from_ne_bytes(st[0..8].try_into().unwrap_or([0u8; 8]));
    let f_type = u64::from_ne_bytes(fs[0..8].try_into().unwrap_or([0u8; 8]));
    let f_flags = u64::from_ne_bytes(
        fs[STATFS_FLAGS..STATFS_FLAGS + 8]
            .try_into()
            .unwrap_or([0u8; 8]),
    );
    let rw = f_flags & ST_RDONLY == 0;
    // ⚠ One reader beside the writers: the memo scan rewinds first like
    // `memo::lookup` does, and shares its exposure to a concurrent
    // writer's append (writes are whole records under `O_APPEND`; a scan
    // interleaved with one sees whole records or a torn tail it drops).
    let mut area = [0u8; 4096];
    let mut recs: [RecMount; 64] = [RecMount {
        src_start: 0,
        src_len: 0,
        tgt_start: 0,
        tgt_len: 0,
        ro: false,
    }; 64];
    let taken = unsafe { scan_mounts(memo, &mut area, &mut recs) };
    let mut out = [0u8; FIXTURE_MAX];
    let n = match kind {
        MountFile::Mounts => render_mounts(
            Fstype::of_magic(f_type),
            rw,
            &area,
            &recs[..taken],
            &mut out,
        ),
        MountFile::MountInfo => render_mountinfo(
            major_of(dev),
            minor_of(dev),
            Fstype::of_magic(f_type),
            rw,
            &area,
            &recs[..taken],
            &mut out,
        ),
    };
    if !unsafe { write_all(m, &out[..n]) } || !unsafe { rewind(m) } {
        close_quiet(m);
        return fail(saved);
    }
    if !crate::emulate::tally(crate::emulate::OP_PROC, 5, 0, 0) {
        let e = errno();
        close_quiet(m);
        set_errno(e);
        return None;
    }
    // The serve is close-on-exec; the request may not have asked for it.
    // A descriptor flag, not a description one, so reconciling it touches
    // nothing shared.
    if flags & crate::procfs::O_CLOEXEC == 0 && !unsafe { set_cloexec(m, false) } {
        let e = errno();
        close_quiet(m);
        set_errno(e);
        return None;
    }
    Some(m)
}

/// The image-side link a `/dev` spelling resolves through.
///
/// `/proc/self` leaves are kernel-named and need no image path, so this
/// answers true for them without a call. The `/dev` spellings resolve
/// through an image link (`/dev/fd`, `/dev/stdin`, ...), and where the
/// image holds no such link the kernel answers `ENOENT`: the emulation
/// must not answer past a link the image does not have, so it checks
/// first and declines where the check fails, with the real errno
/// restored past the check itself.
unsafe fn proc_image_link_ok(path: &[u8]) -> bool {
    let link: &[u8] = if path.starts_with(b"/dev/fd/") {
        b"/dev/fd\0"
    } else if path == b"/dev/stdin\0" || path == b"/dev/stdout\0" || path == b"/dev/stderr\0" {
        path
    } else {
        return true;
    };
    let Some(lstat) = next_lstat() else {
        return false;
    };
    let mut st = [0u8; crate::procfs::STAT_LEN];
    if unsafe {
        lstat(
            link.as_ptr() as *const c_char,
            st.as_mut_ptr() as *mut c_void,
        )
    } != 0
    {
        set_errno(crate::procfs::ENOENT);
        return false;
    }
    true
}

/// What a standard stream link reads back: its target, exactly as the
/// kernel reports it on a live `/proc`. `None` for anything else.
fn proc_stdio_target(path: &[u8]) -> Option<&'static [u8]> {
    if path == b"/dev/stdin\0" {
        Some(b"/proc/self/fd/0")
    } else if path == b"/dev/stdout\0" {
        Some(b"/proc/self/fd/1")
    } else if path == b"/dev/stderr\0" {
        Some(b"/proc/self/fd/2")
    } else {
        None
    }
}

/// Serve `open` of a `/proc/self` leaf after the real call failed, T-0413.
///
/// Only absolute spellings the matchers name, and only where the table
/// left the path untouched: a deliberate mapping wins over this
/// emulation, and the wrappers establish the untouched part by comparing
/// pointers before calling here.
unsafe fn proc_emulate_open(path: *const c_char, flags: c_int, mode: c_uint) -> Option<c_int> {
    if path.is_null() {
        return None;
    }
    let n = crate::map::strnlen(path, crate::map::OUT);
    if n == 0 || n >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, n + 1) };
    // The `/dev` spellings resolve through an image link; the `/proc`
    // spellings are kernel-named. Nothing answers past a link the image
    // does not hold.
    if !unsafe { proc_image_link_ok(bytes) } {
        return None;
    }
    if let Some(fdno) = crate::procfs::fd_number(bytes) {
        return unsafe { proc_open_fd(fdno, flags) };
    }
    if crate::procfs::is_self_exe(bytes) {
        return unsafe { proc_open_exe(flags, mode) };
    }
    if let Some(kind) = crate::procfs::mount_file(bytes) {
        return unsafe { proc_open_mounts(kind, flags) };
    }
    None
}

/// Serve `readlink` of a `/proc/self` leaf after the real call failed,
/// T-0413. Same spelling rules as [`proc_emulate_open`].
unsafe fn proc_emulate_readlink(path: *const c_char, buf: *mut c_char, n: usize) -> Option<isize> {
    if path.is_null() || buf.is_null() {
        return None;
    }
    let len = crate::map::strnlen(path, crate::map::OUT);
    if len == 0 || len >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, len + 1) };
    // The standard stream links read back their target, exactly as the
    // kernel reports them: they are links, not leaves.
    if let Some(target) = proc_stdio_target(bytes) {
        if !unsafe { proc_image_link_ok(bytes) } {
            return None;
        }
        let take = core::cmp::min(target.len(), n);
        unsafe {
            core::ptr::copy_nonoverlapping(target.as_ptr(), buf as *mut u8, take);
        }
        if !crate::emulate::tally(crate::emulate::OP_PROC, 2, 0, 0) {
            set_errno(crate::procfs::ENOENT);
            return Some(-1);
        }
        return Some(take as isize);
    }
    if !unsafe { proc_image_link_ok(bytes) } {
        return None;
    }
    if let Some(fdno) = crate::procfs::fd_number(bytes) {
        return unsafe { proc_readlink_fd(fdno, buf, n) };
    }
    if crate::procfs::is_self_exe(bytes) {
        return unsafe { proc_readlink_exe(buf, n) };
    }
    None
}

// --------------------------------------- the emulated operations, T-0708
//
// Four calls the runtime cannot honour, counted in the memo file beside
// the ownership records (`emulate`, `inspect` reading them back). The one
// rule across all four: no tally behind it, no emulation. Where the memo
// descriptor was not handed, each falls back to the real call and its
// honest failure instead of reporting an uncounted success.

/// Linux `O_*` for the mknod stand-in, UAPI `asm-generic/fcntl.h`.
/// Arch-generic on Linux.
const O_WRONLY: c_int = 0o1;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;

/// File-type bits, UAPI `linux/stat.h`. A directory is refused with the
/// real call's answer; everything else becomes the regular file the
/// specification names.
const S_IFMT: c_uint = 0o170000;
const S_IFDIR: c_uint = 0o40000;

/// Say the `CLONE_NEWNET` strip out loud, T-0708's own rule. An empty netns
/// is `ENETUNREACH` for every connection, which reads as a network outage
/// rather than as a stripped flag, so the strip is never silent.
fn say_newnet(what: &[u8], flags: u32) {
    crate::say::line(&[
        what,
        b": stripping CLONE_NEWNET from flags=",
        crate::say::Num::new(flags as u64).as_bytes(),
        b": no new network namespace is made, so every connection uses \
          the host network (TODO/interpose.md T-0708)",
    ]);
}

/// Create the regular-file stand-in for `mknod`, T-0708.
///
/// The path is prepared, a directory is refused, `O_EXCL` answers the
/// existing file honestly, and the mode's permission bits ride through
/// `open` (which applies the umask exactly as the real call would). The
/// device is ignored: there is no node to put it in.
///
/// Returns 0, or -1 with the kernel's errno.
///
/// # Safety
/// `path` must be readable up to its NUL within `OUT` bytes.
unsafe fn mknod_make(path: *const c_char, mode: c_uint) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"mknod", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    if mode & S_IFMT == S_IFDIR {
        set_errno(EPERM);
        return -1;
    }
    let Some(open) = next_open() else {
        set_errno(EINVAL);
        return -1;
    };
    let Some(close) = next_close() else {
        set_errno(EINVAL);
        return -1;
    };
    let fd = unsafe { open(p, O_WRONLY | O_CREAT | O_EXCL, mode & 0o7777) };
    if fd < 0 {
        return -1;
    }
    unsafe { close(fd) };
    // No tally behind it, no success: the tally is what counts this
    // emulation, so one that does not land (a memo write failing under a
    // full disk) unmakes the stand-in and fails with the write's own errno
    // rather than reporting a file nothing counted.
    if crate::emulate::tally(crate::emulate::OP_MKNOD, mode, 0, 0) {
        return 0;
    }
    let e = errno();
    if let Some(unlink) = next_unlink() {
        unsafe { unlink(p) };
    }
    set_errno(e);
    -1
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn mknod(path: *const c_char, mode: c_uint, dev: u64) -> c_int {
    let _ = dev;
    // No tally behind it, no emulation: the real call and its honest
    // failure (this runtime refuses the node) rather than an uncounted
    // success.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_mknod() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(path, mode, dev) };
    }
    unsafe { mknod_make(path, mode) }
}

/// # Safety
/// The payload's own contract for this entry point: `__xmknod` is the
/// spelling glibc before 2.33 compiles `mknod` to, version first.
#[no_mangle]
pub unsafe extern "C" fn __xmknod(
    ver: c_int,
    path: *const c_char,
    mode: c_uint,
    dev: *const c_void,
) -> c_int {
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_xmknod() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(ver, path, mode, dev) };
    }
    unsafe { mknod_make(path, mode) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn mount(
    source: *const c_char,
    target: *const c_char,
    fstype: *const c_char,
    flags: c_ulong,
    data: *const c_void,
) -> c_int {
    let _ = fstype;
    let _ = data;
    // No tally behind it, no emulation.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_mount() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(source, target, fstype, flags, data) };
    }
    let mut tbuf = [0u8; crate::map::OUT];
    let t = unsafe { crate::map::prepare(b"mount", crate::map::AT_FDCWD, target, &mut tbuf) };
    if t.is_null() {
        return -1;
    }
    // The source is often not a path at all (`proc`, `tmpfs`, `none`): an
    // absolute one is prepared, anything else travels as the caller said it.
    // ⛔ `sbuf` lives to the end of the function because the tally below
    // reads through `s`: returning through a narrower scope would borrow a
    // dead buffer.
    let mut sbuf = [0u8; crate::map::OUT];
    let s = if !source.is_null() && unsafe { *(source as *const u8) } == b'/' {
        let s = unsafe { crate::map::prepare(b"mount", crate::map::AT_FDCWD, source, &mut sbuf) };
        if s.is_null() {
            return -1;
        }
        s
    } else {
        source
    };
    let sn = if s.is_null() {
        0
    } else {
        crate::map::strnlen(s, crate::map::OUT)
    };
    let tn = crate::map::strnlen(t, crate::map::OUT);
    if sn >= crate::map::OUT || tn >= crate::map::OUT {
        set_errno(ENAMETOOLONG);
        return -1;
    }
    // Mount flags fit 32 bits (every `MS_*` is one), so the cast keeps them.
    if unsafe { crate::emulate::tally_mount(flags as u32, s as *const u8, sn, t as *const u8, tn) }
    {
        return 0;
    }
    let Some(f) = next_mount() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { f(source, target, fstype, flags, data) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[no_mangle]
pub unsafe extern "C" fn unshare(flags: c_int) -> c_int {
    // A flag-less call succeeds genuinely: forward it rather than counting
    // a no-op as an emulation.
    if flags == 0 {
        let Some(f) = next_unshare() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(flags) };
    }
    // No tally behind it, no emulation.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_unshare() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(flags) };
    }
    if (flags as u32) & crate::emulate::CLONE_NEWNET != 0 {
        say_newnet(b"unshare", flags as u32);
    }
    if crate::emulate::tally(crate::emulate::OP_UNSHARE, flags as u32, 0, 0) {
        return 0;
    }
    // The tally did not land: the real call and whatever it answers,
    // rather than an uncounted success.
    let Some(f) = next_unshare() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { f(flags) }
}

/// # Safety
/// The payload's own contract for this entry point: seven registers, so a
/// caller passing four leaves garbage the callee never reads, and a caller
/// passing seven has every one forwarded. Stable Rust cannot declare the
/// C-variadic shape, so this is the fixed-arity equivalent (the `open`
/// family in this file is the precedent).
#[no_mangle]
pub unsafe extern "C" fn clone(
    func: *mut c_void,
    stack: *mut c_void,
    flags: c_int,
    arg: *mut c_void,
    ptid: *mut c_void,
    tls: *mut c_void,
    ctid: *mut c_void,
) -> c_int {
    let Some(f) = next_clone() else {
        set_errno(EINVAL);
        return -1;
    };
    let stripped = crate::emulate::strip_ns(flags as u32) as c_int;
    // Nothing of ours in the flags: forward exactly, count nothing. This
    // is the path every thread creation takes.
    if stripped == flags {
        return unsafe { f(func, stack, flags, arg, ptid, tls, ctid) };
    }
    // No tally behind it, no emulation: forward the caller's own flags.
    if crate::memo::memo_fd().is_none() {
        return unsafe { f(func, stack, flags, arg, ptid, tls, ctid) };
    }
    if (flags as u32) & crate::emulate::CLONE_NEWNET != 0 {
        say_newnet(b"clone", flags as u32);
    }
    if crate::emulate::tally(crate::emulate::OP_CLONE, flags as u32, stripped as u32, 0) {
        return unsafe { f(func, stack, stripped, arg, ptid, tls, ctid) };
    }
    // The tally did not land: the caller's own flags go through, so
    // whatever the kernel answers is genuine rather than uncounted.
    unsafe { f(func, stack, flags, arg, ptid, tls, ctid) }
}

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
            let rc = unsafe { f(p, $flags, mode) };
            // T-0413: where no procfs is mounted the real call fails
            // `ENOENT`, and only there does the emulation answer. `p` is
            // the caller's own pointer exactly where the table left the
            // path untouched, so a deliberate mapping always wins.
            // T-0501 beside it: a `--device` guest path names no node in
            // the image, so the same `ENOENT` gate serves the duplicate
            // of the host descriptor the entry opened before the chroot.
            if rc < 0 && errno() == crate::procfs::ENOENT && p == $path {
                if let Some(e) = unsafe { proc_emulate_open($path, $flags, mode) } {
                    return e;
                }
                if let Some(e) = unsafe { crate::device::serve_open($path, $flags) } {
                    return e;
                }
            }
            rc
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
            // T-0413, as above. Relative paths never name the emulated
            // leaves (the matchers anchor on absolute spellings), so the
            // pointer comparison is the whole guard here too. T-0501's
            // device serve rides the same gate.
            let rc = unsafe { f($dirfd, p, $flags, mode) };
            if rc < 0 && p == $path && errno() == crate::procfs::ENOENT {
                if let Some(e) = unsafe { proc_emulate_open($path, $flags, mode) } {
                    return e;
                }
                if let Some(e) = unsafe { crate::device::serve_open($path, $flags) } {
                    return e;
                }
            }
            rc
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
        b"; podbox runs the payload as uid 0 and does not change it \
          (TODO/interpose.md T-0711)",
    ]);
    set_errno(e);
    rc
}

/// The wall's usual errno names, T-0711. Kernel UAPI numbers, identical
/// on every Linux architecture this object loads on. Anything else
/// answers empty, and the caller prints the number alone.
fn errno_name(e: c_int) -> &'static [u8] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    extern "C" {
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
        let dir = std::env::temp_dir().join(format!("podbox-fchmodat-{}", std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("podbox-realpath-{}", std::process::id()));
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
}
