//! `syscall(3)`, T-1697: the door that a raw system call takes past the
//! path table.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-1697. libuv stats a
//! path with `syscall(SYS_statx, ...)`, not with the `statx` of libc. Node
//! imports `syscall` from libc and no `statx`, so before this entry point the
//! kernel resolved each guest path of `fs.stat`, `fs.lstat` and the module
//! loader on the host, and npm could not find its modules. The drive of
//! T-1697 (`experiments/results/raw-statx-1697.txt`) has the strace lines.
//!
//! ⭐ **Three arms, chosen by the number.**
//!
//! 1. A number that an entry point of this object maps goes to that entry
//!    point, with the arguments in the kernel's order. So a raw `statx` gets
//!    the same rewrite, walk, exe override and owner report as a call of
//!    `statx`, and each action keeps one implementation.
//! 2. A number that takes a path and that no entry point maps is refused
//!    with `ENOSYS` and a named line, where a table is set. A caller of a
//!    raw call must handle `ENOSYS` for an older kernel. `io_uring_setup`
//!    is refused the same way: a ring can open, stat or rename a path that
//!    the table never sees. libuv then uses `epoll_ctl` and its thread
//!    pool, so its line prints only under `PODVM_INTERPOSE_DEBUG`. With no
//!    table, these numbers go to the kernel unchanged.
//! 3. Each other number goes to the kernel unchanged, with all six
//!    arguments, through this object's own `syscall` instruction.
//!
//! ⛔ **The forward is an instruction, not the next `syscall`.** libc's
//! `syscall` is C-variadic, and a call through a fixed pointer does not set
//! `al`, which a variadic prologue reads. The libc function is one `syscall`
//! instruction and the `errno` rule below, so the instruction is the same
//! call. Rust std inside this object calls `syscall` for `futex`, and that
//! call takes arm 3.
//!
//! ⚠ The fixed seventh argument: stable Rust cannot define a C-variadic
//! function (rust-lang/rust#44930). On x86_64 SysV a variadic caller puts
//! the first six integer arguments in registers and the seventh on the
//! stack, where a fixed seventh parameter reads it. A caller that passes
//! fewer leaves values that the kernel does not read for that number.
//!
//! ⚠ Limits. A route answers what its entry point answers, also where the
//! payload's libc has no symbol for it (`EINVAL`, as the entry point does);
//! `statx` and `openat2` answer `ENOSYS` there instead, so the caller falls
//! back to a call that this object maps. A program that issues the
//! `syscall` instruction itself (a static program, Go, liburing) does not
//! come through here: T-1652 owns that.

use super::*;
use core::ffi::c_long;

const ENOSYS: c_int = 38;

mod nr;
pub(crate) use nr::*;

/// One system call, with the `errno` rule of libc's `syscall`: a result in
/// `-4095..=-1` is `-errno`.
///
/// # Safety
/// The kernel's own contract for number `n`.
pub(crate) unsafe fn forward(n: c_long, a: [c_long; 6]) -> c_long {
    let r: c_long;
    // SAFETY: the caller upholds the contract of number `n`; the
    // instruction changes `rcx` and `r11`, which are named.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => r,
            in("rdi") a[0],
            in("rsi") a[1],
            in("rdx") a[2],
            in("r10") a[3],
            in("r8") a[4],
            in("r9") a[5],
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    // SAFETY: as above; on aarch64 the number is in `x8`, the result in
    // `x0`, and `svc #0` changes no other register (T-1721).
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") n,
            inlateout("x0") a[0] => r,
            in("x1") a[1],
            in("x2") a[2],
            in("x3") a[3],
            in("x4") a[4],
            in("x5") a[5],
            options(nostack),
        );
    }
    if (-4095..0).contains(&r) {
        set_errno(-r as c_int);
        return -1;
    }
    r
}

