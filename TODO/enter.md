Area: enter.

Record semantics: [task rules](RULES.md#5-entry-closure).


`crates/podvm-enter`. `TOOL.md` section 6.5, milestone M3.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

Five steps, and step 2 is the one that gets skipped.

```
1. resolve the rootfs path; refuse it if it is a symlink
2. open every fd the child needs WHILE THE OUTER ROOT IS STILL CURRENT
3. chroot(rootfs); chdir("/")
4. resolve the program path, now, inside the new root, in this process
5. exec
```

# T-0501: Open every descriptor before the root changes

**Source:** `TOOL.md` section 6.5, `paper_final.md` section 10.5
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

A chroot cuts off every path outside the new root. A child with no
explicit stdio opens `/dev/null`, which an extracted rootfs does
not have, and the failure names a missing file rather than a
missing step.

## Premise

Read, and the same trap is measured from the other side by T-0401:
the shell that redirects into a non-existent `/dev/null` creates a
growing file.

## Approach

Open stdio, the log sinks, any host device the config exposes via
`--device`, and a PTY pair if `-t` was asked for and T-0503 says it
is available, **before** `chroot`. A descriptor opened before the
change keeps working after it.
⭐ This is the only route by which a PTY can reach a chrooted
payload at all: allocate the pair outside, pass the descriptors in,
`TIOCSCTTY` in the child.

## Decision

Pass descriptors rather than bind-mounting anything. There is no
attach path on this runtime, so a descriptor is the only thing that
crosses the boundary.

## Prove

`podvm run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'echo hi' | grep -qx hi`

## Done

**Done 2026-09-09.** `crates/podvm-enter/src/lib.rs`. Every buffer the child
touches after the `chroot`, the argv, the environment, the working directory and
the candidate program paths, is built **before** the fork, because between
`clone` and `execve` only async-signal-safe work is permitted and nothing there
may allocate.

⚠ **stdio is inherited rather than re-opened, deliberately**, and that is what
makes `podvm run <image> cmd | consumer` give the consumer the payload's bytes.
The `/dev/null` trap this entry names is real and is why podvm never *closes*
stdio: a child with none opens `/dev/null`, which an extracted rootfs does not
have. ⛔ The PTY pair is **not implemented**; it is
[T-0503](#t-0503-probe-devptmx-and-refuse--t-by-name-where-it-is-absent)'s and
M4's, and `-t` is refused by name rather than degraded. `--device`
maps since 2026-09-26, below.

**Partial, 2026-09-25.** Issue 55 reopens the `--device` half: the
Approach names `--device` among the descriptors opened before the
root changes, the Done records it not implemented, and the parity
table carried no row for it, so `run --device` answered `unknown
option` with no reason. Either implement `--device
host[:container[:perms]]` through the device plan, or add a `None`
row naming why and let the parser refuse it by name. The parser keeps
asking the table (T-0801), not a second list. The `None` row half
landed with T-0801 (`--device` refused naming shims, not devices);
the device-map run below stays open. Fix area is
`crates/podvm-enter`'s device plan. Risk if wrong is a docker flag
answered as an omission rather than a decision. Prove is one run
mapping a host device and reading it inside, with the refusal arm
(`experiments/355-parity-curated.sh` clause 1, `--device` at 125
naming status None) beside it.

**Done 2026-09-26.** `--device HOST[:GUEST[:PERMS]]` maps through
the device plan both halves name: `src/device.rs` of `pod-enter` (PodUtils/podcrates)
parses (both paths absolute, perms from r, w and m) and opens the
host paths before the root changes (child descriptors from 41, host
descriptors relocated clear of them), and
`crates/podvm-interpose/src/device.rs` serves the exact guest
spelling as a duplicate where the real open fails ENOENT. The
access mode is checked against the row first (EACCES, before
creation, as the kernel answers it); creation is then the node's
own semantics on devices (a no-op duplicate, EEXIST for
exclusive), and a fall-through everywhere else, including a
creating open the image satisfies: that lands in the image, not
the device, because the serve only ever answers failures, and
the banner says so. `run` and `create` share the parser; the
spec travels in the container record so `start` (the launcher)
and `exec` re-open it on every entry; the machine tier refuses
naming the chroot tier; `m` parses and grants nothing, said on
the banner. `openat2` is an honest gap: it carries no serve hook,
as the procfs emulation before it.
Prove: `experiments/363-device-map.sh` exits 0
(`experiments/results/device-map.txt`): 11 clauses green on the
lane (`rust:1.98.1-bookworm` job containers, host kernel
7.2.0-WSL2-STABLE, lane-built musl release binary with both
interposer objects) - a host file and `/dev/zero` read back
byte-identical, the opener matrix (six numeric flag shapes:
EACCES on the three ungranted, descriptors on the three
granted), served creation and EEXIST through a missing parent,
the image-shadow boundary pinned by read-back, stat honestly
ENOENT, a missing host path at 125, the machine tier at 125,
the musl payload served with the static opener honestly ENOENT,
the create/start/logs round trip through the launcher, and
`exec` re-serving the record's mapping with no flag of its own
(the spec is configuration, read from the record past T-0505's
refusal of the original's environment).
`experiments/355-parity-curated.sh` exits 0 beside it with
`--device` off the refused list (40) and malformed shapes
refused in its clause 5. Unit-pinned: enter parse and round
trip (8 tests), interpose find and answer (6 tests), the CLI
spellings (1 test); lane suites green (cli 151, enter 85,
interpose 44, 0 failed).

# T-0502: Resolve the program inside the new root, in the process that changed it

**Source:** `TOOL.md` section 6.5; `paper_final.md` section 5.5
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

A prior implementation resolved the program in a child whose
environment had been replaced. The child recomputed its store path
from an empty environment, got a **relative** path, created a fresh
empty rootfs under its working directory, chrooted into **that**,
and reported `stat /bin/sh: no such file or directory` from a
rootfs where that path exists.

## Premise

Read, in the account of a shipped bug. The general shape is in the
corpus too: `references/89luca89__lilipod` is the Go implementation
the account is about, and it is refused as a seed on language
grounds and on licence grounds ([reference-map.md](reference-map.md)).

## Approach

Resolve the program path **after** `chroot` and `chdir("/")`, in
the process that did them, never in the parent and never at command
construction time. Do not strip the environment the child needs to
find the store.
⚠ A relative store path is the specific hazard. Compute every store
path from an absolute root captured before any environment change,
and assert it is absolute.

## Decision

Resolve in the child rather than pass a pre-resolved absolute path
from the parent. A parent-resolved path names the outer tree, and
the outer tree is gone.

## Prove

`env -i "$(command -v podvm)" run --rm public.ecr.aws/docker/library/alpine:3.20 /bin/sh -c 'echo ok' | grep -qx ok`

## Done

**Done 2026-09-09.** `podvm_enter::run` resolves the program **after**
`fchdir`, `chroot(".")` and `chdir("/")`, in the process that did them.
`Plan::program_candidates` builds the list before the fork and the child tries
each with `execve`; nothing is resolved in the parent.

⭐ Driven: `podvm run <image> sh -c 'echo onpath'` finds `sh` along the image's
own `PATH` **inside the new root**, and `experiments/300-run.sh` clause 4 asserts
it. A parent-resolved path would have named the outer tree, which is gone.

⚠ The relative-store hazard this entry names is closed from the other side too:
`Store::default_root` is computed before any environment change, and clause 7 of
`300-run.sh` runs with `PODVM_STORE` set to a path inside `/workspace` and
enters a rootfs under it.

# T-0503: Probe `/dev/ptmx`, and refuse `-t` by name where it is absent

**Source:** `TOOL.md` section 0, section 6.5, section 6.8; `paper_final.md` section 10.5
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`-t` either works or it does not, and the corpus disagrees with
itself about which. A degraded PTY that cannot open a terminal is
worse than a refusal, because the payload waits.

## Premise

⚠ **Unresolved, and stated as unresolved.** The target's mount
table shows exactly six device nodes bind-mounted into `/dev`
(`full`, `null`, `random`, `tty`, `urandom`, `zero`), no `ptmx` and
no `devpts`, and `mknod` cannot create one. An earlier account of
the same runtime asserts the host `/dev/ptmx` works and published no
capture. A mount table does not list plain files, so neither source
settles it. One `stat("/dev/ptmx")` on the target would, and nobody
has run one.

## Approach

`stat("/dev/ptmx")` in the **outer** environment, before the chroot,
as part of T-0101's probe set. If present, allocate the pair
outside and pass the descriptors in per T-0501. If absent, `-t` is a
**named-reason refusal**: `-t requires /dev/ptmx in the outer
environment; it is absent`.
⚠ A payload that reopens `/dev/pts/N` by name is unsupported either
way, and the banner says so rather than letting it fail deeper.

## Decision

Refuse rather than degrade. `-t` is a request for a terminal, and a
terminal that is not there cannot be approximated by a pipe without
changing what every interactive program does.

## Prove

`podvm probe --json | jq -e 'has("ptmx")' && { podvm run --rm -t public.ecr.aws/docker/library/alpine:3.20 true || podvm run --rm -t public.ecr.aws/docker/library/alpine:3.20 true 2>&1 | grep -q 'ptmx'; }`

## Done

**Done 2026-09-22.** Both verbs ask one shared predicate
(`podvm-probe/src/probes.rs` `ptmx_usable`): only the OPEN row with `Ok`
promises a pty. `run` and `exec` refuse by name otherwise, through two
call sites that shared one had three copies of the check. A five-case
unit test pins the predicate: denied, skipped, absent, and
stat-Ok-beside-open-denied all refuse. Driven on host podman 6.1.2 with
the shipped binary: the probe names ptmx present and usable, and
`run --rm -t` exits 0 with no refusal text. The refusal arm cannot fire
on this machine, which is the environmental skip 250 records rather
than a gap in the wiring. The full lane check is green.

**Partial, 2026-09-08.** The probe half is implemented and measured; the refusal
half needs `run`, which is M3.

**What holds now.** Two rows in T-0101's set, in the outer environment, before
any chroot: `stat(/dev/ptmx)` and `open(/dev/ptmx, O_RDWR)`. The first half of
the `Prove` runs and exits 0. Readings taken on 2026-09-08:

| where | `.ptmx` |
| --- | --- |
| this host, unconfined | `present: true, usable: true`, chardev 5:2 mode 666 |
| inside `experiments/20-enter-target.sh` | `present: false, usable: false`, `ENOENT` both |

⭐ **Existence is not function, so there are two rows and not one.** A device
node that stats and will not open is exactly the degraded `-t` this entry
refuses, and `.ptmx.usable` is the OPEN. ⚠ The open passes `O_NOCTTY`: without
it, opening a terminal can make it the prober's controlling terminal, which is
a mutation of the process doing the measuring.

⛔ **THE RECONSTRUCTION DOES NOT SETTLE THE PREMISE, AND MUST NOT BE READ AS
HAVING DONE SO.** It reproduces the target's `/dev` from the same mount table
this entry says cannot answer the question, so its `ENOENT` is that mount table
repeated back, not an independent measurement. ⭐ What HAS changed is that the
question is now one command on the machine that matters: `podvm probe --json |
jq .ptmx` on the target, by anyone who can reach it. Until somebody runs it
there, the premise stays unresolved and is stated as unresolved.

**Partial, 2026-09-25.** Issue 37 reopens the ordering half: where
both chroot and ptmx are denied, `run --rm -t` exits 125 at the
T-1317 gate (`run.rs` `prepare`) naming `chroot(2)` denied and never
reaches the ptmx refusal (`run.rs` below it), so the T-0503 arm is
unreachable on exactly the constrained hosts it serves. Check the
flag-specific refusal before the generic chroot gate where the flag
alone decides (ptmx before chroot for `-t`), or append the masked
reasons to the chroot message. Exit stays 125, one line per the
honesty rules. Fix area is `run.rs` `prepare` order, the `lifecycle`
gate against the `ptmx_usable` predicate. Risk if wrong is a second
ordering mask beside the first. Prove is `run --rm -t` on a fixture
denying both, naming ptmx, beside the chroot-only arm naming
chroot.

**Done, 2026-09-25.** The flag-specific refusal moved ahead of the
generic gate: `run.rs` `prepare` asks `probes::ptmx_usable` first
where `-t` was given, naming ptmx at 125 whatever else holds, and
prints the chroot sentence beside it where chroot is denied too, so
neither reason masks the other. `create` keeps the strict chroot
gate; foreground `run` takes the two-tier `ensure_entry_possible`
gate; the late `-t` check in `prepare` is gone and `exec` keeps its
own. Prove is `experiments/356-no-chroot-rung.sh` (exit 0,
`experiments/results/no-chroot-rung.txt`), whose fixture is a mount
namespace hiding `/dev/pts` plus `experiments/deny-chroot.c`
denying `chroot(2)` with EPERM:

```
$ fixture podvm run --rm -t ALPINE true; echo $?
podvm run: -t was asked for and /dev/ptmx is not usable on this machine [...]
podvm run: chroot(2) is denied on this machine as well [...]
125
```

The chroot-only arm is the same drive's `fx-refuse` (125 naming
`chroot(2) is denied`, rootfs byte-identical after), and the capable
arm is `sane-tty` (`run --rm -t` exits 0, `system info`
`.EnteredRung` reads `chroot`). The guard is the order itself with
the late check deleted: no `-t` path reaches a gate that can mask
it.

# T-0504: Refuse a rootfs path that is a symlink

**Source:** `TOOL.md` section 6.5 step 1
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`chroot` follows a symlink. A rootfs path that is one puts the
payload somewhere the runtime did not choose, and every containment
claim after that is about a different directory.

## Premise

Read.

## Approach

`lstat` the resolved rootfs path and refuse if it is a symlink,
naming the path and its target. Then hold a directory fd on it and
use `fchdir` plus `chroot(".")`, so nothing between the check and
the change can swap the path.

## Decision

Refuse rather than resolve. Resolving accepts a rootfs the store
does not own, and the store is where the ownership sidecar and the
lock (T-0204) live.

## Prove

`ln -sfn "$(podvm inspect --format '{{.RootfsPath}}' public.ecr.aws/docker/library/alpine:3.20)" /tmp/rootlink && ! podvm run --rm --rootfs /tmp/rootlink public.ecr.aws/docker/library/alpine:3.20 true`

## Done

**Done 2026-09-09.** `podvm_enter::RootDir::open` `lstat`s with
`AT_SYMLINK_NOFOLLOW`, refuses a symlink **naming its target**, refuses anything
that is not a directory, and then holds the directory open with
`O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC`. The entry uses `fchdir` on that descriptor
and `chroot(".")`, so nothing between the check and the change can swap the path.

⚠ Two tests, and the second is what makes the first mean something: a symlinked
rootfs is refused, **and the directory it points at is accepted**, so the check
is about the symlink rather than about the path being rejected wholesale.

# T-0505: `exec` is a fresh chroot, and `inspect` says so

**Source:** `TOOL.md` section 4.2, section 6.8; `paper_final.md` section 10.6
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`docker exec` enters the container's namespaces. podvm has none to
enter, so its `exec` re-runs the section 6.5 sequence against the same
rootfs. It shares the filesystem tree with the original process and
nothing else, and a caller that assumes otherwise gets a process
that cannot see the original's `/proc`, its signals or its
environment.

## Premise

Read.

## Approach

Implement `exec` as a fresh entry, and mark it **Degraded** in the
parity table with the difference stated: "a fresh chroot re-entry;
shares only the filesystem". `inspect` carries the true mode per
container so a caller can check rather than assume.
⛔ Do not imply containment. A pidfd addresses one process; it does
not contain descendants and it is not a PID namespace, and
grandchildren that reparent are outside podvm's reach.

## Decision

A fresh chroot rather than refusing `exec`. `exec` is load-bearing
for M4's lifecycle test and for every agent workflow, and the
filesystem-only sharing is enough for the overwhelming majority of
uses provided it is stated.

## Prove

`./experiments/320-cli-contract.sh` clause 4 and `./experiments/300-run.sh` clause 8: exec's stdout is the payload's, the banner says it is a fresh chroot, and `inspect --format '{{.Exec.Shares}}'` reads `filesystem`

## Done

**Done, 2026-09-09.** `crates/podvm/src/exec.rs`, `Exec.Mode` and
`Exec.Shares` on `inspect`, and clause 8 of `experiments/300-run.sh`.

⛔ **The `Prove` above was rewritten, and the original is here because the
rewrite is the finding.** It read

```
podvm run -d --name execprobe alpine:latest sleep 30 && podvm exec execprobe ... && podvm rm -f execprobe
```

and every verb in it except `exec` is M4's: `run -d`, `--name` and `rm` need
container state, which [T-1105](milestones.md) creates. ⛔ The work order puts
this entry BEFORE M4 because M4 needs it, so a `Prove` that needs M4 could never
have run in the order it was written for. The mechanism does not depend on that
half: `exec` re-enters a rootfs, and today a rootfs is named by an image
reference and at M4 by a container name that resolves to the same directory.

⭐ **The degradation is stated in three places and they cannot disagree**,
because all three read one pair of constants in `crates/podvm/src/images.rs`:
the banner on every `exec`, `podvm inspect --format '{{.Exec.Mode}}'` for a
program, and the `Degraded` row for the verb in [T-0801](cli.md)'s parity table.
A unit test asserts the banner contains what `inspect` reports, and clause 8
asserts it on the shipped binary.

⛔ **`exec` never pulls and never extracts.** A verb that creates the thing it
claims to attach to is exactly the lie `TOOL.md` section 4.1 forbids, so an
image the store does not hold, or holds and has never extracted, is a refusal
naming `podvm run`. ⚠ It also has no default command: an image's `Cmd` is what
`run` starts, and re-running it from `exec` is a process nobody asked for.

⛔ **`--format` had to learn a dotted name for this.** `{{.Exec.Shares}}` was
refused as an unsupported traversal, because the walker rejected any `.` inside
a name. A dotted name is now ONE registered name, so an unregistered one is
still refused rather than resolving half of itself and rendering a blank.

# T-0506: A foreign-architecture container, and never a rung measured by the emulator

**Source:** Found on 2026-09-09 by running `podvm probe` under `qemu-aarch64`, while closing [T-0911](deps.md)
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

⛔ **A probe run under `qemu-user` measures the emulator, and
printing its rung as the machine's is the exact lie podvm exists
to refuse.** Measured on 2026-09-09,
`experiments/results/multiarch.txt` clause 4: `qemu-aarch64`
answers `EINVAL` to `clone(CLONE_NEWNS)`, `ENOSYS` to `fsmount` and
`move_mount`, and reports `Seccomp: 0` and `NoNewPrivs: 0` however
the host is confined. Every one of those is a statement about QEMU.
⚠ It was harmless while podvm ran only where it was built. The
multiarch work makes it reachable by an ordinary user: `podvm run`
on a `linux/arm64` image on an amd64 host executes its payload,
and any `podvm probe` inside it, under precisely this emulator.

## Premise

⭐ **Measured, in this tree.** The aarch64 binary reports rung
`namespace` under qemu on a host that is also `namespace`, so the
two agree today and the agreement is a coincidence of this host
rather than evidence. The rows that disagree are printed in the
same result file, and they are the ones a caller would act on.
⚠ It is the same shape as `experiments/20-enter-target.sh`'s `/dev`
answering out of the mount table the question doubts
([T-0503](#t-0503-probe-devptmx-and-refuse--t-by-name-where-it-is-absent)),
and it has the same remedy: say which instrument answered.

## Approach

Read the image's platform and the host's, and where they differ:
1. **Say so, once, on stderr**, naming both. ⛔ Never silently.
2. Read `/proc/sys/fs/binfmt_misc` for an interpreter registered
   for the image's architecture, and whether its flags carry `F`.
   ⭐ Measured 2026-09-09, clause 5: an `F` registration holds the
   interpreter open, so a **bare chroot with no qemu inside it**
   runs the foreign binary. Without `F` the interpreter must be
   reachable by path inside the rootfs, which is the copy-in below.
3. Where an interpreter is registered without `F`, copy the static
   interpreter into the rootfs, the operator's ruling of
   2026-09-09, and ⛔ **disclose the write**: podvm put a file in
   the payload's tree, which is a thing the payload can see, and
   the honesty rules do not have an exception for convenience.
4. Where no interpreter is registered at all, **refuse** with exit
   125, naming the host platform, the image platform, whether
   `binfmt_misc` is mounted, and what would fix it. ⛔ Never exec
   into a bare `Exec format error`.
5. ⛔ **Mark the probe answer as the emulator's** wherever podvm
   runs under one, in the evidence block and in `--json`, and
   refuse to write it into `$store/probe.json` under a key that
   does not name the emulator. [T-0111](probe.md)'s cache key is
   about confinement; this adds the interpreter.

## Decision

Report and refuse before falling back. A runtime that guesses which
architecture a payload wanted has the same problem as one that
guesses a rung. ⚠ The copy-in is deliberate and disclosed rather
than refused, because refusing it would make podvm useless on
every machine whose binfmt registration predates `F`, which is most
of them.

## Prove

`podvm run --rm --platform linux/arm64 <image> /bin/true` succeeds where an interpreter is registered and exits 125 naming both platforms where none is, and `podvm probe --json` run inside it carries the interpreter in `measured_by`

## Done

**Done 2026-09-09.** `src/binfmt.rs` of `pod-enter` (PodUtils/podcrates) and the `run` verb,
driven by `experiments/300-run.sh` clause 5 and
`experiments/260-multiarch.sh` clause 5.

| | |
| --- | --- |
| `podvm run --platform linux/arm64 <image> /bin/uname -m`, on an amd64 host | **`aarch64`**, exit 0 |
| what podvm said on stderr | it named the image's platform, the host's, and the interpreter, and that the binfmt `F` flag makes it reachable inside the chroot |
| `--platform linux/riscv64`, nothing registered | exit **125**, naming the ELF machine it looked for, how many registrations are enabled, and that installing `qemu-user-static` is what docker and podman rely on too |

⛔ **podvm reads the registrations and never writes one.** Registering an
interpreter is a machine-wide change and podvm is not the machine's owner.
What it does write, on the operator's ruling of 2026-09-09, is the interpreter
**into the rootfs** where the registration lacks `F`, and that write is
disclosed in the banner naming the exact path: the payload can see that file,
and the honesty rules have no exception for a helpful edit.

⚠ **The `F` flag is read out of the registration rather than assumed.** Without
it the kernel opens the interpreter by path **at exec time**, and that path is
inside the new root; with it the interpreter is opened when the registration is
made and held. Measured, `experiments/results/multiarch.txt` clause 5: an `F`
registration ran an aarch64 binary in a bare chroot containing nothing but that
binary.

⚠ **The `e_machine` is parsed out of the registration's own magic**, at byte 18
of the ELF header, so podvm answers "is there an interpreter for THIS image"
rather than "is there any interpreter at all". A magic too short to reach byte
18 does not select on the architecture, and podvm treats that as no match
rather than as a match.

**Point 5 done, 2026-09-09.** `pod_hostprobe::interp`,
`measured_by` in `report::document`, a line in the banner, and `interpreter` as
the eighth component of the cache key. Driven by
`experiments/260-multiarch.sh` clause 6.

| | |
| --- | --- |
| `measured_by.emulated`, aarch64 podvm under qemu on this amd64 host | **`true`** |
| `measured_by.interpreter` and `cache_key.interpreter` | both `/usr/bin/qemu-aarch64-static` |
| what the banner says, beside the rung | `⛔ MEASURED BY /usr/bin/qemu-aarch64-static, NOT BY THIS MACHINE` |
| that answer offered to a native podvm sharing the store | refused: `the instrument changed` |

⭐ **What can be detected was measured rather than assumed**, by running the
arm64 rootfs's own tools under `qemu-aarch64-static`:

| asked | under the emulator | usable |
| --- | --- | --- |
| `/proc/cpuinfo` | `model name: ARMv8 Processor rev 0 (v8l)` | ⛔ no, emulated |
| `readlink /proc/self/exe` | the guest binary's own path | ⛔ no, emulated |
| `/proc/self/maps` | guest mappings only; qemu's own are filtered out | ⛔ no |
| `ls /proc/sys/fs/binfmt_misc` | the **host's** registrations | ⭐ yes |

⛔ **So the claim is conditional and says so.** podvm cannot prove it is
running natively; what it establishes is that this machine routes binaries of
podvm's **own** ELF machine through an interpreter. The other state is
`NoEvidence`, it carries the list of what was checked, and it is never printed
as "measured natively": `TODO/probe.md` T-0109 rule 1 forbids collapsing "no
evidence" into an answer.

⚠ **Two routes in, and only one leaves a `binfmt_misc` trace.** An explicit
`qemu-aarch64-static ./podvm` involves no registration at all, so the
environment is checked for the variables qemu-user reads. That is weaker
evidence and is reported as its own sentence rather than merged with the first.

⛔ **The reader moved to `podvm-probe` rather than being copied.**
`podvm-enter::binfmt` asks "can this machine execute a FOREIGN image"; this
asks the mirror question about podvm's OWN architecture. One parser answers
both, and `podvm-enter` re-exports it.

⚠ **A defect this found in `experiments/260-multiarch.sh`**: clause 4's reading
depended on whatever binfmt state the machine happened to be in, because
`podvm probe` re-execs itself once per probe and an explicitly-invoked aarch64
podvm cannot start its own children without a registration. The rung read
`namespace` on a machine that had one and `unsupported` on a machine that did
not, and the committed reading recorded only the first. The registration is now
made once, before clause 4, and the trap removes it.

# T-1317: A chroot-denied host gets an up-front refusal naming chroot, not a 125 after the work

**Source:** issue 12, client beta testing 2026-09-22 (every rung dies at
  `chroot(".")` EPERM while probe selects `interpose`);
  `crates/podvm-enter/src/lib.rs` (fchdir/chroot/chdir entry
  sequence, T-0502)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

On a host that denies `chroot(2)` itself, probe selects
`interpose` and then every entry path (`run`, forced memfd,
`start`) dies at `chroot(".")` EPERM, exit 125, after fixups,
placement and the banner have all run. The ladder rungs
change where payload bytes come from, not whether a chroot
happens, so nothing reaches the payload on exactly the class
of host the README names first.

## Premise

Measured by the reporter (uid 0, empty capability bounding
set, seccomp 2): probe prints `chroot=EPERM` beside "would
permit interpose", and the run fails post-banner. The entry
sequence design (chroot always happens) confirms the
mechanism on this tree.

## Approach

Refuse `run`/`start`/`create` up front where the entered
rung's chroot is denied, naming `chroot(2)` before any
fixup mutates the rootfs, reusing the probe's own chroot
leg as the gate. Out of scope: a no-chroot rung (rejected
below), rewriting the ladder, touching the threat model.

## Decision

Up-front refusal, for three reasons. A no-chroot rung would
be strictly weaker than today's chroot rung while costing a
subsystem, and the banner machinery would have to describe a
mode weaker than anything it names today. Documenting
chroot-denied hosts as unsupported contradicts the README's
first paragraph, which names these hosts as the audience.
The refused-rung table already has the legs to gate this.

## Prove

`podvm run` on a chroot-denied host (or a fixture that
denies chroot) exits 125 naming `chroot(2)` denied with no
rootfs mutation, and `podvm probe` maps its chroot leg to
the same refusal; on a chroot-capable host every rung runs
as before. Close issue 12 with a comment showing both
outputs and the probe-gated refusal as the guard that stops
recurrence.

## Done

**Done 2026-09-23.** Up-front refusal, as decided. One helper,
`lifecycle::ensure_chroot_usable` (`crates/podvm/src/lifecycle.rs`),
beside the T-1112 gate: it admits where `probes::chroot_usable` finds an
`Ok` chroot row and otherwise refuses at 125 naming `chroot(2)` denied,
the probe's chroot leg, and that no fixup has run. Three call sites.
`run`/`run -d`/`create` share `run::prepare`, where the probe moved ahead
of the fetch, the lock, the extraction and every fixup (its findings are
reused for the banner, so the probe still runs once); `start` probes
after the record lookup and refuses before the launcher starts. The
machine tier returns before the gate and never chroots, so it is
unaffected. `exec` is deliberately ungated: it re-enters a running
container, and with `start` gated no running container exists on a
chroot-denied host; the helper is one call away if that changes. The
refusal precedes the banner on purpose, like the other pre-banner gates:
the banner names fixups, and none have run.

The gate test was watched fail first: with `chroot_usable` stubbed
always-true, `only_an_ok_chroot_promises_entry` fails (the `Denied` and
`Skip` rows promise entry); with the real predicate it passes. The
helper's own test (`lifecycle::a_denied_chroot_is_refused_before_entry`:
`Ok` admits, `Denied`/`Skip`/absent refuse at 125) passes by name in the
lane (`rust:1.98.1-bookworm` through host podman 6.1.2), with full suites
green beside it and clippy `-D warnings` clean.

Driven on the shipped binary in the same lane with a seccomp fixture (a
C launcher denying `chroot(2)` with EPERM, then execing podvm),
`public.ecr.aws/docker/library/alpine:3.20`:

```
$ podvm probe            # unfiltered:  chroot=ok
$ fixture podvm probe    # filtered:    chroot=EPERM
$ fixture podvm run --rm IMG echo hi; echo $?
podvm run: chroot(2) is denied on this machine, so the chroot tier
cannot be entered. [...]
125
```

`create` and `start` refuse the same way at 125 under the fixture. On a
fresh store the refused `run` fetches nothing (no blobs); on the main
store all 93 rootfs files verify unchanged after all three refusals
(`sha256sum -c`), and unfiltered `run` still runs the payload. (A first
pass at that check snapshotted before the capable run and blamed one
file; the file was `etc/hosts`, rewritten by the capable run's own T-0403
fixup, not by any refusal. The rerun snapshots after every unfiltered
step, so the check isolates the refusals.) The guard that stops
recurrence is the probe-gated refusal: no entry path reaches a fixup or
an entry sequence without an `Ok` chroot row.

**Partial, 2026-09-25.** Issue 30 reopens this entry: the up-front
refusal is the correct last resort and the wrong whole answer. On a
chroot-denied host no entry path runs, including forced memfd. Two
corrections ride here. The Done names `exec` among the gated paths;
`exec` is deliberately ungated (no running container exists where
`start` is gated, and the helper is one call away if that changes).
Implement one no-chroot entry rung: userland-exec with the interposer
and no chroot for dynamic payloads, memfd exec with no chroot for
static payloads, or an automatic machine-tier bridge where the T-1301
legs hold. The banner names the entered rung and what it does not
isolate; the up-front gate stays as the last resort with the tried
rungs named. Acceptance names the target shape: chroot `EPERM` with
kvm, tun and ptmx absent, one of the two families runs with the
banner naming the rung, and refusal stays only where neither family
can run with both refusals named. Fix area is
`crates/podvm-enter` entry sequence, the `lifecycle` gate, and the
T-1003 ladder. Risk if wrong is weaker isolation stated as equal.
Prove is `run --rm alpine echo hi` at exit 0 with the banner naming
the rung on the chroot-denying fixture, beside the current refusal
test as the strict arm, with no rootfs mutation on the refusal path.

**Done, 2026-09-25.** Both no-chroot families run, decided in
`lifecycle::decide_entry` before any fixup mutates the rootfs:
dynamic payloads through the image's own loader with the image's
library directories and the interposer preloaded by host path
(`podvm-enter/src/userland.rs` `loader_plan`, `host_env`), static
payloads from a staged memfd exactly as the ladder stages it
(`Family::Memfd`). `Rung::Userland` orders after chroot and never
claims isolation: the banner, `PODVM_ACTIVE_MODE=userland` and
`system info` `.EnteredRung` read `userland` on every userland run.
Refusals name every tried rung with its missing leg at 125: scripts,
foreign images, unresolvable payloads, `--strict` against the weaker
rung, `run -d` and `create` staying chroot-gated, forced non-memfd
rungs, and forced memfd over a dynamic payload naming the unforced
loader family. Three candidates were tested for the alpine 127 seen
mid-drive (unresolvable payload; loader argv shape; applet dispatch);
the lane refuted the first two and confirmed the third: alpine's
`/bin/sh` points at the absolute `/bin/busybox`, which dangles on
the host side, so the resolved file opens and `loader_argv_for`
passes the invoked name as the applet (`busybox sh ...`), the same
selection a chrooted kernel would make. Prove is
`experiments/356-no-chroot-rung.sh` (exit 0,
`experiments/results/no-chroot-rung.txt`) on the both-denied
fixture (pinned alpine `d9e853e8`, pinned debian `833d7afe`,
locally built static hello):

```
$ fixture podvm run --rm DEBIAN sh -c 'echo $PODVM_ACTIVE_MODE'; echo $?
entering without chroot on the loader family: [...] runs [...]/usr/bin/sh [...]
userland
0
$ fixture podvm run --rm ALPINE sh -c 'echo $PODVM_ACTIVE_MODE'; echo $?
entering without chroot on the loader family: [...] runs [...]/bin/busybox
[...] the invoked name "sh" rides as the applet [...]
userland
0
$ fixture podvm run --rm static-hello:1 /hello; echo $?
entering without chroot on the memfd family: [...]
static-hi
0
$ fixture podvm run --rm --strict ALPINE true; echo $?
podvm run: --strict, and this run is degraded in 6 way(s). [...]
125
$ fixture podvm run --rm ALPINE /nonexistent-probe-target; echo $?
podvm run: chroot(2) is denied [...] no no-chroot family runs [...]
125         (rootfs byte-identical after)
```

Forced `PODVM_MODE=memfd` enters the static hello at 0, refuses
over dynamic alpine naming the unforced family at 125, and forced
`fuse` refuses naming the force at 125. Unit guards:
`userland.rs` (loader plan, lib dirs, host env, invocation name,
applet argv), `lifecycle.rs` (rung decision), `ladder.rs` (forced
no-chroot drives and refusals), `system.rs` (`.EnteredRung`
`userland` where chroot is denied); lane `cargo test` green over
the three crates beside the drive (podvm 143, podvm-enter 64,
podvm-probe 97 passed, 0 failed).

# T-1339: Enter the namespace rung where the probe permits it

**Source:** issue 59, beta.7 drive 2026-09-25 (`README.md:60-69`
  promised it, `run` never did); `crates/podvm-enter/src/lib.rs:47-61`
  (`ENTERED_RUNG` is `Chroot` on every machine),
  `pod_hostprobe::select` (returns
  `Rung::Namespace`), `crates/podvm/src/system/df_info.rs:213-216`
  (asserts the two differ)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

No TODO entry mentions the `namespace` rung, and nothing
enters it. Where the probe succeeds at
`unshare(CLONE_NEWNS)` and mount, `select.rs` returns
`Rung::Namespace` and `run` still enters a plain chroot.
The binary reports both facts (`system info` carries
`.Rung` and `.EnteredRung` with the guard asserting they
differ); the rung itself has no backlog entry.

## Premise

Read at file and line on 2026-09-25: the constant, the
selection, and the assert are as cited. No production
path calls `unshare` or `clone` with a `CLONE_NEW*`
flag; the only `unshare` in the tree is the
interposer's emulation. `docs/architecture.md:32-40`
stays carefully worded and needs no change.

## Approach

Enter the rung the probe already selects: where the
namespace legs hold, `unshare` with mount and ID maps,
then the existing chroot sequence inside; where they do
not, the chroot rung as today. The banner names the
achieved rung with what it does not isolate, and
`.EnteredRung` reports `namespace` where entered. The
`system.rs` guard stays: the two facts must never be
conflated again. The README correction of 2026-09-25
(run enters chroot, both facts carried) stands until
this lands, then reads the new behavior.

## Decision

Implement behind the probe, not beside it. No flag
selects the rung; the legs do, the way every other rung
is selected. A host that loses a leg between probe and
entry falls back to chroot with the banner naming it,
never failing a run the chroot rung could carry.
Out of scope: a network or pid namespace (mount and ID maps
first), changing the probe legs, touching the machine
tier.

## Prove

on a namespace-capable host `podvm probe` reports
`namespace` and `podvm system info --format
'{{.EnteredRung}}'` reports `namespace` after a run
that isolates a mount the host cannot see; on a denied
host both report `chroot` as today.

## Done

**Done 2026-09-26.** `run` enters the rung the probe selects.
Where it selects `namespace`, the child unshares a mount
namespace, remounts `/` recursively private, mounts a private
tmpfs on the image's `/tmp`, and runs the chroot sequence
inside; `.EnteredRung` reads `namespace`. The defect the drive
found is fixed: the entry descriptor opens pre-fork in the base
mount namespace, so `fchdir` plus `chroot(".")` anchors the new
root in that mount and hides the private tmpfs. The namespace
rung chroots by path, which resolves under the private mount,
and a (d,i) guard past the chroot compares the opened
descriptor against `/` (`chroot_landed`, `EXDEV` step 10): a
mismatch falls back to chroot with the step named, never
failing a run the chroot rung carries. No user, pid or network
namespace exists at any rung, and the banner says so
(`mount-only`, `private, tmpfs on /tmp`, `host-shared` network
and pids, the never-claim naming user, pid, network and
`/proc`). An image without a real `/tmp` directory falls back
with the cause named. Detached `start` enters through the same
gate (`spawn_selected`: a fresh probe at entry, the fallback
line in the container log). The README rung row reads the new
behavior.
Prove: `experiments/365-namespace.sh` exits 0
(`experiments/results/namespace.txt`): 3 lane clauses green
(probe below namespace, run enters chroot with no fallback
line, `EnteredRung` chroot).
`experiments/366-namespace-base.sh` exits 0
(`experiments/results/namespace-base.txt`): 5 base clauses
green on the wsl-toolkit base (kernel 7.2.0-WSL2-STABLE,
lane-built musl release binary, pinned alpine 3.20 digest) -
probe selects namespace, a run enters it with the honest
banner and `EnteredRung` namespace, the payload's `/tmp` file
reads back inside and is absent from the host rootfs with no
leaked mount, the no-`/tmp` image falls back naming the cause,
and a detached container runs with its logs carrying the
output. Unit guards: `the_entered_rung_follows_the_selection`,
`the_tmp_mount_point_is_a_real_directory`,
`the_fallback_line_names_the_cause_and_the_rung`,
`the_setup_steps_name_themselves`,
`the_chroot_guard_compares_device_and_file`,
`the_namespace_rung_reports_mount_only`,
`the_namespace_banner_names_no_procfs_mount`, and the
`/dev/null` dev/ino asserts the (d,i) guard reads. The
mid-sequence setup-failure arms (steps 7 through 9) have no
driven trigger on any reachable host: the step mapping is
unit-pinned and the sibling no-`/tmp` arm is driven green by
ns-4, but a failing `unshare` or mount past a permitting probe
is exercised nowhere. Full lane `dev.sh check` green beside
the drives.

# T-1407: Anchor userland paths inside the rootfs or refuse

**Source:** issues 69 and 70; `src/userland.rs` of `pod-enter` (PodUtils/podcrates);
  `crates/podvm-interpose/src/map.rs`
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** L
**Status:** done

## Problem

On the userland rung an absolute guest path resolves on the
host. A write lands on the host tree. A shell grandchild
execs host binaries and mixes libc. The package-install
warning in issue 69 is host mutation through the runtime.

## Premise

The rung states the pass-through in `userland.rs:15`. An
empty map table rewrites nothing (`map.rs:68`). Nothing in
`crates/podvm/src` sets `PODVM_MAPS` for a userland
run (measured by grep: zero hits). `host_env` rewrites
`LD_LIBRARY_PATH` (`userland.rs:257`) and leaves `PATH` to
the caller. First-level lookup is image-side through
`resolve_payload` (`ladder.rs:148`).

## Approach

Set a root-anchoring `PODVM_MAPS` for every userland run,
minus the never-rewrite paths that T-0706 and T-0707 name.
Refuse with a named errno where a call cannot be
virtualized. Add an explicit `--unsafe-host-paths` opt-in
for callers that want the current pass-through.

## Decision

Refusal beats a silent host write. The table is owned by
`run` and set before the payload starts. No caller edits
the table by hand.

## Prove

`cargo test -p podvm-enter` exits 0 with the new unit
guards. A tracked drive on a chroot-denying fixture reads
back a rootfs marker through an absolute path, writes no
host `/tmp` file, and runs `sh -c` compound commands with
image binaries only.

## Correction

**Open 2026-09-30 (history).** Filed from issues 69 and 70. Read the current
source before implementation. The two issues close together; keep
one acceptance clause per issue.

## Done

**Done 2026-09-30.** Every userland run sets a root-anchoring
`PODVM_MAPS` table (`userland_maps_table`: guest `/` inside the
rootfs, minus the never-rewrite paths), the caller value scrubbed
first, through one `userland_env` path for run and both exec entries;
`--unsafe-host-paths` opts into the pass-through with its own parity
row, banner line, and `--strict` refusal. `cargo test -p podvm-enter`
exits 0. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): the banner
names the anchoring, a `PODVM_MAPS=/:evil` caller value never reaches
the payload, the absolute-path read sees the image release, and a
`/tmp` write never lands on the host. Issue 69 (host mutation through
the runtime) and issue 70 (libc mixing) close on those two clauses.

# T-1408: Resolve the loader symlink inside the rootfs

**Source:** issue 73; `src/userland.rs` of `pod-enter` (PodUtils/podcrates);
  `src/memfd.rs` of `pod-enter` (PodUtils/podcrates);
  `src/abi/search.rs` of `pod-enter` (PodUtils/podcrates)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

Debian's `/lib64/ld-linux-x86-64.so.2` is a symlink to an
absolute target. The loader check follows it on the host,
where it dangles. Debian and gcc images refuse at 125 on
the userland rung.

## Premise

`is_executable` uses metadata, which follows symlinks
(`memfd.rs:49`). `resolve_in` already walks
rootfs-contained links with a bound (`abi.rs:713`).
`loader_plan` does not use it.

## Approach

Resolve `interp_guest` through `resolve_in`, bounded and
staying inside the rootfs, then run `is_executable` on the
result. Keep the refusal naming the guest path where it
still fails.

## Decision

The rootfs is the resolution root for every guest path on
this rung. The host root is never a fallback.

## Prove

`cargo test -p podvm-enter` exits 0. A tracked drive runs
`public.ecr.aws/debian/debian:bookworm-slim echo hi` and
`public.ecr.aws/docker/library/gcc:13-bookworm gcc --version`
on a chroot-denying fixture. Both images join
`experiments/356-no-chroot-rung.sh`.

## Correction

**Open 2026-09-30 (history).** Filed from issue 73. Read the current source
before implementation.

## Done

**Done 2026-09-30.** `loader_plan` resolves the guest interpreter
through `resolve_in` (bounded, rootfs-contained) before
`is_executable`; the refusal still names the guest path.
`cargo test -p podvm-enter` exits 0 with the absolute-link loader
fixture test. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): debian runs
`sh` on the fixture (clause 4) and `gcc --version` answers on the gcc
image (clause 19), both recorded with their pulled digests.

# T-1409: Answer the payload-visible exe on the loader family

**Source:** issue 74; `crates/podvm-interpose/src/path_calls.rs:63`;
  `src/ladder.rs` of `pod-enter` (PodUtils/podcrates);
  `crates/podvm/src/run/foreground.rs:310`
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Entering through the loader leaves `/proc/self/exe`
pointing at the loader. Go reads `GOROOT` from
`os.Executable` and fails. The same hazard hits any
self-locating runtime.

## Premise

The exe emulation answers only where the real call fails
with `ENOENT` (`lib.rs:1256`). On the userland rung the
host `/proc` is mounted, so the real `readlink` succeeds
with the loader path and `PODVM_GUEST_EXE` (`run.rs:798`)
is never consulted. `ladder.rs:187` documents the
chroot-only assumption.

## Approach

On loader-family entries answer `/proc/self/exe` and the
`readlinkat`, `open`, and `stat` shapes from
`PODVM_GUEST_EXE` even where the real call succeeds.
State the substitution in the banner.

## Decision

The payload sees its own path, not the loader's. The
emulation is named, never silent.

## Prove

`cargo test -p podvm-enter` exits 0. A tracked drive runs
`go version`, `go env GOROOT`, and a hello build on
`public.ecr.aws/docker/library/golang:1.24.7-bookworm`
over a chroot-denying fixture with no extra `-e`.

## Correction

**Open 2026-09-30 (history).** Filed from issue 74. Read the current source
before implementation.

## Done

**Done 2026-09-30.** The loader entry resolves the payload to its
guest path (`ladder.rs:206`), carries it in `PODVM_GUEST_EXE`
(`run.rs:996`), and the interposer answers `/proc/self/exe` and the
`readlinkat`, `open`, and `stat` shapes from it even where the real
call succeeds (`lib.rs:1541`); the banner names the substitution.
`cargo test -p podvm-enter` exits 0 with the `guest_exe` and
`push_guest_exe` arms pinned. `sh experiments/356-no-chroot-rung.sh`
exits 0 ([the drive](../experiments/results/no-chroot-rung.txt)):
clause 12 forces the exe and reads it back, clause 18 prints
exactly `/usr/bin/readlink` for `readlink /proc/self/exe` as a
first-level payload on debian, and clause 19 runs `gcc --version`
on the gcc image.
Amendment in writing: the entry's go trio is superseded. The golang
`go` binary is statically linked (`file(1)`: ELF static, no INTERP),
so the tier enters it on the memfd family, where no interposer loads
and the kernel reports the staged memfd path: `go version` answers
exit 2 naming GOROOT, and go's own telemetry child fails re-exec'ing
the memfd path. The drive pins that answer as the boundary rather
than as a regression.
Flake on record: the first rerun (lane job `dd382b1867b42856`)
missed the debian payload line once on the identical tree and
inputs; the control (lane job `c057b086fb067005`) is green with no
code change between them.

# T-1410: Exec through the no-chroot family

**Source:** issue 80; `crates/podvm/src/exec/command.rs:227`;
  `crates/podvm/src/run/prepare.rs:49`; [T-0505](enter.md)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

`run` works on a chroot-denied host but `exec` on the same
extracted rootfs fails with `EPERM`. `exec` only knows the
chroot path.

## Premise

`exec` calls `podvm_enter::run` directly (`exec.rs:444`).
It never calls `decide_entry`, which is the `run` path
(`run.rs:1078`). T-1317 scoped `exec` out deliberately and
left the helper one call away.

## Approach

Route `exec` through `decide_entry` and the loader and
memfd drive. Keep the fresh-entry-shares-filesystem
contract from T-0505. Refusals name the blocking call.

## Decision

`exec` and `run` share the entry decision. They keep
separate banners and separate records.

## Prove

`cargo test -p podvm` exits 0. A tracked drive
extracts `public.ecr.aws/docker/library/alpine:3.20` and
runs `exec echo hi` with rc 0 on a chroot-denying fixture.

## Correction

**Open 2026-09-30 (history).** Filed from issue 80. Read the current source
before implementation.

## Done

**Done 2026-09-30.** Both exec paths (container and image) route
through `decide_entry` and the loader and memfd drive, keeping the
fresh-entry-shares-filesystem contract with separate banners and
records; refusals name the blocking call. `cargo test -p podvm`
exits 0. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 15
extracts the image and runs image-exec and container-exec at 0 on the
fixture.

# T-1411: A detached launcher for the no-chroot family

**Source:** issue 72; `crates/podvm/src/run/prepare.rs:50`;
  `crates/podvm/src/lifecycle.rs:214`; [T-1317](enter.md)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

`run -d` and `create` refuse where chroot is denied, so
`start`, `ps`, `logs`, `stop`, `rm`, `wait`, `kill`,
`restart`, and `cp` have nothing to act on. `ps` prints an
empty table, which reads as no containers rather than an
unavailable mode.

## Premise

The refusal is deliberate (`run.rs:1079`). `start_one`
refuses at the chroot gate (`lifecycle.rs:213`). T-1317
records `run -d` staying chroot-gated as the decided last
resort. The pidfd and waitid supervision primitives are
present.

## Approach

Drive the userland loader family detached: fork, hold the
pidfd, record the container, keep the foreground path
unchanged. Where a call genuinely blocks detached
operation, the refusal names the call and the affected
verbs.

## Decision

This entry reverses T-1317's last-resort decision in
writing once the drive passes. Until then the refusal
stays.

## Prove

`cargo test --workspace` exits 0. A tracked drive runs
`run -d`, `ps`, `logs`, `stop`, and `rm`, plus `create`
and `start`, on one container over a chroot-denying
fixture.

## Correction

**Open 2026-09-30 (history).** Filed from issue 72. Read the current source
before implementation.

## Done

**Done 2026-09-30.** `create` records a userland entry where chroot
is denied (rung `userland`, exact loader argv in `userland_exec`,
library dirs, `PODVM_MAPS` and `PODVM_GUEST_EXE` in the stored
environment); detached `start` drives it through `spawn_userland`
with pidfd supervision as today; the foreground path is unchanged.
This reverses T-1317's last-resort decision in writing at the three
changed gates, and the strict `ensure_chroot_usable` gate is retired:
only entries with genuinely no detached driver still refuse, naming
the call with the affected verbs. `cargo test --workspace` exits 0.
`sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 9
runs `run -d`, `ps`, `logs`, `stop`, `rm` on one container and
`create` plus `start` with `ps`, `logs`, `stop`, `rm` on a second,
over the fixture. Detached payloads stay builtin-only (no grandchild
exec); the `-w /tmp` join reads back exactly `/tmp`; the grandchild
absolute-exec limit from issue 80's sibling reports pins at 127
against the chroot control at 0.

# T-1412: A missing workdir refuses naming the path

**Source:** issue 81; `src/launch.rs` of `pod-enter` (PodUtils/podcrates);
  `crates/podvm/src/run/parse.rs:226`
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`run -w` to a missing directory is accepted with exit 0.
The payload starts in the rootfs root. A script that
depends on the working directory runs in the wrong place
and reads success.

## Premise

Both rungs discard the `chdir` error: the chroot arm runs
from `/` saying so, and the no-chroot arm runs from the
rootfs with the banner naming the start (`lib.rs:1009`).
The flag parses with no validation (`run.rs:301`).

## Approach

Check the workdir before the fork on both rungs. Refuse
naming the path and the image. Keep `-w /tmp` printing
`/tmp` as the passing control.

## Decision

Refusal beats a silent wrong directory. docker creates the
directory; podvm refuses and names it, and the refusal
says so.

## Prove

`cargo test -p podvm-enter` exits 0. A tracked drive
refuses `-w /no/such/dir` with the path named and prints
`/tmp` for `-w /tmp`.

## Correction

**Open 2026-09-30 (history).** Filed from issue 81. Read the current source
before implementation.

## Done

**Done 2026-09-30.** The workdir is checked before the fork on both
rungs and both exec paths (`check_workdir`: resolved under the
rootfs, refused naming the path and the image, noting that docker
creates the directory while podvm refuses); the child race arrives
as a named step-11 error instead of a silent wrong directory.
`cargo test -p podvm-enter` exits 0. `sh experiments/356-no-chroot-rung.sh`
exits 0 ([the drive](../experiments/results/no-chroot-rung.txt)):
clause 10 refuses the missing directory on the fixture and the sane
host and prints exactly `/tmp` for the control.

# T-1413: A script refusal names the file and its shebang

**Source:** issue 78; `crates/podvm/src/lifecycle/entry.rs:219`;
  `src/memfd.rs` of `pod-enter` (PodUtils/podcrates);
  `src/ladder.rs` of `pod-enter` (PodUtils/podcrates)
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

`node:*-alpine` refuses as a `#!` script without naming the
file or its shebang. The caller cannot tell whether podvm
picked the entrypoint, a wrapper, or the wrong binary.

## Premise

`eligible` returns the unit variant `RoutePastScript`
(`memfd.rs:147`), so path and first line are already
dropped. `payload_bytes` resolves through `resolve_payload`
(`ladder.rs:148`) but returns bytes only. The refusal
(`lifecycle.rs:1779`) cannot name what it never received.

## Approach

Thread the resolved path and first line into the
`no_chroot` message. Longer term, route the interpreter
inside the rootfs where the map table covers it; that half
belongs to T-1407.

## Decision

The refusal names the exact payload path and its `#!`
line. An anonymous refusal is a defect in the message, not
a separate capability.

## Prove

`cargo test -p podvm-enter` exits 0. A tracked drive shows
the node refusal naming the resolved file and its `#!`
line, or the payload runs.

## Correction

**Open 2026-09-30 (history).** Filed from issue 78. Read the current source
before implementation.

## Done

**Done 2026-09-30.** `RoutePastScript` carries the resolved path and
first shebang line into the `no_chroot` refusal, so the node payload
refuses naming the exact file and its `#!` line instead of
anonymously. `cargo test -p podvm-enter` exits 0.
`sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 13
refuses at 126 naming `/hello.sh` and its shebang. Routing the
interpreter inside the rootfs stays with T-1407.

# T-1604: Fix the detached-stdio serial mutex and the 1-of-3 detached start failure

**Source:** CI run 36894578988, job 110478236087, 2026-10-01;
  `crates/podvm/tests/detached_stdio.rs` `:138`, `:292`;
  `experiments/340-detached-stdio.sh`
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`cargo test --workspace` exits 101 on `detached_stdio` with
`test result: FAILED. 1 passed; 2 failed`. Read in order,
this is one defect with two symptoms, not two defects. The
test at `:292` fails first on its own assertion:
`only 1 of 3 detached starts ran`, `left: 1`,
`right: 3`. The test at `:138` then dies on
`called `Result::unwrap()` on an `Err` value:
PoisonError`, because `SERIAL.lock().unwrap()` at `:138`
inherits the poison left by the test that panicked while
holding the same `Mutex` (`:25`). The same
`SERIAL.lock().unwrap()` shape is at `:167` and `:196`.

## Premise

A serialisation mutex must not propagate one test's panic
into the next. `unwrap()` on a poisoned lock makes the
second failure a report of the first, which is why the two
rows read as one confusing cascade instead of one clear
failure. The underlying 1-of-3 failure is engine- or
runner-dependent and identical on both toolchains, so it is
not a Rust 1.99 regression and not a Batch 3 change.

## Approach

Two fixes, because the cascade hides the real failure. First,
make the serialisation mutex poison-tolerant at `:138`,
`:167`, and `:196`, so a failure in one test is reported as
itself and the next test still runs. Second, find and fix
why two of three detached starts did not run on a loaded
runner; drive `experiments/340-detached-stdio.sh` in the
Linux lane under CI-like conditions before changing the
assertion, because a test may not depend on a scheduling
order it cannot control
(`docs/methodology/authoring.md`, entry fields).

## Decision

The cascade is the reporting defect and the 1-of-3 is the
product defect. Fix the mutex first so the second is
measurable, then measure the second. Do not weaken the
assertion at `:292` to accept 1 of 3: that would record a
green run over a real failure.

## Prove

`cargo test -p podvm --test detached_stdio` passes in
the Linux lane, run twice; a deliberately failing test in
the same binary does not poison its neighbours (the plant
this task owes); the saved lane log shows all three
detached starts running;
`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0.

## Done

**Done 2026-11-11.** [The lane log](../experiments/results/detached-stdio-lane.txt)
holds each clause, on the cloud host as uid 0. The plant is the test
`a_planted_panic_with_the_lock_held_poisons_no_neighbour` of
`crates/podvm/tests/detached_stdio.rs`: with `PODVM_PLANT_T1604=1` it
panics while it holds the serial lock, and its name sorts first, so with
`--test-threads=1` each neighbour takes the lock after the panic, in each
run (the order that the earlier control met only by chance). Under the
plant the plant fails and its three neighbours pass; the control, with
`unwrap()` back in the three neighbours, fails each of them on
`PoisonError`. `cargo test -p podvm --test detached_stdio -- --nocapture`
then passes two times, and each run prints its three detached starts,
each with exit 0, an id and an `inspect` row `running PID LAUNCHER -`.
`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0.

**Partial 2026-10-02.** The cascade is fixed and measured; the
one-of-three failure is not reproduced and is not root-caused, and the
entry stays open on it.

The reporting defect is gone. All three takes are now
`SERIAL.lock().unwrap_or_else(|e| e.into_inner())`, which is the shape
`tests/store_gates.rs` and `tests/curated_refusals.rs` already use in
this crate, so the file is no longer a holdout. `cargo test --workspace`
in the lane on `rustc 1.99.0` runs `detached_stdio` green, all three:
`detached_run_without_an_image_is_a_cli_error`, `run_help_names_the_detach_flag`,
and `detached_start_returns_before_the_payload_ends`
([the drive](../experiments/results/ci-lint-1.99.txt)).
`./target/x86_64-pc-windows-msvc/debug/podvm-gate.exe` exits 0.

The second defect also got a fix, and it is the one that matters for the
next attempt. The loop breaks on the first non-zero `run -d` and threw
that start's stderr away, so the suite could only report
`only 1 of 3 detached starts ran` and never name the exit that failed.
Each start now carries its stderr and the count assertion prints every
start: which iteration, which name, its exit code, its elapsed time, its
id, what `inspect` returned, and what `run -d` said. A failure that
cannot say what it saw is a failure the next session re-derives.

**The assertion was not weakened.** `:292` still requires three starts,
and the entry's decision stands.

⚠ **The plant's control failed and the plant is not claimed.** A
temporary test that panicked while holding the guard left its two
neighbours green, which is the result the task wanted, and the control
that put `unwrap()` back did **not** reproduce the cascade, so the green
is not evidence. The measured reason: a thread already parked on a
poisoned `std::sync::Mutex` is handed the lock and its `lock()` returns
`Ok`; only a thread arriving after the poison is set gets `Err`. All
three tests call `lock()` on their first line, so all three are normally
queued before any panic. CI saw the cascade because one test reached
`lock()` after the panic completed. It is a scheduling accident, the fix
removes the accident, and no test buildable on this host demonstrates it
reliably. The temporary test is not in the tree.

**CI then named the exit, and it is not a scheduling accident.** Run
36974640619 on this commit, 2026-10-02, four lane runs after the lane was
green here. The stderr this entry added is what settled it:

```
assertion `left == right` failed: only 1 of 3 detached starts ran:
  start 1 (`run -d` for det1-5332-1) exited Some(126) after 5, id "",
  inspect exited Some(1) reading []
    run -d stderr: podvm run: chroot(2) is denied on this machine, and
    no no-chroot family runs this payload: the loader family declines the
    payload: this podvm binary carries no glibc interposer object, so the
    tier is declined; ./scripts/build-interpose.sh builds them.
```

Not a flaky start and not a scheduling order: the start takes 5 seconds
and exits 126 with a refusal naming its own cause, and the refusal is
correct for the tier. What is wrong is the test, because it asserts a
detached start succeeds in an environment where the product refuses. So
the entry's own rule applies, that a test may not depend on a scheduling
order it cannot control, and here it depends on a *tier* it cannot
control: the CI job that builds the workspace tests does not build the
interposer objects, so the binary under test carries no glibc interposer
and the no-chroot family declines. Every start in that job fails the same
way; CI reported 1 of 3 only because the loop breaks on the first.

The three questions this session could not answer from the lane are now
answered from CI, and they change the fix: it is not in the mutex, not in
the launcher, and not in the store. It is that the assertion is
conditional on the environment and does not say so.

What remains, precisely: make the assertion skip with a named reason
where the tier legitimately declines, the way
`tests/store_gates.rs` already does for a host that refuses namespaces,
and keep it strict where the tier is available. The other half of the
answer, whether the CI job should build the interposer before the
workspace tests, belongs with T-1641's owner because the same
`scripts/build-interpose.sh` path defect this session measured is what
leaves the object out of the binary in the first place. Ruled out with evidence, not by assertion: image-lock
contention, the store lock being held by the payload, name collision, and
the deadline. Duration does not discriminate, because failing CI runs at
44.18 s, 48.20 s and 51.05 s overlap passing lane runs at 46.86 s,
48.01 s, 59.44 s and 65.14 s. With the stderr now carried, the next red
run names the exit instead of asking a session to guess it, and
`experiments/340-detached-stdio.sh` under a loaded runner is the
measurement the Approach asked for and this session could not drive.

**Partial 2026-10-02, the tier skip and the CI decision are in.** The
remaining clause is the lane run, not the design. The record gate,
`cargo fmt`, the four prose checks and the workflow's YAML parse all
exit 0.

The assertion is now conditional on the tier, in the way
`tests/store_gates.rs` already conditions on namespaces. Where the no-
chroot family legitimately declines, the test skips and says why:
`crates/podvm/tests/detached_stdio.rs:93-112` requires exit 126
**and** both halves of the refusal, the `decide_entry` sentence from
`lifecycle.rs:1837-1845` and the absent-object leg from
`interpose.rs:222-226`. The exit code alone is not enough and the
comment says why: 126 is also what podvm returns for a loader this
machine cannot run and for a payload that is not an ELF, and those are
real failures. Every recorded start is checked, not the first, because
the loop breaks on the first non-zero. Where the tier is available the
three-start assertion is byte for byte what it was; the entry's
decision to not weaken it stands and was not traded away.

The CI half is decided and done. The `workspace and interposer tests`
job builds the interposer before `cargo test --workspace`
(`.github/workflows/gate.yml:231-244`), mirroring the static-build job.
The premise is verified in source, not assumed: `crates/podvm/
build.rs:86-91` writes an empty placeholder object when the real one is
absent and `:101-122` records that it is empty so the runtime refuses,
so without the step the binary under test interposes nothing. The
second half of the cause is `scripts/build-interpose.sh`, which resolved
only `target/release/` and exited 2 with the binary built, because
`.cargo/config.toml` sets the musl target and the binary lands in
`target/<triple>/release/`. It now searches the flat directory and the
per-triple ones with the same first-match order `release-notes.sh` and
`verify-release.sh` already use, and its exit 2 names where it looked.

Three defects in this entry's own history were false and are corrected
here rather than carried. The mutex cascade was a scheduling accident,
already recorded above; the 1-of-3 was never a scheduling order; and
the entry's earlier statement that the test could be fixed in the mutex
was wrong, because CI's own stderr named the tier as the cause. What
the previous session measured still holds and is what made this fix
findable: a failure that cannot say what it saw is a failure the next
session has to re-derive.

What still owes: the lane run of `cargo test -p podvm --test
detached_stdio` in CI, where the skip and the strict path are both
exercised in place. It could not run from this host because `ring`'s
build script fails here through `scripts/zig-cc.sh`, which a Windows
process spawner cannot execute (`os error 193`); the failure reproduces
on the clean tree with these edits stashed. The predicate was driven
directly against the captured CI stderr and its negatives instead, and
the writer's first attempt at it was caught by those drives: a
text-only match would have skipped a start that exited 0 having
started a container. Adding the exit-code guard turned the plant red
when removed, which is the direction that matters.


**2026-11-06, the first Linux cloud session.** The lane run reproduced the
failure where chroot is permitted (uid 0): `run -d` exited 0 and took
5.1 s to 5.3 s, over the 5 s bound. A strace with stack traces put 5.4 s
of the 6.05 s in `run::trust_record`, which hashed each blob of the
record (one 193,731,527-byte layer) before each start
(`Store::verify_record_blobs`, T-1419). `trust_record` now calls
`Store::absent_record_blobs`, a `stat` for each blob: T-1419 asks for
presence, `podvm_extract::extract` hashes each layer before it writes
(T-1315), and `podvm verify` hashes all of them. The refusal says
"missing" in place of "missing or corrupt". After the change
`cargo test -p podvm --test detached_stdio` passes, 3 of 3 tests, and
`absent_record_blobs_names_a_deleted_layer_and_does_not_hash` passes.
CI, 2026-11-06: gate run 42177673037 at `0496ce7`: the step
"workspace tests" of the job "workspace and interposer tests" exits 0,
`detached_stdio` included. The steps after it had never run in CI; the
next commit fixes the two that then failed (`podvm-buildstate` not
built for experiment 393, and the diagnostics fixture of `podvm-smoke`
with no stub for `check-build-lane`). Gate run 42179344015 at
`7f87f24` is green in each of its four jobs. Remaining of the Prove: the
plant (a failing test in the same binary does not poison its
neighbours), a second lane run, and the saved lane log.

# T-1647: Stream a payload into a memfd: the 128 MiB read ceiling refuses large runtimes

**Source:** issue 90; issue 114 section 3.3 and section 6 item 6;
  `src/ladder.rs` of `pod-enter` (PodUtils/podcrates);
  `src/abi/elf.rs` of `pod-enter` (PodUtils/podcrates);
  `src/memfd.rs` of `pod-enter` (PodUtils/podcrates);
  A-V-04 of PodUtils/podcrates#39
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

On a chroot-denied host podvm refuses a payload over
134217728 bytes at 125 before it runs. `node` in node:alpine
is 149234536 bytes, and a zig toolchain binary is 172641672
bytes. `doctor` and `probe` do not report the cap, so the
image is pulled first and the run fails after.

## Premise

Two 128 MiB ceilings read the whole file into memory.
`Elf::read` (`abi.rs:157`) serves the tier classifier
(`crates/podvm/src/interpose.rs:194`). `payload_bytes`
(`ladder.rs:238`) serves `decide_entry`
(`crates/podvm/src/lifecycle/script.rs:117`), so the loader
family refuses too, though it never stages the bytes.
`memfd::stage` (`memfd.rs:108`) takes one contiguous buffer.
`memfd::eligible` (`memfd.rs:262`) reads the ELF header and
the program headers, not the whole file.

## Approach

Classify from a bounded read of the ELF header and its
tables. Stage the memfd family's payload by a copy from the
file into the memfd in bounded chunks, and hash during the
copy where a digest is needed. Bound the bytes held in
memory, not the file size. Where a size limit must stay,
report it in `doctor` and `probe` and refuse before the pull.

## Decision

File size is not a refusal reason; memory held is bounded.
A static runtime that locates its install tree through
`/proc/self/exe` cannot run from a memfd (issue 90 measured
zig). That case is out of this entry: T-1648 refuses it by
name and T-1652 serves it.

## Prove

`cargo test -p podvm-enter` exits 0 with a test that
classifies and stages a sparse 200 MiB fixture through a
bounded buffer. `cargo test -p podvm` exits 0.
`sh experiments/356-no-chroot-rung.sh` exits 0 with a clause
that runs `node -e 'console.log(1)'` from the node image
with `--entrypoint node`, past the size check.

## Correction

**Blocked 2026-11-10.** Blocker: the code of the two ceilings and of
`memfd::stage` moved to pod-enter with podcrates v0.9.0 (podvm `423e817`),
so the fix belongs to podcrates. COORD-016 (issue 222) asks for it. The
condition that clears the block: a podcrates tag whose pod-enter
classifies from a bounded read and stages in bounded chunks, taken by
podvm. Then the clause of experiment 356 (`node -e` past the size check)
proves it here.

**Filed 2026-11-06.** Quoted from issue 90 and its comments, measured on
release 0.1.0-beta.12 on the errand host:

```text
$ podvm run --entrypoint node node:alpine -e 'console.log(1)'
podvm run: chroot(2) is denied on this machine, and no no-chroot family
runs this payload: node resolved but its bytes could not be read:
.../rootfs/usr/local/bin/node is 149234536 bytes, over the
134217728-byte ceiling podvm reads a payload within.   [exit 125]
streamed into a memfd in 128 KiB chunks (sandhome shims/memexec.c):
$ time /tmp/sandhome-memexec /state/home/noexec-test/zig version
0.16.0
real    0m0.126s
$ /tmp/sandhome-memexec /state/home/noexec-test/zig env
error: unable to find zig installation directory '/memfd:sandhome (deleted)'
```

**Amended 2026-11-09.** For T-1647, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is one step of that chain: the `node` binary is
over the read ceiling (T-1661). Its clauses do not change. T-1646 holds the
end-to-end clause.

## Done

**Done 2026-11-10.** For T-1647, the last clause of the partial record below is run. `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 on the cloud host, verdict "NO-CHROOT FAMILY RUNS" ([the run](../experiments/results/no-chroot-rung.txt)): the images come from `mirror.gcr.io` by the switch of 2026-11-10, each pulled digest is in the conditions, and the capable host enters the namespace rung.

**Partial, 2026-11-10.** The block cleared: podvm took pod-enter 0.11.0
(podcrates podcrates@T-139, PodUtils/podcrates#46). pod-enter classifies from a
bounded head (`payload_head`, `interp_of_file`) and stages the memfd
family's payload from the open file in bounded chunks (`stage_file`);
the 128 MiB refusal is gone, and the crate tests a sparse 200 MiB fixture.
podvm's three call sites (`ladder.rs`, `lifecycle/userland.rs`,
`lifecycle/script.rs`) use the head. `cargo test --workspace` exits 0.
[The drive](../experiments/results/payload-over-128mib-1647.txt) on the
chroot-denying fixture: `node -e` runs at 0 three times, and node padded
to 161523528 bytes, above the old ceiling of 134217728, runs at 0.
Remaining: the full run of experiment 356 (`public.ecr.aws` answered
HTTP 429).

# T-1648: A static Go payload runs into a GOROOT error instead of a named refusal

**Source:** issue 91; issue 114 section 3.4;
  `crates/podvm/src/lifecycle/entry.rs:353`;
  `crates/podvm/src/lifecycle/entry.rs:384`;
  `crates/podvm/src/interpose.rs:365`;
  `experiments/356-no-chroot-rung.sh:610`;
  A-V-04 of PodUtils/podcrates#39
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

On a chroot-denied host `go version` from the golang image
exits 2 with the payload's own error: "cannot find GOROOT
directory". The error blames the image. The real reason
prints only under `--verbose`. The payload sees the host
tree: `go build -o /tmp/gohello` wrote a 2238818-byte file
into the host `/tmp`. That is a host write by a container
payload, with the cause hidden.

## Premise

`decide_entry` sends every static payload to the memfd
family (`lifecycle.rs:1895`), where no interposer loads. Its
banner says that absolute paths resolve on the host
(`lifecycle.rs:1919`), and the quiet default of T-1416 hides
the banner. `interpose::apply` records the Go decline as a
banner note (`crates/podvm/src/interpose.rs:365`),
which the quiet default also hides. Clause 18 of experiment
356 pins exit 2 naming GOROOT as the boundary
(`356-no-chroot-rung.sh:574`).

## Approach

On a chroot-denied host, refuse a static payload at 125
before the exec, unless `--unsafe-host-paths` opts in. That
is T-1407's flag for this exact host pass-through. The
refusal says the payload is static, says its absolute paths
resolve on the host, and names the routes: a host with
chroot, the opt-in, and the rungs of T-1651 and T-1652 where
they exist. Under the opt-in, the host-root line prints even
under `-q`. Every clause of experiment 356 that drives the
memfd family passes the opt-in. Clause 18 changes from exit
2 to exit 125.

## Decision

Refusal beats a silent host write, T-1407's rule applied to
the memfd family. podvm does not inject `GOROOT`: clause 18
already refused payload-specific magic. The measured
host-path `GOROOT` recipe stays a caller choice under the
opt-in.

## Prove

`cargo test -p podvm` exits 0 with a test: a static
fixture payload refuses at 125 without the opt-in and
enters the memfd family with it.
`sh experiments/356-no-chroot-rung.sh` exits 0 with clause 18
changed: `go version` from the golang image refuses at 125
naming the static reason, and never exits 2 naming GOROOT.

## Correction

**Filed 2026-11-06.** Quoted from issue 91 and its comments, measured on
release 0.1.0-beta.12 on the errand host:

```text
$ podvm run golang:1.24-alpine go version
go: cannot find GOROOT directory: 'go' binary is trimmed and GOROOT is not set
[exit 2]
--verbose: podvm run: interpose: declined for "go": go carries Go build
markers, and Go issues syscalls directly rather than through libc, so the
tier cannot reach it
$ podvm run -e GOROOT=<store>/rootfs/<golang>/usr/local/go \
    golang:1.24-alpine /usr/local/go/bin/go version
go version go1.24.13 linux/amd64          # the host path works
$ podvm exec ... gb /usr/local/go/bin/go build -o /tmp/gohello main.go
$ ls -l /tmp/gohello                      # 2238818 bytes on the HOST
```

**Amended 2026-11-09.** For T-1648, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry serves the static tools of such a job, for example
`go`. It is not on the Node path. Its clauses do not change.

## Done

**Done 2026-11-10.** For T-1648, the last clause of the partial record below is run. `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 on the cloud host, verdict "NO-CHROOT FAMILY RUNS" ([the run](../experiments/results/no-chroot-rung.txt)): the images come from `mirror.gcr.io` by the switch of 2026-11-10, each pulled digest is in the conditions, and the capable host enters the namespace rung.

**Partial, 2026-11-09.** `decide_entry` takes the decided
`--unsafe-host-paths`. On a chroot-denied host a static payload refuses at
125 before the exec without it; the sentence names the static link, the
host paths, the host write and the routes (a host with chroot, the
opt-in, T-1651, T-1652). Under the opt-in, `userland_env` writes the
host-root line to stderr at once, so it prints under the quiet default
and `-q`. `--strict` still refuses the opt-in. Experiment 356 passes the
opt-in on its memfd clauses, and clause 18 (`fx-go-refuse`,
`356-no-chroot-rung.sh:574`) now asks for 125 naming the static reason
and no GOROOT. `cargo test -p podvm` exits 0 (274 passed) with
`a_static_payload_refuses_without_the_opt_in`. [A hand drive](../experiments/results/no-chroot-static-refusal.txt)
through the deny-chroot fixture of experiment 356 gave 125 for the static
hello and for `go version` (no GOROOT), 0 with the opt-in under the
quiet default, `-q` and `--verbose`, and 125 for `--strict
--unsafe-host-paths`. Remaining: one full run of `sh
experiments/356-no-chroot-rung.sh` (its pulls from `public.ecr.aws`
answered HTTP 429 on 2026-11-09). Known gaps, outside this entry: on
`run -d` and `create` the refusal names an opt-in that the detached
refusal of T-1411 then refuses, and `ensure_entry_possible` lets the pull
and the extract run before the static refusal.

# T-1651: An emulate tier on a prefix-preserving, fail-closed qemu-user for static payloads

**Source:** issue 94; issue 114 section 3.1 and section 4;
  `crates/podvm/src/tier.rs:26`;
  `crates/podvm-probe/src/report/document.rs:23`;
  `src/safety.rs` of `pod-extract` (PodUtils/podcrates); [T-0506](enter.md)
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

On a host that denies chroot, user namespaces, mount and
ptrace, a static payload has no rung that keeps it inside
the image. qemu-user with `-L` runs one process in the image
and leaks in three ways. A child it execs runs from the
host. A path absent from the rootfs is read from the host.
An absolute link in the image resolves on the host.

## Premise

`--podvm-tier` takes `machine` or `chroot` only
(`tier.rs:26`). The probe marks an emulator's answers in
`measured_by` (`report.rs:451`), the T-0506 rule. In qemu,
`do_execve` hands the raw guest path to the host, and
`util/path.c` `path()` returns the unprefixed host path
after an `access()` miss. `openat2(RESOLVE_IN_ROOT)` works
unprivileged on the errand host's kernel 7.2.2, and podvm
already wraps it in `podvm_extract::safety`
(`safety.rs:261`).

## Approach

Add an `emulate` tier. podvm invokes a qemu-user as its own
process, patched in three places. `do_execve` applies the
prefix and re-execs the emulator with the same `-L`.
`path()` fails closed with `ENOENT`. Path resolution, `stat`,
`access` and `readlink` included, goes through a root
descriptor with `RESOLVE_IN_ROOT`. The tier answers
`/proc/self/exe` with the guest path (T-1409's rule). The
banner names the tier, and the probe marks its answers as
the emulator's (T-0506).

## Decision

The emulate tier is a rung the caller selects or the ladder
names, never a hidden fallback. qemu stays a separate GPL
process, the boundary the machine tier's `qemu-system`
already keeps. podvm refuses an unpatched qemu, because its
fallback serves host files. For a native static payload
T-1652 is the first choice; this tier covers other
architectures and payloads whose exec or threads that loader
cannot follow. The cost is an estimate of about 2x native
(sandhome measured 0.421 s against 0.845 s on one payload).

## Prove

`cargo test -p podvm` exits 0 with tests for the tier
value and its banner. `sh experiments/356-no-chroot-rung.sh`
exits 0 with an emulate clause: `go version` from the golang
image and a shell pipeline exit 0 under the tier, a path
absent from the rootfs is refused and not read from the
host, and `podvm probe` under the tier reports
`measured_by.emulated` true.

## Correction

**Filed 2026-11-06.** Quoted from issue 94 and its comments, measured
with `qemu-x86_64` 11.1.1 (static-pie) on the errand host:

```text
$ qemu-x86_64 -L <alpine-rootfs> <alpine-rootfs>/bin/busybox cat /etc/os-release
NAME="Alpine Linux"                     # absolute paths are prefixed
$ qemu-x86_64 -L <golang-rootfs> <golang-rootfs>/usr/local/go/bin/go version
go version go1.24.13 linux/amd64        # a static payload runs
$ qemu-x86_64 -L <alpine-rootfs> <alpine-rootfs>/bin/busybox sh -c 'cat /etc/os-release'
cat: /etc/os-release: No such file or directory   # the child left the image
$ qemu-x86_64 -L <alpine-rootfs> <alpine-rootfs>/bin/busybox cat /workspace/hostonly.txt
LEAK-PROBE                              # a host file served on a miss
openat2(rootfd, "/bin/cat", RESOLVE_IN_ROOT)             -> fd 4, magic 7f454c46
openat2(rootfd, "../../etc/os-release", RESOLVE_IN_ROOT) -> fd 4, clamped
openat2(rootfd, "/no/such/file", RESOLVE_IN_ROOT)        -> -1 ENOENT
```

The effort of T-1651 was XL before call C17 of `docs/decisions.md`; pod-todo has no XL.

## Done

**Done 2026-11-11.** `cargo test -p podvm` exits 0 with the tests of the tier
value (`--podvm-tier=emulate`) and of the emulator on `PATH`, and
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 on the
cloud host ([the run](../experiments/results/no-chroot-rung.txt), verdict "NO-CHROOT FAMILY
RUNS", qemu-user 8.2.2 of Ubuntu): in clause 36 `go version` of the golang
image exits 0 under the tier with its banner, an arm64 alpine pipeline
(`uname -m; cat /etc/alpine-release | tr . -`) exits 0 and prints `aarch64`,
a file that only the host has is ENOENT and not read, and `podvm probe
--json` as the payload reports `measured_by.emulated` true. The paragraph
below has the design, which needs no patch of qemu.

**Partial, 2026-11-11.** The design changes, and the Decision holds: qemu
needs no patch. The emulator runs on the native-static rung (T-1652), so
each of its system calls comes to podvm's handler, which resolves each path
in the image with `openat2(RESOLVE_IN_ROOT)`; podvm passes no `-L`, so a miss
is ENOENT in the image, never the host's file (the second leak), and an
absolute link resolves in the image (the third). The loader of the rung puts
the emulator in front of an ELF that it serves (`-0 ARGV0 GUEST ARGS`), and
the `execve` of a guest re-executes podvm with the emulator
(`PODVM_DISPATCH_EMU`), so a child stays in the image (the first leak).
`--podvm-tier=emulate` (`crates/podvm/src/lifecycle/emulate.rs`) finds a
static `qemu-ARCH-static` or `qemu-ARCH` on `PATH`, as the machine tier finds
`qemu-system-*`, and serves a foreign image (its ELF machine) or an image of
this machine (each ELF); `exec` and `create` refuse the tier. The task's
name is the emulator's, so `podvm probe` under the tier reports
`measured_by.emulated` true. A defect found on the way: qemu's host signal
handler writes a full mask into the frame, so the wrapper of the rung
clears SIGSYS there after each handler (Go under qemu-x86_64 died on signal
31). `cargo test -p podvm` and `cargo test -p podvm-dispatch` exit 0 with
tests of the tier value, the emulator on `PATH` and the emulator's route
through an exec and a spawn. On the chroot-denied fixture: `go version` of
the golang image under qemu-x86_64-static, `uname -m; cat
/etc/alpine-release | tr . -` of an arm64 alpine under qemu-aarch64-static,
a host-only path gives ENOENT, and the probe reports `emulated: true`.
Remaining: the run of experiment 356 with clause 36.

# T-1652: A native-static rung: userland execve, Syscall User Dispatch and openat2(RESOLVE_IN_ROOT)

**Source:** issue 109; issue 114 section 1, section 4 and section 6
  item 1; `crates/podvm-probe/src/select.rs:35`;
  `pod_hostprobe::probes`;
  `src/memfd.rs` of `pod-enter` (PodUtils/podcrates);
  A-V-04, G-09 of PodUtils/podcrates#39
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

On the errand host the supervise rung is out of reach. A
tracer reads a path argument through `process_vm_readv`,
which is `EPERM`, or `/proc/<pid>/mem`, which is `EACCES`.
A static payload therefore has no native rung that keeps it
inside the image.

## Premise

Syscall User Dispatch (Linux 5.11 and later) delivers
`SIGSYS` to a handler in the payload's own address space.
The handler reads the path argument with a plain
dereference. Issue 109 measured the trap and an in-root open
on kernel 7.2.2 as uid 966. Dispatch does not survive
`fork`, `clone` or `execve`: a forked child did not trap. The
rung list (`select.rs:35`) has no such rung. The probe's
supervise leg measures `process_vm_readv` (`probes.rs:266`).
The memfd driver stages a payload (`memfd.rs:108`).

## Approach

Build the rung from four parts. A userland execve loader maps
the static payload into its own address space and jumps to
it. `PR_SET_SYSCALL_USER_DISPATCH` with
`PR_SYS_DISPATCH_EXCLUSIVE_ON` makes the loader's own code
the trusted region, so every payload syscall dispatches. The
handler serves path calls through `openat2(rootfd, path,
RESOLVE_IN_ROOT)`. The handler intercepts `clone` and `fork`
to arm dispatch in the child, and `execve` to load again.
Add a probe leg for the `prctl`, a rung word, and
`measured_by.user_dispatch`. Stage the bytes through T-1647.

## Decision

For a native static payload this rung is the first choice
over T-1651: native speed, no extra binary, no privilege. A
refused `prctl` or an older kernel falls back to the emulate
tier by name, never silently. Dynamic payloads stay on the
loader family.

## Prove

`cargo test -p podvm-enter` and `cargo test -p podvm-probe`
exit 0 with tests for the trap, the in-root open and the
re-arm in a child. `sh experiments/356-no-chroot-rung.sh`
exits 0 with a native-static clause: `go version` and a
`go build` of a hello program exit 0 from the golang image,
the probe reports `measured_by.user_dispatch` true, and with
the `prctl` refused the run names the emulate tier.

## Correction

**Filed 2026-11-06.** Quoted from issue 109, measured on the errand
host (kernel 7.2.2, uid 966, no chroot, user namespace, mount or ptrace):

```text
prctl(PR_SET_SYSCALL_USER_DISPATCH, PR_SYS_DISPATCH_EXCLUSIVE_ON, 0, 0, &selector) -> 0
traps=1 raw_getpid_via_sud=4242 real=16567
host has /etc/os-release: no
SUD-CHROOT traps=1 rewritten=1 fd=4 line=NAME="Alpine Linux"
selector: 0 = SYSCALL_DISPATCH_FILTER_ALLOW, 1 = BLOCK
```

**Amended 2026-11-09.** The asks A-V-04 and G-09 of PodUtils/podcrates#39
(COORD-015, a message from a peer session) touch this entry. A-V-04: this
rung serves the static tools of a Node CI job. The `node` binary goes
through the loader family, and T-1646 holds that end-to-end clause. G-09
wants a container engine inside a unit, for `services:` and Docker actions.
No rootfs tier can hold an engine: there is no namespace, no mount and no
bridge. Inside a unit the names `docker` and `podman` answer podvm itself
(T-0803), so an action that runs one container can work. A `services:`
network between containers cannot work on a rootfs tier, because the payload
shares the host network. A real engine runs only in a machine-tier guest,
where the guest kernel is root (T-1656). This entry changes no clause.

The effort of T-1652 was XL before call C17 of `docs/decisions.md`; pod-todo has no XL.

## Done

**Done 2026-11-11.** `cargo test -p podvm-dispatch` exits 0 with seven
tests (the trap and `/proc/self/exe`, the in-root open and `..`, an
absolute link, the re-arm in `fork`, a thread and `posix_spawn`, the exec,
a script); the tests are in `podvm-dispatch`, the crate of the engine, and
not in `podvm-enter` and `podvm-probe` as the Prove first wrote.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 on the
cloud host ([the run](../experiments/results/no-chroot-rung.txt), verdict
"NO-CHROOT FAMILY RUNS"): on the chroot-denied fixture the static hello
runs on the native-static rung with its banner; with the `prctl` denied
(`--deny-dispatch`) it refuses at 125 naming the static reason, the `prctl`
and the emulate tier; in the golang image `go version` exits 0, `go build`
of a hello program exits 0, the built program prints `g356-ok`, nothing
lands on the host; and `podvm probe --json` reports
`measured_by.user_dispatch` true. The paragraph below has the parts and the
four defects found on the way.

**Partial, 2026-11-11.** The rung is in the crate `podvm-dispatch`
(`crates/podvm-dispatch/src/`): a loader of a static x86_64 program
(ET_EXEC or static PIE, and a `#!` script of one) into podvm's own process,
a SIGSYS handler with podvm's code as the region of
`PR_SET_SYSCALL_USER_DISPATCH`, the path calls through
`openat2(RESOLVE_IN_ROOT)` with `/proc`, `/sys` and `/dev` the host's,
`/proc/self/exe` as the guest program, a clone trampoline that arms each
thread, `fork` and `vfork` children that arm again, a wrapper for each
caught signal, the program break, and `execve` as an exec of podvm itself.
`decide_entry` takes the rung for a static payload where chroot is denied
and `--unsafe-host-paths` is absent (`crates/podvm/src/lifecycle/native.rs`);
`podvm probe --json` reports `measured_by.user_dispatch`, and the fixture
`experiments/deny-chroot.c` takes `--deny-dispatch`. The tests are in
`podvm-dispatch`, not in `podvm-enter` (the engine is a crate of its own):
`cargo test -p podvm-dispatch` exits 0 with seven tests (the trap and
`/proc/self/exe`, the in-root open and `..`, an absolute link, the re-arm in
`fork`, a thread and `posix_spawn`, the exec, a script). On the cloud host,
on the chroot-denied fixture, `podvm run --rm GOLANG go version` exits 0,
and with `--deny-dispatch` the run refuses at 125 naming the `prctl` and the
emulate tier; `go build -a` of a hello program takes 4.3 s and its output
runs. Four defects were found and fixed on the way, each with its
measurement in the source: a vfork child on musl's 5 KiB stack, the mask and
the alternate stack that `rt_sigreturn` restores, and an alternate stack
that the kernel keeps while a handler runs on it. Remaining: the run of
experiment 356 with its new clauses (its first run failed only on the
`--rm` of the source step, which extracts the image again; fixed). T-1724
holds a static child of the loader family and `-v`.

# T-1653: `-v` volumes on a host with no /dev/fuse and no mount

**Source:** issue 130 (130 reposts the hidden issue 104); issue 114
  section 3.6; `crates/podvm/src/parity.rs:222`;
  `src/userland.rs` of `pod-enter` (PodUtils/podcrates);
  `crates/podvm-interpose/src/map/rewrite.rs:75`;
  A-V-06, G-07 of PodUtils/podcrates#39
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

`run -v` is refused on every rung. The errand host has no
`/dev/fuse` and denies `mount(2)`, so podvm can make neither
an overlay nor a bind mount. A payload has no way to see a
host directory at a guest path.

## Premise

The parity table refuses `-v, --volume` with status None:
"a volume would be a copy pretending to be a mount"
(`parity.rs:335`). The userland table is one pair, `/` to
the rootfs (`userland.rs:323`). The interposer takes the
longest matching prefix (`map.rs:325`), so a pair for a guest
directory wins over the root pair. T-0703 drove a mapped pair
end to end and left `-v` out of scope.

## Approach

On the loader family, turn each `-v HOST:GUEST` into a
`GUEST:HOST` pair in the `PODVM_MAPS` table beside the root
pair. Apply the T-1407 spelling checks to both sides. Refuse
`:ro` by name until podvm enforces it. The memfd family has
no interposer, so it keeps the refusal until T-1652 maps a
volume prefix to a host directory descriptor. The parity row
moves from None to Degraded and names the family that
serves it.

## Decision

A volume on this host is a path rewrite. It is not a copy and
not an overlay. A write lands in the host directory, as with
a docker bind mount. The two-root view of issue 114 copies
executables, which is the copy the parity row refuses, so
podvm does not use it. A payload execs from a volume only
where the host mount allows exec, and the banner names a
noexec source.

## Prove

`cargo test -p podvm-enter` and `cargo test -p podvm`
exit 0 with table tests for a volume pair and a refused
spelling. `sh experiments/356-no-chroot-rung.sh` exits 0 with
a volume clause on the fixture: a host file reads back at
the guest path, and a guest write lands in the host
directory. A `:ro` volume gives `EROFS` for a guest write and leaves the
host file unchanged, once the read-only layer of T-1655 exists.

## Correction

**Filed 2026-11-06.** Issue 130 rebuilds the hidden issue 104 from
issue 114, which states the case in one table row:

```text
overlay/volumes | no -v (issue #104) | no /dev/fuse, no mount |
two-root view (copy regular executables, symlink data back), not an overlay
references: containers/fuse-overlayfs (needs /dev/fuse), mpartel/bindfs,
trapexit/mergerfs; sandhome sh_promote_tree copies execs, links the rest
```

**Amended 2026-11-09.** The asks A-V-06 and G-07 of PodUtils/podcrates#39
(COORD-015, a message from a peer session) want `-v HOST:GUEST` through the
path map, `:ro` with `EROFS`, and caches kept apart for each trust level.
The path map is the Approach. One clause changes: `:ro` is refused by name
only until the read-only layer of T-1655 exists. After that, `:ro` maps to
it, and a guest write fails with `EROFS`. G-07 needs no cache store in
podvm. podvm keeps no mutable cache between runs. The runner picks one host
directory for each trust level. It mounts that directory `:rw` for a trusted
job and `:ro` for an untrusted job. Two jobs of different trust levels never
share a writable volume source, and the runner is the actor that enforces
this.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC section 10.1 and its WP-V6) says that podvm refuses a workdir
inside a volume. That is stale since podcrates v0.12.0 (5e5629a8): the
`-v` row says that a working directory or a first program in a volume
resolves through the table (`crates/podvm/src/parity.rs:222`), and `exec`
checks its workdir through the container's table
(`crates/podvm/src/exec/enter.rs:203`). One clause is new: the drive adds
`exec -w` into a volume of a `run -d` container. The remaining clause,
`:ro` with `EROFS`, is the one podvm gap of the runner's job-container
table (`/__e` read-only, T-1655, T-1711). A relative first program in a
volume workdir is T-1707.

