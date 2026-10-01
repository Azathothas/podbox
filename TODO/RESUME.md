# Resume

Assignment complete 2026-09-30: every open issue is closed with evidence,
main is pushed, CI is green, and beta.12 is published and exercised.

Tree state: `main` at `1589480`, clean. Tags `v0.1.0-beta.11` (superseded:
binary reports beta.10) and `v0.1.0-beta.12` (current) both point at
CI-green commits. Records: 203 entries, 0 open, 2 partial (T-1350 and
T-1112, KVM-bound), 201 done. No kept lane jobs; the ledger is empty.

Recovery: `git log --oneline -3` shows the closure batch, the plant fix,
the version bump, and the release records. The record-gate binary
(`podbox-gate`) exits 0. Fresh proof re-runs from the tracked scripts in
`experiments/`, never from `.tmp/`.

Next: an operator-present session runs the KVM guest (`sh
experiments/392-kvm-guest.sh --accept-host-risk --binary BINARY --image
IMAGE`), then the ReactOS route. Do not start a KVM guest unattended.
The work order is in TODO/PROGRESS.md alone.
