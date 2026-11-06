# Group 5 audit

Scope: the twelve scripts in `refactor/00-orientation/assignment-map.md`
`## Group 5`, 2,902 lines. No script was run. Every claim below cites a file
and line I read.

## Group 5 summary

| verdict | scripts | lines |
| --- | --- | --- |
| DELETE | 1 | 115 |
| RUST-TEST | 4 | 872 |
| RUST-TOOL | 2 | 962 |
| KEEP-SHELL | 3 | 764 |
| SPLIT | 2 | 189 |
| total | 12 | 2902 |

Line counts are `wc -l` on the twelve files. They match the map's per-script
numbers exactly.

Three findings that matter most:

1. **`experiments/320-cli-contract.sh` clause 3 is recorded against a table
   that has moved.** `experiments/results/cli-contract.txt:24` records six
   verbs, `restart` first. In the current source
   `crates/podbox-cli/src/parity.rs:261` is
   `Row { verb: "restart", flag: Option::None, status: Native, ... }`, and the
   first six `None` verb rows are now `top`, `attach`, `pause`, `unpause`,
   `stats`, `diff` (33 in total). The script takes its verbs from the table at
   run time (`experiments/320-cli-contract.sh:215`), so it would still pass.
   The saved result is stale evidence about a different set, and the entry
   `TODO/cli.md` T-0801 quotes `164 rows` against a source table of `267`
   `Row {` entries.

2. **`experiments/50-interpose-tier.sh` is dead.** No `TODO/*.md` entry names
   it any more. `TODO/interpose.md:365` records that its own `Prove` "named
   `50-interpose-tier.sh` until 2026-09-18" and replaced it with `161`,
   because the script "measures pathmap, somebody else's object". There is no
   `experiments/results/interpose-tier.txt`. The only other references are in
   `references/Azathothas__container-research/tree/` and
   `docs/history/experiments-README.md-before-2026-09-30.txt`, both read-only
   or superseded.

3. **`scripts/plant.sh` is the only script here that the required gate cannot
   lose.** `.github/workflows/gate.yml:56` runs `./scripts/plant.sh` as the
   step named `every check can fail`. It is Bash-only (`local` at
   `scripts/plant.sh:124`), and `scripts/windows/run-in-base.sh` selects the
   interpreter from the job's first line. It is therefore a tool, not a
   measurement, and it must not be deleted without replacing that CI step.

## Group 5 - experiments/146-podvm-initramfs.sh

**What it measures.** Six clauses over pinned inputs. Pull and extract
`public.ecr.aws/docker/library/alpine@sha256:3e9b4b…` (line 53), wrap the
rootfs as a newc cpio with an appended archive carrying `dev/console 5:1` and
an `/init` override, boot `vmlinuz-virt` under TCG to `VMR-GUEST-READY`
(line 168), and prove a stalled origin exits 28 inside 10 s (line 210).

**Pinned inputs.** Image digest at line 53. Kernel URL and sha256 at lines
54-55. `BOOT_TIMEOUT` 180 s and `CURL_TIMEOUT` 60 s at lines 56-57.

**Host assumptions.** Linux, native or in a job container. Requires
`qemu-system-x86_64`, `cpio`, `python3`, `curl`, `sha256sum`, `timeout`
(line 69). Sources `experiments/lib/podvm-guest.sh` (line 49) for the archive
writer. `set -u`, no `pipefail`, so it also runs under dash.

**Exit contract.** 0 match, 1 tested and failed, 2 could not run. Three
`exit 2` paths: line 42 scratch, line 67 binary missing, line 70 tool absent.

**Timeouts.** Boot under `timeout "$BOOT_TIMEOUT"` (line 168). Kernel fetch
`curl --max-time "$CURL_TIMEOUT"` (line 159). Stalled-origin clause `timeout
10` over a 2 s curl ceiling (line 210).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at line 43.

**Positive control.** None. Every clause asserts a success or a refusal.

**Failure control.** The stalled-origin clause (clause 6) is the one arm that
fails if the bound is absent, and it runs on every invocation
(lines 20-23, 185-216).

**TODO entries that own it.** T-1303 (`TODO/podvm.md:314`, `Prove:` at 353);
T-1313 (`TODO/podvm.md:844`, `Prove:` at 883). T-1302 at `TODO/podvm.md:212`
is named in the header as the emulator driver's owner and is not driven here.

**Result file.** `experiments/results/podvm-initramfs.txt`, 32 lines, dated
2026-09-21. It agrees with T-1303's table: base 8,365,568 bytes and full
8,366,168 bytes (result lines 16, 19; entry lines 363-364), `dev/console` in
the appended archive (line 22), marker printed (line 27), qemu rc 124 (line
26), stalled origin exit 28 (line 30).

**Verdict: SPLIT.**

Clause 6 is the only piece that is a deterministic measurement of podbox
rather than of QEMU. Clauses 1 to 5 measure a guest boot, which needs the
emulator and the kernel download.

- Clauses 1 to 5 stay a driven proof. They cannot become a unit test: the
  subject is a kernel unpacker reading a concatenated archive.
- Clause 6 becomes a Rust unit test. It measures podbox's own HTTP client's
  ceiling, and it needs no emulator.

**Conversion plan, clause 6.**

- Assertions today, as a triple.
  - Precondition: a loopback TCP listener that has accepted one connection
    and never answered. Action: one registry fetch with a 2 s ceiling.
    Expected: the fetch fails with a timeout-class error rather than hanging.
- Level: **pure/fault unit**. `docs/conventions/code.md` line 28: "Use fault
  tests for conditions a real service cannot produce on demand." A stalled
  origin is exactly that. A mock would prove nothing; a real socket costs
  milliseconds and no emulator.
- Target file: `crates/podbox-image/src/transport.rs`, `#[cfg(test)] mod
  tests`. That file already owns the request path; the test needs no new
  module.
- Fixtures. No new fixture. `std::net::TcpListener::bind("127.0.0.1:0")` in the
  test, one `thread::spawn` that accepts and sleeps, and the existing timeout
  field on the transport. `experiments/lib/*` is not reusable here; it holds
  shell helpers.
- Plant: a `#[test]` that removes the timeout from the request and asserts the
  test then hangs is not a plant. The plant is the inverse: set the timeout to
  a value longer than the stall and assert the test fails, which is the same
  defect the script's clause 6 exists to catch.
- Command: `cargo test -p podbox-image transport` on Linux. No feature flag,
  no target triple.
- `exit 2` becomes a `#[ignore]`-free skip decision. Rust has no third exit.
  The lane states it in the test name and the message: the test asserts the
  timeout is bounded, and a machine where the stall cannot be arranged is a
  host fact, not a pass. Follow `TODO/RULES.md` section 6: a missing test is
  not a denial.
- Result file. `experiments/results/podvm-initramfs.txt` is kept whole. It is
  the evidence for T-1303's clauses 1 to 5, which do not move. The entry
  T-1313's `Prove:` names `146` and must be repointed at the Rust test; the
  saved file stays as the historical run it is.

## Group 5 - experiments/150-image-acquisition.sh

**What it measures.** Four clauses. The digest `podbox images` reports equals
`docker image inspect` for the same tag (line 134), a second pull fetches
nothing (line 171), every stored blob hashes to its own name (line 193), and
an `http://` registry is refused by name rather than downgraded (line 212).

**Pinned inputs.** `REFERENCE` defaults to
`ghcr.io/pkgforge-dev/archlinux:latest` (line 40), overridable by
`PODBOX_TEST_IMAGE`. Driver image is the pinned debian digest (line 64).
The default is a tag, not a digest, and the script handles the race by
re-pulling both once on a mismatch (lines 148-166).

**Host assumptions.** Linux executes natively; every other OS stages the
binary inside a debian driver through `experiments/lib/engine.sh` (lines
45-84). Needs an engine, or `exit 2` at line 60.

**Exit contract.** 0, 1, 2 as declared at line 25. Five `exit 2` paths,
including two that exit 2 when a registry pull fails (lines 130, 136, 140).

**Timeouts.** `pb 600 pull`, `pb 60 images`, `pb 30 pull` for the http
refusal.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` at line 49.

**Positive control.** Clause 2's `Already exists` text is a positive
control that the second pull really re-resolved the image.

**Failure control.** Clause 4 asserts exit 124 is a failure (line 217), which
is the hang the refusal exists to prevent.

**TODO entries that own it.** T-1102 (`TODO/milestones.md:129`, `Done:` at
152); T-0201 and T-0202 (`TODO/image.md:17` and `TODO/image.md:74`, both name
it at lines 46 and 112); T-0206 (`TODO/image.md:430`, `Source:` names it);
T-1212 (`TODO/gate.md:934`, lists its 23 docker calls, `Prove:` at 964).

**Result file.** `experiments/results/image-acquisition.txt`, 24 lines, dated
2026-09-23. `digests_match yes`, `blobs_stored 7`, `blobs_mismatched 0`,
`second_pull_fetched 0`, `http_refused_rc 1`. T-1212's transcript at
`TODO/gate.md:985` prints the same digest `b2507f19…`. Consistent.

**Verdict: SPLIT.**

- Clauses 1 and 4 are **RUST-TEST**. They need no engine and no network.
- Clauses 2 and 3 are **RUST-TEST** too, against a fixture registry rather
  than a live one. T-0206 already built that fixture; see the plan below.

**Conversion plan, clause 1 (digest parity).**

- Precondition: a store holding one pinned reference's manifest, index and
  config bytes. Action: read the digest `images` reports for it. Expected:
  the digest equals the one computed over the bytes the registry served for
  the reference asked for, which for a multi-platform tag is the index
  digest, not the per-platform manifest digest.
- Level: **pure**. The subject is `podbox_image`'s record construction, which
  takes bytes as input. `docs/conventions/code.md` line 27.
- Target: `crates/podbox-image/src/digest.rs`, `#[cfg(test)] mod tests`, beside
  the code that computes it. The test is in-crate because `Record` and
  `Digest` are crate-private shapes.
