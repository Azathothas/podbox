//! TODO/machine.md T-1656 and T-1718. `run --podvm-tier=machine` and
//! `run --podvm-tier=incus`: the image in the store, extracted, and run in
//! a guest of podvm ([`crate::machine::guest`]) or in an incus instance
//! ([`crate::incus`]).
//!
//! The command, the environment and the working directory come from the
//! image config and the flags, as on each other tier. The run makes no
//! container record: the guest or the instance is disposable, as under
//! `--rm`.

use super::*;

type Guest = fn(&str, &podvm_image::Store, &Opts, &Platform, &Policy) -> i32;

/// The `run` of a tier that runs the image in a guest, or `None`.
pub(super) fn guest_tier(tier: crate::tier::Tier) -> Option<Guest> {
    match tier {
        crate::tier::Tier::Machine => Some(machine_tier),
        crate::tier::Tier::Incus => Some(incus_tier),
        crate::tier::Tier::Runsc => Some(super::runsc::runsc_tier),
        _ => None,
    }
}

/// What both tiers take from the store and the image config.
pub(super) struct Prepared {
    pub(super) digest: String,
    pub(super) rootfs: std::path::PathBuf,
    pub(super) argv: Vec<String>,
    pub(super) env: Vec<String>,
    pub(super) workdir: String,
    pub(super) stdin: Option<Vec<u8>>,
    /// Held while the run uses the rootfs.
    _held: podvm_image::store::Lock,
}

pub(super) fn prepare_image(
    verb: &str,
    store: &podvm_image::Store,
    o: &Opts,
    platform: &Platform,
    policy: &Policy,
    tier: &str,
) -> Result<Prepared, i32> {
    let image = o.image.clone().unwrap_or_default();
    let record = acquire(verb, store, &image, platform, policy, &o.pull)?;
    crate::lifecycle::ensure_linux_guest(verb, &record.os, &record.architecture)?;
    let held = store.hold(&record).map_err(|e| {
        eprintln!("podvm {verb}: {e}");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })?;
    if !podvm_extract::is_extracted(store, &record.manifest_digest) {
        extract_now(verb, store, &record)?;
    }
    let (rootfs, _) = podvm_extract::paths(store, &record.manifest_digest);
    let cfg = config_of(verb, store, &record)?;
    let (image_entry, entry) = super::entrypoints(cfg.config.entrypoint.as_deref(), o);
    let argv = Plan::argv_for(image_entry, cfg.config.cmd.as_deref(), &o.command, entry);
    let argv = super::wrapped(o, argv); // T-1654: `fs.wrap`
    if argv.is_empty() {
        eprintln!(
            "podvm {verb}: {image} declares neither an Entrypoint nor a Cmd, and no \
             command was given. There is nothing to run"
        );
        return Err(EXIT_FLAG_ERROR);
    }
    let workdir = o
        .workdir
        .clone()
        .unwrap_or_else(|| cfg.config.working_dir.clone());
    let workdir = if workdir.is_empty() {
        "/".to_string()
    } else {
        workdir
    };
    let env = Plan::env_for(&podvm_enter::NAMES, &cfg.config.env, &o.env);
    let stdin = crate::machine::guest::read_stdin().map_err(|e| {
        eprintln!("podvm {verb}: {tier} tier: {e}");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })?;
    Ok(Prepared {
        digest: record.manifest_digest.clone(),
        rootfs,
        argv,
        env,
        workdir,
        stdin,
        _held: held,
    })
}

