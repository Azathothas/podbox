# Changelog

Current behavior and limits are in [README](README.md) and
[Limits](docs/limits.md). Task status is in [INDEX](TODO/INDEX.md).
Earlier repository change records are retained in
[the captured changelog](docs/history/CHANGELOG.md-before-2026-09-30.txt).
They describe their recorded revision, not the current build.

## Unreleased

### Fixed: the paths that the loader gives, 2026-11-11

Record: T-1705; issue 230.
Deployed: not deployed; no version change.

- On the userland rung, a path that the loader gives (`dladdr`, the link
  map, `/proc/self/maps`) is a host path under the rootfs, and the
  interposer added the rootfs prefix to it a second time, so an open of it
  failed with `ENOENT` (CoreCLR of the official runner exited 137). A path
  under the host side of the table now passes unchanged.

### Fixed: realpath of the exe on the userland rung, 2026-11-11

Record: T-1704; issue 229.
Deployed: not deployed; no version change.

- On the userland rung, `realpath` and `canonicalize_file_name` of
  `/proc/self/exe` gave the image's loader, while `readlink` gave the
  program; the .NET apphost of the official runner exited 154. Both now
  give the program. `/proc/PID/exe` with the process's own pid is the same
  link, so GNU `realpath /proc/self/exe` prints the program too.
- A program built with `_FORTIFY_SOURCE=2` called `__realpath_chk`,
  `__readlink_chk`, `__readlinkat_chk` and `__getcwd_chk` past the
  interposer and read host paths. The interposer exports the four, with
  glibc's length check.

### Breaking and fixed: the entered rung, 2026-11-11

Record: T-1712, T-1713, call C14 of `docs/decisions.md`; issues 237 and
238.
Deployed: not deployed; no version change.

- `--podvm-tier` of `run`, `create` and `exec` takes `machine` only.
  `--podvm-tier=chroot` had no effect: it ran the ladder, and on a
  chroot-denied host it entered the userland rung with exit 0. It is a
  flag error at 125 now, and the message names the value and
  `.EnteredRung`.
- `system info --format '{{.EnteredRung}}'` reads `userland` wherever
  chroot(2) is denied, whatever rung the probe selects. It read `chroot`
  where the probe selected `supervise`, and `namespace` where it selected
  `namespace`, while `run` entered the userland rung.

### Changed, 2026-11-11

Record: T-1716; PodUtils/podcrates#52 and #54.
Deployed: not deployed; no version change.

- podvm takes the 11 shared crates of `PodUtils/podcrates` at tag
  `v0.15.0`. The volume rules live in `pod-enter` only: a `-v` pair under
  `/proc` or `/.podvm` is refused by name, and a missing `-w` directory
  inside a volume names the volume `HOST:GUEST`, not the image.

## 0.2.0-beta.2 (2026-11-10)

### Breaking: podbox is podvm, 2026-11-10

Record: the owner's ruling of 2026-11-05, a clean break with no
compatibility with the released binaries; commit `98112cb`
(`rename: podbox is podvm`, 2026-11-06); T-1671.
Deployed: version 0.2.0-beta.2 is the first release with the name podvm.
The version moves to 0.2 because the rename breaks each caller.

- The binary is `podvm`, not `podbox`, and there is no `podbox` alias.
  The crates are `podvm` (was `podbox-cli`), `podvm-probe`, `podvm-image`,
  `podvm-extract`, `podvm-complete`, `podvm-enter`, `podvm-gate`,
  `podvm-buildstate`, `podvm-release`, `podvm-supervise`, `podvm-windows`,
  `podvm-ssh`, `podvm-interpose`, and `podvm-measure` (was `podbox-podvm`,
  bins `podvm-parity` and `podvm-workload`). The tool binaries follow:
  `podvm-gate`, `podvm-dev`, `podvm-count`, `podvm-plant`, `podvm-smoke`,
  `podvm-verify` and the rest.
- Every `PODBOX_*` variable is `PODVM_*`, with no fallback to the old
  name. `PODBOX_LANE_INSTANCE` and `PODBOX_WSL_INSTANCE` are both
  `PODVM_WSL_INSTANCE`.
- Every own flag `--podbox-*` is `--podvm-*`: `--podvm-tier`,
  `--podvm-qemu-arg`, `--podvm-mem`, `--podvm-timeout` and the rest.
- The name the binary is invoked under no longer selects the machine tier.
  `podvm` was an argv0 alias that defaulted to it; it is now the binary's
  own name and takes the ladder like every other name. Only
  `--podvm-tier=machine` selects the machine tier. `system install-names`
  installs `docker` and `podman` only.
- State and guest paths move with the name: `/.podbox/` is `/.podvm/`, the
  store, configuration and runtime directories are under `podvm`, not
  `podbox`. Nothing migrates existing state.
