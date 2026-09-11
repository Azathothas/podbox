# The environment, measured

The runtime class this repository characterizes: a container on a
bare-metal multi-tenant host whose tenant is uid 0 **inside a user
namespace**, under a seccomp filter, with a write-allowlist policy and
an address-aware network policy layered below the syscall boundary.
Every table here is produced by a script in `experiments/`; the run
cited is in `experiments/logs/` with its conditions block. The host is
re-audited by its operator continuously — re-run the census before
trusting any row, and amend this file where the host disagrees.

**The defining property of the class: the process looks maximally
privileged and is pervasively denied.** uid 0, every capability bit
set (`CapEff: 000001ffffffffff`), `Seccomp: 2` — and most of what
container, virtualization and debugging tooling is built on answers
EPERM. Nothing in `id` or `/proc/self/status` suggests a restriction,
so every failure reads first as a bug.

## The identity block

```text
uid=0 gid=0 groups=0,65534
CapEff:  000001ffffffffff
CapBnd:  000001ffffffffff
Seccomp: 2      Seccomp_filters: 1
NoNewPrivs: 1
uid_map: 0 0 1   (an earlier revision of this instance mapped 0 -> 1000)
setgroups: deny
```

Read carefully, it already names the mechanisms:

- A full capability mask is what a process gets as **root in a user
  namespace** — by construction, regardless of what the parent held.
  It says nothing about which namespace each capability is scoped to.
- `groups=0,65534`: 65534 is the **overflow gid**, what `getgroups(2)`
  returns for a supplementary group with no mapping in the caller's
  userns. The unmapped supplementary group is a deliberate tell.
- `Seccomp: 2` states that a filter is installed. It says nothing
  about what the filter contains. The contents are characterized by
  probing, not by reading (there is no dump).

## Mechanism N — the user namespace with a partial ID map

The map covers one id. Consequences, all measured:

| operation | verdict | why |
| --- | --- | --- |
| `chown`/`lchown` to any id outside the map | `EINVAL` | an unmapped id translates to INVALID_UID/GID; the kernel reports a *mapping* failure as Invalid argument, not as a permission failure |
| `setuid` to a non-root id | `EINVAL` | same mapping rule — "rootless" is unreachable by definition |
| `setgroups(0)` | `EPERM` | `setgroups: deny` is part of the userns contract |
| write to a directory owned by an unmapped id | `EACCES` | `capable_wrt_inode_uidgid()` requires the inode's owner to be mapped; CAP_DAC_OVERRIDE does not apply |
| `mknod` of a real device number | `EPERM` | CAP_MKNOD is checked in the *initial* userns for device nodes |

**`mknod` with dev 0 is a whiteout and measures nothing** — the kernel
exempts it from the capability check. A device-node probe needs a real
device number; running both, at the same path, in the same directory,
proves the denial is capability-based rather than path-based, because
no path-scoped policy can distinguish two device numbers.

## Mechanism F — the seccomp number denylist

Refused **pre-execution** (EPERM even for arguments the kernel would
reject inside the syscall body — that is the discriminator, see
`docs/attribution.md`):

```text
mount  umount2  pivot_root  fsopen  open_tree  move_mount
unshare  setns  ptrace  bpf  keyctl  open_by_handle_at
process_vm_readv  process_vm_writev
```

Two structural facts:

1. **It is a denylist of syscall NUMBERS.** What it denies is the
   legacy door to each operation. Modern doors to the same operations
   are denied only if separately named (and `fsopen`/`open_tree` were,
   in this instance's current revision — an earlier revision left them
   open; the closure was observed live between sessions).
2. **`clone(2)`/`clone3(2)` with namespace flags execute** (`clone3`
   with a bogus argument answers `EINVAL` — the kernel body rejected
   the argument, so the call was not filtered). `clone(CLONE_NEWNS)`
   succeeds where `unshare(CLONE_NEWNS)` is refused: a process can
   hold a private mount namespace and still be unable to mount
   anything into it. See mechanism-gap below.

## Mechanism M — path-scoped write policy

Writes land only on a fixed allowlist: `/tmp`, `/dev/shm`,
`/workspace`, `/state`. Everything else — `/`, `/etc`, `/usr` — denies
the write. A seccomp filter **cannot** implement this (it sees the
syscall number and six registers; it cannot dereference a pathname
pointer), so a path-scoped policy is a distinct mechanism by
construction. The signature matches a Landlock ruleset (the Landlock
ABI is present on this instance: `landlock_create_ruleset` succeeds),
and the allowlist includes a distinctive path (`/state`) that only
policy would choose. It is policy, not contract: it can be reconfigured
by the operator at any time.

Two provenance effects ride along with M:

