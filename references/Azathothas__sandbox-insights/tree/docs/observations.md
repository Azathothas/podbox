# Observation notebook

This notebook is the local factual base for the capability model. It preserves
the operation, result, discriminator, and limit that matter for engineering.
Kernel-specific rows are marked `CAPTURED`; the current Windows authoring host can
audit their text and recompute derived quantities but cannot rerun them.

## 1. Runtime identity and namespace scope

Status: `CAPTURED`

- The process reported uid 0 and a full-looking capability mask.
- The user-namespace map was `0 1000 1`: only namespace ID 0 existed, mapped to
  host ID 1000.
- Supplementary-group changes were disabled through the namespace `setgroups`
  control.
- A newly created user and network namespace could be owned by the process while
  the inherited procfs mount remained owned by an outer namespace.
- Namespace-granted capabilities disappeared when a process with no mapped uid
  crossed `execve`.

Consequence: capabilities are meaningful only relative to the namespace named by
the kernel check. Namespace-sensitive network setup may have to run in the process
that created the namespace rather than through an external helper.

## 2. Syscall entry and mount-stage separation

Status: `CAPTURED`

- A legacy `mount` call returned unconditional `EPERM`.
- Invalid-argument controls for disputed calls reached path-, PID-, or fd-shaped
  errors such as `ENOENT`, `ESRCH`, and `EBADF` when the syscall body executed.
- During one policy epoch, `fsopen` and `fsmount` created a detached mount,
  `move_mount` could not attach it, and path access through the detached object
  returned `EACCES`.
- On 2026-09-09 the new mount entry points executed; on 2026-09-10 the same syscall
  numbers returned a pre-entry denial while a separate namespace-creation path
  remained available.

Consequence: old and new syscall numbers, then creation, attachment, and use, must
be probed independently. A single `mount supported` Boolean is invalid.

## 3. Path policy and writable surface

Status: `CAPTURED`

The stable writable set was `/tmp`, `/dev/shm`, `/workspace`, and `/state`.
Mapped-owner paths within that set accepted operations that equally permissive
paths elsewhere denied. Detached-tree access failed even after mount-object
creation succeeded.

The verdict shape is consistent with a path-aware policy layered after ordinary
DAC. The exact policy implementation remains an `OPEN` reconstruction task; the
observation supports path dependence, not a universal policy label.

## 4. OCI extraction and ownership

Status: `CAPTURED`

- Applying an unmapped uid or gid during extraction returned `EINVAL`, not
  `EPERM`.
- Multiple extractors failed at the same ownership boundary.
- Ownership-neutral extraction succeeded when intended uid, gid, mode, applied
  values, and reason were written to a path-keyed sidecar.
- Layer order, opaque and per-path whiteouts, links, path containment, and symlink
  traversal still required explicit implementation.
- A self-referential package-cache symlink removed by a later whiteout remained
  under plain archive extraction and broke the package manager.

Consequence: ownership neutrality is a rootfs architecture, not a command-line
flag. Virtual metadata must never be presented as kernel-enforced ownership.

## 5. Network policy

Status: `CAPTURED`

- Unix-domain and UDP bind paths worked.
- TCP bind returned `EPERM` for loopback, wildcard, and host-facing addresses.
- Loopback TCP connect returned `EPERM`, while a closed host-LAN port reached
  `ECONNREFUSED` through the same socket syscall family.
- External TCP port 80 changed from denied to allowed during the observation
  window.

Consequence: the verdict depends on protocol, address, and port. Seccomp cannot
inspect a sockaddr pointer, so syscall-number filtering alone cannot explain the
matrix. Control protocols should prefer Unix sockets or FIFOs locally and measured
working transports elsewhere.

## 6. Device reachability and resource ceilings

Status: `CAPTURED`

- The host exposed evidence of KVM and TUN drivers, but their device nodes were
  absent.
- Real-device `mknod`, `devtmpfs` mounting, handle-based opens, descriptor donation,
  and ring-0-adjacent alternatives each failed at a distinct gate.
- Character device 0:0 was not a valid capability witness because Linux treats it
  as the overlay whiteout special case; a real device number was required.
- A 500 MiB per-file ceiling terminated a VM launcher with `SIGXFSZ`.
- A 64 MiB temporary filesystem produced short writes, missing output, and
  misleading hangs when tools did not preserve the underlying `ENOSPC`.

