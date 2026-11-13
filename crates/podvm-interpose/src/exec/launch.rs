//! The argument and environment vectors of a routed exec, T-1646.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1646. A routed exec
//! gives the kernel new vectors: the guest loader and its options go before
//! the program, and `PODVM_GUEST_EXE` names the new program. The pointers
//! live in a stack array; a vector that does not fit gets an anonymous
//! mapping instead, because the object must not call `malloc`.
//!
//! ⚠ After a `vfork`, the child shares the memory of the parent. A mapping
//! that a successful exec leaves behind stays in the parent. Only a vector
//! longer than [`STACK_SLOTS`] takes a mapping, so the cost is bounded to
//! such calls.

use super::*;
use core::ffi::c_void;

/// How many pointers the stack array holds.
pub const STACK_SLOTS: usize = 256;

/// How many entries one vector may have before the exec answers `E2BIG`.
/// The kernel refuses well before this: its byte limit is a quarter of the
/// stack limit.
const MAX_ENTRIES: usize = 1 << 21;

const E2BIG: c_int = 7;
const ENOMEM: c_int = 12;
const ENAMETOOLONG: c_int = 36;

/// The variable that carries the image's library directories by host path.
/// A duplicate by value of `podvm_enter::userland::LIB_PATH_VAR`, as
/// `map::MAPS_VAR` duplicates its own spelling: this object takes no
/// dependency on the rest of the tree.
pub const LIB_PATH_VAR: &[u8] = b"PODVM_LIB_PATH";

/// `LD_LIBRARY_PATH`, the variable the payload can change.
const LD_PATH_VAR: &[u8] = b"LD_LIBRARY_PATH";

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

/// The pointer storage of one launch: the caller's stack array, or a
/// mapping that `Drop` returns.
pub struct Slots {
    base: *mut *const c_char,
    cap: usize,
    pub n: usize,
    mapped: usize,
}

impl Slots {
    /// Storage for `need` pointers.
    ///
    /// # Safety
    /// `stack` must outlive the returned value and each use of its pointers.
    pub unsafe fn new(
        stack: &mut [*const c_char; STACK_SLOTS],
        need: usize,
    ) -> Result<Slots, c_int> {
        if need <= STACK_SLOTS {
            return Ok(Slots {
                base: stack.as_mut_ptr(),
                cap: STACK_SLOTS,
                n: 0,
                mapped: 0,
            });
        }
        let len = need
            .checked_mul(core::mem::size_of::<*const c_char>())
            .ok_or(E2BIG)?;
        // PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS.
        let p = unsafe { mmap(core::ptr::null_mut(), len, 3, 0x22, -1, 0) };
        if p as isize == -1 {
            return Err(ENOMEM);
        }
        Ok(Slots {
            base: p as *mut *const c_char,
            cap: need,
            n: 0,
            mapped: len,
        })
    }

    /// Add one pointer. The caller counted the need, so a full store is a
    /// defect, and the pointer is dropped rather than written past the end.
    pub fn push(&mut self, p: *const c_char) {
        debug_assert!(self.n < self.cap, "the launch counted too few slots");
        if self.n < self.cap {
            unsafe { *self.base.add(self.n) = p };
            self.n += 1;
        }
    }

    /// The pointer at slot `i`.
    pub fn at(&self, i: usize) -> *const *const c_char {
        unsafe { self.base.add(i) as *const *const c_char }
    }
}

impl Drop for Slots {
    fn drop(&mut self) {
        if self.mapped > 0 {
            unsafe { munmap(self.base as *mut c_void, self.mapped) };
        }
    }
}

/// The number of entries before the null of a vector. A null vector has
/// none, as Linux reads it.
///
/// # Safety
/// `v` must be null or a null-terminated pointer array.
pub unsafe fn count(v: *const *const c_char) -> Result<usize, c_int> {
    if v.is_null() {
        return Ok(0);
    }
    let mut n = 0usize;
    while !unsafe { *v.add(n) }.is_null() {
        n += 1;
        if n > MAX_ENTRIES {
            return Err(E2BIG);
        }
    }
    Ok(n)
}

