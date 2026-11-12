//! The entry of a payload: podvm becomes the payload in its own process.

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::elf::{self, Image};
use crate::handler;
use crate::state::{self, EXE, ROOT, SELF};
use crate::sys::{self, nr, sc, sc6};

const AT_NULL: u64 = 0;
const AT_PHDR: u64 = 3;
const AT_PHENT: u64 = 4;
const AT_PHNUM: u64 = 5;
const AT_BASE: u64 = 7;
const AT_ENTRY: u64 = 9;
const AT_SECURE: u64 = 23;
const AT_PLATFORM: u64 = 15;
const AT_RANDOM: u64 = 25;
const AT_EXECFN: u64 = 31;
const AT_EXECFD: u64 = 2;
const STACK: u64 = 8 << 20;
const BRK_RESERVE: u64 = 1 << 30;

/// The program file in the root, opened for reading.
fn open_in_root(guest: &[u8]) -> Result<i64, String> {
    let mut z = guest.to_vec();
    z.push(0);
    let fd = crate::path::open_in(&z, guest.len(), sys::O_RDONLY | sys::O_CLOEXEC, 0, 0);
    if fd < 0 {
        return Err(format!("{}: errno {}", String::from_utf8_lossy(guest), -fd));
    }
    Ok(fd)
}

/// The guest path of podvm's interposer (`crates/podvm/src/interpose.rs`,
/// `GUEST_PATH`).
const INTERPOSER: &[u8] = b"/.podvm/interpose.so";

/// `LD_PRELOAD` less podvm's interposer: on this rung the handler sees
/// each call, and the interposer would hand it host paths (TODO/enter.md
/// T-1731). None where nothing stays of the variable; each other variable
/// as it is.
fn without_interposer(e: Vec<u8>) -> Option<Vec<u8>> {
    let Some(v) = e.strip_prefix(b"LD_PRELOAD=") else {
        return Some(e);
    };
    let kept: Vec<&[u8]> = v
        .split(|c| *c == b':' || *c == b' ')
        .filter(|p| !p.is_empty() && *p != INTERPOSER)
        .collect();
    if kept.is_empty() {
        return None;
    }
    let mut out = b"LD_PRELOAD=".to_vec();
    out.extend(kept.join(&b':'));
    Some(out)
}

/// The volumes of `PODVM_MAPS` in `envp` (`guest:host` pairs joined by
/// commas; the root pair `/:ROOTFS` is the root): each host directory
/// opened above the payload's usual range (TODO/enter.md T-1724).
fn volumes(envp: &[OsString]) -> Result<(), String> {
    let Some(table) = envp
        .iter()
        .find_map(|e| e.as_bytes().strip_prefix(b"PODVM_MAPS="))
    else {
        return Ok(());
    };
    let mut n = 0usize;
    for pair in table.split(|&c| c == b',').filter(|p| !p.is_empty()) {
        let Some(cut) = pair.iter().position(|&c| c == b':') else {
            continue;
        };
        let (guest, host) = (&pair[..cut], &pair[cut + 1..]);
        if guest == b"/" {
            continue;
        }
        if n == state::MAX_VOLS {
            return Err(format!("more than {} volumes", state::MAX_VOLS));
        }
        let host = std::fs::canonicalize(std::ffi::OsStr::from_bytes(host))
            .map_err(|e| format!("the volume {}: {e}", String::from_utf8_lossy(pair)))?;
        let mut z = host.as_os_str().as_bytes().to_vec();
        z.push(0);
        let fd = sys::openat2(
            sys::AT_FDCWD,
            &z,
            sys::O_PATH | sys::O_DIRECTORY | sys::O_CLOEXEC,
            0,
            0,
        );
        if fd < 0 {
            return Err(format!("the volume {}: errno {}", host.display(), -fd));
        }
        // SAFETY: fcntl and close read no memory.
        let high = unsafe { sc(nr::FCNTL, fd, 1030, 1001) };
        let fd = if high >= 0 {
            sys::close(fd);
            high
        } else {
            fd
        };
        let v = &state::VOLS[n];
        v.guest.set(guest.strip_suffix(b"/").unwrap_or(guest));
        v.host.set(&z[..z.len() - 1]);
        v.fd.store(fd, Ordering::Relaxed);
        n += 1;
    }
    state::NVOLS.store(n, Ordering::Release);
    Ok(())
}

