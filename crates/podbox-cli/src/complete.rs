//! Where the CLI meets [`podbox_complete`], and where
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
//! 2. **the rung podbox selected**, from the probe. Below `namespace` the
//!    payload is in a `chroot` sharing this machine's process table, network,
//!    IPC and mounts;
//! 3. **the completion layer's own report**, one row per fixup, from
//!    [`podbox_complete::Report::degradations`].
//!
//! ⛔ A `None` row is already fatal without `--strict`, and has been since
//! T-0801 made the table binding: `parity::admit` refuses it in the parser. So
//! `--strict` is about the two statuses that otherwise let a run proceed, and
//! about the run itself.

use std::io::Write;

use podbox_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

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
/// `sandlock` failure mode with a flag in front of it. podbox's equivalent is a
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
pub fn banner_loud(store: &podbox_image::Store, ask: &Ask) -> bool {
    if ask.quiet || banner_quiet(store) {
        return false;
    }
    ask.verbose
}

/// Is the banner suppressed on this machine? ⛔ Default no, always.
pub fn banner_quiet(store: &podbox_image::Store) -> bool {
    let p = store.root().join(CONFIG_FILE);
    let Ok(text) = std::fs::read_to_string(&p) else {
        return false;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == BANNER_KEY {
                return matches!(v.trim(), "quiet" | "off" | "false" | "0");
            }
        }
    }
    false
}

/// Run the completion layer over `rootfs`, fold its report into `banner`, and
/// apply `--strict`.
///
/// ⛔ **The completion layer never fails the run on its own.** A fixup podbox
/// could not apply is a `Failed` row and a banner line; the caller turns that
/// into a refusal with `--strict` and not otherwise, because a rootfs podbox
/// could not write a resolver into is one the payload may still be able to use.
pub fn prepare(
    verb: &str,
    rootfs: &str,
    ask: &Ask,
    banner: &mut String,
) -> Result<podbox_complete::Report, i32> {
    let opts = podbox_complete::Options {
        container_name: ask.container_name.clone(),
        add_hosts: ask.add_hosts.clone(),
        source_fixup: !ask.no_source_fixup,
        host_cas: !ask.no_host_cas,
        steps: !ask.no_steps,
        ..podbox_complete::Options::default()
    };
    let report = match podbox_complete::complete(rootfs, &opts) {
        Ok(r) => r,
        Err(e) => {
            // ⛔ Even this is not fatal without --strict: the rootfs is there
            // and the payload may not need what podbox could not do.
            banner.push_str(&format!(
                "podbox: complete: refused: the completion layer could not run at all: {e}. \
                 The payload is entering a rootfs podbox did not prepare\n"
            ));
            if ask.strict {
                eprint!("{banner}");
                eprintln!("podbox {verb}: --strict, and the completion layer could not run");
                return Err(EXIT_RUNTIME_ERROR);
            }
            return Ok(podbox_complete::Report::default());
        }
    };
    banner.push_str(&report.banner());
    Ok(report)
}

/// T-0804's refusal, over all three inputs.
///
/// ⚠ Called after the banner is built and before the payload starts, so a
/// caller that is refused still gets the whole account of why.
///
/// ⭐ TODO/cli.md T-1415. Kind decides, count no longer does: each reason
/// carries its class (see [`classify_strict`]). `--strict` refuses the
/// safety class and warns on the substitution class; `--strict=all` keeps
/// the T-0804 behavior and refuses every reason.
pub fn strict_refusal(
    verb: &str,
    ask: &Ask,
    rung: &str,
    report: &podbox_complete::Report,
    err: &mut dyn Write,
    ctx: &StrictCtx,
) -> Result<(), i32> {
    if !ask.strict {
        return Ok(());
    }
    let reasons = classify_strict(verb, ask, rung, report, ctx);
    if reasons.is_empty() {
        return Ok(());
    }
    if ask.strict_all {
        let _ = writeln!(
            err,
            "podbox {verb}: --strict=all, and this run is degraded in {} way(s). \
             podbox refuses rather than running and letting the payload discover \
             them (TODO/cli.md T-0804, T-1415):",
            reasons.len()
        );
        for (kind, r) in &reasons {
            let _ = writeln!(err, "  - [{}] {r}", kind.word());
        }
        return Err(EXIT_RUNTIME_ERROR);
    }
    let (safety, substitution): (Vec<_>, Vec<_>) = reasons
        .into_iter()
        .partition(|(kind, _)| *kind == StrictKind::Safety);
    for (_, r) in &substitution {
        let _ = writeln!(err, "podbox {verb}: --strict warning [substitution]: {r}");
    }
    if safety.is_empty() {
        return Ok(());
    }
    let _ = writeln!(
        err,
        "podbox {verb}: --strict, and this run is degraded in {} safety-relevant \
         way(s). podbox refuses rather than running and letting the payload \
         discover them (TODO/cli.md T-0804, T-1415):",
        safety.len()
    );
    for (_, r) in &safety {
        let _ = writeln!(err, "  - [safety] {r}");
    }
    Err(EXIT_RUNTIME_ERROR)
}

