//! `TODO/podvm.md` T-1306: the non-goals as measured verdicts, never constants.
//!
//! The specification lists designs that were tried and do not work on its
//! target. Each died on a MECHANISM, and every mechanism is something the
//! probe already measures or that one new census row measures:
//!
//! | non-goal | mechanism (spec evidence) | probe rows cited |
//! | --- | --- | --- |
//! | tcp listener | bind EPERM everywhere (E-75) | `bind(127.0.0.1:0)+listen` |
//! | kvm acceleration | no node, uncreatable (E-10/30/60) | `open(/dev/kvm, O_RDWR)` |
//! | runc-style oci | UTS namespace EPERM (E-43) | `clone(CLONE_NEWUTS)`, `unshare(CLONE_NEWUSER)`, `unshare(CLONE_NEWNS)` |
//! | user-mode linux guest | ptrace EPERM (E-50) | `ptrace(PTRACE_TRACEME)` |
//! | uid_map identity switch | map unwritable, switch EPERM (E-70) | `setuid(1000)`, `setgroups(0,NULL)` |
//! | guest file over the ceiling | SIGXFSZ past RLIMIT_FSIZE (E-34) | `prlimit(RLIMIT_FSIZE)`, judged by T-1305's check |
//!
//! ⛔ A mechanism that is open on another runtime turns a non-goal back into
//! a goal, so a verdict is per run, never a constant: `Refused` where the
//! mechanism is denied, `Open` where it works here, `Unestablished` where
//! the rows cannot say. Cross-run staleness is bounded by the probe cache
//! key (`TODO/probe.md` T-0111), like every other verdict in the document.

use crate::verdict::Verdict;
use crate::Findings;

/// What one run established about one non-goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance {
    /// The mechanism is denied here: podbox will not attempt this.
    Refused,
    /// The mechanism works here: the non-goal is off on this machine.
    Open,
    /// The cited rows cannot say: missing, skipped, or unreadable.
    Unestablished,
}

/// One non-goal with its verdict.
pub struct NonGoal {
    pub name: &'static str,
    /// The spec experiment that measured the mechanism, e.g. `E-75`.
    pub spec: &'static str,
    /// What to do instead, from the spec where it names one.
    pub remedy: &'static str,
    pub stance: Stance,
    /// The legs and errnos behind the stance, or why it cannot be told.
    pub detail: String,
}

/// The table: name, spec tag, remedy, and the probe rows that decide it.
/// The file-ceiling row is judged separately (it needs the numeric limit,
/// not only the leg's verdict); every other row is refused iff any cited
/// row is denied.
struct Def {
    name: &'static str,
    spec: &'static str,
    remedy: &'static str,
    rows: &'static [&'static str],
}

const DEFS: &[Def] = &[
    Def {
        name: "tcp listener",
        spec: "E-75",
        remedy:
            "no TCP listener here; guests talk UDP hostfwd, control over unix sockets and FIFOs",
        rows: &["bind(127.0.0.1:0)+listen"],
    },
    Def {
        name: "kvm acceleration",
        spec: "E-30/60",
        remedy: "no KVM here; the machine tier runs TCG",
        rows: &["open(/dev/kvm, O_RDWR)"],
    },
    Def {
        name: "runc-style oci",
        spec: "E-43",
        remedy: "namespaces denied here; podbox runs the chroot rung instead",
        rows: &[
            "clone(CLONE_NEWUTS)",
            "unshare(CLONE_NEWUSER)",
            "unshare(CLONE_NEWNS)",
        ],
    },
    Def {
        name: "user-mode linux guest",
        spec: "E-50",
        remedy: "ptrace denied here; the machine tier runs qemu TCG instead",
        rows: &["ptrace(PTRACE_TRACEME)"],
    },
    Def {
        name: "uid_map identity switch",
        spec: "E-70",
        remedy: "switch ids outside the container; inside, the interposer memo answers (T-0711)",
        rows: &["setuid(1000)", "setgroups(0,NULL)"],
    },
    Def {
        name: "guest file over the ceiling",
        spec: "E-34",
        remedy: "size guests under the ceiling (--podbox-mem, T-1305)",
        rows: &["prlimit(RLIMIT_FSIZE)"],
    },
];

/// Read the cited rows back: the denied legs with their errnos, and the
/// legs that cannot say.
fn cite(f: &Findings, rows: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut denied = Vec::new();
    let mut unknown = Vec::new();
    for name in rows {
        match f.get(name) {
            Some(o) => match (o.verdict, o.errno) {
                (Verdict::Ok, _) => {}
                (Verdict::Denied, Some(e)) => denied.push(format!("{name}={}", e.name())),
                (Verdict::Denied, None) => denied.push(format!("{name}=denied")),
                (Verdict::Skip, _) if o.reason.is_empty() => {
                    unknown.push(format!("{name}=skip"));
                }
                (Verdict::Skip, _) => unknown.push(format!("{name}=skip: {}", o.reason)),
            },
            None => unknown.push(format!("{name}=(not probed)")),
        }
    }
    (denied, unknown)
}

fn refused(def: &Def, denied: &[String]) -> NonGoal {
    NonGoal {
        name: def.name,
        spec: def.spec,
        remedy: def.remedy,
        stance: Stance::Refused,
        detail: format!("{} refused: {}; {}", def.name, denied.join(" "), def.remedy),
    }
}

fn open(def: &Def, rows: &[&str]) -> NonGoal {
    NonGoal {
        name: def.name,
        spec: def.spec,
        remedy: def.remedy,
        stance: Stance::Open,
        detail: format!("{} open here: {} permitted", def.name, rows.join(" ")),
    }
}

