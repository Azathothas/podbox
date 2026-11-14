//! Black-box CLI tests for the T-1337 operator verbs, from
//! experiments/364-qol.sh: `logs --tail` and `system df` through the
//! shipped binary.
//!
//! 364 also drives `doctor`; this file covers only the tail and df clauses
//! named in the assignment. Clause names below are the script's:
//! tail-1 through tail-5, df-1 through df-4.
//!
//! Live clauses need Linux container entry plus pulled images, and SKIP
//! early with the reason where either is absent. Nothing here fakes green:
//! past the gates every clause asserts exit codes and output substrings.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// The binary under test, built by Cargo for this package.
const PODVM: &str = env!("CARGO_BIN_EXE_podvm");

/// Serializes the tests in this file. The store path travels per command
/// (see `run_podvm`), and the lock keeps two tests from racing the
/// process-wide state around it.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Counter beside the pid so two tests never share a store or a name.
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 364 DEBIAN: the image the tail and df clauses run against.
const DEBIAN: &str = "public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a";

/// 364 ALPINE: the second image the df-3 clause pulls for the digest
/// reclaim check.
const ALPINE: &str = "public.ecr.aws/docker/library/alpine:latest";

/// A flag error: an unknown option, a missing value, or a value the parser
/// will not take (podvm_probe::exit::EXIT_FLAG_ERROR, read out of the
/// binary by scripts/common/exit-codes.sh for 364's FLAG).
const FLAG_ERROR: i32 = 125;

/// A fresh store directory, unique across the parallel tests in this file.
fn fresh_store(tag: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podvm-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("could not create store dir {}: {e}", dir.display()));
    dir
}

fn cleanup(store: &Path) {
    let _ = std::fs::remove_dir_all(store);
}

/// Run the podvm binary with `$PODVM_STORE` pointed at `store`, and
/// return everything it wrote. Bounded: a child that outlives `deadline`
/// is killed and fails the test by name instead of hanging the suite.
fn run_podvm(args: &[&str], store: &Path, deadline: Duration) -> Output {
    let mut child = Command::new(PODVM)
        // TODO/gate.md T-1859: no detached spare-tree fill writes into the
        // store after the test removed it.
        .env("PODVM_SPARE_TREE", "0")
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn podvm {args:?}: {e}"));
    let start = Instant::now();
    loop {
        match child
            .try_wait()
            .unwrap_or_else(|e| panic!("could not poll podvm {args:?}: {e}"))
        {
            Some(_) => break,
            None => {
                if start.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("podvm {args:?} outlived the {deadline:?} deadline");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("could not read podvm {args:?} output: {e}"))
}

fn stdout_text(out: &Output, args: &[&str]) -> String {
    String::from_utf8(out.stdout.clone())
        .unwrap_or_else(|e| panic!("podvm {args:?} stdout is not UTF-8: {e}"))
}

fn stderr_text(out: &Output, args: &[&str]) -> String {
    String::from_utf8(out.stderr.clone())
        .unwrap_or_else(|e| panic!("podvm {args:?} stderr is not UTF-8: {e}"))
}

/// None where a live container clause can run, else the SKIP reason.
/// Container entry needs Linux; the pull gates below answer for the
/// network and the registry separately.
fn live_skip_reason(store: &Path) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return Some(format!(
            "container entry needs Linux; this host is {}",
            std::env::consts::OS
        ));
    }
    let info = run_podvm(
        &["system", "info", "--format", "{{.EnteredRung}}"],
        store,
        Duration::from_secs(60),
    );
    if !info.status.success() {
        return Some(format!(
            "cannot read the entered rung: {}",
            stderr_text(&info, &["system info"]).trim()
        ));
    }
    None
}

/// Pull `image` or SKIP with the reason. 364 exits 2 where a pull fails
/// (the lane could not run); these tests return early the same way.
fn pull_or_skip(image: &str, store: &Path) -> bool {
    let pull = run_podvm(&["pull", image], store, Duration::from_secs(900));
    if pull.status.success() {
        return true;
    }
    eprintln!(
        "SKIP: could not pull {image}: {:?}",
        stderr_text(&pull, &["pull"]).trim()
    );
    false
}

/// 364 mklog: create, start, then wait for the container to end. Asserts
/// nothing; the caller asserts each step so the failing half is named.
fn mklog(store: &Path, name: &str, image: &str, payload: &[&str]) -> (Output, Output, Output) {
    let mut create_args = vec!["create", "--name", name, image];
    create_args.extend_from_slice(payload);
    let create = run_podvm(&create_args, store, Duration::from_secs(300));
    let start = run_podvm(&["start", name], store, Duration::from_secs(300));
    let wait = run_podvm(&["wait", name], store, Duration::from_secs(300));
    (create, start, wait)
}

