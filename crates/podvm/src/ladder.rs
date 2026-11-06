//! TODO/packaging.md T-1003: the CLI half of the launch ladder.
//!
//! Read `PODVM_MODE`, refuse it where no ladder drives, feed the ladder its
//! `Availability` from the probe rows, and drive the chosen rung. Unset means
//! the chroot-by-path entry, untouched: the ladder only engages on a request.

use std::io::Write;

use podvm_enter::ladder::{self, Availability, Mode};
use podvm_enter::memfd::{self, MemfdRefusal};
use podvm_enter::{Plan, RootDir};
use podvm_probe::Findings;

/// Read the request variable. None where unset or empty: the default entry is
/// the chroot by path, and the ladder only engages on a request. An unknown
/// word refuses with the rung list, never a guess.
pub(crate) fn forced_mode() -> Result<Option<Mode>, podvm_enter::Error> {
    parse_request(&std::env::var(Plan::MODE_REQUEST_VAR).unwrap_or_default())
}

/// The pure half of [`forced_mode`], over the value rather than the process
/// environment, so tests drive it without touching process-global state.
fn parse_request(raw: &str) -> Result<Option<Mode>, podvm_enter::Error> {
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
            "PODVM_MODE={raw:?} requests a launch rung, which the machine tier never reaches: it boots a guest rather than entering a rootfs"
        ));
    }
    if verb != "run" {
        return Some(format!(
            "PODVM_MODE={raw:?} requests a launch rung, which `{verb}` does not drive: only foreground `podvm run` honors it"
        ));
    }
    if detach {
        return Some(format!(
            "PODVM_MODE={raw:?} requests a launch rung, which `run -d` does not drive: the detached launcher enters by path"
        ));
    }
    None
}

/// Refuse `PODVM_MODE` on the verbs that never reach the ladder, and on the
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
/// so); rundir and cache from the store's staging directories, made where
/// missing: a rung is up where its staging is writable, down naming it
/// where it is not.
fn availability(
    findings: &Findings,
    memfd_up: bool,
    cache_requested: bool,
    store_root: &std::path::Path,
) -> Availability {
    Availability {
        memfd: memfd_up,
        fuse: podvm_probe::probes::fuse_usable(findings),
        tmpfs: podvm_probe::mounts::Mounts::of(findings).summary() == "full",
        rundir: ensure_stage_base(&store_root.join("runs")),
        cache: cache_requested && ensure_stage_base(&store_root.join("cache")),
        cache_requested,
    }
}

/// Make a staging base exist. True where it exists or was just made:
/// idempotent, empty, and the only mutation availability performs. False
/// names nothing here; the rung refusal names the base.
fn ensure_stage_base(base: &std::path::Path) -> bool {
    std::fs::create_dir_all(base).is_ok()
}

