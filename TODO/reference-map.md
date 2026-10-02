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

The `Tree` column is the capture's own directory, and the gate checks that
it exists and carries a `PROVENANCE.md`. It names the capture. It is not a
citation into the source: no row here reads a line of captured code. Each
row states the determination, where it was read, and what may be done with
it, so the licence decision stands on its own without the captured bytes.

`Read from` names the file inside the capture's `tree/`, or the repository
API capture under `api/`, where the determination was read. The file name is
the record of where the reading happened. A reader can repeat that reading
against the upstream at the recorded commit without this tree.

| Tree | Commit | Licence | Read from | What may be done |
| --- | --- | --- | --- | --- |
| `references/VHSgunzo__pathmap` | `98b3d2a` | MIT | `LICENSE`, and the repository API `.license.spdx_id` | vendor and patch |
| `references/fritzw__ld-preload-open` | `422b2bf` | MIT | `LICENSE`, and the repository API | vendor and patch |
| `references/qaidvoid__onelf` | `158b4af` | MIT | `LICENSE`, and the repository API | vendor and patch |
| `references/io12__userland-execve-rust` | `02ef0e0` | MIT | `LICENSE`, `Cargo.toml:8`, and the repository API | vendor and patch |
| `references/VHSgunzo__ulexec` | `00934f8` | MIT | `LICENSE`, and the repository API | vendor and patch |
| `references/pkgforge-dev__cross-libc-dlopen` | `34482c7` | MIT | `LICENSE`, and the repository API | vendor and patch |
| `references/RuriOSS__ruri` | `711673a` | MIT | `LICENSE`, and the repository API | read; mechanism only |
| `references/RuriOSS__rurima` | `30a0637` | MIT | `LICENSE`, and the repository API | read; mechanism only |
| `references/VHSgunzo__sharun` | `b1ef744` | MIT | `LICENSE`, and the repository API | read; mechanism only |
| `references/VHSgunzo__runimage` | `f1512f2` | MIT | `LICENSE`, and the repository API | read; mechanism only |
| `references/Azathothas__bit-cli` | `cce8131` | MIT | `LICENSE`, and the repository API | read; the work model |
| `references/indigo-dc__udocker` | `638bc42` | Apache-2.0 | `LICENSE`, and the repository API | vendor with notice; mechanism adopted |
| `references/compforge__pathshim` | `8bcc34e` | Apache-2.0 | `LICENSE`, and the repository API | vendor with notice; protocol adopted |
| `references/multikernel__sandlock` | `841265d` | Apache-2.0 | `LICENSE`, and the repository API | read; kept as an exhibit |
| `references/containers__storage` | `83cf574` | Apache-2.0 | `LICENSE`, `NOTICE`, and the repository API | read; mechanism only |
| `references/containers__podman` | `7d39ce8` | Apache-2.0 | `LICENSE`, and the repository API | read; mechanism only |
| `references/talaria0101__sandbox-insights` | `0889f5f` | 0BSD | `LICENSE`, and the repository API | vendor and patch; mechanism only so far |
| `references/talaria0101__nix-experiment` | `d8835f2` | 0BSD | `LICENSE`, and the repository API | vendor and patch; mechanism only so far |
| `references/Azathothas__sandbox-insights` | `bcf415c` | 0BSD | `LICENSE`, which carries the full Zero-Clause BSD text. ⚠ the repository API says `NOASSERTION` and is **wrong**: its classifier does not match the `Zero-Clause BSD` heading | vendor and patch; mechanism only so far |
| `references/talaria0101__vm-research` | `7697b9b` | ⛔ **none stated** | no licence file in the tree; the repository API `.license` is null | ⛔ **read only; copy nothing.** Tracked by operator ruling, 2026-09-11 |
| `references/Azathothas__memfd-ng` | `5da5803` | 0BSD | no licence file in the tree; the repository API | vendor and patch. The operator's own crate; [deps.md](deps.md) T-0909 |
| `references/hust-open-atom-club__Vex` | `ebee4c7` | MIT | `LICENSE`, and the repository API | read; shape only. [podvm.md](podvm.md) T-1307 |
| `references/cubic-vm__cubic` | `4f21a70` | MIT OR Apache-2.0 | `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`; two texts, selected by the manifest. ⚠ the repository API says Apache-2.0 alone | read; kept as an exhibit. [podvm.md](podvm.md) T-1307 |
| `references/Obirvalger__vml` | `688496c` | MIT | `LICENSE`, and the repository API | read; shape only. [podvm.md](podvm.md) T-1307 |
| `references/gevico__tcg-rs` | `88c020b` | MIT | `LICENSE`, and the repository API | read; filed for later. [podvm.md](podvm.md) T-1307 |
| `references/qemu-rs__qemu-rs` | `4854135` | ⛔ **GPL-2.0-or-later** | `Cargo.toml:7`, inherited by both member crates. ⚠ the repository API says `MIT` and is **wrong** | ⛔ **read only; copy nothing.** Copyleft, and podbox is 0BSD |
| `references/carlbomsdata__winquick` | `095dd47` | Apache-2.0 | `LICENSE`, and the repository API | vendor with notice; shape only. [milestones.md](milestones.md) T-1112 |
| `references/talaria0101__sandssh` | `4fc7f8c` | MIT | `LICENSE`, and the repository API | read; one-pair relay and SSH server evidence for [podssh.md](podssh.md) |
| `references/talaria0101__dropssh` | `0aafa21` | MIT | `LICENSE`, and the repository API | read; concurrent relay and server evidence for [podssh.md](podssh.md) |
| `references/dtolnay__faketty` | `ce3e201` | MIT OR Apache-2.0 | `Cargo.toml:9`, `LICENSE-MIT`, `LICENSE-APACHE`; two texts, selected by the manifest | read; test shape and declared limits for [podssh.md](podssh.md) T-1402 |
| `references/sigoden__fakepty` | `a265016` | MIT OR Apache-2.0 | `Cargo.toml:7`, `LICENSE-MIT`, `LICENSE-APACHE`; two texts, selected by the manifest | read; trap catalogue for [podssh.md](podssh.md) T-1402 |
| `references/Azathothas__TEMPLATE` | `6206166` | 0BSD | `LICENSE`, and the repository API | adapted; retained notice in THIRD_PARTY.md |
| `references/Azathothas__container-research` | `0f155e3` | 0BSD | `LICENSE`, and the repository API | copied: `experiments/` seeded from it |
| `references/apptainer__apptainer` | `6099bb1` | BSD-3-Clause plus others | `LICENSE.md`, which says "Apptainer is subject to the Licenses detailed below" and enumerates several. The repository API reports `NOASSERTION` | read only, and per-file if that ever changes |
| `references/mhx__dwarfs` | `9062b57` | ⚠ **split: MIT and GPL-3.0** | `LICENSE`: the code that **reads** a DwarFS image is MIT, the code that **writes** one is GPL-3.0 | read only. A copy would have to be justified file by file, and none is planned |
| `references/containers__bubblewrap` | `26bb788` | LGPL-2.1 | `COPYING` | ⛔ read only |
| `references/dex4er__fakechroot` | `b42d1fb` | LGPL-2.1 or later | `COPYING`: "fakechroot is distributed under the GNU Lesser General Public License (LGPL 2.1 or greater)" | ⛔ read only. The model is adopted; no line is copied |
| `references/salsa-debian__fakeroot` | `860de25` | GPL-3.0 | `COPYING` | ⛔ read only. The model is adopted; no line is copied |
| `references/proot-me__proot` | `65f3f7d` | GPL-2.0 | `COPYING` | ⛔ read only |
| `references/89luca89__lilipod` | `872755a` | GPL-3.0 | `COPYING.md`, and the repository API | ⛔ read only. Refused as a seed on language grounds before the licence was read; the licence settles it independently |
| `references/garywill__treesandbox` | `71acbee` | GPL-3.0 | `LICENSE`, and the repository API | ⛔ read only |
| `references/VHSgunzo__memfd-exec` | `9708cb7` | ⚠ **unresolved** | `Cargo.toml:6` says `license = "MIT"`. **No licence file anywhere in the tree.** The repository API `.license.spdx_id` is null | ⛔ **do not vendor; operator decision retained.** See below |
| `references/novafacing__memfd-exec` | `0a15efe` | ⚠ **unresolved** | `Cargo.toml:5` says `license = "MIT"`. **No licence file either.** The repository API `.license.spdx_id` is null | ⛔ **do not vendor; operator decision retained.** See below |
| `references/ylang-ylang__dockless` | `ed35b5d` | ⚠ **none found** | No licence file, no manifest key, and no statement in `README.md` | ⛔ read only. The CLI posture is adopted as a design; no line is copied |
| `references/VHSgunzo__userland-execve` | none | n/a | **The repository does not exist.** The capture has no `tree/`; its `PROVENANCE.md` records the 404 beside a reachable control | nothing. See below |
| `references/Azathothas__pg-toolkit` | commit `0da2ec94847264eeee801c54b99aaee6603ca020` | 0BSD | `LICENSE`; per-file SPDX where present | selected source capture; mechanism adopted for T-1349 |

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

## The captures and this table

Each capture is a directory at the corpus root, named by the `Tree`
column, and carries its own `PROVENANCE.md` with the commit, the route, the
omissions, and the depth reached. Read that before relying on any claim
about the tree.

A row's `Read from` value names where its licence determination was read
inside that capture. It records the reading, and it does not require the
bytes: the upstream repository at the recorded commit answers the same
question. A one-pass read cannot support a deeper source claim, and
[The sweep record](../docs/history/2026-09-11-reference-sweep.md) says which
captures were read at that depth.

## Earlier determinations

[The prior map](../docs/history/audit-before-2026-09-30/TODO/reference-map.txt)
retains the original findings and measurements.
It is evidence, not a current dependency list or work order.
