//! TODO/interpose.md T-1725. An absolute link of the rootfs resolves in the
//! rootfs for each path call: as a directory of the path and as its last
//! component, for open, stat and access; `lstat` keeps the link; a link
//! whose target exists only on the host gives ENOENT.

use super::*;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

const S_IFMT: u32 = 0o170000;
const S_IFLNK: u32 = 0o120000;

#[test]
fn an_absolute_link_of_the_rootfs_resolves_in_the_rootfs() {
    let dir = std::env::temp_dir().join(format!("podvm-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("root");
    std::fs::create_dir_all(root.join("opt/abs")).unwrap();
    std::fs::create_dir_all(root.join("home")).unwrap();
    std::fs::write(root.join("opt/abs/f"), b"in-rootfs").unwrap();
    std::os::unix::fs::symlink("/opt/abs/f", root.join("home/link")).unwrap();
    std::os::unix::fs::symlink("/opt/abs", root.join("dirlink")).unwrap();
    // The target exists on the host (each Linux host has /proc), not in the rootfs.
    std::os::unix::fs::symlink("/etc/hostname", root.join("hostonly")).unwrap();
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", root.display())) };
    let cat = |p: &core::ffi::CStr| {
        clear_errno();
        let fd = unsafe { crate::open(p.as_ptr(), 0, 0) };
        if fd < 0 {
            return Err(errno());
        }
        let mut b = [0u8; 64];
        let n = unsafe { read(fd, b.as_mut_ptr() as *mut c_void, 64) };
        unsafe { close(fd) };
        Ok(String::from_utf8_lossy(&b[..n.max(0) as usize]).into_owned())
    };
    assert_eq!(
        cat(c"/home/link"),
        Ok("in-rootfs".to_string()),
        "the last component"
    );
    assert_eq!(
        cat(c"/dirlink/f"),
        Ok("in-rootfs".to_string()),
        "a directory of the path"
    );
    assert_eq!(cat(c"/hostonly"), Err(2), "a target that only the host has");
    let mut st = [0u8; 256];
    let mode = |st: &[u8; 256]| u32::from_le_bytes([st[24], st[25], st[26], st[27]]);
    assert_eq!(
        unsafe { crate::stat(c"/home/link".as_ptr(), st.as_mut_ptr() as *mut c_void) },
        0
    );
    assert_eq!(
        mode(&st) & S_IFMT,
        0o100000,
        "stat follows to the file of the rootfs"
    );
    assert_eq!(
        unsafe { crate::lstat(c"/home/link".as_ptr(), st.as_mut_ptr() as *mut c_void) },
        0
    );
    assert_eq!(mode(&st) & S_IFMT, S_IFLNK, "lstat keeps the link");
    assert_eq!(
        unsafe { crate::access(c"/dirlink/f".as_ptr(), 4) },
        0,
        "access through a directory link"
    );
    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(&dir);
}
