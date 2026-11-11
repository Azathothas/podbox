# Changelog

Current behavior and limits are in [README](README.md) and
[Limits](docs/limits.md). Task status is in [INDEX](TODO/INDEX.md).
Earlier repository change records are retained in
[the captured changelog](docs/history/CHANGELOG.md-before-2026-09-30.txt).
They describe their recorded revision, not the current build.

## Unreleased

## 0.2.0-beta.4 (2026-11-11)

### Added: a payload that ignores the proxy variables reaches the broker, 2026-11-11

Record: T-1665; issue 184.
Deployed: version 0.2.0-beta.4.

- With `--network broker` on the syscall tier, each exact name of
  `--allow` is in the container's `/etc/hosts` with a fake address
  `198.18.0.N`. A `connect` to it becomes a tunnel of the broker, with the
  credential of the run, so a program that reads no proxy variable reaches
  the allowed host, and the broker's policy still applies.

### Changed: the SSH archive is packaged by `podvm-package-ssh`, 2026-11-11

Record: T-1737.
Deployed: version 0.2.0-beta.4.

- The release archive of the SSH helpers comes from a Rust binary of
  `podvm-release` in place of a shell script. The archive is
  byte-identical, with the same names, notices and checks.

### Added: provider shapes as profiles, 2026-11-11

Record: T-1659; issue 172.
Deployed: version 0.2.0-beta.4.

- Six built-in profiles put a tool in the shape of its production host:
  `lambda-al2023`, `vercel-sandbox`, `cloudflare-sandbox`, `busybox-bare`,
  `distroless-static` and `node-shell`. A profile names its image
  (`podvm run --profile busybox-bare -- sh`), its user and its variables,
  and lists what it leaves out. `needs` refuses an image that lacks a
  program. `--` ends podvm's flags.
- A static program of a volume runs on the native-static rung.

### Added: the egress broker, 2026-11-11

Record: T-1665; issue 184.
Deployed: version 0.2.0-beta.4.

- `run --network broker --ports P --allow HOST` holds the payload to an
  egress broker of the run (`pod-broker`), chained to the host's proxy:
  407 without the run's credential, 403 for a port, a host or an address
  off the lists, 200 through it. The syscall tier and the machine tier
  hold the payload to it; each other rung refuses the flag by name.

### Added: a loopback proxy reaches a guest of the machine tier, 2026-11-11

Record: T-1665; issue 184.
Deployed: version 0.2.0-beta.4.

- On the machine tier, a proxy variable of the payload that names the
  host's loopback (`127.0.0.1`, `localhost`, `::1`) names the gateway
  10.0.2.2 in the guest, with the scheme, the credentials and the port
  byte for byte. A proxy URL with no scheme or no port refuses the run by
  the name of the variable.

### Added: the scope group of the doctor, and the asset rule, 2026-11-11

Record: T-1667; issue 188.
Deployed: version 0.2.0-beta.4.

- `podvm doctor` checks each capability of the widened scope (the
  native-static rung by a run, the syscall tier, openat2, Landlock,
  user-mode networking, the emulate and incus tiers, the proxy, the space
  of the store, the assets) with ok, MISSING and a fix, or cannot be fixed
  on this host. `--strict` exits 1 on a MISSING.
- A kernel that `PODVM_MACHINE_KERNEL` names is recorded at its first use,
  and a change of it refuses. The fetch of the guest kernel checks the
  free space first.

### Added: the probe inside the payload, and compare, 2026-11-11

Record: T-1666; issue 186.
Deployed: version 0.2.0-beta.4.

- `podvm probe-inside [--podvm-tier T] [--profile P] -o FILE IMAGE` runs
  podvm's static probe as the payload and writes what it gets there:
  records `SECTION TARGET VERDICT DETAIL` for identity, escalation, escape
  surface, procfs, raw sockets, proxy, DNS and the network paths, with
  each secret-shaped value REDACTED. It needs no shell in the image.
- `podvm compare REPORT --reference REF [--expected FILE] [--strict]`
  compares two reports; `--strict` fails only on a difference in an
  invariant section that the expected-differences file does not explain.

### Fixed: a detached thread on the native-static rung, 2026-11-11

Record: T-1730.
Deployed: version 0.2.0-beta.4.