/// Answer `ENOSYS` for number `n`, and say so. `always` is false where the
/// caller has a fallback that this object maps, so the line is a note.
fn refuse(n: c_long, name: &[u8], always: bool) -> c_long {
    if always || say::debug() {
        say::line(&[
            b"syscall ",
            say::Num::new(n as u64).as_bytes(),
            b" (",
            name,
            b"): a raw call that the path table cannot map, so podvm answers \
              ENOSYS (TODO/interpose.md T-1697)",
        ]);
    }
    set_errno(ENOSYS);
    -1
}

/// A routed number whose libc has no symbol for the route: `ENOSYS` where
/// a table is set, and the kernel's own answer where none is.
///
/// # Safety
/// As [`forward`].
unsafe fn absent(n: c_long, name: &[u8], args: [c_long; 6]) -> c_long {
    // SAFETY: as the caller's.
    unsafe {
        if crate::map::table().n > 0 {
            return refuse(n, name, false);
        }
        forward(n, args)
    }
}

/// `syscall(3)`.
///
/// # Safety
/// The payload's own contract for `syscall(3)` and number `n`.
#[allow(invalid_runtime_symbol_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syscall(
    n: c_long,
    a: c_long,
    b: c_long,
    c: c_long,
    d: c_long,
    e: c_long,
    f: c_long,
) -> c_long {
    let args = [a, b, c, d, e, f];
    // SAFETY: each arm passes the payload's arguments in the kernel's
    // order to an entry point whose contract is that number's contract.
    unsafe {
        if let Some(r) = route(n, args) {
            return r;
        }
        let named = UNROUTED.iter().find(|(m, _)| *m == n);
        if (named.is_some() || n == SYS_IO_URING_SETUP) && crate::map::table().n > 0 {
            return match named {
                Some((_, name)) => refuse(n, name, true),
                None => refuse(n, b"io_uring_setup", false),
            };
        }
        forward(n, args)
    }
}

