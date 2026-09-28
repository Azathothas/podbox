## Task

Settle the relay questions with fresh sources, then record the answers.

## Resume point

Done after this commit. Next unit is T-1403: implement the relay
protocol split (multiplexed reverse for remote paths, one-pair for
local paths and tests) per TODO/podssh.md.

## In flight

Nothing half-done. Operator settled both questions: both protocols
split by use, and redacted relay logs may be committed. Fresh sources:
dropssh issue 9, the r12 relay documents, and a zero-residue pair
lifecycle driven from this host (no token value printed or stored).

## Tree state

Three doc files amended, gated, ready to commit. Full gate state from
the last change is green; only these doc lines are new.

## Paste

Continue the podbox SSH session: commit the settled relay answers,
push `main`, then start T-1403 with the multiplexed reverse protocol
from the r12 documents and the redial-pairing measurement from
TODO/podssh.md.
