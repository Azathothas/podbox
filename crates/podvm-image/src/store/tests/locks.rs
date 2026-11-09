use super::*;

/// ⭐ T-0211's plant. This is the defect that surfaced as an intermittent
/// failure of the test above at 2 runs of 6 of the full workspace, and it is
/// made to fail on demand here rather than once a week.
///
/// ⛔ Red before the fix: with the image lock opened without `O_CLOEXEC`,
/// the forked child below inherits the fd, the `flock` outlives
/// `drop(held)`, and `in_use` answers true for an image nothing is using.
/// The forking thread in the real suite is `probe_cache`'s, whose `measure`
/// makes one fresh child per probe; the `clone_fork` here is that, reduced
/// to the one call that matters.
#[test]
fn a_fork_while_the_lock_is_held_does_not_extend_it() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("forkhold");
    let r = record("docker.io/library/alpine", Some("latest"), 17);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap(), "the holder itself did not register");

    // ⛔ A pipe, not a sleep. `clone_fork` sheds the registered fds in the
    // child before it returns there, so the child is only known to have
    // shed once it has run at all, and a parent that asserts before the
    // child is scheduled reads the fd as still open and fails for a reason
    // that has nothing to do with the defect. That is the same shape of
    // intermittent failure T-0211 itself arrived as, so this test is made
    // to wait for the fact rather than for a duration.
    let mut fds = [0i32; 2];
    sys::pipe2(&mut fds, 0).unwrap();
    let (r_fd, w_fd) = (fds[0] as i64, fds[1] as i64);

    // ⚠ `clone_fork` and not `std::process::Command`: the defect is about
    // what a bare `fork` inherits, and spawning a process would exec and so
    // hide it behind the `FD_CLOEXEC` the other test covers.
    let pid = unsafe { sys::clone_fork(sys::SIGCHLD) }.unwrap();
    if pid == 0 {
        // ---- child. It has already shed; say so, then outlive the drop.
        let _ = sys::write(w_fd, b"x");
        std::thread::sleep(std::time::Duration::from_millis(400));
        // ⛔ Never returns into the test harness: exits without unwinding.
        sys::exit_group(0);
    }
    let _ = sys::close(w_fd);
    let mut ack = [0u8; 1];
    assert_eq!(sys::read(r_fd, &mut ack).unwrap(), 1, "the child never ran");

    drop(held);
    let free = s.in_use(&r).map(|u| !u);

    let mut status = 0;
    let _ = sys::close(r_fd);
    let _ = sys::wait4(pid, &mut status);
    assert!(
        free.unwrap(),
        "the holder released the lock and it is still held: a forked child \
             inherited the fd, which is TODO/image.md T-0211{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⛔ **NO TWO TESTS IN THIS MODULE RUN AT THE SAME TIME**, and this is what
/// stops them. Two reasons, and either alone would need it.
///
/// 1. **The fork.** `cargo test` runs tests in THREADS of one process. A
///    bare `fork` copies every open descriptor of that process, including a
///    lock another test is holding in another thread; that child then holds
///    the lock for as long as it lives, and the other test's assertion --
///    "the holder released it and nobody else has it" -- fails for a reason
///    that has nothing to do with its subject. Measured on 2026-09-09:
///    `a_spawned_process_does_not_inherit_the_lock` failed once in a
///    full-workspace run and passed alone and on the retry, which is the
///    shape a flake takes and is not one.
///    [`TODO/image.md`](../../../TODO/image.md) T-0211.
/// 2. **The slots.** Every `Lock` this crate takes registers one of sixteen
///    process-wide fork-shed slots (`sys::FORK_CLOSE_SLOTS`), and a
///    seventeenth is refused by name. Twenty libtest threads holding one to
///    three locks each exhaust them, and the test that asks last is refused
///    for a reason that has nothing to do with its subject. Measured on
///    2026-09-21: 8 refusals in 10 parallel runs of this suite, a different
///    victim each time, and 97 of 97 serial.
///    [`TODO/image.md`](../../../TODO/image.md) T-1310.
///
/// ⚠ It is the same trap as T-0603's, which counted `/proc/self/task`
/// before and after a spawn and failed about one run in five.
///
/// ⛔ Helpers in this module (`scratch`, `record`, `who_holds`) must never
/// take it: a test calls them with it already held, and a second
/// acquisition on this thread deadlocks. Only `#[test]` functions take it,
/// exactly once, as their first line. That is enough by audit: no test
/// outside this module holds a `Lock` (`probe_cache` and `pull` tests only
/// `Store::open` fresh directories, which sweeps nothing and takes no
/// slot), and the implementation review greps every test body below for the
/// acquisition line.
pub(super) static STORE_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// ⭐ T-0211's second half, and it is a **different** failure from the one
/// above rather than the same one written twice.
///
/// `clone_fork` sheds registered fds, so nothing podvm forks itself can
/// carry a lock away. ⛔ `std::process::Command` forks inside libstd and
/// never passes through `clone_fork`, so the shed list cannot reach it and
/// `O_CLOEXEC` on the descriptor is the only thing that does. Reverting
/// either defence turns exactly one of these two tests red, which is how it
/// was confirmed they are independent.
#[test]
fn a_spawned_process_does_not_inherit_the_lock() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("spawnhold");
    let r = record("docker.io/library/alpine", Some("latest"), 19);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    // ⛔ The child announces itself on stdout and the parent reads that
    // before asserting. `O_CLOEXEC` takes the fd away at the **exec**, so a
    // parent that asserts while the child is still between `fork` and
    // `execve` measures the fork window instead, which is the other test's
    // subject, and would make this one fail for the wrong reason.
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("echo ready; sleep 2")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("/bin/sh");
    let mut line = [0u8; 6];
    {
        use std::io::Read;
        child
            .stdout
            .as_mut()
            .unwrap()
            .read_exact(&mut line)
            .expect("the child never reached its exec");
    }
    assert_eq!(&line, b"ready\n");

    drop(held);
    let free = s.in_use(&r).map(|u| !u);
    // ⚠ Reaped before the assertion, so a failure does not also leave a
    // process behind for whatever runs next.
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        free.unwrap(),
        "the holder released the lock and a spawned process is still \
             holding it: the lock fd was not O_CLOEXEC, which is the exec half \
             of TODO/image.md T-0211{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ The multi-architecture case, and the one a platform-blind key gets
/// wrong silently: the second pull deletes the first, its blobs are
/// collected, and a caller who pulled both is left with one.
/// ⭐ The door sweep's finding, and it is the same defect as `find_for`'s
/// through a **different** door: `extract` and `inspect` reach the store by
/// `find_one`, which took `.next()`.
#[test]
fn an_ambiguous_reference_is_refused_by_name_and_never_by_position() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("ambig");
    for (plat, arch, n) in [("linux/amd64", "amd64", 31), ("linux/arm64", "arm64", 33)] {
        let mut r = record("docker.io/library/alpine", Some("latest"), n);
        r.platform = plat.into();
        r.architecture = arch.into();
        s.put_record(r).unwrap();
    }

    // ⚠ No platform asked for: the HOST's is preferred, and there is
    // exactly one of those, so this is not ambiguous.
    let got = s.find_one("alpine:latest").unwrap();
    assert_eq!(got.platform, crate::platform::Platform::host().to_string());

    // ⛔ A platform the store does not hold is refused NAMING what it does.
    let e = s
        .find_one_for(
            "alpine:latest",
            Some(&crate::platform::Platform::parse("linux/riscv64").unwrap()),
        )
        .unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("linux/amd64"), "{text}");
    assert!(text.contains("linux/arm64"), "{text}");

    // ⛔ And where the host's platform is not among them either, it refuses
    // rather than taking the first.
    let solo = scratch("ambig2");
    for (plat, arch, n) in [
        ("linux/riscv64", "riscv64", 35),
        ("linux/s390x", "s390x", 37),
    ] {
        let mut r = record("docker.io/library/alpine", Some("latest"), n);
        r.platform = plat.into();
        r.architecture = arch.into();
        solo.put_record(r).unwrap();
    }
    let e = solo.find_one("alpine:latest").unwrap_err();
    let text = format!("{e}");
    assert!(text.contains("will not pick one by position"), "{text}");
    assert!(text.contains("--platform"), "{text}");

    let _ = std::fs::remove_dir_all(s.root());
    let _ = std::fs::remove_dir_all(solo.root());
}

