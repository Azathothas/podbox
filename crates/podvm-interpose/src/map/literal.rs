//! The rewrite of a literal path, with no `dirfd` and no walk.

use super::*;

/// Match and rewrite a path that is already absolute-or-literal, without any
/// `dirfd` resolution. `symlink`'s target takes this shape: the content is
/// resolved when the link is used, not when it is made, so resolving it now
/// would answer about the wrong directory.
///
/// # Safety
/// As [`absolutize`]: a readable path and a writable `OUT`-byte buffer.
pub unsafe fn prepare_literal(
    what: &[u8],
    path: *const c_char,
    out: &mut [u8; OUT],
) -> *const c_char {
    if path.is_null() {
        return path;
    }
    let t = unsafe { table() };
    if t.n == 0 {
        return path;
    }
    let n = strnlen(path, OUT);
    if n >= OUT {
        return path;
    }
    match unsafe { rewrite(&t, path as *const u8, n, out) } {
        -1 => path,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit, so the call fails"]);
            crate::set_errno(ENAMETOOLONG);
            core::ptr::null()
        }
        n => {
            out[n as usize] = 0;
            out.as_ptr() as *const c_char
        }
    }
}
