use super::*;

/// A named input that must be a file. A named path that is not a file is a
/// refusal, never a substitution: booting a different guest than the named
/// one and reporting its output as that guest's is the failure the Windows
/// driver documents, and this arm does not repeat it.
pub(super) fn resolve_input(path: &Path, flag: &str) -> Result<PathBuf, i32> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    eprintln!("podvm machine ssh: {flag} {} is not a file", path.display());
    Err(EXIT_RUNTIME_ERROR)
}

/// True where the KVM node opens read-write. The sibling decision is
/// `podvm_windows::plan::accel_for` over the machine profile; this arm
/// asks only the one leg its argv needs (the emulator's presence is
/// checked at spawn), so a refusal elsewhere does not change which
/// accelerator a runnable guest gets. The check is openability, not
/// existence: a present node the caller cannot open is TCG, not KVM.
/// (Untestable as root, where permission bits do not bind: the test pins
/// the absent and openable arms and this comment owns the third.)
pub(super) fn kvm_present(node: &str) -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(node)
        .is_ok()
}

/// Quote one word for `/bin/sh -c`, which is what ssh runs the
/// ProxyCommand through. A path with a space or quote in it stays one
/// word; anything else would dial somewhere the caller did not review.
pub(super) fn sh_quote(word: &str) -> String {
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
pub(super) fn qemu_argv(
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
pub(super) fn ssh_argv(proxy: &Path, serial_sock: &Path, args: &Args) -> Vec<String> {
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
pub(super) const EMULATOR_EXITED_MSG: &str = "the emulator exited at startup with status";
pub(super) const SERIAL_DEADLINE_MSG: &str =
    "no SSH server answered on the guest serial line before the deadline";
pub(super) const SESSION_DEADLINE_MSG: &str = "the guest did not answer in time";
pub(super) fn wait_serial(
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
                eprintln!("podvm machine ssh: {EMULATOR_EXITED_MSG} {status}");
                print_log_tail(qemu_log);
                return Err(EXIT_RUNTIME_ERROR);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("podvm machine ssh: cannot poll the emulator: {e}");
                return Err(EXIT_RUNTIME_ERROR);
            }
        }
        if Instant::now() >= end {
            eprintln!("podvm machine ssh: {SERIAL_DEADLINE_MSG}");
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
            eprintln!("podvm machine ssh: [emulator] {line}");
        }
    }
}

/// Memory in bytes to whole MiB, rounded up. The shift-and-maybe-one
/// cannot overflow where an addition would: `u64::MAX` still fits.
pub(super) fn mem_mib(bytes: u64) -> u64 {
    (bytes >> 20) + u64::from(bytes & ((1 << 20) - 1) != 0)
}

/// The per-run directory, removed on every path out. A signal (TERM, INT)
/// and SIGKILL alike skip `Drop`, so an killed run can leave the emulator
/// and this directory behind: the normal paths clean up, and the drive's
/// residue check owns the rest.
pub(super) struct DirGuard {
    dir: Option<PathBuf>,
}

impl DirGuard {
    pub(super) fn arm(dir: PathBuf) -> Self {
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
    // TODO/machine.md T-1305: a guest memory image is a single file.
    match podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE) {
        Ok((cur, _)) if crate::tier::mem_over_ceiling(args.mem, cur) => {
            eprintln!(
                "podvm machine ssh: guest memory {} bytes over the RLIMIT_FSIZE ceiling {cur} bytes",
                args.mem
            );
            return EXIT_RUNTIME_ERROR;
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "podvm machine ssh: prlimit(RLIMIT_FSIZE) could not be read: {}",
                e.name()
            );
            return EXIT_RUNTIME_ERROR;
        }
    }
    let emulator = match on_path("qemu-system-x86_64") {
        Some(p) => p,
        None => {
            eprintln!("podvm machine ssh: no qemu-system-x86_64 on PATH");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let ssh_bin = match on_path("ssh") {
        Some(p) => p,
        None => {
            eprintln!("podvm machine ssh: no ssh on PATH");
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
    // TODO/machine.md T-1682: the run says which accelerator it took, so a
    // KVM proof cannot read a TCG run as KVM.
    eprintln!(
        "podvm machine ssh: accelerator {}",
        match accel {
            Accel::Kvm => "kvm",
            Accel::Tcg => "tcg (an emulator, not hardware isolation)",
        }
    );
    // MiB, rounded up, without an addition that can overflow: a shift and
    // at most one more.
    let mem_mib = mem_mib(args.mem);
    let end = Instant::now() + args.timeout;

    let mut dir: PathBuf = std::env::temp_dir();
    dir.push(format!(
        "podvm-machine-ssh-{}-{}",
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
            eprintln!("podvm machine ssh: cannot make {}: {e}", dir.display());
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
                "podvm machine ssh: cannot write {}: {e}",
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
                "podvm machine ssh: cannot duplicate {}: {e}",
                qemu_log.display()
            );
            return EXIT_RUNTIME_ERROR;
        }
    }
    let mut qemu = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "podvm machine ssh: cannot start {}: {e}",
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
            eprintln!("podvm machine ssh: cannot start {}: {e}", ssh_bin.display());
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
                eprintln!("podvm machine ssh: cannot poll ssh: {e}");
                let _ = child.kill();
                let _ = child.wait();
                break EXIT_RUNTIME_ERROR;
            }
        }
        if Instant::now() >= ssh_end {
            eprintln!(
                "podvm machine ssh: stopped after {} s: {SESSION_DEADLINE_MSG}",
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
