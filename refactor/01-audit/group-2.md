# Group 2 audit

Scope: the twelve scripts named in `## Group 2` of
[assignment-map.md](../00-orientation/assignment-map.md). Each file was read in
full. Each owning entry was read in full. Each cited result file was read.

## Group 2 summary

| Verdict | Count | Scripts |
| --- | --- | --- |
| DELETE | 0 | - |
| RUST-TEST | 1 | `160-store-gc.sh` (its clause 5 splits out as a separate `contain.rs` test) |
| RUST-TOOL | 6 | `153-store-lock-race.sh`, `251-tty-refusal-no-ptmx.sh`, `360-perf-harness.sh`, `366-namespace-base.sh`, `scripts/build-state.py`, `scripts/release-licenses.py` |
| KEEP-SHELL | 4 | `371-validationos-stream.sh`, `384-windows-lane-v6.sh`, `387-mux-two-client.sh`, `396-audit-linux.sh` |
| SPLIT | 1 | `389-remote-ssh.sh` (clause 2 to `cargo test`, clauses 3 to 5 stay shell) |

Twelve scripts, 2929 lines. `wc -l` over the twelve paths returns `2929 total`,
which matches the map.

No script earns DELETE. Every measurement in this group is either live proof of
a shipped path or live proof of a host capability podbox does not own, and none
has a disproved premise. The closest candidate is `153-store-lock-race.sh`: its
own header and T-0215 both record that the cause it was built to find was never
found. Its subjects are the plants, and they still earn a keep.

The three most important findings.

1. **`360-perf-harness.sh` is the only Group 2 script whose budget is read by a
   gate check.** `scripts/check-todo.py:1382` `check_perf_budget` reads
   `experiments/perf-ceilings.tsv` and three result files, and
   `scripts/plant.sh:490` plants case 30 against `perf-lane.txt`. Deleting the
   harness without moving check 30 makes the gate read three stale files
   forever.
2. **T-1338's Done paragraph quotes a TCG guest figure that no saved result
   carries.** `TODO/gate.md:1430-1431` reads "KVM guest boot 0.936 s against TCG
   1.869 s". `experiments/results/perf-kvm.txt` reads `0.935`. The number
   `1.869` appears nowhere under `experiments/results/`; the lane file records
   `guest.tcg.boot ... - s wall could-not-run` and the note "no
   qemu-system-x86_64 on PATH". The Done text asserts a comparison its own
   result file cannot support.
3. **Three of twelve Group 2 scripts are named by no `TODO/*.md` entry at
   all.** `grep -n '396-audit-linux\|build-state.py\|release-licenses' TODO/*.md`
   returns nothing. `scripts/build-state.py` is the build freshness engine that
   `scripts/dev.sh` calls seven times (`dev.sh:54`, `:90`, `:102`, `:163`,
   `:196`, `:199`, `:264`), and `scripts/release-licenses.py` is a release
   packaging step called by `scripts/package-ssh.sh:41`. Each is referenced by a
   module docstring and by another script, never by an entry `Prove`.

## Group 2 - experiments/153-store-lock-race.sh

**What it measures.** Whether the store lock race that made
`cargo test --workspace` fail intermittently is reachable by a podbox process
or only by a test harness. Fifteen clauses: one instrument positive control
(clause 0), the subject (clause 1), five subtractive controls (2, 3, 4, 5, 6),
one filesystem variation (7), two source mutations that amplify or remove a
condition (8, 9, 11), and four plants (10, 12, 13, 14).

- Pinned inputs: none beyond the workspace. It runs
  `cargo test --workspace`, compiled once with `--no-run` before any clause.
- Host assumptions: `cargo` on PATH, a workspace at `$REPO/Cargo.toml`,
  `mktemp`, `stat -f -c %T`, `git`, and a writable `/dev/shm` for clause 7.
  `TMPDIR` moves the store's scratch because `std::env::temp_dir()` reads it.
- Exit contract: 0 ran with no subject failure, 1 a subject run failed or an
  instrument/control plant stayed green, 2 no cargo or no workspace or the test
  binaries would not compile (`153-store-lock-race.sh:111-157`).
- Timeouts: none. `cargo test` runs unbounded. `RUNS` defaults to 12 subject
  runs and `CONTROL_RUNS` defaults to `RUNS` (`:89-90`).
- Cleanup: failing run logs are kept in `$WORK/logs` (`:96-97`); mutated source
  is restored by a `trap` on `EXIT HUP INT TERM` set before the mutation
  (`:339-340`) and again at `:392`, `:441`, `:494`, `:559`.
- Positive control: clause 0, `the_t_0215_instrument_sees_a_lock_that_is_held`
  at `crates/podbox-image/src/store.rs:1629`. It prints the `/proc/locks` row.
- Failure control: clause 10 guts the shed hook and asserts its test goes red;
  clause 12 deletes `LOCK_UN` and asserts the regression test goes red; clause
  14 makes the payload exemption unconditional and asserts the guard goes red.

**TODO entries that own it.** `TODO/image.md` T-0215 (named in the header
comment and in the `Prove` at line 1655). Secondary: `TODO/image.md` T-1310
(names the same 16-slot pool in the same binary).

**Result files it wrote.** `experiments/results/store-lock-race.txt`, 447
lines, dated `2026-09-12T10:04:51Z`, commit `d29305b`, tree printed as
modified.

**Verdict: RUST-TOOL.**

It is not a test. It re-runs a whole suite N times, mutates source files, and
prints a rate. A `cargo test` cannot do that. It is a durable investigation
harness with a per-host rate, and the rate is the only thing it produces that
Rust cannot assert.

### Conversion plan

**Assertions the script makes today.**

- A. Precondition: `cargo test -p podbox-image the_t_0215_instrument_sees_a_lock_that_is_held`
  passes. Action: none. Expected: exit 0, and its output carries a
  `FLOCK ADVISORY` row for the probe inode. Without it every "none" below is
  uninterpretable.
- B. Precondition: the unmutated workspace builds. Action: run
  `cargo test --workspace` twelve times, twice (passes A and B). Expected: both
  passes read 0 failures. This is the subject.
- C. Precondition: B holds. Action: run `cargo test --workspace` with
  `--skip a_fork_while_the_lock_is_held_does_not_extend_it --skip
  a_spawned_process_does_not_inherit_the_lock --skip probe_cache:: --skip
  naming_the_registry_insecure_gets_past_the_policy`. Expected: 0 failures in
  both passes, which rules the fork out as necessary.
- D. Precondition: `crates/podbox-image/src/store.rs` contains exactly one
  whole-line `            let _ = sys::flock(self.fd, sys::LOCK_UN);`. Action:
  delete that line, rebuild. Expected:
  `cargo test -p podbox-image releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives`
  exits non-zero. Restore, rebuild.
- E. Precondition: as D. Action: replace the line
  `        if !self.handed.load(std::sync::atomic::Ordering::Acquire) {` with
  `        if true {`. Expected:
  `cargo test -p podbox-image a_lock_handed_to_the_payload_outlives_this_process_dropping_it`
  exits non-zero. Restore, rebuild.
- F. Precondition: as D, and `PODBOX_T0215_CAPTURE=1` is read by the
  instrument. Action: run `cargo test --workspace` with the release deleted and
  the capture on, thirty times. Expected: every run fails and every capture
  names either a `/proc/locks` row or its absence plus a successful retry on the
  same descriptor.

**Rust level.** Fault tests, per `docs/conventions/code.md` "Use fault tests for
conditions a real service cannot produce on demand." Clauses D, E, and F
inject a defect into the product to prove a test watches it. That is the fault
kind. Clauses A through C measure a real service, so B and C are integration
proof and D through F are its plants.

**Target.** New binary `podbox-lockrace` in a new crate
`crates/podbox-lockrace` with `[[bin]] name = "podbox-lockrace"`. One crate
because the harness needs `cargo` and the workspace together and nothing in the
product depends on it.

**CLI surface.**

```
podbox-lockrace --clauses "0 1 6 12 13 14" --runs 30 --control-runs 30
                --repo PATH --alt-tmp /dev/shm
                --out experiments/results/store-lock-race.txt
```

Exit 0 ran with no subject failure, 1 a subject run failed or a plant stayed
green, 2 no cargo, no workspace, or the binaries would not build.

**Fixtures.** None new. Every skip name and every mutation anchor is read from
the current tree at run time; that is the design and it must survive the
conversion. The four skip names are the forking paths of the
`podbox-image` test binary, currently `store.rs:2019`,
`store.rs:2114`, `probe_cache::` and `pull.rs:891`. Re-read them in the binary
before every run rather than keeping a list, so a moved test is a stale-read
failure and not a silent zero.

**The plant the new tool must carry.** A `--self-test` mode that runs the
mutation machinery against a scratch copy of one file and asserts the anchor
matched exactly once. A mutation that matched zero times leaves the tree
correct and the clause prints `SKIP`, which is what `scripts/plant.sh` guard 1
is about. Without this the tool's own SKIP path is unproven.

**Proof command.**

```
PODBOX_RACE_RUNS=30 cargo run -p podbox-lockrace -- --clauses "0 1 6 12 13 14"
```

