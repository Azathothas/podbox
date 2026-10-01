//! podbox-podvm-workload: what does the machine tier cost per workload class.
//!
//! A behaviour-preserving port of experiments/154-tcg-workload-spread.sh
//! (T-1569). The script stays as a compat shim that execs this binary; the
//! logic lives here. One payload binary per class runs on three platforms,
//! so equal checksums are the control that every platform computed the same
//! thing. The I/O backing differs per platform and is named in the
//! conditions.
//!
//! The guest assembly (base-plus-extras initramfs) is ported, not shelled
//! out to: the base is a `find`/`cpio` pipeline driven without a shell,
//! and the extras archive is a byte-exact newc writer. The console node is
//! bytes in the archive, never a device the host creates.
//!
//! Clauses: 0. conditions and the four static payloads with pinned
//! checksums; 1. host runs, three per class; 2. chroot runs through podbox,
//! three per class; 3. guest assembly, batch boot, console parsed; 4. one
//! row per class: medians, ratios, checksum agreement.
//!
//! Usage: podbox-podvm-workload [--root DIR] [--bin PATH]
//!
//! Exit: 0 every row printed with agreeing checksums, 1 a checksum
//! disagreed or a run failed, 2 a tool, the binary or an input could not
//! run. The guest half needs qemu, a C toolchain, and the network; without
//! them the binary stops at the clause that needs them with exit 2, never
//! with a partial table presented as a result.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

// Every input pinned, as in the script's 146 shape.
const ALPINE_REF: &str = "public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764";
const KURL: &str =
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt";
const KERNEL_SHA256: &str = "6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51";
const QEMU_FLAGS: &str = "-M pc,acpi=off -m 256 -nographic -no-reboot -accel tcg,thread=multi";
const BENCH_CFLAGS: &str = "-O2 -static";
const RUN_TIMEOUT: u64 = 300;
const CLASSES: [&str; 4] = ["int", "sys", "mem", "io"];

fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn run_capture(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    env_extra: &[(&str, &str)],
    secs: u64,
) -> Option<(bool, String, String)> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) => Some((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
        _ => None,
    }
}

fn command_exists(name: &str) -> bool {
    match std::env::var_os("PATH") {
        None => false,
        Some(paths) => std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()),
    }
}

// ------------------------------------------------------------- pure half ---

/// The median of the three elapsed values, or empty where the log holds
/// anything but three runs: a median of fewer runs is a wrong number, not
/// an approximation.
fn elapsed_median(workload: &str, log: &str) -> String {
    let prefix = format!("workload={workload} ");
    let mut values: Vec<(f64, String)> = Vec::new();
    for line in log.split('\n') {
        if !line.starts_with(&prefix) {
            continue;
        }
        if let Some(start) = line.find("elapsed=") {
            let digits: String = line[start + "elapsed=".len()..]
                .chars()
                .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
                .collect();
            if !digits.is_empty() {
                if let Ok(value) = digits.parse::<f64>() {
                    values.push((value, digits));
                }
            }
        }
    }
    if values.len() != 3 {
        return String::new();
    }
    values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    values[1].1.clone()
}

/// Every checksum printed for the class, deduplicated and sorted.
fn checksums(workload: &str, log: &str) -> Vec<String> {
    let prefix = format!("workload={workload} ");
    let mut sums = BTreeSet::new();
    for line in log.split('\n') {
        if !line.starts_with(&prefix) {
            continue;
        }
        if let Some(start) = line.find("checksum=") {
            let hex: String = line[start + "checksum=".len()..]
                .chars()
                .take_while(|ch| ch.is_ascii_hexdigit())
                .collect();
            if !hex.is_empty() {
                sums.insert(hex);
            }
        }
    }
    sums.into_iter().collect()
}

/// A/B to one decimal, or `-` where the numbers cannot divide.
fn ratio(first: &str, second: &str) -> String {
    let (Ok(a), Ok(b)) = (first.parse::<f64>(), second.parse::<f64>()) else {
        return "-".to_string();
    };
    if b > 0.0 && a > 0.0 {
        format!("{:.1}", a / b)
    } else {
        "-".to_string()
    }
}

// ------------------------------------------------------- newc writer --------

