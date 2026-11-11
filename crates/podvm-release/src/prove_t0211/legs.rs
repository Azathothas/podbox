use super::*;

pub(super) fn reproducible(root: &Path) -> i32 {
    let out = root.join("experiments/results/reproducible-build.txt");
    let Some(work) = Guard::temp("podvm-repro") else {
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
BIN=target/x86_64-unknown-linux-musl/release/podvm
[ -x "$BIN" ] || exit 2
mkdir -p /out || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha1
"$BIN" version --verbose > /out/verbose1 || exit 2
cp "$BIN" /out/podvm1 || exit 2
cargo clean --target x86_64-unknown-linux-musl -p podvm || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha2
"$BIN" version --verbose > /out/verbose2 || exit 2
cp "$BIN" /out/podvm2 || exit 2
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
        &[("PODVM_ARTIFACTS", &artifacts.to_string_lossy())],
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
    for name in ["sha1", "sha2", "verbose1", "verbose2", "podvm1", "podvm2"] {
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
    let rc = if fs::read(artifacts.join("podvm1")).unwrap_or_default()
        == fs::read(artifacts.join("podvm2")).unwrap_or_default()
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

fn one(root: &Path, test: &str, log: &Path, patch: &[String]) -> Option<i32> {
    // Read the exit code from the process that produced it, without a pipe.
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new("cargo");
    command
        .args(["test", "-p", "podvm-image"])
        .args(patch)
        .arg(test)
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

pub(super) fn lock_inheritance(root: &Path) -> i32 {
    let out = root.join("experiments/results/lock-inheritance-prove.txt");
    let runs: u32 = std::env::var("PODVM_PROVE_RUNS")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    if !command_exists("cargo") {
        eprintln!("SKIP: no cargo on PATH.");
        return 2;
    }
    if !root.join("crates/podvm-image/src/store.rs").is_file() {
        eprintln!("SKIP: this is not the podvm tree.");
        return 2;
    }
    // Refuse to start on a dirty lock file: the patched builds can change
    // it, and restoring over an uncommitted change would restore a state
    // nobody asked for.
    let clean = run_status("git", &["diff", "--quiet", "--", LOCKFILE], root, &[], 60)
        == Some(true)
        && run_status(
            "git",
            &["diff", "--cached", "--quiet", "--", LOCKFILE],
            root,
            &[],
            60,
        ) == Some(true);
    if !clean {
        eprintln!("SKIP: {LOCKFILE} has uncommitted changes. This proof must own it.");
        return 2;
    }
    let Some(work) = Guard::temp("podvm-prove") else {
        return 2;
    };
    // The mutations go into a copy of the podcrates checkout, never into
    // cargo's own checkout (`prove_t0211/plant.rs`).
    let Some((copy, patch)) = podcrates_copy(root, &work.dir) else {
        eprintln!("SKIP: no copy of the podcrates checkout of pod-supervise.");
        return 2;
    };
    let subject = copy.join(SUBJECT);
    let lockfile = root.join(LOCKFILE);
    let backup = work.dir.join("lock.rs");
    let lock_backup = work.dir.join("Cargo.lock");
    if fs::copy(&subject, &backup).is_err() || fs::copy(&lockfile, &lock_backup).is_err() {
        return 2;
    }
    let restore = || {
        fs::copy(&backup, &subject).ok();
        fs::copy(&lock_backup, &lockfile).ok();
    };
    // Guard the restore on every exit path.
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
            match one(root, test, &work.dir.join("one.log"), &[]) {
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
            let red = one(root, must_red, &work.dir.join("one.log"), &patch);
            let red_summaries = result_lines(&work.dir.join("one.log"));
            let green = one(root, must_green, &work.dir.join("one.log"), &patch);
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

    // The fork defence is the guard in `Lock::try_acquire_noting`. Deleting the
    // call would leave an unbalanced block, so the call is turned into one
    // that registers nothing and still returns true.
    mutate(
        "clause 2  the fork defence removed from Lock::try_acquire_noting",
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
    match run_status("git", &["diff", "--quiet", "--", LOCKFILE], root, &[], 60) {
        Some(true) => say(&format!("  {LOCKFILE} is back as it was")),
        _ => {
            say(&format!("  {LOCKFILE} IS NOT BACK AS IT WAS"));
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
