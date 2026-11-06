# Group 10 audit

## Group 10 summary

Twelve scripts, 2,902 lines, read in full. Verdicts:

| verdict | count | scripts |
| --- | --- | --- |
| RUST-TEST | 1 | `experiments/145-podvm-parity.sh` |
| RUST-TOOL | 3 | `experiments/110-bloat-delta.sh`, `experiments/310-session-startup.sh`, `scripts/build-interpose.sh` |
| SPLIT | 2 | `experiments/106-interpose-identity.sh`, `experiments/250-negative-tests.sh` |
| KEEP-SHELL | 4 | `experiments/210-store-concurrency.sh`, `experiments/280-insecure-registry.sh`, `experiments/357-tcg-profile.sh`, `experiments/385-kvm-open.sh` |
| DELETE | 2 | `experiments/162-tar-symlink-modes.sh`, `experiments/386-podssh-partial.sh` |
| Total | 12 | |

Total lines in the group, from `refactor/00-orientation/assignment-map.md:133-144`:
436 + 275 + 192 + 165 + 305 + 360 + 453 + 123 + 203 + 64 + 89 + 237 = 2,902.

The three findings that decide the group's value:

1. **Two scripts are already covered by a Rust unit test, and one of them is a
   duplicate of a numbered experiment.** `162-tar-symlink-modes.sh` exists to
   catch a defect that `tests::fchmodat_forwards_flags` in
   `crates/podbox-interpose/src/lib.rs:2955` now catches at the unit level.
   `TODO/interpose.md:1476` records the unit test as the guard and the script
   only as the end-to-end confirmation. Delete the script.
2. **`386-podssh-partial.sh` has no owning entry and reuses a retired number.**
   The audit note at `TODO/milestones.md:883` says the earlier unpublished
   KVM driver "reused number 386"; `386-podssh-partial.sh` is that name.
   `experiments/README.md:3` says "Do not reuse a number."
3. **`110-bloat-delta.sh` is the only place the release ceiling lives, and
   `scripts/check-todo.py:406-419` reads that fact.** Moving it to Rust without
   moving check 17 with it turns the gate red. That is a constraint, not a
   blocker, and the plan below carries it.

---

## Group 10 — experiments/106-interpose-identity.sh

**What it measures.** 436 lines. Seven clauses over the interposer identity
tier. Clause A is the control: with no `PODBOX_IDENTITY`, the loaded victim's
report is byte-identical to the bare victim's. Clause G is the errno-by-call
table, printed and recorded, never asserted absolutely. Clauses D, E and F
drive `podbox run --user`, `--strict`, and a declined static payload end to end.
The header forbids asserting any absolute `setuid` outcome.

**Pinned inputs.** `PODBOX_IDENTITY=1000:100`; fedora by digest at line 64;
alpine by digest at line 65; both interposer objects under
`crates/podbox-interpose/target/`; the victim built here from inline C
(lines 95-150).

**Host assumptions.** glibc job container that cannot run a daemon
(lines 191-197). `mktemp -d` scratch, `trap cleanup EXIT INT TERM` at line 59.

**Exit contract.** 0 all matched, 1 one did not, 2 a check could not run
(line 430-431).

**Timeouts.** `timeout 120` on every direct victim call (line 204-206). The
`podbox run` calls in D, E, F carry none.

**Cleanup.** `rm -rf "$WORK"` only. The store under `$WORK/store` goes with it.

**Positive control.** Clause A, bare against loaded (line 213-222).

**Failure control.** None as a separate clause. Clause A would fail if the
object misbehaved with no identity set, which is the control reading.

**TODO entries that own it.**
- `TODO/interpose.md` T-0711, `Prove:` at line 1103, `**Done 2026-09-19.**`
  at line 1108, `**Done 2026-09-26.**` at line 1141.

**Result files.** `experiments/results/interpose-identity.txt`, dated
2026-09-26T06:17:16Z, `podbox 0.1.0-beta.7`.

**Verdict: SPLIT.** Two of the seven clauses are pure logic over deterministic
records and belong in a Rust unit test; four need a live container; clause G
is a recording, not an assertion.

### Conversion plan, part 1: clauses A, B, C0, G become `#[cfg(test)]` in the interposer

- **Assertions as triples today.** (A) precondition: object built, victim
  built, no `PODBOX_IDENTITY`; action: run the victim bare and with
  `LD_PRELOAD`; expected: identical report. (B) precondition: `PODBOX_IDENTITY=1000:100`;
  action: run the victim; expected: every setter `RC=0` and every getter
  answers `1000:100`, with `setreuid(2000,-1)` reporting `2000:1000:1000`.
  (G) precondition: an unmapped wall; action: call each setter; expected: each
  refused setter prints its call and errno beside the number.
- **Level: unit.** `docs/conventions/code.md:28` calls these deterministic
  logic, and the record arithmetic the script asserts is a pure function of
  the requested identity and the current record. The errno-name table is
  already pure: `wall_errnos_carry_their_names` at
  `crates/podbox-interpose/src/lib.rs:3114`.
- **Target.** `crates/podbox-interpose/src/identity.rs`, a new
  `#[cfg(test)] mod tests`. The crate's seven files with `cfg(test)` confirm
  the inline convention; only `crates/podbox-ssh` has a `tests/` directory.
- **Fixtures.** None new. The C victim becomes a Rust `#[test]` calling the
  crate's own setter/getter wrappers directly against `PODBOX_IDENTITY` set on
  the test process, which is the same input the script sets on the process.
- **Plant.** Remove the `PODBOX_IDENTITY` read from `identity.rs` and assert
  `identity.rs:26`'s `IDENTITY_VAR` no longer selects the record. A second
  plant drops the errno-name table and asserts a bare number comes back.
- **Proof command.**
  `RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu identity`
- **`exit 2` becomes.** A Rust test has no third state. The unit test either
  has its inputs or it fails. The skip conditions in the script are build-tool
  conditions that stop existing once the target is a fixed `--target`.
- **Result file.** `experiments/results/interpose-identity.txt` is superseded
  by this part. Keep it as the historical record of the 2026-09-26 reading; the
  entry's table at `TODO/interpose.md:1151-1161` quotes it.

### Conversion plan, part 2: clauses C, D, E, F stay a shell experiment

- **Why they cannot move.** They need a podbox binary, a pulled image, a
  preloaded object placed into an extracted rootfs, and `podbox run`. No Rust
  test harness in this tree builds or runs a container image. `code.md:29`
  reserves this for "the actual default path".
- **What must change anyway.** Clause G's assertion is vacuous: it asserts only
  that the victim ran. That half becomes the unit test above and clause G
  leaves the script.
- **Entry update.** `TODO/interpose.md` T-0711's `Prove:` at line 1103 names
  the script. It keeps naming it, for the clauses that stay.

---

## Group 10 — experiments/110-bloat-delta.sh

**What it measures.** 275 lines. Three things. The total size of the shipping
release binary. The size of the two interposer objects beside it. The size
delta of a named dependency area against the committed baseline, with a
scaffold control that separates "the candidate costs nothing" from "the
scaffold was unreachable and `lto` deleted it" (lines 99-109, 186-205).

**Pinned inputs.** `TARGET=x86_64-unknown-linux-musl` (line 47);
`CEILING_BYTES=8000000` (line 43); the baseline file
`experiments/results/bloat-baseline.txt`.

**Host assumptions.** `cargo`, `readelf`, `stat -c%s`, and `cargo-bloat`
optionally. `mktemp -d` and a `trap` at line 53.

**Exit contract.** 0 measured and under the ceiling, 1 over the ceiling or the
build failed, 2 could not run (lines 40-41). Note the second `exit 2`: a
missing `cargo-bloat` exits 2 (line 272-274).

**Timeouts.** `timeout 60` on the scaffold run only (line 105). The
`cargo build` has none.

**Cleanup.** `rm -rf "$WORK"`.

**Positive control.** The scaffold run, `PODBOX_SWEEP=1` (line 105).

