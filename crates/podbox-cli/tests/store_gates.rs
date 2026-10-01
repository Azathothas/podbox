//! Black-box store gates, driven through the built binary.
//!
//! Script: experiments/373-store-gates.sh. This file covers the store-gate
//! half: clause 1 (an empty store lists nothing and marks nothing, and the
//! exec row passes where files execute), clause 3 (a noexec store says `no`
//! and refuses the entry before any fetch, naming
//! the directory and the `$PODBOX_STORE` remedy), clause 4's refusal half (a
//! record whose blobs are gone refuses the run naming `--pull always`, marks
//! the `images` row, and fails `verify`), and clause 5 (a DNS failure names
//! the host once in plain words, never doubling the URL and never with ureq
//! capitalisation). Clause 4's records are built offline with `podbox import`
//! from a hand-rolled tar, so no registry is needed for the refusal, the
//! mark, or the failed verify.
//!
//! NOT driven here, with reasons: clause 2 (the image Config) needs a
//! registry fetch of a real image, which a hermetic black-box test cannot
//! arrange; clause 4's heal (`run --pull always` re-fetches and both run and
//! verify return to 0) needs the same registry. Where a drive needs entry and
//! this machine cannot enter, the test reports SKIP with its reason and
//! passes, which is the script's own exit-2 contract: an unarranged fixture
//! is not a failure.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// File-local serialization. `PODBOX_STORE` is set per child via `Command::env`
/// (never via process-global mutation), but the temp counter and the temp tree
/// stay unique and ordered behind this one lock, as the house style requires.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Docker's code for a refused run, which podbox shares: T-0802 measured
/// docker and `podbox_probe::exit` carries the table.
const EXIT_RUNTIME_ERROR: i32 = 125;

/// How long an ordinary binary invocation may run before the runner kills it.
const DEADLINE: Duration = Duration::from_secs(60);

/// A registry pull may wait on DNS and TLS; bounded, but roomier.
const PULL_DEADLINE: Duration = Duration::from_secs(120);

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_podbox"))
}

