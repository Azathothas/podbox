# Architecture

podbox turns an OCI image reference into a process on a restricted Linux host.
It measures the mechanisms the host permits, chooses the strongest honest rung,
and keeps acquisition, extraction, completion, entry, and supervision as
separate stages.

## Execution flow

1. `podbox-probe` runs disposable syscall probes and records the evidence.
2. `podbox-image` resolves the platform, fetches and verifies OCI content, and
   commits it to a content-addressed store.
3. `podbox-extract` applies layers beneath an opened root, handles whiteouts,
   and writes intended ownership to a sidecar instead of forcing `chown`.
4. `podbox-complete` makes a rootfs usable where device nodes, package-manager
   ownership changes, or host trust configuration are unavailable.
5. `podbox-enter` resolves and executes the payload after changing root. It can
   arrange a registered foreign-architecture interpreter when the host exposes
   one.
6. `podbox-supervise` owns launcher state, logs, pidfds, signals, and payload
   exit status.
7. `podbox-interpose` is the partial M6 compatibility layer for ownership
   behavior. It is built separately for glibc and musl and selected from the
   payload interpreter.

The CLI composes these stages and translates failures into Docker-shaped exit
codes. The pinned product specification is
[`references/Azathothas__container-research/tree/TOOL.md`](../references/Azathothas__container-research/tree/TOOL.md).

## Execution rungs

From strongest to weakest, selection can report `namespace`, `supervise`,
`chroot`, `interpose`, or `unsupported`. Selection is data derived from the
probe findings in `crates/podbox-probe/src/select.rs`. Entry reports the rung it
actually achieved, so a planned stronger mechanism cannot silently become a
weaker one.

Only the namespace rung can provide namespace isolation. Chroot changes path
resolution. Supervision changes lifecycle ownership. Interposition changes a
bounded set of dynamically linked library calls. The latter three must not be
described as equivalent security boundaries.

## Store layout and ownership

`podbox-image` owns the store root and its concurrency rules. The store contains
verified blobs, image records keyed by platform, extracted root filesystems,
hold records, lifecycle state, configuration, and a keyed probe cache. Paths
are opened and contained relative to trusted directory descriptors wherever a
hostile name could otherwise escape.

Extraction never claims to restore unmapped ownership. The intended uid and
gid are recorded in `.meta.jsonl` beside an extracted rootfs. That record is
metadata for completion and interposition; it does not change a kernel
permission check.

## Invariants

- One image digest identifies one verified byte sequence.
- Layer application is confined beneath the destination root.
- A store mutation is atomic or leaves the prior usable state intact.
- The payload owns its standard streams and its exit status reaches the caller.
- Platform choice is runtime data, not a compile-time architecture constant.
- A denied or unmeasured mechanism is a named state, never inferred success.
- An advisory lock is given up by an explicit unlock, and never by letting its
  descriptor close. Closing gives the lock up only once the last reference to
  the open file description goes, and a `fork` makes a second one, so a holder
  that only closes has not finished when it returns.
  [`TODO/image.md`](../TODO/image.md) T-0215 is what that cost. ⚠ The one
  exception is a lock deliberately handed to a payload, which this process must
  not take back.

The security consequences are detailed in [`SECURITY.md`](../SECURITY.md).
The ordered implementation backlog is [`TODO/PROGRESS.md`](../TODO/PROGRESS.md).
