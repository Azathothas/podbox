//! The probe set, ported from `TOOL.md` section 6.1's minimum set and from the
//! Go instrument that implements it,
//! `references/Azathothas__container-research/tree/verification/probe/main.go`
//! and `.../probe/attribute.go`.
//!
//! ⭐ Row names are the reference's, character for character, so a podbox run
//! and a `probe census` / `probe attribute` run diff against each other. The
//! milestone's acceptance ([`TODO/milestones.md`](../../../TODO/milestones.md)
//! T-1101) is exactly that comparison.
//!
//! Four divergences from the Go instrument, each deliberate and each a defect
//! in it rather than a difference of taste:
//!
//! 1. ⛔ **The bare `mount(MS_SLAVE,/)` row is not ported.** In the caller's
//!    mount namespace it changes the propagation of `/` and everything under
//!    it, for the machine, permanently. `TOOL.md` section 6.1's minimum set
//!    names `mount(tmpfs)` in the current namespace and inside
//!    `clone(CLONE_NEWNS)`; it does not name that one. The `MS_SLAVE`
//!    operation is still measured, inside a private mount namespace where it
//!    cannot escape, which is also where bubblewrap performs it
//!    (`references/containers__bubblewrap/tree/bubblewrap.c:3257-3258`).
//! 2. **A probe that mounts, unmounts.** `mount(tmpfs,/mnt)` and
//!    `move_mount(-> /tmp/mm-probe)` succeed on an unconfined host, and the Go
//!    instrument leaves both mounted. A failure to remove one is reported in
//!    the row rather than left silent.
//! 3. **Nothing is overwritten.** Every scratch file is created `O_EXCL`, and
//!    an existing file at that path is a `skip` naming the file, not a probe
//!    that deletes somebody's data to make room for itself.
//! 4. ⭐ **A failed precondition is a `skip`, never the row's denial.** Where
//!    `fsmount` fails, the Go instrument reports its errno as
//!    `move_mount`'s verdict. `TODO/probe.md` T-0109 is that this is a defect:
//!    move_mount was not measured, so no verdict about it was established.

use crate::sys::{self, CBuf, Errno};
use crate::verdict::Outcome;

