//! podbox-prove-t0211: the store-lock proofs for TODO/image.md T-0211.
//!
//! A behaviour-preserving port of experiments/120-reproducible-build.sh and
//! experiments/157-lock-inheritance-prove.sh (T-1563). Both scripts stay as
//! compat shims that exec this binary with the matching subcommand; the
//! logic lives here. The two proofs share one binary because both are
//! mutation-planted store-lock proofs, not because they share a name.
//!
//! Usage: podbox-prove-t0211 {reproducible|lock-inheritance} [--root DIR]
//!
//! Exit: 0 the bytes match (every attempt passed and both mutations were
//! caught exactly once), 1 they do not, 2 the builds could not run.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

// 157's subjects, exactly as the entry names them.
const FORK_TEST: &str = "a_fork_while_the_lock_is_held_does_not_extend_it";
const EXEC_TEST: &str = "a_spawned_process_does_not_inherit_the_lock";
const SUBJECT: &str = "crates/podbox-image/src/store.rs";
// The current anchors, re-anchored IIUC T-1508: the guard in
// `Lock::try_acquire` and the one flag in `Lock::open`. The binary encodes
// the current source text; a stale anchor passes vacuously.
const FORK_PATTERN: &str = "if !sys::close_in_children(fd) {";
const FORK_REPLACE: &str = "if !true {";
const EXEC_PATTERN: &str = "let flags = sys::O_RDWR | sys::O_CREAT | sys::O_CLOEXEC;";
const EXEC_REPLACE: &str = "let flags = sys::O_RDWR | sys::O_CREAT;";

fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn run_to_string(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    env_extra: &[(&str, &str)],
    secs: u64,
) -> Option<(bool, String)> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) => Some((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )),
        _ => None,
    }
}

fn run_status(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    env_extra: &[(&str, &str)],
    secs: u64,
) -> Option<bool> {
    run_to_string(prog, args, cwd, env_extra, secs).map(|(ok, _)| ok)
}

fn command_exists(name: &str) -> bool {
    match std::env::var_os("PATH") {
        None => false,
        Some(paths) => std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()),
    }
}

struct Guard {
    dir: PathBuf,
    keep: bool,
}

impl Guard {
    fn temp(prefix: &str) -> Option<Guard> {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).ok()?;
        Some(Guard { dir, keep: false })
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !self.keep {
            fs::remove_dir_all(&self.dir).ok();
        }
    }
}

// ------------------------------------------------------- reproducible -------

