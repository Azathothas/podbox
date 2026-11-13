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
//! podvm swallows `EPERM` **and** `EINVAL` and records which.

#![forbid(unsafe_op_in_unsafe_fn)]
// ⭐ T-1601. `invalid_runtime_symbol_definitions` is deny-by-default and
// fires on the `open` family below, whose fixed third argument is deliberate:
// stable Rust cannot define a C-variadic function (rust-lang/rust#44930).
// The allow on the macro body would itself warn as an unknown lint on a
// toolchain without the lint, and this keeps it a no-op there instead.
#![allow(unknown_lints)]
// `c_char` is `i8` on x86_64 and `u8` on aarch64 (T-1721): a cast of a
// `*const c_char` to `*const u8` is needed on one and a no-op on the other.
#![allow(clippy::unnecessary_cast)]
// ⛔ x86_64 ONLY, and refused by name rather than built wrong. The `stat`
// offsets below are this architecture's, measured under both libcs; on i686 and
// arm they differ, and an object that wrote a uid at the wrong offset would
// corrupt whatever field is there. `scripts/build-interpose.sh` builds the two
// x86_64 targets, so this is the set that exists rather than a limitation
// invented here.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!(
    "podvm-interpose serves x86_64 and aarch64: src/lib.rs carries their \
     `struct stat` field offsets, measured under glibc and musl \
     (experiments 105 and 439), and another architecture needs its own \
     measurement (TODO/interpose.md T-0704, T-1721)"
);

pub mod device;
pub mod emulate;
pub mod identity;
pub mod map;
pub mod memo;
pub mod oflags;
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
#[cfg(target_arch = "x86_64")]
const ST_UID: usize = 28;
#[cfg(target_arch = "x86_64")]
const ST_GID: usize = 32;
// aarch64: the generic layout of the kernel, the same under glibc and musl
// (experiments/439-stat-layout.sh): uid=24 gid=28 of 128 bytes.
#[cfg(target_arch = "aarch64")]
const ST_UID: usize = 24;
#[cfg(target_arch = "aarch64")]
const ST_GID: usize = 28;

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
/// `podvm_probe::sys::makedev` uses, because a memo written under one encoding
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
/// that named a gid and left the uid alone must not make podvm invent a uid.
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
            // podvm wrote without setting the bit is one the caller is entitled
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

unsafe extern "C" {
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

/// Is this the wall podvm exists for?
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
// TODO/cli.md T-1694: the global `malloc` and `free` (`RTLD_DEFAULT`),
// the allocator that the payload's own `free` pairs with.
fn next_malloc() -> Option<unsafe extern "C" fn(usize) -> *mut c_void> {
    static NEXT: real::Next = real::Next::new();
    let p = unsafe { NEXT.get_global(b"malloc\0") };
    // SAFETY: `malloc` has this signature.
    (!p.is_null()).then(|| unsafe {
        core::mem::transmute::<*mut c_void, unsafe extern "C" fn(usize) -> *mut c_void>(p)
    })
}
fn next_free() -> Option<unsafe extern "C" fn(*mut c_void)> {
    static NEXT: real::Next = real::Next::new();
    let p = unsafe { NEXT.get_global(b"free\0") };
    // SAFETY: `free` has this signature.
    (!p.is_null()).then(|| unsafe {
        core::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut c_void)>(p)
    })
}
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
crate::real!(pub fn next_mkstemp64 = "mkstemp64"(*mut c_char) -> c_int);
crate::real!(pub fn next_mkostemp64 = "mkostemp64"(*mut c_char, c_int) -> c_int);
crate::real!(pub fn next_mkstemps = "mkstemps"(*mut c_char, c_int) -> c_int);
crate::real!(pub fn next_mkostemps = "mkostemps"(*mut c_char, c_int, c_int) -> c_int);
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

#[macro_use]
mod owner_calls;
pub use owner_calls::*;

#[macro_use]
mod stat_calls;
pub use stat_calls::*;

mod path_calls;
pub use path_calls::*;

mod chk_calls;
pub use chk_calls::*;

mod proc_mounts;
pub use proc_mounts::*;

#[macro_use]
mod create_calls;
pub use create_calls::*;

#[macro_use]
mod identity_calls;
pub use identity_calls::*;

mod dl_calls;
pub use dl_calls::*;

mod exec;
pub use exec::calls::*;

mod raw_calls;
pub use raw_calls::*;

mod sock_calls;
pub use sock_calls::*;

#[cfg(test)]
mod tests;
