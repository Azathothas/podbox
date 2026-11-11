//! The signals of the payload: its actions (each caught signal goes through
//! a wrapper in podvm, whose restorer lies in podvm's code), its mask with
//! no SIGSYS, and its alternate stack on the frame of the trap.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::handler::{
    podvm_dispatch_restorer, reg, sigsys_action, SA_RESTORER, SA_SIGINFO, SIGSYS,
};
use crate::sys::{self, nr, sc6};

const SIGKILL: i64 = 9;
const SIGSTOP: i64 = 19;
const SA_RESETHAND: u64 = 0x8000_0000;
pub(crate) const SYS_BIT: u64 = 1 << (SIGSYS - 1);

/// `struct kernel_sigaction` of x86_64.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KAction {
    pub handler: u64,
    pub flags: u64,
    pub restorer: u64,
    pub mask: u64,
}

/// The actions that the payload set, as it set them.
static HANDLER: [AtomicU64; 65] = [const { AtomicU64::new(0) }; 65];
static FLAGS: [AtomicU64; 65] = [const { AtomicU64::new(0) }; 65];
static MASK: [AtomicU64; 65] = [const { AtomicU64::new(0) }; 65];

pub fn rt_sigaction(sig: i64, act: Option<&KAction>, old: Option<&mut KAction>) -> i64 {
    let a = act.map_or(0, |a| a as *const KAction as i64);
    let o = old.map_or(0, |o| o as *mut KAction as i64);
    // SAFETY: both structs live for the call.
    unsafe { sc6(nr::RT_SIGACTION, sig, a, o, 8, 0, 0) }
}

/// Record the payload's start: SIG_DFL for each signal that is not
/// ignored, and this handler for SIGSYS.
pub fn reset_signals() {
    for sig in 1..=64i64 {
        if sig == SIGKILL || sig == SIGSTOP || sig == SIGSYS {
            continue;
        }
        let mut old = KAction::default();
        if rt_sigaction(sig, None, Some(&mut old)) < 0 {
            continue;
        }
        // podvm's own SIGPIPE is ignored by Rust's runtime, not by the caller.
        if old.handler == 1 && sig != 13 {
            HANDLER[sig as usize].store(1, Ordering::Relaxed);
            continue;
        }
        let dfl = KAction {
            handler: 0,
            flags: SA_RESTORER,
            restorer: podvm_dispatch_restorer as *const () as usize as u64,
            mask: 0,
        };
        rt_sigaction(sig, Some(&dfl), None);
    }
    rt_sigaction(SIGSYS, Some(&sigsys_action()), None);
    // Unblock SIGSYS.
    let unblock: u64 = SYS_BIT;
    // SAFETY: the set lives for the call.
    unsafe {
        sc6(
            nr::RT_SIGPROCMASK,
            1,
            &unblock as *const u64 as i64,
            0,
            8,
            0,
            0,
        )
    };
}

/// The handler of each signal that the payload catches.
unsafe extern "C" fn wrap(sig: i32, info: *mut u8, uc: *mut u8) {
    let s = (sig as usize).min(64);
    let h = HANDLER[s].load(Ordering::Relaxed);
    let fl = FLAGS[s].load(Ordering::Relaxed);
    if fl & SA_RESETHAND != 0 {
        HANDLER[s].store(0, Ordering::Relaxed);
    }
    if h <= 1 {
        return;
    }
    // SAFETY: the payload registered h with these flags.
    unsafe {
        if fl & SA_SIGINFO != 0 {
            let f: extern "C" fn(i32, *mut u8, *mut u8) = core::mem::transmute(h as usize);
            f(sig, info, uc);
        } else {
            let f: extern "C" fn(i32) = core::mem::transmute(h as usize);
            f(sig);
        }
        // ⛔ A handler may write the mask of its return into the frame:
        // qemu-user blocks each host signal there (measured: Go under
        // qemu-x86_64 died on signal 31, T-1651). SIGSYS stays unblocked,
        // or the next trap kills the process.
        let mask = uc.add(296) as *mut u64;
        *mask &= !SYS_BIT;
    }
}

