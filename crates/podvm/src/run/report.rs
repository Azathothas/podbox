//! `run --report PATH` (TODO/cli.md T-1695, A-V-09 and G-06 of
//! PodUtils/podcrates#39): one JSON document for each run, written to PATH
//! with a rename, also when the run refuses before its start.
//!
//! The document goes to a file, never to standard output, which belongs to
//! the payload. It holds no value of an environment variable. A value that
//! podvm does not know is `null`.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// The version of the document's layout.
pub const SCHEMA: u32 = 1;

/// The report of one run, filled while the run goes.
#[derive(Debug)]
pub struct Report {
    pub schema: u32,
    pub podvm: &'static str,
    pub image: Option<String>,
    pub image_digest: Option<String>,
    /// The rung or tier that the run entered, and how podvm knows it.
    pub rung: Option<String>,
    pub measured_by: &'static str,
    pub argv: Vec<String>,
    /// The container of a detached run, which is still running.
    pub container: Option<String>,
    pub detached: bool,
    /// The exit code of the payload; null where the run refused or is
    /// detached.
    pub exit_code: Option<i32>,
    /// The signal that ended the payload, read from the code (128 + N).
    pub signal: Option<i32>,
    /// Why podvm did not run the payload: the exit code of the refusal.
    pub refused: Option<i32>,
    pub started: String,
    pub ended: Option<String>,
    pub wall_ms: Option<u64>,
    /// The CPU time and the peak resident set of the reaped children of
    /// the run (`getrusage(RUSAGE_CHILDREN)`): the payload's tree, and the
    /// steps of the completion layer before it.
    pub user_ms: Option<u64>,
    pub system_ms: Option<u64>,
    pub max_rss_kib: Option<u64>,
    pub timed_out: bool,
    pub timeout_s: Option<u64>,
    pub limits: Vec<String>,
    pub no_new_privs: bool,
    pub confine: Option<String>,
    pub labels: Vec<String>,
    /// Each fixup of the completion layer that changed a byte or failed.
    pub degradations: Vec<String>,
    clock: Option<Instant>,
    /// The payload was entered (or, detached, started): its code is its
    /// own. False is a refusal of podvm.
    pub ran: bool,
}

impl Report {
    pub fn begin(argv: &[String]) -> Report {
        Report {
            schema: SCHEMA,
            podvm: env!("CARGO_PKG_VERSION"),
            image: None,
            image_digest: None,
            rung: None,
            measured_by: "the rung that podvm entered, as its banner names it",
            argv: argv.to_vec(),
            container: None,
            detached: false,
            exit_code: None,
            signal: None,
            refused: None,
            started: podvm_image::clock::now(),
            ended: None,
            wall_ms: None,
            user_ms: None,
            system_ms: None,
            max_rss_kib: None,
            timed_out: false,
            timeout_s: None,
            limits: Vec::new(),
            no_new_privs: false,
            confine: None,
            labels: Vec::new(),
            degradations: Vec::new(),
            clock: Some(Instant::now()),
            ran: false,
        }
    }

    /// The facts of a prepared run.
    pub fn prepared(&mut self, p: &crate::lifecycle::Prepared) {
        self.image = Some(p.image.clone());
        self.image_digest = Some(p.record.manifest_digest.clone());
        self.rung = Some(p.rung.clone());
        self.argv = p.argv.clone();
        self.detached = p.detach;
        self.timeout_s = p.wall_ms.map(|m| m / 1000);
        self.limits = p.hardening.ulimits.clone();
        self.no_new_privs = p.hardening.no_new_privs;
        self.confine = p.hardening.confine.clone();
        self.labels = p.hardening.labels.clone();
        self.degradations = p.completion.clone();
    }

