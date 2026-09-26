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
mod dos;
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
            // ⚠ Absolute, because the overlay `stage` writes lives in a
            // scratch directory while the emulator resolves the backing
            // file at open time: a relative `-b` is resolved against
            // whatever the working directory is then, not now.
            return Ok(std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()));
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
    // ⭐ The positional is an OCI-shaped token on this door, not a path:
    // where it names a file (or looks like a disk path) the Windows flavor
    // owns it and a missing one is refused, never replaced; otherwise the
    // DOS flavor boots its base cache and the token is only bannered, never
    // fetched. The dispatch happens before `resolve_image` because that
    // refusal belongs to the disk door alone.
    if image.is_empty() || !is_disk_request(image) {
        return dos::dos_run(verb, mem, None, qemu_extra, None, &argv.join(" "));
    }
    let named = Some(std::path::PathBuf::from(image));
    let image = match resolve_image(named.as_deref()) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let a = args::Args {
        sub: "run".to_string(),
        guest: None,
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

/// True where the run door's positional names a disk image rather than an
/// OCI-shaped token: it is an existing file, it is anchored like a path
/// (a leading `/`, `\`, `.`, `~` or a drive letter), or it carries a disk
/// suffix. A path-looking token that is missing is refused by name; only a
/// registry-looking token reaches the DOS flavor.
///
/// ⛔ The shape is the whole decision, and it is pinned below: a disk path
/// that fell through to DOS would boot the wrong guest and report its
/// output as the named image's, which is the defect `resolve_image` pins
/// for the `windows` verb. An interior `/` alone is not a path: every OCI
/// reference has one (`registry/org/image:tag`) and none starts with one.
pub(crate) fn is_disk_request(image: &str) -> bool {
    if std::path::Path::new(image).is_file() {
        return true;
    }
    let mut chars = image.chars();
    match chars.next() {
        Some('/') | Some('\\') | Some('.') | Some('~') => return true,
        Some(d) if d.is_ascii_alphabetic() => {
            if matches!(chars.next(), Some(':')) {
                return true;
            }
        }
        _ => {}
    }
    let lower = image.to_ascii_lowercase();
    [
        ".vhdx", ".qcow2", ".qcow", ".vhd", ".vmdk", ".vdi", ".img", ".raw", ".iso",
    ]
    .iter()
    .any(|s| lower.ends_with(s))
}

/// A flag `doctor` never reads, if one was given. `doctor` reports the
/// machine and the paths; tuning flags and inputs change nothing it
/// prints, so each is named rather than silently dropped.
fn doctor_unused(a: &args::Args) -> Option<&'static str> {
    if a.image.is_some() {
        return Some("--image");
    }
    if a.url.is_some() {
        return Some("--url");
    }
    if a.sha256.is_some() {
        return Some("--sha256");
    }
    if a.max_bytes.is_some() {
        return Some("--max-bytes");
    }
    if a.mem.is_some() {
        return Some("--podbox-mem");
    }
    if a.cpus.is_some() {
        return Some("--podbox-cpus");
    }
    if a.timeout.is_some() {
        return Some("--podbox-timeout");
    }
    if !a.emu_args.is_empty() {
        return Some("--podbox-qemu-arg");
    }
    if !a.command.is_empty() {
        return Some("a command");
    }
    if a.guest.is_some() {
        return Some("--guest");
    }
    None
}

