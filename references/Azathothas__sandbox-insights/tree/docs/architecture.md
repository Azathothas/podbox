# Unified architecture

Status: `DESIGN-ONLY`. No complete implementation is claimed.

## 1. Objective

Build one front end that can answer a container-shaped request by selecting an
execution route from measured capabilities, while exposing one consistent
image, lifecycle, diagnostic, and tracing model.

The system is a composition of independently testable components:

```text
request + required semantics
            |
       capability probe ---- build/policy epoch cache
            |
        route selector
       /      |       \
  chroot   shared-kernel   qemu/tcg
       \      |       /
      lifecycle + control
            |
   optional tracing/reporting
```

## 2. Requested semantics come first

The request declares requirements, not a preferred mechanism:

```text
security: none | shared-kernel-confined | separate-guest-kernel
filesystem: image-root | path-view | real-mounts
network: host | private | guest-fabric
lifecycle: one-shot | managed | snapshot-capable
observability: none | events | file-report | intervention
performance: native-required | emulation-allowed
```

The route selector returns:

- selected route;
- probes used and their epoch;
- native, degraded, stubbed, and refused semantics;
- a stable machine-readable report plus a one-line human banner.

A weaker route cannot satisfy a stronger security requirement. No fallback is
permitted after launch without a new explicit selection report.

## 3. Capability probe

Represent probes as structured records:

```json
{
  "operation":"move_mount",
  "arguments":"detached tmpfs -> /workspace/probe",
  "result":{"rc":-1,"errno":"EPERM"},
  "discriminator":{"bogus_destination":"ENOENT"},
  "epoch":{"boot_id":"...","kernel":"...","policy_probe":"..."}
}
```

Probe identity, syscall, path, device, network, resource, and virtualization
planes separately. Cache by boot ID plus a short live policy fingerprint;
invalidate on changed tool build, kernel, boot, or any canary verdict.

Keep probes single-threaded until namespace-sensitive work is complete. Run
mutating operations in disposable children. For namespace capabilities that
disappear across `exec`, invoke a linked in-process helper function rather than
an external command.

## 4. Shared image pipeline

All routes consume one content-addressed rootfs store:

1. fetch manifests and blobs by digest;
2. preflight destination blocks, inodes, and per-file size ceiling;
3. extract in process with path containment and OCI whiteouts;
4. apply only mapped ownership; record intended metadata in sidecar JSONL;
5. run idempotent environment completion;
6. freeze an immutable base and create route-specific writable state.

Route adapters:

- **chroot:** direct directory root;
- **shared-kernel:** same directory plus live namespace/seccomp/LSM controls;
- **microVM:** same tree packed as initramfs, with an appended device archive
  and machine-specific init shim;
- **interpose:** sidecar feeds virtual stat/chown responses and path mappings.

This common pipeline is the main unification: image acquisition and extraction
need not be reinvented per isolation mechanism.

## 5. Route engines

### 5.1 Chroot engine

Use only when `chroot` succeeds and the request permits zero isolation. Open
outer-world descriptors first; switch root; resolve the executable afterward;
supervise direct children with pidfds and `waitid`.

Never claim private PIDs, network, mounts, devices, cgroups, or a kernel
boundary. Reject security-sensitive flags rather than accepting ineffective
syntax.

### 5.2 Shared-kernel engine

Assemble only the controls directly proven available in the current epoch:

- namespace creation through the working syscall path;
- per-run seccomp rules;
- Landlock/path policy where available;
- in-process network setup when namespace privileges would be lost on exec;
- chroot entry using the common rootfs.

The operator's outer seccomp, LSM, procfs provenance, and cgroup-BPF policies
remain outside the engine's control. Report them as observed constraints, not
properties provided by this route.

### 5.3 QEMU/TCG engine

Profiles:

- interactive: `pc,acpi=off`, FIFO serial, completion marker, driver-initiated
  QEMU termination after guest halt;
