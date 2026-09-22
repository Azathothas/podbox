//! `dlsym(RTLD_NEXT, ...)`, resolved once per entry point and cached.
//!
//! ⭐ [`TODO/interpose.md`](../../../TODO/interpose.md) T-0701's Decision. The
//! alternative -- resolving on every call -- is a `dlsym` inside every
//! intercepted `open`, and the corpus paid for it twice:
//! `references/fritzw__ld-preload-open`'s tracker carries "Cache the results of
//! dlsym" as pull request #3 and "fix memory corruption" as #4. The shape to
//! copy is `references/VHSgunzo__pathmap/tree/path-mapping.c:603-606`.
//!
//! ⛔ **No allocation and no lock on this path.** The cache is one
//! `AtomicPtr` per entry point, published with `Release` and read with
//! `Acquire`. Two threads racing resolve the same symbol twice and store the
//! same pointer, which costs a `dlsym` and is correct; a lock here could be
//! re-entered by the loader itself, and T-0701 forbids exactly that.
//!
//! ⚠ **`dlsym` and `gettid` are the two imports that decide which payloads
//! this object can serve.** `dlsym` links against the `dl-stub.S` import
//! library, which records the reference under `libdl.so.2`: the library
//! that has defined it on both sides of glibc's 2.34 `libdl` merge.
//! `gettid` is defined locally through the raw syscall number because
//! glibc only grew the wrapper in 2.30. Newer needs than `GLIBC_2.27` fail
//! the ceiling `scripts/build-interpose.sh` asserts. TODO/interpose.md
//! T-1312 is the entry that paid for both: the nix closure ships glibc
//! 2.27, and an object importing newer refuses every binary in it at
//! startup.
//!
//! What `podbox_enter::abi` reads is still the version sets, and
//! `experiments/80-interposer-abi.sh` still asserts the reader agrees with
//! the loader; lowering the needs only moves the line the prediction draws.

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicPtr, Ordering};

// Pin this object's `dlsym` reference to `GLIBC_2.2.5`. The pin alone is
// not enough: the link still names `libc.so.6`, which never defined that
// version there. With the `dl-stub.S` import library first on the link,
// the versioned reference resolves against its versioned definition, and
// the recorded need stays `dlsym@GLIBC_2.2.5` under `libdl.so.2`, which
// defines it on both sides of the merge.
// ⛔ glibc ONLY: musl defines no symbol versions, so a versioned undefined
// reference fails its link. The musl object needs no pin: its loader
// ignores versions entirely.
#[cfg(target_env = "gnu")]
core::arch::global_asm!(".symver dlsym,dlsym@GLIBC_2.2.5");

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

// `gettid(2)` without the libc wrapper, T-1312's second import.
//
// Rust std calls `gettid`, and glibc only provides the wrapper from 2.30,
// so linking the wrapper pins the whole object above every older payload.
// A Rust definition cannot hide: `#[no_mangle]` exports every unmangled
// name no matter what the version script says, which `nm -D` proved, and
// `objcopy --localize-symbol` leaves the dynamic entry in place, which the
// same command proved next. So the definition is bare assembly with no
// binding directive at all: a label is local by default, which satisfies
// std's reference inside the link and never enters the dynamic set
// (explicit `.globl` or `.local` made LLVM remark a binding transition on
// every build). The raw number is the call the wrapper itself is
// (`SYS_gettid` 186, every kernel this runtime stands on).
// `build-interpose.sh` asserts the absence from `.dynsym`, so payloads keep
// their own libc's `gettid` and `105` check A still agrees both ways.
// Uniform across targets: a raw syscall is the same kernel interface under
// musl, and this crate refuses non-x86_64 at compile.
core::arch::global_asm!(
    ".text",
    // No binding directive at all: a bare label is local by default, which
    // satisfies std's reference inside the link and never enters the
    // dynamic set. Explicit `.globl` or `.local` directives made LLVM
    // remark a binding transition on every build; the default has nothing
    // to transition from.
    ".type gettid, @function",
    "gettid:",
    // Intel syntax: this is Rust's assembler default, not the GNU one.
    "mov eax, 186",
    "syscall",
    "ret",
    ".size gettid, .-gettid",
);

/// `RTLD_NEXT`, which is `((void *) -1)` in both glibc's and musl's `dlfcn.h`.
const RTLD_NEXT: *mut c_void = usize::MAX as *mut c_void;

/// One cached pointer to the next definition of a symbol.
pub struct Next(AtomicPtr<c_void>);

impl Default for Next {
    fn default() -> Next {
        Next::new()
    }
}

impl Next {
    pub const fn new() -> Next {
        Next(AtomicPtr::new(core::ptr::null_mut()))
    }

    /// The real function, or null where the payload's libc does not define it.
    ///
    /// # Safety
    /// `name` must be a NUL-terminated C string, and the caller must transmute
    /// the result to the signature the symbol actually has.
    pub unsafe fn get(&self, name: &[u8]) -> *mut c_void {
        let cached = self.0.load(Ordering::Acquire);
        if !cached.is_null() {
            return cached;
        }
        debug_assert!(
            name.last() == Some(&0),
            "a dlsym name must be NUL-terminated"
        );
        let p = unsafe { dlsym(RTLD_NEXT, name.as_ptr() as *const c_char) };
        // ⚠ Stored even when null is not possible to distinguish from "not
        // resolved yet": a null is NOT cached, so a symbol the loader could not
        // find is retried rather than remembered as absent. That costs a dlsym
        // per call on a payload whose libc lacks the symbol, which is the case
        // podbox refuses before loading anyway.
        if !p.is_null() {
            self.0.store(p, Ordering::Release);
        }
        p
    }

