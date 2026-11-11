# Scripts

Run `sh scripts/cloud-setup.sh` first in a cloud session or on a Linux host.
Then run podvm-dev session (after `cargo build --release -p podvm-gate`).
It reports the host and selects the lane.

| Script | Purpose |
| --- | --- |
| cloud-setup.sh | zig from the podcrates mirrors, the Rust toolchain and the musl target, and a host report |
| podvm-dev session | Host report, CodeGraph update, and lane setup |
| podvm-dev | Linux build, byte-based status, and full development check |
| windows/run-in-base.sh | Parked lane: copied-workspace Linux job through the podvm base |
| dev-lane.sh | Parked lane: the one lane proof runner: job lint, toolset, gc with ledger check, evidence assert |
| check-github-numbers.py | TODO/github.txt against the issues of GitHub (`record-issues.yml`) |
| zig-cc.sh, zig-ar.sh | `zig cc` and `zig ar` for the C code of the musl build |
| build-state.py | Input and output content checks |
| document-state.py | Source-derived document and mismatch check |
| podvm-gate | Task, citation, source-state, and cleanup gate |
| podvm-count | Status and count writer |
| build-interpose.sh | Interpose ceiling declaration; the build is podvm-interpose-build |
| release-licenses.py | Retained licence texts for locked Cargo packages |
| podvm-smoke | Architecture and image-path release smoke |
| release-notes.sh | Exact-commit green gate and build boundary |
| verify-release.sh | An exec shim; `podvm-verify` owns the signature check |

Use Python 3 on Linux and the py launcher on Windows.
The project record gate is not the full Rust gate.
The common gate checks repository conventions and platform twins.

On Linux, run podvm-dev build after source changes and podvm-dev check before commit.
The binaries are under target/x86_64-unknown-linux-musl/release/.
On the parked Windows lane, run the wrapper for that check and the host common strict gate.
Set PODVM_ARTIFACTS to receive /out, including the default check transcript,
podvm, four SSH helpers, and build-state.json.
A custom job must write its own outputs to /out.

A default wrapper check copies the tree, including Git and references/MANIFEST.tsv.
It excludes target, .dev, .tmp, and live CodeGraph files by name.
The container is removed. Save the returned proof, then collect its toolkit
job by id. Keep the base and shared cache.

[containers](../docs/containers.md) owns the lane procedure.
[gate](../docs/methodology/gate.md) owns proof requirements.
[tooling](../docs/agent-tooling.md) owns source discovery and command paths.
[twins history](../docs/history/twins-and-scripts.md) retains the imported
check comparison.
