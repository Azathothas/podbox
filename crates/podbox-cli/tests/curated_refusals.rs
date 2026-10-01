//! Black-box refusal surface for `podbox run`, `podbox create`, and the
//! unimplemented verbs, driven through the built binary.
//!
//! Script: experiments/355-parity-curated.sh. This file covers the refusal
//! half: clause 1 (the 40 refused `run` flags), clause 2 (the 10 refused
//! verbs), clause 4 (`create` inherits the surface through `run`'s parser),
//! clause 5 (malformed `--device` shapes), and the flag-error halves of
//! clause 3 (`--env-file` and `--log-driver` refusals). Every check here
//! refuses at parse time, before any image work, so no registry and no entry
//! are needed; the image positional is a dummy name the parser never reaches.
//!
//! Clause 3's end to end behavior is NOT driven here: `--env-file` values
//! through a payload, the stub trio running, `--log-driver json-file`
//! acceptance, and `--strict` against stubs all need a pulled image and an
//! entry. That half needs a registry and namespaces, which a black-box parse
//! test cannot arrange.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// File-local serialization. `PODBOX_STORE` is set per child via `Command::env`
/// (never via process-global mutation), but the temp counter and the temp tree
/// stay unique and ordered behind this one lock, as the house style requires.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Docker's code for a flag error, which podbox shares: T-0802 measured
/// docker and `podbox_probe::exit` carries the table. A runtime refusal is
/// docker's 125 too, so both refusal shapes below assert the same number and
/// tell each other apart by their stderr sentences instead.
const EXIT_FLAG_ERROR: i32 = 125;
const EXIT_RUNTIME_ERROR: i32 = 125;

/// How long one binary invocation may run before the runner kills it.
/// Refusals answer at parse time; anything slower is wedged, not working.
const DEADLINE: Duration = Duration::from_secs(60);

/// The image positional for refusal drives. The parser refuses the flag
/// before the image is looked at, so this name is never resolved.
const DUMMY_IMAGE: &str = "refusal-probe.invalid/no-such-image:latest";

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_podbox"))
}

fn fresh_store(tag: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-cli-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the per-test store directory");
    dir
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Run the binary with a unique store, bounded. A child that outlives its
/// deadline is killed and fails the test by name instead of wedging the suite.
fn run(args: &[&str], store: &Path) -> Output {
    let mut child = Command::new(bin())
        .args(args)
        .env("PODBOX_STORE", store)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the podbox binary");
    let start = Instant::now();
    loop {
        match child.try_wait().expect("poll the podbox child") {
            Some(_) => break,
            None => {
                if start.elapsed() >= DEADLINE {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("podbox {args:?} outlived the {DEADLINE:?} deadline");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    child.wait_with_output().expect("collect the podbox output")
}

/// Clause 1: the 40 refused flags. Each `run --rm <flag>` exits with the
/// flag-error code and names status None, before any image work happens.
/// `--log-driver` is not among them (it takes `json-file`) and `--device`
/// is not either (it maps; clause 5 covers its malformed shapes).
#[test]
fn refused_run_flags_exit_125_naming_status_none() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("refused-flags");
    #[rustfmt::skip]
    let flags = [
        "--blkio-weight", "--cgroup-parent", "--cidfile", "--cpu-period",
        "--cpu-quota", "--cpu-shares", "--cpuset-cpus", "--detach-keys",
        "--device-cgroup-rule", "--disable-content-trust", "--dns", "--dns-option",
        "--dns-search", "--domainname", "--gpus", "--group-add", "--health-cmd", "--init",
        "--ipc", "--isolation", "--link", "--log-opt", "--mac-address",
        "--mount", "--oom-kill-disable", "--pid", "--pids-limit", "--read-only", "--runtime",
        "--security-opt", "--shm-size", "--stop-signal", "--stop-timeout", "--sysctl",
        "--tmpfs", "--ulimit", "--userns", "--uts", "--volume-driver", "--volumes-from",
    ];
    assert_eq!(
        flags.len(),
        40,
        "the script refuses exactly 40 flags, not {}",
        flags.len()
    );
    let mut bad = Vec::new();
    for f in flags {
        let out = run(&["run", "--rm", f, DUMMY_IMAGE, "true"], &store);
        let err = text(&out.stderr);
        if out.status.code() != Some(EXIT_FLAG_ERROR) || !err.contains("status None") {
            bad.push(format!("{f}: exit {:?} stderr {err:?}", out.status.code()));
        }
    }
    assert!(
        bad.is_empty(),
        "refused flags that did not answer 125 with status None:\n{}",
        bad.join("\n")
    );
    let _ = std::fs::remove_dir_all(&store);
}

/// Clause 2: the 10 verbs. Each `podbox <verb>` exits with the runtime-error
/// code and prints its parity row's note, which starts `podbox: <verb>: `.
#[test]
fn unimplemented_verbs_exit_125_with_their_note() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("verbs");
    let verbs = [
        "manifest",
        "node",
        "plugin",
        "scan",
        "secret",
        "service",
        "stack",
        "trust",
        "checkpoint",
        "config",
    ];
    let mut bad = Vec::new();
    for v in verbs {
        let out = run(&[v], &store);
        let err = text(&out.stderr);
        if out.status.code() != Some(EXIT_RUNTIME_ERROR) || !err.contains(&format!("podbox: {v}: "))
        {
            bad.push(format!("{v}: exit {:?} stderr {err:?}", out.status.code()));
        }
    }
    assert!(
        bad.is_empty(),
        "verbs that did not answer 125 with their note:\n{}",
        bad.join("\n")
    );
    let _ = std::fs::remove_dir_all(&store);
}

/// Clause 4: `create` is served by `run`'s parser, so it inherits the rows.
/// A refused flag under `create` answers the same flag-error code with the
/// same status None sentence, naming the verb the caller typed.
#[test]
fn create_inherits_run_refusals() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("create");
    let out = run(
        &[
            "create",
            "--name",
            "c-refused",
            "--read-only",
            DUMMY_IMAGE,
            "true",
        ],
        &store,
    );
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_FLAG_ERROR),
        "create --read-only exited {:?}, stderr {err:?}",
        out.status.code()
    );
    assert!(
        err.contains("status None"),
        "create --read-only did not name status None: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&store);
}

/// Clause 5: `--device` maps, so it is not refused; but a malformed shape is
/// a flag error naming the shape, before any image work happens.
#[test]
fn device_shape_refusals_name_the_shape() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("device");
    let out = run(
        &[
            "run",
            "--rm",
            "--device=/dev/zero:/a:/b:c",
            DUMMY_IMAGE,
            "true",
        ],
        &store,
    );
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_FLAG_ERROR),
        "three-colon --device exited {:?}, stderr {err:?}",
        out.status.code()
    );
    assert!(
        err.contains("more than two colons"),
        "three-colon --device did not name the shape: {err:?}"
    );
    let out = run(
        &[
            "run",
            "--rm",
            "--device=/dev/zero:/g:rx",
            DUMMY_IMAGE,
            "true",
        ],
        &store,
    );
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_FLAG_ERROR),
        "bad-perms --device exited {:?}, stderr {err:?}",
        out.status.code()
    );
    assert!(
        err.contains("perms take r, w and m only"),
        "bad-perms --device did not name the perms: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&store);
}

