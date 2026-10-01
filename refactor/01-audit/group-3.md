# Group 3 audit

Scope: the thirteen scripts the assignment map assigns to group 3, 2938 lines.
Method: every file read whole. No script was run.

## Group 3 summary

| Script | Lines | Verdict |
| --- | --- | --- |
| `experiments/125-across-distributions.sh` | 253 | RUST-TOOL |
| `experiments/147-podvm-exec.sh` | 314 | KEEP-SHELL |
| `experiments/155-proc-absence.sh` | 400 | SPLIT |
| `experiments/156-closure-records.sh` | 127 | DELETE |
| `experiments/190-parallel-layers.sh` | 624 | RUST-TOOL |
| `experiments/20-enter-target.sh` | 199 | KEEP-SHELL |
| `experiments/325-parity-drive.sh` | 288 | RUST-TEST |
| `experiments/368-run-decay.sh` | 178 | RUST-TEST |
| `experiments/383-ssh-liveness.sh` | 142 | DELETE |
| `experiments/40-language-selection.sh` | 223 | DELETE |
| `experiments/401-emulator-streams.sh` | 45 | DELETE |
| `experiments/targetfs.sh` | 95 | KEEP-SHELL |
| `scripts/package-ssh.sh` | 50 | RUST-TOOL |

Counts: RUST-TEST 2, RUST-TOOL 3, KEEP-SHELL 3, SPLIT 1, DELETE 4.

### The three findings that matter

1. **Three scripts are the plant, not the measurement, and the plant already
   lives in `scripts/plant.sh`.** `401-emulator-streams.sh` mutates
   `crates/podbox-windows/src/lib.rs` with `sed` and re-runs one named test.
   `scripts/plant.sh` already does the same class of mutation and counts
   controls apart from plants (`scripts/plant.sh:4-31`, header guards 1 to 4).
   `156-closure-records.sh` measured a rule that `scripts/check-todo.py:770`
   check 22 now holds. `383-ssh-liveness.sh` runs named `cargo test` filters
   and adds no assertion of its own. Each of the three is a shell wrapper
   around a check that already has a Rust owner.

2. **`147-podvm-exec.sh` measures a protocol that is not in the source.**
   T-1304 rules a per-command 128-bit nonce in every marker
   (`TODO/podvm.md:415-418`). The shipped arm contradicts it in writing:
   `crates/podbox-cli/src/machine/ssh.rs:20-22` says "that is the guest's
   shape, not `TODO/podvm.md` T-1304's: T-1304 multiplexes many commands over
   one serial session with per-command nonces, while here the boot is the
   session." No `VMR-` string exists in `crates/`. The script measures a
   design that was not implemented; the entry T-1304 is marked `done`.

3. **`40-language-selection.sh` has no owning entry, and its subject is
   decided.** `TODO/RULES.md:115` records "Rust; static musl release by
   default" as a standing decision, sourced to captured TOOL.md section 3 and
   T-1002. T-1001 (`TODO/packaging.md:17-25`) already measured the static
   `PT_INTERP`-free property on the real artefact. The script compares three
   implementation languages to settle a choice the repository has made.

## Group 3 — experiments/125-across-distributions.sh

### What it measures

Whether a rootfs reads a supplied `/etc/passwd`, and what decides it. Each of
eleven pinned rows answers four questions: the row's libc, its
`/etc/nsswitch.conf` passwd line, whether a supplied `/etc/passwd` carrying
`podboxsupplied` is visible to a static glibc probe, and what that probe opens
after its own `execve`.

Pinned inputs: eleven manifest digests in `ROWS` (lines 57-69), resolved with
`docker manifest inspect` on 2026-09-08 per the line 55 comment. The probe
source is `experiments/src/across-probe.c`, nine lines, `getpwnam("podboxsupplied")`.
The supplied passwd is written inline at lines 110-113.

Host assumptions: a `cc` or a guest-built `PROBE_STATIC` (line 91-93, exit 2
without); an engine from `experiments/lib/engine.sh` (line 94-95, exit 2
without). `ENG_NETWORK` is not set, so rows run under the engine's default
network. The pinned digests are Docker Hub digests and `qualify` (line 166-172)
prefixes `docker.io/library/` or `docker.io/`.

Exit contract: 0 when every row that pulled produced a reading, 1 when a row
pulled and the harness could not run there, 2 when nothing ran (lines 241-252).
Timeouts: `eng_run 300` per row (line 209). Cleanup: `eng_clear` per row
(line 211); `TRANSCRIPTS` are kept by design.

Controls: the counters are recomputed from the transcripts rather than the
subshell (lines 227-233), which is a self-check on the instrument. There is no
positive control and no failure control. A run where all rows agree and a run
where all rows are unreachable are distinguished only by rule 3 (line 28), not
by a planted row.

Result files: `experiments/results/across-distributions.txt` and eleven
transcripts under `experiments/results/across/`.

### TODO entries that own it

- `TODO/gate.md` T-1211, line 891 and 906, names it as one of four scripts
  converted to `lib/engine.sh`. Line 921 records the run: "11 row(s) produced a
  reading. EXIT:0".
- `TODO/gate.md` T-1203, line 205 is its `Prove`. Line 207 records the run.
- `TODO/complete.md` T-0410, line 656: the script is the source of the six
  `passwd` shapes now driven through the editor by a unit test.
- `TODO/interpose.md:1064` names the matrix as the place to measure first.

Result file and entry agree. `experiments/results/across-distributions.txt`
reports `ran 11, no-pull 0, harness-failed 0` and T-1203's Done record says
"eleven rows ran, eleven transcripts". The `compat` row is recorded as
`probe-crashed-rc136` in both, and both call it a reading about static
portability rather than about nsswitch.

### Verdict: RUST-TOOL

The measurement is a durable operator workflow. It is a matrix runner over a
container engine, not logic. No Rust test can express "pull eleven pinned
images and run one probe in each". The parsing inside `body.sh` is the only
pure part, and it is already owned by a Rust unit test.

Crate: new `crates/podbox-matrix`. Binary: `podbox-matrix`. It is a
single-binary tool with no library half, so `[[bin]]` on a crate with no
`[lib]` is the shape; a `src/main.rs` plus a `src/rows.rs` holding the digest
table keeps the row data in one home.

CLI surface:

```
podbox-matrix across-nsswitch [--out DIR] [--transcripts DIR] [--cc PATH]
                             [--probe-static PATH] [--engine docker|podman]
                             [--rows REF|LOCAL|LIBC|DIGEST ...]
```

Exit: 0 every row that pulled produced a reading, 1 a row pulled and the
harness could not run there, 2 nothing ran or a required tool is absent. The
three-state contract is preserved as the tool's own exit codes, which is where
it belongs once the tool is a first-class artifact.

### Conversion plan

**Assertions the script makes today.**

1. Precondition: a pinned image pulled and a static probe staged. Action: run
   the probe with a supplied `/etc/passwd` naming `podboxsupplied`. Expected:
   the tool prints one row per image, and a row whose command ran is
   distinguishable from a row that could not be pulled.
2. Precondition: every row unreachable. Action: report. Expected: exit 2, not
   0. "Every row unreachable reads exactly like every row agreeing" (line 29).
3. Precondition: a row whose declared libc and measured libc disagree. Action:
   report. Expected: a `declared X, measured Y` line, and the run does not
   fail on it, because "that is a finding about the pin, not about the
   subject" (line 222).

**Rust level.** Integration, not unit. The three verification kinds in
`docs/conventions/code.md:27-29` apply: this is integration proof, because the
subject is a real container engine with real images, and a mock would not prove
a live service. The `exit 2` arm is the third state the kinds do not cover, and
the tool keeps it as a process exit code.

**Target.** `crates/podbox-matrix/src/main.rs`, module
`crates/podbox-matrix/src/rows.rs`. The digest table moves from `ROWS`
verbatim, with the line 55 comment carried as the module doc.

**Fixtures and fakes.** The probe body is a script the tool writes to a
scratch directory; keep the current text, which is the measurement. The probe
binary: `experiments/src/across-probe.c` today. The tool compiles it with `cc`
when present and copies `$PROBE_STATIC` when not, matching lines 102-108. The
engine: `experiments/lib/engine.sh` today. The tool reads the engine directly
by running `docker` or `podman`; the four engine calls in use are `pull`,
`mount` of three paths, `run`, and `rm`. Reuse `experiments/lib/engine.sh` as
the reference for what each call must bound and pin, and port the bound, not
the shell.

**Failure control.** Plant a run where every row's pull is refused: point
`--rows` at a digest that does not exist and assert the tool exits 2 rather than
0. That is the plant the shell script has no test for. It becomes a case in
`scripts/plant.sh` in the same change, per `docs/methodology/gate.md`.

**Proof command.**

```
cargo run -p podbox-matrix --bin podbox-matrix -- across-nsswitch
```

Linux only, with an engine on `PATH`. A Windows run uses
`sh scripts/windows/run-in-base.sh` with `PODBOX_ARTIFACTS` set, which is the
lane wrapper the current header already names.

**The `exit 2` path.** It becomes the binary's exit code 2, kept distinct from
`cargo test`'s model because the tool is not a test. Nothing is lost: the
report is written to `experiments/results/across-distributions.txt` on every
arm, as line 42 of `experiments/results/across-distributions.txt` shows for the
last run.

