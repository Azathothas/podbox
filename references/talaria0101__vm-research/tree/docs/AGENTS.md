# AGENTS.md — implementer's brief (read in full)

This repository is the research base for building microVM tooling (codename
`podvm`: a podman/docker-style runner for microVMs) **inside this exact class
of sandbox**. Everything an implementer needs is here: the measured
environment contract, working recipes with exact commands, failure
signatures, and the evidence logs. `docs/paper.md` is the distilled write-up;
`docs/seccomp.md` is the security-envelope deep-dive.

## 0. The environment contract (all measured, see experiments/logs/10-,11-,60-,66-,75-)

| Fact | Consequence |
|---|---|
| uid 0 in userns `0→1000`; full caps **scoped to it**; seccomp number-denylist | legacy privileged syscalls EPERM: `mknod mount umount2 unshare ptrace bpf keyctl open_by_handle_at setns userfaultfd perf_event_open kexec_load iopl` |
| `clone/clone3` with `NEWUSER/NEWNET/NEWPID/NEWNS` **work** | nested namespaces reachable; creator owns them → in-process namespace privileges (70-) |
| new mount API `fsopen/fsconfig/fsmount` works; `move_mount` EPERM | mounts preparable but not attachable |
| `/dev/kvm`, `/dev/net/tun` absent (host drivers present); uncreatable | no KVM; no tap. **TCG only** (`-accel tcg,thread=multi`) |
| `/proc` mounted from init userns | `uid_map`/`gid_map` unwritable by anyone; nested-ns processes run uid-unmapped; **exec drops nested caps** → namespace privileges are in-process-only |
| `/etc/passwd` absent; `/` read-only | use `patches/libfakepasswd.c` shim or chroot appliance (70-) |
| TCP bind EPERM (all addresses); TCP connect to `127.0.0.0/8` EPERM; external connect OK (was :443-only early-session, :80 relaxed live — see 75-) | no TCP listeners ever; unix sockets + UDP freely bindable |
| UDP bind + egress OK | UDP hostfwd is the guest transport |
| `RLIMIT_FSIZE` 500 MiB/file | guests/images must stay under; `SIGXFSZ` = this limit |
| `/tmp` 64 MiB tmpfs | stage on `/workspace`; set `TMPDIR` for go/gcc |
| outbound internet | works (TCP/UDP/DNS); `http://` and `https://` both fine since relaxation |

## 1. What demonstrably works (evidence: experiments/logs/)

- **Boot**: pinned Alpine 6.12.94-virt kernel, 1.6 s to userspace on
  `-M microvm`, 3.1 s on `-M pc` (20-, 21-).
- **Fleet**: 8 concurrent microvm guests, 4.06 boots/s (62-).
- **In-guest builds**: `apk.static`-provisioned rootfs with gcc 14.2;
  in-guest compile + run of a benchmark, 35.1 Mops/s (71-, 72-).
- **Serial exec protocol**: FIFO-pair transport + marker-bracketed
  single-shot execs with exit-code propagation (73-).
- **Snapshot-fork**: `migrate "exec:cat > snap"` (76 MB) → `-incoming
  "exec:cat snap"`, 3/3 clones, restore ≤ 2.1 s by guest-uptime delta (64-,
  74-).
- **Guest↔guest UDP**: slirp UDP hostfwd both sides (63-).
- **Firecracker's own artifacts** (CI vmlinux-6.1.102 + initramfs) boot
  interactively on `-M pc` after a `/dev/console` cpio append (65-).
- **Private netns fabric**: in-process netlink veth + addresses + ICMP
  across it, via the clone3 gap (70-).
- **Private PID trees**, **chroot appliance** with own `/etc/passwd` (70-).

## 2. What is blocked, and the exact failure signature

