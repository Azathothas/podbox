//! The exec family enters a guest program through the guest loader, T-1646.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-1646. On the userland
//! rung the table maps guest `/` into the rootfs, but the kernel reads the
//! `PT_INTERP` of a mapped ELF file, and the `#!` line of a mapped script, on
//! the HOST root. So a dynamic grandchild ran under the host loader or failed
//! 127. Thus each entry point here reads the head of the file first:
//!
//! * an ELF file with `PT_INTERP`: the exec runs the guest loader, resolved
//!   inside the rootfs, with `--library-path` and the program by host path.
//!   `--argv0` keeps the caller's `argv[0]`, so a busybox applet keeps its
//!   name; a glibc loader older than 2.33 has no `--argv0`, and there the
//!   applet name goes after the program, as the top-level drive does it
//!   (`podvm_enter::userland::loader_argv_for`);
//! * a `#!` script: its interpreter is resolved inside the rootfs the same
//!   way, and the arguments are the kernel's (`interp [arg] script args`);
//! * a static ELF file: the direct exec of its host path, as before.
//!
//! ⛔ A child enters through the same loader as its parent: the host loader
//! never runs a guest binary. An interpreter that does not resolve inside
//! the rootfs fails the exec with `ENOENT` and one line, never a host
//! fallback. The route applies only where the table anchors guest `/` (the
//! userland rung, T-1407): with no table, or a table without that pair,
//! each entry point keeps the earlier behaviour exactly.
//!
//! ⛔ No allocation and no lock: each buffer is in [`Scratch`] on the caller's
//! stack, except the pointer vectors of a long command line ([`launch`]).

use core::ffi::{c_char, c_int};

use crate::map::{Table, AT_FDCWD, OUT};

mod descriptor;
pub(crate) use descriptor::fd_path;
pub mod head;
mod launch;
pub(crate) mod lookup;
mod shell;
mod walk;

pub(crate) mod calls;

use head::HEAD;
use launch::{Slots, Tail, STACK_SLOTS};

const ENOENT: c_int = 2;
const ENOEXEC: c_int = 8;
const EACCES: c_int = 13;
const ENOTDIR: c_int = 20;
const EINVAL: c_int = 22;
const ENAMETOOLONG: c_int = 36;
const ELOOP: c_int = 40;

const X_OK: c_int = 1;
const O_RDONLY_CLOEXEC: c_int = 0o2_000_000;

/// How many `#!` levels one exec follows. Linux stops at the same depth.
const DEPTH: usize = 4;

/// The search list where `PATH` is absent: musl's default, which holds
/// glibc's.
const DEFAULT_PATH: &[u8] = b"/usr/local/bin:/bin:/usr/bin";

/// Every buffer one exec uses, on the caller's stack.
pub struct Scratch {
    /// The guest path that the plan resolves next.
    guest: [u8; OUT],
    /// The resolved guest path of the program, for `PODVM_GUEST_EXE`.
    resolved: [u8; OUT],
    /// The host path of the program.
    host: [u8; OUT],
    /// The host path of the guest loader.
    loader: [u8; OUT],
    /// The program headers, then the loader name, then `--library-path`.
    lib: [u8; OUT],
    /// The new `PODVM_GUEST_EXE` entry.
    exe: [u8; OUT],
    /// The head of each file the plan reads; a `#!` line keeps its NULs.
    heads: [[u8; HEAD]; DEPTH + 1],
    stack: [*const c_char; STACK_SLOTS],
}

impl Scratch {
    pub const fn new() -> Scratch {
        Scratch {
            guest: [0; OUT],
            resolved: [0; OUT],
            host: [0; OUT],
            loader: [0; OUT],
            lib: [0; OUT],
            exe: [0; OUT],
            heads: [[0; HEAD]; DEPTH + 1],
            stack: [core::ptr::null(); STACK_SLOTS],
        }
    }
}

/// The new exec: the file the kernel opens, and the vectors it gets. The
/// pointers point into the [`Scratch`] and the caller's own memory.
pub struct Launch {
    pub path: *const c_char,
    pub argv: *const *const c_char,
    pub envp: *const *const c_char,
    _slots: Slots,
}

/// What one entry point does.
pub enum Route {
    /// The earlier behaviour: the real call, with this path.
    Pass(*const c_char),
    /// The call does not happen; `errno` is set.
    Fail,
    /// The real `execve` (or `posix_spawn`) with this launch.
    Launch(Launch),
}

