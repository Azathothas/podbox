## Task

Review podbox PRs 66 and 67 for the SSH entries, study faketty and fakepty
for the no-pty session entry, assess T-1112 KVM unlock under wsl-toolkit
6.0.0, then amend docs and implement the partial plus machine SSH side.
Remote and relay sides stay deferred.

## Resume point

Work is committed and the full gate is green. Remaining: refresh the
record (done above in PROGRESS.md), commit the record, push `main`,
write the session summary and the next prompt.

## In flight

Nothing half-done. Three commits carry the session: the landing, the
exit-code fix, and the openssh/full-check fix. No review ref remains.
The ValidationOS disk sits outside the tree at
`%USERPROFILE%\podbox-images\ValidationOS.vhdx`.

## Tree state

Dirty only with the record refresh. Full Linux gate green (10 passed,
0 failed); host fast gate green (9 passed, 0 failed, 2 environmental
skips); `check-todo`, markers, one-home, docs, and both secrets checks
green. Base job ledger empty.

## Paste

Continue the podbox SSH session: commit the refreshed record in
TODO/PROGRESS.md, push `main`, then print the summary table and the next
prompt per docs/methodology/sessions.md. T-1403 is next in the work
order, with the protocol choice as the operator question.
