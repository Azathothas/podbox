# Three routes to an isolated build/run environment in this sandbox:
# vanilla chroot, podbox-style namespaces, and microVMs (`podvm`)

Companion to `docs/paper.md` and `docs/podvm-spec.md`. Every claim is
evidence-tagged: **[V]** measured in this repository, **[S]** established
from pinned source, **[C]** cross-validated against
Azathothas/container-research (their N/F/M mechanism model, committed logs).

## 1. The three routes

| | **chroot** | **podbox-style** (container-research) | **podvm** (this work) |
|---|---|---|---|
| Isolation primitive | filesystem view switch (`chroot(2)`) | userns (partial ID map) + per-run seccomp allow/deny + Landlock path rules | hardware VM (qemu TCG here; KVM unavailable) per workload |
| Kernel boundary | **none** — one shared kernel, one shared set of namespaces | none — same, plus narrowed *filesystem write* and *syscall* surfaces | **full emulator-process boundary** — a separate kernel running as tenant userspace code (hardware enforcement is what this sandbox denies; under KVM it would be hardware) |
| Privileges needed | `chroot(2)` only (CAP_SYS_CHROOT in own userns — held) | userns creation (clone3 gap [E-60/77]) + their runtime | none beyond what the sandbox gives |
| Speed of execution | **native** [E-76: 846–867 Mops/s] | native (same class — no emulation) [C] | **TCG: ~20–25× under native** [E-72/76: guest 31–35 vs host 721–755 and chroot-native 846–867 Mops/s] |
| Start latency | ~0 (a path lookup) | ~0 + runtime setup | 1.6 s (microvm/Alpine) … 300 s (NetBSD) [E-66/42/45] |
| Build toolchains | full (apk installs; 172 MiB gcc proven) [E-76] | full [C: udocker runs apk-in-image] | full on Alpine guests [E-71]; host-built ELFs for unikernels [E-45] |
| Guest↔guest / fleet semantics | none (it is one process tree; "fleet" = N chroots) | N containers, namespaces per rung | real: 8 concurrent VMs @ 4.06 boots/s [E-62]; snapshot-fork [E-64]; VM↔VM UDP [E-63] |
| Checkpoint/clone of live state | no | no | **yes** (migrate + resume, ≤2.1 s) [E-74] |
| Kills a hostile workload | no — it shares your kernel; chroot escape preconditions present for uid 0 [E-76] | partially — seccomp+Landlock narrow it; shared kernel remains | **yes** — panic/OOM/compromise dies inside the VM |

## 2. Measured facts per route (this sandbox, 2026-09-09/10)

### 2.1 chroot [E-76]

Works. `chroot(2)` is not filtered (only the mount-family is), the appliance
is a plain directory, and the in-chroot toolchain installs over the network
(`apk add gcc` → 172 MiB) and executes at **native speed** (845.9 / 866.8
Mops/s, checksum identical to host).

Hard limits, all measured:

- `mount(2)` inside the chroot → EPERM: **no /proc, no /dev, no devtmpfs**
  (`ps` shows an empty table; `gcc` survives only because Alpine's cc1 does
  not need /proc).
- `/dev/null` becomes a **regular file** (no `mknod`; the minirootfs ships no
  device nodes) — any tool that requires a real character device fails.
- **No kernel isolation whatsoever**: the workload shares every namespace
  with the tenant; the classic double-`chroot` escape preconditions hold for
  uid 0 (second `chroot()` succeeds — E-76 `VMR-CHROOT-TWICE-OK`).
- No lifecycle semantics: no boot, no snapshot, no fleet isolation, no
  resource ceilings beyond the tenant's own.

### 2.2 podbox-style namespaces [C, cross-validated E-77]

container-research models the sandbox as three mechanisms — **N** (userns,
map `0 1000 1`), **F** (seccomp denylist), **M** (Landlock write allowlist
`{/tmp, /dev/shm, /workspace, /state}`) — and their `podbox` TOOL.md is a
Rust build-order for a runtime on top. Cross-validation of their claims on
this instance (experiment 77-):

