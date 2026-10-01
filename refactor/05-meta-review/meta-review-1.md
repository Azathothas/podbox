# Meta review 1: the meta report, checked claim by claim

## Meta review 1 scope and method

I checked the five load-bearing claims of `refactor/04-meta/meta-1.md` against
the artefacts, independently of the meta report's own commands.

**What I read in full.**

- `refactor/04-meta/meta-1.md` (775 lines), `refactor/03-peer-review-2/round-2.md`
  (589), and the verdict lines, header tables and script-section headings of all
  ten `refactor/01-audit/group-*.md`.
- The eight scripts the meta names as converted: `experiments/70-whiteout-contract.sh`,
  `80-interposer-abi.sh`, `90-nsswitch-contract.sh`, `149-podvm-non-goals.sh`,
  `160-store-gc.sh`, `170-probe-cache.sh`, `220-extract-path-safety.sh`,
  `388-interactive-shell.sh`, in full. Plus the header and clause list of
  `105-interpose-ownership.sh`, `106-interpose-identity.sh`, `153-store-lock-race.sh`,
  `330-exit-codes.sh`, `359-supervision-split.sh`, `362-windows-refusal.sh`,
  `363-device-map.sh`, `85-completion-symlink-escape.sh`, `150-image-acquisition.sh`.
- Every Rust test the meta cites, at the site: `crates/podbox-enter/src/abi.rs:1074-1503`,
  `crates/podbox-ssh/tests/session_interactive.rs` (484, in full),
  `crates/podbox-probe/src/nongoals.rs:215-319`, `crates/podbox-probe/src/report.rs:1040-1130`,
  `crates/podbox-image/src/probe_cache.rs:1-60` and `:200-330`,
  `crates/podbox-image/src/store.rs:425-465` and `:860-900` and `:1930-2400`,
  `crates/podbox-image/src/contain.rs:100-150`,
  `crates/podbox-extract/src/safety.rs:500-600`, `whiteout.rs:78-130`,
  `drive.rs:1-60` and `:120-180` and `:190-300` and `:340-400` and `:470-500`,
  `crates/podbox-complete/src/write.rs:640-720`, `identity.rs:1-30` and `:300-400`,
  `crates/podbox-probe/src/exit.rs:110-167`, `crates/podbox-probe/src/supervise.rs` test names,
  `crates/podbox-image/src/registry.rs:940-1010`, `crates/podbox-cli/src/system.rs:145-230`.
- `Cargo.toml`, `crates/*/Cargo.toml` (all ten), `crates/podbox-interpose/Cargo.toml`,
  `docs/conventions/code.md` (36, in full), `TODO/RULES.md` (121, in full),
  `TODO/INDEX.md:42-58`, `docs/architecture.md:50-85`, `docs/code-map.md:7-16`,
  `.github/workflows/gate.yml:36-72` and `:188-213`, `scripts/plant.sh:45-60` and `:276-294`,
  `scripts/check-todo.py:225-232` and `:296-305`, `scripts/build-interpose.sh` (237, in full),
  `refactor/00-orientation/assignment-map.md`.

**What I counted, and with what command.**

| Quantity | Command | Result |
| --- | --- | --- |
| `#[test]` in `src/` and `build.rs` | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/src crates/*/build.rs \| awk -F: '{s+=$2} END {print s}'` | **950** |
| `#[test]` in `tests/` | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/tests \| awk -F: '{s+=$2} END {print s}'` | **30** |
| `#[test]` over all of `crates/` | same over `crates`, minus `target/` | **980** |
| `#[cfg(test)]` attributes | `grep -rn --include='*.rs' '#\[cfg(test)\]' crates --exclude-dir=target \| wc -l` | **118** |
| Module headers among them | `grep -A1 '#\[cfg(test)\]'` filtered to `mod ` | **117** |
| `#[tokio::test]` | `grep -rn --include='*.rs' '#\[tokio::test\]' crates --exclude-dir=target \| wc -l` | **0** |
| `[dev-dependencies]` | `grep -rn 'dev-dependencies' Cargo.toml crates/*/Cargo.toml` | **0**, exit 1 |
| `tempfile` in any manifest | `grep -rn 'tempfile' Cargo.toml crates/*/Cargo.toml` | **0**, exit 1 |
| `tempfile` in `Cargo.lock` | `grep -n 'tempfile' Cargo.lock` | **0** |
| `tests/` directories | `find crates -type d -name tests -not -path '*/target/*'` | **1**: `crates/podbox-ssh/tests` |
| `Row {` in the parity table | `awk 'NR>=232 && NR<=537' crates/podbox-cli/src/parity.rs \| grep -c 'Row {'` | **265** |
| Scripts in the assignment map | `grep -cE '^ *[0-9]+ +(experiments\|scripts)/' refactor/00-orientation/assignment-map.md` | **121** |
| Verdicts in the ten group bodies | `.tmp/mr1/verdicts4.py` (one verdict per `## Group N - <script>` section) | **121** |
| `Prove` lines naming `scripts/plant.sh` | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c 'Prove'` | **13** |

**Three candidate explanations I considered before the count, and what settled each.**

1. *The 950/30 split is a grep artefact and the true test count is lower* — a
   `#[test]` inside a doc comment or a `no_run` block would inflate it. **Refuted
   at the source:** `grep -rn 'no_run' crates --exclude-dir=target` returns 0 hits,
   so there is no `no_run` block in the tree; and every `#[test]` is a real
   attribute on a real `fn`, which `cargo metadata` confirms as `doctest` and
   `test` targets. The 950 counts a real attribute, not a comment.
2. *`podbox-cli` has no `lib` target, so its 244 `#[test]` functions are unit
   tests of a binary and a `tests/` file there cannot link them.* **Confirmed by
   `cargo metadata`**, and this is a finding the meta did not record: the
   `podbox-cli` target list is `['bin', 'custom-build']` with no `lib`.
3. *A `cdylib`-only crate cannot host `tests/*.rs`.* The meta asserts this from
   cargo's rule. I built the minimal case in a scratch crate and read the actual
   error rather than the rule. See the claim 2 section.

**What I could not settle.**

- **Nothing compiles on this host.** `cargo build` stops at
  `error: linker 'cc' not found`, exactly as round 2 recorded at O4. Every claim
  below about what links is a claim about what `cargo check` and `cargo build`
  printed, and about the crate graph `cargo metadata` reports. No claim here
  rests on a passing test run, because I ran none.
- **The count of tests that actually pass** is unknown and stays unknown here.
  980 is a static count of attributes, not of executed tests.
- I did not run any experiment script, and I read the bodies of 4 of the 121
  scripts only as far as their header clause lists (`105`, `106`, `153`, `330`,
  `359`, `362`, `363`, `150`, `85`). No claim below rests on those bodies.
- `TODO/PROGRESS.md` was not read. Nothing here depends on the work order.

---

## Meta review 1 — claim verification

### Claim 1: 980 test functions, 950 inline and 30 in `tests/` — **CONFIRMED**

My counts reproduce the meta's exactly: **950** inline, **30** in `tests/`,
**980** in total, by three independent commands over three file sets.

The counting method is sound and I checked its edge cases:

- **The pattern is the literal string `#[test]`,** counted per file with
  `grep -c` and summed. It includes `build.rs`: `crates/podbox-cli/build.rs`
  is the only `build.rs` in the tree, and it carries 0 of the 950.
- **Zero `#[tokio::test]` anywhere.** The whole-tree search returns 0 matches.
- **Zero doc-comment false positives.** There is no `no_run` block in the tree
  (`grep -rn 'no_run' crates --exclude-dir=target` returns 0), so there is no
  compiled-out doctest to be miscounted. The 380 sites where a `#[test]` is
  preceded by a `///` line are ordinary doc comments on real test functions.
- **Zero `#[ignore]` anywhere**, so no counted attribute is a permanently
  skipped test. `docs/conventions/code.md:34` says a skipped test proves nothing
  about its subject; there is nothing to discount.

The supporting claims, each with my own command:

| claim | result |
| --- | --- |
| 117 `#[cfg(test)]` modules | **CONFIRMED**: 118 attributes, 117 module headers. The 118th is `crates/podbox-image/src/store.rs:1102`, the call `tests::note_the_refusal(path, fd);` inside an existing module. |
| 116 named `tests`, one named `drive` | **CONFIRMED**: `crates/podbox-extract/src/lib.rs:40` is `mod drive;` and every other header is `mod tests {`. |
| 1 `tests/` directory | **CONFIRMED**: `crates/podbox-ssh/tests` only. |
| 0 `[dev-dependencies]` | **CONFIRMED**: exit 1, no match in any of the eleven manifests. |
| 0 `tempfile` in any manifest or `Cargo.lock` | **CONFIRMED**: exit 1 on the manifests, no match in the lock. |

**Two arithmetic corrections the meta did not record.** The meta's own
per-crate table is wrong in two places; its totals are right.

| crate | meta's row | mine | what differs |
| --- | --- | --- | --- |
| `podbox-enter` | "11 src files" | **10** | the meta's total row says 137 src files; `find crates/*/src -name '*.rs'` returns **126**, and 126 + 4 `podbox-ssh` bins + 4 `podbox-ssh/tests` + 1 `build.rs` = **135**, not 137. The meta's `137` is the count with `podbox-enter` and `podbox-complete` each over by one. |
| `podbox-complete` | "9 src files" | **8** | as above. |

Neither changes a total. The ten per-crate `#[test]` counts, the 980, the 950
and the 30 all reproduce.

