//! `experiments/150-image-acquisition.sh` clauses 1 and 2 as an integration
//! test: pull the T-1531 fixture end to end through the library, then pull
//! the same reference a second time.
//!
//! Clause 1 is the first pull. It succeeds, its transcript is non-empty, and
//! its fetched list holds the config digest and the layer digest. Clause 2
//! is the second pull against the same store. It succeeds, fetches nothing,
//! and reports up to date. The server is the fixture in
//! `tests/common/registry.rs` and the client is `podbox_image::pull::pull`.

mod common;

use podbox_image::{platform::Platform, pull::pull, store::Store, transport::Policy};

/// Every pull in this file runs under one lock. A pull holds fork-shed lock
/// slots and those slots are process-global, so two pulls running together in
/// one process can refuse each other for a reason outside the code under test.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fresh_store(prefix: &str) -> Store {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("podbox-image-{prefix}-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(&dir).expect("open a fresh temp store")
}

#[test]
fn pull_the_fixture_then_pull_it_again_for_up_to_date() {
    let _serialised = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let store = fresh_store("acquisition");
    let want = format!("http://{}", fx.reference());
    let policy = Policy::with_insecure(&[fx.endpoint.as_str()]);
    let platform = Platform::host();

    let mut out = Vec::new();
    let pulled = pull(&store, &want, &platform, &policy, &mut out)
        .expect("pull the T-1531 fixture end to end");
    assert!(
        !out.is_empty(),
        "the first pull succeeded but printed no transcript"
    );
    assert_eq!(
        pulled.fetched.len(),
        2,
        "the first pull fetched {:?}, want the config and the one layer",
        pulled.fetched
    );
    assert_eq!(
        pulled.record.digest, fx.manifest_digest,
        "the record names a manifest the fixture did not serve"
    );
    // ⭐ A fetched list of digest strings is a claim, not a measurement: this
    // is what the pull claims it fetched, checked against the bytes the
    // fixture published for each name. The loopback fixture is immutable, so
    // a body truncated in flight, or one crossed with another fetch, would
    // otherwise pass as fetched here.
    for (want, published) in [
        (&fx.config_digest, &fx.config),
        (&fx.layer_digest, &fx.layer),
    ] {
        assert!(
            pulled.fetched.iter().any(|d| d == want),
            "fetched {:?} holds no digest {want}",
            pulled.fetched
        );
        let path = store
            .blob_path(&podbox_image::digest::Digest::parse(want).expect("a parseable digest"));
        let stored = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("no blob for {want} at {}: {e}", path.display()));
        assert_eq!(
            stored.len(),
            published.len(),
            "the fetch stored {} byte(s) for {want}, the registry published {}",
            stored.len(),
            published.len()
        );
        assert!(
            stored == *published,
            "the fetch stored bytes that are not the published ones for \
             {want}, differing at {:?}",
            stored.iter().zip(published).position(|(a, b)| a != b)
        );
    }
    // The manifest GET is one fetch too, and its record names the bytes the
    // reference resolved to.
    assert_eq!(
        pulled.record.manifest_digest, fx.manifest_digest,
        "the record manifest digest names a manifest the fixture did not serve"
    );
    let manifest_path = store.blob_path(
        &podbox_image::digest::Digest::parse(&fx.manifest_digest).expect("a parseable digest"),
    );
    let stored_manifest = std::fs::read(&manifest_path)
        .unwrap_or_else(|e| panic!("no manifest blob at {}: {e}", manifest_path.display()));
    assert_eq!(
        stored_manifest.len(),
        fx.manifest.len(),
        "the pull stored {} manifest byte(s), the registry published {}",
        stored_manifest.len(),
        fx.manifest.len()
    );
    assert!(
        stored_manifest == fx.manifest,
        "the stored manifest is not the one served, differing at {:?}",
        stored_manifest
            .iter()
            .zip(&fx.manifest)
            .position(|(a, b)| a != b)
    );

    let mut again = Vec::new();
    let second = pull(&store, &want, &platform, &policy, &mut again)
        .expect("pull the same reference a second time");
    assert!(
        second.up_to_date(),
        "the second pull fetched {:?}, want nothing",
        second.fetched
    );
    assert!(!again.is_empty(), "the second pull printed no transcript");
    let text = String::from_utf8_lossy(&again);
    assert!(
        text.contains("up to date"),
        "the second pull names no up-to-date status: {text}"
    );

    let root = store.root().to_path_buf();
    let _ = std::fs::remove_dir_all(&root);
    fx.shutdown();
}
