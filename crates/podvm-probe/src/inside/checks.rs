//! The checks of the probe inside the payload, apart from the network
//! ([`super::net`]). Each check that could change a state runs in a
//! disposable child (T-0101) and gives its errno. No check stops, attaches
//! to or signals another process: `ptrace` is measured by
//! `process_vm_readv` of one word, and a mount that succeeds is unmounted
//! at once.

use super::{Record, Verdict};
use pod_hostprobe::sys;

#[cfg(target_arch = "x86_64")]
mod nr {
    pub const BPF: i64 = 321;
    pub const KEYCTL: i64 = 250;
    pub const PERF_EVENT_OPEN: i64 = 298;
    pub const INIT_MODULE: i64 = 175;
    pub const KEXEC_LOAD: i64 = 246;
}
#[cfg(not(target_arch = "x86_64"))]
mod nr {
    pub const BPF: i64 = 280;
    pub const KEYCTL: i64 = 219;
    pub const PERF_EVENT_OPEN: i64 = 241;
    pub const INIT_MODULE: i64 = 105;
    pub const KEXEC_LOAD: i64 = 104;
}

/// The name of an errno for a detail.
pub fn errno_name(e: i32) -> String {
    sys::Errno(e).name().to_string()
}

/// Run `f` in a disposable child: Ok where it returned 0 or more, else the
/// errno.
fn in_child(f: impl FnOnce() -> sys::Sysres) -> Result<(), i32> {
    // SAFETY: the child runs `f`, then exits with no unwinding.
    let pid = match unsafe { sys::clone_fork(sys::SIGCHLD) } {
        Ok(p) => p,
        Err(e) => return Err(e.0),
    };
    if pid == 0 {
        let code = match f() {
            Ok(_) => 0,
            Err(e) => e.0.clamp(1, 255),
        };
        sys::exit_group(code);
    }
    let mut status = 0;
    let _ = sys::wait4(pid, &mut status);
    let code = (status >> 8) & 0xff;
    if status & 0x7f != 0 {
        return Err(-1);
    }
    if code == 0 {
        Ok(())
    } else {
        Err(code)
    }
}

/// A raw call with six arguments.
fn call(n: i64, a: [u64; 6]) -> sys::Sysres {
    // SAFETY: each caller passes arguments that the call reads or ignores.
    unsafe { sys::sys(n, a) }
}

/// A verdict of an escalation: success is ALLOW; EPERM, EACCES and ENOSYS
/// are DENY; an argument error means the call reached the kernel's own
/// check past the permission, so it is ALLOW too.
fn reach(section: &str, target: &str, r: Result<(), i32>) -> Record {
    match r {
        Ok(()) => Record::new(section, target, Verdict::Allow, "the call succeeded"),
        Err(e @ (1 | 13 | 38)) => Record::new(section, target, Verdict::Deny, &errno_name(e)),
        Err(-1) => Record::new(
            section,
            target,
            Verdict::Unknown,
            "the child ended on a signal",
        ),
        Err(e @ (9 | 14 | 22)) => Record::new(
            section,
            target,
            Verdict::Allow,
            &format!("reached the kernel's argument check ({})", errno_name(e)),
        ),
        Err(e) => Record::new(section, target, Verdict::of_errno(e), &errno_name(e)),
    }
}

const CLONE_NEWNS: u64 = 0x0002_0000;
const CLONE_NEWUSER: u64 = 0x1000_0000;
const CLONE_NEWPID: u64 = 0x2000_0000;
const CLONE_NEWNET: u64 = 0x4000_0000;

/// The escalation section: what the payload can raise.
pub fn escalation() -> Vec<Record> {
    let s = "escalation";
    let mut v = Vec::new();
    for (t, flag) in [
        ("unshare-user", CLONE_NEWUSER),
        ("unshare-mount", CLONE_NEWNS),
        ("unshare-pid", CLONE_NEWPID),
        ("unshare-net", CLONE_NEWNET),
    ] {
        v.push(reach(
            s,
            t,
            in_child(|| call(sys::SYS_UNSHARE, [flag, 0, 0, 0, 0, 0])),
        ));
    }
    v.push(reach(
        s,
        "setuid-0",
        in_child(|| call(sys::SYS_SETUID, [0; 6])),
    ));
    v.push(reach(
        s,
        "chroot",
        in_child(|| call(sys::SYS_CHROOT, [c".".as_ptr() as u64, 0, 0, 0, 0, 0])),
    ));
    v.push(reach(
        s,
        "mount-tmpfs",
        in_child(|| {
            let target = c"/tmp";
            let r = call(
                sys::SYS_MOUNT,
                [
                    c"podvm-sandprobe".as_ptr() as u64,
                    target.as_ptr() as u64,
                    c"tmpfs".as_ptr() as u64,
                    0,
                    0,
                    0,
                ],
            );
            if r.is_ok() {
                // MNT_DETACH: the mount goes at once.
                let _ = call(sys::SYS_UMOUNT2, [target.as_ptr() as u64, 2, 0, 0, 0, 0]);
            }
            r
        }),
    ));
    // bpf(BPF_MAP_CREATE) with a zero size: EINVAL past the permission.
    v.push(reach(s, "bpf", in_child(|| call(nr::BPF, [0; 6]))));
    // keyctl(KEYCTL_GET_KEYRING_ID, KEY_SPEC_SESSION_KEYRING, 0).
    v.push(reach(
        s,
        "keyctl",
        in_child(|| call(nr::KEYCTL, [0, (-3i64) as u64, 0, 0, 0, 0])),
    ));
    v.push(reach(
        s,
        "perf_event_open",
        in_child(|| call(nr::PERF_EVENT_OPEN, [0; 6])),
    ));
    v.push(reach(
        s,
        "init_module",
        in_child(|| call(nr::INIT_MODULE, [0; 6])),
    ));
    v.push(reach(
        s,
        "kexec_load",
        in_child(|| call(nr::KEXEC_LOAD, [0; 6])),
    ));
    v.push(read_pid1_memory());
    v
}

