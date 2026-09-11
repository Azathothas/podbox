//! Provide the low-level operating system operations for the crate.
//!
//! The child uses stack buffers and direct system calls between `fork` and
//! `exec`. It does not use the allocator in this interval.

use std::io::{Error, ErrorKind, Result};
use std::sync::atomic::{AtomicU8, Ordering};

use crate::cvt::{cvt, cvt_r_ssize, cvt_ssize};

// memfd_create flags (linux/memfd.h). Defined here so targets whose libc
// crate lacks the constants still compile; the values are ABI-stable.
pub const MFD_CLOEXEC: libc::c_uint = 0x0001;
pub const MFD_ALLOW_SEALING: libc::c_uint = 0x0002;
pub const MFD_HUGETLB: libc::c_uint = 0x0004;
pub const MFD_EXEC: libc::c_uint = 0x0010;

// Linux and FreeBSD use the same F_SEAL_* bit values. F_SEAL_SEAL is bit 0
// (0x1), so SHRINK, GROW and WRITE are 0x2, 0x4 and 0x8.
#[cfg(target_os = "linux")]
pub const F_ADD_SEALS: libc::c_int = 1033;
#[cfg(target_os = "linux")]
pub const F_GET_SEALS: libc::c_int = 1034;
#[cfg(target_os = "freebsd")]
pub const F_ADD_SEALS: libc::c_int = 19;
#[cfg(target_os = "freebsd")]
pub const F_GET_SEALS: libc::c_int = 20;
pub const F_SEAL_SHRINK: libc::c_int = 0x0002;
pub const F_SEAL_GROW: libc::c_int = 0x0004;
pub const F_SEAL_WRITE: libc::c_int = 0x0008;
pub const SEALS_FULL: libc::c_int = F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE;

#[cfg(target_os = "linux")]
pub const AT_EMPTY_PATH: libc::c_int = 0x1000;
#[cfg(target_os = "linux")]
pub const AT_SYMLINK_FOLLOW: libc::c_int = 0x0400;

// pidfd machinery. Syscall numbers are ABI-stable across every Linux arch;
// CLONE_* flag values likewise (uapi/linux/sched.h).
#[cfg(target_os = "linux")]
pub const SYS_CLONE3: libc::c_long = 435;
#[cfg(target_os = "linux")]
pub const SYS_PIDFD_SEND_SIGNAL: libc::c_long = 424;
#[cfg(target_os = "linux")]
pub const CLONE_VFORK_FLAG: libc::c_long = 0x0000_4000;
#[cfg(target_os = "linux")]
pub const CLONE_PIDFD_FLAG: libc::c_long = 0x0000_1000;

// File-system identifier for hugetlbfs from uapi/linux/magic.h.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub const HUGETLBFS_MAGIC: i64 = 0x9584_58f6;

// The parent's (inherited) environment, passed to execve when the command
// carries no explicit environment changes — exactly what std does. libc does
// not declare it for Linux, so declare the POSIX global here.
extern "C" {
    static mut environ: *mut *mut libc::c_char;
}

/// Read the global `environ` pointer for execve. Called in the forked child:
/// the pointer (and the strings it references) are the fork-time copy.
pub fn inherited_environ() -> *const *const libc::c_char {
    unsafe { environ as *const *const libc::c_char }
}

// ---------------------------------------------------------------------------
// Capability probes
// ---------------------------------------------------------------------------

// 0 = unknown, 1 = supported, 2 = unsupported. Races are harmless: the probe
// syscall is idempotent and its only side effect is one closed fd.
static EXEC_BIT: AtomicU8 = AtomicU8::new(0);
static SEAL_BIT: AtomicU8 = AtomicU8::new(0);
static HUGETLB_BIT: AtomicU8 = AtomicU8::new(0);
#[cfg(target_os = "linux")]
static CLONE3_BIT: AtomicU8 = AtomicU8::new(0);
#[cfg(target_os = "linux")]
static PIDFD_WAIT_BIT: AtomicU8 = AtomicU8::new(0);
static PROC_OK: AtomicU8 = AtomicU8::new(0);

/// Return the result used for an unavailable operating system feature.
fn unsupported() -> Error {
    Error::from_raw_os_error(libc::ENOSYS)
}

/// Return true when `err` reports an unavailable feature.
pub fn is_unsupported(err: &Error) -> bool {
    err.raw_os_error() == Some(libc::ENOSYS)
}

fn probe_flag(flag: libc::c_uint, atom: &AtomicU8) -> bool {
    match atom.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let name = b"memfd-ng-probe\0";
    let supported = unsafe {
        let fd = libc::memfd_create(name.as_ptr() as *const libc::c_char, MFD_CLOEXEC | flag);
        if fd >= 0 {
            libc::close(fd);
            true
        } else {
            false
        }
    };
    atom.store(if supported { 1 } else { 2 }, Ordering::Relaxed);
    supported
}

