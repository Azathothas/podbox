use super::*;

/// T-0804's refusal, over all three inputs.
///
/// ⚠ Called after the banner is built and before the payload starts, so a
/// caller that is refused still gets the whole account of why.
///
/// ⭐ TODO/cli.md T-1415. Kind decides, count no longer does: each reason
/// carries its class (see [`classify_strict`]). `--strict` refuses the
/// safety class and warns on the substitution class; `--strict=all` keeps
/// the T-0804 behavior and refuses every reason.
///
/// ⭐ TODO/cli.md T-1650. `--strict` refuses a capability gap that the
/// payload hits and warns on a gap where podvm cannot tell.
pub fn strict_refusal(
    verb: &str,
    ask: &Ask,
    rung: &str,
    report: &podvm_complete::Report,
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
            "podvm {verb}: --strict=all, and this run is degraded in {} way(s). \
             podvm refuses rather than running and letting the payload discover \
             them (TODO/cli.md T-0804, T-1415, T-1650):",
            reasons.len()
        );
        for (kind, r) in &reasons {
            let _ = writeln!(err, "  - [{}] {r}", kind.word());
        }
        return Err(EXIT_RUNTIME_ERROR);
    }
    let (safety, warned): (Vec<_>, Vec<_>) =
        reasons.into_iter().partition(|(kind, _)| kind.refuses());
    for (kind, r) in &warned {
        let _ = writeln!(err, "podvm {verb}: --strict warning [{}]: {r}", kind.word());
    }
    if safety.is_empty() {
        return Ok(());
    }
    let _ = writeln!(
        err,
        "podvm {verb}: --strict, and this run is degraded in {} safety-relevant \
         way(s). podvm refuses rather than running and letting the payload \
         discover them (TODO/cli.md T-0804, T-1415, T-1650):",
        safety.len()
    );
    for (kind, r) in &safety {
        let _ = writeln!(err, "  - [{}] {r}", kind.word());
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
    /// TODO/cli.md T-1650. The capability gaps of the selected family for
    /// this payload (`Userland::gaps`); empty on the chroot rung.
    pub gaps: Vec<Gap>,
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
    /// TODO/cli.md T-1650. A gap of the selected family: a thing that the
    /// payload cannot do on this rung. `hit` where podvm can tell that the
    /// payload hits it: that refuses under `--strict`, because it decides
    /// whether the command runs at all. Else it warns.
    Capability { hit: bool },
}

impl StrictKind {
    pub fn word(self) -> &'static str {
        match self {
            StrictKind::Safety => "safety",
            StrictKind::Substitution => "substitution",
            StrictKind::Capability { .. } => "capability",
        }
    }

    /// Whether `--strict` refuses this kind (`--strict=all` refuses each).
    pub fn refuses(self) -> bool {
        matches!(
            self,
            StrictKind::Safety | StrictKind::Capability { hit: true }
        )
    }
}

