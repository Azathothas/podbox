//! `TODO/image.md` T-0202 and T-0204: one content-addressed store, shared by
//! images and containers, with a lock.
//!
//! ⛔ **Every blob is verified as it is written, never afterwards.** The reason
//! is in the corpus at
//! `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:149-157`, which
//! checks a payload's content hash before an in-memory exec so an in-memory
//! exec never runs unverified bytes. A layer about to be extracted as root is
//! the same case. Bytes that fail verification never reach `blobs/`: they are
//! written to a staging name inside the store and renamed in only on success.
//!
//! ⛔ **Images are keyed by manifest digest**, and the digest recorded is the
//! one computed over the bytes the registry served for the reference that was
//! asked for. That is what `docker image inspect` reports in `RepoDigests`, and
//! parity with it is what M1 is accepted on ([`TODO/milestones.md`](../../../TODO/milestones.md) T-1102).
//!
//! ⚠ One store for images and containers, not one per container. The GC race
//! that decides it is T-0204, and the answer is [`Lock`]: an advisory lock on an
//! fd rather than a pid file, because a pid file is stale the moment a process
//! dies unexpectedly and the check that clears a stale one is the race being
//! closed.
//!
//! ⛔ That fd is `O_CLOEXEC`, and the one caller that wants the payload to
//! inherit it says so with [`Lock::hand_to_payload`] immediately before its
//! `execve`. T-0211 is what the other shape cost: an fd left inheritable from
//! open time is inherited by every unrelated `fork` while the lock is held, and
//! each such child keeps the `flock` alive for its own lifetime.
//!
//! # The concurrency contract
//!
//! ⭐ **[`TODO/image.md`](../../../TODO/image.md) T-0210, and it is written here
//! rather than in the entry because a contract nobody reads while changing the
//! code is not one.** Two concurrency defects landed in this crate before it
//! existed ([T-0211](../../../TODO/image.md), and `TODO/probe.md` T-0113 in the
//! probe), and the second was found by luck. Every invariant below is driven by
//! `experiments/210-store-concurrency.sh` against real concurrent processes,
//! because a race only a mock can produce is a race the mock's author imagined.
//!
//! **I1. One writer at a time, over the index.** Every read-modify-write of
//! `store.json` happens under [`Store::lock`], an exclusive `flock` on
//! `$store/lock`, waited for a bounded number of times.
//!
//! **I2. A reader needs no lock, and never sees a half-written index.** The
//! index is replaced by [`std::fs::rename`], which is atomic within a
//! filesystem, so a reader sees the previous document or the next one. ⚠ This is
//! why `staging/` is inside the store: a rename across filesystems is `EXDEV`.
//!
//! **I3. A blob is immutable once it is named.** `blobs/` is content-addressed,
//! so two writers racing to produce one name necessarily produce identical
//! bytes and the second rename is a no-op. Bytes that fail verification never
//! get a name. A reader therefore needs no lock for a blob either: it sees the
//! file complete, or not at all.
//!
//! **I4. Nothing deletes a blob a holder needs.** `rmi` and `prune` take the
//! index lock and check [`Store::in_use`] **inside it**, and [`Store::hold`]
//! takes that same lock while it acquires the image lock. ⛔ Checking `in_use`
//! outside the lock is a check whose answer is stale before it is used: a
//! `hold` taken in between kept its image lock and lost its blobs.
//!
//! **I5. A killed process leaves exactly one kind of litter, and it is swept.**
//! Blobs are only ever named by rename after verification and the index only
//! ever replaced by rename, so the only thing a `SIGKILL` can leave is a
//! `*.partial` under `staging/`. [`Store::open`] sweeps them, and it decides
//! what is orphaned by **trying to `flock` each one**: a live writer holds its
//! own staging file for as long as it is writing, so a file this process can
//! lock is a file nobody is writing. ⚠ Never by pid: a pid is reused, and the
//! check that clears a stale pid file is itself the race being closed.
//!
//! **I6. A staging name is unique per CALL, not per process.** ⛔ `TODO/probe.md`
//! T-0113 is this exact defect one crate over: a scratch name carrying only the
//! pid collides between two threads of one process, and `cargo test` and any
//! future concurrent layer fetch ([T-0207](../../../TODO/image.md)) are both
//! threads of one process.
//!
//! **I7. Locks are taken in one order: the index, then an image.** Both `prune`
//! and `hold` take them that way, so neither can wait on the other.

use std::io::Write;
use std::path::{Path, PathBuf};

use podbox_probe::sys::{self, CBuf};
use serde::{Deserialize, Serialize};

use crate::clock;
use crate::contain;
use crate::digest::Digest;
use crate::error::{Error, Result};
use crate::oci;
use crate::reference::Reference;

/// ⭐ A version discriminator on everything persisted.
/// `docs/conventions/code.md`: old data still reads and new code knows which
/// version it is looking at. A store written by a later podbox is refused by
/// name rather than parsed as though its fields meant what they mean here.
pub const INDEX_VERSION: u32 = 1;
const INDEX_FILE: &str = "store.json";
const BLOBS: &str = "blobs";
const LOCKS: &str = "locks";
const STAGING: &str = "staging";
const STORE_LOCK: &str = "lock";