No feature flags. Target triple: none; it must run on the host, so it does not
want the musl target. `experiments/README.md:36` requires this shape, so update
that row.

**What `exit 2` becomes.** The binary returns 2 from `main` with a report that
says which precondition was absent. A caller reading the process status without
a pipe gets 2, which is the existing contract. Under `cargo test` the same
condition is a `#[ignore]`d test with a printed reason, or a plain `panic!`
with the reason; the rule is that the reason is named, and it is never a skip
that reads as green. `docs/conventions/code.md` says "A skipped test proves
nothing about its subject."

**Result file.** Kept. `experiments/results/store-lock-race.txt` is the run
T-0215's table quotes. Nothing supersedes it.

### Dead weight to remove in the conversion

Clauses 2, 3, 4, 5, 7, 8, 9, and 11 were built to find a cause that the Done
record says was never found. `TODO/image.md` T-0215's `Approach` reads
"**Every candidate about which descriptors a CHILD holds was closed and none of
them moved the rate.**" They stay in the Rust tool only if a future race chase
needs them. The recommendation is to keep clause 6 (the fork control, named in
every T-0215 citation) and clauses 12, 13, and 14 (the three plants the `Prove`
runs), and to drop 2 through 5, 7, 8, 9, and 11. The header comment at
`:37-43` already says clause 7 "has already ruled it out".

## Group 2 - experiments/160-store-gc.sh

**What it measures.** Five clauses against a built `podbox` binary and a live
registry. A shared blob survives removing one of two tags. `rmi` under a holder
refuses with 125 and names "in use". `image prune -af` skips by name and keeps
the image. Once the holder goes, `rmi` succeeds and frees every blob. A blob
path that resolves outside the store is refused and a canary survives.

- Pinned inputs: `public.ecr.aws/docker/library/alpine:latest` by default,
  overridable with `PODBOX_TEST_IMAGE` (`:27`). The image is deliberately not
  Docker Hub, for the quota reason in the header.
- Host assumptions: `PODBOX_BIN` default
  `target/x86_64-unknown-linux-musl/release/podbox`; `flock(1)` on PATH
  (`:39-43`); `timeout`, `find`, `head`.
- Exit contract: 0 every clause held, 1 one did not, 2 no binary, no `flock`,
  or a pull failed (`:34-67`, `:117-120`, `:209`).
- Timeouts: `timeout 600` on each pull, `sleep 25` bounds the holder.
- Cleanup: `trap` on `EXIT INT TERM` kills the holder and removes `$WORK`
  (`:31`).
- Positive control: clause 1 and 2 run before the holder exists, so the
  store is proven working before the refusal is driven.
- Failure control: clause 5 replaces a real blob with a symlink to a canary.
  The header records why a stray symlink would prove nothing (`:169-173`).

**TODO entries that own it.** `TODO/image.md` T-0204 (the header names it;
the Done text at line 318 says it "exits 0 and drives all five clauses").
Secondary: `TODO/image.md` T-0206 (moved it off Docker Hub for quota).

**Result files it wrote.** `experiments/results/store-gc.txt`, dated
`2026-09-08T16:54:55Z`, kernel `6.18.44-fc-v24`.

**Verdict: RUST-TEST with a SPLIT clause 5.**

Four of the five clauses are already unit tests in the current tree. The store
test `an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune`
(`crates/podbox-image/src/store.rs:1976`) asserts clauses 2, 3, and 4 through
the shipping functions, and `a_shared_blob_survives_removing_one_of_the_two_images_that_reach_it`
(`store.rs:1950`) asserts clauses 1 and 4. The script's clauses 2, 3, and 4 are
the shipped CLI text around those calls: the exit code and the exact wording.

### Conversion plan for clauses 1 to 4

**Assertions today.**

- A. Precondition: a store holds one record `alpine:latest`. Action: tag the
  same digest `podbox-gc-probe:v1`, then `rmi podbox-gc-probe:v1`. Expected:
  blob count unchanged.
- B. Precondition: `store.hold(&record)` returned a live `Lock`. Action:
  `store.remove("alpine:latest")`, then `store.prune(true)`. Expected: the
  remove errors with a message containing "in use"; `pruned.skipped` equals
  `["alpine:latest"]`.
- C. Precondition: as B. Action: drop the `Lock`. Expected: `in_use` is false
  and `remove` succeeds.

**Rust level.** Pure. `docs/conventions/code.md`: "Use pure tests for
deterministic logic." These are deterministic given a store directory.

**Target.** `crates/podbox-image/src/store.rs`, inside the existing
`mod tests`. The three tests already exist there. What does not exist is the
CLI layer.

**The CLI half that the script adds.** `crates/podbox-cli/src/images.rs:977`
prints `skipped: {name} is in use by a running container` and `:981` prints the
`referenced by container` line. Add to `crates/podbox-cli/src/images.rs`'s
`mod tests` a test that formats each refusal string and asserts the exit code
it returns is `podbox_probe::exit::EXIT_RUNTIME_ERROR` (125, at
`crates/podbox-probe/src/exit.rs:34`) and that the message names the container.

**Fixtures.** `scratch("inuse")` and `record(...)`, the helpers already in
`store.rs`'s test module. Reuse; do not add a second.

**Plant.** Take out the `if !hold` guard in `Store::delete`
(`store.rs:830`) and assert `an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune`
goes red. `scripts/plant.sh` needs a case for it. Today that test has no plant
in `scripts/plant.sh`; grep for `in use` in `scripts/plant.sh` returns nothing.

**Proof command.**

```
cargo test -p podbox-image an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune
cargo test -p podbox-image a_shared_blob_survives_removing_one_of_the_two_images_that_reach_it
cargo test -p podbox-cli images
```

No feature flags, no target triple.

**`exit 2` becomes.** A missing binary or a failed pull is not a Rust concern.
The store tests need no registry. The script's `exit 2` disappears with the
script.

**Result file.** `experiments/results/store-gc.txt` is kept as history. Its
measured values stay.

### Conversion plan for clause 5 (the SPLIT)

**Assertion today.** Precondition: a store with one blob whose path is a symlink
to a file outside the store. Action: `image prune -af`. Expected: the canary
survives, and the output names "outside the store".

**Rust level.** Pure, but it needs a real symlink, so it is a filesystem test
rather than a logic test. It still belongs in `mod tests` because
`crates/podbox-image/src/contain.rs` has its own `mod tests` at line 95 and the
check is pure path resolution.

**Target.** `crates/podbox-image/src/contain.rs`, `mod tests`. Add
`a_symlinked_blob_resolving_outside_the_store_is_refused`: build a temp root, a
canary outside it, a symlink inside pointing at the canary, assert
`within(root, link)` is `Err`, and assert the canary file is still there.

**Plant.** `scripts/plant.sh` needs a case. It cannot live in
`scripts/plant.sh` as a source mutation today because
`experiments/394-ssh-package.sh` and `scripts/plant.sh` do not touch
`contain.rs`. Add the case to `scripts/plant.sh`'s file list and mutate
`contain::within` to return the candidate unresolved; assert the new test name
appears in the red output.

**Proof command.** `cargo test -p podbox-image contain::tests`.

**Result file.** Clause 5's reading stays in
`experiments/results/store-gc.txt`. The result line
`canary_survived yes` is a measurement on 2026-09-08 and stays historical.

## Group 2 - experiments/251-tty-refusal-no-ptmx.sh

**What it measures.** That `podbox run -t` refuses by name where `/dev/ptmx` is
unusable. The script builds the machine where that is true: a private user and
mount namespace in which an empty directory is bound over `/dev/pts`, leaving
the `/dev/ptmx` symlink dangling. It then drives the refusal through the
shipped binary, then runs `250-negative-tests.sh` under the same cover.

- Pinned inputs: `public.ecr.aws/docker/library/alpine:3.20` by tag, not by
  digest (`:33`).
- Host assumptions: `jq`, `timeout`, `unshare` with `--user --map-root-user
  --mount`, `mount --bind`, and `/dev/ptmx` being a symlink into `/dev/pts` or
  a directory (`:112-135`). It sources `scripts/common/exit-codes.sh` and reads
  the exit code table out of the binary rather than writing numbers down
  (`:50-55`).
- Exit contract: 0 refused and named its reason, 1 the wiring is broken, 2 it
  could not run. It has an inner and an outer half; the outer writes
  `$WORK/focused` and the inner is re-executed under `unshare`.
- Timeouts: `timeout 120` on the probe, `timeout 600` on the pull, `timeout
  300` on the focused run, `timeout 2400` on 250, `timeout 2700` on the covered
  self re-run. Exit 124 is mapped to 2 (`:267-270`).
- Cleanup: the outer half traps `rm -rf "$WORK"`; the inner half does not, and
  the header explains why (`:61-64`).
- Positive control: the uncovered probe must answer `true` before the cover is
  built (`:100-102`); if it answers `false` the machine is already the target
  and it drives with no cover.
- Failure control: after binding, it asserts the mount table carries the cover
  (`:150-157`), then asserts the covered probe reads `false`. A cover that did
  not land is `exit 2`, not a pass.

