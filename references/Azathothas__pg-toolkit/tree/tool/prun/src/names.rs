//! The launcher's environment surface, and the one table of carried names.
//!
//! ⛔ **Every name a bundle carries today that is not ours is in this file and
//! nowhere else.** TODO/port.md rule 5 says no upstream name appears in
//! anything this project ships; the same page says the bundle's on-disk names
//! are FROZEN OUTPUT that T-117 renames, because every committed measurement
//! was taken against them. Those two rules meet here: the carried spellings
//! live in one table, `T-117` deletes the right-hand column, and no other file
//! in this crate mentions them.
//!
//! ⭐ Ours is read first in every pair. A bundle that has been rebuilt is
//! already speaking the new names, and one that has not still starts.
//!
//! SPDX-License-Identifier: 0BSD

use crate::envx;

/// One switch, in ours and in the spelling a bundle may still carry.
pub struct Var {
    /// The name this project uses.
    pub ours: &'static str,
    /// The name a bundle built before T-117 carries. Empty where there is
    /// none, which is every switch this launcher added.
    pub carried: &'static str,
    /// What it does, for `--help`.
    pub what: &'static str,
}

/// The whole surface. ⚠ `--help` prints this table, so a switch added to the
/// code and not here is a switch nobody can find out about.
pub const VARS: &[Var] = &[
    Var {
        ours: "PRUN_DIR",
        carried: "SHARUN_DIR",
        what: "the bundle directory, when it is not the one holding this binary",
    },
    Var {
        ours: "PRUN_WORKING_DIR",
        carried: "SHARUN_WORKING_DIR",
        what: "change to this directory before starting the payload",
    },
    Var {
        ours: "PRUN_LOADER_NAME",
        carried: "SHARUN_LDNAME",
        what: "the file name of the loader inside the bundle's library tree",
    },
    Var {
        ours: "PRUN_EXTRA_LIBRARY_PATH",
        carried: "SHARUN_EXTRA_LIBRARY_PATH",
        what: "library directories ahead of the bundle's own",
    },
    Var {
        ours: "PRUN_FALLBACK_LIBRARY_PATH",
        carried: "SHARUN_FALLBACK_LIBRARY_PATH",
        what: "library directories after everything else",
    },
    Var {
        ours: "PRUN_HOST_LIBRARY_PATH",
        carried: "",
        what: "the host directories, replacing the built-in list. Empty means none at all",
    },
    Var {
        ours: "PRUN_MESA_DIR",
        carried: "SHARUN_MESA_PATH",
        what: "an external graphics installation to source drivers and data from",
    },
    Var {
        ours: "PRUN_KEEP_LD_PRELOAD",
        carried: "SHARUN_ALLOW_LD_PRELOAD",
        what: "=1 keeps the caller's LD_PRELOAD instead of clearing it",
    },
    Var {
        ours: "PRUN_KEEP_QT_PLUGIN_PATH",
        carried: "SHARUN_ALLOW_QT_PLUGIN_PATH",
        what: "=1 keeps the caller's QT_PLUGIN_PATH instead of clearing it",
    },
    Var {
        ours: "PRUN_HOST_VULKAN_ICD",
        carried: "SHARUN_ALLOW_SYS_VKICD",
        what: "=1 admits the host's Vulkan driver files, not only its NVIDIA ones",
    },
    Var {
        ours: "PRUN_NO_NVIDIA_PRIME",
        carried: "SHARUN_NO_NVIDIA_EGL_PRIME",
        what: "=1 turns off the NVIDIA-first ordering of the graphics vendor lists",
    },
    Var {
        ours: "PRUN_PRINTENV",
        carried: "SHARUN_PRINTENV",
        what: "=1 prints the environment the payload will start with, to stderr",
    },
    Var {
        ours: "PRUN_REPORT",
        carried: "",
        what: "=1 reports on stderr what the launcher decided and why",
    },
];

/// Read a switch, ours first. ⛔ Both spellings are REMOVED once read: they
/// are the bundle's instruction to its own launcher, and one left in the
/// environment is inherited by every process the payload starts.
pub fn take(ours: &str) -> String {
    let v = VARS.iter().find(|v| v.ours == ours).expect("switch is not in the table");
    let val = if envx::present(v.ours) {
        envx::get(v.ours)
    } else {
        envx::get(v.carried)
    };
    envx::unset(v.ours);
    if !v.carried.is_empty() {
        envx::unset(v.carried);
    }
    val
}

/// Read a switch WITHOUT removing it, for the two the launcher passes on.
pub fn peek(ours: &str) -> String {
    let v = VARS.iter().find(|v| v.ours == ours).expect("switch is not in the table");
    if envx::present(v.ours) {
        envx::get(v.ours)
    } else {
        envx::get(v.carried)
    }
}

/// Whether a switch is on. ⚠ Exactly `1`, as the artefact's surface documents,
/// rather than any non-empty value: a bundle that set one of these to `0`
/// meaning off would otherwise get the opposite of what it asked for.
pub fn on(ours: &str) -> bool {
    take(ours) == "1"
}

/// Announce the bundle's directory under both spellings.
///
/// ⚠ The carried spelling is written because a bundle's own `AppRun.sh` and
/// its hooks read it, and those are the bundle's files rather than this
/// launcher's. T-117 is where that stops.
pub fn export_dir(dir: &str) {
    envx::set("PRUN_DIR", dir);
    envx::set("SHARUN_DIR", dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_switch_has_a_description() {
        for v in VARS {
            assert!(!v.what.is_empty(), "{} has no description", v.ours);
            assert!(v.ours.starts_with("PRUN_"), "{} is not ours", v.ours);
        }
    }

    #[test]
    fn ours_wins_and_both_are_removed() {
        envx::set("PRUN_WORKING_DIR", "/ours");
        envx::set("SHARUN_WORKING_DIR", "/carried");
        assert_eq!(take("PRUN_WORKING_DIR"), "/ours");
        assert!(!envx::present("PRUN_WORKING_DIR"));
        assert!(!envx::present("SHARUN_WORKING_DIR"));
    }

    #[test]
    fn the_carried_spelling_still_answers() {
        envx::unset("PRUN_LOADER_NAME");
        envx::set("SHARUN_LDNAME", "ld-musl-x86_64.so.1");
        assert_eq!(take("PRUN_LOADER_NAME"), "ld-musl-x86_64.so.1");
        assert!(!envx::present("SHARUN_LDNAME"));
    }

    #[test]
    fn a_switch_is_on_only_when_it_says_one() {
        envx::set("PRUN_PRINTENV", "0");
        assert!(!on("PRUN_PRINTENV"));
        envx::set("PRUN_PRINTENV", "yes");
        assert!(!on("PRUN_PRINTENV"));
        envx::set("PRUN_PRINTENV", "1");
        assert!(on("PRUN_PRINTENV"));
    }
}
