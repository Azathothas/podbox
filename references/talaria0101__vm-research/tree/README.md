# vm-research

Which microVM tooling actually runs inside this sandbox, and what runs there
instead when the kernel facilities they demand are denied. Every verdict below
is produced by a numbered script in [`experiments/`](experiments/) and every
log is committed. **Methodology is binding, not advisory**: experiments are
files, inputs are pinned, negative results are committed, numbers carry their
conditions (rules summarised in `docs/AGENTS.md`; per-experiment entries in
`experiments/README.md`).

## The environment, in one paragraph (evidence: `experiments/10-`)

The sandbox is a container on bare-metal Gentoo (kernel 6.18.39, AMD Ryzen 7
7700, 16 threads, 30GiB RAM). We are uid 0 inside a **user namespace** mapped
`0 → 1000`, with a **full capability set scoped to it**, under a **seccomp
filter** that returns `EPERM` for `mknod`, `mount`, `umount2`, `unshare`
(new NS or USER), `ptrace`, `bpf`, `keyctl`, `open_by_handle_at`, and `EACCES`
for `/proc/self/mem`. The host **does have** `kvm` (misc 232), `tun` (misc 200)
and `kvm_amd` loaded — `/proc/misc` proves the drivers exist — but the device
nodes are **not bind-mounted** into our mount namespace, and the filter makes
creating or mounting them impossible. The host CPU has `svm` (AMD-V). Result:
**KVM is unreachable**, and `/dev/net/tun` with it. Three more constraints
surfaced during the work (11-, 33-, 34-): **no TCP bind at all** (loopback or
wildcard — kills every hostfwd-based manager), **no loopback TCP connect**
(outbound TCP and UDP binds are fine), and **RLIMIT_FSIZE = 500 MB** per file
(kills guests sized above it). There is also no `/etc/passwd` and the root fs
is not writable from inside the user namespace; subjects that resolve the
invoking user at startup need the shim in `patches/libfakepasswd.c`. What
remains available: plain `mmap`/`mprotect` **RWX** (software emulation works),
sockets for outbound traffic, and the host network namespace.

## Verdict matrix

Mechanism key: **KVM** = needs `/dev/kvm` (denied here) · **TCG** = software
emulation, runs · **UML** = user-mode Linux port · **proc** = userspace
process-isolation, no machine emulation. Evidence tags per the
`container-research` convention: **[V]** verified in this tree, **[S]**
established from source read, **[R]** reported elsewhere and not reproducible
here.

