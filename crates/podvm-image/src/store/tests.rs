use super::*;

fn scratch(name: &str) -> Store {
    let d = std::env::temp_dir().join(format!("podvm-store-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    Store::open(d).unwrap()
}

// ⭐ **T-0215's diagnostic capture, and it is OFF unless asked for.**
//
// ⛔ **This is the reading that named the mechanism, and it is kept so the
// figures the entry quotes can be re-taken rather than believed.**
// `PODVM_T0215_CAPTURE=1` turns it on;
// `experiments/153-store-lock-race.sh` clause 13 is what sets it, with the
// release deleted, so the capture has something to capture.
//
// ⚠ **It costs a scan of `/proc` and up to 20 ms per refusal**, and a
// legitimate refusal is common: `in_use` answers true whenever a holder
// really is there. That is why it is off by default rather than merely
// `cfg(test)`.
thread_local! {
    static LAST_REFUSAL: std::cell::RefCell<String> =
        const { std::cell::RefCell::new(String::new()) };
}

/// What the last refusal on this thread saw, captured at the `EWOULDBLOCK`
/// itself and printed by [`who_holds`] afterwards.
fn last_refusal() -> String {
    LAST_REFUSAL.with(|c| c.borrow().clone())
}

/// ⭐ **Called from `Lock::try_acquire_noting`'s `EWOULDBLOCK` arm, so it runs
/// before the refusal is even returned to the caller.**
///
/// ⛔ Every instrument before this one ran from the failing assertion and
/// arrived microseconds late: the shortest refusal it chased had already
/// cleared in 11 us, and every capture it took therefore said "nobody".
///
/// ⚠ **The probe's own descriptor is excluded by fd number.** `Lock::open`
/// has just opened one on this very inode, and a scan that counts it finds
/// itself and reports the instrument as the holder. That was the first
/// capture this took.
///
/// ⭐ **The order is cheapest-first and it is deliberate.** `/proc/locks`
/// is one small read and it is the kernel's own answer. Retrying THE SAME
/// DESCRIPTOR is next, because it separates a refusal with a holder from a
/// refusal with none. The scan of every process is last, because it costs
/// milliseconds and the refusals measured here last tens of microseconds.
pub(super) fn note_the_refusal(path: &Path, probe_fd: i64) {
    use std::os::unix::fs::MetadataExt;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("PODVM_T0215_CAPTURE").is_some()) {
        return;
    }
    let Ok(md) = std::fs::metadata(path) else {
        return;
    };
    let (dev, ino) = (md.dev(), md.ino());
    let me = std::process::id();

    // 1. The kernel's own table, read while the refusal is true.
    let mut rows = String::new();
    if let Ok(locks) = std::fs::read_to_string("/proc/locks") {
        let needle = format!(":{ino} ");
        for line in locks.lines() {
            if line.contains("FLOCK") && line.contains(&needle) {
                rows.push_str(&format!("{HUNT_NL}  kernel: {}", line.trim()));
            }
        }
    }
    if rows.is_empty() {
        rows = format!("{HUNT_NL}  kernel: NO FLOCK ROW ON THIS INODE, yet the flock was refused");
    }

    // 2. ⭐ THE SAME DESCRIPTOR, AGAIN, AT ONCE. Nothing else changes: the
    // same fd, the same operation, microseconds later. A refusal that
    // clears here was never a conflict, because no holder can have arrived
    // and left in between.
    let t0 = std::time::Instant::now();
    let mut tries: u64 = 0;
    let verdict;
    loop {
        tries += 1;
        match sys::flock(probe_fd, sys::LOCK_EX | sys::LOCK_NB) {
            Ok(_) => {
                let _ = sys::flock(probe_fd, sys::LOCK_UN);
                verdict = format!(
                    "{HUNT_NL}  THE SAME FD SUCCEEDED on retry {tries} after {} us, \
                         so there was no holder",
                    t0.elapsed().as_micros()
                );
                break;
            }
            Err(e) if e.0 == sys::EWOULDBLOCK.0 => {}
            Err(e) => {
                verdict = format!(
                    "{HUNT_NL}  the same fd answered {} ({}) on retry {tries}",
                    e.name(),
                    e.0
                );
                break;
            }
        }
        if t0.elapsed() > std::time::Duration::from_millis(20) {
            verdict = format!(
                "{HUNT_NL}  the same fd was still refused after {tries} retries and \
                     {} us, so a holder is real",
                t0.elapsed().as_micros()
            );
            break;
        }
    }

    // 3. Every descriptor on the inode anywhere on the host, the probe's
    // own excluded by number.
    let found = scan_every_fd(dev, ino, me, probe_fd);
    let who = if found.is_empty() {
        format!("{HUNT_NL}  no descriptor on this inode in ANY process, the probe's own apart")
    } else {
        found
    };

    LAST_REFUSAL.with(|c| {
        *c.borrow_mut() = format!(
            "{HUNT_NL}AT THE REFUSAL, on inode {ino}, probe fd {probe_fd}:{rows}{verdict}{who}"
        )
    });
}

/// ⭐ [`TODO/image.md`](../../../TODO/image.md) T-0215's instrument, and it
/// is passive: it reads `/proc` and changes nothing it looks at.
///
/// A lock that answers "held" after every holder released it is a second
/// open file description somebody still has, and the question the entry
/// asks is WHO. `/proc/self/fd` names every description THIS process holds
/// and its fd number; `/proc/locks` names the pid holding each `FLOCK` and
/// the inode it is on, so a holder that is a child process rather than this
/// one is visible as a different pid.
///
/// ⛔ Called only from an assertion message, which `assert!` formats on
/// failure alone, so a passing run pays nothing for it. ⚠ It returns a
/// string rather than asserting: an instrument that can fail on its own is
/// a second failure to explain.
///
/// ⚠ One home for the line break the hunt writes, so the two functions
/// below indent their findings the same way this one does.
const HUNT_NL: &str = "\n  ";

fn who_holds(path: &Path) -> String {
    let mut out = format!("\n  T-0215 instrument, path {}", path.display());
    let ino = std::fs::metadata(path)
        .ok()
        .map(|m| std::os::unix::fs::MetadataExt::ino(&m));
    out.push_str(&format!("\n  inode {ino:?}, pid {}", std::process::id()));

    // ⭐ What was seen AT the refusal, where the holder was known to be.
    // Empty unless PODVM_T0215_CAPTURE is set.
    out.push_str(&last_refusal());

    if let Some(ino) = ino {
        out.push_str(&hunt_the_holder(path, ino));
    }

    out.push_str("\n  this process's descriptions on it:");
    let mut mine = 0;
    if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
        for e in entries.flatten() {
            if let Ok(target) = std::fs::read_link(e.path()) {
                // ⚠ `(deleted)` is kept in the comparison rather than
                // stripped: a description on an unlinked inode is exactly
                // the state a swept file leaves behind.
                let t = target.to_string_lossy().to_string();
                if t.starts_with(&path.display().to_string()) {
                    mine += 1;
                    out.push_str(&format!(
                        "\n    fd {} -> {t}",
                        e.file_name().to_string_lossy()
                    ));
                }
            }
        }
    }
    if mine == 0 {
        out.push_str(" none, so the holder is not this process");
    }

    if let (Ok(locks), Some(ino)) = (std::fs::read_to_string("/proc/locks"), ino) {
        out.push_str("\n  /proc/locks rows on this inode:");
        let needle = format!(":{ino} ");
        let mut rows = 0;
        for line in locks.lines() {
            if line.contains("FLOCK") && line.contains(&needle) {
                rows += 1;
                out.push_str(&format!("\n    {}", line.trim()));
            }
        }
        if rows == 0 {
            out.push_str(" none");
        }
    }

    // ⭐ **The children, because a `/proc/locks` row naming THIS pid while
    // this process holds no description on the inode is the signature of a
    // child that inherited one.** There is one lock record per open file
    // description and it keeps the pid that took it, so a child holding the
    // inherited copy still reads as its parent. Naming the children is what
    // tells that apart from a lock this process really holds.
    //
    // ⛔ **THIS HALF HAS NO POSITIVE CONTROL AND CANNOT HAVE ONE HERE.**
    // `children` needs `CONFIG_PROC_CHILDREN`, and where the kernel does
    // not carry it this reads "none" for a process that has children. The
    // control would be a test that forks and asserts the fork is seen,
    // which would add a fork to this binary and so to
    // `experiments/153-store-lock-race.sh`'s fork control, which must have
    // none. ⚠ So "none" here means "none found", and a reader chasing a
    // holder confirms it from outside the process.
    out.push_str("\n  this process's children:");
    let mut kids = 0;
    if let Ok(tasks) = std::fs::read_dir("/proc/self/task") {
        for t in tasks.flatten() {
            if let Ok(list) = std::fs::read_to_string(t.path().join("children")) {
                for pid in list.split_whitespace() {
                    kids += 1;
                    let cmd = std::fs::read_to_string(format!("/proc/{pid}/comm"))
                        .unwrap_or_else(|_| "gone\n".into());
                    out.push_str(&format!("\n    pid {pid} ({})", cmd.trim()));
                }
            }
        }
    }
    if kids == 0 {
        out.push_str(" none");
    }

    // ⚠ **The one active line in this instrument, and the only question the
    // two passive readings cannot answer: is the refusal still true?** It
    // takes the lock and drops it at once. ⛔ It is the same call `in_use`
    // makes, so it observes nothing `in_use` did not already do, and it
    // runs only after an assertion has already failed.
    match try_lock_at(path, sys::LOCK_EX) {
        Ok(Some(_)) => out.push_str(
            "\n  a second attempt SUCCEEDED, so the refusal did not outlive the assertion",
        ),
        Ok(None) => out.push_str("\n  a second attempt was refused as well"),
        Err(e) => out.push_str(&format!("\n  a second attempt errored: {e}")),
    }
    out
}

