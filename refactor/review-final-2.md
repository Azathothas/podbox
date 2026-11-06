# Final usability review: `refactor/PLAN.md` and `refactor/INDEX.md`

A roleplay test. I read `refactor/PLAN.md` first and nothing else, then
`refactor/INDEX.md`, and then `refactor/recon-c.md` only when PLAN.md told me
to. Every number below is from a run I made, not from either document's claim.

All host measurements below are from this machine on 2026-10-01, Git Bash,
`C:\Users\AjamX\Downloads\podbox`.

---

## Verdict

**Not ready to dispatch from.** A 6-agent assignment can be built, but only
after reading `recon-c.md` in full, and one of the six assignments cannot be
proven on this host at all. Three defects are blocking: the task count is wrong
in three places at once, 14 of the 26 relative links do not resolve, and the
lane's failure classes are three, not the one the documents name.

---

## The roleplay walkthrough

### Step 1. What is podbox, and what is the goal?

Yes, and it is the strongest thing in the set. PLAN.md lines 12 to 19 give the
goal and the one-sentence purpose in the same paragraph:

> You are retiring 121 shell, Python, and PowerShell scripts from the `podbox`
> repository by re-expressing their measurements as native Rust.
>
> podbox is a Linux container runtime that runs payloads where namespaces,
> mounts, devices, and ownership changes can be denied.

That is a usable one-sentence answer, and it is the same answer INDEX.md
section 1 gives. No defect.

### Step 2. The repository's rules before touching anything

PLAN.md "The rules that will cost you if you break them" (lines 55 to 73) is a
**second, shorter** rule list, and it is the one I would actually read first.
What it told me, and what I would have obeyed:

| rule | present in PLAN.md | present in INDEX.md |
| --- | --- | --- |
| never call `wsl.exe` with a payload | yes, line 57, with the reason | yes, line 31 |
| run the record gate before and after | yes, line 61 | yes, line 29 |
| read every exit code without a pipe | yes, line 63 | yes, line 40 |
| land a plant with each check | yes, line 65 | yes, line 47 |
| do not narrow a failing run | yes, line 67 | yes, line 416 |
| never commit, no `git add` | yes, line 69 | yes, line 44 (attribution only) |
| ASCII, no em dash, fixed markers | yes, line 71 | yes, line 41 |
| dash for unknown, label estimates | yes, line 73 | yes, line 39 |
| every other repo is read only | **no** | yes, line 35 |
| use CodeGraph before a source search | **no** | yes, line 36 |
| keep credentials out of output | **no** | yes, line 45 |
| unique experiment number | **no** | yes, line 46 |
| bound network and process waits | **no** | yes, line 40 (second half) |

**The `wsl.exe` prohibition is stated in both, prominently, with the mechanism
and the remedy.** I would have obeyed it. That is the single best-executed part
of the set.

The gap is directional: PLAN.md is the file an agent is told to paste into a
fresh session, and it carries 8 of the 13 rules INDEX.md carries. Five rules
live only in the file PLAN.md calls "your orientation" and tells the reader to
read **last** (PLAN.md line 30). An agent that trusts PLAN.md's rule list and
skips INDEX.md is missing the read-only rule, the CodeGraph rule, and the
credentials rule. The risk is lowest for the CodeGraph rule and highest for the
read-only rule, because every task in this plan reads a `references/` repository.

### Step 3. Taking my first task, hop by hop

PLAN.md "How to take one task" (lines 224 to 245) is a 10-step procedure. It is
written, and the order is right. The problem is what it asks me to open.

| step | action | file reads | hops |
| --- | --- | --- | --- |
| 1 | find the row in `recon-c.md` section 1 | read `recon-c.md` lines 88 to 187, 100 rows | 1 |
| 2 | read section 2 for `depends_on`, `writes`, `conflicts_with` | read `recon-c.md` lines 212 to 313 | 2 |
| 3 | read the parent `T-R00N.md` in full | 1 file, unknown which | 3 |
| 4 | read `recon-b.md`, look up the wave | 1 file, unknown section | 4 |
| 5 | read the current source | unknown | 5 |

**Five file reads and five hops before I touch anything, and three of the five
require me to already know which section to read.** PLAN.md line 45 says "Section
1 is the task table, section 2 the dependencies", which helps. Nothing tells me
the byte offsets, so the practical navigation is a text search per task.

**`recon-c.md` is 975 lines and it is navigable, but not the way PLAN.md
implies.** I measured it: 100 task rows over 100 lines, one row per line, with
`Host` as the last column. `grep -n 'T-1531' recon-c.md` returns exactly two
lines, the table row and the dependency row. That is genuinely navigable, and I
will not pretend otherwise. But two facts make it a poor dispatch surface:

- The table is 100 rows and the documents agree it is 99, or 96, or 100.
  If the count is unstable, the table's own row count is not a safe thing to
  reason about.
- The proof column is in a **third** place, section 6, which is a separate
  100-row table. PLAN.md step 8 says "Run the task's proof" without telling me
  section 6 is where proofs live. A reader who reads only section 1 and section
  2 has ids, hosts, dependencies, and write sets, and **no proof command at
  all**. I only found them because I read the whole file.

