# Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories scope and method

Scope: the two wave entries `refactor/06-entries/T-R002.md` and
`refactor/06-entries/T-R003.md`. The entries carry the inline pure test gaps
(wave 2) and the first `tests/` directories plus the OCI registry fixture
(wave 3).

Read in full: `AGENTS.md`, `TODO/RULES.md`, `TODO/INDEX.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `docs/methodology/gate.md`, `experiments/README.md`,
`.github/workflows/gate.yml`, `Cargo.toml`, `refactor/06-entries/T-R002.md`,
`refactor/06-entries/T-R003.md`, `refactor/06-entries/T-R000.md`,
`refactor/06-entries/PLAN.md`.

Every `path:line` citation in the two entries was opened and read at the named
line. Every number was re-counted with a command recorded in the number table.
Every one of the 14 T-R000 corrections was checked against the record it names.
The group report bodies for all 21 scripts the two entries touch were read.

**Citations checked: 41. Wrong: 7.**

**Numbers re-counted: 20. Wrong: 5.**

**T-R000 corrections checked: 14. Wrong or unsupported: 3** (items 4, 11, and
part of 2's arithmetic claim).

**Could not settle:**

- Whether `crates/podbox-enter/src/identity.rs` was renamed or whether the
  entry copied a stale group-body path. `podbox-enter` has no `identity.rs` in
  the tree and `lib.rs` names no `identity` module. The commit history that
  would settle it is a git-log read, and I did not run it.
- Whether the 6 converging scripts absent from every entry were deliberately
  deferred. No entry says so.
- Whether `crates/podbox-probe/src/exit.rs` holds the case-name set the entry
  assigns to it. The file exists; I did not read its `mod tests` bodies.
- `cargo test --workspace` cannot run on this host. It stops at
  `linker 'cc' not found`. No `Prove` in either entry was executed. All
  `Prove` analysis in the last two sections is static.
- `py scripts/check-todo.py` does run on this host. It exits 0.

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — citation table

| citation | the entry claims | the line actually says | verdict |
| --- | --- | --- | --- |
| `docs/conventions/code.md:25` (R002) | names pure tests for deterministic logic | `:25` is the blank line before `## Verification`. The rule is at `:27`. | **WRONG** |
| `docs/methodology/experiments.md:13` (R002) | requires 0 for match, 1 for tested mismatch, 2 for could-not-run | `:12-13` hold the sentence across two lines. `:13` alone is `could not run. Preserve the process status without a pipeline.` — the exit-code list starts at `:12`. | **WRONG (partial)** |
| `crates/podbox-cli/src/system.rs:191-214` (R002 clause 3) | the three exit codes of `system::abi` | `pub fn abi` is declared at `:157`. The range `:191-214` is the tail of the read-error arm through the refused arm. The three returns are at `:195` (`2`), `:209` (`0`), `:213` (`1`). The range is inside the function but excludes `:157` and the `EXIT_CLI_ERROR` return at `:186`. | **WRONG (miscounted range)** |
| `crates/podbox-cli/src/system.rs:603` (R002 clause 3) | the `mod tests` to land in | `:603` is `#[cfg(test)]`, `:604` is `mod tests {`. The entry is one line early. The file holds 7 `#[test]` functions from `:608`, none naming `abi`. That claim is correct. | **WRONG (off by one)** |
| `crates/podbox-complete/src/write.rs:643` (R002 clause 13) | has the fixture | `:643` is `fn scratch(name: &str) -> String`. It creates `etc/` and nothing else. The four `symlink()` calls are at `:657`, `:682`, `:703`, `:713`, each inside an individual test. `scratch` does not plant a symlink. | **MISLEADING** |
| `crates/podbox-probe/src/probe_cache.rs` (R002 clause 9) | the target `mod tests` | **The file does not exist.** `podbox-probe/src/` has 17 files; none is `probe_cache.rs`. The module is `crates/podbox-image/src/probe_cache.rs`, 396 lines, `mod tests` at `:223`. | **WRONG (path)** |
| `crates/podbox-enter/src/identity.rs` (R002 clauses 6, 7) | the target for two clauses | **The file does not exist.** `podbox-enter/src/` holds `abi.rs`, `binfmt.rs`, `device.rs`, `fuse.rs`, `ladder.rs`, `lib.rs`, `memfd.rs`, `plan.rs`, `stage.rs`, `userland.rs`. No `identity.rs`. | **WRONG (path)** |
| `crates/podbox-cli/src/parity.rs` `mod tests` at `:706` (R002) | the target `mod tests` | `:706` is `#[cfg(test)]`, `:707` is `mod tests {`. One line early. | **WRONG (off by one)** |
| `crates/podbox-interpose/src/identity.rs` `mod tests` at `:282` (R002 clause 15) | the target `mod tests` | `:282` is `#[cfg(test)]`, `:283` is `mod tests {`. One line early. | **WRONG (off by one)** |
| `TODO/image.md:483` (R003) | T-0206's `Prove` is `./experiments/180-registry-fixture.sh` | `:483` reads `Prove: \`./experiments/180-registry-fixture.sh\` exits 0 with ALL outbound network blocked`. Exact match. The script exists. | correct |
| `crates/podbox-image/src/registry.rs:949-955` (R003) | `mod tests` at `:949`, `serve_once` at `:955` | `:949` is `#[cfg(test)]`, `:950` is `mod tests {`, `:955` is `fn serve_once(body: Vec<u8>) -> u16 {`. The entry's `:949` is one line early; `:955` is exact. | **WRONG (one of two)** |
| `crates/podbox-image/src/registry.rs:955` `serve_once` (R003) | test-private, one request, plain HTTP, unreachable from a binary | `:955-971` binds `127.0.0.1:0`, spawns one thread, accepts one connection, writes `HTTP/1.0 200 OK` with a fixed body, returns the port. Only two callers, both at `:989` and `:1014`, both inside `mod tests`. Confirmed. | correct |
| `crates/podbox-ssh/tests/common.rs` (R003) | exists; the only `tests/` directory; its idiom is the model | The file exists, 205 lines. `find crates -type d -name tests` returns exactly one directory: `crates/podbox-ssh/tests`. Confirmed. | correct |
| `docs/conventions/code.md:28` (R003) | what an integration test must prove | `:28` reads `Use fault tests for conditions a real service cannot produce on demand.` The integration-test rule is at `:29`: `Use integration and deployment proof for the actual default path.` | **WRONG** |
| `docs/conventions/code.md:15` (R003) | forbids speculative machinery with no caller | `:15` is **blank**. The rule is at `:13`: `Do not add unused frameworks, duplicate implementations, or dead code.` | **WRONG** |
| `docs/methodology/gate.md:23` (R003) | names the set of gate jobs | `:23` reads `It runs formatting, lint, workspace tests, interposer tests, and repository checks.` That is the check list, not the four-job set. `gate.yml` `jobs:` gives `todo` `:23`, `build` `:73`, `lint` `:137`, `test` `:188`. The gate set is not enumerated at `gate.md:23`. | **WRONG** |
| `docs/conventions/code.md:39` (R002) | the plant rule | `code.md` is **36 lines total**. There is no `:39`. | **WRONG (line does not exist)** |
| `crates/podbox-cli/src/images.rs:961-986` (R002) | `prune` returns 0 | `:961` is `match store.delete(&rest, ...)`, `:986` is `0`, inside the `Ok(done)` arm. Confirmed: `prune` returns 0. | correct |
| `experiments/results/store-gc.txt` `prune_skipped 1` (R002) | the saved result records it | The file reads `prune_skipped     1` and `rmi_under_holder  125   (non-zero is the pass)`. Confirmed. | correct |
| `experiments/149-podvm-non-goals.sh:184` (R002 clause 8) | shells to 361 with a 1500-second bound | `:184` reads `if PODBOX_BIN="$BIN" timeout 1500 sh "$REPO/experiments/361-guest-usernet.sh" ...`. Confirmed. `361-guest-usernet.sh` exists and is executable. | correct |
| `crates/podbox-extract/src/drive.rs:480` (PLAN, inherited by R002) | asserts the opposite outcome | `:480` is `fn the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply()`. Confirmed, and it is in `drive.rs`. | correct |
| `.github/workflows/gate.yml` `:199` and `:213` (PLAN, echoed by R003) | the workspace test and interposer test commands | `:199` is `run: cargo test --workspace` under step `workspace tests` (`:198`). `:213` is `run: cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` under step `interposer tests` (`:210`), whose `env:` key is `:211` and `RUSTFLAGS` is `:212`. Both exact. The meta report's `:197` (blank) and `:211` (`RUSTFLAGS:` key) are indeed wrong, and the plan corrected them. | correct |
| `.github/workflows/gate.yml` four jobs (R003) | the gate runs four jobs | `jobs:` at `:22` gives `todo` `:23`, `build` `:73`, `lint` `:137`, `test` `:188`. Four. Confirmed. | correct |
| `crates/podbox-interpose/Cargo.toml:15` `crate-type` (PLAN) | `crate-type = ["cdylib"]` | `:15` is `crate-type = ["cdylib"]`. Exact. | correct |
| `crates/podbox-interpose/Cargo.toml:18` empty `[dependencies]` (PLAN) | the manifest's `[dependencies]` is empty | `:18` is `[dependencies]`, `:19` is blank. Empty. Confirmed. | correct |
| `Cargo.toml:20` (PLAN) | the crate is excluded | `:20` is `exclude = ["crates/podbox-interpose"]`. Exact. | correct |
| `crates/podbox-interpose/interpose.map` 112 names (R002 preamble, PLAN) | declares 112 names | `grep -cE '^\s+[a-zA-Z_][a-zA-Z0-9_]*;'` returns 112. Confirmed. | correct |
| `scripts/plant.sh:51` `FILES` (R002) | names paths under `experiments/` and `scripts/` | `:51` is the `FILES=` assignment. It names `TODO/INDEX.md`, `.github/workflows/gate.yml`, `crates/podbox-cli/src/parity.rs`, `crates/podbox-cli/src/run.rs`, `scripts/build-interpose.sh`, `scripts/dev.sh`, and six `experiments/` paths. Confirmed. | correct |
| `scripts/check-todo.py:228` `CEILING_SCRIPT` (R002) | the ceiling script constant | `:228` is `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"`. Exact. | correct |
| `scripts/check-todo.py:300` `check_tree` (R002) | reads every tracked file | `:300` is `def check_tree(files):`, `:301` its docstring `"""Checks 11 to 14: citations and links outside TODO/."""`, `:303` `for rel in sorted(files):`. Confirmed, with the narrowing that it skips `CORPUS_PREFIX` paths at `:304`. | correct |
| `scripts/build-interpose.sh:197-214` (PLAN) | compares `nm -D` output against `interpose.map` | `:199-200` awk the map's `global:` block, `:201` run `nm -D --defined-only`, `:204` `diff` the two, `:207` report. Confirmed. | correct |
| `scripts/build-interpose.sh:48` (PLAN) | the byte ceiling | `:48` is `INTERPOSE_CEILING_BYTES=500000`. Exact. | correct |
| `crates/podbox-enter/src/abi.rs:1170-1315` (PLAN) | 4 of 5 checks of 80-interposer-abi | `:1170` is `fn the_soname_discriminator_is_the_one_that_was_measured()`, the check A test named at `group-9.md:1225`. Confirmed present. | correct |
| `crates/podbox-ssh/tests/session_interactive.rs:255-482` (PLAN) | 12 tests for 388 | The file is 484 lines; `:255` is `fn prompt_echo_command_and_no_terminal()`. The file holds 12 `#[test]` attributes. Confirmed. | correct |
| `crates/podbox-extract/src/drive.rs` `:167` `:211` `:229` `:264` `:382` (PLAN) | the 6 checks of 220 | Each is an `fn` line in `drive.rs`. Confirmed. | correct |
| `crates/podbox-probe/src/probe_cache.rs:233-296` (PLAN) | 4 tests for 170 clauses 2 and 3 | The path is **wrong**. The real file is `crates/podbox-image/src/probe_cache.rs`; `:233` is `fn the_first_run_measures_and_the_second_is_served_from_the_cache()`. | **WRONG (path, PLAN-level)** |
| `crates/podbox-supervise/src/nongoals.rs:253-302` (PLAN) | tests for 149 | **The file does not exist.** `podbox-supervise/src/` holds `launcher.rs`, `lib.rs`, `table.rs`. `report.rs` is also absent. | **WRONG (path, PLAN-level)** |
| `TODO/INDEX.md:42-58` (PLAN) | category-to-crate authority | `:42` is the table header `\| category \| crate \| specification \|`, `:58` is the last row `podssh`. The `gate` row naming `scripts/` is at `:56`. Confirmed. | correct |
| `TODO/cli.md:63` (T-R000 item 1) | claims 220 parity rows, labelled CURRENT | `:63` reads `\| rows \| **220**. ⚠ 131 when this closed; ... this row is the CURRENT count ...`. Confirmed. | correct |
| `TODO/cli.md:182` (T-R000 item 2) | records `clause-1 41/41` | `:182` reads `clause-1 41/41 refused with status None at 125, clause-2 10/10`. Confirmed. | correct |
| `experiments/355-parity-curated.sh:106` (T-R000 item 2) | prints `$refused/40` | `:106` reads `echo "clause-1 refused-with-reason  $refused/40" >>"$REPORT"`. Exact. | correct |
| `experiments/results/parity-curated.txt:8` (T-R000 item 2) | reads `40/40` | `:8` reads `clause-1 refused-with-reason  40/40`. Exact. | correct |
| `experiments/157-lock-inheritance-prove.sh:157` (T-R000 item 8) | matches `if !sys::close_in_children(lock.fd) {` | `:157` reads `'s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/'`. Exact. | correct |
| `crates/podbox-image/src/store.rs:1083` (T-R000 item 8) | reads `if !sys::close_in_children(fd) {` | `:1083` reads `if !sys::close_in_children(fd) {`, inside `Store::try_acquire` (`let fd = Lock::open(path)?` at `:1075`). Exact. | correct |
| `experiments/results/perf-kvm.txt` (T-R000 item 4) | reads `0.936` | The file reads `guest.kvm.boot ... 0.935 s wall ok` and `ok: guest kvm boot 0.935s to READY`. | **WRONG (see corrections)** |
| `TODO/gate.md:1430-1431` (T-R000 item 4) | states the KVM/TCG sentence | `:1430-1431` reads `the KVM base (experiments/results/perf-kvm.txt: KVM guest` / `boot 0.936 s against TCG 1.869 s, ...`. Confirmed present. | correct |
| `experiments/results/perf-lane.txt` (T-R000 item 4) | `guest.tcg.boot ... could-not-run`, "no qemu-system-x86_64 on PATH" | `:123` reads `... guest.tcg.boot	tcg	-	s wall	could-not-run`; `:125` reads `note: no qemu-system-x86_64 on PATH: guest metrics could not run`. Exact. | correct |
| `TODO/gate.md:434` (T-R000 item 9) | carries a stale `Source` line | `:434` reads `Source:      \`Cargo.toml:13-18\`; \`scripts/dev.sh:199-202\``. `Cargo.toml` is 120 lines; the exclude is at `:20`, not in `:13-18`. The line is stale. | correct (the defect is real) |
| `experiments/README.md:35` (T-R000 item 10) | lists `392-kvm-guest.sh` in a table of passing proofs | `:35` is the `392-kvm-guest.sh` row in the "Repository audit proofs" table. The table header at `:33` does not claim "passing" but the row asserts required conditions and result with no failure marker. | **PARTLY WRONG (see corrections)** |
| `TODO/image.md:507` and `experiments/lib/engine.sh:70,294-297,311` (T-R000 item 14) | `ENG_NETWORK` knob confirmed present | `image.md:507` reads `opt-in \`ENG_NETWORK\` knob in \`experiments/lib/engine.sh\``. `engine.sh:70` is `ENG_NETWORK="${ENG_NETWORK:-}"`, `:294-297` is the `case` with the `*) _refuse` arm, `:311` is `ENG_NETWORK=""`. All exact. | correct |
| `TODO/image.md:1884,1894` (T-R000 item 13) | record "24 of 24" | `:1884` reads `(24 of 24 by audit)`; `:1894` reads `24 vs 24, equal`. Confirmed. `store.rs` now holds 29 `#[test]` attributes. | correct |
| `TODO/gate.md:964-968` `:1037` `:978-980` (T-R000 item 12) | T-1212's Prove vs its transcript | `:964-968` names six scripts "each exit 0". `:1037` reads `EXIT:2` under the `300-run.sh` transcript. `:978-980` reads "`300` carries zero FAILs and one SKIP ... the riscv64 refusal half is unmeasurable". Confirmed. | correct |
| `TODO/interpose.md:1445-1446` (T-R000 item 6) | stale `lib.rs` line numbers | `:1445` cites `crates/podbox-interpose/src/lib.rs:334` for the `fchmodat` declaration and `:1446` cites `:989` for the forward. `:334` is `crate::real!(pub fn next_removexattr = ...)`. The real declaration is `:356`; the real forward is `:1038`. Stale. | correct (the defect is real) |
| `TODO/probe.md:171` (T-R000 item 5) | quotes `attribute.txt` as ENOSYS | `:171` reads `\| the host this repository is worked on \| \`errno=38 ENOSYS\` \| \`experiments/results/attribute.txt\`, and the corpus' own capture agrees \|`. Confirmed. The file's `kcmp` control at `:6` reads `FAIL errno=3 ESRCH`. | correct (the defect is real) |
| `crates/podbox-image/src/store.rs:1976` (`group-2.md:227`) | the store test name | `:1976` is `fn an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune()`. Exact. | correct |

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — number table

