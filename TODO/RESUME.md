## Task

Review podbox PRs 66 and 67 for the SSH entries, study faketty and fakepty
for the no-pty session entry, assess T-1112 KVM unlock under wsl-toolkit
6.0.0, then amend docs and implement the partial plus machine SSH side.
Remote and relay sides stay deferred.

## Resume point

Docs amended and gated next. Then implement: T-1401 partial
(`podbox-ssh` transport plus server, no relay, no remote group) with the
three red-job fixes, plus the T-1404 machine arm. Machine-arm design needs
the Linux machine driver facts first.

## In flight

Doc amendments done in the tree, gate not yet run. Corpus gained
`references/dtolnay__faketty` and `references/sigoden__fakepty`, both at
the studied pins with zero fetch gaps. `experiments/385-kvm-open.sh`
holds. ValidationOS disk verified at
`%USERPROFILE%\podbox-images\ValidationOS.vhdx`. Review ref
`refs/review/pr67` still present for the implementation; delete it before
any commit.

## Tree state

Dirty with the amendment set, uncommitted. Gate run is the next command.

## Paste

Continue the podbox SSH session: run `py scripts/check-todo.py` and the
host gate, then implement the T-1401 partial and T-1404 machine arm per
TODO/PROGRESS.md and TODO/podssh.md. Remote and relay sides stay
deferred. Delete `refs/review/pr67` before any commit.
