# Session summary, 2026-09-28

This table uses the clean starting commit `5701a8b` as its before state.
The after state is the work publication at `81fec0c` and the stated cutoff.

| Row | Before | After |
| --- | --- | --- |
| Elapsed | Start: 2026-09-28 05:27:58 UTC | Cutoff: 2026-09-28 11:16:12 UTC; 5 h 48 m 14 s by UTC clock subtraction |
| Commits | `5701a8b` | Five work commits through `81fec0c`; this summary is in the final record commit |
| Work | 171 entries: 0 open, 1 partial, 170 done | Five completed: T-1343 to T-1347; four SSH entries open: T-1401 to T-1404; one prior partial: T-1112 |
| Changes | Clean tree | 110 files; 16,201 lines added, 996 removed by `git diff` at cutoff |
| Size | Baseline `scc` count not measured | `scc`: 7,714 files, 3,109,461 lines at cutoff; Git diff line counts are in Changes |
| Checks | Record gate: 171 entries; host fast gate: 10 passed, 0 failed, 1 skipped | Record gate: 180 entries; host strict gate: 11 passed, 0 failed, 0 skipped; Linux check: 10 passed, 0 failed, 0 skipped; plant: 43 caught, 0 missed, 4 controls quiet; Windows drive matched |
| CI | `main` gate passed at `5701a8b` | Gate passed all four jobs at `81fec0c`; PR 67 has three failed checks, PR 66 has no checks and conflicts |
| Cost | Not measured | Not measured |
| Health | Tree clean; Windows base absent | Work publication tree clean; job ledger empty; base present; host Podman machine running as found; issue 68 closed |

The raw Windows drive is in
[`experiments/results/windows-lane-v6.txt`](../experiments/results/windows-lane-v6.txt).
The review is in
[`docs/history/reviews/2026-09-28-windows-lane.md`](../docs/history/reviews/2026-09-28-windows-lane.md).