/// ⭐ **T-0215's decisive instrument: WHO holds the lock, asked of every
/// process rather than only this one.**
///
/// ⛔ **The reading that made this necessary.** [`who_holds`] reads
/// `/proc/self/fd`, so a descriptor held by a CHILD is invisible to it by
/// construction. A `/proc/locks` row naming this pid while this process
/// holds no descriptor on the inode is exactly what a child with an
/// inherited description looks like: there is one lock record per open file
/// description and it keeps the pid that took it. Every capture before this
/// answered "nobody" because the only place it looked could not hold the
/// answer.
///
/// ⚠ **It hunts while the refusal lasts and stops at the first holder.**
/// The refusals measured on 2026-09-12 last between 52 and 2827 us, so
/// there is a window to look in, and the passive readings spend it.
///
/// ⛔ Bounded by time. An unbounded hunt inside a failing test is a hang.
fn hunt_the_holder(path: &Path, ino: u64) -> String {
    use std::os::unix::fs::MetadataExt;
    let dev = std::fs::metadata(path).map(|m| m.dev()).unwrap_or(0);
    let me = std::process::id();
    let start = std::time::Instant::now();
    let mut attempts: u64 = 0;
    loop {
        attempts += 1;
        match try_lock_at(path, sys::LOCK_EX) {
            Ok(Some(_)) => {
                return format!(
                    "{}the hunt: the refusal cleared after {} us and {attempts} \
                         attempt(s), so nothing held it by then",
                    HUNT_NL,
                    start.elapsed().as_micros()
                );
            }
            Ok(None) => {}
            Err(e) => return format!("{HUNT_NL}the hunt could not run: {e}"),
        }
        // Still refused. Ask every process which of them has this inode.
        let found = scan_every_fd(dev, ino, me, -1);
        if !found.is_empty() {
            return format!(
                "{}the hunt: STILL REFUSED after {} us and {attempts} attempt(s), \
                     and the holder is:{found}",
                HUNT_NL,
                start.elapsed().as_micros()
            );
        }
        if start.elapsed() > std::time::Duration::from_millis(200) {
            return format!(
                "{}the hunt: STILL REFUSED after {} us and {attempts} attempt(s), \
                     and NO process on this host has a descriptor on the inode",
                HUNT_NL,
                start.elapsed().as_micros()
            );
        }
    }
}

