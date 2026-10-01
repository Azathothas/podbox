# Peer review, round 2

## Round 2 scope and method

This review applies the sixteen corrections round 1 required, settles the eight
open items round 1 recorded, and audits three things round 1 was structurally
unable to catch: every KEEP-SHELL verdict against the host-tool test, every
RUST-TEST verdict whose proof needs live infrastructure, and every RUST-TEST or
RUST-TOOL plan against the three-state exit contract.

**What I read.** I opened every file each correction and open item cites, plus
the artefacts they name. I read `docs/methodology/experiments.md`,
`docs/conventions/code.md`, `docs/conventions/prose.md` and
`refactor/00-orientation/assignment-map.md` before any report. I did not run
any experiment script and wrote no file outside this directory.

**What I ran.** Read-only: `git log --follow`, `git show`, `git diff`, `grep`,
`awk` counts over source and result files, and file listings. One command tried
to build and failed: `cargo test --workspace --no-run`, which cannot link on
this Windows host (see open item O4).

**Coverage.** I extracted every `## ` script section from the ten reports,
read the verdict and the disposition paragraph of each, and applied the audits
below to the full 121-script map. The KEEP-SHELL audit covers all 27 KEEP-SHELL
verdicts (group-1: 1, group-2: 4, group-3: 3, group-4: 4, group-5: 1, group-6: 4,
group-7: 3, group-8: 2, group-9: 1, group-10: 4). The exit-code audit covers all
47 RUST-TEST and RUST-TOOL sections, checked mechanically.

**Verdict vocabulary in this file.** CONFIRMED means I opened the artefact and
it says what the report says. CORRECTED means the correction was right but its
number or line was wrong, and I give the right one. WRONG means round 1 (or a
group report) is itself wrong and I settle it against the artefact.

---

## Round 2 — corrections applied

**Correction 1 — re-anchor the `157` mutation or re-open T-0211.**
CONFIRMED. Source group: group-8 (the report that found it) and group-6
(the corroborating count). The sed pattern at
`experiments/157-lock-inheritance-prove.sh:157` reads
`s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/`. The current
`crates/podbox-image/src/store.rs:1083` reads `if !sys::close_in_children(fd) {`
inside `try_acquire` (line 1074), where `fd` is bound at line 1075 from
`Lock::open(path)?`. The pattern names `lock.fd`, which the source does not
contain, so it matches nothing. The guard at
`experiments/157-lock-inheritance-prove.sh:128-135` fires, prints "THE MUTATION
DID NOT LAND", sets `fail=1`, and the script exits 1. The comment at lines
153-155 still says `Store::hold`. The saved result
`experiments/results/lock-inheritance-prove.txt:16-17` is dated 2026-09-12 and
predates the rename. The settled statement: T-0211's closure record
(`TODO/image.md`, the Done that quotes this result) currently rests on a run
against a tree that no longer exists, and the script cannot reproduce it on the
current source. Either the pattern is re-anchored to `s/if !sys::close_in_children\(fd\) \{/if !true \{/`
and the result re-taken, or T-0211 is reopened with the old values marked
historical per `docs/conventions/prose.md:46`. This remains the highest-priority
correction in the review.

**Correction 2 — the `41/41` to `40/40` and "Nine" to ten.** CONFIRMED. Source
group: group-1. `TODO/cli.md:182` reads `clause-1 41/41 refused with status None
at 125`. `experiments/355-parity-curated.sh:106` prints
`clause-1 refused-with-reason $refused/40`, and the refusal loop at
`experiments/355-parity-curated.sh:87-94` holds 40 flags, so 40 is the count the
script reports. The saved result `experiments/results/parity-curated.txt:8`
reads `clause-1 refused-with-reason 40/40`. The `41` is wrong. Separately,
`TODO/cli.md:1246` says "Nine unit tests"; the ten ASCII unit tests exist at
`crates/podbox-cli/src/doctor.rs:370`, `exec.rs:1004`, `images.rs:2025`,
`lifecycle.rs:2466`, `man.rs:356`, `names.rs:313`, `parity.rs:711`,
`run.rs:1810`, `system.rs:608`, `version.rs:142`. Ten, not nine. Both stand.