- Fixtures: `crates/podbox-image/src/layout.rs:433` already has
  `rootfs_tar()` for a rootfs. Add an index-plus-manifest-plus-config byte
  fixture beside it and share it through a `pub(crate) mod fixtures` or a
  second `#[cfg(test)]` helper. No reuse of `experiments/lib/*`.
- Plant: assert against a manifest digest where the index digest is correct
  and assert the test fails. That is the exact defect T-0202 names.
- Command: `cargo test -p podbox-image digest`.
- `exit 2` becomes the engine-absent path. With the fixture there is no engine,
  so the arm disappears rather than being converted. State that in the entry.
- Result file: `experiments/results/image-acquisition.txt` is superseded for
  clause 1 by the Rust test. Move it to `docs/history/` when T-1102's `Prove:`
  is repointed, because the saved run is the evidence that docker and podbox
  agreed on a real host, and no unit test reproduces a docker daemon.

**Conversion plan, clause 4 (http refusal).**

- Precondition: a reference whose scheme is `http://`. Action: `pull` it.
  Expected: exit non-zero, the message names HTTPS-only, and it does not hang.
- Level: **pure**. `Reference::parse` refuses a scheme before any socket.
- Target: `crates/podbox-image/src/reference.rs`, `#[cfg(test)] mod tests`, for
  the refusal, plus one integration test for the whole binary's message.
- Fixtures: none. A literal string.
- Plant: parse the same reference with the scheme check deleted and assert the
  test fails.
- Command: `cargo test -p podbox-image reference`.
- Result file: same disposition as clause 1.

**Conversion plan, clauses 2 and 3 (second pull, blob naming).**

- Precondition: a store holding a pinned image's blobs. Action: request the
  same reference again, then hash every blob under `blobs/sha256/`. Expected:
  no layer is fetched, and every blob's bytes hash to the name it is stored
  under.
- Level: **integration**, against the loopback registry fixture T-0206 built.
  `docs/conventions/code.md` line 29: "Use integration and deployment proof
  for the actual default path." The default path here is a real HTTPS request
  to a registry.
- Target: a new `crates/podbox-image/tests/store_digest.rs`. This is the first
  image integration test; only `crates/podbox-ssh/tests/` exists today.
- Fixtures: the loopback registry from T-0206. `TODO/image.md:452` names it as
  "a reader over `crates/podbox-image/src/store.rs` rather than a second
  store". Reuse that; do not write a third one. It serves HTTPS with a
  generated certificate, so the test needs a trust anchor the product already
  accepts through `--tls-verify=false`.
- Plant: plant the OCI layout with the wrong digest in `index.json` and assert
  the test fails with the mismatch. That is `T-0202`'s own invariant.
- Command: `cargo test -p podbox-image --test store_digest`.
- `exit 2` becomes: a test that needs the fixture must build it in the test,
  so there is no third state. A host that cannot bind loopback is the only
  gap, and `-` is the honest record.
- Result file: `experiments/results/image-acquisition.txt` is superseded for
  clauses 2 and 3 by the fixture run. Move it with clause 1's.

## Group 5 - experiments/158-interpose-embedding.sh

**What it measures.** Whether a `build.rs` can invoke cargo without
deadlocking on the parent's package-cache lock. Two shapes on a two-crate
fixture with no workspace (lines 67-90): plain, and with a separate
`CARGO_TARGET_DIR`.

**Pinned inputs.** None beyond the toolchain. `BOUND` is 120 s by default
(line 37).

**Host assumptions.** `cargo` and `timeout` on PATH (lines 41-48).

**Exit contract.** 0 a nested build completed, 1 every shape failed or hung,
2 could not run (lines 30-31).

**Timeouts.** `timeout "$BOUND"` on every clause, line 129.

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM`, line 39.

**Positive control.** None. The clauses assert completion.

**Failure control.** The `timeout 124` arm at line 140 is what a deadlock
looks like. The script's own guard at lines 118-126 refuses to report a
clause whose `build.rs` did not write whole; that guard exists because the
first version reported a compile failure as a lock finding (lines 112-117).

**TODO entries that own it.** T-0702 (`TODO/interpose.md`, `Status note:`
at line 249: "`experiments/158-interpose-embedding.sh` ran a nested build in
two shapes on 2026-09-12 and **both completed**").

**Result file.** `experiments/results/interpose-embedding.txt`, 23 lines,
2026-09-12, `cargo 1.98.1`, commit `411663a`, both clauses exit 0 in 0 s,
"a build.rs CAN invoke cargo on this host, in 2 of 2 shapes."

**Verdict: DELETE.**

The measurement has already been taken and its question answered, and the
answer is recorded. T-0702's `Status note` reads the result and refuses the
nested shape anyway: "It is still refused, because it would fire on somebody
else's machine and copying cannot." A host-and-day reading does not become a
test. The hazard T-0702 acts on is real and permanent, and it is enforced
structurally instead: `crates/podbox-cli/build.rs` copies what
`scripts/build-interpose.sh` left behind (T-0702 `Status note:`), so no
`build.rs` runs cargo at all.

Keep `experiments/results/interpose-embedding.txt`. The entry cites the
reading and the file is its evidence. Retiring the number and its script does
not retire the record; `experiments/README.md` line 3 says "Its number is
permanent."

Deletion order, so nothing dangles: repoint T-0702's `Status note:` at the
result file rather than at the script, then delete the script, then record the
number as retired under `experiments/README.md`.

## Group 5 - experiments/320-cli-contract.sh

**What it measures.** Five clauses against the shipped binary: the parity
table is data (line 141), the table decides rather than describes (line 175),
every `None` verb answers with docker's 125 and its own row (line 215),
`docker` and `podman` on PATH run the payload and say which tool ran (line
245), and the `docker` name is refused where a daemon answers unless
`--force` (line 288).

**Pinned inputs.** `IMAGE` defaults to `ghcr.io/pkgforge-dev/archlinux:latest`
(line 46). Driver image is the pinned debian digest (line 60). The table
itself is read out of the binary, not written here.

**Host assumptions.** `jq` on PATH or `exit 2` (line 105). An engine or
`exit 2` (line 56). `scripts/common/exit-codes.sh` reads the exit-code table
out of the binary (line 114), so no number is written twice.

**Exit contract.** 0, 1, 2 as declared at line 29. The script uses a fourth
state: `skipped=1` at line 313 makes the run exit 2 at line 329 when the
refusal half of clause 5 could not be measured because no daemon answered.

**Timeouts.** `pb 300 run`, `pb 900 run -i`, `pb 60` elsewhere.

**Cleanup.** `trap 'eng_cleanup; rm -rf "$WORK"' EXIT INT TERM`, line 44.

**Positive control.** Clause 2's `run -i (a Stub)` arm at line 197 asserts a
`Stub` is accepted, which is the mirror of the `None` arm.

**Failure control.** Clause 5 records `a docker daemon here: reachable` and
asserts the other half where it cannot (line 311). That is a reported
unmeasured half, not a pass.

**TODO entries that own it.** T-0801 (`TODO/cli.md:18`, `Prove:` at 54);
T-0803 (`TODO/cli.md:280`, `Prove:` at 312); T-0808 (`TODO/cli.md:632`, names
it in `Problem:` at line 643); T-1212 (`TODO/gate.md:934`, 21 docker calls,
`Prove:` at 964); T-1103 (`TODO/milestones.md:268`, names it at line 290);
T-0505 (`TODO/enter.md:339`, `Prove:` names clause 4); T-1417
(`TODO/cli.md:1604`, names `373`, not this script).

**Result file.** `experiments/results/cli-contract.txt`, 40 lines, dated
2026-09-23, engine 29.8.1, jq-1.8.2. Rows 164, verbs 57, statuses
`Degraded Native None Stub`, rows with no note 0.

**Verdict: SPLIT.** Four pieces, three destinations.

- Clauses 1, 2, 3: **RUST-TEST**.
- Clause 4: **RUST-TEST**, and it needs argv[0], which a unit test cannot
  set. It becomes a re-exec through the built binary.
- Clause 5: **KEEP-SHELL**, on the reachable-daemon half.

**Conversion plan, clauses 1 to 3.**

- Precondition and action, per clause.
  1. Precondition: the table compiles into the binary. Action: render
     `{{json .Parity}}` and parse it. Expected: at least 60 rows, every
     `status` one of `Native`, `Degraded`, `Stub`, `None`, and every row
     carries a non-empty note.
  2. Precondition: a row with `status: None` and a flag exists. Action:
     invoke that verb with that flag. Expected: exit `EXIT_FLAG_ERROR` and
     the message quotes the row's own `note`. Second action: invoke a verb
     with a flag no row names. Expected: exit `EXIT_FLAG_ERROR` and the
     message says "no row in the parity table". Third action: invoke a `Stub`
     flag. Expected: accepted, exit 0.
  3. Precondition: a verb row with `status: None` exists. Action: invoke
     that verb with no arguments. Expected: exit 125 and the printed note is
     the table's `note`, byte for byte.
- Level: **pure** for clause 1, **integration** for clauses 2 and 3. The
  message text is printed by `parity::admit` (`parity.rs:597`) and by
  `main.rs`'s unknown-verb arm (`main.rs:236`), and a caller sees those bytes
  only through the process. `docs/conventions/code.md` line 32: "A test must
  reject the defect it claims to detect." A test that calls `admit` in-crate
  and reads the table in-crate cannot catch `podbox create --no-steps`, which
  is the defect T-0801's door sweep found.
- Target: a new `crates/podbox-cli/tests/parity_contract.rs`. This is a
  `tests/` integration test because it must re-exec the binary. Only
  `crates/podbox-ssh/tests/` exists today; this is the first CLI one.
- Fixtures: build the binary once in the test with
  `env!("CARGO_BIN_EXE_podbox")`, which cargo provides for integration tests.
  No store, no image, no network: a `None` verb refuses before any of them,
  which is the point of the table.
- Plant: `TODO/interpose.md`'s own defect class. Insert a verb row for a verb
  no parser serves, or drop the `admit_all` call from one parser, and assert
  the test fails. `admit_all` exists at `parity.rs:635` and is what makes an
  unlisted flag unreachable.
- Command: `cargo test -p podbox-cli --test parity_contract`.
- `exit 2` becomes: no third state. The gate needs no `jq` and no engine, so
  both `exit 2` arms disappear. Record that in T-0801's `Prove:`.
- Result file: `experiments/results/cli-contract.txt` is superseded for
  clauses 1 to 3 by the Rust test and moves to `docs/history/`. T-1212's
  `Prove:` names this script and must be repointed at the new test plus
  whatever still needs the engine.

**Conversion plan, clause 4 (argv[0] multicall).**

- Precondition: the binary exists and is invocable under the name `docker`.
  Action: `docker run --rm IMAGE /bin/echo hi` with the podbox store. Expected:
  stdout is exactly `hi`, exit 0, and stderr carries a banner naming
  `invoked as \`docker\``.
