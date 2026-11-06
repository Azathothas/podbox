# Verify T-R004.md and T-R005.md — the four tool crates and the gate

## Scope and method

I verified the two entries assigned to me — `refactor/06-entries/T-R004.md`
(wave 4, `crates/podbox-gate`) and `refactor/06-entries/T-R005.md` (wave 5,
`podbox-buildstate`, `podbox-release`, `podbox-podvm`) — plus the T-R000
correction list that both depend on. I read every file either entry or T-R000
cites, opened every `path:line` citation at that line, and re-counted every
number with my own command.

I read in full: `AGENTS.md`, `TODO/RULES.md`, `TODO/INDEX.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `docs/methodology/gate.md`,
`experiments/README.md`, `.github/workflows/gate.yml`, `Cargo.toml`,
`refactor/06-entries/{PLAN,T-R000,T-R004,T-R005}.md`,
`refactor/06-entries/verdict-ledger.tsv`. I read the ten
`refactor/01-audit/group-*.md` reports for the ledger cross-check, the
orientation inventory, and the meta report and meta review for the proposed
crate names and the cdylib claim.

Counts of what I checked: 61 `path:line` citations opened individually, 12
whole-number claims re-counted, 14 T-R000 corrections assessed, 6 entry Prove
commands and 2 record-gate jobs reasoned about. I ran the record gate
(`py scripts/check-todo.py`, exit 0) because it runs on this host. Per the
task rules I did not run a full `cargo test` build; it fails here at
`linker 'cc' not found`.

What I could not settle: the exact per-script verdict for all 121 scripts.
The group report bodies do not encode verdicts in a uniform machine-readable
way — group-1/2/4/5/6/7/8/9/10 use `**Verdict: X**` while group-3 uses
`### Verdict: X`, and 13 `## Group N - <path>` headers are not script
sections (for example "conversion plan", "entries read"). A naive body parse
returns 135 sections with DELETE inflated to 18. I therefore checked the
ledger's distribution against the group bodies only for the specific scripts
my two entries name, and I report the ledger/PLAN disagreement below as a
finding rather than silently trusting either.

## Citation table

