//! podvm-package-ssh: can the release ship the exact SSH helper names
//! with runnable static bytes? (TODO/packaging.md T-1737, T-1405.)
//!
//! The port of the shell script scripts/package-ssh.sh (gone), with the same messages and exit
//! codes. Each helper (node, operator, proxy, shell) must exist, be
//! executable, be an ELF file with no PT_INTERP, and answer `--help` with
//! exit 125 and `usage: NAME` within 15 s, under QEMU when one is named.
//! The archive `dist/podvm-ssh-ARCH.tar.gz` holds the helpers, the licence,
//! the licence of the shims and the dependency licences, with fixed
//! owners and mtimes, and a `.sha256` beside it.
//!
//! Usage: podvm-package-ssh RELEASE_DIR ARCH [QEMU]
//!
//! Run it from the root of the tree. The dependency licences come from
//! `podvm-release-licenses` beside this binary, or from `PODVM_LICENSES`.
//!
//! Exit: 0 packaged, 1 a helper or a step failed, 2 it could not run.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[path = "package_ssh/elf.rs"]
mod elf;

const HELPERS: [&str; 4] = ["node", "operator", "proxy", "shell"];
const ARCHES: [&str; 7] = [
    "x86_64",
    "aarch64",
    "riscv64gc",
    "loongarch64",
    "armv7",
    "i686",
    "powerpc64le",
];
/// The variables of a live helper: a usage check must not read them.
const SCRUB: [&str; 4] = [
    "PODSSH_RELAY",
    "PODSSH_NAME",
    "PODSSH_NODE_TOKEN",
    "PODSSH_CONNECT_TOKEN",
];
const USAGE_DEADLINE: Duration = Duration::from_secs(15);

/// A failure of the package (exit 1) or of the run (exit 2).
enum Stop {
    Fail(String),
    CannotRun(String),
    Code(i32),
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(arch) => {
            println!("SSH-PACKAGE-OK {arch}: node operator proxy shell and retained notices");
            0
        }
        Err(Stop::Fail(m)) => {
            eprintln!("SSH-PACKAGE-FAIL {m}");
            1
        }
        Err(Stop::CannotRun(m)) => {
            eprintln!("podvm-package-ssh: {m}");
            2
        }
        Err(Stop::Code(c)) => c,
    };
    std::process::exit(code);
}

