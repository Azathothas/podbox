use super::*;

/// Escape a path for a mount-table line: space, tab, newline and backslash
/// take the octal escapes the kernel's own table uses, so a path with a
/// space parses back to one field rather than two.
pub fn escape_mount(s: &[u8], out: &mut [u8; 256]) -> usize {
    let mut at = 0usize;
    for c in s {
        let esc: Option<&[u8; 4]> = match c {
            b' ' => Some(b"\\040"),
            b'\t' => Some(b"\\011"),
            b'\n' => Some(b"\\012"),
            b'\\' => Some(b"\\134"),
            _ => None,
        };
        if let Some(e) = esc {
            if at + 4 > out.len() {
                return at;
            }
            out[at..at + 4].copy_from_slice(e);
            at += 4;
        } else {
            if at + 1 > out.len() {
                return at;
            }
            out[at] = *c;
            at += 1;
        }
    }
    at
}

/// One recorded T-0708 emulated mount, as offsets into the scan's staging
/// area.
///
/// ⛔ Offsets, not borrows: the scan keeps staging path bytes while it
/// takes records, and a borrow handed out early would forbid every later
/// write. The renderers slice these against the area they already hold.
#[derive(Clone, Copy)]
pub struct RecMount {
    pub src_start: usize,
    pub src_len: usize,
    pub tgt_start: usize,
    pub tgt_len: usize,
    pub ro: bool,
}

impl RecMount {
    /// The source bytes staged for this record.
    pub fn src<'a>(&self, area: &'a [u8]) -> &'a [u8] {
        &area[self.src_start..self.src_start + self.src_len]
    }

    /// The target bytes staged for this record.
    pub fn tgt<'a>(&self, area: &'a [u8]) -> &'a [u8] {
        &area[self.tgt_start..self.tgt_start + self.tgt_len]
    }
}

/// Append one line (no trailing newline accounting beyond it) to the
/// fixture, stopping whole lines at the buffer's end: a mount table that
/// ends mid-line is a corrupt table, and a shorter honest one beats it.
fn push_line(out: &mut [u8; FIXTURE_MAX], at: &mut usize, line: &[u8]) {
    if *at + line.len() + 1 > out.len() {
        return;
    }
    out[*at..*at + line.len()].copy_from_slice(line);
    *at += line.len();
    out[*at] = b'\n';
    *at += 1;
}

/// The fixture's ceiling: 8 KiB of stack. A payload with more emulated
/// mounts than fit keeps the root line and the first ones; the fixture is
/// an approximation TOOL.md section 10 licenses, and a bounded one.
pub const FIXTURE_MAX: usize = 8192;

/// Render the `mounts`/`mtab` fixture: the `/` line from live topology,
/// then the recorded emulated mounts.
pub fn render_mounts(
    fstype: Fstype,
    rw: bool,
    area: &[u8; 4096],
    mounts: &[RecMount],
    out: &mut [u8; FIXTURE_MAX],
) -> usize {
    let mut at = 0usize;
    let mut line = [0u8; 512];
    let opts = if rw { &b"rw"[..] } else { &b"ro"[..] };
    {
        let n = line_of_mounts(&mut line, b"podvm", b"/", fstype.word(), opts);
        push_line(out, &mut at, &line[..n]);
    }
    for m in mounts {
        let mopts = if m.ro { &b"ro"[..] } else { &b"rw"[..] };
        let n = line_of_mounts(&mut line, m.src(area), m.tgt(area), b"none", mopts);
        push_line(out, &mut at, &line[..n]);
    }
    at
}

fn line_of_mounts(
    out: &mut [u8; 512],
    src: &[u8],
    tgt: &[u8],
    fstype: &[u8],
    opts: &[u8],
) -> usize {
    let mut at = 0usize;
    let mut esc = [0u8; 256];
    let mut field = |bytes: &[u8]| {
        for c in bytes {
            if at < out.len() {
                out[at] = *c;
                at += 1;
            }
        }
    };
    let n = escape_mount(src, &mut esc);
    field(&esc[..n]);
    field(b" ");
    let n = escape_mount(tgt, &mut esc);
    field(&esc[..n]);
    field(b" ");
    field(fstype);
    field(b" ");
    field(opts);
    field(b" 0 0");
    at
}