**Failure control.** The zero-delta-with-dependencies arm (lines 187-205)
refuses the reading when the scaffold said nothing.

**TODO entries that own it.**
- `TODO/deps.md` T-0910, `Prove:` at line 618, `**Done 2026-09-08.**` at
  line 620. Also the `Prove` of nine other entries in `TODO/deps.md` at lines
  67, 120, 162, 197, 246, 318, 370, 426, 532.
- `TODO/gate.md` T-1207 item 3, `Approach:` line 482, `Prove:` line 504.
- `TODO/gate.md` T-1201, `Approach:` at line 392 (check 19 reads the script's
  cargo calls).

**Result files.** 26 `experiments/results/bloat-*.txt` files. The committed
baseline reads `total_bytes 496184`, `third_party_crates 0`,
`date 2026-09-08T12:27:29Z`. `bloat-interpose.txt` reads
`total_bytes 3503032`, `interpose_musl_bytes 335320`,
`interpose_gnu_bytes 315432`, `date 2026-09-23T00:52:28Z`.

**Verdict: RUST-TOOL.** This is the release gate. `.github/workflows/gate.yml:131`
runs `./experiments/110-bloat-delta.sh ci` as a job step.

**⛔ Constraint the implementor must read first.**
`scripts/check-todo.py:228-229` names this script as `CEILING_SCRIPT` and
searches its text for `^CEILING_BYTES=(\d+)$`. `scripts/check-todo.py:406-419`
then refuses any other file the project wrote that names that number.
`scripts/plant.sh:278-292` plants cases 17a, 17b and 17c against it.
Moving the declaration out of this path turns check 17 red until
`CEILING_SCRIPT` and the plants move in the same change.

### Conversion plan

- **Assertions as triples today.** (Total) precondition: the shipping profile
  built for `x86_64-unknown-linux-musl`; action: `stat` the binary; expected:
  `total < CEILING_BYTES` and `PT_INTERP` count 0. (Delta) precondition: a
  committed baseline carrying `total_bytes`; action: subtract; expected: a
  delta, and a zero delta with a reachable scaffold recorded as below
  instrument resolution rather than as free. (Objects) precondition:
  `scripts/build-interpose.sh` has run; action: `stat` both `.so` files;
  expected: both under `INTERPOSE_CEILING_BYTES`, or `absent`.
- **Level: deployment.** `code.md:29`: "integration and deployment proof for
  the actual default path". A size ceiling is a property of the shipped
  artefact, not of any function.
- **Target crate and binary.** New crate `crates/podbox-size`, one
  `[[bin]]` named `podbox-size`. Keeping the ceiling in a Rust source is what
  lets check 17 find it.
- **CLI surface.**
  `podbox-size total [--target <triple>] [--result-file <path>]`
  `podbox-size objects [--result-file <path>]`
  `podbox-size delta --area <name> [--baseline <path>]`
  `podbox-size gate [--area <name>]` for the CI shape, which today is the
  script run as `ci`.
  Exit codes keep 0, 1, 2, because the gate workflow reads them.
- **Fixtures.** None. The binary, the objects and the baseline file are the
  inputs. The scaffold probe is one `Command` on the built binary with
  `PODBOX_SWEEP=1` set, which is what line 105 does.
- **Plant.** Change `CEILING_BYTES` to a value the committed baseline already
  exceeds and assert the tool exits 1 naming the ceiling. A second plant
  removes the `PODBOX_SWEEP` scaffold arm and asserts a zero delta with a
  dependency present is refused rather than recorded. `scripts/plant.sh:278-292`
  already carries the 17a/17b/17c shapes against this subject; re-point them at
  `crates/podbox-size/src/lib.rs` and update `CEILING_SCRIPT`.
- **Proof commands.**
  `cargo run -p podbox-size --bin podbox-size -- gate --area ci`
  `cargo run -p podbox-size --bin podbox-size -- total`
  `python scripts/check-todo.py` and `python scripts/plant.sh` after the
  check-17 re-point. Target triple is required for the build step:
  `--target x86_64-unknown-linux-musl`.
- **`exit 2` becomes.** Kept. This is a binary, not a test, so three exit
  states are available: the missing `cargo-bloat` and the absent target
  remain 2, the over-ceiling reading stays 1.
- **Result files.** All 26 `bloat-*.txt` files stay. The tool writes the same
  key/value format so check 17's `total_bytes` and check 23's
  `interpose_gnu_bytes` readers at `scripts/check-todo.py:811` keep working
  unchanged.

---

## Group 10 — experiments/145-podvm-parity.sh

**What it measures.** 192 lines. Five clauses about the tier flag and the
`podvm` multicall name. Clause 0 reads the parity table out of the binary and
checks five rows exist. Clause 1 checks both help texts are byte-identical.
Clauses 2 through 4 drive routing: the flag refuses with `PODBOX_EXIT_RUNTIME_ERROR`,
a third value is a flag error, `--podbox-qemu-arg` needs the machine tier, the
name defaults to machine, an explicit flag overrides it, `exec` mirrors `run`,
and `ps` refuses the flag as unlisted.

**Pinned inputs.** `never-pulled-145:tag`, a reference guaranteed not to
resolve so the refusal happens before any network. `jq` on PATH (line 49).

**Host assumptions.** Linux, or a job container with `/work` as the working
directory. `ln -sf` for the `podvm` name (line 57). `set -u` and deliberately
no `pipefail`, because the script also runs under dash (lines 24-26).

**Exit contract.** 0 every row green, 1 a row disagreed, 2 the binary, `jq` or
the table could not run (lines 26-27).

**Timeouts.** `timeout 120` on every `run`/`pvm` call (lines 108, 111).

**Cleanup.** `rm -rf "$WORK"` on the checkout-side scratch
`experiments/.sweep145-work` (lines 34-35).

**Positive control.** Clause 3's second half: `podvm run --podbox-tier=chroot`
must state the override and proceed past the tier, which proves the flag
parser ran rather than everything refusing.

**Failure control.** `run --podbox-tier=sandbox` and `ps --podbox-tier`, which
are inputs the table must refuse.

**TODO entries that own it.**
- `TODO/podvm.md` T-1302, `Prove:` at line 288, `**Done 2026-09-21.**` at
  line 291.
- `TODO/podvm.md` T-1301 names `145` nowhere; T-1302's `Approach:` at line
  250 rejects the whitespace-splitting alternative with the cubic-vm report.

**Result file.** `experiments/results/podvm-parity.txt`, dated
2026-09-21T17:48:21Z, `podbox 0.1.0`, `jq-1.6`, `19 driven, 0 mismatches`.

**Verdict: RUST-TEST.** Every assertion is already made against an internal
function.

### Conversion plan

- **Assertions as triples today.** (Flag parsing) precondition: argv
  containing a tier value; action: `tier::want`; expected: `machine` and
  `chroot` map, `sandbox` and `""` do not. (Name routing) precondition:
  `invoked` and `flag`; action: `tier::resolve`; expected: `podvm` with no
  flag is `Tier::Machine`, `podbox` is `Tier::Ladder`, `podvm` with `chroot`
  is `Tier::Chroot` and carries the override note. (Emulator argument
  collection) precondition: repeated `--podbox-qemu-arg`; action: the parser;
  expected: one token per occurrence, a value containing a space stays whole,
  and the flag is refused where the tier is not machine. (Exit code)
  precondition: a refused tier; action: `tier::enter_machine`; expected:
  `EXIT_RUNTIME_ERROR`. (Parity table) precondition: the table as published;
  action: query by verb and flag; expected: five rows present.
- **Level: unit.** `code.md:27`. Every input is a string or a small struct and
  every expected value is a constant.
