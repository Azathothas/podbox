## The task

Continuous session of 2026-09-22. T-0909 closed and pushed (`fb9fc31`,
CI success). Next: [T-0206](image.md), the loopback registry fixture.

## The resume point

Tree clean at `fb9fc31`. Counts: 142 entries, 8 open, 1 partial,
2 blocked, 131 done. Next pool: image T-0206 (fixture over `zot`;
its `Decision` still names `registry:2` and is amended in the same
change that writes the 180 script), T-0207, T-0209, T-1003 (ladder),
gate T-1207 (needs a ruling), T-0413/T-1207/T-1112 (need operator
rulings: surface, do not implement), T-0606.

## In flight

Nothing half-written. No lane job running. Eight kept job containers
from this stretch await `wsl-toolkit gc` at close-out
(`6afbc73b91703705`, `6524c71a847bc5c6`, `96e01323ef22489a`,
`214d912c825e1b17`, `6dee3013d86c0076`, `23d32a28383609d6`,
`b114d62fe05235f4`, `dc9c0aab1b5cdddc` (re-list before collecting).

## State

Tree clean at `fb9fc31`, CI success verified by re-list. Podman
machine `podman-machine-default` running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0909 pushed (fb9fc31, CI green) and the tree is
clean. Continue at T-0206 (zot fixture; amend the registry:2 Decision
in the same change as the 180 script). Push straight to main with no
branches. Engine clauses run on host podman; stop the podman machine
at close-out.
```
