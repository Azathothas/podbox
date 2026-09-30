# Measurements

A reported measurement needs a tracked script.
Keep each script in experiments with a unique number.
Do not reuse a number when replacing a script.

Print the question, pinned inputs, host, tool versions, UTC time, and scope.
Resolve inputs explicitly. Do not depend on the caller's current directory.
For Windows job inputs, the wrapper enters /work before the caller runs.
State any required binary or licensed image path as an argument.

Use exit 0 for a match, 1 for a tested mismatch, and 2 when the proof
could not run. Preserve the process status without a pipeline.
A missing observation is not a denial or a zero.
Save negative results with the same conditions as successful results.

Store proof under experiments/results. Keep original measured values.
State any private-path substitution.
Remove owned scratch only after its result is saved.
Do not remove the saved result.

Use an independent observer when the subject's report is insufficient.
For example, inspect the emulator arguments to prove KVM selection.
Check whether the observer changes the operation.
Use a positive control and a failure control where the instrument can
otherwise pass without reaching its subject.

A result applies to its host and conditions.
It does not establish a universal capability or performance value.
A historical result remains historical after a new build.
Do not repeat a proof without a changed input or an unresolved condition.

[references.md](references.md) owns study of another source tree.
[history.md](history.md) owns superseded explanations.