| # | Subject | Mechanism | Verdict in this sandbox | Evidence |
|---|---------|-----------|------------------------|----------|
| 10 | environment census | — | seccomp EPERM map, KVM/TUN nodes absent (host drivers present), RWX mmap allowed | [V] `logs/10-` |
| 11 | network census | — | no TCP bind (loopback or wildcard), no loopback TCP connect; outbound TCP + UDP bind OK | [V] `logs/11-` |
| 20 | vanilla QEMU TCG (`-M pc`) | TCG | **WORKS** — Alpine 6.12.94 to userspace + clean poweroff, 3.14 s wall | [V] `logs/20-` |
| 21 | vanilla QEMU TCG (`-M microvm`) | TCG | **WORKS** — same kernel on the microvm machine, 3.25 s wall | [V] `logs/21-` |
| 30 | firecracker v1.16.1 | KVM | BLOCKED — runs, API serves, dies at `Error creating KVM object: ENOENT` | [V] `logs/30-` |
| 31 | kata-containers 4.1.0 | KVM | BLOCKED — [S] kvm-module deps in amd64 hypervisor impl + [V] census; 970 MB bundle declined as decided | [S]+[V] `logs/31-` |
| 32 | flintlock v0.15.1 | KVM (firecracker backend) | BLOCKED — chain via first-hand 30- | [S]+[V] `logs/32-` |
| 33 | lima v2.2.0 | TCG (qemu backend) | BLOCKED at the last step — reaches qemu under TCG auto-fallback after two env completions; its ssh-hostfwd transport needs a TCP bind, and no bind is allowed | [V] `logs/33-` |
| 34 | smolvm v1.14.4 | KVM (libkrun) | BLOCKED — create works (OCI pull); start dies SIGXFSZ (RLIMIT_FSIZE=500 MB) even at `--mem 256`, never runs | [V] `logs/34-` |
| 35 | forkd v0.5.3 | KVM (firecracker fork) | BLOCKED — own preflight: `✗ kvm /dev/kvm does not exist` | [V] `logs/35-` |
| 36 | clawk v0.4.0 | KVM (firecracker on Linux) | BLOCKED — [S] provider chain + [V] census | [S]+[V] `logs/36-` |
| 37 | microsandbox v0.6.17 | KVM (libkrun) | BLOCKED — [S] README + libkrun `Kvm::new()` | [S]+[V] `logs/37-` |
| 38 | boxlite v0.10.0 | KVM | BLOCKED — [S] README; binary untestable (cp310/311 wheels, host has py3.14) | [S]+[V] `logs/38-` |
| 39 | muvm 0.6.0 | KVM (libkrun) | BLOCKED — [S] links libkrun | [S]+[V] `logs/39-` |
| 40 | cratera v1.2.0 | KVM (firecracker) | BLOCKED — its own doctor: `✗ /dev/kvm not found` | [V] `logs/40-` |
| 41 | virtkit v0.66.0 | KVM (libkrun) | BLOCKED — segfaults on every verb; binary carries libkrunfw + `Error creating the Kvm object` | [V] `logs/41-` |
| 42 | smolBSD `00bc6e0` | TCG (qemu backend) | **WORKS** — NetBSD 11.0_STABLE microVM boots to shell in ~5 min, guest command executed (`VMR-SMOLBSD-OK`) | [V] `logs/42-` |
| 43 | pullrun v0.7.9 | KVM + runc | BLOCKED — container mode dies inside runc (UTS namespace EPERM); vm mode is firecracker | [V] `logs/43-` |
| 44 | preloop v0.32.5 | KVM | BLOCKED — local engine exits before ready; own strings: `VM pool cannot open /dev/kvm` | [V] `logs/44-` |
| 45 | nanos via ops 0.1.46 | TCG (qemu backend) | **WORKS** — unikernel from a static ELF, boots under TCG, guest prints `VMR-NANOS-OK pid=2` | [V] `logs/45-` |
| 50 | extra: user-mode Linux 7.1um1 | UML (ptrace) | BLOCKED — needs ptrace for syscall interception; dies at its own self-check | [V] `logs/50-` |
| 51 | extra: QEMU `-M isapc` | TCG | FAIL — 6.12 kernel cannot boot the ISA-only machine (panic); sandbox-independent | [V] `logs/51-` |
| 60 | seccomp gap hunt | — | **filter gaps found** (new mount API + clone/clone3 namespaces unfiltered) but KVM still unreachable at every measured path | [V] `logs/60-` |
| 62 | microvm fleet ×8 | TCG | **WORKS** — 8/8 concurrent boots, 4.06 boots/s | [V] `logs/62-` |
| 63 | guest↔guest UDP | TCG | **WORKS** — VM-to-VM payloads via UDP hostfwd (no TCP bind) | [V] `logs/63-` |
| 64 | snapshot-fork | TCG | **WORKS** — migrate-to-file, 3/3 clones resumed and ran | [V] `logs/64-` |
| 65 | firecracker's own artifacts | TCG | **WORKS** — official CI kernel+initramfs, interactive guest shell on `-M pc`, `VMR-FC-ARTIFACTS-OK` | [V] `logs/65-` |
| 66 | TCG sizing | TCG | boot 1.6 s; in-guest md5 digest identical to host, ~3× host speed | [V] `logs/66-` |
| 70 | seccomp-gap value | — | **demonstrated**: nested-netns veth pair + addresses + ICMP echo-reply built in-process via raw netlink (parent cannot: RTNETLINK EPERM); NEWPID private tree; chroot appliance with own /etc/passwd. Boundaries: mount-attach EPERM, uid_map unwritable (procfs provenance) → exec drops caps, so value is in-process-only | [V] `logs/70-` |
| 71 | in-guest toolchain | TCG | **WORKS** — apk installs gcc 14.2 in-guest over slirp https; bench compiled and run in-guest, 35.1 Mops/s; root-caused: alpine gcc driver prefix-from-argv[0] (fix: absolute-path invocation) | [V] `logs/71-` |
| 72 | bench matrix | TCG | host 720–755 Mops/s; microvm-alpine 35.1; pc-alpine 31.4–35.1; nanos 33.1; firecracker-kernel 34.3–35.3; checksum identical on all platforms | [V] `logs/72-` |
| 76 | chroot route | native | **WORKS** — in-chroot apk + gcc install, native 845.9/866.8 Mops/s; no /proc, no /dev, no kernel boundary; double-chroot escape preconditions hold | [V] `logs/76-` |
| 77 | container-research crossval | — | N/F/M model claims cross-validated: write allowlist + seccomp list + procfs denials identical; **fsopen/open_tree patched live** (was open on 09-09), **clone3 ns gap still open** | [V]/[C] `logs/77-` |
| — | three-route comparison | — | `docs/comparison.md`: chroot vs podbox-style vs podvm — evidence-backed pros/cons/limits | — |

