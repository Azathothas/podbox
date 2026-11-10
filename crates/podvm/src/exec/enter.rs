use super::*;
use podvm_enter::ExitCode as _;

/// Open the device spec where one rides the plan's environment, T-0501.
///
/// `exec` takes no `--device` flag (docker parity: it re-enters), but a
/// container created with mappings carries the spec in its record, and a
/// re-entry that dropped it would serve nothing where `run` served. A
/// spec gone bad since `create` refuses the exec naming it.
pub(super) fn serve_devices(plan: &mut Plan, err: &mut dyn Write) -> std::result::Result<(), i32> {
    match podvm_enter::device::specs_of(&plan.env).and_then(|s| podvm_enter::device::open_all(&s)) {
        Ok(opened) => {
            plan.fds.pass.extend(opened.pass.iter().copied());
            podvm_enter::device::push_serve(&mut plan.env, &opened);
            Ok(())
        }
        Err(e) => {
            let _ = writeln!(err, "podvm exec: {e}");
            Err(e.exit_code())
        }
    }
}

/// Enter an existing rootfs. ⭐ ONE PATH for a container and for an image: the
/// only difference is where the rootfs came from, and duplicating the entry
/// would be a second implementation of the thing this verb exists to be.
pub(super) fn enter(
    target: &str,
    rootfs: &str,
    state: podvm_supervise::table::State,
    o: &Opts,
    store: &podvm_image::Store,
) -> i32 {
    use podvm_supervise::table::State;
    if state == State::Created {
        eprintln!(
            "podvm exec: {target} has been created and never started, so its rootfs              is there and nothing is running in it. note: podvm will enter it anyway,              because a fresh chroot shares only the filesystem and needs nothing to              be running (TODO/enter.md T-0505)"
        );
    }
    // ⚠ The container's own environment is not inherited: T-0505's whole
    // subject is that this shares the filesystem and NOTHING else, and silently
    // copying the original's environment would be the implication it refuses.
    let mut env = podvm_enter::Plan::env_for(&podvm_enter::NAMES, &[], &o.env);
    // ⭐ T-0711: the requested identity, resolved against the re-entered
    // rootfs exactly as `run` resolves it against the extracted one.
    if let Err(code) = crate::lifecycle::apply_user("exec", rootfs, o.user.as_ref(), &mut env) {
        return code;
    }
    // ⭐ T-0505's record, read once: the memo below, the device spec, the
    // container's own image name for the workdir refusal, and the image
    // record the entry decision reads the architecture from.
    let container = match podvm_supervise::get(store, target) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("podvm exec: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    // ⭐ T-0710: the container's host memo, re-handed on this fresh re-entry.
    // A fresh chroot shares only the filesystem; without the same descriptor
    // a second process answers `stat` with the real uid and contradicts the
    // first, so an entry that cannot be handed one refuses.
    let memo_path = podvm_supervise::table::memo_path(store, &container.id);
    if let Err(e) = podvm_supervise::table::ensure_memo_file(&memo_path) {
        eprintln!("podvm exec: the ownership memo could not be created: {e}");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != podvm_supervise::table::MEMO_FD_VAR);
    env.push(podvm_supervise::table::memo_fd_env());
    // ⭐ TODO/enter.md T-0501. The container's device spec is
    // configuration, not environment: T-0505 refuses the original's
    // environment on a re-entry, but the devices are the container's
    // (docker's `exec` sees them), so the spec is read from the record
    // and opened fresh here rather than inherited. A spec gone bad
    // since `create` refuses the exec naming it.
    let exec_devices = match podvm_enter::device::specs_of(&container.env) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm exec: {e}");
            return e.exit_code();
        }
    };
    podvm_enter::device::push_spec(&mut env, &exec_devices);
    let findings = podvm_probe::run();
    // ⭐ TODO/enter.md T-1410. The image record behind the container, read
    // for the architecture the entry decision needs: a refusal here names
    // the lookup, because without it no rung can be decided safely.
    let record = match store.find_one(&container.image) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("podvm exec: {target}: the image record cannot be read: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    // ⭐ TODO/milestones.md T-1112, as on the image path below and in
    // `run`'s `prepare`: the OS gate sits on every entry path.
    if let Err(code) =
        crate::lifecycle::ensure_linux_guest("exec", &record.os, &record.architecture)
    {
        return code;
    }
    let host = Platform::host();
    let support_native = matches!(
        podvm_enter::binfmt::support_for(&record.architecture, &host.arch),
        podvm_enter::binfmt::Support::Native
    );
    // ⭐ TODO/enter.md T-1410. `exec` and `run` share the entry decision:
    // where chroot is denied the loader and memfd families drive, with
    // T-0505's fresh-entry contract unchanged (separate banners, separate
    // records). A refusal names the blocking call.
    let userland = match crate::lifecycle::decide_entry(
        "exec",
        rootfs,
        &o.command,
        &env,
        support_native,
        &findings,
        o.unsafe_host_paths,
        None,
    ) {
        Ok(u) => u,
        Err(code) => return code,
    };
    // ⭐ T-0702 and T-0706, as in `run`: a fresh chroot re-entry is a fresh
    // payload, so it is classified and placed again rather than inheriting
    // the first entry's answer. After the decision, so a refusal writes
    // nothing; T-1649: a `#!` payload on the userland rung classifies its
    // interpreter.
    let gargv = userland.as_ref().map_or(&o.command, |u| &u.guest_argv);
    let mut interpose_note = String::new();
    if let Err(e) = crate::interpose::apply("exec", rootfs, gargv, &mut env, &mut interpose_note) {
        eprintln!("{e}");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⭐ The loader transform, mirroring `run`'s `prepare`: without a
    // chroot the loader resolves guest paths on the host root, so the
    // preload becomes the host path and the libraries come from the
    // image. A caller preload naming guest paths is dropped and named.
    let mut dropped_preload: Option<String> = None;
    if let Some(u) = &userland
        && u.family == podvm_enter::userland::Family::Loader
    {
        let (hosted, dropped) = podvm_enter::userland::host_env(
            rootfs,
            crate::interpose::GUEST_PATH,
            &env,
            &u.lib_dirs,
        );
        env = hosted;
        dropped_preload = dropped;
    }
    let path_dirs = podvm_enter::Plan::path_from(&env);
    // ⭐ TODO/complete.md T-0413: the re-entered payload's guest path, as in
    // `run`'s `prepare`. A fresh chroot re-entry is a fresh payload.
    crate::run::push_guest_exe(
        &mut env,
        rootfs,
        gargv.first().map(String::as_str).unwrap_or(""),
        &path_dirs,
    );
    // ⭐ TODO/enter.md T-1407 and T-1409, as in `run`'s `prepare`: the
    // root-anchoring table (or the explicit opt-out) and the loader
    // exe-force join here, banner-named below.
    let mut userland_note = String::new();
    // ⭐ T-1415, as in `run`'s `prepare`: the gate below reads what
    // `userland_env` decided, not the flag beside it.
    let mut strict_unsafe = false;
    if let Some(u) = &userland {
        match crate::lifecycle::userland_env(
            "exec",
            rootfs,
            env,
            u.family,
            o.unsafe_host_paths,
            &mut std::io::stderr(),
        ) {
            Ok(ue) => {
                env = ue.env;
                userland_note = ue.banner;
                strict_unsafe = ue.unsafe_paths;
            }
            Err(code) => return code,
        }
    }
    let working_dir = o.workdir.clone().unwrap_or_else(|| "/".to_string());
    // ⭐ TODO/enter.md T-1412, as in `run`'s `prepare`: the workdir is
    // checked before the fork and before any fixup mutates the rootfs. The
    // image named is the container's own, read from its record above.
    if let Err(e) = podvm_enter::check_workdir(rootfs, &working_dir, &container.image) {
        eprintln!("podvm exec: {e}");
        return e.exit_code();
    }
    let selection = podvm_probe::select::Selection::choose(&findings);
    // ⭐ T-0804 rule 4, as in `run`'s `prepare`: the banner is built from
    // the rung `exec` ENTERS with, chrooted or userland.
    let entered = userland
        .as_ref()
        .map(|_| podvm_probe::select::Rung::Userland)
        .unwrap_or(podvm_enter::ENTERED_RUNG);
    let mut banner = podvm_probe::report::entry_banner(&findings, &selection, entered);
    if let Some(note) = crate::names::alias_note() {
        banner.push_str(&note);
    }
    banner.push_str(&degradation());
    banner.push_str(&interpose_note);
    // ⭐ TODO/enter.md T-0501. The re-entry serves the container's
    // mappings, so it names them like the entry did.
    for m in &exec_devices {
        banner.push_str(&podvm_enter::device::banner_line(m));
    }
    // ⭐ TODO/enter.md T-1317, T-1407 and T-1409: the family account and
    // the anchoring beside it, as in `run`'s `prepare`.
    if let Some(u) = &userland {
        banner.push_str(&u.banner);
        banner.push('\n');
        if let Some(d) = &dropped_preload {
            banner.push_str(&format!(
                "podvm: userland: caller LD_PRELOAD {d:?} dropped: it names guest \
                 paths, which resolve on the host without a chroot\n"
            ));
        }
        banner.push_str(&userland_note);
    }
    // ⭐ M5. `exec` completes the rootfs exactly as `run` does, and for the same
    // reason: a fresh chroot re-entry is a fresh payload, and the `/dev/null` a
    // previous one turned into a file is still a file.
    let mut ask = o.ask.clone();
    ask.container_name = Some(target.to_string());
    let mut completion = match crate::complete::prepare("exec", rootfs, &ask, &mut banner) {
        Ok(r) => r,
        Err(code) => return code,
    };
    let mut err = std::io::stderr().lock();
    // ⭐ T-0804 rule 1, and ahead of every refusal below for `run`'s own
    // reasons: a refused caller gets the banner too, and T-0412's steps have to
    // be named before they run.
    // ⭐ TODO/cli.md T-1416: quiet by default, the banner under
    // `--verbose`; a refusal still prints, because it never passes
    // through here.
    let loud = crate::complete::banner_loud(store, &ask);
    let quiet = !loud;
    if loud {
        let _ = write!(err, "{banner}");
    }
    if let Err(code) = crate::complete::strict_refusal(
        "exec",
        &ask,
        entered.word(),
        &completion,
        &mut err,
        &crate::complete::StrictCtx {
            chroot_usable: podvm_probe::probes::chroot_usable(&findings),
            unsafe_host_paths: strict_unsafe,
            preload_dropped: dropped_preload.is_some(),
            gaps: crate::lifecycle::gaps_of(userland.as_ref()),
        },
    ) {
        return code;
    }
    if o.tty && !podvm_probe::probes::ptmx_usable(&findings) {
        let _ = writeln!(err, "{TTY_REFUSAL}");
        return podvm_enter::EXIT_RUNTIME_ERROR;
    }
    // ⭐ T-0412, and `exec` runs them for the same reason it completes the
    // rootfs at all: a fresh chroot re-entry is a fresh payload, and a keyring
    // or a CA index a previous one destroyed is still destroyed.
    // ⭐ T-0710: steps share the container's memo, opened once for the steps
    // and again for the payload below.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = podvm_supervise::table::open_memo(&memo_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            "exec",
            rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }
    let memo = match podvm_supervise::table::open_memo(&memo_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podvm exec: the ownership memo could not be opened: {e}"
            );
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    let mut plan = Plan {
        argv: o.command.clone(),
        env,
        working_dir,
        fds: Fds {
            pass: vec![(podvm_supervise::table::MEMO_CHILD_FD, memo_host)],
        },
        // ⚠ Empty: the banner was printed above, before the steps.
        banner: String::new(),
        path_dirs,
        names: podvm_enter::NAMES,
    };
    if let Err(c) = serve_devices(&mut plan, &mut err) {
        return c;
    }
    let root = match RootDir::open(rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podvm exec: {e}");
            return e.exit_code();
        }
    };
    // ⭐ TODO/enter.md T-1410. The shared entry decision drives here: the
    // userland families through the one shared drive, the chroot sequence
    // as before. The container's memo is the container's own, so nothing
    // ephemeral is removed on any path out.
    match &userland {
        Some(u) => {
            let outcome =
                crate::lifecycle::enter_userland("exec", &root, &plan, u, rootfs, &mut err);
            drop(memo);
            outcome.code
        }
        None => match podvm_enter::run(&root, &plan, &mut err) {
            Ok(c) => c,
            Err(e) => {
                let _ = writeln!(err, "podvm exec: {e}");
                e.exit_code()
            }
        },
    }
}