/// One newc record, byte-identical to the lib's Python writer: eight hex
/// fields, the NUL-terminated name, and 4-byte alignment of header and
/// data. The console node is these bytes, never a host device.
fn newc(
    name: &str,
    mode: u32,
    filesize: usize,
    rdevmaj: u32,
    rdevmin: u32,
    data: &[u8],
) -> Vec<u8> {
    let name_bytes = format!("{name}\0");
    let mut record = format!(
        "07070100000000{mode:08x}00000000000000000000000100000000{filesize:08x}0000000000000000{rdevmaj:08x}{rdevmin:08x}{:08x}00000000",
        name_bytes.len()
    )
    .into_bytes();
    record.extend_from_slice(name_bytes.as_bytes());
    while record.len() % 4 != 0 {
        record.push(0);
    }
    record.extend_from_slice(data);
    while record.len() % 4 != 0 {
        record.push(0);
    }
    record
}

/// The appended archive for one /init override: dev/, dev/console 5:1, the
/// init file executable, and the trailer.
fn extras_archive(init: &[u8]) -> Vec<u8> {
    let mut archive = newc("dev", 0o040755, 0, 0, 0, b"");
    archive.extend_from_slice(&newc("dev/console", 0o020600, 0, 5, 1, b""));
    archive.extend_from_slice(&newc("init", 0o100755, init.len(), 0, 0, init));
    archive.extend_from_slice(&newc("TRAILER!!!", 0, 0, 0, 0, b""));
    archive
}

/// The rootfs as an uncompressed newc archive: `find . -print0` piped to
/// `cpio -o -H newc --null`, driven without a shell so no path is parsed
/// twice.
fn base_archive(rootfs: &Path, out: &Path) -> bool {
    let find = Command::new("find")
        .args([".", "-print0"])
        .current_dir(rootfs)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut find = match find {
        Ok(child) => child,
        Err(_) => return false,
    };
    let from_find = match find.stdout.take() {
        Some(pipe) => pipe,
        None => return false,
    };
    let target = match fs::File::create(out) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let cpio = Command::new("cpio")
        .args(["--quiet", "-o", "-H", "newc", "--null"])
        .current_dir(rootfs)
        .stdin(from_find)
        .stdout(target)
        .stderr(Stdio::null())
        .spawn();
    let mut cpio = match cpio {
        Ok(child) => child,
        Err(_) => return false,
    };
    let cpio_ok = cpio.wait().map(|status| status.success()).unwrap_or(false);
    let find_ok = find.wait().map(|status| status.success()).unwrap_or(false);
    cpio_ok && find_ok && out.is_file()
}

// ----------------------------------------------------------------- driver ---

struct Report {
    text: String,
    driven: u32,
    fails: u32,
    failed: bool,
}

impl Report {
    fn say(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }
    fn pass(&mut self, what: &str) {
        self.driven += 1;
        self.say(&format!("  ok: {what}"));
    }
    fn miss(&mut self, what: &str) {
        self.failed = true;
        self.fails += 1;
        self.say(&format!("  FAIL: {what}"));
    }
}

fn main() {
    let mut root_flag: Option<String> = None;
    let mut bin_flag: Option<String> = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if arg == "--bin" {
            bin_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else if let Some(path) = arg.strip_prefix("--bin=") {
            bin_flag = Some(path.to_string());
        } else {
            eprintln!("podbox-podvm-workload: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podbox-podvm-workload: --root needs a directory");
            std::process::exit(2);
        }
        None => repo_root().unwrap_or_else(|| {
            eprintln!("SKIP: cannot locate the checkout root");
            std::process::exit(2);
        }),
    };
    let bin = match bin_flag {
        Some(path) if !path.is_empty() => Some(path),
        Some(_) => {
            eprintln!("podbox-podvm-workload: --bin needs a path");
            std::process::exit(2);
        }
        None => None,
    };
    std::process::exit(drive(&root, bin));
}