- Level: **integration**. This is the only level that can set argv[0].
  `docs/conventions/code.md` line 33 warns that a mock does not prove a live
  deployment; a symlink and a real process is the real thing.
- Target: `crates/podbox-cli/tests/argv0_multicall.rs`, beside
  `parity_contract.rs`. It needs an image and a store, so it is the one test
  that cannot use an empty store; give it the pinned M5 alpine digest the rest
  of the tree uses.
- Fixtures: `names::ALIASES` is private to the crate, so the test derives the
  names from `podbox system info --format '{{json .Parity}}'` rows for
  `docker`, `podman` and `podvm`, exactly as `experiments/320-cli-contract.sh`
  does by hand. The banner text is in `names.rs:65`, produced by
  `alias_note()` at `names.rs:50`.
- Plant: delete the `if let Some(note) = crate::names::alias_note()` block at
  `run.rs:1390` and assert the test fails on the missing banner while stdout
  and the exit code stay green. That isolates exactly the clause.
- Command: `cargo test -p podbox-cli --test argv0_multicall -- --test-threads=1`.
  `--test-threads=1` because both names write the same store.
- `exit 2` becomes: no image and no store is a test that cannot set up, which
  cargo reports as a failure. State in the entry that this test needs a pull.
- Result file: superseded for clause 4, moved to `docs/history/` with the
  rest.

**Conversion plan, clause 5 (the docker-name ruling). KEEP-SHELL.**

The refusal half depends on a reachable docker daemon. `names.rs:94`
`docker_daemon()` connects to `/var/run/docker.sock` or `$DOCKER_HOST` and
asks `/_ping`. A unit test cannot arrange a daemon, and the entry says so:
`TODO/cli.md:334` records "a machine with no daemon reports the refusal half
as unmeasured rather than as a pass".

- Keep a driver for the refusal half only, and make it small. The half that
  `podbox system install-names` on a daemon-free machine already has a
  unit-testable shape: `names.rs:333` asserts every alias has a row, and
  `names.rs:347` asserts the daemon check answers one of three states.
  Neither asserts the refusal.
- What a Rust integration test can take: install into a temp dir with
  `$DOCKER_HOST` pointing at a socket the test itself listens on and answers
  `200` to. That is a fake daemon, and `docs/conventions/code.md` line 33
  says a mock does not prove a live service. It proves the branch, not the
  deployment.
- So: **the fake-daemon branch becomes a Rust integration test** in
  `crates/podbox-cli/tests/install_names.rs`, and the **real-daemon half
  stays a tracked shell driver**, reduced to the install, the refusal, the
  `--force` override, and the symlink assertion. That driver is
  KEEP-SHELL: the host dependency is a reachable docker daemon, which no
  crate can arrange.
- Level for the Rust half: **fault**, because a live daemon is a condition a
  test cannot produce on demand (`code.md` line 28).
- Target: `crates/podbox-cli/tests/install_names.rs`; `#[cfg(unix)]`, because
  `names.rs` uses `UnixStream` and `/proc/self/exe`.
- Plant: delete the `Daemon::Reachable` arm at `names.rs:251` and assert the
  test fails with the test's own message, not with a generic one.
- Command: `cargo test -p podbox-cli --test install_names`.
- Result file: the daemon-free reading moves to `docs/history/`; the
  daemon-present reading is kept as the only evidence that the ruling
  behaves as the operator ruled it.

## Group 5 - experiments/340-detached-stdio.sh

**What it measures.** Whether `podbox run -d` returns before its payload
ends, when the caller captures the id with `id="$(...)"` (line 127). Forty
iterations by default (line 55), each asserting a return inside 5 s
(line 62), a state of `running` (line 153), and a non-zero launcher pid
(line 165).

**Pinned inputs.** `IMAGE` defaults to
`ghcr.io/pkgforge-dev/archlinux:latest` (line 54). Payload
`/bin/sleep 20` (line 59).

**Host assumptions.** Linux native only: `[ -x "$BIN" ] || exit 2` at line
64, and `nproc` at line 73.

**Exit contract.** 0, 1, and 2 for a pull failure (line 96). The script
ends `exit "$fail"` at line 212, where `fail` is 0 or 1.

**Timeouts.** `timeout 1800` on pull and extract (lines 92, 98),
`timeout 300` on `run -d`, `timeout 60` on `inspect` and `logs`, `timeout
120` on `rm`.

**Cleanup.** `trap cleanup EXIT INT TERM` at line 52 kills every burner with
`kill -9`. A final sweep removes every container the script started (lines
180-182), which is what keeps a `sleep 20` per iteration from outliving it.

**Positive control.** The one `inspect` reading four fields at once (line
150), so all four describe the same read.

**Failure control.** None in the script. The load it keeps is not a control;
it is the condition the entry named. The control exists elsewhere: T-0608's
record shows the same command returning in under a second with stdout
redirected to a file instead of captured
(`TODO/supervise.md:614`), which is the negative arm.

**TODO entries that own it.** T-0608 (`TODO/supervise.md:529`, `Prove:` at
581, re-run at 634).

**Result file.** `experiments/results/detached-stdio.txt`, 11 lines,
2026-09-09, `prompt_and_running 40`, `slowest_run_d_s 1`, `reproduced 0`.
T-0608's recorded run at `TODO/supervise.md:636` reads the same 40 of 40 and
the same 1 s. Consistent.

**Verdict: RUST-TEST, with one clause that cannot move.**

The instrument is the caller's own stdout, and a Rust test can be that
caller exactly: `Stdio::piped()` on `Command::new(podbox)` gives a real pipe,
and reading it to EOF is what command substitution does.

**Conversion plan.**

- Assertions today, as a triple.
  - Precondition: a container that runs a 20 s payload. Action: start it
    detached and read the child's stdout to EOF, which is what
    `id="$(podbox run -d ...)"` does. Expected: stdout yields the container
    id and EOF arrives within 5 s, while the container is still running.
  - Second action: `inspect --format '{{.State}} {{.Pid}}
    {{.LauncherPid}} {{.ExitCode}}'`. Expected: state `running` with a
    non-zero launcher pid.
- Level: **integration**. The subject is a forked process's descriptor
  lifetime. A unit test inside `podbox-supervise` runs in the process whose
  descriptors matter, so it cannot be the caller. `docs/conventions/code.md`
  line 29.
- Target: `crates/podbox-supervise/tests/detached_stdio.rs`. A new `tests/`
  directory in a crate that today has none; every current test is inline
  (`launcher.rs:646` is the only `#[cfg(test)] mod` in the crate).
- Fixtures: none needed beyond a pulled image and `PODBOX_STORE` set to a
  temp dir. `table::dir` and `table::log_path` already give the store its
  shape. Keep the load: spawn `nproc` busy loops from the test, and kill them
  in a `Drop` guard exactly as the script's `cleanup` does at line 48.
- Plant: delete the `/dev/null` `dup2` triple at `launcher.rs:256-268` and
  assert the test fails on the EOF timing, not on the payload. The source
  comment at `launcher.rs:232` names this exact regression and the date it
  was measured.
- Command: `cargo test -p podbox-supervise --test detached_stdio`. Linux
  only, `--test-threads=1` because the load is process-wide.
- `exit 2` becomes: no image is a pull failure. The test must pull, or take
  a fixture image path from an env var and refuse with the test's own message
  when it is absent. `TODO/RULES.md` section 6: a missing test is not a
  denial, so the absence must be named rather than read as a pass.
- Result file: `experiments/results/detached-stdio.txt` is superseded by the
  Rust test. Move it to `docs/history/` beside the earlier pre-fix evidence
  T-0608 already quotes inline.

**What does not move.** The `ls -l /proc/<launcher>/fd` reading at line 145 is
best-effort diagnostic text, written only when a defect reproduces. It has no
assertion and does not need a port.

## Group 5 - experiments/358-ladder-rungs.sh

**What it measures.** Seven clauses over the launch ladder. Forced `rundir`
enters with `PODBOX_ACTIVE_MODE=rundir` (line 112); forced `cache` refuses
naming `PODBOX_CACHE=1` without it and enters with it (lines 121-124);
forced `tmpfs` takes whichever arm the host grants (lines 138-158); forced
`fuse` likewise (lines 173-193); forced `memfd` runs a static hello (line
220); an unknown word refuses with the rung list (line 224); and a packed
OCI-layout tarball saves, loads, runs, and refuses a flipped byte as a digest
mismatch (lines 236-284).