/// `podbox windows`.
pub fn windows(args: &[String]) -> i32 {
    let a = match args::parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return 0,
        Err(code) => return code,
    };
    // ⛔ A flag the subcommand never reads is refused rather than
    // silently dropped: an accepted-and-ignored limit is a setting the
    // caller believes is honored. Each arm below names what it takes, and
    // anything else on that arm is a flag error naming the flag.
    match a.sub.as_str() {
        "doctor" => {
            if let Some(f) = doctor_unused(&a) {
                eprintln!("podbox windows: {f} does nothing on doctor\n{USAGE}");
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            doctor::doctor(&a)
        }
        "setup" => {
            if a.url.is_some() || a.sha256.is_some() || a.max_bytes.is_some() {
                eprintln!(
                    "podbox windows: setup provisions a disk image, it does not fetch one; --url/--sha256/--max-bytes belong to fetch\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            if a.guest.is_some() {
                eprintln!(
                    "podbox windows: setup provisions the Windows disk image; the DOS flavor needs no provisioning\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            if !a.command.is_empty() {
                eprintln!(
                    "podbox windows: setup provisions an image, it runs no command; a command belongs to run\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            setup::setup(&a)
        }
        "fetch" => {
            if a.guest.is_some() {
                eprintln!(
                    "podbox windows: fetch downloads a Windows disk image; --guest belongs to run\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            if a.mem.is_some()
                || a.cpus.is_some()
                || a.timeout.is_some()
                || !a.emu_args.is_empty()
                || !a.command.is_empty()
            {
                eprintln!(
                    "podbox windows: fetch downloads a file; guest tuning and commands belong to run and setup\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            setup::fetch(&a)
        }
        "run" => {
            if a.url.is_some() || a.sha256.is_some() || a.max_bytes.is_some() {
                eprintln!(
                    "podbox windows: run boots an image, it does not fetch one; --url/--sha256/--max-bytes belong to fetch\n{USAGE}"
                );
                return podbox_image::error::EXIT_FLAG_ERROR;
            }
            if a.guest == Some(args::Guest::Dos) {
                if a.image.is_some() {
                    eprintln!(
                        "podbox windows: the DOS flavor boots its base cache, not a disk                          you hand it; --image belongs to the windows flavor\n{USAGE}"
                    );
                    return podbox_image::error::EXIT_FLAG_ERROR;
                }
                dos::dos_run("windows", a.mem, a.cpus, &a.emu_args, a.timeout, &a.command)
            } else {
                run::run("windows", &a)
            }
        }
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
        assert!(
            !should_drive("run", true, true, "windows"),
            "detached run never reaches a driver"
        );
        assert!(
            !should_drive("run", false, false, "windows"),
            "the chroot tier shares the host kernel"
        );
        assert!(
            !should_drive("create", false, true, "windows"),
            "create writes a record"
        );
        assert!(
            !should_drive("run", false, true, "linux"),
            "a Linux request is the Linux path"
        );
        assert!(!should_drive("exec", false, true, "windows"));
        assert!(
            !should_drive("pull", false, true, "windows"),
            "there is no registry in this path"
        );
    }

    #[test]
    fn a_registry_token_is_not_a_disk_and_a_disk_shape_is() {
        assert!(!is_disk_request("freedos:1.4"));
        assert!(!is_disk_request("alpine:3.20"));
        assert!(!is_disk_request(
            "public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc"
        ));
        assert!(!is_disk_request("ver"));
        assert!(!is_disk_request(""));
        // an existing file is always a disk, whatever its name
        let here = std::env::current_exe().expect("the test binary is a file");
        assert!(is_disk_request(here.to_str().unwrap()));
        // and a path-looking token is a disk even where it is missing,
        // so a typo is refused rather than booted as DOS
        assert!(is_disk_request("/definitely/not/here.vhdx"));
        assert!(is_disk_request("./win.qcow2"));
        assert!(is_disk_request("C:\\images\\w.vhdx"));
        assert!(is_disk_request("disk.VHDX"));
        assert!(is_disk_request("disk.img"));
        assert!(!is_disk_request("not-a-path"));
    }

    #[test]
    fn a_flag_no_subcommand_reads_is_refused_naming_the_flag() {
        let w = |xs: &[&str]| windows(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let err = podbox_image::error::EXIT_FLAG_ERROR;
        // ⛔ `doctor` answers through the probe, which does not hold in a
        // test harness, so a through-`windows()` assertion would pass for
        // the wrong reason (the probe refusal, not the flag refusal): the
        // gate function is pinned directly instead.
        let doc = |xs: &[&str]| {
            let a = args::parse(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .unwrap()
                .unwrap();
            doctor_unused(&a)
        };
        assert_eq!(doc(&["doctor", "--image", "x"]), Some("--image"));
        assert_eq!(doc(&["doctor", "--podbox-mem", "1G"]), Some("--podbox-mem"));
        assert_eq!(doc(&["doctor"]), None);
        assert_eq!(w(&["fetch", "--url", "u", "--podbox-mem", "1G"]), err);
        assert_eq!(w(&["fetch", "--url", "u", "--guest", "dos"]), err);
        assert_eq!(w(&["setup", "--image", "x", "--url", "u"]), err);
        assert_eq!(w(&["setup", "--image", "x", "--", "ver"]), err);
        assert_eq!(w(&["run", "--guest", "dos", "--image", "x"]), err);
        assert_eq!(w(&["run", "--image", "x", "--url", "u", "--", "ver"]), err);
        assert_eq!(w(&["fetch", "--url", "u", "--podbox-mem", "1G"]), err);
        assert_eq!(w(&["fetch", "--url", "u", "--", "ver"]), err);
        assert_eq!(w(&["setup", "--image", "x", "--", "ver"]), err);
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
