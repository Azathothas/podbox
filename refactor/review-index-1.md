# Review 1: `refactor/INDEX.md` for standalone-ness and accuracy

Reviewed: `refactor/INDEX.md`, 249 lines, dated 2026-10-01 in its own header.
Reviewed on: this Windows host, Git Bash, `rustc 1.98.0` host
`x86_64-pc-windows-msvc`, branch `main`, `refactor/` untracked.
Method: every table row, number, path, line reference, command and count below was
re-run. Where a claim needed Linux, the substitution and its limit are named.

⛔ **Lane state, because it bounds what this review can confirm.** Neither Linux
lane ran today. `wslc` 3.0.1.0 is present at `C:\Program Files\WSL\wslc.exe` and
its server answers `wslc info` (exit 0), but `wslc images`, `wslc list` and
`wslc container list` all returned exit 124 under a 90 s bound, and six attempts
at `wslc run --rm -v .../podbox:/work -w /work rust:1.98.1-bookworm sh -c 'id -u'`
produced zero bytes of output in about 25 minutes. `run-in-base.sh` exits 2 with
`current system boot ID differs from cached boot ID`, and
`wsl-toolkit --instance podbox base status --probe` reports `usable false` and
`STALE-RUN-STATE`. **Every claim below that requires a linked Linux binary is
UNVERIFIABLE on this host today**, and I say which.

Claims checked: 96. FALSE: 11. UNVERIFIABLE: 6. Partly false: 5.

## 1. Verdict table

### 1.1 The four host measurements ("The host answer, measured")

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 1 | `cargo check -p podbox-probe` exit 0 (L33) | **TRUE** | `cargo check -p podbox-probe` | `EXIT=0`, `Finished dev profile ... in 0.09s` |
| 2 | `cargo check --target x86_64-unknown-linux-gnu` on probe+enter+windows exit 0 (L34) | **TRUE** | `cargo check --target x86_64-unknown-linux-gnu -p podbox-probe -p podbox-enter -p podbox-windows` | `EXIT=0`. Cold, into a fresh target dir, the same three crates took **2.89 s**, not 2.36 s. The figure is not reproducible and carries no conditions. |
| 3 | `cargo check --target x86_64-pc-windows-gnu` exit 101 (L35) | **TRUE** | same, `--target x86_64-pc-windows-gnu` | `EXIT=101` |
| 3a | `E0433: cannot find unix in os` (L35) | **TRUE** | `grep -oE 'error\[E[0-9]+\]'` on the output | 2 occurrences: `crates/podbox-probe/src/child.rs:20` and `crates/podbox-probe/src/sys.rs:813` |
| 3b | `pre_exec` not found (L35) | **FALSE as an error class** | same | `pre_exec` is reported as **`E0599`**, not `E0433`: `error[E0599]: no method named pre_exec found ... --> crates/podbox-probe/src/sys.rs:815:13`. The output carries **5 errors**: 2 `E0433` and 3 `E0599` (`child.rs:71`, `child.rs:73`, `sys.rs:815`). The document names one class and hides four errors. |
| 3c | located at `crates/podbox-probe/src/sys.rs:815` | **TRUE** | `sed -n '805,825p'` | `815 \| cmd.pre_exec(|| {` |
| 3d | "a typecheck failure, not a link failure" (L35) | **TRUE** | read of the output | no linker text; `could not compile podbox-probe (lib) due to 5 previous errors` |
| 4 | `wslc run` + `cargo test -p podbox-probe` = **108 passed, 0 failed, exit 0** (L36) | **UNVERIFIABLE** | the exact command, six attempts | `wslc run` produced **0 bytes** in ~25 min. Substitute: `grep -c '#\[test\]' crates/podbox-probe/src/*.rs` = **108**, and 108 is the only test attribute in that crate. The count is consistent with the claim; the run is not confirmed. Two caveats the document omits: `.cargo/config.toml` sets `target = "x86_64-unknown-linux-musl"`, so a bare `cargo test` in this lane builds but does not run the gnu suite, and the claim names no target flag. |
| 5 | measurements "all taken 2026-10-01 on this machine" (L28) | **UNVERIFIABLE** for rows 3 and 4 | n/a | rows 1-3 reproduce today. Row 4's date cannot be checked; the lane is down. |

### 1.2 The per-crate Unix-only file counts (L40-41)

INDEX pattern: `std::os::unix`, `libc::`, `nix::`, `pre_exec`, `flock(`,
`syscall(`. The document does not state the scope. I used the scope that
reproduces the numbers, which is `crates/<c>/src/**.rs` and excludes
`crates/<c>/src/bin/`, `crates/<c>/tests/`, and `build =` output.

| crate | INDEX | mine | verdict |
| --- | --- | --- | --- |
| enter | 8/10 | 8/10 | TRUE |
| ssh | 8/14 | 8/14 | TRUE |
| cli | 6/30 | 6/30 | TRUE |
| image | 6/19 | 6/19 | TRUE |
| complete | 5/8 | 5/8 | TRUE |
| probe | 2/17 | 2/17 | TRUE |
| extract | 2/9 | 2/9 | TRUE |
| windows | 2/7 | 2/7 | TRUE |
| supervise | 1/3 | 1/3 | TRUE |

All nine reproduce, but only on `src/` alone. Over the whole crate directory
`podbox-ssh` is **11/18**, and `podbox-cli` is **6/31**. A reader who runs the
natural command gets a different number for two crates and has no way to know
which scope was meant. The claim is true and undocumented.

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 6 | "Eight of nine crates" (L41) | **TRUE** | the table above | 8 of 9 exceed zero |
| 7 | `podbox-supervise` at 1/3 is the only one a single `mod` could gate (L42) | **TRUE** | `grep -lE ... crates/podbox-supervise/src/*.rs` | the single hit is `launcher.rs:28`, `use std::os::unix::net::{UnixListener, UnixStream};` |
| 8 | its one file is `src/launcher.rs`, "which every other crate depends on" (L43) | **FALSE** | `grep -rn 'podbox-supervise' --include=Cargo.toml .` | exactly **one** crate depends on it: `crates/podbox-cli/Cargo.toml:28`. Seven of the eight other crates do not, and the workspace root at `Cargo.toml:10` lists it as a member, not a dependency. `launcher.rs` is additionally only used inside `podbox-supervise`'s own `lib.rs`; no crate outside it calls `launcher::`. |