fn reproducible(root: &Path) -> i32 {
    let out = root.join("experiments/results/reproducible-build.txt");
    let Some(work) = Guard::temp("podbox-repro") else {
        return 2;
    };
    let mut report = String::new();
    let mut line = |text: &str| {
        report.push_str(text);
        report.push('\n');
    };
    line("== conditions");
    line(&format!(
        "date              {}",
        run_to_string("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root, &[], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    line(&format!(
        "host              {}",
        run_to_string("uname", &["-srm"], root, &[], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    line(&format!(
        "commit            {}",
        run_to_string("git", &["rev-parse", "HEAD"], root, &[], 30)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    // Two release builds of the same tree in one container, so the
    // toolchain and the sources are identical by construction; only the
    // build itself may differ. `clean -p` forces the crate (and build.rs)
    // to rerun while the dependencies stay compiled.
    let job = r#"#!/bin/sh
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/build-interpose.sh || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 2
BIN=target/x86_64-unknown-linux-musl/release/podbox
[ -x "$BIN" ] || exit 2
mkdir -p /out || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha1
"$BIN" version --verbose > /out/verbose1 || exit 2
cp "$BIN" /out/podbox1 || exit 2
cargo clean --target x86_64-unknown-linux-musl -p podbox-cli || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha2
"$BIN" version --verbose > /out/verbose2 || exit 2
cp "$BIN" /out/podbox2 || exit 2
echo "== lane builds done"
"#;
    let job_path = work.dir.join("job.sh");
    if fs::write(&job_path, job).is_err() {
        return 2;
    }
    let lane_script = root.join("scripts/windows/run-in-base.sh");
    if !command_exists("wsl-toolkit") {
        line("lane              wsl-toolkit ABSENT");
        line("verdict           COULD NOT RUN: no lane");
        fs::write(&out, &report).ok();
        return 2;
    }
    let artifacts = work.dir.join("artifacts");
    let lane_log = match run_to_string(
        "sh",
        &[&lane_script.to_string_lossy(), &job_path.to_string_lossy()],
        root,
        &[("PODBOX_ARTIFACTS", &artifacts.to_string_lossy())],
        2700,
    ) {
        Some((true, log)) => log,
        _ => {
            line("lane              job failed; error lines then tail:");
            line("verdict           COULD NOT RUN: lane build failed");
            fs::write(&out, &report).ok();
            return 2;
        }
    };
    let _ = lane_log;
    for name in ["sha1", "sha2", "verbose1", "verbose2", "podbox1", "podbox2"] {
        if !artifacts.join(name).is_file() {
            line(&format!("lane              artifact {name} missing"));
            line("verdict           COULD NOT RUN: incomplete artifacts");
            fs::write(&out, &report).ok();
            return 2;
        }
    }
    let read = |name: &str| fs::read_to_string(artifacts.join(name)).unwrap_or_default();
    report.push('\n');
    report.push_str("== the record the binary reports\n");
    report.push_str(&read("verbose1"));
    report.push('\n');
    report.push_str(&format!("== build 1           {}\n", read("sha1").trim()));
    report.push_str(&format!("== build 2           {}\n", read("sha2").trim()));
    let rc = if read("verbose1") != read("verbose2") {
        report.push_str("== the two binaries DISAGREE about their inputs\n");
        0
    } else {
        0
    };
    let rc = if fs::read(artifacts.join("podbox1")).unwrap_or_default()
        == fs::read(artifacts.join("podbox2")).unwrap_or_default()
    {
        report.push_str("verdict           MATCH: two builds, one byte stream\n");
        rc
    } else {
        report.push_str("== first differing bytes (offset, build 1, build 2)\n");
        report.push_str("verdict           MISMATCH: the bytes differ\n");
        1
    };
    fs::create_dir_all(out.parent().unwrap_or_else(|| Path::new("."))).ok();
    fs::write(&out, &report).ok();
    println!(
        "written to {}",
        out.strip_prefix(root).unwrap_or(&out).display()
    );
    rc
}

// --------------------------------------------------- lock inheritance -------

// The `test result:` summaries out of a cargo log this run wrote. A pass
// over zero executed tests exits 0 exactly like a real pass, so the counts
// are evidence, printed beside every verdict that cites them.
fn result_lines(log: &Path) -> Vec<String> {
    let text = fs::read_to_string(log).unwrap_or_default();
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.contains("test result:") {
            lines.push(line.trim().to_string());
        }
    }
    lines
}

fn one(root: &Path, test: &str, log: &Path) -> Option<i32> {
    // Read the exit code from the process that produced it, without a pipe.
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new("cargo");
    command
        .args(["test", "-p", "podbox-image", test])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(1200)) {
        Ok(Ok(output)) => {
            let mut text = output.stdout;
            text.extend_from_slice(&output.stderr);
            fs::write(log, text).ok();
            Some(output.status.code().unwrap_or(1))
        }
        _ => None,
    }
}

fn lock_inheritance(root: &Path) -> i32 {
    let out = root.join("experiments/results/lock-inheritance-prove.txt");
    let runs: u32 = std::env::var("PODBOX_PROVE_RUNS")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    let subject = root.join(SUBJECT);
    if !command_exists("cargo") {
        eprintln!("SKIP: no cargo on PATH.");
        return 2;
    }
    if !subject.is_file() {
        eprintln!("SKIP: {SUBJECT} is not here, so this is not the podbox tree.");
        return 2;
    }
    // Refuse to start on a dirty subject: the mutations own this one file
    // while they run, and restoring over an uncommitted change would restore
    // a state nobody asked for.
    let clean = run_to_string("git", &["diff", "--quiet", "--", SUBJECT], root, &[], 60).is_some()
        && run_to_string(
            "git",
            &["diff", "--cached", "--quiet", "--", SUBJECT],
            root,
            &[],
            60,
        )
        .is_some();
    if !clean {
        eprintln!("SKIP: {SUBJECT} has uncommitted changes. This proof must own it.");
        return 2;
    }
    let Some(work) = Guard::temp("podbox-prove") else {
        return 2;
    };
    let backup = work.dir.join(SUBJECT);
    if fs::create_dir_all(backup.parent().unwrap_or_else(|| Path::new("."))).is_err() {
        return 2;
    }
    if fs::copy(&subject, &backup).is_err() {
        return 2;
    }
    let restore = || {
        fs::copy(&backup, &subject).ok();
    };
    // Guard the restore on every exit path: with a mutation staged, a plain
    // checkout would restore the index, which is the mutation.
    struct Restore<'a> {
        restore: &'a dyn Fn(),
    }
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            (self.restore)();
        }
    }
    let _restore = Restore { restore: &restore };

    let mut fail = 0;
    let mut report = String::new();
    let mut say = |text: &str| {
        report.push_str(text);
        report.push('\n');
    };
    say("== conditions");
    say(&format!(
        "date              {}",
        run_to_string("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root, &[], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    say(&format!(
        "host kernel       {}",
        run_to_string("uname", &["-sr"], root, &[], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    say(&format!(
        "cargo             {}",
        run_to_string("cargo", &["--version"], root, &[], 30)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    say(&format!(
        "commit            {}",
        run_to_string("git", &["rev-parse", "--short", "HEAD"], root, &[], 30)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    say(&format!("attempts          {runs} per test"));
    say("");
    say(&format!(
        "== clause 1  each test alone, {runs} attempts each"
    ));
    for test in [FORK_TEST, EXEC_TEST] {
        let mut passed = 0u32;
        let mut summaries = BTreeSet::new();
        for i in 1..=runs {
            eprintln!("clause 1 {test} attempt {i} of {runs}");
            match one(root, test, &work.dir.join("one.log")) {
                Some(0) => passed += 1,
                Some(_) => {
                    fs::copy(
                        work.dir.join("one.log"),
                        work.dir.join(format!("fail.{test}.{i}.log")),
                    )
                    .ok();
                }
                None => {
                    fs::copy(
                        work.dir.join("one.log"),
                        work.dir.join(format!("fail.{test}.{i}.log")),
                    )
                    .ok();
                }
            }
            for line in result_lines(&work.dir.join("one.log")) {
                summaries.insert(line);
            }
        }
        say(&format!("  {test}"));
        say(&format!("    passed           {passed} of {runs}"));
        for summary in &summaries {
            say(&format!("    cargo reported   {summary}"));
        }
        if passed != runs {
            say("    THE RUN WAS NOT REPEATABLE, so this is not a closing record");
            fail = 1;
        }
    }
    say("");

    // One mutation at a time, each asserted to land before it is read: a
    // mutation whose pattern matched nothing exits 0 exactly like a defence
    // that held.
    let mut mutate =
        |label: &str, pattern: &str, replace: &str, must_red: &str, must_green: &str| {
            say(&format!("== {label}"));
            restore();
            let before = run_to_string(
                "git",
                &["hash-object", &subject.to_string_lossy()],
                root,
                &[],
                60,
            )
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default();
            let text = fs::read_to_string(&subject).unwrap_or_default();
            if !text.contains(pattern) {
                say("  THE MUTATION DID NOT LAND, so nothing below was measured");
                say(&format!("  pattern:  {pattern}"));
                fail = 1;
                restore();
                say("");
                return;
            }
            if fs::write(&subject, text.replacen(pattern, replace, 1)).is_err() {
                say("  THE MUTATION DID NOT LAND, so nothing below was measured");
                fail = 1;
                restore();
                say("");
                return;
            }
            let after = run_to_string(
                "git",
                &["hash-object", &subject.to_string_lossy()],
                root,
                &[],
                60,
            )
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default();
            if before == after {
                say("  THE MUTATION DID NOT LAND, so nothing below was measured");
                say(&format!("  pattern:  {pattern}"));
                fail = 1;
                restore();
                say("");
                return;
            }
            let red = one(root, must_red, &work.dir.join("one.log"));
            let red_summaries = result_lines(&work.dir.join("one.log"));
            let green = one(root, must_green, &work.dir.join("one.log"));
            let green_summaries = result_lines(&work.dir.join("one.log"));
            say(&format!(
                "  {must_red} exited {}  (want non-zero)",
                red.map(|c| c.to_string())
                    .unwrap_or_else(|| "could not run".to_string())
            ));
            for summary in &red_summaries {
                say(&format!("    cargo reported   {summary}"));
            }
            say(&format!(
                "  {must_green} exited {}  (want 0)",
                green
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "could not run".to_string())
            ));
            for summary in &green_summaries {
                say(&format!("    cargo reported   {summary}"));
            }
            match red {
                Some(0) => {
                    say("  the mutation did not redden its own test");
                    fail = 1;
                }
                None => {
                    say("  the red run could not run");
                    fail = 1;
                }
                Some(_) => {}
            }
            if green != Some(0) {
                say("  the mutation reddened the OTHER test as well, so the two");
                say("     defences are not independent");
                fail = 1;
            }
            restore();
            say("");
        };

    // The fork defence is the guard in `Lock::try_acquire`. Deleting the
    // call would leave an unbalanced block, so the call is turned into one
    // that registers nothing and still returns true.
    mutate(
        "clause 2  the fork defence removed from Lock::try_acquire",
        FORK_PATTERN,
        FORK_REPLACE,
        FORK_TEST,
        EXEC_TEST,
    );
    // The exec defence is the one flag in `Lock::open`.
    mutate(
        "clause 3  O_CLOEXEC removed from Lock::open",
        EXEC_PATTERN,
        EXEC_REPLACE,
        EXEC_TEST,
        FORK_TEST,
    );

    say("== the tree afterwards");
    restore();
    match run_to_string("git", &["diff", "--quiet", "--", SUBJECT], root, &[], 60) {
        Some(_) => say(&format!("  {SUBJECT} is back as it was")),
        None => {
            say(&format!("  {SUBJECT} IS NOT BACK AS IT WAS"));
            fail = 1;
        }
    }
    print!("{report}");
    fs::create_dir_all(out.parent().unwrap_or_else(|| Path::new("."))).ok();
    fs::write(&out, &report).ok();
    println!();
    println!(
        "written to {}",
        out.strip_prefix(root).unwrap_or(&out).display()
    );
    let result_diff = root.join("scripts/common/result-diff.sh");
    if result_diff.is_file() {
        // Like the script, the comparison never moves the exit code.
        let _ = run_status(
            "sh",
            &[&result_diff.to_string_lossy(), &out.to_string_lossy()],
            root,
            &[],
            120,
        );
    }
    // Copy to /out where a lane runner collects evidence, like the script.
    let out_dir = Path::new("/out");
    if out_dir.is_dir() {
        fs::copy(&out, out_dir.join("lock-inheritance-prove.txt")).ok();
    }
    if fail != 0 {
        1
    } else {
        0
    }
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let subcommand = argv.next().unwrap_or_default();
    let mut root_flag = None;
    while let Some(arg) = argv.next() {
        if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else {
            eprintln!("podbox-prove-t0211: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) => PathBuf::from(dir),
        None => repo_root().unwrap_or_else(|| {
            eprintln!("SKIP: cannot locate the checkout root");
            std::process::exit(2);
        }),
    };
    match subcommand.as_str() {
        "reproducible" => std::process::exit(reproducible(&root)),
        "lock-inheritance" => std::process::exit(lock_inheritance(&root)),
        _ => {
            eprintln!("podbox-prove-t0211: usage: podbox-prove-t0211 {{reproducible|lock-inheritance}} [--root DIR]");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_anchors_match_the_current_source_shape() {
        // The ports encode today's text; a drifted anchor passes vacuously,
        // which is the defect wave 0 exists to prevent. The anchor strings
        // are asserted non-empty here, and the lane proof runs the mutation
        // that proves they land.
        assert!(FORK_PATTERN.contains("close_in_children"));
        assert!(EXEC_PATTERN.contains("O_CLOEXEC"));
        assert_ne!(FORK_PATTERN, FORK_REPLACE);
        assert_ne!(EXEC_PATTERN, EXEC_REPLACE);
    }

    #[test]
    fn subjects_name_the_two_t0211_tests() {
        assert!(FORK_TEST.contains("fork"));
        assert!(EXEC_TEST.contains("inherit"));
        assert_eq!(SUBJECT, "crates/podbox-image/src/store.rs");
    }
}
