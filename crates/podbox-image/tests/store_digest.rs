//! `experiments/150-image-acquisition.sh` clauses 1 and 3 as an integration
//! test: the digests a pull records name the bytes the fixture served.
//!
//! After pulling the T-1531 fixture, the record digest equals the served
//! manifest digest and the record config digest equals the served config
//! digest. The config blob and the layer blob exist under the store root at
//! the paths their parsed digests name, and the bytes at each path hash back
//! to the digest that names them. The served manifest bytes hash to the
//! manifest digest as well, which checks the name before checking the disk.

mod common;

use podbox_image::{
    digest::Digest, platform::Platform, pull::pull, store::Store, transport::Policy,
};

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
fn the_record_and_the_blobs_name_the_served_bytes() {
    let _serialised = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let store = fresh_store("store-digest");
    let want = format!("http://{}", fx.reference());
    let policy = Policy::with_insecure(&[fx.endpoint.as_str()]);

    let mut out = Vec::new();
    let pulled = pull(&store, &want, &Platform::host(), &policy, &mut out)
        .expect("pull the T-1531 fixture end to end");
    assert_eq!(
        pulled.record.digest, fx.manifest_digest,
        "the record digest names a manifest the fixture did not serve"
    );
    assert_eq!(
        pulled.record.manifest_digest, fx.manifest_digest,
        "the record manifest digest names a manifest the fixture did not serve"
    );
    assert_eq!(
        pulled.record.config_digest, fx.config_digest,
        "the record config digest names a config the fixture did not serve"
    );

    // ⭐ The manifest the pull resolved to is on disk under its own digest,
    // and the bytes there are the ones the registry served. This is the
    // only place `fx.manifest` is read, and the file's header names the
    // manifest as one of the three documents a pull lands.
    let path = store.root().join(
        Digest::parse(&fx.manifest_digest)
            .expect("the fixture serves parseable digests")
            .blob_path(),
    );
    let served = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("the pull stored no manifest at {}: {e}", path.display()));
    assert_eq!(
        served.len(),
        fx.manifest.len(),
        "the stored manifest is {} byte(s), the registry published {}",
        served.len(),
        fx.manifest.len()
    );
    assert!(
        served == fx.manifest,
        "the stored manifest is not the one served, differing at {:?}",
        served.iter().zip(&fx.manifest).position(|(a, b)| a != b)
    );

    for (want, published) in [
        (&fx.config_digest, &fx.config),
        (&fx.layer_digest, &fx.layer),
    ] {
        let parsed = Digest::parse(want).expect("the fixture serves parseable digests");
        let path = store.root().join(parsed.blob_path());
        assert!(
            path.is_file(),
            "blob {want} has no file at {}",
            path.display()
        );
        let bytes = std::fs::read(&path).expect("read the stored blob");
        assert_eq!(
            Digest::of(&bytes).to_string(),
            *want,
            "the bytes at {} do not hash to {want}",
            path.display()
        );
        // ⭐ And the bytes are the ones the registry published, not merely
        // bytes that hash to the right name. The loopback fixture is
        // immutable, so these are the only bytes {want} can name, and a
        // truncation or a crossed write inside the fetch that landed here
        // would pass the hash above.
        assert_eq!(
            bytes.len(),
            published.len(),
            "the stored {want} is {} byte(s), the registry published {}",
            bytes.len(),
            published.len()
        );
        assert!(
            bytes == *published,
            "the stored {want} is not the published blob, differing at {:?}",
            bytes.iter().zip(published).position(|(a, b)| a != b)
        );
    }

    // ⚠ And the published bytes hash to the name, checked last and with the
    // crate's own digest: this reads the FIXTURE side of the pair, so it is
    // a check on what the registry published rather than on what was fetched,
    // and it cannot stand in for the comparisons above.
    assert_eq!(
        Digest::of(&fx.manifest).to_string(),
        fx.manifest_digest,
        "the served manifest bytes do not hash to the served manifest digest"
    );

    let root = store.root().to_path_buf();
    let _ = std::fs::remove_dir_all(&root);
    fx.shutdown();
}
