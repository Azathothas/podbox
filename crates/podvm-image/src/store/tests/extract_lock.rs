use super::*;

/// ⭐ The invariant of `Store::hold_extract`.
///
/// ⛔ Red without it: two callers that both read `is_extracted` as false write
/// one tree at the same time. Measured 2026-11-14: eight `podvm create` at
/// once of a never-extracted image, `v0.2.0-beta.16` created 12 of 48
/// containers over six rounds and lost all eight in four of them. The failures
/// were `cannot create the symlink "bin/arch": EEXIST`, a copy of a tree
/// another process had removed, and a payload that "names no file in the
/// image". The lock is exclusive, so the second extraction waits for the first
/// and then finds its tree.
#[test]
fn an_extraction_lock_holds_out_a_second_extraction() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = std::sync::Arc::new(scratch("extracthold"));
    let r = record("docker.io/library/alpine", Some("latest"), 18);
    s.put_record(r.clone()).unwrap();

    let held = s.hold_extract(&r).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    let s2 = std::sync::Arc::clone(&s);
    let r2 = r.clone();
    let t = std::thread::spawn(move || {
        let second = s2.hold_extract(&r2).is_ok();
        let _ = tx.send(second);
    });
    assert!(
        rx.recv_timeout(std::time::Duration::from_millis(400))
            .is_err(),
        "a second extraction of one image got the lock while the first held it: \
         two of them would write the same tree"
    );

    drop(held);
    assert!(
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("the second extraction never got the lock after the first ended"),
        "the second extraction failed after the first released the lock"
    );
    t.join().unwrap();
}