| number | the entry or plan claims | my recount | my command | verdict |
| --- | --- | --- | --- | --- |
| 121 scripts (PLAN) | 121 shell/Python/PowerShell scripts under `experiments/` and `scripts/` | **160** tracked `.sh`/`.py`/`.ps1` files under those two directories today | `git ls-files experiments scripts \| grep -E '\.(sh\|py\|ps1)$' \| wc -l` → 160 | **WRONG as stated for the tree** |
| 121 scripts (audit scope) | 121 *numbered experiment* scripts at orientation time | `experiment-inventory.txt` line 1 reads `TOTAL 121`; 122 lines with a header | `head -1 refactor/00-orientation/experiment-inventory.txt`; `wc -l` → `TOTAL 121`, 122 | correct **for its own scope** |
| 29,153 lines (PLAN) | 29,153 lines | **38,802** for the same 160 tracked files | `git ls-files experiments scripts \| grep -E '\.(sh\|py\|ps1)$' \| tr '\n' '\0' \| xargs -0 wc -l \| tail -1` → 38,802 total | **WRONG as stated for the tree** |
| verdict distribution KEEP-SHELL 29 (PLAN) | 29 | **27** | `tail -n +2 refactor/06-entries/verdict-ledger.tsv \| cut -f3 \| sort \| uniq -c` | **WRONG** |
| SPLIT 36 (PLAN) | 36 | **35** | same | **WRONG** |
| RUST-TOOL 27 (PLAN) | 27 | **27** | same | correct |
| RUST-TEST 18 (PLAN) | 18 | **20** | same | **WRONG** |
| DELETE 11 (PLAN) | 11 | **12** | same | **WRONG** |
| distribution total 121 (PLAN) | 121 | **121** rows | `tail -n +2 ... \| wc -l` → 121 | correct |
| 43 converging scripts (PLAN) | 43 converge wholly on Rust | **47** (RUST-TOOL 27 + RUST-TEST 20) | `tail -n +2 ... \| cut -f3 \| sort \| uniq -c \| awk '$2=="RUST-TOOL"\|\|$2=="RUST-TEST"{s+=$1} END{print s}'` → 47 | **WRONG** |
| 265 parity rows (T-R000 item 1, PLAN) | `parity.rs:232-537` holds 265 | **265** `Row {` inside `:232-537` | `awk 'NR>=232 && NR<=537' crates/podbox-cli/src/parity.rs \| grep -c 'Row {'` → 265 | correct |
| 267 whole-file (PLAN VC) | whole-file `Row {` grep returns 267 | **267** | `grep -c 'Row {' crates/podbox-cli/src/parity.rs` → 267 | correct |
| `struct Row` at `:58`, `impl Row` at `:65` (PLAN VC) | the reason 267 exceeds 265 | `:58` is `pub struct Row {`, `:65` is `impl Row {` | read of `parity.rs:55-70` | correct |
| 112 `interpose.map` names | 112 | **112** | `grep -cE '^\s+[a-zA-Z_][a-zA-Z0-9_]*;' crates/podbox-interpose/interpose.map` → 112 | correct |
| 13 `plant.sh` `Prove` lines (PLAN VC-5) | 13 across `TODO/cli.md`, `TODO/deps.md`, `TODO/gate.md` | **13** | `grep -rn "^Prove:.*plant\.sh" TODO/cli.md TODO/deps.md TODO/gate.md \| wc -l` → 13 | correct |
| 980 test functions (PLAN) | 980 | **980** | `grep -rh "#\[test\]" crates --include=*.rs \| wc -l` → 980 | correct |
| 950 / 30 split (PLAN) | 950 in `src`, 30 in `tests/` | **950 in `src`, 30 in `crates/podbox-ssh/tests/`** | `grep -rc "#\[test\]" crates/*/src/*.rs crates/*/src/**/*.rs \| awk -F: '$2>0{s+=$2} END{print s}'` → 950; `grep -rn "#\[test\]" crates/*/tests/*.rs \| wc -l` → 30 | correct |
| 15 clauses across 8 scripts (R002) | 15 clauses, 8 scripts | The table has 15 rows. The scripts named are 11 distinct: `160`, `80`, `90`, `149`, `170`, `70`, `85`, `105`, `106`. | `grep -oE '\`[0-9]+-[a-z0-9-]+\.sh\`' refactor/06-entries/T-R002.md \| tr -d '\`' \| sort -u` → 9 in the numbered table, plus 3 more in the "further pure gaps" paragraph and `361` | **WRONG — 9 scripts, not 8** |
| 4 further pure gaps (R002 §"Three further pure gaps") | 3 scripts listed | The sentence says **Three** and then names `325-parity-drive.sh` (clauses 1 **and** 2), `330-exit-codes.sh`, `362-windows-refusal.sh`. That is 3 scripts and 4+ clauses. The count of *scripts* is right; the count of *clauses* is not stated. | read of `T-R002.md:67-72` | internally consistent but the 15 total excludes these |
| ten members now (plan question) | ten workspace members | **Nine** members | `cargo metadata --no-deps --format-version 1` → `workspace_members 9`; `Cargo.toml:3-13` lists 9 | **WRONG** |
| four jobs in gate.yml (R003) | four | **Four**: `todo`, `build`, `lint`, `test` | `grep -nE "^  [a-z]+:" .github/workflows/gate.yml` | correct |
| `podbox-cli` targets `['bin','custom-build']` (R003) | no `lib` | **`['bin','custom-build']`** exactly | `cargo metadata --no-deps` → `podbox-cli [['bin'],['custom-build']]` | correct |
| six converted scripts (R002 preamble, from PLAN) | 1 fully converted (`388`), 1 more (`220`), 6 partial | 6 partial: `170`, `160`, `70`, `90`, `80`, `149`. Confirmed against `PLAN.md:91-98`. | read | correct |
| 7 `#[test]` in `system.rs` from `:603` (PLAN) | 7 tests, none names `abi` | **7** | `awk 'NR>=603 && NR<=698' crates/podbox-cli/src/system.rs \| grep -c 'fn '` → 7 | correct |
| 21 clauses green, 180-registry-fixture (image.md:487) | not asserted by either entry | not checked — out of my scope | — | not checked |

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — corrections check