- **Target.** `crates/podbox-cli/src/tier.rs`, extend the existing
  `#[cfg(test)] mod tests` at line 226. Four of the six assertions already
  exist there by name: `the_flag_spells_two_tiers_and_nothing_else`,
  `podvm_defaults_to_machine_and_podbox_to_the_ladder`,
  `podvm_with_chroot_states_the_override_and_podbox_does_not`,
  `refusing_a_tier_is_a_runtime_error_on_any_machine`. The parser arm goes in
  `crates/podbox-cli/src/run.rs` and `crates/podbox-cli/src/exec.rs`, beside
  `the_tier_flag_takes_two_values_and_qemu_args_collect_whole` at
  `run.rs:1936` and `exec.rs:1086`. The table-shape arm goes in
  `crates/podbox-cli/src/parity.rs`.
- **What is already covered and needs no new test.** The help-text equality.
  `crates/podbox-cli/src/man.rs:37` reads `crate::names::ALIASES`, and
  `names.rs:32` lists `podvm`. The five word assertions in clause 1 are a
  `USAGE` constant, so a test asserting `USAGE` carries `run`, `exec`,
  `podvm` and `--podbox-tier` replaces the `grep` loop.
- **Fixtures.** None. Every value is a literal.
- **Plant.** Change `tier::want` to accept `sandbox` and assert the parser
  tests fail. Second plant: make `tier::resolve` ignore the explicit flag and
  assert `podvm_with_chroot_states_the_override_and_podbox_does_not` fails.
  Third plant: split a `--podbox-qemu-arg` value on whitespace and assert the
  collection test fails with the space-preserving expectation.
- **Proof command.**
  `cargo test -p podbox-cli tier` and `cargo test -p podbox-cli qemu_args`.
  No feature flag and no target triple: the crate builds natively.
- **`exit 2` becomes.** The unit tests cannot hit it. The
  `podbox_exit_codes "$BIN"` precondition at line 53 disappears, because
  `podbox_probe::exit::CASES` at `crates/podbox-probe/src/exit.rs:62` is
  the table the tests read directly. That also retires the `jq`
  dependency for this measurement.
- **Result file.** `experiments/results/podvm-parity.txt` is superseded. The
  conditions block carries a lane reading with no lasting value once the
  assertions are unit tests. Move it to `docs/history/` if the entry's
  closure quote at `TODO/podvm.md:301` keeps referring to it; otherwise delete
  it and update that line to name the test.

---

## Group 10 — experiments/162-tar-symlink-modes.sh

**What it measures.** 165 lines. Whether a tarball carrying symlinks unpacks
under podbox with the same result as in the driver container. Four rows per
side: `TAR_RC`, two link targets, one file mode.

**Pinned inputs.** The debian bookworm-slim digest at line 23, which the
header says is "the row 240 and 152 drive".

**Host assumptions.** Linux runs the binary natively; anything else stages it
into the driver through `experiments/lib/engine.sh` (lines 26-37). Requires a
C toolchain and `tar` inside the image.

**Exit contract.** 0 the unpack ran and matched on both sides, 1 a side
failed, 2 nothing ran.

**Timeouts.** `eng_run 1200` on the non-native lane (line 111); none on the
native lane.

**Cleanup.** `rm -rf "$WORK"` and `eng_clear` (lines 115, 30).

**Positive control.** The `ctl` side, the same subject run straight in the
driver with no podbox.

**Failure control.** None in the script. The defect it was filed for is the
defect.

**TODO entries that own it.**
- `TODO/interpose.md` T-1311, `Prove:` at line 1471, `**Done 2026-09-21.**`
  at line 1476. The `Approach:` at line 1466 names the unit test as the first
  step: "Add a unit test that compares the interposed `fchmodat` against
  libc's own for a symlink with `AT_SYMLINK_NOFOLLOW` and with flags 0."

**Result files.** `experiments/results/tar-symlink-modes.txt` (2026-09-21,
`podbox sha256 6adb54c6444c7303cd8dffe9492abe5503ac29203b3058f67b806324b49855ce`)
and `experiments/results/tar-symlink-modes-prefix.txt`, the pre-fix run.

**Verdict: DELETE.** The defect is fixed and the fix is guarded by a unit test.

**Evidence.**
- `crates/podbox-interpose/src/lib.rs:356` declares the real function with four
  arguments: `next_fchmodat = "fchmodat"(c_int, *const c_char, c_uint, c_int) -> c_int`.
- `crates/podbox-interpose/src/lib.rs:1038` forwards all four:
  `path_at_int!(fchmodat, next_fchmodat, (dirfd: c_int, path: *const c_char, mode: c_uint, flags: c_int), dirfd, path)`.
- `crates/podbox-interpose/src/lib.rs:2955` holds `fchmodat_forwards_flags`,
  which compares the interposed entry point against libc's own on a symlink
  with flags 0 and with `AT_SYMLINK_NOFOLLOW`.
- `TODO/interpose.md:1476` names that test as the guard: "Guard:
  `tests::fchmodat_forwards_flags`". The same paragraph records the script
  only as the end-to-end confirmation on host podman 6.1.2.

**What deleting costs.** The end-to-end confirmation that a real `tar` through
a real payload now succeeds under podbox. The saved result file already holds
that reading, and `TODO/interpose.md:1483` quotes it.

**Result files.** Keep both. `tar-symlink-modes-prefix.txt` is the negative
result and `TODO/RULES.md:63` requires committing negative results.

---

## Group 10 — experiments/210-store-concurrency.sh

**What it measures.** 305 lines. Four clauses against the store's seven
invariants I1 to I7, declared in
`crates/podbox-image/src/store.rs:37-76`. Clause 1: eight concurrent pull
processes over three references into one index. Clause 2: a prune racing a
live hold. Clause 3: a SIGKILL mid-pull, then any later command sweeps the
litter. Clause 4: a sweep that must leave a live writer alone.

**Pinned inputs.** Three alpine references from `public.ecr.aws`, tag-based
rather than digest-pinned (lines 33-37). Eight workers
(`PODBOX_CONCURRENCY_WORKERS`, line 38).

**Host assumptions.** A real network and a registry with no quota. `sha256sum`
required (line 45). `mktemp -d` with a `trap` at line 28.

**Exit contract.** 0 every clause held, 1 one did not, 2 could not run.
Line 303-305 gives three states and says "could not run is never a failure".

**Timeouts.** `timeout 900` per worker, `timeout 300` on the holder and on
prune. Lines 217-223 explain why the SIGKILL victim deliberately has no
`timeout`: `timeout 900 podbox pull &` makes `$!` the pid of `timeout`.

**Cleanup.** `rm -rf "$WORK"`.

**Positive control.** Clause 4, the live-writer arm.

**Failure control.** Clause 2's `chroot(2) is denied` skip at line 191, which
is the measurement of a machine that refuses the payload before the race.

**TODO entries that own it.**
- `TODO/image.md` T-0210, `Prove:` at line 969, `**Done, 2026-09-09.**` at
  line 972. The `Decision:` at line 966 is explicit: "A stress experiment
  rather than a unit test. The failure is a race, and a race that only a mock
  can produce is a race the mock's author imagined."
- `TODO/image.md` T-1321, `Source:` at line 2008, which names this script's
  `verify()` shell loop as "the starting shape" for the product verb.

**Result file.** `experiments/results/store-concurrency.txt`, dated
2026-09-25T08:26:23Z: clause 1 `blobs=12 bad=0 records=3 missing=0
partials=0`, clause 2 `image locks held while the payload runs: 1`, clause 3
`staging files after SIGKILL: 1` and `after any later command: 0`, clause 4
`the pull exited 0`.

**Verdict: KEEP-SHELL.** The specific host dependency is real processes
racing a shared filesystem through a network registry, plus `kill -9` on a
running pull and a prune racing it. `TODO/image.md:966` already ruled the
alternative: the decision field says a mock would be a race the mock's author
imagined, and this audit has no measurement that overrules it.

**What may still change.** The `verify()` function is shell and the store now
has a product verb. `TODO/image.md:2008-2011` says the `verify()` loop is
"shell, not product" and T-1321 built `podbox verify`. Substituting
`podbox verify` for the shell loop removes about thirty lines and one reader.
That is a change to keep, not a conversion.