**One structural fact the meta's table omits, and it changes a wave.**
`cargo metadata --no-deps` reports the `podbox-cli` package's targets as
`['bin', 'custom-build']`. **`crates/podbox-cli` has no `lib` target.** Its 244
inline `#[test]` functions are compiled into the `bin` target and are reachable
only by name from the test harness. A `crates/podbox-cli/tests/*.rs` file cannot
`use podbox_cli::…`, because there is no such library. Every `podbox-cli` test
the meta schedules in wave 4 — `completion_containment.rs`, `curated_surface.rs`,
`ascii_output.rs`, `verify_export.rs` — is therefore either a black-box
`CARGO_BIN_EXE_podbox` driver or needs a `lib` target added first. The meta calls
this "a stated structural change"; it is two structural changes, and the second
is not stated.

### Claim 2: 8 scripts fully or mostly converted, 4 partially — **CORRECTED**

The count of 8 fully-or-mostly and 4 partial is right in shape and wrong in
composition, because the meta asks "does a named Rust test exist for a clause"
where the question that decides the plan is "does the Rust test assert what the
shell clause asserted". Read clause by clause, **2 of the 8 are genuinely
converted, 4 are partial, and 2 have a clause the script asserted and nothing
covers.** The full table is in the next section.

Two specific claims inside the conversions section are wrong:

1. **"`160-store-gc.sh` is 4 of 5 clauses already in Rust"** and "all five
   clauses of `160` are in Rust" (stated twice, at the conversions table and at
   finding 4). **REFUTED.** The store test at `crates/podbox-image/src/store.rs:1976`
   asserts `store.remove` returns an error containing `"in use"` and that
   `store.prune(true)` returns it in `skipped`. It calls `s.hold(&r)` and
   `s.in_use(&r)`. The script's clause 3 asserts the *shipping CLI* answers: that
   `podbox rmi` exits non-zero, that `podbox image prune -af` exits 0 and prints
   `skipped:`, and that `podbox images -q` still lists the image. Those are
   `crates/podbox-cli/src/images.rs`, and nothing in that file asserts them.
   The meta itself says so in its own proposal table, at the `160` row, where it
   says "the one real gap is the CLI text" — and then in finding 4 and in the
   eight-script list it calls `160` fully converted. The meta contradicts itself
   on the same script in the same document.
2. **"Group-4's `70` plan is 4 of 4 checks done as predicates, so the whole
   RUST-TEST half of `70` is deletion plus a record."** **REFUTED.** Read as
   predicates, checks A, C and D are covered (`whiteout.rs:89`, `safety.rs:543`,
   `drive.rs:297` and `:89`). Check B is not a predicate at all: the script reads
   `tar tvzf` over a real alpine layer and asserts uid 0 gid 42
   (`70-whiteout-contract.sh:100-104`), and no Rust test reads a real layer.
   The closest is `drive.rs:480`, which drives a crafted layer with
   `E::Owned("etc/shadow", b"x", 0, 42)` and asserts the extractor *did not apply*
   the gid. The script clause and the Rust test are about opposite outcomes.
   Clause B is a *measurement* that has no predicate form, so `70` cannot be
   deleted on a predicate count.

### Claim 3: six group header tables disagree with their bodies, both sets 121 — **CORRECTED**

Six groups disagree, and the meta names the right six. My body count reproduces
the meta's before-VC totals exactly: **KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20,
SPLIT 35, DELETE 12 = 121.** The per-group body counts also match the meta's
table line for line.

| group | body n | header n | agree? |
| --- | --- | --- | --- |
| 1 | 9 | 9 | **no** — body SPLIT 3 / RUST-TOOL 2; header SPLIT 2 / RUST-TOOL 3 |
| 2 | 12 | 12 | yes |
| 3 | 13 | 13 | yes |
| 4 | 12 | 12 | **no** — body KEEP-SHELL 4 / RUST-TEST 4; header KEEP-SHELL 3 / RUST-TEST 5 |
| 5 | 12 | 12 | **no** — body SPLIT 5 / RUST-TEST 3 / RUST-TOOL 1 / KEEP-SHELL 1 / DELETE 2; header SPLIT 2 / RUST-TEST 4 / RUST-TOOL 2 / KEEP-SHELL 3 / DELETE 1 |
| 6 | 13 | 13 | **no** — body RUST-TEST 0; header RUST-TEST 1 |
| 7 | 12 | 12 | **no** — body SPLIT 4 / RUST-TEST 2; header SPLIT 3 / RUST-TEST 3 |
| 8 | 13 | 13 | yes |
| 9 | 13 | 13 | **no** — body SPLIT 7 / RUST-TEST 1 / KEEP-SHELL 1 / RUST-TOOL 3 / DELETE 1; header SPLIT 5 / RUST-TEST 4 / KEEP-SHELL 2 / RUST-TOOL 2 / DELETE 0 |
| 10 | 12 | 12 | yes |
| **total** | **121** | — | 4 agree, 6 differ |

**"Both sets summing to 121" is where two sources disagree, and the meta does
not say so.** The body total is 121. The header tables do **not** sum to 121.
Adding the six disagreeing groups' own header numbers gives 109, and the
meta's claim that the header set also totals 121 is false. Here is the
arithmetic from the headers alone:

| group | header says | header sum |
| --- | --- | --- |
| 1 | RUST-TOOL 3, SPLIT 2, RUST-TEST 2, KEEP-SHELL 1, DELETE 1 | 9 |
| 2 | DELETE 0, RUST-TEST 1, RUST-TOOL 6, KEEP-SHELL 4, SPLIT 1 | 12 |
| 3 | per-script table: 13 rows, and `Counts:` line 2/3/3/1/4 | 13 |
| 4 | `Metric` table: RUST-TEST 5, RUST-TOOL 2, KEEP-SHELL 3, SPLIT 2 | 12 |
| 5 | DELETE 1, RUST-TEST 4, RUST-TOOL 2, KEEP-SHELL 3, SPLIT 2 | 12 |
| 6 | RUST-TEST 1, RUST-TOOL 1, KEEP-SHELL 3+1+1, SPLIT 6, DELETE 1 | 14 |
| 7 | `Item` table: DELETE 1, RUST-TEST 3, RUST-TOOL 2, KEEP-SHELL 3, SPLIT 3 | 12 |
| 8 | RUST-TOOL 4, RUST-TEST 4, SPLIT 3, KEEP-SHELL 2 | 13 |
| 9 | RUST-TEST 4, RUST-TOOL 2, KEEP-SHELL 2, SPLIT 5, DELETE 0 | 13 |
| 10 | RUST-TEST 1, RUST-TOOL 3, SPLIT 2, KEEP-SHELL 4, DELETE 2 | 12 |
| | | **112** |

Two separate header defects produce that 112 against a 121 body:

- **Group 6 double-counts.** Its table has three `KEEP-SHELL` rows (`:3` counts
  three named scripts, then `:KEEP-SHELL (build helper)` and
  `:KEEP-SHELL (lane fixture)` each add one more) and its own closing line says
  "**DELETE 1, RUST-TEST 1, RUST-TOOL 1, KEEP-SHELL 4, SPLIT 6**" = 13. The
  table is 14; the closing line is 13; the body is 13 with RUST-TEST 0.
- **Group 4 contradicts itself.** `group-4.md:13-17` reads `DELETE 0, RUST-TEST
  5, RUST-TOOL 2, KEEP-SHELL 3, SPLIT 2`; `group-4.md:21-34` is a per-script
  table reading RUST-TEST 4 and KEEP-SHELL 4, which matches the body. Two header
  artefacts, one file, two answers, and the meta's own table quotes the wrong one
  as the disagreeing artefact.

