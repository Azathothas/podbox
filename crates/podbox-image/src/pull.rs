//! `podbox pull`: resolve, precheck, fetch, verify, record.
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
use crate::registry::Client;
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
/// (`podbox_probe::sys::FORK_CLOSE_SLOTS`). `FETCH_WORKERS` workers plus
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
    FETCH_WORKERS + FETCH_LOCK_HEADROOM <= podbox_probe::sys::FORK_CLOSE_SLOTS,
    "T-0207: workers plus transients must stay under the fork-shed slots"
);

/// One blob a worker fetches: a layer or the config, in manifest order.
struct FetchJob {
    digest: Digest,
    size: u64,
}

/// What the fetch hands back per job: the worker wrote the bytes, this
/// value says what to do with the staged file.
///
/// ⛔ The worker owns the staged file's lifecycle in every arm, and the fetch
/// only writes: one staging discipline rather than one per fetch shape.
enum FetchStep {
    /// Written and verified; the worker flushes, syncs and commits it.
    Done(StagedFile),
    /// The rest was cancelled while this was staged; the worker drops the
    /// file and removes the staged path without committing.
    Skipped(StagedFile),
}

/// What one slot carries back to the thread that prints.
enum SlotOutcome {
    Done {
        notes: Vec<u8>,
    },
    /// A job no worker started: another one failed first.
    Skipped,
    Failed(Error),
}

/// Per job, in manifest order: a finished worker's buffered lines where one
/// finished, and the first failure in manifest order where one failed.
struct FetchReport {
    notes: Vec<Option<Vec<u8>>>,
    error: Option<Error>,
}

/// Remove one staged file after a failed or cancelled fetch.
///
/// ⛔ Committed blobs are NOT removed here and must not be: they are
/// content-addressed and verified, so keeping them is harmless and a later
/// pull reuses them. Only the unverified staging file goes, so no pull
/// leaves a partial store behind.
fn discard_staged(staged: &std::path::Path) {
    let _ = std::fs::remove_file(staged);
}

/// Fetch one job through the existing store functions: stage, let the fetch
/// write and verify, then flush, sync and commit.
///
/// ⛔ The flush, the sync and the commit live here rather than in each fetch,
/// so there is one write path and not two. A failure removes the staged file
/// here and sets the cancellation, so no worker leaves a `*.partial` behind
/// however the fetches interleave.
fn fetch_one<F>(
    store: &Store,
    job: &FetchJob,
    fetch: &F,
    cancel: &std::sync::atomic::AtomicBool,
) -> SlotOutcome
where
    F: Fn(&FetchJob, StagedFile, &mut Vec<u8>, &std::sync::atomic::AtomicBool) -> Result<FetchStep>
        + Sync,
{
    use std::sync::atomic::Ordering;
    let (staged, file) = match store.stage(job.digest.short()) {
        Ok(x) => x,
        Err(e) => {
            cancel.store(true, Ordering::Release);
            return SlotOutcome::Failed(e);
        }
    };
    // ⛔ Re-checked after staging: another worker may have failed while this
    // one staged, and a blob fetched past the cancellation is a download the
    // pull then throws away.
    if cancel.load(Ordering::Acquire) {
        drop(file);
        discard_staged(&staged);
        return SlotOutcome::Skipped;
    }
    let mut notes = Vec::new();
    match fetch(job, file, &mut notes, cancel) {
        Ok(FetchStep::Done(mut file)) => {
            let result = file
                .flush()
                .map_err(|e| Error::io(staged.display().to_string(), e))
                .and_then(|()| {
                    file.sync_all()
                        .map_err(|e| Error::io(staged.display().to_string(), e))
                })
                .and_then(|()| store.commit(&staged, &job.digest));
            match result {
                Ok(()) => SlotOutcome::Done { notes },
                Err(e) => {
                    drop(file);
                    discard_staged(&staged);
                    cancel.store(true, Ordering::Release);
                    SlotOutcome::Failed(e)
                }
            }
        }
        Ok(FetchStep::Skipped(file)) => {
            drop(file);
            discard_staged(&staged);
            SlotOutcome::Skipped
        }
        Err(e) => {
            discard_staged(&staged);
            cancel.store(true, Ordering::Release);
            SlotOutcome::Failed(e)
        }
    }
}