### 1.3 The lane section (L57-88)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 9 | `wslc` 3.0.1.0 wins for proofs (L59) | **FALSE as a live claim** | `timeout 90 wslc list` etc. | the version is 3.0.1.0, but every container and image subcommand exits 124 and `wslc run` emits nothing. "Wins" over a lane that does not execute anything is not a measured result. |
| 10 | "`wslc` is **not on the Git Bash PATH**" (L70) | **TRUE** | `which wslc.exe` | `which: no wslc.exe in (...)`, exit 1. `C:\Windows\System32\wslc.exe` does not exist. |
| 11 | the invocation form and `MSYS_NO_PATHCONV=1` (L61-68) | **UNVERIFIABLE** | the exact command | `wslc --help` and `--version` answer instantly, so the binary is not the problem. No output from `run`. The `E_FAIL` claim is not reproducible today. |
| 12 | "`wslc run` executes as uid 0" (L72) | **UNVERIFIABLE** | same | never reached a shell. The claim is plausible and is carried by `recon-c.md:912`, which recorded `0` from `id -u`. It is inherited, not re-measured by INDEX. |
| 13 | 2 of 51 `podbox-complete` tests fail in that lane, at `devices.rs:603` and `:629` (L72-73) | **PARTLY FALSE** | `awk 'NR>=598 && NR<=635' crates/podbox-complete/src/devices.rs` | the two **line numbers are correct**: `:603` is `assert!(f.degraded, "{f:?}");` inside a shim assertion, and `:629` is `assert_eq!(f.action, Action::Rewrote, ...)`. But "**2 of 51**" misreads the source. `recon-c.md:836` records `test result: FAILED. 49 passed; 2 failed`, which is **49 passed of 51 total**, not 2 of 51. |
| 14 | "`gate.yml:199` runs the same command as non-root" (L75) | **TRUE** | `awk 'NR==199' .github/workflows/gate.yml` | `199 \| run: cargo test --workspace`, on `runs-on: ubuntu-latest` |
| 15 | "**9 tasks** assert a degradation, a refusal or a `skipped:` path" (L75-76) | **TRUE** | `recon-c.md:781` | `T-1517, T-1518, T-1520, T-1521, T-1529, T-1534, T-1539, T-1587, T-1578` |
| 16 | "Recon C §7 item 8 lists them" (L227) | **PARTLY FALSE** | `awk 'NR>=832' refactor/recon-c.md` | §7 item 8 (L832-837) is the `cargo test --workspace` lane result and the two `devices.rs` failures. **It does not list the nine.** The nine are at `recon-c.md:781`, in **§6 Acceptance proofs**. The citation points one section off. |
| 17 | `run-in-base.sh` copies 342.9 MiB, 12,040 entries (L81) | **PARTLY FALSE** | `sh scripts/windows/run-in-base.sh /tmp/j1.sh` | the script printed `workspace: 12046 entries, 343.1 MiB copied`. Entries are **12,046**, not 12,040. Size is 343.1 MiB, not 342.9. Both figures are within noise of each other; neither is the one in the document, and neither carries a date or the exclusion set used. `recon-a.md:434` records a different pair for the same artefact: **619 MiB and 11,984 entries**. Two sources in the same directory give two numbers for one copy, and the index quotes a third. |
| 18 | "Use `run-in-base.sh` for anything that mutates, and `wslc` for read-only proofs" (L84-85) | **UNVERIFIABLE, and not currently executable** | `sh scripts/windows/run-in-base.sh /tmp/j1.sh` | exit **2**, `Error: current system boot ID differs from cached boot ID`. The document presents both lanes as available. One is broken. |
| 19 | recon A measured `run-in-base.sh` failing on a stale boot ID, and recon B on the same (L85-86) | **TRUE** | `grep -n 'boot ID' refactor/recon-b.md` | `recon-b.md:56` B-29. Confirmed live today. |
| 20 | the base is repairable with `podman machine start` then `wsl-toolkit --instance podbox base ensure --repair` (L87-88) | **UNVERIFIABLE** | not run | the command mutates a VM, which is outside my scope. The recipe matches `recon-a.md:500-521`. |

### 1.4 The 12-row corrections table (L96-109)