**So yes: INDEX.md must carry the task table itself.** Not the full table. The
five columns a dispatcher needs, keyed by id, in one place: `id | Host |
writes | depends_on | proof`. Without `writes` there is no collision check; with
it, the 6-agent assignment in this review takes about ten minutes instead of a
full read of a 975-line file.

### Step 4. Running my first proof

**The command shape works, and my measurement contradicts the documents on the
point they call blocking.**

PLAN.md line 238 says, for the default lane, "`cargo test` is a lane". It never
gives a complete lane command. PLAN.md line 103 gives the wrapper,
`sh scripts/windows/run-in-base.sh JOB.sh`, and that works, but the reader has
to know that `JOB.sh` is a file they must write, the tree is copied to `/work`,
and the exit code is the job's. None of that is in PLAN.md. I read
`scripts/windows/run-in-base.sh` in full as PLAN.md line 109 directs, and its
header carries it.

The critical claim is PLAN.md lines 112 to 119:

> First, the bare job container is **not** bootstrapped. It has `cc` and `ssh`
> but **no `zig` and no `jq`**, so `cargo test` fails there on the `ring` C
> build step with `error occurred in cc-rs: command did not execute successfully`.

**The first two sentences are true. The consequence is false.** Measured in the
bare container, uid 0, no bootstrap:

| command | result |
| --- | --- |
| `cargo test -p podbox-probe --no-run` | exit 0, executable produced |
| `cargo test --workspace --target x86_64-unknown-linux-gnu --no-run` | exit 0, 19 executables produced |
| `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` | **exit 0, 48 passed, 0 failed** |
| `command -v zig` | MISSING |
| `command -v jq` | MISSING |

`ring v0.17.14` compiles with the stock `cc` that is present. The claim names
the crate and the error message and neither occurs. Zig is a linker for the
musl interposer (`crates/podbox-cli/build.rs:4` names `zig cc`), and
`build-interpose.sh` is a separate script the interpose tests do not require.
The interpose proof, which PLAN.md line 239 gives as its own command, **passes
unbootstrapped**, and that is the one command in the set most likely to need
zig.

**This matters more than being wrong.** The false claim costs the stated
condition, and the stated condition is one of only three `⛔` blocks in the lane
section, sitting directly under "Two things you must know about this lane, both
measured". An agent that trusts it prepends a network fetch and a several-minute
bootstrap to **every** job, including the ~19 of 99 tasks whose `Host` is
`native-windows` and which need no lane at all. On a lane that copies 343.2 MiB
across 12,048 entries, that is the single most expensive line in the brief and
it buys nothing.

### Step 5. A test fails. Real, or lane artifact?

**Not clear enough, because the documents name one artifact class and the lane
produces at least three.** Measured in one `cargo test --workspace` run on the
default lane:

| observation | class | would the documents catch it |
| --- | --- | --- |
| `test result: FAILED. 49 passed; 2 failed`, panic at `devices.rs:603`, `mknod(2)` succeeded because uid 0 | lane artifact, uid 0 | yes, PLAN.md lines 121 to 126, INDEX.md lines 162 to 167, both naming this exact line |
| `three_authenticated_ssh_sessions_share_one_node_connection ... FAILED`, panic at `crates/podbox-ssh/tests/common.rs:41` | **missing-binary artifact** | **no** |
| the same run's `rc=0` | **wrapper artifact**, the grep's exit status, not cargo's | **no** |

The middle row is the one that costs hours. `common.rs:41` is
`panic!("e2e needs {name:?} on PATH and it is absent")`, reached from
`mux_two_client.rs:1058-1060`, which requires `sshd`, `ssh` and `ssh-keygen`.
The container has `ssh` and not `sshd`. An agent that has read PLAN.md knows
about uid 0, decides the test asserts a degraded path, and starts rewriting a
test that is fine. The fix is one word in a job script, `openssh` in the
bootstrap, or `apt-get install openssh-server`. The panic message says which,
and neither document tells the reader that the panic text is the diagnostic.

The third row is subtler and I hit it myself. My first lane invocation ended
`rc=0` while the test suite inside it had failed, because the pipeline's last
command was `grep`. PLAN.md line 63 warns about pipes, and correctly, but it
warns about the *host* reading the exit code. The `⛔` block at line 121 does
not warn that the job script you write can swallow the failure the same way. An
agent writing a job script with `| tail` or `| grep` reintroduces, inside the
lane, the exact error the rule was written to prevent.

**Adequate on uid 0. Absent on the other two.** PLAN.md lines 121 to 126 do
stop the phantom `devices.rs` bug, and they name the file and the line. That is
good work and I want to be exact about it. It is one third of the answer.

### Step 6. Something the plan does not cover

PLAN.md lines 181 to 191 are honest and the closing sentence is the right one:

> **If your task is blocked on a decision, you are blocked.** Move to another task
> and record what you found. A decision you made up and wrote down is a defect,
> not progress.

And lines 264 to 268 give a real fallback: say which part, name the blocker,
hand back what runs.

**But the instruction is not actionable as written, for a specific reason.** It
names nine decisions, D-1 through D-9, and points at INDEX.md section 7 for each
with its options and costs. Only **D-1** has a default. INDEX.md line 231:

> ⛔ **This file assumes (c)** because it is the smallest change that preserves the
> check.

For the other eight, "you may not invent" plus "you are blocked" plus "move to
another task" strands an agent on a 99-task plan where **D-3 alone blocks 13
tasks** (INDEX.md line 249) and **D-5 blocks 4 of the 7 agents in Batch 4**
(INDEX.md lines 264 to 265). A fresh agent with no mandate cannot take 17 of
the plan's 99 tasks. That is not a defect in the agent; it is a missing
escalation path.

The instruction that would fix it is one sentence: *an open decision goes to
the operator with the options and the cost; do not resolve it inside a task.*
INDEX.md section 7 already holds the options and the costs. The documents
have assembled the packet and never say to send it.

**D-9 is worse, and it is a live example.** INDEX.md line 281: "The ninth
uid-0 task. Section 8 names nine tasks that must not be proven in a root
container. Eight are identified. The ninth needs an id assigned before Batch 1
dispatches." PLAN.md line 187 lists D-9 among the nine and moves on. A task that
does not have an id cannot be dispatched, tracked, or proven, and nothing tells
the reader whether to mint the id.

### Step 7. Six slots, six tasks, no shared file

**I could not do this from PLAN.md and INDEX.md.** I read all 975 lines of
`recon-c.md`. INDEX.md section 9 (lines 323 to 369) carries agent counts and
batch names, and section 8 (lines 286 to 321) carries a nine-row **shape** table
of host totals. Neither carries a single task's `writes` set, and `writes` is the
only thing that makes "no two agents write the same file" checkable. The
document even says so, twice, and points me away:

> **Use the per-row `Host` value, never this table's totals, when dispatching a
> single task.** (INDEX.md line 317)

> **Use the row's `Host`, not INDEX.md section 8's totals**, which are a shape
> and not a dispatch instruction. (PLAN.md lines 227 to 228)

Both are right, and together they mean the dispatch table does not exist in
either file. Two readings of the same failure to publish a dispatch table.

**INDEX.md would need to carry, per task: `id`, `Host`, `writes`, `depends_on`,
`proof`.** That is five columns, 100 rows, and it is the table that makes
sections 8 and 9 checkable instead of advisory. It also dissolves this whole
section of the review.

Here is the assignment I built, using `recon-c.md` section 2 for the write sets.
**It is a valid 6-agent assignment, and it is not the one the documents
recommend**, because it draws from the pool the documents do not assign to any
batch, and for the reason in the defect list.

| slot | task id | file it writes | proof command | can the proof run now? |
| --- | --- | --- | --- | --- |
| 1 | T-1581 | `TODO/INDEX.md` | `py scripts/todo-count.py --check` | **yes**, measured exit 0 |
| 2 | T-1551 | `crates/podbox-gate/src/grammar.rs` | `cargo test -p podbox-gate grammar::tests` | **no** |
| 3 | T-1511 | `experiments/371-validationos-stream.sh`, `experiments/370-windows-guest.sh` | `sh experiments/370-windows-guest.sh` and `371-validationos-stream.sh` each write their own numbered result | **no** |
| 4 | T-1510 | `experiments/README.md` | `sed -n '35p' experiments/README.md` no longer claims a passing KVM proof | **yes** |
| 5 | T-1541 | `experiments/163-ladder-drive.sh` | `py scripts/check-todo.py` exits 0, and `grep -rn '163-ladder-drive' TODO/ docs/ .github/` returns nothing | **yes** |
| 6 | T-1548 | `experiments/354-lifecycle-same-store.sh` | same shape as slot 5 | **yes** |

Every `depends_on` is a dash, so nothing is blocked. The six write sets are
pairwise disjoint, and slots 1 to 6 touch `TODO/INDEX.md` once, in one slot, so
the record-gate serialisation in INDEX.md line 320 is respected by
construction.

Slot 3 is honest about its shape: `recon-c.md` says its proof needs a Windows
guest, and this host has none. It is the weakest slot in the set. I kept it
because it is the only way to show what a **non-lane** proof costs, which is the
gap the `wslc` recommendation creates.

### Step 8. Can each proof actually run?

**Four of six, yes. Two of six, no.** Measured, not inferred:

| slot | proof | verdict |
| --- | --- | --- |
| 1 | `py scripts/todo-count.py --check` | runs native, exit 0. I ran it. |
| 2 | `cargo test -p podbox-gate grammar::tests` | **cannot run.** `crates/podbox-gate` does not exist; there are 9 workspace members. The proof names a binary that the task itself creates. This is a chicken-and-egg in the plan, not a host limit. |
| 3 | the two guest scripts | **cannot run.** Needs a Windows guest, and `recon-c.md` marks it `no` on the host column. |
| 4 | `sed -n '35p' experiments/README.md` | runs native. |
| 5 | gate plus grep | runs native. I ran the gate: exit 0. |
| 6 | gate plus grep | runs native. |

**On the uid 0 question: four of the six are unaffected.** Every native proof is
unaffected by the container's uid, which is worth saying because both documents
frame uid 0 as a global property of the environment. It is a property of one
lane, and the `native-windows` class does not enter it. None of the six is in
INDEX.md section 10's list of root-unsafe tasks, so the answer to "can this
proof run now" is governed by the host, not by the uid.