#[test]
fn two_platforms_of_one_tag_are_two_images_and_not_one() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("twoplat");
    let mut amd = record("docker.io/library/alpine", Some("latest"), 21);
    amd.platform = "linux/amd64".into();
    amd.architecture = "amd64".into();
    let mut arm = record("docker.io/library/alpine", Some("latest"), 23);
    arm.platform = "linux/arm64".into();
    arm.architecture = "arm64".into();

    s.put_record(amd.clone()).unwrap();
    s.put_record(arm.clone()).unwrap();
    assert_eq!(
        s.find("alpine:latest").unwrap().len(),
        2,
        "the second pull replaced the first: the platform is not in the key"
    );

    // ⛔ Re-pulling ONE platform replaces only that one. A moving tag is
    // still the normal case and this must not accumulate.
    s.put_record(amd.clone()).unwrap();
    assert_eq!(s.find("alpine:latest").unwrap().len(), 2);

    // Asked for a platform: exactly that one comes back, by name.
    let p = crate::platform::Platform::parse("linux/arm64").unwrap();
    let got = s.find_for("alpine:latest", Some(&p)).unwrap();
    assert_eq!(got.matched.len(), 1);
    assert_eq!(got.matched[0].platform, "linux/arm64");

    // ⚠ Asked for a platform the store does not hold: NOTHING matches, and
    // what it does hold is reported separately so the caller can say "held,
    // for another platform" rather than "no such image".
    let none = crate::platform::Platform::parse("linux/riscv64").unwrap();
    let miss = s.find_for("alpine:latest", Some(&none)).unwrap();
    assert!(
        miss.matched.is_empty(),
        "a wrong-platform record was returned"
    );
    assert_eq!(miss.other_platforms.len(), 2);

    // ⛔ The regression that produced `Found`: with ONE record in the store
    // and a different platform asked for, a count-based short circuit
    // returned it and podvm ran the wrong architecture in silence.
    let solo = scratch("solo");
    let mut only = record("docker.io/library/alpine", Some("latest"), 29);
    only.platform = "linux/amd64".into();
    solo.put_record(only).unwrap();
    let arm = crate::platform::Platform::parse("linux/arm64").unwrap();
    let got = solo.find_for("alpine:latest", Some(&arm)).unwrap();
    assert!(
        got.matched.is_empty(),
        "one record in the store was returned for a platform it is not"
    );
    assert_eq!(got.other_platforms, vec!["linux/amd64".to_string()]);
    let _ = std::fs::remove_dir_all(solo.root());

    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ **The other half of T-0215's fix, and the half that would break
