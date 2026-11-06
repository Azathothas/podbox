//! `podvm machine ssh`: a shell in a Linux guest podvm itself runs.
//!
//! Podman parity (`podman machine ssh` opens a shell in podman's own VM),
//! kept on its own axis per `docs/decisions/remote-verb.md`: this is a
//! guest podvm starts, against `remote ssh`, which is a machine somebody
//! else started.
//!
//! The server placement is what makes this arm rather than a second
//! spelling of remote: the SSH server executes IN the guest. podvm boots
//! a Linux guest from the named kernel and initramfs with the first serial
//! port on a per-run unix socket, speaks real SSH over that socket with a
//! real `ssh` client through the lane's `proxy` binary, and reports the
//! guest command's exit code as its own. The guest provides the server
//! (dropbear or sshd on its serial line); the handshake itself is the
//! probe, so no separate probe can go stale beside it.
//!
//! One session per boot: an inetd-mode server answers one connection and
//! exits, so this arm runs one command (or one login shell) per boot.
//! That is the guest's shape, not `TODO/machine.md` T-1304's: T-1304
//! multiplexes many commands over one serial session with per-command
//! nonces, while here the boot is the session.
//!
//! The `ssh` and `qemu-system-x86_64` binaries are trusted-PATH lookups,
//! resolved the way the SSH crate resolves its servers: named, required,
//! and refused where absent. The `proxy` helper resolves beside this
//! binary first, then on PATH, like the rest of the remote group.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use podvm_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podvm_windows::plan::Accel;

pub const USAGE: &str = "\
usage: podvm machine ssh --kernel <file> --initramfs <file> --user-key <file> [--user <name>] [-- command ...]
  --kernel <file>          the guest kernel, for example vmlinuz-virt
  --initramfs <file>       the guest initramfs, carrying the SSH server on
                           its serial console and the account to log in as
  --user-key <file>        the private key the guest authorises for --user
  --user <name>            the guest account (default root)
  --podvm-mem <size>      guest memory, for example 256M (default 256M)
  --podvm-timeout <secs>  stop the guest past this long (default 600, and a
                           stop is a refusal, never an empty success)
  --podvm-qemu-arg <arg>  one extra emulator argument, repeatable, appended
                           last so a caller can override a default
  -h, --help               print this usage and exit 0

  Boots a disposable Linux guest, runs one SSH command in it, and exits
  with the guest command's exit code. With no command, ssh opens the
  guest's login shell on this terminal.
";

/// Guest memory where `--podvm-mem` is absent.
const DEFAULT_MEM: u64 = 256 << 20;

/// The whole run where `--podvm-timeout` is absent.
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
    "--podvm-mem",
    "--podvm-timeout",
    "--podvm-qemu-arg",
];

/// One parsed `podvm machine ssh` invocation.
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
        eprintln!("podvm machine ssh: {flag} needs a value\n{USAGE}");
        EXIT_FLAG_ERROR
    })
}

/// Parse the arm. `Ok(None)` is `-h/--help`: printed, and the exit is 0.
pub(super) fn parse(args: &[String]) -> Result<Option<Args>, i32> {
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
            "--podvm-mem" => {
                let v = value(&mut it, "--podvm-mem")?;
                mem = crate::tier::parse_mem(v).ok_or_else(|| {
                    eprintln!("podvm machine ssh: --podvm-mem {v} is not a size\n{USAGE}");
                    EXIT_FLAG_ERROR
                })?;
            }
            "--podvm-timeout" => {
                let v = value(&mut it, "--podvm-timeout")?;
                let n: u64 = v.parse().map_err(|_| {
                    eprintln!(
                        "podvm machine ssh: --podvm-timeout {v} is not a count of seconds\n{USAGE}"
                    );
                    EXIT_FLAG_ERROR
                })?;
                if n == 0 {
                    eprintln!(
                        "podvm machine ssh: --podvm-timeout 0 is refused: name a positive count of seconds\n{USAGE}"
                    );
                    return Err(EXIT_FLAG_ERROR);
                }
                timeout = Duration::from_secs(n);
            }
            "--podvm-qemu-arg" => emu_args.push(value(&mut it, "--podvm-qemu-arg")?.clone()),
            other if other.starts_with('-') => {
                // FLAGS is the list this arm takes; the arms above are its
                // shape. Anything outside the list never reaches an arm, and
                // anything inside it without an arm above refuses rather
                // than falling through (the parity cross-test forbids that
                // drift, and the positive tests pin every arm).
                if !FLAGS.contains(&other) {
                    eprintln!("podvm machine ssh: {other} is not a flag this arm has\n{USAGE}");
                } else {
                    eprintln!("podvm machine ssh: {other} is listed but has no arm\n{USAGE}");
                }
                return Err(EXIT_FLAG_ERROR);
            }
            other => command.push(other.to_string()),
        }
    }
    let kernel = kernel.ok_or_else(|| {
        eprintln!("podvm machine ssh: --kernel names the guest kernel\n{USAGE}");
        EXIT_FLAG_ERROR
    })?;
    let initramfs = initramfs.ok_or_else(|| {
        eprintln!("podvm machine ssh: --initramfs names the guest initramfs\n{USAGE}");
        EXIT_FLAG_ERROR
    })?;
    let user_key = user_key.ok_or_else(|| {
        eprintln!("podvm machine ssh: --user-key names the key the guest authorises\n{USAGE}");
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

mod qemu;
pub use qemu::*;

#[cfg(test)]
mod tests;