**Pinned inputs.** `ALPINE` is the pinned alpine 3.20 digest (line 40). The
static hello is built in the script from a heredoc (line 201).

**Host assumptions.** `/bin/sh` script run from the checkout root, `REPO`
taken from `pwd` (line 34) because the Windows wrapper enters `/work` first.
Needs `cargo`, `cc`, and the interposer objects
(`scripts/build-interpose.sh`, line 60). Builds `target/…/debug/podbox`.

**Exit contract.** Declared at lines 28-29 as 0, 1, 2. The script's own
paths `exit 2` on a missing toolchain, a failed bootstrap, a failed pull, and
a failed static compile, and `exit 1` on a failed build, a failed import, and
a missing binary. The last line `[ "$fail" -eq 0 ]` yields 0 or 1.

**Timeouts.** 1200 on bootstrap and interpose, 1800 on the build, 600 on the
pull, 120 on every `step`.

**Cleanup.** `WORK` is under the checkout and is removed first (line 36). The
`/out` copy at line 292 hands evidence back through the job boundary.

**Positive control.** Clause 5's static hello enters `memfd` and prints
`static-hi`, which is the entry arm against every refusal arm around it.

**Failure control.** Clause 7's flipped byte: the loader must refuse naming
the digest mismatch (line 284). That is the one clause that cannot pass
without reaching its subject.

**TODO entries that own it.** T-1003 (`TODO/packaging.md:113`, `Prove:` at
135 and again at 148). T-1421-adjacent evidence is in `TODO/cli.md:1604`,
which names `373` and not this script.

**Result file.** `experiments/results/ladder-rungs.txt`, 138 lines, dated
2026-09-30, podbox 0.1.0-beta.10. Verdict `LADDER RUNGS HOLD`. T-1003's
`Done:` at `TODO/packaging.md:150` describes the same run and states the
limit: "live FUSE and tmpfs entry are unproved here (`/dev/fuse` absent,
`mount(2)` denied even in a userns, all measured)". The result file agrees:
`tmpfs arm REFUSAL` at line 68 and `fuse arm REFUSAL` at line 83.

**Verdict: SPLIT.**

- Clauses 1, 2, 5, 6: **RUST-TEST**. Deterministic rung selection on
  synthetic availability.
- Clauses 3 and 4: **RUST-TEST, fault level**, with the refusal arm as the
  only one any reachable host can drive, exactly as the script says at lines
  132-137 and 165-172.
- Clause 7: **RUST-TEST**, mostly already in `layout.rs`.
- The bootstrap and build preamble: **not** a measurement. It is a lane
  preamble and it belongs to the driver that runs the tests, not to a test.

**Conversion plan, clauses 1, 2, 5, 6 (rung selection).**

- Assertions today, as a triple.
  - Precondition: every rung leg reported available, `PODBOX_MODE=rundir`
    set. Action: choose. Expected: `Mode::Rundir`, and the refusal list never
    names it.
  - Precondition: `PODBOX_MODE=cache` with no `PODBOX_CACHE`. Action:
    choose. Expected: refusal naming `PODBOX_CACHE=1`.
  - Precondition: the payload is a static ELF. Action: choose with
    `PODBOX_MODE=memfd`. Expected: `Mode::Memfd`, no refusal.
  - Precondition: `PODBOX_MODE=memfd2`. Action: choose. Expected: refusal
    naming `memfd, fuse, tmpfs, rundir, cache`.
- Level: **pure**. `podbox_enter::ladder::choose` at `ladder.rs:258` takes an
  `Availability` struct and returns a `Mode`. That is a function on data.
  `code.md` line 27.
- Target: `crates/podbox-enter/src/ladder.rs`, `#[cfg(test)] mod tests`,
  beside `the_cache_runs_only_when_asked_for` at line 379. The rung words the
  script greps for are in the refusal string at `ladder.rs:71`.
- Fixtures: `ladder.rs:306` and `ladder.rs:317` already provide `all_up()` and
  `all_down()`. Add per-rung availability setters; no new fixture family.
- Plant: delete the forced-mode branch in `choose` and assert the test fails
  for `rundir`. `ladder.rs:341` already tests that a forced mode that is down
  refuses rather than falling through; the complement is what the script
  measures.
- Command: `cargo test -p podbox-enter ladder`.
- `exit 2` becomes: no third state. `Availability` is constructed in-crate.
- Result file: `experiments/results/ladder-rungs.txt` is superseded for
  clauses 1, 2, 5, 6.

**Conversion plan, clauses 3 and 4 (tmpfs and fuse, both arms).**

- Precondition: a host that grants `mount(2)` or opens `/dev/fuse`, or a host
  that denies it. Action: force the rung. Expected on the granting host: exit
  0, `PODBOX_ACTIVE_MODE` names the rung, `/etc/alpine-release` reads, and no
  staging directory survives. Expected on the denying host: exit
  `EXIT_RUNTIME_ERROR`, the message names the mount or `dev/fuse`, and no
  staging survives.
- Level: **fault**. `code.md` line 28. The entry arm needs a mount a real
  service cannot grant on demand; the refusal arm is the only one reachable
  here, and T-1003 says so at line 156.
- Target: `crates/podbox-enter/src/ladder.rs`, `#[cfg(test)] mod tests`, for
  the availability mapping (`tmpfs_is_up_only_where_a_mount_attached` at
  `crates/podbox-cli/src/ladder.rs:410` is the same shape on the CLI side).
  The refusal text assertions go in the same module.
- Fixtures: a `#[cfg(unix)]` test that calls `unshare(CLONE_NEWUSER |
  CLONE_NEWNS)` and attempts the mount, treating `EPERM` as the denial arm
  and a success as the entry arm. On a host that grants it, the same test
  takes the other arm, which is what the script's `if [ "$rc" -eq 0 ]` does.
- Plant: change the refusal string and assert the test fails on the missing
  `mount` or `dev/fuse` substring, not on the exit code. The entry T-1003's
  `Done:` depends on the refusal naming the cause.
- Command: `cargo test -p podbox-enter fuse -- --test-threads=1`.
- `exit 2` becomes: the test reports which arm it took in its assertion
  message, so a denial is a pass and a grant is a pass, and the report says
  which.
- Result file: kept. It is the only reading that says the tmpfs and fuse
  entry arms are unproved, and T-1003's `Done:` cites that limit.

**Conversion plan, clause 7 (save, load, corrupt refusal).**

- Precondition: a store holding one image. Action: `save -o`, then `load -i`
  into a fresh store, then run over the loaded record. Expected: `Loaded
  image:` printed, the run enters `rundir`, `/etc/alpine-release` reads, and
  no staging survives. Then: flip one byte in the largest blob, repack beside
  the untouched index, and load again. Expected: non-zero exit naming the
  digest mismatch, and the bytes stay out of the store.
- Level: **integration**. `layout::save` at `layout.rs:49` and `layout::load`
  at `layout.rs:130` take a `Store` and a path; both are crate-visible but
  the save side writes to a `dyn Write` the test can supply directly, so this
  is one level lower than the script and still a real round trip.
- Target: `crates/podbox-image/tests/layout_roundtrip.rs`, extending the
  `#[cfg(test)] mod tests` at `layout.rs:423` where it can stay. Prefer the
  in-crate module: `save`, `load` and `commit_staged` are private, and the
  existing tests at lines 449 and 464 are already there.
- Fixtures: `rootfs_tar()` at `layout.rs:433` builds the tarball bytes. Build
  a full OCI layout from it: `oci-layout`, `index.json`, and the config and
  layer blobs, then `save` writes the tarball and `load` reads it back. The
  corrupt arm repacks with one byte changed, which is what the script does
  with `dd` at line 269.
- Plant: remove the `commit_staged` digest comparison at `layout.rs:204` and
  assert the test fails with the mismatch message. That is the defect the
  clause exists to catch.
- Command: `cargo test -p podbox-image layout`.
- `exit 2` becomes: the static hello no longer needs `cc`, so that arm
  disappears. Say so in T-1003's `Prove:`.
- Result file: superseded for clause 7, moved to `docs/history/`; kept for
  clauses 3 and 4 as above.

**What happens to the preamble.** Lines 49-77 bootstrap the lane, build the
interposer, and build the debug binary. That is the driver, not the
measurement. After conversion it belongs to whatever runs the suite, which is
already `scripts/dev.sh` (`dev.sh:222` builds the interposer before the
binary) and `.github/workflows/gate.yml`.

## Group 5 - experiments/365-namespace.sh

**What it measures.** The denied-host half of the namespace rung: the probe
selects below `namespace` (line 56), a run enters `chroot` with no fallback
line on the banner (line 66), and `.EnteredRung` reads `chroot` (line 80).

**Pinned inputs.** The pinned debian digest (line 29).

**Host assumptions.** `/bin/sh`, checkout from `pwd` (line 22). Linux, and a
lane where `unshare` and `mount` are refused. Requires the binary at line 43
and a successful pull at line 51, both `exit 2`.

**Exit contract.** Declared at lines 16-17. The script `exit 2` at lines 23,
27, 43, 46, 52 and ends `[ "$fail" -eq 0 ]`.

**Timeouts.** 600 on the pull, 120 on every podbox call.

**Cleanup.** `WORK` removed first at line 27. `podbox rm ns365` at line 76.

**Positive control.** Clause 3 asserts the positive value `chroot`, not the
absence of an error.

**Failure control.** Clause 2 asserts the banner does **not** say
"entered chroot instead" (line 70). A run that fell back and said so fails.

**TODO entries that own it.** T-1339 (`TODO/enter.md:687`, `Prove:` at 758).

**Result file.** `experiments/results/namespace.txt`, 25 lines, dated
2026-09-26, podbox 0.1.0-beta.7. Verdict `NAMESPACE LANE SERVED`, `fail=0`.