**On the bootstrap question: none of the six needs it.** Four are native, and
the two lane proofs I did run, the interpose suite and a workspace build, both
passed in the bare container. Had I believed PLAN.md lines 114 to 119 I would
have said all six need a bootstrap, and I would have been wrong about two of
them in the expensive direction.

### Step 9. Maximum safe fleet width

**The documents claim 19 peak, 6 sustainable, 1 for the gate crate. I can
confirm 6 and 1, and 19 is the number I cannot reconcile with anything.**

| claim | source | my verdict |
| --- | --- | --- |
| 19 peak, "wave-0 and wave-6 window only" | INDEX.md line 364, PLAN.md lines 217 to 220 | **plausible, unverifiable from the documents, and contradicted by the batch arithmetic.** See below. |
| 6 sustainable | INDEX.md line 365, `recon-c.md` section 5 | **plausible.** It is the number that survives the record-gate and workflow serialisations, and both documents state the mechanism, which I can check: 41 rows in `recon-c` section 2 carry a `TODO/` path in `writes`, and the workflow is one task. |
| 1 for the gate crate | INDEX.md line 366, `recon-c.md` Batch 2 | **confirmed.** Batch 2 is 8 tasks over one crate, one `Cargo.toml` row, and `plant.sh`, and every one of the 8 rows in `recon-c` section 2 shares `crates/podbox-gate/src/`. One agent is correct. |

**The 19 does not survive arithmetic.** Both documents assign 19 to the window
where the wave-0 and wave-6 work runs. My counts of those two groups are 14 and
11, which is 25, not 19, and they overlap by nothing, so 19 is not a subset of
them either. `recon-c.md` line 380 offers a reconciliation, that 19 is "8
wave-0 batches, 11 wave-6 deletions, and 9 wave-2 unit tests", and 8 plus 11
plus 9 is 28, of which 19 can be in flight. I cannot reconstruct which 19, and
INDEX.md does not name them. For an orchestrator the number is close to
useless: what matters is **which 19**, and neither file lists them. A peak that
cannot be enumerated is a slogan.

There is a larger point underneath the 19, and it is the finding I would put in
front of an operator first. `recon-c.md` covers **100** task ids, T-1501 to
T-1600. The five batches in both documents cover **78** of them. The
**22 uncovered ids are T-1515, T-1516, T-1550, and T-1582 through T-1600.** No
batch claims them, and no document says they are unassigned. They are decision
records and the two remaining deletions, and 19 of the 22 are `native-windows`,
so they are the cheapest work in the plan. Two of the six slots in my
assignment, 5 and 6, are `T-1541` and `T-1548`, which is in-batch; a reader
checking my work against the batch table will find them there. The rest of the
22 has no home.

### Step 10. The top 5 ways a dispatched agent gets it wrong

Ordered by expected cost.

**1. It takes a task outside a batch, or a task no batch claims.**
An agent follows PLAN.md line 232, "Take work by batch", and reaches for the
tasks that are cheapest and most obviously available. 22 of the 100 ids are in
no batch, and 19 of those are `native-windows` decision records that look like
free work. Nothing in either document lists them.
*Fails at:* PLAN.md line 232 and INDEX.md lines 323 to 359, which name five
batches and never state the coverage gap.
*Documented defect:* D-7 says 26 scripts are unassigned, which is a plan
question, not a task question. The 22 unassigned **tasks** are a different leak.

**2. It bootstraps the lane, or fails to.**
PLAN.md line 119: "**Run that bootstrap as the first line of any job that
compiles.**" That is an unconditional instruction resting on a false premise
that I measured. The agent pays a network fetch and several minutes per job for
nothing, on a lane that already copies 343.2 MiB across 12,048 entries.
*Fails at:* PLAN.md lines 112 to 119. Three of its four sentences are true. The
fourth, the one carrying the instruction, is false.

**3. It reads a red test as a real defect.**
The panic at `crates/podbox-ssh/tests/common.rs:41` is a missing `sshd`, not a
bug. An agent that has correctly learned the uid-0 rule will decide the test
asserts a degraded path and start rewriting a correct test. The same shape
applies to `jq` and `zig` being absent.
*Fails at:* PLAN.md lines 121 to 126 and INDEX.md lines 162 to 167, which name
exactly one artifact class. The documents never tell the reader to read the
panic text.

**4. It believes a green `rc=0` from the lane.**
`run-in-base.sh` reports the job's exit code. A job script that ends in
`| tail` or `| grep` reports the filter's. My own first run did exactly this and
printed `rc=0` over a failing suite.
*Fails at:* PLAN.md line 63, which warns about pipes on the host and nowhere
warns about pipes inside the job script. The lane section at lines 100 to 110
does not mention exit codes at all.