**The saved result.** Kept. `experiments/results/across-distributions.txt` and
`experiments/results/across/*.out` remain, and each transcript keeps its
conditions. The tool writes the same files, so T-1203's and T-1211's `Prove`
records keep pointing at live evidence.

## Group 3 — experiments/147-podvm-exec.sh

### What it measures

Whether one command crosses the serial line and returns its own status, while a
wrong status, a deadline and a forged marker each report distinctly. Six
clauses: conditions, assembly, readiness, zero status, non-zero status, a
marker-shaped line with the wrong nonce, and an overlong command reporting a
deadline.

Pinned inputs: `ALPINE_REF` pinned by digest (line 51), `KURL` with
`KERNEL_SHA256` checked after download (lines 52-53, 124), deadlines from
`PODBOX_147_BOOT` 120 s, `PODBOX_147_CMD` 20 s, `PODBOX_147_CURL_TIMEOUT` 60 s.

Host assumptions: Linux, native or in a job container. `qemu-system-x86_64`,
`cpio`, `python3`, `curl`, `sha256sum`, `timeout`, `mkfifo`, `stdbuf` all
required on `PATH` (line 68-70, exit 2). `experiments/lib/podvm-guest.sh` is
sourced (line 73).

Exit contract: 0 every clause green, 1 a clause failed, 2 a tool, the binary
or an input could not run. Timeouts: 600 s on qemu (line 188), 120 s boot
deadline, 20 s per command, 60 s curl. Cleanup: `cleanup()` kills qemu and
the reader, closes fds 7 and 8, and `rm -rf "$WORK"` (lines 136-154).
`keep_logs` copies the guest and qemu logs beside the result on failure and the
pass path removes them (lines 146-153, 303).

Controls: the readiness handshake is a round trip retried until the guest
answers, not a sleep (lines 205-222). The deadline arm is a failure control
for the wait. Clause 5 is a failure control for the marker matcher. There is no
mutation of podbox source, so nothing here proves the shipped code; it proves
the spec and the driver.

Result file: `experiments/results/podvm-exec.txt`.

### TODO entries that own it

- `TODO/podvm.md` T-1304, line 429 is its `Prove`, and lines 431-435 record the
  run.
- `TODO/podvm.md` T-1303, line 437-439: the assembly moved to
  `experiments/lib/podvm-guest.sh`, and 146 was re-driven.
- `TODO/podvm.md:892-896` cites the run in a later entry.

Result file and entry agree on all four outcomes: `true` 0, `exit 3` 3, forged
marker ignored with 7 reported, `sleep 30` past 8 s reporting DEADLINE.

### Verdict: KEEP-SHELL

The host dependencies are `qemu-system-x86_64`, a pinned Alpine kernel
fetched over the network, and `mkfifo` plus `stdbuf -o0` for the CRLF
stripping the driver depends on. `crates/podbox-cli/src/machine/ssh.rs` boots
a guest in Rust, but it opens a unix serial socket and speaks SSH over it, not
a fifo pair, and it does not carry the marker protocol. The fifo pair, the
`O_RDWR` hold before qemu starts (line 177-181) and the unbuffered `tr` are
host-tool facts no crate replaces.

This is the strict reading of KEEP-SHELL: the script drives `qemu`, a pinned
kernel download, and named POSIX utilities for a protocol the source does not
implement.

### What must happen beyond the verdict

The script is not dead. Its owner is wrong. T-1304's `Prove` names it, the
entry is `done`, and the shipped arm rejects the protocol it measured. That
contradiction is in `## Group 3 findings`.

The nonce marker logic itself is pure and is where a Rust test belongs, if the
protocol is ever implemented. A future implementation exposes a line matcher
over guest stdout. Precondition: a line stream carrying `VMR-END-<nonce> <n>`.
Action: match. Expected: the first exact line-anchored match with the current
nonce returns its status, a marker with a different nonce returns nothing, and
`DEADLINE` is a separate value from any status. Level: unit, because the inputs
are deterministic strings and the failure is a pure matching defect. Target:
`crates/podbox-cli/src/machine/ssh.rs`, a `#[cfg(test)] mod tests`. The six
forgery shapes the current record names (`TODO/podvm.md:452-453`) become six
table cases. No such function exists today, so this is contingent on the
protocol landing.

## Group 3 — experiments/155-proc-absence.sh

### What it measures

Five things, and they are not one measurement:

1. The five probe legs `readdir(/)`, `open(/bin, O_RDONLY) by name`,
   `creat(/, O_CREAT|O_EXCL)`, `stat(/dev/ptmx)`, `open(/dev/ptmx, O_RDWR)`
   run and print their own errno.
2. A denied arm staged by `setpriv` where the host allows it.
3. Clause 4, six arms of the `/proc` emulation: process substitution succeeds,
   a live-interface read still fails naming the missing procfs, `/proc/self/exe`
   answers the resolved guest path, `/proc/mounts` serves the generated
   fixture, a pipe descriptor readlinks as `pipe:[ino]`, and `/dev/stdin`
   opens descriptor 0.
4. Clause 5, a `cp -r` of a traversed-but-unlistable directory as `nobody`
   exits 125 naming the listing and `EACCES`.

Pinned inputs: the debian digest at line 71 and again at line 174, the same
`public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe…`.

Host assumptions: `jq` required (line 48, exit 2). An engine is required only on
a non-Linux lane; natively none is needed (lines 60-67). `setpriv` stages the
denied arm where present and its absence is a note, not a failure (line 131-135).

Exit contract: 0 every clause held, 1 one did not, 2 a leg could not run. The
`$WORK/worst` marker is written by `leg_not_run` (line 30-33) and drives the
exit 2 at line 398.

Timeouts: 120 s per probe run, 300 s per pull and per `run`, 600 s for the
driver row, 120 s for the `eng_pb` shim. Cleanup: `eng_cleanup` and
`rm -rf "$WORK"` on exit. Clause 5 removes `/tmp/pb155-nolist` in both arms.

Controls: the denied arm in clause 3 is a positive control for the probe
itself, showing `creat` refused with `EACCES` beside an ok listing. Clause 5
proves its fixture bites before asserting the refusal, by constructing a
mode-711 directory that cannot be listed.

Result file: `experiments/results/proc-absence.txt`.

### TODO entries that own it

- `TODO/complete.md` T-0413, line 990 is the `Prove` for the procfs banner.
  Lines 1001 and 1068 name clauses 4 and 4a-4f. Lines 1072-1092 record the run.
- `TODO/complete.md` T-0414, line 1140 is the `Prove` for the root-listing and
  ptmx legs. Lines 1148-1152 and 1193-1200 record them, with clause 5 named at
  line 1194.
- `TODO/complete.md:1164` cites clause `30-probe-chroot` from
  `experiments/results/triage-353.txt` for the all-ok reading.

Result file and entries agree clause for clause. `experiments/results/proc-absence.txt`
lines 24-49 carry 4, 4b, 4c, 4d, 4e, 4f exactly as T-0413's record lists them,
and line 51-53 carries clause 5 exactly as T-0414's record lists it, quoting
`podbox cp: cannot list /tmp/pb155-nolist: Permission denied (os error 13)`.

One disagreement. T-0414's `Prove` at line 1140 says the script "exits 2 rather
than 1 where a leg could not run". `experiments/results/proc-absence.txt` has
no could-not-run line, because every leg ran on the lane. That is the entry's
own three-state design working, not a contradiction.

### Verdict: SPLIT

The five parts have four different owners.

| Piece | Goes to | Why |
| --- | --- | --- |
| Clauses 1 and 2, the five probe legs and the ptmx pair | already Rust: `crates/podbox-probe/src/probes.rs` Census set | T-0414 records "a structural unit test pins their names, order, group and namespace flags" (`TODO/complete.md:1145-1147`) |
| Clause 3, the denied arm | DELETE from the script | it needs `setpriv` and stages a privilege the lane often lacks, and the legs above already ran |
| Clause 4a-4f, the procfs emulation | RUST-TEST | already covered by the interpose suite; the entry records 37 passed |
| Clause 5, the denied listing | RUST-TEST | the `cp` arm is pure error-message construction |

## Group 3 — experiments/156-closure-records.sh

### What it measures

Whether every closed `TODO/*.md` entry carries a recorded run under its `Prove`
field, and in which of the two shapes. An `awk` program (lines 64-116) walks
each entry, tracks `Status:` and `Prove:`, and counts text after `Prove` that
is neither a separator, a blank line, nor an indented continuation.

Pinned inputs: the `TODO/*.md` files in this checkout, minus
`INDEX|PROGRESS|RULES|RESUME|reference-map` (line 62). Tools: `awk` and `ls`.

Host assumptions: a `TODO` directory and `awk` (lines 43-50, exit 2). Nothing
else. No engine, no container, no build.

Exit contract: 0 every closed entry carries a record, 1 at least one does not,
2 could not run. An unknown option exits 2 (line 39). `--help` exits 0 (line 35).

Timeouts: none. Cleanup: none; the script writes no scratch.

Controls: none. It is a survey, not a measurement, and its own header says so
in the line 20 note.

