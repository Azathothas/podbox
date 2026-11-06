use super::*;

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
fn an_index_written_by_a_later_podvm_is_refused_rather_than_reinterpreted() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("podvm-store-{}-future", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join(INDEX_FILE),
        format!("{{\"podvm_store\":{},\"images\":[]}}", INDEX_VERSION + 1),
    )
    .unwrap();
    let e = Store::open(&d).unwrap_err();
    assert!(format!("{e}").contains("newer podvm"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_store_root_prefers_podvm_store_and_never_falls_to_tmpdir_silently() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    // ⚠ The corpus's shipped default is `env::temp_dir()` with no space
    // check, at
    // `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`.
    // podvm asks the caller instead, and the caller ranks by free space.
    let got = Store::default_root(|| Some("/from-the-probe".into())).unwrap();
    assert!(
        got.starts_with(
            std::env::var_os("PODVM_STORE")
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

/// T-1337: reclaimable sizing matches `delete` without unlinking: a
/// shared blob counts once, and only where no survivor needs it.
#[test]
fn reclaimable_counts_unshared_blobs_once() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("reclaimable");
    let shared = b"shared layer bytes";
    let a_only = b"a private layer";
    let b_only = b"b private layer!!";
    let ds = Digest::of(shared);
    let da = Digest::of(a_only);
    let db = Digest::of(b_only);
    s.put_bytes(shared, &ds, "blob").unwrap();
    s.put_bytes(a_only, &da, "blob").unwrap();
    s.put_bytes(b_only, &db, "blob").unwrap();
    let mut ra = record("t/a", Some("1"), 7);
    ra.layers = vec![ds.to_string(), da.to_string()];
    let mut rb = record("t/b", None, 8);
    rb.layers = vec![ds.to_string(), db.to_string()];
    s.put_record(ra).unwrap();
    s.put_record(rb).unwrap();
    let all = s.list().unwrap();
    let rb = all.iter().find(|r| r.tag.is_none()).unwrap().clone();
    // The shared blob survives with A: only B's own layer counts.
    assert_eq!(
        s.reclaimable_bytes(std::slice::from_ref(&rb)).unwrap(),
        b_only.len() as u64
    );
    // Both doomed: every blob once, the shared one included.
    let ra = all.iter().find(|r| r.tag.is_some()).unwrap().clone();
    assert_eq!(
        s.reclaimable_bytes(&[ra, rb]).unwrap(),
        (shared.len() + a_only.len() + b_only.len()) as u64
    );
    // A missing blob counts nothing rather than failing the report.
    assert_eq!(
        s.blob_bytes("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
        0
    );
    assert_eq!(s.blob_bytes("bogus"), 0);
    let _ = std::fs::remove_dir_all(s.root());
}