/// What `--strict` classifies on besides the three T-0804 inputs.
#[derive(Debug, Clone, Default)]
pub struct StrictCtx {
    /// False where chroot is denied: the userland rung is then the best
    /// available rung, not a rung declined, and the rung difference warns
    /// rather than refuses.
    pub chroot_usable: bool,
    /// True where `--unsafe-host-paths` opted out of path virtualization
    /// (TODO/enter.md T-1407): a chosen host write, and safety-relevant.
    pub unsafe_host_paths: bool,
    /// True where a caller preload was dropped on the userland rung: guest
    /// paths in it would resolve on the host, which is host and libc
    /// mixing, and safety-relevant.
    pub preload_dropped: bool,
}

/// One `--strict` reason's class. The refusal line names it, so a caller
/// can tell a danger from a stand-in without parsing the banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictKind {
    /// No path virtualization, host and libc mixing, escape: still
    /// refuses under `--strict`.
    Safety,
    /// A dev-shim substitution and its kin: warns under `--strict`,
    /// refuses under `--strict=all`.
    Substitution,
}

impl StrictKind {
    pub fn word(self) -> &'static str {
        match self {
            StrictKind::Safety => "safety",
            StrictKind::Substitution => "substitution",
        }
    }
}

/// Classify every `--strict` reason by kind.
///
/// Safety (refuses): a Degraded or Stub flag the caller passed; a rung
/// below the floor where a better rung existed; a fixup podbox tried and
/// could not (`Failed`: escape-shaped or otherwise unprepared); a step
/// podbox would run inside the image; the `--unsafe-host-paths` opt-in;
/// a dropped caller preload. Substitution (warns): a stand-in podbox put
/// where the real thing cannot be made (dev-shim files and their kin);
/// the userland rung where chroot is denied and nothing better exists.
pub fn classify_strict(
    verb: &str,
    ask: &Ask,
    rung: &str,
    report: &podbox_complete::Report,
    ctx: &StrictCtx,
) -> Vec<(StrictKind, String)> {
    use StrictKind::{Safety, Substitution};
    let mut reasons: Vec<(StrictKind, String)> = Vec::new();

    // 1. the flags this invocation passed.
    for f in &ask.seen_flags {
        let Some(row) = crate::parity::flag(verb, f) else {
            continue;
        };
        if matches!(
            row.status,
            crate::parity::Status::Degraded | crate::parity::Status::Stub
        ) {
            reasons.push((
                Safety,
                format!(
                    "{} is {} in the parity table: {}",
                    row.flag.unwrap_or(f),
                    row.status.word(),
                    row.note
                ),
            ));
        }
    }

    // 2. the rung. ⛔ The floor is `Selection`'s own, not a word repeated here:
    // `podbox probe --strict` gates on the same constant, and two spellings of
    // one floor is the drift T-1206 is about.
    let floor = podbox_probe::select::Selection::STRICT_FLOOR.word();
    if rung != floor {
        // ⭐ T-1415. Where chroot is denied the userland rung is the best
        // the machine does: warn, so the callers who want the gate most
        // still get a run. Where a better rung existed, refuse.
        let best_available =
            rung == podbox_probe::select::Rung::Userland.word() && !ctx.chroot_usable;
        let text = format!(
            "the selected rung is `{rung}` and not `{floor}`, so the payload shares \
             this machine's process table, network, IPC and mount namespaces"
        );
        reasons.push((if best_available { Substitution } else { Safety }, text));
    }

    // 3. the completion layer: what podbox could not do refuses; a
    // stand-in it put warns.
    for f in report.degradations() {
        let text = format!(
            "{} {} ({}): {}",
            f.action.word(),
            if f.path.is_empty() { "-" } else { &f.path },
            f.entry,
            f.detail
        );
        let kind = if f.action == podbox_complete::Action::Failed {
            Safety
        } else {
            Substitution
        };
        reasons.push((kind, text));
    }

    // 4. ⭐ T-0412's steps, and they are counted BEFORE any of them runs. podbox
    // executing a command inside somebody else's image that the caller did not
    // write is exactly the kind of difference from docker `--strict` exists to
    // refuse, and refusing after running one would be podbox acting and then
    // declining to have acted.
    for s in &report.steps {
        reasons.push((
            Safety,
            format!(
                "podbox would run `{}` ({}) inside this image before the payload: {}",
                s.argv.join(" "),
                s.entry,
                s.why
            ),
        ));
    }

    // 5. ⭐ T-1415. The userland opt-outs: no path virtualization by
    // choice, and a dropped preload that would mix host and guest libc.
    if ctx.unsafe_host_paths {
        reasons.push((
            Safety,
            "--unsafe-host-paths opts out of path virtualization: absolute guest \
             paths resolve on the host, and a write lands on the host tree \
             (TODO/enter.md T-1407)"
                .to_string(),
        ));
    }
    if ctx.preload_dropped {
        reasons.push((
            Safety,
            "a caller LD_PRELOAD names guest paths, which resolve on the host \
             without a chroot: host and guest libc mix (TODO/enter.md T-1317)"
                .to_string(),
        ));
    }

    reasons
}