/// The machine tier of `run` and `create`: `run` boots a guest; `create`
/// keeps a record of no guest, so it reports the profile and exits 125.
pub(super) fn machine_tier(
    verb: &str,
    store: &podvm_image::Store,
    o: &Opts,
    platform: &Platform,
    policy: &Policy,
) -> i32 {
    if verb != "run" {
        return crate::tier::enter_machine(verb, o.mem);
    }
    if let Some(m) = o.mem
        && let Ok((cur, _)) = podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE)
        && crate::tier::mem_over_ceiling(m, cur)
    {
        eprintln!(
            "podvm {verb}: machine tier refused: guest memory {m} bytes over the \
             RLIMIT_FSIZE ceiling {cur} bytes (prlimit(RLIMIT_FSIZE) cur={cur})"
        );
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // TODO/machine.md T-1751: a sealed run takes each value of the run
    // from the bundle; a flag would put it in the argv of podvm.
    if o.sealed
        && (!o.command.is_empty()
            || !o.env.is_empty()
            || o.workdir.is_some()
            || !o.volumes.is_empty()
            || o.tty
            || o.broker)
    {
        eprintln!(
            "podvm {verb}: --podvm-sealed takes the command, the environment, the working \
             directory and the files from the bundle on stdin: it refuses a command, -e, -w, \
             -v, -t and --network broker (TODO/machine.md T-1751)"
        );
        return EXIT_FLAG_ERROR;
    }
    let mut p = match prepare_image(verb, store, o, platform, policy, "machine") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let sealed = if o.sealed {
        let Some(b) = p.stdin.take() else {
            eprintln!(
                "podvm {verb}: --podvm-sealed reads the bundle of the run from stdin, a pipe \
                 or a file (TODO/machine.md T-1751)"
            );
            return EXIT_FLAG_ERROR;
        };
        if let Err(why) = crate::machine::guest::sealed::check(&b) {
            eprintln!("podvm {verb}: --podvm-sealed: {why} (TODO/machine.md T-1751)");
            return EXIT_FLAG_ERROR;
        }
        Some(b)
    } else {
        None
    };
    // `-v HOST:GUEST[:ro]`: a 9p mount of the host directory
    // (TODO/machine.md T-1723).
    let volumes: Vec<_> = o
        .volumes
        .iter()
        .map(|v| {
            (
                std::path::PathBuf::from(&v.host),
                v.guest.clone(),
                o.ro_volumes.contains(&v.guest),
            )
        })
        .collect();
    let mut copies = Vec::new();
    // TODO/probe.md T-1666: the probe of probe-inside, at /.podvm.
    let probe = crate::sandprobe::guest_copy(store.root());
    copies.extend(probe.clone().map(|d| (d, "/.podvm".to_string())));
    // TODO/net.md T-1665: a loopback proxy is the gateway in the guest.
    // TODO/machine.md T-1778: with no NIC there is no gateway, and the
    // guest's loopback is where --podvm-channel listens, so the variable
    // stays as it is.
    let rewritten = if o.net_none || o.sealed {
        Ok(p.env.clone())
    } else {
        crate::machine::proxy::rewrite_env(&p.env)
    };
    let mut env = match rewritten {
        Ok(e) => e,
        Err(why) => {
            eprintln!("podvm {verb}: machine tier: {why}");
            return EXIT_FLAG_ERROR;
        }
    };
    env.extend(crate::sandprobe::host_env(store.root()));
    if o.net_none && o.broker {
        eprintln!(
            "podvm {verb}: --podvm-net none leaves no NIC for --network broker: the broker \
             is a forward of the guest's network (TODO/machine.md T-1751)"
        );
        return EXIT_FLAG_ERROR;
    }
    // TODO/net.md T-1665: the guest reaches the broker alone, at a forward
    // of its restricted network.
    let broker = if o.broker {
        match crate::broker::start(&o.allow, &o.ports) {
            Ok(b) => {
                let (h, p) = crate::broker::GUEST_FORWARD;
                crate::broker::replace_proxy_env(&mut env, b.env(h, p));
                Some(b.port)
            }
            Err(e) => {
                eprintln!("podvm {verb}: --network broker: {e} (TODO/net.md T-1665)");
                return podvm_image::error::EXIT_RUNTIME_ERROR;
            }
        }
    } else {
        None
    };
    let code = crate::machine::guest::boot(&crate::machine::guest::Request {
        verb,
        store_root: store.root(),
        rootfs: &p.rootfs,
        digest: &p.digest,
        argv: p.argv,
        env,
        workdir: p.workdir,
        mem: o.mem,
        cpus: o.cpus,
        broker,
        timeout: o.timeout.map(std::time::Duration::from_secs),
        qemu_args: o.qemu_args.clone(),
        copies,
        volumes,
        tty: o.tty,
        stdin: p.stdin,
        net_none: o.net_none || o.sealed,
        channel: o.channel.clone(),
        sealed,
    });
    if let Some(d) = probe {
        let _ = std::fs::remove_dir_all(d);
    }
    code
}

/// The incus tier (T-1718): `run` only; each other verb refuses by name.
pub(super) fn incus_tier(
    verb: &str,
    store: &podvm_image::Store,
    o: &Opts,
    platform: &Platform,
    policy: &Policy,
) -> i32 {
    if verb != "run" && verb != "create" {
        eprintln!(
            "podvm {verb}: --podvm-tier=incus serves run and create (TODO/machine.md T-1718, \
             T-1722)"
        );
        return EXIT_FLAG_ERROR;
    }
    if o.mem.is_some()
        || o.cpus.is_some()
        || !o.qemu_args.is_empty()
        || o.broker
        || o.net_none
        || o.channel.is_some()
        || o.sealed
    {
        eprintln!(
            "podvm {verb}: --podvm-tier=incus takes no --podvm-mem, --podvm-cpus, \
             --podvm-qemu-arg, --podvm-net, --podvm-channel, --podvm-sealed or --network \
             broker yet \
             (TODO/machine.md T-1718)"
        );
        return EXIT_FLAG_ERROR;
    }
    let p = match prepare_image(verb, store, o, platform, policy, "incus") {
        Ok(p) => p,
        Err(c) => return c,
    };
    crate::incus::run::run(&crate::incus::run::Job {
        verb,
        store_root: store.root(),
        rootfs: &p.rootfs,
        digest: &p.digest,
        argv: p.argv,
        env: p.env,
        workdir: p.workdir,
        stdin: p.stdin,
        // TODO/machine.md T-1722: create and run -d keep the instance.
        detach: (verb == "create" || o.detach).then(|| crate::incus::verbs::Detach {
            name: o.name.clone(),
            start: verb == "run",
            image: o.image.clone().unwrap_or_default(),
            stdin: o.interactive,
        }),
        // TODO/machine.md T-1722: each -v is a disk device of the instance.
        volumes: o
            .volumes
            .iter()
            .map(|v| {
                (
                    v.host.clone(),
                    v.guest.clone(),
                    o.ro_volumes.contains(&v.guest),
                )
            })
            .collect(),
    })
}
