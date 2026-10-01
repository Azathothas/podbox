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

    for want in [&fx.config_digest, &fx.layer_digest] {
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
    }

    assert_eq!(
        Digest::of(&fx.manifest).to_string(),
        fx.manifest_digest,
        "the served manifest bytes do not hash to the served manifest digest"
    );

    let root = store.root().to_path_buf();
    let _ = std::fs::remove_dir_all(&root);
    fx.shutdown();
}
