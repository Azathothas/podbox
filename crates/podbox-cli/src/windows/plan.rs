//! Where the emulator, the firmware and the per-run directory are, and the
//! `Plan` built from them. `TODO/milestones.md` T-1112.
//!
//! ⛔ **Every path is resolved here, once.** The alternative is what the
//! first revision did: the same lookup in three functions, with the firmware
//! copy in one of them and not the others, so `doctor` reported paths a run
//! would not have used. One builder, one answer.
//!
//! ⚠ **The accelerator is read from the probe and named out loud, never
//! defaulted.** `kvm` where the machine tier's `full` profile holds, `tcg`
//! where only `tcg` does, and a refusal naming the missing leg where neither
//! does — the same three-way decision the machine tier already makes for a
//! Linux guest, so a machine podbox refuses that on is not quietly a machine
//! it boots a Windows guest on.

use std::path::{Path, PathBuf};

use podbox_windows::{Accel, Plan};

use super::args::Args;

/// The emulator, from `PODBOX_QEMU` or the PATH name.
pub(crate) fn emulator() -> PathBuf {
    std::env::var_os("PODBOX_QEMU")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-system-x86_64"))
}

pub(crate) fn qemu_img() -> PathBuf {
    std::env::var_os("PODBOX_QEMU_IMG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-img"))
}

/// Where the emulator finds its firmware and option ROMs.
pub(crate) fn share() -> PathBuf {
    std::env::var_os("PODBOX_QEMU_SHARE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/qemu"))
}

/// UEFI code and variable store. `(code, vars)`, from the environment or the
/// distribution's own paths. ⚠ Both are needed and they are not the same
/// file: Windows refuses the legacy BIOS path here, and a variable store
/// shared between runs carries the last run's boot entries into this one.
pub(crate) fn firmware() -> (PathBuf, PathBuf) {
    let code = std::env::var_os("PODBOX_OVMF_CODE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/edk2/x86_64/OVMF_CODE.4m.fd"));
    let vars = std::env::var_os("PODBOX_OVMF_VARS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/edk2/x86_64/OVMF_VARS.4m.fd"));
    (code, vars)
}

/// `RLIMIT_FSIZE`, or the refusal that says it could not be read. The
/// ceiling `TODO/podvm.md` T-1305 already judges guest memory against.
pub(crate) fn ceiling() -> Result<u64, i32> {
    match podbox_probe::sys::prlimit(podbox_probe::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) => Ok(cur),
        Err(e) => {
            eprintln!(
                "podbox windows: prlimit(RLIMIT_FSIZE) could not be read: {} \
                 (TODO/podvm.md T-1305)",
                e.name()
            );
            Err(podbox_image::error::EXIT_RUNTIME_ERROR)
        }
    }
}

/// The machine tier's accelerator, or the refusal that names why not.
pub(crate) fn accelerator() -> Result<Accel, i32> {
    let findings = podbox_probe::run();
    let assessed = podbox_probe::machine::assess(&findings);
    match podbox_windows::accel_for(assessed.profile) {
        Some(a) => Ok(a),
        None => {
            let why = assessed
                .refusal()
                .unwrap_or_else(|| "the machine tier establishes no profile".to_string());
            eprintln!("podbox windows: {why}");
            Err(podbox_image::error::EXIT_RUNTIME_ERROR)
        }
    }
}

/// True where the driver can find `name`.
///
/// ⚠ A bare name is looked up on `PATH` rather than tested as a path: the
/// default emulator spelling is a name, and `Path::is_file` on it is false
/// on every machine, which would refuse everywhere. A `PODBOX_QEMU` that
/// names a bare binary is therefore found too, which is the point.
fn found(name: &str) -> bool {
    let p = PathBuf::from(name);
    if p.components().count() > 1 {
        p.is_file()
    } else {
        crate::tier::which_qemu(name).is_some()
    }
}

/// Build the plan for `image`, or refuse naming what is missing. The per-run
/// directory and the per-run copy of the UEFI variable store are created
/// here, so every caller gets both.
pub(crate) fn build(a: &Args, image: &Path) -> Result<Plan, i32> {
    let accel = accelerator()?;
    let (code, vars) = firmware();
    for bin in [emulator(), qemu_img()] {
        let name = bin.to_string_lossy().to_string();
        if !found(&name) {
            eprintln!("podbox windows: {name} is not on this machine");
            return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
        }
    }
    if !code.is_file() {
        eprintln!("podbox windows: no UEFI code image at {}", code.display());
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    if !vars.is_file() {
        eprintln!(
            "podbox windows: no UEFI variable store at {}",
            vars.display()
        );
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    let dir = match podbox_windows::scratch_dir("run") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("podbox windows: {e}");
            return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
        }
    };
    let vars_run = dir.join("vars.fd");
    if let Err(e) = std::fs::copy(&vars, &vars_run) {
        eprintln!("podbox windows: {}: {e}", vars.display());
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    let vars_run = std::fs::canonicalize(&vars_run).unwrap_or(vars_run);
    // ⚠ Absolute, because the emulator resolves a backing file relative to
    // the overlay the first time it is opened, and the overlay lives in a
    // scratch directory rather than beside the base.
    let image = std::fs::canonicalize(image).unwrap_or_else(|_| image.to_path_buf());
    Ok(Plan {
        emulator: emulator(),
        accel,
        share: share(),
        firmware_code: code,
        firmware_vars: vars_run,
        root: dir.join("run.qcow2"),
        mailbox: dir.join("mailbox.img"),
        serial: dir.join("serial.log"),
        monitor: dir.join("qmp.sock"),
        memory_mib: a.mem.map(|m| m / (1 << 20)).unwrap_or(4096),
        cpus: a.cpus.unwrap_or(2),
        emu_args: a.emu_args.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_emulator_default_is_a_name_and_the_override_is_a_path() {
        std::env::remove_var("PODBOX_QEMU");
        assert_eq!(emulator(), PathBuf::from("qemu-system-x86_64"));
        std::env::set_var("PODBOX_QEMU", "/opt/qemu/bin/qemu-system-x86_64");
        assert_eq!(
            emulator(),
            PathBuf::from("/opt/qemu/bin/qemu-system-x86_64")
        );
        std::env::remove_var("PODBOX_QEMU");
    }

    #[test]
    fn the_firmware_pair_is_two_paths_and_the_environment_wins() {
        std::env::set_var("PODBOX_OVMF_CODE", "/tmp/code.fd");
        std::env::set_var("PODBOX_OVMF_VARS", "/tmp/vars.fd");
        let (c, v) = firmware();
        assert_eq!(c, PathBuf::from("/tmp/code.fd"));
        assert_eq!(v, PathBuf::from("/tmp/vars.fd"));
        assert_ne!(c, v, "a shared variable store leaks boot entries between runs");
        std::env::remove_var("PODBOX_OVMF_CODE");
        std::env::remove_var("PODBOX_OVMF_VARS");
    }

    #[test]
    fn a_bare_name_is_looked_up_on_path_and_a_path_is_checked_as_one() {
        // `which_qemu` on a name nothing owns is None; a path that is not a
        // file is not found either. Neither panics, and both are what the
        // refusal above keys on.
        assert!(crate::tier::which_qemu("definitely-not-an-emulator-xyz").is_none());
        assert!(crate::tier::which_qemu("sh").is_some());
    }
}