/// The bound one step gets.
///
/// ⛔ **A bound rather than a wait**, because `AGENTS.md` makes that a
/// requirement of podbox and not only of the agent working on it: a step is a
/// program from somebody else's image and podbox has no idea what it does.
/// ⚠ Five minutes because `pacman-key --populate` builds a keyring with `gpg`
/// and `openssl rehash` reads every file in a directory of six hundred; both
/// are seconds here and neither has a documented worst case.
pub const STEP_TIMEOUT_MS: i64 = 300_000;

/// ⭐ **T-0412. Run the report's steps INSIDE the rootfs, before the payload.**
///
/// The completion layer runs on the host and every fixup it can make is a
/// write. Two are not -- `pacman-key --init` runs `gpg` in the rootfs, and a
/// hash-indexed CApath is indexed by a program that reads the certificates --
/// so the layer returns an argv and this runs it.
///
/// ⛔ **The banner has already named every one of these**, which is why this is
/// the caller's job rather than the library's: the announcement has to precede
/// the command, and only the caller knows what else it is about to print.
///
/// ⛔ **A step that fails is a `Failed` row and never a failed run.** The
/// payload may not need what the step would have provided, and refusing here
/// would make podbox less useful than the bare `chroot` it replaces. `--strict`
/// is how a caller turns it into a refusal, and it refuses before any step runs.
///
/// ⚠ The step's stdout is redirected to podbox's STDERR. T-1104: the payload
/// owns stdout, and a step's output on it would corrupt every pipeline
/// `podbox run <image> cmd | consumer` is in.
///
/// T-0412 steps sharing the entry's host memo, for T-0710.
///
/// `memo_host` is the host's own number for the file the environment already
/// names at `MEMO_CHILD_FD`: steps are payload processes in the same rootfs,
/// so a `chown` they virtualize must land in the same record the payload
/// reads. `None` runs them without a memo, as before where no tier loads.
pub fn run_steps_with_memo(
    verb: &str,
    rootfs: &str,
    env: &[String],
    report: &mut podbox_complete::Report,
    quiet: bool,
    err: &mut dyn Write,
    memo_host: Option<i64>,
) {
    if report.steps.is_empty() {
        return;
    }
    let steps = report.steps.clone();
    let root = match podbox_enter::RootDir::open(rootfs) {
        Ok(r) => r,
        Err(e) => {
            report.fixups.push(failed_step(
                steps[0].entry,
                steps[0].id,
                format!("podbox could not open the rootfs to run its steps in: {e}"),
            ));
            let _ = writeln!(err, "podbox {verb}: complete: {e}");
            return;
        }
    };
    for (i, s) in steps.iter().enumerate() {
        let started = std::time::Instant::now();
        let outcome = one_step(&root, rootfs, s, env, memo_host);
        let took = started.elapsed().as_secs_f64();
        let (line, failure) = match outcome {
            Ok(podbox_enter::Bounded::Exited(0)) => (
                format!(
                    "podbox: complete: step {} of {}: `{}` exited 0 in {took:.1} s",
                    i + 1,
                    steps.len(),
                    s.argv.join(" ")
                ),
                None,
            ),
            Ok(podbox_enter::Bounded::Exited(c)) => (
                format!(
                    "podbox: complete: step {} of {}: `{}` exited {c} in {took:.1} s. \
                     note: podbox does NOT fail the run for it: the payload may not need \
                     what it would have done",
                    i + 1,
                    steps.len(),
                    s.argv.join(" ")
                ),
                Some(format!("`{}` exited {c}", s.argv.join(" "))),
            ),
            Ok(podbox_enter::Bounded::TimedOut { after_ms }) => (
                format!(
                    "podbox: complete: step {} of {}: `{}` was still running after \
                     {} s and podbox killed it",
                    i + 1,
                    steps.len(),
                    s.argv.join(" "),
                    after_ms / 1000
                ),
                Some(format!(
                    "`{}` did not finish within {} s and was killed",
                    s.argv.join(" "),
                    after_ms / 1000
                )),
            ),
            Err(e) => (
                format!(
                    "podbox: complete: step {} of {}: `{}` could not be started: {e}",
                    i + 1,
                    steps.len(),
                    s.argv.join(" ")
                ),
                Some(format!("`{}` could not be started: {e}", s.argv.join(" "))),
            ),
        };
        if !quiet {
            let _ = writeln!(err, "{line}");
        }
        report.fixups.push(match failure {
            None => podbox_complete::Fixup {
                entry: s.entry,
                id: s.id,
                path: String::new(),
                action: podbox_complete::Action::Ran,
                detail: format!("`{}` exited 0. {}", s.argv.join(" "), s.why),
                degraded: false,
            },
            Some(why) => failed_step(s.entry, s.id, why),
        });
    }
}

