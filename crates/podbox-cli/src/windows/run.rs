//! One boot, one command, and the guest's own exit code.
//! `TODO/milestones.md` T-1112.

use std::time::Duration;

use podbox_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podbox_windows::{Plan, Request};

use super::args::{Args, USAGE};
use super::{plan, resolve_image};

/// The default wait for a boot that has to reach the mailbox, run one line
/// and power itself off. Measured at 28 s on a warm overlay under `tcg`, so
/// the margin is for a cold boot and a slow command rather than for luck.
pub(crate) const DEFAULT_TIMEOUT: u64 = 600;

/// `podbox windows run`, and the guest half of `podbox run`.
///
/// ⛔ **The exit status is the guest's, and a timeout is never a zero.** A
/// guest that ran the command reports its own status; `0..=124` passes
/// through, because that is the range docker reserves for a container's own
/// status. A guest status of 125 or more cannot be passed through without
/// reading as podbox itself failing, so it is capped and named out loud
/// rather than silently remapped.
///
/// ⚠ **`RLIMIT_FSIZE` does not gate this, and saying why matters.** The
/// Linux tiers hit that ceiling with a guest memory backing file
/// (`TODO/podvm.md` T-1305); here QEMU's guest memory is anonymous and the
/// overlay is sparse, so the only file that must fit is the base image the
/// caller already has, and the ceiling binds where a file really is written
/// — in [`super::setup::fetch`], whose download is one.
pub(crate) fn run(verb: &str, a: &Args) -> i32 {
    let image = match resolve_image(a.image.as_deref()) {
        Ok(p) => p,
        Err(c) => return c,
    };
    if !image.is_file() {
        eprintln!(
            "podbox {verb}: {} is not a file\n{USAGE}",
            image.display()
        );
        return EXIT_FLAG_ERROR;
    }
    let plan = match plan::build(a, &image) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let command = if a.command.trim().is_empty() {
        "ver".to_string()
    } else {
        a.command.clone()
    };
    let request = Request {
        command,
        token: podbox_windows::agent::nonce(),
        // 0 is docker's "no timeout": a caller who says it means the guest
        // may take as long as it takes, and a finite default is still the
        // caller's to override.
        timeout: match a.timeout.unwrap_or(DEFAULT_TIMEOUT) {
            0 => Duration::from_secs(u32::MAX as u64),
            n => Duration::from_secs(n),
        },
    };
    report(&plan, &request, &image);
    // ⭐ The per-run directory is about a hundred megabytes and is not the
    // caller's to remember: it goes unless `PODBOX_WINDOWS_KEEP` says this
    // is the run somebody needs to look at.
    let kept = std::env::var_os("PODBOX_WINDOWS_KEEP").is_some();
    let dir = plan.root.parent().map(|d| d.to_path_buf());
    // ⚠ The mailbox carries the agent text as well as the command, so the
    // volume a run is read back from is self-describing: an operator who
    // dumps it sees the exact agent that was installed, not a stale copy.
    if let Err(e) = podbox_windows::stage(
        &plan::qemu_img(),
        &image,
        &plan,
        podbox_windows::agent::AGENT_CMD,
        &request,
    ) {
        eprintln!("podbox {verb}: {e}");
        podbox_windows::cleanup(&plan);
        return EXIT_RUNTIME_ERROR;
    }
    match podbox_windows::run(&plan, &request) {
        Ok(out) => {
            use std::io::Write;
            let _ = std::io::stdout().write_all(&out.stdout);
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().write_all(&out.stderr);
            after(&plan, kept, dir.as_deref());
            match podbox_windows::cap_status(out.code) {
                Some(c) => c,
                None => {
                    eprintln!(
                        "podbox {verb}: the guest exited {}, which is outside docker's \
                         0-124 range, so podbox reports {EXIT_RUNTIME_ERROR}",
                        out.code
                    );
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        Err(e) => {
            eprintln!("podbox {verb}: {e}");
            after(&plan, kept, dir.as_deref());
            EXIT_RUNTIME_ERROR
        }
    }
}

/// Remove the run's directory, or say where it was kept.
///
/// ⚠ Named after every exit, not only the good one: the run that needs
/// looking at is the one that failed, and a directory that survives a
/// failure by accident is how a hundred megabytes per attempt accumulates.
fn after(plan: &Plan, kept: bool, dir: Option<&std::path::Path>) {
    podbox_windows::cleanup(plan);
    if kept {
        if let Some(d) = dir {
            eprintln!("podbox windows: kept {} (PODBOX_WINDOWS_KEEP)", d.display());
        }
    }
}

/// One line of what was asked and where, before the boot.
///
/// ⚠ On stderr, so the guest's own stdout is never mixed with ours: a
/// caller who pipes the command's output has to get the command's output.
fn report(plan: &Plan, request: &Request, image: &std::path::Path) {
    eprintln!(
        "podbox windows: {} {} {:?} over {}, {} MiB x{} cpu, token {}",
        plan.accel.word(),
        plan.accel.cpu(),
        request.command,
        image.display(),
        plan.memory_mib,
        plan.cpus,
        request.token
    );
}
