# Selected source capture

Repository: https://github.com/Azathothas/pg-toolkit
Commit: `0da2ec94847264eeee801c54b99aaee6603ca020`
Captured: 2026-09-30, from the local checkout, through git archive.
Licence: project-owned source is 0BSD. LICENSE is retained.

Included: LICENSE, README.md, go.mod, assets.go, docs/build.md,
internal/buildplan, internal/proc, internal/rootfs, internal/wrapper,
tool/prun, and tool/runtime/binary.

Omitted: other source, dependencies, references, binaries, output,
Git metadata, agent instructions, and caches.
No setup, build, test, gate, index update, or runtime was run in that checkout.
Only its existing CodeGraph index was queried.

Read depth: selected functions and their caller contracts, with three review
lenses: mechanism, failure path, and transfer fit.
This is not three full-tree passes. The ELF loader was not read in full.
Its deeper adoption remains work, not an implemented capability.

Tracker gap: no issue, pull request, review, release, or discussion capture.
The local source comparison does not establish tracker status or upstream
acceptance. The audit makes no such claim.
