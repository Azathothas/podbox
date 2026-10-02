//! Black-box CLI tests for detached start stdio, from
//! experiments/340-detached-stdio.sh.
//!
//! The script's question: does `podbox run -d` hand back the id before the
//! payload ends, when the caller reads stdout the way every script does?
//! Command substitution reads the pipe to EOF, so a launcher holding the
//! caller's stdout blocks that read for the whole life of the container.
//! The script measured this on 2026-09-09: a `sleep 25` payload took 25 s
//! to hand back an id it had printed in the first second.
//!
//! Live clauses need Linux container entry plus a pulled image, and SKIP
//! early with the reason where either is absent. Nothing here fakes green:
//! past the gates every clause asserts exit codes and output substrings.
//!
//! ⭐ T-1604. A third gate sits between: the tier a detached start enters.
//! Where the binary under test carries no interposer object and the machine
//! denies chroot(2), no no-chroot family runs a dynamic payload and `run -d`
//! refuses at 126 with that sentence. That refusal is correct for the tier,
//! so the test SKIPS with its reason, the way `tests/store_gates.rs` skips
//! where a host refuses namespaces. Where the tier IS available the clause
//! is unchanged and still requires every start.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// The binary under test, built by Cargo for this package.
const PODBOX: &str = env!("CARGO_BIN_EXE_podbox");

/// Serializes the tests in this file. The store path travels per command
/// (see `run_podbox`), and the lock keeps two tests from racing the
/// process-wide state around it.
///
/// ⛔ Taken with `unwrap_or_else(|e| e.into_inner())`, never `unwrap()`.
/// A mutex guards ordering, not memory this test trusts afterwards: every
/// test here builds its own store and reads only its own child's output, so a
/// panic while the guard is held leaves nothing the next test could observe.
/// What it does leave is a POISONED mutex, and `unwrap()` turns that into a
/// second failure that is only a report of the first, so a real defect reads
/// as a confusing cascade. TODO/enter.md T-1604; the house shape is the one in
/// `tests/store_gates.rs` and `tests/curated_refusals.rs`.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Counter beside the pid so two tests never share a store or a name.
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 340 PROMPT_SECONDS: a detached start does a fork and two table writes,
/// so anything near the payload's own duration is the defect.
const PROMPT_SECONDS: u64 = 5;

/// 340 PAYLOAD_SECONDS: long enough that a blocking `run -d` is
/// unmistakable, short enough that a reproduction stays cheap.
const PAYLOAD_SECONDS: &str = "20";

/// 340 loops 40 times. This file runs 3 starts: enough to catch a handoff
/// that fails sometimes, cheap enough for a suite.
const ITERATIONS: u32 = 3;

/// A fresh store directory, unique across the parallel tests in this file.
fn fresh_store(tag: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-cli-{}-{}-{n}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("could not create store dir {}: {e}", dir.display()));
    dir
}

fn cleanup(store: &Path) {
    let _ = std::fs::remove_dir_all(store);
}

/// ⭐ T-1604. The refusal the no-chroot family emits when this binary carries
/// no interposer object for the payload's libc: `lifecycle.rs` `decide_entry`
/// wraps `interpose::Reach::Declined` as "the loader family declines the
/// payload: ...", and `interpose::classify_elf` names the absent object.
const TIER_REFUSAL_HEAD: &str =
    "chroot(2) is denied on this machine, and no no-chroot family runs this payload";
/// The absent-object leg on its own. It is the only sentence in the tree that
/// reports the tier is missing, and it is absent from every other no-chroot
/// refusal, so it is what separates a tier skip from a product failure.
const TIER_REFUSAL_TAIL: &str = "carries no";

/// docker's found-but-not-invocable code, which podbox shares
/// (TODO/cli.md T-0802). The no-chroot tier declines with it.
const EXIT_CANNOT_INVOKE: i32 = 126;

