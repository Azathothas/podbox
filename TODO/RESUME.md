## The task

Continuous session of 2026-09-22. T-0705/T-0707 done, pushed, CI-green.
T-0708 (emulate mknod/mount/unshare/clone with memo tally) implemented in
the worktree plus the T-0810 fix (mkdir in `supervise::create`). Drive
both, close both, push straight to main with no branches. Engine clauses
run on host podman; stop `podman-machine-default` at close-out.

## The resume point

Lane full check on the final tree first (`run-in-base.sh`, one job at a
time). Then rebuild the shipped binary (`.dev/t1309-build-pb.sh`, ELF>=3,
verify strings), re-drive T-0708 E1-E8 with E4 rewritten as a direct
`clone()` row (python ctypes, no toolchain in payload), 240 plus
t0705/t0707 regressions, write the T-0810 Done and the T-0708 Prove
amendment plus Done, host gates, commit each entry alone, push, verify
each CI run by re-listing.

## In flight

T-0708 plus T-0810-fix in the worktree, uncommitted: M
`crates/podbox-interpose/{interpose.map,src/lib.rs,src/memo.rs,src/real.rs}`,
?? `src/emulate.rs`, M `crates/podbox-cli/{src/interpose.rs,src/lifecycle.rs}`,
M `crates/podbox-supervise/src/{table.rs,lib.rs}`. One comment fix applied
(`ps` does not read the Emulated names). TODO text untouched: both entries
still open, amendments owed at close. `.dev/t0708-drive.sh` E4 still the
old fork row, rewrite owed before the drive.

## State

Tree dirty, 8 modified plus 1 untracked. Host `check-todo.py` green
(RC=0, read from the process). Podman machine `podman-machine-default`
running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0708 is implemented in the worktree with the T-0810
fix beside it. Lane-check the final tree, drive E1-E8 with E4 as a direct
clone row, regress, close both entries, push straight to main with no
branches, verify CI. Engine clauses run on host podman; stop the podman
machine at close-out.
```
