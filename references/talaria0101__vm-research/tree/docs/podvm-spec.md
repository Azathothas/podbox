# `podvm` design specification — a podman-style microVM runner for
# capability-denied sandboxes

Status: specification backed by measured evidence. Every numbered claim [E-n]
cites an experiment in `experiments/` with a committed log. An implementer
should be able to build the tool from this document plus the cited logs
without repeating any discovery work.

## 0. Target environment contract (all measured)

- [E-10] uid 0 in userns `0→1000`, full caps scoped to it, seccomp
  number-denylist (`mknod mount umount2 unshare ptrace bpf keyctl
  open_by_handle_at setns userfaultfd perf_event_open kexec_load iopl` →
  EPERM). `clone/clone3` with NEWUSER/NEWNET/NEWPID/NEWNS **work**; new mount
  API (`fsopen/fsconfig/fsmount`) **works**; `move_mount` EPERM.
- [E-10] `/dev/kvm`, `/dev/net/tun` nodes absent; uncreatable. Host drivers
  present (`/proc/misc`).
- [E-75] TCP bind EPERM everywhere; TCP connect to 127.0.0.0/8 EPERM; RFC1918
  unreachable; external TCP allowed (port :80 was denied early-session, then
  relaxed live — handle both); UDP bind/egress OK; AF_UNIX OK.
- [E-66/34] `RLIMIT_FSIZE` 500 MiB per file.
- [E-33/71] No `/etc/passwd`; `/` read-only; `LD_PRELOAD` shim
  (`patches/libfakepasswd.c`) fixes user lookups for dynamic binaries.
- [E-71] Outbound network from guests via slirp works (https required
  early-session; :80 permitted after relaxation).
- Details and mechanism analysis: `docs/seccomp.md`.

## 1. Image model

- An **image is a rootfs directory** (not a disk image): Alpine minirootfs
  3.22.5 provisioned on the host by `apk.static --root` with `gcc musl-dev
  make` [E-71/72: in-guest gcc 14.2.0 compile verified].
- Ship the alpine `virt` **module tree** (`virtio_net.ko`, `net_failover.ko`,
  …) inside the image: `virtio_net` is a module and busybox `insmod`
  silently fails on unresolved deps — use `modprobe` [E-63].
- Image caching by name; provisioning is idempotent (`apk.static --root` is
  safe to re-run).

## 2. Initramfs assembly

- Root tree cpio (newc) gzipped, **plus an appended raw-cpio archive
  containing `/dev/console (5:1)`** and (optionally) an `init` override —
  appended archives are unpacked after the base [E-65: firecracker
  initramfs ships /dev empty; PID1 without a console is silent and useless].
- `/init` shim: mount proc/sysfs/devtmpfs, print a ready marker
  (`VMR-GUEST-READY`), then `exec /bin/sh` (long-running) or run the
  workload and power off (throwaway).
- Beware: `-M microvm` delivers kernel printk to the 16550 but drops
  *userspace* UART writes; `-M pc,acpi=off` is fully interactive [E-65].
- `poweroff` on `-M pc,acpi=off` halts instead of exiting qemu — the driver
  must treat "System halted" (+ completion marker) as completion and kill
  qemu [E-42/65].

## 3. Machine profile

- `-M pc,acpi=off` (interactive) / `-M microvm` (printk-only) [E-21/65].
- `-accel tcg,thread=multi` (KVM unreachable [E-10/60]; TCG boots 1.6–3.2 s
  [E-20/21/66]).
- `-m ≤ 400M` unless file-backed memory: `RLIMIT_FSIZE` 500 MiB kills
  larger guests with `SIGXFSZ` [E-34, exit 153].
- Netdev: `virtio-net-pci` (pc) / `virtio-net-device` (microvm) + slirp.
- Serial: FIFO pair via `-serial pipe:$D/serial`; the parent process holds
  both ends `O_RDWR` before qemu starts (qemu's pipe backend opens
  *existing* fifos and blocks otherwise) [E-73].

## 4. Exec protocol (guest command execution)

- Readiness: retry-feed `\n` + nonce `echo VMR_<tag>` until it echoes
  (bootstrap feeding is lossy before the console is up) [E-73].
- Exec: single-shot `echo TAG_B; { cmd ; } ; echo TAG_E $?` with
  line-anchored marker matching; exit code propagated [E-73: rc=0 on uname/
  id/gcc; rc=1 on a genuinely failing compile].
- Absolute-path invocation for guest compilers: alpine gcc resolves
  `cc1`/`crtbegin`/`-lgcc` from `argv[0]` — bare `gcc` from a PID1 ash
  fails ENOENT [E-71].
- Compile flags: `-fno-use-linker-plugin` unless `bfd-plugins` provisioned.
- Scale: same protocol drove an in-guest `apk`-installed gcc to build and
  run the benchmark (35.1 Mops/s in-guest on the prepared-rootfs variant,
  29.9/19.0 on other runs — same checksum) [E-71/72].

## 5. Fleet, fork, network

- Fleet: throwaway `-M microvm` members, 128 MiB, 1 vCPU; 8/8 proof at
  4.06 boots/s [E-62]; size fleets ≤ 400 MiB memory images [E-34].
- Fork: quiesce at a known guest tick, `migrate "exec:cat > snap"` (76 MB
  for 128 MiB), resume with `-incoming "exec:cat snap"`; restore ≤ 2.1 s
  (uptime-delta method) [E-74].
- Guest↔guest data: **UDP hostfwd** (both slirps) [E-63]. Control plane:
  unix sockets and FIFOs (both unrestricted). Never plan a TCP listener.
- Egress: slirp DNS 10.0.2.3, guest 10.0.2.15/24; TLS-capable fetchers
  required while the :80 rule was active.

## 6. Privileged extensions via the clone3 gap

- `nspriv`-style in-process operations inside `CLONE_NEWUSER|NEWNET`:
  netlink `RTM_NEWLINK` veth creation, `SIOCSIFADDR`/`IFF_UP`, raw-ICMP
  ping across the pair — all succeed where the parent namespace gets
  `RTNETLINK EPERM` [E-70: REPLY RECEIVED].
- Constraints: exec drops the granted caps (uid unmapped — `uid_map`
  unwritable, procfs provenance), so privileged namespace work must happen
  in-process; mount attachment (`move_mount`) is EPERM even in the child;
  `uid_map`-mapped "non-root" identity is impossible [E-70/60].
- Private PID trees via `CLONE_NEWPID` (child reports PID 1) [E-70].

## 7. Non-goals / blocked designs (do not attempt)

- TCP listeners of any kind [E-75].
- KVM-accelerated anything [E-30/60/65 chain].
- runc-style OCI containers [E-43: UTS namespace EPERM].
- User-mode Linux guests [E-50: ptrace EPERM].
- `uid_map`-based identity switching [E-70: procfs provenance].
- Guests with memory images or disks > 500 MiB per file [E-34].

## 8. Reference implementation notes

Working PoCs for items 2-6 live in `experiments/73-` (exec protocol),
`experiments/74-` (fork latency), `experiments/62-` (fleet),
`experiments/70-` (nspriv network fabric), `experiments/71-` (in-guest
toolchain), `experiments/63-` (UDP networking). Each prints the exact
commands it runs; each log is committed.