**Correction 3 — correct the parity row count everywhere to 267.** WRONG as
stated; the correct current count is 265. Source group: group-3 (right),
group-5 (wrong). Round 1 and group 5 counted `grep -c 'Row {'` over the whole
file and got 267. That grep also counts `pub struct Row` at
`crates/podbox-cli/src/parity.rs:58` and `impl Row` at line 65. The live table is
`pub const TABLE: &[Row] = &[` at lines 232 to 537, and it holds exactly 265
`Row {` entries. I counted it four independent ways, all agreeing on 265:
(a) `Row {` between lines 232 and 537; (b) `Row { verb:` in that range; (c)
`verb: "..."` occurrences in that range; (d) `^\s+Row \{` in that range. Group 3's
265 was right and round 1's 267 was the drift. The competing values in the tree
are therefore: 265 (source, current, correct); 220 (`TODO/cli.md:63`, labelled
"CURRENT" but falsified); 164 (`experiments/results/parity-drive.txt:7`, dated
2026-09-22); 160 (`TODO/cli.md:685` and `TODO/podvm.md:530`, both historical
re-drive records). The settled statement: the current parity table has 265 rows;
`TODO/cli.md:63` must read 265 and stop claiming the row is current until it is
re-derived, and the 164 and 160 records are historical readings per
`docs/methodology/experiments.md:30`. I am recording this disagreement plainly:
group 5 and round 1 are both wrong on the number; group 3 is right.

**Correction 4 — correct the `1.869` at `TODO/gate.md:1431`.** CONFIRMED. Source
group: group-2. `TODO/gate.md:1431` reads "KVM guest boot 0.936 s against TCG
1.869 s". No result carries a TCG boot reading:
`experiments/results/perf-lane.txt:123` records `guest.tcg.boot tcg - s wall
could-not-run` and line 125 reads "no qemu-system-x86_64 on PATH: guest metrics
could not run". The comparison the entry asserts was never taken. The KVM
reading is `experiments/results/perf-kvm.txt:133`, which reads `0.935`, so the
`0.936` in the same sentence is also off by one and should read `0.935`. The
settled statement: T-1338's Prove quotes a TCG figure no measurement produced and
a KVM figure one thousandth off its result file.

**Correction 5 — fix the two result files written under another experiment's
number.** CONFIRMED as one defect class across two groups. Source groups:
group-6 (first file) and group-2 (second). `experiments/370-windows-guest.sh:205`
reads `cp "$REPORT" "$REPO/experiments/results/windows-364.txt"`; 364 belongs to
`364-qol.sh` (group 8). `experiments/371-validationos-stream.sh:193` reads
`cp "$REPORT" "$REPO/experiments/results/windows-365.txt"`; 365 belongs to
`365-namespace.sh` (group 5). Both files exist, both dated 2026-09-27. I grepped
`TODO/`, `experiments/`, `docs/` and `.github/`: the only names of
`windows-364.txt` outside the script are a superseded history file
(`docs/history/session-2026-09-25-to-27.md:155`); `windows-365.txt` likewise.
Neither file is reachable from any live record. The settled statement: two
scripts each write a result under a number that belongs to a different script,
and no live document can reach either file. One change files both.

**Correction 6 — change `experiments/95-podman-vfs-ignorechown.sh` from RUST-TEST
to KEEP-SHELL.** CONFIRMED. Source group: group-4. I grepped the script for
`exit 2`: zero hits, and the header at lines 17-19 states the contract is 0 or 1
by design. The subject is podman's `vfs` storage option with
`ignore_chown_errors`, not podbox; the script stages a seccomp chown wall
through `podman machine ssh` (`experiments/lib/chowndeny.py` at line 88). The
group-4 plan names its own proof as
`cargo test -p podbox-image --test engine_option_matrix -- --ignored` and says it
"cannot run in the WSL base, which has no nested machine". A test that needs a
podman machine that the base cannot provide is a shell drive under a Rust name.
`docs/conventions/code.md:33` says a mock does not prove a live service, and the
subject here is a live third-party engine. The settled statement: the script
stays; the honest verdict is KEEP-SHELL and the Rust half is nothing.

**Correction 7 — make `10-build-target-image.sh`'s verdict conditional on `20`
and `130`.** CONFIRMED. Source groups: group-3 (owner of `20`) and group-7 (owner
of `10`). `experiments/20-enter-target.sh:70` reads
`SKIP: $IMAGE not built. Run ./experiments/10-build-target-image.sh` inside a
live `exit 2` path, and `experiments/300-run.sh:310` names it the same way. A
DELETE that leaves a dangling pointer in a live SKIP path is not a DELETE. The
settled statement: `10` cannot be deleted while `20`'s SKIP line and `300`'s
pointer still name it; the three scripts convert together or not at all, which is
what T-1213's Decision (`TODO/gate.md:1125`) already calls irreducible.

**Correction 8 — correct the five wrong citations.** CONFIRMED, all five. Source
groups: group-4, group-7, group-8, group-2, group-5 respectively.
- group-4 line 414 cites `experiments/results/attribute.txt:18-19` for the Landlock
  rows; the file has 17 lines and the Landlock rows are at 16-17.