| Citation | Claim in the entry | What the line actually says | Verdict |
| --- | --- | --- | --- |
| T-R004 `.github/workflows/gate.yml:36-56` | the plant step region | lines 36-56 are the `check-todo`, `todo-count idempotent`, and `every check can fail` (plant) steps | correct |
| T-R004 `gate.yml:99` | interposer build step | `run: ./scripts/build-interpose.sh` | correct |
| T-R004 `gate.yml:199` | `cargo test --workspace` | `run: cargo test --workspace` | correct |
| T-R004 `gate.yml:213` | the interposer test | `run: cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` | correct |
| T-R004 `gate.yml:37` | check-todo runs in CI | `run: ./scripts/check-todo.py` | correct |
| T-R004 `gate.yml:42,56,59,71` | todo job steps | `run: |` (idempotent), `./scripts/plant.sh`, `run: |` (common checks), licence sha256 | correct |
| T-R004 `gate.yml:102,107,131` | build job steps | `cargo build --release --target ...`, `run: |` (no PT_INTERP), `./experiments/110-bloat-delta.sh ci` | correct |
| T-R004 `gate.yml:154,159,164,171,176,186` | lint job steps | fmt, clippy, interposer clippy, markers, shell parse, `py_compile` | correct |
| T-R004 `gate.yml:202,205,208` | test job steps | `393-build-freshness.py`, `398-gate-diagnostics.py`, `400-kvm-cleanup.py` | correct |
| T-R004 `gate.yml:55-56` | plant runs as step `every check can fail` | step name at 55, `run: ./scripts/plant.sh` at 56 | correct |
| T-R004 `gate.yml:197` is blank, `:211` is `env:` | the meta report's citations are off | `:197` blank; `:211` is the `env:` key under interposer tests | correct — both meta citations are indeed wrong as stated |
| T-R004 `scripts/check-todo.py:228` | `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` | exact line matches | correct |
| T-R004 `scripts/check-todo.py:300` | `check_tree` reads every tracked file | `def check_tree(files):` | correct |
| T-R004 `scripts/check-todo.py` is 1,738 lines | largest script | `wc -l` = 1738 | correct |
| T-R004 `scripts/plant.sh:51` | `FILES` names `experiments/110-bloat-delta.sh` | `FILES="TODO/INDEX.md ... experiments/110-bloat-delta.sh ..."` | correct |
| T-R004 `scripts/build-interpose.sh:197-214` | export check for BOTH targets | inside `for t in $TARGETS` (loop at :61, two targets); declares/exports compare at 197-214 | correct |
| T-R004 `build-interpose.sh:204-205` | prints `ok: exports %s names...` | the printf is at 205; 204 is the `if diff -q` guard | acceptable range, message at 205 |
| T-R004 `build-interpose.sh:199-200` | the awk that counts `interpose.map` names | awk at 199, `$CRATE/interpose.map` at 200 | correct |
| T-R004 `build-interpose.sh:48` | byte ceiling | `INTERPOSE_CEILING_BYTES=500000` | correct |
| T-R004 `crates/podbox-interpose/Cargo.toml:15` | `crate-type = ["cdylib"]` | exact | correct |
| T-R004 `crates/podbox-interpose/Cargo.toml:18` | empty `[dependencies]` | `[dependencies]` (empty) | correct |
| T-R004 `Cargo.toml:20` | crate excluded from workspace | `exclude = ["crates/podbox-interpose"]` | correct |
| T-R004 `docs/methodology/gate.md:23` | names the check set | "It runs formatting, lint, workspace tests, interposer tests, and repository checks." | correct |
| T-R004 `group-1.md` "one crate, two binaries... share the `ROW` grammar" | model quote | group-1:1028-1029 "One crate, two binaries, because the two share the `ROW` grammar..." | correct |
| T-R005 `TODO/INDEX.md:42-58` | category table, authority on crate ownership | the 14-row category/crate/specification table | correct |
| T-R005 `TODO/INDEX.md:52` | `deps` → "the whole tree" | `\| [deps](deps.md) \| the whole tree \|` | correct |
| T-R005 `TODO/INDEX.md:53` | `packaging` → "the artefact" | `\| [packaging](packaging.md) \| the artefact \|` | correct |
| T-R005 `TODO/INDEX.md:55` | `podvm` → "the machine tier" | `\| [podvm](podvm.md) \| the machine tier \|` | correct |
| T-R005 `TODO/RULES.md` grep 0 hits in 121 lines | no crate/member/workspace rule | `grep -in 'crate\|member\|workspac' TODO/RULES.md` = exit 1, 0 hits; `wc -l` = 121 | correct |
| T-R005 `experiments/157-lock-inheritance-prove.sh:157` | sed names `lock.fd` | `'s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/'` | correct |
| T-R005 `crates/podbox-image/src/store.rs:1083` | `if !sys::close_in_children(fd) {` inside `Store::try_acquire` | line matches exactly; `try_acquire` at :1074, and it is NOT inside `hold` (at :527) | correct |
| T-R005 `scripts/dev.sh:41` | writes `.dev/last-build-inputs` | `STAMP="$STATE/last-build-inputs"`, `STATE="$REPO/.dev"` at :37 | correct |
| T-R005 `experiments/153-store-lock-race.sh` (755 lines) | line count | `wc -l` = 755 | correct |
| T-R005 `experiments/190-parallel-layers.sh` (624 lines) | line count | `wc -l` = 624 | correct |
| T-R005 `145-podvm-parity.sh` pure half | asserts `podbox --help`/`podvm --help` verb set | :87-99 "both help texts list one set of verbs", compares and greps for run/exec/podvm/--podbox-tier | correct |
| T-R005 `TODO/deps.md T-0910`, `packaging.md T-1328/T-1002/T-1004/T-1314`, `complete.md T-1324`, `image.md T-0211/T-0215`, `enter.md T-0503`, `probe.md T-0111`, `packaging.md T-1005` | entry references | each id present in its named file | correct |
| T-R000 item 1 `TODO/cli.md:63` | says 220 rows, labelled CURRENT | `\| rows \| **220**. ... this row is the CURRENT count ...` | correct |
| T-R000 item 1 `experiments/results/parity-drive.txt:7` | says 164 | `== 0. the table is data: 164 rows` | correct |
| T-R000 item 1 `TODO/cli.md:685` | says 160 | line 685 is "rows, 199 driven, 0 mismatches, 0 unreachable here". The figure 160 is at :684. | **WRONG — off by one** |
| T-R000 item 1 `crates/podbox-cli/src/parity.rs:232-537` holds 265 | 265 rows | `232` = `pub const TABLE: &[Row] = &[`, `537` = `];`; `Row {` count in range = 265 | correct |
| T-R000 item 1 `parity.rs:58` / `:65` | `struct Row` / `impl Row` explain the +2 | both lines match; whole-file `Row {` grep = 267 | correct |
| T-R000 item 2 `experiments/355-parity-curated.sh:106` | prints `$refused/40` | `echo "clause-1 refused-with-reason  $refused/40" >>"$REPORT"` | correct |
| T-R000 item 2 saved result reads 40/40 | — | `experiments/results/parity-curated.txt:8` "clause-1 refused-with-reason  40/40" | correct |
| T-R000 item 2 `TODO/cli.md:182` | records `clause-1 41/41` | `clause-1 41/41 refused with status None at 125, clause-2 10/10` | correct |
| T-R000 item 3 `TODO/cli.md:1246` | names nine ASCII tests | "Nine unit tests hold the constants"; actual `*_is_plain_ascii` test fns = 10 | correct (record is stale by one) |
| T-R000 item 4 `TODO/gate.md:1430-1431` | "KVM guest boot 0.936 s against TCG 1.869 s" | the KVM/TCG sentence is at :1431 ("boot 0.936 s against TCG 1.869 s"); 1430 is the line above | acceptable range |
| T-R000 item 4 `experiments/results/perf-kvm.txt` reads `0.936` | the KVM figure | the file reads **0.935** at :133; `0.936` appears nowhere in `experiments/results/` | **WRONG — figure is 0.935** |
| T-R000 item 4 `1.869` appears nowhere under results | TCG figure unsupported | `1.869` appears only in `TODO/gate.md:1431` and a history doc, not in any results file | correct |
| T-R000 item 4 lane file `could-not-run`, "no qemu-system-x86_64 on PATH" | TCG leg did not run | `perf-lane.txt:123-124` `guest.tcg.boot ... could-not-run` | correct |
| T-R000 item 5 `TODO/probe.md:171` | quotes attribute.txt as ENOSYS | `errno=38 ENOSYS` ... `experiments/results/attribute.txt` | correct |
| T-R000 item 5 attribute.txt reads ESRCH at line 7 | re-captured reading | ESRCH is at line **6** (`kcmp ... ESRCH`); line 7 is `fsopen(tmpfs) OK` | **WRONG — line is 6** |
| T-R000 item 5 `git log --follow` shows commit `6eb941f` | re-capture commit | `6eb941f gate: close T-1213...` is the top commit for attribute.txt | correct |
| T-R000 item 6 `TODO/interpose.md:87`, `:1445-1446` | stale lib.rs line numbers | :87 is an export-count line; :1445-1446 cite `lib.rs:334`/`:989` | plausible stale citations; not fully re-derived |
| T-R000 item 9 `TODO/gate.md:434` | stale `Source` line | `Source: \`Cargo.toml:13-18\`; \`scripts/dev.sh:199-202\`` — a static range that has drifted | plausible; not fully re-derived |
| T-R000 item 10 `experiments/README.md:35` | `392-kvm-guest.sh` overstates a passing proof | row exists in the "Repository audit proofs" table describing required conditions, not a pass; PROGRESS:63 records 2026-09-30 KVM failures | finding is defensible; wording "overstates" is a judgement |
| T-R000 item 11 `371:193`, `370:205` | write `windows-365.txt` / `windows-364.txt` | both cp lines match; both files exist | correct |
| T-R000 item 12 `gate.md:964-968`, `:1037`, `:978-980` | Prove wants exit 0; transcript records EXIT:2 and prose calls it SKIP | all three match | correct |
| T-R000 item 13 `TODO/image.md:1884,1894` | "24 of 24" store tests; current 29 | :1884 and :1894 both read 24; `store.rs` has 30 `#[test]` now | record stale; current figure differs (see corrections) |
| T-R000 item 14 `experiments/lib/engine.sh:70,294-297,311` | `ENG_NETWORK` present | all five sites match | correct |