| # | finding | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 1 | seven waves, not six; `PLAN.md:198` says `T-R000`..`T-R005`; `T-R005.md:3` and `T-R006.md:1` both call themselves sixth | **TRUE, one citation wrong** | `ls refactor/06-entries/T-R00*.md`; `sed -n '196,200p;3p'` | 7 files. `PLAN.md:198` reads `- \`T-R000.md\` .. \`T-R005.md\` — the six waves, one entry each`. `T-R005.md:3` reads `This is the sixth and last entry`. **`T-R006.md:1` does not call itself sixth**; it reads `# Wave 6 entry — delete the twelve obsolete scripts`. The sixth-self-naming is in `T-R005.md:3` alone. |
| 2 | `check-todo.py:228` reads `^CEILING_BYTES=(\d+)$`; check 23 reads `INTERPOSE_CEILING_BYTES` from `build-interpose.sh:48`; the regex does not match the Rust spelling | **TRUE** | `sed -n '228,229p' scripts/check-todo.py`; `sed -n '48p' scripts/build-interpose.sh`; `py -c "re.search(...)"` | `:228` is `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"`, `:229` is the regex. `build-interpose.sh:48` is `INTERPOSE_CEILING_BYTES=500000`. Regex test: matches `CEILING_BYTES=8000000` = **True**, matches `pub const CEILING_BYTES: u64 = 8000000;` = **False**. |
| 2a | "Wave 4 deletes both files" | **TRUE** | `grep -n '110-bloat-delta\|build-interpose' refactor/06-entries/T-R004.md` | `T-R004.md:52` names `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` as a path that moves; `:86` lists `scripts/build-interpose.sh` as `podbox-interpose-build`'s replacement source |
| 2b | `T-R004:45` forbids changing what the gate accepts (L99) | **TRUE** | `awk 'NR>=40 && NR<=50' refactor/06-entries/T-R004.md` | `:44-47`: `The port is behaviour-preserving, not a redesign. A rewrite that changes which records it accepts is a different task with its own review, and this entry does not authorise it.` |
| 3 | the `todo` job runs zero cargo commands and has no `bootstrap-env.sh` | **TRUE** | `awk '/^  todo:/,/^  build:/' .github/workflows/gate.yml \| grep -c 'cargo\|bootstrap-env'` | prints `0`, grep exit 1. The job body is 50 lines: checkout, check-todo, todo-count, plant, common checks, licence digest. |
| 4 | the `110` inversion: `check-todo.py:228`, `plant.sh:51`, `plant.sh:191`, `gate.yml:131`, plus **11 `Prove` lines in `TODO/deps.md`** | **PARTLY FALSE** | `grep -n '110-bloat-delta' ...`; `grep -c '^Prove:.*110-bloat-delta' TODO/deps.md` | the four code citations are exact. The `Prove` count is **10, not 11**: lines 67, 120, 162, 197, 246, 318, 370, 426, 532, 618. `recon-c.md:930` already says "plus **10** `Prove` lines in `TODO/deps.md`". The index contradicts the report it cites, and the paragraph at L114-116 that claims to have re-confirmed the count re-ran a grep that does not isolate `Prove` lines. |
| 5 | a second workflow consumes four subjects at `nightly.yml:84,103,112,179` | **TRUE** | `sed -n '84p;103p;112p;179p' .github/workflows/nightly.yml` | `./scripts/build-interpose.sh`, `sh scripts/nightly-smoke.sh ...`, `sh scripts/package-ssh.sh ...`, `sh scripts/release-notes.sh ...` |
| 6 | 26 retirement scripts in no wave; 48 unmatched = 22 SPLIT, 21 KEEP-SHELL, 5 RUST-TEST of which one is KEEP-SHELL post-VC-3 | **TRUE** | re-derived: for each ledger row, `basename not in concat(T-R00*.md)` | 121 rows, **48 unmatched**: SPLIT 22, KEEP-SHELL 21, RUST-TEST 5. The five RUST-TEST are `130-probe-parity.sh`, `151-spawn-ambiguity.sh`, `30-attribution-census.sh`, `368-run-decay.sh`, `95-podman-vfs-ignorechown.sh`. `22 + 5 - 1 = 26`. Exact. |
| 7 | ledger and plan disagree in four of five rows; the command and both number sets | **TRUE** | `tail -n +2 verdict-ledger.tsv \| cut -f3 \| sort \| uniq -c`; `sed -n '42,46p' PLAN.md` | ledger: SPLIT 35, RUST-TEST 20, KEEP-SHELL 27, DELETE 12, RUST-TOOL 27. `PLAN.md:42-46`: 29 / 36 / 27 / 18 / 11. Four rows differ; RUST-TOOL matches. |
| 7a | "`PLAN.md:22` warns about exactly this failure and then reproduces it" (L104) | **TRUE** | `sed -n '22,27p' PLAN.md` | `⛔ The ledger is PRE-VC. The table in section 2 is POST-VC.` |
| 7b | the ledger/plan delta is a real defect | **PARTLY FALSE, unstated** | applied the three VC edits to the ledger | four of the five rows reconcile exactly once VC-2, VC-3 and VC-7 are applied: DELETE 12-1=11 ✓, SPLIT 35+1=36 ✓, RUST-TEST 20-2=18 ✓, RUST-TOOL 27 ✓. **KEEP-SHELL does not**: 27 + 1 (VC-3) = 28, and the plan says **29**. One script is unexplained. `recon-c.md:940-941` asserts the other two "VC added" it, which the ledger does not contain. The index reports the disagreement as a defect without reporting that it is one script wide. |
| 8 | T-R002's "source defect" is not a defect; the three sources agree | **TRUE** | `cat experiments/results/store-gc.txt`; `sed -n '961,986p' images.rs`; `sed -n '138,139p' 160-store-gc.sh` | result: `prune_skipped 1`. `images.rs:961` is `store.delete(&rest, Held::Skip)` and `:986` is `0`. `160-store-gc.sh:138` asserts `[ "$prune_rc" -eq 0 ]`, `:139` asserts `skipped:` appears. All three agree. |
| 9 | `system.rs` `mod tests` at 604 not 603; `fn abi` at 157 not 191-214; `parity.rs` `mod tests` at 707 not 706 | **TRUE** | `grep -n 'mod tests' ...; grep -n 'fn abi' system.rs` | `system.rs:604`, `parity.rs:707`, `system.rs:157: pub fn abi(...)`. `PLAN.md:123` carries the 603 and `:191-214` claims. |
| 9a | the `706` claim in `PLAN.md` | **UNVERIFIABLE as a defect** | `grep -n '706' refactor/06-entries/PLAN.md` | no `PLAN.md` line I read cites `parity.rs:706`. The row's evidence column names only the `grep -n 'mod tests'` command, which cannot show a 706. The `706` half of this finding has no cited source. |
| 10 | `PLAN.md:124` names `crates/podbox-supervise/src/nongoals.rs`; it is in `podbox-probe` | **TRUE** | `awk 'NR==124' PLAN.md`; `ls crates/podbox-supervise/src/` | `:124` reads `tests at \`crates/podbox-supervise/src/nongoals.rs:253-302\``. The directory holds `launcher.rs`, `lib.rs`, `table.rs`. `crates/podbox-probe/src/nongoals.rs` exists. |
| 11 | concurrent `TODO/` edits are unsafe; `todo-count.py` rewrites one Counts block from all 203 rows | **TRUE** | `sed -n '159,161p' scripts/todo-count.py`; `grep -cE '^\| \[T-[0-9]{4}\]' TODO/INDEX.md` | `start = lines.index("## Counts")`. **203** rows today. The document says "22 tasks must serialise"; `recon-c.md:614` says the same 22. |
| 12 | the plan is untracked; `git ls-files refactor` returns 0 | **TRUE** | `git ls-files refactor \| wc -l`; `git status --porcelain refactor` | `0`, and `?? refactor/` |
| 12a | "`check_tree` never reads it, so no citation in it is held" (L109) | **TRUE, and it is the one that matters** | `scripts/check-todo.py:284` | `check_tree` iterates `git ls-files -z`, which is tracked files only. `CORPUS_PREFIX` is `references/`, not `refactor/`. So INDEX's own 96 citations are held by nothing. |

