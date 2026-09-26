//! `TOOL.md` section 6.5: the chroot entry sequence.
//!
//! [`TODO/milestones.md`](../../../TODO/milestones.md) T-1104 and
//! [`TODO/enter.md`](../../../TODO/enter.md) T-0501 to T-0506.
//!
//! ⭐ **The order of this sequence is the design, and every step of it is an
//! entry.** Doing them in a different order is not a style choice; each ordering
//! constraint below was paid for by a defect somebody shipped.
//!
//! 1. the rootfs is resolved from the store and **refused if it is a symlink**
//!    ([`TODO/enter.md`](../../../TODO/enter.md) T-0504), then held as a
//!    directory descriptor so nothing can swap the path afterwards;
//! 2. the image lock is taken and handed to the payload
//!    ([`TODO/image.md`](../../../TODO/image.md) T-0204, T-0211), so a
//!    concurrent `rmi` cannot delete a rootfs a running container is executing
//!    out of;
//! 3. **every descriptor is opened before the root changes** (T-0501): a chroot
//!    cuts off every path outside the new root, and a child with no explicit
//!    stdio opens `/dev/null`, which an extracted rootfs does not have;
//! 4. the platform is checked against the host's, and a foreign one is either
//!    run through a registered `binfmt_misc` interpreter or **refused by name**
//!    (T-0506), never exec'd into a bare `Exec format error`;
//! 5. `fchdir`, `chroot(".")`, `chdir("/")`, and only **then** is the program
//!    resolved, in the process that changed the root (T-0502).
//!
//! ⛔ **The banner goes to stderr and the payload owns stdout.** T-1104's own
//! decision: a banner on stdout corrupts every pipeline the payload is in.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod abi;
pub mod binfmt;
pub mod device;
pub mod ladder;
pub mod memfd;
pub mod plan;
pub mod stage;
pub mod userland;

use std::io::Write;

use podbox_probe::sys::{self, CBuf};

pub use plan::{Plan, Program};

/// ⭐ **THE RUNG THIS CRATE ENTERS WHERE NOTHING ELSE APPLIES**, which is
/// not always the rung the probe selects.
///
/// [`crate::run`] performs `TOOL.md` section 6.5's sequence and nothing else: it
/// `chroot`s. It does not `unshare`, it mounts nothing, and it creates no
/// network or pid namespace, on any machine.
///
/// ⛔ **The banner must say THIS and not what the machine would permit**, or it
/// breaks [`TODO/cli.md`](../../../TODO/cli.md) T-0804 rule 4: no output may
/// imply namespaces, cgroups or devices exist when they do not. Measured on
/// 2026-09-09: this host's probe selects `namespace`, and `podbox run` printed
/// `mode=namespace (namespaces: as configured; mounts: full)` while entering a
/// plain chroot. The machine's own selection is still reported, beside this,
/// because "what podbox did" and "what this machine would permit" are two facts
/// and a reader needs both.
///
/// ⚠ The namespace rung (T-1339) does not move this constant: it stays the
/// rung entered where no other rung applies, and [`entered_rung`] maps a
/// namespace selection onto `Rung::Namespace`. One mapping, so the banner,
/// the record and `--strict` cannot disagree.
pub const ENTERED_RUNG: podbox_probe::select::Rung = podbox_probe::select::Rung::Chroot;

/// The rung `podbox run` enters with for a probe selection.
///
/// `userland` where the probe selects `interpose`: chroot is denied
/// there, so a run enters without it where a family holds, and refuses
/// where none does. `namespace` where the probe selects it (T-1339):
/// the mount namespace is attempted and chroot is the fallback, named
/// on the banner. Every other selection enters the chroot sequence.
pub fn entered_rung(selection: podbox_probe::select::Rung) -> podbox_probe::select::Rung {
    use podbox_probe::select::Rung as R;
    match selection {
        R::Interpose => R::Userland,
        R::Namespace => R::Namespace,
        _ => ENTERED_RUNG,
    }
}

/// The entered word for a probe selection.
///
/// `userland` where the probe selects `interpose`: chroot is denied
/// there, so a run enters without it where a family holds, and refuses
/// where none does. Every other selection enters its own rung's word.
/// TODO/enter.md T-1317.
pub fn entered_word(selection: podbox_probe::select::Rung) -> &'static str {
    entered_rung(selection).word()
}

// ⛔ **docker's codes, from the one file that holds them.**
// [`TODO/cli.md`](../../../TODO/cli.md) T-0802. This crate declared its own 125,
// 126 and 127 and `podbox-image` declared 125 again; a correction to one of them
// did not reach the others.
pub use podbox_probe::exit::{EXIT_CANNOT_INVOKE, EXIT_NOT_FOUND, EXIT_RUNTIME_ERROR};

#[derive(Debug)]
pub enum Error {
    /// Something podbox could not do. Exits 125.
    Runtime(String),
    /// The payload's command could not be invoked. Exits 126.
    CannotInvoke(String),
    /// The payload's command was not found. Exits 127.
    NotFound(String),
    /// The namespace setup failed before the chroot sequence. Never
    /// escapes: [`spawn_namespace`] catches it, enters chroot instead,
    /// and names the fallback on the banner. Exits 125 where it does.
    NsSetup {
        /// The readiness-pipe step that failed: unshare, privatise, mount.
        step: &'static str,
        errno: sys::Errno,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Runtime(s) | Error::CannotInvoke(s) | Error::NotFound(s) => write!(f, "{s}"),
            Error::NsSetup { step, errno } => write!(
                f,
                "namespace setup failed at {step}: {} ({})",
                errno.name(),
                errno.0
            ),
        }
    }
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Runtime(_) | Error::NsSetup { .. } => EXIT_RUNTIME_ERROR,
            Error::CannotInvoke(_) => EXIT_CANNOT_INVOKE,
            Error::NotFound(_) => EXIT_NOT_FOUND,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A directory descriptor on the rootfs, opened before anything changes.
///
/// ⛔ T-0504. `chroot` **follows a symlink**, so a rootfs path that is one puts
/// the payload somewhere podbox did not choose and every containment claim
/// afterwards is about a different directory. The path is `lstat`ed, refused if
/// it is a symlink, and then held open: `fchdir` onto a descriptor cannot be
/// redirected by anything that happens to the path in between.
#[derive(Debug)]
pub struct RootDir {
    fd: i64,
    pub path: String,
}

