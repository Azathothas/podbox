## The task

Continuous session of 2026-09-21. CI on main is green and kept job
containers older than 24 h are removed. T-0805 is implemented, proven on
the shipped binary, and closing in this change. Next is T-0809, then
T-0808, in listed order. Push straight to main, no branches. Work
unattended; the operator reads the result later.

## The resume point

T-0805 closes here (diagnose table, ownership note, sidecar example).
After the push, start T-0809: read the entry and the sandbox-insights wall
it cites first, then wire the spawn row. The podman machine runs now; stop
it again at close-out (it was stopped at session start).

## In flight

T-0805 code complete: `crates/podbox-cli/src/diagnose.rs` (new),
sidecar first-dropped example, shared report in `extract` and `run`.
Unit job green (71 cli, 46 extract, clippy clean). Proof job green
(pull 0, extract 0, note beside `gid 42`). Entry, counts, and record
update in progress. No half-written implementation elsewhere.

## State (tree dirty, closing)

```text
T-0805 change: cli.md entry, INDEX.md counts, PROGRESS.md record,
diagnose.rs, sidecar.rs, lib.rs, images.rs, main.rs, run.rs
```

Counts move to 34 open, 3 partial, 3 blocked, 97 done. Host gate and the
Linux check run before the commit. Temp lane scripts `t0805-job.sh` and
`t0805-proof.sh` go before the commit.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
This is a continuous session: work until manually stopped, finishing as
many entries as possible. T-0805 is on main. Work TODO/PROGRESS.md in
listed order from T-0809 (read the entry and its cited wall first).
The store-contention fix is authorised: author the T-0211/T-0215-family
entry, then implement it, in its own change. Push straight to main and
create no branches. The guest lane still has no docker daemon, so engine
clauses run on host podman; stop the podman machine again at close-out.
```
