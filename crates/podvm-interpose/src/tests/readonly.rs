//! TODO/cli.md T-1655. Under a table with a read-only root and a writable
//! directory, a write under the root answers `EROFS` and a read passes; a
//! write in the writable directory lands in the host directory.

use super::*;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
}

#[test]
fn a_write_under_a_read_only_prefix_answers_erofs() {
    let dir = std::env::temp_dir().join(format!("podvm-ro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("root/etc")).unwrap();
    std::fs::create_dir_all(dir.join("root/tmp")).unwrap();
    std::fs::write(dir.join("root/etc/hostname"), b"ro\n").unwrap();
    let root = dir.join("root");
    // SAFETY: only a test changes these variables; the interposer tests run
    // with one thread (gate.yml).
    unsafe {
        std::env::set_var("PODVM_MAPS", format!("/:{}", root.display()));
        std::env::set_var("PODVM_RO", "ro:/,rw:/tmp");
    }
    let open = |p: &core::ffi::CStr, flags: c_int| {
        clear_errno();
        let fd = unsafe { crate::open(p.as_ptr(), flags, 0o644) };
        (fd, errno())
    };
    let (fd, e) = open(c"/etc/new", 0o1 | 0o100);
    assert_eq!((fd, e), (-1, crate::map::EROFS), "a create under /");
    let (fd, e) = open(c"/etc/hostname", 0o2);
    assert_eq!((fd, e), (-1, crate::map::EROFS), "a write open under /");
    let (fd, _) = open(c"/etc/hostname", 0);
    assert!(fd >= 0, "a read open under /");
    unsafe { close(fd) };
    let (fd, _) = open(c"/tmp/new", 0o1 | 0o100);
    assert!(fd >= 0, "a create under /tmp");
    unsafe { close(fd) };
    assert!(root.join("tmp/new").exists());
    clear_errno();
    assert_eq!(unsafe { crate::mkdir(c"/etc/d".as_ptr(), 0o755) }, -1);
    assert_eq!(errno(), crate::map::EROFS);
    assert!(unsafe { crate::fopen(c"/etc/hostname".as_ptr(), c"w".as_ptr()) }.is_null());
    let f = unsafe { crate::fopen(c"/etc/hostname".as_ptr(), c"r".as_ptr()) };
    assert!(!f.is_null());
    // SAFETY: as above.
    unsafe {
        std::env::remove_var("PODVM_MAPS");
        std::env::remove_var("PODVM_RO");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