impl RootDir {
    pub fn open(path: &str) -> Result<RootDir> {
        let c = CBuf::new(path)
            .ok_or_else(|| Error::Runtime(format!("{path:?} contains a NUL byte")))?;

        // ⛔ `AT_SYMLINK_NOFOLLOW`, so this asks about the path itself and not
        // about whatever it points at.
        let st = sys::fstatat(sys::AT_FDCWD as i64, &c, sys::AT_SYMLINK_NOFOLLOW)
            .map_err(|e| Error::Runtime(format!("{path}: {} ({})", e.name(), e.0)))?;
        if st.st_mode & sys::S_IFMT == sys::S_IFLNK {
            let target = sys::readlink(&c).unwrap_or_else(|_| "(unreadable)".into());
            return Err(Error::Runtime(format!(
                "{path} is a symlink to {target}. podbox refuses it rather than \
                 resolving it: chroot follows a symlink, so entering it would \
                 put the payload somewhere podbox did not choose, and the \
                 ownership sidecar and the image lock belong to the store's own \
                 directory (TODO/enter.md T-0504)"
            )));
        }
        if st.st_mode & sys::S_IFMT != sys::S_IFDIR {
            return Err(Error::Runtime(format!("{path} is not a directory")));
        }

        let fd = sys::open(
            &c,
            sys::O_RDONLY | sys::O_DIRECTORY | sys::O_CLOEXEC | sys::O_NOFOLLOW,
            0,
        )
        .map_err(|e| Error::Runtime(format!("opening {path}: {} ({})", e.name(), e.0)))?;
        Ok(RootDir {
            fd,
            path: path.to_string(),
        })
    }

    pub fn fd(&self) -> i64 {
        self.fd
    }
}

impl Drop for RootDir {
    fn drop(&mut self) {
        let _ = sys::close(self.fd);
    }
}

/// What the payload will be given, resolved before the fork.
///
/// ⛔ T-0501. Everything the child touches after the `chroot` exists before it.
/// A descriptor opened before the root changes keeps working after it, and it is
/// the only thing that crosses the boundary on this runtime: there is no attach
/// path and nothing can be bind-mounted.
/// ⚠ The default is EMPTY, and that is the design rather than a stub: stdio is
/// inherited rather than re-opened, so the caller's stdout IS the payload's,
/// which is what makes `podbox run ... | consumer` work at all. What goes in
/// here is what a `--device` or a `-t` pty pair will add, opened before the
/// root changes.
#[derive(Default)]
pub struct Fds {
    /// `(child_fd, host_fd)` pairs, dup'd into place in the child.
    pub pass: Vec<(i64, i64)>,
}

/// Run a payload inside `root`, and return its exit status.
///
/// ⛔ **The exit status is the payload's**, not podbox's. T-0802: an automated
/// caller reads the exit code first, and a runtime that improves a payload's
/// code lies in the field read first.
pub fn run(root: &RootDir, plan: &Plan, err: &mut dyn Write) -> Result<i32> {
    let child = spawn(root, plan, err)?;
    child.wait_forwarding(err)
}

/// A payload that has reached its `execve`, and its pid.
///
/// ⛔ [`TODO/supervise.md`](../../../TODO/supervise.md) T-0602: this value only
/// exists once the exec has SUCCEEDED, established by reading the error pipe to
/// EOF rather than by sleeping and looking. A failure before the exec arrives on
/// that pipe as an errno and becomes an [`Error`] here, so a caller never has to
/// decide whether a process it cannot see yet is starting or already dead.
#[derive(Debug)]
pub struct Child {
    pub pid: i64,
}
/// How a bounded wait ended. ⛔ Three outcomes and not two: `TimedOut` is
/// `TODO/RULES.md` section 8's own rule, and a bound reached is neither a
/// failure nor an exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounded {
    /// docker's exit code for however it ended, as [`Child::wait`] reports it.
    Exited(i32),
    /// The bound was reached and the payload was killed. ⚠ It carries the
    /// bound rather than a code, because there is no code: nothing exited on
    /// its own.
    TimedOut { after_ms: i64 },
}

impl Child {
    /// Reap it, and return docker's exit code for however it ended.
    pub fn wait(&self) -> Result<i32> {
        let mut status = 0i32;
        loop {
            match sys::wait4(self.pid, &mut status) {
                Ok(_) => break,
                // ⚠ `EINTR` is not a failure. A signal delivered to podbox while
                // it waits must not be reported as the payload having died.
                Err(e) if e == sys::EINTR => continue,
                Err(e) => {
                    return Err(Error::Runtime(format!(
                        "wait4 on the payload failed: {} ({})",
                        e.name(),
                        e.0
                    )))
                }
            }
        }
        Ok(exit_status(status))
    }

    /// Reap it, forwarding this process's own SIGINT/SIGTERM to it first.
    ///
    /// TODO/supervise.md T-1335: a foreground waiter holds the terminal but
    /// the payload holds the work, so a Ctrl-C that kills only the waiter
    /// orphans the payload. The two signals are blocked and read from a
    /// signalfd beside the payload's pidfd in one `ppoll`, so the signal
    /// never kills this process between the decision and the forward, and a
    /// reused pid can never be signalled instead. Each forward and a
    /// signaled end are named on `err`; a clean exit prints nothing.
    ///
    /// ⚠ No threads, and none can be added here: the T-0603 assertion scans
    /// this crate, and a thread would make the clone below fork a threaded
    /// process. The wait is single-threaded by construction.
    ///
    /// ⚠ A payload that ignores the signal is waited on, not killed: killing
    /// it would be `stop`'s decision made inside `run`, and the caller that
    /// wants it dead already has `stop` and `kill`.
    ///
    /// ⚠ SIGINT is forwarded, never re-raised: the waiter stays alive to
    /// reap the payload, and the code it returns is the payload's own. A
    /// payload that dies on the default disposition still ends 130; one
    /// that traps or ignores SIGINT outlives it, and the waiter with it.
    pub fn wait_forwarding(&self, err: &mut dyn Write) -> Result<i32> {
        let mut old = 0u64;
        if let Err(e) = sys::sigblock_shutdown(&mut old) {
            return self.wait_unforwarded(
                err,
                &format!("the shutdown mask refused: {} ({})", e.name(), e.0),
            );
        }
        let sfd = match sys::signalfd_shutdown() {
            Ok(f) => f,
            Err(e) => {
                let _ = sys::sigrestore(old);
                return self
                    .wait_unforwarded(err, &format!("signalfd refused: {} ({})", e.name(), e.0));
            }
        };
        let pidfd = match sys::pidfd_open(self.pid) {
            Ok(f) => f,
            Err(e) => {
                let _ = sys::close(sfd);
                let _ = sys::sigrestore(old);
                return self
                    .wait_unforwarded(err, &format!("pidfd_open refused: {} ({})", e.name(), e.0));
            }
        };
        let code = self.forward_loop(pidfd, sfd, err);
        let _ = sys::close(pidfd);
        let _ = sys::close(sfd);
        let _ = sys::sigrestore(old);
        code
    }

