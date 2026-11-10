# references/

This directory held 46 captured upstream trees until 2026-11-06. On that
date the owner ruled that the corpus leaves the live tree and that each
reference is tracked in a GitHub issue with the label `reference` (T-1606).

| File | What |
| --- | --- |
| `MANIFEST.tsv` | Every file the trees held, with its line count, at commit `452d792`. The record gate resolves each citation into `references/` against it. |
| `README.md` | This page: each tree, its issue and the state of its upstream on 2026-11-06. |

Read a captured file from the history of `main`:

```sh
git show 452d792760abec03caa5c5c905be8d7efe6d4de5:references/TREE/PROVENANCE.md
```

Each issue names the upstream, the captured commit, the licence
determination of [the reference map](../TODO/reference-map.md), what podvm
took from the tree, and the entries that cite it. Study a new source at its
upstream and record it in an issue; do not capture it here
([the method](../docs/methodology/references.md)).

A reference issue stays open only while open work of podvm needs it. It
closes when podvm or a sibling project (PodUtils/podcrates, podssh or
podnet) takes its lesson, or when it has no remaining use (the owner,
2026-11-10). Its closing comment names where the lesson went.
[The issue map](../TODO/issues.md) gives the state of each issue and why
an open one stays open.

| Tree | Issue | Upstream on 2026-11-06 |
| --- | --- | --- |
| `89luca89__lilipod` | [#132](https://github.com/PodUtils/podvm/issues/132) | 200 |
| `apptainer__apptainer` | [#148](https://github.com/PodUtils/podvm/issues/148) | 200 |
| `Azathothas__bit-cli` | [#134](https://github.com/PodUtils/podvm/issues/134) | 200 |
| `Azathothas__container-research` | [#135](https://github.com/PodUtils/podvm/issues/135) | 200 |
| `Azathothas__memfd-ng` | [#136](https://github.com/PodUtils/podvm/issues/136) | 200 |
| `Azathothas__pg-toolkit` | [#137](https://github.com/PodUtils/podvm/issues/137) | 200 (private) |
| `Azathothas__sandbox-insights` | [#138](https://github.com/PodUtils/podvm/issues/138) | 200 |
| `Azathothas__TEMPLATE` | [#133](https://github.com/PodUtils/podvm/issues/133) | 200 |
| `carlbomsdata__winquick` | [#149](https://github.com/PodUtils/podvm/issues/149) | 200 |
| `compforge__pathshim` | [#150](https://github.com/PodUtils/podvm/issues/150) | 200 |
| `containers__bubblewrap` | [#151](https://github.com/PodUtils/podvm/issues/151) | 200 |
| `containers__podman` | [#152](https://github.com/PodUtils/podvm/issues/152) | 200 (moved to podman-container-tools/podman) |
| `containers__storage` | [#153](https://github.com/PodUtils/podvm/issues/153) | 200 |
| `cubic-vm__cubic` | [#154](https://github.com/PodUtils/podvm/issues/154) | 200 |
| `dex4er__fakechroot` | [#155](https://github.com/PodUtils/podvm/issues/155) | 200 |
| `dtolnay__faketty` | [#156](https://github.com/PodUtils/podvm/issues/156) | 200 |
| `fritzw__ld-preload-open` | [#157](https://github.com/PodUtils/podvm/issues/157) | 200 |
| `garywill__treesandbox` | [#158](https://github.com/PodUtils/podvm/issues/158) | 200 |
| `gevico__tcg-rs` | [#159](https://github.com/PodUtils/podvm/issues/159) | 200 |
| `hust-open-atom-club__Vex` | [#160](https://github.com/PodUtils/podvm/issues/160) | 200 |
| `indigo-dc__udocker` | [#161](https://github.com/PodUtils/podvm/issues/161) | 200 |
| `io12__userland-execve-rust` | [#162](https://github.com/PodUtils/podvm/issues/162) | 200 |
| `mhx__dwarfs` | [#163](https://github.com/PodUtils/podvm/issues/163) | 200 |
| `multikernel__sandlock` | [#164](https://github.com/PodUtils/podvm/issues/164) | 200 |
| `novafacing__memfd-exec` | [#165](https://github.com/PodUtils/podvm/issues/165) | 200 |
| `Obirvalger__vml` | [#139](https://github.com/PodUtils/podvm/issues/139) | 200 |
| `pkgforge-dev__cross-libc-dlopen` | [#171](https://github.com/PodUtils/podvm/issues/171) | 200 |
| `proot-me__proot` | [#173](https://github.com/PodUtils/podvm/issues/173) | 200 |
| `qaidvoid__onelf` | [#175](https://github.com/PodUtils/podvm/issues/175) | 200 |
| `qemu-rs__qemu-rs` | [#177](https://github.com/PodUtils/podvm/issues/177) | 200 |
| `RuriOSS__ruri` | [#140](https://github.com/PodUtils/podvm/issues/140) | 200 |
| `RuriOSS__rurima` | [#141](https://github.com/PodUtils/podvm/issues/141) | 200 |
| `salsa-debian__fakeroot` | [#179](https://github.com/PodUtils/podvm/issues/179) | 200 (salsa.debian.org) |
| `sigoden__fakepty` | [#181](https://github.com/PodUtils/podvm/issues/181) | 200 |
| `talaria0101__dropssh` | [#183](https://github.com/PodUtils/podvm/issues/183) | 404 (account suspended) |
| `talaria0101__nix-experiment` | [#185](https://github.com/PodUtils/podvm/issues/185) | 404 (account suspended) |
| `talaria0101__sandbox-insights` | [#187](https://github.com/PodUtils/podvm/issues/187) | 404 (account suspended) |
| `talaria0101__sandssh` | [#190](https://github.com/PodUtils/podvm/issues/190) | 404 (account suspended) |
| `talaria0101__vm-research` | [#191](https://github.com/PodUtils/podvm/issues/191) | 404 (account suspended) |
| `VHSgunzo__memfd-exec` | [#142](https://github.com/PodUtils/podvm/issues/142) | 200 |
| `VHSgunzo__pathmap` | [#143](https://github.com/PodUtils/podvm/issues/143) | 200 |
| `VHSgunzo__runimage` | [#144](https://github.com/PodUtils/podvm/issues/144) | 200 |
| `VHSgunzo__sharun` | [#145](https://github.com/PodUtils/podvm/issues/145) | 200 |
| `VHSgunzo__ulexec` | [#146](https://github.com/PodUtils/podvm/issues/146) | 200 |
| `VHSgunzo__userland-execve` | [#147](https://github.com/PodUtils/podvm/issues/147) | 404 (does not exist) |
| `ylang-ylang__dockless` | [#192](https://github.com/PodUtils/podvm/issues/192) | 200 |

The five `talaria0101` upstreams answer 404 because GitHub suspended that
account; the captures in the history of `main` are the only copies.