- A static payload whose thread ends detached (musl unmaps the thread's
  own stack) no longer dies with SIGSEGV on the native-static rung and
  the syscall tier.

### Added: the syscall tier, 2026-11-11

Record: T-1657; issue 169.
Deployed: version 0.2.0-beta.4.

- `run --podvm-tier=syscall` runs a static program on the native-static
  rung with the view of a syscall-interpreting tier: `uname -r` names the
  tier, `/tmp` is a directory of the run with a cap (`--tmpfs
  /tmp:size=`, 256 MiB by default), `/proc/self` names guest paths, and a
  `-v` volume writes through. It needs no ptrace, no qemu and no user
  namespace. It is a compatibility layer, not a boundary.

### Added: profiles, 2026-11-11

Record: T-1654; issue 166.
Deployed: version 0.2.0-beta.4.

- `podvm run --profile NAME [-s KEY=VALUE]` runs with a TOML recipe of run
  flags: built in (`default`, `workspace`, `readonly`, `machine`,
  `incus`) or under `$XDG_CONFIG_HOME/podvm/profiles`. `extends` merges a
  profile over its parent, and `-s` sets one key. An unknown key and a
  wrong type refuse, naming the key. `podvm profile list` and `podvm
  profile show NAME` print them.
- `--podvm-cpus N` gives a guest of the machine tier N vCPUs.

### Fixed: no terminal in a guest without -t, 2026-11-11

Record: T-1728.
Deployed: version 0.2.0-beta.4.

- On the machine tier, the command's stdout and stderr are pipes, as
  docker's are without `-t`. Before, they were the serial terminals, and
  `ls` wrote colour escapes into podvm's stdout.

### Changed: each container has its own rootfs, 2026-11-11

Record: T-1709; issue 234.
Deployed: version 0.2.0-beta.4.

- `run` and `create` copy the extracted image into the container's own
  tree, `<store>/trees/<id>/`, by `FICLONE` where the filesystem supports
  it, else by a kernel copy. A write in one container is not read by
  another container or by a later run, and `--add-host` values of two
  concurrent runs stay apart. `rm` and `--rm` remove the tree; `--rm`
  keeps the image's extracted tree. `--verbose` prints the cost of the
  copy.

### Added: the emulate tier, 2026-11-11

Record: T-1651; issue 94.
Deployed: version 0.2.0-beta.4.

- `run --podvm-tier=emulate` runs the payload under a static qemu-user of
  `PATH` on the native-static rung: each path stays in the image (a path
  absent from it is ENOENT), and an exec of the guest enters the emulator
  again. With `--platform`, an image of another architecture runs on it.
  qemu needs no patch and no `-L`.

### Changed: podcrates v0.21.0, and pod-incus for the incus tier, 2026-11-11

Record: T-1727; PodUtils/podcrates#63 and #65.
Deployed: version 0.2.0-beta.4.

- Each `pod-*` crate is at podcrates `v0.21.0`. The incus tier calls the
  REST API through `pod-incus`; podvm's own client is gone.
  Experiment 434 runs again against the rootless incusd.

### Fixed: absolute links of the image, and exec under :ro, 2026-11-11

Record: T-1725, T-1726; PodUtils/podrunner#61 and #62.
Deployed: version 0.2.0-beta.4.

- On the loader family, a path call resolves each link of the image in
  guest names: an absolute link such as `/usr/bin/python3 ->
  /usr/bin/python3.12` reads the file of the rootfs, not of the host.
- `podvm exec` into a container with a `:ro` volume or `--read-only`
  applies the read-only layer of its run; it wrote there before.

### Added: a native-static rung, 2026-11-11

Record: T-1652, T-1724; issue 109.
Deployed: version 0.2.0-beta.4.

- Where chroot is denied, a static x86_64 payload runs in a podvm process
  of its own: Syscall User Dispatch gives each of its system calls to
  podvm, which resolves each path in the image with
  `openat2(RESOLVE_IN_ROOT)` (crate `podvm-dispatch`). The golang image's
  `go version` and `go build` run on it. It was a refusal at 125 (T-1648),
  which stays where the kernel refuses the `prctl`; `--unsafe-host-paths`
  keeps the memfd family. `podvm probe --json` reports
  `measured_by.user_dispatch`.
