//! podbox-dev: the session environment and build driver, as a binary.
//!
//! A behaviour-preserving port of three scripts (T-1555): the retired
//! `dev.sh` driver (the background bootstrap-plus-build and the
//! foreground check), `session-start.sh` (the host report and lane
//! setup), and `310-session-startup.sh` (the cold-start measurement).
//! Arguments are parsed by hand; `clap` was ruled out at T-0908. The crate
//! carries no dependencies.
//!
//! The verbs mirror the scripts exactly:
//!
//!   podbox-dev                  start the environment and build behind reading
//!   podbox-dev start            the same, explicit
//!   podbox-dev status           running, ready, failed, stale or unknown
//!   podbox-dev wait             block until the background work finishes
//!   podbox-dev build            one foreground build, for after a source change
//!   podbox-dev check            fmt, clippy, build, tests and both gates
//!   podbox-dev session          the host report, then the lane setup
//!   podbox-dev startup          the cold-start measurement T-1005 cites
//!
//! Exit: 0 the requested thing succeeded, 1 it failed, 2 it could not run.
//! `status` answers `stale` with exit 1 where the inputs moved under a
//! `ready`, exactly as the script did: a `ready` predating the last edit is
//! worse than no answer.
//!
//! Documented approximations, each untriggerable on a green tree. The
//! script asks `kill -0` whether the worker lives; this binary runs the
//! same `kill` program rather than linking a signal call, so the answer is
//! the program's and not an emulation. Detach goes through the same
//! `setsid` program the script used, re-executing this binary's hidden
//! `__worker` verb. Timestamps and durations come from the same `date`
//! program. The Python helpers (`scripts/build-state.py`) are invoked, not
//! reimplemented: their port is wave 5 (T-1559) and this binary must agree
//! with them until then, which running them guarantees.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

const DEFAULT_TARGET: &str = "x86_64-unknown-linux-musl";
const DEV_WAIT_DEFAULT: u64 = 1800;
const CHECK_STEP_TIMEOUT_DEFAULT: &str = "1800";

// What a change has to pass before it is committed, in the order that fails
// cheapest first. Mirrors `dev.sh check` step for step, including the
// interposer-before-binary order: `crates/podbox-cli/build.rs` embeds the
// two objects, so a binary built first embeds two empty placeholders and
// the check would pass with a podbox that interposes nothing.
// T-1555 repoint: the record gate step runs the gate binary, not the
// deleted check-todo.py.
const CHECK_STEPS: &[&str] = &[
    "cargo fmt --all -- --check",
    "cargo fmt --manifest-path crates/podbox-interpose/Cargo.toml -- --check",
    "cargo clippy --workspace --all-targets -- -D warnings",
    "RUSTFLAGS='-C target-feature=-crt-static' cargo clippy --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --all-targets -- -D warnings",
    "./scripts/build-interpose.sh",
    "cargo build --release --target $TARGET",
    "sh experiments/394-ssh-package.sh",
    "cargo test --workspace",
    "RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu",
    "python3 experiments/393-build-freshness.py",
    "./target/release/podbox-smoke --diagnostics",
    "python3 experiments/400-kvm-cleanup.py",
    "./target/release/podbox-gate",
    "./scripts/common/check-gate.sh --fast",
];

fn lexical_clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            _ => out.push(component.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(lexical_clean(&candidate));
        }
        candidate = candidate.join("..");
    }
    None
}

fn target_triple() -> String {
    env::var("PODBOX_TARGET").unwrap_or_else(|_| DEFAULT_TARGET.to_string())
}

fn state_dir(root: &Path) -> PathBuf {
    root.join(".dev")
}

fn log_path(root: &Path) -> PathBuf {
    state_dir(root).join("build.log")
}

fn pid_path(root: &Path) -> PathBuf {
    state_dir(root).join("pid")
}

fn status_path(root: &Path) -> PathBuf {
    state_dir(root).join("status")
}

fn stamp_path(root: &Path) -> PathBuf {
    state_dir(root).join("last-build-inputs")
}

