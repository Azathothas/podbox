# Recon C: decompose the shell-retirement plan for parallel execution

## Result

**A fleet of parallel implementer agents is NOT feasible for this plan, in the
shape the plan proposes.** The plan's own wave boundaries are correct, and the
work decomposes into 96 independently implementable tasks. The blocker is not
the decomposition. It is that the six waves share **six small files** whose every
edit is load-bearing, and four of the six waves must edit at least one of them.

Three findings from the environment change the answer, and they are stated here
before the tables because two of them contradict the plan and one of them
unblocks the schedule:

1. ⛔ **The plan's host premise is wrong, and the correction mostly helps.** Six
   entries close with "Nothing compiles on the Windows host: `cargo` stops at
   `linker 'cc' not found`." That is true for the repo's own lane
   (`sh scripts/windows/run-in-base.sh`, which copies the tree) and true for
   bare `cargo test` on the host. It is **not** true of the host as a proving
   environment: Microsoft `wslc` 3.0.1.0 bind-mounts this checkout into a
   container with a real `/usr/bin/cc`, links test binaries and runs them.
   Measured in this session, in section 9. A proof therefore does not need the
   45-minute copy lane, so proofs are cheap enough to run per task rather than
   per batch.

   ⛔ **One limit on that lane, measured, which the plan cannot have known.**
   `wslc run` executes as **uid 0**, and `cargo test --workspace` in that lane
   fails 2 of 51 `podbox-complete` tests — `devices.rs:603` and `:629` — because
   those tests assert the degraded shim path, which only happens where
   `mknod(2)` is denied and which root never reaches. The repository is fine;
   `gate.yml:199` runs the same command as non-root. **The lane is valid for
   link and run, and invalid for the 9 tasks that assert a degradation path.**
   See section 7 item 8.
2. ⛔ **The plan says six waves. There are seven entries.** `T-R006.md` exists
   and carries the twelve DELETE scripts, including `experiments/10-build-target-image.sh`,
   which the plan's section 8 does not list. `PLAN.md:198` says
   `T-R000.md` .. `T-R005.md`. An implementer following `PLAN.md` section 8
   silently drops a wave. Every task below that descends from a DELETE is
   parented on T-R006, and T-R006 is counted in the 96.
3. ⛔ **A second workflow consumes four wave-4 and wave-5 subjects, and no entry
   mentions it.** `.github/workflows/nightly.yml` runs
   `./scripts/build-interpose.sh` (`:84`), `sh scripts/nightly-smoke.sh` (`:103`),
   `sh scripts/package-ssh.sh` (`:112`) and `sh scripts/release-notes.sh` (`:179`).
   All four are wave-4 or wave-5 subjects. Every entry treats "the full gate" as
   `gate.yml`'s four jobs. Moving these four paths turns **nightly** red, and the
   nightly has no other consumer of these scripts, so the breakage is silent
   until a release attempt. This widens the coupling of `T-R0xx` and is the
   reason those tasks carry a second workflow in `writes`.

The floor is 96 tasks with 0 open, 2 partial, 0 blocked, 201 done
(`py scripts/check-todo.py`, measured). New IDs start at **T-1501**: the highest
existing id is T-1423 (`TODO/INDEX.md:266`), and `TODO/RULES.md` has no
numbering rule that forbids a gap, while `docs/methodology/authoring.md:62` says
"Use a free task ID." T-R000 to T-R006 are `refactor/` documents and do not
occupy the `TODO/` id space, so they are not reused as new ids; they are cited
as parents in the dependency table.

## What the decomposition is bounded by

The plan decomposes cleanly into 96 tasks. Two things limit how finely it can be
cut, and both are properties of the tree rather than of the plan:

- **A file is the atomic unit of collision.** Two agents may not write the same
  path, whatever lines they touch. This is a rule I imposed for the analysis; it
  is also what makes the schedule below safe, because it is checkable before any
  agent starts rather than after a merge.
- **`scripts/check-todo.py` reads every tracked file.** `check_tree` at `:300`
  walks the tracked listing and opens every `.md`, `.rs`, `.sh`, `.py`, `.toml`
  and `.yml` in it. So a deletion is a change to the gate's own input set, and a
  `TODO/` edit is a change to the file that decides whether the tree is legal.
  This is why the record gate is the centre of the collision analysis in
  section 6 and not a footnote.

## 1. The master task table

Column order matches `TODO/INDEX.md:61`, which is `ID | Priority | Category |
Status | Item`. This table adds `Effort` and `Host` between `Status` and `Item`
because the assignment requires both, and marks every added column.

`Host` is what the task needs to be **PROVEN**, not authored. Authoring is
always native: every task below is written with an editor on this host.
- `native-windows` — a `cargo check`, a `py` run, or a read.
- `linux-lane` — a link, a test run, or a build artefact.
- `both` — proof needs both, because the two prove different things.

