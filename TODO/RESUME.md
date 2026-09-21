## The task

Continuous session of 2026-09-21. T-1310 is done and on main. Next is T-0408
(zypper row: run and record). Push straight to main, no branches. Engine
clauses run on host podman; stop the podman machine at close-out.

## The resume point

T-0408: fetch a musl podbox binary of the current tree (Linux job, via /out),
run the opensuse-leap sweep row on host podman, record the transcript under
the entry, close it.

## In flight

T-0408 implementation open. Build job for the musl binary next. No
half-written code. Tree clean at `ccd1554` (verify at resume).

## State (tree clean, T-1310 landed)

```text
ccd1554 image: serialise the store tests and pin the sixteen-slot ceiling (T-1310)
f9aa0bb image: author the store-contention entry under T-0211/T-0215 (T-1310)
```

Counts 138: 32 open, 3 partial, 3 blocked, 100 done. Host reader green.
Linux gate green in the T-1310 job. CI state: re-check at resume.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1310 is on main, T-0408 (zypper row) is next.
Fetch a musl podbox binary of the current tree in a Linux job, run the
opensuse-leap sweep row on host podman, record the transcript under the
entry and close it. Push straight to main, no branches. The guest lane
has no docker daemon; engine clauses run on host podman.
```