**Which set is authoritative.** The bodies. Each body verdict is a per-script
decision argued in the section it sits in; each header is a distribution written
before or independently of those sections, and in four of the six cases the
header contradicts a per-script table the same group also wrote. A downstream
writer working from a header writes the wrong entry for six groups.

**Whether a reader would be misled.** Yes, and the sums do not catch it: the
per-group counts are equal in all ten cases, so a reader who checks totals sees
9=9, 12=12 and is confirmed. The defect is only visible by counting labels. The
meta's sentence "Both sets total 121, so neither is caught by a sum" is
half-right: the bodies total 121 and the headers do not, and the reason a sum
does not catch it is that the *group* totals match, not the label totals.

### Claim 4: 18 new crates proposed, 4 recommended — **CORRECTED**

The 18 names exist as the meta lists them, and the meta's arithmetic on the cost
is fair. Two corrections and one endorsement.

**Correction 1: the four-crate collapse is right for `podbox-gate` and
`podbox-plant` and wrong for `podbox-buildstate`.** `TODO/INDEX.md:42-58` is the
category table, and its `crate` column is the authority the meta invokes. Read
against it:

| meta's crate | the scripts folded in | the category table's `crate` cell for those subjects | verdict |
| --- | --- | --- | --- |
| `podbox-gate` | `check-todo.py`, `todo-count.py`, `zig-cc.sh`, `dev.sh`, `nightly-smoke.sh`, `398-gate-diagnostics.py` | `gate.md`'s cell reads **`scripts/`** (line 56) | **correct.** One subject, one cell. Group-1 and group-2 already fold by shared grammar. |
| `podbox-buildstate` | `build-state.py`, `release-licenses.py`, `session-start.sh`, `310-session-startup.sh` | `packaging`'s cell reads **"the artefact"** (line 53); `session-start.sh` is `packaging` T-1005 and `310` is `packaging` T-1005 | **partly correct.** `build-state.py` and `release-licenses.py` share a subject (the build's freshness and its licence manifest) and belong together. `session-start.sh` and `310-session-startup.sh` are *session* measurements, not build state. Folding four into one bin crate puts two unrelated subjects behind one `podbox-buildstate` name. |
| `podbox-plant` | `plant.sh` | `gate`'s cell reads **`scripts/`** | **correct, and it is `podbox-gate`'s own subject.** See correction 2. |
| `podbox-verify` | `verify-release.sh`, `120-reproducible-build.sh`, `157-lock-inheritance-prove.sh`, `build-interpose.sh`, `110-bloat-delta.sh`, `document-state.py`, `release-notes.sh`, `395`, `399` | `packaging` **"the artefact"**, but `110-bloat-delta.sh` is `deps` whose cell is **"the whole tree"** (line 52), and `build-interpose.sh` is `packaging` T-1002 | **wrong.** This is six subjects behind one name, and one of them (`110`) is owned by the one category whose `crate` cell is the whole tree, which is the opposite of a crate. |

**Correction 2: `podbox-gate` and `podbox-plant` are the same crate.** The meta
lists them as two of its four, justified separately, and then argues at length
that `plant.sh` "goes last, and it goes with the 11 `Prove` lines" while `check-todo.py`
"is the project's instrument for every other change in this plan". Both are the
gate. `TODO/INDEX.md:56` puts `gate` in one cell. `scripts/check-todo.py:300`
`check_tree` reads every tracked file, which is why `plant.sh` being a tracked
file is a coupling between them. One crate, `podbox-gate`, with `podbox-gate`,
`podbox-count` and `podbox-plant` binaries, is the shape the category table
already describes. The meta's own wave 5 and wave 8 then split one crate across
two waves.

**Correction 3: the 14 that "do not" share a subject are not fourteen
independents.** The meta's closing line is that "the other fourteen do not" do
what group-1 and group-2 do. Two of them plainly do:

- `120-reproducible-build.sh` and `157-lock-inheritance-prove.sh` are already
  folded by the meta itself into `podbox-rebuild` (its own table, row
  `crates/podbox-rebuild`), and round 2 correction 15 groups them as the two
  plans sharing crate, level, fixtures, plant, command and `exit 2`. They are
  the same subject — a mutation-planted proof about the store's lock
  discipline — and the meta's own table says so.
- `document-state.py` and `release-notes.sh` are folded into `podbox-docs` by
  the meta's own table, which lists both binaries under one crate.

**What the rules actually say, checked.** `docs/conventions/code.md:13` reads
"Do not add unused frameworks, duplicate implementations, or dead code." The
meta quotes this and I do not think it carries the weight the meta puts on it:
a new crate is not a framework, and the meta's four crates are not unused. The
rule that *does* apply is `TODO/INDEX.md`'s category table, and that is what I
checked above. `TODO/RULES.md` says nothing about crates, members, or workspace
shape; I searched it (`grep -in 'crate\|member\|workspac'` returns 0 hits) and
the meta's claim to be checked against it cannot be checked, because it is not
there. The meta's citation of `TODO/RULES.md` for the crate question is
inaccurate.

**Crates I endorse.** Four, with the composition corrected:

1. **`crates/podbox-gate`** — `check-todo.py`, `todo-count.py`, `zig-cc.sh`,
   `dev.sh`, `nightly-smoke.sh`, `398-gate-diagnostics.py`, **and `plant.sh`**,
   as binaries `podbox-gate`, `podbox-count`, `podbox-plant`. One category, one
   cell in the category table, one tracked-file coupling.
2. **`crates/podbox-buildstate`** — `build-state.py` and `release-licenses.py`
   only, as `podbox-buildstate` and `podbox-release-licenses`. Build freshness
   and the licence manifest are one subject.
3. **`crates/podbox-release`** — `verify-release.sh`, `110-bloat-delta.sh`,
   `document-state.py`, `release-notes.sh`. All four read the shipped artefact.
   `110` stays a bin here and NOT a workspace-member library, because
   `TODO/deps.md` and the `deps` category cell ("the whole tree") make it a
   measurement of the tree rather than a component of it. Its move must carry
   `scripts/check-todo.py:228` `CEILING_SCRIPT` and the four plants at
   `scripts/plant.sh:278-293` with it, which I verified by reading both.
4. **`crates/podbox-podvm`** — `145-podvm-parity.sh` and `154-tcg-workload-spread.sh`.
   The `podvm` category's cell is "the machine tier", a distinct subject from
   packaging and from the gate.

I do not endorse the other proposed names as crates. `podbox-lockrace`,
`podbox-ptmx-cover`, `podbox-perf`, `podbox-nsdrive` and `podbox-session` are
**binaries**, not subjects: each is one script with one proof, and each has a
home. `153` belongs to `podbox-image` as a `tests/` file or a `podbox-gate` bin;
`251` to `podbox-enter`; `360` to the gate's perf budget, which
`scripts/check-todo.py:1382` `check_perf_budget` already reads;
`366` to `podbox-probe`; `310` to `podbox-gate` beside `dev.sh`, since both
measure a lane's startup. One crate per script is the shape the meta says it is
rejecting; it is what eighteen names actually is.

