//! `podbox machine ssh`: a shell in a Linux guest podbox itself runs.
//!
//! Podman parity (`podman machine ssh` opens a shell in podman's own VM),
//! kept on its own axis per `docs/decisions/remote-verb.md`: this is a
//! guest podbox starts, against `remote ssh`, which is a machine somebody
//! else started.
//!
//! The server placement is what makes this arm rather than a second
//! spelling of remote: the SSH server executes IN the guest. podbox boots
//! a Linux guest from the named kernel and initramfs with the first serial
//! port on a per-run unix socket, speaks real SSH over that socket with a
//! real `ssh` client through the lane's `proxy` binary, and reports the
//! guest command's exit code as its own. The guest provides the server
//! (dropbear or sshd on its serial line); the handshake itself is the
//! probe, so no separate probe can go stale beside it.
//!
//! One session per boot: an inetd-mode server answers one connection and
//! exits, so this arm runs one command (or one login shell) per boot.
//! That is the guest's shape, not `TODO/podvm.md` T-1304's: T-1304
//! multiplexes many commands over one serial session with per-command
//! nonces, while here the boot is the session.
//!
//! The `ssh` and `qemu-system-x86_64` binaries are trusted-PATH lookups,
//! resolved the way the SSH crate resolves its servers: named, required,
//! and refused where absent. The `proxy` helper resolves beside this
//! binary first, then on PATH, like the rest of the remote group.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use podbox_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podbox_windows::plan::Accel;

pub const USAGE: &str = "\
usage: podbox machine ssh --kernel <file> --initramfs <file> --user-key <file> [--user <name>] [-- command ...]
  --kernel <file>          the guest kernel, for example vmlinuz-virt
  --initramfs <file>       the guest initramfs, carrying the SSH server on
                           its serial console and the account to log in as
  --user-key <file>        the private key the guest authorises for --user
  --user <name>            the guest account (default root)
  --podbox-mem <size>      guest memory, for example 256M (default 256M)
  --podbox-timeout <secs>  stop the guest past this long (default 600, and a
                           stop is a refusal, never an empty success)
  --podbox-qemu-arg <arg>  one extra emulator argument, repeatable, appended
                           last so a caller can override a default
  -h, --help               print this usage and exit 0

  Boots a disposable Linux guest, runs one SSH command in it, and exits
  with the guest command's exit code. With no command, ssh opens the
  guest's login shell on this terminal.
";

/// Guest memory where `--podbox-mem` is absent.
const DEFAULT_MEM: u64 = 256 << 20;

/// The whole run where `--podbox-timeout` is absent.
const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// How often the serial wait retries the dial. The wait is a deadline,
/// not a sleep: every poll also checks the emulator is still alive.
const DIAL_POLL: Duration = Duration::from_millis(200);

/// `/dev/kvm` where it lives. A parameter rather than a constant so the
/// accelerator choice is unit-testable without the device.
const KVM_NODE: &str = "/dev/kvm";

/// Every dash-flag `parse` below accepts. The parity table carries one row
/// per entry, and the cross-check in `crate::parity`'s tests holds the two
/// lists identical in both directions, so a flag cannot land in one and
/// miss the other. The match arms below must list exactly these.
pub(crate) const FLAGS: &[&str] = &[
    "--kernel",
    "--initramfs",
    "--user-key",
    "--user",
    "--podbox-mem",
    "--podbox-timeout",
    "--podbox-qemu-arg",
];

/// One parsed `podbox machine ssh` invocation.
#[derive(Debug)]
pub(crate) struct Args {
    kernel: PathBuf,
    initramfs: PathBuf,
    user_key: PathBuf,
    user: String,
    mem: u64,
    timeout: Duration,
    emu_args: Vec<String>,
    command: Vec<String>,
}

/// One value after a flag, or the flag error that says which one is missing.
fn value<'a>(it: &mut std::slice::Iter<'a, String>, flag: &str) -> Result<&'a String, i32> {
    it.next().ok_or_else(|| {
        eprintln!("podbox machine ssh: {flag} needs a value\n{USAGE}");
        EXIT_FLAG_ERROR
    })
}

