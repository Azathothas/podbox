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

/// ⭐ **Called from `Lock::try_acquire`'s `EWOULDBLOCK` arm, so it runs
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
    match Lock::try_acquire(path, sys::LOCK_EX) {
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
        match Lock::try_acquire(path, sys::LOCK_EX) {
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

/// ⛔ INVARIANT I6. Two `stage` calls in ONE process must not take one
/// name. `TODO/probe.md` T-0113 is this defect one crate over, and it was
/// found by luck there; this is the assertion that would have found it.
#[test]
fn two_staging_calls_in_one_process_take_two_names() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("stage-unique");
    let (a, _fa) = s.stage("layer").unwrap();
    let (b, _fb) = s.stage("layer").unwrap();
    assert_ne!(a, b, "one process staged two blobs over one name");
    assert!(a.is_file() && b.is_file());
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ T-0214. A restarted transfer writes byte zero at offset zero, and the
/// file holds ONLY the second attempt.
///
/// ⛔ Truncate AND rewind, which is what this asserts by writing something
/// SHORTER the second time: `set_len(0)` alone leaves the offset where the
/// cut-short attempt left it, so the retry lands past a hole and the file is
/// longer than what arrived. ⚠ On a sparse filesystem that hole reads back
/// as NUL bytes and the digest fails a long way from the cause.
#[test]
fn a_restarted_staging_file_holds_only_the_second_attempt() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    use crate::registry::Restart;
    let s = scratch("restart");
    let (path, mut f) = s.stage("layer").unwrap();
    f.write_all(b"the first attempt, cut short").unwrap();
    f.restart().unwrap();
    f.write_all(b"second").unwrap();
    f.flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⛔ INVARIANT I5. An abandoned staging file is swept and one being
/// written is not, and the difference is a lock rather than a pid.
#[test]
fn the_sweep_takes_an_abandoned_partial_and_leaves_a_held_one() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("sweep");
    // Abandoned: staged, then the handle dropped without a commit, which is
    // what a SIGKILL leaves behind.
    let (dead, handle) = s.stage("abandoned").unwrap();
    drop(handle);
    // Live: still held, exactly as a writer mid-blob holds it.
    let (live, _held) = s.stage("live").unwrap();

    let swept = s.sweep_staging();
    assert!(
        swept.contains(&dead),
        "the abandoned file was not swept{}",
        who_holds(&dead)
    );
    assert!(!dead.exists(), "the abandoned file is still there");
    assert!(live.exists(), "the sweep took a file being written");
    assert!(!swept.contains(&live));
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⚠ And opening a store is what runs it, because every command does that
/// and nothing else would.
#[test]
fn opening_a_store_sweeps_what_a_killed_process_left() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("podvm-store-{}-openswp", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let s = Store::open(&d).unwrap();
    let (dead, handle) = s.stage("abandoned").unwrap();
    drop(handle);
    assert!(dead.is_file());
    let _again = Store::open(&d).unwrap();
    assert!(
        !dead.exists(),
        "a second open did not sweep{}",
        who_holds(&dead)
    );
    let _ = std::fs::remove_dir_all(&d);
}

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

#[test]
fn a_blob_whose_bytes_do_not_match_never_reaches_the_blob_directory() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⛔ The central rule of T-0202, driven rather than asserted about.
    let s = scratch("verify");
    let claimed = Digest::of(b"alpine");
    let e = s.put_bytes(b"not alpine", &claimed, "layer").unwrap_err();
    assert!(format!("{e}").contains("digest mismatch"), "{e}");
    assert!(!s.has_blob(&claimed));
    let _ = std::fs::remove_dir_all(s.root());
}

/// T-1321: the sweep reports exactly the blob whose bytes changed, and
/// a clean set verifies silent.
#[test]
fn verify_reports_the_flipped_blob_and_nothing_else() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("health");
    let dg = Digest::of(b"good bytes");
    let db = Digest::of(b"bad bytes");
    s.put_bytes(b"good bytes", &dg, "blob").unwrap();
    s.put_bytes(b"bad bytes", &db, "blob").unwrap();
    let path = s.blob_path(&db);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&path, &bytes).unwrap();
    let hits = s.verify_blobs(&[dg.to_string(), db.to_string()]).unwrap();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].want, db.to_string());
    let clean = s.verify_blobs(&[dg.to_string()]).unwrap();
    assert!(clean.is_empty(), "{clean:?}");
    let _ = std::fs::remove_dir_all(s.root());
}