/// None where the no-chroot family could run the payload, else the SKIP
/// reason naming why the tier is unavailable.
///
/// ⛔ It matches the REFUSAL, not the exit code alone. 126 is what podbox
/// returns for a loader this machine cannot run, a `#!` script whose
/// interpreter dangles, and a payload that is not an ELF file as well. Those
/// are real failures, so the sentence is required too.
///
/// ⚠ A non-zero exit is REQUIRED as well, and the two together are the
/// condition. A start that exited 0 has started a container, and a skip there
/// would discard a green run over a real defect. Neither half is the test.
fn tier_skip_reason(code: Option<i32>, stderr: &str) -> Option<String> {
    if code != Some(EXIT_CANNOT_INVOKE) {
        return None;
    }
    if !stderr.contains(TIER_REFUSAL_HEAD) || !stderr.contains(TIER_REFUSAL_TAIL) {
        return None;
    }
    Some(format!(
        "the no-chroot tier is unavailable: run -d exited {code:?} and the \
         binary under test carries no interposer object, so the loader family \
         declines every dynamic payload on a machine that denies chroot(2). \
         Build the objects first: ./scripts/build-interpose.sh"
    ))
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

/// The image `run -d` starts. 340 defaults to
/// `ghcr.io/pkgforge-dev/archlinux:latest` with a `PODBOX_RUN_IMAGE`
/// override; this file keeps both spellings.
fn run_image() -> String {
    std::env::var("PODBOX_RUN_IMAGE")
        .unwrap_or_else(|_| "ghcr.io/pkgforge-dev/archlinux:latest".to_string())
}

/// None where a live detached start can run, else the SKIP reason.
/// Container entry needs Linux; the pull gate below answers for the
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

/// 340-detached-stdio.sh, hermetic shape: `run --help` prints the usage the
/// live test relies on, including the detach flag, and exits 0.
#[test]
fn run_help_names_the_detach_flag() {
    // The guarantee this pins: the usage text the caller reads before any
    // detached start documents `-d`/`--detach`.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("help");
    let out = run_podbox(&["run", "--help"], &store, Duration::from_secs(60));
    assert_eq!(
        out.status.code(),
        Some(0),
        "run --help exited {:?}, stderr was {:?}",
        out.status.code(),
        stderr_text(&out, &["run --help"])
    );
    let stdout = stdout_text(&out, &["run --help"]);
    assert!(
        stdout.contains("usage: podbox run"),
        "run --help printed no usage line: {stdout:?}"
    );
    assert!(
        stdout.contains("-d, --detach"),
        "run --help names no detach flag: {stdout:?}"
    );
    cleanup(&store);
}

/// 340-detached-stdio.sh, hermetic shape: `run -d` with no image is a CLI
/// error (exit 1), not a flag error and not a start. The parser refuses a
/// missing image after the flags parse (crates/podbox-cli/src/run.rs).
#[test]
fn detached_run_without_an_image_is_a_cli_error() {
    // The guarantee this pins: a `run -d` that names no image never starts
    // anything, and answers 1 the way `docker run` with no image does.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("noimage");
    let out = run_podbox(
        &["run", "-d", "--name", "det-noimage"],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "run -d with no image exited {:?}, stderr was {:?}",
        out.status.code(),
        stderr_text(&out, &["run -d"])
    );
    let stderr = stderr_text(&out, &["run -d"]);
    assert!(
        stderr.contains("usage: podbox run"),
        "run -d with no image printed no usage: {stderr:?}"
    );
    cleanup(&store);
}

/// 340-detached-stdio.sh, prompt return and running record: `run -d`
/// hands back the id inside PROMPT_SECONDS while the caller reads stdout
/// to EOF, and one `inspect` then reads `running` with a launcher pid.
/// `Command::output` reads the pipe to EOF exactly like the script's
/// `id="$(...)"`, so a launcher holding stdout blocks it the same way.
#[test]
fn detached_start_returns_before_the_payload_ends() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("detached");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP detached start: {reason}");
        cleanup(&store);
        return;
    }
    let image = run_image();
    // 340 pulls and extracts ONCE, outside the loop, so the loop measures
    // the handoff and never the network.
    let pull = run_podbox(&["pull", &image], &store, Duration::from_secs(900));
    if !pull.status.success() {
        eprintln!(
            "SKIP detached start: could not pull {image}: {:?}",
            stderr_text(&pull, &["pull"]).trim()
        );
        cleanup(&store);
        return;
    }
    let extract = run_podbox(&["extract", &image], &store, Duration::from_secs(900));
    if !extract.status.success() {
        eprintln!(
            "SKIP detached start: could not extract {image}: {:?}",
            stderr_text(&extract, &["extract"]).trim()
        );
        cleanup(&store);
        return;
    }

    struct Start {
        name: String,
        code: Option<i32>,
        took: Duration,
        id: String,
        inspect_code: Option<i32>,
        row: String,
        stderr: String,
    }
    let mut starts: Vec<Start> = Vec::new();
    for i in 1..=ITERATIONS {
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let name = format!("det{i}-{}-{n}", std::process::id());
        let before = Instant::now();
        let out = run_podbox(
            &[
                "run",
                "-d",
                "--name",
                &name,
                &image,
                "/bin/sleep",
                PAYLOAD_SECONDS,
            ],
            &store,
            Duration::from_secs(120),
        );
        let took = before.elapsed();
        let id = stdout_text(&out, &["run -d"]).trim().to_string();
        // 340: ONE inspect, four fields, so all four describe the same
        // read of the table rather than four reads a scheduler could
        // interleave.
        let inspect = run_podbox(
            &[
                "inspect",
                "--format",
                "{{.State}} {{.Pid}} {{.LauncherPid}} {{.ExitCode}}",
                &name,
            ],
            &store,
            Duration::from_secs(60),
        );
        let row = stdout_text(&inspect, &["inspect"]).trim().to_string();
        // Whatever happened, nothing is left running: a sleep per
        // iteration outlives this test otherwise.
        let _ = run_podbox(&["rm", "-f", &name], &store, Duration::from_secs(120));
        let code = out.status.code();
        // ⭐ T-1604. The stderr of the start is KEPT, because the loop below
        // breaks on the first non-zero exit and the break used to throw this
        // away: the suite could then only say "only 1 of 3 detached starts ran"
        // and never name which exit the second one took. A failure that cannot
        // say what it saw is a failure the next session has to re-derive.
        let stderr = stderr_text(&out, &["run -d"]);
        starts.push(Start {
            name,
            code,
            took,
            id,
            inspect_code: inspect.status.code(),
            row,
            stderr,
        });
        if code != Some(0) {
            break;
        }
    }
    // 340's sweep: whatever happened, remove what `ps -aq` still lists.
    let ps = run_podbox(&["ps", "-aq"], &store, Duration::from_secs(60));
    for id in stdout_text(&ps, &["ps -aq"])
        .split_whitespace()
        .collect::<Vec<_>>()
    {
        let _ = run_podbox(&["rm", "-f", id], &store, Duration::from_secs(120));
    }

    // ⭐ T-1604. Where the tier is legitimately unavailable, the assertion
    // below would otherwise report a product failure for a refusal the
    // product is right to make. The refusal is read as well as the exit, so a
    // 126 from any other cause still fails the test.
    //
    // ⚠ EVERY start is checked, not the first. The loop above breaks on the
    // first non-zero, so a tree where the first start declines and a later one
    // would have run must not skip on the strength of the first alone.
    if let Some(reason) = starts
        .iter()
        .find_map(|s| tier_skip_reason(s.code, &s.stderr))
    {
        eprintln!(
            "SKIP detached start: {reason}\n{}",
            starts
                .iter()
                .enumerate()
                .map(|(n, s)| format!("  start {} exited {:?}", n + 1, s.code))
                .collect::<Vec<_>>()
                .join("\n")
        );
        cleanup(&store);
        return;
    }

    // ⭐ T-1604. The loop breaks on the first non-zero `run -d`, so this count is
    // the assertion that fires when a start fails, and it must carry WHICH start
    // and WHAT it said. The stderr below is what names the exit.
    let ran: Vec<String> = starts
        .iter()
        .enumerate()
        .map(|(n, s)| {
            format!(
                "  start {} (`run -d` for {}) exited {:?} after {:?}, id {:?}, \
                 inspect exited {:?} reading [{}]\n    run -d stderr: {}",
                n + 1,
                s.name,
                s.code,
                s.took.as_secs(),
                s.id,
                s.inspect_code,
                s.row,
                if s.stderr.trim().is_empty() {
                    "(empty)".to_string()
                } else {
                    s.stderr.trim().to_string()
                }
            )
        })
        .collect();
    assert_eq!(
        starts.len(),
        ITERATIONS as usize,
        "only {} of {ITERATIONS} detached starts ran:\n{}",
        starts.len(),
        ran.join("\n")
    );
    for (n, s) in starts.iter().enumerate() {
        assert_eq!(
            s.code,
            Some(0),
            "start {} (`run -d` for {}) exited {:?} with id {:?}",
            n + 1,
            s.name,
            s.code,
            s.id
        );
        assert!(
            !s.id.is_empty(),
            "start {} (`run -d` for {}) printed no id",
            n + 1,
            s.name
        );
        assert!(
            !s.id.chars().any(char::is_whitespace),
            "start {} printed more than an id: {:?}",
            n + 1,
            s.id
        );
        assert!(
            s.took.as_secs() < PROMPT_SECONDS,
            "start {} REPRODUCED 340: run -d took {}s to hand back an id \
             for a payload that runs {PAYLOAD_SECONDS}s; it did not detach",
            n + 1,
            s.took.as_secs()
        );
        assert_eq!(
            s.inspect_code,
            Some(0),
            "start {}: inspect of {} exited {:?}, row was {:?}",
            n + 1,
            s.name,
            s.inspect_code,
            s.row
        );
        let mut fields = s.row.split_whitespace();
        let state = fields.next().unwrap_or("");
        let launcher = fields.nth(1).unwrap_or("");
        assert_eq!(
            state,
            "running",
            "start {}: [{}] for a payload that runs {PAYLOAD_SECONDS}s",
            n + 1,
            s.row
        );
        assert!(
            !launcher.is_empty() && launcher != "0",
            "start {}: state running and NO launcher pid: [{}]",
            n + 1,
            s.row
        );
    }
    cleanup(&store);
}