**TODO entries that own it.** `TODO/milestones.md` T-1109 (the header names it
as "condition 1"; the Done text at `:580-598` says 250 exits 0 under the cover
this script builds). Secondary: `TODO/enter.md` T-0503 (the refusal it drives)
and `TODO/enter.md` T-1317 (the ordering that put the ptmx refusal ahead of the
chroot gate).

**Result files it wrote.** `experiments/results/tty-refusal-no-ptmx.txt`, dated
`2026-09-25T09:07:07Z`, binary `podbox 0.1.0-beta.7`, verdict "the refusal
fired where ptmx is unusable, and named its reason." It also renewed
`experiments/results/negative-tests.txt`.

**Verdict: RUST-TOOL.**

The predicate is already unit-tested. What is missing is the machine. A Rust
test cannot bind-mount over `/dev/pts` without privileges a normal test does
not have, and `mount --bind` is a host capability. The refusal wiring itself is
already covered by unit tests, so this stays a driver tool, not a test.

### Conversion plan

**Assertion today.** Precondition: `podbox probe --json` answers
`.ptmx.usable` `true` on the host. Action: `unshare --user --map-root-user
--mount`, `mount --bind $empty /dev/pts`, re-read `podbox probe --json`,
then `podbox run -t --rm IMAGE true`. Expected: the covered probe reads
`false`; the run exits `PODBOX_EXIT_RUNTIME_ERROR` (125) and its stderr
contains `cannot allocate a pty`.

**Rust level.** Deployment proof, per `docs/conventions/code.md` "Use
integration and deployment proof for the actual default path." The predicate
`ptmx_usable` is a pure unit test at `crates/podbox-probe/src/probes.rs:1618`
and does not prove the CLI refusal fires. The refusal call site is
`crates/podbox-cli/src/run.rs:1123` and again `crates/podbox-cli/src/exec.rs:522`.

**Target.** New binary `podbox-ptmx-cover` in a new crate
`crates/podbox-ptmx-cover`, or a second `[[bin]]` in `podbox-lockrace` if the
two tools are landed together. Prefer a separate crate: this tool needs the
built `podbox` binary on PATH and that dependency is not the lock race's.

**CLI surface.**

```
podbox-ptmx-cover --bin PATH --image IMAGE --out FILE --with-negative-suite
```

It runs the cover, prints the conditions block, the cover, the focused verdict,
and optionally the negative suite. Exit 0 refused and named, 1 broken, 2 could
not run.

**Fixtures.** Reuse `scripts/common/exit-codes.sh`'s table by reading
`podbox system info --format '{{json .ExitCodes}}'` through the Rust
`podbox_cli` exit-code reader rather than shelling to `jq`. The shell helper
exists to stop a second declaration of a number; the Rust side must not
reintroduce it.

**Plant the tool needs.** A `--dry-run` mode that reports what it would cover
and what `readlink /dev/ptmx` says, without binding. A cover that silently
fails to bind must read as a failure; the shell script proves this by reading
`/proc/mounts` and the Rust tool must do the same through
`std::fs::read_to_string("/proc/mounts")`.

**Proof command.**

```
cargo run -p podbox-ptmx-cover -- --bin target/x86_64-unknown-linux-musl/release/podbox
```

No feature flags.

**`exit 2` becomes.** Exit 2 from `main` with the named reason: no `unshare`,
no `mount`, the probe answered neither true nor false, or the mount table does
not carry the cover.

**Result file.** `experiments/results/tty-refusal-no-ptmx.txt` is kept. It is
the reading T-1109 quotes.

## Group 2 - experiments/360-perf-harness.sh

**What it measures.** Wall time and peak RSS for every podbox verb, every
forced ladder rung, guest boot and command latency under TCG or KVM, and binary
size with PT_INTERP state. It writes ten-column TSV rows. It does not compare
against a budget; check 30 does.

- Pinned inputs: debian by digest
  `sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a`,
  alpine by digest
  `sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764`,
  kernel URL with `KERNEL_SHA256=6b58e5d779e...` (`:47-50`).
- Host assumptions: the binary, `timeout`, `curl`, `sha256sum`, `stat`, `awk`,
  `grep`, optionally `/usr/bin/time`, `cc`, `qemu-system-x86_64`, `cpio`, and
  `/dev/kvm` for the kvm shape (`:92-101`, `:287-295`).
- Exit contract: 0 every metric ran, 1 a metric failed, 2 the shape could not
  run (`:92-96`, `:397`).
- Timeouts: `MTIMEOUT` per call, 600 for pulls, 120 elsewhere, `BOOT_TIMEOUT`
  600 (`:53`, `:110`, `:145`, `:150`).
- Cleanup: `rm -rf "$WORK"` at entry (`:41`). No trap. Nothing is removed at
  exit, so a failed drive leaves `experiments/.sweep360-work-$SHAPE` behind.
- Positive control: the guest metric asserts the pinned kernel's sha256 before
  booting (`:315-318`). A wrong kernel is a refusal to measure, not a number.
- Failure control: a missing tool, a hung verb at rc 124, and a rung at 125
  each record `could-not-run` or `refused` rather than a number (`:246-253`).

**TODO entries that own it.** `TODO/gate.md` T-1338 (header and `Prove`).
Secondary: `TODO/gate.md` T-1340 (the run-decay cause) and `TODO/gate.md`
T-1341 (the `run.kept` row).

**Result files it wrote.** `experiments/results/perf-lane.txt` (commit
`048e2a0`, binary `0.1.0-beta.7`, verdict `PERF lane SERVED`) and
`experiments/results/perf-kvm.txt` (commit `unknown`, qemu 11.1.1, verdict
`PERF kvm SERVED`). `experiments/results/perf-seeds.tsv` holds transcribed
seeds and is not re-measured.

**Verdict: RUST-TOOL.**

This is a durable gate runner. Check 30 is `scripts/check-todo.py:1382`
`check_perf_budget`, and it reads the harness output.
`scripts/plant.sh:490` plants case 30 against
`experiments/results/perf-lane.txt`. It cannot become a test.

### Conversion plan

**Assertions the script makes today.** It makes no assertion in the
precondition/action/expected sense. It emits rows and lets check 30 compare.
Stated that way:

- A. Precondition: the binary exists and is executable. Action: run every verb
  under `timeout`. Expected: each verb produces exactly one TSV row pair
  (`metric` and `metric.rss`) with a state of `ok`, `failed`, `refused`, or
  `could-not-run`.
- B. Precondition: the store is cold and the debian pull succeeds. Action:
  `podbox pull DEBIAN`. Expected: exit 0. A failure here is `exit 2`, not a
  failed metric (`:147`).
- C. Precondition: qemu, cpio, an extracted alpine rootfs, and a fetched kernel
  whose sha256 matches the pin. Action: boot under `-accel tcg` or `-accel
  kvm`. Expected: a `VMR-PERF-READY` marker, then `VMR-PERF-CMD`, then
  `VMR-PERF-DONE` inside `BOOT_TIMEOUT`.

**Rust level.** Deployment proof for A through C. A wall-time reading is not a
unit test and cannot be: it is a measurement on a named host. The comparison
is what is pure, and it already lives in check 30.

**Target.** New binary `podbox-perf` in a new crate `crates/podbox-perf`, or a
`[[bin]]` in a new `crates/podbox-perf`. It stays out of the product crates: it
must not pull `ureq`, `tar`, `flate2`, `ruzstd`, `sha2` and the rest into any
binary that ships.

**CLI surface.**

```
podbox-perf --shape lane|kvm --bin PATH --repo PATH
            --out experiments/results/perf-SHAPE.txt
            --debian DIGEST --alpine DIGEST --kernel-url URL --kernel-sha256 HEX
            --metric-timeout 120 --boot-timeout 600
```

Print the same conditions block and the same ten TSV columns. Exit 0 every
metric ran, 1 a metric failed, 2 the shape could not run.

**Fixtures.** `experiments/lib/podvm-guest.sh` provides `podvm_base`,
`podvm_extras`, and `podvm_concat`, which the shell sources at `:321`. Port
these three to Rust in the new crate. They are shell because they build a cpio
from a directory; the Rust version shells to `cpio` the same way and keeps the
marker protocol (`VMR-PERF-READY`) unchanged so the committed
`experiments/results/perf-kvm.txt` stays comparable.

The static-fixture rung (`:197-212`) builds `hello.c` with `cc -O2
-static-pie` and imports the tar as `perfstatic:1`. Port that to a Rust
`std::process::Command` over `cc` with the same flags.

**Plant the new tool needs.** `--emit-fixture` writes a results file with one
deliberately absurd value for a named metric and exits 0. Check 30 must go red
naming that metric. That is `scripts/plant.sh` case 30's role today, so the
conversion must keep `plant.sh` case 30 working by having it mutate the same
column (`run.repeat` at `scripts/plant.sh:491`) and keep the reading in a file
with the same shape.

**Proof command.**

```
cargo run -p podbox-perf -- --shape lane --bin target/x86_64-unknown-linux-musl/release/podbox
py scripts/check-todo.py
sh scripts/plant.sh
```

No feature flags. The tool runs on the host, so no target triple.

