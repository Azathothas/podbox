# Work model

podbox uses one task model.
[authoring](authoring.md) defines entry fields.
[TODO rules](../../TODO/RULES.md) define project procedure.

| File | Role |
| --- | --- |
| TODO/INDEX.md | Entries sorted by id and derived counts |
| TODO/PROGRESS.md | Current state and the only work order |
| TODO/RESUME.md | Exact unfinished state and cold-start next action |
| TODO/category.md | Original requests, status, acceptance, and closure evidence |

Use open for unimplemented acceptance.
Use partial for implemented work with remaining clauses.
Use blocked when a named external condition prevents progress.
Use done only after the full stated acceptance is proved.
A historical Done paragraph does not override the current status.

Keep the title when a premise changes.
Correct current text in place and retain required earlier evidence in history.
Do not treat a task's captured Problem as a claim of current source absence.

Set status through podbox-count and run the independent record gate.
The task, index, progress, and implementation belong in the same change.
Do not create a second work order in another page or kickoff prompt.

Use priorities and effort classes as defined in INDEX.
A scope change needs an explicit acceptance record.
Do not close an incomplete item by changing its meaning silently.
Leave the remaining clauses in that item or a linked, open successor.

[sessions](sessions.md) owns start and closure.
[reviews](reviews.md) owns the three review questions.