## Done

**Done 2026-11-11.** The read-only layer of T-1655 exists (the record of
T-1655), and a `:ro` volume uses it: `-v HOST:GUEST:ro` puts `ro:GUEST` in
`PODVM_RO`, and the interposer answers `EROFS` for a write under GUEST.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0
([the run](../experiments/results/no-chroot-rung.txt)): clause 26 on the
chroot-denied fixture reads `vol-in` from a `:ro` volume, `touch` in it
prints "Read-only file system", and no file appears in the host
directory; the other volume clauses hold as before. `cargo test -p podvm`
exits 0 with `the_read_only_table_names_each_volume` and the parse of
`:ro` in `volumes_parse_in_each_spelling`.

**Still partial, 2026-11-10 (podcrates v0.12.0).** podvm takes the 11
crates at v0.12.0; one `pod_enter::userland::Maps` carries the root and
each volume through the workdir check, the entry, the `#!` follow and the
interposer. `-w` and a first program inside a volume now run.
`cargo test --workspace` exits 0 (649 passed), the interposer tests exit
0 (69 passed), and `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with the steps fx-vol-workdir, fx-vol-copy and fx-vol-prog
([the drive](../experiments/results/volumes-1653-v0120.txt)). Remaining:
`:ro` with EROFS (T-1655).

**Still partial, 2026-11-10.** `PODVM_356_MIRROR=1 sh
experiments/356-no-chroot-rung.sh` exits 0 with clause 26
([the run](../experiments/results/no-chroot-rung.txt)). Remaining: `:ro`
with EROFS (T-1655), and `-w` and a first program inside a volume, which
wait for the pod-enter of podcrates v0.12.0 (COORD-018, #224).

**Partial, 2026-11-10.** On the loader family each `-v HOST:GUEST[:rw]`
becomes a `GUEST:HOST` pair in `PODVM_MAPS` after the root pair
(`lifecycle/volume.rs`); the host side is canonical, so the reverse map
holds after a `cd` into the volume. The T-1407 spelling checks run on
both sides in podvm-enter's glue. Refused by name: `:ro` (T-1655), other
options, named volumes, the memfd family (T-1652), the chroot rung, the
machine tier, and `-w` or a first program inside a volume (pod-enter
resolves those inside the rootfs; COORD-018). A container record keeps
the table for `exec`. The banner names each volume and a noexec source.
The parity row is Degraded and names the loader family. `cargo test
--workspace` exits 0 (737 passed) and the interposer tests exit 0.
[The drive](../experiments/results/volumes-1653.txt): before, each `-v`
exited 125; after, a read, a write that lands in HOST, a relative write
after `cd /data`, grandchildren from the volume, and `run -d` then `exec`
work, and each refusal above exits 125. Clause 26 of experiment 356 holds
it. Remaining: `:ro` with EROFS (T-1655), `-w` and a first program inside
a volume (COORD-018), and the full experiment 356 run (`public.ecr.aws`
answered HTTP 429).

# T-1657: A syscall-interpreting tier: the gVisor capability without user namespaces

**Source:** issue 169; #193; the owner's ruling of 2026-11-06
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

gVisor's `runsc` answers a payload's syscalls in user space, so
the payload does not reach the host kernel for files. On the
target host `runsc` cannot start: it needs a user namespace,
mount and ptrace, and the host denies all three. podvm has no
tier that gives the same view: a fixed kernel identity, a
private `/proc`, a sized `/tmp` and a placed `/workspace` that
writes through.

## Premise

T-1652 builds the native-static rung: a userland execve loader
with Syscall User Dispatch, whose handler serves path calls
through `openat2(RESOLVE_IN_ROOT)`. T-1651 is the emulate tier
for foreign and other payloads. `--podvm-tier` takes `machine`
or `chroot` only (`crates/podvm/src/tier.rs:22`). The rung list
is `crates/podvm-probe/src/select.rs:35`. The issue names the
podcrates crate `pod-sud` (wave M7).

## Approach

On top of the T-1652 handler, answer `uname` with a fixed
string that names the tier. Serve a private `/proc` view for
the payload's own entries. Serve `/tmp` from a store directory
with a size cap. Place `/workspace` as a path-map entry whose
writes reach the host directory, and plant the F14 trap: a
write that lands in a private overlay fails the test. Resolve
`..`, absolute links and a missing file in the root or fail
closed. Where a host grants user namespaces, offer `runsc` as
an optional backend from the namespace rung.

## Decision

An in-process handler shares the payload's address space. It
is a compatibility layer, not a boundary, and the banner says
so, as it does for the interposer. podvm does not copy gVisor's
network stack. The optional `runsc` backend is not the answer
for the target host. Build on `pod-sud` once podcrates
releases it.

## Prove

`cargo test -p podvm-enter` exits 0 with tests for the `uname`
answer, the `/tmp` cap and fail-closed resolution of `..`, an
absolute link and a missing file.
`sh experiments/413-syscall-tier.sh` exits 0 on a
chroot-denied fixture: a static payload reads `/etc/os-release`
from the image with no ptrace and no qemu, its write to
`/workspace` is on the host after the run, `uname -r` names the
tier, and the banner says the layer is not a boundary.

## Correction

**Filed 2026-11-06.** Quoted from issue 169:

```text
runsc --root RUN/runsc-root --rootless --network=none|host --ignore-cgroups
      --platform=systrap run -bundle RUN/bundle sand-ID