fn read_head(fd: i64) -> [u8; 256] {
    let mut h = [0u8; 256];
    // SAFETY: h lives for the call.
    unsafe { sc6(nr::PREAD64, fd, h.as_mut_ptr() as i64, 256, 0, 0, 0) };
    h
}

/// The program that maps: its guest path, its open descriptor, its head
/// and the argv that it gets.
type Resolved = (Vec<u8>, i64, [u8; 256], Vec<Vec<u8>>);

/// The program to map and its argv: a script names its interpreter, up to
/// four levels, as the kernel allows.
fn resolve(mut exe: Vec<u8>, mut argv: Vec<Vec<u8>>) -> Result<Resolved, String> {
    for _ in 0..5 {
        let fd = open_in_root(&exe)?;
        let head = read_head(fd);
        let Some((interp, arg)) = elf::shebang(&head) else {
            return Ok((exe, fd, head, argv));
        };
        sys::close(fd);
        let mut next = vec![interp.clone()];
        next.extend(arg);
        next.push(exe.clone());
        next.extend(argv.into_iter().skip(1));
        argv = next;
        exe = interp;
    }
    Err("too many levels of #! interpreters".into())
}

/// The program to map: its descriptor, its head and its argv.
type Loaded = (i64, [u8; 256], Vec<Vec<u8>>);

/// TODO/enter.md T-1651. An ELF that the emulator serves runs under it:
/// the emulator (a static program of the host) is the image, with
/// `-0 ARGV0 GUEST ARGS...`. Each of its own paths resolves in the root,
/// so it needs no `-L`, and a miss is ENOENT.
fn emulated(prog: &[u8], pfd: i64, head: [u8; 256], argv: Vec<Vec<u8>>) -> Result<Loaded, String> {
    if head.get(..4) != Some(b"\x7fELF".as_slice()) {
        return Ok((pfd, head, argv));
    }
    let Some(emu) = state::emulator_for(u16::from_le_bytes([head[18], head[19]])) else {
        return Ok((pfd, head, argv));
    };
    sys::close(pfd);
    let mut z = emu.to_vec();
    z.push(0);
    let efd = sys::openat2(sys::AT_FDCWD, &z, sys::O_RDONLY | sys::O_CLOEXEC, 0, 0);
    if efd < 0 {
        return Err(format!(
            "the emulator {}: errno {}",
            String::from_utf8_lossy(emu),
            -efd
        ));
    }
    let base = emu.rsplit(|&c| c == b'/').next().unwrap_or(emu).to_vec();
    // The task's name is the emulator's, as a kernel's binfmt handler
    // gives it: the probe reads it (T-0506).
    let mut name = [0u8; 16];
    let n = base.len().min(15);
    name[..n].copy_from_slice(&base[..n]);
    // SAFETY: PR_SET_NAME reads 16 bytes of name.
    unsafe { sc(nr::PRCTL, 15, name.as_ptr() as i64, 0) };
    let mut out = vec![
        base,
        b"-0".to_vec(),
        argv.first().cloned().unwrap_or_else(|| prog.to_vec()),
        prog.to_vec(),
    ];
    out.extend(argv.into_iter().skip(1));
    Ok((efd, read_head(efd), out))
}

/// The auxiliary vector of podvm itself, as pairs.
fn own_auxv() -> Vec<(u64, u64)> {
    let b = std::fs::read("/proc/self/auxv").unwrap_or_default();
    b.as_chunks::<16>()
        .0
        .iter()
        .map(|c| {
            (
                u64::from_le_bytes(c[..8].try_into().unwrap_or([0; 8])),
                u64::from_le_bytes(c[8..].try_into().unwrap_or([0; 8])),
            )
        })
        .take_while(|(k, _)| *k != AT_NULL)
        .collect()
}

