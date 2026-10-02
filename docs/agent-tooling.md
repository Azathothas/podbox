# Tool map

Use an installed tool before you add another implementation.
A name on PATH can resolve to a shim or an alias. Run its version or help command.

## Source discovery

Run `codegraph sync` in podbox before source discovery.
Ask `codegraph explore` for a code area.
Use `query` for a symbol and `node` for source and callers.
Read command help before an unfamiliar option.
Use `rg` after the index identifies the source.
Prose is not indexed source and can be searched directly.

Do not sync or initialize another repository during a read-only review.
Use its existing index, or index a captured copy inside podbox.
Verify source against the captured commit.

## Repository tools

| Tool | Purpose |
| --- | --- |
| podbox-dev session and check | Discover the host, build and check on Linux |
| [Windows job wrapper](../scripts/windows/run-in-base.sh) | Copy the checkout and run a Linux job |
| [lane proof runner](../scripts/dev-lane.sh) | Run one proof on the lane: lint, toolset, gc, evidence |
| [record gate](../crates/podbox-gate/src/main.rs) | Check task and document consistency |
| [count writer](../crates/podbox-gate/src/count.rs) | Derive task counts |
| [plant](../crates/podbox-gate/src/plant.rs) | Demonstrate gate refusals |
| [common checks](../scripts/common/) | Documents, markers, attribution, and paired checks. The secret-shape scan was retired; the [secrets workflow](../.github/workflows/secrets.yml) owns that subject |
| [doctor](../scripts/doctor/) | Environment report |
| [build state](../scripts/build-state.py) | An exec shim; the `podbox-buildstate` binary owns the logic |
| [document state](../scripts/document-state.py) | An exec shim; the `document-state` binary owns the logic |
| [experiment map](../experiments/README.md) | Repeatable runtime evidence |

## External tools

Use `gh` first for this repository's GitHub reads.
Use `curl` for a URL or a read-only proxy when that route fails.
The proxy requires curl's own user agent.
[Remote rules](security/remote-ops.md) govern both routes.

Use `wsl-toolkit --instance podbox` from PowerShell for manual base commands.
[Container procedures](containers.md) define the project wrapper.
The installed tool manual owns its flags.

On Windows, use `py` for Python.
Read [shell rules](conventions/shell.md) before a payload crosses shells.
[Historical tool changes](history/upstream-tool-shape.md) are evidence, not current instructions.
