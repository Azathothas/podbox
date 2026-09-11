# Reference map

The corpus, and what may be done with each tree.

Per `docs/methodology/references.md` section 4 the shape chosen here is
**tracked, in the tree**: every reference lives under `references/<owner>__<repo>/`
on `main`, at a pinned commit, with its tracker beside it.

```sh
ls references/                                  # the corpus
cat references/VHSgunzo__pathmap/PROVENANCE.md  # commit, route, and gaps
```

That shape is chosen over the side branch for one reason:
`scripts/check-todo.py` asserts that **every cited path and line resolves**, and
a gate cannot resolve a citation into a branch it is not on. The cost is a clone
that carries it. ⚠ Re-measured on 2026-09-08 by cloning the pushed `main`
afresh, twice, because the earlier figures had drifted and then the history was
rewritten: **154 MB of working tree** and **29 MB of git objects** (`du -sh
.git`; `git count-objects -vH` reports one pack of 27.04 MiB over 7,931
objects). ⭐ Before the rewrite the object figure was 51 MB, and the difference
is the artefact debt [PROGRESS.md](PROGRESS.md) records as cleared, not the
corpus.

Each directory holds `PROVENANCE.md` (the commit, the route, and what could not
be fetched), `api/` (issues and pull requests in both states, comments, review
comments, releases, tags), and `tree/` (the source at that commit, with `.git`
already stripped and the commit captured first).

⛔ **Cite the commit beside every line reference.** The row below carries it.

## Registry dependencies, resolved before use

⛔ **The same rule, for crates rather than trees.** [deps.md](deps.md)'s
question 4 requires the licence determination to be made before a candidate
lands and recorded here. Every row below was read from the crate's own
`Cargo.toml` and confirmed by a licence file present in the published crate, on
2026-09-08, at the version `cargo metadata` resolved for the pin in the root
`Cargo.toml`.

⚠ These are **pins, not enabled dependencies**. A member crate takes one with
`foo.workspace = true` when the milestone that needs it lands, so none of them
is in `Cargo.lock` or in the artefact today.

| Crate | Resolved | Licence | Licence file in the crate | Entry that landed it |
| --- | --- | --- | --- | --- |
| `rustls` | 0.23.44 | Apache-2.0 OR ISC OR MIT | yes, 3 | [T-0905](deps.md) |
| `rustls-pki-types` | 1.15.1 | MIT OR Apache-2.0 | yes, 2 | [T-0905](deps.md) |
| `rustls-pemfile` | 2.2.0 | Apache-2.0 OR ISC OR MIT | yes, 4 | [T-0905](deps.md) |
| `webpki-roots` | 1.0.9 | CDLA-Permissive-2.0 | yes, 1 | [T-0905](deps.md) |
| `ureq` | 2.12.1 | MIT OR Apache-2.0 | yes, 2 | [T-0906](deps.md) |
| `tar` | 0.4.46 | MIT OR Apache-2.0 | yes, 2 | [T-0907](deps.md) |
| `flate2` | 1.1.10 | MIT OR Apache-2.0 | yes, 2 | [T-0907](deps.md) |
| `ruzstd` | 0.7.3 | MIT | yes, 1 | [T-0907](deps.md) |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | yes, 2 | [T-0908](deps.md) |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | yes, 2 | [T-0908](deps.md) |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | yes, 2 | [T-0908](deps.md) |

⚠ **`webpki-roots` is CDLA-Permissive-2.0 and that is not an oversight.** It is
a **dataset** of root certificates rather than code, and CDLA-Permissive-2.0
permits redistribution with the notice retained, which is what a 0BSD artefact
embedding it needs.

⛔ **Measured and refused**, so a later session does not re-argue them:
`clap`, `goblin`, `oci-spec`, `seccompiler`, the `landlock` crate, `rustix` and
`libc`. Each has a committed number under `experiments/results/` and a ruling in
its entry.

⚠ **Transitively, the landed set resolves 86 packages** on
`x86_64-unknown-linux-musl` with every pin wired, of which `ring` is the only
one that compiles C. Their licences are not enumerated here: what binds is that
each direct dependency above permits redistribution, and a transitive licence
audit belongs with the packaging entry that ships a notice file
([T-1004](packaging.md)) rather than with the sweep that priced them.

## Licence, resolved before use

