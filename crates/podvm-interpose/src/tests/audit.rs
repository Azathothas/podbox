//! TODO/interpose.md T-1754, T-1755: the path calls of the audit take the
//! path table: a guest path that only the rootfs has is reached.

use super::*;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

unsafe extern "C" fn every(_: *const c_void) -> c_int {
    1
}

unsafe extern "C" fn same(_: *const *const c_void, _: *const *const c_void) -> c_int {
    0
}

#[test]
fn the_audited_calls_take_the_path_table() {
    let dir = std::env::temp_dir().join(format!("podvm-audit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("root");
    let guest = format!("/podvm-t1754-{}", std::process::id());
    let inner = root.join(guest.trim_start_matches('/'));
    std::fs::create_dir_all(inner.join("d")).unwrap();
    std::fs::write(inner.join("f"), b"x").unwrap();
    std::os::unix::fs::symlink("f", inner.join("l")).unwrap();
    let c = |s: &str| std::ffi::CString::new(format!("{guest}{s}")).unwrap();
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", root.display())) };

    let mut list: *mut *mut c_void = core::ptr::null_mut();
    let n = unsafe {
        crate::scandir64(
            c("").as_ptr(),
            (&mut list as *mut *mut *mut c_void).cast(),
            every,
            same,
        )
    };
    assert!(
        n >= 4,
        "scandir64 lists the rootfs directory: {n}, errno {}",
        errno()
    );
    for i in 0..n as usize {
        unsafe { free(*list.add(i)) };
    }
    unsafe { free(list as *mut c_void) };

    // lutimes sets the times of the link, not of its target.
    let times = [0i64; 4];
    assert_eq!(
        unsafe { crate::lutimes(c("/l").as_ptr(), times.as_ptr() as *const c_void) },
        0,
        "lutimes: errno {}",
        errno()
    );
    let link = std::fs::symlink_metadata(inner.join("l")).unwrap();
    let file = std::fs::metadata(inner.join("f")).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(link.mtime(), 0, "the link's own time");
    assert_ne!(file.mtime(), 0, "the target keeps its time");

    assert_eq!(
        unsafe { crate::mkfifo(c("/p").as_ptr(), 0o600) },
        0,
        "mkfifo: errno {}",
        errno()
    );
    assert!(inner.join("p").exists(), "the fifo is in the rootfs");
    let mut st = [0u8; 256];
    assert_eq!(
        unsafe { crate::statvfs64(c("/d").as_ptr(), st.as_mut_ptr() as *mut c_void) },
        0
    );

    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(&dir);
}