| Subject | Signature | Log |
|---|---|---|
| firecracker 1.16.1 | `Error creating KVM object: No such file or directory` | 30- |
| cratera 1.2.0 `doctor` | `✗ /dev/kvm not found` | 40- |
| forkd 0.5.3 preflight | `✗ kvm /dev/kvm does not exist` | 35- |
| smolvm 1.14.4 | machine start → `SIGXFSZ` (FSIZE) / silent no-op; libkrun | 34- |
| virtkit 0.66.0 | segfault (139); binary carries libkrunfw | 41- |
| pullrun 0.7.9 container mode | runc: `private UTS namespace` EPERM | 43- |
| UML 7.1um1 | `ptrace: Operation not permitted` self-check | 50- |
| lima 2.2.0 | reaches qemu/TCG, dies at ssh-hostfwd TCP bind | 33- |
| kata/flintlock/microsandbox/boxlite/muvm/clawk | libkrun/firecracker → `/dev/kvm` (source-cited) | 31/32/36/37/38/39- |

Full matrix: README.md. Rule: **never claim what a committed log does not
back**; tag claims [V] (verified in-tree) or [S] (pinned source read).

## 3. Recipes (known-good; each exercised by a committed experiment)

### Boot a guest to an interactive driver (pc machine, userspace serial works)
```sh
# initramfs = alpine rootfs tree (cpio+gzip) with /init shim + APPENDED
# cpio archive containing /dev/console (5:1) — firecracker-style initramfs
# ships /dev empty and PID1 then has NO console (65-).
qemu-system-x86_64 -accel tcg,thread=multi \
  -M pc,acpi=off -m 1024M -smp 2 \
  -kernel vmlinuz-virt -initrd initramfs.cpio \
  -append "console=ttyS0 rdinit=/init" \
  -display none -no-reboot -monitor none -serial pipe:$D/serial
# hold both fifo ends O_RDWR before qemu starts; drive via marker-bracketed
# single-shot execs (73-) — retry-feed only until the shell is proven alive.
```

### Machine selection
- `-M pc,acpi=off`: **interactive userspace serial works**. Poweroff is
  unavailable (halts) → driver must kill qemu after the completion marker.
- `-M microvm`: kernel `printk` reaches the UART but **userspace ttyS0
  writes never surface**; boots need `acpi=off` (ACPI-table hang). Use for
  printk-only workloads (62-, 64- pattern).

### Networking
- Guest egress: `-netdev user` (slirp) + **virtio-net-pci on pc /
  virtio-net-device (mmio) on microvm**; alpine `virt` ships `virtio_net` as
  a module — ship the module tree and `modprobe` (busybox `insmod` silently
  fails on deps, 63-).
- Guest↔guest: **UDP hostfwd** (63-). TCP hostfwd is impossible (bind EPERM).
- Private per-tenant fabrics: `nspriv`-style in-process netlink (70-).

### Guest rootfs
- Provision on the host with `apk.static --root` (71-); never `apk` inside a
  fresh guest unless network-in-guest is required.
- Exec-prefix gotcha: alpine gcc resolves `cc1`/`crtbegin` from `argv[0]`;
  always invoke `/usr/bin/gcc` by absolute path (71-).
- Compile with `-fno-use-linker-plugin` unless `bfd-plugins` are provisioned.

### Transfers into the guest
- Seed files by cpio (initramfs) — 73- seeds `/root/bench.c`.
- In-guest network fetch works (slirp) — DNS 10.0.2.3, addr 10.0.2.15/24.

## 4. Hard rules for claims

1. Every experiment = numbered script + committed log; conditions printed;
   exit 0/1/2 = ran-and-held / ran-and-blocked / could-not-run.
2. Inputs pinned by URL + SHA-256 (`experiments/lib.sh`).
3. Negative results are committed.
4. Never trust a subject's exit code alone (cratera's `doctor` exits 0 on
   failure; smolvm's `start` returns 0 without starting) — grep the log for
   the decisive line.
5. Numbers carry conditions; a replaced experiment keeps its number and the
   new one gets the next.
6. Review passes (≥3 per push wave) re-derive claims from logs, not memory.

## 5. Where to look

- Verdicts: `README.md` matrix · Evidence: `experiments/logs/`
- Security envelope: `docs/seccomp.md` · Paper: `docs/paper.md`
- Patches + shims: `patches/` · Reviews: `reviews/REVIEW-{1..5}.md`