Bottom line. **End-to-end runnables: four** — vanilla qemu on the `pc` and
`microvm` machines, smolBSD (NetBSD), nanos (unikernel) — plus
**firecracker's own kernel+initramfs running under qemu** (65-), an **8-guest
fleet at 4.06 boots/s** (62-), **live VM→VM networking over UDP** (63-), and
**snapshot-fork cloning** (64-). Everything on TCG; sizes and digests in 66-.
Lima reaches qemu under TCG but its ssh-hostfwd transport needs a TCP bind,
which no subject may take.

The twelve KVM-dependent subjects are blocked by measured facts, not
assumptions: `/dev/kvm` exists on the host but its node is not exposed and
cannot be created — the 60- census closes every theoretical path (nested
userns via the clone3 gap, new mount API, handle-based open, iopl/perf/kexec
are all individually measured EPERM/ENOENT inside and outside a nested
userns). The same census proves two **sandbox filter gaps** (new mount API
and clone-family namespace creation execute), which is a finding about the
sandbox, not a path to KVM: the nested-userns child still cannot reach
`/dev/kvm` (ENOENT), cannot mknod it (init-ns CAP_MKNOD), and cannot mount
devtmpfs (FS_USERNS_MOUNT). The runc and UML escape hatches die on UTS
namespace EPERM and ptrace EPERM respectively (43-, 50-); the no-bind rule
kills hostfwd managers (33-).

**Which microVM to use here:** `qemu -M microvm` + Alpine — fastest boot
(1.6 s), fastest executing of the guests (35.1 Mops/s, checksum-equal to the
host run), and the only self-sufficient platform (installs its own toolchain
over the network, builds in-guest). nanos for single-binary workloads. The
full argument is in `docs/paper.md` (arXiv-style), §5.5.

## Layout

```
experiments/   numbered, runnable scripts + committed logs/ (the evidence)
docs/AGENTS.md self-contained router for any future session
docs/paper.md  arXiv-style paper distilling the findings
docs/seccomp.md  security-envelope deep-dive (filter + network policy)
docs/comparison.md  chroot vs podbox-style vs podvm, evidence-backed
docs/podvm-spec.md  podvm runner design spec, evidence-cited
reviews/       REVIEW-1..N.md — peer-review passes, each a fresh read
patches/       simple upstream patches, only where a one-file change unblocks
```

## Reproduce

```sh
./experiments/10-probe-environment.sh   # the census everything interprets
./experiments/20-qemu-tcg-pc-boot.sh    # the control: TCG boots, 3.1 s
./experiments/<nn>-<subject>.sh         # one per subject above
```

Every script pins its inputs (`experiments/lib.sh`), prints its conditions,
and exits 0 ran+held / 1 ran+failed / 2 could-not-run.
