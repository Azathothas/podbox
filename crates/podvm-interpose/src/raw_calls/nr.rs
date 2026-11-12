//! The system call numbers of each architecture (TODO/interpose.md T-1721).
//!
//! x86_64 has the numbers of the kernel's `syscall_64.tbl`. aarch64 has the
//! generic table (`include/uapi/asm-generic/unistd.h`), which has no call of
//! a path with no directory descriptor (`open`, `stat`, `access` and the
//! rest): each of those gets a negative number here, which no call has, so
//! its arm of the match never applies and libc's own `*at` call goes
//! through the arm of that call.

use core::ffi::c_long;

#[cfg(target_arch = "x86_64")]
mod arch {
    use super::c_long;
    pub const SYS_OPEN: c_long = 2;
    pub const SYS_STAT: c_long = 4;
    pub const SYS_LSTAT: c_long = 6;
    pub const SYS_ACCESS: c_long = 21;
    pub const SYS_EXECVE: c_long = 59;
    pub const SYS_TRUNCATE: c_long = 76;
    pub const SYS_GETCWD: c_long = 79;
    pub const SYS_CHDIR: c_long = 80;
    pub const SYS_RENAME: c_long = 82;
    pub const SYS_MKDIR: c_long = 83;
    pub const SYS_RMDIR: c_long = 84;
    pub const SYS_CREAT: c_long = 85;
    pub const SYS_LINK: c_long = 86;
    pub const SYS_UNLINK: c_long = 87;
    pub const SYS_SYMLINK: c_long = 88;
    pub const SYS_READLINK: c_long = 89;
    pub const SYS_CHMOD: c_long = 90;
    pub const SYS_CHOWN: c_long = 92;
    pub const SYS_LCHOWN: c_long = 94;
    pub const SYS_UTIME: c_long = 132;
    pub const SYS_MKNOD: c_long = 133;
    pub const SYS_STATFS: c_long = 137;
    pub const SYS_MOUNT: c_long = 165;
    pub const SYS_SETXATTR: c_long = 188;
    pub const SYS_LSETXATTR: c_long = 189;
    pub const SYS_GETXATTR: c_long = 191;
    pub const SYS_LGETXATTR: c_long = 192;
    pub const SYS_LISTXATTR: c_long = 194;
    pub const SYS_LLISTXATTR: c_long = 195;
    pub const SYS_REMOVEXATTR: c_long = 197;
    pub const SYS_LREMOVEXATTR: c_long = 198;
    pub const SYS_UTIMES: c_long = 235;
    pub const SYS_OPENAT: c_long = 257;
    pub const SYS_MKDIRAT: c_long = 258;
    pub const SYS_FCHOWNAT: c_long = 260;
    pub const SYS_NEWFSTATAT: c_long = 262;
    pub const SYS_UNLINKAT: c_long = 263;
    pub const SYS_RENAMEAT: c_long = 264;
    pub const SYS_LINKAT: c_long = 265;
    pub const SYS_SYMLINKAT: c_long = 266;
    pub const SYS_READLINKAT: c_long = 267;
    pub const SYS_FCHMODAT: c_long = 268;
    pub const SYS_FACCESSAT: c_long = 269;
    pub const SYS_UTIMENSAT: c_long = 280;
    pub const SYS_RENAMEAT2: c_long = 316;
    pub const SYS_EXECVEAT: c_long = 322;
    pub const SYS_STATX: c_long = 332;
    pub const SYS_CLOSE: c_long = 3;
    /// The numbers that take a path and that no entry point maps.
    pub const UNROUTED: &[(c_long, &[u8])] = &[
        (134, b"uselib"),
        (155, b"pivot_root"),
        (161, b"chroot"),
        (163, b"acct"),
        (166, b"umount2"),
        (167, b"swapon"),
        (168, b"swapoff"),
        (179, b"quotactl"),
        (254, b"inotify_add_watch"),
        (259, b"mknodat"),
        (301, b"fanotify_mark"),
        (303, b"name_to_handle_at"),
        (428, b"open_tree"),
        (429, b"move_mount"),
        (431, b"fsconfig"),
        (433, b"fspick"),
        (442, b"mount_setattr"),
        (452, b"fchmodat2"),
        (463, b"setxattrat"),
        (464, b"getxattrat"),
        (465, b"listxattrat"),
        (466, b"removexattrat"),
        (467, b"open_tree_attr"),
        (468, b"file_getattr"),
        (469, b"file_setattr"),
    ];
}

