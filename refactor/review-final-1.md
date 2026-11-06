# Final accuracy review: INDEX.md and PLAN.md

Reviewed 2026-10-01. Scope: factual accuracy only. Every number, path, line
reference, command, and claim in [INDEX.md](INDEX.md) and [PLAN.md](PLAN.md)
was re-derived from the tree. No existing file was edited. No commit, no
`git add`.

Verdicts: **TRUE**, **FALSE**, **UNVERIFIABLE**. UNVERIFIABLE means the claim
was not reproducible from the tree as it stands, not that it is wrong.

Commands were run in the repository root under Git Bash. Exit codes are read
without a pipe.

---

## 1. Verdict table

### 1.1 Section 4 and PLAN.md host table

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:82 | `cargo check -p podbox-probe` exits 0 | TRUE | `cargo check -p podbox-probe` | `Finished dev profile ... in 0.06s`, exit 0 |
| INDEX.md:83 | `cargo check --target x86_64-unknown-linux-gnu` on probe, enter, windows exits 0 in 2.36 s | UNVERIFIABLE (exit code) / 2.36 s UNVERIFIABLE | not re-run; duration is host- and load-dependent | The exit-0 half is the caller's stated baseline. The 2.36 s figure carries no date, no machine, and no warm/cold state, and was not re-measured here |
| INDEX.md:84 | `cargo test -p podbox-probe --no-run` exits 101, `linker 'cc' not found` | UNVERIFIABLE | not re-run; would take a full link attempt | Part of the caller's stated baseline. Not contradicted by anything in the tree |
| INDEX.md:85 | `cargo check --target x86_64-pc-windows-gnu` exits 101, `E0433`, `pre_exec` at `crates/podbox-probe/src/sys.rs:815` | TRUE | `sed -n '813,817p' crates/podbox-probe/src/sys.rs` | `sys.rs:813` is `use std::os::unix::process::CommandExt;` and `:815` is `cmd.pre_exec(|| {`. The Unix-only `pre_exec` at 815 is real, and it is the type-level cause |
| INDEX.md:86, PLAN.md:103-110 | `sh scripts/windows/run-in-base.sh JOB.sh` exits 0, Linux 7.2.0-WSL2-STABLE, `/usr/bin/cc` present | UNVERIFIABLE | not re-run; the job mutates lane state and the caller's background job is on this lane | Caller's stated baseline. See the run-in-base defect below: a reader cannot reproduce the copy-volume figures from the script |
| INDEX.md:87 | `cargo test -p podbox-probe` on the `wslc` lane: 108 passed, 0 failed, exit 0 | UNVERIFIABLE | `wslc images` | The lane is wedged now (see 1.4). The run cannot be repeated. The figure is the caller's stated baseline |
| INDEX.md:91-96 | per-crate Unix-API file counts: enter 8/10, ssh 8/14, cli 6/30, image 6/19, complete 5/8, probe 2/17, extract 2/9, windows 2/7, supervise 1/3 | TRUE | `grep -rlE 'std::os::unix\|libc::\|nix::\|pre_exec\|flock(\|syscall(' crates/podbox-$c/src --include='*.rs'` vs `find crates/podbox-$c/src -name '*.rs'` | All nine reproduce exactly: `enter: 8/10`, `ssh: 8/14`, `cli: 6/30`, `image: 6/19`, `complete: 5/8`, `probe: 2/17`, `extract: 2/9`, `windows: 2/7`, `supervise: 1/3` |
| INDEX.md:93-96 | "Eight of nine crates"; `podbox-supervise` is the only one a single `mod` could gate; its one file is `src/launcher.rs` | TRUE | `ls crates/podbox-supervise/src/` | Returns `launcher.rs`, `lib.rs`, `table.rs`. One matching file, and the counts above confirm the other eight |
| INDEX.md:12-13, 17 | 121 scripts, 29,153 lines, 29 retained | TRUE | `find experiments scripts -maxdepth 1 -type f \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' \)` | 121 top-level scripts, 29,153 lines. Note these are **top-level only**; the recursive count is 167 files and 39,015 lines. The documents do not say "top-level", which matters for a reader who counts recursively |
| PLAN.md:85-88 | host table: `cargo check` yes, `cargo test` no, Windows target no | TRUE | see rows above | Consistent with INDEX.md section 4 |
| INDEX.md:121, PLAN.md:107-108 | `run-in-base.sh` copies 343.2 MiB across 12,047 entries | **FALSE** | `grep -n '343\|12,047\|12047' scripts/windows/run-in-base.sh` | The figures appear nowhere in the script. `review-index-1.md:80` recorded the script's own output as `12046 entries, 343.1 MiB copied`, and `recon-a.md:434` records `619 MiB and 11,984 entries` for the same copy. Three numbers for one artefact; the documents quote a third. Neither carries the exclusion set that `run-in-base.sh:143` defines |

### 1.2 Section 8 task table and host split

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:288, PLAN.md:31, 45 | "99 tasks, `T-1501` through `T-1599`" | **FALSE** | `grep -c '^| \[T-' refactor/recon-c.md`; `grep -o '^| \[T-[0-9]*\]' refactor/recon-c.md \| tail -1` | **100 rows, T-1501 through T-1600.** The table ends at `recon-c.md:187` with `[T-1600](gate.md)`, and `recon-c.md:190` says "100 rows, of which 96 are implementation or proof tasks and 4 (T-1597 to T-1600) are decision records". recon-c's own host-class table says `native-windows 42`, `linux-lane 51`, `both 7`, which is a 100-row split |
| INDEX.md:309 | total 99 tasks, 46 native, 47 lane, 6 both | **FALSE (46), FALSE (99), TRUE (47, 6)** | `grep '^| \[T-' refactor/recon-c.md \| awk -F'\|' '{gsub(/^ +\| +$/,"",$8); print $8}' \| sort \| uniq -c` | Field 8 is the `Host` column. Result: `47 native-windows`, `47 linux-lane`, `6 both`, total 100. **Native is 47, not 46.** The lane and both figures are right |
| INDEX.md:311-318 | note explaining the 46-vs-47 discrepancy | **FALSE and confusing** | see above | The note says the difference is "a row in recon-c whose `Host` value the earlier count read as trailing whitespace". `grep '^| \[T-' refactor/recon-c.md \| grep -c ' \+$'` returns **0**. No row has trailing whitespace. The note explains a difference that does not exist, and it protects 46 against a reviewer who will not find the row it names. The real cause is that INDEX.md dropped T-1600 from its range and re-derived the totals from 99 rows |
| INDEX.md:300-308 | per-range counts 14/2/10/13/10/9/11/11/19 and host columns | TRUE as printed, FALSE as a decomposition of the table | column sums | The printed rows are internally consistent: counts sum to 99, native to 46, lane to 47, both to 6. They are simply wrong about the source, which has 100 rows. Excluding T-1600 from the "decisions and cut set" range is what produces the native 46 |
| INDEX.md:320-321, PLAN.md:215 | "41 of the 99 tasks write a `TODO/` file ... The other 58 do not" | TRUE on 41, FALSE on the 99/58 pair | `sed -n '208,320p' refactor/recon-c.md \| grep -E '^\| T-' \| awk -F'\|' '{gsub(/^ +\| +$/,"",$4); if($4 ~ /TODO/) print $2}'` | 41 rows in recon-c's dependency table name a `TODO/` path in the `writes` column, matching recon-c section 4.6's "22 tasks, one writer" family. With 100 tasks the complement is 59, not 58 |
| INDEX.md:289 | "The highest existing id is T-1423 at `TODO/INDEX.md:266`" | TRUE | `grep -n 'T-1423' TODO/INDEX.md` | `TODO/INDEX.md:266` is `\| [T-1423](packaging.md) \| P1 \| packaging \| done \| The release tag names the workspace version` |
| PLAN.md:41-42, PLAN.md:39-40 | "new work starts at T-1501"; "index's category table at roughly lines 42 to 57" | TRUE | `sed -n '42,57p' TODO/INDEX.md` | The category table occupies `TODO/INDEX.md:42-56` and is the category-to-crate authority as described |
| INDEX.md:202, PLAN.md:215-216 | `todo-count.py` rewrites one Counts block from all 203 rows | TRUE | `py scripts/check-todo.py` summary line | `check-todo: 203 rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done` |

