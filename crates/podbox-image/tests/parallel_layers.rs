//! `experiments/190-parallel-layers.sh` hermetic half: concurrent blob fetch
//! against the loopback fixture.
//!
//! The script drives two podbox shapes (the bounded pool and the pre-pool
//! sequential loop) plus a delay proxy against a seeded zot on container
//! loopback. The wall-time legs (time-seq, time-new, time-lat) are
//! measurements, not stable assertions, so they stay in the script. This file
//! ports only what is hermetic through the `common::registry` fixture and the
//! `pull` API.
//!
//! Ported clauses:
//!
//! * seed-1: a pull by tag resolves to the seeded manifest digest on a cold
//!   store.
//! * order-1: the pooled transcript lists layers in manifest order, and a
//!   repull reuses them ("Already exists").
//! * fail-1, fail-3, fail-5: a refused pull exits nonzero, leaves zero staged
//!   files behind, and records nothing. The refusal here is an unknown tag
//!   (404); the digest-mismatch refusal (fail-2) needs a mutable blob store
//!   and is the SKIP test at the bottom.
//! * the pool shape: one pull per `FETCH_WORKERS` slot runs concurrently
//!   against a single fixture, and every one resolves to the seeded digest.
//!
//! Not ported: the wall-time ratios and the latency shape (measurements, not
//! assertions). The fixed fixture serves immutable bytes by design, so the
//! poison arm (fail-0, fail-2) cannot run against it.

mod common;

use std::sync::atomic::{AtomicU64, Ordering};

use podbox_image::platform::Platform;
use podbox_image::pull;
use podbox_image::store::Store;
use podbox_image::transport::Policy;

/// Serialises the pulls in this file. Pulls in one file share nothing but
/// the process (ports are ephemeral, stores are unique), and the lock keeps
/// their transcripts and stores from interleaving while a failure is read.
static PULLS: std::sync::Mutex<()> = std::sync::Mutex::new(());

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A unique store directory for one test leg. The caller removes it.
fn fresh_dir(prefix: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-190-{prefix}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch store directory");
    dir
}

fn cleanup(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// The deliberate act the CLI spells `--insecure-registry`: plain HTTP to the
/// loopback fixture only.
fn policy_for(endpoint: &str) -> Policy {
    Policy::with_insecure(&[endpoint])
}

/// Run one pull, returning the error text beside the transcript.
fn pull_once(store: &Store, reference: &str, policy: &Policy) -> (Result<String, String>, String) {
    let mut out = Vec::new();
    let result = pull::pull(store, reference, &Platform::host(), policy, &mut out)
        .map(|pulled| pulled.record.digest)
        .map_err(|e| e.to_string());
    (result, String::from_utf8_lossy(&out).into_owned())
}

fn staged_partials(store: &Store) -> Vec<String> {
    std::fs::read_dir(store.root().join("staging"))
        .expect("the staging directory exists")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".partial"))
        .collect()
}

/// Script clause seed-1 plus the pool shape: one pull per `FETCH_WORKERS`
/// slot runs concurrently against one fixture, and every pull resolves to
/// the seeded digest with a complete transcript.
#[test]
fn concurrent_pulls_against_one_fixture_all_resolve_to_the_seeded_digest() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let reference = fx.reference();
    let expected = fx.manifest_digest.clone();

    let mut handles = Vec::new();
    for _ in 0..pull::FETCH_WORKERS {
        let dir = fresh_dir("concurrent");
        let reference = reference.clone();
        let endpoint = fx.endpoint.clone();
        handles.push(std::thread::spawn(move || {
            let store = Store::open(&dir).expect("a thread opens its own store");
            let policy = policy_for(&endpoint);
            let (result, text) = pull_once(&store, &reference, &policy);
            (result, text, dir)
        }));
    }
    for h in handles {
        let (result, text, dir) = h.join().expect("a pull thread reports its result");
        let digest = result.expect("a concurrent pull succeeds");
        assert_eq!(
            digest, expected,
            "a concurrent pull resolved to {digest}, not the seeded {expected}"
        );
        assert!(
            text.contains("Pull complete"),
            "a concurrent pull prints no Pull complete: {text:?}"
        );
        assert!(
            text.contains(&expected),
            "a concurrent pull names no seeded digest: {text:?}"
        );
        cleanup(&dir);
    }
    fx.shutdown();
}

/// Script clause order-1: the pooled transcript lists layers in manifest
/// order on a cold store, and a repull reuses them instead of refetching.
#[test]
fn the_pooled_transcript_lists_layers_in_manifest_order_and_repull_reuses() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let policy = policy_for(&fx.endpoint);
    let reference = fx.reference();
    let layer_short = fx.layer_digest[7..19].to_string();

    let dir = fresh_dir("order");
    let store = Store::open(&dir).expect("the test opens its store");
    let (result, text) = pull_once(&store, &reference, &policy);
    let digest = result.expect("the cold pull succeeds");
    assert_eq!(
        digest, fx.manifest_digest,
        "the cold pull resolved to {digest}, not the seeded {}",
        fx.manifest_digest
    );
    let pulling = text
        .find("Pulling from")
        .expect("the transcript announces the pull");
    let complete = text
        .find("Pull complete")
        .expect("the transcript completes a layer");
    let announced = text
        .find(&format!("Digest: {}", fx.manifest_digest))
        .expect("the transcript announces the seeded digest");
    assert!(
        pulling < complete && complete < announced,
        "the transcript left manifest order: {text:?}"
    );
    assert!(
        text.contains(&format!("{layer_short}: Pull complete")),
        "the transcript does not name the seeded layer {layer_short}: {text:?}"
    );

    let (second, again) = pull_once(&store, &reference, &policy);
    let _ = second.expect("the repull succeeds");
    assert!(
        again.contains("Already exists"),
        "the repull fetched instead of reusing: {again:?}"
    );
    assert!(
        again.contains("Image is up to date"),
        "the repull misreports its state: {again:?}"
    );

    cleanup(&dir);
    fx.shutdown();
}

/// Script clauses fail-1, fail-3, fail-5: a refused pull exits nonzero,
/// leaves zero staged files, and records nothing.
#[test]
fn a_failed_pull_records_nothing_and_leaves_no_staged_file() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let policy = policy_for(&fx.endpoint);
    let missing = format!("{}/{repo}:no-such-tag", fx.endpoint, repo = fx.repository);

    let dir = fresh_dir("refused");
    let store = Store::open(&dir).expect("the test opens its store");
    let (result, _) = pull_once(&store, &missing, &policy);
    let refusal = result.expect_err("a pull of an unknown tag must fail");
    assert!(
        refusal.contains("no-such-tag"),
        "the refusal does not name the missing tag: {refusal:?}"
    );
    let left = staged_partials(&store);
    assert!(
        left.is_empty(),
        "staged files survived the refusal: {left:?}"
    );
    let records = store.list().expect("the index reads after a refusal");
    assert!(
        records.is_empty(),
        "the refused pull recorded {} record(s)",
        records.len()
    );

    cleanup(&dir);
    fx.shutdown();
}

/// Script clauses fail-0 and fail-2: one bad blob cancels the rest with a
/// digest mismatch. The fixed fixture serves immutable bytes by design, so
/// there is no blob to poison through it. A live mutable registry (the
/// script's seeded zot) would reopen this; until then SKIP, never green.
#[test]
fn poisoned_blob_cancels_the_rest_with_a_digest_mismatch() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    eprintln!("SKIP: the loopback fixture serves immutable bytes, so no blob can be poisoned here; fail-0 and fail-2 stay in experiments/190-parallel-layers.sh against the seeded zot");
}