**`exit 2` becomes.** Exit 2 from `main` after writing the report, with the
metric names recorded as `could-not-run`, which is what the script already does
for the per-metric cases and what `experiments/README.md:11` defines. The
whole-shape `exit 2` (no binary at `:92`) is the one case with no metric to
attach to; the Rust tool writes a report naming the absent binary and exits 2.

**Result files.** Both kept. They are the readings check 30 compares, and
`perf-seeds.tsv` is compared beside them (`check-todo.py:1377-1379`).

**Contradiction to fix in the same change.** `TODO/gate.md:1430-1431` quotes
"KVM guest boot 0.936 s against TCG 1.869 s". The saved KVM row reads `0.935`
(`experiments/results/perf-kvm.txt`), and the saved lane file records
`guest.tcg.boot ... - s wall could-not-run` with the note "no
qemu-system-x86_64 on PATH". The `1.869` figure has no source in
`experiments/results/`. Correct the Done text in place; do not leave it, and do
not carry it into the new tool's documentation.

## Group 2 - experiments/366-namespace-base.sh

**What it measures.** Five clauses on a capable host. `probe` selects
`namespace`. A run enters it with an honest banner and no fallback line, and
`.EnteredRung` reads `namespace`. A file the payload writes to `/tmp` is
visible to the payload and absent from the host rootfs, with no leaked mount.
An image without `/tmp` falls back to chroot and names the cause. A detached
container starts through the same gate and its logs carry the payload output.

- Pinned inputs: alpine 3.20 by digest
  `sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc`.
- Host assumptions: `BIN="${PB_BIN:-/workspaces/pb365/podbox}"` and
  `WORK="${PB_WORK:-/tmp/pb365-work}"` (`:25-26`), `unshare -m true` succeeding
  (`:46`), and uid 0. The result file records `id 0:0`.
- Exit contract: 0 every clause matched, 1 one disagreed, 2 no binary, no
  `timeout`, `unshare` refused, pull failed, or extract failed.
- Timeouts: `timeout 120` on every podbox verb, `timeout 600` on the pull.
- Cleanup: the script removes `$STORE` at line 170 and removes containers and
  the imported test image. It has no trap, so a killed run leaves
  `/tmp/pb365-work` and its containers.
- Positive control: none in the script. It is driven by clause 1, which asks
  `probe` first and dumps `probe` output into the report when it disagrees.
- Failure control: none. ns-4 is the nearest: it stages an image with no `/tmp`
  and expects the fallback line.

**TODO entries that own it.** `TODO/enter.md` T-1339 (the `Prove` at `:740-747`
names it, and the Done text at `:762-764` quotes it). Secondary:
`experiments/365-namespace.sh`, the denied-host half, is T-1339's other arm.

**Result files it wrote.** `experiments/results/namespace-base.txt`, dated
`2026-09-26T12:54:56Z`, kernel `7.2.0-WSL2-STABLE`, binary
`0.1.0-beta.7`, verdict `NAMESPACE BASE SERVED`, `fail=0`.

**Verdict: RUST-TOOL.**

Every unit guard T-1339 lists already exists in
`crates/podbox-enter/src/lib.rs` and `crates/podbox-cli/src/system.rs`. What
only a drive can prove is that the mount the namespace rung makes is
invisible to the host. That needs a real host and a real binary.

### Conversion plan

**Assertions today.**

- A. Precondition: `unshare -m true` succeeds as root. Action:
  `podbox probe`. Expected: first line begins `namespace`.
- B. Precondition: alpine is pulled. Action:
  `podbox run --rm --name ns365 ALPINE /bin/echo base-hi`. Expected: exit 0,
  stdout holds `base-hi`, stderr holds `mode=namespace`,
  `namespaces: mount-only`, `network: host-shared`, and
  `this mode does NOT provide:`, and does NOT hold
  `entered chroot instead`.
- C. Precondition: B passed. Action:
  `podbox system info --format '{{.EnteredRung}}'`. Expected: `namespace`.
- D. Precondition: the alpine rootfs is extracted. Action: run a payload that
  writes `/tmp/ns365mark` and reads it back. Expected: stdout holds
  `mark365`; `$ROOTFS/tmp/ns365mark` does not exist on the host; `/proc/self/mounts`
  carries no line naming `$ROOTFS/tmp`.
- E. Precondition: a tar of `bin` and `lib` with no `/tmp` is imported as
  `notmp365:test`. Action: `podbox run --rm notmp365:test /bin/sh -c 'echo
  fallback-hi'`. Expected: exit 0, stdout holds `fallback-hi`, stderr holds
  `entered chroot instead` and `no /tmp mount point`.
- F. Precondition: alpine is pulled. Action: `podbox create --name ns365d`,
  `start`, `wait`. Expected: `podbox logs ns365d` holds `detached-hi`.

**Rust level.** Deployment proof for A through F. D is the only one that
cannot be asserted anywhere else: it reads the host's own mount table and rootfs
after the payload exits, which is outside the process under test.

**Target.** New binary `podbox-nsdrive` in a new crate `crates/podbox-nsdrive`.

**CLI surface.**

```
podbox-nsdrive --bin PATH --alpine DIGEST --store DIR --work DIR
               --out experiments/results/namespace-base.txt
```

Exit 0 every clause matched, 1 one disagreed, 2 no binary, `unshare` refused,
or the pull failed.

**Fixtures.** Build the no-`/tmp` image in Rust: `podbox extract`, copy `bin`
and `lib` into a scratch tree, `tar` them with `std::process::Command`, then
`podbox import`. `crates/podbox-extract` already has `tar` and `flate2`, so
the new crate should shell to the `podbox` binary rather than reimplement it.
The alpine applet symlinks (`sh -> /bin/busybox`) must be preserved; the
shell script's comment at `:128-130` records why, and the plant is that a
missing symlink reads as a missing shell.

**Plant the new tool needs.** `--with-mutations` deletes
`crates/podbox-enter/src/lib.rs:747`'s `/tmp` requirement, rebuilds, and asserts
ns-4 no longer prints `no /tmp mount point`. That is a source mutation and
belongs in `scripts/plant.sh`, not in the tool. The tool needs its own
positive control: assert `probe` answers `namespace` before any clause runs and
exit 2 naming it when it does not. Without it, a host that lost the namespace
leg makes every clause report a confusing failure instead of a skip.

**Proof command.**

```
cargo run -p podbox-nsdrive -- --bin target/x86_64-unknown-linux-musl/release/podbox
cargo test -p podbox-enter
cargo test -p podbox-cli system
```

No feature flags.

**`exit 2` becomes.** Exit 2 with the named absent condition: no binary,
`unshare -m true` refused, the pull failed, or `probe` does not select
`namespace`. The last one matters: the current script treats a capable host
that stopped being capable as a failed clause.

**Result file.** `experiments/results/namespace-base.txt` is kept.

**Contradiction to fix.** The script defaults `BIN` to
`/workspaces/pb365/podbox` and `WORK` to `/tmp/pb365-work`. The path names the
sibling script `365-namespace.sh`, not this one. The Rust tool takes both as
required arguments with no default.

## Group 2 - experiments/371-validationos-stream.sh

**What it measures.** That a lane with a sub-3 GB file-size ceiling can fetch
the ValidationOS disk out of a 2.46 GB ISO without ever writing a file the
ceiling forbids. It walks UDF structures with HTTP range requests, parses the
one VHDX extent, fetches it, and checks the magic, the digest, and that
`qemu-img` reads it.

- Pinned inputs: the Tayberry ISO URL, `ISO_LEN=2460880896`,
  `VHDX_LBA=1010`, `VHDX_LEN=910163968`, `VHDX_SHA256=063442aa9f...` (`:38-42`).
- Host assumptions: `curl`, `python3`, `prlimit`, `df`, `sha256sum`, and
  `qemu-img` or `scripts/common/bootstrap-env.sh qemu` (`:89-96`).
- Exit contract: 0 every clause matched, 1 a clause disagreed, 2 the lane could
  not run. It writes its report to `experiments/results/windows-365.txt`, a
  file name that belongs to a different experiment.
- Timeouts: `timeout 60` on HEAD, `timeout 300` per range, `timeout 900` on the
  extent fetch, `timeout 1200` on the bootstrap.
- Cleanup: `rm -rf "$WORK"` at entry (`:47`). No trap. The fetched VHDX at
  `$OUT` is left behind by design.
- Positive control: clause 1 checks HEAD length equals `ISO_LEN`; clauses 2
  through 4 assert each parsed structure's tag; clause 5 checks the magic and
  the digest. Every location is parsed, never hardcoded, apart from
  `VHDX_LBA` and `VHDX_LEN`.
- Failure control: the ceiling pre-flight at `:63-70` refuses before any write
  when the ceiling or the free space cannot hold the extent.

**TODO entries that own it.** `TODO/milestones.md` T-1112 (the entry's
`Source` line reads "experiments 369, 370, 371 and 392"). T-1112 is
`Status: partial`.

**Result files it wrote.** `experiments/results/windows-365.txt`, dated
`2026-09-27T02:02:05Z`, verdict `VOS STREAM HOLDS`. A second reading,
`experiments/results/vos-stream-371-ceiling.txt`, dated `2026-09-27T02:58:38Z`,
records the ceiling biting. `experiments/results/validationos-vhdx-host.txt`
records a host-side fetch by a script outside the tree.

