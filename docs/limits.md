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
verified per blob on `load`.

⛔ **Live FUSE and tmpfs entry are a limit of this host, not pending work.**
The prover exists: `experiments/358-ladder-rungs.sh` drives both entry arms
and asserts payload word, bytes, and cleanup wherever the host grants
`/dev/fuse` and `mount(2)`. This host grants neither, measured at the node,
at `mount(2)`, and inside a user namespace. No task clears it, so none owns
it. T-1003 in [packaging](../TODO/packaging.md) stays `done`: everything it
claimed to deliver is delivered and its `Prove` exits 0.

## SSH

The source build produces separate `node`, `operator`, `proxy`, and `shell` executables.
The remote CLI needs compatible helpers beside podbox or on PATH.
The standalone beta.9 assets contain podbox only.
That gap closed at beta.10, where every release target carries the helper
archive, and T-1405 in [podssh](../TODO/podssh.md) is done.

The relay legs carry 10 s DNS and write deadlines; a stalled peer ends
the operation loud instead of wedging the loop. Node redial pairing is
proven on the loopback fake relay; pairing against the live relay is the
one remaining clause of T-1406, which is `partial` and owns both the
bounds and the reconnect evidence. The earlier T-1403 acceptance proves
concurrent sessions under its tested conditions.

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
failed. The operator permitted an unattended run on 2026-10-02, on this
host only, conditional on a watchdog outside the guest. That watchdog has
landed: `--unattended` starts the proof detached and holds a T-1609
watchdog on its session, and refuses unless the watchdog answers a probe
first. The driver also still requires `--accept-host-risk`, and refuses
beside another emulator or below 6144 MiB available.

`/dev/kvm` in this base is `crw-rw---- root kvm`, set by hand because the
node shipped as `crw------- root root` and the guest account is uid 1000.
That mode is per boot: the base's own
`/usr/lib/tmpfiles.d/static-nodes-permissions.conf` replays
`z /dev/kvm 0666 - kvm -` on every boot. After a reboot, reapply the mode
or run the guest as root.

## Development base

A Windows base probe can report usable without cgroup delegation.
On the recorded host, engine memory and CPU limits are accepted but not enforced.
Read the current `base status --probe` result before a resource-limit claim.
Keep other base instances and host distributions unchanged.

⛔ **This is a limit of the base, not pending work, and no task owns it because
no task can clear it.** The delegation is absent from the host's WSL
configuration and the toolkit does not install it. `base ensure --repair`
reports the condition and declines to repair it; the same reading is recorded
under T-1610, which installed everything the KVM proof needs and left this
unchanged. It clears if and only if the base gains cgroup delegation, which is a
change to the host rather than to this tree, so an entry here would be a row
with no implementable clause.

⚠ **`podbox` does not depend on it and does not enforce it either way.** The CLI
refuses `--memory` and `--cpus` on its own parity table
([cli](../TODO/cli.md) T-0804, `resource limits need a cgroup this runtime does
not grant`), so a caller gets exit 125 and a named reason instead of an accepted
flag that does nothing. The hazard is the engine underneath podbox, which
accepts those limits silently; [RULES](../TODO/RULES.md) section 8 says an
option accepted by an engine does not prove enforcement.