/// Classify every `--strict` reason by kind.
///
/// Safety (refuses): a Degraded or Stub flag the caller passed; a rung
/// below the floor where a better rung existed; a fixup podvm tried and
/// could not (`Failed`: escape-shaped or otherwise unprepared); a step
/// podvm would run inside the image; the `--unsafe-host-paths` opt-in;
/// a dropped caller preload. Substitution (warns): a stand-in podvm put
/// where the real thing cannot be made (dev-shim files and their kin);
/// the userland rung where chroot is denied and nothing better exists.
/// Capability (T-1650): each gap of the selected family, which refuses
/// where the payload hits it and warns elsewhere.
pub fn classify_strict(
    verb: &str,
    ask: &Ask,
    rung: &str,
    report: &podvm_complete::Report,
    ctx: &StrictCtx,
) -> Vec<(StrictKind, String)> {
    use StrictKind::{Capability, Safety, Substitution};
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
    // `podvm probe --strict` gates on the same constant, and two spellings of
    // one floor is the drift T-1206 is about.
    let floor = podvm_probe::select::Selection::STRICT_FLOOR.word();
    if rung != floor {
        // ⭐ T-1415. Where chroot is denied the userland rung is the best
        // the machine does: warn, so the callers who want the gate most
        // still get a run. Where a better rung existed, refuse.
        let best_available =
            rung == podvm_probe::select::Rung::Userland.word() && !ctx.chroot_usable;
        let text = format!(
            "the selected rung is `{rung}` and not `{floor}`, so the payload shares \
             this machine's process table, network, IPC and mount namespaces"
        );
        reasons.push((if best_available { Substitution } else { Safety }, text));
    }

    // 3. the completion layer: what podvm could not do refuses; a
    // stand-in it put warns.
    for f in report.degradations() {
        let text = format!(
            "{} {} ({}): {}",
            f.action.word(),
            if f.path.is_empty() { "-" } else { &f.path },
            f.entry,
            f.detail
        );
        let kind = if f.action == podvm_complete::Action::Failed {
            Safety
        } else {
            Substitution
        };
        reasons.push((kind, text));
    }

    // 4. ⭐ T-0412's steps, and they are counted BEFORE any of them runs. podvm
    // executing a command inside somebody else's image that the caller did not
    // write is exactly the kind of difference from docker `--strict` exists to
    // refuse, and refusing after running one would be podvm acting and then
    // declining to have acted.
    for s in &report.steps {
        reasons.push((
            Safety,
            format!(
                "podvm would run `{}` ({}) inside this image before the payload: {}",
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

    // 6. ⭐ T-1650. The capability gaps of the selected family. A gap that
    // the payload hits names a rung that runs the command.
    for g in &ctx.gaps {
        let text = match &g.hit {
            None => g.text.to_string(),
            Some(why) => format!(
                "{}. This command hits it: {why}. The chroot rung runs this \
                 command on a host that permits chroot(2); this host has no \
                 rung that runs it yet (TODO/enter.md T-1657)",
                g.text
            ),
        };
        let hit = g.hit.is_some();
        reasons.push((Capability { hit }, text));
    }

    reasons
}

/// The bound one step gets.
///
/// ⛔ **A bound rather than a wait**, because `AGENTS.md` makes that a
/// requirement of podvm and not only of the agent working on it: a step is a
/// program from somebody else's image and podvm has no idea what it does.
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
/// would make podvm less useful than the bare `chroot` it replaces. `--strict`
/// is how a caller turns it into a refusal, and it refuses before any step runs.
///
/// ⚠ The step's stdout is redirected to podvm's STDERR. T-1104: the payload
/// owns stdout, and a step's output on it would corrupt every pipeline
/// `podvm run <image> cmd | consumer` is in.
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
    report: &mut podvm_complete::Report,
    quiet: bool,
    err: &mut dyn Write,
    memo_host: Option<i64>,
) {
    if report.steps.is_empty() {
        return;
    }
    let steps = report.steps.clone();
    let root = match podvm_enter::RootDir::open(rootfs) {
        Ok(r) => r,
        Err(e) => {
            report.fixups.push(failed_step(
                steps[0].entry,
                steps[0].id,
                format!("podvm could not open the rootfs to run its steps in: {e}"),
            ));
            let _ = writeln!(err, "podvm {verb}: complete: {e}");
            return;
        }
    };
    for (i, s) in steps.iter().enumerate() {
        let started = std::time::Instant::now();
        let outcome = one_step(&root, rootfs, s, env, memo_host);
        let took = started.elapsed().as_secs_f64();
        let (line, failure) = match outcome {
            Ok(podvm_enter::Bounded::Exited(0)) => (
                format!(
                    "podvm: complete: step {} of {}: `{}` exited 0 in {took:.1} s",
                    i + 1,
                    steps.len(),
                    s.argv.join(" ")
                ),
                None,
            ),
            Ok(podvm_enter::Bounded::Exited(c)) => (
                format!(
                    "podvm: complete: step {} of {}: `{}` exited {c} in {took:.1} s. \
                     note: podvm does NOT fail the run for it: the payload may not need \
                     what it would have done",
                    i + 1,
                    steps.len(),
                    s.argv.join(" ")
                ),
                Some(format!("`{}` exited {c}", s.argv.join(" "))),
            ),
            Ok(podvm_enter::Bounded::TimedOut { after_ms }) => (
                format!(
                    "podvm: complete: step {} of {}: `{}` was still running after \
                     {} s and podvm killed it",
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
                    "podvm: complete: step {} of {}: `{}` could not be started: {e}",
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
            None => podvm_complete::Fixup {
                entry: s.entry,
                id: s.id,
                path: String::new(),
                action: podvm_complete::Action::Ran,
                detail: format!("`{}` exited 0. {}", s.argv.join(" "), s.why),
                degraded: false,
            },
            Some(why) => failed_step(s.entry, s.id, why),
        });
    }
}

fn failed_step(entry: &'static str, id: &'static str, why: String) -> podvm_complete::Fixup {
    podvm_complete::Fixup {
        entry,
        id,
        path: String::new(),
        action: podvm_complete::Action::Failed,
        detail: why,
        degraded: true,
    }
}

/// One step, entered exactly as the payload is.
///
/// ⛔ The same `podvm_enter` sequence and not a second one: a step that
/// resolved its program in the parent, or entered by a path rather than by the
/// descriptor T-0504 holds, would be a quieter entry path with none of the
/// guarantees the loud one has.
fn one_step(
    root: &podvm_enter::RootDir,
    rootfs: &str,
    s: &podvm_complete::Step,
    env: &[String],
    memo_host: Option<i64>,
) -> Result<podvm_enter::Bounded, podvm_enter::Error> {
    // ⛔ The step's stdout becomes podvm's stderr, and it is DUPLICATED first:
    // handing `(1, 2)` to the child's `dup2`-then-close loop would close the
    // child's own stderr with it.
    let mirror = podvm_probe::sys::dup_cloexec(2)
        .map_err(|e| podvm_enter::runtime(format!("dup of stderr: {}", e.name())))?;
    let mut pass = vec![(1, mirror)];
    // ⭐ T-0710: steps share the entry's memo, where one was handed. Without
    // it a fixup's `chown` would fail real where the payload's succeeds
    // virtualized, and the two would disagree about who owns the file.
    if let Some(host) = memo_host {
        pass.push((podvm_supervise::table::MEMO_CHILD_FD, host));
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
        &podvm_enter::Plan::path_from(env),
    );
    let plan = podvm_enter::Plan {
        argv: s.argv.clone(),
        env: step_env,
        working_dir: "/".to_string(),
        fds: podvm_enter::Fds { pass },
        // ⚠ Empty: the banner named this step before podvm got here.
        banner: String::new(),
        path_dirs: Vec::new(),
        names: podvm_enter::NAMES,
        limits: Vec::new(),
        no_new_privs: false,
    };
    let mut sink = std::io::sink();
    let spawned = podvm_enter::spawn(root, &plan, &mut sink);
    // ⚠ The parent's copy goes here whatever happened: the child got its own.
    let _ = podvm_probe::sys::close(mirror);
    spawned?.wait_bounded(STEP_TIMEOUT_MS)
}

/// Parse `--add-host name:ip` and friends out of the shared argument surface.
///
/// ⚠ Returned as an error code rather than a panic so the caller's own usage
/// text is what the user sees.
pub fn add_host(ask: &mut Ask, verb: &str, value: &str) -> Result<(), i32> {
    if !value.contains(':') {
        eprintln!("podvm {verb}: --add-host takes name:ip, not {value:?}");
        return Err(EXIT_FLAG_ERROR);
    }
    ask.add_hosts.push(value.to_string());
    Ok(())
}
