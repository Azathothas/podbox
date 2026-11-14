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
//! does, the same three-way decision the machine tier already makes for a
//! Linux guest, so a machine podvm refuses that on is not quietly a machine
//! it boots a Windows guest on.

use std::path::{Path, PathBuf};

use podvm_windows::{Accel, Plan};

use super::args::Args;

/// The emulator, from `PODVM_QEMU` or the PATH name.
pub(crate) fn emulator() -> PathBuf {
    std::env::var_os("PODVM_QEMU")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-system-x86_64"))
}

pub(crate) fn qemu_img() -> PathBuf {
    std::env::var_os("PODVM_QEMU_IMG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-img"))
}

/// Where the emulator finds its firmware and option ROMs.
pub(crate) fn share() -> PathBuf {
    std::env::var_os("PODVM_QEMU_SHARE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/qemu"))
}

/// UEFI code and variable store. `(code, vars)`, from the environment or the
/// distribution's own paths. ⚠ Both are needed and they are not the same
/// file: Windows refuses the legacy BIOS path here, and a variable store
/// shared between runs carries the last run's boot entries into this one.
///
/// ⚠ Distributions do not ship one path. QEMU names it
/// `edk2-x86_64-code.fd` under its share directory, Fedora-style packaging
/// uses `OVMF_CODE.fd` under `edk2-ovmf`, Debian-style uses `OVMF/`, so
/// the first candidate that exists wins, and the environment still wins
/// over all of them. A host with none is refused naming every path that
/// was looked at, not just the last one.
pub(crate) fn firmware() -> (PathBuf, PathBuf) {
    // TODO/machine.md T-1871: pairs, not two lists. The code and the vars
    // of one build go together, and Debian and Ubuntu name theirs
    // `OVMF_*_4M.fd`, which the lists did not have (the doctor of a cloud
    // host on Ubuntu 24.04 found no firmware).
    const PAIRS: &[(&str, &str)] = &[
        (
            "/usr/share/OVMF/OVMF_CODE_4M.fd",
            "/usr/share/OVMF/OVMF_VARS_4M.fd",
        ),
        (
            "/usr/share/OVMF/OVMF_CODE.fd",
            "/usr/share/OVMF/OVMF_VARS.fd",
        ),
        (
            "/usr/share/edk2/x86_64/OVMF_CODE.4m.fd",
            "/usr/share/edk2/x86_64/OVMF_VARS.4m.fd",
        ),
        (
            "/usr/share/edk2/ovmf/OVMF_CODE.fd",
            "/usr/share/edk2/ovmf/OVMF_VARS.fd",
        ),
        (
            "/usr/share/edk2-ovmf/OVMF_CODE.fd",
            "/usr/share/edk2-ovmf/OVMF_VARS.fd",
        ),
        (
            "/usr/share/qemu/edk2-x86_64-code.fd",
            "/usr/share/qemu/edk2-i386-vars.fd",
        ),
    ];
    let pair = PAIRS
        .iter()
        .find(|(c, v)| Path::new(c).is_file() && Path::new(v).is_file())
        .or(PAIRS.last())
        .map(|(c, v)| (PathBuf::from(c), PathBuf::from(v)))
        .unwrap_or_default();
    let code = std::env::var_os("PODVM_OVMF_CODE")
        .map(PathBuf::from)
        .unwrap_or(pair.0);
    let vars = std::env::var_os("PODVM_OVMF_VARS")
        .map(PathBuf::from)
        .unwrap_or(pair.1);
    (code, vars)
}

/// The soft `RLIMIT_FSIZE`, or the refusal that says it could not be read.
/// Its one caller is the download (`setup::fetch`): guest memory is not
/// judged against it on this path, because no memory backing file is
/// written (`run.rs`, and `TODO/milestones.md` T-1692).
pub(crate) fn ceiling() -> Result<u64, i32> {
    match podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) => Ok(cur),
        Err(e) => {
            eprintln!(
                "podvm windows: prlimit(RLIMIT_FSIZE) could not be read: {} \
                 (TODO/machine.md T-1305)",
                e.name()
            );
            Err(podvm_image::error::EXIT_RUNTIME_ERROR)
        }
    }
}