**Verdict: KEEP-SHELL.**

It needs `curl` with byte-range support against an anonymous third-party
origin, a `RLIMIT_FSIZE` ceiling to test itself against, and `qemu-img` to
confirm the fetched extent is a readable disk. None of that is podbox. The UDF
parsing is the only part that is pure logic, and it is five assertions over
fixed byte layouts.

### Why KEEP-SHELL and not SPLIT

The UDF parsing could become a Rust unit test over four saved byte ranges
(8 KiB each). That test would prove the parser. It would not prove that the
lane ceiling permits the fetch, which is the actual claim in the question line.
The claim needs the ceiling. Keep the script.

### What a Rust rewrite would have to keep

If the operator wants the parsing retired anyway, the only Rust-safe part is a
UDF reader over fixtures, and it must say plainly that it does not measure the
ceiling. That is a different claim from the one T-1112 owns.

**Result file naming.** `:193` writes `experiments/results/windows-365.txt`.
Experiment 365 is `experiments/365-namespace.sh` and its result is
`experiments/results/namespace.txt`. The file name collides with no other
script today, but `experiments/README.md:5` says a number is permanent and a
citation of it must keep meaning what it meant. `windows-365.txt` cites 365
while carrying 371's reading.

**What `exit 2` becomes.** Nothing. The script keeps its three-state contract.

## Group 2 - experiments/384-windows-lane-v6.sh

**What it measures.** That the Windows wrapper runs two distinct jobs
concurrently, restores tracked executable modes, gives a caller the right
checkout, and leaves no kept job after job-specific collection.

- Pinned inputs: alpine 3.20 by digest `sha256:d9e853e...`, and the
  `experiments/352-ascii-output.sh` caller probe (`:15`, `:55`).
- Host assumptions: `wsl-toolkit`, `jq`, and `timeout` on the Windows host
  (`:17-22`); instance `podbox`; a clean job ledger before the drive
  (`:26-30`).
- Exit contract: 0 matched, 1 not, 2 could not run.
- Timeouts: `timeout 3m` on every `wsl-toolkit` call, `PODBOX_JOB_TIMEOUT=10m`,
  `timeout 12m` on each wrapper call.
- Cleanup: `trap 'rm -rf "$work"'`; the three jobs are collected by id
  (`:77-85`) and a final `gc --json` must read zero (`:86-91`).
- Positive control: the two job scripts print `job=one` and `job=two`, and the
  script asserts each log carries its own and not the other's (`:62-65`).
- Failure control: the caller probe must exit 2 and name
  `/work/target/x86_64-unknown-linux-musl/release/podbox` (`:64`). An image
  with no binary is the only way to get that code.

**TODO entries that own it.** `TODO/gate.md` T-1343 (`Prove` at `:1594` and
again at `:1634` under T-1344). Secondary: `TODO/gate.md` T-1345 and T-1346,
whose `Prove` runs `scripts/plant.sh` through the same wrapper.

**Result files it wrote.** `experiments/results/windows-lane-v6.txt`, dated
`2026-09-28T11:01:21Z`, `wsl-toolkit 6.0.0`, commit `c838759`, verdict
`matched`, `retained_before_gc=3`, `retained_after_gc=0`.

**Verdict: KEEP-SHELL.**

It drives `wsl-toolkit`, a Windows-only CLI that copies the checkout into a
container in a WSL distribution. No Rust crate in this tree wraps it, and
writing one would be a second implementation of a host tool. The job-id
collection contract is also `wsl-toolkit`'s.

### What it must keep

The caller probe depends on `experiments/352-ascii-output.sh`. Group 1 owns
that script. If 352 is deleted, clause 3 of 384 has no vehicle and the
`caller_path` assertion disappears. Name this in the 352 conversion.

**`exit 2` becomes.** Nothing.

## Group 2 - experiments/387-mux-two-client.sh

**What it measures.** That concurrent authenticated SSH sessions share one
`podbox-ssh` node connection against the live multiplexed relay.

- Pinned inputs: none local. It reaches the live relay at
  `https://tcp.ssh.relay.ajam.dev` and reads its version from `/health`
  (`:26-27`, `:51`).
- Host assumptions: `cd /work` (`:21`), so it only runs as a Windows job input;
  `bootstrap-env.sh zig openssh tools`; `ssh`, `sshd`, `ssh-keygen`, `curl`,
  `jq`; `/run/sshd` exists.
- Exit contract: 0 every clause held, 1 one disagreed, 2 the lane could not
  run.
- Timeouts: `timeout 25m` on the build, `-m 20` and `-m 25` on every curl, a
  60-iteration 2 s poll for the node to come online, `ConnectTimeout=10` on
  every ssh.
- Cleanup: `trap 'rm -rf "$work"'`; the node is killed and reaped at `:184-185`;
  the pair is stopped by API at `:176`.
- Positive control: clause 4 polls `/v1/status` until `online` is `true`. A node
  that never registers fails the clause.
- Failure control: clause 3 asserts the node token is refused 403 where the
  connect token is accepted 200, and clause 7 greps the committed log for each
  of the three tokens and fails on a match.

**TODO entries that own it.** `TODO/podssh.md` T-1403 (`Prove` at `:102`).
Secondary: `TODO/podssh.md` T-1406, which owns reconnect and which the result
file exercises at its last two lines (`relay closed the node socket: 1001
operator stopped reverse relay` then `node socket ended; redialing`).

**Result files it wrote.** `experiments/results/mux-two-client.txt`, dated
`2026-09-28T20:04:16Z`, relay version `2026-09-28-r12`, verdict `TWO CLIENTS ON
ONE NODE CONNECTION HOLD`.

**Verdict: KEEP-SHELL.**

It drives a third-party hosted relay over the network, with a real `ssh`
client and a real `sshd` per session. `crates/podbox-ssh/tests/mux_two_client.rs`
already covers every clause against a high-fidelity fake relay on loopback,
and its own header says so: "the live drive in
`experiments/387-mux-two-client.sh` is the acceptance gate against the real
relay." Removing the live drive removes the only proof that the fake's rules
match the real relay.

**What a Rust test already covers.** `mux_two_client.rs` has
`three_authenticated_ssh_sessions_share_one_node_connection` at line 1051,
which is the script's clause 5 verbatim on a loopback fake. It also has
`two_sessions_share_one_node_connection_through_cat` at line 881 and
`node_redial_pairs_a_second_session_after_the_first_socket_drops` at line 972,
the latter being T-1406's arm. The script's remaining unique value is the
live relay, the token role split against the live `/v1/status`, and the stop
lifecycle against the live `/v1/stop`.

**`exit 2` becomes.** Nothing.

## Group 2 - experiments/389-remote-ssh.sh

**What it measures.** That `podbox remote ssh` drives real commands with real
exit codes on both placements, and that every help and error path answers. Five
clauses. Help paths print usage with 0. `remote ssh forward` as an ssh
ProxyCommand over a loopback `sshd` runs a command and passes exit 42 through,
twice. `remote ssh serve` and `remote ssh connect` do the same through the
live relay. The parity table carries the rows and no token reaches the log.

- Pinned inputs: none local, apart from the live relay at
  `https://tcp.ssh.relay.ajam.dev` (`:31-32`).
- Host assumptions: `cd /work` (`:23`); `bootstrap-env.sh rust cc zig openssh
  tools`; `ssh`, `sshd`, `ssh-keygen`, `curl`, `jq`; `/run/sshd`; a loopback
  port 2223.
- Exit contract: 0 every clause held, 1 one disagreed, 2 the lane could not
  run. Note line 308: `exit "$fail"`, so the script returns 1 on a failure and
  0 otherwise, and every `exit 2` is an explicit one.
- Timeouts: `timeout 25m` on the build, `timeout 60` on every `podbox` verb,
  `timeout 90` on the relay session, `-m 20` and `-m 25` on curl, a
  60-iteration 2 s poll.
- Cleanup: `trap` kills `SERVE_PID` and `SSHD_PID`, copies `$OUT` to `/out`, and
  removes the work dir.
- Positive control: the `sshd` daemon is polled for 30 s and the script exits 2
  if it never answers (`:159-162`), so the forward clause cannot pass against
  nothing.
- Failure control: `forward` to a refused port must fail loud with a non-zero
  ssh exit (`:195-205`). A forward that reads as success fails the clause.

**TODO entries that own it.** `TODO/podssh.md` T-1404 (`Prove` at `:126`).

**Result files it wrote.** `experiments/results/remote-ssh.txt`, dated
`2026-09-29T08:39:40Z`, commit `799b983`, verdict `REMOTE-OK: serve and
connect drive real commands with exit codes`.

**Verdict: SPLIT.**

Clause 2 is pure CLI table logic and is already unit tested in
`crates/podbox-cli/src/remote/mod.rs`'s `mod tests` at line 231:
`bare_and_help_print_usage_and_exit_zero` (line 235),
`an_unknown_member_names_itself_and_is_a_runtime_error` (line 245),
`an_unlisted_flag_is_a_flag_error` (line 254), and
`a_dash_word_after_ssh_is_a_flag_refusal_not_a_command` (line 312). Clause 3's
direct placement is covered by
`crates/podbox-ssh/tests/ssh_over_unix_e2e.rs`: `ssh_runs_a_command_through_proxy_and_socketpair`
(line 107) and `remote_exit_code_passes_through` (line 121). Clauses 4 and 5
need the live relay and stay.