    /// The old wait, named as the fallback: the forward could not be armed,
    /// so the payload is reaped with signals unforwarded rather than left.
    fn wait_unforwarded(&self, err: &mut dyn Write, why: &str) -> Result<i32> {
        let _ = writeln!(
            err,
            "waiting without shutdown forwarding ({why}); a SIGINT or SIGTERM \
             to this process will not reach the payload"
        );
        self.wait()
    }

    /// One `ppoll` over the payload's exit and this process's shutdown
    /// signals, re-polled on the bound: reaching it means nothing happened
    /// rather than anything being wrong (T-0602's shape, not a second one).
    fn forward_loop(&self, pidfd: i64, sfd: i64, err: &mut dyn Write) -> Result<i32> {
        loop {
            // ⛔ Fresh set every poll: `ppoll` leaves old `revents` standing,
            // and a stale one would reap or forward twice.
            let mut fds = [
                sys::PollFd {
                    fd: pidfd as i32,
                    events: sys::POLLIN,
                    revents: 0,
                },
                sys::PollFd {
                    fd: sfd as i32,
                    events: sys::POLLIN,
                    revents: 0,
                },
            ];
            match sys::ppoll(&mut fds, 60_000) {
                Ok(_) => {}
                Err(e) if e == sys::EINTR => continue,
                Err(_) => return self.wait(),
            }
            if fds[1].revents & (sys::POLLIN | sys::POLLHUP) != 0 {
                let mut buf = [0u8; 128];
                // ⛔ `ssi_signo` is the first word. A short read is not a
                // signal, and anything but SIGINT/SIGTERM cannot arrive: the
                // mask holds exactly those two bits.
                if let Ok(n) = sys::read(sfd, &mut buf) {
                    if n >= 4 {
                        let signo = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as i64;
                        if signo == sys::SIGINT as i64 || signo == sys::SIGTERM as i64 {
                            // ⚠ ESRCH is not a failure: the payload exited
                            // between the read and this kill, and the pidfd
                            // half reaps it below.
                            let _ = sys::kill(self.pid, signo);
                            let _ = writeln!(
                                err,
                                "forwarded {} to the payload (pid {})",
                                signal_name(signo),
                                self.pid
                            );
                        }
                    }
                }
            }
            if fds[0].revents & (sys::POLLIN | sys::POLLHUP) != 0 {
                match sys::waitid_pidfd(pidfd, false) {
                    Ok(Some(x)) if x.code == sys::CLD_EXITED => return Ok(x.status),
                    Ok(Some(x)) => {
                        let code = 128 + x.status;
                        let _ = writeln!(
                            err,
                            "the payload died on {}: exit {code}",
                            signal_name(x.status as i64)
                        );
                        return Ok(code);
                    }
                    // Spurious readiness: re-poll rather than verdict.
                    Ok(None) => {}
                    Err(_) => return self.wait(),
                }
            }
        }
    }

    /// Reap it, but never wait longer than `ms`.
    ///
    /// ⭐ **`ppoll` on a pidfd, which is `TODO/supervise.md` T-0601 and T-0602's
    /// mechanism rather than a second one.** A pid plus a sleep is a heuristic;
    /// a pidfd names one process and `ppoll` waits on the CONDITION with an
    /// upper bound, so there is no interval to be wrong about and no window in
    /// which a reused pid could be signalled instead.
    ///
    /// ⛔ **On the bound the child is `SIGKILL`ed and then reaped.** A step
    /// podbox started inside somebody else's image and then walked away from
    /// would keep the rootfs busy and hold the image lock, and `AGENTS.md`
    /// makes bounded waits a requirement of podbox itself and not only of the
    /// agent working on it.
    pub fn wait_bounded(&self, ms: i64) -> Result<Bounded> {
        let pidfd = sys::pidfd_open(self.pid).map_err(|e| {
            Error::Runtime(format!(
                "pidfd_open on pid {} failed: {} ({}); podbox will not wait on a \
                 bare pid",
                self.pid,
                e.name(),
                e.0
            ))
        })?;
        let mut fds = [sys::PollFd {
            fd: pidfd as i32,
            events: sys::POLLIN,
            revents: 0,
        }];
        let ready = loop {
            match sys::ppoll(&mut fds, ms) {
                Ok(n) => break n,
                // ⚠ `EINTR` is not the bound. A signal delivered to podbox while
                // it waits must not read as the step having hung.
                Err(e) if e == sys::EINTR => continue,
                Err(e) => {
                    let _ = sys::close(pidfd);
                    return Err(Error::Runtime(format!(
                        "ppoll on the step's pidfd failed: {} ({})",
                        e.name(),
                        e.0
                    )));
                }
            }
        };
        if ready == 0 {
            let _ = sys::kill(self.pid, 9);
        }
        let ended = sys::waitid_pidfd(pidfd, false);
        let _ = sys::close(pidfd);
        if ready == 0 {
            return Ok(Bounded::TimedOut { after_ms: ms });
        }
        match ended {
            Ok(Some(e)) if e.code == sys::CLD_EXITED => Ok(Bounded::Exited(e.status)),
            // ⛔ 128+signal, which is `exit_status`'s own rule and docker's: a
            // killed step reporting 0 or 1 is the lie T-0802 is about.
            Ok(Some(e)) => Ok(Bounded::Exited(128 + e.status)),
            Ok(None) => Err(Error::Runtime(
                "waitid reported no status for a pidfd that was ready".into(),
            )),
            Err(e) => Err(Error::Runtime(format!(
                "waitid on the step failed: {} ({})",
                e.name(),
                e.0
            ))),
        }
    }
}

/// Enter `root` and exec the plan, returning as soon as the exec has succeeded.
///
/// ⛔ **The readiness signal is an `O_CLOEXEC` pipe, and it is the mechanism
/// T-0602 asks for.** The child holds the write end; a successful `execve`
/// closes it and the parent's read returns EOF, and any failure before the exec
/// is written to it as an errno. There is no interval anywhere in it, so there
/// is no scheduling assumption to be wrong about.
///
/// ⭐ **THE CHROOT-BY-PATH ENTRY.** The payload sees `PODBOX_ACTIVE_MODE`
/// naming [`ENTERED_RUNG`], whatever the ladder chose: this is the rung the
/// sequence implements, and [`spawn_ladder`] is the one that reports a rung.
pub fn spawn(root: &RootDir, plan: &Plan, err: &mut dyn Write) -> Result<Child> {
    spawn_with(root, plan, ENTERED_RUNG.word(), None, None, true, None, err)
}