| Their claim | This sandbox | Agreement |
|---|---|---|
| N: map `0 1000 1`, `setgroups` deny | identical (10-) | ✓ |
| F: `unshare`/`setns`/`mount`/`pivot_root`/`ptrace`/`process_vm_readv` EPERM | identical (10-/60-/77-) | ✓ |
| F: `clone(NEWNS)` via clone3 **not** denied | identical (60-/77-) | ✓ |
| M: write allowlist `{/tmp, /dev/shm, /workspace, /state}`; `/etc`, `/usr` denied | **exactly identical** (77-) | ✓ |
| M: `move_mount` EPERM by LSM, ENOENT discriminator | consistent with 60-/77- | ✓ |
| M: `/proc/pid/mem` O_RDWR EACCES | identical (10-) | ✓ |

**Additions this study contributes beyond their corpus**: the KVM/microVM
dimension (entirely absent from their tool survey), the **network-policy
layer** — a fourth mechanism their model does not cover (§`docs/seccomp.md`
§4: address/port/family-aware bind/connect denials with live reconfiguration
— the cgroup-BPF SOCK_ADDR signature), the **live patching event** (their F13
new-mount-API gap existed on 2026-09-09 and was closed by 2026-09-10, while
the clone3-namespace gap survived the same patch cycle — measured both sides,
§5.2 of the paper), and **in-guest toolchain builds for microVM guests**
(E-71).

Strengths: native speed, image-tooling reuse (OCI pull via their M1-M2
pattern), a real mode ladder (chroot rung → userns rung → Landlock rung).
Weaknesses here: mount attachment denial blocks overlayfs layering — their
TOOL.md's extraction strategy must fall back to ownership-neutral unpacking
(udocker-style libc interposition, their §11a); no VM semantics; and the
Landlock allowlist is **policy**, reconfigurable by the operator (we watched
a policy change live), so the write surface is not contractual.

### 2.3 podvm [E-70/72/62/63/64/65/73/74]

A TCG microVM is the only route with a **kernel boundary**: the workload's
kernel is not the host's. What that buys, measured: fleet at 4.06 boots/s;
snapshot-fork of live machines (≤2.1 s restore); guest↔guest UDP networking;
in-guest toolchain installs over the network; crash/compromise containment
(a guest `panic(8)` kills the guest, not the tenant). What it costs: ~21×
compute (TCG, [E-72]); 1.6–300 s boot by guest; no `/dev/kvm` acceleration
(today and, per §5.1 of the paper, provably not obtainable from inside);
no TCP listeners anywhere in the path.

## 3. Head-to-head on the decisions that matter

**"I need to compile and run untrusted code as fast as possible, and I trust
it completely."** chroot. Native speed beats everything; the trust assumption
carries the risk. [E-76]

**"I need the code confined but still native-fast, with filesystem
hygiene."** podbox-style. The Landlock write-allowlist plus per-run seccomp
is real confinement *for the surfaces it covers*; accept the shared kernel
and the policy-reconfigurability caveat. [C, E-77]

**"I need isolation I can defend, or the workload is hostile, or I need
checkpoint/clone/fleet-of-kernels."** podvm. The only route where hostile
code cannot touch the tenant kernel. You pay 20–25× compute on TCG. [E-62/64/72]

**"I need to build toolchains from the network inside the environment."**
chroot and podvm-Alpine both work [E-76/71]; podbox needs the udocker-style
extraction fallback because `move_mount` denial blocks layer application
[their §11a].

## 4. Interaction between the routes (they compose)

- chroot is a *component* of both others: podbox's lowest rung is a chroot
  rung; a podvm guest rootfs can be assembled from the same appliance tree.
- The clone3 gap (still open 2026-09-10) grants podbox-class privileges
  (namespace ownership) to any tenant process, in-process; podvm needs none
  of them. A patch closing clone3+NEWUSER would kill the first and leave the
  second untouched — routes fail independently, which is the argument for
  having all three in the toolbox.

## 5. Evidence index

chroot: `experiments/logs/76-chroot-route.log` ·
cross-validation: `experiments/logs/77-container-research-crossval.log` ·
podvm capabilities: `experiments/logs/{62,63,64,65,71,73,74}-*.log` ·
environment: `experiments/logs/{10,11,60,66,75}-*.log` ·
upstream model: Azathothas/container-research `paper_final.md` (2026-09-07)
+ `TOOL.md` (podbox build order), mirrored at
`experiments/work/container-research/`.