**5. It writes a `TODO/` file in parallel with another agent.**
41 of 99 tasks write a `TODO/` file, `todo-count.py` rewrites one Counts block
from all 203 rows, and `check_tree` then reads the whole tree. Two agents
writing two different `TODO/` files both leave `TODO/INDEX.md` needing a
refresh, and the second refresh reads the first agent's rows. The **failure is
silent**: the gate is green on each agent's own tree and wrong on the merge.
*Fails at:* PLAN.md line 215 and INDEX.md line 320, which state the
serialisation and state it **without naming the file that serialises**. The
single writer is `TODO/INDEX.md`, through `scripts/todo-count.py`, and neither
document says so. A reader who honours the rule still has to guess which writer
owns it.

---

## Contradictions

I checked the six the review brief named, plus what I found while measuring.

| # | question | PLAN.md | INDEX.md | agree? |
| --- | --- | --- | --- | --- |
| 1 | task count | 99 (line 12) | 99 (line 288) | **no, both wrong** |
| 2 | batch agent counts | 1, 8, 1, 4, 7 (lines 201 to 207) | 1, 8, 1, 4, 7 (lines 329 to 352) | yes |
| 3 | batch task counts | 3, 28, 8, 11, 14 | 3, 28, 8, 11, 14 | **no, sums to 64 of 99** |
| 4 | open decisions | nine (line 181) | nine, D-1 to D-9 (line 213) | yes |
| 5 | default lane | `run-in-base.sh` (line 103) | `run-in-base.sh` (line 112) | yes |
| 6 | bootstrap required | "run that bootstrap as the first line of any job that compiles" (line 119) | **never mentioned** | **no** |

### 1. The task count is unstable, and both documents are wrong

Three different numbers, in three files, for one table:

| source | count | ids |
| --- | --- | --- |
| `refactor/recon-c.md` line 5 and line 189 | 96, and 100 rows | T-1501 to T-1600 |
| `refactor/INDEX.md` line 288 | 99 | T-1501 to T-1599 |
| `refactor/PLAN.md` line 12 | 99 | not stated |

**Measured: `recon-c.md` section 1 has 100 task rows, T-1501 through T-1600,
with no gap.** INDEX.md line 288 says "**99 tasks, `T-1501` through `T-1599`**".
T-1600 is a row. INDEX.md's id range is wrong by one at the top end, and its
count is wrong by one against its own range, and both are wrong against the
table it names as authoritative. It also names T-1597 through T-1600 itself, as
"the four decision records the plan's own Decision fields require", in
`recon-c.md` line 190, and then excludes them from the range.

`recon-c.md` line 5 and line 50 say 96. Line 189 says 100 rows, of which 96 are
implementation and 4 are decision records. So `recon-c.md` contradicts itself
twice and INDEX.md resolves it by discarding four rows. The discarded four are
T-1597 to T-1600, and INDEX.md keeps T-1597 in a different place, at line 366,
inside the "how much work sits outside the serial core" figure. It cannot be in
two places.

Host split, measured from the `Host` column of all 100 rows:
`native-windows` 47, `linux-lane` 47, `both` 6. INDEX.md line 314 says an
independent review "recomputed them and got 47 native, 47 lane, 6 both" and
then keeps 46 native. My count is 47, which agrees with INDEX.md's own
recount and disagrees with INDEX.md's own table. The table's columns sum
correctly, 46 plus 47 plus 6 equals 99, so the total was arithmetically
self-consistent and factually one short.

### 2. Batch 1 is 28 in both documents and 48 in the task table

| batch | PLAN.md line | INDEX.md line | recon-c rows I assign |
| --- | --- | --- | --- |
| 0 | 3 | 3 | 3 |
| 1 | 28 | 28 | **48** |
| 2 | 8 | 8 | 8 |
| 3 | 11 | 11 | 11 |
| 4 | 14 | 14 | 14 |
| **sum** | **64 of 99** | **64 of 99** | **84, covering 78** |

INDEX.md line 334 states Batch 1's contents, and they do not add up:
"The 14 wave-0 corrections grouped by the file each writes, **the 11 wave-6
deletions**, the 10 recorded deletions, and 13 wave-2 unit tests." That is 14
plus 11 plus 10 plus 13, which is **48**, and it is written under a heading
that says 28. And the 11 wave-6 deletions are counted twice in that sentence,
because `recon-c.md` puts the 11 DELETE tasks at T-1570 to T-1580 and the 10
recorded deletions at T-1540 to T-1549, while INDEX.md line 302 puts the 10 at
T-1540 to T-1549 under **T-R001, wave 1**, not wave 6.

**The batch totals cover 78 of 100 ids. Twenty-two are in no batch**, listed in
step 9 above, and no document says so.

### 3. The bootstrap appears in one document and not the other

PLAN.md line 119 makes it mandatory. INDEX.md's lane section, lines 110 to
181, describes both lanes and never mentions `bootstrap-env.sh` or `zig` or
`jq`. An agent that reads INDEX.md as its orientation, which is what both files
tell it to do first, has no way to know. An agent that reads PLAN.md has a false
mandate. Neither is a good outcome, and the union of the two is not right
either.

### 4. Nine uid-0 tasks, eight named, and the ninth is unnameable

PLAN.md line 126 sends the reader to "INDEX.md section 10. Check it before you
trust a failure." INDEX.md section 10, lines 376 to 385, says nine, names
three bullets, of which the third is a category rather than ids, and closes "The
ninth needs an id under D-9." INDEX.md line 281 says the same. So the document
sends the reader to a list that states it is incomplete. The rule as written
cannot be followed, because the agent cannot know whether its own task is the
ninth.

