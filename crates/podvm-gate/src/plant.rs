//! podvm-plant: break each of the gate's checks on purpose and assert red.
//!
//! A behaviour-preserving port of the retired `plant.sh` harness (T-1554).
//! The gate reports success identically whether its assertions passed or
//! whether it examined nothing, and this binary tells those apart: it plants
//! one defect per check into tracked files, asserts the gate goes red with
//! that defect's own message, then restores the tree from a copy.
//!
//! Read the finding, not the exit code. A gate already red for another reason
//! exits 1 either way, so every case asserts that the planted defect's own
//! message appears and that it did not appear on the clean tree.
//!
//! Four guards, each of which exists because the harness shape without it
//! reports success while planting nothing:
//!
//!   1. THE MUTATION MUST LAND. A rewrite that matches nothing leaves the
//!      source correct, the gate green, and the case reads as a missing
//!      assertion when what is missing is the plant. Asserted by hashing the
//!      files before and after.
//!   2. RESTORE FROM A COPY, NEVER A CHECKOUT. Checkout restores from the
//!      index, so anything a plant staged survives it, and a run started
//!      while work is staged restores the plant instead of the source.
//!      Measured: with a plant staged, restoring one file from the index
//!      leaves the plant in the worktree.
//!   3. ONE FILE LIST. A restore that carries its own second copy of the list
//!      stops putting back the first file a new case learns to touch, and
//!      every later case then measures the leftovers. Everything any case may
//!      touch is named in FILES once, and backup and restore iterate it.
//!   4. CONTROLS ARE COUNTED APART FROM PLANTS. A control stays quiet; a
//!      plant goes red. Adding them together reports more defects caught
//!      than were.
//!
//! Exit: 0 every plant was caught and every control stayed quiet, 1 a plant
//! was missed or a control fired, 2 could not run.
//!
//! Arguments are parsed by hand; the crate carries no dependencies. The
//! binary takes no arguments and extras are ignored, exactly as the script
//! ignored them. The gate under test is the `podvm-gate` binary beside this
//! binary's own path, which is `./target/release/podvm-gate` in CI after
//! the todo job's build step copies it there.
//!
//! Two deliberate divergences from the script, both recorded rather than
//! silent. The script located the checkout from its own path, with a special
//! case for a `/in/job.sh` delivery; a binary is never delivered that way,
//! so this binary walks up from its own directory for `TODO/INDEX.md`, the
//! same walk the gate and the counter use. The script ran the gate and the
//! test tools with no bound; this binary bounds every child process (gate
//! 300 s, cargo 900 s) and a bound that fires reads as could-not-run.
//!
//! T-1579's pins live beside the ported cases: one case per wave-2 source
//! and per batch-4 test file. Those cases drive `cargo test`, not the gate:
//! each inserts a failing assertion naming its own file into that file's own
//! test, asserts the run goes red with that message, then restores. With
//! `PODVM_PLANT_TRANSCRIPT` set to a path, each of those red runs is
//! recorded to that file as evidence, truncated fresh at the start of the
//! run; unset, the binary prints only the
//! verdict lines, exactly like the script. The transcript path must not name
//! a FILES entry. It is written after each case restores, so it never moves
//! the hashes guard 1 compares.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

// Guard 3: ONE LIST. Everything any case may touch is named here once, and
// both the backup and the restore iterate this and nothing else. The wave-2
// sources and the batch-4 test files joined this list under D-3 (T-1578):
// each T-1579 pin mutates its file, so the dirty-tree guard and the
// backup/restore must cover it, the same as every older case's file.
const FILES: &[&str] = &[
    "TODO/INDEX.md",
    "TODO/PROGRESS.md",
    "TODO/probe.md",
    "TODO/enter.md",
    "TODO/RULES.md",
    "TODO/reference-map.md",
    "README.md",
    "docs/conventions/prose.md",
    "docs/runtime-state.md",
    "docs/findings.md",
    "experiments/110-bloat-delta.sh",
    "experiments/results/bloat-baseline.txt",
    "experiments/results/bloat-image.txt",
    "experiments/results/bloat-interpose.txt",
    "experiments/360-perf-harness.sh",
    "experiments/perf-ceilings.tsv",
    "experiments/results/perf-lane.txt",
    "experiments/results/perf-seeds.tsv",
    ".github/workflows/gate.yml",
    "crates/podvm-supervise/src/lib.rs",
    "crates/podvm/src/parity.rs",
    "crates/podvm/src/run.rs",
    "scripts/build-interpose.sh",
    "crates/podvm-gate/src/dev/steps.rs",
    "crates/podvm-gate/src/interpose_build.rs",
    "crates/podvm/src/images/tests.rs",
    "crates/podvm/src/system/tests.rs",
    "crates/podvm/src/parity/tests.rs",
    "crates/podvm-interpose/src/memo.rs",
    "crates/podvm-interpose/src/identity.rs",
    "crates/podvm-probe/src/exit.rs",
    "crates/podvm/tests/curated_refusals.rs",
    "crates/podvm/tests/detached_stdio.rs",
    "crates/podvm/tests/qol.rs",
    "crates/podvm/tests/store_gates.rs",
    "crates/podvm-image/tests/acquisition.rs",
    "crates/podvm-image/tests/across_distributions.rs",
    "crates/podvm-image/tests/common/mod.rs",
    "crates/podvm-image/tests/common/registry.rs",
    "crates/podvm-image/tests/parallel_layers.rs",
    "crates/podvm-image/tests/registry_fixture.rs",
    "crates/podvm-image/tests/store_digest.rs",
];

const GATE_TIMEOUT_SECS: u64 = 300;
const CARGO_TIMEOUT_SECS: u64 = 900;

struct ChildOut {
    code: Option<i32>,
    combined: String,
}

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

fn gate_path() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = format!("podvm-gate{}", env::consts::EXE_SUFFIX);
    Some(dir.join(name))
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(path) {
            Ok(meta) => meta.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

// Every child process is bounded. A bound that fires reads as could-not-run;
// the caller turns that into exit 2. Standard output and standard error are
// concatenated in that order; substring matching cannot tell the orders
// apart, which is all the verdicts below need.
fn run_bounded(
    program: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(String, String)],
    timeout_secs: u64,
) -> Result<ChildOut, String> {
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {program}: {e}"))?;
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                let mut combined = stdout;
                combined.push_str(&stderr);
                return Ok(ChildOut {
                    code: status.code(),
                    combined,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program} timed out after {timeout_secs}s"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("could not wait for {program}: {e}")),
        }
    }
}

#[path = "plant/harness.rs"]
mod harness;
use harness::*;

#[path = "plant/run.rs"]
mod run;
use run::*;

#[path = "plant/cases_middle.rs"]
mod cases_middle;
use cases_middle::*;

#[path = "plant/cases_late.rs"]
mod cases_late;
use cases_late::*;

#[path = "plant/finish.rs"]
mod finish;
use finish::*;

#[path = "plant/pins.rs"]
mod pins;
use pins::*;

fn main() {
    std::process::exit(run());
}
