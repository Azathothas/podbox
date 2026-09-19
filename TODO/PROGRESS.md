# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl, the shipped binary EMBEDS both, and since 2026-09-18 it PLACES the
selected object inside the rootfs, CLASSIFIES the payload with a named
decline, and REWRITES mapped paths across 88 entry points. Since 2026-09-19
it also FAKES the identity under `--user` and refuses honestly without it,
and HOLDS the ownership memo on the host beside the container record with a
bounded read that refuses past its ceiling instead of answering stale.
End-to-end acceptance remains open.
M7 packaging has not started. ⭐ **M8 is the
nix acceptance**, [milestones.md](milestones.md) T-1111, and it drives the
shipped binary, so it is the last gate rather than an early one. The machine
tier, [podvm.md](podvm.md), is specified and not started.

137 entries: 37 open, 3 partial, 1 blocked, 96 done.

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

⭐ **THAT GAP IS CLOSED. `cargo test --workspace` is deterministic again**, as
of 2026-09-12. It read between 2 and 10 of 12 and between 5 and 12 of 20 across
twenty-two passes over two days, and it reads **0 of 30 in each of two passes**
now. [T-0215](image.md) carries the mechanism, the captures that named it, the
one-syscall fix and the clause that reddens the fix on demand.
⛔ **`close(2)` is not a release while anything else references the same open
file description**, and a `fork` makes exactly that.

The raw source-era record, including all measurements and resolved questions,
is preserved at
[`docs/history/source-progress-ea5b671.md`](../docs/history/source-progress-ea5b671.md).

## Where the work runs

⭐ **Three host lanes, and `./scripts/session-start.sh` picks one.** A Linux
host and a container run everything directly. A Windows host runs the record and
document checks on the host, and every Linux step in a disposable container
inside the distribution `wsl-toolkit-podbox`.
[`docs/containers.md`](../docs/containers.md) holds the procedure, the
exclusions and the traps each lane has.

⛔ **`wsl.exe` is never called.** `sh scripts/windows/run-in-base.sh` is the
Windows half of `./scripts/dev.sh check`, and it ran the complete check in
**1 m 19 s** on 2026-09-11 and **1 m 28 s** on 2026-09-12, both warm.
⭐ **A job hands its evidence back through `/out`** now, named by
`PODBOX_ARTIFACTS`, because the container is removed when it exits. ⚠ The pack
step failed once on 2026-09-12 and the report was recovered from the job's own
stdout, which the scripts print before they copy.

⭐ **`main` takes a direct push now.** `enforce_admins` was turned off on
2026-09-11 and a direct push was verified. The four required checks still run
and still have to be green. [RULES.md](RULES.md) section 2 carries it, and a
publish branch is the fallback if protection is ever restored.

## What this session did

Session of 2026-09-19, third half. Closed [T-0710](interpose.md) with its
proof (7 checks, exit 0) and filed the engine conversions as entries without
implementing them, per [authoring.md](../docs/methodology/authoring.md).

⭐ **The ownership memo lives on the host now.** `table::memo_path` beside the
container record, `PODBOX_MEMO_FD` 17 handed at every spawn (foreground
ephemeral under staging, `create`/`run -d` moved to `containers/<id>/`,
`exec` re-handed, launcher and T-0412 steps sharing it) and refused where not
handed. The interposer reads and writes through the handed number only, seeks
to end on record and rewinds on lookup; past 4 MiB it answers `BeyondCeiling`
and the `stat` family reports the real owner marked degraded, never stale.
`experiments/results/interpose-ownership.txt` carries the run: OLD_STAT=0:42,
NEW_STAT=0:0 refusal with the degraded line, 5242944 bytes. A guest `podbox
run` smoke test answered 0:42 through the host memo.

⭐ **Four conversion entries, no conversions.** [T-1210](gate.md) through
[T-1213](gate.md) cover the sixteen remaining engine scripts in four groups;
the helper and the 105 conversion are the shape each cites. Authoring and
implementing stay in different sessions.

What stays current from last time:

⚠ **The lock-table flake fired seven times today and went quiet the eighth.**
Eight guest `cargo test --workspace` runs read 7, 2, 5, 3, 2, 1 and 3
failures, then 0, all in the `podbox-image` store lock-table tests with the
recorded "already holds 16 locks" refusal. Same tree, same message as the
earlier 2026-09-19 readings, count moving with scheduling luck. Interpose
(11), cli (66) and every other suite are green on all eight runs; the green
eighth pass is the gate this change commits against.

⚠ **The guest lane still cannot run a docker daemon.** Dockerd fails
creating the DOCKER chain with `iptables ... Permission denied`, measured
2026-09-19: the job container holds no `NET_ADMIN`. Engine clauses move to host
podman, where `105` is green with all seven checks. Reopen condition for the
guest lane: a job container with `NET_ADMIN`, or a base-level daemon the jobs
can reach.

## Current work order

1. [T-1209](gate.md): the 37 remaining `Prove` lines that pull from Docker Hub,
   the mapping that decides each replacement, and the check and plant that stop
   the next one. ⚠ Take the mapping and the sweep before the check: the check
   goes green only once nothing violates it.
2. [T-1210](gate.md) through [T-1213](gate.md): convert the sixteen engine
   scripts through `experiments/lib/engine.sh`, one group at a time, without
   changing what any of them asserts.
3. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
4. [T-0408](complete.md): run the zypper row and record it. It is one container
   run, and it is the only entry reopened for having no evidence at all.
5. [T-1207](gate.md) and [T-1208](gate.md): the excluded interposer crate's gate
   coverage, and the check that a closed entry carries its recorded run.
   ⭐ T-1208's shape is RULED now, so what is left is the check, its plant, and
   converting the four prose records that entry names.
6. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
7. [T-1111](milestones.md): M8, the nix acceptance, after M7.
8. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe. ⭐ T-1302's shape is ruled, so its implementation is a flag,
    a third `ALIASES` entry and the collision rule.

## In progress

No implementation entry is half-written. One entry is newly `done` in this
change and goes out with it: [T-0710](interpose.md). Four entries are newly
`open` in this change and go out with it: [T-1210](gate.md) through
[T-1213](gate.md), the engine conversions, authoring only. Three entries
remain `partial`: [T-0503](enter.md), [T-0704](interpose.md) and
[T-1109](milestones.md), each carrying its remaining conditions in its own file.

[T-0408](complete.md) is `open` rather than `done` for the simpler reason that
reopened it: it carried a `Prove` line and nothing after it.

⛔ **Read the CONDITIONS BLOCK of a reading before quoting its figures.**
`experiments/results/store-lock-race.txt` prints whether the tree was modified
and what differed from the commit, because every clause in that script measures
a change and the commit alone names a state that was not run. The command line
above each clause's figures is still the authority on what they measured, and a
mutating clause prints the line it WROTE as well as the line it matched.

## Operator questions

⭐ **None is open.** Every ruling is written into the entry that owns it, which
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

⚠ **`/dev/ptmx` on the target is a measurement, not a ruling**, and it belongs
to [T-0503](enter.md). [T-0414](complete.md) is the probe leg for it, and that
entry also carries the second denial only one instance of the class has shown:
`readdir("/")` answering `EACCES`.

⛔ **Nothing is blocked.** The docker-daemon gap above stops the docker-driven
experiment clauses, not the work: `106` proves the substitution, and the next
docker-driven item names its own route when it gets there.

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