### 5. A numbering citation inside PLAN.md points at a document that is not the one it names

PLAN.md line 170 to 171:

> ⛔ **There are seven waves, not six.** `T-R006.md` carries the twelve DELETE
> scripts and `PLAN.md` section 8 does not list it.

A reader in `refactor/` resolves `T-R006.md` and `PLAN.md` to their own
directory, where `PLAN.md` is **this brief**, whose section 8 does not exist.
Section 8 exists in `06-entries/PLAN.md`, the master plan. The sentence needs
`06-entries/` on both names. The identical confusion appears at PLAN.md line
212 and 213, where "`PLAN.md` section 6 names as blocking" points at a section
of a file the reader has not opened. Three instances, all the same defect.

### 6. Ledger counts, and whether that is a contradiction

`recon-c.md` line 764 gives T-1582's proof as returning "SPLIT 35, DELETE 12,
KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20". I measured the ledger: **SPLIT 35,
DELETE 12, KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20**, 121 rows. Correct.
INDEX.md line 198 gives the post-VC figures as 29, 35, 27, 19, 11 and says the
ledger "gives" 29, 35, 27, 19, 11 "after VC-2 and VC-3 are applied", which is
accurate, and the ledger is pre-VC. So the two reconcile and both documents say
so. **Not a contradiction.** T-1582 and T-1583 exist to close it, which is a
task in no batch.

### 7. Em dashes, and the house style

`docs/conventions/prose.md` line 51: "Use ASCII prose where possible. **Do not
use an em dash.**"

| file | em dashes |
| --- | --- |
| `refactor/INDEX.md` | **0** |
| `refactor/PLAN.md` | **5** |
| `refactor/recon-c.md` | 108 |
| `refactor/recon-a.md` | 23 |
| `refactor/06-entries/PLAN.md` | 12 |
| `refactor/review-index-2.md` | 56 |

All 5 in `refactor/PLAN.md` are in one place, lines 30 to 45, the "Start here"
list, where each numbered item is `**[link](path)** [em dash] description`. A
prose-convention breach in a document whose own rule list, at line 71, says
"**Prose: ASCII, no em dash.**" INDEX.md has none. The fix is five characters
and the set has a linter for it, `scripts/common/check-markers.sh`.

The `⛔` and `⭐` markers are used correctly and with meaning throughout both
files, at density rather than decoration. That is right and I want it on the
record.

### 8. Things a reader would reasonably misread

> **Peak fleet is 19 on a clean tree, during the wave-0 and wave-6 window
> only.** (PLAN.md line 217)

Reads as "dispatch 19 agents and 19 will be independent." There is no such
list, and my count of the two named waves is 25, so 19 is neither the total nor
a subset. See step 9.

> **Within Batch 1, 5 of the 8 agents write no `TODO/` file and run fully
> parallel; the rest queue.** (PLAN.md line 217)

"5 of the 8" then "the rest". This is ambiguous between 5 of 8 and 6 of 8, and
INDEX.md line 337 repeats "5 of the 8 agents run fully parallel". Which 5 is
not stated. `recon-c.md` line 655 names a different count, 13 tasks, in one
sentence. I cannot reconcile 5, 6, and 13.

> **Their proof is `cargo test --workspace` on the section 5.1 lane, which runs
> non-root, or on `ubuntu-latest` in CI at `gate.yml:199`.** (INDEX.md line 383)

This is the sentence an agent is most likely to act on, and it is wrong in the
way that costs the most. **The section 5.1 lane does not run non-root.** It runs
uid 0. I measured it: `UID=0`, on every job, including the one that produced the
workspace results in this review. The `wslc` section, 5.2, states uid 0 at line
163. Section 5.1 never says so, and this sentence hands the reader a remedy
that produces the identical failure. The only working option in that sentence
is CI. An agent that picks the lane option spends a full lane cycle discovering
what PLAN.md lines 121 to 126 already warned about, and the contradiction will
cost them the trust the brief was built to earn.

> **Batch 4 alone is a complete unit**: 14 tasks, 7 agents, no dependency on the
> gate crate, and it closes the missing OCI registry fixture. (PLAN.md line 210)

"Complete, shippable" while D-5 "blocks 4 of the 7 agents in Batch 4" per
INDEX.md line 264. Four of the seven cannot start, so the batch is not
dispatchable as advertised, and both documents say the batch is 7 agents. The
sentence is a recommendation to a reader who has not read section 7.

> **The plan is untracked, so no citation in it is held.** (INDEX.md line 203)

Correct, and worth more prominence than it gets. `git status --porcelain` shows
`?? refactor/`, confirmed. Every citation in both files, including the
line-number citations I was asked to check, resolves to nothing in CI.

---

## Link resolution

**The defect the brief predicted is real and it is the largest single defect in
the set: 14 relative links out of `refactor/` do not resolve, and the
repository's own checker reports all 14.**

Every link in both files that points **out** of `refactor/` is missing the `../`
prefix. Every link that points **within** `refactor/` resolves. The pattern is
total and consistent, so the fix is mechanical.

