//! `podvm-api`: the typed request of `podvm run` and the record of its
//! `--report`, for a program that runs podvm (TODO/cli.md T-1745, the ask
//! V-1 of PodUtils/podcrates#124).
//!
//! The flags of podvm change at each beta (the ruling "Compatibility",
//! 2026-11-05). A caller that writes them itself breaks at each beta. This
//! crate holds the map from a typed [`Run`] to the argv of the podvm
//! binary of the same version, and the reader of the record of layout
//! [`SCHEMA`]. A test of podvm holds each flag that the map writes to a row
//! of podvm's parity table, so a renamed flag fails podvm's CI, not the
//! caller.
//!
//! ⛔ The crate spawns the binary; it does not run podvm in the caller's
//! process. A run makes podvm the child subreaper of the payload's tree,
//! arms `PR_SET_PDEATHSIG` on the forking thread and gives the payload the
//! process's standard streams: each is a property of a whole process, and a
//! caller that starts many runs from its threads must not take them on.
//!
//! The values of the environment go through `--env-file`, a file of mode
//! 0600 in a private directory of mode 0700, never through argv.

mod report;
#[cfg(test)]
mod tests;

pub use report::Report;

use std::fmt;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// The layout version of the record that this crate reads.
pub const SCHEMA: u64 = 1;

/// One run of `podvm run --rm`: the request, field by field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Run {
    pub image: String,
    /// The command in the image; empty runs the image's own command.
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub volumes: Vec<Volume>,
    pub workdir: Option<String>,
    pub labels: Vec<(String, String)>,
    pub ulimits: Vec<Ulimit>,
    /// `--timeout`: whole seconds over 0.
    pub timeout_s: Option<u64>,
    /// `--podvm-tier`; `None` lets the ladder select the rung.
    pub tier: Option<Tier>,
    /// `--podvm-confine`, as podvm's parity table gives the spec.
    pub confine: Option<String>,
    pub pull: Pull,
}

/// `-v HOST:GUEST[:ro|:rw]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub host: PathBuf,
    pub guest: String,
    pub read_only: bool,
}

/// `--ulimit NAME=SOFT[:HARD]`; -1 is no limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ulimit {
    pub name: String,
    pub soft: i64,
    pub hard: Option<i64>,
}

/// The boundaries that `--podvm-tier` selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Machine,
    Runsc,
    Syscall,
    Incus,
    Emulate,
}

impl Tier {
    pub fn word(self) -> &'static str {
        match self {
            Tier::Machine => "machine",
            Tier::Runsc => "runsc",
            Tier::Syscall => "syscall",
            Tier::Incus => "incus",
            Tier::Emulate => "emulate",
        }
    }
}

/// `--pull`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pull {
    #[default]
    Missing,
    Never,
    Always,
}

impl Pull {
    pub fn word(self) -> &'static str {
        match self {
            Pull::Missing => "missing",
            Pull::Never => "never",
            Pull::Always => "always",
        }
    }
}

/// Why a run gave no record. A message names a field or a key, never the
/// value of an environment variable.
#[derive(Debug)]
pub enum Error {
    /// The request is not one that podvm takes.
    Invalid(String),
    /// The private directory, the env file or the spawn failed.
    Io(String),
    /// podvm ended with no record (`status` is its exit).
    NoReport { status: Option<i32>, why: String },
    /// The record is not of layout [`SCHEMA`], or a field is not of its type.
    Record(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Invalid(w) => write!(f, "podvm-api: the request is refused: {w}"),
            Error::Io(w) => write!(f, "podvm-api: {w}"),
            Error::NoReport { status, why } => {
                write!(
                    f,
                    "podvm-api: podvm ended (exit {status:?}) with no record: {why}"
                )
            }
            Error::Record(w) => write!(f, "podvm-api: the record does not read: {w}"),
        }
    }
}

impl std::error::Error for Error {}

fn invalid(w: impl Into<String>) -> Error {
    Error::Invalid(w.into())
}

impl Run {
    pub fn new(image: impl Into<String>, command: &[&str]) -> Run {
        Run {
            image: image.into(),
            command: command.iter().map(|s| s.to_string()).collect(),
            ..Run::default()
        }
    }

    /// Refuse a request that podvm would read in another way than the
    /// fields say: a value that a flag cannot carry is an error here, not a
    /// silent change.
    pub fn check(&self) -> Result<(), Error> {
        if self.image.is_empty() || self.image.starts_with('-') {
            return Err(invalid("the image is empty or starts with '-'"));
        }
        for (k, v) in &self.env {
            if k.is_empty() || k.contains('=') || k.starts_with('#') || k.trim() != k {
                return Err(invalid(format!("the environment key {k:?} is not a name")));
            }
            if v.contains(['\n', '\r', '\0']) || v.trim_end() != v {
                return Err(invalid(format!(
                    "the value of {k} has a line break, a NUL or a trailing blank, which the env file cannot carry"
                )));
            }
        }
        for vol in &self.volumes {
            let host = vol.host.to_string_lossy();
            if !vol.host.is_absolute()
                || host.contains(':')
                || !vol.guest.starts_with('/')
                || vol.guest.contains(':')
            {
                return Err(invalid(format!(
                    "the volume {host}:{} needs two absolute paths with no ':'",
                    vol.guest
                )));
            }
        }
        if let Some(w) = &self.workdir
            && !w.starts_with('/')
        {
            return Err(invalid(format!("the workdir {w:?} is not absolute")));
        }
        for (k, _) in &self.labels {
            if k.is_empty() || k.contains('=') {
                return Err(invalid(format!("the label key {k:?} is empty or has '='")));
            }
        }
        for u in &self.ulimits {
            if u.name.is_empty() || !u.name.bytes().all(|b| b.is_ascii_lowercase()) {
                return Err(invalid(format!(
                    "the ulimit name {:?} is not a word",
                    u.name
                )));
            }
        }
        if self.timeout_s == Some(0) {
            return Err(invalid("--timeout takes whole seconds over 0"));
        }
        Ok(())
    }

