# Review: is `refactor/INDEX.md` dispatchable?

Adversarial fitness review of [INDEX.md](INDEX.md) as a **dispatch document** for
several concurrent implementer agents. Default posture: it cannot be dispatched
from until proved. Every numeric claim below was recomputed from
[recon-c.md](recon-c.md) §1/§2, not read off INDEX.md. Every host and lane
claim was re-measured on this machine on 2026-10-01.

Scope of this pass: INDEX.md, recon-c.md §1–§9 in full, recon-b.md in full, the
seven `T-R00N.md` `Decision` fields, the ledger, and 14 source lines cited by
INDEX.md or its sources. **Not covered:** recon-a.md in full, PLAN.md in full,
`refactor/07-verify/`, and whether any individual task's clause list is
correct. No file other than this one was modified; nothing was staged or
committed.

## Verdict

**No. INDEX.md cannot be dispatched from today.** It is a good analyst's
summary with four load-bearing arithmetic errors and no per-agent work
package. An orchestrator with 6 slots can assign 6 agents to Batch 1's
`TODO/`-free tasks, and every one of them will have to guess something.

The four things that make it un-dispatchable, in order of cost:

1. **Six of its eight parent-range host splits are wrong**, and so are its
   headline counts. INDEX.md says 42 native / 7 both / 51 linux-lane.
   [recon-c.md](recon-c.md) §1, which INDEX.md names as the authority, says
   **47 native / 6 both / 47 linux-lane**. Only 2 of 8 rows survive
   verification. An agent told "T-R002 wave 2 is 2 native, 11 linux-lane" will
   pick a lane for 13 tasks against a table that is wrong for all 13.
2. **The `conflicts_with` column is not derivable from the `writes` column**,
   the method INDEX.md hands over. 5 of 100 rows contradict it — T-1508, T-1540,
   T-1579, T-1580, and T-1537/T-1573. The dispatcher will either split safe
   work (T-1537/T-1573 both write `crates/podbox-probe/tests/namespace.rs`)
   or stall on work that was never a conflict.
3. **Four decisions are missing.** INDEX.md names D-1 and D-2. recon-b has four
   `UNRESOLVED-DECISION` items (B-08, B-15, B-20, B-30) and the entries carry
   four more open ones. An agent dispatched from INDEX.md hits every one.
4. **The 9-task uid-0 list is not in the document.** INDEX.md says "Recon C §7
   item 8 lists them". §7 is an item 8 that is *struck through and replaced*,
   and it names a different nine, two of which do not need the lane. The list
   must be in the dispatch document.

## Batch verification table

| batch | claimed agents | claimed tasks | verified? | evidence |
| --- | --- | --- | --- | --- |
| 0 | 1 | 3 (`T-1581, T-1558, T-1551`) | **yes** | IDs exist; `writes` = `TODO/INDEX.md`, `gate.yml`+`nightly.yml`, `crates/podbox-gate/src/grammar.rs`; all three `depends_on` blank. Count 3 correct. |
| 1 | 8 | 19 | **no** | Prose names 8 groups + 11 deletions + 9 wave-2 = 28 tasks, not 19, and names 10 groups not 8. Computed max file-disjoint width is 8, so the **agent count is right and the task count is wrong by 9**. Also 1 of 13 `writes` groups is unresolvable (`crates/podbox-cli/src/windows/`). |
| 2 | 1 | 8 (`T-1552..T-1557, T-1577, T-1578`) | **yes, id list exact** | `T-1552..T-1557` + `T-1577` + `T-1578` is precisely recon-c §5 Batch 2. `T-1578`'s blank `depends_on` is a real row defect: it is a gate-crate member and its own `conflicts_with` names T-1554, yet it is predecessorless. |
| 3 | 4 | 11 | **yes, with a caveat** | The 11 (`T-1559..T-1569`) map 1:1 to the three crates: buildstate 2, release 7, podvm 2. Split of 1/3/1 is feasible — `Cargo.toml` is named by exactly one Batch-3 task (`T-1559`). But INDEX.md's own parenthetical "release (per file, **minus the two** that need Batch 0)" contradicts recon-c §5, which excludes `T-1562` and `T-1563`; `T-1563` is excluded by `depends_on` (`T-1508`), **not** by Batch 0, and `T-1508` is in Batch 1. |
| 4 | 7 | 10 | **partly** | Zero `writes` intersection with Batch 2 — the claim **holds**. But recon-c §5's Batch 4 is **14 tasks in 9 agent groups**, not 10/7. INDEX.md dropped `T-1537`, `T-1572` and `T-1573`, and `T-1573` is the one that collides with `T-1537`. |

Agent counts **survive**: 1, 8, 1, 4, 7. They were re-derived rather than
trusted — Batch 1's 8 is the true max of the conflict graph, and Batch 4's
resources (1 image, 1 fixture, 1 cli-shape, 3 cli, 1 probe, 1 enter,
3 serial) sum to 7. Task counts **do not survive**: 3 ✓, 19 ✗ (28), 8 ✓, 11 ✓,
10 ✗ (14).

## 1. Task granularity and locatability

**Locating a row costs 2–4 hops from INDEX.md and fails structurally for one
class of task.** `Host` is a per-*task* field in recon-c §1; the parent-range
table in INDEX.md:152-161 is per-*range*. A dispatched agent holding a task id
must: open INDEX.md, find the range (3–5 guesses, because T-1540..T-1549 is
orphaned between the `T-R003` and `T-R004` rows), open recon-c.md, scroll to
§1 (a 975-line file, the table starts at line 86), read the row, then scroll to
§2 (line 212) for `depends_on`/`writes`/`conflicts_with`, and again to §6
(line 704) for the proof command. **recon-c is navigable for a human** — ids
are contiguous, §1 is one table, §2 is one table, and the rows are ordered. It
is not a dispatch surface, because INDEX.md reproduces none of it.

