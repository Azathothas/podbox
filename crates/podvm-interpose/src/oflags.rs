//! The open flags of the target, T-1744.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-1744. Four flags of
//! `open` have other values on aarch64 than on x86_64 (the kernel's
//! `asm-generic/fcntl.h` and `arm64/include/uapi/asm/fcntl.h`). Each file of
//! the interposer takes its flags from here; the test
//! `the_open_flags_agree_with_the_c_headers` compares each value with the
//! value that `cc` gives on the machine of the test.

use core::ffi::c_int;

#[cfg(target_arch = "aarch64")]
mod arch {
    use core::ffi::c_int;
    pub const O_DIRECTORY: c_int = 0o40000;
    pub const O_NOFOLLOW: c_int = 0o100000;
    pub const O_DIRECT: c_int = 0o200000;
    pub const O_LARGEFILE: c_int = 0o400000;
}

#[cfg(not(target_arch = "aarch64"))]
mod arch {
    use core::ffi::c_int;
    pub const O_DIRECT: c_int = 0o40000;
    pub const O_LARGEFILE: c_int = 0o100000;
    pub const O_DIRECTORY: c_int = 0o200000;
    pub const O_NOFOLLOW: c_int = 0o400000;
}

pub use arch::{O_DIRECT, O_DIRECTORY, O_LARGEFILE, O_NOFOLLOW};

pub const O_WRONLY: c_int = 0o1;
pub const O_RDWR: c_int = 0o2;
pub const O_CREAT: c_int = 0o100;
pub const O_EXCL: c_int = 0o200;
pub const O_NOCTTY: c_int = 0o400;
pub const O_TRUNC: c_int = 0o1000;
pub const O_APPEND: c_int = 0o2000;
pub const O_NONBLOCK: c_int = 0o4000;
pub const O_CLOEXEC: c_int = 0o2000000;
pub const O_PATH: c_int = 0o10000000;
/// `O_TMPFILE` holds `O_DIRECTORY`, so it differs by architecture too.
pub const O_TMPFILE: c_int = 0o20000000 | O_DIRECTORY;