**Note on tag pinning.** Lines 33-37 use `:latest`, `:3.20` and `:3.19`, not
digests. `scripts/check-todo.py:21a/21b` plants cases that refuse an
unqualified image in a `Prove:` line, but this script's references are not in
a `Prove:` line. The entry's own `Prove:` at line 969 names no image. The
result file's conditions block names only `alpine:latest` by tag, so the
reading applies to whatever those tags held on 2026-09-25.

---

## Group 10 — experiments/250-negative-tests.sh

**What it measures.** 360 lines. Eleven refusals, driven from outside through
the shipped binary. Every clause asserts two things: the exit code, and that
the message names the reason. Clause 1 is `TOOL.md` section 9's three. Clause
2 is `--strict` and its fourth reason. Clause 3 is `-t` where `/dev/ptmx` is
unusable. Clause 4 is the parity table refusing an unlisted flag, a `None`
flag and a `None` verb. Clause 5 is HTTPS-only with a timeout because the
failure it prevents is a hang. Clause 6 is a container podbox did not see end
having no exit code. Clause 7 is the census exit-code rule.

**Pinned inputs.** `public.ecr.aws/docker/library/alpine:3.20` (line 43) and
`registry.opensuse.org/opensuse/leap:15.6` (line 49). A Go victim generated by
`experiments/src/govictim.sh`. Exit codes read from the binary through
`scripts/common/exit-codes.sh`.

**Host assumptions.** `jq` (line 55). `probe --json` to read
`.ptmx.usable` (line 244). An announced CA bundle, which the script installs
itself if `apt-get` exists (lines 88-99). A detached container whose launcher
pid it can `kill -9` (lines 291-300).

**Exit contract.** 0 every refusal happened and named its reason, 1 one did
not, 2 could not run. Three states: `skipped=1` sets 2 (line 359).

**Timeouts.** `timeout 600` on the pull, `timeout 300` per refusal, `timeout 30`
on the HTTPS clause (line 274), `timeout 900` on the census.

**Cleanup.** `rm -rf "$WORK"`; the store is inside it.

**Positive control.** The second half of clause 2: the same run without
`--strict` must exit 0 (lines 202-205). Without it, `--strict` could be the
default in disguise.

**Failure control.** Clause 5 asserts `rc != 124`, so a hang fails rather than
passes. Clause 3's arm selection (lines 244-256) is the conditional control:
a machine with a pty asserts the run succeeds and records that the refusal arm
was not measured.

**TODO entries that own it.**
- `TODO/milestones.md` T-1109, `Prove:` at line 578, `**Done 2026-09-25.**` at
  line 580, `**Partial, 2026-09-09.**` at line 598.
- `TODO/milestones.md` T-1109's `Decision:` at line 574: "Negative tests live
  in `experiments/` with the positive ones and share their exit-code
  convention, rather than in a separate suite."
- `TODO/supervise.md` line 531 records a finding observed while writing it.

**Result file.** `experiments/results/negative-tests.txt`, dated
2026-09-25T09:07:12Z, `podbox 0.1.0-beta.7`, `flag-error code 125`,
`cli-error code 1`. All eleven rows read `ok`, clause 7 reads `rc=2`.

**Verdict: SPLIT.** Four clauses are unit tests of the message the source
already prints. Seven need a live container or a machine with a particular
property.

### Conversion plan, part 1: clauses 1a, 1b, 4 become `#[cfg(test)]`

- **Assertions as triples today.** (Network) precondition: `run` with
  `--network=none`; expected: the parity row's own note, containing
  "there is no network namespace to select", at `EXIT_FLAG_ERROR`. (Volume)
  precondition: `-v host:/mapped:ro`; expected: "podbox cannot mount(2) on
  this runtime, so a volume", at `EXIT_FLAG_ERROR`. (Unlisted flag)
  precondition: `run --no-such-flag`; expected: "It has no row in the parity
  table", at `EXIT_FLAG_ERROR`. (None status) precondition:
  `run --privileged`; expected: "status None", at `EXIT_FLAG_ERROR`. (None
  verb) precondition: `stats`; expected: "cgroup this runtime does not grant",
  at `EXIT_RUNTIME_ERROR`.
- **Level: unit.** These are refusals produced by the parity table and the
  verb dispatchers, both pure functions over the argument vector.
  `crates/podbox-cli/src/parity.rs:611` holds the unlisted-flag message.
- **Target.** `crates/podbox-cli/src/parity.rs` and
  `crates/podbox-cli/src/lifecycle.rs`, extending each crate's existing
  `#[cfg(test)]` module. `parity.rs:999` already has
  `admit_all_refuses_before_any_arm_runs`.
- **Fixtures.** None. The parity `ROWS` table at `parity.rs:359` and its
  neighbours is the fixture.
- **Plant.** Change the `--network` row's note in `parity.rs` and assert the
  refusal test fails on the needle. Second plant: change the unlisted-flag
  message at `parity.rs:611` to the word `unknown option` alone and assert the
  test that requires "no row in the parity table" fails.
- **Proof command.** `cargo test -p podbox-cli parity` and
  `cargo test -p podbox-cli refuse`.
- **`exit 2` becomes.** The three states collapse. A test either matches the
  table or it fails. The clause-1b reason that the message must name is kept
  whole, which is the point the entry makes at line 618.
- **Result files.** No new file. These rows are assertions inside the crate.

### Conversion plan, part 2: clauses 1c, 2, 3, 5, 6, 7 stay a shell experiment

- **Why.** 1c stages `experiments/src/govictim.sh` into an extracted rootfs
  and runs `podbox run` on it. 2 counts T-0412's steps, which requires an
  image that carries them. 3 needs a machine where `/dev/ptmx` is unusable.
  5 needs a network failure to not happen. 6 needs a detached container whose
  launcher can be killed. 7 runs another experiment.
- **What may be added.** The errno-name unit test already exists at
  `crates/podbox-interpose/src/lib.rs:3114` for a related refusal, so the Go
  decline message could gain the same treatment. Whether it should is the
  implementor's call; the entry does not ask for it.
- **Entry update.** `TODO/milestones.md` T-1109's `Prove:` at line 578 keeps
  naming the script, which is correct: the script no longer carries all eleven.

---

## Group 10 — experiments/280-insecure-registry.sh

**What it measures.** 453 lines. Seven clauses over registry transport policy.
The two halves pull opposite ways and the script holds both: the default still
refuses plain HTTP and still refuses an untrusted certificate (clauses 1 and 2),
and an explicitly named registry becomes reachable (clauses 3, 5, 6 and 7).
Clause 6 is the announcement discipline: a downgrade is named on stderr, and
an ordinary pull says nothing.

**Pinned inputs.** The debian driver at line 68, the ECR `registry` image
pinned to an index digest at line 71, and the source image
`ghcr.io/pkgforge-dev/archlinux:latest` at line 302, which is tag-pinned.

**Host assumptions.** `experiments/lib/engine.sh` (lines 46-51). `openssl`
(line 166). `jq`. On a non-native lane, `zig` to build
`experiments/lib/tcpfwd.c` (lines 88-107) because the driver's `/etc/hosts`
pins `localhost` to 127.0.0.1. A proxy may be set in the environment and is
deliberately left set (lines 188-191).

**Exit contract.** 0 every clause held, 1 one did not, 2 could not run.

**Timeouts.** 60, 120, 300 and 900 seconds per call site (lines 153-159).
Fixture readiness is a bounded loop of 30 one-second `curl` attempts (lines
291-298).

**Cleanup.** `eng_rm` by name plus `eng_cleanup` plus `rm -rf "$WORK"`
(lines 53-64). The header at lines 54-59 explains why by name: ids captured
in `$( )` are lost when the subshell exits.

**Positive control.** Clause 6's second half, an ordinary pull that must say
nothing (lines 411-415).

**Failure control.** Clause 4, naming one registry insecure and checking a
second one is still refused (lines 358-369).

**TODO entries that own it.**
- `TODO/image.md` T-0213, `Prove:` at line 1379, `**Done 2026-09-09.**` at
  line 1381.
