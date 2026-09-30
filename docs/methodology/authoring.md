# Authoring

Use this procedure to turn a proposed change into a task.
[TODO rules](../../TODO/RULES.md) define this repository's entry format.

## Authorization

A planning intake authorizes analysis and a proposed plan.
It does not authorize implementation.
A direct repair or implementation request authorizes that work.
Use the operator's stated scope and standing decisions.

Do not ask for the same approval again.
Ask only for a missing decision that changes the required result.
Complete independent inspection while the decision is pending.

## Ground the task

Read the affected source through the code index.
Read existing tasks before you add another.
Verify claims against the current tree.
Measure a behavioral premise when a repeatable command can settle it.

Keep the title when a premise is disproved.
Write the correction beneath the premise.
Do not replace the original title to hide the finding.

## Scope and decisions

State the requested change and its acceptance clauses.
Name the existing path the change will use.
Name dependencies and checkpoints.
State remaining input and the action that supplies it.
Record a resolved choice once in the task.

Do not add a speculative abstraction for an absent caller.
Do not remove validation to shorten a required implementation.
Read [code rules](../conventions/code.md) and [patterns](../conventions/forbidden-patterns.md).

## Entry fields

Use Source, Category, Priority, Effort, Status, Problem, Premise,
Approach, Decision, and Prove.
A proof names a runnable command.
A comparison also needs a committed measurement script.

Use conditions that a fixture can arrange.
Do not require a scheduling order that the test cannot control.
Do not use a guessed wait duration as proof of readiness.

## Approval and implementation

For a planning-only request, present the scope, choices, tasks, and proof commands.
Wait for implementation authorization when that authorization is absent.
For an authorized repair, record the grounded task and proceed within scope.
The current operator request takes precedence over a generic separate-session rule.

Run the [gate](gate.md) for completed implementation.
Update the entry, index, and progress record with the change.
A partial task names the clauses that remain.

## Numbering

Use a free task ID.
Use `scripts/todo-count.py` to update status and counts.
Do not reuse an experiment number.