/// Does the table anchor guest `/`? Only then is the host root not the
/// guest root, and only then does the route change an exec.
pub(crate) fn anchored(t: &Table) -> bool {
    t.pairs[..t.n]
        .iter()
        .any(|p| p.from_n == 1 && unsafe { *p.from_p } == b'/')
}

/// The route for one exec. `search` is the `p` variants: a name with no `/`
/// is found on `PATH`, in guest names.
///
/// # Safety
/// `path`, `argv` and `envp` as the exec's own contract; `buf` is the
/// caller's `OUT` buffer, which a `Pass` and a search name live in.
#[allow(clippy::too_many_arguments)]
pub unsafe fn route(
    what: &[u8],
    dirfd: c_int,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    search: bool,
    s: &mut Scratch,
    buf: &mut [u8; OUT],
) -> Route {
    if path.is_null() {
        return Route::Pass(path);
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return Route::Pass(path);
    }
    if !anchored(&t) {
        let p = unsafe { crate::map::prepare(what, dirfd, path, buf) };
        return if p.is_null() {
            Route::Fail
        } else {
            Route::Pass(p)
        };
    }
    let n = crate::map::strnlen(path, OUT);
    if n >= OUT {
        return Route::Pass(path);
    }
    if n == 0 {
        crate::set_errno(ENOENT);
        return Route::Fail;
    }
    let name = unsafe { core::slice::from_raw_parts(path as *const u8, n) };
    let planned = if search && !name.contains(&b'/') {
        unsafe { search_path(what, &t, name, argv, envp, s, buf) }
    } else {
        let gn = match unsafe { exe_name(&t, dirfd, path, &mut s.guest) } {
            Some(gn) => gn,
            None => {
                crate::say::line(&[what, b": podvm cannot resolve the path, so the call fails"]);
                crate::set_errno(ENOENT);
                return Route::Fail;
            }
        };
        unsafe { plan(what, &t, gn, path, argv, envp, s, false) }
    };
    match planned {
        Ok(l) => Route::Launch(l),
        Err(e) => {
            crate::set_errno(e);
            Route::Fail
        }
    }
}

/// The guest name of the program into `out`. `/proc/self/exe` names the
/// payload's own guest path where the loader-family override is on
/// (T-1409): the kernel's answer there is the loader.
unsafe fn exe_name(
    t: &Table,
    dirfd: c_int,
    path: *const c_char,
    out: &mut [u8; OUT],
) -> Option<usize> {
    if unsafe { crate::procfs::exe_override_hit(path) }
        && let Some(n) = unsafe { crate::path_calls::proc_exe_value(out) }
    {
        return Some(n);
    }
    let n = unsafe { crate::map::absolutize(t, dirfd, path, out) };
    (n > 0).then_some(n as usize)
}

/// Find `name` on `PATH` in guest names, as `execvp` does: the first entry
/// that plans wins; an absent, non-directory or looping entry is skipped;
/// `EACCES` is kept and answered where nothing else is found.
#[allow(clippy::too_many_arguments)]
unsafe fn search_path(
    what: &[u8],
    t: &Table,
    name: &[u8],
    argv: *const *const c_char,
    envp: *const *const c_char,
    s: &mut Scratch,
    buf: &mut [u8; OUT],
) -> Result<Launch, c_int> {
    let list = match unsafe { crate::map::lookup_env(b"PATH") } {
        Some((p, n)) => unsafe { core::slice::from_raw_parts(p, n) },
        None => DEFAULT_PATH,
    };
    let mut denied = false;
    for dir in list.split(|&c| c == b':') {
        let dir: &[u8] = if dir.is_empty() { b"." } else { dir };
        if dir.len() + 1 + name.len() >= OUT {
            continue;
        }
        buf[..dir.len()].copy_from_slice(dir);
        buf[dir.len()] = b'/';
        buf[dir.len() + 1..dir.len() + 1 + name.len()].copy_from_slice(name);
        buf[dir.len() + 1 + name.len()] = 0;
        let gn = unsafe {
            crate::map::absolutize(t, AT_FDCWD, buf.as_ptr() as *const c_char, &mut s.guest)
        };
        if gn <= 0 {
            continue;
        }
        match unsafe {
            plan(
                what,
                t,
                gn as usize,
                buf.as_ptr() as *const c_char,
                argv,
                envp,
                s,
                false,
            )
        } {
            Ok(l) => return Ok(l),
            Err(EACCES) => denied = true,
            Err(ENOENT | ENOTDIR | ELOOP | ENAMETOOLONG) => {}
            Err(e) => return Err(e),
        }
    }
    Err(if denied { EACCES } else { ENOENT })
}