- `TODO/gate.md` T-1212, `Approach:` at line 946, `Prove:` at line 960,
  `**Done 2026-09-23.**` at line 965.

**Result file.** `experiments/results/insecure-registry.txt`, dated
2026-09-23T02:00:43Z, `engine 29.8.1 (fixtures: docker)`. All seven clauses
green. Clause 7's last line reads `registries.conf:2`, the line-number
refusal.

**Verdict: KEEP-SHELL.** The host dependency is two live `registry:2`
instances, one speaking plain HTTP and one with a script-generated certificate
nothing trusts, published on host ports and reached through a `tcpfwd`
forward on the non-native lane. `crates/podbox-image/src/transport.rs:324`
already has a `#[cfg(test)] mod tests` with
`tls_verify_true_beats_a_host_named_insecure` at line 357, so the policy
resolution is covered; what is not covered by any unit test is that the
transport actually reaches and refuses, and that needs the fixtures.

**What the experiment demonstrates that a unit test cannot.** `TODO/image.md:1385-1391`
records the defect this found: the first build pulled a loopback registry
through the environment's proxy and returned HTTP 405, which reads as a
broken registry. The fix is in the transport, and only a real socket proves it.

---

## Group 10 — experiments/310-session-startup.sh

**What it measures.** 123 lines. Five clauses about a fresh session's cost. A
cold release build into an empty target directory. The same build warm. What
`bootstrap-env.sh --check` costs. How many words the session can read while
that happens. Whether `dev.sh start` returns immediately.

**Pinned inputs.** `x86_64-unknown-linux-musl`; `TODO/PROGRESS.md` and
`AGENTS.md` for the word count (line 98).

**Host assumptions.** At least 2,048 MB free under the repository, checked
before the write (lines 34-38). `cargo` on PATH.

**Exit contract.** 0 it ran, 2 it could not (line 20). There is no 1: the
script has no failure state, only a measurement and a refusal.

**Timeouts.** None. The cold build is unbounded.

**Cleanup.** `rm -rf "$WORK"`, which holds the cold target directory.

**Positive control.** Clause 5's second half, `dev.sh wait` then `status`,
which must report `ready` (line 114-115).

**Failure control.** None. Nothing here can be made to fail.

**TODO entries that own it.**
- `TODO/packaging.md` T-1313, which quotes the result at line 240 of that
  file and names the script at line 278: "`experiments/310-session-startup.sh`
  takes the numbers and clause 5 is `dev.sh` returning in 1 s".
- The `Approach:` that names `dev.sh` as the answer is in the same entry.

**Result file.** `experiments/results/session-startup.txt`, dated
2026-09-09T03:19:54Z, `6.18.44-fc-v24`, 4 cpus: cold build 29 s, 150 M, 87
crates; warm 0 s; `dev.sh start` 1 s.

**Verdict: RUST-TOOL.** This is a performance reading about the development
environment, and it is re-taken whenever a session starts on a new machine.
`scripts/dev.sh` is the durable workflow this measures and `scripts/dev.sh`
itself is another group's script.

### Conversion plan

- **Assertions as triples today.** (Cold build) precondition: an empty target
  directory and at least 2,048 MB free; action: `cargo build --release
  --target x86_64-unknown-linux-musl --target-dir <cold>`; expected: exit 0,
  and the report records the wall clock, the size and the crate count.
  (Warm build) precondition: the same directory just built; action: build
  again; expected: a shorter reading. (Reading) precondition: the two
  documents; action: `wc -w`; expected: a count, so a reader can check it.
  (Non-blocking) precondition: a cold tree; action: `dev.sh start`; expected:
  it returns in seconds, and `dev.sh wait` then `status` reports `ready`.
- **Level: deployment.** `code.md:29`. It measures the machine, not a
  function.
- **Target crate and binary.** New crate `crates/podbox-session`, one
  `[[bin]]` named `podbox-session`.
- **CLI surface.**
  `podbox-session startup [--result-file <path>] [--min-free-mb <n>]`
  `podbox-session words [--result-file <path>]`
  `podbox-session dev-check`
  Exit codes keep 0 and 2, matching the script's own two-state contract.
- **Fixtures.** None. `std::fs` reads the two documents; `std::process::Command`
  drives cargo and `dev.sh`.
- **Plant.** The free-space guard is the one branch that can refuse. Set
  `--min-free-mb` above the real free space and assert the tool exits 2 naming
  the shortfall rather than starting a cold build. Second plant: point the
  words subcommand at a file that does not exist and assert it exits 2 naming
  the file.
- **Proof command.**
  `cargo run -p podbox-session --bin podbox-session -- startup`.
  No feature flag. The target triple is a default of the tool, not a build
  requirement.
- **`exit 2` becomes.** Kept, for the same reason as `podbox-size`: a binary
  keeps the third state that a test cannot have.
- **Result file.** `experiments/results/session-startup.txt` stays and is
  rewritten by the tool in the same `key value` shape. It is a reading about
  one machine on one day and `docs/conventions/prose.md:46` forbids replacing
  its values with another host's.

**What the tool cannot carry.** Clause 3's install path. Lines 88-92 state the
install half is not measured because the container already has every
component. The tool keeps that wording verbatim in its report; it must not
estimate a number.

---

## Group 10 — experiments/357-tcg-profile.sh

**What it measures.** 203 lines. Five clauses about the TCG machine profile.
Clause 1: the probe document carries `tiers.machine.refusal == null`,
`profile` `tcg`, the accel leg listing `tcg`, and `/dev/kvm` and `/dev/net/tun`
denied. Clause 2: the evidence text names the profile with the no-hardware
sentence and the tun shape. Clause 3: both names print the TCG holds message
at 125. Clause 4: with qemu hidden from `PATH` the tier refuses naming the
emulator. Clause 5: `146` re-driven with this binary boots to `VMR-GUEST-READY`.

**Pinned inputs.** The alpine digest and the `vmlinuz-virt` digest, pinned
inside `146` and quoted in the result file. The store, because the probe
caches per store (`TODO/probe.md` T-0111), so clause 4 uses a fresh
`store-noemu` (lines 168-175).

**Host assumptions.** `REPO="$(pwd)"` rather than `$0` (line 31), because the
Windows wrapper runs the job at `/in/job.sh`. `bootstrap-env.sh rust cc zig
tools` (line 46). `apt-get` for qemu (lines 88-96). `jq`. `cpio`, `python3`,
`curl`. A kvm-less lane.

**Exit contract.** 0 every clause matched, 1 a clause disagreed, 2 the lane
could not run. Line 203 is `[ "$fail" -eq 0 ]`, which yields 0 or 1 and
never 2 at the end; 2 comes from the early exits at lines 43, 44, 51, 56, 64,
74, 76, 99, 105, 130.

**Timeouts.** `timeout 1200` on bootstrap and interpose, `timeout 1800` on the
build and on `146`, `timeout 600`/`1200` on apt, `timeout 120` per probe step.

**Cleanup.** No `trap`. `$WORK` at line 33 is `rm -rf`'d at the start of the
next run rather than at exit. A failed run leaves
`experiments/.sweep357-work` behind.

**Positive control.** Clause 4 uses an empty `PATH` directory rather than
editing the real one, so the refusal is caused by the hiding and not by a
damaged environment.

**Failure control.** Clause 1's `denied` assertions: the legs must read
denied, so a probe that stopped discriminating fails.

**TODO entries that own it.**
- `TODO/podvm.md` T-1301, `Prove:` at line 99, `**Done, 2026-09-25.**` at
  line 160.

**Result file.** `experiments/results/tcg-profile.txt`, dated
2026-09-25T14:15:28Z, `QEMU emulator version 7.2.22 (Debian 1:7.2+dfsg-7+deb12u18+b3)`,
`kvm absent (TCG lane)`. The appended 146 section records
`the guest printed VMR-GUEST-READY`.