⛔ **The determination is made before the tree is used, not after.** podbox is
0BSD. A permissive tree may be vendored with its own notice retained; a copyleft
tree may be **read** and its mechanism described, and no line of it may be
copied into this repository.

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
| `references/Azathothas__TEMPLATE` | `6206166` | 0BSD | `tree/LICENSE`, `api/repo.json` | copied verbatim into `docs/` and `scripts/common/` |
| `references/Azathothas__container-research` | `0f155e3` | 0BSD | `tree/LICENSE`, `api/repo.json` | copied: `experiments/` seeded from it |
| `references/apptainer__apptainer` | `6099bb1` | BSD-3-Clause plus others | `tree/LICENSE.md`, which says "Apptainer is subject to the Licenses detailed below" and enumerates several. `api/repo.json` reports `NOASSERTION` | read only, and per-file if that ever changes |
| `references/mhx__dwarfs` | `9062b57` | ⚠ **split: MIT and GPL-3.0** | `tree/LICENSE`: the code that **reads** a DwarFS image is MIT, the code that **writes** one is GPL-3.0 | read only. A copy would have to be justified file by file, and none is planned |
| `references/containers__bubblewrap` | `26bb788` | LGPL-2.1 | `tree/COPYING` | ⛔ read only |
| `references/dex4er__fakechroot` | `b42d1fb` | LGPL-2.1 or later | `tree/COPYING`: "fakechroot is distributed under the GNU Lesser General Public License (LGPL 2.1 or greater)" | ⛔ read only. The model is adopted; no line is copied |
| `references/salsa-debian__fakeroot` | `860de25` | GPL-3.0 | `tree/COPYING` | ⛔ read only. The model is adopted; no line is copied |
| `references/proot-me__proot` | `65f3f7d` | GPL-2.0 | `tree/COPYING` | ⛔ read only |
| `references/89luca89__lilipod` | `872755a` | GPL-3.0 | `tree/COPYING.md`, `api/repo.json` | ⛔ read only. Refused as a seed on language grounds before the licence was read; the licence settles it independently |
| `references/garywill__treesandbox` | `71acbee` | GPL-3.0 | `tree/LICENSE`, `api/repo.json` | ⛔ read only |
| `references/VHSgunzo__memfd-exec` | `9708cb7` | ⚠ **unresolved** | `tree/Cargo.toml:6` says `license = "MIT"`. **No licence file anywhere in the tree.** `api/repo.json` `.license.spdx_id` is null | ⛔ **do not vendor until resolved.** See below |
| `references/novafacing__memfd-exec` | `0a15efe` | ⚠ **unresolved** | `tree/Cargo.toml:5` says `license = "MIT"`. **No licence file either.** `api/repo.json` `.license.spdx_id` is null | ⛔ **do not vendor until resolved.** See below |
| `references/ylang-ylang__dockless` | `ed35b5d` | ⚠ **none found** | No licence file, no manifest key, and no statement in `tree/README.md` | ⛔ read only. The CLI posture is adopted as a design; no line is copied |
| `references/VHSgunzo__userland-execve` | none | n/a | **The repository does not exist.** `PROVENANCE.md` records the 404 beside a reachable control | nothing. See below |

### ⭐ The three that were not resolved, and the rulings that closed them

⭐ **All three were settled by the operator on 2026-09-11.** Each was read at
the source first, and the reading changed one of the three answers.