/// Parse the arm. `Ok(None)` is `-h/--help`: printed, and the exit is 0.
fn parse(args: &[String]) -> Result<Option<Args>, i32> {
    let mut kernel = None;
    let mut initramfs = None;
    let mut user_key = None;
    let mut user = "root".to_string();
    let mut mem = DEFAULT_MEM;
    let mut timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);
    let mut emu_args: Vec<String> = Vec::new();
    let mut command: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--" => {
                command.extend(it.by_ref().cloned());
                break;
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--kernel" => kernel = Some(PathBuf::from(value(&mut it, "--kernel")?)),
            "--initramfs" => initramfs = Some(PathBuf::from(value(&mut it, "--initramfs")?)),
            "--user-key" => user_key = Some(PathBuf::from(value(&mut it, "--user-key")?)),
            "--user" => user = value(&mut it, "--user")?.clone(),
            "--podbox-mem" => {
                let v = value(&mut it, "--podbox-mem")?;
                mem = crate::tier::parse_mem(v).ok_or_else(|| {
                    eprintln!("podbox machine ssh: --podbox-mem {v} is not a size\n{USAGE}");
                    EXIT_FLAG_ERROR
                })?;
            }
            "--podbox-timeout" => {
                let v = value(&mut it, "--podbox-timeout")?;
                let n: u64 = v.parse().map_err(|_| {
                    eprintln!(
                        "podbox machine ssh: --podbox-timeout {v} is not a count of seconds\n{USAGE}"
                    );
                    EXIT_FLAG_ERROR
                })?;
                if n == 0 {
                    eprintln!(
                        "podbox machine ssh: --podbox-timeout 0 is refused: name a positive count of seconds\n{USAGE}"
                    );
                    return Err(EXIT_FLAG_ERROR);
                }
                timeout = Duration::from_secs(n);
            }
            "--podbox-qemu-arg" => emu_args.push(value(&mut it, "--podbox-qemu-arg")?.clone()),
            other if other.starts_with('-') => {
                // FLAGS is the list this arm takes; the arms above are its
                // shape. Anything outside the list never reaches an arm, and
                // anything inside it without an arm above refuses rather
                // than falling through (the parity cross-test forbids that
                // drift, and the positive tests pin every arm).
                if !FLAGS.contains(&other) {
                    eprintln!("podbox machine ssh: {other} is not a flag this arm has\n{USAGE}");
                } else {
                    eprintln!("podbox machine ssh: {other} is listed but has no arm\n{USAGE}");
                }
                return Err(EXIT_FLAG_ERROR);
            }
            other => command.push(other.to_string()),
        }
    }
    let kernel = kernel.ok_or_else(|| {
        eprintln!("podbox machine ssh: --kernel names the guest kernel\n{USAGE}");
        EXIT_FLAG_ERROR
    })?;
    let initramfs = initramfs.ok_or_else(|| {
        eprintln!("podbox machine ssh: --initramfs names the guest initramfs\n{USAGE}");
        EXIT_FLAG_ERROR
    })?;
    let user_key = user_key.ok_or_else(|| {
        eprintln!("podbox machine ssh: --user-key names the key the guest authorises\n{USAGE}");
        EXIT_FLAG_ERROR
    })?;
    Ok(Some(Args {
        kernel,
        initramfs,
        user_key,
        user,
        mem,
        timeout,
        emu_args,
        command,
    }))
}

/// A named input that must be a file. A named path that is not a file is a
/// refusal, never a substitution: booting a different guest than the named
/// one and reporting its output as that guest's is the failure the Windows
/// driver documents, and this arm does not repeat it.
fn resolve_input(path: &Path, flag: &str) -> Result<PathBuf, i32> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    eprintln!(
        "podbox machine ssh: {flag} {} is not a file",
        path.display()
    );
    Err(EXIT_RUNTIME_ERROR)
}

