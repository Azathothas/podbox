//! `dlopen` and `dlmopen`, T-1645.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-1645. The payload's
//! loader opens a guest object by its guest path, and on the userland rung
//! that path names nothing on the host. Each entry point here routes a path
//! argument through [`crate::map::prepare`], as `open` does, and then gives
//! the call to the next definition.
//!
//! ⭐ **A path is a name that holds a `/`.** A bare soname is a search, not a
//! path: the loader searches its own list, and the loader family points that
//! list into the rootfs. A name that holds a `$` is not changed either: the
//! loader expands `$ORIGIN`, `$LIB` and `$PLATFORM` against the object that
//! called it, and only the loader knows that object.
//!
//! ⛔ **The next definition sees the payload's return address, not ours.**
//! glibc's `dlopen` and `dlmopen` read their own return address to find the
//! calling object. The loader uses that object for the `DT_RPATH` and
//! `DT_RUNPATH` search of a bare soname, for `$ORIGIN`, and for the
//! namespace of the new object. A wrapper that called the next definition
//! would make this object the caller of every load, and a library that finds
//! its plugins through its own `DT_RUNPATH` would stop to find them, also
//! with an empty table. Thus each entry point is a short assembly trampoline:
//! it calls the Rust route, then JUMPS to the next definition with the stack
//! as the payload left it.
//!
//! ⚠ A jump frees nothing on the stack of this object, so the rewritten name
//! cannot live there. It lives in one buffer for each thread. The loader
//! copies the name before it runs a constructor, so a `dlopen` that a
//! constructor starts on the same thread can use the buffer again. A loader
//! error message after the constructors can name the later path.

use super::*;
use core::cell::UnsafeCell;
use core::ffi::c_long;

crate::real!(pub fn next_dlopen = "dlopen"(*const c_char, c_int) -> *mut c_void);
crate::real!(pub fn next_dlmopen = "dlmopen"(c_long, *const c_char, c_int) -> *mut c_void);

/// What the trampoline does next: jump to `next` with `path` in the path
/// register, or answer null where `next` is null. Two pointers, so the C
/// ABI returns them in `rax` and `rdx`.
#[repr(C)]
pub struct Route {
    pub path: *const c_char,
    pub next: *const c_void,
}

thread_local! {
    /// The rewritten name. One for each thread, because the jump leaves
    /// the stack of this object before the loader reads the name. A const
    /// initializer and no destructor: no allocation and no registration.
    static NAME: UnsafeCell<[u8; crate::map::OUT]> =
        const { UnsafeCell::new([0u8; crate::map::OUT]) };
}

/// Is `name` a path that the table can apply to: a `/` in it, no `$`?
///
/// # Safety
/// `name` must be null or NUL-terminated.
pub unsafe fn is_mapped_name(name: *const c_char) -> bool {
    if name.is_null() {
        return false;
    }
    let n = crate::map::strnlen(name, crate::map::OUT);
    let bytes = unsafe { core::slice::from_raw_parts(name as *const u8, n) };
    bytes.contains(&b'/') && !bytes.contains(&b'$')
}

/// The route for one call: the name to pass, and where to jump.
///
/// # Safety
/// `name` must be null or NUL-terminated within `OUT` bytes.
unsafe fn route(what: &[u8], name: *const c_char, next: *const c_void) -> Route {
    if next.is_null() || !unsafe { is_mapped_name(name) } {
        return Route { path: name, next };
    }
    let buf = NAME.with(|b| b.get());
    // SAFETY: the buffer belongs to this thread, and nothing else on this
    // thread holds a reference to it while `prepare` writes it.
    let p = unsafe { crate::map::prepare(what, crate::map::AT_FDCWD, name, &mut *buf) };
    if p.is_null() {
        // `prepare` set errno and said one line. The load does not happen,
        // and it never falls back to the unmapped name.
        return Route {
            path: core::ptr::null(),
            next: core::ptr::null(),
        };
    }
    Route { path: p, next }
}

/// # Safety
/// Called from the `dlopen` trampoline only, with the payload's argument.
unsafe extern "C" fn route_dlopen(name: *const c_char) -> Route {
    let next = next_dlopen().map_or(core::ptr::null(), |f| f as *const c_void);
    unsafe { route(b"dlopen", name, next) }
}

/// # Safety
/// Called from the `dlmopen` trampoline only, with the payload's argument.
unsafe extern "C" fn route_dlmopen(name: *const c_char) -> Route {
    let next = next_dlmopen().map_or(core::ptr::null(), |f| f as *const c_void);
    unsafe { route(b"dlmopen", name, next) }
}

// `void *dlopen(const char *name, int mode)`: `rdi` name, `esi` mode.
// The push keeps `mode` and aligns the stack to 16 bytes for the call.
// Intel syntax: this is Rust's assembler default. The label `2` is not
// `0` or `1`, which LLVM can read as a binary number in Intel syntax.
core::arch::global_asm!(
    ".text",
    ".globl dlopen",
    ".type dlopen, @function",
    "dlopen:",
    "push rsi",
    "call {route}",
    "pop rsi",
    "test rdx, rdx",
    "jz 2f",
    "mov rdi, rax",
    "jmp rdx",
    "2:",
    "xor eax, eax",
    "ret",
    ".size dlopen, .-dlopen",
    route = sym route_dlopen,
);

// `void *dlmopen(Lmid_t lmid, const char *name, int mode)`: `rdi` lmid,
// `rsi` name, `edx` mode. The namespace argument goes to the next
// definition unchanged. Two pushes and 8 bytes align the stack for the
// call; `r11` holds the target, because the pop of `mode` takes `rdx`.
core::arch::global_asm!(
    ".text",
    ".globl dlmopen",
    ".type dlmopen, @function",
    "dlmopen:",
    "push rdi",
    "push rdx",
    "sub rsp, 8",
    "mov rdi, rsi",
    "call {route}",
    "add rsp, 8",
    "mov r11, rdx",
    "mov rsi, rax",
    "pop rdx",
    "pop rdi",
    "test r11, r11",
    "jz 2f",
    "jmp r11",
    "2:",
    "xor eax, eax",
    "ret",
    ".size dlmopen, .-dlmopen",
    route = sym route_dlmopen,
);