/// Fetch every job on at most [`FETCH_WORKERS`] threads, `std` only.
///
/// ⛔ Strided, not chunked: consecutive layers of one image are often one
/// size, and contiguous chunks would put every large layer on one worker.
/// ⛔ A failure in one worker cancels the rest: the flag stops any worker
/// starting another job, and every staged file is removed by its own worker.
/// What is already committed stays, content-addressed and harmless.
/// ⛔ This prints nothing. The caller assembles the transcript from `notes`
/// in manifest order, however the fetches interleaved.
fn fetch_parallel<F>(store: &Store, jobs: &[FetchJob], fetch: F) -> FetchReport
where
    F: Fn(&FetchJob, StagedFile, &mut Vec<u8>, &std::sync::atomic::AtomicBool) -> Result<FetchStep>
        + Sync,
{
    use std::sync::atomic::{AtomicBool, Ordering};
    if jobs.is_empty() {
        return FetchReport {
            notes: Vec::new(),
            error: None,
        };
    }
    let cancel = AtomicBool::new(false);
    // ⚠ Shared, not moved: each worker borrows the fetch and the flag through
    // these, so the bounds stay `Sync` rather than growing a `Send` no caller
    // needs. Both are `Copy` as shared references, so the `move` closures
    // below share them instead of taking them.
    let fetch = &fetch;
    let cancel = &cancel;
    // ⛔ A worker that ends without reporting is a loud refusal, never a
    // silent default: a missing blob read as fetched is the corruption
    // `docs/conventions/code.md` forbids.
    let per_worker = match std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(FETCH_WORKERS);
        for w in 0..FETCH_WORKERS {
            handles.push(s.spawn(move || {
                let mut mine = Vec::new();
                let mut i = w;
                while i < jobs.len() {
                    if cancel.load(Ordering::Acquire) {
                        mine.push((i, SlotOutcome::Skipped));
                    } else {
                        mine.push((i, fetch_one(store, &jobs[i], fetch, cancel)));
                    }
                    i += FETCH_WORKERS;
                }
                mine
            }));
        }
        let mut out = Vec::with_capacity(FETCH_WORKERS);
        for h in handles {
            out.push(h.join().map_err(|_| {
                Error::Store(
                    "a fetch worker ended without reporting its jobs, so the \
                     pull is refused rather than reported partial"
                        .into(),
                )
            })?);
        }
        Ok::<_, Error>(out)
    }) {
        Ok(w) => w,
        Err(e) => {
            return FetchReport {
                notes: (0..jobs.len()).map(|_| None).collect(),
                error: Some(e),
            };
        }
    };
    // ⛔ Merged by INDEX, never in completion order: the transcript the
    // caller prints from `notes` stays in manifest order however the fetches
    // interleaved.
    let mut slots: Vec<Option<SlotOutcome>> = (0..jobs.len()).map(|_| None).collect();
    for (i, outcome) in per_worker.into_iter().flatten() {
        slots[i] = Some(outcome);
    }
    let mut notes = Vec::with_capacity(jobs.len());
    let mut error: Option<Error> = None;
    for (i, slot) in slots.into_iter().enumerate() {
        match slot {
            Some(SlotOutcome::Done { notes: n }) => notes.push(Some(n)),
            Some(SlotOutcome::Skipped) => notes.push(None),
            Some(SlotOutcome::Failed(e)) => {
                notes.push(None);
                if error.is_none() {
                    error = Some(e);
                }
            }
            // ⛔ Unreachable by construction: every index is dealt to exactly
            // one worker, and every worker reports every index it was dealt.
            // A silent default here would be a missing blob read as fetched.
            None => {
                notes.push(None);
                if error.is_none() {
                    error = Some(Error::Store(format!(
                        "worker assignment missed blob {i} of {}",
                        jobs.len()
                    )));
                }
            }
        }
    }
    FetchReport { notes, error }
}

