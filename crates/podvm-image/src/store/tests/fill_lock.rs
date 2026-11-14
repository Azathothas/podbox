use super::*;

/// ⭐ T-1781's second lock, and the defect it exists to stop: a **fill** hold
/// is not a payload hold, so a removal goes ahead while a spare tree is being
/// copied. ⛔ Red before this: `rm` starts the detached `podvm tree-prefill`,
/// the fill took the payload hold, and the `rmi` that followed in the same
/// second was refused with "in use by a running container" while `podvm ps -a`
/// was empty. Measured 4 times in 8 on the shipped `v0.2.0-beta.16` binary.
///
/// ⚠ The control at the end is the half that must not change: the payload hold
/// still refuses, which is T-0204 and is what keeps a GC out of a running
/// payload's extraction.
#[test]
fn a_fill_hold_does_not_refuse_a_removal() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("fillhold");
    let r = record("docker.io/library/alpine", Some("latest"), 41);
    s.put_record(r.clone()).unwrap();

    let filling = s.hold_fill(&r).unwrap();
    assert!(
        !s.in_use(&r).unwrap(),
        "a fill hold registered as a payload hold, so a removal refuses a cache"
    );
    s.remove("alpine:latest").expect("a removal during a fill");
    assert!(
        s.find("alpine:latest").unwrap().is_empty(),
        "the removal kept the image the fill was copying"
    );
    assert!(
        !s.image_fill_lock_path(&r).unwrap().exists(),
        "the removal left the fill lock of the image it deleted"
    );
    drop(filling);

    // ⛔ The control. A payload hold still refuses: this test is not a licence
    // to remove an image out from under a running container.
    let r = record("docker.io/library/busybox", Some("latest"), 42);
    s.put_record(r.clone()).unwrap();
    let payload = s.hold(&r).unwrap();
    assert!(
        s.remove("busybox:latest").is_err(),
        "a payload hold stopped refusing a removal"
    );
    drop(payload);
    let _ = std::fs::remove_dir_all(s.root());
}