fn run(args: &[String]) -> Result<String, Stop> {
    let usage = || Stop::CannotRun("usage: podvm-package-ssh RELEASE_DIR ARCH [QEMU]".into());
    let (bin_dir, arch) = match args {
        [d, a] | [d, a, _] => (PathBuf::from(d), a.clone()),
        _ => return Err(usage()),
    };
    let qemu = args.get(2).filter(|q| !q.is_empty()).cloned();
    if !bin_dir.is_dir() || !ARCHES.contains(&arch.as_str()) {
        return Err(usage());
    }
    let root = std::env::current_dir().map_err(|e| Stop::CannotRun(e.to_string()))?;
    if !root.join("crates/podvm-ssh/shims/LICENSE").is_file() {
        return Err(Stop::CannotRun(
            "run it from the root of the podvm tree".into(),
        ));
    }
    let work = std::env::temp_dir().join(format!("podvm-package-ssh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package = work.join("package");
    fs::create_dir_all(&package).map_err(|e| Stop::CannotRun(e.to_string()))?;
    let result = build(&root, &bin_dir, &arch, qemu.as_deref(), &work, &package);
    let _ = fs::remove_dir_all(&work);
    result.map(|()| arch)
}

fn build(
    root: &Path,
    bin_dir: &Path,
    arch: &str,
    qemu: Option<&str>,
    work: &Path,
    package: &Path,
) -> Result<(), Stop> {
    for helper in HELPERS {
        check_helper(&bin_dir.join(helper), helper, qemu)?;
        copy(&bin_dir.join(helper), &package.join(helper))?;
    }
    copy(&root.join("LICENSE"), &package.join("LICENSE"))?;
    copy(
        &root.join("crates/podvm-ssh/shims/LICENSE"),
        &package.join("SSH-SHIM-LICENSE"),
    )?;
    let licenses = licenses_binary()?;
    let status = Command::new(&licenses)
        .arg("--output")
        .arg(package.join("dependency-licenses"))
        .status()
        .map_err(|e| Stop::CannotRun(format!("{}: {e}", licenses.display())))?;
    if !status.success() {
        return Err(Stop::Code(status.code().unwrap_or(1)));
    }
    let epoch = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(v) if !v.is_empty() => v,
        _ => git_epoch(root)?,
    };
    let tar = work.join("package.tar");
    step(
        Command::new("tar")
            .args(["--sort=name", &format!("--mtime=@{epoch}")])
            .args(["--owner=0", "--group=0", "--numeric-owner", "-cf"])
            .arg(&tar)
            .arg("-C")
            .arg(package)
            .arg("."),
        "tar",
    )?;
    step(Command::new("gzip").arg("-n").arg(&tar), "gzip")?;
    let dist = root.join("dist");
    fs::create_dir_all(&dist).map_err(|e| Stop::CannotRun(e.to_string()))?;
    let name = format!("podvm-ssh-{arch}.tar.gz");
    copy(&work.join("package.tar.gz"), &dist.join(&name))?;
    let sum = Command::new("sha256sum")
        .arg(&name)
        .current_dir(&dist)
        .output()
        .map_err(|e| Stop::Fail(format!("sha256sum: {e}")))?;
    if !sum.status.success() {
        return Err(Stop::Fail("sha256sum failed".into()));
    }
    fs::write(dist.join(format!("{name}.sha256")), &sum.stdout)
        .map_err(|e| Stop::Fail(format!("{name}.sha256: {e}")))?;
    Ok(())
}

/// The contract of one helper: present, executable, static ELF, usage.
fn check_helper(binary: &Path, helper: &str, qemu: Option<&str>) -> Result<(), Stop> {
    let executable = fs::metadata(binary)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false);
    if !executable {
        return Err(Stop::Fail(format!("missing executable: {helper}")));
    }
    let bytes = fs::read(binary).map_err(|e| Stop::Fail(format!("{helper}: {e}")))?;
    match elf::has_interp(&bytes) {
        Err(_) => return Err(Stop::Fail(format!("invalid ELF: {helper}"))),
        Ok(true) => return Err(Stop::Fail(format!("dynamic interpreter: {helper}"))),
        Ok(false) => {}
    }
    let mut cmd = match qemu {
        Some(q) => {
            let mut c = Command::new(q);
            c.arg(binary);
            c
        }
        None => Command::new(binary),
    };
    cmd.arg("--help");
    for v in SCRUB {
        cmd.env_remove(v);
    }
    let (code, out) = bounded(&mut cmd, USAGE_DEADLINE);
    if code != Some(125) || !out.contains(&format!("usage: {helper}")) {
        let shown = code.map_or("none".to_string(), |c| c.to_string());
        return Err(Stop::Fail(format!(
            "usage contract: {helper} (exit {shown})"
        )));
    }
    Ok(())
}

/// Run `cmd` with stdin closed and its output to a file; kill it at the
/// deadline. The exit code (None for a signal or a kill) and the output.
fn bounded(cmd: &mut Command, deadline: Duration) -> (Option<i32>, String) {
    let log = std::env::temp_dir().join(format!(
        "podvm-package-ssh-usage-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    let Ok(file) = fs::File::create(&log) else {
        return (None, String::new());
    };
    let Ok(err) = file.try_clone() else {
        return (None, String::new());
    };
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(err))
        .spawn();
    let Ok(mut child) = child else {
        let _ = fs::remove_file(&log);
        return (None, String::new());
    };
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if start.elapsed() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let out = fs::read_to_string(&log).unwrap_or_default();
    let _ = fs::remove_file(&log);
    (code, out)
}

fn copy(from: &Path, to: &Path) -> Result<(), Stop> {
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| Stop::Fail(format!("copy {}: {e}", from.display())))
}

fn step(cmd: &mut Command, what: &str) -> Result<(), Stop> {
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(Stop::Fail(format!("{what} exited {:?}", s.code()))),
        Err(e) => Err(Stop::Fail(format!("{what}: {e}"))),
    }
}

fn git_epoch(root: &Path) -> Result<String, Stop> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "-1", "--format=%ct"])
        .output()
        .map_err(|e| Stop::CannotRun(format!("git: {e}")))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || text.is_empty() {
        return Err(Stop::CannotRun(
            "no SOURCE_DATE_EPOCH and no git commit".into(),
        ));
    }
    Ok(text)
}

/// `PODVM_LICENSES`, else `podvm-release-licenses` beside this binary.
fn licenses_binary() -> Result<PathBuf, Stop> {
    if let Ok(p) = std::env::var("PODVM_LICENSES")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let exe = std::env::current_exe().map_err(|e| Stop::CannotRun(e.to_string()))?;
    let sibling = exe.with_file_name("podvm-release-licenses");
    if sibling.is_file() {
        return Ok(sibling);
    }
    Err(Stop::CannotRun(format!(
        "{} is not built; cargo build --release -p podvm-buildstate --bin podvm-release-licenses",
        sibling.display()
    )))
}