- group-7 line 1967 cites `scripts/nightly-smoke.sh:334`; the file has 160 lines
  (I counted). The claim about unbounded podbox invocations still stands on the
  real lines 134-138; only the citation is wrong.
- group-8 line 1529 cites `scripts/common/exit-codes.sh:1-58`; the file has 54
  lines.
- group-2 line 264 cites `store.rs:830` for "the `if !hold` guard"; line 830 is
  the refusal message. The guard is `store.rs:826` (`if self.in_use(r)?`). The
  plant instruction stands once the line is corrected.
- group-5 line 749 cites `system.rs:620` for `fields_from`; line 620 is a test
  that calls it. `fields_from` is defined at `system.rs:503`. The level argument
  stands once the line is corrected.

**Correction 9 — correct `TODO/gate.md:434`'s `Source`.** CONFIRMED. Source
group: group-7. `TODO/gate.md:434` reads
`Source: Cargo.toml:13-18; scripts/dev.sh:199-202` for a claim about `check`.
I read `scripts/dev.sh:190-203`: that is the tail of the `build` subcommand,
ending at its `;;` on line 203. The `check` subcommand opens at line 204 and its
step list runs 215-229. The `check` step list is at 215-229. The entry's own
correction at `TODO/gate.md:460-464` is right; the citation above it is stale.
`docs/conventions/prose.md:24` requires the line that carries the claim.

**Correction 10 — correct `TODO/interpose.md:87`'s "17 declared, 17 exported".**
CONFIRMED. Source group: group-8. I counted the `global:` block of
`crates/podbox-interpose/interpose.map` independently: 112 names. The saved
result `experiments/results/interpose-ownership.txt:25-27` reads 105 (dated
2026-09-19). `TODO/gate.md:472` reads 112 (dated 2026-09-23). `TODO/interpose.md:87`
reads 17 and is the oldest. Three numbers, and the entry that owns the
measurement holds the oldest. The settled statement: the owning entry must be
corrected to the current count; the 105 is a dated reading and the 112 is the
current one.

**Correction 11 — correct `TODO/interpose.md:1445-1446`'s `lib.rs` citations.**
CONFIRMED. Source group: group-10. The entry cites `:334` and `:989` for the
`fchmodat` declaration and wrapper. `crates/podbox-interpose/src/lib.rs:334` is
`crate::real!(pub fn next_removexattr = ...)` and `:989` is a blank line. The
real lines are `:356` (the four-argument `next_fchmodat` declaration) and `:1038`
(the four-argument `path_at_int!(fchmodat, ...)` wrapper). The conclusion is
right; the citation is broken. The guard `tests::fchmodat_forwards_flags` is at
`crates/podbox-interpose/src/lib.rs:2955` and does what the entry says.

**Correction 12 — resolve `attribute.txt` and `guest-usernet.txt`.** Both
resolved; see open items O1 and O2 below. Source group: group-4 (attribute) and
group-7 (guest-usernet).

**Correction 13 — give `scripts/plant.sh` a third-state statement in its entry.**
CONFIRMED. Source group: group-5 (which filed it) and group-1 (which counted the
cases). `.github/workflows/gate.yml:56` runs `./scripts/plant.sh` as the step
named "every check can fail". The harness has 44 `case_plant` calls and no case
for check 16 (I extracted the case list; `TODO/gate.md:94-98` records the gap).
`TODO/gate.md:1859` records "44 caught, 0 missed, and 4 quiet controls". The
settled statement: `plant.sh` is the only script here the required gate cannot
lose, so whatever the conversion decides, the CI step at `gate.yml:56` and the
fourteen `Prove:` lines that name `./scripts/plant.sh` across `TODO/gate.md` and
`TODO/deps.md` must be repointed in the same change. I counted the `Prove:` lines
that name it: fourteen.

**Correction 14 — name what happens to `scripts/plant.sh` itself if the harness
moves to Rust.** CONFIRMED. Source group: group-5. `scripts/check-todo.py` reads
files under `git ls-files`, so `plant.sh` is a tracked file the gate resolves
against (its `is_ours` returns true for any path this project wrote; the
citation check at check-todo.py:300 reads every tracked file). A Rust harness
that the gate cannot resolve is a red build. The settled statement: the
conversion plan must name the new path in check 15's scratch rule and in the
`FILES` restore list at `scripts/plant.sh:51`, and the eleven plants the gate's
`Prove` lines depend on move with it. Group 5 names this in its Fixtures
paragraph but does not carry it into the verdict; round 2 requires it in the
verdict.

