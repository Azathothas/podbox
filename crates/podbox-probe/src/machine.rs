//! `TODO/podvm.md` T-1301: the machine tier as legs, and the refusal rule.
//!
//! One leg per fact the tier needs, each measured, never inferred from
//! another leg. The tier is refused when any leg is missing, and the refusal
//! names the leg: not a reduced machine tier. The legs run as [`crate::probes`]
//! rows in [`crate::probes::Group::Machine`] order; this module reads their
//! verdicts back through [`crate::probes::MACHINE_LEGS`], so a rename moves
//! the rows, the constant and this assessment together.
//!
//! ⛔ A row that is absent is a missing leg, not a passing one. A document
//! written before the legs existed carries no verdict for them, and reading
//! that absence as anything but missing would be the silent non-isolation
//! T-1301 exists to refuse.

use crate::verdict::Outcome;
use crate::Findings;

/// One leg's verdict. `None` where the findings carry no such row: the leg
/// was never measured, which is missing rather than any verdict.
pub struct Leg {
    pub name: &'static str,
    pub outcome: Option<Outcome>,
}

impl Leg {
    fn missing(&self) -> bool {
        match &self.outcome {
            Some(o) => o.verdict != crate::verdict::Verdict::Ok,
            None => true,
        }
    }

    fn detail(&self) -> String {
        match &self.outcome {
            Some(o) => match (o.verdict, o.errno) {
                (crate::verdict::Verdict::Ok, _) => format!("{}=ok", self.name),
                (crate::verdict::Verdict::Denied, Some(e)) => {
                    format!("{}={}", self.name, e.name())
                }
                (crate::verdict::Verdict::Denied, None) => {
                    format!("{}=denied", self.name)
                }
                (crate::verdict::Verdict::Skip, _) if o.reason.is_empty() => {
                    format!("{}=skip", self.name)
                }
                (crate::verdict::Verdict::Skip, _) => {
                    format!("{}=skip: {}", self.name, o.reason)
                }
            },
            None => format!("{}=(not probed)", self.name),
        }
    }
}

/// What the legs established: every leg with its verdict, and the names of
/// the ones that are missing.
pub struct Assessment {
    pub legs: Vec<Leg>,
    /// The legs that are missing, in run order. Empty exactly when the tier
    /// holds.
    pub missing: Vec<&'static str>,
}

impl Assessment {
    /// True when any leg is missing: a denial, a skip, or a row that is
    /// absent. A machine tier with no acceleration and no network is two
    /// different products depending on which leg failed, so there is no
    /// partial tier.
    pub fn refused(&self) -> bool {
        !self.missing.is_empty()
    }

    /// The refusal, naming every missing leg with what it measured. `None`
    /// where the tier holds.
    pub fn refusal(&self) -> Option<String> {
        if self.missing.is_empty() {
            return None;
        }
        let mut out = String::from("machine tier refused:");
        for leg in &self.legs {
            if self.missing.contains(&leg.name) {
                out.push_str(&format!(" {}", leg.detail()));
            }
        }
        Some(out)
    }
}

/// Read the machine legs out of a finished run.
pub fn assess(f: &Findings) -> Assessment {
    let legs: Vec<Leg> = crate::probes::MACHINE_LEGS
        .iter()
        .map(|&name| Leg {
            name,
            outcome: f.get(name).cloned(),
        })
        .collect();
    let missing: Vec<&'static str> = legs
        .iter()
        .filter(|l| l.missing())
        .map(|l| l.name)
        .collect();
    Assessment { legs, missing }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys;

    fn findings(rows: &[(&'static str, Outcome)]) -> Findings {
        Findings {
            rows: rows.to_vec(),
            ..Findings::empty()
        }
    }

    fn all_ok() -> Vec<(&'static str, Outcome)> {
        crate::probes::MACHINE_LEGS
            .iter()
            .map(|&name| (name, Outcome::ok()))
            .collect()
    }

    #[test]
    fn six_clear_legs_hold_the_tier() {
        let a = assess(&findings(&all_ok()));
        assert_eq!(a.legs.len(), 6);
        assert!(a.missing.is_empty());
        assert!(!a.refused());
        assert!(a.refusal().is_none());
    }

    #[test]
    fn a_denied_leg_refuses_and_names_its_errno() {
        let mut rows = all_ok();
        rows[1] = ("open(/dev/kvm, O_RDWR)", Outcome::denied(sys::ENOENT));
        let a = assess(&findings(&rows));
        assert!(a.refused());
        assert_eq!(a.missing, vec!["open(/dev/kvm, O_RDWR)"]);
        let refusal = a.refusal().expect("a refusal prints");
        assert!(
            refusal.contains("open(/dev/kvm, O_RDWR)=ENOENT"),
            "{refusal}"
        );
    }

    #[test]
    fn a_skipped_leg_refuses_and_names_its_reason() {
        // The common shape on a machine with no emulator: the leg could not
        // run, which is missing rather than any verdict about acceleration.
        let mut rows = all_ok();
        rows[0] = (
            "qemu-system-x86_64 --version",
            Outcome::skip(Some(sys::ENOENT), "no qemu-system-x86_64 on PATH"),
        );
        let a = assess(&findings(&rows));
        assert!(a.refused());
        assert_eq!(a.missing, vec!["qemu-system-x86_64 --version"]);
        let refusal = a.refusal().expect("a refusal prints");
        assert!(
            refusal.contains("no qemu-system-x86_64 on PATH"),
            "{refusal}"
        );
    }

    #[test]
    fn a_row_that_is_absent_is_missing_rather_than_any_verdict() {
        // A document written before the legs existed: five legs measured, the
        // sixth never run. Its absence refuses the tier; it is not read as a
        // denial and not as a pass.
        let mut rows = all_ok();
        rows.pop();
        let a = assess(&findings(&rows));
        assert!(a.refused());
        assert_eq!(a.missing, vec!["qemu-system-x86_64 -accel help"]);
        let refusal = a.refusal().expect("a refusal prints");
        assert!(
            refusal.contains("qemu-system-x86_64 -accel help=(not probed)"),
            "{refusal}"
        );
    }

    #[test]
    fn the_refusal_names_every_missing_leg_in_run_order() {
        let mut rows = all_ok();
        rows[1] = ("open(/dev/kvm, O_RDWR)", Outcome::denied(sys::EACCES));
        rows[3] = (
            "open(/dev/net/tun, O_RDWR)",
            Outcome::skip(Some(sys::ENOENT), "no tun node here"),
        );
        let a = assess(&findings(&rows));
        assert_eq!(
            a.missing,
            vec!["open(/dev/kvm, O_RDWR)", "open(/dev/net/tun, O_RDWR)"]
        );
        let refusal = a.refusal().expect("a refusal prints");
        assert!(
            refusal.contains("open(/dev/kvm, O_RDWR)=EACCES"),
            "{refusal}"
        );
        assert!(
            refusal.contains("open(/dev/net/tun, O_RDWR)=skip"),
            "{refusal}"
        );
        assert!(
            refusal.find("kvm").unwrap() < refusal.find("tun").unwrap(),
            "{refusal}"
        );
    }
}