/// T-1321: a noted pull reads back with all six fields.
#[test]
fn provenance_round_trips_all_six_fields() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("provenance");
    let p = crate::health::Provenance {
        registry: "registry-1.docker.io".into(),
        repository: "docker.io/library/alpine".into(),
        tag: Some("3.20".into()),
        manifest_digest: "sha256:c64c687cbea9300178b30c95835354e34c4e4febc4badfe27102879de0483b5e"
            .into(),
        pulled_at: clock::now(),
        podvm_version: "0.1.0".into(),
    };
    s.note_pull(&p).unwrap();
    let all = s.read_provenance().unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    let back = &all[0];
    assert_eq!(back.registry, p.registry);
    assert_eq!(back.repository, p.repository);
    assert_eq!(back.tag, p.tag);
    assert_eq!(back.manifest_digest, p.manifest_digest);
    assert_eq!(back.pulled_at, p.pulled_at);
    assert_eq!(back.podvm_version, p.podvm_version);
    let _ = std::fs::remove_dir_all(s.root());
}

/// T-1320: a saved image loads into a fresh store with the same record
/// and readable blobs, under its own name.
#[test]
fn a_saved_image_loads_into_a_fresh_store() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let a = scratch("save-a");
    let layer = b"layer-bytes";
    let config = br#"{"architecture":"amd64","os":"linux"}"#;
    let dl = Digest::of(layer);
    let dc = Digest::of(config);
    let manifest = format!(
        r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"{dc}","size":{}}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar","digest":"{dl}","size":{}}}]}}"#,
        config.len(),
        layer.len(),
    );
    let dm = Digest::of(manifest.as_bytes());
    a.put_bytes(layer, &dl, "layer").unwrap();
    a.put_bytes(config, &dc, "config").unwrap();
    a.put_bytes(manifest.as_bytes(), &dm, "manifest").unwrap();
    a.put_record(Record {
        repository: "docker.io/library/saved-test".into(),
        tag: Some("v1".into()),
        digest: dm.to_string(),
        digest_media_type: crate::oci::MEDIA_OCI_MANIFEST.into(),
        manifest_digest: dm.to_string(),
        config_digest: dc.to_string(),
        platform: "linux/amd64".into(),
        layers: vec![dl.to_string()],
        stored_bytes: (layer.len() + config.len()) as u64,
        architecture: "amd64".into(),
        os: "linux".into(),
        created: None,
        pulled_at: clock::now(),
    })
    .unwrap();
    let tarball = std::env::temp_dir().join(format!("podvm-save-{}.tar", std::process::id()));
    let mut f = std::fs::File::create(&tarball).unwrap();
    crate::layout::save(&a, "saved-test:v1", &mut f).unwrap();
    drop(f);
    let b = scratch("save-b");
    let loaded = crate::layout::load(&b, &tarball).unwrap();
    assert_eq!(loaded.manifest_digest, dm.to_string());
    assert_eq!(loaded.layers, vec![dl.to_string()]);
    let found = b.find_one("saved-test:v1").unwrap();
    assert_eq!(found.manifest_digest, dm.to_string());
    let back: Vec<u8> = b.read_blob(&dm).unwrap();
    assert_eq!(back, manifest.as_bytes());
    let _ = std::fs::remove_dir_all(a.root());
    let _ = std::fs::remove_dir_all(b.root());
    let _ = std::fs::remove_file(&tarball);
}

/// T-1320: a tarball whose bytes do not match their names is refused on
/// the way in, the same verification pull performs.
#[test]
fn a_tampered_tarball_is_refused_on_load() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let a = scratch("save-tamper-a");
    let layer = b"layer-bytes";
    let dc = Digest::of(b"{}");
    let dl = Digest::of(layer);
    a.put_bytes(layer, &dl, "layer").unwrap();
    a.put_bytes(b"{}", &dc, "config").unwrap();
    let manifest = format!(
        r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"{dc}","size":2}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar","digest":"{dl}","size":{}}}]}}"#,
        layer.len(),
    );
    let dm = Digest::of(manifest.as_bytes());
    a.put_bytes(manifest.as_bytes(), &dm, "manifest").unwrap();
    a.put_record(Record {
        repository: "docker.io/library/saved-test".into(),
        tag: Some("v1".into()),
        digest: dm.to_string(),
        digest_media_type: crate::oci::MEDIA_OCI_MANIFEST.into(),
        manifest_digest: dm.to_string(),
        config_digest: dc.to_string(),
        platform: "linux/amd64".into(),
        layers: vec![dl.to_string()],
        stored_bytes: (layer.len() + 2) as u64,
        architecture: "amd64".into(),
        os: "linux".into(),
        created: None,
        pulled_at: clock::now(),
    })
    .unwrap();
    let tarball = std::env::temp_dir().join(format!("podvm-tamper-{}.tar", std::process::id()));
    let mut f = std::fs::File::create(&tarball).unwrap();
    crate::layout::save(&a, "saved-test:v1", &mut f).unwrap();
    drop(f);
    let mut bytes = std::fs::read(&tarball).unwrap();
    let at = bytes
        .windows(layer.len())
        .position(|w| w == layer)
        .expect("the tarball carries the layer bytes");
    bytes[at] ^= 1;
    std::fs::write(&tarball, &bytes).unwrap();
    let b = scratch("save-tamper-b");
    let e = crate::layout::load(&b, &tarball).unwrap_err();
    assert!(format!("{e}").contains("mismatch"), "{e}");
    let _ = std::fs::remove_dir_all(a.root());
    let _ = std::fs::remove_dir_all(b.root());
    let _ = std::fs::remove_file(&tarball);
}

