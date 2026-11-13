//! TODO/interpose.md T-1756: an exec into an environment of the caller's
//! own keeps podvm's variables.

use std::ffi::{CStr, CString};

#[test]
fn an_exec_with_its_own_environment_keeps_the_variables_of_podvm() {
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe {
        std::env::set_var("PODVM_T1756", "kept");
        std::env::set_var("PODVM_GUEST_EXE", "/bin/old");
        std::env::set_var("LD_PRELOAD", "/.podvm/interpose.so");
    }
    let path = CString::new("PATH=/usr/bin:/bin").unwrap();
    let mine = CString::new("PODVM_T1756=caller").unwrap();
    let envp = [path.as_ptr(), core::ptr::null()];
    let mut out = [core::ptr::null(); crate::exec::launch::CARRY];
    let n = unsafe { crate::exec::launch::missing(envp.as_ptr(), &mut out) };
    let names: Vec<String> = out[..n]
        .iter()
        .map(|p| unsafe { CStr::from_ptr(*p) }.to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"PODVM_T1756=kept".to_string()), "{names:?}");
    assert!(
        names.contains(&"LD_PRELOAD=/.podvm/interpose.so".to_string()),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|e| e.starts_with("PODVM_GUEST_EXE=")),
        "the exe of one program is made fresh, never carried"
    );
    // A name that the caller sets keeps the caller's value.
    let envp = [path.as_ptr(), mine.as_ptr(), core::ptr::null()];
    let n = unsafe { crate::exec::launch::missing(envp.as_ptr(), &mut out) };
    assert!(
        !out[..n].iter().any(|p| unsafe { CStr::from_ptr(*p) }
            .to_bytes()
            .starts_with(b"PODVM_T1756=")),
        "the caller's own value wins"
    );
    unsafe {
        std::env::remove_var("PODVM_T1756");
        std::env::remove_var("PODVM_GUEST_EXE");
        std::env::remove_var("LD_PRELOAD");
    }
}

#[test]
fn the_copy_of_the_start_is_taken_before_main() {
    // The constructor ran when the test binary loaded.
    assert!(!crate::snapshot::env().is_null());
    assert!(crate::snapshot::ours(b"PODVM_MAPS") && crate::snapshot::ours(b"LD_PRELOAD"));
    assert!(!crate::snapshot::ours(b"PATH") && !crate::snapshot::ours(b"LD_PRELOADX"));
}