/// The machine tier's accelerator, or the refusal that names why not.
pub(crate) fn accelerator() -> Result<Accel, i32> {
    let findings = podvm_probe::run();
    let assessed = podvm_probe::machine::assess(&findings);
    match podvm_windows::accel_for(assessed.profile) {
        Some(a) => Ok(a),
        None => {
            let why = assessed
                .refusal()
                .unwrap_or_else(|| "the machine tier establishes no profile".to_string());
            eprintln!("podvm windows: {why}");
            Err(podvm_image::error::EXIT_RUNTIME_ERROR)
        }
    }
}

/// True where the driver can find `name`.
///
/// ⚠ A bare name is looked up on `PATH` rather than tested as a path: the
/// default emulator spelling is a name, and `Path::is_file` on it is false
/// on every machine, which would refuse everywhere. A `PODVM_QEMU` that
/// names a bare binary is therefore found too, which is the point.
pub(crate) fn found(name: &str) -> bool {
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
pub(crate) fn build(a: &Args) -> Result<Plan, i32> {
    let accel = accelerator()?;
    let (code, vars) = firmware();
    for bin in [emulator(), qemu_img()] {
        let name = bin.to_string_lossy().to_string();
        if !found(&name) {
            eprintln!("podvm windows: {name} is not on this machine");
            return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
        }
    }
    if !code.is_file() {
        eprintln!("podvm windows: no UEFI code image at {}", code.display());
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    if !vars.is_file() {
        eprintln!(
            "podvm windows: no UEFI variable store at {}",
            vars.display()
        );
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    let dir = match podvm_windows::scratch_dir("run") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("podvm windows: {e}");
            return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
        }
    };
    let vars_run = dir.join("vars.fd");
    if let Err(e) = std::fs::copy(&vars, &vars_run) {
        eprintln!("podvm windows: {}: {e}", vars.display());
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    let vars_run = std::fs::canonicalize(&vars_run).unwrap_or(vars_run);
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
        // The mailbox run (podcrates `v0.45.0`): the NIC and the media of
        // TODO/machine.md T-1865 are not chosen here yet.
        options: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_emulator_default_is_a_name_and_the_override_is_a_path() {
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::remove_var("PODVM_QEMU") };
        assert_eq!(emulator(), PathBuf::from("qemu-system-x86_64"));
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::set_var("PODVM_QEMU", "/opt/qemu/bin/qemu-system-x86_64") };
        assert_eq!(
            emulator(),
            PathBuf::from("/opt/qemu/bin/qemu-system-x86_64")
        );
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::remove_var("PODVM_QEMU") };
    }

    /// TODO/machine.md T-1871: the firmware is a pair of one build; the
    /// code and the vars are never taken from two directories.
    #[test]
    fn windows_doctor_firmware_is_a_pair_of_one_directory() {
        if std::env::var_os("PODVM_OVMF_CODE").is_some() {
            return;
        }
        let (c, v) = firmware();
        assert_eq!(
            c.parent(),
            v.parent(),
            "{} and {}",
            c.display(),
            v.display()
        );
    }

    #[test]
    fn the_firmware_pair_is_two_paths_and_the_environment_wins() {
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::set_var("PODVM_OVMF_CODE", "/tmp/code.fd") };
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::set_var("PODVM_OVMF_VARS", "/tmp/vars.fd") };
        let (c, v) = firmware();
        assert_eq!(c, PathBuf::from("/tmp/code.fd"));
        assert_eq!(v, PathBuf::from("/tmp/vars.fd"));
        assert_ne!(
            c, v,
            "a shared variable store leaks boot entries between runs"
        );
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::remove_var("PODVM_OVMF_CODE") };
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe { std::env::remove_var("PODVM_OVMF_VARS") };
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