**Verdict: RUST-TEST.**

Clause 1 is the odd one and the entry says so. The result at line 15 records
`ok: probe selects supervise where unshare is refused`, while the script's
own question names `chroot` and T-1339's `Prove:` at `TODO/enter.md:759`
says "probe below namespace". The script accepts any non-`namespace` word
(line 57), and `supervise` is one of five rung words in
`crates/podbox-probe/src/select.rs:48`. The entry's summary and the saved
reading use different words for the same clause.

**Conversion plan.**

- Assertions today, as a triple.
  1. Precondition: a machine where the namespace legs are denied. Action:
     `Selection::choose(&findings)`. Expected: the selection is not
     `Rung::Namespace`.
  2. Precondition: a rootfs with no namespace rung. Action: build the entry
     banner. Expected: `mode=chroot`, and no line names a fallback.
  3. Precondition: a completed run on such a machine. Action: read
     `.EnteredRung`. Expected: `chroot`.
- Level: **pure** for 1 and 3, **integration** for 2. `fields_from` at
  `system.rs:620` and `entry_banner` at
  `crates/podbox-probe/src/report.rs:54` both take constructed inputs, so the
  banner is a function on data.
- Target: clauses 1 and 3 into
  `crates/podbox-cli/src/system.rs` `#[cfg(test)] mod tests`, beside
  `the_entered_rung_is_namespace_where_selected` at line 632; clause 2 into
  `crates/podbox-probe/src/report.rs` `#[cfg(test)] mod tests`.
- Fixtures: a hand-built `Findings` with the namespace legs refused. T-1339's
  `Done:` names eight existing unit guards, including
  `the_fallback_line_names_the_cause_and_the_rung`, so the fixture shape
  exists. Reuse it; do not write a second.
- Plant: make `entered_rung` return `Rung::Namespace` unconditionally and
  assert the banner test fails naming `mode=`.
- Command: `cargo test -p podbox-cli system` and `cargo test -p podbox-probe
  report`.
- `exit 2` becomes: a host that grants namespaces is the opposite condition
  and the clause cannot run there. The Rust test constructs the denial, so
  the clause becomes runnable everywhere, which is a gain the script could
  not claim. Record that.
- Result file: superseded. Move to `docs/history/` with the rest.

## Group 5 - experiments/373-store-gates.sh

**What it measures.** Five clauses. An empty store lists nothing and marks
nothing, and `doctor store_exec` prints `yes` (lines 107-113). `inspect`
exposes the image `Config` in `--format` and in the JSON document (lines
123-128). A noexec store refuses `run` pre-pull at 125 naming the directory
and `$PODBOX_STORE` (lines 135-139). A deleted blob refuses `run`, marks the
`images` row, fails `verify`, and heals under `--pull always` (lines 143-154).
A DNS failure names the host once in plain words (line 159).

**Pinned inputs.** The pinned alpine 3.20 digest (line 38).

**Host assumptions.** `/bin/sh` from `pwd` (line 32). Needs `cargo`, `cc`,
the interposer objects, and `unshare -Urm` (line 49), each `exit 2`.

**Exit contract.** Declared at lines 28-29. `exit 2` for a missing tool, a
missing namespace, a failed bootstrap, a failed interpose, or a failed pull;
`exit 1` for a failed build, a missing binary, and a missing blob to delete.

**Timeouts.** 1200 on bootstrap and interpose, 1800 on the build, 600 on the
pull, 120 on every step.

**Cleanup.** `WORK` removed first (line 35). `/out` copy at line 170.

**Positive control.** Clause 1's `store_exec: yes` on a normal directory,
against clause 3's `store_exec: no` on the noexec one.

**Failure control.** Clause 4's deleted blob. It must fail and it must heal;
`blob-verify` at 125 then `blob-verify2` at 0 is the pair that proves the
failure was the blob and not the tool.

**TODO entries that own it.** T-1417 (`TODO/cli.md:1604`, `Prove:` names it);
T-1418 (`TODO/image.md:2255` area, `Prove:` at 2289); T-1419
(`TODO/image.md:2298` area, `Prove:` at 2336); T-1420 (`TODO/image.md:2355`
area, `Prove:` at 2377). T-1321 is named as extended by T-1419 at
`TODO/image.md:2345`.

**Result file.** `experiments/results/store-gates.txt`, 127 lines, dated
2026-09-30, podbox 0.1.0-beta.10. Verdict `STORE GATES HOLD`. All four
entries quote this file, and the quoted clauses match: clause 2's
`PATH=/usr/local/sbin:…` at result line 31, clause 3's refusal at line 55,
clause 4's refusal at line 62 and heal at line 99, clause 5's single host at
line 125.

**Verdict: RUST-TEST.** This is the clearest case in the group: every clause
is already unit-provable, and most already are unit-proved.

- Clause 1: covered in source. `exec_probe.rs` carries
  `a_writable_directory_probes_executable_and_litters_nothing`.
- Clause 2: covered in source. `images.rs:2139`
  `missing_blobs_lists_blobs_with_no_file` and the `Config` field tests at
  `images.rs:2130`.
- Clause 3: the refusal text is in `lifecycle.rs:1711` and
  `lifecycle.rs:1740`; `lifecycle.rs:2432`
  `store_exec_passes_where_a_created_file_executes` covers the positive arm.
- Clause 4: covered in source. `health.rs:210`
  `verify_blobs_names_a_deleted_blob_as_absent` and `health.rs:243`.
- Clause 5: the renderer is `crates/podbox-image/src/error.rs`.

**Conversion plan.**

- Assertions today, as a triple, per clause.
  1. Precondition: a store directory holding nothing. Action: `images`.
    Expected: exit 0, a header row and no data row, and no `missing from
    the` mark. Second: `doctor store_exec` on it. Expected: `store_exec:
    yes`, exit 0.
  2. Precondition: a record whose config blob is present. Action:
    `inspect --format '{{.Config.Env}}'` and `{{.Config.Cmd}}`, then
    `inspect` with no template. Expected: `PATH=` in the first, `/bin/sh` in
    the second, and a `Config` object in the document.
  3. Precondition: a store on a `noexec` mount. Action: `doctor
    store_exec`. Expected: `store_exec: no`, exit 1. Action: `run --pull
    never`. Expected: exit `EXIT_RUNTIME_ERROR` naming the directory and
    `$PODBOX_STORE`, before any byte is fetched.
  4. Precondition: a record whose one blob has been deleted. Action: `run
    --pull never`. Expected: exit 125 naming `--pull always`. Action:
    `images`. Expected: the record marked. Action: `verify`. Expected: exit
    125. Action: `run --pull always`. Expected: exit 0 and the payload's
    stdout. Action: `verify`. Expected: exit 0.
  5. Precondition: a registry host that does not resolve. Action: `pull`.
    Expected: the host named once, the URL not doubled, and no `Dns Failed`
    capitalisation.
- Level: clauses 1, 2, 4, 5 are **pure**. Clause 3 is **fault**: a `noexec`
  mount is a condition a test cannot arrange by writing files. `code.md`
  line 28.
- Target:
  - clauses 1, 2, 4 into `crates/podbox-image/src/health.rs` and
    `crates/podbox-cli/src/images.rs` `#[cfg(test)] mod tests`, where the
    helpers already exist (`health.rs:165` `scratch`, `health.rs:171`
    `plant`).
  - clause 5 into `crates/podbox-image/src/error.rs` `#[cfg(test)] mod tests`.
  - clause 3 into a new `crates/podbox-image/tests/noexec_store.rs`. It
    needs `unshare(CLONE_NEWUSER | CLONE_NEWNS)` and `mount -t tmpfs
    -o noexec`, which an in-crate unit test can do but which reads as a host
    capability test rather than a logic test. It is also the one clause where
    a mock would prove nothing: the whole point is that the kernel refuses.
- Fixtures: `health.rs`'s `scratch` and `plant` helpers are exactly the right
  ones and are already private to that module. Move them to
  `crates/podbox-image/src/testsupport.rs` behind `#[cfg(test)]` if clause 3
  needs them too, and say so, because one write path per shared action is
  `code.md` line 2.
- Plant: clause 3, delete the `ensure_store_exec` call at `lifecycle.rs`
  (the gate itself) and assert the test fails with the refusal message.
  Clause 4, delete the `verify_record_blobs` call at `run.rs:1536` and assert
  the test fails naming `--pull always`. Both are one-line defects that turn
  a refusal into a wrong success, and both are exactly what the entry's Done
  claims is fixed.
- Command: `cargo test -p podbox-image` and `cargo test -p podbox-cli
  images`. The noexec test runs on Linux only; give it
  `#[cfg(all(unix, target_os = "linux"))]`.
- `exit 2` becomes: no third state. Every fixture is built in-crate. The one
  gap is a host that refuses `unshare`, and `unshare -Urm true` at line 49 is
  the script's own probe; the Rust test reports the refusal in its assertion
  message so the state is named, not silently passed.
- Result file: `experiments/results/store-gates.txt` is superseded for all
  five clauses by the Rust tests, and is cited by four Done records. Move it
  to `docs/history/` only after those four `Prove:` lines are repointed.
  It is the last reading that proves the whole gate on one lane at once, and
  the entry `TODO/image.md` T-1419's Done leans on that.

**One contradiction to record.** Result line 83 reads `verify: 4 blobs
checked, 1 mismatched` for a 3.20 alpine whose `save` in
`experiments/results/ladder-rungs.txt:107` records `as 3 blobs`. The two runs
are the same digest on the same day. A `save` that counts manifest, config,
and layer as three and a `verify` that counts four are counting different
things, and neither document says which. That is a finding, not a
disagreement about behaviour.

## Group 5 - experiments/390-machine-ssh.sh

