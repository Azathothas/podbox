# AGENTS.md

podbox runs Linux payloads where namespaces, mounts, devices, or ownership
changes can be denied. It also has a machine tier and SSH transport.
It measures operations and reports the mechanism it enters.

## Start each session

Run this first from the repository root:

```sh
sh scripts/session-start.sh
```

The script selects the host procedure. On Windows, the instance is `podbox`.
Read these files in order:

1. [TODO/PROGRESS.md](TODO/PROGRESS.md): current state and the only work order.
2. [TODO/RESUME.md](TODO/RESUME.md): unfinished work and the next action.
3. [Session procedure](docs/methodology/sessions.md): start, evidence, and close.
4. The task entry and the documents in the routing table below.

Run the record gate before edits. Read each process exit code.

```sh
py scripts/check-todo.py                 # Windows
sh scripts/windows/run-in-base.sh       # Windows: full Linux check
./scripts/check-todo.py                  # Linux
./scripts/dev.sh status                  # Linux: build state
```

The record gate reports status consistency. It does not prove that a feature
works. Check the task's acceptance command and its evidence.

## Task routing

Read each named page in full. Read the union when two rows apply.

| Task | Required reading |
| --- | --- |
| Windows or a container | [Container procedures](docs/containers.md) |
| Hosted or temporary machine | [Hosted sessions](docs/hosted-sessions.md) |
| Find source or a caller | [Tool map](docs/agent-tooling.md) |
| Product contract | Captured `TOOL.md` sections 3, 4, and 6 in `references/Azathothas__container-research/tree/`; [Architecture](docs/architecture.md); [Limits](docs/limits.md) |
| Implement a task | Its `TODO/` entry; [Gate](docs/methodology/gate.md); [Code](docs/conventions/code.md); [Patterns](docs/conventions/forbidden-patterns.md) |
| Author work or fix a defect | [Authoring](docs/methodology/authoring.md); [TODO rules](TODO/RULES.md); affected source |
| Take a measurement | [Experiments](docs/methodology/experiments.md); a related experiment |
| Add a check | [Gate](docs/methodology/gate.md); `scripts/plant.sh` |
| Study another repository | [References](docs/methodology/references.md); [Reference map](TODO/reference-map.md) |
| Change carried source | [Vendoring](docs/methodology/vendoring.md); [Third-party notice](THIRD_PARTY.md) |
| Edit a document | [Prose](docs/conventions/prose.md); [Document roles](docs/conventions/docs.md) |
| Commit | [Git](docs/conventions/git.md) |
| Shell or quoting | [Shell](docs/conventions/shell.md) |
| Remote operations | [Remote rules](docs/security/remote-ops.md) |
| Credentials | [Secret rules](docs/security/secrets.md) |
| Superseded text | [History](docs/methodology/history.md) |
| Change the work model | [Work model](docs/methodology/work-todo.md); [Authoring](docs/methodology/authoring.md) |
| Close the session | [Sessions](docs/methodology/sessions.md); [Reviews](docs/methodology/reviews.md) |

## Repository rules

- Work on `main`. Use the configured `origin` for authorized publication.
- Every other repository is read-only. Fix carried source in this tree.
- Use CodeGraph before a source search. Prose can be searched directly.
- On Windows, use `wsl-toolkit --instance podbox`. Do not call `wsl.exe`.
- Read the linked rule and current source. A previous record is a claim to verify.
- Run a task's proof before completion. Record partial work with its remaining clauses.
- Change the task, index, and progress record with the implementation.
- Use `scripts/todo-count.py` to derive counts. Do not edit counts by hand.
- Give each experiment a unique number. Keep its script and result.
- Record an unknown value as a dash. Label an estimate each time it appears.
- Read exit codes without a pipe. Bound network and process waits.
- Keep credentials out of source, output, and commit messages.
- Attribute commits and release notes to the operator alone.
- Write ASD-STE100 text. Use technical names, short sentences, and named actors.
- Correct live text in place. Keep needed superseded evidence in `docs/history/`.
- Add a plant with each new check. A green check without a failure test is insufficient.

## Session close

Complete the operator's requested scope. If the operator stops the session,
save the current state and a precise resume point.

Run three distinct reviews. Run the full host and Linux checks.
Collect this session's ended jobs after saving their evidence.
Keep the base and restore any host engine state that the session changed.
Save and print the measured summary. Print the next prompt in chat.

[TODO/RULES.md](TODO/RULES.md) owns the repository procedure.
[The history index](docs/history/README.md) points to superseded records.
