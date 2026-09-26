# Progress

## State

168 entries: 3 open, 1 partial, 1 blocked, 163 done. The 2026-09-25
triage of twenty-two open issues (29 through 38, 49 through 60) plus
dependabot PR 9 is recorded below. Twenty entries reopened as
partial, three opened new (T-1337, T-1338, T-1339). PR 9 (rustls
0.23.44 to 0.23.45) merged on green CI. T-0801, T-0605 and T-1325
closed 2026-09-25 on the issue-60 drive (355) with the curated
check and its plant: the curated docker surface has a row
everywhere, `--env-file` is real, `--log-driver` takes
`json-file`. T-0503 and T-1317 closed 2026-09-25 on the no-chroot
drive (356): `-t` refuses naming ptmx ahead of the chroot gate,
and both no-chroot families run (loader for dynamic, memfd for
static) with the banner naming the rung. T-1301 closed 2026-09-25
on the TCG drive (357): the machine legs split into required,
accelerated and networked, `tiers.machine.refusal` null with
`profile` `tcg` on the kvm-less tcg-listing lane, 146 re-driven
to VMR-GUEST-READY. T-1003 closed 2026-09-25 on the ladder drive
(358): forced rundir and cache enter with the rung word (`runs/`
cleaned, cache persistent), tmpfs and fuse refuse at 125 naming
the mount and the node, memfd regresses green, the unknown word
lists the rungs; tmpfs is wired (`sys::mount`/`umount`,
`stage_tmpfs`/`release_tmpfs`) with its entry arm undriven where
no mount holds, and FUSE stays ordered-and-refused with its
blocker named in the entry. T-0606 closed 2026-09-25 on the
supervision drive (359): mediation and supervision split into two
assessments, `tiers.supervision` available with both new rows ok,
the banner silent where mediation holds, the 230 loop 20 of 20;
the denied-notify fallback is unit-pinned where the lane cannot
show it. T-1306 closed 2026-09-26 on the guest-networking drive
(361): the kvm node stays refused naming ENOENT while its remedy
runs executable, datagrams crossing both ways under TCG
user-mode networking with 149 green at 15 driven 0 mismatches;
the netboot driver chain (failover, net_failover, virtio_net) is
measured link by link. T-1112 worked 2026-09-26 to blocked:
winquick studied in the entry's order, the platform gate moved
ahead of tier dispatch (a Windows request over the machine tier
read as a Linux guest), driven by 362 with the store untouched
on every path; the guest arm needs a KVM host with a licensed
image, which no reachable machine is. No other open issue is closed:
each closes with its fix commit, drive output and guard, per the
operator rule.

M0 through M8 are implemented and the machine tier holds its probe.
End-to-end acceptance is measured per entry.

## Baseline

The source tree was captured at `ea5b671b`.
Its hosted gate was green. In an isolated WSL environment using the exact
`rust:1.98.1-bookworm` image, bootstrap, formatting, workspace clippy, the
x86_64-musl release build, TODO consistency, and marker checks passed.

The workspace test run found one real portability defect: when `mknod` was
denied, device completion removed an existing shim before recreating it. The
replacement now creates a sibling candidate and renames it atomically only
after successful device creation. The focused regression test passes in the
same rootless environment. The complete migrated-tree validation is recorded
in [`docs/history/migration-2026-09-11.md`](../docs/history/migration-2026-09-11.md).

The raw source-era record, including all measurements and resolved questions,
is preserved at
[`docs/history/source-progress-ea5b671.md`](../docs/history/source-progress-ea5b671.md).