## Number table

| Number | Claim | My recount | Command | Verdict |
| --- | --- | --- | --- | --- |
| 121 scripts | corpus size | 121 | inventory rows + live tree both = 121, all present | correct |
| 29,153 lines | corpus lines | 29,153 | inventory field sum and `wc -l` over the 121 live files | correct |
| verdict distribution | KEEP-SHELL 29, SPLIT 36, RUST-TOOL 27, RUST-TEST 18, DELETE 11 | ledger field 3: SPLIT 35, RUST-TOOL 27, KEEP-SHELL 27, RUST-TEST 20, DELETE 12 | `awk -F'\t' '{print $3}' verdict-ledger.tsv \| sort \| uniq -c` | **PLAN table disagrees with the ledger on 4 of 5 verdicts** |
| distribution sums to 121 | — | ledger total = 121, unique = 121 | same awk, plus `sort -u \| wc -l` | correct (total) |
| "43 converge wholly" | RUST-TOOL + RUST-TEST | ledger: 27 + 20 = **47**. PLAN's own table: 27 + 18 = **45** | arithmetic on both sources | **WRONG — three totals (43, 45, 47) exist** |
| 265 parity rows | `parity.rs:232-537` | 265 | `sed -n '232,537p' \| grep -c 'Row {'` | correct |
| 267 whole-file `Row {` | the over-count | 267 | `grep -c 'Row {' parity.rs` | correct |
| `struct Row` :58, `impl Row` :65 | explain the +2 | both match | `sed -n '58p;65p'` | correct |
| 112 interpose.map names | declared exports | 112 | the script's own awk, piped to `wc -l` | correct |
| 13 plant.sh `Prove` lines | across 3 TODO files | 13, in `cli.md`, `deps.md`, `gate.md` | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c Prove` | correct |
| 980 test functions | total `#[test]` | 980 | `grep -rc '#\[test\]' crates/ --include=*.rs` summed | correct |
| 950/30 split | src/ vs tests/ | 950 src, 30 tests | meta's own command, and `crates/*/tests` | correct |
| 15 unwritten clauses | PLAN / T-R002 | 15 enumerated rows in T-R002:47 table | `sed -n '47,80p' T-R002.md` | correct |
| 43 converging scripts | PLAN total cell | see above — 43 is wrong against both the table (45) and ledger (47) | arithmetic | **WRONG** |
| 18 proposed crate names | meta report | 18 new names (not existing members) | meta-1 grep minus `Cargo.toml` members | correct |
| 10 of the 18 are bins | T-R005 list | all 10 are members of the 18 | set difference | correct |
| 18 proposed crate names | meta report | 18 new names (not existing members) | meta-1 grep minus `Cargo.toml` members | correct |