fn drive(root: &Path, bin_flag: Option<String>) -> i32 {
    let out = root.join("experiments/results/tcg-workload-spread.txt");
    let mut report = Report {
        text: String::new(),
        driven: 0,
        fails: 0,
        failed: false,
    };
    let curl_timeout: u64 = std::env::var("PODBOX_154_CURL_TIMEOUT")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(60);
    let boot_timeout: u64 = std::env::var("PODBOX_154_TIMEOUT")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(600);
    let binary = match bin_flag {
        Some(path) => PathBuf::from(path),
        None => std::env::var("PODBOX_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("target/x86_64-unknown-linux-musl/release/podbox")),
    };
    if !binary.is_file() {
        eprintln!("SKIP: {} is not an executable. Build it:", binary.display());
        eprintln!("      cargo build --release --target x86_64-unknown-linux-musl");
        return 2;
    }
    // Only the tools the binary actually drives. The lib's Python writer
    // and the script's text utilities are ported, so they are not needed.
    for tool in [
        "qemu-system-x86_64",
        "cpio",
        "curl",
        "sha256sum",
        "gcc",
        "file",
    ] {
        if !command_exists(tool) {
            eprintln!("SKIP: {tool} is not on PATH");
            return 2;
        }
    }
    let name = binary.to_string_lossy().into_owned();
    let scratch = std::env::temp_dir().join(format!(
        "podbox-workload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    if fs::create_dir_all(&scratch).is_err() {
        eprintln!("SKIP: cannot stage payloads");
        return 2;
    }
    let outcome = drive_inner(
        root,
        &name,
        &scratch,
        curl_timeout,
        boot_timeout,
        &mut report,
    );
    report.say("");
    report.say(&format!(
        "== counts: {} driven, {} mismatches",
        report.driven, report.fails
    ));
    print!("{}", report.text);
    if fs::create_dir_all(out.parent().unwrap_or_else(|| Path::new("."))).is_ok() {
        fs::write(&out, &report.text).ok();
    }
    println!();
    println!(
        "written to {}",
        out.strip_prefix(root).unwrap_or(&out).display()
    );
    // Copy to /out where a lane runner collects evidence, like the
    // lock-inheritance proof: the report beside the full boot log.
    let out_dir = Path::new("/out");
    if out_dir.is_dir() {
        fs::copy(&out, out_dir.join("tcg-workload-spread.txt")).ok();
        fs::copy(scratch.join("boot.log"), out_dir.join("workload-boot.log")).ok();
    }
    fs::remove_dir_all(&scratch).ok();
    outcome
}

/// Three bounded runs, raw lines appended: every platform counts
/// `^workload=` lines, never exits.
fn bench3(
    binary: &str,
    dir: &Path,
    log: &Path,
    extra_env: &[(&str, &str)],
    run_args: &[&str],
    secs: u64,
) -> bool {
    let mut combined = String::new();
    for _ in 1..=3 {
        match run_capture(binary, run_args, dir, extra_env, secs) {
            Some((true, stdout, stderr)) => {
                combined.push_str(&stdout);
                combined.push_str(&stderr);
            }
            _ => return false,
        }
    }
    fs::write(log, combined).is_ok()
}

fn drive_inner(
    root: &Path,
    binary: &str,
    scratch: &Path,
    curl_timeout: u64,
    boot_timeout: u64,
    report: &mut Report,
) -> i32 {
    let payloads = scratch.join("payloads");
    if fs::create_dir_all(&payloads).is_err() {
        eprintln!("SKIP: cannot stage payloads");
        return 2;
    }
    for class in CLASSES {
        let source = root.join(format!("experiments/154-bench-{class}.c"));
        let target = payloads.join(format!("bench-{class}"));
        let target_text = target.to_string_lossy().into_owned();
        let source_text = source.to_string_lossy().into_owned();
        let mut argv: Vec<&str> = BENCH_CFLAGS.split(' ').collect();
        argv.extend(["-o", &target_text, &source_text]);
        match run_capture("gcc", &argv, root, &[], 300) {
            Some((true, _, _)) => {}
            _ => {
                eprintln!("SKIP: bench-{class} did not build static");
                return 2;
            }
        }
        match run_capture("file", &[&target.to_string_lossy()], root, &[], 30) {
            Some((true, stdout, _)) if stdout.contains("statically linked") => {}
            _ => {
                eprintln!("SKIP: bench-{class} is not static");
                return 2;
            }
        }
    }

    report.say("== conditions");
    report.say(&format!(
        "date              {}",
        run_capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root, &[], 10)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "host kernel       {}",
        run_capture("uname", &["-r"], root, &[], 10)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "podbox            {}",
        run_capture(binary, &["version"], root, &[], 60)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "qemu              {}",
        run_capture("qemu-system-x86_64", &["--version"], root, &[], 30)
            .map(|(_, t, _)| t.lines().next().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "gcc               {}",
        run_capture("gcc", &["--version"], root, &[], 30)
            .map(|(_, t, _)| t.lines().next().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!("image             {ALPINE_REF}"));
    report.say(&format!(
        "kernel            {KURL} (sha256 {KERNEL_SHA256})"
    ));
    report.say(&format!("qemu flags        {QEMU_FLAGS}"));
    report.say(&format!("payload cflags    gcc {BENCH_CFLAGS}"));
    report.say(&format!(
        "host io backing   {}",
        run_capture("df", &["-T", &scratch.to_string_lossy()], root, &[], 30)
            .map(|(_, t, _)| t.lines().last().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    for class in CLASSES {
        let digest = run_capture(
            "sha256sum",
            &[&payloads.join(format!("bench-{class}")).to_string_lossy()],
            root,
            &[],
            30,
        )
        .map(|(_, t, _)| t.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_default();
        report.say(&format!("payload bench-{class}  {digest}"));
    }
    report.say("");

    report.say("== 1. host runs");
    let host_dir = scratch.join("host");
    if fs::create_dir_all(&host_dir).is_err() {
        eprintln!("SKIP: cannot stage host runs");
        return 2;
    }
    for class in CLASSES {
        let target = payloads.join(format!("bench-{class}"));
        let log = scratch.join(format!("host-{class}.log"));
        if !bench3(
            &target.to_string_lossy(),
            &host_dir,
            &log,
            &[],
            &[],
            RUN_TIMEOUT,
        ) {
            report.miss(&format!("host bench-{class} failed"));
            continue;
        }
        let text = fs::read_to_string(&log).unwrap_or_default();
        let count = text
            .split('\n')
            .filter(|line| line.starts_with(&format!("workload={class} ")))
            .count();
        if count == 3 {
            report.pass(&format!("host bench-{class} ran three times"));
        } else {
            report.miss(&format!("host bench-{class} ran {count} of 3 times"));
        }
    }

    report.say("");
    report.say("== 2. chroot runs through podbox");
    // A store of this run's own, so the measurement never touches the
    // operator's store.
    let store = scratch.join("store").to_string_lossy().into_owned();
    let store_env = [("PODBOX_STORE", store.as_str())];
    match run_capture(binary, &["pull", ALPINE_REF], root, &store_env, 900) {
        Some((true, _, _)) => {}
        Some((false, stdout, stderr)) => {
            report.miss("the pull failed");
            for line in format!("{stdout}{stderr}")
                .split('\n')
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .iter()
                .rev()
            {
                report.say(line);
            }
        }
        None => {
            report.miss("the pull failed");
        }
    }
    let mut rootfs = String::new();
    if !report.failed {
        match run_capture(binary, &["extract", ALPINE_REF], root, &store_env, 900) {
            Some((true, stdout, _))
                if !stdout.trim().is_empty() && Path::new(stdout.trim()).is_dir() =>
            {
                rootfs = stdout.trim().to_string();
            }
            _ => {
                report.miss("no rootfs extracted");
            }
        }
    }
    let mut chroot_logs: BTreeSet<String> = BTreeSet::new();
    if !rootfs.is_empty() {
        if fs::create_dir_all(Path::new(&rootfs).join("bench")).is_err() {
            eprintln!("SKIP: cannot stage payloads into the rootfs");
            return 2;
        }
        for class in CLASSES {
            if fs::copy(
                payloads.join(format!("bench-{class}")),
                Path::new(&rootfs).join(format!("bench/bench-{class}")),
            )
            .is_err()
            {
                eprintln!("SKIP: cannot stage payloads into the rootfs");
                return 2;
            }
        }
        report.say(&format!(
            "chroot io backing {}",
            run_capture("df", &["-T", &rootfs], root, &[], 30)
                .map(|(_, t, _)| t.lines().last().unwrap_or("").trim().to_string())
                .unwrap_or_default()
        ));
        if fs::create_dir_all(scratch.join("chroot")).is_err() {
            eprintln!("SKIP: cannot stage chroot runs");
            return 2;
        }
        for class in CLASSES {
            let log = scratch.join(format!("chroot-{class}.log"));
            let err_log = scratch.join(format!("chroot-{class}.err"));
            let guest_path = format!("/bench/bench-{class}");
            let mut combined = String::new();
            let mut errors = String::new();
            for _ in 1..=3 {
                // The script breaks the loop on a failed run and counts
                // lines, so a short log fails the row below, not here.
                match run_capture(
                    binary,
                    &["run", "--pull=never", ALPINE_REF, &guest_path],
                    &scratch.join("chroot"),
                    &store_env,
                    RUN_TIMEOUT,
                ) {
                    Some((true, stdout, stderr)) => {
                        combined.push_str(&stdout);
                        errors.push_str(&stderr);
                    }
                    Some((false, stdout, stderr)) => {
                        combined.push_str(&stdout);
                        errors.push_str(&stderr);
                        errors.push_str("run rc=1\n");
                        break;
                    }
                    None => {
                        errors.push_str("run timed out\n");
                        break;
                    }
                }
            }
            fs::write(&log, &combined).ok();
            fs::write(&err_log, &errors).ok();
            chroot_logs.insert(class.to_string());
            let count = combined
                .split('\n')
                .filter(|line| line.starts_with(&format!("workload={class} ")))
                .count();
            if count == 3 {
                report.pass(&format!(
                    "chroot bench-{class} ran through podbox three times"
                ));
            } else {
                report.miss(&format!("chroot bench-{class} ran {count} of 3 times"));
                for line in errors
                    .split('\n')
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                {
                    report.say(line);
                }
            }
        }
    }

    report.say("");
    report.say("== 3. guest assembly and batch boot");
    let mut boot_log = String::new();
    if !report.failed && !rootfs.is_empty() {
        if base_archive(Path::new(&rootfs), &scratch.join("base.cpio")) {
            report.say("  base archive built");
        } else {
            report.miss("the base cpio did not build");
        }
    }
    if !report.failed {
        let init = "#!/bin/sh\necho VMR-BENCH-READY\ncd /\nfor b in bench-int bench-sys bench-mem bench-io; do\n\tfor i in 1 2 3; do\n\t\t/bench/$b\n\tdone\ndone\necho VMR-BENCH-DONE\npoweroff -f\n";
        let extras = extras_archive(init.as_bytes());
        if fs::write(scratch.join("extras.cpio"), &extras).is_err() || extras.is_empty() {
            report.miss("the extras archive did not build");
        }
        let mut full = fs::read(scratch.join("base.cpio")).unwrap_or_default();
        full.extend_from_slice(&extras);
        if full.is_empty() || fs::write(scratch.join("full.cpio"), &full).is_err() {
            report.miss("concat failed");
        } else {
            report.pass(&format!("assembly {} bytes", full.len()));
        }
    }
    if !report.failed {
        let kernel = scratch.join("vmlinuz-virt");
        match run_capture(
            "curl",
            &[
                "-fsSL",
                "--max-time",
                &curl_timeout.to_string(),
                "-o",
                &kernel.to_string_lossy(),
                KURL,
            ],
            root,
            &[],
            curl_timeout + 120,
        ) {
            Some((true, _, _)) => {}
            _ => {
                report.miss("the kernel did not fetch");
            }
        }
        if !report.failed {
            match run_capture("sha256sum", &[&kernel.to_string_lossy()], root, &[], 60) {
                Some((true, stdout, _)) if stdout.starts_with(KERNEL_SHA256) => {
                    report.say("  kernel hash matches the pin");
                }
                Some((true, stdout, _)) => {
                    report.say(&format!(
                        "  kernel sha256: {}",
                        stdout.split_whitespace().next().unwrap_or("")
                    ));
                    report.miss("the kernel hash does not match the pin");
                }
                _ => {
                    report.miss("the kernel hash does not match the pin");
                }
            }
        }
    }
    if !report.failed && scratch.join("full.cpio").is_file() {
        let mut argv: Vec<&str> = QEMU_FLAGS.split(' ').collect();
        let kernel = scratch.join("vmlinuz-virt").to_string_lossy().into_owned();
        let initrd = scratch.join("full.cpio").to_string_lossy().into_owned();
        argv.extend([
            "-kernel",
            &kernel,
            "-initrd",
            &initrd,
            "-append",
            "console=ttyS0 panic=-1",
        ]);
        let boot = scratch.join("boot.log");
        match run_capture("qemu-system-x86_64", &argv, root, &[], boot_timeout) {
            Some((ok, stdout, stderr)) => {
                fs::write(&boot, format!("{stdout}{stderr}")).ok();
                boot_log = format!("{stdout}{stderr}");
                report.say(&format!(
                    "  qemu exited {} with {} console bytes",
                    if ok { "0" } else { "nonzero" },
                    boot_log.len()
                ));
            }
            None => {
                boot_log = fs::read_to_string(&boot).unwrap_or_default();
                report.say(&format!(
                    "  qemu did not return in {boot_timeout}s or failed to spawn; {} console bytes on file",
                    boot_log.len()
                ));
            }
        }
        // The script records the rc without asserting on it: 124 is the
        // halt the spec records, because poweroff halts on pc,acpi=off.
        report.say("  qemu ran to the boot log");
        if boot_log.contains("VMR-BENCH-DONE") {
            report.pass("the guest ran the batch to VMR-BENCH-DONE");
        } else {
            report.miss("no done marker in the boot log");
            for line in boot_log
                .split('\n')
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .iter()
                .rev()
            {
                report.say(line);
            }
        }
    }

    report.say("");
    report.say("== 4. one row per class");
    let boot_text = fs::read_to_string(scratch.join("boot.log")).unwrap_or(boot_log);
    for class in CLASSES {
        let host_text =
            fs::read_to_string(scratch.join(format!("host-{class}.log"))).unwrap_or_default();
        let chroot_text = if chroot_logs.contains(class) {
            fs::read_to_string(scratch.join(format!("chroot-{class}.log"))).unwrap_or_default()
        } else {
            String::new()
        };
        let he = elapsed_median(class, &host_text);
        let ce = elapsed_median(class, &chroot_text);
        let ge = elapsed_median(class, &boot_text);
        let mut all = BTreeSet::new();
        for sum in checksums(class, &host_text)
            .into_iter()
            .chain(checksums(class, &chroot_text))
            .chain(checksums(class, &boot_text))
        {
            all.insert(sum);
        }
        let joined = all.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
        if all.len() == 1 && !he.is_empty() && !ce.is_empty() && !ge.is_empty() {
            report.pass(&format!(
                "{class}: host {he}s chroot {ce}s ({}x) guest {ge}s ({}x) checksum {joined}",
                ratio(&ce, &he),
                ratio(&ge, &he)
            ));
        } else {
            let hc = checksums(class, &host_text).join(" ");
            let cc = checksums(class, &chroot_text).join(" ");
            let gc = checksums(class, &boot_text).join(" ");
            report.miss(&format!(
                "{class}: host=[{he}/{hc}] chroot=[{ce}/{cc}] guest=[{ge}/{gc}]"
            ));
        }
    }
    if report.failed {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medians_need_exactly_three_runs() {
        let log = "workload=int elapsed=1.0 checksum=aa\nworkload=int elapsed=3.0 checksum=aa\nworkload=int elapsed=2.0 checksum=aa\n";
        assert_eq!(elapsed_median("int", log), "2.0");
        assert_eq!(
            elapsed_median("int", "workload=int elapsed=1.0 checksum=aa\n"),
            ""
        );
        assert_eq!(elapsed_median("sys", log), "");
        let frac = "workload=io elapsed=0.5 checksum=aa\nworkload=io elapsed=1.5 checksum=aa\nworkload=io elapsed=1.0 checksum=aa\n";
        assert_eq!(elapsed_median("io", frac), "1.0");
    }

    #[test]
    fn checksums_deduplicate_per_class() {
        let log = "workload=mem elapsed=1.0 checksum=aa\nworkload=mem elapsed=2.0 checksum=bb\nworkload=sys elapsed=1.0 checksum=cc\n";
        assert_eq!(
            checksums("mem", log),
            vec!["aa".to_string(), "bb".to_string()]
        );
        assert_eq!(checksums("sys", log), vec!["cc".to_string()]);
    }

    #[test]
    fn ratios_print_one_decimal_or_a_dash() {
        assert_eq!(ratio("2.0", "1.0"), "2.0");
        assert_eq!(ratio("1.0", "3.0"), "0.3");
        assert_eq!(ratio("1.0", "0.0"), "-");
        assert_eq!(ratio("", "1.0"), "-");
    }

    #[test]
    fn extras_archive_matches_the_lib_shape() {
        // dev/ directory, dev/console 5:1, an executable init, a trailer:
        // four records, each starting with the newc magic, 4-aligned.
        let archive = extras_archive(b"#!/bin/sh\n");
        assert!(archive.len() % 4 == 0);
        let magic = b"070701";
        let mut count = 0;
        for window in archive.windows(magic.len()) {
            if window == magic {
                count += 1;
            }
        }
        assert_eq!(count, 4);
        assert!(archive.windows(12).any(|window| window == b"dev/console\0"));
        assert!(archive.windows(11).any(|window| window == b"TRAILER!!!\0"));
    }
}
