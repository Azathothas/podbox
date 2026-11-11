//! TODO/machine.md T-1656. `run --podvm-tier=machine`: the image in the
//! store, extracted, and booted as a guest ([`crate::machine::guest`]).
//!
//! The command, the environment and the working directory come from the
//! image config and the flags, as on each other tier. The run makes no
//! container record: the guest is disposable, as under `--rm`.

use super::*;

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
    let image = o.image.clone().unwrap_or_default();
    let record = match acquire(verb, store, &image, platform, policy, &o.pull) {
        Ok(r) => r,
        Err(c) => return c,
    };
    if let Err(c) = crate::lifecycle::ensure_linux_guest(verb, &record.os, &record.architecture) {
        return c;
    }
    let _held = match store.hold(&record) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("podvm {verb}: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    if !podvm_extract::is_extracted(store, &record.manifest_digest)
        && let Err(c) = extract_now(verb, store, &record)
    {
        return c;
    }
    let (rootfs, _) = podvm_extract::paths(store, &record.manifest_digest);
    let cfg = match config_of(verb, store, &record) {
        Ok(c) => c,
        Err(c) => return c,
    };
    let argv = Plan::argv_for(
        cfg.config.entrypoint.as_deref(),
        cfg.config.cmd.as_deref(),
        &o.command,
        o.entrypoint.as_deref(),
    );
    if argv.is_empty() {
        eprintln!(
            "podvm {verb}: {image} declares neither an Entrypoint nor a Cmd, and no \
             command was given. There is nothing to run"
        );
        return EXIT_FLAG_ERROR;
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
    crate::machine::guest::boot(&crate::machine::guest::Request {
        verb,
        store_root: store.root(),
        rootfs: &rootfs,
        digest: &record.manifest_digest,
        argv,
        env,
        workdir,
        mem: o.mem,
        timeout: o.timeout.map(std::time::Duration::from_secs),
        qemu_args: o.qemu_args.clone(),
    })
}