impl Pulled {
    /// True when nothing had to be fetched, which is docker's "up to date".
    pub fn up_to_date(&self) -> bool {
        self.fetched.is_empty()
    }
}

/// Run a pull, writing docker's transcript to `out`.
///
/// ⚠ The transcript is the **terminal state of each layer**, printed once it is
/// known. There is no synthetic progress bar:
/// `docs/conventions/forbidden-patterns.md` forbids a hardcoded or synthetic
/// progress display, and a percentage podbox cannot measure is exactly one.
/// ⭐ `platform` is what the caller asked for, resolved by
/// [`Platform::wanted`] before it gets here so the flag, the environment and
/// the host are settled in one place rather than three.
pub fn pull(
    store: &Store,
    want: &str,
    platform: &Platform,
    policy: &Policy,
    out: &mut dyn Write,
) -> Result<Pulled> {
    let reference = Reference::parse(want)?;

    // ⭐ TODO/image.md T-0213. The refusal is HERE and not in `Reference::parse`,
    // because only here is the transport policy in scope, and a refusal that
    // cannot name the flag that would permit it is a refusal a caller cannot
    // act on.
    if reference.plain_http && !policy.permits_explicit_http(reference.endpoint()) {
        return Err(Error::PlainHttpRefused(format!(
            "{want:?} names http://, and {0} is not configured as an insecure \
             registry. podbox does not downgrade a connection on its own: \
             tcp/80 is black-holed on the runtimes podbox targets, so an \
             automatic fallback hangs rather than failing (T-0201). Permit it \
             deliberately with `--insecure-registry {0}`, with \
             $PODBOX_INSECURE_REGISTRIES, or with a line in the registries \
             config file",
            reference.endpoint()
        )));
    }
    // ⛔ Announced once, before anything is fetched. An agent cannot notice a
    // downgraded transport the way a person might.
    if let Some(said) = policy.disclosure(reference.endpoint()) {
        let _ = writeln!(std::io::stderr(), "{said}");
    }
    let probe = crate::probe_cache::resolve(store);

    let mut client = Client::with_policy(policy.clone());
    let endpoint = reference.endpoint().to_string();
    let repository = reference.repository.clone();

    let _ = writeln!(
        out,
        "{}: Pulling from {repository}",
        reference
            .tag
            .clone()
            .unwrap_or_else(|| reference.manifest_selector())
    );

    // ---------------------------------------------------------- the manifest
    let top = client.manifest(&endpoint, &repository, &reference.manifest_selector())?;
    let parsed = Fetched::parse(&top.bytes, Some(&top.media_type))?;

    // ⭐ `resolved` is what the REFERENCE resolved to, and it is the value
    // `docker image inspect` reports in RepoDigests. For a multi-platform tag
    // that is the index digest, not the per-platform manifest digest, and
    // recording the wrong one is exactly the parity M1 is accepted on.
    let resolved = top.digest.clone();
    let (manifest_digest, manifest_bytes, manifest) = match parsed {
        Fetched::Manifest(m) => (resolved.clone(), top.bytes.clone(), m),
        Fetched::Index(index) => {
            let picked = oci::select_platform(&index, platform)?;
            let d = picked.parsed_digest()?;
            let inner = client.manifest(&endpoint, &repository, &d.to_string())?;
            match Fetched::parse(&inner.bytes, Some(&inner.media_type))? {
                Fetched::Manifest(m) => (inner.digest, inner.bytes, m),
                Fetched::Index(_) => {
                    return Err(Error::Oci(format!(
                        "{endpoint}/{repository} served an index where the \
                         {platform} manifest should be. podbox does not follow \
                         an index into another index: an image is one level deep"
                    )))
                }
            }
        }
    };

    // ------------------------------------------------------------- the space
    //
    // ⛔ Before a single layer is fetched. Streaming until ENOSPC leaves a
    // partial store to clean up on a filesystem that is already full, which is
    // the state in which cleanup is least likely to work (T-0203).
    let need_bytes: u64 = manifest
        .layers
        .iter()
        .chain(std::iter::once(&manifest.config))
        .filter(|d| {
            d.parsed_digest()
                .map(|x| !store.has_blob(&x))
                .unwrap_or(true)
        })
        .map(|d| d.size)
        .sum::<u64>()
        + top.bytes.len() as u64
        + manifest_bytes.len() as u64;
    // One inode per blob, plus the index, the lock, the cache and the staging
    // file that exists while each blob is in flight.
    let need_inodes = manifest.layers.len() as u64 + 8;
    space::require(
        &store.root().to_string_lossy(),
        &reference.to_string(),
        need_bytes,
        need_inodes,
        STORE_HEADROOM_BYTES,
    )?;

    // ------------------------------------------------------------- the blobs

    for (bytes, d, what) in [
        (&top.bytes, &resolved, "manifest"),
        (&manifest_bytes, &manifest_digest, "manifest"),
    ] {
        if store.has_blob(d) {
            continue;
        }
        // ⛔ The bytes as served. Re-serialising the parsed document produces a
        // different digest and breaks the parity this milestone is accepted on.
        store.put_bytes(bytes, d, what)?;
    }

    // ⚠ The config is fetched with the layers and is NOT announced with them.
    // Driving a real pull on 2026-09-08 showed the config's short digest
    // printed as a third `Pull complete` beside two layers, which reads as an
    // image with three layers. docker's transcript names layers only, and a
    // transcript that names something else is a display that lies.
    let announced = manifest.layers.len();
    let mut present = Vec::new();
    let mut jobs = Vec::new();
    let mut reused = Vec::new();
    for descriptor in manifest
        .layers
        .iter()
        .chain(std::iter::once(&manifest.config))
    {
        let d = descriptor.parsed_digest()?;
        if store.has_blob(&d) {
            present.push(true);
            reused.push(d.to_string());
        } else {
            present.push(false);
            jobs.push(FetchJob {
                digest: d,
                size: descriptor.size,
            });
        }
    }

    // ⭐ TODO/image.md T-0207. One worker per thread, each with its own
    // client: `Client::blob` takes `&mut self`, so a client is not shareable
    // across threads, and each worker builds one from the same policy.
    // ⛔ No `BufWriter` around the sink. T-0214 restarts it on a body that was
    // cut short, and a `BufWriter` has no way to discard what it is holding;
    // the fetch already writes one 128 KiB chunk at a time, so the buffer was
    // adding a copy rather than a saving.
    let report = fetch_parallel(store, &jobs, |job, file, buf, cancel| {
        // ⛔ The flag is read here as well as before staging: another worker
        // may have failed while this one staged, and a blob fetched past the
        // cancellation is a download the pull then throws away.
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(FetchStep::Skipped(file));
        }
        let mut client = Client::with_policy(policy.clone());
        let back = client.blob(
            &endpoint,
            &repository,
            &job.digest,
            Some(job.size),
            file,
            buf,
        )?;
        Ok(FetchStep::Done(back))
    });

    // ⛔ The transcript is assembled HERE, in manifest order, however the
    // fetches interleaved: every worker buffered its lines and this loop
    // prints them by index. A progress display whose order depends on
    // scheduling is a display that reports the machine's mood.
    // ⚠ The completed lines print even where a later blob failed: the
    // sequential loop this replaces printed what it had before returning the
    // error, and the failure-injection leg of
    // `experiments/190-parallel-layers.sh` asserts the
    // order of what is there.
    let mut fetched = Vec::new();
    let mut notes = report.notes.iter();
    for (n, descriptor) in manifest
        .layers
        .iter()
        .chain(std::iter::once(&manifest.config))
        .enumerate()
    {
        let is_layer = n < announced;
        let d = descriptor.parsed_digest()?;
        if present[n] {
            if is_layer {
                let _ = writeln!(out, "{}: Already exists", d.short());
            }
            continue;
        }
        let lines = notes.next().expect("one note per missing blob");
        if let Some(lines) = lines {
            let _ = out.write_all(lines);
        }
        if is_layer {
            let _ = writeln!(out, "{}: Pull complete", d.short());
        }
        fetched.push(d.to_string());
    }
    if let Some(e) = report.error {
        return Err(e);
    }

    // ------------------------------------------------------------ the record
    let config_digest = manifest.config.parsed_digest()?;
    let config: oci::Config = serde_json::from_slice(&store.read_blob(&config_digest)?)
        .map_err(|e| Error::Oci(format!("the image config does not parse: {e}")))?;
    if !config.architecture.is_empty() && !config.os.is_empty() {
        // ⛔ The variant is checked against what was asked for, not assumed
        // from the index entry. `docs/conventions/forbidden-patterns.md`: a
        // cache holding a variant it was not keyed by serves it to the next
        // unqualified fetch, and `Exec format error` is how that surfaces.
        // ⛔ Checked against what was ASKED FOR, not against the host. A
        // deliberate `--platform linux/arm64` on an amd64 machine must not be
        // refused here; a registry serving amd64 bytes under an arm64
        // descriptor must be.
        if !platform.matches(&config.os, &config.architecture, None) {
            return Err(Error::Oci(format!(
                "the config of {reference} declares {}/{}, and podbox asked for \
                 {platform}. The store records the platform of what it holds, \
                 so a mismatch here is refused rather than recorded",
                config.os, config.architecture,
            )));
        }
    }

    let record = store::record_of(
        &reference,
        &resolved,
        &top.media_type,
        &manifest_digest,
        &manifest,
        &config,
        &platform.to_string(),
    )?;
    store.put_record(record.clone())?;
    // ⭐ TODO/image.md T-1321. How the bytes got here, recorded at pull
    // time rather than derived later: the sidecar never lives inside
    // image metadata.
    store.note_pull(&crate::health::Provenance {
        registry: reference.endpoint().to_string(),
        repository: reference.canonical_repository(),
        tag: reference.tag.clone(),
        manifest_digest: manifest_digest.to_string(),
        pulled_at: record.pulled_at.clone(),
        podbox_version: env!("CARGO_PKG_VERSION").to_string(),
    })?;

    let _ = writeln!(out, "Digest: {resolved}");
    let _ = writeln!(
        out,
        "Status: {} for {reference}",
        if fetched.is_empty() {
            "Image is up to date"
        } else {
            "Downloaded newer image"
        }
    );
    Ok(Pulled {
        record,
        reused,
        fetched,
        probe_source: probe.source,
    })
}

