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
    assert!(
        pulled.fetched.iter().any(|d| d == &fx.config_digest),
        "fetched {:?} holds no config digest {}",
        pulled.fetched,
        fx.config_digest
    );
    assert!(
        pulled.fetched.iter().any(|d| d == &fx.layer_digest),
        "fetched {:?} holds no layer digest {}",
        pulled.fetched,
        fx.layer_digest
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