Consequence: driver registration, node visibility, node creation, descriptor
acquisition, and usable ioctl are separate stages. Resource probes must precede
large image, memory-backing, and snapshot creation.

## 7. Observation without ptrace

Status: `CAPTURED`

- Ptrace attachment was denied.
- A child-launched tracee could install seccomp user notification on itself and
  pass the listener descriptor to a supervisor through `SCM_RIGHTS`.
- While the tracee was blocked, `/proc/<pid>/mem` allowed argument reads where
  `process_vm_readv` was unavailable.
- Selected open operations could be emulated in supervisor context and the real fd
  injected through `SECCOMP_IOCTL_NOTIF_ADDFD`.
- An outer seccomp `ERRNO` action outranked the tracee's inner `USER_NOTIF`, making
  already-denied syscalls invisible to the inner observer.

The backend therefore has four required legs: listener creation, listener handoff,
argument read, and result or fd injection. Failure of any leg disqualifies an
enforcement or dry-run claim.

## 8. Trace fidelity and semantic intervention

Status: `CAPTURED`

- Relative paths had to be resolved through the tracee's cwd or
  `/proc/<pid>/fd/<dirfd>`, never through a numerically equal tracer descriptor.
- File reports required scanning inherited descriptors after successful `execve`
  and close-on-exec processing.
- Continued syscalls had an unknown result; reporting fabricated success corrupted
  summaries.
- JSON streams required begin, syscall, signal, exit, and summary events plus
  escaping of every tracee-controlled string.
- Anti-debug rewrites had to preserve byte length and carry a seam window across
  iovec boundaries.
- Suppressed exits needed a per-process attempt bound and external timeout because
  code after an exit path could fault or loop.

Consequence: tracing is an execution-context problem, and intervention features
must be opt-in because they change program semantics.

## 9. Rootfs entry and environment completion

Status: `CAPTURED`

The reliable order was: validate the root, open all outer-world descriptors,
`chroot`, `chdir("/")`, resolve the executable inside the new root, then `exec`.
Resolving too early selected host paths; recomputing storage paths after an
environment reset created a second empty root and produced false missing-binary
errors.

Minimal roots also needed deterministic user/group files, host resolver material,
trust initialization, package-manager identity adjustment, and deliberate handling
of device-like files. If `/dev/null` was absent, shell redirection could create a
growing regular file with that name.

## 10. Software-emulated microVM workflow

Status: `CAPTURED`

- An Alpine guest reached userspace in approximately 1.6 seconds on the microvm
  machine profile.
- Eight 128 MiB guests reached their workload markers in 1.97 seconds total,
  equivalent to 4.06 boots per second for that run.
- A 76 MB migration snapshot resumed into three independent clones.
- Two guests exchanged traffic through UDP host forwards when TCP listeners were
  unavailable.
- In-guest package installation and compilation produced the same benchmark
  checksum as native and chroot runs.
- An initramfs with an empty `/dev` became usable after appending a raw cpio archive
  containing `/dev/console`.
- The `pc,acpi=off` profile delivered interactive userspace serial; the tested
  `microvm` artifacts delivered printk but not the same shell output.

Under TCG, the guest kernel is separate from the host kernel but executes as logic
inside the QEMU process. The boundary is the emulator process, not hardware-backed
virtualization.

## 11. Benchmark samples

Status: `LOCAL-DERIVATION` from `experiments/data/benchmark-samples.csv`

| Route | Samples (Mops/s) | Checksum |
|---|---:|---|
| native host | 721.4, 754.8 | `165be307` |
| chroot | 845.9, 866.8 | `165be307` |
| microVM under TCG | 35.1, 35.1 | `165be307` |

Cross-products of the four native-speed samples and two guest samples produce a
20.55x to 24.70x interval. The defensible claim is a rounded 20-25x compute cost
for this one CPU-oriented workload. It is not a general TCG slowdown factor.

## 12. Reproduction boundary

The gate can rerun local structure checks, literal evidence anchors, consistency
rules, PDF generation, and benchmark arithmetic. It cannot rerun Linux namespaces,
seccomp, path policy, ptrace, KVM, or QEMU workloads on the Windows authoring host.
Those rows remain `CAPTURED`; future Linux runs should add new numbered experiments
and must not silently overwrite the conditions recorded here.
