//! Starting a payload without handing it to the kernel's loader.
//!
//! ⭐ **Why the launcher needs this.** A payload has to be started with a
//! library search path and an argv0 the launcher chooses, against a loader the
//! bundle carries rather than the host's, and without the file on disk being
//! patched to say so. `execve` cannot do that: the interpreter is baked into
//! the file's `PT_INTERP`, and rewriting it means writing to the payload.
//! Mapping the image here and jumping to its entry point is what removes the
//! write.
//!
//! The shape is the same one the kernel uses, and there are only four parts:
//! map the image's `PT_LOAD` segments, map its interpreter beside it, build
//! the stack the ABI describes - argc, argv, envp and the auxiliary vector -
//! and jump to the entry point with the stack pointer set.
//!
//! ⛔ **Three things here are deliberately not what the subject does**, and each
//! is a defect rather than a preference:
//!
//!   - **it maps segments with their own protections.** The subject forces
//!     `PROT_WRITE` on every mapping and marks it `TODO: read only fix`, which
//!     leaves the payload's `.text` writable for its whole life. Ours maps
//!     writable, zero-fills, then drops each segment to the protection its
//!     own header asks for;
//!   - ⛔ **nothing here panics on the image's own bytes.** The subject
//!     unwraps thirty times over attacker-controlled fields and asserts that
//!     the image is position-independent. Every one of those is a `Refusal`
//!     with a sentence;
//!   - **it reserves the span and maps into it.** The reservation is what
//!     stops a second mapping landing in the middle of the image.
//!
//! ⚠ **What this is NOT.** It is not a dynamic loader: it does not resolve a
//! symbol, does not process `DT_NEEDED`, and does not run an initialiser. When
//! the image names an interpreter, the interpreter does all of that after the
//! jump, which is exactly what the kernel would have arranged. When it does
//! not - a static PIE, or a dynamic loader started as the program - the image
//! relocates itself, as it does under the kernel, which applies no relocation
//! either. The subject applied RELATIVE relocations here; that made a loader
//! started as the program (`ld.so --argv0 NAME PROGRAM`) unstartable, because
//! its own table carries other types.
//!
//! ⛔ **AND THIS PROJECT ALREADY OWNS A LOADER, which is the opposite decision
//! and has to be read before this file is changed.**
//! `tool/runtime/binary/elfload.c` maps an object, walks `DT_NEEDED`, relocates
//! it - `DT_RELR` included - honours symbol versioning, runs the initialisers,
//! and **binds every undefined symbol to the libc already linked into the
//! image**, answering a `DT_NEEDED` the process already satisfies instead of
//! opening it. That is what keeps a SECOND libc out, and its residue is
//! measured: of every shared object on one build host, the number that crash
//! it and that glibc's own `ld.so` loads cleanly is zero.
//! `docs/binary/host-dlopen.md` is the page.
//!
//! ⚠ **This file makes the other choice on purpose.** It maps a real `ld.so`
//! beside the payload and jumps into it, the way the kernel does. The
//! interpreter arms agree with the kernel on the eleven pinned loaders
//! (`scripts/common/uexec-across.sh`). The fault they once showed was the
//! page-tail zeroing in `map_image`, not the second loader; the history is in
//! `docs/history/entries/port.md`.
//!
//! ⛔ **So: a third loader is a decision, not a default.** Anything added here
//! that resolves a symbol or answers a `DT_NEEDED` is work `elfload.c` has
//! already done in 1,398 lines of shipped C.
//!
//! SPDX-License-Identifier: 0BSD

use std::ffi::CString;
use std::path::Path;

use crate::elf::{self, LoadPlan};

// ⛔ No crate. The launcher takes no dependency, so the handful of calls this
// needs are declared here rather than pulled in behind a wrapper this project
// would then own.
extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut core::ffi::c_void;
    fn mprotect(addr: *mut core::ffi::c_void, len: usize, prot: i32) -> i32;
    fn getauxval(kind: u64) -> u64;
    fn getgid() -> u32;
    fn getegid() -> u32;
    fn sysconf(name: i32) -> i64;
}

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_EXEC: i32 = 4;

