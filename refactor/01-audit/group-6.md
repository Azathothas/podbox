# Group 6 audit: 13 scripts, 2907 lines

## Group 6 summary

Thirteen scripts, 2907 lines. Verdict counts:

| Verdict | Count | Scripts |
| --- | --- | --- |
| RUST-TEST | 1 | `326-store-contention-prove.sh` (clause 4 only) |
| RUST-TOOL | 1 | `verify-release.sh` |
| KEEP-SHELL | 3 | `100-interpose-symbols.sh`, `180-registry-fixture.sh`, `290-microvm.sh` |
| SPLIT | 6 | `152`, `170`, `170`-companion `20-`-call, `351`, `356`, `370` |
| DELETE | 1 | `350-tool-live.sh` |
| RUST-TEST (gate clause only) | 0 | - |
| KEEP-SHELL (build helper) | 1 | `zig-ar.sh` |
| KEEP-SHELL (lane fixture) | 1 | `372-pull-ceiling.sh` |

Counting each script once, by its dominant verdict: **DELETE 1, RUST-TEST 1,
RUST-TOOL 1, KEEP-SHELL 4, SPLIT 6**. Line total read from
`refactor/00-orientation/assignment-map.md:74-86` and confirmed with `wc -l`
on every file: 338, 303, 274, 415, 233, 108, 167, 189, 501, 208, 116, 49, 6.

### The three most important findings

1. **`experiments/370-windows-guest.sh:205` writes `experiments/results/windows-364.txt`.**
   The script is 370; the result file carries 364. The number belongs to
   `364-qol.sh`, which is in Group 8. The file `experiments/results/windows-364.txt`
   exists (I read its head: date `2026-09-27T02:00:39Z`, `kvm absent`, `count 52`),
   so the collision is live, not theoretical. A grep for `windows-364` across
   `*.md` and `*.sh` returns only the script itself and
   `docs/history/session-2026-09-25-to-27.md:155`. No TODO entry cites it by
   that name. `TODO/milestones.md:868` names the script (`sh experiments/370-windows-guest.sh`),
   never the result file. So the file is unreadable by name from any record.

2. **`experiments/350-tool-live.sh` has no owning TODO entry.** A repository-wide
   grep for `350-tool-live` returns five paths: the script, the assignment map,
   the orientation inventory, `.dev/session-20260918-toollive/commit-msg.txt`, and
   the superseded `docs/history/experiments-README.md-before-2026-09-30.txt:105`,
   which calls it "the tool answers on a live machine, not only in fixtures".
   `TODO/RULES.md` section 6 requires "a tracked script with pinned inputs and
   printed conditions". The script has neither a pinned input nor a TODO owner.
   It also measures two binaries (`text-tool`, `wsl-toolkit`) that this repository
   does not build, ship, or test; its saved result
   (`experiments/results/tool-live.txt`, 2026-09-18) shows a Windows host at
   `MINGW64_NT-10.0-26200`. It is host-tooling verification, not podbox
   measurement.

3. **`TODO/gate.md:290` records `100-interpose-symbols.sh` as `known-absent` in
   the gate's own coverage table, yet `TODO/gate.md:827` names it as a `Prove`.**
   The two lines sit in the same entry (T-1210). I read both. The table row reads
   verbatim: `| 100-lifecycle-loop.sh (T-1105, T-0607) | 100-interpose-symbols.sh | 230- | <!-- known-absent -->`. The `Prove` field at line 827 reads
   `./experiments/80-interposer-abi.sh, ./experiments/100-interpose-symbols.sh`.
   The `known-absent` marker is a gate-coverage statement about
   `230-lifecycle-loop.sh`, whose own number is a different script. I could not
   settle from the row alone what the marker claims about 100, because the
   column headers are not on the line I read. What would settle it:
   `TODO/gate.md` section 3, the coverage table's own definition. I did not read
   that section.

## Group 6 — experiments/100-interpose-symbols.sh

**What it measures.** Four checks over a path-taking libc surface. Arm A: an
interposer defining `execve` alone, driven through seven exec entry points,
counting how many reach it. Arm B (the declared control): the same seven with
every entry point defined. Arm C: the exported symbol set of
`references/VHSgunzo__pathmap/tree/path-mapping.c`, read from a built object
with `nm -D --defined-only`, asserting every exec entry point arm A measured as
bypassable is defined. Arm D (the kept check): enumerate the undefined symbols
of a pinned debian rootfs read from outside with `readelf`, intersect with a
hard-coded `PATH_TAKING` list (lines 87-99), subtract what the interposer
defines, and report the remainder as a number.

**Pinned inputs.** Rootfs `debian@sha256:2f65600e1252c5649d2213e1d1ea4d74253d26514dc6530102a875e429245929`
(line 49). Subject `references/VHSgunzo__pathmap/tree/path-mapping.c` (line 48;
I confirmed the file exists, 80964 bytes). The C sources for the caller and the
two interposer arms are heredocs in the script (lines 121-187).

**Host assumptions.** `cc` or `zig` on PATH (lines 62-75, `exit 2` when both
absent). `nm` on PATH (line 113). An engine from `experiments/lib/engine.sh`
(soft, line 58). Linux for native execution; elsewhere the ELF is staged inside
the rootfs (lines 79-82, 208-218).

**Exit contract.** 0 every check that ran matched, 1 a check ran and did not
match, 2 could not run (lines 39-40, 112-114, 143, 189, 191, 200, 284-303).

**Timeouts.** `eng_run 120` per caller run (line 213), `eng_run 600` for the
rootfs tar (line 301). `curl -sS` fetch inside `eng_pull` (line 200).

**Cleanup.** `rm -rf "$OUT"` at line 46 wipes the scratch before the run; the
scratch `experiments/.symbols` persists after, and I confirmed the directory
exists.

**Controls.** Arm B is the declared positive control and the script says so at
line 17: "⛔ THE CONTROL. The same seven with every entry point defined. Without
it, A's zeros are equally consistent with a broken harness." There is no failure
control. The saved result
(`experiments/results/interpose-symbols.txt`) shows `ok A: 1 of 7`, `ok B: 7 of 7`,
`ok C: every exec entry point A measured as bypassable is defined here`, and
`ok D: the gap is a number this command produces`. Arm D's numbers in the file
are 400 files read, 1839 distinct undefined symbols, 60 path-taking names
reached, 5 not defined.

**Owning entries.** `TODO/gate.md` T-1210 (read in full, lines 786-878; the
`Prove` at 827 and the `Done` at 831 with its transcript at 843-845).
`TODO/interpose.md` T-0703 (read in full, lines 283-402; the `Approach` at 332
names check D, the `Done` at 368 says the object "interposes 89 names", and
line 375 cites "experiments/100-interpose-symbols.sh's own rule" for excluding
descriptor-taking functions). `TODO/gate.md:290` (coverage row, read).

**Verdict: KEEP-SHELL.** Three host dependencies no Rust crate in this tree can
replace. (1) It compiles C source from `references/VHSgunzo__pathmap/tree/` and
reads the resulting ELF's dynamic symbol table with `nm -D --defined-only`
(line 254). That is an object-file reader against a third-party `.so`, and the
repository has no ELF dynamic-symbol reader. (2) Arm A and arm B are
`LD_PRELOAD` interposition experiments: they measure which calls a C-compiled
caller routes through a preloaded object, and they need a C compiler plus a
loader on a live Linux kernel. (3) Arm D streams a rootfs out of an engine
container with `tar` and reads 400 binaries with `readelf` (lines 301-309).