### 1.5 The counts paragraph (L52-55)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 13 | recon C's 100 tasks: **42 native-windows, 51 linux-lane, 7 both** | **FALSE** | parsed the `Host` column of all 100 rows in `recon-c.md` §1 | **47 native-windows, 47 linux-lane, 6 both.** The index copies the summary table at `recon-c.md:198-200`, which is itself wrong against its own §1. The index should not have propagated it. |
| 14 | recon A's 66 work units: 21 native, 34 authorable now, 7 Linux kernel, 4 blocked | **TRUE** | `sed -n '410,418p' refactor/recon-a.md` | the rollup totals row reads 21 / 34 / 7 / 4 / 66, and the six per-wave rows sum to it. |
| 14a | "Authoring is native for all 100" (L53) | **TRUE** | `recon-c.md:80-81` | `Host is what the task needs to be PROVEN, not authored. Authoring is always native` |

### 1.6 Recon B's class counts (L92-93)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 15 | 30 blockers: **21 TREE, 3 HOST, 3 SELF-REFERENTIAL, 4 UNRESOLVED-DECISION, 6 FALSE** | **PARTLY FALSE** | parsed the `class` column of all 30 `B-NN` rows | as classified in the table: TREE **15**, HOST **2**, SELF-REFERENTIAL **2**, UNRESOLVED-DECISION **4**, FALSE 4 + "FALSE (minor)" 2 + "FALSE (as stated)" 1 = **7**. These sum to 30. The index's five numbers sum to **37**. |
| 15a | against recon-b's own summary | **TRUE** | `sed -n '59,68p' refactor/recon-b.md` | recon-b's table reads TREE 21, HOST 2, SELF-REFERENTIAL 3, UNRESOLVED-DECISION 4, FALSE 6, plus a row "TREE + SELF-REFERENTIAL overlap 2 (B-05, B-06)". recon-b's own summary double-counts the overlap, which is why it sums to 36. The index **changed HOST from 2 to 3** and kept the rest, producing 37. The index silently "corrected" one number and left the arithmetic broken. |
| 16 | "Recon B found 30 blockers" (L92) | **TRUE** | `grep -cE '^\| B-[0-9]+ \|' refactor/recon-b.md` | 30 |

### 1.7 The task-list table (L152-161)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 17 | 100 tasks, `T-1501` through `T-1600`, no gaps, no collisions | **TRUE** | parsed every `T-NNNN` in `recon-c.md` §1 | exactly **100** rows, min 1501, max 1600, 100 unique, **no gaps** |
| 18 | the eight ranges cover all 100 (14+2+13+10+9+11+11+20) | **FALSE, arithmetic and coverage** | same parse, grouped | the eight ranges sum to **90** and leave **T-1540 to T-1549 uncovered**. Those ten are real wave-1 deletions (`experiments/156-closure-records.sh` and nine others) and appear in `recon-c.md:127-136`. The document's own claim "total 100" is unreachable from its table. |
| 19 | the "100" total (L142) | **FALSE** | sum of the eight claimed counts | 14+2+13+10+9+11+11+20 = **90**, not 100. |
| 20 | the per-group **host splits** | **FALSE, 6 of 8 rows** | same parse, `Host` per group | wave0 12 native / 2 both ✓. wave1 2 linux ✓. **wave2: measured 13 linux-lane, claimed "2 native, 11 linux-lane".** **wave3: measured 9 linux-lane + 1 both, claimed "1 native, 7 both, 2 linux-lane".** **wave4: measured 2 native + 1 both + 6 linux-lane, claimed "1 native, 8 both".** **wave5: measured 11 linux-lane, claimed "1 native, 8 both, 2 linux-lane".** **wave6: measured 3 native + 6 linux + 2 both, claimed "11 native".** decisions 20 native ✓. |
| 21 | highest existing id is T-1423 at `TODO/INDEX.md:266` | **TRUE** | `awk 'NR==266' TODO/INDEX.md`; `grep -oE '\bT-1[0-9]{3}\b' TODO/INDEX.md \| sort -u \| tail -1` | `\| [T-1423](packaging.md) \| P1 \| packaging \| done \| The release tag names the workspace version` |
| 22 | `docs/methodology/authoring.md:62` says "Use a free task ID." | **PARTLY FALSE** | `awk 'NR>=62 && NR<=65' docs/methodology/authoring.md` | the sentence is at **:64**. `:62` is the heading `## Numbering`. |
| 23 | column order matches `TODO/INDEX.md:61` | **TRUE** | `awk 'NR==61' TODO/INDEX.md` | `\| ID \| Priority \| Category \| Status \| Item \|` |
| 24 | four decision records: T-1551, T-1558, and two scope records (L163-165) | **PARTLY FALSE, unverifiable in part** | `sed -n '189,192p' refactor/recon-c.md`; `grep -n '1597\|1598\|1599\|1600' refactor/INDEX.md` | recon-c names the four as **T-1597 to T-1600**, and the index never names them. The index names T-1551 and T-1558, which recon-c does **not** call decision records. So the count 4 is right and both named ids disagree with the source. "two scope records" is unnamed and unfindable. |
| 25 | "No task id collides with an existing `TODO/` id" (L167) | **TRUE** | the parse above against `TODO/INDEX.md`'s 203 rows | 1501-1600 is above the highest, 1423 |

