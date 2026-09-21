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
/// Where the tier holds there is still nothing that runs a guest: that
/// arrives with `TODO/podvm.md` T-1303 and T-1304, and this says so rather
/// than running something else. Either way the command cannot run, which is
/// docker's 125.
pub fn enter_machine(verb: &str) -> i32 {
    // ⭐ The name is said here because this path never reaches the banner,
    // which is where the alias note otherwise prints. A refusal that names
    // `podbox` to a caller who typed `podvm` is the silent name-taking
    // `TOOL.md` section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        eprint!("{note}");
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
        assert_eq!(enter_machine("t1302"), EXIT_RUNTIME_ERROR);
    }
}