/// The initial stack: argc, argv, envp and the auxiliary vector, with the
/// strings above them. The stack pointer for the entry.
fn stack(img: &Image, argv: &[Vec<u8>], envp: &[Vec<u8>], execfn: &[u8]) -> Result<u64, String> {
    // SAFETY: an anonymous map for the stack.
    let base = unsafe { sc6(nr::MMAP, 0, STACK as i64, 3, 0x22 | 0x2_0000, -1, 0) };
    if base < 0 {
        return Err(format!("the stack: errno {}", -base));
    }
    let top = base as u64 + STACK;
    let mut at = top;
    let mut put = |bytes: &[u8]| -> u64 {
        at -= bytes.len() as u64 + 1;
        // SAFETY: at lies in the new stack, below what was written.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len());
            *((at + bytes.len() as u64) as *mut u8) = 0;
        }
        at
    };
    let argp: Vec<u64> = argv.iter().map(|a| put(a)).collect();
    let envp_p: Vec<u64> = envp.iter().map(|e| put(e)).collect();
    let execfn_p = put(execfn);
    let platform = put(b"x86_64");
    let mut random = [0u8; 16];
    // SAFETY: random lives for the call.
    unsafe { sc(318, random.as_mut_ptr() as i64, 16, 0) };
    at -= 16;
    // SAFETY: at lies in the new stack.
    unsafe { core::ptr::copy_nonoverlapping(random.as_ptr(), at as *mut u8, 16) };
    let random_p = at;
    let mut aux: Vec<(u64, u64)> = own_auxv()
        .into_iter()
        .filter(|(k, _)| {
            ![
                AT_PHDR,
                AT_PHENT,
                AT_PHNUM,
                AT_BASE,
                AT_ENTRY,
                AT_EXECFN,
                AT_EXECFD,
                AT_RANDOM,
                AT_PLATFORM,
                AT_SECURE,
            ]
            .contains(k)
        })
        .collect();
    aux.extend([
        (AT_PHDR, img.phdr),
        (AT_PHENT, 56),
        (AT_PHNUM, img.phnum),
        (AT_BASE, img.base),
        (AT_ENTRY, img.entry),
        (AT_SECURE, 0),
        (AT_RANDOM, random_p),
        (AT_EXECFN, execfn_p),
        (AT_PLATFORM, platform),
        (AT_NULL, 0),
    ]);
    let mut words: Vec<u64> = vec![argp.len() as u64];
    words.extend(&argp);
    words.push(0);
    words.extend(&envp_p);
    words.push(0);
    for (k, v) in aux {
        words.extend([k, v]);
    }
    let mut sp = (at - words.len() as u64 * 8) & !15;
    if sp < base as u64 + 65536 {
        return Err("argv and the environment do not fit the stack".into());
    }
    let start = sp;
    for w in words {
        // SAFETY: sp lies in the new stack, below the strings.
        unsafe { *(sp as *mut u64) = w };
        sp += 8;
    }
    Ok(start)
}

/// The range of podvm's own code that holds the handler, from
/// `/proc/self/maps`.
fn region() -> Result<(usize, usize), String> {
    let here = handler::on_sigsys as *const () as usize;
    let maps =
        std::fs::read_to_string("/proc/self/maps").map_err(|e| format!("/proc/self/maps: {e}"))?;
    for line in maps.lines() {
        let range = line.split(' ').next().unwrap_or("");
        let Some((a, b)) = range.split_once('-') else {
            continue;
        };
        let (Ok(a), Ok(b)) = (usize::from_str_radix(a, 16), usize::from_str_radix(b, 16)) else {
            continue;
        };
        if a <= here && here < b {
            return Ok((a, b - a));
        }
    }
    Err("the handler is in no mapping of /proc/self/maps".into())
}

/// Become the payload: `exe` is its guest path, `argv` and `envp` its own.
/// Returns only an error, before anything of the payload ran.
pub fn enter(
    root: &Path,
    exe: &[u8],
    emu: Option<&[u8]>,
    argv: Vec<OsString>,
    envp: Vec<OsString>,
) -> String {
    if let Some(spec) = emu
        && !state::set_emulator(spec)
    {
        return format!(
            "the emulator {:?} is not MACHINE:HOSTPATH",
            String::from_utf8_lossy(spec)
        );
    }
    match try_enter(root, exe, argv, envp) {
        Ok(never) => match never {},
        Err(e) => e,
    }
}

