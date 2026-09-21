//! The tier flag and the `podvm` default. `TODO/podvm.md` T-1302.
//!
//! `--podbox-tier` selects the tier and `podvm` is this binary under another
//! name whose only effect is to change that flag's default. There is one
//! implementation and one parity table; the name is an entry point to the
//! flag rather than a second code path.
//!
//! ⛔ **The explicit flag wins over `argv[0]`, and podbox states the tier it
//! selected whenever the two disagree.** The tier is not cosmetic: a limit
//! the machine tier enforces and the chroot tier cannot is silent
//! degradation wearing a flag, so a caller who cannot tell which tier ran
//! cannot tell whether a limit was honoured.

use podbox_image::error::EXIT_RUNTIME_ERROR;

/// What `--podbox-tier` accepts. Anything else is a flag error at parse
/// time, so this type never carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    Machine,
    Chroot,
}

/// The validated value of one `--podbox-tier` spelling, or `None` where the
/// spelling names no tier podbox has.
pub fn want(s: &str) -> Option<Want> {
    match s {
        "machine" => Some(Want::Machine),
        "chroot" => Some(Want::Chroot),
        _ => None,
    }
}

/// The validated value of one `--podbox-mem` spelling, in bytes, or `None`
/// where the spelling names no size. `TODO/podvm.md` T-1305.
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
    /// The default under every name but `podvm`: the ladder selects.
    Ladder,
    /// `--podbox-tier=chroot`: the chroot tier, even under the `podvm` name.
    Chroot,
    /// `--podbox-tier=machine`, or the `podvm` name with no flag: the legs
    /// are assessed and the tier is refused naming every missing one.
    Machine,
}

/// A resolved invocation: the tier, and the sentence the collision rule
/// requires where the flag overrode the name.
pub struct Resolved {
    pub tier: Tier,
    pub note: Option<String>,
}

/// Resolve the flag against the name podbox was invoked under.
///
/// `invoked` is [`crate::names::invoked_as`], passed in rather than read
/// here so the matrix is unit-testable: that function reads `argv[0]`,
/// which a test cannot set.
pub fn resolve(invoked: &str, flag: Option<Want>) -> Resolved {
    match (invoked == "podvm", flag) {
        (true, None) => Resolved {
            tier: Tier::Machine,
            note: None,
        },
        (false, None) => Resolved {
            tier: Tier::Ladder,
            note: None,
        },
        (_, Some(Want::Machine)) => Resolved {
            tier: Tier::Machine,
            note: None,
        },
        (true, Some(Want::Chroot)) => Resolved {
            tier: Tier::Chroot,
            note: Some(
                "the flag --podbox-tier=chroot wins over the podvm default \
                 (machine); the chroot tier runs"
                    .to_string(),
            ),
        },
        (false, Some(Want::Chroot)) => Resolved {
            tier: Tier::Chroot,
            note: None,
        },
    }
}

