# Architecture

podbox has a rootfs execution path and a machine execution path.
The CLI selects the path from arguments and runtime probes.
[The generated state](runtime-state.md) lists the source-defined mechanisms.

## Rootfs execution

1. The probe runs operations in disposable children.
2. The image client selects a platform and verifies registry content.
3. The extractor applies ordered layers and whiteouts.
4. The completion layer prepares devices, account files, resolver data, and package configuration.
5. The entry layer resolves the payload in the selected execution context.
6. The supervisor manages process state, logs, signals, and exit status.

Extraction stores intended ownership in a sidecar.
It restores mapped ownership only where the selected path permits it.
The sidecar does not change the kernel's ownership checks.

## Selection and entry

`crates/podbox-probe/src/select.rs` defines runtime rungs.
Selection reports available mechanisms. Entry reports the mechanism it actually enters.

| Path | Behavior |
| --- | --- |
| namespace | A mount namespace and private temporary filesystem, followed by rootfs entry |
| supervise | Notification mediation where the required argument-reading and descriptor mechanisms hold |
| chroot | Root change with host kernel and shared process and network resources |
| userland | Image loader plus interposition for dynamic payloads; staged memfd for eligible static payloads |
| interpose | Library-call compatibility for payloads that the object reaches |
| unsupported | Refusal with the missing operation |

The namespace path does not create user, PID, or network namespaces.
A stronger request cannot silently receive a weaker result.
[Limits](limits.md) gives host and payload restrictions.

## Launch modes

`PODBOX_MODE` requests a foreground rootfs launch mode.
The launch code validates the request before staging or acquisition.
Detached run, create, exec, and machine entry reject modes they do not drive.
`crates/podbox-cli/src/ladder.rs` owns this validation.

The modes can stage an executable in memfd, use a temporary rootfs,
reuse a cache, or enter by path. These modes describe storage and launch.
They do not add an isolation claim.

The vendored userland loader is not linked into the runtime.
The current userland entry lives in `crates/podbox-enter/src/userland.rs`.
[THIRD_PARTY](../THIRD_PARTY.md) records the carried loader's actual patch state.

## Interposer

The interposer is a separate crate outside the Cargo workspace.
The build produces one object for glibc and one for musl.
The CLI build script embeds the produced objects and reports absent inputs.

It supplies path, ownership, identity, device, and proc compatibility.
It cannot cover every syscall or static payload.
The embedded pair targets x86_64.
The runtime checks payload and object architecture before use.

## Machine execution

`--podbox-tier=machine` selects a guest rather than rootfs entry.
The `podvm` executable name selects that tier by default.
An explicit tier flag takes precedence over the executable name.

The Linux guest path uses kernel and initramfs inputs, serial execution,
bounded waits, and a per-run guest.
The Windows path uses a disk overlay, FAT mailbox, guest agent, and QMP.
The DOS path supplies its own guest preparation and command protocol.
The machine probe selects KVM, TCG, or a named refusal from measured host operations.

## SSH

The `podbox-ssh` crate owns transport, TLS, WebSocket frames, multiplexing,
server supervision, and the interactive session without a pty.
Remote dispatch calls the helper executables beside podbox or on PATH.
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
