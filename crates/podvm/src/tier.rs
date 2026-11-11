//! The tier flag. `TODO/machine.md` T-1302.
//!
//! `--podvm-tier=machine` selects the machine tier; with no flag the ladder
//! selects. The name the binary was invoked under does not change the
//! default. `TODO/cli.md` T-1712 removed the value `chroot`: it had no
//! effect since call C2, so a caller that asked for chroot got whatever the
//! ladder entered. `system info --format '{{.EnteredRung}}'` names the rung
//! that a run enters (T-1713).
//!
//! ⛔ **The tier is not cosmetic**: a limit the machine tier enforces and the
//! chroot tier cannot is silent degradation wearing a flag, so a caller who
//! cannot tell which tier ran cannot tell whether a limit was honoured.

use podvm_image::error::EXIT_RUNTIME_ERROR;

/// What `--podvm-tier` accepts. Anything else is a flag error at parse
/// time, so this type never carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    Machine,
}

/// The validated value of one `--podvm-tier` spelling, or `None` where the
/// spelling names no tier that the flag selects.
pub fn want(s: &str) -> Option<Want> {
    match s {
        "machine" => Some(Want::Machine),
        _ => None,
    }
}

/// The flag error of a value that [`want`] refuses, for `verb`.
///
/// ⛔ `TODO/cli.md` T-1712. `chroot` is refused by name with the reason: a
/// value that selected nothing gave a caller another mechanism with exit 0.
pub fn refusal(verb: &str, value: &str) -> String {
    format!(
        "podvm {verb}: --podvm-tier takes machine, not {value:?}: with no flag the \
         ladder selects the rung, and `podvm system info --format \
         '{{{{.EnteredRung}}}}'` names the rung that a run enters (TODO/cli.md T-1712)"
    )
}

/// The validated value of one `--podvm-mem` spelling, in bytes, or `None`
/// where the spelling names no size. `TODO/machine.md` T-1305.
///
/// A plain integer is bytes; a trailing `K`, `M`, `G` or `T`
/// (case-insensitive, with an optional `iB`/`B`) scales it by a binary
/// power. Zero is refused: a zero-byte guest is not a configuration.
/// Overflow is refused rather than wrapped.
pub fn parse_mem(s: &str) -> Option<u64> {
    let cut = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (digits, suffix) = s.split_at(cut);
    let n: u64 = digits.parse().ok()?;
    if n == 0 {
        return None;
    }
    let upper = suffix.to_ascii_uppercase();
    let suf = upper.strip_suffix('B').unwrap_or(&upper);
    let suf = suf.strip_suffix('I').unwrap_or(suf);
    let mult: u64 = match suf {
        "" => 1,
        "K" => 1u64 << 10,
        "M" => 1u64 << 20,
        "G" => 1u64 << 30,
        "T" => 1u64 << 40,
        _ => return None,
    };
    n.checked_mul(mult)
}

/// True where a configured guest memory in bytes crosses the file-size
/// ceiling `cur` read from `prlimit(RLIMIT_FSIZE)`. `u64::MAX` is infinity:
/// no finite guest crosses it, so an unbounded machine never refuses here.
pub fn mem_over_ceiling(mem: u64, cur: u64) -> bool {
    cur != u64::MAX && mem > cur
}

/// The tier an invocation runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// The default with no flag: the ladder selects.
    Ladder,
    /// `--podvm-tier=machine`: the legs are assessed and the tier is
    /// refused naming every missing one.
    Machine,
}

/// Resolve the flag to the tier that runs.
///
/// ⛔ The name podvm was invoked under plays no part: every name the
/// binary answers to takes the same default, the ladder, and only the flag
/// selects another tier.
pub fn resolve(flag: Option<Want>) -> Tier {
    match flag {
        None => Tier::Ladder,
        Some(Want::Machine) => Tier::Machine,
    }
}

