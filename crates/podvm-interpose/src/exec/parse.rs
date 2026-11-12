//! The pure parsers of the head of an executable file, T-1646.
//!
//! [`TODO/interpose.md`](../../../../TODO/interpose.md) T-1646 and
//! [`TODO/gate.md`](../../../../TODO/gate.md) T-1684. Each function reads
//! bytes only: no libc call, no allocation, no crate item. So the fuzz
//! targets of `crates/podvm-interpose/fuzz/` include this file as it is,
//! and never link the interposer, whose exports would replace libc in the
//! fuzzer. `head.rs` re-exports each item.

/// The bytes of a file head that the parsers read: the kernel's own
/// `BINPRM_BUF_SIZE`, which bounds a `#!` line. A 64-bit ELF header is 64
/// bytes, so it fits.
pub const HEAD: usize = 256;

/// The size of one 64-bit program header.
pub const PHENT: usize = 56;

/// `PT_INTERP`.
const PT_INTERP: u32 = 3;

/// The ELF machine of this object (`EM_X86_64` 62, `EM_AARCH64` 183).
#[cfg(target_arch = "x86_64")]
const EM_SELF: u16 = 62;
/// The machine of this object: an exec of another machine fails by name.
#[cfg(target_arch = "aarch64")]
const EM_SELF: u16 = 183;

/// What the kernel does with a file, as far as the exec family must know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `#!` line.
    Script,
    /// A native 64-bit ELF executable. Its program headers are `phnum`
    /// entries at `phoff`.
    Elf { phoff: u64, phnum: usize },
    /// An ELF file of another class, byte order or machine. The kernel
    /// decides; T-1646 routes only the native machine.
    Foreign,
    /// Neither: the kernel answers `ENOEXEC`, or a `binfmt_misc` handler
    /// takes it.
    Other,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut v = [0u8; 8];
    v.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(v)
}

/// Classify the first bytes of a file. `head` holds the bytes read, no more.
pub fn kind(head: &[u8]) -> Kind {
    if head.len() >= 2 && head[0] == b'#' && head[1] == b'!' {
        return Kind::Script;
    }
    if head.len() < 4 || &head[..4] != b"\x7fELF" {
        return Kind::Other;
    }
    // The header fields this reads: class (4), data (5), type (16),
    // machine (18), phoff (32), phentsize (54), phnum (56).
    if head.len() < 64 || head[4] != 2 || head[5] != 1 || u16_at(head, 18) != EM_SELF {
        return Kind::Foreign;
    }
    // ET_EXEC or ET_DYN. Another type is not a program.
    let ty = u16_at(head, 16);
    if (ty != 2 && ty != 3) || u16_at(head, 54) as usize != PHENT {
        return Kind::Other;
    }
    Kind::Elf {
        phoff: u64_at(head, 32),
        phnum: u16_at(head, 56) as usize,
    }
}

/// The `PT_INTERP` entry in a table of program headers: its file offset and
/// its size. `None` is a static executable.
pub fn interp_entry(phdrs: &[u8]) -> Option<(u64, u64)> {
    let mut at = 0usize;
    while at + PHENT <= phdrs.len() {
        if u32_at(phdrs, at) == PT_INTERP {
            return Some((u64_at(phdrs, at + 8), u64_at(phdrs, at + 32)));
        }
        at += PHENT;
    }
    None
}

/// The interpreter name and the optional argument of a `#!` line, as
/// ranges into the head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shebang {
    pub interp: (usize, usize),
    pub arg: Option<(usize, usize)>,
}

/// Parse a `#!` line as `fs/binfmt_script.c` does.
///
/// The line ends at the first newline. Blanks (space and tab) before the
/// name are skipped. The name ends at a blank. The rest of the line, with
/// its outer blanks removed, is ONE argument, or none where it is empty.
/// Where the head holds no newline, the name must end at a blank inside
/// the head, or the kernel refuses with `ENOEXEC`; a long argument is cut
/// at the end of the head, as the kernel cuts it. `None` is that refusal.
/// `whole` tells that the head holds the whole file: the kernel pads a
/// short file with NUL bytes, so its end also ends the name.
pub fn shebang(head: &[u8], whole: bool) -> Option<Shebang> {
    if head.len() < 2 || head[0] != b'#' || head[1] != b'!' {
        return None;
    }
    let blank = |c: u8| c == b' ' || c == b'\t';
    let nl = head.iter().position(|&c| c == b'\n');
    let mut end = nl.unwrap_or(head.len());
    while end > 2 && blank(head[end - 1]) {
        end -= 1;
    }
    let mut at = 2usize;
    while at < end && blank(head[at]) {
        at += 1;
    }
    let start = at;
    while at < end && !blank(head[at]) && head[at] != 0 {
        at += 1;
    }
    if at == start {
        return None;
    }
    if nl.is_none() && at == head.len() && !whole {
        return None;
    }
    let interp = (start, at);
    while at < end && blank(head[at]) {
        at += 1;
    }
    let arg = (at < end).then_some((at, end));
    Some(Shebang { interp, arg })
}

/// Is `name` the file name of a glibc loader older than 2.33, which does
/// not know `--argv0`?
///
/// glibc installed its loader as `ld-2.NN.so` up to 2.33, and as the
/// `PT_INTERP` name itself from 2.34. So the resolved file name tells the
/// version: `ld-2.31.so` is too old, `ld-2.33.so` and
/// `ld-linux-x86-64.so.2` take the option. The musl loader takes it too.
pub fn loader_lacks_argv0(name: &[u8]) -> bool {
    let Some(rest) = name.strip_prefix(b"ld-2.") else {
        return false;
    };
    let Some(digits) = rest.strip_suffix(b".so") else {
        return false;
    };
    if digits.is_empty() || digits.len() > 3 || !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let minor = digits.iter().fold(0u32, |v, d| v * 10 + (d - b'0') as u32);
    minor < 33
}

/// Does the guest loader named `name` (its resolved file) take
/// `--inhibit-cache`, so the host's `/etc/ld.so.cache` plays no part
/// (TODO/enter.md T-1719)? glibc 2.16 and later do: `ld-2.NN.so` with NN of
/// 16 or more, and the `ld-linux*` file of glibc 2.34 and later. musl has
/// no cache and no such option.
pub fn loader_takes_inhibit_cache(name: &[u8]) -> bool {
    if name.starts_with(b"ld-linux") {
        return true;
    }
    let Some(digits) = name
        .strip_prefix(b"ld-2.")
        .and_then(|r| r.strip_suffix(b".so"))
    else {
        return false;
    };
    !digits.is_empty()
        && digits.len() <= 3
        && digits.iter().all(u8::is_ascii_digit)
        && digits.iter().fold(0u32, |v, d| v * 10 + (d - b'0') as u32) >= 16
}

/// The final element of a path.
pub fn basename(p: &[u8]) -> &[u8] {
    match p.iter().rposition(|&c| c == b'/') {
        Some(i) => &p[i + 1..],
        None => p,
    }
}