Result files: none. It prints to stdout only. That is itself a finding; the
repository's own rule in `TODO/RULES.md:60` says results are saved under
`experiments/results/`, and this script never wrote one.

### TODO entries that own it

- `TODO/gate.md` T-1208, line 562: "Premise: **Measured by
  `experiments/156-closure-records.sh`, which is the instrument and not a
  reader.**" Line 567 gives the reading: "131 entries, 85 closed, 85 carry a
  record of the run and 0 do not".

Result file: none, so there is nothing to compare. The entry's numbers cannot
be checked against an artefact, which is the second half of the reason the
rule needed a check.

### Verdict: DELETE

The measurement is superseded by a check that exists and has a plant. T-1208
lines 619-622: "**Done 2026-09-21.** Check 22 in `scripts/check-todo.py` (every
`done` entry opens its record with `**Done` on the first unindented line after
`Prove`), its plant case in `scripts/plant.sh`".

`scripts/check-todo.py:770` is `def check_closure_records(entries)`, called at
line 1672, counting into `seen["closure_records"]` against the
`"closure_records": 0` initialiser at line 219. Its docstring names the
authority: "TODO/gate.md T-1208 is the ruling; RULES.md section 5 is the shape
it states."

T-1208 also strengthens what 156 measured. 156 accepted either shape and
counted them apart (line 104-105, `bold` and `prose`). T-1208's Decision
(line 599-602) picked one shape and the check holds only that one. The script
therefore asserts less than the gate now holds, and running it would report
the superseded rule.

The delete carries two record edits, both in `TODO/gate.md` T-1208: the
`Premise` at line 562 names the script as the instrument, and the entry has no
other citation of it. Under `docs/conventions/prose.md:41-45`, correcting live
text in place means the `Premise` sentence is reworded to name
`scripts/check-todo.py:770` and the 2026-09-12 reading is kept as the
historical premise with its commit.

## Group 3 — experiments/190-parallel-layers.sh

### What it measures

Whether a bounded fetch pool beats a sequential loop on a multi-layer image, and
what the pool buys on a latency-bound link. Four shapes: loopback sequential,
loopback pooled, latency sequential, latency pooled, two runs each, cold store
each leg. Plus five failure clauses: a poisoned layer makes the pull exit 125
naming the digest mismatch, leaves zero `*.partial` files, keeps the transcript
in manifest order, and writes no record.

Pinned inputs: zot `v2.1.21` with byte count and sha256, plus the release
checksums line (lines 43-48, verified at 204-226). The sequential shape is
pull.rs at commit `a0953f12fc70b43cd74a94d01d3aa201428330d9` (line 60). The
image is seeded at run time: 8 random 6291456-byte layers, `multi:layers`. The
delay is `experiments/delay-proxy.c` at 2000 ms, built in the lane (lines 172-191).

Host assumptions: an engine from `lib/engine.sh` (line 74, exit 2). The
non-native lane builds two podbox binaries and the proxy through
`sh scripts/windows/run-in-base.sh` (lines 148, 183). The driver image builds
from `experiments/180-driver.Dockerfile` (line 327).

Exit contract: 0 every clause held, 1 one did not, 2 could not run. The
verdict requires a `driver: ` line, because "an empty driver script exits 0
having done nothing" (lines 602-607).

Timeouts: 600 s on the zot fetch and the driver build, 900 s on the driver run.
Cleanup: `eng_cleanup` and `rm -rf "$WORK"` on exit.

Controls: the isolation clauses `iso-1` and `iso-2` assert no default route and
no name resolution from inside the driver (lines 355-364), so the timing cannot
be attributed to the network. `cfg-1`, `cfg-2` verify the zot configs.
`seed-1` fetches the manifest by tag and compares the digest. `fail-0` asserts
the poison actually changed the blob. `lat-0` asserts the proxy forwards.
`order-1` asserts the pooled transcript stays in manifest order. This is the
best-controlled script in the group.

Result file: `experiments/results/parallel-layers.txt`.

### TODO entries that own it

- `TODO/image.md` T-0207, line 605 is the `Prove`. Lines 624-654 record the
  first run, and 671-706 the latency shape.
- `TODO/gate.md:1353` names it in a check list.

Result file and entry agree. `experiments/results/parallel-layers.txt` line 20
records seed `sha256:b9530074e42685afec5afbdbf4b642d4dd74f345c1670df92ced5a8c3c4ca3b5`,
and the entry at line 685 abbreviates it as `sha256:b9530074…`. The four
timing pairs in the result (lines 36-44, 50-57) give the means the entry
tabulates at 689-692: loopback 3.83 s against 1.51 s and latency 24.62 s
against 11.55 s. The entry's `2.53x` and `2.13x` ratios are those means, and
line 705 records the earlier `1.78x` as a prior run.

### Verdict: RUST-TOOL

This is a measurement harness over a pinned registry server, a delay proxy, two
lane-built binaries, and a container driver. Nothing about it is product logic.
The product-side properties it asserts are already Rust unit tests: the entry
records at line 647 that `cargo test -p podbox-image` had 117 passed with each
guard seen red first, and the three guards it names are the bound, the order
merge, and the cancel cleanup. What the script uniquely measures is a wall-time
ratio against a shaped link, which is a fixture concern.

Crate: new `crates/podbox-matrix` again, one binary per experiment family, or a
second binary in the same crate. The crate's name should describe the job, not
the experiment number, because the numbers are permanent and the jobs are not.
Binary: `podbox-parallel-layers`. CLI surface:

```
podbox-parallel-layers [--bin-new PATH] [--bin-seq PATH] [--seq-commit SHA]
                       [--layers N] [--layer-bytes N] [--proxy-delay-ms N]
                       [--runs N] [--out FILE]
```

### Conversion plan

**Assertions the script makes today.**

1. Precondition: the same seeded eight-layer image served by pinned zot, all
   outbound network blocked, cold store each leg. Action: pull with the pooled
   binary and with the sequential binary. Expected: both exit 0 and inspect to
   the seeded manifest digest, and the pooled mean is below the sequential
   mean.
2. Precondition: the same image over a 2000 ms delay proxy in plain HTTP.
   Action: pull both shapes. Expected: both exit 0, same digest, and the ratio
   is recorded beside the loopback one.
3. Precondition: layer index 3 poisoned so its digest no longer matches.
   Action: pull with the pooled binary. Expected: exit 125, the log names
   `digest mismatch`, zero `*.partial` files remain, the transcript stays in
   manifest order, and no record is written.

**Rust level.** Integration, and the shape the kinds call deployment proof:
the subject is a real registry on loopback with real timing, and no mock
reaches it. The three-state exit contract is preserved in the binary.

**Target.** `crates/podbox-matrix/src/bin/podbox-parallel-layers.rs`, with
`src/bin/zot.rs` for the seed and the config. The seed function at lines 235-268
is pure apart from the filesystem: eight random blobs, a config, a manifest, an
index and a layout file. It becomes a Rust function returning the manifest
digest, which removes the shell's unquoted JSON assembly.

**Fixtures and fakes.** zot: downloaded and verified as today, lines 204-226.
The delay proxy: `experiments/delay-proxy.c` stays as is; the tool compiles it
through the same lane job. The driver image: `experiments/180-driver.Dockerfile`
stays. The sequential binary: the tool shells to `git show` for the pinned
commit exactly as the job does at line 130, and keeps both refusal guards, since
a stale pin is the failure the entry's own Done record warns about.

**Failure control.** The tool refuses when the staged `pull.rs` carries
`FETCH_WORKERS` in the sequential shape, and when the worktree carries none in
the new shape. Those refusals are guards, and per `docs/methodology/gate.md`
each needs a plant proving it fires. The plant: stage a `pull.rs` with the
constant present in the sequential job and assert the tool exits 2 naming the
stale pin.

**Proof command.**

```
cargo run -p podbox-matrix --bin podbox-parallel-layers
```

Linux, with an engine on `PATH`. On a non-Linux host the tool calls
`sh scripts/windows/run-in-base.sh` for the two binary builds and the proxy
build, matching lines 148 and 183.

**The `exit 2` path.** The binary's exit code 2, for a missing engine, an
unreachable or wrong-digest zot, a failed lane build, and each shape refusal.
The report is written first in every arm, matching current behaviour where
`cp "$WORK/report" "$OUT"` runs at line 618 before the exit.

**The saved result.** Kept, and rewritten in place by the tool. The entry at
`TODO/image.md:689-692` tabulates numbers from it, so keeping the file keeps
those citations live. The entry's own note at line 705 that the loopback ratio
moves run to run is why the tool must print its conditions block on every run.

## Group 3 — experiments/20-enter-target.sh

### What it measures

Whether a container can supply the target runtime's kernel-visible shape, and
which parts the host refuses. It is an entry point, not an assertion script: it
stages a harness, enters the reconstruction, and hands the payload through.

Pinned inputs: `TARGET_IMAGE`, default `container-research/target:1` (line 28),
resolved to an image id by `eng_image_inspect` (line 76-77). The harness
sources live in `HARNESS_SRC`, default
`$REPO/references/Azathothas__container-research/tree/verification` (line 93),
and the script refuses with a named message where `confine` or `probe` is
missing (lines 95-101).