### Piece 1: clause 2, CLI table logic

**Assertion today.** Precondition: none. Action: run `podbox remote`,
`remote -h`, `remote ssh`, `remote ssh -h`, `remote ssh serve -h`,
`remote ssh connect -h`, `remote ssh forward -h`, `machine`, `machine ssh -h`,
then `podbox ssh`, `podbox remote relay`, `podbox remote --bogus`, and
`podbox remote ssh --bogus`. Expected: the nine help paths print a line
beginning `usage:` and exit 0; the four refusals exit 125 and name the reason.

**Rust level.** Pure. Every arm is a table lookup against `parity.rs`.

**Target.** `crates/podbox-cli/src/remote/mod.rs`, `mod tests`. Add the missing
arms. `machine ssh -h` and `remote ssh serve -h` are not covered today; the
existing tests cover the bare and unknown cases.

**Plant.** Add `scripts/plant.sh` cases that remove one `remote ssh` row from
the parity table and assert the refusal names the wrong member. Today the only
plant is for `check-todo.py`'s own table checks.

**Proof command.** `cargo test -p podbox-cli remote`.

**`exit 2` becomes.** No binary and no registry are needed. Nothing becomes 2.

**Result file.** The clause 2 lines in `experiments/results/remote-ssh.txt`
stay as history.

### Piece 2: clauses 3, 4, 5, live drive

**Assertion today.** Clause 3: preconditions, a loopback `sshd` on port 2223
answering; action, `ssh -o ProxyCommand="podbox remote ssh forward tcp
127.0.0.1 2223" localhost "echo MARKER"` then the same with `exit 42`;
expected, the marker back at exit 0 and 42 passed through.
Clause 4: the relay pair is minted, `remote ssh serve` registers, one session
through `remote ssh connect` runs `echo MARKER2; exit 42`, expected 42 with the
marker, then stop 200 and status 403.
Clause 5: `podbox system info --format '{{json .Parity}}'` contains
`"remote ssh"`, and no token or pair name is in the committed log.

**Rust level.** Clause 3 is integration proof, and
`ssh_over_unix_e2e.rs` already is that test in Rust. Clauses 4 and 5 are
deployment proof against a live third-party service.

**Target.** Move clause 3 into `crates/podbox-ssh/tests/ssh_over_unix_e2e.rs`
if the existing two tests do not already cover the ProxyCommand-with-target
shape; the script's clause 3 uses
`remote ssh forward tcp 127.0.0.1 2223` and the existing test uses a unix
socket through `proxy`. That difference is real and worth one more test.

Clauses 4 and 5 stay shell, for the same reason as 387: a live relay.

**Target for the live half.** No Rust target. Keep
`experiments/389-remote-ssh.sh`, reduced to clauses 4 and 5.

**Fixtures.** `crates/podbox-ssh/tests/common.rs` already provides
`require_binary`, `fresh_tmp`, `make_keys`, `ensure_privsep`, `run_ssh`, and
`wait_for`. Reuse it rather than writing the key generation again. It is
`mod common` and the new test joins the existing suite.

**Plant.** For clause 3's new test, plant the refusal: point the ProxyCommand
at a closed port and assert the test's own guard rejects a 0.

**Proof command.**

```
cargo test -p podbox-ssh
sh experiments/389-remote-ssh.sh
```

No feature flags.

**`exit 2` becomes.** For the reduced shell script, nothing. For the new Rust
integration test, a missing `sshd` on PATH is a `panic!` naming the binary,
which is what `common.rs` does at `require_binary`.

**Result file.** `experiments/results/remote-ssh.txt` is kept whole. A reduced
script writes the same file; do not split it.

## Group 2 - experiments/396-audit-linux.sh

**What it measures.** That the audit tree passes the full Linux gate and every
record-check plant, and returns its artefacts.

- Pinned inputs: none. It runs on whatever tree the wrapper copied.
- Host assumptions: `PWD` is `/work` and `.git` exists (`:7`); `/out` can be
  created (`:8`); `scripts/common/bootstrap-env.sh rust cc zig tools openssh`
  succeeds (`:14`); `./scripts/dev.sh check` and `./scripts/plant.sh` both
  succeed; five release binaries and `dist/podbox-ssh-x86_64.tar.gz` exist.
- Exit contract: 0 when both suites pass and every copy succeeds, 2 when the
  tree is wrong or the bootstrap fails, and `exit "$rc"` from a failed suite
  (`:22`, `:26`). The copies at `:28-32` use `exit 1` for a missing artefact,
  which is a failure to copy rather than a failed check.
- Timeouts: none. `dev.sh check` and `plant.sh` run unbounded.
- Cleanup: none. It is a container job input, and the wrapper removes the
  container.
- Positive control: none. It is a gate runner, and the plants are its control.
- Failure control: `scripts/plant.sh` is the failure control, run in the same
  job at `:23`.

**TODO entries that own it.** **None by name.** `grep -h 'experiments/396-'
TODO/*.md` returns nothing. Of the ten repository-audit scripts `393` through
`401`, only three appear in an entry `Prove`: `393-build-freshness.py` under
`TODO/gate.md` T-1349, `398-gate-diagnostics.py` under T-1351, and
`399-publication.py` under `TODO/podssh.md` T-1405. `394`, `395`, `396`, `397`,
`400` and `401` appear in no entry. `TODO/PROGRESS.md:57` links
`experiments/results/repo-audit-linux.txt`, which is this script's output, but
PROGRESS is not an entry.

**Result files it wrote.** It writes to `/out` only. Its transcript is
`experiments/results/repo-audit-linux.txt`, saved by a caller, not by the
script.

**Verdict: KEEP-SHELL.**

It is a Windows job input. It requires `PWD=/work` and a copied container
checkout. It is not a gate that can run on a developer machine, and the
Linux-native path already exists as `scripts/dev.sh check`.

### The real finding

This script is an entry payload, not an experiment. It sits under
`experiments/` because the wrapper takes a job input, and
`experiments/README.md:33-44` lists it under "Repository audit proofs". The
listing is the only ownership it has.

## Group 2 - scripts/build-state.py

**What it measures.** Whether a build is fresh, from input and output bytes.
Four actions: `inputs` prints the input digest, `record` writes
`.dev/build-state.json`, `status` prints `ready` or a `stale:` reason and
returns 0, 1, or 2, and `commit` prints the build commit.

