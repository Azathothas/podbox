//! Where the CLI meets [`podvm_complete`], and where
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0804's one switch is enforced.
//!
//! ⭐ **`--strict` turns every degradation into a refusal**, so a caller that
//! needs real semantics demands them in one flag instead of parsing the banner.
//! Three things feed it, and all three are read from something that was
//! measured rather than declared:
//!
//! 1. **the flags the caller passed**, against the parity table's own status
//!    for each. A `Degraded` or `Stub` row is a difference from docker that
//!    this invocation actually incurs;
//! 2. **the rung podvm selected**, from the probe. Below `namespace` the
//!    payload is in a `chroot` sharing this machine's process table, network,
//!    IPC and mounts;
//! 3. **the completion layer's own report**, one row per fixup, from
//!    [`podvm_complete::Report::degradations`].
//!
//! ⛔ A `None` row is already fatal without `--strict`, and has been since
//! T-0801 made the table binding: `parity::admit` refuses it in the parser. So
//! `--strict` is about the two statuses that otherwise let a run proceed, and
//! about the run itself.

use std::io::Write;

use podvm_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

/// What the caller asked the completion layer for.
#[derive(Debug, Clone, Default)]
pub struct Ask {
    pub container_name: Option<String>,
    pub add_hosts: Vec<String>,
    /// `--no-source-fixup`. T-0411.
    pub no_source_fixup: bool,
    /// `--no-host-cas`. T-0407.
    pub no_host_cas: bool,
    /// `--no-steps`. T-0412.
    pub no_steps: bool,
    /// `--strict`. T-0804.
    pub strict: bool,
    /// `-q`, `--quiet`. T-1416: force the banner off.
    pub quiet: bool,
    /// `--verbose`. T-1416: print the banner.
    pub verbose: bool,
    /// `--strict=all`. T-1415: the T-0804 behavior, refusing every
    /// Degraded and Stub difference including dev-shim substitutions.
    pub strict_all: bool,
    /// Every flag this invocation actually passed, in the caller's spelling, so
    /// `--strict` can look each one up. ⚠ Collected by the parser rather than
    /// re-derived: a flag that took a value is one argument here and two on the
    /// command line, and re-splitting is a second parser.
    pub seen_flags: Vec<String>,
}

/// `<store>/config`, and the one key T-0804 defines.
///
/// ⭐ **Suppressible by config, never by the command line.** `ruri` allows
/// `--disable-warnings` to silence its degradation notices, which is the
/// `sandlock` failure mode with a flag in front of it. podvm's equivalent is a
/// file a machine's operator sets once, so a single run cannot hide what it is.
pub const CONFIG_FILE: &str = "config";
pub const BANNER_KEY: &str = "banner";

/// Whether the banner prints on this run.
///
/// TODO/cli.md T-1416. Operator ruling 2026-09-30 (issues 77 and 79),
/// REVERSING the T-0804 command-line rule in writing: quiet by default
/// (empty stderr on success), the banner under `--verbose`, and
/// `-q`/`--quiet` forcing it off. Refusals and payload stderr are
/// untouched by this switch: it silences a notice, never a refusal. The
/// store config's `banner = quiet` still forces off, as before.
///
/// ⛔ `-v` is NOT the verbose switch: `run -v` is docker's `--volume`,
/// refused by the parity table with status None, and bundled clusters
/// expand before admission, so reclaiming `-v` would turn a volume
/// refusal into a banner switch. The verbose switch is long-only
/// `--verbose`, and `-q`/`--quiet` is the force-off.
pub fn banner_loud(store: &podvm_image::Store, ask: &Ask) -> bool {
    if ask.quiet || banner_quiet(store) {
        return false;
    }
    ask.verbose
}

/// Is the banner suppressed on this machine? ⛔ Default no, always.
pub fn banner_quiet(store: &podvm_image::Store) -> bool {
    let p = store.root().join(CONFIG_FILE);
    let Ok(text) = std::fs::read_to_string(&p) else {
        return false;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == BANNER_KEY
        {
            return matches!(v.trim(), "quiet" | "off" | "false" | "0");
        }
    }
    false
}

/// Run the completion layer over `rootfs`, fold its report into `banner`, and
/// apply `--strict`.
///
/// ⛔ **The completion layer never fails the run on its own.** A fixup podvm
/// could not apply is a `Failed` row and a banner line; the caller turns that
/// into a refusal with `--strict` and not otherwise, because a rootfs podvm
/// could not write a resolver into is one the payload may still be able to use.
pub fn prepare(
    verb: &str,
    rootfs: &str,
    ask: &Ask,
    banner: &mut String,
) -> Result<podvm_complete::Report, i32> {
    let opts = podvm_complete::Options {
        container_name: ask.container_name.clone(),
        add_hosts: ask.add_hosts.clone(),
        source_fixup: !ask.no_source_fixup,
        host_cas: !ask.no_host_cas,
        steps: !ask.no_steps,
        ..podvm_complete::Options::default()
    };
    let report = match podvm_complete::complete(rootfs, &opts) {
        Ok(r) => r,
        Err(e) => {
            // ⛔ Even this is not fatal without --strict: the rootfs is there
            // and the payload may not need what podvm could not do.
            banner.push_str(&format!(
                "podvm: complete: refused: the completion layer could not run at all: {e}. \
                 The payload is entering a rootfs podvm did not prepare\n"
            ));
            if ask.strict {
                eprint!("{banner}");
                eprintln!("podvm {verb}: --strict, and the completion layer could not run");
                return Err(EXIT_RUNTIME_ERROR);
            }
            return Ok(podvm_complete::Report::default());
        }
    };
    banner.push_str(&report.banner());
    Ok(report)
}

mod strict;
pub use strict::*;

mod capability;
pub use capability::*;

#[cfg(test)]
mod tests;