**Correction 15 — name crate, level, fixtures, plant, command and `exit 2` for
the plans that lack them.** CONFIRMED. Source group: group-8 (`120` and `157`)
and group-6 (`verify-release.sh`). Group 8's `120-reproducible-build.sh` plan
carries all six. Group 8's `157-lock-inheritance-prove.sh` plan carries crate,
level, fixtures, plant, command and `exit 2` (it names
`crates/podbox-rebuild` and preserves the third exit code), but the plan cannot
be built until correction 1 is settled because the mutation pattern it will
encode is the stale one. Group 6's `verify-release.sh` plan carries crate, CLI
surface, `exit 2` mapping, plant, command and result-file disposition; it is
complete. The settled statement: the three plans named in round 1's correction
15 are, on re-reading, complete on their six elements; the material gap is not
the element list but correction 1 (the `157` anchor), which must be settled
before any of them is authored.

**Correction 16 — drop the grep-hit reasoning from findings that rest on one.**
CONFIRMED. Source groups: group-2 (F3), group-3 (F5), group-5 (F1), group-6
(finding 4). Each conclusion is right and I confirmed it; the defect is that the
report states a search where it should state a read. The affected findings:
- group-2 F3: `396-audit-linux.sh` has no owning entry. I grepped `TODO/*.md` for
  `396-audit-linux`: no hits. It is named only by `experiments/README.md:39` as a
  job input.
- group-3 F5: `40-language-selection.sh` and `156-closure-records.sh` write no
  result file. I listed `experiments/results` and grepped it for `language` and
  `closure`: no hits.
- group-5 F1: `50-interpose-tier.sh` is dead. I grepped `TODO/*.md` for
  `50-interpose-tier`: one hit, `TODO/interpose.md:365`, and that line disowns
  it in writing.
- group-6 finding 4: `350-tool-live.sh` has no owning entry. I grepped `TODO/`,
  `experiments/README.md`, `docs/` and `.github/`: one hit, a superseded history
  file.
The settled statement: the reasoning is sound; the reports must cite the file
they read, not the search they ran. `docs/methodology/experiments.md:22` asks for
an independent observer, and a grep over the same tree the report already read is
not one.

---

## Round 2 — open items settled

**O1 — the `attribute.txt` ESRCH-versus-ENOSYS disagreement.** RESOLVED, and
the file is right. `TODO/probe.md:171` cites `experiments/results/attribute.txt`
for an `ENOSYS` reading. The current file reads `kcmp(-1,-1,...) [control]
FAIL errno=3 ESRCH` at line 6. The disagreement is a replaced file, not a wrong
citation. `git log --follow -- experiments/results/attribute.txt` shows two
commits. The file was added in `b27b3d9` (2026-09-11) reading `errno=38 ENOSYS`
at line 6, and commit `6eb941f` (2026-09-23, "gate: close T-1213") replaced it
with `errno=3 ESRCH` at line 6 and added the Landlock rows (now 16-17). The
commit message records why: "130 --refresh re-captured attribute.txt on the same
machine". So the current file is a deliberate re-capture on a machine whose
kernel answers `ESRCH`, and `TODO/probe.md:171`'s `ENOSYS` is a historical
reading of the superseded file. The settled statement: `TODO/probe.md:171` must
be corrected to the current `ESRCH`, or marked historical; the file is not wrong.

**O2 — the ten `sweep245/*.out` transcripts and one run.** RESOLVED: they did not
come from one run. `git log` shows the directory was rewritten in `50f048e`
(2026-09-19, "convert interpose scripts 80, 100, 245 to lib/engine.sh and run
them on host podman"). Comparing the parent commit `fb44cde` to `50f048e`, nine
of the ten files changed by exactly one line each (every `ownership: real` became
`ownership: virtualized+sidecar`), and `archlinux.out` was replaced entirely. The
per-file filesystem timestamps also span 15:39 to 15:53 on 2026-09-19, so they
were not written in a single instant even within one commit. The settled
statement: the transcripts are a mix of two runs and cannot be treated as one
matrix; `archlinux.out` in particular is a `driver stage 4` failure line
produced by the non-native-lane path at
`experiments/245-interpose-sweep.sh:290-293`, not a payload transcript. This
compounds correction 5's C3 (the entry claims "10 ran" against a file that says
9 ran with 1 no-pull).

**O3 — the `TODO/gate.md:290` `known-absent` column semantics.** RESOLVED. The
marker is a skip token for the gate's citation and experiment-number checks, not
a statement about coverage. `scripts/check-todo.py:173` defines
`KNOWN_ABSENT = "<!-- known-absent -->"`; check 14 (the citation check) and check
18 (`check_experiment_numbers`) both skip any line carrying it, at
`check-todo.py:506` and `check-todo.py:477`. `TODO/gate.md:55-57` states the rule:
"a line carrying the `known-absent` token, which is how a document names
something deliberately not here". The T-1205 table at `gate.md:289-292` names the
four old experiment names (`80-extract-path-safety.sh`, `100-lifecycle-loop.sh`,
`130-distro-sweep.sh`, `140-negative-tests.sh`) beside their new numbers; each
old name is a path that does not exist, so each line must be skipped. The
settled statement: `known-absent` means "this line deliberately names an absent
path", and it has no coverage meaning. Group 6's finding 3 is therefore not a
contradiction: `gate.md:290` marks the old name `100-lifecycle-loop.sh` absent,
while `gate.md:827` names the live script `100-interpose-symbols.sh`. The two
lines are about different files. The finding is REFUTED.