**What it measures.** Six clauses. Help paths print usage with 0 and refusals
name their reason with 125 (lines 96-138). A static dropbear builds from a
pinned commit (line 52). The socket bridge builds static (line 214). The
guest assembles and every archive is asserted per piece (lines 264-318). One
SSH command runs in the guest podbox boots, with exit 42 passed through
(lines 341-362). The wait is bounded and nothing remains (lines 376-400).

**Pinned inputs.** Alpine digest (line 49), kernel URL and sha256 (lines
50-51), dropbear commit `59870ad43153fe8d4f1c96f5d5752116c94f31ff` (line 52),
curl timeout 60 (line 54).

**Host assumptions.** `/work` as the working directory (line 26), because the
wrapper runs the file at `/in/job.sh`. Bootstrap of rust, cc, zig, openssh,
tools, qemu, vmtools (line 70). Then twelve binaries checked on PATH (line
71). `cargo build -p podbox-cli -p podbox-ssh --bins` under a 25-minute
timeout (line 79).

**Exit contract.** 0, 1, 2 as declared at line 23. `exit 2` at lines 26, 32,
37, 38, 70, 71, 84, 89, 252.

**Timeouts.** 25m on the build, 500 on each `machine ssh`, 60 on the help
paths and the deadline clause.

**Cleanup.** `trap 'cp "$OUT" /out/ …; rm -rf "$work"' EXIT HUP INT TERM` at
line 43, which hands partial evidence home on an early exit. Clause 6 asserts
no `podbox-machine-ssh-*` directory and no run process remains (lines 385-395).

**Positive control.** Clause 5's first boot asserts exact output
`guest-42\nPLACED\n0` (line 345).

**Failure control.** Clause 1's four refusals and clause 6's one-second
deadline, which must stop with 125 and not hang (line 380).

**TODO entries that own it.** T-1404 (`TODO/podssh.md:126`, `Prove:` names it
alongside `389` and `391`).

**Result file.** `experiments/results/machine-ssh.txt`, 228 lines, dated
2026-09-29, podbox 0.1.0-beta.9. Verdict `MACHINE-OK: serial SSH drives a
real guest command with its exit code`. Every clause green.

**Verdict: SPLIT.**

- Clauses 1, 6 (the refusal arms, the deadline, the argv and the argv-quoting
  tests): **RUST-TEST**. `crates/podbox-cli/src/machine/ssh.rs:614` already
  holds fifteen unit tests covering help, missing inputs, bad sizes, defaults,
  the qemu argv, the ssh argv, `sh_quote`, the directory guard, and the
  serial-wait deadline with its fast-failure and socket arms.
- Clauses 2, 3, 4, 5 (the real SSH session in a booted guest): **KEEP-SHELL**.
  The host dependencies are a qemu system emulator, a kernel download, a
  dropbear build from a pinned commit, and an ssh client. `docs/conventions/
  code.md` line 33: "A mock does not prove a live service or a deployment."
  There is no substitution that proves an SSH session to a real guest.

**Conversion plan, clauses 1 and 6.**

- Assertions today, as a triple.
  - Precondition: `machine ssh` with no flags. Action: parse. Expected:
    exit `EXIT_FLAG_ERROR` naming `--kernel`.
  - Precondition: `--kernel` naming a path that is not a file. Action: parse.
    Expected: exit `EXIT_RUNTIME_ERROR` naming the path.
  - Precondition: `--podbox-timeout 0`. Action: parse. Expected: exit
    `EXIT_FLAG_ERROR` naming a positive count.
  - Precondition: a serial socket with no listener behind the emulator
    process. Action: `wait_serial` with a one-second deadline. Expected:
    `EXIT_RUNTIME_ERROR`, and a fast failure where the emulator has already
    exited.
- Level: **pure** and **fault**. All four are function-on-data; the deadline
  arms are the fault kind because a live emulator that exits at startup is a
  condition a test arranges rather than waits for.
- Target: `crates/podbox-cli/src/machine/ssh.rs` `#[cfg(test)] mod tests`.
  Nine of these assertions already exist there at lines 622-700 and 900-1010.
  The work is to confirm each script clause has a named test and to add the
  three that do not.
- Fixtures: `DirGuard` at `ssh.rs:396` already arms and removes a per-run
  directory; the tests use it at line 793. Reuse it.
- Plant: delete the `Ok(0)` arm of the `/dev/null` open in the deadline path
  and assert the test fails with `before the deadline` rather than on a
  timeout. `run_phase_refusals_say_different_things` at line 971 already
  pins the three sentences apart, so the plant cannot pass on a merged
  message.
- Command: `cargo test -p podbox-cli machine::ssh`.
- `exit 2` becomes: no third state.
- Result file: kept whole. The saved run is the evidence for clauses 2 to 5,
  which do not move, and T-1404's `Prove:` still names the script.

**What stays and why, stated for the implementor.** The remaining driver
keeps the guest assembly and one boot. It must keep the assertions the entry
depends on and drop nothing: exact output bytes, exit 42, the bounded
deadline, the residue check, and the parity row check at line 396. Those five
are the entry's acceptance and the reason the script exists.

## Group 5 - experiments/400-kvm-cleanup.py

**What it measures.** That the shared emulator selector stops only its own
emulator. It builds a throwaway shell script from
`experiments/lib/kvm-owned.sh` plus two case arms (lines 27-29), asserts the
selector does not match the observer itself (line 33), spawns two processes
whose argv[0] is `qemu-system-x86_64` (lines 37-40), asserts only the owned
one is selected (line 43), stops it, and asserts the other survives and the
list is then empty (lines 48-50).

**Pinned inputs.** None. `KVM` is a `tempfile.TemporaryDirectory` (line 23).

**Host assumptions.** Linux. `sys.platform != 'linux'` returns 2 at line 21,
because "the process selector requires Linux ps". `experiments/lib/
kvm-owned.sh:4` runs `ps -eo pid,args` and matches on argv[0] and the owned
path.

**Exit contract.** 0 on success, 2 on a non-Linux host or a failed
`check=True`. An `AssertionError` raises and the process exits non-zero, which
is 1 by convention but is not the script's own code. Every child wait is
bounded by `timeout=5` or `timeout=20`.

**Cleanup.** `finally` at lines 52-56 kills and reaps both processes.
`TemporaryDirectory` removes the scratch.

**Positive control.** `assert processes[1].poll() is None` at line 49: the
unrelated process must survive. Without it a selector that stopped everything
would pass.

**Failure control.** `assert call('list') == []` at line 33, plus the
assertion at line 33's own line: the observer is excluded. Together they
separate "selects nothing" from "selects only mine".