/// Enter `root` on a ladder rung and exec the plan, returning on the exec.
///
/// ⭐ T-1003's drive beside [`spawn`]: the same sequence, except the payload
/// sees `PODBOX_ACTIVE_MODE` naming the ladder rung and the child execs the
/// memfd where one was handed in (`Some(fd)`), or the path candidates where
/// the payload routes past fd-exec (`None`: a `#!` script, which fails
/// through it).
///
/// ⛔ Only rung-complete rungs drive through here: memfd, the run
/// directory and the tmpfs mount the CLI stages before calling, and the
/// persistent cache (FUSE is ordered and refused by `ladder::choose`).
/// Reaching this call with an unwired rung is the caller skipping the
/// choice, so it refuses rather than exec'ing down a rung nobody drove.
pub fn spawn_ladder(
    root: &RootDir,
    plan: &Plan,
    mode: ladder::Mode,
    fd: Option<i64>,
    err: &mut dyn Write,
) -> Result<Child> {
    use ladder::Mode as M;
    match mode {
        M::Memfd | M::RunDir | M::Cache | M::Tmpfs => {}
        M::Fuse => {
            return Err(Error::Runtime(format!(
                "the {} rung is ordered by the ladder but not rung-complete: only \
                 rung-complete rungs drive through this entry (TODO/packaging.md T-1003)",
                mode.name()
            )));
        }
    }
    spawn_with(root, plan, mode.name(), None, fd, true, None, err)
}

/// Enter `root` without `chroot(2)` and exec, returning on the exec.
///
/// TODO/enter.md T-1317. The payload runs with the host's root and its
/// working directory inside the image: `exec_argv` is the exact argv the
/// child execs (the image's loader, the payload by host path, then the
/// payload's own arguments), or `plan.argv` where `fd_exec` carries a
/// staged memfd. `active` is the word the payload reads as
/// `PODBOX_ACTIVE_MODE`. The readiness pipe, the exit codes and the
/// forwarding wait are the chroot entry's: one fork shape, not two.
pub fn spawn_userland(
    root: &RootDir,
    plan: &Plan,
    exec_argv: Vec<String>,
    active: &str,
    fd_exec: Option<i64>,
    err: &mut dyn Write,
) -> Result<Child> {
    if fd_exec.is_none() && exec_argv.is_empty() {
        return Err(Error::Runtime(
            "the userland entry was handed no argv and no memfd: there is nothing to exec".into(),
        ));
    }
    let override_argv: Vec<CBuf> = exec_argv
        .iter()
        .map(|a| {
            CBuf::new(a)
                .ok_or_else(|| Error::Runtime(format!("the argument {a:?} contains a NUL byte")))
        })
        .collect::<Result<_>>()?;
    spawn_with(
        root,
        plan,
        active,
        Some(override_argv),
        fd_exec,
        false,
        None,
        err,
    )
}

/// Run a payload inside `root` without `chroot(2)`, and return its exit status.
///
/// ⛔ Like [`run`]: the exit status is the payload's, not podbox's.
pub fn run_userland(
    root: &RootDir,
    plan: &Plan,
    exec_argv: Vec<String>,
    active: &str,
    fd_exec: Option<i64>,
    err: &mut dyn Write,
) -> Result<i32> {
    let child = spawn_userland(root, plan, exec_argv, active, fd_exec, err)?;
    // The parent's copy of the memfd serves nothing past the fork: the
    // child execs from its own. It closes here, on the only path that
    // holds one, so no caller leaks a descriptor.
    if let Some(f) = fd_exec {
        let _ = sys::close(f);
    }
    child.wait_forwarding(err)
}

/// Run a payload inside `root` on a ladder rung, and return its exit status.
///
/// ⛔ Like [`run`]: the exit status is the payload's, not podbox's.
pub fn run_ladder(
    root: &RootDir,
    plan: &Plan,
    mode: ladder::Mode,
    fd: Option<i64>,
    err: &mut dyn Write,
) -> Result<i32> {
    let r = spawn_ladder(root, plan, mode, fd, err);
    // The parent's copy of the memfd serves nothing past the fork: the child
    // execs from its own. It closes on every path, including the refusal
    // ones, so a refused launch leaks no descriptor.
    if let Some(f) = fd {
        let _ = sys::close(f);
    }
    let child = r?;
    child.wait_forwarding(err)
}

/// What the namespace rung mounts, built before the fork.
///
/// One private tmpfs on the image's `/tmp`: the mount a run isolates
/// from the host (TODO/enter.md T-1339's Prove). No user, pid or
/// network namespace and no `/proc`: out of scope beside this rung,
/// and the banner says so.
pub struct NsMount {
    tmp_target: CBuf,
}

/// The image's `/tmp` as a mount target, or `None` where it cannot be
/// one: missing, a file, or a symlink. Mounting over a link would land
/// on its target, which is outside the image, so a link refuses like a
/// missing directory. `None` enters chroot with the fallback named,
/// never a namespace rung without its mount: the banner's mount
/// topology is fixed text, and a rung without the mount would print it
/// falsely.
pub fn ns_tmp_target(root: &str) -> Option<String> {
    let target = format!("{}/tmp", root.trim_end_matches('/'));
    let not_link = std::fs::symlink_metadata(&target)
        .map(|m| !m.is_symlink())
        .unwrap_or(false);
    let is_dir = std::fs::metadata(&target)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    (not_link && is_dir).then_some(target)
}

/// The fallback line: the cause, then the rung actually entered.
/// Pure, so the drive and the unit test read the same words.
pub fn ns_fallback_line(cause: &str) -> String {
    format!("podbox: {cause}; entered chroot instead (TODO/enter.md T-1339)")
}

/// Whether the chroot landed on the opened descriptor: same device and
/// same file. Pure, so the child and the unit test read the same words.
///
/// TODO/enter.md T-1339: the (d,i) guard past the namespace rung's path
/// chroot. A path checked and then passed can be swapped in between
/// (T-0504's class); a mismatch here refuses before the exec rather
/// than entering wrong.
pub fn chroot_landed(anchored: &sys::Stat, landed: &sys::Stat) -> bool {
    anchored.st_dev == landed.st_dev && anchored.st_ino == landed.st_ino
}

/// The namespace setup step a readiness-pipe byte names, or `None`
/// where the byte is not a setup step. Pure, so the parent and the
/// unit test read the same words: a step the parent cannot name is a
/// fallback the banner cannot explain (TODO/enter.md T-1339).
pub fn ns_setup_step(step: u8) -> Option<&'static str> {
    match step {
        7 => Some("unshare(CLONE_NEWNS)"),
        8 => Some("remounting / recursively private"),
        9 => Some("mounting tmpfs on /tmp"),
        10 => Some("verifying the chroot landed on the opened rootfs"),
        _ => None,
    }
}