    /// The end of the run with exit code `code`: the payload's code where
    /// [`Report::ran`], else a refusal of podvm.
    pub fn end(&mut self, code: i32) {
        self.ended = Some(podvm_image::clock::now());
        self.wall_ms = self.clock.map(|c| c.elapsed().as_millis() as u64);
        if !self.ran {
            self.refused = Some(code);
        } else if !self.detached {
            self.exit_code = Some(code);
            self.signal = (129..=192).contains(&code).then(|| code - 128);
            if let Ok(u) = podvm_probe::sys::rusage_children() {
                self.user_ms = Some(u.user_us / 1000);
                self.system_ms = Some(u.system_us / 1000);
                self.max_rss_kib = Some(u.max_rss_kib);
            }
        }
    }

    /// The document.
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": self.schema,
            "podvm": self.podvm,
            "image": self.image,
            "image_digest": self.image_digest,
            "rung": self.rung,
            "measured_by": self.measured_by,
            "argv": self.argv,
            "container": self.container,
            "detached": self.detached,
            "exit_code": self.exit_code,
            "signal": self.signal,
            "refused": self.refused,
            "started": self.started,
            "ended": self.ended,
            "wall_ms": self.wall_ms,
            "user_ms": self.user_ms,
            "system_ms": self.system_ms,
            "max_rss_kib": self.max_rss_kib,
            "timed_out": self.timed_out,
            "timeout_s": self.timeout_s,
            "limits": self.limits,
            "no_new_privs": self.no_new_privs,
            "confine": self.confine,
            "labels": self.labels,
            "degradations": self.degradations,
        })
    }

    /// Write the document to `path`: a file beside it, then a rename.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.json()).map_err(|e| e.to_string())?;
        let dir = path
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        std::fs::write(&tmp, text + "\n").map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
    }
}

/// The PATH of `--report` in `args`, read before the parse so that a run
/// that the parse refuses still gets its report. The scan stops at `--`.
pub fn path_in(args: &[String]) -> Option<PathBuf> {
    let mut it = args.iter().take_while(|a| a.as_str() != "--");
    while let Some(a) = it.next() {
        if a == "--report" {
            return it.next().map(PathBuf::from);
        }
        if let Some(p) = a.strip_prefix("--report=") {
            return Some(PathBuf::from(p));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn report_has_every_field() {
        let mut r = Report::begin(&v(&["img", "sh", "-c", "exit 3"]));
        r.labels = v(&["job=7"]);
        r.rung = Some("namespace".into());
        r.ran = true;
        r.end(3);
        let j = r.json();
        for k in [
            "schema",
            "podvm",
            "image",
            "image_digest",
            "rung",
            "measured_by",
            "argv",
            "container",
            "detached",
            "exit_code",
            "signal",
            "refused",
            "started",
            "ended",
            "wall_ms",
            "user_ms",
            "system_ms",
            "max_rss_kib",
            "timed_out",
            "timeout_s",
            "limits",
            "no_new_privs",
            "confine",
            "labels",
            "degradations",
        ] {
            assert!(j.get(k).is_some(), "the report has no {k}: {j}");
        }
        assert_eq!(j["exit_code"], 3);
        assert_eq!(j["signal"], serde_json::Value::Null);
        assert_eq!(j["labels"][0], "job=7");
        assert!(j["wall_ms"].is_u64());
        let mut s = Report::begin(&[]);
        s.ran = true;
        s.end(137);
        assert_eq!(s.signal, Some(9));
    }

    #[test]
    fn report_written_on_refusal() {
        let dir = std::env::temp_dir().join(format!("podvm-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("r.json");
        let mut r = Report::begin(&v(&["nope"]));
        r.end(125);
        r.write(&path).unwrap();
        let j: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(j["refused"], 125);
        assert_eq!(j["exit_code"], serde_json::Value::Null);
        assert_eq!(j["rung"], serde_json::Value::Null);
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(left.len(), 1, "one file, no temporary left: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_path_is_found_before_the_parse() {
        assert_eq!(
            path_in(&v(&["--report", "r.json", "img"])),
            Some("r.json".into())
        );
        assert_eq!(
            path_in(&v(&["--report=r.json", "img"])),
            Some("r.json".into())
        );
        assert_eq!(path_in(&v(&["img", "--", "--report", "x"])), None);
        assert_eq!(path_in(&v(&["img"])), None);
    }
}