const MAP_PRIVATE: i32 = 0x02;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_STACK: i32 = 0x0002_0000;
const MAP_FAILED: isize = -1;

/// The auxiliary vector entries this builds. ⚠ The set is not arbitrary: a
/// loader reads `AT_PHDR`, `AT_PHENT`, `AT_PHNUM`, `AT_ENTRY` and `AT_BASE` to
/// find the image it is loading, libc reads `AT_PAGESZ`, `AT_HWCAP`,
/// `AT_CLKTCK` and the four id entries, and the stack protector reads
/// `AT_RANDOM` before `main` is reached.
mod at {
    pub const NULL: u64 = 0;
    pub const PHDR: u64 = 3;
    pub const PHENT: u64 = 4;
    pub const PHNUM: u64 = 5;
    pub const PAGESZ: u64 = 6;
    pub const BASE: u64 = 7;
    pub const FLAGS: u64 = 8;
    pub const ENTRY: u64 = 9;
    pub const UID: u64 = 11;
    pub const EUID: u64 = 12;
    pub const GID: u64 = 13;
    pub const EGID: u64 = 14;
    pub const PLATFORM: u64 = 15;
    pub const HWCAP: u64 = 16;
    pub const CLKTCK: u64 = 17;
    pub const SECURE: u64 = 23;
    pub const RANDOM: u64 = 25;
    pub const EXECFN: u64 = 31;
}

/// `_SC_CLK_TCK`, which is 2 on every Linux libc this targets.
const SC_CLK_TCK: i32 = 2;

/// PF_X, PF_W, PF_R from the ELF specification.
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