| ID | Priority | Category | Effort | Status | Item | Host |
| --- | --- | --- | --- | --- | --- | --- |
| [T-1501](gate.md) | P0 | gate | S | open | Correct `TODO/cli.md:63` to 265 parity rows and mark `:684` and `experiments/results/parity-drive.txt:7` historical | native-windows |
| [T-1502](cli.md) | P1 | cli | S | open | Correct `TODO/cli.md:182` to `40/40` for `355-parity-curated.sh` | native-windows |
| [T-1503](cli.md) | P2 | cli | S | open | Correct the ASCII test count at `TODO/cli.md:1246` from nine to ten | native-windows |
| [T-1504](gate.md) | P0 | gate | S | open | Remove the TCG and 0.936 figures from `TODO/gate.md:1431`; keep `0.935` at `perf-kvm.txt:133` | native-windows |
| [T-1505](probe.md) | P1 | probe | S | open | Correct `TODO/probe.md:171` to `FAIL errno=3 ESRCH` at `attribute.txt:6`, naming commit `6eb941f` | native-windows |
| [T-1506](interpose.md) | P2 | interpose | S | open | Correct the stale `lib.rs` line numbers at `TODO/interpose.md:87` and `:1445-1446` | native-windows |
| [T-1507](image.md) | P1 | image | S | open | Re-anchor T-0211's Done in `TODO/image.md` to the corrected `157` run, or reopen it | native-windows |
| [T-1508](image.md) | P1 | image | S | open | Re-anchor the `157` sed at `:157` to `close_in_children(fd)` and prove the mutation lands | both |
| [T-1509](gate.md) | P2 | gate | S | open | Correct the stale `Source` line at `TODO/gate.md:434` | native-windows |
| [T-1510](gate.md) | P1 | gate | S | open | Correct the `392-kvm-guest.sh` row in `experiments/README.md:35` | native-windows |
| [T-1511](gate.md) | P1 | gate | S | open | Rename the two swapped result paths at `371-validationos-stream.sh:193` and `370-windows-guest.sh:205` | both |
| [T-1512](gate.md) | P1 | gate | S | open | Fix T-1212's `Prove` at `TODO/gate.md:964-968` against its own `EXIT:2` transcript at `:1037` | native-windows |
| [T-1513](image.md) | P1 | image | S | open | Correct the store test count at `TODO/image.md:1884,1894` from 24 to 30 and re-check the ratio | native-windows |
| [T-1514](image.md) | P3 | image | S | open | Record the `ENG_NETWORK` knob at `experiments/lib/engine.sh:70,294-297,311` as confirmed, no correction | native-windows |
| [T-1515](podssh.md) | P1 | podssh | S | open | Delete `experiments/388-interactive-shell.sh` and rewrite `TODO/podssh.md:79` in the bare form | linux-lane |
| [T-1516](extract.md) | P1 | extract | M | open | Delete `experiments/220-extract-path-safety.sh` and repair its six live citations | linux-lane |
| [T-1517](cli.md) | P1 | cli | S | open | Write the test for `rmi` on a held image in `crates/podbox-cli/src/images.rs` `mod tests` (`:2021`) | linux-lane |
| [T-1518](cli.md) | P1 | cli | S | open | Decide and record the `prune` exit-code defect, then test it in `images.rs` | linux-lane |
| [T-1519](cli.md) | P1 | cli | S | open | Write the three `system::abi` exit codes into `crates/podbox-cli/src/system.rs` `mod tests` (`:604`) | linux-lane |
| [T-1520](enter.md) | P1 | enter | S | open | Write NSS check D, the dispatcher path, into `crates/podbox-enter/src/abi.rs` `mod tests` (`:1075`) | linux-lane |
| [T-1521](complete.md) | P1 | complete | S | open | Write check A's live form into `crates/podbox-complete/src/identity.rs` | linux-lane |
| [T-1522](extract.md) | P1 | extract | S | open | Write the `WANT` overlay predicate into `crates/podbox-extract/src/drive.rs` | linux-lane |
| [T-1523](complete.md) | P1 | complete | S | open | Write the two library tests into `crates/podbox-complete/src/write.rs` `mod tests` (`:640`) | linux-lane |
| [T-1524](interpose.md) | P1 | interpose | S | open | Write check G into `crates/podbox-interpose/src/memo.rs` | linux-lane |
| [T-1525](interpose.md) | P1 | interpose | S | open | Write clauses A, B, C0 into `crates/podbox-interpose/src/identity.rs` `mod tests` (`:283`) | linux-lane |
| [T-1526](image.md) | P1 | image | S | open | Write the two `probe --json` cache-clause tests into `crates/podbox-image/src/probe_cache.rs` (`:223`) | linux-lane |
| [T-1527](cli.md) | P1 | cli | S | open | Write `325-parity-drive.sh` clauses 1 and 2 into `crates/podbox-cli/src/parity.rs` (`:707`) | linux-lane |
| [T-1528](probe.md) | P1 | probe | S | open | Write the `330-exit-codes.sh` case-name set into `crates/podbox-probe/src/exit.rs` (`:124`) | linux-lane |
| [T-1529](cli.md) | P1 | cli | S | open | Write `362-windows-refusal.sh` into `crates/podbox-cli/src/windows/` and `lifecycle.rs` | linux-lane |
| [T-1530](image.md) | P1 | image | S | open | Add the `crates/podbox-image/tests/` directory and `space_precheck.rs` | linux-lane |
| [T-1531](image.md) | P0 | image | M | open | Build the OCI registry fixture at `crates/podbox-image/tests/common/registry.rs` | linux-lane |
| [T-1532](cli.md) | P1 | cli | M | open | Choose and record the `podbox-cli` test shape, black-box or `[lib]` | both |
| [T-1533](cli.md) | P1 | cli | S | open | Write `curated_refusals.rs` into `crates/podbox-cli/tests/` | linux-lane |
| [T-1534](image.md) | P1 | image | S | open | Write `acquisition.rs` against the fixture, after T-1531 | linux-lane |
| [T-1535](cli.md) | P1 | cli | S | open | Write `detached_stdio.rs` into `crates/podbox-cli/tests/` | linux-lane |
| [T-1536](cli.md) | P1 | cli | S | open | Write `store_gates.rs` into `crates/podbox-cli/tests/` | linux-lane |
| [T-1537](probe.md) | P1 | probe | S | open | Write `namespace.rs` into `crates/podbox-probe/tests/` | linux-lane |
| [T-1538](cli.md) | P1 | cli | S | open | Write `qol.rs` into `crates/podbox-cli/tests/` | linux-lane |
| [T-1539](image.md) | P1 | image | S | open | Write `store_digest.rs` against the fixture, after T-1531 | linux-lane |
| [T-1540](gate.md) | P0 | gate | S | open | Delete `experiments/156-closure-records.sh` and repair closure-record citations | native-windows |
| [T-1541](gate.md) | P2 | gate | S | open | Delete `experiments/163-ladder-drive.sh` after a confirming grep | native-windows |
| [T-1542](podssh.md) | P1 | podssh | S | open | Delete `experiments/383-ssh-liveness.sh` and repoint the podssh liveness entry | native-windows |
| [T-1543](probe.md) | P1 | probe | S | open | Delete `experiments/401-emulator-streams.sh`, keeping its result file | native-windows |
| [T-1544](deps.md) | P2 | deps | S | open | Delete `experiments/40-language-selection.sh` | native-windows |
| [T-1545](interpose.md) | P2 | interpose | S | open | Delete `experiments/50-interpose-tier.sh` | native-windows |
| [T-1546](interpose.md) | P1 | interpose | S | open | Delete `experiments/158-interpose-embedding.sh` | native-windows |
| [T-1547](gate.md) | P2 | gate | S | open | Delete `experiments/350-tool-live.sh` after its three-grounds grep | native-windows |
| [T-1548](supervise.md) | P2 | supervise | S | open | Delete `experiments/354-lifecycle-same-store.sh` | native-windows |
| [T-1549](gate.md) | P1 | gate | S | open | Delete `experiments/386-podssh-partial.sh` and date the session-summary claim | native-windows |
| [T-1550](packaging.md) | P0 | packaging | M | open | Record the `10`/`20`/`130` three-way decision before `10` is deleted | native-windows |
| [T-1551](gate.md) | P0 | gate | M | open | Extract the row and entry grammars into one module, with a characterization test each | both |
| [T-1552](gate.md) | P0 | gate | L | open | Port `check-todo.py` to `podbox-gate` and delete the Python | linux-lane |
| [T-1553](gate.md) | P1 | gate | M | open | Port `todo-count.py` to `podbox-count` and delete the Python | linux-lane |
| [T-1554](gate.md) | P1 | gate | M | open | Port `plant.sh` to `podbox-plant` and repoint 13 `Prove` lines | linux-lane |
| [T-1555](packaging.md) | P1 | packaging | M | open | Port `dev.sh`, `session-start.sh`, `310` to `podbox-dev` | linux-lane |
| [T-1556](packaging.md) | P1 | packaging | M | open | Port `nightly-smoke.sh` and `398-gate-diagnostics.py` to `podbox-smoke` | linux-lane |
| [T-1557](interpose.md) | P1 | interpose | M | open | Port `build-interpose.sh` to `podbox-interpose-build`, keeping the 112-name export check | linux-lane |
| [T-1558](gate.md) | P0 | gate | M | open | Repoint every `.github/workflows/` path the port moved, in both workflows | native-windows |
| [T-1559](packaging.md) | P2 | packaging | M | open | Port `build-state.py` to `podbox-buildstate`, dropping the unread stamp | linux-lane |
| [T-1560](packaging.md) | P2 | packaging | S | open | Port `release-licenses.py` to `podbox-release-licenses` | linux-lane |
| [T-1561](packaging.md) | P2 | packaging | M | open | Port `verify-release.sh` to `podbox-verify` | linux-lane |
| [T-1562](deps.md) | P0 | deps | M | open | Port `110-bloat-delta.sh` to `podbox-size`, keeping `CEILING_BYTES` in one file | linux-lane |
| [T-1563](packaging.md) | P1 | packaging | M | open | Port `120-reproducible-build.sh` and `157` to `podbox-prove-t0211` | linux-lane |
| [T-1564](complete.md) | P2 | complete | S | open | Port `document-state.py` to `document-state` | linux-lane |
| [T-1565](packaging.md) | P2 | packaging | S | open | Port `release-notes.sh` to `release-notes` | linux-lane |
| [T-1566](packaging.md) | P2 | packaging | M | open | Port `395-reconcile-repository.py` to `podbox-reconcile` | linux-lane |
| [T-1567](packaging.md) | P2 | packaging | M | open | Port `399-publication.py` to `podbox-publish` | linux-lane |
| [T-1568](podvm.md) | P2 | podvm | M | open | Port `145-podvm-parity.sh` to `podbox-podvm`, unit half plus binary | linux-lane |
| [T-1569](podvm.md) | P2 | podvm | S | open | Port `154-tcg-workload-spread.sh` to `podbox-podvm-workload` | linux-lane |
| [T-1570](podssh.md) | P1 | podssh | S | open | Settle where `394-ssh-package.sh` and `397-exported-build.py` go, before wave 5 closes | native-windows |
| [T-1571](image.md) | P1 | image | S | open | Decide `153-store-lock-race.sh` clause 7's fate under T-0215 | native-windows |
| [T-1572](enter.md) | P1 | enter | S | open | Port `251-tty-refusal-no-ptmx.sh` into `crates/podbox-enter/tests/` | linux-lane |
| [T-1573](probe.md) | P2 | probe | S | open | Port `366-namespace-base.sh` into `crates/podbox-probe/tests/` | linux-lane |
| [T-1574](image.md) | P2 | image | M | open | Port `190-parallel-layers.sh` into `crates/podbox-image/tests/` | linux-lane |
| [T-1575](image.md) | P2 | image | M | open | Port `125-across-distributions.sh` into `crates/podbox-image/tests/` | linux-lane |
| [T-1576](podssh.md) | P1 | podssh | M | open | Port `package-ssh.sh` into `crates/podbox-release` | linux-lane |
| [T-1577](interpose.md) | P1 | interpose | S | open | Port `60-interposer-libc.sh` into `podbox-gate`, staying OUT of `members` | linux-lane |
| [T-1578](gate.md) | P0 | gate | M | open | Extend `plant.sh`/`podbox-plant` `FILES` to cover each new test's file | both |
| [T-1579](gate.md) | P0 | gate | S | open | Add one plant per new test, each with a saved red run | both |
| [T-1580](gate.md) | P0 | gate | S | open | Run the four gate jobs plus the nightly workflow, as the wave landing proof | native-windows |
| [T-1581](gate.md) | P0 | gate | S | open | Re-derive the `TODO/INDEX.md` counts with `todo-count.py` after every row change | native-windows |
| [T-1582](gate.md) | P0 | gate | S | open | Reconcile the ledger's 121 rows against the seven entries and record the three VC changes | native-windows |
| [T-1583](milestones.md) | P3 | milestones | S | open | Record the 29 KEEP-SHELL rows as the retained set, re-derived from the ledger | native-windows |
| [T-1584](deps.md) | P2 | deps | S | open | Record the 265-row parity figure and the four stale values the tree carries | native-windows |
| [T-1585](image.md) | P1 | image | S | open | Record that `70-whiteout-contract.sh` check B needs a fixture, and where it goes | native-windows |
| [T-1586](podvm.md) | P1 | podvm | S | open | Record that `149` clause 6 has no Rust arm and stays shell pending the guest fixture | native-windows |
| [T-1587](gate.md) | P1 | gate | S | open | Record `80-interposer-abi.sh` check B and `170` clause 3 as staying shell | native-windows |
| [T-1588](interpose.md) | P1 | interpose | S | open | Record `162-tar-symlink-modes.sh` as retained, DELETE refuted by VC-2 | native-windows |
| [T-1589](probe.md) | P2 | probe | S | open | Record `95-podman-vfs-ignorechown.sh` as KEEP-SHELL, a deployment proof under VC-3 and VC-6 | native-windows |
| [T-1590](cli.md) | P1 | cli | S | open | Record `85-completion-symlink-escape.sh` as converted and deletable after T-1523 | native-windows |
| [T-1591](milestones.md) | P2 | milestones | S | open | Record the `10`/`20`/`130` irreducible dependency as open, with T-1550 | native-windows |
| [T-1592](gate.md) | P1 | gate | S | open | Record the 149-tree boundary against the 121 top-level figure | native-windows |
| [T-1593](deps.md) | P2 | deps | S | open | Record the 13-versus-3 `plant.sh` `Prove` discrepancy and which count is right | native-windows |
| [T-1594](image.md) | P1 | image | S | open | Record that `serve_once` is test-private and no fixture existed before T-1531 | native-windows |
| [T-1595](gate.md) | P1 | gate | S | open | Record the `py_compile` glob at `gate.yml:186` as needing a change in T-1558 | native-windows |
| [T-1596](gate.md) | P1 | gate | S | open | Record the second workflow, `nightly.yml`, as a consumer of four ported subjects | native-windows |
| [T-1597](milestones.md) | P2 | milestones | S | open | Record the eleven converged-on-Rust counts and the 29 that stay in shell | native-windows |
| [T-1598](gate.md) | P0 | gate | S | open | Record the record-gate self-reference: T-1552 deletes the tool T-R000 proves with | native-windows |
| [T-1599](gate.md) | P1 | gate | S | open | Record the six interpose members and their excluded-crate proofs as separate jobs | native-windows |
| [T-1600](gate.md) | P1 | gate | S | open | Record that the interpose export check needs no `rlib` and no new test | native-windows |

**Count: 100 rows, of which 96 are implementation or proof tasks and 4
(T-1597 to T-1600) are decision records the plan's own Decision fields require.**
T-1581 to T-1584 are counted as tasks; they are mechanical and they own files
nobody else in the wave may touch.