### 1.8 The gate state (L111-116)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 26 | `py scripts/check-todo.py` now **exits 0** | **TRUE** | `py scripts/check-todo.py`; `echo $?` | `check-todo: 203 rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done` / `check-todo: ok` / `EXIT=0`. The gate is not red, so the `wsl-toolkit` lane-job history is not a live problem and no `gc` call is needed. |
| 27 | it was "red on **five** retained `wsl-toolkit` lane jobs" and "the **four** ids were removed" (L112-113) | **PARTLY FALSE, self-contradictory** | the gate's own source and the two reports | `check-todo.py:1341` emits one `err()` per kept item. `recon-b.md:55` records **four** findings; `recon-c.md:607` records **4 problems** "naming **three** distinct job ids". Five is a fifth number, appearing in no source. The sentence also says five were found and four removed, which leaves the gate red by its own account. |
| 28 | the `110` consumer count was confirmed by the quoted grep (L114-116) | **FALSE** | the quoted command, rerun | `grep -n '110-bloat-delta' scripts/check-todo.py scripts/plant.sh .github/workflows/*.yml TODO/*.md` returns **22** lines, of which 10 are `Prove:` lines in `TODO/deps.md` and 4 are prose. It cannot produce "11 Prove lines", which is the claim being confirmed. |

### 1.9 Batches and fleet (L170-215)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 29 | Batch 0 "3 tasks", "unblock 60 of the remaining 92" (L176-177) | **FALSE** | counted the `depends_on` column of `recon-c.md` §2 | 100 - 3 = **97** remaining, not 92. Of those, **25** name one of the three directly and **38** name one transitively. Neither is 60. |
| 30 | Batch 2 = "T-1552 .. T-1557, T-1577, T-1578" (L188) | **TRUE** | `sed -n '660,661p' refactor/recon-c.md` | `T-1552, T-1553, T-1554, T-1555, T-1556, T-1557, T-1577, T-1578` |
| 31 | Batch 3 = "4 agents, 11 tasks" (L194) | **TRUE** | `recon-c.md:668-673` | the header says 6, the body corrects to 4. The index took the correction. |
| 32 | Batch 4 = "7 agents, 10 tasks" (L198) | **PARTLY FALSE** | `recon-c.md:675-678` | recon-c's body names 2 + 4 + 5 + 3 agents for 10 tasks, which is 9 groups before merging. The index says 7. |
| 33 | "100 tasks and 43 batches" (L174) | **TRUE** | `sed -n '627,628p' refactor/recon-c.md` | `the decomposition is 43 batches` |
| 34 | "19 concurrent agents", sustainable 6, floor 1, 79 of 89 six-wide (L208-215) | **TRUE** | `recon-c.md:632-635, 691` | verbatim match. |

### 1.10 Acceptance proofs (L217-232)

| # | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 35 | `Cargo.toml:20` excludes `crates/podbox-interpose` | **TRUE** | `awk 'NR==20' Cargo.toml` | `exclude = ["crates/podbox-interpose"]` |
| 36 | its proof is the command at `gate.yml:213` | **TRUE** | `awk 'NR==213' .github/workflows/gate.yml` | `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` |
| 37 | `docs/conventions/code.md:32` requires a check to land with a plant | **TRUE** | `awk 'NR==32' docs/conventions/code.md` | `A test must reject the defect it claims to detect.` This is about a test, not about planting. The claim is a fair reading; the line does not mention a plant. |
| 38 | `scripts/plant.sh:74` refuses to start on a dirty tree | **TRUE** | `awk 'NR>=70 && NR<=79' scripts/plant.sh` | `:74` is `if ! git diff --quiet -- $FILES \|\| ! git diff --cached --quiet -- $FILES; then`, and `:78` is `exit 2` |
| 39 | "wave 2 adds 15 plants" (L83) | **TRUE** | `grep -n '15' refactor/06-entries/T-R002.md` | `:111` `Each of the 15 tests gets one scripts/plant.sh case` |
| 40 | `.github/workflows/nightly.yml` runs the four scripts (L102) | **TRUE** | see finding 5 above | |
| 41 | `T-R004:45` forbids changing what the gate accepts (L99, L128) | **TRUE** | see 2b above | |

### 1.11 House-gate conformance of the document itself

| # | check | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| 42 | `sh scripts/common/check-markers.sh` | **FAIL for this file** | the script, filtered | `marker check failed, 975 problem(s)`. `refactor/INDEX.md` accounts for **23** of them. |
| 43 | `sh scripts/common/check-docs.sh` | **FAIL for this file** | the script, filtered | `documentation check failed, 129 problem(s)`. One is `refactor/INDEX.md:61 shell-unsafe placeholder. bash reads it as a redirect; use UPPER_SNAKE`. The same class is `refactor/06-entries/T-R006.md:87`. Separately, **all 100 rows of recon-c.md's task table** are reported as `broken link -> gate.md` and so on, because they are written `TODO/`-relative inside `refactor/`. The document that INDEX calls "the authority" is 100 broken links wide. |
| 44 | scope of those two checks | note | `check-markers.sh:136-143`, `check-docs.sh:103-104` | both use `git ls-files` **plus** `git ls-files --others --exclude-standard`, so untracked `refactor/` **is** in scope. The findings are real, not an artefact of untrackedness. |
| 45 | `py scripts/check-todo.py` is green | **TRUE** | see 26 | it stays green only because `check_tree` reads tracked files only (finding 12a). |

## 2. Standalone-ness: the five reader questions

Assume the reader has this file, the repository, and no other file under
`refactor/`.

**1. What is the actual goal, in one paragraph? Is it stated?**
Partly. INDEX.md:3-9 says it "consolidates three independent analyses" and "states
what an implementor does next". It never says what the work **is**. The goal
exists in one sentence in `PLAN.md:3-5`: retire 121 shell, Python and PowerShell
scripts under `experiments/` and `scripts/` by re-expressing their measurements
as native Rust tests or native Rust tooling. A reader of INDEX alone cannot state
the goal in one paragraph, because INDEX does not contain it. **Fix: add three
sentences to the preamble naming the corpus, the count and the target form.**

**2. What must a reader know about podbox itself?**
Not here. INDEX never says what podbox is, what a chroot container is, or that
`podbox-interpose` is a `cdylib` excluded from the workspace. The only pointer is
"Read PLAN.md first", and PLAN.md points onward to `TOOL.md` sections 3, 4 and 6,
which are in `references/Azathothas__container-research/tree/`. The chain is three
links deep and the first is not marked as mandatory for the parts INDEX carries.
An implementor who starts at INDEX and does not follow PLAN cannot tell what a
"wave" or a "gate" is.

