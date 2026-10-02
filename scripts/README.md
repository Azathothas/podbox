# Scripts

Run podbox-dev session first (after `cargo build --release -p podbox-gate`).
It reports the host and selects the lane.

| Script | Purpose |
| --- | --- |
| podbox-dev session | Host report, CodeGraph update, and lane setup |
| podbox-dev | Linux build, byte-based status, and full development check |
| windows/run-in-base.sh | Copied-workspace Linux job through the podbox base |
| dev-lane.sh | The one lane proof runner: job lint, toolset, gc with ledger check, evidence assert |
| build-state.py | Input and output content checks |
| document-state.py | Source-derived document and mismatch check |
| podbox-gate | Task, citation, source-state, and cleanup gate |
| podbox-count | Status and count writer |
| build-interpose.sh | Interpose ceiling declaration; the build is podbox-interpose-build |
| package-ssh.sh | Static helper smoke and release archive |
| release-licenses.py | Retained licence texts for locked Cargo packages |
| podbox-smoke | Architecture and image-path release smoke |
| release-notes.sh | Exact-commit green gate and build boundary |
| verify-release.sh | An exec shim; `podbox-verify` owns the signature check |

Use Python 3 on Linux and the py launcher on Windows.
The project record gate is not the full Rust gate.
The common gate checks repository conventions and platform twins.

On Linux, run podbox-dev build after source changes and podbox-dev check before commit.
On Windows, run the wrapper for that check and the host common strict gate.
Set PODBOX_ARTIFACTS to receive /out, including the default check transcript,
podbox, four SSH helpers, and build-state.json.
A custom job must write its own outputs to /out.

A default wrapper check copies the tree, including Git and references.
It excludes target, .dev, .tmp, and live CodeGraph files by name.
The container is removed. Save the returned proof, then collect its toolkit
job by id. Keep the base and shared cache.

[containers](../docs/containers.md) owns the lane procedure.
[gate](../docs/methodology/gate.md) owns proof requirements.
[tooling](../docs/agent-tooling.md) owns source discovery and command paths.
[twins history](../docs/history/twins-and-scripts.md) retains the imported
check comparison.
