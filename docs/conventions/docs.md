# Document roles

[Prose](prose.md) defines the writing style.
Source and verified runs establish behavior.

## Current documents

| Document | Owns |
| --- | --- |
| `AGENTS.md` | Session entry point and required reading |
| `README.md` | Product entry point and build start |
| `HUMAN.md` | Operator procedure and supplied input |
| `SECURITY.md` | Trust model and report route |
| `THIRD_PARTY.md` | Carried-source notices and patch state |
| `TODO/PROGRESS.md` | Current baseline, verification, and the only work order |
| `TODO/RESUME.md` | Unfinished work and the next action |
| `TODO/INDEX.md` | Task rows and generated counts |
| `TODO/RULES.md` | Repository work procedure |
| `TODO/reference-map.md` | Reference provenance, licenses, and permitted use |
| `docs/architecture.md` | Runtime paths and invariants |
| `docs/code-map.md` | Source ownership |
| `docs/limits.md` | Current constraints and proof gaps |
| `docs/runtime-state.md` | Generated source declarations |
| `CHANGELOG.md` | Shipped changes and publication state |
| `experiments/results/` | Captured measurements |
| `docs/history/` | Superseded wording, research depth, and reviews |

Do not put a work order in a second page.
Do not use a history page as a current instruction.
A task can describe its own remaining acceptance clauses.

## Truth and drift

When source or a run conflicts with a document, correct the document.
When the code fails a requirement, record and fix the code defect.
A technical page does not take precedence over contrary runtime evidence.

Update the affected documents with the implementation.
Use links to the primary fact.
Use generated fields for manifest values, mechanism names, and counts.
Add a plant when a new gate holds a document field.

Each page needs an incoming link.
Each source citation needs a revision where the tree can move.
Avoid line citations into a live file unless the gate checks them.

## Historical material

Move needed superseded wording to [history](../methodology/history.md).
Keep the evidence that explains a changed decision.
Remove repeated session narrative from current pages.
Git commit `005638d` on main holds the complete document set before the
2026-09-30 audit. Read an earlier page with `git show 005638d:PATH`.

## Changelog

Keep entries newest first.
Give each heading a date.
Name the task, result, or commit that supports the entry.
State whether a version changed and whether publication occurred.
Keep published entries. Amend an incorrect claim in place.
The [changelog check](../../scripts/common/check-changelog.sh) holds its mechanical fields.