#[test]
fn a_blob_that_matches_is_stored_under_its_digest_and_reads_back() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("roundtrip");
    let d = Digest::of(b"alpine");
    s.put_bytes(b"alpine", &d, "layer").unwrap();
    assert!(s.has_blob(&d));
    assert!(s.blob_path(&d).ends_with(d.hex()));
    assert_eq!(s.read_blob(&d).unwrap(), b"alpine");
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn a_stored_blob_edited_behind_podvms_back_is_caught_on_the_way_out() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("tamper");
    let d = Digest::of(b"alpine");
    s.put_bytes(b"alpine", &d, "layer").unwrap();
    std::fs::write(s.blob_path(&d), b"tampered").unwrap();
    let e = s.read_blob(&d).unwrap_err();
    assert!(format!("{e}").contains("digest mismatch"), "{e}");
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn a_shared_blob_survives_removing_one_of_the_two_images_that_reach_it() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⭐ Reachability is computed over what SURVIVES. Computing it over the
    // doomed set deletes a shared layer with the first image that goes.
    let s = scratch("shared");
    let a = record("docker.io/library/alpine", Some("3.20"), 7);
    let mut b = record("docker.io/library/alpine", Some("3.21"), 7);
    b.digest = Digest::of(b"another index").to_string();
    for r in [&a, &b] {
        for blob in r.blobs() {
            let d = Digest::parse(blob).unwrap();
            s.put_bytes(blob.as_bytes(), &Digest::of(blob.as_bytes()), "x")
                .ok();
            std::fs::write(s.blob_path(&d), b"payload").unwrap();
        }
        s.put_record(r.clone()).unwrap();
    }
    let shared = Digest::parse(&a.layers[0]).unwrap();
    s.remove("alpine:3.20").unwrap();
    assert!(s.has_blob(&shared), "a layer 3.21 still needs was deleted");
    s.remove("alpine:3.21").unwrap();
    assert!(!s.has_blob(&shared), "the last reference did not free it");
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⭐ T-0204's acceptance, minus the container: the lock is the whole
    // mechanism and it is driven here through the shipping functions.
    let s = scratch("inuse");
    let r = record("docker.io/library/alpine", Some("latest"), 9);
    s.put_record(r.clone()).unwrap();
    assert!(!s.in_use(&r).unwrap());

    let held = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap());
    let e = s.remove("alpine:latest").unwrap_err();
    assert!(format!("{e}").contains("in use"), "{e}");
    let pruned = s.prune(true).unwrap();
    assert_eq!(pruned.skipped, vec!["alpine:latest".to_string()]);
    assert!(pruned.untagged.is_empty());

    drop(held);
    let lock_path = s.image_lock_path(&r).unwrap();
    assert!(
        !s.in_use(&r).unwrap(),
        "the holder released it and it still reads as in use{}",
        who_holds(&lock_path)
    );
    assert!(
        s.remove("alpine:latest").is_ok(),
        "rmi refused an image nothing is using{}",
        who_holds(&lock_path)
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ T-0211's plant. This is the defect that surfaced as an intermittent
/// failure of the test above at 2 runs of 6 of the full workspace, and it is
/// made to fail on demand here rather than once a week.
///
/// ⛔ Red before the fix: with the image lock opened without `O_CLOEXEC`,
/// the forked child below inherits the fd, the `flock` outlives
/// `drop(held)`, and `in_use` answers true for an image nothing is using.
/// The forking thread in the real suite is `probe_cache`'s, whose `measure`
/// makes one fresh child per probe; the `clone_fork` here is that, reduced
/// to the one call that matters.
#[test]
fn a_fork_while_the_lock_is_held_does_not_extend_it() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("forkhold");
    let r = record("docker.io/library/alpine", Some("latest"), 17);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap(), "the holder itself did not register");

    // ⛔ A pipe, not a sleep. `clone_fork` sheds the registered fds in the
    // child before it returns there, so the child is only known to have
    // shed once it has run at all, and a parent that asserts before the
    // child is scheduled reads the fd as still open and fails for a reason
    // that has nothing to do with the defect. That is the same shape of
    // intermittent failure T-0211 itself arrived as, so this test is made
    // to wait for the fact rather than for a duration.
    let mut fds = [0i32; 2];
    sys::pipe2(&mut fds, 0).unwrap();
    let (r_fd, w_fd) = (fds[0] as i64, fds[1] as i64);

    // ⚠ `clone_fork` and not `std::process::Command`: the defect is about
    // what a bare `fork` inherits, and spawning a process would exec and so
    // hide it behind the `FD_CLOEXEC` the other test covers.
    let pid = unsafe { sys::clone_fork(sys::SIGCHLD) }.unwrap();
    if pid == 0 {
        // ---- child. It has already shed; say so, then outlive the drop.
        let _ = sys::write(w_fd, b"x");
        std::thread::sleep(std::time::Duration::from_millis(400));
        // ⛔ Never returns into the test harness: exits without unwinding.
        sys::exit_group(0);
    }
    let _ = sys::close(w_fd);
    let mut ack = [0u8; 1];
    assert_eq!(sys::read(r_fd, &mut ack).unwrap(), 1, "the child never ran");

    drop(held);
    let free = s.in_use(&r).map(|u| !u);

    let mut status = 0;
    let _ = sys::close(r_fd);
    let _ = sys::wait4(pid, &mut status);
    assert!(
        free.unwrap(),
        "the holder released the lock and it is still held: a forked child \
             inherited the fd, which is TODO/image.md T-0211{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⛔ **NO TWO TESTS IN THIS MODULE RUN AT THE SAME TIME**, and this is what
/// stops them. Two reasons, and either alone would need it.
///
/// 1. **The fork.** `cargo test` runs tests in THREADS of one process. A
///    bare `fork` copies every open descriptor of that process, including a
///    lock another test is holding in another thread; that child then holds
///    the lock for as long as it lives, and the other test's assertion --
///    "the holder released it and nobody else has it" -- fails for a reason
///    that has nothing to do with its subject. Measured on 2026-09-09:
///    `a_spawned_process_does_not_inherit_the_lock` failed once in a
///    full-workspace run and passed alone and on the retry, which is the
///    shape a flake takes and is not one.
///    [`TODO/image.md`](../../../TODO/image.md) T-0211.
/// 2. **The slots.** Every `Lock` this crate takes registers one of sixteen
///    process-wide fork-shed slots (`sys::FORK_CLOSE_SLOTS`), and a
///    seventeenth is refused by name. Twenty libtest threads holding one to
///    three locks each exhaust them, and the test that asks last is refused
///    for a reason that has nothing to do with its subject. Measured on
///    2026-09-21: 8 refusals in 10 parallel runs of this suite, a different
///    victim each time, and 97 of 97 serial.
///    [`TODO/image.md`](../../../TODO/image.md) T-1310.
///
/// ⚠ It is the same trap as T-0603's, which counted `/proc/self/task`
/// before and after a spawn and failed about one run in five.
///
/// ⛔ Helpers in this module (`scratch`, `record`, `who_holds`) must never
/// take it: a test calls them with it already held, and a second
/// acquisition on this thread deadlocks. Only `#[test]` functions take it,
/// exactly once, as their first line. That is enough by audit: no test
/// outside this module holds a `Lock` (`probe_cache` and `pull` tests only
/// `Store::open` fresh directories, which sweeps nothing and takes no
/// slot), and the implementation review greps every test body below for the
/// acquisition line.
static STORE_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// ⭐ T-0211's second half, and it is a **different** failure from the one
/// above rather than the same one written twice.
///
/// `clone_fork` sheds registered fds, so nothing podvm forks itself can
/// carry a lock away. ⛔ `std::process::Command` forks inside libstd and
/// never passes through `clone_fork`, so the shed list cannot reach it and
/// `O_CLOEXEC` on the descriptor is the only thing that does. Reverting
/// either defence turns exactly one of these two tests red, which is how it
/// was confirmed they are independent.
#[test]
fn a_spawned_process_does_not_inherit_the_lock() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("spawnhold");
    let r = record("docker.io/library/alpine", Some("latest"), 19);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    // ⛔ The child announces itself on stdout and the parent reads that
    // before asserting. `O_CLOEXEC` takes the fd away at the **exec**, so a
    // parent that asserts while the child is still between `fork` and
    // `execve` measures the fork window instead, which is the other test's
    // subject, and would make this one fail for the wrong reason.
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("echo ready; sleep 2")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("/bin/sh");
    let mut line = [0u8; 6];
    {
        use std::io::Read;
        child
            .stdout
            .as_mut()
            .unwrap()
            .read_exact(&mut line)
            .expect("the child never reached its exec");
    }
    assert_eq!(&line, b"ready\n");

    drop(held);
    let free = s.in_use(&r).map(|u| !u);
    // ⚠ Reaped before the assertion, so a failure does not also leave a
    // process behind for whatever runs next.
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        free.unwrap(),
        "the holder released the lock and a spawned process is still \
             holding it: the lock fd was not O_CLOEXEC, which is the exec half \
             of TODO/image.md T-0211{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ The multi-architecture case, and the one a platform-blind key gets
/// wrong silently: the second pull deletes the first, its blobs are
/// collected, and a caller who pulled both is left with one.
/// ⭐ The door sweep's finding, and it is the same defect as `find_for`'s
/// through a **different** door: `extract` and `inspect` reach the store by
/// `find_one`, which took `.next()`.
#[test]
fn an_ambiguous_reference_is_refused_by_name_and_never_by_position() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("ambig");
    for (plat, arch, n) in [("linux/amd64", "amd64", 31), ("linux/arm64", "arm64", 33)] {
        let mut r = record("docker.io/library/alpine", Some("latest"), n);
        r.platform = plat.into();
        r.architecture = arch.into();
        s.put_record(r).unwrap();
    }

    // ⚠ No platform asked for: the HOST's is preferred, and there is
    // exactly one of those, so this is not ambiguous.
    let got = s.find_one("alpine:latest").unwrap();
    assert_eq!(got.platform, crate::platform::Platform::host().to_string());

    // ⛔ A platform the store does not hold is refused NAMING what it does.
    let e = s
        .find_one_for(
            "alpine:latest",
            Some(&crate::platform::Platform::parse("linux/riscv64").unwrap()),
        )
        .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("linux/amd64"), "{text}");
    assert!(text.contains("linux/arm64"), "{text}");

    // ⛔ And where the host's platform is not among them either, it refuses
    // rather than taking the first.
    let solo = scratch("ambig2");
    for (plat, arch, n) in [
        ("linux/riscv64", "riscv64", 35),
        ("linux/s390x", "s390x", 37),
    ] {
        let mut r = record("docker.io/library/alpine", Some("latest"), n);
        r.platform = plat.into();
        r.architecture = arch.into();
        solo.put_record(r).unwrap();
    }
    let e = solo.find_one("alpine:latest").unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("will not pick one by position"), "{text}");
    assert!(text.contains("--platform"), "{text}");

    let _ = std::fs::remove_dir_all(s.root());
    let _ = std::fs::remove_dir_all(solo.root());
}

