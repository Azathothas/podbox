## The task

Continuous session of 2026-09-21. CI on main is green and kept job
containers older than 24 h are removed. T-0805 and T-0809 are on main.
T-0808 is implemented, proven by 325, and closing in this change. Next is
the authorised store-suite contention fix. Push straight to main, no
branches. Work unattended; the operator reads the result later.

## The resume point

T-0808 closes here (admit-first everywhere, 325 green at 153 rows and
192 driven, results committed). After the push, author the contention
entry under the T-0211/T-0215 family per the authoring methodology, then
implement it, in its own change. The podman machine runs now; stop it
again at close-out (it was stopped at session start).

## In flight

T-0808 code complete: `parity::admit_all` pre-pass in 18 parsers,
`rows_of` group arms with unit test, `experiments/325-parity-drive.sh`
(new, executable) green with the banner naming `-i` on both Stub runs,
`experiments/results/parity-drive.txt` committed. Entry, T-0801
correction, counts, and record update in progress. No half-written
implementation elsewhere.

## State (tree dirty, closing)

```text
T-0808 change: cli.md entries, INDEX.md counts, PROGRESS.md record,
parity.rs, images.rs, lifecycle.rs, system.rs, names.rs, main.rs,
325-parity-drive.sh, results/parity-drive.txt
```

Counts move to 32 open, 3 partial, 3 blocked, 99 done. Host gate green.
The Linux check runs before the commit; the store-suite contention flake
stays a named risk (three reds around T-0805, all the 16-slot signature).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
This is a continuous session: work until manually stopped, finishing as
many entries as possible. T-0808 is on main. Author the store-contention
entry under the T-0211/T-0215 family per the authoring methodology, then
implement it, in its own change. Push straight to main and create no
branches. The guest lane still has no docker daemon, so engine clauses
run on host podman; stop the podman machine again at close-out.
```
