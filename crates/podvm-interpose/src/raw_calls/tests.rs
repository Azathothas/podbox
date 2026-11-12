//! T-1697: a raw call takes the same table as the call of libc.

use super::*;
use std::ffi::CString;

// The numbers of this machine that the table of the object does not name
// (TODO/interpose.md T-1721).
#[cfg(target_arch = "x86_64")]
const MKNODAT: c_long = 259;
#[cfg(target_arch = "x86_64")]
const GETPID: c_long = 39;
#[cfg(target_arch = "aarch64")]
const MKNODAT: c_long = 33;
#[cfg(target_arch = "aarch64")]
const GETPID: c_long = 172;

const AT_FDCWD: c_long = -100;
const AT_SYMLINK_NOFOLLOW: c_long = 0x100;
const STATX_BASIC_STATS: c_long = 0x7ff;
const ENOENT: c_int = 2;

/// A rootfs with a file of seven bytes and the directory that npm's
/// `lstat` missed, anchored at guest `/` until [`unset`].
fn rootfs(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-t1697-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("usr/local/lib/node_modules")).unwrap();
    std::fs::create_dir_all(d.join("etc")).unwrap();
    std::fs::write(d.join("etc/podvm-t1697"), b"guest!\n").unwrap();
    // SAFETY: only a test changes this variable, one test at a time.
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", d.display())) };
    d
}

fn unset(d: &std::path::Path) {
    // SAFETY: as in `rootfs`.
    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(d);
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

/// The Prove clause: a raw `statx` of a guest path answers from the
/// rootfs. The control is the same call issued past the table, which is
/// what libuv did before: the host has no such path.
#[test]
fn a_raw_statx_of_a_guest_path_answers_from_the_rootfs() {
    let d = rootfs("statx");
    let file = CString::new("/etc/podvm-t1697").unwrap();
    let dir = CString::new("/usr/local/lib/node_modules").unwrap();
    let mut stx = [0u8; 256];
    let call = |p: &CString, flags: c_long, buf: &mut [u8]| {
        let args = [AT_FDCWD, p.as_ptr() as c_long, flags, STATX_BASIC_STATS];
        unsafe {
            syscall(
                SYS_STATX,
                args[0],
                args[1],
                args[2],
                args[3],
                buf.as_mut_ptr() as c_long,
                0,
            )
        }
    };
    let past = unsafe {
        forward(
            SYS_STATX,
            [
                AT_FDCWD,
                file.as_ptr() as c_long,
                0,
                STATX_BASIC_STATS,
                stx.as_mut_ptr() as c_long,
                0,
            ],
        )
    };
    let past_errno = crate::errno();
    let rc = call(&file, 0, &mut stx);
    let size = u64_at(&stx, 40);
    // npm's miss: `lstat` of the modules directory.
    let rc_dir = call(&dir, AT_SYMLINK_NOFOLLOW, &mut stx);
    let mode = u16::from_le_bytes([stx[28], stx[29]]);
    // The `stat` shape of the same table: `newfstatat`.
    let mut st = [0u8; 144];
    let rc_at = unsafe {
        syscall(
            SYS_NEWFSTATAT,
            AT_FDCWD,
            file.as_ptr() as c_long,
            st.as_mut_ptr() as c_long,
            0,
            0,
            0,
        )
    };
    unset(&d);
    assert_eq!(
        (past, past_errno),
        (-1, ENOENT),
        "the control found a host file"
    );
    assert_eq!((rc, size), (0, 7), "errno {}", crate::errno());
    assert_eq!((rc_dir, mode & 0o170000), (0, 0o040000));
    assert_eq!((rc_at, u64_at(&st, 48)), (0, 7));
}

/// A raw call that takes a path and that no entry point maps answers
/// `ENOSYS` where a table is set, and so does `io_uring_setup`. With no
/// table both reach the kernel, as each number with no path does.
#[test]
fn a_raw_call_with_no_route_is_refused_where_a_table_is_set() {
    let d = rootfs("refuse");
    let fifo = CString::new("/etc/podvm-t1697-fifo").unwrap();
    const S_IFIFO: c_long = 0o010000;
    let mknodat = |p: &CString| unsafe {
        syscall(
            MKNODAT,
            AT_FDCWD,
            p.as_ptr() as c_long,
            S_IFIFO | 0o600,
            0,
            0,
            0,
        )
    };
    let refused = mknodat(&fifo);
    let refused_errno = crate::errno();
    let mut params = [0u8; 120];
    let ring = unsafe {
        syscall(
            SYS_IO_URING_SETUP,
            1,
            params.as_mut_ptr() as c_long,
            0,
            0,
            0,
            0,
        )
    };
    let ring_errno = crate::errno();
    let host = d.join("etc/podvm-t1697-fifo");
    unsafe { std::env::remove_var("PODVM_MAPS") };
    let host_c = CString::new(host.to_string_lossy().as_bytes()).unwrap();
    let made = mknodat(&host_c);
    let is_fifo = std::os::unix::fs::FileTypeExt::is_fifo(
        &std::fs::symlink_metadata(&host).unwrap().file_type(),
    );
    let pid = unsafe { syscall(GETPID, 0, 0, 0, 0, 0, 0) };
    unset(&d);
    assert_eq!((refused, refused_errno), (-1, ENOSYS));
    assert_eq!((ring, ring_errno), (-1, ENOSYS));
    assert_eq!((made, is_fifo), (0, true), "errno {}", crate::errno());
    assert_eq!(pid, std::process::id() as c_long);
}

/// The kernel's `getcwd` answers the length with the NUL, and the guest
/// name after a raw `chdir` into the rootfs.
#[test]
fn a_raw_getcwd_answers_the_guest_name() {
    let back = std::env::current_dir().unwrap();
    let d = rootfs("cwd");
    let etc = CString::new("/etc").unwrap();
    let rc = unsafe { syscall(SYS_CHDIR, etc.as_ptr() as c_long, 0, 0, 0, 0, 0) };
    let mut buf = [0u8; 64];
    let n = unsafe {
        syscall(
            SYS_GETCWD,
            buf.as_mut_ptr() as c_long,
            buf.len() as c_long,
            0,
            0,
            0,
            0,
        )
    };
    unsafe { std::env::remove_var("PODVM_MAPS") };
    std::env::set_current_dir(back).unwrap();
    unset(&d);
    assert_eq!(rc, 0);
    assert_eq!((n, &buf[..5]), (5, &b"/etc\0"[..]));
}
