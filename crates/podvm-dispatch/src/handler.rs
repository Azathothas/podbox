//! The SIGSYS handler: each system call of the payload comes here.
//!
//! The kernel gives the call in the registers of the trap
//! (`PR_SET_SYSCALL_USER_DISPATCH`); the handler serves it from podvm's own
//! code, whose calls do not dispatch, and writes the result to RAX.
//!
//! ⛔ A forced SIGSYS that arrives while SIGSYS is blocked resets the
//! handler to SIG_DFL and the kernel kills the process. So the handler has
//! `SA_NODEFER`, and each mask of the payload loses SIGSYS.

use core::sync::atomic::Ordering;

use crate::signals::{keep_thread_state, sigaction, sigaltstack, sigprocmask, KAction};

use crate::state::{self, BRK_BASE, BRK_CUR, BRK_END};
use crate::sys::{self, nr, sc, sc6};

pub const SIGSYS: i64 = 31;
pub const SA_SIGINFO: u64 = 4;
pub const SA_ONSTACK: u64 = 0x0800_0000;
pub const SA_RESTORER: u64 = 0x0400_0000;
pub const SA_NODEFER: u64 = 0x4000_0000;
const CLONE_VM: i64 = 0x100;
const CLONE_VFORK: i64 = 0x4000;
const CLONE_SIGHAND: i64 = 0x800;
const CLONE_THREAD: i64 = 0x1_0000;
const SIGCHLD: i64 = 17;
const PR_SET_SYSCALL_USER_DISPATCH: i64 = 59;
/// The stack of a vfork child, and the part of the given stack it keeps.
const VFORK_STACK: i64 = 256 << 10;
const WINDOW: i64 = 256;

/// The registers of `ucontext_t.uc_mcontext.gregs` (x86_64).
pub mod reg {
    pub const R8: usize = 0;
    pub const R9: usize = 1;
    pub const R10: usize = 2;
    pub const RDI: usize = 8;
    pub const RSI: usize = 9;
    pub const RDX: usize = 12;
    pub const RAX: usize = 13;
    pub const RIP: usize = 16;
    pub const EFL: usize = 17;
    /// The offset of `gregs` in `ucontext_t`.
    pub const OFFSET: usize = 40;
}

unsafe extern "C" {
    pub(crate) fn podvm_dispatch_restorer();
    fn podvm_dispatch_clone_thread(flags: i64, sp: i64, ptid: i64, ctid: i64, tls: i64) -> i64;
}

core::arch::global_asm!(
    ".globl podvm_dispatch_restorer",
    "podvm_dispatch_restorer:",
    "mov eax, 15",
    "syscall",
    "ud2",
    // clone(flags, sp, ptid, ctid, tls). The child starts on `sp` with the
    // frame that `clone` wrote below it: the region, then the registers of
    // the payload, its flags and its RIP. It arms dispatch for itself and
    // goes back to the payload with RAX = 0 and RSP = sp.
    ".globl podvm_dispatch_clone_thread",
    "podvm_dispatch_clone_thread:",
    "mov r10, rcx",
    "mov eax, 56",
    "syscall",
    "test rax, rax",
    "jz 2f",
    "ret",
    "2:",
    "mov rdx, [rsp - 152]",
    "mov r10, [rsp - 144]",
    "mov edi, 59",
    "mov esi, 1",
    "xor r8d, r8d",
    "mov eax, 157",
    "syscall",
    "lea rsp, [rsp - 136]",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rbp",
    "pop rdi",
    "pop rsi",
    "pop rdx",
    "pop rcx",
    "pop rbx",
    "pop rax",
    "popfq",
    "ret",
);

/// Arm dispatch for the calling thread: each call outside podvm's code
/// traps.
pub fn arm() -> i64 {
    let start = state::REGION_START.load(Ordering::Relaxed) as i64;
    let len = state::REGION_LEN.load(Ordering::Relaxed) as i64;
    // SAFETY: a NULL selector: the kernel reads no memory.
    unsafe { sc6(nr::PRCTL, PR_SET_SYSCALL_USER_DISPATCH, 1, start, len, 0, 0) }
}