/// Enter `root` in a mount namespace and exec, falling back to chroot.
///
/// The child unshares a mount namespace, remounts `/` recursively
/// private so its mounts cannot propagate to the host, mounts its
/// private tmpfs on the image's `/tmp`, and then runs the chroot
/// sequence inside. Any setup failure abandons the namespace and
/// enters chroot instead with the fallback named: a host that loses
/// a leg between probe and entry still runs what the chroot rung
/// could carry (TODO/enter.md T-1339's Decision).
pub fn spawn_namespace(root: &RootDir, plan: &Plan, err: &mut dyn Write) -> Result<Child> {
    let target = match ns_tmp_target(&root.path) {
        Some(t) => t,
        None => {
            let _ = writeln!(
                err,
                "{}",
                ns_fallback_line(&format!(
                    "the image holds no /tmp mount point under {}",
                    root.path
                ))
            );
            return spawn_with(root, plan, ENTERED_RUNG.word(), None, None, true, None, err);
        }
    };
    let target = match CBuf::new(&target) {
        Some(t) => t,
        None => {
            let _ = writeln!(
                err,
                "{}",
                ns_fallback_line("the /tmp mount point contains a NUL byte")
            );
            return spawn_with(root, plan, ENTERED_RUNG.word(), None, None, true, None, err);
        }
    };
    let mount = NsMount { tmp_target: target };
    match spawn_with(
        root,
        plan,
        podbox_probe::select::Rung::Namespace.word(),
        None,
        None,
        true,
        Some(&mount),
        err,
    ) {
        Err(Error::NsSetup { step, errno }) => {
            let _ = writeln!(
                err,
                "{}",
                ns_fallback_line(&format!(
                    "the namespace rung refused at {step} ({} ({}))",
                    errno.name(),
                    errno.0
                ))
            );
            spawn_with(root, plan, ENTERED_RUNG.word(), None, None, true, None, err)
        }
        other => other,
    }
}

/// Run a payload inside `root` in a mount namespace, and return its exit status.
///
/// ⛔ Like [`run`]: the exit status is the payload's, not podbox's.
pub fn run_namespace(root: &RootDir, plan: &Plan, err: &mut dyn Write) -> Result<i32> {
    let child = spawn_namespace(root, plan, err)?;
    child.wait_forwarding(err)
}

/// Enter `root` on the rung the probe selected: the namespace rung
/// where selected, the chroot sequence everywhere else. One gate for
/// every entry path (foreground `run`, detached `start`): a rung
/// decided in one place and entered in another is the hole
/// `docs/conventions/code.md` refuses.
///
/// ⚠ Userland has its own entries and never passes through here: it is
/// the no-chroot family, not a rung of this sequence.
pub fn spawn_selected(
    root: &RootDir,
    plan: &Plan,
    rung: podbox_probe::select::Rung,
    err: &mut dyn Write,
) -> Result<Child> {
    use podbox_probe::select::Rung as R;
    match rung {
        R::Namespace => spawn_namespace(root, plan, err),
        _ => spawn(root, plan, err),
    }
}

/// Run a payload inside `root` on the rung the probe selected.
///
/// ⛔ Like [`run`]: the exit status is the payload's, not podbox's.
pub fn run_selected(
    root: &RootDir,
    plan: &Plan,
    rung: podbox_probe::select::Rung,
    err: &mut dyn Write,
) -> Result<i32> {
    let child = spawn_selected(root, plan, rung, err)?;
    child.wait_forwarding(err)
}