- `/proc/self/mem` opens read-only; writes answer `EACCES`.
- `uid_map`/`gid_map` are **unwritable by anyone**, including a parent
  for its own namespace child: `map_write()` checks
  `file_ns_capable()` against the *procfs mount's* user namespace
  (the initial one), which no tenant process holds. This one check is
  what makes the namespace gap's privileges in-process-only.

## Mechanism P — the address-aware network policy

Measured rows (experiment 15-):

| probe | verdict |
| --- | --- |
| `bind` TCP loopback/wildcard | `EPERM` |
| `bind` UDP loopback/wildcard, inet4 and inet6 | OK |
| `bind` AF_UNIX | OK |
| `connect` TCP loopback | allowed — the kernel answered (`ECONNREFUSED` for a closed port) on this instance |
| `connect` TCP external :80 / :443 | allowed on this instance |

The *shape* of the denial set matters more than its current extent:
denials that distinguish addresses, ports and protocol families cannot
be seccomp (again: seccomp cannot dereference the `sockaddr`
pointer). Address- + port- + family-aware `EPERM` with live
reconfigurability is the signature of a cgroup-BPF `SOCK_ADDR` program
family attached to the sandbox cgroup. Tool design consequence: **never
plan around TCP listeners**; unix sockets, UDP and serial/fifo
transports are the durable set. Earlier revisions of this instance
denied loopback TCP connect and port :80 egress; both were observed
relaxing live. Treat the current rows as one point in a reconfigured
space.

## The namespace gap and its boundary

`clone3` with `CLONE_NEWUSER|CLONE_NEWNET|NEWPID` creates namespaces
the process OWNS (experiment 40-). Owner-namespace capability checks
pass, in-process: a raw-ICMP socket, a veth pair via raw netlink, a
private PID tree all work in the child, where the parent's identical
operations answer `EPERM`.

The boundary is measured, not assumed:

1. `uid_map` stays **empty** (mechanism M's procfs provenance), so
   namespace children run with an unmapped uid.
2. `execve` from an unmapped-uid process **drops the capability set**
   granted at namespace creation.
3. Mount attachment (`move_mount`) is denied even in the child.

Therefore the gap buys **in-process privileges** — private fabrics,
private PID trees, in-process setup — not a container runtime. Any
tool that needs the gap must probe for it at runtime; it is
operator-closable without notice (the adjacent mount-API gap WAS
closed between sessions while this one survived).

## What is plainly available

- `mmap`/`mprotect` **RWX** — software emulation and JITs work.
- `memfd_create` + exec from it.
- `landlock_create_ruleset` — the ABI is present (usable both as the
  explanation for M and as a tool a tenant could apply to itself).
- `openat2`, `io_uring` — the modern API surface reaches the kernel.
- chroot(2) — unfiltered, needs only CAP_SYS_CHROOT in the tenant's
  own userns (held).
- seccomp **user-notification** — the tracee can filter itself, hand a
  tracer the listener, and be shepherded: all six primitives proven
  (experiment 45-). The one boundary: an outer `ERRNO` filter outranks
  `USER_NOTIF`, so syscalls the sandbox denies never notify.
- Outbound network (TCP + UDP), DNS.
- qemu TCG — full machine emulation, ~2s to guest userspace (50-),
  with the `RLIMIT_FSIZE` 500 MiB-per-file ceiling on guest images and
  a 64 MiB `/tmp`.

## Hard limits

| limit | value | impact |
| --- | --- | --- |
| `RLIMIT_FSIZE` | 500 MiB per file | memory images/disks above it die with `SIGXFSZ`; size guests under it |
| `/tmp` | 64 MiB tmpfs | stage on `/workspace`; a FULL `/tmp` makes writers lose output **silently** |
| `/etc/passwd` | absent, root fs read-only | subjects that resolve the invoking user at startup need an environment-completion shim or an appliance with its own |
| `/dev` | not listable by the tenant; no `/dev/kvm`, `/dev/net/tun` nodes | host drivers ARE present (`/proc/misc`) — the wall is node exposure, not hardware |
| device nodes | uncreatable (`mknod` initial-ns check) | guest images get device nodes **byte-written into cpio archives**; the guest kernel's unpacker creates them |

## The KVM wall, closed

The host runs KVM (`kvm_amd` loaded, misc 232 in `/proc/misc`) — the
hardware is not the wall. Every tenant-side path to a `/dev/kvm` fd is
individually measured closed: the node is absent from every reachable
mount view; `mknod` of it needs initial-ns CAP_MKNOD; mounting
devtmpfs fails the `FS_USERNS_MOUNT` gate; `open_by_handle_at` is
filtered; no fd-donor process exists in the PID namespace; and the
ring-0 shortcuts (`iopl`, `kexec_load`, `perf_event_open`, `bpf`) are
all individually denied. Each closure is a distinct kernel gate, not
one magic wall — which is why each needs its own probe.