## Corrections check

Item-by-item for `refactor/06-entries/T-R000.md`:

1. **`TODO/cli.md:63` 220 → 265.** The record does claim 220 and labels it CURRENT. The corrected value 265 matches the counted range. The record is genuinely wrong. **Correction valid** — but the entry's own citation `TODO/cli.md:685` for the "160" figure is off by one (the 160 is at :684). The `experiments/results/parity-drive.txt:7` = 164 is correct. So the correction is right, one supporting citation is wrong.
2. **`TODO/cli.md:182` 41/41 → 40/40.** The record does say 41/41. The script prints `$refused/40` and the saved result reads 40/40. **Correction valid.**
3. **`TODO/cli.md:1246` nine → ten ASCII tests.** The record says "Nine"; the source has 10 `*_is_plain_ascii` functions. **Correction valid.**
4. **`TODO/gate.md:1430-1431` drop the TCG 1.869 figure.** The record does state 1.869, and 1.869 appears in no results file; the lane records the TCG leg `could-not-run`. Removing TCG and keeping KVM is right. **But the KVM value is wrong:** the entry says `perf-kvm.txt` reads `0.936`; the file reads **0.935** (`:133`). So the correction's *kept* value is misstated. **Correction partly wrong** — the direction is right, the retained figure is off by 0.001.
5. **`TODO/probe.md:171` ENOSYS → re-captured ESRCH.** The record quotes ENOSYS; the file has no ENOSYS and reads ESRCH. The substantive correction is right, and the re-capture commit `6eb941f` is real. **But the entry says ESRCH is at line 7; it is at line 6.** **Correction valid, cited line wrong.**
6. **`TODO/interpose.md:87` and `:1445-1446` stale lib.rs lines.** Both lines exist and carry lib.rs citations. I did not re-derive the drift; the entry asserts it is stale. **Not fully verified; plausible.**
7. **`TODO/image.md` T-0211 re-anchor.** T-0211's Done is dated `2026-09-12`; the entry says it quotes a 2026-09-12 green run of `157`. The date matches. **Correction valid in substance; the green-run claim rests on the record, which I did not re-run.**
8. **`experiments/157:157` re-anchor to `store.rs:1083`.** The script's sed names `lock.fd`; `store.rs:1083` reads `close_in_children(fd)` inside `try_acquire` (not `hold`). The mutation-landed guard would fire and the script exits 1 today. **Correction valid** — this is the wave-0 keystone and it is correct.
9. **`TODO/gate.md:434` stale `Source`.** The line carries a static range `Cargo.toml:13-18; scripts/dev.sh:199-202`. Whether it is stale needs a re-derive. **Not fully verified.**
10. **`experiments/README.md:35` overstates `392-kvm-guest.sh`.** The row is in a "proofs" table describing required conditions; PROGRESS records 2026-09-30 KVM failures. The finding is defensible, but "overstates a passing proof" is a judgement the row does not literally make. **Weak but not false.**
11. **`371:193` / `370:205` write the wrong result filename.** Both cp lines match and both files exist. **Correction valid.**
12. **`TODO/gate.md` T-1212 Prove vs EXIT:2 / SKIP.** All three citations match. **Correction valid.**
13. **`TODO/image.md:1884,1894` "24 of 24" vs current 29.** The record says 24; `store.rs` now holds 30 `#[test]`, not 29. The direction (record stale) is right, but **the entry's corrected figure 29 is itself off** — my count is 30. **Correction partly wrong** — correct that the record is stale, but 29 understates the current 30.
14. **`experiments/lib/engine.sh:70,294-297,311` `ENG_NETWORK`.** All sites present. Entry explicitly says "No correction". **Valid as a no-op record.**

