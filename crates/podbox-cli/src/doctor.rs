//! `podbox doctor`: the setup half beside `podbox probe`'s measurement half.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-1337 (issue 38). `probe` prints
//! legs, verdicts and refusals and never says how to fix one: a QEMU too
//! old for migration, firmware present or absent, helper tools present, an
//! image installed, disk free against the ceiling. Doctor reuses the
//! machine legs with the T-1301 checks and prints one fix line per
//! missing piece.
//!
//! ⭐ The shape is copied from `references/carlbomsdata__winquick`
//! `tree/src/facts.rs:125-180` (Status, Check, Doctor with the ordered
//! fix list, ok/note/fail builder) and none of its content: check lines
//! with fix lines, and the health agreeing with the problem list.
//!
//! ⛔ Deliberately no firmware check (the tier boots `-kernel` directly;
//! nothing consumes firmware), no helper-tool checks (cpio and busybox
//! are experiment tooling; the runtime consumes none) and no
//! image-presence check. Each would be a check whose subject the
//! runtime never reads.
//!
//! Exits: 0 every check holds, 1 a check fails with its fix, 2 a
//! required leg was never measured so the assessment is incomplete.

use podbox_probe::machine::{self, Assessment};
use podbox_probe::verdict::Verdict;

pub const DOCTOR_USAGE: &str = "\
usage: podbox doctor

  Check what this machine can run and say how to fix what it cannot:
  the machine legs with one fix line per missing piece, and the disk
  free against the file-size ceiling. kvm or tun missing alone is a
  note, not a failure: that is the TCG profile, not a refusal
  (TODO/podvm.md T-1301, T-1306).

  exits 0 every check holds, 1 a check fails with its fix, 2 a
  required leg was never measured.
";

/// One check's answer. A note is measured and not a problem: kvm or
/// tun missing under TCG, no file-size ceiling, an unreadable limit
/// the legs already refuse. Only a fail carries a fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Note,
    Fail,
}

impl Status {
    fn word(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Note => "note",
            Status::Fail => "fail",
        }
    }
}

/// One line of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: String,
}

/// The report: check lines in run order, fix lines in the same order,
/// and the legs that were never measured, if any.
#[derive(Debug, Default)]
pub struct Doctor {
    pub checks: Vec<Check>,
    pub fixes: Vec<String>,
    pub unrunnable: Vec<&'static str>,
}

impl Doctor {
    pub fn ok(&mut self, name: &str, detail: String) {
        self.checks.push(Check {
            name: name.to_string(),
            status: Status::Ok,
            detail,
        });
    }

    pub fn note(&mut self, name: &str, detail: String) {
        self.checks.push(Check {
            name: name.to_string(),
            status: Status::Note,
            detail,
        });
    }

    pub fn fail(&mut self, name: &str, detail: String, fix: String) {
        self.checks.push(Check {
            name: name.to_string(),
            status: Status::Fail,
            detail,
        });
        self.fixes.push(fix);
    }

    /// True exactly where the assessment is incomplete: a required leg
    /// the findings never measured. The caller exits 2, because a fix
    /// list built on missing measurements is a guess.
    pub fn unrunnable(&mut self, name: &'static str) {
        self.unrunnable.push(name);
    }

    /// 2 the assessment is incomplete, 1 a check fails, 0 all hold.
    /// Health agrees with the problem list: a fail without a fix, or
    /// a fix without a fail, is a defect in the caller, asserted
    /// below.
    pub fn exit_code(&self) -> i32 {
        if !self.unrunnable.is_empty() {
            2
        } else if self.checks.iter().any(|c| c.status == Status::Fail) {
            1
        } else {
            0
        }
    }

    pub fn render(&self, profile: &str) -> String {
        let mut out = String::new();
        for c in &self.checks {
            out.push_str(&format!("{} {}: {}\n", c.status.word(), c.name, c.detail));
        }
        for f in &self.fixes {
            out.push_str(&format!("fix: {f}\n"));
        }
        if !self.unrunnable.is_empty() {
            out.push_str(&format!(
                "unmeasured: {} (the probe did not measure them; re-run)\n",
                self.unrunnable.join(", "),
            ));
        }
        out.push_str(&format!("profile: {profile}\n"));
        out
    }
}

/// `podbox doctor`.
pub fn doctor(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, DOCTOR_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, DOCTOR_USAGE) {
        return c;
    }
    // ⭐ The `logout` shape (TODO/cli.md T-0209): flags collect and the
    // verb decides after the loop.
    let mut help = false;
    for a in args {
        match a.as_str() {
            "-h" | "--help" => help = true,
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, DOCTOR_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => {
                eprintln!("podbox doctor: unknown option {other:?}");
                eprint!("{DOCTOR_USAGE}");
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
        }
    }
    if help {
        print!("{DOCTOR_USAGE}");
        return 0;
    }
    let findings = podbox_probe::run();
    let assessed = machine::assess(&findings);
    let profile = match assessed.profile {
        Some(machine::Profile::Full) => "full",
        Some(machine::Profile::Tcg) => "tcg",
        None => "refused",
    };
    let mut report = Doctor::default();
    examine_legs(&assessed, &mut report);
    examine_ceiling(&mut report);
    print!("{}", report.render(profile));
    report.exit_code()
}