/// Pull every offered tag of a repository: `pull -a` (TODO/cli.md T-1331).
///
/// The offer is the registry's tags listing; each tag pulls through
/// `pull`, so per-tag verification, provenance and output are the same
/// as a single pull. A tag with no manifest for this platform is named
/// and skipped: stopping the run on it would make `pull -a` unusable
/// against any repository that also serves another OS. Any other failing
/// tag stops the run naming the tag: a partial set with a green exit
/// would read as everything fetched. An offer with nothing for this
/// platform at all is an error, not an empty success.
pub fn pull_all(
    store: &Store,
    endpoint: &str,
    repository: &str,
    platform: &Platform,
    policy: &Policy,
    out: &mut dyn Write,
) -> Result<Vec<Pulled>> {
    let mut client = Client::with_policy(policy.clone());
    let mut tags = client.tags(endpoint, repository)?;
    tags.sort();
    if tags.is_empty() {
        return Err(Error::Store(format!(
            "{endpoint}/{repository} offers no tags to pull"
        )));
    }
    let _ = writeln!(out, "Pulling {} tag(s) from {repository}", tags.len());
    let mut pulled = Vec::with_capacity(tags.len());
    let mut skipped = 0usize;
    for tag in &tags {
        let want = format!("{endpoint}/{repository}:{tag}");
        match pull(store, &want, platform, policy, out) {
            Ok(p) => pulled.push(p),
            Err(Error::NoPlatform { want, offered }) => {
                let _ = writeln!(
                    out,
                    "tag {tag}: skipped, this index offers no {want} manifest. It offers: {offered}"
                );
                skipped += 1;
            }
            Err(e) => return Err(Error::Store(format!("pull -a stopped at tag {tag}: {e}"))),
        }
    }
    if pulled.is_empty() {
        return Err(Error::Store(format!(
            "{endpoint}/{repository} offers no {platform} manifest under any of its {} tag(s) ({skipped} skipped)",
            tags.len()
        )));
    }
    Ok(pulled)
}