/// How a probe is run.
#[derive(Clone, Copy)]
pub enum Kind {
    /// The measurement is `clone(2)` with these flags: was the child built?
    /// The child does nothing but exit.
    Clone(u64),
    /// Run the body in a re-executed child, which is additionally placed in
    /// the namespaces named by the flags. A clone that the flags make fail is
    /// a failed precondition, so the row is a `skip` carrying its errno.
    Child {
        ns_flags: u64,
        body: fn() -> Outcome,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// `probe census`: what this machine permits.
    Census,
    /// `probe attribute`: the bogus-argument discriminator and its controls.
    /// `TODO/probe.md` T-0102.
    Attribution,
    /// The machine tier's legs. `TODO/podvm.md` T-1301: one leg per fact and
    /// a verdict per leg, each measured, never inferred from another leg.
    Machine,
    /// The supervise tier's legs. `TODO/supervise.md` T-0606: the tier's own
    /// rows, not the census and attribution rows the selection used to
    /// borrow. A borrowed row answers its own group's contract; a tier leg
    /// answers the tier, so a rename moves the block, the constant and the
    /// assessment together, like the machine legs above.
    Supervise,
}

pub struct Probe {
    pub name: &'static str,
    pub group: Group,
    pub kind: Kind,
}

const NS: u64 = sys::CLONE_NEWNS;
const UTS: u64 = sys::CLONE_NEWUTS;

/// ⛔ Fixed order. Two runs of this set are diffable only if the rows come out
/// in the same sequence, which is the same reason the Go instrument carries an
/// `order` array beside its map.
///
/// ⚠ `rustfmt` is turned off for this item and only this item. It is a table,
/// one row per probe, and the default formatting expands each row to four
/// lines: the set stops being readable as a set, which is the one thing this
/// declaration is for. `docs/conventions/code.md` requires an escape hatch to
/// say why, and this is why.
#[rustfmt::skip]
pub static PROBES: &[Probe] = &[
    // ------------------------------------------------------------- census
    Probe { name: "unshare(CLONE_NEWNS)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_unshare_newns } },
    Probe { name: "unshare(CLONE_NEWUSER)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_unshare_newuser } },
    Probe { name: "unshare(CLONE_NEWPID)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_unshare_newpid } },
    Probe { name: "clone(CLONE_NEWNS)", group: Group::Census, kind: Kind::Clone(NS) },
    Probe { name: "clone(CLONE_NEWUTS|NEWNS)", group: Group::Census, kind: Kind::Clone(UTS | NS) },
    Probe { name: "clone(CLONE_NEWUSER)", group: Group::Census,
            kind: Kind::Clone(sys::CLONE_NEWUSER) },
    // ⭐ Named by TOOL.md section 6.1's minimum set and by the banner of
    // section 6.8, and absent from the Go instrument. The UTS namespace is the
    // one the `chroot` rung can still use, so the banner's `namespaces=` field
    // is a measurement rather than an assumption.
    Probe { name: "clone(CLONE_NEWUTS)", group: Group::Census, kind: Kind::Clone(UTS) },
    Probe { name: "sethostname(in NEWUTS)", group: Group::Census,
            kind: Kind::Child { ns_flags: UTS, body: p_sethostname } },
    Probe { name: "mount(tmpfs,/mnt)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_mount_tmpfs } },
    Probe { name: "mount(tmpfs,/mnt) in clone(NEWNS)", group: Group::Census,
            kind: Kind::Child { ns_flags: NS, body: p_mount_tmpfs } },
    Probe { name: "mount(MS_SLAVE,/) in clone(NEWNS)", group: Group::Census,
            kind: Kind::Child { ns_flags: NS, body: p_mount_slave } },
    Probe { name: "pivot_root(/tmp,/tmp)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_pivot_root } },
    Probe { name: "ptrace(PTRACE_TRACEME)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_ptrace } },
    // ⭐ The pair of TODO/probe.md T-0106. Both, in the same directory: a
    // whiteout that succeeds where a real device number fails proves the
    // denial is capability-based, because no path-scoped policy can tell two
    // device numbers apart at one path.
    Probe { name: "mknod(chr 1:3 /tmp/nodprobe)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_mknod_real } },
    Probe { name: "mknod(chr 0:0 = whiteout)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_mknod_whiteout } },
    Probe { name: "setuid(1000)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_setuid_1000 } },
    Probe { name: "setuid(0)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_setuid_0 } },
    Probe { name: "setgid(0)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_setgid_0 } },
    Probe { name: "setgroups(0,NULL)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_setgroups } },
    Probe { name: "chown(f,0,0)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_chown_0_0 } },
    Probe { name: "chown(f,0,42)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_chown_0_42 } },
    Probe { name: "lchown(f,0,42)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_lchown_0_42 } },
    Probe { name: "chown(f,1000,0)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_chown_1000_0 } },
    Probe { name: "chroot(/tmp)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_chroot } },
    Probe { name: "memfd_create", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_memfd } },
    Probe { name: "memfd_create+exec", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_memfd_exec } },
    Probe { name: "setsid", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_setsid } },
    Probe { name: "prctl(PR_SET_PDEATHSIG)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_pdeathsig } },
    Probe { name: "exec(/tmp/execprobe)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_exec_file } },
    Probe { name: "getrandom", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_getrandom } },
    Probe { name: "write(/etc/probe)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_write_etc } },
    Probe { name: "write into uid-1000-owned dir", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_write_squash } },
    // ⭐ TODO/enter.md T-0503 puts these two in THIS set, in the OUTER
    // environment, before any chroot. `-t` either works or it does not, and
    // the corpus disagrees with itself about which: the target's mount table
    // shows six device nodes and no `ptmx`, and an earlier account asserts the
    // host's works and published no capture. A mount table does not list plain
    // files, so neither settles it and one stat does. The open is the second
    // row because existence is not function: rule 1 of TOOL.md section 6.1 is
    // to probe the operation you need.
    Probe { name: "stat(/dev/ptmx)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_stat_ptmx } },
    Probe { name: "open(/dev/ptmx, O_RDWR)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_open_ptmx } },
    // ⭐ TODO/packaging.md T-1003: the FUSE rung's probe input. Opened, not
    // stat-ed, like the ptmx leg below T-0503: a node that exists and answers
    // EACCES on open cannot carry the tier. In the outer environment before
    // any chroot, like the ptmx pair above it.
    Probe { name: "open(/dev/fuse, O_RDWR)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_open_fuse } },
    // ⭐ TODO/complete.md T-0414, in the OUTER environment like the ptmx
    // pair: list the root, open an entry in it by name, create a file at
    // its top level. Each leg is its own verdict with its own errno.
    Probe { name: "readdir(/)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_list_root } },
    Probe { name: "open(/bin, O_RDONLY) by name", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_open_by_name } },
    Probe { name: "creat(/, O_CREAT|O_EXCL)", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_creat_toplevel } },
    // ⭐ TODO/podvm.md T-1306. The spec target answers EPERM to every TCP
    // bind, loopback or wildcard, which kills every hostfwd-based manager.
    // A census row because it asks what this machine permits, like the
    // ptmx pair above it.
    Probe { name: "bind(127.0.0.1:0)+listen", group: Group::Census,
            kind: Kind::Child { ns_flags: 0, body: p_tcp_listen } },

    // -------------------------------------------------------- attribution
    // TODO/probe.md T-0102. A seccomp filter sees the syscall number and six
    // argument registers, cannot dereference a pointer, and runs before the
    // syscall body. An argument the kernel rejects INSIDE the body therefore
    // separates a filter from a policy that runs later.
    Probe { name: "mount(2) bogus target", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_mount_bogus } },
    Probe { name: "umount2(2) bogus target", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_umount_bogus } },
    Probe { name: "pivot_root(2) bogus paths", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_pivot_bogus } },
    Probe { name: "process_vm_readv(bogus pid)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_process_vm_readv } },
    // ⛔ The controls travel with the discriminator, in the same run. A control
    // that ran at a different time answers about a different machine state.
    Probe { name: "pidfd_getfd(-1,-1) [control]", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_pidfd_getfd } },
    Probe { name: "kcmp(-1,-1,...) [control]", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_kcmp } },
    Probe { name: "fsopen(tmpfs)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_fsopen } },
    Probe { name: "fsmount(tmpfs)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_fsmount } },
    Probe { name: "open_tree(/tmp, CLONE)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_open_tree } },
    Probe { name: "move_mount(-> bogus dest)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_move_mount_bogus } },
    Probe { name: "move_mount(-> /tmp/mm-probe)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_move_mount_real } },
    Probe { name: "openat(detached tmpfs, O_DIRECTORY)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_openat_detached } },
    Probe { name: "seccomp(NEW_LISTENER)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_seccomp_listener } },
    Probe { name: "open /proc/self/mem O_RDONLY", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_mem_rdonly } },
    Probe { name: "open /proc/self/mem O_RDWR", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_mem_rdwr } },
    Probe { name: "landlock_create_ruleset(VERSION)", group: Group::Attribution,
            kind: Kind::Child { ns_flags: 0, body: a_landlock } },

    // ------------------------------------------------------------ machine
    // TODO/podvm.md T-1301. One leg per fact the machine tier needs, each
    // measured by the operation itself: the emulator by running it, the two
    // device nodes by opening them, the file-size bound by reading it, the
    // image space by statfs, the accelerator by asking the emulator.
    // ⛔ Never infer a leg from another leg. A leg whose precondition is
    // another leg's subject says which one and stops, like every other
    // precondition in this file.
    Probe { name: "qemu-system-x86_64 --version", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_qemu_version } },
    Probe { name: "open(/dev/kvm, O_RDWR)", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_kvm } },
    Probe { name: "prlimit(RLIMIT_FSIZE)", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_fsize } },
    Probe { name: "open(/dev/net/tun, O_RDWR)", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_tun } },
    Probe { name: "image space (statfs .)", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_space } },
    Probe { name: "qemu-system-x86_64 -accel help", group: Group::Machine,
            kind: Kind::Child { ns_flags: 0, body: m_accel } },

    // ---------------------------------------------------------- supervise
    // TODO/supervise.md T-0606. The supervise tier's own three legs, each
    // measured by the operation itself: the notification listener by
    // creating one, the argument channel by copying through it, the
    // race-safety by asking for it. ⛔ Never infer a leg from another leg,
    // and never borrow a row that answers another group's contract.
    // The listener and ptrace rows share their bodies with the census and
    // attribution rows that run the same operation: one implementation, two
    // readings, so the operation cannot drift between the groups.
    Probe { name: "seccomp(NEW_LISTENER) [supervise]", group: Group::Supervise,
            kind: Kind::Child { ns_flags: 0, body: a_seccomp_listener } },
    Probe { name: "process_vm_readv(own pid) [supervise]", group: Group::Supervise,
            kind: Kind::Child { ns_flags: 0, body: s_read_channel } },
    Probe { name: "ptrace(PTRACE_TRACEME) [supervise]", group: Group::Supervise,
            kind: Kind::Child { ns_flags: 0, body: p_ptrace } },
];

/// The machine tier's legs, in the order they run. [`crate::machine`] and the
/// report read the verdicts through these names, so a rename moves all three.
pub const MACHINE_LEGS: &[&str] = &[
    "qemu-system-x86_64 --version",
    "open(/dev/kvm, O_RDWR)",
    "prlimit(RLIMIT_FSIZE)",
    "open(/dev/net/tun, O_RDWR)",
    "image space (statfs .)",
    "qemu-system-x86_64 -accel help",
];

/// The supervise tier's legs, in the order they run. [`crate::supervise`],
/// the report and the selection read the verdicts through these names, so a
/// rename moves all four.
pub const SUPERVISE_LEGS: &[&str] = &[
    "seccomp(NEW_LISTENER) [supervise]",
    "process_vm_readv(own pid) [supervise]",
    "ptrace(PTRACE_TRACEME) [supervise]",
];

pub fn find(name: &str) -> Option<&'static Probe> {
    PROBES.iter().find(|p| p.name == name)
}

// ---------------------------------------------------------------- helpers

/// A path that cannot exist. A syscall that resolves paths and is not filtered
/// must answer `ENOENT` for it.
const BOGUS: &str = "/proc/self/nonexistent-probe-path";

fn c(s: &str) -> CBuf {
    // Every literal below is NUL-free, so this cannot fail. Constructing it
    // through the checked path anyway keeps one constructor.
    CBuf::new(s).expect("probe path literals carry no interior NUL")
}

fn exists(path: &str) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Create a scratch file the probe owns.
///
/// ⛔ `O_EXCL`. A probe that deletes whatever is in its way has destroyed data
/// to measure a permission, and the errno it then reports is about a file it
/// created rather than the one that was there.
fn make_scratch(path: &str) -> Result<(), Outcome> {
    match create_exclusive(path, 0o600) {
        Ok(fd) => {
            let _ = sys::close(fd);
            Ok(())
        }
        Err(sys::EEXIST) => Err(already_there(path)),
        Err(e) => Err(Outcome::skip(
            Some(e),
            format!("could not create the scratch file {path}"),
        )),
    }
}

fn create_exclusive(path: &str, mode: u64) -> Result<i64, Errno> {
    sys::open(&c(path), sys::O_WRONLY | sys::O_CREAT | sys::O_EXCL, mode)
}

fn already_there(path: &str) -> Outcome {
    Outcome::skip(
        Some(sys::EEXIST),
        format!("{path} already exists and this probe does not overwrite; remove it and re-run"),
    )
}

fn unscratch(path: &str) {
    let _ = sys::unlink(&c(path));
}

/// `/bin/sh` is the interpreter both exec probes need. Its absence is a
/// missing precondition, and the `ENOENT` it would otherwise produce reads
/// exactly like a denial. TODO/probe.md T-0109 rule 2.
fn need_sh() -> Result<(), Outcome> {
    if exists("/bin/sh") {
        Ok(())
    } else {
        Err(Outcome::skip(
            Some(sys::ENOENT),
            "/bin/sh is absent, so there is no interpreter for the payload this \
             probe executes; the exec was not attempted",
        ))
    }
}

/// Run a path as a child process and report what happened to it.
///
/// ⛔ Through [`sys::shed_after_fork`], because this is the one place podbox
/// spawns a process with libstd rather than with `sys::clone_fork`, and a probe
/// makes one fresh child per probe. Without the hook every lock any other
/// thread holds is duplicated into this child at the `fork` and outlives its
/// holder's release. [`TODO/image.md`](../../../TODO/image.md) T-0215.
fn run_payload(path: &str) -> Outcome {
    let mut cmd = std::process::Command::new(path);
    match sys::shed_after_fork(&mut cmd).status() {
        Ok(st) if st.success() => Outcome::ok(),
        Ok(st) => Outcome::skip(
            None,
            format!("the payload {path} ran and ended {st}; the exec itself succeeded"),
        ),
        Err(e) => match e.raw_os_error() {
            Some(n) => Outcome::denied(Errno(n)),
            None => Outcome::skip(None, format!("spawning {path} failed: {e}")),
        },
    }
}

/// Which of the three calls behind a detached mount answered.
///
/// ⭐ Named rather than compared as a string: `TODO/probe.md` T-0103 asks for
/// creation, configuration and attachment to be separate verdicts, and a
/// caller cannot tell them apart from one errno.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Fsopen,
    Fsconfig,
    Fsmount,
}

impl Step {
    fn label(self) -> &'static str {
        match self {
            Step::Fsopen => "fsopen(tmpfs)",
            Step::Fsconfig => "fsconfig(FSCONFIG_CMD_CREATE)",
            Step::Fsmount => "fsmount(tmpfs)",
        }
    }
}

/// A detached tmpfs mount fd, or the step that failed and its errno.
fn detached_tmpfs() -> Result<i64, (Errno, Step)> {
    let fs = c("tmpfs");
    let fd = unsafe { sys::sys(sys::SYS_FSOPEN, [fs.ptr(), 0, 0, 0, 0, 0]) }
        .map_err(|e| (e, Step::Fsopen))?;
    let r = unsafe {
        sys::sys(
            sys::SYS_FSCONFIG,
            [fd as u64, sys::FSCONFIG_CMD_CREATE, 0, 0, 0, 0],
        )
    };
    if let Err(e) = r {
        let _ = sys::close(fd);
        return Err((e, Step::Fsconfig));
    }
    let mfd = unsafe { sys::sys(sys::SYS_FSMOUNT, [fd as u64, 0, 0, 0, 0, 0]) };
    let _ = sys::close(fd);
    mfd.map_err(|e| (e, Step::Fsmount))
}

fn precondition(step: &str, e: Errno) -> Outcome {
    Outcome::skip(
        Some(e),
        format!(
            "{step} failed with {} ({}), so this operation was never attempted",
            e.name(),
            e.0
        ),
    )
}

// ---------------------------------------------------------------- census

fn p_unshare_newns() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_UNSHARE, [sys::CLONE_NEWNS, 0, 0, 0, 0, 0]) })
}

fn p_unshare_newuser() -> Outcome {
    // ⛔ TODO/probe.md T-0101: the kernel refuses `unshare(CLONE_NEWUSER)` to a
    // multithreaded caller with EINVAL regardless of policy, so an EINVAL from
    // a threaded prober is the prober's own answer and not the machine's.
    // podbox is single-threaded by construction (`TOOL.md` section 4.2) and
    // this child was freshly execed, so the invariant holds. It is checked
    // rather than assumed, because the day somebody adds a thread pool the
    // check is what says so.
    match threads_of_self() {
        Some(n) if n > 1 => {
            return Outcome::skip(
                None,
                format!(
                    "this prober has {n} threads, and the kernel refuses \
                         unshare(CLONE_NEWUSER) to any multithreaded caller with \
                         EINVAL whatever the policy; the machine was not asked"
                ),
            )
        }
        _ => {}
    }
    Outcome::from(unsafe { sys::sys(sys::SYS_UNSHARE, [sys::CLONE_NEWUSER, 0, 0, 0, 0, 0]) })
}

fn p_unshare_newpid() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_UNSHARE, [sys::CLONE_NEWPID, 0, 0, 0, 0, 0]) })
}

fn threads_of_self() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("Threads:"))
        .and_then(|v| v.trim().parse().ok())
}

fn p_sethostname() -> Outcome {
    // Runs inside clone(CLONE_NEWUTS), so a success renames that namespace's
    // host and nothing else. Without the namespace the row is a `skip`, which
    // the child harness produces from the clone's own errno.
    let name = c("podbox-probe");
    Outcome::from(unsafe { sys::sys(sys::SYS_SETHOSTNAME, [name.ptr(), 12, 0, 0, 0, 0]) })
}

fn p_mount_tmpfs() -> Outcome {
    // ⛔ The target is NOT checked before the call, and that is the point. A
    // seccomp filter runs before the syscall body, so a filtered mount(2)
    // answers EPERM whether or not /mnt exists; a `stat` first would turn that
    // measurement into a skip and this machine's policy would go unmeasured.
    // The precondition is separated afterwards instead, from the errno: only
    // ENOENT is a statement about the target rather than about the mount.
    // TOOL.md section 2.1 is the same argument, used the other way round.
    let (src, tgt, fs, data) = (c("none"), c("/mnt"), c("tmpfs"), sys::cempty());
    let r = unsafe {
        sys::sys(
            sys::SYS_MOUNT,
            [src.ptr(), tgt.ptr(), fs.ptr(), 0, data.ptr(), 0],
        )
    };
    match r {
        Err(sys::ENOENT) => Outcome::skip(
            Some(sys::ENOENT),
            "mount(2) executed and answered ENOENT, which is about the target \
             /mnt and not about the mount policy: /mnt does not exist here. The \
             policy itself is in the `mount(2) bogus target` row.",
        ),
        Err(e) => Outcome::denied(e),
        Ok(_) => {
            // A probe that mounts, unmounts. Where it cannot, the row says so.
            match unsafe { sys::sys(sys::SYS_UMOUNT2, [tgt.ptr(), 0, 0, 0, 0, 0]) } {
                Ok(_) => Outcome::ok(),
                Err(e) => Outcome::ok_with(format!(
                    "⚠ the tmpfs mounted at /mnt could not be removed: umount2 \
                     answered {} ({}). It is still mounted.",
                    e.name(),
                    e.0
                )),
            }
        }
    }
}

fn p_mount_slave() -> Outcome {
    // Only ever reached inside clone(CLONE_NEWNS): see the module header. This
    // is bubblewrap's `MS_SLAVE` remount, the one whose failure it reports as
    // "Failed to make / slave"
    // (`references/containers__bubblewrap/tree/bubblewrap.c:3257-3258`).
    let (src, tgt, fs, data) = (sys::cempty(), c("/"), sys::cempty(), sys::cempty());
    Outcome::from(unsafe {
        sys::sys(
            sys::SYS_MOUNT,
            [
                src.ptr(),
                tgt.ptr(),
                fs.ptr(),
                sys::MS_SLAVE | sys::MS_REC,
                data.ptr(),
                0,
            ],
        )
    })
}

fn p_pivot_root() -> Outcome {
    let p = c("/tmp");
    Outcome::from(unsafe { sys::sys(sys::SYS_PIVOT_ROOT, [p.ptr(), p.ptr(), 0, 0, 0, 0]) })
}

fn p_ptrace() -> Outcome {
    // PTRACE_TRACEME is request 0.
    Outcome::from(unsafe { sys::sys(sys::SYS_PTRACE, [0, 0, 0, 0, 0, 0]) })
}

fn p_mknod_real() -> Outcome {
    mknod_at("/tmp/nodprobe", sys::DEV_1_3)
}

fn p_mknod_whiteout() -> Outcome {
    // makedev(0,0) is WHITEOUT_DEV and `vfs_mknod` exempts it from the
    // capability check, which is why it tests nothing on its own and why
    // TODO/probe.md T-0106 requires the pair.
    mknod_at("/tmp/wprobe", 0)
}

fn mknod_at(path: &str, dev: u64) -> Outcome {
    if exists(path) {
        return already_there(path);
    }
    let p = c(path);
    // ⛔ `sys::MKNOD` and not a bare number: on an architecture whose kernel has
    // no `mknod(2)` this is `mknodat(2)`, with the directory descriptor first,
    // and the row says which was issued. `TODO/probe.md` T-0101 measures a
    // NAMED syscall, so calling one and printing another would be the exact
    // dishonesty this probe set exists to avoid.
    let e = sys::MKNOD;
    let args = if e.via_at {
        [sys::AT_FDCWD, p.ptr(), sys::S_IFCHR | 0o600, dev, 0, 0]
    } else {
        [p.ptr(), sys::S_IFCHR | 0o600, dev, 0, 0, 0]
    };
    let r = unsafe { sys::sys(e.nr, args) };
    if r.is_ok() {
        unscratch(path);
    }
    Outcome::from(r)
}

fn p_setuid_1000() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_SETUID, [1000, 0, 0, 0, 0, 0]) })
}

