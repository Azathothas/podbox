## The task

Continuous session of 2026-09-22. T-0206 closed and pushed (CI success).
Next: [T-0209](image.md), the registry credential UX against the T-0206
fixture's required-credential mode, or [T-1003](packaging.md), the launch
ladder.

## The resume point

Tree clean at the T-0206 commit. Counts: 142 entries, 7 open, 1 partial,
2 blocked, 132 done. Next pool: image T-0209 (credential UX) and T-0207,
packaging T-1003 (ladder, owns the vendored userland-execve `goblin`/`nix`
patch-out and the `fexecve` call), gate T-1207 (needs a ruling),
T-0413/T-1207/T-1112 (need operator rulings: surface, do not implement),
T-0606.

## In flight

Nothing half-written. No lane job running. Eight kept job containers
from this stretch await `wsl-toolkit gc` at close-out
(`6afbc73b91703705`, `6524c71a847bc5c6`, `96e01323ef22489a`,
`214d912c825e1b17`, `6dee3013d86c0076`, `23d32a28383609d6`,
`b114d62fe05235f4`, `dc9c0aab1b5cdddc` (re-list before collecting).

## State

Tree dirty with the T-0206 change only (uncommitted). Podman machine
`podman-machine-default` running. No lane job running. Eight T-0909 plus
new T-0206 kept job containers await `wsl-toolkit gc` at close-out
(re-list before collecting).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0206 closed in the worktree, uncommitted. Gate
the whole tree, commit it whole, push straight to main with no
branches, verify CI by re-listing, then continue at T-0209 or T-1003.
Engine clauses run on host podman; stop the podman machine at close-out.
```
