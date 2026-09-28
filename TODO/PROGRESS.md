# Progress

## State

180 entries: 4 open, 1 partial, 0 blocked, 175 done.

The Windows lane now uses `wsl-toolkit 6.0.0`. Issue 68 closed after its
hosted gate passed. Pull requests 66 and 67 are open; neither
is on `main`. Their work is in [podssh.md](podssh.md) as T-1401 to
T-1404. Pull request 67 is the base: pull request 66 is an earlier subset
with no check runs and contributes no unique file. Pull request 67 carries
three red jobs (cage build with no tracked shim, one lint denial, one
document denial), so it is not ready to merge.

T-1112 is unblocked and still unproved. The host KVM node answers API
version 12, QEMU 11.1.1 with `qemu-img` installs from Arch extra, and the
ValidationOS disk is installed outside the tree with a matching digest.
What remains is the KVM guest run.

T-1112 remains partial. Its Windows guest arms ran under `tcg`; its
`kvm` arm needs a licensed image on a KVM host, under the entry's
accept-terms gate. The entry names the exact proof that will close it.

## Baseline and current lane

This session started 2026-09-28T05:27:58Z from clean `main` at
`5701a8b`. The host is Windows
`MINGW64_NT-10.0-26200`. The selected lane is the
`wsl-toolkit-podbox` base, with kernel `7.2.0-WSL2-STABLE` and
`podman 6.1.2`. The base was absent at session start and the session
entry point built it. Its status reports no cgroup delegation, so a
memory or CPU limit accepted by the engine is not proof that the limit
was enforced.

`py scripts/check-todo.py` was green at the start: 171 entries, 0
open, 1 partial, 170 done. `sh scripts/common/check-gate.sh --fast`
was green: 10 passed, 0 failed, 1 skipped. On this Windows host,
`sh scripts/dev.sh status` answers `unknown`; use
`wsl-toolkit --instance podbox base status --probe` for the lane.
Run `sh scripts/windows/run-in-base.sh` for the Linux build and test
gate. [`docs/containers.md`](../docs/containers.md) holds the
current procedure and the installed manual owns the flag reference.

## This session

T-1343 changed the Windows wrapper to send each job file through its own
input, kept CRLF removal, and selected ephemeral container life. It
removed the extra mode repair from that wrapper. Check 29 now reads the
tool's JSON cleanup report, including ended base sessions. Case 29b in
`scripts/plant.sh` supplies a kept session and requires the check to
name it.

`experiments/384-windows-lane-v6.sh` drove two jobs from one checkout
at the same time. Both received their own input and found an executable
script in the copied tree. Both exited 0. A third path probe ran an
existing experiment input and named the correct `/work` binary path
before it returned 2 for the absent binary. The cleanup report showed
three host job records; job-specific cleanup left zero. The conditions
and output are in
[`experiments/results/windows-lane-v6.txt`](../experiments/results/windows-lane-v6.txt).
A live ended base session made check 29 fail by id; the check passed after
that session was collected.

T-1344 checked the older experiment callers after the input path moved.
Twelve already use the working directory. Two now select that directory
for a Windows input and retain file-relative discovery for a native run.
All fifteen changed experiment scripts parse under `sh -n`.

T-1345 fixed the wrapper's caller interpreter. The first plant run
stopped at a Bash option because the wrapper used `sh` for every job.
The wrapper now reads an explicit Bash first line and selects Bash for
that job. POSIX jobs still use `sh`.
T-1346 changed the plant script's checkout path for a Windows job input.
Its first Bash run looked for `//scripts/check-todo.py` because `$0` was
`/in/job.sh`. The input form now takes the checkout from `/work`; a
native run still uses the script file's parent.
T-1347 moved check 27's shipped status source from a former sentence
in this page to the done entries in `TODO/milestones.md`. The first full
plant run caught 42 cases and missed 27a. The repeat caught all 43
plants; four controls stayed quiet.

The Windows procedure and script comments now state the behaviour read
from the 6.0.0 manual and measured on this host. The former text is in
[`docs/history/containers-before-toolkit-6.txt`](../docs/history/containers-before-toolkit-6.txt).
The old progress record and index order text are in `docs/history/`
so this page can give the current answer without a past session's
narrative.

The two SSH source trees were captured with their trackers and licences
under `references/`. The focused source comparison is in
[`docs/history/references/ssh-relay-2026-09-28.md`](../docs/history/references/ssh-relay-2026-09-28.md).
It found that the current dropssh source supports concurrent sessions
on one connection while its README still describes the old one-session
path. The current sandssh tree points to a separate shell project;
PR 67's task text says the shell is in sandssh. These source facts are
reflected in T-1402 and T-1403.

## Verification

The Windows drive last ran at 2026-09-28T11:01:21Z on this host. It
reported `verdict=matched`, two jobs with exit 0, the caller path check
with its expected exit 2, and zero kept records after collection.
The full plant suite ran in the Windows base on commit `c838759`:
43 caught, 0 missed, 4 controls quiet. The final Linux `dev.sh check`
exited 0: 10 passed, 0 failed, 0 skipped. The Windows host strict gate
passed 11 checks with no skip. The hosted gate for `81fec0c` passed all
four jobs. [Issue 68](https://github.com/Azathothas/podbox/issues/68)
closed after that result.
The base cleanup report is empty after the last drive.

The [session summary](SESSION-SUMMARY-2026-09-28.md) has the final
checks, change size, remote state, and machine state.

## Work order

1. T-1401 partial: land PR 67's SSH transport and server on `main`
   without the relay and remote group; fix its three red jobs and prove
   a real SSH command against a socketpair server in a fresh tree. The
   relay and remote group stay deferred.
2. T-1403: specify and prove two clients on one relay connection.
   Test frame direction, isolation, close, and bounded cleanup.
3. T-1402: provide and prove an interactive session where no pty exists.
   A one-shot command is not this proof.
4. T-1404 machine arm now: dispatch `machine ssh` to a guest podbox
   runs, with podman parity. The remote SSH arm stays deferred with the
   relay.
5. T-1112: finish the KVM image arm when the named host and licensed
   image are available.

## Open questions

The SSH relay protocol must name which peer speaks the simple rendezvous
form and which speaks the multiplexed reverse form. T-1403 records the
tests that settle it. No operator choice is needed before T-1401 starts.
T-1112 still needs the KVM guest run named in its entry.
