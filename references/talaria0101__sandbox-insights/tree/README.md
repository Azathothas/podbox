# sandbox-insights

What a Linux sandbox that presents uid 0 with a full capability set
while refusing most kernel operations actually permits — measured,
attributed to mechanism, and turned into working recipes and design
lessons. Every claim below is produced by a numbered, runnable script
in [`experiments/`](experiments/) and every log is committed. The
host is re-audited by its operator continuously; **method is binding,
not advisory** (rules in [`docs/AGENTS.md`](docs/AGENTS.md)).

## The environment, in one paragraph (evidence: `experiments/logs/10-`, `15-`)

The tenant is uid 0 **inside a user namespace** (map `0 0 1` at
measurement) with the full capability mask — and a seccomp denylist
that refuses `mount`, `umount2`, `pivot_root`, the new mount API,
`unshare`, `setns`, `ptrace`, `bpf`, `keyctl`, `open_by_handle_at`
and `process_vm_*` **before the syscall body runs**; a path-scoped
write allowlist (`/tmp`, `/dev/shm`, `/workspace`, `/state`) that no
filter could implement; an address-aware network policy (TCP bind
denied, UDP and unix bind fine); `/proc` mounted from the initial
userns, which makes `uid_map` unwritable by anyone and `/proc/self/mem`
read-only; `RLIMIT_FSIZE` 500 MiB/file; no `/etc/passwd`; no
`/dev/kvm` node (the host drivers exist — the wall is exposure, not
hardware). What remains is a real capability tier: RWX mappings,
memfd, chroot, `clone3` with namespace flags, Landlock, seccomp
user-notification, outbound network, and qemu TCG.

## The verdict matrix

Tag **[V]** = verified in this tree (runnable) · **[S]** = established
from pinned source read. Every row cites its experiment.

| # | subject | verdict on the logged run | evidence |
|---|---------|---------------------------|----------|
| 10 | mechanism census | identity + N/F/M rows settled; `fsopen` filtered (was open in an earlier revision — closed live); `clone3` executes; Landlock ABI present | [V] `logs/10-` |
| 15 | network policy | TCP bind denied everywhere; UDP + unix bind OK; loopback TCP connect allowed now (earlier revision denied it); external egress OK | [V] `logs/15-` |
| 20 | bogus-argument attribution | whole mount family + `unshare`/`setns`/`ptrace`/`process_vm_readv` refused pre-entry; `clone3`/`openat2`/`io_uring`/controls execute | [V] `logs/20-` |
| 25 | the ownership wall | real pinned rootfs: `Cannot change ownership to uid 0, gid 42: Invalid argument`; ownership-neutral extraction clean | [V] `logs/25-` |
| 30 | Go spawn matrix | `Credential{0,0}` FAIL; `NoSetGroups:true` OK; clone flags alone OK; the combined row FAILs with the **identical string** — the ambiguity that builds wrong architectures | [V] `logs/30-` |
| 35 | interposition reach | dynamic C intercepted; static C invisible; Go invisible; the intercepted `lchown(0,42)` still EINVAL — seeing a call is not clearing it | [V] `logs/35-` |
| 40 | the namespace gap | `clone3(NEWUSER\|NEWNET)` spawns; raw-ICMP socket OK in the child (owner privilege); `uid_map` EMPTY; `NEWPID` child reports PID 1; **exec drops the granted set** — in-process only | [V] `logs/40-` |
| 45 | ptrace-free tracing | all six seccomp user-notification primitives proven live: self-install, SCM_RIGHTS handoff, CONTINUE shepherding, forged retval (tracee read 4242), ADDFD (tracee read an injected fd), `/proc/pid/mem` while blocked; outer-ERRNO precedence shown | [V] `logs/45-` |
| 50 | TCG boot | pinned Alpine 6.12.94-virt to guest userspace in **2.0–5.6 s** across runs (load-dependent); initramfs device nodes written byte-wise (no host mknod); driver kills qemu after the marker (acpi=off halts, never exits) | [V] `logs/50-` |
| 55 | driven guest | FIFO-pair serial protocol; marker-bracketed exec, exit codes propagated; in-guest bench **checksum-identical** to host; TCG tax workload-shaped: integer 1.0–3.2×, dependent-FP 6.9–12× | [V] `logs/55-` |
| 60 | chroot appliance | native speed (817.1 host vs 880.0 chroot Mops/s, checksum identical); empty `ps`; double-chroot escape **precondition** holds for uid 0; no kernel boundary | [V] `logs/60-` |

Plus the blocked column, so nobody re-derives it: TCP listeners,
KVM anything, runc-style OCI containers (UTS/mount unshare denied),
ptrace-based tracers and UML guests, uid-map identity switching,
guest images over 500 MiB/file (`SIGXFSZ`), overlayfs layering
(mount attachment denied).

## Layout

```
docs/           the distillation
  AGENTS.md       the router — read in full before working here
  environment.md  the measured contract
  attribution.md  the four-mechanism model + the discriminator
  walls.md        where tooling stops and what the stop is
  recipes.md      what runs, exactly (TCG, chroot, senotif, shims)
  design-lessons.md  honest degradation, mode ladder, diagnostics
  paper.md        the paper
  reviews/        documented review passes (REVIEW-1..N)
scripts/        the instrument library (standalone, self-asserting)
experiments/    numbered scripts + committed logs/ (the evidence)
```

## Reproduce

```sh
./experiments/10-environment-census.sh    # the census everything interprets
./experiments/15-network-policy.sh
./experiments/20-mechanism-attribution.sh # the discriminator + controls
./experiments/45-senotif-primitives.sh    # the tracer tier, proven
./experiments/50-tcg-boot.sh              # a real kernel in ~2 s
```

Every script pins its inputs (`scripts/lib.sh`), prints its
conditions, and exits 0 ran-and-held / 1 ran-and-failed-or-negative /
2 could-not-run. **Re-run the census before trusting any claim** —
the filter and the network policy change on a day scale, and a closed
gap is a committed negative result, not a failure.

## Licence

[0BSD](LICENSE). Everything here — the paper, the instruments, the
experiments, the evidence — is free for any use.