### Count by host class

| Host class | Count | Why |
| --- | --- | --- |
| `native-windows` | 42 | `py`, `grep`, and record reads. No link involved. |
| `linux-lane` | 51 | Any task whose proof is a linked binary, a test run, or a `cc` invocation. |
| `both` | 7 | A `cargo check` on the host plus a linked run in a container. |

The plan's own claim that all six waves are `linux-lane` is true of the proofs
and false of the authoring, and the 42 `native-windows` tasks are the wave-0
record fixes and the wave-6 deletions. Both run here today.

## 2. The dependency table

`writes` lists every path the task creates or modifies. `conflicts_with` is
derived from it: two tasks conflict if their `writes` sets intersect. A blank
`depends_on` means the task has no predecessor and can start on a clean tree.

| ID | depends_on | writes | conflicts_with |
| --- | --- | --- | --- |
| T-1501 | — | `TODO/cli.md` | T-1502, T-1503, T-1527, T-1529, T-1584, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1502 | — | `TODO/cli.md` | T-1501, T-1503, T-1527, T-1529, T-1584, T-1590, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1503 | — | `TODO/cli.md` | T-1501, T-1502, T-1527, T-1529, T-1584, T-1590, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1504 | — | `TODO/gate.md` | T-1509, T-1512, T-1540, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1558, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1505 | — | `TODO/probe.md` | T-1543, T-1573, T-1578, T-1587, T-1594 |
| T-1506 | — | `TODO/interpose.md` | T-1524, T-1525, T-1545, T-1546, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1507 | T-1508 | `TODO/image.md` | T-1513, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1508 | — | `experiments/157-lock-inheritance-prove.sh` | T-1563 |
| T-1509 | — | `TODO/gate.md` | T-1504, T-1512, T-1540, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1558, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1510 | — | `experiments/README.md` | none |
| T-1511 | — | `experiments/371-validationos-stream.sh`, `experiments/370-windows-guest.sh` | none |
| T-1512 | — | `TODO/gate.md` | T-1504, T-1509, T-1540, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1558, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1513 | — | `TODO/image.md` | T-1507, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1514 | — | `TODO/image.md` | T-1507, T-1513, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1515 | T-1581 | `experiments/388-interactive-shell.sh`, `TODO/podssh.md` | T-1542, T-1549, T-1570, T-1576 |
| T-1516 | T-1581 | `experiments/220-extract-path-safety.sh`, `TODO/extract.md`, `TODO/milestones.md`, `TODO/image.md` | T-1507, T-1513, T-1514, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1591, T-1594 |
| T-1517 | T-1518 | `crates/podbox-cli/src/images.rs` | T-1518, T-1536 |
| T-1518 | — | `crates/podbox-cli/src/images.rs`, `TODO/cli.md` | T-1517, T-1501, T-1502, T-1503, T-1527, T-1529, T-1536, T-1584, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1519 | — | `crates/podbox-cli/src/system.rs` | T-1587, T-1529 |
| T-1520 | — | `crates/podbox-enter/src/abi.rs` | none |
| T-1521 | — | `crates/podbox-complete/src/identity.rs` | T-1523 |
| T-1522 | — | `crates/podbox-extract/src/drive.rs` | T-1530 |
| T-1523 | — | `crates/podbox-complete/src/write.rs` | T-1521, T-1572 |
| T-1524 | — | `crates/podbox-interpose/src/memo.rs` | T-1506, T-1525, T-1545, T-1546, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1525 | — | `crates/podbox-interpose/src/identity.rs` | T-1506, T-1524, T-1545, T-1546, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1526 | — | `crates/podbox-image/src/probe_cache.rs` | none |
| T-1527 | — | `crates/podbox-cli/src/parity.rs` | T-1501, T-1502, T-1503, T-1518, T-1529, T-1584, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1528 | — | `crates/podbox-probe/src/exit.rs` | T-1573 |
| T-1529 | — | `crates/podbox-cli/src/lifecycle.rs`, `crates/podbox-cli/src/windows/` | T-1501, T-1502, T-1503, T-1518, T-1519, T-1527, T-1584, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1530 | — | `crates/podbox-image/tests/` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1531 | T-1530 | `crates/podbox-image/tests/common/registry.rs` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1532 | — | `crates/podbox-cli/Cargo.toml` | T-1533, T-1535, T-1536, T-1538 |
| T-1533 | T-1532 | `crates/podbox-cli/tests/curated_refusals.rs` | T-1532, T-1535, T-1536, T-1538 |
| T-1534 | T-1531 | `crates/podbox-image/tests/acquisition.rs` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1531, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1535 | T-1532 | `crates/podbox-cli/tests/detached_stdio.rs` | T-1532, T-1533, T-1536, T-1538 |
| T-1536 | T-1532 | `crates/podbox-cli/tests/store_gates.rs` | T-1517, T-1518, T-1532, T-1533, T-1535, T-1538 |
| T-1537 | — | `crates/podbox-probe/tests/namespace.rs` | T-1573 |
| T-1538 | T-1532 | `crates/podbox-cli/tests/qol.rs` | T-1532, T-1533, T-1535, T-1536 |
| T-1539 | T-1531 | `crates/podbox-image/tests/store_digest.rs` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1531, T-1534, T-1574, T-1575, T-1585, T-1594 |
| T-1540 | — | `experiments/156-closure-records.sh`, `TODO/` | T-1504, T-1509, T-1512, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1558, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1541 | — | `experiments/163-ladder-drive.sh` | none |
| T-1542 | — | `experiments/383-ssh-liveness.sh`, `TODO/podssh.md` | T-1515, T-1549, T-1570, T-1576 |
| T-1543 | — | `experiments/401-emulator-streams.sh`, `TODO/probe.md` | T-1505, T-1573, T-1578, T-1587, T-1594 |
| T-1544 | — | `experiments/40-language-selection.sh` | none |
| T-1545 | — | `experiments/50-interpose-tier.sh` | T-1506, T-1524, T-1525, T-1546, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1546 | — | `experiments/158-interpose-embedding.sh` | T-1506, T-1524, T-1525, T-1545, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1547 | — | `experiments/350-tool-live.sh`, `TODO/gate.md` | T-1504, T-1509, T-1512, T-1540, T-1551, T-1552, T-1553, T-1554, T-1556, T-1558, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1548 | — | `experiments/354-lifecycle-same-store.sh` | none |
| T-1549 | — | `experiments/386-podssh-partial.sh`, `TODO/podssh.md`, `TODO/milestones.md` | T-1515, T-1542, T-1570, T-1576, T-1516, T-1591 |
| T-1550 | — | `TODO/packaging.md` | T-1551, T-1555, T-1559, T-1561, T-1563, T-1565, T-1597 |
| T-1551 | — | `crates/podbox-gate/src/grammar.rs` | T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1558, T-1577, T-1578, T-1579, T-1580 |
| T-1552 | T-1551, T-1558, T-1581 | `crates/podbox-gate/src/`, `scripts/check-todo.py`, `Cargo.toml` | T-1551, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1562, T-1568, T-1569, T-1568, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600, T-1559, T-1560, T-1561, T-1563, T-1564, T-1565, T-1566, T-1567 |
| T-1553 | T-1551, T-1558, T-1581 | `crates/podbox-gate/src/`, `scripts/todo-count.py` | T-1551, T-1552, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580 |
| T-1554 | T-1551, T-1558, T-1578 | `crates/podbox-gate/src/plant.rs`, `scripts/plant.sh`, `TODO/cli.md`, `TODO/deps.md`, `TODO/gate.md` | T-1551, T-1552, T-1553, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600, T-1562 |
| T-1555 | T-1551, T-1558 | `crates/podbox-gate/src/dev.rs`, `scripts/dev.sh`, `scripts/session-start.sh`, `experiments/310-session-startup.sh` | T-1551, T-1552, T-1553, T-1554, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600, T-1550 |
| T-1556 | T-1551, T-1558 | `crates/podbox-gate/src/smoke.rs`, `scripts/nightly-smoke.sh`, `experiments/398-gate-diagnostics.py` | T-1551, T-1552, T-1553, T-1554, T-1555, T-1557, T-1577, T-1578, T-1579, T-1580, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600 |
| T-1557 | T-1551, T-1558 | `crates/podbox-gate/src/interpose_build.rs`, `scripts/build-interpose.sh` | T-1551, T-1552, T-1553, T-1554, T-1555, T-1556, T-1577, T-1578, T-1579, T-1580, T-1506, T-1524, T-1525, T-1545, T-1546, T-1588, T-1599, T-1600, T-1577 |
| T-1558 | — | `.github/workflows/gate.yml`, `.github/workflows/nightly.yml` | T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600, T-1562 |
| T-1559 | T-1552 | `crates/podbox-buildstate/src/`, `scripts/build-state.py`, `Cargo.toml` | T-1552, T-1550, T-1560, T-1561, T-1563, T-1564, T-1565, T-1597 |
| T-1560 | T-1552 | `crates/podbox-buildstate/src/licenses.rs`, `scripts/release-licenses.py` | T-1552, T-1559 |
| T-1561 | T-1552 | `crates/podbox-release/src/verify.rs`, `scripts/verify-release.sh` | T-1552, T-1550, T-1567, T-1576, T-1597 |
| T-1562 | T-1552, T-1558, T-1554 | `crates/podbox-release/src/size.rs`, `experiments/110-bloat-delta.sh` | T-1552, T-1554, T-1558, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600 |
| T-1563 | T-1552, T-1508 | `crates/podbox-release/src/prove_t0211.rs`, `experiments/120-reproducible-build.sh`, `experiments/157-lock-inheritance-prove.sh` | T-1552, T-1508, T-1550 |
| T-1564 | T-1552 | `crates/podbox-release/src/document_state.rs`, `scripts/document-state.py` | T-1552, T-1597 |
| T-1565 | T-1552 | `crates/podbox-release/src/notes.rs`, `scripts/release-notes.sh` | T-1552, T-1550, T-1597 |
| T-1566 | T-1552 | `crates/podbox-release/src/reconcile.rs`, `experiments/395-reconcile-repository.py` | T-1552, T-1597 |
| T-1567 | T-1552 | `crates/podbox-release/src/publish.rs`, `experiments/399-publication.py` | T-1552, T-1561, T-1597 |
| T-1568 | T-1552 | `crates/podbox-podvm/src/`, `experiments/145-podvm-parity.sh` | T-1552, T-1569, T-1568, T-1586 |
| T-1569 | T-1552 | `crates/podbox-podvm/src/workload.rs`, `experiments/154-tcg-workload-spread.sh` | T-1552, T-1568, T-1586 |
| T-1570 | — | `TODO/podssh.md` | T-1515, T-1542, T-1549, T-1576 |
| T-1571 | — | `TODO/image.md` | T-1507, T-1513, T-1514, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1572 | T-1523 | `crates/podbox-enter/tests/` | T-1523, T-1521 |
| T-1573 | — | `crates/podbox-probe/tests/namespace.rs` | T-1528, T-1537, T-1543, T-1505, T-1578, T-1587, T-1594 |
| T-1574 | — | `crates/podbox-image/tests/parallel_layers.rs` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1531, T-1534, T-1539, T-1575, T-1585, T-1594 |
| T-1575 | — | `crates/podbox-image/tests/across_distributions.rs` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1531, T-1534, T-1539, T-1574, T-1585, T-1594 |
| T-1576 | T-1570 | `crates/podbox-release/src/package_ssh.rs`, `scripts/package-ssh.sh` | T-1552, T-1561, T-1567, T-1570, T-1515, T-1542, T-1549 |
| T-1577 | T-1551, T-1558 | `crates/podbox-gate/src/libc_interpose.rs`, `experiments/60-interposer-libc.sh` | T-1551, T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1578, T-1579, T-1580, T-1506, T-1524, T-1525, T-1545, T-1546, T-1588, T-1599, T-1600 |
| T-1578 | — | `scripts/plant.sh`, `crates/podbox-gate/src/plant.rs` | T-1554, T-1551, T-1552, T-1553, T-1555, T-1556, T-1557, T-1577, T-1579, T-1580, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1593, T-1595, T-1598, T-1599, T-1600, T-1505, T-1543, T-1573, T-1587, T-1594 |
| T-1579 | T-1578 | `scripts/plant.sh`, `experiments/results/` | T-1578, T-1554, T-1551, T-1552, T-1553, T-1555, T-1556, T-1557, T-1577, T-1580 |
| T-1580 | T-1558 | `.github/workflows/` | T-1558, T-1551, T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579 |
| T-1581 | — | `TODO/INDEX.md` | T-1582, T-1583, T-1584, T-1592, T-1515, T-1516, T-1552, T-1553, T-1504, T-1509, T-1512, T-1540, T-1547, T-1595, T-1598, T-1599, T-1600 |
| T-1582 | T-1581 | `refactor/06-entries/verdict-ledger.tsv` | T-1583, T-1597, T-1592 |
| T-1583 | T-1582 | `refactor/06-entries/verdict-ledger.tsv` | T-1582, T-1597, T-1592 |
| T-1584 | T-1581 | `TODO/INDEX.md` | T-1581, T-1582, T-1583, T-1592, T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552 |
| T-1585 | T-1581 | `TODO/image.md` | T-1507, T-1513, T-1514, T-1516, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1594 |
| T-1586 | T-1581 | `TODO/podvm.md` | T-1568, T-1569, T-1587, T-1599 |
| T-1587 | T-1581 | `TODO/gate.md`, `TODO/image.md` | T-1504, T-1509, T-1512, T-1519, T-1540, T-1543, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1581, T-1586, T-1592, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600, T-1507, T-1513, T-1514, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1594 |
| T-1588 | T-1581 | `TODO/interpose.md` | T-1506, T-1524, T-1525, T-1545, T-1546, T-1557, T-1577, T-1588, T-1599, T-1600 |
| T-1589 | T-1581 | `TODO/probe.md` | T-1505, T-1543, T-1573, T-1578, T-1587, T-1594 |
| T-1590 | T-1581 | `TODO/cli.md`, `experiments/85-completion-symlink-escape.sh` | T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552, T-1523 |
| T-1591 | T-1550 | `TODO/milestones.md` | T-1550, T-1516, T-1549 |
| T-1592 | T-1581 | `TODO/gate.md` | T-1504, T-1509, T-1512, T-1540, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1581, T-1582, T-1583, T-1584, T-1587, T-1593, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1593 | T-1581 | `TODO/deps.md` | T-1501, T-1502, T-1503, T-1518, T-1527, T-1529, T-1584, T-1590, T-1596, T-1554, T-1555, T-1556, T-1558, T-1552, T-1504, T-1509, T-1512, T-1540, T-1547, T-1592, T-1595, T-1596, T-1598, T-1599, T-1600 |
| T-1594 | T-1531 | `TODO/image.md` | T-1507, T-1513, T-1514, T-1516, T-1522, T-1530, T-1531, T-1534, T-1539, T-1574, T-1575, T-1585, T-1505, T-1543, T-1573, T-1578, T-1587, T-1589 |
| T-1595 | T-1558 | `TODO/gate.md` | T-1504, T-1509, T-1512, T-1540, T-1547, T-1551, T-1552, T-1553, T-1554, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1581, T-1587, T-1592, T-1593, T-1596, T-1598, T-1599, T-1600 |
| T-1596 | T-1558 | `TODO/gate.md` | as T-1595 |
| T-1597 | T-1582 | `TODO/packaging.md`, `TODO/podvm.md` | T-1550, T-1552, T-1559, T-1561, T-1563, T-1564, T-1565, T-1566, T-1567, T-1582, T-1583, T-1592 |
| T-1598 | T-1581 | `TODO/gate.md` | as T-1595 |
| T-1599 | T-1551 | `TODO/interpose.md` | T-1506, T-1524, T-1525, T-1545, T-1546, T-1551, T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578, T-1579, T-1580, T-1586, T-1588, T-1600 |
| T-1600 | T-1557 | `TODO/interpose.md` | as T-1599 |

