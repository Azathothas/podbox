//! What the probe prints, and on which channel.
//!
//! `TODO/probe.md` T-0108 and T-0110, and `TOOL.md` sections 6.8 and 6.9.
//!
//! ⛔ **The rung goes to stdout and the evidence to stderr.** A payload's
//! stdout is data to whatever consumes it, and a banner on it corrupts every
//! pipeline. The same split `pathshim` uses at
//! `references/compforge__pathshim/tree/src/main.rs:134-138`.
//!
//! ⛔ **No output may imply namespaces, cgroups or devices exist when they do
//! not.** Every field below is derived from a row that ran, and a row that did
//! not run prints as a skip with its reason rather than as an absence.

use crate::identity::Identity;
use crate::mounts::Mounts;
use crate::select::{banner_cell, Provides, Rung, Selection, BANNER_ROWS};
use crate::verdict::Verdict;
use crate::{json, Findings};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every probe row, in the reference harness's own format, so a podvm run and
/// a `verification/probe` run diff against each other line by line.
pub fn rows(f: &Findings) -> String {
    let mut out = String::new();
    for (name, o) in &f.rows {
        out.push_str(&o.row(name));
        out.push('\n');
    }
    out
}

/// `TOOL.md` section 6.8's banner. Two lines of mode, then the probe verdicts
/// that decided it.
pub fn banner(f: &Findings, sel: &Selection) -> String {
    entry_banner(f, sel, sel.rung)
}

/// The same banner for a caller that ENTERS, which is a different fact from the
/// one the prober measured.
///
/// ⭐ **`entered` is the rung the entry sequence actually implements**, and it
/// is what every field is derived from. [`TODO/cli.md`](../../../TODO/cli.md)
/// T-0804 rule 4: no output may imply namespaces, cgroups or devices exist when
/// they do not. Measured on 2026-09-09: this host's probe selects `namespace`
/// and `podvm_enter` performs a plain `chroot` on every machine, so
/// `podvm run` printed `mode=namespace (namespaces: as configured; mounts:
/// full)` for a payload that had neither.
///
/// ⚠ Where the two differ the machine's own answer is printed beside the mode
/// rather than dropped: "what podvm did" and "what this machine would permit"
/// are two facts, and a caller deciding whether to move the workload needs the
/// second.
pub fn entry_banner(f: &Findings, sel: &Selection, entered: Rung) -> String {
    let p = Provides::of(entered, f);
    let mut out = format!(
        "podvm {VERSION}: mode={} (namespaces: {}; mounts: {}; devices: {};\n\
         ownership: {}; network: {}; pids: {})\n",
        entered.word(),
        p.namespaces,
        p.mounts,
        p.devices,
        p.ownership,
        p.network,
        p.pids
    );
    let cells: Vec<String> = BANNER_ROWS.iter().map(|n| banner_cell(f, n)).collect();
    out.push_str("probes: ");
    out.push_str(&cells[..cells.len().min(4)].join(" "));
    out.push('\n');
    if cells.len() > 4 {
        out.push_str("        ");
        out.push_str(&cells[4..].join(" "));
        out.push('\n');
    }
    if !entered.must_never_claim().is_empty() {
        out.push_str(&format!(
            "this mode does NOT provide: {}\n",
            entered.must_never_claim()
        ));
    }
    // TODO/supervise.md T-0606. Where syscall mediation is refused but
    // the lifecycle still tracks, the banner says so: supervision
    // without mediation is the fallback, and a silent one would be the
    // Continue-style false success the entry refuses. Where mediation
    // holds there is no fallback to state, so the line stays off.
    if crate::supervise::assess(f).refused() && !crate::supervise::assess_supervision(f).refused() {
        out.push_str(
            "supervision tracks the payload through pidfd and waitid; \
             no syscall mediation on this machine\n",
        );
    }
    // TODO/complete.md T-0413 part 1. The chroot entry sequence mounts
    // nothing, so no /proc is mounted in the payload root. The failure this
    // produces names /dev/fd rather than /proc (measured under T-0412:
    // `/dev/fd/62: No such file or directory`), so the banner names the
    // filesystem and the casualty before the payload meets either.
    // T-0413 emulation: the interposer answers pipe descriptors, the
    // resolved exe path and mount-table reads, so those are named as
    // emulated and only the rest stays a casualty.
    // ⭐ TODO/enter.md T-1339: the namespace rung mounts a tmpfs on
    // `/tmp` and no /proc, so the sentence holds there too.
    if entered == Rung::Chroot || entered == Rung::Namespace {
        out.push_str(
            "no /proc is mounted in the payload root: the interposer emulates \
             /proc/self/fd pipe descriptors, /proc/self/exe and the mount-table \
             files where exactly answerable (counted under Interpose.Emulated.procfs); \
             anything else under /proc fails there, and the error names /dev/fd rather \
             than /proc\n",
        );
    }
    // ⛔ Said, not dropped. A machine that would permit more than podvm took
    // is a fact the caller acts on, and printing only the mode would read as
    // "this is all this machine can do".
    if entered != sel.rung {
        out.push_str(&format!(
            "note: this machine would permit `{}`; podvm's entry sequence is `{}` and \
             creates no namespace and mounts nothing\n",
            sel.rung.word(),
            entered.word()
        ));
    }
    // ⛔ TODO/enter.md T-0506 point 5. Where an emulator produced these rows,
    // the banner says so HERE, beside the rung, rather than further down: the
    // rung is the line a caller acts on, and a rung measured by qemu is a
    // statement about qemu.
    if let Some(note) = crate::interpreter().note() {
        out.push_str(&note);
        out.push('\n');
    }
    out
}

