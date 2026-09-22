//! [`TODO/packaging.md`](../../../TODO/packaging.md) T-1003: the launch ladder,
//! and the order it is tried in.
//!
//! memfd, then FUSE, then an ephemeral tmpfs, then a private run directory,
//! then a persistent cache: the onelf shape
//! (`references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:125-131`),
//! shortened where this runtime refuses a rung by probe rather than omitting
//! it. Each **forced** mode refuses with a named reason rather than falling
//! through, copying that file's `:216-218`, `:235-237` and `:255-257`.
//!
//! ⭐ **Rung-complete is the memfd rung; the rest is sketched.** The memfd
//! driver ([`crate::memfd`]) writes bytes, seals where accepted, and execs the
//! fd. FUSE and tmpfs are probe inputs: this module orders them and refuses
//! them by name, and `podbox-probe` measures them. The run-dir and cache rungs
//! need extraction the CLI does not wire yet, so they answer "not implemented"
//! until it does. The single file with an embedded rootfs is a follow-up the
//! entry names, not a rung built here.
//!
//! ⚠ This module orders and refuses; it never measures. What the machine
//! permits arrives as [`Availability`], read by the caller from the probe, so
//! every decision below is a pure function unit tests drive.

use crate::memfd::MemfdRefusal;
use crate::{Error, Result};

/// A rung of the ladder, in trial order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Memfd,
    Fuse,
    Tmpfs,
    RunDir,
    Cache,
}

impl Mode {
    /// The trial order: memfd, FUSE, tmpfs, run directory, cache.
    pub const ORDER: [Mode; 5] = [
        Mode::Memfd,
        Mode::Fuse,
        Mode::Tmpfs,
        Mode::RunDir,
        Mode::Cache,
    ];

    /// The word `PODBOX_MODE` carries and `PODBOX_ACTIVE_MODE` reports.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Memfd => "memfd",
            Mode::Fuse => "fuse",
            Mode::Tmpfs => "tmpfs",
            Mode::RunDir => "rundir",
            Mode::Cache => "cache",
        }
    }

    /// Parse a `PODBOX_MODE` request. `run-dir` is accepted beside `rundir`:
    /// docker spells compound flags both ways and the request must not fail on
    /// a hyphen.
    pub fn parse(s: &str) -> Result<Mode> {
        match s {
            "memfd" => Ok(Mode::Memfd),
            "fuse" => Ok(Mode::Fuse),
            "tmpfs" => Ok(Mode::Tmpfs),
            "rundir" | "run-dir" | "run_dir" => Ok(Mode::RunDir),
            "cache" => Ok(Mode::Cache),
            other => Err(Error::Runtime(format!(
                "PODBOX_MODE={other:?} names no launch rung. The rungs are: \
                 memfd, fuse, tmpfs, rundir, cache (TODO/packaging.md T-1003)"
            ))),
        }
    }
}

/// What the prober found, as the ladder's inputs.
///
/// ⭐ Passed in rather than read here. FUSE needs `/dev/fuse`, which `mknod`
/// cannot create, and an ephemeral tmpfs needs a mount: both rungs are refused
/// by probe, not omitted, so the same binary uses them where they exist.
pub struct Availability {
    /// The kernel takes `memfd_create` and the payload is static-PIE with no
    /// `PT_INTERP` ([`crate::memfd::eligible`]).
    pub memfd: bool,
    /// `/dev/fuse` opens.
    pub fuse: bool,
    /// A mount is permitted.
    pub tmpfs: bool,
    /// ⛔ Sketch: extraction to a private run directory is not wired yet, so
    /// this stays false until the CLI wires it. Setting it true claims a rung
    /// that does not exist.
    pub rundir: bool,
    /// ⛔ Sketch, as above, for the persistent cache.
    pub cache: bool,
    /// The publisher asked for cache at pack time, or `PODBOX_CACHE` opts this
    /// launch in: the cache rung runs only when asked for, copying onelf's
    /// `CACHE_REQUESTED` gate.
    pub cache_requested: bool,
}

impl Availability {
    /// Why `mode` is down, naming the probe or the missing wiring.
    pub fn refusal_reason(mode: Mode, memfd_why: Option<&MemfdRefusal>) -> String {
        match mode {
            Mode::Memfd => match memfd_why {
                Some(why) => format!("memfd mode is unavailable: {why}"),
                None => {
                    "memfd mode is unavailable: memfd_create is refused on this kernel".to_string()
                }
            },
            Mode::Fuse => {
                "/dev/fuse did not open on this machine, so FUSE mode is unavailable".to_string()
            }
            Mode::Tmpfs => {
                "a mount is refused on this runtime, so tmpfs mode is unavailable".to_string()
            }
            Mode::RunDir => "the private run-directory rung is not implemented yet \
                 (TODO/packaging.md T-1003: sketched, not rung-complete)"
                .to_string(),
            Mode::Cache => "the persistent-cache rung is not implemented yet \
                 (TODO/packaging.md T-1003: sketched, not rung-complete)"
                .to_string(),
        }
    }

    fn is_up(&self, mode: Mode) -> bool {
        match mode {
            Mode::Memfd => self.memfd,
            Mode::Fuse => self.fuse,
            Mode::Tmpfs => self.tmpfs,
            Mode::RunDir => self.rundir,
            Mode::Cache => self.cache,
        }
    }
}

