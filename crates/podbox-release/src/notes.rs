//! release-notes: the nightly release notes TODO/packaging.md T-1334 needs.
//!
//! A behaviour-preserving port of scripts/release-notes.sh (T-1565). The
//! script stays as a compat shim that execs this binary; the logic lives
//! here. The notes carry the two facts a downloader otherwise cannot get:
//! the gate state of the commit TAG names, and the reproducibility boundary
//! of the bytes beside them. Both are computed, never copied.
//!
//! Usage: release-notes TAG
//!
//! Exit: 0 the notes printed, 2 the exact-commit gate or required read is
//! absent. `gh` and the network are required, so a machine without them is
//! exit 2: "could not run" is the third state, never a pass and never a
//! failure.

use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn run_to_string(prog: &str, args: &[&str], secs: u64) -> Option<(bool, String, String)> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) => Some((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
        _ => None,
    }
}

fn command_exists(name: &str) -> bool {
    match std::env::var_os("PATH") {
        None => false,
        Some(paths) => std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()),
    }
}

fn main() {
    let tag = std::env::args().nth(1).unwrap_or_default();
    if tag.is_empty() {
        eprintln!("release-notes: usage: release-notes.sh TAG");
        std::process::exit(2);
    }
    if !command_exists("gh") {
        eprintln!("release-notes: gh is not installed");
        std::process::exit(2);
    }
    let repo = std::env::var("GH_REPO").unwrap_or_else(|_| "Azathothas/podbox".to_string());
    // The gate conclusion on the tagged commit: the newest completed `gate`
    // run on main whose head SHA is the commit. A tag never moves, so the
    // newest completed run for the commit is the state of the commit.
    let commit = match run_to_string("git", &["rev-parse", &format!("{tag}^{{commit}}")], 60) {
        Some((true, stdout, _)) if !stdout.trim().is_empty() => stdout.trim().to_string(),
        _ => {
            eprintln!("release-notes: no commit for {tag}");
            std::process::exit(2);
        }
    };
    let query = format!("[.[] | select(.headSha == \"{commit}\")][0] | \"\\(.conclusion // \"none\") \\(.url // \"\")\"");
    let gate = match run_to_string(
        "gh",
        &[
            "run",
            "list",
            "--repo",
            &repo,
            "--workflow",
            "gate.yml",
            "--branch",
            "main",
            "--status",
            "completed",
            "--limit",
            "20",
            "--json",
            "headSha,conclusion,url",
            "--jq",
            &query,
        ],
        120,
    ) {
        Some((_, stdout, _)) => stdout.trim().to_string(),
        None => {
            eprintln!("release-notes: the gate runs could not be read");
            std::process::exit(2);
        }
    };
    // The script's case arms on `success *`: anything else refuses.
    if !gate.starts_with("success ") {
        eprintln!("release-notes: no successful main gate for the exact build commit");
        std::process::exit(2);
    }
    println!(
        "Nightly pre-release: seven static CLI binaries passed their architecture smoke checks."
    );
    println!("The smoke checks version, target, interposer digests, static linkage, image pull, and extraction.");
    println!("The full runtime acceptance uses the host architecture.");
    println!("Each binary embeds the x86_64 interposer pair. Other architectures refuse that interposition path until compatible objects exist.");
    println!();
    println!("Build commit: {commit}");
    println!("Gate on that commit: {gate}");
    println!();
    println!("Reproducibility boundary: byte-identical within one host and toolchain.");
    println!("Cross-host builds differ in the embedded glibc interposer object.");
    println!("Its digest is in `podbox version --verbose`.");
    println!(
        "Reproduction requires the builder's host glibc as well as its Rust and Zig toolchains."
    );
    // The script appends the SSH paragraph where the tagged tree carried
    // the packaging script. `git cat-file -e` names that condition exactly.
    let ssh_probe = format!("{commit}:scripts/package-ssh.sh");
    if run_to_string("git", &["cat-file", "-e", &ssh_probe], 60)
        .map(|(ok, _, _)| ok)
        .unwrap_or(false)
    {
        println!();
        println!("Each architecture also has a signed podbox-ssh archive and checksum.");
        println!("It contains node, operator, proxy, shell, and the locked registry package licence texts.");
        println!("Place compatible helpers beside podbox or on PATH. Supply the required external SSH server where the selected path needs it.");
        println!("Helper smoke checks static ELF linkage and usage status. It does not prove every architecture's live SSH session.");
        println!();
        println!("Current runtime limits and incomplete guest and server acceptance:");
        println!("https://github.com/{repo}/blob/{commit}/docs/limits.md");
    }
}