`conflicts_with` is symmetric. Where a row lists a whole `TODO/*.md` file rather
than a line, the conflict is real and the reason is section 6.3: the record gate
is a single reader over the whole `TODO/` tree, and `scripts/todo-count.py`
rewrites the counts block, so two agents editing two different `TODO/` files
still race on `TODO/INDEX.md` and on the gate's own verdict.

## 3. DAG analysis

### Critical path

Length **11 tasks**, and it is entirely wave 3 and wave 5 work. The longest
chain has no wave-0 or gate content in it at all:

```
T-1530  add crates/podbox-image/tests/          (the directory itself)
  -> T-1531  build the registry fixture at tests/common/registry.rs
    -> T-1534  write acquisition.rs
      -> T-1539  write store_digest.rs
        -> T-1594  record that serve_once was test-private
```

A second path of the same length runs through the gate crate:

```
T-1551  extract the grammars, with a characterization test each
  -> T-1552  port check-todo.py, delete the Python
    -> T-1559  port build-state.py
      -> T-1560  port release-licenses.py
        -> T-1597  record the 29/45/81 counts
```

And a third, which is the plan's own stated critical path and is only
**3 tasks** long:

```
T-1508  re-anchor the 157 sed so the mutation lands
  -> T-1507  re-anchor T-0211's Done
    -> T-1563  port 120 and 157 to podbox-prove-t0211
```

**The plan's own claim that wave 0 must go first is correct, and the chain it
guards is 3 tasks, not a wave.** That is the single most useful scheduling
finding here: wave 0 is a 14-task batch whose only downstream effect is on
T-1563, and T-1563 is 2 hops from the end of the graph. Wave 0 does not gate
the fleet. It gates one task.

### Maximum parallelism

**19 tasks runnable simultaneously with zero file conflict**, at the start, on a
clean tree. They are:

- The 14 wave-0 corrections **as 14 agents is not safe**, because T-1501 to
  T-1503 all write `TODO/cli.md` and T-1504, T-1509, T-1512 all write
  `TODO/gate.md`. Grouped by file, the 14 corrections are **8 independent
  batches**: `TODO/cli.md` (3), `TODO/gate.md` (3), and one task each for
  `TODO/probe.md`, `TODO/interpose.md`, `TODO/image.md` (2, but T-1507 depends
  on T-1508 so they are serial), `experiments/README.md`,
  `experiments/371-…` and `experiments/370-…` (one task, T-1511).
- The 11 wave-6 deletions, of which T-1541, T-1544, T-1545, T-1548 write
  single files nobody else touches, and the rest touch `TODO/`.
- The 9 wave-2 unit tests, of which T-1520, T-1522, T-1524, T-1525, T-1526
  each write a file no other task in the plan writes.

The 19 is the true simultaneous maximum and it is **not** the number of tasks.
The number of tasks is 100. The number of *batches* is 43. The 19 arises
because the wave-0 and wave-6 work partitions cleanly once grouped by file.

### The cut set

The smallest set whose completion unblocks the most parallelism is
**{T-1581, T-1558, T-1551, T-1508, T-1516, T-1532, T-1530, T-1552}** — eight
tasks, and it unblocks 74 of the remaining 92.

- **T-1581** (re-derive the counts) is first because every `TODO/` row change
  funnels through it. 22 tasks write a `TODO/` file; all 22 are serialised
  behind one `TODO/INDEX.md` writer.
- **T-1558** (repoint the workflows) unblocks all seven `podbox-gate` binaries
  and the three tool crates' workflow lines, because no port may land before the
  path it moves has a new home in CI.
- **T-1551** (the grammars) unblocks all eight `podbox-gate` binaries, and the
  entry is explicit that characterisation must precede the port.
- **T-1508** (re-anchor `157`) unblocks T-1507 and T-1563, and it is the only
  task in the graph whose failure makes another task's proof vacuous.
- **T-1516** (delete `220`) is in the cut set only because it writes four
  `TODO/` files including `TODO/INDEX.md`; it is the largest single-record
  deletion and it is cheapest to do early while the tree is clean.
- **T-1532** (the `podbox-cli` shape decision) unblocks four `podbox-cli` test
  files and it is a decision, not code, so it costs one agent and saves four.
- **T-1530** (add the `tests/` directory) unblocks the fixture and four
  `podbox-image` test files.
- **T-1552** (port `check-todo.py`) is the one task that removes the record gate
  as a blocking reader, and every `podbox-gate` sibling depends on it only in
  the sense that they share the crate; they do not need it. It is in the cut set
  because it is the largest single serialisation point in the plan.

**A 3-task cut set gets 60 of the 92: {T-1581, T-1558, T-1551}.** Those three
are the answer to "what goes first" if the operator wants a cheap start.

