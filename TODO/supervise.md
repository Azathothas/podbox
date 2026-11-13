Area: supervise.

Record semantics: [task rules](RULES.md#5-entry-closure).


`crates/podvm-supervise`. `TOOL.md` section 6.6, milestone M4.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

Running state is launcher state, in the launcher. The prior art's own lifecycle
capture contains a failed run because it decided "running" by sleeping three
seconds and looking, and M4's acceptance is twenty consecutive passes because
one pass proves nothing about a race.

# T-0601: One pidfd per direct child, `waitid` for status

**Source:** `TOOL.md` section 4.2, section 6.6; `paper_final.md` section 10.6
**Category:** supervise
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

Membership inferred from the filesystem races against exit, depends
on other processes' root links being readable, and trusts a path any
writable payload can create. Scanning `/proc/*/root/<marker>` is all
three at once.

## Premise

Read.

## Approach

Hold a pidfd per direct child from the moment it is created. Exit
status via `waitid`, wakeups via the pidfd in a poll set. Keep the
container table in the launcher, persisted to the store, and never
reconstruct it from `/proc`.
⛔ Say what a pidfd is not: it addresses one process, it does not
contain descendants, and it is not a PID namespace. Grandchildren
that reparent are outside podvm's reach and `inspect` says so
rather than implying containment.

## Decision

A pidfd rather than a pid plus a start time. The pid is reusable
and the start-time check is a heuristic; a pidfd cannot address a
process that has been reaped.

## Prove

`podvm run -d --name pidfdprobe public.ecr.aws/docker/library/alpine:3.20 sleep 5 && podvm wait pidfdprobe | grep -qx 0 && podvm rm pidfdprobe`

## Done

**Done, 2026-09-09.** `crates/podvm-supervise/src/launcher.rs`. One launcher per
container holds a pidfd on its payload from the moment the payload exists,
`ppoll`s it for readiness and reaps it with `waitid(P_PIDFD)`, so the status
cannot be redirected by pid reuse between the readiness and the reap. The
container table is `crates/podvm-supervise/src/table.rs` and nothing in it is
reconstructed from `/proc`.

⛔ **Said, not implied**, and `podvm inspect --format '{{.Contains}}'` prints it:
a pidfd addresses ONE process. It does not contain descendants and it is not a
PID namespace. Since T-1708 the launcher (and podvm itself for a foreground
run) is the child subreaper of the payload's tree, so a grandchild that
reparents comes to it and is killed at the end; a process that reparents
above it stays outside podvm's reach.

⚠ **A defect the `waitid` path cost, and it is a fixed offset rather than a
race:** `si_status` was read at byte 20 of the `siginfo_t`, which is `si_uid`. A
SIGTERMed payload reported exit **128** instead of 143. On a 64-bit architecture
the three leading ints are followed by four bytes of padding and `_sifields`
begins at 16, so `si_status` is at 24. `pod_hostprobe::sys` carries
the reasoning beside the read.

# T-0602: Never decide "running" by sleeping and looking

**Source:** `TOOL.md` section 6.6; `paper_final.md` section 10.6
**Category:** supervise
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

A fixed sleep is a scheduling assumption. The prior art's own
capture of this lifecycle contains a failed run for exactly that
reason, and it passed on other days.

## Premise

Read, in the account of that capture. The general rule is in the
binding methodology: `docs/methodology/authoring.md:1-20` states
that an acceptance "waits on the condition, never on a guessed
duration", and that "both of these will happen" is the same
assumption as "this will happen in N seconds".

## Approach

Wait on the pidfd, or on a readiness fd the child writes to and the
parent reads. `start` returns when the child has reached a defined
state, not when a timer expires. Every wait has an upper bound and
a distinct outcome for the bound being reached, per
[RULES.md](RULES.md) section 8.
⛔ podvm inherits this as a product requirement, not just as
session hygiene: a container runtime that hangs waiting is unusable
by the audience it is built for.

## Decision

A readiness fd over polling `ps`. Polling reintroduces the interval
the entry exists to remove.

## Prove

`for i in $(seq 20); do podvm run -d --name loop$i public.ecr.aws/docker/library/alpine:3.20 sleep 1 && podvm ps -q | grep -q . && podvm stop loop$i && podvm rm loop$i || exit 1; done`

## Done

**Done, 2026-09-09**, and it took three failing runs and a door sweep to get
there. `./experiments/230-lifecycle-loop.sh 20` reports **20 of 20 consecutive
passes**.

⭐ **What is in.** Readiness is an `O_CLOEXEC` pipe that the payload's `execve`
closes, so `start` returns when the exec has SUCCEEDED rather than when a timer
expired; the launcher waits on `ppoll` over the payload's pidfd and its control
socket; `stop` and `wait` block on the launcher through that socket with a read
timeout. ⛔ There is no `sleep` anywhere in the path, and
`experiments/230-lifecycle-loop.sh` has none either.

⛔ **What the acceptance found before it passed, and this is the entry's whole
point.** The twenty-pass loop failed three runs out of three:

| run | consecutive passes | failed at |
| --- | --- | --- |
| 20 iterations | **10 of 20** | `stop` |
| 20 iterations, again | **9 of 20** | `stop` |
| 3 iterations | **1 of 3** | `stop` |

Always the same message: `no launcher is listening for <id>: No such file or
directory`. The container table still says running at that moment, so the
launcher's own lock was still held and its control socket was not there.
⚠ Reproduced by hand three times in a row against a warm store and it passed
every time, so what the loop adds is a cold store, an `rm` of the previous
container immediately before, and a `pull` at the start.

⭐ **THE CAUSE, found by the door sweep and not by the loop: `stop` connects
TWICE.** Once to send `SIGTERM`, once to wait for the exit. A payload that dies
quickly lets the launcher reap it, remove its control socket and exit **in
between the two connects**, so the second one answers `ENOENT` and `stop`
reported the fastest possible success as "the container is not running". ⛔ The
failure was more likely the faster the container stopped, which is why a
hand-driven loop with a person's pauses in it passed every time.

⭐ **The fix is that a launcher which is already gone is a container that already
stopped, and `Ended` says which of three things happened**: the launcher
answered with a code, there is no launcher left to answer, or the bound was
reached. Only the last means podvm does not know, and collapsing the first two
is what made this a failure. `podvm wait` had the same shape and takes the same
fix, re-reading the table because the launcher's last act is to write the code
into it.

⚠ **Two candidates were written down before the sweep and BOTH were wrong**,
which is worth keeping: `contain::within` re-appends the tail after resolving the
nearest existing ancestor, so `remove_dir_all` can never widen to the containers
directory; and `flock` is held on an open file description, so `reconcile`
opening and closing its own fd cannot release a launcher's lock. Reading the code
for what else reaches the socket is what found it.

# T-0603: `PR_SET_PDEATHSIG` fires on the creating thread's exit

**Source:** `TOOL.md` section 6.6; `paper_final.md` section 10.6
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

It does not fire on an ancestor's death. In a threaded supervisor
this produces surprising early exits that look like crashes, and a
wrapper such as `timeout(1)` around the launcher is enough to kill a
detached child unexpectedly.

## Premise

Read. It is a kernel property, not a runtime property, and it is one
of the four rows behind `TOOL.md` section 3's language decision: a Go
supervisor cannot control which thread it is on.
`references/89luca89__lilipod/tree/pkg/procutils/proc_utils.go:48-56`
sets `Pdeathsig: syscall.SIGTERM` in the same `SysProcAttr` as the
`Credential` that fails, which is the shape the account describes.

## Approach

podvm is single-threaded on the path that clones, chroots and
execs (`TOOL.md` section 4.2), so the creating thread is the process. Keep
it that way: spawn threads only for log pumps and the notification
supervisor, and assert it in a test rather than in a comment.
Where `PDEATHSIG` is used, lock the spawning thread.

## Decision

Keep the spawn path single-threaded rather than pinning a thread.
Pinning works and it is a constraint every future contributor has to
know; a single-threaded path is a constraint the type system can be
made to hold.

## Prove

`podvm run -d --name pdprobe public.ecr.aws/docker/library/alpine:3.20 sleep 30 && grep -qx 1 /proc/$(podvm inspect --format '{{.Pid}}' pdprobe)/status.threads 2>/dev/null || test "$(ls /proc/$(pgrep -f 'podvm run' | head -1)/task | wc -l)" -le 3`

## Done

**Done, 2026-09-09.** The spawn path is single-threaded and nothing on it spawns
a thread: the launcher forks with `clone_fork`, the payload's `chroot` and
`execve` happen in that child, and `PR_SET_PDEATHSIG` is not used at all, so the
"fires on the creating thread's exit" trap has no way to bite. The launcher also
`setsid`s, so it outlives the shell that started it and holds no controlling
terminal.

⚠ **Asserted in two places rather than commented, and the first version of the
first one was wrong.** A unit test read `/proc/self/task` before and after the
spawn path and failed about one run in five, because `cargo test` runs tests in
THREADS of one process and that count moves for reasons that have nothing to do
with podvm: a test whose name claimed more than it checked. It now reads the
two crates' own sources and asserts neither spawns a thread at all, with the
needles assembled at run time so the assertion does not match itself. ⭐ The
runtime half is clause 3 of `experiments/230-lifecycle-loop.sh`, which reads
**1** thread off a real detached launcher.

# T-0604: Running state is launcher state

**Source:** `TOOL.md` section 6.6
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Two launchers, or a launcher restart, must not disagree about what
is running, and neither may read the answer out of a directory a
payload can write to.

## Premise

Read.

## Approach

A single container table in the store, written by the launcher,
locked with the same inheritable lock fd as T-0204. On start, a
launcher reconciles the table against its pidfds and marks anything
it cannot address as exited, recording when it noticed. It never
infers membership from a marker file.

## Decision

Reconcile on start rather than trust the table. A launcher killed
with `SIGKILL` leaves the table saying "running", and the only
honest answer after that is "this process exited while podvm was
not watching".

## Prove

`podvm run -d --name stateprobe public.ecr.aws/docker/library/alpine:3.20 sleep 30 && kill -9 "$(podvm inspect --format '{{.Pid}}' stateprobe)" && podvm ps -a --format '{{.Names}} {{.Status}}' | grep '^stateprobe ' | grep -qi exited && podvm rm stateprobe`

## Done

**Done, 2026-09-09.** Clause 2 of `experiments/230-lifecycle-loop.sh` drives it:
a detached container whose LAUNCHER is `SIGKILL`ed reads `dead`, `inspect
--format '{{.ExitCode}}'` prints a dash, a time is recorded for when it was
noticed, and `podvm wait` on it exits **125** rather than printing a number.

⭐ **The mechanism.** A launcher holds an exclusive `flock` on its container's
own lock file for its whole life, so `table::reconcile` decides whether a
launcher is gone by TRYING TO TAKE IT: a lock this process can take is a launcher
that is not there. ⛔ Never by pid, and never with a start-time check beside it: a
pid is reused and the check that clears a stale pid file is the race being
closed. Such a container becomes `dead`, with the time it was NOTICED and ⛔ **no
exit code at all**; `podvm wait` on it refuses rather than printing a number,
and `inspect --format '{{.ExitCode}}'` prints a dash.

⚠ **The payload outlives its launcher and that is not hidden.** Killing the
launcher does not kill the container's process, so the clause kills it too on the
way out; podvm has no PID namespace and `inspect --format '{{.Contains}}'` says
exactly that.

# T-0605: Capture logs at spawn, from the descriptors opened in step 2

**Source:** `TOOL.md` section 6.6, section 6.5
**Category:** supervise
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

A log sink opened after the chroot cannot reach the store, which is
outside the new root. Opening it afterwards is the same mistake as
step 2 of section 6.5 in a different place.

## Premise

Read.

## Approach

Open the log sinks in step 2 of the entry sequence, before the root
change, and hand the descriptors to the child. `logs` reads the
store file; `attach` is a tail of the same file plus a signal proxy
and is marked Degraded for that reason.
`--log-driver` accepts `json-file` only, and any other value is a
named refusal rather than a silent substitution.

## Decision

A file in the store rather than a pipe pumped by the launcher. A
pipe loses everything written after the launcher dies, and the
launcher is the process most likely to be killed by an outer
`timeout`.

## Prove

`podvm run --rm --name logprobe public.ecr.aws/docker/library/alpine:3.20 sh -c 'echo out; echo err >&2' >/dev/null 2>&1; podvm logs logprobe 2>&1 | grep -q out`

## Done

**Done, 2026-09-09.** Clause 4 of `experiments/230-lifecycle-loop.sh`:
`podvm run -d ... sh -c 'echo out; echo err >&2'` then `podvm logs` reads back
`out err`, so both streams reached one file opened before the chroot.

The log sink is opened in the launcher BEFORE anything changes root, and its
descriptors are handed to the payload as fds 1 and 2 through the same
`Plan.fds.pass` the entry sequence already had; stdin is `/dev/null`, opened in
the same place. `podvm logs` reads the file out of the store. ⛔ A sink opened
after the chroot cannot reach the store, which is outside the new root, and that
is step 2 of `TOOL.md` section 6.5 made in a different place.

⚠ `--log-driver` is not implemented at all rather than accepted and ignored, and
`attach` is a `None` row in the parity table for the same reason.

⚠ The two streams are interleaved into one file rather than kept apart, which
is what `json-file` does and is why `logs` has no `--tail` or stream selector.

**Partial, 2026-09-25.** Issue 56 reopens the `--log-driver`
half: the Approach requires `--log-driver` to accept `json-file`
only with any other value a named refusal, the Done records it not
implemented at all, and the parity table carries no row for it, so
the parser answers `unknown option`. Measured on the lane
2026-09-25 (`experiments/results/triage-353.txt` clause
`56-log-driver`): exit 125 with the no-row message. Add the
`--log-driver` row with its parser arm: `json-file` accepted, any
other value refused naming the value and this runtime's single
sink. A stream split stays a separate entry; it does not ride here.
Fix area is `crates/podvm/src/parity.rs`, the `run` parser,
and the `crates/podvm-supervise` sink. Risk if wrong is a docker
flag that reads as an omission rather than a decision. Prove is the
accept arm and the refusal arm through the shipped binary, beside
the table row.

**Done, 2026-09-25.** The row and both arms landed with T-0801:
`--log-driver` is Stub in the parity table (`json-file` accepted
and changing nothing: one raw interleaved file per container
either way), the `run` parser accepts `json-file` in both
spellings and refuses any other value naming it. No sink change:
the single file is what `json-file` selects here, and a stream
split stays out of scope as decided. Prove through the shipped
binary on the lane (`experiments/355-parity-curated.sh`, same
conditions as T-0801 above): `log-driver-json` and
`log-driver-json-eq` exit 0, `log-driver-syslog` exits 125
naming the value. In-suite:
`log_driver_takes_json_file_and_nothing_else`.

# T-0606: The notification tier: probe three legs, refuse the tier, never fall back per call

**Source:** `TOOL.md` section 4.1, section 6.6; `references/multikernel__sandlock`
**Category:** supervise
**Milestone:** none
**Priority:** P0
**Effort:** L
**Status:** done

## Problem

`supervise` is the only rung whose failure is silent by default. Its
listener keeps working after its argument-reading channel dies, and
a supervisor whose per-syscall fallback is "continue" then reports
success for mediation it never performed.

## Premise

⭐ **Read at file and line, and the whole chain is traceable in one
tree.** In `references/multikernel__sandlock`:

1. `tree/crates/sandlock-core/src/seccomp/notif.rs:1015-1034`
   `read_child_mem_vm()` calls `libc::process_vm_readv` at
   `:1026`. That is the only read channel; there is no
   `/proc/pid/mem` fallback anywhere in the tree.
2. `tree/crates/sandlock-core/src/seccomp/notif.rs:1070-1081`
   `read_child_mem()` propagates the failure.
3. `tree/crates/sandlock-core/src/chroot/dispatch.rs:213-231`
   `read_path()` returns `None` on any read failure.
4. `tree/crates/sandlock-core/src/cow/dispatch.rs:203-206` turns
   that `None` into `NotifAction::Continue`, and the same shape
   appears at `:200`, `:219`, `:226`, `:238`, `:245`, `:253`,
   `:276` and `:293`.
5. `Continue` lets the kernel run the syscall unmediated, so
   nothing is recorded.
6. `tree/crates/sandlock-cli/src/main.rs:740-747` then prints
   `sandlock: dry-run: no filesystem changes`.

On a runtime where `process_vm_readv` is filtered, every step 4
fires and step 6 reports no changes while the files change.
⭐ **Its own audit examined every one of those sites and missed
this.** `tree/crates/sandlock-core/src/cow/dispatch.rs:6-24` is a
module header written by that audit, titled "Continue safety (issue
#27)", arguing each `Continue` is safe because "we're not making a
security decision whose validity depends on the kernel re-reading
the same memory we read". That is correct about security and silent
about reporting.
⭐ **And its tracker settles a second question `TOOL.md` does not
raise.** Issue #27 is open. `seccomp_unotify(2)`'s own manual warns
the mechanism must not be used to make security policy decisions,
because it is race-prone. The fix that survived four rounds of
argument (PRs #28, #29, #33, #35) requires `PTRACE_SEIZE` plus
`PTRACE_INTERRUPT` on every thread of every process in the sandbox
before sending `Continue`, and **`ptrace` is filtered on this
runtime**. So `supervise` here has no read channel and no way to be
made race-safe if it had one. PR #29 also dropped path strings from
the policy surface entirely, on the ruling that "path-based access
control belongs in static Landlock rules".

## Approach

Probe all three legs before selecting the tier: the listener, a
working channel for reading the child's arguments, and ptrace to
make a `Continue` race-safe. Refuse the tier if any is missing.
⛔ Never fall back per call. A per-call fallback is what produces a
false report, and it is indistinguishable from success in the
output.
⚠ The third leg is ptrace, not the `SECCOMP_IOCTL_NOTIF_ADDFD`
injection an earlier revision of this Approach named. Nothing in
the tree injects a descriptor (no reference to `ADDFD` anywhere
in `crates/`), so a leg for it would green-light a mechanism
with no consumer and no incident behind it. What the Problem
needs is the only known fix for race-unsafe `Continue`, which
issue #27 settles is `PTRACE_SEIZE` before it, and the Decision
below refuses the tier where that fix cannot run.
⚠ The channel leg aims `process_vm_readv` at live memory, not
`open /proc/self/mem`. The Premise's step 1 settles that the
lineage's only channel is `process_vm_readv` with no mem
fallback, so a mem leg would pass a channel the implementation
does not use. The legs are the tier's own rows
(`Group::Supervise`), never the census and attribution rows
that answer other groups' contracts.

## Decision

Refuse the whole tier when any leg is missing, rather than offering
a reduced `supervise`. A reduced one is what the corpus measures
failing: the listener keeps working, so the mode reports as active,
and only the mediation is gone. There is no output that
distinguishes it from the working case, which is why the refusal has
to happen before the tier is entered.
Status note: **open, and it was wrongly marked blocked.** Reconciled on
2026-09-11. ⛔ `docs/methodology/sessions.md` defines blocked as
"somebody outside this session must act", and nobody does. The
deliverable here is podvm's OWN three-leg probe and the tier
refusal it drives, and both are writable today against a target
where every leg is denied.
⚠ What the corpus settles is the OUTCOME on that target, not
whether the work can start: both read channels are denied there and
`ptrace` is filtered, so the tier is refused. ⛔ That refusal still
has to be a measured verdict from the probe rather than a constant
in the code, because the next runtime may permit a leg this one
does not. Treating "the answer is known" as "the work is blocked"
is what kept this entry closed to work for two sessions.

## Prove

`podvm probe --json | jq -e '.tiers.supervise | (.legs | length == 3) and (([.legs[] | select(.verdict != "ok")] | length == 0) == (.refusal == null))'`
⚠ Amended 2026-09-22: the committed spelling selected `.ok` and
`.refused` fields the legs never carried (they carry `verdict`
strings and the tier carries one `refusal`), so it could never
pass. The new spelling asserts what the tier promises: three
legs, and a refusal exactly where not every leg is `ok`.

## Done

**Done 2026-09-22.** The tier's own three legs, and the refusal they drive.
`pod_hostprobe::probes` gains `Group::Supervise` with the block
and `SUPERVISE_LEGS`: `seccomp(NEW_LISTENER) [supervise]`, a live
`process_vm_readv` aimed at one byte of own memory, and
`ptrace(PTRACE_TRACEME) [supervise]`. The listener and ptrace rows share
their bodies with the census and attribution rows that run the same
operation: one implementation, two readings. `s_read_channel` is new; the
bogus-pid discriminator beside it separates a filter from a policy and
never copies a byte, so it cannot answer the channel question.
`pod_hostprobe::supervise` assesses the legs the way
`machine.rs` does, with the refusal naming every missing leg. The report
carries both tiers in one `tiers` object plus a `supervise, one leg per
fact` text block, and the selection reads the owned rows: all three `ok`
enters the tier, anything else pushes the named `Rejection` and falls
through. The "never fall back per call" half is an audit, not a deletion:
no notify mediation exists anywhere in the tree (`podvm-supervise` is the
lifecycle in `launcher.rs`/`table.rs`), so there is no per-call fallback
to remove, and the refusal before entry is what enforces the Decision.
Driven 2026-09-22 on a lane-built musl release binary: `probe --json`
exits 0, the Prove exits 0, all three legs read `ok` with `refusal: null`
(the lane container permits all three; the target refuses all three, and
that path is covered by the assessment unit tests and the selection
refusal test, not by this run). The lane found four defects, all fixed
here: the tiers merge dropped the `o.obj` closure's `)` and broke the
build, one new test asserted the text block against the JSON document,
the machine-shape test needed both tiers' rows to keep its no-null
guard honest, and one fmt spot in the selection fixture. Counts move to
4 open and 135 done.

**Partial, 2026-09-25.** Issue 34 reopens this entry: the tier
refusal shipped, and no degraded supervision exists. Where
`process_vm_readv` or ptrace is `EPERM` the whole tier is gone,
including lifecycle tracking that needs neither. Split mediation
(notify) from supervision (`pidfd` with `waitid` with the launcher
state, as T-0601, T-0602 and T-0604 already hold): refuse mediation
where the legs are missing, keep supervision with a banner stating
no syscall mediation. One citation correction: the issue traces the
`Continue` dispatch to `notif.rs:1015-1034`, but those lines carry
only the `read_child_mem_vm` helper; the chroot and cow dispatch
half of the chain lives where this entry's Premise puts it
(`chroot/dispatch.rs:213-231`, `cow/dispatch.rs:200-293`), so a
re-check starts there. Measured on the lane 2026-09-25
(`experiments/results/triage-353.txt` clause `30-probe-chroot`):
all three supervise legs `ok` with `refusal: null`, so the lane
cannot show the fallback; the target shape (`ok`, `EPERM`,
`EPERM`) is the driver. Fix area is `pod_hostprobe::probes`
with `supervise.rs` and the report selection. Risk if wrong is a
silent `Continue`-style false success: the banner must state
mediation off wherever the fallback runs. Prove is the probe Prove
extended with supervision available where `pidfd` and `waitid` hold
even when the notify legs are denied, with the lifecycle loop 230
still 20 of 20 on such a host.

**Done 2026-09-25.** Mediation and supervision are split. Two new
`Group::Supervise` rows measure the pair the lifecycle runs on:
`pidfd_open(own pid)` (the number answering on its own pid) and
`waitid(P_PIDFD, child)` (a trivial fork reaped through its pidfd,
the composition the launcher runs; the verdict comes from waitid
answering under WNOHANG, so nothing waits). `supervise.rs` reads
them through `SUPERVISION_LEGS` beside mediation's three, with its
own `supervision refused:` sentence. The document carries
`tiers.supervision` (legs plus refusal) beside `tiers.supervise`,
whose three legs and Prove are unchanged; the evidence prints a
`supervision, one leg per fact` block; the run banner states `no
syscall mediation on this machine` exactly where mediation is
refused and supervision holds, and stays silent where mediation
holds. Selection is untouched: a refused mediation still keeps
the Supervise rung out.

Driven by `experiments/359-supervision-split.sh`, exit 0 on the
lane (`rust:1.98.1-bookworm` job container, kernel
`7.2.0-WSL2-STABLE`), report in
`experiments/results/supervision-split.txt`: the mediation Prove
holds (three legs, refusal null iff all ok); supervision is
available (two legs, refusal null, both rows `ok` in
`probe --rows`); the banner carries no mediation-off line on a
run; the 230 loop passes 20 of 20 on the debug binary over the
pinned alpine. The denied-notify shape is unit-pinned (mediation
refused naming EPERM beside held supervision, banner line on,
banner line off where mediation holds): the lane permits all
three notify legs, so no lane run can show the fallback. Full
`podvm-probe` suite green on the same drive (107 passed,
0 failed).

# T-0607: The lifecycle, twenty times, twenty passes

**Source:** `TOOL.md` section 5 M4, section 9
**Category:** supervise
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

One pass proves nothing about a race, and the race is the reason
this milestone exists.

## Premise

Read, from the account of a capture containing a failed run beside
a passing one.

## Approach

`create`, detached `start`, `ps` shows it running, `exec` prints a
marker, `stop`, `rm`. Twenty consecutive iterations, all passing,
with no sleep anywhere in the loop or in the implementation under
it. A single failure fails the milestone; retrying it is what turned
a race into a published pass elsewhere.

## Decision

Twenty rather than a timed soak. A count is reproducible on a
different machine and a duration is not, and
`docs/methodology/authoring.md:1-20` rules out the duration form.

## Prove

`./experiments/230-lifecycle-loop.sh 20` exits 0

## Done

**Done, 2026-09-09.** `./experiments/230-lifecycle-loop.sh 20` reports **20 of
20 consecutive passes**, and the three clauses after it run for the first time.

⭐ **This entry earned its existence before it passed.** Three runs failed at
10 of 20, 9 of 20 and 1 of 3, always at `stop`, and the cause was a real race in
`stop` itself rather than in the loop: [T-0602](supervise.md) carries it. A
single pass would have shipped that defect, and a retry would have published it.

⚠ **The script never retries and never continues past a failure**, so the number
it reports is how many iterations happened BEFORE the first one, which is the
only number this entry is about.

# T-0608: A detached container that reads `exited` with no launcher, seen twice and not reproduced

**Source:** Observed on 2026-09-09 while writing `experiments/250-negative-tests.sh`
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

⛔ **`podvm run -d` returned a container id, and one second later
`inspect` read `exited`, pid `0`, launcher pid `0`, exit code `0`,
for a payload that was `sleep 20`.** `logs` was empty. Whatever
happened, the record says the container ran and ended cleanly, and
it did not: a `sleep 20` that exits 0 in under a second is not the
payload's answer, and reporting it as one is the class of lie
[T-0604](supervise.md) exists to refuse.

## Premise

⚠ **SEEN TWICE AND NOT REPRODUCED, and that is written down rather
than smoothed into a diagnosis.** Both sightings were in the same
shell, against `public.ecr.aws/docker/library/alpine:3.20`, in a
fresh store, while `experiments/240-distro-sweep.sh` had all four
CPUs. Immediately afterwards:

- six consecutive `run -d` of the same image in a fresh store read
  `running` with a launcher pid: **0 bad of 6**;
- the same image and `ghcr.io/pkgforge-dev/archlinux:latest`, one
  each: both `running`;
- `experiments/230-lifecycle-loop.sh 3` passed **3 of 3**, and its
  own T-0604 clause reports `a launcher pid was recorded: yes`.

⛔ No cause is claimed. Two candidates are worth ruling out first
and neither has been: the readiness pipe's bound being reached under
load and reported as an exit rather than as "podvm does not know",
and `reconcile` running between `create` and `start` and writing a
terminal state over a container that had not started yet. ⚠ The
second would explain the `0` exit code, because `reconcile`'s
`dead` path leaves no code and this record had one.

## Approach

Reproduce it before changing anything.
1. a loop like `230-lifecycle-loop.sh`'s, but with the machine
   LOADED, because both sightings were under a full four-CPU build.
   ⚠ Load is the one condition the passing runs did not share;
2. record, for every iteration, the state, the pid, the launcher
   pid and the exit code, and stop on the first that is terminal
   within a second;
3. only then read `start` and `reconcile` against what the
   reproduction shows.
⛔ Do not "fix" the two candidates above on the strength of the
observation. M4's own race was found by enumerating the doors after
two written-down causes were both wrong
([T-0602](supervise.md)), and the entry keeps them for that reason.

## Decision

An entry rather than a note in `PROGRESS.md`, because an
intermittent that nobody is assigned to reproduce is one that is
rediscovered instead of fixed. ⚠ P1 and not P0: it was not seen in
any acceptance run, and the acceptance is what a release gates on.

## Prove

`./experiments/340-detached-stdio.sh 40` reproduced it on iteration 1 before the fix and reports 40 of 40 after it

## Done

**Done 2026-09-09.** `crates/podvm-supervise/src/launcher.rs`, and ⛔ **neither
of the two candidates above was it.** The entry refused to act on either and
that was right.

⭐ **THE LAUNCHER KEPT THE CALLER'S STDOUT.** `clone` copies every descriptor and
the launcher never execs, so it inherited the caller's stdin, stdout and stderr
and held them for the container's whole life. `podvm run -d` prints a container
id, so every caller captures it -- `id="$(podvm run -d ...)"` -- and command
substitution reads that pipe **to EOF**. The id was written in the first second;
the shell was released when the container ended.

⚠ **The sighting was accurate and the timing was the other way round.** "`run -d`
returned an id and one second later `inspect` read `exited`, pid 0, launcher pid
0, code 0" is what a caller sees when it is released at the END: the launcher had
already written the terminal record, and `pid` and `launcher_pid` are `None` on
that path by design.

Measured on 2026-09-09, before the fix:

```
$ id=$(podvm run -d --name subst <arch> /bin/sleep 20)   # took 20 s
$ ls -l /proc/<launcher>/fd/1
  l-wx------ 1 root root 64 ... 1 -> pipe:[93367]
```

⛔ And with stdout redirected to a file rather than captured, the same command
returned in under a second, which is why eight `run -d` in a row had looked fine:
**the instrument was the caller's own stdout**, and a shell that is not reading
the pipe never waits on it.

⚠ **THE LOAD WAS NOT THE CAUSE**, and the entry's warning about it was still
worth having: it reproduces on an idle machine in one iteration. Both sightings
happened to be under a four-CPU build because that is when somebody was watching.
`experiments/340-detached-stdio.sh` keeps the load anyway -- a scheduler under
pressure is where a handoff bug would hide, and removing the one condition the
entry named would be answering a different question.

The fix is one `dup2` triple after `setsid()`: the launcher's own 0, 1 and 2 go
to `/dev/null`, which is what daemonising has always meant. ⛔ It **refuses**
rather than shrugging where `/dev/null` cannot be opened, because a launcher that
cannot let go of the caller's stdio will hang it, and hanging silently is exactly
this defect.

⚠ **The payload's own stdio was never the problem** and is unchanged: `0` is
`/dev/null` and `1` and `2` are the container log, handed over before the chroot
([T-0605](#t-0605-the-container-log-and-what-podvm-may-claim-about-it)).

Prove, run 2026-09-09:

```
$ ./experiments/340-detached-stdio.sh 40
  before the fix: ⛔ iteration 1 REPRODUCED IT
  after:          detached starts that returned promptly and read running: 40 of 40
                  the longest `run -d` took: 1s, against a bound of 5s
```

# T-1318: `logs -f` follows a container's log, bounded like the rest

**Source:** issue 16, client beta testing 2026-09-22 (automated
  callers poll `logs` in a loop);
  `crates/podvm/src/parity.rs` (the `logs` row: `-f`
  not implemented, T-0605 covers capture-at-spawn only)
**Category:** supervise
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** done

## Problem

`logs` prints what the container has written; following it
(`-f`) has no entry, so automated callers poll in a loop.
T-0605 covers capture at spawn, not following.

## Premise

Read from the table: the row names the gap and the owner
of the other half.

## Approach

Follow by tailing the container's log file (inotify where
present, bounded poll where not), exiting on container exit
with a bounded wait, never an unbounded one. Out of scope:
log rotation (named if needed), changing capture.

## Decision

File-follow, not a new channel. The log file already
exists; the verb watches it.

## Prove

`podvm logs -f` on a payload writing lines on a timer
shows each line before the container exits, and `-f` exits
promptly after; `logs` without `-f` is byte-identical.
Close issue 16 (logs third) with a comment showing the run
and the bounded follow as the guard that stops recurrence.

## Done

**Done 2026-09-23.** File-follow, as decided: bounded poll (100 ms),
never inotify (no new crate, same behaviour everywhere).
`podvm-supervise::follow` prints what is there, then poll-appends from
the log file until the container reaches a terminal state (`Exited` or
`Dead`), with one bounded final drain (200 ms) so a write racing the
state flip is still shown. The poll step is its own function
(`follow_once`) so the unit test drives arrivals deterministically: this
crate spawns no thread anywhere, not even in tests (the
`nothing_on_the_spawn_path_can_spawn_a_thread` guard reads every source
line and failed the first version of the test, which used a writer
thread; the step function is the shape that respects it). A shrink
resets the offset; rotation stays out of scope. A container that never
exits follows forever, which is docker's answer too. The CLI parses
`-f`/`--follow` in one parser (a known flag is never accepted in one
position and lost in another), the parity row moves `None` to `Native`,
and the no-`-f` path is byte-for-byte the old code. One normalization to
record: `logs <container> --help` now prints usage like every other
position, instead of looking the container up; flags are honoured in any
position, extra positionals still ignored.

Both tests were watched fail first (`follow` and `parse_logs` missing),
then pass by name in the lane (`rust:1.98.1-bookworm` through host
podman 6.1.2), with full suites green beside them (cli 109, image 117,
extract 47 and 47 across its two targets, supervise 10, complete 50,
probe 97, 0 failed) and clippy `-D warnings`
clean. Two fixups landed between the proves and each was re-proven: a
stray `Write` import and a `question_mark` lint in the parser (both
caught by clippy), and the writer-thread test rewritten as `follow_once`
steps (caught by the spawn-path guard test).

Driven on the shipped binary in the same lane: a detached payload
writing `line1`..`line5` one second apart (one external `/bin/echo`
process per line, so no stdio buffering can hold a line back):

```
$ podvm logs -f timer1 > follow.out & sleep 3; grep -c line follow.out
2                                            # line1, line2 while running
$ wait; echo rc=$?; cat follow.out
rc=0
line1
line2
line3
line4
line5
$ podvm logs timer1 | cmp - follow.out && echo IDENTICAL
IDENTICAL
```

`-f` on an exited container prints and exits promptly; on a missing
container it errors at 125 like `logs` does. The guard that stops
recurrence is the bounded follow itself: callers watch one command that
ends, instead of polling in a loop.

# T-1335: The launcher forwards its own shutdown signals to the payload

**Source:** issue 16, client beta testing 2026-09-22 (Ctrl-C on
  foreground `run` has no specified behavior);
  `crates/podvm/src/parity.rs` (the `stop`/`kill` rows:
  pidfd reach, reparented grandchildren out of reach)
**Category:** supervise
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** done

## Problem

`stop`/`kill` reach the payload by pidfd, but the inverse,
the launcher itself receiving SIGINT/SIGTERM (Ctrl-C on a
foreground `podvm run`) has no specified behavior: no
entry says whether the payload is forwarded the signal or
dies with the launcher via `PR_SET_PDEATHSIG` (T-0603).

## Premise

Read from the table and the T-0603 shape: the mechanism for
one direction exists, the other is unspecified.

## Approach

Forward SIGINT/SIGTERM to the payload and wait boundedly
before exiting with the payload's disposition named; if the
design is PDEATHSIG-only, write the one parity-table
sentence naming it instead and close the question that way.
Out of scope: process groups beyond the payload, signal
masks callers set themselves.

## Decision

Forward-and-name, unless the drive shows PDEATHSIG already
covers it, in which case the sentence is the fix. The drive
showed `prctl` is called nowhere: no PDEATHSIG covers any
path. Both waiters forward: the foreground waiter names on
stderr, and the setsid'd launcher (whose stdio is /dev/null)
forwards silently with the signaled exit recorded as today
and the parity sentence documenting it. Operator order
2026-09-23: full launcher coverage, not the foreground path
alone.

## Prove

`./experiments/351-signal-forward.sh` exits 0. Close
issue 16 (signal third) with a comment showing the run
and the signalfd forward as the guard that stops
recurrence.

## Done

**Done 2026-09-23.** Forward-and-name, as decided: both waiters block
SIGINT/SIGTERM and read them from a signalfd beside the payload's
pidfd in one `ppoll`, so the signal never kills the waiter between
the decision and the forward, and a reused pid can never be signalled
instead (`pod_hostprobe::sys` `sigblock_shutdown`,
`signalfd_shutdown`; `crates/podvm-enter/src/lib.rs`
`Child::wait_forwarding`, naming each forward and a signaled end on
stderr; `crates/podvm-supervise/src/launcher.rs` `serve`, silent,
the signaled exit in the table record). A payload that traps or
ignores the signal is waited on, not killed. The parity rows carry
the forward (`crates/podvm/src/parity.rs`).

`experiments/351-signal-forward.sh` drives it on the shipped binary
in the lane (`rust:1.98.1-bookworm` through host podman 6.1.2),
`experiments/results/signal-forward.txt` carrying the run: foreground
TERM rc 143 with both lines, foreground INT rc 130 with both lines,
a clean exit quiet, a TERM to the launcher pid recorded as 143 by
`wait`, a TERM-ignoring payload waited on with the forward line, no
orphan left by any clause.

Two findings from the drive, both kept. The signalfd word is bit
index (n-1), not (1 << n): the first build named SIGQUIT+SIGSTKFLT,
blocked nothing, and every forwarded clause failed with empty
stderr; `the_shutdown_mask_holds_exactly_the_forwarded_pair` pins
16386 so the word cannot drift again. A `&`-backgrounded shell
without job control bequeaths an ignored SIGINT that `trap -`
cannot reset and SIG_IGN survives exec, so the harness runs every
foreground leg through `env --default-signal=INT,QUIT`: no line of
podvm resets a caller-bequeathed ignore, and whether a foreground
`run` should is follow-up work recorded here, not expanded here.

Units by exact name (`a_wait_status_becomes_dockers_exit_code`,
`forwarded_signals_are_named_and_unknown_ones_numbered`,
`the_shutdown_mask_holds_exactly_the_forwarded_pair`,
`nothing_on_the_spawn_path_can_spawn_a_thread`), the workspace
suites and clippy `-D warnings` green in the same lane run. The
guard that stops recurrence is the forward itself plus the 351
experiment: a waiter that stops forwarding fails its clauses.

# T-1421: A foreground run leaves a container record

**Source:** issue 84; `crates/podvm/src/run/parse.rs:192`;
  `crates/podvm/src/run/foreground.rs:270`;
  [T-0607](supervise.md)
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`run --name fg1` succeeds but `ps -a` is empty and `logs
fg1` finds no container. `--name` is accepted then
discarded. docker always records foreground runs as
`Exited`.

## Premise

Only `-d` creates a record (`run.rs:522`). The foreground
path holds the store and enters directly. The comment
confirms the design: a run of its own writes no record
(`run.rs:762`). `p.name` is consumed only on the detached
path.

## Approach

Record every foreground run with id, name, image, argv,
times, exit, and log. Where records cannot persist, refuse
`--name` by name instead of dropping it. Keep the ephemeral
`--rm` path deleting its record on exit.

## Decision

`--name` always names something `ps -a` can list. An
accepted flag is never silently dropped.

## Prove

`cargo test --workspace` exits 0. A tracked drive shows
`ps -a` listing `fg1` as `Exited 0` with `logs` printing
the output after a foreground `run --name fg1`.

## Correction

**Open 2026-09-30 (history).** Filed from issue 84. Related to T-1411 but
distinct: that entry owns detached lifecycle where chroot is denied;
this one owns foreground records on all hosts.

## Done

**Done 2026-09-30.** Every foreground run leaves a record with id,
name, image, argv, times, exit, and log; `--name` always names
something `ps -a` can list, and where records cannot persist the
flag is refused by name instead of dropped. The ephemeral `--rm`
path deletes its record on exit. `cargo test --workspace` exits 0.
`sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 16
lists the foreground run as Exited with its output in `logs`,
refuses the duplicate name, and shows `--rm` removing its record.
Related to T-1411 but distinct: that entry owns detached lifecycle
where chroot is denied; this one owns foreground records on all
hosts.

# T-1691: `stop -t 0` is a stop: no grace sends SIGKILL at once, and a failed bound is not a gone launcher

**Source:** issue 215; T-0602 (bounded waits); call C12
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`podvm stop -t 0 NAME` exits 0, prints NAME and leaves the
container running. `std` refuses a zero read timeout, so
`launcher::wait` returns an error before it sends WAIT, and
`wait_or_gone` maps each error to `LauncherGone`. SIGKILL never
comes.

## Premise

The zero comes from `crates/podvm/src/lifecycle/logs_stop.rs:208`,
which also multiplies with no overflow check. The early error
is in `launcher::wait`. The mapping is `wait_or_gone`, and the
comment above it is false. Both are `pod_supervise::launcher`
and `pod_supervise` of PodUtils/podcrates since podcrates@T-060. The
parity row is `crates/podvm/src/parity.rs:391`. The files of
`podvm-supervise` go to `pod-supervise` (podcrates podcrates@T-060), which
podcrates imports after this fix.

## Approach

1. `launcher::wait` returns a distinct error for "no launcher"
and for any other failure. `wait_or_gone` maps only the first
to `LauncherGone`; the other reaches the caller.
2. `stop` with a zero grace sends SIGKILL at once and reports
`killed` (call C12).
3. The grace parser is a function with `checked_mul` and unit
tests.
4. The parity row says what zero does.

## Decision

Call C12: `-t 0` means SIGKILL now, as docker does.

## Prove

`cargo test -p podvm -p podvm-supervise` exits 0 with tests for
the parser and the error mapping, and a drive on the cloud host:
a container that ignores SIGTERM ends at `podvm stop -t 0` and
`podvm ps` does not list it as running.

## Correction

**Filed 2026-11-09.** From issue 215 (an outside account). The triage of
2026-11-09 confirmed each claim at `2be31b2` and found the unchecked
multiply.

## Done

**Done 2026-11-09.** `launcher::wait` returns `WaitError::Gone` when no
launcher answers and `WaitError::Failed` for any other failure, and
`ended` maps only `Gone` to `Ended::LauncherGone`. A zero bound answers
`Ended::Bound` at once, so `stop -t 0` sends SIGKILL (call C12). The grace
parser `grace_ms` refuses an overflow, and the parity row says what zero
does. `cargo test -p podvm -p podvm-supervise` exits 0 with
`only_a_gone_launcher_is_a_stopped_container` and
`the_stop_grace_is_checked_seconds_and_zero_is_valid`; `cargo test
--workspace` exits 0. [The drive](../experiments/results/stop-t0.txt): a
detached alpine container that traps SIGTERM shows `Exited (137)` in
`podvm ps -a` after `podvm stop -t 0 t0`, which exits 0 and says the
container was killed at once. The image came from `mirror.gcr.io`,
because Docker Hub and `public.ecr.aws` answered HTTP 429 that day.

# T-1708: `stop`, `kill` and `rm -f` end the whole tree: the launcher is a subreaper

**Source:** issue 233; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), SPEC section 10.4, WP-C2 and
  WP-V4, its measurement M2 (`05-lifecycle.log`); G-12 and
  A-V-07 of PodUtils/podcrates#39; ask 2.3 of COORD-021 (issue
  227); T-0601 (the pidfd decision); T-1655 (`--timeout`);
  `src/launcher.rs` of `pod-supervise` (PodUtils/podcrates);
  `crates/podvm/src/parity.rs:289`
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`stop`, `kill` and `rm -f` signal the payload only. A
grandchild that the payload's exit orphaned, or that left the
process group, keeps running with parent pid 1. `rm -f` exits
0 and removes the record, so podvm forgets processes that
still run in the container's rootfs. M2: for `sh -c 'sleep N
& sleep N.5 & wait'` under `run -d`, both sleeps stay after
`stop -t 1` and after `rm -f`, 6 of 6 runs.

## Premise

The launcher signals one pid (`kill(pid, sig)` in
`src/launcher.rs` of `pod-supervise` v0.12.0, line 351).
T-0601 states that a pidfd does not contain descendants. The
`--init` row says that orphans reparent to the host init
(`crates/podvm/src/parity.rs:289`). No code sets
`PR_SET_CHILD_SUBREAPER` (grep of the tree and of podcrates
v0.12.0). The launcher is double-forked: its parent is pid 1
before a stop (M2). A subreaper needs no privilege.

## Approach

The launcher sets `PR_SET_CHILD_SUBREAPER` before it forks
the payload, so that orphans of the payload reparent to it. A
stop or a kill signals the payload, then each descendant that
the parent chain of `/proc/PID/stat` finds, and repeats until
the walk is empty or the grace ends; then it sends SIGKILL to
each process that stays, reaps and counts. The foreground run
does the same in podvm's own process. `rm -f` and `--rm`
remove the record when the count is zero; a process that
stays after SIGKILL past the bound makes the verb exit 125
and name the pids. `inspect` gives the count that the end
killed. The code is in `pod-supervise`: send it to podcrates
as a COORD issue (`AGENTS.md`, section 9, item 4; ask 2.3 of
COORD-021 asks podcrates for the same mechanism), take the
tag, and correct the ⛔ text of T-0601 and the `--init` note
in place.

## Decision

The launcher reaches each process of the container that did
not reparent above it. A process that reparents above it (to
an outer subreaper, or to init before the launcher existed)
stays out of reach, and `inspect` says so. No cgroup and no
PID namespace. Under a caller's subreaper the launcher is a
descendant of it, so the caller's walk also sees each
process.

## Prove

`cargo test -p podvm` exits 0 with tests
`stop_ends_an_orphaned_grandchild` and
`rm_force_counts_the_tree`. `sh experiments/429-tree-kill.sh`
exits 0 on the chroot-denying fixture and on the chroot rung:
the M2 payload under `run -d`, then `stop -t 1`, leaves no
process after 60 s, 20 of 20; the same with a grandchild
that calls `setsid`, after `rm -f`, and for a foreground run
that gets SIGTERM; under an outer subreaper the launcher's
parent is that subreaper.

## Correction

**Blocked 2026-11-11.** podcrates did the mechanism as its podcrates@T-162 (on its
`main`, after `v0.15.0`): the launcher is the subreaper of the payload's
tree, reaps each descendant, kills the tree at the end of the payload, and
writes the end with the count of residual processes (a number, or
`unknown`) and a wall bound (`set_wall`). `stop` and `kill` still signal
the payload; the tree kill follows its end. Blocker: no podcrates tag
carries podcrates@T-162. Clears when: a podcrates tag carries podcrates@T-162 (COORD-026,
issue 244). Then podvm moves the pins, gives `inspect` the count, makes
`rm -f` exit 125 with the pids where the count is not 0, and runs the
Prove.

**Filed 2026-11-10.** From issue 233 and the podrunner design review
(the ruling "podrunner", 2026-11-10), WP-C2: "tree kill, timeout,
rusage. The mechanism of section 10.4 placed in podvm's launcher". T-1655 keeps
the flags; this entry is the mechanism. `rm -f` reports a removed
container while its processes run, so the entry is P1.

## Done

**Done 2026-11-11.** The mechanism is `pod-supervise` of podcrates
`v0.16.0` (its podcrates@T-162; the record below). podvm's part: `inspect` gives the
end of the last run (`End.Reason`, `End.RunMs`, `End.UserMs`,
`End.SystemMs`, `End.MaxRssKiB`, `End.Residual`; a dash where no run
ended) and `.Contains` says what the launcher reaches; `rm -f` of a running
container stops it with SIGKILL through its launcher and, where processes
stayed, keeps the record and exits 125 with their count (call C15: the end
carries a count, not pids); a foreground `run` makes podvm the child
subreaper and kills the descendants that stay after the payload
(`crates/podvm-supervise/src/tree.rs`). The `--init` row and the text of
T-0601 say the same. Found on the way and fixed: a foreground `run` that
got SIGTERM ended with 143 and did not forward it, because the tee thread of
the log left SIGINT and SIGTERM unblocked (a mask is per thread); the pump
thread now blocks both, so the waiter's signalfd takes them. The Prove, on
the cloud host, 2026-11-11: `cargo test -p podvm` exits 0 with
`rm_of_a_stopped_record_with_a_residual_removes_it` and
`an_unknown_residual_reads_unknown_and_a_record_with_no_end_reads_dashes`
(the Prove's `stop_ends_an_orphaned_grandchild` and `rm_force_counts_the_tree`
are the crate's tests `a_grandchild_that_left_and_ignores_sigterm_is_killed`
and `with_no_kill_the_grandchild_is_counted`, and the drive below).
`PODVM_429_MIRROR=1 sh experiments/429-tree-kill.sh` exits 0
([the run](../experiments/results/tree-kill.txt)): on the namespace rung and
on the chroot-denied fixture (userland), the M2 payload under `run -d`
then `stop -t 1` leaves no process in 20 of 20 rounds, with
`.End.Residual` 0; a grandchild that called `setsid` is gone after
`rm -f`; a foreground run that gets SIGTERM leaves no process; under an
outer subreaper the launcher's parent is that subreaper. `--timeout` is
T-1655's flag over `set_wall`.

**Partial, 2026-11-11.** podcrates `v0.16.0` (`60a675c`,
PodUtils/podcrates#57) carries podcrates@T-162, and podvm takes it: the launcher of
each detached run is the child subreaper of the payload's tree
(`Settings::kill_tree`, on by default), kills each descendant at the end of
the payload, and writes `Container::end` (the reason, the times, the peak
resident set and `residual_processes`, a number or `unknown`).
`cargo test --workspace` exits 0 at the take. Remaining: `inspect` and
`ps` show the end and the count, `rm -f` exits 125 and names the pids
where the count is not 0, the foreground run does the same in podvm's own
process, `--timeout` over `set_wall` (T-1655), and the new experiment 429 under
`experiments/` (`429-tree-kill.sh`) with the runs of the Prove.

# T-1715: The container table keeps `-e` values in a file that other users can read

**Source:** issue 240; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), its R1:1.6 and SPEC section 9.1;
  `src/table.rs` of `pod-supervise` (PodUtils/podcrates);
  `crates/podvm-image/src/store.rs:253`; T-1696 (`--secret`, dropped 2026-11-11);
  T-1708 (the same COORD batch)
**Category:** supervise
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

`run -e K=V` and `--env-file` store each value in the
container record. The table writer uses the process umask,
and the store directory gets no mode, so with umask 022 the
table is 0644 in a 0755 directory. Each local user can read a
credential passed with `-e` while the record exists.
podrunner passes the egress token in the environment where
the hold is `none` (SPEC section 9.1).

## Premise

`write` in `src/table.rs` of `pod-supervise` v0.12.0 writes
a staging file with `std::fs::write` and renames it; no mode.
`crates/podvm-image/src/store.rs:253` calls `create_dir_all`
with no mode. The record keeps `env`, and `exec` reads the
volume table from it. The modes on disk are Inferred from the
source, not run.

## Approach

Create the store root and each container directory with mode
0700, and write the table and its staging file with mode 0600.
At store open, a root that group or other can read gets a
banner note and a doctor row. Send the `pod-supervise` part
to podcrates as a COORD issue, in one batch with T-1708; the
store root is podvm's.

## Decision

podvm keeps `env` in the record, because `exec` needs it, and
protects the file. A secret belongs in a file that the
caller stages (podrunner stages the credential of the
runner, PodUtils/podrunner#6), and the help of `-e` says so.

## Prove

`cargo test -p podvm` exits 0 with tests that the store root
is 0700 and the table is 0600 under umask 022.

## Correction

**Filed 2026-11-10.** From issue 240 and the podrunner design review
(the ruling "podrunner", 2026-11-10), R1:1.6: "A credential given with
`-e` or `--env-file` is therefore at rest in the table, in a file created
with default modes".

## Done

**Done 2026-11-11.** podcrates `v0.16.0` (`60a675c`, its podcrates@T-175,
PodUtils/podcrates#57) writes the table and each log 0600 from the first
byte and creates each container directory and `staging/` 0700; podvm takes
it with the pins of its 11 crates. The record below gives podvm's part.
Proof, on the cloud host, 2026-11-11: `cargo test -p podvm-supervise`
exits 0 with `the_record_of_an_env_value_is_private_under_umask_022` (a
record with an `-e` value under umask 022: the store root and the
container directory 0700, the table 0600), beside the tests of the record
below.

**Partial, 2026-11-11.** podvm's part is done. `Store::open` creates a
store root that does not exist with mode 0700 (`create_private_root` in
`crates/podvm-image/src/store.rs`); its parents keep the default mode, and
a root that exists keeps its mode. `Store::exposed_mode` gives the mode of
a root that group or other can reach; the banner of `run` names it
(`store_note` in `crates/podvm/src/run/acquire.rs`), and `podvm doctor`
gives the note row `store_mode` with the `chmod 700` that fixes it. The
help of `-e` says that the record keeps each value and that a secret
belongs in a file. Because the table, the logs and each container
directory are under the root, a root of mode 0700 keeps them from each
other user whatever their own modes. Proof, on the cloud host,
2026-11-11: `cargo test -p podvm-image --lib store::` exits 0 with
`a_new_store_root_is_private` and `an_existing_open_root_is_reported_and_kept`;
`cargo test -p podvm --bin podvm doctor::` exits 0 with
`the_store_mode_row_names_an_exposed_root`; `podvm doctor` with
`PODVM_STORE` on a new path prints `ok store_mode`, and on a directory of
mode 755 prints `note store_mode: ... is mode 755`. Remaining: the
`pod-supervise` part (the table and its staging file 0600, each container
directory 0700, the log 0600), asked of podcrates by COORD-026 (issue
244), and the test that the table is 0600 under umask 022, at the tag that
carries it.

# T-1720: `wait` gives no exit code for a payload that ends at once

**Source:** PodUtils/podrunner#49 (COORD-002 of podrunner, a message from
  a peer session); `crates/podvm/src/lifecycle/wait_rm_cp.rs`
**Category:** supervise
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

podrunner measured, with the released `v0.2.0-beta.2` on a
host with no root and no user namespace, that `podvm wait` of
a detached payload that ends at once exits 125 with "has no
exit code podvm measured: it is Up" in 4 of 5 runs. Its jobs
then read no exit code.

## Premise

`wait` of `pod-supervise` reads the record again when the
launcher is gone; a record that still says running there
gives no code. In `v0.12.0` (the crates of beta.2) the
launcher can go between its answer and its write of the
record (Inferred); since `v0.16.0` it writes the record
first (its podcrates@T-162).

## Approach

Settle in `wait`: where the answer has no code and the record
still says running, read the record again every 50 ms for up
to 10 s, until it has the code or is not running.

## Decision

`wait` never answers "Up" for a payload that ended; a bound
still ends with no code, never a made-up one.

## Prove

`cargo test -p podvm` exits 0 with
`wait_settles_a_record_that_still_says_running`, and ten
`run -d` of `sh -c 'exit 3'` then `wait` print 3.

## Done

**Done 2026-11-11.** `settle` in `crates/podvm/src/lifecycle/wait_rm_cp.rs`.
On the cloud host (the chroot-denied fixture of experiment 356, userland),
ten `run -d` of `sh -c 'exit 3'` then `wait` printed 3 in 10 of 10 runs,
with the current tree and with the released beta.2 (digest
`0af41a96233c7f9f3ed48b80ebb3271fe9eaa1295616ea0375625f8035baf6a9`): the
race does not show on this host, so the guard is the fix of the cause
that the Premise infers. `cargo test -p podvm` exits 0 with
`wait_settles_a_record_that_still_says_running`.

# T-1748: A foreground payload dies when its `podvm run` is killed with SIGKILL

**Source:** PodUtils/podcrates#124 (the asks of podswarm to podvm, a message from a
  peer session; PodUtils/podcrates#107 is its tracker), ask V-4
**Category:** supervise
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

When the `podvm` process of a foreground `run` dies by SIGKILL, its
payload can stay. A podswarm worker that kills `podvm` on a timeout must
leave no payload.

## Premise

The detached case has this since T-1732: the launcher of `run -d` sets
`Plan.die_with_parent` of `pod-enter` (`PR_SET_PDEATHSIG(SIGKILL)`), and
experiment 437 holds it. A foreground run sets no such flag on each
rung.

## Approach

Set `Plan.die_with_parent` for a foreground run on each rung where the
payload is a child of `podvm`, and name each rung where it is not (the
machine tier ends its emulator in its own way).

## Decision

Same mechanism as T-1732, with no second one.

## Prove

`sh experiments/437-payload-dies-with-launcher.sh` exits 0, with a clause
that SIGKILL of a foreground `podvm run IMAGE sleep 60` leaves no live
process of the payload within 5 s

## Done

**Done 2026-11-13.** `crates/podvm/src/run/foreground.rs` sets
`Plan.die_with_parent` for the payload of a foreground `run`, as the
launcher of `run -d` does (T-1732). The fork runs on the thread that waits
for the payload, so `PR_SET_PDEATHSIG` stays armed while the payload runs
(T-0603). `PODVM_437_IMAGE=mirror.gcr.io/library/busybox:musl
PODVM_437_DYNAMIC=mirror.gcr.io/library/debian:bookworm-slim sh
experiments/437-payload-dies-with-launcher.sh` exits 0
([the run](../experiments/results/payload-dies-with-launcher.txt)): clause 4
(the rung that the cloud host selects, a direct exec) and clause 5 (the
loader family of the userland rung on the chroot-denied fixture) give a
zombie payload within 5 s of SIGKILL to `podvm`. The release
`0.2.0-beta.6` exits 1 on the same script
([the run](../experiments/results/payload-dies-with-launcher-beta6.txt)):
the payload sleeps on in both clauses. The machine tier is not on this
path: its emulator ends with its own bound (T-1656).

