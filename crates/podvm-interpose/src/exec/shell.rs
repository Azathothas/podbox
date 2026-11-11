//! `system(3)`, `popen(3)` and `pclose(3)`, T-1703.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1703. glibc and
//! musl start `/bin/sh` for these through `posix_spawn` inside libc, past
//! the preload, so the kernel reads the loader of the host's `/bin/sh`.
//! Where the table anchors guest `/`, podvm exports them and starts the
//! shell through its own routed [`posix_spawn`](super::calls::posix_spawn):
//! the image's `/bin/sh` enters through the image's loader. Elsewhere each
//! one is the real call.
//!
//! The signal rule of POSIX for `system`: the caller ignores `SIGINT` and
//! `SIGQUIT` and blocks `SIGCHLD` while it waits; the shell gets the
//! caller's mask and the default action for each signal that the caller
//! did not ignore.
//!
//! ⛔ No allocation and no lock: the open streams of `popen` are a fixed
//! table of atomics. The opaque libc types are buffers larger than both
//! libcs' own (glibc and musl: `posix_spawnattr_t` 336 bytes,
//! `posix_spawn_file_actions_t` 80, `struct sigaction` 152). podvm writes
//! only the handler, at offset 0 on both.

use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

use crate::map::OUT;

crate::real!(pub fn next_system = "system"(*const c_char) -> c_int);
crate::real!(pub fn next_popen = "popen"(*const c_char, *const c_char) -> *mut c_void);
crate::real!(pub fn next_pclose = "pclose"(*mut c_void) -> c_int);