fn try_enter(
    root: &Path,
    exe: &[u8],
    argv: Vec<OsString>,
    envp: Vec<OsString>,
) -> Result<std::convert::Infallible, String> {
    let root = std::fs::canonicalize(root).map_err(|e| format!("{}: {e}", root.display()))?;
    if !ROOT.set(root.as_os_str().as_bytes()) {
        return Err("the root's path is too long".into());
    }
    let me = std::fs::read_link("/proc/self/exe").map_err(|e| format!("/proc/self/exe: {e}"))?;
    SELF.set(me.as_os_str().as_bytes());
    let mut z = ROOT.get().to_vec();
    z.push(0);
    let fd = sys::openat2(
        sys::AT_FDCWD,
        &z,
        sys::O_PATH | sys::O_DIRECTORY | sys::O_CLOEXEC,
        0,
        0,
    );
    if fd < 0 {
        return Err(format!("{}: errno {}", root.display(), -fd));
    }
    // Above the payload's usual range: F_DUPFD_CLOEXEC from 1000, or the
    // highest descriptor the limit allows.
    // SAFETY: fcntl and close read no memory.
    let high = unsafe { sc(nr::FCNTL, fd, 1030, 1000) };
    let fd = if high >= 0 {
        sys::close(fd);
        high
    } else {
        fd
    };
    state::ROOT_FD.store(fd, Ordering::Relaxed);
    volumes(&envp)?;
    let argv: Vec<Vec<u8>> = argv.into_iter().map(OsStringExt::into_vec).collect();
    let (prog, pfd, head, argv) = resolve(exe.to_vec(), argv)?;
    EXE.set(&prog);
    let (pfd, head, argv) = emulated(&prog, pfd, head, argv)?;
    // TODO/enter.md T-1731: a dynamic program's interpreter, from the root.
    let mut img = elf::load(pfd, &head, open_in_root)
        .map_err(|e| format!("{}: {e}", String::from_utf8_lossy(&prog)))?;
    if img.start == img.entry {
        // A static program: no interpreter, so AT_BASE is 0.
        img.base = 0;
    }
    sys::close(pfd);
    // The break: a reserved range after the program, opened as it grows.
    // SAFETY: an anonymous map with no access.
    let brk = unsafe {
        sc6(
            nr::MMAP,
            img.end as i64,
            BRK_RESERVE as i64,
            0,
            0x22 | 0x4000 | 0x10_0000,
            -1,
            0,
        )
    };
    if brk >= 0 {
        state::BRK_BASE.store(brk as usize, Ordering::Relaxed);
        state::BRK_CUR.store(brk as usize, Ordering::Relaxed);
        state::BRK_END.store((brk as u64 + BRK_RESERVE) as usize, Ordering::Relaxed);
    }
    let envp: Vec<Vec<u8>> = envp
        .into_iter()
        .map(OsStringExt::into_vec)
        .filter_map(without_interposer)
        .collect();
    let execfn = exe.to_vec();
    let sp = stack(&img, &argv, &envp, &execfn)?;
    if !crate::pool::init() {
        return Err("the pool of path buffers cannot be mapped".into());
    }
    let (start, len) = region()?;
    state::REGION_START.store(start, Ordering::Relaxed);
    state::REGION_LEN.store(len, Ordering::Relaxed);
    crate::signals::reset_signals();
    // Rust's own signal stack is small; the payload sets its own.
    let disable = [0u64, 2, 0];
    // SAFETY: a stack_t of SS_DISABLE.
    unsafe { sc(131, disable.as_ptr() as i64, 0, 0) };
    let r = handler::arm();
    if r < 0 {
        return Err(format!("prctl(PR_SET_SYSCALL_USER_DISPATCH): errno {}", -r));
    }
    // SAFETY: the payload's entry, with its stack; nothing of podvm runs
    // on this thread after this but the handler.
    unsafe {
        core::arch::asm!(
            "mov rsp, rdi",
            "xor edx, edx",
            "xor ebp, ebp",
            "jmp rsi",
            in("rdi") sp,
            in("rsi") img.start,
            options(noreturn)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::without_interposer;

    #[test]
    fn the_interposer_leaves_ld_preload_and_each_other_object_stays() {
        let f = |s: &str| {
            without_interposer(s.as_bytes().to_vec()).map(|v| String::from_utf8(v).unwrap())
        };
        assert_eq!(f("LD_PRELOAD=/.podvm/interpose.so"), None);
        assert_eq!(
            f("LD_PRELOAD=/.podvm/interpose.so:/usr/lib/libjemalloc.so"),
            Some("LD_PRELOAD=/usr/lib/libjemalloc.so".into())
        );
        assert_eq!(f("HOME=/root"), Some("HOME=/root".into()));
    }
}