    /// The argv of podvm for this request, with the record at `report` and
    /// the env file at `env_file` (needed where [`Run::env`] is not empty).
    pub fn argv(&self, report: &Path, env_file: Option<&Path>) -> Result<Vec<String>, Error> {
        self.check()?;
        let mut a: Vec<String> = vec!["run".into(), "--rm".into()];
        a.push("--report".into());
        a.push(report.display().to_string());
        a.push("--pull".into());
        a.push(self.pull.word().into());
        if !self.env.is_empty() {
            let f = env_file.ok_or_else(|| invalid("the environment needs an env file"))?;
            a.push("--env-file".into());
            a.push(f.display().to_string());
        }
        for v in &self.volumes {
            let mode = if v.read_only { "ro" } else { "rw" };
            a.push("-v".into());
            a.push(format!("{}:{}:{mode}", v.host.display(), v.guest));
        }
        if let Some(w) = &self.workdir {
            a.push("--workdir".into());
            a.push(w.clone());
        }
        for (k, v) in &self.labels {
            a.push("--label".into());
            a.push(format!("{k}={v}"));
        }
        for u in &self.ulimits {
            let hard = u.hard.map(|h| format!(":{h}")).unwrap_or_default();
            a.push("--ulimit".into());
            a.push(format!("{}={}{hard}", u.name, u.soft));
        }
        if let Some(t) = self.timeout_s {
            a.push("--timeout".into());
            a.push(t.to_string());
        }
        if let Some(t) = self.tier {
            a.push("--podvm-tier".into());
            a.push(t.word().into());
        }
        if let Some(c) = &self.confine {
            a.push("--podvm-confine".into());
            a.push(c.clone());
        }
        a.push(self.image.clone());
        a.extend(self.command.iter().cloned());
        Ok(a)
    }

    /// The job of this request for the binary at `podvm`: its private
    /// directory and its command, which the caller can change (the
    /// environment of podvm, the standard streams) before [`Job::run`].
    pub fn job(&self, podvm: &Path) -> Result<Job, Error> {
        let dir = private_dir()?;
        let report = dir.join("report.json");
        let env_file = if self.env.is_empty() {
            None
        } else {
            let f = dir.join("env");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&f)
                .map_err(|e| Error::Io(format!("{}: {e}", f.display())))?;
            for (k, v) in &self.env {
                writeln!(file, "{k}={v}")
                    .map_err(|e| Error::Io(format!("{}: {e}", f.display())))?;
            }
            Some(f)
        };
        let argv = match self.argv(&report, env_file.as_deref()) {
            Ok(a) => a,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(e);
            }
        };
        let mut command = Command::new(podvm);
        command.args(&argv).stdin(Stdio::null());
        Ok(Job {
            dir,
            report,
            command,
        })
    }

    /// Run the request with the binary at `podvm` and read its record. The
    /// payload's streams are the caller's; standard input is empty.
    pub fn run(&self, podvm: &Path) -> Result<Report, Error> {
        self.job(podvm)?.run()
    }
}

/// A prepared run: the private directory and the command of podvm.
#[derive(Debug)]
pub struct Job {
    dir: PathBuf,
    report: PathBuf,
    command: Command,
}

impl Job {
    /// The command, for the caller's changes before the run.
    pub fn command(&mut self) -> &mut Command {
        &mut self.command
    }

    /// Run podvm, wait for it, and read the record. The private directory
    /// goes in each case.
    pub fn run(mut self) -> Result<Report, Error> {
        let status = self.command.status();
        let out = match status {
            Err(e) => Err(Error::Io(format!("podvm does not start: {e}"))),
            Ok(s) => match std::fs::read_to_string(&self.report) {
                Ok(text) => Report::parse(&text),
                Err(e) => Err(Error::NoReport {
                    status: s.code(),
                    why: e.to_string(),
                }),
            },
        };
        let _ = std::fs::remove_dir_all(&self.dir);
        out
    }
}

/// A new directory of mode 0700 under the temporary directory.
fn private_dir() -> Result<PathBuf, Error> {
    static N: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir();
    for _ in 0..16 {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = base.join(format!("podvm-api-{}-{n}", std::process::id()));
        match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(Error::Io(format!("{}: {e}", dir.display()))),
        }
    }
    Err(Error::Io(
        "no free private directory in the temporary directory".into(),
    ))
}
