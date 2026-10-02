# AGENTS.md

podbox runs Linux payloads where namespaces, mounts, devices, or ownership
changes can be denied. It also has a machine tier and SSH transport.
It measures operations and reports the mechanism it enters.

## Start each session

Run this first from the repository root:

```sh
cargo build --release -p podbox-gate
./target/release/podbox-dev session
```

The script selects the host procedure. On Windows, the instance is `podbox`.
Read these files in order:

1. [TODO/PROGRESS.md](TODO/PROGRESS.md): current state and the only work order.
2. [TODO/RESUME.md](TODO/RESUME.md): unfinished work and the next action.
3. [Session procedure](docs/methodology/sessions.md): start, evidence, and close.
4. The task entry and the documents in the routing table below.

Run the record gate before edits. Read each process exit code.

```sh
./target/release/podbox-gate                 # Windows
sh scripts/dev-lane.sh run JOB.sh --artifacts DIR  # Windows: full Linux proof
./target/release/podbox-gate                  # Linux
./target/release/podbox-dev status      # Linux: build state
```

The record gate reports status consistency. It does not prove that a feature
works. Check the task's acceptance command and its evidence.

**On Windows, `cargo build`, `cargo test`, `cargo run` and `cargo bench`
build a Linux target and cannot work here.** `.cargo/config.toml` sets
`[build] target = "x86_64-unknown-linux-musl"`, so the target is Linux on
every host. Run those through the lane instead:

```sh
sh scripts/dev-lane.sh run JOB.sh --artifacts DIR
```

`cargo check`, `cargo clippy` and `cargo fmt` are correct on the host;
they never link. `os error 193` from `scripts/zig-cc.sh` is not a host
limitation, it is the wrong machine: the same build finished in 55 s
through the lane on 2026-10-02. T-1642,
`scripts/common/check-build-lane.sh`, refuses the wrong one.

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
| Add a check | [Gate](docs/methodology/gate.md); `./target/release/podbox-plant` |
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
- Use `./target/release/podbox-count` to derive counts. Do not edit counts by hand.
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