unsafe extern "C" {
    static environ: *const *const c_char;
    fn sigaction(sig: c_int, act: *const c_void, old: *mut c_void) -> c_int;
    fn sigprocmask(how: c_int, set: *const c_void, old: *mut c_void) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    fn fdopen(fd: c_int, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fileno(f: *mut c_void) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn posix_spawnattr_init(a: *mut c_void) -> c_int;
    fn posix_spawnattr_destroy(a: *mut c_void) -> c_int;
    fn posix_spawnattr_setflags(a: *mut c_void, flags: i16) -> c_int;
    fn posix_spawnattr_setsigmask(a: *mut c_void, set: *const c_void) -> c_int;
    fn posix_spawnattr_setsigdefault(a: *mut c_void, set: *const c_void) -> c_int;
    fn posix_spawn_file_actions_init(fa: *mut c_void) -> c_int;
    fn posix_spawn_file_actions_destroy(fa: *mut c_void) -> c_int;
    fn posix_spawn_file_actions_adddup2(fa: *mut c_void, fd: c_int, to: c_int) -> c_int;
    fn posix_spawn_file_actions_addclose(fa: *mut c_void, fd: c_int) -> c_int;
}

const SIGINT: c_int = 2;
const SIGQUIT: c_int = 3;
const SIGCHLD: c_int = 17;
const SIG_BLOCK: c_int = 0;
const SIG_SETMASK: c_int = 2;
const SIG_IGN: usize = 1;
const POSIX_SPAWN_SETSIGDEF: i16 = 0x04;
const POSIX_SPAWN_SETSIGMASK: i16 = 0x08;
const O_CLOEXEC: c_int = 0o2_000_000;
const F_SETFD: c_int = 2;
const EINTR: c_int = 4;
const EINVAL: c_int = 22;
const EMFILE: c_int = 24;

/// The shell, in guest names.
const SH: &core::ffi::CStr = c"/bin/sh";

/// An opaque libc object, larger than both libcs' own.
#[repr(C, align(16))]
struct Opaque([u64; 64]);

impl Opaque {
    const fn new() -> Opaque {
        Opaque([0; 64])
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
}

/// A kernel signal set with the given signals.
fn sigset(sigs: &[c_int]) -> Opaque {
    let mut s = Opaque::new();
    for &n in sigs {
        let b = (n - 1) as usize;
        s.0[b / 64] |= 1 << (b % 64);
    }
    s
}

/// Does the table anchor guest `/`? Only then do these differ from libc.
fn routed() -> bool {
    let t = unsafe { crate::map::table() };
    t.n > 0 && super::anchored(&t)
}

/// Start `sh -c cmd` through the routed spawn. Returns the error number.
unsafe fn spawn_sh(
    pid: &mut c_int,
    cmd: *const c_char,
    fa: *const c_void,
    attr: *const c_void,
) -> c_int {
    let argv = [c"sh".as_ptr(), c"-c".as_ptr(), cmd, core::ptr::null()];
    unsafe { super::calls::posix_spawn(pid, SH.as_ptr(), fa, attr, argv.as_ptr(), environ) }
}

/// Wait for `pid` through `EINTR`. The status, or `-1`.
unsafe fn reap(pid: c_int) -> c_int {
    let mut status = 0;
    loop {
        if unsafe { waitpid(pid, &mut status, 0) } >= 0 {
            return status;
        }
        if crate::errno() != EINTR {
            return -1;
        }
    }
}

/// # Safety
/// The payload's own contract for `system(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn system(cmd: *const c_char) -> c_int {
    if !routed() {
        return match next_system() {
            Some(f) => unsafe { f(cmd) },
            None => -1,
        };
    }
    if cmd.is_null() {
        // A shell exists: the route refuses a run where none resolves.
        return 1;
    }
    let mut ign = Opaque::new();
    ign.0[0] = SIG_IGN as u64;
    let (mut old_int, mut old_quit, mut old_mask) = (Opaque::new(), Opaque::new(), Opaque::new());
    unsafe {
        sigaction(SIGINT, ign.ptr(), old_int.ptr());
        sigaction(SIGQUIT, ign.ptr(), old_quit.ptr());
        sigprocmask(SIG_BLOCK, sigset(&[SIGCHLD]).ptr(), old_mask.ptr());
    }
    let mut def = [0; 2];
    let mut k = 0;
    for (sig, old) in [(SIGINT, &old_int), (SIGQUIT, &old_quit)] {
        if old.0[0] != SIG_IGN as u64 {
            def[k] = sig;
            k += 1;
        }
    }
    let mut attr = Opaque::new();
    let mut pid = 0;
    let r = unsafe {
        posix_spawnattr_init(attr.ptr());
        posix_spawnattr_setsigmask(attr.ptr(), old_mask.ptr());
        posix_spawnattr_setsigdefault(attr.ptr(), sigset(&def[..k]).ptr());
        posix_spawnattr_setflags(attr.ptr(), POSIX_SPAWN_SETSIGDEF | POSIX_SPAWN_SETSIGMASK);
        let r = spawn_sh(&mut pid, cmd, core::ptr::null(), attr.ptr());
        posix_spawnattr_destroy(attr.ptr());
        r
    };
    // glibc's answer where the shell does not start: the status of `exit 127`.
    let status = if r == 0 {
        unsafe { reap(pid) }
    } else {
        127 << 8
    };
    let e = crate::errno();
    unsafe {
        sigaction(SIGINT, old_int.ptr(), core::ptr::null_mut());
        sigaction(SIGQUIT, old_quit.ptr(), core::ptr::null_mut());
        sigprocmask(SIG_SETMASK, old_mask.ptr(), core::ptr::null_mut());
    }
    crate::set_errno(if r != 0 { r } else { e });
    status
}

/// The streams that [`popen`] opened, and their shells.
const SLOTS: usize = 128;
static FILES: [AtomicPtr<c_void>; SLOTS] = [const { AtomicPtr::new(core::ptr::null_mut()) }; SLOTS];
static PIDS: [AtomicI32; SLOTS] = [const { AtomicI32::new(0) }; SLOTS];

/// # Safety
/// The payload's own contract for `popen(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut c_void {
    if !routed() {
        return match next_popen() {
            Some(f) => unsafe { f(cmd, mode) },
            None => core::ptr::null_mut(),
        };
    }
    let m = if mode.is_null() {
        &[][..]
    } else {
        let n = crate::map::strnlen(mode, OUT);
        unsafe { core::slice::from_raw_parts(mode as *const u8, n) }
    };
    let read = match m.first() {
        Some(b'r') => true,
        Some(b'w') => false,
        _ => {
            crate::set_errno(EINVAL);
            return core::ptr::null_mut();
        }
    };
    let keep_cloexec = m.contains(&b'e');
    if !FILES.iter().any(|f| f.load(Ordering::Acquire).is_null()) {
        crate::set_errno(EMFILE);
        return core::ptr::null_mut();
    }
    let mut fds = [-1; 2];
    if unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) } != 0 {
        return core::ptr::null_mut();
    }
    let (mine, theirs, target) = if read {
        (fds[0], fds[1], 1)
    } else {
        (fds[1], fds[0], 0)
    };
    let mut fa = Opaque::new();
    let mut pid = 0;
    let r = unsafe {
        posix_spawn_file_actions_init(fa.ptr());
        // dup2 clears FD_CLOEXEC on the target; a pipe end that is already
        // the target keeps it, so it loses it here, in the child's copy.
        if theirs == target {
            fcntl(theirs, F_SETFD, 0);
        } else {
            posix_spawn_file_actions_adddup2(fa.ptr(), theirs, target);
        }
        // POSIX: the shell does not keep the streams of earlier calls.
        for f in &FILES {
            let p = f.load(Ordering::Acquire);
            if !p.is_null() {
                posix_spawn_file_actions_addclose(fa.ptr(), fileno(p));
            }
        }
        let r = spawn_sh(&mut pid, cmd, fa.ptr(), core::ptr::null());
        posix_spawn_file_actions_destroy(fa.ptr());
        close(theirs);
        r
    };
    if r != 0 {
        unsafe { close(mine) };
        crate::set_errno(r);
        return core::ptr::null_mut();
    }
    if !keep_cloexec {
        unsafe { fcntl(mine, F_SETFD, 0) };
    }
    let fmode = if read { c"r" } else { c"w" };
    let f = unsafe { fdopen(mine, fmode.as_ptr()) };
    if f.is_null() {
        let e = crate::errno();
        unsafe {
            close(mine);
            reap(pid);
        }
        crate::set_errno(e);
        return core::ptr::null_mut();
    }
    // Claim a slot, then store the pid: `pclose` of this stream comes
    // only after this return, so it reads the pid stored here.
    let claimed = (0..SLOTS).find(|&i| {
        FILES[i]
            .compare_exchange(
                core::ptr::null_mut(),
                f,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    });
    match claimed {
        Some(i) => PIDS[i].store(pid, Ordering::Release),
        None => crate::say::line(&[
            b"popen: podvm has no free slot, so pclose of this stream does not wait for the shell",
        ]),
    }
    f
}

/// # Safety
/// The payload's own contract for `pclose(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pclose(f: *mut c_void) -> c_int {
    let hit = (0..SLOTS).find(|&i| {
        !f.is_null()
            && FILES[i]
                .compare_exchange(
                    f,
                    core::ptr::null_mut(),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
    });
    let Some(i) = hit else {
        return match next_pclose() {
            Some(real) => unsafe { real(f) },
            None => -1,
        };
    };
    let pid = PIDS[i].load(Ordering::Acquire);
    unsafe {
        fclose(f);
        reap(pid)
    }
}