/// Whether `PODVM_CACHE` opts this launch into the cache rung: `1` or
/// `true`, case-insensitive. Anything else leaves it out, so an unset or
/// misspelled value cannot silently litter the disk.
fn cache_requested() -> bool {
    matches!(
        std::env::var("PODVM_CACHE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true"
    )
}

/// Stage a private per-run root under the store's runs/ base: a unique
/// directory replicating the extracted tree, removed by the caller when
/// the payload exits. A tree that is already there refuses rather than
/// merging: staging never donates a stale file to a run.
fn stage_rundir(
    store_root: &std::path::Path,
    rootfs: &str,
) -> Result<std::path::PathBuf, podvm_enter::Error> {
    use podvm_enter::Error;
    let base = store_root.join("runs");
    std::fs::create_dir_all(&base).map_err(|e| {
        Error::Runtime(format!(
            "PODVM_MODE=rundir was forced but {} could not be prepared: {e}",
            base.display()
        ))
    })?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = base.join(format!("run-{}-{nanos}", std::process::id()));
    podvm_enter::stage::copy_tree(std::path::Path::new(rootfs), &dir)
        .map_err(|r| Error::Runtime(format!("PODVM_MODE=rundir was forced but {r}")))?;
    Ok(dir)
}

/// Enter a caller-staged directory rung and clean up behind it. `keep`
/// leaves the tree (the cache rung's persistence); otherwise the staged
/// root goes when the payload exits, on success and on failure alike.
fn enter_staged(
    staged: &std::path::Path,
    plan: &Plan,
    mode: Mode,
    keep: bool,
    err: &mut dyn Write,
) -> podvm_enter::Result<i32> {
    let staged_str = staged.to_string_lossy().into_owned();
    let code = (|| {
        let staged_root = RootDir::open(&staged_str)?;
        podvm_enter::run_ladder(&staged_root, plan, mode, None, err)
    })();
    if !keep {
        let _ = std::fs::remove_dir_all(staged);
    }
    code
}

/// Resolve the payload for the memfd rung: the bytes with the eligibility
/// answer beside them. A payload that is not there is a refusal naming it,
/// not a rung decision: no rung can run what nothing resolved.
fn memfd_payload(
    rootfs: &str,
    argv0: &str,
    path_dirs: &[String],
) -> Result<(Vec<u8>, Option<MemfdRefusal>), podvm_enter::Error> {
    use podvm_enter::Error;
    let bytes = ladder::payload_bytes(rootfs, argv0, path_dirs)
        .map_err(|e| Error::Runtime(format!("PODVM_MODE=memfd was forced but {e}")))?;
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
    store_root: &std::path::Path,
    cache_digest: &str,
    err: &mut dyn Write,
) -> podvm_enter::Result<i32> {
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
    let avail = availability(findings, memfd_up, cache_requested(), store_root);
    // ⭐ TODO/enter.md T-1317. Without chroot the ladder's fd-exec still
    // runs a static payload: the memfd enters with the host's root. A
    // forced memfd over anything else refuses naming the force and the
    // unforced loader family, and every other forced rung refuses naming
    // the force: their mechanisms need what this host denies.
    if !podvm_probe::probes::chroot_usable(findings) {
        if mode == Mode::Memfd {
            match staged {
                Some(bytes) => {
                    let fd = memfd::stage(&bytes)?;
                    return podvm_enter::run_userland(
                        root,
                        plan,
                        plan.argv.clone(),
                        podvm_probe::select::Rung::Userland.word(),
                        Some(fd),
                        err,
                    );
                }
                None => {
                    return Err(podvm_enter::Error::Runtime(format!(
                        "PODVM_MODE=memfd was forced but {}: without chroot(2) \
                         only a static payload enters, through the unforced \
                         loader family where the tier reaches it: run without \
                         PODVM_MODE (TODO/enter.md T-1317)",
                        memfd_why
                            .as_ref()
                            .map(|w| w.to_string())
                            .unwrap_or_else(|| { "the payload did not stage".to_string() })
                    )));
                }
            }
        }
        return Err(podvm_enter::Error::Runtime(format!(
            "PODVM_MODE={} was forced but chroot(2) is denied on this machine, \
             and only the memfd rung enters without it (TODO/enter.md T-1317)",
            mode.name()
        )));
    }
    let chosen = ladder::choose(Some(mode), &avail, memfd_why.as_ref())?;
    if chosen == Mode::Memfd {
        match staged {
            Some(bytes) => {
                let fd = memfd::stage(&bytes)?;
                podvm_enter::run_ladder(root, plan, Mode::Memfd, Some(fd), err)
            }
            // Admission staged the bytes it chose on. Reaching here means the
            // choice and the staging disagreed, which is a defect rather than
            // a rung, and it refuses as one.
            None => Err(podvm_enter::Error::Runtime(
                "the ladder chose the memfd rung it had just refused: this is a podvm defect, not a payload one (TODO/packaging.md T-1003)".to_string(),
            )),
        }
    } else if chosen == Mode::RunDir {
        // ⭐ TODO/packaging.md T-1003. The private per-run root: resolved
        // first (no rung runs what nothing resolved), staged from the
        // extracted tree, entered by path, removed on exit.
        ladder::resolve_payload(rootfs, argv0, path_dirs).map_err(|e| {
            podvm_enter::Error::Runtime(format!("PODVM_MODE=rundir was forced but {e}"))
        })?;
        let staged = stage_rundir(store_root, rootfs)?;
        enter_staged(&staged, plan, Mode::RunDir, false, err)
    } else if chosen == Mode::Cache {
        // ⭐ TODO/packaging.md T-1003. The persistent per-image root:
        // copied once under the digest, reused while its marker names
        // it, kept when the payload exits.
        ladder::resolve_payload(rootfs, argv0, path_dirs).map_err(|e| {
            podvm_enter::Error::Runtime(format!("PODVM_MODE=cache was forced but {e}"))
        })?;
        let staged =
            podvm_enter::stage::stage_cache(store_root, cache_digest, std::path::Path::new(rootfs))
                .map_err(|r| {
                    podvm_enter::Error::Runtime(format!("PODVM_MODE=cache was forced but {r}"))
                })?;
        enter_staged(&staged, plan, Mode::Cache, true, err)
    } else if chosen == Mode::Tmpfs {
        // ⭐ TODO/packaging.md T-1003. The ephemeral per-run root: resolved
        // first (no rung runs what nothing resolved), mounted and staged
        // from the extracted tree, entered by path, unmounted and removed
        // on exit. Where the mount is refused the staging names it, after
        // the ladder's own attach verdict already refused it above.
        ladder::resolve_payload(rootfs, argv0, path_dirs).map_err(|e| {
            podvm_enter::Error::Runtime(format!("PODVM_MODE=tmpfs was forced but {e}"))
        })?;
        let staged = podvm_enter::stage::stage_tmpfs(store_root, std::path::Path::new(rootfs))
            .map_err(|r| {
                podvm_enter::Error::Runtime(format!("PODVM_MODE=tmpfs was forced but {r}"))
            })?;
        let staged_str = staged.to_string_lossy().into_owned();
        let code = (|| {
            let staged_root = RootDir::open(&staged_str)?;
            podvm_enter::run_ladder(&staged_root, plan, Mode::Tmpfs, None, err)
        })();
        podvm_enter::stage::release_tmpfs(&staged);
        code
    } else if chosen == Mode::Fuse {
        // ⭐ TODO/packaging.md T-1003. The served root: resolved first
        // (no rung runs what nothing resolved), served over a FUSE mount
        // straight from the extracted tree, entered by path on the
        // mount, killed, unmounted and removed on exit. Where the node
        // does not open or the mount is refused the staging names it,
        // after the ladder's own open row already refused it above.
        ladder::resolve_payload(rootfs, argv0, path_dirs).map_err(|e| {
            podvm_enter::Error::Runtime(format!("PODVM_MODE=fuse was forced but {e}"))
        })?;
        let mounted = podvm_enter::fuse::serve_tree(store_root, std::path::Path::new(rootfs))
            .map_err(|r| {
                podvm_enter::Error::Runtime(format!("PODVM_MODE=fuse was forced but {r}"))
            })?;
        let mounted_str = mounted.dir.to_string_lossy().into_owned();
        let code = (|| {
            let mounted_root = RootDir::open(&mounted_str)?;
            podvm_enter::run_ladder(&mounted_root, plan, Mode::Fuse, None, err)
        })();
        podvm_enter::fuse::release_mount(&mounted);
        code
    } else {
        // Admission staged what it chose on. Reaching here means the
        // choice and the staging disagreed, which is a defect rather
        // than a rung, and it refuses as one.
        Err(podvm_enter::Error::Runtime(
            "the ladder chose a rung no arm drives: this is a podvm defect, not a payload one (TODO/packaging.md T-1003)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests;