fn p_setuid_0() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_SETUID, [0, 0, 0, 0, 0, 0]) })
}

fn p_setgid_0() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_SETGID, [0, 0, 0, 0, 0, 0]) })
}

fn p_setgroups() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_SETGROUPS, [0, 0, 0, 0, 0, 0]) })
}

const CHOWN_TARGET: &str = "/tmp/chown-probe-target";

/// ⛔ The entry point is part of the measurement, so it is passed in rather
/// than assumed. Where the kernel has no `chown(2)`, every architecture taking
/// its table from `asm-generic/unistd.h`, [`sys::CHOWN`] is `fchownat(2)` and
/// the row prints that name. The **verdict** is about the operation (may this
/// process give a file an id it does not map?), and the entry point is the
/// evidence under it.
fn chown_probe(e: sys::Entry, uid: u64, gid: u64) -> Outcome {
    if let Err(o) = make_scratch(CHOWN_TARGET) {
        return o;
    }
    let p = c(CHOWN_TARGET);
    let at_flags = if e.name.contains("AT_SYMLINK_NOFOLLOW") {
        sys::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };
    let args = if e.via_at {
        [sys::AT_FDCWD, p.ptr(), uid, gid, at_flags, 0]
    } else {
        [p.ptr(), uid, gid, 0, 0, 0]
    };
    let r = unsafe { sys::sys(e.nr, args) };
    unscratch(CHOWN_TARGET);
    Outcome::from(r)
}

