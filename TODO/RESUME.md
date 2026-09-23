## The task

Continue the batch from checkpoint `a299ab5`: lane-prove T-1335, close
T-1335 with proof plus client issue 16, then work the remaining TODO
order. Tag-gated entries wait on `v0.1.0-beta.6`.

## The resume point

Lane-prove first, one lane job at a time:
`PODBOX_ARTIFACTS=.tmp/pb-w35-out sh scripts/windows/run-in-base.sh
.tmp/pb-w35-prove.sh`. Then close T-1335.

## In flight

T-1335 implementation uncommitted in 5 files (`TODO/supervise.md`,
`crates/podbox-cli/src/parity.rs`, `crates/podbox-enter/src/lib.rs`,
`crates/podbox-probe/src/sys.rs`,
`crates/podbox-supervise/src/launcher.rs`): foreground
`wait_forwarding`, launcher serve signalfd, parity sentences, entry
Decision. Signalfd pointer bug fixed, never lane-proven.

## State

Tree dirty (5 files) at `a299ab5`. Gate green at the checkpoint, to be
re-verified this session before any commit.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Continue the podbox
batch from checkpoint a299ab5: lane-prove T-1335
(.tmp/pb-w35-prove.sh), close it with proof plus client issue 16,
then work the remaining TODO order. Work unattended; push straight
to main with no branches.
```