/// Why a payload could not be started this way.
///
/// ⛔ Every one of these is a sentence rather than a panic, because every one
/// of them is reached by handing this function a file somebody else wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The file could not be read, or is not an ELF this can map.
    Unreadable(String),
    /// A fixed-address image. ⚠ Not a defect in the image: an `ET_EXEC` wants
    /// its own addresses, and those may already be occupied by the launcher.
    NotRelocatable,
    /// A field that does not fit the address space, or arithmetic that wrapped.
    OutOfRange(&'static str),
    /// A mapping the kernel refused.
    MapFailed(&'static str),
    /// The auxiliary vector did not carry something the ABI requires.
    NoAuxv(&'static str),
}

impl Refusal {
    pub fn message(&self) -> String {
        match self {
            Refusal::Unreadable(e) => format!("cannot read the image: {e}"),
            Refusal::NotRelocatable => {
                "the image is ET_EXEC, so it wants fixed addresses this process may already be using; \
                 a position-independent build is what this route can start"
                    .to_string()
            }
            Refusal::OutOfRange(what) => format!("the image's {what} does not fit this address space"),
            Refusal::MapFailed(what) => format!("the kernel refused the mapping for {what}"),
            Refusal::NoAuxv(what) => format!(
                "this process's own auxiliary vector carries no {what}, and the payload's has to be built from it"
            ),
        }
    }
}

/// A mapped image.
#[derive(Debug, Clone, Copy)]
pub struct Mapped {
    /// Where the image was placed. Zero for a non-relocatable one.
    pub base: usize,
    /// The absolute address of the entry point.
    pub entry: usize,
}

fn page_size() -> usize {
    let p = unsafe { getauxval(at::PAGESZ) } as usize;
    if p == 0 {
        4096
    } else {
        p
    }
}

fn round_down(v: usize, page: usize) -> usize {
    v - (v % page)
}

fn round_up(v: usize, page: usize) -> usize {
    match v % page {
        0 => v,
        r => v + (page - r),
    }
}

/// The `mmap` protection a segment's own flags ask for.
fn prot_of(flags: u32) -> i32 {
    let mut p = PROT_NONE;
    if flags & PF_R != 0 {
        p |= PROT_READ;
    }
    if flags & PF_W != 0 {
        p |= PROT_WRITE;
    }
    if flags & PF_X != 0 {
        p |= PROT_EXEC;
    }
    p
}

/// Map the image at `path`, returning where it landed.
///
/// ⛔ Two mappings, deliberately. The first reserves the whole span with no
/// access at all; the second places each segment inside it with `MAP_FIXED`.
/// Reserving first is what makes the placement safe: without it, the second
/// segment's fixed mapping could land on something the allocator handed out
/// between the two calls.
pub fn map_image(path: &Path, plan: &LoadPlan) -> Result<Mapped, Refusal> {
    if !plan.pie {
        return Err(Refusal::NotRelocatable);
    }
    let page = page_size();
    let span = plan.span().ok_or(Refusal::OutOfRange("span"))?;
    let span = usize::try_from(span).map_err(|_| Refusal::OutOfRange("span"))?;
    let span = round_up(span, page);
    if span == 0 {
        return Err(Refusal::OutOfRange("span"));
    }

    // The reservation. PROT_NONE, so anything that reaches a hole between two
    // segments faults instead of reading whatever was there.
    let base = unsafe {
        mmap(
            core::ptr::null_mut(),
            span,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if base as isize == MAP_FAILED {
        return Err(Refusal::MapFailed("the image's address reservation"));
    }
    let base = base as usize;

    let file = std::fs::File::open(path).map_err(|e| Refusal::Unreadable(e.to_string()))?;
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    for s in &plan.load {
        let vaddr = usize::try_from(s.vaddr).map_err(|_| Refusal::OutOfRange("segment address"))?;
        let filesz = usize::try_from(s.filesz).map_err(|_| Refusal::OutOfRange("segment size"))?;
        let memsz = usize::try_from(s.memsz).map_err(|_| Refusal::OutOfRange("segment size"))?;
        let offset = i64::try_from(s.offset).map_err(|_| Refusal::OutOfRange("segment offset"))?;

        let want = base
            .checked_add(vaddr)
            .ok_or(Refusal::OutOfRange("segment address"))?;
        let start = round_down(want, page);
        let slack = want - start;

        // ⚠ Mapped WRITABLE here whatever the segment asks for, and dropped to
        // its real protection at the end. The zero fill below writes, and a
        // read-only segment cannot be written to; the subject's answer to that
        // was to leave every segment writable for good.
        if filesz > 0 {
            let len = round_up(filesz + slack, page);
            let p = unsafe {
                mmap(
                    start as *mut core::ffi::c_void,
                    len,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_FIXED,
                    fd,
                    offset - slack as i64,
                )
            };
            if p as isize == MAP_FAILED {
                return Err(Refusal::MapFailed("a segment's file pages"));
            }
        }

        // Anything past the file bytes is .bss and has to read as zero. The
        // reservation is anonymous, so the whole pages beyond are already
        // zero; what is not is the tail of the last file page, which carries
        // whatever the file had there.
        //
        // ⛔ To the END OF THE PAGE, not to the end of .bss. That is what the
        // kernel's padzero clears, and glibc's ld.so depends on it: its
        // early calloc takes memory from `_end` to the end of that page and
        // does not clear it, because the kernel already did. Zeroing only to
        // `_end` gave the executable's link_map a garbage l_info[], and ld.so
        // faulted in elf_get_dynamic_info before its first openat.
        // It shows only where `_end` shares a page with the file bytes:
        // Debian 12, Rocky 8 and openSUSE Leap 15.6 among the pinned eleven.
        // The subject zeroes to the page end too.
        if memsz > filesz {
            let file_end = want + filesz;
            // Only a page that has file bytes is mapped here. With no file
            // bytes, the first page is still the PROT_NONE reservation, which
            // is zero already and becomes writable in `protect`.
            if filesz > 0 {
                let zero_to = round_up(file_end, page);
                if zero_to > file_end {
                    unsafe {
                        core::ptr::write_bytes(file_end as *mut u8, 0, zero_to - file_end);
                    }
                }
            }
            // Whole pages past that need to exist and be writable.
            let anon_from = round_up(file_end, page);
            let anon_to = round_up(want + memsz, page);
            if anon_to > anon_from {
                let p = unsafe {
                    mmap(
                        anon_from as *mut core::ffi::c_void,
                        anon_to - anon_from,
                        PROT_READ | PROT_WRITE,
                        MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                        -1,
                        0,
                    )
                };
                if p as isize == MAP_FAILED {
                    return Err(Refusal::MapFailed("a segment's zero-filled tail"));
                }
            }
        }
    }

    let entry = base
        .checked_add(usize::try_from(plan.entry).map_err(|_| Refusal::OutOfRange("entry point"))?)
        .ok_or(Refusal::OutOfRange("entry point"))?;
    Ok(Mapped { base, entry })
}

/// Drop every segment to the protection its own header asks for.
///
/// ⛔ Called after the zero fill, never before. It is the second half of the
/// defect this module exists not to have: mapping writable is safe only if
/// something takes the write bit away again.
pub fn protect(plan: &LoadPlan, m: Mapped) -> Result<(), Refusal> {
    let page = page_size();
    for s in &plan.load {
        let vaddr = usize::try_from(s.vaddr).map_err(|_| Refusal::OutOfRange("segment address"))?;
        let memsz = usize::try_from(s.memsz).map_err(|_| Refusal::OutOfRange("segment size"))?;
        if memsz == 0 {
            continue;
        }
        let want = m.base + vaddr;
        let start = round_down(want, page);
        let len = round_up(want + memsz, page) - start;
        if unsafe { mprotect(start as *mut core::ffi::c_void, len, prot_of(s.flags)) } != 0 {
            return Err(Refusal::MapFailed("a segment's final protection"));
        }
    }
    Ok(())
}

/// The stack an entry point is handed, built into a buffer.
///
/// ⭐ Built as bytes first and copied into the mapping once, rather than
/// written through a moving pointer. The layout has to be known before the
/// addresses inside it can be, because every string's address depends on where
/// the block ends up: building it backwards into a buffer whose end address is
/// known is what makes that a calculation instead of two passes.
struct Stack {
    /// The bytes, in reverse: the last thing pushed is the lowest address.
    rev: Vec<u8>,
    /// Where the block's highest byte will be.
    top: usize,
}

impl Stack {
    fn new(top: usize) -> Self {
        Stack { rev: Vec::new(), top }
    }

    /// Push raw bytes and return the address they will have.
    fn bytes(&mut self, b: &[u8]) -> usize {
        for byte in b.iter().rev() {
            self.rev.push(*byte);
        }
        self.top - self.rev.len()
    }

    fn cstr(&mut self, s: &CString) -> usize {
        self.bytes(s.as_bytes_with_nul())
    }

    fn word(&mut self, v: usize) {
        self.bytes(&v.to_ne_bytes());
    }
}

/// This process's OWN auxiliary vector, as the kernel wrote it, in the kernel's
/// order.
///
/// ⛔ **Read from `/proc/self/auxv` and NOT from `getauxval`, and the
/// difference is measured rather than argued.** glibc rewrites `AT_HWCAP` in
/// the vector it answers from: on a machine whose kernel wrote **529267711**,
/// `getauxval(AT_HWCAP)` returns **2**, which is glibc's own private encoding.
/// A launcher that asked the libc therefore handed the payload the libc's value
/// under the kernel's name, and the payload's own libc - which may not be that
/// one - would read it as CPU feature bits.
///
/// ⭐ **And a subset is the wrong shape.** Enumerating tags means every entry
/// nobody thought of is silently dropped: against the kernel this was missing
/// `AT_SYSINFO_EHDR` - the vDSO - as well as `AT_MINSIGSTKSZ`, `AT_HWCAP2` and
/// two `AT_RSEQ_*` tags, twenty-two entries against seventeen. Carrying the
/// vector through and overriding only what describes the new IMAGE is both
/// smaller and faithful to whatever the kernel supplies next.
/// `scripts/common/uexec-stack.sh` is the reading, and the kernel is its oracle.
///
/// ⚠ The pointer-valued entries kept here point into THIS process's memory,
/// which stays mapped: `uexec` jumps inside the process rather than replacing
/// it, so the vDSO, the platform string and everything else are still there.
fn own_auxv() -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let raw = match std::fs::read("/proc/self/auxv") {
        Ok(r) => r,
        Err(_) => return out,
    };
    for c in raw.chunks_exact(16) {
        let t = u64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
        let v = u64::from_ne_bytes([c[8], c[9], c[10], c[11], c[12], c[13], c[14], c[15]]);
        if t == at::NULL {
            break;
        }
        out.push((t, v));
    }
    out
}

/// Put one entry in the vector, in place where it is already there and at the
/// end where it is not.
///
/// ⚠ In PLACE, so the kernel's order survives every override. An entry appended
/// instead would move `AT_PHDR` to the end of a vector the kernel wrote it near
/// the front of, and the comparison against the kernel would then differ on
/// every run for a reason that is not a defect.
fn auxv_set(v: &mut Vec<(u64, u64)>, kind: u64, value: u64) {
    for e in v.iter_mut() {
        if e.0 == kind {
            e.1 = value;
            return;
        }
    }
    v.push((kind, value));
}

/// Build the stack for a payload, and return its address.
///
/// ⚠ `argv` here is the whole vector including argv0, because argv0 is what
/// the launcher is choosing. A caller that passes only the arguments gets a
/// payload whose first argument is read as its own name.
fn build_stack(
    m: Mapped,
    plan: &LoadPlan,
    interp_base: Option<usize>,
    path: &CString,
    argv: &[CString],
    envp: &[CString],
) -> Result<usize, Refusal> {
    let size = 8 * 1024 * 1024;
    let mem = unsafe {
        mmap(
            core::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK,
            -1,
            0,
        )
    };
    if mem as isize == MAP_FAILED {
        return Err(Refusal::MapFailed("the payload's stack"));
    }
    let top = mem as usize + size;
    let mut st = Stack::new(top);

    // The strings first, because the arrays below hold their addresses.
    let path_at = st.cstr(path);
    let mut env_at = Vec::with_capacity(envp.len());
    for e in envp.iter().rev() {
        env_at.push(st.cstr(e));
    }
    let mut arg_at = Vec::with_capacity(argv.len());
    for a in argv.iter().rev() {
        arg_at.push(st.cstr(a));
    }

    // ⛔ AT_RANDOM's sixteen bytes are copied from this process's own auxiliary
    // vector rather than invented. libc reads them before `main` for the stack
    // protector, and a payload handed a null pointer there faults in libc's own
    // startup with no message anybody can act on.
    let random_ptr = unsafe { getauxval(at::RANDOM) } as *const u8;
    if random_ptr.is_null() {
        return Err(Refusal::NoAuxv("AT_RANDOM"));
    }
    let random = unsafe { core::slice::from_raw_parts(random_ptr, 16) };
    let random_at = st.bytes(random);

    // AT_PLATFORM is optional: an emulated architecture does not always supply
    // one, and passing a null pointer is worse than omitting the entry.
    let platform_ptr = unsafe { getauxval(at::PLATFORM) } as *const u8;
    let platform_at = if platform_ptr.is_null() {
        None
    } else {
        let mut n = 0usize;
        // Bounded: a platform string is a few bytes and an unterminated one
        // would otherwise be walked until it faults.
        while n < 64 && unsafe { *platform_ptr.add(n) } != 0 {
            n += 1;
        }
        let s = unsafe { core::slice::from_raw_parts(platform_ptr, n + 1) };
        Some(st.bytes(s))
    };

    let phdr = plan
        .phdr_vaddr()
        .ok_or(Refusal::NoAuxv("a mapped program header table"))?;
    let phdr = m.base + usize::try_from(phdr).map_err(|_| Refusal::OutOfRange("header address"))?;

    // ⛔ THE KERNEL'S OWN VECTOR, carried through, in MEMORY order. Only the
    // entries that describe the IMAGE are replaced; everything else - the vDSO,
    // the CPU feature words, the identity, whatever the kernel adds next - goes
    // to the payload exactly as this process received it. `own_auxv` says why,
    // with what the enumerated version was measured to drop.
    let mut auxv = own_auxv();
    if auxv.is_empty() {
        // ⚠ No `/proc` to read. The enumerated vector is the lesser answer and
        // it is not silently the same one: it carries what the ABI requires and
        // nothing else, and `AT_HWCAP` comes from the libc rather than the
        // kernel. A bed without /proc gets a payload that starts.
        auxv = vec![
            (at::PHDR, 0),
            (at::PHENT, 0),
            (at::PHNUM, 0),
            (at::PAGESZ, page_size() as u64),
            (at::BASE, 0),
            (at::FLAGS, 0),
            (at::ENTRY, 0),
            (at::UID, crate::sys::uid() as u64),
            (at::EUID, crate::sys::euid() as u64),
            (at::GID, unsafe { getgid() } as u64),
            (at::EGID, unsafe { getegid() } as u64),
            (at::HWCAP, unsafe { getauxval(at::HWCAP) }),
            (at::CLKTCK, unsafe { sysconf(SC_CLK_TCK) } as u64),
            (at::SECURE, unsafe { getauxval(at::SECURE) }),
            (at::RANDOM, 0),
            (at::EXECFN, 0),
        ];
    }

    // What describes the new image rather than this process.
    auxv_set(&mut auxv, at::PHDR, phdr as u64);
    auxv_set(&mut auxv, at::PHENT, plan.phentsize as u64);
    auxv_set(&mut auxv, at::PHNUM, plan.phnum as u64);
    auxv_set(&mut auxv, at::BASE, interp_base.unwrap_or(0) as u64);
    auxv_set(&mut auxv, at::ENTRY, m.entry as u64);
    auxv_set(&mut auxv, at::FLAGS, 0);
    auxv_set(&mut auxv, at::EXECFN, path_at as u64);
    // ⚠ These two are COPIES on the payload's own stack rather than the
    // pointers this process was given. Both would work - the old stack stays
    // mapped - and a copy is what survives anything that later does not.
    auxv_set(&mut auxv, at::RANDOM, random_at as u64);
    if let Some(p) = platform_at {
        auxv_set(&mut auxv, at::PLATFORM, p as u64);
    }

    // ⛔ The alignment pad goes in BEFORE the auxiliary vector, so that `argc`
    // - which is the last thing pushed and therefore the lowest address - lands
    // 16-byte aligned. The ABI requires that of the stack pointer at the entry
    // point, and a libc that uses aligned SSE stores in its startup faults
    // without it rather than reporting anything.
    // ⚠ `+ 1` for the AT_NULL terminator, which is not in the list.
    let after =
        ((auxv.len() + 1) * 2 + arg_at.len() + env_at.len() + 3) * core::mem::size_of::<usize>();
    while (st.rev.len() + after) % 16 != 0 {
        st.rev.push(0);
    }

    // ⛔ The terminator FIRST, because the first thing pushed lands at the
    // highest address, and then the entries from last to first - so the vector
    // comes out in the kernel's own order with AT_NULL above it.
    st.word(0);
    st.word(at::NULL as usize);
    for (kind, value) in auxv.iter().rev() {
        st.word(*value as usize);
        st.word(*kind as usize);
    }
    st.word(0);
    for a in env_at {
        st.word(a);
    }
    st.word(0);
    for a in arg_at {
        st.word(a);
    }
    st.word(argv.len());

    let mut data = st.rev;
    data.reverse();
    let sp = top - data.len();
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), sp as *mut u8, data.len()) };
    Ok(sp)
}