/// The digest a store already holds for a reference, for the "already present"
/// path that never touches the network.
pub fn local(store: &Store, want: &str) -> Result<Option<Digest>> {
    match store.find(want)?.first() {
        Some(r) => Ok(Some(Digest::parse(&r.digest)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn par_store(name: &str) -> Store {
        let d = std::env::temp_dir().join(format!("podbox-pull-par-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Store::open(d).unwrap()
    }

    fn par_jobs(n: usize) -> (Vec<String>, Vec<FetchJob>) {
        let mut contents = Vec::new();
        let mut jobs = Vec::new();
        for i in 0..n {
            let content = format!("t0207-blob-{i}");
            contents.push(content.clone());
            jobs.push(FetchJob {
                digest: Digest::of(content.as_bytes()),
                size: content.len() as u64,
            });
        }
        (contents, jobs)
    }

    /// ⛔ TODO/image.md T-0207. The merge is by INDEX, never in completion
    /// order. The forced interleave is job `i` sleeping `(n - i) * 25` ms, so
    /// the last job in manifest order finishes first; a merge in completion
    /// order would carry job 5's lines at index 0 and this would go red.
    #[test]
    fn the_transcript_stays_in_manifest_order_however_fetches_interleave() {
        let store = par_store("order");
        let n = 6;
        let (contents, jobs) = par_jobs(n);
        let report = fetch_parallel(&store, &jobs, |job, mut file, buf, _cancel| {
            use std::io::Write;
            let idx = jobs
                .iter()
                .position(|j| j.digest == job.digest)
                .expect("a job this call was not given");
            std::thread::sleep(std::time::Duration::from_millis(((n - idx) * 25) as u64));
            // ⚠ The bytes match the digest, so the commit below stores a
            // consistent blob rather than an unverified one.
            file.write_all(contents[idx].as_bytes())
                .map_err(|e| Error::io("t0207 order probe", e))?;
            buf.write_all(format!("job {idx}").as_bytes())
                .map_err(|e| Error::io("t0207 order probe", e))?;
            Ok(FetchStep::Done(file))
        });
        assert!(
            report.error.is_none(),
            "no job fails here: {:?}",
            report.error.map(|e| e.to_string())
        );
        assert_eq!(report.notes.len(), n);
        for (i, note) in report.notes.iter().enumerate() {
            assert_eq!(
                note.as_deref(),
                Some(format!("job {i}").as_bytes()),
                "slot {i} carries another job's lines"
            );
        }
        for job in &jobs {
            assert!(store.has_blob(&job.digest));
        }
        let _ = std::fs::remove_dir_all(store.root());
    }

    /// ⛔ TODO/image.md T-0207. One worker's failure cancels the rest and no
    /// staged file survives: the poisoned job fails with partial bytes on
    /// disk, the in-flight jobs observe the cancellation rather than running
    /// out their wait, the job never started stays skipped, and the staging
    /// directory holds zero `*.partial` afterwards.
    #[test]
    fn a_failure_cancels_the_rest_and_leaves_no_staged_file() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        const POISON: usize = 1;
        let store = par_store("cancel");
        let n = 6;
        let (_, jobs) = par_jobs(n);
        let jobs_ref = &jobs;
        let observed = AtomicUsize::new(0);
        let report = fetch_parallel(&store, &jobs, |job, mut file, _buf, cancel| {
            use std::io::Write;
            let idx = jobs_ref
                .iter()
                .position(|j| j.digest == job.digest)
                .expect("a job this call was not given");
            if idx == POISON {
                // ⛔ Let the others stage first, so the cancellation has
                // something in flight to reach, then fail with partial
                // bytes the worker must remove.
                std::thread::sleep(std::time::Duration::from_millis(100));
                file.write_all(b"partial")
                    .map_err(|e| Error::io("t0207 poison probe", e))?;
                drop(file);
                return Err(Error::DigestMismatch {
                    what: "t0207 poison probe".to_string(),
                    want: job.digest.to_string(),
                    got: "sha256:dead".to_string(),
                });
            }
            // In flight: wait for the cancellation, bounded, then yield
            // without committing.
            let start = std::time::Instant::now();
            let bound = std::time::Duration::from_secs(30);
            while start.elapsed() < bound {
                if cancel.load(Ordering::Acquire) {
                    observed.fetch_add(1, Ordering::Release);
                    return Ok(FetchStep::Skipped(file));
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            drop(file);
            Err(Error::Store(
                "t0207 cancel probe ran out its wait without seeing the cancellation".into(),
            ))
        });
        // The poison's error, not a cancellation artefact, and every other
        // slot holds nothing.
        let Some(e) = report.error.as_ref() else {
            panic!("the poisoned job did not fail");
        };
        assert!(
            e.to_string().contains(&jobs[POISON].digest.to_string()),
            "the reported failure is not the poisoned blob: {e}"
        );
        assert!(
            report.notes.iter().all(|x| x.is_none()),
            "a job committed beside a failure"
        );
        // Three in flight (0, 2, 3) observed the flag; jobs 4 and 5 never
        // started, because their workers were busy on job 0 and the poison
        // when it failed.
        assert_eq!(
            observed.load(Ordering::Acquire),
            3,
            "an in-flight worker missed the cancellation"
        );
        // ⛔ Zero staged files left, and nothing committed either.
        let staging = store.root().join("staging");
        let left: Vec<String> = std::fs::read_dir(&staging)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|x| x.ends_with(".partial"))
            .collect();
        assert!(
            left.is_empty(),
            "staged files survived the failure: {left:?}"
        );
        for job in &jobs {
            assert!(
                !store.has_blob(&job.digest),
                "a blob was committed beside a failure"
            );
        }
        let _ = std::fs::remove_dir_all(store.root());
    }

    /// ⛔ The refusal T-0201 won, kept, and now able to say what would permit
    /// it. A message a caller cannot act on is a message that costs a session.
    #[test]
    fn an_http_reference_is_refused_and_the_refusal_names_the_flag() {
        let store = Store::open(
            std::env::temp_dir().join(format!("podbox-pull-http-{}", std::process::id())),
        )
        .unwrap();
        let mut out = Vec::new();
        let e = pull(
            &store,
            "http://localhost:5000/x:latest",
            &Platform::host(),
            &Policy::default(),
            &mut out,
        );
        let Err(e) = e else {
            panic!("a pull succeeded with nothing listening")
        };
        let text = format!("{e}");
        assert!(
            text.contains("--insecure-registry localhost:5000"),
            "{text}"
        );
        assert!(text.contains("T-0201"), "{text}");
        // ⛔ And nothing was fetched: the refusal is before the network.
        assert!(out.is_empty(), "it printed a transcript before refusing");
        let _ = std::fs::remove_dir_all(store.root());
    }

    /// ⭐ The same reference, with the registry named insecure, gets past the
    /// policy. ⚠ It then fails to CONNECT, because nothing is listening on
    /// localhost:5000 in a test, and that is the right place to stop: this
    /// asserts the policy decision, and `experiments/280-insecure-registry.sh`
    /// drives the whole path against a registry that is really there.
    #[test]
    fn naming_the_registry_insecure_gets_past_the_policy() {
        let store = Store::open(
            std::env::temp_dir().join(format!("podbox-pull-ok-{}", std::process::id())),
        )
        .unwrap();
        let mut out = Vec::new();
        let e = pull(
            &store,
            "http://localhost:5000/x:latest",
            &Platform::host(),
            &Policy::with_insecure(&["localhost:5000"]),
            &mut out,
        );
        let Err(e) = e else {
            panic!("a pull succeeded with nothing listening")
        };
        let text = format!("{e}");
        assert!(
            !text.contains("--insecure-registry"),
            "the policy still refused it: {text}"
        );
        let _ = std::fs::remove_dir_all(store.root());
    }
}