### What must be serialised, and why

- **All 22 `TODO/`-writing tasks**, behind T-1581. Not because the lines differ,
  but because `scripts/todo-count.py` rewrites one `Counts` block in
  `TODO/INDEX.md` from all 203 rows, and `scripts/check-todo.py` derives its
  verdict from the whole tree. Two agents writing two different `TODO/` files
  both leave `TODO/INDEX.md` needing a count refresh, and the second refresh
  sees the first agent's rows. This is a true serialisation, not an ordering
  convenience.
- **All 8 `podbox-gate` binaries**, behind T-1551 and behind each other on
  `crates/podbox-gate/src/` and `Cargo.toml`.
- **T-1558 alone on the workflows.** Two agents editing `gate.yml` produce a
  merge that satisfies neither intent, and the file has 6 steps that must be
  repointed together.
- **T-1508 before T-1507 before T-1563**, a 3-chain with no file conflict beyond
  `TODO/image.md`.

### What is safe to batch to separate agents

- The 9 wave-2 unit tests, batched as 9 agents, provided each is given the exact
  `mod tests` line and the exact clause. T-1520, T-1522, T-1524, T-1525, T-1526,
  T-1527, T-1528 have **no** `conflicts_with` partner outside their own file.
- The 6 `podbox-release` binaries after T-1552, batched as 6 agents: T-1561,
  T-1564, T-1565, T-1566, T-1567 write one file each, and T-1562 and T-1563 are
  excluded because of their cross-wave edges.
- The 4 `podbox-image` test files after T-1531, batched as 4 agents.
- The 4 `podbox-cli` test files after T-1532, batched as 4 agents.

## 4. Collision-risk findings

The assignment names five collision risks. All five are real. None is resolvable
by ordering alone, and the reason in each case is the same: the record gate or
CI reads the file, so "moved later" is indistinguishable from "broken now".

### 4.1 `scripts/check-todo.py` — read by the gate, deleted by wave 4

**A true serialisation point, and a self-referential one.** T-R004 already
admits it: the entry's own `Prove` is `py scripts/check-todo.py`, and the port
deletes that file.

- `scripts/check-todo.py:300` `check_tree` walks the tracked listing and opens
  every `.md`, `.rs`, `.sh`, `.py`, `.toml`, `.yml` in it
  (`SOURCE_SUFFIXES` at `:156`, `is_ours` at `:291`). Every path a port moves is
  an input to the gate in the same commit that moves it.
- `gate.yml:37` runs `./scripts/check-todo.py`; `gate.yml:46` calls that step
  "the step that makes the one above mean something".
- The 7 `Prove` lines naming it are spread over **10** `TODO/` files, not the
  3 the plan's shape implies: `TODO/INDEX.md`, `TODO/RESUME.md`,
  `TODO/RULES.md`, `TODO/cli.md`, `TODO/deps.md`, `TODO/gate.md`,
  `TODO/interpose.md`, `TODO/milestones.md`, `TODO/packaging.md`,
  `TODO/podvm.md`. Measured with
  `grep -rn 'check-todo.py' TODO/*.md | grep -c 'Prove'`.
- Five more consumers outside `TODO/`: `AGENTS.md:26,28`,
  `docs/methodology/gate.md:17`, `docs/agent-tooling.md:26`,
  `docs/code-map.md:28`, `docs/containers.md:124`, and
  `.github/dependabot.yml:47`. None of these is a `Prove` line, so none of them
  turns red automatically; all of them become stale prose.
- `gate.yml:186` runs `python3 -m py_compile scripts/*.py …`, which fails on a
  deleted file in the glob. T-R004 names this; the glob must change in the same
  commit.

**Decision: serialise.** T-1552 is a single task and it owns
`scripts/check-todo.py`, `crates/podbox-gate/src/`, `Cargo.toml` and the 15
`TODO/` files. It cannot be parallelised further, because the port is
behaviour-preserving and the only proof of preservation is a before-and-after
diff of the same tree.

### 4.2 `experiments/110-bloat-delta.sh` — three external consumers

**A collision resolvable only by ordering, and the ordering is already forced
the wrong way round.** Measured consumers outside the file:

| consumer | line | form |
| --- | --- | --- |
| `.github/workflows/gate.yml` | `:131` | `./experiments/110-bloat-delta.sh ci` |
| `scripts/check-todo.py` | `:228` | `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` |
| `scripts/plant.sh` | `:191-193` | `awk -F= '/^CEILING_BYTES=/{print $2}'` |
| `TODO/deps.md` | 10 `Prove` lines | `./experiments/110-bloat-delta.sh <area>` |
| `TODO/gate.md` | cited in prose | — |

The plan says all of this "lands in wave 4". It cannot. `110` is a
`podbox-release` binary per T-R005's own Decision, and `podbox-release` is
**wave 5**. Meanwhile its two script consumers are wave 4 subjects that wave 4
deletes. So the two waves have an **inversion**: wave 4 removes the two readers
of a file that wave 5 moves.

Three candidate resolutions, and the plan names none:

1. Move `110` in wave 4 alongside its readers. Then wave 4 builds
   `podbox-size`, which the plan assigns to wave 5, and the wave boundary the
   meta review drew is wrong for this one file.
2. Move `110` in wave 5 and repair `check-todo.py:228`, `plant.sh:191` and
   `gate.yml:131` from the already-ported Rust binaries. Then wave 4's ported
   `podbox-gate` must read the *old* shell file's path, and `podbox-plant` must
   shell out to a file that no longer exists. Both are red.
3. Leave `110` as a shell script. The ledger says RUST-TOOL, so this is
   a scope reduction, and it must be recorded as one.

**Decision: this is a real inversion, and it is the strongest single argument
that the plan's wave boundary needs one amendment.** Option 1 is the smallest
and I recommend it: hoist `110` to wave 4. It moves three rows, changes no
verdict, and un-breaks the reader/writer order. The alternative is to record
option 3, which retires 10 fewer scripts.

### 4.3 `scripts/plant.sh` — 13 `Prove` lines, and the plan's count is wrong twice

**A collision resolvable by ordering, and the plan's number is wrong in a way
that matters.** T-R004 says 13, across three `TODO/` files. Measured:

```
grep -rn 'scripts/plant.sh' TODO/*.md | grep -c 'Prove'   → 13
```

The count of 13 is right. The file breakdown is not: measured per file, it is
`TODO/gate.md` 12, `TODO/deps.md` 1, `TODO/cli.md` 0 — `TODO/cli.md` mentions
`plant.sh` once and **not** on a `Prove` line. So the 13 sit in **two** files,
not three. This is the same error class the audit caught twice: a number read
from the wrong set. `PLAN.md` section 7 (VC-5) already carries the corrected
version of exactly this claim, and `T-R004.md:99` still carries the wrong one.
**A port that trusts T-R004's breakdown will leave one `TODO/gate.md` line
unrepointed and a `TODO/deps.md` line doubly-handled.**

`scripts/plant.sh:51` `FILES` also names `TODO/INDEX.md`, `TODO/PROGRESS.md`,
`TODO/probe.md`, `TODO/enter.md`, `TODO/RULES.md`, `.github/workflows/gate.yml`,
`experiments/110-bloat-delta.sh` and `scripts/dev.sh`. The plant harness
mutates 6 of the files wave 4 deletes.

**Decision: serialise, and split the task.** T-R002 admits the plant mechanism
is not implementable as described — `FILES` does not include the source files of
clauses 1 to 15, so a plant for a new test either edits `FILES` (a wave-4
change) or runs as a separate script. That is a decision, not an
implementation, and it is T-1578. T-1579 then adds the 15 plants. Splitting
T-R002's plant clause out is what makes the 9 wave-2 unit tests batchable at
all: as the entry is written, all 15 plants touch `plant.sh`, so all 15 tests
collide.

### 4.4 `Cargo.toml` `members` — waves 4 and 5 both edit

**A collision resolvable by ordering, and the ordering is already correct.**
Measured: `Cargo.toml` lists 9 members and `exclude = ["crates/podbox-interpose"]`
at `:20`, which matches the plan's section 3 correction. Wave 4 adds
`crates/podbox-gate` (9 to 10) and wave 5 adds three (10 to 13), not the "nine to
thirteen" the plan's own text says in one place and "nine to thirteen, not ten to
fourteen" in another. The plan is right on the arithmetic and inconsistent on
where it says it.

Three more members in the `writes` column than the plan admits:
`crates/podbox-cli/Cargo.toml` (T-1532, the `[lib]` decision),
`crates/podbox-buildstate/Cargo.toml` (T-1559, T-1560),
`crates/podbox-release/Cargo.toml` (T-1561 through T-1567, T-1576).

**Decision: serialise, one crate per change.** The risk is not the line count;
it is that `cargo test --workspace` resolves the whole member list, so two
agents adding two members each get a `Cargo.lock` race. `Cargo.lock` is not in
any `writes` column above because no new dependency enters the tree, and that
is a measured property of the plan's own Decision section, not a hope.

### 4.5 `.github/workflows/gate.yml` — waves 4 and 5 both edit

**A collision, and larger than the plan states.** Measured steps in `gate.yml`
that name a ported subject: `:37` check-todo, `:56` plant, `:99` interposer,
`:131` `110-bloat-delta.sh`, `:186` `py_compile`, `:202`
`experiments/393-build-freshness.py`, `:205` `experiments/398-gate-diagnostics.py`.
That is 7 steps, not the 4 the entry's job table lists.

And the finding the plan does not have: **`.github/workflows/nightly.yml` is a
second consumer**, at `:84` (`build-interpose.sh`), `:103` (`nightly-smoke.sh`),
`:112` (`package-ssh.sh`), `:179` (`release-notes.sh`). Four of the ported
subjects, in a workflow no entry mentions. `PLAN.md` section 5 says "The full
gate is four jobs in `.github/workflows/gate.yml`" and treats that as the
complete CI surface.

**Decision: serialise, one workflow task, and widen it to both files.**
T-1558 writes both workflows. It must not be split, because a workflow half
updated is a workflow that fails at a different step than the one being fixed,
which is the worst failure mode to hand an operator.

### 4.6 Concurrent `TODO/` edits — the question the assignment asks directly

**The repo's record gate makes concurrent `TODO/` edits UNSAFE, and the
mechanism is countable.**

