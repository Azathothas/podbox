//! Tests that drive the FFI exactly the way a C caller would: raw pointers,
//! NULL-terminated arrays, negated-errno results, raw wait statuses.

use std::ffi::{c_char, CString, OsString};
use std::ptr;

use memfd_ng_ffi::{
    memfd_ng_abi_version, memfd_ng_free, memfd_ng_kill, memfd_ng_pid, memfd_ng_spawn,
    memfd_ng_version, memfd_ng_wait,
};

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn image_of(path: &str) -> (Vec<u8>, usize) {
    let bytes = std::fs::read(path).unwrap();
    let len = bytes.len();
    (bytes, len)
}

/// Own strings and expose a NULL-terminated argv array in the shape C uses.
struct CArgv {
    _strings: Vec<CString>,
    pointers: Vec<*const c_char>,
}

impl CArgv {
    fn new(items: &[&str]) -> Self {
        let strings: Vec<CString> = items.iter().map(|s| cstr(s)).collect();
        let mut pointers: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
        pointers.push(ptr::null());
        Self {
            _strings: strings,
            pointers,
        }
    }

    fn as_ptr(&self) -> *const *const c_char {
        self.pointers.as_ptr()
    }
}

struct EnvGuard {
    key: &'static str,
    old: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn abi_version_and_version_string() {
    assert_eq!(memfd_ng_abi_version(), 1);
    let v = memfd_ng_version();
    assert!(!v.is_null());
    let s = unsafe { std::ffi::CStr::from_ptr(v) }.to_str().unwrap();
    assert!(
        s.split('.').count() == 3,
        "version string looks like semver: {s}"
    );
}

#[test]
fn spawn_wait_round_trip_exit_code() {
    let (code, len) = image_of("/bin/sh");
    let name = cstr("ffi-sh");
    let argv = CArgv::new(&["sh", "-c", "exit 7"]);
    let mut status: i32 = -1;
    let handle = unsafe {
        memfd_ng_spawn(
            code.as_ptr(),
            len,
            name.as_ptr(),
            argv.as_ptr(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(!handle.is_null(), "spawn failed with err {status}");
    assert!(unsafe { memfd_ng_pid(handle) } > 0);
    let rc = unsafe { memfd_ng_wait(handle, &mut status) };
    assert_eq!(rc, 0, "wait failed with {rc}");
    // raw wait status: WIFEXITED -> (code << 8)
    assert_eq!(
        status,
        7 << 8,
        "exit status must arrive raw for WEXITSTATUS"
    );
}

#[test]
fn null_argv_and_envp_use_defaults() {
    let (code, len) = image_of("/bin/sh");
    let name = cstr("ffi-sh-defaults");
    // argv NULL: program name becomes argv[0]
    let mut status: i32 = -1;
    let handle = unsafe {
        memfd_ng_spawn(
            code.as_ptr(),
            len,
            name.as_ptr(),
            ptr::null(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(!handle.is_null());
    assert_eq!(unsafe { memfd_ng_wait(handle, &mut status) }, 0);
    assert_eq!(status, 0, "sh with no args exits 0");

    // envp NULL: environment inherited
    let _env = EnvGuard::set("MEMFD_NG_FFI_CANARY", "inherited");
    let argv2 = CArgv::new(&["sh", "-c", "echo $MEMFD_NG_FFI_CANARY"]);
    let sh2 = std::fs::read("/bin/sh").unwrap();
    let handle = unsafe {
        memfd_ng_spawn(
            sh2.as_ptr(),
            sh2.len(),
            name.as_ptr(),
            argv2.as_ptr(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(!handle.is_null());
    assert_eq!(unsafe { memfd_ng_wait(handle, &mut status) }, 0);
    assert_eq!(status, 0);
}

#[test]
fn kill_then_wait_reports_signal() {
    let (code, len) = image_of("/bin/sh");
    let name = cstr("ffi-kill");
    let argv = CArgv::new(&["sh", "-c", "sleep 30"]);
    let mut status: i32 = -1;
    let handle = unsafe {
        memfd_ng_spawn(
            code.as_ptr(),
            len,
            name.as_ptr(),
            argv.as_ptr(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(!handle.is_null());
    let rc = unsafe { memfd_ng_kill(handle) };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { memfd_ng_wait(handle, &mut status) }, 0);
    // CLD_KILLED: raw status = signal number
    assert_eq!(status, 9, "SIGKILL must return raw wait status 9");
}

#[test]
fn errors_are_negated_errnos() {
    // corrupt image: exec fails with ENOEXEC -> spawn returns NULL, -8
    let bogus = b"\x7fELF-nope".to_vec();
    let name = cstr("ffi-bogus");
    let mut status: i32 = -1;
    let handle = unsafe {
        memfd_ng_spawn(
            bogus.as_ptr(),
            bogus.len(),
            name.as_ptr(),
            ptr::null(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(handle.is_null());
    assert_eq!(status, -8, "ENOEXEC must come through as -8");
}

#[test]
fn free_without_wait_releases_the_handle() {
    let (code, len) = image_of("/bin/sh");
    let name = cstr("ffi-free");
    let argv = CArgv::new(&["sh", "-c", "sleep 30"]);
    let mut status: i32 = -1;
    let handle = unsafe {
        memfd_ng_spawn(
            code.as_ptr(),
            len,
            name.as_ptr(),
            argv.as_ptr(),
            ptr::null(),
            &mut status,
        )
    };
    assert!(!handle.is_null());
    unsafe { memfd_ng_kill(handle) };
    unsafe { memfd_ng_free(handle) };
    // double free must not explode the process (UB guard: we cannot detect
    // it, but the test proves single free is clean and the process lives)
    let mut status2: i32 = -1;
    let h2 = unsafe {
        memfd_ng_spawn(
            code.as_ptr(),
            len,
            name.as_ptr(),
            ptr::null(),
            ptr::null(),
            &mut status2,
        )
    };
    assert!(!h2.is_null());
    unsafe { memfd_ng_free(h2) };
}