/// Every `/proc/<pid>/fd` entry whose target is this `(dev, ino)`.
///
/// ⚠ `std::fs::metadata` on `/proc/<pid>/fd/<n>` follows the magic link and
/// stats the OPEN FILE, so it answers for an UNLINKED inode as well, which
/// a swept `*.partial` is. ⛔ Comparing the link TEXT would miss both that
/// and any second path to the same inode.
fn scan_every_fd(dev: u64, ino: u64, me: u32, skip_fd: i64) -> String {
    use std::os::unix::fs::MetadataExt;
    let mut out = String::new();
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return out;
    };
    for p in procs.flatten() {
        let name = p.file_name();
        let Some(pid) = name.to_str().and_then(|t| t.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        for f in fds.flatten() {
            // ⛔ The probe's own descriptor, by number and in this process
            // alone. `Lock::open` has just made one on this inode, and a
            // scan that counts it reports the instrument as the holder.
            let n = f
                .file_name()
                .to_str()
                .and_then(|t| t.parse::<i64>().ok())
                .unwrap_or(-1);
            if pid == me && n == skip_fd {
                continue;
            }
            let Ok(md) = std::fs::metadata(f.path()) else {
                continue;
            };
            if md.ino() != ino || (dev != 0 && md.dev() != dev) {
                continue;
            }
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .unwrap_or_else(|_| "gone".into());
            let target = std::fs::read_link(f.path())
                .map(|t| t.display().to_string())
                .unwrap_or_else(|_| "-".into());
            let whose = if pid == me {
                "THIS PROCESS"
            } else {
                "ANOTHER PROCESS"
            };
            out.push_str(&format!(
                "{}  pid {pid} ({}) fd {} -> {target}  [{whose}]",
                HUNT_NL,
                comm.trim(),
                f.file_name().to_string_lossy()
            ));
        }
    }
    out
}

