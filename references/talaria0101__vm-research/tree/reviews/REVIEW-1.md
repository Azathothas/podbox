# REVIEW 1 — every verdict row re-checked against its committed log

Date: 2026-09-09 · Reviewer posture: fresh read, zero reliance on session
memory; each claim below was re-derived from `experiments/logs/` at review
time. Commands and their outputs are quoted.

## Checks that passed

| Claim | Log line found |
|---|---|
| firecracker dies at KVM object creation | `Error creating KVM object: No such file or directory (os error 2) …` (30-) |
| 20- wall time 3.14 s | `### wall seconds to qemu exit: 3.138788217` |
| 21- wall time 3.25 s, guest uptime 1.47 s | `### wall seconds to qemu exit: 3.247923550` + `VMR-GUEST-UPTIME: 1.47` |
| smolBSD guest executed a command | `VMR-SMOLBSD-OK` (standalone line, CR-terminated) + `smolBSD 11.0_STABLE (SMOL) #17 … amd64` from in-guest `uname -a` (42-) |
| nanos guest ran | `VMR-NANOS-OK pid=2` (45-) |
| forkd preflight | `✗ kvm /dev/kvm does not exist` (35-) |
| cratera doctor | `✗ /dev/kvm not found. Host does not have hardware virtualization enabled.` (40-) |
| UML killed by ptrace ban | `Checking that ptrace can change system call numbers...ptrace: Operation not permitted` (50-) |
| isapc kernel panic | `Kernel panic - not syncing: Attempted to kill the idle task!` (51-) |
| pullrun container mode | `runc run failed: unable to set hostname without a private UTS namespace` (43-) |
| virtkit segfaults | `vk --version -> rc=139`, `vk version -> rc=139` (41-) |
| lima TCG auto-fallback | `[hostagent] /dev/kvm is not available. Disabling KVM. Expect very poor performance.` (33-) |
| microsandbox [S] quote | `**Linux**: KVM enabled.` (37-) |
| libkrun [S] quote | `vstate.rs:422: let kvm = Kvm::new().expect("Error creating the Kvm object");` (37-, 39-) |

## Defects found and fixed by this review

1. **Lima's decisive evidence was not committed.** The bind-failure proof
   (`Could not set up host forwarding rule 'tcp:127.0.0.1:40543-:22'`) lived in
   `work/lima-home/alpine-tcg/ha.stderr.log`, which is gitignored. The README
   row cited evidence the repo did not contain. Fix: `logs/33-lima-ha-stderr.log`
   now carries the trimmed hostagent log with a conditions header.
2. **smolvm's log was incoherent.** The committed run had a stale machine from
   an unlogged manual test (`create` errored "already exists"), so the
   `start` rc (153, SIGXFSZ) sat next to a hardcoded decision line claiming
   "start returned 0" from the earlier manual run. Fix: script now
   `machine delete`s before `create` (log is reproducible), and the recorded
   decision matches the log: **start rc=153 (SIGXFSZ at RLIMIT_FSIZE=500 MB)
   even at `--mem 256`; state stays `created`; exec reports `not running`.**
   README.md and experiments/README.md rows corrected to this wording.
3. **Exit-code drift in two early scripts** (30-, 33-): unconditional `exit 1`
   / pipeline-masked codes at call sites. Cosmetic — the verdicts come from the
   logged markers, which R1 verified line-by-line above. Left as-is; noted here
   so nobody mistakes the exit code for the verdict source.

## Verdict

Matrix rows and logs now agree for all 22 experiments. The two defects above
are the kind this review exists for: both were caught by re-reading logs
without trusting the narrative.