**`podbox-interpose-check`: CONFIRMED, and the meta's reasoning is right.** A
crate that checks whether the interposer can be built as a `cdylib` cannot be a
workspace member, because `Cargo.toml:15-20` excludes `podbox-interpose` for
exactly that reason ("A member would share this workspace's dependency
unification and feature resolution, which is the coupling it exists to avoid").
Adding a checking crate to `members` inverts what it measures. The meta's
conclusion stands and `docs/architecture.md:55` corroborates it.

### Claim 5: nine implementation waves, no Rust OCI registry fixture — **CORRECTED**

**The registry-fixture claim is CONFIRMED and stronger than the meta states.**
`crates/podbox-image/src/registry.rs:955` `serve_once` is a `fn` inside
`mod tests` at `:949` — it is a **test helper, not a production fixture**. It
binds a `TcpListener`, accepts **one** connection, reads up to 4096 bytes, writes
`HTTP/1.0 200 OK` with a fixed `Content-Length`, and returns a port. It is not
HTTPS, not an OCI registry, and not reachable from any binary. `TODO/image.md:483`
gives T-0206's Prove as `./experiments/180-registry-fixture.sh` and `:485-495`
records it as a downloaded `zot-linux-amd64-minimal` v2.1.21 binary. **There is
no Rust OCI registry fixture in the tree, and the only Rust loopback is
test-private.** The meta's wave 4 boundary condition on `store_digest` is right
and is the correct dependency edge.

**The two gate commands are CONFIRMED, but the meta's line numbers are off by
one and one of its two claims about them is wrong.**

`.github/workflows/gate.yml:198-199` is the `workspace tests` step and
`:210-213` is the `interposer tests` step:

```yaml
198	      - name: workspace tests
199	        run: cargo test --workspace
...
210	      - name: interposer tests
211	        env:
212	          RUSTFLAGS: -C target-feature=-crt-static
213	        run: cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu
```

The meta cites `gate.yml:197` and `:211`. Line 197 is blank; line 211 is the
`env:` key. The commands are right; the lines are off. This is the same
off-by-a-couple pattern the meta found in group 1 and group 4, in its own
citations, which is worth saying.

**A larger correction: these are not the two commands that gate every wave.**
`gate.yml` has four jobs, and the required gate is not the two the meta names.
Reading the job list: `todo` (`:36-37` `check-todo`, `:41` `todo-count is
idempotent`, `:55-56` `./scripts/plant.sh`, `:58` maintained repository checks),
`build` (`:99` `build-interpose.sh`, `:102` `cargo build --release --target
x86_64-unknown-linux-musl`, `:131` `./experiments/110-bloat-delta.sh ci`),
`lint` (`:154` fmt, `:159` `cargo clippy --workspace --all-targets -- -D
warnings`, `:164` interposer clippy, `:171` markers and prose, `:176` shell
parses, `:186` `py_compile`), and `test` (`:199`, `:213`). A wave that changes
a manifest, a script, or a document is gated by `todo`, `build` and `lint`, not
only by `test`. **The meta's wave proofs are incomplete**: waves 5, 6, 7 and 8
change the build and the gate itself, and their stated proof — `cargo test -p
<crate>` — does not run `clippy --all-targets -D warnings`, does not run
`build-interpose.sh`, and does not run `plant.sh`. `docs/methodology/gate.md:23`
names the full set: "formatting, lint, workspace tests, interposer tests, and
repository checks."

**The wave boundaries, each with the entry that justifies it.**

| boundary | justified? | the entry that makes it a boundary |
| --- | --- | --- |
| 0 → 1 | **yes** | `157-lock-inheritance-prove.sh:157`'s sed pattern. The pattern reads `close_in_children(lock\.fd)`; `crates/podbox-image/src/store.rs:1083` reads `if !sys::close_in_children(fd) {`. A plant encoding the current source cannot be written before the re-anchor, because the plant would pass vacuously. A **shared fixture** (the mutation anchor), not convenience. |
| 1 → 2 | **no** | Wave 1 deletes; wave 2 adds tests. No crate, fixture, record or dependency edge separates them. Both are `cargo test --workspace` in a tree where every target is an existing `mod tests`. **This is convenience.** It should be one wave. |
| 2 → 3 | **no** | Wave 3's two entries (`70`'s new test, `85`'s two `write.rs` tests) target `crates/podbox-extract/src/drive.rs` and `crates/podbox-complete/src/write.rs`. Wave 2 targets ten other files. Nothing is shared and nothing is ordered. The meta's own stated reason — "keeps the wave's fixture work in one place" — is a convenience, and it is also wrong: `write.rs:643` `scratch` already creates the temp rootfs, so there is no fixture work to keep in one place. **This is convenience.** |
| 3 → 4 | **yes** | The first `tests/` directories outside `podbox-ssh`, and the first `[[bin]]` targets. A **shared structural change**: `cargo metadata` shows `podbox-cli` has no `lib`, so every wave-4 `podbox-cli/tests/*.rs` file needs either a `CARGO_BIN_EXE_podbox` driver or a new `[lib]`. A real boundary, and a bigger one than the meta states. |
| 4 → 5 | **yes** | `crates/podbox-gate` is a **shared crate**: `check-todo.py:228` `CEILING_SCRIPT` names `experiments/110-bloat-delta.sh`, and `scripts/plant.sh:51` `FILES` lists that same path. Moving either without the other turns the gate red. A real **record-gate interaction**. |
| 5 → 6 | **no** | Two crates, no shared code, no shared fixture, no record-gate interaction. The meta's own reason is "neither needs a fixture another wave builds", which is an absence of a reason. **This is convenience.** |
| 6 → 7 | **no** | Nine proposed crates with no shared code. The meta's stated reason is that they "share a shape: each is a binary with a three-state exit". A shared shape is not a shared crate, fixture, or dependency edge. **This is convenience**, and it is the wave the meta says is justified only by a shape. |
| 7 → 8 | **yes, but the reason is the record gate, not the instrument** | `scripts/check-todo.py:300` `check_tree` reads every tracked file, and `plant.sh` is a tracked file. `.github/workflows/gate.yml:55-56` runs `./scripts/plant.sh` as the step named `every check can fail`. I counted **13** `Prove` lines naming `scripts/plant.sh` across `TODO/cli.md`, `TODO/deps.md` and `TODO/gate.md` — the meta says 11, and says 14 in the same document at the `plant.sh` proposal row. The boundary is real: the CI step and the `Prove` lines must repoint in one change. |

**Three of the nine boundaries are convenience.** Six waves are real; the nine
should be six, with 2+3 merged into wave 1, and 5+6+7 merged. What survives is:
record fixes, deletions, inline tests, `tests/` directories, `podbox-gate`, the
`podbox-ssh`/`podbox-release` bins, and `plant.sh`.

---

## Meta review 1 — conversions already done, clause by clause

The question that decides an entry's label is not "does a named Rust test exist
for a clause" but "does the Rust test assert what the shell clause asserted". A
test that covers the pure logic while the clause also drove an engine, a root
requirement, or a timeout is a **partial**, and the entry says "finish the
conversion" rather than "delete the script".

Legend: **✓** covered by a named test at the cited line; **~** the pure half is
covered and the clause also asserted something live; **✗** the script asserted
it and nothing now does.

### `388-interactive-shell.sh` — 11 clauses, 12 tests. **FULLY CONVERTED.**

| clause | script line | Rust test | verdict |
| --- | --- | --- | --- |
| 1 build the shell binary | `:43` | `setup()` at `session_interactive.rs:53` uses `env!("CARGO_BIN_EXE_shell")` | ✓ |
| 2 key-only sshd on loopback with ForceCommand | `:78` | `:46-62` builds the same config via `podbox_ssh::write_sshd_config` and starts `sshd -i` | ✓ |
| 3 prompt, echo, no terminal | `:109` | `:255` | ✓ |
| 4 editing repairs a typo | `:125` | `:276` | ✓ |
| 5 history recalls | `:140` | `:288` | ✓ |
| 6 state persists | `:154` | `:304` | ✓ |
| 7 signal kills the command | `:200` | `:323` | ✓ |
| 8 exit code passes through | `:213` | `:374` | ✓ |
| 9 exec refused naming the variable | `:226` | `:389` | ✓ |
| 10 subsystem never reaches the shell | `:245` | `:410` | ✓ |
| 11 untrapped SIGINT gives 130 | `:263` | `:351` | ✓ |

Three tests are not in the script: `:431` Ctrl-D, `:450` client EOF, `:472`
large output. The needle rule the script states at `:12-14` is restated at
`:14-20`. **The script is redundant. Delete it.** This is the one unambiguous
case in the set.

### `70-whiteout-contract.sh` — 4 checks. **PARTIAL, and not deletable.**

| check | script | Rust | verdict |
| --- | --- | --- | --- |
| A member names carry no `./` prefix | `:78-90`, reads real layers via `docker save` | `safety.rs:530` `ordinary_layer_paths_survive` asserts `components("bin/")`, `components("./etc/shadow")` | ~ the predicate is covered; the measurement over two real pinned layers is not |
| B `alpine` `etc/shadow` is uid 0 gid 42 | `:93-104` | `drive.rs:480` asserts the **opposite** outcome on a crafted layer (the gid is *not* applied) | ✗ |
| C `voidlinux` absolute self-referential symlink, whiteouted next layer | `:107-127` | `safety.rs:543` (the link) + `whiteout.rs:89` + `drive.rs:297` (the removal) | ~ |
| D udocker's `*/.wh.*` glob misses a layer-root whiteout | `:130-149` | `whiteout.rs:89` and `drive.rs:297` | ✓ |

**What a converting entry must still write.** Check B needs a test that reads a
real `alpine` layer's `etc/shadow` and asserts `0/42`. That is a fixture
concern, not a predicate concern, and it needs either a checked-in layer blob or
the registry fixture claim 5 says does not exist. **A converting entry cannot
delete `70`; it must keep the script for check B or resolve the fixture first.**
The meta says "the whole RUST-TEST half of `70` is deletion plus a record", and
that is wrong.

### `220-extract-path-safety.sh` — 6 checks. **CONVERTED, and the meta missed
the tests that do it.**

The meta cites only `safety.rs` and `drive.rs:382`. The tests that actually carry
the checks are in `drive.rs`, which is `#[cfg(test)] mod drive;` at
`crates/podbox-extract/src/lib.rs:40` and whose own header reads "The
acceptance, driven in-process against crafted layers."

| check | script | Rust | verdict |
| --- | --- | --- | --- |
| A `evil -> /etc` then `evil/passwd` | `:210` | `drive.rs:167` `an_entry_traversing_a_symlink_the_same_layer_made_is_refused` — asserts the entry name, `why` contains "leaves the destination", and the layer digest | ✓ |
| B `../escaped` refused lexically | `:214` | `drive.rs:211`, **and** `drive.rs:221-224` asserts nothing landed beside the destination | ✓ |
| C `/etc/...` absolute refused | `:218` | `drive.rs:229`, **and** `:236` asserts `/etc/podbox-should-never-write-this` does not exist | ✓ |
| D hard link outside the destination | `:222` | `drive.rs:382` | ✓ |
| E legitimate absolute symlinks survive, stored verbatim | `:226-246` | `drive.rs:264` asserts `read_link` returns `/var/cache/xbps` and `/proc/self/mounts` verbatim | ✓ |
| F nothing landed outside, checked on the filesystem | `:249-266` | spread across `drive.rs:221`, `:236`, `:253` | ✓ |

**What a converting entry must still write.** Nothing for A–E. Check F's
canary is `/etc/podbox-must-never-write-this` at `$OUT`, `$OUT/store/escaped`
and the real `/etc/podbox-must-never-write-this`; the Rust tests assert the
absolute path and the sibling, not a whole-tree sweep. A closing entry that
deletes the script loses nothing, and the wave-1 proof is the existing suite.
**This is a genuine conversion the meta under-reported.**

### `160-store-gc.sh` — 5 clauses. **PARTIAL, not "all five in Rust".**

| clause | script | Rust | verdict |
| --- | --- | --- | --- |
| 1 a second tag shares the first tag's blobs | `:72-76` | `store.rs:1950` | ~ library level, two records rather than a `tag` command |
| 2 removing one of the two names frees nothing | `:78-90` | `store.rs:1950` asserts the shared blob survives then frees | ~ |
| 3 an image something holds is refused by `rmi` and skipped by `prune` | `:93-148` | `store.rs:1976` asserts `s.remove()` errors and `s.prune(true)` returns the name in `skipped` | ✗ the **CLI** answers are unasserted: `podbox rmi` exit code, `podbox image prune -af` exit 0, the `skipped:` line, `podbox images -q` still listing the image |
| 4 once the holder goes, the image can be removed | `:151-161` | `store.rs:1976:1993-2004` drops the holder and asserts `remove` succeeds | ~ |
| 5 a blob resolving outside the store is refused | `:164-210` | `contain.rs:118` `a_symlink_pointing_out_of_the_store_is_refused` | ✗ the script's point is that the canary **survived** *and* that `prune` **named** "outside the store" (`:195-200`); the Rust test asserts only that `within()` errors. Nothing drives `delete` with a symlinked blob. |

**What a converting entry must still write.** Two tests, both in
`crates/podbox-cli/src/images.rs`, and both about the CLI's own answers:
that `rmi` on a held image exits non-zero with `in use` in stderr, and that
`image prune` exits 0, prints `skipped:`, and leaves the image listed. The
committed reading `experiments/results/store-gc.txt` records
`rmi_under_holder 125` and `prune_skipped 1`, and the meta's own proposal table
says a test written to the plan's assertion "would fail on `prune`" because
`images.rs:961-986` returns 0. **The meta is right about the gap and wrong
about the script being converted.** Its finding 4 and its eight-script list both
say all five clauses are in Rust; its own proposal table says one is not. The
two statements are in the same document.

### `90-nsswitch-contract.sh` — 4 checks. **PARTIAL. Check A is not a predicate.**

| check | script | Rust | verdict |
| --- | --- | --- | --- |
| A a supplied `/etc/passwd` is read under `files` and ignored otherwise | `:100-120`, runs a static glibc probe in a `ubuntu:20.04` container | `identity.rs:336` `every_measured_passwd_shape_ends_up_naming_files_first` | ✗ the Rust test drives the **editor** (`prepend_files`), not the glibc NSS dispatcher. `identity.rs:12` records the probe's reading as a *transcript*, and `:20-22` states the static probe still answered `NOTFOUND` — that is a recorded measurement, not an assertion. |
| B what pinned images ship | `:123-147` | none | ✗ |
| C `PT_INTERP`-free is not dependency-free | `:149-172`, `strace` | none | ✗ |
| D gconv modules and `DT_NEEDED` | `:174-191`, `readelf` | none | ✗ |

**What a converting entry must still write.** The meta's own proposal table
already says this — "check D's target is right and new". It understates it: B, C
and D are all unconverted, and A's *live* form is unconverted even though A's
*consequence* was written down. `TODO/complete.md:656-660` is explicit that
"the unit test is the measurement" for the six **shapes**, which is a different
measurement from the NSS dispatch. **A converting entry writes a static-glibc
probe fixture and a container drive for A, and a `readelf`/`strace` drive for
B, C and D.** `90` is not deletable.

### `149-podvm-non-goals.sh` — 7 clauses. **PARTIAL. Two of the meta's
corrections confirmed, one more gap.**

| clause | script | Rust | verdict |
| --- | --- | --- | --- |
| 0 six rows with a stance each | `:102-112` | `nongoals.rs:253` and `report.rs:1069` | ✓ both the assessment and the rendered document |
| 1 every refusal names an errno | `:115-128` | `nongoals.rs:261` — one row, the `bind` one | ~ |
| 2 every open row carries no refusal language | `:130-143` | `nongoals.rs:280` — one row | ~ |
| 3 every unestablished row names why | `:145-157` | `nongoals.rs:290` — one row, `(not probed)` | ~ |
| 4 kvm refused naming `open(/dev/kvm, O_RDWR)=ENOENT` | `:160-166` | `report.rs:1088` | ✓ in document and evidence, but on **synthetic** `cited_ok()` findings, not on this lane's `/dev/kvm` |
| 5 the stderr evidence carries the block | `:169-175` | `report.rs:1105-1107` asserts the exact string `non-goals, one stance per blocked design` | ✓ **the meta's finding 8 is confirmed**: this test exists and group-9's plan restates it |
| 6 the promoted goal runs, `361` green | `:177-193` | none | ✗ |

**What a converting entry must still write.** The six row **names** pinned as a
constant (the meta is right about this gap) and a clause-6 arm that shells to
`experiments/361-guest-usernet.sh`. A converting entry **cannot delete `149`**:
clause 6 is a live 1500-second drive. The meta's proposal table is right; its
"only the six-names constant is new" line in the conversions section is not,
because clause 6 was never in Rust.

### `80-interposer-abi.sh` — 5 checks. **PARTIAL. Check B has no predicate.**

| check | script | Rust | verdict |
| --- | --- | --- | --- |
| A the SONAME discriminator | `:186-211`, `readelf` on two built objects | `abi.rs:1170` `Flavour::of_libc_name` and `of_needed` | ~ the discriminator is asserted; the `DT_NEEDED` read off a real built object is not |
| B the four cross-libc arms, controls first | `:214-289`, engine, `LD_PRELOAD` into `ubuntu` and `alpine` | none | ✗ |
| C the version predicate, predicted then confirmed by the loader | `:292-331` | `abi.rs:1250` asserts the **predicate**; `:1293` the control | ~ the meta's own note, "the gap is one test on the `abi` wrapper's exit 2" |
| D the `.dynsym` trap | `:334-351`, `nm` on a stripped libc | `abi.rs:1220` | ✓ the meta's line and its 0/3136 reading are correct; the test skips when no libc is present at `:1228-1231`, which is the honest third state |
| E `podbox system abi` against the loader | `:354-438` | none. `system.rs:157` `pub fn abi` has **no** test: `system.rs`'s `mod tests` at `:603` holds 7 tests and none names `abi` | ✗ |

**What a converting entry must still write.** A test for `system::abi`'s three
exit codes (0 admitted, 1 refused, 2 unreadable, `system.rs:191-214`) against
two real files, and the engine drive for check B. **Check B cannot move**: it is
four `LD_PRELOAD` pairings against a live loader in two containers, and
`docs/conventions/code.md:33` says a mock does not prove a live service.
`80` keeps a KEEP-SHELL driver for check B whatever the entry says about the rest.

### `170-probe-cache.sh` — 3 clauses. **CONVERTED, with clause 1 outstanding.**

| clause | script | Rust | verdict |
| --- | --- | --- | --- |
| 1 two runs on one machine agree on the rung | `:90-113` | none | ✗ |
| 2 first `--cached` run measures, second is served | `:116-152` | `probe_cache.rs:233` | ✓ `Source::Measured` then `Source::Cache`, same `rung()` |
| 3 the cache is refused inside the reconstruction | `:154-243` | `probe_cache.rs:247`, `:268`, `:296` | ~ the refusal and the reason are asserted; the *reconstruction* (rung `chroot` or `supervise` against a staged `probe.json`) is not |

**What a converting entry must still write.** Clause 1 is two `podbox probe
--json` invocations compared with `jq`; it is a binary-level check and belongs
in a `tests/` file. Clause 3's engine half stays shell. **A converting entry
writes the clause-1 drive and keeps `170` for clause 3's reconstruction arm.**

**Summary of the eight.** Genuinely converted and deletable: **1** (`388`).
Converted with one clause that cannot be a predicate: **1** (`220`).
Partial: **6** (`70`, `160`, `90`, `149`, `80`, `170`). The four the meta lists
as partial stay partial. **The eight-script list is wrong in composition, and
`160` — the script the meta calls fully converted twice — is the worst of it.**

---

## Meta review 1 — new crate architecture

The recommendation is directionally right and wrong in three specific merges.
I confirmed the category table is the authority, and I found the rules the meta
cites do not say what it says they say.

**`docs/conventions/code.md:13` — "Do not add unused frameworks, duplicate
implementations, or dead code."** A new crate is none of those three. The rule
that bears on an eighteen-crate proposal is not in `code.md` at all: it is
`TODO/INDEX.md:42-58`, where every category names one crate and the `crate`
column is the column that decides a behaviour's owner. **The meta names the
right authority and the wrong rules.** `TODO/RULES.md` contains no rule about
crates, members, or workspace shape — `grep -in 'crate\|member\|workspac' TODO/RULES.md`
returns 0 hits across 121 lines — so the meta's check "against `TODO/RULES.md`"
cannot have been performed.

**What the category table actually decides, per proposed crate.**

| proposed | the category cell its subject lives in | merge verdict |
| --- | --- | --- |
| `podbox-gate` | `gate` → `scripts/` (`INDEX.md:56`) | **endorsed**, with `plant.sh` folded in |
| `podbox-buildstate` | `packaging` → "the artefact" (`INDEX.md:53`) | **endorsed for two scripts only**; `session-start.sh` and `310` are session subjects |
| `podbox-plant` | `gate` → `scripts/` | **fold into `podbox-gate`**; one category, one cell |
| `podbox-verify` | `packaging` → "the artefact", but `110` is `deps` → "the whole tree" (`INDEX.md:52`) | **split**; `deps` having the whole tree as its crate cell is the opposite of a crate |
| `podbox-rebuild` | `packaging` T-1004 and `image` T-0211 — **two categories** | **endorsed as one crate**, `120` + `157`, both mutation-planted store-lock proofs |
| `podbox-podvm` | `podvm` → "the machine tier" (`INDEX.md:55`) | **endorsed**, with `154` |
| `podbox-docs` | `complete` T-1324 and `packaging` — two categories | **endorsed**, one crate, two bins |
| `podbox-dev`, `podbox-session` | `packaging` T-1005 — one category, but `session-start.sh` measures documents and `310` measures startup | **merge into `podbox-gate`**, not into `podbox-buildstate` |
| `podbox-interpose-build` | `packaging` T-1002 | **not a crate**; a bin in `podbox-gate`, beside the `gate.yml:99` step that runs it |
| `podbox-lockrace`, `podbox-ptmx-cover`, `podbox-perf`, `podbox-nsdrive` | `image` T-0215, `enter` T-0503, `gate` T-1338, `probe` T-0111 | **not crates.** Each is one script. `153` → `podbox-image/tests/` or a `podbox-gate` bin; `251` → `podbox-enter`; `360` → the gate, beside `check_perf_budget`; `366` → `podbox-probe` |
| `podbox-reconcile`, `podbox-publish` | `packaging` T-1314 | **endorsed as bins in `podbox-release`**, not as crates |
| `podbox-interpose-check` | `interpose` → `crates/podbox-interpose` | **confirmed excluded**, and the meta's reasoning is right |

**The endorsed list, four crates:**

1. `crates/podbox-gate` — `check-todo.py`, `todo-count.py`, `zig-cc.sh`,
   `dev.sh`, `nightly-smoke.sh`, `398-gate-diagnostics.py`, `plant.sh`, `310`,
   `session-start.sh`, `build-interpose.sh`, `360`. Bins: `podbox-gate`,
   `podbox-count`, `podbox-plant`, `podbox-dev`, `podbox-smoke`.
2. `crates/podbox-buildstate` — `build-state.py`, `release-licenses.py`. Bins:
   `podbox-buildstate`, `podbox-release-licenses`.
3. `crates/podbox-release` — `verify-release.sh`, `110-bloat-delta.sh`,
   `120-reproducible-build.sh`, `157-lock-inheritance-prove.sh`,
   `document-state.py`, `release-notes.sh`, `395`, `399`. Bins: `podbox-verify`,
   `podbox-size`, `podbox-prove-t0211`, `document-state`, `release-notes`,
   `podbox-reconcile`, `podbox-publish`.
4. `crates/podbox-podvm` — `145-podvm-parity.sh`, `154-tcg-workload-spread.sh`.
   Bins: `podbox-podvm`, `podbox-podvm-workload`.

Ten members become **fourteen**, not twenty-eight. Each carries a
`TODO/INDEX.md` category row that already exists, so the meta's claim that a new
category row is needed is wrong for all four.

**`podbox-interpose` and `tests/`: the meta's claim is CONFIRMED, with the
mechanism it did not state.** I built the minimal case rather than citing the
rule. In a scratch crate with `crate-type = ["cdylib"]` and a
`tests/integration.rs` that names the crate, `cargo build --tests` prints:

```
error[E0433]: cannot find module or crate `cdylibprobe` in this scope
 --> tests\integration.rs:2:45
```

`cargo metadata` still lists a `["test"]` target, so the file *is* built; what
fails is the link. A `cdylib` produces no importable Rust library for a test
target to name. Adding `rlib` fixes name resolution — with
`crate-type = ["cdylib", "rlib"]`, `cargo check --tests` is clean. **So the meta
is right that the group-8 commands cannot work, and right that `rlib` fixes
name resolution. Its cost argument is wrong on two counts:**

1. **The cost is not shared resolution.** `crates/podbox-interpose/Cargo.toml:18`
   `[dependencies]` is empty, and it is excluded from the workspace
   (`Cargo.toml:20`), so no `Cargo.lock` entry, feature resolution, or
   dependency unification is shared with anything. Adding `rlib` adds one
   artefact to `crates/podbox-interpose/target/` and nothing else. The meta's
   phrase "a change to the one crate whose whole point is that it depends on
   nothing" describes a coupling that does not exist in the manifest.
2. **The real cost is the release profile.** `crates/podbox-interpose/Cargo.toml:20-25`
   sets `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = "symbols"`,
   `panic = "abort"` for `[profile.release]`. `scripts/build-interpose.sh:126-127`
   builds with `cargo build --release --target <t>`, so an `rlib` in
   `crate-type` is a **release-profile artefact of the shipped interposer**,
   built for both `x86_64-unknown-linux-musl` and `x86_64-unknown-linux-gnu`
   on every `build-interpose.sh` run and on `gate.yml:99`. It does not reach
   the binary (`crates/podbox-cli/build.rs` copies the `.so` only), but it does
   cost build time on the lane and it does change the crate whose byte ceiling
   `scripts/build-interpose.sh:48` measures.

**What a converting entry must do with check A and check B.** Three options, and
the entry picks one and says so: (a) add `rlib` and accept the release-profile
artefact, naming `build-interpose.sh:48` and the `T-1312` ceiling as the
constraint; (b) put the check in the **gate**, as `podbox-gate` reads the built
`.so` with `nm` and compares against `crates/podbox-interpose/interpose.map` —
which `build-interpose.sh:197-214` **already does**, at `:204-205` printing
`ok: exports %s names, exactly what interpose.map declares`; (c) keep the
script. **Option (b) is the right one and the meta did not see it:** the check
the group-8 plan wants to write in Rust already exists in
`scripts/build-interpose.sh:201-214` for both targets, and
`crates/podbox-interpose/interpose.map` declares **112** names today, which I
counted with `build-interpose.sh:199-200`'s own `awk`. Check B (`struct stat`
and `struct statx` offsets under both libcs) still needs a `cc` invocation and
belongs in the same bin.

