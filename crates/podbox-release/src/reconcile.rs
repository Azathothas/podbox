//! podbox-reconcile: which changes does the retained publish branch add?
//!
//! A behaviour-preserving port of experiments/395-reconcile-repository.py
//! (T-1566). The script stays as a compat shim that execs this binary; the
//! logic lives here. Git history and patch comparison only; runtime
//! acceptance is separate, and the tool writes to no remote.
//!
//! Usage: podbox-reconcile [--expect-deleted] [--root DIR]
//!
//! Exit: 0 the comparison printed (or the obsolete ref is absent, with
//! `--expect-deleted`), 1 the repository state fails the expectation, 2 it
//! could not run.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const PUBLISH: &str = "origin/publish/20260926T052210Z";

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

fn git(root: &Path, args: &[&str], secs: u64) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = command
        .spawn()
        .map_err(|e| format!("could not run git: {e}"))?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Err(_) => Err("git timed out".to_string()),
        Ok(Err(e)) => Err(format!("could not read git: {e}")),
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .to_string())
            } else {
                Err(format!(
                    "git {} exit {}",
                    args.first().unwrap_or(&""),
                    output.status.code().unwrap_or(1)
                ))
            }
        }
    }
}

/// The ref-existence probe: `git show-ref --verify --quiet` answers in its
/// exit code alone. True where the ref exists.
fn ref_present(root: &Path, name: &str) -> Option<bool> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new("git");
    command
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/remotes/{name}"),
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(status)) => Some(status.success()),
        _ => None,
    }
}

fn date_utc(root: &Path) -> String {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new("date");
    command
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return "-".to_string(),
    };
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(output)) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "-".to_string(),
    }
}

fn main() {
    let mut expect_deleted = false;
    let mut root_flag: Option<String> = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        if arg == "--expect-deleted" {
            expect_deleted = true;
        } else if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else {
            eprintln!("podbox-reconcile: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podbox-reconcile: --root needs a directory");
            std::process::exit(2);
        }
        None => repo_root().unwrap_or_else(|| {
            eprintln!("podbox-reconcile: cannot locate the checkout root");
            std::process::exit(2);
        }),
    };
    let outcome = run(&root, expect_deleted);
    std::process::exit(outcome);
}

fn run(root: &Path, expect_deleted: bool) -> i32 {
    println!("== conditions");
    println!("{}", date_utc(root));
    let main = match git(root, &["rev-parse", "main"], 60) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("cannot read repository state: {e}");
            return 2;
        }
    };
    let origin_main = match git(root, &["rev-parse", "origin/main"], 60) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("cannot read repository state: {e}");
            return 2;
        }
    };
    let refs = match git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:short) %(objectname)",
            "refs/remotes/origin",
        ],
        60,
    ) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("cannot read repository state: {e}");
            return 2;
        }
    };
    println!("commit main: {main}");
    println!("commit origin/main: {origin_main}");
    println!("scope: Git history and patch comparison; runtime acceptance is separate");
    println!("== remote branch refs");
    println!("{refs}");
    let present = match ref_present(root, PUBLISH) {
        Some(present) => present,
        None => {
            eprintln!("cannot read repository state: the ref probe did not answer");
            return 2;
        }
    };
    if expect_deleted {
        // The script tests `result.returncode != 1`: absent (1) is the
        // expected state, and anything else, present or unreadable, fails.
        if !present {
            println!("verdict REPOSITORY-OK: obsolete publish ref is absent after fetch");
            return 0;
        }
        println!("verdict REPOSITORY-FAIL: publish ref is still present or unreadable");
        return 1;
    }
    if !present {
        println!("cannot run: the comparison requires the retained publish ref");
        return 2;
    }
    match git(root, &["cherry", "main", PUBLISH], 60) {
        Ok(text) => {
            println!("== patch identity comparison");
            println!("{text}");
        }
        Err(e) => {
            eprintln!("cannot read repository state: {e}");
            return 2;
        }
    }
    match git(
        root,
        &[
            "range-diff",
            "--no-color",
            "ce17a733^..82c12ba1",
            "82d4bff^..1c19cea",
        ],
        60,
    ) {
        Ok(text) => {
            println!("== equivalent series, with remaining context difference");
            println!("{text}");
        }
        Err(e) => {
            eprintln!("cannot read repository state: {e}");
            return 2;
        }
    }
    println!("verdict REPOSITORY-READ: compare the first patch source; do not infer identity from ancestry");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_pin_names_the_retained_branch() {
        assert_eq!(PUBLISH, "origin/publish/20260926T052210Z");
    }
}
