//! TODO/interpose.md T-1743. The fortified opens of glibc take the path
//! table as `open` and `openat` do. GNU tar 1.35 of Ubuntu 24.04 opens the
//! directory of `-C` with `__openat_2` and then creates each member with
//! `openat` on that descriptor (PodUtils/podcrates#102).

use super::*;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_CLOEXEC: c_int = 0o2000000;
const O_PATH: c_int = 0o10000000;
#[cfg(target_arch = "aarch64")]
const O_DIRECTORY: c_int = 0o40000;
#[cfg(not(target_arch = "aarch64"))]
const O_DIRECTORY: c_int = 0o200000;

fn body(fd: c_int) -> String {
    let mut b = [0u8; 64];
    let n = unsafe { read(fd, b.as_mut_ptr() as *mut c_void, 64) };
    unsafe { close(fd) };
    String::from_utf8_lossy(&b[..n.max(0) as usize]).into_owned()
}

#[test]
fn the_fortified_opens_take_the_path_table() {
    let dir = std::env::temp_dir().join(format!("podvm-fortify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("root");
    // A guest directory that no host has: an unmapped open answers ENOENT.
    let guest = format!("/podvm-t1743-{}", std::process::id());
    let inner = root.join(guest.trim_start_matches('/')).join("d");
    std::fs::create_dir_all(&inner).unwrap();
    std::fs::write(inner.join("f"), b"in-rootfs").unwrap();
    assert!(
        !std::path::Path::new(&guest).exists(),
        "the host has no {guest}"
    );
    let d = std::ffi::CString::new(format!("{guest}/d")).unwrap();
    let f = std::ffi::CString::new(format!("{guest}/d/f")).unwrap();
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", root.display())) };

    // tar's `chdir_do`: an absolute directory with O_PATH, then a member
    // relative to that descriptor.
    let flags = O_PATH | O_DIRECTORY | O_CLOEXEC;
    let at = unsafe { crate::__openat_2(crate::map::AT_FDCWD, d.as_ptr(), flags) };
    assert!(
        at >= 0,
        "__openat_2 of the absolute directory: errno {}",
        errno()
    );
    let out = unsafe { crate::openat(at, c"g".as_ptr(), O_WRONLY | O_CREAT, 0o644) };
    assert!(
        out >= 0,
        "openat of a member on the descriptor: errno {}",
        errno()
    );
    unsafe { close(out) };
    assert!(inner.join("g").exists(), "the member is in the rootfs");
    let rel = unsafe { crate::__openat64_2(at, c"f".as_ptr(), O_RDONLY) };
    assert!(
        rel >= 0,
        "__openat64_2 relative to the descriptor: errno {}",
        errno()
    );
    assert_eq!(body(rel), "in-rootfs");
    unsafe { close(at) };

    let fd = unsafe { crate::__open_2(f.as_ptr(), O_RDONLY) };
    assert!(fd >= 0, "__open_2: errno {}", errno());
    assert_eq!(body(fd), "in-rootfs");
    let fd = unsafe { crate::__open64_2(f.as_ptr(), O_RDONLY) };
    assert!(fd >= 0, "__open64_2: errno {}", errno());
    assert_eq!(body(fd), "in-rootfs");

    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fortified_open_needs_a_mode_as_glibc_says() {
    assert!(crate::chk_calls::needs_mode(O_CREAT));
    assert!(crate::chk_calls::needs_mode(O_WRONLY | O_CREAT));
    assert!(crate::chk_calls::needs_mode(
        0o20000000 | O_DIRECTORY | O_WRONLY
    ));
    assert!(!crate::chk_calls::needs_mode(O_RDONLY));
    assert!(!crate::chk_calls::needs_mode(
        O_PATH | O_DIRECTORY | O_CLOEXEC
    ));
    assert!(!crate::chk_calls::needs_mode(O_DIRECTORY));
}