The durable Rust-shaped part is the `PATH_TAKING` list and the subtraction. That
belongs in `crates/podbox-interpose`, and I checked: the current object already
interposes 89 names and `crates/podbox-interpose/src/lib.rs` holds the entry
points (`execveat` at line 1012 through `path_at_int!`, and seven
`path_at_int!`/`path_at!` invocations by the script's own grep). Arm C's
assertion about podbox's own object is the one clause that reads as a Rust test,
and I name it below rather than folding it into this verdict.

**No conversion plan.** A KEEP-SHELL verdict carries no plan. The implementor
still owes one change if the operator wants the gap named from Rust: see the
findings section for the arm C test.

## Group 6 — experiments/152-nix-acceptance.sh

**What it measures.** Seven rows in one `podbox run` of the shipped binary
against a pinned nix 2.2.2 + nixpkgs 22.05 pair: REGISTER, FETCH, EVAL, BUILD,
RUN, NEGATIVE (raw `unshare -Urm` refusal named), PROCHOOKS (six setup hooks
using `< <(`). The header at lines 8-13 states the pin is forced: nix >= 2.3
calls `posix_openpt()` unconditionally and this runtime has no `/dev/ptmx`.

**Pinned inputs.** Tarball URL and `NIX_TARBALL_BYTES="23607712"` (lines 33-34),
`NIXPKGS_TAG="22.05"` and its tarball URL (35-36), base
`public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a`
(37). The subject is a heredoc, lines 80-208.

**Host assumptions.** `PODBOX_BIN` or the default release path (line 27). An
engine on a non-Linux lane (line 217). `timeout` (line 265).

**Exit contract.** 0 every row green, 1 a row failed naming it, 2 nothing ran
(lines 21-22, 45, 56, 217, 220, 224, 229, 251, 257-259).

**Timeouts.** `RUN_TIMEOUT=3600` default (line 40), `CURL_TIMEOUT=60` (41),
`eng_run 4500` for the driver (252), `eng_run 600` for the CA install (227).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` (line 47), a `mktemp -d` on
Linux.

**Controls.** Row 6 (NEGATIVE) is the declared negative row and the script says
why nix's own sandbox is the wrong probe (lines 178-182): "as root it clones
namespaces this chroot grants, or substitutes without building at all, so both
outcomes read wrong. Raw unshare is the request itself". There is no positive
control on the harness itself: rows 1-5 pass only if the whole pipeline works.

**Owning entries.** `TODO/milestones.md` T-1111 (read in full, lines 726-845;
`Prove` at 824, `Done` at 826 with the seven-row table at 836-844).
`TODO/interpose.md:1353` cites the result for the T-1111 nix unpack.
`TODO/podvm.md:847` and `:896` cite `152-nix-acceptance.sh:94` and
`experiments/results/nix-acceptance.txt`.

**Verdict: SPLIT.** The script mixes one durable acceptance with two pieces of
reusable logic.

- **Piece 1, the seven-row pipeline: KEEP-SHELL.** It needs a network fetch of
  a 23 MiB tarball over TLS from `nixos.org` and a `github.com` archive, a
  `nix-build` that compiles `hello` locally, and a real `unshare` syscall
  outcome. No Rust test can assert a remote tarball's byte count.
- **Piece 2, the tarball layout acceptance: RUST-TEST.** Lines 98-107 are pure
  decisions over a member listing: the list must contain `.reginfo`, and the
  layout must be either `./store/`-prefixed, `store/`-prefixed, or one wrapper
  directory. That is deterministic logic on a pinned input. The conversion plan
  is below.
- **Piece 3, the `PROCHOOKS` count: RUST-TEST.** Lines 201-207 count the setup
  hooks in a pinned nixpkgs tree that use `< <(`. Same shape.

**Result file.** `experiments/results/nix-acceptance.txt` and
`-transcript.txt` both exist. `TODO/milestones.md:828-829` names both. Keep
both as historical readings of 2026-09-21; the row verdicts in them stay the
record of the shipped binary.

## Group 6 — experiments/170-probe-cache.sh

**What it measures.** Three clauses about `$store/probe.json`. Clause 1: two
`podbox probe --json` runs on one machine agree on `.rung` (lines 90-113).
Clause 2: with `PODBOX_STORE` set, the first `probe --cached` prints "measured
now" and the second prints "served from", and the two stdouts are byte-equal
(lines 116-151). Clause 3 (the clause the entry exists for): a copy of the
host's store is staged into a confined process; the confined run must measure
freshly, name `mnt_ns` as a differing component, and select the lane's rung
(lines 154-238).

**Pinned inputs.** `DRIVER` image digest (line 55), `PODBOX_BIN` (line 23). The
boot id and rungs are read at run time, not pinned.

**Host assumptions.** `jq` on PATH (line 73, `exit 2` without). A podbox binary
that executes natively, or an engine to stage it in (lines 45-51, 66-72). The
script calls `experiments/20-enter-target.sh` (line 167) on a native lane.

**Exit contract.** 0 every clause held, 1 one did not, 2 could not run (line 18,
46-50, 68-71, 73, 105-108, 242).

**Timeouts.** `eng_run 120` per probe run (100-102, 128), `eng_run 300` for the
confined run (189).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` (line 33).

**Controls.** Clause 2's byte-equality check is the positive control for the
cache (`[ "$(cat c1.out)" = "$(cat c2.out)" ]`, line 147). Clause 3 is the
failure control: it asserts the cache is *refused*. The script prints the
reading either way and treats a served cache as a FAIL (line 255:
`cache_served_host %s   (no is the pass)`). There is no negative control that
plants a defect in the key comparison.

**Owning entries.** `TODO/probe.md` T-0111 (read in full, lines 594-703; the
`Prove` at 657, the `Done` at 659 with the reading at 664-676, and the test
name at 684 `the_cache_key_agrees_with_the_identity_block_it_is_derived_from`).
`TODO/gate.md` T-1211 (read in full, lines 881-910; the engine conversion, whose
`Prove` at 906 names this script).

**Verdict: SPLIT.**

- **Clauses 1 and 2: RUST-TEST, and they already exist as Rust.** I read
  `crates/podbox-image/src/probe_cache.rs` and found
  `the_first_run_measures_and_the_second_is_served_from_the_cache` (line 233),
  `a_cache_written_under_another_mount_namespace_is_refused_and_says_which`
  (line 247), `a_cache_keyed_on_the_boot_id_alone_would_not_have_caught_that`
  (line 268), and `an_answer_taken_by_another_instrument_is_never_served`
  (line 296). These are exactly clauses 2 and 3's assertions, in Rust, with
  fixtures the script does not have. The script's clause 2 and 3 are therefore
  re-measurements of existing unit tests, and the conversion is to delete them
  from the script and cite the Rust tests.
- **Clause 1 (`podbox probe --json` twice agrees): KEEP-SHELL.** It drives the
  installed binary's CLI and compares two documents. That is an
  integration-deployment check by the three kinds in `docs/conventions/code.md`
  section "Verification", and `probe --json` does not touch the store, so no
  crate-level test can produce it.

**Contradiction found.** The script's own comment at line 117-118 says: "The
helper carries no `-e`". The script is `#!/usr/bin/env bash` with
`set -uo pipefail` (line 19), which is `-u` and `-o pipefail` but no `-e`. That
claim holds. No defect. Recording it because the comment is unusual and a
reviewer will check it.

## Group 6 — experiments/180-registry-fixture.sh

**What it measures.** That podbox can pull the four OCI distribution endpoints
from a loopback `zot` fixture with all outbound network blocked. 21 clauses in
one `--network=none` driver container: two isolation clauses (`iso-1` no default
route, `iso-2` no name resolves), two config-verify clauses, six open-server
clauses (manifest by tag, by digest, blob, podbox pull by tag, by digest), five
required-server clauses (401 anon with Basic challenge, 200 authed, manifest
anon 401, manifest authed 200, wrong password 401), and two post-run clauses
(the report carries no FAIL, the test password is absent).

**Pinned inputs.** `zot-linux-amd64-minimal` v2.1.21, `85459246` bytes,
`sha256:d2422616a28dbae10a92c1df9daa23980e2f7f3deb0079968b928f23492196cb`
(lines 33-36), verified against the release checksums file every run (136-141).
Seed repository `fixture:t1` with hand-built blobs (148-183).

**Host assumptions.** An engine (line 55, `exit 2` without). `openssl` on PATH
with a Windows-spelling path (lines 228-234). `curl` with or without the
`ZOT_PROXY` fallback (lines 119-129). A podbox binary, built in the lane by
`scripts/windows/run-in-base.sh` when absent (lines 86-105).

**Exit contract.** 0 every clause held, 1 one did not, 2 could not run (line 19,
55, 101-104, 130-133, 141, 222, 233, 243).

**Timeouts.** `curl --max-time 600` for the binary (120), `60` for checksums
(137), `15` per inside clause, `eng_run 600` for the driver (389), `eng_build
600` (243).

**Cleanup.** `trap cleanup EXIT INT TERM` where cleanup calls `eng_cleanup` and
`rm -rf "$WORK"` (lines 57-61). The password lives in `$WORK` and is asserted
absent from the report (line 401).

**Controls.** `req-1` through `req-5` are a declared auth control set: the same
server shape answering 401 and 200. `iso-1` and `iso-2` are the negative
controls for the network claim, and the script says so at line 271: "the Prove's
block, asserted not assumed". There is no failure control that plants a bad
manifest.

**Owning entry.** `TODO/image.md` T-0206 (read in full, lines 430-558; the
`Prove` at 483, the `Done` at 485-522 with the trap list at 515-522, and the
closing `Partial`-style note at 551-558 that keeps it open at P3).

**Verdict: KEEP-SHELL.** Four host dependencies. (1) The `zot` binary is an
85 MiB third-party executable downloaded per run and verified by sha256; no Rust
crate in this tree replaces it, and `TODO/image.md:468` records the licence
decision (Apache-2.0) that made it acceptable. (2) The isolation clauses read
`/proc/net/route` and `getent hosts` from inside a `--network=none` container
(lines 272-281), which needs an engine, not a mock. (3) The certificate is made
by the host `openssl` with `MSYS_NO_PATHCONV=1` and Windows path spellings
(lines 228-234); that is a Windows lane artifact no test can arrange on Linux.
(4) The podbox pull clauses drive the shipped binary against a live TLS server
on container loopback.

**Contradiction found.** `TODO/image.md:507` records "one opt-in `ENG_NETWORK`
knob in `experiments/lib/engine.sh` (`none` only, unset by default, anything else
refused)". The script sets `ENG_NETWORK=none` at line 388 with a shellcheck
suppression for SC2034 and the comment "read by experiments/lib/engine.sh's
eng_run". I did not read `experiments/lib/engine.sh` in this pass, so I cannot
confirm the knob exists as recorded.

## Group 6 — experiments/290-microvm.sh

**What it measures.** Whether a QEMU microvm with a stock Alpine `virt` kernel
can answer the three probe questions this host cannot. Five clauses: the VM
booted and podbox ran; the `kcmp(2)` control answers `ESRCH`; Landlock answers
`ok`; `move_mount(-> /tmp/mm-probe)` is present in the document; the rung is
reported with an explicit warning that a fully-capable VM is the easy case.

**Pinned inputs.** `ALPINE_REL="${PODBOX_MVM_ALPINE:-v3.21}"` and the netboot
base URL (lines 48-49). The initramfs is built here from `busybox` plus the
podbox binary (lines 103-132). `PODBOX_BIN` (line 40).

**Host assumptions.** `qemu-system-x86_64`, `cpio`, `jq` on PATH (lines 59-64,
`exit 2` without). A `busybox` at `/bin/busybox` or `/usr/bin/busybox` (65-67).

**Exit contract.** 0 every clause held, 1 one did not, 2 could not run (line 35,
54-58, 59-64, 91-95, 152).

**Timeouts.** `BOOT_TIMEOUT=600` for the whole boot (line 52), `timeout 300`
per kernel file fetch (91).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` (line 43), `mktemp -d`.

**Controls.** The `jq -e .` validation of the carved JSON before any read
(lines 160-168) is the declared control, and the script states the failure it
prevents: "A `jq` over a truncated document answers `null` for every question,
which reads exactly like a measured absence." There is no failure control.

**Owning entry.** `TODO/probe.md` T-0100 (read in full, lines 760-835; the
`Approach` at 798, the `Prove` at 819, the `Done` at 821, and the premise table
at 781-787). The entry also names `T-0102` and `T-0101` as the questions the
script closes. `TODO/podvm.md:863` cites `experiments/290-microvm.sh:91` for the
`timeout 300 curl`.

**Verdict: KEEP-SHELL.** The dependency is `qemu-system-x86_64 -M microvm` and
the ability to build a cpio initramfs and read a document back off a serial
console. This is the clearest KEEP-SHELL in the group: a Rust test cannot
observe what a *different kernel* answers to `landlock_create_ruleset`. The
script says so at lines 27-32: "It is not the target runtime. It is a second,
DIFFERENT machine whose kernel has the options this one lacks".

**Result file.** `experiments/results/microvm.txt` exists and agrees with the
entry. I read it: `qemu exit 0`, `wall clock 6s, under TCG`, guest kernel
`6.12.81-0-virt`, guest LSMs `lockdown,capability,landlock`, VM `kcmp` `ESRCH`,
Landlock `ok   ABI 6`, `move_mount` `ok`, VM rung `namespace`, host rung
`namespace`. The entry's table at 781-787 says the same, and its measured
`move_mount` row says host `ok`, VM `ok`, target `EPERM`. The result file says
the same. **They agree.**

## Group 6 — experiments/326-store-contention-prove.sh

**What it measures.** Four clauses about the `podbox-image` test suite. Clause
1: run the suite N times (default 10) at libtest's default thread count and
count runs carrying the 16-slot signature `already holds 16 locks` (lines 45-69).
Clause 2: run `the_seventeenth_concurrent_registration_is_refused_by_name` alone
three times (71-81). Clause 3: one serial control, `-- --test-threads=1`
(83-89). Clause 4: audit structurally that every `#[test]` fn in
`crates/podbox-image/src/store.rs` takes `STORE_TESTS.lock()` (91-100).

**Pinned inputs.** `RUNS="${PODBOX_PROVE_RUNS:-10}"` (line 17). The test name
is a literal (line 74). The audit keys on the literal `STORE_TESTS.lock()`
(line 93).

**Host assumptions.** `cargo` and `rustc` on PATH (printed, line 32-33, not
gated). `git` for the HEAD short hash (line 30). `nproc` (31).

**Exit contract.** 0 the measurement ran and matched, 1 it ran and something
failed, 2 it could not run (lines 10-11, 13-15, 19-20, 37-40).

**Timeouts.** `timeout 1200` for the compile, `timeout 600` per parallel run,
`timeout 300` per ceiling run, `timeout 900` for the serial control.

**Cleanup.** `rm -f "$log"` per run (line 66). The results file is truncated at
the start (line 20).

**Controls.** Clause 3 is the declared serial control. Clause 4 is a structural
audit, and the script names the failure it prevents at line 98: "audit: MISMATCH
(a test without the mutex reintroduces the flake silently)". There is no plant:
nothing in this script makes the check go red.

**Owning entry.** `TODO/image.md` T-1310 (read in full, lines 1767-1902; the
`Prove` at 1881, the `Done` at 1883-1902 with the before/after table at
1889-1895, and the task list at 1862-1867).

**Verdict: SPLIT.** Clause 4 is the only clause that is a test rather than a
measurement, and it is a text audit of the source that a Rust test can own.
Clauses 1, 2 and 3 measure a property of the harness over time and cannot.

**Conversion plan for clause 4 only.**

**Assertion as a triple.** Precondition: `crates/podbox-image/src/store.rs`
contains N `#[test]` functions in its test module. Action: read the file and
count `#[test]`-annotated `fn` declarations and count occurrences of the
acquisition string. Expected: the two counts are equal and greater than zero.

**Level.** Pure test, deterministic logic on a fixed input file. Per
`docs/conventions/code.md` section "Verification", "Use pure tests for
deterministic logic." It is not a fault test: there is no service whose fault
must be produced. It is not integration or deployment proof.

**Target file and module.** `crates/podbox-image/src/store.rs`, inside the
existing `mod tests` (which begins before line 2539, where the ceiling test
lives). Name the test
`every_test_in_this_module_takes_the_store_mutex`.

**Fixtures.** None beyond the source file itself. The test reads
`file!()`'s own path, which is what makes it a self-audit. It must not read a
copy; the point is to audit the file the compiler just accepted.

**Plant.** The repository requires a plant per `scripts/plant.sh` header: "An
assertion no one has seen fail is not an assertion." The plant: delete one
`STORE_TESTS.lock()` acquisition line from one test body in `store.rs`, run
`cargo test -p podbox-image every_test_in_this_module_takes_the_store_mutex`,
and assert the test fails and names the test whose acquisition is missing. I
verified the acquisition string is present: `grep -c "STORE_TESTS.lock()"`
returns 29, and the `#[test]`-fn count from the script's own awk returns 29. So
the tree is currently balanced. The plant must be restored from a copy, never
`git checkout --`, per `scripts/plant.sh` guard 2.

**Exact command.**
`cargo test -p podbox-image every_test_in_this_module_takes_the_store_mutex`
No feature flags. No target triple: the audit is pure text and runs on the host.
On Windows the repository routes through `sh scripts/windows/run-in-base.sh`
(per `AGENTS.md` and `TODO/image.md:1873`).

**`exit 2` becomes.** A Rust test has no third exit code. The clause's
`exit 2` fires when the compile fails (lines 37-40). A compile failure in Rust
is a build error: `cargo test` exits 101 and the test never runs. The test must
therefore panic with a message naming the file if it cannot read its own source,
rather than skipping. A skip would read as green on a machine that proved
nothing, which `crates/podbox-ssh/tests/common.rs:9-11` names as the rule:
"A missing binary fails loud and names the binary. A skip would read as green on
a machine that proved nothing."

**Result file.** `experiments/results/store-contention-prove.txt` (758 lines) is
kept whole as a historical reading. It records the lane, the HEAD `f9aa0bb`, the
toolchain, and the clause 4 count. The entry cites it at `TODO/image.md:1900`.
The clause 4 result moves into the Rust test's own assertion; the result file
keeps the 2026-09-21 lane reading and does not need regenerating.

**Contradiction found.** `TODO/image.md:1894` records "mutex audit (test
functions vs acquisitions) | - (new) | 24 vs 24, equal" and
`TODO/image.md:1884` says "taken once by every test in it (24 of 24 by audit)".
The saved result file agrees: `test functions: 24, acquisitions: 24` at line 755.
The current tree disagrees: I ran the script's own awk and its own grep against
the current `store.rs` and both return **29**. The tree has grown by five tests
since 2026-09-21 and the balance still holds, so the entry's figure is a
historical reading rather than a defect. The finding is that the entry quotes a
number that no longer matches the tree, and the script re-derives it, which is
the correct arrangement.

## Group 6 — experiments/350-tool-live.sh

**What it measures.** That `text-tool` and `wsl-toolkit` answer on this host:
`text-tool --help`, a write/append/edit-with-`--expect`/count/eol round trip, a
`--expect` refusal at exit 1, `wsl-toolkit --version` and four `--help` calls,
`doctor`, `--instance podbox ready`, `base status --probe`, and one
`ready --smoke` container.

**Pinned inputs.** None. Lines 8-9: "Inputs: the `text-tool` and `wsl-toolkit`
binaries on PATH. No image, no network, no base state is assumed. Every version
is printed on the way out." The only environment assertion is
`echo "instance=podbox"` (line 35), a constant.

**Host assumptions.** Both tools on PATH (lines 36-37, 41, 51, 84, 108, 121).
`timeout` (109, 112, 122, 135).

**Exit contract.** 0 every leg that could run matched, 1 a leg ran and did not
match, 2 a leg could not run (lines 14-15, 165-166).

**Timeouts.** `timeout 60` on doctor, `120` on ready, `180` on base status,
`600` on smoke (lines 109, 112, 122, 135).

**Cleanup.** `trap 'rm -rf "$TMPWORK"' EXIT HUP INT TERM` (line 24).

**Controls.** The `--expect` refusal at line 67-69 is the only negative
control: it asserts exit 1 with the wrong expectation and nothing written.
There is no control for the `wsl-toolkit` legs.

**Owning entry.** None. This is finding 2 above.

**Verdict: DELETE.** Three grounds, each independent.

1. **No owning entry.** `TODO/RULES.md` section 6 requires a measurement to
   have a tracked script with pinned inputs. This script has no TODO entry, so
   nothing requires it to exist and nothing reads its result.
2. **It measures binaries this repository does not own.** `text-tool` and
   `wsl-toolkit` are named in `TODO/podssh.md` and `docs/` as host tooling this
   repository consumes, not as code it builds. `AGENTS.md` line 9 says the
   Windows route is `wsl-toolkit --instance podbox`, which makes them inputs to
   podbox, not subjects of podbox. `experiments/README.md` line 21 states the
   rule this script breaks: "Later scripts test podbox behavior. Their owning
   task gives the acceptance."
3. **Its result is already superseded.** The only other mention of the script
   in the repository is
   `docs/history/experiments-README.md-before-2026-09-30.txt:105`, a
   superseded file. The live `experiments/README.md` does not list it, and its
   "Repository audit proofs" table (lines 33-44) lists ten scripts, none of them
   this one.

**What becomes of the result file.** `experiments/results/tool-live.txt` should
be deleted with the script, or moved to `docs/history/` if the operator wants
the 2026-09-18 reading kept. `docs/methodology/history.md` owns superseded
records; I did not read it, so I cannot say which home it prescribes.
`experiments/README.md:60-63` says "Do not remove the saved result" and points
at `docs/history/experiments-README.md-before-2026-09-30.txt` as the retained
earlier guide, so a reader expecting the result file to persist is the concern.

**If the operator disagrees**, the fallback is RUST-TEST for the `text-tool`
round trip alone, as a `tests/*.rs` integration test in a new crate, using
`crates/podbox-ssh/tests/common.rs` as the template for the bounded-process
runner and its panic-on-absent-binary rule. That is a different project: it
would make this repository test a tool it does not build, which I do not
recommend and have not planned.

## Group 6 — experiments/351-signal-forward.sh

**What it measures.** Five clauses about signal forwarding from a waiter to a
payload. Foreground TERM forwards, is named, exits 143. Foreground INT forwards,
is named, the payload decides the exit, 130 for busybox `sleep`. A clean exit
stays quiet. A TERM to the `setsid`'d launcher is forwarded silently and `wait`
records 143. A payload that traps TERM is waited on, not killed, with the
forward line present.

**Pinned inputs.** `IMAGE="${PODBOX_RUN_IMAGE:-public.ecr.aws/docker/library/alpine:3.20}"`
(line 71), a tag, not a digest. `PODBOX_BIN` (55). `PODBOX_STORE` (56).

**Host assumptions.** `setsid`, `env`, `pgrep`, `pkill`, `timeout` (lines 76-82,
90; each `exit 2` when absent). The `env --default-signal=INT,QUIT` wrapper is
required, and the script explains why at lines 22-28: a `&`-backgrounded shell
without job control ignores SIGINT and SIG_IGN survives exec.

**Exit contract.** 0 every clause forwarded or waited as specified, 1 one did
not, 2 could not run (lines 39-40, 67, 77, 89-90, 93).

**Timeouts.** `timeout 120` on pull, `timeout 30` on create/start/rm,
`timeout 60` on run/wait, a 30-second poll loop for the payload (98-106), a
`sleep 6` before the trap check (line 179).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` (69). `clean_sleep` before
every foreground clause (126, 139, 174) and `pkill -9 -f '^sleep 30$'` after
(184).

**Controls.** The clean-exit clause (155-158) is the positive control: it
asserts `RC=0` and *no* `forwarded| died on` in stderr, so a waiter that always
prints cannot pass. The trap clause (173-186) is the failure control: a payload
that ignores TERM must survive. `no_orphan` after each clause (120-123) is the
leak check.

**Owning entry.** `TODO/supervise.md` T-1335 (read in full, lines 725-802; the
`Prove` at 758, the `Done` at 763-801, and the results citation at 777).
`TODO/gate.md:1632` names the script in a `Prove` for
`sh -n experiments/351-signal-forward.sh` (a syntax check, not a run).

**Verdict: SPLIT.**

- **Clauses 1, 2, 5 (the forwarding behavior): RUST-TEST.** The mechanism is
  `Child::wait_forwarding` in `crates/podbox-enter/src/lib.rs:299` and `serve`
  in `crates/podbox-supervise/src/launcher.rs:532`, both of which I confirmed
  exist at those lines. The unit names the entry already lists at
  `TODO/supervise.md:794-797` (`a_wait_status_becomes_dockers_exit_code`,
  `forwarded_signals_are_named_and_unknown_ones_numbered`,
  `the_shutdown_mask_holds_exactly_the_forwarded_pair`,
  `nothing_on_the_spawn_path_can_spawn_a_thread`) exist as Rust. What the
  script adds is the live signal delivery over a real payload.
- **Clauses 3 and 4 (the quiet exit, the launcher table record): RUST-TEST.**
  Both are decision logic over a wait status and a stderr buffer.
- **The live delivery: KEEP-SHELL.** Sending SIGTERM to a real process group and
  reading the payload's exit code needs a live kernel, a pulled image, and
  `pgrep`. `env --default-signal=INT,QUIT` is a `coreutils` behavior no Rust
  test can arrange around.

**Result file.** `experiments/results/signal-forward.txt` exists and the entry
cites it at line 777. I read its head: `bin: /work/target/.../podbox`,
`podbox 0.1.0`, the pull, and `term-rc=143`. The entry at 778-781 describes
exactly that. **They agree.**

## Group 6 — experiments/356-no-chroot-rung.sh

**What it measures.** Nineteen numbered clauses over a two-part denial fixture
(a mount namespace hiding `/dev/pts` plus `experiments/deny-chroot.c` denying
`chroot(2)` with EPERM). It drives the loader family on glibc and musl, the
memfd family on a locally built static hello, the strict arms, the refusal arm
with a rootfs byte-identity check, the forced ladder, the detached lifecycle,
workdir, maps, exe-force, script refusal, quiet, exec, records, the grandchild
limit, `readlink /proc/self/exe`, and `gcc --version`.

**Pinned inputs.** Four images (lines 80-87): pinned alpine
`sha256:d9e853e8...`, pinned debian `sha256:833d7afe...`, and two tags
(`golang:1.24.7-bookworm`, `gcc:13-bookworm`) whose digests are read back after
pull (166-167). A locally built `static-pie` hello (172-195) and a locally built
script image (200-210).

**Host assumptions.** `cargo`, `cc`, and `unshare -Urm` must all work
(97-99; each `exit 2`). `./scripts/common/bootstrap-env.sh` (101) and
`./scripts/build-interpose.sh` (109) must succeed. `scripts/common/exit-codes.sh`
is sourced for the exit table (129-130). The Windows wrapper runs this at
`/in/job.sh` with cwd `/work` (line 74).

**Exit contract.** 0 every clause matched, 1 a clause disagreed, 2 the lane
could not run (68-69, 97-99, 106-107, 113, 118, 162, 179, 186, 204, 227).

**Timeouts.** `timeout 120` per `step` (144), `1200` for bootstrap and
interpose, `1800` for the build, `600` per pull (161).

**Cleanup.** `rm -rf "$WORK"` at the start (76). No EXIT trap; `WORK` is
`experiments/.sweep356-work` and persists. The `/tmp/ul356-probe` escape check
removes its own probe (372).

**Controls.** The `fx-smoke` and `fx-probe` clauses (246-249) are the declared
fixture self-check, and the script says the fixture "proves itself before
anything asserts through it". `fx-grandchild` paired with `sane-grandchild`
(380-383) is a true positive/negative pair on the same limit. `sane-*` clauses
are the capable-host controls against the `fx-*` denial clauses. There is no
failure control that plants a defect in the code under test.

**Owning entries.** `TODO/enter.md` T-0503 (read in full, lines 170-280; the
`Done` at 253-278, which quotes the script's own output). `TODO/enter.md`
T-1317 (read in full, lines 515-680; the `Partial` at 606 and the `Done` at 627
with the five-command transcript at 652-672). `TODO/supervise.md` T-1421
(`Prove` at 829, the `Done` at 833 citing clause 16). `TODO/cli.md` lines 1451,
1501-1502, 1554-1555 (T-0804, T-1416 by context of the cited clauses).
`TODO/enter.md` lines 788, 841, 885, 945, 984, 1039, 1081 mark T-1407 through
T-1413, each of which names this script in its own record.

**Verdict: SPLIT.** This is the group's largest single script and it mixes
three different things.

- **Clauses 1-2, 21-19 (the rung decision and the refusal text): RUST-TEST.**
  `crates/podbox-cli/src/lifecycle.rs:1824` holds `decide_entry`. The clause
  bodies assert exit codes and message substrings against a decided rung, which
  is pure logic over a `Family` enum.
- **Clauses 4, 4b, 5, 9, 11, 12, 18, 19 (the live loader and memfd families):**
  KEEP-SHELL as a live drive. They need real glibc and musl loaders, a real
  interposer object on `LD_PRELOAD`, and a real rootfs. `crates/podbox-enter/src/userland.rs`
  already holds the unit guards the entry names at `TODO/enter.md:677-679`.
- **The fixture itself (`experiments/deny-chroot.c` plus the mount namespace):**
  this is the reusable part. It is a C file this repository already owns and
  compiles.

**Result file.** `experiments/results/no-chroot-rung.txt` exists and at least
eleven TODO lines cite it by name. I did not read its body in this pass. What
settles whether the current run still matches: reading the file's `verdict`
line. I am recording that as not done.

## Group 6 — experiments/370-windows-guest.sh

**What it measures.** Nine clauses over the machine tier's non-Linux guest
route. The crate's own `podbox-windows` tests pass; `windows --help` names
four subcommands and an unknown one exits 125; a bad sha256 pin refuses before
the URL is used; a 4096-byte body over a 1024-byte `--max-bytes` ceiling is
refused with neither the destination nor a `.part` left; `windows doctor`
names the accelerator or refuses at 125; `run --podbox-tier=machine
--platform windows/amd64` reaches the guest driver rather than the OCI path
and leaves the store byte-identical; and clauses 7 and 8, which need a
configured image, are skipped out loud when one is absent.

**Pinned inputs.** `PORT=8731` (41), `BIN` at a debug path (40), the four
images fetched at clause 4 (`head -c 4096 /dev/zero`, 107) and clause 7/8
(`PODBOX_WINDOWS_IMAGE`, `PODBOX_DOS_BASE`, 162, 185).

**Host assumptions.** cwd is `/work` under the Windows wrapper (35). A debug
podbox binary at the pinned path (58, `exit 2`). `python3` for the local file
server at clause 4 (108). `qemu-system-x86_64` is printed, not required (48-52).

**Exit contract.** 0 every clause matched, 1 a clause disagreed, 2 the lane
could not run (32, 58).

**Timeouts.** `--podbox-timeout 240` on setup (167). The clause 4 server is
killed at 115. Clause 7 and 8 time themselves with `date +%s` and report the
elapsed seconds (168, 173).

**Cleanup.** `rm -rf "$WORK"` at the start (38). The `python3` server is
`kill`ed at 115. No EXIT trap; `WORK` is `experiments/.sweep370-work` and
persists.

**Controls.** The bad-pin clause (98-102) is the negative control for the
digest check: it asserts exit 125 and that no `.part` exists. The ceiling
clause (104-126) has a named lane-limit branch at 118-121: if loopback TCP is
refused, the script records that the live ceiling is not exercised and points
at the crate's own no-network tests instead. That is a declared non-assertion,
and it is honest. The store byte-identity check (146, 156-157) is a mutation
control. There is no failure control.

**Owning entry.** `TODO/milestones.md` T-1112 (read in full, lines 848-894;
`Status: partial`, the `Prove` at 867-872, and the `Partial` record at 874-894
which names `369`, `370`, `371` and `392` in its `Source` line at 850).

**Verdict: SPLIT.**

- **Clauses 1-6: RUST-TEST, and clause 1 already is one.** The script itself
  runs `cargo test -p podbox-windows --lib` (72-73) and records
  `count 52` in the saved result. The help-surface, bad-pin, ceiling, doctor,
  and run-routing clauses are all decisions over argv, a filesystem, and an
  exit code.
- **Clauses 7 and 8: KEEP-SHELL.** Both need a licensed Windows image or a
  FreeDOS base, and the script states why at lines 10-13: "podbox ships no
  Windows image and no DOS base, fetches no licensed image itself, and
  redistributes none". A guest exit code and a guest banner cannot be asserted
  without the guest.

**Contradiction found (finding 1 above).** Line 205 writes
`experiments/results/windows-364.txt`. The script is 370.

## Group 6 — experiments/372-pull-ceiling.sh

**What it measures.** Two clauses on one image with two file-size ceilings set
by `ulimit -S -f` (512-byte blocks) around the pull alone. Clause 1: a 10 MiB
ceiling refuses the debian blobs up front at 125, naming the object bytes, the
ceiling, and the room, with no `*.partial` left. Clause 2: the same bytes pull
clean with no ceiling.

**Pinned inputs.** `DEBIAN` digest (line 30), `CEILING_BLOCKS=20480` (32), the
store path (65).

**Host assumptions.** cwd `/work` (24). `cargo` (44, `exit 2`).
`./scripts/common/bootstrap-env.sh` (46) and `cargo build` (54) must succeed.
`timeout` (46, 54, 79, 100).

**Exit contract.** 0 every clause matched, 1 a clause disagreed, 2 the lane
could not run (18-19, 44, 49-52, 58-59, 62).

**Timeouts.** `1200` bootstrap, `1800` build, `300` the limited pull (79),
`600` the control pull (100).

**Cleanup.** `rm -rf "$WORK"` at the start (26). The `ulimit` is set in a
subshell only (75-81), and the script says why at 74-75: "The limit applies
inside the subshell alone: the build above already wrote past it, and the
report stays outside it."

**Controls.** Clause 2 is the declared control: the same bytes, no ceiling,
clean pull. The script states the design at lines 6-9 and 13-14: "The mid-stream
cap against a lying origin is unit-pinned in `crates/podbox-image/src/registry.rs`
(an infinite reader served over loopback), because no honest registry lies on
demand." There is no failure control.

**Owning entry.** `TODO/image.md` T-1342 (read in full, lines 2194-2248; the
`Approach` at 2224 names this script, the `Prove` at 2234, the `Done` at 2237
with the numbers).

**Verdict: SPLIT.**

- **Clause 1's message shape: RUST-TEST.** `crates/podbox-image/src/space.rs`
  holds `file_ceiling()` (line 33) and `require()` (line 146), and the entry
  records that both the pre-flight and the mid-stream cap are unit-pinned
  (`TODO/image.md:2241-2244`: "including the infinite-body refusal, the
  exact-body pass and the pre-flight cases"). I confirmed `registry.rs:397`
  holds `fn drain<W: Write>`. The assertions this script makes about the
  refusal text are therefore already Rust.
- **The live `ulimit` pull: KEEP-SHELL.** `ulimit -S -f` sets `RLIMIT_FSIZE` on
  the process, and `SIGXFSZ` is the failure the entry is about
  (`TODO/image.md:2205-2207`). No unit test produces that signal. A Rust test
  could `setrlimit` on itself, but then it would be testing the kernel's
  behavior, not podbox's pre-flight, and the entry says the pre-flight is the
  subject.

**Result file.** `experiments/results/pull-ceiling-372.txt` exists and agrees
with the entry. I read it: `PULL rc=125`, the refusal line
`declares 28232655 byte(s), over this process's file-size ceiling of 10485760
byte(s) (945.60 GiB free ...)`: nothing was downloaded`, `partials none`,
`clause 1 HOLDS`, `clause 2 HOLDS`, `verdict PULL CEILING HOLDS`. The entry at
2237-2240 says "a 10 MiB soft ceiling refuses the debian blob (28232655 bytes) up
front at 125 naming the object, the ceiling and the room, with no partial
left". **They agree**, byte for byte on 28232655.

**Second finding inside this script.** Lines 33-41 write the `rustc --version`
and `cargo --version` output into the conditions block unfiltered, and the saved
result carries six lines of `rustup` auto-install chatter at the top
(`info: syncing channel updates`, `warn: the missing active toolchain ... has
been auto-installed`). `TODO/RULES.md` section 8 says "Bound external commands",
and `experiments/README.md:11` says "Required conditions were absent; the test
could not run". The chatter is not a defect in the measurement; it is noise in
the record. `372`'s result is the only Group 6 result file I read that carries
toolchain-install output in its conditions block.

## Group 6 — experiments/verify-release.sh

**What it measures.** That a downloader can verify a published nightly binary
or SSH archive against a sigstore bundle: fetch the asset and its `.sigstore`
from the tag's pre-release, then `cosign verify-blob` with the workflow
identity pinned to that tag.

**Pinned inputs.** `TAG` and `ARCH` as arguments (19-20). The seven-arch
allowlist (27). `KIND` binary or ssh (28-32). The certificate identity is
constructed from `TAG`:
`https://github.com/$REPO/.github/workflows/nightly.yml@refs/tags/$TAG` (47),
and the issuer `https://token.actions.githubusercontent.com` (48). `REPO`
defaults to `Azathothas/podbox` and honours `GH_REPO` (26).

**Host assumptions.** `cosign` (23) and `gh` (24), each `exit 2` without.

**Exit contract.** 0 the bundle verifies and names the workflow identity, 1 the
bytes or the identity did not verify, 2 it could not run (15-16, 22-24, 27,
31, 33, 38).

**Timeouts.** `timeout 120` on the download (36) and on the verify (45).

**Cleanup.** `trap 'rm -rf "$WORK"' EXIT INT TERM` (34).

**Controls.** None in the script. The control the entry describes is external:
`TODO/packaging.md:492-494` records "the same check against a binary with one
flipped byte exits 1 on the digest mismatch."

**Owning entries.** `TODO/packaging.md` T-1328 (read in full, lines 449-501;
the `Prove` at 480, the `Done` at 489-500). `TODO/podssh.md` T-14xx (read
lines 150-175; line 163 cites `sh scripts/verify-release.sh v0.1.0-beta.10
x86_64 ssh` reporting `Verified OK`). `README.md:102-103` documents both
spellings. `scripts/README.md:20` lists it.

**Verdict: RUST-TOOL.**

- **Crate.** New `crates/podbox-verify`, binary `podbox-verify`. The repository
  has ten `crates/podbox-*` crates; this is the eleventh and the only one whose
  subject is the release rather than the runtime. A separate crate is right
  because the dependency set (`cosign` invocation, `gh` invocation) does not
  belong to any runtime crate, and `TODO/RULES.md` section 7 keeps third-party
  surface explicit.
- **CLI surface.** `podbox-verify TAG ARCH [binary|ssh]`, matching the
  documented command at `README.md:102-103` so the documentation stays true
  after the move. It must accept `GH_REPO` from the environment (26).
- **The `exit 2` mapping.** The script uses three codes. A Rust binary has
  `ExitCode` and may use three values, so the mapping is direct: 0 verified,
  1 verification failed, 2 could not run. Emit a usage error at 2 to match the
  current behavior at line 22.
- **Missing tool handling.** `scripts/zig-ar.sh` shows the paired pattern for a
  host dependency; for `verify-release` the equivalent is to exec `cosign` and
  `gh` by name and report their absence at exit 2 with the tool named, which is
  what lines 23-24 do today.
- **The identity stays constructed, not stored.** Line 47 builds the identity
  from `TAG`. A Rust rewrite must not write the identity into a constant, because
  `TODO/packaging.md:494-497` records that the review already corrected it from
  `@refs/heads/main` to `@refs/tags/<tag>` and that a wrong constant "would have
  failed closed on honest artefacts".

**Plant.** The plant is the entry's own: run the verifier against an asset with
one flipped byte and assert exit 1 with the digest mismatch named, then against
an absent asset and assert exit 2. `TODO/packaging.md:492-494` records the first
as already performed on `v0.1.0-beta.6`. The plant script is
`scripts/plant.sh`, and it is Group 5's; the new case belongs there, which is
outside this audit's scope to write.

**Exact command.** `cargo test -p podbox-verify` and, for the live proof,
`podbox-verify v0.1.0-beta.10 x86_64 ssh`. No feature flags. The live leg needs
`cosign` and `gh` on PATH and the network.

**Result file.** The script writes none. It prints to stdout. The saved proof
is `experiments/results/ssh-release-beta10.txt`, cited at `TODO/podssh.md:164`.
Keep it as the historical reading of that tag.

## Group 6 — experiments/zig-ar.sh

**What it measures.** Nothing. Six lines: `exec zig ar "$@"`. The header
states the reason at lines 2-5: "cc-rs takes a PROGRAM. Paired with it, so an
archive is built by the toolchain that built the objects rather than by whatever
`ar` the host happens to carry."

**Pinned inputs.** None.

**Host assumptions.** `zig` on PATH.

**Exit code contract.** None of its own. It inherits `zig ar`'s status through
`exec`. It never returns 2.

**Controls.** None. There is nothing to control.

**Owning entry.** None directly. Its configuration home is `.cargo/config.toml`,
where I counted 8 references: `AR_x86_64_unknown_linux_musl` (line 45),
`AR_aarch64_unknown_linux_musl` (47), `AR_riscv64gc_unknown_linux_musl` (49),
`AR_loongarch64_unknown_linux_musl` (51), `AR_armv7_unknown_linux_musleabihf`
(53), `AR_i686_unknown_linux_musl` (55), `AR_powerpc64le_unknown_linux_musl`
(59), `AR_s390x_unknown_linux_gnu` (64), all with `relative = true`.

**Verdict: KEEP-SHELL.** The host dependency is `zig` itself, named in
`.cargo/config.toml` as the archiver for eight targets. Cargo's `[target.*] ar`
key takes a program path, not a crate, so a Rust binary cannot occupy this slot
without a build-time shim that is strictly more machinery than six lines of
shell. `scripts/zig-cc.sh` is Group 1 and is the same pattern for the compiler.
Together they are the pair the header names.

**Contradiction found.** `TODO/gate.md:290` lists `100-interpose-symbols.sh`
as `known-absent`. I have not settled what that column means. See finding 3.

## Group 6 findings

### Contradictions between scripts, entries, and current source

1. **`370-windows-guest.sh:205` writes a result file named for a different
   script.** `cp "$REPORT" "$REPO/experiments/results/windows-364.txt"`. Number
   364 belongs to `364-qol.sh`, which is in Group 8 per
   `refactor/00-orientation/assignment-map.md:105`. The file exists on disk with
   a 2026-09-27 reading. A repository-wide grep finds it named only by the script
   and by `docs/history/session-2026-09-25-to-27.md:155`. `TODO/milestones.md`
   T-1112 names the script at line 868 and the result file nowhere. A reader who
   follows the record cannot find this file.

2. **`TODO/image.md:1884` and `:1894` record 24 store tests; the tree has 29.**
   The entry says "taken once by every test in it (24 of 24 by audit)" and the
   saved result says `test functions: 24, acquisitions: 24`
   (`experiments/results/store-contention-prove.txt:755`). Running the script's
   own awk and grep against the current `crates/podbox-image/src/store.rs`
   returns 29 and 29. The balance still holds, so this is a stale figure rather
   than a missing mutex. The script is right to re-derive it every run and
   `TODO/RULES.md` section 4 says counts are derived, not edited.

3. **`TODO/gate.md:290` marks a row `known-absent` that a `Prove` at line 827
   names.** Both lines are inside T-1210. I read the row and the `Prove`. I
   could not determine the coverage table's column semantics from the line
   alone. What would settle it: `TODO/gate.md` section 3, the table's own
   definition. Not read.

4. **`experiments/README.md:21` states the rule `350-tool-live.sh` breaks.**
   "Later scripts test podbox behavior. Their owning task gives the
   acceptance." The script measures `text-tool` and `wsl-toolkit`, which
   `AGENTS.md` names as the host route, and no TODO entry owns it.

5. **`TODO/image.md:507` records an `ENG_NETWORK` knob in
   `experiments/lib/engine.sh`** that `180-registry-fixture.sh:388` sets.
   I did not read `experiments/lib/engine.sh`, so I cannot confirm the knob
   exists with the recorded shape (`none` only, unset by default, anything else
   refused).

6. **`experiments/results/pull-ceiling-372.txt` carries `rustup` auto-install
   output in its conditions block** (lines 3-8 of the file). Not a measurement
   defect; a record-hygiene one. No other Group 6 result I read has it.

### Dead scripts and scripts with no owning entry

- **`experiments/350-tool-live.sh`** has no owning TODO entry. See the verdict
  above. Its only other mention is in a superseded history file.
- **`scripts/zig-ar.sh`** has no owning TODO entry, but this is correct: it is
  build configuration referenced 8 times from `.cargo/config.toml`, not a
  measurement. It needs no entry because `experiments/README.md:3` scopes
  "Each numbered script" and this file is not numbered.

### Entries whose acceptance command is a script with no automated test

| Entry | Acceptance command | Automated test behind it |
| --- | --- | --- |
| T-1111 (`TODO/milestones.md:824`) | `./experiments/152-nix-acceptance.sh` | None. The seven rows drive a real nix build over the network. |
| T-1112 (`TODO/milestones.md:867-872`) | `sh experiments/370-windows-guest.sh` | Clause 1 only, and that clause is `cargo test -p podbox-windows --lib`. Clauses 7 and 8 have no automated test and need a licensed image. |
| T-0206 (`TODO/image.md:483`) | `./experiments/180-registry-fixture.sh` | None. The fixture is a live `zot` server. |
| T-0100 (`TODO/probe.md:819`) | `./experiments/290-microvm.sh` | None. The subject is another kernel's answers. |
| T-1310 (`TODO/image.md:1881`) | `./experiments/326-store-contention-prove.sh` | Clause 4 can become one (plan above). Clauses 1-3 measure suite flakiness over 10 runs. |
| T-1342 (`TODO/image.md:2234`) | `372` plus `cargo test -p podbox-image` | The mid-stream cap and pre-flight are unit-pinned already per the entry; the live `ulimit` leg is not. |
| T-1335 (`TODO/supervise.md:758`) | `./experiments/351-signal-forward.sh` | Four unit tests named at 794-797; the live signal delivery is not one. |
| T-0503 (`TODO/enter.md:170`), T-1317 (`TODO/enter.md:515`) | `experiments/356-no-chroot-rung.sh` | Unit guards named at `TODO/enter.md:677-679`; the live loader and memfd families are not. |
| T-1210 (`TODO/gate.md:827`) | `./experiments/100-interpose-symbols.sh` | None. It compiles and reads a third-party `.so`. |
| T-1328 (`TODO/packaging.md:480`) | `sh scripts/verify-release.sh <tag> <arch>` | None today. The plant is a flipped byte, run manually. |

## Group 6 — entries read

Each entry was read in full at the lines given, not at the grep line.

| Entry | File | Lines | Why it was read |
| --- | --- | --- | --- |
| T-0111 | `TODO/probe.md` | 594-703 | Owns `170-probe-cache.sh`; the `Prove` at 657 and the reading at 664-676. |
| T-0100 | `TODO/probe.md` | 760-835 | Owns `290-microvm.sh`; the `Approach` at 798, the `Prove` at 819. |
| T-0206 | `TODO/image.md` | 430-558 | Owns `180-registry-fixture.sh`; the `Prove` at 483, the `Done` at 485-522. |
| T-1310 | `TODO/image.md` | 1767-1902 | Owns `326-store-contention-prove.sh`; the `Prove` at 1881, the audit figure at 1894. |
| T-1342 | `TODO/image.md` | 2194-2248 | Owns `372-pull-ceiling.sh`; the `Approach` at 2224, the `Done` at 2237-2244. |
| T-1111 | `TODO/milestones.md` | 726-845 | Owns `152-nix-acceptance.sh`; the `Prove` at 824, the seven-row table at 836-844. |
| T-1112 | `TODO/milestones.md` | 848-894 | Owns `370-windows-guest.sh`; the `Prove` at 867-872, the `Partial` at 874-894. |
| T-1335 | `TODO/supervise.md` | 725-802 | Owns `351-signal-forward.sh`; the `Prove` at 758, the unit names at 794-797. |
| T-1421 | `TODO/supervise.md` | 804-848 | Cites `356-no-chroot-rung.sh` clause 16 at 838-841. |
| T-1328 | `TODO/packaging.md` | 449-501 | Owns `verify-release.sh`; the `Prove` at 480, the identity correction at 494-497. |
| T-1210 | `TODO/gate.md` | 786-878 | Owns the `100-` engine conversion; the `Prove` at 827, the `known-absent` row at 290. |
| T-1211 | `TODO/gate.md` | 881-910 | Owns the `170-` engine conversion; the `Prove` at 906. |
| T-0703 | `TODO/interpose.md` | 283-402 | Names `100-` check D at 332 and its exclusion rule at 375. |
| T-0503 | `TODO/enter.md` | 170-280 | Owns `356-` clause 3; the `Done` at 253-278. |
| T-1317 | `TODO/enter.md` | 515-680 | Owns `356-` clauses 4-8; the `Done` at 627-679. |
| T-1411 | `TODO/enter.md` | 984-1000 | Owns `356-` clause 9. |
| SSH packaging entry | `TODO/podssh.md` | 150-175 | Cites `verify-release.sh` at 163. |

### Saved result files read

`experiments/results/interpose-symbols.txt` (arm A 1/7, B 7/7, C, D 60 reached
and 5 not defined), `registry-fixture.txt` (44 lines, 21 clauses green),
`store-contention-prove.txt` (758 lines, 10 of 10 green, audit 24 vs 24),
`microvm.txt` (VM kcmp ESRCH, Landlock ABI 6, rung namespace),
`probe-cache.txt` (boot ids equal, `cache_served_host no`),
`pull-ceiling-372.txt` (rc 125, 28232655 bytes, no partials),
`signal-forward.txt` (head: term-rc=143), `tool-live.txt` (head: Windows host,
`text-tool` round trip), `windows-364.txt` (head: 2026-09-27, kvm absent,
`count 52`).

Not read: `experiments/results/nix-acceptance.txt`,
`nix-acceptance-transcript.txt`, and `no-chroot-rung.txt`. What would settle
their agreement with their entries: reading each file and comparing its verdict
rows to `TODO/milestones.md:836-844` and to the clause list in
`TODO/enter.md:652-672` respectively.
