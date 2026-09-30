# Current limits

This page records constraints and unproved behavior.
A historical success applies to its captured revision and conditions.
Read [PROGRESS](../TODO/PROGRESS.md) for current verification.

## Host and payload

- A chroot changes paths. It does not provide process or network isolation.
- The namespace path provides mount behavior only.
- A userland entry retains the host root and shared host resources.
- A static userland payload reads its own exe as the staged memfd path,
  so a self-locating runtime cannot resolve itself there.
- A userland grandchild that execs an absolute path runs it past the
  loader mapping and fails where the host holds no such file.
- Interposition covers cooperative library calls, without a security boundary.
- The embedded interposer pair is x86_64. Other payload architectures need another execution path.
- A foreign architecture needs the host interpreter support required by the selected path.
- A pidfd addresses a direct child. It does not contain all descendants.
- A denied terminal operation produces a named refusal.
- Host CA completion changes payload trust data and can be disabled.

Forced FUSE mode enters through a rung-complete read-only server; its live
entry arm and the permitted-tmpfs entry arm prove automatically where a
host grants the device and the mount, and refuse naming them where it
does not. The embedded-rootfs format is the `save` OCI-layout tarball,
verified per blob on `load`. T-1003 in [packaging](../TODO/packaging.md)
owns the remaining live-entry prover (a mount-granting host) and the
binary-appended footer as future work.

## SSH

The source build produces separate `node`, `operator`, `proxy`, and `shell` executables.
The remote CLI needs compatible helpers beside podbox or on PATH.
The standalone beta.9 assets contain podbox only.
T-1405 in [podssh](../TODO/podssh.md) owns release packaging for those helpers.

The relay legs carry 10 s DNS and write deadlines; a stalled peer ends
the operation loud instead of wedging the loop. Node redial pairing is
proven on the loopback fake relay; pairing against the live relay stays
open. T-1406 owns the bounds and the reconnect evidence. The earlier
T-1403 acceptance proves concurrent sessions under its tested conditions.

The passwd shim compiles from retained source on a glibc-dynamic lane
and serves the restricted host to exit 42; no workspace or release
build links it. T-1401 owns that build input and live acceptance.

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
