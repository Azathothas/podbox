# Group 8 audit

## Group 8 summary

Thirteen scripts, 2912 lines, read in full. Verdicts:

| Verdict | Count | Scripts |
| --- | --- | --- |
| RUST-TOOL | 4 | `60-interposer-libc.sh`, `120-reproducible-build.sh`, `395-reconcile-repository.py`, `399-publication.py` |
| RUST-TEST | 4 | `130-probe-parity.sh`, `151-spawn-ambiguity.sh`, `157-lock-inheritance-prove.sh`, `364-qol.sh` |
| SPLIT | 3 | `105-interpose-ownership.sh`, `154-tcg-workload-spread.sh`, `161-path-rewrite.sh` |
| KEEP-SHELL | 2 | `200-registry-auth.sh`, `392-kvm-guest.sh` |
| DELETE | 0 | - |

Totals: 4 + 4 + 3 + 2 = 13. `200-registry-auth.sh` is a KEEP-SHELL whose
credential-file and refusal clauses lift to Rust; the table gives the primary
verdict and its section gives the split.

No script in this group is a plain DELETE. Every premise still holds against
current source. The work is re-expression, not removal.

The three most important findings:

1. **Three different symbol counts for the same fact, and the live entry holds
   the oldest.** `crates/podbox-interpose/interpose.map` declares **112** names
   today (counted from the file). `TODO/gate.md:472` records 112, measured
   2026-09-23. `experiments/results/interpose-ownership.txt:25-27` records 105,
   taken 2026-09-19. `TODO/interpose.md:87` records 17, in the T-0701 Done
   paragraph. The entry that owns the measurement is the one that is wrong.
2. **Script 157 clause 2 can no longer land its mutation, and the script would
   exit 1 saying so.** The sed pattern is
   `s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/`. The source at
   `crates/podbox-image/src/store.rs:1083` reads
   `if !sys::close_in_children(fd) {`, and the enclosing function is
   `Store::try_acquire`, not `Store::hold`. The script's own guard
   ("THE MUTATION DID NOT LAND", `157-lock-inheritance-prove.sh:129`) would
   fire and set `fail=1`. The image.md T-0211 Done record quotes a green run
   from 2026-09-12 that predates the rename.
3. **`experiments/README.md:35` lists `392-kvm-guest.sh` in the proof table as
   a passing repository audit proof, and its own result file says
   `verdict KVM-GUEST-FAIL` with `proof exit: 1`** at
   `experiments/results/kvm-guest.txt:69-70`. T-1112 and T-1350 both record it
   as partial, so the README table is the outlier.

## Group 8 - experiments/60-interposer-libc.sh

**What it measures.** Two questions, both of which the script's own header
marks as answered elsewhere. A: can `crates/podbox-interpose` be built as a
cdylib under the workspace's own `-C target-feature=+crt-static`? The script
unsets `RUSTFLAGS` deliberately so the root `.cargo/config.toml` is the
condition under test, and requires the build to fail with `does not support
these crate types` (`:76-96`). A2: with `-crt-static` and `scripts/zig-cc.sh`
as the musl linker, does each target produce an object, and what does each
record in `DT_NEEDED` (`:99-131`)? B: can a musl-linked preload object load
into a glibc payload? The script reads `DT_NEEDED` first and **exits 2 rather
than answering from an object that is not musl-linked** (`:167-195`).

**Pinned inputs.** The toolchain named by `rust-toolchain.toml`, the two
targets `x86_64-unknown-linux-musl` and `x86_64-unknown-linux-gnu`, this
host's `/usr/bin/env` as the glibc payload, and the exact `WANT` string that
check A greps for.

**Host assumptions.** `cargo`, `rustup`, `readelf`, `ldd`, and `zig` for the
musl arm. Without `zig` the script says so in the conditions block and falls
back to the default linker, which produces a glibc object under a musl target
name (`:113-118`).

**Exit contract.** 2 when cargo, the crate, or either target is absent
(`:67-72`), when the payload is absent (`:141`), when the glibc control failed
to load (`:163`, and that is `exit 1` because nothing below is interpretable),
and when the musl object records `libc.so.6` (`:193`).

**Timeouts.** None. Four `cargo build --release` runs, unbounded.

**Positive control.** The glibc object preloaded into a glibc payload, which
must exit 0 before anything below is read (`:156-164`). The comment there is
the right rule: "THE CONTROL COMES FIRST. Without a matching-libc object that
does load, a cross-libc object that does not load proves nothing about the
libc." **Failure control.** The same, inverted: the musl object must fail to
load (`:203-211`).

**Owning entries.** `TODO/interpose.md` T-0701, whose `Premise` is
"`experiments/60-interposer-libc.sh` check A records that the crate cannot be
built as a cdylib at all under the workspace's own `+crt-static`". T-0702,
whose `Premise` and Done both read the result, and which records the 2026-09-08
re-run through `zig cc` as the measurement that answered the question. That
entry also carries the correction the script's header explains: the entry was
written expecting this script to show a musl object failing to load, and the
script could not do so until `zig cc` arrived. The header names
`experiments/80-interposer-abi.sh` as the script that answered B with the C
reference as its subject.

**Result file.** `experiments/results/interposer-libc.txt`, exit 0, 2026-09-08.
`:20-21` records `musl-target DT_NEEDED libc.so` against
`gnu-target DT_NEEDED libgcc_s.so.1 libc.so.6 ld-linux-x86-64.so.2`, and
`:25-26` records the refusal `invalid ELF header`. The verdict at `:29-31` is
that one object per libc is required.

