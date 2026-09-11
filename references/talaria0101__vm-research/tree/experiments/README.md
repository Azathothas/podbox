# Experiments

Numbered in the order they were run. Each is a script; each log in `logs/` is
committed evidence produced by the script of the same number. Exit codes:
0 = ran and held, 1 = ran and the subject is blocked/failed, 2 = could not run.
Shared pins and helpers: `lib.sh`. Downloads land in `work/` (gitignored);
only logs and pins are committed.

| # | Subject | Question | Verdict | Exit |
|---|---------|----------|---------|------|
| 10 | environment census | what is denied, and by which mechanism | seccomp EPERM: mknod/mount/umount2/unshare/ptrace/bpf/keyctl/open_by_handle_at; EACCES /proc/self/mem; ENOENT /dev/kvm + /dev/net/tun (host drivers present per /proc/misc); RWX mmap OK | 0 |
| 11 | network census | what the network plane allows | TCP bind (loopback+wildcard) EPERM, loopback TCP connect EPERM, external TCP connect OK, UDP bind loopback OK | 0 |
| 20 | qemu `-M pc`, TCG | does software emulation boot a real kernel | WORKS: pinned Alpine 6.12.94-virt → userspace → clean poweroff, 3.14 s wall | 0 |
| 21 | qemu `-M microvm`, TCG | the dedicated microvm machine type | WORKS: same kernel, 3.25 s wall, guest uptime 1.47 s | 0 |
| 30 | firecracker v1.16.1 | where does a microVM launch die | binary + API serve; `InstanceStart` → `Error creating KVM object: No such file or directory (os error 2)` | 1 |
| 31 | kata-containers 4.1.0 | KVM chain without a 970 MB download | [S] kvm-module deps in `hypervisor_linux_amd64.go`; [V] census. BLOCKED | 1 |
| 32 | flintlock v0.15.1 | backend dependency chain | [S] backend = firecracker; [V] first-hand 30- failure. BLOCKED | 1 |
| 33 | lima v2.2.0 | qemu-backed manager end-to-end | reached qemu under TCG auto-fallback after two env completions (no /etc/passwd → `patches/libfakepasswd.c`; uid-0 guard → `patches/lima-v2.2.0-root-guard.diff`); killed by no-bind rule (ssh hostfwd is its only Linux transport) | 1 |
| 34 | smolvm v1.14.4 | libkrun create + start | create works (OCI pull + config); start dies SIGXFSZ (rc 153: RLIMIT_FSIZE=500 MB) even at `--mem 256`; state stays `created`; exec: `not running` | 1 |
| 35 | forkd v0.5.3 | preflight verdict | `✗ kvm /dev/kvm does not exist → enable KVM` | 1 |
| 36 | clawk v0.4.0 | Linux provider backend | [S] `--provider firecracker` (Linux default); [V] census + 30-. BLOCKED | 1 |
| 37 | microsandbox v0.6.17 | libkrun chain | [S] README "Linux: KVM enabled" + libkrun `Kvm::new()` (vstate.rs:422). BLOCKED | 1 |
| 38 | boxlite v0.10.0 | KVM chain, binary untestable (cp310/311 wheels vs py3.14) | [S] README "KVM enabled (`/dev/kvm` accessible)". BLOCKED | 1 |
| 39 | muvm muvm-0.6.0 | libkrun chain | [S] links libkrun; `Kvm::new()` vstate.rs:422. BLOCKED | 1 |
| 40 | cratera v1.2.0 | its own `doctor` | `✗ /dev/kvm not found` (+ `/var/tmp` EACCES); doctor exits 0 even on failure (noted) | 1 |
| 41 | virtkit v0.66.0 | "rootless microVM toolkit" | every verb segfaults (rc 139); strings carry `libkrunfw.so.5`, `Error creating the Kvm object` → libkrun-based | 1 |
| 42 | smolBSD @ `00bc6e0` | qemu-backed NetBSD microVMs, native TCG fallback | WORKS: upstream prebuilt `build-amd64.img` + sha-checked `netbsd-SMOL` kernel; boots to shell in ~5 min TCG; guest `uname -a` ran (`VMR-SMOLBSD-OK`) | 0 |
| 43 | pullrun v0.7.9 | dual-mode: runc container / firecracker VM | container mode → runc → `unable to set hostname without a private UTS namespace` (namespace ban); vm mode → kernel image pull 401, then the 30- firecracker chain. BLOCKED | 1 |
| 44 | preloop v0.32.5 | local CI "hardware isolation" | engine exits before ready; its own strings: `the VM pool cannot open /dev/kvm` | 1 |
| 45 | nanos via ops 0.1.46 | unikernel: one app, one kernel | WORKS: ops builds image from a static ELF, boots TCG (qemu auto-fallback), guest prints `VMR-NANOS-OK pid=2`; needs explicit `-m` (ops default memory is empty) | 0 |
| 50 | user-mode Linux 7.1um1 | the no-/dev/kvm microVM, tested as extra | dies at its own startup self-check: `ptrace: Operation not permitted` (UML intercepts syscalls via ptrace) | 1 |
| 51 | qemu `-M isapc`, TCG | most stripped vanilla machine type | 6.12.94-virt kernel panics on ISA-only machine (i486 default CPU, then idle-task panic with `-cpu qemu64`) — machine/kernel incompatibility, sandbox-independent | 1 |

