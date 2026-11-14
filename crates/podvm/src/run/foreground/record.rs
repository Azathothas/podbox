//! The record of a foreground run: made before the entry, live while the
//! payload runs, and ended with the code (TODO/supervise.md T-1421,
//! T-1786).

use super::tee::*;
use podvm_image::clock;

/// One foreground run's record-in-progress.
///
/// TODO/supervise.md T-1421. docker always records a foreground run as
/// `Exited`, so `ps -a` lists it and `logs` reads it. podvm does the
/// same: the record is created before the entry (so a duplicate `--name`
/// refuses before anything runs), the payload's stdout is teed to the
/// container log while it runs, and the record lands `Exited` with the
/// code and the times when it ends. `--rm` deletes the record on exit
/// instead of finalizing it.
pub(super) struct Foreground {
    pub(super) id: String,
    rm: bool,
    started: String,
    tee: Vec<Tee>,
    /// TODO/supervise.md T-1786: the hold of the payload, on the rungs
    /// where podvm spawns the payload itself and has its pid.
    pub(super) hold: Option<podvm_supervise::Hold>,
}

/// Create the foreground record and start the stdout tee, before the
/// entry. A `--name` that cannot be recorded refuses here naming it,
/// instead of running and dropping the name: an accepted flag is never
/// silently dropped (T-1421's decision).
pub(super) fn fg_begin(
    store: &podvm_image::Store,
    p: &crate::lifecycle::Prepared,
) -> std::result::Result<Foreground, i32> {
    let c = match podvm_supervise::create(
        store,
        p.name.as_deref(),
        &p.image,
        &p.record.manifest_digest,
        &p.rootfs,
        p.argv.clone(),
        p.env.clone(),
        p.working_dir.clone(),
        &p.rung,
        p.completion.clone(),
        p.completion_degraded,
        crate::lifecycle::detached_drive(p.userland.as_ref()),
    ) {
        // TODO/cli.md T-1695: the labels and the options of the run join
        // the record, as for `run -d`.
        Ok(c) => match podvm_supervise::set_hardening(store, &c.id, p.hardening.clone()) {
            Ok(()) => c,
            Err(e) => {
                let _ = podvm_supervise::remove(store, &c.id, true);
                eprintln!("podvm run: the run could not be recorded: {e}");
                return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
            }
        },
        Err(e) => {
            crate::run::drop_unrecorded(store, &p.rootfs); // T-1709
            match &p.name {
                Some(n) => {
                    eprintln!("podvm run: --name {n:?}: the run could not be recorded: {e}")
                }
                None => eprintln!("podvm run: the run could not be recorded: {e}"),
            }
            return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
        }
    };
    let log_path = podvm_supervise::table::log_path(store, &c.id);
    match tee_output_to(&log_path) {
        Ok(tee) => Ok(Foreground {
            id: c.id,
            rm: p.rm,
            started: clock::now(),
            tee,
            hold: None,
        }),
        Err(why) => {
            eprintln!("podvm run: the run's output could not be captured for its record: {why}");
            let _ = podvm_supervise::remove(store, &c.id, true);
            Err(podvm_image::error::EXIT_RUNTIME_ERROR)
        }
    }
}

/// Finalize the foreground record: stdout restored first (no byte still
/// in flight), then `Exited` with the code and the times, or the
/// record's removal under `--rm`.
pub(super) fn fg_end(store: &podvm_image::Store, fg: Foreground, code: i32) {
    // stderr first, then stdout: the reverse of the order they were taken.
    for tee in fg.tee.into_iter().rev() {
        untee(tee);
    }
    // TODO/cli.md T-1655: the `--tmpfs` directories go with the run.
    let _ = std::fs::remove_dir_all(crate::run::tmpfs_base(store));
    // TODO/supervise.md T-1786: the hold writes `Exited`, the code and the
    // time, and gives the code to each `wait`.
    let held = fg.hold.map(|h| h.finish(code).is_ok()).unwrap_or(false);
    if fg.rm {
        let _ = podvm_supervise::remove(store, &fg.id, true);
        return;
    }
    if held {
        return;
    }
    let finished = clock::now();
    let started = fg.started.clone();
    let id = fg.id.clone();
    let _ = podvm_supervise::table::update(store, |t| {
        match t.containers.iter_mut().find(|c| c.id == id) {
            Some(c) => {
                c.state = podvm_supervise::table::State::Exited;
                c.exit_code = Some(code);
                if c.started_at.is_none() {
                    c.started_at = Some(started.clone());
                }
                c.finished_at = Some(finished.clone());
                Ok(())
            }
            None => Err(podvm_supervise::Error::new(format!(
                "the foreground record {id} is gone"
            ))),
        }
    });
}

/// TODO/supervise.md T-1786: the record reads `Running` while the payload
/// `pid` runs. A hold that cannot start leaves the run as it was: the
/// record lands at the end, and a line names why.
pub(super) fn take_hold(
    store: &podvm_image::Store,
    id: &str,
    pid: i64,
    hold: &mut Option<podvm_supervise::Hold>,
    err: &mut dyn std::io::Write,
) {
    match podvm_supervise::foreground(store, id, pid) {
        Ok(h) => *hold = Some(h),
        Err(e) => {
            let _ = writeln!(err, "podvm run: the record of the run is not live: {e}");
        }
    }
}
