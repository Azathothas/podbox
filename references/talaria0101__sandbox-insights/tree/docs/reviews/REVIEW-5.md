# REVIEW 5 — final gate: mechanical checks scripted, full live re-execution

Date: 2026-09-10. Method: run the mechanical half as checks (citation
resolution, header presence, licence, tree contents), then wipe
`work/` and `experiments/logs/` and re-execute all eleven experiments
from a clean state, exit codes read unpiped, orphans counted after.

## Mechanical half (scripted, all clean)

- every experiment/log citation in the docs resolves to a committed
  log (`logs/NN-*.log` exists for every `NN-` named);
- every experiment script opens with its question header (inputs,
  tools, exit codes);
- licence present and 0BSD; `.gitignore` keeps `work/` out of the
  tree; committed tree is 300 KB-scale (lean);
- no internal doc link is broken;
- the `scripts/ldpreload/` fixture gained the README its build lines
  deserved (found by this pass: the instructions existed only in a C
  comment).

## Live re-execution (clean state)

| experiment | rc |
|---|---|
| 10 census | 0 |
| 15 network | 0 |
| 20 attribution | 0 |
| 25 ownership wall | 0 |
| 30 Go spawn matrix | 0 |
| 35 interposition reach | 0 |
| 40 namespace gap | 0 |
| 45 senotif primitives | 0 |
| 50 TCG boot | 0 |
| 55 driven guest + bench | 0 |
| 60 chroot appliance | 0 |

Zero qemu orphans after the sweep. All final numbers in the documents
re-checked against these definitive logs; the chroot pairing (now
817.1 host vs 880.0 chroot Mops/s, checksum identical) replaced the
previous pairing in all four surfaces that cite it — the last of the
drift class REVIEW-2 found.

## Closing verdict

The repository's claims are backed by logs a reader can re-run; the
instruments carry their own controls; the router routes; the paper
opens its limitations with what it has *not* established. What
remains unknown is what the tree says remains unknown: the filter's
unprobed entries, tomorrow's policy revision, and the generalisation
of one machine's numbers — all stated where the claims are made.
Assume more remain.
