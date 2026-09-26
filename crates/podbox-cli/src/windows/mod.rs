//! `podbox windows`: a disposable Windows guest driven through a mailbox.
//! `TODO/milestones.md` T-1112.
//!
//! ⛔ **Why a guest that is not Linux is a driver and not a platform on
//! `run`.** Every other platform podbox accepts is an OCI image, and the
//! whole `run` path — pull, extract, fixups, chroot — exists to turn one
//! into a rootfs. A Windows guest is not that: it is a disk image an
//! emulator boots, and none of the OCI path applies to it. So the driver is
//! reached two ways, and both end in the same code: the `podbox windows`
//! verb for a caller who wants `setup` and `doctor` as well, and
//! `podbox run --podbox-tier=machine --platform windows/amd64`, which is the
//! acceptance command `TODO/milestones.md` T-1112 already writes down.
//!
//! ⭐ **The shape is the reference implementation's and the refusal is
//! not.** One emulator child process, UEFI firmware, a per-run overlay over
//! a read-only base so the guest's writes die with the run, a FAT mailbox
//! the host and guest both see, `-nic none`, no daemon and no libvirt. What
//! differs is that the accelerator is the machine tier's own profile, so
//! `tcg` runs where `kvm` is missing, which is the only reason the feature
//! can exist on the machines podbox targets.
//!
//! ## The modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | [`args`] | the flag surface, and nothing else |
//! | [`plan`] | where the emulator, the firmware and the per-run directory are, and the `Plan` built from them |
//! | [`doctor`] | what this machine would do, and the refusal where it would do nothing |
//! | [`setup`] | acquisition and the one provisioning boot |
//! | [`run`] | one boot, one command, and the guest's own exit code |
//!
//! ⚠ This was one file. It was split when it passed 400 lines and the four
//! questions it answered started sharing locals: the paths, the flags, the
//! acquisition and the run had grown into each other, and a reader could no
//! longer tell which of them created the per-run directory.

mod args;
mod doctor;
mod plan;
mod run;
mod setup;

use std::path::PathBuf;

pub(crate) use args::USAGE;

/// True where this invocation should reach the guest driver.
///
/// ⛔ **Only foreground `run` on the machine tier naming Windows.** Every
/// other door keeps its own refusal by name, and the reasons differ:
/// `create` writes a record for a fleet that does not exist, detached `run`
/// never reaches a guest driver at all, the chroot tier shares the host
/// kernel and cannot boot anything, and a Linux request belongs to the
/// Linux path. Widening this is how a limit becomes a silent degradation.
pub(crate) fn should_drive(verb: &str, detach: bool, machine: bool, os: &str) -> bool {
    verb == "run" && !detach && machine && os == "windows"
}

