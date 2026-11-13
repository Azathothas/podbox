//! TODO/enter.md T-1729. `run --podvm-tier=runsc`: the image in the
//! store, a copy of it for the container (T-1709), and gVisor's runsc
//! ([`crate::runsc`]). The run makes no container record: the copy goes at
//! the end, as under `--rm`.

use super::*;

/// The runsc tier: `run` only; each flag that the tier does not serve
/// refuses by name.
pub(super) fn runsc_tier(
    verb: &str,
    store: &podvm_image::Store,
    o: &Opts,
    platform: &Platform,
    policy: &Policy,
) -> i32 {
    let refuse = |what: &str| {
        eprintln!("podvm {verb}: --podvm-tier=runsc {what} (TODO/enter.md T-1729)");
        EXIT_FLAG_ERROR
    };
    if verb != "run" || o.detach {
        return refuse("serves a foreground run only: the bundle goes at its end");
    }
    if o.tty {
        return refuse("takes no -t: the bundle has no terminal");
    }
    if o.mem.is_some()
        || o.cpus.is_some()
        || !o.qemu_args.is_empty()
        || o.broker
        || o.confine.is_some()
        || o.timeout.is_some()
        || !o.tmpfs.is_empty()
        || o.net_none
        || o.channel.is_some()
        || o.sealed
    {
        return refuse(
            "takes no --podvm-mem, --podvm-cpus, --podvm-qemu-arg, --podvm-net, \
             --podvm-channel, --podvm-sealed, --network broker, --podvm-confine, --timeout or --tmpfs yet",
        );
    }
    let user = match crate::runsc::numeric_user(o.user.as_deref()) {
        Ok(u) => u,
        Err(e) => return refuse(&e),
    };
    // The checks of the host come before the pull: a host with no runsc
    // or no user namespaces fetches nothing.
    if let Err(e) = crate::runsc::binary().and_then(|_| crate::runsc::user_namespaces()) {
        eprintln!("podvm {verb}: --podvm-tier=runsc: {e} (TODO/enter.md T-1729)");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    let p = match super::machine::prepare_image(verb, store, o, platform, policy, "runsc") {
        Ok(p) => p,
        Err(c) => return c,
    };
    let tree = match super::tree::Tree::fill(verb, store, &p.digest) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let rootfs = std::path::PathBuf::from(tree.rootfs());
    crate::runsc::run(&crate::runsc::Job {
        verb,
        store_root: store.root(),
        rootfs: &rootfs,
        argv: p.argv,
        env: p.env,
        workdir: p.workdir,
        stdin: p.stdin,
        volumes: &o.volumes,
        ro: &o.ro_volumes,
        read_only: o.read_only,
        user,
    })
}
