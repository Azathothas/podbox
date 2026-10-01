# Plan: retire the shell script corpus

This is the final plan of a seven-round audit. It retires 121 shell, Python and
PowerShell scripts under `experiments/` and `scripts/` by re-expressing their
measurements as native Rust tests or native Rust tooling.

## 1. What was examined

| round | artefact | what it did |
| --- | --- | --- |
| orientation | `00-orientation/assignment-map.md` | 121 scripts, 29,153 lines, split into ten balanced groups |
| 1 | `01-audit/group-1.md` .. `group-10.md` | ten auditors, one per group, each reading at least ten `TODO/` entries in full |
| 2 | `02-peer-review-1/round-1.md` | 1,760 citations checked, 5 wrong, 16 corrections required |
| 3 | `03-peer-review-2/round-2.md` | corrections applied, 8 open items settled, verdict changes VC-1..VC-7 |
| 4 | `04-meta/meta-1.md` | analysed from the implementation side: 980 test functions, waves, crate architecture |
| 5 | `05-meta-review/meta-review-1.md` | reviewed the meta report: 2 claims corrected, 3 waves rejected, 4 crates endorsed |

The ledger in `06-entries/verdict-ledger.tsv` is the machine-readable result. It
holds one row per script, taken from the group report BODIES, never from their
header tables. Six of ten headers disagree with their own bodies.

⛔ **The ledger is PRE-VC. The table in section 2 is POST-VC.** Three rows change:
`experiments/95-podman-vfs-ignorechown.sh` RUST-TEST → KEEP-SHELL (VC-3),
`experiments/151-spawn-ambiguity.sh` RUST-TEST → SPLIT (VC-7), and
`experiments/162-tar-symlink-modes.sh` DELETE → RETAIN as a deployment proof
(VC-2). A reader who trusts the ledger over section 2 lands three scripts in the
wrong wave.

⛔ **The 121 figure is scoped to top-level files.** It counts `experiments/*` and
`scripts/*` at depth 1 only. `scripts/common/` holds 28 more scripts the gate runs
at `gate.yml:59`, and the whole tree carries 149. A whole-tree `find` returns 167
because it includes `scripts/__pycache__`. The boundary matters: retiring "the
121" must not delete `scripts/common/check-*.sh`, which are the gate's checks,
not measurements.

## 2. The verdict distribution

Counted from the bodies, after VC-1 through VC-7:

| verdict | count | converges on Rust? |
| --- | --- | --- |
| KEEP-SHELL | 29 | no. A host tool, device or OS facility. |
| SPLIT | 36 | partly. Every one has a Rust half. |
| RUST-TOOL | 27 | yes. A Rust binary replaces the script. |
| RUST-TEST | 18 | yes. A Rust test replaces the script. |
| DELETE | 11 | no. The measurement is obsolete. |
| **total** | **121** | 45 converge wholly on a Rust test or tool |

**81 scripts retire (45 RUST-TEST, RUST-TOOL, and SPLIT scripts with a Rust half)
plus 11 deletions. 29 stay in shell. 81 + 29 + 11 = 121.**

