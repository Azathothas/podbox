## Task

Start T-1404: add the remote SSH arm after the relay holds, per
TODO/podssh.md. Its entry holds the dispatch state and the proof.

## Resume point

T-1402 is done and committed locally on `main` with the record in the
same change: the server-side line discipline (`crates/podbox-ssh`),
the `shell` ForceCommand server, 12 interactive tests beside the unit
suite, three lane mutation reds, and the bounded 388 drive against a
real daemon with the interactive verdict. Next unit is the T-1404
remote arm, then the T-1112 guest run, in that order. Do not push:
the release sequence pushes once every entry is done.

## In flight

Nothing half-done. The session holds: echo, editing, capped history,
state, group signals with the trap-handler selective kill, exit codes,
exec refusal naming `SSH_ORIGINAL_COMMAND`, and the narrowed subsystem
split (exec refused by the shell, sftp refused by the daemon).
Node redial pairing stays unmeasured; writes and DNS stay without a
timeout on the relay legs, stated as a limit in the entries.

## Tree state

Committed locally, gates green: host strict 11 passed with no skip,
the lane suite green on the committed tree, `py scripts/check-todo.py`
green at 180 entries with 1 open. Collect kept lane jobs by ID after
their evidence is saved. The base cleanup report is empty.

## Paste

Continue the podbox SSH session: start T-1404 with the remote dispatch
from TODO/podssh.md.