**TODO entries that own it.** T-1350 (`TODO/gate.md:1775`, `Partial 2026-
09-30:` at line 1796: "Experiment 400 proves that the observer and another
process are excluded.").

**Result file.** `experiments/results/kvm-cleanup.txt`, 6 lines, dated
2026-09-30. `ok: observer excluded; owned process stopped; unrelated process
retained`, `verdict KVM-CLEANUP-OK`, `proof exit: 0`.

**Verdict: KEEP-SHELL.**

The subject is `experiments/lib/kvm-owned.sh`, 19 lines of `ps -eo pid,args`
and `awk`. Its matcher depends on the exact `ps` output format and on argv[0]
being the emulator name. The Python driver is the only thing that can arrange
two processes with a controlled argv[0] and assert one survives; rewriting it
in Rust would not remove the shell dependency, because the shell script is
what `experiments/lib/kvm-guest-base.sh:8` and `:202` call in the real KVM
path.

The host dependency, stated precisely: `ps -eo pid,args` on Linux, plus
`awk` matching `$2 ~ /(^|\/)qemu-system-x86_64$/`.

**What it is already.** It runs in the required gate:
`.github/workflows/gate.yml:208` and `scripts/dev.sh:227` both invoke
`python3 experiments/400-kvm-cleanup.py`, and
`experiments/results/repo-audit-linux.txt:69` records `verdict KVM-CLEANUP-OK`
inside `dev.sh check`. That is more than any other Group 5 script has.

**What is worth changing.** Two things, neither of which is a port.

1. The exit code. An `AssertionError` leaves 1 by accident rather than by
   contract, and `dev.sh` treats anything other than 0 and 2 as a failure.
   Catch it and return 1 explicitly, and let the 2 path stay the one the
   header declares.
2. The spawn shape at lines 37-40 is a fixture that lies: it passes
   `qemu-system-x86_64` as argv[0] to a Python interpreter. The comment at
   `experiments/results/kvm-cleanup.txt:3` says "Linux processes with
   controlled argv", which is true and is the point, but a reader will read it
   as an emulator. Say so in the source comment.

Neither needs a Rust crate. Leave the script.

## Group 5 - experiments/50-interpose-tier.sh

**What it measures.** Four walls against `VHSgunzo/pathmap`, somebody else's
LD_PRELOAD interposer, at commit `98b3d2a` (line 26). Path mapping without
`mount(2)`; chown to an unmapped gid; the ptrace tracer half; and
`memfd_create + fexecve`.

**Pinned inputs.** The upstream commit, pinned (line 26).

**Host assumptions.** `gcc` or `exit 2` (line 28). It clones the upstream
repository (line 37) and builds two targets with `make` (line 42). It then
enters the reconstruction through `experiments/20-enter-target.sh` (line 103),
which builds a container image and runs it privileged with `mount(2)` and
`pivot_root(2)`.

**Exit contract.** 0 ran, 1 a measurement failed to produce a verdict, 2 could
not run. The script's own final line is `exit "$rc"` where `rc` is
`20-enter-target.sh`'s code (line 114).

**Timeouts.** None. There is no `timeout` anywhere in the file. The `make`,
the `git clone`, and the container run are all unbounded.

**Cleanup.** None. `OUT` defaults to `experiments/.interpose` (line 21) and
`STAGE` to `experiments/.stage` (line 22). Neither is removed.

**Positive control.** None. Every block prints a verdict line; none asserts.

**Failure control.** None. Wall 2 at lines 74-84 is the closest: it
compares a bare `chown` against one under the interposer. That is a control,
and it does not test the subject's boundary.

**TODO entries that own it.** **None.** No `TODO/*.md` file names it. The
last owner disowned it in writing: `TODO/interpose.md:365` records that
T-0703's `Prove` "named `50-interpose-tier.sh` until 2026-09-18, which
measures pathmap, somebody else's object, and cannot verify this one: the
wrong subject with a green exit".

**Result file.** None. `experiments/results/interpose-tier.txt` does not
exist. The only mentions outside this script are
`references/Azathothas__container-research/tree/experiments/README.md:33`,
`references/Azathothas__container-research/tree/paper_final.md:1878`,
`references/Azathothas__container-research/tree/TOOL.md:1043`, and
`docs/history/experiments-README.md-before-2026-09-30.txt:61`. Three of the
four are read-only corpus; the fourth is superseded history.

**Verdict: DELETE.**

The evidence is four-fold and each part is quotable. The owning entry
disowned it by name. No result file exists. No entry references it. The
subject is another project's object, and `TODO/reference-map.md:25` records
`references/VHSgunzo__pathmap` as "vendor and patch", a decision the current
tree carries in `crates/podbox-interpose/`, not as "measure upstream in a
container".

Two independent reasons, because either alone would be weak. The premise is
superseded: T-0703's Done at `TODO/interpose.md:367` records that podbox's own
object interposes 89 names and `experiments/161-path-rewrite.sh` drives it.
And the script has no gate: no timeout on a `git clone`, a `make`, or a
privileged container run, and no cleanup of the two directories it creates
under `experiments/`.

Deletion order: retire the number under `experiments/README.md`, then delete
the script. The number is permanent per that file, so retirement is the
record, and `experiments/50` must not be reused. Nothing else needs changing,
because nothing references it.

## Group 5 - scripts/plant.sh

**What it measures.** Not a measurement. It is the harness that proves each
check in `scripts/check-todo.py` can fail. Forty-four plant cases and four
controls, each asserted by the defect's own message rather than by an exit
code (line 141).

**Pinned inputs.** One file list, named once at line 51, iterated by both the
backup and the restore. Four values read out of other files at run time and
never written into the harness: `TAKEN_EXP` (line 62), `CEILING_NUM`
(line 191), `INTERPOSE_CEILING_NUM` (line 202), `CC_WRAPPER` (line 317).

**Host assumptions.** `git` (line 47). Bash, not POSIX sh: `local` at line
124. The clean tree must have no uncommitted changes to any listed file, or
it exits 2 (line 74). `cargo` is not required; the `todo` job needs no
toolchain (the comment at line 329).

**Exit contract.** 0 every plant caught and every control quiet, 1 a plant
missed or a control fired, 2 could not run. Three `exit 2` paths: the gate
missing, no `git`, a dirty tree, no experiment to collide with, a missing
`CEILING_BYTES`, a missing `INTERPOSE_CEILING_BYTES`, a missing `CC_WRAPPER`,
and the gate already red (line 114).

**Timeouts.** None. Every `sed`, `git` and gate invocation is bounded by
nothing. This is the one finding in this group about a gate script: an
unbounded child is what `TODO/RULES.md` section 8 forbids, and `plant.sh`
runs it in CI.

**Cleanup.** `trap 'restore; rm -rf "$BACKUP"' EXIT INT TERM` at line 107.
`restore()` copies every listed file back from the backup, removes the staged
scratch directory, and unstages the duplicate script (lines 89-97).

**Positive control.** Four controls at lines 509-519, counted apart from the
plants (guard 4, line 525).

**Failure control.** The whole harness is the failure control, and its own
failure modes are guarded: guard 1 hashes the working state before and after
each mutation (lines 102-105), so a plant that lands nowhere reports MISS
rather than a silent green.

**TODO entries that own it.** T-1201 (`TODO/gate.md:16`, `Prove:` at 62 and
65); T-1202 (`TODO/gate.md:74`, `Prove:` at 125); T-1204 (`TODO/gate.md:230`,
`Prove:` at 250); T-1205 (`TODO/gate.md:290`, `Prove:` at 324); T-1207
(`TODO/gate.md:490`, `Prove:` names it); T-1210 (`TODO/gate.md:682`,
`Prove:` at 756); T-1325 (`TODO/gate.md:1201`, `Prove:` at 1256);
T-1345 (`TODO/gate.md:1640`, `Prove:` at 1665); T-1346 (`TODO/gate.md:1676`,
`Prove:` at 1692); T-1422 (`TODO/gate.md:1818`, `Prove:` at 1859); T-1209
(`TODO/deps.md`, `Prove:` at 618).

**Result file.** `experiments/results/plant-2026-09-30.txt`, 66 lines.
Baseline exit 0, 44 plants caught, 0 missed, 4 controls quiet, 0 fired. Its
verdict line reads "every check that was planted against went red with its own
message."

**Verdict: RUST-TOOL.**

It is a durable operator workflow, not a measurement: a gate runner the
repository's own CI invokes. It becomes a binary in a new
`crates/podbox-plant` crate.

**Conversion plan.**

- Assertions today, as a triple. Forty-four, one per check. In the shape the
  script uses:
  - Precondition: a clean checkout of this repository, with a clean index.
    Action: plant defect D in file F, run the gate, restore F from a copy.
    Expected: the gate exits non-zero and its output contains D's own
    message, and that message is absent from the clean-tree baseline.
  - Control: the same shape with a legitimate edit. Expected: exit 0.
- Level: **deployment**. `docs/conventions/code.md` line 29: "Use integration
  and deployment proof for the actual default path." The subject is a whole
  repository's gate run, not a function. A unit test would prove nothing and
  `code.md` line 34 says a skipped test proves nothing about its subject.
- Target: new crate `crates/podbox-plant`, binary `podbox-plant`, `src/
  main.rs` plus `src/case.rs` for one case and `src/tree.rs` for the backup,
  restore and hash. It is a `[[bin]]` target, not a library: nothing else
  calls it.
- CLI surface, matching what CI and the entries already type:
  ```
  podbox-plant                    run every case, print the same lines, exit 0/1/2
  podbox-plant --case NAME        run one case by name, for a single-plant loop
  podbox-plant --list             print the case names and their expected substrings
  podbox-plant --out PATH         also write the transcript (the result file)
  ```
  Keep the case names verbatim. `TODO/gate.md:1859` and `TODO/deps.md:618`
  quote them, and a name that moves breaks a citation the same way an
  experiment number does.
- Fixtures. None of `experiments/lib/*` applies. The four run-time values
  stay run-time reads of the repository, never literals: `TAKEN_EXP` from the
  `experiments/` listing, `CEILING_NUM` from `experiments/110-bloat-delta.sh`,
  `INTERPOSE_CEILING_BYTES` from `scripts/build-interpose.sh`, and the `CC_`
  wrapper from `.cargo/config.toml`. The reason is at lines 176-180: the gate
  reads `plant.sh` like any other file, so a literal bad citation in it is a
  permanent gate failure. A Rust binary the gate does not read is safer
  still, but the run-time read must stay, because check 18 reads the
  experiments directory, not the harness.
- Plant for the plant harness. The harness's own subject is check 15, "tracked
  experiment scratch". Plant it by staging a file under
  `experiments/.plantscratch` and asserting the gate goes red naming it; the
  current script does this at line 271. In the Rust binary this becomes a unit
  test on the restore path: run one case, then assert the working tree is
  byte-identical to the backup. The current script's guard 1 already implies
  it but never asserts it after the whole suite.
- Command: `cargo run -p podbox-plant --bin podbox-plant`. The gate step in
  `.github/workflows/gate.yml:56` changes from `./scripts/plant.sh` to
  `cargo run --release -p podbox-plant --bin podbox-plant`. `TODO/gate.md`
  T-1201, T-1202, T-1204, T-1205, T-1207, T-1210, T-1325, T-1345, T-1346 and
  T-1422 all name `./scripts/plant.sh` in a `Prove:` and all eleven must be
  repointed in the same change, per the "one fact, one home" rule at
  `docs/conventions/prose.md:35`.
- `exit 2` becomes `cargo`'s own. The dirty-tree guard at line 74 becomes an
  early return with a message naming the files, and `main` exits 2. The gate
  already red arm at line 114 becomes the same.
- Result file. `experiments/results/plant-2026-09-30.txt` is superseded by
  the binary's own transcript. Move it to `docs/history/` after the CI step
  changes. T-1422's `Done:` cites it for the 44/0/4/0 counts, and those
  counts move with every new case, so the file's own verdict line is the
  answer and the entry's number ages.
- **What does not change.** The four guards stay, and their reasons stay in
  the commit that introduced them: the mutation must land, the restore is
  from a copy and never `git checkout --`, one file list, and controls
  counted apart from plants. `docs/methodology/gate.md:44` requires the new
  check and its plant to land in the same change, so a new check in
  `check-todo.py` and a new case in `podbox-plant` arrive together.

## Group 5 findings

**F1. `experiments/50-interpose-tier.sh` has no owning entry and no result.**
`TODO/interpose.md:365` is the only TODO reference and it disowns the script
by name. No file at `experiments/results/interpose-tier.txt` exists. Four
other references, all in read-only corpus or superseded history. It also has
no timeout on a `git clone`, a `make`, or a privileged container run, and it
creates `experiments/.interpose` and `experiments/.stage` without removing
them.

**F2. `320` clause 3's saved reading is against a table that moved.**
`experiments/results/cli-contract.txt:24` names `restart` first among six
`None` verbs. `crates/podbox-cli/src/parity.rs:261` now carries
`status: Native` for `restart`, and the first six `None` verb rows are `top`,
`attach`, `pause`, `unpause`, `stats`, `diff`, 33 in all. The script reads
the verbs from the table at run time (line 215), so the script is right and
the saved file is stale.

**F3. `TODO/cli.md:63` quotes `rows | **220**` against a 267-row source
table.** The entry says the number "is the CURRENT count so the two cannot
drift apart", which is the claim the number falsifies. `TODO/cli.md:685`
separately records "160 rows, 199 driven" and line 707 "164 rows, 202
driven". Three readings in one entry and a fourth in the source. A count
`TODO/RULES.md` section 4 calls derived is written out here by hand.

**F4. `373` and `358` count the same image's blobs differently, on the same
day.** `experiments/results/store-gates.txt:83` reads
`verify: 4 blobs checked, 1 mismatched`;
`experiments/results/ladder-rungs.txt:107` reads `saved … as 3 blobs`. Both
runs are the pinned alpine 3.20 digest on 2026-09-30. Neither document says
which four and which three.

**F5. `365`'s result and `TODO/enter.md:759` name different rungs for the
same clause.** The result at line 15 records `probe selects supervise`;
the entry's `Prove:` says "probe below namespace" and the script's own
comment at line 2 says "the probe selects chroot". `supervise` is a valid
rung word in `crates/podbox-probe/src/select.rs:48`, and the script accepts
any non-`namespace` word, so all three are consistent with the code and
inconsistent with each other.

**F6. `390`'s own result contradicts its source on the interposer.**
`experiments/results/machine-ssh.txt:160-161` records two build warnings, "no
gnu interposer at … Run ./scripts/build-interpose.sh" and the same for musl,
while `TODO/interpose.md:249` records that `crates/podbox-cli/build.rs` COPIES
what the script left behind and that the objects are embedded. The saved run
predates the copy shape by nine days. The clause still passed, because
`machine ssh` does not load the interposer, so this is a stale reading rather
than a defect.

**F7. `scripts/plant.sh` bounds no child.** Every `sed`, `git` invocation and
gate run is unbounded, and the harness runs in `.github/workflows/gate.yml:56`.
`TODO/RULES.md` section 8: "Bound external commands, child waits, and network
operations." This is the one Group 5 script whose own gate rule it breaks.

**F8. Entries whose acceptance command is a script with no automated test.**
Every entry below names a script and nothing else, and every one of those
scripts needs a live host condition a `cargo test` cannot reach. They are not
defects; they are the work this conversion is for, and each names its
replacement above.

| entry | `Prove:` names | file:line |
| --- | --- | --- |
| T-1303 | `146-podvm-initramfs.sh` | `TODO/podvm.md:353` |
| T-1313 | `146-podvm-initramfs.sh` | `TODO/podvm.md:883` |
| T-1102 | `150-image-acquisition.sh` | `TODO/milestones.md:152` |
| T-0801 | `320-cli-contract.sh` | `TODO/cli.md:54` |
| T-0803 | `320-cli-contract.sh` | `TODO/cli.md:312` |
| T-1212 | four scripts including `150` and `320` | `TODO/gate.md:964` |
| T-0608 | `340-detached-stdio.sh 40` | `TODO/supervise.md:581` |
| T-1003 | `358-ladder-rungs.sh` | `TODO/packaging.md:135` |
| T-1339 | `365-namespace.sh` | `TODO/enter.md:758` |
| T-1417 | `373-store-gates.sh` | `TODO/cli.md:1604` |
| T-1418 | `373-store-gates.sh` | `TODO/image.md:2289` |
| T-1419 | `373-store-gates.sh` | `TODO/image.md:2336` |
| T-1420 | `373-store-gates.sh` | `TODO/image.md:2377` |
| T-1404 | `390-machine-ssh.sh` | `TODO/podssh.md:126` |

**F9. `320` uses a fourth exit state the contract does not have.** Line 329
returns 2 when `skipped=1`, which is set at line 311 when no docker daemon
answered. `experiments/README.md:11` defines 2 as "Required conditions were
absent", so the use is defensible, but a clause that could not be measured and
a clause that could not run share one code. The entry T-0803 anticipated this:
`TODO/cli.md:334` says a daemon-free machine "reports the refusal half as
unmeasured rather than as a pass". A Rust test makes the distinction free.

**F10. `150` clause 1 depends on a registry docker can reach and a tag that
can move.** Line 40 defaults to a `:latest` tag, and lines 148-166 re-pull
both once on a mismatch to separate a moved tag from a wrong digest. T-0206
(`TODO/image.md:430`) exists because Docker Hub's anonymous quota made this
script unrunnable on demand, and its `Approach` at line 452 is the loopback
fixture. The conversion should take that fixture rather than invent one.

## Group 5 - entries read

Read in full, not by grep line.

| entry | file | what it owns for Group 5 |
| --- | --- | --- |
| T-0801 | `TODO/cli.md:18` | `320` clauses 1 to 3; the parity table as data |
| T-0803 | `TODO/cli.md:280` | `320` clauses 4 and 5; the docker-name ruling |
| T-0808 | `TODO/cli.md:632` | `320` as the narrow driver T-0808 replaced |
| T-1330 | `TODO/cli.md:970` | bundled shorts, which `320` clause 2's `-i` arm depends on |
| T-1331 | `TODO/cli.md:1025` | `restart` moving to Native, which is finding F2 |
| T-1417 | `TODO/cli.md:1580` | `373` clause 2; `inspect Config` |
| T-1102 | `TODO/milestones.md:129` | `150` as M1's acceptance |
| T-1103 | `TODO/milestones.md:268` | `320` as M3's acceptance |
| T-0201 | `TODO/image.md:17` | `150` clause 4; HTTPS only |
| T-0202 | `TODO/image.md:74` | `150` clauses 2 and 3; digest parity |
| T-0206 | `TODO/image.md:430` | `150` on a tag and on Hub quota; the loopback fixture |
| T-1418 | `TODO/image.md` | `373` clause 3; the noexec store gate |
| T-1419 | `TODO/image.md` | `373` clause 4; missing-blob refusal and mark |
| T-1420 | `TODO/image.md` | `373` clause 5; transport errors |
| T-1302 | `TODO/podvm.md:212` | the emulator driver `146` deliberately does not drive |
| T-1303 | `TODO/podvm.md:314` | `146` clauses 1 to 5 |
| T-1304 | `TODO/podvm.md:383` | the exec protocol `147` owns, not `146` |
| T-1313 | `TODO/podvm.md:844` | `146` clause 6, the stalled-origin bound |
| T-0702 | `TODO/interpose.md` | `158`; the nested-build hazard and the copy shape |
| T-0703 | `TODO/interpose.md` | `50`, disowned at line 365 |
| T-1003 | `TODO/packaging.md:113` | `358`; all seven clauses |
| T-1339 | `TODO/enter.md:687` | `365`; the namespace rung's denied-host half |
| T-0505 | `TODO/enter.md` | `320` clause 4 as `exec`'s acceptance |
| T-0608 | `TODO/supervise.md:529` | `340`; the detached stdio handoff |
| T-1401 | `TODO/podssh.md:16` | the restricted-server sibling of `390` |
| T-1404 | `TODO/podssh.md:126` | `390`; the machine arm |
| T-1201 | `TODO/gate.md:16` | `plant.sh` cases 11 and 14 |
| T-1202 | `TODO/gate.md:74` | `plant.sh` as the harness |
| T-1204 | `TODO/gate.md:230` | `plant.sh` cases 17a to 17d |
| T-1205 | `TODO/gate.md:290` | `plant.sh` cases 18a and 18b |
| T-1207 | `TODO/gate.md` | `plant.sh` cases 23a to 25 |
| T-1210 | `TODO/gate.md` | `plant.sh` case 26 and the curated arm |
| T-1212 | `TODO/gate.md:934` | `150` and `320` as engine users |
| T-1325 | `TODO/gate.md:1201` | `plant.sh` cases 26a to 26e and 27 |
| T-1345 | `TODO/gate.md:1640` | `plant.sh` under the Windows wrapper |
| T-1346 | `TODO/gate.md:1676` | `plant.sh` finding its checkout from a job input |
| T-1350 | `TODO/gate.md:1775` | `400`; the owned-emulator selector |
| T-1422 | `TODO/gate.md:1818` | `plant.sh` case 29b and the `.bat` twin |

Thirty-seven entries read in full. `AGENTS.md`, `TODO/RULES.md`,
`docs/methodology/experiments.md`, `docs/methodology/authoring.md`,
`docs/conventions/code.md`, `docs/conventions/prose.md`,
`docs/methodology/gate.md` and `experiments/README.md` were read in full as
the required orientation.

## What I could not settle

- **`-`, for `373`'s blob count.** Four checked against three saved, same
  digest, same day. Whether `save` counts the manifest and `verify` counts the
  index, or one of them is wrong, needs `crates/podbox-image/src/layout.rs:49`
  and `health.rs:119` read together against a live store. Settling it: build
  one store from the pinned alpine and print both counts.
- **`-`, for the parity row count T-0801 claims to keep current.** 220 in the
  entry, 267 `Row {` occurrences in the source, 164 in the saved reading. I
  counted occurrences with `grep`, which does not distinguish a row from a row
  inside a comment. Settling it: `podbox system info --format '{{json
  .Parity}}' | jq 'length'` on a built binary.
- **`-`, for whether `experiments/146-podvm-initramfs.sh` still runs.** T-1303
  and T-1313 are `done`, the result file is dated 2026-09-21, and no later run
  exists. I was told not to run it, so I cannot say whether clause 5 still
  boots.
- **`-`, for the `stdout` channel of `320` clause 2's `-i` arm under a real
  image.** `todo/cli.md:707` records it as unreachable on one lane and says the
  cause is unestablished. Nothing in the tree settles it and no test drives it.