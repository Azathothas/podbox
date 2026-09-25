# Progress

## State

M0 through M8 are implemented and the machine tier holds its probe.
End-to-end acceptance is measured per entry. M7 packaging runs through
the nightly workflow on every `v*` tag.

165 entries: 0 open, 0 partial, 0 blocked, 165 done. Every P0 is done.
The 2026-09-23 triage from nineteen client-filed issues plus the
operator-ordered `man` verb is fully closed: T-1112, T-1315 through
T-1322, T-1330, T-1331, T-1332, T-1333, T-1335, T-1336, T-1327,
T-1329, T-1328 and T-1334 closed below. [T-1109](milestones.md)
closed this session on its own Prove: 250 exits 0 under the 251
cover, with the census in its allowed third state. Nothing is
blocked. Issues 10-13, 15-17, 19-21, 26 and 28
are closed, each with its proof comment (fix commit, drive output,
guard); no open issue remains.

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

## What this session did, 2026-09-25

The handover named commit `6f2aa79` with T-1336 next; the tree had
moved past it (T-1336 done at `7ec5085`, T-1327 and T-1329 closed in
staged work). Reconciled first, then finished the batch: the staged
non-tag work for T-1328/T-1334 plus the two closes, committed as
`6d86129` and pushed to `main`.

Three review lenses ran over the staged files before the commit. The
claim audit verified the T-0704 citation fix (`grep` empty on the
publish path), the cosign-installer pin (digest matches tag
`v3.10.1` through the API), and the release-notes drive against
`v0.1.0-beta.5`. It found the defect that mattered: the staged
verifier expected the publish identity at `@refs/heads/main`, but
the nightly answers version tags alone, so a tag-built bundle
carries `@refs/tags/<tag>` and the staged command would have failed
closed on honest artefacts. Fixed before the commit, and the beta.6
Prove below confirms it. Three prose fixes rode along: the smoke
header now describes save/load instead of a loopback helper, the
T-1329 Done names the save/load substitution and the native-leg
boundary, and the T-1327 Done cites the unit test that pins the
decline instead of the smoke leg.

[T-1328](packaging.md) closed on `v0.1.0-beta.6` (nightly run
36110971684, conclusion success, 21 assets: seven binaries, seven
hashes, seven bundles). `sh scripts/verify-release.sh v0.1.0-beta.6
x86_64` exits 0 with `Verified OK`; the same check against a binary
with one flipped byte exits 1 on the digest mismatch.

[T-1334](packaging.md) closed on the same tag's notes, read back
through the release API; the entry names the facts found there.
Committed in the change: the two entry records, the
counts, this file. The tag sits at the non-tag commit, whose gate
CI is green (4/4 jobs), so the notes' gate line and the bytes agree.

`v0.1.0-beta.6` is tagged at the close-out commit; the nightly (run
36110971684) concluded success with all seven legs green, and the
pre-release named nightly carries twenty-one assets, verified back
through the release API.

The operator then ordered every open issue closed with proof. Eleven
closed directly from their entries' recorded drives (10: T-1315,
11: T-1316, 12: T-1317, 13: T-1334 on beta.6, 15: T-1321, 16:
T-1318/T-1320/T-1323/T-1335, 19 and 20: T-1322, 21: T-1319, 26:
T-1327/T-1328/T-1329 on beta.6, 28: T-1333), each comment showing
the fix commit, the proof output and the guard. The twelfth, issue
17, had no proof to show: T-0210 carried its follow-up as prose the
script never received. Implemented as commit `9899280` (429 and
TOOMANYREQUESTS join both network patterns; check 2 captures the
holder output and a 125 naming `chroot(2)` denied reads SKIP),
driven exit 0 in the lane with all four clauses green, patterns
proved against synthetic transcripts, results file renewed. Issue
17 closed on that drive. The direct order to write the tracker
overrode the read-only API rule per the precedence in section 0:
operator instruction outranks project methodology, and the work
orders themselves direct the close with proof.

[T-1109](milestones.md) closed on its own Prove this session; the
entry carries the drive. `experiments/251-tty-refusal-no-ptmx.sh` is
new and builds the missing pty-less machine, drives the focused
`run -t` refusal with its reason, and runs the full 250 under the
same cover to exit 0 with zero FAILs. The census stays in the allowed
third state, with the cause recorded. Driven twice in the lane
(kernel `7.2.0-WSL2-STABLE`, `0.1.0-beta.7` binary); reports renewed
under `experiments/results/`. Two script defects were found and fixed
before any claim: the first shape bound over the symlink and the
mount answered `not a directory`; the reader then used jq
`// "missing"`, which fires on false as well as null and hid the
covered `false` for one run.

Release polish rode in the same change: workspace version `0.1.0` to
`0.1.0-beta.7` (`Cargo.lock` carries the same seven lines and nothing
else; the prerelease sorts below the future stable `0.1.0`, so the
final beta stays distinct from it), a changelog entry naming the bump
and the T-1109 close, and a doc sweep that found no stale claim (the
README status, the architecture page and the containers page all still
read true; `check-docs` green).

`v0.1.0-beta.7` is tagged at the non-tag commit `4f087d0`, whose gate
CI is green (run 36117430690, success). The nightly (run 36117747572)
concluded success with all seven legs green plus the publish job, and
the pre-release named nightly carries twenty-one assets: seven
binaries, seven hashes, seven bundles. `sh scripts/verify-release.sh
v0.1.0-beta.7 x86_64` and the aarch64 leg each exit 0 with `Verified
OK`; the notes, read back through the release API, carry the build
commit, the gate success with its run URL, and the reproducibility
boundary line.

## Current work order

Nothing is open. 165 entries are done, no issue is open, and the final
beta `v0.1.0-beta.7` is published and verified. The next session takes
whatever the operator orders.

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

Nothing half-written. The teardown is done: the kept lane job
containers went through `gc --apply`, session scratch is removed,
and the tree is clean with the gate green.

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