---

## Meta review 1 — wave corrections

| wave | the meta's boundary | my verdict | the entry that justifies it, or the reason it is not a boundary |
| --- | --- | --- | --- |
| 0 | record fixes first | **CONFIRMED** | `experiments/157-lock-inheritance-prove.sh:157`'s sed pattern versus `crates/podbox-image/src/store.rs:1083`. A plant encoding the current source. **Shared fixture.** |
| 1 | deletions | **CONFIRMED, but incomplete** | Deleting a script whose `Prove` is live is a record-gate change: `scripts/check-todo.py:300` `check_tree` reads every tracked file, so a dangling `Prove` is a red gate. **Record-gate interaction.** But wave 1 lists only 6 scripts and my clause analysis removes 2 of them from the deletable set. |
| 2 | inline pure gaps | **CONFIRMED** | Every target is an existing `mod tests`; `crates/podbox-cli/src/images.rs` for the `160` CLI gap, `crates/podbox-probe/src/exit.rs` for `330`. **No new directory, no new crate.** |
| 3 | the interposer `WANT` string | **REJECT as a boundary.** Merge into wave 2 | `write.rs:643` already builds the temp rootfs the meta says the wave would have to gather. The stated reason is fixture grouping; there is no fixture to group. **Convenience.** |
| 4 | first `tests/` directories | **CONFIRMED, and understated** | `cargo metadata` shows `podbox-cli` has targets `['bin','custom-build']` and **no `lib`**. Every `podbox-cli/tests/*.rs` in this wave needs a `CARGO_BIN_EXE_podbox` driver or a new `[lib]` target. **Shared structural change**, larger than the meta states. |
| 4's condition | `store_digest` blocked without a Rust registry fixture | **CONFIRMED and corrected** | `crates/podbox-image/src/registry.rs:955` `serve_once` is inside `mod tests` at `:949` — it is **test-private**, answers one request with one body, and is unreachable from any binary. `TODO/image.md:483` gives T-0206's Prove as `./experiments/180-registry-fixture.sh`. **No Rust registry fixture exists.** The wave-4 block is real. |
| 5 | `crates/podbox-gate` | **CONFIRMED, and it must absorb wave 8** | `scripts/check-todo.py:228` `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` and `scripts/plant.sh:51` `FILES` list the same path. Moving either alone turns the gate red. **Record-gate interaction.** |
| 6 | `podbox-buildstate` + `podbox-dev` | **REJECT as a boundary.** Merge into 5 | Two crates, no shared code, no shared fixture, no record-gate interaction. The meta's reason is an absence. **Convenience.** |
| 7 | the release and build tools | **REJECT as a boundary.** Merge into 5 | Nine proposed crates with no shared code. The stated reason is a shared *shape* (a three-state binary), which is not a shared crate, fixture, or edge. **Convenience.** Split as endorsed above: `podbox-buildstate`, `podbox-release`, `podbox-podvm` sit beside `podbox-gate`. |
| 8 | `plant.sh` last | **CONFIRMED as a boundary; the reason is wrong** | `gate.yml:55-56` runs `./scripts/plant.sh` as the step named `every check can fail`, and `scripts/check-todo.py:300` `check_tree` reads it as a tracked file. **I counted 13 `Prove` lines naming it** across `TODO/cli.md`, `TODO/deps.md` and `TODO/gate.md`; the meta says 11 in one place and 14 in another. The meta's reason is "two instruments at once", which is a preference. The reason is the record gate. |
| **all waves** | two gate commands | **CORRECTED** | `gate.yml` runs four jobs. `todo` (`:36-37`, `:41`, `:55-56`, `:58`), `build` (`:99`, `:102`, `:131`), `lint` (`:154`, `:159`, `:164`, `:171`, `:176`, `:186`), `test` (`:199`, `:213`). The meta names only the two `test`-job commands. Waves 5–8 change the build and the gate, so `cargo test -p <crate>` does not prove them. `docs/methodology/gate.md:23` names the full set. |
| **all waves** | the two line citations | **CORRECTED** | `gate.yml:199` is `cargo test --workspace`; `gate.yml:213` is the interposer command. The meta cites `:197` (blank) and `:211` (the `env:` key). The commands are right, the lines are off — the same off-by-a-couple pattern the meta found in two group reports and did not check in itself. |