`refactor/PLAN.md`, 8 dead of 11 links:

| line | link target | resolves? | exists at `../`? |
| --- | --- | --- | --- |
| 38 | `TODO/RULES.md` | dead | yes |
| 38 | `TODO/INDEX.md` | dead | yes |
| 43 | `AGENTS.md` | dead | yes |
| 43 | `docs/conventions/prose.md` | dead | yes |
| 50 | `docs/methodology/gate.md` | dead | yes |
| 57 | `docs/containers.md` | dead | yes |
| 109 | `scripts/windows/run-in-base.sh` | dead | yes |

`refactor/INDEX.md`, 10 dead of 20 links:

| line | link target | resolves? | exists at `../`? |
| --- | --- | --- | --- |
| 31 | `docs/containers.md` | dead | yes |
| 43 | `docs/conventions/prose.md` | dead | yes |
| 48 | `scripts/plant.sh` | dead | yes |
| 49 | `docs/methodology/gate.md` | dead | yes |
| 51 | `AGENTS.md` | dead | yes |
| 51 | `TODO/RULES.md` | dead | yes |
| 52 | `docs/conventions/code.md` | dead | yes |
| 53 | `docs/conventions/forbidden-patterns.md` | dead | yes |
| 54 | `docs/methodology/authoring.md` | dead | yes |

**The repository already has the check that catches this and it already fails
on these two files.** `sh scripts/common/check-docs.sh` exits 1 and reports 118
broken links, of which:

- 100 are `refactor/recon-c.md`, the `TODO/<category>.md` links in the task
  table. INDEX.md line 204 records these as expected, because the tasks are not
  filed yet. Fine, and honestly recorded.
- 10 are `refactor/INDEX.md`.
- **8 are `refactor/PLAN.md`.**
- 2 are in older audit files, `01-audit/group-2.md` and `01-audit/group-9.md`.

The 18 in PLAN.md and INDEX.md are not recorded anywhere. They are the
repository's own rule, `docs/conventions/prose.md` line 36, "Link to that source
elsewhere", and the checker that owns it, and both documents fail both.

The cost is highest on PLAN.md line 43, `AGENTS.md`, which is the repository
rules file, and on PLAN.md line 38, `TODO/RULES.md`, which is the record rules
and the entry format. A fresh agent clicking those two, which are the two most
clicked links in the brief, gets a 404 in every renderer.

**What did resolve, so the review is not one-sided:** every link within
`refactor/` resolves, including `06-entries/PLAN.md`, `06-entries/T-R000.md`,
`06-entries/verdict-ledger.tsv`, `00-orientation/`, `07-verify/`, and all five
`recon-*.md` and both `review-index-*.md`. PLAN.md line 39's pointer into
`TODO/INDEX.md` at "roughly lines 42 to 57" is correct: I read lines 42 to 57
and it is the 15-row category-to-crate table.

**The anchors I was asked to spot-check are all correct.** `gate.yml:197` is the
bootstrap step, `:199` is `cargo test --workspace`, `:213` is the interpose
command. `Cargo.toml:20` is `exclude = ["crates/podbox-interpose"]`.
`TODO/INDEX.md:266` is the T-1423 row. `scripts/common/bootstrap-env.sh` exists.
`TODO/PROGRESS.md` and `TODO/RESUME.md` both exist. The citations are good; the
links built on them are not.

---

## Prioritised defects