This session measured on the lane: `rust:1.98.1-bookworm` job
containers in `wsl-toolkit-podbox`, host kernel
`7.2.0-WSL2-STABLE`, lane-built musl debug binary reporting
`0.1.0-beta.7` at commit `3ac5a72` (dirty: the untracked 353 drive
script), interposer objects absent (no `build-interpose.sh` in the
lane). `experiments/353-open-issue-triage.sh` drove 37 clauses
with isolated stores, exit 0, report in
`experiments/results/triage-353.txt`.
`experiments/354-lifecycle-same-store.sh` drove the full loop in
one store, exit 0, report in
`experiments/results/lifecycle-same-store.txt`. All seven
`v0.1.0-beta.7` assets verified sha256-ok with outer `e_machine`
per arch and exactly two embedded `0x3e` `ET_DYN` objects each
(issue 58's table confirmed).

## Where the work runs

⭐ **Three host lanes, and `./scripts/session-start.sh` picks one.** A Linux
host and a container run everything directly. A Windows host runs the record and
document checks on the host, and every Linux step in a disposable container
inside the distribution `wsl-toolkit-podbox`.
[`docs/containers.md`](../docs/containers.md) holds the procedure, the
exclusions and the traps each lane has.

⛔ **`wsl.exe` is never called.** `sh scripts/windows/run-in-base.sh` is the
Windows half of `./scripts/dev.sh check`.

⭐ **The base is a native lane, and this session proved it.** The
wsl-toolkit base (`wsl-toolkit-podbox`, Arch, kernel 7.2.0-WSL2-STABLE)
runs user namespaces (`unshare -Urm` exits 0), mounts binfmt_misc and
cgroup v2, and answers root inside `base exec`. The shipped binary
reports `namespace` there. Kept in the base, documented here because it
is persistent shared infra: docker 29.8.1, go 1.27.1, jq 1.8.2,
qemu-user-static 11.1.1-4. Scratch (`/root/pb-wk`, `/root/pb-bin`,
`/root/target-docker.tar`) is removed at session end.

⛔ **`main` takes a direct push now.** `enforce_admins` was turned off on
2026-09-11 and a direct push was verified. The four required checks still run
and still have to be green. [RULES.md](RULES.md) section 2 carries it, and a
force push stays refused.

Two lane traps paid for again this session, both recorded where the
next session looks first. Do not disable path conversion for
`run-in-base.sh` (the header names it: the wrapper path arrives
untranslated and the job fails naming a `C:\tmp` path). A staged job
takes the checkout from its working directory (`/work`), never from
`$0`, which names the staging path (the 353 and 354 headers name
it).

## What this session did, 2026-09-25 (triage)

Oriented per AGENTS.md (session-start, PROGRESS, RESUME,
sessions.md, containers.md, agent-tooling.md, gate.md, RULES.md,
authoring.md, experiments.md, reviews.md, the conventions), gate
green at start (`check-todo.py` ok, 165 done). Three read-only
subagents verified every issue citation and the Actions tree in
parallel: issues 49 through 60 and 32, 33, 35, 36, 38 cite
clean; four corrections ride in the entries (29: winquick is
`0.5.0`, `ed6a6401` resolves to nothing, beta.7 is `4f087d0`;
30: `exec` is deliberately ungated, the issue overstates the
gate; 31: the onelf path needs the `tree/` infix; 34: the
`Continue` dispatch lives where T-0606's Premise puts it, not at
the quoted `notif.rs` range).

Drove `experiments/353-open-issue-triage.sh` (37 clauses, exit
0) and `experiments/354-lifecycle-same-store.sh` (11 steps, exit
0) on the lane with isolated stores. The 353 lifecycle tail
(`start`, `exec`, `logs`, `stop`, `rm` at 125) is a harness
artefact, not a product finding: each clause ran in a fresh
store holding no record. 354 holds one store and the loop is
green end to end (`pull`, `run --rm`, `create`, `start`,
`exec`, `ps`, `logs`, `stop`, `wait`, `rm`, `ps -a`).

Reopened twenty entries as partial with the decision underneath
each: T-1112, T-1317, T-0501, T-0503, T-1003, T-0413, T-0414,
T-0415, T-0605, T-0606, T-1301, T-1306, T-1308, T-0207, T-0209,
T-0711, T-1327, T-1332, T-0801, T-1325. Opened three new ones:
T-1337 (doctor, df, tail), T-1338 (perf harness and gate),
T-1339 (namespace rung). Counts move only through
`scripts/todo-count.py` with `scripts/check-todo.py`.

Fixed in place, each verified against the tree: the README rung
table (run enters chroot with `.Rung` and `.EnteredRung`
carried; the machine tier runs where its legs hold; the
interposer is x86_64-only), and the `logs` parity base note
(now names `-f` following through T-1318). The 353 report
carries the old note bytes: the drive ran before the fix.

Merged PR 9 after its branch went green on all four gate
checks: rustls 0.23.44 to 0.23.45 (handshake alignment,
second-ClientHello rejection, key zeroize). Actions audit:
checkout v7, upload-artifact v5, cosign-installer v3.10.1, all
SHA-pinned; no deprecated action, no removed command, no old
runner; the single notice on every job is the ubuntu-latest to
Ubuntu 26 migration of 2026-10-19.

## Triage table

Lane is the `rust:1.98.1-bookworm` job container on kernel
`7.2.0-WSL2-STABLE` with the lane-built musl debug binary,
unless a row says code or entry. `353` is
`experiments/results/triage-353.txt` with its clause.

| issue | reproduces | side | fix area with files | risk if wrong | Prove asserts |
| --- | --- | --- | --- | --- | --- |
| 29 T-1112 | yes, 125, nothing fetched (353 `29-platform`) | operator brief (guest arm missing) | new `podvm` windows driver beside T-1303/T-1304, `lifecycle.rs` gate last | TCG claimed as equivalent; licensed image redistributed | guest `ver` string on a KVM host, or 125 naming the exact missing leg with no fetch |
| 30 T-1317 | on target shape per entry fixture; lane is chroot-capable so `run` exits 0 (353 `30-run`) | operator brief (no rung runs) | `crates/podbox-enter`, `lifecycle.rs` gate, T-1003 ladder | weaker isolation stated as equal | `run --rm alpine echo hi` exits 0 with the rung banner on the denying fixture, beside the refusal strict arm |
| 31 T-1003 | yes, rung refusals at 125 where chroot holds (353 `31-rundir`, `31-bogus`) | operator brief (table is the work list) | `podbox-enter/src/ladder.rs`, `plan.rs`, probe feeds, `cli/src/ladder.rs` | a rung that fetches but never enters | forced run enters with the rung word in `PODBOX_ACTIVE_MODE` per rung |
| 32 T-0413 | legs read (353 probe JSON); payload arm per recorded 155 drive | operator brief (fixture half out) | `podbox-complete`, interposer map, probe banner | fixture answering wrong where it looks right | 155 extended with an emulation success arm beside the failure arm |
| 33 T-0414 | yes, legs read ok on lane (353 probe JSON) | operator brief (remedy open) | `probe/src/probes.rs` Census, every enumerating call site | failure reading as missing file | 155 extended with a list-`/` payload on an `EACCES` host, plus a unit test |
| 34 T-0606 | legs 3 ok refusal null on lane; fallback absent in code | operator brief (no degraded supervision) | `probe/src/probes.rs`, `supervise.rs`, report selection | silent `Continue`-style false success | supervision available where `pidfd`/`waitid` hold with notify denied; loop 230 still 20/20 |
| 35 T-1301 | yes, 125 naming legs both spellings (353 `35-tier-machine`, `35-podvm`) | operator brief (no TCG without KVM/tun) | `MACHINE_LEGS` in `probes.rs`, `machine.rs` | emulated run claimed hardware isolated | TCG profile refusal null on a kvm-less tcg-capable host; initramfs boots with ready marker |
| 36 T-1306 | yes, six stances read (353 probe JSON) | operator brief (nothing promoted) | non-goal assessment, `149` script | new code weakening honesty rules | `149` with a positive arm for the promoted goal, refusals unchanged |
| 37a logs note | yes, old bytes in 353 output; fixed post-drive in `parity.rs` | shipped text wrong | `cli/src/parity.rs:264` | table rotting the contract | corrected note in `system info`; T-1325 check 27 holds it |
| 37b ordering | in code (`run.rs:629` before `:815`); lane ptmx ok so `-t` exits 0 | operator brief (arm unreachable) | `run.rs` prepare order, `lifecycle` gate vs `ptmx_usable` | a second mask beside the first | `run --rm -t` on a both-denied fixture naming ptmx, beside the chroot-only arm |
| 38 QOL | yes, help shows no doctor/df, logs only `-f` (353 `38-*`) | new work (T-1337) | `cli` system/logs verbs, probe machine legs | scope creep across three verbs | doctor exits 1 with fix lines; df sums to store accounting; `--tail 5` last five lines |
| 49 T-0415 | yes, in code (`devices.rs:129`, no test) | spec residual (Done admits it) | `complete/src/devices.rs` | refusal regressing green | new unit test on the unlink-failure arm; live-image arm or dropped claim |
| 50 T-1332 | yes, in entry text (`cli.md:1088-1090`) | spec residual (Done admits it) | T-1332 prove script, `cli/src/man.rs` | silent fallback passing | bogus-PAGER run with stderr asserted empty |
| 51 T-0207 | yes, results file loopback-only | spec residual (Done admits it) | `image/src/pull.rs`, `190` script | close claiming an untaken number | `190` with loopback and latency-bound ratios committed |
| 52 T-0711 | yes, in entry text (`interpose.md:1129-1131`) | spec residual (follow-up owed) | `interpose/src/identity.rs`, `interpose.map` | refusal wording not settled by measure | recorded errno-by-call reading; refusal naming call and errno |
| 53 T-1308 | yes, in entry text (`podvm.md:668-670`) | spec residual (Done admits it) | measurement script, results | two tallies disagreeing | re-driven `154` with agreeing tallies and flags in conditions |
| 54 T-0209 | yes, `logout` 125 None row; no `IsTerminal` in `credentials.rs` | spec residual (residual list) | `image/src/credentials.rs`, `cli` | TTY hang where docker refuses | TTY refusal with test; `logout` arm either way; exit-code note |
| 55 T-0501 | yes, 125 no-row both verbs (353 `55-*`) | spec residual plus table gap | `cli/src/parity.rs`, run/exec parsers, enter device plan | omission read as decision | device-map run plus refusal arm; T-1325 gate extended |
| 56 T-0605 | yes, 125 no-row (353 `56-log-driver`) | spec residual plus table gap | `cli/src/parity.rs`, run parser, supervise sink | omission read as decision | accept and refusal arms through the binary beside the row |
| 57 PERF | yes, by construction (one host one shape each) | new work (T-1338) | `experiments/`, scripts, new gate check | benchmark hiding its host | harness green on sandbox and KVM host; gate red on planted regression |
| 58 T-1327 | yes: outer `e_machine` per arch with exactly two embedded `0x3e` `ET_DYN` objects in all seven sha-verified assets | coverage axis (decision stands) | `podbox-interpose`, `interpose.rs`, `build.rs`, `build-interpose.sh`, smoke, nightly workflow | all-arch claim over six payload-less archs | `readelf -h` per shipped binary reading its own `e_machine`, smoke-asserted |
| 59 namespace | yes (`59-entered`: `supervise chroot`); README fixed | shipped text wrong; rung unimplemented (T-1339) | `README.md` (done), `podbox-enter` ladder | isolation read that never happened | mount-invisible run with `.EnteredRung` `namespace` on a capable host |
| 60 parity | yes, 3 flags and 2 verbs driven; rest spot-checked absent | spec invariant violated (T-0801) | `cli/src/parity.rs`, parsers for promoted flags | enumerable table omitting scripted surface | curated flag list driven row by row with reasons; usage sentence corrected |

## Current work order

Next session implements earnestly, entry by entry, each with
fallbacks built in. A refusal is the last resort after all rungs
fail, never the whole answer: it names the tried rungs with the
missing leg, banners every degradation on stderr, keeps payload
stdout clean, and keeps docker exit codes. Every number ships
with its script in `experiments/`, pinned inputs, printed
conditions, exits 0 matched, 1 failed, 2 could not run. For a
conclusion that ships: three candidate explanations before
testing, test to refute, one more pass for what is missing.
Close each GitHub issue only with a proof comment showing the
fix commit, the drive output, and the guard that stops
recurrence.

1. T-0801 (46 flags, 10 verbs) with the T-1325 curated-list
   check and plant. Unblocks the honest table.
2. T-0503 ordering, then T-1317 no-chroot rung. The target
   shape needs one family running.
3. T-1301 TCG split. Unblocks the machine tier on the target
   and the T-1112 prerequisites.
4. T-1003 rundir and cache, then FUSE and tmpfs.
5. T-0606 fallback supervision, T-1306 one promotion.
6. T-1112 Linux KVM Windows guest per the winquick shape.
7. T-0413 emulation, T-0414 remedy, T-0415 arms.
8. Small batch: T-0207 latency shape, T-0209 logout and TTY
   guard, T-1332 stderr arm, T-0711 errno reading, T-1308
   tally, T-0501 device, T-0605 log-driver.
9. New work: T-1337 QOL verbs, T-1339 namespace rung,
   T-1338 perf harness with baselines and gate.
10. T-1327 per-arch objects or the qemu-user leg, with the
    smoke `e_machine` assertion first. Issue 58's asset table
    already re-verified this session (seven sha-verified
    downloads, header reads in the T-1327 amendment).

⛔ **Read the CONDITIONS BLOCK of a reading before quoting its figures.**
`experiments/results/store-lock-race.txt` prints whether the tree was modified
and what differed from the commit, because every clause in that script measures
a change and the commit alone names a state that was not run. The command line
above each clause's figures is still the authority on what they measured, and a
mutating clause prints the line it WROTE as well as the line it matched.

## In progress

Item 4 done on the lane (`rust:1.98.1-bookworm` job containers in
`wsl-toolkit-podbox`, host kernel `7.2.0-WSL2-STABLE`, kvm, tun
and fuse absent): `experiments/358-ladder-rungs.sh` exits 0
(`experiments/results/ladder-rungs.txt`), targeted units green
(`podbox-enter` stage+ladder 23 passed, `podbox-cli` ladder 14
passed), full suites green (`podbox-enter` 74 passed, the
`podbox-cli` binary 146 passed, 0 failed). Item 5 first half
done on the same lane: `experiments/359-supervision-split.sh`
exits 0 (`experiments/results/supervision-split.txt`), mediation
Prove holds, supervision available with both new rows ok, banner
silent where mediation holds, 230 loop 20 of 20, full
`podbox-probe` suite 107 passed 0 failed. Item 5 second half
done on the same lane: `experiments/149-podvm-non-goals.sh`
exits 0 with 15 driven 0 mismatches
(`experiments/results/podvm-non-goals.txt`), its clause 6
running `experiments/361-guest-usernet.sh` to green
(`experiments/results/guest-usernet.txt`, datagrams both ways).
Item 6 worked to blocked on the same lane:
`experiments/362-windows-refusal.sh` exits 0
(`experiments/results/windows-refusal.txt`), KVM denied ENOENT,
run, machine-tier run and create each exit 125 naming
windows/amd64 with the store untouched, CLI units 26 passed;
the guest arm is blocked on a KVM host with a licensed image.
T-0413, T-0414 and T-0415 closed 2026-09-26 on the proc-absence
drive (155): the interposer emulates `/proc/self/fd` pipes,
`/proc/self/exe` and the mount-table files exactly or refuses
(tallied `OP_PROC`, `inspect` carries
`Interpose.Emulated.procfs`), completion stages the four
conventional `/dev` links the pinned debian row lacks, the
by-name resolver is pinned past a denied listing as `nobody`,
and `cp` names a required listing with its errno (`cannot list
<dir>: Permission denied`, exit 125); `155` exits 0
(`experiments/results/proc-absence.txt`), ten exact-name unit
tests green, workspace 564 passed 0 failed, interpose 37 passed
0 failed, both clippys clean. T-0207 closed 2026-09-26 on the
latency drive (`190` exits 0,
`experiments/results/parallel-layers.txt`): loopback 3.83 s
against 1.51 s (2.53x), latency-bound 24.62 s against 11.55 s
(2.13x) through the new delay proxy at 2 s an exchange, the
serial manifest diluting the parallel gain per Amdahl; the fixed
bound of 4 stays. Item 8 continues with T-0209. T-1332 closed
2026-09-26 on the pager-isolation drive: a bogus PAGER with piped
stdout exits 0 with empty stderr beside byte-identical output,
and under a pty the fallback is loud; no product change. T-0209
closed 2026-09-26 on its drive: `login` refuses a terminal stdin
instead of hanging (exit 1, naming the pipe), new `logout`
verb removes the stored entry from file or helper (nothing
stored reads as not logged in, exit 1; failed erase keeps
everything, 125), exit codes by the measured 125/1
discriminator with docker's own login codes recorded
unmeasured; eight unit tests plus end-to-end legs green. Small
batch continues with T-0711. T-0711 closed 2026-09-26 on the
identity drive (`106` exits 0,
`experiments/results/interpose-identity.txt`): the victim covers
the full setter matrix and clause G records the errno-by-call
table on the denying lane (grants where the wall maps the id,
EPERM elsewhere); the honest refusal names call, number and name
(`setgid failed with errno 1 (EPERM)`), unit-pinned. T-1308
closed 2026-09-26 on the re-driven workload spread (`154`
exits 0, `experiments/results/tcg-workload-spread.txt`): 14
driven, 0 mismatches, section 1 counting `^workload=` lines
requiring 3 like sections 2 and 4, the pinned `QEMU_FLAGS` and
`BENCH_CFLAGS` printed in the conditions; int 7.5x, sys 10.7x,
mem 1.4x, io 4.9x, every checksum agreeing on every platform.
T-0501 closed 2026-09-26 on the device-map drive (`363` exits 0,
`experiments/results/device-map.txt`): 11 clauses green, the
host file and `/dev/zero` reading back byte-identical, the
six-open flag matrix exact, served creation and EEXIST through
a missing parent, the image-shadow boundary pinned, stat
honest, refusals at 125, musl served with the static opener
honestly ENOENT, the launcher round trip, and `exec`
re-serving the record's mapping; `355` exits 0
beside it. Small batch continues with T-0605.

## Operator questions

⭐ **None is open.**

| question | status | where it lives |
| --- | --- | --- |
| whether `experiments/lib/engine.sh` gains a bounded build entry, and whether the reconstruction's `--privileged` run gets an explicit escape or stays outside the helper | ruled 2026-09-21: build entry yes, narrow fixture-only escape yes | [T-1213](gate.md) |
| whether kept podbox containers and unused base wsl machines may be pruned | ruled 2026-09-22: yes, prune what we do not use or need, safely, touching nothing else | PROGRESS.md (this file) |
| whether a beta binary may be published | ruled 2026-09-22: yes, once the top-10 priority tasks finish and the session ends, under a pre-release tag; work first | PROGRESS.md (this file) |
| where the three remaining interposer checks belong | ruled 2026-09-22: in `dev.sh check`, each with its plant; per-commit toolchain cost accepted | [T-1207](gate.md) |
| whether the non-Linux guest starts | ruled 2026-09-23: unparked, work next; stays P3, displaces nothing | [T-1112](milestones.md) |
| what the next session owes | ruled 2026-09-22: continuous until ten tasks finish, with T-1314 last, then the nightly matrix on the next `v*` tag | PROGRESS.md (this file) |
| how nightly releases work | ruled 2026-09-22: named nightly, every `v*` tag triggers, all seven archs, smoke per arch, stable manual later | [T-1314](packaging.md) |
| which identity signs the beta artefacts | ruled 2026-09-23: keyless via Sigstore, OIDC from the publish job | [T-1328](packaging.md) |

Every settled ruling is written into the entry that owns it, which
is where an implementer reads it.

| question | ruled on 2026-09-11 | where it lives |
| --- | --- | --- |
| where the ownership memo lives | on the host, beside the container record | [T-0710](interpose.md) |
| what `setuid` does on a uid 0 payload | honest failure by default, and a flag turns on the lie | [T-0711](interpose.md) |
| whether host CA injection stays on | yes, unchanged: inject on an announcement, mark degraded, `--strict` refuses | [T-0407](complete.md) |
| what to do with a corpus dependency bump | close it, and fence `references/` off from every updater | `.github/dependabot.yml` |
| the two `memfd-exec` trees, which declare MIT in a manifest and ship no licence file | do not vendor either; the operator maintains a 0BSD crate that does the job | [reference-map.md](reference-map.md), [T-0909](deps.md) |
| `dockless`, which states no licence at all | keep the tree, study it, copy nothing, re-implement where useful | [reference-map.md](reference-map.md) |
| `VHSgunzo/userland-execve`, which does not exist | keep the row as a corrected citation, not a deletion | [reference-map.md](reference-map.md) |
| whether an unlicensed research tree may be tracked here | yes, track the whole tree; the corpus rule wins and nothing may be copied from it | [reference-map.md](reference-map.md) |
| where the docker half runs when the picked engine is not a daemon | nowhere: `have_docker` follows `ENGINE_NAME`, the comparison columns read `-`, the half is recorded as skipped | [T-1212](gate.md) |
| how `280` reaches its wrapper through a helper that execs `/pb` | positional words through `eng_run`, never through `eng_pbrun` | [T-1212](gate.md) |

⚠ **`/dev/ptmx` on the target is a measurement, not a ruling**, and it belongs
to [T-0503](enter.md). [T-0414](complete.md) is the probe leg for it, and that
entry also carries the second denial only one instance of the class has shown:
`readdir("/")` answering `EACCES`.

## What the next session should decide, and neither needs the operator

One is open and it needs no operator:

- ⚠ **Whether the gate should report a rate rather than a pass or a fail.**
  T-0215 is closed, so nothing is red today, but CI reported green for a suite
  that failed two runs in five and a single run is still not evidence for a racy
  one. That is [T-1204](gate.md)'s neighbourhood and it needs a ruling before
  the next intermittent check arrives.

The two this section carried before are settled: the eight unregistered `Lock`
sites were a test-shape question answered by measurement on [T-0211](image.md)'s
invariant, and [T-0706](interpose.md)'s Go row is proved by unit test with no
image invented.

## Operator rulings for the continuous session, 2026-09-21

- Work continuously unless manually stopped, finishing as many entries
  as possible. The five-entry session end does not apply.
- Work the full work order in listed order until stopped.
- Spawn subagents wherever independent work allows it.
- The store-suite contention fix is authorised: author the
  [T-0211](image.md)/[T-0215](image.md)-family entry under the authoring
  methodology, then implement it, in its own change.
- The kept wsl-toolkit job containers go at session start
  (`gc --apply`); past results already live in `TODO/`.
- CI on main is verified first; a red CI becomes the top priority.