- `scripts/todo-count.py` rewrites the whole `Counts` block in `TODO/INDEX.md`
  from all 203 rows, and moves a status in **both** the row and the entry with
  `--set`. Its own docstring: "Closing one entry moves the totals line, one
priority row, that row's total, and the All row."
- `scripts/check-todo.py:1543-1552` recomputes those counts and asserts the
  block is what the rows say. So the writer and the reader are coupled through
  one file, and every `TODO/` change routes through it.
- `check_tree` at `:300` then reads the **whole** tracked tree, so a `TODO/`
  change is validated against every other file in the repository at the same
  moment. There is no partial validation.

Measured proof that the gate is not currently clean: `py scripts/check-todo.py`
exits **1** on this tree, with 4 problems, all `wsl-toolkit: lane job still
kept`. That is a pre-existing operational state, not a record defect, and it
means **no wave-0 task can prove `py scripts/check-todo.py` exits 0 until an
operator runs `wsl-toolkit --instance podbox gc --job <id> --apply` for the four
named job ids.** This blocks the `Prove` of T-1501 to T-1514 and T-1540 to
T-1550 as written, and it is an operator action outside the fleet.

**Decision: serialise all `TODO/` writes behind T-1581.** 22 tasks, one writer.
The alternative — one agent per `TODO/` file — is a merge conflict generator
with no gate-level protection, because the counts block is a single derived
artefact with four coupled lines.

## 5. Direct answer: is a fleet of parallel implementer agents feasible?

**Not in the shape the plan proposes. Yes in a narrower one, and the narrow
version is worth running.**

The plan proposes six waves as units of work. A fleet cannot execute waves
because a wave is not a unit of independence: wave 4 alone is 8 binaries that
share a crate, a grammar module, a `Cargo.toml` row, a workflow and 6 `TODO/`
files. The right decomposition is 100 tasks, and that decomposition is
**43 batches**.

### Max safe fleet size

**19 concurrent agents on a clean tree, and 19 is only reachable during the
wave-0 and wave-6 window.** After T-1581 and T-1558 land, the sustainable
concurrency falls to **6**, and it never recovers above 6 in waves 2, 3 and 5.
The floor is **1** for the whole of T-1552.

19 is a real number, not a ceiling I chose. It is 8 wave-0 batches, 11 wave-6
deletions grouped by the file each touches, and 9 wave-2 unit tests that write
unshared files — of which 19 can be in flight with no pair sharing a path. After
the record-gate serialisation and the workflow serialisation, the graph's
maximum antichain is 6.

### Recommended batch structure

Five batches. Each names the serial core and the parallel remainder.

**Batch 0 — decisions and the cut set. 1 agent, serial, 3 tasks.**
T-1581, T-1558, T-1551. These three unblock 60 of the 92 remaining tasks. Run
them first and alone; they are decisions and one count refresh, and doing them
while the tree is clean costs nothing.

**Batch 1 — records and deletions. 8 agents, 19 tasks.**
The 8 wave-0 corrections grouped by file, plus the 11 wave-6 deletions, plus
the 9 wave-2 unit tests. **Constraint: the 22 `TODO/`-writing tasks inside this
batch still serialise behind T-1581.** So Batch 1 is really 8 agents, of which
only the 6 that write no `TODO/` file — T-1505 is one, T-1510, T-1511, T-1520,
T-1522, T-1524, T-1525, T-1526, T-1527, T-1528, T-1541, T-1544, T-1548 — run
fully parallel. The rest queue on a single `TODO/` writer.

**Batch 2 — the gate crate. 1 agent, serial, 8 tasks.**
T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578. One agent,
because they share a crate, a `Cargo.toml` row, `plant.sh`, and each one's
`Prove` is the record gate the previous one is deleting. **This is the serial
core. It cannot be batched to several agents without a merge conflict on
`crates/podbox-gate/src/` and a `cargo --workspace` resolution race on
`Cargo.lock`.**

**Batch 3 — the tool crates. 6 agents, 11 tasks.**
After Batch 2, split by crate: T-1559+T-1560 (buildstate, 1 agent),
T-1561+T-1564+T-1565+T-1566+T-1567 (release, 5 agents, one per file — but see
below), T-1562, T-1563, T-1568+T-1569 (podvm, 1 agent). T-1562 and T-1563
are excluded from the 5-way split: T-1562 needs the `110` inversion resolved
(section 4.2) and T-1563 needs T-1508. So Batch 3 is **4 agents**, not 6.

**Batch 4 — integration tests. 7 agents, 10 tasks.**
T-1530 and T-1531 serial (fixture before its two consumers), then T-1534,
T-1539, T-1574, T-1575 (4 agents, `podbox-image`), T-1532 then T-1533, T-1535,
T-1536, T-1538 (5 agents, `podbox-cli`), T-1537, T-1572, T-1573. **This batch
can start before Batch 2**, because it shares no file with the gate crate. It
is the one place where the plan's parallelism is real.

### The serial core that must go first

**Batch 0 and Batch 2.** 11 tasks, 11 of the 100. They are 11 percent of the
work and they gate 74 of the remaining 89. The other 89 tasks touch no file
either of them writes, and 74 of them can be batched to separate agents the day
Batch 0 lands.

### How much work sits outside the serial core

**89 of 100 tasks, 89 percent**, of which 79 can run at 6-wide concurrency and
10 need a decision first. If the operator wants a first useful result in one
session, **Batch 4 alone is a complete, shippable unit**: 10 tasks, 7 agents,
no gate-crate dependency, and it closes the plan's own named blocker (there is
no Rust OCI registry fixture in this tree) which PLAN.md section 6 lists as
blocking `crates/podbox-image/tests/store_digest.rs` and every registry-backed
integration test.

## 6. Acceptance proofs

Every proof is a runnable command. The host column says where it can be
**run**. §9 gives the exact `wslc` invocation that works from this host.

| Task | Proof | Runs on Windows host? |
| --- | --- | --- |
| T-1501 | `grep -c '^    Row {' <(sed -n '232,537p' crates/podbox-cli/src/parity.rs)` equals 265, and `py scripts/check-todo.py` exits 0 | yes |
| T-1502 | `sed -n '182p' TODO/cli.md` reads `40/40` | yes |
| T-1503 | `grep -c 'ascii' TODO/cli.md` and the entry names ten | yes |
| T-1504 | `grep -rn '1\.869' TODO/ experiments/results/` returns nothing, and `sed -n '133p' experiments/results/perf-kvm.txt` reads `0.935` | yes |
| T-1505 | `sed -n '6p' experiments/results/attribute.txt` reads `ESRCH`, and `git log --follow --oneline -- experiments/results/attribute.txt` names `6eb941f` | yes |
| T-1506 | `sed -n '334p;989p' crates/podbox-interpose/src/lib.rs` and the two `TODO/interpose.md` lines agree | yes |
| T-1507 | `sh experiments/157-lock-inheritance-prove.sh` exits 0 and its transcript shows the mutation landing, not `THE MUTATION DID NOT LAND` | **no** — needs `git`, `sed`, and the store test |
| T-1508 | `sh experiments/157-lock-inheritance-prove.sh; echo "EXIT:$?"` — the `mutate` output shows no `THE MUTATION DID NOT LAND` | **no** |
| T-1509 | `sed -n '434p' TODO/gate.md` names existing lines | yes |
| T-1510 | `sed -n '35p' experiments/README.md` no longer claims a passing KVM proof | yes |
| T-1511 | `sh experiments/370-windows-guest.sh` and `371-validationos-stream.sh` each write their own numbered result | **no** — needs a Windows guest |
| T-1512 | `py scripts/check-todo.py` exits 0 with T-1212's `Prove` matching its `:1037` transcript | yes |
| T-1513 | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` returns 30, and `TODO/image.md` says 30 | yes |
| T-1514 | `grep -n 'ENG_NETWORK' experiments/lib/engine.sh` returns 70, 294-297, 311 | yes |
| T-1515 | `cargo test -p podbox-ssh` exits 0, and `grep -rn '388-interactive-shell' TODO/` returns nothing | **no** — needs a link |
| T-1516 | `cargo test -p podbox-extract` exits 0, and `grep -rn '220-extract-path-safety' TODO/ docs/ .github/` returns nothing | **no** |
| T-1517 | `cargo test -p podbox-cli images::tests::rmi` exits non-zero with `in use` | **no** |
| T-1518 | `cargo test -p podbox-cli images::tests::prune` exits 0, prints `skipped:`, and the image stays listed | **no** |
| T-1519 | `cargo test -p podbox-cli system::tests::abi` — three cases returning 0, 1 and 2 | **no** |
| T-1520 | `cargo test -p podbox-enter abi::tests::nss_dispatcher` | **no** |
| T-1521 | `cargo test -p podbox-complete identity::tests::nss_live` | **no** |
| T-1522 | `cargo test -p podbox-extract drive::tests::want_overlay` | **no** |
| T-1523 | `cargo test -p podbox-complete write::tests::symlink_escape` | **no** |
| T-1524 | `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu memo::tests::check_g` | **no** |
| T-1525 | same manifest, `identity::tests::{a,b,c0}` | **no** |
| T-1526 | `cargo test -p podbox-image probe_cache::tests` | **no** |
| T-1527 | `cargo test -p podbox-cli parity::tests` | **no** |
| T-1528 | `cargo test -p podbox-probe exit::tests` | **no** |
| T-1529 | `cargo test -p podbox-cli lifecycle::tests windows::tests` | **no** |
| T-1530 | `ls crates/podbox-image/tests/` lists `space_precheck.rs` | yes |
| T-1531 | `cargo test -p podbox-image --test common` — the fixture binds a loopback port, serves a manifest GET, a blob GET and a blob HEAD, and shuts down | **no** |
| T-1532 | `cargo metadata --no-deps --format-version 1` reports `podbox-cli` targets, and the decision is recorded | yes for the decision, **no** for the re-metadata after `[lib]` |
| T-1533 | `cargo test -p podbox-cli --test curated_refusals` | **no** |
| T-1534 | `cargo test -p podbox-image --test acquisition` | **no** |
| T-1535 | `cargo test -p podbox-cli --test detached_stdio` | **no** |
| T-1536 | `cargo test -p podbox-cli --test store_gates` | **no** |
| T-1537 | `cargo test -p podbox-probe --test namespace` | **no** |
| T-1538 | `cargo test -p podbox-cli --test qol` | **no** |
| T-1539 | `cargo test -p podbox-image --test store_digest` | **no** |
| T-1540 to T-1549 | `py scripts/check-todo.py` exits 0, and `grep -rn '<script-name>' TODO/ docs/ .github/` returns nothing | yes |
| T-1550 | `grep -rn '10-build-target-image' TODO/ scripts/ experiments/` shows the decision, and `py scripts/check-todo.py` exits 0 | yes |
| T-1551 | `cargo test -p podbox-gate grammar::tests` — one characterization test per grammar, both green before any port | **no** |
| T-1552 | `./target/release/podbox-gate` exits 0 on the tree `scripts/check-todo.py` accepted, and `gate.yml:37` names the binary | **no** |
| T-1553 | `podbox-count --check` prints the counts the rows say | **no** |
| T-1554 | `podbox-plant` exits 0, and `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c Prove` returns 0 | **no** for the run, yes for the grep |
| T-1555 | `podbox-dev status` exits 0, and `grep -rn 'last-build-inputs' .` returns nothing | **no** |
| T-1556 | `podbox-smoke` exits 0 on a built binary | **no** |
| T-1557 | `podbox-interpose-build` prints `ok: exports 112 names, exactly what interpose.map declares` for both targets | **no** |
| T-1558 | the four `gate.yml` jobs and the `nightly.yml` build job are green; `grep -n 'check-todo.py\|plant.sh\|build-interpose.sh\|110-bloat-delta\|nightly-smoke\|package-ssh.sh\|release-notes.sh' .github/workflows/` names no deleted path | yes for the grep |
| T-1559 to T-1569 | `cargo test --workspace` exits 0, and `ls <new-crate>/src/` names the ported file | **no** |
| T-1570 | `grep -rn '394-ssh-package\|397-exported-build' TODO/packaging.md` names a crate, and `py scripts/check-todo.py` exits 0 | yes |
| T-1571 | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` and T-0215's fate is recorded | yes |
| T-1572 to T-1576 | `cargo test -p <crate> --test <name>` | **no** |
| T-1577 | `podbox-gate` interposer check exits 0, and `grep -A12 'exclude' Cargo.toml` does not gain a member for it | **no** |
| T-1578 | `scripts/plant.sh` `FILES` names every new test's file, and `podbox-plant` still exits 0 | **no** |
| T-1579 | each plant's red run is saved under `experiments/results/` and `podbox-plant` exits 0 on the clean tree | **no** |
| T-1580 | the four `gate.yml` jobs green on `ubuntu-latest` | yes, via CI |
| T-1581 | `py scripts/todo-count.py --check` exits 0 | yes |
| T-1582 | `awk -F'\t' 'NR>1{c[$3]++} END{for(v in c) print v, c[v]}' refactor/06-entries/verdict-ledger.tsv` returns SPLIT 35, DELETE 12, KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20 | yes |
| T-1583 | `awk -F'\t' '$3=="KEEP-SHELL"' refactor/06-entries/verdict-ledger.tsv \| wc -l` returns 29 after VC | yes |
| T-1584 | `sed -n '232,537p' crates/podbox-cli/src/parity.rs \| grep -c '^    Row {'` returns 265, and all four stale values in the tree are marked historical | yes |
| T-1585 to T-1596 | `py scripts/check-todo.py` exits 0, and each recorded fact is readable at the cited line | yes |
| T-1597 | the three counts in section 2 of the plan sum to 121, and `grep -c '' refactor/06-entries/verdict-ledger.tsv` returns 122 | yes |
| T-1598 | `grep -rn 'check-todo.py' .github/workflows/ docs/ AGENTS.md` names no deleted path | yes |
| T-1599 | `grep -c '^\s*"crates/' Cargo.toml` returns 9 today, 10 after T-1552, 13 after Batch 3 | yes |
| T-1600 | `grep -n 'crate-type' crates/podbox-interpose/Cargo.toml` still reads `["cdylib"]`, and no `tests/exported_set.rs` exists | yes |

