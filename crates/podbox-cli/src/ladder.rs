//! TODO/packaging.md T-1003: the CLI half of the launch ladder.
//!
//! Read `PODBOX_MODE`, refuse it where no ladder drives, feed the ladder its
//! `Availability` from the probe rows, and drive the chosen rung. Unset means
//! the chroot-by-path entry, untouched: the ladder only engages on a request.

use std::io::Write;

use podbox_enter::ladder::{self, Availability, Mode};
use podbox_enter::memfd::{self, MemfdRefusal};
use podbox_enter::{Plan, RootDir};
use podbox_probe::Findings;

/// Read the request variable. None where unset or empty: the default entry is
/// the chroot by path, and the ladder only engages on a request. An unknown
/// word refuses with the rung list, never a guess.
pub(crate) fn forced_mode() -> Result<Option<Mode>, podbox_enter::Error> {
    parse_request(&std::env::var(Plan::MODE_REQUEST_VAR).unwrap_or_default())
}

/// The pure half of [`forced_mode`], over the value rather than the process
/// environment, so tests drive it without touching process-global state.
fn parse_request(raw: &str) -> Result<Option<Mode>, podbox_enter::Error> {
    let req = raw.trim();
    if req.is_empty() {
        return Ok(None);
    }
    ladder::Mode::parse(req).map(Some)
}

/// Refuse a request the verb cannot honor. Only foreground `run` drives the
/// ladder: `create` and `run -d` launch through the supervisor, `exec`
/// re-enters, and the machine tier never reaches the chroot entry. A force
/// that fell through any of those would run the payload from a place the
/// caller forbade, so each refuses naming what was asked.
fn scope_refusal(verb: &str, detach: bool, machine_tier: bool, raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }
    if machine_tier {
        return Some(format!(
            "PODBOX_MODE={raw:?} requests a launch rung, which the machine tier never reaches: it boots a guest rather than entering a rootfs"
        ));
    }
    if verb != "run" {
        return Some(format!(
            "PODBOX_MODE={raw:?} requests a launch rung, which `{verb}` does not drive: only foreground `podbox run` honors it"
        ));
    }
    if detach {
        return Some(format!(
            "PODBOX_MODE={raw:?} requests a launch rung, which `run -d` does not drive: the detached launcher enters by path"
        ));
    }
    None
}

/// Refuse `PODBOX_MODE` on the verbs that never reach the ladder, and on the
/// machine tier. `run`'s `prepare` and `exec` both ask this before anything is
/// fetched, so a force refused here costs the caller nothing.
pub(crate) fn refuse_where_undriven(
    verb: &str,
    detach: bool,
    machine_tier: bool,
) -> Result<(), String> {
    let raw = std::env::var(Plan::MODE_REQUEST_VAR).unwrap_or_default();
    if verb == "run" && !detach && !machine_tier {
        return Ok(());
    }
    match scope_refusal(verb, detach, machine_tier, &raw) {
        Some(text) => Err(text),
        None => Ok(()),
    }
}

/// Feed the ladder from the probe rows. FUSE from the open row beside ptmx's
/// own rule; tmpfs from the attach verdict (an ephemeral tmpfs needs a mount
/// the payload can use, and `Mounts::summary` is the one predicate that says
/// so); rundir and cache stay down until the CLI wires them, because claiming
/// one is claiming a rung that does not exist.
fn availability(findings: &Findings, memfd_up: bool, cache_requested: bool) -> Availability {
    Availability {
        memfd: memfd_up,
        fuse: podbox_probe::probes::fuse_usable(findings),
        tmpfs: podbox_probe::mounts::Mounts::of(findings).summary() == "full",
        rundir: false,
        cache: false,
        cache_requested,
    }
}

