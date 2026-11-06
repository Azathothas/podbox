use super::*;

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

// The script's `running`: a pid file naming a process `kill -0` reaches.
// The check runs the same `kill` program, so a pid the shell would call
// live reads live here too.
fn running(root: &Path) -> Option<String> {
    let pid = read_trim(&pid_path(root))?;
    if pid.is_empty() {
        return None;
    }
    let rc = Command::new("kill")
        .arg("-0")
        .arg(&pid)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if rc.success() {
        Some(pid)
    } else {
        None
    }
}

fn append_line(log: &Path, line: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

fn run_to_log(
    root: &Path,
    log: &Path,
    prog: &str,
    args: &[&str],
    extra_env: &[(String, String)],
) -> i32 {
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(root)
        .envs(extra_env.iter().cloned());
    let output = command.output();
    match output {
        Ok(output) => {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
                let _ = file.write_all(&output.stdout);
                let _ = file.write_all(&output.stderr);
            }
            output.status.code().unwrap_or(1)
        }
        Err(_) => 127,
    }
}

pub(super) fn capture(prog: &str, args: &[&str], cwd: &Path) -> Option<(i32, String)> {
    let output = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Some((output.status.code().unwrap_or(1), text))
}

pub(super) fn date_utc() -> String {
    capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], Path::new("."))
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn epoch_secs() -> u64 {
    capture("date", &["+%s"], Path::new("."))
        .and_then(|(_, text)| text.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

// What a build depends on. Deliberately not a tree walk: `target/` and
// `references/` are enormous and neither is an input to the build.
fn inputs_digest(root: &Path, target: &str) -> Option<String> {
    capture(
        "python3",
        &["scripts/build-state.py", "inputs", "--target", target],
        root,
    )
    .map(|(_, text)| text)
}

fn build_commit(root: &Path) -> Option<String> {
    capture("python3", &["scripts/build-state.py", "commit"], root)
        .map(|(_, text)| text.lines().next().unwrap_or("").trim().to_string())
}

// ---------------------------------------------------------------- the worker
//
// Bootstrap FIRST and the build second, because the build needs `zig` for
// `ring`'s C and would fail without it in a way that reads as a code error.
pub(super) fn do_work(root: &Path, target: &str, bin: &Path, log: &Path) -> i32 {
    let t0 = epoch_secs();
    append_line(log, &format!("== podvm-dev started {}", date_utc()));
    append_line(log, "== bootstrap");
    let boot_rc = run_to_log(root, log, "scripts/common/bootstrap-env.sh", &[], &[]);
    let t1 = epoch_secs();
    append_line(
        log,
        &format!(
            "== bootstrap finished in {}s, rc={boot_rc}",
            t1.saturating_sub(t0)
        ),
    );
    if boot_rc != 0 {
        append_line(log, "FAILED: bootstrap");
        let _ = fs::write(status_path(root), "failed\n");
        return 1;
    }
    append_line(log, "== build the interposer inputs");
    let interpose_rc = run_to_log(root, log, "scripts/build-interpose.sh", &[], &[]);
    if interpose_rc != 0 {
        let _ = fs::write(status_path(root), "failed\n");
        return 1;
    }
    append_line(log, &format!("== cargo build --release --target {target}"));
    let commit = build_commit(root).unwrap_or_default();
    let build_rc = run_to_log(
        root,
        log,
        "cargo",
        &["build", "--release", "--target", target],
        &[("PODVM_BUILD_COMMIT".to_string(), commit)],
    );
    let t2 = epoch_secs();
    append_line(
        log,
        &format!(
            "== build finished in {}s, rc={build_rc}",
            t2.saturating_sub(t1)
        ),
    );
    append_line(log, &format!("== total {}s", t2.saturating_sub(t0)));
    if build_rc != 0 {
        append_line(log, "FAILED: cargo build");
        let _ = fs::write(status_path(root), "failed\n");
        return 1;
    }
    let record_rc = run_to_log(
        root,
        log,
        "python3",
        &["scripts/build-state.py", "record", "--target", target],
        &[],
    );
    if record_rc != 0 {
        let _ = fs::write(status_path(root), "failed\n");
        return 1;
    }
    match inputs_digest(root, target) {
        Some(digest) => {
            if fs::write(stamp_path(root), digest).is_err() {
                return 1;
            }
        }
        None => return 1,
    }
    let _ = fs::write(status_path(root), "ready\n");
    append_line(log, &format!("== ok. {}", bin.display()));
    0
}

pub(super) fn start_bg(root: &Path, target: &str) -> ExitCode {
    if !need_state(root) {
        eprintln!("podvm-dev: cannot create .dev state");
        return ExitCode::from(2);
    }
    if let Some(pid) = running(root) {
        println!("podvm-dev: already running (pid {pid}). Attaching rather than");
        println!("        starting a second cargo: two builds on one target directory");
        println!("        block on the same lock and the second looks like a hang.");
        println!("        log: .dev/build.log");
        return ExitCode::from(0);
    }
    let log = log_path(root);
    let _ = fs::write(&log, "");
    let _ = fs::write(status_path(root), "running\n");
    let exe = match env::current_exe() {
        Ok(exe) => exe,
        Err(_) => {
            eprintln!("podvm-dev: cannot locate its own binary");
            return ExitCode::from(2);
        }
    };
    // `setsid` and a full detach, so the build survives the shell that
    // started it. A session's turns are separate processes.
    let child = Command::new("setsid")
        .arg(exe)
        .arg("__worker")
        .env("PODVM_DEV_ROOT", root)
        .env("PODVM_DEV_TARGET", target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(root)
        .spawn();
    match child {
        Ok(child) => {
            let _ = fs::write(pid_path(root), format!("{}\n", child.id()));
            print_start_text();
            ExitCode::from(0)
        }
        Err(e) => {
            eprintln!("podvm-dev: setsid failed: {e}");
            ExitCode::from(2)
        }
    }
}

fn print_start_text() {
    println!("podvm-dev: the environment and the build are running in the background.");
    println!();
    println!("  log     .dev/build.log");
    println!("  status  podvm-dev status");
    println!("  wait    podvm-dev wait");
    println!();
    println!("Do not wait for it. Read these now, in this order, which is what");
    println!("AGENTS.md's routing table says and needs no toolchain:");
    println!();
    println!("     1. AGENTS.md            the router, and the absolutes");
    println!("     2. TODO/PROGRESS.md     the state, the work order, the open questions");
    println!("     3. TODO/RESUME.md       what the last session left in flight");
    println!("     4. the TODO/ entry your task names, in full");
    println!();
    println!("By the time that reading is done this will have finished.");
}

fn tail_lines(path: &Path, count: usize) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].iter().map(|line| line.to_string()).collect()
}

pub(super) fn do_status(root: &Path, target: &str) -> ExitCode {
    if let Some(pid) = running(root) {
        println!("running (pid {pid})");
        for line in tail_lines(&log_path(root), 3) {
            println!("  {line}");
        }
        return ExitCode::from(0);
    }
    let st = read_trim(&status_path(root)).unwrap_or_else(|| "unknown".to_string());
    println!("{st}");
    match st.as_str() {
        "ready" => {
            let rc = Command::new("python3")
                .args(["scripts/build-state.py", "status", "--target", target])
                .current_dir(root)
                .stdin(Stdio::null())
                .status()
                .map(|status| status.code().unwrap_or(1))
                .unwrap_or(1);
            ExitCode::from(rc as u8)
        }
        "failed" => {
            println!("  the last lines of .dev/build.log:");
            for line in tail_lines(&log_path(root), 12) {
                println!("  {line}");
            }
            ExitCode::from(1)
        }
        _ => ExitCode::from(2),
    }
}

pub(super) fn do_wait(root: &Path, target: &str) -> ExitCode {
    // Bounded. A runtime whose audience is automated may not wait
    // unbounded, and neither may its build script.
    let limit: u64 = env::var("PODVM_DEV_WAIT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEV_WAIT_DEFAULT);
    let mut waited: u64 = 0;
    while running(root).is_some() && waited < limit {
        thread::sleep(Duration::from_secs(2));
        waited += 2;
    }
    if running(root).is_some() {
        eprintln!("podvm-dev: still running after {limit}s. It is not abandoned; raise");
        eprintln!("        $PODVM_DEV_WAIT or watch .dev/build.log");
        return ExitCode::from(2);
    }
    do_status(root, target)
}

pub(super) fn do_build(root: &Path, target: &str) -> ExitCode {
    if !need_state(root) {
        eprintln!("podvm-dev: cannot create .dev state");
        return ExitCode::from(2);
    }
    // Foreground, for after a source change. It does NOT bootstrap: by the
    // time a session is editing code the environment is already up, and
    // paying for an apt check on every edit is the cost this script exists
    // to remove.
    let rc = Command::new("./scripts/build-interpose.sh")
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(1);
    if rc != 0 {
        return ExitCode::from(1);
    }
    let commit = match build_commit(root) {
        Some(commit) => commit,
        None => return ExitCode::from(1),
    };
    let rc = Command::new("cargo")
        .args(["build", "--release", "--target", target])
        .env("PODVM_BUILD_COMMIT", &commit)
        .current_dir(root)
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(1);
    if rc != 0 {
        return ExitCode::from(1);
    }
    let rc = Command::new("python3")
        .args(["scripts/build-state.py", "record", "--target", target])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(1);
    if rc != 0 {
        return ExitCode::from(1);
    }
    match inputs_digest(root, target) {
        Some(digest) => {
            if fs::write(stamp_path(root), digest).is_err() {
                return ExitCode::from(1);
            }
        }
        None => return ExitCode::from(1),
    }
    let _ = fs::write(status_path(root), "ready\n");
    println!("podvm-dev: {}", bin_path(root, target).display());
    ExitCode::from(0)
}

pub(super) fn do_check(root: &Path, target: &str) -> ExitCode {
    let commit = match build_commit(root) {
        Some(commit) => commit,
        None => return ExitCode::from(1),
    };
    let limit = env::var("PODVM_CHECK_STEP_TIMEOUT")
        .unwrap_or_else(|_| CHECK_STEP_TIMEOUT_DEFAULT.to_string());
    // What a change has to pass before it is committed, in the order that
    // fails cheapest first. THE INTERPOSER IS BUILT BEFORE THE BINARY,
    // because it is an INPUT to it.
    let mut pass: u32 = 0;
    let mut fail: u32 = 0;
    let mut skip: u32 = 0;
    for step in CHECK_STEPS {
        // The binary's own target fills the one step that names it.
        let expanded = step.replace("$TARGET", target);
        println!("== {expanded}");
        // The status is read from the step itself, unpiped: piping a check
        // into anything reports the pipeline's status, so a guard that
        // failed reads as green.
        let step_rc = Command::new("timeout")
            .arg(&limit)
            .arg("sh")
            .arg("-c")
            .arg(&expanded)
            .env("PODVM_BUILD_COMMIT", &commit)
            .current_dir(root)
            .status()
            .map(|status| status.code().unwrap_or(1))
            .unwrap_or(1);
        // A step that cannot run reports the third state and does not read
        // as a failure. Exit 2 is "could not run" everywhere in this tree;
        // only a real failure fails the check.
        match step_rc {
            0 => pass += 1,
            2 => {
                println!("   SKIP: {expanded} (it could not run here; nothing about its subject was verified)");
                skip += 1;
            }
            _ => {
                println!("   FAILED: {expanded}");
                fail += 1;
            }
        }
    }
    println!("== {pass} passed, {fail} failed, {skip} skipped");
    // A RUN THAT PASSED NOTHING IS NOT A GREEN RUN. Zero failures out of
    // zero checks executed is the shape check-gate.sh exists to refuse.
    if skip > 0 {
        println!("   A SKIP IS NOT A PASS. Those steps did not run and nothing about");
        println!("     their subject was verified.");
    }
    let rc = if fail > 0 || pass == 0 {
        1
    } else if skip > 0 {
        2
    } else {
        0
    };
    if rc == 0 {
        if !need_state(root) {
            return ExitCode::from(1);
        }
        let record_rc = Command::new("python3")
            .args(["scripts/build-state.py", "record", "--target", target])
            .current_dir(root)
            .stdin(Stdio::null())
            .status()
            .map(|status| status.code().unwrap_or(1))
            .unwrap_or(1);
        if record_rc != 0 {
            return ExitCode::from(1);
        }
        match inputs_digest(root, target) {
            Some(digest) => {
                if fs::write(stamp_path(root), digest).is_err() {
                    return ExitCode::from(1);
                }
            }
            None => return ExitCode::from(1),
        }
        let _ = fs::write(status_path(root), "ready\n");
    }
    ExitCode::from(rc as u8)
}
