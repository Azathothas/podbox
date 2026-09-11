# Design lessons: tools for a runtime that refuses most tooling

Everything on this page is derived from a measured wall or a measured
failure mode in this repository (`docs/walls.md`,
`docs/attribution.md`, the experiment logs). The recurring theme: on a
runtime that looks maximally privileged and is pervasively denied,
**honesty is the design constraint** — silent degradation does not
merely weaken a tool, it lies.

## 1. Report the mode you achieved; never let a weaker mode satisfy a stronger request

The mode ladder, from strongest to weakest, with the ceiling of each
measured:

| mode | requires | provides | must never claim |
| --- | --- | --- | --- |
| machine (TCG) | qemu; file sizes under the FSIZE ceiling | a separate kernel; crash/compromise containment inside the guest; fleet/fork/snapshot | hardware enforcement (this sandbox denies it; under TCG the boundary is the emulator process, not hardware) |
| appliance (chroot) | CAP_SYS_CHROOT, a prepared rootfs | a reproducible userspace; native speed | any isolation: shared kernel, PID space, namespaces; the double-chroot escape *precondition* holds for uid 0 |
| supervise (seccomp-notif) | listener + ADDFD + an argument-reading channel | mediation of syscalls whose arguments it can read | coverage of syscalls an outer layer denies before the tracee's own filter (they never notify) |
| interpose (LD_PRELOAD) | dynamically linked payloads | path virtualization for cooperative C | ownership virtualization; anything for static or Go payloads |

A user who believes they have isolation when they have a chroot is
worse off than one who is told. A workload that *requires* isolation
must fail when isolation is unavailable — never silently degrade.

`supervise` deserves its own warning: it is the one tier whose failure
mode is **silent by default**. Its argument-reading channel can vanish
while its listener keeps working; a supervisor whose per-syscall
fallback is "continue" then reports success for mediation it never
performed. Probe all three legs (listener, ADDFD, argument read) and
refuse the tier when any is missing — never fall back per call.

## 2. Degradation must be chosen and loud

Two tools can implement the same tier with opposite fallbacks: one
refuses the mode and names the reason (honest — the user learns the
boundary), the other continues the syscall directly and reports
success (false — the report now describes mediation that never
happened). The second is not a weaker version of the first; it is a
different, worse behaviour. Every fallback direction in a tool built
for this environment should be a deliberate, documented decision.

## 3. Probe at runtime; the contract moves

This runtime class is re-audited by its operator continuously. Filter
gaps close (the new-mount-API gap here was open one session and closed
the next, while the clone-family gap outlived it); network policy
relaxes live (loopback connect, port :80). Therefore:

- anything built on a gap **probes the gap every run**, cheaply, with
  the bogus-argument discriminator;
- every document that states a gap states its date;
- a closed gap is a committed negative result, not a failure — the
  probe set is the tool's early-warning system.

## 4. Classify payloads before promising anything about them

Interposition coverage, supervision coverage, even *tracing* coverage
are properties of the payload: dynamic vs static vs Go-vs-cgo vs raw
syscalls. The classification (PT_INTERP, build markers) is advisory —
no ELF property proves raw-syscall freedom — so the tool must be able
to say which payloads it reached, and the honest default when unsure
is "not reached", stated.

## 5. Diagnostics: name operation, errno, mechanism, remedy

In this environment the raw errno is actively misleading (`EINVAL`
from a mapping check; `operation not permitted` from three different
refusals; a libarchive `-20` that is a status code, not an errno). A
one-line mode banner at startup — mode, why, what it does not provide
— plus failure messages of the shape

```text
<operation> (<args>): <errno>
  <the mechanism inferred, with the file that proves it>
  <the remedy taken or available>
```

prevents the entire class of misdiagnosis this repository documents.

## 6. Probes are part of the product

A tool for this class should ship its census: the same
`scripts/census.c` discipline — disposable children, operation-named
verdicts, errno semantics preserved, "could not run" distinct from
"denied", bogus-argument discriminators with controls. It converts
"doesn't work here" bug reports into attribution rows, and it doubles
as the tool's own preflight.

## 7. The environment completes; the tool explains

Missing `/etc/passwd`, missing `/dev/null`, no `/proc` in a chroot —
these are environment-completion jobs (shims, byte-written device
nodes, static `/proc` fixtures), and each has a measured recipe here.
The design rule: complete the environment **loudly** — the shim logs
what it served; the fixture names itself; the banner says which
completions are active. A tool that completes the environment silently
is indistinguishable from one that lies about the environment.
