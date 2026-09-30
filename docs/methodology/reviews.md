# Reviews

Review every file the session changes.
Use three distinct questions.

## 1. Path review

Which entry points reach the changed behavior?
Use the code index to list callers.
Check sibling commands, defaults, and refusal paths.
Check composed behavior, resource ownership, and cleanup.

For documents, check every linked procedure and its actual entry point.
A document that describes an unreachable path is a defect.

## 2. Failure review

Can the changed guard fail for its intended reason?
Plant the defect.
Check the plant's own message and exit code.
Restore the tree and verify a clean control.

Check that a test name matches its assertion.
Check that another failure cannot satisfy the expected refusal.
A new repository check needs its plant in the same change.

## 3. Evidence and resume review

Does each claim match source or a repeatable result?
Check versions, counts, conditions, publication state, and current refs.
Separate current state from historical results.

Can a fresh session proceed from tracked files?
Verify its input paths, command, acceptance clauses, and cleanup.
Remove requirements for session-local files.
Ensure that the work order has one home.

## Record

Name the files and paths each pass reviewed.
Name the findings and their fixes.
For an empty result, name the condition that would have produced a finding.
Record remaining limitations in their task or current limits page.
Do not label an unperformed review as complete.

## Mechanical checks

Let the gate check links, counts, references, generated fields, and characters.
Spend the reading on behavior and evidence.
Read [the gate procedure](gate.md) before completion.
