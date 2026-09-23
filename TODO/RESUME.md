## The task

Session of 2026-09-23, continued. Triaged nineteen client-filed issues
(10-28) plus the operator-ordered `podbox man` verb and ASCII-only CLI
scrub: 22 entries T-1315 to T-1336 filed across ten TODO files with
every premise confirmed live, signing identity ruled keyless Sigstore.
Committed and pushed as `4d49337`. Counts: 165 items, 23 open.

## The resume point

Implement. Next session works T-1112 (unparked, P3) and T-1315 to
T-1336 in entry order, closes each issue with proof, and publishes the
next beta. Nothing is blocked.

## In flight

Nothing half-written. Tree clean at `4d49337`, gate green.

## State

Tree clean. CI gate on the triage commit to be confirmed by the next
session at start (`gh run list`). Base lane teardown complete
(dockerd stopped, scratch removed, job ledger empty); kept packages
documented in PROGRESS.

## Standing operator rulings, 2026-09-22 and 2026-09-23

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
- Publishing is authorized once the top-10 priority tasks finish and the
  session ends, under a pre-release tag. Work first. Betas so far:
  beta.1 through beta.5, each nightly-published with fourteen assets
  verified back.
- T-1112 unparked 2026-09-23: work it next, stays P3.
- Nightly signing: keyless via Sigstore, OIDC from the publish job.
- `podbox man` models `wsl-toolkit man --no-pager`; CLI output carries
  no emoji and no markers on any path.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
podbox is a container runtime for agent sandboxes that answers to docker
and podman. Read TODO/PROGRESS.md first, then TODO/RESUME.md.
Implement T-1112 and T-1315 through T-1336 in entry order; close each
client issue with a comment proving the fix and naming the recurrence
guard; publish the next beta and verify its assets back. Work
unattended; push straight to main with no branches.
```
