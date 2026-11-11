//! `podvm probe-inside` and `podvm compare` (TODO/probe.md T-1666).
//!
//! `probe-inside` runs podvm's own static binary as the payload of a `run`:
//! `prepare` copies it into the container's own tree (T-1709) at
//! [`GUEST`], or into the guest of the machine tier, and the payload's
//! stdout is the report of `podvm_probe::inside`. So the probe needs no
//! interpreter in the image: busybox and distroless images take it.
//! `compare` reads a report and a reference.

use std::io::Write;
use std::sync::OnceLock;

use podvm_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podvm_probe::inside;

/// The guest path of the probe.
pub const GUEST: &str = "/.podvm/sandprobe";

/// Set by `probe-inside` for its one `run`: the host path of the probe.
static PROBE: OnceLock<std::path::PathBuf> = OnceLock::new();

const USAGE_INSIDE: &str = "\
usage: podvm probe-inside [--podvm-tier T] [--profile P [-s KEY=VALUE]...]
                          [--platform P] [-o FILE] IMAGE

  Run podvm's static probe inside IMAGE on the tier that a run takes, and
  write its report: one record SECTION<TAB>TARGET<TAB>VERDICT<TAB>DETAIL a
  line, with each secret-shaped value REDACTED (TODO/probe.md T-1666).";

const USAGE_COMPARE: &str = "\
usage: podvm compare REPORT --reference REF [--expected FILE] [--strict]

  Compare two reports of probe-inside. Only a difference in an invariant
  section with no cause in the expected-differences file fails, and only
  with --strict (exit 1) (TODO/probe.md T-1666).";

/// Place the probe in a rootfs of `prepare`, where `probe-inside` runs.
pub fn place(rootfs: &str) -> Result<(), String> {
    let Some(src) = PROBE.get() else {
        return Ok(());
    };
    let dst = std::path::Path::new(rootfs).join(GUEST.trim_start_matches('/'));
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::copy(src, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    std::fs::set_permissions(&dst, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .map_err(|e| format!("{}: {e}", dst.display()))
}

/// The host directory that holds the probe, for the copy into a guest of
/// the machine tier (at `/.podvm`).
pub fn guest_copy(store_root: &std::path::Path) -> Option<std::path::PathBuf> {
    let src = PROBE.get()?;
    let dir = store_root
        .join("staging")
        .join(format!("sandprobe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::copy(src, dir.join("sandprobe")).ok()?;
    Some(dir)
}

/// The variable that names a host path (the store) for the escape check.
pub fn host_env(store_root: &std::path::Path) -> Option<String> {
    PROBE.get()?;
    Some(format!("{}={}", inside::HOST_VAR, store_root.display()))
}

/// `podvm probe-inside`.
pub fn probe_inside(args: &[String]) -> i32 {
    let mut pass: Vec<String> = Vec::new();
    let mut out: Option<String> = None;
    let mut image: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE_INSIDE}");
                return 0;
            }
            "-o" | "--output" => out = it.next().cloned(),
            "--podvm-tier" | "--profile" | "-s" | "--set" | "--platform" | "--pull" => {
                let Some(v) = it.next() else {
                    eprintln!("podvm probe-inside: {a} needs a value\n{USAGE_INSIDE}");
                    return EXIT_FLAG_ERROR;
                };
                pass.push(a.clone());
                pass.push(v.clone());
            }
            x if [
                "--podvm-tier=",
                "--profile=",
                "--set=",
                "--platform=",
                "--pull=",
            ]
            .iter()
            .any(|p| x.starts_with(p)) =>
            {
                pass.push(a.clone())
            }
            x if x.starts_with("-o=") || x.starts_with("--output=") => {
                out = x.split_once('=').map(|(_, v)| v.to_string())
            }
            x if x.starts_with('-') => {
                eprintln!("podvm probe-inside: unknown option {x:?}\n{USAGE_INSIDE}");
                return EXIT_FLAG_ERROR;
            }
            x if image.is_none() => image = Some(x.to_string()),
            x => {
                eprintln!("podvm probe-inside: one IMAGE only, and {x:?} is a second");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    let Some(image) = image else {
        eprintln!("{USAGE_INSIDE}");
        return EXIT_FLAG_ERROR;
    };
    let me = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("podvm probe-inside: /proc/self/exe: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let _ = PROBE.set(me);
    if let Some(path) = &out {
        // The report is the payload's stdout: it goes to the file.
        let file = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("podvm probe-inside: -o {path}: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        let mut f = &file;
        let _ = writeln!(f, "# image {image}\n# flags {}", pass.join(" "));
        use std::os::fd::AsRawFd;
        // SAFETY: dup3 of an open file onto stdout.
        let r = unsafe {
            podvm_probe::sys::sys(
                podvm_probe::sys::SYS_DUP3,
                [file.as_raw_fd() as u64, 1, 0, 0, 0, 0],
            )
        };
        if let Err(e) = r {
            eprintln!("podvm probe-inside: -o {path}: dup3: {}", e.name());
            return EXIT_RUNTIME_ERROR;
        }
    }
    let mut run = vec!["--rm".to_string()];
    run.extend(pass);
    run.extend([
        "--entrypoint".to_string(),
        GUEST.to_string(),
        image,
        inside::ARG.to_string(),
    ]);
    crate::run::run(&run)
}

/// `podvm compare`.
pub fn compare(args: &[String]) -> i32 {
    let (mut report, mut reference, mut expected, mut strict) = (None, None, None, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE_COMPARE}");
                return 0;
            }
            "--reference" => reference = it.next().cloned(),
            "--expected" => expected = it.next().cloned(),
            "--strict" => strict = true,
            x if x.starts_with('-') => {
                eprintln!("podvm compare: unknown option {x:?}\n{USAGE_COMPARE}");
                return EXIT_FLAG_ERROR;
            }
            x if report.is_none() => report = Some(x.to_string()),
            x => {
                eprintln!("podvm compare: one REPORT only, and {x:?} is a second");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    let (Some(report), Some(reference)) = (report, reference) else {
        eprintln!("{USAGE_COMPARE}");
        return EXIT_FLAG_ERROR;
    };
    let read = |p: &str| {
        std::fs::read_to_string(p).map_err(|e| {
            eprintln!("podvm compare: {p}: {e}");
            EXIT_RUNTIME_ERROR
        })
    };
    let here = match read(&report) {
        Ok(t) => inside::parse_report(&t),
        Err(c) => return c,
    };
    let refs = match read(&reference) {
        Ok(t) => inside::parse_report(&t),
        Err(c) => return c,
    };
    let exp = match expected.as_deref().map(read) {
        Some(Ok(t)) => inside::compare::parse_expected(&t),
        Some(Err(c)) => return c,
        None => Vec::new(),
    };
    let o = inside::compare::compare(&here, &refs, &exp);
    println!("{}", o.summary());
    for d in &o.differ {
        println!(
            "{}\t{}\t{} here, {} in the reference\t{}",
            d.section,
            d.target,
            d.here.word(),
            d.reference.word(),
            match (&d.reason, d.invariant()) {
                (Some(r), _) => format!("explained: {r}"),
                (None, true) => "UNEXPLAINED in an invariant section".to_string(),
                (None, false) => "not an invariant section".to_string(),
            }
        );
    }
    o.code(strict)
}
