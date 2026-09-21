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

139 entries: 30 open, 3 partial, 3 blocked, 103 done.

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

Session of 2026-09-21. Closed [T-1212](gate.md) **blocked** after converting
the last four engine scripts and running all six on host podman 6.1.2.

⭐ **All six image, registry and CLI scripts run through
`experiments/lib/engine.sh` now, with no assertion changed.**
`280-insecure-registry.sh` exits 0 with all seven clauses green after three
conversion repairs. `300-run.sh` exits 2: every runnable clause green, clause 5 SKIP
(no `binfmt_misc` without privilege) and clause 7 SKIP (no docker daemon).
`320-cli-contract.sh` exits 2: clauses 1 to 4 green, the install-names half
green, clause 5's refusal half SKIP without a daemon.
`330-exit-codes.sh` exits 2: every podbox-against-its-own-table row `ok`,
every `docker` column `-`. `150` exits 1 on the index-vs-child digest
difference and `270` exits 1 on the 125-vs-1 platform error, both settled
last session and re-recorded in the entry. `experiments/results/` carries
all six runs.

Three conversion repairs, each found by running the converted script whole:
`280` re-stages its driver mounts after the fixture block's `eng_clear`,
drives its wrapper through `eng_run` with positional words (never through
`eng_pbrun`, which execs `/pb` with the words it is given), removes its
fixtures by name (serve ids captured in `$( )` never reach `eng_cleanup`),
and builds its TLS certificate against an empty config file named through
`winpath` after four lane failures. `320` strips this lane's jq CRLF bytes
before podbox ever sees a verb. One self-inflicted taint: editing `300`
mid-run skipped one byte of the running script, so the run was repeated
clean. Never edit a running script.

⭐ **[T-1213](gate.md) is implemented under the operator ruling and
blocked on lane findings.** The ruling (build entry yes, narrow
privileged escape yes) arrived the same session, so the helper gained
`eng_build` and `eng_privrun`, all three scripts converted with no
assertion changed, and all three ran: `10` exits 0 (image builds,
census green), `20` enters the reconstruction (N+F+M, payload codes
pass), `130` exits 1 (unconfined rung is `supervise` in the driver,
four rows differ against the reference kernel). The entry carries the
runs, the refusal-path probes, and what clears each half.

[T-0805](cli.md) closed in this change: the section-8 table as data with
the ownership row wired to live maps, one shared report for `extract` and
`run`, and the sidecar example it names. Prove runs pull then extract and
reads green beside `gid 42`; pull alone cannot name a dropped id, and the
entry carries the finding.

[T-0809](cli.md) closed in this change: the probe evidence names the
spawn wall per context with both legs measured, and 151 drives both
refusals behind one identical payload message to green. No generic
string matcher ships; the mapping keys on the legs.

[T-0808](cli.md) closed in this change: admit-first everywhere, with the
325 driver green against the shipped binary. T-0801 claimed every dash-arg
goes through `parity::admit` and only `run`/`exec` did; the seven `images`,
eight `lifecycle`, two `system` and one `names` parsers plus `probe`,
`version` and the `image` group now open with `parity::admit_all`, and the
group subverbs resolve their rows through `parity::rows_of`. 153 rows, 192
driven, 0 mismatches, 0 unreachable here
(`experiments/results/parity-drive.txt`); `run -i` and `exec -i` both ran
a self-pulled `alpine:3.20` with the banner naming `-i`. The first drive
found 4 mismatches, all 4 the driver's (group subverbs probed as
top-level verbs, `exec` driven with the run-only `--pull`). Release build,
clippy with `-D warnings`, and the three unit-test packages stayed green
throughout. The T-0801 universal claim now holds, with the correction
under it.

[T-1310](image.md) authored and implemented in two changes under the
[T-0211](image.md)/[T-0215](image.md) family. Task 1 re-measured the contention
in this lane on the unmodified tree: 8 of 10 parallel runs refused with the
16-slot signature across eight victim tests, 2 of 2 serial green (nproc 20,
`experiments/results/store-contention-prefix.txt`). No flip: production holds
at most three locks by inspection, so candidate 1 stands. The implementation
serialises all 24 store tests under one mutex, pins the ceiling with a
deterministic seventeenth-refused test, and adds the pool-bound contract to
T-0207. After: 10 of 10 parallel green, ceiling 3 of 3, serial green, audit
24 of 24, full `dev.sh check` green
(`experiments/326-store-contention-prove.sh`,
`experiments/results/store-contention-prove.txt`).

[T-1208](gate.md) check 22 lands here with its plant case: every `done` entry
must open its record with `**Done` on the first unindented line after `Prove`.
The check found five prose records, not the four the entry names (T-0505
beside T-0204, T-1103, T-0107 and T-0108), and all five are converted in this
change. The plant strips the marker across `TODO/probe.md` and asserts the
gate goes red naming the defect. T-1207's stale `Problem` is corrected in
place (the interpose steps are already in `dev.sh check`); its Decision still
needs a ruling, so it stays open. [T-1208](gate.md) closed after the plant
run on the committed tree: 27 caught, 0 missed, 3 controls quiet, full gate
green alongside.