Host assumptions: an engine (line 53-54, exit 2); `go` for the harness build
(line 94, exit 2); `gcc` for the C probe, native lane only (line 113).
`eng_privrun` is required, because `mount(2)` and `pivot_root(2)` build the
topology.

Exit contract: 0 the payload ran, its own code otherwise, 2 could not run. The
payload's own exit code is the script's exit code (lines 183-198).

Timeouts: 3600 s on the privileged run. Cleanup: `eng_cleanup` and
`rm -rf "$WORK"` on exit.

Controls: the harness is built for the image's architecture, not the host's,
because "on a non-native lane host go targets windows and the build dies on
Linux-only syscalls" (lines 104-107). The `--stage` destination is removed
first, because a nested `cp -a` produced `dest/store/store` and made a clause
pass for the wrong reason, measured 2026-09-08 (lines 152-164).

Result files: none of its own. It writes a conditions block to stderr
(lines 189-194). Its transcript is whatever the payload printed, which
`130-probe-parity.sh` captures as the rung.

### TODO entries that own it

- `TODO/gate.md` T-1213, lines 1111-1113 name it as one of the three scripts
  that convert together. Line 1127 is its half of the `Prove`. Line 1142
  records: "`20` exits 0 natively (N+F+M, `/bin/id` answers uid 0)".
- `TODO/milestones.md` T-1108, line 533 is the `Prove` of the packaging
  milestone: `readelf -l … | grep -c INTERP | grep -qx 0 &&
  ./experiments/20-enter-target.sh --stage … -- /workspace/podbox version`.
  Lines 535-539 record the run.
- `TODO/milestones.md` T-1101, line 78 names it in a `Prove`.
- `TODO/probe.md:418`, `:657`, `:739` and `TODO/enter.md:222` cite it as the
  place a probe run selects `chroot`.

The entries agree with the script: the script sources `lib/engine.sh`
(line 48), which is exactly what T-1213's Done record claims.

### Verdict: KEEP-SHELL

Three host dependencies no crate replaces. It needs a privileged container to
call `eng_privrun` for `mount(2)` and `pivot_root(2)`. It builds Go binaries
from a read-only reference corpus, with a cross-target requirement that is a
property of the Go toolchain, not of podbox. And it depends on
`experiments/10-build-target-image.sh` and `experiments/130-probe-parity.sh`
in a fixed three-way relationship that T-1213's Decision (line 1125) calls
irreducible: "Converting `130` without `20` tests nothing, and converting `20`
without `10` builds nothing."

Its result is a payload transcript, not a verdict. There is nothing to turn
into a Rust assertion.

## Group 3 — experiments/325-parity-drive.sh

### What it measures

Whether the shipped binary agrees with every row of the parity table it
publishes. The table is read out of the binary itself through
`podbox system info --format '{{json .Parity}}'` (line 74), so a row the binary
does not publish is a row the driver cannot drive.

Pinned inputs: the table, validated at line 77-81 with `jq -e 'length >= 60
and all(.status | IN("Native","Degraded","Stub","None"))'`. The store is a
fresh `PODBOX_STORE` (line 61). Exit codes come from
`scripts/common/exit-codes.sh` (line 58).

Host assumptions: `$BIN` executable (line 50-54, exit 2), `jq` (line 55, exit 2),
Linux native or in a job container.

Exit contract: 0 every row driven or reported unreachable, 1 a row disagreed,
2 the binary, jq or the table could not run.

Timeouts: 60 s per ordinary probe, 120 s for `run` and `exec` arms, 300 s for
the pull and extract in clause 3. Cleanup: `rm -rf "$WORK"` on exit.

Controls: three. The table itself is validated before use. A `None` row must
refuse with the flag-error code and its own note, read back out of the table by
`jq` (lines 152-162), so the note cannot be invented by the driver. A `Native`
row must reach a parser: "has no arm for it" is a mismatch (line 183), and
`parity::no_arm` panics under `cfg(test)` so it is visible
(`crates/podbox-cli/src/parity.rs:658-672`). And clause 3 reports a `Stub` row it
could not reach as `unreachable here`, never as passed (line 48).

Result file: `experiments/results/parity-drive.txt`.

### TODO entries that own it

- `TODO/cli.md` T-0808, line 680 is the `Prove`. Lines 684-698 record the first
  green run and note that the first drive found 4 mismatches, all in the
  driver. Lines 700-713 record the re-drive at 164 rows.
- `TODO/image.md:847` records a third drive at 164 rows, 202 driven.

Result file and entry agree. `experiments/results/parity-drive.txt` header says
`the table is data: 164 rows` and the counts line says `164 rows, 202 driven, 0
mismatches, 2 unreachable here`, which is T-0808's re-drive record verbatim.
The two unreachable rows are `run -i` and `exec -i`, named in both.

One disagreement, and it is a live one. The binary's table has 265 rows today,
counted from `crates/podbox-cli/src/parity.rs:232-537`. The saved result and
every entry record 164. No `Prove` in the tree re-drives the script. The
script's own `jq` gate is `length >= 60` (line 77), so 265 would pass it, and
the drive would report 265 rows and its own count. The 164 is historical on the
revision that produced it, which `docs/methodology/experiments.md:30` allows
("A historical result remains historical after a new build") but which no
current entry has re-established.

### Verdict: RUST-TEST

The subject is `parity::admit`, `parity::admit_all`, `parity::flag`,
`parity::rows_of` and `parity::no_arm` in `crates/podbox-cli/src/parity.rs`.
All are pure functions over `TABLE` and an argument list. The whole of clause 1
and clause 2 is a table walk calling `admit`, and clause 2's "no row in the
parity table" refusal is `admit`'s `Option::None` arm at
`crates/podbox-cli/src/parity.rs:599-621`.

The three clauses split by level:

- Clause 1's `None` refusal and clause 2's unlisted-flag refusal: unit. The
  inputs are a verb, a flag and a usage string. Expected: `Err(EXIT_FLAG_ERROR)`
  and a stderr line containing the row's own note. This is the shape
  `docs/conventions/code.md:27` names as a pure test.
- Clause 1's "reaches a parser with an arm for it" for the non-SSH verbs:
  cannot be unit-tested, because the arms are the parsers' own `match` arms
  across eleven files. This is what makes the drive a cross-process
  measurement. It is also already structurally closed: T-0808's Decision
  (line 672-679) says the pre-pass makes "an arm for a flag with no row
  unreachable and a row with no arm ends at `parity::no_arm`". The one
  remaining gap is the SSH verbs, which parse literally and are bound by
  `ssh_flag_rows_match_their_parsers_in_both_directions`
  (`crates/podbox-cli/src/parity.rs:959-991`).
- Clause 3's `Stub` rows that need a running payload: stays a drive.

## Group 3 — 325-parity-drive.sh conversion plan

**Assertions the script makes today.**

1. Precondition: a row with `status: None` for verb V and flag F. Action:
   invoke V with F. Expected: exit `PODBOX_EXIT_FLAG_ERROR` and stderr carrying
   that row's own `note` text.
2. Precondition: a flag no row names, for any verb path P. Action: invoke P
   with `--no-such-flag`. Expected: exit `PODBOX_EXIT_FLAG_ERROR`, stderr
   contains `no row in the parity table` and the literal `podbox P:` the caller
   typed.
3. Precondition: a row with `status: Native`, `Degraded` or `Stub`. Action:
   invoke the verb with the flag. Expected: no refusal, and not
   `has no arm for it`.
4. Precondition: a `Stub` row needing a running payload and no image pulled.
   Action: invoke. Expected: reported as unreachable, never as driven.

**Rust level.** Assertions 1 and 2 are unit, in `#[cfg(test)] mod tests` in
`crates/podbox-cli/src/parity.rs`. That is the pure kind: deterministic
inputs, and the defect is a wrong refusal or a wrong note. Assertion 3 is
already structurally enforced by the pre-pass and needs one table-walk test that
asserts `admit` returns `Ok` for every non-`None` row, which is pure and is not
in the tree today. Assertion 4 cannot be a test and stays a drive.

**Target file and module.** `crates/podbox-cli/src/parity.rs`, the existing
`#[cfg(test)] mod tests` at lines 706-707. New test names:
`every_none_row_refuses_with_its_own_note_and_the_flag_error_code`,
`an_unlisted_flag_refuses_naming_the_path_the_caller_typed`,
`every_non_none_row_is_admitted`, and
`group_subverbs_refuse_naming_the_subverb_not_the_group`.

**Fixtures.** The table itself is the fixture, read from `TABLE` in the same
crate, so there is nothing to add. The exit code comes from
`podbox_image::error::EXIT_FLAG_ERROR`, already imported in the test
(`crates/podbox-cli/src/parity.rs:1001`). Capturing stderr needs no new crate:
the existing tests in this module already call `admit_all` for its return value,
so the new tests should call `admit` and assert on the return, and use one
`std::process::Command` on the test binary only for the note text if the
message must be observed rather than inferred. Prefer the return value; assert
the note text by refactoring the message construction into a `pub(crate) fn
refusal_text(verb, flag) -> Option<String>` that `admit` prints, so the test
reads the exact bytes podbox prints without a subprocess.

