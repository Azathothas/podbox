# Group 4 audit

Scope: the 12 scripts listed under `## Group 4` in
`refactor/00-orientation/assignment-map.md`. No other group's script was read.

## Group 4 summary

| Metric | Value |
| --- | --- |
| Scripts | 12 |
| Lines (assignment map) | 2903 |
| Lines re-counted from the files | 2903 (matches) |
| DELETE | 0 |
| RUST-TEST | 5 |
| RUST-TOOL | 2 |
| KEEP-SHELL | 3 |
| SPLIT | 2 |

Verdicts by script:

| Script | Lines | Verdict |
| --- | --- | --- |
| `experiments/245-interpose-sweep.sh` | 583 | KEEP-SHELL |
| `experiments/270-multiarch-image.sh` | 292 | SPLIT |
| `experiments/30-attribution-census.sh` | 127 | RUST-TEST |
| `experiments/300-run.sh` | 404 | SPLIT |
| `experiments/353-open-issue-triage.sh` | 184 | KEEP-SHELL |
| `experiments/369-windows-tcg-dos.sh` | 259 | KEEP-SHELL |
| `experiments/382-restricted-sshd.sh` | 320 | KEEP-SHELL |
| `experiments/70-whiteout-contract.sh` | 154 | RUST-TEST |
| `experiments/85-completion-symlink-escape.sh` | 225 | RUST-TEST |
| `experiments/95-podman-vfs-ignorechown.sh` | 201 | RUST-TEST |
| `scripts/document-state.py` | 90 | RUST-TOOL |
| `scripts/release-notes.sh` | 64 | RUST-TOOL |

### The 3 most important findings

**1. `TODO/gate.md` T-1212's `Prove` says six scripts "each exit 0" and its own
transcript records `EXIT:2` for one of them.**
`TODO/gate.md:964-968` reads `` `./experiments/150-...`, `./experiments/270-...`, `./experiments/280-...`, `./experiments/300-run.sh`, ... `` each exit 0 on host podman with the
conditions block naming the driver. The transcript six lines below it at
`TODO/gate.md:1026-1037` ends `sh experiments/300-run.sh; echo EXIT:$?` /
`EXIT:2`. The entry's own prose at `TODO/gate.md:978-980` explains why: "`300`
carries zero FAILs and one SKIP". A `Prove` that reads exit 0 against a script
that reports 2 has no state for the third answer.

**2. `70-whiteout-contract.sh` writes its result outside `experiments/results/`
and the tracked result file is a manual transcription, not a script output.**
`experiments/70-whiteout-contract.sh:32` reads `OUT="${OUT:-$HERE/.whiteout}"`
and `:33` runs `rm -rf "$OUT"; mkdir -p "$OUT"`. It never writes
`experiments/results/`. The tracked `experiments/results/whiteout-contract.txt`
was committed by hand. Six TODO entries cite that file as a measurement
(`TODO/extract.md:16,88,136,173,278`, `TODO/milestones.md:193`). Its date line,
`2026-09-08T06:11:14Z`, does not match `TODO/extract.md:187`, which dates the
same work `2026-09-08` with the same count, so the file is plausible as a
transcription, but nothing in the tree makes that mechanical.

**3. Two scripts assert against numbers the owning entry has already corrected.**
`experiments/270-multiarch-image.sh:266` and its printed heading still say
"a malformed `--platform` is a USAGE error", while `TODO/image.md:1272-1282`
records that podbox's `pull` parser takes any string and the verb refuses
`a/b/c/d` at the cli-error code 1, not 125. The script body was fixed (line 276
asserts `$PODBOX_EXIT_CLI_ERROR`) and the printed heading was not; the saved run
`experiments/results/multiarch-image.txt:28` reads `exit 1 (1 is a cli error,
TODO/cli.md T-0802)`, which is the corrected value under a stale heading.

---

## Group 4 — experiments/70-whiteout-contract.sh

**What it measures.** Four facts about the OCI layer contract, against two
images pinned by digest and one crafted tar.
- A: no layer of either image writes a `./` prefix on its first member
  (`:78-90`).
- B: `alpine`'s `etc/shadow` is uid 0 gid 42 (`:93-104`).
- C: `voidlinux-musl` ships `var/cache/xbps -> /var/cache/xbps` (absolute,
  self-referential) in layer 1 and whiteouts it in layer 2 (`:107-127`).
- D: udocker's selector `tar t --wildcards -f LAYER '*/.wh.*'` misses a
  layer-root whiteout that a basename oracle finds, on a crafted archive with
  one root and one nested whiteout (`:130-149`).

**Pinned inputs.** `alpine@sha256:d9e853e...4b6bc` and
`voidlinux/voidlinux-musl@sha256:d5c970d0...54cd`, at `:35-36`.

**Host assumptions.** `tar` on PATH, `docker` on PATH, and `docker info`
answering (`:48-50`). `jq` is optional; without it, `layers()` greps the
manifest (`:71-75`).

**Exit-code contract.** `:154` is `exit "$rc"` where `rc` is 0 or 1 only. There
is no 2 path once past `:50`. The header comment at `:27-28` claims "2 could not
run (no docker, no tar, no network)", and the SKIP paths at `:48-50,64-66` do
exit 2. The header is correct.

**Timeouts.** None. `docker pull`, `docker save` and `tar xf` are unbounded.

**Cleanup.** `OUT` is `rm -rf`'d at the start (`:33`) and never removed at the
end. It holds two unpacked `docker save` trees per run, so the script leaves
scratch behind in `experiments/.whiteout/`.

**Positive control.** D's nested whiteout is the control: the glob finds
`dir/.wh.nested` and misses only `.wh.rootfile`, so the instrument
discriminates. `experiments/results/whiteout-contract.txt:26-28` shows both
sides.

**Failure control.** None. No arm asserts a bad input is caught.

**TODO entries that own it.** T-0303 (`TODO/extract.md:153-201`, `Prove:` at
:185 is `./experiments/70-whiteout-contract.sh exits 0`); T-0305
(`TODO/extract.md:267-316`, Premise at :278 cites the file); T-0307
(`TODO/extract.md:374-424`, `Prove:` at :399); T-1103
(`TODO/milestones.md:182-265`, `Prove:` at :203, and :247 names its exit 2
behaviour as a reason the `Prove` was rewritten).

**Result files written.** None under `experiments/results/`. See finding 2.

**Verdict: RUST-TEST.**

### Conversion plan — 70-whiteout-contract.sh

**Assertions today, as triples.**

1. Precondition: `podbox-extract` holds a layer stream whose entries have no
   `./` prefix. Action: `whiteout::classify` on each entry path. Expected: a
   root-level `.wh.xbps` is classified as a removal, not skipped.
   (Today this is asserted on the two real images' first members, which is a
   statement about the images, not about podbox.)
2. Precondition: a layer stream carries `etc/shadow` with uid 0 gid 42 and the
   process cannot apply gid 42. Action: `apply` the layer. Expected: the
   sidecar row records `uid 0 gid 42` with `applied.gid` 0 and a `reason`
   naming the unmapped gid.
3. Precondition: a layer carries a symlink whose target is
   `/var/cache/xbps`. Action: `safety::rebase_symlink_target`. Expected: the
   target is rebased and the link is admitted, and the file on disk still
   carries the image's verbatim target.
4. Precondition: a crafted tar with `.wh.rootfile` at the root and
   `dir/.wh.nested`. Action: `remove::collect` then `remove::apply`. Expected:
   both removals are collected; a selector anchored on `/.wh.` would return one.

**Rust level.** **Pure/unit**, for 1, 3 and 4, and **fixture** for 2.
`docs/conventions/code.md:27` says "Use pure tests for deterministic logic."
A, C and D are pure decisions over a byte stream and a path string. B needs a
layer blob on disk but no network: `crates/podbox-extract` already commits tars
through its own store scaffolding, which `TODO/extract.md:487-490` names
("The test commits an uncompressed tar through the store scaffolding").

**Target file paths and module names.**

- A, D: `crates/podbox-extract/src/whiteout.rs`, inside the existing
  `mod tests` at `:81`. The test
  `a_root_level_whiteout_is_found_and_a_glob_would_miss_it` at `:89` already
  covers D against the defect. Add
  `a_layer_with_no_dot_slash_prefix_yields_the_same_operations` beside it,
  driving `classify` over the member names the two images actually carry
  (`bin/`, `bin`, `etc/` from `experiments/results/whiteout-contract.txt:10-12`).
- B: `crates/podbox-extract/src/sidecar.rs`, `mod tests`. The row to assert is
  quoted verbatim in `TODO/extract.md:131`:
  `{"path":"etc/shadow","uid":0,"gid":42,"mode":"0640","applied":{"uid":0,"gid":0},"reason":"gid 42 unmapped"}`.
  `TODO/extract.md:146-149` records that
  `the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply` exists
  already; the new test asserts the intended half.
- C: `crates/podbox-extract/src/safety.rs`, `mod tests`, beside
  `rebase_symlink_target` which `TODO/extract.md:299` names as the landed
  mechanism.
- Layer ordering (the `TODO/extract.md:399` half): `crates/podbox-extract/src/remove.rs`,
  `mod tests`, where `a_layer_may_delete_a_path_and_recreate_it_in_the_same_layer`
  already exists per `TODO/extract.md:399`.

**Fixtures.** No new fixture. `experiments/70-whiteout-contract.sh:134-136`
builds the crafted tar with three `tar` commands; a Rust test builds the same
three members with `tar::Builder` and the same no-prefix member names. The
`tar::Builder` `..` refusal named at `TODO/extract.md:64-67` does not apply to
these three names.