#[test]
fn two_platforms_of_one_tag_are_two_images_and_not_one() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("twoplat");
    let mut amd = record("docker.io/library/alpine", Some("latest"), 21);
    amd.platform = "linux/amd64".into();
    amd.architecture = "amd64".into();
    let mut arm = record("docker.io/library/alpine", Some("latest"), 23);
    arm.platform = "linux/arm64".into();
    arm.architecture = "arm64".into();

    s.put_record(amd.clone()).unwrap();
    s.put_record(arm.clone()).unwrap();
    assert_eq!(
        s.find("alpine:latest").unwrap().len(),
        2,
        "the second pull replaced the first: the platform is not in the key"
    );

    // ⛔ Re-pulling ONE platform replaces only that one. A moving tag is
    // still the normal case and this must not accumulate.
    s.put_record(amd.clone()).unwrap();
    assert_eq!(s.find("alpine:latest").unwrap().len(), 2);

    // Asked for a platform: exactly that one comes back, by name.
    let p = crate::platform::Platform::parse("linux/arm64").unwrap();
    let got = s.find_for("alpine:latest", Some(&p)).unwrap();
    assert_eq!(got.matched.len(), 1);
    assert_eq!(got.matched[0].platform, "linux/arm64");

    // ⚠ Asked for a platform the store does not hold: NOTHING matches, and
    // what it does hold is reported separately so the caller can say "held,
    // for another platform" rather than "no such image".
    let none = crate::platform::Platform::parse("linux/riscv64").unwrap();
    let miss = s.find_for("alpine:latest", Some(&none)).unwrap();
    assert!(
        miss.matched.is_empty(),
        "a wrong-platform record was returned"
    );
    assert_eq!(miss.other_platforms.len(), 2);

    // ⛔ The regression that produced `Found`: with ONE record in the store
    // and a different platform asked for, a count-based short circuit
    // returned it and podvm ran the wrong architecture in silence.
    let solo = scratch("solo");
    let mut only = record("docker.io/library/alpine", Some("latest"), 29);
    only.platform = "linux/amd64".into();
    solo.put_record(only).unwrap();
    let arm = crate::platform::Platform::parse("linux/arm64").unwrap();
    let got = solo.find_for("alpine:latest", Some(&arm)).unwrap();
    assert!(
        got.matched.is_empty(),
        "one record in the store was returned for a platform it is not"
    );
    assert_eq!(got.other_platforms, vec!["linux/amd64".to_string()]);
    let _ = std::fs::remove_dir_all(solo.root());

    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ **The other half of T-0215's fix, and the half that would break