**Failure control.** Each new test needs a plant. `every_none_row_...` fails if
one `None` row's status is flipped to `Degraded` in a scratch copy of
`parity.rs`. `an_unlisted_flag_...` fails if `admit`'s `Option::None` arm is
changed to return `Ok(())`. `every_non_none_row_is_admitted` fails if one
non-`None` row's status is set to `None`. `scripts/plant.sh` already names
`crates/podbox-cli/src/parity.rs` in its `FILES` list (`scripts/plant.sh:51`),
so the backup
and restore already cover it and a case needs no new file entry.

**Proof command.**

```
cargo test -p podbox-cli parity::
```

No feature flags, no target triple. The crate builds on the host target. On a
Windows host, `sh scripts/windows/run-in-base.sh` with `PODBOX_ARTIFACTS` set.

**The `exit 2` path.** Assertions 1 and 2 do not have one: the table is a
compile-time constant, so "could not read the table" is a compile error, which
is a stronger refusal than exit 2. `parity::json()` is the only consumer that
reads it dynamically, and the unit tests call it directly rather than shelling
to the binary. Assertion 4's "could not run" is the remaining third state and
it stays in the drive.

**The saved result.** Superseded for clauses 1, 2 and 3, because the unit
tests hold the same properties from inside the process and cannot go stale
against a row count. Kept for clause 3, and the file is re-driven. T-0808's
re-drive record at lines 700-713 must then be amended with the new count, since
it currently reads 164 against the source's 265.

## Group 3 — experiments/368-run-decay.sh

### What it measures

Why early payload runs cost about 2 s and late ones about 0.08 s. Three
candidates, each with a stated refutation (lines 7-18): `run --rm` deleting the
unreferenced rootfs; page-cache warming of the probe storm; one-time fixup
steps. The drive times three keeper-less runs, one `create`, three kept runs,
`rm`, and two more runs, recording rootfs presence after each step.

Pinned inputs: the debian digest at line 40, the same one 155 uses.

Host assumptions: `cargo` (line 51, exit 2), `./scripts/common/bootstrap-env.sh`
(line 53, exit 2 on failure), `cargo build` (line 61, exit 1 on failure), and a
Linux lane. The script takes the checkout from `pwd`, not `$0` (line 34),
because "the Windows wrapper runs the job at /in/job.sh with /work as the
working directory".

Exit contract: 0 every prediction held, 1 the drive ran and one failed, 2 the
lane could not run.

Timeouts: 1200 s bootstrap, 1800 s build, 120 s per timed command. Cleanup:
none beyond the work directory, which is not removed by the script.

Controls: this is the best-controlled script in the group after 190. The
`probe` leg at line 135 is a positive control with no rootfs work, and it bounds
every flat per-run cost below 0.1 s. The rootfs presence after each step is
recorded independently of the wall time, so a fast run with a missing tree is
visible. The keeper message on stderr is checked as its own file (line 95).

Result file: `experiments/results/run-decay-368.txt`, and it is also copied to
`/out` at line 176.

### TODO entries that own it

- `TODO/gate.md` T-1340, line 1489 names it in the Done record: "`experiments/
  368-run-decay.sh` exits 0 (`experiments/results/run-decay-368.txt`), 11 of 11
  predictions held".
- `TODO/gate.md` T-1341, line 1527 cites the same file as its `Source`.

Result file and entry agree. The result's verdict block (lines 131-145) lists
eleven `hold:` lines and `verdict DECAY ISOLATED`. T-1340's record gives
`a1 1.746 s`, `a2 4.776 s`, `a3 4.778 s`, `create 4.764 s`, `b1` through `b3`
`0.074 to 0.078 s`, `c1 0.108 s`, `c2 5.129 s`, probe `0.059 s`. Every number
matches `experiments/results/run-decay-368.txt` lines 132 exactly.

### Verdict: RUST-TEST

The measurement isolates one code path, and that path is 30 lines of Rust:
`crates/podbox-cli/src/run.rs:951-982`, the `--rm` arm, calling
`podbox_supervise::referencing` at line 963 and
`podbox_extract::remove_extracted` at line 973. `referencing` is
`crates/podbox-supervise/src/lib.rs:312`.

Six of the eleven predictions are not timing at all. They are about whether the
rootfs is present, whether the keeper message is printed, and which branch
`--rm` took. Those are pure state assertions over a scratch store and do not
need a wall clock.

Four predictions are wall-time. Those are the ones that cannot become a unit
test, and they do not need to: T-1341 already budgets the warm cost in
`experiments/360-perf-harness.sh` as the `run.kept` row, with a 0.25 s ceiling
and the sibling rss ceiling, and its Done record at `TODO/gate.md:1558-1565`
reports `run.kept 0.030 s` on the lane and `0.060 s` on the kvm base. The
timing half has a home. What has no home is the branch half.

### Conversion plan

**Assertions the script makes today.**

1. Precondition: a store with a pulled and extracted image and no container
   record referencing its digest. Action: `run --rm` the image. Expected: the
   rootfs directory is removed, and no keeper message is printed.
2. Precondition: the same store with one container record referencing the
   digest. Action: `run --rm` the image. Expected: the rootfs stays, and stderr
   carries `podbox run: --rm keeps the rootfs: it is referenced by container
   NAME` with the names sorted.
3. Precondition: the same store, `referencing` returns an error. Action:
   `run --rm`. Expected: the rootfs stays and stderr carries `--rm could not
   check references`, because "deleting on an unanswered question is how the
   defect above happens" (`run.rs:961-962`).
4. Precondition: `remove_extracted` fails. Action: `run --rm`. Expected: the
   rootfs stays and stderr carries `--rm could not remove the rootfs`.
5. Precondition: a store with a `create`d keeper. Action: `rm` the keeper.
   Expected: the rootfs is present afterwards.

**Rust level.** Pure unit, for 1 through 4: each needs a scratch store, a
store record and a payload that runs. The existing `#[cfg(test)] mod tests` in
`crates/podbox-cli/src/run.rs` already builds scratch stores, since lines
1678, 1705, 1763, 1806 and 2033 all call `remove_dir_all` on scratch paths. The
fakes needed are a store fixture and a digest string; the image does not need
to be real, because the assertions are about the store's tree, not the payload's
output. Assertion 5 is the same shape.

The two branches that need a fault are 3 and 4: a `referencing` error and a
`remove_extracted` error cannot be produced by a live service on demand, which
is exactly `docs/conventions/code.md:28`'s fault kind. They need the error
injected at the call boundary. Today the call is `podbox_supervise::referencing`
called directly, so the injection point is a small `pub(crate) trait` or a
closure parameter, and that change is the whole cost of the conversion.

**Target file and module.** `crates/podbox-cli/src/run.rs`, the existing
`#[cfg(test)] mod tests` that owns the `remove_dir_all` scratch helpers. New
test names: `run_rm_removes_the_rootfs_when_nothing_references_it`,
`run_rm_keeps_the_rootfs_and_names_the_referring_container`,
`run_rm_keeps_the_rootfs_when_the_reference_query_fails`,
`run_rm_keeps_the_rootfs_when_removal_fails`,
`removing_the_last_keeper_leaves_the_rootfs`.

**Fixtures and fakes.** The store fixture: reuse the scratch-store helper
already in `run.rs`'s test module. A container record: build it through
`podbox_supervise::create` with a name, which is what the real path does. The
two fakes: one for `referencing` returning `Err`, one for `remove_extracted`
returning `Err`. Both are one-line error values today, since the errors are
`podbox_supervise::Result` and `podbox_extract::Result`.

**Failure control.** `run_rm_keeps_the_rootfs_when_the_reference_query_fails`
is the plant for the whole conversion: it fails the moment the `Err` arm is
deleted, and that arm is the one whose removal would silently delete a rootfs a
keeper needs. `scripts/plant.sh` already names `crates/podbox-cli/src/run.rs`
in its `FILES` list (`scripts/plant.sh:51`), so no new file entry is needed.
Three more cases
follow: flipping the `Ok(referrers) if !referrers.is_empty()` guard, inverting
the `Ok(_)` arm, and removing the `Err(e)` arm.

**Proof command.**

```
cargo test -p podbox-cli run::tests::
```

No feature flags. The crate's tests are inline and need no `tests/` directory,
which is why this is not a new integration test: the code under test is a
private arm of a private function, and a `tests/*.rs` file could not reach it.
`crates/podbox-ssh/tests/` is the only such directory today, and this does not
need a second one.

**The `exit 2` path.** A unit test has no third state, and none is needed: the
"could not run" in this script is the lane lacking a toolchain, which becomes
the absence of the test rather than a code path. The one genuine third state
inside the code, the `referencing` error, becomes a fault test with a real
`Err`, which is stronger than a shell-level "could not run".

**The saved result.** Superseded for the branch half, because the unit tests
hold it from inside the process. Kept for the timing half, and cited as the
`Source` of T-1341 (`TODO/gate.md:1527`). The file stays where it is and T-1341
keeps its citation.

## Group 3 — experiments/383-ssh-liveness.sh

### What it measures

Whether every stalled SSH operation ends inside its bound, whether a dropped
node socket redials and pairs again, and whether the workers clean up. Seven
clauses, and five of the seven are named `cargo test` invocations.