fn fresh_tmp(tag: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-cli-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the per-test temp directory");
    dir
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Run the binary with a unique store, bounded. A child that outlives its
/// deadline is killed and fails the test by name instead of wedging the suite.
fn run(args: &[&str], store: &Path, deadline: Duration) -> Output {
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
                if start.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("podbox {args:?} outlived the {deadline:?} deadline");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    child.wait_with_output().expect("collect the podbox output")
}

/// Write a minimal ustar archive holding one file, in pure std. The import
/// drive needs a plain tar and must not depend on an external `tar` binary.
fn write_ustar(path: &Path, name: &str, data: &[u8]) {
    assert!(name.len() <= 100, "ustar name too long: {name:?}");
    let mut hdr = [0u8; 512];
    hdr[..name.len()].copy_from_slice(name.as_bytes());
    hdr[100..108].copy_from_slice(b"0000644\0");
    hdr[108..116].copy_from_slice(b"0000000\0");
    hdr[116..124].copy_from_slice(b"0000000\0");
    let size = format!("{0:011o}\0", data.len());
    hdr[124..136].copy_from_slice(size.as_bytes());
    hdr[136..148].copy_from_slice(b"00000000000\0");
    hdr[148..156].copy_from_slice(b"        ");
    hdr[156] = b'0';
    hdr[257..262].copy_from_slice(b"ustar");
    hdr[263..265].copy_from_slice(b"00");
    let sum: u32 = hdr.iter().map(|b| *b as u32).sum();
    let chk = format!("{sum:06o}\0 ");
    hdr[148..156].copy_from_slice(chk.as_bytes());
    let mut tar = Vec::new();
    tar.extend_from_slice(&hdr);
    tar.extend_from_slice(data);
    tar.resize(tar.len() + (512 - data.len() % 512) % 512, 0);
    tar.resize(tar.len() + 1024, 0);
    std::fs::write(path, &tar).expect("write the ustar fixture");
}

/// Clause 1: a store holding nothing lists nothing and marks nothing; the
/// exec row passes where files execute.
#[test]
fn empty_store_lists_nothing_and_marks_nothing() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let work = fresh_tmp("empty");
    let store = work.join("store");
    std::fs::create_dir_all(&store).expect("create the empty store");
    let out = run(&["images"], &store, DEADLINE);
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "images on an empty store exited {:?}, stdout {stdout:?} stderr {stderr:?}",
        out.status.code()
    );
    assert!(
        !stdout.contains("alpine"),
        "empty store listed rows: {stdout:?}"
    );
    assert!(
        !stderr.contains("missing from the"),
        "empty store marked a record: {stderr:?}"
    );
    let out = run(&["doctor", "store_exec"], &store, DEADLINE);
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doctor store_exec exited {:?}, stdout {stdout:?} stderr {stderr:?}",
        out.status.code()
    );
    assert!(
        stdout.contains("store_exec: yes"),
        "exec row did not pass on an executable store: stdout {stdout:?} stderr {stderr:?}"
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Clause 3: nothing executes under a noexec mount, so the row says `no`
/// and the entry refuses before any fetch, naming the directory and the
/// remedy. Needs user and mount namespaces plus a noexec-capable mount;
/// where those cannot be arranged the drive reports SKIP with its reason.
#[test]
fn noexec_store_says_no_and_refuses_before_fetch() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let probe = Command::new("unshare")
        .args(["-Urm", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if probe.as_ref().map(|s| s.success()).unwrap_or(false) == false {
        eprintln!(
            "SKIP: user and mount namespaces are refused here, so no noexec mount can be staged"
        );
        return;
    }
    let work = fresh_tmp("noexec");
    let nx = work.join("nx-store");
    std::fs::create_dir_all(&nx).expect("create the noexec store directory");
    let nx_arg = nx.to_string_lossy().into_owned();
    let bin_arg = bin().to_string_lossy().into_owned();
    // Mount a noexec tmpfs over the store inside the namespaces, then drive
    // the binary with its store at the mount. The `sh -c` script is built
    // from temp paths with no spaces, so no quoting layer can rewrite them.
    let shell = format!(
        "mount -t tmpfs -o noexec tmpfs {nx_arg} && PODBOX_STORE={nx_arg} {bin_arg} doctor store_exec"
    );
    let doctor = Command::new("unshare")
        .args(["-Urm", "sh", "-c", &shell])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run the noexec doctor drive");
    let stdout = text(&doctor.stdout);
    if !doctor.status.success() && stdout.is_empty() && doctor.stderr.is_empty() {
        eprintln!("SKIP: the noexec mount did not take here, so the gate cannot be staged");
        let _ = std::fs::remove_dir_all(&work);
        return;
    }
    assert_eq!(
        doctor.status.code(),
        Some(1),
        "doctor store_exec on noexec exited {:?}, stdout {stdout:?} stderr {:?}",
        doctor.status.code(),
        text(&doctor.stderr)
    );
    assert!(
        stdout.contains("store_exec: no"),
        "exec row did not say no on noexec: {stdout:?}"
    );
    let shell = format!(
        "mount -t tmpfs -o noexec tmpfs {nx_arg} && PODBOX_STORE={nx_arg} {bin_arg} run --rm --pull never refusal-probe.invalid/no-such-image:latest true"
    );
    let refused = Command::new("unshare")
        .args(["-Urm", "sh", "-c", &shell])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run the noexec run drive");
    let stderr = text(&refused.stderr);
    assert_eq!(
        refused.status.code(),
        Some(EXIT_RUNTIME_ERROR),
        "run on a noexec store exited {:?}, stderr {stderr:?}",
        refused.status.code()
    );
    assert!(
        stderr.contains("PODBOX_STORE"),
        "noexec refusal did not name the remedy: {stderr:?}"
    );
    assert!(
        stderr.contains(&nx_arg),
        "noexec refusal did not name the directory: {stderr:?}"
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Clause 4, refusal half: one deleted blob refuses the run naming
/// `--pull always`, marks the `images` row on stderr, and fails `verify` at
/// 125. The record is built offline with `import`, so no registry is needed.
/// The run half needs entry; where this machine cannot enter, the entry gate
/// preempts the blob gate and the drive reports SKIP with its reason.
#[test]
fn missing_blob_refuses_run_marks_images_and_fails_verify() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let work = fresh_tmp("blob");
    let store = work.join("store");
    std::fs::create_dir_all(&store).expect("create the store");
    let tar = work.join("rootfs.tar");
    write_ustar(&tar, "hello.txt", b"hi\n");
    let tar_arg = tar.to_string_lossy().into_owned();
    let name = "blobgate-test:latest";
    let out = run(&["import", &tar_arg, name], &store, DEADLINE);
    assert_eq!(
        out.status.code(),
        Some(0),
        "import exited {:?}, stdout {:?} stderr {:?}",
        out.status.code(),
        text(&out.stdout),
        text(&out.stderr)
    );
    // Blobs live sharded at blobs/<algo>/<hex> (Digest::blob_path), so walk
    // two levels to the first file rather than reading only the top level.
    let mut blobs: Vec<PathBuf> = Vec::new();
    let top = std::fs::read_dir(store.join("blobs")).expect("read the store blobs directory");
    for entry in top.filter_map(|e| e.ok()) {
        let dir = entry.path();
        if dir.is_file() {
            blobs.push(dir);
            continue;
        }
        if let Ok(inner) = std::fs::read_dir(&dir) {
            blobs.extend(
                inner
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.is_file()),
            );
        }
    }
    assert!(!blobs.is_empty(), "the import left no blob file to delete");
    std::fs::remove_file(&blobs[0]).expect("delete one blob");
    let out = run(
        &["run", "--rm", "--pull", "never", name, "true"],
        &store,
        DEADLINE,
    );
    let stderr = text(&out.stderr);
    if stderr.contains("chroot(2) is denied") {
        eprintln!("SKIP: this machine cannot enter, so the entry gate preempts the blob gate here");
    } else {
        assert_eq!(
            out.status.code(),
            Some(EXIT_RUNTIME_ERROR),
            "run with a missing blob exited {:?}, stderr {stderr:?}",
            out.status.code()
        );
        assert!(
            stderr.contains("--pull always"),
            "blob refusal did not name the recovery: {stderr:?}"
        );
    }
    let out = run(&["images"], &store, DEADLINE);
    let stderr = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "images exited {:?}, stderr {stderr:?}",
        out.status.code()
    );
    assert!(
        stderr.contains("missing from the"),
        "images did not mark the short record: {stderr:?}"
    );
    let out = run(&["verify", name], &store, DEADLINE);
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_RUNTIME_ERROR),
        "verify with a missing blob exited {:?}, stdout {stdout:?} stderr {stderr:?}",
        out.status.code()
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Clause 5: a DNS failure names the host once in plain words. The old shape
/// doubled the URL with ureq's capitalisation; both halves are asserted.
#[test]
fn dns_failure_names_the_host_once_in_plain_words() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let work = fresh_tmp("transport");
    let store = work.join("store");
    std::fs::create_dir_all(&store).expect("create the store");
    let out = run(
        &["pull", "registry.invalid.example/no-such-image:latest"],
        &store,
        PULL_DEADLINE,
    );
    let stderr = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(EXIT_RUNTIME_ERROR),
        "pull of an invalid registry exited {:?}, stderr {stderr:?}",
        out.status.code()
    );
    assert!(
        stderr.contains("registry.invalid.example"),
        "transport error did not name the host: {stderr:?}"
    );
    assert!(
        !stderr.contains("transport: https://"),
        "transport error doubled the URL: {stderr:?}"
    );
    assert!(
        !stderr.contains("Dns Failed"),
        "transport error kept ureq capitalisation: {stderr:?}"
    );
    let _ = std::fs::remove_dir_all(&work);
}