/// The action of SIGSYS: this handler, on the payload's signal stack.
pub fn sigsys_action() -> KAction {
    KAction {
        handler: on_sigsys as *const () as usize as u64,
        flags: SA_SIGINFO | SA_RESTORER | SA_ONSTACK | SA_NODEFER,
        restorer: podvm_dispatch_restorer as *const () as usize as u64,
        mask: 0,
    }
}

fn brk(want: i64) -> i64 {
    let base = BRK_BASE.load(Ordering::Relaxed) as i64;
    let end = BRK_END.load(Ordering::Relaxed) as i64;
    let cur = BRK_CUR.load(Ordering::Relaxed) as i64;
    if base == 0 || want < base || want > end {
        return cur;
    }
    let page = |v: i64| (v + 4095) & !4095;
    if page(want) > page(cur) {
        // SAFETY: the range lies in the reserved range of the break.
        let r = unsafe { sc(nr::MPROTECT, page(cur), page(want) - page(cur), 3) };
        if r < 0 {
            return cur;
        }
    }
    BRK_CUR.store(want as usize, Ordering::Relaxed);
    want
}

/// clone, fork and vfork. A child of a copied address space arms dispatch
/// in this handler; a thread starts in the trampoline.
fn clone(a: [i64; 6], g: *mut i64) -> i64 {
    let (mut flags, sp, ptid, ctid, tls) = (a[0], a[1], a[2], a[3], a[4]);
    if sp == 0 && flags & CLONE_VM != 0 {
        // A vfork on the same stack would write over this handler's frame
        // in the parent: it becomes a fork, which the child cannot tell.
        if flags & (CLONE_THREAD | CLONE_SIGHAND) != 0 {
            return -sys::EINVAL;
        }
        flags &= !CLONE_VM;
    }
    if sp == 0 {
        // SAFETY: a fork: the child returns here with a copy of the frame.
        let r = unsafe { sc6(nr::CLONE, flags, 0, ptid, ctid, tls, 0) };
        if r == 0 {
            arm();
        }
        return r;
    }
    // ⛔ A vfork child on a stack of its own (musl's posix_spawn gives
    // about 5 KiB) has no room for this handler and the kernel's frame: an
    // overflow wrote over the parent's `args` (measured: a fault at
    // address 59). The parent waits for the child's exec or exit, so the
    // child takes a larger stack for that time: the top of the given one,
    // where the libc keeps the function and its argument, is copied there.
    let mut moved = 0i64;
    let mut sp = sp;
    if flags & CLONE_VM != 0 && flags & CLONE_VFORK != 0 {
        // SAFETY: an anonymous map for the child's stack.
        let m = unsafe { sc6(nr::MMAP, 0, VFORK_STACK, 3, 0x22, -1, 0) };
        if m >= 0 {
            let top = (m + VFORK_STACK - WINDOW) & !15;
            // SAFETY: the window above sp is the libc's, in the parent's
            // stack; the copy lands in the new map.
            unsafe {
                core::ptr::copy_nonoverlapping(sp as *const u8, top as *mut u8, WINDOW as usize)
            };
            sp = top;
            moved = m;
        }
    }
    // The frame of the trampoline, below the child's stack.
    let f = (sp - 152) as *mut i64;
    // SAFETY: the payload gave sp as the top of a stack it owns; the frame
    // lies in the unused part below it. g holds the trap's registers.
    unsafe {
        let gr = |i: usize| *g.add(i);
        let regs = [
            gr(7),
            gr(6),
            gr(5),
            gr(4),
            gr(3),
            gr(2),
            gr(1),
            gr(0), // r15..r8
            gr(10),
            gr(8),
            gr(9),
            gr(12),
            gr(14),
            gr(11),
            0, // rbp rdi rsi rdx rcx rbx rax
        ];
        *f = state::REGION_START.load(Ordering::Relaxed) as i64;
        *f.add(1) = state::REGION_LEN.load(Ordering::Relaxed) as i64;
        for (i, v) in regs.iter().enumerate() {
            *f.add(2 + i) = *v;
        }
        *f.add(17) = gr(reg::EFL);
        *f.add(18) = gr(reg::RIP);
        let r = podvm_dispatch_clone_thread(flags, sp, ptid, ctid, tls);
        if moved != 0 {
            // The child has exec'd or exited: its stack is free.
            sc(nr::MUNMAP, moved, VFORK_STACK, 0);
        }
        r
    }
}

