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
const PODBOX: &str = env!("CARGO_BIN_EXE_podbox");

/// Serializes the tests in this file. The store path travels per command
/// (see `run_podbox`), and the lock keeps two tests from racing the
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
/// will not take (podbox_probe::exit::EXIT_FLAG_ERROR, read out of the
/// binary by scripts/common/exit-codes.sh for 364's FLAG).
const FLAG_ERROR: i32 = 125;

/// A fresh store directory, unique across the parallel tests in this file.
fn fresh_store(tag: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-cli-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("could not create store dir {}: {e}", dir.display()));
    dir
}

fn cleanup(store: &Path) {
    let _ = std::fs::remove_dir_all(store);
}

/// Run the podbox binary with `$PODBOX_STORE` pointed at `store`, and
/// return everything it wrote. Bounded: a child that outlives `deadline`
/// is killed and fails the test by name instead of hanging the suite.
fn run_podbox(args: &[&str], store: &Path, deadline: Duration) -> Output {
    let mut child = Command::new(PODBOX)
        .args(args)
        .env("PODBOX_STORE", store)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn podbox {args:?}: {e}"));
    let start = Instant::now();
    loop {
        match child
            .try_wait()
            .unwrap_or_else(|e| panic!("could not poll podbox {args:?}: {e}"))
        {
            Some(_) => break,
            None => {
                if start.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("podbox {args:?} outlived the {deadline:?} deadline");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("could not read podbox {args:?} output: {e}"))
}

fn stdout_text(out: &Output, args: &[&str]) -> String {
    String::from_utf8(out.stdout.clone())
        .unwrap_or_else(|e| panic!("podbox {args:?} stdout is not UTF-8: {e}"))
}

fn stderr_text(out: &Output, args: &[&str]) -> String {
    String::from_utf8(out.stderr.clone())
        .unwrap_or_else(|e| panic!("podbox {args:?} stderr is not UTF-8: {e}"))
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
    let info = run_podbox(
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
    let pull = run_podbox(&["pull", image], store, Duration::from_secs(900));
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
    let create = run_podbox(&create_args, store, Duration::from_secs(300));
    let start = run_podbox(&["start", name], store, Duration::from_secs(300));
    let wait = run_podbox(&["wait", name], store, Duration::from_secs(300));
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
/// touched (crates/podbox-cli/src/lifecycle.rs `parse_tail`).
#[test]
fn logs_tail_with_a_non_count_is_a_flag_error() {
    // The guarantee this pins: `--tail=x` answers 125 and names the value
    // on stderr instead of printing a log.
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("tail4");
    let out = run_podbox(
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
    let tail5 = run_podbox(
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
    let full = run_podbox(&["logs", &name], &store, Duration::from_secs(120));
    let wide = run_podbox(
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
    let tail0 = run_podbox(
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

    let rm = run_podbox(&["rm", &name], &store, Duration::from_secs(120));
    assert_eq!(
        rm.status.code(),
        Some(0),
        "rm {name} exited {:?}, stderr was {:?}",
        rm.status.code(),
        stderr_text(&rm, &["rm"])
    );
    cleanup(&store);
}

/// 364 tail-5: `logs -f --tail 3` prints the last three lines, then follows
/// new lines to the container's end instead of replaying the file. The
/// container already ended (mklog waits), so follow prints the tail and
/// exits.
#[test]
fn logs_follow_starts_from_the_tail() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("follow");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP logs follow: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let name = format!("t364b-{}-{n}", std::process::id());
    let (create, start, wait) = mklog(
        &store,
        &name,
        DEBIAN,
        &[
            "/bin/bash",
            "-c",
            "for i in 1 2 3 4 5; do echo line$i; done",
        ],
    );
    assert_mklog_ok(&create, &start, &wait, &name);

    let follow = run_podbox(
        &["logs", "-f", "--tail", "3", &name],
        &store,
        Duration::from_secs(120),
    );
    assert_eq!(
        follow.status.code(),
        Some(0),
        "logs -f --tail 3 exited {:?}, stderr was {:?}",
        follow.status.code(),
        stderr_text(&follow, &["logs -f --tail 3"])
    );
    assert_eq!(
        follow.stdout,
        b"line3\nline4\nline5\n",
        "logs -f --tail 3 printed {:?}",
        stdout_text(&follow, &["logs -f --tail 3"])
    );

    let rm = run_podbox(&["rm", &name], &store, Duration::from_secs(120));
    assert_eq!(
        rm.status.code(),
        Some(0),
        "rm {name} exited {:?}, stderr was {:?}",
        rm.status.code(),
        stderr_text(&rm, &["rm"])
    );
    cleanup(&store);
}

/// 364 df-4: `system df --bogus` is a flag error and `system df --help`
/// prints usage at 0. Hermetic: `df` decides its arguments before it
/// touches the store.
#[test]
fn system_df_flag_shapes_need_no_store() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df4");
    let bogus = run_podbox(
        &["system", "df", "--bogus"],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        bogus.status.code(),
        Some(FLAG_ERROR),
        "system df --bogus exited {:?} (want {FLAG_ERROR}), stderr was {:?}",
        bogus.status.code(),
        stderr_text(&bogus, &["system df --bogus"])
    );
    let help = run_podbox(&["system", "df", "--help"], &store, Duration::from_secs(60));
    assert_eq!(
        help.status.code(),
        Some(0),
        "system df --help exited {:?}, stderr was {:?}",
        help.status.code(),
        stderr_text(&help, &["system df --help"])
    );
    let stdout = stdout_text(&help, &["system df --help"]);
    assert!(
        stdout.contains("podbox system df"),
        "system df --help printed no usage: {stdout:?}"
    );
    cleanup(&store);
}

/// 364 df-2, hermetic shape: the totals lines are present on a fresh store,
/// and accounting leaves the store untouched (the image count is identical
/// before and after).
#[test]
fn system_df_reports_totals_and_touches_nothing() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("dfempty");
    let before = run_podbox(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.status.code(),
        Some(0),
        "images -q exited {:?}, stderr was {:?}",
        before.status.code(),
        stderr_text(&before, &["images -q"])
    );
    let df = run_podbox(&["system", "df"], &store, Duration::from_secs(60));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    assert!(
        stdout.contains("REPOSITORY"),
        "system df printed no header: {stdout:?}"
    );
    for line in [
        "Images:",
        "Containers:",
        "Stored blobs:",
        "Extracted rootfs:",
        "Reclaimable:",
    ] {
        assert!(
            stdout.contains(line),
            "system df output has no {line:?} line: {stdout:?}"
        );
    }
    let after = run_podbox(&["system", "df"], &store, Duration::from_secs(60));
    assert_eq!(
        after.status.code(),
        Some(0),
        "second system df exited {:?}",
        after.status.code()
    );
    let recount = run_podbox(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.stdout,
        recount.stdout,
        "df changed the store: {:?} before, {:?} after",
        stdout_text(&before, &["images -q"]),
        stdout_text(&recount, &["images -q"])
    );
    cleanup(&store);
}

/// 364 df-1: `system df` exits 0 and rows the pulled and extracted image
/// with nonzero stored and extracted bytes.
#[test]
fn system_df_rows_the_pulled_image_with_nonzero_bytes() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df1");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    // 364's tail clauses run containers before df-1, so the image is
    // extracted by the time df rows it. A bare pull leaves ROOTFS at 0 B
    // and the script's own last-field check would refuse it too.
    let extract = run_podbox(&["extract", DEBIAN], &store, Duration::from_secs(900));
    if !extract.status.success() {
        eprintln!(
            "SKIP system df: could not extract {DEBIAN}: {:?}",
            stderr_text(&extract, &["extract"]).trim()
        );
        cleanup(&store);
        return;
    }
    let df = run_podbox(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    assert!(
        stdout.contains("REPOSITORY"),
        "system df printed no header: {stdout:?}"
    );
    let row = stdout
        .lines()
        .find(|l| l.contains("debian"))
        .unwrap_or_else(|| panic!("system df has no debian row: {stdout:?}"))
        .to_string();
    // 364 reads the last two columns (stored, rootfs) and refuses a zero
    // in either field exactly, not a "0 B" anywhere in the row.
    let fields: Vec<&str> = row.split_whitespace().collect();
    assert!(
        fields.len() >= 4,
        "debian row is too short to bill: {row:?} in {stdout:?}"
    );
    let stored = fields[fields.len() - 4..fields.len() - 2].join(" ");
    let rootfs = fields[fields.len() - 2..].join(" ");
    assert!(
        stored != "0 B" && rootfs != "0 B",
        "debian row bills nothing: stored={stored:?} rootfs={rootfs:?} in {row:?}"
    );
    cleanup(&store);
}

/// 364 df-2, live shape: with an image present the totals lines are still
/// there and the store is untouched by df (the image count is identical
/// before and after).
#[test]
fn system_df_leaves_a_live_store_untouched() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df2");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df totals: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    let before = run_podbox(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.status.code(),
        Some(0),
        "images -q exited {:?}, stderr was {:?}",
        before.status.code(),
        stderr_text(&before, &["images -q"])
    );
    let df = run_podbox(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    for line in [
        "Images:",
        "Containers:",
        "Stored blobs:",
        "Extracted rootfs:",
        "Reclaimable:",
    ] {
        assert!(
            stdout.contains(line),
            "system df output has no {line:?} line: {stdout:?}"
        );
    }
    let again = run_podbox(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        again.status.code(),
        Some(0),
        "second system df exited {:?}",
        again.status.code()
    );
    let after = run_podbox(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        stdout_text(&before, &["images -q"]),
        stdout_text(&after, &["images -q"]),
        "df changed the store"
    );
    cleanup(&store);
}

/// 364 df-3: a digest pull is reclaimable, to the byte. df's Reclaimable
/// line equals `image prune -f`'s Total reclaimed space with a dangling
/// image, and df after reads zero.
#[test]
fn system_df_reclaimable_matches_prune_to_the_byte() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df3");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df reclaimable: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    if !pull_or_skip(ALPINE, &store) {
        cleanup(&store);
        return;
    }
    let inspect = run_podbox(
        &["inspect", "--format", "{{.RepoDigests}}", ALPINE],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        inspect.status.code(),
        Some(0),
        "inspect of {ALPINE} exited {:?}, stderr was {:?}",
        inspect.status.code(),
        stderr_text(&inspect, &["inspect"])
    );
    let digest_ref = stdout_text(&inspect, &["inspect"]).trim().to_string();
    assert!(
        !digest_ref.is_empty(),
        "inspect printed no RepoDigests for {ALPINE}"
    );
    let rmi = run_podbox(&["rmi", ALPINE], &store, Duration::from_secs(120));
    assert_eq!(
        rmi.status.code(),
        Some(0),
        "rmi {ALPINE} exited {:?}, stderr was {:?}",
        rmi.status.code(),
        stderr_text(&rmi, &["rmi"])
    );
    let digest_pull = run_podbox(&["pull", &digest_ref], &store, Duration::from_secs(900));
    if !digest_pull.status.success() {
        eprintln!(
            "SKIP system df reclaimable: digest pull of {digest_ref} failed: {:?}",
            stderr_text(&digest_pull, &["pull"]).trim()
        );
        cleanup(&store);
        return;
    }
    let images = run_podbox(&["images"], &store, Duration::from_secs(60));
    let images_stdout = stdout_text(&images, &["images"]);
    assert!(
        images_stdout.contains("<none>"),
        "digest pull is not dangling: {images_stdout:?}"
    );

    let df = run_podbox(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}",
        df.status.code()
    );
    let df_stdout = stdout_text(&df, &["system df"]);
    let reclaim_line = df_stdout
        .lines()
        .find(|l| l.starts_with("Reclaimable:"))
        .unwrap_or_else(|| panic!("system df has no Reclaimable line: {df_stdout:?}"))
        .to_string();
    // 364 strips the trailing "(what ...)" note before comparing.
    let r1 = reclaim_line
        .trim_start_matches("Reclaimable:")
        .trim()
        .split(" (")
        .next()
        .unwrap_or("")
        .to_string();
    assert!(
        !r1.is_empty() && r1 != "0 B",
        "Reclaimable reads {r1:?} with a dangling image: {df_stdout:?}"
    );

    let prune = run_podbox(&["image", "prune", "-f"], &store, Duration::from_secs(300));
    assert_eq!(
        prune.status.code(),
        Some(0),
        "image prune -f exited {:?}, stderr was {:?}",
        prune.status.code(),
        stderr_text(&prune, &["image prune -f"])
    );
    let prune_stdout = stdout_text(&prune, &["image prune -f"]);
    let r2 = prune_stdout
        .lines()
        .find(|l| l.contains("Total reclaimed space:"))
        .and_then(|l| l.split("Total reclaimed space:").nth(1))
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    assert!(
        !r2.is_empty(),
        "prune printed no Total reclaimed space line: {prune_stdout:?}"
    );
    assert_eq!(
        r1, r2,
        "prune freed {r2:?} but df reported {r1:?}: {prune_stdout:?}"
    );

    let df_after = run_podbox(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df_after.status.code(),
        Some(0),
        "system df after prune exited {:?}",
        df_after.status.code()
    );
    let after_stdout = stdout_text(&df_after, &["system df"]);
    let after_line = after_stdout
        .lines()
        .find(|l| l.starts_with("Reclaimable:"))
        .unwrap_or("");
    assert!(
        after_line.starts_with("Reclaimable: 0 B"),
        "df after prune reads {after_line:?}, want Reclaimable 0 B: {after_stdout:?}"
    );
    cleanup(&store);
}