**Verdict: RUST-TOOL, and a small part of it is already a Rust check.**
`scripts/build-interpose.sh` now asserts `DT_NEEDED` after every build
(T-0701's Done records this, and the entry says it asserts "on an exact word:
`libc.so.6` contains `libc.so`, so a substring test would call a glibc object
musl-linked, which is the very mistake being caught"). So A2's substance is
already in the build. What remains unique to this script is the A refusal, the
DT_NEEDED read, and the B cross-libc load.

Conversion plan:

**A, the cdylib refusal. Precondition/action/expected.**
Precondition: the root `.cargo/config.toml` present, `RUSTFLAGS` unset.
Action: `cargo build --release --target x86_64-unknown-linux-musl` in
`crates/podbox-interpose`. Expected: the build fails and the log contains
`does not support these crate types`. Rust level: **deployment proof**, and it
is the one assertion in the group that must never go green, because a green
A means the interposer silently stopped being a preload object. It cannot be a
`cargo test`: a test that runs after a successful build has already lost the
condition.

- Crate: new `crates/podbox-interpose-check`, binary `podbox-interpose-check`.
  It belongs beside `scripts/build-interpose.sh` in the sense that both read
  the same two objects, and the implementor should decide whether it replaces
  the DT_NEEDED assertion already in that script rather than adding a second
  reader. One read path, per `docs/conventions/code.md:3`.
- CLI surface: `podbox-interpose-check --crate crates/podbox-interpose
  [--expect-cdylib-refused] [--check-needed] [--cross-libc]`. Exit 0, 1 or
  2, as the experiment contract requires.
- Fixtures: none. It drives `cargo` and `readelf`, bounded, reading each
  status from the process that produced it. The `--cross-libc` arm needs a
  glibc payload, so it takes `--payload /usr/bin/env` as an argument, which is
  what `docs/methodology/experiments.md:10` requires ("State any required
  binary or licensed image path as an argument").
- Plant: add `crate-type = ["cdylib", "rlib"]` to the crate's `Cargo.toml` and
  assert the tool reports that A did not refuse. That plant fails today, which
  is the point: it is the assertion nobody has seen fire. The control arm:
  restore and assert exit 0.
- Command: `cargo run -p podbox-interpose-check -- --expect-cdylib-refused
  --check-needed --cross-libc`.
- `exit 2` path: preserved. "zig is absent" is a state, and the current script
  is careful to say so rather than reporting a glibc object as a musl one.
  That care must survive the port.
- Result file: `interposer-libc.txt` is **superseded** by a run of the tool.
  Keep it as the record of the 2026-09-08 measurement, which is what
  `TODO/interpose.md` T-0702's Done quotes at length.
- The header's own warning is a constraint on the port: an object that is not
  musl-linked cannot measure whether a musl-linked one loads. The tool must
  keep the `DT_NEEDED` read in front of the load attempt and exit 2, not
  attempt the load.

## Group 8 - experiments/105-interpose-ownership.sh

**What it measures.** Seven checks (A through G) on podbox's own LD_PRELOAD
object. A: the object's exported `T` symbols equal the `global:` block of
`crates/podbox-interpose/interpose.map`, for both the gnu and musl objects, and
`rust_eh_personality` is in neither. B: `offsetof` on `struct stat` and
`struct statx` under both libcs equals a hardcoded `WANT` string
(`105-interpose-ownership.sh:228`). C, the control, first: on a host that can
chown, the object changes nothing and writes no memo. D and E: under
`--cap-drop=CHOWN`, the bare chown fails and the preloaded chown exits 0 with
`stat` reading `0:42`, glibc and musl. F: `podbox system abi` refuses the
object against a glibc 2.31 payload from ELF, and the loader agrees. G: past the
4 MiB memo ceiling the answer is correct or a refusal, never stale.

**Pinned inputs.** `GLIBC_PAYLOAD` (fedora by digest), `GLIBC_TOO_OLD` (ubuntu
by digest), `MUSL_PAYLOAD` (alpine by digest), all three at
`105-interpose-ownership.sh:82-86`. `GNU_SO` and `MUSL_SO` from
`scripts/build-interpose.sh`, overridable in the environment. The offsets in
`WANT`.

**Host assumptions.** A docker daemon or host podman via
`experiments/lib/engine.sh`. `nm`, `readelf`, `cc` or `zig`. An object built
against a glibc newer than the pinned payload's, which is what makes F fire.
On Windows the binary and the C programs must be staged into a container
because NTFS carries no mode bit (`105-interpose-ownership.sh:175-178`).

**Exit contract.** `exit 2` when the engine is absent (`:72`), when the objects
are not built (`:107`), or when any check reported `could_not` (`:458`).
Otherwise `rc` from the FAIL lines. There is no cleanup trap on the object
build; `eng_cleanup` runs on EXIT.

**Timeouts.** `eng_run 180` for the subject, `eng_run 60` for the reader and
the staged offsets programs. `eng_pull` runs untimed before any timed clause.

**Positive control.** C, and it comes first by design (`:270-286`). **Failure
control.** D and E run the bare chown without the object and require it to
fail; if the bare chown succeeds the script says "so there is no wall here" and
sets `could_not` (`:302-303`).

**Owning entries.** `TODO/interpose.md` T-0701 (`Prove` names this script and
check A), T-0702 (Status note and Done, cites the run), T-0704 (`Prove` and
Done), T-0710 (`Premise` and `Prove`, and the Done that check G closed).
`TODO/gate.md` T-1210 and T-1211 name it as the `Source` for the
`lib/engine.sh` conversion. `TODO/gate.md` T-1209 item 2 asks for the same
exported-symbol check to move into `dev.sh check`.

**Result files.** `experiments/results/interpose-ownership.txt` (7 checks,
exit 0, 2026-09-19).

**Verdict: SPLIT.** The seven checks are four different kinds of assertion with
four different right homes.

| Piece | Goes to | Why |
| --- | --- | --- |
| A (exported set equals `interpose.map`, no personality) | `crates/podbox-interpose/tests/exported_set.rs` | ELF and `nm` facts, deterministic per build, no runtime |
| B (offsets) | `crates/podbox-interpose/tests/struct_offsets.rs` plus a `const` cross-check | same, and it is a fact about a header podbox hardcodes |
| C, D, E (the wall) | keep as a KEEP-SHELL driver | needs `--cap-drop=CHOWN` and a real loader |
| F (the reader refuses) | already unit-tested in `crates/podbox-enter/src/abi.rs` | the reader is Rust; the container only supplies a second libc file |
| G (past the ceiling) | pure Rust test in `crates/podbox-interpose/src/memo.rs` tests | the ceiling is a constant and the record is a struct |

Conversion plan:

**A, precondition/action/expected.**
Precondition: `scripts/build-interpose.sh` has run and both `.so` files exist.
Action: read the `global:` block of `interpose.map`, sort it; read
`nm -D --defined-only` `T` symbols from each object, sort; diff both.
Expected: both diffs are empty, and `grep -c rust_eh_personality` is 0 for each
object. Rust level: **integration test** (a `tests/*.rs` file), because the
subject is the built artefact and the assertion needs an external reader, not
the crate's internals. `docs/conventions/code.md` kinds: this is
integration-and-deployment proof for the actual default path, because the object
is what the shipped binary embeds, and no mock substitutes for the exported
symbol table.

- Target: `crates/podbox-interpose/tests/exported_set.rs`, module `exported_set`.
- Fixtures: none beyond the built objects. Path constant
  `env!("CARGO_MANIFEST_DIR")/interpose.map`; object paths under
  `target/x86_64-unknown-linux-{gnu,musl}/release/libpodbox_interpose.so`.
  Reuse `scripts/zig-cc.sh` and `scripts/gnu-link-stub.sh` only through the
  build script that already drives them; the test does not invoke them.
- Plant: append a symbol to the `global:` block that `src/lib.rs` does not
  define, rebuild, assert the test fails. Then remove it, rebuild, assert green.
  The second half is the half that proves the test can pass.
- Command: `./scripts/build-interpose.sh && cargo test --manifest-path
  crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --
test exported_set`.
- `exit 2` path: the objects are absent. A Rust test that cannot find them should
  `panic!` with the build command in the message, or the case is `#[ignore]`d
  with that reason. There is no third state, and that is a loss the entry must
  name: today a missing object is `exit 2` and reads as "no information"; as a
  test it reads as failure. Recommended: fail loudly, and give the gate a
  separate build step so the object is never absent.
- Result file: `interpose-ownership.txt` is superseded for check A. Its
  "declared 105" line is stale and must not be re-read. Keep the file for
  C through G.

**B, precondition/action/expected.**
Precondition: a C compiler for each libc. Action: print `sizeof` and six
`offsetof` values for `struct stat`, and `sizeof` plus six for `struct statx`.
Expected: both libcs print the `WANT` string, and it equals the constants
`crates/podbox-interpose/src/lib.rs` carries. Rust level: **integration
test** with a compiled C fixture, because the subject is the platform's
`sys/stat.h`, not Rust. The `WANT` comparison is the part that can go stale and
it belongs in the test, not in a script header.

- Target: `crates/podbox-interpose/tests/struct_offsets.rs`, module
  `struct_offsets`. The C source moves from the heredoc at
  `105-interpose-ownership.sh:147-172` into
  `crates/podbox-interpose/tests/fixtures/offsets.c`, checked in.
- Fixtures: `offsets.c` (new, moved verbatim). Build with `cc` and, for musl,
  `scripts/zig-cc.sh` with `ZIG_TARGET=x86_64-linux-musl`, which
  `105-interpose-ownership.sh:188-190` already shows.
- Plant: change one expected offset in the test by 8. Assert the test fails with
  a message naming the field. Then restore.
- Command: same as A, with `--test struct_offsets`.
- `exit 2` path: a compiler is absent. Same loss as A: a test cannot report
  "could not run", so the build step must be part of the command above.
- Result file: superseded for B. The line
  `interpose-ownership.txt:34-35` is the record it wrote.

**G, precondition/action/expected.**
Precondition: a memo file over 4 MiB containing a record for `(dev, ino)` and a
later record for the same pair. Action: `lookup` the pair. Expected: not the
first record's value. The current code returns `BeyondCeiling` past the ceiling
and the caller answers the real owner marked degraded, which
`interpose-ownership.txt:78-79` records verbatim. Rust level: **pure unit
test**, because the ceiling is a constant, the record layout is a struct, and
the scan is a slice operation. This is the piece of the script that most
deserves to leave it.

- Target: `crates/podbox-interpose/src/memo.rs`, the existing
  `#[cfg(test)] mod` (the file already carries 55 test attributes across the
  crate; `memo.rs` is in that set). Test name:
  `a_lookup_past_the_ceiling_never_returns_a_stale_record`.
- Fixtures: none. Build the memo in a `Vec<u8>` in the test.
- Plant: change the ceiling constant from 4 MiB to 64 MiB so the scan completes
  and the stale record returns. Assert the test fails. Restore.
- Command: `cargo test --manifest-path crates/podbox-interpose/Cargo.toml
  --target x86_64-unknown-linux-gnu memo::tests`.
- `exit 2` path: none. This check cannot be skipped, which is an improvement.
- Result file: check G's four lines in `interpose-ownership.txt:77-82` become
  superseded by the unit test. Keep the file.

**C, D, E, F: KEEP-SHELL remainder.** `C` is the control for `D` and `E` and
cannot be lifted: it needs a host that grants chown. `D` and `E` need
`--cap-drop=CHOWN` and a real ELF loader, which a Rust test cannot arrange
without a container. `F` needs a second libc file to exist, which the test can
take as a committed fixture: `crates/podbox-enter/src/abi.rs:1074` already
carries the reader's tests, and `interpose-abi.txt` check E is that reader
against a real payload libc. Move F to the remainder only, and let the
`abi.rs` unit tests carry the reader.

## Group 8 - experiments/120-reproducible-build.sh

**What it measures.** Two `cargo build --release --target
x86_64-unknown-linux-musl` runs in one container, separated by
`cargo clean -p podbox-cli`, and the sha256 of `podbox` from each. It also
compares the two `version --verbose` documents and prints the first five
differing byte offsets when the binaries differ.

**Pinned inputs.** The tree at `HEAD`; the lane base; the two release builds.
The commit is printed, not asserted. `experiments/results/reproducible-build.txt:4`
records commit `0d4cf6214acfd4daf8c11474263a96f4bf9b2187`.

**Host assumptions.** `wsl-toolkit` on PATH (`:42`) and
`scripts/windows/run-in-base.sh`. The script refuses to run without both, and
that refusal is `exit 2` (`:44-47`). It is a Windows-lane script: it hands
`/tmp` paths to the wrapper and deliberately does not set `MSYS_NO_PATHCONV`
(`:28-33`).

**Exit contract.** 0 for identical bytes, 1 for a mismatch, 2 for no lane, a
failed lane job, or a missing artifact. The `trap` removes
`experiments/.sweep120-work` on EXIT, HUP, INT, TERM.

**Timeouts.** None of its own; the wrapper bounds the job.

**Controls.** **No positive control and no failure control.** Nothing in the
script can make the two builds differ, and nothing proves the two builds were
of the same tree. The one thing it asserts that could fail is a digest
mismatch. This is the weakest instrument in the group and it is the Prove of a
closed entry.

**Owning entry.** `TODO/packaging.md` T-1004. `Prove` is
`` `./experiments/120-reproducible-build.sh`; a runnable verdict ``. Done
2026-09-22.

**Result file.** `experiments/results/reproducible-build.txt`, verdict
`MATCH: two builds, one byte stream` (`:17`), and the two builds hash to
`83b0549b25e1f051a413b8800136a64dcf1283b9a0bef88bb6ec0f09712c209a` (`:15-16`).

**Verdict: RUST-TOOL.** This is a durable packaging check, not a one-off
measurement. It is the reproducibility half of T-1004's `Approach` and the
operator will want it on every release.

Conversion plan:

**Precondition/action/expected.**
Precondition: a clean tree, a release target triple, and a toolchain that can
link it. Action: build release, record the sha256 of the binary and the
`version --verbose` text; `cargo clean -p podbox-cli`; build again; record
both. Expected: the two sha256 values are equal. When they differ, the report
names the first five differing offsets and the two `verbose` documents are
diffed. This is a **deployment** check in the `docs/conventions/code.md` sense:
it exercises the actual release build, and no unit test substitutes for it.

- Crate: new `crates/podbox-reproducible`, binary `podbox-reproducible`.
- CLI surface:
  `podbox-reproducible --target x86_64-unknown-linux-musl --manifest-path <p>
  --out <file> [--runs N]`. It exits 0 on a match, 1 on a mismatch, 2 when a
  precondition is absent, which preserves the three-state contract the
  experiment contract in `experiments/README.md:7-11` requires. Keeping the
  third exit code is the one place a Rust tool may keep it, because this is a
  process, not a test.
- Fixtures: none. It drives `cargo` as a subprocess, bounded, with the process
  status read from the process that produced it.
- Plant: `scripts/plant.sh` case. Build once, then plant a change into
  `crates/podbox-cli/build.rs` that embeds a timestamp, and assert the tool
  reports a mismatch naming an offset. The control half: run on the clean tree
  and assert exit 0.
- Command: `cargo run --release -p podbox-reproducible -- --target
  x86_64-unknown-linux-musl`.
- `exit 2` path: preserved as written above. A Rust binary may carry it; a
  `cargo test` may not.
- Result file: `reproducible-build.txt` stays. A second run overwrites it with
  its own date, and `docs/conventions/prose.md:46` forbids substituting values
  from another host, so the file must keep recording the commit it was taken
  at. The `120-` script is deleted; its number is not reused
  (`experiments/README.md:4`).

## Group 8 - experiments/130-probe-parity.sh

**What it measures.** Three clauses, which are T-1101's whole acceptance. 1:
`podbox probe` inside `20-enter-target.sh` selects `chroot`. 2: the same binary
unconfined selects `namespace`. 3: every attribution row podbox reports inside
the reconstruction equals the row of the same name in
`experiments/results/attribute.txt`, comparing **verdict and errno, never raw
text** (`:100-112`). One divergence is recorded and allowed: the kcmp control
where the reference says `denied errno=38` and podbox says `skip` (`:173-181`).

**Pinned inputs.** `attribute.txt`, which `30-attribution-census.sh --capture`
produces. `$BIN`, the release binary, overridable by `PODBOX_BIN`. The debian
driver image by digest at `:50`.

**Host assumptions.** A docker daemon or host podman (`:45-46`). On Linux it
runs the binary directly under `timeout`; elsewhere it stages the ELF into the
debian driver because an ELF built on Windows does not execute there (`:25-28`).

**Exit contract.** 2 when there is no engine, when the driver cannot be
staged, when the reference file is absent, or when the reconstruction did not
run. 1 when any row differed or was missing, or when a rung was wrong. 0
otherwise.

**Timeouts.** `pb 60` for `version` and the unconfined probe.

**Positive control.** Clause 2, the unconfined rung, which must read
`namespace`; and clause 1, which must read `chroot`. The pair is a control for
each other: a probe that always answered one value fails one of them.
**Failure control.** The `normalise` filter drops a row that does not match
`|(ok|denied|skip)|`; the `missing` counter and the empty `all.txt` guard at
`:151` catch the case where the instrument produced nothing. There is no
deliberately wrong row planted, which is the gap.

**Owning entries.** `TODO/milestones.md` T-1101 (`Prove`, and the Done).
`TODO/gate.md` T-1213 (`Source` names `130-probe-parity.sh:1-40`; `Prove`;
Done). `TODO/probe.md` cites it twice: T-0101's block at `:91` for the sixteen
attribution numbers, and the boot-id entry at `:623` for the host-versus-
reconstruction rung pair.

**Result file.** `experiments/results/probe-parity.txt`, `rows_matched 16`,
`rows_recorded 0`, `rows_differed 0`, `rows_missing 0` (`:6-9`).

**Verdict: RUST-TEST.** The comparison logic is pure; the rungs are an
integration fact. Split by level inside one test file.

Conversion plan:

**Clause 3, the row comparison. Precondition/action/expected.**
Precondition: two row tables in `name|verdict|errno` form. Action: for each
reference row, look up the podbox row by name, compare verdict and errno.
Expected: every row matches; the single recorded kcmp divergence passes; any
other difference fails. Rust level: **pure unit test**, because both inputs are
deterministic text. This is the clause worth keeping a unit test for, and it is
the clause that has a plant-shaped hole today.

- Target: new module `crates/podbox-probe/tests/probe_parity.rs`, module
  `probe_parity`. The crate under test is `podbox-probe`, whose
  `#[json.rs]` and `report.rs` already own the row rendering. The comparison
  itself belongs in a new public function `podbox_probe::parity::compare_rows`
  in `crates/podbox-probe/src/parity.rs`, tested from
  `crates/podbox-probe/src/parity.rs`'s own `#[cfg(test)] mod`, with the
  integration file carrying only the fixture-driven case.
- Fixtures: `experiments/results/attribute.txt` and the `## every row podbox
  reported` block of `experiments/results/probe-parity.txt`, checked in under
  `crates/podbox-probe/tests/fixtures/`. Both already exist as tracked files,
  so this is a copy with a recorded origin, not new data.
- Plant: the one this needs. Change one reference row's errno from `1` to `2`
  in the fixture and assert the comparison fails naming the row. Then change
  one podbox row's verdict from `denied` to `ok` and assert the same. Then
  assert a clean compare returns zero differences. The recorded-divergence case
  is the third plant: remove the kcmp special case and assert a kernel without
  `CONFIG_CHECKPOINT_RESTORE` would be reported as a disagreement.
- Command: `cargo test -p podbox-probe --test probe_parity`.
- `exit 2` path: the reference file is absent. In the test that is a
  `panic!` with the `--capture` command in the message, and the reason the
  fixture must be committed rather than generated. The 2-state is lost; the
  unit test no longer measures a live capture, which is the honest trade and
  belongs in the entry.
- Result file: `probe-parity.txt` is superseded for the row comparison. Clauses
  1 and 2 keep the script.

**Clauses 1 and 2. Precondition/action/expected.**
Precondition: the reconstruction image built by `10-build-target-image.sh` and
a staged release binary. Action: run `podbox probe` inside it, and again
unconfined. Expected: `chroot` and `namespace`. Rust level:
**integration-and-deployment proof**, because the rung depends on what the
kernel grants the process, which only a real run answers. This half is
KEEP-SHELL, and it stays a script: it needs an engine, a built image and a
driver. The Rust part it gains is `podbox_probe::select::select_mode`'s existing
decision table, which `crates/podbox-probe/src/select.rs:458-558` already
unit-tests with constructed outcomes including the kcmp cases.

- Remaining target: the script keeps clauses 1 and 2 and drops clause 3.
  Number stays `130`; the script is not deleted and its number is not reused.
- Fixtures: `experiments/10-build-target-image.sh` and
  `experiments/20-enter-target.sh` stay as they are; they are group 1 and 3.
- Plant: none new. The existing `130` has no plant, which is a finding below.
- Command: `sh experiments/130-probe-parity.sh; echo EXIT:$?`, exit 0.
- `exit 2` path: unchanged.
- Result file: `probe-parity.txt` keeps clauses 1 and 2 and drops the
  `## every row podbox reported` block. Rewrite it in place, keeping the
  `confined_rung` and `unconfined_rung` lines.

## Group 8 - experiments/151-spawn-ambiguity.sh

**What it measures.** Behind one identical Go string
(`fork/exec /bin/true: operation not permitted`, `:102`), does `podbox probe`
name the refused call per context. Leg A: in a plain context, the clone payload
fails and `probe`'s stderr contains `clone(CLONE_NEWNS) here: denied` and
`meets the clone denial`. Leg B: under `unshare -Ur`, the credential payload
fails with the byte-identical string and `probe` contains both
`meets the setgroups denial` and `Credential{NoSetGroups: true}`. Leg C: the
credential payload exits 0 where setgroups is allowed, which proves the failure
is the denial and not the binary.

**Pinned inputs.** Two Go sources written as heredocs at `:52-91`; the module
`sweep151`; go 1.19.8 per the result file. `/bin/true`.

**Host assumptions.** `go` and `unshare` on PATH (`:36-37`). A kernel where
`clone(CLONE_NEWNS)` is denied, and a user namespace where `setgroups` reads
`deny` under `unshare -Ur`. The script explicitly runs under `sh` in a dash job
container, so it sets no `pipefail` (`:16-18`).

**Exit contract.** 2 when the binary, go or unshare is absent, or when a
payload did not build. 1 when any assertion failed **and** nothing was skipped.
**If any leg skipped, it exits 2** (`:171`). That precedence is unusual and
correct: a leg that could not run means the measurement did not happen.

**Timeouts.** `timeout 60` on each probe and payload, `timeout 120` on the
probe under unshare.

**Positive control.** Leg C. **Failure control.** Leg A's negative case is
implied by leg B: the two payloads must produce byte-identical output, and
`:133-137` fails if they differ. The `control` in the true sense is absent: no
case runs a probe that should *not* attribute.

**Owning entry.** `TODO/cli.md` T-0809. `Prove` is
`` `./experiments/151-spawn-ambiguity.sh` asserts podbox distinguishes a
refused clone from a refused `setgroups` ``. Done 2026-09-21, and the Done
already records that **five unit tests pin the leg matrix** in
`crates/podbox-probe/src/report.rs`, including the unmeasured leg. Those tests
are at `report.rs:1147-1180` and I read them.

**Result file.** `experiments/results/spawn-ambiguity.txt`, all three legs
green, the shared string recorded twice (`:10` and `:15`).

**Verdict: RUST-TEST, mostly already done.** This is the clearest DELETE-shaped
finding in the group, and it is not a DELETE, because the Go payloads are the
only thing that cannot be a Rust test.

What already exists. `report.rs:1147-1180` carries
`two_clear_legs_mean_no_spawn_note`, `two_denied_legs_name_both_and_tell_them
_apart_by_shape`, and `an_unmeasured_leg_is_never_attributed`. cli.md's Done
says five tests. Those cover the diagnostic. The script covers the other half:
that a real Go payload really does produce one ambiguous string behind two
different refusals.

Conversion plan:

**The diagnostic half. Precondition/action/expected.**
Precondition: a probe document with the clone leg denied and the setgroups leg
clear, and the reverse. Action: render the spawn note. Expected: the clear-legs
case names neither wall; the denied-legs case names both, tells them apart by
payload shape, and names `Credential{NoSetGroups: true}`. Rust level: **pure
unit test**. This already exists at `report.rs:1147-1180`; the conversion keeps
it and adds the negative arm the script never had.

- Target: `crates/podbox-probe/src/report.rs`, the existing `#[cfg(test)] mod
  tests` at `:755`. Add
  `an_attribution_is_not_emitted_where_no_leg_is_denied`, and extend
  `an_unmeasured_leg_is_never_attributed` to cover the clone leg as well as the
  setgroups leg.
- Fixtures: constructed `Outcome` values, the pattern `report.rs:343-376`
  already uses.
- Plant: delete the `if c_denied` arm that emits "A payload that sets clone
  flags meets the clone denial here" (`report.rs:367`) and assert
  `two_denied_legs_name_both_and_tell_them_apart_by_shape` fails. The control
  arm: assert it passes on the restored tree.
- Command: `cargo test -p podbox-probe report::tests`.
- `exit 2` path: none. A leg that could not be measured is a value, not a
  state, and `an_unmeasured_leg_is_never_attributed` is exactly that rule.
- Result file: `spawn-ambiguity.txt` becomes superseded for the diagnostic. It
  still carries the Go payload evidence.

**The payload half. Precondition/action/expected.**
Precondition: a container where `setgroups` is denied. Action: build two Go
programs, one with clone flags and one with a credential, run both, compare
their bytes. Expected: byte-identical, both non-zero. Rust level: this is a
**fixture** measurement and it has no pure form: the subject is a third-party
language runtime's `os/exec`. It stays a shell script, and the Go sources move
from the heredocs at `:52-91` into `experiments/src/` beside
`across-probe.c` and `nsswitch-q.c`, so they are readable rather than embedded.

- Target: the script stays, reduced. Number stays `151`. Its Go sources move to
  `experiments/src/spawn-clone.go` and `experiments/src/spawn-cred.go`; the
  script reads them from there with `[ -r ... ] || exit 2`.
- Fixtures: the two Go files, now tracked.
- Plant: change `spawn-cred.go` to print a distinct string and assert the
  script reports "the two strings differ". The control arm: restore and assert
  exit 0.
- Command: `./experiments/151-spawn-ambiguity.sh; echo EXIT:$?`.
- `exit 2` path: unchanged, including the skip-overrides-fail precedence at
  `:171`.
- Result file: `spawn-ambiguity.txt` is rewritten to carry the Go leg only, and
  the conditions block keeps `go` and the two setgroups readings.

## Group 8 - experiments/154-tcg-workload-spread.sh

**What it measures.** The machine tier's cost per workload class, with the
same checksum on every platform. Four static C payloads (`154-bench-{int,sys,
mem,io}.c`), each run three times on three platforms: native on the host, chroot
through podbox, and a TCG guest assembled from the alpine rootfs plus an
`/init` batch. One row per class: median of three, ratios, and agreement that
all three platforms printed the same checksum. A checksum disagreement is
`exit 1`.

**Pinned inputs.** `ALPINE_REF` by digest, the alpine netboot `vmlinuz-virt` URL
and its sha256 (`154-tcg-workload-spread.sh:55-57`), `QEMU_FLAGS` and
`BENCH_CFLAGS` printed in the conditions block (`:64-65`), the four payload
digests printed per run.

**Host assumptions.** `qemu-system-x86_64`, `cpio`, `python3`, `curl`,
`sha256sum`, `timeout`, `gcc`, `file`, `awk`, `sort`, all enumerated at `:79`.
It sources `experiments/lib/podvm-guest.sh`, so `podvm_base`, `podvm_extras`
and `podvm_concat` are the shared driver, and its header says the lib is
untouched so 146 and 147's evidence stands. The guest needs no disk device, so
its file-I/O leg measures initramfs, not a disk; podvm.md's Done records that
limit.

**Exit contract.** 2 when the binary, a tool or a payload build is absent. 1
when a run failed, a tally was wrong, or two platforms disagreed. 0 otherwise.

**Timeouts.** `RUN_TIMEOUT=300` per run, `BOOT_TIMEOUT=600` for the guest,
`CURL_TIMEOUT=60` for the kernel. qemu exiting 124 is **recorded as the halt the
spec records**, not a failure (`:214`).

**Positive control.** The native host runs, same payload binary. **Failure
control.** The checksum agreement across three platforms: two platforms
computing something different is a refusal, and the same binary on all three
platforms is what makes the checksum meaningful.

**Owning entry.** `TODO/podvm.md` T-1308. `Prove` names this script. Three
records: Done 2026-09-22, Partial 2026-09-25 (issue 53 reopened it on two
report-internal notes), Done 2026-09-26.

**Result file.** `experiments/results/tcg-workload-spread.txt`, 14 driven, 0
mismatches, both pinned flags in the conditions block (`:9-10`), which is
exactly what the Partial record asked for.

**Verdict: SPLIT.**

| Piece | Goes to | Why |
| --- | --- | --- |
| The three medians, the ratios, the checksum-agreement rule | Rust, pure | arithmetic over captured numbers with a stated rule |
| The host, chroot and guest runs | keep as a KEEP-SHELL driver | needs qemu, a rootfs, an initramfs and a pinned kernel URL |

Conversion plan:

**The aggregation. Precondition/action/expected.**
Precondition: three log files, one per platform, each holding exactly three
`workload=<class> elapsed=<s> checksum=<hex>` lines for that class. Action: take
the second elapsed value in sorted order as the median; collect the distinct
checksums. Expected: exactly one checksum across all three platforms, three
medians non-empty, and ratios printed to one decimal or `-` where a number
cannot divide. Rust level: **pure unit test**. The current `elap` function
(`:226-234`) has a documented defect its own author recorded in podvm.md's
Partial: an `io error=` line counts as a run in section 2 and is refused in
section 4, so the two tallies disagree. A Rust function cannot carry that
disagreement.

- Target: new `crates/podbox-podvm/src/spread.rs` in a new crate
  `crates/podbox-podvm`, module `spread`, with `#[cfg(test)] mod` inside it.
  Alternatively, if a new crate is refused, the same module under
  `crates/podbox-cli/src/machine/` beside `doctor.rs`. The implementor should
  prefer the new crate: the aggregation belongs to the machine tier and
  `podbox-cli` already carries the CLI half of it.
- Fixtures: three captured log files under
  `crates/podbox-podvm/tests/fixtures/`, taken from the saved run. Or, better,
  the loader takes `&str` and the fixtures are inline `const` strings, which is
  what a pure test wants.
- Plant: the plant this most needs is the issue-53 defect itself. Add a log
  line `workload=io error=5` and assert the counter refuses it in both places.
  The control: a clean three-line log gives median and one checksum.
- Command: `cargo test -p podbox-podvm spread`.
- `exit 2` path: none. The aggregation cannot be skipped.
- Result file: `tcg-workload-spread.txt` is superseded for the row section.
  Keep the conditions block and the three platform sections, which the script
  still produces.

**The runs. KEEP-SHELL remainder.** The script keeps sections 1 through 3 and
hands section 4 its three log files to a new
`podbox-spread` binary or, simpler, prints them in the machine-readable form
the Rust function reads. The number `154` stays with the script.

## Group 8 - experiments/157-lock-inheritance-prove.sh

**What it measures.** Two things. Clause 1: do
`a_fork_while_the_lock_is_held_does_not_extend_it` and
`a_spawned_process_does_not_inherit_the_lock`, run **alone** in their own
process, pass `PODBOX_PROVE_RUNS` times each (default 30). Clause 2: removing
the fork defence reddens the fork test and leaves the exec test green. Clause 3:
removing `O_CLOEXEC` does the reverse. Each mutation is asserted to have
landed before either test is read (`:125-135`), and the file is restored from a
copy, never `git checkout --` (`:63-66`).

**Pinned inputs.** `SUBJECT="crates/podbox-image/src/store.rs"` (`:39`), the two
test names, and two exact sed patterns.

**Host assumptions.** `cargo` on PATH; the tree is the podbox tree; the
subject has **no uncommitted changes**, and the script refuses otherwise
(`:54-59`).

**Exit contract.** 2 when cargo is absent, the subject is not here, or the
subject is dirty. 1 when an attempt failed or a mutation was not caught
exactly once. 0 otherwise.

**Timeouts.** None. `cargo test` runs unbounded.

**Positive control.** Each test run 30 times. **Failure control.** The two
mutations, each asserted to land by hashing the file before and after
(`git hash-object`, `:125-130`), which is `scripts/plant.sh` guard 1 applied
inside a script.

**Owning entry.** `TODO/image.md` T-0211. `Prove` names the two `cargo test`
commands and the two mutations. Done 2026-09-12 with `PODBOX_PROVE_RUNS=30`.

**Result file.** `experiments/results/lock-inheritance-prove.txt`, 30 of 30
both, mutation 1 reddening 101/0, mutation 2 reddening 0/101, and
`crates/podbox-image/src/store.rs is back as it was` (`:24`).

**Verdict: RUST-TEST, and the script is broken today.** See finding 2 in the
summary. The measurement is a **fault** test in the `docs/conventions/code.md`
sense: it injects a defect the real service cannot produce on demand, and it
proves two defences are independent. That is exactly what a fault test is for,
and it cannot be a `cargo test`, because a test cannot edit its own subject.

**A defect in the current script, read at file and line.** Clause 2's sed at
`157-lock-inheritance-prove.sh:157` is
`s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/`.
`crates/podbox-image/src/store.rs:1083` reads
`if !sys::close_in_children(fd) {`, inside `Store::try_acquire`, which begins
at `:1074`. The binding is `fd`, obtained from `Lock::open(path)?` at `:1075`.
The pattern matches nothing. The script's guard fires, sets `fail=1`, and
exits 1. The saved result at
`experiments/results/lock-inheritance-prove.txt:16-17` shows
`exited 101 (want non-zero)`, which is the 2026-09-12 state, before the
rename.

Conversion plan:

**Precondition/action/expected.**
Precondition: `cargo` present, `crates/podbox-image/src/store.rs` clean in
both the worktree and the index. Action: run each test alone
`PODBOX_PROVE_RUNS` times; then mutate `Store::try_acquire`'s registration
guard and run both; then mutate `Lock::open`'s flag and run both; then
restore. Expected: `RUNS` of `RUNS` passes, each mutation reddens exactly its
own test, and `git diff --quiet` holds at the end. Rust level: **fault
test**, and it stays a process.

- Crate: new `crates/podbox-rebuild`, binary `podbox-rebuild` under the
  name of its one job: it proves repeatability and mutation independence, and
  T-0211's real need is a nightly or on-demand answer, not a per-commit one
  (30 cargo invocations is not a commit-cost check). Binary name
  `podbox-prove-t0211` is more honest about scope; the implementor should
  choose one and keep it in `Cargo.toml`'s `[[bin]]` with a `--help` that says
  what it proves and what it does not.
- CLI surface: `podbox-prove-t0211 --crate podbox-image --subject
  crates/podbox-image/src/store.rs --runs 30 [--out FILE]`. Exit 0/1/2 as
  above. The third exit code is required here: "the subject is dirty" is a
  state, not a failure, and forcing it into 1 would train a reader to ignore
  the output.
- Fixtures: none. The two mutation patterns must be **anchored to the current
  source** before this tool is written. As of this audit the fork pattern must
  read `s/if !sys::close_in_children\(fd\) \{/if !true \{/` and the comment
  above it should say `Store::try_acquire`, not `Store::hold`
  (`157-lock-inheritance-prove.sh:153-158` still says `Store::hold`). The exec
  pattern at `:162` still matches: `store.rs:1032` reads
  `let flags = sys::O_RDWR | sys::O_CREAT | sys::O_CLOEXEC;`.
- Plant: this tool is a plant harness, so the plant is the harness's own
  negative arm. Add `--dry-run-mutation` that applies the sed and reports
  whether it landed without running any test, then assert that on the current
  tree the fork pattern reports `did not land`. That test fails today and is
  the thing that would have caught the rename.
- Command: `cargo run -p podbox-rebuild -- --crate podbox-image --runs 30`.
- `exit 2` path: preserved, as above.
- Result file: `lock-inheritance-prove.txt` stays, and must be **re-taken**
  before T-0211's Done can stand again. The current file describes a tree that
  no longer exists. Under
  `docs/conventions/prose.md:46` the old measured values must not be replaced
  with new ones from another host; a re-run on the current tree is a new
  measurement and belongs in a new dated file, with the old one kept.

## Group 8 - experiments/161-path-rewrite.sh

**What it measures.** Nine things, per libc, against podbox's own object
driven through podbox. A through J: mapped `open`/`read` equals the real file
with an unmapped control that must fail; stat parity; chdir plus a relative
read; exec of a mapped path from a loaded shell; symlink, link, rename,
mkdir/rmdir; glob expansion; `tar` and `find` under a map (the descriptor-
relative forwarder); a 213-line C program driving the `*at` family,
`eaccess`, exact open mode bits, `linkat`/`symlinkat`/`readlinkat`,
`utimensat`, xattr parity, `ftw`/`nftw` counts, `scandir`, `statfs` and
`mkstemp`, plus the memo proof by dropping privilege; the gap clause I, where
the rootfs's own imports minus what the objects define is empty once the owned
exclusions are removed; and the entry's count, J, of 60 or more defined text
symbols per object.

**Pinned inputs.** `GLIBC_IMG` and `MUSL_IMG` by digest (`:40-41`),
`MAPS="/mapped:/etc,/vbin:/bin"` (`:42`), the C program written inline at
`:69-215`, and `PATH_TAKING` read out of `experiments/100-interpose-symbols.sh`
by `sed` at `:329` rather than copied.

**Host assumptions.** The binary, `cc`, `nm`, `zig` (the musl program must be
dynamic, `:224-227`), and `readelf`. The object must already be built at
`crates/podbox-interpose/target/...` or clause I says so (`:348-349`).

**Exit contract.** 2 when the binary or a tool is absent. 1 on any FAIL. The
whole body is inside a `{ ... } >"$OUT"` block, and the exit is
`exit "$fail"` after it, deliberately not piped to `tee` (`:380-385`). The
comment there is right: a pipeline would report the subshell's 0.

**Timeouts.** None of its own; every `podbox run` and `podbox pull` is
unbounded. **This is a real gap**: the repository rule
`TODO/RULES.md:74` says "Bound external commands, child waits, and network
operations", and this script bounds none of them.

**Positive control.** Clause A2, the unmapped control read, which must fail.
Clause E2's three-way equality. **Failure control.** Clause H's memo proof,
where the child drops to uid 65534 so `chown` returns `EPERM` on any host, and
`SKIP memo proof: cannot drop privilege here` (`:200`) where it cannot. That is
the best instrument in the group and it is the model the others should copy.

**Owning entry.** `TODO/interpose.md` T-0703. `Prove` is
`` `./experiments/161-path-rewrite.sh` exits 0 and `nm -D --defined-only
... | grep -c ' T ' | awk '$1 >= 60'` ``. Done 2026-09-18, and the Done names
the 89 names, the eleven absent ones and the `161` clauses G, G2, H and J2
explicitly.

**Result file.** `experiments/results/interpose-paths.txt`, all clauses green
on both libcs, `ok   I: every gap name is owned`, and
`ok   J: gnu defines 89 (>= 60)` with the same for musl.

**Verdict: SPLIT.** The script is one driver over four separable subjects.

| Piece | Goes to | Why |
| --- | --- | --- |
| H (the C program's 30 asserts) | a tracked C fixture under `experiments/src/`, run by a KEEP-SHELL driver | needs a real loader and two libcs |
| I (the gap is empty) | Rust, integration test | ELF and `nm` facts, deterministic per build |
| J (60 or more defined symbols) | Rust, integration test, same file as 105's A | same subject as check A |
| A through G, J, J2 | keep as a KEEP-SHELL driver | needs `podbox run` and an image |

Conversion plan:

**I and J. Precondition/action/expected.**
Precondition: the debian rootfs extracted by podbox, and both objects built.
Action: enumerate the path-taking libc names the rootfs's own binaries in
`bin usr/bin lib usr/lib sbin usr/sbin` import, with
`readelf -sW --dyn-syms` and `awk '$7=="UND"{print $8}'`, strip `@.*`, sort
unique; read `PATH_TAKING` out of `100-interpose-symbols.sh`; subtract what
`nm -D --defined-only ... | awk '$2=="T"{print $3}'` defines in each object.
Expected: every name in the difference is in
`{mount, umount, umount2, dlopen, dlmopen, execl, execlp, execle, execlpe}`,
and each object defines 60 or more text symbols. Rust level: **integration
test**, because the subject is the built object and the extracted rootfs.

- Target: extend `crates/podbox-interpose/tests/exported_set.rs` (created
  under 105's conversion) with a `gap` module and a `symbol_count` case, so
  105 and 161 share one file rather than two readers of the same object.
- Fixtures: `100-interpose-symbols.sh`'s `PATH_TAKING` block, read at run time
  by the same `sed` as `:329` rather than copied, which is what the comment at
  `:326-328` insists on. The debian rootfs: podbox extracts it in the test's
  setup, or the test takes the rootfs path from `PODBOX_TEST_ROOTFS` and
  refuses without it.
- Plant: delete `mount` from `interpose.map` and from `src/lib.rs` and assert
  the gap test names `mount` as unowned. The control: restore and assert
  empty. For J, lower the threshold constant to 1000 and assert the test fails,
  then restore.
- Command: `cargo test --manifest-path crates/podbox-interpose/Cargo.toml
  --target x86_64-unknown-linux-gnu --test exported_set`.
- `exit 2` path: the object is absent. Same as 105's A: fail loudly, and make
  the build step part of the command.
- Result file: `interpose-paths.txt` keeps clauses A through H and J2; the `I`
  and `J` sections are superseded. Note that its `J` line says **89** while
  105's result says **105** on a later date and the source says **112** today;
  all three are historical and the file must keep the value from its own date.

**H, the C program. KEEP-SHELL remainder.**
- Target: `experiments/src/t161.c`, moved verbatim from the heredoc at
  `:69-215`. The script reads it with `[ -r ... ] || exit 2` and compiles it
  with `cc` and `zig cc -target x86_64-linux-musl -dynamic`, as at `:216-223`.
- Fixtures: `t161.c` itself, now tracked and reviewable, which today it is not.
- Plant: delete one `T(...)` line's assertion (change `T(fd >= 0, ...)` to
  `T(1, ...)`) and assert the script reports `FAIL` for that line. Restore.
- Command: `./experiments/161-path-rewrite.sh; echo EXIT:$?`.
- `exit 2` path: unchanged.
- Result file: `interpose-paths.txt` keeps the H output for both libcs, which
  is 66 `ok` lines each and the most valuable content in the file.

## Group 8 - experiments/200-registry-auth.sh

**What it measures.** Whether podbox logs in to a private registry and pulls,
with the credential never entering the tree's logs, errors or result files.
Seventeen driver clauses: loopback isolation asserted not assumed (no default
route, no name resolves), the required config verifying, an anonymous 401 with a
Basic challenge, `login` storing owner-only and reading back as the login user,
authed pulls by tag and by digest to the seeded digest, a schemeful server,
a wrong password failing naming 401, an anonymous pull failing naming the
missing login, a bogus helper failing naming the helper, and two usage shapes at
125 and 1. The verdict asserts the per-run password is absent from the report.

**Pinned inputs.** `ZOT_VERSION v2.1.21`, `ZOT_BIN_SHA256
d2422616a28dbae10a92c1df9daa23980e2f7f3deb0079968b928f23492196cb`,
`ZOT_BIN_BYTES 85459246`, the checksums URL, a proxy base, `REQ_PORT 5443`,
`REPO_NAME fixture`, `REPO_TAG t1`, `FIX_USER fixture` (`:29-39`). The
password is 16 random bytes per run, never printed, held only in `$WORK`
(`:187-188`).

**Host assumptions.** An engine via `lib/engine.sh`; a driver image built from
`experiments/180-driver.Dockerfile` (`:211`); `openssl`; the binary staged
host-side, built in the lane if absent (`:78-95`). The homes live on tmpfs
inside the container because the Windows-backed `/w` mount reports every mode
as 777 (`:268-271`).

**Exit contract.** 2 for no engine, an unreachable zot, a failed seed, a
failed cert, or a failed driver build. 1 for any refused clause, including the
password reaching the report (`:409-413`). 0 otherwise.

**Timeouts.** `curl --max-time 600` for zot, 60 for the checksums file,
`eng_build 600`, `eng_run 600`, the in-driver poll 20 x 1 s.

**Positive control.** auth-4 and auth-5, where the authed pull inspects to the
seeded digest. **Failure control.** auth-7 (wrong password must fail and name
401), auth-8 (no login must fail naming the missing login), auth-9 (a bogus
helper must fail naming the helper), iso-1 and iso-2, and the password-absence
grep. Each refusal clause uses a **fresh store and a fresh home**, and the
comment at `:299-301` gives the reason: a second pull of a held reference
exits 0 without touching the network, which would let a refusal clause pass
without being refused. That is the sharpest instrument reasoning in the group.

**Owning entries.** `TODO/image.md` T-0209 (`Prove`, Done 2026-09-22) and
T-0206 (`Decision` names the htpasswd-required mode as the shape T-0209 drives;
`Prove` names `180-registry-fixture.sh`).

**Result file.** `experiments/results/registry-auth.txt`, all clauses green, the
password absent. T-0209's Done says "all 15 driver clauses green"; the file
carries **17** `ok` lines inside the driver section (`iso-1`, `iso-2`, `cfg-1`,
`req-1`, and `auth-1` through `auth-11`). The count in the entry is stale; the
clauses themselves agree.

**Verdict: KEEP-SHELL, with a Rust piece lifted out.** The dominant claim is a
live TLS registry on loopback with a real credential, an htpasswd file, and
asserted network isolation. No Rust crate replaces `zot`, `openssl passwd -5`,
`curl --cacert`, or a `--network=none` container. But three of the seventeen
clauses are not about the network at all and are already unit-pinned:
image.md's Done says "17 credentials tests and 10 login parser tests green".

Conversion plan:

**The credential-file clauses. Precondition/action/expected.**
Precondition: a `HOME` on a filesystem with mode bits. Action: `login -u USER
--password-stdin HOST`; stat the stored file; base64-decode the `auth` entry
and read back the user. Expected: exit 0 with `Login Succeeded`, mode 600, and
the decoded entry reads back as the login user. Rust level: **pure unit test**
for the write and the read-back, since both are file operations over a
temporary directory.

- Target: `crates/podbox-image/src/credentials.rs`, the existing
  `#[cfg(test)] mod` at `:462`. Add
  `a_stored_entry_reads_back_as_the_login_user` and
  `a_stored_file_is_owner_only` if the 17 existing tests do not already cover
  them. The implementor should read the 17 first: image.md's Done claims they
  are there, and a second test for a covered fact is cost, not coverage.
- Fixtures: `std::env::temp_dir()` with a per-test name, matching what the
  existing 17 do.
- Plant: drop the `0o600` in the write path and assert the new test fails.
  Restore and assert green.
- Command: `cargo test -p podbox-image credentials::tests`.
- `exit 2` path: none for the file clauses.
- Result file: `registry-auth.txt` is superseded for auth-2 and auth-3.

**The refusals. Precondition/action/expected.**
Precondition: a config naming a helper that does not exist, and a home with no
stored login. Action: pull. Expected: the pull fails and the message names
`podbox-no-such-helper-xyz` (auth-9) or `no login is stored` (auth-8). Rust
level: **pure unit test** for the config parsing and the refusal message, since
both are decided before any socket opens.

- Target: `crates/podbox-image/src/credentials.rs` tests and
  `crates/podbox-image/src/registry.rs` tests. The refusal strings are the
  subject and they are data in the tree.
- Fixtures: a `config.json` string in the test, as the 10 parser tests use.
- Plant: make the reader fall back to anonymous on a helper error, the way
  T-0209's `Approach` forbids, and assert auth-9's equivalent test fails.
  Restore.
- Command: `cargo test -p podbox-image`.
- `exit 2` path: none.
- Result file: superseded for auth-8 and auth-9.

**The registry clauses. KEEP-SHELL remainder.** auth-1, auth-4 through auth-7,
auth-10, auth-11, cfg-1, req-1, iso-1 and iso-2 all need a live server or the
real refusal path through `podbox pull`. The script stays with its number
`200`, its pin block, its tmpfs homes and its per-clause store isolation. The
seeding function at `:138-164` should move to
`experiments/lib/seed-repo.sh` so `180-registry-fixture.sh`, which owns the
fixture, and `200` stop holding two copies; the comment at `:136-137` already
says "this script stands alone, so the seeder rides along", and that is the
duplication to remove.

## Group 8 - experiments/364-qol.sh

**What it measures.** Twelve clauses over three operator verbs, through the
shipped binary. tail-1 to tail-5: `logs --tail 5` prints lines 6 to 10 of a
ten-line log exactly; plain `logs` is byte-identical to `logs --tail 99`;
`--tail 0` is empty at exit 0; a non-count `--tail` is a flag error naming the
value; `-f --tail 3` prints the tail then follows to the end. df-1 to df-4:
`system df` exits 0 with a header and the debian row, billing stored bytes
beside extracted rootfs bytes; the five totals lines are present and the store
is untouched; a digest-pulled alpine dangles and its `Reclaimable` line equals
`image prune -f`'s `Total reclaimed space` string exactly, with df reading 0 B
after; `--bogus` is a flag error and `--help` prints usage at 0. doctor-1 to
doctor-3: `doctor` exits 0 with profile tcg, the kvm node line and the ceiling
line, and no fix line; with `PATH` stripped of the emulator it exits 1 naming
`qemu-system-x86_64 --version` with `fix: install QEMU`; `--bogus` and `--help`
behave.

**Pinned inputs.** `DEBIAN` by digest (`:42`). `FLAG` is **not written down**:
it is read out of the binary by `scripts/common/exit-codes.sh` at `:58-60`,
which is the rule `TODO/cli.md` T-0802 exists to enforce.

**Host assumptions.** A Linux lane, a release binary, and a network for the
two pulls. It runs under `sh`, sources only `exit-codes.sh`, and uses
`timeout` on every call.

**Exit contract.** 2 when the binary is not executable, the exit-code table
cannot be read, or a pull fails (`:56-69`, `:180-182`). 1 on any missed clause.
0 otherwise. Note the last line is `[ "$fail" -eq 0 ]`, so 0 and 1 only reach
the shell; 2 is taken on the explicit paths.

**Timeouts.** `timeout 600` on both pulls, `timeout 120` on everything else.

**Positive control.** tail-2, the byte-identity between plain `logs` and a wide
tail, which passes only if the default was left untouched. **Failure control.**
tail-4, df-4 and doctor-3 each ask for a bad flag and require the flag-error
code, which is the same refusal in three verbs. doctor-2 strips `PATH`
entirely (`env "PATH=$WORK/empty"`, `:264`), which is a real fault condition
rather than a mock. df-3's prune comparison is a cross-check between two verbs
on the same fact, to the byte.

**Owning entry.** `TODO/cli.md` T-1337. Three Done paragraphs, 2026-09-26, one
per verb, each naming the clauses and the result file. The header says "One
script growing verb by verb; each commit drives the whole script, so every verb
stays green beside the new one" (`:6-7`).

**Result file.** `experiments/results/qol.txt`, all 12 clauses green,
`verdict QOL TAIL+DF+DOCTOR SERVED`, `fail=0`. cli.md's Done quotes
`26.9 MiB stored beside 74.6 MiB rootfs` and the `3.7 MiB` prune string; the
file carries both.

**Verdict: RUST-TEST, in two levels, and the rest is KEEP-SHELL.** This is the
script with the most clauses and the most of them are pure logic on
deterministic inputs that already has unit tests.

What already exists. `crates/podbox-supervise/src/lib.rs:352` `follow_from`,
`:382` `tail_offset`, with tests at `:660-668`. `doctor.rs` has tests at
`:365` and cli.md's Done records "health agrees with the problem list (fail
carries exactly one fix, unmeasured outranks failed), help and bad flags need
no probe". `system.rs` has tests at `:603` and the Done records
"`reclaimable_bytes` counts an unshared blob once and only where no survivor
needs it, `dir_bytes` counts once and never follows, df help and bad flags need
no store, short IDs are twelve hex digits". **The Done paragraphs name the unit
tests that already cover the pure halves.** The script covers the rest: that the
shipped binary wires them to the right store and the right file.

Conversion plan:

**A. Tail. Precondition/action/expected.**
Precondition: a captured ten-line log file and a container record naming it.
Action: `logs --tail 5`, `logs`, `logs --tail 99`, `logs --tail 0`,
`logs -f --tail 3`. Expected: the last five lines exactly; plain identical to
the wide tail; empty at exit 0; a non-count refused at the flag-error code
naming the value; the last three then the rest to the end. Rust level:
**integration-and-deployment proof** for the first half, because it needs a
container and a real captured file, and **pure unit test** for `tail_offset`
and `follow_from`, which the crate already has.

- Integration target: new `crates/podbox-cli/tests/logs_tail.rs`, module
  `logs_tail`. It is a `tests/*.rs` file because it drives the binary, and it
  is the first `tests/` directory in `podbox-cli`; `crates/podbox-ssh/tests/`
  is the only precedent, with `common.rs` as its shared helper
  (`crates/podbox-ssh/tests/common.rs`).
- Fixtures: none new. The container is created with `create`/`start`/`wait`
  exactly as `364-qol.sh:71-76` does, against a scratch `PODBOX_STORE` under
  `std::env::temp_dir()`. The ten-line payload is the same
  `/bin/bash -c 'for i in 1 2 3 4 5 6 7 8 9 10; do echo line$i; done'`.
- Plant: change `tail_offset` at `crates/podbox-supervise/src/lib.rs:382` to
  return 0 always, and assert `logs_tail` fails on tail-1 while the existing
  `tail_offset_names_the_last_lines` at `:660` also fails. The control: assert
  the file's own no-op case passes on the restored tree.
- Command: `cargo test -p podbox-cli --test logs_tail`.
- `exit 2` path: no image, no binary. The test must then fail, because there is
  no third state. The gate already pulls the debian image, so the fixture is
  available where the gate runs.
- Result file: `qol.txt` keeps the tail sections.

**B. `system df`. Precondition/action/expected.**
Precondition: a store with one tagged image and one digest-pulled dangling
image. Action: `system df`, then `image prune -f`, then `system df` again.
Expected: the row bills stored beside rootfs in bytes not `0 B`; the five
totals lines are present; the image count is identical before and after df; the
`Reclaimable` string equals prune's `Total reclaimed space` string exactly;
df after prune reads `0 B`. Rust level: **integration test** over a real store
directory, plus the pure unit tests that already exist.

- Integration target: `crates/podbox-cli/tests/system_df.rs`, module
  `system_df`. Build the store through `podbox-image`'s own store API rather
  than through a pull, so the test needs no network: `Store::reclaimable_bytes`
  and `Store::blob_bytes` are already the gate chain
  (`crates/podbox-image/src/space.rs`).
- Fixtures: a store under `temp_dir()` seeded with two blob entries and two
  `index.json` records, one tagged and one not.
- Plant: make `Store::reclaimable_bytes` count a survivor's blob, and assert
  the test fails on the "only where no survivor needs it" clause. The control:
  restore and assert green.
- Command: `cargo test -p podbox-cli --test system_df`.
- `exit 2` path: none; a store is a directory, not a service.
- Result file: `qol.txt` keeps the df sections.

**C. `doctor`. Precondition/action/expected.**
Precondition: a `PATH` with no emulator. Action: `doctor`. Expected: exit 1
with `fail qemu-system-x86_64 --version` and `fix: install QEMU`. Rust level:
**fault test**, because "no emulator on PATH" is a condition a real service
cannot produce on demand and it is exactly what `docs/conventions/code.md`
names a fault test for.

- Target: `crates/podbox-cli/src/doctor.rs`, the existing `#[cfg(test)] mod`
  at `:365`, for the renderer, plus a new
  `crates/podbox-cli/tests/doctor_legs.rs` for the process-level fault. The
  leg names and the leg renderer are already public from
  `podbox_probe::machine` (cli.md's Done says so and the import should be
  read before this is written), so the wording cannot drift.
- Fixtures: `PATH` set to an empty directory, as `364-qol.sh:263-264` does.
- Plant: drop the emulator from the required-leg list in
  `crates/podbox-probe/src/machine.rs` and assert the test fails. Restore.
- Command: `cargo test -p podbox-cli --test doctor_legs`.
- `exit 2` path: an unmeasured required leg is a **value** here, not a state:
  `doctor` exits 2 for it (cli.md's Done says so) and the test asserts that as
  an exit code of the binary under test, which a Rust test can read. This is
  the cleanest instance in the group of the third state surviving as data.
- Result file: `qol.txt` keeps the doctor sections.

**The script remainder.** `364-qol.sh` keeps the two pulls and the store
fixture and loses all twelve clauses. It becomes a seeding step for the tests,
or it is deleted and the seeding moves into the test files. Its number `364`
stays with whatever remains, per `experiments/README.md:4`.

## Group 8 - experiments/392-kvm-guest.sh

**What it measures.** Whether the machine tier's KVM route returns guest
output and a guest exit status, with the emulator command line watched from
outside. The 13-line shell file is a shim: it resolves `HERE` and `ROOT`,
checks for a `py` launcher, and `exec`s `scripts/windows/kvm-guest.py` (`:9-11`).
The Python launcher requires `--accept-host-risk`, `--binary` and `--image`,
converts the host paths to `/mnt/<drive>/...`, concatenates
`experiments/lib/kvm-owned.sh` and `experiments/lib/kvm-guest-base.sh` into one
driver, runs it through `wsl-toolkit --instance podbox base exec --timeout 65m`,
and rewrites host paths to `$BINARY`, `$IMAGE`, `$SCRATCH` and `$BASE_HOME` labels
before writing `experiments/results/kvm-guest.txt` (`kvm-guest.py:44-87`).

**Pinned inputs.** The explicit binary and image paths, the image length and
digest pinned in `experiments/lib/kvm-guest-base.sh`, and the base's own
resources: the launcher refuses without `--accept-host-risk`
(`kvm-guest.py:33-39`) and again if `wsl-toolkit` is absent (`:40-43`).

**Host assumptions.** A Windows host, a WSL instance named `podbox`, a licensed
operator disk, at least 6144 MiB free in the base, and no other emulator
running. `experiments/README.md:56-58` records that nested KVM stopped the
Windows host on 2026-09-30 and that an agent must not run this proof
unattended.

**Exit contract.** 2 when `--accept-host-risk` is absent, `wsl-toolkit` is
absent, a path is not a local-drive file, or the toolkit call throws. 0 only
when the toolkit exit is 0 **and** the report carries
`verdict KVM-GUEST-OK` (`kvm-guest.py:85-87`). Otherwise 1, or 2 where the
toolkit returned 2.

**Timeouts.** `--timeout 65m` on the toolkit call, `timeout=4200` on the
subprocess. The in-guest bound is set in the driver.

**Controls.** The positive control is the emulator command line read from
outside, an independent observer rather than podbox's own report
(`experiments/results/kvm-guest.txt:41-43` shows `-accel kvm -cpu host`).
The failure control is the residue check at `:50-67`: no per-run scratch
directory remains, no owned emulator remains, owned scratch removed. Two of the
three residue lines are FAIL in the saved run.

**Owning entries.** `TODO/gate.md` T-1350 (`Prove` names the command; Partial
2026-09-30, and the Partial is accurate). `TODO/milestones.md` T-1112
(`Prove` names it among three, Status `partial`, and the record says "A
successful guest run remains acceptance"). `TODO/RESUME.md:17` names it as the
next command.

**Result files.** `kvm-guest.txt` is the current one and it **fails**:
`FAIL: exit 42 failed with 125`, `FAIL: owned emulator remains after the
command`, `FAIL: owned emulator still exists`, `verdict KVM-GUEST-FAIL`,
`proof exit: 1`. Dated siblings: `kvm-guest-2026-09-29.txt`,
`kvm-guest-2026-09-30-first.txt`, `-second.txt`, `-third.txt`.

**Verdict: KEEP-SHELL.** This is the one script in the group that genuinely
needs a host tool no Rust crate can replace, and the justification is
specific, not general.

The dependencies are `wsl-toolkit --instance podbox base exec`, a nested-KVM
QEMU on `/dev/kvm`, an OVMF pflash pair from `/usr/share/edk2-ovmf`, a licensed
910,163,968-byte operator disk, and a QMP socket. A Rust binary would be a
client of all five and a provider of none. The 13-line shell file itself adds
nothing: it is a path resolver and a `py` lookup, and
`kvm-guest.py:33-39` is where the real logic is. **The 13 lines should be
deleted anyway**, because `AGENTS.md:65` says "On Windows, use `wsl-toolkit
--instance podbox`. Do not call `wsl.exe`", and a shim whose only job is to
find a Python launcher adds a path to fail. The launcher should be invoked
directly and `392-` retired to a pointer, or the number carried by the launcher
and the shell file deleted under `experiments/README.md:4`'s "do not reuse a
number" rule, which is about reuse, not about deletion.

What is worth lifting to Rust is small and is already lifted: the unit test
`a_noisy_emulator_is_not_blocked_on_its_own_output` that T-1112's record names,
which is the source defect 1112 found and fixed. The rest needs the host.

## Group 8 - experiments/395-reconcile-repository.py

**What it measures.** Which changes the retained publish branch
`origin/publish/20260926T052210Z` adds to `main`. It prints the conditions, the
`main` and `origin/main` commits, every `origin` ref, then `git cherry main
PUBLISH` and a `range-diff` between two hardcoded ranges. With
`--expect-deleted` it asserts the ref is gone and exits 0, or exits 1 if the
ref is present. Without it, a missing ref is `exit 2`.

**Pinned inputs.** The branch name (`:11`) and two hardcoded commit ranges
(`:47`): `ce17a733^..82c12ba1` against `82d4bff^..1c19cea`.

**Host assumptions.** A git checkout with `origin` reachable. Every `git`
call is bounded at 60 s, and `show-ref` at 30 s.

**Exit contract.** 2 on `OSError`, `RuntimeError` or `SubprocessError`, and
when the ref is required and absent. 1 when `--expect-deleted` and the ref is
present. 0 otherwise, including the `REPOSITORY-READ` case, which is a
**reading, not an assertion**: `verdict REPOSITORY-READ: compare the first
patch source; do not infer identity from ancestry` (`:48`).

**Timeouts.** 60 s per `git`, 30 s for `show-ref`.

**Positive control.** `--expect-deleted`. **Failure control.** None. There is
no clause that can go red on a wrong answer; the script reports and a human
reads. The saved result agrees:
`experiments/results/publish-branch-comparison.txt` ends with the range-diff
and no verdict line matching `REPOSITORY-OK`.

**Owning entry.** **None.** No `TODO/*.md` entry names
`experiments/395-reconcile-repository.py`; the only file naming it in the
repository is `experiments/README.md:38`, which lists it in the audit proof
table with "Git refs; patch identity, range comparison, and optional
deleted-branch check". That is a table row, not an entry. T-1405 in
`TODO/podssh.md` documents the publish-branch **fallback**, and
`docs/conventions/git.md` is the rule it serves.

**Result file.** `experiments/results/publish-branch-comparison.txt`,
2026-09-30T01:43:13Z, showing `main 005638db` and `origin/main 9d4bb558` as
**different commits** at the time of the run.

**Verdict: RUST-TOOL.** This is a one-off comparison of a branch that has since
been published and removed. As a durable tool it belongs beside the other
repository tools, not in `experiments/`.

Conversion plan:

**Precondition/action/expected.**
Precondition: a checkout whose `origin` is fetchable, and a publish branch
named on the command line rather than written into the source. Action: print
`main` and `origin/main`; list `origin` refs; run `git cherry` and a
`range-diff`; with `--expect-deleted`, assert the named ref is absent.
Expected: with `--expect-deleted`, exit 0 when absent and 1 when present.
Without it, print the comparison and exit 0 as a reading. Rust level: not a
test. It is a developer tool.

- Crate: new `crates/podbox-reconcile`, binary `podbox-reconcile`.
  Alternatively a `[[bin]]` under an existing crate; the implementor should
  prefer the new crate because nothing in the ten existing crates owns git.
- CLI surface:
  `podbox-reconcile --branch <ref> [--expect-deleted] [--base A..B --compare
  C..D]`. The branch moves off the source: `:11` currently hardcodes
  `origin/publish/20260926T052210Z`, which is one session's branch written into
  a tracked script, and `docs/conventions/prose.md:22` requires a claim to name
  its evidence rather than carry a stale identifier. The two hardcoded ranges
  at `:47` move to arguments for the same reason.
- Fixtures: none. It shells to `git`, bounded, reading each status from the
  process that produced it.
- Plant: run it against a branch that does not exist. Assert exit 2 and the
  `cannot run: the comparison requires the retained publish ref` message. That
  is the control, and it is the only assertion the script has today.
- Command: `cargo run -p podbox-reconcile -- --branch origin/publish/<name>
  --expect-deleted`.
- `exit 2` path: preserved. A tool may carry three states.
- Result file: `publish-branch-comparison.txt` is **superseded** and stays
  tracked as history for that comparison. `docs/conventions/prose.md:45-47`
  keeps historical results historical; do not re-run it and overwrite.
- Note the observed drift: the file records `main 005638db` against
  `origin/main 9d4bb558`, so the checkout was ahead of or behind `origin` when
  the comparison ran. Any re-run must state both commits, as the script does.

## Group 8 - experiments/399-publication.py

**What it measures.** Read-only CI and release acceptance at an exact commit.
It asserts `origin` is the approved HTTPS remote, that `remote main` equals
local `HEAD`, that `gate.yml` completed with a `success` conclusion **on that
commit and that branch**, and with `--deleted-branch` that the named branch is
absent from `origin`. With `--release TAG` it validates the tag format
`v<major>.<minor>.<patch>-beta.<n>`, resolves the tag's commit, asserts
`nightly.yml` succeeded on that commit and that tag, reads the architecture
matrix out of `.github/workflows/nightly.yml` at that tag, builds the expected
asset set (`podbox-<arch>`, `podbox-ssh-<arch>.tar.gz`, and `.sha256` and
`.sigstore` for each), and asserts every one is present and non-empty. The doc
string says "No remote writes" (`:2`).

**Pinned inputs.** `REPO = 'Azathothas/podbox'` (`:13`) and the two remote
allowances at `:51`.

**Host assumptions.** `git` and `gh` on PATH with authentication, and
`GIT_TERMINAL_PROMPT=0` and `GH_PROMPT_DISABLED=1` set in the child
environment (`:18`). Network access to github.com.

**Exit contract.** 1 on `ValueError`, which covers the remote mismatch, the
HEAD mismatch, a failed workflow, a malformed tag, a non-unique architecture
matrix, a wrong tag identity, a prerelease flag, a present obsolete branch, and
missing or empty assets. 2 on `OSError`, `KeyError`, `RuntimeError` or
`TimeoutExpired`. 0 on `verdict PUBLICATION-OK`.

**Timeouts.** 120 s per command, `stdin=DEVNULL`, and a workflow list capped at
30 runs (`:29`).

**Positive control.** None in the strict sense: the whole script is a set of
refusals. **Failure control.** `--deleted-branch` is the near-miss clause, and
the release path is guarded by a `re.fullmatch` on the tag format before
anything is resolved. The script also has the defect class `T-0709` was amended
for: it filters `matching[0]` by commit and branch, so a wrong commit finds no
match and raises rather than reading a different run's verdict (`:31-33`).

**Owning entry.** `TODO/podssh.md` T-1405, **in its Done record only**. The
`Prove` is `` `sh scripts/package-ssh.sh RELEASE_DIR ARCH QEMU` exits 0 ``;
`experiments/399-publication.py` appears once, at `:159`, inside the Done
paragraph, as `` `py experiments/399-publication.py --release v0.1.0-beta.10`
reports PUBLICATION-OK at build commit `d6cb926` with 7 architectures and 42
required non-empty assets ``. So the script has a result file and a claim but
no `Prove`.

**Result file.** `experiments/results/publication.txt`, 2026-09-30, commit
`d6cb926`, `gate.yml: success`, `nightly.yml: success`,
`7 architectures; 42 required non-empty assets`, `verdict PUBLICATION-OK`.
Also `publication-beta12.txt`.

**Verdict: RUST-TOOL.** This is a release gate. It reads a remote and a CI
conclusion; a test cannot own that, and an operator will want it before every
tag.

Conversion plan:

**Precondition/action/expected.**
Precondition: `git` and `gh` authenticated, a checkout whose `origin` matches
`https://github.com/Azathothas/podbox(.git)`, and local `HEAD` equal to
`origin`'s `main`. Action: the checks above. Expected: `PUBLICATION-OK` and
exit 0. Rust level: not a test. It is a release gate, and
`docs/conventions/code.md` calls that deployment proof.

- Crate: new `crates/podbox-publish`, binary `podbox-publish`. It sits beside
  `crates/podbox-reconcile` and the two should share one `git` helper module
  rather than each calling `git` itself; two crates with one helper is the
  duplication the repository forbids.
- CLI surface:
  `podbox-publish [--release vX.Y.Z-beta.N] [--deleted-branch NAME] [--repo
  Azathothas/podbox] [--out FILE] [--timeout 120]`. Exit 0/1/2 as above. It
  prints the conditions block the experiments require: UTC time, scope, local
  and remote commits, every workflow conclusion with its URL, and the asset
  count.
- Fixtures: none. It shells to `git` and `gh`, bounded, with `stdin` closed and
  the prompt environment disabled exactly as `:18` does.
- Plant: `scripts/plant.sh` case. Point `--repo` at a repository that has no
  such release, and assert the tool refuses naming the missing assets rather
  than printing PUBLICATION-OK. The control: run against the real tag and
  assert 0. **A plant that runs against the real repository must be
  non-destructive**: this tool must never gain a write verb.
- Command: `cargo run -p podbox-publish -- --release v0.1.0-beta.10`.
- `exit 2` path: preserved verbatim, including `ValueError` meaning a refusal
  and the other four meaning a state.
- Result file: `publication.txt` is **superseded** by a run of the tool and
  must be re-taken, since it names a release that is now behind
  `beta.12`. Keep `publication-beta12.txt` and both, per
  `docs/conventions/prose.md:45-47`.
- The T-1405 `Prove` gap is a finding below, not part of this plan.

## Group 8 findings

**1. T-0701's Done record states a symbol count that the source contradicts
three ways.** `TODO/interpose.md` says "17 declared, 17 exported, and 0
`rust_eh_personality`". `crates/podbox-interpose/interpose.map` declares 112
names today, counted from the file with the same `awk` the script uses
(`105-interpose-ownership.sh:121-122`). `experiments/results/interpose-ownership.txt:25-27`
records 105 on 2026-09-19. `TODO/gate.md:472` records 112, measured
2026-09-23. The 112 figure is corroborated by
`crates/podbox-interpose/src/lib.rs`, which carries 51 `no_mangle` attributes
plus the macro families T-0703's Done describes. The entry most stale is the
one that owns the measurement. It matters because the count is the assertion:
a reader comparing "17" with today's object concludes the interposer lost 95
entry points.

**2. `157-lock-inheritance-prove.sh` cannot land its clause-2 mutation, and
would exit 1 saying so.** `157-lock-inheritance-prove.sh:157` matches
`if !sys::close_in_children(lock.fd) {`.
`crates/podbox-image/src/store.rs:1083` reads
`if !sys::close_in_children(fd) {`, and the enclosing function is
`Store::try_acquire` (`:1074`), not `Store::hold` as the comment at
`157-lock-inheritance-prove.sh:153-155` says. The script's own guard at
`:128-135` prints "THE MUTATION DID NOT LAND, so nothing below was measured",
sets `fail=1`, and the run exits 1. The saved result
(`experiments/results/lock-inheritance-prove.txt:16-17`) is from 2026-09-12
and predates the rename, and `TODO/image.md`'s T-0211 Done quotes it. **T-0211's
closure record is currently unreproducible.** The script's guard is the right
guard and it is what caught this; that is worth saying plainly.

**3. `experiments/README.md:35` presents a failing proof in the passing table.**
The row reads "`392-kvm-guest.sh` | Windows toolkit base, explicit Linux binary,
pinned operator disk; guest version, exit 42, run path, and cleanup" under the
heading "Repository audit proofs | Script | Required conditions and result".
`experiments/results/kvm-guest.txt:46` reads `FAIL: exit 42 failed with 125`,
`:52-53` read `FAIL: owned emulator remains after the command` and
`FAIL: owned emulator still exists`, and `:69-70` read `verdict KVM-GUEST-FAIL`
and `proof exit: 1`. Both owning entries are accurate: T-1112 is `partial` and
T-1350 is `partial`, and T-1350's Partial names the failed repetitions. The
README table is the only place the failure is not stated.

**4. T-1101's Done quotes a probe-parity reading the saved result contradicts.**
`TODO/milestones.md` records "`15 matched, 1 recorded divergence, 0 differed, 0
missing`" and shows `kcmp` as the recorded divergence.
`experiments/results/probe-parity.txt:6-9` reads `rows_matched 16`,
`rows_recorded 0`, `rows_differed 0`, `rows_missing 0`, dated 2026-09-23, and
`:56` reads `kcmp(-1,-1,...) [control]|denied|3`. The `30-` refresh that
T-1213 drove changed the reference file, so the kcmp row now matches and the
recorded-divergence arm in `130-probe-parity.sh:173-181` no longer fires. T-1213's
Done (gate.md) correctly says "16 matched and 0 differed". **Two entries
disagree about the same saved file and the newer one is right.**

**5. T-0209's Done undercounts its own clauses.** image.md says "all 15 driver
clauses green". `experiments/results/registry-auth.txt` carries 17 `ok` lines
between `== driver run (network none)` and `driver: all clauses held`: `iso-1`,
`iso-2`, `cfg-1`, `req-1`, and `auth-1` through `auth-11`. The script has
`fails` as a counter and reports "driver: $fails clause(s) failed", so the
count is available; the entry's prose number is hand-written and drifted.

**6. Three entries in this group have no `Prove` that names their script.**
T-0809's `Prove` does name `151-spawn-ambiguity.sh` and T-1337's Done does name
`364-qol.sh`, so those are fine. But: `399-publication.py` is named only in
T-1405's Done, never in its `Prove`; `395-reconcile-repository.py` is named by
no entry at all, only by `experiments/README.md:38`; and `120-reproducible-build.sh`
is T-1004's `Prove` and that is correct. An acceptance command that no `Prove`
owns is a measurement no reviewer can re-run from the record.

**7. No script in this group is in the gate.** `scripts/dev.sh:215-229` lists
the check steps and none of them is a group-8 script. T-1209 item 2 asked for
the exported-symbol check to move into `dev.sh check`; the Done for that item
is not in this tree's `TODO/gate.md` beyond the Ruled paragraph, and the
`dev.sh` list I read does not include it. So check A, the only cheap and total
check in the group, runs on demand and in no automated gate.

**8. One instrument in the group has no failure control at all.**
`120-reproducible-build.sh` has no positive control and no failure control: no
case can make the two builds differ, and nothing proves the two builds saw the
same tree. It is the `Prove` of a closed entry (T-1004) and its result is
`MATCH`. A check that has never been seen red is exactly what
`scripts/plant.sh:6-9` says is not an assertion. Its conversion plan carries
the plant.

**9. `161-path-rewrite.sh` bounds nothing.** Every `podbox run`, `podbox pull`
and `podbox extract` in that script is unbounded, which
`TODO/RULES.md:74` requires ("Bound external commands, child waits, and
network operations"). `105-interpose-ownership.sh` bounds every engine call;
`161` does not. Not a conversion issue, but it is a live rule violation in a
script that keeps its remainder after the split.

**10. `200-registry-auth.sh` carries a second copy of T-0206's seeder.** The
comment at `:136-137` says "this script stands alone, so the seeder rides
along", and `seed_repo` at `:138-164` is that second copy. `180-registry-fixture.sh`
owns the fixture. Two copies of a seeder that writes OCI layout bytes by hand
is a second declaration of a format, which is what
`docs/conventions/code.md:3` forbids.

**11. Dead or near-dead: none confirmed.** I did not find a script whose
subject no longer exists. `120`'s lane dependency, `392`'s `wsl-toolkit`,
`395`'s publish branch and `399`'s network are all still live. `392`'s 13-line
shim is the closest thing: it has one caller, the operator, and its logic is
already in `scripts/windows/kvm-guest.py`. That is a finding, not a deletion I
am authorising.

**12. What I could not settle.** Whether `161`'s clause I is currently green.
The clause reads `PATH_TAKING` out of `100-interpose-symbols.sh` and subtracts
it from the rootfs's imports, and I read the code but not the current
`PATH_TAKING` list, so I cannot say whether the gap is still empty. Running
`161` is out of scope. What would settle it: reading
`experiments/100-interpose-symbols.sh`'s `PATH_TAKING` block and comparing it
with `nm -D --defined-only` over a freshly built object, which needs a build.
Likewise I cannot say whether `experiments/results/interpose-ownership.txt`'s
105 and today's 112 differ by seven real entry points or by seven
committed-since changes; the git log on `interpose.map` shows four commits
after the map's last recorded run, and mapping each to a name count is work I
did not do.

## Group 8 - entries read

Read in full, with the script work each one owns.

| Entry | File | What it establishes for this group |
| --- | --- | --- |
| T-0701 | `TODO/interpose.md:23` | `Prove` names `105-...sh` and check A; the four cdylib constraints; the Done that states 17 symbols |
| T-0702 | `TODO/interpose.md:127` | `60-interposer-libc.sh` check A as the `Premise`; the 2026-09-08 re-measurement quoted from `interposer-libc.txt`; the SONAME conclusion |
| T-0703 | `TODO/interpose.md:283` | `Prove` is `161-path-rewrite.sh` exits 0 plus the `nm` count at 60; the Done's 89 names and the eleven absences |
| T-0704 | `TODO/interpose.md:405` | `Prove` is `105-...sh` exits 0; the control-comes-first rule; the fakeroot errno correction |
| T-0709 | `TODO/interpose.md:804` | the reader's four implementation findings; why check F exists and what it asserts |
| T-0710 | `TODO/interpose.md:936` | `105-...sh` check G as the `Prove`; the 4 MiB ceiling; the fd-9 versus fd-17 lane trap |
| T-1101 | `TODO/milestones.md:60` | `Prove` is `130-probe-parity.sh` exits 0; the three clauses; the Done that finding 4 contradicts |
| T-1112 | `TODO/milestones.md:855` | `Prove` names `392-kvm-guest.sh` among three; Partial 2026-09-30; the stream defect and its unit test |
| T-0809 | `TODO/cli.md:717` | `Prove` is `151-spawn-ambiguity.sh`; the two walls behind one string; the Done's five unit tests in `report.rs` |
| T-1337 | `TODO/cli.md:1258` | three Done paragraphs, 2026-09-26, each naming `364-qol.sh` clauses and `qol.txt` |
| T-0206 | `TODO/image.md:430` | the zot fixture `200` depends on; the htpasswd-required mode; the licence determination |
| T-0209 | `TODO/image.md:772` | `Prove` is `200-registry-auth.sh` exits 0; the seventeen driver clauses; the count finding |
| T-0211 | `TODO/image.md:1054` | the two tests, the two mutations, and the Done that finding 2 makes unreproducible |
| T-1004 | `TODO/packaging.md:172` | `Prove` is `120-reproducible-build.sh`; the reporting requirement; the MATCH result |
| T-1308 | `TODO/podvm.md:719` | `Prove` is `154-tcg-workload-spread.sh` one row per class; the three records and the two issue-53 notes |
| T-1209 | `TODO/gate.md:634` | item 2 asks for check A in `dev.sh check`; the 112-name lane measurement at `:472` |
| T-1210 | `TODO/gate.md:788` | `105-...sh` as the `Source` for the engine conversion |
| T-1211 | `TODO/gate.md:881` | the same, for the distribution and probe scripts |
| T-1212 | `TODO/gate.md:935` | the same, for the image, registry and CLI scripts |
| T-1213 | `TODO/gate.md:1102` | `130-probe-parity.sh:1-40` as the `Source`; the Done that re-drove 130 to 16 matched |
| T-1350 | `TODO/gate.md:1775` | `Prove` is the `392-...` command; Partial 2026-09-30; the four artifacts |
| T-1405 | `TODO/podssh.md:142` | the `399-publication.py` claim in the Done; the 7 architectures and 42 assets |

Twenty-one entries, above the ten required. Supporting source read for the
plans: `crates/podbox-supervise/src/lib.rs:352,382,660-668`;
`crates/podbox-probe/src/report.rs:343-376,755,1147-1180`;
`crates/podbox-probe/src/probes.rs:211,1103-1115`;
`crates/podbox-probe/src/select.rs:255,302,458-558`;
`crates/podbox-enter/src/abi.rs:157(system.rs),1074`;
`crates/podbox-image/src/store.rs:1032,1074,1083,2019,2114`;
`crates/podbox-image/src/credentials.rs:462`;
`crates/podbox-cli/src/doctor.rs:365`; `crates/podbox-cli/src/system.rs:123,157,603`;
`scripts/windows/kvm-guest.py:1-94`; `scripts/common/exit-codes.sh:1-58`;
`scripts/plant.sh:1-60`; `scripts/dev.sh:195-245`;
`crates/podbox-interpose/interpose.map`; `crates/podbox-cli/src/interpose.rs`
(read for the `place` call T-0702's Done names).