/// Render the `mountinfo` fixture: ids are this table's own sequence from
/// 1, never anyone else's mount ids. The emulated mounts carry `0:0`
/// because an emulated mount has no device; the `/` line carries the live
/// one, which is the real topology the fixture is generated from.
pub fn render_mountinfo(
    major: u32,
    minor: u32,
    fstype: Fstype,
    rw: bool,
    area: &[u8; 4096],
    mounts: &[RecMount],
    out: &mut [u8; FIXTURE_MAX],
) -> usize {
    let mut at = 0usize;
    let mut line = [0u8; 768];
    let opts = if rw { &b"rw"[..] } else { &b"ro"[..] };
    {
        let n = line_of_mountinfo(
            &mut line,
            1,
            1,
            major,
            minor,
            b"/",
            b"/",
            opts,
            fstype.word(),
            b"podvm",
            opts,
        );
        push_line(out, &mut at, &line[..n]);
    }
    for (id, m) in (2u32..).zip(mounts.iter()) {
        let mopts = if m.ro { &b"ro"[..] } else { &b"rw"[..] };
        let n = line_of_mountinfo(
            &mut line,
            id,
            1,
            0,
            0,
            b"/",
            m.tgt(area),
            mopts,
            b"none",
            m.src(area),
            mopts,
        );
        push_line(out, &mut at, &line[..n]);
    }
    at
}