/// The disk image a run boots: the one named on the command line, or the
/// cached base image where none was named. A named path that does not exist
/// is a refusal, never a substitution.
///
/// ⛔ **A named image is the caller's decision and is never replaced.** An
/// earlier revision fell back to the cache whenever the named path was
/// missing, so `--image /mnt/mine.vhdx` with a typo booted a *different*
/// guest and reported *its* output: the caller reads a result they believe
/// came from the image they chose. A missing file is named as missing.
///
/// ⚠ Falling back to the cache where **nothing** was named is a different
/// thing and is kept: on every other platform the positional is an image
/// reference nobody has resolved yet, and here it is a path, so a caller who
/// names none and has a cache should get the cache rather than a refusal
/// about a reference this path never had.
fn resolve_image(image: Option<&std::path::Path>) -> Result<PathBuf, i32> {
    if let Some(p) = image {
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
        eprintln!(
            "podbox windows: {} is not a file. A named image is never replaced \
             by the cache; pass the provisioned image, or name none to use {} \
             (TODO/milestones.md T-1112)",
            p.display(),
            podbox_windows::base_cache().display()
        );
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    let base = podbox_windows::base_cache();
    if base.is_file() {
        return Ok(base);
    }
    eprintln!(
        "podbox windows: no Windows disk image to boot and no base image is \
         cached at {}. `podbox windows fetch` downloads one, `podbox windows \
         setup` installs the agent into it (TODO/milestones.md T-1112)",
        base.display()
    );
    Err(podbox_image::error::EXIT_RUNTIME_ERROR)
}

/// The `run` verb's seam into this driver.
///
/// ⭐ One entry point for both doors. A second, `run`-shaped implementation
/// of the boot is exactly the kind of copy that diverges, and the one
/// nobody exercises is the one that rots.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_windows(
    verb: &str,
    mem: Option<u64>,
    arch: &str,
    image: &str,
    argv: &[String],
    qemu_extra: &[String],
) -> i32 {
    if arch != "amd64" {
        eprintln!(
            "podbox {verb}: windows/{arch} is refused: this guest follows the \
             host, so an x86_64 host runs windows/amd64 and nothing else, and \
             an emulated Windows guest would be a different product at a \
             different speed (TODO/milestones.md T-1112)"
        );
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    let named = if image.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(image))
    };
    let image = match resolve_image(named.as_deref()) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let a = args::Args {
        sub: "run".to_string(),
        image: Some(image),
        mem,
        cpus: None,
        timeout: None,
        emu_args: qemu_extra.to_vec(),
        command: podbox_windows::agent::command_from_argv(argv),
        url: None,
        sha256: None,
        max_bytes: None,
    };
    run::run(verb, &a)
}

/// `podbox windows`.
pub fn windows(args: &[String]) -> i32 {
    let a = match args::parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return 0,
        Err(code) => return code,
    };
    match a.sub.as_str() {
        "doctor" => doctor::doctor(&a),
        "setup" => setup::setup(&a),
        "fetch" => setup::fetch(&a),
        "run" => run::run("windows", &a),
        other => {
            eprintln!("podbox windows: {other} is not a subcommand\n{USAGE}");
            podbox_image::error::EXIT_FLAG_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_foreground_machine_tier_windows_run_reaches_the_driver() {
        assert!(should_drive("run", false, true, "windows"));
        // every other door keeps its own refusal, and each for its own reason
        assert!(!should_drive("run", true, true, "windows"), "detached run never reaches a driver");
        assert!(!should_drive("run", false, false, "windows"), "the chroot tier shares the host kernel");
        assert!(!should_drive("create", false, true, "windows"), "create writes a record");
        assert!(!should_drive("run", false, true, "linux"), "a Linux request is the Linux path");
        assert!(!should_drive("exec", false, true, "windows"));
        assert!(!should_drive("pull", false, true, "windows"), "there is no registry in this path");
    }

    #[test]
    fn a_non_amd64_arch_is_refused_naming_the_arch() {
        let c = run_windows("run", None, "arm64", "", &[], &[]);
        assert_eq!(c, podbox_image::error::EXIT_RUNTIME_ERROR);
    }

    #[test]
    fn a_named_image_that_is_missing_is_refused_and_never_replaced_by_the_cache() {
        // ⛔ The defect this pins: a typo used to boot the cached guest and
        // report its output as the named image's.
        let missing = std::path::PathBuf::from("/definitely/not/here.img");
        assert_eq!(
            resolve_image(Some(&missing)),
            Err(podbox_image::error::EXIT_RUNTIME_ERROR)
        );
        // a named file that exists is taken as named, cache or no cache
        let here = std::env::current_exe().expect("the test binary is a file");
        assert_eq!(resolve_image(Some(&here)), Ok(here));
        // and naming none still falls back to the cache, or refuses without one
        match resolve_image(None) {
            Ok(p) => assert_eq!(p, podbox_windows::base_cache()),
            Err(c) => assert_eq!(c, podbox_image::error::EXIT_RUNTIME_ERROR),
        }
    }
}