/// [T-0204](#t-0204-images-rmi-tag-and-a-store-gc-that-cannot-delete-a-running-containers-rootfs)
/// in silence.**
///
/// ⛔ `Lock::drop` releases explicitly now, and one lock must NOT be
/// released: the one [`Lock::hand_to_payload`] gave to a running container.
/// The payload holds a duplicate of THIS open file description, and
/// `LOCK_UN` belongs to the description rather than to a descriptor, so
/// releasing here would release the payload's lock as well. A concurrent
/// `rmi` or `prune` could then delete the rootfs the container is executing
/// out of, which is the defect T-0204 and invariant I4 were paid for.
///
/// ⚠ **Red if the exemption goes, and not intermittently.** Make the
/// release unconditional and the first assertion below fails every time.
///
/// ⭐ `dup` stands in for the payload, for the same reason it stands in for
/// a fork in the test above: what the payload gets from `fork` and `execve`
/// is a second descriptor on one description.
#[test]
fn a_lock_handed_to_the_payload_outlives_this_process_dropping_it() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("handed");
    let r = record("docker.io/library/alpine", Some("latest"), 33);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    let fd = held
        .hand_to_payload()
        .expect("hand the lock to the payload");
    // The payload's copy, which a `fork` would have made for it.
    let payload = sys::dup_cloexec(fd).expect("dup the handed descriptor");

    drop(held);
    let still_held = s.in_use(&r).unwrap();
    // ⚠ Closed before the assertion, so a failure leaves nothing behind.
    let _ = sys::close(payload);
    assert!(
        still_held,
        "the lock was handed to the payload and this process released it \
             anyway, so a running container's image reads as free and a prune \
             may delete the rootfs it is executing out of (TODO/image.md T-0204)"
    );

    // ⛔ **THE RELEASE ARRIVES, WHICH IS NOT THE SAME AS ARRIVING AT ONCE,
    // AND MUST NOT BE.** The exemption exists so that THIS process does not
    // release a lock the payload is keeping, so only the LAST reference can
    // release it and that is by design not this thread. The handed lock
    // therefore keeps the asynchronous release this entry is about, and it
    // is the one place that is correct: while the payload lives the lock IS
    // held, and the only window left reads an image as in use for a few
    // hundred microseconds after its container really ended, which is the
    // safe direction and self-corrects on the next attempt.
    // ⚠ Asserting immediacy here is what a first draft of this test did,
    // and it failed 9 and 13 of 30 for exactly the reason T-0215 names.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut freed = false;
    while std::time::Instant::now() < deadline {
        if !s.in_use(&r).unwrap() {
            freed = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        freed,
        "the payload's descriptor is closed and five seconds later the image \
             still reads as in use, so the exemption is a permanent hold rather \
             than a release this thread does not make{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}

/// ⭐ **T-0215's regression test, and it is DETERMINISTIC where the defect
/// it guards was one run in two.**
///
/// ⛔ **The defect.** `Lock::drop` released the lock by closing its
/// descriptor, and a `close` releases an `flock` only when it drops the
/// LAST reference to the open file description. A `fork` makes a second
/// reference, so after one the holder's own `close` no longer completes the
/// release: the record goes away later, when the other reference does, and
/// that is asynchronous. `in_use` read those windows as an image still in
/// use after its holder had released it, and a sweep left an abandoned file
/// behind.
///
/// ⭐ **`dup` is the fork, without the fork.** A duplicate descriptor
/// shares one open file description with the original, which is exactly
/// what a child gets and exactly what makes the `close` insufficient. So
/// this test needs no second process, no thread and no timing: it holds a
/// second reference across the drop and asks whether the lock is gone.
///
/// ⚠ Red before the fix, and not intermittently: with `Drop` closing alone,
/// the duplicate keeps the description alive and `in_use` answers true
/// every time.
#[test]
fn releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("dupfree");
    let r = record("docker.io/library/alpine", Some("latest"), 31);
    s.put_record(r.clone()).unwrap();

    let held = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap(), "the holder itself did not register");

    // ⭐ A second reference to the SAME open file description, which is
    // what a `fork` hands a child. `F_DUPFD_CLOEXEC` and not `dup`, because
    // every descriptor in this tree is close-on-exec.
    let twin = sys::dup_cloexec(held.fd()).expect("dup the lock descriptor");

    drop(held);

    let free = !s.in_use(&r).unwrap();
    // ⚠ Closed before the assertion, so a failure does not leave a
    // descriptor behind for whatever runs next.
    let _ = sys::close(twin);
    assert!(
        free,
        "the holder released the lock and it is still held, because a \
             duplicate of its open file description outlived it: closing a \
             descriptor releases an flock only when it drops the LAST reference \
             (TODO/image.md T-0215){}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );

    // ⛔ And the lock file is usable again afterwards, so the explicit
    // release did not leave the descriptor in a state nothing can take.
    let again = s.hold(&r).unwrap();
    drop(again);
    let _ = std::fs::remove_dir_all(s.root());
}

#[test]
fn two_holders_of_one_image_both_have_to_go_before_it_is_free() {
    let _serialised = STORE_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let s = scratch("twohold");
    let r = record("docker.io/library/alpine", Some("latest"), 11);
    s.put_record(r.clone()).unwrap();
    let a = s.hold(&r).unwrap();
    let b = s.hold(&r).unwrap();
    assert!(s.in_use(&r).unwrap());
    drop(a);
    assert!(s.in_use(&r).unwrap(), "one holder left and it read as free");
    drop(b);
    assert!(
        !s.in_use(&r).unwrap(),
        "both holders went and it still reads as in use{}",
        who_holds(&s.image_lock_path(&r).unwrap())
    );
    let _ = std::fs::remove_dir_all(s.root());
}