| 60 | seccomp/syscall census | is there a filter gap (clone3 got EINVAL while unshare got EPERM) | new mount API **unfiltered** (fsopen/fsmount/fsconfig execute); `clone3`/`clone` with NEWUSER\|NEWNS **work** — nested userns reachable (sandbox gap); KVM still unreachable at every path: mknod EPERM even on own-tmpfs (init-ns CAP_MKNOD), devtmpfs create EPERM (FS_USERNS_MOUNT), sysfs EPERM, open_by_handle_at EPERM, iopl(3)/perf/kexec EPERM; /dev/kvm ENOENT inside nested userns | 0 |
| 62 | microvm fleet (N=8) | fleet capability | **8/8 guests booted with proof, 1.97 s wall, 4.06 boots/s aggregate** (1 vcpu/128M each, TCG) | 0 |
| 63 | guest↔guest UDP net | can VMs talk without TCP bind | **WORKS** — A slirp → host UDP 9002 (hostfwd udp bind) → B's slirp → B nc:7000; payload `PING-from-A-seq1` seen in B (virtio_net.ko + module tree shipped; TCP not needed) | 0 |
| 64 | snapshot-fork | forkd's primitive without KVM | **3/3 clones resumed and ran** — parent snapshotted via `migrate exec:cat` (76 MB), children via `-incoming exec:cat`, each to its own poweroff | 0 |
| 65 | firecracker's own artifacts | does the firecracker guest spec run here | **WORKS** — official CI vmlinux-6.1.102 + initramfs on qemu; fixes engineered: /dev/console node appended as cpio (their initramfs ships /dev empty), `-M microvm` prints via kernel only (userspace ttyS0 silent), `-M pc` fully interactive; guest shell: `uname`, `id`, `VMR-FC-ARTIFACTS-OK` | 0 |
| 66 | TCG sizing numbers | what TCG costs/delivers | cold boot 1.58–1.64 s (n=3, to userspace+bench+poweroff); in-guest md5 16 MiB = 0.07–0.08 s guest-time, digest identical to host; host same md5 0.026 s (~3× TCG factor on this workload) | 0 |

| 70 | seccomp-gap value | practical use of the clone3 gap | nested-netns **veth pair created + addresses + ICMP echo-reply** in-process via raw netlink (parent: RTNETLINK EPERM); NEWPID private tree; chroot appliance with own /etc/passwd. Boundary: uid_map unwritable → exec drops caps → value is in-process-only | 0 |
| 71 | in-guest toolchain | compile INSIDE a microVM | apk adds gcc 14.2 in-guest (over slirp https); in-guest compile + run of bench.c (35.1 Mops/s). Key fix: absolute-path gcc invocation (driver prefix from argv[0]) | 0 |
| 72 | bench matrix | fastest/most performant platform | host 720–755 Mops/s; microvm-alpine 35.1; pc-alpine 31.4/35.1; nanos 33.1; fc-kernel 34.3/35.3; identical checksum everywhere. Boot: microvm 1.6s / pc 3.1s / nanos ~2s / netbsd ~300s | 0 |

| 76 | chroot route | the cheap competitor | native 845.9/866.8 Mops/s in-chroot build+run; no /proc (ps empty), no /dev devices, no kernel boundary, double-chroot escape preconditions OK. tar needs --no-same-owner (gid-42 chown aborts extraction) | 0 |
| 77 | container-research crossval | their N/F/M model vs this sandbox | write allowlist {/tmp,/dev/shm,/workspace,/state} identical; pivot_root+process_vm_readv EPERM identical; fsopen/open_tree patched live between 09-09 and 09-10; clone3 ns gap still open | 0 |

Considered, not run (no pinned artefact or decided by existing evidence):
`qemu-system-ppc64` (binary present, no guest kernel pinned), gVisor/runsc
(ptrace platform needs the banned ptrace; KVM platform needs /dev/kvm — both
walls already first-hand in 10-), libkrun standalone (the common backend of
34/37/39/41; its KVM open is cited first-hand from source).

## Conditions of this run

One sandbox, one day, one machine: Gentoo bare-metal host (kernel
6.18.39-gentoo-dist-bin), AMD Ryzen 7 7700 (16 threads), 30 GiB RAM, qemu
10.2.3, tcg only. Numbers are from that machine on that day. Logs are
conditions-first for this reason.
