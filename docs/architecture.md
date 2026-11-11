# Architecture

podvm has a rootfs execution path and a machine execution path.
The CLI selects the path from arguments and runtime probes.
[The generated state](runtime-state.md) lists the source-defined mechanisms.
[The code map](code-map.md) names the crate that owns each part.

## Shared crates

Most mechanisms are crates of PodUtils/podcrates (prefix `pod-`).
podvm takes them by git tag; [the code map](code-map.md) names the tag.
Each local `podvm-*` crate gives podvm's names, paths and settings to its
`pod-*` crate and keeps the parts that only podvm needs.
A change to the mechanism of a `pod-*` crate goes to podcrates as a COORD
issue (`AGENTS.md`, section 9).

## Rootfs execution

1. The probe runs operations in disposable children (`pod-hostprobe`).
2. The image client selects a platform and verifies registry content (`pod-oci`).
3. The extractor applies ordered layers and whiteouts (`pod-extract`).
4. The completion layer prepares devices, account files, resolver data, and package configuration (`pod-rootfs`).
5. The entry layer resolves the payload in the selected execution context (`pod-enter`).
6. The supervisor manages process state, logs, signals, and exit status (`pod-supervise`).

Extraction stores intended ownership in a sidecar.
It restores mapped ownership only where the selected path permits it.
The sidecar does not change the kernel's ownership checks.

## Selection and entry

`pod_hostprobe::select` defines the runtime rungs.
`crates/podvm-probe/src/select.rs` re-exports it and keeps podvm's banner cells.
Selection reports available mechanisms. Entry reports the mechanism it actually enters.

| Path | Behavior |
| --- | --- |
| namespace | A mount namespace and private temporary filesystem, followed by rootfs entry |
| supervise | Notification mediation where the required argument-reading and descriptor mechanisms hold |
| chroot | Root change with host kernel and shared process and network resources |
| userland | Image loader plus interposition for dynamic payloads; a staged memfd for a static payload only under `--unsafe-host-paths`, else a refusal at 125 (T-1648) |
| interpose | Library-call compatibility for payloads that the object reaches |
| unsupported | Refusal with the missing operation |

The namespace path does not create user, PID, or network namespaces.
A stronger request cannot silently receive a weaker result.
[Limits](limits.md) gives host and payload restrictions.

## Launch modes

`PODVM_MODE` requests a foreground rootfs launch mode.
The launch code validates the request before staging or acquisition.
Detached run, create, exec, and machine entry reject modes they do not drive.
`crates/podvm/src/ladder.rs` owns this validation.

The modes can stage an executable in memfd, use a temporary rootfs,
reuse a cache, or enter by path. These modes describe storage and launch.
They do not add an isolation claim.

The vendored userland loader is not linked into the runtime.
The current userland entry lives in `src/userland.rs` of `pod-enter`.
[THIRD_PARTY](../THIRD_PARTY.md) records the carried loader's actual patch state.

## Interposer

The interposer is a separate crate outside the Cargo workspace.
`podvm-interpose-build` produces one object for glibc and one for musl.
The CLI build script embeds the produced objects.
An absent object becomes an empty placeholder, and the runtime then refuses the payload by name.

It supplies path, ownership, identity, device, and proc compatibility.
It cannot cover every syscall or static payload.
The embedded pair targets x86_64 on each release architecture.
The runtime checks payload and object architecture before use.

## Machine execution

`--podvm-tier=machine` selects the machine tier. Each other value is a flag error (T-1712).
With no flag the ladder selects, under every name the binary answers to.
`podvm system info --format '{{.EnteredRung}}'` names the rung that a run enters (T-1713).
`crates/podvm/src/tier.rs` owns the flag.

For an OCI image, the machine tier assesses the machine legs and exits 125.
The message names the profile it found (KVM or TCG) or each missing leg.
`run` does not boot a Linux guest yet; T-1656 in [the machine tasks](../TODO/machine.md) owns that work.

Three commands boot a guest now:

| Command | Guest |
| --- | --- |
| `podvm machine ssh` | A Linux guest from a named kernel and initramfs, with a serial transport and a real SSH client |
| `podvm windows run`, or foreground `run --podvm-tier=machine --platform windows/amd64` | A Windows disk with a per-run overlay, a FAT mailbox, a guest agent, and QMP (`pod-guest`) |
| `podvm windows run --guest dos` | FreeDOS from its base cache, with its own guest preparation and command protocol (`pod-guest`) |

The machine probe selects KVM, TCG, or a named refusal from measured host operations.
Under TCG the emulator process is the boundary, not hardware isolation.
Each guest run has bounded waits.

## SSH

The `podvm-ssh` crate owns transport, TLS, WebSocket frames, multiplexing,
server supervision, and the interactive session without a pty.
Remote dispatch calls the helper executables beside podvm or on PATH.
The remote relay path uses the multiplexed protocol.
Local forwarding uses the transport appropriate to its fixed endpoint.

Machine SSH boots the named guest with a serial socket and runs a real SSH client.
The SSH handshake checks the far-end endpoint.
[The SSH decision](decisions/ssh-server-in-a-cage.md) records server restrictions.
[The remote decision](decisions/remote-verb.md) records command placement.

## Store and lifecycle

The image store contains verified blobs, platform-specific image records,
extracted rootfs trees, lifecycle records, configuration, and probe data.
The store opens paths relative to trusted roots where name containment is required.
A mutation commits complete data or preserves the prior usable state.

The launcher holds a pidfd for each direct child.
It does not claim that one pidfd contains descendants.
`exec` enters a fresh context over the stored rootfs.
It does not join all namespaces of the original payload.

An advisory lock needs an explicit unlock when a child can inherit its descriptor.
A descriptor close alone can leave the inherited lock held.
T-0215 in [the image tasks](../TODO/image.md) owns that rule.

## Verification authority

Source and repeatable runs settle behavior.
An architecture statement that conflicts with either is a document defect.
The captured research specification defines product requirements.
It does not establish the current host's capabilities.

[SECURITY](../SECURITY.md) names trust boundaries.
[PROGRESS](../TODO/PROGRESS.md) gives the work order.