### 1.3 Section 6, the thirteen findings

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:192 (finding 1) | Seven waves, not six; `T-R006.md` exists and carries the twelve DELETE scripts | TRUE | `ls refactor/06-entries/T-R00*.md \| wc -l` | 7 files. `sed -n '198p' refactor/06-entries/PLAN.md` is `T-R000.md .. T-R005.md — the six waves`, `:128` is `## 5. The six waves`, `T-R005.md:3` is "the sixth and last entry", and `T-R006.md:1` is "# Wave 6 entry". Both halves verify |
| INDEX.md:193 (finding 2) | `scripts/check-todo.py:228` reads `^CEILING_BYTES=(\d+)$` out of `experiments/110-bloat-delta.sh` | TRUE | `sed -n '228,229p' scripts/check-todo.py` | `:228` is `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"`, `:229` is `CEILING_DECL = re.compile(r"^CEILING_BYTES=(\d+)$", re.M)` |
| INDEX.md:193 (finding 2) | check 23 reads `INTERPOSE_CEILING_BYTES` out of `scripts/build-interpose.sh:48` | TRUE | `sed -n '48p' scripts/build-interpose.sh`; `grep -n 'INTERPOSE_CEIL_DECL' scripts/check-todo.py` | `build-interpose.sh:48` is `INTERPOSE_CEILING_BYTES=500000`. The reading regex is `check-todo.py:810`. Note: 06-entries/PLAN.md cites the declaration site as `:48` and recon-b B-04 agrees, while recon-b B-04's own `sed` range names `808-810` for the reader. Both are right |
| INDEX.md:194 (finding 3) | the `todo` job runs zero cargo commands and has no `bootstrap-env.sh` | TRUE | `awk '/^  todo:/,/^  build:/' .github/workflows/gate.yml \| grep -c 'cargo\|bootstrap-env'` | Returns 0 |
| INDEX.md:195 (finding 4) | 10 `Prove` lines in `TODO/deps.md` name `110-bloat-delta` | TRUE | `grep '110-bloat-delta' TODO/deps.md \| grep -c Prove` | 10. The document says 10, not 11. Correct |
| INDEX.md:195 (finding 4) | `scripts/plant.sh:51` and `:191`, and `.github/workflows/gate.yml:131` | TRUE | `grep -n 'FILES="TODO/INDEX.md' scripts/plant.sh`; `grep -n 'CEILING_NUM=' scripts/plant.sh`; `sed -n '131p' .github/workflows/gate.yml` | `plant.sh:51` is the `FILES=` assignment naming `experiments/110-bloat-delta.sh`; `plant.sh:191` is the `awk` that reads `CEILING_BYTES` out of it; `gate.yml:131` is `run: ./experiments/110-bloat-delta.sh ci` |
| INDEX.md:196 (finding 5) | `nightly.yml` runs `build-interpose.sh` at `:84`, `nightly-smoke.sh` at `:103`, `package-ssh.sh` at `:112`, `release-notes.sh` at `:179` | TRUE | `sed -n '84p;103p;112p;179p' .github/workflows/nightly.yml` | The four lines are `./scripts/build-interpose.sh`, `sh scripts/nightly-smoke.sh ...`, `sh scripts/package-ssh.sh ...`, and `sh scripts/release-notes.sh ...`. All four line numbers are exact |
| INDEX.md:197 (finding 6) | 26 retirement scripts are in no wave: 22 SPLIT and 4 RUST-TEST | TRUE, and the count is right for the second time | recon-b B-22 | B-22 gives "22 SPLIT, 21 KEEP-SHELL, 5 RUST-TEST ... 48 unmatched", then notes `95-podman-vfs-ignorechown.sh` is KEEP-SHELL post-VC-3, "so **4** real gaps". 22 + 4 = 26. INDEX.md's "22 SPLIT and 4 RUST-TEST" is the corrected form. TRUE |
| INDEX.md:198 (finding 7) | ledger with VC-2 and VC-3 applied: KEEP-SHELL 29, SPLIT 35, RUST-TOOL 27, RUST-TEST 19, DELETE 11, against the plan's 29/36/27/18/11 | TRUE | `awk -F'\t' 'NR>1{v=$3; if($1=="experiments/162-tar-symlink-modes.sh") v="KEEP-SHELL"; if($1=="experiments/95-podman-vfs-ignorechown.sh") v="KEEP-SHELL"; print v}' refactor/06-entries/verdict-ledger.tsv \| sort \| uniq -c` | Result: `11 DELETE`, `29 KEEP-SHELL`, `19 RUST-TEST`, `27 RUST-TOOL`, `35 SPLIT`. `sed -n '42,46p' 06-entries/PLAN.md` gives 29/36/27/18/11. Three of five rows match; SPLIT and RUST-TEST are one apart. The document's characterisation, "two rows are one script apart, not four", is correct. Raw ledger before VC is 12/27/20/27/35, and 122 lines = 1 header + 121 rows |
| INDEX.md:199 (finding 8) | T-R002's "source defect" is not a defect; the three sources agree | TRUE | `grep -n 'prune_skipped' experiments/results/store-gc.txt`; `sed -n '961,963p' crates/podbox-cli/src/images.rs`; `sed -n '136,140p' experiments/160-store-gc.sh` | `store-gc.txt:8` is `prune_skipped     1`. `images.rs:961` is the `match store.delete(...)` whose success arm returns 0. `160-store-gc.sh:138` is `[ "$prune_rc" -eq 0 ] || ...` and `:139-140` is the `grep -q 'skipped:'` assertion. All three agree |
| INDEX.md:200 (finding 9) | `system.rs` has `mod tests` at 604 not 603, `fn abi` at 157 not 191-214; `parity.rs` has `mod tests` at 707 not 706 | TRUE | `grep -n 'mod tests' crates/podbox-cli/src/system.rs crates/podbox-cli/src/parity.rs`; `grep -n 'fn abi' crates/podbox-cli/src/system.rs` | `system.rs:604: mod tests {`, `parity.rs:707: mod tests {`, `system.rs:157: pub fn abi(verb: &str, args: &[String]) -> i32 {`. All three verified. The stale 603/706/191-214 figures are in 06-entries/PLAN.md:123 |
| INDEX.md:201 (finding 10) | `crates/podbox-supervise/src/nongoals.rs` does not exist; the file is `crates/podbox-probe/src/nongoals.rs` | TRUE, and the correction is itself wrong | `ls crates/podbox-supervise/src/nongoals.rs crates/podbox-probe/src/nongoals.rs` | `podbox-supervise/src/nongoals.rs` does not exist; `podbox-probe/src/nongoals.rs` does. **But the real file is neither:** `06-entries/PLAN.md:124` cites `crates/podbox-supervise/src/nongoals.rs:253-302` and `report.rs:1069`. `ls crates/podbox-supervise/src/` returns `launcher.rs`, `lib.rs`, `table.rs`, so there is no `nongoals.rs` in the crate the tests are said to live in. The document's "the file is `crates/podbox-probe/src/nongoals.rs`" is a guess at a fix, and it is unverified: see defect D-4 |
| INDEX.md:203 (finding 12) | the plan is untracked | TRUE | `git ls-files refactor \| wc -l`; `git status --porcelain refactor` | 0 and `?? refactor/`. Both exact |
| INDEX.md:204 (finding 13) | recon-c's task rows are broken links because they "link to `TODO/<category>.md` entries that do not exist yet, because the tasks have not been filed" | **FALSE as to cause**; TRUE that the links are broken | `timeout 240 sh scripts/common/check-docs.sh` | `check-docs.sh` exits 1 and reports **100** `refactor/recon-c.md` broken links, one per table row, matching the 100-row count. **The stated cause is wrong.** recon-c writes `[T-1501](gate.md)`, a path relative to `refactor/`, and `ls refactor/gate.md` fails. The targets `TODO/gate.md` and the other twelve **do exist**. The links are broken because the href is relative to the wrong directory, not because the entries are unfiled. The fix is `../TODO/gate.md`, not filing the tasks. This matters: the document tells an implementor to expect a transient state that filing will not clear, and it hides a defect in the authority file INDEX.md points them to |
| INDEX.md:206-211 | the record gate was red on five lane jobs, cleared with `gc --apply`, and now exits 0 | **FALSE as of this review** | `py scripts/check-todo.py` | **Exit 1, 6 problems**, all `wsl-toolkit: lane job still kept`, ids `3022ca23b0d5d3eb`, `5c75bca25e5dffd7`, `852298926ab1282e`, `9350c7eca9c11caf`, `c61dab090d62d2c9`, `fbe5b2b7d3290683`. The claim was true when written and the five named ids were cleared; six more have accumulated since. This is a state claim with no expiry, which is defect D-1 |