**The orphaned range is the concrete failure.** recon-c §1 parents `T-1540`
.. `T-1550` on eleven different `TODO/` files (`gate`, `podssh`, `probe`,
`deps`, `interpose`, `supervise`, `packaging`) and their items are
`experiments/*.sh` deletions — wave-6 DELETE work. INDEX.md's table attributes
`T-1550` to T-R004 and `T-1570..T-1580` to T-R006, so **`T-1540` .. `T-1549`
appear in no row of INDEX.md's table at all.** An agent told "your batch is
Batch 1" cannot confirm which parent owns its task.

**The 6-task dispatch test, and what each agent had to invent.** All six are
in Batch 1, from four different ranges, pairwise file-disjoint — the graph
supports it. What each still lacks:

| agent | task | range | `writes` | what it must invent |
| --- | --- | --- | --- | --- |
| A1 | `T-1508` | T-R000 | `experiments/157-lock-inheritance-prove.sh` | the exact `mod tests`/sed line (recon-c §1 only says "re-anchor the `157` sed"), and it is **one of the 9 uid-0 tasks**, which INDEX.md does not list |
| A2 | `T-1510` | T-R000 | `experiments/README.md` | that its proof is `sed -n '35p'` — only in recon-c §6 |
| A3 | `T-1511` | T-R000 | `370-` and `371-` scripts | that it needs a **Windows guest** and cannot run natively — recon-c §6 says "**no**", INDEX.md's table says T-R000 is "12 native, 2 both" and A3 is one of the "2 both" without saying which |
| A4 | `T-1519` | T-R002 | `crates/podbox-cli/src/system.rs` | the three `abi` exit codes, and that `system.rs` is Unix-only at the type level |
| A5 | `T-1541` | **none** | `experiments/163-ladder-drive.sh` | its own parent entry, from `T-R006.md` |
| A6 | `T-1548` | **none** | `experiments/354-lifecycle-same-store.sh` | its own parent entry |

Four of six were dispatchable as file assignments. **All six were missing the
proof command**, and two were missing the parent that carries the clause list.
An agent is given "Read the `T-R00N.md` that owns your task" (INDEX.md:237) and
for A5/A6 there is no way to get there from INDEX.md.

**Locatability of `writes` is itself broken for one task.** `T-1529` writes
`` `crates/podbox-cli/src/windows/` `` — a directory. recon-c's own §1 item
(`T-1573` ← `T-1537`) and the Batch 4 "one file each" claim assume file-level
granularity. A `windows/` directory owned by one agent collides with every
future `windows/*.rs` in the plan, and `cargo metadata` reports the real target
list, which recon-c did not read.

## 2. Decision completeness — six decisions missing

INDEX.md:118 says "**Two** decisions that must be made before any code". That
is not true. The full set an agent dispatched from INDEX.md can hit:

| id | what | where it comes from | blocks | in INDEX.md? |
| --- | --- | --- | --- | --- |
| D-1 | the two size ceilings | INDEX.md:123-134; recon-b B-03/B-04 | T-1552, T-1554, T-1557, T-1562, T-1595 | yes, defaults to (c) |
| D-2 | the 26 unassigned scripts | INDEX.md:136-138; recon-b B-22 | 26 scripts, whole scope | yes, undecided |
| **D-3** | wave 2's plant mechanism | recon-b B-08 `UNRESOLVED-DECISION` | T-1517..T-1529, T-1578, and Batch 1's whole wave-2 group | **no** |
| **D-4** | wave 1's `Prove` for the two deletions | recon-b B-15 | T-1515, T-1516 | **no** |
| **D-5** | `podbox-cli` test shape, per file | recon-b B-20; T-R003:38-44; T-1532 | T-1533, T-1535, T-1536, T-1538 — 4 Batch-4 agents | **no** |
| **D-6** | who writes `experiments/perf-ceilings.tsv` and the three result files check 30 reads | recon-b B-30 | check 30; no entry names the owner | **no** |
| **D-7** | the exact split of 32 tasks (4 per range) into 4 decision records | INDEX.md:165 itself | Batch 0's "3 tasks unblock 60" | **partial, incoherent** |
| **D-8** | whether `T-R006` is authorised work at all | recon-c §7 item 4 | 11 of 100 tasks | **no** |