**3. Can a reader find their task?**
No, and the ID ranges in the task-list table make it worse rather than better.
The table at L152-161 omits T-1540 to T-1549 entirely, so a reader assigned a
wave-1 deletion cannot find their id anywhere in INDEX. The table's `host split`
column is wrong for 6 of its 8 rows, so a reader who trusts it will not know
whether their task can be proven here. Navigation then requires §1 of a
78,539-byte, 970-line file, in which the task rows sit between lines 88 and 187,
with no anchor, no index, and **100 broken links** pointing at `TODO/` files that
the link checker resolves relative to `refactor/`. A reader who clicks T-1551's
`gate.md` gets nothing. **Fix: carry a per-task index in INDEX itself, or state
the line range and give a working grep.**

**4. Are the repository's own rules stated or linked?**
Four of the six are absent, and the one the assignment names as the safety rule
is unreachable. Mentions counted by name in INDEX.md: `AGENTS.md` **0**,
`TODO/RULES.md` **0**, `docs/methodology/gate.md` **0**,
`docs/containers.md` **0**, `docs/conventions/prose.md` **0**,
`docs/conventions/code.md` 1, `docs/methodology/authoring.md` 1.

⛔ **The `wsl.exe` rule is not reachable.** `AGENTS.md:65` says "On Windows, use
`wsl-toolkit --instance podbox`. Do not call `wsl.exe`." `docs/containers.md:62`
says "Never call `wsl.exe` from this project. A direct call can alter another
distribution." INDEX.md contains **no** occurrence of `wsl.exe`, `AGENTS.md`, or
`docs/containers.md`, and it **recommends a second engine** (`wslc`) by full path
without ever mentioning the rule that governs engine choice. A reader following
INDEX has been handed an engine invocation with none of the surrounding
constraint, and the file they are most likely to open next, `recon-wslc.md:429`,
states the rule only to explain why it did not test something. **This is the
single most dangerous omission in the document.**

**5. Is anything stated that a reader would reasonably misread?**
Three things.

⛔ **"authoring is always native" is not clear enough to stop a wrong
assumption.** L47 reads `**authoring** — writing Rust or markdown. Always native.
Always available.` The three-way split is correct and the phrase is the right
one, but the line sits above a **Counts** paragraph that then gives
42/51/7, and the reader is left to work out that the 51 and 7 are the tasks that
are *writable* here and *not provable* here. The document never says in one
sentence that a task in the `linux-lane` column cannot be **finished** on this
host, only started. Given that INDEX elsewhere states both lanes as available
(claim 18) and neither is, this is the misreading most likely to happen.

⛔ **D-1 option (c) is presented as a decision this index has made.** L132-134:
"**(c) is the smallest change that preserves the check, and it is the one this
index assumes until the operator says otherwise.**" The section header two lines
above says "Two decisions that must be made before any code" and "These are not
mine to take." The document then takes one. An implementor reading quickly will
treat (c) as settled and write the ceiling declaration as shell, which is the
outcome the section says needs an operator.

⛔ **"Findings 2, 3, 4 ... are not in any entry and will stop you" (L239) is
mis-scoped.** Those three findings are also not in `recon-a.md` or
`recon-c.md`'s task rows as tasks; finding 2's fix is D-1 and finding 4's is
T-1558, both of which exist. Saying they are in no entry is true. Saying they are
not in any entry while listing 26 other findings that were not checked for the
same property implies an audit that was not done.

**Verdict: NOT standalone.** A reader with this file and the repository cannot
begin implementation without reading `PLAN.md`, `recon-c.md`, `AGENTS.md`,
`docs/containers.md` and `TODO/RULES.md`, and one of those five is not linked.

## 3. Prose and style conformance

The repository's rule is `docs/conventions/prose.md`: ASD-STE100, "Use ASCII
prose where possible. Do not use an em dash." (`prose.md:51`), markers from a
fixed five (`prose.md:54-60`), "Use markers only when they improve a procedure"
(`prose.md:62`).

**Em dashes. FALSE conformance.** 14 em dashes in 12 lines: L1, L47, L48, L50,
L127, L176, L181, L187, L194, L198, L199, L213. `prose.md:51` forbids the
character. `scripts/common/check-markers.sh` reports each one as
`U+2014 is outside the five`. The document is 23 marker-check problems wide. For
comparison the repository's own prose documents carry **zero** em dashes:
`docs/architecture.md` 0, `TODO/RULES.md` 0, `AGENTS.md` 0, `README.md` 0. The
sibling reports in the same directory are inconsistent (`recon-b.md` 0,
`recon-wslc.md` 0, `recon-a.md` 23, `recon-c.md` 106), so INDEX is following its
neighbours rather than the rule. **Fix: replace all 14 with a colon, a full stop,
or a rewritten clause.**

**Markers. Correctly used, thinly.** Three `⛔`, all on load-bearing
constraints: L72 (the uid-0 limit), L222 (the nine tasks), L229
(`podbox-interpose` is excluded). No `⭐`, no `⚠`, no `✅`, no `❌`. The index
makes no first-choice claim and reports no failed machine result inline, so
`prose.md:62` is satisfied. The two ⛔ blocks at L72-78 and L222-227 are the
strongest parts of the document, and both are also the two with defects (claims
13 and 16), which is worth noting: the marker flags exactly the sentences a
reader will trust most.

**Contractions. None.** Zero matches for the usual set. Compliant.

**House emphasis. Does not match.** The repository uses `*italic*` for the
name of a thing and `**bold**` sparingly for a constraint. INDEX uses `**bold**`
**108 times in 249 lines**, roughly every 2.3 lines, and uses `*italic*` **zero
times**. Compare `T-R004.md`, where bold introduces a decision and a ⛔
introduces a coupling, and `crates/podbox-supervise/src/lib.rs`, which uses both
markers with distinct meanings. In INDEX the bold carries no consistent meaning:
it marks crate names, task ids, verdicts, and whole sentences in the same
register. **Fix: reserve `**` for constraints, as the rest of the tree does.**