- fleet: `microvm`, 128 MiB/1 vCPU defaults, printk-oriented workload;
- size guard: every file-backed memory or disk object below the measured
  `RLIMIT_FSIZE` with margin;
- network: slirp with UDP host forwards; Unix sockets/FIFOs for control;
- snapshot: QEMU migration stream to a checked destination, then independent
  `-incoming` clones.

An implementation must probe these profiles against its pinned kernel and
initramfs. The observed userspace-serial asymmetry is not safe to generalize to
all guest artifacts.

## 6. Observability plane

Backends:

1. ptrace when a live canary works and attach semantics are required;
2. seccomp user notification for child-launched tracees when all four backend
   legs pass;
3. no tracing, with an explicit reason, when neither qualifies.

The event schema includes begin, syscall, signal, exit, and summary. Syscall
records carry table-arity raw arguments, decoded paths, errno, duration when
known, continuation state, notes, and the classic decoded line. File reports
aggregate by resolved path and operation, mark outcome as success/failure/
unknown, and include post-exec inherited descriptors.

Intervention features are individually selectable. Anti-debug rewrites and
bounded exit suppression are opt-in because they change tracee behavior.

## 7. Lifecycle and state

- pidfd per direct child or QEMU process;
- readiness fd or nonce protocol, never a fixed sleep;
- append-only lifecycle journal with atomic state transitions;
- immutable image data separate from mutable instance state;
- logs opened before root changes and size-bounded;
- snapshot metadata records QEMU version, machine profile, kernel/initramfs
  digests, memory size, and guest uptime marker;
- cleanup is idempotent and never deletes evidence logs automatically.

Grandchildren outside a PID namespace are not owned merely because a direct
child is. A fresh chroot `exec` shares only the filesystem tree. Both facts
belong in `inspect` output.

## 8. Diagnostics

Every refusal names:

```text
requested semantic -> required operation -> observed result -> likely plane
-> available alternatives -> what each alternative does not provide
```

Example:

```text
separate guest kernel requested; KVM unavailable:
  /dev/kvm absent; node creation EPERM; devtmpfs creation EPERM
selected qemu/tcg: emulator-process boundary, not hardware virtualization;
recorded CPU workload was 20-25x below native
```

## 9. Build order and acceptance gates

| Milestone | Deliverable | Acceptance |
|---|---|---|
| M-1 | observation corpus, evidence ledger, work index | every local anchor resolves; conditions are present |
| M0 | capability probe and epoch cache | matches the recorded probe outcomes on versioned fixtures |
| M1 | image store and safe extractor | path-escape fixtures refused; whiteout and sidecar fixtures pass |
| M2 | chroot route | package install + compile + run; banner says no isolation |
| M3 | lifecycle | 20 consecutive create/start/exec/stop/remove passes without sleeps |
| M4 | shared-kernel route | every enabled control has a positive and negative witness |
| M5 | TCG route | boot, framed exec, exit propagation, size guard, UDP data path |
| M6 | snapshot/fleet | 8-member fleet and 3 independent resumes with condition blocks |
| M7 | tracing | backend parity suite plus hostile targets and versioned fixtures |
| M8 | interposition | path view and ownership sidecar both pass; known-ineligible payloads refused |
| M9 | packaging | static front end; dependency/size delta measured; clean-host install |

Each milestone passes automated suites, a driven end-to-end path, and three
independent review lenses. Platform skips remain visible as exit 2.

## 10. Security statement

- chroot and libc interposition are compatibility mechanisms, not isolation;
- shared-kernel confinement narrows only the surfaces its current probes and
  rules cover;
- TCG supplies a guest kernel boundary through QEMU's process and emulation,
  not CPU virtualization hardware;
- outer sandbox policy remains the ultimate boundary for every route;
- a seccomp-notify supervisor that cannot read arguments must fail closed for
  enforcement requests.
