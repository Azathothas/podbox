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

use podvm_probe::sys;
use serde::{Deserialize, Serialize};

use crate::clock;
use crate::contain;
use crate::digest::Digest;
use crate::error::{Error, Result};
use crate::oci;
use crate::reference::Reference;

/// ⭐ A version discriminator on everything persisted.
/// `docs/conventions/code.md`: old data still reads and new code knows which
/// version it is looking at. A store written by a later podvm is refused by
/// name rather than parsed as though its fields meant what they mean here.
pub const INDEX_VERSION: u32 = 1;
const INDEX_FILE: &str = "store.json";
const BLOBS: &str = "blobs";
const LOCKS: &str = "locks";
const STAGING: &str = "staging";
const STORE_LOCK: &str = "lock";

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
    /// the value `podvm images --format '{{.Digest}}'` prints.
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
    pub podvm_store: u32,
    #[serde(default)]
    pub images: Vec<Record>,
}

impl Default for Index {
    fn default() -> Index {
        Index {
            podvm_store: INDEX_VERSION,
            images: Vec::new(),
        }
    }
}

/// What a platform-qualified lookup found.
///
/// ⛔ Two states rather than an empty vector, because they need different
/// sentences: "podvm does not hold this image" and "podvm holds this image,
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
    /// ⚠ `$PODVM_STORE` first, then the XDG data directory, then `$HOME`.
    /// Deliberately **not** `$TMPDIR`: on the runtime podvm targets `/tmp` is
    /// 64 MiB, which almost no image fits in, and `env::temp_dir()` with no
    /// space check is the shipped default this project read in the corpus at
    /// `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`.
    /// Where none of the three is set, `fallback` is consulted; the caller
    /// supplies the probe's write allowlist, ranked by free space (T-0203).
    pub fn default_root(fallback: impl FnOnce() -> Option<String>) -> Result<PathBuf> {
        if let Some(p) = std::env::var_os("PODVM_STORE") {
            let p = PathBuf::from(p);
            if !p.as_os_str().is_empty() {
                return Ok(p);
            }
        }
        for (var, suffix) in [("XDG_DATA_HOME", "podvm"), ("HOME", ".local/share/podvm")] {
            if let Some(base) = std::env::var_os(var) {
                let base = PathBuf::from(base);
                if base.is_absolute() {
                    return Ok(base.join(suffix));
                }
            }
        }
        match fallback() {
            Some(p) => Ok(PathBuf::from(p).join("podvm")),
            None => Err(Error::Store(
                "no store directory: $PODVM_STORE, $XDG_DATA_HOME and $HOME are \
                 all unset or relative, and no probed writable path could hold \
                 one. Set $PODVM_STORE to a directory podvm may write"
                    .into(),
            )),
        }
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Store> {
        let root = root.into();
        create_private_root(&root)?;
        for d in [
            root.join(BLOBS).join(crate::digest::SHA256),
            root.join(LOCKS),
            root.join(STAGING),
        ] {
            std::fs::create_dir_all(&d).map_err(|e| Error::io(d.display().to_string(), e))?;
        }
        let store = Store { root };
        // ⛔ Refuse a store a later podvm wrote, rather than reading its fields
        // as though they meant what they mean here.
        let index = store.read_index()?;
        if index.podvm_store > INDEX_VERSION {
            return Err(Error::Store(format!(
                "{} declares store version {} and this podvm writes {INDEX_VERSION}. \
                 A newer podvm made it; this one will not edit it",
                store.index_path().display(),
                index.podvm_store
            )));
        }
        // ⛔ Invariant I5. Every command opens the store, so this is where an
        // abandoned staging file is reclaimed. `docs/conventions/code.md` calls
        // it the sweep that heals the drift the happy path let slip.
        store.sweep_staging();
        Ok(store)
    }

    /// The permission bits of the store root where group or other can
    /// reach it, or `None` where only its owner can (or the root cannot be
    /// read). ⛔ TODO/supervise.md T-1715: the container table under the
    /// root keeps each `-e` value, so an exposed root gives them to each
    /// local user. podvm does not change the mode of a root that it did
    /// not create; the banner and `podvm doctor` name it.
    pub fn exposed_mode(&self) -> Option<u32> {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&self.root).ok()?.permissions().mode() & 0o777;
        (mode & 0o077 != 0).then_some(mode)
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

    /// The bytes one blob file occupies, or 0 where it is absent.
    /// TODO/cli.md T-1337: `system df` never exits non-zero for
    /// accounting alone, so a missing file counts nothing rather than
    /// failing the whole report.
    pub fn blob_bytes(&self, digest: &str) -> u64 {
        let Ok(d) = Digest::parse(digest) else {
            return 0;
        };
        std::fs::metadata(self.blob_path(&d))
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// What `delete(doomed)` would free, measured the same way but
    /// without unlinking anything: reachable blobs over the surviving
    /// index, under the same lock. TODO/cli.md T-1337: this is what
    /// `system df` reports as reclaimable, so the number it prints is
    /// the number `image prune` would free on a quiet store.
    pub fn reclaimable_bytes(&self, doomed: &[Record]) -> Result<u64> {
        let _guard = self.lock()?;
        let index = self.read_index()?;
        let doomed_keys: Vec<(String, Option<String>, String)> = doomed
            .iter()
            .map(|r| (r.repository.clone(), r.tag.clone(), r.digest.clone()))
            .collect();
        let mut keep: Vec<String> = Vec::new();
        for r in &index.images {
            if doomed_keys
                .iter()
                .any(|(repo, tag, dig)| r.repository == *repo && r.tag == *tag && r.digest == *dig)
            {
                continue;
            }
            keep.extend(r.blobs().into_iter().map(str::to_string));
        }
        let mut freed = 0u64;
        let mut counted: Vec<String> = Vec::new();
        for r in doomed {
            for b in r.blobs() {
                if keep.iter().any(|k| k == b) || counted.iter().any(|c| c == b) {
                    continue;
                }
                counted.push(b.to_string());
                freed += self.blob_bytes(b);
            }
        }
        Ok(freed)
    }

    pub fn has_blob(&self, d: &Digest) -> bool {
        self.blob_path(d).is_file()
    }

    pub fn read_index(&self) -> Result<Index> {
        let path = self.index_path();
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                Error::Store(format!(
                    "{} does not parse: {e}. It is podvm's own index; move it \
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
}

mod blobs;
mod files;
mod records;
pub use files::*;

/// Create the store root with mode 0700 where it does not exist yet; its
/// parents get the default modes. ⛔ TODO/supervise.md T-1715: the
/// container table and the logs live under the root, and the table keeps
/// each `-e` value, so only the owner may reach them. A root that exists
/// keeps its mode ([`Store::exposed_mode`] reports it).
fn create_private_root(root: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    if root.is_dir() {
        return Ok(());
    }
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent.display().to_string(), e))?;
    }
    match std::fs::DirBuilder::new().mode(0o700).create(root) {
        Ok(()) => Ok(()),
        // Another podvm made it between the check and the create.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && root.is_dir() => Ok(()),
        Err(e) => Err(Error::io(root.display().to_string(), e)),
    }
}

#[cfg(test)]
mod tests;