pub(crate) fn sigaction(sig: i64, act: i64, oact: i64, size: i64) -> i64 {
    if !(1..=64).contains(&sig) || size != 8 {
        // SAFETY: the payload's own call; the kernel gives EINVAL.
        return unsafe { sc6(nr::RT_SIGACTION, sig, act, oact, size, 0, 0) };
    }
    let s = sig as usize;
    let prev = KAction {
        handler: HANDLER[s].load(Ordering::Relaxed),
        flags: FLAGS[s].load(Ordering::Relaxed),
        restorer: 0,
        mask: MASK[s].load(Ordering::Relaxed),
    };
    if act != 0 {
        // SAFETY: the payload gave act as a struct kernel_sigaction.
        let a = unsafe { *(act as *const KAction) };
        if sig != SIGSYS {
            let mut k = KAction {
                handler: a.handler,
                flags: a.flags | SA_RESTORER,
                restorer: podvm_dispatch_restorer as *const () as usize as u64,
                mask: a.mask & !SYS_BIT,
            };
            if a.handler > 1 {
                k.handler = wrap as *const () as usize as u64;
                k.flags |= SA_SIGINFO;
            }
            let r = rt_sigaction(sig, Some(&k), None);
            if r < 0 {
                return r;
            }
        }
        HANDLER[s].store(a.handler, Ordering::Relaxed);
        FLAGS[s].store(a.flags, Ordering::Relaxed);
        MASK[s].store(a.mask, Ordering::Relaxed);
    }
    if oact != 0 {
        // SAFETY: the payload gave oact for a struct kernel_sigaction.
        unsafe { *(oact as *mut KAction) = prev };
    }
    0
}

pub(crate) fn sigprocmask(how: i64, set: i64, old: i64, size: i64) -> i64 {
    let copy: u64 = if set != 0 && size == 8 {
        // SAFETY: the payload gave set for a sigset of 8 bytes.
        let v = unsafe { *(set as *const u64) };
        if how != 1 {
            v & !SYS_BIT
        } else {
            v
        }
    } else {
        0
    };
    let p = if set != 0 && size == 8 {
        &copy as *const u64 as i64
    } else {
        set
    };
    // SAFETY: the sets live for the call.
    unsafe { sc6(nr::RT_SIGPROCMASK, how, p, old, size, 0, 0) }
}

/// ⛔ `rt_sigreturn` restores the signal mask from the frame. A
/// `rt_sigprocmask` that this handler ran for the payload would be undone
/// when it returns (measured: Go's mask), so the frame takes the thread's
/// mask after the call.
///
/// # Safety
/// `uc` is the frame of this SIGSYS.
pub(crate) unsafe fn keep_thread_state(n: i64, a: [i64; 6], uc: *mut u8) {
    if n == nr::RT_SIGPROCMASK && a[1] != 0 {
        // SAFETY: `uc_sigmask` is at 296 in the x86_64 ucontext.
        unsafe { sc6(nr::RT_SIGPROCMASK, 0, 0, uc.add(296) as i64, 8, 0, 0) };
    }
}

/// The payload's `sigaltstack`, on the frame's `uc_stack`.
///
/// ⛔ The frame holds the thread's alternate stack before the trap, and
/// `rt_sigreturn` sets the thread's from it. A call that the handler ran
/// itself would be undone at the return (measured: Go lost its signal
/// stack and spun in `needm`), and while this handler runs on the
/// alternate stack the kernel refuses a change with EPERM (measured: a
/// thread that disabled its stack and unmapped it lost the handler's
/// frame). So the answer comes from the frame, and the change goes into it,
/// with `SS_AUTODISARM`: the kernel then disarms the stack while a handler
/// runs on it, so that the change of a later call applies at the return.
/// The payload does not see the flag.
///
/// # Safety
/// `g` is the gregs of this SIGSYS frame.
pub(crate) unsafe fn sigaltstack(new: i64, old: i64, g: *mut i64) -> i64 {
    const SS_ONSTACK: i64 = 1;
    const SS_DISABLE: i64 = 2;
    const SS_AUTODISARM: i64 = 1 << 31;
    // SAFETY: `uc_stack` is at 16 in the ucontext, and gregs at 40.
    unsafe {
        let st = (g as *mut u8).sub(reg::OFFSET).add(16) as *mut i64;
        let (sp0, flags0, size0) = (*st, *(st.add(1) as *const i32) as i64, *st.add(2));
        let user_sp = *g.add(15);
        let on = flags0 & SS_DISABLE == 0 && user_sp > sp0 && user_sp <= sp0 + size0;
        if old != 0 {
            let o = old as *mut i64;
            *o = sp0;
            *(o.add(1) as *mut i32) = if on { SS_ONSTACK } else { flags0 & SS_DISABLE } as i32;
            *o.add(2) = size0;
        }
        if new != 0 {
            if on {
                return -sys::EPERM;
            }
            let n = new as *const i64;
            let (sp, flags, size) = (*n, *(n.add(1) as *const i32) as i64, *n.add(2));
            let mode = flags & !SS_AUTODISARM;
            if mode != 0 && mode != SS_ONSTACK && mode != SS_DISABLE {
                return -sys::EINVAL;
            }
            if mode == SS_DISABLE {
                *st = 0;
                *(st.add(1) as *mut i32) = SS_DISABLE as i32;
                *st.add(2) = 0;
            } else {
                if size < 2048 {
                    return -sys::ENOMEM;
                }
                *st = sp;
                *(st.add(1) as *mut i32) = SS_AUTODISARM as i32;
                *st.add(2) = size;
            }
        }
    }
    0
}