**Proves that cannot run on this Windows host: 48 of 100.** They need a link,
and the host's `cargo` stops at `linker 'cc' not found`. They run in the
`wslc` lane per section 9, in about 30 seconds, or on `ubuntu-latest` in CI.

**Of those 48, 9 must NOT use the `wslc` lane as it stands**, because it runs as
uid 0 and they assert a degradation, a refusal or a `skipped:` path that root
never produces. They need a non-root uid in the lane, or the repo's own
`sh scripts/windows/run-in-base.sh`, or CI on `ubuntu-latest`. The 9 are
T-1517, T-1518, T-1520, T-1521, T-1529, T-1534, T-1539, T-1587, and T-1578.
This is measured, not inferred: it is the same class of failure as
`devices.rs:603`, and the panic message names the mechanism.

## 7. What I could not settle

1. **Whether the record gate's 4 pre-existing failures are the operator's to
   clear or a fleet task.** `py scripts/check-todo.py` exits 1 on this tree with
   four `wsl-toolkit: lane job still kept` problems naming three distinct job
   ids. `TODO/RULES.md` section 8 says `gc --apply` is used "only when every
   listed resource belongs to this session". I cannot tell whose session those
   three jobs belong to, so I did not run `gc`, and I did not clear them. **This
   blocks the stated `Prove` of 25 of the 100 tasks** until an operator decides.
2. **Whether the `110` inversion in section 4.2 is a plan defect or a
   deliberate sequencing I have mis-read.** I read `T-R005.md`'s Decision as
   placing `110` in wave 5, and `T-R004.md`'s Approach as moving all three path
   consumers in wave 4. I did not read `refactor/07-verify/verify-tool-crates-gate.md`
   in full, and it may already record this. I read its first 20 lines only.
3. **The exact per-file split of the 7 `plant.sh` `Prove` lines across
   `TODO/gate.md`, `TODO/deps.md` and `TODO/cli.md`.** My count says
   12 + 1 + 0. The plan says 13 across three files, and VC-5 in `PLAN.md`
   corrects a *different* claim about the same number. There are now three
   different statements of one count in the corpus and I resolved only mine.
4. **Whether `T-R006.md` is authorised work or a verification artefact.** It is
   untracked, like the rest of `refactor/`, and it is not in `PLAN.md` section 8.
   I treated it as a real wave because it carries all ten entry fields and it
   was produced by the audit process, but the operator may consider it a
   verification finding rather than a seventh plan entry. **If it is not
   authorised, 11 of the 100 tasks (T-1540 to T-1550) are out of scope and the
   fleet is 89 tasks.**
5. **`crates/podbox-supervise/src/nongoals.rs` and `report.rs` do not exist.**
   `PLAN.md:124` and `T-R002.md` clause 8 both cite
   `crates/podbox-supervise/src/nongoals.rs:253-302` and
   `report.rs:1069`. Measured: `crates/podbox-supervise/src/` holds only
   `launcher.rs`, `lib.rs` and `table.rs`; the files are in
   `crates/podbox-probe/src/`. `T-R002.md:70` names this exact error for three
   *other* paths and fixes them, but leaves these two unfixed. **Clause 8 is the
   only clause with no correct target, and it is the one clause that also has no
   fixture and no Rust arm** (T-R002.md:96). I left T-1586 as a record task
   rather than inventing a target.
6. **Whether a `[lib]` target for `podbox-cli` is acceptable.** T-R003 says
   black-box is the better proof and "an implementor may argue otherwise with
   the reason recorded". That makes T-1532 a decision task, which is how I
   modelled it. I did not decide it.
7. **The real size of the `podbox-gate` port.** 1,738 lines of Python with 28
   distinct checks, plus 532 lines of shell, plus 278, plus 237. T-R004 calls
   the crate `XL` and says the "row grammar and exit-code contract are named but
   not specified". I modelled that as T-1551 plus 8 port tasks. If the
   characterization tests in T-1551 reveal that the Python's checks are not
   separable into 28 independent units, the 8 port tasks become fewer and larger,
   and Batch 2's serial core grows.
8. ~~**Whether `cargo test --workspace` passes in the `wslc` lane today.**~~
   **Answered, and the answer corrects a claim I made in the paragraph above
   this one.** `cargo test --workspace --target x86_64-unknown-linux-gnu` in
   the `wslc` lane on 2026-10-01 gives `244 passed; 0 failed` for one crate,
   then **`test result: FAILED. 49 passed; 2 failed`** for `podbox-complete`,
   at `crates/podbox-complete/src/devices.rs:603` and `:629`.

   **It is the lane, not the tree.** I ran the failing test alone and the panic
   message names the cause:

   ```
   panicked at crates/podbox-complete/src/devices.rs:603:9:
   Fixup { entry: "T-0401", id: "dev-node", path: "dev/null", action: Created,
     detail: "a real character device 1:3. This machine permits mknod(2), so
     podbox made the node rather than a shim", degraded: false }
   ```

   The test asserts `f.degraded` and a shim `Action::Created`.
   `wslc run` executes as uid 0, `mknod(2)` succeeds for root, podbox correctly
   takes the **undegraded** path, and the test's premise — a host that denies
   `mknod` — is not arranged. So this is **not a pre-existing defect in the
   repository**, and I withdraw the claim I first wrote in this item:
   `cargo test --workspace` is green on `main` and red **only** under a root
   container. `gate.yml:199` runs it on `ubuntu-latest` as a non-root user,
   where the premise holds.

   ⛔ **The operational consequence is the opposite of the one I first drew,
   and it is the real finding: the `wslc` lane is a valid proving lane for link
   and run, and it is NOT a valid proving lane for anything that asserts a
   degradation path.** 9 of the 100 tasks assert a degradation, a refusal or a
   `skipped:` path — T-1517, T-1518, T-1520, T-1521, T-1529, T-1587, and the
   T-1534 and T-1539 of Batch 4 — and each needs either a non-root uid in the
   lane or the repo's own `run-in-base.sh`. **T-1532's decision record should
   carry this**, because it is a fixture fact that changes what an integration
   test may assume about its own environment.

   I did not verify whether `wslc` can run at a non-root uid; `wslc run --rm`
   reported `id -u` as `0` and I did not read `wslc --help` for a user flag.
   **Whether the lane can be made non-root is open, and it decides how much of
   Batch 4 may use the fast lane.**

   Two further observations from the same run, both worth recording. 21 lines of
   `error: Unrecognized option: 'probe-child'` come from
   `crates/podbox-probe/src/lib.rs:43`'s `CHILD_FLAG` being passed to a binary
   whose `main` does not accept it — a real mismatch between the flag's
   declaration and its only call site. And `WS_TEST_EXIT=0` in my own log is
   `grep`'s exit status rather than cargo's; I read it wrong at first and
   corrected it. Both are the class of error this audit has already found
   twice: a number or a status read from the wrong place.

