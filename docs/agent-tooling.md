# Tool map

Use an installed tool before you add another implementation.
A name on PATH can resolve to a shim or an alias. Run its version or help command.

## Source discovery

Use CodeGraph before a source search when a `.codegraph/` exists.
Run `codegraph sync`, then ask `codegraph explore` for a code area.
Use `query` for a symbol and `node` for source and callers.
Without an index, use `rg` and the [code map](code-map.md).
Read command help before an unfamiliar option.
Prose is not indexed source and can be searched directly.

The mechanism of most runtime parts is in a `pod-*` crate of
PodUtils/podcrates. Cargo keeps its source under
`~/.cargo/git/checkouts/` at the commit that `Cargo.lock` names.
Read it there; do not edit it there.

Do not sync or initialize another repository during a read-only review.
Use its existing index, or index a captured copy inside podvm.
Verify source against the captured commit.

## Repository tools

The workspace builds the target `x86_64-unknown-linux-musl`
(`.cargo/config.toml`), so a release binary is under
`./target/x86_64-unknown-linux-musl/release/`.

| Tool | Purpose |
| --- | --- |
| [cloud setup](../scripts/cloud-setup.sh) | Install zig and the toolchain, and report the host |
| `podvm-dev` `session`, `build`, `check` | Discover the host, build and check on Linux |
| [record gate](../crates/podvm-gate/src/main.rs) | Check task and document consistency |
| [count writer](../crates/podvm-gate/src/count.rs) | Derive task counts |
| `podvm-todo check` | pod-todo's reader of the record, with the facts of `TODO/todo.toml` |
| [plant](../crates/podvm-gate/src/plant.rs) | Demonstrate gate refusals |
| [common checks](../scripts/common/) | Documents, markers, attribution, the changelog and the other shell checks of the tree. The [secrets workflow](../.github/workflows/secrets.yml) owns the credential scan |
| [doctor](../scripts/doctor/) | Environment report |
| [build state](../scripts/build-state.py) | An exec shim; the `podvm-buildstate` binary owns the logic |
| [document state](../scripts/document-state.py) | An exec shim; the `document-state` binary owns the logic |
| [experiment map](../experiments/README.md) | Repeatable runtime evidence |
| [Windows job wrapper](../scripts/windows/run-in-base.sh) | Parked lane: copy the checkout and run a Linux job |
| [lane proof runner](../scripts/dev-lane.sh) | Parked lane: run one proof with lint, toolset, gc, and evidence |

## External tools

Use shell tools (`git`, `curl`, `cargo`, `gh`) before MCP tools.
For a read-only query of GitHub, use a mirror that `AGENTS.md`, section 4, names.
Dispatch a workflow of this repository with the GitHub tool
`actions_run_trigger` (`AGENTS.md`, section 9, item 5).
[Remote rules](security/remote-ops.md) govern each route.

For the parked Windows lane, use `wsl-toolkit --instance podvm` from
PowerShell and `py` for Python.
[Container procedures](containers.md) define the project wrapper.
Read [shell rules](development.md#shell) before a payload crosses shells.
[Historical tool changes](history/upstream-tool-shape.md) are evidence, not current instructions.