/// `process_vm_readv` of one word of PID 1: the access of `ptrace` to it,
/// with no stop and no attach.
fn read_pid1_memory() -> Record {
    let s = "escalation";
    let t = "read-pid1-memory";
    let maps = match std::fs::read_to_string("/proc/1/maps") {
        Ok(m) => m,
        Err(e) => {
            let n = e.raw_os_error().unwrap_or(0);
            return Record::new(
                s,
                t,
                Verdict::of_errno(n),
                &format!("/proc/1/maps: {}", errno_name(n)),
            );
        }
    };
    let Some(addr) = maps
        .lines()
        .next()
        .and_then(|l| l.split('-').next())
        .and_then(|a| u64::from_str_radix(a, 16).ok())
    else {
        return Record::new(s, t, Verdict::Unknown, "/proc/1/maps names no mapping");
    };
    let mut word = [0u8; 8];
    let local = [word.as_mut_ptr() as u64, 8u64];
    let remote = [addr, 8u64];
    let r = call(
        sys::SYS_PROCESS_VM_READV,
        [1, local.as_ptr() as u64, 1, remote.as_ptr() as u64, 1, 0],
    );
    reach(s, t, r.map(|_| ()).map_err(|e| e.0))
}

/// The escape-surface section: host objects that a payload must not reach.
/// `host` is a host path that podvm gives (its store); where the payload
/// can see it, a host path stays reachable.
pub fn escape_surface(host: Option<&str>) -> Vec<Record> {
    let s = "escape-surface";
    let mut v = Vec::new();
    let mut targets: Vec<String> = [
        "/proc/1/root/etc/hostname",
        "/proc/1/environ",
        "/proc/1/mem",
        "/proc/kcore",
        "/proc/sysrq-trigger",
        "/dev/mem",
        "/dev/kmsg",
        "/sys/kernel/security",
        "/var/run/docker.sock",
        "/run/podman/podman.sock",
        "/run/containerd/containerd.sock",
    ]
    .iter()
    .map(|t| t.to_string())
    .collect();
    if let Some(h) = host {
        targets.push(h.to_string());
    }
    for t in targets {
        let r = std::fs::File::open(&t);
        let target = if Some(t.as_str()) == host {
            "host-store"
        } else {
            t.as_str()
        };
        v.push(match r {
            Ok(_) => Record::new(s, target, Verdict::Allow, "opened"),
            Err(e) => {
                let n = e.raw_os_error().unwrap_or(0);
                // A socket opens with ENXIO; it exists.
                if n == 6 {
                    Record::new(s, target, Verdict::Allow, "exists (a socket)")
                } else {
                    Record::new(s, target, Verdict::of_errno(n), &errno_name(n))
                }
            }
        });
    }
    v
}

/// One field of `/proc/self/status`.
pub fn status_field(name: &str) -> Option<String> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(':'))
        .map(|v| v.trim().to_string())
}