| severity | file:line | what is wrong | the fix |
| --- | --- | --- | --- |
| **P0** | `refactor/INDEX.md:383` | "the section 5.1 lane, which runs non-root" is false. Measured `UID=0` on every 5.1 job. The stated remedy reproduces the failure it claims to fix. | Replace with CI as the only option, or state that both lanes run uid 0 and the repo lane is no different. |
| **P0** | `refactor/INDEX.md:288` | "99 tasks, T-1501 through T-1599" against a measured 100 rows through T-1600. Drops T-1600 and contradicts `recon-c.md:189`. | Reconcile to one number and say how it was derived. Recommend 100, and name the four decision records. |
| **P0** | `refactor/INDEX.md:334` | Batch 1's contents sum to 48 under a heading that says 28. Both documents carry 28. The 11 wave-6 deletions are also counted twice. | Recompute Batch 1 from the table, and state which ids it covers. |
| **P0** | `refactor/PLAN.md:112-119` | The bootstrap mandate rests on a false premise. `cargo test --workspace` and the interpose suite both pass in the bare container, exit 0, with `ring` compiling. | Delete the mandate. Keep the measured `zig`/`jq` absence, and record that it blocked nothing measured. |
| **P0** | `refactor/PLAN.md:38,43,50,57,109` and `refactor/INDEX.md:31,43,48,49,51,52,53,54` | 18 relative links out of `refactor/` do not resolve. The repo's own `check-docs.sh` exits 1 on them. | Add the `../` prefix. Mechanical, and a checker owns it. |
| **P1** | `refactor/PLAN.md:121-126`, `refactor/INDEX.md:162-167` | One lane artifact class is documented, three are measured. A missing-`sshd` panic reads as a bug. | Add the missing-binary class and name `crates/podbox-ssh/tests/common.rs:41`. Add the "read the panic text" instruction. |
| **P1** | both, batch section | 22 of 100 task ids are in no batch: T-1515, T-1516, T-1550, T-1582 to T-1600. | Add a Batch 5, or state that the 19 native decision records are taken opportunistically. |
| **P1** | `refactor/INDEX.md:8` | Section 8 is the "shape" table and carries no task. Two documents tell the dispatcher to use a per-row value the documents do not carry. | Add the dispatch table: `id \| Host \| writes \| depends_on \| proof`, 100 rows. |
| **P1** | `refactor/INDEX.md:364` | "19 concurrent agents" is not reconstructible from either document, and the two named waves total 25. | Name the 19 task ids, or replace the number with the two batch counts it is derived from. |
| **P1** | `refactor/INDEX.md:281`, `refactor/PLAN.md:187` | D-9: the ninth uid-0 task has no id, so the rule at section 10 cannot be applied to it. | Assign the id, or mark the rule as applying to 8 tasks. |
| **P1** | `refactor/PLAN.md:215`, `refactor/INDEX.md:320` | The `TODO/` serialisation is stated without naming the file that serialises, which is `TODO/INDEX.md` via `scripts/todo-count.py`. | Name it. An agent that honours the rule still has to guess the writer. |
| **P2** | `refactor/PLAN.md:181-191` | "You may not invent them" plus "you are blocked" plus "move to another task" has no escalation. D-3 and D-5 block 17 tasks. | Add: an open decision goes to the operator with the options and the cost from INDEX.md section 7. |
| **P2** | `refactor/PLAN.md:103`, `100-110` | The lane command is given as `JOB.sh` with no statement that the reader must author the job, that the tree lands at `/work`, or that the exit code is the job's. | Add three sentences, or a five-line example job. |
| **P2** | `refactor/PLAN.md:63` | The pipe rule covers reading the host's exit code, not writing a job script that swallows the suite's status. | Extend the rule to the job script. |
| **P2** | `refactor/PLAN.md:30,212,213` | `PLAN.md` and `T-R006.md` resolve to the brief, not the master plan. Three instances. | Qualify as `06-entries/PLAN.md`. |
| **P2** | `refactor/PLAN.md:30,34,38,43,45` | 5 em dashes, in the one place that states "no em dash". | Replace with a colon. |
| **P3** | `refactor/PLAN.md:55-73` vs `refactor/INDEX.md:25-49` | 5 of 13 rules live only in the file PLAN.md tells the reader to read last. | State in PLAN.md that the list is a subset, and link the rest. |
| **P3** | `refactor/PLAN.md:217`, `refactor/INDEX.md:337` | "5 of the 8 agents" is ambiguous against 6, and `recon-c.md:655` says 13. | Name the 5. |
| **P3** | `refactor/INDEX.md:210` vs `refactor/INDEX.md:264` | "Batch 4 alone is a complete, shippable unit", 7 agents, while D-5 blocks 4 of the 7. | Qualify: 3 of 7 until D-5 lands. |
| **P3** | `refactor/INDEX.md:314` | The table says 46 native; the recount in the same paragraph says 47; measured 47. | Change 46 to 47 in the table. |

---

## What I ran, and what it did not cover

Commands I ran and read the exit code of, unpiped:

- `sh scripts/windows/run-in-base.sh` with four authored jobs, to measure the
  lane. All four exited 0. Measured: uid 0; `zig` and `jq` absent; `cc`, `ssh`,
  `cargo`, `rustc`, `git`, `python3` present; `cargo test -p podbox-probe
  --no-run` green; `cargo test --workspace --no-run` green, 19 executables; the
  interpose suite 48 passed, 0 failed; `cargo test --workspace` with one
  `podbox-ssh` failure at `common.rs:41`.
- `py scripts/check-todo.py`, four times, before and after cleanup.
- `sh scripts/common/check-docs.sh`, exit 1, 118 broken links.
- `py scripts/todo-count.py --check`, exit 0.
- `awk` over `refactor/06-entries/verdict-ledger.tsv`, and `sed` and `grep` over
  `recon-c.md`, for every count in this report.
- Link resolution: every markdown link in PLAN.md and INDEX.md, tested with
  `[ -e ]` from `refactor/`, then re-tested at `../`.
- `git status --porcelain`, `?? refactor/`.

**Not covered.** I did not run a non-root lane, because I did not find one; the
`wslc` non-root question that `recon-c.md` line 868 leaves open is still open,
and `recon-c.md:781` gives nine root-unsafe tasks of which I measured none
directly. I did not run CI, so `gate.yml:199` and `:213` are verified as
citations, not as behaviours. I did not verify any of the line-number citations
into `crates/` and `TODO/` beyond the anchors listed above, which is a sample
and not a sweep. I did not read `recon-a.md`, `recon-b.md`, or
`recon-wslc.md` in full, and `recon-c.md` I read in full.

**Tree state.** No existing file was edited, nothing was staged, nothing was
committed. My four lane jobs were removed with `wsl-toolkit gc --job <id>
--apply`, and the record gate exits 0 as it did before I started. My scratch
job scripts were deleted. `git status --porcelain` shows `?? refactor/` only.