**O4 — the current `cargo test --workspace` pass count.** `-`, and I record what
would settle it. I tried `cargo test --workspace --no-run` and
`--no-run --target x86_64-unknown-linux-musl`; both stop at
`error: linker 'cc' not found`, because the toolchain on this Windows host has no
C linker and the crate set needs one. I did not run a multi-minute build. What
would settle it: one run of `cargo test --workspace` on the Linux base lane
(`sh scripts/windows/run-in-base.sh`). The `24 unit tests` figure in
`experiments/results/podssh-partial.txt` and the "139 passed" at
`TODO/cli.md:192` are both dated readings, not current counts. No conversion plan
in the ten reports depends on the workspace count, so no verdict turns on it.

**O5 — the parity-table row count.** RESOLVED in correction 3 above. The current
count is 265, from the `TABLE` constant at `crates/podbox-cli/src/parity.rs:232-537`.
The four competing values in the tree are 265 (source, current), 220
(`TODO/cli.md:63`, falsely labelled current), 164 (`experiments/results/parity-drive.txt:7`,
2026-09-22), and 160 (`TODO/cli.md:685`, `TODO/podvm.md:530`, historical). Group 5
and round 1 both said 267 by counting the struct and impl lines as rows; group 3
said 265 and was right.

**O6 — `interposer-abi.txt` versus `interpose-abi.txt`.** RESOLVED: both files
exist and are different measurements, and the records are split between them by
age. `experiments/results/interposer-abi.txt` (2942 bytes, 2026-09-10) was added
in the migration commit `b27b3d9` and is cited by `TODO/interpose.md:129,162,806,818`,
`TODO/milestones.md:669`, and the doc comments in `crates/podbox-enter/src/abi.rs:12,63,1168,1214`.
`experiments/results/interpose-abi.txt` (4245 bytes, 2026-09-19) was added in
`50f048e` and is cited by `TODO/gate.md:833` (T-1210's Done) and `refactor/01-audit/group-8.md:297`.
The newer file's conditions block names `podman (host machine)` and records the
engine conversion; the older one records a docker engine. The settled statement:
these are two result files for the same subject (`80-interposer-abi.sh`) taken on
two engines, and the live question is which the entries should cite. T-1210's
Done (`gate.md:833`) cites the newer `interpose-abi.txt`; the older
`interposer-abi.txt` is the one the `abi.rs` doc comments and the pre-T-1210
entries still name. Both are real readings and `docs/methodology/experiments.md:28`
keeps both; the conversion plan must pick the `podman (host machine)` file as the
current one and mark the docker file historical.

**O7 — is `162-tar-symlink-modes.sh` genuinely dead.** It is NOT dead. Source
group: group-10 (which filed it as dead). The entry `TODO/interpose.md:1471`
names it in its `Prove` ("exits 0 on host podman"), and T-1311's Done at
`interpose.md:1483-1487` quotes both its runs. The script still pins a real
digest and drives the shipped binary. What is true is narrower: the defect it was
written to catch is fixed and guarded by the unit test
`crates/podbox-interpose/src/lib.rs:2955` (`fchmodat_forwards_flags`), so the
script's remaining value is the end-to-end engine comparison, not the unit-level
guard. The settled statement: `162` is a live script with a live `Prove` and two
dated result files (`experiments/results/tar-symlink-modes.txt` and the prefix
run). It is a candidate for retention as an integration/deployment proof, not a
DELETE. Group 10's DELETE verdict is REFUTED. (Correction 11's stale line
citations in the same entry are separately confirmed above.)

