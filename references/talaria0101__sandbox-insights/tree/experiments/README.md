# experiments

Numbered in the order they were first run. The number is the sequence,
not a priority; a replaced experiment keeps its number and the new one
gets the next, so a citation of `20-` keeps meaning what it meant.
Every script pins its inputs (`../scripts/lib.sh`), prints its
conditions, keeps its evidence in `logs/`, and never deletes its own
output.

| # | script | the question it answers | status on the logged run |
| --- | --- | --- | --- |
| 10 | `10-environment-census.sh` | what does this runtime permit, by mechanism — identity (N), syscall filter (F), path/provenance policy (M), and the unfiltered modern interfaces? | census taken; see the log for the full table |
| 15 | `15-network-policy.sh` | what does the network envelope permit, and what mechanism do the denials imply? | TCP bind denied everywhere; UDP + unix bind OK; loopback TCP connect reached the kernel (ECONNREFUSED); external :80/:443 OK |
| 20 | `20-mechanism-attribution.sh` | for each denial, is it the FILTER or a kernel-internal check? | every mount-family call and `process_vm_readv` answered EPERM to a bogus argument (refused pre-entry); controls (`pidfd_getfd`, `kcmp`) answered argument-shaped errnos |
| 25 | `25-ownership-wall.sh` | does image extraction hit a MAPPING wall no privilege clears? | real pinned rootfs: `Cannot change ownership to uid 0, gid 42: Invalid argument`; ownership-neutral extraction clean |
| 30 | `30-go-spawn-matrix.sh` | which `os/exec` SysProcAttr shapes call setgroups in the child? | `Credential{0,0}` FAIL; `NoSetGroups:true` OK; clone flags alone OK; combined row FAILs with the identical string |
| 35 | `35-interposition-reach.sh` | which payload classes does LD_PRELOAD reach? | dynamic C intercepted; static C invisible; Go invisible; the intercepted `lchown` to gid 42 still EINVAL — seeing a call is not clearing it |
| 40 | `40-namespace-gap.sh` | do the clone-family namespace flags work where unshare is denied, and what do owned namespaces buy? | clone3 NEWUSER\|NEWNET spawns; raw-ICMP socket OK in the child (owner CAP_NET_RAW); uid_map EMPTY; NEWPID child reports PID 1; exec drops the granted set (uid 65534) |
| 45 | `45-senotif-primitives.sh` | can a syscall tracer exist where ptrace is denied? | all six primitives proven: self-install, SCM_RIGHTS handoff, CONTINUE shepherding, forged retval (tracee read 4242), ADDFD (tracee read an injected fd), /proc/pid/mem while blocked; outer-ERRNO precedence shown |
| 50 | `50-tcg-boot.sh` | does software emulation boot a real kernel here, and how fast? | pinned Alpine 6.12.94-virt to guest userspace in **2.0–5.6s** across runs (load-dependent); initramfs assembled with NO host mknod; driver kills qemu after the marker (acpi=off halts, never exits) |
| 55 | `55-tcg-exec-and-bench.sh` | can the guest be driven (exec + exit codes), and does in-guest compute match the host? | FIFO-pair serial protocol; marker-bracketed exec; in-guest bench 256.8 Mops/s with checksum **identical** to the host run |
| 60 | `60-chroot-appliance.sh` | what does the cheap route — chroot into a real rootfs — buy and cost? | native speed (817.1 host vs 880.0 chroot Mops/s, checksum identical); no /proc (empty ps); double-chroot PRECONDITION holds for uid 0; appliance ships its own /etc/passwd |

## Exit codes

| code | meaning |
| --- | --- |
| 0 | the measurement ran and held |
| 1 | the measurement ran and the thing under test failed (or a committed negative result — each script says which) |
| 2 | the measurement could not run here (missing tool, unavailable facility) |

A "could not run" must never be readable as a denial; the scripts
keep the two apart by construction.

## Conditions

Every log opens with its conditions block: date, host, kernel, CPU,
memory, uid, capability mask, seccomp mode, uid_map, and the subject.
A number quoted from these logs without that block is a rumour.

## What this class of sandbox re-audits continuously

The operator changes the filter and the network policy on a day scale.
The census (10-) is cheap: re-run it before trusting any claim in
`docs/environment.md`, compare the fresh log against the document,
and amend the document in the same change where the host disagrees.
The same rule applies to the status column above: it describes the
committed logs, and the logs are one re-run away from being re-derived.
