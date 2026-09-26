//! `podbox windows`: a disposable Windows guest driven through a mailbox.
//! `TODO/milestones.md` T-1112.
//!
//! ⛔ **Why this is a verb of its own rather than `--platform windows/amd64`
//! on `run`.** Every other platform podbox accepts is an OCI image, and the
//! whole `run` path — pull, extract, fixups, chroot — exists to turn one into
//! a rootfs. A Windows guest is not that: it is a disk image the emulator
//! boots, and nothing about the OCI path applies to it. Folding it into
//! `run` would mean a verb whose first half is dead for half its inputs,
//! which is the kind of branch that rots. The image reference is a path and
//! the "command" is a `cmd.exe` line, so the surface is stated separately and
//! the refusal in [`crate::lifecycle::ensure_linux_guest`] points here.
//!
//! ⭐ **Three subcommands, and each one is a question somebody actually
//! asks.** `doctor` answers "can this machine run a guest at all", naming the
//! accelerator; `setup` answers "install the agent into this image" and is
//! the only step that needs the console; `run` is the feature.

use std::path::{Path, PathBuf};
use std::time::Duration;

use podbox_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podbox_windows as w;

const USAGE: &str = "\
usage: podbox windows <doctor|setup|run> [options] [--] [command]
  doctor                       report the machine profile and the accelerator
                               a guest run would use, and refuse where none
  setup    --image <disk>      install the guest agent into a Windows disk
                               image, once, writing <disk>.podbox.qcow2 beside
                               it. Pass THAT to `run`. This is the only step
                               that uses the console.
  run      --image <disk>      boot a disposable guest, run one cmd.exe line,
                               and print its stdout, stderr and exit code.
options:
  --image <path>               the Windows disk image (vhdx, qcow2 or raw)
  --podbox-mem <size>          guest memory, for example 4G
  --podbox-cpus <n>            guest cpus
  --podbox-timeout <seconds>   how long a run may take before it is stopped
  --podbox-qemu-arg <arg>      one extra emulator argument, repeatable
";

/// One parsed `podbox windows` invocation.
struct Args {
    sub: String,
    image: Option<PathBuf>,
    mem: Option<u64>,
    cpus: Option<u32>,
    timeout: Option<u64>,
    emu_args: Vec<String>,
    command: String,
}

/// The largest guest memory the file-size ceiling allows, which is also the
/// bound on the base image itself. `TODO/podvm.md` T-1305.
fn ceiling() -> Result<u64, i32> {
    match podbox_probe::sys::prlimit(podbox_probe::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) => Ok(cur),
        Err(e) => {
            eprintln!(
                "podbox windows: prlimit(RLIMIT_FSIZE) could not be read: {} \
                 (TODO/podvm.md T-1305)",
                e.name()
            );
            Err(EXIT_RUNTIME_ERROR)
        }
    }
}

/// The emulator, from `$PODBOX_QEMU` or the PATH name.
fn emulator() -> PathBuf {
    std::env::var_os("PODBOX_QEMU")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-system-x86_64"))
}

fn qemu_img() -> PathBuf {
    std::env::var_os("PODBOX_QEMU_IMG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("qemu-img"))
}

/// Where the emulator finds its firmware and option ROMs.
fn share() -> PathBuf {
    std::env::var_os("PODBOX_QEMU_SHARE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/qemu"))
}

/// UEFI code and variable store. `(code, vars)`, from the environment or the
/// distribution's own paths.
fn firmware() -> (PathBuf, PathBuf) {
    let code = std::env::var_os("PODBOX_OVMF_CODE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/edk2/x86_64/OVMF_CODE.4m.fd"));
    let vars = std::env::var_os("PODBOX_OVMF_VARS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/edk2/x86_64/OVMF_VARS.4m.fd"));
    (code, vars)
}

/// A scratch directory for this run: the mailbox, the overlay, the per-run
/// variable store and the monitor socket all live here and none of them
/// outlives the run.
fn scratch() -> Result<PathBuf, i32> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(format!("podbox-windows-{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("podbox windows: {}: {e}", dir.display());
        return Err(EXIT_RUNTIME_ERROR);
    }
    Ok(dir)
}

