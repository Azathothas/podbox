# Current limits

This page records constraints and unproved behavior.
A historical success applies to its captured revision and conditions.
Read [PROGRESS](../TODO/PROGRESS.md) for current verification.

## Host and payload

- A chroot changes paths. It does not provide process or network isolation.
- The namespace path provides mount behavior only.
- A userland entry retains the host root and shared host resources.
- A static x86_64 payload on a host that denies chroot runs on the
  native-static rung (T-1652): Syscall User Dispatch (Linux 5.11) gives
  each of its system calls to podvm, which resolves each path in the
  image with `openat2(RESOLVE_IN_ROOT)`. `/proc`, `/sys` and `/dev` are
  the host's, and `/proc/self/exe` names the guest program. Threads,
  `fork`, `vfork` and `execve` keep the rung; an `execve` of a dynamic
  program maps the image's interpreter beside it (T-1731), and podvm
  drops its own interposer from `LD_PRELOAD` there. A static child of a dynamic program on the
  loader family enters the same rung through podvm (T-1724). A volume
  (`-v HOST:GUEST`) is a host directory descriptor of the handler: a path
  under GUEST resolves in HOST with `RESOLVE_IN_ROOT` there, so `..` stays
  in the volume; `:ro` and `--read-only` are refused on the rung (T-1724). Where the kernel refuses the `prctl`, the payload
  refuses at 125, because its absolute paths would resolve on the host
  (T-1648). Under `--unsafe-host-paths` it runs from a staged memfd with
  the host root. It then reads its own exe as the staged memfd path, so a
  self-locating runtime cannot resolve itself there.
- A userland grandchild that execs a dynamic ELF or a `#!` script enters
  through the image's own loader or interpreter, resolved inside the
  rootfs (T-1646). `fexecve`, `execveat` with flags, `system()` and
  `popen()` take the same route (T-1703). A `memfd` enters through
  `/proc/self/fd/N`, so the started program keeps that descriptor. A
  child ELF of another machine or class (also i386) fails its exec with
  `ENOEXEC` and one line.
- A Unix socket path takes the path table (T-1710), also through `-v`. A
  socket whose host path is longer than 108 bytes binds as
  `/proc/self/fd/N/NAME`; the process that bound it reads its guest name
  back, and another process reads that kernel name.
- A raw call through libc's `syscall()` takes the path table, so libuv's
  `statx` resolves inside the rootfs (T-1697). A raw call with a path that
  the table cannot map, and `io_uring_setup`, answer `ENOSYS`. A program
  that issues the `syscall` instruction itself (a static program, Go,
  liburing) passes the interposer (T-1652).
- A volume (`-v HOST:GUEST`) on a host that denies chroot is a path
  rewrite of the userland loader family, not a mount (T-1653). A write
  lands in HOST. A static program or a raw `syscall` instruction sees
  the image path, and `/proc/mounts` does not list the volume. podvm
  refuses a volume on the chroot rung, the memfd family and the machine
  tier. A working directory or a first program in a volume resolves
  through the same table, to the file of the volume.
- `:ro` and `--read-only` are a read-only layer of the same interposer
  (T-1655): a write through libc under a read-only guest path fails with
  `EROFS`, and `/dev` stays writable. A raw `syscall` instruction passes
  the layer. Each entry with no interposer table refuses both by name.
- Interposition covers cooperative library calls, without a security boundary.
- The embedded interposer pair is x86_64, and only the x86_64 binary embeds it (T-1721). Other payload architectures need another execution path.
- A foreign architecture needs the host interpreter support required by the selected path.
- A pidfd addresses a direct child. The launcher, and podvm for a foreground
  run, is the child subreaper of the payload's tree and kills each
  descendant at the end (T-1708); a process that reparents above it stays
  out of reach.
- A denied terminal operation produces a named refusal.
- Host CA completion changes payload trust data and can be disabled.
- `--network broker` (T-1665) holds a payload to an egress broker of the
  run: on the syscall tier the handler refuses each `connect` but the
  broker's, and on the machine tier the guest network is restricted to a
  forward to the broker. Each other rung refuses it. A payload that
  ignores the proxy variables is refused, not served through the broker.
- `--podvm-tier=syscall` (T-1657) runs an x86_64 program of the image
  on the native-static rung (a dynamic one with the image's interpreter,
  T-1731), and the handler also answers `uname` with the
  release `6.1.0-podvm-syscall`, gives `/tmp` as a directory of the run
  with a cap (`--tmpfs /tmp:size=`, 256 MiB by default; a write over it
  fails with `ENOSPC`), and names guest paths in the links of
  `/proc/self`. The entries of other processes in `/proc` are the
  host's. The cap counts the writes of a descriptor opened under `/tmp`;
  a write through `mmap`, `sendfile`, `splice` or `copy_file_range`, and a
  rename into `/tmp`, are not counted. A dynamic program and a detached
  run refuse by name. The handler shares the payload's address space, so
  the tier is a compatibility layer, not a boundary.
