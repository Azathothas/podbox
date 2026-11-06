# Verify PLAN.md sections 1, 2, 3 and 7

Scope: `refactor/06-entries/PLAN.md` section 1 (the pipeline summary),
section 2 (the verdict distribution), section 3 (the four-crate
architecture) and section 7 (the standing corrections VC-1..VC-7), with
`refactor/06-entries/T-R000.md` and `refactor/06-entries/verdict-ledger.tsv`
as the two documents section 7 leans on.

## Verify PLAN.md sections 1, 2, 3 and 7 — scope and method

### What I read

Read in full: `AGENTS.md`, `TODO/RULES.md`, `TODO/INDEX.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `refactor/06-entries/PLAN.md`,
`refactor/06-entries/T-R000.md`, `refactor/06-entries/verdict-ledger.tsv`,
`docs/methodology/gate.md`, `experiments/README.md`, `Cargo.toml`,
`.github/workflows/gate.yml`, `crates/podbox-interpose/Cargo.toml`,
`crates/podbox-cli/Cargo.toml`.

Read the cited ranges of: `crates/podbox-cli/src/parity.rs`,
`crates/podbox-image/src/store.rs`, `crates/podbox-cli/src/machine.rs`,
`crates/podbox-cli/src/machine.rs`, `crates/podbox-interpose/src/lib.rs`,
`scripts/check-todo.py`, `scripts/plant.sh`, `scripts/build-interpose.sh`,
`experiments/lib/engine.sh`, `experiments/157-lock-inheritance-prove.sh`,
`experiments/95-podman-vfs-ignorechown.sh`, `experiments/355-parity-curated.sh`,
`experiments/20-enter-target.sh`, `experiments/300-run.sh`,
`experiments/371-validationos-stream.sh`, `experiments/370-windows-guest.sh`,
`experiments/results/perf-kvm.txt`, `experiments/results/perf-lane.txt`,
`experiments/results/parity-drive.txt`, `experiments/results/attribute.txt`,
`experiments/results/parity-curated.txt`, `crates/podbox-interpose/interpose.map`.

Read for cross-checking: all ten `refactor/01-audit/group-*.md` verdict
bodies and header tables, `refactor/02-peer-review-1/round-1.md` sections
1 and the corrections list, `refactor/03-peer-review-2/round-2.md` sections
"verdict changes required" and "KEEP-SHELL verdict audit", and
`refactor/04-meta/meta-1.md` sections on the verdict table and waves.

### What I checked

- **194 citations opened by line number.** 121 ledger `report_line` values,
  20 `gate.yml` step lines in T-R004, 8 `gate.yml` lines in PLAN, 4
  `crates/podbox-interpose/Cargo.toml` lines, 5 `store.rs`/`157` lines,
  6 `scripts/check-todo.py` and `scripts/plant.sh` lines,
  `build-interpose.sh` at `:44-52` and `:195-216`, 24 lines across the
  14 T-R000 corrections, and the `TODO/`, `docs/`, and `experiments/`
  citation in each.
- **16 numbers recounted with my own command**, listed in the number
  table below.
- **All 14 T-R000 corrections checked against the artefact**, not the record.
- **The 121 `verdict-ledger.tsv` rows checked against the group report
  bodies** by re-deriving each verdict from its body section, not by
  reading the ledger's own label.

### What I could not settle

- The disjoint partition behind PLAN section 2's KEEP-SHELL breakdown.
  Round 2 states the categories as a partition over 27 scripts, but its own
  items sum to 30 and its categories overlap. The grouping is not
  reconstructible from the reports. I report the sum, not a verdict.
- Whether "43 converge" was intended as RUST-TEST + SPLIT (54) or
  RUST-TEST + RUST-TOOL (45). Neither yields 43. See the number table.
- `git log --follow` provenance for `attribute.txt` beyond the two
  commits `git log` returns; `6eb941f` is confirmed as the re-capture.
- The Rust build. `cargo test --workspace` cannot run on this host
  (`linker 'cc' not found`). No compile-time claim is verified by me.

---

## Verify PLAN.md sections 1, 2, 3 and 7 — citation table

| citation | claim | what the line says | verdict |
| --- | --- | --- | --- |
| `refactor/00-orientation/assignment-map.md` (exists) | round-0 artefact, 121 scripts / 29,153 lines / ten groups | `assignment-map.md:3` reads `Total scripts: 121, total lines: 29153, groups: 10` | TRUE |
| `refactor/01-audit/group-1.md` .. `group-10.md` (exist) | ten auditors | all ten files present | TRUE |
| `refactor/02-peer-review-1/round-1.md` (exists) | 1,760 citations checked, 5 wrong, 16 corrections required | `:17` reads `I extracted 1,760 path:line citations`; `:33` reads `1,760 citations checked, 5 wrong (0.28%)`; the corrections list at `:678` has 16 numbered items | TRUE |
| `refactor/03-peer-review-2/round-2.md` (exists) | corrections applied, 8 open items settled, VC-1..VC-7 | `## Round 2 — open items settled` has 8 `O<n>` items; VC-1..VC-7 defined at `:375,385,394,402,410,486,492` | TRUE |
| `refactor/04-meta/meta-1.md` (exists) | 980 test functions | `meta-1.md:83` total column reads `980`; I recounted 980 under `crates/` | TRUE |
| `refactor/05-meta-review/meta-review-1.md` (exists) | 2 claims corrected, 3 waves rejected, 4 crates endorsed | `:143` `Claim 2 ... CORRECTED`; `:179` `Claim 3 ... CORRECTED`; `:246` `Claim 4 ... CORRECTED`; waves 3, 6, 7 marked REJECT at `:692,696,697`; `:613` `The endorsed list, four crates` | TRUE |
| `TODO/INDEX.md:42-58` | assigns each behaviour a category and a crate; the four target categories exist | `:42` is the table header `\| category \| crate \| specification \|`; `:56` is the `gate` row; `packaging` `:53`, `podvm` `:55`, `gate` `:56` all present | TRUE |
| `TODO/RULES.md` | has no crate, member or workspace rule | `grep -n -i -E "crate\|member\|workspace\|cargo" TODO/RULES.md` exits 1, no match | TRUE |
| `Cargo.toml:20` | `crates/podbox-interpose` is excluded | `:20` reads `exclude = ["crates/podbox-interpose"]` | TRUE |
| `crates/podbox-interpose/Cargo.toml:15` | `crate-type = ["cdylib"]` | `:15` reads `crate-type = ["cdylib"]` | TRUE |
| `crates/podbox-interpose/Cargo.toml:18` | `[dependencies]` is empty | `:18` reads `[dependencies]`; `:19` is blank; no key follows | TRUE |
| `.github/workflows/gate.yml:99` | the interposer build runs here | `:99` reads `run: ./scripts/build-interpose.sh` | TRUE |
| `.github/workflows/gate.yml:56` | `plant.sh` runs as the step `every check can fail` | `:55` reads `- name: every check can fail`; `:56` reads `run: ./scripts/plant.sh` | TRUE |
| `gate.yml:197` / `:211` (meta report's citation) | the test-job commands | `:197` is blank; `:211` reads `env:` | FALSE — the meta report was right to be corrected, and PLAN section 5's correction is right |
| `gate.yml:199` / `:213` | the test-job commands | `:199` reads `run: cargo test --workspace`; `:213` reads the interposer `cargo test` | TRUE |
| `gate.yml:37,42,59,71,102,107,131,154,159,164,171,176,186,202,205,208` | the remaining job steps in T-R004's table | all 16 resolve to the named step bodies | TRUE |
| `crates/podbox-interpose/interpose.map` | declares 112 names | 112 names inside the `global:` block (`:15` to `:127`), counted with awk between `global:` and `local:` | TRUE |
| `scripts/build-interpose.sh:197-214` | already compares `nm -D` output against `interpose.map` | `:199-203` build the declared and exported name lists, `:204` diffs them, `:214` closes the branch | TRUE |
| `scripts/build-interpose.sh:204-205` (T-R004) | prints `ok: exports %s names` here | the printf is at `:205` alone; `:204` is the `if diff -q` line | IMPRECISE — off by one, harmless |
| `scripts/build-interpose.sh:48` | the byte ceiling | `:48` reads `INTERPOSE_CEILING_BYTES=500000` | TRUE |
| `scripts/check-todo.py:228` | `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` | `:228` reads exactly that | TRUE |
| `scripts/check-todo.py:300` | `check_tree` reads every tracked file | `:300` reads `def check_tree(files):`; its docstring says `Checks 11 to 14: citations and links outside TODO/` | TRUE |
| `scripts/plant.sh:51` | `FILES` names the same path | `:51` reads `FILES="TODO/INDEX.md ... experiments/110-bloat-delta.sh ..."` | TRUE |
| `crates/podbox-image/src/store.rs:1083` | reads `if !sys::close_in_children(fd) {` | `:1083` reads `if !sys::close_in_children(fd) {` | TRUE |
| `crates/podbox-image/src/store.rs:1083` — enclosing `fn` (PLAN section 5) | `inside try_acquire` | the enclosing function is `Lock::try_acquire`, declared at `:1074`, inside `impl Lock` at `:1014` | TRUE as worded (PLAN says only `try_acquire`) |
| the same line in T-R000 `:85` and T-R005 `:80` | `inside Store::try_acquire` | the function is `Lock::try_acquire`. `impl Store` at `:215` has no `try_acquire`; `Store::hold` is at `:527` | **WRONG in both entries** — PLAN section 5 contradicts its own two entries |
| `experiments/157-lock-inheritance-prove.sh:157` | the sed names `lock.fd` | `:157` reads `'s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/'` | TRUE |
| the script "fails today" | the guard fires and exits 1 | `SUBJECT` is `store.rs` at `:39`; `sed` cannot match, the `git hash-object` compare at `:128` is equal, so `fail=1` at `:131` | TRUE |
| `experiments/386-podssh-partial.sh:74,75,77` | asserts three strings `machine.rs:53` no longer has | `:74,75,77` are three `check` lines asserting machine-verb strings; `machine.rs:53` reads `crate::parity::no_arm("machine", first)` | TRUE as a stale-clause claim; the exact line of the removed strings is not stated |
| `TODO/milestones.md:877` and not `:883` (VC-1) | `:877` carries the retired-number claim | `:877` reads `arguments. The audit found its unpublished driver reused number 386 and`; `:883` reads `writes more than one pipe buffer blocks` | TRUE |
| `TODO/interpose.md:1471` (VC-2) | names `162` in T-1311's live `Prove` | `:1471` reads ``Prove: `./experiments/162-tar-symlink-modes.sh` exits 0 on host`` | TRUE |
| `docs/conventions/code.md:29` (VC-2) | deployment proof rule | `:29` reads `Use integration and deployment proof for the actual default path.` | TRUE |
| `docs/conventions/code.md:33` (VC-3) | the rule that settles `95` | `:33` reads `A mock does not prove a live service or deployment.` | TRUE that the line exists and is a level rule; `:29` is the more direct citation |
| `experiments/95-podman-vfs-ignorechown.sh:17-19` | the script never exits 2 | `:17-19` read `Exit: 0 the combination opens the path, 1 it does not. Never 2: this script runs where the session's engine host is` | TRUE |
| `experiments/20-enter-target.sh:70` (VC-4) | names `10` in a live path | `:70` reads ``SKIP: $IMAGE not built. Run ./experiments/10-build-target-image.sh`` | TRUE |
| `experiments/300-run.sh:310` (VC-4) | names `10` in a live path | `:310` reads `say "        ./experiments/10-build-target-image.sh"` | TRUE |
| `TODO/gate.md:1125` (VC-4) | the three-way dependency is irreducible | `:1124-1125` read `Decision: The three convert as one unit. Converting 130 without 20 tests nothing` | TRUE |
| `crates/podbox-cli/src/parity.rs:232-537` | holds 265 rows | `:232` is `pub const TABLE: &[Row] = &[`; `:537` is `];`; 265 `Row {` lines inside | TRUE |
| `parity.rs:58` and `:65` | why a whole-file count returns 267 | `:58` reads `pub struct Row {`; `:65` reads `impl Row {` | TRUE |
| `TODO/cli.md:63` labelled CURRENT | 220 | `:63` reads `\| rows \| **220**. ⚠ 131 when this closed; ... this row is the CURRENT count` | TRUE that it is stale and falsely labelled |
| `experiments/results/parity-drive.txt:7` | 164 | `:7` reads `== 0. the table is data: 164 rows` | TRUE |
| `TODO/cli.md:685` | 160 | `:685` reads `rows, 199 driven, 0 mismatches, 0 unreachable here` | TRUE |
| `TODO/cli.md:182` | `clause-1 41/41` | `:182` reads `clause-1 41/41 refused with status None at 125, clause-2 10/10` | TRUE |
| `experiments/355-parity-curated.sh:106` | prints `$refused/40` | `:106` reads `echo "clause-1 refused-with-reason  $refused/40" >>"$REPORT"` | TRUE |
| `experiments/results/parity-curated.txt:8` | the saved result reads `40/40` | `:8` reads `clause-1 refused-with-reason  40/40` | TRUE |
| `TODO/cli.md:1246` | names nine ASCII tests where ten exist | `:1246` reads `Nine unit tests hold the constants the script drives` | TRUE |
| `TODO/gate.md:1430-1431` | `KVM guest boot 0.936 s against TCG 1.869 s` | `:1430-1431` read `the KVM base (experiments/results/perf-kvm.txt: KVM guest boot 0.936 s against TCG 1.869 s,` | TRUE |
| `experiments/results/perf-kvm.txt` reads `0.936` (T-R000 correction 4) | the corrected KVM figure | `perf-kvm.txt:133` reads `guest.kvm.boot kvm 0.935 s wall ok`; `:134` `ok: guest kvm boot 0.935s to READY`. The only `0.936` in the tree is `TODO/gate.md:1431` itself | **WRONG** — the correction says keep 0.936; the source reads 0.935 |
| `1.869` appears nowhere under `experiments/results/` | the TCG figure is unsourced | `grep -rn '1\.869' experiments/results/` returns nothing | TRUE |
| lane file records `guest.tcg.boot ... could-not-run` with `no qemu-system-x86_64 on PATH` | the TCG half never ran | `perf-lane.txt:123` reads `guest.tcg.boot tcg - s wall could-not-run` | TRUE |
| `TODO/probe.md:171` (T-R000 correction 5) | quotes `attribute.txt` as ENOSYS | `:171` reads `\| the host this repository is worked on \| errno=38 ENOSYS \| experiments/results/attribute.txt` | TRUE |
| `attribute.txt` reads ESRCH at line 7 | superseded file | `:6` reads `kcmp(-1,-1,...) [control] FAIL errno=3 ESRCH`. Line 7 is `fsopen(tmpfs) OK`. `grep -c ENOSYS` returns 0 | WRONG line number — the ESRCH row is line 6, not line 7 |
| commit `6eb941f` re-captured it | git provenance | `git show 6eb941f -- experiments/results/attribute.txt` shows `-kcmp(...) errno=38 ENOSYS` / `+kcmp(...) errno=3 ESRCH` | TRUE |
| `TODO/interpose.md:87` and `:1445-1446` carry stale `lib.rs` line numbers (T-R000 correction 6) | the correction is needed | `:87` reads `it**, in both directions and for both libcs: 17 declared, 17 exported` — the map declares 112. `:1445-1446` cite `lib.rs:334` and `lib.rs:989`; the `fchmodat` declaration is at `:356` and its `path_at_int!` at `:1038` | TRUE — the citations are stale |
| `TODO/image.md` T-0211 Done (T-R000 correction 7) | quotes a 2026-09-12 run of `157` | present in `TODO/image.md`; the re-anchor depends on correction 8 | TRUE |
| `TODO/gate.md:434` carries a stale `Source` (T-R000 correction 9) | `Cargo.toml:13-18` is stale | `:434` reads ``Source: `Cargo.toml:13-18`; `scripts/dev.sh:199-202```. `Cargo.toml:13` is `]`, and `:15-18` is the exclusion comment, not the member list. `dev.sh:199-202` is `build-state.py record`, and holds | TRUE that the `Cargo.toml` range is stale; `dev.sh:199-202` resolves but names nothing about interpose |
| `experiments/README.md:35` (T-R000 correction 10) | lists `392-kvm-guest.sh` as a passing proof | `:35` is one row of the `## Repository audit proofs` table | TRUE that the row exists; whether it "overstates" depends on the PROGRESS reading |
| `experiments/371-validationos-stream.sh:193` | writes `windows-365.txt` | `:193` reads `cp "$REPORT" "$REPO/experiments/results/windows-365.txt"` | TRUE |
| `experiments/370-windows-guest.sh:205` | writes `windows-364.txt` | `:205` reads `cp "$REPORT" "$REPO/experiments/results/windows-364.txt"` | TRUE |
| both result files exist with no citing entry | uncited readings | `ls` confirms both exist; `grep -rn 'windows-365\|windows-364' TODO/` returns nothing | TRUE |
| `TODO/gate.md` T-1212 `Prove` at `:964-968` (T-R000 correction 12) | requires six scripts each to exit 0 | `:964-968` name `150`, `270`, `280`, `300`, `320`, `330` and require `each exit 0` | TRUE |
| `:1037` records `EXIT:2` for `300-run.sh` | the entry contradicts itself | `:1026` reads `$ sh experiments/300-run.sh; echo EXIT:$?`; `:1037` reads `EXIT:2`; `:1038` starts the next script | TRUE |
| `:978-980` calls it a SKIP | the prose marks a skip | `:978-980` read `300 carries zero FAILs and one SKIP: clause 7 runs ... and the riscv64 refusal half is unmeasurable`. The SKIP line itself is at `:1032` | IMPRECISE — `:978-980` describes the skip, `:1032` records it |
| `TODO/image.md:1884,1894` record `24 of 24` (T-R000 correction 13) | the figure is stale | `:1884` reads `taken once by every test in it (24 of 24 by audit)`; `:1894` reads `24 vs 24, equal` | TRUE that both carry 24 |
| the current tree has 29 store tests | the corrected figure | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` returns **30** | **WRONG** — the tree has 30, not 29 |
| `TODO/image.md:507` and `experiments/lib/engine.sh:70,294-297,311` (T-R000 correction 14) | `ENG_NETWORK` present, no correction | `:70` `ENG_NETWORK="${ENG_NETWORK:-}"`; `:294-296` the case; `:297` the refusal; `:311` the reset | TRUE |
| `crates/podbox-cli/Cargo.toml` has no `[lib]` | wave 3's structural claim | the manifest has `build = "build.rs"` and one `[[bin]]`, and no `[lib]` | TRUE |
| `experiments/110,120,157,360,398,310,145,154,395,399` (section 3 subjects) | each subject exists | all ten resolve on disk | TRUE |

---

## Verify PLAN.md sections 1, 2, 3 and 7 — number table

| number | claim | my recount | my command | verdict |
| --- | --- | --- | --- | --- |
| 121 scripts | the corpus under `experiments/` and `scripts/` | 121 ledger rows; 105 top-level `experiments/*` + 16 top-level `scripts/*`. A naive `find` over both trees returns **167** because `lib/`, `common/`, `windows/`, `doctor/` and dot-directories hold 46 more | `find experiments -maxdepth 1 -type f \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' \) \| wc -l` = 105; same for `scripts` = 16 | TRUE for 121, but only under the unstated top-level rule. PLAN never states that rule |
| 29,153 lines | the corpus | 29,153 | `wc -l` over the 121 ledger paths | TRUE |
| KEEP-SHELL 29 | section 2 table | 29 after VC-2 and VC-3 | ledger + VC applied | TRUE |
| SPLIT 36 | section 2 table | 36 after VC-7 | ledger + VC applied | TRUE |
| RUST-TOOL 27 | section 2 table | 27 | ledger column 3 | TRUE |
| RUST-TEST 18 | section 2 table | 18 after VC-3 and VC-7 | ledger + VC applied | TRUE |
| DELETE 11 | section 2 table | 11 after VC-2 | ledger + VC applied | TRUE |
| total 121 | section 2 | 121 | sum | TRUE |
| section 2 table, pre-VC | — | KEEP-SHELL 27, RUST-TOOL 27, SPLIT 35, RUST-TEST 20, DELETE 12 = 121 | re-derived from the ten group bodies | TRUE |
| header labels total 112 (section 1) | the headers undercount by 9 | the ten header tables total **120**, not 112 | count-table regex per group; group 3 uses a per-script table | **WRONG** — 120, not 112 |
| six of ten headers disagree (sections 1 and 2's premise) | — | 6 of 10 (groups 1, 4, 5, 6, 7, 9) | per-group header counter vs per-section body verdict | TRUE |
| 43 converge wholly (section 2 total row) | — | undefined. RUST-TEST 18 + RUST-TOOL 27 = **45**. RUST-TEST 18 + SPLIT 36 = **54** (meta-1:410's own definition). PLAN's table columns give 45 | arithmetic on my recounted cells | **WRONG** — 43 matches neither reading, and contradicts PLAN's own table |
| 84 of 121 retired, 29 shell, 11 deleted (section 2) | — | the parts do not partition. SPLIT+RUST-TOOL+RUST-TEST = 81; with DELETE, 92. 29 + 84 + 11 = 124. 29 + 84 = 113 | arithmetic | **WRONG** — 84 double-counts the 11 DELETEs, or omits them, and the headline does not sum to 121 under any reading |
| 265 parity rows | section 7 and section 4 | 265 | `awk 'NR>=232 && NR<=537' parity.rs \| grep -c 'Row {'` = 265 | TRUE |
| 267 from a whole-file grep | why the wrong count arose | 267 | `grep -c 'Row {' parity.rs` = 267 | TRUE |
| `struct Row` at `:58`, `impl Row` at `:65` | the two extra hits | both confirmed | `grep -n 'struct Row'` / `grep -n 'impl Row'` | TRUE |
| 112 `interpose.map` names | section 3 | 112 | awk between `global:` (`:15`) and `local:` (`:128`) | TRUE |
| 13 `plant.sh` `Prove` lines across three `TODO/` files (VC-5) | — | **18** entry `Prove` blocks name `plant.sh`: 1 in `cli.md`, 1 in `deps.md`, 16 in `gate.md`. T-R004 repeats "13" | per-entry `Prove:` block regex over `TODO/*.md` | **WRONG** — 18, not 13 |
| 14 `Prove:` lines (round-2 VC-5 text) | — | same enumeration gives 18; 12 `Prove:` lines in `gate.md` carry `plant.sh` on the same line, 16 including continuations | two independent counts | **WRONG** — round-2's "fourteen" is also not reproducible |
| 980 test functions | section 1 | 980 | `grep -rn --include='*.rs' -c '#\[test\]' crates/ \| awk -F: '{s+=$2}'` | TRUE |
| 950 `#[test]` under `src/` | the 950/30 split | 950 | same over `crates/*/src crates/*/build.rs` | TRUE |
| 30 in `tests/` | the remainder | 30 | 980 − 950 | TRUE |
| 0 async | — | 0 | `grep -rn '#\[tokio::test\]' crates/ \| wc -l` | TRUE |
| 15 unwritten clauses | section 4 | 15 | the T-R002 numbered table has 15 rows | TRUE (see self-sufficiency: the rows name 9 distinct scripts, not 8) |
| 11 `mod tests` | meta-1's own per-crate count | `meta-1.md:83` reads `137` in that column | — | not verified: the figure is meta-1's own and its units are not stated |
| 8 `#[cfg(test)]` modules, 1,760 citations, 5 wrong, 16 corrections | section 1 | 1,760 / 5 / 16 all confirmed by `round-1.md` | — | TRUE |
| the KEEP-SHELL breakdown 10/3/4/3/3/2/1/1/1 | section 2's "round 2 re-examined all 29" | the items sum to **30**, and round-2 applies them to **27**. I could not reproduce the partition: the categories overlap, and 11 of the 29 scripts match "QEMU or an emulator" by keyword, 3 name `wsl-toolkit`, 3 name `zot`/`openssl`/`htpasswd` | awk keyword sweep over the 29 | **WRONG as applied to 29.** It is round-2's 27-figure with two scripts added and no re-derivation. The correct summary count is 29; the breakdown does not support it |
| 10 → 14 members | section 3 | 9 members today + 4 new = 13, not 14 | `Cargo.toml:3-13` lists 9 | **WRONG** — 9 members become 13. `meta-review-1.md:629` says "fourteen" and carries the same error |
| 24 → 30 store tests | T-R000 correction 13 | 30 | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` | **WRONG** — the correction names 29; the tree has 30 |
| 17 declared interpose names (`interpose.md:87`) | the value T-R000 correction 6 targets | the map declares 112 | awk over the `global:` block | TRUE that 17 is wrong |

---

## Verify PLAN.md sections 1, 2, 3 and 7 — corrections check

Each item: is the record actually wrong, and is the corrected value right.

1. **`TODO/cli.md:63` parity rows 220 → 265.** The record is wrong
   (220, labelled CURRENT). The correction is right: 265, counted inside
   `:232-537`. The instruction to mark `parity-drive.txt:7` and
   `cli.md:685` historical rather than delete them is sound and matches
   `docs/methodology/history.md`. **SOUND.**
2. **`TODO/cli.md:182` `clause-1 41/41` → 40/40.** The record is wrong.
   `355-parity-curated.sh:106` prints `/40` and `parity-curated.txt:8`
   records `40/40`. **SOUND.** The reconciliation (46 curated flags, 40
   refused, 6 driven) is group-1's arithmetic, restated; I did not
   re-derive the 46.
3. **`TODO/cli.md:1246` nine ASCII tests → ten.** The record is wrong.
   I found 10 `*ascii*` test functions under `crates/podbox-cli/src/`.
   **SOUND.**
4. **`TODO/gate.md:1430-1431` drop the TCG figure.** The `1.869` figure
   has no source under `experiments/results/`, and `perf-lane.txt:123`
   records `guest.tcg.boot ... could-not-run`. Removing it is right.
   **PARTLY DEFECTIVE.** The correction says "`experiments/results/perf-kvm.txt`
   reads `0.936`; keep the KVM one with its conditions." It reads
   **`0.935`** (`:133`, `:134`). `0.936` appears nowhere but in the record
   being corrected. Following this correction reproduces the error it
   claims to remove.
5. **`TODO/probe.md:171` ENOSYS → the re-captured reading.** The record
   is wrong: `attribute.txt` carries no ENOSYS, and commit `6eb941f`
   changed that exact row from ENOSYS to ESRCH. Naming the commit is
   right. **DEFECTIVE in detail.** "The file reads ESRCH at line 7" is
   wrong: the ESRCH row is `attribute.txt:6`; line 7 is `fsopen(tmpfs) OK`.
6. **`TODO/interpose.md:87` and `:1445-1446` stale `lib.rs` lines.**
   Both records are wrong: `:87` claims "17 declared, 17 exported" against
   a map with 112 names, and `:1445-1446` cite `lib.rs:334` and `:989`
   where the `fchmodat` declaration is at `:356` and its wrapper at
   `:1038`. The correction is right that both are stale but names no
   replacement value for either, so an implementor still has to derive
   them. **SOUND but incomplete.**
7. **`TODO/image.md` T-0211 Done quotes a 2026-09-12 `157` run.**
   The record is a closed entry quoting a run whose mutation anchor no
   longer matches. **SOUND**, and correctly deferred to correction 8.
8. **`157`'s sed anchor.** The record is wrong: `157:157` matches
   `lock\.fd` and `store.rs:1083` reads `fd`, and `SUBJECT` is `store.rs`
   (`:39`), so the mutation cannot land and `fail=1` follows. Re-anchor is
   right. **PARTLY DEFECTIVE.** It says the call is "inside
   `Store::try_acquire`, not `Store::hold`". The enclosing function is
   `Lock::try_acquire` (`:1074`, in `impl Lock` at `:1014`). There is no
   `Store::try_acquire`. PLAN section 5 states it correctly; these two
   documents do not.
9. **`TODO/gate.md:434` stale `Source`.** The `Cargo.toml:13-18` range is
   stale — it opens on `]` and covers only the exclusion comment. **SOUND.**
   The correction names no replacement range. The second citation,
   `scripts/dev.sh:199-202`, resolves to `build-state.py record` and has no
   bearing on T-1207's subject, so the `Source` line has a second defect
   the correction does not mention.
10. **`experiments/README.md:35` overstates `392-kvm-guest.sh`.** The
    row exists and is in the audit-proofs table. **NOT SETTLED by me.** The
    correction's claim rests on `TODO/PROGRESS.md` recording the 2026-09-30
    KVM runs as failed; I did not open PROGRESS.md's KVM record, so I report
    this as plausible, not confirmed.
11. **`371:193` writes `windows-365.txt`; `370:205` writes
    `windows-364.txt`.** Both records are wrong as to ownership, both files
    exist, and no `TODO/` entry cites either name. **SOUND.**
12. **T-1212's `Prove` contradicts its own transcript.** The `Prove` at
    `:964-968` requires six scripts to exit 0; the transcript at `:1026`
    invokes `300-run.sh` and `:1037` records `EXIT:2`. **SOUND**, with one
    imprecision: the correction cites `:978-980` as where the prose "calls
    it a SKIP". Lines `:978-980` do describe the skip in the Done record;
    the SKIP is recorded in the transcript at `:1032`. Both readings are
    supportable, but the citation is loose.
13. **`TODO/image.md:1884,1894` "24 of 24" store tests.** The record is
    stale. **DEFECTIVE.** The corrected figure is given as 29. The tree
    holds **30** `#[test]` functions in `store.rs`, all inside its single
    `mod tests` (which begins at `:1245`). An implementor writing "29" into
    the record writes a wrong number.
14. **`TODO/image.md:507` `ENG_NETWORK`.** All four cited lines exist and
    the knob is present. Recording "no correction" is right. **SOUND.**
    This item is correctly a non-action.

---

## Verify PLAN.md sections 1, 2, 3 and 7 — self-sufficiency gaps

Ordered by how badly an implementor with no prior context would be stopped.

1. **VC-6 is absent from PLAN section 7.** Section 7 is titled "Standing
   corrections" and the body lists VC-1, VC-2, VC-3, VC-4, VC-5 and VC-7.
   `round-2.md:486` defines VC-6 as `95` → deployment proof. An implementor
   reading only PLAN section 7 cannot know it exists. Worse, section 2's
   KEEP-SHELL paragraph attributes the 29-verdict re-examination to "Round 2"
   without noting that round 2 examined 27 and that two scripts arrived
   afterwards.
2. **The verdict arithmetic in section 2 does not close.** An implementor
   implementing "84 of 121 retired, 29 stay in shell, 11 are deleted"
   cannot derive 84 from the table, and 29 + 84 + 11 = 124. The entry never
   says whether "retired" includes the DELETEs. They must guess, and the
   table's own total row says a third thing (43).
3. **The 121-script boundary is never stated.** The plan says 121 scripts
   "under `experiments/` and `scripts/`". A reader who counts that way gets
   167. The unstated rule — top-level files only, excluding `lib/`,
   `common/`, `windows/`, `doctor/` and dot-directories — is load-bearing,
   because `scripts/common/` holds 28 scripts the gate depends on. An
   implementor retiring "121 scripts" could delete a `scripts/common/`
   check the gate runs, or leave the corpus half retired.
4. **The ledger is pre-VC and says nothing about it.** PLAN section 1
   presents `verdict-ledger.tsv` as the machine-readable result, "after
   VC-1 through VC-7". It is not. Reading column 3 gives KEEP-SHELL 27,
   SPLIT 35, RUST-TEST 20, DELETE 12 — and `95` and `151` still carry the
   labels VC-3 and VC-7 reverse. An implementor taking the ledger as
   authoritative assigns `95` a Rust test that cannot exist and labels
   `151` a single RUST-TEST where the plan requires a SPLIT.
5. **No entry states which crate a wave-1/2 script's *tests* go in.** Wave 1
   deletes three scripts and repoints two `TODO/podssh.md` lines; wave 2
   writes fifteen tests. Neither settles the `[lib]` question that section 5
   itself identifies as the reason wave 3 exists. `podbox-cli` has no
   `[lib]` today, so any `crates/podbox-cli/tests/*.rs` is impossible until
   that is decided. T-R002's table does give a `target mod tests` per
   clause, but three of its fifteen rows target `crates/podbox-interpose/`,
   which section 3 says cannot host `tests/*.rs` at all.
6. **The KEEP-SHELL breakdown is unusable as a work list.** Section 2 gives
   ten categories summing to 30 for 29 scripts. An implementor asked to
   "re-examine the 29 against the narrow test" has no per-script mapping and
   must re-derive the whole partition from the ten group reports.
7. **The `rlib` decision is asserted by a build nobody can repeat.**
   Section 3 says the meta review "verified this by building the minimal
   case". The build is not in the tree and cannot run here (`linker 'cc' not
   found`). An implementor inherits a conclusion whose only evidence is a
   transcript in `meta-review-1.md:633-641`.
8. **Section 3 names ten members becoming fourteen.** It is nine becoming
   thirteen. An implementor counting `Cargo.toml` and the table finds a gap
   and does not know which is authoritative.
9. **The "13 `Prove` lines" that must repoint is 18.** Both PLAN section 7
   and T-R004 state 13. An implementor grepping `plant.sh` in `TODO/` finds
   18 entry Prove blocks and cannot tell whether 5 were missed or 5 do not
   need repointing.
10. **`build-interpose.sh:197-214` is cited for a check the plan says needs
    no new test, but the plan never says who re-points the `gate.yml` step.**
    Section 3 mentions the `gate.yml:99` build; T-R004's table covers the
    workflow steps. An implementor reading section 3 alone does not know the
    step moves.

---

## Verify PLAN.md sections 1, 2, 3 and 7 — internal consistency

- **Entries contradict each other on the wave-0 subject.** PLAN section 5
  and T-R000 agree the call at `store.rs:1083` is inside `try_acquire`;
  T-R000:85 and T-R005:80 both say `Store::try_acquire`. There is no such
  function. Two of the three documents are wrong and the third is right.
- **`157` is named in wave 0 and wave 5.** Not a conflict: T-R005:78-85
  explicitly defers the re-anchor to wave 0 and gates wave 5 on it. But
  nothing in PLAN section 5's wave table records that ordering dependency,
  so an implementor reading only PLAN sees `157` in two waves with no note.
- **`110` is named in wave 4 and wave 5.** T-R005:90-92 states the move
  lands in wave 4 and that wave 5 cannot proceed without it. PLAN section 3
  lists `110` as a `podbox-release` subject while section 5's wave-4 row
  says wave 4 is `crates/podbox-gate`. An implementor must read T-R005 to
  learn that wave 5 performs the `podbox-size` move and wave 4 only clears
  the path. That ordering is stated nowhere in PLAN.
- **`plant.sh` appears in T-R002, T-R004 and T-R005.** T-R002 adds fifteen
  plant cases to `plant.sh`; T-R004 writes `plant.sh` as a `podbox-gate`
  binary. No entry states which lands first or whether the fifteen cases
  must be re-authored after the rewrite.
- **`scripts/dev.sh` appears in T-R004 and T-R005.** Same shape. T-R005
  lists it in a table without a fence. An implementor cannot tell if
  T-R005 claims it.
- **Crates agree across entries**: `podbox-gate`, `podbox-buildstate`,
  `podbox-release`, `podbox-podvm` are named identically in PLAN section 3,
  T-R004 and T-R005. The binary names also agree with `meta-review-1.md`.
  No conflict found here.
- **Member count disagrees with the tree.** `Cargo.toml:3-13` has 9 members.
  PLAN section 3 and `meta-review-1.md:629` both say 14 after adding 4.
  The tree supports 13.
- **Prove commands**: `py scripts/check-todo.py` (T-R000, T-R004) runs
  green on this host — exit 0, `check-todo: ok`. `cargo test --workspace`
  (T-R001, T-R002, T-R005) and `cargo test -p podbox-image` (T-R003) cannot
  run here; that is the documented host blocker, not a defect.
- **Ledger wave coverage**: every ledger row resolves to a group body
  verdict, and the 121 scripts partition cleanly across the ten groups.
  No ledger script is unassigned, because the ledger records verdicts
  rather than waves. No wave in PLAN section 5 enumerates its scripts, so
  "is any script in the ledger assigned to no wave" cannot be answered from
  these documents at all.

---

## Verify PLAN.md sections 1, 2, 3 and 7 — what each Prove establishes

| prove command | what it proves | what it does not |
| --- | --- | --- |
| `py scripts/check-todo.py` (T-R000) | the 203 `TODO/` rows agree with the 203 entries, the counts add up, every reference resolves, and every cited path and line exists. I ran it: exit 0, `check-todo: ok` | that any measurement is correct, that any stale value has been corrected, or that `157` re-anchors. It is a consistency gate over records only, exactly as T-R000's own Counts paragraph says |
| `py scripts/check-todo.py` (T-R004) | the same, plus that no cited path was left dangling by the move | that the six `podbox-gate` binaries behave. A `podbox-gate` binary that returns success without checking anything passes this |
| `cargo test --workspace` (T-R001) | the twelve and six named tests still pass after the three scripts are deleted | that the deleted scripts' remaining shell behaviour is gone. A test can pass without reaching the deleted clause. It also cannot run here |
| `cargo test --workspace` (T-R002) | the fifteen new tests pass and nothing broke | that any of the fifteen can fail. T-R002 says this itself: "It does not prove the new tests can fail; that is what the plants do" |
| `cargo test --workspace` (T-R005) | the three new crates compile and their tests pass | that a `podbox-buildstate` binary reproduces `scripts/build-state.py`'s recorded inputs. No golden file is named for any of the three |
| `cargo test -p podbox-image` (T-R003) | the new `tests/` files and the registry fixture pass against the image crate | that the fixture is reachable from a binary, or that a registry-backed test reaches a real registry. `registry.rs:955 serve_once` is test-private, so an integration test in `tests/` cannot call it as the plan's own section 6 states |
| section 2's `py scripts/check-todo.py exits 0` (wave 0 row) | same as T-R000 | the same. Wave 0 changes no source, so this prove is insensitive to whether wave 0 did its work |
| "all four gate jobs" (waves 4 and 5) | the four jobs in `gate.yml` are green, which includes the plant step at `:56` | that the new crates are covered by a plant. The gate's plant step exercises `scripts/plant.sh`, which waves 4 and 5 rewrite |
| "the four jobs are green" as a wave-5 proof | the whole workspace is green including the new crates | that the six `podbox-gate` binaries do the work `check-todo.py` did. The gate at `:37` still runs `./scripts/check-todo.py` until that step is repointed |

### Values that are really estimates or really unknown

| value | status | what would settle it |
| --- | --- | --- |
| 43 converging scripts | unknown. Neither reading of "converge" produces it | a definition of converge, then a recount over the ledger |
| 84 retired | unknown. The parts do not partition | a statement of whether DELETEs are inside "retired" |
| the KEEP-SHELL breakdown | unknown as a partition; it sums to 30 for 29 scripts | a per-script category column in the ledger |
| header total 112 | wrong; the headers total 120 | nothing; recount it as 120 |
| "ten members become fourteen" | wrong; nine become thirteen | count `Cargo.toml:3-13` |
| `experiments/README.md:35` overstates its proof | plausible, unverified by me | read `TODO/PROGRESS.md`'s KVM record for 2026-09-30 |
| `1.869` is unsourced | confirmed unsourced; its true origin is unknown | a result file outside `experiments/results/`, or a re-measurement |
| the `0.936` KVM figure | wrong; `perf-kvm.txt` reads `0.935` | nothing; correct it to 0.935 |
| the `29` store tests | wrong; the tree holds 30 | nothing; correct it to 30 |

---

## Verify PLAN.md sections 1, 2, 3 and 7 — corrections required

1. **PLAN section 7 must carry VC-6.** `round-2.md:486` defines it as the
   level statement for `95`. Without it the standing list is VC-1..VC-5 and
   VC-7 with a hole where the audit recorded a decision.
2. **Fix the "43 converge" total row.** The table's own columns give 45
   (RUST-TOOL 27 + RUST-TEST 18). `meta-1.md:410` defines it as 54
   (RUST-TEST 18 + SPLIT 36). Pick one and say which, then recompute.
3. **Fix "84 of 121 retired".** The parts sum to 124, not 121. State
   whether DELETEs are inside or outside the 84. 81 + 11 = 92 retired, with
   29 kept, is the arithmetic that closes.
4. **Fix "header labels total 112".** The ten group header tables total 120.
   The point survives — a sum check still misses the six disagreeing
   headers — but the number is wrong and an implementor may re-derive 112
   and doubt the finding.
5. **Fix "ten members become fourteen".** `Cargo.toml` has 9 members; four
   new makes 13. Fix it in PLAN section 3 and in `meta-review-1.md:629`, or
   state what the tenth member is.
6. **Correct the KEEP-SHELL paragraph in section 2.** It attributes a
   ten-category re-examination of **29** scripts to round 2, which examined
   **27**. The two added by VC-2 and VC-3 have no category. The items as
   printed also sum to 30, not 27 or 29.
7. **Fix the `13 Prove` lines.** PLAN section 7 and T-R004 both say 13. I
   count 18 entry `Prove` blocks naming `plant.sh` (1 `cli.md`, 1
   `deps.md`, 16 `gate.md`). Each must repoint or be listed as exempt.
8. **Fix `Store::try_acquire` in T-R000:85 and T-R005:80.** The function is
   `Lock::try_acquire`, declared at `store.rs:1074` inside `impl Lock`
   (`:1014`). PLAN section 5 states it correctly and the two entries
   contradict it.
9. **Fix the corrected KVM figure in T-R000 correction 4.** It says
   `perf-kvm.txt` reads `0.936` and that 0.936 should be kept. The file
   reads `0.935` at `:133` and `:134`. Keeping 0.936 preserves the error.
10. **Fix the corrected store-test figure in T-R000 correction 13.** It
    says 29. `crates/podbox-image/src/store.rs` holds 30 `#[test]`
    functions.
11. **Fix the ESRCH line number in T-R000 correction 5.** It says line 7.
    `attribute.txt:6` carries the ESRCH row; `:7` is `fsopen(tmpfs) OK`.
12. **Replace the ledger with the post-VC verdicts, or label it pre-VC.**
    `verdict-ledger.tsv` still carries `95` as RUST-TEST and `151` as
    RUST-TEST, both of which the standing corrections reverse, and its
    `162` DELETE that VC-2 refutes. PLAN section 1 says the ledger is the
    machine-readable result after VC-1 through VC-7. It is not. An
    implementor reading only the ledger builds the wrong plan for three
    scripts.
13. **State the 121-script boundary in the plan itself.** The figure holds
    only for top-level `experiments/` and `scripts/` files. `scripts/common/`
    holds 28 more the gate runs. Without the rule, "retire 121 scripts"
    is not executable.
14. **Give corrections 6 and 9 their replacement values.** Correction 6
    says `interpose.md:87` and `:1445-1446` carry stale `lib.rs` lines but
    names no replacement (`interpose.map` declares 112, not 17;
    `fchmodat` is at `lib.rs:356` and `:1038`). Correction 9 says
    `gate.md:434`'s `Cargo.toml:13-18` is stale but names no replacement
    range, and does not mention that its second citation,
    `scripts/dev.sh:199-202`, resolves to unrelated code.
15. **Reconcile the `157` and `110` wave ordering into PLAN section 5.**
    Both are named in two waves, and only the entry files explain which
    wave performs the move and which merely clears the path.
16. **Reconcile `plant.sh` and `dev.sh` across T-R002, T-R004 and T-R005.**
    `plant.sh` is rewritten as a `podbox-gate` binary in T-R004 while
    T-R002 adds fifteen plant cases to it. No entry says which lands first
    or whether the fifteen cases survive the rewrite.
17. **Say what `cargo metadata` reports today, and name the wave-3
    decision.** Section 5 asserts `['bin','custom-build']` with no `lib` for
    `podbox-cli`. `crates/podbox-cli/Cargo.toml` confirms it. T-R002
    nonetheless targets `crates/podbox-cli/src/*` inline modules, which is
    safe, and T-R003 targets `crates/podbox-image/tests/`, which is not
    `podbox-cli`. State whether any wave-2 target lands in a crate with no
    `[lib]`.