The 29 KEEP-SHELL verdicts are the most reliable part of the corpus. Round 2
re-examined all 27 it saw against a narrow test — does the script need a host
tool, a device, or an OS facility a Rust binary genuinely cannot provide? None
rests on "a Rust binary would have to shell out", which is not a reason. They
need QEMU or an emulator, `wsl-toolkit` (3), a live registry or relay, a C
toolchain against real libcs, `/dev/kvm` or `binfmt_misc`, `zot`/`openssl`/
`htpasswd`, and a real pty. The two added by VC are `162` (a deployment proof
over a real engine) and `95` (podman's own storage option, not podbox code).

⛔ **The Round 2 KEEP-SHELL breakdown counted 27 scripts, and its category list
sums to 30.** That figure was not re-derived when VC added two, so it is not
reproduced here. An implementor who needs the per-subject breakdown must re-derive
it from the 29 rows.

## 3. The shape of the target

**Four new crates, not eighteen.** The ten group reports proposed eighteen
`crates/podbox-*` members. `TODO/INDEX.md:42-57` assigns each behaviour a
category and a crate, and that table — not the number of files — decides. Ten of
the eighteen names are a binary with a home, not a crate. The endorsed four:

| crate | binaries | subjects | category row |
| --- | --- | --- | --- |
| `crates/podbox-gate` | `podbox-gate`, `podbox-count`, `podbox-plant`, `podbox-dev`, `podbox-smoke`, `podbox-interpose-build` | `check-todo.py`, `todo-count.py`, `plant.sh`, `dev.sh`, `nightly-smoke.sh`, `398`, `310`, `session-start.sh`, `build-interpose.sh`, `360` | `gate` exists |
| `crates/podbox-buildstate` | `podbox-buildstate`, `podbox-release-licenses` | `build-state.py`, `release-licenses.py` | `packaging` exists |
| `crates/podbox-release` | `podbox-verify`, `podbox-size`, `podbox-prove-t0211`, `document-state`, `release-notes`, `podbox-reconcile`, `podbox-publish` | `verify-release.sh`, `110`, `120`, `157`, `document-state.py`, `release-notes.sh`, `395`, `399` | `packaging` exists |
| `crates/podbox-podvm` | `podbox-podvm`, `podbox-podvm-workload` | `145`, `154` | `podvm` exists |

⛔ **The workspace has NINE members today, not ten.**
`grep -c '^    "crates/' Cargo.toml` returns 9: probe, image, extract, complete,
enter, supervise, windows, ssh, cli. `podbox-interpose` is excluded at
`Cargo.toml:20`, so it is a tenth crate in the tree but not a member. Four new
crates take **nine to thirteen**, not ten to fourteen. Wave 5 adds three of the
four; wave 4 adds the fourth.

All four categories already exist in the index, so no new category row is
required. `TODO/RULES.md` has no crate, member or workspace rule at all —
`grep -in 'crate\|member\|workspac'` returns 0 hits in 121 lines.
`TODO/INDEX.md:42-57` is the authority and this plan cites it rather than
`RULES.md`.

**`crates/podbox-interpose` cannot host `tests/*.rs`.** `crate-type = ["cdylib"]`
at `crates/podbox-interpose/Cargo.toml:15` yields
`error[E0433]: cannot find module or crate` in a test file: a `cdylib` produces
no importable Rust library for a test target to name. The meta review verified
this by building the minimal case, not by citing the rule. Adding `rlib` fixes
name resolution. Its cost is not shared dependency resolution — the manifest's
`[dependencies]` is empty at `:18` and the crate is excluded at `Cargo.toml:20` —
it is a second release-profile artefact built for two targets on every
`scripts/build-interpose.sh` run and on `gate.yml:99`, under the byte ceiling at
`scripts/build-interpose.sh:48`. **The plan does not add `rlib`.** The interposer
export check needs no new test: `scripts/build-interpose.sh:197-214` already
compares `nm -D` output against `crates/podbox-interpose/interpose.map`, which
declares 112 names, for both targets. Check B, the `struct stat`/`struct statx`
offsets under both libcs, needs a `cc` invocation and joins the same
`podbox-gate` binary.

## 4. Work already done in Rust

Eight scripts are partly or wholly re-expressed already. The meta review checked
each clause by clause rather than trusting the meta report's summary, and the
result differs:

| script | state | what remains |
| --- | --- | --- |
| `388-interactive-shell.sh` | **fully converted**, 12 tests, `crates/podbox-ssh/tests/session_interactive.rs:255-482` | none. Delete the script and repoint `TODO/podssh.md:79` and `:126` in the same change. |
| `220-extract-path-safety.sh` | **fully converted**, all 6 checks in `crates/podbox-extract/src/drive.rs` at `:167`, `:211`, `:229`, `:264`, `:382`, plus filesystem checks at `:221`, `:236`, `:253` | none. The meta report cited only `safety.rs` and called it 5 of 6. |
| `170-probe-cache.sh` | clauses 2 and 3 done, 4 tests at `crates/podbox-probe/src/probe_cache.rs:233-296` | clause 1 is two `podbox probe --json` runs compared with `jq`; clause 3's engine half stays shell. |
| `160-store-gc.sh` | 3 of 5. `crates/podbox-image/src/contain.rs:118` is clause 5 | clause 3 (CLI exit codes) and one naming assertion. `experiments/results/store-gc.txt` records `rmi_under_holder 125` and `prune_skipped 1`; `crates/podbox-cli/src/images.rs:961-986` returns 0 from `prune`, so a test written to the plan's assertion fails. |
| `70-whiteout-contract.sh` | 4 predicates done | check B is a measurement over a real `alpine` layer and no Rust test reads one. `crates/podbox-extract/src/drive.rs:480` asserts the OPPOSITE outcome on a crafted layer. Not deletable. |
| `90-nsswitch-contract.sh` | check A partial | checks B, C, D unconverted; check A's live form unconverted. `crates/podbox-enter/src/identity.rs:336` drives the editor, not the NSS dispatcher. Not deletable. |
| `80-interposer-abi.sh` | 4 of 5 checks at `crates/podbox-enter/src/abi.rs:1170-1315` | check B is four `LD_PRELOAD` pairings against live loaders, not a predicate. Check E has no test at all: `crates/podbox-cli/src/system.rs:603` holds 7 tests and none names `abi`. Write the three exit codes of `system::abi` at `:191-214`. |
| `149-podvm-non-goals.sh` | partial, tests at `crates/podbox-supervise/src/nongoals.rs:253-302` and `report.rs:1069`, `:1088`, `:1105-1107` | clause 6 shells to `361-guest-usernet.sh` with a 1500-second bound. No Rust arm. |

**15 specific clauses across the 8 are asserted by no Rust test today.**

## 5. The seven waves

Round 2 of the meta review rejected three of the meta report's nine boundaries as
convenience and kept six; verification added a seventh for the DELETE scripts
no wave owned. A wave lands with one green full gate, not one
`cargo test`.

The full gate is four jobs in `.github/workflows/gate.yml`, and the commands are
at `:199` and `:213` — not `:197` and `:211` as the meta report cited.

| wave | subject | boundary reason | proof |
| --- | --- | --- | --- |
| **0** | record fixes, no Rust | `experiments/157-lock-inheritance-prove.sh:157`'s sed names `lock.fd`; `crates/podbox-image/src/store.rs:1083` reads `if !sys::close_in_children(fd) {` inside `Lock::try_acquire` (`:1074`, `impl Lock` at `:1014`). A plant encoding the current source must land first, or wave 1's plant passes vacuously. | `py scripts/check-todo.py` exits 0 |
| **1** | delete what is already in Rust | `scripts/check-todo.py:300` `check_tree` reads every tracked file, so a `Prove` naming a deleted script is a red gate. The `TODO/podssh.md:79`, `:126` repoint lands in the same change. | `cargo test --workspace`, gate job `todo` |
| **2** | the inline pure gaps | every target is an existing `mod tests`. No new directory, no new crate, so one `cargo test --workspace` covers the wave. | `cargo test --workspace` |
| **3** | the first `tests/` directories | `cargo metadata` reports `podbox-cli` targets as `['bin','custom-build']` with **no `lib`**. Every `podbox-cli/tests/*.rs` is a `CARGO_BIN_EXE_podbox` black-box driver or needs a new `[lib]`. Shared structural change. | `cargo test -p <crate>`, full gate |
| **4** | `crates/podbox-gate`, absorbing the old waves 5, 6, 7 and 8 | `scripts/check-todo.py:228` `CEILING_SCRIPT = "experiments/110-bloat-delta.sh"` and `scripts/plant.sh:51` `FILES` name the same path. Moving either alone turns the gate red. | all four gate jobs |
| **5** | `podbox-buildstate`, `podbox-release`, `podbox-podvm` beside it | no shared crate, fixture or edge with wave 4. They are separate crates, so they land as their own wave. | all four gate jobs |
| **6** | the DELETE scripts: eleven deletions, `162` retained by VC-2 | verification found twelve DELETE rows assigned to no wave; each carries its group report evidence in `T-R006.md`. | `py scripts/check-todo.py` |

## 6. Blocked work, named

These cannot proceed on this host. Each names its blocker rather than hiding it.

| work | blocker | what clears it |
| --- | --- | --- |
| `crates/podbox-image/tests/store_digest.rs` and every registry-backed integration test | **there is no Rust OCI registry fixture in this tree.** `crates/podbox-image/src/registry.rs:955` `serve_once` is a `fn` inside `mod tests` at `:949` — test-private, one request, plain HTTP, unreachable from any binary. `TODO/image.md:483` gives T-0206's `Prove` as `./experiments/180-registry-fixture.sh`. | build a Rust registry fixture first, as its own wave-0-adjacent entry |
| `experiments/392-kvm-guest.sh`, T-1350, T-1112 | KVM and ReactOS; nested KVM stopped the Windows host on 2026-09-30 | an operator-present session. `docs/limits.md` and `TODO/PROGRESS.md` own this. Do not start a guest unattended. |
| `cargo test --workspace` on this host | `linker 'cc' not found` | the Linux base lane, `sh scripts/windows/run-in-base.sh` |
| any `podbox-cli/tests/*.rs` | `podbox-cli` has no `[lib]` target | the wave-3 decision, stated in the entry |
| `157`'s clause-2 mutation | the stale sed anchor | wave 0 |

## 7. Standing corrections

Round 2 settled these. The final entries obey them.

- **VC-1** `experiments/386-podssh-partial.sh` DELETE stands, on the stale-clause
  reason. `experiments/386-podssh-partial.sh:74,75,77` asserts three strings
  `crates/podbox-cli/src/machine.rs:53` no longer has; the `005638d` diff removed
  them. The retired-number reason cites `TODO/milestones.md:877`, not `:883`.
- **VC-2** `experiments/162-tar-symlink-modes.sh` is **not** dead. DELETE is
  REFUTED. `TODO/interpose.md:1471` names it in T-1311's live `Prove`. Retained
  as a deployment-level check under `docs/conventions/code.md:29`.
- **VC-3** `experiments/95-podman-vfs-ignorechown.sh` RUST-TEST → KEEP-SHELL, and
  a deployment proof at that (`docs/conventions/code.md:29`). Its own exit-2
  path does not exist; the subject is podman's storage option, not podbox.
- **VC-4** `experiments/10-build-target-image.sh` DELETE is conditional on `20`
  and `130`. `experiments/20-enter-target.sh:70` and `experiments/300-run.sh:310`
  name it in live paths. `TODO/gate.md:1125` calls the three-way dependency
  irreducible.
- **VC-5** `scripts/plant.sh` RUST-TOOL stands, and its entry must carry both
  that `scripts/check-todo.py:300` reads it as a tracked file and that
  `gate.yml:56` runs it as the step named `every check can fail`. **13** `Prove`
  lines name it, counted with
  `grep -rn 'scripts/plant.sh' TODO/*.md | grep -c 'Prove'` — verified again in
  the final verification pass, which corrected an earlier claim that they sit in
  three files: `TODO/cli.md` contributes none, so they are in two.
- **VC-6** `experiments/95-podman-vfs-ignorechown.sh` is a **deployment** proof at
  that level, not a unit test. It needs a live podman machine and a rootful
  engine; its own planned proof was `#[ignore]`d for exactly that reason. This
  is the level statement VC-3 leaves implicit.
- **VC-7** `experiments/151-spawn-ambiguity.sh` keeps its SPLIT label.
- The parity table holds **265** rows: `crates/podbox-cli/src/parity.rs:232-537`.
  Counts of 267 counted `struct Row` at `:58` and `impl Row` at `:65`. The tree
  carries four values for one fact — 265 (source, current), 220
  (`TODO/cli.md:63`, falsely labelled CURRENT), 164
  (`experiments/results/parity-drive.txt:7`), 160 (`TODO/cli.md:685`).

## 8. The task entries

The entries are in `refactor/06-entries/`, one file per wave:

- `T-R000.md` .. `T-R006.md` — the seven waves, one entry each
- `verdict-ledger.tsv` — all 121 scripts with their verdict and source report line
- `T-R000.md` carries the 14 wave-0 corrections inline, each with the line that
  settles it; there is no separate `record-fixes.md`.

Each entry follows the ten fields in `docs/methodology/authoring.md`: Source,
Category, Priority, Effort, Status, Problem, Premise, Approach, Decision, Prove.