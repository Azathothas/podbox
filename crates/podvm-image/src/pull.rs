//! `podvm pull`: resolve, precheck, fetch, verify, record.
//!
//! The order is the whole design and every step of it is an entry:
//!
//! 1. the reference is normalised, and `http://` is refused ([`crate::reference`], T-0201);
//! 2. the probe answer is taken from `$store/probe.json` where its key still
//!    holds ([`crate::probe_cache`], T-0111), because `pull` is one of the hot
//!    paths `TOOL.md` section 6.1 asks not to re-probe;
//! 3. the manifest is fetched and its digest **computed over the served bytes**
//!    (T-0202);
//! 4. blocks **and inodes** are checked at the destination before a single
//!    layer is fetched, and the refusal names the destination, the free amount,
//!    the required amount and the unit ([`crate::space`], T-0203);
//! 5. every blob is verified as it is written and is renamed into `blobs/` only
//!    on success (T-0202).
//!
//! ⛔ No extraction. `TODO/milestones.md` T-1102: M1 acquires, M2 extracts.

use std::io::Write;

use crate::digest::Digest;
use crate::error::{Error, Result};
use crate::oci::{self, Fetched};
use crate::platform::Platform;
use crate::reference::Reference;
use crate::space;
use crate::store::{self, Record, StagedFile, Store};
use crate::transport::Policy;

/// What the store needs beyond the payload: the index, a staging copy of the
/// largest blob and the lock files.
///
/// ⚠ It is **not** an estimate of the extracted size. M2 checks that against
/// the layer contents it is about to write, and inventing a multiplier here
/// would be a fabricated number (`AGENTS.md` absolute 3).
pub const STORE_HEADROOM_BYTES: u64 = 16 * 1024 * 1024;

pub struct Pulled {
    pub record: Record,
    /// Blobs that were already in the store, by digest.
    pub reused: Vec<String>,
    pub fetched: Vec<String>,
    pub probe_source: crate::probe_cache::Source,
}

/// ⭐ `TODO/image.md` T-0207. How many blobs one pull fetches at once.
///
/// Fixed rather than scaled to the machine's cores: the constraint is the
/// registry's willingness to serve, not this machine's cores, and a
/// CPU-scaled bound on a many-core builder is how a client earns a rate
/// limit. Threads over an async runtime: `TODO/deps.md` T-0906 ruled a
/// blocking client on a measured size delta, and an async runtime for a
/// handful of downloads would reopen a decision that was closed against a
/// number.
///
/// ⛔ The arithmetic this has to satisfy: every worker holds exactly one
/// staging [`Lock`](crate::store::Store::stage) while it writes, and every
/// lock registers a fork-shed slot
/// (`podvm_probe::sys::FORK_CLOSE_SLOTS`). `FETCH_WORKERS` workers plus
/// [`FETCH_LOCK_HEADROOM`] transients stay under that ceiling, and a
/// seventeenth lock is refused by name rather than held unsafely. The `const`
/// assert below pins it at compile time.
pub const FETCH_WORKERS: usize = 4;
/// Transients any thread may hold while a fetch runs: the index lock at
/// record time, a sweep on another thread, and one spare. See
/// [`FETCH_WORKERS`].
pub const FETCH_LOCK_HEADROOM: usize = 3;
// ⛔ Pinned at compile time rather than in a test: raising `FETCH_WORKERS`
// past the headroom refuses the build instead of leaking lock fds into
// forked children in production. A `const` assert fires before any test
// runs, which is what makes it stronger than the runtime test it replaces.
const _: () = assert!(
    FETCH_WORKERS + FETCH_LOCK_HEADROOM <= podvm_probe::sys::FORK_CLOSE_SLOTS,
    "T-0207: workers plus transients must stay under the fork-shed slots"
);

// ⭐ The fetch pool is `pod_oci::pull` (PodUtils/podcrates): podvm's
// names for it, and the store as its `Blobs`.
use pod_oci::pull::{over_ceiling as blob_over_ceiling, Job as FetchJob, Step as FetchStep};

/// Fetch every job on at most [`FETCH_WORKERS`] threads
/// (`pod_oci::pull::fetch_parallel`).
///
/// ⛔ A failure in one worker cancels the rest, and every staged file is
/// removed by its own worker. What is already committed stays,
/// content-addressed and harmless. ⛔ This prints nothing: the caller
/// assembles the transcript from `notes` in manifest order.
fn fetch_parallel<F>(store: &Store, jobs: &[FetchJob], fetch: F) -> pod_oci::pull::Report<Error>
where
    F: Fn(
            &FetchJob,
            StagedFile,
            &mut Vec<u8>,
            &std::sync::atomic::AtomicBool,
        ) -> Result<FetchStep<StagedFile>>
        + Sync,
{
    pod_oci::pull::fetch_parallel(store, jobs, FETCH_WORKERS, fetch)
}

/// The store is the pool's blob store: a staging file per call (invariant
/// I6), committed by a rename into `blobs/` after the flush and the sync.
///
/// ⛔ The flush, the sync and the commit live here rather than in each fetch,
/// so there is one write path and not two.
impl pod_oci::pull::Blobs for Store {
    type Staged = StagedFile;
    type Error = Error;

    fn stage(&self, digest: &Digest) -> Result<(std::path::PathBuf, StagedFile)> {
        Store::stage(self, digest.short())
    }

    fn commit(&self, path: &std::path::Path, mut file: StagedFile, digest: &Digest) -> Result<()> {
        file.flush()
            .map_err(|e| Error::io(path.display().to_string(), e))?;
        file.sync_all()
            .map_err(|e| Error::io(path.display().to_string(), e))?;
        Store::commit(self, path, digest)
    }
}

impl Pulled {
    /// True when nothing had to be fetched, which is docker's "up to date".
    pub fn up_to_date(&self) -> bool {
        self.fetched.is_empty()
    }
}
mod run;
pub use run::*;

/// The digest a store already holds for a reference, for the "already present"
/// path that never touches the network.
pub fn local(store: &Store, want: &str) -> Result<Option<Digest>> {
    match store.find(want)?.first() {
        Some(r) => Ok(Some(Digest::parse(&r.digest)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests;
