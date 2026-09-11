# Research method: the compact form

This method was developed by iterating against false conclusions, hangs, data
loss, and repeated diagnostic work. It keeps the rules that changed outcomes.

## 1. Decompose the environment before testing products

Measure seven independent planes:

| Plane | Direct questions |
|---|---|
| identity | uid/gid maps, supplementary groups, `setgroups`, namespace owners |
| syscall | which syscall numbers are refused before entry; which modern equivalents execute |
| path | which exact paths are readable/writable; which denials depend on path |
| device | driver present? node visible? node creatable? descriptor obtainable another way? |
| network | bind/connect by family, protocol, address, and port |
| resource | file-size, space, inode, memory, process, and time ceilings |
| virtualization | KVM, TUN, RWX mappings, TCG, seccomp notification, ptrace |

Capabilities and uid 0 are not answers. They are inputs to checks in particular
namespaces.

## 2. Build discriminating probes

- Probe the operation required, not the privilege commonly associated with it.
- Run mutating probes in disposable children.
- Record errno and the exact arguments, not a Boolean.
- Pair a real argument with a deliberately invalid one. If an invalid path or
  PID still returns `EPERM`, a pre-entry filter is likely. If it reaches
  `ENOENT`, `ESRCH`, or `EBADF`, the syscall executed and a later gate answered.
- Carry a positive control proving the instrument can observe both branches.
- Probe old and new interfaces separately (`mount` versus
  `fsopen`/`fsmount`/`move_mount`; `unshare` versus `clone3`).
- Probe creation, attachment, and use as separate stages.
- Never use Go's multithreaded runtime as the witness for
  `unshare(CLONE_NEWUSER)`.
- Never use `mknod` device 0:0 as the witness for `CAP_MKNOD`; it is the
  whiteout special case.

## 3. Treat the instrument as a product

Every measurement worth quoting has:

- a numbered script in the tree;
- immutable local inputs or fixture digests;
- conditions printed in its output;
- a committed success or negative log;
- exit 0 for matched, 1 for contradicted, 2 for could-not-run;
- a timeout and no interactive prompt;
- an expectation so it becomes a regression gate.

Measure from outside the subject when possible. A subject can return success
without doing the work. Then ask whether the observer changed the behavior.

## 4. Separate assumptions from measurements

Study a system in at least four passes with different questions:

1. purpose and shape;
2. actual mechanism in current implementation;
3. handling of this project's hard boundary;
4. what transfers and what must not.

Treat plans and review comments as evidence of intent, never behavior. Preserve
enough local evidence that the next session can verify a claim without repeating
the entire investigation.

Adopt mechanisms, not accidental architecture. A narrow mechanism transfers
more safely than a whole tool shaped by one environment.

## 5. Review with independent lenses

At least three passes, each with a distinct failure hypothesis:

1. **Door sweep:** what other caller, backend, flag, or path reaches the
   behavior?
2. **Guard mutation:** plant the defect or failure the guard claims to catch;
   observe the unpiped exit code.
3. **Claim audit:** re-derive every published number and sentence from code,
   logs, or a versioned fixture.

High-risk work adds:

4. **Composition pass:** drive the actual end-to-end user path.
5. **Adversarial fairness pass:** attack denominators, comparison conditions,
   alternative explanations, and stronger-than-evidence wording.

## 6. Preserve the expensive failures

The following failures paid for these lessons:

- a probe reported its child's exit code rather than the operation's result;
- two probes shared a temporary path and one erased the other binary;
- a prerequisite failure appeared as a policy denial;
- a tracer produced valid artifacts then killed itself, while tests passed;
- a documented option was never registered in option parsing;
- relative paths and inherited descriptors made toy file reports look complete;
- a supervisor silently continued when it could no longer read arguments,
  turning dry-run into a false report;
- a fixed sleep made lifecycle status race;
- a full 64 MiB `/tmp` looked like unrelated hangs and regressions;
- one-shot controls supported a chain of explanations that did not reproduce.

Each belongs in tests, not institutional memory.

## 7. Work in dependency order

1. freeze inputs, observations, and claims;
2. build the probe and route selector;
3. build image acquisition and safe extraction;
4. add the cheapest execution route;
5. add lifecycle and readiness without sleeps;
6. add environment completion;
7. add stronger confinement/isolation routes;
8. add tracing and report parity across backends;
9. package only after size, file, and temporary-space ceilings are measured.

No milestone advances on an acceptance test that has never run green.