/// True where the KVM node opens read-write. The sibling decision is
/// `podbox_windows::plan::accel_for` over the machine profile; this arm
/// asks only the one leg its argv needs (the emulator's presence is
/// checked at spawn), so a refusal elsewhere does not change which
/// accelerator a runnable guest gets. The check is openability, not
/// existence: a present node the caller cannot open is TCG, not KVM.
/// (Untestable as root, where permission bits do not bind: the test pins
/// the absent and openable arms and this comment owns the third.)
fn kvm_present(node: &str) -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(node)
        .is_ok()
}

/// Quote one word for `/bin/sh -c`, which is what ssh runs the
/// ProxyCommand through. A path with a space or quote in it stays one
/// word; anything else would dial somewhere the caller did not review.
fn sh_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\"'\"'"))
}

/// The emulator's argv, in order. The machine line follows
/// `experiments/147-podvm-exec.sh` (`-M pc,acpi=off`, the virt kernel with
/// `console=ttyS0 panic=-1`); the first serial port on a unix socket the
/// emulator binds is this arm's own choice, where 147 uses the pipe
/// backend. The `-cpu` half is the shared `Accel` decision; the `-accel`
/// string keeps 147's `tcg,thread=multi` shape for that machine line.
/// `quiet loglevel=0` is load-bearing, not cosmetic: the serial line is
/// the SSH transport, so any kernel printk past it interleaves framing
/// bytes with the session (measured: `Bad packet length` on an
/// uninitialized-urandom notice). Panics still print at EMERG, which is
/// the only guest text the arm ever needs to see.
fn qemu_argv(
    accel: Accel,
    mem_mib: u64,
    kernel: &Path,
    initramfs: &Path,
    serial_sock: &Path,
    emu_args: &[String],
) -> Vec<String> {
    let mut argv: Vec<String> = Vec::new();
    let mut push = |s: &str| argv.push(s.to_string());
    push("-M");
    push("pc,acpi=off");
    push("-m");
    push(&mem_mib.to_string());
    push("-display");
    push("none");
    push("-monitor");
    push("none");
    push("-no-reboot");
    push("-accel");
    push(match accel {
        Accel::Kvm => "kvm",
        Accel::Tcg => "tcg,thread=multi",
    });
    push("-cpu");
    push(accel.cpu());
    push("-kernel");
    push(&kernel.to_string_lossy());
    push("-initrd");
    push(&initramfs.to_string_lossy());
    push("-append");
    push("console=ttyS0 panic=-1 quiet loglevel=0");
    push("-serial");
    push(&format!(
        "unix:{},server=on,wait=off",
        serial_sock.display()
    ));
    for extra in emu_args {
        argv.push(extra.clone());
    }
    argv
}

/// How long the proxy holds the client's first bytes for the guest's
/// banner before failing loud. The guest serves it in about two
/// seconds; a minute is headroom under the arm's own ten-minute run
/// deadline, and the hold never outlives the run that owns it.
const HOLD_SECS: u64 = 60;

/// The ssh client's argv. The hostname is inert: the ProxyCommand carries
/// the bytes and known-hosts checking is off, so it names the guest for
/// the reader rather than addressing anything. The command words ride as
/// ssh's own argv, the proved shape: ssh joins them with spaces for the
/// guest shell, exactly as the relay drives already do, so no `--`
/// separator is inserted (whether OpenSSH would treat one as end-of-flags
/// or as remote-command text is unsettled, and this arm does not depend
/// on the answer).
fn ssh_argv(proxy: &Path, serial_sock: &Path, args: &Args) -> Vec<String> {
    // The hold keeps the client's version string off the wire until the
    // guest's banner proves its serial line open: bytes sent before that
    // open are discarded at the open, and the server then reads a
    // truncated first line and exits. Measured end to end, not reasoned.
    let proxy_cmd = format!(
        "{} unix {} --hold-for-banner {}",
        sh_quote(&proxy.to_string_lossy()),
        sh_quote(&serial_sock.to_string_lossy()),
        HOLD_SECS,
    );
    let mut argv = vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "UserKnownHostsFile=/dev/null".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-i".to_string(),
        args.user_key.to_string_lossy().into_owned(),
        "-l".to_string(),
        args.user.clone(),
        "-o".to_string(),
        format!("ProxyCommand={proxy_cmd}"),
        "guest".to_string(),
    ];
    argv.extend(args.command.iter().cloned());
    argv
}