/// Set the stack pointer and jump. Never returns.
///
/// ⛔ It cannot return, and that is why it takes no arguments it could have
/// got wrong: after the stack pointer moves, this function's own frame is
/// gone. Everything it needs is in a register before the first instruction.
#[cfg(target_arch = "x86_64")]
unsafe fn jump(sp: usize, entry: usize) -> ! {
    // rdx must be zero: the ABI says it holds a function to register with
    // atexit, and libc calls whatever is there.
    core::arch::asm!(
        "mov rsp, {sp}",
        "xor rdx, rdx",
        "jmp {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn),
    )
}

#[cfg(target_arch = "aarch64")]
unsafe fn jump(sp: usize, entry: usize) -> ! {
    core::arch::asm!(
        "mov sp, {sp}",
        "mov x0, xzr",
        "br {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn),
    )
}

#[cfg(target_arch = "riscv64")]
unsafe fn jump(sp: usize, entry: usize) -> ! {
    core::arch::asm!(
        "mv sp, {sp}",
        "mv a0, zero",
        "jr {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn),
    )
}

/// What the caller wants started, and how.
pub struct Request {
    /// The image.
    pub program: std::path::PathBuf,
    /// The whole argument vector, argv0 included.
    pub argv: Vec<CString>,
    /// The environment, as `NAME=VALUE`.
    pub envp: Vec<CString>,
    /// A loader to use instead of the image's own `PT_INTERP`.
    ///
    /// ⭐ This is the field the launcher exists to set. `Some(path)` starts
    /// the payload against the bundle's loader without the payload's own
    /// `PT_INTERP` being rewritten on disk, which is the whole point;
    /// `None` uses whatever the image names.
    pub interpreter: Option<std::path::PathBuf>,
    /// Refuse to use an interpreter at all, for an image that needs none.
    pub no_interpreter: bool,
}