/// Apply a machine-tier selection: assess the legs and refuse, naming them.
///
/// `mem` is the validated `--podbox-mem` value in bytes, or `None` where no
/// memory was configured. Where one was, it is judged BEFORE the machine is:
/// a guest over the `RLIMIT_FSIZE` ceiling is refused naming both numbers on
/// every machine, which is what `experiments/148-podvm-fleet.sh` drives
/// (`TODO/podvm.md` T-1305). The legs keep their refusal where no memory was
/// configured, and where the ceiling cannot be read the refusal names that
/// instead of running a guest no bound was checked against.
///
/// Where the tier holds there is still nothing that runs a guest: that
/// arrives with `TODO/podvm.md` T-1303 and T-1304, and this says so rather
/// than running something else. Either way the command cannot run, which is
/// docker's 125.
pub fn enter_machine(verb: &str, mem: Option<u64>) -> i32 {
    // ⭐ The name is said here because this path never reaches the banner,
    // which is where the alias note otherwise prints. A refusal that names
    // `podbox` to a caller who typed `podvm` is the silent name-taking
    // `TOOL.md` section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        eprint!("{note}");
    }
    if let Some(m) = mem {
        match podbox_probe::sys::prlimit(podbox_probe::sys::RLIMIT_FSIZE) {
            Ok((cur, _)) if mem_over_ceiling(m, cur) => {
                eprintln!(
                    "podbox {verb}: machine tier refused: guest memory {m} bytes \
                     over the RLIMIT_FSIZE ceiling {cur} bytes \
                     (prlimit(RLIMIT_FSIZE) cur={cur})"
                );
                return EXIT_RUNTIME_ERROR;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "podbox {verb}: machine tier refused: \
                     prlimit(RLIMIT_FSIZE) could not be read: {}",
                    e.name()
                );
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    let findings = podbox_probe::run();
    match podbox_probe::machine::assess(&findings).refusal() {
        Some(r) => {
            eprintln!("podbox {verb}: {r}");
            EXIT_RUNTIME_ERROR
        }
        None => {
            eprintln!(
                "podbox {verb}: the machine tier holds on this machine; running \
                 a guest arrives with TODO/podvm.md T-1303 and T-1304"
            );
            EXIT_RUNTIME_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_spells_two_tiers_and_nothing_else() {
        assert_eq!(want("machine"), Some(Want::Machine));
        assert_eq!(want("chroot"), Some(Want::Chroot));
        assert_eq!(want("sandbox"), None);
        assert_eq!(want(""), None);
    }

    #[test]
    fn podvm_defaults_to_machine_and_podbox_to_the_ladder() {
        assert_eq!(resolve("podvm", None).tier, Tier::Machine);
        assert!(resolve("podvm", None).note.is_none());
        assert_eq!(resolve("podbox", None).tier, Tier::Ladder);
        assert!(resolve("podbox", None).note.is_none());
        assert_eq!(resolve("docker", None).tier, Tier::Ladder);
    }

    #[test]
    fn an_explicit_machine_needs_no_note_under_either_name() {
        assert_eq!(resolve("podvm", Some(Want::Machine)).tier, Tier::Machine);
        assert_eq!(resolve("podbox", Some(Want::Machine)).tier, Tier::Machine);
        assert!(resolve("podvm", Some(Want::Machine)).note.is_none());
        assert!(resolve("podbox", Some(Want::Machine)).note.is_none());
    }

    #[test]
    fn podvm_with_chroot_states_the_override_and_podbox_does_not() {
        let p = resolve("podvm", Some(Want::Chroot));
        assert_eq!(p.tier, Tier::Chroot);
        let note = p.note.expect("the collision rule requires a sentence");
        assert!(note.contains("--podbox-tier=chroot"), "{note}");
        assert!(note.contains("chroot tier runs"), "{note}");
        assert!(resolve("podbox", Some(Want::Chroot)).note.is_none());
    }

    #[test]
    fn refusing_a_tier_is_a_runtime_error_on_any_machine() {
        // Both branches refuse with 125: legs missing, or legs holding with
        // no guest driver yet. The number is therefore asserted on whatever
        // machine runs the suite.
        assert_eq!(enter_machine("t1302", None), EXIT_RUNTIME_ERROR);
    }

    /// ⭐ TODO/podvm.md T-1305. The memory spelling takes bytes and the four
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

    /// ⭐ TODO/podvm.md T-1305. The ceiling comparison: over refuses, equal
    /// and under pass through, and infinity never refuses a finite guest.
    #[test]
    fn the_ceiling_comparison_refuses_only_over() {
        assert!(mem_over_ceiling(1 << 30, 524_288_000));
        assert!(!mem_over_ceiling(1024, 524_288_000));
        assert!(!mem_over_ceiling(524_288_000, 524_288_000));
        assert!(!mem_over_ceiling(1 << 40, u64::MAX));
    }
}
