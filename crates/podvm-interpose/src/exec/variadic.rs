//! `execl`, `execlp` and `execle`, TODO/interpose.md T-1754.
//!
//! glibc builds the vector of these variadic calls and calls its internal
//! `execve`, past this object, so an absolute path of the image ran the
//! host's program. Stable Rust cannot define a C-variadic function
//! (rust-lang/rust#44930), so each takes 32 fixed slots after the path, as
//! `open_fixed!` takes the mode: on x86_64 SysV and on aarch64 Linux a
//! variadic pointer argument travels as a fixed one does (a register, then
//! the stack). A slot past the caller's last argument reads the caller's
//! own stack frame and is never used: the vector ends at the first null.
//! A call with no null in the slots (32 arguments or more) fails with
//! E2BIG, never unmapped.

use crate::set_errno;
use core::ffi::{c_char, c_int};

/// The slots of a call.
pub const SLOTS: usize = 32;
const E2BIG: c_int = 7;

/// The vector up to and with the first null, and the slot after it.
fn split(slots: &[*const c_char; SLOTS]) -> Option<(usize, *const c_char)> {
    let end = slots.iter().position(|p| p.is_null())?;
    Some((
        end,
        slots.get(end + 1).copied().unwrap_or(core::ptr::null()),
    ))
}

/// # Safety
/// The payload's own contract for `execl(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execl(
    path: *const c_char,
    a0: *const c_char,
    a1: *const c_char,
    a2: *const c_char,
    a3: *const c_char,
    a4: *const c_char,
    a5: *const c_char,
    a6: *const c_char,
    a7: *const c_char,
    a8: *const c_char,
    a9: *const c_char,
    a10: *const c_char,
    a11: *const c_char,
    a12: *const c_char,
    a13: *const c_char,
    a14: *const c_char,
    a15: *const c_char,
    a16: *const c_char,
    a17: *const c_char,
    a18: *const c_char,
    a19: *const c_char,
    a20: *const c_char,
    a21: *const c_char,
    a22: *const c_char,
    a23: *const c_char,
    a24: *const c_char,
    a25: *const c_char,
    a26: *const c_char,
    a27: *const c_char,
    a28: *const c_char,
    a29: *const c_char,
    a30: *const c_char,
    a31: *const c_char,
) -> c_int {
    let slots = [
        a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19,
        a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31,
    ];
    let Some((end, _)) = split(&slots) else {
        set_errno(E2BIG);
        return -1;
    };
    let argv = &slots[..=end];
    unsafe { crate::execv(path, argv.as_ptr()) }
}

/// # Safety
/// The payload's own contract for `execlp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execlp(
    file: *const c_char,
    a0: *const c_char,
    a1: *const c_char,
    a2: *const c_char,
    a3: *const c_char,
    a4: *const c_char,
    a5: *const c_char,
    a6: *const c_char,
    a7: *const c_char,
    a8: *const c_char,
    a9: *const c_char,
    a10: *const c_char,
    a11: *const c_char,
    a12: *const c_char,
    a13: *const c_char,
    a14: *const c_char,
    a15: *const c_char,
    a16: *const c_char,
    a17: *const c_char,
    a18: *const c_char,
    a19: *const c_char,
    a20: *const c_char,
    a21: *const c_char,
    a22: *const c_char,
    a23: *const c_char,
    a24: *const c_char,
    a25: *const c_char,
    a26: *const c_char,
    a27: *const c_char,
    a28: *const c_char,
    a29: *const c_char,
    a30: *const c_char,
    a31: *const c_char,
) -> c_int {
    let slots = [
        a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19,
        a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31,
    ];
    let Some((end, _)) = split(&slots) else {
        set_errno(E2BIG);
        return -1;
    };
    let argv = &slots[..=end];
    unsafe { crate::execvp(file, argv.as_ptr()) }
}

/// # Safety
/// The payload's own contract for `execle(3)`: the environment follows the
/// null that ends the arguments.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execle(
    path: *const c_char,
    a0: *const c_char,
    a1: *const c_char,
    a2: *const c_char,
    a3: *const c_char,
    a4: *const c_char,
    a5: *const c_char,
    a6: *const c_char,
    a7: *const c_char,
    a8: *const c_char,
    a9: *const c_char,
    a10: *const c_char,
    a11: *const c_char,
    a12: *const c_char,
    a13: *const c_char,
    a14: *const c_char,
    a15: *const c_char,
    a16: *const c_char,
    a17: *const c_char,
    a18: *const c_char,
    a19: *const c_char,
    a20: *const c_char,
    a21: *const c_char,
    a22: *const c_char,
    a23: *const c_char,
    a24: *const c_char,
    a25: *const c_char,
    a26: *const c_char,
    a27: *const c_char,
    a28: *const c_char,
    a29: *const c_char,
    a30: *const c_char,
    a31: *const c_char,
) -> c_int {
    let slots = [
        a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19,
        a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31,
    ];
    let Some((end, envp)) = split(&slots) else {
        set_errno(E2BIG);
        return -1;
    };
    if end + 1 >= SLOTS {
        set_errno(E2BIG);
        return -1;
    }
    let argv = &slots[..=end];
    unsafe { crate::execve(path, argv.as_ptr(), envp as *const *const c_char) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vector_ends_at_the_first_null() {
        let a = c"a".as_ptr();
        let mut s = [core::ptr::null::<c_char>(); SLOTS];
        s[0] = a;
        s[1] = a;
        assert_eq!(split(&s).map(|x| x.0), Some(2));
        let full = [a; SLOTS];
        assert!(split(&full).is_none(), "no null: refused, never forwarded");
    }
}
