//! The probe inside the payload (TODO/probe.md T-1666): what a payload
//! actually gets inside a tier, as records of the sandprobe format.
//!
//! One record is one line `SECTION<TAB>TARGET<TAB>VERDICT<TAB>DETAIL`. A
//! line that starts with `#` is a note (the tier, the image, the kernel).
//! The verdicts are the sandprobe vocabulary ([`Verdict`]). A verdict is
//! written only for something observed; a check that could not run is
//! `UNKNOWN` with the reason (the T-0109 rule).
//!
//! [`checks`] runs the checks in the payload; [`compare`] reads two reports;
//! [`scrub`] replaces a value that looks like a secret with `REDACTED`
//! before it is written.

pub mod checks;
pub mod compare;
pub mod net;
pub mod scrub;

#[cfg(test)]
mod tests;

/// The verdicts of the sandprobe format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// The action succeeded.
    Allow,
    /// The kernel or a layer refused it (the detail names the errno).
    Deny,
    /// The target does not exist here.
    Absent,
    /// The check could not run (the detail names the reason).
    Unknown,
    /// The action did not finish in its bound.
    Timeout,
    /// A name did not resolve.
    Unresolved,
    /// The peer refused the connection.
    Refused,
    /// The packets went nowhere: no answer and no error.
    Dropped,
    /// A resource ran out.
    Exhausted,
}

impl Verdict {
    pub const ALL: [Verdict; 9] = [
        Verdict::Allow,
        Verdict::Deny,
        Verdict::Absent,
        Verdict::Unknown,
        Verdict::Timeout,
        Verdict::Unresolved,
        Verdict::Refused,
        Verdict::Dropped,
        Verdict::Exhausted,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Verdict::Allow => "ALLOW",
            Verdict::Deny => "DENY",
            Verdict::Absent => "ABSENT",
            Verdict::Unknown => "UNKNOWN",
            Verdict::Timeout => "TIMEOUT",
            Verdict::Unresolved => "UNRESOLVED",
            Verdict::Refused => "REFUSED",
            Verdict::Dropped => "DROPPED",
            Verdict::Exhausted => "EXHAUSTED",
        }
    }

    pub fn from_word(w: &str) -> Option<Verdict> {
        Verdict::ALL.into_iter().find(|v| v.word() == w)
    }

    /// The verdict of an errno of a call that was refused.
    pub fn of_errno(e: i32) -> Verdict {
        match e {
            2 | 6 | 19 | 97 => Verdict::Absent, // ENOENT, ENXIO, ENODEV, EAFNOSUPPORT
            111 => Verdict::Refused,            // ECONNREFUSED
            110 => Verdict::Timeout,            // ETIMEDOUT
            12 | 23 | 24 | 28 => Verdict::Exhausted, // ENOMEM, ENFILE, EMFILE, ENOSPC
            101 | 113 => Verdict::Dropped,      // ENETUNREACH, EHOSTUNREACH
            _ => Verdict::Deny,
        }
    }
}

/// The sections where a difference between two reports can fail a strict
/// compare: each is a boundary claim, not a property of the host.
pub const INVARIANT: &[&str] = &[
    "escalation",
    "escape-surface",
    "procfs",
    "raw-socket",
    "proxy",
    "tcp-direct",
    "tcp-direct-ip",
    "udp-direct",
    "loopback",
    "metadata",
    "dns",
];

/// One record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub section: String,
    pub target: String,
    pub verdict: Verdict,
    pub detail: String,
}

impl Record {
    pub fn new(section: &str, target: &str, verdict: Verdict, detail: &str) -> Record {
        Record {
            section: section.to_string(),
            target: target.to_string(),
            verdict,
            detail: detail.to_string(),
        }
    }

    /// The line of the record, with each field free of a tab or a newline
    /// and the detail scrubbed.
    pub fn line(&self) -> String {
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        format!(
            "{}\t{}\t{}\t{}",
            clean(&self.section),
            clean(&self.target),
            self.verdict.word(),
            clean(&scrub::scrub(&self.detail))
        )
    }

    /// The record of one line, or None for a note, an empty line or a line
    /// that is not a record.
    pub fn parse(line: &str) -> Option<Record> {
        if line.starts_with('#') || line.trim().is_empty() {
            return None;
        }
        let mut f = line.splitn(4, '\t');
        let section = f.next()?;
        let target = f.next()?;
        let verdict = Verdict::from_word(f.next()?)?;
        let detail = f.next().unwrap_or("");
        Some(Record::new(section, target, verdict, detail))
    }

    /// True where the section is an invariant one.
    pub fn invariant(&self) -> bool {
        INVARIANT.contains(&self.section.as_str())
    }
}

/// The records of a report.
pub fn parse_report(text: &str) -> Vec<Record> {
    text.lines().filter_map(Record::parse).collect()
}

/// The count of each verdict, as `ALLOW 3, DENY 2`.
pub fn tally(records: &[Record]) -> String {
    let mut parts = Vec::new();
    for v in Verdict::ALL {
        let n = records.iter().filter(|r| r.verdict == v).count();
        if n > 0 {
            parts.push(format!("{} {n}", v.word()));
        }
    }
    parts.join(", ")
}

/// The variable through which podvm gives the probe a host path (its
/// store), so the report shows whether a host path stays reachable.
pub const HOST_VAR: &str = "PODVM_SANDPROBE_HOST";

/// The hidden argument of podvm that runs the probe in the payload.
pub const ARG: &str = "--podvm-sandprobe";

/// Run each check in this process (the payload) and give the report: the
/// notes, then one line for each record.
pub fn run(product: &str) -> String {
    let host = std::env::var(HOST_VAR).ok();
    let mut records = checks::identity();
    records.extend(checks::escalation());
    records.extend(checks::escape_surface(host.as_deref()));
    records.extend(checks::procfs());
    records.extend(checks::raw_socket());
    records.extend(checks::proxy());
    records.extend(net::sections());
    records.extend(checks::env());
    let mut out = format!(
        "# sandprobe of {product} (TODO/probe.md T-1666)\n# records {}: {}\n",
        records.len(),
        tally(&records)
    );
    for r in &records {
        out.push_str(&r.line());
        out.push('\n');
    }
    out
}