/// Plan the exec of the guest path in `s.guest[..gn]`. `file0` is the name
/// the caller gave, which a script receives as its own path. With `pre`,
/// the first level is already resolved: `s.resolved` and `s.host` hold
/// the same NUL-terminated name, a `/proc/self/fd/N` of a descriptor that
/// has no guest name (T-1703).
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn plan(
    what: &[u8],
    t: &Table,
    mut gn: usize,
    file0: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    s: &mut Scratch,
    pre: bool,
) -> Result<Launch, c_int> {
    let access = crate::next_access().ok_or(EINVAL)?;
    // Each `#!` level: its interpreter name and its argument (or null).
    let mut levels = [(core::ptr::null::<c_char>(), core::ptr::null::<c_char>()); DEPTH];
    let mut depth = 0usize;
    let (rn, tail) = loop {
        let rn = if pre && depth == 0 {
            crate::map::strnlen(s.resolved.as_ptr() as *const c_char, OUT)
        } else {
            unsafe { walk::resolve(t, &s.guest[..gn], &mut s.resolved, &mut s.host) }?.0
        };
        let host = s.host.as_ptr() as *const c_char;
        if unsafe { access(host, X_OK) } != 0 {
            return Err(crate::errno());
        }
        let head = &mut s.heads[depth];
        let file = match unsafe { head::read_head(host, head, &mut s.lib) } {
            Ok(f) => f,
            // Execute without read: the kernel can run what this cannot read.
            Err(EACCES) => break (rn, Tail::Direct),
            Err(e) => return Err(e),
        };
        match file {
            head::File::Script(got) => {
                if depth == DEPTH {
                    return Err(ELOOP);
                }
                let sb = head::shebang(&head[..got], got < HEAD - 1).ok_or(ENOEXEC)?;
                head[sb.interp.1] = 0;
                let arg = match sb.arg {
                    Some((a, b)) => {
                        head[b] = 0;
                        unsafe { head.as_ptr().add(a) as *const c_char }
                    }
                    None => core::ptr::null(),
                };
                let interp = unsafe { head.as_ptr().add(sb.interp.0) as *const c_char };
                levels[depth] = (interp, arg);
                depth += 1;
                let n = unsafe { crate::map::absolutize(t, AT_FDCWD, interp, &mut s.guest) };
                gn = usize::try_from(n).ok().filter(|&n| n > 0).ok_or(ENOENT)?;
            }
            head::File::Dynamic => break (rn, Tail::Loader),
            head::File::Direct => break (rn, Tail::Direct),
            head::File::Foreign => {
                // T-1703: the kernel would read the loader of another
                // machine, or its binfmt handler, on the host root.
                crate::say::line(&[
                    what,
                    b": ",
                    &s.resolved[..rn],
                    b" is an ELF file of another machine or class, which the image's loader cannot run, so the exec fails (TODO/interpose.md T-1703)",
                ]);
                return Err(ENOEXEC);
            }
        }
    };
    let mut no_argv0 = false;
    if let Tail::Loader = tail {
        // `s.lib` holds the `PT_INTERP` name. Resolve it inside the rootfs.
        let n = unsafe {
            crate::map::absolutize(t, AT_FDCWD, s.lib.as_ptr() as *const c_char, &mut s.guest)
        };
        let resolved = match usize::try_from(n) {
            Ok(n) if n > 0 => {
                unsafe { walk::resolve(t, &s.guest[..n], &mut s.lib, &mut s.loader) }.ok()
            }
            _ => None,
        };
        let usable = resolved
            .is_some_and(|_| unsafe { access(s.loader.as_ptr() as *const c_char, X_OK) } == 0);
        let Some((ln, _)) = resolved.filter(|_| usable) else {
            let gl = crate::map::strnlen(s.guest.as_ptr() as *const c_char, OUT);
            crate::say::line(&[
                what,
                b": the interpreter ",
                &s.guest[..gl],
                b" of ",
                &s.resolved[..rn],
                b" does not resolve inside the rootfs, so the exec fails (T-1646)",
            ]);
            return Err(ENOENT);
        };
        no_argv0 = head::loader_lacks_argv0(head::basename(&s.lib[..ln]));
        if crate::say::debug() {
            crate::say::line(&[
                what,
                b": ",
                &s.resolved[..rn],
                b" enters through the guest loader ",
                &s.lib[..ln],
            ]);
        }
    }
    unsafe {
        launch::assemble(
            t,
            tail,
            no_argv0,
            &levels[..depth],
            rn,
            file0,
            argv,
            envp,
            s,
        )
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_t1703;
