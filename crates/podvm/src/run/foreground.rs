use super::*;
use podvm_enter::ExitCode as _;

/// One foreground run's record-in-progress.
///
/// TODO/supervise.md T-1421. docker always records a foreground run as
/// `Exited`, so `ps -a` lists it and `logs` reads it. podvm does the
/// same: the record is created before the entry (so a duplicate `--name`
/// refuses before anything runs), the payload's stdout is teed to the
/// container log while it runs, and the record lands `Exited` with the
/// code and the times when it ends. `--rm` deletes the record on exit
/// instead of finalizing it.
struct Foreground {
    id: String,
    rm: bool,
    started: String,
    tee: Option<Tee>,
}

mod tee;
use tee::*;

/// Create the foreground record and start the stdout tee, before the
/// entry. A `--name` that cannot be recorded refuses here naming it,
/// instead of running and dropping the name: an accepted flag is never
/// silently dropped (T-1421's decision).
fn fg_begin(
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
        Ok(c) => c,
        Err(e) => {
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
    match tee_stdout_to(&log_path) {
        Ok(tee) => Ok(Foreground {
            id: c.id,
            rm: p.rm,
            started: clock::now(),
            tee: Some(tee),
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
fn fg_end(store: &podvm_image::Store, fg: Foreground, code: i32) {
    if let Some(tee) = fg.tee {
        untee(tee);
    }
    // TODO/cli.md T-1655: the `--tmpfs` directories go with the run.
    let _ = std::fs::remove_dir_all(super::tmpfs_base(store));
    if fg.rm {
        let _ = podvm_supervise::remove(store, &fg.id, true);
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
            None => Err(podvm_supervise::Error(format!(
                "the foreground record {id} is gone"
            ))),
        }
    });
}

pub fn run(args: &[String]) -> i32 {
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm run: {e}");
            return e.exit_code();
        }
    };
    let p = match prepare(args, &store, "run") {
        Ok(p) => p,
        Err(code) => return code,
    };
    let mut err = std::io::stderr().lock();

    // ⭐ M4. `-d` hands the container to a detached launcher and returns as soon
    // as its payload is running, which is a condition rather than a duration:
    // TODO/supervise.md T-0602.
    if p.detach {
        // ⚠ The banner was printed by `prepare`, which had to print it before
        // running T-0412's steps inside the rootfs.
        drop(err);
        let c = match podvm_supervise::create(
            &store,
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
            Ok(c) => c,
            Err(e) => {
                eprintln!("podvm run: {e}");
                let _ = std::fs::remove_file(&p.memo_host_path);
                return podvm_image::error::EXIT_RUNTIME_ERROR;
            }
        };
        let kept = podvm_supervise::set_wall(&store, &c.id, p.wall_ms)
            .and_then(|_| podvm_supervise::set_hardening(&store, &c.id, p.hardening.clone()));
        if let Err(e) = kept {
            eprintln!("podvm run: {e}");
            let _ = podvm_supervise::remove(&store, &c.id, true);
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
        // ⭐ T-0710: the ephemeral memo becomes the container's, beside its
        // record. The environment already carries the constant descriptor
        // number, so nothing in it moves with the file.
        let dest = podvm_supervise::table::memo_path(&store, &c.id);
        if let Err(e) = std::fs::rename(&p.memo_host_path, &dest) {
            eprintln!("podvm run: the ownership memo could not be stored: {e}");
            let _ = podvm_supervise::remove(&store, &c.id, true);
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
        return match podvm_supervise::start(&store, &c.id, &p.record) {
            Ok(c) => {
                println!("{}", c.id);
                0
            }
            Err(e) => {
                eprintln!("podvm run: {e}");
                // ⚠ The record is removed again: a container that could not be
                // started is not one `ps` should list as created, and leaving
                // it would also hold its name against a retry.
                let _ = podvm_supervise::remove(&store, &c.id, true);
                podvm_image::error::EXIT_RUNTIME_ERROR
            }
        };
    }

    // ---------------------------------------------------- the foreground entry
    let held = match store.hold(&p.record) {
        Ok(h) => h,
        Err(e) => {
            let _ = writeln!(err, "podvm run: {e}");
            let _ = std::fs::remove_file(&p.memo_host_path);
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    let root = match RootDir::open(&p.rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podvm run: {e}");
            let _ = std::fs::remove_file(&p.memo_host_path);
            return e.exit_code();
        }
    };
    // ⭐ T-0710: the host memo, handed as a descriptor that survives the
    // `chroot` and the `execve`. The file lives beside no container record
    // here, but still on the host under staging, where the payload cannot
    // reach it by path.
    let memo = match crate::interpose::open_memo(&p.memo_host_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podvm run: the ownership memo could not be opened: {e}"
            );
            let _ = std::fs::remove_file(&p.memo_host_path);
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    // ⭐ TODO/enter.md T-0501. The host paths open HERE, before the fork
    // whose child chroots: a descriptor opened now keeps working after the
    // root changes, and it is the only thing that crosses the boundary.
    let mut pass = vec![(crate::interpose::MEMO_CHILD_FD, memo_host)];
    let mut env = p.env.clone();
    match podvm_enter::device::specs_of(&env).and_then(|s| podvm_enter::device::open_all(&s)) {
        Ok(opened) => {
            pass.extend(opened.pass.iter().copied());
            podvm_enter::device::push_serve(&mut env, &opened);
        }
        Err(e) => {
            let _ = writeln!(err, "podvm run: {e}");
            drop(memo);
            let _ = std::fs::remove_file(&p.memo_host_path);
            return e.exit_code();
        }
    }
    let plan = Plan {
        argv: p.argv.clone(),
        env,
        working_dir: p.working_dir.clone(),
        fds: Fds { pass },
        // ⚠ Empty, and that is the same reason the launcher's is: `prepare`
        // printed the banner, because T-0412's steps run inside the rootfs
        // after it and every one of them has to be named before it runs.
        banner: String::new(),
        path_dirs: Plan::path_from(&p.env),
        names: podvm_enter::NAMES,
        limits: p.hardening.limits(),
        no_new_privs: p.hardening.no_new_privs,
    };
    // ⭐ T-0204 and T-0211. The lock is handed to the payload immediately
    // before the fork that leads to its exec, and to nothing else.
    // ⭐ TODO/supervise.md T-1421. The foreground record is created
    // before the entry, and the payload's stdout is teed to its log:
    // every foreground run leaves a record `ps -a` lists as `Exited`.
    let fg = match fg_begin(&store, &p) {
        Ok(f) => f,
        Err(code) => {
            drop(memo);
            let _ = std::fs::remove_file(&p.memo_host_path);
            return code;
        }
    };
    if let Err(e) = held.hand_to_payload() {
        let _ = writeln!(err, "podvm run: {e}");
        fg_end(&store, fg, podvm_image::error::EXIT_RUNTIME_ERROR);
        drop(memo);
        let _ = std::fs::remove_file(&p.memo_host_path);
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⛔ TODO/supervise.md T-1708: podvm is the child subreaper of the
    // payload's tree, so an orphan of the payload is reached at the end.
    let reaper = podvm_supervise::tree::become_subreaper();
    // TODO/cli.md T-1655: `--timeout` ends the tree at the bound.
    let wall = p.wall_ms.map(super::wall::arm_wall);
    // ⭐ TODO/packaging.md T-1003. A forced launch rung drives through the
    // ladder; unset means the chroot by path below, untouched. The banner
    // names the entered rung, so a forced run says where it went.
    let code = match p.ladder {
        Some(mode) => {
            let _ = writeln!(
                err,
                "podvm run: entering on the {} rung at PODVM_MODE's request \
                 (TODO/packaging.md T-1003)",
                mode.name()
            );
            // T-1707: a relative program resolves against the workdir.
            let argv0 = plan.argv.first().cloned().unwrap_or_default();
            let argv0 = podvm_enter::program_in(&argv0, &plan.working_dir);
            match crate::ladder::enter_forced(
                &root,
                &plan,
                mode,
                &p.rootfs,
                &argv0,
                &plan.path_dirs,
                &p.findings,
                store.root(),
                &p.record.manifest_digest,
                &mut err,
            ) {
                Ok(c) => c,
                Err(e) => {
                    let _ = writeln!(err, "podvm run: {e}");
                    e.exit_code()
                }
            }
        }
        None => {
            // ⭐ TODO/enter.md T-1317 and T-1410. The no-chroot families
            // enter through the one shared drive beside the chroot
            // sequence below, so `run` and `exec` cannot diverge on it.
            if let Some(u) = &p.userland {
                let outcome = crate::lifecycle::enter_userland("run", &root, &plan, u, &mut err);
                if !outcome.entered {
                    fg_end(&store, fg, outcome.code);
                    drop(memo);
                    let _ = std::fs::remove_file(&p.memo_host_path);
                    return outcome.code;
                }
                outcome.code
            } else {
                // ⭐ TODO/enter.md T-1339. The namespace rung where
                // `prepare` selected it (`p.rung` is the entered word it
                // set beside the banner), with the chroot fallback named
                // on the banner; the chroot sequence everywhere else.
                let code = if p.rung.as_str() == podvm_probe::select::Rung::Namespace.word() {
                    podvm_enter::run_namespace(&root, &plan, &mut err)
                } else {
                    podvm_enter::run(&root, &plan, &mut err)
                };
                match code {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = writeln!(err, "podvm run: {e}");
                        e.exit_code()
                    }
                }
            }
        }
    };
    if wall.is_some_and(super::wall::Wall::disarm) {
        let _ = writeln!(
            err,
            "podvm run: --timeout {} s came: the payload's tree was killed (TODO/cli.md T-1655)",
            p.wall_ms.unwrap_or(0) / 1000
        );
    }
    super::exit_note::note(code, &plan, &p.rootfs, &mut err);
    end_tree(reaper, &mut err);
    drop(err);
    // ⭐ TODO/supervise.md T-1421. The record lands `Exited` with the code
    // and the times (or is removed under `--rm`), after stdout is
    // restored so no logged byte is still in flight.
    fg_end(&store, fg, code);
    // ⭐ T-0710: the ephemeral memo goes with the run. The child holds its own
    // dup past the `execve`; this handle and the staging file are the parent's.
    drop(memo);
    let _ = std::fs::remove_file(&p.memo_host_path);

    if p.rm {
        // ⚠ The lock is dropped first: `remove_extracted` deletes the tree the
        // lock exists to protect, and holding it while deleting would be podvm
        // refusing its own request.
        drop(held);
        // ⭐ TODO/image.md T-1322. The rootfs is shared, so `--rm` removes it
        // only where no container record references it: a failed run must
        // not delete the rootfs `keeper` needs. The run's own record is
        // already finalized above (removed under `--rm`), so it never
        // counts itself. Where the query itself errors, the rootfs stays:
        // deleting on an unanswered question is how the defect above
        // happens.
        match podvm_supervise::referencing(&store, &p.record.manifest_digest) {
            Ok(referrers) if !referrers.is_empty() => {
                let mut names: Vec<&str> = referrers.iter().map(|c| c.name.as_str()).collect();
                names.sort_unstable();
                eprintln!(
                    "podvm run: --rm keeps the rootfs: it is referenced by container {}",
                    names.join(", ")
                );
            }
            Ok(_) => {
                if let Err(e) = podvm_extract::remove_extracted(&store, &p.record.manifest_digest) {
                    eprintln!("podvm run: --rm could not remove the rootfs: {e}");
                }
            }
            Err(e) => {
                eprintln!("podvm run: --rm could not check references: {e}");
            }
        }
    }
    code
}

/// Hand the payload's resolved guest path to the interposer, T-0413.
///
/// [`podvm_enter::ladder::GUEST_EXE_VAR`] carries the path the guest kernel
/// would print for `/proc/self/exe`, resolved here because this side owns
/// the rootfs host path and the child's resolution happens past allocation
/// time. A caller-supplied value is scrubbed first: without that a nested
/// `podvm run` would inherit the outer payload's path. Nothing is pushed
/// where nothing resolves, and the interposer prefers a real answer where
/// one exists, so the variable can ride every entry including the no-chroot
/// ones without poisoning them. `root` is the rootfs, or the table of the
/// run, where a payload in a volume has its guest path in the volume
/// (TODO/enter.md T-1653).
pub(crate) fn push_guest_exe<G: podvm_enter::abi::GuestRoot + ?Sized>(
    env: &mut Vec<String>,
    root: &G,
    argv0: &str,
    path_dirs: &[String],
) {
    let ours = [podvm_enter::ladder::GUEST_EXE_VAR, podvm_dispatch::SELF_VAR];
    env.retain(|e| !ours.contains(&e.split('=').next().unwrap_or("")));
    if let Some(g) = podvm_enter::ladder::guest_exe(root, argv0, path_dirs) {
        env.push(format!("{}={g}", podvm_enter::ladder::GUEST_EXE_VAR));
    }
    // TODO/enter.md T-1724: a static child of the loader family enters the
    // native-static rung through podvm itself.
    if podvm_dispatch::available().is_ok()
        && let Ok(me) = std::fs::read_link("/proc/self/exe")
    {
        env.push(format!("{}={}", podvm_dispatch::SELF_VAR, me.display()));
    }
}

#[cfg(test)]
mod tee_tests;