**Verdict: KEEP-SHELL.** The host dependencies are a kvm-less machine with
`qemu-system-x86_64` installed at run time, a real initramfs boot under TCG,
and a Windows job wrapper that changes the working directory. Clause 5 alone
is a QEMU boot.

**What may still change.** Clauses 1, 2 and 4 are covered at unit level
already. `crates/podbox-probe/src/machine.rs:305-375` holds five tests over
the same assessor: `six_clear_legs_hold_the_full_tier`,
`a_denied_kvm_leg_runs_tcg_without_a_refusal`,
`a_denied_tun_leg_runs_tcg_without_a_refusal`,
`an_accelerator_list_without_tcg_blocks_every_profile`,
`a_kvm_node_the_emulator_does_not_list_is_not_full`,
`a_skipped_emulator_blocks_every_profile`. The script keeps the probe
document as published and the boot.

**Cleanup defect worth fixing.** No `trap` at line 33. Every other script in
this group removes its scratch. This one leaves it. That is a small change to
keep.

---

## Group 10 — experiments/385-kvm-open.sh

**What it measures.** 64 lines. Whether the wsl-toolkit base exposes a
functional `/dev/kvm`: opened `O_RDWR`, `KVM_GET_API_VERSION` with a null
argument answering 12, and `KVM_CREATE_VM` returning a usable fd. The header
says the null argument is load-bearing, because passing a buffer to that
`_IO` ioctl returns `EINVAL` on this kernel.

**Pinned inputs.** `wsl-toolkit --instance podbox base exec --script`, and the
git commit at line 47. `ioctl` numbers `0xAE00` and `0xAE01` (lines 32, 35).

**Host assumptions.** `wsl-toolkit` and `timeout` on `PATH` (lines 17-21), a
Linux `python3` inside the base, and the host's `/dev/kvm`. On Windows the
script runs on the host side and stages the probe into the base.

**Exit contract.** 0 every clause held, 1 a clause disagreed, 2 could not run.
Line 64 is `[ "$rc" -eq 0 ]`, so 0 or 1; the 2s are the early exits at lines
13, 14, 19, 22, 41.

**Timeouts.** `timeout 1m` on the version, `timeout 5m` on the base exec.

**Cleanup.** `rm -rf "$work"` on a `trap` at line 23.

**Positive control.** None. Nothing here can fail for a reason other than the
subject.

**Failure control.** None.

**TODO entries that own it.**
- `TODO/milestones.md` T-1112, whose `Premise:` names "the licensed
  ValidationOS input", and whose remaining acceptance names `392`. The entry
  does not name `385` in its `Prove:`; the entry that does is the history
  record `docs/history/audit-before-2026-09-30/TODO/milestones.txt:970` and
  `docs/history/README.md:60`.

**Result file.** `experiments/results/kvm-open.txt`, dated
2026-09-28T13:15:27Z, `wsl-toolkit 6.0.0`, commit `a431cb7`,
`KVM_GET_API_VERSION 12`, `KVM_CREATE_VM fd 4`, `verdict KVM OPEN HOLDS`.

**Verdict: KEEP-SHELL.** The host dependency is `wsl-toolkit`, the tool this
project's Windows lane is built on, and `/dev/kvm` inside that tool's base. No
Rust crate in this tree can open a device node in a base it does not own.
`AGENTS.md:65` makes `wsl-toolkit --instance podbox` the only sanctioned
Windows route, and this script calls `wsl-toolkit` directly.

**Note.** The current `TODO/milestones.md` T-1112 `Prove:` line names `369`,
`370` and `392`, not `385`. The only live references to `385` are the session
summary at `TODO/SESSION-SUMMARY-2026-09-28-SSH.md:25` and two history files.
That is a record gap, not a reason to delete the script: the measurement is
still the gate on the KVM base and `experiments/README.md:35` records the
result as current evidence for a Windows toolkit base.

---

## Group 10 — experiments/386-podssh-partial.sh

**What it measures.** 89 lines. Four clauses. The SSH binaries exist. The
`podbox-ssh` crate suite is green. The CLI builds. Eight machine-surface rows
answer through the built binary.

**Pinned inputs.** `cargo test -p podbox-ssh`, `cargo build -p podbox-cli`,
and the built binary found by `ls -d target/*/debug/podbox | head -n 1`
(line 55), which is a glob rather than a pinned path.

**Host assumptions.** `cd /work` (line 12), so the script only runs inside the
Windows job wrapper. `/out` exists (lines 86-87). `bootstrap-env.sh zig
openssh` (line 31).

**Exit contract.** 0 every clause held, 1 a clause disagreed, 2 the lane could
not run. Line 89 is `[ "$fail" -eq 0 ]`, giving 0 or 1; the 2s are the early
exits.

**Timeouts.** `timeout 20m` on the test run, `timeout 25m` on the build. The
eight `check` calls at lines 64 have none.

**Cleanup.** `rm -rf "$work"` on a `trap` at line 16.

**Positive control.** The `ok` rows: a refusal that returns 0 where 125 was
wanted fails.

**Failure control.** None as a clause.

**TODO entries that own it.** None. `grep` across `TODO/` finds the script
only at `TODO/SESSION-SUMMARY-2026-09-28-SSH.md:22`, a session summary rather
than an entry. The two entries its header names, `TODO/podssh.md` T-1401 and
T-1404, name other scripts in their `Prove:` lines (T-1401 names
`382-restricted-sshd.sh`; T-1404 names `389`, `390` and `391`).

**Result file.** `experiments/results/podssh-partial.txt`, dated
2026-09-28T13:16:45Z, commit `a431cb7`, 24 unit tests, `verdict PODSSH PARTIAL
HOLDS`.

**Verdict: DELETE.** Three reasons, each sufficient.

1. **The number is retired.** `TODO/milestones.md:883` records the audit
   finding: "The audit found its unpublished driver reused number 386 and
   named private input paths. Experiment 392 replaces that driver with explicit
   inputs and owned scratch." `experiments/README.md:3` says "Do not reuse a
   number." This file occupies the retired number. Line 87 writes
   `Cargo.lock` to `/out`, a private mount path outside the repository, which is
   the other half of the same finding.
2. **Clause 2 is now `cargo test`.** It runs `cargo test -p podbox-ssh` and
   records the outcome. That is the gate command itself. `TODO/podssh.md`
   T-1401's `Prove:` at line 38 and T-1406's `Prove:` at line 188 both name
   `cargo test -p podbox-ssh` directly, with no script between.
3. **Clause 4's rows are unit tests.** The eight rows assert exit codes and
   message text for `machine`, `machine --help`, `machine ssh`,
   `machine ssh guest`, `machine ssh --help`, `machine relay`,
   `machine --bogus` and `man machine`. Every one of them is reachable as a
   function call: `machine` at `crates/podbox-cli/src/machine.rs:35` takes
   `&[String]` and returns `i32`, and `machine.rs:33-39` dispatches the
   unknown-member arm. `crates/podbox-cli/src/machine.rs:81-88` already holds
   `bare_and_help_print_usage_and_exit_zero` and
   `machine.rs:88` holds `assert_eq!(machine(&["relay".to_string()]), EXIT_RUNTIME_ERROR)`.

**What deleting costs.** A lane result proving the machine surface through a
*built binary* rather than through the function. `TODO/podssh.md` T-1402's
`Prove:` at line 79 and T-1404's `Prove:` at line 126 already carry
`cargo test --workspace` and other tracked drives, so the coverage does not
rest on this script.

**Result file.** Keep
`experiments/results/podssh-partial.txt` as the 2026-09-28 reading, and keep
the session summary line that cites it. `TODO/RULES.md:63` requires the
negative and historical record, and the audit at `TODO/milestones.md:883` is
itself evidence that the number must stay retired.

---

## Group 10 — scripts/build-interpose.sh