/// Arm 1: the entry point that maps number `n`, or `None`.
///
/// # Safety
/// As [`syscall`].
unsafe fn route(n: c_long, args: [c_long; 6]) -> Option<c_long> {
    let [a, b, c, d, e, _] = args;
    let s = |v: c_long| v as *const c_char;
    let k = |v: c_long| v as *const c_void;
    let m = |v: c_long| v as *mut c_void;
    let w = |v: c_long| v as *mut c_char;
    let i = |v: c_long| v as c_int;
    let u = |v: c_long| v as c_uint;
    let z = |v: c_long| v as usize;
    let v = |v: c_long| v as *const *const c_char;
    // SAFETY: as the caller's.
    let r = unsafe {
        match n {
            SYS_STATX if next_statx().is_none() => absent(n, b"statx", args),
            SYS_STATX => statx(i(a), s(b), i(c), u(d), m(e)) as c_long,
            SYS_STAT => stat(s(a), m(b)) as c_long,
            SYS_LSTAT => lstat(s(a), m(b)) as c_long,
            SYS_NEWFSTATAT => fstatat(i(a), s(b), m(c), i(d)) as c_long,
            SYS_OPEN => open(s(a), i(b), u(c)) as c_long,
            SYS_OPENAT => openat(i(a), s(b), i(c), u(d)) as c_long,
            SYS_OPENAT2 if next_openat2().is_none() => absent(n, b"openat2", args),
            SYS_OPENAT2 => openat2(i(a), s(b), k(c), z(d)) as c_long,
            SYS_CREAT => creat(s(a), u(b)) as c_long,
            SYS_ACCESS => access(s(a), i(b)) as c_long,
            SYS_FACCESSAT => faccessat(i(a), s(b), i(c), 0) as c_long,
            SYS_FACCESSAT2 => faccessat(i(a), s(b), i(c), i(d)) as c_long,
            SYS_READLINK => readlink(s(a), w(b), z(c)) as c_long,
            SYS_READLINKAT => readlinkat(i(a), s(b), w(c), z(d)) as c_long,
            SYS_GETCWD if a != 0 => cwd(w(a), z(b)),
            SYS_CHDIR => chdir(s(a)) as c_long,
            SYS_MKDIR => mkdir(s(a), u(b)) as c_long,
            SYS_MKDIRAT => mkdirat(i(a), s(b), u(c)) as c_long,
            SYS_RMDIR => rmdir(s(a)) as c_long,
            SYS_UNLINK => unlink(s(a)) as c_long,
            SYS_UNLINKAT => unlinkat(i(a), s(b), i(c)) as c_long,
            SYS_RENAME => rename(s(a), s(b)) as c_long,
            SYS_RENAMEAT => renameat(i(a), s(b), i(c), s(d)) as c_long,
            SYS_RENAMEAT2 => renameat2(i(a), s(b), i(c), s(d), u(e)) as c_long,
            SYS_LINK => link(s(a), s(b)) as c_long,
            SYS_LINKAT => linkat(i(a), s(b), i(c), s(d), i(e)) as c_long,
            SYS_SYMLINK => symlink(s(a), s(b)) as c_long,
            SYS_SYMLINKAT => symlinkat(s(a), i(b), s(c)) as c_long,
            SYS_CHMOD => chmod(s(a), u(b)) as c_long,
            SYS_FCHMODAT => fchmodat(i(a), s(b), u(c), 0) as c_long,
            SYS_CHOWN => chown(s(a), u(b), u(c)) as c_long,
            SYS_LCHOWN => lchown(s(a), u(b), u(c)) as c_long,
            SYS_FCHOWNAT => fchownat(i(a), s(b), u(c), u(d), i(e)) as c_long,
            SYS_TRUNCATE => truncate(s(a), b) as c_long,
            SYS_UTIME => utime(s(a), k(b)) as c_long,
            SYS_UTIMES => utimes(s(a), k(b)) as c_long,
            SYS_UTIMENSAT => utimensat(i(a), s(b), k(c), i(d)) as c_long,
            SYS_MKNOD => mknod(s(a), u(b), u(c) as u64) as c_long,
            SYS_STATFS => statfs(s(a), m(b)) as c_long,
            SYS_MOUNT => mount(s(a), s(b), s(c), d as c_ulong, k(e)) as c_long,
            SYS_SETXATTR => setxattr(s(a), s(b), k(c), z(d), i(e)) as c_long,
            SYS_LSETXATTR => lsetxattr(s(a), s(b), k(c), z(d), i(e)) as c_long,
            SYS_GETXATTR => getxattr(s(a), s(b), m(c), z(d)) as c_long,
            SYS_LGETXATTR => lgetxattr(s(a), s(b), m(c), z(d)) as c_long,
            SYS_LISTXATTR => listxattr(s(a), w(b), z(c)) as c_long,
            SYS_LLISTXATTR => llistxattr(s(a), w(b), z(c)) as c_long,
            SYS_REMOVEXATTR => removexattr(s(a), s(b)) as c_long,
            SYS_LREMOVEXATTR => lremovexattr(s(a), s(b)) as c_long,
            SYS_EXECVE => execve(s(a), v(b), v(c)) as c_long,
            SYS_EXECVEAT => execveat(i(a), s(b), v(c), v(d), i(e)) as c_long,
            _ => return None,
        }
    };
    Some(r)
}

/// The kernel's `getcwd`: the length with the NUL, where libc's answers the
/// buffer.
///
/// # Safety
/// `buf` must be writable for `size` bytes.
unsafe fn cwd(buf: *mut c_char, size: usize) -> c_long {
    // SAFETY: as the caller's.
    let r = unsafe { getcwd(buf, size) };
    if r.is_null() {
        return -1;
    }
    (crate::map::strnlen(buf as *const c_char, size) + 1) as c_long
}

#[cfg(test)]
mod tests;