Pinned inputs: the workspace at `git rev-parse --short HEAD` (line 34). Loopback
only, per the header at lines 10-16: "no live relay, no KVM guest, no traffic
past 127.0.0.1".

Host assumptions: `cd /work` succeeds (line 21, exit 2); `bootstrap-env.sh` with
`rust cc zig tools openssh` (line 39, exit 2); `timeout` (line 41, exit 2);
`mkdir /run/sshd` (line 40, exit 2).

Exit contract: 0 every clause held, 1 a clause disagreed, 2 the lane could not
run. Note that line 142 is `[ "$fail" -eq 0 ]`, which yields 1 on failure and 0
otherwise, and never 2 after the bootstrap.

Timeouts: 25 m on the build, 10 m on clauses 2, 3 and 4, 15 m on clause 5.

Cleanup: `rm -rf "$work"` on exit. Clause 7 scans `/proc` for a leftover
`node` or `operator` process, which is a hygiene check on the suite, not a
clause about the subject.

Controls: clause 6 greps the results log for the fixed test constants
`node-tok-z` and `conn-tok-z` and fails if either appears, which is a real
control. Clause 7 is a second control. But clauses 2 through 5 add nothing: they
are `cargo test -p podbox-ssh --lib transport::tests::stalled_resolver` and
three more named filters, plus one `--test mux_two_client` filter. Each is a
named Rust test. The script asserts only that the filter matched something,
which a `cargo test` filter that matches nothing reports as success.

Result file: `experiments/results/ssh-liveness.txt`.

### TODO entries that own it

- `TODO/podssh.md` T-1406, line 189 is the `Prove`: "A tracked drive must also
  end each stalled operation within its bound, prove reconnect pairing, and
  check worker cleanup." Line 198 records the run.

Result file and entry agree. The result's seven clause lines and the verdict
`STALLED OPS END BOUNDED, NODE REDIALS AND PAIRS` match the entry's record
verbatim, including `cargo test -p podbox-ssh` lib 101 passed with 7 new fault
tests and relay 15 passed.

### Verdict: DELETE

The assertions are already Rust tests, and the entry says so. T-1406 lines
196-198: "`cargo test -p podbox-ssh` exits 0: lib 101 passed with 7 new fault
tests (stalled resolver, non-reading peer, worker cleanup), relay 15 passed
with the new reconnect mode. `sh experiments/383-ssh-liveness.sh` exits 0".

Each of clauses 2, 3, 4 and 5 names a test that exists in
`crates/podbox-ssh`:
`transport::tests::stalled_resolver`,
`transport::tests::non_reading_unix_peer_ends_writes_within_the_bound`,
`transport::tests::non_reading_tcp_peer_ends_writes_within_the_bound`,
`transport::tests::pipe_write_names_its_leg_on_a_stall`,
`ws::tests::stalled_frame_send_ends_within_the_write_bound_naming_the_leg`,
`transport::tests::resolver_workers_do_not_accumulate_across_iterations`, and
`node_redial_pairs_a_second_session_after_the_first_socket_drops` in
`crates/podbox-ssh/tests/mux_two_client.rs`.

`cargo test -p podbox-ssh` runs all of them. The script adds the two log
controls, which belong in the Rust test that greps its own output, not in a
shell wrapper that greps a file the tests wrote to.

The delete carries one record edit. T-1406's `Prove` at line 189 requires "a
tracked drive", and the whole drive goes. Under `docs/conventions/prose.md:44`,
`docs/history/methodology` holds the superseded wording; the entry's `Done`
record keeps its measured numbers.

## Group 3 — experiments/40-language-selection.sh

### What it measures

Which implementation language can supply four things podbox needs: a
`PT_INTERP`-free static binary, a single-threaded process at `main()`, an
`LD_PRELOAD` interposer, and a small artifact. Candidates are Rust, Go and C,
with C as the control.

Pinned inputs: `experiments/src/lang/rust-static/main.rs`,
`experiments/src/lang/rust-shim/lib.rs`, and `experiments/src/lang/go-static/`.
The C probe, the C shim, the Go shim and the victim payload are written
inline as heredocs (lines 80-207).

Host assumptions: `gcc` is required as the control, and its absence exits 2
(line 40). `rustc` and `go` are optional; a missing one prints a row and
continues.

Exit contract: 0 the comparison ran, 1 a candidate failed a property it claims,
2 could not run.

Timeouts: 20 s on each shim check (line 141), reported as `HUNG` on rc 124.

Cleanup: none. It writes into `${OUT:-$HERE/.langbuild}` and never removes it.

Controls: C is the control, and the header says it "is here to prove the probes
work" (line 15). The victim forks, and a shim that only survives a straight-line
program has not been tested (lines 101-105). The Go answer is structural and
the script says so (lines 178-181).

Result files: none. It prints to stdout. No result file exists for it in
`experiments/results/`.

### TODO entries that own it

None. A search of `TODO/*.md` for `40-language-selection` and for
`language selection` returns nothing. The script's own line 222 points at the
decision it supports: "TOOL.md section 3 carries the decision these rows
support."

The decision it supports is recorded. `TODO/RULES.md:115` lists "Rust; static
musl release by default" in the standing-decisions table, sourced to "captured
TOOL.md section 3; T-1002". T-1001 (`TODO/packaging.md:17-25`) measured the
static property on the artefact, and its `Prove` at line 30 is
`readelf -l … | grep -c INTERP | grep -qx 0`.

### Verdict: DELETE

The measurement is obsolete in two senses at once. Its premise, that a
language must be chosen, was disproved by a decision already recorded in
`TODO/RULES.md:115`. And the one property it measured that still applies, the
`PT_INTERP`-free static binary, is measured on the real artefact by T-1001's
`Prove`, which reads the shipped binary rather than a toy program.

The remaining rows are properties of Go's runtime, not of podbox, and
`TODO/interpose.md` already carries the finding the script's own reading
section states: no preload of any language sees a Go payload's `lchown`
(lines 219-221 of the script, and the interpose entries that depend on it).

The script is not cited by any entry, so the delete has no record edit beyond
`experiments/README.md`, which names the first three scripts as the initial
target reconstruction (`experiments/README.md:18-20`) and does not name 40.

## Group 3 — experiments/401-emulator-streams.sh

### What it measures

Whether the Windows guest driver drains or redirects the emulator's streams. The
real assertion is a mutation: the script rewrites
`crates/podbox-windows/src/lib.rs` with `sed` to restore the unread pipes, and
asserts the named test then fails.

Pinned inputs: `TEST=a_noisy_emulator_is_not_blocked_on_its_own_output` (line
12), `SRC=crates/podbox-windows/src/lib.rs` (line 13).

Host assumptions: a git checkout (line 10, exit 2), `cargo` (line 11, exit 2).

Exit contract: 0 matched, 1 failed, 2 could not run. The `exit 2` cases are the
missing checkout, the missing cargo, and the mutation not changing the source
(line 35-38).

Timeouts: `timeout 900` on each `cargo test`.

Cleanup: a `trap` on `EXIT HUP INT TERM` copies the saved source back and
removes the temp (line 15). This is the one script in the group with a
correct restore trap.

Controls: this script is a control. It is the mutation half of the proof, and
the mutation is a positive one: it asserts the test fails against the defect it
claims to detect, which is `docs/conventions/code.md:32` in shell form.

Result file: `experiments/results/emulator-streams.txt`.

### TODO entries that own it

- `TODO/milestones.md` T-1112, lines 881-887: "Source defect found 2026-09-30:
  `podbox_windows::run` piped the emulator's stdout and stderr and did not read
  them before exit. … The unit test
  `a_noisy_emulator_is_not_blocked_on_its_own_output` proves it."
- `TODO/PROGRESS.md:28` cites the result file.

Result file and entry agree. `experiments/results/emulator-streams.txt` line 8
records `ok: a_noisy_emulator_is_not_blocked_on_its_own_output passes`, lines
10-11 record the mutant panicking with `the run blocked for 30.011004565s`, and
line 13 records `verdict EMULATOR-STREAMS-OK`. The entry names the same test
and the same defect.

### Verdict: DELETE

The repository already owns this proof, in the place the rules put it.
`AGENTS.md:77`: "Add a plant with each new check. A green check without a
failure test is insufficient." `scripts/plant.sh` is that mechanism, and its
header (lines 4-31) states its own contract: each case asserts the planted
defect's OWN message appears and that it did not appear on the clean tree, and
guards 1 to 4 cover a mutation that matches nothing.