/// True when `/proc` is mounted and `access(F_OK)` passes, cached per process.
pub fn proc_available() -> bool {
    #[cfg(feature = "test-hooks")]
    if hook_disabled(b"MEMFD_NG_TEST_NO_PROC\0") {
        return false;
    }
    match PROC_OK.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let ok = unsafe {
                libc::access(b"/proc/self\0".as_ptr() as *const libc::c_char, libc::F_OK) == 0
            };
            PROC_OK.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

// ---------------------------------------------------------------------------
// memfd
// ---------------------------------------------------------------------------

/// Create an anonymous executable file and return the raw fd.
///
/// `MFD_EXEC` (kernel 6.3+) is probed once and set when supported, so
/// `vm.memfd_noexec` enforcement modes keep working; on older kernels the
/// creation falls back to `MFD_CLOEXEC` and the kernel default. When
/// `allow_sealing` is true and the kernel supports it, `MFD_ALLOW_SEALING`
/// is added so the image can be sealed once fully written.
pub fn memfd_create(name: *const libc::c_char, allow_sealing: bool) -> Result<libc::c_int> {
    let mut flags = MFD_CLOEXEC;
    if probe_flag(MFD_EXEC, &EXEC_BIT) {
        flags |= MFD_EXEC;
    }
    if allow_sealing && probe_flag(MFD_ALLOW_SEALING, &SEAL_BIT) {
        flags |= MFD_ALLOW_SEALING;
    }
    cvt(unsafe { libc::memfd_create(name, flags) })
}

/// Create an anonymous executable file on hugetlbfs (`MFD_HUGETLB`, kernel
/// 4.14+). Probed once; `Err` with raw `ENOSYS` means the kernel refused the
/// facility outright and the caller should stage an ordinary memfd.
pub fn memfd_create_hugetlb(name: *const libc::c_char, allow_sealing: bool) -> Result<libc::c_int> {
    if HUGETLB_BIT.load(Ordering::Relaxed) == 2 {
        return Err(unsupported());
    }
    let mut flags = MFD_CLOEXEC | MFD_HUGETLB;
    if probe_flag(MFD_EXEC, &EXEC_BIT) {
        flags |= MFD_EXEC;
    }
    if allow_sealing {
        flags |= MFD_ALLOW_SEALING;
    }
    let fd = unsafe { libc::memfd_create(name, flags) };
    if fd >= 0 {
        HUGETLB_BIT.store(1, Ordering::Relaxed);
        return Ok(fd);
    }
    let err = Error::last_os_error();
    // Linux 4.14 and 4.15 reject MFD_HUGETLB | MFD_ALLOW_SEALING with EINVAL.
    // Do not cache that result because a later unsealed request can succeed.
    // ENOMEM is also not cached because huge pages can be added at runtime.
    let unavailable = matches!(
        err.raw_os_error(),
        Some(libc::ENOSYS) | Some(libc::EOPNOTSUPP)
    ) || (err.raw_os_error() == Some(libc::EINVAL) && !allow_sealing);
    if unavailable {
        HUGETLB_BIT.store(2, Ordering::Relaxed);
        return Err(unsupported());
    }
    Err(err)
}

/// The hugetlb page size of a hugetlbfs memfd (`fstatfs(2).f_bsize`), or
/// `None` when the fd is not on hugetlbfs — the caller's signal to degrade
/// to an ordinary memfd rather than guess alignment. Only 64-bit Linux has an
/// ABI-stable `KernelStatfs` here; elsewhere hugetlb always degrades.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub fn hugetlb_page_size(fd: libc::c_int) -> Option<i64> {
    let mut fs: KernelStatfs = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::syscall(libc::SYS_fstatfs, fd, &mut fs as *mut KernelStatfs) == 0 };
    if ok && fs.f_type == HUGETLBFS_MAGIC && fs.f_bsize > 0 {
        Some(fs.f_bsize)
    } else {
        None
    }
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
pub fn hugetlb_page_size(_fd: libc::c_int) -> Option<i64> {
    None
}

/// Seal a memfd against the given `F_SEAL_*` bits. Returns false when the
/// kernel has no sealing support (the image still runs, it just stays
/// modifiable through writable fds).
pub fn add_seals(fd: libc::c_int, seals: libc::c_int) -> bool {
    unsafe { libc::fcntl(fd, F_ADD_SEALS, seals) == 0 }
}

/// Read the active `F_SEAL_*` bits of a memfd (`F_GET_SEALS`), or None when
/// the fd does not support sealing at all.
pub fn get_seals(fd: libc::c_int) -> Option<libc::c_int> {
    let bits = unsafe { libc::fcntl(fd, F_GET_SEALS) };
    if bits < 0 {
        None
    } else {
        Some(bits)
    }
}