/// How long the index lock is waited for before podbox says who is holding it.
/// ⚠ Bounded, per `RULES.md` section 8: a runtime whose audience is automated
/// may not wait unbounded.
const LOCK_ATTEMPTS: u32 = 100;
const LOCK_SLEEP: std::time::Duration = std::time::Duration::from_millis(50);

/// ⛔ Invariant I6. What makes a staging name unique per CALL rather than per
/// process, which is the difference `TODO/probe.md` T-0113 cost one crate over.
static STAGE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// `<domain>/<repository>`, canonical. ⛔ Never the display form: `alpine`
    /// and `docker.io/library/alpine` are one image and a store keyed on the
    /// display form holds it twice.
    pub repository: String,
    pub tag: Option<String>,
    /// What the reference resolved to, computed over the served bytes. This is
    /// the value `podbox images --format '{{.Digest}}'` prints.
    pub digest: String,
    pub digest_media_type: String,
    /// The single-platform manifest under it. Equal to `digest` when the
    /// registry served a manifest rather than an index.
    pub manifest_digest: String,
    /// docker's image ID: the digest of the config blob.
    pub config_digest: String,
    /// ⛔ Recorded, because `docs/conventions/forbidden-patterns.md`: fetching a
    /// variant into a cache keyed without the variant serves the variant to the
    /// next unqualified fetch.
    pub platform: String,
    pub layers: Vec<String>,
    /// What the layers and the config occupy in `blobs/`, summed from the
    /// descriptors and checked against what arrived.
    pub stored_bytes: u64,
    pub architecture: String,
    pub os: String,
    /// From the image config. `None` where the config declares none.
    pub created: Option<String>,
    pub pulled_at: String,
}

impl Record {
    /// Everything in `blobs/` this record needs.
    pub fn blobs(&self) -> Vec<&str> {
        let mut v: Vec<&str> = vec![
            self.digest.as_str(),
            self.manifest_digest.as_str(),
            self.config_digest.as_str(),
        ];
        v.extend(self.layers.iter().map(String::as_str));
        v.sort_unstable();
        v.dedup();
        v
    }

    /// How docker prints the repository column.
    pub fn display_repository(&self) -> String {
        match self.repository.strip_prefix("docker.io/") {
            Some(rest) => match rest.strip_prefix("library/") {
                Some(short) if !short.contains('/') => short.to_string(),
                _ => rest.to_string(),
            },
            None => self.repository.clone(),
        }
    }

