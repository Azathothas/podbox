## The task

Continuous session of 2026-09-22. T-0209 closed in the worktree,
uncommitted. Gate the whole tree, commit, push, verify CI, continue.

## The resume point

Tree clean at the T-0209 commit. Counts: 142 entries, 6 open, 1 partial,
2 blocked, 133 done. Next pool: image T-0207 (bounded concurrency),
packaging T-1003 (ladder, owns the vendored userland-execve `goblin`/`nix`
patch-out and the `fexecve` call), gate T-1207 (needs a ruling),
T-0413/T-1112 (need operator rulings: surface, do not implement),
T-0606.

## In flight

Nothing half-written. No lane job running. No kept job containers from
this stretch: every Linux step ran in disposable host-podman
containers (`--rm`), which leave nothing behind.

## State

Tree dirty with the T-0209 change only (uncommitted). Podman machine
`podman-machine-default` running, as found. The wsl-toolkit base is
still unusable (`getpwnam` fails for root and toolkit; `base ensure`
changes nothing); `base recreate` is not taken unasked.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0209 closed in the worktree, uncommitted. Gate
the whole tree, commit it whole, push straight to main with no
branches, verify CI by re-listing, then continue at T-0207 or T-1003.
Engine clauses run on host podman; leave the podman machine as found.
```
