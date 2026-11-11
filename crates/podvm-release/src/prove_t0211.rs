//! podvm-prove-t0211: the store-lock proofs for TODO/image.md T-0211.
//!
//! A behaviour-preserving port of experiments/120-reproducible-build.sh and
//! experiments/157-lock-inheritance-prove.sh (T-1563). Both scripts stay as
//! compat shims that exec this binary with the matching subcommand; the
//! logic lives here. The two proofs share one binary because both are
//! mutation-planted store-lock proofs, not because they share a name.
//!
//! Usage: podvm-prove-t0211 {reproducible|lock-inheritance} [--root DIR]
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
// The mutated file, relative to the copy of the podcrates checkout
// (`prove_t0211/plant.rs`). `Lock` moved there with podcrates T-060.
const SUBJECT: &str = "crates/pod-supervise/src/lock.rs";
// The file of this tree that the patched builds can change. The proof
// restores it after each mutation.
const LOCKFILE: &str = "Cargo.lock";
// The current anchors: the guard in `Lock::try_acquire_noting` and the one
// flag in `Lock::open`. The binary encodes the current source text, and it
// refuses a mutation that does not land.
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

#[path = "prove_t0211/legs.rs"]
mod legs;
use legs::*;
#[path = "prove_t0211/plant.rs"]
mod plant;
use plant::*;

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
            eprintln!("podvm-prove-t0211: unexpected argument: {arg}");
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
            eprintln!("podvm-prove-t0211: usage: podvm-prove-t0211 {{reproducible|lock-inheritance}} [--root DIR]");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
#[path = "prove_t0211/tests.rs"]
mod tests;