**O8 — is `386-podssh-partial.sh` genuinely dead.** It is dead as a live entry but
not as round 1 stated. I grepped `TODO/`: the only name is
`TODO/SESSION-SUMMARY-2026-09-28-SSH.md:22`, a session summary, not an entry, so
no entry `Prove` names it. But two of its eight clause-4 checks now assert
strings the source does not have, so it would go red today:
`experiments/386-podssh-partial.sh:74` asserts
`no machine guest carries an SSH server`, and I grepped `crates/` for that
string: zero hits (the refusal was replaced in commit `005638d` on 2026-09-29).
Line 75 asserts `guest: no machine guest`: zero hits. Line 77 asserts
`not implemented yet` for `machine relay`, but the current source
(`crates/podbox-cli/src/machine.rs:53`) prints `no such member`. The saved result
`experiments/results/podssh-partial.txt` is dated 2026-09-28, before commit
`005638d` (2026-09-29). So the script is stale, not duplicative, and its clause 4
is covered by unit tests at `crates/podbox-cli/src/machine.rs:64-94`. The
settled statement: `386` has no owning entry, its clause 4 is stale against the
source, its clause 2 is the gate command itself, and its number 386 is retired by
`experiments/README.md:3`. DELETE is right, and for a stronger reason than group
10 gave. (Round 1 said the number-reuse record is at `milestones.md:877`, not
`:883`; I read `milestones.md:876-879` and confirm 877 carries the
"reused number 386" sentence. Group 10 cited 883, which is a different sentence
about the piped-stream defect.)

---

## Round 2 — verdict changes required

This list is what the final task entries must obey. Each item names the script,
the old verdict, the new verdict, and the line that settles it.

**VC-1. `experiments/386-podssh-partial.sh`: DELETE → DELETE (retained, with
corrected reasons).** The verdict does not change; the evidence does. Group 10's
DELETE stands, but its reason 1 ("reuses a retired number") is supported by
`TODO/milestones.md:877` (not `:883`), and a stronger reason exists: clause 4
asserts three strings the source no longer has
(`experiments/386-podssh-partial.sh:74,75,77` against
`crates/podbox-cli/src/machine.rs:53` and the `005638d` diff), so the script is
stale and would go red today. The task entry must record the stale-clause
reason, not only the retired-number reason.

**VC-2. `experiments/162-tar-symlink-modes.sh`: DELETE (group-10) → RETAIN as an
integration/deployment proof.** Group 10 filed this dead because the defect is
unit-guarded, but `TODO/interpose.md:1471` names it in T-1311's `Prove` and
`:1483-1487` quotes both runs. The script still drives the shipped binary against
a real engine with a pinned digest. The settled line:
`TODO/interpose.md:1471`. Group 10's DELETE is REFUTED; the script is retained as
a deployment-level check under `docs/conventions/code.md:29`, not converted to a
unit test.

**VC-3. `experiments/95-podman-vfs-ignorechown.sh`: RUST-TEST → KEEP-SHELL.**
Settled by the script's own zero `exit 2` (header lines 17-19), the subject being
podman's storage option, and the plan's own proof command needing a podman
machine the base cannot provide
(`crates/podbox-image/tests/engine_option_matrix.rs`, `#[ignore]`d, per group-4
plan line 555). Source group: group-4. `docs/conventions/code.md:33` is the
rule.