/// Choose the rung: the forced one where asked, else the first one up.
///
/// ⛔ A forced mode that is down REFUSES with a named reason rather than
/// falling through to a rung the caller did not ask for. Falling through
/// would run the payload from a place the caller forbade, and after the exec
/// nothing can be said about where it went.
pub fn choose(
    forced: Option<Mode>,
    avail: &Availability,
    memfd_why: Option<&MemfdRefusal>,
) -> Result<Mode> {
    if let Some(mode) = forced {
        if avail.is_up(mode) {
            return Ok(mode);
        }
        return Err(Error::Runtime(format!(
            "PODBOX_MODE={} was forced but {}",
            mode.name(),
            Availability::refusal_reason(mode, memfd_why)
        )));
    }
    for mode in Mode::ORDER {
        // ⛔ The cache leaves an extraction on disk, so it runs only when asked
        // for. Otherwise a machine with nothing else up silently litters it.
        if mode == Mode::Cache && !avail.cache_requested {
            continue;
        }
        if avail.is_up(mode) {
            return Ok(mode);
        }
    }
    if avail.cache && !avail.cache_requested {
        return Err(Error::Runtime(
            "no execution rung is available on this host; set PODBOX_CACHE=1 to \
             allow a persistent extraction under the cache directory \
             (TODO/packaging.md T-1003)"
                .to_string(),
        ));
    }
    Err(Error::Runtime(format!(
        "no execution rung is available on this host: {}",
        Mode::ORDER
            .iter()
            .map(|m| Availability::refusal_reason(*m, memfd_why))
            .collect::<Vec<_>>()
            .join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EXIT_RUNTIME_ERROR;

    fn all_up() -> Availability {
        Availability {
            memfd: true,
            fuse: true,
            tmpfs: true,
            rundir: true,
            cache: true,
            cache_requested: true,
        }
    }

    fn all_down() -> Availability {
        Availability {
            memfd: false,
            fuse: false,
            tmpfs: false,
            rundir: false,
            cache: false,
            cache_requested: false,
        }
    }

    /// ⭐ The order is the design: memfd first, cache last and only when asked.
    #[test]
    fn the_trial_order_is_memfd_first_and_cache_last() {
        assert_eq!(
            Mode::ORDER.map(Mode::name),
            ["memfd", "fuse", "tmpfs", "rundir", "cache"]
        );
        assert_eq!(choose(None, &all_up(), None).unwrap(), Mode::Memfd);
    }

    /// ⛔ A forced mode that is down refuses naming it, rather than falling
    /// through to a rung the caller did not ask for.
    #[test]
    fn a_forced_mode_that_is_down_refuses_rather_than_falling_through() {
        let e = choose(Some(Mode::Fuse), &all_down(), None).unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("fuse"), "{text}");
        assert!(text.contains("/dev/fuse"), "{text}");
        assert_eq!(e.exit_code(), EXIT_RUNTIME_ERROR);
    }

    /// A forced mode that is up is honoured, including past an earlier rung.
    #[test]
    fn a_forced_mode_that_is_up_is_honoured() {
        assert_eq!(
            choose(Some(Mode::Tmpfs), &all_up(), None).unwrap(),
            Mode::Tmpfs
        );
    }

    /// Unknown requests refuse with the list, never a guess.
    #[test]
    fn an_unknown_request_refuses_with_the_list() {
        let e = Mode::parse("memfd2").unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("memfd2"), "{text}");
        assert!(text.contains("rundir"), "{text}");
        assert_eq!(e.exit_code(), EXIT_RUNTIME_ERROR);
    }

    /// `run-dir` and `run_dir` spell the same rung as `rundir`.
    #[test]
    fn the_run_dir_spellings_agree() {
        assert_eq!(Mode::parse("run-dir").unwrap(), Mode::RunDir);
        assert_eq!(Mode::parse("run_dir").unwrap(), Mode::RunDir);
        assert_eq!(Mode::parse("rundir").unwrap(), Mode::RunDir);
    }

    /// ⛔ The cache runs only when asked for: a cache-capable machine without
    /// the opt-in skips it rather than littering the disk silently.
    #[test]
    fn the_cache_runs_only_when_asked_for() {
        let mut avail = all_down();
        avail.cache = true;
        let e = choose(None, &avail, None).unwrap_err();
        assert!(format!("{e}").contains("PODBOX_CACHE=1"), "{e}");

        avail.cache_requested = true;
        assert_eq!(choose(None, &avail, None).unwrap(), Mode::Cache);
    }

    /// With nothing up and nothing asked, the refusal names every rung.
    #[test]
    fn nothing_up_is_a_refusal_naming_every_rung() {
        let e = choose(None, &all_down(), None).unwrap_err();
        let text = format!("{e}");
        for rung in ["memfd", "/dev/fuse", "tmpfs", "run-directory", "cache"] {
            assert!(text.contains(rung), "{text}");
        }
    }

    /// The memfd refusal carries the eligibility answer beside it.
    #[test]
    fn a_forced_memfd_names_why_the_payload_cannot_go_that_way() {
        let why = MemfdRefusal::HasInterp("/lib64/ld-linux-x86-64.so.2".to_string());
        let e = choose(Some(Mode::Memfd), &all_down(), Some(&why)).unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("ld-linux"), "{text}");
    }

    /// A refusal is docker's runtime error, read from the error and not a
    /// literal: [`Error::exit_code`] is the one place that maps it.
    #[test]
    fn refusals_are_dockers_runtime_error() {
        let e = choose(Some(Mode::Fuse), &all_down(), None).unwrap_err();
        assert_eq!(e.exit_code(), EXIT_RUNTIME_ERROR);
    }
}