    pub fn name(&self) -> String {
        match &self.tag {
            Some(t) => format!("{}:{t}", self.display_repository()),
            None => format!("{}@{}", self.display_repository(), self.digest),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    pub podbox_store: u32,
    #[serde(default)]
    pub images: Vec<Record>,
}

impl Default for Index {
    fn default() -> Index {
        Index {
            podbox_store: INDEX_VERSION,
            images: Vec::new(),
        }
    }
}

/// What a platform-qualified lookup found.
///
/// ⛔ Two states rather than an empty vector, because they need different
/// sentences: "podbox does not hold this image" and "podbox holds this image,
/// for another platform" send a caller to different remedies.
#[derive(Debug, Default)]
pub struct Found {
    pub matched: Vec<Record>,
    /// The platforms the store DOES hold for this reference, when none matched.
    pub other_platforms: Vec<String>,
}

impl Found {
    pub fn one(self) -> Option<Record> {
        self.matched.into_iter().next()
    }
}

#[derive(Debug)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Where the store lives, and why.
    ///
    /// ⚠ `$PODBOX_STORE` first, then the XDG data directory, then `$HOME`.
    /// Deliberately **not** `$TMPDIR`: on the runtime podbox targets `/tmp` is
    /// 64 MiB, which almost no image fits in, and `env::temp_dir()` with no
    /// space check is the shipped default this project read in the corpus at
    /// `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`.
    /// Where none of the three is set, `fallback` is consulted; the caller
    /// supplies the probe's write allowlist, ranked by free space (T-0203).
    pub fn default_root(fallback: impl FnOnce() -> Option<String>) -> Result<PathBuf> {
        if let Some(p) = std::env::var_os("PODBOX_STORE") {
            let p = PathBuf::from(p);
            if !p.as_os_str().is_empty() {
                return Ok(p);
            }
        }
        for (var, suffix) in [("XDG_DATA_HOME", "podbox"), ("HOME", ".local/share/podbox")] {
            if let Some(base) = std::env::var_os(var) {
                let base = PathBuf::from(base);
                if base.is_absolute() {
                    return Ok(base.join(suffix));
                }
            }
        }
        match fallback() {
            Some(p) => Ok(PathBuf::from(p).join("podbox")),
            None => Err(Error::Store(
                "no store directory: $PODBOX_STORE, $XDG_DATA_HOME and $HOME are \
                 all unset or relative, and no probed writable path could hold \
                 one. Set $PODBOX_STORE to a directory podbox may write"
                    .into(),
            )),
        }
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Store> {
        let root = root.into();
        for d in [
            root.clone(),
            root.join(BLOBS).join(crate::digest::SHA256),
            root.join(LOCKS),
            root.join(STAGING),
        ] {
            std::fs::create_dir_all(&d).map_err(|e| Error::io(d.display().to_string(), e))?;
        }
        let store = Store { root };
        // ⛔ Refuse a store a later podbox wrote, rather than reading its fields
        // as though they meant what they mean here.
        let index = store.read_index()?;
        if index.podbox_store > INDEX_VERSION {
            return Err(Error::Store(format!(
                "{} declares store version {} and this podbox writes {INDEX_VERSION}. \
                 A newer podbox made it; this one will not edit it",
                store.index_path().display(),
                index.podbox_store
            )));
        }
        // ⛔ Invariant I5. Every command opens the store, so this is where an
        // abandoned staging file is reclaimed. `docs/conventions/code.md` calls
        // it the sweep that heals the drift the happy path let slip.
        store.sweep_staging();
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILE)
    }

    pub fn blob_path(&self, d: &Digest) -> PathBuf {
        self.root.join(d.blob_path())
    }

    pub fn has_blob(&self, d: &Digest) -> bool {
        self.blob_path(d).is_file()
    }

    pub fn read_index(&self) -> Result<Index> {
        let path = self.index_path();
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                Error::Store(format!(
                    "{} does not parse: {e}. It is podbox's own index; move it \
                     aside to start a fresh store",
                    path.display()
                ))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Index::default()),
            Err(e) => Err(Error::io(path.display().to_string(), e)),
        }
    }

    /// ⛔ Written to a staging name inside the store and renamed over the index,
    /// so a process killed mid-write leaves the previous index intact rather
    /// than a truncated one. `rename(2)` is atomic within a filesystem, which
    /// is why the staging directory is inside the store and not in `/tmp`.
    pub fn write_index(&self, index: &Index) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(index)
            .map_err(|e| Error::Store(format!("serialising the index: {e}")))?;
        // ⛔ Invariants I5 and I6: unique per call, named `.partial` so the one
        // sweep rule covers it, and held while it is written so the sweep
        // cannot take it out from under this write.
        let tmp = self.root.join(STAGING).join(format!(
            "{INDEX_FILE}.{}.{}.partial",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        {
            let mut f = StagedFile::create(&tmp)?;
            f.write_all(&bytes)
                .and_then(|_| f.write_all(b"\n"))
                .and_then(|_| f.sync_all())
                .map_err(|e| Error::io(tmp.display().to_string(), e))?;
        }
        std::fs::rename(&tmp, self.index_path())
            .map_err(|e| Error::io(self.index_path().display().to_string(), e))
    }

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
        // podbox has edited the store.
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

    fn image_lock_path(&self, record: &Record) -> Result<PathBuf> {
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

    // ----------------------------------------------------------- the queries

    /// Every record, newest pull first.
    pub fn list(&self) -> Result<Vec<Record>> {
        let mut images = self.read_index()?.images;
        images.sort_by(|a, b| {
            b.pulled_at
                .cmp(&a.pulled_at)
                .then(a.repository.cmp(&b.repository))
                .then(a.tag.cmp(&b.tag))
        });
        Ok(images)
    }

    /// Records a reference names. A tag or digest names at most one; a bare
    /// repository or an image id may name several.
    ///
    /// ⚠ Selected by NAME, never by position: `docs/conventions/code.md`.
    pub fn find(&self, want: &str) -> Result<Vec<Record>> {
        let all = self.list()?;
        // An image id, full or docker's twelve-digit short form.
        let id_like = want.len() >= 12
            && want
                .trim_start_matches("sha256:")
                .bytes()
                .all(|b| b.is_ascii_hexdigit());
        if id_like {
            let hex = want.trim_start_matches("sha256:");
            let hits: Vec<Record> = all
                .iter()
                .filter(|r| {
                    r.config_digest
                        .trim_start_matches("sha256:")
                        .starts_with(hex)
                })
                .cloned()
                .collect();
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
        let Ok(reference) = Reference::parse(want) else {
            return Ok(Vec::new());
        };
        let repo = reference.canonical_repository();
        let hits: Vec<Record> = all
            .into_iter()
            .filter(|r| {
                r.repository == repo
                    && match (&reference.digest, &reference.tag) {
                        (Some(d), _) => r.digest == d.to_string(),
                        (None, Some(t)) => r.tag.as_deref() == Some(t.as_str()),
                        (None, None) => true,
                    }
            })
            .collect();
        Ok(hits)
    }

    /// The records a reference names, narrowed to one platform.
    ///
    /// ⛔ `None` means **this machine's**, not "any". A caller that asked for
    /// no platform is asking about the image it could run, and handing it an
    /// arm64 record on an amd64 host is the `Exec format error` this whole
    /// module exists to turn into a sentence. ⚠ Where nothing matches the host
    /// the hits are returned unfiltered rather than emptied, so the caller can
    /// say "podbox holds this image, for another platform" instead of "no such
    /// image", which is a different and more useful failure.
    pub fn find_for(
        &self,
        want: &str,
        platform: Option<&crate::platform::Platform>,
    ) -> Result<Found> {
        let hits = self.find(want)?;
        let host = crate::platform::Platform::host();
        let want_p = platform.unwrap_or(&host);
        // ⛔ Filtered whatever the count. An earlier version short-circuited on
        // a single hit, and a store holding only `linux/amd64` then answered a
        // `--platform linux/arm64` request with the amd64 record: podbox ran the
        // wrong architecture and said nothing. A count is not a match.
        let matched: Vec<Record> = hits
            .iter()
            .filter(|r| r.platform == want_p.to_string())
            .cloned()
            .collect();
        Ok(Found {
            other_platforms: if matched.is_empty() {
                let mut p: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                p.sort_unstable();
                p.dedup();
                p
            } else {
                Vec::new()
            },
            matched,
        })
    }

    pub fn find_one(&self, want: &str) -> Result<Record> {
        self.find_one_for(want, None)
    }

    /// One record, or a refusal that says why there is more than one.
    ///
    /// ⛔ **NEVER `.next()` ON AN AMBIGUOUS REFERENCE.** Until 2026-09-09 this
    /// took the first hit, and that was harmless only because a store could not
    /// hold two records for one tag. [T-0212](../../../TODO/image.md) made it
    /// hold one per platform, and the same line then meant `podbox extract
    /// alpine` silently unpacked whichever platform was pulled most recently:
    /// selected **by position**, which `docs/conventions/code.md` forbids and
    /// which `Store::find`'s own comment calls out three functions above.
    ///
    /// ⚠ The host's platform is preferred rather than demanded, because a store
    /// holding exactly one foreign platform and asked for no platform in
    /// particular is not ambiguous: there is one answer and refusing it would be
    /// pedantry. Ambiguity is two or more surviving that preference.
    pub fn find_one_for(
        &self,
        want: &str,
        platform: Option<&crate::platform::Platform>,
    ) -> Result<Record> {
        let hits = self.find(want)?;
        if hits.is_empty() {
            return Err(Error::NoSuchImage(want.to_string()));
        }
        if hits.len() == 1 && platform.is_none() {
            return Ok(hits.into_iter().next().expect("length checked"));
        }
        let host = crate::platform::Platform::host();
        let prefer = platform.unwrap_or(&host);
        let narrowed: Vec<Record> = hits
            .iter()
            .filter(|r| r.platform == prefer.to_string())
            .cloned()
            .collect();
        match narrowed.len() {
            1 => Ok(narrowed.into_iter().next().expect("length checked")),
            0 if platform.is_some() => {
                let mut have: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                have.sort_unstable();
                have.dedup();
                Err(Error::NoSuchImage(format!(
                    "{want} for {prefer}. The store holds it for {}",
                    have.join(", ")
                )))
            }
            0 if hits.len() == 1 => Ok(hits.into_iter().next().expect("length checked")),
            _ => {
                let mut have: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                have.sort_unstable();
                have.dedup();
                Err(Error::Usage(format!(
                    "{want} names {} images in this store, for {}. podbox will \
                     not pick one by position: name the platform with \
                     --platform, or the image by its digest",
                    hits.len(),
                    have.join(", ")
                )))
            }
        }
    }

    // ------------------------------------------------------------ the writes

    /// Record a pull. Replaces the record for the same repository, tag **and
    /// platform**, because a moving tag is the normal case.
    ///
    /// ⛔ **The platform is part of the key and that is a multi-architecture
    /// decision.** `alpine:latest` for `linux/amd64` and for `linux/arm64` are
    /// two different images that share one name, and keying without the
    /// platform means the second pull silently deletes the first: the record
    /// goes, the blobs are collected, and a caller who pulled both has one.
    /// podman keeps both and so does podbox. `Store::find` is where the
    /// resulting ambiguity is resolved, by name and never by position.
    pub fn put_record(&self, record: Record) -> Result<()> {
        let _guard = self.lock()?;
        let mut index = self.read_index()?;
        index.podbox_store = INDEX_VERSION;
        index.images.retain(|r| {
            !(r.repository == record.repository
                && r.tag == record.tag
                && r.platform == record.platform
                && (record.tag.is_some() || r.digest == record.digest))
        });
        index.images.push(record);
        self.write_index(&index)
    }

    /// `podbox tag <src> <dst>`. The destination points at the same manifest
    /// digest; nothing is fetched and no blob is copied.
    pub fn tag(&self, src: &str, dst: &str) -> Result<Record> {
        let source = self.find_one(src)?;
        let target = Reference::parse(dst)?;
        if target.digest.is_some() {
            return Err(Error::Usage(format!(
                "{dst:?} names a digest. A tag is a name podbox assigns, and a \
                 digest is one the content assigns; the second cannot be set"
            )));
        }
        let record = Record {
            repository: target.canonical_repository(),
            tag: target.tag.clone(),
            pulled_at: clock::now(),
            ..source
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    /// `podbox rmi`. Removes the records a reference names and then every blob
    /// no surviving record reaches.
    ///
    /// ⛔ Refuses while a container holds the image, and says which.
    pub fn remove(&self, want: &str) -> Result<Removed> {
        let doomed = self.find(want)?;
        if doomed.is_empty() {
            return Err(Error::NoSuchImage(want.to_string()));
        }
        // ⛔ Invariant I4: the `in_use` check is inside `delete`, under the
        // index lock, and never here. This function used to make it and it was
        // stale by the time `delete` acted on it.
        self.delete(&doomed, Held::Refuse)
    }

    /// `podbox image prune`. Without `all`, only untagged records; with it,
    /// every record no container holds.
    pub fn prune(&self, all: bool) -> Result<Removed> {
        let mut doomed = Vec::new();
        for r in self.list()? {
            if !all && r.tag.is_some() {
                continue;
            }
            doomed.push(r);
        }
        // ⛔ Invariant I4: what is held is decided under the lock, by `delete`,
        // and `Held::Skip` is what makes a held image a named skip here where
        // `rmi` refuses outright.
        self.delete(&doomed, Held::Skip)
    }

    fn delete(&self, doomed: &[Record], on_held: Held) -> Result<Removed> {
        let _guard = self.lock()?;
        // ⛔ INVARIANT I4. Inside the lock, so a `hold` cannot be taken between
        // this answer and the unlinks below: `Store::hold` takes the same lock.
        let mut skipped = Vec::new();
        let mut kept = Vec::new();
        for r in doomed {
            if self.in_use(r)? {
                match on_held {
                    Held::Refuse => {
                        return Err(Error::Store(format!(
                            "{} is in use by a running container and was not removed. \
                             A GC that deletes an extraction out from under a payload \
                             makes its failure read as a missing file rather than as a \
                             concurrent deletion (TODO/image.md T-0204)",
                            r.name()
                        )))
                    }
                    Held::Skip => {
                        skipped.push(r.name());
                        continue;
                    }
                }
            }
            kept.push(r.clone());
        }
        let doomed: &[Record] = &kept;
        let mut index = self.read_index()?;
        let doomed_keys: Vec<(String, Option<String>, String)> = doomed
            .iter()
            .map(|r| (r.repository.clone(), r.tag.clone(), r.digest.clone()))
            .collect();
        index.images.retain(|r| {
            !doomed_keys
                .iter()
                .any(|(repo, tag, dig)| r.repository == *repo && r.tag == *tag && r.digest == *dig)
        });

        // ⭐ Reachability over what SURVIVES, not over what was deleted. A blob
        // several tags share is kept while any of them remains, and computing
        // the doomed set instead would delete it with the first.
        let mut keep: Vec<String> = Vec::new();
        for r in &index.images {
            keep.extend(r.blobs().into_iter().map(str::to_string));
        }
        let mut freed_bytes = 0u64;
        let mut freed = Vec::new();
        for r in doomed {
            for b in r.blobs() {
                if keep.iter().any(|k| k == b) || freed.iter().any(|f| f == b) {
                    continue;
                }
                let d = Digest::parse(b)?;
                let path = self.blob_path(&d);
                // ⛔ One gate per action: every unlink this crate performs is
                // resolved against the store root first.
                let resolved = contain::within(&self.root, &path)?;
                if let Ok(meta) = std::fs::metadata(&resolved) {
                    freed_bytes += meta.len();
                }
                match std::fs::remove_file(&resolved) {
                    Ok(()) => freed.push(b.to_string()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => freed.push(b.to_string()),
                    Err(e) => return Err(Error::io(resolved.display().to_string(), e)),
                }
            }
            let lock = self.image_lock_path(r)?;
            if lock.exists() {
                let resolved = contain::within(&self.root, &lock)?;
                let _ = std::fs::remove_file(resolved);
            }
        }
        self.write_index(&index)?;
        Ok(Removed {
            untagged: doomed.iter().map(Record::name).collect(),
            deleted: freed,
            freed_bytes,
            skipped,
        })
    }
}

/// What [`Store::delete`] does about an image a holder is using.
///
/// ⛔ Two behaviours and one check, because the check has to happen under the
/// index lock (invariant I4) and only the caller knows whether being held is a
/// refusal (`rmi`, which names one image) or a skip (`prune`, which sweeps).
#[derive(Debug, Clone, Copy)]
enum Held {
    Refuse,
    Skip,
}

#[derive(Debug)]
pub struct Removed {
    pub untagged: Vec<String>,
    pub deleted: Vec<String>,
    pub freed_bytes: u64,
    /// ⛔ Named, never silent. T-0204: `prune` and `rmi` skip anything locked
    /// **and say which**.
    pub skipped: Vec<String>,
}

/// A staging file, held exclusively for as long as this value lives.
///
/// ⛔ **Invariant I5's mechanism.** [`Store::sweep_staging`] decides what is
/// abandoned by trying to lock each `*.partial`, so a file being written has to
/// be locked or the sweep would delete it out from under its writer. The lock
/// goes when this value does, however the process ends.
///
/// ⚠ **Two open file descriptions on one inode, not one.** `flock` is held on
/// the description [`Lock`] opened, and the writes go through a second one this
/// type opens after it. They are independent: closing the writer releases no
/// lock, and the lock's own `Drop` is the only thing that does. ⛔ So a
/// `StagedFile` costs two descriptors, and a reader counting descriptors while
/// chasing a lock that outlived its holder has to know that
/// ([`TODO/image.md`](../../../TODO/image.md) T-0215).
pub struct StagedFile {
    file: std::fs::File,
    /// ⚠ Held for its `Drop`, and never read. The lock is the point.
    _lock: Lock,
}

impl StagedFile {
    fn create(path: &Path) -> Result<StagedFile> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io(parent.display().to_string(), e))?;
        }
        // ⛔ The lock FIRST. Creating the file and locking it afterwards leaves
        // a window in which a sweep sees an unlocked `*.partial` and removes it.
        let lock = match Lock::try_acquire(path, sys::LOCK_EX)? {
            Some(l) => l,
            None => {
                return Err(Error::Store(format!(
                    "{} is already being written by another podbox. Invariant I6 \
                     makes this name unique per call, so two writers on one name is \
                     a defect rather than contention",
                    path.display()
                )))
            }
        };
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| Error::io(path.display().to_string(), e))?;
        Ok(StagedFile { file, _lock: lock })
    }

    pub fn sync_all(&self) -> std::io::Result<()> {
        self.file.sync_all()
    }
}

impl Write for StagedFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// ⭐ [`TODO/image.md`](../../../TODO/image.md) T-0214. A transfer that was cut
/// short starts again on this same file, under the same lock and at the same
/// name: the alternative is a second staging path per attempt, and a sweep that
/// has to tell them apart.
///
/// ⛔ Truncate AND rewind. `set_len(0)` alone leaves the offset where the failed
/// attempt left it, so the next byte lands two megabytes in and the file is a
/// hole followed by the retry.
impl crate::registry::Restart for StagedFile {
    fn restart(&mut self) -> std::io::Result<()> {
        use std::io::Seek;
        self.file.set_len(0)?;
        self.file.seek(std::io::SeekFrom::Start(0))?;
        Ok(())
    }
}

/// An advisory lock held for as long as this value lives.
pub struct Lock {
    fd: i64,
    pub path: PathBuf,
    /// ⛔ Set by [`Lock::hand_to_payload`], and it turns the explicit release in
    /// `Drop` OFF. A handed lock is meant to outlive this process in the
    /// payload, and `LOCK_UN` would take it from the payload as well: the
    /// payload holds a duplicate of THIS open file description, and a lock
    /// belongs to the description rather than to a descriptor.
    handed: std::sync::atomic::AtomicBool,
}

impl Lock {
    fn open(path: &Path) -> Result<i64> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io(parent.display().to_string(), e))?;
        }
        let Some(c) = CBuf::new(&path.to_string_lossy()) else {
            return Err(Error::Store(format!(
                "{} contains a NUL and cannot reach the kernel",
                path.display()
            )));
        };
        // ⛔ `O_CLOEXEC`, like every other fd in this tree. T-0204's mechanism
        // wants the image lock to survive **one** exec, and T-0211 is what it
        // cost to get that by leaving the fd inheritable from the moment it was
        // opened: every unrelated fork in between inherits it too.
        // [`Lock::hand_to_payload`] is the deliberate act, made one call before
        // the `execve` it is for.
        let flags = sys::O_RDWR | sys::O_CREAT | sys::O_CLOEXEC;
        sys::open(&c, flags, 0o644).map_err(|e| {
            Error::Store(format!(
                "opening the lock {}: {} ({})",
                path.display(),
                e.name(),
                e.0
            ))
        })
    }

    /// `None` where somebody else holds it. ⛔ Always `LOCK_NB`: a blocking
    /// `flock` on a lock held through an exec is an unbounded wait, which
    /// `RULES.md` section 8 forbids.
    /// An exclusive lock, or `None` where somebody holds it.
    ///
    /// ⚠ Public because `podbox-supervise` reconciles its container table with
    /// exactly this question ([`TODO/supervise.md`](../../../TODO/supervise.md)
    /// T-0604): a launcher holds its container's lock for its whole life, so a
    /// lock another process can take is a launcher that is gone. ⛔ The same
    /// mechanism and not a second one: a pid file would be stale the moment a
    /// launcher is killed, and the check that clears a stale one is the race.
    pub fn try_exclusive(path: &Path) -> Result<Option<Lock>> {
        Lock::try_acquire(path, sys::LOCK_EX)
    }

    /// Take an exclusive lock, waiting the bounded number of attempts.
    pub fn exclusive(path: &Path) -> Result<Lock> {
        Lock::acquire(path, sys::LOCK_EX)
    }

    /// ⛔ **Every lock this type takes is registered for shedding here, and
    /// this is the only place a `Lock` is built.** A caller cannot forget,
    /// because a caller is not asked.
    /// [`TODO/image.md`](../../../TODO/image.md) T-0215 is what the previous
    /// shape cost: only [`Store::hold`] registered, so a staging lock, an index
    /// lock and a container lock were each inherited by every child a
    /// concurrent `clone_fork` made, and the child then held the `flock` for
    /// its own lifetime. The image read as in use after its holder released it.
    ///
    /// ⚠ The `Lock` is built BEFORE the registration is checked, so the fd is
    /// closed by its own `Drop` on the refusal path rather than leaked.
    fn try_acquire(path: &Path, op: u64) -> Result<Option<Lock>> {
        let fd = Lock::open(path)?;
        match sys::flock(fd, op | sys::LOCK_NB) {
            Ok(_) => {
                let lock = Lock {
                    fd,
                    path: path.to_path_buf(),
                    handed: std::sync::atomic::AtomicBool::new(false),
                };
                if !sys::close_in_children(fd) {
                    return Err(Error::Store(format!(
                        "this process already holds {} locks, which is every slot \
                         podbox has for fds a fork must shed. One more would leak \
                         into every child forked from here (T-0211), so it is \
                         refused rather than held unsafely",
                        sys::FORK_CLOSE_SLOTS
                    )));
                }
                Ok(Some(lock))
            }
            Err(sys::EWOULDBLOCK) => {
                // ⛔ **THE ONLY MOMENT A HOLDER WOULD BE THERE.**
                // [`TODO/image.md`](../../../TODO/image.md) T-0215: every
                // instrument that ran from the failing assertion arrived
                // microseconds late and reported "nobody". This runs at the
                // `EWOULDBLOCK` itself. ⚠ Test builds only, and silent unless
                // `PODBOX_T0215_CAPTURE` is set.
                #[cfg(test)]
                tests::note_the_refusal(path, fd);
                let _ = sys::close(fd);
                Ok(None)
            }
            Err(e) => {
                let _ = sys::close(fd);
                Err(Error::Store(format!(
                    "flock({}): {} ({})",
                    path.display(),
                    e.name(),
                    e.0
                )))
            }
        }
    }

    fn acquire(path: &Path, op: u64) -> Result<Lock> {
        for _ in 0..LOCK_ATTEMPTS {
            if let Some(l) = Lock::try_acquire(path, op)? {
                return Ok(l);
            }
            std::thread::sleep(LOCK_SLEEP);
        }
        Err(Error::Store(format!(
            "{} is held by another podbox after {:?}. Another pull or prune is \
             running; podbox waits a bounded time and then says so rather than \
             blocking (RULES.md section 8)",
            path.display(),
            LOCK_SLEEP * LOCK_ATTEMPTS
        )))
    }

    /// Hand this one lock to the payload, and to nothing else.
    ///
    /// ⭐ T-0204 needs the image lock to outlive `podbox` itself: the guard is
    /// what stops a concurrent `rmi` or `prune` deleting a rootfs a running
    /// container is executing out of, and podbox is not the process that holds
    /// the container open. This undoes both of [`Store::hold`]'s defences for
    /// this one descriptor, it stops being shed by `clone_fork` and its
    /// `FD_CLOEXEC` is cleared, so the very next `fork` and `execve` carry it
    /// into the payload.
    ///
    /// ⛔ Called immediately before the fork that leads to that `execve`, never
    /// at open time. T-0211 is what the second shape costs: a lock that is
    /// inheritable for its whole life is inherited by every unrelated `fork` in
    /// that window, in this tree, by the fifty short-lived children one
    /// `podbox probe` makes, and each one holds the `flock` open for its own
    /// lifetime. The image then reads as in use after its holder released it,
    /// and `rmi` refuses an image nothing is using.
    ///
    /// ⚠ The returned fd is deliberately raw: its consumer is the code between
    /// `fork` and `execve`, where allocating is not allowed.
    pub fn hand_to_payload(&self) -> Result<i64> {
        // ⛔ Order matters. The fd stops being shed only after `FD_CLOEXEC` is
        // cleared, so a fork racing this call either sheds it or inherits a
        // descriptor that is still close-on-exec. Neither outcome leaks a lock.
        let flags = sys::fcntl(self.fd, sys::F_GETFD, 0).map_err(|e| {
            Error::Store(format!(
                "reading the descriptor flags of {}: {} ({})",
                self.path.display(),
                e.name(),
                e.0
            ))
        })?;
        let cleared = (flags as u64) & !sys::FD_CLOEXEC;
        sys::fcntl(self.fd, sys::F_SETFD, cleared).map_err(|e| {
            Error::Store(format!(
                "clearing FD_CLOEXEC on {}: {} ({})",
                self.path.display(),
                e.name(),
                e.0
            ))
        })?;
        sys::stop_closing_in_children(self.fd);
        // ⛔ And this one is never released explicitly. `Drop` calls `LOCK_UN`
        // on every other lock so the release finishes in the releasing thread
        // (T-0215), and doing it here would take the lock away from the payload
        // that is meant to keep it: the payload holds a duplicate of THIS open
        // file description, and a lock belongs to the description.
        self.handed
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(self.fd)
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // ⛔ Deregistered before the close, so a fork racing this never sheds a
        // descriptor number that has already been handed back to the kernel and
        // reused by another thread.
        sys::stop_closing_in_children(self.fd);

        // ⛔ **THE RELEASE IS EXPLICIT, AND `close` ALONE WAS THE DEFECT.**
        // [`TODO/image.md`](../../../TODO/image.md) T-0215. A `close` releases
        // the lock only when it drops the LAST reference to the open file
        // description. A `fork` makes a second reference, so after one the
        // holder's own `close` no longer completes the release: the lock record
        // is taken away later, when the child's copy goes, and that happens
        // asynchronously. ⚠ Measured on 2026-09-12: a `flock` refused with NO
        // row in `/proc/locks` and NO descriptor on the inode in any process on
        // the host, and the SAME descriptor succeeded on the next attempt, 1 us
        // later. The worst transient ran 1190 us. `in_use` read those as an
        // image in use after its holder released it.
        // ⭐ `LOCK_UN` removes the record HERE, in this thread, whatever else
        // holds a reference, so the release is finished when this line is.
        if !self.handed.load(std::sync::atomic::Ordering::Acquire) {
            let _ = sys::flock(self.fd, sys::LOCK_UN);
        }

        // ⚠ The close still matters and is not replaced: it is what makes the
        // lock correct across an unexpected death, where no `Drop` runs at all.
        let _ = sys::close(self.fd);
    }
}

