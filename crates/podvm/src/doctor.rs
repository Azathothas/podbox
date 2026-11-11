//! `podvm doctor`: the setup half beside `podvm probe`'s measurement half.
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

use podvm_probe::machine::{self, Assessment};
use podvm_probe::verdict::Verdict;

pub const DOCTOR_USAGE: &str = "\
usage: podvm doctor [store_exec]

  Check what this machine can run and say how to fix what it cannot:
  the machine legs with one fix line per missing piece, the disk free
  against the file-size ceiling, and whether a memfd executes
  (memfd_exec, TODO/enter.md T-1688). kvm or tun missing alone is a
  note, not a failure: that is the TCG profile, not a refusal
  (TODO/machine.md T-1301, T-1306).

  store_exec   print one row: whether files created under the store
               execute (`yes` or `no` with the detail). A `noexec`
               store refuses the first run before pulling (TODO/image.md
               T-1418), and this row is how a caller checks that first.

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

/// `podvm doctor`.
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
    // verb decides after the loop. One positional, `store_exec`, prints
    // the store-exec row alone (TODO/image.md T-1418); anything else is
    // refused as before.
    let mut help = false;
    let mut store_exec = false;
    for a in args {
        match a.as_str() {
            "-h" | "--help" => help = true,
            "store_exec" if !store_exec => store_exec = true,
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, DOCTOR_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => {
                eprintln!("podvm doctor: unknown option {other:?}");
                eprint!("{DOCTOR_USAGE}");
                return podvm_image::error::EXIT_FLAG_ERROR;
            }
        }
    }
    if help {
        print!("{DOCTOR_USAGE}");
        return 0;
    }
    if store_exec {
        return store_exec_row();
    }
    let findings = podvm_probe::run();
    let assessed = machine::assess(&findings);
    let profile = match assessed.profile {
        Some(machine::Profile::Full) => "full",
        Some(machine::Profile::Tcg) => "tcg",
        None => "refused",
    };
    let mut report = Doctor::default();
    examine_legs(&assessed, &mut report);
    examine_ceiling(&mut report);
    examine_memfd_exec(
        podvm_probe::probes::chroot_usable(&findings),
        podvm_enter::memfd::memfd_exec(),
        &mut report,
    );
    match podvm_image::open_store() {
        Ok(store) => examine_store_mode(store.root(), store.exposed_mode(), &mut report),
        Err(e) => report.note(
            "store_mode",
            format!("podvm cannot open its store to read the mode of its root: {e}"),
        ),
    }
    print!("{}", report.render(profile));
    report.exit_code()
}

/// TODO/supervise.md T-1715. The store root keeps the container table, and
/// the table keeps each `-e` value: a root that group or other can reach
/// gives them to each local user. A note, not a failure: podvm runs, and
/// the fix is one `chmod` of a directory that podvm did not create.
fn examine_store_mode(root: &std::path::Path, exposed: Option<u32>, report: &mut Doctor) {
    match exposed {
        None => report.ok(
            "store_mode",
            format!("{} is reachable by its owner only", root.display()),
        ),
        Some(mode) => report.note(
            "store_mode",
            format!(
                "{} is mode {mode:o}: other users can read the container table, which \
                 keeps each -e value. Run chmod 700 {} (TODO/supervise.md T-1715)",
                root.display(),
                root.display()
            ),
        ),
    }
}

/// TODO/image.md T-1418. The `store_exec` row alone: whether files
/// created under the store execute.
///
/// `yes` (0) runs the entry on; `no` (1) carries the probe's detail,
/// which names the directory and the `$PODVM_STORE` remedy; a probe
/// that cannot run (2) names its path and never reads as "no".
fn store_exec_row() -> i32 {
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm doctor: {e}");
            return 2;
        }
    };
    store_exec_row_at(store.root())
}