The test the script drives is a normal Rust test at
`crates/podbox-windows/src/lib.rs:635-691`, in the crate's `#[cfg(test)] mod
tests`. It carries its own comment naming the defect, and it does not need a
shell wrapper: `cargo test -p podbox-windows --lib` runs it.

The specific `sed` the script applies is worth keeping somewhere. It rewrites
two lines: `crates/podbox-windows/src/lib.rs:226` `.stdout(Stdio::null())` to
`.stdout(Stdio::piped())`, and line 227's `.stderr(stderr)` to
`.stderr(Stdio::piped())`. That pair becomes a `scripts/plant.sh` case whose
mutation is asserted to land, using plant guard 1, which hashes the files
before and after.

`experiments/README.md:44` lists 401 in the repository audit proofs table with
the required condition "Linux Rust toolchain, no guest; a fake emulator's
256 KiB streams do not block the run, and restored unread pipes fail the
test". That row is corrected in place to name the plant case, under
`docs/conventions/prose.md:41`.

## Group 3 — experiments/targetfs.sh

### What it measures

Nothing asserts. It reconstructs the target's mount topology as pid 1 of a
privileged container, then `exec`s `/workspace/.harness/enter.sh`.

The topology, per its own header (lines 8-14): a tmpfs root owned by uid 1000,
a 64 MiB tmpfs `/tmp`, a 256 MiB `/dev/shm`, six bind-mounted device nodes and
no others, `/usr /lib /lib64 /bin /sbin` from the host, a handful of individual
`/etc` files, `/proc`, and two writable trees. Absent on purpose: `/etc/passwd`,
`/run`, `/var`, `/dev/fuse`, `/dev/ptmx`, `/sys`.

Pinned inputs: `TARGET_HOST_UID` default 1000 (line 20), `TARGET_WORKSPACE`
optional bind (line 82).

Host assumptions: uid 0 (line 25, `die` exits 2), a tmpfs mount capability
(lines 26-27, `die` exits 2), `pivot_root` (lines 87-91).

Exit contract: 0 ran, 1 the payload failed, 2 could not run. `die` is the only
writer of 2.

Timeouts: none. Cleanup: none; the container's exit is the cleanup.

Controls: the deliberate absences are the control. Line 30: "Creating `/run`
or `/var` here would quietly repair the very absence half the paper's failures
depend on." Line 67: `mknod(2)` is denied inside, so anything not bound cannot
be created, which is why every FUSE path is unreachable. Line 56: `/etc` is
never copied whole, because `getpwuid(0)` failing is load-bearing.

Result files: none. `experiments/README.md:28` names it: "Reconstruction entry
point; do not run it on the host".

### TODO entries that own it

No entry names `targetfs.sh` directly. It is the body behind
`experiments/20-enter-target.sh`, which `experiments/Dockerfile.target:42-45`
copies and makes the `ENTRYPOINT`. Its owners are therefore the entries that
own `20`:

- `TODO/gate.md` T-1213, lines 1111 and 1127.
- `TODO/milestones.md` T-1108, line 533.
- `TODO/milestones.md` T-1101, line 78.

### Verdict: KEEP-SHELL

It is the container's pid 1 and it exists to call `mount(2)` and
`pivot_root(2)` before any confinement is applied, which its own header states
at lines 4-7: "Everything here needs mount(2), which is exactly what the
reconstructed runtime will not have - so it all happens before the confinement
is applied, and none of it is reachable from inside."

A Rust binary could do the same syscalls, but it would be a second
implementation of a mount topology that exists to be a fixture for a Go
harness, and the file is 95 lines of shell with no logic to test. The specific
host dependency is the container runtime's privileged mount and `pivot_root`
support, reached through `eng_privrun` in `20`. Nothing to convert.

## Group 3 — scripts/package-ssh.sh

### What it measures

Whether the release ships the exact SSH helper names with runnable static bytes.
Four helpers, `node`, `operator`, `proxy` and `shell`, each checked for: an
executable present, a readable ELF header, no `PT_INTERP` in the program
headers, and a `--help` run that exits 125 with `usage: <helper>` in the
output. Then a reproducible tarball with the licence notices and a sha256 file.

Pinned inputs: three positional arguments, `RELEASE_DIR`, `ARCH` and an
optional `QEMU` (line 3). `ARCH` is checked against a seven-value allowlist
(line 12). The output archive path is `$ROOT/dist/podbox-ssh-$ARCH.tar.gz`
(line 47).

Host assumptions: `readelf` (lines 19-22, exit 1 on a bad header), `tar` with
`--sort` and `--mtime` (line 43), `gzip -n` (line 45), `sha256sum` (line 49),
and `python3` running `scripts/release-licenses.py` (line 41).

Exit contract: 0 matched, 1 failed, 2 could not run. The `exit 2` cases are a
missing directory, an empty arch, an arch outside the allowlist, a failed
`mktemp -d`, and a missing `dist` (line 46). The `exit 1` cases are each
`SSH-PACKAGE-FAIL` message.

Timeouts: `timeout 15` on each `--help` run (lines 27, 30). Cleanup:
`trap 'rm -rf "${WORK:?}"'` on exit (line 14).

Controls: the missing-helper and invalid-ELF cases. T-1405's Partial record at
`TODO/podssh.md:168-172` names them: "The missing-helper and invalid-ELF
controls fail with their own messages."

Result files: it writes to `$ROOT/dist/`, not to `experiments/results/`. The
`dist/` artifacts are the nightly's upload path, per
`.github/workflows/nightly.yml:112`, which calls the script with
`target/${{ matrix.triple }}/release`, `matrix.arch` and `matrix.qemu`.

### TODO entries that own it

- `TODO/podssh.md` T-1405, line 156 is its `Prove`: "`sh scripts/package-ssh.sh
  RELEASE_DIR ARCH QEMU` exits 0; the release matrix publishes every helper
  archive with its digest and signature."
- `TODO/podssh.md:162` cites `experiments/results/publication.txt` and line
  164 cites `experiments/results/ssh-release-beta10.txt`.

There is no saved result file of its own. T-1405's Done record points at
`experiments/399-publication.py` and `scripts/verify-release.sh` for the
publication half, and at `experiments/results/repo-audit-linux.txt` for the
native packaging half. The script's own exit code is the evidence, and the
nightly workflow is the record.

### Verdict: RUST-TOOL

This is a packaging step in a release pipeline, run by a CI workflow. It is not
a measurement: it produces an artifact. `docs/methodology/experiments.md:3`
requires a tracked script for "a reported measurement", and this reports none.

The packaging logic is pure and belongs in Rust: which helper names are
required, the arch allowlist, the reproducible-tar argument set, the sha256
line, the notice set. The three host tools it shells to have Rust equivalents
or are not needed: `readelf` is a program-header parse, `tar --sort=name
--mtime=@EPOCH --owner=0 --group=0 --numeric-owner` is a deterministic archive
write, and `sha256sum` is a digest. The `python3` call at line 41 is a
second implementation in another language and the clearest single argument for
the move: `scripts/release-licenses.py` is 72 lines and belongs in the same
binary.

Crate: new `crates/podbox-release`. Binary: `podbox-package-ssh`.
`[[bin]]` only, no library half, because the CLI surface is the whole product.
The crate is new; the release surface has no home today.

CLI surface:

```
podbox-package-ssh --release-dir DIR --arch ARCH [--qemu PATH]
                   [--dist DIR] [--output-dir DIR] [--epoch SECONDS]
                   [--licence-notice]
```

`--arch` keeps the seven-value allowlist at line 12 as a typed enum, so an
unknown arch is a parse error rather than an exit 2 from a `case`.

### Conversion plan

**Assertions the script makes today.**

1. Precondition: a release directory holding the four helper binaries.
   Action: check each. Expected: each is executable, `readelf -h` succeeds,
   `readelf -l` shows no `INTERP`, and `--help` exits 125 with
   `usage: <helper>` on stdout or stderr.
2. Precondition: a helper is missing from the release directory. Action: check.
   Expected: exit 1 with `SSH-PACKAGE-FAIL missing executable: <helper>`.
3. Precondition: a helper is a dynamically linked ELF. Action: check.
   Expected: exit 1 with `SSH-PACKAGE-FAIL dynamic interpreter: <helper>`.
4. Precondition: all four helpers pass. Action: package. Expected:
   `dist/podbox-ssh-$ARCH.tar.gz` and a matching `.sha256`, the archive holding
   the four helpers, `LICENSE`, `SSH-SHIM-LICENSE` and
   `dependency-licenses`, built with sorted names, a fixed mtime, and owner
   and group zero.

**Rust level.** Pure, for 2, 3 and the arch allowlist, in a `#[cfg(test)] mod
tests`. Integration, for 1 and 4, because assertion 1 runs the helper's real
`--help` and assertion 4 writes a real archive; `docs/conventions/code.md:29`
calls this deployment proof, since the shipped archive is the subject.

**Target file and module.** `crates/podbox-release/src/main.rs` with the
package logic in `crates/podbox-release/src/package.rs`. The arch allowlist
becomes an enum in `package.rs`. `#[cfg(test)] mod tests` in `package.rs` for
the pure half; a new `crates/podbox-release/tests/package.rs` integration test
for the archive half, because it needs a real directory and a real tar read
back. This is the first crate besides `podbox-ssh` to need a `tests/`
directory, and the reason is that the assertion crosses a process boundary.

**Fixtures and fakes.** The four helper binaries: the crate's own
`podbox-ssh` bins, built by `cargo build -p podbox-ssh --bins`. A
dynamically-linked negative fixture: a tiny C file compiled without
`-static`, or a checked-in four-byte stand-in with a valid `PT_INTERP`; the
former needs `cc` and the latter needs no tool, so prefer a fixture that does
not. The licence texts: `LICENSE` and
`crates/podbox-ssh/shims/LICENSE` at the current line 39-40 paths.

