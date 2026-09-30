# pg-toolkit source audit

The local checkout was read-only at
commit `0da2ec94847264eeee801c54b99aaee6603ca020`.
Its Git status was clean before and after capture.
No project setup or check was run.
[Provenance](../../../references/Azathothas__pg-toolkit/PROVENANCE.md)
states the selected files, read depth, omissions, and tracker gap.

## Transfer decisions

| Mechanism | Source at the captured commit | Decision |
| --- | --- | --- |
| Input names and bytes plus build conditions | internal/buildplan/plan.go, SetDigest | Adopted in T-1349 with an independent Python implementation |
| Output digest checked before Current | internal/buildplan/plan.go, Classify | Adopted in the same local build record |
| Cleanup constrained to owned outputs | internal/buildplan/plan.go, SafeToRemove | Applied to the new KVM scratch procedure; retain scoped cleanup |
| Operation probe in a separate mount namespace | internal/rootfs/run.go, EntryProbe | Confirms podbox probe discipline; no code copy needed |
| Explicit direct, installed-loader, and mapped-loader routes | tool/prun/src/exec.rs | Confirms named routes and refusal reporting; podbox already owns its entry logic |
| Static libc provider table for host shared objects | internal/wrapper/hostdlopen.go; tool/runtime/binary/elfload.c | Candidate for a separate measured loader experiment; not copied into the runtime |
| Process result separates launch failure from child status | internal/proc/proc.go, Run and classify | Confirms podbox's result model; its unbounded Run must not replace bounded podbox waits |

## Fit and limits

pg-toolkit packages applications. podbox runs root filesystems and guests.
Their mechanisms can transfer without adopting the other product's workflow.

The shared-object provider table targets a static glibc executable.
podbox's default release uses musl. A direct copy is not a compatible change.
The loader also has explicit TLS and symbol-query limitations.
Any adoption needs a pinned, isolated experiment that tests those conditions
against podbox's actual loader. Do not claim general plugin support.

The rootfs route uses namespace and mount operations.
Those operations can be denied on the target host. Copying that route would
not add an entry mechanism on a host that rejects its prerequisites.

The selected source is 0BSD with its notice retained.
GNU libiconv is not captured. pg-toolkit's output can have separate LGPL
relinking obligations when it links that library.
Do not copy its dependency packaging into a podbox release without a new
licence and build-input decision.

## Review record

Mechanism pass: read SetDigest, Classify, EntryProbe, the launcher routes,
and provider-table generation against their callers.
Failure pass: checked missing output, changed bytes, cleanup path rejection,
namespace identity checks, and process-start versus child-exit status.
Transfer pass: compared those mechanisms with podbox's manifests, entry
mechanisms, build script, and target constraints.

Only T-1349 was adopted as a capability change in this audit.
The other rows state confirmed practice or an unimplemented candidate.
