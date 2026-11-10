//! podvm-dev: the session environment and build driver, as a binary.
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
//!   podvm-dev                  start the environment and build behind reading
//!   podvm-dev start            the same, explicit
//!   podvm-dev status           running, ready, failed, stale or unknown
//!   podvm-dev wait             block until the background work finishes
//!   podvm-dev build            one foreground build, for after a source change
//!   podvm-dev check            fmt, clippy, build, tests and both gates
//!   podvm-dev session          the host report, then the lane setup
//!   podvm-dev startup          the cold-start measurement T-1005 cites
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
// interposer-before-binary order: `crates/podvm/build.rs` embeds the
// two objects, so a binary built first embeds two empty placeholders and
// the check would pass with a podvm that interposes nothing.
// T-1555 repoint: the record gate step runs the gate binary, not the
// deleted check-todo.py.
const CHECK_STEPS: &[&str] = &[
    "cargo fmt --all -- --check",
    "cargo fmt --manifest-path crates/podvm-interpose/Cargo.toml -- --check",
    "cargo clippy --workspace --all-targets -- -D warnings",
    "RUSTFLAGS='-C target-feature=-crt-static' cargo clippy --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --all-targets -- -D warnings",
    "./scripts/build-interpose.sh",
    "cargo build --release --target $TARGET",
    "sh experiments/394-ssh-package.sh",
    "cargo test --workspace",
    "RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu",
    "python3 experiments/393-build-freshness.py",
    "./target/x86_64-unknown-linux-musl/release/podvm-smoke --diagnostics",
    "python3 experiments/400-kvm-cleanup.py",
    "./target/x86_64-unknown-linux-musl/release/podvm-gate",
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
    env::var("PODVM_TARGET").unwrap_or_else(|_| DEFAULT_TARGET.to_string())
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
        .join("podvm")
}

fn need_state(root: &Path) -> bool {
    fs::create_dir_all(state_dir(root)).is_ok()
}

#[path = "dev/steps.rs"]
mod steps;
use steps::*;

#[path = "dev/session.rs"]
mod session;
use session::*;

#[path = "dev/startup.rs"]
mod startup;
use startup::*;

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    // The build worker re-executes this binary detached; it reads its
    // paths from the environment the starter set, like the script's
    // `declare -f` snapshot carried the functions.
    if argv.first().map(|arg| arg.as_str()) == Some("__worker") {
        let root = env::var("PODVM_DEV_ROOT").unwrap_or_else(|_| ".".to_string());
        let target = env::var("PODVM_DEV_TARGET").unwrap_or_else(|_| DEFAULT_TARGET.to_string());
        let root = PathBuf::from(root);
        let bin = bin_path(&root, &target);
        let log = env::var("PODVM_DEV_LOG")
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
                eprintln!("podvm-dev: start takes no arguments");
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
                eprintln!("podvm-dev: startup takes no arguments");
                return ExitCode::from(2);
            }
            do_startup(&root, &target)
        }
        "-h" | "--help" | "help" => {
            print_help();
            ExitCode::from(0)
        }
        _ => {
            eprintln!("podvm-dev: unknown command {verb}");
            print_help();
            ExitCode::from(2)
        }
    }
}