Summary: 14 items. Fully valid: 2, 3, 8, 11, 12, 14. Valid in substance with a wrong supporting citation: 1 (`cli.md:685` should be `:684`), 5 (ESRCH at line 6, not 7). Partly wrong on the retained value: 4 (KVM is 0.935, not 0.936), 13 (current store tests are 30, not 29). Not fully re-derived: 6, 9. Judgement call: 10. Item 7's green-run claim rests on the record.

## Self-sufficiency gaps

An implementor with only these documents and the repository would have to
guess or look up the following.

Wave 4 (`crates/podbox-gate`):

1. **The row grammar is never specified.** T-R004:43 says the file grammar is
   "extracted so both `podbox-gate` and `podbox-count` share it", and quotes
   group-1's "`ROW` grammar, the status list and the priority list", but the
   entry never names the record fields, the status enum values, or the row
   regex. An implementor must read `scripts/check-todo.py` from scratch. The
   entry gives no field list.
2. **The exit-code contract is asserted, not written.** T-R004:40 says the six
   binaries "share the row grammar and the exit-code contract" but never states
   the contract (0 ok / 1 fail / 2 could-not-run, per `docs/methodology/experiments.md:12`).
3. **`podbox-smoke` has no defined scope.** T-R004:68 lists it as replacing
   `nightly-smoke.sh` and `398-gate-diagnostics.py`, but never says which
   checks it runs. `398`'s own header ("Drive gate failure output ... the
   PowerShell twin path") defines the subject, and the entry does not point at it.