- `--podvm-tier=runsc` (T-1729) needs a `runsc` (`PODVM_RUNSC` or
  `PATH`, with its `gvisor-bin/` sidecars) and a host that grants user
  namespaces; otherwise it refuses by name. It serves a foreground `run`
  only, with no terminal, a numeric `--user`, and no `--timeout`,
  `--tmpfs`, `--network broker` or `--podvm-confine`. The network is the
  host's. Where podvm runs as root, `runsc` keeps a mounted `null-netns`
  in `<store>/runsc/state`; unmount it before you remove the store.
- Each container has its own rootfs (T-1709): `run` and `create` copy
  the extracted image into `<store>/trees/<id>/`, and completion and the
  payload write only in that copy. A write in one container is not read
  by another container or by a later run. The copy is a `FICLONE` of each
  file where the store's filesystem supports it (XFS, btrfs), else a
  kernel copy of the bytes. The cost on the cloud host (experiment 430):
  alpine 3.20, 7.4 MiB, 12 to 114 ms by copy on ext4 and 5 ms by
  `FICLONE` on XFS; golang 1.24.7-bookworm, 814 MiB, 2.3 to 7.9 s by copy
  (two runs, the second on a loaded host) and 0.8 s by `FICLONE`. A copy uses the disk of the image again, a clone
  does not. `rm` and `--rm` remove the copy. The copy keeps no extended
  attribute, no time of a directory and no hard link inside the image.
  The image's own tree is the source. `exec` with an image reference,
  `cp` with an image reference, `extract`, and the machine and incus
  tiers use the image's own tree.

Forced FUSE mode enters through a rung-complete read-only server; its live
entry arm and the permitted-tmpfs entry arm prove automatically where a
host grants the device and the mount, and refuse naming them where it
does not. The embedded-rootfs format is the `save` OCI-layout tarball,
verified per blob on `load`.

⛔ **Live FUSE and tmpfs entry are a limit of the measured host, not pending work.**
The prover exists: `experiments/358-ladder-rungs.sh` drives both entry arms
and asserts payload word, bytes, and cleanup wherever the host grants
`/dev/fuse` and `mount(2)`. The Windows base of the parked lane granted
neither, measured at the node, at `mount(2)`, and inside a user namespace.
A cloud session has not run the prover; its result is -. No task clears
the limit, so none owns it. T-1003 in [packaging](../TODO/packaging.md)
stays `done`: everything it claimed to deliver is delivered and its
`Prove` exits 0.

## SSH

The source build produces separate `node`, `operator`, `proxy`, and `shell` executables.
The remote CLI needs compatible helpers beside podvm or on PATH.
Each release target since beta.10 carries the helper archive
(T-1405 in [podssh](../TODO/podssh.md)). The beta.9 assets contain podvm only.

The relay legs carry 10 s DNS and write deadlines; a stalled peer ends
the operation loud instead of wedging the loop. Node redial pairing is
proven on the loopback fake relay; pairing against the live relay is the
one remaining clause of T-1406, which is `partial` and owns both the
bounds and the reconnect evidence. The earlier T-1403 acceptance proves
concurrent sessions under its tested conditions.

The passwd shim compiles from retained source on a glibc-dynamic lane
and serves the restricted host to exit 42; no workspace or release
build links it. T-1401 records that proof (`experiments/382-restricted-sshd.sh`).

The server session does not provide a real pty.
It supplies its own line discipline under ForceCommand.
The server configuration controls exec and subsystem requests.

## Guests

`run --podvm-tier=machine IMAGE CMD` boots a disposable guest from the
image and gives the command's exit code (T-1656). The guest
runs under KVM where `/dev/kvm` opens and under TCG elsewhere;
`PODVM_MACHINE_ACCEL=kvm` asks for KVM and refuses by name without it.
The guest kernel is the pinned `vmlinuz-virt` of Alpine 3.22.6, which
podvm fetches into the store. The image needs `/bin/sh`. The guest
has QEMU's user-mode network: it reaches the host's loopback at
10.0.2.2. `-v HOST:GUEST[:ro]` is a 9p mount of the host directory (T-1723): a write
in the guest lands there with the host user's ownership, and `:ro` refuses a
write with EROFS. The 9p modules come from the pinned modloop. With a kernel
of `PODVM_MACHINE_KERNEL` a volume is a copy, and the run says so. stderr comes back on its own
serial port. stdin reaches the guest where it is a pipe or a file (at
most 64 MiB, read before the boot). With `-t` the command runs on a terminal
of a virtio console with the size of the host terminal, stdin goes there
live, and stdout and stderr share it, as under docker (T-1723); a change of
the size during the run does not reach the guest. `create` and `exec`
with the machine tier report the profile and exit 125.
`podvm machine ssh` and `podvm windows` boot a guest now
([architecture](architecture.md)).