/// The one entry sequence [`spawn`], [`spawn_ladder`] and [`spawn_userland`] share.
///
/// `active` is the word the payload reads as `PODBOX_ACTIVE_MODE`, and
/// `fd_exec` is the written memfd the child execs where one was handed in.
/// `argv_exec` is the exact argv the child execs instead of resolving
/// candidates (the userland loader invocation); `chroot` selects the
/// chroot sequence or the fchdir-only one; `ns` carries the namespace
/// rung's mount or nothing. One function, so the fork, the
/// root discipline and the readiness pipe cannot drift between the three
/// entries (`docs/conventions/code.md`).
///
/// ⚠ Eight parameters, each one used. They are the fork shape's fixed
/// set (everything the child touches is built before it), not a bundle
/// looking for a struct.
#[allow(clippy::too_many_arguments)]
fn spawn_with(
    root: &RootDir,
    plan: &Plan,
    active: &str,
    argv_exec: Option<Vec<CBuf>>,
    fd_exec: Option<i64>,
    chroot: bool,
    ns: Option<&NsMount>,
    err: &mut dyn Write,
) -> Result<Child> {
    // ---------------------------------------------------------- before the fork
    //
    // ⛔ Every allocation the child needs happens HERE. Between `clone` and
    // `execve` only async-signal-safe work is permitted, so nothing below the
    // fork may allocate, format a string, or take a lock.
    let argv_owned: Vec<CBuf> = plan
        .argv
        .iter()
        .map(|a| {
            CBuf::new(a)
                .ok_or_else(|| Error::Runtime(format!("the argument {a:?} contains a NUL byte")))
        })
        .collect::<Result<_>>()?;
    let envp_owned: Vec<CBuf> = {
        // ⭐ T-1003. The request variable never crosses the exec, whatever built
        // the plan: [`Plan::env_for`] scrubs what it builds, and this filters
        // what it is handed, so a hand-built plan cannot leak one either. The
        // rung actually entered is reported under a different name beside it,
        // so a nested `podbox run` starts unforced. Which name that is depends
        // on the entry: [`spawn`] reports [`ENTERED_RUNG`], [`spawn_ladder`]
        // reports the ladder rung that drove.
        let active_var = format!("{}={active}", Plan::MODE_ACTIVE_VAR);
        plan.env
            .iter()
            .filter(|e| {
                let name = e.split('=').next().unwrap_or("");
                name != Plan::MODE_REQUEST_VAR && name != Plan::MODE_ACTIVE_VAR
            })
            .chain(std::iter::once(&active_var))
            .map(|e| {
                CBuf::new(e)
                    .ok_or_else(|| Error::Runtime(format!("the environment {e:?} contains a NUL")))
            })
            .collect::<Result<_>>()?
    };
    let workdir = CBuf::new(if plan.working_dir.is_empty() {
        "/"
    } else {
        &plan.working_dir
    })
    .ok_or_else(|| Error::Runtime("the working directory contains a NUL byte".into()))?;
    // ⭐ T-1317. Without a chroot the working directory is addressed
    // relatively from the rootfs descriptor the child fchdirs onto: the
    // guest-absolute path minus its leading slash. A NUL byte falls back
    // to the rootfs itself rather than refusing past allocation time.
    let workdir_rel: Option<CBuf> = if chroot {
        None
    } else {
        let rel = plan.working_dir.trim_start_matches('/');
        if rel.is_empty() {
            None
        } else {
            CBuf::new(rel)
        }
    };
    let slash = CBuf::new("/").expect("a literal");
    let dot = CBuf::new(".").expect("a literal");
    // ⭐ T-1339. The fstype and source the namespace rung mounts, built
    // before the fork like everything else the child touches.
    let tmpfs = CBuf::new("tmpfs").expect("a literal");
    // ⭐ T-1003. The empty path `execveat` execs a descriptor through, built
    // before the fork like everything else the child touches: between `clone`
    // and `execve` only async-signal-safe work is permitted.
    let empty = sys::cempty();
    // ⭐ T-1339. The rootfs path the namespace rung chroots by, built
    // before the fork like everything else the child touches. The
    // entry fd was opened before the fork in the base mount namespace,
    // and fchdir+chroot(".") anchors the new root in that mount: the
    // tmpfs this child mounts in its own namespace stays invisible
    // under it (measured on the wsl-toolkit base, kernel
    // 7.2.0-WSL2-STABLE). The path resolves in this namespace, under
    // the private mount. `rootpath` is that path; the (d,i) guard
    // past the chroot refuses a swapped one.
    let rootpath = CBuf::new(&root.path).expect("root path has no NUL");

    let mut argv: Vec<*const u8> = argv_owned.iter().map(|c| c.ptr() as *const u8).collect();
    argv.push(std::ptr::null());
    let mut envp: Vec<*const u8> = envp_owned.iter().map(|c| c.ptr() as *const u8).collect();
    envp.push(std::ptr::null());
    // ⭐ T-1317. The exact argv's pointer table, built here for the same
    // reason: nothing below the fork may allocate.
    let argv_exec_ptrs: Option<Vec<*const u8>> = argv_exec.as_ref().map(|ov| {
        let mut v: Vec<*const u8> = ov.iter().map(|c| c.ptr() as *const u8).collect();
        v.push(std::ptr::null());
        v
    });

    // ⭐ T-0502. The program is resolved INSIDE the new root, so the candidate
    // paths are built here and tried there. A parent-resolved absolute path
    // names the outer tree, and the outer tree is gone after the chroot.
    let candidates: Vec<CBuf> = plan
        .program_candidates()
        .into_iter()
        .filter_map(|p| CBuf::new(&p))
        .collect();
    if candidates.is_empty() {
        return Err(Error::NotFound(format!(
            "{:?}: no candidate path could be built for it",
            plan.argv.first().map(String::as_str).unwrap_or("")
        )));
    }

    // ⛔ The banner before the fork, so it cannot interleave with the payload's
    // own first output. T-1104: stderr, never stdout.
    let _ = write!(err, "{}", plan.banner);
    let _ = err.flush();

    // ⭐ T-0602's readiness channel, created before the fork like everything
    // else the child touches. `O_CLOEXEC` on both ends is the whole trick: the
    // `execve` closes the write end, and the parent's read then returns EOF.
    let mut pipe = [0i32; 2];
    sys::pipe2(&mut pipe, sys::O_CLOEXEC).map_err(|e| {
        Error::Runtime(format!(
            "pipe2 for the readiness channel failed: {} ({})",
            e.name(),
            e.0
        ))
    })?;
    let (read_end, write_end) = (pipe[0] as i64, pipe[1] as i64);

    // --------------------------------------------------------------- the fork
    let pid = match unsafe { sys::clone_fork(sys::SIGCHLD) } {
        Ok(0) => {
            // ------ child. Async-signal-safe work only, then execve. ------
            let _ = sys::close(read_end);
            // ⚠ Four bytes to the readiness pipe, which is async-signal-safe and
            // is all the parent needs to tell a failure from a successful exec.
            let report = |step: u8, e: sys::Errno| -> ! {
                let msg = [step, (e.0 & 0xff) as u8, ((e.0 >> 8) & 0xff) as u8, 0];
                let _ = sys::write(write_end, &msg);
                sys::exit_group(EXIT_RUNTIME_ERROR)
            };
            for (child_fd, host_fd) in &plan.fds.pass {
                if host_fd != child_fd {
                    let _ = sys::dup2(*host_fd, *child_fd);
                    let _ = sys::close(*host_fd);
                }
            }
            // ⭐ T-1339. The namespace rung, inside the child: a mount
            // namespace of its own, `/` recursively private so its
            // mounts cannot propagate to the host, then its private
            // tmpfs on the image's `/tmp`. Any failure reports its step
            // and the parent enters chroot instead; nothing here may
            // allocate, format, or take a lock.
            if let Some(m) = ns {
                if let Err(e) = sys::unshare(sys::CLONE_NEWNS) {
                    report(7, e);
                }
                if let Err(e) = sys::mount(&empty, &slash, &empty, sys::MS_REC | sys::MS_PRIVATE) {
                    report(8, e);
                }
                if let Err(e) = sys::mount(&tmpfs, &m.tmp_target, &tmpfs, 0) {
                    report(9, e);
                }
            }
            // ⛔ `fchdir` onto the checked descriptor: a path checked and
            // then passed can be swapped in between, and a descriptor
            // cannot (TODO/enter.md T-0504). The working directory is the
            // rootfs from here until `chdir` moves it below.
            if let Err(e) = sys::fchdir(root.fd) {
                report(1, e);
            }
            if chroot {
                if ns.is_some() {
                    // ⭐ T-1339. The path chroot on the namespace rung. The
                    // entry descriptor was opened before the fork, in the
                    // base mount namespace, and `fchdir` plus `chroot(".")`
                    // anchors the new root in that mount: the tmpfs this
                    // child mounted in its own namespace stays invisible
                    // under it. Measured on the wsl-toolkit base, kernel
                    // 7.2.0-WSL2-STABLE: the child's own stat of the mount
                    // target read tmpfs while `/tmp` past a dot chroot read
                    // the host directory, with the (d,i) below agreeing on
                    // the same directory throughout. The path resolves in
                    // this namespace, under the private mount. The (d,i)
                    // guard below refuses a swapped path rather than
                    // entering it, so T-0504's discipline holds here too.
                    if let Err(e) = sys::chroot(&rootpath) {
                        report(2, e);
                    }
                } else if let Err(e) = sys::chroot(&dot) {
                    report(2, e);
                }
                // ⛔ `chdir("/")` after the chroot. Without it the working directory
                // is still the old root's inode, which is a documented way out of a
                // chroot and is not a containment podbox may claim.
                if let Err(e) = sys::chdir(&slash) {
                    report(3, e);
                }
                // The image's WorkingDir, if it exists. ⚠ A missing one is not
                // fatal: docker creates it, and podbox running from `/` and saying
                // so is better than refusing after the point of no return.
                let _ = sys::chdir(&workdir);
                // ⭐ T-1339. The (d,i) guard: the path chroot above must
                // land on the opened descriptor. A mismatch is a swapped
                // path (T-0504's class) or a mount the new root cannot
                // see, and it refuses here, before the exec, rather than
                // entering wrong: the parent falls back to chroot with
                // the step named. `fstatat` on the descriptor needs no
                // path, so it answers past the chroot; `EXDEV` names a
                // landing off its file. `slash`, not `dot`: the working
                // directory below may be the image's WorkingDir.
                if ns.is_some() {
                    let anchored = sys::fstatat(root.fd, &empty, sys::AT_EMPTY_PATH).ok();
                    let landed = sys::fstatat(sys::AT_FDCWD as i64, &slash, 0).ok();
                    if !matches!((anchored, landed), (Some(a), Some(b)) if chroot_landed(&a, &b)) {
                        report(10, sys::EXDEV);
                    }
                }
            } else {
                // ⭐ T-1317. No chroot: the working directory stays inside
                // the image, addressed relatively from the descriptor above.
                // A missing one is not fatal either: podbox runs from the
                // rootfs and the banner says where the payload started.
                if let Some(w) = &workdir_rel {
                    let _ = sys::chdir(w);
                }
            }

            // ⭐ Resolved HERE, in the process that changed the root. Where the
            // ladder handed a written memfd in, it is exec'd without resolving
            // a path at all; the chroot above still holds beneath it.
            if let Some(mfd) = fd_exec {
                if let Err(e) =
                    unsafe { crate::memfd::exec_fd(mfd, &empty, argv.as_ptr(), envp.as_ptr()) }
                {
                    let msg = [5u8, (e.0 & 0xff) as u8, ((e.0 >> 8) & 0xff) as u8, 0];
                    let _ = sys::write(write_end, &msg);
                }
                sys::exit_group(EXIT_NOT_FOUND)
            }
            // ⭐ T-1317. The exact argv, exec'd once: the loader names its own
            // failures, so there is no candidate list to walk. A memfd above
            // takes precedence: it is the staged bytes, not a path.
            if let Some(ov) = &argv_exec {
                let ptrs = argv_exec_ptrs
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(std::ptr::null());
                if let Err(e) = unsafe { sys::execve(&ov[0], ptrs, envp.as_ptr()) } {
                    let msg = [6u8, (e.0 & 0xff) as u8, ((e.0 >> 8) & 0xff) as u8, 0];
                    let _ = sys::write(write_end, &msg);
                }
                sys::exit_group(EXIT_NOT_FOUND)
            }
            let mut last = sys::Errno(2i32);
            for c in &candidates {
                if let Err(e) = unsafe { sys::execve(c, argv.as_ptr(), envp.as_ptr()) } {
                    last = e;
                }
            }
            // Every candidate failed. 127 is the shell's convention and
            // docker's for "not found", and the parent reports it unchanged.
            let msg = [4u8, (last.0 & 0xff) as u8, ((last.0 >> 8) & 0xff) as u8, 0];
            let _ = sys::write(write_end, &msg);
            sys::exit_group(EXIT_NOT_FOUND)
        }
        Ok(pid) => pid,
        Err(e) => {
            let _ = sys::close(read_end);
            let _ = sys::close(write_end);
            return Err(Error::Runtime(format!(
                "clone(2) for the payload failed: {} ({})",
                e.name(),
                e.0
            )));
        }
    };

    // -------------------------------------------------------------- the parent
    // ⛔ The parent closes ITS write end first, or the read below never sees
    // EOF: the pipe stays open on this process's own copy of it forever.
    let _ = sys::close(write_end);
    let mut buf = [0u8; 4];
    let got = loop {
        match sys::read(read_end, &mut buf) {
            Ok(n) => break n,
            Err(e) if e == sys::EINTR => continue,
            Err(_) => break 0,
        }
    };
    let _ = sys::close(read_end);
    if got > 0 {
        // The child failed before its exec and said where. It has already
        // exited, so it is reaped here rather than left as a zombie.
        let mut st = 0i32;
        let _ = sys::wait4(pid, &mut st);
        let errno = sys::Errno(i32::from(buf[1]) | (i32::from(buf[2]) << 8));
        // ⭐ T-1339. The namespace setup never fails the run: the caller
        // enters chroot instead, so these steps arrive as `NsSetup`
        // rather than as a refusal. The step names live in
        // [`ns_setup_step`], beside the unit test that pins them.
        if let Some(step) = ns_setup_step(buf[0]) {
            return Err(Error::NsSetup { step, errno });
        }
        let step = match buf[0] {
            1 => "fchdir onto the rootfs descriptor",
            2 => "chroot into the rootfs",
            3 => "chdir(\"/\") after the chroot",
            // ⭐ T-1003: the ladder's fd-exec, which never resolves a path.
            5 => "execveat of the memfd",
            // ⭐ T-1317: the userland entry argv, exec'd once.
            6 => "execve of the userland entry argv",
            _ => "execve of every candidate path",
        };
        let text = format!(
            "the payload could not be started: {step} failed with {} ({})",
            errno.name(),
            errno.0
        );
        let named = format!(
            "{:?}: {text}",
            plan.argv.first().map(String::as_str).unwrap_or("")
        );
        return Err(if buf[0] != 4 && buf[0] != 5 && buf[0] != 6 {
            Error::Runtime(text)
        } else if invocable_but_refused(errno) {
            // ⭐ 126 and not 127, and the difference is docker's. Measured by
            // `experiments/330-exit-codes.sh` on 2026-09-09: `docker run alpine
            // /etc/passwd` exits **126** and podbox exited 127, because every
            // execve failure was folded into "not found". A caller branching on
            // 127 retries with a different path; one branching on 126 does not.
            Error::CannotInvoke(named)
        } else {
            Error::NotFound(named)
        });
    }
    Ok(Child { pid })
}

