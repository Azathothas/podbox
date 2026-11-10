use super::*;
use podvm_enter::ExitCode as _;

/// The `-t` refusal, with T-0503's entry named. The usability predicate
/// lives in `podvm-probe`, beside the rows it reads; `run` asks the same
/// one.
pub(super) const TTY_REFUSAL: &str =
    "podvm exec: -t was asked for and /dev/ptmx is not usable on this \
machine, so podvm cannot allocate a pty. It refuses rather than running without one and \
letting the payload discover it (TODO/enter.md T-0503)";

pub fn exec(args: &[String]) -> i32 {
    let o = match parse(args) {
        Ok(o) => o,
        Err(code) => return code,
    };
    // ⭐ TODO/packaging.md T-1003. `exec` re-enters and never reaches the
    // ladder, so a force refused here names that rather than falling through
    // it silently.
    if let Err(text) = crate::ladder::refuse_where_undriven("exec", false, false) {
        eprintln!("podvm exec: {text}");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⭐ TODO/machine.md T-1302, as in `run`'s `prepare`: the tier is decided
    // before the store is touched, so a machine-tier refusal names the legs
    // with nothing pulled for a tier that cannot run.
    let tier = crate::tier::resolve(o.tier);
    let platform = match Platform::wanted(o.platform.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("podvm exec: {e}");
            return e.exit_code();
        }
    };
    // ⭐ TODO/milestones.md T-1112, as in `run`'s `prepare`: the OS gate
    // sits before the tier dispatch. `exec` never pulls, so a platform
    // no tier can enter is refused before the store is even opened,
    // rather than reaching the machine tier's leg refusal.
    if let Err(c) = crate::lifecycle::ensure_linux_guest("exec", &platform.os, &platform.arch) {
        return c;
    };
    if tier == crate::tier::Tier::Machine {
        return crate::tier::enter_machine("exec", o.mem);
    }
    if !o.qemu_args.is_empty() {
        eprintln!("podvm exec: --podvm-qemu-arg needs --podvm-tier=machine");
        return EXIT_FLAG_ERROR;
    }
    if o.mem.is_some() {
        eprintln!("podvm exec: --podvm-mem needs --podvm-tier=machine");
        return EXIT_FLAG_ERROR;
    }
    let image = o.image.clone().expect("checked in parse");

    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm exec: {e}");
            return e.exit_code();
        }
    };

    // ⭐ M4. A CONTAINER NAME FIRST, then an image reference. `docker exec` takes
    // a container and podvm's takes either, because until M4 there were no
    // containers and an image was the only thing to re-enter. ⚠ The mechanism is
    // the same either way: a container's rootfs is a directory in the store and
    // so is an image's.
    if let Some((rootfs, state)) = crate::lifecycle::rootfs_of(&store, &image) {
        return enter(&image, &rootfs, state, &o, &store);
    }

    // ⛔ Never pulls. `exec` re-enters something that is already there, so a
    // reference the store does not hold is a refusal that names `run`, not a
    // fetch the caller did not ask for.
    let found = store.find_for(&image, Some(&platform)).unwrap_or_default();
    let others = found.other_platforms.clone();
    let Some(record) = found.one() else {
        if others.is_empty() {
            eprintln!(
                "podvm exec: {image} is not in the store. `podvm exec` never pulls: \
                 `podvm run {image} ...` is what puts it there"
            );
        } else {
            eprintln!(
                "podvm exec: the store holds {image} for {}, and {platform} was asked \
                 for. `podvm exec` never pulls",
                others.join(", ")
            );
        }
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    };

    // ⛔ The lock BEFORE the extraction check, so a concurrent `rmi` cannot
    // delete the rootfs between podvm deciding it is there and entering it.
    // TODO/image.md T-0204, the same ordering `run` takes.
    let held = match store.hold(&record) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("podvm exec: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    if !podvm_extract::is_extracted(&store, &record.manifest_digest) {
        eprintln!(
            "podvm exec: {image} is in the store and has never been extracted, so \
             there is no rootfs to re-enter. `podvm run {image} ...` or `podvm \
             extract {image}` is what puts one there"
        );
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    };
    // ⭐ TODO/milestones.md T-1112. The stored record is what gets entered.
    if let Err(c) = crate::lifecycle::ensure_linux_guest("exec", &record.os, &record.architecture) {
        return c;
    }
    let (rootfs, _) = podvm_extract::paths(&store, &record.manifest_digest);
    let rootfs = rootfs.to_string_lossy().to_string();

    // --------------------------------------------------------------- the plan
    let cfg = match crate::run::config_of("exec", &store, &record) {
        Ok(c) => c,
        Err(code) => return code,
    };
    // ⛔ The image's Entrypoint is NOT prepended. docker's `exec` runs the
    // command given and nothing around it, and an entrypoint that wraps a
    // second entry is a process the caller did not write.
    let argv = o.command.clone();
    let mut env = Plan::env_for(&podvm_enter::NAMES, &cfg.config.env, &o.env);
    // ⭐ T-0711: the one call, as on the container path above: a second
    // spelling of the same resolve-and-set would be the second copy
    // `docs/conventions/code.md` refuses.
    if let Err(code) = crate::lifecycle::apply_user("exec", &rootfs, o.user.as_ref(), &mut env) {
        return code;
    }
    // ⭐ T-0710: an ephemeral host memo, as foreground `run` holds. No
    // container record exists on this path, but the payload still cannot reach
    // the file by path; it is handed the descriptor at spawn and deleted on
    // exit.
    let memo_host_path = crate::interpose::ephemeral_memo_path(&store);
    if let Err(e) = podvm_supervise::table::ensure_memo_file(&memo_host_path) {
        eprintln!("podvm exec: the ownership memo could not be created: {e}");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != podvm_supervise::table::MEMO_FD_VAR);
    env.push(podvm_supervise::table::memo_fd_env());
    let findings = podvm_probe::run();
    // ⭐ TODO/enter.md T-1410. The image path shares the entry decision
    // with `run` and with the container path above: where chroot is
    // denied the loader and memfd families drive. A refusal names the
    // blocking call.
    let host = Platform::host();
    let support_native = matches!(
        podvm_enter::binfmt::support_for(&record.architecture, &host.arch),
        podvm_enter::binfmt::Support::Native
    );
    let userland = match crate::lifecycle::decide_entry(
        "exec",
        &rootfs,
        &argv,
        &env,
        support_native,
        &findings,
        o.unsafe_host_paths,
        None,
    ) {
        Ok(u) => u,
        Err(code) => return code,
    };
    // ⭐ T-0702 and T-0706, as on the container path above: the image path is
    // a second way to reach the same entry, not a second answer to it. Placed
    // after the decision, so a refusal writes nothing into the image; T-1649:
    // a `#!` payload on the userland rung classifies its interpreter.
    let gargv = userland.as_ref().map_or(&argv, |u| &u.guest_argv);
    let mut interpose_note = String::new();
    if let Err(e) = crate::interpose::apply("exec", &rootfs, gargv, &mut env, &mut interpose_note) {
        eprintln!("{e}");
        let _ = std::fs::remove_file(&memo_host_path);
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⭐ The loader transform and the maps/exe-force, as on the container
    // path above and in `run`'s `prepare`.
    let mut dropped_preload: Option<String> = None;
    if let Some(u) = &userland
        && u.family == podvm_enter::userland::Family::Loader
    {
        let (hosted, dropped) = podvm_enter::userland::host_env(
            &rootfs,
            crate::interpose::GUEST_PATH,
            &env,
            &u.lib_dirs,
        );
        env = hosted;
        dropped_preload = dropped;
    }
    let path_dirs = Plan::path_from(&env);
    // ⭐ TODO/complete.md T-0413: the re-entered payload's guest path, as in
    // `run`'s `prepare` and the container path above.
    crate::run::push_guest_exe(
        &mut env,
        &rootfs,
        gargv.first().map(String::as_str).unwrap_or(""),
        &path_dirs,
    );
    // ⭐ TODO/enter.md T-1407 and T-1409, as above: the table and the
    // loader exe-force join here, banner-named below.
    let mut userland_note = String::new();
    // ⭐ T-1415, as above: the gate below reads what `userland_env`
    // decided, not the flag beside it.
    let mut strict_unsafe = false;
    if let Some(u) = &userland {
        match crate::lifecycle::userland_env(
            "exec",
            &rootfs,
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
    let working_dir = o
        .workdir
        .clone()
        .unwrap_or_else(|| cfg.config.working_dir.clone());
    // ⭐ TODO/enter.md T-1412, as in `run`'s `prepare` and the container
    // path above: refused naming the path and the image before any fixup
    // mutates the rootfs.
    if let Err(e) = podvm_enter::check_workdir(&rootfs, &working_dir, &image) {
        eprintln!("podvm exec: {e}");
        return e.exit_code();
    }

    // ------------------------------------------------------------- the banner
    let selection = podvm_probe::select::Selection::choose(&findings);
    // ⭐ T-0804 rule 4, as on the container path above: the banner names
    // the rung this re-entry takes, chrooted or userland.
    let entered = userland
        .as_ref()
        .map(|_| podvm_probe::select::Rung::Userland)
        .unwrap_or(podvm_enter::ENTERED_RUNG);
    let mut banner = podvm_probe::report::entry_banner(&findings, &selection, entered);
    // ⭐ TODO/cli.md T-0803. Where podvm was reached under somebody else's
    // name, the banner says which name was used and that this is podvm.
    // Taking the name is the product requirement; taking it silently is what
    // TOOL.md section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        banner.push_str(&note);
    }
    banner.push_str(&degradation());
    banner.push_str(&interpose_note);
    // ⭐ TODO/enter.md T-1317, T-1407 and T-1409: the family account and
    // the anchoring beside it, as on the container path above.
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
    // ⭐ M5. The same completion the container path takes, from the same
    // function: two entry paths that complete a rootfs differently would be two
    // answers to one question, and the one nobody exercises is the one that
    // diverges.
    let mut ask = o.ask.clone();
    ask.container_name = Some(image.clone());
    let mut completion = match crate::complete::prepare("exec", &rootfs, &ask, &mut banner) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let mut err = std::io::stderr().lock();
    // ⭐ T-0804 rule 1, ahead of the refusals: see the container path above.
    // ⭐ TODO/cli.md T-1416: quiet by default, the banner under `--verbose`.
    let loud = crate::complete::banner_loud(&store, &ask);
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
    if o.tty {
        // ⛔ T-0503, and the same shared predicate `enter` asks above: `Ok`
        // and nothing else, because a skip is not a pass.
        if !podvm_probe::probes::ptmx_usable(&findings) {
            let _ = writeln!(err, "{TTY_REFUSAL}");
            return podvm_enter::EXIT_RUNTIME_ERROR;
        }
    }
    // ⭐ T-0412's steps, on the image path as on the container one.
    // ⭐ T-0710: steps share the ephemeral memo, opened once for the steps and
    // again for the payload below.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = podvm_supervise::table::open_memo(&memo_host_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            "exec",
            &rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }

    let memo = match podvm_supervise::table::open_memo(&memo_host_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podvm exec: the ownership memo could not be opened: {e}"
            );
            let _ = std::fs::remove_file(&memo_host_path);
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    let mut plan = Plan {
        argv,
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
        drop(memo);
        let _ = std::fs::remove_file(&memo_host_path);
        return c;
    }

    // -------------------------------------------------------------- the entry
    let root = match RootDir::open(&rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podvm exec: {e}");
            drop(memo);
            let _ = std::fs::remove_file(&memo_host_path);
            return e.exit_code();
        }
    };
    if let Err(e) = held.hand_to_payload() {
        let _ = writeln!(err, "podvm exec: {e}");
        drop(memo);
        let _ = std::fs::remove_file(&memo_host_path);
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // -------------------------------------------------------------- the entry
    // ⭐ TODO/enter.md T-1410. The shared entry decision drives here too:
    // the userland families through the one shared drive, the chroot
    // sequence as before. The ephemeral memo is removed on every path
    // out, as before.
    let code = match &userland {
        Some(u) => {
            crate::lifecycle::enter_userland("exec", &root, &plan, u, &rootfs, &mut err).code
        }
        None => match podvm_enter::run(&root, &plan, &mut err) {
            Ok(c) => c,
            Err(e) => {
                let _ = writeln!(err, "podvm exec: {e}");
                e.exit_code()
            }
        },
    };
    drop(memo);
    let _ = std::fs::remove_file(&memo_host_path);
    code
}