#[cfg(target_arch = "aarch64")]
mod arch {
    use super::c_long;
    // The calls that the generic table has not: a number that no call has.
    pub const SYS_OPEN: c_long = -1002;
    pub const SYS_STAT: c_long = -1004;
    pub const SYS_LSTAT: c_long = -1006;
    pub const SYS_ACCESS: c_long = -1021;
    pub const SYS_RENAME: c_long = -1082;
    pub const SYS_MKDIR: c_long = -1083;
    pub const SYS_RMDIR: c_long = -1084;
    pub const SYS_CREAT: c_long = -1085;
    pub const SYS_LINK: c_long = -1086;
    pub const SYS_UNLINK: c_long = -1087;
    pub const SYS_SYMLINK: c_long = -1088;
    pub const SYS_READLINK: c_long = -1089;
    pub const SYS_CHMOD: c_long = -1090;
    pub const SYS_CHOWN: c_long = -1092;
    pub const SYS_LCHOWN: c_long = -1094;
    pub const SYS_UTIME: c_long = -1132;
    pub const SYS_MKNOD: c_long = -1133;
    pub const SYS_UTIMES: c_long = -1235;
    // The generic table.
    pub const SYS_SETXATTR: c_long = 5;
    pub const SYS_LSETXATTR: c_long = 6;
    pub const SYS_GETXATTR: c_long = 8;
    pub const SYS_LGETXATTR: c_long = 9;
    pub const SYS_LISTXATTR: c_long = 11;
    pub const SYS_LLISTXATTR: c_long = 12;
    pub const SYS_REMOVEXATTR: c_long = 14;
    pub const SYS_LREMOVEXATTR: c_long = 15;
    pub const SYS_GETCWD: c_long = 17;
    pub const SYS_MKDIRAT: c_long = 34;
    pub const SYS_UNLINKAT: c_long = 35;
    pub const SYS_SYMLINKAT: c_long = 36;
    pub const SYS_LINKAT: c_long = 37;
    pub const SYS_RENAMEAT: c_long = 38;
    pub const SYS_MOUNT: c_long = 40;
    pub const SYS_STATFS: c_long = 43;
    pub const SYS_TRUNCATE: c_long = 45;
    pub const SYS_FACCESSAT: c_long = 48;
    pub const SYS_CHDIR: c_long = 49;
    pub const SYS_FCHMODAT: c_long = 53;
    pub const SYS_FCHOWNAT: c_long = 54;
    pub const SYS_OPENAT: c_long = 56;
    pub const SYS_CLOSE: c_long = 57;
    pub const SYS_READLINKAT: c_long = 78;
    pub const SYS_NEWFSTATAT: c_long = 79;
    pub const SYS_UTIMENSAT: c_long = 88;
    pub const SYS_EXECVE: c_long = 221;
    pub const SYS_RENAMEAT2: c_long = 276;
    pub const SYS_EXECVEAT: c_long = 281;
    pub const SYS_STATX: c_long = 291;
    /// The numbers that take a path and that no entry point maps.
    pub const UNROUTED: &[(c_long, &[u8])] = &[
        (27, b"inotify_add_watch"),
        (33, b"mknodat"),
        (39, b"umount2"),
        (41, b"pivot_root"),
        (51, b"chroot"),
        (60, b"quotactl"),
        (89, b"acct"),
        (224, b"swapon"),
        (225, b"swapoff"),
        (263, b"fanotify_mark"),
        (264, b"name_to_handle_at"),
        (428, b"open_tree"),
        (429, b"move_mount"),
        (431, b"fsconfig"),
        (433, b"fspick"),
        (442, b"mount_setattr"),
        (452, b"fchmodat2"),
        (463, b"setxattrat"),
        (464, b"getxattrat"),
        (465, b"listxattrat"),
        (466, b"removexattrat"),
        (467, b"open_tree_attr"),
        (468, b"file_getattr"),
        (469, b"file_setattr"),
    ];
}

pub(crate) use arch::*;

// The numbers since Linux 5.1 are one table on each architecture.
pub(crate) const SYS_IO_URING_SETUP: c_long = 425;
pub(crate) const SYS_OPENAT2: c_long = 437;
pub(crate) const SYS_FACCESSAT2: c_long = 439;
