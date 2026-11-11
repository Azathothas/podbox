//! Raw system calls with no libc and no thread-local storage.
//!
//! ⛔ The handler runs in a thread whose FS register belongs to the
//! payload, so no function here touches `errno`, a lock or the allocator.
//! A result is the kernel's: a value of `-4095..=-1` is `-errno`.

use core::arch::asm;

pub const AT_FDCWD: i64 = -100;
pub const AT_SYMLINK_NOFOLLOW: i64 = 0x100;
pub const AT_REMOVEDIR: i64 = 0x200;
pub const AT_EMPTY_PATH: i64 = 0x1000;
pub const O_RDONLY: i64 = 0;
pub const O_DIRECTORY: i64 = 0o200000;
pub const O_NOFOLLOW: i64 = 0o400000;
pub const O_CLOEXEC: i64 = 0o2000000;
pub const O_PATH: i64 = 0o10000000;
pub const O_CREAT: i64 = 0o100;
pub const O_TMPFILE: i64 = 0o20200000;
pub const RESOLVE_IN_ROOT: u64 = 0x10;

pub const ENOENT: i64 = 2;
pub const EBADF: i64 = 9;
pub const EFAULT: i64 = 14;
pub const EACCES: i64 = 13;
pub const EPERM: i64 = 1;
pub const EINVAL: i64 = 22;
pub const ENOEXEC: i64 = 8;
pub const ENOMEM: i64 = 12;
pub const ENOSYS: i64 = 38;
pub const ENAMETOOLONG: i64 = 36;
pub const ERANGE: i64 = 34;
pub const EBUSY: i64 = 16;

/// The numbers of x86_64 that the handler names.
pub mod nr {
    pub const OPEN: i64 = 2;
    pub const CLOSE: i64 = 3;
    pub const STAT: i64 = 4;
    pub const LSTAT: i64 = 6;
    pub const MMAP: i64 = 9;
    pub const MPROTECT: i64 = 10;
    pub const MUNMAP: i64 = 11;
    pub const BRK: i64 = 12;
    pub const RT_SIGACTION: i64 = 13;
    pub const RT_SIGPROCMASK: i64 = 14;
    pub const RT_SIGRETURN: i64 = 15;
    pub const PREAD64: i64 = 17;
    pub const ACCESS: i64 = 21;
    pub const DUP2: i64 = 33;
    pub const GETPID: i64 = 39;
    pub const CLONE: i64 = 56;
    pub const FORK: i64 = 57;
    pub const VFORK: i64 = 58;
    pub const EXECVE: i64 = 59;
    pub const FCNTL: i64 = 72;
    pub const TRUNCATE: i64 = 76;
    pub const GETCWD: i64 = 79;
    pub const CHDIR: i64 = 80;
    pub const RENAME: i64 = 82;
    pub const MKDIR: i64 = 83;
    pub const RMDIR: i64 = 84;
    pub const CREAT: i64 = 85;
    pub const LINK: i64 = 86;
    pub const UNLINK: i64 = 87;
    pub const SYMLINK: i64 = 88;
    pub const READLINK: i64 = 89;
    pub const CHMOD: i64 = 90;
    pub const CHOWN: i64 = 92;
    pub const LCHOWN: i64 = 94;
    pub const UTIME: i64 = 132;
    pub const MKNOD: i64 = 133;
    pub const SIGALTSTACK: i64 = 131;
    pub const STATFS: i64 = 137;
    pub const PRCTL: i64 = 157;
    pub const CHROOT: i64 = 161;
    pub const MOUNT: i64 = 165;
    pub const UMOUNT2: i64 = 166;
    pub const PIVOT_ROOT: i64 = 155;
    pub const SETXATTR: i64 = 188;
    pub const LSETXATTR: i64 = 189;
    pub const GETXATTR: i64 = 191;
    pub const LGETXATTR: i64 = 192;
    pub const LISTXATTR: i64 = 194;
    pub const LLISTXATTR: i64 = 195;
    pub const REMOVEXATTR: i64 = 197;
    pub const LREMOVEXATTR: i64 = 198;
    pub const UTIMES: i64 = 235;
    pub const INOTIFY_ADD_WATCH: i64 = 254;
    pub const OPENAT: i64 = 257;
    pub const MKDIRAT: i64 = 258;
    pub const MKNODAT: i64 = 259;
    pub const FCHOWNAT: i64 = 260;
    pub const FUTIMESAT: i64 = 261;
    pub const NEWFSTATAT: i64 = 262;
    pub const UNLINKAT: i64 = 263;
    pub const RENAMEAT: i64 = 264;
    pub const LINKAT: i64 = 265;
    pub const SYMLINKAT: i64 = 266;
    pub const READLINKAT: i64 = 267;
    pub const FCHMODAT: i64 = 268;
    pub const FACCESSAT: i64 = 269;
    pub const UTIMENSAT: i64 = 280;
    pub const DUP3: i64 = 292;
    pub const RENAMEAT2: i64 = 316;
    pub const EXECVEAT: i64 = 322;
    pub const STATX: i64 = 332;
    pub const CLOSE_RANGE: i64 = 436;
    pub const CLONE3: i64 = 435;
    pub const OPENAT2: i64 = 437;
    pub const FACCESSAT2: i64 = 439;
    pub const FCHMODAT2: i64 = 452;
}