fn unestablished(def: &Def, unknown: &[String]) -> NonGoal {
    NonGoal {
        name: def.name,
        spec: def.spec,
        remedy: def.remedy,
        stance: Stance::Unestablished,
        detail: format!("{} unestablished: {}", def.name, unknown.join(" ")),
    }
}

/// Assess every non-goal against one finished run.
pub fn assess(f: &Findings) -> Vec<NonGoal> {
    let mut out = Vec::with_capacity(DEFS.len());
    for def in DEFS {
        if def.name == "guest file over the ceiling" {
            out.push(assess_ceiling(f, def));
            continue;
        }
        let (denied, unknown) = cite(f, def.rows);
        if !denied.is_empty() {
            out.push(refused(def, &denied));
        } else if !unknown.is_empty() {
            out.push(unestablished(def, &unknown));
        } else {
            out.push(open(def, def.rows));
        }
    }
    out
}

/// The ceiling row needs the numeric limit, not only the leg's verdict: a
/// finite ceiling refuses over-ceiling guests (judged by T-1305's check),
/// and an infinite one refuses nothing.
fn assess_ceiling(f: &Findings, def: &Def) -> NonGoal {
    let (denied, unknown) = cite(f, def.rows);
    if !unknown.is_empty() {
        return unestablished(def, &unknown);
    }
    if !denied.is_empty() {
        return unestablished(
            def,
            &[format!("the bound could not be read: {}", denied.join(" "))],
        );
    }
    match crate::sys::prlimit(crate::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) if cur != u64::MAX => NonGoal {
            name: def.name,
            spec: def.spec,
            remedy: def.remedy,
            stance: Stance::Refused,
            detail: format!(
                "{} refused: RLIMIT_FSIZE ceiling {cur} bytes, guests above it refused (T-1305); {}",
                def.name, def.remedy
            ),
        },
        Ok(_) => NonGoal {
            name: def.name,
            spec: def.spec,
            remedy: def.remedy,
            stance: Stance::Open,
            detail: format!(
                "{} open here: no per-file ceiling on this machine",
                def.name
            ),
        },
        Err(e) => unestablished(
            def,
            &[format!("the bound could not be read: {}", e.name())],
        ),
    }
}

/// The word the report prints and the document carries.
pub fn word(s: Stance) -> &'static str {
    match s {
        Stance::Refused => "refused",
        Stance::Open => "open",
        Stance::Unestablished => "unestablished",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys;
    use crate::verdict::Outcome;

    fn findings(rows: &[(&'static str, Outcome)]) -> Findings {
        Findings {
            rows: rows.to_vec(),
            ..Findings::empty()
        }
    }

    fn all_ok() -> Vec<(&'static str, Outcome)> {
        DEFS.iter()
            .flat_map(|d| d.rows.iter())
            .map(|&name| (name, Outcome::ok()))
            .collect()
    }

    fn goal<'a>(rows: &'a [NonGoal], name: &str) -> &'a NonGoal {
        rows.iter()
            .find(|g| g.name == name)
            .expect("a row per non-goal")
    }

    #[test]
    fn six_non_goals_come_back_in_order() {
        let goals = assess(&findings(&all_ok()));
        assert_eq!(goals.len(), 6);
        assert_eq!(goals[0].name, "tcp listener");
        assert_eq!(goals[5].name, "guest file over the ceiling");
    }

    #[test]
    fn a_denied_mechanism_refuses_naming_its_errno() {
        let mut rows = all_ok();
        for (n, o) in rows.iter_mut() {
            if *n == "bind(127.0.0.1:0)+listen" {
                *o = Outcome::denied(sys::EPERM);
            }
        }
        let goals = assess(&findings(&rows));
        let g = goal(&goals, "tcp listener");
        assert_eq!(g.stance, Stance::Refused);
        assert!(
            g.detail.contains("bind(127.0.0.1:0)+listen=EPERM"),
            "{}",
            g.detail
        );
        assert!(g.detail.contains("UDP"), "{}", g.detail);
    }

    #[test]
    fn a_permitted_mechanism_turns_the_non_goal_off() {
        // The spec's port rule flipped mid-session, so a permitted bind is
        // the live case, not a contradiction: the refusal goes away.
        let goals = assess(&findings(&all_ok()));
        let g = goal(&goals, "tcp listener");
        assert_eq!(g.stance, Stance::Open);
        assert!(!g.detail.contains("refused"), "{}", g.detail);
    }

    #[test]
    fn a_missing_row_is_unestablished_rather_than_any_stance() {
        // A document written before the bind leg existed: five rows cited,
        // the sixth never run. Its absence establishes nothing.
        let mut rows = all_ok();
        rows.retain(|(n, _)| *n != "bind(127.0.0.1:0)+listen");
        let goals = assess(&findings(&rows));
        let g = goal(&goals, "tcp listener");
        assert_eq!(g.stance, Stance::Unestablished);
        assert!(g.detail.contains("(not probed)"), "{}", g.detail);
    }

    #[test]
    fn the_ceiling_row_names_its_number_or_says_there_is_none() {
        // Whatever machine runs the suite, the row carries a number or the
        // absence of one, and never a bare verdict.
        let goals = assess(&findings(&all_ok()));
        let g = goal(&goals, "guest file over the ceiling");
        match crate::sys::prlimit(crate::sys::RLIMIT_FSIZE) {
            Ok((cur, _)) if cur != u64::MAX => {
                assert_eq!(g.stance, Stance::Refused);
                assert!(g.detail.contains(&cur.to_string()), "{}", g.detail);
            }
            Ok(_) => {
                assert_eq!(g.stance, Stance::Open);
                assert!(g.detail.contains("no per-file ceiling"), "{}", g.detail);
            }
            Err(_) => assert_eq!(g.stance, Stance::Unestablished),
        }
    }
}