/// The value of `name` in the vector `envp`, and the index of its entry.
///
/// # Safety
/// `envp` must be null or a null-terminated array of NUL-terminated strings.
pub unsafe fn env_value(envp: *const *const c_char, name: &[u8]) -> Option<(usize, &'static [u8])> {
    if envp.is_null() {
        return None;
    }
    let mut i = 0usize;
    while i < MAX_ENTRIES {
        let e = unsafe { *envp.add(i) };
        if e.is_null() {
            break;
        }
        if let Some(v) = crate::map::match_var(e, name) {
            let n = crate::map::strnlen(v as *const c_char, 65536);
            return Some((i, unsafe { core::slice::from_raw_parts(v, n) }));
        }
        i += 1;
    }
    None
}

/// The most variables of podvm that an exec adds to an `envp`.
pub const CARRY: usize = 24;

/// TODO/interpose.md T-1756: the entries of this process's environment
/// that podvm set and that `envp` lacks by name: each `PODVM_*` but
/// `PODVM_GUEST_EXE` (which names one program), and `LD_PRELOAD`. A program
/// that runs a child with an environment of its own (the triggers of apk,
/// `env -i`) dropped them, and its child ran with no table: `/bin/busybox`
/// of a trigger resolved on the host.
///
/// # Safety
/// `envp` as [`env_value`].
pub unsafe fn missing(envp: *const *const c_char, out: &mut [*const c_char; CARRY]) -> usize {
    let mut n = 0;
    // `environ` first, then the copy of the start: a process that cleared
    // its environment (`env -i`) still carries the table (T-1756).
    for own in [crate::map::environ_ptr(), crate::snapshot::env()] {
        let mut i = 0;
        while !own.is_null() && i < MAX_ENTRIES && n < CARRY {
            let e = unsafe { *own.add(i) };
            if e.is_null() {
                break;
            }
            i += 1;
            let len = crate::map::strnlen(e, OUT);
            let entry = unsafe { core::slice::from_raw_parts(e as *const u8, len) };
            let Some(eq) = entry.iter().position(|&b| b == b'=') else {
                continue;
            };
            let name = &entry[..eq];
            let named =
                |v: &[*const c_char]| v.iter().any(|p| crate::map::match_var(*p, name).is_some());
            if crate::snapshot::ours(name)
                && name != crate::procfs::GUEST_EXE_VAR
                && unsafe { env_value(envp, name) }.is_none()
                && !named(&out[..n])
            {
                out[n] = e;
                n += 1;
            }
        }
    }
    n
}

/// Append `src` to `out[..*n]`, with a `:` before it where `out` is not
/// empty. `ENAMETOOLONG` where it does not fit with the NUL.
fn push_dir(out: &mut [u8; OUT], n: &mut usize, src: &[u8]) -> Result<(), c_int> {
    let sep = (*n > 0) as usize;
    if *n + sep + src.len() >= OUT {
        return Err(ENAMETOOLONG);
    }
    if sep == 1 {
        out[*n] = b':';
    }
    out[*n + sep..*n + sep + src.len()].copy_from_slice(src);
    *n += sep + src.len();
    Ok(())
}