**Six waves survive, not nine:** 0 (record fixes), 1 (deletions), 2 (inline
tests, absorbing the old 3), 4 (`tests/` directories), 5 (`podbox-gate`,
absorbing the old 6, 7 and 8), and the `podbox-buildstate` / `podbox-release` /
`podbox-podvm` set beside it.

---

## Meta review 1 — corrections required before task entries

1. **Build every entry from the ten group bodies, never from a header table.**
   Six groups' headers disagree with their bodies. I reproduce the meta's body
   counts exactly: KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20, SPLIT 35, DELETE
   12 = 121, and the after-VC distribution KEEP-SHELL 29, RUST-TOOL 27, SPLIT
   36, RUST-TEST 18, DELETE 11 = 121.
2. **Delete the "both sets total 121" sentence.** The header tables total 112,
   not 121. The per-group totals match, which is why the defect survives a sum
   check; the label totals do not. Group 6's table is 14 against a 13 body and
   its own closing line; group 4 carries two headers with two answers.
3. **Correct group 4's verdict for `95-podman-vfs-ignorechown.sh` to KEEP-SHELL**
   before any entry is written. Round 2 VC-3 settles it, and the meta's
   proposal table agrees while the header at `group-4.md:32` still reads
   RUST-TEST. The entry must read the body.
