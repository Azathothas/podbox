//! `podbox windows doctor`: what this machine would do, and nothing else.
//! `TODO/milestones.md` T-1112.

use podbox_windows::Accel;

use super::args::Args;
use super::plan;

/// Report the profile, the accelerator and the paths a run would use — and
/// refuse, at the runtime-error code, where a run would refuse.
///
/// ⛔ **`doctor` must not answer where `run` would refuse.** A diagnostic
/// that prints a green answer on a machine the driver then declines is
/// worse than no diagnostic: it is the claim that the guest works, made by
/// the one command a caller runs to find out. So this shares
/// [`plan::accelerator`] with `run` rather than repeating the decision.
pub(crate) fn doctor(a: &Args) -> i32 {
    let _ = a;
    let accel = match plan::accelerator() {
        Ok(a) => a,
        Err(c) => return c,
    };
    let findings = podbox_probe::run();
    let assessed = podbox_probe::machine::assess(&findings);
    println!(
        "machine profile: {}",
        assessed
            .profile
            .map(podbox_probe::machine::Profile::word)
            .unwrap_or("none")
    );
    println!("accelerator: {} (cpu {})", accel.word(), accel.cpu());
    if accel == Accel::Tcg {
        println!(
            "tcg: the emulator process is the boundary, not hardware isolation. \
             This is the portable profile and it is slower; it is not a refusal."
        );
    }
    println!("emulator: {}", plan::emulator().display());
    println!("qemu-img: {}", plan::qemu_img().display());
    println!("firmware share: {}", plan::share().display());
    let (code, vars) = plan::firmware();
    println!("ovmf code: {}", code.display());
    println!("ovmf vars: {}", vars.display());
    let base = podbox_windows::base_cache();
    println!(
        "base image: {} ({})",
        base.display(),
        if base.is_file() {
            "present"
        } else {
            "absent; `podbox windows fetch` downloads one, `podbox windows \
             setup` installs the agent into it"
        }
    );
    0
}
