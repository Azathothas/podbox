# recon-b: is the shell-retirement plan followable?

This is the third adversarial pass over `refactor/06-entries/`. The first two
found the authors' own errors twice. This pass found more.

Scope: `PLAN.md`, `T-R000.md` .. `T-R006.md` (SEVEN entries, not six),
`verdict-ledger.tsv`, and the machinery they depend on. Every claim below has a
command. Commands were run on this Windows host with Git Bash and Python. Where a
claim needed Linux, I say so and name the substitute I used.

Baseline facts this pass measured, not assumed:

- `python scripts/check-todo.py` on this tree: exit 1, four `wsl-toolkit` lane-job
  findings from check 29, and `203 rows, 203 entries`. The gate is red before
  any plan work starts. This is host state, not a plan defect.
- `refactor/` is **untracked**: `git status --porcelain refactor` reports
  `?? refactor/`, and `git ls-files refactor` returns 0 files. The plan is
  invisible to the gate that it reorganises. See blocker B-01.
- The plan's own 121 figure is exact: `ls experiments/*.* scripts/*.* | grep -E
  '\.(sh|py|ps1)$' | sort -u | wc -l` returns 121, and `comm` against the ledger
  shows zero rows on either side of the difference. The scoping warning at
  PLAN.md:29 is correct.

## Blocker table

| id | wave | blocker | class | evidence (command + line) | minimum fix | in scope? |
| --- | --- | --- | --- | --- | --- | --- |
| B-01 | all | The whole plan is untracked, so `check_tree` never reads it and no entry's citation is held. Every statement below about what the gate "notices" is about `TODO/`, not about the plan. | TREE | `git ls-files refactor \| wc -l` -> 0; `git status --porcelain refactor` -> `?? refactor/` | Track the seven entries, or state that the plan is a scratch artefact and that no `Prove` in it is a `TODO/` `Prove`. | outside: a records decision, not code |
| B-02 | 4 | The `Prove` `./target/release/podbox-gate` names a binary the `todo` job never builds. `todo` runs **zero** cargo commands and has no `bootstrap-env.sh`, so the binary it must run does not exist on that runner. The entry's own replacement text is unrunnable as written. | SELF-REFERENTIAL | `awk '/^  todo:/,/^  build:/' .github/workflows/gate.yml \| grep -c 'cargo\|bootstrap-env'` -> 0 | Add a build step to the `todo` job before the check-todo step, or move the check into `build` and prove it there. See the exact text in the UNRESOLVED-DECISION section. | inside |
| B-03 | 4 | `check-todo.py` check 17 reads the ceiling as `^CEILING_BYTES=(\d+)$` **out of `experiments/110-bloat-delta.sh`**. Delete that file for a Rust binary and the declaration grammar is gone. The port keeps the check only by inventing a shell-shaped file, which check 17's own "declared in exactly one place" rule then holds against the binary. | TREE | `sed -n '228,229p' scripts/check-todo.py`; and against `experiments/110-bloat-delta.sh` the regex matches `CEILING_BYTES=8000000`, against `pub const CEILING_BYTES: u64 = 8000000;` it matches nothing | Pick one home before wave 4 and write it in the entry: either `110`'s replacement keeps a machine-readable `CEILING_BYTES=` line, or check 17 is rewritten to read the number from the binary's `--print-ceiling`. The second changes the check, which T-R004:45 forbids. | inside |
| B-04 | 4 | Check 23 is the same defect for the interposer: `INTERPOSE_CEILING_BYTES=(\d+)$` read out of `scripts/build-interpose.sh`, which wave 4 deletes. No entry mentions this ceiling. | TREE | `sed -n '808,810p' scripts/check-todo.py`; `grep -l 'INTERPOSE_CEILING' refactor/06-entries/*.md` -> no file | Same decision as B-03, for `scripts/build-interpose.sh:48` (`INTERPOSE_CEILING_BYTES=500000`). | inside |
| B-05 | 4 | Check 24 requires the literal strings `interpose.map` and `nm -D --defined-only` **inside `scripts/build-interpose.sh`**. Wave 4 deletes that file, so the check errors with "is not tracked" whatever the port does. The entry assumes the export check is free. | TREE | `sed -n '895,910p' scripts/check-todo.py` | Repoint `INTERPOSE_BUILD` at the ported binary's source, and re-word the check to look for the export-set comparison in whatever the binary does. This is a check rewrite, which the entry forbids itself. | inside, but it contradicts T-R004:45 |
| B-06 | 4 | Check 25 requires `scripts/dev.sh` to exist and to contain `case "$step_rc" in` plus a `\n\t\t2)` arm naming `SKIP`. Wave 4 folds `dev.sh` into `podbox-dev` and deletes it. No entry mentions check 25. | TREE | `sed -n '813,814p'` and `sed -n '928,942p' scripts/check-todo.py`; `grep -l 'check 25' refactor/06-entries/*.md` -> no file | Same as B-05. The `2)`-arm check is a **shell-grammar** assertion; a Rust binary has no such text, so the check must change shape. | inside, contradicts T-R004:45 |
| B-07 | 4 | Check 31 runs `scripts/document-state.py` as a subprocess. Wave 5 makes `document-state` a **binary** and deletes the `.py`. The entry names `document-state.py` as a subject and never names this check. `OSError` is caught, so the failure mode is an `err()`, not a crash, which makes it a quiet red. | TREE | `sed -n '1698,1710p' scripts/check-todo.py`; `grep -l 'check 31' refactor/06-entries/*.md` -> no file | Name check 31 in T-R005 and repoint it at the new binary. | inside |
| B-08 | 2 | T-R002's plant requirement is not implementable as written, and the entry admits it but leaves the choice open. `plant.sh:51` `FILES` names **no** source file that any of the 15 clauses targets. `FILES` carries exactly three `crates/` paths: `podbox-supervise/src/lib.rs`, `podbox-cli/src/parity.rs`, `podbox-cli/src/run.rs`. | UNRESOLVED-DECISION | `sed -n '51p' scripts/plant.sh \| tr ' ' '\n' \| grep '^crates'` -> those three only; checked `images.rs`, `system.rs`, `abi.rs`, `identity.rs`, `drive.rs`, `memo.rs` -> all NO | See the exact decision text. Do not start wave 2 first. | inside |
| B-09 | 2 | Adding the new source files to `FILES` makes `plant.sh` **refuse to start** when any of them is dirty, and its `backup`/`restore` copies every entry. Adding `crates/podbox-cli/src/images.rs` means an ordinary edit to that file blocks the plant step at `gate.yml:56`. | TREE | `sed -n '74,79p'` (the `git diff --quiet -- $FILES` guard) and `sed -n '84,90p'` (backup and restore iterate `$FILES`) | Decide with B-08: a separate plant script for wave 2 keeps `FILES` stable and avoids widening the dirty-tree guard to every file under test. | inside |
| B-10 | 5 | T-R005 says wave 5 shares "no crate, fixture or dependency edge with" wave 4. It shares **two files**: `gate.yml:186` and the `check-todo.py` ceiling/interpose inputs. Wave 5 deleting the last `scripts/*.py` empties that glob. | FALSE (as stated) | `sed -n '3,6p' refactor/06-entries/T-R005.md` against `sed -n '186p' .github/workflows/gate.yml`; `ls scripts/*.py` today is five files, of which wave 4 removes two and wave 5 removes three | Correct the isolation claim, and assign the `py_compile` glob to one owner. See the ordering section. | inside |
| B-11 | 5 | `gate.yml:186` fails when **all** of `scripts/*.py` are gone. Verified in a scratch directory with the same shell: after the last `.py` is removed, bash leaves the pattern literal and `py_compile` exits 1 with `[Errno 22] Invalid argument: 'scripts/*.py'`. After wave 4 alone the glob still matches three files, so the step passes then and breaks only at wave 5. T-R004 names this step but scopes the fix to wave 4's two files. | TREE | scratch test: `rm scripts/b.py; bash -c 'python -m py_compile scripts/*.py'` -> `pycompile_exit=1`; `ls scripts/*.py` -> 5 files now | One owner, in the wave that empties the glob. T-R004:148 already claims the step; extend it to wave 5's three deletions. | inside |
| B-12 | 5 | Deleting `experiments/398-gate-diagnostics.py` breaks `gate.yml:205` unless that step is repointed. T-R004 names `398` as `podbox-smoke`'s subject and lists `:205` as a `test` step, but **never links them**. It cites `build-interpose.sh:204-205` at `:107`, which reads as if it covered the workflow line. | TREE | `grep -n '205' refactor/06-entries/T-R004.md` -> only the step table at `:142` and the unrelated `:107`; `sed -n '205p' .github/workflows/gate.yml` | Add one line to T-R004: `gate.yml:205` becomes the ported diagnostics binary in the same change. | inside |
| B-13 | 5 | `experiments/393-build-freshness.py` runs at `gate.yml:202` and `scripts/dev.sh:222`. T-R005 says `393` is "tested by" `podbox-buildstate` and never deletes or repoints it. The same holds for `scripts/dev.sh:222` and `scripts/dev.sh` itself in wave 4. | TREE | `sed -n '222p' scripts/dev.sh`; `sed -n '202p' .github/workflows/gate.yml`; `grep -rn '393-build-freshness' refactor/06-entries/*.md` -> one hit, T-R005:55, and it assigns no owner | Either keep `393` and say so, or repoint both call sites in the same change. | inside |
| B-14 | 1 | T-R001 says deleting `220` "leaves six `Prove` or citation references to repair across `TODO/`". There are **six** `TODO/` sites, which is what the entry means, but the tree carries **ten** tracked sites, two of them in `crates/` source and one in `experiments/85`. The two `crates/` sites are doc comments naming a script that will not exist. | TREE | `git grep -n '220-extract-path-safety' \| grep -v '^docs/history/' \| wc -l` -> 10; per file: `TODO/extract.md` 4, `TODO/milestones.md` 2, `TODO/image.md` 1, `crates/podbox-extract/src/drive.rs:543`, `crates/podbox-extract/src/lib.rs:278`, `experiments/85-completion-symlink-escape.sh:20` | Widen the instruction from `TODO/` to every tracked site and list the three non-`TODO/` files by line. | inside |
| B-15 | 1 | T-R001's `Prove` for the `220` deletion is `cargo test --workspace`, which reaches **no** `220` clause. The six tests are `#[cfg(test)]` unit tests in `podbox-extract`; the entry itself says the proof does not establish the claim. The wave's proof therefore rests entirely on a table in prose. | UNRESOLVED-DECISION | `sed -n '134,143p' refactor/06-entries/T-R001.md` states this; the `Prove` at `:56` is unchanged | Either accept the prose table as the closure record and say so in the `Prove` line, or delete `220` only after its clauses have plants. See the decision text. | inside |
| B-16 | 4 | T-R004:100 says the 13 `Prove` lines sit "across three `TODO/` files". The count 13 is right; the file count is **two**. `TODO/cli.md` contributes 0 to `Prove` lines (its single mention, `:1251`, is prose). This is the same claim PLAN.md:182 corrected once already, uncorrected a second time in the entry. | FALSE | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c 'Prove'` -> 13; per-file Prove counts -> `TODO/gate.md` 12, `TODO/deps.md` 1, `TODO/cli.md` 0 | Change "three" to "two" and name the files. | inside, one-word fix |
| B-17 | 4 | `scripts/plant.sh` is not named by 13 `Prove` **lines** and nothing else. It is named in **37** places under `TODO/` and 11 under `docs/`. What must repoint is not 13 lines. | TREE | `grep -rn 'scripts/plant.sh' TODO/ \| wc -l` -> 37; `grep -rn 'scripts/plant.sh' docs/ .github/ \| wc -l` -> 11 | State the true repair set: 13 `Prove` lines, plus every bare `` `scripts/plant.sh` `` citation, plus `gate.yml:56`. Under check 14 a bare citation is the one that turns the gate red on delete; a `sh`-prefixed one is not. | inside |
| B-18 | 2 | The bare-versus-`sh` distinction the plan rests on is real, and it cuts the other way for `220`. A **bare** `` `experiments/…` `` citation is caught by check 14. A `` `sh experiments/…` `` or `` `./experiments/…` `` citation is **not caught at all**, in either direction. So deleting `220` while its `./experiments/220-…` `Prove` lines stand leaves the gate **green**. | TREE | ran the gate's own `BARE` regex at `:203-206` over six spellings: bare `experiments/388…` CAUGHT; `` `sh experiments/388…` ``, `` `./experiments/220…` ``, `` `sh experiments/389…` ``, `` `./scripts/plant.sh` ``, `` `py scripts/check-todo.py` `` all NOT CAUGHT | Grep both forms separately before and after each deletion, as T-R006:85 says. The plan states the rule for wave 1's `388` but never applies it to `220`, whose six sites are all `./` or bare-prose. | inside |
| B-19 | 1 | The `388` replacement text T-R001 proposes does not satisfy check 14's purpose either. It cites `crates/podbox-ssh/tests/session_interactive.rs`, which resolves, so it is correct; but note the range is wrong. The entry says 12 tests at `:255-482`. The file has 12 `#[test]` at `:254` through `:471`. | FALSE (minor) | `grep -c '#\[test\]' crates/podbox-ssh/tests/session_interactive.rs` -> 12; line numbers 254,275,287,303,322,350,373,388,409,430,449,471 | Correct the range to `:254-471`. | inside, one-line fix |
| B-20 | 3 | T-R003 picks black-box over `[lib]` for `podbox-cli` in prose ("This entry says so; an implementor may argue otherwise") but five of the seven test files it lists are `podbox-cli` tests, and three of those (`curated_refusals.rs`, `store_gates.rs`, `qol.rs`) assert internal state rather than driving a binary. Black-box cannot express them. | UNRESOLVED-DECISION | `sed -n '60,72p' refactor/06-entries/T-R003.md` against its own table at `:95-104` | Name per file which shape each one takes. See the decision text. | inside |
| B-21 | 5 | `experiments/394-ssh-package.sh` and `experiments/397-exported-build.py` are RUST-TOOL and unassigned; the entry says so. It misses that `394` is **called by `scripts/dev.sh:222`**, so it is a live caller, not only a missing assignment. Wave 4 deletes `dev.sh`, so the coupling and the gap land in the same change. | TREE | `sed -n '222p' scripts/dev.sh` -> `"sh experiments/394-ssh-package.sh" \` | Resolve in wave 4, not wave 5: the `dev.sh` deletion is what removes the caller. | inside |
| B-22 | all | 48 of the 121 scripts are named in no entry. 21 are KEEP-SHELL (legitimately absent), but **22 SPLIT and 5 RUST-TEST** are unassigned. `PLAN.md:44` says every SPLIT has a Rust half, so these 27 are retirement work with no wave. `151-spawn-ambiguity.sh` is one of the five and PLAN.md:187 explicitly re-affirms its SPLIT label. | TREE | loop over the ledger comparing each basename against the concatenation of the seven entries: 48 unmatched; by verdict 22 SPLIT, 21 KEEP-SHELL, 5 RUST-TEST. The five RUST-TEST: `130-probe-parity.sh`, `151-spawn-ambiguity.sh`, `30-attribution-census.sh`, `368-run-decay.sh`, `95-podman-vfs-ignorechown.sh` (the last is KEEP-SHELL post-VC-3, so **4** real gaps) | Either add the 26 to a wave or state the retirement stops short of them. As written `PLAN.md:49`'s "81 scripts retire" has no implementation for 26 of them. | outside: a scope decision |
| B-23 | all | The plan's own verdict table disagrees with its own ledger in four of five rows, and the authors flagged the ledger as PRE-VC without re-deriving the table from it. | FALSE | `tail -n +2 verdict-ledger.tsv \| cut -f3 \| sort \| uniq -c` -> SPLIT 35, RUST-TOOL 27, KEEP-SHELL 27, RUST-TEST 20, DELETE 12. `PLAN.md:42-46` claims 29 / 36 / 27 / 18 / 11 | Reconcile. Either the table is post-VC and the ledger needs updating, or the table is stale. `PLAN.md:22` warns about exactly this failure and then reproduces it. | inside |
| B-24 | all | `PLAN.md` describes six waves and `T-R005.md:3` calls itself "the sixth and last entry", but `T-R006.md` exists and carries the twelve DELETE scripts. `PLAN.md:198` still says `T-R000.md` .. `T-R005.md`. `T-R005` and `T-R006` both number themselves "sixth". | FALSE | `ls refactor/06-entries/T-R00*.md` -> 7 files; `sed -n '198p' refactor/06-entries/PLAN.md`; `sed -n '3p' refactor/06-entries/T-R005.md`; `sed -n '1p' refactor/06-entries/T-R006.md` | Update `PLAN.md` §5 and §8 to seven waves and renumber `T-R005`/`T-R006`. | inside |
| B-25 | 6 | T-R006 deletes 11 scripts (its own table) but its `Source` cites "12 rows, `verdict` = `DELETE`" and its title says twelve. The twelfth is `162`, refuted by VC-2 and correctly retained, so the table is right and the title and `Source` are wrong. | FALSE (minor) | `sed -n '52,62p' refactor/06-entries/T-R006.md \| grep -c '^\| \`experiments'` -> 11; ledger DELETE rows -> 12, of which `162` is retained | Retitle to eleven and correct `Source`. | inside, two-word fix |
| B-26 | 1 | `T-R002` clause 2 asserts `image prune` "exits 0, prints `skipped:`, leaves the image listed" and calls the source a defect because `experiments/results/store-gc.txt` records `prune_skipped 1`. The result file agrees with the source. `160-store-gc.sh:138` asserts `prune_rc -eq 0` and `:139` asserts `skipped:` appears. There is no defect. | FALSE | `sed -n '961,986p' crates/podbox-cli/src/images.rs` -> returns `0` on `Ok(done)`; `sed -n '138,139p' experiments/160-store-gc.sh` -> asserts 0 and `skipped:`; `experiments/results/store-gc.txt` -> `prune_skipped 1`, `images_left 1`, all consistent | Delete the "source defect" framing from T-R002:90-94. The clause is already satisfied and its test can be written straight. | inside |
| B-27 | 4 | `plant.sh:46` refuses to start unless `scripts/check-todo.py` **is executable**. Wave 4 deletes that file. If the port also deletes it before `plant.sh` is ported, `plant.sh` exits 2 and `gate.yml:56` skips the whole harness, silently. | SELF-REFERENTIAL | `sed -n '43,46p' scripts/plant.sh`: `GATE="$ROOT/scripts/check-todo.py"` then `[ -x "$GATE" ] \|\| exit 2` | `podbox-plant` must exist and be executable before `scripts/check-todo.py` is deleted, in the same commit that changes `gate.yml:56`. | inside |
| B-28 | 2 | The gate already exits 1 on this host (check 29, four kept lane jobs). `plant.sh:113-117` SKIPs with exit 2 when the baseline gate is red. So on this machine **every** plant reads SKIP, and wave 2's "each plant's red run is its own evidence" cannot be produced without first clearing the lane. | HOST | `python scripts/check-todo.py; echo $?` -> 1, four `wsl-toolkit: lane job still kept` lines; `sed -n '113,117p' scripts/plant.sh` | `wsl-toolkit --instance podbox gc --job <id> --apply` for the four ids, or run wave 2 on the Linux lane where the lane report is absent. | inside: housekeeping, stated as a blocker |
| B-29 | 2 | Neither Linux lane is currently usable from here, so the plan's own escape hatch does not open. | HOST | `sh scripts/windows/run-in-base.sh /tmp/rb/j1.sh` -> exit 2, `Error: current system boot ID differs from cached boot ID`; `wslc.exe image ls` -> hung past 60 s and was killed | Clear `/tmp/wsl-toolkit-run-1000/containers` and `.../libpod/tmp` inside the base, or use `wslc` with a bounded wait. Until then every `cargo test` in waves 1 to 5 is unverifiable locally and the gate is the only proof. | outside: host repair |
| B-30 | 5 | `scripts/build-interpose.sh` is a wave-4 subject and `experiments/360-perf-harness.sh` is too (`PLAN.md:75`), but check 30 reads `experiments/perf-ceilings.tsv` and three result files, and no entry names which binary now writes them. | UNRESOLVED-DECISION | `sed -n '1376,1379p' scripts/check-todo.py`; `grep -l 'perf-ceilings' refactor/06-entries/*.md` -> no file | Name the writer of `experiments/perf-ceilings.tsv` and the three result files in whichever wave owns `360`. | inside |

### Class counts

| class | count |
| --- | --- |
| TREE | 21 |
| HOST | 2 |
| SELF-REFERENTIAL | 3 (B-02, B-27, and B-05/B-06 by extension) |
| UNRESOLVED-DECISION | 4 (B-08, B-15, B-20, B-30) |
| FALSE | 6 (B-10, B-16, B-19, B-23, B-24, B-25, B-26) |
| TREE + SELF-REFERENTIAL overlap | 2 (B-05, B-06) |

## The five most dangerous

1. **B-03 / B-04, the ceiling declarations.** Wave 4 deletes both files that own
   the repository's two size ceilings, and both checks read the value with a
   shell-grammar regex against those files. Nothing in the plan notices. The
   first wave-4 commit takes the gate red through checks 17 and 23, and the
   entry's own `Prove` is the tool that reports it. This is the deepest one:
   T-R004:45 forbids changing what the gate accepts, but the port cannot preserve
   check 17 at all unless a file with a `CEILING_BYTES=` line survives.

2. **B-02, the replacement `Prove` cannot run.** `./target/release/podbox-gate`
   is the correct shape of command, but the `todo` job has no toolchain and no
   build step. The plan's replacement for its own self-referential `Prove` is
   therefore also unrunnable. The entry names the defect and then supplies a fix
   that inherits it.

3. **B-05 / B-06 / B-07, three checks that read the deleted files as text.**
   Check 24 wants two literal strings, check 25 wants a `case` arm and a `\t\t2)`
   tab, check 31 runs a `.py` that wave 5 deletes. All three are silent-red:
   they `err()` rather than crash. An implementor following the entry will see a
   red gate with a message naming a file the plan says it already replaced.

4. **B-22, 26 retirement scripts with no wave.** The plan claims 81 scripts
   retire. 26 of them, 22 SPLIT plus 4 RUST-TEST, are named in none of the seven
   entries. The plan's own coverage was verified by presence in prose, and the
   three SELF-TEST gaps repeat.

5. **B-26, a "source defect" that is not one.** T-R002 would send an implementor
   to write a test for the wrong reason and open a defect entry against correct
   code. The result file, the source, and the original script's assertion all
   agree. It is the clearest FALSE in the plan and it sits on a clause the wave
   is required to deliver.

## UNRESOLVED-DECISION sections

Each decision the entry hands to an implementor without settling, with the text
the plan should have written.

### B-08, wave 2's plant mechanism

T-R002:136-142 states the problem correctly and then says "An implementor must
choose one and record it." The choice has a consequence the entry does not name:
option one widens `plant.sh`'s dirty-tree guard (B-09).

Replace the closing paragraph with:

> **Decision: wave 2's plants land in a new script, `scripts/plant-integration.sh`,
> and not in `scripts/plant.sh`.** `FILES` at `scripts/plant.sh:51` is the set the
> harness owns end to end: it copies each entry before a case and restores each
> after, and it refuses to start when any entry is dirty
> (`scripts/plant.sh:74-79`). Adding the seven new source files widens that guard
> to every file wave 2 writes tests into, so an ordinary edit to
> `crates/podbox-cli/src/images.rs` would SKIP the whole harness at
> `gate.yml:56`. A second script carries the same harness shape with its own
> `FILES` and the same "did the mutation land" hash, so the invariant is
> preserved and the guard is not widened. It is transitional: `plant.sh` is
> ported in wave 4, and the second script's cases move into `podbox-plant` with
> it. Recorded here so wave 2 does not reopen the question.

### B-15, wave 1's `Prove`

T-R001:134-143 says the `Prove` does not establish the claim and then leaves the
`Prove` unchanged. The entry must choose.

Replace the `Prove` line with:

> **Prove:** `cargo test -p podbox-extract --lib` exits 0, and
> `cargo test -p podbox-ssh` exits 0; `py scripts/check-todo.py` exits 0 with no
> bare citation of `experiments/388-interactive-shell.sh` or
> `experiments/220-extract-path-safety.sh` remaining, verified with
> `git grep -n '388-interactive-shell\|220-extract-path-safety' -- TODO/ crates/ experiments/`.
> **This does not re-establish the deleted clauses.** The clause-to-test table
> above is the closure record and it is checkable by reading, not by running. A
> plant for either script is the work of a later wave.

`cargo test --workspace` alone does not reach the record half, so it is not
sufficient as written.

### B-20, wave 3's `podbox-cli` test shape

T-R003:60-72 argues for black-box and then lists seven files, five of them in
`podbox-cli`, three of which assert internals. Decide per file:

> **Decision: black-box where the assertion is about what the operator sees, and
> a new `[lib]` where it is about internal state.** `curated_refusals.rs`,
> `detached_stdio.rs` and `qol.rs` drive `CARGO_BIN_EXE_podbox` and assert only
> exit codes and output fields. `store_gates.rs` asserts which records a store
> operation skipped, which is not observable from the binary's exit code, and
> needs `[lib]`. Adding `[lib]` is **not** silent: `T-R004:45` forbids changing
> the release surface in this plan, so that one file waits for its own entry.

### B-30, who writes the perf readings

Replace with:

> **Decision: `podbox-perf` writes `experiments/perf-ceilings.tsv` and the three
> files check 30 reads, and no other binary writes them.** check 30 at
> `scripts/check-todo.py:1376-1379` names the four paths, so the port has one
> writer per path or the gate reads a file nobody updates.

## Blockers the plan did not name

These are outside every `⛔` in the seven entries.

1. **`gate.yml:176` is not a hazard.** The plan treats the shell-parse step as a
   deletion risk implicitly. It is not: `scripts/gnu-link-stub.sh`,
   `scripts/zig-ar.sh` and `scripts/zig-cc.sh` survive every wave, so
   `scripts/*.sh` always matches and the `for` loop always runs. Measured by
   listing `scripts/*.sh` and subtracting the eight wave-4 and wave-5 deletions.
   Worth stating so an implementor does not "fix" a step that is not broken.

2. **Check 23's committed reading.** Check 23 reads
   `experiments/results/bloat-interpose.txt` and holds each libc's size under
   `INTERPOSE_CEILING_BYTES`. Wave 4 ports `build-interpose.sh`, which is what
   writes that file. The entry does not name the reading, so the port can
   silently stop refreshing it.

3. **`plant.sh`'s `CEILING_BYTES` read is a fourth consumer of `110`.** T-R005
   names three. `scripts/plant.sh:191-193` reads the number out of `110` with
   `awk` and then **exports it**, because writing the literal would make the
   clean tree red. That is a fourth path coupling, and it is the reason the
   `110` replacement must keep a line `plant.sh` can parse, not merely keep the
   number.

4. **`dev.sh` is the caller of three scripts the plan reassigns.** `dev.sh:220`
   runs `./scripts/build-interpose.sh`, `:222` runs `394-ssh-package.sh`,
   `:225` runs `393-build-freshness.py`, `:226` runs `398-gate-diagnostics.py`.
   Wave 4 deletes `dev.sh` and wave 5 reassigns three of its four callees. No
   entry lists the caller set.

5. **`experiments/110-bloat-delta.sh` has 20 tracked invocations with 13
   distinct area arms** (`ci`, `interpose`, `baseline`, `memfd`, `tls`,
   `syscalls`, `seccomp`, `oci`, `landlock`, `http`, `archive`, `cli`,
   `experiments`), spread over `TODO/deps.md`, `TODO/gate.md`, `gate.yml` and
   `check-todo.py`. The plan's framing of three consumers understates the CLI
   contract the replacement binary inherits.

6. **Five of the `220` deletion sites are not `Prove` lines.** `TODO/extract.md:68`
   and `:308`, `TODO/image.md:547`, `crates/podbox-extract/src/drive.rs:543` and
   `lib.rs:278` are prose and doc comments. Only `:50` and `:237` are `Prove`.
   The entry's "six `Prove` or citation references" is right in count and wrong
   in kind, and the count that matters for the gate is the bare-citation count.

7. **`check 18` constrains every new experiment number.** A wave that adds a
   fixture or a driver under `experiments/` must not reuse a number, and
   `plant.sh:59-69` derives the taken set from the listing at run time. T-R003's
   fixture is named `crates/podbox-image/tests/common/registry.rs`, which is
   outside `experiments/`, so it dodges the rule by accident rather than by
   decision.

8. **`T-R003` says `cargo test --workspace` reaches all seven test files.** It
   reaches `crates/podbox-probe/tests/namespace.rs` only if `podbox-probe` is a
   member; it is, at `Cargo.toml:6`. So the claim holds. But the entry's own
   parenthetical admits `cargo test -p podbox-image` reaches 2 of 7, and the
   fix it chose (`--workspace`) is correct. No defect; recorded because the
   arithmetic was worth checking.

9. **`PLAN.md:75` lists `398` and `310` and `360` as `podbox-gate` subjects, and
   `T-R005:55` places `393` under `podbox-buildstate`.** Four scripts have two
   homes across two documents. Resolve before either wave starts.

10. **The plan's `docs/methodology/experiments.md:13` citation is off by one.**
    The exit-code rule is at `:12`. Minor, but this plan's whole argument is that
    citation accuracy is the gate's subject.

## Wave ordering

The order 0 to 6 is **substantially correct**. Three constraints are wrong or
missing.

**Correct and load-bearing.** Wave 0 before wave 1 is right and the reason holds:
`experiments/157-lock-inheritance-prove.sh:157` matches
`if !sys::close_in_children(lock\.fd) \{`, and
`crates/podbox-image/src/store.rs:1083` reads `if !sys::close_in_children(fd) {`
inside `fn try_acquire` at `:1074`, with `impl Lock` at `:1014` and
`Store::hold` at `:527`. The script's own text at `:157` also calls it `Store::hold`,
which is the wrong function. So the sed does not match, the mutation never lands,
and the guard that fires is the script's own. Verified by reading both sites.

Wave 1's rule that a deletion and its repoint are one change is right, and the
mechanism is `check_tree` at `:300` reading `git ls-files` at `:284`.

**Wrong: waves 4 and 5 are not independent.** T-R005:3-6 claims no shared crate,
fixture or dependency edge. They share `gate.yml:186` (B-10, B-11) and the
`check-todo.py` inputs. The correct ordering constraint the plan omits:

> **Wave 5 is ordered after wave 4 for one reason, and it is a file, not a crate:
> `gate.yml:186`.** Wave 4 removes two of the five `scripts/*.py`; wave 5 removes
> the remaining three. The glob `scripts/*.py` is still non-empty after wave 4 and
> empty after wave 5, and an empty glob fails (B-11). Whoever removes the last
> `.py` owns the step's rewrite. If wave 5 lands first, wave 4's two deletions
> leave three files and nothing breaks; if wave 4 lands first, wave 5 breaks CI.

**Wrong: wave 4's `Prove` cannot be its own acceptance.** T-R004:63 says the
`Prove` becomes `./target/release/podbox-gate` and `gate.yml:37` runs it. The
second half is now true by construction, since the port edits `gate.yml:37`. The
first half is not runnable on the `todo` runner (B-02). The constraint the plan
omits:

> **Wave 4 must add a build step to the `todo` job, or move the record gate into
> the `build` job.** Today `todo` runs no cargo command and bootstraps nothing
> (measured: 0). A record gate that needs a compiled binary is a different kind
> of gate from one that reads text, and `check-todo.py:56-58` says so in its own
> words: "This check reads the COMMITTED evidence and never builds anything, so
> it runs on a fresh clone with no toolchain." The port breaks that property. The
> entry must either preserve it, by having the binary's grammar stay text and the
> check stay a script, or give it up in writing.

**Missing: nothing enforces wave 3 before wave 2's clause 8, 9 and 11.** T-R002
lists clause 8 (guest user networking), clause 9 (`probe --json`) and clause 11
(the `alpine` layer) as wave-2 rows whose targets are "wave 3". An implementor
working T-R002 in order hits a row it must skip. The entry should mark those three
rows out of scope for wave 2 rather than leaving them in its table.

## What I could not verify

- **No Linux execution happened.** Both lanes failed (B-29). Every claim here
  that needed a compiler, `nm`, `cargo build` or a Linux `python3` was verified by
  reading source and by running the gate's own regexes and code paths under this
  host's Python, which is the same regex engine. The 112-name count was produced
  by `build-interpose.sh`'s own `awk` at `:199-200`, run on this host: 112. That
  awk is POSIX and its result does not depend on the Linux lane.
- **`cargo test` was never run.** Consistent with the host constraint.
- **The `wslc` lane** was probed once and hung; it is uncharacterised beyond that.
- **The four `wsl-toolkit` lane jobs** check 29 reports were not cleared, so no
  gate run in this pass was green. The red is B-28 and is host state.