/// The full stderr block: the banner, then what the diagnostic of `TOOL.md`
/// section 6.9 needs to name the mechanism and the remedy rather than an errno
/// on its own.
pub fn evidence(f: &Findings, sel: &Selection) -> String {
    let mut out = banner(f, sel);

    out.push_str("\nidentity\n");
    out.push_str(&identity_block(&f.identity));

    let m = Mounts::of(f);
    out.push_str("\nmounts, as four verdicts and not one\n");
    for (half, o) in m.rows() {
        out.push_str(&format!("  {half:<10} {}\n", short(o)));
    }
    if let Some(note) = m.capability_note() {
        out.push_str(&format!("  {note}\n"));
    }

    out.push_str("\nwritable, by writing\n");
    for w in &f.writable {
        let space = match (&w.space, w.space_error) {
            (Some(s), _) => format!(
                "{} blocks of {} B free, {} inodes free",
                s.blocks_free, s.block_size, s.inodes_free
            ),
            (None, Some(e)) => format!("statfs: {} ({})", e.name(), e.0),
            (None, None) => "statfs: not attempted".to_string(),
        };
        out.push_str(&format!(
            "  {:<26} {:<8} {space}  [{}]\n",
            w.path,
            short(&w.outcome),
            w.source
        ));
    }

    out.push_str("\ncontrols for the bogus-argument discriminator\n");
    for name in crate::select::control_rows() {
        match f.get(name) {
            Some(o) => out.push_str(&format!("  {}\n", o.row(name))),
            None => out.push_str(&format!("  {name:<34} (not probed)\n")),
        }
    }
    if sel.controls_answered {
        out.push_str("  the discriminator is still separating a filter from a policy\n");
    } else {
        for note in &sel.control_notes {
            out.push_str(&format!("  refused: {note}\n"));
        }
        out.push_str(
            "  refused: a probe whose control has stopped answering has stopped\n     \
             discriminating. Read the mechanism attributions below as unconfirmed.\n",
        );
    }

    if let Some(note) = spawn_wall_note(f.get("clone(CLONE_NEWNS)"), f.get("setgroups(0,NULL)")) {
        out.push_str(&note);
    }

    if !sel.rejected.is_empty() {
        out.push_str("\nrungs considered and not reached\n");
    }
    for r in &sel.rejected {
        out.push_str(&format!(
            "  {:<12} needs {}\n",
            r.rung.word(),
            r.requirement
        ));
        out.push_str(&format!("  {:<12}   got  {}\n", "", r.evidence));
    }

    let skipped: Vec<&(&str, crate::verdict::Outcome)> = f
        .rows
        .iter()
        .filter(|(_, o)| o.verdict == Verdict::Skip)
        .collect();
    if !skipped.is_empty() {
        out.push_str(
            "\ncould not run. refused: None of these is a denial, and none of them was\n\
             counted as one when the rung above was chosen.\n",
        );
        for (name, o) in skipped {
            out.push_str(&format!("  {name}\n      {}\n", o.reason));
        }
    }

    out.push_str(&format!(
        "\nprobe children were re-executed from {}\n",
        f.self_exe
    ));
    out.push_str(&machine_block(f));
    out.push_str(&supervise_block(f));
    out.push_str(&non_goals_block(f));
    out
}