/// Parse the verb. `Ok(None)` is `-h/--help`: printed, and the exit is 0.
fn parse(args: &[String]) -> Result<Option<Args>, i32> {
    let mut it = args.iter().peekable();
    let sub = match it.next() {
        Some(s) => s.clone(),
        None => {
            eprintln!("podbox windows: no subcommand\n{USAGE}");
            return Err(EXIT_FLAG_ERROR);
        }
    };
    if sub == "-h" || sub == "--help" {
        print!("{USAGE}");
        return Ok(None);
    }
    let mut a = Args {
        sub,
        image: None,
        mem: None,
        cpus: None,
        timeout: None,
        emu_args: Vec::new(),
        command: String::new(),
    };
    let mut words: Vec<String> = Vec::new();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--" => {
                words.extend(it.by_ref().cloned());
                break;
            }
            "--image" => {
                let v = it.next().ok_or(EXIT_FLAG_ERROR)?;
                a.image = Some(PathBuf::from(v));
            }
            "--podbox-mem" => {
                let v = it.next().ok_or(EXIT_FLAG_ERROR)?;
                a.mem = Some(crate::tier::parse_mem(v).ok_or_else(|| {
                    eprintln!("podbox windows: --podbox-mem {v} is not a size");
                    EXIT_FLAG_ERROR
                })?);
            }
            "--podbox-cpus" => {
                let v = it.next().ok_or(EXIT_FLAG_ERROR)?;
                a.cpus = Some(v.parse().map_err(|_| {
                    eprintln!("podbox windows: --podbox-cpus {v} is not a count");
                    EXIT_FLAG_ERROR
                })?);
                if a.cpus == Some(0) {
                    eprintln!("podbox windows: --podbox-cpus 0 is not a machine");
                    return Err(EXIT_FLAG_ERROR);
                }
            }
            "--podbox-timeout" => {
                let v = it.next().ok_or(EXIT_FLAG_ERROR)?;
                a.timeout = Some(v.parse().map_err(|_| {
                    eprintln!("podbox windows: --podbox-timeout {v} is not a count of seconds");
                    EXIT_FLAG_ERROR
                })?);
            }
            "--podbox-qemu-arg" => {
                let v = it.next().ok_or(EXIT_FLAG_ERROR)?;
                a.emu_args.push(v.clone());
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            other if other.starts_with('-') => {
                eprintln!("podbox windows: {other} is not a flag this verb has\n{USAGE}");
                return Err(EXIT_FLAG_ERROR);
            }
            other => words.push(other.to_string()),
        }
    }
    a.command = words.join(" ");
    Ok(Some(a))
}

/// Build the emulator plan for `image`, or refuse naming what is missing.
fn plan_for(a: &Args, image: &Path) -> Result<(w::Plan, w::Accel), i32> {
    let findings = podbox_probe::run();
    let assessed = podbox_probe::machine::assess(&findings);
    let accel = match w::accel_for(assessed.profile) {
        Some(x) => x,
        None => {
            let why = assessed
                .refusal()
                .unwrap_or_else(|| "the machine tier establishes no profile".to_string());
            eprintln!("podbox windows: {why}");
            return Err(EXIT_RUNTIME_ERROR);
        }
    };
    let (code, vars) = firmware();
    for (what, p) in [
        ("the emulator", emulator()),
        ("OVMF code", code.clone()),
        ("OVMF variables", vars.clone()),
    ] {
        if !p.is_file() && p.components().count() > 1 {
            eprintln!("podbox windows: {what} is not at {}", p.display());
            return Err(EXIT_RUNTIME_ERROR);
        }
    }
    let dir = scratch()?;
    let vars_run = dir.join("vars.fd");
    if let Err(e) = std::fs::copy(&vars, &vars_run) {
        eprintln!("podbox windows: {}: {e}", vars.display());
        return Err(EXIT_RUNTIME_ERROR);
    }
    // ⚠ Per run, because a shared variable store carries the last run's boot
    // entries into this one.
    let vars_run = std::fs::canonicalize(&vars_run).unwrap_or(vars_run);
    // The emulator resolves the backing file relative to the overlay when it
    // is first opened, so an overlay in another directory needs the base
    // path to be absolute.
    let image = std::fs::canonicalize(image).unwrap_or_else(|_| image.to_path_buf());
    Ok((
        w::Plan {
            emulator: emulator(),
            accel,
            share: share(),
            firmware_code: code,
            firmware_vars: vars_run,
            root: dir.join("run.qcow2"),
            mailbox: dir.join("mailbox.img"),
            serial: dir.join("serial.log"),
            monitor: dir.join("mon.sock"),
            memory_mib: a.mem.map(|m| m / (1 << 20)).unwrap_or(4096),
            cpus: a.cpus.unwrap_or(2),
            emu_args: a.emu_args.clone(),
        },
        accel,
    ))
}

/// `doctor`: the machine profile, the accelerator, and nothing else.
fn doctor(_a: &Args) -> i32 {
    let findings = podbox_probe::run();
    let assessed = podbox_probe::machine::assess(&findings);
    let profile = assessed.profile;
    println!(
        "machine profile: {}",
        profile.map(podbox_probe::machine::Profile::word).unwrap_or("none")
    );
    match w::accel_for(profile) {
        Some(accel) => {
            println!("accelerator: {} (cpu {})", accel.word(), accel.cpu());
            if accel == w::Accel::Tcg {
                println!(
                    "⚠ tcg: the emulator process is the boundary, not hardware \
                     isolation. This is the portable profile and it is slower; \
                     it is not a refusal."
                );
            }
            println!("emulator: {}", emulator().display());
            println!("firmware share: {}", share().display());
            let (code, vars) = firmware();
            println!("ovmf code: {}", code.display());
            println!("ovmf vars: {}", vars.display());
        }
        None => {
            let why = assessed
                .refusal()
                .unwrap_or_else(|| "no profile holds".to_string());
            eprintln!("podbox windows: {why}");
            return EXIT_RUNTIME_ERROR;
        }
    }
    0
}

