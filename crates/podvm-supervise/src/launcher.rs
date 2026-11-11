//! The entry of a detached payload: what podvm's launcher does between its
//! lock and its pidfd.
//!
//! [`TODO/supervise.md`](../../../TODO/supervise.md) T-0601, T-0602, T-0603,
//! T-0605.
//!
//! ⭐ **The launcher is `pod_supervise::launcher` (PodUtils/podcrates).** It
//! double-forks, takes the container's lock, binds the control socket, puts
//! its own stdio on `/dev/null`, opens the payload's stdio on the log, holds
//! the pidfd, answers `SIG` and `WAIT`, and writes `running` and `exited`.
//! [`enter`] is podvm's part, run in that launcher before the root changes:
//! the ownership memo, the devices, the image hold, the plan, and the rung.
//!
//! ⛔ **Nothing here spawns a thread.** `PR_SET_PDEATHSIG` fires on the
//! creating THREAD's exit, and this runs on the path that clones, chroots and
//! execs (`nothing_on_the_spawn_path_can_spawn_a_thread`).

use std::os::fd::AsRawFd;

use pod_supervise::launcher::{Launch, Payload};
use podvm_enter::{Fds, Plan, RootDir};
use podvm_image::Store;

use crate::table::{self, Container};

/// What the launcher keeps open while the payload runs, and drops after it.
struct Kept {
    _memo: std::fs::File,
    _held: podvm_image::store::Lock,
    _root: RootDir,
    _child: podvm_enter::Child,
}

/// podvm's entry for `pod_supervise::start`: it runs in the launcher, and
/// its `Err` text reaches the caller as the start's failure.
pub fn enter<'a>(
    store: &'a Store,
    record: &'a podvm_image::Record,
) -> impl FnOnce(&Container, Launch<'_>) -> Result<Payload, String> + 'a {
    move |container, launch| enter_in_launcher(store, record, container, launch)
}

fn enter_in_launcher(
    store: &Store,
    record: &podvm_image::Record,
    container: &Container,
    launch: Launch<'_>,
) -> Result<Payload, String> {
    let Launch { mut pass, log } = launch;
    // ⭐ T-0710: the host memo, beside this container's record. Opened here,
    // before the root changes, and dup'd to the constant child number the
    // environment names. An entry that cannot be handed one refuses rather
    // than starting with no record.
    let memo_path = table::memo_path(store, &container.id);
    let memo = table::open_memo(&memo_path)
        .map_err(|e| format!("the ownership memo could not be opened: {e}"))?;
    pass.push((table::MEMO_CHILD_FD, memo.as_raw_fd() as i64));

    // ⭐ TODO/enter.md T-0501. The container's device spec re-opens HERE,
    // in the process that enters: a descriptor cannot cross the client
    // that ran `create`, so the record carries the spec and each entry
    // opens it before its own root changes. A host path gone since
    // `create` refuses the start naming it.
    let mut env = container.env.clone();
    let opened = podvm_enter::device::specs_of(&env)
        .and_then(|s| podvm_enter::device::open_all(&s))
        .map_err(|e| format!("starting with --device: {e}"))?;
    pass.extend(opened.pass.iter().copied());
    podvm_enter::device::push_serve(&mut env, &opened);

    let root = RootDir::open(&container.rootfs).map_err(|e| e.to_string())?;
    // The stored environment already carries the constant descriptor number
    // from `create`; containers made before T-0710 do not, so ensure it here
    // rather than refusing a record that predates the rule.
    if table::memo_fd_of(&env).is_none() {
        env.push(table::memo_fd_env());
    }
    let plan = Plan {
        argv: container.argv.clone(),
        env: env.clone(),
        working_dir: container.working_dir.clone(),
        fds: Fds { pass },
        // ⚠ Empty: the banner was printed by the process the caller was
        // watching, before it forked the launcher. Printing it again here
        // would put it in the container's LOG.
        banner: String::new(),
        path_dirs: Plan::path_from(&env),
        names: podvm_enter::NAMES,
        limits: container.of.hardening.limits(),
        no_new_privs: container.of.hardening.no_new_privs,
        // podcrates v0.22.0 (COORD-046, T-205): the payload gets SIGKILL
        // when this launcher dies, so it never outlives a killed
        // supervisor. The launcher forks from its one thread.
        die_with_parent: true,
        // TODO/enter.md T-1658: the confine layer of `run -d`.
        confine: container.of.hardening.confine_spec(&container.rootfs)?,
    };

    // ⭐ TODO/image.md T-0204 and T-0211. The image lock is taken here and
    // handed to the payload immediately before the fork that leads to its
    // exec, so a concurrent `rmi` or `prune` cannot delete the rootfs a
    // running container is executing out of. ⛔ In the LAUNCHER and not in
    // the caller: the caller exits as soon as the container is up.
    let held = store
        .hold(record)
        .map_err(|e| format!("holding the image: {e}"))?;
    held.hand_to_payload()
        .map_err(|e| format!("handing the image lock to the payload: {e}"))?;

    // ⭐ TODO/enter.md T-1411: where the record carries the loader drive
    // `create` decided, the launcher execs it with no chroot through
    // `spawn_userland`; else T-1339: the rung the probe selects, decided
    // where the payload is entered. The fallback line joins the container
    // log: the launcher's stderr is /dev/null.
    let child = if container.userland_exec.is_empty() {
        let findings = podvm_probe::run();
        let selection = podvm_probe::select::Selection::choose(&findings);
        podvm_enter::spawn_selected(&root, &plan, selection.rung, log).map_err(|e| e.to_string())?
    } else {
        // ⭐ TODO/enter.md T-1653: the volumes of the record join the root
        // pair, so a working directory in a volume resolves in the volume.
        let volumes = podvm_enter::userland::volumes_of(&container.env);
        let maps =
            podvm_enter::userland::Maps::new(&podvm_enter::NAMES, &container.rootfs, &volumes)
                .map_err(|why| format!("starting with -v: {why}"))?;
        podvm_enter::spawn_userland(
            &root,
            &plan,
            &maps,
            container.userland_exec.clone(),
            podvm_probe::select::Rung::Userland.word(),
            None,
            log,
        )
        .map_err(|e| e.to_string())?
    };
    let pid = child.pid;
    Ok(Payload {
        pid,
        keep: Box::new(Kept {
            _memo: memo,
            _held: held,
            _root: root,
            _child: child,
        }),
    })
}

#[cfg(test)]
mod tests;