**Failure control.** Two plants, and they are the two the entry already names
as controls. `an_absent_helper_is_refused_by_name` fails if the
missing-executable arm is deleted. `a_dynamic_helper_is_refused_by_name` fails
if the `INTERP` check is deleted, and its fixture must be a real dynamic ELF so
the arm has something to find. Both become cases in `scripts/plant.sh`, which
adds `crates/podbox-release/src/package.rs` to its `FILES` list. The third
guard, the `exit 125` and `usage:` contract, is asserted by a plant that
builds a fake helper exiting 0 and asserts the tool refuses it.

**Proof command.**

```
cargo test -p podbox-release
```

No feature flags. The integration half needs `cargo build -p podbox-ssh --bins`
first, which `cargo test` does not do; the integration test should build them
itself through `cargo` as a subprocess, or the proof command becomes the pair
below. The cleaner answer is for the integration test to use
`env!("CARGO_BIN_EXE_podbox-node")`, which cargo sets for a workspace member's
binaries and which the test then has directly.

**The `exit 2` path.** The tool keeps the same three exits. A missing release
directory, an unknown arch, a missing `dist` and a failed temp directory are 2,
because they are the operator's environment rather than the artifact's state.
A missing helper, a bad ELF, a dynamic interpreter and a failed usage contract
are 1, because those are the artifact's state. That is a clean split and it is
the script's existing split, so the migration is exact.

**The saved result.** There is none to move, so nothing is superseded. The
`dist/` artifacts and the nightly upload are unchanged. What changes is
`.github/workflows/nightly.yml:112`, which calls the new binary. The result
files T-1405 cites, `experiments/results/publication.txt` and
`experiments/results/ssh-release-beta10.txt`, belong to other tools and are
untouched.

## Group 3 findings

### F1. T-1304 is marked done, and the source rejects the protocol it measured

`TODO/podvm.md:389` reads `Status: done`. Its `Prove` at line 429 is
`./experiments/147-podvm-exec.sh`. The shipped arm contradicts the entry's
Decision in writing: `crates/podbox-cli/src/machine/ssh.rs:20-22` says "That is
the guest's shape, not `TODO/podvm.md` T-1304's: T-1304 multiplexes many
commands over one serial session with per-command nonces, while here the boot
is the session."

A search for `VMR-` across `crates/` returns nothing. The markers exist only in
`experiments/147-podvm-exec.sh` and in `.dev/` scratch.

Consequence: a Done entry's `Prove` names a script that measures a design the
source does not contain. `docs/conventions/code.md:34` says "A skipped test
proves nothing about its subject", and a test of an unimplemented design
proves nothing about the code either. What would settle it: an entry that
records the protocol as not implemented and moves the `Prove` to whatever the
shipped arm is measured by, or an implementation.

### F2. The parity table has 265 rows; every record of the drive says 164

`crates/podbox-cli/src/parity.rs:232-537` holds 265 `Row {` entries. The last
recorded drive, `experiments/results/parity-drive.txt`, says `164 rows, 202
driven, 0 mismatches, 2 unreachable here`, and its date is 2026-09-22. The
entries that cite it, `TODO/cli.md:700-713` and `TODO/image.md:847`, carry the
same 164.

The script's own gate is `length >= 60` (line 77), so 265 passes it and the
drive would report 265. No entry has re-driven. The result is historical on
its revision, which `docs/methodology/experiments.md:30` permits, but the
comparison the script exists to make, "the shipped binary agrees with every row
of the parity table it publishes", has not been made against the shipped
binary since 164 rows existed.

What would settle it: one re-drive. It is the cheapest open item in the group.

### F3. Four scripts have no automated check behind their entry

- `20-enter-target.sh` is the `Prove` of `TODO/milestones.md` T-1108
  (line 533) and half of T-1213's (line 1127). Running it needs a privileged
  container and a Go toolchain. The `readelf` half of T-1108's `Prove` is
  automatable; the `20` half is not.
- `147-podvm-exec.sh` is T-1304's only `Prove`, and per F1 it measures an
  unimplemented protocol.
- `10-language-selection.sh`'s subject, the static `PT_INTERP`-free property,
  is T-1001's `Prove` and that is `readelf`, not this script. The script adds
  nothing behind an entry.
- `targetfs.sh` has no entry of its own and is a fixture, so this is correct.

### F4. Two scripts carry the word "experiment" and are neither measurements nor checks

`targetfs.sh` is a container entry point, and `package-ssh.sh` is a packaging
step with a CI caller at `.github/workflows/nightly.yml:112`. Neither writes to
`experiments/results/`. `TODO/RULES.md:60` says "Save results under
`experiments/results/`", and `docs/methodology/experiments.md:3` requires a
tracked script for "a reported measurement". `package-ssh.sh` reports none.

This is not a defect in either script. It is a defect in the numbering, and
`experiments/README.md:18-20` already scopes the numbering to "the initial
target reconstruction" plus the later podbox measurements. What would settle
it: a record line saying which scripts are fixtures and which are pipeline
steps. The two group 9 scripts `397-exported-build.py` and
`399-publication.py` have the same shape and are not mine to rule on.

### F5. Two scripts have no result file at all

`40-language-selection.sh` and `156-closure-records.sh` both print to stdout
and write nothing under `experiments/results/`. T-1208's record at
`TODO/gate.md:563-567` quotes 156's numbers, and no artefact holds them. That
is the second half of T-1208's argument for a check rather than a reader, and
it is worth stating that the measurement that argued for the check could not
itself keep a record.

### F6. `368-run-decay.sh` does not clean its work directory

Line 37 creates `experiments/.sweep368-work` and removes it first
(line 37, `rm -rf "$WORK"; mkdir -p "$WORK/out"`), and no `trap` removes it
afterwards. The file's last lines (176-178) copy the report to `/out` and print
it. `TODO/RULES.md:85` says "Remove only scratch that this session owns".
The directory holds a store with a pulled image, so it is the largest leftover
in the group. This is a finding about the script, not about the entry.

### F7. The run-decay numbers the entry quotes are exact

Recorded because it is the one place a stale number would have been easy to
assume. `TODO/gate.md:1495-1502` quotes `a1 1.746 s`, `a2 4.776 s`, `a3
4.778 s`, `4.764 s`, `0.074 to 0.078 s`, `0.002 s`, `0.108 s`, `c2 5.129 s`
and `0.059 s`. `experiments/results/run-decay-368.txt:132` reads `walls:
a1=1.746 a3=4.778 b1=0.078 b3=0.074 c1=0.108 c2=5.129`, and lines 22, 30, 48,
66, 74, 82, 90, 100, 108, 116 and 126 carry the rest. Every quoted value
matches. The entry also correctly states the two candidates it left unsettled
(line 1505-1509) rather than claiming all three.

## Group 3 — entries read

Read in full, not from a grep line.

| Entry | File | Why it was read |
| --- | --- | --- |
| T-1303 | `TODO/podvm.md:314-380` | Owns the assembly 147 reuses, and names `experiments/lib/podvm-guest.sh` |
| T-1304 | `TODO/podvm.md:383-460` | `Prove` is 147; the Decision rules the nonce protocol |
| T-1305 | `TODO/podvm.md:463-534` | Named in T-1304's Done record as the `--podbox-mem` rows that moved the 164 count |
| T-0410 | `TODO/complete.md:585-677` | Cites 125's six measured `passwd` shapes and the unit test that drives them |
| T-0413 | `TODO/complete.md:941-1095` | `Prove` is 155; the procfs emulation clauses 4a-4f |
| T-0414 | `TODO/complete.md:1097-1227` | `Prove` is 155; the root-listing legs and clause 5 |
| T-0808 | `TODO/cli.md:655-713` | `Prove` is 325; the re-drive at 164 rows and 2 unreachable |
| T-1112 | `TODO/milestones.md:848-894` | Records the `podbox_windows::run` stream defect and the unit test 401 drives |
| T-1108 | `TODO/milestones.md:516-556` | `Prove` names 20 and the static `PT_INTERP` check |
| T-0207 | `TODO/image.md:568-708` | `Prove` is 190; the loopback and latency ratios |
| T-1340 | `TODO/gate.md:1440-1522` | Done record cites 368 and its eleven predictions |
| T-1341 | `TODO/gate.md:1525-1566` | `Source` is 368's result file; owns the `run.kept` timing row |
| T-1208 | `TODO/gate.md:549-640` | Premise is 156; the check that replaced it |
| T-1203 | `TODO/gate.md:141-213` | `Prove` is 125; the eleven-row reading |
| T-1211 | `TODO/gate.md:881-930` | Names 125 in the engine conversion and records its run |
| T-1213 | `TODO/gate.md:1102-1150` | Names 20 in the three-way conversion and records its run |
| T-1405 | `TODO/podssh.md:142-172` | `Prove` is `scripts/package-ssh.sh` |
| T-1406 | `TODO/podssh.md:176-208` | `Prove` names 383 and the `cargo test` it wraps |
| T-1002 | `TODO/packaging.md:64-63` | The standing decision 40 supports; read with T-1001 |
| T-1001 | `TODO/packaging.md:17-30` | The static `PT_INTERP` proof 40 measured as a toy program |

Twenty entries read in full, against a required ten.