1. **`memfd-exec`, both the fork and its upstream. ⛔ RULED: do not vendor
   either.** The only MIT statement in either tree is the `license` key at
   `references/VHSgunzo__memfd-exec/tree/Cargo.toml:5` and
   `references/novafacing__memfd-exec/tree/Cargo.toml:5`; neither ships a
   licence file and GitHub classifies neither. ⭐ **The ruling does not rest on
   that ambiguity.** The operator maintains
   [`Azathothas/memfd-ng`](https://github.com/Azathothas/memfd-ng), 0BSD, which
   is this project's own licence, and which does the same job with an
   allocation-free tmpfs ladder fallback. The corpus carries it at
   `references/Azathothas__memfd-ng`. [deps.md](deps.md) T-0909 keeps its
   Decision to write podbox's own three-syscall path and now has a second
   candidate to measure against it rather than a licence question to wait on.

2. **`dockless` has no licence statement of any kind. ⭐ RULED: keep the tree,
   study it, copy nothing, re-implement where it is useful.** Under default
   copyright there is no permission to copy, and the tree stays in the corpus so
   every citation already written still resolves. Its value here is a
   **posture**, which `T-0801` records as a design decision in podbox's own
   words.

3. **`VHSgunzo/userland-execve` does not exist. ⭐ RULED: keep the row as a
   corrected citation.** `TOOL.md` section 3.5 calls it "the second fork to
   vendor". `references/VHSgunzo__userland-execve/PROVENANCE.md` records the 404
   with a reachable control beside it, and
   `references/VHSgunzo__ulexec/tree/Cargo.toml:29` shows that `ulexec` depends
   on the original crate, `userland-execve = "0.2.0"`, whose repository
   crates.io gives as `io12/userland-execve-rust`. That tree is in the corpus
   and its licence is clean. ⛔ The row is not deleted: a previous session cited
   a repository that does not exist, and that disagreement is the finding.

### ⛔ Two badges that were wrong, and the trees that settled them

⭐ **A licence is read from the tree, never from the code host's badge.** Both
of these were recorded from the badge by an earlier reading and both were wrong
in a way that changed the answer.

| tree | the badge said | the tree says | effect |
| --- | --- | --- | --- |
| `references/Azathothas__sandbox-insights` | `NOASSERTION` | `tree/LICENSE` carries the full Zero-Clause BSD text, headed `Zero-Clause BSD` | ⭐ **0BSD, vendorable.** The badge is `NOASSERTION` only because the heading is not the canonical string its classifier matches |
| `references/qemu-rs__qemu-rs` | `MIT` | `tree/Cargo.toml:7` declares `license = "GPL-2.0-or-later"`, and both member crates take `license.workspace = true` | ⛔ **copyleft, and not vendorable into a 0BSD binary.** The same reasoning `TOOL.md` section 3.3 applies to lilipod's GPL-3.0 |

## Verdicts

Per `docs/methodology/references.md`, exactly one per reference.

| Tree | Verdict | What transfers, and where it is written down |
| --- | --- | --- |
| `references/VHSgunzo__pathmap` | **adopt (vendor and patch)** | The path half of the interposer. [interpose.md](interpose.md) T-0703, T-0705, T-0707 |
| `references/fritzw__ld-preload-open` | **adopt (rulings)** | pathmap's upstream. Its tracker carries the absolute-`LD_PRELOAD` ruling and the musl build recipe. [interpose.md](interpose.md) T-0702 |
| `references/salsa-debian__fakeroot` | **adopt (model)** | The ownership half, and the errno test that is wrong here. [interpose.md](interpose.md) T-0704 |
| `references/dex4er__fakechroot` | **adopt (model)** | The path half as a per-entry-point library, and the exclude list. [interpose.md](interpose.md) T-0707 |
| `references/pkgforge-dev__cross-libc-dlopen` | **adopt (mechanism)** | Cross-libc loading, the struct-layout ceiling, the lock rule, the preload ordering requirement. [interpose.md](interpose.md) T-0701, T-0702 |
| `references/indigo-dc__udocker` | **adopt (mechanism), confirms (design)** | Ownership-neutral extraction, the inter-layer re-permission pass, the per-container mode selector. [extract.md](extract.md) T-0302, T-0303, T-0306; [probe.md](probe.md) T-0107 |
| `references/RuriOSS__ruri` | **adopt (discipline)** | Probe-then-refuse with a named floor, and one switch that turns every degradation into a refusal. [probe.md](probe.md) T-0107; [cli.md](cli.md) T-0804 |
| `references/compforge__pathshim` | **adopt (protocol)** | The probe's three-channel contract and the disposable-child verdict. [probe.md](probe.md) T-0101, T-0110 |
| `references/qaidvoid__onelf` | **adopt (packaging)** | The launch ladder, the separate request and result variables, the lock held through exec, the lexical symlink check. [packaging.md](packaging.md) T-1001, T-1002; [extract.md](extract.md) T-0304 |
| `references/io12__userland-execve-rust` | **adopt (vendor and patch)** | Userland exec, and the stack-size costing its tracker carries. [packaging.md](packaging.md) T-1003 |
| `references/VHSgunzo__ulexec` | **adopt (mechanism)** | The two rungs driven from one tool. [packaging.md](packaging.md) T-1003 |
| `references/VHSgunzo__memfd-exec` | **adopt (mechanism), blocked on licence** | The memfd rung, its probe-before-use, and a regression to avoid. [deps.md](deps.md) T-0909 |
| `references/novafacing__memfd-exec` | **confirms** | The upstream the fork's regression is measured against. [deps.md](deps.md) T-0909 |
| `references/multikernel__sandlock` | **anti-pattern exhibit** | The complete chain from a filtered read to a false report, and its own audit's blind spot. [supervise.md](supervise.md) T-0606 |
| `references/containers__storage` | **confirms** | Wall 1 at the exact line, and the configuration option that clears it. [extract.md](extract.md) T-0302; [image.md](image.md) T-0205 |
| `references/containers__podman` | **confirms** | Where the layer applier is reached from. [image.md](image.md) T-0205 |
| `references/containers__bubblewrap` | **confirms** | The witness that separates a refused clone from a refused mount. [probe.md](probe.md) T-0102 |
| `references/apptainer__apptainer` | **confirms** | A fourth tool at wall 1, in Go, so interposition cannot rescue it. [interpose.md](interpose.md) T-0706 |
| `references/89luca89__lilipod` | **refused as a seed; lessons adopted** | The dependency precheck, the `Credential` shape, and the fabricated-root `exec` bug. [enter.md](enter.md) T-0502; [cli.md](cli.md) T-0806 |
| `references/proot-me__proot` | **refused** | `ptrace` is filtered, and its own diagnostic misdirects. [probe.md](probe.md) T-0101; [interpose.md](interpose.md) T-0706 |
| `references/RuriOSS__rurima` | **refused (pull)** | `tar -xpf` as root, which is wall 1 again. [extract.md](extract.md) T-0301 |
| `references/ylang-ylang__dockless` | **adopt (CLI posture), refused (engine)** | Fail-fast guardrails that name the incident behind each. [cli.md](cli.md) T-0801, T-0804 |
| `references/garywill__treesandbox` | **refused** | It dies at `getpwuid(0)` before any namespace call. [complete.md](complete.md) T-0404 |
| `references/mhx__dwarfs` | **confirms** | `short write: -20` is a libarchive status and not an errno. [image.md](image.md) T-0203 |
| `references/VHSgunzo__sharun` | **filed elsewhere** | It solves relocation, not filesystem virtualization. Nothing in podbox owns it today; [packaging.md](packaging.md) T-1003 names where it would land |
| `references/VHSgunzo__runimage` | **filed elsewhere** | The launcher whose failure is bubblewrap's, already carried by that row |
| `references/Azathothas__container-research` | **adopt (the specification)** | `TOOL.md` and `paper_final.md` are the source of every entry here |
| `references/Azathothas__TEMPLATE` | **adopt (the methodology)** | `docs/` is copied from it verbatim and binds |
| `references/Azathothas__bit-cli` | **adopt (the work model)** | The shape of `INDEX.md`, `RULES.md`, `PROGRESS.md` and this file |
| `references/VHSgunzo__userland-execve` | **refused (does not exist)** | Recorded so no session re-derives the 404 |

## What was deleted, and why

⛔ **Trimmed by deleting, never by moving**, per
`docs/methodology/references.md` section 1: a trim that rewrites paths
invalidates every citation already written.

| Tree | Deleted | Why |
| --- | --- | --- |
| `references/containers__podman` | `tree/vendor`, `tree/test` | 104 MB of vendored Go modules and test data. podman at `7d39ce8` does not vendor `containers/storage`, so the wall-1 citation is taken against `references/containers__storage` at its own commit instead. ⚠ That is checkable without the deleted directory: `curl -sS -H 'User-Agent: curl/8.5.0' 'https://api.gh.pkgforge.dev/repos/containers/podman/contents/vendor/github.com/containers?ref=7d39ce8fe43f7d400dc3decc0c5961c18857ee55' \| jq -r '.[].name'` lists seven entries on 2026-09-08 and `storage` is not among them |
| `references/containers__storage` | `tree/tests`, `tree/vendor` | 24 MB. Nothing here cites either |
| `references/ylang-ylang__dockless` | `tree/vendor/udocker-englib-1.2.11.tar.gz` | a 45 MB binary tarball of udocker's fakechroot engine libraries. Nothing can be cited at a line inside a tarball. Re-fetchable from the upstream release the tree names |
| `references/89luca89__lilipod` | `tree/vendor` | 15 MB of vendored Go modules. The cited code is in `tree/pkg/` and `tree/cmd/` |
| `references/Azathothas__bit-cli` | `tree/vendor` | 11 MB. `tree/TODO/` is what this reference is here for |
| `references/mhx__dwarfs` | `tree/test` | 8.4 MB of test corpora |
| `references/apptainer__apptainer` | `tree/e2e` | 5.4 MB of end-to-end tests |
| `references/Obirvalger__vml` | `tree/vendor` | **439 MB across 19,335 files** of vendored crates, which that project ships so it can build offline. The tree drops from 19,402 files to 67. Everything cited here is in `tree/src/` and `tree/README.md` |
| `references/qemu-rs__qemu-rs` | `tree/.github/rsrc/id_rsa` | ⛔ **a private-key file in that project's CI resources.** It stays out by `.gitignore`, and deliberately: `AGENTS.md` absolute 9 says a secret never enters this tree, not expired, not redacted-looking and not in an example. Nothing here cites it |

## ⭐ The ten mined on 2026-09-11, and what each one is

⭐ **All ten are in the corpus**, fetched with `scripts/common/mine-repo.sh`,
each with its `api/` tracker and its `tree/` at a captured commit, each
reporting **0 gaps**. ⛔ An earlier reading kept four of them outside the tree
and read them at their URLs; `../docs/methodology/references.md` section 4 says
an untracked corpus exists on one machine only, and the operator ruled on
2026-09-11 that the trees are tracked here even where the upstream carries no
licence.

⚠ **The tracker pass returned almost nothing for the first four, and that is
itself the finding.** Zero issues, zero pull requests, zero comments, zero
review comments, zero releases, zero tags and zero discussions across all four:
they are solo research dumps with no maintainer argument to mine. The six tools
below are the opposite, and three of them carry real decisions.

| repository | commit | licence, read in the tree | what may be done with it |
| --- | --- | --- | --- |
| `talaria0101/sandbox-insights` | `0889f5f` | 0BSD, `tree/LICENSE` | ⭐ vendorable. Nothing here needed a copy |
| `talaria0101/nix-experiment` | `d8835f2` | 0BSD, `tree/LICENSE` | ⭐ vendorable. Nothing here needed a copy |
| `Azathothas/sandbox-insights` | `bcf415c` | ⭐ **0BSD**, `tree/LICENSE`, full Zero-Clause BSD text | ⭐ vendorable. The badge said `NOASSERTION` and was wrong |
| `talaria0101/vm-research` | `7697b9b` | ⛔ **none.** No licence file, and the badge is null | ⛔ **copy nothing.** Tracked as evidence by operator ruling; take mechanisms only |
| `Azathothas/memfd-ng` | `5da5803` | 0BSD | ⭐ vendorable, and it is the operator's own. [deps.md](deps.md) T-0909 measures it |
| `hust-open-atom-club/Vex` | — | MIT, `tree/LICENSE` | ⭐ vendorable. ⚠ Wanted for its **shape**: a Docker-like CLI over saved `qemu-system-*` configurations |
| `cubic-vm/cubic` | — | ⭐ **`MIT OR Apache-2.0`**, `tree/Cargo.toml`, both files present | ⭐ vendorable. The badge said Apache-2.0 alone. Wanted for its verb set and one shipped defect |
| `Obirvalger/vml` | — | MIT, `tree/LICENSE` | ⭐ vendorable. Wanted for the machine-as-a-directory shape |
| `gevico/tcg-rs` | — | MIT, `tree/LICENSE` | ⭐ vendorable. ⚠ An emulator engine, not a manager. [podvm.md](podvm.md) T-1307 |
| `qemu-rs/qemu-rs` | — | ⛔ **GPL-2.0-or-later**, `tree/Cargo.toml:7` | ⛔ **do not vendor.** Refused; [podvm.md](podvm.md) T-1307 carries the reason |
| `carlbomsdata/winquick` | — | Apache-2.0 | vendorable with its notice. [milestones.md](milestones.md) T-1112 reads its SHAPE and takes no code |

⛔ **Two of the eleven carry no permission to copy** — `vm-research` because it
states no licence, `qemu-rs` because it states a copyleft one. That
determination is made the same way for both: the repository's own statement,
read in the tree, before anything is used.

⭐ **What is taken from all of them is a MECHANISM, never a line of text.** A
mechanism is a fact about a kernel or a tool, and a fact is not anybody's to
license. Every entry that derives from one states it in this project's own
words, with the measurement it rests on named as somebody else's.

⛔ **So every number read from them is somebody else's number, taken on somebody
else's host.** [`../docs/methodology/experiments.md`](../docs/methodology/experiments.md)
is what turns one into a measurement here: reproduce the mechanism, and expect
the number to differ.

## Gaps

⛔ **A silently skipped source is the failure the procedure exists to prevent.**

- **Discussions were not fetched for any tree.** They are GraphQL only and
  `scripts/common/mine-repo.sh` reached GitHub through the credential-free REST
  proxy. Every `PROVENANCE.md` records this as its one gap. Where a project
  keeps its design argument in Discussions, this corpus does not have it.
- **`references/salsa-debian__fakeroot` has no `api/` at all.** fakeroot is not
  on GitHub; its source is a GitLab instance and its defect tracker is the
  Debian BTS, and `mine-repo.sh` speaks neither. The tracker pass of
  `docs/methodology/references.md` section 3 was **not taken** for that
  reference, and its verdict is code-only.
  `https://bugs.debian.org/cgi-bin/pkgreport.cgi?pkg=fakeroot` is where it would
  be taken.
- **`references/VHSgunzo__userland-execve` has no `tree/`.** The repository does
  not exist.