**What it measures.** 237 lines, and its header says what it is not: "Question:
none. This is a build step, not a measurement." Per target it builds the
cdylib and then asserts six things about the object: `DT_NEEDED` names the
right libc (lines 129-139), the newest `GLIBC_` need is at or below 2.27
(lines 145-157), `gettid` is not dynamically exported (lines 167-172), the
exported `T` set equals the globals in `interpose.map` both ways (lines
197-214), `rust_eh_personality` is not exported (lines 215-220), and the size
is under `INTERPOSE_CEILING_BYTES` (lines 225-231).

**Pinned inputs.** `TARGETS="${TARGETS:-x86_64-unknown-linux-musl
x86_64-unknown-linux-gnu}"` (line 29); `INTERPOSE_CEILING_BYTES=500000`
(line 48); `crates/podbox-interpose/interpose.map`; `dl-stub.S` and
`dl-stub.map`.

**Host assumptions.** `rustup target list --installed` (line 62); `zig` for the
musl linker (line 74); host `cc` for the stub (line 100); `readelf`, `nm`,
`stat`, `diff`, `wc`. `RUSTFLAGS` is set as an environment variable rather
than through a crate-local `.cargo/config.toml`, and the header at lines 6-11
gives the measured reason: cargo merges config files up the tree and appends
the parent's rustflags after the child's.

**Exit contract.** 0 every requested target built, 1 a build failed, 2 could
not run. Lines 65-68 and 98 and 111 make a failure outrank a skip, so a skip
never clears one.

**Timeouts.** None. `dev.sh` wraps the whole thing in
`PODBOX_CHECK_STEP_TIMEOUT` (default 1800) at `scripts/dev.sh:229`.

**Cleanup.** None. It leaves `crates/podbox-interpose/target/dl-stub/` and the
two `.so` files in place, which is what the next step needs.

**Positive control.** The `diff -q` between declared and exported names
compares two independent sources; a build that dropped the version script
fails it.

**Failure control.** `TODO/gate.md:517-525` records the plants: a blinded map
input reads declared 0 against exported 112, and a bogus map global reads 113
against 112.

**TODO entries that own it.**
- `TODO/gate.md` T-1207, `Approach:` line 482, `Prove:` line 502,
  `**Done 2026-09-23.**` at line 510.
- `TODO/interpose.md` T-1312, `Prove:` at line 1400, `**Done 2026-09-21.**` at
  line 1403.
- `TODO/podvm.md` T-1327 through `TODO/packaging.md` T-1314, which names it in
  its `Approach:` at line 313.
- `TODO/complete.md` line 1371, which moved it before `cargo build`.

**Result file.** None. It writes no `experiments/results/` file; its
assertions are its stdout, which `scripts/dev.sh:220` prints.

**Verdict: RUST-TOOL.** This is the one build step in the group that a Rust
binary can replace outright, because almost everything it does is
`std::process::Command` plus reading two text files and one symbol table.

### Conversion plan

- **Assertions as triples today.** (DT_NEEDED) precondition: the object built
  for a target; action: `readelf -dW`; expected: the `NEEDED` list names
  `libc.so` for a musl target and `libc.so.6` for a gnu one, as an exact
  word. (Version ceiling) precondition: the object built; action:
  `readelf -VW`; expected: the newest `GLIBC_x.y` need is at or below 2.27.
  (`gettid`) precondition: the object built; action: `nm -D`; expected: no
  uppercase `T` entry named `gettid`. (Exports) precondition: the object built
  and `interpose.map` parsed; action: compare the `T` set against the
  `global:` block; expected: set equality, both directions, and no
  `rust_eh_personality`. (Size) precondition: the object built; action:
  `metadata().len()`; expected: under `INTERPOSE_CEILING_BYTES`.
  (`libdl.so.2` needed) precondition: a gnu-target object; action:
  `readelf -dW`; expected: `libdl.so.2` in `NEEDED`.
- **Level: deployment.** Every assertion is about a linked artefact.
- **Target crate and binary.** New crate `crates/podbox-interpose-build`, one
  `[[bin]]` named `podbox-interpose-build`.
- **CLI surface.**
  `podbox-interpose-build [--targets <list>] [--json]`
  `podbox-interpose-build --verify <so-path> --map <path> --target <triple>`
  where `--verify` runs the six assertions against an object that already
  exists and does not build anything.
  Exit codes keep 0, 1 and 2, because `scripts/dev.sh:195` and `:220` read
  them and `TODO/gate.md:504` names exit 1 in the `Prove`.
- **The part that stays shell, and why.** Two pieces of this script need a
  host toolchain Rust cannot replace:
  - `-C linker=$ROOT/scripts/zig-cc.sh` for the musl arm. The header at lines
    31-38 records the measured reason: `rustc` passes `-lgcc_s` even under
    `panic = "abort"`, and `musl-tools` ships no musl-linked `libgcc_s.so.1`.
    `zig cc` carries its own `compiler-rt`. A Rust binary can *invoke*
    `scripts/zig-cc.sh` as a linker path, which is what it should do; it
    cannot replace zig.
  - `cc -shared -nostdlib -Wl,--version-script=... dl-stub.S` at line 100, and
    the prepended stub through `scripts/gnu-link-stub.sh` at line 115. These are
    C assembly and linker-line work. The binary invokes them as `Command`s and
    asserts their results.
  So the tool replaces the shell control flow, the `readelf`/`nm` parsing and
  the two comparison arms, and shells out for the two link steps.
- **Fixtures.** `crates/podbox-interpose/interpose.map` and
  `crates/podbox-interpose/dl-stub.S` are already tracked. Nothing new.
- **Plant.** `TODO/gate.md:517-525` already names both plants and their
  expected messages. Re-point case 24 of `scripts/plant.sh:391` at the Rust
  source, and keep case 23a/23b at `scripts/plant.sh:382-385` reading
  `INTERPOSE_CEILING_BYTES`, which moves to the binary's source.
- **Proof commands.**
  `cargo run -p podbox-interpose-build --bin podbox-interpose-build`
  `cargo run -p podbox-interpose-build --bin podbox-interpose-build -- --verify crates/podbox-interpose/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so --map crates/podbox-interpose/interpose.map --target x86_64-unknown-linux-musl`
- **`exit 2` becomes.** Kept, and it is load-bearing. Lines 62-69 make an
  uninstalled target a 2 that a failure outranks, and `TODO/gate.md:504`
  requires the run to stay green when the interpose build is forced to SKIP.
  A binary is the only way to keep that third state alongside 0 and 1.
- **Result file.** None today and none after. Nothing to keep, supersede or
  move. `scripts/check-todo.py:808` names this script as `INTERPOSE_BUILD` and
  `:810` greps it for `^INTERPOSE_CEILING_BYTES=(\d+)$`; that path must move in
  the same change as the crate.

**Callers to update with it.** `scripts/dev.sh:85`, `:195` and `:220`;
`.github/workflows/gate.yml:99`; `.github/workflows/nightly.yml:84`;
`TODO/gate.md:395` and `:482`.

---

## Group 10 findings

### Contradictions between a script and the current source

1. **`145-podvm-parity.sh` asserts a string that has moved.** Clause 1 checks
   that `podvm --help` names `run`, `exec`, `podvm` and `--podbox-tier`.
   `crates/podbox-cli/src/man.rs:37` reads `crate::names::ALIASES` when
   building the man pages, and `names.rs:32` is
   `&["docker", "podman", "podvm"]`. The help text is now assembled from those
   tables rather than held as one literal, so the `grep` loop in the script
   tests a rendered string that the crate composes. Not a defect; a reason the
   assertion belongs in the crate.

2. **`386-podssh-partial.sh` asserts two strings the source no longer has.**
   The script wants `machine relay` to answer `not implemented yet`
   (line 77) and `machine ssh` to answer `no machine guest carries an SSH
   server` (line 74). Neither string is in `crates/podbox-cli/src/` today:
   `grep -F` for `no machine guest carries an SSH server` over `crates/`
   returns 0 hits. `machine relay` now reaches
   `crates/podbox-cli/src/machine.rs:52`, which prints
   `no such member` and returns `EXIT_RUNTIME_ERROR`. The script's saved result
   shows `ok machine unknown member` from a 2026-09-28 build, so the message
   changed after the reading. The clause would go red today. This is a second
   reason the script is stale rather than merely duplicative.