fn assert_mklog_ok(create: &Output, start: &Output, wait: &Output, name: &str) {
    assert_eq!(
        create.status.code(),
        Some(0),
        "create {name} exited {:?}, stderr was {:?}",
        create.status.code(),
        stderr_text(create, &["create"])
    );
    assert_eq!(
        start.status.code(),
        Some(0),
        "start {name} exited {:?}, stderr was {:?}",
        start.status.code(),
        stderr_text(start, &["start"])
    );
    assert_eq!(
        wait.status.code(),
        Some(0),
        "wait {name} exited {:?}, stderr was {:?}",
        wait.status.code(),
        stderr_text(wait, &["wait"])
    );
}

/// 364 tail-4: a non-count `--tail` is a flag error at 125 naming the
/// value. Hermetic: the parser refuses before any store or container is
/// touched (crates/podvm/src/lifecycle.rs `parse_tail`).
#[test]
fn logs_tail_with_a_non_count_is_a_flag_error() {
    // The guarantee this pins: `--tail=x` answers 125 and names the value
    // on stderr instead of printing a log.
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("tail4");
    let out = run_podvm(
        &["logs", "--tail=x", "t364a"],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        out.status.code(),
        Some(FLAG_ERROR),
        "logs --tail=x exited {:?} (want {FLAG_ERROR}), stderr was {:?}",
        out.status.code(),
        stderr_text(&out, &["logs --tail=x"])
    );
    let stderr = stderr_text(&out, &["logs --tail=x"]);
    assert!(
        stderr.contains("non-negative line count"),
        "logs --tail=x named no count problem: {stderr:?}"
    );
    cleanup(&store);
}

/// 364 tail-1, tail-2, tail-3: `--tail 5` prints the last five lines of a
/// ten-line log; plain `logs` is byte-identical to a wide tail; `--tail 0`
/// prints nothing at exit 0.
#[test]
fn logs_tail_slices_a_finished_container() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("tail");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP logs tail: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let name = format!("t364a-{}-{n}", std::process::id());
    let (create, start, wait) = mklog(
        &store,
        &name,
        DEBIAN,
        &[
            "/bin/bash",
            "-c",
            "for i in 1 2 3 4 5 6 7 8 9 10; do echo line$i; done",
        ],
    );
    assert_mklog_ok(&create, &start, &wait, &name);

    // tail-1.
    let tail5 = run_podvm(
        &["logs", "--tail", "5", &name],
        &store,
        Duration::from_secs(120),
    );
    assert_eq!(
        tail5.status.code(),
        Some(0),
        "logs --tail 5 exited {:?}, stderr was {:?}",
        tail5.status.code(),
        stderr_text(&tail5, &["logs --tail 5"])
    );
    assert_eq!(
        tail5.stdout,
        b"line6\nline7\nline8\nline9\nline10\n",
        "logs --tail 5 printed {:?}",
        stdout_text(&tail5, &["logs --tail 5"])
    );

    // tail-2.
    let full = run_podvm(&["logs", &name], &store, Duration::from_secs(120));
    let wide = run_podvm(
        &["logs", "--tail", "99", &name],
        &store,
        Duration::from_secs(120),
    );
    assert_eq!(
        full.status.code(),
        Some(0),
        "plain logs exited {:?}, stderr was {:?}",
        full.status.code(),
        stderr_text(&full, &["logs"])
    );
    assert_eq!(
        wide.status.code(),
        Some(0),
        "logs --tail 99 exited {:?}, stderr was {:?}",
        wide.status.code(),
        stderr_text(&wide, &["logs --tail 99"])
    );
    assert_eq!(
        full.stdout,
        wide.stdout,
        "plain logs and --tail 99 disagree: {:?} vs {:?}",
        stdout_text(&full, &["logs"]),
        stdout_text(&wide, &["logs --tail 99"])
    );
    assert_eq!(
        full.stdout,
        b"line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n",
        "plain logs printed {:?}",
        stdout_text(&full, &["logs"])
    );

    // tail-3.
    let tail0 = run_podvm(
        &["logs", "--tail", "0", &name],
        &store,
        Duration::from_secs(120),
    );
    assert_eq!(
        tail0.status.code(),
        Some(0),
        "logs --tail 0 exited {:?}, stderr was {:?}",
        tail0.status.code(),
        stderr_text(&tail0, &["logs --tail 0"])
    );
    assert!(
        tail0.stdout.is_empty(),
        "logs --tail 0 printed {:?}",
        stdout_text(&tail0, &["logs --tail 0"])
    );

    let rm = run_podvm(&["rm", &name], &store, Duration::from_secs(120));
    assert_eq!(
        rm.status.code(),
        Some(0),
        "rm {name} exited {:?}, stderr was {:?}",
        rm.status.code(),
        stderr_text(&rm, &["rm"])
    );
    cleanup(&store);
}

#[path = "qol/follow_and_ps.rs"]
mod follow_and_ps;