/// Every machine leg as a check line: required legs fail with one fix
/// each, kvm and tun note their absence under TCG, the rest report.
///
/// ⛔ kvm or tun missing alone never fails: that is the TCG profile
/// (TODO/podvm.md T-1301), and failing it would refuse a machine the
/// tier runs on. A required leg the findings never measured is
/// unrunnable rather than failed: a fix for an unmeasured leg is a
/// guess (TODO/cli.md T-1337).
fn examine_legs(assessed: &Assessment, report: &mut Doctor) {
    for leg in &assessed.legs {
        let Some(outcome) = &leg.outcome else {
            if assessed.tcg_blocked_by.contains(&leg.name) {
                report.unrunnable(leg.name);
            } else {
                report.note(leg.name, "not probed".to_string());
            }
            continue;
        };
        match outcome.verdict {
            Verdict::Ok => report.ok(leg.name, leg.detail()),
            _ if leg.name == machine::KVM_LEG => report.note(
                leg.name,
                format!(
                    "{}: guests run under TCG without hardware acceleration",
                    leg.detail()
                ),
            ),
            _ if leg.name == machine::TUN_LEG => report.note(
                leg.name,
                format!("{}: guests use user-mode networking", leg.detail()),
            ),
            _ => report.fail(leg.name, leg.detail(), fix_for(leg.name, leg.detail())),
        }
    }
}

/// The remedy for one failed required leg. Each names the command that
/// must answer and what it must say: a fix that names a package
/// manager would be wrong on half the machines this runs on, and a
/// fix that restates the failure is not a fix.
fn fix_for(name: &str, detail: String) -> String {
    if name == machine::EMU_LEG {
        return "install QEMU so that `qemu-system-x86_64 --version` prints a version".to_string();
    }
    if name == machine::ACCEL_LEG {
        return "install QEMU so that `qemu-system-x86_64 -accel help` lists an accelerator"
            .to_string();
    }
    if name == machine::FSIZE_LEG {
        return format!("prlimit(RLIMIT_FSIZE) must be readable: {detail}");
    }
    if name == machine::SPACE_LEG {
        return format!("statfs must answer for the working directory: {detail}");
    }
    format!("{name} must hold: {detail}")
}

/// The disk free against the file-size ceiling: a guest over the
/// ceiling dies with SIGXFSZ (TODO/podvm.md T-1301 leg 3), so free
/// room below it is a setup failure with a fix, not a note.
///
/// Measured here rather than read from the legs: the fsize leg
/// reports the bound and the space leg reports the room, and neither
/// compares the two. No ceiling (infinity) is a note. An unreadable
/// limit or room is a note too: the legs already refuse those, and a
/// second failure for one cause is noise.
fn examine_ceiling(report: &mut Doctor) {
    const NAME: &str = "disk free against the file-size ceiling";
    let at = match podbox_image::open_store() {
        Ok(s) => s.root().display().to_string(),
        Err(_) => ".".to_string(),
    };
    let max = match podbox_probe::sys::prlimit(podbox_probe::sys::RLIMIT_FSIZE) {
        Ok((_cur, max)) => max,
        Err(e) => {
            report.note(
                NAME,
                format!(
                    "RLIMIT_FSIZE could not be read ({}); the legs say whether it matters",
                    e.name()
                ),
            );
            return;
        }
    };
    if max == u64::MAX {
        report.note(
            NAME,
            "no file-size ceiling (RLIMIT_FSIZE is infinity)".to_string(),
        );
        return;
    }
    let free = match podbox_image::space::read(&at) {
        Ok(have) => have.free.bytes,
        Err(e) => {
            report.note(NAME, format!("free space at {at} could not be read: {e}"));
            return;
        }
    };
    if free < max {
        report.fail(
            NAME,
            format!(
                "free {} at {at} is below the {} file-size ceiling",
                podbox_image::space::mib(free),
                podbox_image::space::mib(max)
            ),
            format!("free disk space at {at} or point PODBOX_STORE at a roomier filesystem"),
        );
    } else {
        report.ok(
            NAME,
            format!(
                "free {} at {at} covers the {} file-size ceiling",
                podbox_image::space::mib(free),
                podbox_image::space::mib(max)
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_usage_is_plain_ascii() {
        // TODO/cli.md T-1336: usages print on a constrained host.
        let bad = DOCTOR_USAGE.bytes().filter(|b| *b > 0x7F).count();
        assert_eq!(bad, 0, "doctor usage carries {bad} non-ASCII bytes");
    }

    /// The winquick shape, held: a fail carries exactly one fix, health
    /// agrees with the problem list, and the incomplete assessment
    /// outranks a failed one.
    #[test]
    fn health_agrees_with_the_problem_list() {
        let mut d = Doctor::default();
        assert_eq!(d.exit_code(), 0);
        d.ok("emu", "qemu=ok".to_string());
        d.note("kvm", "missing under tcg".to_string());
        assert_eq!(d.exit_code(), 0);
        d.fail("accel", "denied".to_string(), "install QEMU".to_string());
        assert_eq!(d.exit_code(), 1);
        let text = d.render("tcg");
        assert!(text.contains("fail accel: denied\n"));
        assert!(text.contains("fix: install QEMU\n"));
        assert!(text.contains("profile: tcg\n"));
        assert_eq!(d.fixes.len(), 1);
        d.unrunnable("prlimit(RLIMIT_FSIZE)");
        assert_eq!(d.exit_code(), 2);
        assert!(d.render("refused").contains("unmeasured: "));
    }

    /// TODO/cli.md T-1337: doctor decides its arguments before it
    /// probes, so `--help` prints and a bad flag refuses hermetic.
    #[test]
    fn doctor_help_and_bad_flag_need_no_probe() {
        assert_eq!(doctor("doctor", &["--help".to_string()]), 0);
        assert_eq!(
            doctor("doctor", &["--bogus".to_string()]),
            podbox_image::error::EXIT_FLAG_ERROR
        );
        assert_eq!(
            doctor("doctor", &["some-thing".to_string()]),
            podbox_image::error::EXIT_FLAG_ERROR
        );
    }
}