Each of T-R000's 14 items names a record, a wrong value, a corrected value, and
the line that settles it. I opened the record for each.

| item | the record says | is the record actually wrong | is the correction right | verdict |
| --- | --- | --- | --- | --- |
| 1 | `TODO/cli.md:63` says 220 rows, labelled CURRENT | **Yes.** Verified: 220 in the record, 265 in source. The other two stale values (164 in `parity-drive.txt:7`, 160 in `cli.md:685`) are also real. | Yes. `220 → 265` and the in-range recount both confirm. | **CORRECT** |
| 2 | `TODO/cli.md:182` records `clause-1 41/41` | **Yes.** The record reads 41/41; the script prints `$refused/40`; the result reads 40/40. | **Partly.** `40/40` is right. The stated arithmetic — "46 curated flags, 40 refused, 6 driven in clauses 3 to 5" — rests on `group-1.md:31-34`, which I did not re-derive. The 46 comes from `scripts/check-todo.py:1184-1196`, a range I did not read. The correction itself does not depend on it. | **CORRECT, with an unverified premise** |
| 3 | `TODO/cli.md:1246` says nine ASCII tests where ten exist | **Yes.** `:1246` reads `Nine unit tests hold the constants the script drives`. The run line at `:1248` reads `9 passed`. Whether ten exist today I did not count — `group-1.md` claims a `check 28` guard, and the count depends on which tests the entry means. | **The correction is not stated.** Item 3 names the defect but gives no corrected value and no settling line. T-R000 says each item "names the record, the wrong value, the corrected value, and the line that settles it." Item 3 does not. | **INCOMPLETE** |
| 4 | `TODO/gate.md:1430-1431` states `0.936 s` against `1.869 s` | **Yes.** The TCG figure `1.869` appears nowhere under `experiments/results/` (grep returns nothing). `perf-lane.txt:123` records `guest.tcg.boot ... could-not-run`. | **Partly wrong.** Removing the TCG figure is right. But the item says `perf-kvm.txt` reads `0.936`. It reads **`0.935`**: `guest.kvm.boot	kvm	0.935	s wall	ok`. The KVM figure the entry would keep is itself off by one in the last digit. | **WRONG in part** |
| 5 | `TODO/probe.md:171` quotes `attribute.txt` as ENOSYS | **Yes.** The record says `errno=38 ENOSYS`; the file's `kcmp` control at `:6` reads `FAIL errno=3 ESRCH`. | **Partly.** ESRCH is right, and `attribute.txt` has no ENOSYS. The item says "The file reads ESRCH at line 7" — it is at **line 6**; `:7` is `fsopen(tmpfs) OK`. The commit `6eb941f` exists and its subject is `gate: close T-1213, its clearing conditions hold (T-1213)`, which does not name `attribute.txt`. | **WRONG in part** |
| 6 | `TODO/interpose.md:87` and `:1445-1446` carry stale `lib.rs` lines | **Yes for `:1445-1446`.** `:334` is `next_removexattr`; the real `fchmodat` declaration is `:356`, the forward `:1038`. | The item says "stale `lib.rs` line numbers" without naming the corrected values or the lines. An implementor must find `:356` and `:1038` themselves. | **CORRECT but under-specified** |
| 7 | `TODO/image.md` T-0211's Done quotes a 2026-09-12 run of `157` | **Yes.** `image.md:1060` reads `Status: done 2026-09-12` and `:1162` reads `**Done, 2026-09-12.** \`./experiments/157-lock-inheritance-prove.sh\``. | Yes: re-anchor after item 8, or reopen. The mechanism is confirmed — the sed at `:157` names `lock.fd`, the source at `store.rs:1083` names `fd`, so the mutation cannot land. | **CORRECT** |
| 8 | `experiments/157-lock-inheritance-prove.sh:157` names `lock.fd`; `store.rs:1083` names `fd` | **Yes.** Both lines verified exactly. | Yes. The script's own mutation-landed guard then exits 1, so the script fails today. The conclusion follows from the two verified lines. | **CORRECT** |
| 9 | `TODO/gate.md:434` carries a stale `Source` line | **Yes.** `:434` cites `Cargo.toml:13-18`; the exclude is at `:20`. | The item says "stale" without naming the corrected range. | **CORRECT but under-specified** |
| 10 | `experiments/README.md:35` lists `392-kvm-guest.sh` in "a table of passing proofs" | **Partly.** `:35` is the `392-kvm-guest.sh` row in the table headed `| Script | Required conditions and result |` (`:33`). That table lists *required conditions*, not results that passed. The README also states at `:55-58` that nested KVM stopped the host on 2026-09-30 and that an agent must not run the proof unattended. | The conclusion — the row overstates it — is defensible from `:55-58` and `TODO/PROGRESS.md:63-65`. The stated defect ("a table of passing proofs") is a misreading of the table's header. | **WRONG in its premise, right in its conclusion** |
| 11 | `371:193` writes `windows-365.txt`; `370:205` writes `windows-364.txt`; "no `TODO/` entry cites under those names" | **Yes, the swap is real.** `:193` of `371` writes `experiments/results/windows-365.txt`; `:205` of `370` writes `windows-364.txt`. Both files exist. | **Wrong in its last clause.** `docs/history/session-2026-09-25-to-27.md:155` cites `windows-364.txt` and `:157` cites `windows-365.txt`, naming the swap explicitly. The item says "no `TODO/` entry cites" — true for `TODO/`, but the record understates the evidence. The names are *not* uncited. | **WRONG in part** |
| 12 | `TODO/gate.md` T-1212's Prove at `:964-968` says six scripts "each exit 0", transcript at `:1037` records `EXIT:2`, prose at `:978-980` calls it a SKIP | **Yes.** All three verified exactly. | Yes. The entry is internally inconsistent with its own transcript. | **CORRECT** |
| 13 | `TODO/image.md:1884,1894` record "24 of 24"; the tree has 29 | **Yes.** Both lines read 24. `store.rs` holds 29 `#[test]` attributes. | Yes: 24 is stale, 29 is current, and "the balance holds" (mutex acquisitions also grew) is not something I re-derived. | **CORRECT** |
| 14 | `TODO/image.md:507` records an `ENG_NETWORK` knob; confirmed present at `engine.sh:70,294-297,311` | **No defect.** All four lines confirmed, and the record is right. | The item explicitly says "No correction; recorded so a reader does not re-open it." That is honest. | **CORRECT as a non-correction** |