inside: uname -r is 4.19.0-gvisor; /usr is not writable; /workspace writes
      through to the host
F14: `runsc do -volume SRC:/workspace` wrote into gVisor's in-memory overlay
      and never reached the host; a bundle with a bind mount did
F14: runsc refuses rootless create ("Rootless mode not supported with
      "create""), so an image under gVisor needed rootful podman
--ignore-cgroups is required with no cgroup delegation
```

## Done

**Done 2026-11-11.** `cargo test -p podvm-dispatch` exits 0 with
`the_syscall_tier_answers_uname_with_its_release`,
`the_syscall_tier_caps_tmp_across_a_fork` (a cap of 1000 bytes: 800 bytes
fit, 400 more from a child fail with ENOSPC, an unlink gives the space
back), `a_missing_file_of_the_root_is_enoent_and_not_the_hosts`, the unit
test `the_tier_counts_growth_and_names_its_release`, and the resolution
of `..` and an absolute link of T-1652.
`PODVM_413_MIRROR=1 sh experiments/413-syscall-tier.sh` exits 0 on the
chroot-denying fixture (`experiments/results/syscall-tier.txt`):

| clause | reading |
| --- | --- |
| 1 | the static busybox of `busybox:musl` reads `ID=podvm413` from the image's `/etc/os-release` |
| 2 | `TracerPid: 0`, no qemu process, the banner names the syscall tier |
| 3 | the write to `/workspace` is on the host after the run |
| 4 | `uname -r` prints `6.1.0-podvm-syscall` |
| 5 | the banner: "The handler shares the payload's address space: it is a compatibility layer, not a boundary" |
| 6 | `--tmpfs /tmp:size=1m`: `dd` stops with "No space left on device" at 1048576 bytes, and no directory of `/tmp` stays |
| 7 | a dynamic program and `run -d` refuse by name (exit 125) |

Decisions and deviations:

* The tier is built in `podvm-dispatch` (`tier.rs`), as T-1652 built the
  rung: podcrates has no `pod-sud`, and the handler is podvm's. The tests
  are in `podvm-dispatch`, where the handler is, not in `podvm-enter`.
* `/tmp` is a directory of the run (the `--tmpfs` of T-1655). The handler
  counts the growth of each file that a descriptor opened under `/tmp`
  writes, in a shared page of a memfd, so the count holds across `fork`
  and `execve`. `docs/limits.md` names what the count does not see.
* The private `/proc` view is the links of `/proc/self` in guest paths
  (T-1652). The entries of other processes stay the host's.
* The tier runs a static x86_64 program; a dynamic one refuses and names
  the emulate tier. A detached run refuses, because `/tmp` belongs to a
  foreground run.
* The optional `runsc` backend of the Approach is T-1729: the Decision
  says it is not the answer for the target host.
* A dynamic program on the rung is T-1731, and the `--strict` gap of the
  loader family names it (the gap named T-1657 until this entry was done,
  and CI run 387 failed on `each_listed_gap_names_an_entry_that_is_not_done`).

# T-1658: A confine layer: Landlock, seccomp and no_new_privs on every rootfs tier

**Source:** issue 170; #193; the owner's ruling of 2026-11-06;
  A-V-01 of PodUtils/podcrates#39
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

bwrap and bailey confine a host process with user and mount
namespaces, which the target host denies. Landlock needs no
privilege, but podvm applies no Landlock ruleset, no seccomp
deny filter and no built environment to a payload. A profile
cannot say "read these paths, write these, connect to port 443
only".

## Premise

The probe measures `landlock_create_ruleset(VERSION)`
(`pod_hostprobe::probes`, body at
`probes.rs:1331`) and sets `PR_SET_NO_NEW_PRIVS` in a child
(`probes.rs:1287`). The Landlock and seccomp sweeps are T-0903
and T-0902. Landlock domains stack, so a payload can add a
ruleset over the host's, but whether the target host's outer
policy allows a nested ruleset is not measured. The issue
names the podcrates crate `pod-confine` (wave M7).

## Approach

Before the exec, set `PR_SET_NO_NEW_PRIVS`, add a Landlock
ruleset from the profile's read, write and execute lists and
TCP ports (ABI 4 or later for the network rules), load a
seccomp deny filter for namespace, mount, ptrace, keyctl, bpf
and similar calls, build the environment, and set rlimits.
Report the Landlock ABI and the nested-ruleset answer in
`podvm probe`. Place `/workspace` through the path map, where
landrun could not (F30). Ship the bailey policy of issue 170
as a built-in `errand` profile.

## Decision

Where Landlock is absent, a profile that asks for it refuses by
name; it never runs unconfined. The default-route trap (F2)
and the trust-parent trap (F3) are causes to probe and name
before the start. Build on `pod-confine` once podcrates
releases it.

## Prove

`cargo test -p podvm-enter` and `cargo test -p podvm-probe`
exit 0 with tests for the ruleset builder and the probe's ABI
answer. `sh experiments/414-confine.sh` exits 0 on a host that
grants Landlock: a profile with `fs.read` and `net.ports =
[443]` makes `cat /etc/hostname` fail with `EACCES`, a connect
to port 22 fail with `EACCES` and port 443 pass;
`/proc/self/status` shows `NoNewPrivs 1` and `Seccomp 2`; with
Landlock absent the profile refuses by name.

## Correction

**Filed 2026-11-06.** Quoted from issue 170:

```text
landrun: --best-effort, --rox system paths, --ro /etc files, --rwx project,
      --rw state, --connect-tcp PORT per net.ports
F1: bailey v0.2.4 musl cannot start where mounts are relatime in a user
      namespace (MS_STRICTATIME, EPERM): nix 0.29 drops ST_RELATIME on musl
F2: bailey's brokered modes refuse without an IPv4 default route
F3: bailey refuses a policy under a group-writable parent with no sticky bit
F4: /usr/bin/cc points into /etc/alternatives, which the read list omitted
F30: landrun: `ls /` and /etc/hostname give EACCES; /tmp is not writable;
      no TCP unless net.ports
L4: bailey's port rule refuses bind() to a source port outside net.ports
reference compare: no capabilities, NoNewPrivs 1, seccomp mode 2, every
      namespace and mount call refused; 382 same, 9 differ
```

**Amended 2026-11-09.** The ask A-V-01 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants one profile to build the confine layer:
Landlock, a seccomp deny filter, `PR_SET_NO_NEW_PRIVS` and rlimits. The
Approach holds each part. Two clauses are new. The rlimit part reads the
same profile as T-1655, so the limits have one source. A refusal for a
missing Landlock names the missing facility: Landlock itself, or the ABI
level that a network rule needs. The library is `pod-confine` (podcrates
podcrates@T-108). The peer asks for P0. The priority stays P2, because nothing in
podvm is wrong without the layer (the priority rules of `TODO/INDEX.md`).
The owner can change it.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC sections 4.2, 6.5, 10.1 and its question DR-4) puts a trusted
job only on a unit whose boundary is at least `confined`, unless the owner
accepts `redirect`; no podvm rung is `confined` today. This layer is the
route by which a podvm rung can be `confined` on a host with no namespaces
(ask 2.7 of COORD-021, issue 227, names `pod-confine`, podcrates podcrates@T-108).
Two clauses are new. First, the report of T-1695 and `podvm probe --json`
state whether the layer was applied, with the Landlock ABI, the seccomp
mode and `NoNewPrivs`, so that a caller reads it and does not infer it.
Second, the experiment adds a canary file outside `fs.read` that the
payload cannot read, a host loopback TCP listener that it cannot reach
(Landlock ABI 4 or later), and records whether a host Unix socket is
reachable.

**Blocked 2026-11-11 on podcrates.** The confine step must run in the
payload's child after the root change, the `chdir`, the rlimits and
`no_new_privs`, and before the exec. That child is `pod-enter`'s, which
podvm takes frozen, and its `Plan` has no step for a ruleset or a filter.
COORD-040 (issue 259) asks podcrates for `pod-confine` and for
`Plan.confine`, as the Decision says. Measured here: the cloud host
(Linux 6.18) answers Landlock ABI 7. The condition that clears the block:
a podcrates tag with `pod-confine` and `Plan.confine`.

**Unblocked 2026-11-11.** podcrates `v0.23.0` has `pod-confine` and
`Plan.confine` (the answer to COORD-040, PodUtils/podcrates#69), and podvm
takes `v0.24.0` (T-1679). Each `Plan` of podvm names `confine: None` until
this entry sets it from the profile.

## Done

**Done 2026-11-12.** podvm takes podcrates `v0.27.0`: its podcrates@T-227 opens
the path of each rule of `Plan.confine` in the child after the mounts, so
a rule on `/tmp` covers the tmpfs of the namespace rung (COORD-048, issue
267). `sh experiments/414-confine.sh` exits 0 on the namespace rung with
each clause; clause 6 (the profile `errand`) writes `/tmp` (`tmp=0`)
and is denied `/etc/shadow`. With the dispatcher families of the same
day (clause 4), the layer runs on each rung with a chroot and on each
family of the userland rung.

**Partial 2026-11-11.** `run --podvm-confine=SPEC` applies the layer of
`pod-confine` through `Plan.confine` of `pod-enter`: SPEC is
`read=P,...;write=P,...;exec=P,...;connect=PORT,...` with guest paths
(`crates/podvm-enter/src/confine.rs`). The spec adds read and execute of
the system directories of the image that exist (`/bin`, `/sbin`, `/usr`,
`/lib`, `/lib64`, `/lib32`, podvm's `/.podvm`), read of `/proc` and three
device nodes, and read and write of `/dev/null`; `/etc` is not among them.
A profile takes `confine = true` with `fs.read`, `fs.write` and
`net.ports`. The layer runs on a rung with a chroot (namespace, chroot),
for a foreground run, and is decided after the fixups, so a node that the
completion layer made gets its rule. Each other case refuses by name with
exit 125 before anything runs: a rung with no chroot, `run -d`, a kernel
with no Landlock, an ABI under 3, and `connect` under ABI 4
(`crates/podvm/src/lifecycle/confine.rs`, the test
`each_case_with_no_mechanism_refuses_by_name`). The probe of
`probe-inside` reads no_new_privs and the seccomp mode with prctl(2), so
they hold where no `/proc` is mounted (`crates/podvm-probe/src/inside/checks.rs`).

`sh experiments/414-confine.sh` exits 0 on the cloud host (Linux 6.18,
Landlock ABI 7, the namespace rung): `/etc/passwd` reads, `/etc/shadow`
and a write to `/tmp` fail with "Permission denied", a TCP connect to the
allowed host listener answers and the other fails; the control without
the layer reads `/etc/shadow` and reaches both ports; `probe-inside` reads
NoNewPrivs 1 and Seccomp 2 under the layer and 0 and 0 without it; the
userland rung and `run -d` refuse by name. `cargo test -p podvm-enter` and
`cargo test -p podvm` exit 0 with the tests of the spec, the profile key
and the refusals. Two readings of the Prove changed: busybox:musl has no
`/etc/hostname`, so the outside read is `/etc/shadow`; the network of the
cloud host is a proxy only, so the TCP clause uses two listeners of the
host loopback in place of ports 22 and 443.

podvm takes podcrates `v0.25.0` (COORD-051, PodUtils/podcrates#76): the
layer now closes the Landlock scopes of ABI 6, denies MPTCP under a TCP
list and io_uring, and `apply` drops each capability outside docker's
default set (`Spec::new()`), so a confined root keeps no `CAP_SYS_ADMIN`
with which it read the environment of its unconfined ancestors past the
domain. Experiment 414 exits 0 again at that tag; `cargo test --workspace`
exits 0.

`run -d` and `exec` take the layer too (later the same day): the record
keeps the SPEC (`Hardening.confine` of
`crates/podvm-supervise/src/table.rs`), the launcher of `run -d` builds the
spec against the container's tree, and `exec` applies it on a rung with a
chroot and refuses it by name elsewhere. Experiment 414 clause 5 (exit 0):
the detached payload and an `exec` into it both fail to read
`/etc/shadow`, and the `exec` reads `/etc/passwd`.

The built-in profile `errand` carries bailey's policy of issue 170:
`confine = true`, bailey's read list (the ld.so files, the CA stores,
`nsswitch.conf`, `services`, `protocols`, `/etc/alternatives` of F4,
`passwd`, `group`) and write of `/tmp`, `/dev/full` and `/dev/tty`. Each is
written `?/P`, a new form of the SPEC: an optional path gets no rule where
the image lacks it, so one profile serves many images. Experiment 414
clause 6 (exit 0): NoNewPrivs 1, Seccomp 2, `unshare-user`,
`mount-tmpfs`, `bpf` and `keyctl` refused, `/` unlistable, `/etc/shadow`
denied, `/etc/passwd` read, the verdicts of bailey's reference report.
Found: on the namespace rung the payload cannot write `/tmp`, because the
child mounts a tmpfs there after `pod-enter` opens the rules; COORD-048
(issue 267) asks podcrates to open the rules in the child.

The loader family of the userland rung takes the layer too: `pod-enter`
has no chroot there, so each guest path of the spec becomes its host path
through the table of the interposer (the rootfs, and the volume of a `-v`;
`host_spec` of `crates/podvm-enter/src/confine.rs`), and the host's
`/proc` and device nodes join it. The launcher of `run -d` builds the same
host spec from the record's table. Experiment 414 clause 4 (exit 0, the
chroot-denying fixture): alpine on the loader family reads `/etc/passwd`,
is denied `/etc/shadow` and `/tmp`, and reads and writes a `-v` volume at
`/workspace`; by hand, `run -d` and an `exec` into it hold the same.

The dispatcher families take the layer too. On the native-static rung and
the syscall tier the payload's process runs podvm's own dispatcher, and on
the emulate tier a host qemu: the host spec gives each of those host paths
read and execute (`decide` of `crates/podvm/src/lifecycle/confine.rs`).
Experiment 414 clause 4 (the chroot-denying fixture): the static busybox
on the native-static rung and on the syscall tier, and the arm64 busybox
on the emulate tier, read `/etc/passwd` and are denied `/etc/shadow`.

The report of
T-1695 (`run --report`) names the confine SPEC of the run. The ABI is in `podvm probe` already (the
probe `landlock_create_ruleset(VERSION)`), and `podvm doctor` names what
the layer does with it: under ABI 3 it is MISSING, under ABI 4 it runs
with no TCP rule (`crates/podvm/src/doctor/scope.rs`).


# T-1688: The memfd rung measures whether a memfd executes, names `vm.memfd_noexec`, and refuses before staging

**Source:** issue 212; T-1667 (the doctor row shape); T-1003 (the fd exec)
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

`kernel_takes_memfd()` proves that `memfd_create` answers, not
that the descriptor executes. Under `vm.memfd_noexec=1` the
ladder promises the memfd rung and the launch fails at step 5,
"execveat of the memfd". Under `vm.memfd_noexec=2` the entry
refuses with a text that does not name the sysctl. `podvm
doctor` has no row for it.

## Premise

The probe is `src/memfd.rs` of `pod-enter` (PodUtils/podcrates): it creates
and closes a memfd and never executes one. The callers are
`crates/podvm/src/ladder.rs:202` and
`crates/podvm/src/lifecycle/entry.rs:356`. The late step is
`src/launch.rs` of `pod-enter` (PodUtils/podcrates). The census row
`memfd_create+exec` of `pod_hostprobe::probes` runs a shell
script through `/proc/self/fd/N`, not `execveat` of a static
payload, and podvm does not read it. Under value 2, `MFD_EXEC`
is denied too, so "pass `MFD_EXEC`" (the issue) fixes value 1
only.

## Approach

Add one probe in `podvm-enter`: create a memfd, write a small
static trampoline, fork, `execveat` it, read the exit status.
It returns `Exec`, `NoExec` (with the sysctl value when
`/proc/sys/vm/memfd_noexec` is readable) or `NoMemfd`. The
doctor row `memfd_exec`, the ladder and the entry check read
this one probe. The refusal comes before staging and names the
sysctl, with a fix line for each value: 1 allows `MFD_EXEC`; 2
needs the sysctl set to 0 or another rung.

## Decision

The doctor row and the refusal read one probe, so they cannot
disagree. A denial with no readable sysctl reads as "denied,
cause unknown", not as the sysctl.

## Prove

`cargo test -p podvm-enter -p podvm` exits 0 with tests for the
pure mapping (measured result and sysctl value to the answer
and the fix line), and `podvm doctor` prints the row
`memfd_exec` on the cloud host.

## Correction

**Filed 2026-11-09.** From issue 212 (the owner). The triage of
2026-11-09 confirmed each claim at `2be31b2` and corrected two: the census
row exists in `pod-hostprobe` but tests another exec path, and `MFD_EXEC`
does not help under value 2.

## Done

**Done 2026-11-09.** `memfd::probe::memfd_exec` stages a static
132-byte trampoline (x86_64 and aarch64) the way the launch stages a
payload, executes it through `execveat(AT_EMPTY_PATH)` in a forked child,
and answers `Exec`, `NoExec`, `NoMemfd` or `Unmeasured`, each with the
value of `vm.memfd_noexec`. The ladder, the entry check and the new doctor
row `memfd_exec` read this one probe, and `kernel_takes_memfd` is gone.
`memfd_create` passes `MFD_EXEC`, so value 1 no longer blocks the rung;
under value 2 the refusal comes before staging and names the sysctl. The
pure function `probe::answer` gives the text and the fix line. `cargo
test -p podvm-enter -p podvm` exits 0 (404 passed), with
`the_mapping_names_the_sysctl_only_where_it_is_the_cause` and
`a_memfd_sealed_against_exec_reads_as_no_exec`. [The drive](../experiments/results/memfd-exec.txt):
`podvm doctor` prints `ok memfd_exec` at value 0 and 1, and the note with
the fix at value 2, each value set in a child pid namespace. Limits: the
forced-memfd refusal was not driven end to end (no static image in the
store), and the aarch64 trampoline was assembled and compiled, not run.

# T-1700: A run on the userland rung sometimes exits 0 with no output

**Source:** T-1653 (its drive of 2026-11-10)
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

On the userland rung (chroot denied), `podvm run --rm alpine
cat /etc/alpine-release` sometimes exits 0 and prints
nothing. It lost its output in 1 of 20 runs on `e99fc9b`
and in 2 of 15 runs after T-1653. The chroot rung (0 of 20)
and the deny-chroot fixture alone (0 of 60) never lost it.
A run that loses its output and exits 0 is a wrong answer
with no error.

## Premise

The defect is older than T-1653. Experiment 356 saw the same
shape once before (`fx-gc-script` exited 0 with no output,
recorded with T-1650). The cause is not measured.

## Approach

Reproduce in a loop on the fixture, then trace the payload's
stdout from the loader family's launch to the caller: the
descriptor podvm gives the payload, a close or a flush before
exit, and the exit path of the launcher. Fix the cause, not
the symptom.

## Decision

A lost output is a defect even when the exit code is 0.

## Prove

A loop of 200 runs of `cat /etc/alpine-release` on the
chroot-denying fixture prints the release each time, and
`cargo test --workspace` exits 0 with a test of the cause.

## Correction

**Filed 2026-11-10.** Found by the drive of T-1653.

## Done

**Done 2026-11-10.** The cause is podvm's own `untee`
(`crates/podvm/src/run/foreground.rs`), and it touches every foreground
run on every rung, not only the userland rung. `untee` closed the saved
stdout before it joined the pump thread. When the payload wrote and exited
just before the restore, the pump wrote the bytes after the close: the
write to stdout failed with EBADF, the bytes went to the container log
only, and podvm exited 0. A trace of the losing runs (strace
`--seccomp-bpf`, write and sendfile only) shows the payload's `sendfile`,
its exit, then `write(8, ...) = -1 EBADF` and `write(9, ...) = 8`. The fix
restores fd 1, joins the pump, then closes the saved descriptor. The test
`untee_delivers_every_byte_before_the_saved_descriptor_closes` fails 20 of
20 times with the old order and passes 20 of 20 with the fix; `cargo test
--workspace` exits 0. [The loop](../experiments/results/lost-output-1700.txt)
lost 1 to 12 runs in 150 before (the userland rung with stdout a file, a
pipe or a tty, the memfd family, the namespace rung) and none after (200
of 200 on the fixture).

# T-1707: A relative first program resolves against the image root, not against `-w`

**Source:** issue 232; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), its measurement M2
  (`03-runner-payload.log`); `src/ladder.rs` of `pod-enter`
  (PodUtils/podcrates); `src/userland.rs` of `pod-enter`
  (PodUtils/podcrates); T-1653 (the table);
  `crates/podvm/src/parity.rs:211`
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`podvm run -w /opt/runner IMAGE ./bin/Runner.Listener` exits
127 with "./bin/Runner.Listener names no file in the image".
docker runs `/opt/runner/bin/Runner.Listener`: the kernel
resolves a relative path against the working directory. An
image with a `WORKDIR` and a relative command fails the same
way. The runner starts as `./run.sh` from its root.

## Premise

`resolve_payload` in `src/ladder.rs` of `pod-enter` v0.12.0
(line 147) resolves an argv0 that holds a `/` from the image
root. `src/userland.rs` of `pod-enter` v0.12.0 (line 128) has
the same shape. The `-w` row says "chdir inside the new root"
(`crates/podvm/src/parity.rs:211`), so the working directory
exists before the exec. M2 observed exit 127, 3 of 3.

## Approach

Join a relative argv0 that holds a `/` to the working
directory before the resolution, on each rung and through
the T-1653 table, so that a workdir in a volume works too.
Send the change to podcrates as a COORD issue (`AGENTS.md`,
section 9, item 4), take the release, and add the podvm test.

## Decision

docker's rule: a relative program is relative to the working
directory. A bare name still searches `PATH`.

## Prove

`cargo test -p podvm` exits 0 with a test that `-w /opt/x
./prog` resolves to `/opt/x/prog`.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with a new clause: `-w /bin ./busybox true` exits 0
on the userland rung, and a relative program in a volume
workdir runs.

## Correction

**Filed 2026-11-10.** From issue 232 and the podrunner design review
(the ruling "podrunner", 2026-11-10), M2 surprise 9: "the first program
of `run` is resolved against the image root, not `-w`". docker's `-w`
with a relative program does not work, so the entry is P1.

## Done

**Done 2026-11-11.** podvm needs no change of `pod-enter`: each lookup of
the first program is a call of podvm. `podvm_enter::program_in`
(`crates/podvm-enter/src/lib.rs`) joins a relative program that holds a
`/` to the working directory; a bare name still searches `PATH`, and the
payload's `argv[0]` keeps the caller's spelling. The joined name goes to
`decide_entry` and `script::follow` (resolution, head, `invocation_path`,
`interpose::classify`), to `interpose::apply` (the preload), to
`push_guest_exe` (`/proc/self/exe`), to the memfd stage of a userland run,
and to a forced ladder rung, in `run` and in both paths of `exec`. The
chroot and namespace rungs exec after `chdir`, as the kernel resolves it.
A first drive found that `interpose::apply` still took the raw `argv[0]`:
the payload ran with no preload, and its children ran the host's
binaries; the clause of experiment 356 now runs a child that sees the
image only through the interposer, not `./busybox true`.

Proof: `cargo test -p podvm` exits 0 with
`a_relative_program_resolves_against_the_working_directory`.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 with
clause 31 ([the run](../experiments/results/no-chroot-rung.txt)):
`-w /bin ./busybox sh -c 'ls /etc/alpine-release'` lists the image's
file, and `-w /data ./tool356.sh` runs a script of a volume workdir.
Experiment 428 runs `-w /opt/runner ./bin/Runner.Listener --version`.

# T-1716: Take the `pod-enter` change of podcrates podcrates@T-155: `Maps::new` takes `&Names` and names the volume of a missing `-w`

**Source:** PodUtils/podcrates#52 (COORD-027, the answer to issue 226);
  podcrates podcrates@T-155 (commit d8b77b5, no tag on 2026-11-10);
  PodUtils/podcrates#53 (COORD-028: `v0.15.0` carries it);
  `crates/podvm-enter/src/lib.rs:26`,
  `crates/podvm-enter/src/lib.rs:172`,
  `crates/podvm-enter/src/lib.rs:177`,
  `crates/podvm-enter/src/lib.rs:188`
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

podvm keeps its own volume rules in `podvm-enter`
(`volume_maps`, `MAX_VOLUMES`, `NEVER_MAPPED`), while
`pod-enter` now refuses a reserved guest path and an
oversize volume count itself, and names the volume of a
missing `-w` directory. Two copies of one rule can drift.

## Premise

COORD-027 gives the change: `Maps::new(&names, rootfs,
&volumes)` takes `&Names` first; `Names::reserved` is new;
`GuestRoot::volume_label` is new; `check_workdir` names the
volume `HOST:GUEST` of a missing `-w`. podvm uses podcrates
`v0.12.0` (5e5629a); `v0.14.0` is the last tag on
2026-11-10 and needs no podvm change (podcrates#50).

## Approach

At the podcrates tag that carries d8b77b5 (planned
`v0.15.0`): set `NAMES.reserved` to `["/proc", "/.podvm"]`
and `NAMES.product` to `podvm`, call `Maps::new(&NAMES,
rootfs, &volumes)`, drop `volume_maps`, `MAX_VOLUMES`,
`NEVER_MAPPED` and `under`, and keep `volumes_of`. Follow
the adoption map of the `pod-enter` README. Move each pin of
the 11 crates to that tag in one commit.

## Decision

The volume rules live in `pod-enter` only; podvm fills the
names.

## Prove

`cargo test --workspace` exits 0 with a test that a `-v`
pair to `/proc` is refused by name and that a missing `-w`
inside a volume names the volume; `rg -n "NEVER_MAPPED|
MAX_VOLUMES" crates/` prints nothing.

## Correction

**Blocked 2026-11-10.** Blocker: no podcrates tag carries d8b77b5.
Clears when: a podcrates tag (planned `v0.15.0`) carries podcrates podcrates@T-155.

**Unblocked 2026-11-11.** podcrates `v0.15.0` (`27a4ccd`) carries podcrates@T-155
(PodUtils/podcrates#54, COORD-029).

**Filed 2026-11-10.** From COORD-027 (PodUtils/podcrates#52), the answer
of podcrates to the two asks of issue 226 (T-1653).

## Done

**Done 2026-11-11.** podvm takes the 11 crates of PodUtils/podcrates at
`v0.15.0` (`27a4ccd`). Each pin of `crates/*/Cargo.toml` moves from
`v0.12.0` in one commit, and `Cargo.lock` changes only the version and
the git revision of each `pod-*` package (no new package). `NAMES.reserved` is `["/proc", "/.podvm"]`;
`NAMES.product` was `podvm` before. Each caller builds the table with
`Maps::new(&NAMES, rootfs, &volumes)` (`run/parse.rs`,
`lifecycle/volume.rs`, the launcher of `podvm-supervise`, and the tests),
and `volume_maps`, `MAX_VOLUMES`, `NEVER_MAPPED` and `under` are gone;
`volumes_of` stays. podvm had no volume rule for `-w` left in its own code
since T-1653 (the take of `v0.12.0`), so `check_workdir` alone names the
volume now. The Prove, on the cloud host, 2026-11-11: `cargo test
--workspace` exits 0 (650 tests in 53 suites, 0 failed), with
`a_volume_the_table_cannot_spell_is_refused` (a `-v` pair to
`/proc/self` and to `/.podvm` is refused by name: "podvm never rewrites a
path under /proc") and `a_guest_path_in_a_volume_resolves_in_the_volume`
(a missing `-w /data/none` inside a volume says "in the volume
HOST:/data is not a directory in the volume"; a missing `-w /none` still
names the image). `rg -n "NEVER_MAPPED|MAX_VOLUMES" crates/` prints
nothing (exit 1). `cargo clippy --workspace --all-targets -- -D
warnings` and `cargo fmt --all --check` exit 0.

# T-1719: The loader's library directories leave out `/usr/local/lib` and the image's `ld.so.conf`

**Source:** the work of T-1705, 2026-11-11; `src/userland.rs` of
  `pod-enter` (PodUtils/podcrates), `lib_dirs`
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

On the userland rung podvm starts the image's loader with
`--library-path` and the standard library directories of the
image. A library that the image keeps in `/usr/local/lib`, or
in a directory that only its `/etc/ld.so.conf` names, is not
found by that path. The loader runs on the host, so it reads
the host's `/etc/ld.so.cache`, not the image's (Inferred, not
run).

## Premise

`lib_dirs` of `pod-enter` v0.15.0 pushes the loader's own
directory, then `/lib`, `/lib64`, `/usr/lib`, `/usr/lib64` and
the multiarch directories of five machines; it reads no
`ld.so.conf` and no `/usr/local/lib`. An image that builds a
program from source with `make install` and `ldconfig` keeps
its libraries in `/usr/local/lib`. A program with an
`$ORIGIN` run path is not affected.

## Approach

Measure first: an image with a library in `/usr/local/lib`
and an `ld.so.conf.d` entry for it, and a program with no run
path that links it, on the chroot-denied fixture of
experiment 356. If it fails, ask podcrates by a COORD issue to
add `/usr/local/lib`, `/usr/local/lib64` and each directory of
the image's `/etc/ld.so.conf` (with its `include` lines, read
through the table, bounded) to `lib_dirs`, and take the tag.

## Decision

The loader searches the directories that the image's own
`ldconfig` would search; the host's cache plays no part.

## Prove

`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with a clause: the program of that image runs at 0.

## Correction

**Filed 2026-11-11.** Found during T-1705, which put its test library in
`/usr/lib/x86_64-linux-gnu` because `lib_dirs` has no `/usr/local/lib`.

**Measured 2026-11-11; blocked on podcrates.** An image of alpine 3.20
with `libt1719.so` in `/usr/local/lib`, and a dynamic musl program that
needs it with no run path (built with zig cc): on the chroot-denied
fixture of experiment 356 the run exits 127 with "Error loading shared
library libt1719.so: No such file or directory"; on the namespace rung
the same image prints `lib-42` at 0. With a file of that name in the
host's `/usr/local/lib` (by a mistake of the drive, removed after it),
the userland rung printed `lib-42`: the guest loader took the host's
library. Thus the premise holds, and a miss also reaches the host's
directories. COORD-029 (issue 247) asks podcrates for `/usr/local/lib`,
the image's `ld.so.conf` and `ld-musl-ARCH.path`, and `--inhibit-cache`
for glibc. The blocker clears when a podcrates tag carries it; then podvm
takes the tag and adds the clause of the Prove.

## Done

**Done 2026-11-11.** podvm takes the 11 crates of podcrates at `v0.19.0`
(`9a11bbd`, PodUtils/podcrates#62), which carries its podcrates@T-182 for COORD-029
(issue 247). `lib_dirs` of `pod-enter` now gives `/usr/local/lib`,
`/usr/local/lib64` and their multiarch forms, then each directory of the
image's `/etc/ld.so.conf` (with `include`) or `/etc/ld-musl-ARCH.path`,
each resolved through the table; `LoaderPlan::inhibit_cache` is true for
a glibc loader of 2.16 or later. podvm gives that loader
`--inhibit-cache` (`crates/podvm/src/lifecycle/entry.rs`), and the
interposer gives it to the loader of each glibc child
(`loader_takes_inhibit_cache` in
`crates/podvm-interpose/src/exec/head.rs`), so the host's `ld.so.cache`
plays no part. A library that no directory of the image holds is still
searched in the loader's built-in host directories.

Proof: `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0
with clause 32 ([the run](../experiments/results/no-chroot-rung.txt)): on
the fixture, a musl program of alpine with its library in
`/usr/local/lib`, and a glibc program of debian with its library in a
directory that only `/etc/ld.so.conf.d` names, each print `lib-42`. The
interposer tests pass (83), with
`a_glibc_loader_from_2_16_takes_inhibit_cache`.

# T-1724: A static child of the loader family, and a volume, on the native-static rung

**Source:** T-1652 (measured on the cloud host, 2026-11-11);
  `crates/podvm-interpose/src/exec/launch.rs` (`Tail::Direct`);
  `crates/podvm/src/lifecycle/native.rs`
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

A dynamic shell of the golang image on the loader family ran
`go build` as its child: the interposer executes a static
child directly (`Tail::Direct`), so go ran with the host's
root and wrote `/tmp/g/g` on the host (measured, 2026-11-11).
The capability gap names it ("a statically linked child keeps
the direct exec"). The native-static rung also refuses a run
with `-v`: its handler has no table of volumes.

## Premise

T-1652 gives `podvm --podvm-dispatch-enter ROOT EXE ARGV...`,
which enters a static program in the root. The interposer
reads its table (`PODVM_MAPS`) and already knows the guest
path of each child (`PODVM_GUEST_EXE`).

## Approach

Give the interposer the host path of podvm (a variable of the
loader family's environment). For a static ELF child of the
machine, the exec route runs podvm with `--podvm-dispatch-enter`,
the rootfs and the guest path, in place of the direct exec. Give
the handler of `podvm-dispatch` the volumes of the run: a guest
prefix of a volume resolves in the volume's host directory, with
`RESOLVE_IN_ROOT` at that directory.

## Decision

A static child keeps the direct exec only where the native-static
rung cannot arm, and the capability gap says so then.

## Prove

`RUSTFLAGS="-C target-feature=-crt-static" cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu -- --test-threads=1`
exits 0 with a test of the route of a static child, and
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with a clause where a dynamic shell of the golang image
runs `go build`, the output is in the rootfs and not on the host,
and a clause where a static payload reads a file of a `-v`
volume on the native-static rung.

## Correction

**Filed 2026-11-11.** From the measurement of T-1652.

## Done

**Done 2026-11-11.** The interposer's tests exit 0 (85) with
`a_static_child_enters_the_native_static_rung_through_podvm`, and
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 on the cloud
host ([the run](../experiments/results/no-chroot-rung.txt), verdict "NO-CHROOT FAMILY RUNS"): in clause 33 the golang
image's `sh` runs `go build` and the program it built prints `g724-ok`, with
no `/tmp/g724` and no `/go` on the host; a static payload on the
native-static rung reads and writes a file of `-v HOST:/data` (the write lands
in HOST), and `-w /data` reads back as `/data`.

**Partial, 2026-11-11.** The static child: the interposer reads a static
ELF of the machine as `File::Static`, and where podvm names itself in
`PODVM_DISPATCH_SELF` (`podvm_dispatch::SELF_VAR`, set by podvm where Syscall
User Dispatch arms) the exec route runs `podvm --podvm-dispatch-enter ROOTFS
GUEST ARGV...` (`Tail::Dispatch` in `crates/podvm-interpose/src/exec/launch.rs`);
a `#!` script with a static interpreter takes the same route with the
kernel's arguments. The interposer's tests exit 0 (84, with the new test of
the route). On the chroot-denied fixture of experiment 356, `sh -c 'go build
-o h main.go && ./h'` in the golang image printed `g724-ok`, and nothing
landed on the host (no `/tmp/g724`, no `/go`). The `--strict` gap of the loader family names
this entry now.

The volume, the same day: the handler reads the volume pairs of
`PODVM_MAPS` at the entry, opens each host directory, and resolves a guest
path under a volume with `openat2(VOLFD, REST, RESOLVE_IN_ROOT)`
(`path::base_of`, `path::open_in`); a host path in a volume reads back as
its guest path (`path::guest_of_host`, for `getcwd` and `/proc/self/fd`).
`refuse_unserved` lets a volume through on the rung; `:ro` and
`--read-only` stay refused by name (T-1655). On the fixture, a static probe
read and wrote a file of `-v HOST:/data`, the write landed in HOST, and
`-w /data` read back as `/data`. Remaining: the run of experiment 356 with
clause 33.

# T-1729: An optional `runsc` backend where a host grants user namespaces

**Source:** T-1657 (its Approach, the part that it did not take)
**Category:** enter
**Milestone:** none
**Priority:** P3
**Effort:** M
**Status:** done

## Problem

gVisor's `runsc` answers a payload's system calls in its own
kernel. podvm cannot offer it on the target host, which denies
user namespaces, mount and ptrace. A host that grants user
namespaces could run it from the namespace rung.

## Premise

Issue 169 quotes `runsc --rootless --network=none|host
--ignore-cgroups --platform=systrap run`, and F14: `runsc do
-volume` wrote into gVisor's overlay, and rootless `create` is
refused. The syscall tier of T-1657 gives the view on the
target host.

## Approach

Where the probe reports user namespaces and a `runsc` on
`PATH`, `--podvm-tier=runsc` builds an OCI bundle from the
container's own tree (T-1709) and runs it, with `/workspace`
as a bind mount of the bundle (F14).

## Decision

Optional, and never a fallback of the syscall tier: a caller
asks for it by name.

## Prove

`sh experiments/436-runsc.sh` exits 0 on a host with user namespaces and `runsc`: `uname -r` names gVisor, a write to `/workspace` is on the host after the run, and on a host without them the tier refuses by name

## Correction

**Filed 2026-11-11.** Split from T-1657. Measured 2026-11-12: rootless
`runsc create` is not needed; `runsc --rootless run` of a bundle works,
and a bind mount of the bundle writes on the host. gVisor 20261102 needs
its sidecars of `gvisor-bin/` beside `runsc`.

## Done

**Done 2026-11-12.** `crates/podvm/src/runsc.rs` and
`crates/podvm/src/run/runsc.rs`: `--podvm-tier=runsc` checks for `runsc`
(`PODVM_RUNSC`, else `PATH`) and for user namespaces (`unshare` in a
child) before the pull, copies the image into the container's own tree,
writes the bundle (each `-v` a bind mount, `:ro` and `--read-only`
read-only, a numeric `--user`), and starts `runsc --rootless
--network=host --ignore-cgroups --platform=systrap --overlay2=none run`.
The bundle, the tree and the state of the container go at the end. `sh
experiments/436-runsc.sh` exits 0 on this host (Linux 6.18, gVisor
20261102 by its SHA-512): `uname -r` gives `4.19.0-gvisor`, the exit code
3 of the payload is the run's, a write with a line of stdin is on the
host, `--read-only` and `:ro` give EROFS, nothing of the run stays, and
the tier refuses at 125 by name with no `runsc` and where `unshare(2)` is
denied ([the run](../experiments/results/runsc.txt)). The tests of
`runsc_tests.rs` hold the bundle and the arguments.

# T-1732: Take podcrates v0.22.0: a detached payload ends with its launcher

**Source:** PodUtils/podcrates#71 (COORD-046, a message from a peer
  session); podcrates podcrates@T-205 and podcrates@T-202; podrunner's test 26;
  `crates/podvm-supervise/src/launcher.rs:78`
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

A payload of `podvm run -d` outlived a launcher that was
killed. The container then ran with no supervisor: no wall
bound, no log, no exit code.

## Premise

podcrates `v0.22.0` (`93de04a`) gives `Plan.die_with_parent`:
the child sets `PR_SET_PDEATHSIG(SIGKILL)` first after the
fork, and ends where its parent ended before the call. The
signal fires at the exit of the thread that forked, so it is
opt-in. The launcher forks from its one thread.

## Approach

Move each pin of `crates/*/Cargo.toml` from `v0.21.0` to
`v0.22.0` in one commit. Set `die_with_parent: true` in the
plan of the launcher, and `false` at each other site, where
the behaviour does not change.

## Decision

Only the launcher opts in. A foreground run keeps its
behaviour: a docker client that is killed does not stop its
container either.

## Prove

`sh experiments/437-payload-dies-with-launcher.sh` exits 0:
after SIGKILL to the launcher of `podvm run -d
public.ecr.aws/docker/library/busybox:musl sleep N`, the
payload is gone or a zombie within 5 s, and `ps -a` names
the container Dead.

## Done

**Done 2026-11-11.** The 9 manifests name `v0.22.0`, and `Cargo.lock`
changes only the version and the git revision of each `pod-*` package.
`PODVM_437_IMAGE=mirror.gcr.io/library/busybox:musl sh
experiments/437-payload-dies-with-launcher.sh` exits 0 on the cloud host
(`experiments/results/payload-dies-with-launcher.txt`): the payload is a
child of the launcher, it is a zombie (state Z) right after the launcher's
SIGKILL, and `ps -a` says "Dead (exited while the launcher was not
watching)". The control, the same build with `die_with_parent: false` in
the launcher, leaves the payload alive (state S), and the experiment
exits 1.

# T-1731: A dynamic program on the native-static rung: its loader under the handler

**Source:** T-1657 (the syscall tier refuses a dynamic program); the `--strict` gap of the loader family (T-1650)
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The native-static rung and the syscall tier run a static
program only: an `execve` of a dynamic one fails with
`ENOEXEC`, and `--podvm-tier=syscall` refuses it. A dynamic
program runs on the loader family instead, where a raw system
call (Go with cgo, liburing) passes the interposer and its
absolute path resolves on the host. That is the gap that
`--strict` names for the loader family.

## Premise

The loader of `podvm-dispatch` maps a static ELF
(`crates/podvm-dispatch/src/elf.rs`). A dynamic ELF names an
interpreter (`PT_INTERP`), which is itself a static program of
the image (`ld-musl-*.so.1`, `ld-linux-*.so.2`).

## Approach

Map the interpreter of the image beside the program, give the
auxiliary vector its `AT_BASE`, `AT_PHDR`, `AT_ENTRY` and
`AT_EXECFD` values, and start at the interpreter's entry. Each
library that it opens resolves through the handler, so each
path of the program and of its libraries is in the image.
Let the syscall tier take a dynamic program then.

## Decision

The interposer is not needed on this rung: the handler sees
each call, raw or not.

## Prove

`cargo test -p podvm-dispatch` exits 0 with a test that runs a dynamic program of a root under the handler; `sh experiments/413-syscall-tier.sh` exits 0 with a clause where a dynamic program of an image runs on the syscall tier and a raw `openat` of an absolute path reads the image's file

## Correction

**Filed 2026-11-11.** Split from T-1657, so that the `--strict` gap of the
loader family names an entry that is not done.

## Done

**Done 2026-11-12.** `elf::load` of `crates/podvm-dispatch/src/elf.rs`
maps a dynamic program, then reads its `PT_INTERP`, opens that path in
the root, and maps the interpreter (a shared object with no interpreter of
its own) as a second image; `enter.rs` gives `AT_BASE` the interpreter's
load address, keeps `AT_ENTRY` at the program's entry, and starts at the
interpreter's entry. The check of an `execve` takes a dynamic program, and
`--podvm-tier=syscall` takes one (`podvm_dispatch::host_program`). Found
on the way: the payload of this rung inherited podvm's interposer in
`LD_PRELOAD`, which turned each guest path into its host path before the
handler saw it, so each `execve` of a dynamic shell failed with `ENOENT`;
the entry now drops that object from `LD_PRELOAD` (each other object
stays).

`cargo test -p podvm-dispatch` exits 0 with
`a_dynamic_program_of_the_root_runs_with_its_own_interpreter` (the
host's dynamic `cat`, its interpreter and its libraries in a root read
the root's marker) and
`the_interposer_leaves_ld_preload_and_each_other_object_stays`.
`PODVM_413_MIRROR=1 sh experiments/413-syscall-tier.sh` exits 0 with
clause 7: alpine's dynamic busybox prints `3.20.10`, and `rawcat`, a
dynamic glibc program of the image, reads the image's `/etc/os-release`
through a raw `openat` system call (`ID=podvm413`).

# T-1730: A detached thread of a static payload ends with SIGSEGV on the native-static rung

**Source:** Found by T-1666: the probe inside the payload died on signal 11 on the chroot-denying fixture
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

musl ends a detached thread with `__unmapself`: `munmap` of
the thread's own stack, then `exit`. On the native-static rung
the `munmap` traps into the SIGSYS handler, which runs on that
stack (a thread has no alternate stack). The handler unmapped
it, and its return faulted: the whole payload died with
SIGSEGV. Each static program that drops the handle of a thread
(Rust's `std::thread::spawn`, a DNS resolve with a bound) hits
it.

## Premise

Measured on 2026-11-11 with `strace -f`: the thread called
`munmap(0x7f61d59fd000, 2109440)` and took SIGSEGV at
`0x7f61d5bfec50`, in the range it had unmapped.

## Approach

Where a `munmap` takes the stack that the handler runs on, the
handler makes the `munmap` and the `exit` of the thread in one
routine of assembly that touches no stack
(`podvm_dispatch_unmap_exit`).

## Decision

A call that unmaps the handler's own stack cannot return, so
its only valid next call is the exit; musl makes it.

## Prove

`cargo test -p podvm-dispatch` exits 0 with `a_detached_thread_ends_and_the_process_goes_on`, and the test fails with the new arm taken out

## Correction

**Filed 2026-11-11.** Found by T-1666.

## Done

**Done 2026-11-11.** `cargo test -p podvm-dispatch` exits 0, 9 tests of
`tests/dispatch.rs`. The plant: with the arm of `MUNMAP` taken out
(`if false &&`), `a_detached_thread_ends_and_the_process_goes_on` failed;
with it back, it passed.

# T-1740: Raw system calls of a dynamic program on the loader family: measure the syscall tier, then choose the default

**Source:** T-1731 (a dynamic program on the native-static rung); the `--strict` gap of the loader family (T-1650)
**Category:** enter
**Milestone:** none
**Priority:** P3
**Effort:** M
**Status:** partial

## Problem

A dynamic program on the loader family can issue the `syscall`
instruction itself (Go with cgo, liburing). That call passes the
interposer, so its absolute path resolves on the host, and a write lands
on the host tree. The loader family is the default for a dynamic program
on a host that denies chroot.

## Premise

Since T-1731, the native-static rung and `--podvm-tier=syscall` run a
dynamic program with each of its system calls given to podvm's handler,
raw or not, where Syscall User Dispatch arms. Each call then costs a
signal (SIGSYS) and a return through the handler. Not measured: the cost
against the loader family for a build or a test run.

## Approach

Measure the wall time of one workload (a `go build`, an `npm test`) on
the loader family and on the syscall tier on one host. Make the syscall
tier the default for a dynamic program where the cost is small, or keep
the loader family and let `--strict` name `--podvm-tier=syscall` as the
rung that runs a payload that hits the gap.

## Decision

Until the measurement, the loader family stays the default, and the gap
names `--podvm-tier=syscall` as its remedy.

## Prove

`sh experiments/438-raw-syscall-cost.sh` exits 0 with the two wall times of each workload, and `cargo test -p podvm` exits 0 with the default that the measurement chose

## Correction

None.

## Done

**Partial 2026-11-12.** `PODVM_438_STORE=... sh
experiments/438-raw-syscall-cost.sh` exits 0 on the chroot-denying
fixture (4 CPUs, Linux 6.18), alpine 3.20, wall times in ms:

```text
workload  loader-family  syscall-tier
exec      1880           2392   (2000 runs of cat: +27 %)
io        631            609    (md5sum of the image, 20 times)
calls     311            667    (dd bs=1 of 200000 bytes: x2.1; the copy alone 0.12 s against 0.50 s)
```

Decision (the call of this session, AGENTS.md section 10): the loader
family stays the default for a dynamic program. A workload that is made
of system calls runs up to 4 times slower on the syscall tier, and the
tier changes what the payload sees (`/tmp` capped, `uname` fixed, no
`run -d`). The `--strict` gap names `--podvm-tier=syscall` as the rung
that runs a payload with raw system calls
(`crates/podvm/src/complete/capability.rs`).

Remaining: the gap stays open while the tier costs this much; a cheaper
mechanism (a filter that gives the handler only the calls with a path)
would let the tier be the default. Not measured: Go with cgo and
liburing themselves.


# T-1742: The loader family does not find the glibc of a Debian image on arm64

**Source:** issue 282 (COORD-061); T-1721 (experiment 356 on arm64,
  clauses 18, 19 and 33); pod-enter v0.30.0 `abi::libc_beside`
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

On arm64, the loader family declines each dynamic glibc program of a
Debian bookworm image (gcc, golang): "/lib/ld-linux-aarch64.so.1 names no
C library podvm can find in the image".

## Premise

`/lib` is a link to `usr/lib`, and `libc.so.6` is in
`/usr/lib/aarch64-linux-gnu/`. The search of pod-enter walks with a stack,
so it walks the last name of a directory first, and it stops after 512
entries. `x86_64-linux-gnu` sorts last and is found; `aarch64-linux-gnu`
sorts first and the search stops before it. Measured 2026-11-12 on a
synthetic tree: 800 entries in the other directories of `/usr/lib` give
`None`, 500 give the libc.

## Approach

pod-enter belongs to podcrates: a breadth-first walk that walks each real
directory one time (issue 282). podvm takes that tag.

## Decision

The fix belongs to pod-enter (issue 282); podvm keeps no second copy of
the search.

## Prove

`cargo test -p podvm` exits 0, and clauses 19 and 22 of experiment 356
exit 0 on arm64 (`.github/workflows/arm.yml`)

## Correction

**Unblocked 2026-11-12.** podvm takes podcrates `v0.31.0` (`45b145a`,
PodUtils/podcrates#90): `libc_beside` looks first at the multiarch places
of each library root, and walks a real directory once. Its test
`the_multiarch_libc_is_found_beside_large_trees_of_a_merged_usr` holds the
tree of issue 282 for aarch64 and x86_64. Remaining: the run of `arm.yml`.

## Done

**Done 2026-11-12.** `cargo test -p podvm` exits 0 on podcrates
`v0.31.0`, and `.github/workflows/arm.yml` run 43228497851 runs experiment
356 on arm64 with exit 0: clauses 19 and 22 (the gcc bookworm image, with
the glibc object) and the dladdr and library clauses of the debian image
pass; the loader family finds `/usr/lib/aarch64-linux-gnu/libc.so.6`.

# T-1749: A read-only image as the lower layer of the root, with a tmpfs upper layer, and no copy

**Source:** PodUtils/podcrates#124 (the asks of podswarm to podvm, a message from a
  peer session; PodUtils/podcrates#107 is its tracker), ask V-5
**Category:** enter
**Milestone:** none
**Priority:** P3
**Effort:** L
**Status:** partial

## Problem

`run` copies the image into a tree for each container. A podswarm task
for each attempt pays the copy each time.

## Premise

A user with no root has no overlay mount where namespaces are denied. On
the userland rung the path table of the interposer can map a path to a
read-only lower tree and a per-run upper tree; on the namespace rung an
overlay mount in the user namespace can do it (Linux 5.11 and later).

## Approach

Measure the cost of the copy first. Then, for each rung, choose the way:
an overlay mount in the user namespace where it mounts, else a copy-up
of the path table on the loader family. Each rung that cannot do it keeps
the copy and says so.

## Decision

Measure before the design: the entry can close with a measurement that
shows that the copy is not the cost.

## Prove

`cargo test -p podvm` exits 0, with a test that a second `run` of the same
image writes no copy of the image (the store holds one tree of the image
after two runs)

## Done

**Partial 2026-11-13.** The first step of the Approach, the measurement.
`PODVM_442_MIRROR=1 sh experiments/442-run-copy-cost.sh` exits 0
([the run](../experiments/results/run-copy-cost.txt)) on the cloud host
(ext4, no FICLONE, so each fill copies the bytes): the median of 5 runs
of `run --rm IMAGE true` is 77 ms for busybox:musl (1.5 MiB, fill 5 ms,
6 %), 2861 ms for ubuntu:24.04 (74.6 MiB, fill 681 ms, 23 %) and 12611 ms
for golang:1.24.7-bookworm (810.3 MiB, fill 8377 ms, 66 %). The copy is
the cost of a large image, so the entry goes on. Remaining: the root with
no copy for each rung (an overlay mount in the user namespace where it
mounts; else a copy-up of the path table on the loader family), and the
Prove. The overlay mount belongs to `pod-enter`'s namespace rung: COORD-071
(issue 294) asks podcrates for it. The loader family is podvm's part.

**Partial 2026-11-14.** T-1781 takes the copy off each run after the
first: a spare tree of the image, filled after the run before, and the
removal of a `--rm` tree after podvm's exit (golang: 181 ms with the
spare against 8.5 s, experiment 448). The root with no copy stays the
goal of this entry: the first run of an image and each `create` still
copy, and a spare uses the disk of one more tree.

**Partial 2026-11-14, later.** podcrates `v0.42.0` carries the overlay
root of the namespace rung (podcrates@T-486, `Root::Overlay` with
`run_namespace_root`), and podvm takes the tag. The remaining step is
the completion: `pod_rootfs` completes one root path, and with an
overlay its writes must land in the upper layer while its reads see the
image below. A copy-up of a fixed list of paths would break the first
time `pod_rootfs` touches a new path, and it cannot serve a listing that
both layers hold. COORD-090 (issue 326) asks podcrates for a two-layer
completion; podvm wires the overlay root when a tag carries it.


# T-1758: The namespace rung has no /proc in the root: go cannot find GOROOT, /proc/self/status is missing

**Source:** experiment 443 (`experiments/443-qa-surface.sh`, the QA pass of 2026-11-13, [the run](../experiments/results/qa-surface.txt))
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

On the namespace rung `go version` fails with `readlink /proc/self/exe: no such file or directory`, and `/proc/self/status` is missing, although the rung can mount.

## Premise

The rung mounts a tmpfs on /tmp and nothing on /proc; the mount of proc needs a PID namespace or a bind of the host's /proc (podcrates' `pod-enter`).

## Approach

Mount proc in the namespace rung (a new PID namespace where it is permitted, else a read-only bind of /proc), in `pod-enter` through a COORD issue if the step is the crate's.

## Decision

The fix in the entry is podvm's; a cause in a crate of podcrates goes there as a COORD issue.

## Prove

`sh experiments/443-qa-surface.sh` gives PASS for the rows `imgs.go-version.ns`, `tools.go-mod.ns`, `run.proc.ns`, and a test of the entry holds the fix

## Done

**Done 2026-11-13.** podcrates `v0.37.0` (PodUtils/podcrates#134, its
podcrates@T-491) mounts a fresh proc on the /proc of the image in the namespace rung
of `pod-enter`, after the tmpfs on /tmp; a refused mount names itself and
the rung goes on. podvm takes the tag. On the namespace rung of the cloud
host, `run --rm alpine head -2 /proc/self/status` prints `Name: head`, and
`run --rm golang:1.24-bookworm go version` prints `go version go1.24.13
linux/amd64`. `cargo test --workspace` passes (798 tests). The rows of the
Prove come with the next rerun of experiment 443.

# T-1759: No /dev/shm on either rung: python multiprocessing fails

**Source:** experiment 443 (`experiments/443-qa-surface.sh`, the QA pass of 2026-11-13, [the run](../experiments/results/qa-surface.txt))
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`multiprocessing.Pool(2)` fails with FileNotFoundError on the namespace rung; `/dev/shm` does not exist on both rungs. On the userland rung python works only because `sem_open` reaches the host's /dev/shm.

## Premise

The rootfs has no /dev/shm, and podvm makes none.

## Approach

Give each run a private /dev/shm: a tmpfs on the namespace rung, a directory of the run mapped by the path table on the userland rung.

## Decision

The fix in the entry is podvm's; a cause in a crate of podcrates goes there as a COORD issue.

## Prove

`sh experiments/443-qa-surface.sh` gives PASS for the rows `imgs.python-mp.ns`, `run.shm.ns`, `run.shm.ul`, and a test of the entry holds the fix

## Done

**Done 2026-11-13.** `podvm-complete` makes `dev/shm` in the rootfs after
the completion of `pod-rootfs`, with mode 1777 (`crates/podvm-complete/src/shm.rs`,
the row `dev-shm`). It does not write through a `dev` or `dev/shm` of the
image that is a link or not a directory. The namespace rung runs on its
private copy of the rootfs, and the path table of the userland rung maps
/dev/shm into it. On both rungs, `echo x > /dev/shm/F` works and
`multiprocessing.Pool(2).map` gives `[1, 2]` in `python:3.12-slim`; the
rows of the Prove pass in the rerun of experiment 443 before the release.
The test `dev_shm_is_made_once_and_never_through_a_link` holds the step.
It is a directory, not a tmpfs: `--shm-size` stays unserved.

# T-1764: `--podvm-confine` without /etc in the read list breaks the loader on the namespace rung

**Source:** experiment 443 (`experiments/443-qa-surface.sh`, the QA pass of 2026-11-13, [the run](../experiments/results/qa-surface.txt))
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

`--podvm-confine 'read=/usr;write=/tmp'` with python exits 127: `libpython3.12.so.1.0: cannot open shared object`.

## Premise

ld.so reads `/etc/ld.so.cache`, which the spec does not allow, and its search does not reach /usr/local/lib.

## Approach

Allow `/etc/ld.so.cache` and the library directories of the image in each confine spec, as the system directories are.

## Decision

The fix in the entry is podvm's; a cause in a crate of podcrates goes there as a COORD issue.

## Prove

`sh experiments/443-qa-surface.sh` gives PASS for the rows `run.confine-py.ns`, and a test of the entry holds the fix

## Done

**Done 2026-11-13.** `SYSTEM_READ` of `crates/podvm-enter/src/confine.rs`
now holds the files of /etc that the dynamic loader reads before `main`:
`/etc/ld.so.cache` (glibc) and `/etc/ld-musl-x86_64.path` and
`/etc/ld-musl-aarch64.path` (musl). Each gets a read rule where the image
has it. Without the cache, glibc's loader searched only /lib and /usr/lib,
and python of /usr/local/lib did not start. By hand on the cloud host:
`run --podvm-confine 'read=/usr;write=/tmp' python:3.12-slim python3 -c
'open("/etc/hostname").read()'` now raises `PermissionError` on
/etc/hostname, as the row expects. The test
`the_loader_reads_its_cache_under_each_spec` holds the rule, and the old
test still holds that no other file of /etc gets one. The row of the Prove
comes with the rerun of experiment 443 before the release.


# T-1804: `--read-only` with `-v HOST:/GUEST/:ro` leaves the volume writable

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), F-04 (CODE-04; verify item 5 and V-4)
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

`podvm run --read-only -v $PWD/vol:/data/:ro alpine:3.20 sh -c 'echo CHANGED > /data/f'`
succeeds and the file changes on the host. Without the trailing slash,
or without `--read-only`, the write is refused. The flag that makes a run
stricter makes a `:ro` volume writable, and `--strict` does not see it.
The table is `PODVM_RO=ro:/,rw:/dev,rw:/data/,ro:/data`.

## Premise

Two causes. `crates/podvm/src/run/parse/values.rs:104` records the `:ro`
guest without its trailing slash, and the volume keeps `/data/`; so
`read_only_table` (`crates/podvm/src/lifecycle/volume.rs:331`) emits both
`rw:/data/` and `ro:/data`. `denied_in`
(`crates/podvm-interpose/src/map/readonly.rs:102`) strips one trailing
slash from each prefix and keeps the first entry on a tie of length,
which is the `rw` entry.

## Approach

Normalize each guest path once, where the volume is parsed, and build
each table from that one spelling. In `denied_in`, let an `ro` entry win
a tie of length, so that a duplicate can never widen access.

## Decision

Both changes: the first removes the duplicate, the second makes a
duplicate safe. The interposer is podvm's own crate.

## Prove

`cargo test -p podvm-interpose denied_in` exits 0 and `cargo test -p podvm --bin podvm read_only_table` exits 0, with tests that `ro:/,rw:/data/,ro:/data` denies `/data/f` and that `--read-only -v H:/data/:ro` gives no `rw:` entry for `/data`

## Done

**Done 2026-11-14.** Both causes. `run/parse/values.rs` gives the guest
path of each `-v` one spelling (`lifecycle::volume::guest_spelling`: no
trailing `/`, except `/`), and `read_only_table` compares each path in
that spelling, also for a record of an older podvm. `denied_in`
(`podvm-interpose`, `map/readonly.rs`) lets an `ro:` entry win a tie of
length. Tests: `denied_in_lets_ro_win_a_tie` (run in
`crates/podvm-interpose` with `cargo test --release --target
x86_64-unknown-linux-gnu denied_in`, because the crate is outside the
workspace), `read_only_table_has_one_spelling` and
`read_only_table_of_a_volume_with_a_trailing_slash`. By hand: `run
--read-only -v H:/data/:ro alpine:3.20 sh -c 'echo CHANGED > /data/f'`
exits 1 and the host file keeps its text, on the namespace rung and on
the fixture of the userland rung, with and without the `/`; beta.15
exits 0 and changes the file on both. Experiment 443 has the row
`run.readonly-ro-slash`.


# T-1819: An `exec=` rule of `--podvm-confine` does not deny a program on the loader family

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), O-03 (MACH-07; verify V-1; REPORT, "Findings that were rejected or corrected", rows 1, 2 and 6)
**Category:** enter
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

On the userland rung, `--podvm-confine 'read=/etc;write=/tmp;exec=/bin;connect=443'`
lets `/tmp/s.sh` (a script) and `/tmp/bb` (busybox, copied) run; a
script and a program in a volume also run, and `exec=` empty runs
`/vol/bb`. The same spec on the namespace rung denies the script (126).
The read, write and connect rules are enforced. The results for a static
program disagree: the report owner says that a static program in `/tmp`
runs under `read=/tmp;exec=/bin`, and the verifier says that a static
child is denied under each of `exec=/bin`, `exec=/tmp`, `exec=/tmp/pv`
and `exec=/` (126), so a rule that grants `/tmp` denies it.

## Premise

On the loader family the interposer starts a dynamic program or a script
through the loader of the image, which is in a system directory that
the layer always grants; Landlock then checks EXECUTE on the loader and
only READ on the program. The layer is built in `pod-enter` and
`pod-confine` (podcrates v0.42.0), reached from
`crates/podvm/src/lifecycle/confine.rs`. The static case is not settled.

## Approach

First measure the static case on both rungs with a static ELF, not a
script. Then the interposer checks the exec table against the resolved
program path before it starts the loader, and refuses with EACCES; and
podvm asks podcrates to check that `LANDLOCK_ACCESS_FS_EXECUTE` is in the
handled set and granted for each `exec=` path (proposed COORD issue).
Until both hold, refuse `exec=` on the loader family by name.

## Decision

A confine rule that the binary cannot enforce is refused, not accepted
(the recommendation of the report, step 3).

## Prove

`cargo test -p podvm --test confine_exec` exits 0, with tests on the userland rung that a script, a dynamic ELF and a static ELF outside each `exec=` rule are denied, and that a static ELF inside `exec=/tmp` runs

## Correction

**Amended, 2026-11-14.** The test target `confine_exec` of the Prove was
not made: the measurement needs an image and the fixture of the userland
rung, so it is in experiment 443 (rows `run.confine-exec-script`,
`-copy`, `-static` and `-in`, both rungs), with the unit tests
`exec_allowed_in_takes_each_prefix` and
`exec_table_names_the_system_dirs_and_each_exec_path`.

## Done

**Done 2026-11-14.** Two causes, both in podvm's own code; no wait for
podcrates. (1) On the userland rung the kernel executes the loader, so
Landlock never checked EXECUTE on the program: podvm now gives the
interposer the exec table of the layer, `PODVM_EXEC` (the system
directories of the image and each `exec=` path, as guest paths;
`lifecycle/confine.rs`, `exec_table`, set by `decide`, which drops a
caller's own), and `plan` of the interposer (`exec.rs`) refuses with
EACCES each level (the program, a script and its interpreter) whose
resolved guest path no prefix holds (`map/readonly.rs`,
`exec_allowed`). (2) The static case of the report: a static child of
the loader family enters the native-static rung through podvm's own
binary (T-1724), and the layer gave that binary execute only on the
other families, so a static program that `exec=` granted got 126; it
now gets read and execute on each family. Tests:
`exec_allowed_in_takes_each_prefix` (interposer) and
`exec_table_names_the_system_dirs_and_each_exec_path`. A deviation from
the Prove: the measurement is in experiment 443, not in a test of
`cargo test --test confine_exec`, because it needs an image and the
fixture of the userland rung; the rows `run.confine-exec-script`,
`-copy`, `-static` and `-in` run on both rungs, and the section run
gave 237 pass and 0 fail. By hand, on the fixture
and on the namespace rung: outside `exec=`, a script, a copied busybox
and a static program give 126 with the line "the confine layer denies
the exec of this path (exec=)"; inside `exec=/vol`, the script and the
static program run. The ask of COORD-098 (issue 344, podcrates@T-583)
stays for the check of the crate. ⚠ The interposer is not a boundary:
a raw `execve` system call passes it, as the module doc of
`map/readonly.rs` says of each check of the table.

# T-1887: `--podvm-confine` on the userland rung resolves an absolute link of the image against the host root

**Source:** PodUtils/podvm#364 (Nemo-010, 2026-11-14), with a patch
**Category:** enter
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

On the userland rung, `host_spec` (`crates/podvm-enter/src/confine.rs`)
maps each guest path of the spec to a host path by a lexical join onto
the rootfs. A link of the image whose target starts with `/` is then
opened from the host root. `podvm run --podvm-confine exec=/bin/sh
alpine:3.20 /bin/sh -c 'echo hi'` exits 125 with "does not exist":
`bin/sh` links to `/bin/busybox`, which the image has and the host does
not. 81 of the 82 names in `/bin` of the image are such links. A link to
a path that only the host has compiles a rule on the host inode.

## Premise

On a rung with a chroot, `compile_in` opens each rule with
`RESOLVE_IN_ROOT`, so a link resolves inside the image. The loader
family has no root descriptor, so podvm must resolve the links itself,
as the interposer resolves them for the payload.

## Approach

Take the patch of the report: `host_resolved` walks the guest path one
component at a time inside the tree that serves it (the rootfs or a
volume), restarts at that tree on an absolute target, stops `..` at its
root, and bounds the walk at 40 links. `host_spec` uses the result for
the existence test and for the rule. Leave out the debug print of the
patch.

## Decision

A guest path resolves inside the tree that serves it, on every rung.
The host decides no verdict of the spec. A required path whose walk
leaves the tree is T-1888.

## Prove

`cargo test -p podvm-enter confine` exits 0 with the three tests of the report, and on the userland fixture `podvm run --rm --podvm-confine exec=/bin/sh` of `mirror.gcr.io/library/alpine:3.20` with `/bin/sh -c 'echo hi'` prints `hi` and exits 0

## Done

**Done 2026-11-14.** The patch of PodUtils/podvm#364 (Nemo-010) is in
`crates/podvm-enter/src/confine.rs` (`host_resolved`), without its debug
print. `cargo test -p podvm-enter confine` exits 0 with the three tests
of the report. On the userland fixture with
`mirror.gcr.io/library/alpine:3.20` and `/bin/sh -c 'echo hi'`:
`exec=/bin/sh`, `read=/bin/cat`, `exec=?/bin/sh`, `read=/bin`,
`read=/etc/mtab` and `exec=/bin/busybox` each print `hi` and exit 0;
beta.17 on the same fixture refused `exec=/bin/sh` at 125 with ENOENT on
the host path of the tree.

# T-1888: A required confine path whose link leaves the tree still grants a host inode

**Source:** PodUtils/podvm#364, "What the patch does not change"
**Category:** enter
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

After T-1887, a required path (no `?`) whose walk finds nothing inside
the tree falls back to the lexical host path. A link of the image to a
path that only the host has (`etc/abs-host -> /bin/sh`) then compiles a
rule on the host's inode, and the run exits 0.

## Premise

A rule that names a host inode is the class of surprise that T-0405
refuses for `etc/mtab`. The payload's own path calls go through the
interposer and see the tree, so the rule grants more than the tree
holds.

## Approach

Make `host_spec` fallible: a required path that does not resolve inside
the tree refuses by name, with the guest path the caller wrote, and an
optional path takes no rule.

## Decision

A required path that the image lacks refuses, wherever the host has it.

## Prove

`cargo test -p podvm-enter confine_required_link` exits 0, with a test that a required rule on a link to a host-only path refuses with the guest path in the message

## Done

**Done 2026-11-14.** `host_spec` returns a `Result`: a required path
that does not resolve inside its tree refuses with the guest path, and
the message says whether the image lacks the path or holds a link out
of the tree. `create` keeps the SPEC and leaves the check to `start`,
as before. `cargo test -p podvm-enter confine_required_link` exits 0.
By hand on the userland fixture: `create --podvm-confine
read=/etc/abs-host`, then a link `etc/abs-host -> /usr/bin/python3` in
the tree (a path of the host only), and `start` refuses at 125 with
"the path \"/etc/abs-host\" of the spec is a link of the image that
leaves the tree"; a link to `/bin/busybox` starts and exits 0; `run
--podvm-confine read=/nosuch` refuses at 125 with "is not in the image",
and `read=?/nosuch` runs.

# T-1892: Two exec tables reach the payload: podvm's guest paths and the crate's host paths

**Source:** the take of podcrates `v0.45.0` (2026-11-14), podcrates@T-583
**Category:** enter
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** open

## Problem

Since podcrates `v0.45.0`, `pod-enter` writes the exec table of the
confine layer into the environment of a payload with no chroot, as host
paths, under `Names::exec_table`. podvm's interposer reads its own table
of guest paths in `PODVM_EXEC` (T-1819). podvm names the crate's table
`PODVM_EXEC_HOST`, so a confined payload gets two variables, and the
interposer reads one of them.

## Premise

`pod_confine::ExecTable::allows_in` checks a resolved host path against
the table; the interposer of podvm resolves each program through the
table of the rootfs and can check the host path too.

## Approach

Move the check of the interposer to the crate's table of host paths
(`PODVM_EXEC_HOST`, the format of `ExecTable::encode`), remove
`exec_table` of `lifecycle/confine.rs`, and keep one variable.

## Decision

One table of each layer reaches the payload.

## Prove

`cargo test -p podvm --bin podvm exec_table` exits 0, and a run on the userland fixture with `--podvm-confine exec=/vol` has `PODVM_EXEC_HOST` and no `PODVM_EXEC` in its environment, and still gives 126 for a program outside the table
