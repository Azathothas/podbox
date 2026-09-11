# The walls: where tooling stops, and what the stop actually is

Every tool studied on this runtime class stops at one of four walls.
They are worth naming separately because each has a different fix, and
because three of the four present an error message that **misleads**
— the errno or the message names a cause several layers away from the
kernel decision. Each wall below is reproduced in this repository by a
numbered experiment; the reproductions are the load-bearing evidence.

## Wall 1 — ownership: `chown`/`lchown` to an unmapped ID

**The single most consequential wall**, and the most misleading,
because it returns `EINVAL` where a permission problem is expected.

```text
tar: ./etc/shadow: Cannot change ownership to uid 0, gid 42: Invalid argument
```

*(experiment 25-, against the real pinned Alpine minirootfs; the same
wall is reproduced with a synthetic archive whose entry carries gid 42)*

In a user namespace, an ID with no mapping translates to
INVALID_UID/INVALID_GID and the kernel reports that as `EINVAL`. It is
not a policy against non-zero owners, and no granted privilege clears
it — it is a mapping failure.

Who hits it: **every extractor that restores ownership by default.**
GNU tar as root (the unpacker of several container tools); Go
unpackers issuing `lchown` directly; container-storage layer appliers;
package managers that chown a download directory to a dedicated user
(pacman's `DownloadUser`); and the archive member that trips first is
usually `/etc/shadow`, because it ships `root:shadow` and gid 42 is
outside every plausible map.

The fix and its cost: extract **ownership-neutrally**
(`--no-same-owner --no-same-permissions`) — and understand that the
resulting tree's ownership differs from the image's, which changes the
behaviour of anything that checks. The better form keeps the intended
metadata in a sidecar keyed by path and applies it only where the
target ID is mapped — faithful re-export, honest answers about
ownership, and a diagnostic that names the gap, without pretending the
kernel checks were satisfied.

Related trap: **libc interposition cannot see the wall's Go form.**
Go's `os` package issues `chown`/`lchown` as direct syscalls, with or
without cgo (experiment 35-). A preload shim that answers `chown` the
way fakeroot does covers dynamically linked C only; classification of
payloads is mandatory before promising interposition (wall 3).

## Wall 2 — credentials: Go's `setgroups` on every privileged spawn

Go's `os/exec` issues `setgroups(2)` in the child whenever
`SysProcAttr.Credential` is non-nil, subject to one guard that a
`GidMappingsEnableSetgroups: true` set **without** any `GidMappings`
does not take (the first conjunct is false — this exact shape shipped
in a real container tool and cost a real debugging session). Under
mechanism N, `setgroups` is denied, so the spawn fails:

```text
fork/exec /bin/true: operation not permitted
```

*(experiment 30-, full matrix)*

The ambiguity that makes this wall expensive: a spawn that sets
namespace clone flags **and** a `Credential` fails with the identical
string whether the refused call was the clone or the setgroups — the
clone runs first, the setgroups runs in the child before `execve`, and
either failure surfaces as the same `fork/exec ...: operation not
permitted`. Whole wrong architectures have been built on reading that
message as "namespaces are denied".

The fix is one field: **`Credential.NoSetGroups = true`**. It
suppresses the `setgroups` call and nothing else; it is a no-op on
unrestricted hosts; and dropping `Credential` entirely is *not*
required and changes behaviour elsewhere. The matrix that settles it
is ten rows and is committed.

## Wall 3 — interposition reach: coverage is a property of the payload

An `LD_PRELOAD` interposer sees exactly the payload classes whose
calls go through the dynamic loader's symbol resolution:

| payload class | seen by the shim? |
| --- | --- |
| dynamically linked C (libc calls) | **yes** |
| statically linked | **no** — no loader, no preload at all |
| Go (any linkage) | **no** — raw syscalls |

*(experiment 35-)*

Two consequences:

1. **Classification before promise.** `PT_INTERP` present → dynamic;
   absent → static; Go build markers present → unreachable regardless
   of linkage. No ELF property *proves* raw-syscall coverage, so
   classification stays advisory and the tool must be able to say
   which payloads it reached — silently degraded coverage produces
   false reports (see `docs/design-lessons.md`).
2. **Seeing a call is not clearing it.** In the same experiment the
   shim intercepts `lchown(path, 0, 42)` and the kernel still answers
   `EINVAL`: path virtualization and ownership virtualization are two
   different jobs that the word "interposition" hides. A per-process
   bind view is genuinely available to the interpose tier without any
   mount privilege; ownership is not.

## Wall 4 — mounts: available namespace, unusable mount

`clone(CLONE_NEWNS)` succeeds and `mount(2)` is refused — in the fresh
namespace too. Every mount-shaped requirement is unsatisfiable
regardless of namespace acrobatics: bubblewrap-style `MS_SLAVE`
propagation changes, overlay drivers, `pivot_root`'s mount-point
requirement, FUSE (which additionally needs a `/dev/fuse` node this
runtime cannot create).

Two refinements that change conclusions:

- **The discriminating message.** Not every failure message is as
  misleading as wall 1's: bubblewrap's `Failed to make / slave:
  Operation not permitted` *proves* its earlier `clone(CLONE_NEWNS)`
  succeeded (the clone is unconditional and first; the mount is only
  reachable after), while a refused clone prints a different message.
  Reading which messages discriminate is a skill worth practicing:
  reproduce the ambiguous case with the mechanisms composed
  independently (the census's toggles) and match the observed message
  to the branch.
- **Creation vs attachment.** The new mount API can split the wall:
  filesystem contexts creatable, attachment denied (or, in this
  instance's current revision, everything refused pre-entry — the
  split's extent is revisable, the need to probe both halves is not).

The practical replacements, measured in this repository: **copy-in
seeding** (a chroot appliance receives its files by `cp`, not bind
mounts), **byte-written device nodes** (guest-side cpio unpacking,
experiment 50-), and **ownership-neutral extraction** (wall 1's fix)
for anything that would have been an overlay layer apply.

## The diagnostic pattern behind all four

The errors surface at the libc or language-runtime boundary, layers
above the kernel decision, and the interesting cases reuse an errno
(`EINVAL` from a mapping check) or a string (`operation not permitted`
from two different refused calls) that points somewhere else. A
diagnostic for this class of environment should therefore name, in one
message: the operation, the raw errno, the mechanism inferred, and the
remedy:

```text
cannot restore ownership of etc/shadow (uid 0, gid 42): EINVAL
  gid 42 is not mapped in this user namespace (/proc/self/gid_map: 0 0 1)
  extracting without ownership; intended metadata recorded in .meta.jsonl
```

A tool that prints only the raw errno in this environment is
misinforming its user most of the time.