- Pinned inputs: none. It walks `crates`, `vendor`, `scripts`, `.cargo`,
  `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and the two embedded
  `libpodbox_interpose.so` objects (`:39-61`).
- Host assumptions: `git`, `rustc`, `cargo`, `zig` on PATH, each with a 10 s
  timeout (`:20-26`, `:71-75`).
- Exit contract: 0 ready, 1 stale, 2 absent or unmeasurable (`:80-100`,
  `:131-134`).
- Timeouts: 10 s on every subprocess.
- Cleanup: `record` writes `.dev/build-state.json.new` then replaces
  (`:122-126`), so a partial write never lands.
- Positive control: the fixture `experiments/393-build-freshness.py` writes a
  record and asserts `state` returns `("ready", 0)` before it mutates anything
  (`393-build-freshness.py:47`).
- Failure control: the same fixture, one per input file plus the outputs
  (`:49-70`).

**TODO entries that own it.** `TODO/gate.md` T-1349 by name in the module
docstring (`build-state.py:2`) and by `Prove`
(`python3 experiments/393-build-freshness.py` exits 0 with changed source and
output refused; full dev check returns 0).

**Result files it wrote.** `.dev/build-state.json`, which is gitignored. Its
proof is `experiments/results/build-freshness.txt`.

**Verdict: RUST-TOOL.**

It is a build-system component called five times by `scripts/dev.sh`
(`:54`, `:90`, `:102`, `:163`, `:196-199`, `:264`). It hashes the whole build
input set and must run before every build.

### Conversion plan

**Assertions today.**

- A. Precondition: a record exists whose `inputs` equals the current input
  digest. Action: hash all five output binaries. Expected: `ready`, exit 0.
- B. Precondition: as A. Action: change one input file's bytes and restore its
  timestamp. Expected: `stale: build inputs changed`, exit 1.
- C. Precondition: as A. Action: change the primary binary's bytes. Expected:
  `stale: binary bytes changed`, exit 1.
- D. Precondition: the binary is missing. Expected: `absent`, exit 2.
- E. Precondition: no record file. Expected: `stale: no build record`, exit 1.

**Rust level.** Pure. `docs/conventions/code.md`: "Use pure tests for
deterministic logic." The only impurity is hashing files, which is
deterministic given bytes.

**Target.** A library inside `podbox-cli` is wrong; this is not CLI behaviour
and `podbox-cli` is the shipped binary. Create `crates/podbox-buildstate` as a
library plus one `[[bin]] name = "podbox-buildstate"`. Add it to the workspace
`members` list in `Cargo.toml`.

**CLI surface.** Identical to today's, because `scripts/dev.sh` already parses
it:

```
podbox-buildstate inputs  --root PATH --target TRIPLE
podbox-buildstate record  --root PATH --target TRIPLE
podbox-buildstate status  --root PATH --target TRIPLE --binary PATH
podbox-buildstate commit  --root PATH
```

Same exit codes: 0, 1, 2. Same stdout strings, because `scripts/dev.sh:163`
prints `status` and the exit code is what it reads.

**Fixtures.** None. It reads the real tree.

**Plant.** The Rust port must ship `tests/build_freshness.rs`, a port of
`experiments/393-build-freshness.py`, as a `cargo test`. It writes the same
fixture tree into a temp directory and asserts the five arms. `crates/podbox-buildstate/tests/`
would be the first integration-test directory outside `crates/podbox-ssh/`.

`scripts/plant.sh` needs no case: this is not a `check-todo.py` check.

**Proof command.**

```
cargo test -p podbox-buildstate --test build_freshness
cargo run -p podbox-buildstate --bin podbox-buildstate -- status --target x86_64-unknown-linux-musl
```

No feature flags.

**`exit 2` becomes.** Identical. The binary already has three states and Rust
`main` can return 2.

**Result file.** `experiments/results/build-freshness.txt` is kept.
`experiments/393-build-freshness.py` becomes obsolete once the Rust test
lands; Group 1 owns it, and this conversion should say so in that handoff.

## Group 2 - scripts/release-licenses.py

**What it measures.** Nothing measured. It produces an artefact: the retained
licence text for every locked Cargo package, plus an `inventory.json` with a
sha256 per file.

- Pinned inputs: `Cargo.lock` through `cargo metadata --locked --format-version
  1`, over both the workspace and `crates/podbox-interpose/Cargo.toml` (`:23-30`).
- Host assumptions: `cargo` on PATH, with a 180 s timeout.
- Exit contract: 0 the inventory is written, 2 on any `OSError`, `ValueError`,
  `KeyError`, or `SubprocessError` (`:66-68`).
- Timeouts: 180 s on each `cargo metadata`.
- Cleanup: none needed; it writes into `--output`.
- Positive control: the identifier check `re.fullmatch(r'[A-Za-z0-9_.+-]+',
  name + version)` at `:35` and the containment check
  `resolved.is_relative_to(source)` at `:54` are the refusals.
- Failure control: none. There is no test of this script anywhere in the tree.

**TODO entries that own it.** `TODO/podssh.md` T-1405 by the module
docstring (`release-licenses.py:2`). T-1405's `Prove` names
`scripts/package-ssh.sh`, not this script. The only recorded proof of this
script is one line in the Linux check transcript: "release-licenses: retained
texts for 91 locked registry packages", in
`.dev/artifacts/linux-check.txt:302` and five sibling artifact copies.

**Result files it wrote.** None tracked. It writes into the package directory
that `scripts/package-ssh.sh:41` stages into the release archive.

**Verdict: RUST-TOOL.**

It is a packaging step. It runs in the release path through
`scripts/package-ssh.sh`, which `TODO/podssh.md` T-1405 owns.

### Conversion plan

**Assertions today.**

- A. Precondition: the locked package set is non-empty. Action: run
  `cargo metadata --locked`. Expected: at least one package with a `source`.
- B. Precondition: as A. Action: find licence text for each package by name
  prefix. Expected: at least one file per package; a package with none is a
  `ValueError` and exit 2.
- C. Precondition: as A. Action: resolve each licence path. Expected: it is
  inside its own package source directory; a path that leaves is a
  `ValueError` and exit 2.
- D. Precondition: a package name or version contains a character outside
  `[A-Za-z0-9_.+-]`. Expected: `ValueError`, exit 2.

**Rust level.** Pure, except A, which reads the network-backed registry index
through `cargo metadata`. B, C, and D are pure path and name logic and become
unit tests. A becomes the binary's runtime behaviour.

**Target.** A library inside a new `crates/podbox-release` with one
`[[bin]] name = "podbox-release"`, or a second `[[bin]]` in `crates/podbox-buildstate`
if the two are landed together. Prefer separate: this one needs
`serde_json` for the inventory and `sha2` for the digests, and that is a
different dependency set from the freshness tool.

**CLI surface.**

```
podbox-release licenses --output DIR --root PATH
                         --manifest Cargo.toml
                         --manifest crates/podbox-interpose/Cargo.toml