/// Wait until the serial socket dials, the emulator dies, or the deadline
/// passes. An emulator that died at startup is a fast failure carrying
/// the emulator's own log tail, never a full-deadline wait: the wait
/// below cannot tell them apart, so this does.
///
/// The run-phase refusals share one exit code, so the sentence is the
/// only attribution the drive can grep. The constants keep the three
/// apart: merging any two would leave the bounded run owning nothing.
const EMULATOR_EXITED_MSG: &str = "the emulator exited at startup with status";
const SERIAL_DEADLINE_MSG: &str =
    "no SSH server answered on the guest serial line before the deadline";
const SESSION_DEADLINE_MSG: &str = "the guest did not answer in time";
fn wait_serial(
    sock: &Path,
    end: Instant,
    qemu: &mut std::process::Child,
    qemu_log: &Path,
) -> Result<(), i32> {
    loop {
        if std::os::unix::net::UnixStream::connect(sock).is_ok() {
            return Ok(());
        }
        match qemu.try_wait() {
            Ok(Some(status)) => {
                eprintln!("podbox machine ssh: {EMULATOR_EXITED_MSG} {status}");
                print_log_tail(qemu_log);
                return Err(EXIT_RUNTIME_ERROR);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("podbox machine ssh: cannot poll the emulator: {e}");
                return Err(EXIT_RUNTIME_ERROR);
            }
        }
        if Instant::now() >= end {
            eprintln!("podbox machine ssh: {SERIAL_DEADLINE_MSG}");
            return Err(EXIT_RUNTIME_ERROR);
        }
        std::thread::sleep(DIAL_POLL);
    }
}

fn print_log_tail(log: &Path) {
    if let Ok(text) = std::fs::read_to_string(log) {
        let lines: Vec<&str> = text.lines().collect();
        let tail = &lines[lines.len().saturating_sub(30)..];
        for line in tail {
            eprintln!("podbox machine ssh: [emulator] {line}");
        }
    }
}

/// Memory in bytes to whole MiB, rounded up. The shift-and-maybe-one
/// cannot overflow where an addition would: `u64::MAX` still fits.
fn mem_mib(bytes: u64) -> u64 {
    (bytes >> 20) + u64::from(bytes & ((1 << 20) - 1) != 0)
}

/// The per-run directory, removed on every path out. A signal (TERM, INT)
/// and SIGKILL alike skip `Drop`, so an killed run can leave the emulator
/// and this directory behind: the normal paths clean up, and the drive's
/// residue check owns the rest.
struct DirGuard {
    dir: Option<PathBuf>,
}

impl DirGuard {
    fn arm(dir: PathBuf) -> Self {
        DirGuard { dir: Some(dir) }
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        if let Some(dir) = self.dir.take() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

/// Stop the emulator. The guest is disposable, so this is SIGKILL and a
/// reap, with no graceful half: there is no guest state worth flushing
/// past what the completed session already wrote.
fn stop_qemu(qemu: &mut std::process::Child) {
    let _ = qemu.kill();
    let _ = qemu.wait();
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(name))
            .find(|p| crate::remote::is_executable(p))
    })
}