/// Was the file there and refused, rather than absent?
///
/// ⛔ **The errno decides, not a guess.** `execve` answers `ENOENT` for a path
/// that is not there -- and also for a dynamic loader the binary names and that
/// is not there, which is the trap -- while `EACCES`, `ENOEXEC`, `EISDIR`,
/// `EPERM` and `ETXTBSY` all mean the file was found and could not be run.
/// docker's codes split exactly there: 127 for the first, 126 for the rest.
///
/// ⚠ `ELOOP` and `ENAMETOOLONG` are `NotFound`'s side deliberately: both are
/// answers about resolving the path rather than about the file at the end of it.
fn invocable_but_refused(e: sys::Errno) -> bool {
    matches!(e.0, 1 | 8 | 13 | 21 | 26)
}

/// docker's translation of a wait status into an exit code.
///
/// ⛔ A signalled payload exits `128 + signal`, which is the shell's convention
/// and docker's. Reporting 0 or 1 for a killed process is the lie T-0802 is
/// about: a caller that branches on the code cannot tell a clean exit from a
/// SIGKILL.
pub fn exit_status(status: i32) -> i32 {
    if status & 0x7f == 0 {
        (status >> 8) & 0xff
    } else {
        128 + (status & 0x7f)
    }
}

/// The word the forwarding wait prints for a signal: the name for the two
/// it forwards, `signal <n>` for anything the reap reports that it did not
/// send (a terminal's SIGINT arrives both ways, so the reap can name a
/// signal the forward never saw).
fn signal_name(signo: i64) -> String {
    match signo {
        n if n == sys::SIGINT as i64 => "SIGINT".to_string(),
        n if n == sys::SIGTERM as i64 => "SIGTERM".to_string(),
        n => format!("signal {n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_status_becomes_dockers_exit_code() {
        // exited(0), exited(3), and SIGKILL(9).
        assert_eq!(exit_status(0), 0);
        assert_eq!(exit_status(3 << 8), 3);
        assert_eq!(exit_status(9), 137, "a SIGKILLed payload is 128+9");
        assert_eq!(exit_status(15), 143, "a SIGTERMed payload is 128+15");
    }

    #[test]
    fn forwarded_signals_are_named_and_unknown_ones_numbered() {
        assert_eq!(signal_name(2), "SIGINT");
        assert_eq!(signal_name(15), "SIGTERM");
        assert_eq!(signal_name(9), "signal 9");
    }

    #[test]
    fn the_shutdown_mask_holds_exactly_the_forwarded_pair() {
        // ⛔ Signal NUMBER n is bit index (n-1): SIGINT is bit 1 (value 2)
        // and SIGTERM is bit 14 (value 16384), so the word is 16386. The
        // lookalike `(1 << SIGINT) | (1 << SIGTERM)` names bits 2 and 15
        // (SIGQUIT and SIGSTKFLT): a mask with the wrong bits blocks
        // nothing that arrives, and the waiter dies on the real signal.
        // Measured 2026-09-23: the wrong word made the lane prove fail
        // every forwarded clause with empty stderr (T-1335).
        assert_eq!(sys::SHUTDOWN_SIGNALS, 2 | 16384);
    }

    #[test]
    fn a_symlinked_rootfs_is_refused_and_names_its_target() {
        let dir = std::env::temp_dir().join(format!("podbox-enter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(dir.join("real"), &link).unwrap();

        let e = RootDir::open(link.to_str().unwrap()).unwrap_err();
        let text = format!("{e}");
        assert!(text.contains("is a symlink"), "{text}");
        assert!(text.contains("T-0504"), "{text}");
        // ⛔ And the real directory is accepted, so the check is about the
        // symlink and not about the path being rejected wholesale.
        assert!(RootDir::open(dir.join("real").to_str().unwrap()).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rootfs_that_is_not_a_directory_is_refused() {
        let dir = std::env::temp_dir().join(format!("podbox-enter-f-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("file");
        std::fs::write(&f, b"x").unwrap();
        let e = RootDir::open(f.to_str().unwrap()).unwrap_err();
        assert!(format!("{e}").contains("not a directory"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TODO/enter.md T-1339: the entered rung follows the selection.
    /// Namespace enters namespace now, not chroot; interpose still
    /// enters userland and everything else the chroot sequence.
    #[test]
    fn the_entered_rung_follows_the_selection() {
        use podbox_probe::select::Rung as R;
        assert_eq!(entered_rung(R::Namespace), R::Namespace);
        assert_eq!(entered_rung(R::Interpose), R::Userland);
        assert_eq!(entered_rung(R::Chroot), R::Chroot);
        assert_eq!(entered_word(R::Namespace), "namespace");
        assert_eq!(entered_word(R::Chroot), "chroot");
    }

    /// TODO/enter.md T-1339: the /tmp mount point is a real directory.
    /// Missing, a file, or a symlink (whose target is outside the
    /// image) is no mount point, and the entry falls back.
    #[test]
    fn the_tmp_mount_point_is_a_real_directory() {
        let dir = std::env::temp_dir().join(format!("podbox-ns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.to_str().unwrap();
        assert!(ns_tmp_target(root).is_none(), "no /tmp yet");
        std::fs::create_dir_all(dir.join("tmp")).unwrap();
        assert_eq!(ns_tmp_target(root), Some(format!("{root}/tmp")));
        std::fs::remove_dir(dir.join("tmp")).unwrap();
        std::fs::write(dir.join("tmp"), b"x").unwrap();
        assert!(ns_tmp_target(root).is_none(), "a file is no mount point");
        std::fs::remove_file(dir.join("tmp")).unwrap();
        std::os::unix::fs::symlink("/tmp", dir.join("tmp")).unwrap();
        assert!(ns_tmp_target(root).is_none(), "a link is no mount point");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TODO/enter.md T-1339: the fallback names the cause and the rung
    /// actually entered, in the words the drive asserts.
    #[test]
    fn the_fallback_line_names_the_cause_and_the_rung() {
        let line =
            ns_fallback_line("the namespace rung refused at unshare(CLONE_NEWNS) (EPERM (1))");
        assert!(line.contains("unshare(CLONE_NEWNS)"), "{line}");
        assert!(line.contains("entered chroot instead"), "{line}");
        assert!(line.contains("T-1339"), "{line}");
    }

    /// TODO/enter.md T-1339: every setup step has a name for the
    /// fallback line, and no other byte reads as one. The parent maps
    /// through this function, so a step it cannot name cannot arrive
    /// unnamed on the banner.
    #[test]
    fn the_setup_steps_name_themselves() {
        assert_eq!(ns_setup_step(7), Some("unshare(CLONE_NEWNS)"));
        assert_eq!(ns_setup_step(8), Some("remounting / recursively private"));
        assert_eq!(ns_setup_step(9), Some("mounting tmpfs on /tmp"));
        assert_eq!(
            ns_setup_step(10),
            Some("verifying the chroot landed on the opened rootfs")
        );
        assert_eq!(ns_setup_step(0), None);
        assert_eq!(ns_setup_step(1), None);
        assert_eq!(ns_setup_step(2), None);
        assert_eq!(ns_setup_step(11), None);
    }

    /// TODO/enter.md T-1339: the (d,i) guard compares device and file.
    /// Same device and file lands; either differing refuses.
    #[test]
    fn the_chroot_guard_compares_device_and_file() {
        let landed = sys::Stat {
            st_dev: 2096,
            st_ino: 39833,
            ..Default::default()
        };
        assert!(chroot_landed(&landed, &landed));
        let elsewhere = sys::Stat {
            st_dev: 2096,
            st_ino: 39834,
            ..Default::default()
        };
        assert!(!chroot_landed(&landed, &elsewhere));
        let other_fs = sys::Stat {
            st_dev: 146,
            st_ino: 39833,
            ..Default::default()
        };
        assert!(!chroot_landed(&landed, &other_fs));
    }
}