4. **`podbox-count`'s output contract is unnamed.** It replaces `todo-count.py`
   and must write the `TODO/INDEX.md` count tables idempotently, but the entry
   does not say which tables or what the idempotence check is.
5. **The `podbox-interpose-build` check B is one line.** T-R004:100 says check
   B (struct stat/statx offsets under both libcs) "needs a `cc` invocation and
   belongs in this binary too", without naming the offsets, the two libcs, or
   the expected values. `T-R002` item 4 has the same gap.

Wave 5 (three crates):

6. **`build-state.py`'s fixed fixture is described in prose, not values.**
   T-R005:55 says the fixture is "a file changes with equal size and time,
   output changes, missing output is detected" but does not give the fixture
   bytes or the expected outputs. The implementor must read
   `scripts/build-state.py` and `experiments/393-build-freshness.py`.
7. **`podbox-prove-t0211`'s two scripts are joined without a shared interface.**
   T-R005:87-88 says `120` and `157` are "one binary because both are
   mutation-planted store-lock proofs", but names no shared function, no
   mutation-guard contract, and does not say which clauses each contributes.
8. **`157`'s re-anchored text is not given.** T-R005:81-85 says the binary
   "must encode the current source text", but the entry does not state what the
   re-anchored sed pattern is. Wave 0 defines the correction, but the implementor
   of wave 5 must locate `store.rs:1083` themselves.
9. **`395-reconcile-repository.py` and `399-publication.py` have no interface.**
   The table names them as `podbox-reconcile` and `podbox-publish` with no
   arguments, inputs, or output rows. These are non-trivial git operations.
10. **`145`'s "live-payload rows stay shell" is not itemised.** T-R005:104 says
    the pure half becomes a unit test and the rest stays shell, but never says
    which rows are pure versus live.
11. **The single-script RUST-TOOL table's `153` fate is open.** T-R005:113 says
    "decide its fate before porting" — that decision is not made in the entry.
    An implementor cannot finish `153` without an architectural decision the
    entry defers.
12. **`60-interposer-libc.sh` must stay out of `members`, but the entry does not
    say where it lives.** T-R005:119 says it is "podbox-gate's interposer bin"
    and must stay out of `members`, but does not resolve whether it is a
    `[[bin]]` inside `podbox-gate` (which would be a member) or a separate
    excluded manifest.

## Internal consistency

- **Crate names agree.** `PLAN.md` endorses `podbox-gate`, `podbox-buildstate`,
  `podbox-release`, `podbox-podvm`. T-R004 uses `podbox-gate`; T-R005 uses the
  other three. `podbox-release` is endorsed by the meta review
  (`meta-review-1.md:307,621`) though it was not one of the 18 the meta report
  proposed. No name conflicts between the entries and PLAN.
- **Member arithmetic.** `Cargo.toml` has 10 members. T-R005:129 says the three
  crates take it to 13, and 14 after wave 4. That is 10 + 3 (wave 5) = 13 and
  10 + 4 (wave 4 adds `podbox-gate`) = 14. Consistent.
- **Unassigned RUST-TOOL scripts.** `experiments/394-ssh-package.sh` and
  `experiments/397-exported-build.py` are RUST-TOOL in the ledger
  (`verdict-ledger.tsv:90,93`, both `**Verdict: RUST-TOOL.**` in group-9) and
  both files exist. Neither appears in any entry or in PLAN. **Two RUST-TOOL
  scripts are assigned to no wave.**