/// The pure-addressable half of [`store_exec_row`], over a directory
/// rather than the process store, so tests drive it without touching
/// process-global state.
fn store_exec_row_at(dir: &std::path::Path) -> i32 {
    match podvm_image::exec_probe::probe_store_exec(dir) {
        Ok(probe) => {
            println!(
                "store_exec: {}: {}",
                if probe.ok { "yes" } else { "no" },
                probe.detail
            );
            if probe.ok {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("podvm doctor: the store exec probe could not run: {e}");
            2
        }
    }
}
///
/// Every machine leg as a check line: required legs fail with one fix
/// each, kvm and tun note their absence under TCG, the rest report.
///
/// ⛔ kvm or tun missing alone never fails: that is the TCG profile
/// (TODO/machine.md T-1301), and failing it would refuse a machine the
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
/// ceiling dies with SIGXFSZ (TODO/machine.md T-1301 leg 3), so free
/// room below it is a setup failure with a fix, not a note.
///
/// Measured here rather than read from the legs: the fsize leg
/// reports the bound and the space leg reports the room, and neither
/// compares the two. No ceiling (infinity) is a note. An unreadable
/// limit or room is a note too: the legs already refuse those, and a
/// second failure for one cause is noise.
fn examine_ceiling(report: &mut Doctor) {
    const NAME: &str = "disk free against the file-size ceiling";
    let at = match podvm_image::open_store() {
        Ok(s) => s.root().display().to_string(),
        Err(_) => ".".to_string(),
    };
    // ⛔ T-1690 (issue 214): the kernel raises SIGXFSZ at the SOFT limit,
    // and every other reader (tier.rs, machine/ssh/qemu.rs, windows/plan.rs,
    // nongoals.rs, pod_hostprobe::space) judges the soft one. The hard limit
    // is only printed when it differs.
    let (soft, hard) = match podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE) {
        Ok(limits) => limits,
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
    if soft == u64::MAX {
        report.note(
            NAME,
            "no file-size ceiling (the soft RLIMIT_FSIZE is infinity)".to_string(),
        );
        return;
    }
    let free = match podvm_image::space::read(&at) {
        Ok(have) => have.free.bytes,
        Err(e) => {
            report.note(NAME, format!("free space at {at} could not be read: {e}"));
            return;
        }
    };
    match judge_ceiling(soft, hard, free, &at) {
        Ok(detail) => report.ok(NAME, detail),
        Err(detail) => report.fail(
            NAME,
            detail,
            format!("free disk space at {at} or point PODVM_STORE at a roomier filesystem"),
        ),
    }
}

/// The free room at `at` against the soft file-size ceiling `soft`. `Ok`
/// when the room covers the ceiling, `Err` when it is below. Both name the
/// ceiling, and the hard limit as well when it differs (T-1690).
fn judge_ceiling(soft: u64, hard: u64, free: u64, at: &str) -> Result<String, String> {
    let mib = podvm_image::space::mib;
    let ceiling = if hard == soft {
        format!("{} file-size ceiling", mib(soft))
    } else if hard == u64::MAX {
        format!(
            "{} file-size ceiling (soft; the hard limit is infinity)",
            mib(soft)
        )
    } else {
        format!(
            "{} file-size ceiling (soft; the hard limit is {})",
            mib(soft),
            mib(hard)
        )
    };
    if free < soft {
        Err(format!("free {} at {at} is below the {ceiling}", mib(free)))
    } else {
        Ok(format!("free {} at {at} covers the {ceiling}", mib(free)))
    }
}

/// TODO/enter.md T-1688. The row `memfd_exec`: whether a memfd executes
/// through `execveat(AT_EMPTY_PATH)`, read from the one probe that the
/// ladder and the entry check read, so the row and a refusal cannot
/// disagree.
///
/// ⚠ A denial fails only where chroot(2) is denied too: there the static
/// no-chroot family needs the memfd. Where chroot holds, only a forced
/// `PODVM_MODE=memfd` needs it, so the denial is a note with its fix in
/// the detail, as kvm missing is a note under TCG. An unmeasured probe is
/// a note: it never reads as "no".
fn examine_memfd_exec(chroot_ok: bool, probe: &podvm_enter::memfd::MemfdExec, report: &mut Doctor) {
    use podvm_enter::memfd::probe::{answer, Standing};
    const NAME: &str = "memfd_exec";
    let a = answer(probe);
    match (a.standing, a.fix) {
        (Standing::Holds, _) => report.ok(NAME, a.detail),
        (Standing::Denied, Some(fix)) if !chroot_ok => report.fail(NAME, a.detail, fix),
        (Standing::Denied, Some(fix)) => report.note(
            NAME,
            format!(
                "{}; chroot(2) holds, so only PODVM_MODE=memfd needs it; fix: {fix}",
                a.detail
            ),
        ),
        _ => report.note(NAME, a.detail),
    }
}

#[cfg(test)]
mod tests;
