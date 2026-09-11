# Capability model and hard-won techniques

## 1. The environment is a vector, not a privilege level

A process can be root in one user namespace, own a newly created network
namespace, lack authority over the procfs mount it reads, be denied one syscall
number while a newer interface executes, and be constrained by path- or
address-aware policy that capability bits cannot express.

Use seven independently measured planes:

| ID | Plane | Direct evidence | Engineering consequence |
|---|---|---|---|
| N | identity and namespace | uid/gid maps, namespace owners, `setgroups` | which IDs exist and where a capability is meaningful |
| F | syscall filter | pre-entry errno for a syscall number | legacy and modern interfaces may diverge |
| M | path policy | same operation, different path, different verdict | DAC metadata alone cannot predict access |
| W | network policy | verdict changes by protocol, address, or port | transport must be selected empirically |
| D | device exposure | driver, node, fd, and ioctl stages | host hardware may remain unreachable |
| R | resource ceiling | file size, blocks, inodes, memory, tasks, time | prerequisites can fail before the target mechanism runs |
| V | virtualization and observation | KVM, TCG, ptrace, user notification, proc memory | backend availability is compositional, not Boolean |

Do not merge these planes into a single inferred policy. Their failure signatures
overlap, but their remedies do not.

## 2. Diagnostic techniques that prevent false conclusions

### 2.1 Invalid-argument errno discrimination

A seccomp filter evaluates a syscall before its body and cannot dereference a
pathname or sockaddr pointer. Call a disputed syscall with an argument whose
in-kernel validation has a characteristic error:

| Probe | Evidence the body executed | Likely pre-entry result |
|---|---|---|
| missing mount destination | `ENOENT` or another path-shaped error | unconditional `EPERM` |
| impossible process ID | `ESRCH` | unconditional policy result |
| invalid descriptor | `EBADF` | unconditional policy result |

This is a discriminator, not universal proof of seccomp: a later LSM can also
return `EPERM`. Pair it with positive controls and a control-flow explanation.

### 2.2 Probe creation, attachment, and use separately

“Mount support” is at least three questions:

1. Can a filesystem context and detached mount be created?
2. Can the mount be attached to the namespace?
3. Can a path-resolving operation use it?

The same decomposition applies to devices (driver, node, fd, ioctl), tracing
(listener, notification, argument read, result injection), and VMs (artifact,
VMM, accelerator, console, network, lifecycle). A success at one stage says
nothing about the next.

### 2.3 Namespace ownership and procfs provenance

Linux capabilities are checked relative to a namespace owner. Two consequences
are particularly easy to miss:

- A process can own a new user and network namespace and gain `CAP_NET_ADMIN`
  there while the same netlink operation fails in the parent network namespace.
- A process can own a child user namespace and still be unable to write its ID
  maps when the relevant procfs mount is owned by an outer namespace.

If a nested process has no mapped uid, `execve` can drop its namespace-granted
capabilities. Namespace-sensitive setup may therefore have to remain in the
creating process as a linked helper instead of spawning a familiar command.

### 2.4 Use paired witnesses to defeat special cases

- Character device 0:0 is an overlay whiteout special case. Pair it with a real
  device number such as 1:3 when testing `mknod` authority.
- `unshare(CLONE_NEWUSER)` from a multithreaded Go process can return `EINVAL`
  independently of sandbox policy. Use a single-threaded helper or `clone`.
- A missing fixture is `could not run`, not `denied`.
- A wrapper must derive its exit status from the named operation; a clean wrapper
  exit does not prove its grandchild succeeded.

## 3. Rootfs and image construction

### 3.1 Ownership-neutral extraction is an architecture

An OCI layer is ordinary archive I/O until it restores an unmapped uid or gid.
At that boundary, ownership changes return `EINVAL` because the requested ID does
not exist in the current mapping.

Extract without applying unmapped ownership and record the intent:

```json
{"path":"etc/shadow","intended":{"uid":0,"gid":42,"mode":"0640"},
 "applied":{"uid":0,"gid":0},"reason":"gid 42 unmapped"}
```

The sidecar supports diagnostics, re-export, and optional ownership virtualization.
It does not change kernel DAC and must not be described as real ownership.

### 3.2 Implement the complete layer contract

An extractor must:

- apply layers in order;
- process `.wh.<name>` and `.wh..wh..opq` after each layer;
- preserve hard links and symlinks;
- reject absolute paths and parent traversal;
- reject traversal through a symlink created earlier in the same layer;
- preflight blocks, inodes, and the per-file ceiling;
- retain the failure record when extraction stops.

A strong regression fixture creates a self-referential cache symlink and removes
it with a later whiteout. Plain archive extraction leaves the loop behind.

### 3.3 Root entry has a strict order

```text
validate the rootfs
open required outer-world descriptors
chroot(rootfs), then chdir("/")
resolve the executable inside the new root
exec
```

Opening descriptors first preserves stdio, logs, PTYs, and explicitly exposed
devices. Resolving the executable after the switch avoids executing a host path.
Persist storage identity before environment reset so cleanup cannot accidentally
target a newly computed empty root.

### 3.4 Environment completion is product behavior

Minimal images can require deterministic completion:

- synthesize `/etc/passwd` and `/etc/group` when lookups are required;
- install current resolver material instead of trusting a build-time file;
- initialize trust stores or keyrings rather than disabling verification;
- adapt package-manager privilege-drop users that are unmapped;
- invoke compilers by absolute path when `argv[0]` controls helper discovery;
- provide regular-file device shims only when their degraded semantics are enough.

