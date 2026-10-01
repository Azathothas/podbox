# Gate

Completion requires automated checks, a relevant driven proof, and review.
Each part detects a different defect class.

## Automated checks

On Linux:

```sh
./target/release/podbox-dev check
```

On Windows:

```sh
./target/release/podbox-gate
sh scripts/common/check-gate.sh --strict
sh scripts/windows/run-in-base.sh
```

The Linux check builds the interposer before the CLI.
It runs formatting, lint, workspace tests, interposer tests, and repository checks.
The host strict check also exercises the available PowerShell halves.
[Containers](../containers.md) gives job transport and collection.

Read each process exit code without a pipe.
A skip is not a pass.
Report a missing capability and the command that can supply it.
Inspect failures even when another part succeeds.

## Driven proof

Run the changed surface as a caller would use it.
Use the real binary and the appropriate host or guest.
Assert output, state, and exit code.
Keep deterministic fixtures for error paths.

A unit test does not prove a remote service or a guest.
A historical result does not prove an edited binary.
A source declaration does not prove a reachable command.

For a document change, execute the documented procedure that changed.
Verify every changed behavioral claim against its source and relevant result.
Read [experiments](experiments.md) for captured evidence.

## New checks

Add the check and its plant in the same change.
The plant must change its subject.
It must report the check's own failure message.
It must restore the tree.
It must pass on the clean control.
Run the full `./target/release/podbox-plant` suite after a check change.

## Reviews

Run the three distinct passes in [reviews](reviews.md).
Record each scope, finding, and correction.
A pass can find no defect; state its scope and failure condition.
Do not manufacture findings or repeat one review as three passes.

## Completion

Record the proof command, conditions, exit, and result.
Update the task, counts, current record, and affected documents together.
A remaining acceptance clause keeps the task partial.
A named external dependency can keep it blocked.
The gate checks recorded consistency; the proof establishes behavior.