/// TODO/machine.md T-1301's legs, one per fact, with the tier's profile or
/// its refusal. The verdicts are the rows that ran; a profile names what
/// runs, and the refusal names the legs blocking every profile.
fn machine_block(f: &Findings) -> String {
    let mach = crate::machine::assess(f);
    let mut out = String::from("\nmachine, one leg per fact\n");
    for leg in &mach.legs {
        match &leg.outcome {
            Some(o) => out.push_str(&format!("  {}\n", o.row(leg.name))),
            None => out.push_str(&format!("  {name:<34} (not probed)\n", name = leg.name)),
        }
    }
    match mach.profile {
        Some(crate::machine::Profile::Full) => out.push_str("  the tier holds: every leg ok\n"),
        Some(crate::machine::Profile::Tcg) => {
            // ⛔ The TCG boundary is the emulator process, never hardware
            // isolation, and tun decides the network shape. Both are said
            // here, on every TCG run, not just where they are missing.
            let net = if mach.tun_ok() {
                "/dev/net/tun holds"
            } else {
                "no /dev/net/tun: user-mode networking"
            };
            out.push_str(&format!(
                "  profile: tcg (the emulator process is the boundary, not hardware isolation; {net})\n"
            ));
        }
        // The refusal of pod-hostprobe holds its verdict ("machine tier
        // refused: LEG; LEG"), so the row adds no word before it (C337).
        None => match mach.refusal() {
            Some(r) => out.push_str(&format!("  {r}\n")),
            None => out.push_str("  machine tier refused\n"),
        },
    }
    out
}

/// TODO/supervise.md T-0606's legs, one per fact, with the tier's refusal
/// where any leg is missing. The shape is the machine block's above it: the
/// verdicts are the rows that ran, and the refusal names the legs that did
/// not hold, so a missing leg reads as a named refusal and never as a
/// silent pass.
fn supervise_block(f: &Findings) -> String {
    let sup = crate::supervise::assess(f);
    let mut out = String::from("\nsupervise, one leg per fact\n");
    for leg in &sup.legs {
        match &leg.outcome {
            Some(o) => out.push_str(&format!("  {}\n", o.row(leg.name))),
            None => out.push_str(&format!("  {name:<34} (not probed)\n", name = leg.name)),
        }
    }
    // As in the machine block: the refusal holds its verdict (C337).
    match sup.refusal() {
        Some(r) => out.push_str(&format!("  {r}\n")),
        None => out.push_str("  the tier holds: every leg ok\n"),
    }
    // TODO/supervise.md T-0606. Supervision beside mediation, one writer
    // of the section: the pair the lifecycle runs on, with its own
    // refusal where the launcher cannot track.
    let vis = crate::supervise::assess_supervision(f);
    out.push_str("\nsupervision, one leg per fact\n");
    for leg in &vis.legs {
        match &leg.outcome {
            Some(o) => out.push_str(&format!("  {}\n", o.row(leg.name))),
            None => out.push_str(&format!("  {name:<34} (not probed)\n", name = leg.name)),
        }
    }
    match vis.refusal() {
        Some(r) => out.push_str(&format!("  {r}\n")),
        None => out.push_str("  supervision holds: every leg ok\n"),
    }
    out
}
/// specification tried and this machine may or may not refuse. A refusal
/// names its leg and errno with the remedy; an open mechanism says so with
/// no refusal language; what the rows cannot say is unestablished rather
/// than either.
fn non_goals_block(f: &Findings) -> String {
    let mut out = String::from("\nnon-goals, one stance per blocked design\n");
    for g in crate::nongoals::assess(f) {
        match g.stance {
            crate::nongoals::Stance::Refused => {
                out.push_str(&format!("  refused: {}\n", g.detail));
            }
            crate::nongoals::Stance::Open => {
                out.push_str(&format!("  {}\n", g.detail));
            }
            crate::nongoals::Stance::Unestablished => {
                out.push_str(&format!("  ? {}\n", g.detail));
            }
        }
    }
    out
}