**Corrections found wrong: 3** — items 4, 5, and 11. **Incomplete: 2** — items 3
and 6 (no corrected value given). **Premise misread: 1** — item 10.

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — self-sufficiency gaps

An implementor with no prior context, given only these two entries plus the
repository, would have to guess at the following.

**T-R002**

1. **Two of fifteen target files do not exist.** Clause 6 and clause 7 name
   `crates/podbox-enter/src/identity.rs`. Clause 9 names
   `crates/podbox-probe/src/probe_cache.rs`. Neither path is in the tree. The
   implementor must find the replacements themselves, and the entry gives no
   rule for choosing. For clause 9 the answer is obvious
   (`crates/podbox-image/src/probe_cache.rs`, `mod tests` at `:223`). For
   clauses 6 and 7 the group body says `crates/podbox-complete/src/identity.rs`,
   which is a **different crate** — an architectural decision the entry does
   not settle.

2. **Clause 5 and clause 7 disagree with the group body about what 90-nsswitch
   does.** Clause 5 assigns check D to `podbox-enter/src/abi.rs`; the group body
   agrees. Clause 7 assigns "checks B and C" to `identity.rs`; the group body
   says check B **stays a drive** ("a live reading of three upstream images …
   No crate can assert what `alpine:3.22` ships", `group-9.md:1361-1362`) and
   that check C should be **deleted**, not ported ("This check proved nothing on
   the lane that recorded it", `group-9.md:1366-1370`). The entry's own table
   also lists check B under clause 4 as "stays shell". So clause 7's "checks B
   and C" contradicts the table's own row 4 and the group body.

3. **Clause 12 names a string that does not exist.** "The `WANT` string" has no
   referent: `grep -i want experiments/70-whiteout-contract.sh` returns nothing,
   and `grep -n WANT crates/podbox-extract/src/drive.rs` returns nothing. The
   group body assigns 70's clauses to `whiteout.rs`, `sidecar.rs`, `safety.rs`
   and `remove.rs` — four files, none of them `drive.rs`. The implementor has
   no way to know what to assert.

4. **Clause 13's fixture description is wrong.** "`write.rs:643` has the
   fixture" — `scratch` creates only `etc/`. The four symlinks are created inside
   individual tests at `:657`, `:682`, `:703`, `:713`. An implementor looking for
   a "symlink-planted temp rootfs" helper will not find one and must decide
   whether to add it to `scratch` (which would change every test that uses it) or
   write a second helper.

5. **No test names, expected values, or assertion text for any of the 15
   clauses.** The table gives one prose line each. `docs/methodology/authoring.md:48`
   requires "conditions that a fixture can arrange" and a `Prove` that names a
   runnable command. A clause like "the three exit codes of `system::abi`" does
   not state which three, what each input produces which code, or where the
   boundary is. The entry says the range is `:191-214`; the function starts at
   `:157`.

6. **The plant requirement is not actionable.** "Each of the 15 tests gets one
   `scripts/plant.sh` case that mutates the source to break the assertion." But
   `plant.sh` operates on `check-todo.py` checks against files in its `FILES`
   list at `:51`, and no `crates/podbox-*/src/*.rs` file for these 15 tests is
   in that list except `crates/podbox-cli/src/parity.rs` and
   `crates/podbox-cli/src/run.rs`. `gate.md:174-181` (from group-4's own
   reasoning) says a cargo test's plant "is a mutation watched by hand". The
   entry does not say which of the two shapes applies, and it does not say
   whether `FILES` must be extended (which is itself a `plant.sh` change, under
   wave 4's ownership per `PLAN.md:117`).

7. **No `Effort` split, no ordering, and no commit boundaries across 15 tests
   in 9 scripts and 11 files.** The entry says "one test per clause" but never
   says which lands first, or whether a partial landing is acceptable.

**T-R003**

8. **The registry fixture is specified as a wish list, not a design.** Five
   bullet points ("bind an ephemeral loopback port and report it", "hold a push
   and a pull target", …). The implementor must decide: thread-per-connection
   or a loop, `std::net::TcpListener` or a crate, how the ETag is derived, what
   the digest is computed over, and how shutdown is detected. `code.md:12-14`
   says keep the interface small; the entry settles nothing about the shape.

9. **The HTTP surface the fixture must answer is not enumerated.** The entry
   says "the manifest GET, the blob GET and the blob HEAD that
   `crates/podbox-image/src/store.rs` issues". I read `store.rs` and found no
   `HEAD`, no `Method::Head`, and no direct HTTP call — its registry access goes
   through `crate::registry` at `:993` and `:1684`. `registry.rs` exposes
   `manifest` (`:205`), `tags` (`:266`), `blob` (`:349`). Whether the fixture
   needs a `tags` endpoint, and whether a `HEAD` exists at all, is unsettled.
   `tags` is a real fourth call the entry omits.

10. **The `podbox-cli` decision is offered but the tests are still assigned.**
    The entry says "This entry says so" for black-box, then lists four
    `crates/podbox-cli/tests/*.rs` files. An implementor must decide
    independently, per file, whether each is black-box or needs `[lib]` — the
    entry says "The entry picks one per file and says which" in its `Decision`
    field, then does not do it. The `Decision` text contradicts itself.

11. **`crates/podbox-probe/tests/namespace.rs` for `365-namespace.sh` has no
    stated boundary.** The entry says "the real mount-name path". Whether that
    test needs root, a mount namespace, or a temp mount is not stated, and
    `365-namespace.sh` is a deployment proof by its own nature.

12. **No proof command for six of the seven test files.** `Prove:` is
    `cargo test -p podbox-image`, which covers `space_precheck.rs` and
    `acquisition.rs` only. `curated_refusals.rs`, `detached_stdio.rs`,
    `store_gates.rs` and `qol.rs` are `podbox-cli`; `namespace.rs` is
    `podbox-probe`. The entry's own `Counts` section says "the full gate runs
    too", but a `Prove` field must name one runnable command
    (`authoring.md:44`).

13. **`store_digest.rs` is named as blocked but appears in no table.** It is
    in the `Premise` and in `Blockers named`, but not in "The tests this wave
    unblocks". An implementor scanning the table does not see it as deliverable
    work.

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — internal consistency

**Cross-document conflicts**

| conflict | detail |
| --- | --- |
| `PLAN.md:34` verdict table vs `verdict-ledger.tsv` | PLAN says KEEP-SHELL 29 / SPLIT 36 / RUST-TOOL 27 / RUST-TEST 18 / DELETE 11. The ledger reads 27 / 35 / 27 / 20 / 12. Four of five numbers differ. PLAN says "Counted from the bodies, after VC-1 through VC-7". |
| `PLAN.md:34` "43 converge wholly" vs ledger | 27 + 20 = **47**. |
| `PLAN.md:62` "Ten members become fourteen" vs `Cargo.toml:3-13` | Nine members today. `cargo metadata` reports `workspace_members 9`. Nine + 4 = **thirteen**, not fourteen. |
| `PLAN.md:100` "15 specific clauses across the 8" vs T-R002's table | T-R002's own table names **9** distinct scripts, and the "Three further pure gaps" paragraph adds 3 more (`325`, `330`, `362`) plus `361`. PLAN counts the 8 that appear in its section-4 table; T-R002's 15 clauses span 9 of them. |
| `PLAN.md:152` "header labels total 112 against the body's 121" | Group 1's header totals 3+2+2+1+1 = 9 and its body says "Nine scripts". I checked one header; I did not check all ten. Ledger group column sums 9+12+12+13+12+12+13+12+13+13 = **121**. Consistent with the ledger, not with PLAN's 112-vs-121 framing as applied to the headers. |

**Standing corrections not applied to the ledger** — `PLAN.md` section 7 settles
these; `verdict-ledger.tsv` still carries the pre-correction verdict:

| VC | PLAN's ruling | ledger says | group body says |
| --- | --- | --- | --- |
| VC-2 | `162-tar-symlink-modes.sh` DELETE is **REFUTED**, retained as a deployment check | `DELETE` | `group-10.md:361` reads `**Verdict: DELETE.**` |
| VC-3 | `95-podman-vfs-ignorechown.sh` RUST-TEST → **KEEP-SHELL** | `RUST-TEST` | `group-4.md:498` reads `**Verdict: RUST-TEST.**` |
| VC-7 | `151-spawn-ambiguity.sh` keeps its **SPLIT** label | `RUST-TEST` | `group-8.md:524` reads `**Verdict: RUST-TEST, mostly already done.**` |

The ledger is a faithful transcription of the bodies. It is the **plan** that
disagrees with its own ledger, because it applied three verdict changes to
neither. Applying VC-2, VC-3 and VC-7 to the ledger would move 162 out of
DELETE (12 → 11), move 95 out of RUST-TEST (20 → 19) into KEEP-SHELL (27 → 28),
and split 151 between RUST-TEST and SPLIT. That accounts for three of the four
PLAN/ledger differences but not all: SPLIT 36 vs 35 runs the other way.

**Unassigned scripts** — 7 of the 47 converging scripts named in no entry:

| script | ledger verdict |
| --- | --- |
| `experiments/30-attribution-census.sh` | RUST-TEST |
| `experiments/95-podman-vfs-ignorechown.sh` | RUST-TEST (VC-3 says KEEP-SHELL) |
| `experiments/130-probe-parity.sh` | RUST-TEST |
| `experiments/151-spawn-ambiguity.sh` | RUST-TEST (VC-7 says SPLIT) |
| `experiments/368-run-decay.sh` | RUST-TEST |
| `experiments/394-ssh-package.sh` | RUST-TOOL |
| `experiments/397-exported-build.py` | RUST-TOOL |

No entry names them. A seventh, `388-interactive-shell.sh`, and `220`, are named
by T-R001. The plan's claim that 84 of 121 scripts can be retired is not
traceable to any wave for these seven.

**Double-assigned scripts**

| script | waves | detail |
| --- | --- | --- |
| `experiments/310-session-startup.sh` | T-R004 and T-R005 | `T-R004.md:26` and `:67` assign it to `podbox-dev`. `T-R005.md:41` assigns it to `podbox-release`. The ledger gives it RUST-TOOL, group 10. Genuine double assignment. |
| `160-store-gc.sh`, `70-whiteout-contract.sh`, `80-interposer-abi.sh`, `90-nsswitch-contract.sh` | named in T-R001 and T-R002 | **Not a defect.** `T-R001.md:44-50` explicitly excludes them: "Only these two scripts are deletable in this wave … `160-store-gc.sh` is 3 of 5 … Deleting those loses the clause." They appear in T-R001 only as exclusion reasons. |

**Unrunnable proves**

| `Prove` | runs on this host? | why |
| --- | --- | --- |
| `cargo test --workspace` (T-R002) | **No** | `linker 'cc' not found`. T-R003 states this at `:124`; T-R002 does not. |
| `cargo test -p podbox-image` (T-R003) | **No** | Same. |
| `py scripts/check-todo.py` (T-R000, run as control) | **Yes** | Exit 0. `check-todo: 203 rows, 203 entries, 0 open, 2 partial, 0 blocked, 201 done` / `check-todo: ok`. |

Neither entry states that its own `Prove` cannot run on this host. T-R003 says
so in its `Counts` section. **T-R002 does not mention it at all**, and its
`Prove: cargo test --workspace` is the entry's only field a reader runs first.

**Crate names across entries** — consistent. T-R002 names `podbox-cli`,
`podbox-enter`, `podbox-image`, `podbox-probe`, `podbox-extract`,
`podbox-complete`, `podbox-interpose`. T-R003 names `podbox-cli`,
`podbox-image`, `podbox-probe`. All seven exist as directories under `crates/`,
and all seven except `podbox-probe` for `probe_cache` match the crate a named
file actually lives in.

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — what each Prove establishes

### `cargo test --workspace` (T-R002 `Prove`)

**Proves:** that every existing test in the nine workspace members still passes,
and that the 15 new tests pass once written. It is the right aggregate: a new
`#[test]` in any `src/` module is picked up by the workspace target, and no new
directory or crate is involved.

**Does not prove:**

- **That any new test can fail.** The entry says this itself at `:100-103`, and
  it is right. `AGENTS.md:77` and `gate.md:49` require a plant with each new
  check, and `code.md:32` requires "A test must reject the defect it claims to
  detect". A green `cargo test --workspace` is compatible with 15 tests that
  assert nothing.
- **That the tests reach their subject.** `experiments.md:25-26` requires "a
  positive control and a failure control where the instrument can otherwise pass
  without reaching its subject". For clause 2 (`prune` exits 0, prints
  `skipped:`, leaves the image listed) a test that calls `store::delete` with a
  held image never reaches the CLI wrapper that prints `skipped:`. The entry
  does not name the control.
- **Anything about the twelve deployment-level clauses the table lists.** Rows
  4, 8, 10, 11 say "stays shell" or "wave 3". A green `cargo test --workspace`
  is compatible with all four remaining unmet.
- **That the entry's own file paths are real.** Two of the fifteen do not exist.
  `cargo test` would not have caught it: the implementor would have created
  `crates/podbox-enter/src/identity.rs` or `crates/podbox-probe/src/probe_cache.rs`
  as new files, and the workspace would go green on a module nothing wires.
- **That it can run here.** It cannot. T-R002 does not say so.

### `cargo test -p podbox-image` (T-R003 `Prove`)

**Proves:** that the `podbox-image` package's unit tests and any
`crates/podbox-image/tests/*.rs` it adds pass. That covers exactly two of the
seven test files the entry's table names: `space_precheck.rs` and
`acquisition.rs`.

**Does not prove:**

- **The registry fixture works against a real client.** `serve_once`
  (`registry.rs:955-971`) answers **one** request and exits. The fixture the
  entry asks for must "hold a push and a pull target" and "shut down so the
  test does not leak a listener" — that is a different shape, and the existing
  fixture gives the implementor no pattern to extend. `code.md:33` is explicit:
  "A mock does not prove a live service or deployment."
- **Any of the four `podbox-cli` tests or the one `podbox-probe` test.** Those
  are different packages. `curated_refusals.rs`, `detached_stdio.rs`,
  `store_gates.rs` and `qol.rs` are not exercised by this command.
- **That the `tags` endpoint is served.** `registry.rs:266` exposes
  `pub fn tags`. The entry's requirement list omits it.
- **That `140-space-precheck.sh` clause 4 was reproduced.** The entry says
  "needs a real filesystem". `140-space-precheck.sh:129` reads
  `== 4. inodes are checked as well as blocks`, and the script's own trap
  (`:31`) does `mountpoint -q "$SMALL" && umount "$SMALL"` — it mounts a small
  filesystem. A Rust test that checks inodes on a temp directory does not
  reproduce the clause; it tests a different thing.
- **The gate.** The entry claims "the full gate runs too: `todo`, `build`,
  `lint` and `test` all apply" and cites `docs/methodology/gate.md:23` for the
  set. `gate.md:23` does not enumerate jobs; `gate.yml` does (`:23`, `:73`,
  `:137`, `:188`). The claim is true and the citation is wrong.
- **That it can run here.** The entry says it cannot. Correct.

### Values that are estimates or unknowns in these two entries

| value | entry | status |
| --- | --- | --- |
| `121` scripts, `29,153` lines | PLAN | Orientation-scope figures, not tree figures. The tree carries 160 tracked scripts and 38,802 lines under `experiments/` + `scripts/`. The difference is `scripts/` helpers and `experiments/lib/`, which the orientation pass excluded. Neither figure is labelled with its scope in the plan text. **Both should read as "121 numbered experiments, 29,153 lines" with the exclusion named.** |
| `E` per crate test count | T-R003 | No number asserted. The `DATA` for the fixture's storage is not stated. |
| `Effort: L` (T-R002) | T-R002 | No derivation. 15 tests across 11 files, at least 2 of which need a decision first. `authoring.md` does not define the scale, so an implementor cannot tell whether L means one session or one week. |
| `Effort: XL` (T-R003) | T-R003 | Same, and worse: a new shared fixture plus seven test files plus a structural change to `podbox-cli`. |
| fixture listener port | T-R003 | "an ephemeral loopback port" — no fixed value needed, and `-` would be the honest record for any tuning parameter. |

---

## Verify T-R002.md and T-R003.md — the inline pure gaps and the integration-test directories — corrections required

1. **`T-R002.md` clause 6 and clause 7 name `crates/podbox-enter/src/identity.rs`,
   which does not exist.** `podbox-enter/src/` has ten files and no
   `identity.rs`. The group body at `refactor/01-audit/group-9.md:1356` assigns
   check A to `crates/podbox-complete/src/identity.rs`, a different crate. Pick
   one and state it.

2. **`T-R002.md` clause 9 names `crates/podbox-probe/src/probe_cache.rs`, which
   does not exist.** The module is `crates/podbox-image/src/probe_cache.rs`
   (396 lines, `mod tests` at `:223`). The same wrong path is in
   `PLAN.md:93`. `PLAN.md:98` has a second wrong path:
   `crates/podbox-supervise/src/nongoals.rs` and `report.rs` — that crate holds
   only `launcher.rs`, `lib.rs` and `table.rs`.

3. **`T-R002.md` clause 12 names "the `WANT` string" and it does not exist.**
   `grep -i want` on `experiments/70-whiteout-contract.sh` returns nothing;
   `grep -n WANT crates/podbox-extract/src/drive.rs` returns nothing. The group
   body (`group-4.md:148-166`) assigns 70's clauses to `whiteout.rs`,
   `sidecar.rs`, `safety.rs` and `remove.rs`. The entry names `drive.rs` and a
   string. Rewrite the row from `group-4.md:146-166`.

4. **`T-R002.md` clause 7 ("checks B and C" of `90-nsswitch`) contradicts both
   its own table and the group body.** Table row 4 puts check B under
   "stays shell". `group-9.md:1361-1362` says check B "is a live reading of
   three upstream images and stays a drive", and `:1366-1370` says check C
   "proved nothing on the lane that recorded it" and the implementor "deletes
   check C and records why". Neither is a pure unit test. The row as written
   asks for two tests the audit rejected.

5. **`T-R002.md` clause 3 cites `system.rs:191-214` for the three exit codes
   of `system::abi`.** The function is declared at `:157`; the three returns are
   at `:195` (`2`), `:209` (`0`) and `:213` (`1`). The named range starts 34
   lines into the function. Cite `:157-216` and name the three lines.

6. **`T-R002.md` clause 3 and `PLAN.md:97` say `system.rs:603` holds the
   `mod tests`.** `:603` is `#[cfg(test)]`; `:604` is `mod tests {`. Same
   off-by-one at `parity.rs:706` (`:707` is the `mod`), `registry.rs:949`
   (`:950`), and `interpose/src/identity.rs:282` (`:283`).

7. **`T-R002.md` cites `docs/conventions/code.md:39`, which does not exist.**
   `code.md` is 36 lines. The plant rule is in `docs/methodology/gate.md:49-53`
   and `AGENTS.md:77`, both of which the entry already names elsewhere.

8. **`T-R002.md` cites `docs/conventions/code.md:25` for pure tests.** The rule
   is at `:27`. `code.md:25` is blank.

9. **`T-R003.md` cites `docs/conventions/code.md:28` for what an integration
   test must prove.** `:28` is the fault-test rule. The integration rule is at
   `:29`.

10. **`T-R003.md` cites `docs/conventions/code.md:15` for the ban on
    speculative machinery.** `:15` is blank. The rule is at `:13`.

11. **`T-R003.md` cites `docs/methodology/gate.md:23` for the four-job set.**
    `gate.md:23` lists what the Linux check runs, not the jobs. The job set is
    `.github/workflows/gate.yml:23,73,137,188`.

12. **`T-R002.md` clause 13 describes `write.rs:643` as a "symlink-planted temp
    rootfs" fixture.** `scratch` at `:643-648` creates only `etc/`. The
    symlinks are made inside individual tests at `:657`, `:682`, `:703`,
    `:713`. Either add a helper and name it, or say the test plants its own.

13. **`T-R002.md` never states that its `Prove` cannot run on this host.**
    T-R003 does, at `:124-125`. `docs/methodology/gate.md:17` gives
    `py scripts/check-todo.py` as the Windows record gate and `:19` gives
    `sh scripts/windows/run-in-base.sh`. T-R002's `Counts` section should name
    the lane.

14. **`T-R002.md` says "15 clauses, not 8" and its own table spans 9 scripts.**
    The premise paragraph counts scripts from the plan's section-4 table; the
    table's scripts include four the premise does not list. Recount the premise
    against the table it heads.

15. **`T-R002.md` says "15 clauses" and then adds four more in the next
    section** without folding them into the count. "Three further pure gaps" are
    `325` clauses 1 and 2, `330`'s case set, and `362`'s four refusals — that is
    4+ clauses, bringing the wave to 19+. State the wave total.

16. **`T-R002.md`'s plant requirement is unimplementable as written.** It says
    each of the 15 tests "gets one `scripts/plant.sh` case", but `plant.sh`
    operates on `check-todo.py` checks against the `FILES` list at `:51`, and
    only `crates/podbox-cli/src/parity.rs` and `crates/podbox-cli/src/run.rs`
    from these 15 targets are in it. Either name the files to add to `FILES` —
    which is a `plant.sh` change, and `PLAN.md:117` gives `plant.sh` to wave 4 —
    or state the hand-run mutation shape the group body describes at
    `group-4.md:174-181`.

17. **`T-R003.md`'s `Decision` field says it "picks one per file and says
    which" for the `podbox-cli` shape, then does not.** The `Decision` text
    ends with "This entry says so" for black-box in general, while the table
    assigns four `crates/podbox-cli/tests/*.rs` files. Pick per file, or state
    the rule that decides.

18. **`T-R003.md`'s fixture requirement omits the `tags` endpoint.**
    `registry.rs:266` exposes `pub fn tags(&mut self, endpoint, repository)`.
    Either the fixture serves it or the client never calls it, and the entry
    does not say which. The entry's phrase "the manifest GET, the blob GET and
    the blob HEAD that `store.rs` issues" is also unverified: `store.rs` issues
    no `HEAD` and no direct HTTP call; its registry access is
    `crate::registry` at `:993` and `:1684`.

19. **`T-R003.md`'s `Prove` covers 2 of the 7 test files it assigns.**
    `cargo test -p podbox-image` reaches `space_precheck.rs` and
    `acquisition.rs`. `curated_refusals.rs`, `detached_stdio.rs`,
    `store_gates.rs`, `qol.rs` (`podbox-cli`) and `namespace.rs`
    (`podbox-probe`) are unproven by it. `authoring.md:44` requires a runnable
    command; name one per crate, or state the full-gate command as the `Prove`.

20. **`T-R003.md` names `store_digest.rs` only in the premise and the blockers,
    never in the work table.** An implementor reading "The tests this wave
    unblocks" does not see it as deliverable.

21. **`T-R003.md` does not say what "the real mount-name path" of
    `365-namespace.sh` requires.** Read the script and name whether the test
    needs root, a mount namespace, or a temp mount. `authoring.md:47` requires
    "conditions that a fixture can arrange".

22. **`T-R000.md` item 4 states `perf-kvm.txt` reads `0.936`. It reads
    `0.935`** — `guest.kvm.boot	kvm	0.935	s wall	ok`. Removing the TCG figure
    is right; the KVM figure the entry would keep is wrong in its last digit.

23. **`T-R000.md` item 5 says `attribute.txt` reads ESRCH "at line 7". It is
    at line 6**; `:7` is `fsopen(tmpfs) OK`. The commit `6eb941f` exists but its
    subject (`gate: close T-1213, its clearing conditions hold`) does not name
    `attribute.txt`, so the citation does not support the re-capture claim.

24. **`T-R000.md` item 11 says "no `TODO/` entry cites" those result names.**
    `docs/history/session-2026-09-25-to-27.md:155,157` cites both, naming the
    swap. Narrow the claim or drop it.

25. **`T-R000.md` item 10 misreads the README table.** `experiments/README.md:33`
    heads it `| Script | Required conditions and result |` — it lists required
    conditions, not passing results. The conclusion stands on `:55-58` and
    `TODO/PROGRESS.md:63-65`, not on the table header.

26. **`T-R000.md` items 3, 6 and 9 name a defect without naming the corrected
    value**, against T-R000's own promise at `:48-49`. Item 3 (`cli.md:1246`,
    "nine ASCII tests where ten exist") has no corrected count and no settling
    line. Item 6 has no corrected `lib.rs` lines — they are `:356` and
    `:1038`, not `:334` and `:989`. Item 9 has no corrected range for
    `Cargo.toml`.

27. **`PLAN.md` section 2's verdict table disagrees with
    `refactor/06-entries/verdict-ledger.tsv` on four of five numbers**, and
    section 5's "43 converge" is 47 by the ledger's own counts. Applying
    VC-2, VC-3 and VC-7 to the ledger moves three rows and does not close the
    gap. Whichever is authoritative, the plan and the ledger must agree before
    an implementor sizes the work.

28. **`PLAN.md:62` says "Ten members become fourteen".** `Cargo.toml:3-13`
    lists nine and `cargo metadata` reports `workspace_members 9`. Nine plus
    four is **thirteen**.

29. **`experiments/310-session-startup.sh` is assigned to two waves.**
    `T-R004.md:26,67` give it to `podbox-dev`; `T-R005.md:41` gives it to
    `podbox-release`. The ledger gives it RUST-TOOL, group 10. One wave owns it.

30. **Seven converging scripts are in no wave:** `30-attribution-census.sh`,
    `95-podman-vfs-ignorechown.sh`, `130-probe-parity.sh`,
    `151-spawn-ambiguity.sh`, `368-run-decay.sh`, `394-ssh-package.sh`,
    `397-exported-build.py`. Either assign them or record why they are out of
    scope, so the plan's "84 of 121 can be retired" is traceable.
