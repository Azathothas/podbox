# Verify T-R000.md and T-R001.md — the record fixes and the deletions

## Scope and method

Assigned documents: `refactor/06-entries/T-R000.md` (118 lines) and
`refactor/06-entries/T-R001.md` (111 lines). `refactor/06-entries/PLAN.md` was
read as the cross-document reference. Six files were restored after two
deliberate deletion experiments; `git status --porcelain` shows only the
untracked `refactor/` tree, and `py scripts/check-todo.py` exits 0 on the
restored tree.

Read in full: `AGENTS.md`, `TODO/RULES.md`, `TODO/INDEX.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `docs/methodology/gate.md`, `experiments/README.md`,
`.github/workflows/gate.yml`, `Cargo.toml`,
`crates/podbox-interpose/Cargo.toml`.

Also read: all ten `refactor/01-audit/group-*.md` headers and bodies,
`refactor/00-orientation/assignment-map.md`, `refactor/04-meta/meta-1.md`,
`refactor/02-peer-review-1/round-1.md`, `verdict-ledger.tsv`,
`scripts/check-todo.py` (checks 7, 11 to 14, 18), `scripts/plant.sh`,
`scripts/build-interpose.sh`, `scripts/dev.sh`, `experiments/lib/engine.sh`,
and the cited regions of fourteen experiment scripts and eleven Rust or
Python files.

Counts: 61 `path:line` citations from the two assigned documents opened
individually. 26 numbers re-counted with a named command. 14 T-R000
corrections checked against the record each names. 121 ledger rows checked
against the group report bodies rather than against the ledger.

What I could not settle:

- Whether `crates/podbox-cli/src/parity.rs` should hold 265 or 267 rows. 265 is
  what the table holds; 267 is what a whole-file text search returns. The
  correction is right; see the number table.
- The count `13` for `Prove` lines naming `scripts/plant.sh`. It is 16 in the
  current tree. I did not build, so I cannot say whether the tree grew by three
  after PLAN.md was written or whether the original count was wrong.
- Any build-dependent claim. `cargo` stops at `linker 'cc' not found` here, as
  the entry says. Static reading only.
- `TODO/cli.md:685`'s 160 figure. The record's own arithmetic is
  153 + 5 + 2, which sums to 160; whether the 153 was ever correct against the
  current table is not derivable from the text.

---

## Citation table

| citation | the document claims it says | the line actually says | verdict |
| --- | --- | --- | --- |
| `TODO/RULES.md#5-entry-closure` | entry closure section | `TODO/RULES.md:45` is `## 5. Entry closure` | TRUE |
| `PLAN.md` section 4 | the work already done in Rust table | `PLAN.md:83` is `## 4. Work already done in Rust` | TRUE |
| `refactor/03-peer-review-2/round-2.md` | a round-2 record | file exists, 3,000+ lines | TRUE |
| `refactor/05-meta-review/meta-review-1.md` | the meta review | file exists | TRUE |
| `TODO/cli.md:63` | parity table has 220 rows, labelled CURRENT | `| rows | **220**. ⚠ 131 when this closed; ... this row is the CURRENT count so the two cannot drift apart |` | TRUE that the record is stale |
| `crates/podbox-cli/src/parity.rs:232-537` | the table holds 265 | `:232` is `pub const TABLE: &[Row] = &[`; `:537` is `];` | TRUE |
| `experiments/results/parity-drive.txt:7` | says 164 | `== 0. the table is data: 164 rows` | TRUE |
| `TODO/cli.md:685` | says 160 | `160\nrows, 199 driven, 0 mismatches` | TRUE |
| `TODO/cli.md:182` | records `clause-1 41/41` | `clause-1 41/41 refused with status None at 125` | TRUE |
| `experiments/355-parity-curated.sh:106` | prints `$refused/40` | `echo "clause-1 refused-with-reason  $refused/40"` | TRUE |
| `TODO/cli.md:1246` | names nine ASCII tests | `Nine unit tests hold the constants the script drives` | TRUE |
| `TODO/gate.md:1430-1431` | states "KVM guest boot 0.936 s against TCG 1.869 s" | `:1430` `the KVM base (...perf-kvm.txt...: KVM guest`; `:1431` `boot 0.936 s against TCG 1.869 s, forced rungs refuse where` | TRUE |
| `experiments/results/perf-kvm.txt` | reads 0.936 | reads **0.935** at `:133` and `:134` | FALSE. The file never holds 0.936 |
| `experiments/lib/engine.sh:70,294-297,311` | `ENG_NETWORK` present at those lines | `:70` `ENG_NETWORK="${ENG_NETWORK:-}"`; `:294` `case "${ENG_NETWORK:-}" in`; `:295-296` the `none` and `*` arms; `:311` `ENG_NETWORK=""` | TRUE |
| `TODO/gate.md:434` | carries a stale `Source` line | `:434` `Source:      `Cargo.toml:13-18`; `scripts/dev.sh:199-202`` | TRUE that a stale line exists |
| `experiments/README.md:35` | lists `392-kvm-guest.sh` in a table of passing proofs | `\| 392-kvm-guest.sh \| Windows toolkit base, explicit Linux binary, ... \|` | TRUE |
| `TODO/PROGRESS.md` | records the KVM runs of 2026-09-30 as failed | `:63` `The KVM repetitions of 2026-09-30 failed at command shutdown and setup.` | TRUE |
| `experiments/371-validationos-stream.sh:193` | writes `experiments/results/windows-365.txt` | `cp "$REPORT" "$REPO/experiments/results/windows-365.txt"` | TRUE |
| `experiments/370-windows-guest.sh:205` | writes `experiments/results/windows-364.txt` | `cp "$REPORT" "$REPO/experiments/results/windows-364.txt"` | TRUE |
| `TODO/gate.md:964-968` | T-1212's `Prove` requires six scripts to each exit 0 | `:964` `Prove:` through `:968` `driver, and experiments/results/ carries the runs.` | TRUE |
| `TODO/gate.md:1037` | records `EXIT:2` for `experiments/300-run.sh` | `:1037` is the only `EXIT:2` in gate.md; `:1038` names the next script, so `:1037` closes 300's transcript | TRUE |
| `TODO/gate.md:978-980` | prose calls it a SKIP | `:978-980` `` `300` carries zero FAILs and one SKIP: clause 7 runs ... the riscv64 refusal half is unmeasurable`` | TRUE |
| `TODO/image.md:1884,1894` | record "24 of 24" | `:1884` `taken once by every test in it (24 of 24 by audit)`; `:1894` `\| 24 vs 24, equal \|` | TRUE |
| `TODO/image.md:507` | records an `ENG_NETWORK` knob | `:507` `opt-in ENG_NETWORK knob in experiments/lib/engine.sh` | TRUE |
| `TODO/probe.md:171` | quotes attribute.txt as ENOSYS | `:171` `\| the host this repository is worked on \| errno=38 ENOSYS \| experiments/results/attribute.txt ...` | TRUE |
| `experiments/results/attribute.txt:7` | reads ESRCH at line 7 | `:7` `kcmp(-1,-1,...) [control]          FAIL errno=3 ESRCH` | TRUE |
| commit `6eb941f` | re-captured the file | `git show --follow 6eb941f` shows `-ENOSYS` → `+ESRCH` on that line | TRUE |
| `TODO/interpose.md:87` | carries a stale `lib.rs` line number | `:87` reads `17 declared, 17 exported, and 0` — no `lib.rs` line at all | TRUE that it is stale, but the entry never says **how** |
| `TODO/interpose.md:1445-1446` | carry stale `lib.rs` line numbers | `:1445` `crates/podbox-interpose/src/lib.rs:334`; `:1446` `crates/podbox-interpose/src/lib.rs:989` | TRUE that both are wrong |
| `crates/podbox-interpose/src/lib.rs:334` | the `fchmodat` declaration | `:334` is `next_renamexattr`. `fchmodat` is declared at **`:356`** | FALSE as a claim that `:334` is right; TRUE only as a claim that it is stale |
| `crates/podbox-interpose/src/lib.rs:989` | the `fchmodat` wrapper | `:989` is `path_int!(remove, next_remove, ...)`. The wrapper is at **`:1038`** | FALSE as a claim that `:989` is right |
| `TODO/image.md` T-0211 Done | quotes a 2026-09-12 run of `157` | `:1162` `**Done, 2026-09-12.** ./experiments/157-lock-inheritance-prove.sh with PODBOX_PROVE_RUNS=30` | TRUE |
| `experiments/157-lock-inheritance-prove.sh:157` | matches `if !sys::close_in_children(lock.fd) {` | `:157` is `'s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/'` | TRUE |
| `crates/podbox-image/src/store.rs:1083` | reads `if !sys::close_in_children(fd) {` inside `Store::try_acquire`, not `Store::hold` | `:1083` is exactly `                if !sys::close_in_children(fd) {`; `try_acquire` starts at `:1074`, `hold` at `:527` | TRUE |
| `crates/podbox-ssh/tests/session_interactive.rs:255-482` | 12 tests | 12 `#[test]` at 254, 275, 287, 303, 322, 350, 373, 388, 409, 430, 449, 471. File is 484 lines. The range starts one line late and ends two short of the last test body | TRUE for the count; the range is off by one at the start |
| "the script is cited in their doc comments" | the tests name `388-interactive-shell.sh` | `grep -n 388 crates/podbox-ssh/tests/session_interactive.rs` returns nothing. The only citation of that script in the tree is `TODO/podssh.md:79` | FALSE |
| `crates/podbox-extract/src/lib.rs:40` | `#[cfg(test)] mod drive;` | `:39` is `#[cfg(test)]`, `:40` is `mod drive;`. The pair is at 39-40; `:40` alone is half the attribute | TRUE if read as the `mod drive;` line |
| `crates/podbox-extract/src/drive.rs:167` | path escape | `fn an_entry_traversing_a_symlink_the_same_layer_made_is_refused()` | TRUE |
| `crates/podbox-extract/src/drive.rs:211` | symlink target | `fn a_dotdot_path_is_refused()` — a `..` member, not a symlink target | MISLABELLED. `:192` is the relative-escaping-symlink test |
| `crates/podbox-extract/src/drive.rs:221` | a filesystem check | `assert!(` — the continuation of test `:167` | TRUE |
| `crates/podbox-extract/src/drive.rs:229` | tar member mode | `fn an_absolute_path_is_refused()` — an absolute member name, not a mode | MISLABELLED |
| `crates/podbox-extract/src/drive.rs:236` | a filesystem check | `assert!(!std::path::Path::new("/etc/podbox-should-never-write-this").exists());` | TRUE |
| `crates/podbox-extract/src/drive.rs:253` | a filesystem check | `assert!(!s.path().join("after").exists());` | TRUE |
| `crates/podbox-extract/src/drive.rs:264` | device node | `fn an_absolute_symlink_is_created_and_points_where_the_image_said()` — an absolute symlink, not a device node | MISLABELLED |
| `crates/podbox-extract/src/drive.rs:382` | ownership | `fn a_hard_link_out_of_the_destination_is_refused()` — a hard link, not ownership. Ownership is `:480` | MISLABELLED |
| `crates/podbox-extract/src/drive.rs:480` | a test asserting the OPPOSITE outcome to `70` check B | `fn the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply()`; `:486-487` `assert_ne!(md.gid(), 42, ...)` | TRUE |
| `crates/podbox-cli/src/system.rs:603` | holds 7 tests and none names `abi` | `:603` is `mod tests {`. `grep -c '#\[test\]' system.rs` is 7, and none is named `abi` | TRUE |
| `crates/podbox-cli/src/images.rs:961-986` | `prune` returns 0 | `:986` is `            0` inside `Ok(done)` of `store.delete(&rest, Held::Skip)` | TRUE |
| `scripts/check-todo.py:300` | `check_tree` reads every tracked file | `:300` is `def check_tree(files):` | TRUE |
| `scripts/check-todo.py:228` | `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` | `:228` is exactly that | TRUE |
| `scripts/plant.sh:51` | `FILES` | `:51` is `FILES="TODO/INDEX.md TODO/PROGRESS.md ... scripts/dev.sh"` | TRUE |