/// The `--library-path` value of a routed exec, with a NUL, and its length.
///
/// The image's library directories come from [`LIB_PATH_VAR`], which
/// `podvm_enter::userland::host_env` sets from the same list it puts on
/// `LD_LIBRARY_PATH`: first the value in the child's vector, then the value
/// in this process. A payload can change `LD_LIBRARY_PATH`, so that value
/// goes first where it differs, each directory brought to host names (a
/// directory under the rootfs stays; a guest directory goes through the
/// table). The image's directories always follow it, so that a glibc payload
/// never falls back to the host's library cache for its `libc.so.6`.
/// Zero is no value: the loader then searches its defaults.
///
/// # Safety
/// `envp` as [`env_value`].
pub unsafe fn lib_path(
    t: &Table,
    envp: *const *const c_char,
    out: &mut [u8; OUT],
) -> Result<usize, c_int> {
    let image = unsafe { env_value(envp, LIB_PATH_VAR) }
        .map(|(_, v)| v)
        .or_else(|| {
            unsafe { crate::map::lookup_env(LIB_PATH_VAR) }
                .map(|(p, n)| unsafe { core::slice::from_raw_parts(p, n) })
        });
    let mut n = 0usize;
    if let Some((_, user)) = unsafe { env_value(envp, LD_PATH_VAR) }
        && Some(user) != image
    {
        let mut host = [0u8; OUT];
        for dir in user.split(|&c| c == b':') {
            if dir.first() != Some(&b'/') {
                continue;
            }
            let under_root =
                unsafe { crate::map::longest_to(t, dir.as_ptr(), dir.len()) }.is_some();
            let r = if under_root {
                -1
            } else {
                unsafe { crate::map::rewrite(t, dir.as_ptr(), dir.len(), &mut host) }
            };
            match r {
                -1 => push_dir(out, &mut n, dir)?,
                r if r < 0 => return Err(ENAMETOOLONG),
                r => push_dir(out, &mut n, &host[..r as usize])?,
            }
        }
    }
    if let Some(img) = image
        && !img.is_empty()
    {
        push_dir(out, &mut n, img)?;
    }
    out[n] = 0;
    Ok(n)
}

/// `PODVM_GUEST_EXE=<guest>` into `out`, with a NUL.
pub fn guest_exe_entry(guest: &[u8], out: &mut [u8; OUT]) -> Result<(), c_int> {
    let var = crate::procfs::GUEST_EXE_VAR;
    if var.len() + 1 + guest.len() >= OUT {
        return Err(ENAMETOOLONG);
    }
    out[..var.len()].copy_from_slice(var);
    out[var.len()] = b'=';
    out[var.len() + 1..var.len() + 1 + guest.len()].copy_from_slice(guest);
    out[var.len() + 1 + guest.len()] = 0;
    Ok(())
}

/// What the head of the last file planned says.
pub enum Tail {
    /// Exec the host path itself: static, foreign, or not ELF.
    Direct,
    /// Exec podvm, whose host path is in `Scratch::loader`, on the
    /// native-static rung: `podvm --podvm-dispatch-enter ROOTFS GUEST
    /// ARGV...`, with the rootfs in `Scratch::lib` (TODO/enter.md T-1724).
    Dispatch,
    /// Exec the guest loader; its name is in `Scratch::lib`.
    Loader,
}