fn bin_path(root: &Path, target: &str) -> PathBuf {
    root.join("target")
        .join(target)
        .join("release")
        .join("podbox")
}

fn need_state(root: &Path) -> bool {
    fs::create_dir_all(state_dir(root)).is_ok()
}

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

fn capture(prog: &str, args: &[&str], cwd: &Path) -> Option<(i32, String)> {
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

fn date_utc() -> String {
    capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], Path::new("."))
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn epoch_secs() -> u64 {
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
fn do_work(root: &Path, target: &str, bin: &Path, log: &Path) -> i32 {
    let t0 = epoch_secs();
    append_line(log, &format!("== podbox-dev started {}", date_utc()));
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
        &[("PODBOX_BUILD_COMMIT".to_string(), commit)],
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

fn start_bg(root: &Path, target: &str) -> ExitCode {
    if !need_state(root) {
        eprintln!("podbox-dev: cannot create .dev state");
        return ExitCode::from(2);
    }
    if let Some(pid) = running(root) {
        println!("podbox-dev: already running (pid {pid}). Attaching rather than");
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
            eprintln!("podbox-dev: cannot locate its own binary");
            return ExitCode::from(2);
        }
    };
    // `setsid` and a full detach, so the build survives the shell that
    // started it. A session's turns are separate processes.
    let child = Command::new("setsid")
        .arg(exe)
        .arg("__worker")
        .env("PODBOX_DEV_ROOT", root)
        .env("PODBOX_DEV_TARGET", target)
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
            eprintln!("podbox-dev: setsid failed: {e}");
            ExitCode::from(2)
        }
    }
}

