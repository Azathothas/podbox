/// Create the host memo file where absent, leaving what is there alone.
///
/// An existing memo is a container's ownership memory and must survive a
/// second entry, so this never truncates. A fresh file is empty, which reads
/// as "no record yet" rather than as degraded.
pub fn ensure_memo_file(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Open the host memo for handing to the payload.
///
/// `O_APPEND` so the kernel serialises one-record writes from concurrent
/// payload processes with no lock in the interposer. The returned handle must
/// outlive the spawn: the child dups it to `MEMO_CHILD_FD` before the
/// `chroot`.
pub fn open_memo(path: &std::path::Path) -> Result<std::fs::File, String> {
    ensure_memo_file(path)?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Emulation tally counts, T-0708: how many times the preloaded tier told
/// the payload a compatibility story (a regular file for a node, a mount
/// success, an unshare success, a stripped clone) rather than the kernel's
/// answer.
///
/// T-0413 adds the fifth story: a `/proc/self` answer where no procfs is
/// mounted (a descriptor duplicate, a `readlink` synthesis, an exe
/// passthrough, a mount-table serve).
///
/// Read from the ownership memo file beside the container record: the
/// interposer appends 32-byte tally records (device `u64::MAX`, the
/// operation in the inode word, mount path chains behind their header)
/// through the same descriptor it writes ownership intent through. Counts
/// count complete records only: a mount header whose path chain breaks is
/// dropped, never half-counted.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EmulatedCounts {
    pub mknod: u64,
    pub mount: u64,
    pub unshare: u64,
    pub clone: u64,
    pub procfs: u64,
}

/// Tally record kinds, in the inode word beside `u64::MAX`. One fact in two
/// homes with `podvm-interpose`'s `emulate` module; the values agree by
/// driving the behaviour end to end.
pub(super) const TALLY_MKNOD: u64 = 1;
pub(super) const TALLY_MOUNT: u64 = 2;
pub(super) const TALLY_UNSHARE: u64 = 3;
pub(super) const TALLY_CLONE: u64 = 4;
pub(super) const TALLY_PROC: u64 = 5;

fn tally_u64(b: &[u8]) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[..8]);
    u64::from_le_bytes(a)
}

fn tally_u32(b: &[u8]) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&b[..4]);
    u32::from_le_bytes(a)
}

/// Scan the memo file for complete tally records, T-0708.
///
/// Ownership records pass through untouched (their device word is a real
/// one). A mount header opens a chain of exactly the continuations it
/// names; anything else, including another header, drops the open chain.
/// The scan streams in fixed chunks, so a memo grown large by a busy
/// payload cannot grow this reader's memory with it.
pub fn emulated_counts(memo: &std::path::Path) -> EmulatedCounts {
    const REC: usize = 32;
    const CHUNK_RECS: usize = 4096;
    let mut out = EmulatedCounts::default();
    let Ok(mut f) = std::fs::File::open(memo) else {
        return out;
    };
    use std::io::Read;
    let mut buf = vec![0u8; REC * CHUNK_RECS];
    // Continuations still owed to an open mount header. Zero means none.
    let mut pending: u64 = 0;
    loop {
        // Fill to a record boundary: a record never straddles two reads,
        // so the scan below needs no carry-over.
        let mut have = 0usize;
        while have < buf.len() {
            match f.read(&mut buf[have..]) {
                Ok(0) => break,
                Ok(n) => have += n,
                Err(_) => break,
            }
        }
        if have == 0 {
            break;
        }
        let mut at = 0usize;
        while at + REC <= have {
            let r = &buf[at..at + REC];
            if tally_u64(&r[0..8]) != u64::MAX {
                // An ownership record. It also breaks any open chain: the
                // writer places each chain contiguously, so anything else
                // between a header and its continuations is a torn write.
                pending = 0;
                at += REC;
                continue;
            }
            match tally_u64(&r[8..16]) {
                TALLY_MKNOD => {
                    pending = 0;
                    out.mknod += 1;
                }
                TALLY_MOUNT => {
                    let want = tally_u32(&r[20..24]) as u64 + tally_u32(&r[24..28]) as u64;
                    if want == 0 {
                        out.mount += 1;
                    } else {
                        pending = want;
                    }
                }
                TALLY_UNSHARE => {
                    pending = 0;
                    out.unshare += 1;
                }
                TALLY_CLONE => {
                    pending = 0;
                    out.clone += 1;
                }
                TALLY_PROC => {
                    pending = 0;
                    out.procfs += 1;
                }
                // A continuation where one is owed feeds the open chain;
                // anywhere else (stray, or an unknown kind) it is skipped,
                // and an unknown header drops the open chain with it.
                0 => {
                    if pending > 0 {
                        pending -= 1;
                        if pending == 0 {
                            out.mount += 1;
                        }
                    }
                }
                _ => {
                    pending = 0;
                }
            }
            at += REC;
        }
        if have < buf.len() {
            break;
        }
    }
    out
}