/// Where `setup` leaves the provisioned image: beside the vendor's, under a
/// name that says which one it came from.
///
/// ⛔ **The provisioning writes have to outlive the boot that made them, and
/// the first revision got that wrong.** `run`'s disk is a disposable overlay
/// over the base image, so an installer that wrote into a scratch overlay left
/// the agent in a file `run` then threw away: the guest booted, the task was
/// gone, and every run timed out. `setup` therefore produces a *new base* —
/// the vendor's image plus the agent — and `run` makes its overlay over that.
pub fn provisioned_path(image: &Path) -> PathBuf {
    let stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "windows".to_string());
    image.with_file_name(format!("{stem}.podbox.qcow2"))
}

/// `setup`: provision the image once, into [`provisioned_path`].
fn setup(a: &Args) -> i32 {
    let Some(image) = a.image.clone() else {
        eprintln!("podbox windows setup: --image is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    if !image.is_file() {
        eprintln!("podbox windows setup: {} is not a file", image.display());
        return EXIT_RUNTIME_ERROR;
    }
    let out = provisioned_path(&image);
    if out.exists() {
        eprintln!(
            "podbox windows setup: {} already exists; remove it to provision again",
            out.display()
        );
        return EXIT_FLAG_ERROR;
    }
    let (mut plan, _) = match plan_for(a, &image) {
        Ok(p) => p,
        Err(c) => return c,
    };
    // ⚠ The provisioned image IS this boot's disk, not a disposable overlay
    // over it: what the installer writes here has to survive.
    plan.root = out.clone();
    match w::provision(
        &qemu_img(),
        &image,
        &plan,
        Duration::from_secs(a.timeout.unwrap_or(240)),
    ) {
        Ok(report) => {
            println!("{}", String::from_utf8_lossy(&report));
            println!("provisioned {}", out.display());
            0
        }
        Err(e) => {
            eprintln!("podbox windows setup: {e}");
            eprintln!("podbox windows setup: nothing was left at {}", out.display());
            let _ = std::fs::remove_file(&out);
            EXIT_RUNTIME_ERROR
        }
    }
}

/// `run`: boot, execute one line, and report everything back.
fn run(a: &Args) -> i32 {
    let Some(image) = a.image.clone() else {
        eprintln!("podbox windows run: --image is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    if a.command.is_empty() {
        eprintln!("podbox windows run: no command was given\n{USAGE}");
        return EXIT_FLAG_ERROR;
    }
    let ceiling = match ceiling() {
        Ok(c) => c,
        Err(c) => return c,
    };
    if let Ok(meta) = std::fs::metadata(&image) {
        if crate::tier::mem_over_ceiling(meta.len(), ceiling) {
            eprintln!(
                "podbox windows run: the image {} is {} bytes, over the \
                 RLIMIT_FSIZE ceiling {ceiling} bytes (TODO/podvm.md T-1305)",
                image.display(),
                meta.len()
            );
            return EXIT_RUNTIME_ERROR;
        }
    }
    let (plan, accel) = match plan_for(a, &image) {
        Ok(p) => p,
        Err(c) => return c,
    };
    if accel == w::Accel::Tcg {
        eprintln!(
            "podbox windows: tcg on this machine: the emulator process is the \
             boundary, not hardware isolation"
        );
    }
    let request = w::Request {
        command: a.command.clone(),
        token: token(),
        timeout: Duration::from_secs(a.timeout.unwrap_or(900)),
    };
    if let Err(e) = w::stage(&qemu_img(), &image, &plan, w::agent::AGENT_CMD, &request) {
        eprintln!("podbox windows run: {e}");
        return EXIT_RUNTIME_ERROR;
    }
    match w::run(&plan, &request) {
        Ok(out) => {
            use std::io::Write;
            let mut out_stream = std::io::stdout().lock();
            let _ = out_stream.write_all(&out.stdout);
            let _ = out_stream.flush();
            let mut err = std::io::stderr().lock();
            let _ = err.write_all(&out.stderr);
            let _ = err.flush();
            // The guest's own exit code is the run's, so a caller can gate on
            // it the way it would on any other command.
            out.code
        }
        Err(e) => {
            // The mailbox is kept where the run left it: a refusal with no
            // artifact is a refusal nobody can act on.
            eprintln!("podbox windows run: {e}");
            eprintln!(
                "podbox windows: the mailbox is at {}",
                plan.mailbox.display()
            );
            EXIT_RUNTIME_ERROR
        }
    }
}

/// A token for this run: the process id and the wall clock, which is enough
/// to tell two in-flight runs apart on one machine.
fn token() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{t}", std::process::id())
}

/// `podbox windows`.
pub fn windows(args: &[String]) -> i32 {
    let a = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return 0,
        Err(code) => return code,
    };
    match a.sub.as_str() {
        "doctor" => doctor(&a),
        "setup" => setup(&a),
        "run" => run(&a),
        other => {
            eprintln!("podbox windows: {other} is not a subcommand\n{USAGE}");
            EXIT_FLAG_ERROR
        }
    }
}