/// Whether `PODBOX_CACHE` opts this launch into the cache rung: `1` or
/// `true`, case-insensitive. Anything else leaves it out, so an unset or
/// misspelled value cannot silently litter the disk.
fn cache_requested() -> bool {
    matches!(
        std::env::var("PODBOX_CACHE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true"
    )
}

/// Resolve the payload for the memfd rung: the bytes with the eligibility
/// answer beside them. A payload that is not there is a refusal naming it,
/// not a rung decision: no rung can run what nothing resolved.
fn memfd_payload(
    rootfs: &str,
    argv0: &str,
    path_dirs: &[String],
) -> Result<(Vec<u8>, Option<MemfdRefusal>), podbox_enter::Error> {
    use podbox_enter::Error;
    let bytes = ladder::payload_bytes(rootfs, argv0, path_dirs)
        .map_err(|e| Error::Runtime(format!("PODBOX_MODE=memfd was forced but {e}")))?;
    let why = memfd::eligible(&bytes).err();
    Ok((bytes, why))
}

/// Choose the forced rung and drive it. Admission (is the rung up?) and
/// execution (write this rung's bytes) meet in one place, so the two cannot
/// disagree about what "up" meant.
#[allow(clippy::too_many_arguments)]
pub(crate) fn enter_forced(
    root: &RootDir,
    plan: &Plan,
    mode: Mode,
    rootfs: &str,
    argv0: &str,
    path_dirs: &[String],
    findings: &Findings,
    err: &mut dyn Write,
) -> podbox_enter::Result<i32> {
    let mut memfd_up = false;
    let mut memfd_why: Option<MemfdRefusal> = None;
    let mut staged: Option<Vec<u8>> = None;
    if mode == Mode::Memfd {
        let (bytes, why) = memfd_payload(rootfs, argv0, path_dirs)?;
        memfd_why = why;
        memfd_up = memfd_why.is_none() && memfd::kernel_takes_memfd();
        if memfd_up {
            staged = Some(bytes);
        }
    }
    let avail = availability(findings, memfd_up, cache_requested());
    // ⭐ TODO/enter.md T-1317. Without chroot the ladder's fd-exec still
    // runs a static payload: the memfd enters with the host's root. A
    // forced memfd over anything else refuses naming the force and the
    // unforced loader family, and every other forced rung refuses naming
    // the force: their mechanisms need what this host denies.
    if !podbox_probe::probes::chroot_usable(findings) {
        if mode == Mode::Memfd {
            match staged {
                Some(bytes) => {
                    let fd = memfd::stage(&bytes)?;
                    return podbox_enter::run_userland(
                        root,
                        plan,
                        plan.argv.clone(),
                        podbox_probe::select::Rung::Userland.word(),
                        Some(fd),
                        err,
                    );
                }
                None => {
                    return Err(podbox_enter::Error::Runtime(format!(
                        "PODBOX_MODE=memfd was forced but {}: without chroot(2) \
                         only a static payload enters, through the unforced \
                         loader family where the tier reaches it: run without \
                         PODBOX_MODE (TODO/enter.md T-1317)",
                        memfd_why
                            .as_ref()
                            .map(|w| w.to_string())
                            .unwrap_or_else(|| { "the payload did not stage".to_string() })
                    )));
                }
            }
        }
        return Err(podbox_enter::Error::Runtime(format!(
            "PODBOX_MODE={} was forced but chroot(2) is denied on this machine, \
             and only the memfd rung enters without it (TODO/enter.md T-1317)",
            mode.name()
        )));
    }
    let chosen = ladder::choose(Some(mode), &avail, memfd_why.as_ref())?;
    if chosen == Mode::Memfd {
        match staged {
            Some(bytes) => {
                let fd = memfd::stage(&bytes)?;
                podbox_enter::run_ladder(root, plan, Mode::Memfd, Some(fd), err)
            }
            // Admission staged the bytes it chose on. Reaching here means the
            // choice and the staging disagreed, which is a defect rather than
            // a rung, and it refuses as one.
            None => Err(podbox_enter::Error::Runtime(
                "the ladder chose the memfd rung it had just refused: this is a podbox defect, not a payload one (TODO/packaging.md T-1003)".to_string(),
            )),
        }
    } else {
        // Ordered but not rung-complete: `spawn_ladder` refuses naming the
        // rung, which is the caller skipping the choice made audible.
        podbox_enter::run_ladder(root, plan, chosen, None, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use podbox_probe::verdict::Outcome;

    fn findings_with(rows: Vec<(&'static str, Outcome)>) -> Findings {
        Findings {
            rows,
            ..Findings::empty()
        }
    }

    /// Unset and empty mean the default entry: the ladder only engages on a
    /// request, and no request is the common case.
    #[test]
    fn no_request_means_the_default_entry() {
        assert_eq!(parse_request("").unwrap(), None);
        assert_eq!(parse_request("   ").unwrap(), None);
        assert_eq!(parse_request("memfd").unwrap(), Some(Mode::Memfd));
    }

    /// An unknown word refuses with the rung list and exits docker's runtime
    /// error, never a guess at what was meant.
    #[test]
    fn an_unknown_request_refuses_with_the_list() {
        let e = parse_request("memfd2").unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("memfd2"), "{text}");
        assert!(text.contains("rundir"), "{text}");
        assert_eq!(e.exit_code(), podbox_enter::EXIT_RUNTIME_ERROR);
    }

    /// Only foreground `run` drives the ladder. Every other verb, the
    /// detached form and the machine tier refuse naming what was asked, and
    /// no request refuses nowhere.
    #[test]
    fn the_ladder_is_foreground_run_or_a_named_refusal() {
        assert!(scope_refusal("run", false, false, "").is_none());
        assert!(scope_refusal("run", false, false, "memfd").is_none());
        for (verb, detach, machine) in [
            ("exec", false, false),
            ("create", false, false),
            ("run", true, false),
            ("run", false, true),
        ] {
            let text = scope_refusal(verb, detach, machine, "memfd")
                .expect("a force outside foreground run refuses");
            assert!(text.contains("PODBOX_MODE"), "{text}");
            assert!(text.contains("memfd"), "{text}");
        }
    }

    /// The FUSE feed is the open row beside ptmx's rule: an `Ok` open is up,
    /// a denial is down, and a missing row is down rather than a promise.
    #[test]
    fn fuse_is_up_only_where_the_open_row_answered_ok() {
        let up = findings_with(vec![("open(/dev/fuse, O_RDWR)", Outcome::ok())]);
        assert!(availability(&up, false, false).fuse);
        let down = findings_with(vec![(
            "open(/dev/fuse, O_RDWR)",
            Outcome::denied(podbox_probe::sys::EPERM),
        )]);
        assert!(!availability(&down, false, false).fuse);
        assert!(!availability(&Findings::empty(), false, false).fuse);
    }

    /// The tmpfs feed is the attach verdict: only a mount the payload can use
    /// counts, and anything less refuses naming the mount.
    #[test]
    fn tmpfs_is_up_only_where_a_mount_attached() {
        let attached = findings_with(vec![("move_mount(-> /tmp/mm-probe)", Outcome::ok())]);
        assert!(availability(&attached, false, false).tmpfs);
        assert!(!availability(&Findings::empty(), false, false).tmpfs);
    }

    /// Rundir and cache stay down until the CLI wires them: claiming one is
    /// claiming a rung that does not exist.
    #[test]
    fn the_unwired_rungs_stay_down() {
        let avail = availability(&Findings::empty(), true, true);
        assert!(avail.memfd);
        assert!(!avail.rundir);
        assert!(!avail.cache);
        assert!(avail.cache_requested);
    }

    /// A forced sketch rung refuses naming the sketch rather than falling
    /// through to a rung the caller did not ask for. No fork happens: the
    /// refusal precedes every entry. The findings admit chroot, so the
    /// no-chroot branch below is not what refuses here.
    #[test]
    fn a_forced_sketch_rung_refuses_before_any_entry() {
        let dir = std::env::temp_dir().join(format!("podbox-ladder-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
        let plan = Plan {
            argv: vec!["true".to_string()],
            env: Vec::new(),
            working_dir: "/".to_string(),
            fds: podbox_enter::Fds::default(),
            banner: String::new(),
            path_dirs: vec!["/bin".to_string()],
        };
        let mut sink = Vec::new();
        let usable = findings_with(vec![("chroot(/tmp)", Outcome::ok())]);
        let e = enter_forced(
            &root,
            &plan,
            Mode::RunDir,
            dir.to_str().unwrap(),
            "true",
            &plan.path_dirs,
            &usable,
            &mut sink,
        )
        .unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("not implemented"), "{text}");
        assert_eq!(e.exit_code(), podbox_enter::EXIT_RUNTIME_ERROR);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TODO/enter.md T-1317. Where chroot is denied, a forced sketch rung
    /// refuses naming the force and the denial rather than the sketch:
    /// their mechanisms need what this host denies, and only the memfd
    /// rung enters without it.
    #[test]
    fn a_forced_sketch_rung_without_chroot_names_the_denial() {
        let dir = std::env::temp_dir().join(format!("podbox-ladder-cli-nc{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
        let plan = Plan {
            argv: vec!["true".to_string()],
            env: Vec::new(),
            working_dir: "/".to_string(),
            fds: podbox_enter::Fds::default(),
            banner: String::new(),
            path_dirs: vec!["/bin".to_string()],
        };
        let mut sink = Vec::new();
        let denied = findings_with(vec![(
            "chroot(/tmp)",
            Outcome::denied(podbox_probe::sys::EPERM),
        )]);
        let e = enter_forced(
            &root,
            &plan,
            Mode::RunDir,
            dir.to_str().unwrap(),
            "true",
            &plan.path_dirs,
            &denied,
            &mut sink,
        )
        .unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("PODBOX_MODE=rundir was forced"), "{text}");
        assert!(text.contains("chroot(2) is denied"), "{text}");
        assert_eq!(e.exit_code(), podbox_enter::EXIT_RUNTIME_ERROR);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A forced memfd over a missing payload refuses naming the force and
    /// the missing file, not a rung decision: no rung can run what nothing
    /// resolved. No fork happens: the resolve precedes every entry.
    #[test]
    fn a_forced_memfd_over_a_missing_payload_names_it() {
        let dir = std::env::temp_dir().join(format!("podbox-ladder-cli-m{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
        let plan = Plan {
            argv: vec!["absent".to_string()],
            env: Vec::new(),
            working_dir: "/".to_string(),
            fds: podbox_enter::Fds::default(),
            banner: String::new(),
            path_dirs: vec!["/bin".to_string()],
        };
        let mut sink = Vec::new();
        let e = enter_forced(
            &root,
            &plan,
            Mode::Memfd,
            dir.to_str().unwrap(),
            "absent",
            &plan.path_dirs,
            &Findings::empty(),
            &mut sink,
        )
        .unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("PODBOX_MODE=memfd was forced"), "{text}");
        assert!(text.contains("names no file"), "{text}");
        assert_eq!(e.exit_code(), podbox_enter::EXIT_RUNTIME_ERROR);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A forced memfd over a script payload refuses naming the force and the
    /// route-past, rather than exec'ing down a rung the bytes cannot take.
    /// No fork happens here either: the refusal precedes the entry.
    #[test]
    fn a_forced_memfd_over_a_script_refuses_naming_both() {
        let dir = std::env::temp_dir().join(format!("podbox-ladder-cli-s{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin/run.sh"), b"#!/bin/sh\necho hi\n").unwrap();
        let root = RootDir::open(dir.to_str().unwrap()).expect("a temp dir opens");
        let plan = Plan {
            argv: vec!["run.sh".to_string()],
            env: Vec::new(),
            working_dir: "/".to_string(),
            fds: podbox_enter::Fds::default(),
            banner: String::new(),
            path_dirs: vec!["/bin".to_string()],
        };
        let mut sink = Vec::new();
        let e = enter_forced(
            &root,
            &plan,
            Mode::Memfd,
            dir.to_str().unwrap(),
            "run.sh",
            &plan.path_dirs,
            &Findings::empty(),
            &mut sink,
        )
        .unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("PODBOX_MODE=memfd was forced"), "{text}");
        assert!(text.contains("routes past"), "{text}");
        assert_eq!(e.exit_code(), podbox_enter::EXIT_RUNTIME_ERROR);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
