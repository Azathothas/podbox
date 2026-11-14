//! `TOOL.md` section 6.6: pidfd, waitid, logs, the container lifecycle.
//!
//! Milestone M4, [`TODO/milestones.md`](../../../TODO/milestones.md) T-1105 and
//! [`TODO/supervise.md`](../../../TODO/supervise.md) T-0601 to T-0607.
//!
//! ⭐ **The mechanism is `pod-supervise` (PodUtils/podcrates).** The table,
//! the launcher, its lock, pidfd and control socket, the logs and the verbs
//! live there. This crate keeps what is podvm's: the settings (the store, its
//! lock, the `podvm_containers` key, the `podvm_` names, `podvm ps -a`,
//! docker's 125), the fields of a podvm record ([`table::Detail`]), the entry
//! of a payload ([`launcher`]: the probe, the plan of `podvm-enter`, the
//! image hold, the memo, the devices) and the ownership memo.
//!
//! ⭐ **Running state is launcher state.** [`table`] is the record and the
//! launcher is the only thing that writes `running` into it. A launcher that
//! was killed leaves a container marked [`table::State::Dead`] with the time
//! it was NOTICED rather than an exit code nobody measured.
//!
//! ⛔ **What podvm does not have, said once here rather than implied
//! everywhere:** a container is a chroot, not a namespace. There is no PID
//! namespace, so a payload's grandchildren that reparent are outside podvm's
//! reach; `stop` signals the payload and says so.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod launcher;
pub mod table;

use podvm_image::Store;
use table::{Container, Detail};

pub use pod_supervise::end::{End, Reason, Residual};

pub mod tree;
pub use pod_supervise::{new_id, tail_offset};

/// Everything this crate can fail with, as one sentence for the caller,
/// and the class of `pod-supervise` where it gave one (TODO/supervise.md
/// T-1805): `NotFound` and `Usage` are the caller's error, exit 1.
#[derive(Debug)]
pub struct Error(pub String, pub Option<pod_core::Class>);

impl Error {
    /// An error of podvm's own, with no class.
    pub fn new(text: impl Into<String>) -> Error {
        Error(text.into(), None)
    }

