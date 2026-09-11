# REVIEW 4 — the reader-routing pass: a stranger works from AGENTS.md alone

Date: 2026-09-10. Method: read `docs/AGENTS.md` as an implementer with
no prior context and act on it — follow every row of the routing
table, run the start-here commands and the gate, and check that
everything the router promises exists where it promises it. A router
that restates nothing but names everything is only good if the names
resolve.

## Findings

| # | finding | resolution |
| --- | --- | --- |
| 1 | **The experiments README had the same document drift** REVIEW-2 caught in the docs (the `2.04s` boot, the stale chroot pairing) — the router routes readers there first for statuses. | synced; the lesson recorded: drift fixes must sweep *every* reader-facing surface, not just the paper — there are four places a number lives (README, experiments README, topic doc, paper) and a fix that touches one has touched one |
| 2 | The gate names three experiments; all three exist, are executable, and pass unpiped. The start-here census runs in ~2 s and needs no network. | clean |
| 3 | The routing table's eight task rows each resolve to a real file, and each target opens with the answer to the question the row asks. | clean |
| 4 | The absolutes say the census must be re-run "before trusting any claim" — but nothing told the reader what *changed* looks like. | `experiments/README.md` now closes with the explicit instruction: re-run the census, compare against `docs/environment.md`, amend in the same change where the host disagrees |
| 5 | The tree table in AGENTS.md described `work/` as "never committed" — true only because `.gitignore` says so. A reader cannot see that from the router. | `.gitignore` mention added to the router's tree table |
| 6 | The experiments table's status column cites specific numbers (bench speeds, boot time) that are *the current run's* — a future re-run makes them stale by design. Each row now reads as a description of the committed log, and the conditions rule (§ absolutes #5) covers the rest. | wording audited |

## What a stranger can now do

Run two commands and have the environment contract re-derived in under
a minute; follow any task row to a self-contained document; find every
instrument the paper cites in `scripts/` with its experiment number in
the file header. The one thing the router still cannot do for a
stranger is distinguish a stale status from a false one — that is
what the census-first rule is for, and it is the first thing the file
says.