/// Start the payload. On success this never returns.
pub fn exec(req: &Request) -> Refusal {
    // ⚠ The Ok arm is uninhabited, so it is matched away rather than returned:
    // `start` cannot succeed and come back, and the type says so.
    match start(req) {
        Err(e) => e,
        Ok(never) => match never {},
    }
}

fn start(req: &Request) -> Result<core::convert::Infallible, Refusal> {
    let plan = elf::load_plan(&req.program).map_err(|e| Refusal::Unreadable(e.to_string()))?;

    // The interpreter, when there is one to map. ⚠ It is mapped like any other
    // image and given its own base; the payload's `AT_BASE` is what tells it
    // where it landed, and the jump goes to ITS entry point rather than the
    // payload's.
    let interp_path: Option<std::path::PathBuf> = if req.no_interpreter {
        None
    } else if let Some(p) = &req.interpreter {
        Some(p.clone())
    } else {
        plan.interp.as_ref().map(std::path::PathBuf::from)
    };

    // ⭐ The interpreter is mapped FIRST. mmap hands out addresses from the top
    // down, so the second mapping lands below the first, and the payload then
    // sits below its interpreter, the order the kernel gives them.
    // scripts/common/uexec-stack.sh reads that order as `map-order`.
    let mut interp_mapped = None;
    if let Some(p) = &interp_path {
        let iplan = elf::load_plan(p).map_err(|e| Refusal::Unreadable(e.to_string()))?;
        let im = map_image(p, &iplan)?;
        // ⛔ The interpreter is not relocated here either. It is built to
        // relocate itself before it touches anything, which is what makes it
        // the thing that can bootstrap everything else.
        protect(&iplan, im)?;
        interp_mapped = Some(im);
    }
    // No relocation here, with or without an interpreter: the kernel applies
    // none, and every image it can start relocates itself.
    let mapped = map_image(&req.program, &plan)?;
    protect(&plan, mapped)?;

    let path = CString::new(req.program.to_string_lossy().as_bytes())
        .map_err(|_| Refusal::Unreadable("the image's path contains a NUL".to_string()))?;
    let sp = build_stack(
        mapped,
        &plan,
        interp_mapped.map(|m| m.base),
        &path,
        &req.argv,
        &req.envp,
    )?;
    let entry = interp_mapped.map(|m| m.entry).unwrap_or(mapped.entry);
    unsafe { jump(sp, entry) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protections_follow_the_segment_flags() {
        // ⛔ The defect this module exists not to have: the subject maps every
        // segment writable and says so in a TODO. A read-execute segment must
        // come out read-execute.
        assert_eq!(prot_of(PF_R | PF_X), PROT_READ | PROT_EXEC);
        assert_eq!(prot_of(PF_R | PF_W), PROT_READ | PROT_WRITE);
        assert_eq!(prot_of(PF_R), PROT_READ);
        assert_eq!(prot_of(0), PROT_NONE);
        assert_eq!(prot_of(PF_R | PF_W) & PROT_EXEC, 0);
        assert_eq!(prot_of(PF_R | PF_X) & PROT_WRITE, 0);
    }

    #[test]
    fn rounding_is_to_the_page() {
        assert_eq!(round_down(4096, 4096), 4096);
        assert_eq!(round_down(4097, 4096), 4096);
        assert_eq!(round_down(0, 4096), 0);
        assert_eq!(round_up(0, 4096), 0);
        assert_eq!(round_up(1, 4096), 4096);
        assert_eq!(round_up(4096, 4096), 4096);
        assert_eq!(round_up(4097, 4096), 8192);
    }

    /// ⭐ The stack's addresses are what every string in it is found by, so the
    /// arithmetic is asserted here rather than only in a run.
    #[test]
    fn the_stack_addresses_what_it_pushed() {
        let top = 0x1000_0000usize;
        let mut st = Stack::new(top);
        let a = st.cstr(&CString::new("hello").unwrap());
        assert_eq!(a, top - 6, "six bytes with the NUL");
        let b = st.cstr(&CString::new("xy").unwrap());
        assert_eq!(b, top - 9);
        // The bytes are held reversed, and reversing them puts the last push
        // at the lowest address.
        let mut data = st.rev.clone();
        data.reverse();
        assert_eq!(&data[..3], b"xy\0");
        assert_eq!(&data[3..], b"hello\0");
        assert_eq!(top - data.len(), b);
    }

    /// ⛔ The alignment the ABI requires of the stack pointer, asserted over
    /// argument and environment counts that produce every residue. A libc using
    /// aligned stores in its startup faults without it, with no message.
    #[test]
    fn argc_lands_sixteen_byte_aligned() {
        let word = core::mem::size_of::<usize>();
        for args in 1..8usize {
            for envs in 0..8usize {
                for auxv in 16..19usize {
                    for strings in 0..40usize {
                        let after = (auxv * 2 + args + envs + 3) * word;
                        let mut len = strings;
                        while (len + after) % 16 != 0 {
                            len += 1;
                        }
                        assert_eq!(
                            (len + after) % 16,
                            0,
                            "args {args} envs {envs} auxv {auxv} strings {strings}"
                        );
                    }
                }
            }
        }
    }

    // ⛔ The kernel is the source, and these two hold the shape of that.
    #[test]
    fn the_process_reads_its_own_auxiliary_vector() {
        let v = own_auxv();
        // ⚠ Not asserted to be non-empty: a bed without /proc is a real state
        // and the caller has a fallback for it. What IS asserted is that
        // nothing read back is a terminator, because the walk stops at one -
        // a vector carrying AT_NULL would mean the loop never ended.
        assert!(v.iter().all(|(t, _)| *t != at::NULL), "AT_NULL survived the walk");
        if !v.is_empty() {
            // ⭐ Every kernel supplies these three, so an empty-ish read that
            // returned junk would not look like a vector.
            assert!(
                v.iter().any(|(t, _)| *t == at::PAGESZ),
                "no AT_PAGESZ in {v:?}"
            );
        }
    }

    #[test]
    fn setting_an_entry_keeps_the_kernels_order() {
        let mut v = vec![(at::PHDR, 1), (at::PHENT, 2), (at::PHNUM, 3)];
        auxv_set(&mut v, at::PHENT, 56);
        // ⛔ IN PLACE. An entry appended instead would move AT_PHDR to the end
        // of a vector the kernel wrote it near the front of, and the comparison
        // against the kernel would differ on every run for no defect.
        assert_eq!(v, vec![(at::PHDR, 1), (at::PHENT, 56), (at::PHNUM, 3)]);
        auxv_set(&mut v, at::EXECFN, 9);
        assert_eq!(v.last(), Some(&(at::EXECFN, 9)), "a new tag goes at the end");
    }

    #[test]
    fn every_refusal_says_something_different() {
        let all = [
            Refusal::Unreadable("x".into()),
            Refusal::NotRelocatable,
            Refusal::OutOfRange("span"),
            Refusal::MapFailed("stack"),
            Refusal::NoAuxv("AT_RANDOM"),
        ];
        let mut seen: Vec<String> = Vec::new();
        for r in &all {
            let m = r.message();
            assert!(!m.is_empty());
            assert!(!seen.contains(&m), "two refusals share {m:?}");
            seen.push(m);
        }
    }
}