/// Clause 3, flag-error half: `--log-driver` takes `json-file` and refuses
/// any other driver naming it. Both the space and the `=` spelling refuse.
#[test]
fn log_driver_refuses_anything_but_json_file() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("log-driver");
    for args in [
        vec!["run", "--rm", "--log-driver", "syslog", DUMMY_IMAGE, "true"],
        vec!["run", "--rm", "--log-driver=syslog", DUMMY_IMAGE, "true"],
    ] {
        let out = run(&args, &store);
        let err = text(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(EXIT_FLAG_ERROR),
            "{args:?} exited {:?}, stderr {err:?}",
            out.status.code()
        );
        assert!(
            err.contains("takes json-file"),
            "{args:?} did not name the accepted value: {err:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&store);
}

/// Clause 3, flag-error half: a line without `=` and a missing file are flag
/// errors, never silent env. Both refuse at parse time, before any fetch.
#[test]
fn env_file_refusals_name_the_file() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let store = fresh_store("env-file");
    let dir = store.join("env");
    std::fs::create_dir_all(&dir).expect("create the env-file directory");
    let bad = dir.join("bad.env");
    std::fs::write(&bad, "FOO=from-file\nNOEQUALS\n").expect("write the bad env file");
    let bad_arg = format!("--env-file={}", bad.display());
    let out = run(&["run", "--rm", &bad_arg, DUMMY_IMAGE, "true"], &store);
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_FLAG_ERROR),
        "bad env-file exited {:?}, stderr {err:?}",
        out.status.code()
    );
    assert!(
        err.contains("has no `=`"),
        "bad env-file did not name the line fault: {err:?}"
    );
    let missing = dir.join("absent.env");
    let missing_arg = format!("--env-file={}", missing.display());
    let out = run(&["run", "--rm", &missing_arg, DUMMY_IMAGE, "true"], &store);
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_FLAG_ERROR),
        "missing env-file exited {:?}, stderr {err:?}",
        out.status.code()
    );
    assert!(
        err.contains(&missing.to_string_lossy().into_owned()),
        "missing env-file did not name the path: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&store);
}
