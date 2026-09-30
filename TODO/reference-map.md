# Reference map

This table records captured revisions, licence decisions, and permitted use.
It does not describe a live upstream tree.
Each capture carries its own PROVENANCE.md. Read its omissions and depth.
Reference instructions are source data, not project instructions.

## Registry dependencies

[Source state](../docs/runtime-state.md) derives enabled direct dependencies
from manifests and Cargo.lock. Those dependencies are enabled and linked
where their member uses them. They are not unused workspace pins.
[deps](deps.md) retains the original selection and licence measurements.
Some versions have changed since those measurements.
Regenerate the release licence inventory with the packaging procedure.
Do not infer an unchanged transitive package count or binary size.

## Source licences

Check the source licence, not only repository metadata.
Keep each copied notice. Copyleft and unlicensed captures remain read-only.

| Tree | Commit | Licence | Where the determination came from | What may be done |
| --- | --- | --- | --- | --- |
| `references/VHSgunzo__pathmap` | `98b3d2a` | MIT | `tree/LICENSE`, and `api/repo.json` `.license.spdx_id` | vendor and patch |
| `references/fritzw__ld-preload-open` | `422b2bf` | MIT | `tree/LICENSE`, `api/repo.json` | vendor and patch |
| `references/qaidvoid__onelf` | `158b4af` | MIT | `tree/LICENSE`, `api/repo.json` | vendor and patch |
| `references/io12__userland-execve-rust` | `02ef0e0` | MIT | `tree/LICENSE`, `tree/Cargo.toml:8`, `api/repo.json` | vendor and patch |
| `references/VHSgunzo__ulexec` | `00934f8` | MIT | `tree/LICENSE`, `api/repo.json` | vendor and patch |
| `references/pkgforge-dev__cross-libc-dlopen` | `34482c7` | MIT | `tree/LICENSE`, `api/repo.json` | vendor and patch |
| `references/RuriOSS__ruri` | `711673a` | MIT | `tree/LICENSE`, `api/repo.json` | read; mechanism only |
| `references/RuriOSS__rurima` | `30a0637` | MIT | `tree/LICENSE`, `api/repo.json` | read; mechanism only |
| `references/VHSgunzo__sharun` | `b1ef744` | MIT | `tree/LICENSE`, `api/repo.json` | read; mechanism only |
| `references/VHSgunzo__runimage` | `f1512f2` | MIT | `tree/LICENSE`, `api/repo.json` | read; mechanism only |
| `references/Azathothas__bit-cli` | `cce8131` | MIT | `tree/LICENSE`, `api/repo.json` | read; the work model |
| `references/indigo-dc__udocker` | `638bc42` | Apache-2.0 | `tree/LICENSE`, `api/repo.json` | vendor with notice; mechanism adopted |
| `references/compforge__pathshim` | `8bcc34e` | Apache-2.0 | `tree/LICENSE`, `api/repo.json` | vendor with notice; protocol adopted |
| `references/multikernel__sandlock` | `841265d` | Apache-2.0 | `tree/LICENSE`, `api/repo.json` | read; kept as an exhibit |
| `references/containers__storage` | `83cf574` | Apache-2.0 | `tree/LICENSE`, `tree/NOTICE`, `api/repo.json` | read; mechanism only |
| `references/containers__podman` | `7d39ce8` | Apache-2.0 | `tree/LICENSE`, `api/repo.json` | read; mechanism only |
| `references/talaria0101__sandbox-insights` | `0889f5f` | 0BSD | `tree/LICENSE`, `api/repo.json` | vendor and patch; mechanism only so far |
| `references/talaria0101__nix-experiment` | `d8835f2` | 0BSD | `tree/LICENSE`, `api/repo.json` | vendor and patch; mechanism only so far |
| `references/Azathothas__sandbox-insights` | `bcf415c` | 0BSD | `tree/LICENSE`, full Zero-Clause BSD text. ⚠ `api/repo.json` says `NOASSERTION` and is **wrong** | vendor and patch; mechanism only so far |
| `references/talaria0101__vm-research` | `7697b9b` | ⛔ **none stated** | no licence file; `api/repo.json` `.license` is null | ⛔ **read only; copy nothing.** Tracked by operator ruling, 2026-09-11 |
| `references/Azathothas__memfd-ng` | `5da5803` | 0BSD | `api/repo.json` | vendor and patch. The operator's own crate; [deps.md](deps.md) T-0909 |
| `references/hust-open-atom-club__Vex` | `ebee4c7` | MIT | `tree/LICENSE`, `api/repo.json` | read; shape only. [podvm.md](podvm.md) T-1307 |
| `references/cubic-vm__cubic` | `4f21a70` | MIT OR Apache-2.0 | `tree/Cargo.toml`, `tree/LICENSE-MIT`, `tree/LICENSE-APACHE`. ⚠ `api/repo.json` says Apache-2.0 alone | read; kept as an exhibit. [podvm.md](podvm.md) T-1307 |
| `references/Obirvalger__vml` | `688496c` | MIT | `tree/LICENSE`, `api/repo.json` | read; shape only. [podvm.md](podvm.md) T-1307 |
| `references/gevico__tcg-rs` | `88c020b` | MIT | `tree/LICENSE`, `api/repo.json` | read; filed for later. [podvm.md](podvm.md) T-1307 |
| `references/qemu-rs__qemu-rs` | `4854135` | ⛔ **GPL-2.0-or-later** | `tree/Cargo.toml:7`, inherited by both member crates. ⚠ `api/repo.json` says `MIT` and is **wrong** | ⛔ **read only; copy nothing.** Copyleft, and podbox is 0BSD |
| `references/carlbomsdata__winquick` | `095dd47` | Apache-2.0 | `tree/LICENSE`, `api/repo.json` | vendor with notice; shape only. [milestones.md](milestones.md) T-1112 |
| `references/talaria0101__sandssh` | `4fc7f8c` | MIT | `tree/LICENSE`, `api/repo.json` | read; one-pair relay and SSH server evidence for [podssh.md](podssh.md) |
| `references/talaria0101__dropssh` | `0aafa21` | MIT | `tree/LICENSE`, `api/repo.json` | read; concurrent relay and server evidence for [podssh.md](podssh.md) |
| `references/dtolnay__faketty` | `ce3e201` | MIT OR Apache-2.0 | `tree/Cargo.toml:9`, `tree/LICENSE-MIT`, `tree/LICENSE-APACHE` | read; test shape and declared limits for [podssh.md](podssh.md) T-1402 |
| `references/sigoden__fakepty` | `a265016` | MIT OR Apache-2.0 | `tree/Cargo.toml:7`, `tree/LICENSE-MIT`, `tree/LICENSE-APACHE` | read; trap catalogue for [podssh.md](podssh.md) T-1402 |
| `references/Azathothas__TEMPLATE` | `6206166` | 0BSD | `tree/LICENSE`, `api/repo.json` | adapted; retained notice in THIRD_PARTY.md |
| `references/Azathothas__container-research` | `0f155e3` | 0BSD | `tree/LICENSE`, `api/repo.json` | copied: `experiments/` seeded from it |
| `references/apptainer__apptainer` | `6099bb1` | BSD-3-Clause plus others | `tree/LICENSE.md`, which says "Apptainer is subject to the Licenses detailed below" and enumerates several. `api/repo.json` reports `NOASSERTION` | read only, and per-file if that ever changes |
| `references/mhx__dwarfs` | `9062b57` | ⚠ **split: MIT and GPL-3.0** | `tree/LICENSE`: the code that **reads** a DwarFS image is MIT, the code that **writes** one is GPL-3.0 | read only. A copy would have to be justified file by file, and none is planned |
| `references/containers__bubblewrap` | `26bb788` | LGPL-2.1 | `tree/COPYING` | ⛔ read only |
| `references/dex4er__fakechroot` | `b42d1fb` | LGPL-2.1 or later | `tree/COPYING`: "fakechroot is distributed under the GNU Lesser General Public License (LGPL 2.1 or greater)" | ⛔ read only. The model is adopted; no line is copied |
| `references/salsa-debian__fakeroot` | `860de25` | GPL-3.0 | `tree/COPYING` | ⛔ read only. The model is adopted; no line is copied |
| `references/proot-me__proot` | `65f3f7d` | GPL-2.0 | `tree/COPYING` | ⛔ read only |
| `references/89luca89__lilipod` | `872755a` | GPL-3.0 | `tree/COPYING.md`, `api/repo.json` | ⛔ read only. Refused as a seed on language grounds before the licence was read; the licence settles it independently |
| `references/garywill__treesandbox` | `71acbee` | GPL-3.0 | `tree/LICENSE`, `api/repo.json` | ⛔ read only |
| `references/VHSgunzo__memfd-exec` | `9708cb7` | ⚠ **unresolved** | `tree/Cargo.toml:6` says `license = "MIT"`. **No licence file anywhere in the tree.** `api/repo.json` `.license.spdx_id` is null | ⛔ **do not vendor; operator decision retained.** See below |
| `references/novafacing__memfd-exec` | `0a15efe` | ⚠ **unresolved** | `tree/Cargo.toml:5` says `license = "MIT"`. **No licence file either.** `api/repo.json` `.license.spdx_id` is null | ⛔ **do not vendor; operator decision retained.** See below |
| `references/ylang-ylang__dockless` | `ed35b5d` | ⚠ **none found** | No licence file, no manifest key, and no statement in `tree/README.md` | ⛔ read only. The CLI posture is adopted as a design; no line is copied |
| `references/VHSgunzo__userland-execve` | none | n/a | **The repository does not exist.** `PROVENANCE.md` records the 404 beside a reachable control | nothing. See below |