    /// The default-version definition of a symbol that also carries a
    /// compatibility version, T-1309.
    ///
    /// Several forwarded names carry a compat beside the default
    /// (`realpath@GLIBC_2.2.5` beside `realpath@@GLIBC_2.3`), and an
    /// unversioned `dlsym` can return the compat: measured in-process on
    /// the rocky 9 libc, where it answers the same pointer `dlvsym`
    /// answers for 2.2.5. The compat answers `EINVAL` where the caller
    /// passes NULL, so every `realpath(path, NULL)` fails once
    /// interposed. Resolving the default version by name forwards what
    /// the payload would have bound bare.
    ///
    /// Same cache discipline as [`Next::get`]: a null is retried, a pointer
    /// is published once. Where the payload's libc predates the default,
    /// `dlvsym` finds nothing and the lookup falls back to `dlsym`, which
    /// keeps today's answer there instead of failing every call.
    ///
    /// `dlvsym` itself is resolved at runtime through the already-pinned
    /// `dlsym`: a link-time `dlvsym` reference stamps the 2.34 version,
    /// which breaks the 2.27 ceiling the build asserts. The looked-up
    /// `dlvsym` is one body under both of its versions, so whichever it
    /// answers behaves the same.
    ///
    /// # Safety
    /// As [`Next::get`], for both `name` and `version`.
    pub unsafe fn get_versioned(&self, name: &[u8], version: &[u8]) -> *mut c_void {
        #[cfg(not(target_env = "gnu"))]
        {
            // musl defines no symbol versions and no `dlvsym`: one version
            // per name exists there, so plain `dlsym` already answers it.
            let _ = version;
            return unsafe { self.get(name) };
        }
        #[cfg(target_env = "gnu")]
        {
            let cached = self.0.load(Ordering::Acquire);
            if !cached.is_null() {
                return cached;
            }
            debug_assert!(
                name.last() == Some(&0),
                "a dlsym name must be NUL-terminated"
            );
            debug_assert!(
                version.last() == Some(&0),
                "a dlvsym version must be NUL-terminated"
            );
            type DlvsymFn =
                unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_void;
            let q = unsafe { dlsym(RTLD_NEXT, c"dlvsym".as_ptr()) };
            let mut p = if q.is_null() {
                core::ptr::null_mut()
            } else {
                let dlvsym: DlvsymFn = unsafe { core::mem::transmute(q) };
                unsafe {
                    dlvsym(
                        RTLD_NEXT,
                        name.as_ptr() as *const c_char,
                        version.as_ptr() as *const c_char,
                    )
                }
            };
            if p.is_null() {
                p = unsafe { dlsym(RTLD_NEXT, name.as_ptr() as *const c_char) };
            }
            if !p.is_null() {
                self.0.store(p, Ordering::Release);
            }
            p
        }
    }
}

/// Declare one interposed entry point.
///
/// ⛔ A macro rather than a copied body, which is
/// `references/VHSgunzo__pathmap/tree/path-mapping.c:250-262`'s own decision:
/// a new entry point is one line, and the forwarding cannot be got subtly wrong
/// in the twentieth copy.
#[macro_export]
macro_rules! real {
    ($vis:vis fn $binding:ident = $sym:literal ( $($arg:ty),* $(,)? ) -> $ret:ty) => {
        /// The payload's own definition, resolved on first use.
        ///
        /// ⚠ The BINDING and the SYMBOL are named separately, because the
        /// exported entry point in `lib.rs` has the symbol's own name: one
        /// identifier for both would be podbox resolving `chown` to itself.
        $vis fn $binding() -> Option<unsafe extern "C" fn($($arg),*) -> $ret> {
            static NEXT: $crate::real::Next = $crate::real::Next::new();
            let p = unsafe { NEXT.get(concat!($sym, "\0").as_bytes()) };
            if p.is_null() {
                None
            } else {
                // ⚠ The transmute is the whole point of this module and is why
                // every use of it names the signature at the call site.
                Some(unsafe { core::mem::transmute::<
                    *mut core::ffi::c_void,
                    unsafe extern "C" fn($($arg),*) -> $ret,
                >(p) })
            }
        }
    };
    // T-1309: the same binding pinned to the symbol's default version.
    // Several forwarded names carry a compatibility version beside the
    // default, and an unversioned `dlsym` can return the compat instead.
    // On musl the version is ignored: one version per name exists there,
    // so plain `dlsym` already answers it.
    ($vis:vis fn $binding:ident = $sym:literal @ $ver:literal ( $($arg:ty),* $(,)? ) -> $ret:ty) => {
        /// The payload's own definition, resolved on first use.
        ///
        /// ⚠ The BINDING and the SYMBOL are named separately, because the
        /// exported entry point in `lib.rs` has the symbol's own name: one
        /// identifier for both would be podbox resolving `chown` to itself.
        $vis fn $binding() -> Option<unsafe extern "C" fn($($arg),*) -> $ret> {
            static NEXT: $crate::real::Next = $crate::real::Next::new();
            let p = unsafe {
                NEXT.get_versioned(
                    concat!($sym, "\0").as_bytes(),
                    concat!($ver, "\0").as_bytes(),
                )
            };
            if p.is_null() {
                None
            } else {
                // ⚠ The transmute is the whole point of this module and is why
                // every use of it names the signature at the call site.
                Some(unsafe { core::mem::transmute::<
                    *mut core::ffi::c_void,
                    unsafe extern "C" fn($($arg),*) -> $ret,
                >(p) })
            }
        }
    };
}
