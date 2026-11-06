use super::*;

impl Store {
    /// A staging file inside the store, for a blob whose digest is not yet
    /// proved. ⚠ Inside the store because the commit is a `rename(2)`, and a
    /// rename across filesystems fails with `EXDEV`.
    ///
    /// ⛔ **Invariant I6: unique per CALL.** The name carried only the pid until
    /// 2026-09-09, and `TODO/probe.md` T-0113 is that same defect one crate
    /// over: two threads of one process take one name and overwrite each other.
    /// `cargo test` runs tests in threads, and [T-0210](../../../TODO/image.md)'s
    /// own bounded-concurrency sibling would make it reachable in production.
    ///
    /// ⛔ **Invariant I5: the writer holds it.** The returned handle carries an
    /// exclusive `flock` for as long as it lives, which is what lets
    /// [`Store::sweep_staging`] tell an abandoned file from one being written
    /// without asking about a pid.
    pub fn stage(&self, hint: &str) -> Result<(PathBuf, StagedFile)> {
        let safe: String = hint
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .take(24)
            .collect();
        let path = self.root.join(STAGING).join(format!(
            "{safe}.{}.{}.partial",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let f = StagedFile::create(&path)?;
        Ok((path, f))
    }

    /// ⛔ **Invariant I5.** Remove every abandoned staging file, deciding what
    /// is abandoned by trying to lock it.
    ///
    /// ⚠ Returns what it removed rather than printing: the caller decides
    /// whether a sweep is worth a line, and `Store::open` runs on every command.
    /// ⛔ Only inside this store's own `staging/`, resolved through
    /// [`crate::contain`], because a sweep is an unlink and every unlink this
    /// crate performs is gated the same way.
    pub fn sweep_staging(&self) -> Vec<PathBuf> {
        let dir = self.root.join(STAGING);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut swept = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("partial") {
                continue;
            }
            let Ok(resolved) = contain::within(&self.root, &path) else {
                continue;
            };
            // ⭐ The whole test: a live writer holds this file, so a lock this
            // process can take means nobody is writing it.
            match Lock::try_acquire(&resolved, sys::LOCK_EX) {
                Ok(Some(_held)) => {
                    if std::fs::remove_file(&resolved).is_ok() {
                        swept.push(resolved);
                    }
                }
                _ => continue,
            }
        }
        swept
    }

    /// Name a staged file by its proved digest. ⛔ The caller has already
    /// verified it: [`crate::digest::Verifier`] is what produces the digest
    /// passed here, so a blob that failed verification has no path to this
    /// function.
    pub fn commit(&self, staged: &Path, d: &Digest) -> Result<()> {
        let dest = self.blob_path(d);
        contain::within(&self.root, &dest)?;
        std::fs::rename(staged, &dest).map_err(|e| Error::io(dest.display().to_string(), e))
    }

    /// A small blob already in memory, verified before it is written.
    pub fn put_bytes(&self, bytes: &[u8], want: &Digest, what: &str) -> Result<()> {
        let got = Digest::of(bytes);
        if got != *want {
            return Err(Error::DigestMismatch {
                what: what.to_string(),
                want: want.to_string(),
                got: got.to_string(),
            });
        }
        let (staged, mut f) = self.stage(what)?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| Error::io(staged.display().to_string(), e))?;
        drop(f);
        self.commit(&staged, want)
    }

    pub fn read_blob(&self, d: &Digest) -> Result<Vec<u8>> {
        let p = self.blob_path(d);
        let bytes = std::fs::read(&p).map_err(|e| Error::io(p.display().to_string(), e))?;
        // ⛔ Re-verified on the way out. `docs/conventions/code.md`: a guard
        // re-checking an invariant the happy path already knows earns its keep
        // the one time it fires, and here it fires when something outside
        // podvm has edited the store.
        let got = Digest::of(&bytes);
        if got != *d {
            return Err(Error::DigestMismatch {
                what: format!("stored blob {}", p.display()),
                want: d.to_string(),
                got: got.to_string(),
            });
        }
        Ok(bytes)
    }

    // ------------------------------------------------------------- the locks

    /// The store-wide lock, held across an index read-modify-write.
    pub fn lock(&self) -> Result<Lock> {
        Lock::acquire(&self.root.join(STORE_LOCK), sys::LOCK_EX)
    }

    pub(super) fn image_lock_path(&self, record: &Record) -> Result<PathBuf> {
        let d = Digest::parse(&record.digest)?;
        Ok(self.root.join(LOCKS).join(format!("{}.lock", d.hex())))
    }

    /// ⭐ T-0204's mechanism. A **shared** advisory lock, released only when the
    /// last holder's fd is closed, however that process ends. Several
    /// containers may share one image, so the lock is shared; the GC asks for
    /// an exclusive one and is refused while any holder remains.
    ///
    /// ⛔ Two defences, because they cover different failures, and
    /// [`TODO/image.md`](../../../TODO/image.md) T-0211 is what having neither
    /// cost. `O_CLOEXEC` keeps the lock out of an unrelated **exec**, and
    /// registering the fd with [`sys::close_in_children`] keeps it out of an
    /// unrelated **fork**, which `O_CLOEXEC` cannot do, because there is no
    /// close-on-fork and `flock` is held on the open file description a fork
    /// duplicates. The one caller that wants a payload to inherit it says so
    /// with [`Lock::hand_to_payload`], which undoes both.
    ///
    /// ⚠ Both defences come from [`Lock`] itself now, and neither is this
    /// function's to take: `Lock::open` sets `O_CLOEXEC` and
    /// [`Lock::try_acquire`] registers. T-0215 is why this one is no longer
    /// special.
    pub fn hold(&self, record: &Record) -> Result<Lock> {
        let path = self.image_lock_path(record)?;
        // ⛔ INVARIANT I4 AND I7. The index lock is taken first and dropped at
        // the end of this function, so a `prune` cannot be between its own
        // `in_use` check and its unlink while this hold is being taken. Without
        // it the check is stale before it is used: measured as a reading of the
        // code on 2026-09-09, `prune` asked `in_use`, a `run` took its hold, and
        // `prune` then deleted the blobs the run was about to execute out of
        // AND the lock file it was holding. Both take index-then-image, so
        // neither can wait on the other.
        let _index = self.lock()?;
        Lock::acquire(&path, sys::LOCK_SH)
    }

    /// Whether any process holds [`Store::hold`] on this image.
    ///
    /// ⚠ Tested by asking for the exclusive lock and reading `EWOULDBLOCK`,
    /// then dropping it immediately. There is no way to ask "is this locked"
    /// without trying, and a pid file that could be asked is the stale-state
    /// race this replaces.
    pub fn in_use(&self, record: &Record) -> Result<bool> {
        let path = self.image_lock_path(record)?;
        if !path.exists() {
            return Ok(false);
        }
        match Lock::try_acquire(&path, sys::LOCK_EX) {
            Ok(Some(_probe)) => Ok(false),
            Ok(None) => Ok(true),
            Err(e) => Err(e),
        }
    }
}