fn short(o: &crate::verdict::Outcome) -> String {
    match (o.verdict, o.errno) {
        (Verdict::Ok, _) => "ok".to_string(),
        (Verdict::Denied, Some(e)) => format!("denied {}", e.name()),
        (Verdict::Denied, None) => "denied".to_string(),
        (Verdict::Skip, Some(e)) => format!("skip ({})", e.name()),
        (Verdict::Skip, None) => "skip".to_string(),
    }
}

/// Wall 2 (`TODO/cli.md` T-0809): one string, two refused calls.
///
/// A Go payload that spawns a child fails `fork/exec <path>: operation not
/// permitted` whether the clone or the setgroups met the denial, so the note
/// names both legs with what this machine measured and tells the two apart
/// by the payload shape. It prints only where a leg is denied: two clear
/// legs mean the wall is absent here.
fn spawn_wall_note(
    clone: Option<&crate::verdict::Outcome>,
    setgroups: Option<&crate::verdict::Outcome>,
) -> Option<String> {
    fn leg(o: Option<&crate::verdict::Outcome>) -> (String, bool, bool) {
        match o {
            Some(x) => match x.verdict {
                Verdict::Denied => (short(x), true, false),
                Verdict::Ok => (short(x), false, true),
                Verdict::Skip => (short(x), false, false),
            },
            None => ("not measured".to_string(), false, false),
        }
    }
    let (c_text, c_denied, c_clear) = leg(clone);
    let (s_text, s_denied, s_clear) = leg(setgroups);
    if !c_denied && !s_denied {
        return None;
    }
    let mut out = String::from(
        "\nspawn, and the two calls behind one string\n  A Go payload that spawns a \
         child fails `fork/exec <path>: operation not permitted` whether the clone \
         or the setgroups met the denial.\n",
    );
    out.push_str(&format!(
        "  clone(CLONE_NEWNS) here: {c_text}. setgroups(0,NULL) here: {s_text}.\n"
    ));
    if c_denied {
        out.push_str("  A payload that sets clone flags meets the clone denial here.\n");
    } else if c_clear {
        out.push_str("  The clone is clear here, so a failing spawn did not meet it.\n");
    } else {
        out.push_str("  The clone leg was not measured, so no spawn is attributed to it.\n");
    }
    if s_denied {
        out.push_str(
            "  A payload that sets a credential meets the setgroups denial here; the \
             remedy is Credential{NoSetGroups: true}.\n",
        );
    } else if s_clear {
        out.push_str("  Setgroups is clear here, so a failing spawn did not meet it.\n");
    } else {
        out.push_str("  The setgroups leg was not measured, so no spawn is attributed to it.\n");
    }
    Some(out)
}

fn identity_block(id: &Identity) -> String {
    let mut out = format!("  uid={} gid={} groups={:?}\n", id.uid, id.gid, id.groups);
    for (label, v) in [
        ("/proc/self/uid_map", &id.uid_map),
        ("/proc/self/gid_map", &id.gid_map),
        ("/proc/self/setgroups", &id.setgroups),
    ] {
        match v {
            Some(s) => out.push_str(&format!("  {label}: {s}\n")),
            None => out.push_str(&format!("  {label}: unreadable\n")),
        }
    }
    for (label, v) in [
        ("CapEff", &id.cap_eff),
        ("Seccomp", &id.seccomp),
        ("Seccomp_filters", &id.seccomp_filters),
        ("NoNewPrivs", &id.no_new_privs),
        ("Threads", &id.threads),
    ] {
        match v {
            Some(s) => out.push_str(&format!("  {label}: {s}\n")),
            None => out.push_str(&format!("  {label}: unreadable\n")),
        }
    }
    if let Some(map) = &id.uid_map
        && crate::identity::is_single_id_map(map)
    {
        out.push_str(
            "  the uid map is a single range of one id. That is the fact that\n     \
                 explains every EINVAL from chown(2) and setuid(2) here: the id is\n     \
                 not mapped, so no capability makes it valid.\n",
        );
    }
    if id.groups.contains(&65534) {
        out.push_str(
            "  note: gid 65534 is overflowgid, which getgroups(2) returns for a group\n     \
             with no mapping. It is not evidence of a supplementary group.\n",
        );
    }
    for why in &id.unreadable {
        out.push_str(&format!("  note: not read: {why}\n"));
    }
    out
}

mod document;
pub use document::*;

fn none_if_empty(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `Rung` is re-exported here so a caller printing the stdout word does not
/// have to reach into `select`.
pub fn word(rung: Rung) -> &'static str {
    rung.word()
}

#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod tests;
