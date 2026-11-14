//! TODO/supervise.md T-1807, in a test binary of its own: the launcher
//! of `pod_supervise::start` forks, and a fork of a process whose other
//! test threads hold a lock of the allocator can stop the child in it.

use pod_supervise::launcher::{Launch, Payload};
use podvm_image::Store;
use podvm_supervise::{create, get, kill, remove, settings, table};
use std::os::fd::{BorrowedFd, OwnedFd};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn stdio(launch: &Launch<'_>, n: i64) -> Stdio {
    let (_, host) = launch.pass.iter().copied().find(|(c, _)| *c == n).unwrap();
    // SAFETY: the launcher holds `host` open for the whole entry.
    let fd: OwnedFd = unsafe { BorrowedFd::borrow_raw(host as i32) }
        .try_clone_to_owned()
        .unwrap();
    Stdio::from(fd)
}

fn enter(_: &table::Container, launch: Launch<'_>) -> Result<Payload, String> {
    let child = Command::new("/bin/sleep")
        .arg("30")
        .stdin(stdio(&launch, 0))
        .stdout(stdio(&launch, 1))
        .stderr(stdio(&launch, 2))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(Payload {
        pid: child.id() as i64,
        keep: Box::new(child),
    })
}

/// A record after `kill` and `start` holds no `exit_code`, `finished_at`
/// or `end` of the run before, and a record that `reconcile` marks `Dead`
/// names no pid. The payload is a plain `sleep` through
/// `pod_supervise::start`, so the test needs no image.
#[test]
fn restart_clears_end_and_a_dead_record_names_no_pid() {
    let d = std::env::temp_dir().join(format!("podvm-t1807-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let s = Store::open(&d).unwrap();
    let set = settings(&s);
    let c = create(
        &s,
        Some("r1"),
        "img",
        "sha256:0",
        "/tmp",
        vec!["sleep".into(), "30".into()],
        Vec::new(),
        "/".into(),
        "chroot",
        Vec::new(),
        0,
        Vec::new(),
    )
    .unwrap();
    let until = |test: &dyn Fn(&table::Container) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let c = get(&s, "r1").unwrap();
            if test(&c) {
                return c;
            }
            assert!(Instant::now() < deadline, "no change in 10 s: {c:?}");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    pod_supervise::start::<table::Detail, _>(&set, "r1", enter).unwrap();
    kill(&s, "r1", 15).unwrap();
    let ended = until(&|c| c.exit_code.is_some());
    assert_eq!(ended.exit_code, Some(143), "{ended:?}");
    let again = pod_supervise::start::<table::Detail, _>(&set, "r1", enter).unwrap();
    for x in [again, get(&s, "r1").unwrap()] {
        assert_eq!(x.state, table::State::Running, "{x:?}");
        assert!(
            x.exit_code.is_none() && x.finished_at.is_none() && x.end.is_none(),
            "a restarted record holds the end of the run before: {x:?}"
        );
    }
    // A launcher that dies with no end: `reconcile` marks the record Dead.
    let pids = get(&s, "r1").unwrap();
    for p in [pids.launcher_pid, pids.pid].into_iter().flatten() {
        let _ = Command::new("kill").args(["-9", &p.to_string()]).status();
    }
    let dead = until(&|c| c.state != table::State::Running);
    assert_eq!(dead.state, table::State::Dead, "{dead:?}");
    assert_eq!((dead.pid, dead.launcher_pid), (None, None), "{dead:?}");
    let _ = remove(&s, &c.id, true);
    let _ = std::fs::remove_dir_all(&d);
}