/// The identity section: facts, not boundaries.
pub fn identity() -> Vec<Record> {
    let s = "identity";
    let mut v = Vec::new();
    let fact = |t: &str, d: Option<String>| match d {
        Some(d) => Record::new(s, t, Verdict::Allow, &d),
        None => Record::new(s, t, Verdict::Unknown, "no /proc/self/status here"),
    };
    // The ids from the calls, so they hold where no /proc is mounted.
    for (t, n) in [("uid", sys::SYS_GETUID), ("gid", sys::SYS_GETGID)] {
        v.push(match call(n, [0; 6]) {
            Ok(id) => Record::new(s, t, Verdict::Allow, &id.to_string()),
            Err(e) => Record::new(s, t, Verdict::of_errno(e.0), &errno_name(e.0)),
        });
    }
    // TODO/enter.md T-1658: no_new_privs and the seccomp mode from
    // prctl(2) as well, so they hold where no /proc is mounted (the
    // namespace rung); /proc/self/status is the fallback.
    const PR_GET_SECCOMP: u64 = 21;
    const PR_GET_NO_NEW_PRIVS: u64 = 39;
    for (t, f, op) in [
        ("nonewprivs", "NoNewPrivs", PR_GET_NO_NEW_PRIVS),
        ("seccomp", "Seccomp", PR_GET_SECCOMP),
    ] {
        match call(sys::SYS_PRCTL, [op, 0, 0, 0, 0, 0]) {
            Ok(n) => v.push(Record::new(s, t, Verdict::Allow, &n.to_string())),
            Err(_) => v.push(fact(t, status_field(f))),
        }
    }
    for (t, f) in [
        ("capeff", "CapEff"),
        ("capbnd", "CapBnd"),
        ("tracerpid", "TracerPid"),
    ] {
        v.push(fact(t, status_field(f)));
    }
    v.push(kernel());
    v.push(fact("pid", Some(std::process::id().to_string())));
    v
}

/// The release of `uname(2)`: the kernel that the payload is told of.
fn kernel() -> Record {
    #[cfg(target_arch = "x86_64")]
    const UNAME: i64 = 63;
    #[cfg(not(target_arch = "x86_64"))]
    const UNAME: i64 = 160;
    let mut u = [0u8; 390];
    match call(UNAME, [u.as_mut_ptr() as u64, 0, 0, 0, 0, 0]) {
        Ok(_) => {
            let rel = &u[130..195];
            let end = rel.iter().position(|&c| c == 0).unwrap_or(rel.len());
            Record::new(
                "identity",
                "kernel",
                Verdict::Allow,
                &String::from_utf8_lossy(&rel[..end]),
            )
        }
        Err(e) => Record::new(
            "identity",
            "kernel",
            Verdict::of_errno(e.0),
            &errno_name(e.0),
        ),
    }
}

/// The procfs section: what `/proc` shows of other processes.
pub fn procfs() -> Vec<Record> {
    let s = "procfs";
    let mut v = Vec::new();
    v.push(match std::fs::read_dir("/proc") {
        Ok(d) => {
            let n = d
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                })
                .count();
            Record::new(s, "pids-visible", Verdict::Allow, &n.to_string())
        }
        Err(e) => {
            let n = e.raw_os_error().unwrap_or(0);
            Record::new(s, "pids-visible", Verdict::of_errno(n), &errno_name(n))
        }
    });
    for t in [
        "/proc/1/cmdline",
        "/proc/1/status",
        "/proc/self/status",
        "/proc/sys/kernel/yama/ptrace_scope",
    ] {
        v.push(match std::fs::read(t) {
            Ok(b) => {
                let first = String::from_utf8_lossy(&b)
                    .split(['\0', '\n'])
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect::<String>();
                Record::new(s, t, Verdict::Allow, &first)
            }
            Err(e) => {
                let n = e.raw_os_error().unwrap_or(0);
                Record::new(s, t, Verdict::of_errno(n), &errno_name(n))
            }
        });
    }
    v
}

/// The raw-socket section.
pub fn raw_socket() -> Vec<Record> {
    let s = "raw-socket";
    let mut v = Vec::new();
    for (t, a) in [
        ("packet", [17u64, 3, 0x0300, 0, 0, 0]),
        ("inet-raw-icmp", [2, 3, 1, 0, 0, 0]),
        ("inet-dgram-icmp", [2, 2, 1, 0, 0, 0]),
        ("netlink-route", [16, 3, 0, 0, 0, 0]),
    ] {
        v.push(match call(sys::SYS_SOCKET, a) {
            Ok(fd) => {
                let _ = sys::close(fd);
                Record::new(s, t, Verdict::Allow, "opened")
            }
            Err(e) => reach(s, t, Err(e.0)),
        });
    }
    v
}

/// The proxy section: the proxy variables of the payload, scrubbed.
pub fn proxy() -> Vec<Record> {
    ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "NO_PROXY"]
        .iter()
        .map(|n| {
            let val = std::env::var(n).or_else(|_| std::env::var(n.to_ascii_lowercase()));
            match val {
                Ok(v) => Record::new("proxy", n, Verdict::Allow, &v),
                Err(_) => Record::new("proxy", n, Verdict::Absent, "unset"),
            }
        })
        .collect()
}

/// The env section: each variable, with a secret-shaped value scrubbed.
pub fn env() -> Vec<Record> {
    let mut v: Vec<Record> = std::env::vars_os()
        .map(|(k, val)| {
            let k = k.to_string_lossy().to_string();
            let val: String = val.to_string_lossy().chars().take(120).collect();
            let shown = if super::scrub::secret_name(&k) {
                super::scrub::REDACTED.to_string()
            } else {
                val
            };
            Record::new("env", &k, Verdict::Allow, &shown)
        })
        .collect();
    v.sort_by(|a, b| a.target.cmp(&b.target));
    v
}
