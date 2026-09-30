# Vendoring

## Scope

This procedure applies to copied source, carried dependencies, and patches.
A research capture is read-only evidence.
It does not become runtime code without a separate integration decision.

## License and provenance

Read the source license before copying code.
Record the captured commit and permitted use in [the reference map](../../TODO/reference-map.md).
Retain license and notice files.
Mark actual modifications in [THIRD_PARTY](../../THIRD_PARTY.md).
Do not infer permission from a repository badge.

Do not compile copyleft or unlicensed source into the 0BSD runtime.
Study mechanisms and write compatible implementation where permitted.
Respect each retained source's actual terms.

## Tree and patches

Modify carried source in place.
A clean checkout must contain the source that the build compiles.
Do not depend on a setup-time patch that an editor cannot see.

Record the original revision, changed files, reason, and reproduction.
Distinguish patched source, compiled source, and source kept for study.
A manifest patch does not establish that the source is linked.

Fix a carried defect in this tree.
Do not write to another repository.
[Remote rules](../security/remote-ops.md) define that boundary.

## Reconciliation

When a new revision overlaps a local patch, compare behavior.
Check the old reproduction against the proposed revision.
Keep a necessary local change.
Remove a redundant patch only after its proof passes.
Do not advance the recorded base while a conflict remains.

## Exclusions

Do not copy agent instruction files, credentials, build outputs, or caches.
Capture a reference with its original paths.
Document any deliberate trim.
Do not remove a required license or notice.

[Reference study](references.md) defines the reading and evidence procedure.
[History](history.md) owns superseded source records.