- **Verdict/disposition conflicts between the ledger and the entries.**
  - `experiments/398-gate-diagnostics.py` is **RUST-TEST** in the ledger and in
    the group-7 body (`group-7.md:1437`), but T-R004:68 assigns it as a source
    replaced by the `podbox-smoke` binary (a RUST-TOOL action).
  - `experiments/157-lock-inheritance-prove.sh` is **RUST-TEST** in the ledger,
    but T-R005:70 gives it a binary (`podbox-prove-t0211`).
  - `experiments/145-podvm-parity.sh` is **RUST-TEST** in the ledger, but
    T-R005:100 gives it a binary (`podbox-podvm`). The entry does partially
    mitigate this at :104 (the pure half becomes a unit test, live rows stay
    shell) — that is SPLIT handling, not a binary replacement.
  - `experiments/154-tcg-workload-spread.sh` is **SPLIT** in the ledger, but
    T-R005:101 gives it a binary with no split treatment.
  These are disposition errors, not citation errors: the entries convert
  RUST-TEST/SPLIT scripts into binaries the ledger did not authorise.
- **No script is double-assigned across waves** within T-R004/T-R005. The
  `110-bloat-delta.sh` coupled-path note is consistent between the entries
  (T-R005:92 points at wave 4; T-R004:52 owns the change).
- **Prove commands.** T-R004's `py scripts/check-todo.py` runs and exits 0 on
  this host. T-R005's `cargo test --workspace` does not run on this host
  (`linker 'cc' not found`); both entries state this and route to
  `sh scripts/windows/run-in-base.sh`, which exists.
- **The 110 coupled-change set is understated.** T-R005:92 and T-R004:52 name
  `check-todo.py:228` and `plant.sh:51` as the coupled path sites. `110` is also
  run by `gate.yml:131`, and read by `plant.sh:191` (`CEILING_NUM`) and named in
  `TODO/deps.md` and `TODO/gate.md`. Moving it touches more than the two files
  the entries name.
- **PLAN points at a file that does not exist.** `PLAN.md:167` names
  `refactor/06-entries/record-fixes.md` as a deliverable. That file is absent
  from `refactor/06-entries/`. The wave-0 corrections live only inside
  T-R000.md. This is a cross-document defect, not in my two entries, but an
  implementor following PLAN's file list will look for a file that is not there.

## What each Prove establishes

**T-R004 `py scripts/check-todo.py`.**
- Proves: the record gate still accepts the tree after the six binaries land
  and the `check-todo.py`/`todo-count.py` paths move — counts, links, and
  cited paths/lines stay consistent.
- Does not prove: that `podbox-gate` behaves like the Python gate it replaces
  (check-todo is the reader; after the port it runs *the Rust gate* or the
  Python one — the entry does not say which the Prove runs). It does not prove
  the port is behaviour-preserving. It does not exercise `podbox-smoke`,
  `podbox-interpose-build`, or the perf harness. It cannot pass without reaching
  its subject only if the ported gate is wired to be the thing check-todo runs;
  the entry does not state that wiring, so a green Prove could mean the old
  Python gate still runs.

