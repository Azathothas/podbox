//! podvm-workload: what does the machine tier cost per workload class.
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
//! checksums; 1. host runs, three per class; 2. chroot runs through podvm,
//! three per class; 3. guest assembly, batch boot, console parsed; 4. one
//! row per class: medians, ratios, checksum agreement.
//!
//! Usage: podvm-workload [--root DIR] [--bin PATH]
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

// Every input pinned, as in the script's 146 shape: the image and the
// kernel (TODO/deps.md T-1679).
#[path = "workload/pins.rs"]
mod pins;
use pins::*;

// The netboot archive of 3.22.6, not `netboot/vmlinuz-virt`: that directory
// follows each point release, and the pinned kernel is the one of 3.22.6
// (T-1687, PodUtils/podcrates#6).
/// The init script of the guest. It ends with `reboot -f`, not `poweroff
/// -f`: on `-M pc,acpi=off` the kernel cannot power off and halts, so qemu
/// never exits; a reset with `-no-reboot` makes qemu exit at once (T-1702).
const GUEST_INIT: &str = "#!/bin/sh\necho VMR-BENCH-READY\ncd /\nfor b in bench-int bench-sys bench-mem bench-io; do\n\tfor i in 1 2 3; do\n\t\t/bench/$b\n\tdone\ndone\necho VMR-BENCH-DONE\nreboot -f\n";

const KARCHIVE_MEMBER: &str = "boot/vmlinuz-virt";
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

/// Runs a long guest with its console in a file, so that a timeout keeps
/// what the guest printed (T-1702). The child dies at the timeout: before
/// this the tool dropped the pipes of a child that kept running, which
/// showed as 0 console bytes and left the emulator alive. Returns the exit
/// success, or `None` where the child did not spawn or was killed at the
/// timeout.
fn run_to_file(prog: &str, args: &[&str], cwd: &Path, log: &Path, secs: u64) -> Option<bool> {
    let out = fs::File::create(log).ok()?;
    let err = out.try_clone().ok()?;
    let mut child = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) if start.elapsed() < Duration::from_secs(secs) => {
                std::thread::sleep(Duration::from_millis(200));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
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
            if !digits.is_empty()
                && let Ok(value) = digits.parse::<f64>()
            {
                values.push((value, digits));
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

fn drive(root: &Path, bin_flag: Option<String>) -> i32 {
    let out = root.join("experiments/results/tcg-workload-spread.txt");
    let mut report = Report {
        text: String::new(),
        driven: 0,
        fails: 0,
        failed: false,
    };
    // The netboot archive is about 300 MB (T-1687), so the default allows
    // ten minutes.
    let curl_timeout: u64 = std::env::var("PODVM_154_CURL_TIMEOUT")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(600);
    let boot_timeout: u64 = std::env::var("PODVM_154_TIMEOUT")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(600);
    let binary = match bin_flag {
        Some(path) => PathBuf::from(path),
        None => std::env::var("PODVM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("target/x86_64-unknown-linux-musl/release/podvm")),
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
        "podvm-workload-{}-{}",
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

#[path = "workload/drive.rs"]
mod drive;
use drive::*;

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
            eprintln!("podvm-workload: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podvm-workload: --root needs a directory");
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
            eprintln!("podvm-workload: --bin needs a path");
            std::process::exit(2);
        }
        None => None,
    };
    std::process::exit(drive(&root, bin));
}

#[cfg(test)]
#[path = "workload/tests.rs"]
mod tests;
