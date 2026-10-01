//! podbox-verify: the downloader's half of TODO/packaging.md T-1328.
//!
//! A behaviour-preserving port of scripts/verify-release.sh (T-1561). The
//! script stays as a compat shim that execs this binary; the logic lives
//! here. `gh` and `cosign` are driven directly, never through a shell, with
//! the same 120-second bound the script's `timeout` gave them.
//!
//! Usage: podbox-verify TAG ARCH [binary|ssh]
//!
//! Exit: 0 the bundle verifies and names the workflow identity, 1 the bytes
//! or the identity did not verify, 2 it could not run.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

static SCRATCH_SEQ: AtomicU64 = AtomicU64::new(0);

fn command_exists(name: &str) -> bool {
    match std::env::var_os("PATH") {
        None => false,
        Some(paths) => std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()),
    }
}

fn run_bounded(prog: &str, args: &[&str], cwd: &std::path::Path, secs: u64) -> RunEnd {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return RunEnd::Failed,
    };
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Err(_) => RunEnd::TimedOut,
        Ok(Err(_)) => RunEnd::Failed,
        Ok(Ok(status)) => {
            if status.success() {
                RunEnd::Ok
            } else {
                RunEnd::Failed
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum RunEnd {
    Ok,
    Failed,
    TimedOut,
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let tag = argv.next().unwrap_or_default();
    let arch = argv.next().unwrap_or_default();
    let kind = argv.next().unwrap_or_else(|| "binary".to_string());
    if tag.is_empty() || arch.is_empty() {
        eprintln!("verify-release: usage: verify-release.sh TAG ARCH [binary|ssh]");
        std::process::exit(2);
    }
    if !command_exists("cosign") {
        eprintln!("verify-release: cosign is not installed");
        std::process::exit(2);
    }
    if !command_exists("gh") {
        eprintln!("verify-release: gh is not installed");
        std::process::exit(2);
    }
    let repo = std::env::var("GH_REPO").unwrap_or_else(|_| "Azathothas/podbox".to_string());
    if ![
        "x86_64",
        "aarch64",
        "riscv64gc",
        "loongarch64",
        "armv7",
        "i686",
        "powerpc64le",
    ]
    .contains(&arch.as_str())
    {
        std::process::exit(2);
    }
    let asset = match kind.as_str() {
        "binary" => format!("podbox-{arch}"),
        "ssh" => format!("podbox-ssh-{arch}.tar.gz"),
        _ => {
            eprintln!("verify-release: kind must be binary or ssh");
            std::process::exit(2);
        }
    };
    let seq = SCRATCH_SEQ.fetch_add(1, Ordering::SeqCst);
    let work = std::env::temp_dir().join(format!("podbox-verify-{}-{seq}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        std::process::exit(2);
    }
    // A scratch directory that never survives the run, whatever exits first.
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }
    let _guard = Guard(work.clone());
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // The identity the publish job signs as: the nightly workflow at the tag
    // being verified, keyless through the job's OIDC. The workflow answers
    // version tags alone, so a bundle for TAG carries the tag's ref, never
    // the main branch. A bundle from any other identity fails closed here
    // rather than verifying as "some signature".
    if run_bounded(
        "gh",
        &[
            "release",
            "download",
            &tag,
            "--repo",
            &repo,
            "--dir",
            &work.to_string_lossy(),
            "--pattern",
            &asset,
            "--pattern",
            &format!("{asset}.sigstore"),
        ],
        &cwd,
        120,
    ) != RunEnd::Ok
    {
        eprintln!("verify-release: the nightly has no {asset} assets for {tag}");
        std::process::exit(2);
    }
    let bundle = work.join(format!("{asset}.sigstore"));
    let downloaded = work.join(&asset);
    let identity =
        format!("https://github.com/{repo}/.github/workflows/nightly.yml@refs/tags/{tag}");
    // A wait past the bound is "could not run" under the exit contract,
    // never a verification failure.
    match run_bounded(
        "cosign",
        &[
            "verify-blob",
            "--bundle",
            &bundle.to_string_lossy(),
            "--certificate-identity",
            &identity,
            "--certificate-oidc-issuer",
            "https://token.actions.githubusercontent.com",
            &downloaded.to_string_lossy(),
        ],
        &cwd,
        120,
    ) {
        RunEnd::Ok => std::process::exit(0),
        RunEnd::TimedOut => std::process::exit(2),
        RunEnd::Failed => std::process::exit(1),
    }
}