- The WSL instance is `podvm` (`wsl-toolkit --instance podvm`,
  `wsl-toolkit-podvm`), not `podbox`.
- The releases up to `v0.1.0-beta.12` keep their `podbox-*` asset names and
  their signing identity, the nightly workflow of `Azathothas/podbox`;
  `podvm-verify` checks each tag through `v0.1.0-beta.13` against that
  identity, and each later tag against the identity of `PodUtils/podvm`.
- The repository is `PodUtils/podvm`. From this release, the assets are
  `podvm-ARCH` and `podvm-ssh-ARCH.tar.gz`, and the nightly workflow of
  `PodUtils/podvm` at the tag signs each one.

### Security: a layer whiteout cannot delete outside the rootfs, 2026-11-10

Record: T-1698; commit `c89b769`.
Deployed: version 0.2.0-beta.2. Each earlier podbox and podvm release has
the defect.

- A whiteout entry of an image layer skipped the path check that every
  other layer entry gets. A whiteout of `..` at the root of a layer
  deleted the contents of the directory above the destination. On a
  kernel with no `openat2`, `../.wh.NAME` reached past the destination.
  Thus a crafted image could delete files outside its rootfs.
- Now podvm refuses such a layer, and the refusal names the layer digest
  and the entry. Do not pull an image that you do not trust with an
  earlier release.

### Fixed, 2026-11-10

Record: the release run 42800267302 and its nightly run 42800606497 on the
tag `v0.2.0-beta.1`; T-1671.
Deployed: version 0.2.0-beta.2.

- The licence inventory of a release lists only the packages that a
  release triple builds. Before, it read the packages of each platform and
  refused `r-efi`, a UEFI crate with no licence file.
- The nightly build installs `qemu-user-binfmt` on Ubuntu 26.04, where
  `qemu-user-static` is only a virtual package.

### Fixed in the code, 2026-11-10

Record: the task and the issue on each line, done on 2026-11-09 and
2026-11-10.
Deployed: version 0.2.0-beta.2.

- A foreground run sometimes exited 0 with no output. podvm closed the
  saved standard output before its copy thread wrote the last bytes. Each
  rung had the defect. T-1700, commit `72523d7`.
- npm can find its modules on the loader family. The interposer now sends
  a raw `syscall` of a path through the path table, because libuv calls
  `statx` that way. The `mkstemp` family keeps the guest name. T-1697,
  T-1699, T-1701.
- A payload larger than 128 MiB runs: podvm reads a bounded head to
  classify it and stages it in bounded parts. Before, podvm refused
  `node:alpine` and other large runtimes. T-1647, #90.
- On the loader family, `-v HOST:GUEST` and `-v HOST:GUEST:rw` work with
  no mount and no `/dev/fuse`. A working directory (`-w`) and a first
  program inside a volume work: each resolves to the file of the volume.
  podvm refuses `:ro`, named volumes and the other rungs by name. T-1653
  (partial), #130.
- On a host that denies `chroot`: a relative path gets the rootfs prefix
  once; a glibc payload is admitted against its full `NEEDED` closure,
  read inside the rootfs; `dlopen` and `dlmopen` load guest objects; and a
  dynamic child program runs through the image's loader. T-1643 to T-1646,
  #86 to #89, #127 to #129, #131.
- A static payload on a host that denies `chroot` refuses at exit 125 and
  names the opt-in `--unsafe-host-paths`. Before, a static Go payload ran
  into a `GOROOT` error. T-1648, #91.
- A `#!` payload runs its interpreter from the image, and `--strict` knows
  the capability gaps of the loader family. T-1649, T-1650, #92, #93.
- `doctor` measures whether a memfd can execute and names
  `vm.memfd_noexec`; the disk-free row reads the soft `RLIMIT_FSIZE`. The
  probe remedy agrees with its stance. `stop -t 0` sends SIGKILL at once.
  The `windows --podvm-mem` row states what the verb does. README and
  SECURITY name `v0.1.0-beta.12` as the last podbox release. T-1688 to
  T-1693, #212 to #217.

### Changed, 2026-11-10

Record: T-1681, T-1677, T-1669, T-1671; the commits `deps: podcrates ...`.
Deployed: version 0.2.0-beta.2.

