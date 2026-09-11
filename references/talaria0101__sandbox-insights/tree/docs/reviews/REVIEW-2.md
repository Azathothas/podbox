# REVIEW 2 — the claim audit: every published sentence against the logs

Date: 2026-09-10. Method: for every number and verdict in `README.md`,
`docs/environment.md`, `docs/paper.md`, `docs/walls.md` and
`docs/recipes.md`, open the committed log it cites and check the
sentence against it; then follow every internal link. The finding
class this pass exists for: **document drift** — a number true when
written, false next to the log a reader actually opens.

## Findings

| # | finding | resolution |
| --- | --- | --- |
| 1 | **TCG numbers had drifted in three documents at once.** The boot was written as `2.04 s` (an earlier log), the final log reads `5.56 s` on a loaded host — both real, neither was the other. The TCG tax was written as `3.2× / 12×` from one pairing while later committed pairings read near-native for the integer bench (835 vs 866 Mops/s) and 6.9× for the FP chain. A reader comparing the README against the committed log would have found the documents wrong. | every TCG number restated as the range across committed runs with the load caveat in the same sentence (README, paper §6.1, recipes); the paper now states explicitly that the tax is workload- *and run-shaped*, and that a tight register-only loop can run near native under TCG's code cache — which the drift itself demonstrated |
| 2 | **The chroot pairing cited stale values** (720.0/720.3); the final log reads 713.9/718.4. | synced in README, paper, recipes |
| 3 | **`docs/environment.md` overreached the measured family**: "UDP bind allowed" rested on inet4 probes only. | IPv6 probed: `udp6 ::1` binds, `tcp6 ::1` denies — the family-aware claim now covers both, backed by the probe rather than extrapolation |
| 4 | Paper §6.1 said the driver "kills qemu after the marker" while `50-` still measured wall-to-timeout — the text described a fix the instrument had not adopted. | instrument adopted it (REVIEW-1 #7); text and log now agree |
| 5 | The experiments README's status column predated the final logs (it described the 55- flake era). | statuses re-checked against the committed logs; each entry now states what the current log shows |
| 6 | Link check: every `docs/` and README link resolves; every experiment number cited in prose exists in `experiments/`. | clean; no action |

## The audit's verdict on the corpus

Every load-bearing claim in the tree now names a log that contains it,
and the logs carry their conditions. The residual risk is the one §12
of the paper names: the contract moves, and the documents state dates
— re-run the census before trusting any row.

## What would have made this pass fire sooner

A script that extracts every `NNN` experiment citation and every
`logs/` path from the docs and checks they exist — mechanical, cheap,
and it would have caught finding 4 in a second. Adopted as practice
for the next pass (REVIEW-5 runs it).