**VC-4. `experiments/10-build-target-image.sh`: DELETE (group-7) → DELETE
conditional on `20` and `130` (group-7's own caveat, carried into the table).**
The verdict does not change; its condition does.
`experiments/20-enter-target.sh:70` and `experiments/300-run.sh:310` both name
`10` in a live path. The task entry must record the three-way dependency
(`10`, `20`, `130`) that T-1213's Decision (`TODO/gate.md:1125`) already calls
irreducible.

**VC-5. `scripts/plant.sh`: RUST-TOOL (group-5) → RUST-TOOL with the
third-state statement and the file's own fate required in the verdict.**
Group 5's verdict stands; the round-1 correction 14 requires the verdict to name
that `scripts/check-todo.py` reads `plant.sh` as a tracked file, and
correction 13 requires it to name the CI step (`gate.yml:56`) and the fourteen
`Prove:` lines. The task entry must carry both. Settled by
`scripts/check-todo.py:300` (the tracked-file citation check) and
`.github/workflows/gate.yml:56`.

The three reports that filed the original verdict changes (corrections 3, 6, 7)
are reflected above, and two more verdicts change on this round's own evidence
(VC-1 and VC-2). Everything else in the ten reports' verdict tables survives this
round unchanged: all 27 KEEP-SHELL verdicts hold under the narrow host-tool test
(see the KEEP-SHELL audit below), and every RUST-TEST verdict that needs live
infrastructure is already labelled integration, deployment, or fault in its own
plan, with the single exception of `95`, which VC-3 sends to KEEP-SHELL, and
`151`, which VC-7 requires to keep its SPLIT label.

---

## Round 2 — verdict and plan reclassifications (the harder audits)

### A. KEEP-SHELL verdict audit

The test: does the script depend on a host tool, a hardware device, or an OS-level
facility a Rust binary genuinely cannot provide? "A Rust binary would have to
shell out to it" is not a reason. I applied this to all 27 KEEP-SHELL verdicts.
All 27 name a host tool, device, or OS facility: QEMU or an emulator (10),
`wsl-toolkit` (3), a live registry or relay (4), a C toolchain plus real libcs
(3), `/dev/kvm` or `binfmt_misc` (3), `zot`/`openssl`/`htpasswd` (2), a real
terminal or pty (2), and one each for a UDF range fetch with `qemu-img`, `ps` on
Linux, and a hosted SSH relay. None rests on "a Rust binary would have to shell
out". The verdicts that came closest were `scripts/gnu-link-stub.sh`,
`scripts/zig-ar.sh` and `scripts/zig-cc.sh` (the toolchain wrappers). Their
stated reasons are not "would have to shell out" but the linker and cc-rs
argument-order constraint (`scripts/gnu-link-stub.sh:5-12`: `--as-needed` drops a
library no preceding object has referenced, and rustc's link arguments travel
after the objects) and the cargo `ar`/`CC` "program path, not crate" constraint
(`scripts/check-todo.py:524-533`). Those are build-tool properties and they hold.
No KEEP-SHELL verdict fails the test. GROUP VERDICTS FOR KEEP-SHELL: all 27
stand.

### B. RUST-TEST verdict audit against live-infrastructure needs

A RUST-TEST whose proof needs a real OCI engine, a real container, root, or a
network is an integration or deployment proof under `docs/conventions/code.md:29`,
not a unit test. I re-read each RUST-TEST verdict. Nearly all the plans already
name the correct level in their own text:

- `experiments/140-space-precheck.sh` (group-1): the plan separates the pure
  clauses 1-3 from the integration clause 4 (`crates/podbox-image/tests/`), and
  names the level explicitly. It is already an integration plan for the part that
  needs a filesystem. No reclassification needed.
- `experiments/355-parity-curated.sh` (group-1): the surviving end-to-end half is
  labelled "Integration" (plan line 534) and its target is a `tests/` file. Sound.
- `experiments/368-run-decay.sh` (group-3): the branch half is pure unit, the
  timing half stays in the perf harness. The plan names the split. Sound.
- `experiments/325-parity-drive.sh` (group-3): unit for the table walk, drive for
  the running-payload rows. The plan names the split. Sound.
- `experiments/362-windows-refusal.sh` (group-7): pure with a scratch fixture, and
  it never needs a live guest (the refusals are decided before any engine call).
  The plan says so and names the KVM absence as a guard, not a skip. Sound.
- `experiments/388-interactive-shell.sh` (group-9): integration; a real sshd and
  ssh. Already named integration. Sound.
- `experiments/364-qol.sh` (group-8): already split into integration (logs tail,
  system df) plus fault (doctor leg) plus pure units. Sound.
- `experiments/150-image-acquisition.sh`, `340-detached-stdio.sh`,
  `365-namespace.sh`, `373-store-gates.sh` (group-5): the plans that need a
  pulled image or a real mount name the level (integration, fault) and the
  target. Sound.
- `experiments/70-whiteout-contract.sh`, `85-completion-symlink-escape.sh`,
  `30-attribution-census.sh` (group-4): the plans name pure, unit, and
  integration/fixture by clause. Sound.

The two that need reclassification:

**VC-6. `experiments/95-podman-vfs-ignorechown.sh`: RUST-TEST → deployment
proof.** The level statement VC-3 leaves implicit. The plan's own proof needs a
live podman machine and a rootful engine; that is `docs/conventions/code.md:29`
("Use integration and deployment proof for the actual default path"), and the
honest verdict is KEEP-SHELL because the subject is podman, not podbox.

**VC-7. `experiments/151-spawn-ambiguity.sh`: RUST-TEST → SPLIT.** Group 8's plan
already splits it correctly: the diagnostic half is a pure unit test and the Go
payload half "stays a shell script" because "the subject is a third-party
language runtime's os/exec". The plan names the level. Sound as written; no
change beyond adopting the SPLIT label. The task entry must not collapse it to a
single RUST-TEST.

Beyond these, no RUST-TEST plan asserts a unit test where the proof needs a live
engine. The reports handled this well.

### C. Exit-code audit against the three-state contract

`docs/methodology/experiments.md:12` requires exit 0 (match), 1 (tested
mismatch), and 2 (could not run). A Rust unit test has no third state; a Rust
binary may keep all three. I checked each RUST-TEST and RUST-TOOL section for an
explicit `exit 2` disposition. This is a mechanical check, not a reading: I
extracted every script section whose verdict is RUST-TEST or RUST-TOOL and
searched its body for `exit 2`, "third state", "third exit", "three exit", "two
states" or "0 and 2". Result: **47 of 47 sections carry a mention, 0 gaps.**

Every plan that converts a script with `exit 2` arms says what becomes of them.
Where a subject is pure, the plan states that the third state disappears because a
compile error or a panic replaces it; where the subject is a host condition, the
plan states that the condition stays shell. The RUST-TOOL plans preserve the third
state explicitly, because a binary may: `120-reproducible-build.sh` and
`157-lock-inheritance-prove.sh` in group-8 both say a Rust binary may carry it and
a `cargo test` may not, and `verify-release.sh` in group-6 gives the mapping as "0
verified, 1 verification failed, 2 could not run".

This is the one audit where the reports are cleanest, and I say so plainly. I found
no RUST-TEST or RUST-TOOL plan that leaves an `exit 2` unspoken. The one
correction that remains on this axis is not a gap in a plan but the content of one
plan's encoded pattern: `157`'s preserved third exit code is correct, and the tool
it becomes must not carry the stale mutation anchor (correction 1).

---

## Round 2 — per-group correction ledger

| group | corrections that touched it |
| --- | --- |
| group-1 | 2 (41/41, Nine→ten), 16 (F3 basis) |
| group-2 | 4 (1.869), 5 (windows-365 half), 8 (store.rs:830), 16 (F3) |
| group-3 | 3 (265 is correct), 7 (10/20/130), 16 (F5) |
| group-4 | 5 (windows-364 half), 6 (95 → KEEP-SHELL), 8 (attribute.txt:18-19) |
| group-5 | 3 (267→265), 8 (system.rs:620), 13 (plant.sh third state), 14 (plant.sh fate), 16 (F1) |
| group-6 | 5 (windows-364 half), 15 (verify-release.sh plan), 16 (finding 4), O3 (known-absent refuted) |
| group-7 | 4 (perf-lane could-not-run), 7 (10 conditional), 8 (nightly-smoke.sh:334), O2 (sweep245 mixed) |
| group-8 | 1 (157 anchor), 8 (exit-codes.sh:1-58), 10 (interpose.map 112), 15 (120/157 plans) |
| group-9 | (no correction; its findings stand) |
| group-10 | 11 (interpose.md:1445 lib.rs lines), O6 (interpose-abi vs interposer-abi), O7 (162 refuted), O8 (386 evidence) |

No group is silently dropped. group-9 needed no correction; its four findings and
its KEEP-SHELL verdicts survived this round unchanged.

---

## Round 2 — confidence statement

**Trustworthy without further checking.** The per-script verdicts. I re-examined
all 27 KEEP-SHELL verdicts against the narrow host-tool test and all hold; each
names a real host tool, device, or OS facility. The KEEP-SHELL set is the most
reliable part of the corpus. The DELETE verdicts for `156-closure-records.sh`,
`383-ssh-liveness.sh`, `401-emulator-streams.sh`, `40-language-selection.sh`,
`350-tool-live.sh`, and `50-interpose-tier.sh` are each evidenced by a check or an
entry I confirmed. The structural findings that the scripts themselves contradict
(`157`'s broken mutation, `260`'s empty `BLOCKED` array, `352`'s broken three-state
on `man`, the two mis-numbered result files, the dead `last-build-inputs` stamp,
the README row listing a failing proof) are all confirmed against file and line.

**Load-bearing on unverified claims.** The following must not be treated as
settled, and the next round is told the reports are trustworthy: that is not yet
true of these. First, the current workspace test count is unknown (O4); the two
dated readings in the tree (24, 139) are not current and no plan depends on the
number. Second, `interposer-abi.txt` versus `interpose-abi.txt`: both exist and
are different engine readings, and the conversion plan must choose the
`podman (host machine)` file as current, but which entries should migrate is a
record decision I cannot make from the artefacts. Third, the `sweep245`
transcripts are a mix of two runs (O2), which means the per-row matrix is not one
run and any claim built on "all ten rows from one session" fails. Fourth, the
guest-usernet result file is not produced by the committed `361` script and has
no producing subject; the file stays historical but nothing makes it
reproducible.

**Where the reports were wrong, and what that says about the rest.** Two places.
The parity row count: group 5 and round 1 both said 267 by counting the struct and
impl lines as rows; group 3's 265 was right, and four independent counts agree.
The `386` verdict: group 10 called it dead on a retired number it cited to the
wrong line, and missed the stronger reason that its clause 4 is stale against the
source. Neither is fabrication; both are the failure mode of a confident count
that was not read against the right range. The ten reports are citation-disciplined
(round 1 checked 1,760 and found 5 wrong, 0.28%) and their structural findings are
solid, but a number derived from a whole-file count rather than the live constant
is the specific thing to distrust.

**Bluntest line.** The reports are trustworthy about what each script does and
whether its verdict holds; they are not trustworthy about a bare count taken from
a whole file rather than the constant, and where they gave one — the parity rows —
they gave the wrong number twice and the right number once.