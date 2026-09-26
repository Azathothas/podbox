//! The DOS flavor of `podbox windows run`: FreeDOS from the base cache,
//! one typed line, the guest's own exit code. `TODO/milestones.md` T-1112.
//!
//! ⛔ **This is the same driver as the other flavor, not a second one.**
//! The mailbox, the token, the QMP monitor, the per-run directory and the
//! overlay discipline all live in `podbox-windows` and are shared; what is
//! here is only the DOS route to them: the base cache refuses where the
//! other flavor's cache would be booted by mistake, the batch line is built
//! from the argv, and the run is the crate's `dos::run`. A second mailbox
//! writer or a second monitor here is the copy that diverges.

use std::time::Duration;

use podbox_image::error::EXIT_RUNTIME_ERROR;

use super::plan;

/// How long a DOS run may take before it is stopped. The boot walk alone
/// is 42 s on the calibrated host; `pause` never answers and reaches this
/// with no status rather than an empty success.
pub(crate) const DOS_TIMEOUT: u64 = 180;

/// The batch line for a joined command line, which is what `podbox windows
/// run` hands over. A leading `cmd /c ` drops, because `cmd.exe` is what
/// the guest already is; an empty remainder is `ver`, the acceptance
/// command. The line goes to the guest verbatim otherwise: quoting inside
/// it is the guest shell's own.
pub(crate) fn line_from_command(command: &str) -> String {
    let line = command.trim();
    let rest = match line.split_once(char::is_whitespace) {
        Some(("cmd", rest)) => match rest.trim_start().split_once(char::is_whitespace) {
            Some(("/c", r)) => r,
            _ if rest.trim_start() == "/c" => "",
            _ => line,
        },
        _ => line,
    };
    if rest.trim().is_empty() {
        "ver".to_string()
    } else {
        rest.to_string()
    }
}

/// `podbox windows run --guest dos`, and the DOS half of `podbox run`.
///
/// `command` is one command line: the run verb hands its argv joined, and
/// the windows verb hands its own command string. Either way the batch
/// line drops a leading `cmd /c ` and defaults an empty line to `ver`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dos_run(
    verb: &str,
    mem: Option<u64>,
    cpus: Option<u32>,
    emu_args: &[String],
    timeout: Option<u64>,
    command: &str,
) -> i32 {
    let base = podbox_windows::dos::base_cache();
    if !base.is_file() {
        eprintln!(
            "podbox {verb}: no DOS base image at {} and none was named. The DOS flavor boots \
             FreeDOS, not a disk you hand it: `experiments/363-windows-tcg-dos.sh` fetches \
             FreeDOS 1.4 LiteUSB (pinned sha256, fetch ceiling) and writes the base. No base \
             is fetched or committed here (TODO/milestones.md T-1112)",
            base.display()
        );
        return EXIT_RUNTIME_ERROR;
    }
    let accel = match super::plan::accelerator() {
        Ok(x) => x,
        Err(c) => return c,
    };
    let emulator = plan::emulator();
    let qemu_img = plan::qemu_img();
    for bin in [&emulator, &qemu_img] {
        let name = bin.to_string_lossy().to_string();
        if !plan::found(&name) {
            eprintln!("podbox {verb}: {name} is not on this machine");
            return EXIT_RUNTIME_ERROR;
        }
    }
    let mem_mib = mem
        .map(|m| m / (1 << 20))
        .unwrap_or(podbox_windows::dos::DEFAULT_MEM_MIB);
    let dir = match podbox_windows::scratch_dir("dos-run") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("podbox {verb}: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    // ⚠ Absolute, because the emulator resolves the backing file relative
    // to the overlay on first open, and the overlay lives in scratch.
    let base = std::fs::canonicalize(&base).unwrap_or(base);
    let plan = podbox_windows::Plan {
        emulator,
        accel,
        share: plan::share(),
        firmware_code: Default::default(),
        firmware_vars: Default::default(),
        root: dir.join("root.qcow2"),
        mailbox: dir.join("mailbox.img"),
        serial: dir.join("serial.log"),
        monitor: dir.join("qmp.sock"),
        memory_mib: mem_mib,
        cpus: cpus.unwrap_or(podbox_windows::dos::CPUS),
        emu_args: emu_args.to_vec(),
    };
    let batch = line_from_command(command);
    let token = podbox_windows::agent::nonce();
    let timeout = match timeout.unwrap_or(DOS_TIMEOUT) {
        0 => Duration::from_secs(u32::MAX as u64),
        n => Duration::from_secs(n),
    };
    eprintln!(
        "podbox {verb}: machine tier {} (the emulator process is the boundary, not hardware \
         isolation; -nic none, no networking). Guest FreeDOS 1.4 (FreeCom 0.86) for \
         windows/amd64: DOS batch, portable, not Windows 11. Base {} mem {} MiB \
         (TODO/milestones.md T-1112)",
        accel.word(),
        base.display(),
        mem_mib
    );
    if let Err(e) = podbox_windows::dos::stage(&qemu_img, &base, &plan, &batch, &token) {
        eprintln!("podbox {verb}: {e}");
        podbox_windows::cleanup(&plan);
        return EXIT_RUNTIME_ERROR;
    }
    match podbox_windows::dos::run(&plan, &batch, &token, timeout) {
        Ok(out) => {
            use std::io::Write;
            let _ = std::io::stdout().write_all(&out.stdout);
            let _ = std::io::stdout().flush();
            // ⚠ DOS stderr stays on the guest's VGA screen: only stdout
            // crosses in WQOUT.TXT, which the banner states by saying
            // nothing about stderr.
            podbox_windows::cleanup(&plan);
            kept(&plan);
            match podbox_windows::cap_status(out.code) {
                Some(c) => c,
                None => {
                    eprintln!(
                        "podbox {verb}: the guest exited {}, which is outside docker's \
                         0-124 range, so podbox reports {EXIT_RUNTIME_ERROR}",
                        out.code
                    );
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        Err(podbox_windows::Error::Timeout(d)) => {
            eprintln!(
                "podbox {verb}: DEADLINE: no completion line with the run token within {} s. \
                 The guest never wrote WQCODE.TXT, so there is no status to return \
                 (TODO/milestones.md T-1112)",
                d.as_secs()
            );
            podbox_windows::cleanup(&plan);
            kept(&plan);
            EXIT_RUNTIME_ERROR
        }
        Err(e) => {
            eprintln!("podbox {verb}: {e}");
            podbox_windows::cleanup(&plan);
            kept(&plan);
            EXIT_RUNTIME_ERROR
        }
    }
}

fn kept(plan: &podbox_windows::Plan) {
    if std::env::var_os("PODBOX_WINDOWS_KEEP").is_some() {
        if let Some(d) = podbox_windows::scratch_of(plan) {
            eprintln!("podbox windows: kept {} (PODBOX_WINDOWS_KEEP)", d.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cmd_prefix_drops_and_an_empty_line_is_ver() {
        assert_eq!(line_from_command("cmd /c ver"), "ver");
        assert_eq!(line_from_command("cmd /c dir /zzz"), "dir /zzz");
        assert_eq!(line_from_command("  cmd /c ver  "), "ver");
        assert_eq!(line_from_command("ver"), "ver");
        assert_eq!(line_from_command(""), "ver");
        assert_eq!(line_from_command("cmd /c"), "ver");
        assert_eq!(line_from_command("cmd"), "cmd");
        assert_eq!(line_from_command("cmd.exe /c ver"), "cmd.exe /c ver");
    }
}
