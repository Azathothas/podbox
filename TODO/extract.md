Area: extract.

Record semantics: [task rules](RULES.md#5-entry-closure).


`crates/podvm-extract`. `TOOL.md` section 6.3, milestone M2.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

The component with the most exposure and the least room for shortcuts. Four
separate tools in the corpus stop here, all at the same wall, and the wall is
`chown` to an id the user namespace does not map.

⭐ **The measured facts this file rests on are in
`experiments/results/whiteout-contract.txt`**, taken by
`experiments/70-whiteout-contract.sh` against two images pinned by digest. Four
requirements come out of it and each is an entry below.

# T-0301: Extract in-process, at entry level, never through system `tar`

**Source:** `TOOL.md` section 6.3, `paper_final.md` section 10.4
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

Shelling out to `tar` inherits its ownership semantics, its exit
codes and its path behaviour. That is how the ownership wall
reaches five separate tools rather than one.

## Premise

Measured in the corpus, at file and line, in two of them.
`references/RuriOSS__rurima/tree/src/archive.c:95` builds a `tar`
argument vector containing `-xpf`, which preserves permissions and
ownership, and that is where its `docker pull` dies.
`references/indigo-dc__udocker/tree/udocker/container/structure.py:285-287`
also shells out, and survives only because it passes
`--no-same-owner` and `--no-same-permissions`. The flags are the
mechanism; the shell-out is not.

## Approach

Drive a tar reader at **entry level**: read a header, decide, act.
Never call a library's `unpack()` convenience, which makes the
ownership, permission and path decisions this component exists to
make. The candidate crate and its measured size delta are T-0907.

## Decision

In-process over a Rust tar reader, rather than a vendored C
libarchive. libarchive is where `short write: -20` came from
(T-0203), it is a C dependency that complicates `crt-static`, and
the entry-level API podvm needs is the smaller half of what it
offers.

## Prove

`./experiments/220-extract-path-safety.sh` exits 0, and
`strace -f -e trace=execve -o /tmp/e.txt podvm extract --force public.ecr.aws/docker/library/alpine:3.20; grep -c 'execve.*"tar"' /tmp/e.txt` prints 0


## Correction

**Corrected 2026-11-11.** The script experiments/220-extract-path-safety.sh
is gone (T-1734): the tests of podcrates' `pod-extract` prove its six
checks (A `an_entry_traversing_a_symlink_the_same_layer_made_is_refused`,
which also asserts that nothing landed outside; B `a_dotdot_path_is_refused`;
C `an_absolute_path_is_refused`; D `a_hard_link_out_of_the_destination_is_refused`;
E `an_absolute_symlink_is_created_and_points_where_the_image_said`), which
podcrates' CI runs at the tag that podvm pins (`v0.22.0`).

## Done

**Done, 2026-09-08.** `src/apply.rs` of `pod-extract` (PodUtils/podcrates) drives
`tar::Archive::entries()` and makes every decision itself. 515 entries out of a
real `alpine`, 4,664 out of a two-layer `voidlinux-musl`.

⭐ **Measured on 2026-09-08, both halves.** The shipped binary reports
`statically linked` and carries no `archive` anything; a full extraction issues
**one** `execve`, which is podvm itself. The `Prove` above is rewritten because
the original piped `grep -c` into `grep -qx 0`, and `AGENTS.md` names that
exact trap twice: `grep -c` exits 1 on zero matches, so the pipeline's status
was `grep`'s and the clause could not fail the way it was written.

⚠ **A finding about the crate, not about podvm.** `tar::Builder` REFUSES to
write a member whose path contains `..` ("paths in archives must not have
`..`"), and `tar::Archive`'s reader hands that same member straight to the
caller. The writer's validation is not the reader's, so the hostile archives in
experiments/220-extract-path-safety.sh (retired by T-1734) and in the crate's own tests are built
at the header level. Taking a Rust tar crate does not make this safe by itself,
which is what [T-0304](extract.md) is for.

# T-0302: Ownership-neutral extraction plus the sidecar

**Source:** `TOOL.md` section 6.3, `paper_final.md` section 9.1 and section 10.4
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

`chown` and `lchown` to an unmapped id return **`EINVAL`**, not
`EPERM`, and that is where GNU tar, containers/storage's layer
applier, Apptainer's Go unpacker, pacman's `DownloadUser` and
rurima's `tar -xpf` all stop. `/etc/shadow` is the usual first
casualty because it is `root:shadow`, gid 42.

## Premise

⭐ **Measured here, against a pinned image.**
`experiments/results/whiteout-contract.txt` check B reads
`-rw-r----- 0/42 ... etc/shadow` out of
`alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc`.
The wall itself is at
`references/containers__storage/tree/pkg/archive/archive.go:798-811`:
with `chownOpts` nil it takes the ids straight from the tar header
and calls `idtools.SafeLchown`. The switch is at
`references/containers__storage/tree/pkg/archive/archive.go:1104`.

## Approach

Never restore ownership by default. Record intended metadata in a
sidecar keyed by path, and apply it only where the target id is
mapped:

```json
{"path":"etc/shadow","uid":0,"gid":42,"mode":"0640",
 "applied":{"uid":0,"gid":0},"reason":"gid 42 unmapped"}
```

The sidecar buys three things `--no-same-owner` alone does not:
faithful re-export, an honest answer to a workload that asks who
owns a file, and a diagnostic that names the gap.
⛔ It changes no kernel permission check and must never be
presented as if it does.
The design to copy is `udocker`'s, which is ownership-neutral by
construction rather than by recovery:
`references/indigo-dc__udocker/tree/udocker/container/structure.py:285-287`.

## Decision

A sidecar, not `--no-same-owner` semantics alone. The alternative
loses the intended metadata, and T-0704's interposer needs it to
answer a `stat` truthfully.
⚠ **A note that corrects the corpus, not a decision.**
`references/containers__storage/tree/pkg/archive/archive.go:804-808`
already carries `ignoreChownErrors`, which turns exactly this
failure into a warning, and it is reachable by configuration:
`references/containers__storage/tree/types/options.go:473-474` and
`references/containers__storage/tree/drivers/vfs/driver.go:59-62`.
That does not change podvm's design and it does change a claim
about podman, which is T-0205.

## Prove

`podvm pull public.ecr.aws/docker/library/alpine:3.20 && podvm extract public.ecr.aws/docker/library/alpine:3.20 && jq -e 'select(.path=="etc/shadow") | .uid==0 and .gid==42 and .applied.gid==0' "$(podvm inspect --format '{{.RootfsPath}}' public.ecr.aws/docker/library/alpine:3.20)/../.meta.jsonl"`

## Done

**Done, 2026-09-08.** `src/sidecar.rs` of `pod-extract` (PodUtils/podcrates), written per entry
by `src/apply.rs` of `pod-extract` (PodUtils/podcrates). The `Prove` above runs and exits 0
against a real `alpine`, and the row it reads is, verbatim:

```
{"path":"etc/shadow","uid":0,"gid":42,"mode":"0640","applied":{"uid":0,"gid":0},"reason":"gid 42 unmapped"}
```

⭐ **One entry of 515 carried an id this machine could not apply, and it is
`etc/shadow`.** The measurement in
`experiments/results/whiteout-contract.txt` check B predicted exactly that file
and exactly that gid, and the extraction found it without being told.
`voidlinux-musl` has three: `usr/bin/wall` and `usr/bin/write` at gid 5, and
`usr/bin/xbps-uchroot` at gid 101.

⛔ **The `Prove` no longer runs `podvm run --rm`**, which is M3, because
`podvm extract` is the verb that puts the rootfs there and this entry is about
the sidecar rather than about entering the image. [T-1103](milestones.md)
carries the clauses that genuinely need M3 and is `partial` for them.

⛔ **The honest half is asserted too.** A test reading only the sidecar would
pass on an implementation that claimed an ownership it never applied, so
`the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply` checks the
file on disk and asserts its gid is the extractor's and NOT 42.

# T-0303: Whiteouts are matched on the basename, never with a path glob

**Source:** `TOOL.md` section 6.3; measured against `references/indigo-dc__udocker`
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

A whiteout at the **root** of a layer is silently ignored by a
selector anchored on a slash, and real OCI layers do not write a
`./` prefix, so there is no slash to anchor on.

## Premise

⭐ **Measured here, and the measurement contradicted the first
reading of the code.** `experiments/70-whiteout-contract.sh`
check A reads the first member of every layer of two pinned images
and gets `bin/`, `bin` and `etc/`: no `./` prefix anywhere. Check D
then runs udocker's own selector,
`tar t --wildcards -f LAYER '*/.wh.*'`
(`references/indigo-dc__udocker/tree/udocker/container/structure.py:242`),
against a crafted archive with one whiteout at the layer root and
one nested, and the root one is missed. Output in
`experiments/results/whiteout-contract.txt`.
⚠ The crafted archive is deliberate: neither pinned image happens
to carry a root-level whiteout, so passing on them would say
nothing.

## Approach

Take the basename of every entry and test it for the `.wh.` prefix.
Handle `.wh..wh..opq` as an opaque-directory marker separately: it
removes the contents of the directory it sits in, which is what
`references/indigo-dc__udocker/tree/udocker/container/structure.py:249-256`
does by listing that directory and removing each entry.

## Decision

Basename matching over any glob. A glob is what produced the
defect, and there is no glob that is both correct for a prefixed
archive and a bare one.

## Prove

`./experiments/70-whiteout-contract.sh` exits 0

## Done

**Done, 2026-09-08.** `src/whiteout.rs` of `pod-extract` (PodUtils/podcrates).

⭐ **Mutation-proved against the defect it exists for.** Replacing the basename
test with udocker's own slash-anchored selector (`path.contains("/.wh.")`) turns
three tests red, and the one that names it is
`a_root_level_whiteout_is_found_and_a_glob_would_miss_it`. The mutation
reproduces check D's measured reading exactly: the nested whiteout is found and
the root-level one is missed.

⚠ **Two cases the entry did not name, both settled here.** `.wh.` with nothing
after it names no file and is left as an ordinary entry, because turning it into
a removal of `""` would remove the directory the marker sits in; and a whiteout
whose own name carries a trailing slash is still a removal, because the
basename must not depend on the directory marker.

# T-0304: Refuse an entry that resolves outside the destination, including through a symlink from the same layer

**Source:** `TOOL.md` section 5 M2 acceptance 3, section 6.3; `paper_final.md` section 10.4
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

A crafted layer creates `evil -> /etc` as its first entry and
writes `evil/passwd` as its second. Both entries are lexically
inside the destination and the second lands outside it.

## Premise

Read, and half the mechanism is in the corpus at file and line.
`references/qaidvoid__onelf/tree/crates/onelf-format/src/manifest.rs:19-48`
is `symlink_target_within_root`, a **purely lexical** check whose
own comment says why it is lexical: "so it cannot be defeated by
filesystem races". Beside it,
`references/qaidvoid__onelf/tree/crates/onelf-format/src/manifest.rs:10-12`
rejects `""`, `.`, `..`, `/` and NUL as a path component.
⚠ **That is only half of what M2 acceptance 3 needs.** onelf
validates a symlink entry when the symlink is created. It does not
validate a later regular entry whose path traverses that symlink,
which is exactly the crafted case.

## Approach

Both halves. Validate every symlink entry lexically on creation,
as onelf does. Then resolve every entry's path against the
**accumulated tree**, with `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)`
per component where the kernel has it, and an `O_NOFOLLOW` walk
holding a directory fd where it does not. Refuse, name the entry
and the layer digest, and fail the extraction rather than skipping
the entry.

## Decision

Refuse rather than sanitize. Rewriting a hostile path to something
safe produces an image that differs from its digest with nothing
saying so, and the caller cannot tell a repaired layer from a
clean one.

## Prove

`./experiments/220-extract-path-safety.sh` exits 0


## Correction

**Corrected 2026-11-11.** The script experiments/220-extract-path-safety.sh
is gone (T-1734): the tests of podcrates' `pod-extract` prove its six
checks (A `an_entry_traversing_a_symlink_the_same_layer_made_is_refused`,
which also asserts that nothing landed outside; B `a_dotdot_path_is_refused`;
C `an_absolute_path_is_refused`; D `a_hard_link_out_of_the_destination_is_refused`;
E `an_absolute_symlink_is_created_and_points_where_the_image_said`), which
podcrates' CI runs at the tag that podvm pins (`v0.22.0`).

## Done

**Done, 2026-09-08.** `src/safety.rs` of `pod-extract` (PodUtils/podcrates), and the script
exits 0 with six checks. `experiments/results/extract-path-safety.txt` is the
reading; the refusal it captures names the layer digest and the entry:

```
refusing layer sha256:43a4608399...: the entry "evil/passwd" is not extractable
because resolving it against what the layers have written so far leaves the
destination ("evil": ELOOP).
```

⭐ **BOTH MECHANISMS ARE DRIVEN, ON ONE KERNEL, AND THAT IS DELIBERATE.**
Measured on 2026-09-08: this host is 6.18.44 and `openat2(2)` is present, so an
ordinary run never enters the `O_NOFOLLOW` walk and the walk,  which is what
every kernel before Linux 5.6 runs,  would have shipped having refused nothing.
`safety::Resolve` exists only so the tests can force it, and
`the_walk_and_openat2_refuse_the_same_traversal` asserts the two give the same
answer to the same attack AND both still admit a legitimate path.

⛔ **Mutation-proved.** Removing `RESOLVE_NO_SYMLINKS` and `O_NOFOLLOW` turns
three tests red, reporting "the traversal was not refused".

⚠ **The refusal is asserted on the FILESYSTEM, not on the exit code.** Check F
of the script and every refusal test look for the file the hostile layer tried
to write. A refusal that still wrote it would pass a check that read only the
status.

# T-0305: An absolute symlink target is rootfs-relative, not a refusal

**Source:** Found while reading `references/qaidvoid__onelf` against M2
**Category:** extract
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

A distro rootfs is full of legitimate absolute symlinks. Rejecting
them, as a bundle packer reasonably does, would refuse almost every
real image.

## Premise

⭐ **Measured here.** `experiments/results/whiteout-contract.txt`
check C reads
`lrwxrwxrwx 0/0 ... var/cache/xbps -> /var/cache/xbps` out of layer
1 of
`voidlinux/voidlinux-musl@sha256:d5c970d0015c3aa2559a2b5a87b158839969fbb1941a9ef52cc484a5392554cd`.
The refusal that must not be copied is
`references/qaidvoid__onelf/tree/crates/onelf-format/src/manifest.rs:27-29`,
which returns false for any absolute target.

## Approach

Interpret an absolute target **relative to the rootfs**, which is
what it will mean once the payload is inside the chroot, and apply
T-0304's containment check to the result. `/etc/mtab ->
/proc/self/mounts` and `/bin -> usr/bin` are then correct and
contained; a target that still escapes after rebasing is refused.
⚠ The same absolute-symlink fact bites the completion layer from
the other side: writing through `/etc/mtab` with `>` follows the
link out of the rootfs. That is T-0405.

## Decision

Rebase rather than reject, and rebase rather than dereference at
extraction time. Dereferencing would bake the build host's tree
into the image.

## Prove

`podvm pull ghcr.io/void-linux/void-musl:latest && podvm extract ghcr.io/void-linux/void-musl:latest && ! test -L "$(podvm inspect --format '{{.RootfsPath}}' ghcr.io/void-linux/void-musl:latest)/var/cache/xbps"`

## Done

**Done, 2026-09-08.** `safety::rebase_symlink_target`, and driven against the
image the entry names. `var/cache/xbps` is **gone** from the extracted tree,
layer 2's single whiteout having removed it, which is the reading the entry
predicted. **543 symlinks survive**, including `etc/mtab -> /proc/self/mounts`
(absolute, rebased and allowed), `usr/sbin -> bin` (relative) and
`var/lock -> ../run/lock` (a legitimate climb).

⛔ **Rebasing decides whether a link is ALLOWED; it never decides what is
stored.** The link on disk carries the target the image wrote, verbatim, and
check E of experiments/220-extract-path-safety.sh asserted exactly that, and the test `an_absolute_symlink_is_created_and_points_where_the_image_said` of `pod-extract` asserts it now.
Storing the rebased form would bake this host's idea of the rootfs into the
image, which is the same mistake as dereferencing and is what this entry's
Decision refuses.

⚠ The `Prove` is rewritten: the original ran `readlink` under `podvm run`,
which is M3, and asserted "prints nothing and exits non-zero",  a shape that
also passes when `run` itself is missing. It now asks the filesystem.

# T-0306: Re-permission between layers, or the second layer fails

**Source:** `references/indigo-dc__udocker`, read against `TOOL.md` section 6.3
**Category:** extract
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

Ownership-neutral extraction leaves directories with the image's
modes and the extractor's ownership. A mode-0555 directory in
layer 1 cannot be written into when layer 2 extracts over it, and
the failure looks like a corrupt layer.

## Premise

Read at file and line, not measured here.
`references/indigo-dc__udocker/tree/udocker/container/structure.py:296-303`
runs a `find` after every layer that adds `u+x` to directories
missing it, `u+w` and `u+r` to anything missing them, and `chgrp`
to the caller's own gid. Its docstring at
`references/indigo-dc__udocker/tree/udocker/container/structure.py:265-267`
states the reason: "permissions are changed to avoid file
permission issues when extracting the next layer".

## Approach

After each layer, walk what that layer wrote and add owner read,
write and, for directories, execute. Record every mode changed in
the sidecar beside the ownership, so the intended mode survives a
re-export. Do not `chgrp`: podvm extracts as the only mapped id
already.

## Decision

Widen only the owner bits, and only on entries this extraction
wrote. Widening group or other bits changes what the image means
for a payload that reads modes, and widening everything makes the
sidecar the only record of the real image.

## Prove

`cargo test -p podvm-extract a_read_only_directory_in_one_layer_can_be_written_into_by_the_next` passes, and `podvm extract ghcr.io/void-linux/void-musl:latest` succeeds on a two-layer image

## Done

**Done, 2026-09-08.** The pass runs after each layer, over what **that layer
wrote**, and adds owner read, write and execute to directories.

⭐ **The case is crafted, because it has to be.** A two-layer real image does
not reliably contain a mode-0555 directory in layer 1 that layer 2 writes into,
so passing on one would say nothing,  the same argument
[T-0303](extract.md)'s premise makes about root-level whiteouts. The crafted
pair is exactly that, and it fails without the pass.

⛔ **Owner bits only**, asserted:
`re_permissioning_widens_the_owner_and_leaves_group_and_other_alone` checks that
a 0555 directory ends 0755 and that the group and other bits are untouched.
Widening those would change what the image means for a payload that reads modes.
⛔ **No `chgrp`**, which is udocker's third action: podvm extracts as the only
mapped id already.

⚠ The `Prove` no longer runs `podvm run --rm` (M3) against
`debian:bookworm-slim`. The real two-layer image it now names is one
[T-1103](milestones.md) already pulls, so the acceptance costs no extra
registry traffic,  and Docker Hub's anonymous quota is what
[T-0206](image.md) exists for.

# T-0307: Hard links, symlinks and the layer order

**Source:** `TOOL.md` section 6.3, `paper_final.md` section 10.4
**Category:** extract
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

Dropping ownership does not discharge the rest of the OCI layer
contract, and a runtime that treats extraction as "untar each
layer" produces a tree that is not the image.

## Premise

Read. The contract is: apply layers in order; interpret whiteouts;
preserve hard links and symlinks; refuse an escaping entry.
The whiteout ordering is more precise than "after each layer":
`references/indigo-dc__udocker/tree/udocker/container/structure.py:279`
applies layer N's whiteouts to the accumulated tree **before**
extracting N, which is the order that makes a layer able to both
delete a path and recreate it.

## Approach

For each layer in manifest order: apply that layer's whiteouts to
the accumulated tree, then extract that layer's entries. Materialize
a hard link as a link to the already-extracted target within the
same destination, and refuse one whose target is outside it.
Preserve symlinks as symlinks; never dereference.

## Decision

Whiteouts before the layer's own entries, per udocker. The opposite
order deletes what the same layer just wrote.

## Prove

`./experiments/70-whiteout-contract.sh` exits 0 and `cargo test -p podvm-extract a_layer_may_delete_a_path_and_recreate_it_in_the_same_layer` passes

## Done

**Done, 2026-09-08.** `src/remove.rs` of `pod-extract` (PodUtils/podcrates) collects a layer's
whiteouts and applies them to the accumulated tree **before** that layer's own
entries, per
`references/indigo-dc__udocker/tree/udocker/container/structure.py:279`.

⭐ **The ordering is asserted by the one case that distinguishes the two
orders**: a layer that whiteouts `f` and writes `f`. Under udocker's order the
new `f` survives; under "whiteouts after the layer" it is deleted by the same
layer that wrote it. Also driven: an opaque marker empties its directory, keeps
the directory, and does not remove a file the same layer writes afterwards.

⚠ **It costs a second pass over each layer stream**, because a tar is a stream
and the whiteouts are scattered through it. The alternative is holding a
decompressed layer in memory, which for a distro base image is hundreds of
megabytes. Stated in the module header rather than left for whoever profiles it.

⛔ A hard link is materialised within the destination and one whose target is
outside it is **refused**, driven both ways. A hard link out of the rootfs is a
file the payload can write through, which the containment check would otherwise
never see because the escape is in the link target rather than in the path.

⚠ The `Prove` no longer runs `xbps-install` under `podvm run` (M3). The
`Symbolic link loop` it watched for is the T-0305 case, and the extracted tree
is asserted directly there.

# T-1315: Extract re-verifies the blob digest against the manifest

**Source:** issue 10, client beta testing 2026-09-22 (tampered store blob
  extracts cleanly); `src/layer.rs` of `pod-extract` (PodUtils/podcrates),
  `src/layer.rs` of `pod-extract` (PodUtils/podcrates),
  `crates/podvm-image/src/store/blobs.rs:48`
**Category:** extract
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`extract_layers` opens each layer with `File::open` plus a
decompressor and never recomputes a digest, so a blob whose
bytes changed after pull extracts and the payload-visible
file carries the tampered bytes while the manifest still
names the original digest. The read-side re-verification
the store already has (`Store::read_blob`) is not used on
this path: extract goes through `store.blob_path` and reads
the file directly. Gzip layers are caught only by gzip's own
CRC, zstd by frame checks, uncompressed tar by nothing.

## Premise

Measured by the reporter on the beta.1 asset and on a
rebuild: a one-byte flip at blob offset 514 extracts as
`heLlo` for honest `hello`. The code path above confirms the
mechanism on this tree. SECURITY.md states the invariant
("a blob is accepted only when its computed digest matches
the manifest"); the store is the operator's authority, so
this is a gap in the stated invariant and its enforcement.

## Approach

Hash each blob while streaming it into the decompressor in
`extract_layers` and compare against the manifest digest
before applying entries: one pass, no extra I/O. Out of
scope: changing what pull verifies, changing the store
layout, touching the threat model.

## Decision

Stream-and-compare, not `read_blob` (which would read every
byte twice) and not a document amendment (the invariant is
worth keeping).

## Prove

`cargo test -p podvm-extract` with a new test that commits
a tar through the store scaffolding, flips one content byte
in the stored blob, and asserts `extract` refuses with a
digest mismatch naming the blob; plus the reporter binary
repro (`podvm extract` after a one-byte flip exits non-zero
with the tampered bytes never written). Close issue 10 with
a comment showing the fix commit, the test output, and the
per-extract hash as the guard that stops recurrence.

## Done

**Done 2026-09-23.** Stream-and-compare, as decided, with one widening
the Decision left open: two passes, not one. `layer::open_hashed`
(`src/layer.rs` of `pod-extract` (PodUtils/podcrates)) returns the decompressor over a
reader that hashes every raw byte delivered (`Rc<RefCell<Sha256>>` inside
the file reader, so no decoder `into_inner` is needed), and
`extract_layers` (`src/lib.rs` of `pod-extract` (PodUtils/podcrates)) verifies both
passes through the store's own `DigestMismatch` shape. The first pass is
the gate: whiteouts are collected in memory and the digest is compared
before `remove::apply` or `apply_layer` touches the tree, so a blob
edited after pull is refused with the tampered bytes never written. The
second pass re-checks, so a change between the two reads is still caught.
`layer::open` is deleted; no new crate (`sha2` joins `podvm-extract`,
already ruled under T-0908).

The new test was watched fail first
(`a_blob_edited_after_pull_is_refused_and_never_written` panics with "a
tampered blob extracted cleanly"), then green after the widening, in the
lane (`rust:1.98.1-bookworm` through host podman 6.1.2). The test commits
an uncompressed tar through the store scaffolding (the sharpest case in
the Problem: no gzip CRC backstop), flips `hello` to `heLlo` in the
stored blob, and asserts `extract` errors with a mismatch naming the
manifest digest and the `greeting` file absent from the rootfs. Full
suites green beside it: podvm-extract 47, podvm 107,
podvm-image 117, podvm-probe 96, 0 failed; clippy `-D warnings` clean.

Driven on the shipped binary in the same lane, against
`public.ecr.aws/docker/library/alpine:3.20` (one layer flipped at its
midpoint, then `extract --force`):

```
tampered-extract-rc=125
podvm extract: stored blob /tmp/px-store/blobs/sha256/25f1d6b1...:
digest mismatch, expected sha256:25f1d6b1... and computed
sha256:40459f6e.... The bytes were discarded
```

The refusal names the blob, both digests, and the discard. The code order
(`lib.rs` pass 1: `open_hashed`, collect, `verified`, then apply) is why
no entry was written: the gate at line 229 fires before the first tree
mutation at line 230. One honest reading of the same run: afterwards the
rootfs directory is absent, which is `--force`'s own documented
remove-first (`images.rs` `remove_extracted`, before any blob is read),
not applied tamper; a re-pull plus extract restores it. The guard that
stops recurrence is the per-extract hash: every layer of every extract is
hashed against the manifest before its entries touch the tree, so there
is no read path left that trusts stored bytes.

# T-1698: A whiteout of `..` deletes outside the destination

**Source:** PodUtils/podcrates (the import of pod-extract, its podcrates@T-057,
  podcrates `a40d45a`); `src/remove.rs` of `pod-extract` (PodUtils/podcrates);
  `src/whiteout.rs` of `pod-extract` (PodUtils/podcrates)
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

A whiteout skipped the lexical check of every other entry.
`.wh...` at the root of a layer named `..`, and its removal
deleted the contents of the directory above the destination.
`../.wh.x` reached past the destination where the kernel has
no `openat2`. A crafted image could delete files outside its
rootfs. Each released podvm and podbox binary has the defect.

## Premise

`whiteout::classify` takes the basename, and `remove::collect`
took `dirname_parts` as it was: `..` stayed a component, and a
name of `..` or `.` went to `remove_at`.

## Approach

`remove::collect` gives each whiteout the rule of
`safety::components`: no absolute path, no NUL, no `..` in the
directory, and a name of `.` or `..` refuses the layer with the
layer digest and the entry, as T-0304 refuses any entry.

## Decision

Refuse the layer, never skip the whiteout (T-0304's rule). The
same check lives in pod-extract (podcrates `a40d45a`), which
podvm takes with podcrates v0.11.0.

## Prove

`cargo test -p podvm-extract` exits 0 with the three tests of
`drive/whiteouts.rs`, and each of them fails with the check
turned off.

## Correction

**Filed 2026-11-10.** Reported by the podcrates session, which found the
defect while it imported `podvm-extract` into pod-extract.

## Done

**Done 2026-11-10.** `remove::checked_dir` holds the rule; `collect` takes
the layer digest for the refusal. `cargo test -p podvm-extract` exits 0
(50 passed) with `a_whiteout_of_dotdot_at_the_root_refuses_the_layer`,
`a_whiteout_that_climbs_out_refuses_the_layer` and
`a_whiteout_of_dot_refuses_the_layer`. With the check turned off, each of
the three fails, and the `..` case reached the directory above the
destination ([the plant](../experiments/results/whiteout-escape-1698.txt)).

# T-1734: Retire the experiment 220-extract-path-safety.sh, and point its eight citations at the tests

**Source:** row 1516 of the refactor plan (`refactor/recon-c.md` on the
  branch `archive/podbox-worktree`), filed by [T-1673](gate.md)
**Category:** extract
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** done

## Problem

The experiment 220-extract-path-safety.sh proves the path safety of
extraction, and eight live lines of the record cite it
(`TODO/extract.md`, `TODO/image.md`, `TODO/milestones.md`, and the
header of the experiment 85-completion-symlink-escape.sh (retired by T-1735)). The code
that it proves now lives in podcrates' `pod-extract`, whose own tests
prove the same rules.

## Premise

`pod-extract` at podcrates `v0.22.0` carries the containment and the
link rules with their tests; podvm takes it by tag.

## Approach

Map each clause of the script to a test of `pod-extract` or of
`crates/podvm-extract`. Point each of the eight citations at the test
or at the crate, then delete the script.

## Decision

A clause with no test keeps the script, and the clause becomes an ask
to podcrates.

## Prove

`cargo test -p podvm-extract` exits 0, and
`git ls-files experiments | grep -c 220-extract-path-safety` prints 0.

## Done

**Done 2026-11-11.** Each of the six checks has its test in podcrates'
`pod-extract` at `v0.22.0` (`src/drive.rs`): A, an entry that traverses a
symlink of the same layer, `an_entry_traversing_a_symlink_the_same_layer_made_is_refused`,
which also asserts that nothing was written through the link; B
`a_dotdot_path_is_refused`; C `an_absolute_path_is_refused`; D
`a_hard_link_out_of_the_destination_is_refused`; E
`an_absolute_symlink_is_created_and_points_where_the_image_said`; F, nothing
outside the destination, is the tree assertion of A, and B to D refuse
before any write. The script is gone; T-0301, T-0304 and T-1103 name the
tests in a Correction, and the prose mentions name the script as past
text. The tests of `pod-extract` run in podcrates' CI at the tag that podvm
pins; podvm's own `cargo test -p podvm-extract` exits 0, and `git ls-files
experiments | grep -c 220-extract-path-safety` prints 0.




# T-1813: Extraction drops the setuid and setgid bits of a file whose mode already has owner write

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), F-14 (IMG-01; verify item 13)
**Category:** extract
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

An imported tar with `suid_rw` 4755 and `sgid_rw` 2755 extracts both as
755, while 4555, 2555 and 6555 keep their bits. On `debian:bookworm-slim`
each of the 11 setuid and setgid programs (`su`, `passwd`, `mount`,
`chage`) is 0755 on disk, and `.meta.jsonl` still says `"mode":"4755"`
with no note of the change. Measured as uid 1000; not measured as root.

## Premise

`pod-extract` (podcrates v0.42.0, `src/apply.rs`, lines 317 to 321) opens
the file with `mode | 0o600` and calls `fchmod` only when that differs
from `mode & 0o7777`. For 4755 the two are equal, and the `write` of the
content clears S_ISUID and S_ISGID (the verifier measured the kernel
side). T-0306 rules that each changed mode is recorded.

## Approach

podvm asks podcrates: call `fchmod(fd, mode & 0o7777)` after the last
write whenever the mode has a bit of `0o7000`, and record a mode that
could not be set (proposed COORD issue). podvm takes the tag. The QA
patch 0005 sets the bits again in podvm after the extraction; this entry
does not take it, because the sidecar would still disagree for a moment
and the fix belongs to the crate.

## Decision

The fix is in `pod-extract`; podvm proves it with its own test.

## Prove

`cargo test -p podvm --test extract_modes` exits 0, with a test that a layer row of mode 4755, 2755 and 6755 ends with that mode as uid 1000 and as root

## Done

**Done 2026-11-14.** podcrates `v0.44.0` keeps `S_ISUID` and
`S_ISGID` (podcrates@T-581). `cargo test -p podvm --test extract_modes`
exits 0: a layer of 4755, 2755 and 6755 extracts with those modes in
this process (root here) and as uid 1000 through `setpriv`.

# T-1881: pod-extract masks the mode of a directory, keeps the first layer's mode, and its widening can chmod a directory outside the tree through a symlink

**Source:** PodUtils/podvm#354 (COORD-107 of the tester Nemo-010, to podcrates, 2026-11-14)
**Category:** extract
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** partial

## Problem

On `v0.2.0-beta.14`, measured by the reporter on btrfs, uid 966, umask
022: `alpine:latest` extracts `tmp` and `var/tmp` as `1755` where the
layer says `1777`, and `.meta.jsonl` records `1777`; a directory that a
later layer declares again at `0700` keeps `0755`; and a layer that puts
a directory `evil` at `0555` over a symlink `evil -> /tmp/outtarget` of
an earlier layer leaves the symlink, and the widening pass chmods
`/tmp/outtarget` from `0700` to `0755`, outside the tree.

## Premise

`pod-extract` (podcrates `v0.43.0`, `src/apply.rs`): `make_dir` creates
with `mode | 0o700`, which the umask masks, and returns `Ok(())` on
`EEXIST` with no check that the name is a directory; `repermission`
calls `fchmodat` on the name, which follows a symlink; nothing puts the
mode of the image back after the last layer. The reporter gives a patch
(`openat` with `O_NOFOLLOW|O_DIRECTORY` and `fchmod`, a replace of a
non-directory, and `restore_dir_modes` after the last layer) and a test.

## Approach

podcrates fixes the crate (the reporter's COORD-107 is addressed to it);
podvm takes the tag and checks the measured table on its fixture.

## Decision

A write outside the tree is P0. The file half (setuid bits) is
COORD-096 (issue 342, podcrates@T-581).

## Prove

On the fixture of the userland rung, with the reporter's images: `tmp` of `public.ecr.aws/docker/library/alpine:3.20` is `1777` in the tree, the directory declared again at `0700` is `0700`, `evil` is a directory `0555`, and a directory outside the tree that the symlink names keeps its mode

## Done

**Partial 2026-11-14.** podvm takes podcrates `v0.44.0`
(podcrates@T-597). A layer, imported and extracted at umask 022 on this
host, with `tmp` at 1777, `again` at 0755 then 0700, a symlink `evil` to
a directory outside the tree at 0700, then `evil` at 0555: `tmp` is
1777, `again` is 0700, and the directory outside the tree keeps 0700.
Remaining: `evil` stays a symlink where the layer makes a directory,
and the owner bits after the last layer, which podcrates@T-598 fixes in
the tag after `v0.44.0`.

# T-1882: pod-extract counts the `./` archive root as an entry it could not make

**Source:** PodUtils/podvm#352 (COORD-106 of the tester Nemo-010, to podcrates, 2026-11-14)
**Category:** extract
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** blocked

## Problem

`podvm extract` of an image whose layer holds `./` and `./a.txt` prints
"podvm: 1 entries were not materialised (ArchiveRoot). podvm cannot mknod
on this class of runtime"; the tree has its one file, and nothing was
lost.

## Premise

`pod-extract` (`src/apply.rs`, the root branch of `apply_layer`) adds
the root entry to `skipped` and names it `ArchiveRoot`. The reporter
gives a patch and a test.

## Approach

podcrates removes the counter; podvm takes the tag. T-1883 waits for it.

## Decision

The root names the destination: it is no loss.

## Prove

`podvm extract` of the reporter's one-file image prints no "not materialised" line, and an image with a fifo still prints one

## Blocker

podcrates fixes `pod-extract` (PodUtils/podvm#352) as podcrates@T-599,
in `v0.44.0` (PodUtils/podcrates#153). The entry clears when podvm takes
that tag.