fn p_chown_0_0() -> Outcome {
    chown_probe(sys::CHOWN, 0, 0)
}

fn p_chown_0_42() -> Outcome {
    chown_probe(sys::CHOWN, 0, 42)
}

fn p_lchown_0_42() -> Outcome {
    chown_probe(sys::LCHOWN, 0, 42)
}

fn p_chown_1000_0() -> Outcome {
    chown_probe(sys::CHOWN, 1000, 0)
}

fn p_chroot() -> Outcome {
    let p = c("/tmp");
    Outcome::from(unsafe { sys::sys(sys::SYS_CHROOT, [p.ptr(), 0, 0, 0, 0, 0]) })
}

fn memfd(name: &str) -> Result<i64, Errno> {
    // ⛔ No MFD_CLOEXEC: the descriptor has to survive a fork and exec for
    // /proc/self/fd/N to name it in the payload.
    let n = c(name);
    unsafe { sys::sys(sys::SYS_MEMFD_CREATE, [n.ptr(), 0, 0, 0, 0, 0]) }
}

fn p_memfd() -> Outcome {
    match memfd("probe") {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn p_memfd_exec() -> Outcome {
    if let Err(o) = need_sh() {
        return o;
    }
    let fd = match memfd("probe") {
        Ok(fd) => fd,
        Err(e) => return precondition("memfd_create", e),
    };
    let script = b"#!/bin/sh\nexit 0\n";
    if let Err(e) = sys::write(fd, script) {
        let _ = sys::close(fd);
        return precondition("write to the memfd", e);
    }
    let out = run_payload(&format!("/proc/self/fd/{fd}"));
    let _ = sys::close(fd);
    out
}

fn p_setsid() -> Outcome {
    Outcome::from(unsafe { sys::sys(sys::SYS_SETSID, [0, 0, 0, 0, 0, 0]) })
}

fn p_pdeathsig() -> Outcome {
    Outcome::from(unsafe {
        sys::sys(
            sys::SYS_PRCTL,
            [sys::PR_SET_PDEATHSIG, 15 /* SIGTERM */, 0, 0, 0, 0],
        )
    })
}

const EXEC_TARGET: &str = "/tmp/execprobe";

fn p_exec_file() -> Outcome {
    if let Err(o) = need_sh() {
        return o;
    }
    if exists(EXEC_TARGET) {
        return already_there(EXEC_TARGET);
    }
    let fd = match create_exclusive(EXEC_TARGET, 0o755) {
        Ok(fd) => fd,
        Err(sys::EEXIST) => return already_there(EXEC_TARGET),
        Err(e) => return precondition(&format!("creating {EXEC_TARGET}"), e),
    };
    let r = sys::write(fd, b"#!/bin/sh\nexit 0\n");
    let _ = sys::close(fd);
    if let Err(e) = r {
        unscratch(EXEC_TARGET);
        return precondition(&format!("writing {EXEC_TARGET}"), e);
    }
    let out = run_payload(EXEC_TARGET);
    unscratch(EXEC_TARGET);
    out
}

fn p_getrandom() -> Outcome {
    let mut buf = [0u8; 16];
    Outcome::from(unsafe {
        sys::sys(
            sys::SYS_GETRANDOM,
            [buf.as_mut_ptr() as u64, buf.len() as u64, 0, 0, 0, 0],
        )
    })
}

/// ⭐ The one probe whose failure to create a file IS the measurement, which
/// is why it does not go through `make_scratch`. An `EEXIST` is still a skip:
/// the file that was in the way was somebody else's, and nothing about the
/// write policy was learned.
fn p_write_etc() -> Outcome {
    const P: &str = "/etc/probe";
    match create_exclusive(P, 0o600) {
        Ok(fd) => {
            let _ = sys::close(fd);
            unscratch(P);
            Outcome::ok()
        }
        Err(sys::EEXIST) => already_there(P),
        Err(e) => Outcome::denied(e),
    }
}

/// The root-squash observation: a directory owned by an id that is not mapped
/// in this user namespace is unwritable even with `CAP_DAC_OVERRIDE`.
///
/// The fixture cannot be built from inside the confinement, because `chown` to
/// an unmapped id is exactly what is denied. `experiments/20-enter-target.sh`
/// stages it; the Go instrument looks for its own copy under `/tmp`.
fn p_write_squash() -> Outcome {
    const CANDIDATES: [&str; 2] = ["/workspace/.fixtures/squash-probe", "/tmp/squash-probe"];
    let Some(dir) = CANDIDATES.iter().find(|d| exists(d)) else {
        return Outcome::skip(
            Some(sys::ENOENT),
            format!(
                "no fixture directory owned by an unmapped id: neither {} nor {} \
                 exists. Create one as another uid outside the confinement.",
                CANDIDATES[0], CANDIDATES[1]
            ),
        );
    };
    let path = format!("{dir}/podbox-write-probe");
    match create_exclusive(&path, 0o600) {
        Ok(fd) => {
            let _ = sys::close(fd);
            unscratch(&path);
            Outcome::ok_with(format!("wrote into {dir}"))
        }
        Err(sys::EEXIST) => already_there(&path),
        Err(e) => Outcome::denied(e),
    }
}

pub const PTMX: &str = "/dev/ptmx";

fn p_stat_ptmx() -> Outcome {
    match sys::stat(&c(PTMX)) {
        Ok(st) => Outcome::ok_with(format!(
            "{} {}:{} mode {:o}",
            st.kind(),
            st.rdev_major(),
            st.rdev_minor(),
            st.st_mode & 0o7777
        )),
        // ⛔ ENOENT here is the answer, not a missing precondition: the
        // question this row exists for is whether the file is there.
        Err(e) => Outcome::denied(e),
    }
}

fn p_open_ptmx() -> Outcome {
    // ⚠ O_NOCTTY. Opening a terminal without it can make it the prober's
    // controlling terminal, which mutates the process doing the measuring.
    // The child is disposable either way; the flag is what makes the row a
    // measurement rather than a side effect.
    match sys::open(&c(PTMX), sys::O_RDWR | sys::O_NOCTTY, 0) {
        Ok(fd) => {
            // A successful open of a working ptmx allocates a pty pair. Closing
            // it releases the slave, so the probe leaves no pty behind.
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

/// Whether the FUSE rung may promise a mount, T-1003.
///
/// One home for the question the ladder asks: only the OPEN row counts, with
/// an `Ok` verdict and nothing else, which is `ptmx_usable`'s own rule. The
/// `mknod` this runtime refuses cannot create the node where it is absent,
/// so absence is the answer rather than a missing precondition.
pub const FUSE: &str = "/dev/fuse";

fn p_open_fuse() -> Outcome {
    open_probe(FUSE, sys::O_RDWR | sys::O_CLOEXEC)
}

/// The FUSE rung's probe input, beside `ptmx_usable`: true only where the
/// open row ran and answered `Ok`. A `Skip` never ran, so it promises
/// nothing (T-0109 rule 1); a `Denied` row is the refusal itself.
pub fn fuse_usable(findings: &crate::Findings) -> bool {
    findings.rows.iter().any(|(n, out)| {
        n.starts_with("open(/dev/fuse") && matches!(out.verdict, crate::verdict::Verdict::Ok)
    })
}

// ⭐ TODO/complete.md T-0414. A third instance of the target class answers
// EACCES to readdir("/") while opening entries by name still works, and
// denies creat at the top level: a launcher died on the first, running
// `find /` to collect bind sources. Three legs, each its own verdict, in
// the outer environment before any chroot.
fn p_list_root() -> Outcome {
    match sys::open(
        &c("/"),
        sys::O_RDONLY | sys::O_DIRECTORY | sys::O_CLOEXEC,
        0,
    ) {
        Err(e) => Outcome::denied(e),
        Ok(fd) => {
            let res = sys::getdents64(fd);
            let _ = sys::close(fd);
            match res {
                Ok(entries) => Outcome::ok_with(format!("{} entries", entries.len())),
                Err(e) => Outcome::denied(e),
            }
        }
    }
}

fn p_open_by_name() -> Outcome {
    // A fixed FHS name, documented: the leg asks whether opening by name
    // works, not whether this name exists. ENOENT where it does not is the
    // kernel's answer, not a harness failure.
    match sys::open(&c("/bin"), sys::O_RDONLY | sys::O_CLOEXEC, 0) {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn p_creat_toplevel() -> Outcome {
    let path = format!("/podbox-probe-creat-{}", std::process::id());
    let cpath = match sys::CBuf::new(&path) {
        Some(b) => b,
        None => {
            return Outcome::skip(None, "the creat path is unrepresentable".to_string());
        }
    };
    // O_EXCL: refusing to clobber is part of the question, and the probe
    // leaves nothing behind either way.
    match sys::open(
        &cpath,
        sys::O_WRONLY | sys::O_CREAT | sys::O_EXCL | sys::O_CLOEXEC,
        0o600,
    ) {
        Err(e) => Outcome::denied(e),
        Ok(fd) => {
            let _ = sys::close(fd);
            match sys::unlink(&cpath) {
                Ok(_) => Outcome::ok(),
                Err(e) => Outcome::skip(
                    Some(e),
                    format!("created {path} but could not unlink it: left in place"),
                ),
            }
        }
    }
}

/// Whether the chroot rung may be entered, T-1317.
///
/// One home for the question `run`, `create` and `start` all ask before any
/// fixup mutates the rootfs: only the chroot row with an `Ok` verdict
/// promises entry. A `Skip` never ran, so it promises nothing (T-0109
/// rule 1); a `Denied` row is the refusal itself.
pub fn chroot_usable(findings: &crate::Findings) -> bool {
    findings.rows.iter().any(|(n, out)| {
        n.starts_with("chroot(") && matches!(out.verdict, crate::verdict::Verdict::Ok)
    })
}

/// Whether `-t` may promise a pty, T-0503.
///
/// One home for the question `run` and `exec` both ask: only the OPEN row
/// counts, with an `Ok` verdict and nothing else. A `Skip` never ran, so it
/// promises nothing (T-0109 rule 1); a `Denied` row is the refusal itself.
/// Existence is not function, so the stat row does not answer.
pub fn ptmx_usable(findings: &crate::Findings) -> bool {
    findings.rows.iter().any(|(n, out)| {
        n.starts_with("open(/dev/ptmx") && matches!(out.verdict, crate::verdict::Verdict::Ok)
    })
}

fn p_tcp_listen() -> Outcome {
    // socket+bind+listen on 127.0.0.1 port 0: the kernel picks the port, so
    // no fixture can collide, and nothing is ever accepted on it. Closing
    // releases the port, so the probe leaves no listener behind.
    // sockaddr_in, 16 bytes: family, port, address, 8 zero bytes.
    let fd = match sys::socket(sys::AF_INET, sys::SOCK_STREAM | sys::SOCK_CLOEXEC, 0) {
        Ok(fd) => fd,
        Err(e) => return Outcome::denied(e),
    };
    let mut addr = [0u8; 16];
    // The family in native order: this workspace builds for big-endian
    // architectures too (TODO/deps.md T-0911), where a hard-coded
    // little-endian 2 reads as family 512. Port 0 and the zero bytes are
    // order-free; the address is big-endian by definition.
    let fam = 2u16.to_ne_bytes();
    addr[0] = fam[0];
    addr[1] = fam[1];
    addr[4] = 127;
    addr[7] = 1;
    let r = match sys::bind(fd, addr.as_ptr() as u64, 16) {
        Ok(_) => sys::listen(fd, 1).map(|_| ()),
        Err(e) => Err(e),
    };
    let _ = sys::close(fd);
    match r {
        Ok(()) => Outcome::ok_with("loopback TCP bind+listen permitted"),
        Err(e) => Outcome::denied(e),
    }
}

// ------------------------------------------------------------ attribution

fn a_mount_bogus() -> Outcome {
    let (src, tgt, fs) = (c("none"), c(BOGUS), c("tmpfs"));
    Outcome::from(unsafe { sys::sys(sys::SYS_MOUNT, [src.ptr(), tgt.ptr(), fs.ptr(), 0, 0, 0]) })
}

fn a_umount_bogus() -> Outcome {
    let tgt = c(BOGUS);
    Outcome::from(unsafe { sys::sys(sys::SYS_UMOUNT2, [tgt.ptr(), 0, 0, 0, 0, 0]) })
}

fn a_pivot_bogus() -> Outcome {
    let p = c(BOGUS);
    Outcome::from(unsafe { sys::sys(sys::SYS_PIVOT_ROOT, [p.ptr(), p.ptr(), 0, 0, 0, 0]) })
}

fn a_process_vm_readv() -> Outcome {
    let mut buf = [0u8; 1];
    let local = [buf.as_mut_ptr() as u64, 1u64];
    let remote = [0x1000u64, 1u64];
    Outcome::from(unsafe {
        sys::sys(
            sys::SYS_PROCESS_VM_READV,
            [
                999_999,
                local.as_ptr() as u64,
                1,
                remote.as_ptr() as u64,
                1,
                0,
            ],
        )
    })
}

/// The supervise tier's argument channel, T-0606.
///
/// ⛔ Not the discriminator above. A bogus pid separates a filter from a
/// later policy; it never copies a byte. This aims the remote end at one
/// byte of our own memory and copies it: the returned count is the
/// integrity check, so an `Ok` answers the channel carries arguments and a
/// denial answers it is filtered or absent. `references/multikernel__sandlock`
/// reads the child's arguments through exactly this call and has no
/// `/proc/pid/mem` fallback anywhere in its tree.
/// ⚠ The count is read through `Outcome::from` beside every other body: for
/// a valid single-byte iov the kernel answers 1 or an errno, and a short
/// count has no errno to carry, so there is no honest third verdict for it.
fn s_read_channel() -> Outcome {
    static MARK: u8 = 0x5a;
    let mut buf = [0u8; 1];
    let local = [buf.as_mut_ptr() as u64, 1u64];
    let remote = [&MARK as *const u8 as u64, 1u64];
    Outcome::from(unsafe {
        sys::sys(
            sys::SYS_PROCESS_VM_READV,
            [
                sys::getpid() as u64,
                local.as_ptr() as u64,
                1,
                remote.as_ptr() as u64,
                1,
                0,
            ],
        )
    })
}

fn a_pidfd_getfd() -> Outcome {
    let m1 = -1i64 as u64;
    Outcome::from(unsafe { sys::sys(sys::SYS_PIDFD_GETFD, [m1, m1, 0, 0, 0, 0]) })
}

fn a_kcmp() -> Outcome {
    let m1 = -1i64 as u64;
    let r = unsafe { sys::sys(sys::SYS_KCMP, [m1, m1, 0, 0, 0, 0]) };
    match r {
        // ⭐ TODO/probe.md T-0102's correction. `kcmp(2)` exists only where the
        // kernel was built with CONFIG_CHECKPOINT_RESTORE. ENOSYS is that
        // kernel saying the control is unavailable, which is not the same
        // statement as "the control answered". A control that cannot answer is
        // reported as unable to, so the discriminator it belongs to says it has
        // stopped discriminating rather than reporting a mechanism.
        Err(sys::ENOSYS) => Outcome::skip(
            Some(sys::ENOSYS),
            "kcmp(2) is not present on this kernel (CONFIG_CHECKPOINT_RESTORE is \
             unset), so this control cannot answer here",
        ),
        other => Outcome::from(other),
    }
}

fn a_fsopen() -> Outcome {
    let fs = c("tmpfs");
    match unsafe { sys::sys(sys::SYS_FSOPEN, [fs.ptr(), 0, 0, 0, 0, 0]) } {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn a_fsmount() -> Outcome {
    match detached_tmpfs() {
        Ok(mfd) => {
            let _ = sys::close(mfd);
            // ⭐ TODO/probe.md T-0103: fsmount runs the same may_mount() check
            // move_mount and unshare(CLONE_NEWNS) use, so its success proves no
            // mount denial on this machine is a capability problem. Saying so
            // in the row stops the next reader hunting for a capability.
            Outcome::ok_with("may_mount() passes, so no mount denial here is a capability problem")
        }
        Err((e, Step::Fsmount)) => Outcome::denied(e),
        Err((e, step)) => precondition(step.label(), e),
    }
}

fn a_open_tree() -> Outcome {
    let p = c("/tmp");
    match unsafe {
        sys::sys(
            sys::SYS_OPEN_TREE,
            [sys::AT_FDCWD, p.ptr(), sys::OPEN_TREE_CLONE, 0, 0, 0],
        )
    } {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn a_move_mount_bogus() -> Outcome {
    let mfd = match detached_tmpfs() {
        Ok(fd) => fd,
        Err((e, step)) => return precondition(step.label(), e),
    };
    let (empty, dest) = (sys::cempty(), c(BOGUS));
    let r = unsafe {
        sys::sys(
            sys::SYS_MOVE_MOUNT,
            [
                mfd as u64,
                empty.ptr(),
                sys::AT_FDCWD,
                dest.ptr(),
                sys::MOVE_MOUNT_F_EMPTY_PATH,
                0,
            ],
        )
    };
    let _ = sys::close(mfd);
    Outcome::from(r)
}

fn a_move_mount_real() -> Outcome {
    const DEST: &str = "/tmp/mm-probe";
    let mfd = match detached_tmpfs() {
        Ok(fd) => fd,
        Err((e, step)) => return precondition(step.label(), e),
    };
    let dest = c(DEST);
    if let Err(e) = sys::mkdir(&dest, 0o755) {
        if e != sys::EEXIST {
            let _ = sys::close(mfd);
            return precondition(&format!("mkdir {DEST}"), e);
        }
    }
    let empty = sys::cempty();
    let r = unsafe {
        sys::sys(
            sys::SYS_MOVE_MOUNT,
            [
                mfd as u64,
                empty.ptr(),
                sys::AT_FDCWD,
                dest.ptr(),
                sys::MOVE_MOUNT_F_EMPTY_PATH,
                0,
            ],
        )
    };
    let _ = sys::close(mfd);
    match r {
        Err(e) => Outcome::denied(e),
        Ok(_) => match unsafe { sys::sys(sys::SYS_UMOUNT2, [dest.ptr(), 0, 0, 0, 0, 0]) } {
            Ok(_) => Outcome::ok(),
            Err(e) => Outcome::ok_with(format!(
                "⚠ the tmpfs attached at {DEST} could not be removed: umount2 \
                 answered {} ({}). It is still mounted.",
                e.name(),
                e.0
            )),
        },
    }
}

fn a_openat_detached() -> Outcome {
    let mfd = match detached_tmpfs() {
        Ok(fd) => fd,
        Err((e, step)) => return precondition(step.label(), e),
    };
    let dot = c(".");
    let r = unsafe {
        sys::sys(
            sys::SYS_OPENAT,
            [
                mfd as u64,
                dot.ptr(),
                sys::O_RDONLY | sys::O_DIRECTORY,
                0,
                0,
                0,
            ],
        )
    };
    if let Ok(fd) = r {
        let _ = sys::close(fd);
    }
    let _ = sys::close(mfd);
    Outcome::from(r)
}

/// `struct sock_filter` and `struct sock_fprog`, in the kernel's layout. The
/// six padding bytes are the alignment of the pointer that follows the `u16`,
/// and writing them out is what stops the pointer landing four bytes early.
#[repr(C)]
struct SockFilter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[repr(C)]
struct SockFprog {
    len: u16,
    pad: [u8; 6],
    filter: *const SockFilter,
}

fn a_seccomp_listener() -> Outcome {
    // BPF_RET|BPF_K with SECCOMP_RET_ALLOW: a filter that permits everything
    // and exists only to carry the notification listener.
    let filter = [SockFilter {
        code: 0x06,
        jt: 0,
        jf: 0,
        k: 0x7fff_0000,
    }];
    let prog = SockFprog {
        len: 1,
        pad: [0; 6],
        filter: filter.as_ptr(),
    };
    if let Err(e) = unsafe { sys::sys(sys::SYS_PRCTL, [sys::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0, 0]) } {
        return precondition("prctl(PR_SET_NO_NEW_PRIVS)", e);
    }
    let r = unsafe {
        sys::sys(
            sys::SYS_SECCOMP,
            [
                sys::SECCOMP_SET_MODE_FILTER,
                sys::SECCOMP_FILTER_FLAG_NEW_LISTENER,
                &prog as *const SockFprog as u64,
                0,
                0,
                0,
            ],
        )
    };
    match r {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn open_probe(path: &str, flags: u64) -> Outcome {
    let p = c(path);
    match sys::open(&p, flags, 0) {
        Ok(fd) => {
            let _ = sys::close(fd);
            Outcome::ok()
        }
        Err(e) => Outcome::denied(e),
    }
}

fn a_mem_rdonly() -> Outcome {
    open_probe("/proc/self/mem", sys::O_RDONLY)
}

fn a_mem_rdwr() -> Outcome {
    open_probe("/proc/self/mem", sys::O_RDWR)
}

fn a_landlock() -> Outcome {
    // (NULL, 0, LANDLOCK_CREATE_RULESET_VERSION) returns the ABI rather than a
    // ruleset fd. This row reports whether an LSM of the class that explains
    // mechanism M is present at all, which is not a verdict about podbox.
    match unsafe { sys::sys(sys::SYS_LANDLOCK_CREATE_RULESET, [0, 0, 1, 0, 0, 0]) } {
        Ok(abi) => Outcome::ok_with(format!("ABI {abi}")),
        Err(e) => Outcome::denied(e),
    }
}

// ------------------------------------------------------------ machine

/// The emulator the machine tier drives. Named once, so the two legs that
/// spawn it cannot drift onto two different binaries.
const QEMU: &str = "qemu-system-x86_64";

/// What running the emulator answered: its stdout, or why there is none.
enum EmuAnswer {
    Said(String),
    Absent,
    Unusable(Errno),
    Exited(i32),
    Failed(String),
}

/// Run the emulator with `args` and take its stdout.
///
/// ⛔ Through [`sys::shed_after_fork`], like [`run_payload`]: libstd forks
/// inside `output()`, and without the hook every lock another thread holds is
/// duplicated into the child at the fork. `TODO/image.md` T-0215.
fn qemu_output(args: &[&str]) -> EmuAnswer {
    let mut cmd = std::process::Command::new(QEMU);
    cmd.args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());
    let out = match sys::shed_after_fork(&mut cmd).output() {
        Ok(o) => o,
        Err(e) => match e.raw_os_error() {
            // The lookup is `execvp`'s, so ENOENT is the binary's absence from
            // PATH and not a statement about the machine's acceleration.
            Some(n) if n == sys::ENOENT.0 => return EmuAnswer::Absent,
            Some(n) => return EmuAnswer::Unusable(Errno(n)),
            None => return EmuAnswer::Failed(format!("spawning {QEMU} failed: {e}")),
        },
    };
    if !out.status.success() {
        return EmuAnswer::Exited(out.status.code().unwrap_or(-1));
    }
    EmuAnswer::Said(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn m_qemu_version() -> Outcome {
    match qemu_output(&["--version"]) {
        EmuAnswer::Said(out) => match out.lines().next().map(str::trim).filter(|l| !l.is_empty()) {
            Some(line) => Outcome::ok_with(line.chars().take(160).collect::<String>()),
            None => Outcome::skip(
                None,
                "qemu-system-x86_64 --version ran and answered nothing",
            ),
        },
        EmuAnswer::Absent => Outcome::skip(
            Some(sys::ENOENT),
            "no qemu-system-x86_64 on PATH, so the emulator leg could not run",
        ),
        EmuAnswer::Unusable(e) => Outcome::denied(e),
        EmuAnswer::Exited(code) => Outcome::skip(
            None,
            format!(
                "qemu-system-x86_64 --version ran and exited {code}; the emulator \
                 runs but did not answer"
            ),
        ),
        EmuAnswer::Failed(why) => Outcome::skip(None, why),
    }
}

fn m_kvm() -> Outcome {
    // ⛔ OPENED, not stat-ed. A node that exists and answers EACCES on open is
    // not acceleration. T-1301. Absence is the answer too, so ENOENT is a
    // denial rather than a missing precondition, like `stat(/dev/ptmx)`.
    open_probe("/dev/kvm", sys::O_RDWR | sys::O_CLOEXEC)
}

fn m_fsize() -> Outcome {
    match sys::prlimit(sys::RLIMIT_FSIZE) {
        Ok((cur, max)) => {
            Outcome::ok_with(format!("RLIMIT_FSIZE cur={} max={}", rlim(cur), rlim(max)))
        }
        Err(e) => Outcome::denied(e),
    }
}

/// An rlimit value in words. `RLIM64_INFINITY` is not a size, so printing it
/// as digits would be the plausible-looking number AGENTS.md absolute 3
/// refuses.
fn rlim(v: u64) -> String {
    if v == u64::MAX {
        "infinity".to_string()
    } else {
        v.to_string()
    }
}

fn m_tun() -> Outcome {
    // Opened the same way as the kvm leg: a node that cannot be opened cannot
    // carry the tier's network.
    open_probe("/dev/net/tun", sys::O_RDWR | sys::O_CLOEXEC)
}

fn m_space() -> Outcome {
    // Blocks and inodes both, per `TODO/RULES.md` section 8, for the directory
    // the caller runs in, named in the row. A destination the tier will use is
    // not known at probe time; T-1302's consumer checks its own.
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => match e.raw_os_error() {
            Some(n) => {
                return Outcome::skip(
                    Some(Errno(n)),
                    "the working directory could not be read, so no space was measured",
                )
            }
            None => {
                return Outcome::skip(
                    None,
                    format!("the working directory could not be read: {e}"),
                )
            }
        },
    };
    let path = cwd.to_string_lossy().into_owned();
    let Some(p) = CBuf::new(&path) else {
        return Outcome::skip(
            None,
            "the working directory contains a NUL and cannot reach the kernel",
        );
    };
    match sys::statfs(&p) {
        Ok(s) => Outcome::ok_with(format!(
            "{} blocks of {} B free, {} inodes free at {path}",
            s.f_bavail, s.f_bsize, s.f_ffree
        )),
        Err(e) => Outcome::denied(e),
    }
}

fn m_accel() -> Outcome {
    // ⭐ Ask the emulator rather than deciding from the kvm leg. An
    // accelerator the build was compiled without is as absent as a missing
    // node, and the node alone cannot say which builds carry what.
    match qemu_output(&["-accel", "help"]) {
        EmuAnswer::Said(out) => {
            let accels: Vec<&str> = out
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.ends_with(':'))
                .collect();
            if accels.is_empty() {
                Outcome::skip(
                    None,
                    "qemu-system-x86_64 -accel help ran and listed no accelerator",
                )
            } else {
                Outcome::ok_with(format!("accelerators: {}", accels.join(" ")))
            }
        }
        EmuAnswer::Absent => Outcome::skip(
            Some(sys::ENOENT),
            "no qemu-system-x86_64 on PATH, so the accelerator list could not be read",
        ),
        EmuAnswer::Unusable(e) => Outcome::denied(e),
        EmuAnswer::Exited(code) => Outcome::skip(
            None,
            format!("qemu-system-x86_64 -accel help ran and exited {code}; the list is unread"),
        ),
        EmuAnswer::Failed(why) => Outcome::skip(None, why),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_name_is_unique() {
        let mut names: Vec<&str> = PROBES.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two probes share a row name");
    }

    #[test]
    fn the_set_is_at_least_the_size_its_acceptance_names() {
        // TODO/probe.md T-0101's Prove asserts `.probes | length >= 24`.
        assert!(PROBES.len() >= 24, "{} probes", PROBES.len());
    }

    #[test]
    fn the_attribution_block_is_the_reference_order() {
        // TODO/milestones.md T-1101 compares this block against
        // experiments/results/attribute.txt row for row, so its names and its
        // order are the reference's `attrOrder`.
        let got: Vec<&str> = PROBES
            .iter()
            .filter(|p| p.group == Group::Attribution)
            .map(|p| p.name)
            .collect();
        assert_eq!(
            got,
            vec![
                "mount(2) bogus target",
                "umount2(2) bogus target",
                "pivot_root(2) bogus paths",
                "process_vm_readv(bogus pid)",
                "pidfd_getfd(-1,-1) [control]",
                "kcmp(-1,-1,...) [control]",
                "fsopen(tmpfs)",
                "fsmount(tmpfs)",
                "open_tree(/tmp, CLONE)",
                "move_mount(-> bogus dest)",
                "move_mount(-> /tmp/mm-probe)",
                "openat(detached tmpfs, O_DIRECTORY)",
                "seccomp(NEW_LISTENER)",
                "open /proc/self/mem O_RDONLY",
                "open /proc/self/mem O_RDWR",
                "landlock_create_ruleset(VERSION)",
            ]
        );
    }

    #[test]
    fn the_machine_block_is_six_legs_in_order() {
        // TODO/podvm.md T-1301: `crate::machine` and the report read the
        // verdicts through `MACHINE_LEGS`, so the block and the constant
        // answer together or not at all.
        let got: Vec<&str> = PROBES
            .iter()
            .filter(|p| p.group == Group::Machine)
            .map(|p| p.name)
            .collect();
        assert_eq!(got.as_slice(), MACHINE_LEGS);
    }

    #[test]
    fn the_supervise_block_is_three_legs_in_order() {
        // TODO/supervise.md T-0606: `crate::supervise`, the report and the
        // selection read the verdicts through `SUPERVISE_LEGS`, so the
        // block, the constant and the three readers answer together or not
        // at all.
        let got: Vec<&str> = PROBES
            .iter()
            .filter(|p| p.group == Group::Supervise)
            .map(|p| p.name)
            .collect();
        assert_eq!(got.as_slice(), SUPERVISE_LEGS);
    }

    #[test]
    fn the_propagation_change_is_only_ever_probed_inside_a_mount_namespace() {
        // The one probe with an unbounded effect on the caller's machine.
        for p in PROBES {
            if p.name.contains("MS_SLAVE") {
                match p.kind {
                    Kind::Child { ns_flags, .. } => {
                        assert_eq!(ns_flags & sys::CLONE_NEWNS, sys::CLONE_NEWNS, "{}", p.name)
                    }
                    Kind::Clone(_) => panic!("{} must run a body", p.name),
                }
            }
        }
    }

    /// T-0503: only the OPEN row with `Ok` promises a pty. Existence is not
    /// function, a denial is the refusal itself, and a skip never ran.
    #[test]
    fn only_an_ok_open_promises_a_pty() {
        fn findings(rows: Vec<(&'static str, Outcome)>) -> crate::Findings {
            crate::Findings {
                rows,
                identity: crate::identity::Identity::default(),
                writable: Vec::new(),
                self_exe: String::new(),
            }
        }
        let open = "open(/dev/ptmx, O_RDWR)";
        let stat = "stat(/dev/ptmx)";
        assert!(ptmx_usable(&findings(vec![(open, Outcome::ok())])));
        assert!(!ptmx_usable(&findings(vec![(
            open,
            Outcome::denied(crate::sys::Errno(2))
        )])));
        assert!(!ptmx_usable(&findings(vec![(
            open,
            Outcome::skip(None, "nope")
        )])));
        assert!(!ptmx_usable(&findings(vec![])));
        // Stat Ok beside an open denial: the node is there and will not open.
        assert!(!ptmx_usable(&findings(vec![
            (stat, Outcome::ok()),
            (open, Outcome::denied(crate::sys::Errno(13))),
        ])));
    }

    /// T-1003: only an `Ok` open promises a FUSE mount. Existence is not
    /// function, a denial is the refusal itself, and a skip never ran.
    #[test]
    fn only_an_ok_open_promises_a_fuse_mount() {
        fn findings(rows: Vec<(&'static str, Outcome)>) -> crate::Findings {
            crate::Findings {
                rows,
                identity: crate::identity::Identity::default(),
                writable: Vec::new(),
                self_exe: String::new(),
            }
        }
        let open = "open(/dev/fuse, O_RDWR)";
        assert!(fuse_usable(&findings(vec![(open, Outcome::ok())])));
        assert!(!fuse_usable(&findings(vec![(
            open,
            Outcome::denied(crate::sys::Errno(2))
        )])));
        assert!(!fuse_usable(&findings(vec![(
            open,
            Outcome::skip(None, "nope")
        )])));
        assert!(!fuse_usable(&findings(vec![])));
    }

    /// T-1317: only an `Ok` chroot row promises entry. A denial is the
    /// up-front refusal itself, and a skip never ran.
    #[test]
    fn only_an_ok_chroot_promises_entry() {
        fn findings(rows: Vec<(&'static str, Outcome)>) -> crate::Findings {
            crate::Findings {
                rows,
                identity: crate::identity::Identity::default(),
                writable: Vec::new(),
                self_exe: String::new(),
            }
        }
        let chroot = "chroot(/tmp)";
        assert!(chroot_usable(&findings(vec![(chroot, Outcome::ok())])));
        assert!(!chroot_usable(&findings(vec![(
            chroot,
            Outcome::denied(crate::sys::Errno(1))
        )])));
        assert!(!chroot_usable(&findings(vec![(
            chroot,
            Outcome::skip(None, "nope")
        )])));
        assert!(!chroot_usable(&findings(vec![])));
    }

    /// T-1003: the FUSE row is a Census leg in the outer environment before
    /// any chroot, beside the ptmx pair it copies the rule from.
    #[test]
    fn the_fuse_row_opens_the_node_outside_any_namespace() {
        let p = PROBES
            .iter()
            .find(|p| p.name == "open(/dev/fuse, O_RDWR)")
            .expect("the FUSE row is in PROBES");
        assert!(matches!(p.group, Group::Census), "{}", p.name);
        match p.kind {
            Kind::Child { ns_flags, .. } => {
                assert_eq!(ns_flags, 0, "{} runs outside any namespace", p.name)
            }
            Kind::Clone(_) => panic!("{} is not a child probe", p.name),
        }
    }
    #[test]
    fn the_root_listing_block_is_three_outer_legs_in_order() {
        let want = [
            "readdir(/)",
            "open(/bin, O_RDONLY) by name",
            "creat(/, O_CREAT|O_EXCL)",
        ];
        let mut at = 0;
        for p in PROBES {
            if at < want.len() && p.name == want[at] {
                assert!(
                    matches!(p.group, Group::Census),
                    "{} is not a Census leg",
                    p.name
                );
                match p.kind {
                    Kind::Child { ns_flags, .. } => {
                        assert_eq!(ns_flags, 0, "{} runs outside any namespace", p.name)
                    }
                    Kind::Clone(_) => panic!("{} is not a child probe", p.name),
                }
                at += 1;
            }
        }
        assert_eq!(at, want.len(), "root-listing legs missing from PROBES");
    }
}
