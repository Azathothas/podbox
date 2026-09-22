# Third-party material

podbox itself is [0BSD](LICENSE). That license does not relicense anyone else's
work. Copied files and the redistributed research corpus retain their original
terms and notices.

[`TODO/reference-map.md`](TODO/reference-map.md) records the per-tree license
determination, captured revision, and permitted use for every reference.

## Template material copied into the project

| Material | Source revision | License | Adaptation |
| --- | --- | --- | --- |
| `docs/conventions/`, `docs/methodology/`, `docs/security/` | `Azathothas/TEMPLATE` at `6206166`, through `Azathothas/container-research` at `0f155e3` | 0BSD | Two unused competing work-model pages retired; router moved to the project root |
| `docs/agent-tooling.md`, `docs/containers.md`, `docs/hosted-sessions.md`, `docs/methodology/lean-adoption.md`, `docs/public/README.md`, `docs/history/twins-and-scripts.md` | `Azathothas/TEMPLATE` commit `ea26c7d91a087a7132f56034fb77f8d40d7766cb` | 0BSD | Paths integrated into the podbox document set |
| `scripts/README.md`, `scripts/doctor/`, and maintained files in `scripts/common/` | same current template revision | 0BSD | Corpus-aware scope added where immutable references must not be rewritten |
| `.editorconfig`, `.gitattributes`, `.githooks/commit-msg`, `.github/dependabot.yml`, `.github/workflows/remote-items.yml` | same current template revision | 0BSD | Package ecosystems and workflow paths adapted to this Rust project |
| Seeded experiment scripts 10 through 50, `experiments/Dockerfile.target`, `experiments/targetfs.sh`, and `experiments/src/` | `Azathothas/container-research` at `0f155e3` | 0BSD | Extended by project-owned experiments and results |

The template license is retained at
[`references/Azathothas__TEMPLATE/tree/LICENSE`](references/Azathothas__TEMPLATE/tree/LICENSE).
`AGENTS.md`, the root public documents, architecture, code map, progress record,
and migration history are podbox's own work following the template's roles.

The common marker, documentation, one-home, and public-safety checks ignore
`references/` where applying project prose or fingerprint rules would require
editing a pinned third-party tree. Experiment results remain tracked evidence
and are not silently normalized.

## Redistributed reference corpus

Nothing under `references/` is compiled into podbox or placed on a runtime code
path. Each tree is a captured upstream revision with its own `PROVENANCE.md` and
license material where upstream provided it.

Some trees were trimmed by deleting uncited vendored dependencies, test
corpora, or binary archives. No license, notice, or copyright file was removed.
The exact trims and determinations are in
[`TODO/reference-map.md`](TODO/reference-map.md).

| License | Trees |
| --- | --- |
| MIT | `VHSgunzo__pathmap`, `fritzw__ld-preload-open`, `qaidvoid__onelf`, `io12__userland-execve-rust`, `VHSgunzo__ulexec`, `pkgforge-dev__cross-libc-dlopen`, `RuriOSS__ruri`, `RuriOSS__rurima`, `VHSgunzo__sharun`, `VHSgunzo__runimage`, `Azathothas__bit-cli` |
| Apache-2.0 | `indigo-dc__udocker`, `compforge__pathshim`, `multikernel__sandlock`, `containers__storage`, `containers__podman` |
| 0BSD | `Azathothas__TEMPLATE`, `Azathothas__container-research` |
| BSD-3-Clause and others listed by upstream | `apptainer__apptainer` |
| Split MIT and GPL-3.0 | `mhx__dwarfs` |
| LGPL-2.1 | `containers__bubblewrap`, `dex4er__fakechroot` |
| GPL-2.0 | `proot-me__proot` |
| GPL-3.0 | `89luca89__lilipod`, `garywill__treesandbox`, `salsa-debian__fakeroot` |
| Declared in `Cargo.toml` only, no license file captured | `VHSgunzo__memfd-exec`, `novafacing__memfd-exec` |
| No license found | `ylang-ylang__dockless` |
| Not applicable; repository absent | `VHSgunzo__userland-execve` |

Copyleft and unlicensed reference trees are studied, not copied into podbox.
[T-0909](TODO/deps.md) closed 2026-09-22: neither `memfd-exec` tree is
vendored, and `io12/userland-execve-rust` is vendored whole below.

## Vendored source

Third-party source compiled or carried for compilation in this tree. Each
row retains its original licence and notices.

| Path | Upstream commit | Licence | State |
| --- | --- | --- | --- |
| `vendor/userland-execve` | `io12/userland-execve-rust` at `02ef0e0` | MIT (`LICENSE` present) | whole and unpatched: six source files plus `LICENSE`, `Cargo.toml`, `README.md`, byte-identical to the corpus tree. Not compiled yet; [T-1003](TODO/packaging.md) patches its `goblin` and `nix` pins out when it wires the ladder. Retire check: `diff -r references/io12__userland-execve-rust/tree/src vendor/userland-execve/src` with `cmp` of the three root files. |

## Build dependencies

Rust dependencies are resolved through `Cargo.lock` and fetched by Cargo; their
source is not vendored into this repository. The workspace manifest records
why each dependency family was selected, and [`TODO/deps.md`](TODO/deps.md)
links the measured size and licensing decisions. Release packaging must produce
the final dependency-license inventory before M7 can close.