Two more are decided *in the entry* but block a dispatched agent with no
statement in INDEX.md, so they do not count as missing decisions — they are
missing instructions: **T-1550** (the `10`/`20`/`130` three-way, `T-R005:37`
T-R004-adjacent) and **T-1571** (clause 7's fate under T-0215).

**D-3 is the one that blocks Batch 1 today.** recon-b: "Do not start wave 2
first." `plant.sh:51` `FILES` names only three `crates/` paths
(`podbox-supervise/src/lib.rs`, `podbox-cli/src/parity.rs`,
`podbox-cli/src/run.rs`) — verified — and it refuses to start on a dirty tree
(`plant.sh:74-79`, verified). So a wave-2 agent must either widen `FILES`, which
makes an ordinary edit to `crates/podbox-cli/src/images.rs` SKIP the whole
harness at `gate.yml:56`, or invent a second script. INDEX.md mentions
`plant.sh:74` only to argue for `run-in-base.sh` over `wslc`; it never says the
choice is open.

**D-5 is the one that blocks Batch 4.** INDEX.md:203-204 calls Batch 4 "a
complete, shippable unit" and its recommended single batch, while four of its
agents are unbuildable until the per-file shape is recorded.

**D-7, the text INDEX.md already needs.** The decision-record count is
inconsistent in three places at once:

- INDEX.md:165 — "They are T-1551 (D-1), T-1558 (the `110` hoist), and two
  scope records." T-1551 is *Extract the row and entry grammars into one
  module* (`crates/podbox-gate/src/grammar.rs`), host `both`, and recon-c
  `:1551` gives it a different `conflicts_with` list from T-1552. It is the
  grammar extraction, not a ceiling decision. It is also in Batch 0, so the
  sentence implies T-1551 is a record with no code.
- INDEX.md:176 — Batch 0 is "3 tasks: `T-1581`, `T-1558`, `T-1551`", then calls
  them "decisions and a count refresh". One is a count refresh, one is a
  workflow repoint, one is new code.
- INDEX.md:213 — "The serial core is 11 tasks". Batch 0 has 3 and Batch 2 has
  8, so recon-c's sentence is about Batch 2 alone. Under the heading "Fleet
  width", where INDEX.md:208 has just claimed 19 peak agents "reachable only
  during the wave-0 and wave-6 window", a reader takes 11 to include Batch 0.

**D-8, and it is load-bearing for the count.** recon-c §7 item 4: `T-R006.md`
is untracked, is not in `PLAN.md` §8, and *"the operator may consider it a
verification finding rather than a seventh plan entry. **If it is not
authorised, 11 of the 100 tasks (T-1540 to T-1550) are out of scope and the
fleet is 89 tasks.**" INDEX.md finding 1 promotes T-R006 to canonical ("**There
are seven waves, not six**") with no mention that its authorisation is open.
`git ls-files refactor` returns 0 lines, so the whole plan is untracked and
nothing in it is held by the gate.

### Exact text INDEX.md needs for each missing decision

Append this block after INDEX.md:138 and change "Two decisions" to "Eight
decisions":

> **D-3, wave 2's plant mechanism (recon-b B-08). Unresolved. Do not start
> T-1517 to T-1529 until it is recorded.** `FILES` at `scripts/plant.sh:51` is
> the set the harness owns end to end: it copies each entry before a case,
> restores each after, and refuses to start when any entry is dirty
> (`scripts/plant.sh:74-79`). It names only three `crates/` paths and none that
> wave 2 writes tests into. Adding the new source files widens the guard, so an
> ordinary edit to `crates/podbox-cli/src/images.rs` SKIPs the whole harness at
> `gate.yml:56`. **Recommended: a second script, `scripts/plant-integration.sh`,
> with its own `FILES` and the same "did the mutation land" hash, transitional
> into `podbox-plant` in wave 4.** A plant lands with its test in the same
> change (`docs/conventions/code.md:32`), so this decision is a prerequisite of
> Batch 1's wave-2 group, not of T-1578.

> **D-4, wave 1's `Prove` (recon-b B-15). Unresolved.** T-R001:56 proves the
> two deletions with `cargo test --workspace`, which reaches no deleted clause.
> Choose: accept the clause-to-test table in T-R001 as the closure record and
> say so in the `Prove` line, or delete `220` only after its clauses have
> plants. **Recommended replacement text:** `Prove: cargo test -p podbox-extract
> --lib` and `cargo test -p podbox-ssh` exit 0, and `py scripts/check-todo.py`
> exits 0 with no bare citation of `388-interactive-shell.sh` or
> `220-extract-path-safety.sh` remaining, verified with `git grep -n
> '388-interactive-shell\|220-extract-path-safety' -- TODO/ crates/
> experiments/`. This does not re-establish the deleted clauses.

> **D-5, `podbox-cli` test shape, per file (recon-b B-20, T-R003:38-44).
> Unresolved. T-1532 is the decision task and four Batch-4 agents wait on it.**
> Black-box where the assertion is about what the operator sees; a new `[lib]`
> where it is about internal state. `curated_refusals.rs`,
> `detached_stdio.rs` and `qol.rs` drive `CARGO_BIN_EXE_podbox` and assert exit
> codes and output fields. `store_gates.rs` asserts which records a store
> operation skipped, which is not observable from an exit code, and needs
> `[lib]`. **Adding `[lib]` is not silent** — it is a new artefact in the
> release profile, and `T-R004:45` forbids changing the release surface in this
> plan — so `store_gates.rs` waits for its own entry unless the operator
> authorises it here. Recon-c §7 item 8 requires this record to carry the uid-0
> lane fact as well, because a fixture that asserts a degradation path cannot be
> proved in a root container.

> **D-6, who writes the perf readings (recon-b B-30). Unresolved.** Check 30
> at `scripts/check-todo.py:1376-1379` names `experiments/perf-ceilings.tsv`
> and three result files, and no entry names which binary writes them after
> `360-perf-harness.sh` and `build-interpose.sh` are ported. Name one writer per
> path in whichever wave owns `360`, or the gate reads a file nobody updates.

> **D-7, which four tasks are the decision records (INDEX.md:165 is
> inconsistent).** State the id list. On recon-c's own text the four are
> **T-1532** (B-20/D-5, "Choose and record the `podbox-cli` test shape"),
> **T-1550** (the `10`/`20`/`130` three-way, "Record the `10`/`20`/`130`
> three-way decision"), **T-1570** ("Settle where `394-ssh-package.sh` and
> `397-exported-build.py` go"), **T-1571** ("Decide
> `153-store-lock-race.sh` clause 7's fate under T-0215") — plus **T-1551** for
> D-1, which is why the set is five and not four. Then correct Batch 0: it
> contains T-1581 (a count refresh), T-1558 (a workflow repoint) and T-1551
> (new code in `crates/podbox-gate/src/grammar.rs`), not three decisions.

> **D-8, is `T-R006.md` authorised work (recon-c §7 item 4)? Unresolved, and
> it moves 11 tasks.** `T-R006.md` is untracked (`git ls-files refactor` → 0)
> and is not in `PLAN.md` §8. If it is a verification finding rather than a
> seventh entry, `T-1540` to `T-1550` are out of scope and the fleet is 89
> tasks, not 100. INDEX.md finding 1 states there are seven waves without
> recording that this is a decision. **No Batch-1 agent may be dispatched to a
> `T-15xx` DELETE task until this is answered**, because those tasks are
> 11 of Batch 1's 28.

## 3. Collision safety

Batch 2's id list is exact and Batch 4 shares no file with it — both verified by
comparing the `writes` columns directly, and recon-c's own `conflicts_with`
agrees except for one row, `T-1573`, which names `T-1578` with no `writes`
overlap. That is finding 3 below.

**Finding 11's "22 tasks" does not verify. There are 41.** Computed from
recon-c §2 `writes`: 41 tasks name a `TODO/*.md` file, and one more (`T-1540`)
writes the `TODO/` **directory**, which conflicts with all of them. recon-c
§4.6 states 22; §3 repeats 22; INDEX.md:108 repeats 22. The rule is right and
the number is wrong by 19. **The consequence is the opposite of INDEX.md's
reassurance**: the serialisation is not 22 tasks queued behind one writer, it
is **one 41-task chain plus one gate-crate task that also writes three `TODO/`
files (`T-1554`)**. INDEX.md:183-185 says "the tasks that write a `TODO/` file
serialise behind one writer. Of the 8 agents, roughly 6 write no `TODO/` file
and run fully parallel." Verified: the true figure is **5 of 8** —

| Batch-1 group | `writes` | `TODO/`? |
| --- | --- | --- |
| A `TODO/cli.md` | T-1501, T-1502, T-1503 | yes |
| B `TODO/gate.md` | T-1504, T-1509, T-1512 | yes |
| C `TODO/image.md` | T-1507, T-1513, T-1514 | yes |
| D `TODO/probe.md` | T-1505 | yes |
| E `TODO/interpose.md` | T-1506 | yes |
| F `experiments/157-…` | T-1508 | no |
| G `experiments/README.md` | T-1510 | no |
| H `experiments/370-`+`371-` | T-1511 | no |
| I-1 `experiments/163-` | T-1541 | no |
| I-2 `experiments/40-` | T-1544 | no |
| I-3 `experiments/50-` | T-1545 | no |
| I-4 `experiments/158-` | T-1546 | no |
| I-5 `experiments/354-` | T-1548 | no |
| I-6 `experiments/350-`+`TODO/gate.md` | T-1547 | yes |
| I-7 `383-`+`TODO/podssh.md` | T-1542 | yes |
| I-8 `401-`+`TODO/probe.md` | T-1543 | yes |
| I-9 `156-`+**`TODO/`** | T-1540 | yes, whole tree |
| I-10 `386-`+`TODO/milestones.md`+`TODO/podssh.md` | T-1549 | yes |
| I-11 `TODO/packaging.md` | T-1550 | yes |
| J-1..J-4 `crates/*/src/*.rs`, no `TODO/` | T-1520, T-1522, T-1524, T-1525, T-1526 | no |
| J-5 `images.rs`+`TODO/cli.md` | T-1518 | yes |
| J-6 `images.rs` | T-1517 | no, but depends on J-5 |
| J-7 `system.rs` | T-1519 | no |
| J-8 `lifecycle.rs`+`windows/` | T-1529 | no |
| J-9 `write.rs` | T-1523 | no |
| J-10 `probe_cache.rs` | T-1526 (dup of J-4) | no |

The eight-agent maximum is a **lower bound set by 6 non-`TODO/` agents**, not
an upper bound. INDEX.md therefore under-states its own safe width here, which
is the one place in this review where INDEX.md is conservative rather than
optimistic. But the 41-vs-22 error is not cosmetic: it moves the "sustainable
concurrency is 6" claim (§4) and it changes which tasks Batch 0 must precede.

**Fleet widths: 19 peak, 6 sustainable, floor 1 — consistent with recon-c §3
and §5, with one qualification.** The three numbers are restated faithfully.
Two of the three are reachable, and the mechanism differs from what INDEX.md
implies: 19 is not "19 agents, 19 tasks", it is a **file-partition of 28
tasks** (8 wave-0 groups + 11 deletions + 9 wave-2 tests), and the floor of 1
lasts for 8 tasks, not "all of T-1552" (INDEX.md:211). INDEX.md's "89 of 100
tasks, 79 can run 6-wide, 10 need a decision first" reproduces recon-c §5
exactly and is not independently checkable from §1/§2; I did not verify it.

## 4. Host-class correctness — the counts are wrong

`Host` in recon-c §1 is a per-task field. INDEX.md converted it into a
per-range table and got six of eight rows wrong. Recomputed from recon-c §1:

| INDEX.md row | INDEX.md claims | recon-c §1 actual | verdict |
| --- | --- | --- | --- |
| T-R000 `T-1501..T-1514` | 12 native, 2 both | 12 native, 2 both | **OK** |
| T-R001 `T-1515, T-1516` | 2 linux-lane | 2 linux-lane | **OK** |
| T-R002 `T-1517..T-1529` | 2 native, 11 linux-lane | **13 linux-lane** | wrong |
| T-R003 `T-1530..T-1539` | 1 native, 7 both, 2 linux-lane | **9 linux-lane, 1 both** | wrong |
| T-R004 `T-1550..T-1558` | 1 native, 8 both | **6 linux-lane, 1 both, 2 native** | wrong |
| T-R005 `T-1559..T-1569` | 1 native, 8 both, 2 linux-lane | **11 linux-lane** | wrong |
| T-R006 `T-1570..T-1580` | 11 native | **6 linux-lane, 2 both, 3 native** | wrong |
| decisions `T-1581..T-1600` | 20 native | 20 native | **OK** |
| **total** | **42 native, 7 both, 51 linux-lane** | **47 native, 6 both, 47 linux-lane** | **wrong** |

The pattern is consistent and diagnosable: the `both` counts were inflated and
the `linux-lane` counts moved into it. Every task recon-c marks `both` is a
`cargo check` plus a linked run — there are 6. INDEX.md claims 7 in total and
up to 8 in one range.

**What a dispatched agent does with `Host`: nothing, because the proving
command is not given.** INDEX.md:220 says "`py scripts/check-todo.py` and
`cargo check` are native. `cargo test` is the `wslc` command above." That is a
three-way mapping, not a per-task one. For `linux-lane` it does supply the
`wslc` invocation at INDEX.md:62-65 with the `MSYS_NO_PATHCONV=1` warning and
the full-path warning, which is the single most operationally useful thing in
the document. For `native-windows` it supplies one command (`check-todo.py`) for
51 tasks whose proofs are `grep`, `sed`, `awk` and `ls`. For `both` it says
nothing about which two, or which runs where.

**The `wslc` default target is a trap INDEX.md does not mention.**
`.cargo/config.toml` sets `target = "x86_64-unknown-linux-musl"`, so a bare
`cargo test` inside the lane **builds but does not run** (recon-c §9 says so
explicitly). An agent that runs INDEX.md:220's "`cargo test` is the `wslc`
command" with no `--target` gets a silent no-op suite and reads it as green.
INDEX.md gives `--target x86_64-unknown-linux-gnu` exactly once, at line 231,
in the `podbox-interpose` note.

**The `both` class is under-determined for the lane question.**
[INDEX.md:229-232](INDEX.md) says `cargo test --workspace` "does not reach
`crates/podbox-interpose`" and gives the separate command. Verified at
`gate.yml:213` — the line reads
`cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target
x86_64-unknown-linux-gnu`, matching INDEX.md. Good. But nothing tells an agent
that `T-1524` and `T-1525`, both `linux-lane`, need that command and not
`cargo test --workspace`. The `Host` class is per-crate-orthogonal; the lane
rule is per-crate.

**Verdict on §4: the exact proving command is not given per host class, and
the class itself is miscounted.** An agent is left inferring it from
recon-c §6, which is the only place it exists.

## 5. The 9-task uid-0 list

**recon-c carries two different nines and neither is in INDEX.md.**

- [recon-c.md:777-783](recon-c.md) §6: "The 9 are **T-1517, T-1518, T-1520,
  T-1521, T-1529, T-1534, T-1539, T-1587, and T-1578**."
- [recon-c.md:858-871](recon-c.md) §7 item 8: "**T-1517, T-1518, T-1520,
  T-1521, T-1529, T-1587**, and the T-1534 and T-1539 of Batch 4" — i.e. the
  same **eight**, and item 8 is the one **struck through and replaced** in
  place (§7 item 8 opens `~~…~~` and the replacement below it is the live
  text).

**INDEX.md:222-227 points at the struck one** ("Recon C §7 item 8 lists
them") **and adds a ninth by description** ("the tasks targeting
`crates/podbox-complete/src/devices.rs`, plus any task whose subject is a
refusal or a `skipped:` path"). Measured against recon-c §1:

| id | recon-c §1 `Host` | item | in the `wslc`-excluded set? |
| --- | --- | --- | --- |
| T-1517 | linux-lane | `rmi` on a held image | yes |
| T-1518 | linux-lane | `prune` exit code | yes |
| T-1520 | linux-lane | NSS check D | yes |
| T-1521 | linux-lane | check A's live form | yes |
| T-1529 | linux-lane | `362-windows-refusal.sh` | yes |
| T-1534 | linux-lane | `acquisition.rs` | yes |
| T-1539 | linux-lane | `store_digest.rs` | yes |
| **T-1578** | **both** | extend `plant.sh` `FILES` | **no — provenance defect** |
| **T-1587** | **native-windows** | record that two scripts stay shell | **no — no proof, no lane** |

**`T-1587` needs no lane at all.** Its `writes` are `TODO/gate.md` and
`TODO/image.md` and its recon-c §6 proof is "`py scripts/check-todo.py` exits
0" — native. It is in the uid-0 list only because recon-c §7 item 8 justifies
it as "a `skipped:` path" **subject**, a record about a skipped path, not a test
that asserts one. **`T-1578`'s inclusion is indefensible on its own evidence**:
`T-1578` depends on nothing, its proof is "`scripts/plant.sh` `FILES` names every
new test's file, and `podbox-plant` still exits 0", and `plant.sh` SKIPs rather
than fails when the gate is red (`plant.sh:113-117`) — a SKIP is not a uid-0
failure. If `T-1578` is a typo for **`T-1579`**, which is in Batch 2 and runs
plants, the list becomes exactly 8 and recon-c's two lists agree.

**The ninth task is missing from both.** INDEX.md says the set is "the tasks
targeting `crates/podbox-complete/src/devices.rs`, plus any task whose subject
is a refusal or a `skipped:` path". No task in recon-c §1 targets
`devices.rs`. The two tests that fail under uid 0 are `devices.rs:603` and
`:629`, and the task that exercises them is **`T-1523`** (write.rs)? No — the
readable candidate is **`T-1521`**'s sibling: recon-c §6 gives no `devices.rs`
row at all. **The ninth is `T-1521` or the `devices.rs` tests are simply not
scheduled**, and I cannot settle it from these documents. INDEX.md's own
sentence is the best evidence: "the tasks targeting
`crates/podbox-complete/src/devices.rs`" — that task does not exist in the
100-row table, so either the table is missing a row or the sentence is a
category description that resolves to nothing.

**This is a blocker.** 8–9 tasks out of 100 must not be proved in the lane
INDEX.md hands every agent, the set is unstable across two sources, and two
members are demonstrably wrong. An agent will get this wrong.

### The text INDEX.md needs

Replace INDEX.md:222-227 with:

> ⛔ **Nine tasks must not use the `wslc` lane.** `wslc run` executes as uid 0;
> `mknod(2)` succeeds for root, so a test that asserts the degraded shim path
> never reaches it. The list, from recon-c §7 item 8 and recon-c §6, is:
> **T-1517, T-1518, T-1520, T-1521, T-1529, T-1534, T-1539, and one task in the
> `T-R002` group that targets `crates/podbox-complete/src/devices.rs` — that
> row is missing from recon-c §1 and is an operator decision, see D-9.**
> **T-1587 and T-1578 are NOT in the lane-excluded set**, despite recon-c §6
> listing them: `T-1587` writes `TODO/gate.md` and `TODO/image.md` and is proved
> by `py scripts/check-todo.py`, which is native; `T-1578` writes
> `scripts/plant.sh` and `crates/podbox-gate/src/plant.rs` and its proof is a
> `FILES` content check. Neither links a binary. Their proofs for the affected
> assertions are **T-1579** (one plant per new test, each with a saved red run)
> and **T-1534**/`T-1539`. **D-9: confirm the ninth id against the tree before
> dispatch.** The escape for any task in the set is
> `sh scripts/windows/run-in-base.sh JOB.sh`, or `ubuntu-latest` in CI at
> `gate.yml:199`.

## 6. Ordering safety

**Batch 0 lands, Batch 1 starts. Trace one agent: `T-1550`.** It is native,
it writes `TODO/packaging.md`, and its recon-c §6 proof is "`grep -rn
'10-build-target-image' TODO/ scripts/ experiments/` shows the decision, and
`py scripts/check-todo.py` exits 0". Its `conflicts_with` names T-1562 and the
release tasks. `T-1550`'s decision is the `10`/`20`/`130` three-way, and
`110-bloat-delta.sh` is one of those three subjects. Its `conflicts_with` does
**not** name `T-1581` or `T-1584`, the two tasks that own `TODO/INDEX.md`, so a
mechanical dispatcher can start it **concurrently with the very task that
refreshes the counts its own edit invalidates**. And its decision is D-1-adjacent
and unrecorded in INDEX.md.

**Against finding 2 (the two ceilings).** A Batch-1 agent does not trip it,
because both ceiling files survive until wave 4 and nothing in Batch 1 touches
them. But **the ceiling is a live reading, not a constant**:
`scripts/check-todo.py:228-229` reads `CEILING_SCRIPT =
"experiments/110-bloat-delta.sh"` and matches `^CEILING_BYTES=(\d+)$` against
it — verified, both files' mtimes are 2026-09-30 19:38 and untouched. So the
only exposure in Batch 1 is a *task that edits a ceiling file*, and there is
none. **Finding 2 is correctly deferred to Batch 2, and Batch 0 is a
sufficient guard for it.**

**Against finding 4 (the `110` inversion).** `T-1562` is a **Batch-3** task, so
the batch structure is safe — *provided the hoist happens*. INDEX.md:108's
finding and Batch 0's `T-1558` implement hoist option 1 (recon-c §4.2
recommendation), and `T-1562` is never in a batch with a reader. **But the
hoist changes the plan's wave assignment, and nothing in INDEX.md says so
explicitly**: T-1558's recon-c §1 item is "Repoint every `.github/workflows/`
path the port moved, in both workflows" — a workflow repoint — and Batch 0's
prose calls it a decision. An agent executing T-1558 as written repoints CI and
does not touch `110`. **The hoist has no task id.** This is defect 1.

**Against finding 5 (`nightly.yml`).** `T-1558` writes both workflows — verified
in recon-c §2 (`writes` = `.github/workflows/gate.yml`,
`.github/workflows/nightly.yml`) and confirmed at `nightly.yml:84,103,112,179`.
INDEX.md finding 5 surfaces it. **The residual risk is the undocumented
fourth script**: INDEX.md:102 names `build-interpose.sh:84`,
`nightly-smoke.sh:103`, `package-ssh.sh:112` and `release-notes.sh:179`, but
recon-b B-21 adds a fifth subject with a caller in wave 4 —
`scripts/dev.sh:222` runs `sh experiments/394-ssh-package.sh`, and `394` is
one of the 26 unassigned scripts (D-2). So `T-1555` (which deletes `dev.sh`) and
D-2 interact, and no batch or task names it.

**Ordering is otherwise sound**: Batch 0's three tasks have no predecessors and
no `TODO/INDEX.md` writer other than `T-1581` itself, and Batch 4 is provably
independent of Batch 2 (0 `writes` intersections, and 13 of its 14 tasks are
transitively self-contained; `T-1572` alone needs `T-1523`, a Batch-1 task that
INDEX.md does not flag).

## 7. Failure modes INDEX.md does not cover

**1. A Batch-1 agent runs its proof in `wslc` and contaminates the checkout.**
INDEX.md:80-86 argues for `run-in-base.sh` for "anything that mutates" and
says "**Use `run-in-base.sh` for anything that mutates, and `wslc` for read-only
proofs**". That is the right rule, and it is unenforceable as written because
**INDEX.md never says which tasks mutate.** From recon-c §6: `T-1508`
(`sh experiments/157-lock-inheritance-prove.sh` — a `sed` that mutates source),
`T-1511` (writes two numbered result files), `T-1515`, `T-1516`, every `T-154x`
deletion, `T-1579` (writes `experiments/results/`). That is 16 of Batch 1's 28
tasks, and 4 of the 8 `TODO/`-free agents. An agent told "`wslc` for read-only
proofs" will bind-mount the real checkout and mutate it — which is exactly what
INDEX.md says a bind mount would do, and then hands the agent a lane it forbade
three lines earlier. Worse, `wslc` bind-mounts at `/work`, so a mutation lands
in the operator's tree with no copy and no `/out`.

**2. Two `TODO/`-writing agents land at once and `todo-count.py` races.** The
mechanism is right (recon-c §4.6, INDEX.md finding 11) and the number is wrong
(41, not 22 — §3 above). **The mitigation is missing entirely.** INDEX.md says
the tasks "serialise behind one writer" and never says who the writer is, what
"serialise" means operationally (a work queue? a lock file? one agent holding
all `TODO/` tasks?), or what an agent does when it finds the tree red on
arrival. `scripts/todo-count.py` rewrites one `Counts` block in
`TODO/INDEX.md` from all 203 rows and moves a status in both the row and the
entry with `--set`; `scripts/check-todo.py:1543-1552` recomputes and asserts
it. Two agents = lost update, and the loser sees a red gate that is not its
fault.

**3. A plant mutates the tree and `plant.sh:74` refuses.** Verified:
`scripts/plant.sh:74-79` runs `git diff --quiet -- $FILES` and
`git diff --cached -- $FILES` and **exits 2** on either, after
`[ -x "$GATE" ] || exit 2` at `:45`. Two consequences INDEX.md does not
carry:

- **The guard is per-`FILES`, and `FILES` is 23 entries wide.** `FILES` already
  contains `TODO/INDEX.md`, `TODO/PROGRESS.md`, `TODO/probe.md`, `TODO/enter.md`,
  `TODO/RULES.md`, `.github/workflows/gate.yml`, `experiments/110-bloat-delta.sh`
  and `scripts/dev.sh` (recon-c §4.3). So the "concurrent `TODO/` edits"
  serialisation and the plant dirty-tree guard are **the same guard**, and any
  agent holding a `TODO/` task blocks every plant for as long as its change is
  uncommitted. With 41 `TODO/`-writing tasks in one chain, that is a
  train-wide stall that INDEX.md never mentions.
- **The plant runs red-gated.** `plant.sh:113-117` SKIPs with exit 2 when the
  baseline gate is red. INDEX.md:111-115 says this was fixed and the gate now
  exits 0 — **verified: `py scripts/check-todo.py` → `GATE_EXIT=0`**, "203
  rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done". That claim
  holds. But a single `TODO/`-writing agent that leaves the gate red turns every
  concurrent plant into a SKIP, and a SKIP read as a pass is the failure mode.

**4. How does an agent stop safely?** INDEX.md has no stop protocol. Its only
guidance is "Record what you could not do and what still owes, in the task
entry" and "Do not narrow a failing run until it passes". An agent in a 28-task
shared tree has no instruction on: whether to `git stash` or revert, whether to
leave a half-ported file that breaks `cargo check` for the next agent, whether
a failed port should delete the Python it was replacing (B-27: `plant.sh:45`
requires `check-todo.py` to be executable, so deleting it before `podbox-plant`
exists makes `gate.yml:56` skip silently), or how to report a blocked task when
it cannot reach the operator. Given that `git` operations are a shared mutable
resource and `T-1552` owns `Cargo.toml` and `crates/podbox-gate/src/` — 40
`conflicts_with` partners — "stop safely" is undefined.

**5. `refactor/` is untracked, so no task's evidence is held.** `git ls-files
refactor` → 0. INDEX.md finding 12 records this as "a records decision, not
code" and moves on. For dispatch it means **no agent can branch, and no
`Prove` in the plan is checked by `check_tree`** — the gate never reads
`refactor/`, so the entire dispatch document is outside the one gate the plan
tries to replace.

## Prioritised defect list

| severity | location | what is wrong | the fix |
| --- | --- | --- | --- |
| **blocker** | INDEX.md:152-161, and :52-55 | Six of eight parent-range host splits are wrong and the headline 42/7/51 is 47/6/47. A dispatched agent routes lanes from a table that is wrong for 88 of 100 tasks. | Delete the host-split column from the range table, or regenerate it from recon-c §1 and add a one-line check: the eight column-sums must equal 47/6/47. Per-task `Host` is already in recon-c §1 and should be the only copy. |
| **blocker** | INDEX.md:222-227 | The 9-task uid-0 list is not in the document, points at a struck-through recon-c item, and two of its members (`T-1587`, `T-1578`) do not need the lane. The described ninth resolves to a task that does not exist in recon-c §1. | Insert the §5 block above verbatim, including the explicit `T-1587`/`T-1578` exclusions and a `D-9` for the ninth id. |
| **blocker** | INDEX.md:108, :183-185 | "22 tasks must serialise" is 41 tasks (42 counting `T-1540`, which writes the whole `TODO/` directory). The consequence is understated and the Batch-1 free-lane count is 5, not "roughly 6". | Replace 22 with 41 and add the per-group `TODO/`-yes/no table from §3 above, so an orchestrator can see which of the 8 agents are free before dispatch. Name the writer and the queue. |
| **blocker** | INDEX.md:118-138 | Six decisions are missing: D-3 (wave-2 plant mechanism, blocks Batch 1's 13 tasks), D-4 (wave-1 `Prove`, blocks T-1515/T-1516), D-5 (`podbox-cli` test shape, blocks 4 of Batch 4's 7 agents), D-6 (perf-ceiling writer, blocks check 30), D-7 (which four tasks are the decision records — INDEX.md:165 is self-contradictory), D-8 (is `T-R006` authorised — moves 11 tasks). | Add the seven quoted blocks from §2. Change "Two decisions" to "Eight". Reconcile the T-1551 sentence: it is the grammar extraction, not a record. |
| **blocker** | INDEX.md:181-185 | Batch 1's "19 tasks" is 28 (8 groups + 11 deletions + 9 wave-2), the prose names 10 groups where it says 8, and `T-1540..T-1549` appear in no parent-range row because INDEX.md attributes T-R006 to `T-1570..T-1580`. An agent cannot find its own parent entry. | Print the Batch-1 task list with each task's `writes` and parent, add a `T-1540 .. T-1550` row to the range table, and correct 19 → 28. |
| **major** | INDEX.md:198-204 | Batch 4's "10 tasks" is recon-c's 14 in 9 groups. `T-1537` and `T-1573` are omitted and they both write `crates/podbox-probe/tests/namespace.rs` — the one place where a `conflicts_with` entry has no `writes` overlap, so a dispatcher reading only `writes` gives two agents the same file. | Print all 14 ids. Add to INDEX.md: "`T-1537` and `T-1573` are mutually exclusive; they were split because T-1573 depends on T-1528 while T-1537 does not. Never assign both." |
| **major** | INDEX.md:80-86, :220 | "Use `wslc` for read-only proofs" is unenforceable: no task is marked mutating. 16 of Batch 1's 28 mutate (`T-1508`, `T-1511`, `T-1515`, `T-1516`, all `T-154x` deletions, `T-1579`), including 4 of the 8 `TODO/`-free agents. An agent will bind-mount the real checkout and mutate it. | Add a `mutates: yes/no` column to the Batch-1 table in INDEX.md, with the `run-in-base.sh` command for the `yes` rows. Say explicitly that `wslc` bind-mounts at `/work` and has no copy. |
| **major** | INDEX.md:220-232 | The proving command is not given per host class. Worse, `.cargo/config.toml` sets `target = "x86_64-unknown-linux-musl"`, so a bare `cargo test` in the lane **builds and does not run** — a silent green. `--target x86_64-unknown-linux-gnu` appears once, in the interpose note. | Add: "every `cargo test` in the `wslc` lane requires `--target x86_64-unknown-linux-gnu`." Add the interpose caveat to `T-1524` and `T-1525`, which are `linux-lane` and need `gate.yml:213`'s form, not `--workspace`. |
| **major** | INDEX.md:176-178 | Batch 0's "3 tasks unblock 60 of the remaining 92" credits T-1551 with D-1. T-1551 is *Extract the row and entry grammars into one module* — new code in `crates/podbox-gate/src/grammar.rs`. The decision record for D-1 has no id. | Restate Batch 0 as: T-1581 (count refresh), T-1558 (workflow repoint **plus the `110` hoist**), T-1551 (new code). Give D-1 its own record id, or state that the record goes in `T-R004`'s `Decision` field with no `TODO/` task. |
| **major** | INDEX.md:108, Batch 0 | The `110` hoist — the fix for finding 4, the single thing gating 9 `TODO/deps.md` `Prove` lines and check 17 — **has no task id**. `T-1558`'s item is a workflow repoint. An agent executing Batch 0 as written repoints CI and leaves the inversion in place. | Either extend `T-1558`'s item text to include the hoist, or add `T-1601`. Say which. |
| **major** | INDEX.md (no section) | No stop protocol for an agent in a 28-task shared tree. Nothing on `git stash` vs revert, on leaving a half-ported file that breaks `cargo check` for the next agent, on B-27 (`plant.sh:45` requires `check-todo.py` executable, so deleting it before `podbox-plant` exists makes `gate.yml:56` skip silently), or on reporting a blocked task. | Add a "If you must stop" section: leave the tree green, never delete a file whose replacement does not yet exist, record the block in the task entry and in `TODO/RESUME.md`, and do not revert another agent's path. |
| **major** | INDEX.md:234-246 | "What an implementor does next" is 6 steps that stop at "take work from your batch" and "land with a plant". There is no work-package template: no required output shape, no branch or commit rule, no statement that agents share one checkout (so each thinks it has its own). | Add a per-agent work package: task id, parent entry, `writes` set, `depends_on`, proof command, the decisions that apply, the `TODO/`-writer rule, the lane rule, the stop rule. This is the single highest-leverage addition. |
| **minor** | INDEX.md:194-196 | Batch 3's parenthetical "release (per file, minus the two that need Batch 0)" contradicts recon-c §5, which excludes `T-1562` and `T-1563`. `T-1563` is excluded by `depends_on` (`T-1508`), and `T-1508` is in **Batch 1**, not Batch 0. | "release: 5 agents, one per file, minus `T-1562` (the `110` inversion) and `T-1563` (waits on `T-1508`, a Batch-1 task)." |
| **minor** | INDEX.md:152-161 | `T-1540 .. T-1549` are attributed to no parent row. recon-c §1 parents them on eleven different `TODO/` files and they are DELETE work. | Add the row: "T-R006 wave 6, the eleven `experiments/` deletions, T-1540 .. T-1550 \| 11 \| 3 native, 2 both, 6 linux-lane". Note the overlap with INDEX.md's existing T-R006 row `T-1570..T-1580`. |
| **minor** | INDEX.md:198-204, :212-215 | "The serial core is 11 tasks" is recon-c's Batch 2 alone. Under "Fleet width", where the previous line claims 19 peak agents "reachable only during the wave-0 and wave-6 window", a reader takes 11 to include Batch 0. | "The serial core is Batch 2: 8 tasks. With Batch 0 it is 11." And "gating 74 of the remaining 89" → the 89 excludes the 11. |
| **minor** | INDEX.md:211 | "The floor is 1 for all of T-1552" — the floor of 1 lasts for the whole of Batch 2, 8 tasks. | "The floor is 1 for all of Batch 2 (8 tasks)." |
| **minor** | INDEX.md:102, finding 5 | `nightly.yml` has 4 named subjects, but recon-b B-21 adds a fifth coupling INDEX.md never surfaces: `scripts/dev.sh:222` runs `394-ssh-package.sh`, and `394` is one of the 26 unassigned scripts. `T-1555` deletes `dev.sh`, so the coupling and the scope gap land in the same change. | Add one line to finding 5 naming `dev.sh:222` and cross-referencing D-2. |
| **minor** | INDEX.md:108 vs recon-c §2 | `T-1540` writes the `TODO/` **directory** and `T-1529` writes the `crates/podbox-cli/src/windows/` **directory**. Two of 100 `writes` cells are not file-level, so the "a file is the atomic unit of collision" rule that every count rests on does not hold for them. | Give `T-1540` the list of `TODO/*.md` files it actually touches, and `T-1529` its target file list, or state that a directory claim means "every file under it, now and later". |
| **minor** | INDEX.md:12 (`Sources` table) | Lists `recon-c.md` as "100-task decomposition, DAG, batches" without noting that §6 holds the only copy of every proof command and §7 item 8 is struck through. The reader cannot tell which parts of it are superseded. | Note the struck item and point at §6 for proofs. |
| **minor** | INDEX.md:109, finding 12 | Records that `refactor/` is untracked and stops. For dispatch: no agent can branch, and `check_tree` never reads `refactor/`, so the dispatch document and every `Prove` in it are outside the gate the plan replaces. | Add: "No agent may branch. The plan is untracked, so no citation in it is held by the gate; correctness depends on reading, not on the gate." |

## Minimum additions to make INDEX.md dispatchable

1. **Regenerate the range table's host column** from recon-c §1, or delete it
   and point at §1. Six of eight rows are wrong.
2. **Insert the 9-task uid-0 list verbatim**, with `T-1587` and `T-1578`
   explicitly excluded, plus a `D-9` for the ninth id that does not exist in
   recon-c §1.
3. **Add D-3 through D-8** using the quoted blocks above, and reconcile the
   T-1551 / decision-record-count contradiction at INDEX.md:165.
4. **Correct 22 to 41** `TODO/`-writing tasks and print the Batch-1
   `TODO/`-yes/no table so the 5 free agents are identifiable.
5. **Print Batch 1's 28 tasks and Batch 4's 14 tasks with their `writes`**, add
   the missing `T-1540..T-1550` parent row, and add the `T-1537`/`T-1573`
   exclusion note.
6. **Add a per-agent work package template** and a stop protocol. This is the
   difference between a document that describes a plan and a document you can
   dispatch six agents from.

With 1–6, six agents can be dispatched **to Batch 1's five `TODO/`-free groups
plus the fixture/test work in Batch 4 that does not wait on D-5**, and no two
of them will share a file. They cannot be dispatched to the wave-2 group, the
`podbox-cli` group, or any `T-15xx` DELETE task until D-3, D-5 and D-8 are
answered by the operator.
