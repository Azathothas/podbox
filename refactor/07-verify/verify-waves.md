# Verify PLAN.md sections 4, 5 and 6 scope and method

## What I read

Required reading, in full: `AGENTS.md`, `TODO/RULES.md`, `TODO/INDEX.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `docs/methodology/gate.md`,
`experiments/README.md`, `.github/workflows/gate.yml`, `Cargo.toml`.

In scope: `refactor/06-entries/PLAN.md` sections 4, 5 and 6, and
`T-R000.md` .. `T-R005.md` and `verdict-ledger.tsv`, which carry the citations
and numbers those sections depend on.

Also read as the artefacts under test: all ten `refactor/01-audit/group-*.md`
bodies, `refactor/02-peer-review-1/round-1.md`,
`refactor/03-peer-review-2/round-2.md`, `refactor/04-meta/meta-1.md`,
`refactor/05-meta-review/meta-review-1.md`, and the source files each citation
names.

## What I counted

- **Citations opened and read: 96.** Every `path:line` and `T-NNNN` in PLAN.md
  sections 4, 5, 6, in the six entries, and in the ledger's 121 `report_line`
  values. 121 of 121 ledger citations were resolved mechanically to the exact
  line, then every mismatch was opened and read.
- **Numbers re-counted: 22**, each with its own command, listed in the number
  table.
- **Corrections checked: 14** of 14, each against the record it names.
- **Ledger verdicts re-derived from the group bodies: 121 of 121.**

## What I could not settle

1. Whether `crates/podbox-image/src/store.rs:1083` or `:1108` is the line an
   implementor should cite. Both are `if !sys::close_in_children(fd) {` inside
   `Store::try_acquire`, which starts at `:1074`. I did not settle which one the
   re-anchored sed should target. Both are inside the function, so both satisfy
   the entry's claim as written.
2. Whether the `80-interposer-abi.sh` checks A to D are "four of five" covered by
   `abi.rs:1170-1315`. The range holds 7 tests. Check A maps to the test at
   `:1170`; checks B, C and D are `LD_PRELOAD` and live-loader arms in the
   script, and `abi.rs:11` records that its four facts are measured by the
   script. The mapping is a judgement the plan asserts and I did not refute.
3. Whether `experiments/384-windows-lane-v6.sh` and the other 71 scripts named in
   neither PLAN.md nor any entry are in scope for a later wave. The plan does
   not say. This is a finding, not a blocked check.
4. `T-0001`..`T-0007` in the `05-meta-review` correction column are unverified
   for content. They are outside sections 4, 5, 6.

## Host limits

`cargo test --workspace` fails on this host at `linker 'cc' not found`. No
`Prove` that needs a build was executed. `py scripts/check-todo.py` was executed
and exits 0.

---

# Verify PLAN.md sections 4, 5 and 6 citation table

## Section 4 — work already done in Rust

| citation | the document claims | what the line says | verdict |
| --- | --- | --- | --- |
| `crates/podbox-ssh/tests/session_interactive.rs:255-482` | 12 tests, fully converted | file is 484 lines; `#[test]` at 254, 275, 287, 303, 322, 350, 373, 388, 409, 430, 449, 471 — **12**; `grep -c '#\[test\]'` returns 12 | **TRUE** |
| `experiments/388-interactive-shell.sh` has 11 clauses | 12 tests against 11 clauses | `grep -oE 'clause [0-9]+' \| sort -uV` returns clause 1 through clause 11, no more | **TRUE** |
| `TODO/podssh.md:79` | live `Prove` naming the script | `Prove: cargo test -p podbox-ssh exits 0; sh experiments/388-interactive-shell.sh verifies…` | **TRUE** |
| `TODO/podssh.md:126` | live `Prove` naming the script | `Prove: cargo test --workspace exits 0; sh experiments/389…390…391…` — **names `389`, `390`, `391`, NOT `388`** | **FALSE.** The entry would repoint a `Prove` that does not name the script. `388` is cited at `podssh.md:79` only. |
| `crates/podbox-extract/src/drive.rs` at `:167, :211, :229, :264, :382` | 6 checks, all 6 converted | test fns begin at exactly 167, 211, 229, 264, 382 | **TRUE** |
| `drive.rs:221, :236, :253` | filesystem checks | 221 is the second half of `a_dotdot_path_is_refused`; 236 the tail of `an_absolute_path_is_refused`; 253 the tail of `a_refusal_fails_the_extraction_rather_than_skipping_the_entry` — all three are the `assert!` that nothing landed outside | **TRUE** |
| `crates/podbox-extract/src/lib.rs:40` | `#[cfg(test)] mod drive;` | `:39` is `#[cfg(test)]`, `:40` is `mod drive;` | **TRUE** |
| `crates/podbox-probe/src/probe_cache.rs:233-296` | clauses 2 and 3 done, 4 tests | **the file does not exist.** `probe_cache.rs` is at `crates/podbox-image/src/probe_cache.rs` (396 lines). At that path `:233` and `:247` are test fns, `:268` and `:296` are test fns — 4 tests | **FALSE on the crate.** `podbox-probe` has no `probe_cache.rs`; `podbox-image/src/lib.rs:36` declares `pub mod probe_cache;`. |
| `crates/podbox-image/src/contain.rs:118` | is clause 5 | `:117` is `#[test]`, `:118` is `fn a_symlink_pointing_out_of_the_store_is_refused()` | **TRUE** |
| `experiments/results/store-gc.txt` records `rmi_under_holder 125` and `prune_skipped 1` | both readings | `rmi_under_holder  125`, `prune_skipped     1` | **TRUE** |
| `crates/podbox-cli/src/images.rs:961-986` returns 0 from `prune` | prune returns 0 | `:961` is `match store.delete(...)`, `:986` is `0`, `:989` is the closing brace | **TRUE** |
| `crates/podbox-extract/src/drive.rs:480` | asserts the OPPOSITE outcome on a crafted layer | `:480` is `fn the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply()` | **MISLEADING.** The test asserts gid is **not** 42, i.e. that the kernel did **not** apply the ownership. It does not touch a whiteout, and `70-whiteout-contract.sh` check B is about `alpine` `etc/shadow` gid 42. The test is about the sidecar, not about check B's outcome. Calling it "the opposite outcome" conflates two subjects. |
| `crates/podbox-enter/src/identity.rs:336` | drives the editor, not the NSS dispatcher | **the file does not exist.** `podbox-enter/src/` has no `identity.rs`. The only `identity.rs:336` in the tree is `crates/podbox-complete/src/identity.rs:336`, `every_measured_passwd_shape_ends_up_naming_files_first()`, whose doc comment at `:333` says "driven through the editor" | **FALSE on the crate.** The claim about the editor is TRUE; the crate is `podbox-complete`, not `podbox-enter`. |
| `crates/podbox-enter/src/abi.rs:1170-1315` | 4 of 5 checks | 7 test fns in the range (1170, 1199, 1220, 1250, 1274, 1293, 1315) | **TRUE as a range.** See "could not settle" #2 for the mapping. |
| `crates/podbox-cli/src/system.rs:603` | holds 7 tests, none names `abi` | `:603` is `#[cfg(test)]`, `:604` is `mod tests`; the 7 test fns are at 608, 619, 637, 647, 658, 676, 691; `grep -c '#\[test\]'` returns 7; none contains `abi` | **TRUE** |
| `crates/podbox-cli/src/system.rs:191-214` | the three exit codes of `system::abi` | `:191` `(Err(e), _) =>` … `:195` `return 2;` … `:209` `0` … `:213` `1`; `:191-214` holds all three returns | **TRUE** |
| `crates/podbox-supervise/src/nongoals.rs:253-302` | tests for `149` | **the file does not exist.** `nongoals.rs` is at `crates/podbox-probe/src/nongoals.rs`. At that path `:253` through `:302` are 5 test fns | **FALSE on the crate.** |
| `crates/podbox-supervise/src/report.rs:1069, :1088, :1105-1107` | three more test sites | **the file does not exist under `podbox-supervise`.** `podbox-supervise/src/` is `launcher.rs`, `lib.rs`, `table.rs`. `report.rs` is at `crates/podbox-probe/src/report.rs`; `:1069` and `:1088` are test fns, `:1105-1107` is an `assert!` block inside the `:1088` test | **FALSE on the crate.** |
| `149` clause 6 shells to `361` with a 1500-second bound | no Rust arm | `experiments/149-podvm-non-goals.sh:184`: `timeout 1500 sh "$REPO/experiments/361-guest-usernet.sh"` | **TRUE** |
| "**15 specific clauses across the 8 are asserted by no Rust test today**" | 15 | T-R002's table lists 15 rows. Counted: 15 | **CONSISTENT** with T-R002. See the number table. |

## Section 4 — 265 parity rows (PLAN section 7, relied on by section 4's frame)

| citation | claim | what the line says | verdict |
| --- | --- | --- | --- |
| `crates/podbox-cli/src/parity.rs:232-537` | 265 rows | **TRUE.** `sed -n '232,537p' \| grep -c 'Row {'` returns **265**. `:232` is `pub const TABLE: &[Row] = &[`, `:537` is the closing `];` | **TRUE** |
| 267 is the whole-file `Row {` grep | the over-count | `grep -c 'Row {'` on the whole file returns **267** | **TRUE** |
| `struct Row` at `:58` | the two extra hits | `:58` is `pub struct Row {` | **TRUE** |
| `impl Row` at `:65` | the second extra hit | `:65` is `impl Row {` | **TRUE** |
| "the tree carries four values for one fact" | 265, 220, 164, 160 | see the number table | **TRUE** |

## Section 5 — the six waves

| citation | the document claims | what the line says | verdict |
| --- | --- | --- | --- |
| `.github/workflows/gate.yml:199` | `cargo test --workspace` | **TRUE** | **TRUE** |
| `.github/workflows/gate.yml:213` | the interposer test command | **TRUE** — `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` | **TRUE** |
| "`:197` is blank" (the meta report's error) | — | `:197` is blank | **TRUE** — the entry's correction of the meta report is right |
| "`:211` is the `env:` key" (the meta report's error) | — | `:211` is `env:` | **TRUE** — the entry's correction is right |
| `experiments/157-lock-inheritance-prove.sh:157` | the sed names `lock.fd` | **TRUE** — `157: 's/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/'` | **TRUE** |
| `crates/podbox-image/src/store.rs:1083` | reads `if !sys::close_in_children(fd) {` inside `try_acquire` | **TRUE** — `:1074` is `fn try_acquire`, `:1083` is the guard. `Store::hold` is at `:527` and does **not** contain the guard | **TRUE** |
| `scripts/check-todo.py:300` | `check_tree` reads every tracked file | **TRUE** — `:300` is `def check_tree(files):`; the docstring at `:301` says "Checks 11 to 14: citations and links outside TODO/" and the loop at `:303` iterates `sorted(files)` | **TRUE** |
| `scripts/check-todo.py:228` | `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` | **TRUE** — exact match | **TRUE** |
| `scripts/plant.sh:51` | `FILES` names the same path | **TRUE** — `FILES=` at `:51` lists `experiments/110-bloat-delta.sh` | **TRUE** |
| `Cargo.toml:20` | `podbox-interpose` is excluded | **TRUE** — `exclude = ["crates/podbox-interpose"]` | **TRUE** |
| `crates/podbox-interpose/Cargo.toml:15` | `crate-type = ["cdylib"]` | **TRUE** — `:15` is `crate-type = ["cdylib"]` | **TRUE** |
| `crates/podbox-interpose/Cargo.toml:18` | `[dependencies]` is empty | **TRUE** — `:18` is `[dependencies]`, `:19` is blank | **TRUE** |
| `gate.yml:99` | the interposer build step | **TRUE** — `- name: interposer, one object per libc` / `run: ./scripts/build-interpose.sh` | **TRUE** |
| `scripts/build-interpose.sh:48` | the byte ceiling | **TRUE** — `INTERPOSE_CEILING_BYTES=500000` | **TRUE** |
| `scripts/build-interpose.sh:197-214` | the `nm -D` comparison against `interpose.map`, both targets | **TRUE** — `:197-200` build the declared list, `:201-213` the exported list and the `diff` | **TRUE** |
| `interpose.map` declares 112 names | counted with the script's own `awk` at `:199-200` | **TRUE** — running that exact awk over `crates/podbox-interpose/interpose.map` returns **112** (the file is 130 lines, the rest is header and `local:` names) | **TRUE** |
| `gate.yml:36-56`, `:99`, `:199`, `:213` in T-R004's Source | four jobs | **TRUE** — jobs at `:23` (todo), `:73` (build), `:137` (lint), `:188` (test) | **TRUE** |
| `gate.yml:37` | `py scripts/check-todo.py` | **TRUE** — `run: ./scripts/check-todo.py` under `- name: check-todo` at `:36` | **TRUE** |
| `gate.yml:42`, `:56`, `:59`, `:71` (todo job steps) | the todo job's steps | `:42` is `run: |` of "todo-count is idempotent", `:56` is `run: ./scripts/plant.sh`, `:59` is `run: |` of "maintained repository checks", `:71` is the licence `sha256sum` | **TRUE** |
| `gate.yml:102`, `:107`, `:131` (build job steps) | the build job's steps | `:102` `cargo build --release --target x86_64-unknown-linux-musl`, `:107` `run: |` (no PT_INTERP), `:131` `./experiments/110-bloat-delta.sh ci` | **TRUE** |
| `gate.yml:154`, `:159`, `:164`, `:171`, `:176`, `:186` (lint job steps) | the lint job's steps | all six are the `run:` or `run: |` line of the named step | **TRUE** |
| `gate.yml:202`, `:205`, `:208` (test job steps) | the test job's steps | `:202` `python3 experiments/393-build-freshness.py`, `:205` `python3 experiments/398-gate-diagnostics.py`, `:208` `python3 experiments/400-kvm-cleanup.py` | **TRUE** |
| `gate.yml:186` | `py_compile scripts/*.py scripts/windows/*.py experiments/39*.py experiments/400-kvm-cleanup.py` | **TRUE** — exact match, `:186` | **TRUE** |
| `gate.yml:55-56` | runs plant as the step named `every check can fail` | **TRUE** — `:55` is the `- name:`, `:56` is the `run:` | **TRUE** |
| `TODO/INDEX.md:42-58` | the category table | **TRUE** — `:42` is the header row, `:57` is `podssh`, `:58` is blank | **PARTLY.** The table ends at `:57`, not `:58`. The rows the entries use — `:52` deps, `:53` packaging, `:55` podvm, `:56` gate — are all inside the range. |
| `TODO/INDEX.md:52` | `deps` has crate cell "the whole tree" | **TRUE** | **TRUE** |
| `TODO/INDEX.md:53` | `packaging` has crate cell "the artefact" | **TRUE** | **TRUE** |
| `TODO/INDEX.md:55` | `podvm` has crate cell "the machine tier" | **TRUE** | **TRUE** |
| `TODO/INDEX.md:56` | `gate` has crate cell `scripts/` | **TRUE** | **TRUE** |
| `TODO/RULES.md` has no crate/member/workspace rule | 0 hits in 121 lines | **TRUE** — `wc -l` is 121; `grep -inc 'crate\|member\|workspac'` returns 0 | **TRUE** |
| `crates/podbox-ssh/tests/common.rs` exists | the only `tests/` directory | **TRUE** — it exists, and it is the only `tests/` under `crates/` | **TRUE** |
| `docs/methodology/gate.md:23` | names the gate job set | `:23` reads "It runs formatting, lint, workspace tests, interposer tests, and repository checks." That is the `dev.sh check` sentence, **not** a list of the four `gate.yml` jobs | **MISLEADING.** The claim is a paraphrase, not a quotation. The rule is at `gate.md:6-25`. |
| `docs/conventions/code.md:25` | pure tests for deterministic logic | **TRUE** — `:25` is `## Verification`, `:27` is "Use pure tests for deterministic logic." The rule is at `:27`, the section at `:25`. | **TRUE as a section citation** |
| `docs/conventions/code.md:39` | the plant-with-a-check rule | **the file is 36 lines.** `:39` does not exist | **FALSE.** The rule is at `docs/methodology/gate.md:47-54` ("Add the check and its plant in the same change"), which T-R002 also cites as "TODO/gate.md's header rule". |
| `docs/conventions/code.md:28` | fault tests | `:28` is "Use fault tests for conditions a real service cannot produce on demand." | **TRUE** |
| `docs/conventions/code.md:29` | integration and deployment proof | `:29` is exactly that line | **TRUE** |
| `docs/conventions/code.md:15` | forbids speculative machinery | `:15` is "Bound buffers, retries, network operations, and child waits." The forbidding line is `:13`, "Do not add unused frameworks, duplicate implementations, or dead code." | **FALSE** |
| `docs/conventions/code.md:33` | the deployment rule cited by VC-3 | `:33` is "A mock does not prove a live service or deployment." VC-3's rule is the deployment one, `:29` | **FALSE** |
| `docs/methodology/experiments.md:13` | exit 0 / 1 / 2 | `:12-13` reads "Use exit 0 for a match, 1 for a tested mismatch, and 2 when the proof could not run." | **TRUE** |
| `scripts/check-todo.py` is 1,738 lines | the largest script in the corpus | `wc -l` returns **1738**, and the assignment map lists it at 1738 | **TRUE** |
| `scripts/dev.sh:41` | writes `.dev/last-build-inputs` at three sites | `:41` is `STAMP="$STATE/last-build-inputs"` — an assignment, not a write. The three write sites are `:106`, `:200`, `:265` | **PARTLY.** The path is right; `:41` declares it and nothing reads it. |
| `TODO/packaging.md` T-1002, T-1004, T-1005 | the owning entries | all three exist in `TODO/INDEX.md:162,164,165` under `packaging` | **TRUE** |
| `TODO/complete.md` T-1324 | owns `document-state.py` | `TODO/INDEX.md:215` — yes, `complete`, done | **TRUE** |
| `TODO/packaging.md` T-1314 | owns `395` and `399` | `TODO/INDEX.md:205` — yes | **TRUE** |
| `TODO/deps.md` T-0908, T-0910 | `clap` refused; `110` ceiling | `TODO/INDEX.md:156` T-0908, `:158` T-0910, both under `deps` | **TRUE** |
| `TODO/enter.md` T-0503, `TODO/probe.md` T-0111, `TODO/image.md` T-0215, T-0209 | the single-script homes | all four exist: `INDEX.md:115`, `:73`, `:90`, `:84` | **TRUE** |
| `TODO/gate.md:1125` | calls the three-way dependency irreducible | **TRUE** — `:1125` is `Decision: The three convert as one unit. Converting 130 without 20 tests nothing, and converting 20 without 10 builds nothing.` | **TRUE** |
| `experiments/20-enter-target.sh:70`, `experiments/300-run.sh:310` | name `10` in a live path | **TRUE** — `:70` is a SKIP naming `10-build-target-image.sh`; `:310` is a `say` naming it | **TRUE** |
| `TODO/image.md:483` | T-0206's `Prove` is `./experiments/180-registry-fixture.sh` | **TRUE** | **TRUE** |
| `crates/podbox-image/src/registry.rs:955` `serve_once` | a `fn` inside `mod tests` at `:949` | **TRUE** — `:949` is `#[cfg(test)]`, `:950` is `mod tests {`, `:955` is `fn serve_once(body: Vec<u8>) -> u16` | **TRUE** |
| `TODO/INTERPOSE.md:1471` (VC-2) | names `162` in T-1311's live `Prove` | **TRUE** — `:1471` is `Prove: ./experiments/162-tar-symlink-modes.sh exits 0 on host podman` | **TRUE** |
| `experiments/386-podssh-partial.sh:74,75,77` (VC-1) | three strings `machine.rs:53` no longer has | `:74`, `:75`, `:77` assert `"no machine guest carries an SSH server"`, `"guest: no machine guest"`, `"not implemented yet"`. `machine.rs:53` reads `eprintln!("podbox machine: {other:?}: no such member\n{USAGE}")` — **none of the three strings is there** | **TRUE** (but see corrections 1 below) |
| `crates/podbox-cli/src/machine.rs:53` and the `005638d` diff | the commit removed them | `git log --oneline -1 005638d` resolves to "Dispatch and prove the remote and machine SSH verbs" | **TRUE** |
| `TODO/milestones.md:877` not `:883` (VC-1) | the retired-number reason | `:877` reads "named private input paths. Experiment 392 replaces that driver" — this is the numbering-reuse ground. `:883` reads "writes more than one pipe buffer blocks" — unrelated | **TRUE** |
| `95-podman` has no `exit 2` (VC-3) | its own exit-2 path does not exist | `:17-19` reads "Exit: 0 the combination opens the path, 1 it does not. Never 2" | **TRUE** |
| `355-parity-curated.sh:106` | prints `$refused/40` | **TRUE** | **TRUE** |
| `experiments/results/perf-kvm.txt` `0.936` | the KVM figure | **FALSE — the file says `0.935`.** `:133` reads `guest.kvm.boot kvm 0.935 s wall ok`. `grep -rn '0\.936' experiments/results/` returns nothing. | **FALSE** |
| `1.869` appears nowhere under `experiments/results/` | — | `grep -rn '1\.869' experiments/results/` returns nothing | **TRUE** |
| the lane file records `guest.tcg.boot … could-not-run` with "no qemu-system-x86_64 on PATH" | — | **`perf-kvm.txt` holds no `guest.tcg.boot` row at all.** `grep -n 'guest.tcg.boot\|qemu-system' experiments/results/perf-kvm.txt` returns nothing. `qemu  QEMU emulator version 11.1.1` is at `:12`. | **FALSE** — the quote is not in that file |
| `experiments/README.md:35` | lists `392-kvm-guest.sh` in a table of passing proofs | **TRUE** — `:35` is a row in "Repository audit proofs" | **TRUE** |
| `TODO/gate.md:434` | a stale `Source` line | `:434` is `Source: Cargo.toml:13-18; scripts/dev.sh:199-202`. `Cargo.toml:13-18` covers the `exclude` comment block, not `:20` where the exclusion is | **TRUE** |
| `experiments/371-validationos-stream.sh:193` | writes `windows-365.txt` | **TRUE** | **TRUE** |
| `experiments/370-windows-guest.sh:205` | writes `windows-364.txt` | **TRUE** | **TRUE** |
| `TODO/gate.md:964-968` (correction 12) | six scripts to "each exit 0" | **TRUE** — `:964-968` lists all six with "each exit 0" | **TRUE** |
| `TODO/gate.md:1037` | records `EXIT:2` for `300-run.sh` | **TRUE** — `:1026` is the `sh experiments/300-run.sh` invocation, `:1037` is `EXIT:2` | **TRUE** |
| `TODO/gate.md:978-980` | calls it a SKIP | **TRUE** — `:978-980` read "carries zero FAILs and one SKIP" | **TRUE** |
| `experiments/lib/engine.sh:70,294-297,311` | `ENG_NETWORK` present | **TRUE** — all four lines are `ENG_NETWORK` lines | **TRUE** |
| `scripts/check-todo.py:228` + `scripts/plant.sh:51` name the same path | a coupled change | **TRUE** | **TRUE** |
| `crates/podbox-cli/src/parity.rs` `mod tests` at `:706` (T-R002) | the `mod tests` for `325`'s clauses | **FALSE.** `mod tests` is at **`parity.rs:707`**; `:706` is the `#[cfg(test)]` attribute | **FALSE by one** |
| `crates/podbox-probe/src/exit.rs` (T-R002) | `330`'s case-name set target | exists, 3 tests | **TRUE** |
| `crates/podbox-interpose/src/identity.rs` `mod tests` at `:282` (T-R002) | the `mod tests` | `:282` is `#[cfg(test)]`, **`:283` is `mod tests {`** | **FALSE by one** |
| `crates/podbox-complete/src/write.rs:643` has the fixture | the temp-rootfs fixture | **TRUE** — `:643` is `fn scratch(name: &str) -> String` | **TRUE** |
| `crates/podbox-probe/src/lifecycle.rs`… | T-R002's `362` targets | T-R002 names `crates/podbox-cli/src/lifecycle.rs`, `crates/podbox-cli/src/windows/dos.rs`, `crates/podbox-cli/src/windows/mod.rs` — all three exist | **TRUE** |

## Section 6 — the blockers

| citation | the document claims | what the line says | verdict |
| --- | --- | --- | --- |
| `crates/podbox-image/src/registry.rs:955` / `:949` | test-private `serve_once` | **TRUE** (see above) | **TRUE** |
| `TODO/image.md:483` | T-0206's `Prove` is the shell fixture | **TRUE** | **TRUE** |
| `crates/podbox-image/tests/store_digest.rs` | blocked, no Rust fixture | the directory `crates/podbox-image/tests/` **does not exist** | **TRUE** |
| `docs/limits.md` and `TODO/PROGRESS.md` own the KVM block | — | `limits.md:58` "The fresh 2026-09-30 KVM runs failed", `:62-63` "Nested KVM in the Windows base can stop the Windows host. On 2026-09-30…"; `PROGRESS.md:77` "T-1350 and T-1112 stay partial", `:80` "An agent must not start a KVM guest without the operator present." | **TRUE** |
| T-1350, T-1112 are the KVM-bound entries | — | `TODO/INDEX.md:241` T-1350 partial, `:178` T-1112 partial | **TRUE** |
| `linker 'cc' not found` | blocks `cargo test --workspace` on this host | matches the harness note and the entries' own statement | **TRUE** |
| `sh scripts/windows/run-in-base.sh` | the Linux base lane | exists, 6,107 bytes, executable | **TRUE** |
| `podbox-cli` has no `[lib]` | blocks every `podbox-cli/tests/*.rs` | `crates/podbox-cli/Cargo.toml` has no `[lib]`; the crate's targets are `bin` plus the `build.rs` custom-build | **TRUE** |

---

# Verify PLAN.md sections 4, 5 and 6 number table

| number | the document's claim | my recount and command | verdict |
| --- | --- | --- | --- |
| 121 scripts | the corpus | `grep -E '^\s+[0-9]+\s+(experiments\|scripts)/' refactor/00-orientation/assignment-map.md \| sed -E 's/^\s+[0-9]+\s+//' \| sort -u \| wc -l` → **121**; every one exists on disk (0 missing) | **TRUE**, but only against the assignment map. A whole-tree `find experiments scripts -name '*.sh' -o -name '*.py' -o -name '*.ps1'` returns **167**. The corpus is 121 because the plan takes the map's set; `experiments/lib/`, `experiments/src/`, `scripts/common/`, `scripts/windows/` and the 12 `.ps1` files are outside it. The plan never states that boundary. |
| 29,153 lines | the corpus | `awk '{s+=$1} END{print s}'` over the map's 121 rows → **29153**; re-running `wc -l` on each of the 121 files and summing → **29153**. The whole tree's 167 scripts sum to **39,015** | **TRUE** for the map's set |
| KEEP-SHELL 29 | verdict table | ledger column 3 → **27** | **FALSE** |
| SPLIT 36 | verdict table | **35** | **FALSE** |
| RUST-TOOL 27 | verdict table | **27** | **TRUE** |
| RUST-TEST 18 | verdict table | **20** | **FALSE** |
| DELETE 11 | verdict table | **12** | **FALSE** |
| 121 total | verdict table sums | 27+35+27+20+12 = **121** | **TRUE** — the ledger's own total is 121, but four of the five row counts are wrong. Every ledger row was checked against the group body at its cited `report_line`; **121 of 121 match**. The error is in the summary table, not the ledger. |
| 43 converge wholly on Rust | 27 RUST-TOOL + 18 RUST-TEST | 27 + 20 = **47** | **FALSE**, as derived from the wrong table. The 84-retired figure (121 − 29 KEEP-SHELL − 11 DELETE) becomes 121 − 27 − 12 = **82** |
| 84 of 121 retire | 121 − 29 − 11 + 5? | with the ledger's real numbers: 121 − 27 − 12 = **82** | **FALSE** as stated |
| 265 parity rows | `parity.rs:232-537` | `sed -n '232,537p' crates/podbox-cli/src/parity.rs \| grep -c 'Row {'` → **265** | **TRUE** |
| 267 whole-file | the over-count | `grep -c 'Row {' crates/podbox-cli/src/parity.rs` → **267** | **TRUE** |
| 220, falsely labelled CURRENT | `TODO/cli.md:63` | `TODO/cli.md:63` reads "rows \| **220**. ⚠ … this row is the CURRENT count" | **TRUE** |
| 164 | `parity-drive.txt:7` | `:7` reads "== 0. the table is data: 164 rows" | **TRUE** |
| 160 | `TODO/cli.md:685` | `:683-686` reads "160 rows, 199 driven, 0 mismatches" | **TRUE** |
| 112 `interpose.map` names | build-interpose's own awk | `awk '/global:/{g=1;next} /local:/{g=0} g && /;/{gsub(/[ \t;]/,"");if($0!="")print}' crates/podbox-interpose/interpose.map \| wc -l` → **112** | **TRUE** |
| 13 `plant.sh` `Prove` lines | across `cli.md`, `deps.md`, `gate.md` | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c 'Prove'` → **13**; by file: `deps.md` 1, `gate.md` 12. **`TODO/cli.md` contributes 0** — it has one `plant.sh` hit and it is not on a `Prove` line | **FALSE on the file list.** The count is 13. The attribution to three files is wrong: it is two. |
| 980 test functions | `meta-1.md:33` | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/src crates/*/build.rs \| awk -F: '{s+=$2} END{print s}'` → **950** in `src/`+`build.rs`; `grep -rn '#\[test\]' crates/*/tests` → **30**. 950 + 30 = **980** | **TRUE** |
| 950 / 30 split | `meta-1.md:87` | **950** inline, **30** in `crates/podbox-ssh/tests/` | **TRUE** |
| 15 unwritten clauses | PLAN section 4 and T-R002 | T-R002's table has 15 numbered rows | **TRUE** — but 4 of the 15 are marked "stays shell" (4, 8, 10, 11), so 11 become Rust tests and 4 do not. The plan counts the shell-kept rows as gaps. |
| 43 converging scripts | 27 + 18 (per table) | 27 + 20 = **47** | **FALSE**, from the same error |
| 24,738-line corpus (T-R001:21) | the two deleted scripts' corpus | no count in the tree gives 24,738. `29,153 − 24,738 = 4,415`. The two scripts are 304 and 282 lines, 586 together. | **UNSOURCED** — see corrections required |
| 1,738 lines for check-todo.py | T-R004:21 | `wc -l scripts/check-todo.py` → **1738** | **TRUE** |
| 10 workspace members (T-R005:129) | "ten members become thirteen" | `tomllib.load('Cargo.toml')` → `len(workspace.members)` = **9**; `crates/` holds **10** directories, one of which (`podbox-interpose`) is in `exclude` | **FALSE.** The current member count is 9. Nine + 3 (wave 5) = 12, and 12 + 1 (wave 4's `podbox-gate`) = 13, not 14. PLAN section 3's "Ten members become fourteen" is off by one for the same reason. |
| 1,760 citations, 5 wrong (round 1) | PLAN section 1 | `refactor/02-peer-review-1/round-1.md:17,33` both state 1,760 checked, 5 wrong | **TRUE** as a restatement |
| 2 claims corrected, 3 waves rejected, 4 crates (round 5) | PLAN section 1 | `grep -c 'REJECT as a boundary' meta-review-1.md` → **3**; the endorsed list at `:613` names **4**; "CORRECTED" appears at `:143,179,246,336,699,700` — six, not two | **PARTLY.** "3 waves rejected" and "4 crates" TRUE. "2 claims corrected" FALSE: the report labels six rows CORRECTED, four in its Claim section and two in its table. |
| 6 of 10 headers disagree with their bodies (PLAN:20, T-R000:34) | header labels total 112 | read by hand: group 1 header 9 / body 9; group 2 12/12; group 3 13/13; group 4 12/12; group 5 12/12; group 6 13/13; group 7 12/12; group 8 13/13; group 9 13/13; group 10 12/12. **The ten headers and the ten bodies agree on every script count.** | **FALSE** |
| header labels total 112 | against a body total of 121 | the ten headers sum to **121** | **FALSE** |
| 7 changed-job claims in `gate.yml` | T-R004's four-job table | jobs at `:23`, `:73`, `:137`, `:188`; every step line T-R004 names resolves inside its named job | **TRUE** |
| 6 ASCII tests where 10 exist (T-R000 correction 3) | `TODO/cli.md:1246` names nine | `grep -rn 'fn .*ascii' crates/podbox-cli/src/*.rs \| wc -l` → **10** test fns across 10 files; `:1246` reads "Nine unit tests hold the constants" | **TRUE** — the record is wrong by one and the correction is right |
| "24 of 24" store tests (T-R000 correction 13) | the tree has 29 | `awk 'NR>=1246 && /#\[test\]/' store.rs \| wc -l` → **30**; `STORE_TESTS.lock()` from `:1246` → **29** | **PARTLY.** The record says 24; the tree has **30** test functions and **29** mutex acquisitions. The entry's "29" is the acquisition count, not the test count. "The balance holds" is then 30 vs 30, not 29. |
| 44 plants caught (TODO/gate.md:1859) | — | not re-checked; outside sections 4, 5, 6 | — |

---

# Verify PLAN.md sections 4, 5 and 6 corrections check

T-R000 lists 14. Each was checked against the record it names.

| # | record | is the record actually wrong? | is the corrected value right? | verdict |
| --- | --- | --- | --- | --- |
| 1 | `TODO/cli.md:63` says 220 rows, labelled CURRENT | **YES.** `265` is the count | **YES** | **CORRECT** |
| 2 | `TODO/cli.md:182` records `clause-1 41/41` | **YES.** `parity-curated.txt:8` reads `clause-1 refused-with-reason  40/40` | the entry's value **40/40** is right. Its **arithmetic is wrong**: it says "46 curated flags, 40 refused by the loop, 6 driven in clauses 3 to 5". Clauses 3 to 5 are `--env-file`, the stub trio, `--log-driver=json-file` and `create` — 7 outcomes, not 6. The reconciliation does not close. | **CORRECT on the value, WRONG on the reasoning** |
| 3 | `TODO/cli.md:1246` names nine ASCII tests | **YES.** 10 exist | **YES** — 10 | **CORRECT** |
| 4 | `TODO/gate.md:1430-1431` says "KVM guest boot 0.936 s against TCG 1.869 s" | **YES** on the TCG half: `1.869` appears nowhere under `experiments/results/`. | **NO.** The entry says `experiments/results/perf-kvm.txt` reads `0.936`. It reads **`0.935`** at line 133. The entry's own claim that the KVM figure is sound carries a wrong value. | **HALF WRONG** — remove the TCG figure, and the KVM figure is 0.935, not 0.936. The entry's supporting sentence "the lane file records `guest.tcg.boot … could-not-run` with 'no qemu-system-x86_64 on PATH'" is also not in `perf-kvm.txt`, which has no TCG boot row and reports `qemu  QEMU emulator version 11.1.1`. |
| 5 | `TODO/probe.md:171` quotes `attribute.txt` as ENOSYS | **YES.** `attribute.txt:6` reads `kcmp(-1,-1,...) [control] FAIL errno=3 ESRCH`. `:171` claims `errno=38 ENOSYS` | **YES** on the commit: `git log --follow -- experiments/results/attribute.txt` returns `6eb941f` and `b27b3d9`; `6eb941f` is the later one. **NO on the line number.** The entry says "The file reads ESRCH at line **7**." It reads ESRCH at line **6**. | **CORRECT on substance, WRONG on the line number** |
| 6 | `TODO/interpose.md:87` and `:1445-1446` carry stale `lib.rs` line numbers | **YES.** `:1445-1446` name `crates/podbox-interpose/src/lib.rs:334` and `:989` as the `fchmodat` declaration and forward. `:334` is `next_removexattr`; the `fchmodat` declaration is at **`lib.rs:356`**, the forward at **`lib.rs:1038`**. `:87` carries "17 declared, 17 exported", and `interpose.map` declares **112** today | the entry names **no corrected lines**. It says "carry stale `lib.rs` line numbers" and stops. | **DEFECT IN THE PLAN.** A correction that does not state the corrected value is not implementable. An implementor must re-derive both numbers and the `17` figure themselves. |
| 7 | T-0211's Done quotes a 2026-09-12 run of `157` | **YES** — `TODO/image.md:1162` reads "**Done, 2026-09-12.** `./experiments/157-lock-inheritance-prove.sh` with `PODBOX_PROVE_RUNS=30`". The script cannot pass today (see correction 8) | the instruction "Re-anchor it after correction 8 or reopen it" is right. It does not say **which**. | **PARTIAL** — an implementor chooses between reopening T-0211 and re-running, with no stated criterion. |
| 8 | `experiments/157-lock-inheritance-prove.sh:157` matches `if !sys::close_in_children(lock.fd) {`; `store.rs:1083` reads `if !sys::close_in_children(fd) {` inside `Store::try_acquire` | **YES, both.** `:157` is the sed; `store.rs:1083` is inside `try_acquire` (`:1074`). `Store::hold` is at `:527` and has no such guard | **YES** | **CORRECT** — with one correction to the entry's own claim: the entry says "The script's own mutation-landed guard fires correctly and then exits 1." It does not. `mutate()` at `:118-151` compares `git hash-object` before and after (`:125-127`); a sed that matches nothing leaves the hashes equal, so it prints "⛔ THE MUTATION DID NOT LAND" and sets `fail=1`. `:131` is `fail=1`, and the script ends `[ "$fail" -eq 0 ] \|\| exit 1`. The outcome is the same (exit 1) and the **reason is the mutation not landing, not the guard firing**. The entry asserts the opposite mechanism. |
| 9 | `TODO/gate.md:434` carries a stale `Source` line | **YES** — `Source: Cargo.toml:13-18; scripts/dev.sh:199-202`. The exclusion is `Cargo.toml:20` | the entry names **no corrected value** | **DEFECT IN THE PLAN**, same class as 6 |
| 10 | `experiments/README.md:35` lists `392-kvm-guest.sh` in a table of passing proofs | **YES.** `README.md:35` is a row in "Repository audit proofs"; `PROGRESS.md:63-67` records the 2026-09-30 KVM runs as failed | the entry says "The README row overstates it" and names no replacement text | **PARTIAL** — the diagnosis is right, the correction is not written |
| 11 | `371:193` writes `windows-365.txt`; `370:205` writes `windows-364.txt` | **YES, both.** Both lines confirmed. Both result files exist (706 and 2,231 bytes, dated 2026-09-27) | the entry says "Both files exist with readings no `TODO/` entry cites under those names." `grep -rn 'windows-365\|windows-364' TODO/ docs/` finds hits only in `docs/history/session-2026-09-25-to-27.md` and the `refactor/` reports. **TRUE.** | **CORRECT** — but the entry prescribes no action. Renaming a result file under `docs/methodology/experiments.md:5` ("Do not reuse a number when replacing a script") is an operator decision the entry does not make. |
| 12 | T-1212's `Prove` at `:964-968` requires six scripts to "each exit 0" but its transcript records `EXIT:2` for `300-run.sh` and the prose calls it a SKIP | **YES, all three.** `:964-968` says "each exit 0"; `:1037` is `EXIT:2` immediately after the `sh experiments/300-run.sh` at `:1026`; `:978-980` says "zero FAILs and one SKIP" | the entry does not state a corrected `Prove` | **PARTIAL** — the contradiction is found and named, the fix is not written |
| 13 | `TODO/image.md:1884,1894` record "24 of 24" store tests; the tree has 29 | **YES**, the record says 24. | **PARTLY WRONG.** The tree has **30** `#[test]` in `store.rs`'s test module and **29** `STORE_TESTS.lock()` acquisitions. The entry's 29 is the acquisition count. "The balance holds" is 30 vs 30, not 29 vs 29. Both figures change; the entry states neither. | **PARTLY WRONG** |
| 14 | `TODO/image.md:507` records an `ENG_NETWORK` knob; confirmed present at `engine.sh:70,294-297,311`; **no correction** | the record is **RIGHT** | the entry says so and records it so a reader does not re-open it | **CORRECT as a no-op.** Verified: all four lines carry `ENG_NETWORK`. |

**Corrections found wrong or incomplete: 5** (items 2, 4, 5, 6, 8 in part; plus 9, 10, 12, 13 as incomplete). Two (6 and 9) name no corrected value at all.

---

# Verify PLAN.md sections 4, 5 and 6 self-sufficiency gaps

For each entry: could an implementor with no prior context start and finish given
only this document plus the repository?

### T-R000 (wave 0)

Could start. Could not finish:

1. **Correction 6 names no corrected line.** The implementor must find the new
   `lib.rs` lines for `fchmodat` and decide whether the "17 declared, 17
   exported" figure at `interpose.md:87` is stale too. It is — the map declares
   112. Not stated.
2. **Correction 9 names no corrected `Source` line.** The implementor must
   decide whether `Cargo.toml:13-18` should become `:20`, `:15-20`, or the whole
   exclusion block.
3. **Correction 7 does not say which way to resolve T-0211** — re-anchor and
   re-run, or reopen the entry.
4. **Correction 10 does not write the replacement README row.**
5. **Correction 12 does not write the corrected `Prove` for T-1212.**
6. **The `Prove` is `py scripts/check-todo.py`.** It runs and exits 0 today. It
   proves the records agree with the tree. It cannot prove any correction is
   *right* — a wrong correction that keeps the tree self-consistent is green.

### T-R001 (wave 1)

Could start. Gaps:

1. **`TODO/podssh.md:126` does not name `388`.** Repointing it is a no-op that
   will look done. The implementor either skips it or repoints the wrong entry.
2. **"Repoint the owning `TODO/extract.md` entry's `Prove`"** — the entry names
   no line and no entry id. The implementor must search `TODO/extract.md` for
   the `220` reference themselves.
3. **`24,738-line-corpus` has no source.** No command in the tree produces it.
4. **The clause-to-test table maps 6 script checks to 9 test lines.** The
   mapping is a judgement (which test is check D). It is not stated which test
   asserts which of A to F.
5. **`Drive` vs `safety`** — the entry says the tests are in `drive.rs` and that
   the meta cited only `safety.rs`. It does not say whether `safety.rs` holds
   duplicate tests that should be removed. `ls crates/podbox-extract/src/`
   shows `safety.rs` exists.

### T-R002 (wave 2)

Could start on the pure clauses. Gaps:

1. **Three targets name a crate that does not exist** (see citation table). The
   implementor writing to `crates/podbox-probe/src/probe_cache.rs`,
   `crates/podbox-enter/src/identity.rs` and `crates/podbox-supervise/src/`
   creates a second module in the wrong crate, or a new crate.
2. **Clauses 4, 8, 10, 11 are listed in the "15 clauses" table but marked
   "stays shell" or "needs a fixture".** They are not gaps to close. The
   Problem line says "15 clauses … asserted by no Rust test today" and the
   Approach says "One test per clause". An implementor following Approach
   literally writes 15 tests, four of which have no subject.
3. **Clause 2 is a source defect, not a test gap** — the entry says so, then
   leaves the choice between "assert what the source returns" and "fix the
   source as a separate defect entry" open. Two different waves result.
4. **The two extra gaps at the end of the table** (`325` clauses 1 and 2,
   `330`'s case-name set, `362`'s three files) are described in one sentence
   with no clause detail, no assertion text and no expected value. That is
   three scripts' worth of work in one line.
5. **The `Prove` is `cargo test --workspace`, which cannot reach
   `podbox-interpose`.** The workspace has 9 members and `podbox-interpose` is
   in `exclude`. Clauses 14 and 15 land in that crate. Their tests are run by
   `gate.yml:213` and `gate.yml:164`, neither of which this entry names.

### T-R003 (wave 3)

Could start on the structural decision. Gaps:

1. **The black-box-vs-`[lib]` decision is left open.** The entry states the
   recommendation and says an implementor "may argue otherwise with the reason
   recorded". Wave 3 is XL effort and the decision governs every
   `podbox-cli/tests/*.rs` in it. No default is stated as binding.
2. **`crates/podbox-image/tests/common/registry.rs` has no interface.** The
   entry lists five capabilities in prose. An implementor must design the API.
3. **The entry lists eight scripts it unblocks, then says the wave is blocked on
   a fixture it must build first.** It does not say which of the eight land with
   the fixture and which land after.
4. **The `Prove` is `cargo test -p podbox-image`**, which proves the fixture
   and the `podbox-image` consumers. Three of the eight tests land in
   `podbox-cli/tests/` and one in `podbox-probe/tests/`. `cargo test -p
   podbox-image` does not reach them.
5. **`355-parity-curated.sh` "the surviving end-to-end half"** — no clause is
   named. The implementor must re-derive it from the script.

### T-R004 (wave 4)

Could start. Gaps:

1. **`crates/podbox-gate` does not exist and the entry does not say where its
   six binaries live.** One `[[bin]]` per binary, six `src/bin/*.rs` files, or
   one library with six thin mains? Not stated. `Cargo.toml:3-13` has no
   pattern to copy.
2. **The `podbox-smoke` bin replaces two scripts** (`nightly-smoke.sh`,
   `398-gate-diagnostics.py`) and **the `podbox-dev` bin replaces three**
   (`dev.sh`, `session-start.sh`, `310-session-startup.sh`).** How one binary
   absorbs three scripts with different argument surfaces is unstated. Which
   subcommand each becomes is an implementor's decision.
3. **"Arguments are parsed by hand"** — the entry forbids `clap` and names no
   substitute. The implementor must pick an argument-parsing shape.
4. **The 1,738-line Python port has no interim plan.** The entry says
   behaviour-preserving. It does not say whether the Python and the Rust both
   exist during the port, or which one CI runs on each day.
5. **`TODO/RULES.md#4` and `#5`** are cited as "source of record" for
   `podbox-count` and `podbox-gate` without quoting them. I did not read those
   anchors; the entry assumes they say what the implementor needs.
6. **`gate.yml:186`'s replacement glob is not written.** The entry says the glob
   "must change in the same commit" and does not say to what.

### T-R005 (wave 5)

Could start on `podbox-buildstate`. Gaps:

1. **The member arithmetic is wrong** (see number table): the tree has 9
   members, not 10, so the entry's "ten become thirteen … and fourteen after
   wave 4" is off by one. An implementor following it writes a `members` list
   with a name that does not exist.
2. **`scripts/check-state` does not exist** (cited at `:125`). The build-state
   inputs come from `scripts/build-state.py`, which the same entry replaces.
3. **`dev.sh:41` "and its two sibling sites"** — the sibling sites are the
   three `inputs_digest >"$STAMP"` writes at `dev.sh:106, :200, :265`. Not
   named. The implementor must find them, and `:41` itself is the declaration
   they must decide the fate of.
4. **`crates/podbox-release` is seven binaries covering eight subjects**, and
   `podbox-prove-t0211` covers two. The entry does not say whether each binary
   is a subcommand or a separate `[[bin]]`.
5. **`153-store-lock-race.sh` "decide its fate before porting"** — the entry
   defers the decision rather than making it. An implementor starting that row
   stops.
6. **`60-interposer-libc.sh` "must stay OUT of `members`"** — correct, but the
   entry does not say where it lives. If not a member, where does the source sit
   so `podbox-gate`'s interposer bin can read it?

---

# Verify PLAN.md sections 4, 5 and 6 internal consistency

## Cross-document conflicts

1. **`TODO/podssh.md:126` vs T-R001 and PLAN section 4.** Both say to repoint
   it. `:126` names `389`, `390`, `391`. It is not a `388` reference.
2. **The verdict counts.** PLAN section 2 and every restatement of it say
   29/36/27/18/11. The ledger holds 27/35/27/20/12. The ledger is right: all 121
   rows were checked against their group body and 121 match.
3. **The header-vs-body claim.** PLAN section 1 and T-R000's Decision both rest
   on "Six of ten headers disagree with their own bodies, and the header labels
   total 112 against the body's 121". Read by hand, all ten headers sum to 121
   and agree with the ten bodies. The claim is false and T-R000's Decision is
   built on it. T-R000's *action* (correct records) is unaffected; its
   *justification* is wrong.
4. **The wave-2 clause count.** PLAN section 4 says 15 unwritten clauses. T-R002
   lists 15, of which four keep their shell script. PLAN section 4 also says
   "15 specific clauses across the 8 are asserted by no Rust test today", which
   is true of all 15 but implies all 15 become Rust.
5. **`podbox-dev` is in two crates.** T-R004:67 puts `dev.sh` behind
   `podbox-gate`'s `podbox-dev`. T-R005:59-61 says `dev.sh:41` is a
   `podbox-buildstate` concern ("`scripts/build-state.py` replaces a stamp
   nothing reads"). PLAN section 3 puts `dev.sh` under `podbox-gate` and
   `build-state.py` under `podbox-buildstate`. T-R005's own Decision sentence
   says "`session-start.sh` and `310` join `podbox-gate`, not
   `podbox-buildstate`" — it does not say the same for `dev.sh`, which it
   discusses under the `podbox-buildstate` heading. The heading and the sentence
   disagree.
6. **`110-bloat-delta.sh` is named by two files and moves in one wave.**
   T-R004:51-55 puts the coupled path change in wave 4. T-R005:90-92 says the
   same and says "that lands in wave 4". T-R005's own crate table at `:69` puts
   `110` behind `podbox-release`, a **wave-5** crate. The path moves in wave 4
   and the script is deleted in wave 5. Between them the gate is red unless the
   path change is the last act of wave 4.
7. **Member counts.** PLAN section 3 says "Ten members become fourteen".
   T-R005:129 says "ten members become thirteen here and fourteen after wave 4".
   The tree has **9**. See the number table.
8. **`157-lock-inheritance-prove.sh` is both a record fix and a binary.**
   T-R000 correction 8 re-anchors the script's sed. T-R005 replaces the script
   with `podbox-prove-t0211`. T-R005 states the dependency. Consistent.
9. **`plant.sh` Prove count.** T-R004:82-83 says "13 `Prove:` lines across
   three `TODO/` files". The count is 13; the files are two (`deps.md` 1,
   `gate.md` 12). `TODO/cli.md` contributes none.
10. **`docs/conventions/code.md` line citations.** T-R003 uses `:28` and `:15`;
    T-R002 uses `:25` and `:39`; PLAN section 7 uses `:29` and `:33`. Of these,
    `:28`, `:25`, `:29` are right; `:15`, `:39`, `:33` are wrong (see citation
    table). The file is 36 lines, so every citation above 36 is impossible.

## Unassigned scripts

71 of the 121 scripts are named in **neither** PLAN.md nor any of the six
entries. They are not in a wave. The plan's own waves cover roughly 50 scripts
(31 in section 4, 15 in T-R002, 6 in T-R001, plus the crate subjects). The
verdict ledger assigns all 121 a verdict, but no wave claims the KEEP-SHELL
scripts that are not "already in Rust", and the DELETE scripts have no entry at
all. `experiments/10-build-target-image.sh` (DELETE), `50-interpose-tier.sh`
(DELETE), `158-interpose-embedding.sh` (DELETE), `350-tool-live.sh` (DELETE),
`354-lifecycle-same-store.sh` (DELETE), `163-ladder-drive.sh` (DELETE) and the
rest of the 12 DELETEs are in no wave. VC-1 through VC-4 discuss four of them in
PLAN section 7, but no entry carries the work.

## Double-assigned scripts

Eight scripts appear in more than one entry or in both an entry and PLAN.md.
Three of the overlaps are real conflicts, not cross-references:

- `110-bloat-delta.sh` — T-R004 (coupled path move) and T-R005 (deleted in
  wave 5). See conflict 6.
- `dev.sh` — T-R004 (podbox-gate) and T-R005 (podbox-buildstate). See conflict 5.
- `plant.sh` — T-R002 (wave-2 plants added to it), T-R004 (replaced by
  `podbox-plant`), T-R005 (its `FILES` line moved). A wave-2 plant is added to
  a file that wave 4 deletes. Ordering is stated (0,1,2,3,4,5) but the entry
  does not say what a wave-2 plant does when wave 4 lands.
- `check-todo.py` — named in all five, as a gate to run, as a subject, and as
  the file to port. Consistent.
- `157`, `180`, `300`, `build-interpose.sh` — cross-references, consistent.

## Unrunnable Proves

| entry | `Prove` | runs on this host? | why |
| --- | --- | --- | --- |
| T-R000 | `py scripts/check-todo.py` | **YES** | ran it: exit 0, `check-todo: ok` |
| T-R001 | `cargo test --workspace` | **NO** | `linker 'cc' not found` |
| T-R002 | `cargo test --workspace` | **NO** | same, and it cannot reach `podbox-interpose` in any case |
| T-R003 | `cargo test -p podbox-image` | **NO** | same |
| T-R004 | `py scripts/check-todo.py` | **YES** | ran it: exit 0. But the entry also says "all four gate jobs", which do not run here. |
| T-R005 | `cargo test --workspace` | **NO** | same |

## Crate names across entries

Consistent. `podbox-gate`, `podbox-buildstate`, `podbox-release`, `podbox-podvm`
appear with the same bin lists in PLAN section 3, T-R004 and T-R005. The six
`podbox-gate` bins, the two `podbox-buildstate` bins, the seven
`podbox-release` bins and the two `podbox-podvm` bins match. `podbox-podvm-workload`
appears in both PLAN and T-R005. No name is spelled two ways.

The one bin-name divergence: T-R004 lists `podbox-smoke` replacing
`nightly-smoke.sh` and `398-gate-diagnostics.py`; the meta-review's endorsed
list at `:618` names five bins for `podbox-gate` and does not include
`podbox-interpose-build`. PLAN and T-R004 add it. That is an addition, not a
conflict.

---

# Verify PLAN.md sections 4, 5 and 6 what each Prove establishes

Per `docs/methodology/code.md:30` ("State what each kind proves") and
`docs/methodology/experiments.md:25-26` ("Use a positive control and a failure
control where the instrument can otherwise pass without reaching its subject").

### T-R000 — `py scripts/check-todo.py`

**Proves:** the `TODO/` records are internally consistent, that every cited
path and line resolves, and that derived counts match their rows. It ran and
exited 0 on this tree.

**Does not prove:** that any of the 14 corrections is *right*. Every
correction is a judgement about what a record should say; `check-todo.py`
checks that the record and the tree agree, not that the agreement is the true
value. A correction that moved `0.936` to `0.936` stays green. This is the
entry's own `Counts` line and it is right to state it.

**A check that would pass without reaching its subject:** the entry's own
correction 14 is a no-op that is explicitly recorded as such, and the gate is
green. That is by design and is stated.

### T-R001 — `cargo test --workspace`

**Proves:** the workspace builds and its 950 inline tests plus 30 `tests/` tests
pass after two scripts are deleted and two `Prove` lines are repointed.

**Does not prove:** that the deleted clauses still hold. T-R001 says this. A test
that was already green before the deletion proves what it proved before. The
clause-to-test table is the whole claim, and it is checkable only by reading.

**Cannot run on this host.** `linker 'cc' not found`. The entry names
`sh scripts/windows/run-in-base.sh` as the substitute and is right to.

**A check that would pass without reaching its subject:** if an implementor
deletes `388` and `220` and the repoint of `TODO/podssh.md:126` is a no-op
because that line does not name `388`, the gate is green and nothing was
repointed.

### T-R002 — `cargo test --workspace`

**Proves:** every new inline test passes and no existing test broke — for the
eleven pure clauses that land in a workspace member.

**Does not prove:** that the new tests can fail. The entry says this and puts
the burden on the plants.

**Cannot reach its own subject for two of its fifteen clauses.** Clauses 14 and
15 land in `crates/podbox-interpose`, which `Cargo.toml:20` excludes. `cargo
test --workspace` never compiles that crate. Its tests run at `gate.yml:213`,
which this entry does not name. The entry's `Prove` line is incomplete for its
own table.

**A check that would pass without reaching its subject:** four of the fifteen
rows are marked "stays shell" or "needs a fixture". Written literally, the
Approach produces fifteen tests; four of them would have no subject to assert
against and would pass vacuously or not compile.

### T-R003 — `cargo test -p podbox-image`

**Proves:** the registry fixture binds, serves, shuts down, and the two
`podbox-image` tests that use it pass.

**Does not prove:** the other six scripts the wave unblocks. Three land in
`crates/podbox-cli/tests/` and one in `crates/podbox-probe/tests/`.
`cargo test -p podbox-image` does not reach any of them. The entry does state
"The full gate runs too", so the reader is not misled — but the `Prove` field
names one command for a wave with six test files.

**Cannot run on this host.** `linker 'cc' not found`.

**A check that would pass without reaching its subject:** the fixture must
"shut down so the test does not leak a listener". A fixture that answers one
request and returns is exactly `serve_once`, which the entry rules out by name
but does not test for. Nothing in the `Prove` detects a listener leak.

### T-R004 — `py scripts/check-todo.py`

**Proves:** the ported gate accepts the same records the Python one accepted.
That is the whole claim, and the entry says so.

**Does not prove:** the build, the lint, or the test sides. The entry names
"all four gate jobs" as the real proof and none of them runs on this host.

**A check that would pass without reaching its subject:** `check-todo.py` is
itself the subject being ported. Once `check-todo.py` is deleted, the `Prove`
command is `py scripts/check-todo.py` and that file no longer exists. The entry
does not say what the command becomes after the port. This is a real gap: the
`Prove` line is the thing the wave deletes.

**A second one:** the entry says `scripts/check-todo.py` must not remain as a
forwarding shim, "because the record gate counts tracked files". The gate that
counts them is the one being replaced. Nothing in the `Prove` checks that the
Python file is gone.

### T-R005 — `cargo test --workspace`

**Proves:** the three new crates build and their unit tests pass.

**Does not prove:** the three deleted Python scripts' behaviour. The entry does
not say this — unlike T-R001, T-R004 and T-R002, T-R005's `Counts` section has
no "what it does not prove" sentence. `cargo test --workspace` passing after
`build-state.py` and `document-state.py` are deleted says nothing about whether
the Rust replaces them.

**Cannot run on this host.** `linker 'cc' not found`.

**Values the entry asserts that are estimates or unknown:**

- "ten members become thirteen … and fourteen after wave 4" — the current count
  is **9**, counted with `tomllib`. The figure is wrong, not an estimate.
- `scripts/check-state` — **does not exist** on this host. The build-state
  inputs come from `scripts/build-state.py`, which the same entry replaces.
  Record `-`; what would settle it is the name the ported binary takes.
- `experiments/153-store-lock-race.sh` "(755 lines)" — **TRUE**;
  `grep -E '^\s+[0-9]+\s+' assignment-map.md` gives 755 and the file is 755
  lines.
- `experiments/190-parallel-layers.sh` "(624 lines)" — **TRUE**, same check.

---

# Verify PLAN.md sections 4, 5 and 6 corrections required

1. **Fix the verdict distribution in PLAN section 2, section 4's frame, and
   T-R000's implied table.** The ledger holds KEEP-SHELL 27, SPLIT 35,
   RUST-TOOL 27, RUST-TEST 20, DELETE 12. The plan says 29/36/27/18/11. All 121
   ledger rows were checked against the group body at the cited line and 121
   match. The derived figures follow: 47 converge wholly on Rust, not 43; 82
   retire, not 84.

2. **Fix the three wrong crate paths.** `probe_cache.rs` is
   `crates/podbox-image/src/`, not `podbox-probe`. `identity.rs:336` is
   `crates/podbox-complete/src/`, not `podbox-enter`. `nongoals.rs` and
   `report.rs` are `crates/podbox-probe/src/`, not `podbox-supervise`. An
   implementor following T-R002 writes to three paths that do not exist.

3. **Fix the member count.** `Cargo.toml` has **9** members, not 10. Nine + 3
   (wave 5) = 12, and 12 + 1 (wave 4) = 13. PLAN section 3's "Ten members
   become fourteen" and T-R005's "ten … thirteen … fourteen" are both off.

4. **Fix `TODO/podssh.md:126`.** It does not name `388-interactive-shell.sh`.
   Only `:79` does. Repointing `:126` is a no-op.

5. **Fix correction 4's KVM figure.** `experiments/results/perf-kvm.txt:133`
   reads `0.935`, not `0.936`. The TCG half of the correction is right — remove
   `1.869` — but the supporting sentence about a `guest.tcg.boot` row and a
   missing `qemu-system-x86_64` is not in that file.

6. **Write the corrected values for corrections 6 and 9.** Both name a wrong
   record and stop. Correction 6 needs the new `lib.rs` lines (`fchmodat` is at
   `:356` and `:1038`, not `:334` and `:989`) and a decision on the
   "17 declared, 17 exported" figure at `interpose.md:87`, which is now 112.
   Correction 9 needs the replacement `Source` line for `gate.md:434`.

7. **Fix the header-vs-body claim.** PLAN section 1 and T-R000's Decision rest
   on "Six of ten headers disagree with their own bodies, and the header labels
   total 112". Read by hand, all ten headers sum to 121 and match the ten
   bodies. The Decision's action stands; its justification does not.

8. **Fix the 13-plant attribution.** The count is 13 and the files are
   `TODO/deps.md` (1) and `TODO/gate.md` (12). `TODO/cli.md` contributes none.
   An implementor repointing "13 lines across three files" will search `cli.md`
   for a line that is not there.

9. **Fix the `docs/conventions/code.md` citations.** The file is 36 lines.
   `:39` does not exist (T-R002 uses it for the plant rule, which is
   `docs/methodology/gate.md:47-54`). `:15` is "Bound buffers", not the
   speculative-machinery rule, which is `:13`. `:33` is the mock rule; VC-3's
   deployment rule is `:29`.

10. **Fix correction 5's line number.** `experiments/results/attribute.txt`
    reads ESRCH at line **6**, not 7. The correction's substance is right.

11. **Fix correction 8's mechanism.** The entry says the mutation-landed guard
    "fires correctly and then exits 1". It does not fire. `mutate()` at `:118`
    compares `git hash-object` before and after; a sed matching nothing leaves
    them equal, so it prints "THE MUTATION DID NOT LAND" and sets `fail=1` at
    `:131`. The exit code is the same and the reason is different. An implementor
    who expects the guard to fire will look for the wrong evidence.

12. **Fix the wave-2 clause count.** Four of the fifteen rows keep their shell
    script. Eleven become Rust tests. The `Approach` says "One test per clause"
    and the `Problem` says "15 clauses … asserted by no Rust test today"; both
    read as fifteen tests.

13. **Name T-R002's second `Prove` command.** `cargo test --workspace` does not
    reach `crates/podbox-interpose`, which is excluded at `Cargo.toml:20`.
    Clauses 14 and 15 are proven at `gate.yml:213`.

14. **Resolve `dev.sh`.** T-R004 puts it behind `podbox-gate`'s `podbox-dev`.
    T-R005 discusses it under the `podbox-buildstate` heading and says the
    `build-state.py` port "must not reintroduce the stamp, and `dev.sh:41` and
    its two sibling sites are deleted or repointed in the same change". PLAN
    section 3 puts it under `podbox-gate`. An implementor does not know which
    crate owns the `dev.sh` conversion.

15. **Order the `110-bloat-delta.sh` move.** The coupled path change is wave 4
    (`check-todo.py:228` and `plant.sh:51`), the script's deletion is wave 5.
    Between them, `check-todo.py` names a file that no longer exists. The entries
    do not say that the path change is the last act of wave 4.

16. **Give every `podbox-cli/tests/*.rs` file a stated shape.** T-R003 says an
    implementor "may argue otherwise" with the reason recorded. Six of the eight
    files in its table land in `podbox-cli`, and the black-box-vs-`[lib]`
    choice governs all six. No default is binding.

17. **Say what T-R004's `Prove` command becomes.** It is
    `py scripts/check-todo.py`, and wave 4 deletes `scripts/check-todo.py`. The
    entry must name the post-port command.

18. **Assign the 71 unassigned scripts.** 12 DELETEs, 27 KEEP-SHELLs and the
    remaining SPLITs appear in no wave and in no entry. A reader cannot tell
    whether a KEEP-SHELL script is deliberately retained or overlooked.

19. **Settle the `153-store-lock-race.sh` decision.** T-R005 says "decide its
    fate before porting" and leaves it open. An implementor starting that row
    stops.

20. **Give `crates/podbox-gate` a layout.** One `[[bin]]` per binary, six
    `src/bin/*.rs`, or one library with six thin mains. `Cargo.toml:3-13` has no
    pattern to copy and the entry does not pick one.