fn failed_step(entry: &'static str, id: &'static str, why: String) -> podbox_complete::Fixup {
    podbox_complete::Fixup {
        entry,
        id,
        path: String::new(),
        action: podbox_complete::Action::Failed,
        detail: why,
        degraded: true,
    }
}

/// One step, entered exactly as the payload is.
///
/// ⛔ The same `podbox_enter` sequence and not a second one: a step that
/// resolved its program in the parent, or entered by a path rather than by the
/// descriptor T-0504 holds, would be a quieter entry path with none of the
/// guarantees the loud one has.
fn one_step(
    root: &podbox_enter::RootDir,
    rootfs: &str,
    s: &podbox_complete::Step,
    env: &[String],
    memo_host: Option<i64>,
) -> Result<podbox_enter::Bounded, podbox_enter::Error> {
    // ⛔ The step's stdout becomes podbox's stderr, and it is DUPLICATED first:
    // handing `(1, 2)` to the child's `dup2`-then-close loop would close the
    // child's own stderr with it.
    let mirror = podbox_probe::sys::dup_cloexec(2)
        .map_err(|e| podbox_enter::Error::Runtime(format!("dup of stderr: {}", e.name())))?;
    let mut pass = vec![(1, mirror)];
    // ⭐ T-0710: steps share the entry's memo, where one was handed. Without
    // it a fixup's `chown` would fail real where the payload's succeeds
    // virtualized, and the two would disagree about who owns the file.
    if let Some(host) = memo_host {
        pass.push((podbox_supervise::table::MEMO_CHILD_FD, host));
    }
    // ⭐ TODO/complete.md T-0413: a step is its own exec with its own
    // `/proc/self/exe`, so it resolves its own guest path rather than
    // inheriting the payload's. A step name that resolves nothing sets
    // nothing: exact or refused, like every other answer.
    let mut step_env = env.to_vec();
    crate::run::push_guest_exe(
        &mut step_env,
        rootfs,
        s.argv.first().map(String::as_str).unwrap_or(""),
        &podbox_enter::Plan::path_from(env),
    );
    let plan = podbox_enter::Plan {
        argv: s.argv.clone(),
        env: step_env,
        working_dir: "/".to_string(),
        fds: podbox_enter::Fds { pass },
        // ⚠ Empty: the banner named this step before podbox got here.
        banner: String::new(),
        path_dirs: Vec::new(),
    };
    let mut sink = std::io::sink();
    let spawned = podbox_enter::spawn(root, &plan, &mut sink);
    // ⚠ The parent's copy goes here whatever happened: the child got its own.
    let _ = podbox_probe::sys::close(mirror);
    spawned?.wait_bounded(STEP_TIMEOUT_MS)
}