/// Keep the root's descriptor out of reach of close, dup2 and close_range.
fn guard_fd(n: i64, a: [i64; 6]) -> Option<i64> {
    let root = state::root_fd();
    match n {
        nr::CLOSE if a[0] == root => Some(-sys::EBADF),
        nr::DUP2 | nr::DUP3 if a[1] == root => Some(-sys::EBUSY),
        nr::CLOSE_RANGE if a[0] <= root && root <= a[1] => {
            // SAFETY: two calls around the root's descriptor.
            unsafe {
                if a[0] < root {
                    sc(nr::CLOSE_RANGE, a[0], root - 1, a[2]);
                }
                if root < a[1] {
                    sc(nr::CLOSE_RANGE, root + 1, a[1], a[2]);
                }
            }
            Some(0)
        }
        _ => None,
    }
}

/// The SIGSYS handler.
///
/// # Safety
/// The kernel calls it with a SIGSYS of dispatch.
pub unsafe extern "C" fn on_sigsys(_sig: i32, info: *mut u8, uc: *mut u8) {
    // SAFETY: the kernel's siginfo and ucontext for this signal.
    unsafe {
        let g = uc.add(reg::OFFSET) as *mut i64;
        let n = *(info.add(24) as *const i32) as i64;
        let a = [
            *g.add(reg::RDI),
            *g.add(reg::RSI),
            *g.add(reg::RDX),
            *g.add(reg::R10),
            *g.add(reg::R8),
            *g.add(reg::R9),
        ];
        let r = call(n, a, g);
        *g.add(reg::RAX) = r;
        if r >= 0 {
            keep_thread_state(n, a, uc);
        }
    }
}

fn call(n: i64, a: [i64; 6], g: *mut i64) -> i64 {
    if let Some(r) = guard_fd(n, a) {
        return r;
    }
    match n {
        nr::CLONE => clone(a, g),
        nr::FORK => clone([SIGCHLD, 0, 0, 0, 0, 0], g),
        nr::VFORK => clone([CLONE_VFORK | SIGCHLD, 0, 0, 0, 0, 0], g),
        nr::CLONE3 => -sys::ENOSYS,
        nr::EXECVE => crate::exec::execveat(sys::AT_FDCWD, a[0], a[1], a[2], 0),
        nr::EXECVEAT => crate::exec::execveat(a[0], a[1], a[2], a[3], a[4]),
        nr::RT_SIGACTION => sigaction(a[0], a[1], a[2], a[3]),
        nr::RT_SIGPROCMASK => sigprocmask(a[0], a[1], a[2], a[3]),
        nr::RT_SIGRETURN => -sys::ENOSYS,
        // SAFETY: g is the gregs of this frame.
        nr::SIGALTSTACK => unsafe { sigaltstack(a[0], a[1], g) },
        nr::BRK => brk(a[0]),
        nr::PRCTL if a[0] == PR_SET_SYSCALL_USER_DISPATCH => -sys::EINVAL,
        // SAFETY: each other call runs as the payload gave it.
        _ => crate::calls::path_call(n, a)
            .unwrap_or_else(|| unsafe { sc6(n, a[0], a[1], a[2], a[3], a[4], a[5]) }),
    }
}