/// True when `fd` is a regular file — the only kind the kernel will exec.
pub fn is_regular_file(fd: libc::c_int) -> bool {
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        libc::fstat(fd, &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFREG
    }
}

/// Write the whole buffer, looping over partial writes.
pub fn write_all(fd: libc::c_int, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
        let n = cvt_r_ssize(|| unsafe {
            libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len())
        })?;
        if n == 0 {
            return Err(Error::from(ErrorKind::WriteZero));
        }
        buf = &buf[n..];
    }
    Ok(())
}

/// Read until the buffer is full or the stream ends.
fn read_fill(fd: libc::c_int, buf: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = cvt_ssize(unsafe {
            libc::read(
                fd,
                buf[filled..].as_mut_ptr() as *mut libc::c_void,
                buf.len() - filled,
            )
        })?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

// ---------------------------------------------------------------------------
// Linux pidfd process creation and child handling.
// ---------------------------------------------------------------------------

/// `struct clone_args` for the clone3 syscall (uapi/linux/sched.h). Every
/// field is `__u64`, including `pidfd`, which holds the address of an int as a
/// u64 — so the layout is correct on 32-bit as well as 64-bit. The base
/// 64-byte layout is accepted by every kernel that implements clone3; later
/// additions (set_tid, cgroup) are appended by the kernel, not us.
#[cfg(target_os = "linux")]
#[repr(C, align(8))]
struct CloneArgs {
    flags: u64,
    pidfd: u64,
    child_tid: u64,
    parent_tid: u64,
    exit_signal: u64,
    stack: u64,
    stack_size: u64,
    tls: u64,
}

#[cfg(target_os = "linux")]
const _: [(); 64] = [(); std::mem::size_of::<CloneArgs>()];
#[cfg(target_os = "linux")]
const _: [(); 8] = [(); std::mem::align_of::<CloneArgs>()];

/// What `clone3_vfork_pidfd` decided for the caller.
#[cfg(target_os = "linux")]
pub enum ForkOutcome {
    /// The calling thread continues as the child (pid would be 0).
    Child,
    /// The calling thread is the parent; the child has `pid` and the parent
    /// holds a `pidfd` that stays valid across PID reuse.
    Parent {
        pid: libc::pid_t,
        pidfd: libc::c_int,
    },
}

/// Fork via `clone3(CLONE_VFORK | CLONE_PIDFD, exit_signal = SIGCHLD)`
/// (kernel 5.3+).
///
/// - `CLONE_VFORK` suspends the parent until the child execs or exits, so
///   the freshly forked child runs immediately instead of racing the
///   scheduler — the same latency win posix_spawn buys. Without `CLONE_VM`
///   the child still gets a private copy-on-write address space, so it can
///   never corrupt the suspended parent.
/// - `CLONE_PIDFD` returns a pidfd, which makes `kill`/`wait` immune to PID
///   reuse and lets callers poll(2) the child.
///
/// `Err` with raw `ENOSYS` (cached) means the kernel lacks clone3 or refuses
/// the flag pair; the caller must fall back to plain `fork()`. Any other
/// error is returned to the caller.
#[cfg(target_os = "linux")]
pub fn clone3_vfork_pidfd() -> Result<ForkOutcome> {
    if CLONE3_BIT.load(Ordering::Relaxed) == 2 {
        return Err(unsupported());
    }
    let mut pidfd: libc::c_int = -1;
    let mut args = CloneArgs {
        flags: (CLONE_VFORK_FLAG | CLONE_PIDFD_FLAG) as u64,
        pidfd: &mut pidfd as *mut libc::c_int as u64,
        child_tid: 0,
        parent_tid: 0,
        exit_signal: libc::SIGCHLD as u64,
        stack: 0,
        stack_size: 0,
        tls: 0,
    };
    let ret = unsafe {
        libc::syscall(
            SYS_CLONE3,
            &mut args as *mut CloneArgs,
            std::mem::size_of::<CloneArgs>(),
        )
    };
    if ret >= 0 {
        CLONE3_BIT.store(1, Ordering::Relaxed);
        return Ok(if ret == 0 {
            ForkOutcome::Child
        } else {
            ForkOutcome::Parent {
                pid: ret as libc::pid_t,
                pidfd,
            }
        });
    }
    let err = Error::last_os_error();
    if matches!(err.raw_os_error(), Some(libc::ENOSYS) | Some(libc::EINVAL)) {
        // no clone3 (< 5.3), or a hardening layer that refuses the call:
        // Use fork when clone3 is not available.
        CLONE3_BIT.store(2, Ordering::Relaxed);
        return Err(unsupported());
    }
    Err(err)
}

/// Decode a filled siginfo into the raw wait-status encoding that
/// `ExitStatus` understands (same encoding waitpid returns).
#[cfg(target_os = "linux")]
fn siginfo_to_wait_status(info: &libc::siginfo_t) -> i32 {
    let status = unsafe { info.si_status() };
    match info.si_code {
        libc::CLD_EXITED => (status & 0xff) << 8,
        libc::CLD_KILLED => status,
        libc::CLD_DUMPED => status | 0x80,
        _ => 0,
    }
}

/// Wait for a child by pidfd (`waitid(P_PIDFD, WEXITED)`), immune to PID
/// reuse. Returns `Ok(None)` only with `nohang` and a still-running child.
/// `Err` with raw `ENOSYS` (cached) = kernel without P_PIDFD (< 5.4); the
/// caller falls back to waitpid on the cached pid.
#[cfg(target_os = "linux")]
pub fn waitid_pidfd(pidfd: libc::c_int, nohang: bool) -> Result<Option<i32>> {
    if PIDFD_WAIT_BIT.load(Ordering::Relaxed) == 2 {
        return Err(unsupported());
    }
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let mut options = libc::WEXITED;
    if nohang {
        options |= libc::WNOHANG;
    }
    let ret = unsafe { libc::waitid(libc::P_PIDFD, pidfd as libc::id_t, &mut info, options) };
    if ret != 0 {
        let err = Error::last_os_error();
        if matches!(err.raw_os_error(), Some(libc::EINVAL) | Some(libc::ENOSYS)) {
            // idtype P_PIDFD unrecognized = kernel < 5.4. Our options and id
            // are otherwise always valid, so EINVAL cannot mean anything else
            // here.
            PIDFD_WAIT_BIT.store(2, Ordering::Relaxed);
            return Err(unsupported());
        }
        return Err(err);
    }
    if info.si_signo == 0 {
        // WNOHANG and no state change: siginfo was left zeroed.
        return Ok(None);
    }
    Ok(Some(siginfo_to_wait_status(&info)))
}

#[cfg(not(target_os = "linux"))]
pub fn waitid_pidfd(_pidfd: libc::c_int, _nohang: bool) -> Result<Option<i32>> {
    Err(unsupported())
}

/// Deliver SIGKILL by pidfd (`pidfd_send_signal`): hits the exact child even
/// if its PID was recycled. `Err` with raw `ENOSYS` = kernel < 5.1.
#[cfg(target_os = "linux")]
pub fn pidfd_send_signal_kill(pidfd: libc::c_int) -> Result<()> {
    let ret = unsafe {
        libc::syscall(
            SYS_PIDFD_SEND_SIGNAL,
            pidfd,
            libc::SIGKILL,
            std::ptr::null::<libc::c_void>(),
            0u32,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pidfd_send_signal_kill(_pidfd: libc::c_int) -> Result<()> {
    Err(unsupported())
}

// ---------------------------------------------------------------------------
// Descriptor execution sequence.
// ---------------------------------------------------------------------------

/// Render a file descriptor as `/proc/self/fd/N` into a stack buffer and
/// return the NUL-terminated length, or None when it does not fit.
fn proc_fd_path(fd: libc::c_int, buf: &mut [u8; 32]) -> Option<usize> {
    if fd < 0 {
        return None;
    }
    const PREFIX: &[u8] = b"/proc/self/fd/";
    buf[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut digits = [0u8; 12];
    let mut n = fd;
    let mut len = 0;
    if n == 0 {
        digits[0] = b'0';
        len = 1;
    }
    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    let total = PREFIX.len() + len;
    if total + 1 > buf.len() {
        return None;
    }
    for (i, d) in digits[..len].iter().rev().enumerate() {
        buf[PREFIX.len() + i] = *d;
    }
    buf[total] = 0;
    Some(total)
}

/// Test hooks can disable one execution method.
/// Compiled only under the `test-hooks` feature.
#[cfg(feature = "test-hooks")]
fn hook_disabled(name: &[u8]) -> bool {
    // libc marks getenv unsafe; it only reads `environ`, never mutates.
    unsafe { !libc::getenv(name.as_ptr() as *const libc::c_char).is_null() }
}
#[cfg(not(feature = "test-hooks"))]
fn hook_disabled(_name: &[u8]) -> bool {
    false
}

/// Execute `fd` in place of the current process.
///
/// Linux first uses `execveat(fd, "", AT_EMPTY_PATH)`. Linux then uses
/// `execve("/proc/self/fd/N")` when procfs is available. FreeBSD uses
/// `fexecve(fd)`.
///
/// The function tries the next method after an environmental error. It
/// returns all image errors immediately.
///
/// # Safety
/// On success this function never returns.
pub unsafe fn exec_fd(
    fd: libc::c_int,
    argv: *const *const libc::c_char,
    envp: *const *const libc::c_char,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    if !hook_disabled(b"MEMFD_NG_TEST_NO_EXECVEAT\0") {
        libc::syscall(
            libc::SYS_execveat,
            fd,
            b"\0".as_ptr(),
            argv,
            envp,
            AT_EMPTY_PATH,
        );
        let err = Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ENOSYS) | Some(libc::ENOENT) => {}
            _ => return Err(err),
        }
    }

    // FreeBSD: fexecve(2) is a kernel facility, not a procfs workaround.
    #[cfg(target_os = "freebsd")]
    {
        libc::fexecve(fd, argv, envp);
        let err = Error::last_os_error();
        if err.raw_os_error() != Some(libc::ENOENT) {
            return Err(err);
        }
    }

    if !hook_disabled(b"MEMFD_NG_TEST_NO_PROC\0") && proc_available() {
        let mut buf = [0u8; 32];
        if let Some(_len) = proc_fd_path(fd, &mut buf) {
            libc::execve(buf.as_ptr() as *const libc::c_char, argv, envp);
            let err = Error::last_os_error();
            // ENOENT means that procfs is not available. QEMU user-mode
            // emulation can return ENOEXEC for this method. A named path can
            // still succeed after either error.
            if !matches!(err.raw_os_error(), Some(libc::ENOENT) | Some(libc::ENOEXEC)) {
                return Err(err);
            }
        }
    }

    // The descriptor methods are not available. Use ENOSYS as the internal
    // result. Image errors have already returned from this function.
    Err(Error::from_raw_os_error(libc::ENOSYS))
}

/// Execute a named path in place of the current process.
///
/// # Safety
/// On success this function never returns.
pub unsafe fn exec_path(
    path: *const libc::c_char,
    argv: *const *const libc::c_char,
    envp: *const *const libc::c_char,
) -> Result<()> {
    libc::execve(path, argv, envp);
    Err(Error::last_os_error())
}

// ---------------------------------------------------------------------------
// Temporary-file sequence. This code runs in the child without allocation.
// ---------------------------------------------------------------------------

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
const ST_NOEXEC_FLAG: libc::c_long = 0x0008;

/// Kernel-ABI statfs (what the raw syscall fills). The libc crate's statfs
/// struct has no f_flags on gnu targets, so the noexec check talks to the
/// kernel directly; this layout matches every 64-bit Linux ABI.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[repr(C)]
struct KernelStatfs {
    f_type: i64,
    f_bsize: i64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_fsid: [u32; 2],
    f_namelen: i64,
    f_frsize: i64,
    f_flags: i64,
    f_spare: [i64; 4],
}

/// Return true when the mount for `dir` forbids execution.
///
/// Other targets use the execution result because this check is only defined
/// for 64-bit Linux.
fn mount_noexec(dir: *const libc::c_char) -> bool {
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        let mut fs: KernelStatfs = unsafe { std::mem::zeroed() };
        let ok = unsafe { libc::syscall(libc::SYS_statfs, dir, &mut fs as *mut KernelStatfs) == 0 };
        ok && (fs.f_flags & ST_NOEXEC_FLAG) != 0
    }
    #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
    {
        let _ = dir;
        false
    }
}

/// Append `n` as decimal ASCII. Returns the new length.
fn push_dec(buf: &mut [u8], mut len: usize, mut n: u64) -> usize {
    let mut digits = [0u8; 20];
    let mut d = 0;
    if n == 0 {
        digits[0] = b'0';
        d = 1;
    }
    while n > 0 {
        digits[d] = b'0' + (n % 10) as u8;
        n /= 10;
        d += 1;
    }
    for byte in digits[..d].iter().rev() {
        if len < buf.len() {
            buf[len] = *byte;
            len += 1;
        }
    }
    len
}

/// Fill 16 random bytes without the heap: getrandom(2), /dev/urandom, then a
/// monotonic-clock mix as a last resort.
fn fill_random(out: &mut [u8; 16]) {
    #[cfg(target_os = "linux")]
    unsafe {
        let n = libc::syscall(libc::SYS_getrandom, out.as_mut_ptr(), out.len(), 0);
        if n == out.len() as libc::c_long {
            return;
        }
    }
    unsafe {
        let fd = libc::open(
            b"/dev/urandom\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd >= 0 {
            let ok = matches!(read_fill(fd, out), Ok(n) if n == out.len());
            libc::close(fd);
            if ok {
                return;
            }
        }
        let mut ts: libc::timespec = std::mem::zeroed();
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
        let mix = (ts.tv_nsec as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ ((libc::getpid() as u64) << 32)
            ^ (libc::getuid() as u64);
        out[..8].copy_from_slice(&mix.to_ne_bytes());
        out[8..].copy_from_slice(&mix.swap_bytes().to_ne_bytes());
    }
}

const FALLBACK_PREFIX: &[u8] = b".memfd-ng-";
const HEX: &[u8] = b"0123456789abcdef";

/// A NUL-terminated fallback-file path built on the stack, reported to the
/// parent so it can unlink the file after reaping the child.
pub struct NamedPath {
    bytes: [u8; 192],
    len: usize, // includes the NUL
}

impl NamedPath {
    pub fn as_ptr(&self) -> *const libc::c_char {
        self.bytes.as_ptr() as *const libc::c_char
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len - 1]
    }
}

/// An image staged on an executable filesystem.
pub struct TmpfsPayload {
    /// Read-only descriptor for the execution sequence.
    ///
    /// A value of -1 means that only the named path is available. The function
    /// closes the write descriptor before it returns.
    pub fd: libc::c_int,
    /// The temporary file path when one exists.
    ///
    /// The parent removes this path after it receives the execution result.
    pub named: Option<NamedPath>,
}

/// Create and fill an image file on an executable filesystem.
///
/// With procfs, the function replaces the write descriptor with a read-only
/// descriptor. It reports a named path to the parent. Without procfs, it
/// closes the write descriptor and returns the named path.
///
/// The whole function avoids the allocator so it is safe to run in a forked
/// child before exec, where another thread may hold a malloc lock.
pub fn tmpfs_payload(code: &[u8]) -> Result<TmpfsPayload> {
    unsafe {
        let uid = libc::getuid() as u64;
        let pid = libc::getpid() as u64;
        let mut rnd = [0u8; 16];
        fill_random(&mut rnd);
        let env = |name: &[u8]| libc::getenv(name.as_ptr() as *const libc::c_char);

        // Check XDG_RUNTIME_DIR first. Then check TMPDIR or /tmp. Check
        // /dev/shm, /var/tmp, and $HOME/.cache after these directories. Each
        // tuple contains the path and its path-construction flags.
        let candidates: [(*const libc::c_char, bool, bool, bool); 5] = [
            (env(b"XDG_RUNTIME_DIR\0"), false, false, false),
            (env(b"TMPDIR\0"), true, false, false),
            (
                b"/dev/shm\0".as_ptr() as *const libc::c_char,
                false,
                false,
                true,
            ),
            (
                b"/var/tmp\0".as_ptr() as *const libc::c_char,
                false,
                false,
                true,
            ),
            (env(b"HOME\0"), false, true, false),
        ];
        let mut last_err: Option<Error> = None;

        for (dir, default_tmp, cache, system) in candidates {
            // This test hook skips the fixed system directories. It lets a
            // test select a directory through the environment.
            if system && hook_disabled(b"MEMFD_NG_TEST_NO_SYSDIRS\0") {
                continue;
            }
            let dir = if !dir.is_null() {
                dir
            } else if default_tmp {
                b"/tmp\0".as_ptr() as *const libc::c_char
            } else {
                continue; // Skip an environment variable that is not set.
            };

            if mount_noexec(dir) {
                last_err.get_or_insert(Error::from_raw_os_error(libc::EACCES));
                continue;
            }

            // "<dir>/.memfd-ng-<uid>-<pid>-<32 hex>" on the stack.
            let mut path = [0u8; 192];
            let mut len = {
                let d = dir as *const u8;
                let mut l = 0usize;
                while *d.add(l) != 0 && l < path.len() - 64 {
                    path[l] = *d.add(l);
                    l += 1;
                }
                l
            };
            if len > 0 && path[len - 1] != b'/' {
                path[len] = b'/';
                len += 1;
            }
            // Create $HOME/.cache when this candidate uses the home directory.
            // The next open call reports any mkdir failure that prevents use.
            if cache {
                const CACHE: &[u8] = b".cache";
                if len + CACHE.len() + 1 >= path.len() {
                    continue;
                }
                path[len..len + CACHE.len()].copy_from_slice(CACHE);
                // The zero-filled buffer provides the NUL terminator for mkdir.
                libc::mkdir(path.as_ptr() as *const libc::c_char, 0o700);
                path[len + CACHE.len()] = b'/';
                len += CACHE.len() + 1;
            }
            if len + FALLBACK_PREFIX.len() + 1 + 20 + 1 + 20 + 1 + 32 >= path.len() {
                continue;
            }
            path[len..len + FALLBACK_PREFIX.len()].copy_from_slice(FALLBACK_PREFIX);
            len += FALLBACK_PREFIX.len();
            len = push_dec(&mut path, len, uid);
            path[len] = b'-';
            len += 1;
            len = push_dec(&mut path, len, pid);
            path[len] = b'-';
            len += 1;
            for byte in rnd.iter() {
                path[len] = HEX[(*byte >> 4) as usize];
                path[len + 1] = HEX[(*byte & 0xf) as usize];
                len += 2;
            }
            path[len] = 0;
            let cpath = path.as_ptr() as *const libc::c_char;

            // On Linux, first try O_TMPFILE. This method creates the image in
            // an inode without a name.
            #[cfg(target_os = "linux")]
            if !hook_disabled(b"MEMFD_NG_TEST_NO_OTMPFILE\0") {
                let fd = libc::open(dir, libc::O_TMPFILE | libc::O_RDWR | libc::O_CLOEXEC, 0o700);
                if fd >= 0 {
                    let staged = libc::fchmod(fd, 0o700) == 0 && write_all(fd, code).is_ok();
                    if !staged {
                        let err = Error::last_os_error();
                        // Close the unnamed inode after a write failure.
                        libc::close(fd);
                        last_err.get_or_insert(err);
                    } else if proc_available() {
                        // Reopen the file as read-only through procfs. Link the
                        // inode to a path for QEMU user-mode emulation. This
                        // linkat form does not require additional privileges.
                        // The parent removes the path after execution.
                        let mut pbuf = [0u8; 32];
                        let mut ro = -1;
                        if proc_fd_path(fd, &mut pbuf).is_some() {
                            ro = libc::open(
                                pbuf.as_ptr() as *const libc::c_char,
                                libc::O_RDONLY | libc::O_CLOEXEC,
                            );
                        }
                        if ro >= 0 {
                            let linked = libc::linkat(
                                libc::AT_FDCWD,
                                pbuf.as_ptr() as *const libc::c_char,
                                libc::AT_FDCWD,
                                cpath,
                                AT_SYMLINK_FOLLOW,
                            ) == 0;
                            libc::close(fd);
                            if linked {
                                let mut bytes = [0u8; 192];
                                bytes[..path.len()].copy_from_slice(&path);
                                return Ok(TmpfsPayload {
                                    fd: ro,
                                    named: Some(NamedPath {
                                        bytes,
                                        len: len + 1,
                                    }),
                                });
                            }
                            // Return the read-only descriptor if linkat fails.
                            return Ok(TmpfsPayload {
                                fd: ro,
                                named: None,
                            });
                        }
                        let err = Error::last_os_error();
                        libc::close(fd);
                        last_err.get_or_insert(err);
                    } else {
                        // Without procfs, try to link the anonymous inode with
                        // AT_EMPTY_PATH. This operation requires
                        // CAP_DAC_READ_SEARCH. Close the write descriptor
                        // before execution to prevent ETXTBSY.
                        let linked = libc::linkat(
                            fd,
                            b"\0".as_ptr() as *const libc::c_char,
                            libc::AT_FDCWD,
                            cpath,
                            AT_EMPTY_PATH,
                        ) == 0;
                        if linked {
                            libc::close(fd);
                            let mut bytes = [0u8; 192];
                            bytes[..path.len()].copy_from_slice(&path);
                            return Ok(TmpfsPayload {
                                fd: -1,
                                named: Some(NamedPath {
                                    bytes,
                                    len: len + 1,
                                }),
                            });
                        }
                        let err = Error::last_os_error();
                        libc::close(fd); // This closes the unnamed inode.
                        last_err.get_or_insert(err);
                    }
                }
                // Use a named file if O_TMPFILE is not available.
            }

            // This test hook requires the O_TMPFILE method.
            #[cfg(feature = "test-hooks")]
            if hook_disabled(b"MEMFD_NG_TEST_NO_NAMED_STAGE\0") {
                last_err.get_or_insert(Error::from_raw_os_error(libc::EPERM));
                continue;
            }

            let open_flags = libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC;
            let mut fd = libc::open(cpath, open_flags, 0o700);
            if fd < 0 {
                // Generate a new 128-bit suffix and try once more.
                fill_random(&mut rnd);
                let mut hlen = len - 32;
                for byte in rnd.iter() {
                    path[hlen] = HEX[(byte >> 4) as usize];
                    path[hlen + 1] = HEX[(byte & 0xf) as usize];
                    hlen += 2;
                }
                fd = libc::open(cpath, open_flags, 0o700);
            }
            if fd < 0 {
                last_err.get_or_insert(Error::last_os_error());
                continue;
            }

            // open(2)'s mode is filtered by the umask, which may strip the
            // execute bit; fchmod(2) is not, so set it explicitly.
            if libc::fchmod(fd, 0o700) != 0 {
                let err = Error::last_os_error();
                libc::close(fd);
                libc::unlink(cpath);
                last_err.get_or_insert(err);
                continue;
            }
            if let Err(err) = write_all(fd, code) {
                libc::close(fd);
                libc::unlink(cpath);
                last_err.get_or_insert(err);
                continue;
            }

            // Replace the write descriptor with a read-only descriptor.
            // This prevents ETXTBSY for regular files. Keep the path for
            // emulators that require path-based execution. The parent removes
            // the path after execution.
            if proc_available() {
                let mut pbuf = [0u8; 32];
                if proc_fd_path(fd, &mut pbuf).is_some() {
                    let ro = libc::open(
                        pbuf.as_ptr() as *const libc::c_char,
                        libc::O_RDONLY | libc::O_CLOEXEC,
                    );
                    if ro >= 0 {
                        libc::close(fd);
                        let mut bytes = [0u8; 192];
                        bytes[..path.len()].copy_from_slice(&path);
                        return Ok(TmpfsPayload {
                            fd: ro,
                            named: Some(NamedPath {
                                bytes,
                                len: len + 1,
                            }),
                        });
                    }
                }
                let err = Error::last_os_error();
                libc::close(fd);
                libc::unlink(cpath);
                last_err.get_or_insert(err);
                continue;
            }

            // Without procfs, close the write descriptor and return the path.
            // The additional byte in the length is the NUL terminator.
            libc::close(fd);
            let mut bytes = [0u8; 192];
            bytes[..path.len()].copy_from_slice(&path);
            return Ok(TmpfsPayload {
                fd: -1,
                named: Some(NamedPath {
                    bytes,
                    len: len + 1,
                }),
            });
        }

        Err(last_err.unwrap_or_else(|| Error::from(ErrorKind::Unsupported)))
    }
}

// ---------------------------------------------------------------------------
// CLOEXEC-pipe protocol
// ---------------------------------------------------------------------------

/// A message the forked child sends before exec or exit.
#[derive(Debug)]
pub enum PipeMsg {
    /// Child execed successfully and the pipe closed: EOF.
    Success,
    /// Child failed to exec; carries the real errno.
    Failure(i32),
    /// Child is about to exec a still-named tmpfs file (no-procfs corner);
    /// the parent owns the name now and must unlink it after reaping.
    NamedPath(Vec<u8>),
}

/// Write the errno message: 4 big-endian errno bytes + `NOEX` footer.
pub fn pipe_write_errno(pipe: libc::c_int, err: &Error) {
    let mut msg = [0u8; 8];
    msg[..4].copy_from_slice(&(err.raw_os_error().unwrap_or(libc::EIO) as u32).to_be_bytes());
    msg[4..].copy_from_slice(b"NOEX");
    pipe_write_all(pipe, &msg);
}

/// Write the named-path message: 2 big-endian length bytes + `PATH` footer +
/// the path bytes (no NUL). Only used in the no-procfs corner.
pub fn pipe_write_named_path(pipe: libc::c_int, path: &[u8]) {
    if path.len() > u16::MAX as usize {
        return;
    }
    let mut head = [0u8; 6];
    head[..2].copy_from_slice(&(path.len() as u16).to_be_bytes());
    head[2..].copy_from_slice(b"PATH");
    pipe_write_all(pipe, &head);
    pipe_write_all(pipe, path);
}

fn pipe_write_all(pipe: libc::c_int, mut buf: &[u8]) {
    while !buf.is_empty() {
        let n = unsafe { libc::write(pipe, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if n < 0 {
            let errno = Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::EINTR {
                continue;
            }
            break; // parent died; nothing left to tell
        }
        if n == 0 {
            break;
        }
        buf = &buf[n as usize..];
    }
}

/// Read one message from the CLOEXEC pipe, looping over EINTR.
///
/// Wire shapes (headers are read 6 bytes at a time so the PATH image is
/// never overshot):
/// - failure: `[4B errno BE]["NOEX"]` — 8 bytes
/// - named path: `[2B len BE]["PATH"] + len bytes`
pub fn pipe_read(pipe: libc::c_int) -> Result<PipeMsg> {
    let mut head = [0u8; 8];
    let mut got = 0;
    while got < 6 {
        let n = cvt_ssize(unsafe {
            libc::read(pipe, head[got..].as_mut_ptr() as *mut libc::c_void, 6 - got)
        })?;
        if n == 0 {
            if got == 0 {
                return Ok(PipeMsg::Success);
            }
            return Err(Error::new(
                ErrorKind::InvalidData,
                "short read on exec pipe",
            ));
        }
        got += n;
    }

    if &head[2..6] == b"PATH" {
        let len = u16::from_be_bytes([head[0], head[1]]) as usize;
        let mut path = vec![0u8; len];
        let n = read_fill(pipe, &mut path)?;
        path.truncate(n);
        return Ok(PipeMsg::NamedPath(path));
    }

    while got < 8 {
        let n = cvt_ssize(unsafe {
            libc::read(pipe, head[got..].as_mut_ptr() as *mut libc::c_void, 8 - got)
        })?;
        if n == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "short read on exec pipe",
            ));
        }
        got += n;
    }
    if &head[4..8] == b"NOEX" {
        let code = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as i32;
        return Ok(PipeMsg::Failure(code));
    }
    Err(Error::new(ErrorKind::InvalidData, "bad exec pipe message"))
}