fn print_start_text() {
    println!("podbox-dev: the environment and the build are running in the background.");
    println!();
    println!("  log     .dev/build.log");
    println!("  status  podbox-dev status");
    println!("  wait    podbox-dev wait");
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

fn do_status(root: &Path, target: &str) -> ExitCode {
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

fn do_wait(root: &Path, target: &str) -> ExitCode {
    // Bounded. A runtime whose audience is automated may not wait
    // unbounded, and neither may its build script.
    let limit: u64 = env::var("PODBOX_DEV_WAIT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEV_WAIT_DEFAULT);
    let mut waited: u64 = 0;
    while running(root).is_some() && waited < limit {
        thread::sleep(Duration::from_secs(2));
        waited += 2;
    }
    if running(root).is_some() {
        eprintln!("podbox-dev: still running after {limit}s. It is not abandoned; raise");
        eprintln!("        $PODBOX_DEV_WAIT or watch .dev/build.log");
        return ExitCode::from(2);
    }
    do_status(root, target)
}

fn do_build(root: &Path, target: &str) -> ExitCode {
    if !need_state(root) {
        eprintln!("podbox-dev: cannot create .dev state");
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
        .env("PODBOX_BUILD_COMMIT", &commit)
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
    println!("podbox-dev: {}", bin_path(root, target).display());
    ExitCode::from(0)
}

fn do_check(root: &Path, target: &str) -> ExitCode {
    let commit = match build_commit(root) {
        Some(commit) => commit,
        None => return ExitCode::from(1),
    };
    let limit = env::var("PODBOX_CHECK_STEP_TIMEOUT")
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
            .env("PODBOX_BUILD_COMMIT", &commit)
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

fn print_help() {
    println!("podbox-dev: get a fresh session to writing code as fast as possible.");
    println!();
    println!("  podbox-dev                  start the build behind the reading");
    println!("  podbox-dev start            the same, explicit");
    println!("  podbox-dev status           running, ready, failed, stale or unknown");
    println!("  podbox-dev wait             block until the background work finishes");
    println!("  podbox-dev build            one foreground build, after a source change");
    println!("  podbox-dev check            fmt, clippy, build, tests and both gates");
    println!("  podbox-dev session          the host report, then the lane setup");
    println!("  podbox-dev startup          the cold-start measurement T-1005 cites");
    println!();
    println!("Exit: 0 the requested thing succeeded, 1 it failed, 2 it could not run.");
}
// ------------------------------------------------------------ the session
//
// The one command a session runs first. It answers four questions and then
// starts the environment: where am I, what time is it, what is installed,
// and which lane does this host use. It creates nothing that a second run
// would create again.

fn say(quiet: bool, text: &str) {
    if !quiet {
        println!("{text}");
    }
}

fn row(quiet: bool, key: &str, value: &str) {
    if !quiet {
        println!("  {key:<14} {value}");
    }
}

fn tool_version(tool: &str, root: &Path) -> Option<String> {
    let _ = root;
    let (rc, text) = match tool {
        "python3" | "py" => capture(
            tool,
            &["-c", "import sys; print(sys.version.split()[0])"],
            Path::new("."),
        )?,
        "zig" => capture("zig", &["version"], Path::new("."))?,
        "go" => capture("go", &["version"], Path::new("."))?,
        "wsl-toolkit" => capture("wsl-toolkit", &["version"], Path::new("."))?,
        _ => capture(tool, &["--version"], Path::new("."))?,
    };
    let _ = rc;
    Some(text.lines().next().unwrap_or("").trim().to_string())
}

fn command_present(name: &str) -> bool {
    // `command -v` through `sh`: `command` is a shell builtin, not a
    // binary, so it cannot be executed directly.
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn do_session(root: &Path, target: &str, args: &[String]) -> ExitCode {
    let mut check = false;
    let mut quiet = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check = true,
            "--quiet" => quiet = true,
            "-h" | "--help" => {
                println!("podbox-dev session - the one command a session runs first.");
                return ExitCode::from(0);
            }
            _ => {
                eprintln!("podbox-dev: unknown option {arg}");
                return ExitCode::from(2);
            }
        }
    }
    let mut problems: u32 = 0;

    // 1. when. Read from the machine, never typed.
    let now = capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let local = capture("date", &["+%Y-%m-%dT%H:%M:%S%z"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    say(quiet, "== when");
    row(quiet, "utc", &now);
    row(quiet, "local", &local);

    // 2. where.
    let uname_s = capture("uname", &["-s"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let uname_r = capture("uname", &["-r"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let uname_m = capture("uname", &["-m"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mut kind = "unknown".to_string();
    match uname_s.as_str() {
        "Linux" => {
            kind = "linux".to_string();
            // A container and a WSL guest are both Linux and neither is the
            // host. Each is named rather than folded into one, because the
            // setup differs.
            let dockerenv = Path::new("/.dockerenv").exists();
            let cgroup = fs::read_to_string("/proc/1/cgroup").unwrap_or_default();
            let grouped = cgroup.contains("docker")
                || cgroup.contains("podman")
                || cgroup.contains("containerd")
                || cgroup.contains("libpod");
            let osrelease = fs::read_to_string("/proc/sys/kernel/osrelease")
                .unwrap_or_default()
                .to_lowercase();
            if dockerenv || grouped {
                kind = "container".to_string();
            } else if osrelease.contains("microsoft") {
                kind = "wsl-guest".to_string();
            }
        }
        other
            if other.starts_with("MINGW")
                || other.starts_with("MSYS")
                || other.starts_with("CYGWIN") =>
        {
            kind = "windows".to_string();
        }
        "Darwin" => kind = "darwin".to_string(),
        _ => {}
    }
    say(quiet, "");
    say(quiet, "== where");
    row(quiet, "kind", &kind);
    row(quiet, "uname", &format!("{uname_s} {uname_r} {uname_m}"));
    row(quiet, "cwd", &root.to_string_lossy());
    let user = capture("id", &["-un"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let uid = capture("id", &["-u"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    row(quiet, "user", &format!("{user}, uid {uid}"));
    if !root.join("AGENTS.md").is_file() || !root.join("TODO").is_dir() {
        eprintln!("podbox-dev: this is not the podbox tree. AGENTS.md or TODO/ is missing.");
        return ExitCode::from(2);
    }

    // 3. the tree.
    say(quiet, "");
    say(quiet, "== the tree");
    if command_present("git")
        && Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    {
        let branch = capture("git", &["rev-parse", "--abbrev-ref", "HEAD"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "branch", &branch);
        let head = capture("git", &["log", "-1", "--format=%h %s"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "head", &head);
        // grep -c exits 1 when it counts zero, which breaks an && chain.
        // The count is read from this process instead.
        let dirty = capture("git", &["status", "--porcelain"], root)
            .map(|(_, text)| text.lines().count().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "dirty", &format!("{dirty} file(s)"));
        let gname = capture("git", &["config", "user.name"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_default();
        let gmail = capture("git", &["config", "user.email"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_default();
        let identity = format!(
            "{} {}",
            if gname.is_empty() { "UNSET" } else { &gname },
            if gmail.is_empty() { "UNSET" } else { &gmail }
        );
        row(quiet, "identity", &identity);
        // The identity is the operator's and is never invented. An unset
        // one is named here rather than at the commit.
        if gname.is_empty() || gmail.is_empty() {
            eprintln!("  ! the git identity is unset. Read it from the history, then set it in this repository.");
            problems += 1;
        }
        // Two different states, and they need two different next steps.
        if branch == "main" {
        } else if branch.starts_with("publish/") {
            eprintln!("  ! on {branch}, which is a publish branch a previous session left open. Merge it or delete it before starting work: gh pr merge --rebase --delete-branch");
            problems += 1;
        } else {
            eprintln!("  ! on {branch}. Work happens on main. TODO/RULES.md section 2 settles this and it is not open.");
            problems += 1;
        }
    } else {
        eprintln!("  ! git is absent, or this directory is not a checkout");
        problems += 1;
    }

    // 4. tools. A NAME ON PATH IS NOT A WORKING PROGRAM: each version is
    // read by RUNNING the tool rather than by finding it.
    say(quiet, "");
    say(quiet, "== tools");
    for tool in [
        "git",
        "cargo",
        "rustc",
        "zig",
        "python3",
        "py",
        "jq",
        "curl",
        "tar",
        "podman",
        "docker",
        "go",
        "scc",
        "codegraph",
        "wsl-toolkit",
    ] {
        if command_present(tool) {
            let version = tool_version(tool, root)
                .filter(|version| !version.is_empty())
                .unwrap_or_else(|| "on PATH and answered nothing".to_string());
            row(quiet, tool, &version);
        } else {
            row(quiet, tool, "-");
        }
    }

    // 5. codegraph. The index is per machine and is not tracked, so it is
    // built here and synced every session.
    say(quiet, "");
    say(quiet, "== codegraph");
    if !command_present("codegraph") {
        row(
            quiet,
            "index",
            "codegraph is absent. grep is the fallback and it is slower.",
        );
    } else if check {
        if root.join(".codegraph").is_dir() {
            row(quiet, "index", "present, and --check builds nothing");
        } else {
            row(quiet, "index", "absent, and --check builds nothing");
        }
    } else if root.join(".codegraph").is_dir() {
        let ok = Command::new("codegraph")
            .arg("sync")
            .arg(".")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if ok {
            row(quiet, "index", "synced");
        } else {
            row(quiet, "index", "sync failed. Run codegraph sync");
        }
    } else {
        let ok = Command::new("codegraph")
            .arg("init")
            .arg(".")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if ok {
            row(quiet, "index", "built");
        } else {
            row(quiet, "index", "init failed. Run codegraph init");
        }
    }

    // 6. the lane. THE LANE IS THIS COMMAND'S DECISION AND NOT THE
    // SESSION'S. A session that picks the lane by hand runs the Debian
    // bootstrap against an Arch base, or calls wsl.exe on a host where
    // docs/containers.md forbids it.
    say(quiet, "");
    say(quiet, "== the lane");
    let lane = match kind.as_str() {
        "linux" | "container" | "wsl-guest" => "native",
        "windows" => "wsl-toolkit",
        _ => "none",
    };
    row(quiet, "lane", lane);
    match lane {
        "native" => {
            row(
                quiet,
                "setup",
                "scripts/common/bootstrap-env.sh, started by podbox-dev",
            );
            if check {
                let rc = Command::new("./scripts/common/bootstrap-env.sh")
                    .arg("--check")
                    .current_dir(root)
                    .status()
                    .map(|status| status.code().unwrap_or(1))
                    .unwrap_or(1);
                if rc != 0 {
                    eprintln!("  ! bootstrap --check exits {rc}");
                    problems += 1;
                }
            } else {
                // In the background on purpose. It compiles behind the
                // reading, and the reading needs no toolchain.
                let exe =
                    env::current_exe().unwrap_or_else(|_| root.join("target/release/podbox-dev"));
                let rc = Command::new(exe)
                    .arg("start")
                    .env("PODBOX_TARGET", target)
                    .current_dir(root)
                    .status()
                    .map(|status| status.code().unwrap_or(1))
                    .unwrap_or(1);
                if rc != 0 {
                    eprintln!("  ! podbox-dev start did not start");
                    problems += 1;
                }
            }
        }
        "wsl-toolkit" => {
            if !command_present("wsl-toolkit") {
                eprintln!(
                    "  ! wsl-toolkit is not on PATH. docs/agent-tooling.md says where it lives."
                );
                problems += 1;
            } else {
                row(
                    quiet,
                    "instance",
                    "podbox, which is the distribution wsl-toolkit-podbox",
                );
                if check {
                    let rc = Command::new("wsl-toolkit")
                        .args(["--instance", "podbox", "base", "status", "--probe"])
                        .current_dir(root)
                        .status()
                        .map(|status| status.code().unwrap_or(1))
                        .unwrap_or(1);
                    if rc != 0 {
                        eprintln!("  ! the base is not usable. Run this command without --check.");
                        problems += 1;
                    }
                } else {
                    // Idempotent. It says ALREADY EXISTS and creates nothing
                    // when the distribution is already registered.
                    let rc = Command::new("wsl-toolkit")
                        .args(["--instance", "podbox", "base", "ensure", "--probe"])
                        .current_dir(root)
                        .status()
                        .map(|status| status.code().unwrap_or(1))
                        .unwrap_or(1);
                    if rc != 0 {
                        eprintln!("  ! base ensure failed. It builds from an OCI image, so it needs a container engine on this host: podman machine start");
                        problems += 1;
                    }
                }
                row(quiet, "host half", "sh scripts/common/check-gate.sh --fast");
                row(
                    quiet,
                    "linux half",
                    "sh scripts/windows/run-in-base.sh JOB.sh",
                );
            }
        }
        _ => {
            eprintln!("  ! this host has no lane. docs/containers.md names the three that exist.");
            problems += 1;
        }
    }

    // 7. the reading.
    say(quiet, "");
    say(quiet, "== read these, in this order");
    say(
        quiet,
        "  1. AGENTS.md           the router and the absolutes",
    );
    say(
        quiet,
        "  2. TODO/PROGRESS.md    the state, the work order, the open questions",
    );
    say(
        quiet,
        "  3. TODO/RESUME.md      what the last session left in flight",
    );
    say(quiet, "  4. the TODO/ entry that your task names, in full");

    say(quiet, "");
    if problems > 0 {
        say(
            quiet,
            &format!("== {problems} thing(s) need attention. Each is named above."),
        );
        ExitCode::from(1)
    } else {
        say(
            quiet,
            &format!("== ready. The session start instant is {now}."),
        );
        ExitCode::from(0)
    }
}

// ------------------------------------------------------------ the startup
//
// How long a fresh session waits before it can run podbox, and how much of
// that wait can be spent reading instead. WHAT THIS CAN AND CANNOT MEASURE
// HERE: the container this runs in already has its tools and its cargo
// registry cache, so the apt and download halves cannot be re-measured
// without destroying them. They are reported as `not measured here` rather
// than estimated, and the compile half, which is the one that dominates on
// a warm registry, is measured properly against an empty target directory.

fn word_count(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.split_whitespace().count())
}

fn do_startup(root: &Path, _target: &str) -> ExitCode {
    let cargo_here = Command::new("cargo")
        .arg("--version")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !cargo_here {
        eprintln!("SKIP: cargo is not on PATH");
        return ExitCode::from(2);
    }
    // Checked before writing: a cold target directory is hundreds of
    // megabytes and the writable allowance is fixed while `df` misleads.
    let avail_mb: u64 = capture("df", &["-Pm", &root.to_string_lossy()], root)
        .and_then(|(_, text)| {
            text.lines()
                .nth(1)?
                .split_whitespace()
                .nth(3)?
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0);
    if avail_mb < 2048 {
        eprintln!(
            "SKIP: {avail_mb}MB free under {}, and a cold target needs more",
            root.display()
        );
        return ExitCode::from(2);
    }
    let work = env::temp_dir().join(format!("podbox-startup-{}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        eprintln!("SKIP: no temp dir");
        return ExitCode::from(2);
    }
    let mut report = String::new();
    let mut push = |line: &str| {
        report.push_str(line);
        report.push('\n');
    };
    let date = date_utc();
    let kernel = capture("uname", &["-r"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let cpus = capture("nproc", &[], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let rustc = capture("rustc", &["--version"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let registry = if env::var("HOME")
        .map(|home| PathBuf::from(home).join(".cargo/registry").is_dir())
        .unwrap_or(false)
    {
        "present, so no crate is downloaded below"
    } else {
        "absent"
    };
    push("== conditions");
    push(&format!("date              {date}"));
    push(&format!("host kernel       {kernel}"));
    push(&format!("cpus              {cpus}"));
    push(&format!("rustc             {rustc}"));
    push(&format!("cargo registry    {registry}"));
    push("");

    // 1. the compile, against an EMPTY target directory. A separate target
    // dir, so the repository's own is not destroyed to take a reading.
    push("== 1. the compile, against an EMPTY target directory");
    let cold = work.join("coldtarget");
    let cold_log = work.join("cold.log");
    let t0 = epoch_secs();
    let cold_out = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
            "--target-dir",
        ])
        .arg(&cold)
        .current_dir(root)
        .stdin(Stdio::null())
        .output();
    let cold_rc = match &cold_out {
        Ok(output) => {
            let mut combined = output.stdout.clone();
            combined.extend_from_slice(&output.stderr);
            let _ = fs::write(&cold_log, combined);
            output.status.code().unwrap_or(1)
        }
        Err(_) => 127,
    };
    let t1 = epoch_secs();
    let cold_s = t1.saturating_sub(t0);
    let cold_size = capture("du", &["-sh", &cold.to_string_lossy()], root)
        .map(|(_, text)| text.split_whitespace().next().unwrap_or("-").to_string())
        .unwrap_or_else(|| "-".to_string());
    let cold_crates = fs::read_to_string(&cold_log)
        .map(|text| {
            text.lines()
                .filter(|line| line.starts_with("   Compiling"))
                .count()
        })
        .unwrap_or(0);
    push(&format!("  exit              {cold_rc}"));
    push(&format!("  wall clock        {cold_s}s"));
    push(&format!("  target size       {cold_size}"));
    push(&format!("  crates compiled   {cold_crates}"));

    // 2. the same build, warm.
    push("");
    push("== 2. the same build, warm");
    let t0 = epoch_secs();
    let _ = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
            "--target-dir",
        ])
        .arg(&cold)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!("  wall clock        {}s", t1.saturating_sub(t0)));

    // 3. what the environment costs when it is already there.
    push("");
    push("== 3. what the environment costs when it is already there");
    let t0 = epoch_secs();
    let _ = Command::new("./scripts/common/bootstrap-env.sh")
        .arg("--check")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!("  bootstrap --check {}s", t1.saturating_sub(t0)));
    push("  the INSTALL path is not measured here: this container already has");
    push("    every component, and re-measuring would mean removing them. What a");
    push("    genuinely fresh machine pays for apt and the zig download is");
    push("    not measured rather than estimated.");

    // 4. what a session can read while that happens. Words rather than a
    // guess at a reading speed: the number a reader can check.
    push("");
    push("== 4. what a session can read while that happens");
    let mut words: usize = 0;
    for file in ["TODO/PROGRESS.md", "AGENTS.md"] {
        let count = word_count(&root.join(file)).unwrap_or(0);
        push(&format!("  {file:<22} {count} words"));
        words += count;
    }
    push(&format!("  together              {words} words"));
    push("  That is the reading AGENTS.md opens with, and it needs no");
    push(&format!(
        "    toolchain. podbox-dev start starts the {cold_s}s of clause 1 behind it."
    ));

    // 5. the driver returns immediately rather than blocking.
    push("");
    push("== 5. podbox-dev returns immediately rather than blocking");
    let exe = env::current_exe().unwrap_or_else(|_| root.join("target/release/podbox-dev"));
    let t0 = epoch_secs();
    let _ = Command::new(&exe)
        .arg("start")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!(
        "  podbox-dev start  {}s to return",
        t1.saturating_sub(t0)
    ));
    let _ = Command::new(&exe)
        .arg("wait")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let status_first = Command::new(&exe)
        .arg("status")
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    push(&format!("  then wait says    {status_first}"));

    print!("{report}");
    let out = root.join("experiments/results/session-startup.txt");
    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out, &report).is_err() {
        eprintln!("podbox-dev: cannot write {}", out.display());
        let _ = fs::remove_dir_all(&work);
        return ExitCode::from(2);
    }
    println!();
    println!("written to experiments/results/session-startup.txt");
    let diff = root.join("scripts/common/result-diff.sh");
    if diff.is_file() {
        let _ = Command::new(diff)
            .arg(&out)
            .current_dir(root)
            .stdin(Stdio::null())
            .status();
    }
    let _ = fs::remove_dir_all(&work);
    ExitCode::from(0)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    // The build worker re-executes this binary detached; it reads its
    // paths from the environment the starter set, like the script's
    // `declare -f` snapshot carried the functions.
    if argv.first().map(|arg| arg.as_str()) == Some("__worker") {
        let root = env::var("PODBOX_DEV_ROOT").unwrap_or_else(|_| ".".to_string());
        let target = env::var("PODBOX_DEV_TARGET").unwrap_or_else(|_| DEFAULT_TARGET.to_string());
        let root = PathBuf::from(root);
        let bin = bin_path(&root, &target);
        let log = env::var("PODBOX_DEV_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| log_path(&root));
        let rc = do_work(&root, &target, &bin, &log);
        return ExitCode::from(rc as u8);
    }
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SKIP: cannot locate the repository root");
            return ExitCode::from(2);
        }
    };
    let target = target_triple();
    let verb = argv.first().cloned().unwrap_or_else(|| "start".to_string());
    let rest = if argv.is_empty() { &[][..] } else { &argv[1..] };
    match verb.as_str() {
        "start" => {
            if !rest.is_empty() {
                eprintln!("podbox-dev: start takes no arguments");
                return ExitCode::from(2);
            }
            start_bg(&root, &target)
        }
        "status" => do_status(&root, &target),
        "wait" => do_wait(&root, &target),
        "build" => do_build(&root, &target),
        "check" => do_check(&root, &target),
        "session" => do_session(&root, &target, rest),
        "startup" => {
            if !rest.is_empty() {
                eprintln!("podbox-dev: startup takes no arguments");
                return ExitCode::from(2);
            }
            do_startup(&root, &target)
        }
        "-h" | "--help" | "help" => {
            print_help();
            ExitCode::from(0)
        }
        _ => {
            eprintln!("podbox-dev: unknown command {verb}");
            print_help();
            ExitCode::from(2)
        }
    }
}
