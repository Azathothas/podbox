## Task

Start T-1402: provide and prove an interactive session without a pty,
per TODO/podssh.md. A one-shot command is not the proof.

## Resume point

T-1403 and T-1401 are done and committed on `main` with the record in
the same change. Next unit is T-1402, then the T-1404 remote arm, then
the T-1112 guest run, in that order.

## In flight

Nothing half-done. The relay holds: multiplexed `reverse-v1` in
`crates/podbox-ssh` with the fake-relay suite, three live mutation
proofs, and the bounded two-client drive against the live r12 relay.
Node redial pairing stays unmeasured; writes and DNS stay without a
timeout on the relay legs, stated as a limit in the entries.

## Tree state

Committed, clean, gates green: host strict 11 passed with no skip, the
lane suite green on the committed tree, `py scripts/check-todo.py`
green at 180 entries with 2 open. The base cleanup report is empty.

## Paste

Continue the podbox SSH session: start T-1402 with the session layer
above the byte transport and the refusal catalogue from TODO/podssh.md.
