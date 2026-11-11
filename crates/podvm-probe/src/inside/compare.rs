//! Compare a report with a reference (TODO/probe.md T-1666).
//!
//! A compare between two hosts is never a pass by itself. Only a difference
//! in an invariant section ([`super::INVARIANT`]) can fail, and only where
//! the expected-differences file gives no cause for it.

use std::collections::{BTreeMap, BTreeSet};

use super::{Record, Verdict};

/// One explained difference: `SECTION<TAB>TARGET<TAB>REASON`, where TARGET
/// may be `*` for each target of the section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub section: String,
    pub target: String,
    pub reason: String,
}

/// The lines of an expected-differences file; `#` starts a note.
pub fn parse_expected(text: &str) -> Vec<Expected> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some(Expected {
                section: f.next()?.to_string(),
                target: f.next()?.to_string(),
                reason: f.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// One difference of verdict for one key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub section: String,
    pub target: String,
    pub here: Verdict,
    pub reference: Verdict,
    /// The reason of the expected-differences file, where one names it.
    pub reason: Option<String>,
}

impl Difference {
    pub fn invariant(&self) -> bool {
        super::INVARIANT.contains(&self.section.as_str())
    }

    /// True where a strict compare fails on it.
    pub fn unexplained_invariant(&self) -> bool {
        self.invariant() && self.reason.is_none()
    }
}

/// The result of a compare.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub same: usize,
    pub differ: Vec<Difference>,
    pub only_reference: usize,
    pub only_here: usize,
}

impl Outcome {
    /// The differences that fail a strict compare.
    pub fn unexplained(&self) -> Vec<&Difference> {
        self.differ
            .iter()
            .filter(|d| d.unexplained_invariant())
            .collect()
    }

    /// The exit code: 0, or 1 under `strict` where an invariant section
    /// differs with no cause.
    pub fn code(&self, strict: bool) -> i32 {
        if strict && !self.unexplained().is_empty() {
            1
        } else {
            0
        }
    }

    /// The summary line of the issue's format.
    pub fn summary(&self) -> String {
        let explained = self.differ.iter().filter(|d| d.reason.is_some()).count();
        format!(
            "{} same, {} differ ({explained} explained, {} unexplained in an invariant \
             section), {} only in the reference, {} only here",
            self.same,
            self.differ.len(),
            self.unexplained().len(),
            self.only_reference,
            self.only_here
        )
    }
}

fn keyed(records: &[Record]) -> BTreeMap<(String, String), Verdict> {
    records
        .iter()
        .map(|r| ((r.section.clone(), r.target.clone()), r.verdict))
        .collect()
}

/// Compare `here` with `reference`, explaining by `expected`.
pub fn compare(here: &[Record], reference: &[Record], expected: &[Expected]) -> Outcome {
    let h = keyed(here);
    let r = keyed(reference);
    let keys: BTreeSet<_> = h.keys().chain(r.keys()).cloned().collect();
    let mut out = Outcome::default();
    for k in keys {
        match (h.get(&k), r.get(&k)) {
            (Some(a), Some(b)) if a == b => out.same += 1,
            (Some(a), Some(b)) => {
                let reason = expected
                    .iter()
                    .find(|e| e.section == k.0 && (e.target == "*" || e.target == k.1))
                    .map(|e| e.reason.clone());
                out.differ.push(Difference {
                    section: k.0.clone(),
                    target: k.1.clone(),
                    here: *a,
                    reference: *b,
                    reason,
                });
            }
            (Some(_), None) => out.only_here += 1,
            (None, Some(_)) => out.only_reference += 1,
            (None, None) => {}
        }
    }
    out
}