    /// True where the caller named something that is not there, or asked
    /// for a change that the state of the container refuses.
    pub fn is_callers(&self) -> bool {
        matches!(
            self.1,
            Some(pod_core::Class::NotFound | pod_core::Class::Usage)
        )
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A failure of `pod-supervise` is its sentence: podvm's callers print it.
impl From<pod_core::Failure> for Error {
    fn from(f: pod_core::Failure) -> Error {
        Error(f.chain(), Some(f.class))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// ⛔ **docker's codes, from the one file that holds them.**
// [`TODO/cli.md`](../../../TODO/cli.md) T-0802.
pub use podvm_probe::exit::EXIT_RUNTIME_ERROR;

/// podvm's settings for `pod-supervise`, for a store root.
///
/// ⛔ The table lock is the store's own lock (invariant I1 of
/// `crates/podvm-image/src/store.rs`), so a `prune` and a container write
/// cannot interleave. The key, the names and the files stay what podvm wrote
/// before the crate, so a store keeps its containers.
pub fn settings_at(root: &std::path::Path) -> pod_supervise::Settings {
    let mut s = pod_supervise::Settings::new(root);
    s.table_lock = root.join("lock");
    s.table_key = "podvm_containers".to_string();
    s.name_prefix = "podvm".to_string();
    s.list_command = Some("podvm ps -a".to_string());
    s.failed_code = EXIT_RUNTIME_ERROR;
    s
}

/// [`settings_at`] for a store.
pub fn settings(store: &Store) -> pod_supervise::Settings {
    settings_at(store.root())
}

/// A container name docker would generate, for a caller that gave none.
pub fn generated_name(id: &str) -> String {
    pod_supervise::generated_name(&settings_at(std::path::Path::new("/")), id)
}

/// Write a `created` record. ⛔ Nothing is started here: `create` and `start`
/// are two verbs because docker's are.
///
/// `userland_exec` is the detached userland drive TODO/enter.md T-1411
/// records: the exact loader argv, empty everywhere else. The launcher
/// execs it with no chroot where it is non-empty and enters through
/// `spawn_selected` where it is empty.
#[allow(clippy::too_many_arguments)]
pub fn create(
    store: &Store,
    name: Option<&str>,
    image: &str,
    manifest_digest: &str,
    rootfs: &str,
    argv: Vec<String>,
    env: Vec<String>,
    working_dir: String,
    rung: &str,
    completion: Vec<String>,
    completion_degraded: usize,
    userland_exec: Vec<String>,
) -> Result<Container> {
    let of = Detail {
        image: image.to_string(),
        manifest_digest: manifest_digest.to_string(),
        rootfs: rootfs.to_string(),
        rung: rung.to_string(),
        userland_exec,
        completion,
        completion_degraded,
        hardening: table::Hardening::default(),
    };
    Ok(pod_supervise::create(
        &settings(store),
        name,
        argv,
        env,
        working_dir,
        of,
    )?)
}

/// Start a created container, returning once its payload is running.
///
/// TODO/supervise.md T-1807: since podcrates `v0.44.0` (podcrates@T-582)
/// `pod_supervise::start` clears the end fields of the run before itself.
pub fn start(store: &Store, want: &str, record: &podvm_image::Record) -> Result<Container> {
    Ok(pod_supervise::start(
        &settings(store),
        want,
        launcher::enter(store, record),
    )?)
}

/// The hold of a foreground payload: the record reads `Running` while it
/// runs, and `kill`, `stop`, `wait` and `ps` reach it.
pub type Hold = pod_supervise::Foreground<table::Detail>;

/// TODO/supervise.md T-1786: take the hold of the foreground payload
/// `payload_pid` of the container `want` (podcrates@T-573). The caller
/// reaps the payload and gives its code to [`Hold::finish`].
///
/// ⛔ TODO/supervise.md T-1708: a new thread takes the signal mask of its
/// creator, and the thread of the hold leaves SIGINT and SIGTERM
/// unblocked. Such a thread takes a signal with the default action, and
/// podvm ends 143 with the payload still running. So the shutdown signals
/// are blocked while the hold starts its thread, and the old mask comes
/// back after.
pub fn foreground(store: &Store, want: &str, payload_pid: i64) -> Result<Hold> {
    let mut old = 0u64;
    let blocked = podvm_probe::sys::sigblock_shutdown(&mut old).is_ok();
    let hold = pod_supervise::foreground(&settings(store), want, payload_pid);
    if blocked {
        let _ = podvm_probe::sys::sigrestore(old);
    }
    Ok(hold?)
}

/// Send a signal to a running container's payload.
pub fn kill(store: &Store, want: &str, sig: i32) -> Result<Container> {
    Ok(pod_supervise::kill(&settings(store), want, sig)?)
}

/// SIGTERM, then wait, then SIGKILL. ⛔ Bounded at every step, and each
/// outcome is distinct: `TODO/RULES.md` section 8. A zero grace is SIGKILL at
/// once, as `docker stop -t 0` does (T-1691, issue 215).
pub fn stop(store: &Store, want: &str, timeout_ms: u64) -> Result<(Container, bool)> {
    Ok(pod_supervise::stop(&settings(store), want, timeout_ms)?)
}

/// Set the wall bound of a container before its start: at `wall_ms` after
/// the start the launcher kills the payload and its tree (TODO/cli.md
/// T-1655, `--timeout`).
pub fn set_wall(store: &Store, want: &str, wall_ms: Option<u64>) -> Result<Container> {
    Ok(pod_supervise::set_wall(&settings(store), want, wall_ms)?)
}

/// Keep the limits of a run in its record before its start (TODO/cli.md
/// T-1655): the launcher sets them in the child.
pub fn set_hardening(store: &Store, want: &str, h: table::Hardening) -> Result<()> {
    let s = settings(store);
    let c: Container = pod_supervise::get(&s, want)?;
    pod_supervise::table::update(&s, |t: &mut pod_supervise::table::Table<Detail>| {
        if let Some(x) = t.containers.iter_mut().find(|x| x.id == c.id) {
            x.of.hardening = h;
        }
        Ok::<_, pod_core::Failure>(())
    })?;
    Ok(())
}

/// Wait for a container to end. `None` is the bound, never a guess.
pub fn wait(store: &Store, want: &str, timeout_ms: u64) -> Result<(Container, Option<i32>)> {
    Ok(pod_supervise::wait(&settings(store), want, timeout_ms)?)
}

/// Remove a container's record and its own directory.
///
/// TODO/image.md T-1709: the container's own tree goes with its record. A
/// record whose rootfs is not a container tree (an image's own tree, from
/// an older podvm) leaves that tree to the image verbs.
pub fn remove(store: &Store, want: &str, force: bool) -> Result<Container> {
    let c: Container = pod_supervise::remove(&settings(store), want, force)?;
    // TODO/image.md T-1781: put aside now, removed after podvm's exit.
    if let Err(e) = podvm_extract::tree::discard(store, &c.of.rootfs) {
        return Err(Error::new(format!(
            "{} is removed, and its rootfs {} stays: {e}",
            c.name, c.of.rootfs
        )));
    }
    Ok(c)
}

/// Every container, reconciled.
pub fn list(store: &Store, all: bool) -> Result<Vec<Container>> {
    Ok(pod_supervise::list(&settings(store), all)?)
}

pub fn get(store: &Store, want: &str) -> Result<Container> {
    Ok(pod_supervise::get(&settings(store), want)?)
}

/// Every container record referencing one manifest digest, running or not.
///
/// TODO/image.md T-1322. One query, three callers (`run --rm`, `rmi`,
/// `prune`): docker's removal rule keys on any container, and `create`
/// takes no hold, so only the records see a created container's
/// reference. Reconciled first, so a dead launcher's record reads `Dead`
/// rather than haunting the answer as `Running`.
pub fn referencing(store: &Store, manifest_digest: &str) -> Result<Vec<Container>> {
    Ok(pod_supervise::matching(
        &settings(store),
        |c: &Container| c.manifest_digest == manifest_digest,
    )?)
}

/// The container's captured output.
pub fn logs(store: &Store, want: &str) -> Result<Vec<u8>> {
    Ok(pod_supervise::logs::<Detail>(&settings(store), want)?)
}

/// Follow a container's log until it reaches a terminal state
/// (TODO/supervise.md T-1318).
pub fn follow(store: &Store, want: &str, out: &mut dyn std::io::Write) -> Result<()> {
    Ok(pod_supervise::follow::<Detail>(
        &settings(store),
        want,
        out,
    )?)
}

/// Follow a container's log from its last `n` lines (TODO/cli.md T-1337).
pub fn follow_tail(store: &Store, want: &str, out: &mut dyn std::io::Write, n: u64) -> Result<()> {
    Ok(pod_supervise::follow_tail::<Detail>(
        &settings(store),
        want,
        out,
        n,
    )?)
}

#[cfg(test)]
mod tests;