[T-1311](interpose.md) is filed in this change and stays `open`. The
T-1111 nix acceptance unpacks its binary tarball under `podbox run` and tar
exits 2 setting modes on symlinks, while the engine control exits 0. The
interposed `fchmodat` declares and forwards three arguments where libc takes
four, so the real call reads a fourth register the wrapper never set. The
fix and its regression proof belong to T-1311, in its own change; T-1111
waits on it.

What stays current from last time:

⚠ **The guest lane still cannot run a docker daemon.** Dockerd fails
creating the DOCKER chain with `iptables ... Permission denied`, measured
2026-09-19: the job container holds no `NET_ADMIN`. Engine clauses move to host
podman. Reopen condition for the guest lane: a job container with
`NET_ADMIN`, or a base-level daemon the jobs can reach.

⚠ **This lane's jq ends every raw-output line with CRLF**, and the shell's
command substitution strips only the trailing one. Multi-line `jq -r`
streams poison every line but the last. Single-value reads stay clean.
`320` clause 3 and `330` clause 0 strip the transport bytes; Python reads
the table's values clean.

⚠ **This lane's OpenSSL reads a system config its own build rejects.**
`req -x509` needs an empty config file at a Windows-spelled path with
conversion off for the call. `280` carries the shape.

⚠ **The store suite failed the commit gate three times, a different
victim each time, all `two_*`** (`two_holders_of_one_image...`, then
`two_staging_calls_in_one_process...`, then `two_platforms...` plus
`two_holders...` again: 95 of 97 around the third). Run 3 names the
mechanism twice: `Store("this process already holds 16 locks, which
is every slot podbox has ... (T-0211)")`. The pool is process-wide
and fixed: `FORK_CLOSE_SLOTS` in `crates/podbox-probe/src/sys.rs`,
one slot per `Lock::try_acquire` in
`crates/podbox-image/src/store.rs`. Libtest runs the suite in threads
of one process. Each `two_*` test holds two or more locks at once:
`two_holders` keeps `a` and `b` live together. Parallel neighbours
exhaust the 16 slots, and the hungry test that asks last is refused.
The victim varies with scheduling. No source changed under any of it:
the tree differs from the green run in `experiments/` and `TODO/`
alone, which `cargo test --workspace` never reads. A fresh container
per run refutes stale state. A cut-down probe without the gate's own
setup proved nothing twice now: it dies building `ring`, so the `zig
tools` bootstrap is load-bearing. A serial run with the gate's own
setup confirmed it: 97 of 97 pass with `--test-threads=1`, all three
victims green uncontended. The suite is correct; the parallelism is
the defect. The fix,
serial store tests or scoped slots, belongs to the
[T-0211](image.md)/[T-0215](image.md) family, not to the change under
gate. The serial run came back green, and the fourth and final
full-gate attempt came back fully green with it (97 of 97 on the
store suite), so the change commits with the three reds named above.

## Current work order

1. [T-0805](cli.md), [T-0809](cli.md), [T-0808](cli.md) and [T-1310](image.md)
   are done. [T-1310](image.md) closed the store-suite contention in its own
   change: one suite mutex, the ceiling pinned, 10 of 10 parallel runs green
   after 8 of 10 refused before.
2. [T-0408](complete.md) is done: the opensuse-leap row ran green (zypper 0 0
   42) and the transcript is recorded under the entry.
3. [T-1207](gate.md) and [T-1208](gate.md): [T-1208](gate.md) is done (check
   22, its plant, five records converted). [T-1207](gate.md)'s stale claim is
   corrected in place; its Decision still needs a ruling.
4. [T-1108](milestones.md) is done: the shipped artefact is static with no
   `PT_INTERP`, and the staged binary runs its version inside the
   reconstruction.
5. [T-1111](milestones.md): M8, the nix acceptance, after M7.
7. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe. ⭐ T-1302's shape is ruled, so its implementation is a flag,
    a third `ALIASES` entry and the collision rule.

[T-1211](gate.md) stays `blocked` on new [T-1309](interpose.md): the rocky
rows read no-compiler under the interposer while the engine control reaches
42. What clears it is T-1309 fixed. [T-1212](gate.md) is `blocked` on a
daemon, privilege, and two expectation owners. [T-1213](gate.md) is
`blocked` on a native lane for clause 2 and a same-machine reference
for clause 3. [T-1209](gate.md) and
[T-1210](gate.md) are `done`.

## In progress

[T-0808](cli.md) is `done` and [T-1310](image.md) is `done` and commits here. No
implementation entry is half-written. [T-1212](gate.md) went `blocked`
in this change with its six runs and what clears each red half, and
[T-1213](gate.md) went `blocked` in this change with its conversion,
its three runs and its lane findings. Three entries
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

⭐ **None is open.** The T-1213 ruling arrived the same session it was
asked in, and it lives in the entry.

| question | status | where it lives |
| --- | --- | --- |
| whether `experiments/lib/engine.sh` gains a bounded build entry, and whether the reconstruction's `--privileged` run gets an explicit escape or stays outside the helper | ruled 2026-09-21: build entry yes, narrow fixture-only escape yes | [T-1213](gate.md) |

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

⛔ **Nothing is blocked except what names its blocker.** [T-1211](gate.md) on
[T-1309](interpose.md) and [T-1212](gate.md) on a daemon, privilege, and two
expectation owners. The docker-daemon gap stops the docker-driven experiment
clauses, not the work: the next docker-driven item names its own route when it
gets there.

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