/// ⭐ **The positive control for [`who_holds`], and without it that
/// instrument's answers mean nothing.**
///
/// ⛔ `docs/methodology/experiments.md`: an absence is not a zero. A probe
/// that reports "nobody holds this" may be looking in the wrong place, and
/// the two readings are told apart only by a case the probe is KNOWN to
/// find. This is that case: the lock is held by this process while the
/// instrument runs, so a report of nothing is the instrument being blind.
///
/// ⚠ **The kernel table is reported and not asserted.** Whether
/// `/proc/locks` lists an `flock` is a property of the host's kernel and
/// its namespaces, not of podvm, so an assertion on it would fail on a
/// machine where podvm is correct. `experiments/153-store-lock-race.sh`
/// runs this test with `--nocapture` and puts the text in the evidence.
#[test]
fn the_t_0215_instrument_sees_a_lock_that_is_held() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("instrument");
    let r = record("docker.io/library/alpine", Some("latest"), 23);
    s.put_record(r.clone()).unwrap();
    let path = s.image_lock_path(&r).unwrap();

    let held = s.hold(&r).unwrap();
    let text = who_holds(&path);
    eprintln!("T-0215 positive control, with the lock HELD:{text}");

    assert!(
        text.contains("\n    fd "),
        "the instrument did not see a description THIS process holds, so \
             its `none` answers say nothing:{text}"
    );
    drop(held);

    // ⚠ And the negative half, which is what the failing assertions read.
    // Nothing holds it now, so the instrument must say so rather than
    // reporting the description that has gone.
    let after = who_holds(&path);
    eprintln!("T-0215 positive control, after the release:{after}");
    assert!(
        !after.contains("\n    fd "),
        "the instrument still names a description after the holder went:{after}"
    );
    let _ = std::fs::remove_dir_all(s.root());
}

mod staging;
fn record(repo: &str, tag: Option<&str>, seed: u8) -> Record {
    let d = |b: u8| Digest::of(&[b, seed]).to_string();
    Record {
        repository: repo.into(),
        tag: tag.map(str::to_string),
        digest: d(1),
        digest_media_type: oci::MEDIA_OCI_INDEX.into(),
        manifest_digest: d(2),
        config_digest: d(3),
        platform: "linux/amd64".into(),
        layers: vec![d(4), d(5)],
        stored_bytes: 100,
        architecture: "amd64".into(),
        os: "linux".into(),
        created: Some("2024-01-01T00:00:00Z".into()),
        pulled_at: clock::now(),
    }
}

mod locks;
use locks::*;
mod refs;

// TODO/supervise.md T-1715: the store root keeps the container table.
mod modes;
