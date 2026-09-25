# Progress

## State

M0 through M8 are implemented and the machine tier holds its probe.
End-to-end acceptance is measured per entry. M7 packaging runs through
the nightly workflow on every `v*` tag.

165 entries: 2 open, 1 partial, 0 blocked, 162 done. Every P0 is done.
The open entries are [T-1328](packaging.md)
and [T-1334](packaging.md): what remains of the
2026-09-23 triage from nineteen client-filed issues plus the
operator-ordered `man` verb, after T-1112, T-1315 through T-1322, T-1330,
T-1331, T-1332,
T-1333, T-1335, T-1336, T-1327 and T-1329 closed below. The
partial entry is [T-1109](milestones.md), carrying its remaining
conditions in its own file. T-1328 and T-1334 stay open tag-gated:
their Prove artefacts (bundles, notes) only exist once the nightly
publish job runs on the next tag.

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

## What this session did, 2026-09-23

The operator's challenge reopened the two blocked entries: the
wsl-toolkit base is a real machine, so the docker-daemon and native-lane
halves could measure there. Both closed.

[T-1213](gate.md) closed on the native base lane. `20` exits 0 (N+F+M,
`/bin/id` answers uid 0, full image id matches the recorded build) and
`130` exits 0 (chroot inside, namespace unconfined, 16 matched with 0
differed after `130 --refresh` re-captured `attribute.txt` with
`census.txt` on the same kernel). `10` stands on its recorded
host-podman build. Committed in the change: the entry record, the
counts, three results files. Commit `6eb941f`.

[T-1212](gate.md) closed on the base docker lane (dockerd 29.8.1).
`150` exits 0 with the clause-1 premise confirmed (the entry carries
the digests). `270`
exits 0: T-0212's owner had already fixed the clause-5a expectation to
read the cli-error code from the binary. `280` exits 0 with all seven
clauses after one native-lane repair (the TLS certs mount hoisted above
the native split: without it the registry exits 1 with no certificate
staged). `320` and `330` exit 0 with their daemon halves against docker
29.8.1 (every 330 comparison cell ok). `300` carries zero FAILs:
clause 7 runs (chroot inside, namespace outside) and the riscv64
refusal half SKIPs where binfmt executes it, guarded in the script.
Committed in the change: two script repairs, six results files, the
entry record, the counts. Commit `e653e8f`.

Three deep review passes ran over every touched file: the claim audit
(each Done sentence against its run log or retrieved hash), the door
sweep (who consumes the hoisted mount and the new guard), the
usability pass. The `check-one-home` guard fired live on one sentence
this session shared between the new record and this file; the entry
kept the fact and this file points at it. One committed sentence
proved imprecise the same session it landed (container port-80 egress
reads as a blanket block; three bounded runs show the podman path
black-holes SYNs while the docker path fetches) and was corrected in
place with the measurements. `docs/containers.md` carries three new
measured traps from the lane.

`v0.1.0-beta.5` is tagged at the T-1212 close-out commit; the nightly
(run 35809485405) concluded success with all seven legs green, and the
pre-release named nightly carries fourteen assets, verified back
through the release API.

## Current work order

1. The nightly for `v0.1.0-beta.5` (run 35809485405) completes, then the
   T-1314 record lands as its own commit, mirroring the beta.4 record:
   the Prove-run paragraph in [packaging.md](packaging.md) and the
   four-line touch here.
2. Session teardown: stop base dockerd, remove base scratch, `gc
   --apply` the three kept job containers, confirm the podman machine
   rests as found (stopped), tree clean, gate green.
3. [T-1109](milestones.md) carries its remaining conditions in its own
   file; [T-1112](milestones.md) is unparked (ruled 2026-09-23) and is
   the next schedulable work, staying P3. Behind it: T-1315 through
   T-1336, triaged from client issues 10-28 (every premise confirmed
   live against the beta.5 tree before authoring; T-0210 extended for
   issue 17 rather than a new entry). Each entry carries its issue's
   closing task: fix commit, proof output, and the recurrence guard.
   Nothing else is open, and nothing is blocked.

[T-0206](image.md) has its fixture technology: `zot` as one pinned
binary plus config, storage dir, generated cert and htpasswd file,
serving the four loopback endpoints with a required credential for
[T-0209](image.md). Owed first is the licence determination
[reference-map.md](reference-map.md) requires before a new tree is
used, then the registry-fixture experiment with all outbound
network blocked.

⛔ **Read the CONDITIONS BLOCK of a reading before quoting its figures.**
`experiments/results/store-lock-race.txt` prints whether the tree was modified
and what differed from the commit, because every clause in that script measures
a change and the commit alone names a state that was not run. The command line
above each clause's figures is still the authority on what they measured, and a
mutating clause prints the line it WROTE as well as the line it matched.

## In progress

[T-1335](supervise.md) closed this session, below. The teardown in the
work order above is still owed.

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

⛔ **Nothing is blocked except what names its blocker.** Nothing is
blocked.

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