```

Exit 0 the inventory is written, 2 on any refusal. Keep the stdout line
`release-licenses: retained texts for N locked registry packages` or change it
and update `scripts/package-ssh.sh:41` in the same change.

**Fixtures.** A fixture tree under
`crates/podbox-release/tests/fixtures/registry/` holding two fake package
directories, each with a `LICENSE` file, and one with a symlinked licence that
leaves its package. Unit tests over `cargo metadata` output as parsed JSON, not
over a live registry.

**Plant.** Unit tests are the plant here: the current script has none. Four
tests, one per arm A through D, each asserting the named refusal. A test that
asserts `is_relative_to` rejects an escaping path is the plant for the
containment arm; without it that line is an assertion nobody has seen fail,
which is `scripts/plant.sh`'s whole argument.

**Proof command.**

```
cargo test -p podbox-release
sh scripts/package-ssh.sh RELEASE_DIR x86_64
```

No feature flags.

**`exit 2` becomes.** Identical. `main` returns 2.

**Result file.** None exists. There is no tracked result file for this script,
and `TODO/podssh.md` T-1405 does not cite one. That is a gap: T-1405's Done
text reads "The archive includes actual licence texts for the locked package
set. [The Linux result](../experiments/results/repo-audit-linux.txt) records
the proof." That is a transcript line, not a measurement.

## Group 2 findings

### F1. T-1338's Done paragraph quotes a figure no result carries

`TODO/gate.md:1430-1431` reads:

> `experiments/results/perf-kvm.txt`: KVM guest boot 0.936 s against TCG
> 1.869 s

`experiments/results/perf-kvm.txt` reads `guest.kvm.boot kvm 0.935 s wall ok`.
The string `1.869` appears nowhere under `experiments/results/`.
`experiments/results/perf-lane.txt:123-124` reads `guest.tcg.boot tcg - s wall
could-not-run` followed by `note: no qemu-system-x86_64 on PATH: guest metrics
could not run`. The comparison T-1338's Done asserts was never taken.

### F2. Two line references in T-1340 and in 360's own report are stale

`TODO/gate.md:1492` and `experiments/360-perf-harness.sh:157` both cite
`crates/podbox-cli/src/run.rs:754-784` for the `--rm` rootfs deletion. That
code now sits at `crates/podbox-cli/src/run.rs:951-980`; line 758 is inside
`podbox_supervise::start`. `TODO/gate.md:1494` cites
`crates/podbox-supervise/src/lib.rs:305-311` for `referencing`, which is now
at line 312. `docs/conventions/prose.md` section "Claims" requires citing the
file and line, so a stale line is a broken citation.

### F3. `396-audit-linux.sh` has no owning entry

`grep -h 'experiments/396-' TODO/*.md` returns nothing. Of the ten
repository-audit scripts `393` through `401`, only three are named in an entry
`Prove`: `393` under `TODO/gate.md` T-1349, `398` under T-1351, and `399` under
`TODO/podssh.md` T-1405. Six have none. `experiments/README.md:39` lists 396
under "Repository audit proofs", which is the only ownership it has.
`TODO/PROGRESS.md:57` links its output without naming the script.

That ratio is the finding. The audit scripts numbered `392` through `401` were
added together and only three were wired to an entry. A reviewer cannot tell
which of the other seven are current.

### F4. `scripts/build-state.py` has no `Prove` that runs it

`TODO/gate.md` T-1349's `Prove` names
`python3 experiments/393-build-freshness.py` and "full dev check". The fixture
imports `build-state.py` by path (`393-build-freshness.py:13-15`) and exercises
it in-process. Nothing runs `build-state.py status` as an acceptance command,
even though `scripts/dev.sh` calls it at lines 54, 90, 102, 163, 196, 199, and
264.

### F5. `scripts/release-licenses.py` has no test and no tracked result

No test anywhere imports it. Its only recorded proof is one stdout line in the
Linux check transcript, copied into five `.dev/*/linux-check.txt` artifact files
and several `.tmp/*.log` files. T-1405's Done cites
`experiments/results/repo-audit-linux.txt` for it, which is a transcript and not
a measurement. Its two refusals, the name check at `:35` and the containment
check at `:54`, are assertions nobody has seen fail.

### F6. `251-tty-refusal-no-ptmx.sh` runs `250-negative-tests.sh` and ignores a non-zero exit

Line 213 runs 250 under `timeout 2400` and line 214 reads `t250`. Line 215
records it. Nothing sets `fail` from it, and line 227 returns
`$(cat "$WORK/focused")`, which the focused clause set. A 250 that exits 1 is
recorded and ignored. The saved result reads `250 exit: 0`, so the reading is
fine; the contract is not.

### F7. `366-namespace-base.sh` has no trap and defaults to another script's path

Line 25 reads `BIN="${PB_BIN:-/workspaces/pb365/podbox}"` and line 26 reads
`WORK="${PB_WORK:-/tmp/pb365-work}"`. The path names `365-namespace.sh`, the
denied-host sibling, not this script. There is no `trap`, so a killed run
leaves `/tmp/pb365-work` and its containers. `scripts/dev.sh` and
`160-store-gc.sh` both trap; this one does not.

### F8. `160-store-gc.sh` writes its result over a live result file each run

Line 233 redirects the `{ ... }` block into `$OUT`, which is
`experiments/results/store-gc.txt`. Every re-run overwrites the tracked
measurement. `docs/methodology/experiments.md` says "Save negative results with
the same conditions as successful results" and "Do not remove the saved
result"; a tracked result a script overwrites in place is not a saved result
in the same sense as `153`'s, which also overwrites but is dated in its own
conditions block.

### F9. `153-store-lock-race.sh` and `160-store-gc.sh` both mutate nothing outside the tree but restate two facts in two homes

`160-store-gc.sh:5-9` restates T-0204's mechanism and cites
`references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:275-278`. The
entry `TODO/image.md` T-0204 states it again in `Premise`.
`docs/conventions/prose.md` "One fact, one home" applies. This is not a defect;
it is a cost the conversion removes when the shell goes.

### F10. The store tests the scripts assert already exist and have no plants

`an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune`
(`crates/podbox-image/src/store.rs:1976`) and
`a_shared_blob_survives_removing_one_of_the_two_images_that_reach_it`
(`store.rs:1950`) cover `160-store-gc.sh` clauses 1 through 4. `grep -n` for
`in use` in `scripts/plant.sh` returns no case. `AGENTS.md` says "Add a plant
with each new check. A green check without a failure test is insufficient."
These tests were not added with a plant.

### F11. `387-mux-two-client.sh`'s unique value is three clauses, not six

`crates/podbox-ssh/tests/mux_two_client.rs:1051`
`three_authenticated_ssh_sessions_share_one_node_connection` is the script's
clause 5 against a loopback fake. The script's remaining unique value is the
live relay: clause 3's token role split, clause 6's stop lifecycle against the
live `/v1/stop`, and clause 7's token grep against the live node log.

### F12. `371-validationos-stream.sh` writes a result file named for a different experiment

Line 193 writes `experiments/results/windows-365.txt`. Experiment 365 is
`365-namespace.sh`, whose result is `namespace.txt`.
`experiments/README.md:5` says "Each numbered script owns a repeatable
measurement. Its number is permanent."

### F13. `360-perf-harness.sh` has no exit trap and leaves its work directory

Line 41 removes and recreates `experiments/.sweep360-work-$SHAPE` at entry.
Nothing removes it at exit. A failed drive leaves the store, the extracted
rootfs, and the fetched kernel in the tree.

## Group 2 - entries read

Read in full, each with the Group 2 script it owns or is read beside.

1. `TODO/image.md` T-0215 "Four lock tests fail in two runs of five, and the
   gate has never said so" - owns `153-store-lock-race.sh`. Its `Prove` at
   line 1655 names the script and clauses `0 1 6 12 13 14`.
2. `TODO/image.md` T-0204 "`images`, `rmi`, `tag`, and a store GC that cannot
   delete a running container's rootfs" - owns `160-store-gc.sh`. Its Done
   text at line 318 records all five clauses and the `flock(1)` harness defect.
3. `TODO/image.md` T-0206 "A registry fixture, so the acceptance stops
   depending on somebody else's quota" - names `160-store-gc.sh` at line 452
   as one of the two scripts moved off Docker Hub.
4. `TODO/image.md` T-1310 "The store suite exhausts the sixteen fork-shed
   slots, and the victim varies" - read beside T-0215; it names
   `FORK_CLOSE_SLOTS` at `crates/podbox-probe/src/sys.rs:702` and the lock
   registration sites.
5. `TODO/milestones.md` T-1109 "The negative tests, which are tests" - owns
   `251-tty-refusal-no-ptmx.sh`. Its Done text at lines 580-598 names the
   cover, the focused verdict, and the renewed `negative-tests.txt`.
6. `TODO/enter.md` T-1339 - owns `366-namespace-base.sh`. Its `Prove` at
   lines 731-735 names both halves and its Done text at line 762 quotes the
   five base clauses.
7. `TODO/enter.md` T-0503 "Probe `/dev/ptmx`, and refuse `-t` by name where it
   is absent" - the predicate `251-tty-refusal-no-ptmx.sh` drives, with the
   T-1317 ordering note at its foot.
8. `TODO/gate.md` T-1338 "A performance harness with baselines and a regression
   gate" - owns `360-perf-harness.sh`. Its `Prove` at line 1399 and its Done
   text at lines 1405-1448, including the TCG figure in F1.
9. `TODO/gate.md` T-1340 "Isolate why early payload runs cost seconds and late
   ones do not" - owns the run curve in `360-perf-harness.sh`, and carries two
   stale line references (F2).
10. `TODO/gate.md` T-1341 "Budget the warm run cost beside the cold one" -
    owns the `run.kept` row in `360-perf-harness.sh`.
11. `TODO/gate.md` T-1343 "The Windows lane uses toolkit 6 job inputs and
    checks every retained job" - owns `384-windows-lane-v6.sh`. Its `Prove` at
    line 1594 names it and at line 1634 names it again with two inputs.
12. `TODO/gate.md` T-1344 "Experiment jobs find the checkout after the Windows
    input change" - its `Prove` at line 1634 names
    `384-windows-lane-v6.sh` and the `/work` executable modes.
13. `TODO/gate.md` T-1349 "Verify build freshness from input and output bytes"
    - owns `scripts/build-state.py` by its module docstring, and
    `393-build-freshness.py` by its `Prove`.
14. `TODO/gate.md` T-1350 "Make Windows proofs use explicit inputs and return
    build artifacts" - read beside `396-audit-linux.sh`; its Done text names
    the five exported executables that 396 copies.
15. `TODO/milestones.md` T-1112 "A disposable guest that is not Linux" - owns
    `371-validationos-stream.sh` through its `Source` line, "experiments 369,
    370, 371 and 392". `Status: partial`.
16. `TODO/podssh.md` T-1403 "Prove concurrent sessions on one relay
    connection" - owns `387-mux-two-client.sh`. Its `Prove` at line 102 names
    it and line 105 links the saved result.
17. `TODO/podssh.md` T-1404 "Add the remote and machine SSH verbs after the
    transport holds" - owns `389-remote-ssh.sh`. Its `Prove` at line 126 names
    it.
18. `TODO/podssh.md` T-1405 "Ship and smoke the SSH helper archive" - owns
    `scripts/release-licenses.py` by its module docstring, through
    `scripts/package-ssh.sh:41`.

## Group 2 - scope of this audit

Read: all twelve Group 2 scripts in full; the eighteen entries above in full;
`experiments/results/store-lock-race.txt`, `store-gc.txt`,
`tty-refusal-no-ptmx.txt`, `perf-lane.txt`, `perf-kvm.txt`, `perf-seeds.tsv`,
`namespace-base.txt`, `windows-365.txt`, `vos-stream-371-ceiling.txt`,
`validationos-vhdx-host.txt`, `windows-lane-v6.txt`, `mux-two-client.txt`,
`remote-ssh.txt`, `negative-tests.txt`, `repo-audit-linux.txt`,
`build-freshness.txt`; `Cargo.toml`, `crates/podbox-image/src/store.rs`,
`crates/podbox-image/src/contain.rs`, `crates/podbox-probe/src/exit.rs`,
`crates/podbox-probe/src/probes.rs`, `crates/podbox-probe/src/sys.rs`,
`crates/podbox-cli/src/run.rs`, `crates/podbox-cli/src/images.rs`,
`crates/podbox-cli/src/remote/mod.rs`, `crates/podbox-enter/src/lib.rs`,
`crates/podbox-ssh/tests/common.rs`,
`crates/podbox-ssh/tests/mux_two_client.rs`,
`crates/podbox-ssh/tests/ssh_over_unix_e2e.rs`, `scripts/dev.sh`,
`scripts/package-ssh.sh`, `scripts/plant.sh`, `scripts/check-todo.py`
(check 29 is `check_post_task_cleanup` at line 1289 and check 30 is
`check_perf_budget` at line 1382), `scripts/common/exit-codes.sh`,
`scripts/windows/run-in-base.sh`, `experiments/352-ascii-output.sh`,
`experiments/393-build-freshness.py`, `experiments/perf-ceilings.tsv`,
`experiments/README.md`, `AGENTS.md`, `TODO/RULES.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/methodology/gate.md`, `docs/conventions/code.md`,
`docs/conventions/prose.md`, `TODO/PROGRESS.md`.

Not read: the sibling scripts in groups 1, 3 through 10, except 352 and 393
above, which Group 2 scripts call. The reference corpus under `references/`.
The `docs/history/` tree.

Not run: no experiment script and no gate was executed. Every claim about
behaviour in this document is read from source or from a saved result file,
never observed. Line numbers were taken from the files at commit `5bd8cb0`
with `refactor/` untracked.