**T-R004 "all four gate jobs" (PLAN's wave-4 proof).**
- Proves: CI is green with the paths moved — build, lint, test, and the record job.
- Does not prove: behaviour parity with the retired scripts. It cannot run on
  this host; the entry routes it to the Linux base lane.
- Unrunnable here: yes. `cargo test --workspace` and the release build both need
  `cc`; the entry says so.

**T-R005 `cargo test --workspace`.**
- Proves: the three new crates build and their unit tests pass, and the
  workspace still resolves with the new `members` rows.
- Does not prove: that the retired scripts' measurements survive. It is a
  compile-and-unit-test gate. It does not run the full CI gate, the record
  gate, the byte ceiling, or the interposer export check. T-R005:124 says "the
  full gate runs too" — but the entry's `Prove:` field names only
  `cargo test --workspace`, so the full gate is a comment, not the proof.
- Cannot run on this host: `linker 'cc' not found`. The entry states this.

**Values that are estimates or unknown.**
- T-R004:18 `Effort: XL`, T-R005:16 `Effort: L` — estimates, per
  `TODO/INDEX.md:32-33` ("Effort is an estimate of scope... not a measured
  duration"). Unlabelled as estimates in the entry headers, which is the
  convention the repo otherwise follows.
- The size of the ported `check-todo.py` (1,738 lines) and the number of gates
  it holds: the entry asserts "the same records the Python one accepts" but
  gives no count of records or checks to compare against. Unknown.
- `EXPECTED 112` interpose names is measured (112), not an estimate.
- The claim that `398` "must stay OUT of `members`" for `60-interposer-libc.sh`
  rests on a self-referential argument ("the subject of the check is the
  exclusion itself"). Whether cargo permits this arrangement is unverified.

## Corrections required

1. **PLAN.md:34 "43 converge" is wrong three ways.** Its own table gives
   RUST-TOOL 27 + RUST-TEST 18 = 45; the ledger gives 27 + 20 = 47. Pick one
   source, restate the count, and make the table agree with it.
2. **PLAN.md:27-33 verdict distribution disagrees with the ledger on four of
   five rows** (ledger: KEEP-SHELL 27, SPLIT 35, RUST-TEST 20, DELETE 12). The
   plan's table is the machine input an implementor will use; reconcile it with
   `verdict-ledger.tsv` or say why the ledger is wrong.
3. **`experiments/394-ssh-package.sh` and `experiments/397-exported-build.py`
   are RUST-TOOL and assigned to no wave.** Assign them to a wave or mark
   KEEP-SHELL. They exist and both carry `**Verdict: RUST-TOOL.**` in group-9.
4. **T-R005 converts RUST-TEST/SPLIT scripts into binaries without saying so.**
   `398` (RUST-TEST) → `podbox-smoke` bin; `157` (RUST-TEST) → `podbox-prove-t0211`
   bin; `145` (RUST-TEST) → `podbox-podvm` bin; `154` (SPLIT) → `podbox-podvm-workload`
   bin. Each is a disposition change from the ledger. State the override and
   why, or route them as tests/splits.
5. **T-R000 item 4 retains the wrong KVM figure.** `perf-kvm.txt:133` reads
   0.935, not 0.936. Correct the retained value.
6. **T-R000 item 13 corrects 24 to the wrong number.** `store.rs` holds 30
   `#[test]`, not 29. Recount and correct.
7. **T-R000 item 1 cites `TODO/cli.md:685` for the 160 figure; it is at :684.**
8. **T-R000 item 5 cites attribute.txt line 7 for ESRCH; it is at line 6.**
9. **T-R005:59 "writes `.dev/last-build-inputs` at three sites" then "dev.sh:41
   and its two sibling sites".** The three *write* sites are `dev.sh:106,200,265`;
   `:41` is the definition. Say which is which so an implementor deletes the
   right lines.
10. **T-R004/T-R005 understate the `110-bloat-delta.sh` coupled change.** The
    entries name `check-todo.py:228` and `plant.sh:51`; `gate.yml:131` runs it
    and `plant.sh:191` reads `CEILING_BYTES` from it. Add them.
11. **Neither entry specifies the `check-todo.py` row grammar or exit-code
    contract**, both of which T-R004 calls the thing the six binaries share.
    Without the fields and status values, wave 4 cannot start.
12. **T-R005:113 defers `153`'s fate** ("decide its fate before porting"). The
    entry cannot be finished as written; make the decision or open it.
13. **T-R005:119 says `60-interposer-libc.sh` must stay out of `members` but
    does not say where it lives** — a `[[bin]]` in the `podbox-gate` member
    would itself be in members. Resolve the location.
14. **T-R005:104 leaves `145`'s "live-payload rows" unitemised**, and
    T-R005's non-trivial git binaries (`395`, `399`) have no interface. An
    implementor has to invent them.
15. **`PLAN.md:167` names `refactor/06-entries/record-fixes.md`, which does not
    exist.** Either create it or correct PLAN's file list.