- A static child of a dynamic program on the loader family (`go build`
  under `sh`) enters the native-static rung through podvm, where it wrote
  on the host's root before (T-1724).
- A volume (`-v HOST:GUEST`) serves a static payload on the native-static
  rung; `:ro` and `--read-only` are refused there by name (T-1724).

### Added: run on an incusd, 2026-11-11

Record: T-1718; issues 243, 251 and 253.
Deployed: version 0.2.0-beta.4.

- `run --podvm-tier=incus IMAGE CMD` runs the command in a disposable
  instance of an incusd that answers on `PODVM_INCUS_SOCKET` or on
  `INCUS_DIR`'s socket, with the image's owners from `.meta.jsonl`, and
  gives its stdout, stderr and exit code apart. Piped stdin, `-e` and `-w`
  reach the command. `create`, `exec`, `-v`, `--podvm-mem` and
  `--podvm-qemu-arg` are refused by name with the tier, and so is a run
  where no incusd answers ([incus](docs/incus.md), experiment 434).

## 0.2.0-beta.3 (2026-11-11)

### Changed: a binary embeds no interposer of another machine, 2026-11-11

Record: T-1721; PodUtils/podrunner#52.
Deployed: version 0.2.0-beta.3.

- `podvm-aarch64` embedded the two x86_64 interposer objects, so the
  userland rung declined each arm64 payload with a machine mismatch. A
  binary for another target now embeds none and says so; the aarch64 port
  of the interposer is T-1721. The smoke of each binary (`podvm-smoke`, in
  `nightly.yml` and `gate.yml`) takes `absent` off x86_64 and fails an
  embed of another machine.

### Fixed: the loader finds the image's library directories, 2026-11-11

Record: T-1719; issue 247.
Deployed: version 0.2.0-beta.3.

- On the userland rung the image's loader searches `/usr/local/lib`, each
  directory of the image's `/etc/ld.so.conf` or `ld-musl-ARCH.path`, and
  not the host's `ld.so.cache` (podcrates `v0.19.0`, `--inhibit-cache` for
  glibc 2.16 and later, also for each child).

### Added: a microVM from an OCI image, 2026-11-11

Record: T-1656 (partial), T-1682 (in progress); issue 168.
Deployed: version 0.2.0-beta.3.

- `run --podvm-tier=machine IMAGE CMD` boots a disposable guest from the
  image and exits with the command's code: under KVM where `/dev/kvm`
  opens, under TCG elsewhere (about 2.4 s for each boot under TCG on the
  cloud host). `PODVM_MACHINE_ACCEL=kvm` refuses by name without KVM.
  The guest kernel and its network modules are pinned and fetched into
  the store; the guest reaches the host's loopback at 10.0.2.2. `-v` is a
  copy into the guest, and the run says so. stderr comes back apart; piped
  stdin reaches the guest. About 0.6 s for each run under KVM on a hosted
  runner.
- `podvm machine ssh` prints the accelerator that it took.
- `.github/workflows/kvm.yml` runs the guest of experiment 390 on a hosted
  runner with `/dev/kvm`, dispatched by hand.

### Fixed: a relative program runs from `-w`, and SIGXFSZ has a reason, 2026-11-11

Record: T-1707, T-1706; issues 232 and 231.
Deployed: version 0.2.0-beta.3.

- `run -w /opt/x ./bin/prog` runs `/opt/x/bin/prog`, as docker does, on
  each rung; `argv[0]` stays `./bin/prog`. On the userland rung the
  interposer and `/proc/self/exe` take the same name.
- A foreground payload that dies on SIGXFSZ (exit 153) gets a line that
  names its soft `RLIMIT_FSIZE`; for a .NET payload, a second line names
  `DOTNET_EnableWriteXorExecute=0`.
- The official `actions/runner` v2.338.2 runs on the userland rung with
  no `--unsafe-host-paths`: `--version`, `config.sh --help`, and `run.sh`
  up to the long poll of a loopback stand-in of the service.
- An empty path reaches the kernel without a line of the interposer.

### Fixed: children that libc starts on the userland rung, 2026-11-11

Record: T-1703, T-1710; issue 235.
Deployed: version 0.2.0-beta.3.

