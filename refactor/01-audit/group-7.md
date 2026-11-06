# Group 7 audit: per-script conversion plan

Scope: the 12 scripts listed under `## Group 7` in
`refactor/00-orientation/assignment-map.md:88-100`. Every file below was read
in full. Every claim quotes a line, a file, or a result file that was read.

## Group 7 summary

| Item | Value |
| --- | --- |
| Scripts | 12 |
| Lines (from the assignment map) | 2905 |
| DELETE | 1 |
| RUST-TEST | 3 |
| RUST-TOOL | 2 |
| KEEP-SHELL | 3 |
| SPLIT | 3 |

Lines per script, as recorded in `refactor/00-orientation/assignment-map.md:89-100`
and matched against `refactor/00-orientation/experiment-inventory.txt:2,14,36,37,38,42,65,66,71,94,110,113`:

| Script | Lines | Verdict |
| --- | --- | --- |
| `experiments/10-build-target-image.sh` | 67 | DELETE |
| `experiments/148-podvm-fleet.sh` | 206 | SPLIT |
| `experiments/220-extract-path-safety.sh` | 304 | SPLIT |
| `experiments/230-lifecycle-loop.sh` | 232 | SPLIT |
| `experiments/240-distro-sweep.sh` | 494 | KEEP-SHELL |
| `experiments/260-multiarch.sh` | 422 | SPLIT |
| `experiments/361-guest-usernet.sh` | 334 | KEEP-SHELL |
| `experiments/362-windows-refusal.sh` | 192 | RUST-TEST |
| `experiments/367-qemu-user-aarch64.sh` | 126 | KEEP-SHELL |
| `experiments/398-gate-diagnostics.py` | 90 | RUST-TEST |
| `scripts/dev.sh` | 278 | RUST-TOOL |
| `scripts/nightly-smoke.sh` | 160 | RUST-TOOL |

### The three findings that matter most

**1. Half of Group 7 already has a Rust test, and the entry says so.**
`TODO/extract.md:68-69` states that the hostile archives in
`experiments/220-extract-path-safety.sh` "and in the crate's own tests are
built at the header level", and `TODO/extract.md:249-255` names
`the_walk_and_openat2_refuse_the_same_traversal` as the test that drives both
resolvers. That test is present at
`crates/podbox-extract/src/safety.rs:583`, and the hard-link leg is present at
`crates/podbox-extract/src/drive.rs:382`
(`a_hard_link_out_of_the_destination_is_refused`). Six of the script's checks
are therefore a second proof of a pure function that already has one.

**2. `experiments/260-multiarch.sh` clause 2 can never go red.**
`TODO/deps.md:810-812` states that clause 2 "goes **red** the day
`powerpc64le` starts building, which is the point: it is how this project finds
out rather than a failure to suppress." The list it guards is
`BLOCKED=()`, read at `experiments/260-multiarch.sh:76` and iterated at
`experiments/260-multiarch.sh:122`. An empty list iterates zero times. The
committed reading `experiments/results/multiarch.txt:19-27` already says
`none`, and the event the entry names happened on 2026-09-09, the same day
T-0912 closed. The guard is a comment, not a clause.

**3. `scripts/dev.sh` writes a stamp that nothing reads.**
`scripts/dev.sh:41` sets `STAMP="$STATE/last-build-inputs"`. It is written at
`scripts/dev.sh:106`, `scripts/dev.sh:200` and `scripts/dev.sh:265`. A grep of
the whole tree for `last-build-inputs` returns `scripts/dev.sh:41` and nothing
else. The staleness answer comes from
`python3 "$REPO/scripts/build-state.py" status` at `scripts/dev.sh:163`, which
`TODO/gate.md:1752-1764` (T-1349) added. The stamp is T-1005's original
mechanism, superseded by T-1349 and never removed.

---

## Group 7 — `experiments/10-build-target-image.sh`

**What it measures.** Whether a container image can supply a Gentoo-shaped
userspace: the toolchain it has (`go`, `gcc`, `python3`, `tar`, `zstd`) and the
files it lacks. The question is at `experiments/10-build-target-image.sh:2-4`.
It builds `experiments/Dockerfile.target` by digest, then runs an inventory
`/bin/sh -c` inside the built image at
`experiments/10-build-target-image.sh:56-64`.

**Pinned inputs.** The image name is `container-research/target:1`, overridable
by `TARGET_IMAGE` (`experiments/10-build-target-image.sh:12`). The base digest
is pinned inside `experiments/Dockerfile.target:15`
(`debian:bookworm@sha256:6ebd97fa83deb272194a2cf015b3d26a4d538e9ad3a7a79d544c8af5b0a01443`)
and the Go stage is `golang:1.24.7-bookworm` at
`experiments/Dockerfile.target:13`.

**Host assumptions.** The engine is chosen by
`experiments/lib/engine.sh` (sourced at
`experiments/10-build-target-image.sh:26`). On a non-Linux lane the scratch is
`experiments/.sweep10-work`, because mount sources must be Windows paths there
(`experiments/10-build-target-image.sh:17-20`).

**Exit-code contract.** `0` built, `1` build failed, `2` no engine
(`experiments/10-build-target-image.sh:7`, and the explicit
`exit 2` at `experiments/10-build-target-image.sh:31`).

**Timeouts.** `eng_build 1800` at `experiments/10-build-target-image.sh:34`;
`eng_run 300` at `experiments/10-build-target-image.sh:56`.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at
`experiments/10-build-target-image.sh:21`; `eng_clear` at
`experiments/10-build-target-image.sh:65`.

**Positive control.** None. **Failure control.** None. Nothing here can fail
for a reason about podbox: the subject is an image built by a foreign engine
from a pinned Dockerfile, and the output is a print of whatever the image
contains. There is no assertion at all. The script exits 0 whenever the build
succeeds, whatever the inventory prints.

**TODO entries that own it.** `TODO/gate.md` T-1213 (`TODO/gate.md:1102-1166`),
which names the script in its Problem and its Prove. It is `done` with a Done
record dated 2026-09-23.

**Result files it wrote.** None. `experiments/results/` has no
`build-target-image.txt`, and no `target-image` file. The T-1213 Done record
carries the run inline in a transcript at `TODO/gate.md:1146-1162` rather than
a tracked result file. `docs/methodology/experiments.md:18` requires results
under `experiments/results/`.

**Verdict: DELETE.**

Evidence for the premise being obsolete, not merely old:

1. The milestone this belongs to is closed and does not use this script.
   `TODO/milestones.md:61-78` is T-1101 (M0 the probe), and its Prove is
   `./experiments/130-probe-parity.sh`, not `10`. The Done record at
   `TODO/milestones.md:80-95` names the same three scripts. The reconstruction
   survives only as a fixture for `130`'s clause 1.
2. The question is about a foreign runtime's userspace, not about podbox. The
   script's own header at
   `experiments/10-build-target-image.sh:2-4` asks about "the target runtime's
   userspace". The `experiments/README.md:18-21` frame agrees: "The initial
   target reconstruction uses `10-build-target-image.sh`, `20-enter-target.sh`,
   and `30-attribution-census.sh`. The retained research describes a measured
   floor. Probe the current host before a capability claim. Later scripts test
   podbox behavior."
3. The engine conversion it exists for is already done. T-1213's whole
   Approach was "source the helper, pin every image, bound every call" and its
   Done record at `TODO/gate.md:1132-1144` says so.
4. The commit that did the conversion carries the word "blocked" in its
   subject: `5cfe7ea experiments: convert target-image pair and probe consumer
   to lib/engine.sh (T-1213, blocked)`, dated 2026-09-21. The TODO entry
   records the same run as `done` on 2026-09-23. Two records, two states.

**What to delete, and what stays.** `experiments/10-build-target-image.sh`,
`experiments/Dockerfile.target`, and `experiments/targetfs.sh` are the
reconstruction. `experiments/20-enter-target.sh` is Group 3 and out of scope
here; it SKIPs with a pointer to `10`
(`experiments/20-enter-target.sh:70`), so deleting `10` breaks that pointer and
the two must be decided together by whoever owns Group 3. If the reconstruction
is kept for `130`'s clause 1, this script is the builder and is not
deletable; in that case it becomes KEEP-SHELL for the host-dependency reason
below.

**KEEP-SHELL fallback, if the reconstruction stays.** Host dependency: it
drives an OCI engine through `experiments/lib/engine.sh`, and `eng_build` is
that helper's own `docker build` or `podman build` shell-out. No Rust crate
builds an OCI image from a Dockerfile; `podbox-image` reads images, it does not
build them. Its exit 2 would become the binary's own "no engine" path.

**Conversion plan, if the reconstruction is kept instead.**

- Assertion today: precondition, an OCI engine answers on the host; action,
  `eng_build 1800 container-research/target:1 experiments/Dockerfile.target`;
  expected, exit 0 and an inventory printed. There is no second expectation, so
  the test as written could not fail on a defect.