**Plant.** The repository requires one per new check
(`AGENTS.md:77`, `scripts/plant.sh`). `plant.sh` operates on `check-todo.py`
checks, not on cargo tests, so the plant for a cargo test is a mutation
watched by hand, which is the shape `TODO/extract.md:189-194` records for this
exact defect ("Mutation-proved against the defect it exists for. Replacing the
basename test with udocker's own slash-anchored selector (`path.contains("/.wh.")`)
turns three tests red"). Reuse that mutation for the new A/D test and record the
count.

**Exact command.**

```sh
cargo test -p podbox-extract whiteout
cargo test -p podbox-extract sidecar
cargo test -p podbox-extract a_layer_may_delete_a_path_and_recreate_it_in_the_same_layer
```

No feature flags, no target triple. These are Linux paths (`openat2`,
`O_NOFOLLOW`), so run them in the base lane, not on the Windows host.

**What `exit 2` becomes.** The script's 2 arms are "no tar", "no docker", "no
daemon" and "cannot pull". None of them is a statement about podbox, so none
becomes a Rust assertion. A Rust test that cannot build its fixture is a
compile error, and one that cannot read the repository is a panic. The
"could not run" state moves to the layer above: the lane driver records it as a
skipped measurement, and the saved result says the crate test did not run.

**What becomes of the result file.** `experiments/results/whiteout-contract.txt`
stays, and becomes a historical record rather than a live reading. It describes
two image digests and a GNU tar 1.35 on 2026-09-08
(`experiments/results/whiteout-contract.txt:2-7`). A new
`experiments/results/whiteout-contract.txt` written by a live run cannot be
produced by a cargo test, so the file is superseded by the test names in
`TODO/extract.md` and moved to `docs/history/` when the entry's `Prove` is
rewritten. Six citations must move with it (`TODO/extract.md:16,88,136,173,278`;
`TODO/milestones.md:193`).

---

## Group 4 — experiments/85-completion-symlink-escape.sh

**What it measures.** Whether the completion layer writes outside the rootfs
when the image ships a symlink where a fixup writes. Six doors, five planted to
point outside the store and one legitimate internal directory link.

**Pinned inputs.** `public.ecr.aws/docker/library/alpine:3.20` (`:56`),
overridable by `PODBOX_COMPLETE_IMAGE`. Not digest-pinned.

**Host assumptions.** `$BIN` executable at
`$REPO/target/x86_64-unknown-linux-musl/release/podbox` (`:51,58`). A live
registry pull with a 600 s timeout (`:77`). A writable `PODBOX_STORE` (`:63`).

**Exit-code contract.** `:225` is `exit "$fail"`, so 0 or 1. There is no 2
after `:61`. The header at `:46` says "0 nothing escaped, 1 something did, 2
could not run", and `:58-61,77-87` do exit 2.

**Timeouts.** `timeout 600` on pull, `timeout 300` on extract and on run.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at `:54`. No engine mounts,
no external process.

**Positive control.** Door F (`:182-191`): `/etc/apt -> /var/aptreal` must be
followed and the drop-in must appear at its target. Without it, a completion
layer that writes nothing satisfies every other assertion. The header states
this at `:32-39` and the saved run shows it
(`experiments/results/completion-symlink-escape.txt:41`).

**Failure control.** Door D (`:192-207`) has two acceptable outcomes and one
inacceptable one, but no arm plants a defect podbox is expected to catch. The
control is the canary comparison itself: `check` at `:135-144` reads the file
and compares bytes, so an escape shows as changed content, not as a code.

**TODO entries that own it.** T-0405 (`TODO/complete.md:271-333`; `Prove:` at
:298, and the run quoted at :319-333); T-0402, T-0403, T-0404 and T-0407 are
named at the doors (`:27-29`) and are closed on this script's run.

**Result files written.** `experiments/results/completion-symlink-escape.txt`.

**Verdict: RUST-TEST.**

### Conversion plan — 85-completion-symlink-escape.sh

**Assertions today, as triples.**

1. Precondition: a rootfs whose `etc/mtab` is a symlink to a path outside the
   rootfs. Action: the completion layer writes `etc/mtab`. Expected: the outside
   file is byte-identical, and `etc/mtab` is now a regular file inside the
   rootfs carrying what podbox wrote.
2. Precondition: `etc/resolv.conf`, `etc/hosts` are symlinks out;
   `etc/passwd` is a symlink out. Action: same. Expected: all three canaries
   intact; each target replaced by a regular file, `etc/passwd` naming `root`.
3. Precondition: `etc/ssl/certs` is a symlink to a directory outside the
   rootfs. Action: the CA-bundle fixup. Expected: either a write at the rebased
   path inside the rootfs, or a `complete: failed` line on the banner. Never a
   write outside.
4. Precondition: `etc/apt -> /var/aptreal`, inside. Action: the apt fixup.
   Expected: `var/aptreal/apt.conf.d/99podbox` exists.

**Rust level.** **Unit**, for the door mechanism; **integration** for the
reachability claim. The script's own header at `:10-15` states the reason it is
not a unit test: "The unit tests in `crates/podbox-complete/src/write.rs` drive
one function. This drives the SHIPPED BINARY through the path a real container
start takes... A containment check that is right in the library and not reached
by `podbox run` is a check nobody has." That reasoning stands, so the library
test is necessary and not sufficient. The split below keeps both.

**Target file paths and module names.**

- Library, doors 1 to 3: `crates/podbox-complete/src/write.rs`, `mod tests` at
  `:640`. Two tests already carry the defect:
  `a_write_through_an_absolute_symlink_does_not_escape` (`:653`) and
  `a_write_through_a_symlinked_directory_is_refused` (`:678`). Add
  `the_link_is_replaced_and_the_file_outside_is_untouched` for the unlink-then-create
  half that `TODO/complete.md:306-310` names as the mechanism, and
  `a_dangling_directory_link_is_refused_and_reported` for door 3's second
  outcome.
- Reachability, doors 1 to 4 through the shipped binary: a new integration test
  in `crates/podbox-cli/tests/`. This is the first `tests/` directory outside
  `crates/podbox-ssh/tests/`, which the assignment note in the task brief
  confirms ("only `crates/podbox-ssh/tests/` exists today"). Target
  `crates/podbox-cli/tests/completion_containment.rs`. It builds a rootfs on
  disk from a fixture tar, plants the six symlinks with `std::os::unix::fs::symlink`,
  calls the completion entry point the `run` verb calls, and reads the canaries
  back with `fs::read_to_string`. It does not shell out to a release binary and
  does not need a registry.

**Fixtures.** The crafted rootfs replaces the registry image. Nothing under
`experiments/lib/*` or `experiments/src/*` serves here; the doors are planted
by the test itself. `experiments/results/completion-symlink-escape.txt:12`
records 13 fixup lines for `alpine`, which shows the plant has to be over a tree
with `dev/`, `etc/`, `var/`, so a minimal fixture must carry those directories
or the run takes a different path.

**Plant.** For the library test, replace the `O_NOFOLLOW` final-component open
in `crates/podbox-complete/src/write.rs` with a plain open and watch doors 1 to
3 go red with "the write landed outside". For the integration test, remove the
call from the `run` verb into the completion layer and watch all four doors
report "still a symlink". Both are mutations, and `TODO/complete.md` records
the same shape for the library. Report the test names that turned red, per
`TODO/extract.md:189-194`.

**Exact command.**

```sh
cargo test -p podbox-complete write
cargo test -p podbox-cli --test completion_containment
```

No feature flags. Linux lane: `open_parent` at `crates/podbox-complete/src/write.rs`
uses `openat2` and `O_NOFOLLOW`.

**What `exit 2` becomes.** The 2 arms are "binary not executable" and "could not
pull". Both disappear: the test never needs a release binary and never needs a
registry. A fixture that cannot be built is a test failure, not a skip, because
the fixture is under the test's own control. That is the honest reading of
`docs/methodology/experiments.md:14` ("A missing observation is not a denial or
a zero").

**What becomes of the result file.** `experiments/results/completion-symlink-escape.txt`
becomes historical. Its conditions block names `alpine:3.20` unpinned and host
kernel `6.18.44-fc-v24` (`:2-5`), which no later run will reproduce. The doors
and their expected outcomes move into the test names; the file moves to
`docs/history/` when `TODO/complete.md:298` is rewritten. Its recorded
degraded CA path (`:22-23`, D refused) is worth keeping as the reason door 3 has
two acceptable outcomes.

---

## Group 4 — experiments/30-attribution-census.sh

**What it measures.** Which mechanism produces which denial inside the
reconstruction, against a written expectation table. Three inputs from
`/workspace/.harness/probe`: `census`, `attribute`, `id` (`:30-32`).

**Pinned inputs.** None of its own. It depends on
`experiments/20-enter-target.sh` and the harness at
`references/Azathothas__container-research/tree/verification`, which
`TODO/milestones.md:104-106` records as the resolution.

**Host assumptions.** `grep` on `/boot/config-$(uname -r)` or
`/proc/config.gz` (`:25-27`). The reconstruction, so its own kernel has no
Landlock on the working host.

**Exit-code contract.** `:125-127`: 1 on any failure, then 2 on any skip, else
0. The header at `:13` matches.

**Timeouts.** None. Every call is `run_in`, which is
`experiments/20-enter-target.sh` (`:20`).

**Cleanup.** `--capture D` writes `identity.txt`, `census.txt` and
`attribute.txt` into `D` (`:33-37`) and never removes them. Without
`--capture` nothing is written at all.

**Positive control.** `pidfd_getfd(-1,-1) [control]` must answer
`errno=9 EBADF` (`:101`), and the header calls it a control in the line itself.
`TODO/probe.md:140-144` states the rule: "The controls are not optional."

**Failure control.** The Landlock block at `:110-120` is a branch, not a
control. No arm feeds a wrong expectation to prove the comparator rejects it.

**TODO entries that own it.** T-0102 (`TODO/probe.md:155` `Prove:`, close at
:157-160: "exits **2** (34 ok, 0 failed, 3 skipped)"); T-0110
(`TODO/probe.md:562` `Prove:`, close at :564-565: "the census exits **2**,
never 1"); T-1101 M0 (`TODO/milestones.md:109`, which is the `--capture
experiments/results` invocation that produced `census.txt`, `attribute.txt` and
`identity.txt`).

**Result files written.** With `--capture`: `experiments/results/census.txt`,
`attribute.txt`, `identity.txt`. Without: none. `TODO/probe.md:108-112`
records that `attribute.txt` did not exist until this capture ran.

**Verdict: RUST-TEST.**

### Conversion plan — 30-attribution-census.sh

**Assertions today, as triples.** The script asserts 19 census rows, 4 identity
rows, 10 attribution rows and 3 Landlock rows against hard-coded expectations
with a reason each. The shape of every one is the same:
- Precondition: probe child `c` ran the call named `row`. Action: the parent
  read verdict `got`. Expected: `got == want`, else FAIL with `why`.

**Rust level.** **Unit**, for the parser and the comparison; **fixture**, for
the expectation table. `docs/methodology/experiments.md:26` requires "a positive
control and a failure control where the instrument can otherwise pass without
reaching its subject." The comparison is pure; the readings are not. What can
be pure is: given this captured `census.txt`, `attribute.txt` and
`identity.txt`, the comparator classifies each row as ok, fail or skip
identically to the shell's `expect` at `:43-58`.

**Target file path and module name.** A new
`crates/podbox-probe/tests/attribution_expectations.rs`, because the subject is
the probe's parent-side interpretation rather than one function. `crates/podbox-probe`
has 17 files with inline `cfg(test)` today and no `tests/` directory, so this
is the second such addition in the tree.

**Fixtures.** The three captured readings already exist and are tracked:
`experiments/results/census.txt`, `experiments/results/attribute.txt`,
`experiments/results/identity.txt`. The test reads them as inputs. That is what
turns a 127-line shell driver into a deterministic check, and it is honest
about its conditions: the file names the host that produced it.
`experiments/results/attribute.txt:18-19` carries the Landlock rows
(`landlock (ABI 10)`, `landlock_create_ruleset(VERSION) OK`), so the captured
set exercises both branches of the shell's Landlock `if`. A second fixture with
those two lines removed exercises the other. Both are derived from the same
tracked file, so neither adds a new input.

**Plant.** Change one expectation in the table, or invert the comparator so
`got == want` becomes `got != want`, and watch the test fail naming the row. A
stronger plant: delete the `pidfd_getfd` control row from the table and watch a
test named `a_missing_control_row_is_a_failure_not_a_pass` go red. The control
rule is at `TODO/probe.md:140-144`, and `TODO/probe.md:187-189` names the
subtler failure this must catch ("A control that answers the **wrong** errno is
caught as well as an absent one").

**Exact command.**

```sh
cargo test -p podbox-probe --test attribution_expectations
```

No feature flags, no target triple. Pure parsing plus file reads; it runs
anywhere the crate builds.

**What `exit 2` becomes.** The 2 arms are "the reconstruction produced no
census" (`:38`) and "an expectation could not be tested here" (`:48,119`). The
first becomes a test that fails: the fixture is a tracked file and its absence
is a defect in the tree, not a missing precondition. The second becomes a third
verdict the test asserts explicitly. `crates/podbox-probe/src/select.rs` already
has a three-variant verdict type; `TODO/probe.md:581-583` names
`Verdict::exit_code` and `Verdict::from_exit_code` as one another's inverse.
The Rust test returns 0 when every row is ok or a named skip, and fails on
either a mismatch or an unattributed skip. Reporting which of those happened is
the test's assertion message, not its status.

**What becomes of the result files.** All three stay where they are. They become
the fixtures, and `TODO/milestones.md:108-112` and `TODO/probe.md:171` keep
citing them as the readings whose values the test now pins. Nothing is
superseded: a file that is a test input is still evidence.

---

## Group 4 — experiments/95-podman-vfs-ignorechown.sh

**What it measures.** Whether podman's vfs driver with
`ignore_chown_errors=true` opens a path where a layer chown fails, with the
chown wall staged rather than assumed.

**Pinned inputs.** `public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e8...4b6bc`
in the generated Dockerfile (`:98`). The staged wall is
`experiments/lib/chowndeny.py` (`:88`), carried as base64 over
`podman machine ssh`.

**Host assumptions.** `podman`, `base64`, `timeout` on PATH (`:69`); a
reachable `podman-machine-default` (`:75`); Git Bash path conversion disabled
(`:31`); `cygpath` for the one local path (`:106-110`).

**Exit-code contract.** `:200-201`: 0 or 1, never 2. The header at `:17-19`
states this deliberately and ties it to `TODO/image.md`'s `Prove`, which reads
"exits 0 or 1, never 2". Every failure path writes `verdict FAIL: engine host
unreachable` and exits 1.

**Timeouts.** `timeout 60` on `podman machine ssh`, `timeout 300` on build,
`timeout 240` on save/load, `timeout 120` on `rmi`, all via `timeout` at
`:43,45,75,113,127,138,140,150,166,182`.

**Cleanup.** `cleanup()` at `:40-48` removes the two machine stores, the image
and `$WORK`, under an `EXIT HUP INT TERM` trap. It also fires on the failure
paths that `exit 1` before the trap, which is correct.

**Positive control.** Clause 0 at `:112-134`: the same fixture must build and
print `hello` rootful before any wall is staged. If it does not, every later
clause is measuring a broken fixture.

**Failure control.** Clause A at `:146-159` is the negative arm: the same load
without the flag must be refused with the `lchown /data/greeting: operation not
permitted` signature. A load that failed for another reason fails the arm
(`:155-158`), so the instrument discriminates.

**TODO entries that own it.** T-0205 (`TODO/image.md:370-426`; `Prove:` at
:407, close at :409-426). The entry also corrects a corpus claim at
`TODO/image.md:381-399`.

**Result files written.** `experiments/results/podman-vfs-ignorechown.txt`.

**Verdict: RUST-TEST.**

### Conversion plan — 95-podman-vfs-ignorechown.sh

**Assertions today, as triples.**

1. Precondition: a rootful podman builds the fixture (a file owned by uid
   1234, which no subuid range maps) and runs it to `hello`. Action: build.
   Expected: exit 0 and output `hello`.
2. Precondition: the same save, loaded under `podman unshare` with a seccomp
   filter denying chown to any uid but the caller's, into a fresh vfs store
   with no `ignore_chown_errors`. Action: `podman load`. Expected: non-zero,
   with `lchown /data/greeting: operation not permitted` in the output.
3. Precondition: the same, with `[storage.options.vfs] ignore_chown_errors="true"`.
   Action: `podman load`. Expected: exit 0.
4. Precondition: the store from 3. Action: `podman run --rm --network=none`.
   Expected: exit 0 and output `hello`.

**Rust level.** **Fault test**, for the wall. `docs/conventions/code.md:28`:
"Use fault tests for conditions a real service cannot produce on demand." The
chown wall is exactly that; a normal machine applies gid 1234. The staging
mechanism is `experiments/lib/chowndeny.py`, a seccomp filter, and it has to
remain a host mechanism.

**Target file path and module name.** The subject is podman, not podbox. This
script measures a claim about the corpus that `TODO/extract.md:116-123` and
`TODO/image.md:381-399` both quote, and the repository's rule for a comparison
is in `docs/methodology/authoring.md:45` ("A comparison also needs a committed
measurement script"). A Rust test for a third-party engine's configuration
belongs beside the other audit drivers, not inside a podbox crate's `mod tests`.
Target: `experiments/395-reconcile-repository.py`-style placement is not right
either, since this is not a git fixture. The honest target is a new
`crates/podbox-image/tests/engine_option_matrix.rs`, marked `#[ignore]` by
default, because it needs a live podman machine. It asserts the two-arm
matrix and reports which engine answered, as `TODO/gate.md:961-962` requires
("Where host podman and a daemon would answer differently, the conditions block
names which one drove").

**Fixtures.** `experiments/lib/chowndeny.py` stays, carried to the machine the
way `:88` does it. The Dockerfile at `:97-101` becomes a fixture constant in the
test. The `podman machine ssh` transport stays; a Rust test still needs it,
because the machine is where podman runs.

**Plant.** Set `ignore_chown_errors="false"` explicitly in the clause-3 store
configuration. `TODO/image.md:425-426` records that this was measured ("a
control with the flag explicitly false fails as without it"). The test must go
red on clause 3. A second plant deletes the clause-2 signature match so the
negative arm passes on any failure, which is the vacuity `scripts/plant.sh`'s
header names (`:3-6`).

**Exact command.**

```sh
podman machine ssh podman-machine-default podman --version
cargo test -p podbox-image --test engine_option_matrix -- --ignored
```

Linux or macOS host with a podman machine. It cannot run in the WSL base, which
has no nested machine.

**What `exit 2` becomes.** The script has none, deliberately. The Rust test
turns the "engine host unreachable" states into an explicit third outcome: the
test panics naming the missing binary, following the rule
`crates/podbox-ssh/tests/common.rs:11-12` already applies ("A missing binary
fails loud and names the binary. A skip would read as green on a machine that
proved nothing"). This is the one case in the group where a `#[ignore]` is
wrong, because the entry's `Prove` forbids exit 2 precisely so the third state
cannot hide.

**What becomes of the result file.**
`experiments/results/podman-vfs-ignorechown.txt` stays and becomes the
current reading. Its conditions block
(`experiments/results/podman-vfs-ignorechown.txt:2-6`: podman client 6.1.2,
machine 5.8.6, subuid `user:524288:65536`) are the conditions a re-run must
match before its numbers replace these.

---

## Group 4 — experiments/270-multiarch-image.sh

**What it measures.** Five clauses about platform selection and the store's
key.
1. No `--platform` pulls the platform podbox was built for (`:130-148`).
2. `--platform` takes a different manifest from the same index, and both
   survive in the store as two records with two image IDs (`:150-177`).
3. The extracted rootfs really holds that architecture, read out of an ELF
   header inside the tree, not out of the metadata (`:179-246`).
4. An index offering nothing for the ask names what it does offer (`:248-263`).
5. A malformed `--platform` is a usage error, before any network (`:265-281`).

**Pinned inputs.** `ghcr.io/pkgforge-dev/archlinux:latest` by tag, not digest
(`:81`). The header at `:78-80` states why and prints the resolved digest so a
moved tag is visible.

**Host assumptions.** An engine, via `experiments/lib/engine.sh` (`:37-42`);
`file(1)` on PATH (`:90`); `jq` for `podbox_exit_codes` (`:99-102`); the binary
executable or readable (`:83-89`).

**Exit-code contract.** `:290-292`: 1 on any failure, then 2 on any skip, else
0.

**Timeouts.** `pb 900` on pulls, `pb 1800` on extract, `pb 60` on reads,
`pb 300`/`pb 120` on the refusals. Every call bounded.

**Cleanup.** `trap 'eng_cleanup; rm -rf "$WORK"' EXIT INT TERM` at `:31`.

**Positive control.** Clause 3 at `:181-184` is the control and the header says
so: "A record can say `linux/arm64` and hold amd64 bytes; this reads the ELF
header of a binary inside the extracted tree and asks the machine, not the
metadata." The saved run shows both words
(`experiments/results/multiarch-image.txt:20-21`: `x86-64` and `ARM aarch64`).

**Failure control.** Clause 4 at `:250-263` asks for a platform the index does
not carry, and requires the refusal to name what it does offer. A bare 404 is
a failure (`:260-262`).

**TODO entries that own it.** T-0212 (`TODO/image.md:1188-1288`; `Prove:` at
:1259, close at :1261-1282, the clause-5 correction at :1272-1282); T-0208
(`TODO/image.md:730-768`, whose close at :758 cites the same run); T-1212
(`TODO/gate.md:935-1037`; `Prove:` at :964, transcript at :993-1000).

**Result files written.** `experiments/results/multiarch-image.txt`.

**Verdict: SPLIT.** The script mixes a pure contract with a live registry
measurement and they have different owners and different futures.

### Conversion plan — 270-multiarch-image.sh, piece by piece

**Piece 1: platform parsing and selection. RUST-TEST, unit.**
Clauses 1 and 5's first half, plus everything `TODO/image.md:1222-1243` states.

- Precondition: a flag value, an environment pair, and the build host's
  platform. Action: `platform::select`. Expected: the flag wins; a bare word is
  an architecture; `armv6` and `armv7` stay distinct; an absent variant matches
  and a different one does not; `a/b/c/d` is refused by name.
- Level: pure. `docs/conventions/code.md:27`.
- Target: `crates/podbox-image/src/platform.rs`, `mod tests`. Seven tests
  already exist and cover the Approach sentences one for one:
  `a_bare_word_is_an_architecture_and_never_an_os` (`:292`),
  `armv6_and_armv7_do_not_collapse_into_one_platform` (`:312`),
  `an_absent_variant_matches_and_a_different_one_does_not` (`:326`),
  `a_malformed_platform_is_refused_by_name` (`:338`),
  `a_flag_beats_the_environment_and_the_environment_beats_the_host` (`:358`).
  What is missing is the one the script measures and the entry names at
  `TODO/image.md:1241-1243`: `select_platform` runs two passes, exact variant
  first, so an exact match beats a loose one appearing earlier in the index.
  Add `an_exact_variant_beats_a_loose_one_that_appears_earlier_in_the_index`.
- Fixtures: none beyond a synthetic index. An index is a list of platform
  strings; build the losing one first.
- Plant: make `select_platform` single-pass. `TODO/image.md:1242-1243` names the
  order that gets wrong. The new test must go red naming the loose earlier
  match.
- Command: `cargo test -p podbox-image platform`.

**Piece 2: the store's key. RUST-TEST, fixture.**
Clause 2's store half, without a registry.

- Precondition: one repository and tag committed under two platforms. Action:
  `Store::put_record` twice, then `find_for`. Expected: two records, two image
  IDs, both present; the tag's index digest is the same for both.
- Level: fixture. The store needs a directory and a lock, not a network.
- Target: `crates/podbox-image/src/store.rs`, `mod tests`, beside
  `two_platforms_of_one_tag_are_two_images_and_not_one` at `:2215`, which
  already exists. Add the index-digest half and the
  `find_for`-returns-hits-unfiltered half from `TODO/image.md:1245-1249`.
- Fixtures: the store scaffolding the crate already uses.
- Plant: drop the platform from the record key. The existing test goes red;
  record which.
- Command: `cargo test -p podbox-image two_platforms`.

**Piece 3: the live index and the extracted bytes. KEEP-SHELL.**
Clauses 1, 2's manifest half, 3 and 4 need a registry that publishes one tag
across eight platforms and an extracted tree to read an ELF header out of.

- The specific host dependency is `ghcr.io/pkgforge-dev/archlinux:latest`, which
  `TODO/image.md:1284-1288` says is part of the entry and not an accident
  ("ghcr has none", of Docker Hub's anonymous quota). No local fixture can
  supply eight platforms of one tag.
- So clauses 1 to 4 stay a script. The premise it records, that the default is
  the build platform and that `--platform` reaches a foreign one, is also what
  `docs/conventions/code.md:29` calls integration proof ("Use integration and
  deployment proof for the actual default path").
- Target: keep the file, narrowed to clauses 1 to 4. It is the
  `Prove` of `TODO/image.md:1259` and the `Prove` of `TODO/gate.md:964`.

**Piece 4: the result file.** The exit code. The `exit 2` path.

- `exit 2` arms today: no engine (`:42`), binary not executable (`:84-88`), no
  `file(1)` (`:90`), exit-code table unreadable (`:100-102`), pull did not
  complete (`:132-136`), no record to extract (`:187-191`), extract failed
  (`:194-198`), no known binary to read (`:222-226`). None is a statement about
  podbox's platform logic; all of them are a measurement that could not run. In
  Rust they become a test that returns early with a named
  `Measurement::CouldNotRun` value, or, for pieces 1 and 2, they do not exist at
  all because the input is a synthetic index.
- The result file `experiments/results/multiarch-image.txt` stays with piece 3.
  Its current values stay: `experiments/results/multiarch-image.txt:16-17` shows
  the two records and the shared index digest at line 14, which is what
  `TODO/image.md:1210-1219` quotes.

**Corrections the implementor must carry.**

1. `:266` prints "a malformed `--platform` is a USAGE error". The entry
   corrected that at `TODO/image.md:1272-1275` ("125 is what a flag parser
   refuses, 1 is what the verb refuses afterwards"). The assertion at `:276` is
   already right; the heading is stale. Fix the heading, or drop clause 5 from
   the script and let `an_malformed_platform_is_refused_by_name` in piece 1 own
   it. Prefer the latter; the entry says the two are the same shape
   (`TODO/image.md:1274-1276`).
2. `TODO/gate.md:945-947` names `270` as one of the scripts the entry converted,
   and the conversion is real (`experiments/270-multiarch-image.sh:37-42` uses
   `engine.sh`). Keep it.

---

## Group 4 — experiments/300-run.sh

**What it measures.** Eight clauses about `podbox run` and `podbox exec` for a
caller that knows docker.
1. stdout is the payload's and nothing else; the banner is on stderr (`:132-145`).
2. the exit code is the payload's, unaltered, and 128+signal for a signal
   (`:147-172`).
3. it is a chroot into the image, not the host (`:174-185`).
4. `-e`, `-w`, `--entrypoint` and bare-name PATH resolution (`:187-202`).
5. a foreign image runs through a registered interpreter or is refused by name
   (`:204-276`).
6. `--pull never` on a platform the store does not hold names what it does hold
   (`:278-290`).
7. inside the reconstruction, the chroot rung is selected (`:292-344`).
8. `exec` is a fresh chroot, stated on stderr, in `inspect`, and it never pulls
   (`:346-393`).

**Pinned inputs.** `ghcr.io/pkgforge-dev/archlinux:latest`
(`:48`, `PODBOX_RUN_IMAGE`).
`container-research/target:1` for clause 7 (`:308`).

**Host assumptions.** An engine (`:57-58`); `binfmt_misc` mounted and
`/usr/bin/qemu-aarch64-static` present for clause 5's first half (`:207-212`);
a docker daemon and a built target image for clause 7 (`:305-311`); `jq` for
the exit-code table (`:109-112`); the podbox binary readable or executable
(`:65-100`).

**Exit-code contract.** `:402-404`: 1 on any failure, then 2 on any skip, else
0.

**Timeouts.** Per call: `pb 2400` for pulls and runs of foreign images, `pb
900` for the rest, `pb 300` for inspect, `pb 60` for version, `pb 120` for the
shim. The binfmt registration is torn down in `cleanup` at `:40-46`.

**Cleanup.** `trap cleanup EXIT INT TERM` at `:46`. It unregisters the binfmt
entry, calls `eng_cleanup` and removes `$WORK`.

**Positive control.** Clause 8's paired read at `:364-373`: the banner on
stderr and `inspect --format '{{.Exec.Mode}}'` must name the same mode, read
from one pair of constants. `TODO/enter.md:358-363` records that a unit test
asserts the same pairing and that clause 8 asserts it on the shipped binary.

**Failure control.** Clause 5's second half (`:242-275`) is a refusal arm: an
architecture nothing executes must exit 125 and name the host platform. It is
a real negative control for the interpreter half.

**TODO entries that own it.** T-1104 (`TODO/milestones.md:269-310`; `Prove:`
at :285, close at :287); T-0505 (`TODO/enter.md:339` `Prove:`, close at :341);
T-0506 (`TODO/enter.md:378-449`; close at :436-438 names clause 5); T-0502
(`TODO/enter.md:159-166`, which names clauses 4 and 7); T-1212
(`TODO/gate.md:935-1037`; `Prove:` at :964, transcript at :1026-1037).

**Result files written.** `experiments/results/run.txt`.

**Verdict: SPLIT.** The eight clauses have three different natures and three
different owners.

### Conversion plan — 300-run.sh, clause by clause

**Piece 1: clauses 1, 2, 4, 6, 8's exit codes. RUST-TEST, unit.**
The pure claim: the payload owns stdout, the exit code is the payload's, and
the banner is on stderr.

- Precondition: a payload that writes `hi` to stdout and a banner to stderr.
  Action: `podbox run`. Expected: stdout is exactly `hi`, stderr carries
  `mode=`, exit 0.
- Precondition: payloads exiting 0, 1, 42, 137 (SIGKILL), 127 (not found).
  Action: `podbox run`. Expected: each code is the payload's own.
- Precondition: `exec` with no command. Action: `podbox exec`. Expected: the
  cli-error code, never the image's `Cmd`.
- Level: **fault** for the signal and not-found rows, pure for the rest.
  `docs/conventions/code.md:28` names fault tests for "conditions a real service
  cannot produce on demand"; a SIGKILLed payload and a missing binary are
  arranged by the test.
- Target: `crates/podbox-cli/src/run.rs` and `crates/podbox-cli/src/exec.rs`,
  inline `mod tests`. `crates/podbox-cli` has 28 files with `cfg(test)` and no
  `tests/` directory, so inline is the house shape here.
- Fixtures: none new. The child-spawning path is the one to fixture; the
  existing `crates/podbox-ssh/tests/common.rs` fixture style
  (`require_binary` at `:28`, `fresh_tmp` at `:46`) is the pattern if a
  subprocess is needed.
- Plant: make the run verb write its banner to stdout instead of stderr. Clause
  1 goes red with "the banner reached stdout", which is the check's own
  message at `:145`. A second plant swaps the `128 + signal` arithmetic for
  `1` and watches the 137 row go red.
- Command: `cargo test -p podbox-cli run` and `cargo test -p podbox-cli exec`.

**Piece 2: clause 5 and clause 6. KEEP-SHELL.**
Clause 5 writes a binfmt registration into `/proc/sys/fs/binfmt_misc/register`
and reads the kernel's answers. The specific host dependency is a writable
`binfmt_misc` and `qemu-aarch64-static`. `TODO/enter.md:446-447` states the
rule: "podbox reads the registrations and never writes one. Registering an
interpreter is a machine-wide change and podbox is not the machine's owner."
Clause 6 needs a store holding a different platform than the one asked for,
which is piece 2 of the 270 plan. Both stay.

**Piece 3: clause 7. KEEP-SHELL, and this is the strongest case in the group.**
The specific host dependency is `experiments/20-enter-target.sh` and the
reconstruction `container-research/target:1`, where `mount(2)` is EPERM and
podbox must fall to the chroot rung. `TODO/milestones.md:301-305` states why
nothing else can stand in: "A run that selected `namespace` in both would have
proven nothing about `chroot`." `crates/podbox-enter` tests the ladder
selection with a fake rung, which is a different statement.

**Piece 4: clause 3. DELETE as a clause.**
`:177-185` compares the image's `/etc/os-release` `$NAME` with the host's and
warns when they match. The saved run shows they matched
(`experiments/results/run.txt:22-25`: image `Arch Linux`, host `Arch Linux`,
"so this clause cannot distinguish them. It asserts only that the file was
readable"). The clause is satisfied by a run that printed nothing. Either pin
the image to a distribution the host is not, or drop the clause. Do not carry
it forward as it stands.

**Piece 5: the exit-2 path and the result file.**
- `exit 2` arms: no engine (`:58`), binary unreadable (`:65-68`), driver inputs
  could not be staged (`:75`), binary not executable (`:94-100`), exit-code
  table unreadable (`:110-112`), clause 5's `binfmt_misc`/qemu/binfmt
  registration (`:207-225`), clause 5's riscv64 branch (`:258-260`), clause 7's
  four arms (`:305-317`). Every one is "the measurement could not run" and none
  becomes a Rust assertion. In pieces 1's tests they do not exist; in pieces 2
  and 3 they remain the script's.
- `experiments/results/run.txt` stays with the script. Its conditions block
  (`:2-7`, kernel `7.2.0-WSL2-STABLE`, engine 29.8.1) are the conditions a
  re-run must match.

**Corrections the implementor must carry.**

1. T-1212's `Prove` at `TODO/gate.md:964-968` requires all six scripts to exit
   0; the transcript at `:1037` records `EXIT:2` for this one, and
   `:978-980` explains it as a SKIP. Rewrite the `Prove` to name the third
   state, or the entry cannot be re-run and pass as written.
2. `experiments/results/run.txt:37` records the riscv64 refusal half as
   unmeasurable on that host. `TODO/enter.md:442-444` records the arm as
   passing on a host where nothing is registered. The clause is
   host-conditional and the script already says so at `:242-250`; keep that
   text when the script is narrowed.

---

## Group 4 — experiments/245-interpose-sweep.sh

**What it measures.** Across ten distribution rows: does a payload needing
virtualized ownership succeed, and does podbox pick the right object without
being told. Per row, two columns for T-0712 (`seen` and `cleared`, with the
chown target held at gid 42 and unmapped on every row), plus a static victim
and a Go victim, both declined by name, plus the selection asserted on bytes,
plus the two cross-libc refusals driven through `podbox system abi`.

**Pinned inputs.** `DISTRO_ROWS_M5` from `scripts/common/distro-matrix.sh:74-85`,
ten rows each with a digest. The driver image
`public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe...4f0a` (`:90`). The
Go victim is generated, not fetched, by `experiments/src/govictim.sh` (`:179`),
and its size is asserted at 384 bytes (`:181-182`).

**Host assumptions.** Native Linux or a non-native lane with an engine (`:56-86`);
the binary executable or readable (`:134-143`); both interposer objects built
(`:129-130`, and `:445-447` handles their absence as `unbuilt-objects`); an
engine for the attribution control (`:82`, `:486-488`).

**Exit-code contract.** `:579-583`: `distro_verdict` returns 0 or 1, then
`refuse_fail` returns 1, else 0. Exit 2 is used at `:85,137,141,162,180,182,264`.

**Timeouts.** `ROW_TIMEOUT` 600 s default (`:52`), an outer bound of
`ROW_TIMEOUT + 900` for a staged row (`:97`), and `timeout 900` on the native
pull (`:320`).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at `:72`. `eng_clear` at
`:499,511`. `eng_run` and `eng_mount` clear their own.

**Positive control.** The engine control at `:482-504`: a row the payload did
not clear is attributed by running the same subject under the engine on the
same image. If the engine succeeds where podbox failed, the failure is
podbox's; if the engine fails too, it is the host's. The header at `:465-481`
states the rule and the static-decline exception.

**Failure control.** The Go victim at `:401-413` requires a named decline AND
`grc -eq 126`, so a victim that is declined for the wrong reason fails. The
static victim at `:384-394` requires the decline text and `src -eq 0`. Both
are negative arms with their own subjects.

**TODO entries that own it.** T-1110 (`TODO/milestones.md:698-722`; `Prove:`
at :698, close at :703-713); T-0712 (`TODO/interpose.md:1227` `Prove:`, close at
:1230-1236); T-1212's sweep, cited at `TODO/gate.md:799,828,846`.

**Result files written.** `experiments/results/interpose-sweep.txt` and
`experiments/results/sweep245/<row>.out`, ten files.

**Verdict: KEEP-SHELL.**

**Justification, with the specific host dependency.** This script measures a
matrix of ten real distributions against real interposer objects, and three
things in it have no Rust substitute:
1. A glibc interposer object and a musl interposer object must be **built by
   the host toolchain** (`crates/podbox-interpose` targets, `:129-130`). The
   selection assertion compares the placed bytes against them
   (`:425-444`).
2. The payload's `chown 0:42` must be **refused by the kernel** so the
   interposer is the only path that clears it. The script builds that wall
   with `--cap-drop=CHOWN` on the driver (`:94`), and the header at `:91-93`
   says why: "The driver is rootful, so a payload chown would succeed natively
   and every row would clear for no reason."
3. The Go victim must **fail to exec** as `ET_REL` (`:401-403`) and the static
   victim must be declined by the interposer at runtime. Both are properties of
   the built objects against a real loader, not of podbox's parsing.

`docs/conventions/code.md:29` names this kind of proof: "Use integration and
deployment proof for the actual default path." The default path here is ten
images podbox does not ship.

**What the implementor must not do.** Do not replace this with a unit test of
`podbox system abi`. `abi.rs` already unit-covers the version refusal, and
`:555-556` says so: "the build-host-newer-than-target refusal is unit-covered
in abi.rs; no matrix row is old enough to fire it, and inventing one is
refused." The matrix arm is the part with no unit equivalent.

**Corrections the implementor must carry.** `TODO/milestones.md:703-704` says
"10 rows, 10 ran, 10 virtualized, 20 declined, 0 host_not_runtime". The saved
run says 9 ran and 1 no-pull
(`experiments/results/interpose-sweep.txt:3-8`: `rows 10`, `ran 9`,
`virtualized 9`, `declined 18`, `no-pull 1`, `broken 0`). The transcript
identifies the row: `experiments/results/sweep245/archlinux.out:1` reads
`driver stage 4 for ghcr.io/pkgforge-dev/archlinux@sha256:b2507f19...`, and
`driver stage 4` is the `extract` failure arm at `:290-293`. The entry's
sentence is wrong about the run its own `Prove` cites, and it was correct for
an earlier run.

---

## Group 4 — experiments/353-open-issue-triage.sh

**What it measures.** Not a measurement. It is a **triage drive**: it runs the
cited command for 22 open issues and prints the command, the output and the
exit code. Its own header at `:2-4` says "which of the 22 open issues reproduce
on this lane, with what command, output and exit code?"

**Pinned inputs.** `alpine:3.20@sha256:d9e853e8...4b6bc` (`:31`).

**Host assumptions.** A Linux lane that runs the job at `/in/job.sh` with
`/work` as the working directory, taken from `pwd` not `$0` (`:22-25`).
`cargo`, then `./scripts/common/bootstrap-env.sh rust cc zig tools` (`:42-50`),
then `cargo build` (`:52-58`).

**Exit-code contract.** 0 when the drive completed, 1 when the binary could not
be built, 2 when the lane could not run (`:18-19`). The final `exit 0` at
`:184` is unconditional, which is correct for a drive: no clause is asserted,
so nothing can fail.

**Timeouts.** `timeout 1200` on bootstrap, `timeout 1800` on the build,
`timeout 120` on every case (`:44,52,69`).

**Cleanup.** No trap. `$WORK` is `experiments/.sweep353-work` and is removed at
the start (`:27`) and never at the end. The report is copied to `/out` if it
exists (`:182`).

**Positive control.** Clause `pull` at `:84` runs before every image-dependent
clause, so a case that fails on a missing image is visible.

**Failure control.** None, and none is possible: the script asserts nothing.

**TODO entries that own it.** No entry owns it. It is cited as a premise by
six: `TODO/cli.md:142` and `:1280` (T-1337), `TODO/complete.md:1164`,
`TODO/image.md:882`, `TODO/podvm.md:143`, `TODO/supervise.md:282` and `:449`.
`TODO/cli.md:1258-1267` is T-1337, the entry the script's header at `:5-7`
claims was "authored beside this script". It names the 2026-09-25 triage of
issues 29-38 and 49-60 plus dependabot PR 9, and its `Source:` is
"issue 38, beta.7 drive 2026-09-25", not this script.

**Result files written.** `$WORK/out/triage-353.txt`, copied to
`experiments/results/triage-353.txt` by the caller. The tracked file is
124278 bytes.

**Verdict: KEEP-SHELL, and it is the clearest case in the group for it.**

**Justification, with the specific host dependency.** Two hard dependencies
no Rust crate replaces:
1. `experiments/src/govictim`-style lane setup: the script bootstraps a toolchain
   on a fresh Linux lane before it can build anything (`:44`). That is the
   `wsl-toolkit` lane, not a cargo test environment.
2. It measures **22 open issues against a beta binary on a lane**, and the
   result is a 124 KB transcript whose value is the exact output a reader needs
   to reproduce. Its six citing entries quote it clause by clause, for example
   `TODO/cli.md:1280-1283` naming clauses `38-system-help` and `38-logs-help`.

Beyond that, a Rust rewrite would be a different thing: it would assert, and a
triage drive must not. Converting its cases to assertions would delete the
evidence six entries rest on.

**What the implementor must do.** Leave it. If the operator wants the drives
gone, this one is the last candidate to argue about, and the argument is that it
produces premises rather than verdicts.

**Correction the implementor must carry.** The header at `:5-7` says "TODO
triage entry T-1337 (authored beside this script)". T-1337 is
`TODO/cli.md:1258`, a `cli` entry about doctor, df and log tail, and it names
issue 38 as its `Source:` (`TODO/cli.md:1260`). It is not a triage entry, and it
does not name this script. Correct the header to name the six entries that cite
the transcript.

---

## Group 4 — experiments/369-windows-tcg-dos.sh

**What it measures.** On a KVM-less lane, does a DOS guest run one command
under TCG, return its stdout and its exit code, leave the base untouched, and
discard the per-run overlay. Nine clauses, listed at `:16-30`.

**Pinned inputs.** The FreeDOS 1.4 LiteUSB zip, 17 MB, sha256
`857dcd2e...38b`, raw image 33554432 bytes (`:42-44`). The URL is
`https://download.freedos.org/1.4/FD14-LiteUSB.zip`.

**Host assumptions.** `cargo`, `cc`, `unzip`, `jq` on PATH (`:69-71,80`);
`./scripts/common/bootstrap-env.sh rust cc zig tools qemu` (`:73`);
`./scripts/build-interpose.sh` (`:82`); 512000 free blocks on `$WORK` (`:131`);
`/dev/kvm` present or absent is recorded, not required (`:51`).

**Exit-code contract.** 0 when every clause matched, 1 when one disagreed, 2
when the lane could not run (`:32-33`). The last line `:259` is
`[ "$fail" -eq 0 ]`, so a build failure at `:90-98` exits 1, not 2. The header
at `:32-33` says 2 is "no toolchain, no build, no fetch, no jq", and the build
path does exit 1 at `:95-97`. The header and the code disagree on which state a
build failure is.

**Timeouts.** `timeout 1200` on bootstrap and interpose, `timeout 1800` on the
build, `timeout 180` wrapping `curl --max-time 120` (two ceilings, `:134`),
`timeout 120` on unzip and probe, `timeout 300` per guest run (`:173`, with
the reason at `:164-168`), `timeout 120` per refusal step (`:228`).

**Cleanup.** No trap. `$WORK` is removed at the start (`:39`) and never at the
end. `PODBOX_DOS_BASE` points into it. The report is copied to
`experiments/results/windows-tcg-dos.txt` at `:256` and to `/out` if present.

**Positive control.** Clause 0 at `:112-127`: the machine profile must be `tcg`
or `full` with refusal `null`, or nothing after it can mean anything.

**Failure control.** Clause 5 (`:199-201`) is the non-zero arm, `dir /zzz`
must exit 1, distinct from 0. Clause 6 (`:203-206`) is the deadline arm,
`pause` must exit `$RUN_ERR` and say `DEADLINE` and `no status`. Clause 8
(`:224-249`) is three refusals. All three are negative arms with subjects the
script controls.

**TODO entries that own it.** T-1112 (`TODO/milestones.md:848-894`; `Prove:`
at :867 names this script first, and the entry is `partial` at :854). The
entry's partial note at `:874-876` says "Earlier tracked results prove FreeDOS
under TCG and ValidationOS under TCG", which is this file and
`experiments/370-windows-guest.sh`.

**Result files written.** `experiments/results/windows-tcg-dos.txt`.

**Verdict: KEEP-SHELL.**

**Justification, with the specific host dependency.** The specific dependency
is `qemu-system-x86_64` in TCG mode. Every clause after 0 runs a real emulator
against a real 32 MiB FreeDOS image: the guest boots, FreeCom answers, a
`DEADLINE` is enforced by the driver reading a completion line through a QMP
mailbox (`:203-206`, and the banner in the saved run at
`experiments/results/windows-tcg-dos.txt:59`), and the base image's sha256 is
compared before and after (`:209-214`). A Rust test can prove
`podbox_windows::run` does not block on the emulator's pipes
(`TODO/milestones.md:881-887` records that defect and its unit test), and
`crates/podbox-windows` has 7 files with `cfg(test)`. It cannot prove a FreeCom
version string reached stdout.

**Corrections the implementor must carry.**
1. `:32-33` versus `:95-97`: the header says a build failure is 2, the code
   exits 1. `docs/methodology/experiments.md:14` says "A missing observation is
   not a denial or a zero"; a build failure is closer to 2. Fix the header or
   the code, and say which.
2. `:32-33` says the script is a measurement of the DOS route, and
   `TODO/milestones.md:881-887` records that the piped-stream defect "can
   explain the exit-42 timeout; no guest run has confirmed it". This script does
   not test exit 42. It tests 0, 1 and 125 (`:190,200,204`). The `Prove` at
   `TODO/milestones.md:867` lists it beside `392-kvm-guest.sh`, which does.
3. The saved run records the binary as `podbox 0.1.0-beta.7`
   (`experiments/results/windows-tcg-dos.txt:11`) and the date as
   2026-09-27, while the entry's partial note is dated 2026-09-30. The result
   stays historical; `docs/methodology/experiments.md:30` says a historical
   result remains historical after a new build.

---

## Group 4 — experiments/382-restricted-sshd.sh

**What it measures.** Whether a real SSH server runs a real command in a host
where chroot is denied and no passwd database exists, on the compiled shim.
Eight clauses, `:20`.

**Pinned inputs.** `crates/podbox-ssh/shims/fakepwd.c`, the retained shim
(`:79`). Keys are minted fresh in the work dir (`:163-167`). The script's
header at `:16-18` states "No secret is involved: the keys are minted fresh in
the work dir and no token exists on this path."

**Host assumptions.** `/work` as the working directory (`:23`); `gcc`, `sshd`,
`ssh`, `ssh-keygen`, `readelf`, `timeout` on PATH (`:48-54`); a
privilege-separation user `sshd` (`:60-69`); a glibc-dynamic lane
(`:89-94`) and a dynamically linked `sshd` (`:135-141`); `setpriv` or a
non-root uid for the chroot denial (`:113-125`); a free loopback port from
22180 (`:174-212`).

**Exit-code contract.** 0 every clause held, 1 a clause disagreed, 2 the lane
could not run (`:20`). The last line `:320` is `[ "$fail" -eq 0 ]`.

**Timeouts.** `timeout 120` on the daemon (`:194`), `timeout 60` on the good
SSH command (`:230`), `timeout 30` on the refused one (`:251`), `timeout 15` on
the post-mortem port check (`:297`). Clause 5 asserts the refusal finished
inside 30 s (`:258`).

**Cleanup.** `trap 'cleanup' EXIT HUP INT TERM` at `:34`. `cleanup` kills the
daemon and removes `$work` (`:28-33`). Clause 7 asserts the work dir is gone
(`:305-312`), so the trap has nothing left to do and the check is meaningful.

**Positive control.** Clause 4's negative half at `:219-227`: `cageuser` must
be absent from the host database, so the login can only have resolved through
the preloaded shim. The header at `:14-16` states the reason.

**Failure control.** Clause 5 at `:245-264`: a wrong key must be refused with
255 inside the bound while the daemon stands. "a hang here is a missing bound,
not a refusal" (`:245-246`). Clause 6 at `:266-275` is the host-identity
control: same hostname, same `/etc/passwd` checksum.

**TODO entries that own it.** T-1401 (`TODO/podssh.md:16-65`; the `Prove:` at
:38-42 requires "a tracked fresh-clone drive must run a real SSH command and
return 42 in the restricted host", and the close at :44-56 names this script
and its verdict).

**Result files written.** `experiments/results/restricted-sshd.txt`, and a
copy to `/out` (`:318`).

**Verdict: KEEP-SHELL.**

**Justification, with the specific host dependency.** Three dependencies, none
replaceable:
1. **A real `sshd` binary and a real `ssh` client.** The header at `:16-17` is
   explicit: "every SSH byte here moves between a real ssh client and a real
   sshd." The script's clause 3 (`:135-141`) exists because "A static sshd
   cannot take LD_PRELOAD", and the header at `:7-10` points at
   `docs/decisions/ssh-server-in-a-cage.md` for that dead end.
2. **A C compiler invoked on the retained shim.** Clause 1 is a build step:
   `gcc -shared -fPIC -O2 -o "$work/fakepwd.so" crates/podbox-ssh/shims/fakepwd.c`
   (`:79`). `TODO/podssh.md:44-46` calls this "the tracked drive that compiles
   it" and the close at :55-56 states "no workspace or release build links the
   shim". Turning that into a cargo buildscript would change what the entry
   proves, which is that the shim is a build input a fresh clone can compile.
3. **A host where `chroot` is denied and no passwd database exists.** Clause 2
   at `:113-131` stages this and fails the drive if it cannot.

The one piece that is a candidate for Rust: the fixture assembly (passwd file,
authorized_keys, sshd config, port probing) is deterministic. But
`crates/podbox-ssh/tests/common.rs` already has this shape for podbox's own
binaries (`require_binary` at `:28`, `fresh_tmp` at `:46`, `CLIENT_DEADLINE` at
:24), and adding a fourth suite there would need a real `sshd` on PATH, which
`require_binary` would then panic on in CI. The honest answer is a fourth
`crates/podbox-ssh/tests/restricted_sshd.rs` suite that reuses `common.rs`, and
that suite is a Rust re-expression of this script. Recommend it as a follow-on,
not as this script's verdict, because the C shim build and the chroot-denial
staging stay shell either way.

**Correction the implementor must carry.** `TODO/podssh.md:44-53` cites the
result file and the verdict line. The result file
(`experiments/results/restricted-sshd.txt:30`) reads
`RESTRICTED SSHD SERVES EXIT 42 ON THE COMPILED SHIM`, and the entry's `Prove:`
at :38-42 is satisfied. No contradiction. Note instead that the entry's
`Prove:` also requires `cargo test -p podbox-ssh` and
`sh scripts/common/check-gate.sh --strict` to exit 0, which this script does not
run; those are separate clauses and the entry records both at :54-56.

---

## Group 4 — experiments/95-podman-vfs-ignorechown.sh

See the full entry above. Verdict: RUST-TEST.

---

## Group 4 — scripts/document-state.py

**What it measures.** It does not measure a runtime behaviour, and says so in
its own docstring at `:2`: "Generate the source state page. T-1348. This does
not prove runtime behavior."

What it does: renders `docs/runtime-state.md` from `Cargo.toml`,
`Cargo.lock`, and three `pub enum` declarations in the source (`:49-53`), and
compares the rendered text against the tracked file.

**Pinned inputs.** `Cargo.toml` workspace members (`:28`), each member's
dependencies (`:32`), `Cargo.lock` versions (`:62`), three enums
(`crates/podbox-probe/src/select.rs` `Rung`, `crates/podbox-cli/src/tier.rs`
`Tier`, `crates/podbox-enter/src/ladder.rs` `Mode`), and the SSH helper sources
`crates/podbox-ssh/src/bin/*.rs` (`:57`).

**Host assumptions.** Python 3.11 or later, for `tomllib` (`:8`). Windows:
`py`; Linux: `python3` (`:40-41`).

**Exit-code contract.** `:74-86`: 0 when the snapshot matches or is written, 1
when it differs, 2 on `OSError, ValueError, KeyError`. The message is printed
to stderr at `:79`.

**Timeouts.** None. It is bounded by the files it reads.

**Cleanup.** None needed. It writes one file, and only under `--write`.

**Positive control.** `:62-64`: a dependency with no registry lock entry raises
rather than rendering a blank, so a lock that lost an entry is a failure and
not a shorter page.

**Failure control.** `:15-23`: an enum that cannot be read, or that parses to
no values, raises. The empty list is an error, not an empty rendering.

**TODO entries that own it.** T-1348 (`TODO/gate.md:1730-1748`; `Prove:` at
:1742 is `python3 scripts/document-state.py` and `sh scripts/plant.sh` exit 0
"a changed snapshot fails with its own message"; close at :1744-1748).

**Result files written.** `docs/runtime-state.md` under `--write`. It is
checked, not written, otherwise.

**Verdict: RUST-TOOL.**

### Conversion plan — scripts/document-state.py

**Assertions today, as triples.**
1. Precondition: the tracked `docs/runtime-state.md`. Action: render from the
   manifests and the three enums. Expected: byte-identical, exit 0.
2. Precondition: `docs/runtime-state.md` edited. Action: same. Expected: exit 1
   with `source state differs; regenerate docs/runtime-state.md`.
3. Precondition: a manifest unreadable, an enum renamed, a lock entry absent.
   Action: same. Expected: exit 2 with `cannot read source state:`.
4. Precondition: `--write`. Action: same. Expected: the file is written and
   `document-state: wrote docs/runtime-state.md` is printed.

**Rust level.** **Pure/unit**, for the rendering and the comparison, plus a
**fixture** for the parse of the manifests. The input is the repository's own
files, so the test reads the same tree the operator edits. That makes it a pure
test over a real fixture, which is `docs/conventions/code.md:27`.

**Crate, binary name, CLI surface.**

- New crate `crates/podbox-docs`, binary `podbox-docs`, with one subcommand.

  ```
  podbox-docs state            # compare docs/runtime-state.md against the tree; 0 match, 1 differ, 2 could not read
  podbox-docs state --write    # regenerate it
  ```

  Exit codes are the script's own three, carried unchanged. A `[[bin]]` in
  `crates/podbox-docs/Cargo.toml` named `podbox-docs`, added to the workspace
  `members` in `Cargo.toml`, which `scripts/document-state.py:28-30` reads, so
  the new member appears in the generated page on the first run.

- Dependencies: the crate needs a TOML parser and `sha2` or nothing. The
  workspace already locks both, and `scripts/document-state.py:33-34` filters
  registry dependencies by the presence of a `path` key, so a path-only
  dependency does not appear in the generated dependency table. Read the
  current `Cargo.lock` for the TOML crate name before writing the manifest; the
  entry at `TODO/extract.md:483-484` records that `sha2` already joined
  `podbox-extract` "already ruled under T-0908", so it is available.

**Module names.** One module per input, so a new input has a home:
`crates/podbox-docs/src/state.rs` (the renderer), `workspace.rs` (manifests and
the lock), `enums.rs` (the three `pub enum` readers), `helpers.rs` (the
`crates/podbox-ssh/src/bin/*.rs` listing). Unit tests inline in each, matching
every other crate in the tree. Add `crates/podbox-docs/src/lib.rs` so the
renderer is testable without the binary.

**Fixtures.** The repository itself, plus two small in-memory strings for the
enum reader: a `pub enum` that parses, and one that parses to nothing. The
second is the positive control at `scripts/document-state.py:19-21` and must
fail.

**Plant.** This is the one Group 4 script that already has a plant, and it is
in the right place. `scripts/plant.sh:497-498` is
`case_plant "31 source state differs" "source state differs"`, which appends
`Stale generated state.` to `docs/runtime-state.md` and asserts the check's own
message. The Rust tool must keep that message, because the plant asserts on it.
`check-todo.py:1698-1709` is the caller: it runs the script through
`subprocess.run` with `sys.executable` and a 30 s timeout, and turns a non-zero
status into `source state differs: <stderr>`. The Rust tool must be invoked
from there instead, and the message must survive the change of language. Add
the tool's path to the `FILES` list at `scripts/plant.sh:53` so the guard's
single-file-list rule holds.

**Exact command.**

```sh
cargo run -p podbox-docs -- state
cargo test -p podbox-docs
```

Then the two callers:

```sh
py scripts/check-todo.py
sh scripts/plant.sh
```

No feature flags, no target triple. It must build on the Windows host, because
`check-todo.py` runs there and `scripts/document-state.py:40` names `py` on
Windows.

**What `exit 2` becomes.** Unchanged, and this is the one case where that is
right. `check-todo.py:1708-1709` already handles the failure as
`source state could not run: {exception}` and does not fold it into the
mismatch message, so the three states survive a language change. The Rust
binary's `main` returns `ExitCode::from(2)` on an IO or parse error with the
error on stderr, and `check-todo.py` keeps its own two branches.

**What becomes of `docs/runtime-state.md`.** It stays and stays generated. Its
home is named at `docs/conventions/prose.md:39` ("The source-state generator
owns its generated page") and that sentence must be updated to name
`crates/podbox-docs` in the same change. Three other documents cite the script
as the home: `docs/agent-tooling.md:32`, `docs/code-map.md:25`, and
`scripts/README.md:11`. All four need the new path. The generated page's own
line at `docs/runtime-state.md:7` says "Regenerate with `py scripts/document-state.py
--write` on Windows", and it is part of the rendered text at
`scripts/document-state.py:40`, so it changes with the tool.

---

## Group 4 — scripts/release-notes.sh

**What it measures.** Not a measurement. It is a **release-notes generator**:
it prints the notes the nightly workflow attaches to a release, carrying the
build commit, that commit's gate conclusion, and the reproducibility boundary
(`:6-12`).

**Pinned inputs.** `TAG` from argv (`:20`). The gate run for that commit, read
back through `gh` (`:30-33`).

**Host assumptions.** `gh` on PATH (`:22`) and the network; a git repository
that can resolve `$TAG^{commit}` (`:25`). `REPO` defaults to
`Azathothas/podbox` and is overridable by `GH_REPO` (`:24`).

**Exit-code contract.** `:17`: 0 the notes printed, 2 the exact-commit gate or
a required read absent. There is no 1. The header at `:13-15` says why: "could
not run" is the third state everywhere in this tree, "never a pass and never a
failure".

**Timeouts.** `timeout 120` on the `gh run list` (`:30`).

**Cleanup.** None.

**Positive control.** The gate conclusion must be `success` for the **exact**
commit (`:34-37`): `success\ *)` is accepted, everything else exits 2. A note
generated from a failed or absent gate does not publish.

**Failure control.** `:53` checks that the tagged commit carries
`scripts/package-ssh.sh` before adding the SSH-helper paragraph, so a release
without the helper archive does not claim it has one.

**TODO entries that own it.** T-1334 (`TODO/packaging.md:562-608`; the close at
:602-608 says "The current notes step refuses publication without a successful
main gate on the exact build commit"). Also cited at
`TODO/interpose.md:1532`, in a comment about attribution. The consumer is
`.github/workflows/nightly.yml:179-189`.

**Result files written.** None. It writes to stdout, redirected by the workflow
to `release-notes.txt` (`.github/workflows/nightly.yml:179`).

**Verdict: RUST-TOOL.**

### Conversion plan — scripts/release-notes.sh

**Assertions today, as triples.**
1. Precondition: `TAG` given, `gh` present, a commit for the tag, and a
   successful `gate` run on `main` whose head SHA is that commit. Action:
   resolve and print. Expected: the notes on stdout, exit 0.
2. Precondition: the tag resolves but no successful main gate carries that
   commit SHA. Action: same. Expected: exit 2 with
   `release-notes: no successful main gate for the exact build commit`.
3. Precondition: `gh` absent, or the runs cannot be read. Action: same.
   Expected: exit 2, naming which.
4. Precondition: the tagged commit has `scripts/package-ssh.sh`. Action: same.
   Expected: the SSH-helper paragraph and the limits link are appended.
5. Precondition: no `TAG`. Expected: exit 2 with the usage line.

**Rust level.** **Integration**, for the gate read, because it is a live API
call to a service. `docs/conventions/code.md:29` names integration proof for
the actual default path, and `code.md:33` warns that "A mock does not prove a
live service or deployment". The note *formatting* is pure and takes a unit
test; the gate read is the part that needs a real service.

**Crate, binary name, CLI surface.**

- New crate `crates/podbox-release`, binary `podbox-release`.

  ```
  podbox-release notes --tag TAG [--repo OWNER/NAME] [--format text|json]
  ```

  `--format json` is the addition the conversion earns: the workflow pipes the
  text form straight to `gh release create`, so the text form must stay
  byte-compatible with what the current script prints, and a JSON form is what a
  later check would read by field name, which `docs/conventions/code.md:23`
  requires ("Structured output is read by field name"). Keep the text form the
  default.

  Exit codes: 0 printed, 2 could not run. No 1, matching the script and its
  header at `:13-15`.

- Replace `.github/workflows/nightly.yml:179` with
  `cargo run --release -p podbox-release -- notes --tag "${{ github.ref_name }}" > release-notes.txt`.
  The workflow needs a cargo toolchain at that step; read
  `.github/workflows/nightly.yml` around line 179 for what the step already
  sets up before assuming a runner change.

**Module names.** `crates/podbox-release/src/notes.rs` (the text renderer,
pure), `gate.rs` (the `gh` call and its parse), `commit.rs` (`git rev-parse`
equivalent, or a direct call to the API), `main.rs` (argv, exit codes).
Inline unit tests, matching the tree.

**Fixtures.** None exist and none are needed for the renderer: three strings
covering a successful gate, a failed gate, and a missing gate, which are the
three branches of `:34-37`. For the `gh` call, a recorded JSON response, which
`docs/methodology/experiments.md` calls "the real remote service" substitute and
`code.md:33` warns must not be mistaken for a live proof. The live proof is the
release itself, and `TODO/packaging.md:602-604` records the last one
(`v0.1.0-beta.6`, commit `6d86129`, gate `success`).

**Plant.** Delete the `--tag` requirement, so the binary resolves a branch head
instead of the tagged commit, and watch the gate lookup miss and exit 2. That is
the exact defect T-1334 exists for: a release whose notes name a commit the
gate never ran on. A second plant swaps `success` for any conclusion in
`gate.rs` and watch a failed-gate release become publishable. The workflow step
is the thing to test; a unit test on `gate.rs` alone does not prove the
workflow stopped.

**Exact command.**

```sh
cargo test -p podbox-release
cargo run -p podbox-release -- notes --tag v0.1.0-beta.12
```

Needs `gh` and the network for the live arm; the unit tests need neither. No
feature flags. The release arm runs on the nightly runner, which is Linux.

**What `exit 2` becomes.** Unchanged. The script has no 1 and the Rust binary
must not add one: the notes step is a gate on publication, and a binary that
returned 1 for "the gate did not pass" would let a workflow that checks only for
a zero exit publish anyway. The two states stay 0 and 2, and the CI step at
`.github/workflows/nightly.yml:179` is what turns a 2 into a failed job.

**What becomes of the release notes.** Nothing tracked. There is no result
file. The evidence is the published release, and `TODO/packaging.md:602-604`
records the read-back through the release API.

---

## Group 4 findings

### Contradictions

**C1. `TODO/gate.md` T-1212's `Prove` cannot be satisfied by its own run.**
`TODO/gate.md:964-968` requires `300-run.sh` to "exit 0 on host podman with the
conditions block naming the driver". `TODO/gate.md:1026-1037` records
`sh experiments/300-run.sh; echo EXIT:$?` / `EXIT:2`, and `:978-980` explains it
as one SKIP. An entry whose `Prove` names an exit code the transcript
contradicts cannot be re-run to green.

**C2. `experiments/270-multiarch-image.sh:266` prints a heading the owning entry
corrected.** The heading reads "a malformed `--platform` is a USAGE error".
`TODO/image.md:1272-1276` records the correction: "125 is what a flag parser
refuses, 1 is what the verb refuses afterwards", and `:1277-1279` says the
script "carried the 125 expectation from before that measurement and went red on
it". The assertion at `:276` is fixed; the heading was not. A reader of stdout
reads a corrected value under a stale claim.

**C3. `TODO/milestones.md` T-1110's close does not match its own result file.**
`TODO/milestones.md:703-704` reads "10 rows, 10 ran, 10 virtualized, 20 declined,
0 host_not_runtime". `experiments/results/interpose-sweep.txt:3-8` reads
`rows 10`, `ran 9`, `virtualized 9`, `declined 18`, `no-pull 1`, `broken 0`.
`experiments/results/sweep245/archlinux.out:1` names the row and the stage:
`driver stage 4 for ghcr.io/pkgforge-dev/archlinux@sha256:b2507f19...`, which is
the `extract` failure arm at `experiments/245-interpose-sweep.sh:290-293`. The
entry's sentence describes a run that is not the one saved.

**C4. `experiments/369-windows-tcg-dos.sh` header and code disagree on the build
state.** `:32-33` says 2 means "no toolchain, no build, no fetch, no jq". The
build failure path at `:90-98` exits 1. One of the two is wrong.

**C5. `experiments/353-open-issue-triage.sh:5-7` names an entry that is not
about it.** It says "TODO triage entry T-1337 (authored beside this script)".
T-1337 is `TODO/cli.md:1258`, category `cli`, about doctor, df and log tail.
Its `Source:` is "issue 38, beta.7 drive 2026-09-25", not this script. Six
entries cite the transcript and none of them calls it the owner.

**C6. `experiments/300-run.sh` clause 3 asserts nothing on the saved run.**
`experiments/results/run.txt:22-25` records the image and the host both
reporting `Arch Linux`, and the script's own warning: "this clause cannot
distinguish them. It asserts only that the file was readable." A clause
satisfied by a run that printed nothing is a clause with no subject.

### Result files that are not script output

**F1. `experiments/results/whiteout-contract.txt` is a manual transcription.**
`experiments/70-whiteout-contract.sh:32` sets `OUT="${OUT:-$HERE/.whiteout}"` and
`:33` removes and recreates it. The script never writes under
`experiments/results/`. Six citations treat the file as a measurement:
`TODO/extract.md:16,88,136,173,278` and `TODO/milestones.md:193`. Nothing in the
tree makes the transcription mechanical, so a later run and the tracked file can
drift without a check noticing.

**F2. `experiments/results/attribute.txt` was produced on a Landlock kernel;
`TODO/probe.md` cites it for an `ENOSYS` control that file does not contain.**
`experiments/results/attribute.txt:7` reads
`kcmp(-1,-1,...) [control] FAIL errno=3 ESRCH`, and `:18-19` carry the Landlock
rows. `TODO/probe.md:168-172` records the target as `ESRCH` and "the host this
repository is worked on" as `errno=38 ENOSYS` from the same file. The file on
disk reads ESRCH and has Landlock rows, so it is a capture from a distro kernel,
not from the WSL host `TODO/probe.md:171` describes. The entry's table and the
file it cites do not agree about which host produced the capture.

### Entries whose acceptance command is a script with no automated test

| Entry | Acceptance command | Has a cargo test? |
| --- | --- | --- |
| T-1104 (`TODO/milestones.md:285`) | `./experiments/300-run.sh exits 0` | no |
| T-0505 (`TODO/enter.md:339`) | `300-run.sh clause 8` | no; `TODO/enter.md:362` says a unit test asserts the same pairing, so the pair is covered and the binary reach is not |
| T-0506 (`TODO/enter.md:434`) | `300-run.sh clause 5` | no |
| T-0502 (`TODO/enter.md:159-166`) | `300-run.sh` clauses 4 and 7 | no |
| T-1110 (`TODO/milestones.md:698`) | `245-interpose-sweep.sh` | no |
| T-0712 (`TODO/interpose.md:1227`) | `245-interpose-sweep.sh` columns | no |
| T-0212 (`TODO/image.md:1259`) | `270-multiarch-image.sh exits 0` | partial; 7 platform tests and 1 store test exist, the live index and the ELF-header read do not |
| T-0205 (`TODO/image.md:407`) | `95-podman-vfs-ignorechown.sh` | no |
| T-0405 (`TODO/complete.md:298`) | `85-completion-symlink-escape.sh` | partial; 2 library tests exist, the shipped-binary reach does not |
| T-0303 (`TODO/extract.md:185`) | `70-whiteout-contract.sh exits 0` | yes, and mutation-proved at `TODO/extract.md:189-194` |
| T-0307 (`TODO/extract.md:399`) | `70-whiteout-contract.sh` plus a cargo test | yes |
| T-0305 (`TODO/extract.md:297`) | a `podbox pull`/`extract` command | yes, `safety::rebase_symlink_target` |
| T-1112 (`TODO/milestones.md:867`) | `369-windows-tcg-dos.sh` | no; the streamed-pipe defect has one, the guest output does not |
| T-1401 (`TODO/podssh.md:38-42`) | `382-restricted-sshd.sh` | no |
| T-1348 (`TODO/gate.md:1742`) | `document-state.py` | no, but `plant.sh:497` plants check 31 |

### Scripts with no owning entry

**N1. `experiments/353-open-issue-triage.sh` has no owning entry.** Six entries
cite its transcript; none names it in a `Prove:`. The header at `:5-7` names
T-1337, which does not own it (C5). It is a premise generator, and
`docs/methodology/experiments.md:5` requires "a tracked script" with "a unique
number" but no entry requires one. Recommend an entry that owns the drive and
states that its output is evidence, not a verdict.

**N2. `experiments/70-whiteout-contract.sh` is named as an acceptance command by
two entries and cited as a premise by two more, but it writes no tracked
result.** See F1. T-0303 and T-0307 both name it in a `Prove:` and both also
name a cargo test; T-1103 names it in a `Prove:` and its clause-1 argument at
`TODO/milestones.md:247` is about its exit 2. The ownership is clear; the
artifact is missing.

### Dead or near-dead scripts

**D1. `experiments/300-run.sh` clause 3 is dead on any host whose distribution
matches the image.** C6. Either pin the image to a different distribution or
drop the clause.

**D2. `experiments/70-whiteout-contract.sh` leaves its scratch.**
`:32-33` removes `$OUT` at the start and never at the end, so a run leaves two
unpacked `docker save` trees under `experiments/.whiteout/`. The
post-task-cleanup rule at `TODO/RULES.md:80-85` says "Remove only scratch that
this session owns", and this scratch is owned by the session that made it.

**D3. `experiments/353-open-issue-triage.sh` and
`experiments/369-windows-tcg-dos.sh` have no exit trap.**
`:27` and `:39` remove `$WORK` at the start and never at the end. The second
leaves a 32 MiB FreeDOS image and a store under
`experiments/.sweep369-work/`. Checked: neither directory is tracked and both
are covered by the `experiments/.*/` ignore rule, so the residue is
uncommitted, but it is residue on a lane.

### The `exit 2` question, answered per script

Rust tests have no third exit code. The mapping each script needs:

| Script | Its `exit 2` arms | What they become |
| --- | --- | --- |
| `70-whiteout-contract.sh` | no tar, no docker, no daemon, cannot pull | disappear; the fixture is in the test |
| `85-completion-symlink-escape.sh` | binary not executable, cannot pull | disappear; the test calls the library |
| `30-attribution-census.sh` | no census, an untestable expectation | a test failure for the first, an asserted third verdict for the second |
| `95-podman-vfs-ignorechown.sh` | none, by design | a panic naming the missing binary, as `crates/podbox-ssh/tests/common.rs:11-12` already does |
| `270-multiarch-image.sh` | 7 arms, all "could not run" | a `CouldNotRun` return in the kept script; nothing in the pure tests |
| `300-run.sh` | 9 arms, all "could not run" | a `CouldNotRun` return in the kept clauses; nothing in the pure tests |
| `document-state.py` | unreadable source | unchanged; `check-todo.py:1708-1709` already separates it |
| `release-notes.sh` | no tag, no gh, gate not green | unchanged; the workflow turns 2 into a failed job |

### What I could not settle

- **Whether `experiments/results/attribute.txt` was intended as the ENOSYS host
  capture.** `TODO/probe.md:171` cites it for `errno=38 ENOSYS`; the file reads
  `errno=3 ESRCH` at line 7 and carries Landlock rows at lines 18-19. Whether
  this is a wrong citation, a replaced file, or a capture from a different host
  is not resolvable from the tree. What would settle it: `git log
  --follow -- experiments/results/attribute.txt` and the capture commit's host
  line.
- **Whether the ten `experiments/results/sweep245/*.out` transcripts were taken
  on the current `DISTRO_ROWS_M5` pins.** All ten pinned names and digests match
  the matrix as it stands (`scripts/common/distro-matrix.sh:74-85`), and the
  result file's `rows 10` agrees with `distro_count_rows`. Whether the
  transcripts are from one run or several is not recorded in them; each names
  only its own row. What would settle it: the `date` and `podbox` lines, which
  per-row transcripts do not carry, only the summary file does.
- **The 22 open issues `353` names.** The script's header at `:3` says 22 and
  `:6-7` says "issues 29-38 and 49-60 plus dependabot PR 9", which is 10 plus
  12 plus 1. The arithmetic does not reconcile and I did not count the
  `run_case` calls. What would settle it: `grep -c '^run_case\|^step' on the
  file, and the issue list from the tracker at the 2026-09-25 date.

---

## Group 4 — entries read

Each entry was read in full, not by grep line. The line reference is the
`Prove:` or the naming citation.

1. **T-1103** M2 extraction that survives the ownership wall — `TODO/milestones.md:182-265`. Owns `70-whiteout-contract.sh`; names its exit 2 at :247.
2. **T-1104** M3 `run` on the chroot rung — `TODO/milestones.md:269-310`. Owns `300-run.sh`; `Prove:` at :285.
3. **T-1110** M6 the interposer across ten distributions — `TODO/milestones.md:660-722`. Owns `245-interpose-sweep.sh`; `Prove:` at :698, close at :703.
4. **T-1112** A disposable guest that is not Linux — `TODO/milestones.md:848-894`. Owns `369-windows-tcg-dos.sh`; `partial`, `Prove:` at :867.
5. **T-1101** M0 the probe census — `TODO/milestones.md:95-124`. Owns the `--capture experiments/results` run of `30-attribution-census.sh` at :109.
6. **T-0302** Ownership-neutral extraction plus the sidecar — `TODO/extract.md:74-149`. Cites `results/whiteout-contract.txt` check B at :88 and :136.
7. **T-0303** Whiteouts are matched on the basename — `TODO/extract.md:153-201`. Owns `70-whiteout-contract.sh`; `Prove:` at :185, mutation record at :189.
8. **T-0305** An absolute symlink target is rootfs-relative — `TODO/extract.md:267-316`. Cites the result file check C at :278.
9. **T-0307** Hard links, symlinks and the layer order — `TODO/extract.md:374-424`. Owns the script plus a cargo test; `Prove:` at :399.
10. **T-0405** `/etc/mtab` is a symlink — `TODO/complete.md:271-333`. Owns `85-completion-symlink-escape.sh`; `Prove:` at :298, run quoted at :319-333.
11. **T-0205** podman's vfs driver with `ignore_chown_errors` — `TODO/image.md:370-426`. Owns `95-podman-vfs-ignorechown.sh`; `Prove:` at :407.
12. **T-0212** The platform is decided at run time — `TODO/image.md:1188-1288`. Owns `270-multiarch-image.sh`; `Prove:` at :1259, clause-5 correction at :1272.
13. **T-0208** `--platform` and the store key — `TODO/image.md:730-768`. Cites the same run at :758.
14. **T-0102** Probe controls — `TODO/probe.md:130-189`. Owns `30-attribution-census.sh`; `Prove:` at :155, close at :157, host table at :168-172.
15. **T-0110** Verdict as a three-variant enum — `TODO/probe.md:550-591`. Owns the script's `Prove:` at :562, close at :564.
16. **T-0712** One argument constant across the matrix — `TODO/interpose.md:1200-1236`. Owns `245-interpose-sweep.sh`; `Prove:` at :1227.
17. **T-0505** `exec` as a fresh chroot — `TODO/enter.md:320-376`. Owns `300-run.sh` clause 8; `Prove:` at :339.
18. **T-0506** A foreign-architecture container — `TODO/enter.md:378-449`. Owns clause 5; close at :436.
19. **T-0502** Resolve the program inside the new root — `TODO/enter.md:150-167`. Owns clauses 4 and 7; names them at :159-166.
20. **T-1212** Convert the engine scripts — `TODO/gate.md:935-1050`. Names all three of my engine-using scripts; `Prove:` at :964, transcript at :993-1037.
21. **T-1348** Check current documents against source declarations — `TODO/gate.md:1730-1748`. Owns `document-state.py`; `Prove:` at :1742.
22. **T-1401** Prove the SSH transport and server in the current tree — `TODO/podssh.md:16-65`. Owns `382-restricted-sshd.sh`; `Prove:` at :38-42, close at :44-56.
23. **T-1337** Doctor, disk usage, and log tail — `TODO/cli.md:1258-1377`. Cites the `353` transcript at :142 and :1280; the entry `353`'s header names.
24. **T-1334** The release carries its gate state and reproducibility boundary — `TODO/packaging.md:562-608`. Owns `release-notes.sh`; close at :602-608.