4. **Stop calling `160-store-gc.sh` converted.** It is 3 of 5. Two tests must be
   written in `crates/podbox-cli/src/images.rs`: `rmi` exits non-zero with
   `in use` on a held image, and `image prune` exits 0, prints `skipped:`, and
   leaves the image listed. `experiments/results/store-gc.txt` records
   `rmi_under_holder 125` and `prune_skipped 1`. A test written to the
   plan's assertion fails on `prune`, because `images.rs:961-986` returns 0.
5. **Stop calling `70-whiteout-contract.sh` deletable.** Check B is a
   measurement over a real `alpine` layer and no Rust test reads one.
   `drive.rs:480` asserts the **opposite** outcome on a crafted layer. Either a
   fixture lands first or `70` stays.
6. **Correct the conversion table for `90-nsswitch-contract.sh`.** Checks B, C
   and D are unconverted and check A's live form is unconverted;
   `identity.rs:336` drives the editor, not the NSS dispatcher. `90` is not
   deletable.
7. **Correct the conversion table for `149-podvm-non-goals.sh`.** Clause 6
   shells to `361-guest-usernet.sh` with a 1500-second bound and has no Rust
   arm. The meta is right that the report test at `report.rs:1105-1107` exists;
   it is wrong that only the six-names constant remains.
8. **Correct the conversion table for `80-interposer-abi.sh`.** Check B is four
   `LD_PRELOAD` pairings against live loaders and cannot be a predicate. Check
   E has no test at all: `crates/podbox-cli/src/system.rs`'s `mod tests` at
   `:603` holds 7 tests and none names `abi`. A converting entry writes the
   three exit codes of `system::abi` (`:191-214`) and keeps check B in shell.
9. **Correct the conversion table for `170-probe-cache.sh`.** Clause 1 is two
   `podbox probe --json` runs compared with `jq` and has no Rust arm. Clause 3's
   engine half stays shell.
10. **Record `220-extract-path-safety.sh` as fully converted, and name the
    tests.** The tests that carry its six checks are in
    `crates/podbox-extract/src/drive.rs` (`#[cfg(test)] mod drive;` at
    `crates/podbox-extract/src/lib.rs:40`), at `:167`, `:211`, `:229`, `:264`,
    `:382`, and the filesystem checks at `:221`, `:236`, `:253`. The meta cites
    only `safety.rs` and `drive.rs:382` and calls the script 5 of 6.
11. **State the `podbox-cli` structural change wherever a `podbox-cli/tests/`
    file appears.** `cargo metadata` reports the package's targets as
    `['bin','custom-build']` with **no `lib`**. Each such file is either a
    `CARGO_BIN_EXE_podbox` black-box driver or requires a new `[lib]`. The meta
    calls this one structural change; it is two.
12. **Correct `crates/podbox-image/src/registry.rs:955`.** `serve_once` is a
    `fn` inside `mod tests` at `:949`, not a fixture any binary can reach. It
    answers one request with one body over plain HTTP/1.0. There is no Rust OCI
    registry fixture in the tree, and `TODO/image.md:483` names T-0206's Prove
    as `./experiments/180-registry-fixture.sh`.