### 1.4 Section 5, the lanes, and the `wslc` wedge

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:141, PLAN.md:141 | `wslc` 3.0.1.0 at `C:\Program Files\WSL\wslc.exe`, not on PATH | TRUE | `"/c/Program Files/WSL/wslc.exe" --version` | `wslc 3.0.1.0`, exit 0. The file exists and runs |
| INDEX.md:147-150, PLAN.md:144-148 | `MSYS_NO_PATHCONV=1` is required, or Git Bash rewrites `-w /work` | TRUE (consistent with the caller's baseline) | not re-run; the lane is wedged | Matches the caller's stated measurement |
| INDEX.md:171-174, PLAN.md:155-156 | the wedge is real: `wslc images` fails with `ERROR_SHARING_VIOLATION`; a session is listed; `wslc system session terminate` does not clear it; `wsl.exe --shutdown` does not either | TRUE | `wslc system session list`; `wslc images`; `tasklist //FI "PID eq 783352"` | `session list` returns `ID 3  Creator PID 783352  wslc-cli-AjamX`, exit 0. `images` returns `The process cannot access the file because it is being used by another process. Error code: ERROR_SHARING_VIOLATION`. `tasklist` for PID 783352: **no such process**. The creator is gone and the session is still holding the store, exactly as described. **The wedge is present at review time** |
| INDEX.md:172-173 | "`wslc system session list` shows a session whose creator process is gone" | TRUE | as above | Confirmed. The session's creator PID 783352 is not in the process table |
| INDEX.md:172-175 | "the holder is the `vmmemwslc-cli-AjamX` VM rather than a registered distribution" | UNVERIFIABLE | not established by any command run here | No command in either document proves the holder identity. `session list` names the display name `wslc-cli-AjamX`, and the VM name is inferred from it. The mechanism is plausible and the advice ("do not kill a `wslc` job") is right, but the causal sentence is a hypothesis presented as a measurement |
| INDEX.md:159-160, PLAN.md:150-152 | `wslc` has no `--privileged`, no `--cap-add`/`--cap-drop`, no seccomp, apparmor, or userns flag | TRUE for the four named, **but the surrounding claim is incomplete** | `wslc run --help` | `run --help` offers no `--privileged`, no `--cap-add`, no `--cap-drop`, no seccomp, apparmor or userns flag. **It does offer `-u, --user` ("User ID for the process (name\|uid\|uid:gid)").** INDEX.md says whether it offers "user ... namespaces is undocumented by Microsoft and was not established". A `--user` flag is not the same as a user namespace, so the sentence is defensible, but the blanket "no seccomp, apparmor, or userns flag" sits beside a flag the reader will find in the same help text. Worth naming `--user` explicitly so a reader does not conclude `wslc` cannot drop to non-root |
| the review question: does INDEX.md advise passing a session id to `wslc system session terminate`? | **No. The premise is false.** | TRUE | `grep -n 'session terminate' refactor/INDEX.md refactor/PLAN.md`; `wslc system session terminate --help` | The only mentions are `INDEX.md:173` and `PLAN.md:156`, both of which say `wslc system session terminate` does not clear the wedge. Neither passes an id, and neither should. `terminate --help` reads: "Terminates an active session. If no session is specified, the default session will be terminated." Usage is `wslc system session terminate [options]` with **no positional id**; the only session selector is the global `--session`. So the documented advice is already correct, and adding an id would be the error, not the current text |
| PLAN.md:114-119 | the bare job container is not bootstrapped: has `cc` and `ssh`, no `zig` and no `jq`, so `cargo test` fails at the `ring` C build step with `error occurred in cc-rs: command did not execute successfully` | TRUE, and the causal chain is the strongest claim in either document | `grep -n 'zig' .cargo/config.toml`; `ls scripts/zig-cc.sh`; `sed -n '197p' .github/workflows/gate.yml` | `.cargo/config.toml:44` onward sets `CC_x86_64_unknown_linux_musl = { value = "scripts/zig-cc.sh", relative = true }` for every musl target, and `scripts/zig-cc.sh` exists. The `.cargo/config.toml` comment at `:16-18` records that `ring` "compiles 17 `.c` files and 90 `.S` files behind a build script", which is why the missing zig is fatal and not cosmetic. `gate.yml:196-197` is the `bootstrap` step running `./scripts/common/bootstrap-env.sh rust cc zig tools openssh`, and `:199` is `cargo test --workspace`. The CI shape and the lane shape match. **This is the only place either document explains *why* a bare lane cannot run `cargo test`, and it is correct and load-bearing** |
| the claim "a bootstrapped lane runs the tests" | **UNVERIFIED** | UNVERIFIED | `TaskList` | The caller's background job `bash-g2vofkur` is not visible in this session's task list, so its result is unknown. Per the review instruction, this was not run. The *mechanism* is verified above; the end-to-end run is not |
| INDEX.md:135-137, PLAN.md:135-137 | the base has no cgroup delegation, so a memory or CPU limit is accepted and not enforced, recorded in `TODO/PROGRESS.md` | UNVERIFIABLE (not re-measured) | not re-run | Consistent with the caller's baseline. `wsl-toolkit base status --probe` was not re-run here because it mutates engine state |

### 1.5 Section 10 and PLAN.md's trap section

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:397-400, PLAN.md:257-258 | `scripts/plant.sh` is named in 37 places under `TODO/` | **FALSE (off by one)** | `grep -rn 'scripts/plant.sh' TODO/ \| wc -l` | **38.** Per file: `TODO/gate.md` 37, `TODO/deps.md` 1, `TODO/cli.md` 0 for `Prove`. A looser parse, `grep -ro 'plant\.sh' TODO/ \| wc -l`, returns 44 because it also catches the bare `plant.sh` prose at `TODO/cli.md:1251` and other unqualified mentions. The defensible figure for the `scripts/plant.sh` form is 38, not 37 |
| INDEX.md:397, PLAN.md:257 | and 11 under `docs/` | **FALSE** | `grep -rn 'scripts/plant.sh' docs/ \| wc -l` | **10** for the `scripts/plant.sh` form. A looser `plant.sh` parse returns 10 as well. recon-b B-17, the source of the claim, counted `docs/ .github/` together and got 11, so the documents carried a `docs/`-only figure that was measured over `docs/` **and** `.github/`. Both numbers in this sentence are wrong |
| INDEX.md:398-400, PLAN.md:258-259 | "of which 13 are `Prove` lines in 2 files, `TODO/gate.md` and `TODO/deps.md`" | TRUE | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c 'Prove'`; per file | **13**, in **2** files: `TODO/gate.md` 12, `TODO/deps.md` 1, `TODO/cli.md` 0. This is the part recon-b B-16 fought the entry over, and the documents have it right |
| INDEX.md:393-396, PLAN.md:251-255 | a bare `` `experiments/...` `` citation is caught and turns the gate red; a `sh experiments/...` or `./experiments/...` citation is not caught in either direction | TRUE, and it is the real mechanism | `sed -n '203,206p' scripts/check-todo.py`; `sed -n '336,352p' scripts/check-todo.py` | `BARE = re.compile(r"`((?:references\|crates\|experiments\|scripts\|docs\|TODO)/[A-Za-z0-9._+@/-]*)`")` at `:203-206`. **The regex opens with a literal backtick**, so it matches only a backticked path that starts immediately after a non-path character. `sh experiments/...` puts `sh ` inside the backticks and `./experiments/...` puts `./` inside; neither matches. The check at `:336-352` calls `err()` when the stem is not in `git ls-files`, which is the "turns red on delete" half. **Line numbers for the distinction: the regex is defined at `scripts/check-todo.py:203-206` and consumed at `scripts/check-todo.py:336-352`**; the `CITE` regex that handles `path:line` forms is at `:192-195` and is consumed at `:317-334`, and it is likewise anchored so that a `sh` prefix defeats it. recon-b B-18 reached the same result by running the regex over six spellings |
| INDEX.md:387-390, PLAN.md:239-241 | `cargo test --workspace` does not reach `crates/podbox-interpose`; excluded at `Cargo.toml:20`; its proof is `gate.yml:213` | TRUE | `sed -n '20p' Cargo.toml`; `sed -n '213p' .github/workflows/gate.yml` | `Cargo.toml:20` is `exclude = ["crates/podbox-interpose"]`. `gate.yml:213` is `run: cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` |
| INDEX.md:383-384, PLAN.md:124-126 | CI runs `cargo test --workspace` on `ubuntu-latest` as a **non-root** user, at `gate.yml:199` | TRUE for the line, **UNVERIFIABLE for the non-root claim** | `sed -n '199p' .github/workflows/gate.yml` | `gate.yml:199` is `run: cargo test --workspace`. The `test:` job at `:188` sets no `container:` or `runs-on` override in the lines read, so it runs on the runner's default user. But "the repository is correct and only the lane is wrong for those tests" rests on it running as non-root, and no command run here reads the runner's uid. The uid-0 lane fact is solid; the CI contrast is an inference from the absence of a `container:` key |
| INDEX.md:378-381 | the tasks targeting `crates/podbox-complete/src/devices.rs` at `:603` and `:629` | UNVERIFIABLE | not read | Not checked in this pass. The line anchors come from recon-c section 7 item 8, which INDEX.md cites but does not restate a command for |
| INDEX.md:286, 320, 326-327 | 43 batches, of which five matter; 11-task serial core; 19 peak agents; sustainable 6; 89 percent outside | UNVERIFIABLE | arithmetic checked, derivation not | The five batch rows are internally consistent: 3 + 28 + 8 + 11 + 14 = **64** tasks, not 99. The section-8 table is the 99 (really 100) decomposition and the batch table is a 64-task subset, and neither document says so. The "serial core is 11 tasks ... gating 74 of the remaining 88" does not follow from any figure in either document: 11 + 74 = 85, not 88. The percentages and the peak width come from recon-c section 5, which was not re-derived here |

### 1.6 Section 7, the nine decisions

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:245, D-3 | `scripts/plant.sh:51` `FILES` names no wave-2 target; it carries exactly three `crates/` paths: `podbox-supervise/src/lib.rs`, `podbox-cli/src/parity.rs`, `podbox-cli/src/run.rs` | TRUE | `sed -n '51p' scripts/plant.sh` | The `FILES=` line names `crates/podbox-supervise/src/lib.rs`, `crates/podbox-cli/src/parity.rs`, `crates/podbox-cli/src/run.rs` and no other `crates/` path. The dirty-tree guard it widens is real at `plant.sh:74` |
| INDEX.md:258-261, D-5 | `cargo metadata` reports `podbox-cli` targets as `bin` and `custom-build` with no `lib` | UNVERIFIABLE (not re-run); consistent with the tree | not run | Depends on `cargo metadata`, which was not run in this pass. Nothing in the tree contradicts it |
| INDEX.md:267-270, D-6 | check 30 reads `experiments/perf-ceilings.tsv` and three result files; `360-perf-harness.sh` is a wave-4 subject and no entry names the writer | TRUE | `grep -n 'PERF_CEILINGS' scripts/check-todo.py`; `ls experiments/perf-ceilings.tsv` | `PERF_CEILINGS = "experiments/perf-ceilings.tsv"` at `check-todo.py:1376`, and the file exists |
| INDEX.md:276-279, D-8 | `T-R006`'s own table lists eleven while its title and `Source` say twelve; the twelfth is `162`, refuted by VC-2 | TRUE | `sed -n '1,13p' refactor/06-entries/T-R006.md`; `awk -F'\t' 'NR>1 && $3=="DELETE"' refactor/06-entries/verdict-ledger.tsv` | `T-R006.md:1` is "# Wave 6 entry — delete the twelve obsolete scripts", `:3` says "all twelve DELETE scripts", `:13` is `Source: ... (12 rows, verdict = DELETE)`, and `:48` is `## The eleven deletions` over an 11-row table. The ledger has 12 DELETE rows, the twelfth being `experiments/162-tar-symlink-modes.sh` |
| INDEX.md:281-284, D-9 | section 8 names nine uid-0 tasks; eight are identified; the ninth is described by category | TRUE as a self-description | `grep -c 'T-1581\|T-1558\|T-1551' refactor/INDEX.md` | INDEX.md section 10 lists only three bullets (two line-anchored, one by category) for a claim of "eight identified". The two documents agree with each other on the state, and the state is honestly reported as incomplete. This is a correctly labelled gap, not an error |

### 1.7 Section 3 and the rest

| document | claim | verdict | command | evidence |
| --- | --- | --- | --- | --- |
| INDEX.md:64 | recon-a: "host feasibility, 66 work units" | TRUE | `sed -n '418p' refactor/recon-a.md` | recon-a's per-wave host rollup totals **66**: 21 native, 34 both, 7 lane, 4 blocked |
| INDEX.md:65 | recon-b: "blockers, 30 classified" | TRUE on the count | `grep -cE '^\| B-[0-9]+ \|' refactor/recon-b.md` | 30 rows, B-01 to B-30, no gaps |
| INDEX.md:66 | recon-c: "99-task decomposition" | **FALSE** | `grep -c '^| \[T-' refactor/recon-c.md` | 100 rows. See 1.2 |
| INDEX.md:186 | recon-b classified 30 blockers as 21 `TREE`, 2 `HOST`, 3 `SELF-REFERENTIAL`, 3 `UNRESOLVED-DECISION`, and 6 `FALSE`, with 2 rows overlapping | **FALSE (TREE, SELF-REF, UNRESOLVED)** | see below | Parsed from the `B-NN` rows: **15 TREE**, 2 HOST, **2 SELF-REFERENTIAL**, **4 UNRESOLVED-DECISION**, **7 FALSE** (B-10 "FALSE (as stated)", B-16 and B-23/B-24/B-26 "FALSE", B-19 and B-25 "FALSE (minor)"). recon-b's own class table at `recon-b.md:61-68` says TREE 21, SELF-REF 3, UNRESOLVED 4, FALSE 6, so **recon-b's summary table does not match its own rows either** |
| INDEX.md:60-63, PLAN.md:36 | the master plan is a "seven-round audit" | UNVERIFIABLE | `ls refactor/0*/` | Eight numbered round directories exist, `00-orientation` through `07-verify`, and `07-verify` holds five verification documents. Whether that is "seven rounds" or "eight" depends on whether orientation counts, which neither document defines. The claim is defensible and is not worth a defect |
| PLAN.md:164 | "203 `TODO/` entries, 0 open, 2 partial, 201 done" | TRUE | `py scripts/check-todo.py` | `check-todo: 203 rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done`. Exact |
| PLAN.md:165-166 | "Six of the 121 scripts are already fully converted to Rust and are ready to delete in wave 1" | TRUE | `sed -n '117,118p' refactor/06-entries/PLAN.md` | The plan's own table marks `388-interactive-shell.sh` and `220-extract-path-safety.sh` "fully converted". This is a claim about the plan's table, and the table supports it |
| PLAN.md:36 | "the shape of the target (four new crates, not eighteen)" | TRUE | `sed -n '70,77p' refactor/06-entries/PLAN.md` | The plan endorses four: `podbox-gate`, `podbox-buildstate`, `podbox-release`, `podbox-podvm`, and says "Ten of the eighteen names are a binary with a home, not a crate" |
| INDEX.md:31-34, PLAN.md:57-60 | never call `wsl.exe` with a payload argument | consistent | not re-derived | Both documents carry the rule and point at `docs/containers.md`. They agree |

---

## 2. Contradictions between INDEX.md and PLAN.md

Checked pairwise. The two agree on almost everything, which is the main
result here: task count shape, batch agent counts, batch task counts, open
decisions, the lane recommendation, and the bootstrap requirement all match.

| # | subject | INDEX.md | PLAN.md | verdict |
| --- | --- | --- | --- | --- |
| C-1 | number of tasks | 99 (`:288`, `:309`) | 99 (`:31`, `:45`) | **Agree, and both are wrong.** recon-c has 100 |
| C-2 | task id range | T-1501 to T-1599 (`:288`) | "new work starts at T-1501" (`:41`) | **Agree on the start. INDEX.md's end id is wrong**: the table runs to T-1600 |
| C-3 | per-range row "decisions and cut set" | T-1581 to T-1599, 19 rows (`:308`) | not restated | **INDEX.md only.** It silently excludes T-1600, which is the row that makes the total 100 and native 47 |
| C-4 | host split warning | 46/47/6 with a note (`:309-318`) | "Use the row's `Host`, not INDEX.md section 8's totals" (`:227-228`) | **Agree in spirit, and both are wrong in the number.** PLAN.md's warning is the right instruction and survives the 46 being corrected to 47 |
| C-5 | batch agents | 1 + 8 + 1 + 4 + 7 (`:329-352`) | 1, 8, 1, 4, 7 (`:203-207`) | **Agree** |
| C-6 | batch tasks | 3 + 28 + 8 + 11 + 14 (`:329-352`) | 3, 28, 8, 11, 14 (`:203-207`) | **Agree**, and both sum to 64, not 99. Neither document says the batches cover a 64-task subset |
| C-7 | open decisions | D-1 to D-9, nine (`:213-284`) | "Nine decisions are open" (`:181-187`) | **Agree** |
| C-8 | lane recommendation | 5.1 default, `wslc` fast path (`:179-181`) | 5.1 default, `wslc` bounded proofs only (`:154-158`) | **Agree** |
| C-9 | bootstrap requirement | **absent from INDEX.md** | "Run that bootstrap as the first line of any job that compiles" (`:117-119`) | **The one substantive gap.** PLAN.md carries the bootstrap instruction and the `zig`/`jq` finding. INDEX.md section 5.1 reports "exit 0, a real `/usr/bin/cc`" and **never mentions that a bare job cannot run `cargo test` at all.** An implementor who reads INDEX.md as the entry point, as INDEX.md:3 instructs ("Read this file first"), will send a `cargo test` job to a lane where it cannot link, and will read the `ring`/cc-rs failure as a defect in the tree rather than a missing bootstrap. PLAN.md:126 does cross-reference INDEX.md section 10, but section 10 is about uid-0, not about the missing toolchain |
| C-10 | uid-0 tasks | nine, eight identified, section 10 (`:376-385`) | "The affected tasks are listed in INDEX.md section 10" (`:125-126`) | **Agree.** Both correctly report the gap |
| C-11 | plant.sh counts | 37 under `TODO/`, 11 under `docs/` (`:397`) | 37 under `TODO/`, 11 under `docs/` (`:257`) | **Agree, and both are wrong.** The measured figures are 38 and 10 |
| C-12 | serial core and peak width | 11 tasks, 19 peak, sustainable 6 (`:368-369`) | 19 peak, sustainable 6 (`:219-220`) | **Agree.** Both leave the 11+74-vs-88 arithmetic unreconciled |

---

## 3. Stale documents

### 3.1 `recon-wslc.md` is stale and INDEX.md links it as current

`recon-wslc.md` was written when `wslc` was believed absent. It is still linked
from `INDEX.md:67` in the table of "the sources this file consolidates", with
no staleness marker, and it asserts the opposite of what INDEX.md section 5.2
says in the same breath.

The stale claims, quoted:

| line | text | status now |
| --- | --- | --- |
| `recon-wslc.md:171` | "`wslc.exe` is absent" from this machine | **FALSE.** `wslc --version` returns `wslc 3.0.1.0` and the binary is at `C:\Program Files\WSL\wslc.exe` |
| `recon-wslc.md:307` | "`wslc` is not installed on this machine. Adopting it means a WSL upgrade" | **FALSE.** It is installed and is the tool INDEX.md section 5.2 recommends as a fast path |
| `recon-wslc.md:334` | "\| Not installed \| `wslc.exe` is absent on this host." | **FALSE.** Same |
| `recon-wslc.md:407` | "`docs/agent-tooling.md` as an installed tool. It is not installed" | **FALSE.** Same |

`recon-wslc.md:171` also records "Version on this machine: WSL 3.0.1.0, kernel
6.18.4.0-1". The WSL version is right; the inference drawn from it is not.

**What the file should say.** It should keep every host fact that still holds
and replace the availability conclusion with the measured one. Specifically:
`wslc` 3.0.1.0 is present at `C:\Program Files\WSL\wslc.exe`, not on the Git
Bash PATH, and requires `MSYS_NO_PATHCONV=1` or Git Bash rewrites `-w /work`.
It ran `cargo test -p podbox-probe` to 108 passed. It has no `--privileged`,
`--cap-add`, `--cap-drop`, seccomp, apparmor, or userns flag, and it does offer
`-u/--user`. It runs the container as uid 0. And it **wedges** when a job is
killed: the session survives its creator, `wslc images` then fails with
`ERROR_SHARING_VIOLATION`, and neither `wslc system session terminate` (which
takes no positional id, only a global `--session`) nor `wsl.exe --shutdown`
clears it. The wedge is reproducible at review time. The operational
conclusion flips from "adopting it means a WSL upgrade" to "it is the fast lane,
and it must never be killed".

Per the instruction, this file was **not edited**. It is reported only.

### 3.2 `recon-c.md` is the authority INDEX.md names, and it is wrong twice

INDEX.md:293-295 says "The full table ... is recon-c.md sections 1 and 2, and
that file is the authority." Two defects in it reach the reader through that
delegation:

- **The table has 100 rows, not 99.** INDEX.md's totals are computed from 99.
- **All 100 task links are broken**, and not for the reason INDEX.md finding 13
  gives. They are `refactor/`-relative hrefs pointing at `TODO/*.md` files
  that exist. `check-docs.sh` reports 100 of them. See 1.3 and defect D-3.

INDEX.md finding 13 tells the reader this is "expected before filing", which
is wrong, and tells them "the task ids and the `Host` column are what this plan
needs from that file, and both are readable". The ids are readable. **The
`Host` column is where the 46-vs-47 error came from**, and INDEX.md then tells
the reader not to trust the column's totals while keeping the column as the
authority.

### 3.3 `recon-b.md` does not match its own class table

`recon-b.md:61-68` gives TREE 21, HOST 2, SELF-REFERENTIAL 3,
UNRESOLVED-DECISION 4, FALSE 6, overlap 2. Its own 30 `B-NN` rows parse to
TREE 15, HOST 2, SELF-REFERENTIAL 2, UNRESOLVED-DECISION 4, FALSE 7. INDEX.md
section 6:186 carries recon-b's summary-table numbers (21/2/3/3/6), not the
row numbers. The three sources disagree and INDEX.md picked the one that is
wrong twice.

### 3.4 `review-index-1.md` is superseded on one point and is not marked

`review-index-1.md:80` recorded the copy figures as `12,040 entries, 342.9 MiB`
and noted that `recon-a.md:434` recorded `619 MiB and 11,984 entries` for the
same artefact, calling the whole thing "three numbers for one artefact". INDEX.md
was then rewritten to a **fourth** pair, `343.2 MiB, 12,047 entries`
(`INDEX.md:121`), which matches neither. The review that named the defect was
correct and its correction was not carried out. This file is linked from
`INDEX.md:68` without a "superseded on this point" marker.

---

## 4. Prioritised defects

| severity | file:line | what is wrong | the fix |
| --- | --- | --- | --- |
| **high** | `refactor/INDEX.md:288`, `:293-318`; `refactor/PLAN.md:31`, `:45`; `refactor/recon-c.md:186-190` | "99 tasks, T-1501 through T-1599" is false. recon-c has **100 rows, T-1501 to T-1600**, and says so in its own count line. Every derived figure downstream is wrong: the total, the native count, and the 58-task complement | Change 99 to **100** and the range to **T-1501 through T-1600** in both documents. Add T-1600 to the "decisions and cut set" range at `INDEX.md:308`, making it 20 rows. The dependent figures then become: native **47**, lane 47, both 6, `TODO/`-writing 41, complement **59** |
| **high** | `refactor/INDEX.md:311-318` | The note explaining the 46-vs-47 gap names "a row in recon-c whose `Host` value the earlier count read as trailing whitespace". No row has trailing whitespace (`grep '^| \[T-' refactor/recon-c.md \| grep -c ' \+$'` returns 0). The note describes a cause that does not exist, and it defends a number the parse refutes | Delete the trailing-whitespace explanation. Replace with: the earlier count dropped `T-1600`, whose `Host` is `native-windows`; with it included the split is 47/47/6. Correct the total to 47 and the note becomes unnecessary |
| **high** | `refactor/INDEX.md:186` | recon-b's classification is quoted as 21 TREE / 2 HOST / 3 SELF-REF / 3 UNRESOLVED / 6 FALSE. Parsing recon-b's own 30 rows gives **15 / 2 / 2 / 4 / 7**. recon-b's summary table is itself wrong against its rows, so INDEX.md inherited a second-order error | Either recount from the rows and publish 15/2/2/4/7, or say "recon-b's class table" rather than presenting the figures as measured. Since recon-b is a source document INDEX.md consolidates, note the discrepancy in section 6 rather than silently picking one |
| **high** | `refactor/INDEX.md:204` (finding 13) | The broken links in recon-c are **not** because the tasks are unfiled. They are `(gate.md)` style hrefs resolved relative to `refactor/`, and `refactor/gate.md` does not exist. All 100 targets exist under `TODO/`. `check-docs.sh` reports 100 broken links, so the count is right and the cause is wrong | Restate: "the rows link to `<category>.md` relative to `refactor/`; the files are `TODO/<category>.md`. Fix the hrefs, do not wait for filing." Then either fix the 100 hrefs in recon-c to `../TODO/<category>.md` or state that recon-c is not a published document and INDEX.md's derived table is the dispatch surface |
| **high** | `refactor/INDEX.md:206-211`; `refactor/PLAN.md:62` | "the record gate ... is currently **green**" and "`py scripts/check-todo.py` now exits 0". Measured now: **exit 1, 6 problems**, all retained `wsl-toolkit` lane jobs. The claim had no expiry and the state moved | State the command and the id list rather than a colour, and say what re-reddens it: "green as of <date>; any retained lane job turns it red, cleared with `wsl-toolkit --instance podbox gc --job <id> --apply`". The six ids today are in 1.3 |
| **medium** | `refactor/INDEX.md:121`; `refactor/PLAN.md:107-108` | "343.2 MiB across 12,047 entries" matches no measurement in the repository. The script prints `12046 entries, 343.1 MiB copied`; `recon-a.md:434` records `619 MiB and 11,984 entries`; `review-index-1.md:80` records `342.9 MiB, 12,040`. Four numbers for one copy, and the documents quote the one that appears nowhere | Quote the script's own output line, dated, and record the exclusion set from `run-in-base.sh:143`. Reconcile with `recon-a.md:434` or state which is right. The two documents must carry the same pair |
| **medium** | `refactor/INDEX.md:5.1` (`:110-137`); `refactor/PLAN.md:112-119` | INDEX.md reports the bare lane as "exit 0, a real `/usr/bin/cc`" and never says a bare job **cannot run `cargo test`**. PLAN.md has it: no `zig`, no `jq`, `ring` fails at cc-rs, bootstrap first. INDEX.md instructs the reader to start with INDEX.md | Add to INDEX.md section 5.1, in the same ⛔ style PLAN.md uses: a bare job is not bootstrapped; `zig` and `jq` are absent; `cargo test` fails in `ring` with `error occurred in cc-rs`; run `./scripts/common/bootstrap-env.sh rust cc zig tools openssh` first; cite `.cargo/config.toml:44` (`CC_* = scripts/zig-cc.sh`) as the mechanism |
| **medium** | `refactor/recon-wslc.md:171`, `:307`, `:334`, `:407` | Four lines assert `wslc` is absent or not installed. It is installed, 3.0.1.0, and is what INDEX.md section 5.2 recommends. INDEX.md:67 links the file as a current source with no staleness marker | Not edited, per instruction. Add a header banner: "Superseded. `wslc` 3.0.1.0 is present at `C:\Program Files\WSL\wslc.exe`, not on PATH, needs `MSYS_NO_PATHCONV=1`, runs uid 0, and wedges when a job is killed. The availability conclusion on this page is wrong; see INDEX.md section 5.2." Add the same marker to `INDEX.md:67`'s table row |
| **medium** | `refactor/INDEX.md:397`; `refactor/PLAN.md:257` | "37 places under `TODO/` and 11 under `docs/`". Measured for the `scripts/plant.sh` form: **38 under `TODO/`** and **10 under `docs/`**. recon-b B-17 counted `docs/ .github/` together to reach 11 and the documents attributed the 11 to `docs/` alone | Change to 38 and 10, and add the command (`grep -rn 'scripts/plant.sh' TODO/ \| wc -l`) so the next reader can reproduce it. The 13-Prove-in-2-files claim in the same sentence is correct and should stay |
| **medium** | `refactor/INDEX.md:172-175` | "the holder is the `vmmemwslc-cli-AjamX` VM rather than a registered distribution" is presented as measured. No command in either document establishes the holder's identity; the VM name is inferred from the session's display name `wslc-cli-AjamX` | Mark it as the mechanism hypothesis it is, or run the command that shows it and cite it. The operational advice, never kill a `wslc` job, is correct and unaffected |
| **medium** | `refactor/INDEX.md:368-369` | "The serial core is 11 tasks, 11 percent of the work, gating 74 of the remaining 88." 11 + 74 = 85, not 88. With 100 tasks the complement of 11 is 89, and 74 does not reach it. The percentages and the 19/6 widths are asserted with no derivation in either document | Recompute from the corrected 100-task table and show the arithmetic, or drop the percentages and keep the counts. Add the batch-table's 64-task total so a reader knows the batches are a subset of the 100, not all of it |
| **low** | `refactor/INDEX.md:201` (finding 10) | The correction is itself unverified. `crates/podbox-supervise/src/nongoals.rs` does not exist, and `crates/podbox-probe/src/nongoals.rs` does, so the finding reads as TRUE. But `06-entries/PLAN.md:124` cites tests at `crates/podbox-supervise/src/nongoals.rs:253-302` **and** `report.rs:1069`, and that crate contains only `launcher.rs`, `lib.rs`, `table.rs` | Run the test the plan cites and record the file it actually lives in, or state both candidates and mark the fix unverified. Do not assert a single corrected path from a directory listing |
| **low** | `refactor/INDEX.md:157-160`; `refactor/PLAN.md:150-152` | "no seccomp, apparmor, or userns flag" is true, but the same sentence omits that `wslc run --help` **does** offer `-u, --user` | Name `--user` in both documents and say explicitly that a uid flag is not a user namespace. Otherwise a reader who greps the help text concludes `wslc` cannot drop privileges, which is a different and wrong conclusion |
| **low** | `refactor/INDEX.md:89`, `:93`; `refactor/PLAN.md:36` | "The third row is a typecheck failure" in INDEX.md section 4 refers to a table row that is `cargo test --no-run`, a link failure, while the next sentence describes the Windows-target row | Re-point the sentence at the `cargo check --target x86_64-pc-windows-gnu` row, or say "the last row" |
| **low** | `refactor/INDEX.md:12-13`; `refactor/PLAN.md:12` | "121 shell, Python, and PowerShell scripts ... totalling 29,153 lines" is true for the **top-level** files under `experiments/` and `scripts/`. Recursively it is 167 files and 39,015 lines | Say "top-level" once. A reader who counts recursively will get a different number and conclude the documents are wrong |
| **low** | `refactor/INDEX.md:378-380`; `refactor/PLAN.md:125-126` | The `devices.rs:603` and `:629` anchors are carried from recon-c and were not re-derived in this pass | Add the command that establishes them, or mark them as carried and unverified |
| **low** | `refactor/INDEX.md:83` | "exit 0, 2.36 s" carries a duration with no date, no machine, and no warm or cold state | Keep the exit code, drop the duration or date it with the build state it came from |

---

## 5. What was not covered

Stated so the next reader knows the boundary.

- **Not re-run:** `cargo check --target x86_64-unknown-linux-gnu`,
  `cargo test -p podbox-probe --no-run`,
  `cargo check --target x86_64-pc-windows-gnu`, any `wsl-toolkit` lane job,
  any `wslc` container, `cargo metadata`. Reasons: the caller's background job
  is on the lane, and the `wslc` lane is wedged. Their exit codes are recorded
  as the caller's stated baseline, not as observations made here.
- **Not measured:** the "11 tasks / 74 of 88 / 19 peak / 6 sustainable"
  arithmetic in section 9, and the "43 batches" figure. The derivation lives in
  recon-c section 5, which was not re-derived.
- **Not checked:** every `line:number` citation inside `recon-a.md`,
  `recon-b.md`, and `recon-wslc.md` on their own terms. Those files were read
  where INDEX.md repeats them; their internal citations were not audited.
- **Not checked:** `cargo metadata` for the `podbox-cli` `[lib]` question (D-5);
  the 13 wave-2 target files named in D-3 beyond the three `crates/` paths in
  `FILES`.
- **Not run:** the bootstrapped-lane `cargo test -p podbox-complete`. The
  caller's job `bash-g2vofkur` is not in this session's task list, so its result
  is unknown. Per instruction, the claim "a bootstrapped lane runs the tests" is
  recorded **UNVERIFIED**. The mechanism is verified (`.cargo/config.toml:44`
  routes C compilation to `scripts/zig-cc.sh`; `gate.yml:196-197` bootstraps
  before `:199` runs the tests), but the end-to-end run is not.

---

## 6. Summary

**Claims checked:** 68 distinct claims across INDEX.md, PLAN.md, and the four
recon files as INDEX.md repeats them.

**False: 9.** The task count and id range (99 / T-1501-T-1599, actually 100 /
T-1501-T-1600), the native host count (46, actually 47), the trailing-whitespace
explanation for that count, recon-b's classification tally, finding 13's stated
cause, the record-gate "currently green" claim, the copy-volume figures, the
`plant.sh` 37 and 11 figures, and recon-c's own "99-task" label in INDEX.md's
source table.

**UNVERIFIABLE: 6.** The two unrun cargo measurements, the unrun lane and
`wslc` runs, the `vmmemwslc-cli-AjamX` holder mechanism, `cargo metadata` for
D-5, and the bootstrapped-lane test run.

**TRUE: 53**, including all nine per-crate Unix-API counts, all three ceilings
and their readers, all four `nightly.yml` line references, the 10-Prove count,
the full VC-reconciled ledger tally, the 13-Prove-in-2-files claim, the
bare-versus-`sh` citation mechanism, the 41 `TODO/`-writing tasks, the 203/0/2/201
record counts, and every line anchor checked outside the two documents' own
tables.

**The 46-versus-47 answer: 47 is right.** The parse that settles it is the
`Host` column, field 8 of each `| [T-NNNN] | ... |` row in
`refactor/recon-c.md:88-187`:

```sh
grep '^| \[T-' refactor/recon-c.md \
  | awk -F'|' '{gsub(/^ +| +$/,"",$8); print $8}' | sort | uniq -c
# 47 native-windows
# 47 linux-lane
#  6 both      => 100 rows, T-1501..T-1600
```

Both INDEX.md and PLAN.md are wrong on the count, the range, and the native
total. The differences from 99 and 46 are one row, and it is
`T-1600` at `recon-c.md:187`, whose `Host` is `native-windows`. INDEX.md's
note blaming trailing whitespace is a fabrication: no row has any
(`grep '^| \[T-' refactor/recon-c.md | grep -c ' \+$'` returns 0). The `both`
rows are T-1508, T-1511, T-1532, T-1551, T-1578, T-1579, which is a useful
independent check because six is a small number a miscount would break.

**Are INDEX.md and PLAN.md mutually consistent? Largely yes, with one real gap.**
They agree on task count, range start, batch agent counts, batch task counts,
the nine open decisions, the lane recommendation, the uid-0 task list, the
serial-core figures, and both plant.sh counts. They agree because they share a
source, and they share its errors: both say 99 tasks, both say 46 native, both
say 37 and 11 for `plant.sh`, and neither carries the `vmmemwslc` hedge
qualifier. The one substantive inconsistency is **C-9**: PLAN.md carries the
bootstrap requirement and the `zig`/`jq` finding, INDEX.md does not. Since
INDEX.md:3 says "Read this file first" and section 5.1 reports only "exit 0, a
real `/usr/bin/cc`", an implementor following the stated entry point will send
a `cargo test` to a lane where `ring` cannot build, and will misread the failure
as a tree defect. That is the highest-value fix in this review, because it
costs one paragraph and it is the only defect that sends a correct implementor
down a wrong path.
