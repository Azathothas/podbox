## The task

Continuous session of 2026-09-22. Close T-0705 (reverse mapping, done in
worktree, 15 dirty files), push straight to main, verify CI, then work the
open entries (T-0707/T-0708 neighbourhood first). Engine clauses run on
host podman; stop `podman-machine-default` at close-out.

## The resume point

Lane rebuild of the binary is running (`.dev/artifacts-pb`, job
`bash-u8m3duwx`). When it lands: run the arch/void full-stderr interpose
probe plus the T-0705 8-row drive, then host gates, then the lane check
only if the tree moved under it, then commit T-0705, push origin main,
watch CI to green.

## In flight

T-0705's change is done in the worktree and uncommitted: `TODO/INDEX.md`,
`TODO/PROGRESS.md`, `TODO/interpose.md`, `crates/podbox-interpose/`
(`interpose.map`, `src/lib.rs`, `src/map.rs`),
`experiments/results/distro-sweep.txt` plus eight sweep transcripts.
`archlinux.out` and `voidlinux-musl.out` are unchanged on disk: measured
as a `tail -5` transcript-window artifact (their CA-rehash step leaves
five trailing warnings that push the interpose line out of the window;
opensuse-leap with one trailing warning still shows it), not a missed
run. A two-row full-stderr probe still owes to close that gap.

## State

Tree dirty, 15 files, HEAD `02592e1`. `fullcheck-T0705.log` read
LANE_RC=0 at 2026-09-22T01:37Z on the final tree (re-verify before
commit). Host gates owe a re-run after the last two PROGRESS/entry
rewordings. Podman machine `podman-machine-default` running; lane job
ledger: this session's containers await GC at close-out.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0705 is done in the worktree and uncommitted;
the lane binary rebuild is the first thing to check. Close T-0705,
push straight to main with no branches, verify CI, then work the
open entries. Engine clauses run on host podman; stop the podman
machine at close-out.
```