13. **Adopt the four-crate list, with the composition corrected.** `podbox-gate`
    (including `plant.sh`, `dev.sh`, `310`, `session-start.sh`, `360`,
    `build-interpose.sh`), `podbox-buildstate` (two scripts only),
    `podbox-release` (`110` as a bin, not a library), `podbox-podvm`. Ten
    members become fourteen. The eighteen names include ten that are binaries
    with a home, not subjects.
14. **Do not cite `TODO/RULES.md` for the crate question.** It has no rule about
    crates, members, or workspace shape; `grep -in 'crate\|member\|workspac'`
    returns 0 hits in 121 lines. The authority is `TODO/INDEX.md:42-58`, and all
    four endorsed crates already have a category row.
15. **Reject the `podbox-interpose/tests/*.rs` plan, and route check A to the
    gate.** `crate-type = ["cdylib"]` at
    `crates/podbox-interpose/Cargo.toml:15` gives `error[E0433]: cannot find
    module or crate` in a `tests/*.rs` file; adding `rlib` fixes name
    resolution. But `scripts/build-interpose.sh:197-214` **already** performs
    check A on both targets, comparing `nm -D` exports against
    `crates/podbox-interpose/interpose.map` (112 names, counted with that
    script's own `awk`). Check A needs no new test. Check B does, and belongs in
    the same bin. If `rlib` is added, the entry names the
    `[profile.release]` cost: the artefact is built for both targets on every
    `build-interpose.sh` run and on `gate.yml:99`, under the byte ceiling at
    `build-interpose.sh:48`.
16. **Give every wave the full gate, not two commands.** `gate.yml` runs four
    jobs. A wave that touches a manifest, a script, or a document is gated by
    `todo`, `build` and `lint` as well as `test`:
    `scripts/check-todo.py`, `scripts/todo-count.py`, `./scripts/plant.sh`,
    `./scripts/build-interpose.sh`, `cargo build --release --target
    x86_64-unknown-linux-musl`, `./experiments/110-bloat-delta.sh ci`,
    `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt`,
    `py_compile`. `docs/methodology/gate.md:23` names the set.
17. **Correct the gate line citations.** `gate.yml:199` is
    `cargo test --workspace`; `gate.yml:213` is the interposer command. The
    meta cites `:197` and `:211`.
18. **Correct the `plant.sh` `Prove` count to 13, and use one number.**
    `grep -rn 'scripts/plant.sh' TODO/*.md | grep -c 'Prove'` returns 13, across
    `TODO/cli.md`, `TODO/deps.md` and `TODO/gate.md`. The meta says 11 in the
    waves section and 14 at its proposal row.
19. **Collapse nine waves to six.** Waves 2+3, and 6+7+8, are separated by
    convenience, not by a shared crate, fixture, dependency edge, or
    record-gate interaction. The surviving boundaries are the `157` mutation
    anchor, the tracked-file citation check, the first `tests/` directories, and
    the `plant.sh` CI step.
20. **Correct the per-crate file counts in the meta's own table.** `podbox-enter`
    has 10 `src` files and `podbox-complete` has 8, not 11 and 9. The `137`
    total should be 135 counting `build.rs`, or 126 counting `src` alone. No
    total changes; the two rows do.
21. **Re-open any line citation that was inferred rather than read.** The
    meta's own pattern statement — "correct wherever the report read the file and
    wrong wherever it inferred a line from a subject name" — applies to the
    meta. Its `gate.yml` citations are inferred and wrong. I verified the ones
    I could: `registry.rs:955`, `space.rs:246`/`:263`/`:238`,
    `space.rs:299`, `contain.rs:118`, `store.rs:1950`/`:1976`/`:2019`/`:2114`
    are all correct.

---

## Meta review 1 — confidence statement

**Settled, and a downstream writer may build on it without re-checking.**

The test surface. 980 test functions, 950 inline and 30 in `tests/`, by three
independent counts over three file sets. 117 `#[cfg(test)]` modules, 116 named
`tests`. Zero `#[tokio::test]`, zero `#[ignore]`, zero `no_run`, zero
`[dev-dependencies]`, zero `tempfile` in any manifest or in `Cargo.lock`. One
`tests/` directory. These are the constraints every conversion entry writes
against, and they are the meta report's strongest work: it counted, named its
commands, found the one false positive in its own `#[cfg(test)]` total, and
reproduced exactly.

The verdict distribution. The ten group bodies total 121 and my independent
count reproduces the meta's per-group and grand totals digit for digit:
KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20, SPLIT 35, DELETE 12 before VC; 29,
27, 36, 18, 11 after. Six groups' headers disagree with their bodies, and the
meta named the right six. **Build every entry from the bodies.**

The parity count. 265, by four methods inside `parity.rs:232-537` and by two
outside. The meta's overrule of round 1 is **correct**: round 1 and group 5
counted `Row {` over the whole file and swept in `pub struct Row` at `:58` and
`impl Row` at `:65`, which is 267. `TODO/cli.md:63` reads 220, and
`experiments/results/parity-drive.txt:7` reads 164. All four are wrong for
today. The meta corrected round 1 on this and was right to.

**`crates/podbox-interpose` cannot host `tests/*.rs` as configured.** The
meta's claim is right, and I settled it with a build rather than a rule:
`error[E0433]: cannot find module or crate` in the test file, with the test
target itself still listed by `cargo metadata`. But the meta's **cost argument
is wrong**, and an entry that repeats it will repeat a fiction: the crate is
excluded from the workspace with an empty `[dependencies]`, so there is no
shared resolution to disturb. The real cost is a second release-profile
artefact on the one crate `build-interpose.sh` builds for two targets under a
byte ceiling. And the check the plan wanted to write **already exists**, at
`scripts/build-interpose.sh:197-214`, for both targets.

**Load-bearing on claims that are not yet settled.**

The conversions. **Of the eight scripts the meta calls fully or mostly
converted, one is genuinely converted.** `388-interactive-shell.sh` is, clause
for clause, all eleven. `220-extract-path-safety.sh` is, all six, in
`crates/podbox-extract/src/drive.rs` — which the meta under-cited rather than
over-cited. The other six are partial, and the one that decides entries is
`160-store-gc.sh`: the meta says its five clauses are all in Rust in two
places and says one is not in a third, in the same document. Three of the eight
(`70`, `90`, `149`) have a clause that is a live measurement with no predicate
form, so **no entry for them may say "delete the script"**.

The registry fixture. `serve_once` is test-private, one request, plain HTTP.
There is no Rust OCI registry fixture, and `store_digest.rs` is blocked until
one exists. This is load-bearing for wave 4 and the meta is right.

The crate count. Four crates is the right shape and the wrong composition.
`podbox-plant` and `podbox-gate` are one category cell. `podbox-verify` as the
meta draws it puts six subjects behind one name and puts `110-bloat-delta.sh`
in `packaging` when the `deps` category's `crate` cell is "the whole tree",
which is the opposite of a crate. Ten of the eighteen names are binaries with
a home, not crates.

The wave structure. **Three of the nine boundaries are convenience**, stated as
such by the meta in two of the three cases, and the boundaries that are real
are real for reasons the meta names correctly: the `157` mutation anchor, the
`check-todo.py:300` tracked-file scan, the `plant.sh` CI step, and the first
`tests/` directories. Nine becomes six.

**What a downstream writer may now treat as settled.** The counting method and
the test surface. The body verdicts and the after-VC distribution. The 265.
The absence of a Rust OCI registry fixture. The `cdylib` link rule. The absence
of dev-dependencies and of `tempfile`, and therefore the rule that any fixture
a plan names is a hand-rolled `std::env::temp_dir()` with a pid or counter
suffix.

**What remains load-bearing on something I could not verify.** Every claim
about what compiles and what passes. This host has no C linker: `cargo build`
stops at `error: linker 'cc' not found`, so I ran no test and observed no pass.
980 is a count of attributes, not of executed tests. The 265 is a count of
`Row {` occurrences in a range, not of rows cargo would accept. A converting
entry's proof must be run on the Linux base lane through
`sh scripts/windows/run-in-base.sh` before it is closed, and until then every
"this works" in this corpus is a claim about a file rather than about a
binary.

**Bluntest line.** The meta report counted the Rust tree better than any round
before it and then read its own conversion list as a plan when it is a list of
partials: of the eight scripts it calls converted, one is, and the one it calls
converted most often, `160-store-gc.sh`, is the one whose own proposal table
says the fifth clause is not written.