Citations checked: 61. Wrong or mislabelled: 6. Wrong in the entry's own text:
3 (`0.936`, "cited in their doc comments", `parity.rs:232-537` is fine but
`lib.rs:334`/`:989` are presented as the values being corrected without the
implementor being told what the corrected values are).

---

## Number table

| number | where | my recount | my command | verdict |
| --- | --- | --- | --- | --- |
| 121 scripts | PLAN §1, ledger | 121, all present in the tree, all with a matching group | `git ls-files experiments scripts \| grep -E '\.(sh\|py\|ps1)$'` against `verdict-ledger.tsv` column 1 | TRUE |
| 29,153 lines | PLAN §1, orientation | 29,153, and every one of the 121 files still matches its map figure line for line | `py` over `refactor/00-orientation/assignment-map.md`, summing each file's current `wc -l` | TRUE |
| total tracked scripts | not stated in the entries | **160** scripts / **38,802** lines, 39 not in the ledger (`experiments/lib/*`, `scripts/common/*`, `scripts/windows/*`, `scripts/doctor/*`, `scripts/dev.sh`, `scripts/plant.sh`, …) | `git ls-files experiments scripts \| grep -E '\.(sh\|py\|ps1)$' \| wc -l` | the plan scopes 121, which is defensible, but an implementor must be told the other 39 are out of scope |
| KEEP-SHELL 29 | PLAN §2 | **27** | `py` counting `verdict-ledger.tsv` column 3 | FALSE |
| SPLIT 36 | PLAN §2 | **35** | same | FALSE |
| RUST-TOOL 27 | PLAN §2 | **27** | same | TRUE |
| RUST-TEST 18 | PLAN §2 | **20** | same | FALSE |
| DELETE 11 | PLAN §2 | **12** | same | FALSE |
| total 121 | PLAN §2 | 121 | same | TRUE |
| "43 converge wholly on Rust" | PLAN §2 | 27 + 18 = 45 | `py` over the ledger | FALSE. The PLAN's own table gives 45; 43 matches no combination of its own five rows |
| "84 of 121 can be retired" | PLAN §2 | 27 + 20 + 35 + 12 = 94 by the ledger; 27 + 18 + 36 + 11 = 92 by the PLAN's own table. Neither is 84 | `py` | FALSE |
| six of ten headers disagree with their bodies | T-R000 Decision | **four** disagree. Groups 1, 2, 5, 8, 10 headers agree with their bodies. 4 and 7 disagree internally between their summary row and their per-script table, but group 3's per-script table agrees with its own bodies | `py` per group: header counts versus `**Verdict:` / `### Verdict:` body headings | FALSE |
| "header labels total 112 against the body's 121" | T-R000 Decision | headers total **121** once group 3 is read as its per-script table and group 6 as its own corrected tally line. Reading only the leading `| VERDICT | N |` rows gives 107 | `py` | FALSE. The sum check the decision rests on would not catch anything, because the total is 121 |
| 1,760 citations / 5 wrong | T-R000 Premise | round-1.md:17 and `:33` say exactly this | read | TRUE as an attribution; not re-counted, and round 1 did not open every citation |
| 265 parity rows | T-R000 item 1, PLAN VC | **265** inside `:232-537` | `sed -n '232,537p' ... \| grep -c 'Row {'` | TRUE |
| 267 | T-R000 Premise | **267** for a whole-file `grep -c 'Row {'`, and the two extras are `struct Row` at `:58` and `impl Row` at `:65` — exactly as claimed | `grep -c 'Row {' crates/podbox-cli/src/parity.rs`; `grep -n 'Row {'` filtered to lines outside 232-537 | TRUE |
| 164 | T-R000 item 1 | `experiments/results/parity-drive.txt:7` reads `164 rows` | read | TRUE |
| 160 | T-R000 item 1 | `TODO/cli.md:685` reads `160 rows` | read | TRUE |
| 220 | T-R000 item 1 | `TODO/cli.md:63` reads `**220**` | read | TRUE |
| 69 verbs | `TODO/cli.md:63` (adjacent) | the table holds **76** distinct `verb:` values | `py` over the range | the neighbouring figure is stale too; T-R000 does not name it |
| clause-1 40 flags | T-R000 item 2 | the script's clause-1 loop carries **40** flags and prints `$refused/40`; `experiments/results/parity-curated.txt:8` reads `40/40` | `py` over the `for f in …; do` block, filtering line continuations | TRUE |
| "46 curated flags, 40 refused, 6 driven in clauses 3 to 5" | T-R000 item 2 | 46 is **not** a count of anything in the script. The script has 40 clause-1 flags, 10 clause-2 verbs, and `--env-file`, `--strict`, `--log-driver`, `--device` elsewhere. The reconciliation does not add up | same parse | FALSE as stated |
| nine ASCII tests | T-R000 item 3 | **10** `*_plain_ascii` tests exist; `TODO/cli.md:1246` says nine | `grep -rn 'fn .*ascii' crates/podbox-cli/src/` | TRUE |
| 0.936 | T-R000 item 4 | the file reads **0.935** | read | FALSE |
| 1.869 | T-R000 item 4 | appears nowhere under `experiments/results/` | `grep -rn '1\.869' experiments/results/` | TRUE |
| `guest.tcg.boot` could-not-run, "no qemu-system-x86_64 on PATH" | T-R000 item 4 | `experiments/results/perf-lane.txt:123` `guest.tcg.boot tcg - s wall could-not-run`; `:125` `note: no qemu-system-x86_64 on PATH` | read | TRUE |
| 24 of 24 store tests | T-R000 item 13 | **30** `#[test]` in `store.rs`; 29 take `STORE_TESTS` (31 `fn` minus `Default::default` at `:184` and `last_refusal` at `:1274`, which are helpers). 30 − 1 helper = 29 | `grep -c '#\[test\]' store.rs`; a `py` pass pairing each `#[test]` fn with its body | the corrected figure is **29**, not the entry's implied 29; the entry says "the current tree has 29", which is right |
| 112 `interpose.map` names | PLAN §3 | **112**, and 112 by the scripts' own awk | `awk '/global:/{g=1;next} /local:/{g=0} g && /;/{...}' interpose.map \| wc -l` | TRUE for the file; the saved `interpose-ownership.txt:25` reads **105**, a stale run from 2026-09-19 |
| 13 `Prove` lines naming plant.sh | PLAN VC-5 | **16** | `py` walking `Prove:` blocks in cli.md, deps.md, gate.md | FALSE |
| 43 plant cases | not stated in the entries | `scripts/plant.sh` has **43** `case_plant` calls | `grep -c '^case_plant ' scripts/plant.sh` | TRUE |
| 980 test functions | PLAN §1, meta | **980** | `grep -rn '#\[test\]' crates --include='*.rs' \| wc -l` | TRUE |
| 950 / 30 split | PLAN §1, meta | **950** under `crates/*/src` (including the excluded interpose) and **30** under `crates/podbox-ssh/tests/` | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/src crates/*/build.rs \| awk -F: '{s+=$2}'` | TRUE |
| 12 tests vs 11 clauses, `388` | T-R001 | 12 `#[test]`, 11 clauses. The mapping holds, but clause 10 (sftp) is "record only" in the script and clause 2 (the sshd daemon) is the fixture, not an assertion | counted both | TRUE for the count |
| 6 checks vs 6 tests, `220` | T-R001 | 6 checks (A to F) and 9 tests that touch the six | read both | TRUE |
| 3 of 5, `160` | T-R001, PLAN §4 | 5 clauses; `contain.rs:118` is a store-symlink test, not a GC clause | read | the "3 of 5" is asserted by both PLAN and T-R001 and neither shows the arithmetic |
| 10 workspace members | PLAN §3, T-R005:128 | **nine** | `py` counting `"crates/` inside the `members = [...]` block in `Cargo.toml` | FALSE. PLAN says "a tree of ten" and "Ten members become fourteen"; T-R005 says "ten members become thirteen here and fourteen after wave 4" |
| `crate-type = ["cdylib"]` at `crates/podbox-interpose/Cargo.toml:15` | PLAN §3 | exact | read | TRUE |
| `[dependencies]` empty at `:18` | PLAN §3 | `:18` is `[dependencies]` with nothing under it; `:19` is blank | read | TRUE |
| excluded at `Cargo.toml:20` | PLAN §3 | `:20` is `exclude = ["crates/podbox-interpose"]` | read | TRUE |
| 17 declared / 17 exported | `TODO/interpose.md:87` | the map declares **112**; the saved result reads **105/105** | read both | the line is stale in three ways and T-R000 correction 6 names none of them |
| 10 members / 14 categories | `TODO/INDEX.md:42-58` | that range is the 15-row category table, ending at `:58` `\| podssh \|` | read | TRUE |

Numbers re-counted: 26. Wrong: 8 (KEEP-SHELL, SPLIT, RUST-TEST, DELETE,
"43 converge", "84 of 121", "six of ten headers", "112", the `13`, the
`0.936`, and the "ten members"). Correct: 18.

---

## Corrections check

**1. `TODO/cli.md:63`, 220 → 265.** The record is wrong. `TODO/cli.md:63`
reads `| rows | **220**. ⚠ 131 when this closed; … this row is the CURRENT
count so the two cannot drift apart`. The table holds 265. The correction is
right. **The mark-as-historical instruction is not enough**: the adjacent row
`| of which verbs | **69** |` is also stale (76 distinct verbs), and the entry
does not name it, so an implementor following the entry literally leaves a
second wrong number in the same table.

**2. `TODO/cli.md:182`, 41/41 → 40/40.** The record is wrong. `TODO/cli.md:182`
reads `clause-1 41/41`. The script prints `$refused/40` at `:106` and
`experiments/results/parity-curated.txt:8` reads `40/40`. The corrected value is
right. **The reconciliation is wrong.** "46 curated flags, 40 refused by the
loop, 6 driven in clauses 3 to 5" does not describe the script: the clause-1
loop holds 40 flags, not 46, and the other clauses carry `--env-file`,
`--strict`, `--log-driver` and `--device`, not six further flags from a 46-item
list. `TODO/cli.md:165` also says "the issue's 46 flags", so there is a
second 46 to reconcile and the entry names only one.

**3. `TODO/cli.md:1246`, nine → ten ASCII tests.** The record is wrong. Ten
`*_plain_ascii` tests exist. **The entry states the defect but not the
correction.** An implementor is left to find the ten by hand. Acceptable, but
the value should be in the entry.

**4. `TODO/gate.md:1430-1431`, remove the TCG figure.** The record is wrong.
`1.869` appears nowhere under `experiments/results/`; the TCG rows live in
`experiments/results/perf-lane.txt:123-125` as `could-not-run`. **The
correction introduces a new wrong number.** `experiments/results/perf-kvm.txt`
reads **0.935**, not 0.936, so "keep the KVM one with its conditions" carries
a figure the file does not hold. The entry also says "The lane file records
`guest.tcg.boot ... could-not-run`" without naming the lane file; the reader
must guess `perf-lane.txt` rather than `perf-kvm.txt`.

**5. `TODO/probe.md:171`, ENOSYS → ESRCH, commit `6eb941f`.** The record is
wrong. `attribute.txt:7` reads `ESRCH`, and `git show 6eb941f` shows the
re-capture changing that line. The correction is right and the commit is named.
**The entry does not say what the corrected row should read.** `TODO/probe.md`'s
table has two rows, one for the target (`ESRCH`, executed) and one for the host
(`ENOSYS`, from this file). Correcting the host row to ESRCH makes both rows
read ESRCH, which destroys the disagreement the entry exists to preserve. The
implementor must decide what the table now argues. The entry does not settle it.

**6. `TODO/interpose.md:87` and `:1445-1446`.** The records are wrong.
`lib.rs:334` is `next_renamexattr` and `lib.rs:989` is
`path_int!(remove, next_remove, …)`; `fchmodat` is at `:356` and `:1038`.
**The correction gives no corrected values at all** — it says "carry stale
`lib.rs` line numbers" and stops. Every other item names the line that settles
it; this one does not. An implementor must re-derive `:356` and `:1038`
himself. Worse, `:87` does not cite `lib.rs` at all: it says `17 declared, 17
exported`, which is a stale *count* against a map that declares 112 and a saved
result that read 105. The entry misreads its own target.

**7. T-0211's Done, re-anchor or reopen.** The record exists and quotes a
2026-09-12 run at `TODO/image.md:1162`. **The entry does not say which.** It
offers two options and settles neither, and the two have different records: a
re-anchor needs a fresh run, a reopen needs a Status change and a count update
through `scripts/todo-count.py`. An implementor must choose. The entry's own
Decision (`T-R000:39-41`) says re-anchor; item 7 contradicts it by leaving the
choice open.

**8. `157` re-anchor to `store.rs:1083`.** Correct. The script's sed at `:157`
names `lock.fd`; the source at `:1083` reads `close_in_children(fd)` inside
`try_acquire`. **The behavioural claim is right and I confirmed the mechanism
from the script itself**: `mutate` at `:125-135` hashes the subject before and
after, prints `⛔ THE MUTATION DID NOT LAND` and sets `fail=1` when they are
equal, and the script's own exit is `[ "$fail" -eq 0 ]`. The entry says "the
script's own mutation-landed guard fires correctly and then exits 1". TRUE.
The sed pattern given is right.

**9. `TODO/gate.md:434`, stale `Source`.** The record is wrong.
`:434` reads `Source: Cargo.toml:13-18; scripts/dev.sh:199-202`. `Cargo.toml:13`
is `]` and the reason is at `:15-19`; `dev.sh:199-202` is `echo ready` and
`echo dev.sh: $BIN`, while `build-interpose.sh` is called at `:85`, `:195` and
`:220`. **The entry names no corrected value.** Same defect as item 6.

**10. `experiments/README.md:35` overstates `392`.** The record is wrong.
`README.md:35` lists the script's required conditions in a table headed
"Repository audit proofs", and `TODO/PROGRESS.md:63` records the 2026-09-30
repetitions as failed. **The corrected text is not given**, and this is the
weakest of the fourteen: `README.md:35` does not state a result at all. It
states required conditions. The table's heading is what overstates, and the
correction belongs to the heading, not to the row. `:35` also cannot be
corrected by marking it historical, because the row is the driver's contract.

**11. `windows-365.txt` / `windows-364.txt`.** The record is wrong.
`371-validationos-stream.sh:193` writes `windows-365.txt`;
`370-windows-guest.sh:205` writes `windows-364.txt`. Both files exist. No
`TODO/` entry cites either name. **The entry gives no correction.** It
describes the collision and stops. An implementor must decide whether to
rename the files, rename the copies, or accept the collision — and
`docs/methodology/experiments.md` says a number is never reused, which
`scripts/check-todo.py` check 18 enforces, so the choice has a gate
consequence the entry does not name.

**12. T-1212's `Prove` versus its transcript.** The record is wrong.
`:964-968` requires six scripts to "each exit 0" and `:1037` records `EXIT:2`
for `300`, with `:978` naming it a SKIP. **The correction is not given.**
Whether the `Prove` changes to "each exits 0 or 2 with the reason named" or the
transcript moves to `docs/history/` is a judgement the entry does not make, and
`TODO/RULES.md:52` ("correct current text in place; keep superseded evidence in
`docs/history/`") points both ways.

**13. `TODO/image.md:1884,1894`, 24 of 24 → 29.** The record is wrong.
`store.rs` now holds 30 `#[test]`, 29 of which take `STORE_TESTS`.
**The balance does not hold at 29.** The recorded audit compares *test functions*
against *mutex acquisitions*: 29 tests against **29 acquisitions**
(`grep -c 'STORE_TESTS.lock()' store.rs` = 29, since `:2102` is the declaration).
So 29 vs 29 is right, and the entry's "the balance holds" is true. Correct.

**14. `TODO/image.md:507`, `ENG_NETWORK` confirmed present.** The record is
right; `experiments/lib/engine.sh` carries it at `:70`, `:294-297` and `:311`.
"No correction; recorded so a reader does not re-open it" is the correct
outcome, and the four cited lines all resolve. TRUE. Note that `:294-297`
resolves as `:294` (`case`), `:295-296` (the arms), `:297` (`esac`) — the range
is right.

Corrections checked: 14. The record is genuinely wrong in all 13 that propose a
change. **None corrects a record that was already right.** Six give no
corrected value (3, 6, 7, 9, 10, 11, 12 — seven, counting item 7's open
choice), one introduces a new wrong number (4), one carries a false
reconciliation (2), and one misreads its target (6).

---

## Self-sufficiency gaps

**T-R000**

1. No instruction for what to do when a record is wrong but the entry gives no
   corrected value. Items 3, 6, 7, 9, 10, 11, 12 are in that state. The entry's
   own preamble promises "the corrected value and the line that settles it";
   seven items break that promise.
2. Correction 5 leaves the two-row table in `TODO/probe.md` with both rows
   reading ESRCH and no instruction about what the entry then argues.
3. Correction 2 does not tell the implementor that `TODO/cli.md:165` also says
   "46 flags", so the reconciliation has a second half it must find.
4. Correction 4 does not name the lane file (`perf-lane.txt`), and hands over a
   figure (`0.936`) the KVM file does not hold.
5. Correction 10 names no change to `experiments/README.md:35`, and the row is
   a conditions row, not a result row. The implementor must decide whether the
   table heading or the row is what changes.
6. Correction 11 does not say whether the two result files are renamed, the
   copies changed, or the collision accepted, and does not say that check 18 of
   `scripts/check-todo.py` reads the choice.
7. Correction 12 does not say which side of `TODO/RULES.md:52` to take.
8. The Decision asserts "Six of ten headers disagree with their own bodies"
   and "header labels total 112". An implementor who goes looking for the six
   finds four, and a sum check over the headers returns 121 or 107, never 112.
   The Decision, which the entries cite as the reason to prefer bodies over
   headers, does not hold.
9. The entry says `py scripts/check-todo.py` is its `Prove`. It does not say
   that command cannot run the experiments it is correcting records about, which
   the Counts section does say. An implementor could stop at a green record
   gate and believe item 8's KVM figure was re-measured.

**T-R001**

10. The clause-to-test table's *check* column is mislabelled in four of six
    rows (see the citation table). `symlink target`, `tar member mode`,
    `device node` and `ownership` do not describe what the cited tests assert.
    An implementor writing a record repoint would record the wrong clause.
11. `session_interactive.rs:255-482` starts at the `#[test]` attribute of the
    first test, not its `fn` at `:255`. Minor, but the entry says the range
    carries 12 tests and the reader has to verify that by counting.
12. "Repoint `TODO/podssh.md:79` and `:126`" does not say to what. The entry
    says repoint "from the script to that file", which is not a `Prove` line:
    `TODO/podssh.md:79` is a compound `Prove` that must keep
    `cargo test -p podbox-ssh exits 0`. The implementor must write it.
13. "Repoint the owning `TODO/extract.md` entry's `Prove` to that file" does
    not name the entry. `TODO/extract.md` has two entries whose `Prove` names
    the script (`:50` and `:237`), plus prose at `:68` and `:308`, plus
    `TODO/milestones.md:203` and its own table at `:213`. Five sites, one
    named.
14. "Assert the exit code each verb actually returns, or fix the source first"
    leaves the decision to the implementor, and the two branches have different
    sizes (a test versus a behaviour change in a release verb).
15. Nothing says what happens to the experiment *number*. `220` and `388` are
    permanent under `experiments/README.md:3`, and check 18 of
    `scripts/check-todo.py` reads a second name on a taken number as a failure.
    Deleting the file releases the number; whether it may be reused is not
    stated in either entry.
16. The `Prove` is `cargo test --workspace`, which cannot run on this host. The
    entry names `sh scripts/windows/run-in-base.sh` at the end, but not as the
    `Prove`, so the record would be closed against a command that exits non-zero
    for a missing linker.
17. The Decision claims "`scripts/check-todo.py:300` `check_tree` reads every
    tracked file, so a `Prove` naming a deleted script is a red gate." I tested
    it: for `220` the gate is red, from six sites. For `388` it is **green**
    until the `sh ` prefix is removed from the citation. The claim is true for
    `220` and conditionally true for `388`; the entry does not distinguish them,
    so an implementor who repoints `:79` mechanically keeps a red-looking line
    that the gate does not catch.

---

## Internal consistency

**Cross-document conflicts**

- **Verdict distribution.** `PLAN.md:29-33` and `T-R000.md:20` both rest on
  the audit's own verdict labels. The labels in the ledger are
  DELETE 12, KEEP-SHELL 27, RUST-TEST 20, RUST-TOOL 27, SPLIT 35. The PLAN's
  table is DELETE 11, KEEP-SHELL 29, RUST-TEST 18, RUST-TOOL 27, SPLIT 36.
  Two of the five differ by one, two by two and two. `T-R000` does not correct
  any of them, so an implementor who builds waves off the PLAN's table assigns
  different scripts to different waves than one who reads the ledger.
- **"Ten members become fourteen."** `Cargo.toml:3-13` lists nine.
  `T-R005.md:128-129` repeats the ten. `PLAN.md:50,62` says ten and fourteen.
  Every count after wave 4 is therefore one too high.
- **The header-vs-body decision.** `PLAN.md:20-21` and `T-R000.md:33-38` both
  say six of ten headers disagree and the labels total 112. My count says four
  disagree and the total is 121. The decision's premise is wrong even though its
  ruling (prefer bodies) is right.
- **The `388` test file.** `PLAN.md:91` and `T-R001.md:26` both give
  `session_interactive.rs:255-482`. Consistent. But `PLAN.md:91` says the tests
  were found because "the script is cited in their doc comments" — they are not
  cited anywhere in that file.
- **The `170` destination.** `T-R001.md:89` routes `170`'s unit half to wave 2
  without naming a file. `T-R002.md:59` names
  `crates/podbox-probe/src/probe_cache.rs`, which does not exist;
  `probe_cache.rs` is `crates/podbox-image/src/probe_cache.rs`. Two entries,
  two answers, one of them a path that cannot be created without moving a
  module across crates.
- **The `90` destination.** `T-R001.md:91` says
  `crates/podbox-enter/src/abi.rs`, wave 2. `T-R002.md:55-57` sends check D to
  `abi.rs` and checks A and B and C to `crates/podbox-enter/src/identity.rs`,
  which does not exist either. The NSS dispatcher lives in
  `crates/podbox-complete/src/identity.rs` (`:6`, `:10`, `:173`).
- **`interpose.map` count.** `PLAN.md:79` says 112, which is right for the file.
  `experiments/results/interpose-ownership.txt:25` reads 105, and
  `TODO/interpose.md:87` reads 17. Three values for one fact, exactly the
  pattern `T-R000` correction 1 fixes for the parity table — and `T-R000`
  correction 6 names the line without naming the defect.
- **`record-fixes.md`.** `PLAN.md:167` says the entries directory holds
  `T-R000.md` .. `T-R005.md`, `verdict-ledger.tsv` **and `record-fixes.md`**.
  That file does not exist. The wave-0 corrections live in `T-R000.md` itself.

**Unassigned scripts.** None of the 121 is orphaned: every ledger row names a
group, every group has ten-to-thirteen rows, and the per-group counts match
`refactor/00-orientation/assignment-map.md` exactly. But the *wave* assignment
is only partial for the two scripts under scope: `388` and `220` are deleted in
wave 1, `157` is re-anchored in wave 0 and rebuilt as a `podbox-release` binary
in wave 5 (`T-R005.md:70,78`). No script is in two waves.

**Double-assigned scripts.** None found. The six "not deleted" scripts in
`T-R001.md:86-93` are each also named in `T-R002.md:51-62`; that is a
deliberate hand-off, and both entries say wave 2.

**Unrunnable proves.** `T-R000`'s `Prove` (`py scripts/check-todo.py`) runs on
this host and exits 0. `T-R001`'s `Prove` (`cargo test --workspace`) cannot:
`linker 'cc' not found`. `T-R001:109-111` says so and names
`sh scripts/windows/run-in-base.sh`, but the `Prove` field itself does not.
`T-R002.md:56,59` name two `crates/podbox-enter/src/identity.rs` and
`crates/podbox-probe/src/probe_cache.rs` targets that are not files in the tree.

**Crate names.** Consistent across `T-R001` and `T-R002` for the crates that
exist (`podbox-ssh`, `podbox-extract`, `podbox-cli`, `podbox-image`). Inconsistent
for the three destinations `T-R002` names that do not.

---

## What each Prove establishes

### `py scripts/check-todo.py` (T-R000)

**Proves.** That the records agree with the tree on the 30 checks the script
runs: counts against rows, index status against entry status, every citation
resolving to a real file and a real line, every experiment number carrying one
name, the parity notes, the ASCII output, the size ceiling, the interposer
sizes and exports. I ran it: `203 rows, 203 entries, 0 open, 2 partial, 0
blocked, 201 done`, `check-todo: ok`, exit 0.

**Does not prove.** That any of the fourteen corrections is right. The gate
checks that a citation *resolves*; it does not check that the cited line says
what the citing record claims. `TODO/cli.md:63`'s 220 resolves today and is
still wrong. Every correction in `T-R000` is a *semantic* fix that this gate
cannot see.

**Does not prove.** That any measurement was re-taken. The entry says so in
its Counts section, but the `Prove` field does not.

**Would pass without reaching its subject.** Yes, and the repository says so
itself: `check-todo.py` "reports success identically whether its assertions
passed or whether it examined nothing", which is why `scripts/plant.sh` exists
(`gate.yml:46-50`). Wave 0 changes no check, so it runs no plant, and
`T-R000`'s `Prove` inherits that gap unchanged.

### `cargo test --workspace` (T-R001)

**Proves.** That the suite is green after both deletions and both repoints,
including the six `crates/podbox-extract/src/drive.rs` tests and the twelve
`crates/podbox-ssh/tests/session_interactive.rs` tests that now stand alone
without their scripts.

**Does not prove.** That the deleted clauses still hold. A test that was
already there proved what it already proved. `T-R001.md:103-107` says this and
is right: the clause-to-test table is what carries the claim, and that table is
checkable only by reading.

**Does not prove.** That the two scripts were fully converted. I checked clause
by clause. `388` clause 2 is "sshd daemon answers on 2222 with ForceCommand",
which the test file exercises as its fixture rather than as an assertion;
clause 10 is "(record only)" in the script and `subsystem_sftp_never_reaches_the_shell`
at `:410` asserts more than the script recorded. `220` check E drives three
symlinks in one extraction where the script reads three `readlink`s off one
extraction — that one holds. The entry's "fully converted" is right for `220`
and loose for `388`.

**Cannot run on this host.** `linker 'cc' not found`. It needs the Linux base
lane. An implementor who records this as the wave's `Prove` without recording
the lane is closing an entry against a command that never ran.

**Would pass without reaching its subject.** Partly. Deleting a script changes
no Rust, so a green suite is the expected result whether the scripts were
converted or not. The proof is red only if the repointed `Prove` lines name a
file git no longer tracks. For `220` I confirmed six sites go red:
`TODO/extract.md:68`, `:308`, `crates/podbox-extract/src/drive.rs:543`,
`crates/podbox-extract/src/lib.rs:278`,
`docs/history/source-progress-ea5b671.md:118`,
`experiments/85-completion-symlink-escape.sh:20`. For `388` I confirmed the
gate stays **green** if the citation keeps its `sh ` prefix, because check 14
matches bare `` `experiments/…` `` and `sh experiments/…` is not bare. Only
after the prefix is stripped does it go red. `T-R001.md:62-63` says "repoint
`TODO/podssh.md:79`", which does not say that.

### A proof the plan does not name but needs

Wave 1 deletes two experiment scripts. `experiments/README.md:3` says "Its
number is permanent. Do not reuse a number", and `scripts/check-todo.py` check
18 (`:470`) reads one number carrying two names as a failure. Neither entry
states whether `220` and `388` may be reused. Nothing in either entry settles
it, and the gate will not complain either way.

---

## Corrections required

1. **Fix the verdict table before anything else.** `PLAN.md:29-33` must read
   DELETE 12, KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20, SPLIT 35, from
   `verdict-ledger.tsv`. I recounted it with
   `py` over the ledger's third column, and against the group report bodies
   with a `py` pass over every `**Verdict:` / `### Verdict:` heading. The two
   disagree only if the ledger is wrong; it is not, because all 121 of its
   `report_line` citations put the named verdict on the cited line.
2. **`PLAN.md:34,36`**: "43 converge wholly on a Rust test or tool" and "84 of
   121 can be retired" follow from the wrong table. With the corrected table
   they are 47 and 94.
3. **Rewrite `T-R000.md:33-38`'s Decision.** "Six of ten headers disagree"
   is four. "The header labels total 112" is 121. The ruling survives; the
   premise does not, and it is cited as authority.
4. **`T-R000.md` item 4**: replace `0.936` with `0.935`, and name
   `experiments/results/perf-lane.txt` as the file holding the TCG rows.
5. **`T-R000.md` item 2**: delete the "46 curated flags, 40 refused, 6 driven"
   sentence. The script has 40 clause-1 flags. Also fix `TODO/cli.md:165`'s
   "46 flags" in the same change.
6. **`T-R000.md` items 3, 6, 9**: give the corrected value. Ten ASCII tests;
   `lib.rs:356` and `:1038`; `Cargo.toml:15-19` and `scripts/dev.sh:85,195,220`.
7. **`T-R000.md` item 6**: `:87` does not cite `lib.rs`. It says `17 declared,
   17 exported` against a map that declares 112 and a result that read 105.
   Correct the count, not a line number.
8. **`T-R000.md` item 5**: say what the two-row table in `TODO/probe.md` becomes
   when both rows read ESRCH.
9. **`T-R000.md` items 7, 10, 11, 12**: give one instruction each. The entry's
   own Decision settles item 7 as re-anchor; make items 10, 11 and 12 match
   `TODO/RULES.md:52`.
10. **`T-R001.md:70-77`**: relabel the clause-to-test table. `drive.rs:211` is
    a `..` member, `:229` an absolute member name, `:264` an absolute symlink,
    `:382` a hard link. Ownership is `:480`, relative-symlink escape is `:192`.
11. **`T-R001.md:26-29`**: delete "the script is cited in their doc comments".
    Nothing in `crates/podbox-ssh/tests/session_interactive.rs` names
    `388-interactive-shell.sh`. Say the tests were located by reading the
    script's clauses against the test names.
12. **`T-R001.md:62-63`**: write the replacement `Prove` text for
    `TODO/podssh.md:79` verbatim, and state that the `sh ` prefix must be
    dropped or check 14 will not see it. For `220`, name all six sites, not
    "the owning `TODO/extract.md` entry": `TODO/extract.md:50`, `:68`, `:237`,
    `:308`, `crates/podbox-extract/src/drive.rs:543`,
    `crates/podbox-extract/src/lib.rs:278`, `TODO/milestones.md:203`, `:213`,
    `experiments/85-completion-symlink-escape.sh:20`,
    `docs/history/source-progress-ea5b671.md:118`.
13. **`T-R001.md:95-99`**: decide the `160-store-gc.sh` question. "Assert the
    exit code each verb actually returns, or fix the source first" is a choice
    left to the implementor, and one branch is a behaviour change to a shipped
    verb.
14. **Both entries**: state that `experiments/README.md:3` makes `220` and
    `388` permanent numbers and that check 18 reads a reuse as a failure.
15. **`T-R001.md:51`**: change the `Prove` to the lane command, or record that
    it was run on `sh scripts/windows/run-in-base.sh` with the lane's exit.
16. **`PLAN.md:50,62` and `T-R005.md:128-129`**: the tree has **nine** members,
    not ten. Nine becomes thirteen.
17. **`PLAN.md:167`**: `refactor/06-entries/record-fixes.md` does not exist.
    Either write it or drop the line.
18. **`T-R002.md:56,57,59`**: three destination files do not exist.
    `crates/podbox-enter/src/identity.rs` → `crates/podbox-complete/src/identity.rs`;
    `crates/podbox-probe/src/probe_cache.rs` → `crates/podbox-image/src/probe_cache.rs`.
    T-R001's "not deleted" table routes the same two scripts elsewhere. Both
    entries must agree before either is implemented.