/// Apply a machine-tier selection: assess the legs and refuse, naming them.
///
/// `mem` is the validated `--podvm-mem` value in bytes, or `None` where no
/// memory was configured. Where one was, it is judged BEFORE the machine is:
/// a guest over the `RLIMIT_FSIZE` ceiling is refused naming both numbers on
/// every machine, which is what `experiments/148-podvm-fleet.sh` drives
/// (`TODO/machine.md` T-1305). The legs keep their refusal where no memory was
/// configured, and where the ceiling cannot be read the refusal names that
/// instead of running a guest no bound was checked against.
///
/// Where a profile holds, `run` still boots no guest from an OCI image: that
/// arrives with `TODO/machine.md` T-1656, and this says so rather
/// than running something else. Either way the command cannot run, which is
/// docker's 125. Under TCG the message states the boundary honestly: the
/// emulator process, never hardware isolation.
pub fn enter_machine(verb: &str, mem: Option<u64>) -> i32 {
    // ⭐ The name is said here because this path never reaches the banner,
    // which is where the alias note otherwise prints. A refusal that names
    // `podvm` to a caller who typed `docker` is the silent name-taking
    // `TOOL.md` section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        eprint!("{note}");
    }
    if let Some(m) = mem {
        match podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE) {
            Ok((cur, _)) if mem_over_ceiling(m, cur) => {
                eprintln!(
                    "podvm {verb}: machine tier refused: guest memory {m} bytes \
                     over the RLIMIT_FSIZE ceiling {cur} bytes \
                     (prlimit(RLIMIT_FSIZE) cur={cur})"
                );
                return EXIT_RUNTIME_ERROR;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "podvm {verb}: machine tier refused: \
                     prlimit(RLIMIT_FSIZE) could not be read: {}",
                    e.name()
                );
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    let findings = podvm_probe::run();
    let assessed = podvm_probe::machine::assess(&findings);
    match assessed.profile {
        Some(podvm_probe::machine::Profile::Full) => {
            eprintln!(
                "podvm {verb}: the machine tier holds on this machine; running \
                 an OCI image as a guest arrives with T-1656"
            );
            EXIT_RUNTIME_ERROR
        }
        Some(podvm_probe::machine::Profile::Tcg) => {
            // ⭐ TODO/machine.md T-1301. The tier runs TCG here: kvm, tun or
            // both are missing, and the message states both halves rather
            // than letting the banner's `tcg` read as acceleration.
            let net = if assessed.tun_ok() {
                "/dev/net/tun holds"
            } else {
                "/dev/net/tun is missing, so guests use user-mode networking"
            };
            eprintln!(
                "podvm {verb}: the machine tier runs TCG on this machine: \
                 the emulator process is the boundary, not hardware \
                 isolation. {net}. Running an OCI image as a guest \
                 arrives with T-1656"
            );
            EXIT_RUNTIME_ERROR
        }
        None => match assessed.refusal() {
            Some(r) => {
                eprintln!("podvm {verb}: {r}");
                EXIT_RUNTIME_ERROR
            }
            None => {
                eprintln!("podvm {verb}: machine tier refused");
                EXIT_RUNTIME_ERROR
            }
        },
    }
}

/// The path of one named binary, searched on `PATH`, or `None`.
///
/// ⚠ The machine legs already prove an emulator runs and lists `tcg`; this
/// is the narrow lookup a driver needs for the two helpers it spawns before
/// it spawns one. `TODO/machine.md` T-1301 owns the legs, this owns the name.
pub fn which_qemu(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(bin);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ TODO/cli.md T-1712. The flag spells one tier. `chroot` selected
    /// nothing, so it is a flag error now, and the refusal names the value,
    /// the one value the flag takes, and where the entered rung is read.
    #[test]
    fn the_flag_spells_machine_and_nothing_else() {
        assert_eq!(want("machine"), Some(Want::Machine));
        assert_eq!(want("chroot"), None);
        assert_eq!(want("sandbox"), None);
        assert_eq!(want(""), None);
        let r = refusal("run", "chroot");
        assert!(
            r.starts_with("podvm run: --podvm-tier takes machine, not \"chroot\""),
            "{r}"
        );
        assert!(r.contains("{{.EnteredRung}}"), "{r}");
        assert!(r.contains("T-1712"), "{r}");
    }

    #[test]
    fn no_flag_is_the_ladder_and_the_flag_selects() {
        assert_eq!(resolve(None), Tier::Ladder);
        assert_eq!(resolve(Some(Want::Machine)), Tier::Machine);
    }

    #[test]
    fn refusing_a_tier_is_a_runtime_error_on_any_machine() {
        // Both branches refuse with 125: legs missing, or legs holding with
        // no guest driver yet. The number is therefore asserted on whatever
        // machine runs the suite.
        assert_eq!(enter_machine("t1302", None), EXIT_RUNTIME_ERROR);
    }

    /// ⭐ TODO/machine.md T-1305. The memory spelling takes bytes and the four
    /// binary suffixes, in either case, with an optional trailing `B`.
    /// Anything else is not a size.
    #[test]
    fn the_mem_spelling_takes_bytes_and_suffixes_and_nothing_else() {
        assert_eq!(parse_mem("512"), Some(512));
        assert_eq!(parse_mem("1K"), Some(1024));
        assert_eq!(parse_mem("2k"), Some(2048));
        assert_eq!(parse_mem("128M"), Some(128 << 20));
        assert_eq!(parse_mem("128MiB"), Some(128 << 20));
        assert_eq!(parse_mem("4GB"), Some(4 << 30));
        assert_eq!(parse_mem("1T"), Some(1 << 40));
        assert_eq!(parse_mem("0"), None);
        assert_eq!(parse_mem(""), None);
        assert_eq!(parse_mem("bogus"), None);
        assert_eq!(parse_mem("1X"), None);
        assert_eq!(parse_mem("-1"), None);
        assert_eq!(parse_mem("1.5G"), None);
        // ⛔ Overflow wraps nowhere: a shift past 64 bits is refused.
        assert_eq!(parse_mem("18446744073709551615T"), None);
        assert_eq!(parse_mem("18446744073709551615"), Some(u64::MAX));
    }

    /// ⭐ TODO/machine.md T-1305. The ceiling comparison: over refuses, equal
    /// and under pass through, and infinity never refuses a finite guest.
    #[test]
    fn the_ceiling_comparison_refuses_only_over() {
        assert!(mem_over_ceiling(1 << 30, 524_288_000));
        assert!(!mem_over_ceiling(1024, 524_288_000));
        assert!(!mem_over_ceiling(524_288_000, 524_288_000));
        assert!(!mem_over_ceiling(1 << 40, u64::MAX));
    }
}