**Sentence shape.** Largely compliant and the best of the document's qualities.
Short declarative sentences, active voice, named actors, the actor named in every
procedure sentence I checked. The D-1 section (L123-134) is the clearest writing
in the file. The two places it slips are L215, `**89 percent of the work sits
outside it**, and 79 of those can run 6-wide`, where a second clause hangs off a
fragment, and L181-185, where a parenthetical list runs to 96 words in one
sentence. `prose.md:13` caps descriptions at 25 words.

**Non-ASCII beyond the em dash.** `§` x9, `→` x7, `⛔` x3. The markers are
allowed. The other sixteen are not on `prose.md`'s list of five, and the marker
check reports `refactor/INDEX.md` problems for them.

**Shell-unsafe placeholder.** `check-docs.sh` names `refactor/INDEX.md:61`. Line
61 is a bare ` ```sh ` fence opener, and the placeholder it is complaining about
is the `-v "C:/...:/work" -w /work` form on L63, whose `>`-bearing path a reader
who strips the quotes will let bash read as a redirect. This is the same finding
as `T-R006.md:87` and the fix is the same: `UPPER_SNAKE` in the example, or drop
the fence.

## 4. Prioritised defect list

| severity | location | what is wrong | the fix |
| --- | --- | --- | --- |
| **blocker** | L152-161 | The task-range table covers **90 of 100** ids. T-1540 to T-1549, ten real wave-1 deletions, appear in no range. The stated total of 100 is not reachable from the table. | Add the range `T-1540 .. T-1549` under a wave-1 deletions row, or merge into `T-R001 wave 1` and re-derive the count with `scripts/todo-count.py`-style arithmetic rather than by hand. |
| **blocker** | L154-161, L52 | The per-group host splits are wrong in **6 of 8 rows**, and the headline 42/51/7 is wrong against the very table it summarises (measured 47/47/6). A reader uses this column to decide what they can prove here. | Re-derive all eight cells and the headline from the `Host` column of `recon-c.md` §1, and fix `recon-c.md:198-200` at the same time. The two files must not disagree. |
| **blocker** | whole file | The `wsl.exe` prohibition is unreachable. `AGENTS.md`, `docs/containers.md` and `TODO/RULES.md` are never named. The file recommends a second engine by absolute path with no reference to the rule that governs engine choice. | Add a constraints paragraph naming `AGENTS.md`, `docs/containers.md` and `TODO/RULES.md`, and state in one sentence: "Never call `wsl.exe`. Use `wsl-toolkit --instance podbox` for the lane." Link both files. |
| **blocker** | L111-116 | The record-gate paragraph is self-contradictory and unsupported: "five" jobs found, "four" ids removed, and the quoted grep cannot produce the "11 `Prove` lines" it claims to have confirmed. The true count is **10**. | Rewrite as: "The gate was red on four `wsl-toolkit` lane-job findings naming three job ids (`recon-b.md` B-28). The ids were removed with `gc --job <id> --apply` and `py scripts/check-todo.py` now exits 0." Change 11 to 10 and cite `recon-c.md:930`, which already says 10. |
| **major** | L176-177 | "unblock 60 of the remaining 92" is wrong twice: 100-3 is 97, and 25 tasks depend on the three directly, 38 transitively. | Recompute from the `depends_on` column of `recon-c.md` §2 and state the method. |
| **major** | L35 | The `x86_64-pc-windows-gnu` row reports one error class and one location for an output carrying 5 errors across 2 classes and 2 files. `pre_exec` is `E0599`, not `E0433`. | Rewrite the cell: "exit 101, 5 errors: 2 `E0433` at `child.rs:20` and `sys.rs:813`, 3 `E0599` at `child.rs:71`, `child.rs:73` and `sys.rs:815`." |
| **major** | L92-93 | Recon B's class counts sum to 37 against 30 blockers. `HOST` was changed from recon-b's 2 to 3 with no source, and the "TREE + SELF-REFERENTIAL overlap" row was dropped without saying so. | Quote recon-b's own table verbatim including the overlap row, or give the five classes as measured from the 30 rows: 15 / 2 / 2 / 4 / 7. State which. |
| **major** | L59-88 | Both lanes are presented as available. Today `wslc run` produces no output and `run-in-base.sh` exits 2 on a stale boot ID. The file's central operational claim cannot be executed. | Add a dated lane-state line: "On 2026-10-01 `wslc run` returned no output and `run-in-base.sh` exits 2. Both lanes need repair before any `linux-lane` proof. `wsl-toolkit --instance podbox base ensure --repair` after `podman machine start` is the recipe for the second; the first is open." |
| **major** | L163-165 | The four decision records are named wrong. recon-c names T-1597 to T-1600; INDEX names T-1551 and T-1558, which recon-c does not call decision records. "two scope records" is unfindable. | Name all four by id, or say "see `recon-c.md:189-190` for the four". |
| **major** | L122 | `docs/methodology/authoring.md:62` is the `## Numbering` heading. The sentence is at `:64`. | Change 62 to 64. |
| **major** | L98 | `T-R006.md:1` is cited as calling itself sixth. It does not; only `T-R005.md:3` does. | Drop the `T-R006.md:1` half of the citation, or replace it with the accurate statement: "`T-R005.md:3` calls itself the sixth and last entry; `T-R006.md` exists and is not in `PLAN.md`'s list." |
| **major** | L103 | The ledger/plan disagreement is reported as a defect without the magnitude. Four of five rows reconcile exactly once the three VC edits are applied. The residual is **one script** on KEEP-SHELL: 27 + 1 = 28 against a stated 29. | Say: "four of five rows reconcile once VC-2, VC-3 and VC-7 are applied to the ledger. KEEP-SHELL does not: 28 against a stated 29. One script is unexplained, and `recon-c.md:941` claims the other two without the ledger carrying them." That is a one-sentence fix and a far stronger finding. |
| **major** | L1, 47, 48, 50, 127, 176, 181, 187, 194, 198, 199, 213 | 14 em dashes, forbidden by `docs/conventions/prose.md:51`, reported by `check-markers.sh` as 23 problems for this file. | Replace each with a colon, a full stop, or a rewritten clause. Then run `sh scripts/common/check-markers.sh`. |
| **major** | L149-151 | INDEX calls `recon-c.md` "the authority" and sends every implementor to its §1 and §2. All 100 task rows there are broken links: `broken link -> gate.md` and so on, because the paths are `TODO/`-relative inside `refactor/`. | Repoint the 100 links as `../TODO/gate.md`, or state that the table's second column is a `TODO/` filename and not a link. |
| **minor** | L226-227 | "Recon C §7 item 8 lists them" points at the `devices.rs` lane result, not at the nine. The nine are at `recon-c.md:781`, in §6. | Change §7 item 8 to §6, and name the nine ids: T-1517, T-1518, T-1520, T-1521, T-1529, T-1534, T-1539, T-1587, T-1578. |
| **minor** | L72-73 | "2 of 51 `podbox-complete` tests fail" misreads `49 passed; 2 failed` as a rate. | "49 tests run, 2 fail, at `devices.rs:603` and `:629`." |
| **minor** | L43 | "its one file is `src/launcher.rs`, which every other crate depends on" is false. One crate depends on it: `crates/podbox-cli/Cargo.toml:28`. | "which `podbox-cli` depends on at `crates/podbox-cli/Cargo.toml:28` and which is used only from `podbox-supervise`'s own `lib.rs`." |
| **minor** | L81 | `342.9 MiB, 12,040 entries` against the script's own `12,046 entries, 343.1 MiB`, and against recon-a's `619 MiB and 11,984 entries` for the same copy. Three numbers for one artefact. | Quote the script's own line, dated, with the exclusion set from `run-in-base.sh:143`. Reconcile with `recon-a.md:434` or say which is right. |
| **minor** | L34 | `2.36 s` is unreproducible and carries no conditions. A cold run into a fresh target dir took 2.89 s. | Drop the figure, or give it with the target dir, the flags and the date. |
| **minor** | L40-41 | The nine counts hold only for `crates/<c>/src/`, which the document does not say. Over the crate directory `podbox-ssh` is 11/18 and `podbox-cli` is 6/31. | State the scope and the pattern in the sentence: "counted over `crates/<c>/src/**/*.rs` with `grep -lE 'std::os::unix\|libc::\|nix::\|pre_exec\|flock\(\|syscall\('`." |
| **minor** | L106 | Finding 9's `706` claim has no cited source. No `PLAN.md` line I read cites `parity.rs:706`, and the row's own evidence command cannot show a 706. | Drop the `706` half, or cite the line that says it. |
| **minor** | L198 | "Batch 4 — 7 agents" against recon-c's body, which names 2 + 4 + 5 + 3 groups. | Reconcile with `recon-c.md:675-678` and say which grouping gives 7. |
| **minor** | L198 | Batch 4 can start before Batch 2, and Batch 4 alone is shippable. That is the single most useful sentence in the file and it is buried in a paragraph about a 7-agent count. | Promote it to its own short section at the top of Batches. |
| **minor** | L3-9 | No goal statement. The corpus, the count of 121 and the target form are absent. | Three sentences: "This retires 121 shell, Python and PowerShell scripts under `experiments/` and `scripts/` by re-expressing their measurements as native Rust tests or native Rust tooling." |
| **minor** | L132-134 | Section header says the two decisions are not the author's to take; the text then takes D-1 option (c). | Either remove the "assumes" sentence, or move it under a heading that says it is a proposal pending the operator's answer. |
| **minor** | L61-65 | `check-docs.sh` names `INDEX.md:61` as a shell-unsafe placeholder, the same class as `T-R006.md:87`. | Use `UPPER_SNAKE` in the `-v` example, or drop the ` ```sh ` fence. |
| **minor** | whole file | 108 `**bold**` spans in 249 lines and zero `*italic*`. The repository uses `*` for a name and `**` for a constraint. | Reserve `**` for constraints. Expect to remove about 60 spans. |
| **minor** | L34, L53-55, L92, L101, L103, L104, L142, L161, L168, L174, L177, L213 | Counts and verdicts are stated without conditions, dates or methods, against `prose.md:23-25`. | Give each one a command and a date in the cell, as the corrections table already does well. |

## 5. What this review did not cover

- **Nothing requiring a linked Linux binary was confirmed.** Every
  `cargo test` claim, the 108-run, the uid-0 limit, the two `devices.rs`
  failures, the nine-task lane block, and both lanes' behaviour are reported as
  UNVERIFIABLE with the substitution named. The 108 figure is corroborated only
  by counting `#[test]` attributes on disk.
