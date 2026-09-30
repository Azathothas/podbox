# Current limits

This page records constraints and unproved behavior.
A historical success applies to its captured revision and conditions.
Read [PROGRESS](../TODO/PROGRESS.md) for current verification.

## Host and payload

- A chroot changes paths. It does not provide process or network isolation.
- The namespace path provides mount behavior only.
- A userland entry retains the host root and shared host resources.
- Interposition covers cooperative library calls, without a security boundary.
- The embedded interposer pair is x86_64. Other payload architectures need another execution path.
- A foreign architecture needs the host interpreter support required by the selected path.
- A pidfd addresses a direct child. It does not contain all descendants.
- A denied terminal operation produces a named refusal.
- Host CA completion changes payload trust data and can be disabled.

Forced FUSE mode remains a refusal, including where the device probe succeeds.
The embedded-rootfs byte store is absent. T-1003 in
[packaging](../TODO/packaging.md) owns those implementations and a permitted
tmpfs live proof.

## SSH

The source build produces separate `node`, `operator`, `proxy`, and `shell` executables.
The remote CLI needs compatible helpers beside podbox or on PATH.
The standalone beta.9 assets contain podbox only.
T-1405 in [podssh](../TODO/podssh.md) owns release packaging for those helpers.

The relay legs have unbounded DNS and write operations.
A peer that stops reading can stop session progress.
Node reconnection pairing lacks a recorded live proof.
T-1406 owns bounds and reconnection evidence.
The earlier T-1403 acceptance proves concurrent sessions under its tested conditions.

The retained passwd shim is not compiled or linked by the current build.
A normal loopback session does not prove the restricted server host.
T-1401 owns that build input and live acceptance.

The server session does not provide a real pty.
It supplies its own line discipline under ForceCommand.
The server configuration controls exec and subsystem requests.

## Guests

The Windows guest has no network in the current driver.
The operator supplies a licensed disk image.
The runtime does not publish or redistribute that image.
T-1112 records the TCG, KVM, and DOS proofs separately.
The fresh 2026-09-30 KVM runs failed at setup or command shutdown.
An earlier KVM success does not establish current reliability.
ReactOS remains unproved in that entry.

Nested KVM in the Windows base can stop the Windows host. On 2026-09-30 a
KVM proof left an emulator that SIGKILL did not remove, and the host then
failed. The proof driver requires `--accept-host-risk` and an operator who
is present. It refuses beside another emulator or below 6144 MiB available.

## Development base

A Windows base probe can report usable without cgroup delegation.
On the recorded host, engine memory and CPU limits are accepted but not enforced.
Read the current `base status --probe` result before a resource-limit claim.
Keep other base instances and host distributions unchanged.