/// Build the vectors of the planned exec.
#[allow(clippy::too_many_arguments)]
pub unsafe fn assemble(
    t: &Table,
    tail: Tail,
    no_argv0: bool,
    inhibit: bool,
    levels: &[(*const c_char, *const c_char)],
    rn: usize,
    file0: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    s: &mut Scratch,
) -> Result<Launch, c_int> {
    let argc = unsafe { count(argv) }?;
    // The kernel's own vector: each `#!` level from the deepest, the
    // caller's name, then the caller's arguments after `argv[0]`.
    let lead = levels
        .iter()
        .map(|l| 1 + !l.1.is_null() as usize)
        .sum::<usize>();
    let full_n = if levels.is_empty() {
        argc.max(1)
    } else {
        lead + 1 + argc.saturating_sub(1)
    };
    let first = match levels.last() {
        Some(l) => l.0,
        None if argc > 0 => unsafe { *argv },
        None => file0,
    };
    let loader = matches!(tail, Tail::Loader);
    let lib_n = if loader {
        unsafe { lib_path(t, envp, &mut s.lib) }?
    } else {
        0
    };
    let applet = loader && no_argv0 && {
        let hn = crate::map::strnlen(s.host.as_ptr() as *const c_char, OUT);
        let fl = crate::map::strnlen(first, OUT);
        let invoked =
            head::basename(unsafe { core::slice::from_raw_parts(first as *const u8, fl) });
        head::basename(&s.host[..hn]) == b"busybox" && !invoked.is_empty() && invoked != b"busybox"
    };
    let dispatch = matches!(tail, Tail::Dispatch);
    let opts = if loader {
        2 + 2 * (lib_n > 0) as usize + 2 * (!no_argv0) as usize + applet as usize + inhibit as usize
    } else if dispatch {
        4
    } else {
        0
    };
    let exe_at = unsafe { env_value(envp, crate::procfs::GUEST_EXE_VAR) }.map(|(i, _)| i);
    // T-1756: podvm's variables that the caller's `envp` dropped, and a
    // fresh `PODVM_GUEST_EXE` where this process has one and `envp` not.
    let mut carry = [core::ptr::null(); CARRY];
    let n_carry = unsafe { missing(envp, &mut carry) };
    let exe_new = exe_at.is_none()
        && [crate::map::environ_ptr(), crate::snapshot::env()]
            .into_iter()
            .any(|e| unsafe { env_value(e, crate::procfs::GUEST_EXE_VAR) }.is_some());
    let rebuild = exe_at.is_some() || exe_new || n_carry > 0;
    let envc = if rebuild { unsafe { count(envp) }? } else { 0 };
    if exe_at.is_some() || exe_new {
        guest_exe_entry(&s.resolved[..rn], &mut s.exe)?;
    }
    let argv_n = opts + full_n + 1;
    let need = argv_n
        + if rebuild {
            envc + n_carry + exe_new as usize + 1
        } else {
            0
        };
    let mut v = unsafe { Slots::new(&mut s.stack, need) }?;
    let host = s.host.as_ptr() as *const c_char;
    if dispatch {
        v.push(s.loader.as_ptr() as *const c_char);
        v.push(c"--podvm-dispatch-enter".as_ptr());
        v.push(s.lib.as_ptr() as *const c_char);
        v.push(s.resolved.as_ptr() as *const c_char);
        v.push(first);
    } else if loader {
        v.push(s.loader.as_ptr() as *const c_char);
        if inhibit {
            v.push(c"--inhibit-cache".as_ptr());
        }
        if lib_n > 0 {
            v.push(c"--library-path".as_ptr());
            v.push(s.lib.as_ptr() as *const c_char);
        }
        if !no_argv0 {
            v.push(c"--argv0".as_ptr());
            v.push(first);
        }
        v.push(host);
        if applet {
            let fl = crate::map::strnlen(first, OUT);
            let base =
                head::basename(unsafe { core::slice::from_raw_parts(first as *const u8, fl) });
            v.push(base.as_ptr() as *const c_char);
        }
    } else {
        v.push(first);
    }
    // The deepest level's argument follows its name, which is `first`;
    // then each shallower level, name and argument.
    if let Some(l) = levels.last()
        && !l.1.is_null()
    {
        v.push(l.1);
    }
    for l in levels.iter().rev().skip(1) {
        v.push(l.0);
        if !l.1.is_null() {
            v.push(l.1);
        }
    }
    if !levels.is_empty() {
        v.push(file0);
    }
    for i in 1..argc {
        v.push(unsafe { *argv.add(i) });
    }
    v.push(core::ptr::null());
    let argv_out = v.at(0);
    let envp_out = if rebuild {
        let start = v.n;
        for i in 0..envc {
            v.push(if Some(i) == exe_at {
                s.exe.as_ptr() as *const c_char
            } else {
                unsafe { *envp.add(i) }
            });
        }
        for c in &carry[..n_carry] {
            v.push(*c);
        }
        if exe_new {
            v.push(s.exe.as_ptr() as *const c_char);
        }
        v.push(core::ptr::null());
        v.at(start)
    } else {
        envp
    };
    Ok(Launch {
        path: if loader || dispatch {
            s.loader.as_ptr() as *const c_char
        } else {
            host
        },
        argv: argv_out,
        envp: envp_out,
        _slots: v,
    })
}