3. **`162-tar-symlink-modes.sh` names a line that no longer holds.**
   `TODO/interpose.md:1446-1448` cites
   `crates/podbox-interpose/src/lib.rs:334` and `:989` for the three-argument
   `fchmodat` that was the defect. The file is 121,328 bytes and those
   declarations are now at `:356` and `:1038`, both carrying four arguments.
   The entry's citation is stale even though its conclusion is correct. Flagged
   to `TODO/interpose.md`, not to the script.

4. **`310-session-startup.sh` counts words from a path that has moved.**
   Line 98 loops over `TODO/PROGRESS.md AGENTS.md` and writes
   `docs/AGENTS.md 2042 words` into the saved result, because the script prints
   `printf '%-22s' "$f"` over its own relative string while reading
   `$REPO/$f`. The printed name and the read path disagree, and `TODO/RULES.md`
   requires structured output read by field name.

### Dead scripts

- `experiments/162-tar-symlink-modes.sh`. The defect is fixed, the fix has a
  named unit test, and the entry names the test as the guard.
- `experiments/386-podssh-partial.sh`. Two of its four clauses are commands the
  gate already runs, its third duplicates existing unit tests, and its number is
  retired.

### Scripts with no owning entry

- `experiments/386-podssh-partial.sh`. The only live reference is
  `TODO/SESSION-SUMMARY-2026-09-28-SSH.md:22`. `TODO/podssh.md` T-1401 and
  T-1404, the entries its own header names, name other drivers in their
  `Prove:` lines.
- `experiments/385-kvm-open.sh`. `TODO/milestones.md` T-1112's `Prove:` names
  `369`, `370` and `392`. The live references are
  `TODO/SESSION-SUMMARY-2026-09-28-SSH.md:25`, `docs/history/README.md:60` and
  `docs/history/audit-before-2026-09-30/TODO/milestones.txt:970`. The
  measurement has a result file and no entry's `Prove:`.
- `experiments/310-session-startup.sh` has `TODO/packaging.md` T-1313 quoting
  its numbers, but T-1313's `Prove:` is about `dev.sh` itself. The script is
  cited in the record rather than owned by an acceptance.

### Entries whose acceptance command is a script with no automated test

- `TODO/deps.md` T-0910. `Prove:` at line 618 is `./experiments/110-bloat-delta.sh
  baseline`. Nine further entries in the same file use `<area>` forms of the
  same script as their acceptance, at lines 67, 120, 162, 197, 246, 318, 370,
  426 and 532. No `cargo test` covers the ceiling or the delta. `TODO/deps.md:629`
  records the consequence: the workflow calls the script because "a value in
  two places drifts".
- `TODO/image.md` T-0213. `Prove:` at line 1379 is the script, and
  `scripts/check-todo.py` reads `experiments/results/insecure-registry.txt`
  through no check. The unit tests in `transport.rs:324` cover policy
  resolution; the seven clauses have no automated equivalent.
- `TODO/podvm.md` T-1302. `Prove:` at line 288 names the script for the
  "drives every row from the flag" half. The routing half is unit-tested; the
  drive half is not.
- `TODO/milestones.md` T-1109. `Prove:` at line 578 is the script. Five of
  eleven clauses are unit-testable and none currently are.

### Other findings

- `experiments/357-tcg-profile.sh` has no `trap`. Line 33 creates
  `experiments/.sweep357-work` and only the *next* run removes it. Every other
  script in this group cleans up on exit.
- `experiments/210-store-concurrency.sh` and `experiments/280-insecure-registry.sh`
  pin their *images* by digest but their *source payloads* by tag
  (`alpine:latest` at line 34 and `archlinux:latest` at line 302). The saved
  result files carry no digest for them, so the readings apply to whatever
  those tags held on the recorded day.
- `experiments/357-tcg-profile.sh` line 71 and `experiments/386-podssh-partial.sh`
  lines 86-87 write to `/out`, a mount path outside the repository. The
  386 script also copies `Cargo.lock` there. `TODO/milestones.md:883` names the
  private-path half of the number-reuse finding, and both files are named.

---

## Group 10 — entries read

Read in full, with the section read.

1. `TODO/deps.md` T-0910, lines 593-645. Owns `110-bloat-delta.sh`. Its
   `**Done 2026-09-08.**` records the ceiling's move into `CEILING_BYTES` and
   names check 17 as the guard against a second copy.
2. `TODO/image.md` T-0210, lines 931-1010. Owns `210-store-concurrency.sh`.
   `Decision:` line 966 is the ruling that keeps it a shell experiment.
3. `TODO/image.md` T-0213, lines 1319-1400. Owns `280-insecure-registry.sh`.
   Records the proxy-405 defect that only a real socket found.
4. `TODO/image.md` T-1321, lines 1995-2110. Cites `210`'s `verify()` shell
   loop as the starting shape for the product verb `podbox verify`.
5. `TODO/milestones.md` T-1109, lines 552-620. Owns
   `250-negative-tests.sh`. `Decision:` line 574 puts negative tests in
   `experiments/`, not a separate suite.
6. `TODO/milestones.md` T-1112, lines 848-894. The entry `385-kvm-open.sh`
   serves. Its `Prove:` names `392`, not `385`, and lines 881-886 record the
   number-reuse finding against `386`.
7. `TODO/interpose.md` T-0711, lines 1040-1200. Owns
   `106-interpose-identity.sh`. Records the errno-by-call table and the
   `wall_errnos_carry_their_names` unit test.
8. `TODO/interpose.md` T-1311, lines 1423-1490. Owns
   `162-tar-symlink-modes.sh`. Names `tests::fchmodat_forwards_flags` as the
   guard and the script as the end-to-end confirmation.
9. `TODO/interpose.md` T-1312, lines 1360-1420. Names
   `scripts/build-interpose.sh` in its `Prove:` at line 1400 and records the
   `dl-stub` and `gettid` fixes.
10. `TODO/podvm.md` T-1301, lines 45-180. Owns `357-tcg-profile.sh`. Records
    the three-design weighing and the `RLIMIT_FSIZE` constant defect.
11. `TODO/podvm.md` T-1302, lines 212-310. Owns `145-podvm-parity.sh`. Records
    the 19 driven rows and the rejected whitespace-splitting alternative.
12. `TODO/gate.md` T-1207, lines 470-560. Owns the export-set and size
    assertions inside `scripts/build-interpose.sh`, with both plant shapes.
13. `TODO/gate.md` T-1212, lines 925-1010. Names `280-insecure-registry.sh`
    and records its seven green clauses on the base lane.
14. `TODO/packaging.md` T-1313, lines 236-330. Cites
    `310-session-startup.sh`'s five numbers and names the script at line 278.
15. `TODO/podssh.md`, the whole page, 208 lines. T-1401 and T-1404 are the
    entries `386-podssh-partial.sh` claims; neither names it. T-1406's
    `Prove:` at line 188 names `cargo test -p podbox-ssh` directly, which is
    clause 2 of the script with no script between.
16. `TODO/SESSION-SUMMARY-2026-09-28-SSH.md`, 42 lines. The only live owner of
    both `385` and `386`.
17. `experiments/README.md`, 63 lines. The exit-code table and the
    "do not reuse a number" rule.
18. `scripts/common/exit-codes.sh`, 54 lines. Read because four of the group's
    scripts source it and its header names the measured failure that produced
    it.
19. `scripts/plant.sh`, the case list and the four guards, lines 1-180 and
    `grep -n 'case_plant'`. Read for the plant cases 17a/17b/17c, 23a/23b, 24
    and 25 that these scripts' conversions must not break.
20. `scripts/check-todo.py`, checks 17, 20 and 23, lines 220-231, 340-420,
    460-470, 630-680 and 807-865. Read because it names three of this group's
    scripts as the homes of values the gate reads.