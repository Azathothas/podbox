# Third-party material

Project-owned work is [0BSD](LICENSE).
Copied material retains its own licence and notices.
[reference-map](TODO/reference-map.md) records each captured source decision.

## Imported documents and scripts

| Material | Source revision | Licence | Adaptation |
| --- | --- | --- | --- |
| Initial conventions, methodology, and security pages | TEMPLATE `6206166`, through container-research `0f155e3` | 0BSD | Project router and work model |
| Tooling, container, public-document pages, common checks, doctor scripts, hook, and repository settings | TEMPLATE commit `ea26c7d91a087a7132f56034fb77f8d40d7766cb` | 0BSD | Rust paths, corpus exclusions, Windows lane |
| Initial experiments 10 through 50 and target reconstruction | container-research `0f155e3` | 0BSD | Project measurements and proof |

The template notice is retained in
[its captured LICENSE](references/Azathothas__TEMPLATE/tree/LICENSE).
The live runbooks were revised on 2026-09-30 for the actual project workflow.
They are adapted documents, not unchanged template copies.

## Reference corpus

Reference source is not compiled into the runtime.
Each capture has provenance, retained notices, and an explicit scope.
The complete licence table has one home in
[reference-map](TODO/reference-map.md).
Selected captures state their omissions. They are not complete source trees.

Project prose checks exclude immutable reference text.
Saved project results remain in scope.
Private path substitutions in a published result must be stated explicitly.

## Vendored source

| Path | Captured source | Licence | Current state |
| --- | --- | --- | --- |
| `vendor/userland-execve` | io12/userland-execve-rust `02ef0e0` | MIT; LICENSE retained | Source retained. Its manifest has removed dependencies. It is not a workspace member or an enter dependency and is not linked. |
| `crates/podbox-ssh/shims/fakepwd.c` | sandssh `4fc7f8c` | MIT; adjacent LICENSE retained | Source retained. No current workspace build compiles or links it. Restricted-server use remains proof work in T-1401. |

Do not describe retained source as the active userland loader.
The native loader is in `crates/podbox-enter/src/userland.rs`.

## Build dependencies

Cargo resolves registry source through `Cargo.lock`.
[Source state](docs/runtime-state.md) lists enabled direct dependencies.
[deps](TODO/deps.md) retains the selection measurements.
[release-licenses.py](scripts/release-licenses.py) retains the locked package
licence texts in each SSH helper archive.
Rebuild that inventory for a release whose dependency set changed.