- podvm takes the 11 shared crates of `PodUtils/podcrates` at tag `v0.12.0`:
  `pod-core`, `pod-hostprobe`, `pod-tls`, `pod-proxy`, `pod-term`,
  `pod-supervise`, `pod-guest`, `pod-enter`, `pod-oci`, `pod-extract` and
  `pod-rootfs`.
  Each HTTP request goes through `pod-proxy`, and `ureq` is gone. With
  `v0.12.0`, the table of a run (`pod-enter`'s `Maps`) resolves each guest
  path, the `#!` parser is `pod-enter`'s, and one `statfs` names a
  `noexec` volume source.
- The workspace builds with Rust 1.99 and edition 2024.
- A release starts from the dispatched workflow `release.yml`, which
  creates the tag and then starts the nightly workflow on it. A pushed tag
  no longer starts a release.

## 0.2.0-beta.1 (2026-11-10)

The tag `v0.2.0-beta.1` is at `16c9ec6` and has no release: its nightly
build failed before the publish (run 42800606497). Version 0.2.0-beta.2
carries each change of that version.

## 0.1.0-beta.13

The tag `v0.1.0-beta.13` has no release: its build failed. Version
0.2.0-beta.2 carries each change below.

- The static build compiles under Rust 1.99: the `open` family's fixed
  third argument is deliberate, so the deny-by-default
  `invalid_runtime_symbol_definitions` lint is allowed at the definition
  and stays a no-op on older toolchains through `unknown_lints`. See
  T-1601.
- The registry fixture's manifest, config, config-digest and layer fields
  are asserted on as bytes, not only as digests, in every test target
  that compiles the fixture. See T-1602.
- The secret-shape regex check is gone. Credential scanning is
  `.github/workflows/secrets.yml`, a pinned trufflehog binary over the
  working tree and the whole history. See T-1603.
- The detached-stdio serialisation mutex no longer turns one test's panic
  into the next test's failure, and a failed detached start now reports
  the exit it took instead of only counting. The one-of-three start
  failure is still open. See T-1604.
- Nine Batch 3 closure paragraphs read as nine paragraphs again, the
  source maps name the four crates the port added and mark each retired
  script as a shim, and the limits page stops describing closed gaps as
  open work. See T-1608, T-1611, T-1612.

## 0.1.0-beta.12

- The release tag names the workspace version. Beta.11's binary
  reports beta.10; the nightly build now refuses a mismatched tag
  before one arch builds. See T-1423.

## 0.1.0-beta.11

Superseded: its binary reports 0.1.0-beta.10. Beta.12 carries the
same source with the version string corrected. See T-1423.

- The no-chroot loader family is complete: anchored maps, loader-symlink
  resolution, payload-visible exe, image and container exec, a detached
  launcher, and script refusal naming the file and its shebang. See
  T-1407 through T-1413.
- Exit codes follow docker: absent reads 127, not invocable reads 126,
  podbox failures stay 125. Strict classifies degradations. Runs are
  quiet by default with a verbose banner. Inspect carries the OCI
  config. See T-1414 through T-1417.
- Store gates refuse on a noexec store, verify and heal blobs, and name
  the host once on transport errors. See T-1418 through T-1420.
- A foreground run leaves a container record. See T-1421.
- SSH carries DNS and write deadlines with reconnect, and the compiled
  shim serves exit 42 to a restricted sshd. See T-1406 and T-1401.
- The ladder runs FUSE, tmpfs, cache, rundir, and memfd rungs with an
  OCI-tarball pack flow. See T-1003 and T-1411.
- The gate plants its kept-session check with a batch-file vehicle on
  Windows. See T-1422.
- Known limits: the static golang binary answers exit 2 naming GOROOT;
  the full exit-code drive needs an engine; KVM and ReactOS stay
  deferred to an operator-present run. See T-1409, T-1414, T-1350,
  and T-1112.

## 0.1.0-beta.10

- The source-state page is generated from manifests, enum declarations, and helper entry points. The gate rejects a changed snapshot.
- Development builds verify input bytes, conditions, and all five output binaries. Interposer objects are built before the CLI.
- The Windows job can export the CLI, helpers, and build record. The KVM proof takes explicit binary and disk inputs.
- Release packaging adds four static SSH helpers, dependency licence texts, a checksum, and a signature for each target.
- Current documents separate source behavior from historical proof. T-1003 and T-1401 are partial because their original acceptance is incomplete.
- The read-only pg-toolkit review supplied the build-freshness method. No runtime code from that tree is linked.
- `podbox windows run` no longer pipes the emulator's output without reading it. A noisy emulator could block and read as a guest timeout. A failed run now names the end of `emulator.log`. See T-1112.
- The KVM proof driver refuses without `--accept-host-risk`, beside another emulator, or below 6144 MiB available. A 2026-09-30 run preceded a Windows host failure. See T-1350.

The full Linux gate and fault plants pass. KVM repetition and publication are
being checked. [PROGRESS](TODO/PROGRESS.md) owns their current state.

## 0.1.0-beta.9

The published assets contain the standalone CLI. The source now has relay,
interactive, remote, and machine SSH paths. The later source commits were
not part of the beta.9 assets. Helpers must be supplied separately for that
release. See the captured changelog for the earlier version sequence.
