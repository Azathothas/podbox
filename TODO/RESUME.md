## The task

Continuous session of 2026-09-21. CI on main is green and kept job
containers older than 24 h are removed. T-0805 is on main. T-0809 is
implemented, proven by 151, and closing in this change. Next is T-0808.
Push straight to main, no branches. Work unattended; the operator reads
the result later.

## The resume point

T-0809 closes here (spawn wall note in probe evidence, 151 green,
results committed). After the push, start T-0808: read the entry again
and build `./experiments/325-parity-drive.sh` per its Approach. The
podman machine runs now; stop it again at close-out (it was stopped at
session start).

## In flight

T-0809 code complete: `spawn_wall_note` in
`crates/podbox-probe/src/report.rs` with five leg-matrix tests,
`experiments/151-spawn-ambiguity.sh` (new, executable) with both halves
green and the control green, `experiments/results/spawn-ambiguity.txt`
committed. Entry, counts, and record update in progress. No half-written
implementation elsewhere.

## State (tree dirty, closing)

```text
T-0809 change: cli.md entry, INDEX.md counts, PROGRESS.md record,
report.rs, 151-spawn-ambiguity.sh, results/spawn-ambiguity.txt
```

Counts move to 33 open, 3 partial, 3 blocked, 98 done. Host gate green.
The Linux check runs before the commit; the store-suite contention flake
stays a named risk (two reds around T-0805, both the 16-slot signature).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
This is a continuous session: work until manually stopped, finishing as
many entries as possible. T-0809 is on main. Work TODO/PROGRESS.md in
listed order from T-0808 (read the entry first, then build 325 per its
Approach). The store-contention fix is authorised: author the
T-0211/T-0215-family entry, then implement it, in its own change. Push
straight to main and create no branches. The guest lane still has no
docker daemon, so engine clauses run on host podman; stop the podman
machine again at close-out.
```