- **`wslc` behaviour was not characterised**, only observed as non-responsive.
  Six `wslc run` attempts and three list subcommands produced no output. I did not
  diagnose why, and I did not attempt an install, a settings change, or a
  `wsl --update`.
- **I did not repair the base.** `wsl-toolkit --instance podbox base ensure
  --repair` and `podman machine start` change a VM, which is outside my scope. I
  ran `base status --probe`, which is read-only, and it reports `usable false`.
- **I did not re-derive recon-a's 66 work units** beyond checking the rollup
  table's internal arithmetic, which sums correctly.
- **I did not run `check-markers.sh` to completion twice**; the run I captured
  took 130 s and the filtered count for `refactor/INDEX.md` is 23. The
  per-character breakdown was obtained with a direct codepoint count instead:
  U+2014 x14, U+00A7 x9, U+2192 x7, U+26D4 x3.
- **I did not review `recon-a.md`, `recon-b.md` or `recon-c.md` on their own
  merits.** Where they are the source of a claim INDEX repeats, I checked the
  claim, and where the claim and its source disagree I reported both.
- **No file was edited, staged or committed.** The only write is this file.

## 6. One finding that is not a defect in INDEX

`recon-wslc.md:171` and `:334` state that `wslc.exe` is **absent** from this host
and that adopting it needs a WSL upgrade. Both are false today: the binary is at
`C:\Program Files\WSL\wslc.exe`, 9,040,184 bytes, dated 2026-09-25, and
`wslc --version` answers `wslc 3.0.1.0` instantly. `recon-c.md:909` already
records the correct fact. INDEX links `recon-wslc.md` at L21 as an input without
noting that its central premise is stale, and `recon-wslc.md:334` is the row that
would have stopped an implementor from trying the lane. **This is the most
consequential stale fact reachable from INDEX, and INDEX does not flag it.**
