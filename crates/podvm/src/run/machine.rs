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
        _ => None,
    }
}

/// What both tiers take from the store and the image config.
struct Prepared {
    digest: String,
    rootfs: std::path::PathBuf,
    argv: Vec<String>,
    env: Vec<String>,
    workdir: String,
    stdin: Option<Vec<u8>>,
    /// Held while the run uses the rootfs.
    _held: podvm_image::store::Lock,
}

fn prepare_image(
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
    let argv = Plan::argv_for(
        cfg.config.entrypoint.as_deref(),
        cfg.config.cmd.as_deref(),
        &o.command,
        o.entrypoint.as_deref(),
    );
    let argv = super::wrapped(&o.wrap, argv); // T-1654: `fs.wrap`
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
    let p = match prepare_image(verb, store, o, platform, policy, "machine") {
        Ok(p) => p,
        Err(c) => return c,
    };
    // `-v HOST:GUEST`: a copy into the guest (`:ro` changes nothing there).
    let mut copies: Vec<_> = o
        .volumes
        .iter()
        .map(|v| (std::path::PathBuf::from(&v.host), v.guest.clone()))
        .collect();
    // TODO/probe.md T-1666: the probe of probe-inside, at /.podvm.
    let probe = crate::sandprobe::guest_copy(store.root());
    copies.extend(probe.clone().map(|d| (d, "/.podvm".to_string())));
    let mut env = p.env;
    env.extend(crate::sandprobe::host_env(store.root()));
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
        timeout: o.timeout.map(std::time::Duration::from_secs),
        qemu_args: o.qemu_args.clone(),
        copies,
        stdin: p.stdin,
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
    if verb != "run" {
        eprintln!(
            "podvm {verb}: --podvm-tier=incus serves run only: each run is a disposable \
             instance (TODO/machine.md T-1718)"
        );
        return EXIT_FLAG_ERROR;
    }
    if !o.volumes.is_empty() || o.mem.is_some() || o.cpus.is_some() || !o.qemu_args.is_empty() {
        eprintln!(
            "podvm {verb}: --podvm-tier=incus takes no -v, --podvm-mem, --podvm-cpus or \
             --podvm-qemu-arg yet (TODO/machine.md T-1718)"
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
    })
}
