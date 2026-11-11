//! TODO/interpose.md T-1704. Under the exe override, `realpath`,
//! `canonicalize_file_name` and `readlink` of `/proc/self/exe` give one
//! answer: the program, never the loader that the kernel link names.

use super::*;

type CanonFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type ReadlinkFn = unsafe extern "C" fn(*const c_char, *mut c_char, usize) -> isize;

/// The environment of the override, set for one test and removed after.
/// `readlink` of the exe counts each answer in the memo (T-0413), so the
/// override holds a memo descriptor as the CLI gives one.
struct Override {
    _memo: std::fs::File,
}

impl Override {
    fn set(exe: &std::path::Path, memo: &std::path::Path) -> Override {
        use std::os::fd::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(memo)
            .expect("memo file");
        // SAFETY: only a test changes these variables; the interposer tests
        // run with one thread (gate.yml), so no thread reads them at once.
        unsafe {
            std::env::set_var("PODVM_GUEST_EXE_FORCE", "1");
            std::env::set_var("PODVM_GUEST_EXE", exe);
            std::env::set_var(
                std::str::from_utf8(crate::memo::MEMO_FD_VAR).unwrap(),
                file.as_raw_fd().to_string(),
            );
            std::env::remove_var("PODVM_MAPS");
        }
        Override { _memo: file }
    }
}

impl Drop for Override {
    fn drop(&mut self) {
        // SAFETY: as in `set`.
        unsafe {
            std::env::remove_var("PODVM_GUEST_EXE_FORCE");
            std::env::remove_var("PODVM_GUEST_EXE");
            std::env::remove_var(std::str::from_utf8(crate::memo::MEMO_FD_VAR).unwrap());
        }
    }
}

/// The program's guest name is a link to the file: `readlink` gives the
/// name as recorded, and `realpath` and `canonicalize_file_name` give the
/// file after the link, not the loader of this test process.
#[test]
fn realpath_of_self_exe_names_the_program_as_readlink_does() {
    let dir = std::env::temp_dir().join(format!("podvm-exe-realpath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let prog = dir.join("prog");
    std::fs::write(&prog, b"\x7fELF").expect("prog");
    let link = dir.join("link");
    std::os::unix::fs::symlink(&prog, &link).expect("link");
    let want = std::fs::canonicalize(&prog).expect("canonical prog");
    let want = want.as_os_str().as_bytes().to_vec();
    let free = libc_free();
    let exe = CString::new("/proc/self/exe").unwrap();
    let _env = Override::set(&link, &dir.join("memo"));

    let (got, e) = call_realpath(wrapped_realpath(), &exe);
    assert!(!got.is_null(), "realpath failed, errno {e}");
    assert_eq!(resolved_bytes(got), want, "realpath");
    unsafe { free(got) };

    let mut out = [0 as c_char; crate::map::OUT];
    let got = unsafe { realpath(exe.as_ptr(), out.as_mut_ptr()) };
    assert!(!got.is_null());
    assert_eq!(
        resolved_bytes(got as *mut c_void),
        want,
        "realpath into a buffer"
    );

    let canon: CanonFn = lookup(RTLD_DEFAULT, "canonicalize_file_name");
    let got = unsafe { canon(exe.as_ptr()) };
    assert!(!got.is_null(), "canonicalize_file_name failed");
    assert_eq!(resolved_bytes(got), want, "canonicalize_file_name");
    unsafe { free(got) };

    let read: ReadlinkFn = lookup(RTLD_DEFAULT, "readlink");
    let mut buf = [0u8; 4096];
    let n = unsafe { read(exe.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
    assert!(n > 0, "readlink failed");
    assert_eq!(&buf[..n as usize], link.as_os_str().as_bytes(), "readlink");

    // T-1704: the fortified entry points take the same routes.
    let mut out = [0 as c_char; crate::map::OUT];
    let got = unsafe { crate::__realpath_chk(exe.as_ptr(), out.as_mut_ptr(), out.len()) };
    assert!(!got.is_null());
    assert_eq!(resolved_bytes(got as *mut c_void), want, "__realpath_chk");
    let mut buf = [0u8; 4096];
    let n = unsafe {
        crate::__readlink_chk(
            exe.as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            100,
            buf.len(),
        )
    };
    assert_eq!(
        &buf[..n as usize],
        link.as_os_str().as_bytes(),
        "__readlink_chk"
    );
    let n = unsafe {
        crate::__readlinkat_chk(
            crate::map::AT_FDCWD,
            exe.as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            100,
            buf.len(),
        )
    };
    assert_eq!(
        &buf[..n as usize],
        link.as_os_str().as_bytes(),
        "__readlinkat_chk"
    );
    let mut cwd = [0 as c_char; 4096];
    let got = unsafe { crate::__getcwd_chk(cwd.as_mut_ptr(), cwd.len(), cwd.len()) };
    assert!(!got.is_null());
    assert_eq!(
        resolved_bytes(got as *mut c_void),
        std::env::current_dir().unwrap().as_os_str().as_bytes(),
        "__getcwd_chk"
    );

    // Another spelling is not the exe: it resolves as before.
    let other = CString::new(dir.as_os_str().as_bytes()).unwrap();
    let (got, _) = call_realpath(wrapped_realpath(), &other);
    assert!(!got.is_null());
    assert_eq!(
        resolved_bytes(got),
        std::fs::canonicalize(&dir).unwrap().as_os_str().as_bytes()
    );
    unsafe { free(got) };
    drop(_env);
    let _ = std::fs::remove_dir_all(&dir);
}