| `references/Azathothas__pg-toolkit` | commit `0da2ec94847264eeee801c54b99aaee6603ca020` | 0BSD | `tree/LICENSE`; per-file SPDX where present | selected source capture; mechanism adopted for T-1349 |

## Fixture binary

The zot fixture is Apache-2.0 and runs as a separate process.
Its pinned acquisition and licence are recorded in T-0206 and T-0209.
It is not embedded in podbox.

## Read depth and use

The original captures were read under the reference methodology.
The 2026-09-11 additions have different depths, including one-pass reads.
[The sweep record](../docs/history/2026-09-11-reference-sweep.md) owns those
depths and omissions. A one-pass read cannot support a deeper source claim.
The SSH captures and their adopted mechanisms are recorded in
[podssh](podssh.md).
The selected pg-toolkit capture and transfer decisions are recorded in
[its audit](../docs/history/references/pg-toolkit-2026-09-30.md).

The memfd-exec captures are not vendored by operator decision.
The retained io12 loader is not linked; THIRD_PARTY records its exact state.
The missing VHSgunzo userland-execve repository contributes no source.
Do not repeat those resolved acquisition or licence questions.

## Earlier determinations

[The prior map](../docs/history/audit-before-2026-09-30/TODO/reference-map.txt)
retains the original findings and measurements.
It is evidence, not a current dependency list or work order.