/// [T-0204](#t-0204-images-rmi-tag-and-a-store-gc-that-cannot-delete-a-running-containers-rootfs)
/// in silence.**
///
/// ⛔ `Lock::drop` releases explicitly now, and one lock must NOT be
/// released: the one [`Lock::hand_to_payload`] gave to a running container.
/// The payload holds a duplicate of THIS open file description, and
/// `LOCK_UN` belongs to the description rather than to a descriptor, so
/// releasing here would release the payload's lock as well. A concurrent
/// `rmi` or `prune` could then delete the rootfs the container is executing
/// out of, which is the defect T-0204 and invariant I4 were paid for.
///
/// ⚠ **Red if the exemption goes, and not intermittently.** Make the
/// release unconditional and the first assertion below fails every time.
///
/// ⭐ `dup` stands in for the payload, for the same reason it stands in for
/// a fork in the test above: what the payload gets from `fork` and `execve`
/// is a second descriptor on one description.
#[test]
fn a_lock_handed_to_the_payload_outlives_this_process_dropping_it() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("handed");
    let r = record("docker.io/library/alpine", Some("latest"), 33);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    let fd = held
        .hand_to_payload()
        .expect("hand the lock to the payload");
    // The payload's copy, which a `fork` would have made for it.
    let payload = sys::dup_cloexec(fd).expect("dup the handed descriptor");

    drop(held);
    let still_held = s.in_use(&r).unwrap();
    // ⚠ Closed before the assertion, so a failure leaves nothing behind.
    let _ = sys::close(payload);
    assert!(
        still_held,
        "the lock was handed to the payload and this process released it \
             anyway, so a running container's image reads as free and a prune \
             may delete the rootfs it is executing out of (TODO/image.md T-0204)"
    );

    // ⛔ **THE RELEASE ARRIVES, WHICH IS NOT THE SAME AS ARRIVING AT ONCE,
    // AND MUST NOT BE.** The exemption exists so that THIS process does not
    // release a lock the payload is keeping, so only the LAST reference can
    // release it and that is by design not this thread. The handed lock
    // therefore keeps the asynchronous release this entry is about, and it
    // is the one place that is correct: while the payload lives the lock IS
    // held, and the only window left reads an image as in use for a few
    // hundred microseconds after its container really ended, which is the
    // safe direction and self-corrects on the next attempt.
    // ⚠ Asserting immediacy here is what a first draft of this test did,
    // and it failed 9 and 13 of 30 for exactly the reason T-0215 names.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut freed = false;
    while std::time::Instant::now() < deadline {
        if !s.in_use(&r).unwrap() {
            freed = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        freed,
        "the payload's descriptor is closed and five seconds later the image \
             still reads as in use, so the exemption is a permanent hold rather \
             than a release this thread does not make{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ **T-0215's regression test, and it is DETERMINISTIC where the defect
/// it guards was one run in two.**
///
/// ⛔ **The defect.** `Lock::drop` released the lock by closing its
/// descriptor, and a `close` releases an `flock` only when it drops the
/// LAST reference to the open file description. A `fork` makes a second
/// reference, so after one the holder's own `close` no longer completes the
/// release: the record goes away later, when the other reference does, and
/// that is asynchronous. `in_use` read those windows as an image still in
/// use after its holder had released it, and a sweep left an abandoned file
/// behind.
///
/// ⭐ **`dup` is the fork, without the fork.** A duplicate descriptor
/// shares one open file description with the original, which is exactly
/// what a child gets and exactly what makes the `close` insufficient. So
/// this test needs no second process, no thread and no timing: it holds a
/// second reference across the drop and asks whether the lock is gone.
///
/// ⚠ Red before the fix, and not intermittently: with `Drop` closing alone,
/// the duplicate keeps the description alive and `in_use` answers true
/// every time.
#[test]
fn releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("dupfree");
    let r = record("docker.io/library/alpine", Some("latest"), 31);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap(), "the holder itself did not register");

    // ⭐ A second reference to the SAME open file description, which is
    // what a `fork` hands a child. `F_DUPFD_CLOEXEC` and not `dup`, because
    // every descriptor in this tree is close-on-exec.
    let twin = sys::dup_cloexec(held.fd).expect("dup the lock descriptor");

    drop(held);

    let free = !s.in_use(&r).unwrap();
    // ⚠ Closed before the assertion, so a failure does not leave a
    // descriptor behind for whatever runs next.
    let _ = sys::close(twin);
    assert!(
        free,
        "the holder released the lock and it is still held, because a \
             duplicate of its open file description outlived it: closing a \
             descriptor releases an flock only when it drops the LAST reference \
             (TODO/image.md T-0215){}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );

    // ⛔ And the lock file is usable again afterwards, so the explicit
    // release did not leave the descriptor in a state nothing can take.
    let again = s.hold(&r).unwrap();
    drop(again);
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn two_holders_of_one_image_both_have_to_go_before_it_is_free() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("twohold");
    let r = record("docker.io/library/alpine", Some("latest"), 11);
    s.put_record(r.clone()).unwrap();
    let a = s.hold(&r).unwrap();
    let b = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap());
    drop(a);
    assert!(s.in_use(&r).unwrap(), "one holder left and it read as free");
    drop(b);
    assert!(
        !s.in_use(&r).unwrap(),
        "both holders went and it still reads as in use{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn a_tag_points_at_the_same_manifest_without_copying_a_blob() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("tag");
    let r = record("docker.io/library/alpine", Some("latest"), 13);
    s.put_record(r.clone()).unwrap();
    let tagged = s.tag("alpine:latest", "myalpine:v1").unwrap();
    assert_eq!(tagged.digest, r.digest);
    // ⚠ docker's normalisation, not a shortcut: a single-component name
    // is a hub `library/` repository whichever verb produced it, and
    // `display_repository` is what turns it back into `myalpine`.
    assert_eq!(tagged.repository, "docker.io/library/myalpine");
    assert_eq!(tagged.display_repository(), "myalpine");
    assert_eq!(s.list().unwrap().len(), 2);
    // Removing one name leaves the other and its blobs.
    s.remove("alpine:latest").unwrap();
    assert_eq!(s.find("myalpine:v1").unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn tagging_something_a_digest_names_is_refused() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("tagdigest");
    s.put_record(record("docker.io/library/alpine", Some("latest"), 17))
        .unwrap();
    let e = s
        .tag("alpine:latest", &format!("x@sha256:{}", "0".repeat(64)))
        .unwrap_err();
    // ⚠ 1 and not 125: an invalid reference is refused AFTER the flags
    // parsed, and docker's own code for that is 1 (T-0802).
    assert_eq!(e.exit_code(), crate::error::EXIT_CLI_ERROR);
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn re_pulling_one_tag_replaces_its_record_rather_than_adding_a_second() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("retag");
    s.put_record(record("docker.io/library/alpine", Some("latest"), 19))
        .unwrap();
    let mut moved = record("docker.io/library/alpine", Some("latest"), 19);
    moved.digest = Digest::of(b"the tag moved").to_string();
    s.put_record(moved.clone()).unwrap();
    let got = s.find("alpine:latest").unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].digest, moved.digest);
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn an_image_is_found_by_tag_by_digest_and_by_docker_short_id() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("find");
    let r = record("docker.io/library/alpine", Some("latest"), 23);
    s.put_record(r.clone()).unwrap();
    assert_eq!(s.find("alpine").unwrap().len(), 1);
    assert_eq!(s.find("alpine:latest").unwrap().len(), 1);
    assert_eq!(s.find(&format!("alpine@{}", r.digest)).unwrap().len(), 1);
    let short = &r.config_digest.trim_start_matches("sha256:")[..12];
    assert_eq!(s.find(short).unwrap().len(), 1);
    assert!(s.find("alpine:3.20").unwrap().is_empty());
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn an_index_written_by_a_later_podvm_is_refused_rather_than_reinterpreted() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("podvm-store-{}-future", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join(INDEX_FILE),
        format!("{{\"podvm_store\":{},\"images\":[]}}", INDEX_VERSION + 1),
    )
    .unwrap();
    let e = Store::open(&d).unwrap_err();
    assert!(format!("{e}").contains("newer podvm"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_store_root_prefers_podvm_store_and_never_falls_to_tmpdir_silently() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⚠ The corpus's shipped default is `env::temp_dir()` with no space
    // check, at
    // `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`.
    // podvm asks the caller instead, and the caller ranks by free space.
    let got = Store::default_root(|| Some("/from-the-probe".into())).unwrap();
    assert!(
        got.starts_with(
            std::env::var_os("PODVM_STORE")
                .map(PathBuf::from)
                .unwrap_or(PathBuf::from(std::env::var_os("HOME").unwrap_or_default()))
        ) || got.starts_with("/from-the-probe"),
        "{}",
        got.display()
    );
}

/// ⛔ T-1310's pin on the ceiling: sixteen slots, and the seventeenth is
/// refused by name.
///
/// Held under `STORE_TESTS`, so no other test in this process holds a slot
/// and the count is deterministic rather than scheduling-dependent. Without
/// the mutex this test is itself flaky: neighbours holding slots make the
/// table fill early and the refusal arrive before the sixteenth. That
/// flakiness is the signal the mutex works, and the suite passing is the
/// signal the table is big enough for one test's needs.
///
/// ⚠ `/dev/null` descriptors stand in for locks: what is counted here is
/// slots, not locks, and opening the real lock files would also contend
/// with the store under test.
#[test]
fn the_seventeenth_concurrent_registration_is_refused_by_name() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⛔ The value in two places, with the check that they agree: sixteen is
    // the contract T-0207 is bounded by, so a change to the constant breaks
    // here on purpose rather than drifting past it in silence.
    assert_eq!(
        sys::FORK_CLOSE_SLOTS,
        16,
        "the ceiling moved: update T-0207's contract and this message with it"
    );
    let null = sys::CBuf::new("/dev/null").expect("/dev/null");
    let mut held = Vec::new();
    for _ in 0..sys::FORK_CLOSE_SLOTS {
        let fd = sys::open(&null, sys::O_RDONLY, 0).expect("open /dev/null");
        assert!(
            sys::close_in_children(fd),
            "slot table filled early: another holder in this process took slots \
                 outside STORE_TESTS"
        );
        held.push(fd);
    }
    let extra = sys::open(&null, sys::O_RDONLY, 0).expect("open /dev/null");
    assert!(
        !sys::close_in_children(extra),
        "a seventeenth slot was taken: the ceiling moved, and T-0207's \
             contract names sixteen"
    );
    for fd in held {
        sys::stop_closing_in_children(fd);
        let _ = sys::close(fd);
    }
    sys::stop_closing_in_children(extra);
    let _ = sys::close(extra);
    // ⛔ And a slot is usable again afterwards, so the test leaves the table
    // as it found it rather than consuming sixteen slots for the rest of
    // the run.
    let fd = sys::open(&null, sys::O_RDONLY, 0).expect("open /dev/null");
    assert!(sys::close_in_children(fd), "a freed slot was not reusable");
    sys::stop_closing_in_children(fd);
    let _ = sys::close(fd);
}

/// T-1337: reclaimable sizing matches `delete` without unlinking: a
/// shared blob counts once, and only where no survivor needs it.
#[test]
fn reclaimable_counts_unshared_blobs_once() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("reclaimable");
    let shared = b"shared layer bytes";
    let a_only = b"a private layer";
    let b_only = b"b private layer!!";
    let ds = Digest::of(shared);
    let da = Digest::of(a_only);
    let db = Digest::of(b_only);
    s.put_bytes(shared, &ds, "blob").unwrap();
    s.put_bytes(a_only, &da, "blob").unwrap();
    s.put_bytes(b_only, &db, "blob").unwrap();
    let mut ra = record("t/a", Some("1"), 7);
    ra.layers = vec![ds.to_string(), da.to_string()];
    let mut rb = record("t/b", None, 8);
    rb.layers = vec![ds.to_string(), db.to_string()];
    s.put_record(ra).unwrap();
    s.put_record(rb).unwrap();
    let all = s.list().unwrap();
    let rb = all.iter().find(|r| r.tag.is_none()).unwrap().clone();
    // The shared blob survives with A: only B's own layer counts.
    assert_eq!(
        s.reclaimable_bytes(std::slice::from_ref(&rb)).unwrap(),
        b_only.len() as u64
    );
    // Both doomed: every blob once, the shared one included.
    let ra = all.iter().find(|r| r.tag.is_some()).unwrap().clone();
    assert_eq!(
        s.reclaimable_bytes(&[ra, rb]).unwrap(),
        (shared.len() + a_only.len() + b_only.len()) as u64
    );
    // A missing blob counts nothing rather than failing the report.
    assert_eq!(
        s.blob_bytes("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
        0
    );
    assert_eq!(s.blob_bytes("bogus"), 0);
    let _ = std::fs::remove_dir_all(s.root());
}