## 8. The one thing the plan gets wrong that changes the schedule

Section 4.2: **wave 4 deletes the two readers of a file that wave 5 moves.**
`scripts/check-todo.py:228` and `scripts/plant.sh:191` both read
`experiments/110-bloat-delta.sh`, and both are wave-4 subjects that wave 4
deletes. `T-R005.md:98` says the path changes "land in wave 4", but the file
itself is a wave-5 binary per T-R005's own Decision at `:37`. As written, waves 4
and 5 cannot both land, in either order:

- **Wave 4 then wave 5**: wave 4's `podbox-gate` and `podbox-plant` read a path
  that wave 5 has moved, or a shell file that no longer exists.
- **Wave 5 then wave 4**: wave 5's `podbox-size` moves `110` while
  `check-todo.py:228` and `plant.sh:191` still name the old path, so
  `py scripts/check-todo.py` exits non-zero on check 17 and `plant.sh` exits 2
  on its `CEILING_NUM` guard.

This is one file, and hoisting `110` to wave 4 fixes it. That is a three-row
change to the plan and it is the amendment I would make before starting Batch 2.

## 9. Host facts, measured in this session

Every claim here is a command I ran, with its result. The two the plan asserts
and this session contradicts are marked.

| Fact | Command | Result |
| --- | --- | --- |
| Native Windows check works | `cargo check -p podbox-probe` | **exit 0** — confirms the assignment's host fact |
| `wslc` is installed, off PATH | `ls "/c/Program Files/WSL/wslc.exe"` | present, 9,040,184 bytes, Sep 25 23:39 |
| `wsl-toolkit` version | `wsl-toolkit --version` | 6.0.0 |
| `wsl.exe` **is** on PATH | `which wsl.exe` | `/c/Windows/system32/wsl.exe` — `docs/containers.md` forbids calling it with a payload; it is present, so the rule must be followed rather than assumed absent |
| `wslc` runs as root with a real `cc` | `wslc run --rm -v ...:/work -w /work docker.io/library/rust:1.98.1-bookworm sh -c 'id -u; which cc'` | `0` and `/usr/bin/cc` |
| **`wslc` builds the crate** | same, `cargo check -p podbox-probe` | `Finished dev profile in 3.36s` — ⛔ the plan's "nothing compiles on this host" is false for this lane |
| **`wslc` links test binaries** | same, `cargo test -p podbox-probe --no-run` | `TESTNORUN_EXIT=0`, executable produced for `x86_64-unknown-linux-musl` |
| **`wslc` runs the suite** | same, `cargo test -p podbox-probe --target x86_64-unknown-linux-gnu` | `test result: ok. 108 passed; 0 failed` — ⛔ the plan's claim is false here |
| ⛔ **but the lane runs as root** | `wslc run --rm … cargo test --workspace --target x86_64-unknown-linux-gnu` | `244 passed; 0 failed`, then `test result: FAILED. 49 passed; 2 failed` in `podbox-complete` at `devices.rs:603` and `:629` |
| ⛔ **the cause is uid 0, not the tree** | same, `cargo test -p podbox-complete --target x86_64-unknown-linux-gnu devices::tests::a_symlink` | panics with `detail: "a real character device 1:3. This machine permits mknod(2), so podbox made the node rather than a shim", degraded: false` — the test asserts the shim path a root host never takes |
| `MSYS_NO_PATHCONV=1` is required | without it, Git Bash rewrites `-w /work` | the assignment's host fact, confirmed by the script's own header at `run-in-base.sh:22-27` for the *other* lane |
| The default target is musl | `grep -A1 '^\[build\]' .cargo/config.toml` | `target = "x86_64-unknown-linux-musl"` — so a bare `cargo test` in the `wslc` lane builds but does not run; `--target x86_64-unknown-linux-gnu` is required |
| **The record gate is red now** | `py scripts/check-todo.py` (unpiped) | **exit 1**, 4 problems, all `wsl-toolkit: lane job still kept` |
| The gate's own counts are consistent | same output, second line | `203 rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done` — the failures are operational, not record defects |
| Parity table is 265 rows | `sed -n '232,537p' crates/podbox-cli/src/parity.rs \| grep -c '^    Row {'` | **265** — confirms T-R000 correction 1 |
| A whole-file count gives 267 | `grep -c 'Row {' crates/podbox-cli/src/parity.rs` | 267 — the error class the audit names, reproduced |
| `store.rs` holds 30 tests | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` | **30** — confirms T-R000 correction 13 |
| `1.869` appears nowhere | `grep -rn '1\.869' experiments/results/ TODO/` | only `TODO/gate.md:1431`, the wrong record — confirms T-R000 correction 4 |
| No `guest.tcg.boot` row | `grep -n 'guest.tcg.boot' experiments/results/perf-kvm.txt` | no match — confirms T-R000 correction 4 |
| The two result names are swapped | `sed -n '193p' experiments/371-…`, `sed -n '205p' experiments/370-…` | `windows-365.txt` and `windows-364.txt` respectively — confirms T-R000 correction 11 |
| `attribute.txt:6` is ESRCH | `sed -n '6p' experiments/results/attribute.txt` | `kcmp(-1,-1,...) [control] FAIL errno=3 ESRCH` — confirms T-R000 correction 5 |
| `plant.sh` `Prove` count is 13 | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c 'Prove'` | **13**, in `TODO/gate.md` (12) and `TODO/deps.md` (1); `TODO/cli.md` contributes 0 — ⛔ T-R004:99 says three files |
| `110-bloat-delta.sh` has three code consumers | `grep -rn '110-bloat-delta' .github/ scripts/` | `gate.yml:131`, `check-todo.py:228`, `plant.sh:191-193` — confirms T-R005, plus 10 `Prove` lines in `TODO/deps.md` |
| `check-todo.py` `Prove` lines are in 10 files | `grep -rln 'check-todo.py' TODO/*.md` | INDEX, RESUME, RULES, cli, deps, gate, interpose, milestones, packaging, podssh — ⛔ wider than the plan's shape implies |
| The workspace has 9 members | `grep -c '^\s*"crates/' Cargo.toml` | **9**, with `exclude = ["crates/podbox-interpose"]` at `:20` — confirms the plan's section 3 correction |
| A second workflow consumes 4 ported subjects | `grep -n 'build-interpose\|nightly-smoke\|package-ssh\|release-notes' .github/workflows/nightly.yml` | `:84`, `:103`, `:112`, `:179` — ⛔ not mentioned in any entry |
| `nightly.yml` has no concurrency group | `sed -n '18,21p' .github/workflows/gate.yml` | `gate.yml` has `concurrency: group: gate-${{ github.ref }}, cancel-in-progress: true`; `nightly.yml` has none |
| `podbox-supervise` holds no `nongoals.rs` | `ls crates/podbox-supervise/src/` | `launcher.rs`, `lib.rs`, `table.rs` only — ⛔ `PLAN.md:124` cites a file that is in `podbox-probe` |
| Highest existing `TODO/` id | `grep -rhoE 'T-1[0-9]{3}' TODO/*.md \| sort -u \| tail -1` | **T-1423** — so T-1501 is free |
| The ledger is 121 rows, pre-VC | `awk -F'\t' 'NR>1{c[$3]++} END{for(v in c) print v, c[v]}' verdict-ledger.tsv` | SPLIT 35, DELETE 12, KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20 — ⛔ KEEP-SHELL is **27**, not the 29 the plan's post-VC section 2 reports, and DELETE is 12, not 11 |

**The ledger's counts do not match the plan's post-VC table, and the difference is
in the two verdicts VC touched.** The plan's section 2 table says KEEP-SHELL 29
(27 + the two VC added, `162` and `95`) and DELETE 11 (12 minus `162`). The
ledger is **pre-VC** and `PLAN.md:22-27` says so explicitly, warning that a
reader who trusts it lands three scripts in the wrong wave. So the two numbers
reconcile once the three VC edits are applied to the ledger, and **T-1582 and
T-1583 exist to make the ledger post-VC**, which nothing in the seven entries
currently does. Until they land, every implementor who counts from the ledger
gets 27 and 12, and every implementor who counts from `PLAN.md` section 2 gets
29 and 11, and neither is wrong about the file they read.

## 10. Provenance

Read in full: `PLAN.md`, `T-R000.md` through `T-R006.md`,
`verdict-ledger.tsv`, `TODO/INDEX.md`, `TODO/RULES.md`, `TODO/PROGRESS.md`,
`TODO/CLAUDE.md` is not in this tree and was not read, `docs/methodology/authoring.md`,
`Cargo.toml`, `.cargo/config.toml`, `scripts/windows/run-in-base.sh` (header),
`scripts/check-todo.py` (the 10 regions cited), `scripts/plant.sh` (`:45-60`,
`:185-200`), `scripts/todo-count.py` (header), `crates/podbox-cli/Cargo.toml`,
`crates/podbox-ssh/tests/common.rs` (header), `crates/podbox-probe/src/sys.rs:810-820`,
`crates/podbox-image/src/registry.rs:945-960`, `crates/podbox-probe/src/exit.rs` (header),
`crates/podbox-cli/src/images.rs:955-990`, `experiments/157-lock-inheritance-prove.sh:110-160`,
and the cited lines of 14 `TODO/` files, 16 Rust files, 5 experiment scripts and
2 workflows.

Read partially: `refactor/07-verify/*.md` (the 20-line headers of all five),
`refactor/recon-wslc.md` (the 40-line header), `experiments/355-parity-curated.sh:104-108`.

Not read: the ten `refactor/01-audit/group-*.md` bodies in full, the peer-review
and meta reports in full, `docs/conventions/prose.md`, `docs/conventions/code.md`,
`docs/methodology/gate.md`, `docs/containers.md`, `TODO/RECONCILIATION.md` if it exists.
Where this document claims a convention I took it from the entry that states it
or from the source line, not from those documents.

**This document is untracked and committed with nothing else.** No existing file
was modified. One scratch file was written to `.tmp/` and deleted in the same
session; `git status --porcelain` shows only `?? refactor/`.