- On the userland loader family, a child that the payload starts through
  `system()`, `popen()`, `fexecve()` or `execveat()` with flags enters
  through the image's loader, as an `execve` child does. Before, libc
  started these past the interposer, and the kernel read the loader of
  the host.
- A child ELF of another machine fails its exec with `ENOEXEC` and a
  line that names it, not a run through the host.
- A Unix socket path in `bind`, `connect`, `sendto` and `sendmsg` takes
  the path table (T-1710, issue 235): a payload's socket is inside the
  rootfs, and `-v HOSTDIR:/run/ctl` puts a host socket at a guest path.
  `getsockname` and the other answers give the guest name.

### Fixed: `wait` of a payload that ends at once, 2026-11-11

Record: T-1720; PodUtils/podrunner#49.
Deployed: version 0.2.0-beta.3.

- `wait` reads the record again, for up to 10 s, where the launcher went
  before the record had the exit code, so it no longer answers "it is Up"
  for a payload that ended.

### Added: `--read-only`, `:ro` volumes and `--tmpfs`, 2026-11-11

Record: T-1655, T-1653; issue 130.
Deployed: version 0.2.0-beta.3.

- On the userland loader family, `run --read-only` and a `-v
  HOST:GUEST:ro` volume make a write under the read-only guest path fail
  with `EROFS`, through the interposer; `/dev` and each writable volume
  stay writable. A raw system call passes the layer. Each other entry
  refuses both by name.
- `run --tmpfs PATH[:OPTIONS]` gives a foreground run an empty writable
  directory at PATH, removed at its end. It is not a mount: `size=` is
  not enforced. A detached run refuses it by name.

### Added: `--timeout`, `--ulimit` and `--security-opt no-new-privileges`, 2026-11-11

Record: T-1655 (partial); PodUtils/podcrates#58.
Deployed: version 0.2.0-beta.3.

- podvm takes the 11 crates at podcrates `v0.17.0`. `run --ulimit
  NAME=SOFT[:HARD]` sets a resource limit of the payload, and
  `--security-opt no-new-privileges` sets `PR_SET_NO_NEW_PRIVS` in it, on
  each rung, for a foreground and a detached run. Each other
  `--security-opt` is refused by name. `--cap-drop ALL` sets the same
  bit; a named capability and `--cap-add` are refused.
- `run --timeout SECONDS` (podman's spelling) kills the payload and its
  tree with SIGKILL at the bound, in a detached run and in a foreground
  run; `.End.Reason` reads `wall`.

### Fixed: the tree of a payload, 2026-11-11

Record: T-1708; issue 233; call C15.
Deployed: version 0.2.0-beta.3.

- `stop`, `kill`, `rm -f` and the end of a foreground `run` end each
  process of the payload's tree, also a grandchild that called `setsid`.
  `inspect` gives the end of the last run (`.End.Reason`, `.End.RunMs`,
  `.End.UserMs`, `.End.SystemMs`, `.End.MaxRssKiB`, `.End.Residual`).
  `rm -f` exits 125 and keeps the record where processes stayed after
  SIGKILL.
- A foreground `run` that got SIGTERM ended with 143 and did not forward
  the signal to the payload. It forwards it now.

### Changed: podcrates v0.16.0, 2026-11-11

Record: T-1708 (partial), T-1715; PodUtils/podcrates#57.
Deployed: version 0.2.0-beta.3.

- podvm takes the 11 crates at `v0.16.0`. The launcher of a detached run
  is the subreaper of the payload's tree and kills each process of the
  tree at the end of the payload, also a grandchild that left the process
  group. The container table and each log are 0600, and each container
  directory is 0700.

### Fixed: the paths that the loader gives, 2026-11-11

Record: T-1705; issue 230.
Deployed: version 0.2.0-beta.3.

- On the userland rung, a path that the loader gives (`dladdr`, the link
  map, `/proc/self/maps`) is a host path under the rootfs, and the
  interposer added the rootfs prefix to it a second time, so an open of it
  failed with `ENOENT` (CoreCLR of the official runner exited 137). A path
  under the host side of the table now passes unchanged.

### Fixed: realpath of the exe on the userland rung, 2026-11-11

Record: T-1704; issue 229.
Deployed: version 0.2.0-beta.3.

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
Deployed: version 0.2.0-beta.3.

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
Deployed: version 0.2.0-beta.3.

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