pub fn ssh(args: &[String]) -> i32 {
    let parsed = match parse(args) {
        Ok(None) => return 0,
        Ok(Some(parsed)) => parsed,
        Err(c) => return c,
    };
    let kernel = match resolve_input(&parsed.kernel, "--kernel") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let initramfs = match resolve_input(&parsed.initramfs, "--initramfs") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let user_key = match resolve_input(&parsed.user_key, "--user-key") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let args = Args {
        kernel,
        initramfs,
        user_key,
        ..parsed
    };
    // The file-size ceiling bounds the guest before it starts, per
    // TODO/podvm.md T-1305: a guest memory image is a single file.
    match podbox_probe::sys::prlimit(podbox_probe::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) if crate::tier::mem_over_ceiling(args.mem, cur) => {
            eprintln!(
                "podbox machine ssh: guest memory {} bytes over the RLIMIT_FSIZE ceiling {cur} bytes",
                args.mem
            );
            return EXIT_RUNTIME_ERROR;
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "podbox machine ssh: prlimit(RLIMIT_FSIZE) could not be read: {}",
                e.name()
            );
            return EXIT_RUNTIME_ERROR;
        }
    }
    let emulator = match on_path("qemu-system-x86_64") {
        Some(p) => p,
        None => {
            eprintln!("podbox machine ssh: no qemu-system-x86_64 on PATH");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let ssh_bin = match on_path("ssh") {
        Some(p) => p,
        None => {
            eprintln!("podbox machine ssh: no ssh on PATH");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let proxy = match crate::remote::resolve_helper("proxy") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let accel = if kvm_present(KVM_NODE) {
        Accel::Kvm
    } else {
        Accel::Tcg
    };
    // MiB, rounded up, without an addition that can overflow: a shift and
    // at most one more.
    let mem_mib = mem_mib(args.mem);
    let end = Instant::now() + args.timeout;

    let mut dir: PathBuf = std::env::temp_dir();
    dir.push(format!(
        "podbox-machine-ssh-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    // Atomic 0700: the mode rides the creation, so no window exists where
    // the directory is visible with looser bits.
    {
        use std::os::unix::fs::DirBuilderExt;
        if let Err(e) = std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            eprintln!("podbox machine ssh: cannot make {}: {e}", dir.display());
            return EXIT_RUNTIME_ERROR;
        }
    }
    let _guard = DirGuard::arm(dir.clone());
    let sock = dir.join("serial.sock");
    let qemu_log = dir.join("qemu.log");
    let log_file = match std::fs::File::create(&qemu_log) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "podbox machine ssh: cannot write {}: {e}",
                qemu_log.display()
            );
            return EXIT_RUNTIME_ERROR;
        }
    };

    let argv = qemu_argv(
        accel,
        mem_mib,
        &args.kernel,
        &args.initramfs,
        &sock,
        &args.emu_args,
    );
    let mut cmd = std::process::Command::new(&emulator);
    cmd.args(&argv).stdin(std::process::Stdio::null());
    match log_file.try_clone() {
        Ok(dup) => {
            cmd.stdout(dup);
            cmd.stderr(log_file);
        }
        Err(e) => {
            eprintln!(
                "podbox machine ssh: cannot duplicate {}: {e}",
                qemu_log.display()
            );
            return EXIT_RUNTIME_ERROR;
        }
    }
    let mut qemu = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "podbox machine ssh: cannot start {}: {e}",
                emulator.display()
            );
            return EXIT_RUNTIME_ERROR;
        }
    };
    if let Err(c) = wait_serial(&sock, end, &mut qemu, &qemu_log) {
        stop_qemu(&mut qemu);
        return c;
    }
    let remaining = end.saturating_duration_since(Instant::now());
    let ssh_argv = ssh_argv(&proxy, &sock, &args);
    let mut child = match std::process::Command::new(&ssh_bin)
        .args(&ssh_argv)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "podbox machine ssh: cannot start {}: {e}",
                ssh_bin.display()
            );
            stop_qemu(&mut qemu);
            return EXIT_RUNTIME_ERROR;
        }
    };
    // The ssh session ends when the guest command ends; the deadline ends
    // it when the guest does not. The poll is 100 ms, so the stop lands
    // within a tenth of the named timeout past it.
    let ssh_end = Instant::now() + remaining;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break crate::remote::status_to_code(status),
            Ok(None) => {}
            Err(e) => {
                eprintln!("podbox machine ssh: cannot poll ssh: {e}");
                let _ = child.kill();
                let _ = child.wait();
                break EXIT_RUNTIME_ERROR;
            }
        }
        if Instant::now() >= ssh_end {
            eprintln!(
                "podbox machine ssh: stopped after {} s: {SESSION_DEADLINE_MSG}",
                args.timeout.as_secs()
            );
            let _ = child.kill();
            let _ = child.wait();
            break EXIT_RUNTIME_ERROR;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    stop_qemu(&mut qemu);
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(xs: &[&str]) -> Result<Option<Args>, i32> {
        parse(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn help_prints_and_exits_zero() {
        assert!(matches!(p(&["-h"]), Ok(None)));
        assert!(matches!(p(&["--help"]), Ok(None)));
        assert!(
            matches!(p(&["--kernel", "k", "-h"]), Ok(None)),
            "help wins anywhere, like the windows parser"
        );
    }

    #[test]
    fn missing_required_inputs_are_flag_errors() {
        use podbox_image::error::EXIT_FLAG_ERROR;
        assert_eq!(p(&[]).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(p(&["--kernel", "k"]).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(
            p(&["--kernel", "k", "--initramfs", "i"]).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(p(&["--kernel"]).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(p(&["--user-key"]).unwrap_err(), EXIT_FLAG_ERROR);
    }

    #[test]
    fn bad_sizes_and_timeouts_are_flag_errors() {
        use podbox_image::error::EXIT_FLAG_ERROR;
        assert_eq!(
            p(&[
                "--kernel",
                "k",
                "--initramfs",
                "i",
                "--user-key",
                "uk",
                "--podbox-mem",
                "huge",
            ])
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            p(&[
                "--kernel",
                "k",
                "--initramfs",
                "i",
                "--user-key",
                "uk",
                "--podbox-mem",
                "0",
            ])
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            p(&[
                "--kernel",
                "k",
                "--initramfs",
                "i",
                "--user-key",
                "uk",
                "--podbox-timeout",
                "0",
            ])
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            p(&[
                "--kernel",
                "k",
                "--initramfs",
                "i",
                "--user-key",
                "uk",
                "--podbox-timeout",
                "soon",
            ])
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            p(&[
                "--kernel",
                "k",
                "--initramfs",
                "i",
                "--user-key",
                "uk",
                "--bogus",
            ])
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn minimal_flags_carry_the_documented_defaults() {
        let a = p(&["--kernel", "k", "--initramfs", "i", "--user-key", "uk"])
            .unwrap()
            .unwrap();
        assert_eq!(a.user, "root");
        assert_eq!(a.mem, 256 << 20);
        assert_eq!(a.timeout, Duration::from_secs(600));
        assert!(a.command.is_empty());
        assert!(a.emu_args.is_empty());
    }

    #[test]
    fn the_parsed_arm_carries_everything() {
        let a = p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--user",
            "guest",
            "--podbox-mem",
            "1G",
            "--podbox-timeout",
            "90",
            "--podbox-qemu-arg",
            "-smp",
            "--podbox-qemu-arg",
            "2",
            "--",
            "echo",
            "hi there",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.kernel, PathBuf::from("k"));
        assert_eq!(a.initramfs, PathBuf::from("i"));
        assert_eq!(a.user_key, PathBuf::from("uk"));
        assert_eq!(a.user, "guest");
        assert_eq!(a.mem, 1 << 30);
        assert_eq!(a.timeout, Duration::from_secs(90));
        assert_eq!(a.emu_args, vec!["-smp".to_string(), "2".to_string()]);
        assert_eq!(a.command, vec!["echo".to_string(), "hi there".to_string()]);
    }

    #[test]
    fn bare_words_without_a_separator_are_the_command() {
        let a = p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "echo",
            "hi",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.command, vec!["echo".to_string(), "hi".to_string()]);
    }

    #[test]
    fn bytes_round_up_to_whole_mib_without_overflow() {
        assert_eq!(mem_mib(0), 0);
        assert_eq!(mem_mib(1), 1);
        assert_eq!(mem_mib(1 << 20), 1);
        assert_eq!(mem_mib((1 << 20) + 1), 2);
        assert_eq!(mem_mib(256 << 20), 256);
        assert_eq!(mem_mib(u64::MAX), (u64::MAX >> 20) + 1);
    }

    #[test]
    fn the_dir_guard_removes_the_directory_on_drop() {
        let dir =
            std::env::temp_dir().join(format!("podbox-machine-ssh-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let _guard = DirGuard::arm(dir.clone());
        }
        assert!(!dir.exists());
    }

    #[test]
    fn a_named_input_that_is_not_a_file_is_refused_never_replaced() {
        let dir = std::env::temp_dir();
        let missing = dir.join("podbox-machine-ssh-no-such-file-xyz");
        assert_eq!(resolve_input(&missing, "--kernel"), Err(EXIT_RUNTIME_ERROR));
    }

    #[test]
    fn the_accelerator_follows_the_node_openability() {
        assert!(!kvm_present("/podbox-machine-ssh-no-such-node-xyz"));
        // Any read-write openable path answers true: the check probes
        // openability, not identity, which is what the argv needs.
        let file = std::env::temp_dir().join("podbox-machine-ssh-kvm-probe-xyz");
        std::fs::write(&file, b"no").unwrap();
        assert!(kvm_present(file.to_str().unwrap()));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn the_qemu_argv_carries_the_proved_machine_line() {
        let argv = qemu_argv(
            Accel::Tcg,
            256,
            Path::new("/work/vmlinuz"),
            Path::new("/work/init.cpio"),
            Path::new("/run/serial.sock"),
            &[],
        );
        let line = argv.join(" ");
        for token in [
            "-M pc,acpi=off",
            "-m 256",
            "-display none",
            "-monitor none",
            "-no-reboot",
            "-accel tcg,thread=multi",
            "-cpu max",
            "-kernel /work/vmlinuz",
            "-initrd /work/init.cpio",
            "-append console=ttyS0 panic=-1 quiet loglevel=0",
            "-serial unix:/run/serial.sock,server=on,wait=off",
        ] {
            assert!(line.contains(token), "missing {token} in {line}");
        }
    }

    #[test]
    fn the_kvm_pair_is_kvm_with_host_cpu() {
        let argv = qemu_argv(
            Accel::Kvm,
            512,
            Path::new("k"),
            Path::new("i"),
            Path::new("s"),
            &["-smp".to_string(), "2".to_string()],
        );
        let line = argv.join(" ");
        assert!(line.contains("-accel kvm"), "{line}");
        assert!(line.contains("-cpu host"), "{line}");
        assert!(line.ends_with("-smp 2"), "caller args append last: {line}");
    }

    #[test]
    fn the_ssh_argv_is_batchmode_with_the_proxy_command() {
        let args = Args {
            kernel: PathBuf::from("k"),
            initramfs: PathBuf::from("i"),
            user_key: PathBuf::from("/keys/u ser"),
            user: "root".to_string(),
            mem: DEFAULT_MEM,
            timeout: Duration::from_secs(600),
            emu_args: Vec::new(),
            command: vec!["echo".to_string(), "hi".to_string()],
        };
        let argv = ssh_argv(
            Path::new("/opt/my bins/proxy"),
            Path::new("/run/serial.sock"),
            &args,
        );
        let line = argv.join(" ");
        for token in [
            "BatchMode=yes",
            "StrictHostKeyChecking=no",
            "UserKnownHostsFile=/dev/null",
            "ConnectTimeout=10",
            "-i /keys/u ser",
            "-l root",
        ] {
            assert!(line.contains(token), "missing {token} in {line}");
        }
        assert!(
            line.contains(
                "ProxyCommand='/opt/my bins/proxy' unix '/run/serial.sock' --hold-for-banner 60"
            ),
            "proxy command quoting: {line}"
        );
        assert!(line.contains(" guest "), "the inert hostname: {line}");
        assert!(
            line.ends_with("guest echo hi"),
            "command rides last with no separator: {line}"
        );
    }

    #[test]
    fn an_empty_command_leaves_the_argv_at_the_hostname() {
        let args = Args {
            kernel: PathBuf::from("k"),
            initramfs: PathBuf::from("i"),
            user_key: PathBuf::from("uk"),
            user: "root".to_string(),
            mem: DEFAULT_MEM,
            timeout: Duration::from_secs(600),
            emu_args: Vec::new(),
            command: Vec::new(),
        };
        let argv = ssh_argv(Path::new("proxy"), Path::new("s"), &args);
        assert_eq!(argv.last().map(String::as_str), Some("guest"));
    }

    #[test]
    fn shell_quoting_survives_spaces_and_quotes() {
        assert_eq!(sh_quote("plain"), "'plain'");
        assert_eq!(sh_quote(""), "''");
        assert_eq!(sh_quote("/opt/my bins/proxy"), "'/opt/my bins/proxy'");
        assert_eq!(sh_quote("o'clock"), "'o'\"'\"'clock'");
        assert_eq!(sh_quote("$(evil)"), "'$(evil)'");
        assert_eq!(sh_quote("back\\slash"), "'back\\slash'");
        assert_eq!(sh_quote("-n"), "'-n'");
    }

    #[test]
    fn the_serial_wait_deadline_is_a_runtime_error() {
        // Deleting the deadline below hangs this test rather than failing
        // it: the arm it guards is the loop's only exit past the emulator
        // check, so this test is the deadline's proof by construction.
        // The message it prints is pinned by
        // run_phase_refusals_say_different_things below.
        let dir =
            std::env::temp_dir().join(format!("podbox-machine-ssh-wait-{}", std::process::id()));
        let sock = dir.join("serial.sock");
        let log = dir.join("qemu.log");
        // No listener, no emulator behind it: use a live-but-idle child
        // as the polled process so the wait exercises the deadline arm.
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        let code = wait_serial(&sock, end, &mut child, &log).unwrap_err();
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(code, EXIT_RUNTIME_ERROR);
    }

    #[test]
    fn run_phase_refusals_say_different_things() {
        // All three run-phase failures share EXIT_RUNTIME_ERROR, so the
        // sentence is the only attribution the bounded drive can grep.
        // Merging any two sentences would leave that drive owning nothing.
        assert!(SERIAL_DEADLINE_MSG.contains("before the deadline"));
        assert!(SESSION_DEADLINE_MSG.contains("did not answer in time"));
        assert!(EMULATOR_EXITED_MSG.contains("exited at startup"));
        assert_ne!(SERIAL_DEADLINE_MSG, SESSION_DEADLINE_MSG);
        assert_ne!(SERIAL_DEADLINE_MSG, EMULATOR_EXITED_MSG);
        assert_ne!(SESSION_DEADLINE_MSG, EMULATOR_EXITED_MSG);
    }

    #[test]
    fn the_serial_wait_fails_fast_where_the_emulator_died() {
        let dir =
            std::env::temp_dir().join(format!("podbox-machine-ssh-dead-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("serial.sock");
        let log = dir.join("qemu.log");
        std::fs::write(&log, "qemu said no\n").unwrap();
        // An already-exited child with no listener: only the fast-failure
        // arm can answer, and it answers at once rather than at the end.
        let mut child = std::process::Command::new("/bin/true").spawn().unwrap();
        let _ = child.wait();
        let start = Instant::now();
        let code =
            wait_serial(&sock, start + Duration::from_secs(60), &mut child, &log).unwrap_err();
        assert!(start.elapsed() < Duration::from_secs(10));
        assert_eq!(code, EXIT_RUNTIME_ERROR);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_serial_wait_accepts_a_bound_socket() {
        let dir =
            std::env::temp_dir().join(format!("podbox-machine-ssh-accept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("serial.sock");
        let log = dir.join("qemu.log");
        let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        assert!(wait_serial(&sock, end, &mut child, &log).is_ok());
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
