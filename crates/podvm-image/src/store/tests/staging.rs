use super::*;

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
    let d = std::env::temp_dir().join(format!("podvm-store-{}-openswp", std::process::id()));
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
        manifest_digest: "sha256:c64c687cbea9300178b30c95835354e34c4e4febc4badfe27102879de0483b5e"
            .into(),
        pulled_at: clock::now(),
        podvm_version: "0.1.0".into(),
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
    assert_eq!(back.podvm_version, p.podvm_version);
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
    let tarball = std::env::temp_dir().join(format!("podvm-save-{}.tar", std::process::id()));
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
    let tarball = std::env::temp_dir().join(format!("podvm-tamper-{}.tar", std::process::id()));
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
fn a_stored_blob_edited_behind_podvms_back_is_caught_on_the_way_out() {
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