fn num(mut v: u32, out: &mut [u8; 768], at: &mut usize) {
    let mut digits = [0u8; 10];
    let mut d = 0usize;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        if *at < out.len() {
            out[*at] = digits[d];
            *at += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn line_of_mountinfo(
    out: &mut [u8; 768],
    id: u32,
    parent: u32,
    major: u32,
    minor: u32,
    root: &[u8],
    mp: &[u8],
    opts: &[u8],
    fstype: &[u8],
    source: &[u8],
    sup: &[u8],
) -> usize {
    let mut at = 0usize;
    let mut esc = [0u8; 256];
    num(id, out, &mut at);
    let mut field = |bytes: &[u8]| {
        if at < out.len() {
            out[at] = b' ';
            at += 1;
        }
        for c in bytes {
            if at < out.len() {
                out[at] = *c;
                at += 1;
            }
        }
    };
    // parent
    {
        let mut tmp = [0u8; 768];
        let mut t = 0usize;
        num(parent, &mut tmp, &mut t);
        field(&tmp[..t]);
    }
    // major:minor
    {
        let mut tmp = [0u8; 768];
        let mut t = 0usize;
        num(major, &mut tmp, &mut t);
        if t < tmp.len() {
            tmp[t] = b':';
            t += 1;
        }
        let mut t2 = t;
        // minor appended after the colon
        let mut digits = [0u8; 10];
        let mut d = 0usize;
        let mut v = minor;
        loop {
            digits[d] = b'0' + (v % 10) as u8;
            v /= 10;
            d += 1;
            if v == 0 {
                break;
            }
        }
        while d > 0 {
            d -= 1;
            if t2 < tmp.len() {
                tmp[t2] = digits[d];
                t2 += 1;
            }
        }
        field(&tmp[..t2]);
    }
    let n = escape_mount(root, &mut esc);
    field(&esc[..n]);
    let n = escape_mount(mp, &mut esc);
    field(&esc[..n]);
    field(opts);
    field(b"-");
    field(fstype);
    let n = escape_mount(source, &mut esc);
    field(&esc[..n]);
    field(sup);
    at
}

/// Scan the memo for complete T-0708 mount records, T-0413.
///
/// The fixed 32-byte layout is the tally's own: a header (`u64::MAX`,
/// op 2, flags, source chains, target chains) with its continuation
/// records contiguous behind it, exactly as `emulate::tally_mount` wrote
/// them. Anything else -- ownership records, another header, EOF, a short
/// read -- drops the open chain rather than half-reading it, which is the
/// host-side scan's own rule. Paths stage into `area`; records name
/// offsets into it, so the scan never borrows what it keeps writing.
/// Past either bound the scan stops taking records.
///
/// Returns the records taken.
///
/// # Safety
/// `fd` must be a readable descriptor on the memo file.
pub unsafe fn scan_mounts(fd: c_int, area: &mut [u8; 4096], recs: &mut [RecMount; 64]) -> usize {
    const REC: usize = crate::memo::RECORD;
    if unsafe { lseek(fd, 0, 0) } < 0 {
        return 0;
    }
    let mut buf = [0u8; 128 * REC];
    let mut taken = 0usize;
    let mut used = 0usize;
    // The open chain, if any: which side is arriving, how many records of
    // it are still owed, and the header's words behind it.
    let mut src_left = 0u32;
    let mut tgt_total = 0u32;
    let mut tgt_left = 0u32;
    let mut s_start = 0usize;
    let mut s_len = 0usize;
    let mut t_start = 0usize;
    let mut t_len = 0usize;
    let mut flags = 0u32;
    let drop_chain = |src_left: &mut u32, tgt_left: &mut u32| {
        *src_left = 0;
        *tgt_left = 0;
    };
    // Records arrive whole: the writer appends 32 bytes at a time under
    // O_APPEND, so a short read is EOF, never a torn record.
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        let mut at = 0usize;
        while at + REC <= n as usize {
            let r = &buf[at..at + REC];
            at += REC;
            let dev = u64::from_le_bytes(r[0..8].try_into().unwrap_or([0u8; 8]));
            let op = u64::from_le_bytes(r[8..16].try_into().unwrap_or([0u8; 8]));
            if dev != u64::MAX {
                // An ownership record: it also breaks any open chain, for
                // the writer's reason (a chain is contiguous or it is torn).
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            if op == crate::emulate::OP_MOUNT {
                let cs = u32::from_le_bytes(r[20..24].try_into().unwrap_or([0u8; 4]));
                let ct = u32::from_le_bytes(r[24..28].try_into().unwrap_or([0u8; 4]));
                flags = u32::from_le_bytes(r[16..20].try_into().unwrap_or([0u8; 4]));
                s_start = used;
                s_len = 0;
                t_start = used;
                t_len = 0;
                src_left = cs;
                tgt_total = ct;
                tgt_left = 0;
                if cs == 0 {
                    // No source side: the target side arrives next, or an
                    // empty mount completes at once.
                    tgt_left = ct;
                    if ct == 0 {
                        push_rec(recs, &mut taken, s_start, 0, t_start, 0, flags);
                    }
                }
                continue;
            }
            if op != crate::emulate::CONT {
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            let in_src = src_left > 0;
            let in_tgt = src_left == 0 && tgt_left > 0;
            if !in_src && !in_tgt {
                continue;
            }
            let piece = &r[16..32];
            if used + 16 > area.len() || taken >= recs.len() {
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            // The last record of a side may be NUL-padded; the padding is
            // not path.
            let mut k = 16usize;
            while k > 0 && piece[k - 1] == 0 {
                k -= 1;
            }
            if in_src && s_len == 0 {
                s_start = used;
            }
            if in_tgt && t_len == 0 {
                t_start = used;
            }
            area[used..used + k].copy_from_slice(&piece[..k]);
            // ⚠ `k == 0` still advances nothing and counts the record: an
            // all-NUL continuation is padding the writer never emits, and
            // counting it keeps the chain aligned with the header.
            used += k;
            if in_src {
                s_len += k;
                src_left -= 1;
                if src_left == 0 {
                    tgt_left = tgt_total;
                    if tgt_left == 0 {
                        push_rec(recs, &mut taken, s_start, s_len, t_start, t_len, flags);
                    }
                }
            } else {
                t_len += k;
                tgt_left -= 1;
                if tgt_left == 0 {
                    push_rec(recs, &mut taken, s_start, s_len, t_start, t_len, flags);
                }
            }
        }
        if n as usize != buf.len() {
            break;
        }
    }
    taken
}

fn push_rec(
    recs: &mut [RecMount; 64],
    taken: &mut usize,
    s_start: usize,
    s_len: usize,
    t_start: usize,
    t_len: usize,
    flags: u32,
) {
    if *taken >= recs.len() {
        return;
    }
    const MS_RDONLY: u32 = 1;
    recs[*taken] = RecMount {
        src_start: s_start,
        src_len: s_len,
        tgt_start: t_start,
        tgt_len: t_len,
        ro: flags & MS_RDONLY != 0,
    };
    *taken += 1;
}