If `/dev/null` is absent, shell redirection can create a growing regular file with
that name. Validate file type before starting a workload.

## 4. Compatibility, tracing, and supervision

### 4.1 Path and ownership interposition are different jobs

An `LD_PRELOAD` path mapper can provide a per-process path view without mounts.
It does not make an unmapped `chown` succeed. A complete compatibility tier needs:

- path virtualization for the `*at` family plus reverse mapping for `getcwd`,
  `readlink`, `readdir`, and `realpath`;
- ownership virtualization backed by the metadata sidecar.

Static binaries, direct syscalls, and libc bypasses remain outside coverage. Reject
known-ineligible payloads, but never infer complete coverage from ELF metadata.

### 4.2 Seccomp notification is a four-leg backend

The ptrace-free sequence is:

1. the tracee enables no-new-privileges and installs a user-notification filter;
2. it sends the listener fd to its supervisor through `SCM_RIGHTS`;
3. the supervisor reads arguments while the tracee is blocked;
4. it continues or emulates the call and injects real fds when required.

Probe all four legs before selecting the backend. If listener delivery works but
argument reads fail, silently continuing the real syscall can turn a claimed
dry-run into mutation. Refuse the tier or require explicit passthrough.

Outer seccomp action precedence also matters: an outer `ERRNO` decision outranks
an inner `USER_NOTIF`, so already-denied syscalls are invisible to the inner
observer.

### 4.3 Preserve the tracee's execution context

- Resolve relative paths through the tracee's dirfd or cwd, never the tracer's.
- Resolve numeric dirfds through `/proc/<pid>/fd/<n>`.
- Scan inherited descriptors after successful `execve` and close-on-exec handling.
- Keep unresolved paths unresolved instead of guessing.
- Represent continued syscalls with an unknown return, not fabricated success.
- Escape every tracee-controlled JSON string.
- Emit begin, syscall, signal, exit, and summary events.

### 4.4 Intervention must preserve shape and remain bounded

Anti-debug rewriting of `/proc/*/status` and `wchan` should replace detected values
with equal-length zero sequences. Vector reads need a seam window across iovec
boundaries. Timing checks and legitimate debugger behavior remain outside the
safe spoofing envelope.

Suppressing `exit_group` can expose later probes, but code after an exit path can
fault or loop. Preserve the suppressed status for immediate-fault interpretation,
cap attempts per process, keep faults visible, and enforce an external timeout.

### 4.5 Readiness is an event

Fixed sleeps do not establish process or VM state. Use pidfds, readiness fds,
nonce markers, or a protocol acknowledgment. When a bounded state pool is full,
answer the pending notification with an explicit continue or error; never drop it
and leave the tracee blocked forever.

## 5. MicroVM engineering without KVM

### 5.1 Prove device unavailability as a chain

An absent `/dev/kvm` is only the first link. Check every reachable node view,
real-device creation, devtmpfs creation, handle-based opening, descriptor donation,
and relevant privileged alternatives. Each path has a distinct gate. Select TCG
only after the required KVM descriptor is proven unreachable.

### 5.2 TCG preserves workflows, not native performance

The recorded workflow reached userspace, booted an eight-member fleet, resumed
three clones from one migration snapshot, moved guest traffic through UDP host
forwards, installed packages, compiled code, and produced checksum-identical
benchmark output.

For one CPU-oriented workload, TCG measured 35.1 Mops/s versus 721.4-866.8 Mops/s
across native and chroot samples, a 20.55-24.70x interval. This is a workload-bound
measurement, not a general TCG factor.

Under TCG, the guest kernel executes as logic inside QEMU. The boundary is the
emulator process and its correctness, not CPU virtualization hardware.

### 5.3 Machine profiles have asymmetric consoles

- `pc,acpi=off` supported interactive userspace serial but required the driver to
  terminate QEMU after the guest completion marker.
- `microvm` was useful for printk-oriented fleet workloads, but the tested guest
  artifacts did not deliver the same userspace serial channel.
- An initramfs with empty `/dev` became usable after appending a raw cpio archive
  containing `/dev/console`, avoiding host-side device creation.

Treat machine, kernel, initramfs, console, and shutdown behavior as one versioned
profile.

### 5.4 Frame serial execution and guest-time snapshots

Hold both FIFO ends open read-write before QEMU starts. Wait for readiness, verify
a random handshake, and wrap each command in line-anchored begin/end markers with
an explicit exit code. Retry feeding only during lossy bootstrap; never duplicate
delivery after framed execution begins.

Measure resume latency with guest uptime at snapshot and the first post-resume
tick. Host process start and log arrival include unrelated overhead. Report the
guest-time bound together with tick precision.

## 6. Route selection

| Need | Route | Required disclosure |
|---|---|---|
| trusted code, maximum speed | chroot | no kernel, process, mount, or network isolation |
| native shared-kernel confinement | namespaces + seccomp + path policy | exact live controls and missing surfaces |
| hostile code or VM lifecycle | QEMU/TCG | emulator-process boundary and measured workload cost |
| syscall observation | ptrace, else seccomp notification | backend limits and outer-filter blind spots |
| cooperative dynamic compatibility | libc interposition | incomplete raw-syscall coverage and no security property |

Requested semantics choose the route. Compatibility may degrade visibly. An
isolation request fails if no qualifying route is available.