`run --podvm-tier=emulate IMAGE CMD` runs the command under a static
qemu-user of `PATH` (`qemu-ARCH-static`, else `qemu-ARCH`), and the
emulator itself runs on the native-static rung (T-1651). So each path of
the guest resolves in the image: a path absent from the image is ENOENT
and never a file of the host, an absolute link stays in the image, and an
exec of the guest enters the emulator again. An image of another
architecture runs the same way, with `--platform`. The cost is that of
qemu's TCG (`go version` of the golang image: about 50 s, with the
extraction). `exec` and `create` refuse the tier.

`run --podvm-tier=incus IMAGE CMD` runs the command in a disposable
instance of an incusd that answers on its socket (`PODVM_INCUS_SOCKET`, or
`INCUS_DIR/unix.socket`), and gives its stdout, stderr and exit code
(T-1718, [incus](incus.md)). podvm imports its extracted image with the
owners that the image intended, and is the init of the instance. The
command is not PID 1. `run -d` and `create` keep the instance, and `ps`,
`start`, `stop`, `rm`, `logs` and `exec` with the tier act on it; incusd
keeps its state, and podvm keeps no record of it (T-1722). `logs -f`,
`inspect` and the ladder do not take the tier yet; `-v` is a disk device of the
instance (T-1722), and `--podvm-mem` and `--podvm-qemu-arg` are refused by
name, and the instance has no network
under route A. A run where no incusd answers exits 125 and names the
socket. The image carries podvm as the init of the instance (T-1722).

The owner lifted the KVM deferral on 2026-11-09 ([decisions](decisions.md)).
A session can start a KVM guest where `/dev/kvm` exists, with a timeout,
and no emulator may stay after the run. The cloud host has no `/dev/kvm`,
so a cloud session runs guests under TCG. `kvm.yml` (T-1682) runs the KVM
leg on a hosted runner.

The Windows guest has no network in the current driver.
The operator supplies a licensed disk image.
The runtime does not publish or redistribute that image.
T-1112 records the TCG, KVM, and DOS proofs separately.
The 2026-09-30 KVM runs on the Windows base failed at setup or command
shutdown. An earlier KVM success does not establish current reliability.
The operator removed the ReactOS clause from T-1112 on 2026-10-02.

## Parked Windows lane

The Windows lane on wsl-toolkit is parked since 2026-11-06
([decisions](decisions.md)). These limits apply to that lane only.

Nested KVM in the Windows base can stop the Windows host. On 2026-09-30 a
KVM proof left an emulator that SIGKILL did not remove, and the host then
failed. The KVM proof driver requires `--accept-host-risk`, and refuses
beside another emulator or below 6144 MiB available. With `--unattended`
it starts the proof detached and holds a T-1609 watchdog on its session.
It refuses unless the watchdog answers a probe first.

`/dev/kvm` in the base is `crw-rw---- root kvm`, set by hand because the
node shipped as `crw------- root root` and the guest account is uid 1000.
That mode is per boot: the base's own
`/usr/lib/tmpfiles.d/static-nodes-permissions.conf` replays
`z /dev/kvm 0666 - kvm -` on every boot. After a reboot, reapply the mode
or run the guest as root.

A Windows base probe can report usable without cgroup delegation.
On the recorded host, engine memory and CPU limits are accepted but not enforced.
Read the current `base status --probe` result before a resource-limit claim.
The delegation is absent from the host's WSL configuration, and the
toolkit does not install it. `base ensure --repair` reports the condition
and does not repair it (T-1610). No task owns it, because only a change
to the host clears it.

⚠ **`podvm` does not depend on cgroup delegation and does not enforce it either way.** The CLI
refuses `--memory` and `--cpus` on its own parity table
([cli](../TODO/cli.md) T-0804, `resource limits need a cgroup this runtime does
not grant`), so a caller gets exit 125 and a named reason instead of an accepted
flag that does nothing. The hazard is the engine underneath podvm, which
accepts those limits silently; [RULES](../TODO/RULES.md) section 8 says an
option accepted by an engine does not prove enforcement.