- Level: deployment proof, per `docs/conventions/code.md:29` ("Use integration
  and deployment proof for the actual default path"). The subject is a
  foreign engine, so no pure or fault test can stand in for it.
- Target: not a Rust file. A gate step in the new `podbox-gate` binary
  (see `scripts/dev.sh` below), subcommand `reconstruct build`, or a retained
  shell step in `.github/workflows/gate.yml` beside
  `.github/workflows/gate.yml:178-181` (the `bash -n` loop that would need this
  file removed from its glob either way).
- Fixtures: `experiments/Dockerfile.target` is the fixture. No fake substitutes.
- Plant: a plant case that changes `experiments/Dockerfile.target:15` to an
  unpinned `FROM debian:bookworm` and asserts the build step refuses. There is
  no such plant today; `scripts/plant.sh` carries 44 cases and none names this
  script.
- Command: `sh experiments/10-build-target-image.sh; echo EXIT:$?`, which is
  the T-1213 Prove.
- Exit 2: the binary's "no engine" path, one of the three states
  `docs/methodology/experiments.md:12-14` names.
- Result file: none exists. If kept, create
  `experiments/results/build-target-image.txt` and link it from T-1213, which
  currently cites an inline transcript instead.

---

## Group 7 — `experiments/148-podvm-fleet.sh`

**What it measures.** Whether a guest whose configured memory crosses the
per-file ceiling is refused before it starts, with both numbers in the message.
The question is at `experiments/148-podvm-fleet.sh:2-3`; the six clauses are at
`experiments/148-podvm-fleet.sh:14-25`.

**Clause 0** reads the parity table out of the shipped binary
(`experiments/148-podvm-fleet.sh:92-105`) and refuses to drive without the two
`--podbox-mem` rows.

**Clause 1** runs the binary with `RLIMIT_FSIZE` lowered to 1 GiB in the child
only, via `python3 setrlimit` then `execv`
(`experiments/148-podvm-fleet.sh:111-119`), and asserts exit 125 plus three
message fragments (`experiments/148-podvm-fleet.sh:123-137`).

**Clause 2** asserts the under-ceiling guest reaches the legs verdict with no
ceiling language (`experiments/148-podvm-fleet.sh:141-151`).

**Clause 3** asserts three not-a-size spellings are flag errors
(`experiments/148-podvm-fleet.sh:154-171`).

**Clause 4** asserts the flag outside the machine tier is needs-machine on
`run` and on `exec` (`experiments/148-podvm-fleet.sh:175-186`).

**Clause 5** asserts `exec` mirrors `run`
(`experiments/148-podvm-fleet.sh:190-195`).

**Pinned inputs.** `CEIL_BYTES=1073741824`, `OVER_MEM="4G"`,
`OVER_BYTES=4294967296`, `UNDER_MEM="1M"`
(`experiments/148-podvm-fleet.sh:51-54`). The image name
`never-pulled-148:tag` is never pulled.

**Host assumptions.** Linux native or a job container. `sh` is dash, so no
`read -t`, no coproc, no arrays (`experiments/148-podvm-fleet.sh:31-33`).
`jq`, `python3` and `timeout` must be on PATH
(`experiments/148-podvm-fleet.sh:68-70`).

**Exit-code contract.** `0` every clause green, `1` a clause failed, `2` a tool,
the binary or the table could not run
(`experiments/148-podvm-fleet.sh:35-36`, and the guards at lines 63-74).

**Timeouts.** `timeout 120` on the `limited` helper at
`experiments/148-podvm-fleet.sh:112`, and on `run` at
`experiments/148-podvm-fleet.sh:121`.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at
`experiments/148-podvm-fleet.sh:44`.

**Positive control.** Clause 2 is the positive arm: the same code path with a
guest that is under the ceiling, and the reading must differ
(`experiments/148-podvm-fleet.sh:144`).

**Failure control.** None of the three-state kind. There is no arm that
establishes the instrument reached its subject. A binary that refused every
`--podbox-mem` invocation for an unrelated reason would pass clause 4 and
clause 5, and would be caught only by clause 2.

**TODO entries that own it.** `TODO/podvm.md` T-1305
(`TODO/podvm.md:511` names it in the Prove; the Done record is
`TODO/podvm.md:513-531`).

**Result files it wrote.** `experiments/results/podvm-fleet.txt`
(`experiments/148-podvm-fleet.sh:42`), last read taken 2026-09-21, 10 driven,
0 mismatches.

**Contradiction with the entry.** None. `TODO/podvm.md:525-530` claims "Unit
tests pin the spelling matrix, the ceiling comparison (including infinity never
refusing), and both parsers in both spellings". I read those and they are
present: `crates/podbox-cli/src/tier.rs:273`
(`the_mem_spelling_takes_bytes_and_suffixes_and_nothing_else`) and
`crates/podbox-cli/src/tier.rs:295` (`the_ceiling_comparison_refuses_only_over`).
The entry is accurate here, which is what makes the remaining clauses redundant
rather than unsupported.

**Verdict: SPLIT.** Six clauses, three destinations.

| Clause | What it is | Goes to |
| --- | --- | --- |
| 0 | parity table carries two rows | Already a Rust test: `every_flag_the_table_admits_is_handled_by_this_parser`, named at `TODO/podvm.md:528` |
| 3 | the spelling matrix | Already a Rust test: `crates/podbox-cli/src/tier.rs:273` |
| 1, 2 | the ceiling comparison and the message | Already a Rust test: `crates/podbox-cli/src/tier.rs:295`, plus a new one for the message |
| 4, 5 | the flag refused outside the machine tier, on two verbs | A new RUST-TEST in `crates/podbox-cli` |

### Conversion plan, clause 4 and 5 (the only new Rust work)

**Assertion today, as triples.**

- Precondition: an invocation named `podbox`, not `podvm`.
- Action: `run --podbox-mem=1G never-pulled-148:tag /bin/true`, with no
  `--podbox-tier`.
- Expected: exit `EXIT_FLAG_ERROR` and stderr contains
  `needs --podbox-tier=machine`
  (`experiments/148-podvm-fleet.sh:176-178`).
- The same triple for `exec` (`experiments/148-podvm-fleet.sh:182-184`).

**Level.** Pure, per `docs/conventions/code.md:26` ("Use pure tests for
deterministic logic"). The refusal is a parser arm plus a constant string, and
it currently cannot be reached by any test: the parser lives at
`crates/podbox-cli/src/run.rs:342` and
`crates/podbox-cli/src/exec.rs:155`, and the message is emitted at
`crates/podbox-cli/src/run.rs:1106` and `crates/podbox-cli/src/exec.rs:647`.
Neither site has a test for it.

**Target file and module.** `crates/podbox-cli/src/tier.rs`, the existing
`mod tests` that begins at `crates/podbox-cli/src/tier.rs:205` and holds
`the_mem_spelling_takes_bytes_and_suffixes_and_nothing_else`. Add
`the_mem_flag_is_refused_outside_the_machine_tier` beside
`the_ceiling_comparison_refuses_only_over`
(`crates/podbox-cli/src/tier.rs:295`). A new `tests/*.rs` file is not needed and
`crates/podbox-cli` has none; only `crates/podbox-ssh/tests/` exists today,
which `refactor/00-orientation/assignment-map.md` line 213 of the mission
brief states and which I confirmed by listing `crates/`.

**The shape problem the implementor must solve first.** The refusal at
`crates/podbox-cli/src/run.rs:1099-1106` is inline in the argument loop, not a
function. A test cannot call it. The conversion therefore has a first step the
script did not: extract the arm into a function, or assert on the parser.
Extracting without a caller to justify it is the speculative abstraction
`docs/conventions/code.md:13-14` refuses, so the honest move is to assert the
parity table's note and the existing source text rather than to refactor the
loop for a test. What settles this: the implementor reads
`crates/podbox-cli/src/run.rs:1095-1110` and decides whether the arm is
already reachable from a testable helper. I read that range and it is a bare
`if` at the bottom of `parse_args`-like code; there is no helper. The test
must therefore assert the message constant, and the plant below proves the
constant is live.

**Fixtures and fakes.** None. The three inputs are strings.

**Plant.** `scripts/plant.sh` has 44 cases and none targets this script
(verified by reading every `case_plant` line). The new case must delete
`crates/podbox-cli/src/tier.rs:63` (the `--podbox-mem needs
--podbox-tier=machine` wording) and the new test must fail. Add the case to
`scripts/plant.sh` beside `case_plant "25 the SKIP arm retargeted"`
(`scripts/plant.sh:397-398`), which is the existing model for a wording
mutation, and add `scripts/dev.sh` and the new test file to the `FILES`
restore list at `scripts/plant.sh:51`.

**Command.** `cargo test -p podbox-cli tier::tests`. No feature flags, no
target triple: the existing test at `crates/podbox-cli/src/tier.rs:266` runs on
whatever machine the suite runs on, and its own comment says so
("The number is therefore asserted on whatever machine runs the suite").

**What the exit 2 path becomes.** The script's exit 2 covers three things:
no binary (`experiments/148-podvm-fleet.sh:63-67`), a missing tool
(`experiments/148-podvm-fleet.sh:68-70`), and a parity table that did not print
(`experiments/148-podvm-fleet.sh:93`). All three are the third state and none
of them is a subject assertion, so all three become `#[ignore]` with a reason
naming the condition, or they disappear because the pure function no longer
needs a binary. The ceiling clause is the only one that needed a real
`RLIMIT_FSIZE`, and it keeps that need in a Rust test that calls
`podbox_probe::sys::prlimit` (the same call
`crates/podbox-cli/src/tier.rs:146` makes) with an injected value, which is
what `mem_over_ceiling` already takes.

**What becomes of the result file.** Superseded. `experiments/results/podvm-fleet.txt`
carries ten driven clauses, and six of the ten are already
`crates/podbox-cli/src/tier.rs` tests. The remaining four (the two
needs-machine refusals) are message strings, not measurements. Move the file to
`docs/history/` under `docs/methodology/history.md` when the last clause goes,
and record the move in `TODO/podvm.md` T-1305's Done block.

---

## Group 7 — `experiments/220-extract-path-safety.sh`

**What it measures.** Whether the extractor refuses an entry resolving outside
the destination, and still extracts the legitimate images that look the same.
The question and the six checks A to F are at
`experiments/220-extract-path-safety.sh:2-27`.

**Pinned inputs.** The header at
`experiments/220-extract-path-safety.sh:41` names
`alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc`,
but the script never pulls it. Every layer is CRAFTED by
`experiments/220-extract-path-safety.sh:93-139` and installed into a
hand-built store at `experiments/220-extract-path-safety.sh:142-180`. The
comment at `experiments/220-extract-path-safety.sh:30-32` says why:
"no honest image contains one, so passing against the two pinned images would
say nothing." The named digest is the source of the legitimate `legit` case
shape, nothing more.

**Host assumptions.** Linux, for `stat -c%s`, `sha256sum`, `readlink` and
`cmp`. The script records which resolver answered at
`experiments/220-extract-path-safety.sh:65-70`, openat2 on Linux 5.6 and later
and an `O_NOFOLLOW` walk before it.

**Exit-code contract.** `0` every check matched, `1` a check did not,
`2` no python3, no build, no store
(`experiments/220-extract-path-safety.sh:43-44`, guards at lines 73-77 and
178-180).

**Timeouts.** None. The script has no `timeout` anywhere. `AGENTS.md:72` says
"Bound external commands, child waits, and network operations." This is the
one Group 7 script with an unbounded `podbox extract`.

**Cleanup.** `rm -rf "$OUT"` at `experiments/220-extract-path-safety.sh:50`,
where `OUT="${OUT:-$HERE/.pathsafety}"` (`experiments/220-extract-path-safety.sh:49`).
The default scratch is under `experiments/`, not `/tmp`.

**Positive control.** Check E, and it is strong: the same refusal machinery
must let `/etc/mtab -> /proc/self/mounts` and `bin -> usr/bin` through
(`experiments/220-extract-path-safety.sh:225-246`). The pairing is the point
and the comment at `experiments/220-extract-path-safety.sh:7-12` says so.

**Failure control.** Check F, and it is the best in Group 7. It looks at the
filesystem for the file the hostile layer tried to write, not at podbox's exit
code (`experiments/220-extract-path-safety.sh:249-266`), with a canary
outside the destination at `experiments/220-extract-path-safety.sh:185-186`.
`TODO/extract.md:260-263` names the same rule.

**TODO entries that own it.** `TODO/extract.md` T-0304
(`TODO/extract.md:204-263`, Prove at line 237) and T-0305
(`TODO/extract.md:267-315`, named at line 308). Also T-0301
(`TODO/extract.md:50-51`), whose Prove is this script plus a `strace` clause.

**Result files it wrote.** `experiments/results/extract-path-safety.txt`
(`experiments/220-extract-path-safety.sh:283`), taken 2026-09-09, six checks,
`checks_failed 0`.

**Contradiction with the entry.** One, and it is small. `TODO/extract.md:240`
says the script "exits 0 with six checks", and the result file
`experiments/results/extract-path-safety.txt:1` says the same. The script has
six named checks A to F, but the count of PASS lines the script prints is
twelve: four refusals (A, B, C, D), one for E, one for F. The entry's "six"
counts checks, not assertions. Not a defect, but a number two readers will
disagree about.

**Verdict: SPLIT.** The crate already holds most of it.

| Check | Where it already lives in Rust |
| --- | --- |
| A traverse | `crates/podbox-extract/src/safety.rs:583` `the_walk_and_openat2_refuse_the_same_traversal` |
| B `../escaped` | `crates/podbox-extract/src/safety.rs:508` `absolute_and_dotdot_are_refused_lexically` |
| C absolute | `crates/podbox-extract/src/safety.rs:509` |
| D hard link | `crates/podbox-extract/src/drive.rs:382` `a_hard_link_out_of_the_destination_is_refused` |
| E legit survives | `crates/podbox-extract/src/safety.rs:543` and `:549`, and `crates/podbox-extract/src/drive.rs:366` |
| F containment | Not covered. No test walks the tree after a refusal. |

### Conversion plan, check F (the only new Rust work)

**Assertion today, as a triple.**

- Precondition: a store containing five crafted one-layer images
  (`traverse`, `dotdot`, `absolute`, `hardlink`, `legit`), each with a pinned
  digest computed at run time
  (`experiments/220-extract-path-safety.sh:93-180`).
- Action: `podbox extract localhost/<name>:crafted` for each of the four
  hostile ones.
- Expected: exit non-zero, the message contains `refusing`, and no path the
  layer tried to write exists on the filesystem:
  `$OUT/escaped`, `$OUT/store/escaped`, `/etc/podbox-must-never-write-this`, or
  `<rootfs>/etc/passwd` in any non-`legit` rootfs
  (`experiments/220-extract-path-safety.sh:249-261`).

**Level.** Pure plus fixture. The decision function is pure; the assertion is
about what landed, which needs a scratch tree the test arranges itself. Per
`docs/conventions/code.md:26-27` this is a pure test with a fixture, and the
fixture must be a fixture rather than a store: `docs/methodology/authoring.md:47`
says "Use conditions that a fixture can arrange."

**Target file and module.** `crates/podbox-extract/src/drive.rs`. I read the
`Scratch`, `layer` and `E` helpers that its existing tests use
(`crates/podbox-extract/src/drive.rs:366-389`), and they already build
in-memory tar layers. Check F becomes
`no_hostile_entry_landed_after_a_refusal` in that file's test block. It does
not need a new `tests/*.rs`: the existing test at
`crates/podbox-extract/src/drive.rs:382` calls `run(s.path(), &[l])` and
asserts on the `Err`, and check F asserts on the same `s.path()` after it.

**Fixtures.** The `E::Hardlink`, `E::Symlink` and `E::File` constructors in
`crates/podbox-extract/src/drive.rs` build every hostile shape the script
crafts with python's `tarfile`. The `traverse` case is the one that needs a
two-entry layer: `E::Symlink("evil", "/etc")` then
`E::File("evil/passwd", b"pwned\n")`, which is what
`crates/podbox-extract/src/safety.rs:588-592` already builds by hand with
`std::os::unix::fs::symlink`. The `experiments/220-extract-path-safety.sh:34-38`
warning applies to the Rust side too and the crate already handles it: the
`drive.rs` layer helper writes headers, so the `..` member is expressible.

**Plant.** `scripts/plant.sh` carries 44 cases and none names this script. The
new case removes `RESOLVE_NO_SYMLINKS` from the flags at
`crates/podbox-extract/src/safety.rs:244` and the new test must fail with "the
traversal was not refused". `TODO/extract.md:257-258` claims exactly this
mutation turns three tests red today; if it still does, the plant asserts the
existing three and the new test is the fourth. What settles the count: run
`cargo test -p podbox-extract` after the mutation and read the failing test
names. Add the case beside `case_plant "24 the export comparison removed"`
(`scripts/plant.sh:391-395`), the existing wording-mutation model, and add
`crates/podbox-extract/src/drive.rs` to the `FILES` restore list at
`scripts/plant.sh:51`.

**Command.** `cargo test -p podbox-extract drive::` for the new test, and
`cargo test -p podbox-extract` for the whole module. No flags, no triple: the
tests are Unix-only by the crate's own nature and already run in
`cargo test --workspace`, which `.github/workflows/gate.yml:199` runs.

**What the exit 2 path becomes.** Two of the three exit-2 arms are harness
conditions and do not survive: "no python3"
(`experiments/220-extract-path-safety.sh:73`) and "cannot craft the layer"
(`experiments/220-extract-path-safety.sh:178`) are the fixture builder's
problems, and a Rust test that cannot build its fixture is a failing test, not
an unrun one. The third, "$PODBOX is not built"
(`experiments/220-extract-path-safety.sh:74-77`), is a real third state and
becomes an `#[ignore]` on the integration-level arm, or disappears if the arm
moves to `crates/podbox-extract/src/drive.rs` and runs in-process.

**What becomes of the result file.** Split.
`experiments/results/extract-path-safety.txt` lines 1-11 (the six scalar
readings) become superseded when the crate tests land, because every one of
them is a `crates/podbox-extract` test outcome.
Lines 13-14, the verbatim refusal message, are evidence about the message text
and stay as the reading for T-0304's Decision. The refusal string
`experiments/results/extract-path-safety.txt:14` names
`refusing layer sha256:786730bb...` and the reason
`("evil": ELOOP)`. Keep the file, retitle its header to name the crate test
that now carries each row, and do not regenerate it from a shell.

---

## Group 7 — `experiments/230-lifecycle-loop.sh`

**What it measures.** Whether the container lifecycle holds twenty times in a
row. The question is at `experiments/230-lifecycle-loop.sh:2-3`. It has four
parts: a twenty-iteration loop (lines 66-118), T-0604's killed-launcher clause
(lines 121-172), T-0603's thread clause (lines 175-195), and T-0605's log
clause (lines 198-216).

**Pinned inputs.** `IMAGE="${PODBOX_RUN_IMAGE:-ghcr.io/pkgforge-dev/archlinux:latest}"`
(`experiments/230-lifecycle-loop.sh:30`) and `N="${1:-20}"`
(`experiments/230-lifecycle-loop.sh:31`). The image is a floating tag, not a
digest. `TODO/RULES.md:63` says "Label an estimate each time it appears. Use a
dash for an unknown value", and `docs/methodology/experiments.md:9` says
"Resolve inputs explicitly." A tag is neither; the conditions block records it
as a string and cannot compare two runs.

**Host assumptions.** A release binary at
`target/x86_64-unknown-linux-musl/release/podbox`, or `PODBOX_BIN`
(`experiments/230-lifecycle-loop.sh:25`). Registry access, once, before the
loop (`experiments/230-lifecycle-loop.sh:56-61`).

**Exit-code contract.** Three, and they compose: `1` if `fail`, `1` if
`passed != N`, `2` if `skipped`
(`experiments/230-lifecycle-loop.sh:229-231`). The pull failure is a direct
`exit 2` at `experiments/230-lifecycle-loop.sh:60`.

**Timeouts.** `timeout 1800` on the pull, `timeout 300` on `create`, `start`
and `run -d`, `timeout 120` on `stop`, `timeout 60` on `ps`, `exec`, `inspect`,
`rm` and `wait` (`experiments/230-lifecycle-loop.sh:56-215`). `dev.sh` is not
involved; these are per-command bounds.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at
`experiments/230-lifecycle-loop.sh:28`. The clauses kill the payload they
created: `experiments/230-lifecycle-loop.sh:168-170` and `:194` and `:215`.
The loop counts what it left at `experiments/230-lifecycle-loop.sh:219`.

**Positive control.** Every step in the loop is a positive assertion: `ps`
shows `Up`, `exec` prints `marker`, the post-stop status reads
`Exited (143)` (`experiments/230-lifecycle-loop.sh:83`, `:89`, `:99`). Clause 2
adds the strongest one in the group: the exit code must be `-` and a notice
time must be recorded (`experiments/230-lifecycle-loop.sh:153-160`).

**Failure control.** `stop` is the one that matters, and the script is honest
about it. `experiments/230-lifecycle-loop.sh:106-114` stops at the first
failure and never retries, and `TODO/supervise.md:110-112` records that the
loop found a real `stop` race three times (10 of 20, 9 of 20, 1 of 3) that a
hand-driven loop passed every time. That is the instrument working.

**TODO entries that own it.** Four, and all four name it:
`TODO/supervise.md` T-0601 (`TODO/supervise.md:93`), T-0602
(`TODO/supervise.md:513`), T-0603 (`TODO/supervise.md:187`), T-0604
(`TODO/supervise.md:215`), T-0605 (`TODO/supervise.md:260`); and
`TODO/milestones.md` T-1105 (`TODO/milestones.md:342`) and
`TODO/milestones.md:340` for the decision.

**Result files it wrote.** `experiments/results/lifecycle-loop.txt`
(`experiments/230-lifecycle-loop.sh:26`), taken 2026-09-09, 20 of 20
consecutive passes, all three clauses green.

**Contradiction with an entry.** `TODO/milestones.md:337-339` and
`TODO/supervise.md:505-509` both say the loop is "create, detached `start`,
`ps` shows it running, `exec` prints a marker, `stop`, `rm`". The script calls
`start` without `-d` (`experiments/230-lifecycle-loop.sh:75`) and creates with
`--name` only (`experiments/230-lifecycle-loop.sh:72`). Whether `start` is
detached by default is a question about `crates/podbox-cli/src/lifecycle.rs`
that I did not read. What settles it: read the `start` implementation and record
whether it detaches. Two entries say "detached start" and the script does not
pass `-d`.

**Verdict: SPLIT.** Four parts, four destinations.

| Part | Lines | Goes to |
| --- | --- | --- |
| The 20-iteration loop | 66-118 | A fault test in `crates/podbox-supervise`, or a retained shell soak |
| T-0604 killed launcher | 121-172 | A fault test |
| T-0603 thread count | 175-195 | A fault test |
| T-0605 log capture | 198-216 | An integration test |

### Conversion plan, the killed-launcher clause (T-0604)

**Assertion today, as a triple.**

- Precondition: a detached container `deadprobe` whose launcher pid was
  recorded and is not `0` (`experiments/230-lifecycle-loop.sh:131-138`).
- Action: `kill -9 $launcher`
  (`experiments/230-lifecycle-loop.sh:141`).
- Expected: `ps -a` reads `deadprobe dead`; `inspect --format '{{.ExitCode}}'`
  prints `-`; `inspect --format '{{.Noticed}}'` prints a time; `podbox wait`
  exits non-zero (`experiments/230-lifecycle-loop.sh:149-165`).

**Level.** Fault, per `docs/conventions/code.md:27` ("Use fault tests for
conditions a real service cannot produce on demand"). A `SIGKILL`ed launcher is
exactly that. `TODO/supervise.md:222-225` names the mechanism: a launcher holds
an exclusive `flock` on its own lock file, and `table::reconcile` decides a
launcher is gone by trying to take it.

**Target file and module.** `crates/podbox-supervise/src/table.rs`, whose
`mod tests` holds six tests today (measured by counting `#[test]` across
`crates/podbox-supervise/src/*.rs`: `lib.rs` 7, `table.rs` 6, `launcher.rs` 1).
`reconcile` is the function under test and it lives in that module.

**Fixtures.** None new. The flock-based `reconcile` needs only a lock file and
a table record, both of which `table.rs`'s existing tests already arrange. What
settles the exact shape: the implementor reads `reconcile` and copies the
arrangement from the test beside it.

**Plant.** Remove the `flock` release path from `reconcile` and the new test
must fail with a `dead` state that was never written. `scripts/plant.sh` has
no case for this script; add one beside
`case_plant "29 a dropped cleanup procedure"`
(`scripts/plant.sh:452`) and add `crates/podbox-supervise/src/table.rs` to the
`FILES` list at `scripts/plant.sh:51`.

**Command.** `cargo test -p podbox-supervise table::`. The existing tests run
in `cargo test --workspace`, which `.github/workflows/gate.yml:199` runs, so
the new test joins CI with no workflow change.

**What the exit 2 path becomes.** The script's `skipped` variable
(`experiments/230-lifecycle-loop.sh:140`) exists because the four parts run in
sequence and one failing skips the rest. Split tests do not have that
coupling: each asserts its own subject, so `skipped` disappears and the script's
`exit 2` at `experiments/230-lifecycle-loop.sh:231` has no successor. The pull
failure at `experiments/230-lifecycle-loop.sh:56-61` is the real third state
and becomes an `#[ignore]` naming "registry unreachable", since
`docs/conventions/code.md:34` says "A skipped test proves nothing about its
subject" and the reason must say so.

### Conversion plan, the twenty-iteration loop (T-1105, T-0602)

**Keep this as a shell soak, or convert to a fault test with a decision.** The
question for the operator: does a Rust test that forks a real launcher twenty
times belong in `cargo test`, where it would run on every push and in every
contributor's local run, or in a nightly lane? The evidence both ways: the loop
is what found the T-0602 race, and `TODO/supervise.md:103-125` records that a
hand-driven loop passed every time, so the instrument's value is the repetition
and the cold store. Against it: twenty iterations each pulling and extracting
an image is minutes of CI time, and `.github/workflows/gate.yml:140` already
caps the lint job at 20 minutes. My recommendation: keep the loop as a shell
soak, move it into the new `podbox-gate` binary as `supervise soak --count 20`,
and have `cargo test` carry the three single-shot clauses above. The count and
the "no sleep anywhere" rule stay exactly as they are
(`experiments/230-lifecycle-loop.sh:12-16`), because those are the measurement
and a Rust rewrite of a loop is the loop.

**What becomes of the result file.** Keep `experiments/results/lifecycle-loop.txt`.
It is the only proof the twenty-run milestone has, and
`TODO/milestones.md:344-345` and `TODO/supervise.md:515-516` both cite the
number from it. If the loop moves to `podbox-gate`, the binary writes the same
file in the same shape and the header names the binary instead of the script.

---

## Group 7 — `experiments/240-distro-sweep.sh`

**What it measures.** Whether `podbox run` installs a C toolchain through ten
distributions' own package managers and builds and runs a two-file program with
it. The question is at `experiments/240-distro-sweep.sh:2-4`.

**Pinned inputs.** `scripts/common/distro-matrix.sh` holds `DISTRO_ROWS_M5`
and `distro_pinned`, sourced at
`experiments/240-distro-sweep.sh:107`. The driver is
`public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a`
(`experiments/240-distro-sweep.sh:89`). No Docker Hub
(`experiments/240-distro-sweep.sh:32-34`).

**Host assumptions.** A native Linux lane, or a non-native lane where the
podbox binary is staged into a bookworm driver container and executed there
(`experiments/240-distro-sweep.sh:293-301`). The non-native lane installs
`ca-certificates` once into the driver and shares the bundle by path
(`experiments/240-distro-sweep.sh:317-323`).

**Exit-code contract.** `0` every row built and ran, `1` a row pulled and did
not, `2` nothing ran, through `distro_verdict`
(`experiments/240-distro-sweep.sh:460-461`). The three engine and toolchain
guards are `exit 2` at lines 79, 80, 83 and 101.

**Timeouts.** `ROW_TIMEOUT="${PODBOX_ROW_TIMEOUT:-1200}"`
(`experiments/240-distro-sweep.sh:95`) and an outer bound of
`ROW_TIMEOUT + 900` (`experiments/240-distro-sweep.sh:98`). The native pull is
`timeout 900` (`experiments/240-distro-sweep.sh:365`), the run is
`timeout "$ROW_TIMEOUT"` (`experiments/240-distro-sweep.sh:387`), the CA
install is `eng_run 600` (`experiments/240-distro-sweep.sh:318`).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at
`experiments/240-distro-sweep.sh:65`. Every row runs with `--rm`
(`experiments/240-distro-sweep.sh:386-390`), and the driver uses a
container-local store under `/tmp/ostore` that is removed per row
(`experiments/240-distro-sweep.sh:272-275`).

**Positive control.** The row passes only if the program RAN, not if the
install returned 0 (`experiments/240-distro-sweep.sh:442-443`). That is the
distinction a two-file project exists to make.

**Failure control.** The strongest in Group 7. A row that fails is run again
under the host engine with the same bytes and no help
(`experiments/240-distro-sweep.sh:410-426`), and the result distinguishes
`podbox is missing a fixup` from `this machine cannot do it either`
(`experiments/240-distro-sweep.sh:37-43`). `TODO/milestones.md:430-433` records
that this control turned the last two failures into two different answers, and
`TODO/milestones.md:443-449` records that `voidlinux-musl` read `host` and was
podbox's after all. The control is the reason this script exists in its current
form.

**TODO entries that own it.** `TODO/milestones.md` T-1106
(`TODO/milestones.md:398`, Done at `:401-475`), `TODO/gate.md` T-1211
(`TODO/gate.md:893`, `:906-907`), `TODO/gate.md` T-1203
(`TODO/gate.md:133-202`, the runner discipline), `TODO/image.md` T-0214
(`TODO/image.md:1406`, `TODO/image.md:1462`), and six rows of
`TODO/complete.md` (T-0406, T-0407, T-0408, T-0409, T-0411, and the T-0402
gateway at `TODO/interpose.md:1242`).

**Result files it wrote.** `experiments/results/distro-sweep.txt`
(`experiments/240-distro-sweep.sh:55`), taken 2026-09-22, ten rows, ten built
and ran, `host_not_runtime 0`. Ten row transcripts under
`experiments/results/sweep/`, one per row; I read `alpine.out` and `debian.out`
and both carry the pull transcript, the stdout readings and the completion
layer's own lines.

**Contradiction with an entry.** One. `TODO/milestones.md:468-475` records the
run as `2026-09-09` in its own Prove block ("Prove, run 2026-09-09"), and
`TODO/milestones.md:401` dates the Done record the same day.
`experiments/results/distro-sweep.txt:2` says `Taken 2026-09-22T04:45:29Z`. The
entry's transcript and the saved result are thirteen days apart, and the entry
does not record the later run. The later run is the current one, because
`scripts/common/result-diff.sh` diffs against it and the row transcripts in
`experiments/results/sweep/` are dated with it. Not a defect in the sweep; a
stale date in the entry.

**Verdict: KEEP-SHELL.**

The specific host dependencies, each of which no Rust crate in this workspace
replaces:

1. **A foreign OCI engine as control.** The failure control at
   `experiments/240-distro-sweep.sh:410-426` shells out to
   `experiments/lib/engine.sh`, which picks a `docker` daemon where one answers
   and host `podman` otherwise. `podbox-image` pulls images; it has no engine
   driver and building one would be a second runtime, which
   `TODO/RULES.md:117` ("Tracked source corpus") and the whole
   `experiments/lib/engine.sh` split exist to prevent.
2. **The distributions' own package managers.** The subject at
   `experiments/240-distro-sweep.sh:162-246` runs `apk`, `apt-get`, `pacman`,
   `dnf`, `microdnf`, `zypper` and `xbps-install` inside the container. This is
   the subject of the measurement. A Rust test would be re-implementing the
   thing under test.
3. **A real network and a real registry.** Ten pulls at up to 1200 s each.
   `docs/conventions/code.md:33` says "A mock does not prove a live service or
   deployment", and this is the live service.

**What must not be lost in any future refactor.** The control. If the sweep
ever runs as a `cargo test`, the control disappears and a machine that cannot
install a toolchain reads as a podbox defect, which is the exact misreading
`TODO/milestones.md:443-449` records happening once already.

**What the exit 2 path becomes.** Nothing. It keeps its three states, which is
the point. `scripts/common/distro-matrix.sh` holds
`distro_verdict` and 125 and 240 both source it, so a Rust rewrite would have
to be a new home for that function and would break the "one runner" rule
`TODO/milestones.md:388-393` states.

---

## Group 7 — `experiments/260-multiarch.sh`

**What it measures.** Whether podbox is a multi-architecture runtime, and what
"runs on aarch64" means when the only aarch64 is an emulator. The question is
at `experiments/260-multiarch.sh:2-3`. Seven clauses, listed at
`experiments/260-multiarch.sh:14-23` and written at lines 92-410.

**Clause 1** runs `cargo check --workspace --target $t` for eight triples
(`experiments/260-multiarch.sh:55-72`, loop at `:94-108`).

**Clause 2** iterates `BLOCKED`, which is empty
(`experiments/260-multiarch.sh:76`, loop at `:122-140`).

**Clause 3** compiles a powerpc64 inline-asm probe crate to ask rustc directly
(`experiments/260-multiarch.sh:146-173`).

**Clause 4** builds podbox for aarch64, registers a `binfmt_misc` entry, runs
it under `qemu-aarch64-static`, and prints the probe rows as a warning
(`experiments/260-multiarch.sh:176-243`).

**Clause 5** reads the registration's flags back, chroots a bare directory
containing only the binary, and runs it
(`experiments/260-multiarch.sh:247-351`).

**Clause 6** is nested inside clause 5 because it needs the live registration
(`experiments/260-multiarch.sh:287-345`). It checks `measured_by.emulated`,
`measured_by.interpreter`, the cache key, the banner, and that a native answer
is refused to an emulated run.

**Clause 7** builds for powerpc64le and runs it under `qemu-ppc64le-static`,
asserting the binary reads `ELF machine 0x15` out of its own header
(`experiments/260-multiarch.sh:354-410`).

**Pinned inputs.** The eight triples at
`experiments/260-multiarch.sh:55-72`, including `s390x-unknown-linux-gnu` as a
check-only target with the reason written inline at lines 68-71.

**Host assumptions.** `cargo`, `rustup`, `qemu-aarch64-static`,
`qemu-ppc64le-static`, `jq`, and a writable `/proc/sys/fs/binfmt_misc`. The
registration is named after the pid and removed by the trap
(`experiments/260-multiarch.sh:42-48`).

**Exit-code contract.** `0` every clause held, `1` one did not, `2` could not
run (`experiments/260-multiarch.sh:24`, `:420-421`).

**Timeouts.** Only two: `timeout 120` on the ppc `version`
(`experiments/260-multiarch.sh:383`) and `timeout 300` on the ppc `probe --json`
(`experiments/260-multiarch.sh:391`). The eight `cargo check` calls and the
aarch64 build at `experiments/260-multiarch.sh:184-186` are unbounded.
`AGENTS.md:72` requires bounding external commands.

**Cleanup.** `cleanup()` unregisters the binfmt entry and removes `$WORK`
(`experiments/260-multiarch.sh:43-48`).

**Positive control.** Clause 7's `ELF machine 0x15` is the best single
assertion in Group 7 and the comment says why
(`experiments/260-multiarch.sh:362-366`): "Asm nobody has executed is a
claim". A wrong powerpc64 register convention is not a build failure, it is a
syscall with the arguments in the wrong places, and powerpc returns a POSITIVE
errno in r3, so a trap that ignored that would read every failure as a huge
success.

**Failure control.** None. No clause establishes that the instrument reached
its subject. Clause 4's saved reading says
`binfmt for aarch64 registered: no` (`experiments/results/multiarch.txt:40`) and
the rung read `unsupported` (`experiments/results/multiarch.txt:44`) for
exactly that reason, and the script recorded it rather than treating it as a
pass. That honesty is in the script; a machine that hid it would not be.

**TODO entries that own it.** `TODO/deps.md` T-0911
(`TODO/deps.md:732` Prove, `:714-731` Decision) and T-0912
(`TODO/deps.md:830` Prove, `:803-812` Premise), and `TODO/enter.md` T-0506
(`TODO/enter.md:437-438` and `:469` name clauses 5 and 6, and `:505-511` records
a defect this script's clause 4 had).

**Result files it wrote.** `experiments/results/multiarch.txt`
(`experiments/260-multiarch.sh:29`), taken 2026-09-09.

**Contradiction with the entries, and it is the important one.**
`TODO/deps.md:810-812` states that clause 2 "goes **red** the day
`powerpc64le` starts building, which is the point: it is how this project finds
out rather than a failure to suppress." The guard is `BLOCKED=()` at
`experiments/260-multiarch.sh:76`, and an empty array body runs zero iterations
at `experiments/260-multiarch.sh:122`. The event
the entry names has already happened: `experiments/results/multiarch.txt:16`
reads `ok powerpc64le-unknown-linux-musl workspace checks` and line 20 reads
`none`. T-0912 closed on 2026-09-09, the same day. A guard that has already
fired and cannot fire again is not a guard.

A second, smaller one. `TODO/deps.md:830` says the Prove "reports 7
architectures checking the workspace and 0 blocked". The Done record at
`TODO/deps.md:832-833` corrects it: "the Prove's number was low: it says seven
architectures and the reading is EIGHT." The saved file agrees with the
correction (`experiments/results/multiarch.txt:10-17`, eight rows). The
`Prove` line at `TODO/deps.md:830` is still uncorrected.

**Verdict: SPLIT.** Seven clauses, four destinations.

| Clause | Goes to |
| --- | --- |
| 1, the eight `cargo check` calls | The gate, as a build matrix |
| 2, the `BLOCKED` loop | DELETE, with the empty-list finding recorded in T-0912 |
| 3, the powerpc64 asm probe | DELETE, superseded by `crates/podbox-probe/Cargo.toml:30` |
| 4, aarch64 under qemu | KEEP-SHELL, in a lane job |
| 5, the `F` flag and the bare chroot | RUST-TEST, one arm; the registration half stays shell |
| 6, `measured_by` and the cache key | RUST-TEST, already partly present |
| 7, the ppc64 trap executed | KEEP-SHELL, needs `qemu-ppc64le-static` |

### Conversion plan, clause 6 (RUST-TEST)

**Assertion today, as a triple.**

- Precondition: an aarch64 podbox running under `qemu-aarch64-static`, with
  `probe --json` producing a document
  (`experiments/260-multiarch.sh:295-297`).
- Action: read `.measured_by.emulated`, `.measured_by.interpreter` and
  `.cache_key.interpreter` out of that document, and grep the banner
  (`experiments/260-multiarch.sh:305-325`).
- Expected: `emulated` is `true`; `interpreter` equals `cache_key.interpreter`;
  the banner contains `NOT BY THIS MACHINE`; and a native podbox sharing the
  store is refused with `the instrument changed`
  (`experiments/260-multiarch.sh:330-339`).

**Level.** Pure, per `docs/conventions/code.md:26`. The reader is
`crates/podbox-probe/src/interp.rs`, which the entry names at
`TODO/enter.md:466-469` ("`crates/podbox-probe/src/interp.rs`,
`measured_by` in `report::document`, a line in the banner, and `interpreter` as
the eighth component of the cache key"). That file carries three tests today.
`TODO/enter.md:478-493` records that the detection is a five-row table of what
is and is not usable under the emulator, which is pure logic over `/proc`
contents and belongs in the same module.

**Target file and module.** `crates/podbox-probe/src/interp.rs`, its existing
`mod tests`. The cache-key half belongs in `crates/podbox-probe/src/select.rs`,
which carries eleven tests today.

**Fixtures.** None new. `TODO/enter.md:481-486` is a table of five probes with
their findings, and each is a pure function over a small input.

**Plant.** Delete `interpreter` from the cache key at
`crates/podbox-probe/src/select.rs` and the new test must fail on
`interpreter == cache_key.interpreter`. No case in `scripts/plant.sh` targets
this; add one beside `case_plant "31 source state differs"`
(`scripts/plant.sh:497`) and add `crates/podbox-probe/src/interp.rs` and
`crates/podbox-probe/src/select.rs` to `FILES` at `scripts/plant.sh:51`.

**Command.** `cargo test -p podbox-probe interp::` and
`cargo test -p podbox-probe select::`. No flags, no triple.

**What the exit 2 path becomes.** The script's `skipped` at
`experiments/260-multiarch.sh:220` and `:349` covers "no qemu", "binfmt not
mounted", "the binary did not build" and "no jq". The first two are the third
state and become `#[ignore]` with the condition named. The third is a build
failure and belongs in the gate matrix, not in a test. The fourth disappears:
`jq` parsing a JSON document in a Rust test is `serde_json`.

**What becomes of the result file.** Split.
`experiments/results/multiarch.txt:37-59` (clause 4) and `:61-74` (clauses 5
and 7) are host measurements and stay.
Lines 9-17 (clause 1) become the gate's build matrix and the file stops
carrying them.
Lines 19-27 (clause 2) are superseded: the list is empty and the question is
settled.
Lines 29-35 (clause 3) are superseded: the answer is in
`crates/podbox-probe/Cargo.toml:15-30` as a target-gated dependency, and
`TODO/deps.md:804-808` already records the finding.

**What must be done to T-0912 in the same change.** Correct
`TODO/deps.md:830` to eight architectures, and replace the clause-2 claim at
`TODO/deps.md:810-812` with the fact: the guard was an empty list, it cannot
fire, and the event it was watching for happened. `docs/conventions/prose.md:44`
requires that correction in place rather than a box below.

---

## Group 7 — `experiments/361-guest-usernet.sh`

**What it measures.** Whether a guest built from a podbox-extracted rootfs
reaches the host through UDP under TCG with user-mode networking, and whether
the host reaches the guest through a forwarded port. The question and seven
clauses are at `experiments/361-guest-usernet.sh:2-29`.

**Pinned inputs.** The image is
`public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764`
(`experiments/361-guest-usernet.sh:44`). The kernel and modloop are fetched and
verified against pins at `experiments/361-guest-usernet.sh:45-48`, checked at
`:130-131` and `:141-142`.

**Host assumptions.** `qemu-system-x86_64` (installed by the script at
`experiments/361-guest-usernet.sh:66-68` when absent), `cpio`, `python3`,
`curl`, `sha256sum`, `timeout`, `unsquashfs`
(`experiments/361-guest-usernet.sh:74-76`). The assembly helper is
`experiments/lib/podvm-guest.sh` (51 lines, sourced at
`experiments/361-guest-usernet.sh:42`), shared with 146, 147, 154 and 360.

**Exit-code contract.** `0` both datagrams crossed, `1` the assembly or either
direction failed, `2` a tool, the binary or an input could not run
(`experiments/361-guest-usernet.sh:28-29`, guards at lines 57-61 and 74-76).

**Timeouts.** `BOOT_TIMEOUT="${PODBOX_361_TIMEOUT:-240}"` and
`CURL_TIMEOUT="${PODBOX_361_CURL_TIMEOUT:-60}"`
(`experiments/361-guest-usernet.sh:49-50`), plus `timeout 600` and `timeout
1200` on the apt path at `experiments/361-guest-usernet.sh:66-68`.

**Cleanup.** No `trap`. The scratch is
`experiments/.sweep361-work` (`experiments/361-guest-usernet.sh:36`) and is
never removed by the script. `experiments/README.md:19` warns that
`.tmp` must not be a dependency of a future test; this scratch is under
`experiments/` instead, which has the same problem. What settles it: check
whether `.sweep361-work` is gitignored, and whether it is present on disk
after a run.

**Positive control.** Clause 2 reads the guest applets out of the busybox
binary's own string table rather than assuming them
(`experiments/361-guest-usernet.sh:108-121`).

**Failure control.** Yes, and it is a real one. The script checks the busybox
for the applets before booting, so a guest without `nc` fails at clause 2 with
a named cause rather than at clause 6 with a timeout
(`experiments/361-guest-usernet.sh:122`, `:159-161`). The listener has a
120-second socket timeout and prints `TIMEOUT waiting for the guest token`
(`experiments/361-guest-usernet.sh:280-285`).

**TODO entries that own it.** `TODO/podvm.md` T-1306
(`TODO/podvm.md:606-635`, the 2026-09-26 Done record). `149-podvm-non-goals.sh`
gains a positive arm that runs 361 for its exit code
(`TODO/podvm.md:615-617`), so T-1306 has a second consumer.

**Result files it wrote.** `experiments/results/guest-usernet.txt`
(`experiments/361-guest-usernet.sh:35`), taken 2026-09-26, both datagrams
crossed.

**Contradiction with the entry, and with itself.** Three.

1. `TODO/podvm.md:622-624` says "361 exits 0" and the reading's last line says
   `360 acceptance: datagrams crossed both ways`
   (`experiments/results/guest-usernet.txt:84`). The script number is 361, and
   the line says 360. The string is at
   `experiments/361-guest-usernet.sh:323`. The saved result was produced by a
   script numbered 360 or by an earlier copy of this one; the rootfs path in
   the same file (`experiments/results/guest-usernet.txt:11`) reads
   `.sweep360-work`, and the current script writes `.sweep361-work`
   (`experiments/361-guest-usernet.sh:36`). The committed result was not
   produced by the committed script.
2. `TODO/podvm.md:623` says "both applets and both pins hold", where "both
   applets" is not a term the script uses. The script lists twelve applets
   (`experiments/361-guest-usernet.sh:114`) and the reading shows all twelve
   (`experiments/results/guest-usernet.txt:14-25`). Not a defect, a loose noun.
3. `experiments/README.md:19-21` names `10`, `20` and `30` as the initial
   reconstruction and says "Later scripts test podbox behavior." 361 tests
   QEMU and a Linux distribution, not podbox. The clause-5 and clause-6
   assertions are about the emulator's `hostfwd`; podbox's contribution is the
   extracted rootfs in clause 1. Worth recording, not worth a verdict change.

**Verdict: KEEP-SHELL.**

The specific host dependencies:

1. **`qemu-system-x86_64` with `-netdev user,hostfwd=udp::`** at
   `experiments/361-guest-usernet.sh:299`. The `hostfwd` forwarding is the
   subject of clause 6. No Rust crate in this workspace configures an emulator.
   `crates/podbox-windows` drives an emulator for a Windows guest; it has no
   user-mode networking and building it would be a second mechanism.
2. **A network fetch of a pinned Alpine netboot kernel and modloop**
   (`experiments/361-guest-usernet.sh:45-48`). `docs/conventions/code.md:33`:
   "A mock does not prove a live service."
3. **`unsquashfs` on a squashfs modloop** to find `virtio_net.ko`
   (`experiments/361-guest-usernet.sh:157`). The comment at
   `experiments/361-guest-usernet.sh:134-137` records why the modules are not in
   the image: "The netboot kernel keeps NIC drivers in the modloop, not in the
   image."

**The one arm that could move.** Clauses 1 and 2, pull and extract of a pinned
image plus the busybox applet census, are podbox behaviour and could become a
Rust test in `crates/podbox-image` and `crates/podbox-extract`. I would not do
it: the applet census is a measurement of a third-party image's contents, not
of podbox, and it is 12 lines of `grep`. Keep the whole script.

**What the exit 2 path becomes.** Nothing. Three states, all correct, all
host-shaped.

**A defect to fix in the same change as any refactor.** The script writes
`360` where it means `361` (`experiments/361-guest-usernet.sh:323`), and
`experiments/results/guest-usernet.txt` carries a `.sweep360-work` path
(`experiments/results/guest-usernet.txt:11`) that the committed script cannot
produce. `docs/conventions/prose.md:46` says "A historical result stays
historical", so the result file stays as it is. The script's string is a
defect and should be corrected to `361`.

---

## Group 7 — `experiments/362-windows-refusal.sh`

**What it measures.** Whether a Windows request refuses by name, before
anything is fetched or mutated, on every door no guest driver serves. The
question and five clauses are at `experiments/362-windows-refusal.sh:2-6` and
`:16-27`.

**Clause 1** asserts the fixture: `open(/dev/kvm, O_RDWR)` denied `ENOENT`
(`experiments/362-windows-refusal.sh:121-131`).

**Clauses 2 to 5** each run a `podbox` invocation expecting exit 125, grep two
message fragments, and compare a store snapshot taken before with one taken
after (`experiments/362-windows-refusal.sh:133-182`). The snapshot is
`find "$STORE" -type f | sort`
(`experiments/362-windows-refusal.sh:134`).

**Clause 2** `run --rm --platform windows/amd64` without the machine tier must
name `windows/amd64` and `podbox windows run`
(`experiments/362-windows-refusal.sh:139-147`).

**Clause 3** with `--podbox-tier=machine` and an OCI token must name the DOS
base image and `369-windows-tcg-dos` (`experiments/362-windows-refusal.sh:152-160`).

**Clause 4** `create` must name `windows/amd64`
(`experiments/362-windows-refusal.sh:163-170`).

**Clause 5** a missing disk path must name the file and say
`never replaced` (`experiments/362-windows-refusal.sh:174-182`).

**Pinned inputs.**
`public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc`
(`experiments/362-windows-refusal.sh:43`). `PODBOX_DOS_BASE` and
`PODBOX_WINDOWS_BASE` are set and unset by the script itself
(`experiments/362-windows-refusal.sh:96-100`) so a base cannot leak in from the
caller.

**Host assumptions.** A Windows job container where the wrapper runs the job at
`/in/job.sh` with `/work` as the working directory
(`experiments/362-windows-refusal.sh:35-38`). `cargo`, `cc`, `jq`,
`scripts/common/bootstrap-env.sh` and `scripts/build-interpose.sh`
(`experiments/362-windows-refusal.sh:57-75`).

**Exit-code contract.** `0` every clause matched, `1` a clause disagreed or a
build failed, `2` the lane could not run
(`experiments/362-windows-refusal.sh:29-32`). Note that the build failure is
`exit 1` (`experiments/362-windows-refusal.sh:82`), not 2. That is correct
(a build that runs and fails is a failure) and worth keeping.

**Timeouts.** `timeout 1200` on bootstrap and interpose, `timeout 1800` on
`cargo build`, `timeout 120` on every step
(`experiments/362-windows-refusal.sh:60`, `:69`, `:77`, `:106`).

**Cleanup.** `rm -rf "$WORK"` at `experiments/362-windows-refusal.sh:39`. No
trap; the report is copied out at `experiments/362-windows-refusal.sh:189`.

**Positive control.** None in the strict sense, but clauses 2 and 4 assert two
different verbs produce the same refusal, and clause 5 is a distinct path. The
store-untouched assertion after every clause is a control of a kind: a refusal
that mutated the store fails four times independently.

**Failure control.** None. Nothing proves the instrument reached the door.
A `podbox` binary that refused everything for any reason would pass all four
clauses, and so would one that never ran at all if it exited 125.

**TODO entries that own it.** `TODO/milestones.md` T-1112
(`TODO/milestones.md:848-894`, whose Prove at `:867-872` names 369, 370 and 392
and NOT 362, and whose `Source` at `:850` names "experiments 369, 370, 371 and
392"). `TODO/podvm.md:8` names 371 for the lane. T-1112 is `partial`.

**Result files it wrote.** `experiments/results/windows-refusal.txt`
(`experiments/362-windows-refusal.sh:189`), taken 2026-09-27, all five clauses,
`WINDOWS REFUSAL HOLDS`.

**Contradiction with the entry.** One, and it is a real gap. T-1112's `Source`
line (`TODO/milestones.md:850`) names 369, 370, 371 and 392. Its `Prove`
(`TODO/milestones.md:867-869`) names 369, 370 and 392. `362` appears nowhere in
the entry, and `TODO/probe.md:8` in the script's own header is the only place
that names T-1112 for it. The script is owned by an entry that does not list
it, and its only saved result predates the entry's `Partial 2026-09-30` record
by three days. The clause 3 message at
`experiments/results/windows-refusal.txt:24` names
`experiments/369-windows-tcg-dos.sh`, and the script asserts that string, so
the script and the result agree; it is the entry that is out of date.

**Verdict: RUST-TEST.** All four refusals are pure string-and-return paths
over a fixture, and every message they must contain is still in the source. I
read each one:

- `crates/podbox-cli/src/lifecycle.rs:1657-1665` emits
  `{os}/{arch} is not an OCI rootfs, so nothing on this path pulls or enters it.
  A disposable {os} guest is its own verb: ...`
- `crates/podbox-cli/src/windows/dos.rs:60-66` emits
  `no DOS base image at {} and none was named. The DOS flavor boots FreeDOS...`
- `crates/podbox-cli/src/windows/mod.rs:85-91` emits
  `{} is not a file. A named image is never replaced by the cache...`
- Every one returns `EXIT_RUNTIME_ERROR` (125), which is what the script reads
  through `scripts/common/exit-codes.sh` at
  `experiments/362-windows-refusal.sh:88-91`.

**Conversion plan.**

- Precondition: `PODBOX_WINDOWS_BASE` and `PODBOX_DOS_BASE` unset, and a
  scratch store.
- Action: call the four refusal functions directly.
- Expected, as four triples:
  1. `reject_guest_platform("windows", "amd64", "run")` returns
     `EXIT_RUNTIME_ERROR` and its message contains `windows/amd64` and
     `podbox windows run`.
  2. `dos_plan` with no base present returns `EXIT_RUNTIME_ERROR` and the
     message contains `DOS base image` and `369-windows-tcg-dos`.
  3. The same platform rejection on the `create` path returns
     `EXIT_RUNTIME_ERROR` and contains `windows/amd64`.
  4. The windows image resolver with a missing path returns
     `EXIT_RUNTIME_ERROR` and contains `is not a file` and `never replaced`.

- Level: pure with a fixture. Per `docs/conventions/code.md:26`, deterministic
  logic on deterministic inputs. The only impure input is the absence of a
  file, which a scratch directory arranges.
- Target file and module. Three files, because the three functions live in
  three modules and the entry names all three crates:
  `crates/podbox-cli/src/lifecycle.rs` test block, which carries 24 tests
  today; `crates/podbox-cli/src/windows/dos.rs` test block, part of the
  `windows` module whose files carry 3 to 10 tests each; and
  `crates/podbox-cli/src/windows/mod.rs` test block. Test names:
  `a_windows_platform_is_refused_naming_its_own_verb`,
  `the_dos_flavor_refuses_naming_the_setup_script`, and
  `a_named_image_is_never_replaced_by_the_cache`.
- Fixtures. None new. A `tempfile`-scope directory with no
  `base.vhdx` in it is the whole fixture, and the environment variables are
  unset in-process rather than by the shell.
- Plant. Change `EXIT_RUNTIME_ERROR` to `EXIT_USAGE_ERROR` at
  `crates/podbox-cli/src/lifecycle.rs:1664` and the new test must fail on the
  code. The message half is a second case: change
  `is not an OCI rootfs` to `is unsupported` and it must fail on the string.
  Add both to `scripts/plant.sh` beside `case_plant "26a a done Prove naming a
  refused flag"` (`scripts/plant.sh:405`), which is the existing model for a
  wording mutation, and add the three source files to `FILES` at
  `scripts/plant.sh:51`.
- Command. `cargo test -p podbox-cli windows::` and
  `cargo test -p podbox-cli lifecycle::`. No flags, no triple.
- What exit 2 becomes. Five conditions: no `cargo`, no `cc`, no bootstrap, no
  `jq`, no binary after build
  (`experiments/362-windows-refusal.sh:57-86`). All five are toolchain
  conditions and all five disappear: a Rust test in the workspace has the
  toolchain by definition, and a build that fails is a failing build. The
  clause-1 fixture condition, `/dev/kvm` absent, is the only real third state;
  it becomes a guard in the new test, not a skip, because the refusals do not
  depend on KVM at all and the script says so at
  `experiments/362-windows-refusal.sh:13-14`.
- What becomes of the result file. Superseded.
  `experiments/results/windows-refusal.txt:13-39` records four refusals and
  four `store untouched` lines. All eight are crate test outcomes. Keep the
  file under `docs/history/` once the tests land, and add a line to T-1112's
  `Partial 2026-09-30` record naming 362 as its refusal-arm proof, which the
  entry currently does not do.

---

## Group 7 — `experiments/367-qemu-user-aarch64.sh`

**What it measures.** Whether an off-arch payload runs end to end where
qemu-user holds it. The question and five clauses are at
`experiments/367-qemu-user-aarch64.sh:2-21`.

**Clause qemu-1** reads `/proc/sys/fs/binfmt_misc/qemu-aarch64` and checks the
interpreter is executable (`experiments/367-qemu-user-aarch64.sh:54-60`).

**Clause qemu-2** pulls the pinned tag with `--platform linux/arm64`
(`experiments/367-qemu-user-aarch64.sh:63-66`).

**Clause qemu-3** resolves the absolute applet symlink and reads
`e_machine` with `readelf`
(`experiments/367-qemu-user-aarch64.sh:69-91`).

**Clause qemu-4** runs the foreign payload and requires exit 0, the payload's
output on stdout, and two stderr fragments: `is ELF machine 0xb7` and
`the musl object is 0x3e` (`experiments/367-qemu-user-aarch64.sh:95-104`).

**Clause qemu-5** reads `EnteredRung` (`experiments/367-qemu-user-aarch64.sh:108-115`).

**Pinned inputs.**
`public.ecr.aws/docker/library/alpine:3.20@sha256:45e09956dc667c5eff3583c9d94830261fb1ca0be10a0a7db36266edf5de9e1d`
(`experiments/367-qemu-user-aarch64.sh:29`), pulled by tag with
`--platform linux/arm64` (`:28`).

**Host assumptions.** A machine with `binfmt_misc` registered for aarch64 and
`qemu-aarch64-static` behind it. The binary path is
`/workspaces/pb365/podbox` and the scratch is `/tmp/pb367-work`
(`experiments/367-qemu-user-aarch64.sh:24-25`). Both are the wsl-toolkit base
layout, hard-coded rather than resolved from the repository
(`experiments/367-qemu-user-aarch64.sh:9-10`).

**Exit-code contract.** `0` every clause matched, `1` a clause disagreed, `2`
the base could not run. Note that the final line is `[ "$fail" -eq 0 ]`
(`experiments/367-qemu-user-aarch64.sh:126`), which yields 1 or 0 and can never
yield 2; the three `exit 2` arms are all earlier
(`experiments/367-qemu-user-aarch64.sh:45-47`, `:65`).

**Timeouts.** `timeout 600` on the pull and the extract, `timeout 300` on the
run, `timeout 120` on `rm`, `rmi` and `system info`
(`experiments/367-qemu-user-aarch64.sh:64`, `:70`, `:95`, `:105`, `:109`,
`:119`).

**Cleanup.** `rm -rf "$WORK"` at `experiments/367-qemu-user-aarch64.sh:26` and
`rm -rf "$STORE"` at `experiments/367-qemu-user-aarch64.sh:120`, plus
`podbox rmi` at `:119`. There is no `trap`; an abort between the pull and the
`rmi` leaves the image.

**Positive control.** Clause qemu-3 is the positive control for qemu-4: the
payload's `e_machine` is proved foreign before the run, so a green clause 4
cannot be a native payload. That ordering is the design and the comment at
`experiments/367-qemu-user-aarch64.sh:71-73` explains the applet-link detail.

**Failure control.** Yes, in clause qemu-4's shape: it requires BOTH the
payload output AND the decline text. A payload that ran because podbox served
an x86_64 object would produce the output and not the decline, and the clause
fails. The mechanism under test is at
`crates/podbox-cli/src/interpose.rs:238-246`, which returns
`Reach::Declined` when `obj.machine != elf.machine`.

**TODO entries that own it.** `TODO/interpose.md` T-1327
(`TODO/interpose.md:1597-1607` in the 2026-09-26 Done record, and the
`Fix area` list at `:1573-1576` names `scripts/nightly-smoke.sh` beside it).

**Result files it wrote.** `experiments/results/qemu-user-aarch64.txt`, taken
2026-09-26, 5 clauses, `fail=0`.

**Contradiction with an entry.** One. `TODO/interpose.md:1597-1598` says the
script "exits 0 (`experiments/results/qemu-user-aarch64.txt`): 5 clauses green
on the KVM base". The reading's conditions line says
`host kernel Linux 7.2.0-WSL2-STABLE` and `id 0:0`
(`experiments/results/qemu-user-aarch64.txt:3-4`) and the store path is
`/root/pb367/work/store` (`experiments/results/qemu-user-aarch64.txt:17`),
while the script's default scratch is `/tmp/pb367-work`
(`experiments/367-qemu-user-aarch64.sh:25`). The `PB_WORK` environment
variable overrode it, which the script permits at `:25`. The entry calls the
lane "the KVM base" and the file records a WSL2 kernel. Minor, and the entry's
own wording at `TODO/interpose.md:1599` says "on the KVM base (binfmt aarch64
with qemu-aarch64-static...)", which is consistent with what the file shows.

A second one, and it is a real problem. The saved reading at
`experiments/results/qemu-user-aarch64.txt:26` carries
`podbox rm: no such container: qemu367`. The script runs
`"$BIN" rm qemu367` at `experiments/367-qemu-user-aarch64.sh:105` and appends
its output to the report (`:105` redirects into `$WORK/report`). The run used
`--rm` at `experiments/367-qemu-user-aarch64.sh:95`, so the container was
already gone and the `rm` failed. The script does not check that exit code:
`|| true` at `:105`. A cleanup that always succeeds whether or not it did
anything is a cleanup that cannot fail, and
`docs/conventions/prose.md:56` lists the ✅ and ❌ markers for exactly this
distinction.

**Verdict: KEEP-SHELL.**

The specific host dependencies:

1. **A live `binfmt_misc` registration for aarch64**
   (`experiments/367-qemu-user-aarch64.sh:47`). The registration is
   machine-wide state that podbox reads but never writes
   (`TODO/enter.md:446-447`: "podbox reads the registrations and never writes
   one"). A test that created one would be changing the machine, which
   `TODO/enter.md:446` refuses for the product and which is no better for a
   test.
2. **A foreign-architecture registry pull and extract**
   (`experiments/367-qemu-user-aarch64.sh:64`, `:70`), which is
   `podbox-image` reaching a real registry for a real arm64 image.
3. **Execution under `qemu-aarch64-static`**, which the script never names
   explicitly: it relies on `binfmt_misc` to route the aarch64 payload, and
   `TODO/enter.md:495-498` records that this leaves no `binfmt_misc` trace when
   qemu is invoked explicitly, which is why the reader reports two stances.

**What must be fixed in the same change as any refactor.** The `rm` at
`experiments/367-qemu-user-aarch64.sh:105` should not run against a container
created with `--rm`, and its `|| true` hides the mismatch. Either drop `--rm`
from `experiments/367-qemu-user-aarch64.sh:95` and let `rm` do the work, or
drop the `rm` and let `--rm` do it. The saved reading shows which was intended:
neither, because the container was already gone.

**What the exit 2 path becomes.** Nothing. Three correct host states.

---

## Group 7 — `experiments/398-gate-diagnostics.py`

**What it measures.** That the gate runner prints the complete failed check's
output rather than the first twelve lines, that the PowerShell twin path
classifies an unavailable check as skipped rather than failed, and that the
JSON verdict reports both. The docstring is at
`experiments/398-gate-diagnostics.py:1-2`, "Drive gate failure output,
lost-output mutations, and skip states. T-1351."

**What it drives.** It copies `scripts/common/check-gate.sh` and
`scripts/common/check-gate.ps1` into a temp directory
(`experiments/398-gate-diagnostics.py:50-52`) and runs them against nine stub
checks, each of which is `exit 0` except a deliberately failing one
(`experiments/398-gate-diagnostics.py:40-56`).

**The four assertions per platform.**

1. A failing check that prints a marker after 20 control lines still reaches
   stdout and the runner exits 1
   (`experiments/398-gate-diagnostics.py:59-60`).
2. With `--json`, the marker does NOT reach stdout and `failed == 1`
   (`:61-63`).
3. A lost-output mutation, which truncates the log to 12 lines, removes the
   marker from stdout while still exiting 1 (`:64-71`). The mutation target
   for `sh` is `sed 's/^/          /' "$OUT/log" ;;`, at
   `scripts/common/check-gate.sh:103`. For `ps1` it is
   `Get-Content -LiteralPath $logFile -ErrorAction`, and the mutation inserts
   `-TotalCount 12`.
4. An unavailable twin, which exits 2, is counted as `skipped` and not
   `failed`, and `--strict` turns that into a failure
   (`experiments/398-gate-diagnostics.py:76-83`).

**Pinned inputs.** The check names are the constant `CHECKS` tuple at
`experiments/398-gate-diagnostics.py:14-16`, and the marker is
`FIXTURE-LATE-DIAGNOSTIC` at `:18`.

**Host assumptions.** `sh` on PATH, and `pwsh` on PATH when `--powershell` is
passed (`experiments/398-gate-diagnostics.py:33-37`). Python 3.

**Exit-code contract.** `0` the verdict, `2` a required interpreter is absent
(`experiments/398-gate-diagnostics.py:36-37`). There is no `1`: an assertion
failure raises `AssertionError` and the process exits non-zero by traceback
(`experiments/398-gate-diagnostics.py:60`, `:62`, `:63`, `:68`, `:71`, `:75`,
`:80`, `:83`). That is a real gap: `docs/methodology/experiments.md:12-13`
requires exit 1 for a tested mismatch.

**Timeouts.** `timeout=90` on every subprocess
(`experiments/398-gate-diagnostics.py:22`).

**Cleanup.** `tempfile.TemporaryDirectory` with a context manager
(`experiments/398-gate-diagnostics.py:38`), so cleanup is on both exits.

**Positive control.** The clean control at
`experiments/398-gate-diagnostics.py:73-75`: with the stub restored and the
runner restored, exit 0 and no marker.

**Failure control.** The mutation at
`experiments/398-gate-diagnostics.py:65-71` is the strongest failure control in
Group 7, and it is the model the repository should copy. It does not assert
that the marker appears; it asserts that the marker STOPS appearing when the
output is truncated, which is the only way to show the test is reading the
thing it claims to read. `assert mutation != source` at
`experiments/398-gate-diagnostics.py:68` is the guard that the mutation
reached its target at all, so a rename in the runner cannot turn the test
green.

**TODO entries that own it.** `TODO/gate.md` T-1351
(`TODO/gate.md:1812-1834`, Prove at `:1827-1829`).

**Result files it wrote.** None. `experiments/results/gate-diagnostics.txt`
exists and `TODO/gate.md:1834` links it as "The proof", but I read the script
and it never writes a file; it prints to stdout
(`experiments/398-gate-diagnostics.py:29-31`, `:84-85`). The seven-line file at
`experiments/results/gate-diagnostics.txt` holds the conditions and the two
`ok:` lines, which is stdout captured by something else. Which is a finding, not
a conversion blocker: `docs/methodology/experiments.md:18` says "Store proof
under experiments/results", and the file's content matches the script's stdout
exactly, so the operator redirected it. The script as committed does not.

**Contradiction with the entry.** One.
`TODO/gate.md:1827` says the Prove is `py experiments/398-gate-diagnostics.py
--powershell`, with both runners. The script defaults to the `sh` runner only
and prints `PowerShell profile: not requested; Windows proof uses --powershell`
(`experiments/398-gate-diagnostics.py:48`). The entry's Prove is correct and
the saved reading is not: `experiments/results/gate-diagnostics.txt:6` reads
`ok: ps1 late failure, lost-output mutation, JSON, clean control, and
unavailable twin`, so the reading WAS taken with `--powershell`, while
`experiments/results/gate-diagnostics.txt:3` says `host=Windows python=3.13.15`
and there is no record of which flag was used. The entry and the reading agree;
the script's default does not match either.

**Verdict: RUST-TEST.**

The subject is a shell script and a PowerShell script, so the test cannot be a
Rust test of Rust code. What it can be is a Rust test that drives them, which
is what it already is in a different language. The conversion has two halves
and they are not equal.

**Half one, which stays.** The mutation logic at
`experiments/398-gate-diagnostics.py:64-71` cannot become a Rust unit test,
because a Rust unit test cannot mutate a shell script in place and re-run it in
a way that proves anything the Python does not already prove. The subject is
two files of another language.

**Half two, which moves.** The gate itself is `scripts/common/check-gate.sh`
(186 lines) and `scripts/common/check-gate.ps1`. A Rust `podbox-gate` binary
(see `scripts/dev.sh` below) owning that logic makes both twins generated
from one implementation, which removes the twin comparison the runner exists to
support. That is the real win and it is a RUST-TOOL, not a test.

**Conversion plan for the half that can move.**

- Precondition: a temp directory holding nine stub checks and a copy of the
  runner.
- Action: run the runner against a check that fails and prints a marker after
  20 control lines; then again with `--json`; then against a mutated runner
  whose output is truncated; then against a check that exits 2.
- Expected: exit 1 with the marker present; exit 1 with the marker absent and
  `failed == 1`; exit 1 with the marker absent; exit 0 with `skipped == 1` and
  `failed == 0`, and exit 1 with `--strict`.
- Level: integration, per `docs/conventions/code.md:29`. The subject is a
  process boundary, and a pure test over a string would not prove the runner
  prints it.
- Target file: `crates/podbox-gate/tests/diagnostics.rs`, a new
  `[[test]]` target, or inline in `crates/podbox-gate/src/lib.rs`. Inline is
  correct: the assertion is about the binary's own behaviour, and a
  `tests/*.rs` file that spawns the binary under test needs the path resolved
  first, which `crates/podbox-ssh/tests/common.rs` solves for a crate with
  host dependencies and does not solve here.
- Fixtures. The nine check names and the marker string are the only fixtures
  (`experiments/398-gate-diagnostics.py:14-16`, `:18`). Both become constants
  in the new crate, asserted against `scripts/common/` by a test that reads the
  runner's own list, so a renamed check cannot make the driver pass.
- Plant. Replace the truncation at the runner's print site and the new test must
  fail with the marker missing. This is the mutation the script already
  performs, so the plant is the same mutation expressed in the project's
  `scripts/plant.sh` form. `scripts/plant.sh` has no case for it today; add one
  beside `case_plant "24 the export comparison removed"`
  (`scripts/plant.sh:391-395`) and add the new crate's source to `FILES` at
  `scripts/plant.sh:51`.
- Command. `cargo test -p podbox-gate`. On Windows also
  `cargo test -p podbox-gate -- --ignored` for the `pwsh` arm, since
  `experiments/398-gate-diagnostics.py:35` treats a missing `pwsh` as exit 2
  and the entry's Prove names the PowerShell runner.
- What exit 2 becomes. One condition, a missing interpreter
  (`experiments/398-gate-diagnostics.py:35-37`). In Rust that is a skipped
  `#[test]` with a reason naming `pwsh`, which `docs/conventions/code.md:34`
  requires: "A skipped test proves nothing about its subject."
- What becomes of the result file. Superseded.
  `experiments/results/gate-diagnostics.txt` is a stdout capture, not a
  result the script wrote. Delete it with the script and let the CI step
  `.github/workflows/gate.yml:204-205` be the record, or make the new Rust
  driver write it, in which case `TODO/gate.md:1834` stays accurate. The
  first is simpler and matches `AGENTS.md:70` ("Give each experiment a unique
  number. Keep its script and result."), which requires the result to have a
  script that produces it.

---

## Group 7 — `scripts/dev.sh`

**What it measures.** Nothing. It is a tool, and the entry says so.
`TODO/packaging.md:264-267` records the Decision as "A shell script over a
`Makefile`", with the reason: "`make` would have to model the dependency graph
cargo already models, and the two would drift; the one thing `make` buys that
cargo does not is running the bootstrap and the build behind a session's
reading, and that is what this is."

**What it does.** Five subcommands, `scripts/dev.sh:149-278`: `start` (default,
detach), `status`, `wait`, `build`, `check`. `start` forks with `setsid` and
returns in about a second, printing what to read while it works
(`scripts/dev.sh:112-147`). `check` runs fourteen steps cheapest-first
(`scripts/dev.sh:215-229`).

**Host assumptions.** `python3`, `cargo`, `setsid`, `scripts/common/bootstrap-env.sh`,
`scripts/build-interpose.sh`, `scripts/build-state.py`. Linux: `setsid` at
`scripts/dev.sh:125` and `kill -0` at `:61` are both POSIX and both absent on
Windows. `AGENTS.md:11-12` says the script selects the host procedure and
points elsewhere on Windows; `scripts/dev.sh:9-11` says the same: "THIS IS THE
LINUX LANE AND IT IS NOT THE ONLY ONE."

**Exit-code contract.** `0` the thing succeeded, `1` it failed, `2` it could
not run (`scripts/dev.sh:28`). `check` composes: `1` if any step failed or
none passed, `2` if any step skipped (`scripts/dev.sh:255-261`).

**Timeouts.** `PODBOX_DEV_WAIT:-1800` on `wait` (`scripts/dev.sh:177`, bounded
at `:179-186`), and `PODBOX_CHECK_STEP_TIMEOUT:-1800` on every check step
(`scripts/dev.sh:234`).

**Cleanup.** None needed; it is a long-lived process by design. The state lives
in `.dev/` under the repository, not `/tmp` (`scripts/dev.sh:35-41`, and
`TODO/packaging.md:273-274`).

**Positive control.** `check` refuses to report green on a run where nothing
passed (`scripts/dev.sh:255`, and the same rule in
`scripts/common/check-gate.sh:149-153`, which names the defect it exists to
catch).

**Failure control.** `scripts/plant.sh:397-398` is
`case_plant "25 the SKIP arm retargeted"`, which rewrites `scripts/dev.sh`'s
`2)` case arm to `9)` and asserts the holding check goes red. That is a real
plant for this script, and the only one in Group 7.

**TODO entries that own it.** `TODO/packaging.md` T-1005
(`TODO/packaging.md:217-285`, Prove at `:275` and Done at `:277-285`),
`TODO/gate.md` T-1207 (`TODO/gate.md:432-514`, whose Problem quotes
`scripts/dev.sh:199-202` and whose Prove at `:502-508` is four `dev.sh check`
clauses), `TODO/gate.md` T-1349 (`TODO/gate.md:1752-1771`, whose Prove at
`:1764` ends "full dev check returns 0"), and `TODO/probe.md:840` and
`TODO/probe.md:884`, which record a defect `scripts/dev.sh check` found on its
first run.

**Result files it wrote.** None of its own. It writes `.dev/build.log`,
`.dev/pid`, `.dev/status`, `.dev/last-build-inputs` and
`.dev/build-state.json` (`scripts/dev.sh:37-41`, `scripts/build-state.py:120`).
It runs four experiments as check steps
(`scripts/dev.sh:222`, `:225-227`), which write their own results.

**Contradiction with the entry, and it is a live defect.**

1. `TODO/gate.md:434` cites `scripts/dev.sh:199-202` as the source for
   T-1207's Problem. Those four lines are the `build` subcommand's tail
   (`scripts/dev.sh:196-201`), not the `check` subcommand. The `check` steps
   are at `scripts/dev.sh:215-229`. The citation points four lines above the
   `;;` that closes `build`, so a reader following it lands on the wrong
   subcommand. `docs/conventions/prose.md:24` requires citing the line that
   carries the claim.
2. `TODO/gate.md:441` says "every step of `./scripts/dev.sh check` is scoped to
   the workspace" and `TODO/gate.md:460-464` corrects it: "CORRECTED
   2026-09-21 BY READING THE TREE: the `Problem` above no longer holds as
   written." The live `check` at `scripts/dev.sh:216-219` and `:224` does run
   against the excluded crate, so the correction is right and the Problem
   above it is stale. `docs/conventions/prose.md:43-44` says correct it in
   place.
3. The dead stamp. `scripts/dev.sh:41` declares `STAMP`, three sites write it,
   and nothing in the tree reads it. T-1005's Done record at
   `TODO/packaging.md:281-285` describes the staleness check as hashing an
   input set, which is what T-1349's `build-state.py inputs` does
   (`scripts/build-state.py:39-77`) and what `scripts/dev.sh:163` calls. The
   stamp is the pre-T-1349 mechanism, still being written on every build.
4. `TODO/packaging.md:240` gives `scripts/dev.sh` a measured cost of "**1 s**"
   from `experiments/results/session-startup.txt`. I read that result file's
   existence in the results listing; I did not read its contents in this pass.
   What settles it: read `experiments/results/session-startup.txt` and check
   the figure is still labelled with its conditions.

**Verdict: RUST-TOOL.**

Crate: `crates/podbox-gate`, new. Binary: `podbox-gate`, from a `[[bin]]`
target. The repository already has ten `podbox-*` crates; the eleventh is
`crates/podbox-supervise` through `crates/podbox-windows`, and a gate tool
belongs beside them.

**CLI surface.**

```
podbox-gate start [--target TRIPLE]      # detach, print the reading list
podbox-gate status                       # running | ready | stale | failed | unknown
podbox-gate wait [--limit SECONDS]       # bounded, then report
podbox-gate build                        # one foreground build, no bootstrap
podbox-gate check [--step-timeout S]     # the fourteen steps, third-state reporting
```

Subcommand names and their default values are `scripts/dev.sh`'s
(`scripts/dev.sh:149`, `:190`, `:204`), the environment names are the same
(`PODBOX_TARGET` at `:43`, `PODBOX_DEV_WAIT` at `:177`,
`PODBOX_CHECK_STEP_TIMEOUT` at `:234`), and the state paths are the same
(`scripts/dev.sh:37-41`).

**Conversion plan.**

- Assertions today. There is no assertion, so the plan is stated as
  behaviours to hold, each as a triple:
  1. Precondition: no build running. Action: `podbox-gate start` twice.
     Expected: the second attaches and prints the first's pid; it does not
     spawn a second cargo (`scripts/dev.sh:114-119`).
  2. Precondition: a build in flight. Action: `podbox-gate status`. Expected:
     `running` and the log's tail, exit 0 (`scripts/dev.sh:153-157`).
  3. Precondition: a source file edited. Action: `podbox-gate status`. Expected:
     `stale`, exit 1, from `build-state.py status` returning 1
     (`scripts/build-state.py:92-93`).
  4. Precondition: a check step exits 2. Action: `podbox-gate check`. Expected:
     the step is named, `SKIP` is printed with the wording "it could not run
     here; nothing about its subject was verified", and the run exits 2
     (`scripts/dev.sh:239-244`, `:257-260`).
  5. Precondition: every step skipped. Action: `podbox-gate check`. Expected:
     red, because zero passes is not a green run (`scripts/dev.sh:255`).
- Level: integration and deployment. Per `docs/conventions/code.md:29`, this
  is the actual default path a contributor takes, and a unit test over a
  step-list would prove nothing about the steps running.
- Target file: `crates/podbox-gate/src/main.rs` for the subcommands, and
  `crates/podbox-gate/src/lib.rs` for the two decision functions worth testing
  pure: the third-state classification of a step's exit code, and the
  "nothing passed is red" rule. Both are currently inline `case` arms at
  `scripts/dev.sh:239-245` and `:255-261`.
- Fixtures and fakes. None new. `scripts/build-state.py` stays a Python
  helper and is called; there is no reason to rewrite it, and
  `TODO/gate.md:1752` T-1349 owns it. `scripts/common/bootstrap-env.sh` and
  `scripts/build-interpose.sh` stay shell, because the first installs
  packages and the second is Group 10's. The interpose crate is excluded from
  the workspace (`Cargo.toml:20`) and its steps keep their
  `--manifest-path` spelling.
- Plant. The existing plant at `scripts/plant.sh:397-398` already targets this
  file's third-state arm and must keep passing after the move, retargeted to
  the Rust source: change the `ExitCode::Code(2)` arm to a pass arm and the
  holding check must go red with its own message. The second case, for the
  zero-passes rule, does not exist and must be added. Add
  `crates/podbox-gate/src/lib.rs` to `FILES` at `scripts/plant.sh:51`.
- Command. `cargo test -p podbox-gate`. No feature flags and no target triple;
  the tool is host-native and `scripts/dev.sh` is the Linux lane by its own
  header (`scripts/dev.sh:9-11`).
- What exit 2 becomes. It is already a first-class state in the plan: the
  binary's `check` returns 2 on any skip, `wait` returns 2 on its bound
  (`scripts/dev.sh:183-187`), and `status` returns 2 on an unknown state
  (`scripts/dev.sh:171`). In Rust that is `std::process::ExitCode` with three
  arms, which is exactly what the exit code contract wants.
- What becomes of `.dev/last-build-inputs`. Delete it, with the three write
  sites. The live freshness answer is `scripts/build-state.py status`, which
  hashes input names, bytes, conditions and five output digests
  (`scripts/build-state.py:39-100`). Nothing reads the stamp; keeping it after
  the move would be a value in two places, which
  `docs/conventions/prose.md:36` forbids.

---

## Group 7 — `scripts/nightly-smoke.sh`

**What it measures.** That each per-arch release binary runs and does its
actual job. The header is at `scripts/nightly-smoke.sh:2-26`: six assertion
groups.

**Group 1** `version` exits 0 and prints the version line
(`scripts/nightly-smoke.sh:52-57`).

**Group 2** `version --verbose` exits 0 and names TRIPLE
(`scripts/nightly-smoke.sh:59-66`).

**Group 3** both interposer digests are 64 lowercase hex digits, never
`absent` (`scripts/nightly-smoke.sh:68-76`).

**Group 4** `crt-static` reads `yes` and the binary carries no `PT_INTERP`
(`scripts/nightly-smoke.sh:66`, `:78-83`).

**Group 5** the seeded-image leg: import, save, load, extract, read the
payload bytes back, then flip one byte in the layer blob and require the load
to refuse (`scripts/nightly-smoke.sh:114-158`). Native legs only.

**Group 6** the embedded objects' `e_machine` beside the artefact's own
(`scripts/nightly-smoke.sh:85-112`). On x86_64 legs the embeds must equal the
artefact; elsewhere the mismatch is printed and expected.

**Pinned inputs.** `BIN`, `TRIPLE` and `QEMU` as positional arguments
(`scripts/nightly-smoke.sh:30-32`). The payload is the literal
`smoke-payload\n` (`scripts/nightly-smoke.sh:132`). No registry, no quota, no
network (`scripts/nightly-smoke.sh:122-124`).

**Host assumptions.** `readelf` for group 4, `od` for group 6, `tar`, `dd`,
`mktemp`, and a writable temp directory. The caller is
`.github/workflows/nightly.yml:103`, which passes
`target/${{ matrix.triple }}/release/podbox ${{ matrix.triple }} ${{ matrix.qemu }}`.

**Exit-code contract.** `0` green, `1` ran and failed, `2` could not run
(`scripts/nightly-smoke.sh:27`, with `fail()` at `:34` and `unrun()` at
`:35`). This is the best-formed contract in Group 7: two named functions,
`SMOKE-FAIL` on stderr for both, and the two are distinguishable only by the
code, which is correct.

**Timeouts.** None. Every `podbox` invocation at
`scripts/nightly-smoke.sh:134-138` is unbounded. `AGENTS.md:72` requires
bounding external commands. In CI the job timeout is the only bound.

**Cleanup.** `trap "rm -rf '$SMOKE_WORK'"` at
`scripts/nightly-smoke.sh:128`, and an explicit `rm -rf` plus `trap -` at
`:156-157`.

**Positive control.** Group 5's honest leg at
`scripts/nightly-smoke.sh:134-140`: the fixture that must pass.

**Failure control.** Group 5's flipped leg at
`scripts/nightly-smoke.sh:141-155`. The comment at
`scripts/nightly-smoke.sh:142-146` explains why the corruption targets the
layer blob inside the OCI layout rather than the outer tar framing: "`load`
hashes each entry's bytes against the descriptor that names it, so a flipped
ENTRY fails closed with a digest mismatch. Flipping the outer tar's own bytes
only corrupts GNU tar headers the loader never hashes." That is a failure
control designed from the mechanism, which is the standard.

**TODO entries that own it.** `TODO/packaging.md` T-1314
(`TODO/packaging.md:289-446`, named at `:350-351` and `:454` and `:508`),
T-1329 (`TODO/packaging.md:504-553`, whose Done at `:539-548` names group 5
and the lane proof at `:550-553`), T-1328
(`TODO/packaging.md:449-500`, which names it as a Source at `:454`), and
`TODO/interpose.md` T-1327 (`TODO/interpose.md:1573-1576` names it in the
`Fix area`; `:1585-1596` records group 6).

**Result files it wrote.** None. It prints `SMOKE-OK`, `SMOKE-EMBED-MACHINE`
and `SMOKE-PULL-EXTRACT-OK` lines to stdout
(`scripts/nightly-smoke.sh:99`, `:107`, `:109`, `:140`, `:155`, `:160`).
`.github/workflows/nightly.yml:102-103` consumes those. Its results live in the
workflow runs, which `TODO/packaging.md:415-445` cites by run number.

**Contradiction with an entry.** None found. `TODO/packaging.md:364-370`
describes the smoke as asserting four things and the script has six groups;
the entry predates T-1329's group 5 and T-1327's group 6, and both later
entries describe the current six. The `Prove` at
`TODO/packaging.md:532-537` names the flipped-fixture failure and the script
implements it at `scripts/nightly-smoke.sh:152-155`. The entry and the script
agree.

**Verdict: RUST-TOOL.**

Crate: `crates/podbox-gate`, the same new crate as `scripts/dev.sh`, because
both are repository tooling and one crate with two binaries is fewer moving
parts than two crates. Binaries: `podbox-gate smoke` as a `[[bin]]` target
alongside the main one, or `podbox-smoke` as a second binary in the same
crate. Recommendation: one binary, `podbox-gate smoke BINARY TRIPLE [QEMU]`,
because the positional surface `.github/workflows/nightly.yml:103` already
passes is a script interface, not a CLI, and a subcommand is the smaller
change to the workflow.

**CLI surface.**

```
podbox-gate smoke BINARY TRIPLE [QEMU]
```

Exit codes as today: 0, 1, 2. Output as today: `SMOKE-OK`, `SMOKE-EMBED-MACHINE`,
`SMOKE-PULL-EXTRACT-OK`, `SMOKE-FAIL` on stderr.

**Conversion plan.**

- Assertions today, as triples. Six, one per group, each stated as
  precondition / action / expected:
  1. Precondition: a release binary for TRIPLE. Action: run `version`.
     Expected: exit 0 and stdout starting `podbox `.
  2. Precondition: the same. Action: run `version --verbose`. Expected: exit 0
     and a `target: TRIPLE` line.
  3. Precondition: the same. Action: read `interpose-gnu` and
     `interpose-musl`. Expected: each exactly 64 characters, all in
     `[0-9a-f]`.
  4. Precondition: the same. Action: read `crt-static` and scan the ELF
     program headers. Expected: `crt-static: yes` and zero `PT_INTERP`.
  5. Precondition: a scratch `PODBOX_STORE`. Action: import a one-file tar,
     save it, load it back, extract it, read the payload bytes; then flip one
     byte of the layer blob and load again. Expected: the honest leg reads
     `smoke-payload`; the flipped leg is refused.
  6. Precondition: the same. Action: read `e_machine` at byte 18 of the
     artefact with `od`, and read `interpose-gnu-machine` and
     `interpose-musl-machine` from the verbose document. Expected: on an
     x86_64 leg both equal the artefact's; elsewhere the difference is printed.
- Level: split, and this is the one script where the split is clean.
  - Groups 3, 4 and 6 read a FILE and a document, with no podbox process in the
    loop. Pure, per `docs/conventions/code.md:26`. The ELF reader is the same
    one the product uses: `crates/podbox-enter/src/abi.rs`, which
    `crates/podbox-cli/src/interpose.rs:252` calls as
    `podbox_enter::abi::Elf::read`. Its `mod tests` carries 15 tests today. The
    e_machine comparison in group 6 is exactly that type's job, and using it
    removes a second ELF parser from the tree.
  - Groups 1, 2 and 5 spawn the binary under test. Integration, per
    `docs/conventions/code.md:29`, and they must stay a process call: a test
    that links the binary under test into itself proves nothing about the
    binary under test.
- Target file and module. Pure half: `crates/podbox-enter/src/abi.rs` test
  block, beside the existing 15, for the ELF reads; and the e_machine
  comparison belongs in the new `crates/podbox-gate/src/smoke.rs`. Integration
  half: `crates/podbox-gate/tests/smoke_e2e.rs`, a new `[[test]]` target,
  because it needs the built artefact's path, which
  `crates/podbox-ssh/tests/common.rs` already solves for `podbox-ssh` and is
  the pattern to copy.
- Fixtures and fakes. The one-file tar at
  `scripts/nightly-smoke.sh:132-133` becomes a fixture built in Rust with the
  `tar` crate. `TODO/packaging.md:529-531` records the Decision: "Loopback
  pull+extract per arch. The fixture shape already exists in the registry-
  fixture work (T-0206 family); reuse it rather than inventing a second
  fixture." The honest fixture is a one-file layer. The flipped fixture is the
  same tar with one byte changed in the blob entry, and the mutation is
  mechanical once the tar is in memory.
- Plant. Two cases, because the script has two kinds of assertion. First, the
  negative arm: change the digest check in the load path and the flipped leg
  must fail to be refused. Second, the e_machine comparison: change
  `crates/podbox-gate/src/smoke.rs`'s x86_64 equality arm to a pass-through
  and the wrong-arch leg must go red. `scripts/plant.sh` has no case for this
  script; add both beside `case_plant "23a an interpose object over the
  ceiling"` (`scripts/plant.sh:382`), which is the existing model for an
  interpose-side mutation, and add `crates/podbox-gate/src/smoke.rs` to
  `FILES` at `scripts/plant.sh:51`.
- Command. `cargo test -p podbox-gate` for the pure half, and for the
  integration half the release binary must exist first:
  `cargo build --release --target x86_64-unknown-linux-musl && cargo test -p
  podbox-gate --test smoke_e2e`. Feature flags: the integration half needs
  `zig` on PATH for `ring`'s C, which
  `.github/workflows/gate.yml:144-148` installs in a bootstrap step for exactly
  this reason.
- What exit 2 becomes. Eight conditions today
  (`scripts/nightly-smoke.sh:37-42`, `:82`, `:92-94`, `:126`, `:131`, `:133`,
  `:150`). All eight are "a tool or a file is absent", and in Rust they become
  `#[ignore]` with the condition named, or `Result` short-circuits that print
  the reason and return 2. The distinction `docs/conventions/code.md:34`
  draws matters here: "A skipped test proves nothing about its subject", so a
  skipped `readelf` leg must say so and must not be counted as a pass. The
  runner's existing third-state discipline at `scripts/dev.sh:239-244` is the
  model.
- What becomes of the result files. None of its own. The workflow runs cited
  at `TODO/packaging.md:415-445` are the record, and they stay. The change to
  `.github/workflows/nightly.yml:103` is the binary name, nothing else, and
  the smoke's stdout lines are the matrix summary rows the publish job reads.

---

## Group 7 findings

### Contradictions

**F1. `experiments/260-multiarch.sh` clause 2 is a guard that cannot fire.**
`TODO/deps.md:810-812` states the clause "goes **red** the day `powerpc64le`
starts building, which is the point: it is how this project finds out rather
than a failure to suppress." The list is `BLOCKED=()`
(`experiments/260-multiarch.sh:76`), iterated at
`experiments/260-multiarch.sh:122`, so it has zero iterations. The event
already happened: `experiments/results/multiarch.txt:16` reads
`ok powerpc64le-unknown-linux-musl workspace checks` and `:20` reads `none`.
T-0912 closed the same day (`TODO/deps.md:832`). A second clause is needed, or
the claim in T-0912 must be corrected to say the guard was an empty list and
the finding came from reading it.

**F2. `TODO/deps.md:830` states seven architectures; the reading and the
correction say eight.** The Prove line is uncorrected and
`experiments/results/multiarch.txt:10-17` lists eight rows.
`TODO/deps.md:832-833` already carries the correction, so the entry
contradicts itself two lines apart.

**F3. `experiments/results/guest-usernet.txt` was not produced by
`experiments/361-guest-usernet.sh`.** The saved reading's last line reads
`360 acceptance` (`experiments/results/guest-usernet.txt:84`) against the
script's `361 acceptance` string (`experiments/361-guest-usernet.sh:323`), and
its rootfs path reads `.sweep360-work`
(`experiments/results/guest-usernet.txt:11`) against the script's
`.sweep361-work` (`experiments/361-guest-usernet.sh:36`).
`TODO/podvm.md:622-624` cites the result as 361's proof. Either the result is
from a 360-era script and the entry's citation is wrong, or the string is a
copy error. What settles it: `git log --follow` on the script and on the
result.

**F4. `TODO/gate.md:434` cites `scripts/dev.sh:199-202` for a claim about
`check`.** Those lines are in the `build` subcommand
(`scripts/dev.sh:190-203`). The `check` steps are at
`scripts/dev.sh:215-229`. `TODO/gate.md:460-464` already corrects the Problem
the citation supports, so the citation is dead weight pointing at the wrong
subcommand.

**F5. `experiments/results/distro-sweep.txt` is dated 2026-09-22;
`TODO/milestones.md` records the run as 2026-09-09.** The result's header reads
`Taken 2026-09-22T04:45:29Z` (`experiments/results/distro-sweep.txt:2`) and the
entry's Prove block at `TODO/milestones.md:468-475` says
`Prove, run 2026-09-09`. Thirteen days apart, and the entry does not record the
later run. The later run is current because the row transcripts in
`experiments/results/sweep/` carry it.

**F6. `experiments/398-gate-diagnostics.py` never writes
`experiments/results/gate-diagnostics.txt`, and `TODO/gate.md:1834` calls that
file "The proof".** I read the script; it prints to stdout
(`experiments/398-gate-diagnostics.py:29-31`, `:84-85`) and writes no file.
The seven-line result's content matches that stdout exactly. Somebody
redirected it. `AGENTS.md:70` requires a kept script and result; here the
result has no script that produces it.

**F7. `experiments/398-gate-diagnostics.py` has no exit 1.** Every assertion
failure is a bare `assert` (`experiments/398-gate-diagnostics.py:60`, `:62`,
`:63`, `:68`, `:71`, `:75`, `:80`, `:83`), so a mismatch exits through a
`traceback` with Python's code, not 1. `docs/methodology/experiments.md:12-13`
requires "1 for a tested mismatch".

**F8. `experiments/367-qemu-user-aarch64.sh` runs `rm` against a container
created with `--rm`, and hides the failure.** `--rm` is at
`experiments/367-qemu-user-aarch64.sh:95`, `rm` at `:105`, and `:105` ends in
`|| true`. The saved reading shows the consequence at
`experiments/results/qemu-user-aarch64.txt:26`:
`podbox rm: no such container: qemu367`.

**F9. `TODO/milestones.md` T-1112 does not list `experiments/362-windows-refusal.sh`.**
The `Source` line (`TODO/milestones.md:850`) names "experiments 369, 370, 371
and 392" and the `Prove` (`:867-869`) names 369, 370 and 392. The script's
only owner is that entry, by the script's own header
(`experiments/362-windows-refusal.sh:8`). Its result
(`experiments/results/windows-refusal.txt`, taken 2026-09-27) predates the
entry's `Partial 2026-09-30` record.

**F10. `TODO/extract.md:240` says six checks; the script prints twelve PASS
lines.** Six named checks A to F
(`experiments/220-extract-path-safety.sh:208-266`), but the refusal helper
emits one line per hostile case (`experiments/220-extract-path-safety.sh:202`),
so the count differs by six. A reader comparing the entry to a run sees a
disagreement that is only a counting convention.

### Dead code and unrun state

**F11. `scripts/dev.sh:41` declares `STAMP`; three sites write it; nothing
reads it.** `scripts/dev.sh:106`, `:200`, `:265` write
`.dev/last-build-inputs`. A tree-wide grep for `last-build-inputs` returns
`scripts/dev.sh:41` alone. The live freshness answer is
`python3 "$REPO/scripts/build-state.py" status` at `scripts/dev.sh:163`, added
by T-1349 (`TODO/gate.md:1752-1764`).

**F12. `experiments/398-gate-diagnostics.py:33` binds `powershell` and
`:35` uses it.** That is live. The unrun state is the default arm at
`experiments/398-gate-diagnostics.py:47-49`: without `--powershell` the
PowerShell runner is never exercised, and `.github/workflows/gate.yml:205`
invokes the script with no flag. So CI runs the `sh` half only, on Ubuntu
(`gate.yml:190`). The `ps1` half runs only when a human remembers the flag,
which `TODO/gate.md:1827` does. What settles it: whether the Windows lane
invokes it with the flag.

**F13. `experiments/240-distro-sweep.sh` and `experiments/260-multiarch.sh`
call `cargo` and `rustup` with no timeout.** `experiments/260-multiarch.sh:99`
runs eight `cargo check --workspace` calls unbounded, plus a release build at
`:184-186`. `AGENTS.md:72` requires bounding external commands. A hung cargo
holds the target lock and the next run looks like a hang, which is the exact
failure `scripts/dev.sh:26-27` names.

**F14. `scripts/nightly-smoke.sh` bounds nothing.** Every `podbox` invocation at
`scripts/nightly-smoke.sh:134-138` is unbounded. The CI job timeout is the only
bound, and `scripts/nightly-smoke.sh:334` of T-1314's problem statement named
"qemu timing flakes" as a known pitfall.

### Scripts with no owning entry, or an owning entry that does not name them

**F15. `experiments/10-build-target-image.sh` is owned by T-1213, whose
Prove the script no longer needs.** `TODO/gate.md:1127-1131` names it beside
`20` and `130`, and T-1213's whole scope was the engine conversion, which is
done. The milestone that used it, T-1101 (`TODO/milestones.md:61-78`), closed
on `130-probe-parity.sh`. The script's question is about a foreign runtime's
userspace (`experiments/10-build-target-image.sh:2-4`), which
`experiments/README.md:18-21` already frames as "The retained research
describes a measured floor. Probe the current host before a capability
claim."

### Entries whose acceptance command is a script with no automated Rust test

| Entry | Acceptance command | Status |
| --- | --- | --- |
| `TODO/extract.md` T-0304 `:237` | `./experiments/220-extract-path-safety.sh` exits 0 | Six of six checks now have crate tests; check F does not |
| `TODO/extract.md` T-0305 `:308` | check E of the same script | `crates/podbox-extract/src/safety.rs:543` and `:549` cover the decision; the on-disk verbatim storage (`crates/podbox-extract/src/apply.rs:172-177`) has no test |
| `TODO/extract.md` T-0301 `:50-51` | the script plus a `strace` clause | The `strace` clause has no test at all |
| `TODO/podvm.md` T-1305 `:511` | `./experiments/148-podvm-fleet.sh` | 6 of 10 clauses covered by `crates/podbox-cli/src/tier.rs:273` and `:295`; the two needs-machine refusals are not |
| `TODO/podvm.md` T-1306 `:608` | `experiments/361-guest-usernet.sh` | KEEP-SHELL; the entry's stated fix area |
| `TODO/deps.md` T-0911 `:732`, T-0912 `:830` | `./experiments/260-multiarch.sh` | Clause 6 convertible; clauses 4, 5, 7 are host |
| `TODO/podbox.md` (supervise) T-0602 `:513`, T-0603 `:187`, T-0604 `:215`, T-0605 `:260` | clauses of `230-lifecycle-loop.sh` | None has a crate test except T-0603's source-reading one, which `TODO/supervise.md:180-186` describes and the script's clause 3 duplicates |
| `TODO/milestones.md` T-1105 `:342` | `./experiments/230-lifecycle-loop.sh 20` | No Rust test; the entry is a count |
| `TODO/milestones.md` T-1106 `:398` | `./experiments/240-distro-sweep.sh` | KEEP-SHELL |
| `TODO/gate.md` T-1351 `:1827` | `py experiments/398-gate-diagnostics.py --powershell` | No Rust test; CI runs the `sh` half only (F12) |
| `TODO/gate.md` T-1213 `:1127` | `./experiments/10-build-target-image.sh` | DELETE candidate |
| `TODO/packaging.md` T-1005 `:275` | `./scripts/dev.sh` timing | No test; a tool |
| `TODO/packaging.md` T-1314 `:532`, T-1329 `:532` | `scripts/nightly-smoke.sh` | No Rust test; a tool |
| `TODO/milestones.md` T-1112 `:867-869` | 369, 370, 392 | 362 is unlisted (F9) |
| `TODO/interpose.md` T-1327 `:1597` | `experiments/367-qemu-user-aarch64.sh` exits 0 | KEEP-SHELL |

### Plants

`scripts/plant.sh` carries 44 `case_plant` calls. Exactly one targets a Group 7
script: `case_plant "25 the SKIP arm retargeted"` at
`scripts/plant.sh:397-398`, which rewrites `scripts/dev.sh`'s third-state arm
and asserts the holding check goes red. Nothing plants a defect against
`experiments/220-extract-path-safety.sh`, `experiments/148-podvm-fleet.sh`,
`experiments/398-gate-diagnostics.py`, `scripts/nightly-smoke.sh`,
`experiments/230-lifecycle-loop.sh`, or any of the four KEEP-SHELL scripts.
Every RUST-TEST and RUST-TOOL verdict above names the plant its new test needs,
and each is new work.

### Third-state inventory

| Script | Exit 2 conditions | A Rust successor for each? |
| --- | --- | --- |
| `10-build-target-image.sh` | no engine (`:31`) | no; host |
| `148-podvm-fleet.sh` | no binary, no tool, no parity table (`:63`, `:68`, `:93`) | all three disappear; the pure function needs no binary |
| `220-extract-path-safety.sh` | no python3, no binary, cannot craft (`:73`, `:74`, `:178`) | all three disappear; a Rust fixture that cannot build is a failing test |
| `230-lifecycle-loop.sh` | pull failed (`:60`), any clause skipped (`:231`) | the pull becomes `#[ignore]` naming the registry; `skipped` disappears |
| `240-distro-sweep.sh` | no engine, no binary, no toolchain (`:79`, `:80`, `:83`, `:101`) | no; host, and correctly so |
| `260-multiarch.sh` | no qemu, no binfmt, no build, no jq (`:180`, `:188`, `:220`, `:249`, `:302`, `:349`, `:369`) | four become `#[ignore]`; `jq` becomes `serde_json`; the builds belong to the gate matrix |
| `361-guest-usernet.sh` | no binary, no tool, qemu install failed (`:57`, `:74`, `:70`) | no; host |
| `362-windows-refusal.sh` | no cargo, no cc, bootstrap failed, no jq, no binary (`:57`, `:58`, `:63`, `:67`, `:85`) | all five disappear; the refusals do not need KVM, which `:13-14` says |
| `367-qemu-user-aarch64.sh` | no binary, no timeout, no binfmt, pull failed (`:45`, `:46`, `:47`, `:65`) | no; host |
| `398-gate-diagnostics.py` | no `sh`, no `pwsh` (`:35`) | one becomes `#[ignore]` naming `pwsh` |
| `scripts/dev.sh` | unknown status, wait bound (`:171`, `:186`) | already three states; becomes `ExitCode` |
| `scripts/nightly-smoke.sh` | eight tool and file conditions (`:37`-`:150`) | each becomes `#[ignore]` naming the tool, per `docs/conventions/code.md:34` |

---

## Group 7 — entries read

Each was read in full, not at the grep line.

| Entry | File and line | What it established about Group 7 |
| --- | --- | --- |
| T-0301 Extract in-process, never through system `tar` | `TODO/extract.md:22-71` | Owns 220 as a Prove clause; records that `tar::Builder` refuses a `..` member while the reader hands it on |
| T-0304 Refuse an entry resolving outside the destination | `TODO/extract.md:204-263` | Owns 220 checks A-D and F; names the mutation and the filesystem-not-exit-code rule |
| T-0305 An absolute symlink target is rootfs-relative | `TODO/extract.md:267-315` | Owns 220 check E; records that rebasing decides allowance and never what is stored |
| T-1005 A session reaches the code in one command | `TODO/packaging.md:217-285` | Owns `scripts/dev.sh`; records the Decision "A shell script over a `Makefile`" and the 1 s cost |
| T-1101 M0 the probe, and nothing else | `TODO/milestones.md:61-130` | The closed milestone; its Prove is `130-probe-parity.sh`, not `10` |
| T-1105 M4 the lifecycle, twenty times | `TODO/milestones.md:325-367` | Owns 230's loop; records the three failing runs at `stop` |
| T-1106 M5 environment completion, ten distributions | `TODO/milestones.md:371-475` | Owns 240; records the engine control and the voidlinux-misread |
| T-1112 A disposable guest that is not Linux | `TODO/milestones.md:848-894` | The intended owner of 362; its Source and Prove omit it (F9) |
| T-1203 A measurement on one host is a property of that host | `TODO/gate.md:133-202` | Owns the runner discipline 240 shares through `scripts/common/distro-matrix.sh` |
| T-1207 The excluded crate the gate does not check | `TODO/gate.md:432-514` | Owns four of `dev.sh check`'s steps; its Source citation is off by four lines (F4) |
| T-1211 Convert the distribution and probe engine scripts | `TODO/gate.md:881-931` | Owns 240's engine conversion; records the T-1309 fix that closed the last two rows |
| T-1213 Convert the target-image pair | `TODO/gate.md:1102-1166` | Owns 10; carries an inline transcript rather than a tracked result |
| T-0506 A foreign-architecture container, and never a rung measured by the emulator | `TODO/enter.md:378-511` | Owns 260 clauses 5 and 6; records the defect 260's clause 4 had |
| T-0602 (the `stop` race) | `TODO/supervise.md:490-525` | Owns 230's loop acceptance; records the 10/20, 9/20, 1/3 failures |
| T-0603 `PR_SET_PDEATHSIG` fires on the creating thread's exit | `TODO/supervise.md:144-188` | Owns 230 clause 3; records that the crate test replaces the /proc read |
| T-0604 Running state is launcher state | `TODO/supervise.md:192-233` | Owns 230 clause 2; names the flock mechanism and the `dead` verdict |
| T-0605 Capture logs at spawn | `TODO/supervise.md:236-286` | Owns 230 clause 4; records that `--log-driver` is not implemented |
| T-0214 A blob body cut off mid-stream is not retried | `TODO/image.md:1404-1470` | 240 is the Source; the fedora `no-pull` is quoted from `experiments/results/sweep/fedora.out` |
| T-0911 Per-architecture syscall numbers and the trap | `TODO/deps.md:690-788` | Owns 260 clause 1; records the `syscalls` gate as stale |
| T-0912 The powerpc gate is the crate's and it is stale | `TODO/deps.md:791-897` | Owns 260 clauses 3 and 7; its own Prove block records `exit 2` and the clause-2 guard is dead (F1) |
| T-1305 The fleet is podbox's job, spelled with the verbs it has | `TODO/podvm.md:470-531` | Owns 148; records the 10 driven 0 mismatches and the existing unit tests |
| T-1306 The non-goals are refusals the code makes | `TODO/podvm.md:535-635` | Owns 361; records the 2026-09-26 drive and the three findings it paid for |
| T-1314 Nightly releases, one tag builds and tests every arch | `TODO/packaging.md:289-446` | Owns `nightly-smoke.sh` groups 1-4; records the seven-arch recipe taken from 260 |
| T-1328 The nightly signs its artefacts | `TODO/packaging.md:449-500` | Names `nightly-smoke.sh` as a Source; no assertion of its own |
| T-1329 The per-arch smoke pulls and extracts | `TODO/packaging.md:504-553` | Owns `nightly-smoke.sh` group 5; records the lane proof and the corruption target |
| T-1327 (interpose coverage) | `TODO/interpose.md:1540-1607` | Owns 367 and `nightly-smoke.sh` group 6; records the 2026-09-26 aarch64 drive |
| T-1349 Verify build freshness from input and output bytes | `TODO/gate.md:1752-1771` | Supersedes `dev.sh`'s stamp (F11); its Prove ends "full dev check returns 0" |
| T-1351 Keep the failed check's complete diagnostic | `TODO/gate.md:1812-1834` | Owns 398; its Prove names `--powershell` and its result does not record the flag (F6) |

Twenty-seven entries read in full, against the ten required.
