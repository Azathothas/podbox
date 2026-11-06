//! podvm-smoke: the release smoke and the gate diagnostics, as a binary.
//!
//! A behaviour-preserving port of two scripts (T-1556):
//! `nightly-smoke.sh` (the per-arch smoke TODO/packaging.md T-1314
//! promises, extended by T-1329) and `398-gate-diagnostics.py`
//! (the failed-gate diagnostic drive TODO/gate.md T-1351 owns).
//! Arguments are parsed by hand; `clap` was ruled out at T-0908. The crate
//! carries no dependencies.
//!
//!   podvm-smoke BINARY TRIPLE [QEMU]   the per-arch smoke, script-compatible
//!   podvm-smoke --diagnostics          the gate-diagnostics drive
//!   podvm-smoke --diagnostics --powershell
//!                                        the same, with the Windows runner too
//!
//! Exit: 0 the smoke is green, 1 it ran and something failed, 2 it could
//! not run. The smoke half keeps the script's six assertion groups, each
//! read from the process that produced it; the diagnostics half keeps the
//! fixture shape (fixed stub checks, a late message after 20 control lines)
//! and the verdict word GATE-DIAGNOSTICS-OK.
//!
//! Documented approximations. The script cleans its scratch through a trap
//! that also fires on a signal; this binary removes the scratch on every
//! return path, which a kill cannot run, exactly like every other ported
//! tool. Waits are bounded by polling the child (90 s per diagnostics run,
//! like the script's timeout), because the standard library offers no
//! blocking wait with a deadline.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DIAGNOSTIC_CHECKS: &[&str] = &[
    "check-docs",
    "check-markers",
    "check-one-home",
    "check-placeholders",
    "check-control-bytes",
    "check-changelog",
    "check-attribution",
    "check-remote-items",
    "check-build-lane",
];
const MARKER: &str = "FIXTURE-LATE-DIAGNOSTIC";
const RUN_TIMEOUT_SECS: u64 = 90;

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(candidate);
        }
        candidate = candidate.join("..");
    }
    None
}

fn which(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

// A bounded run: poll the child and kill it past the deadline. Pipes are
// drained after exit, the same shape the gate and plant binaries use, and
// adequate here because every driven output is small.
fn run_bounded(
    prog: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(String, String)],
    timeout_secs: u64,
) -> Result<Run, String> {
    let mut cmd = Command::new(prog);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {prog}: {e}"))?;
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                return Ok(Run {
                    rc: status.code().unwrap_or(1),
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{prog} timed out after {timeout_secs}s"));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("could not wait for {prog}: {e}")),
        }
    }
}

// ------------------------------------------------------------ the smoke half

fn smoke_fail(triple: &str, message: &str) -> ExitCode {
    eprintln!("SMOKE-FAIL {triple}: {message}");
    ExitCode::from(1)
}

#[path = "smoke/smoke.rs"]
mod smoke;
use smoke::*;

#[path = "smoke/diagnostics.rs"]
mod diagnostics;
use diagnostics::*;

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.first().map(|s| s.as_str()) == Some("--diagnostics") {
        let root = match find_root() {
            Some(root) => root,
            None => {
                println!("cannot run: required interpreter is absent");
                return ExitCode::from(2);
            }
        };
        let mut powershell = false;
        for arg in &argv[1..] {
            match arg.as_str() {
                "--powershell" => powershell = true,
                _ => {
                    eprintln!("SMOKE-FAIL usage: podvm-smoke --diagnostics [--powershell]");
                    return ExitCode::from(2);
                }
            }
        }
        return do_diagnostics(&root, powershell);
    }
    if argv.first().map(|s| s.as_str()) == Some("-h")
        || argv.first().map(|s| s.as_str()) == Some("--help")
    {
        print_help();
        return ExitCode::from(0);
    }
    if argv.len() > 3 {
        eprintln!("SMOKE-FAIL usage: podvm-smoke BINARY TRIPLE [QEMU]");
        return ExitCode::from(2);
    }
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SMOKE-FAIL cannot locate the repository root");
            return ExitCode::from(2);
        }
    };
    let bin = argv.first().cloned().unwrap_or_default();
    let triple = argv.get(1).cloned().unwrap_or_default();
    let qemu = argv.get(2).cloned().unwrap_or_default();
    do_smoke(&bin, &triple, &qemu, &root)
}
