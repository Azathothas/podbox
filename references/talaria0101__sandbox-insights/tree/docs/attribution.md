# Attribution: which mechanism produced this denial

Four mechanisms produce overlapping symptoms in this runtime class —
user namespace with a partial ID map (**N**), syscall number denylist
(**F**), path-scoped write policy (**M**), and the address-aware
network policy (**P**). A denial observed under two of them tells you
nothing about which caused it. This document is the working method for
finding out. `scripts/census.c` and `scripts/attribute.c` implement
everything here.

## The rules

1. **Probe the operation you need, never the privilege that usually
   implies it.** `unshare(CLONE_NEWNS)` failing does not mean `chroot`
   fails; `clone(CLONE_NEWNS)` succeeding does not mean `mount` works
   — on this runtime it does not. Each prerequisite gets its own probe.
2. **Every probe runs in a disposable child.** A successful `chroot`,
   `unshare` or `setuid` mutates the prober. In Go, `syscall.Unshare`
   affects only the calling *thread*, leaving the process half-
   transitioned — worse than either outcome.
3. **A probe reports the verdict of the operation it names.** Never a
   child's exit code unless the child sets it from the operation; and
   the errno must come from the process that made the call — a parent
   reporting "the child's" errno reports its own (`0`), which is how
   censuses end up full of `DENIED (0 Success)` rows that measure
   nothing. This repository's first census draft had exactly that bug.
4. **"Could not run" must never read as "denied."** Exit 1 = ran and
   was denied; exit 2 = precondition missing. A missing fixture that
   reports `ENOENT` as a policy denial is measuring itself.
5. **Record the errno, not a boolean.** `EPERM` and `EINVAL` from the
   same call mean different things: `EINVAL` from `setuid`/`chown`
   points at an unmapped ID — a *mapping* problem, unfixable by
   privilege; `EPERM` points at policy.
6. **Read what is directly readable.** `/proc/self/uid_map`,
   `/proc/self/gid_map`, `/proc/self/setgroups`, `/proc/misc` answer
   in one read what a dozen probes would infer.

## The bogus-argument discriminator

The load-bearing technique. A seccomp filter evaluates the syscall
number and six argument registers. It **cannot dereference a pointer**,
and it runs **before the syscall body**. So one call separates the
mechanisms: call the syscall with an argument the kernel would reject
*inside its own body* —

| answer to the bogus argument | meaning |
| --- | --- |
| path/pid-shaped errno (`ENOENT`, `EBADF`, `ESRCH`, `EINVAL`, `EFAULT`) | the syscall **executed**; any denial is kernel-internal (capability scope, path policy, state) |
| `EPERM` for the same argument | refused **before entry** — a filter named the syscall |

Carry **controls** so a probe that has stopped discriminating says so:
`pidfd_getfd(-1,-1)` must answer `EBADF`, `kcmp(-1,-1,…)` must answer
`ESRCH` from any unfiltered kernel. If a control answers `EPERM`, the
prober's world model is wrong, not the host's. A second control class
checks that the denial layer is *selective* rather than uniform: plain
syscalls with no plausible denylist entry (`membarrier` with a bogus
flag, `mprotect(0,0,0)`) must reach the kernel. Pre-entry `EPERM` is
attributed to the syscall-number filter as the one mechanism known to
be present (`Seccomp: 2`, measured) that denies at that layer — the
entry controls are the standing check that the layer denies a list,
not everything, and the observed live patching of the list (one gap
closed between sessions while an adjacent gap survived) is what a
number-filter does and a uniform entry layer does not.

What this technique settles on this instance (experiment 20-):
the whole mount family — including the new-API `fsopen`/`open_tree`/
`move_mount` — plus `unshare`, `setns`, `ptrace` and
`process_vm_readv` are refused pre-entry; `kcmp`, `pidfd_getfd`,
`clone3`, `openat2`, `io_uring` execute. Cost: one syscall per
question, no privilege required.

## Creation vs attachment

Probe them separately; they split along the mechanism boundary:

- In an earlier revision of this instance, `fsopen`/`fsconfig`/`fsmount`
  executed (filesystem contexts *creatable*) while `move_mount` denied
  (*attachment*). A process could build a filesystem, configure it,
  hold an fd to it — and never put it anywhere.
- `clone(CLONE_NEWNS)` succeeds today while `unshare(CLONE_NEWNS)` is
  refused: the namespace is obtainable, the mounts are not.

A runtime that asks only the old-API question learns one syscall's
worth less than the new-API question would have told it — and the
answer can flip between revisions of the same sandbox, which is why
both halves belong in the standing census.

## Probes that measure nothing

Each of these has produced a wrong verdict somewhere:

- **`mknod(S_IFCHR, 0)`** — a whiteout; the kernel exempts it from
  CAP_MKNOD. It succeeds where a real device number fails, which is
  the *pair* that proves the denial is capability-based.
- **`unshare(CLONE_NEWUSER)` from Go** — the kernel refuses it for any
  multithreaded caller with `EINVAL`, unconditionally. The Go runtime
  is always multithreaded. A Go verdict on that flag is an artefact of
  the prober's language, not of the host. Probe from a single-threaded
  helper, or use `clone`.
- **A subject's exit code as the verdict** — tools exist whose `doctor`
  exits 0 on failure. Grep the log for the decisive line; the exit code
  is a hint, not a verdict.
- **A filter's absence inferred from one allowed syscall** — the filter
  is a list; only the probed entries are known. Anything else is
  "unknown", and the honest tag for it is unknown.

## Attribution table (this instance, current revision)

| denial | mechanism | how it was settled |
| --- | --- | --- |
| `mount`/`umount2`/`pivot_root` | F | EPERM on a bogus target = pre-entry |
| `fsopen`/`open_tree`/`move_mount` | F (this revision) | EPERM on bogus args; an earlier revision split create/attach — the split is revisable, the method is not |
| `unshare`, `setns` | F | EPERM pre-entry |
| `ptrace` (every request, incl. bogus) | F | EPERM before argument validation — not YAMA, not an LSM, which answer after |
| `process_vm_readv/writev` | F | EPERM for a nonexistent pid (kernel would answer ESRCH) |
| `chown` to unmapped gid | N | EINVAL, reproduces with the filter's cooperation irrelevant — it is the mapping check |
| `setuid` non-root | N | EINVAL — rootless unreachable |
| `mknod` real device | N (initial-ns capability) | whiteout succeeds where the real number fails |
| `/proc/self/mem` write; `uid_map` writes | M/provenance | `EACCES` at open, from parent and child alike; `map_write()` checks the procfs mount's userns |
| write allowlist on `/tmp //workspace //state` | M | a filter cannot dereference paths; Landlock ABI present; allowlist content is operator policy |
| TCP bind EPERM, UDP bind OK | P | seccomp cannot read the sockaddr; address/family-aware EPERM = cgroup-BPF SOCK_ADDR class |
| `/dev/kvm` absent | node exposure, not hardware | `/proc/misc` shows the driver; every path to the node measured closed |

## The compositional trick: attribute by *removal*

When one probe cannot separate two mechanisms, compose the mechanisms
independently and remove one at a time. This repository's census models
N, F and M as separate switches (a userns with the target's map; a
filter with per-syscall toggles; a Landlock ruleset scoped to the
allowlist) so that any ambiguous error can be re-run with one denial
removed. On a host you control, this is minutes of work and it is the
difference between "fails under the sandbox" and "fails because of the
filter's rule X".
