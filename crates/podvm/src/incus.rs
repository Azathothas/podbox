//! The incus back end, TODO/machine.md T-1718, in the form of
//! `docs/incus.md`: podvm drives an incusd over its REST API on a unix
//! socket. `--podvm-tier=incus` selects it for `run`.

pub(crate) mod image;
pub(crate) mod run;

#[cfg(test)]
mod tests;

/// `podvm guest-hold`: the init of a podvm instance. As PID 1 it reaps each
/// orphan, and it ends on SIGTERM, SIGINT or SIGPWR (incus stops an
/// instance with SIGPWR); a PID 1 with no handler would ignore each.
pub fn hold() -> i32 {
    unsafe extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        fn pause() -> i32;
        fn _exit(code: i32) -> !;
    }
    extern "C" fn end(_: i32) {
        // SAFETY: _exit is async-signal-safe.
        unsafe { _exit(0) }
    }
    // SAFETY: plain system calls; the handler only exits.
    unsafe {
        for sig in [2, 15, 30] {
            signal(sig, end);
        }
        loop {
            if waitpid(-1, std::ptr::null_mut(), 0) < 0 {
                pause();
            }
        }
    }
}
