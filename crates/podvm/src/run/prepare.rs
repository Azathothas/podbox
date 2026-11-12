use super::*;
use podvm_enter::ExitCode as _;

/// Everything `run` and `create` both need: the image in the store, extracted,
/// its plan resolved and the rung selected, with nothing entered yet.
///
/// ⛔ ONE PATH for both verbs. `create` that resolved its own image would be a
/// second implementation of the hardest half of `run`, and the one nobody
/// exercises is the one that diverges.
pub(crate) fn prepare(
    args: &[String],
    store: &podvm_image::Store,
    verb: &str,
) -> std::result::Result<crate::lifecycle::Prepared, i32> {
    let mut o = parse(verb, args)?;
    // ⭐ TODO/machine.md T-1302. The tier is decided before anything is
    // fetched: a machine-tier refusal names the legs, and there is nothing
    // to pull for a tier that cannot run.
    let tier = crate::tier::resolve(o.tier);
    let guest = matches!(
        tier,
        crate::tier::Tier::Machine | crate::tier::Tier::Incus | crate::tier::Tier::Runsc
    );
    // ⭐ TODO/packaging.md T-1003. A forced launch rung is refused here, before
    // anything is fetched, where the verb cannot honor it: only foreground
    // `run` drives the ladder, and no guest tier reaches it (so it sits first).
    if let Err(text) = crate::ladder::refuse_where_undriven(verb, o.detach, guest) {
        eprintln!("podvm {verb}: {text}");
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    let ladder = match crate::ladder::forced_mode() {
        Ok(forced) => forced,
        Err(e) => {
            eprintln!("podvm {verb}: {e}");
            return Err(e.exit_code());
        }
    };
    let (platform, policy) = crate::lifecycle::platform_and_policy(
        verb,
        o.platform.as_deref(),
        &o.insecure,
        o.tls_verify,
    )?;
    // TODO/machine.md T-1656: `run` copies a volume into the guest.
    let volumes_refused = verb != "run" && !o.volumes.is_empty();
    if guest && (!o.devices.is_empty() || volumes_refused) {
        // ⛔ Refused rather than silently dropped: the guest boots behind
        // an emulator the host cannot hand descriptors or paths to, so a
        // mapping the caller believes is set would serve nothing.
        // TODO/enter.md T-0501 and T-1653.
        eprintln!(
            "podvm {verb}: --device and -v need the chroot tier: the machine \
             tier boots a guest no host descriptor or host path reaches"
        );
        return Err(EXIT_FLAG_ERROR);
    }
    // ⭐ TODO/milestones.md T-1112. A Windows guest on the machine tier is
    // the one platform where the OCI path below has nothing to do: the
    // positional is a disk image, the accelerator comes from the same probe
    // that chose the tier, and the result is the guest's own stdout, stderr
    // and exit code. So it takes the driver's door (the very invocation the
    // entry writes down, `run --podvm-tier=machine --platform windows/amd64`)
    // ahead of the OS gate, which is the refusal for every *other*
    // non-Linux platform.
    if crate::windows::should_drive(
        verb,
        o.detach,
        tier == crate::tier::Tier::Machine,
        &platform.os,
    ) {
        return Err(crate::windows::run_windows(
            verb,
            o.mem,
            &platform.arch,
            o.image.as_deref().unwrap_or_default(),
            &o.command,
            &o.qemu_args,
        ));
    }
    // ⭐ TODO/milestones.md T-1112. The OS gate sits before the tier dispatch
    // and the fetch: a platform no tier can enter refuses naming it.
    crate::lifecycle::ensure_linux_guest(verb, &platform.os, &platform.arch)?;
    if let Some(f) = super::machine::guest_tier(tier) {
        return Err(f(verb, store, &o, &platform, &policy));
    }
    if !o.qemu_args.is_empty() {
        // ⛔ Refused rather than silently dropped. An emulator argument the
        // chroot tier accepts and ignores is a limit the caller believes is
        // set and podvm never passed anywhere.
        eprintln!("podvm {verb}: --podvm-qemu-arg needs --podvm-tier=machine");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.mem.is_some() {
        // ⛔ The same rule for the memory the machine tier judges against
        // the file-size ceiling: accepted-and-ignored elsewhere would be a
        // limit the caller believes is enforced. TODO/machine.md T-1305.
        eprintln!("podvm {verb}: --podvm-mem needs --podvm-tier=machine");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.cpus.is_some() {
        // ⛔ The same rule: a rootfs tier has no vCPU to give (T-1654).
        eprintln!("podvm {verb}: --podvm-cpus needs --podvm-tier=machine");
        return Err(EXIT_FLAG_ERROR);
    }
    let image = o.image.clone().expect("checked in parse");

    // ⭐ TODO/enter.md T-1411. The probe runs HERE, before the fetch, the
    // lock, the extraction and every fixup: the two-tier gate below
    // refuses now, naming what is missing, with the rootfs untouched.
    // The machine tier returned above and never chroots.
    // `findings` is reused for the banner below, so the probe runs once.
    let findings = podvm_probe::run();
    // ⭐ TODO/enter.md T-0503. The flag-specific refusal fires where the
    // flag alone decides, ahead of every gate: `-t` with an unusable ptmx
    // refuses naming ptmx whatever else holds, because no rung here
    // allocates a pty without it. Where chroot is denied too the chroot
    // sentence prints beside it rather than being masked by it. Exit
    // stays 125, one line per reason.
    if o.tty && !podvm_probe::probes::ptmx_usable(&findings) {
        eprintln!(
            "podvm {verb}: -t was asked for and /dev/ptmx is not usable on this \
             machine, so podvm cannot allocate a pty. It refuses rather than \
             running without one and letting the payload discover it \
             (TODO/enter.md T-0503)"
        );
        if !podvm_probe::probes::chroot_usable(&findings) {
            eprintln!(
                "podvm {verb}: chroot(2) is denied on this machine as well \
                 (see `podvm probe`) (TODO/enter.md T-1317)"
            );
        }
        return Err(podvm_enter::EXIT_RUNTIME_ERROR);
    }
    // ⭐ TODO/enter.md T-1411. This reverses T-1317's last-resort decision
    // for records: `create` no longer keeps the strict chroot gate,
    // because the launcher now drives the userland loader family
    // detached. Both verbs take the two-tier gate here (it refuses only
    // where nothing could run); the exact rung is decided post-extract
    // below, and only a detached entry without a driver refuses there.
    crate::lifecycle::ensure_entry_possible(verb, &findings, store.root())?;

    // ------------------------------------------------------------- the image
    let record = acquire(verb, store, &image, &platform, &policy, &o.pull)?;
    // ⭐ TODO/milestones.md T-1112. The stored record is what gets entered,
    // so its OS is gated too: a record for another OS answers here rather
    // than inside the guest.
    crate::lifecycle::ensure_linux_guest(verb, &record.os, &record.architecture)?;

    // ⛔ The lock BEFORE the extraction check, so a concurrent `rmi` cannot
    // delete the rootfs between podvm deciding it is there and entering it.
    // TODO/image.md T-0204. ⚠ Dropped at the end of this function: the caller
    // takes its own, for the process that actually enters.
    let held = store.hold(&record).map_err(|e| {
        eprintln!("podvm {verb}: {e}");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })?;

    if !podvm_extract::is_extracted(store, &record.manifest_digest) {
        extract_now(verb, store, &record)?;
    }
    // ⭐ TODO/image.md T-1709: the container's own copy of the image. Each
    // write below (the object, the fixups, the payload) lands in it.
    let tree = super::tree::Tree::fill(verb, store, &record.manifest_digest)?;
    let rootfs = tree.rootfs();
    crate::sandprobe::place(&rootfs).map_err(|e| {
        eprintln!("podvm {verb}: probe-inside: {e} (TODO/probe.md T-1666)");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })?;

    // ---------------------------------------------------- the foreign platform
    let emulate = tier == crate::tier::Tier::Emulate; // T-1651: qemu, not binfmt
    let support_native = super::foreign::support(verb, &image, &record, &rootfs, emulate)?;
    let mut err = std::io::stderr().lock();

    // --------------------------------------------------------------- the plan
    let cfg = config_of(verb, store, &record)?;
    let (image_entry, entry) = super::entrypoints(cfg.config.entrypoint.as_deref(), &o);
    let argv = Plan::argv_for(image_entry, cfg.config.cmd.as_deref(), &o.command, entry);
    let argv = super::wrapped(&o, argv); // T-1654: `fs.wrap`
    if argv.is_empty() {
        eprintln!(
            "podvm {verb}: {image} declares neither an Entrypoint nor a Cmd, and no \
             command was given. There is nothing to run"
        );
        return Err(EXIT_FLAG_ERROR);
    }
    let working_dir = o
        .workdir
        .clone()
        .unwrap_or_else(|| cfg.config.working_dir.clone());
    // ⭐ TODO/enter.md T-1653: the table of the run. The payload, the
    // working directory and each guest path below resolve through it, so
    // a -w or a first program in a volume is the file of the volume.
    // ⭐ TODO/enter.md T-1657: the syscall tier gives `/tmp` as a directory
    // of the run, with a cap that its handler enforces.
    let syscall = tier == crate::tier::Tier::Syscall;
    if syscall && (o.detach || verb == "create") {
        eprintln!(
            "podvm {verb}: --podvm-tier=syscall serves a foreground run: its /tmp is a \
             directory of the run, removed when the run ends (TODO/enter.md T-1657)"
        );
        return Err(EXIT_FLAG_ERROR);
    }
    if syscall && !o.tmpfs.iter().any(|t| t == "/tmp") {
        o.tmpfs.push("/tmp".to_string());
    }
    let tmp_cap = o.tmp_size.unwrap_or(crate::lifecycle::syscall::TMP_CAP);
    add_tmpfs(verb, store, &mut o)?; // TODO/cli.md T-1655
    let maps = crate::lifecycle::volume::maps(verb, &rootfs, &o.volumes)?;
    let mut env = Plan::env_for(&podvm_enter::NAMES, &cfg.config.env, &o.env);
    env.extend(crate::sandprobe::host_env(store.root())); // T-1666
    crate::profile::check_needs(verb, &rootfs, &env, &o.needs)?; // T-1659
                                                                 // ⭐ T-0711: the requested identity travels in the environment, resolved
                                                                 // against the image, before anything classifies the payload: the object
                                                                 // reads it on first use in every process, which is what survives fork
                                                                 // and exec where a file descriptor would need re-handing.
    crate::lifecycle::apply_user(verb, &rootfs, o.user.as_ref(), &mut env)?;
    // ⭐ T-0710: the host memo, beside the container record once it exists.
    // Ephemeral here under staging; `create` and `run -d` move it to
    // `containers/<id>/`, foreground `run` deletes it on exit. The descriptor
    // number is constant, so the environment set here is final for every later
    // entry into the same container.
    let memo_host_path = crate::interpose::ephemeral_memo_path(store);
    if let Err(e) = crate::interpose::ensure_memo_file(&memo_host_path) {
        eprintln!("podvm {verb}: the ownership memo could not be created: {e}");
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != crate::interpose::MEMO_FD_VAR);
    env.push(crate::interpose::memo_fd_env());
    let broker = crate::broker::for_run(verb, o.broker, syscall, &o.allow, &o.ports)?; // T-1665
    crate::lifecycle::syscall::tier_env(&mut env, syscall, tmp_cap, broker.as_ref());
    if let Some(b) = broker.as_ref().filter(|_| syscall) {
        o.ask.add_hosts.extend(b.fake_hosts()); // T-1665: the fake addresses
    }
    // ⭐ TODO/enter.md T-1411. The exact rung, decided here: the rootfs
    // exists and nothing has been written into it yet (`place` and the
    // fixups run below). This reverses T-1317's last-resort decision for
    // detached entries too: `run -d` and `create` drive the loader
    // family (the record carries its argv) and refuse only where the
    // decided family has no detached driver, naming the blocking call
    // and the affected verbs.
    // ⭐ T-1648, T-1649, T-1707: the opt-in, the word of a refusal, the workdir.
    let userland = if syscall {
        Some(crate::lifecycle::syscall::decide(
            verb,
            &maps,
            &argv,
            &working_dir,
            &env,
            tmp_cap,
        )?)
    } else if emulate {
        let a = &record.architecture;
        Some(crate::lifecycle::emulate::decide(
            verb,
            &maps,
            &argv,
            &working_dir,
            &env,
            a,
        )?)
    } else {
        crate::lifecycle::decide_entry(
            verb,
            &maps,
            &argv,
            &working_dir,
            &env,
            support_native,
            &findings,
            o.unsafe_host_paths,
            o.retry_word(cfg.config.entrypoint.as_deref()),
        )?
    };
    crate::lifecycle::refuse_detached_userland(verb, userland.as_ref(), o.detach)?;
    // ⭐ TODO/enter.md T-1653: only the loader family serves a volume.
    crate::lifecycle::volume::refuse_unserved(verb, &o.volumes, userland.as_ref(), ladder)?;
    // ⭐ T-0702 and T-0706: classify the payload and place the object BEFORE
    // the banner is built, so the banner names the write before anything of
    // the payload's runs. The note joins the banner below. T-1649: a `#!`
    // payload on the userland rung runs, and so classifies, its interpreter.
    let gargv = userland.as_ref().map_or(&argv, |u| &u.guest_argv);
    let mut interpose_note = String::new();
    let first = podvm_enter::program_in(gargv.first().map_or("", String::as_str), &working_dir);
    if let Err(e) = crate::interpose::apply(verb, &maps, &first, &mut env, &mut interpose_note) {
        eprintln!("{e}");
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    // ⭐ TODO/enter.md T-1317. Without a chroot the loader resolves guest
    // paths on the host root: the preload becomes the host path and the
    // libraries come from the image. A caller preload naming guest paths
    // is dropped and named, because it cannot resolve.
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
    // ⭐ TODO/enter.md T-0501. The device spec travels in the environment,
    // beside the descriptors it names: the entering process (foreground
    // `run` here, the launcher for `start`) opens the host paths before
    // the root changes and serves duplicates where the interposer holds.
    // Parsed twice (here and in `parse`) so a malformed shape is a flag
    // error up front and this one is unreachable rather than re-checked.
    let mut devices = Vec::with_capacity(o.devices.len());
    for d in &o.devices {
        match podvm_enter::device::parse(d) {
            Ok(m) => devices.push(m),
            Err(why) => {
                eprintln!("podvm {verb}: --device {why}");
                return Err(EXIT_FLAG_ERROR);
            }
        }
    }
    podvm_enter::device::push_spec(&mut env, &devices);
    // ⭐ TODO/complete.md T-0413, T-1707. The payload's guest path for the
    // interposer's `/proc/self/exe`; a caller's value is scrubbed first.
    let exe = podvm_enter::program_in(gargv.first().map_or("", String::as_str), &working_dir);
    push_guest_exe(&mut env, &maps, &exe, &path_dirs);
    // ⭐ TODO/enter.md T-1407 and T-1409. On a userland entry the
    // root-anchoring table (or the explicit opt-out) and the loader
    // exe-force join the environment here, before the banner names them
    // and before any fixup mutates the rootfs. `PODVM_MAPS` and
    // `PODVM_GUEST_EXE_FORCE` are scrubbed first, so a nested run
    // inherits neither the outer payload's table nor its flag.
    let mut userland_note = String::new();
    // ⭐ T-1415: the gate below reads what `userland_env` decided, not the
    // flag beside it, so the two cannot disagree.
    let mut strict_unsafe = false;
    if let Some(u) = &userland {
        let ue = crate::lifecycle::userland_env(
            verb,
            &maps,
            env,
            u.family,
            o.unsafe_host_paths,
            &mut err,
        )?;
        env = ue.env;
        userland_note = ue.banner;
        strict_unsafe = ue.unsafe_paths;
    }
    ro_env(verb, &o, userland.as_ref(), &mut env)?; // TODO/cli.md T-1655
                                                    // ⭐ TODO/enter.md T-1412. The workdir is checked here, before the fork
                                                    // and before any fixup mutates the rootfs, on both rungs: a missing
                                                    // directory refuses naming the path and the image at 125, rather than
                                                    // starting the payload in the wrong directory with exit 0. docker
                                                    // creates the directory; podvm refuses and says so, and `-w /tmp`
                                                    // prints `/tmp` as the passing control.
    if let Err(e) = podvm_enter::check_workdir(&maps, &working_dir, &image) {
        eprintln!("podvm {verb}: {e}");
        return Err(e.exit_code());
    }

    // ------------------------------------------------------------- the banner
    // ⭐ TODO/probe.md T-0107 and T-0108's remaining halves: the rung is
    // selected here, for a real entry, and the banner names it and what it must
    // never claim. The probe ran up front (T-1317's gate); the banner reuses
    // its findings.
    let (mut banner, entered) = entry_banner(
        &findings,
        userland.as_ref(),
        &interpose_note,
        &devices,
        dropped_preload.as_deref(),
        &userland_note,
    );
    banner.push_str(&store_note(store));
    banner.push_str(&tree.fill.line());
    // ⭐ M5. The completion layer runs HERE: after the rootfs exists and the
    // image lock is held, and before anything is entered. Its report is part of
    // the banner, because every one of these is an edit podvm made inside
    // somebody else's image and the payload can see it.
    let mut ask = o.ask.clone();
    ask.container_name = o.name.clone();
    let mut completion = crate::complete::prepare(verb, &rootfs, &ask, &mut banner)?;
    if !cfg.config.user.is_empty() {
        // ⚠ Read and REPORTED, never applied. podvm cannot setuid to an id
        // this machine does not map, which is the wall the project is about.
        banner.push_str(&format!(
            "podvm: the image asks to run as user {:?}; podvm runs as uid 0 \
             and does not change to it\n",
            cfg.config.user
        ));
    }
    // ⭐ T-0804 rule 1, applied HERE and nowhere else, so `run`, `run -d`,
    // `create` and the launcher cannot disagree about whether this machine
    // prints it. ⛔ It moved ahead of every refusal below on purpose: a refused
    // caller used to get the reasons with no banner under `--strict`, and
    // `create` printed neither, while T-0412's steps mean podvm may now run a
    // command inside the image, which rule 3 says is named before it runs.
    let loud = crate::complete::banner_loud(store, &ask);
    // ⭐ TODO/cli.md T-1416: quiet by default, the banner under
    // `--verbose`. `quiet` still gates the step lines below, so a default
    // run is silent on success; a refusal still prints, because it never
    // passes through here.
    let quiet = !loud;
    if loud {
        let _ = write!(err, "{banner}");
    }
    // ⛔ T-0503, decided up front beside the entry gate above: `-t` with
    // an unusable ptmx refuses there naming ptmx, so no `-t` check
    // remains here. `exec` keeps its own: it is ungated by chroot, so
    // nothing there masks it.
    // ⭐ T-0804, and it is the LAST thing before the run is committed to, so a
    // refused caller still gets the whole account of why on stderr. ⛔ The
    // refusal prints even where the banner is suppressed: the config switch
    // silences a notice, never a refusal.
    // ⭐ T-1415: the userland facts ride beside the three T-0804 inputs, so
    // kind decides rather than count.
    crate::complete::strict_refusal(
        verb,
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
    )?;
    // ⭐ T-0412 and T-0710. The steps share this entry's host memo: a fixup's
    // `chown` must land in the same record the payload reads, or the two
    // disagree about who owns the file. Opened here for the steps alone; the
    // payload's own handle is opened by the caller that spawns it.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = crate::interpose::open_memo(&memo_host_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            verb,
            &rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }
    // TODO/enter.md T-1658: the confine layer, on a rung with a chroot,
    // after the fixups: a node that the completion layer made (`/dev/null`)
    // gets its rule.
    let confine = crate::lifecycle::confine::decide(
        verb,
        o.confine.as_ref(),
        entered,
        &rootfs,
        userland.as_ref(),
    )?;
    let _ = path_dirs;
    // ⚠ The lock this function took goes here. It existed to keep the rootfs
    // from being deleted between the extraction check and now; the process that
    // ENTERS takes its own, and for `-d` that is the launcher rather than this.
    drop(held);
    tree.keep();
    Ok(crate::lifecycle::Prepared {
        record,
        image,
        rootfs,
        argv,
        env,
        working_dir,
        name: o.name.clone(),
        rung: entered.word().to_string(),
        detach: o.detach,
        rm: o.rm,
        wall_ms: o.timeout.map(|s| s * 1000),
        hardening: o.hardening.clone(),
        memo_host_path,
        ladder,
        findings,
        completion: completion
            .fixups
            .iter()
            .filter(|f| f.action.changed() || f.action == podvm_complete::Action::Failed)
            .map(|f| {
                format!(
                    "{} {} ({}, {})",
                    f.action.word(),
                    if f.path.is_empty() { "-" } else { &f.path },
                    f.id,
                    f.entry
                )
            })
            .collect(),
        completion_degraded: completion.degradations().len(),
        userland,
        confine,
    })
}
