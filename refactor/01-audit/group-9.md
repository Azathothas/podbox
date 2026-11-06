# Group 9 audit: conversion of 13 scripts

Scope: the 13 scripts named in `refactor/00-orientation/assignment-map.md`
`## Group 9 (13 scripts, 2927 lines)`. Every file below was read in full. No
experiment was run.

## Group 9 summary

| Verdict | Count | Lines |
| --- | --- | --- |
| DELETE | 0 | 0 |
| RUST-TEST | 4 | 1116 |
| RUST-TOOL | 2 | 723 |
| KEEP-SHELL | 2 | 650 |
| SPLIT | 5 | 438 |
| Total | 13 | 2927 |

Line counts are those quoted in the assignment map. The per-script line counts
below come from the same table.

### The 3 most important findings

1. **Four of the thirteen scripts already have their measurements written as
   Rust unit tests, and the saved results are the pre-conversion record.** The
   task treats these scripts as unconverted work. `crates/podbox-enter/src/abi.rs`
   carries tests that name the experiment files in their doc comments
   (`abi.rs:1168` "Check A of `experiments/results/interposer-abi.txt`, as a
   predicate", `abi.rs:1212` "**The `.symtab` trap, as an assertion rather than
   a comment**", `abi.rs:1246` "Check C, as the predicate rather than as a run").
   `crates/podbox-ssh/tests/session_interactive.rs:255-431` carries eleven
   `#[test]` functions that are the clauses of `388-interactive-shell.sh` by
   name. `crates/podbox-probe/src/nongoals.rs:227` carries five tests that are
   the stance clauses of `149-podvm-non-goals.sh`. `crates/podbox-complete/src/identity.rs`
   is named by `TODO/complete.md:659` as the replacement measurement. The
   remaining job is deletion plus a record, not authoring.

2. **The result file for `149` does not match the script that writes it.**
   `experiments/results/podvm-non-goals.txt` reads `ok: 360 guest networking
   green`. The script at `experiments/149-podvm-non-goals.sh:185` writes
   `pass "361 guest networking green"`. Git dates the result file to commit
   `84d16c3` and shows the script was edited afterwards at `ea6d321`, so the
   saved evidence is stale against the current text. `TODO/podvm.md:622` cites
   the same file and reports 15 driven, which matches the stale run, not a run
   of the current script.

3. **Two scripts have no owning `TODO/` entry, and one of those is a gate.**
   `experiments/394-ssh-package.sh` and `experiments/397-exported-build.py`
   appear in no `TODO/*.md` (grep for the basenames returns nothing). Both are
   named as required audit proofs in `experiments/README.md:37,40`, and
   `397` is cited as evidence in `TODO/gate.md:1793` without being named as its
   acceptance. `experiments/354-lifecycle-same-store.sh` is cited only in
   `docs/history/`, never in `TODO/`. A gate that no entry owns is a gate whose
   deletion nothing would object to.

## Group 9 - experiments/149-podvm-non-goals.sh

**What it measures.** Whether every non-goal in `podbox probe --json` is a
refusal the code makes. It reads `.non_goals[]` for six named rows
(`149:95-101`) and asserts a stance each (clause 0), that a refused row's
detail carries an errno matching `=[A-Z][A-Z0-9]*` (clause 1), that an open
row's detail carries no `refused` (clause 2), that an unestablished row says
`unestablished: ` (clause 3), that the kvm row is refused naming
`open(/dev/kvm, O_RDWR)=ENOENT` (clause 4), that the bare `probe` stderr
carries `non-goals, one stance per blocked design` (clause 5), and that
`361-guest-usernet.sh` exits 0 (clause 6).

**Pinned inputs.** No image. `$PODBOX_BIN` or
`target/x86_64-unknown-linux-musl/release/podbox` (`149:42`).

**Host assumptions.** `jq` on PATH (`149:62`, exit 2 without it). `set -u`
and no `pipefail` so dash can run it (`149:32-34`). Runs natively on Linux.

**Exit contract.** 0 all clauses green, 1 a clause failed, 2 the binary or jq
absent (`149:36-37`). Note clause 6 turns a failed `361` into exit 1, not 2,
so a red guest-network drive reads as a non-goal regression (`149:187`).

**Timeouts.** `timeout 120` per probe call (`149:77,79`),
`timeout 1500` on the `361` drive (`149:184`).

**Cleanup.** `WORK="$REPO/experiments/.sweep149-work"`, removed by trap
(`149:44-45`).

**Positive control.** Clause 0 and clause 2 both assert the shape of rows
this machine permits, and clause 4 asserts a lane-specific refusal.

**Failure control.** None. Every clause is a positive assertion over a
document. A `probe --json` that emitted an empty `.non_goals` array would fail
clause 0 rather than pass vacuously, so the shape is pinned, but no clause
injects a defect.

**Owning entries.** T-1306 (`TODO/podvm.md:578, 586, 622`), read in full.

**Result files.** `experiments/results/podvm-non-goals.txt`.

**Verdict: SPLIT.**

### Conversion plan - 149

**Assertions today.**

- A: precondition, `podbox probe --json` exits 0 and carries `.non_goals`;
  action, read each of six named rows; expected, each row's `stance` is one of
  `refused`, `open`, `unestablished`.
- B: precondition, a row's stance is `refused`; action, read its `detail`;
  expected, the detail matches `=[A-Z][A-Z0-9]*`.
- C: precondition, a row's stance is `open`; action, read its `detail`;
  expected, it does not contain `refused`.
- D: precondition, a row's stance is `unestablished`; action, read its
  `detail`; expected, it matches `unestablished: .+`.
- E: precondition, the `kvm acceleration` row exists; action, read its stance
  and detail; expected, stance is `refused` and the detail names
  `open(/dev/kvm, O_RDWR)=ENOENT`. **This clause asserts one lane's answer and
  is written to fail on a host with `/dev/kvm`.** It must not become a test
  assertion.
- F: precondition, the bare `probe` ran; action, read its stderr; expected, the
  non-goals block is present.
- G: precondition, `361-guest-usernet.sh` is executable; action, run it;
  expected, exit 0. This is a claim about a different script.

**Level.** Unit, pure, for A through D. `assess(&Findings) -> Vec<NonGoal>`
(`crates/podbox-probe/src/nongoals.rs:156`) is a pure function over
caller-supplied rows. **The tests already exist** at `nongoals.rs:227`
(`six_non_goals_come_back_in_order`, `a_denied_mechanism_refuses_naming_its_errno`,
`a_permitted_mechanism_turns_the_non_goal_off`, `a_missing_row_is_unestablished_rather_than_any_stance`,
`the_ceiling_row_names_its_number_or_says_there_is_none`). Per
`docs/conventions/code.md` "Use pure tests for deterministic logic", A through D
are pure logic on injected findings. The implementor adds a test only for the
gap: the six row names are hard-coded in the shell at `149:95-101` and are not
pinned by any test. Add `#[test] fn the_six_documented_rows_keep_their_names()`
in the same module, asserting the names array equals the six strings.

**Target.** `crates/podbox-probe/src/nongoals.rs`, inside the existing
`mod tests` at line 227. Inline, not `tests/`: the function is `pub` but the
test needs the private `findings` helper at line 232.

**Fixtures.** The `findings(rows: &[(&'static str, Outcome)])` helper already in
the module. No new fixture, and none from `experiments/lib/`.

**Plant.** Delete the `Outcome::Denied` arm in `assess` so a denied mechanism
returns `Stance::Open`, run the module's tests, and read the failure: it must
name `errno`. Then restore. A second plant renames the kvm row's detail string
and confirms `the_ceiling_row_names_its_number_or_says_there_is_none` or
`a_denied_mechanism_refuses_naming_its_errno` goes red. Record both in
`scripts/plant.sh` beside the existing 26 plants, per `AGENTS.md` "Add a plant
with each new check".

**Proof command.** `cargo test -p podbox-probe nongoals`. No feature flag and
no target triple: the module builds on the host.

**`exit 2` becomes.** The `jq` absence gate at `149:62` disappears; `assess`
has no external dependency. If a caller cannot build a `Findings`, that is a
compile error, not a third state. Where the report prints a row the host could
not measure, that is `Stance::Unestablished`, which is already a value and not
an exit code.

**Clause E** moves to no test. It is a live reading of one host. Keep the
capability as a saved result file.

**Clause F** becomes a pure test on the report renderer. Target:
`crates/podbox-probe/src/report.rs`, inside its existing `mod tests`. Assert
the rendered evidence string contains `non-goals, one stance per blocked
design` for a fixed `Findings`. That is pure and needs no binary.

**Clause G is not 149's to keep.** It asserts another script's exit code, so it
is a coverage claim, not a measurement of the non-goal assessment. Delete it
with the script, and point `TODO/podvm.md:615` at
`experiments/results/guest-usernet.txt` directly.

**Result file.** Keep `experiments/results/podvm-non-goals.txt` as the record
of the lane reading, and correct it or regenerate it. It currently says `360`
where the script says `361`. A reviewer cannot tell which script produced it.

## Group 9 - experiments/159-interpose-placement.sh

**What it measures.** Three clauses over the shipped binary and a pinned
alpine rootfs. A: `/.podbox/interpose.so` is a file inside the rootfs and the
payload's own `env` names it in `LD_PRELOAD` (`159:61`). B: a static payload
is declined by name and still runs, exit 0 (`159:84-98`). C: a dynamic payload
is not declined, the banner names the placed object, and the payload output
reaches stdout (`159:101-119`).

**Pinned inputs.** `public.ecr.aws/docker/library/alpine@sha256:d9e853e8...`
(`159:29`). `$PODBOX_BIN` or the musl release binary (`159:28`).

**Host assumptions.** A store the run can write (`159:44`, `PODBOX_STORE`
pointed at `$WORK/store`). `podbox extract` and `podbox inspect
--format {{.RootfsPath}}` must work (`159:70-79`).

**Exit contract.** 0 all clauses, 1 one failed, 2 the binary is not
executable (`159:38-42`). There is no third state for a failed pull: the run
returns 1.

**Timeouts.** None. Every `podbox` call is unbounded. This violates
`docs/conventions/code.md` "Bound buffers, retries, network operations, and
child waits".

**Cleanup.** `WORK=$(mktemp -d)` removed by trap (`159:31-36`). The report is
written to `experiments/results/interpose-placement.txt` directly
(`159:30,126-127`).

**Positive control.** Clause C, stated as such in the header (`159:16`).

**Failure control.** Clause A and B would both fail if `place` stopped
running. There is no injected-defect arm.

**Owning entries.** T-0702 (`TODO/interpose.md:266, 269`), T-0706
(`TODO/interpose.md:648, 659`), and T-0711 which cites clause A
(`TODO/interpose.md:455`). All read in full.

**Result files.** `experiments/results/interpose-placement.txt`.

**Verdict: SPLIT.**

### Conversion plan - 159

**Assertions today.**

- A: precondition, a pinned dynamic rootfs and a lane-built binary; action,
  `run --rm` a payload that tests the file and reads its own `env`; expected,
  `/.podbox/interpose.so` exists and `LD_PRELOAD` is exactly
  `/.podbox/interpose.so`.
- B: precondition, the same binary staged into the rootfs as `/podbox`;
  action, `run --rm` it; expected, stderr carries `interpose: declined`
  naming the path, and the exit is 0.
- C: precondition, a dynamic payload; action, `run --rm` it; expected, no
  decline line, a banner line `interpose: /.podbox/interpose.so`, and the
  payload's own output present.

**Level.** Clause A splits again. The part that is podbox's own logic is
`place(rootfs, object_bytes)` at `crates/podbox-cli/src/interpose.rs:296`,
which writes the object to `GUEST_PATH = "/.podbox/interpose.so"` (line 71) by
sibling plus rename, and `merge_preload` at line 348, which puts podbox's
object first where the caller set one. Both are pure over a directory and a
byte slice. That is a **unit** test on a deterministic input, per
`docs/conventions/code.md` "Use pure tests for deterministic logic". The module
already has 20 `#[test]` functions (`interpose.rs:469` onward). The implementor
adds `place` coverage: a temp rootfs, the object bytes, assert the file exists
at the guest path with those exact bytes, and assert `merge_preload` puts
podbox first.

**The rest is integration or deployment proof.** Whether the payload's `env`
carries the value depends on a real chroot entry and a real dynamic loader. Per
`docs/conventions/code.md` "Use integration and deployment proof for the actual
default path", that half stays a drive. A unit test that calls `apply` and
inspects an environment vector proves the function, not the default path.

**Target.** Unit part:
`crates/podbox-cli/src/interpose.rs`, inside `mod tests` at line 469. Drive
part: a new integration test in `crates/podbox-cli/tests/device_and_interpose.rs`
is **not** proposed here, because the crate has no `tests/` directory today
and the drive needs a pulled image plus a store. It belongs in the audit
driver, not in `cargo test`. The implementor keeps the drive as a `scripts/`
entry until the audit lands its own runner, and the unit test above is the part
that moves now.

**Fixtures.** `std::env::temp_dir()` with a per-pid name, the pattern already
used in `crates/podbox-enter/src/abi.rs:1095` and `abi.rs:1364`. No fixture
from `experiments/lib/`. The `include_bytes!` objects at `interpose.rs:38,41`
are build output copied by `build.rs`; a test that needs real object bytes
should read `object(Libc::Glibc)` at line 48 and skip when it is the empty
placeholder, the same honest third state `abi.rs:1228` already uses.

**Plant.** Change `GUEST_PATH` at `interpose.rs:71` to a second path, run
`cargo test -p podbox-cli interpose::tests`, and read the failure: the new
test must name the path it expected. Then restore. Without the plant, a
`place` that wrote to the wrong place and a test that asserted the same
constant would both be green.

**Proof command.** `cargo test -p podbox-cli interpose`. On a musl lane add
`--target x86_64-unknown-linux-musl` to match the binary under test; the unit
test itself needs no flag, because `interpose.rs` builds on the host.

**`exit 2` becomes.** The "binary is not an executable" gate at `159:38`
becomes a `cargo test` compile-time fact. A lane that cannot build the crate
reports a build failure, not a skip. Where the object bytes are the empty
placeholder, the test prints a SKIP line to stderr and returns, which is what
`abi.rs:1229` does and which `docs/conventions/code.md` permits only when the
subject is genuinely unmeasurable.

**Result file.** Keep. It is the record that the placement worked on a real
lane with a real loader.

## Group 9 - experiments/330-exit-codes.sh

**What it measures.** Whether podbox returns docker's exit code, case by
case, on this host, by running the same case through both binaries and
comparing the numbers (`330:8-12`). Section 0 reads podbox's own table from
`podbox system info --format '{{json .ExitCodes}}'` (`330:144`). Sections 1
through 6 are fifteen cases. Section 7 adds the userland-rung trio.

**Pinned inputs.** `public.ecr.aws/docker/library/alpine:3.20`
(`330:61`, overridable by `PODBOX_EXIT_IMAGE`). The driver
`public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe...` on a
non-native lane (`330:75`).

**Host assumptions.** An engine (`330:71`, exit 2 without one). `jq`
(`330:109`, exit 2 without it). A docker **daemon** for the comparison half
(`330:124-125`); on podman every docker column reads `-` and the run is
recorded as half-answered, exiting 2 at `330:368`.

**Exit contract.** 0 all agreed, 1 one disagreed, 2 could not run. Section 0
exits 1 immediately on a failed table read (`330:146-152`), which is right:
the table is the subject.

**Timeouts.** `timeout 300` per case (`330:190,193`), `600` for pulls
(`330:172,173`), `60` for short calls (`330:97,250`), `1500` is not here.

**Cleanup.** `trap 'eng_cleanup; rm -rf "$WORK"'` (`330:57`). On a
non-native lane the scratch is `experiments/.sweep330-work`, under the
checkout, because mount sources must be Windows paths there (`330:55`).

**Positive control.** Section 0 checks podbox against its own table before
comparing to docker (`330:199`). That is the control the header names.

**Failure control.** None. Every case asserts a positive expectation.

**Owning entries.** T-0802 (`TODO/cli.md:222, 270`), T-1414
(`TODO/cli.md:1440-1441, 1447-1458`), T-0409 via `TODO/complete.md:553`, and
T-1212 (`TODO/gate.md:949, 966, 1047`). All read in full.

**Result files.** `experiments/results/exit-codes.txt` and
`experiments/results/exit-codes-2026-09-30.txt`.

**Verdict: SPLIT.**

### Conversion plan - 330

**Assertions today.**

- A: precondition, the binary runs; action, read
  `system info --format '{{json .ExitCodes}}'`; expected, five case rows
  (`flag-error`, `cli-error`, `not-found`, `cannot-invoke`, `runtime-error`).
- B: precondition, a case the flag parser refuses; action, run it through
  podbox and through docker; expected, both give `flag-error`.
- C: precondition, a case the verb refuses after flags parse; action, same;
  expected, both give `cli-error`, which is 1.
- D: precondition, a payload with its own status; action, run it; expected,
  podbox gives the payload's status and docker agrees.
- E: precondition, the bare name; action, run with no arguments; expected, 0.
- F: precondition, a payload exiting 7; action, run it with and without
  `--no-source-fixup`; expected, both report 7.
- G: precondition, `--strict` on a machine below the namespace rung; action,
  run it and read `.StrictOk`; expected, `runtime-error` unless `StrictOk` is
  true, where 0 is also accepted (`330:286-295`).
- H: precondition, the userland rung; action, run the trio; expected,
  `not-found`, `cannot-invoke`, and `runtime-error` for a missing workdir.

**Level.** The codes themselves are pure data in
`crates/podbox-probe/src/exit.rs`, served as JSON
(`TODO/cli.md:253-254`). **Unit.** The existing guard named at `330:311` is
`resolve_codes_follow_the_payload_status`. Target: `crates/podbox-probe/src/exit.rs`,
inside its existing `mod tests`. The implementor adds the case-name set as a
test, so a renamed row cannot pass a comparison that reads it by name.

**Sections 1 through 4 are integration or deployment proof and must stay
drives.** They compare against a live docker daemon, and
`docs/conventions/code.md` says "A mock does not prove a live service or
deployment." A Rust test that shells out to a `docker` binary is a drive with
worse error reporting. Keep them.

**Clause F is already a Rust-test candidate at a different level.** It asserts
that `podbox_complete::complete()` never returns an exit code, which is the
structural claim at `TODO/complete.md:549-551`. Target:
`crates/podbox-complete/src/lib.rs`, asserting the return type carries no code
field, or a runtime test that a failing fixup leaves the report's failure row
and no status. That is pure over a fixture rootfs and belongs with the other
8 `#[test]` functions in that crate.

**Clause H belongs to T-1414** and the entry already records the state: "full
`330` exits 2 SKIP in this lane (no engine; the drive gates on one at its
head)" (`TODO/cli.md:1452-1454`). The entry names the alternative proof:
`experiments/356-no-chroot-rung.txt` clauses 7, 10 and 13. Move clause H out
of 330 into that drive's ownership, or keep it and record the engine gate.

**Target files.** Unit: `crates/podbox-probe/src/exit.rs` (case-name set) and
`crates/podbox-complete/src/lib.rs` (clause F). Drive: unchanged, `330` for
sections 1 through 6, `356` for the userland trio.

**Fixtures.** None new. The exit table is a constant. The completion fixture
is a temp directory, the pattern in `crates/podbox-enter/src/abi.rs:1095`.

**Plant.** Change `EXIT_NOT_FOUND` at `crates/podbox-probe/src/exit.rs:86`
to the `cli-error` value, run `cargo test -p podbox-probe exit`, and read the
failure: the case-name set test must go red, and so must
`resolve_codes_follow_the_payload_status`. Restore after. Record in
`scripts/plant.sh`.

**Proof command.** `cargo test -p podbox-probe exit` and
`cargo test -p podbox-complete`. No flags, no triple.

**`exit 2` becomes.** Three separate third states collapse. No engine is a
drive that cannot run, recorded as such in the result file. No `jq` disappears,
because a Rust test reads JSON through `podbox_probe::json` rather than a
shell. No daemon means the docker column is absent, which is a narrower
question, not a failure.

**Result files.** Keep both. `exit-codes.txt` is the last green run against
docker 29.8.1 and is the record for T-0802's table. `exit-codes-2026-09-30.txt`
is the SKIP run and is cited by `TODO/cli.md:1458`; it stays because it records
a real limitation of the lane, not because it proves anything.

## Group 9 - experiments/354-lifecycle-same-store.sh

**What it measures.** The full lifecycle in one store: `pull`, `run --rm`,
`create`, `start`, `exec`, `ps`, `logs`, `stop`, `wait`, `rm`, `ps -a`
(`354:76-86`). It exists to correct `353-open-issue-triage.sh`, whose
one-store-per-clause shape made every cross-clause verb answer
`no such container` (`354:5-10`).

**Pinned inputs.** The alpine digest at `354:26`.

**Host assumptions.** `cargo` on PATH (`354:35`, exit 2). The
bootstrap toolchain (`354:37`, exit 2 on failure). A 1800 s build
(`354:45`). A debug build at
`target/x86_64-unknown-linux-musl/debug/podbox` (`354:52`).

**Exit contract.** 0 the loop ran end to end, 1 a verb returned another code,
2 the lane could not run (`354:14-15`). The `step` function at `354:59-74`
reads each command's own `$?` and names the mismatch, which is the
discipline `AGENTS.md` requires.

**Timeouts.** `timeout 120` per step (`354:62`), `1200` bootstrap, `1800`
build.

**Cleanup.** `experiments/.sweep354-work` is removed at the start (`354:23`)
but **not by a trap**. A failed run leaves the store, the report and the build
log behind. This is a defect against `docs/methodology/experiments.md` "Remove
owned scratch only after its result is saved": the result is written to `/out`
at `354:93` but the scratch is never removed after it.

**Positive control.** None beyond the expected exit of 0 on every step.

**Failure control.** None. No step expects a non-zero code, so a drive that
asserted nothing would be indistinguishable from this one.

**Owning entries.** None. Grep for `354-lifecycle-same-store` in `TODO/*.md`
returns nothing. It appears in `docs/history/CHANGELOG.md-before-2026-09-30.txt:159`,
`docs/history/progress-before-2026-09-28.txt:53` and
`docs/history/session-2026-09-25-to-27.md:25`.

**Result files.** `experiments/results/lifecycle-same-store.txt`,
verdict `LOOP COMPLETE` at line 132.

**Verdict: DELETE.**

### Evidence for DELETE

The measurement it exists to correct is `353-open-issue-triage.sh`, and `353`
carries its own entry. `354` is a corrective re-run with no entry, no
acceptance, and no TODO citation. The lifecycle it drives is
[T-0607](TODO/supervise.md:493) "The lifecycle, twenty times, twenty passes",
and `230-lifecycle-loop.sh` already runs that loop twenty times with an entry.
`359-supervision-split.sh:139` calls `230-lifecycle-loop.sh 20` for exactly
this. Running it once more through a second script adds no assertion that
`230` does not already make, and adds a store leak.

The proposed replacement is already in place: `230-lifecycle-loop.sh` with
`crates/podbox-supervise`'s tests. The implementor deletes `354`, keeps
`experiments/results/lifecycle-same-store.txt` as the historical record, and
leaves the history citations alone. `docs/history/` is the home for superseded
evidence per `docs/conventions/prose.md` "Correct text in place", and those
three files are already history.

**If the operator keeps it**, the minimum is a TODO entry and a trap, and the
verdict becomes RUST-TEST for the one thing it adds over `230`: nothing. Do
not spend a conversion on it.

## Group 9 - experiments/359-supervision-split.sh

**What it measures.** Five clauses on a lane where syscall mediation is
refused but pidfd and waitid hold. 1: `tiers.supervise` carries three legs and
`refusal` is null exactly when every leg is ok (`359:90`). 2:
`tiers.supervision` carries two legs and a null refusal (`359:97`). 3:
`probe --rows` names `pidfd_open(own pid)` and `waitid(P_PIDFD, child)` as ok
(`359:111-117`). 4: the run banner carries no `no syscall mediation` line
(`359:131`). 5: the 230 loop passes twenty of twenty (`359:139`).

**Pinned inputs.** The alpine digest at `359:35`.

**Host assumptions.** `cargo`, `cc`, and after the bootstrap `jq`
(`359:44-45, 57`, each exit 2). `./scripts/build-interpose.sh` must succeed
(`359:59`, exit 2). A 1800 s debug build (`359:67`).

**Exit contract.** 0 all clauses, 1 a clause disagreed, 2 the lane could not
run (`359:23-24`).

**Timeouts.** `timeout 120` for probe calls, `timeout 600` for the pull
(`359:121`), `timeout 1500` for the loop (`359:139`), `1200` bootstrap,
`1800` build.

**Cleanup.** `experiments/.sweep359-work` is removed at the start
(`359:32`) and never by a trap. Same defect as 354.

**Positive control.** None across the five. Clause 1's jq expression is a
genuine biconditional, so a leg that failed with a null refusal fails it.

**Failure control.** The header is honest about this: "The denied-notify shape
(mediation refused beside held supervision, banner line on) is pinned by unit
tests: the lane permits all three notify legs, so no lane run can show the
fallback" (`359:19-21`). **This clause set has no negative case it can run.**

**Owning entries.** T-0606 (`TODO/supervise.md:476`), read in full.

**Result files.** `experiments/results/supervision-split.txt`.

**Verdict: SPLIT.**

### Conversion plan - 359

**Assertions today.**

- A: precondition, `probe --json` exits 0; action, read `tiers.supervise`;
  expected, `legs | length == 3` and `((all ok) == (refusal == null))`.
- B: precondition, the same document; action, read `tiers.supervision`;
  expected, two legs and a null refusal.
- C: precondition, `probe --rows` exits 0; action, grep the two row names;
  expected, both present.
- D: precondition, a run over a pulled image; action, read its stderr;
  expected, no `no syscall mediation` line.
- E: precondition, the just-built binary; action, run
  `230-lifecycle-loop.sh 20`; expected, exit 0.

**Level.** Clauses A and B are **pure**. They are jq expressions over a JSON
document podbox produced. The document is built in Rust, so the assertion
belongs beside the builder. Target: `crates/podbox-probe/src/report.rs` for the
document shape, or `crates/podbox-probe/src/supervise.rs` where
`SUPERVISION_LEGS` is defined (`TODO/supervise.md:467`). The implementor
asserts the leg count and the refusal biconditional against a constructed
document. `docs/conventions/code.md`: "Use pure tests for deterministic logic."
Per `TODO/supervise.md:484-489` the denied-notify shape is already unit-pinned;
this is the permitted case, so the implementor confirms the existing test
covers the biconditional before adding one.

**Clause C is integration proof** and stays a drive: it runs the probe's row
runner and reads real verdicts from real syscalls. A unit test would prove the
list, not the kernel.

**Clause D is where the negative case lives, and the lane cannot run it.** The
banner text is chosen in Rust, so the selection is a pure function over two
booleans. Target: wherever the run banner is composed,
`crates/podbox-cli/src/run.rs` (17 `#[test]` functions today). Assert: mediation
refused and supervision held produces the line; mediation ok produces no line.
That is the denied-notify shape the header says no lane can show, and a unit
test is the only level that can. This is the single most valuable conversion in
this group.

**Clause E is another script's exit code** and is not 359's measurement. It
belongs to T-0607 and `230`. Remove it from 359's obligations.

**Target files.** `crates/podbox-probe/src/supervise.rs` (clauses A, B),
`crates/podbox-cli/src/run.rs` (clause D). Both inline `mod tests`; neither
crate has a `tests/` directory.

**Fixtures.** A constructed findings list, the pattern in
`crates/podbox-probe/src/nongoals.rs:232`. No fixture from `experiments/lib/`.

**Plant.** Invert the banner condition in `run.rs` so the fallback line prints
where mediation holds, run `cargo test -p podbox-cli run::tests`, and read the
failure: the new test must name the line it expected absent. Restore. Then
change `SUPERVISION_LEGS` to three entries and confirm the leg-count test goes
red.

**Proof command.** `cargo test -p podbox-probe supervise` and
`cargo test -p podbox-cli run::tests`. No flags, no triple.

**`exit 2` becomes.** The `cc`, `jq` and bootstrap gates all disappear for the
converted clauses; they exist only to run the drive. The drive keeps its own
`exit 2` because a drive is still a shell entry until the audit lands its own
runner. The banner selection has no third state: two booleans always yield a
line or its absence.

**Result file.** Keep. It is the lane record, and its verdict line
`SUPERVISION SPLIT HOLDS` is what `TODO/supervise.md:476` cites.

## Group 9 - experiments/363-device-map.sh

**What it measures.** Whether `--device HOST[:GUEST[:PERMS]]` maps a host path
into the payload. Eleven clauses, described at `363:11-33`: byte-identical
read-back (1), `/dev/zero` (2), a six-row opener matrix with numeric flags
(3), served creation and `EEXIST` through a missing parent (3b), the
image-shadow boundary by read-back (3c), honest `stat` ENOENT (3d), a missing
host path at 125 (4), the machine tier at 125 (5), the musl payload served and
the static one honestly ENOENT (6), the create/start/logs round trip (7), and
`exec` re-serving the record's mapping (8).

**Pinned inputs.** The debian digest at `363:46` and the alpine digest at
`363:47`. `experiments/363-opener.c` compiled twice, dynamic and static
(`363:87-88`).

**Host assumptions.** `gcc` on PATH (`363:86`, exit 2). The binary and jq-free
`podbox_exit_codes` helper (`363:64-65`, exit 2).

**Exit contract.** 0 all clauses, 1 a clause disagreed, 2 no binary or no pull
(`363:35-36`). Every pull failure is 2 (`363:75-78`).

**Timeouts.** `timeout 600` per pull, `timeout 120` per case.

**Cleanup.** `experiments/.sweep363-work` removed at the start (`363:44`),
never by a trap. Containers `d363` and `d363e` are removed at `363:264,287`
but only on the success path; a failure between create and rm leaves them.

**Positive control.** Clause 2 (`/dev/zero`) beside clause 1 (a real file), and
the static opener beside the dynamic one in clause 6. The seeded sha256 at
`363:81` is the byte-identity witness.

**Failure control.** The opener matrix at `363:142-159` is nine negative rows
against three positive ones. A `serve` that returned a descriptor where the row
grants nothing would fail three rows, not pass.

**Owning entries.** T-0501 (`TODO/enter.md:100-101`), read in full.

**Result files.** `experiments/results/device-map.txt`.

**Verdict: SPLIT.**

### Conversion plan - 363

**Assertions today.**

- A: precondition, a `--device` row with `r` perms; action, `openat` the guest
  spelling with `O_RDONLY`; expected, a descriptor.
- B: precondition, the same row; action, `openat` with `O_WRONLY` and
  `O_RDWR`; expected, `errno=13`.
- C: precondition, a `w` row; action, `openat` with `O_RDONLY`; expected,
  `errno=13`.
- D: precondition, a `w` row; action, `openat` with `O_WRONLY`; expected, a
  descriptor.
- E: precondition, an `rw` row; action, `openat` with `O_RDWR`; expected, a
  descriptor.
- F: precondition, an `rw` row over a missing parent; action, `openat` with
  `O_CREAT`; expected, a descriptor, and the node's own semantics.
- G: precondition, the same; action, `openat` with `O_CREAT|O_EXCL`; expected,
  `errno=17`.
- H: precondition, a served-only path; action, `stat`; expected, ENOENT, and
  the read names the row.
- I: precondition, a missing host path; action, `run --device`; expected,
  `runtime-error` (125) naming the path.
- J: precondition, the machine tier; action, `run --podbox-tier=machine
  --device`; expected, `flag-error` (125) naming the chroot tier.
- K: precondition, a static payload; action, `openat` the served path;
  expected, `errno=2`, no interposer.
- L: precondition, a container record carrying the spec; action, `start` then
  `logs`, and `exec` with no flag; expected, the mapping is served on both.

**Level.** Assertions A through G are **pure**. `crates/podbox-interpose/src/device.rs`
has six tests at line 252: `find_matches_exact_spelling`,
`find_rejects_near_spellings`, `find_skips_malformed_rows`,
`answer_checks_perms_by_access_mode`, `answer_falls_through_where_no_duplicate_reproduces`,
`answer_serves_creation_as_the_node_itself`. Those cover the shape of every
matrix row. The implementor's remaining unit work is the flag shapes the shell
passes numerically: assert `answer` for `O_WRONLY|O_CREAT` (0101) and
`O_WRONLY|O_CREAT|O_EXCL` (0301) on a device, which is G and the row 3b shell
calls `m7` and `m8`. The parse half is already pinned by eight tests in
`crates/podbox-enter/src/device.rs:308`.

**Clause I and J are CLI refusal paths** and belong where the flag is
dispatched. Target: `crates/podbox-cli/src/parity.rs` (15 `#[test]`
functions) for the `--device` on the machine tier refusal, and the machine-tier
gate itself. These are pure over a parsed argument list.

**Clauses K and L are integration or deployment proof.** K needs a real static
binary with no `PT_INTERP` and a real loader that reads nothing; L needs a
container record on disk, a launcher, and a re-entering `exec`. Per
`docs/conventions/code.md` "A mock does not prove a live service or deployment",
both stay a drive.

**Target files.** Unit: `crates/podbox-interpose/src/device.rs` (creation
rows), `crates/podbox-enter/src/device.rs` (parse, already present),
`crates/podbox-cli/src/parity.rs` (tier refusal). Drive: unchanged.

**Fixtures.** The `table()` helper at `crates/podbox-interpose/src/device.rs:255`
already builds a device table for a test. `experiments/363-opener.c` is the
drive's own fixture and stays with the drive; nothing else in
`experiments/lib/` applies.

**Plant.** Delete the `O_EXCL` branch in `answer`, run
`cargo test -p podbox-interpose device::tests`, and read the failure: a
`O_WRONLY|O_CREAT|O_EXCL` open must come back with the device's own descriptor
where the test expects `EEXIST`. Restore.

**Proof command.** `cargo test -p podbox-interpose device` and
`cargo test -p podbox-cli parity`. No flags, no triple.

**`exit 2` becomes.** `gcc` missing and pull failure are drive states. The
unit tests need neither. Where the interposer crate's serve has no hook for
the requested operation, that is `answer_falls_through_where_no_duplicate_reproduces`
passing, which is a value not a code.

**Result file.** Keep. It is the record `TODO/enter.md:101` cites.

## Group 9 - experiments/388-interactive-shell.sh

**What it measures.** Whether an interactive SSH session works with no pty
anywhere. Eleven clauses over a real `sshd` on loopback with `ForceCommand` set
to the lane-built `shell` binary, driven by a real `ssh` with piped
descriptors: the premise (`test -t` false on both descriptors, clause 3),
editing, history, state, a signal to the command group, the exit code, an exec
refusal, a subsystem probe, and an untrapped SIGINT reporting 130.

**Pinned inputs.** No image. `cargo build -p podbox-ssh --bin shell`
(`388:43`).

**Host assumptions.** `/work` as the working directory (`388:18`, exit 2
otherwise). `ssh`, `sshd`, `ssh-keygen` on PATH (`388:38-40`). The bootstrap
(`388:36`, exit 2). Port 2222 free. `/run/sshd` created (`388:37`).

**Exit contract.** 0 all clauses, 1 a clause disagreed, 2 the lane could not
run (`388:15`).

**Timeouts.** `timeout 25m` on the build (`388:43`), `timeout 60` on most
sessions, `timeout 120` on clause 7, `timeout 20` on clause 10, `timeout 10`
per readiness poll.

**Cleanup.** `mktemp -d` removed by trap (`388:21-22`); the `sshd` is killed at
`388:270`. Keys stay in `$work` and never travel (`388:276`).

**Positive control.** Every needle is a computed marker whose expanded form
never occurs in the typed bytes (`388:12-14`). That is the needle rule and it
is the anti-vacuity control for a discipline that echoes everything.

**Failure control.** The `dropbear` and reverse cases are absent here because
there is no guest. There is no arm that injects a defect into the shell.

**Owning entries.** T-1402 (`TODO/podssh.md:79`), read in full.

**Result files.** `experiments/results/interactive-shell.txt`.

**Verdict: RUST-TEST.**

### Conversion plan - 388

**This conversion is already done.** The target exists:
`crates/podbox-ssh/tests/session_interactive.rs`, and its module comment at
lines 1-21 states the needle rule that `388:12-14` states. Its twelve
`#[test]` functions map to the script's clauses one for one:

| Script clause | Test function (line) |
| --- | --- |
| 3, premise and prompt | `prompt_echo_command_and_no_terminal` (255) |
| 4, editing | `editing_repairs_a_typo` (276) |
| 5, history | `history_recalls_the_previous_command` (288) |
| 6, state | `state_persists_across_commands` (304) |
| 7, signal to the group | `signal_kills_the_command_not_the_shell` (323) |
| 11, untrapped 130 | `signal_without_trap_ends_the_session_with_130` (351) |
| 8, exit code | `shell_exit_code_passes_through` (374) |
| 9, exec refusal | `one_shot_command_is_refused_naming_the_variable` (389) |
| 10, subsystem | `subsystem_sftp_never_reaches_the_shell` (410) |
| none | `ctrl_d_on_empty_line_ends_the_session` (431) |
| none | `client_eof_submits_the_partial_line_then_ends` (450) |
| none | `large_output_does_not_wedge_the_session` (472) |

The test file adds two clauses the script does not have and drops nothing.

**Level.** Integration. It is a real `sshd` and a real `ssh`, which is what
`docs/conventions/code.md` means by "Use integration and deployment proof for
the actual default path". The file says so at line 3: "Each test starts a real
`sshd -i` whose `ForceCommand` is the real `shell` binary".

**Target.** `crates/podbox-ssh/tests/session_interactive.rs`, an existing
integration test. `crates/podbox-ssh/tests/` is the only crate with a
`tests/` directory, and this file is the reason.

**Fixtures.** `crates/podbox-ssh/tests/common.rs`: `require_binary` (28),
`fresh_tmp` (48), `make_keys` (68), `current_user` (89), `ensure_privsep`
(103), `run_ssh` (112), `collect` (155), `wait_for` (194). These replace the
script's `ssh-keygen` invocations at `388:61-62`, the readiness poll at
`388:81-92`, and the output capture at every clause.

**Plant.** Replace the `ECHO` handling in the discipline so the typed bytes are
not echoed, run `cargo test -p podbox-ssh --test session_interactive`, and read
the failure. The editing test must go red, because `editing_repairs_a_typo`
cannot pass when the line discipline stops echoing. Without this plant, a
discipline that echoed nothing would make the editing and history tests fail,
which is the point, but a discipline that echoed everything would make them
pass vacuously, which is the trap `388:12-14` names. Record it in
`scripts/plant.sh`.

**Proof command.**

```sh
cargo test -p podbox-ssh --test session_interactive
```

On a lane where the tools are absent the tests skip through `require_binary`.
No feature flag. No target triple: the fixture runs the host's `ssh` and
`sshd`, so a foreign-triple build does not change what is proved.

**`exit 2` becomes.** `require_binary` at `common.rs:28` is the third state.
The test prints a skip and returns when a binary is absent, which is the
honest form. The `cd /work` requirement at `388:18` disappears: cargo already
runs from the workspace root, and the fixture takes the crate root itself.

**Result file.** Keep `experiments/results/interactive-shell.txt` as the record
of the lane drive. It carries the clause list and the verdict
`INTERACTIVE SESSION WITHOUT PTY HOLDS`. `TODO/podssh.md:79` names the script
in its `Prove`; the implementor rewrites that line to name the test file and
the cargo command, and cites the result file as the historical lane run.

**Remaining work is small.** Delete the script, update `TODO/podssh.md:79` and
`TODO/podssh.md:126` where it appears in T-1404's Prove, and record the
deletion in `docs/history/`.

## Group 9 - experiments/391-machine-bridge.sh

**What it measures.** Six clauses over `experiments/lib/machine-bridge.c`, a
host C shim: a static build, the usage refusal at 125, bytes both ways and the
server's status through a pty, the pinned dropbear banner, three failure edges,
raw mode with VMIN and VTIME, and the half-close that releases a wedged server.

**Pinned inputs.** dropbear at commit `59870ad43153fe8d4f1c96f5d5752116c94f31ff`
from `https://github.com/mkj/dropbear` (`391:34-35`). `zig` for the static
build (`391:61`).

**Host assumptions.** `/work` (`391:24`, exit 2). The bootstrap with
`rust cc zig openssh tools qemu vmtools` (`391:50`, exit 2). Then
`python3 timeout file zig make cc git` each on PATH (`391:51-53`, exit 2).
Network access to github.

**Exit contract.** 0 all clauses, 1 a clause disagreed, 2 the lane could not
run (`391:21`).

**Timeouts.** `timeout 300` on the zig build, `timeout 120` on the git fetch,
`timeout 60` on each pty case, `timeout 10` on each failure edge.

**Cleanup.** `mktemp -d` removed by trap (`391:29-31`). The trap also copies
the report to `/out`.

**Positive control.** The half-close case at `391:373-420` first proves the
splice is live both ways by reading an echoed marker, then kills the tty end.
The raw-mode case proves its own fixture at `391:329` by asserting the device
starts cooked.

**Failure control.** Clause 4 is three negative arms: 125 for an unopenable
tty, 127 for an unexecable server, 137 for a signalled server (`391:288-313`).
Clause 2's pipelined case feeds a megabyte past a dead server and requires the
server's status, not the write's (`391:193`).

**Owning entries.** T-1404 (`TODO/podssh.md:126`), read in full.

**Result files.** `experiments/results/machine-bridge.txt`.

**Verdict: KEEP-SHELL.**

### Why KEEP-SHELL

The host dependency is specific and not replaceable by a Rust test.

1. **The subject is a C file that must link static against musl.** The subject
   is `experiments/lib/machine-bridge.c`; the build is
   `zig cc -target x86_64-linux-musl -static` (`391:61-62`). A Rust test that
   only read the C source would prove the source parses, not that the shim
   links. `scripts/zig-cc.sh` exists for this and `docs/agent-tooling.md` is
   its home.
2. **The pty is the instrument.** Nine of the eleven assertions are Python
   `pty.openpty` plus `termios.tcgetattr` (`391:103, 153, 231, 325, 373`).
   Clause 5 reads the termios control characters at index 6 of `tcgetattr`
   (`391:349-350`). That is a host terminal.
3. **The pinned third-party server is built from a fetched upstream commit**
   with `./configure --enable-static` and `make` (`391:218-220`). No Rust crate
   substitutes for a dropbear autotools build.
4. **The guest composition is another script.** `390-machine-ssh.sh` builds the
   same source at `390:215`. A Rust test would duplicate a C build in two
   places, which is `docs/conventions/code.md` "Do not add duplicate
   implementations".

**What the operator can still take.** The measured *contract* of the shim moves
into Rust as documentation and as unit tests over a fake: 125 on an unopenable
tty, 127 on an unexecable server, 137 on a signalled server, and the half-close.
Those are the exit edges at `391:288-313`, and they are worth pinning in
`crates/podbox-ssh` as a stated contract with no C on the path. The
`experiments/lib/machine-bridge.c` build-and-drive stays.

**Do not delete.** `TODO/podssh.md:126` names it in T-1404's `Prove`, and the
entry is `done` on the strength of it.

## Group 9 - experiments/394-ssh-package.sh

**What it measures.** Whether helper packaging rejects a missing helper and
invalid ELF bytes. It packages the four SSH helpers, then removes `node` and
asserts `missing executable: node` with exit 1, then writes `invalid ELF
fixture` over `node` and asserts `invalid ELF: node` with exit 1, then reads
the tar member list and asserts all four names (`394:17-33`).

**Pinned inputs.** `target/x86_64-unknown-linux-musl/release/{node,operator,proxy,shell}`
(`394:8-9`).

**Host assumptions.** `scripts/package-ssh.sh` present (`394:7`, exit 2).
The four release binaries present and executable (`394:9, 17`, exit 2).
`mktemp -d` (`394:10`, exit 2).

**Exit contract.** 0 matched, 1 failed, 2 could not run (`394:4`).

**Timeouts.** None. Every `package-ssh.sh` call is unbounded, and that script
bounds its own per-helper `--help` at `timeout 15` but not the readelf and tar
calls.

**Cleanup.** `trap 'rm -rf "${WORK:?}"' EXIT HUP INT TERM` (`394:11`).
Correct and idiomatic.

**Positive control.** The first `package-ssh.sh` call at `394:18` must exit 0
before either refusal is tested. It is the control.

**Failure control.** Both negative arms are present and named: the missing
helper and the invalid ELF. This is the best-posed script in the group.

**Owning entries.** None by that name. `TODO/podssh.md:156` gives T-1405 the
Prove `sh scripts/package-ssh.sh RELEASE_DIR ARCH QEMU exits 0`, and
`experiments/README.md:37` names `394-ssh-package.sh` as a required audit proof
with "A native static release build; complete archive and named missing-helper
and ELF refusals". No `TODO/` entry names the experiment file.

**Result files.** None specific. `TODO/podssh.md:168-171` points the native
packaging proof at `experiments/results/repo-audit-linux.txt`.

**Verdict: RUST-TOOL.**

### Conversion plan - 394

**Assertions today.**

- A: precondition, four release helpers in a directory; action, run
  `package-ssh.sh DIR ARCH`; expected, exit 0.
- B: precondition, the directory with `node` removed; action, run it; expected,
  exit 1 and stderr contains `missing executable: node`.
- C: precondition, the directory with `node` replaced by
  `invalid ELF fixture`; action, run it; expected, exit 1 and stderr contains
  `invalid ELF: node`.
- D: precondition, the archive exists; action, read
  `dist/podbox-ssh-x86_64.tar.gz`; expected, members `./node`, `./operator`,
  `./proxy`, `./shell`.

**The tool it measures already exists and is not Rust.**
`scripts/package-ssh.sh` performs the packaging, the ELF check, the usage
contract check, the licence stamping, the reproducible tar, and the checksum.
The experiment is its test. Converting the experiment means converting the
tool, because the tool is the durable workflow and the experiment is only its
proof.

**Crate.** `crates/podbox-ssh` already owns all four binaries as `[[bin]]`
targets. The packaging step needs no new crate; it becomes a binary in the
crate that owns the artifacts, so the artifact set and the packager cannot
drift. New binary name `package-ssh` in `crates/podbox-ssh/Cargo.toml`.

**CLI surface.**

```text
package-ssh <release-dir> <arch> [qemu]
```

Matching the existing `scripts/package-ssh.sh` usage at its line 3, because
`TODO/podssh.md:156` states the Prove against that spelling. Flags: none in
the first shape. `--output DIR` and `--json` may follow, but the first version
needs no flag because `docs/conventions/code.md` says "Do not add unused
frameworks".

**Behaviours to port, in order.**

1. arch allowlist (`x86_64|aarch64|riscv64gc|loongarch64|armv7|i686|powerpc64le`),
   exit 2 otherwise.
2. For each of `node operator proxy shell`: the file must be executable, else
   `SSH-PACKAGE-FAIL missing executable: NAME` and exit 1.
3. The ELF header must parse. **Replace the `readelf` subprocess with
   `podbox_enter::abi::Elf::read`** (`abi.rs:156`). That is the tree's own
   reader and it is what `podbox system abi` uses, so the packager and the
   runtime agree by construction. This is the whole reason the conversion is
   worth doing.
4. A `PT_INTERP` program header is refused with
   `SSH-PACKAGE-FAIL dynamic interpreter: NAME`.
5. The usage contract: run the helper with `--help` under a 15 s bound and
   with `PODSSH_RELAY`, `PODSSH_NAME`, `PODSSH_NODE_TOKEN` and
   `PODSSH_CONNECT_TOKEN` unset; expect exit 125 and `usage: NAME`.
6. Copy `LICENSE` and `crates/podbox-ssh/shims/LICENSE` into the archive.
7. The dependency licences, currently `scripts/release-licenses.py`. This is a
   group-2 script and its own conversion is not mine to plan. Until it lands,
   call it as a subprocess and say so in the code, rather than duplicating it.
8. The reproducible tar: `SOURCE_DATE_EPOCH` or `git log -1 --format=%ct`,
   `--sort=name --mtime --owner=0 --group=0 --numeric-owner`, then `gzip -n`.
9. Write `dist/podbox-ssh-ARCH.tar.gz` and its `.sha256`.
10. Print `SSH-PACKAGE-OK ARCH: ...`.

**Level.** Unit for 1, 2, 3 and 4: they are pure over a directory and a byte
slice. Target `crates/podbox-ssh/tests/package.rs`, a **new** integration test
file, because `package` is a binary and its logic is not reachable from the
library. `crates/podbox-ssh/tests/` exists. Integration for 5 through 10,
because they spawn the real helpers and write a real archive.

**Fixtures.** A temp directory holding files whose bytes are written by the
test, not copied from `target/`. That makes every arm deterministic. A helper
with no `PT_INTERP` is a real static binary: the test can use
`std::env::current_exe()` on a statically linked test binary, or build a
one-line C file with the lane `cc`, which is what the current script depends on
anyway. A file whose bytes are `invalid ELF fixture` is the negative.

**Plant.** Remove the `PT_INTERP` check, run
`cargo test -p podbox-ssh --test package`, and read the failure: a test that
preloads a `PT_INTERP` header into a fixture helper must be refused, and the
test must name `dynamic interpreter`. Without the plant, deleting the check
turns a refusal into an archive that ships a dynamic binary, which is the
defect the check exists to catch.

**Proof command.**

```sh
cargo test -p podbox-ssh --test package
cargo build -p podbox-ssh --release --target x86_64-unknown-linux-musl
```

The second command is the precondition for the drive half, matching
`394:8-9`.

**`exit 2` becomes.** The three `exit 2` sites become distinct causes in the
error message. A missing directory or an unknown arch is a usage error, which
`crates/podbox-cli`'s convention calls the cli-error code (1), but this is not
`podbox` and has no table to read. Keep 2 for "no such directory" and "unknown
arch", and say which. The test asserts the message, not the code, for this
reason.

**Result file.** None today. The implementor writes
`experiments/results/ssh-package.txt` carrying the refusal arms, because
`experiments/README.md:37` requires the proof and the negative result belongs
under `experiments/results/` per `docs/methodology/experiments.md` "Save
negative results with the same conditions as successful results".

## Group 9 - experiments/397-exported-build.py

**What it measures.** Whether the five exported executables match the digests
in the build record beside them (`397:24-35`). It reads
`build-state.json`, checks the schema is `podbox-build/1` and the output set is
exactly `{podbox, node, operator, proxy, shell}` (`397:25-27`), then compares
each file's sha256 with the record (`397:29-35`).

**Pinned inputs.** One directory argument (`397:16`) holding the five
executables and the record.

**Host assumptions.** Python 3.13 or later in the saved run
(`397:19`, `host=Windows python=3.13.15`). No network.

**Exit contract.** 0 all matched, 1 a mismatch or a schema error, 2 an
`OSError` (`397:36-41`). The third state is handled properly and named
(`397:40`, "cannot run: ").

**Timeouts.** None needed; it reads five local files.

**Cleanup.** None needed; it writes only the `--output` path it was given.

**Positive control.** None. Every assertion is "a digest matches".

**Failure control.** None. This is the script's real weakness: a check with
only positive assertions and no mutation has never been shown to go red.
`experiments/README.md:40` calls it out: "all five executable byte digests
agree with the saved build record" is the whole contract.

**Owning entries.** None by name. `TODO/gate.md:1793` cites
`experiments/results/exported-build.txt` as evidence for T-1350. T-1350 is
`partial` (`TODO/gate.md:1781`).

**Result files.** `experiments/results/exported-build.txt`.

**Verdict: RUST-TOOL.**

### Conversion plan - 397

**Assertions today.**

- A: precondition, a directory with `build-state.json` and five executables;
  action, read the record; expected, `schema` is `podbox-build/1` and
  `outputs` is exactly the five names.
- B: precondition, a valid record; action, sha256 each executable; expected,
  each digest equals `outputs[NAME]`.
- C: precondition, a missing file; action, read it; expected, exit 2, not 1.
- D: precondition, a mismatched digest; action, compare; expected, exit 1.

**Why a tool and not a test.** It runs inside the release job, over artefacts
that no test produced in that run, and it must print a report the operator
reads. A `#[test]` cannot run inside `scripts/windows/run-in-base.sh` after the
copy. `docs/methodology/experiments.md` requires the conditions, the inputs and
the scope printed; the Rust version keeps that and loses the Python interpreter
dependency, which `TODO/gate.md:1764` already complains about with
`python3 experiments/393-build-freshness.py`.

**Crate.** `crates/podbox-cli` already owns `system` and `system info`
(`crates/podbox-cli/src/system.rs`). A new `[[bin]]` target
`verify-export` in `crates/podbox-cli/Cargo.toml` is the right home: the
verifier reads the same artefact set the CLI ships.

**CLI surface.**

```text
verify-export <directory> [--output FILE] [--json]
```

`--output` matches the current `--output` (`397:16`). `--json` is new and is
warranted: `TODO/gate.md:1799` says the operator reads "machine JSON" and
`check-gate.sh` already emits it.

**Behaviours to port, in order.**

1. Print the conditions block exactly: UTC instant, `platform.system()`, the
   interpreter's version in Rust this is `env!("CARGO_PKG_VERSION")` plus
   `std::env::consts::OS`, the inputs, and the scope line. The Python version's
   `python=3.13.15` line has no Rust equivalent and is dropped; the scope line
   "this does not compare source or toolchain conditions" is kept because it is
   the sentence that keeps the result honest.
2. Parse `build-state.json`. The crate already has `crates/podbox-probe/src/json.rs`;
   reuse it rather than adding a parser, per `docs/conventions/code.md` "Use
   one parser".
3. Reject a record whose `schema` is not `podbox-build/1` or whose `outputs`
   key set differs from the five names.
4. For each name in sorted order, read the file, take sha256, print
   `NAME: bytes=N sha256=... matched=...`.
5. Write the report and print it.

**Level.** Unit for 3: a record-parsing function over a JSON value, tested with
a fixture that has the wrong schema, a missing name and an extra name.
Integration for 4 and 5: a temp directory with five files of known bytes and a
record carrying their digests, which is deterministic.

**Target.** `crates/podbox-cli/tests/verify_export.rs`, a **new** integration
test file, because the logic lives in a binary target and not in the library.
`crates/podbox-cli` has no `tests/` directory today, so this creates the first
one; that is a structural change worth noting to the operator.

**Fixtures.** Built in the test: write five files from a `&[u8]` literal, build
the record JSON from the same bytes with a sha256 helper. Nothing comes from
`experiments/lib/`. A real release artefact is not needed, and using one would
make the test depend on a build that may not have run, which is what
`TODO/methodology` calls an unpinned input.

**Plant.** Change the comparison at the tool's digest step to compare a file
against itself, run `cargo test -p podbox-cli --test verify_export`, and read
the failure: the mismatch test must go red, and a check that compares a value
with itself is the exact defect the plant exists to catch. The current Python
script has no such test at all, which is why this plant is new work rather
than a port.

**Proof command.** `cargo test -p podbox-cli --test verify_export`. On Windows
the release job runs the built binary:
`.\target\release\verify-export.exe C:\path\to\export --output FILE`. No
feature flag. No target triple: the verifier is not shipped in the musl archive
and the five executables it reads are, so it can be a host build.

**`exit 2` becomes.** `OSError` becomes `std::io::Error`. The third state
survives, because a missing file is genuinely not a mismatch: return 2 and say
`cannot run: {path}: {os error}`. `docs/methodology/experiments.md` "A missing
observation is not a denial or a zero" is satisfied.

**Result file.** Keep `experiments/results/exported-build.txt`. It is the
evidence `TODO/gate.md:1793` cites, and its five digests are a real record of a
real export. The tool's report format should be able to reproduce that file
byte for byte, and the implementor checks that it does, because the operator
may diff them.

## Group 9 - experiments/80-interposer-abi.sh

**What it measures.** Which interposer object may be loaded into a given
payload, decided from ELF metadata alone. Five checks. A: the SONAME
discriminator, taken with no run (`80:186-211`). B: four cross-libc arms with
both controls (`80:214-289`). C: the version predicate, predicted from ELF
then confirmed with the loader (`80:292-332`). D: the `.dynsym` trap
(`80:334-352`). E: `podbox system abi` measured against the loader's own answers
(`80:354-438`).

**Pinned inputs.** Three images by digest (`80:66-72`) and
`references/VHSgunzo__pathmap/tree/path-mapping.c` (`80:65`).

**Host assumptions.** `readelf` (`80:111`, exit 2), a C compiler, either `cc`
or `zig` (`80:81-94`, exit 2 at `80:112`), an engine (`80:215-216`, exit 2).
A musl compiler or zig (`80:148-155`); without one the script exits 2 at
`80:450` even when every other check passed.

**Exit contract.** 0 all checks that ran matched, 1 a check ran and did not
match, 2 a check could not run here (`80:46-47`). The script separates `rc`
from `could_not` (`80:118-121`) so a skip never reads as a pass, which is more
careful than any other script in this group.

**Timeouts.** `eng_run 180` for the arms, `eng_run 60` for the reader.

**Cleanup.** `trap eng_cleanup EXIT INT TERM` (`80:63`); `eng_clear` after
every arm. `OUT` is `experiments/.abi`, removed at the start (`80:53`).

**Positive control.** B's controls come first and are labelled so:
"control glibc object -> glibc" and "control musl object -> musl" load
(`80:281, 284`). E's control is first for the same reason (`80:412-414`). This
is the discipline `docs/methodology/experiments.md` asks for by name.

**Failure control.** The `.dynsym` trap at check D is a failure control on the
reader itself: "A reader that asks `.symtab` finds no symbols at all and
concludes the payload's libc defines nothing, which refuses every artefact and
reads exactly like a check that works" (`80:24-27`).

**Owning entries.** T-0702 (`TODO/interpose.md:266`), T-0709
(`TODO/interpose.md:852, 857`), T-0701 (`TODO/interpose.md:118-123`), T-1210
(`TODO/gate.md:797, 827, 840`), and `TODO/milestones.md:263` for the
numbering rule. Read in full.

**Result files.** `experiments/results/interposer-abi.txt` and
`experiments/results/interpose-abi.txt`. Two files, and
`TODO/interpose.md:806` cites the second while `TODO/interpose.md:129` cites
the first. See the findings.

**Verdict: SPLIT.**

### Conversion plan - 80

**Assertions today.**

- A: precondition, a glibc object and a musl object exist; action, read
  `DT_NEEDED` from each; expected, the glibc object records `libc.so.6` and the
  musl object records `libc.so`.
- B: precondition, both objects and a loader; action, `LD_PRELOAD` each into a
  matching and a mismatching libc; expected, the matching pair loads and the
  mismatched pair is refused.
- C: precondition, an object and a payload libc; action, compare the highest
  `GLIBC_x.y` each names; expected, the prediction matches what the loader
  then does, and the refusal names the same version.
- D: precondition, a shipped `libc.so.6`; action, count defined symbols in
  `.symtab` and `.dynsym`; expected, `.symtab` has none and `.dynsym` has many.
- E: precondition, the same situations; action, run `podbox system abi`;
  expected, its verdict is the loader's, and on C's situation it names the same
  version.

**Level and target, check by check. The conversion is already done for three
of the five.**

- **A is unit, and exists.** `crates/podbox-enter/src/abi.rs:1170`,
  `the_soname_discriminator_is_the_one_that_was_measured`, with the doc comment
  "Check A of `experiments/results/interposer-abi.txt`, as a predicate"
  (`abi.rs:1168`). It drives `Flavour::of_libc_name` and `Flavour::of_needed`
  (`abi.rs:66, 80`) over the shapes. Nothing to write.
- **C is unit, and exists.** `abi.rs:1250`,
  `an_import_at_a_version_the_libc_does_not_declare_is_refused`, doc comment
  "Check C, as the predicate rather than as a run" (`abi.rs:1246`). It builds a
  fake libc declaring `GLIBC_2.31` and a fake object importing `GLIBC_2.34` and
  asserts `admits` refuses naming both versions. The `fake` and `fake_import`
  helpers are the fixture.
- **D is unit, and exists.** `abi.rs:1220`,
  `a_shipped_libc_defines_its_symbols_in_dynsym`, doc comment "**The
  `.symtab` trap, as an assertion rather than a comment**" (`abi.rs:1212`).
  Its skip is the honest third state at `abi.rs:1228-1231`.
- **E is unit, and exists in two places.** `abi.rs:1274`
  `a_glibc_object_against_a_musl_rootfs_is_refused_in_those_words` covers E's
  glibc-object-into-musl arm, and `abi.rs:1293`
  `a_matching_object_and_libc_are_admitted` is the control E would otherwise
  not have. The CLI wrapper's three-way exit is at
  `crates/podbox-cli/src/system.rs:157` and its tests are in
  `crates/podbox-cli/src/system.rs` (7 `#[test]` functions). The implementor
  adds one test that the `abi` wrapper returns 2 for an unreadable path, since
  that third state is the one a Rust test can prove cheaply and the shell
  asserted it only through check E's `*:2` arm at `80:405`.
- **B is integration and stays a drive.** It is a real loader refusing a real
  object across a real libc boundary. `docs/conventions/code.md` forbids a
  mock here. B is the one check whose premise is *that podbox does not need to
  run anything*, so replacing it with a unit test would destroy the finding.

**Target.** All unit work lands in
`crates/podbox-enter/src/abi.rs`, inside `mod tests` at line 1075, and in
`crates/podbox-cli/src/system.rs`. Both inline. The `fake` and `fake_import`
helpers at the end of `abi.rs` are the fixture, and `tiny_elf` at
`abi.rs:1370` covers the minimal-header case.

**Plant.** For A, change `Flavour::of_needed` at `abi.rs:66` to match `libc.so`
by substring, which is the exact mistake the entry warns about at
`TODO/interpose.md:100-101` ("a substring test would call a glibc object
musl-linked, which is the very mistake being caught"). Run
`cargo test -p podbox-enter abi::tests`, and read the failure: the test must
name the shape it was given. Restore. For D, point the reader at `.symtab`, and
the dynsym test must go red with a message about the count.

**Proof command.**

```sh
cargo test -p podbox-enter abi
cargo test -p podbox-cli system
```

No flags. No target triple: `this_binarys_own_header_parses` at `abi.rs:1199`
reads `std::env::current_exe()`, which is the test binary itself and is present
whatever the host.

**`exit 2` becomes.** The `readelf` gate, the compiler gate and the engine gate
are all drive states. The unit tests have no third state at all, which is the
gain: `admis` returns a `Verdict`, and `Flavour::of_libc_name` returns
`Unknown` for anything it does not recognise (`abi.rs:1186`). The script's
`could_not` counter at `80:118` has no counterpart and needs none, because a
value that cannot be decided is a value in the type.

**Result files.** Two files exist and the entries disagree about which is
current. `interpose-abi.txt` is named by `TODO/interpose.md:129` and
`TODO/interpose.md:806`; `interposer-abi.txt` is named by the doc comments in
`abi.rs:1168` and `abi.rs:1214`. See the findings. The implementor resolves
this before deleting either script, because an evidence file cited by a live
test's doc comment cannot be moved without updating that comment.

## Group 9 - experiments/90-nsswitch-contract.sh

**What it measures.** Whether a supplied `/etc/passwd` is read, and under what
condition. Four checks. A: a supplied passwd against two nsswitch settings,
with a user no distribution ships (`90:100-121`). B: what three pinned images
actually ship (`90:123-144`). C: what a `-static` glibc binary opens after its
own execve, counted with strace (`90:149-171`). D: glibc's gconv modules and
their `DT_NEEDED` (`90:174-191`).

**Pinned inputs.** Three images by digest (`90:40-42`).
`experiments/src/nsswitch-q.c` compiled `-static` (`90:57, 86`).

**Host assumptions.** `cc` or a guest-built probe at `Q_STATIC` (`90:58-62`,
exit 2 at `90:73-75`). An engine (`90:76-77`, exit 2). `strace` for check C
only (`90:151`); without it the script prints COULD NOT RUN and continues
(`90:168-171`). A gconv directory for D only (`90:176`); without it, the same.

**Exit contract.** 0 all checks that ran matched, 1 a check ran and did not
match, 2 could not run (`90:31-32`).

**Timeouts.** `eng_run 180` per clause.

**Cleanup.** `OUT` is `experiments/.nsswitch`, removed at the start (`90:38`).
`eng_clear` after each clause.

**Positive control.** Check A's `files` arm is the control, and the script says
so: the `other` arm failing is meaningless unless `files` succeeded
(`90:116-117` names that as "the control is broken").

**Failure control.** None injected. A is a natural experiment: two nsswitch
settings, opposite results.

**Owning entries.** T-0410 (`TODO/complete.md:636, 671`), T-1211
(`TODO/gate.md:890, 906, 917`), and `TODO/image.md:256`. Read in full.

**Result files.** `experiments/results/nsswitch-contract.txt`.

**Verdict: SPLIT.**

### Conversion plan - 90

**Assertions today.**

- A: precondition, a static glibc probe and a supplied passwd naming
  `podboxsupplied`; action, run it under `passwd: files` and under
  `passwd: systemd`; expected, `FOUND` and `NOTFOUND`.
- B: precondition, three pinned images; action, read
  `/etc/nsswitch.conf`; expected, ubuntu and debian name `files`, alpine ships
  none.
- C: precondition, a `-static` probe; action, strace it; expected, the
  `.INTERP` count is 0 and the count of `.so` opens after the last `execve` is
  reported.
- D: precondition, a gconv directory; action, read each module's `DT_NEEDED`;
  expected, all record `libc.so.6`.

**Level.**

- **Check A becomes a Rust unit test, and the entry says so already.**
  `TODO/complete.md:659` names
  `every_measured_passwd_shape_ends_up_naming_files_first` as the replacement
  measurement, driven by `experiments/125-across-distributions.sh`'s six
  measured shapes. Target:
  `crates/podbox-complete/src/identity.rs`, inside its existing `mod tests`
  (8 `#[test]` functions today). `docs/conventions/code.md`: "Use pure tests
  for deterministic logic." The `nsswitch` function's four cases at
  `TODO/complete.md:641-646` are a `match` over a rootfs directory, which is a
  temp directory and nothing more.
- **Check B is a live reading of three upstream images** and stays a drive. No
  crate can assert what `alpine:3.22` ships.
- **Check C is a measurement of a property, and its recorded result is empty.**
  `nsswitch-contract.txt:46-48` reads `static probe PT_INTERP headers: 0`,
  `objects opened after the last execve: 0`, `ld.so.cache lines seen: 0`. The
  count is zero, so check C asserts nothing beyond "the probe is static". The
  lane was Cygwin (`nsswitch-contract.txt:4`, `host libc ldd (cygwin) 3.6.9`)
  and the probe was guest-built (`nsswitch-contract.txt:5`). **This check
  proved nothing on the lane that recorded it.** The implementor deletes check
  C and records why, rather than porting a check whose own output is empty.
- **Check D is a pure ELF reading over a host directory.** Target:
  `crates/podbox-enter/src/abi.rs`, beside the other `DT_NEEDED` tests. Build
  the assertion over the tree's own glibc where present, and skip honestly
  where absent, following the pattern at `abi.rs:1228-1231`. The claim
  ("bundling gconv into a musl artefact reintroduces a second libc") is a
  design rationale already recorded at `TODO/complete.md`; the test pins the
  fact behind it.

**Target files.** `crates/podbox-complete/src/identity.rs` (check A),
`crates/podbox-enter/src/abi.rs` (check D). Both inline `mod tests`.

**Fixtures.** A temp rootfs directory with `etc/nsswitch.conf` in each of the
measured shapes. The six measured strings are in the entry at
`TODO/complete.md:659` and come from `125-across-distributions.sh`, which is
group 3's. The implementor copies the six strings, not the script.

**Plant.** Change the `files` prepend in `identity.rs` to an append, run
`cargo test -p podbox-complete identity`, and read the failure: the test must
name `files` as not-first. This is the exact defect the entry calls out at
`TODO/complete.md:648-651` ("`passwd: sss files` asks the directory service
before the file podbox supplied"), and a test that asserted membership rather
than order would pass.

**Proof command.**

```sh
cargo test -p podbox-complete identity
cargo test -p podbox-enter abi
```

No flags, no triple.

**`exit 2` becomes.** The `cc` and `Q_STATIC` gates are drive states and
disappear for the unit test. The engine gate is a drive state. Check A's third
state is real and survives: a rootfs with no `nsswitch.conf` and no glibc is
case 1, musl, and the function returns without writing. That is a value, not a
code.

**Result file.** Keep `experiments/results/nsswitch-contract.txt`, and
annotate it. `TODO/complete.md:599` cites check A from it and check A is sound.
`TODO/complete.md:662-666` already records the correction about
`alpine:3.20` shipping an nsswitch file that `alpine:3.22` does not, so the
file's own claim about alpine is known to be version-specific and the entry
says so. The empty check C stays in the file as the record of what was
measured; the implementor does not delete lines from a historical result.

## Group 9 - scripts/session-start.sh

**What it measures.** Nothing. It is the session entry command. It answers
four questions and starts the environment: when, where, the tree, the tools,
codegraph, the lane, and what to read (`session-start.sh:5-7`).

**Pinned inputs.** None.

**Host assumptions.** Everything. `uname`, `date`, `git`, thirteen tools
(`session-start.sh:146`), `codegraph`, and then one of three lane setups
(`session-start.sh:182-215`). It resolves its own root from `$0`
(`session-start.sh:21-22`) and refuses a tree without `AGENTS.md` or `TODO/`
(`session-start.sh:88-91`, exit 2).

**Exit contract.** 0 the lane is ready, 1 something needs attention and is
named, 2 it could not run (`session-start.sh:18`). The three-way split is
honoured: the `problems` counter at line 45 drives exit 1 at line 228, and
`exit 2` is separate.

**Timeouts.** None. `scripts/dev.sh` is started on the native lane
(`session-start.sh:190`), and `wsl-toolkit ... base ensure --probe` is
foreground on the Windows lane (`session-start.sh:205`).

**Cleanup.** Delegated. `dev.sh` detaches and manages its own state under
`.dev/`.

**Positive control.** None needed; it is a report.

**Failure control.** None. Nothing here asserts.

**Owning entries.** None by name. The script is named in `AGENTS.md:12`,
`README.md:30,36`, `HUMAN.md:13,24`, `scripts/README.md:3,7`,
`docs/containers.md:21`, `docs/code-map.md:22`, `docs/agent-tooling.md:23`,
`docs/hosted-sessions.md:3`, and `docs/history/sessions/2026-09-11-orientation.md:23`.
**No `TODO/` entry names it.** `TODO/packaging.md:218` (T-1005) is the entry
that owns `dev.sh` and the startup measurement, and it names neither.

**Result files.** None. It writes no result file.

**Verdict: RUST-TOOL.**

### Conversion plan - session-start

**What it asserts today.** Nothing. It reports. That is the whole design, and
`docs/methodology/experiments.md` does not apply to it: it is a tool, not a
measurement. The operator's framing calls these "nasty shell scripts"; this one
is a session entry point, and retiring it as a *measurement* is a category
error. It retires as a *tool* or it stays.

**Crate.** A new crate `crates/podbox-dev`, because this is a development and
operator workflow with no runtime role and no dependency on any podbox crate.
Putting it in `podbox-cli` would ship a session reporter inside the product
binary, which T-1003's own reasoning about artefacts argues against.

**Binary.** `podbox-dev`, one `[[bin]]` target in `crates/podbox-dev/Cargo.toml`.
The multicall shape T-0803 already uses (`TODO/cli.md:290`) applies: the
binary takes its subcommand as `argv[1]`, so `podbox-dev session-start` is the
new spelling and the bare binary is usage.

**CLI surface.**

```text
podbox-dev session-start [--check] [--quiet] [-h|--help]
```

The three flags are the script's own (`session-start.sh:27-40`). `--help` is
the `sed -n '2,18p' "$0"` at line 32, which is a **defect** to fix in the
conversion: it prints a line range of the shell file. A Rust binary prints its
own usage text, and `crates/podbox-cli/src/man.rs` already has the house
pattern for it.

**Behaviours to port, in order.**

1. Resolve the repository root. In Rust this is `env!("CARGO_MANIFEST_DIR")`
   at build time, not `CDPATH= cd -- "$(dirname -- "$0")" && pwd` at run time.
   The two answer differently when the binary is on PATH, which is the case
   that matters.
2. Refuse a tree with no `AGENTS.md` or no `TODO/`, exit 2. Same message.
3. `== when`: UTC and local, from `SystemTime` and a UTC offset. The script's
   `NOW=$(date -u ...)` and `|| NOW="-"` pattern becomes an explicit
   `Option<SystemTime>` rendered as `-` when unavailable.
4. `== where`: `uname -s`, `-r`, `-m` become `std::env::consts::{OS, ARCH}` and
   a `uname -r` subprocess or a `-` dash. The `KIND` case at lines 66-79
   becomes a match: `linux`, `container` (test `/.dockerenv` and read
   `/proc/1/cgroup` for the four markers), `wsl-guest` (grep `microsoft` in
   `/proc/sys/kernel/osrelease`), `windows`, `darwin`, `unknown`.
5. `== the tree`: branch, head, dirty count, identity, and the three
   `note_problem` branches. The dirty count is the script's own trap, at
   lines 100-102: "grep -c exits 1 when it counts zero, which breaks an && chain".
   In Rust the count is a `usize` and the trap disappears, which is the clearest
   example in this group of what the conversion buys.
6. `== tools`: run each of the thirteen, not find it. The script is emphatic
   at lines 130-132: "A NAME ON PATH IS NOT A WORKING PROGRAM. python3 on
   Windows resolves to a Store stub that exits without running anything, so
   each version is read by RUNNING the tool". In Rust, spawn and read the
   first line, with the four special spellings at lines 137-143 preserved:
   `python3 -c import sys; print(sys.version.split()[0])`, `zig version`,
   `go version`, `wsl-toolkit version`.
7. `== codegraph`: absent, `--check`, `init .`, or `sync .`.
8. `== the lane`: `native` for linux, container and wsl-guest; `wsl-toolkit`
   for windows; then either `scripts/common/bootstrap-env.sh --check` plus a
   background `scripts/dev.sh`, or `wsl-toolkit --instance podbox base status
   --probe` and `base ensure --probe`. **These are the host commands the tool
   exists to call.** Keep them as subprocesses. The prohibition is in
   `docs/containers.md` and is about `wsl.exe`, which Rust does not change.
9. `== read these, in this order`: four lines, unchanged.
10. Exit 0, or 1 when `problems > 0`, or 2 from step 2.

**Level.** Integration, because every step 5 through 8 touches the real host.
Target `crates/podbox-dev/tests/session_start.rs`, a new integration test.
**The valuable assertions are the ones that are pure:** the `KIND` match over
five synthetic `/proc` fixtures, the lane derivation, the three
`note_problem` branches, and the argument parsing. Those are unit tests in
`crates/podbox-dev/src/lib.rs` with the `KIND` function taking its inputs as
parameters rather than reading `/proc` itself, which is the design change that
makes them testable at all.

**Fixtures.** Fake rootfs directories containing `AGENTS.md`, `TODO/`, and
synthetic `/proc/1/cgroup` and `/proc/sys/kernel/osrelease` content, passed in
as paths. No fixture from `experiments/lib/`. A real `git` repository is not
needed; the git steps accept a runner that is a trait or a closure.

**Plant.** Change the `KIND` match so a container is reported as `linux`, run
`cargo test -p podbox-dev`, and read the failure: the test must name
`container`. Without the plant, folding container and wsl-guest into `linux`
is invisible, because on a plain Linux CI host all three arms agree.

**Proof command.** `cargo test -p podbox-dev`. No flags, no triple.

**`exit 2` becomes.** Unchanged. A tool that runs on the host has the same
three states. The distinction from an experiment is that this tool's exit 1
means "named problem", not "assertion failed", and the conversion must not
conflate them.

**Result file.** None today, and none needed. The implementor adds
`experiments/results/session-start.txt` only if the operator wants a lane
record; the script has never written one, and inventing one is not an
improvement.

**Precondition the operator must settle.** `AGENTS.md:12` and `README.md:30`
both tell every session to run `sh scripts/session-start.sh` first. Converting
the tool while the documentation still names the shell spelling produces a
tree where the documented command does not exist. The implementor changes
`AGENTS.md`, `README.md`, `HUMAN.md`, `scripts/README.md`, `docs/containers.md`,
`docs/code-map.md`, `docs/agent-tooling.md` and `docs/hosted-sessions.md` in the
same change, or ships a `scripts/session-start.sh` that execs the new binary
during the transition and deletes it later. **The shim is the smaller change
and I recommend it**, because nine files name the script and a session that
follows `AGENTS.md` on the day of the change must still work.

## Group 9 findings

### Contradictions between a script and the current source

1. **`experiments/149-podvm-non-goals.sh:185` writes `361 guest networking
   green`; `experiments/results/podvm-non-goals.txt` records `ok: 360 guest
   networking green`.** Git shows the result committed at `84d16c3` and the
   script edited afterwards at `ea6d321`. The saved evidence is stale against
   the current script. `TODO/podvm.md:622` cites the file and reports the run
   that the stale text describes.

2. **`experiments/90-nsswitch-contract.sh` check C asserted nothing on the lane
   that recorded it.** `nsswitch-contract.txt:47` reads `objects opened after
   the last execve: 0` and line 48 reads `ld.so.cache lines seen: 0`, and
   lines 3-5 record the host as Cygwin with a guest-built probe. The check
   prints `ok` at line 164 with no assertion on the count. `TODO/gate.md:906`
   records the script exiting 0, which is true and proves less than it appears.

3. **`experiments/354-lifecycle-same-store.sh` and
   `experiments/359-supervision-split.sh` both leave their scratch behind.**
   Both remove `experiments/.sweep*-work` at the start (line 23 and line 32
   respectively) and neither installs a trap. `docs/methodology/experiments.md`
   says "Remove owned scratch only after its result is saved"; neither removes
   it after. `363-device-map.sh:44` has the same shape and additionally leaves
   containers `d363` and `d363e` behind when a clause fails between create and
   rm, since the `rm` calls at lines 264 and 287 are on the success path only.

4. **`experiments/330-exit-codes.sh` leaves every `podbox` call unbounded**
   except through the `timeout` wrapper in `pb()` at line 97, which is correct,
   and `experiments/159-interpose-placement.sh` has no bound at all on any of
   its eleven `podbox` invocations. `docs/conventions/code.md` line 16 requires
   bounded child waits. This is the one Group 9 script where the conversion
   strictly improves a stated rule.

5. **`experiments/results/` carries two abi result files and the entries
   disagree about which is current.** `TODO/interpose.md:129` and
   `TODO/interpose.md:806` cite `experiments/results/interposer-abi.txt`. The
   doc comments of the live unit tests cite the other: `abi.rs:1168` names
   `experiments/results/interposer-abi.txt` and `abi.rs:1214` names
   `experiments/results/interposer-abi.txt`. Both files exist. One of the two
   citation sets is wrong, and a reviewer cannot tell which from the tree.

### Dead or superseded scripts

6. **`experiments/354-lifecycle-same-store.sh` has no owning entry.** It is
   cited in three `docs/history/` files and in no `TODO/` file. Its subject,
   the lifecycle loop, is T-0607 and is run twenty times by
   `230-lifecycle-loop.sh`, which `359:139` already calls. This is the clearest
   DELETE in the group.

7. **`experiments/159-interpose-placement.sh` and
   `experiments/80-interposer-abi.sh` are half-superseded by tests that already
   exist.** `crates/podbox-enter/src/abi.rs` carries A, C, D and E of the second;
   `crates/podbox-ssh/tests/session_interactive.rs` carries all eleven clauses
   of `388`. The scripts are not dead, but their remaining value is the lane
   record in `experiments/results/`, not the clauses.

8. **`experiments/391-machine-bridge.sh` duplicates a build with
   `experiments/390-machine-ssh.sh`.** Both compile
   `experiments/lib/machine-bridge.c` (391:57 and 390:210). The duplication is
   load-bearing in the sense that each proves its own layer, and it is listed as
   a KEEP-SHELL for that reason, but a reader should know the C is built twice.

### Scripts with no owning entry

9. **`experiments/394-ssh-package.sh`** is named in `experiments/README.md:37`
   as a required audit proof and in no `TODO/` entry. `TODO/podssh.md:156`
   gives T-1405 a `Prove` against `scripts/package-ssh.sh` instead, so the
   proof exists and the experiment does not own it.

10. **`experiments/397-exported-build.py`** is named in
    `experiments/README.md:40` as a required audit proof and in no `TODO/`
    entry. `TODO/gate.md:1793` cites its result file as evidence for T-1350
    without naming the script.

11. **`scripts/session-start.sh`** is named in nine documentation files and no
    `TODO/` entry. `TODO/packaging.md` T-1005 owns the startup measurement
    (`experiments/310-session-startup.sh`, group 10) and `dev.sh` (group 7),
    but not this.

12. **`experiments/354-lifecycle-same-store.sh`**, as above.

### Entries whose acceptance command is a script with no automated test

13. **T-0410 (`TODO/complete.md:636`)**: `Prove: ./experiments/90-nsswitch-contract.sh
    exits 0 and podbox run --rm ... sh -c 'id podboxsupplied'`. The unit test
    the entry names as the measurement (`TODO/complete.md:659`) tests the
    *editor*, not that a payload resolves the supplied user. The second half of
    the Prove is the live run and has no automated form.

14. **T-0606 (`TODO/supervise.md:476`)**: the acceptance is
    `359-supervision-split.sh`, whose denied-notify shape its own header says
    no lane can show (`359:19-21`). The banner-line-off case therefore has no
    lane proof. The entry says it is unit-pinned, which is true, and the unit
    is the only proof.

15. **T-1402 (`TODO/podssh.md:79`)**: the Prove names both
    `cargo test -p podbox-ssh` and `sh experiments/388-interactive-shell.sh`.
    The first covers every clause and the second duplicates it. No gap, but the
    entry has not been updated to say so.

16. **T-0802 (`TODO/cli.md:222`)**: the Prove is a bare command pair, and the
    Done note at line 270 replaces it with the script. The entry's `Prove` line
    and its `Done` proof name different things, which `docs/methodology/authoring.md`
    "Keep the ten entry fields" does not forbid but does not encourage.

## Group 9 - entries read

Read in full, for the audit:

1. T-0802, `TODO/cli.md:198-273`, "docker's exit codes, unaltered". Owns 330.
2. T-1414, `TODO/cli.md:1412-1461`, "The no-chroot path returns 127 and 126".
   Owns 330 section 7.
3. T-0702, `TODO/interpose.md:127-280`, "One object per libc, and it must live
   inside the rootfs". Owns 80 and 159.
4. T-0709, `TODO/interpose.md:840-930`, the reader `podbox system abi`. Owns 80
   check E.
5. T-0706, `TODO/interpose.md:630-680`, "Classify the payload and decline with a
   named reason". Owns 159 clause B.
6. T-0410, `TODO/complete.md:585-677`, "Supply `/etc/nsswitch.conf`, or the
   supplied `/etc/passwd` is a no-op". Owns 90.
7. T-0409, `TODO/complete.md:500-565`, the fixup-must-not-touch-exit-code rule
   that 330 clause 5 measures.
8. T-1306, `TODO/podvm.md:540-637`, "the non-goals, one stance per blocked
   design". Owns 149.
9. T-0501, `TODO/enter.md:40-121`, "the entry sequence, and what `--device`
   maps". Owns 363.
10. T-0606, `TODO/supervise.md:430-490`, the mediation and supervision split.
    Owns 359.
11. T-1402, `TODO/podssh.md:67-87`, "Provide an interactive session without a
    pty". Owns 388.
12. T-1404, `TODO/podssh.md:114-140`, "Add the remote and machine SSH verbs
    after the transport holds". Owns 391.
13. T-1405, `TODO/podssh.md:142-173`, "Ship and smoke the SSH helper archive".
    Owns 394 by subject.
14. T-1210, `TODO/gate.md:788-878`, the engine-helper conversion that lists 80.
15. T-1211, `TODO/gate.md:881-932`, the engine-helper conversion that lists 90.
16. T-1212, `TODO/gate.md:935-1077`, the engine-helper conversion that lists 330.
17. T-1350, `TODO/gate.md:1775-1808`, "Make Windows proofs use explicit inputs
    and return build artifacts". Owns 397 by result file.
18. T-1005, `TODO/packaging.md:218-285`, "A session reaches the code in one
    command". Adjacent to session-start.sh.
19. T-1104, `TODO/milestones.md:269-290`, M3 `run` on the chroot rung. Records
    the 80-numbering rule at lines 262-265.
20. T-0607, `TODO/supervise.md:493-509`, "The lifecycle, twenty times, twenty
    passes". The subject 354 duplicates.

## What this audit did not do

- It ran nothing. No experiment, no `cargo test`, no plant. Every claim about
  a test's existence is from reading the file, and every claim about a result
  is from reading the result file.
- It did not read the six scripts in the other nine groups.
- It did not open `experiments/lib/engine.sh`, `scripts/dev.sh`,
  `scripts/common/bootstrap-env.sh` or `scripts/common/exit-codes.sh` in full.
  They are named here as collaborators, and a reader converting the drives
  must read them.
- It did not open `crates/podbox-enter/src/abi.rs` past line 1374 of 1503, nor
  `crates/podbox-probe/src/nongoals.rs` past its `mod tests` header. Both were
  read far enough to place the named tests and read their bodies; neither was
  read in full.
- It did not check whether `scripts/plant.sh`'s existing 26 plants already
  cover any of the units proposed here. That is a check the implementor runs.