/// Parse `--add-host name:ip` and friends out of the shared argument surface.
///
/// ⚠ Returned as an error code rather than a panic so the caller's own usage
/// text is what the user sees.
pub fn add_host(ask: &mut Ask, verb: &str, value: &str) -> Result<(), i32> {
    if !value.contains(':') {
        eprintln!("podbox {verb}: --add-host takes name:ip, not {value:?}");
        return Err(EXIT_FLAG_ERROR);
    }
    ask.add_hosts.push(value.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_with_degradation() -> podbox_complete::Report {
        let mut r = podbox_complete::Report::default();
        r.fixups.push(podbox_complete::Fixup {
            entry: "T-0401",
            id: "dev-shim",
            path: "dev/null".into(),
            action: podbox_complete::Action::Created,
            detail: "a regular file".into(),
            degraded: true,
        });
        r
    }

    /// ⛔ Without `--strict` a degraded run is a run. The banner is the report,
    /// and refusing by default would make podbox useless on the machines it
    /// exists for.
    #[test]
    fn without_strict_nothing_is_refused() {
        let ask = Ask::default();
        let mut out = Vec::new();
        assert!(strict_refusal(
            "run",
            &ask,
            "chroot",
            &report_with_degradation(),
            &mut out,
            &StrictCtx::default()
        )
        .is_ok());
        assert!(out.is_empty());
    }

    /// ⭐ With `--strict=all`, all three inputs are named in one refusal
    /// rather than the first one found, exactly the T-0804 behavior, and
    /// every line names its class.
    #[test]
    fn strict_all_names_every_reason_at_once() {
        let ask = Ask {
            strict: true,
            strict_all: true,
            seen_flags: vec!["-i".into(), "--rm".into()],
            ..Ask::default()
        };
        let mut out = Vec::new();
        let e = strict_refusal(
            "run",
            &ask,
            "chroot",
            &report_with_degradation(),
            &mut out,
            &StrictCtx {
                chroot_usable: true,
                ..StrictCtx::default()
            },
        )
        .unwrap_err();
        assert_eq!(e, EXIT_RUNTIME_ERROR);
        let s = String::from_utf8(out).unwrap();
        // the Stub flag
        assert!(s.contains("-i, --interactive"), "{s}");
        // the rung
        assert!(s.contains("`chroot`"), "{s}");
        // the fixup
        assert!(s.contains("dev/null"), "{s}");
        // ⚠ and NOT the Native one.
        assert!(!s.contains("--rm is"), "{s}");
        assert!(s.contains("3 way(s)"), "{s}");
        assert!(s.contains("[safety]"), "{s}");
        assert!(s.contains("[substitution]"), "{s}");
    }

    /// ⭐ TODO/cli.md T-1415. Kind decides: on a chroot-denied host the
    /// userland rung and the dev-shim substitutions warn and the run
    /// proceeds, while a path-virtualization opt-out and a failed fixup
    /// still refuse, each line naming its class.
    #[test]
    fn strict_classifies_safety_against_substitution() {
        let userland = podbox_probe::select::Rung::Userland.word();
        let denied = StrictCtx {
            chroot_usable: false,
            ..StrictCtx::default()
        };
        // The best available rung plus a dev-shim: warnings, exit 0.
        let ask = Ask {
            strict: true,
            ..Ask::default()
        };
        let mut out = Vec::new();
        assert!(strict_refusal(
            "run",
            &ask,
            userland,
            &report_with_degradation(),
            &mut out,
            &denied
        )
        .is_ok());
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("--strict warning [substitution]"), "{s}");
        // The same run with the pass-through opted in: refused, safety.
        let unsafe_ctx = StrictCtx {
            chroot_usable: false,
            unsafe_host_paths: true,
            ..StrictCtx::default()
        };
        let mut out = Vec::new();
        let e = strict_refusal(
            "run",
            &ask,
            userland,
            &report_with_degradation(),
            &mut out,
            &unsafe_ctx,
        )
        .unwrap_err();
        assert_eq!(e, EXIT_RUNTIME_ERROR);
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("[safety]"), "{s}");
        assert!(s.contains("--unsafe-host-paths"), "{s}");
        // A fixup podbox tried and could not: refused even where the rung
        // is the best available.
        let mut failed = podbox_complete::Report::default();
        failed.fixups.push(podbox_complete::Fixup {
            entry: "T-0401",
            id: "dev-shim",
            path: "dev/null".into(),
            action: podbox_complete::Action::Failed,
            detail: "a directory".into(),
            degraded: true,
        });
        let mut out = Vec::new();
        let e = strict_refusal("run", &ask, userland, &failed, &mut out, &denied).unwrap_err();
        assert_eq!(e, EXIT_RUNTIME_ERROR);
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("[safety]"), "{s}");
        // And `--strict=all` refuses the warned run too.
        let ask = Ask {
            strict: true,
            strict_all: true,
            ..Ask::default()
        };
        let mut out = Vec::new();
        let e = strict_refusal(
            "run",
            &ask,
            userland,
            &report_with_degradation(),
            &mut out,
            &denied,
        )
        .unwrap_err();
        assert_eq!(e, EXIT_RUNTIME_ERROR);
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("--strict=all"), "{s}");
    }

    /// ⭐ T-0412. A step is a reason on its own, and it is counted BEFORE any of
    /// them runs: podbox executing a command inside somebody else's image that
    /// the caller did not write is a difference from docker, and refusing after
    /// running one would be podbox acting and then declining to have acted.
    #[test]
    fn strict_refuses_a_run_whose_only_difference_is_a_step() {
        let mut report = podbox_complete::Report::default();
        report.steps.push(podbox_complete::Step {
            entry: "T-0412",
            id: "ca-hash-dir",
            argv: vec![
                "/usr/bin/openssl".into(),
                "rehash".into(),
                "/etc/ssl/certs".into(),
            ],
            why: "libzypp reads this directory and no CAfile at all".into(),
        });
        let ask = Ask {
            strict: true,
            ..Ask::default()
        };
        let floor = podbox_probe::select::Selection::STRICT_FLOOR.word();
        let mut out = Vec::new();
        let e = strict_refusal("run", &ask, floor, &report, &mut out, &StrictCtx::default())
            .unwrap_err();
        assert_eq!(e, EXIT_RUNTIME_ERROR);
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("/usr/bin/openssl rehash /etc/ssl/certs"), "{s}");
        assert!(s.contains("1 safety-relevant way(s)"), "{s}");
    }

    /// ⚠ A run with nothing degraded passes `--strict`, or the flag would be a
    /// refusal rather than a gate.
    #[test]
    fn strict_passes_a_run_with_no_degradation() {
        let ask = Ask {
            strict: true,
            seen_flags: vec!["--rm".into()],
            ..Ask::default()
        };
        let floor = podbox_probe::select::Selection::STRICT_FLOOR.word();
        let mut out = Vec::new();
        assert!(strict_refusal(
            "run",
            &ask,
            floor,
            &podbox_complete::Report::default(),
            &mut out,
            &StrictCtx::default(),
        )
        .is_ok());
    }

    /// ⭐ TODO/cli.md T-1416. Quiet by default, the banner under
    /// `--verbose`, `-q` forcing it off, the store config still forcing
    /// it off: the operator ruling reversing T-0804's command-line rule
    /// in writing. Refusals never pass through here, so nothing about
    /// them is asserted here.
    #[test]
    fn the_banner_is_quiet_by_default_and_loud_under_verbose() {
        let d = std::env::temp_dir().join(format!("podbox-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::env::set_var("PODBOX_STORE", &d);
        let s = podbox_image::open_store().unwrap();
        // Default: quiet. The banner config key still reads as before.
        assert!(!banner_quiet(&s));
        assert!(!banner_loud(&s, &Ask::default()));
        // `--verbose` prints it.
        let verbose = Ask {
            verbose: true,
            ..Ask::default()
        };
        assert!(banner_loud(&s, &verbose));
        // `-q` forces it off, even beside `--verbose`.
        let off = Ask {
            verbose: true,
            quiet: true,
            ..Ask::default()
        };
        assert!(!banner_loud(&s, &off));
        // The store config forces it off under `--verbose` too.
        std::fs::write(s.root().join(CONFIG_FILE), "# a comment\nbanner = quiet\n").unwrap();
        assert!(banner_quiet(&s));
        assert!(!banner_loud(&s, &verbose));
        std::env::remove_var("PODBOX_STORE");
        let _ = std::fs::remove_dir_all(&d);
    }
}