/// Build the record a completed pull writes.
#[allow(clippy::too_many_arguments)]
pub fn record_of(
    reference: &Reference,
    resolved: &Digest,
    resolved_media_type: &str,
    manifest_digest: &Digest,
    manifest: &oci::Manifest,
    config: &oci::Config,
    platform: &str,
) -> Result<Record> {
    Ok(Record {
        repository: reference.canonical_repository(),
        tag: reference.tag.clone(),
        digest: resolved.to_string(),
        digest_media_type: resolved_media_type.to_string(),
        manifest_digest: manifest_digest.to_string(),
        config_digest: manifest.config.parsed_digest()?.to_string(),
        platform: platform.to_string(),
        layers: manifest.layers.iter().map(|l| l.digest.clone()).collect(),
        stored_bytes: manifest.stored_bytes(),
        architecture: config.architecture.clone(),
        os: config.os.clone(),
        created: config.created.clone(),
        pulled_at: clock::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> Store {
        let d = std::env::temp_dir().join(format!("podbox-store-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Store::open(d).unwrap()
    }

    // ⭐ **T-0215's diagnostic capture, and it is OFF unless asked for.**
    //
    // ⛔ **This is the reading that named the mechanism, and it is kept so the
    // figures the entry quotes can be re-taken rather than believed.**
    // `PODBOX_T0215_CAPTURE=1` turns it on;
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
        if !*ON.get_or_init(|| std::env::var_os("PODBOX_T0215_CAPTURE").is_some()) {
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
            rows =
                format!("{HUNT_NL}  kernel: NO FLOCK ROW ON THIS INODE, yet the flock was refused");
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
        // Empty unless PODBOX_T0215_CAPTURE is set.
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
    /// its namespaces, not of podbox, so an assertion on it would fail on a
    /// machine where podbox is correct. `experiments/153-store-lock-race.sh`
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
        let d = std::env::temp_dir().join(format!("podbox-store-{}-openswp", std::process::id()));
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
            manifest_digest:
                "sha256:c64c687cbea9300178b30c95835354e34c4e4febc4badfe27102879de0483b5e".into(),
            pulled_at: clock::now(),
            podbox_version: "0.1.0".into(),
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
        assert_eq!(back.podbox_version, p.podbox_version);
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
        let tarball = std::env::temp_dir().join(format!("podbox-save-{}.tar", std::process::id()));
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
        let tarball =
            std::env::temp_dir().join(format!("podbox-tamper-{}.tar", std::process::id()));
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
    fn a_stored_blob_edited_behind_podboxs_back_is_caught_on_the_way_out() {
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
    /// `clone_fork` sheds registered fds, so nothing podbox forks itself can
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
        // returned it and podbox ran the wrong architecture in silence.
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
    fn an_index_written_by_a_later_podbox_is_refused_rather_than_reinterpreted() {
        let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("podbox-store-{}-future", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join(INDEX_FILE),
            format!("{{\"podbox_store\":{},\"images\":[]}}", INDEX_VERSION + 1),
        )
        .unwrap();
        let e = Store::open(&d).unwrap_err();
        assert!(format!("{e}").contains("newer podbox"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_store_root_prefers_podbox_store_and_never_falls_to_tmpdir_silently() {
        let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        // ⚠ The corpus's shipped default is `env::temp_dir()` with no space
        // check, at
        // `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`.
        // podbox asks the caller instead, and the caller ranks by free space.
        let got = Store::default_root(|| Some("/from-the-probe".into())).unwrap();
        assert!(
            got.starts_with(
                std::env::var_os("PODBOX_STORE")
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
}