/// One system call with six arguments.
///
/// # Safety
/// The caller gives arguments that the kernel may read and write.
#[inline(always)]
pub unsafe fn sc6(n: i64, a: i64, b: i64, c: i64, d: i64, e: i64, f: i64) -> i64 {
    let r: i64;
    // SAFETY: the syscall instruction clobbers rcx and r11 only.
    unsafe {
        asm!("syscall", inlateout("rax") n => r, in("rdi") a, in("rsi") b, in("rdx") c,
             in("r10") d, in("r8") e, in("r9") f, lateout("rcx") _, lateout("r11") _,
             options(nostack));
    }
    r
}

/// # Safety
/// As [`sc6`].
#[inline(always)]
pub unsafe fn sc(n: i64, a: i64, b: i64, c: i64) -> i64 {
    unsafe { sc6(n, a, b, c, 0, 0, 0) }
}

/// `struct open_how` of openat2(2).
#[repr(C)]
pub struct OpenHow {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

/// `openat2(dirfd, path, how)`; `path` ends with a NUL.
pub fn openat2(dirfd: i64, path: &[u8], flags: i64, mode: i64, resolve: u64) -> i64 {
    let how = OpenHow {
        flags: flags as u64,
        mode: mode as u64,
        resolve,
    };
    // SAFETY: the path and the struct live for the call.
    unsafe {
        sc6(
            nr::OPENAT2,
            dirfd,
            path.as_ptr() as i64,
            &how as *const OpenHow as i64,
            core::mem::size_of::<OpenHow>() as i64,
            0,
            0,
        )
    }
}

pub fn close(fd: i64) {
    // SAFETY: closing a descriptor reads no memory.
    unsafe { sc(nr::CLOSE, fd, 0, 0) };
}

/// `readlinkat` into `buf`; the length, or `-errno`.
pub fn readlinkat(dirfd: i64, path: &[u8], buf: &mut [u8]) -> i64 {
    // SAFETY: both buffers live for the call; the kernel writes at most len.
    unsafe {
        sc6(
            nr::READLINKAT,
            dirfd,
            path.as_ptr() as i64,
            buf.as_mut_ptr() as i64,
            buf.len() as i64,
            0,
            0,
        )
    }
}

/// The NUL-ended string at `p` of the payload, at most `max` bytes, or None
/// where `p` is NULL or the string is longer.
///
/// # Safety
/// `p` is NULL or points into memory that the payload gave for a path; a
/// bad pointer faults as it would in the payload.
pub unsafe fn cstr<'a>(p: i64, max: usize) -> Option<&'a [u8]> {
    if p == 0 {
        return None;
    }
    let base = p as *const u8;
    let mut n = 0usize;
    while n < max {
        // SAFETY: the caller's contract; the read stops at the first NUL.
        if unsafe { *base.add(n) } == 0 {
            // SAFETY: n bytes were read above.
            return Some(unsafe { core::slice::from_raw_parts(base, n) });
        }
        n += 1;
    }
    None
}
