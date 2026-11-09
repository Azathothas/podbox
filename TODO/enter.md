# enter

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

---

### T-0501 Open every descriptor before the root changes

Source:      `TOOL.md` section 6.5, `paper_final.md` section 10.5
Category:    enter
Priority:    P0
Effort:      M
Status:      done

Problem:     A chroot cuts off every path outside the new root. A child with no
             explicit stdio opens `/dev/null`, which an extracted rootfs does
             not have, and the failure names a missing file rather than a
             missing step.
Premise:     Read, and the same trap is measured from the other side by T-0401:
             the shell that redirects into a non-existent `/dev/null` creates a
             growing file.
Approach:    Open stdio, the log sinks, any host device the config exposes via
             `--device`, and a PTY pair if `-t` was asked for and T-0503 says it
             is available, **before** `chroot`. A descriptor opened before the
             change keeps working after it.
             ⭐ This is the only route by which a PTY can reach a chrooted
             payload at all: allocate the pair outside, pass the descriptors in,
             `TIOCSCTTY` in the child.
Decision:    Pass descriptors rather than bind-mounting anything. There is no
             attach path on this runtime, so a descriptor is the only thing that
             crosses the boundary.
Prove:       `podvm run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'echo hi' | grep -qx hi`

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
the device plan both halves name: `crates/podvm-enter/src/device.rs`
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

---

### T-0502 Resolve the program inside the new root, in the process that changed it

Source:      `TOOL.md` section 6.5; `paper_final.md` section 5.5
Category:    enter
Priority:    P0
Effort:      S
Status:      done 2026-09-09

Problem:     A prior implementation resolved the program in a child whose
             environment had been replaced. The child recomputed its store path
             from an empty environment, got a **relative** path, created a fresh
             empty rootfs under its working directory, chrooted into **that**,
             and reported `stat /bin/sh: no such file or directory` from a
             rootfs where that path exists.
Premise:     Read, in the account of a shipped bug. The general shape is in the
             corpus too: `references/89luca89__lilipod` is the Go implementation
             the account is about, and it is refused as a seed on language
             grounds and on licence grounds ([reference-map.md](reference-map.md)).
Approach:    Resolve the program path **after** `chroot` and `chdir("/")`, in
             the process that did them, never in the parent and never at command
             construction time. Do not strip the environment the child needs to
             find the store.
             ⚠ A relative store path is the specific hazard. Compute every store
             path from an absolute root captured before any environment change,
             and assert it is absolute.
Decision:    Resolve in the child rather than pass a pre-resolved absolute path
             from the parent. A parent-resolved path names the outer tree, and
             the outer tree is gone.
Prove:       `env -i "$(command -v podvm)" run --rm public.ecr.aws/docker/library/alpine:3.20 /bin/sh -c 'echo ok' | grep -qx ok`

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

---

### T-0503 Probe `/dev/ptmx`, and refuse `-t` by name where it is absent

Source:      `TOOL.md` section 0, section 6.5, section 6.8; `paper_final.md` section 10.5
Category:    enter
Priority:    P1
Effort:      S
Status:      done 2026-09-22

Problem:     `-t` either works or it does not, and the corpus disagrees with
             itself about which. A degraded PTY that cannot open a terminal is
             worse than a refusal, because the payload waits.
Premise:     ⚠ **Unresolved, and stated as unresolved.** The target's mount
             table shows exactly six device nodes bind-mounted into `/dev`
             (`full`, `null`, `random`, `tty`, `urandom`, `zero`), no `ptmx` and
             no `devpts`, and `mknod` cannot create one. An earlier account of
             the same runtime asserts the host `/dev/ptmx` works and published no
             capture. A mount table does not list plain files, so neither source
             settles it. One `stat("/dev/ptmx")` on the target would, and nobody
             has run one.
Approach:    `stat("/dev/ptmx")` in the **outer** environment, before the chroot,
             as part of T-0101's probe set. If present, allocate the pair
             outside and pass the descriptors in per T-0501. If absent, `-t` is a
             **named-reason refusal**: `-t requires /dev/ptmx in the outer
             environment; it is absent`.
             ⚠ A payload that reopens `/dev/pts/N` by name is unsupported either
             way, and the banner says so rather than letting it fail deeper.
Decision:    Refuse rather than degrade. `-t` is a request for a terminal, and a
             terminal that is not there cannot be approximated by a pipe without
             changing what every interactive program does.
Prove:       `podvm probe --json | jq -e 'has("ptmx")' && { podvm run --rm -t public.ecr.aws/docker/library/alpine:3.20 true || podvm run --rm -t public.ecr.aws/docker/library/alpine:3.20 true 2>&1 | grep -q 'ptmx'; }`

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

---

### T-0504 Refuse a rootfs path that is a symlink

Source:      `TOOL.md` section 6.5 step 1
Category:    enter
Priority:    P1
Effort:      S
Status:      done 2026-09-09

Problem:     `chroot` follows a symlink. A rootfs path that is one puts the
             payload somewhere the runtime did not choose, and every containment
             claim after that is about a different directory.
Premise:     Read.
Approach:    `lstat` the resolved rootfs path and refuse if it is a symlink,
             naming the path and its target. Then hold a directory fd on it and
             use `fchdir` plus `chroot(".")`, so nothing between the check and
             the change can swap the path.
Decision:    Refuse rather than resolve. Resolving accepts a rootfs the store
             does not own, and the store is where the ownership sidecar and the
             lock (T-0204) live.
Prove:       `ln -sfn "$(podvm inspect --format '{{.RootfsPath}}' public.ecr.aws/docker/library/alpine:3.20)" /tmp/rootlink && ! podvm run --rm --rootfs /tmp/rootlink public.ecr.aws/docker/library/alpine:3.20 true`

**Done 2026-09-09.** `podvm_enter::RootDir::open` `lstat`s with
`AT_SYMLINK_NOFOLLOW`, refuses a symlink **naming its target**, refuses anything
that is not a directory, and then holds the directory open with
`O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC`. The entry uses `fchdir` on that descriptor
and `chroot(".")`, so nothing between the check and the change can swap the path.

⚠ Two tests, and the second is what makes the first mean something: a symlinked
rootfs is refused, **and the directory it points at is accepted**, so the check
is about the symlink rather than about the path being rejected wholesale.

---

### T-0505 `exec` is a fresh chroot, and `inspect` says so

Source:      `TOOL.md` section 4.2, section 6.8; `paper_final.md` section 10.6
Category:    enter
Priority:    P1
Effort:      S
Status:      done 2026-09-09

Problem:     `docker exec` enters the container's namespaces. podvm has none to
             enter, so its `exec` re-runs the section 6.5 sequence against the same
             rootfs. It shares the filesystem tree with the original process and
             nothing else, and a caller that assumes otherwise gets a process
             that cannot see the original's `/proc`, its signals or its
             environment.
Premise:     Read.
Approach:    Implement `exec` as a fresh entry, and mark it **Degraded** in the
             parity table with the difference stated: "a fresh chroot re-entry;
             shares only the filesystem". `inspect` carries the true mode per
             container so a caller can check rather than assume.
             ⛔ Do not imply containment. A pidfd addresses one process; it does
             not contain descendants and it is not a PID namespace, and
             grandchildren that reparent are outside podvm's reach.
Decision:    A fresh chroot rather than refusing `exec`. `exec` is load-bearing
             for M4's lifecycle test and for every agent workflow, and the
             filesystem-only sharing is enough for the overwhelming majority of
             uses provided it is stated.
Prove:       `./experiments/320-cli-contract.sh` clause 4 and `./experiments/300-run.sh` clause 8: exec's stdout is the payload's, the banner says it is a fresh chroot, and `inspect --format '{{.Exec.Shares}}'` reads `filesystem`

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

---

### T-0506 A foreign-architecture container, and never a rung measured by the emulator

Source:      Found on 2026-09-09 by running `podvm probe` under `qemu-aarch64`, while closing [T-0911](deps.md)
Category:    enter
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     ⛔ **A probe run under `qemu-user` measures the emulator, and
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
Premise:     ⭐ **Measured, in this tree.** The aarch64 binary reports rung
             `namespace` under qemu on a host that is also `namespace`, so the
             two agree today and the agreement is a coincidence of this host
             rather than evidence. The rows that disagree are printed in the
             same result file, and they are the ones a caller would act on.
             ⚠ It is the same shape as `experiments/20-enter-target.sh`'s `/dev`
             answering out of the mount table the question doubts
             ([T-0503](#t-0503-probe-devptmx-and-refuse--t-by-name-where-it-is-absent)),
             and it has the same remedy: say which instrument answered.
Approach:    Read the image's platform and the host's, and where they differ:
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
Decision:    Report and refuse before falling back. A runtime that guesses which
             architecture a payload wanted has the same problem as one that
             guesses a rung. ⚠ The copy-in is deliberate and disclosed rather
             than refused, because refusing it would make podvm useless on
             every machine whose binfmt registration predates `F`, which is most
             of them.
Prove:       `podvm run --rm --platform linux/arm64 <image> /bin/true` succeeds where an interpreter is registered and exits 125 naming both platforms where none is, and `podvm probe --json` run inside it carries the interpreter in `measured_by`

**Done 2026-09-09.** `crates/podvm-enter/src/binfmt.rs` and the `run` verb,
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

---

### T-1317 A chroot-denied host gets an up-front refusal naming chroot, not a 125 after the work

Source:      issue 12, client beta testing 2026-09-22 (every rung dies at
             `chroot(".")` EPERM while probe selects `interpose`);
             `crates/podvm-enter/src/lib.rs` (fchdir/chroot/chdir entry
             sequence, T-0502)
Category:    enter
Priority:    P1
Effort:      M
Status:      done 2026-09-23

Problem:     On a host that denies `chroot(2)` itself, probe selects
             `interpose` and then every entry path (`run`, forced memfd,
             `start`) dies at `chroot(".")` EPERM, exit 125, after fixups,
             placement and the banner have all run. The ladder rungs
             change where payload bytes come from, not whether a chroot
             happens, so nothing reaches the payload on exactly the class
             of host the README names first.
Premise:     Measured by the reporter (uid 0, empty capability bounding
             set, seccomp 2): probe prints `chroot=EPERM` beside "would
             permit interpose", and the run fails post-banner. The entry
             sequence design (chroot always happens) confirms the
             mechanism on this tree.
Approach:    Refuse `run`/`start`/`create` up front where the entered
             rung's chroot is denied, naming `chroot(2)` before any
             fixup mutates the rootfs, reusing the probe's own chroot
             leg as the gate. Out of scope: a no-chroot rung (rejected
             below), rewriting the ladder, touching the threat model.
Decision:    Up-front refusal, for three reasons. A no-chroot rung would
             be strictly weaker than today's chroot rung while costing a
             subsystem, and the banner machinery would have to describe a
             mode weaker than anything it names today. Documenting
             chroot-denied hosts as unsupported contradicts the README's
             first paragraph, which names these hosts as the audience.
             The refused-rung table already has the legs to gate this.
Prove:       `podvm run` on a chroot-denied host (or a fixture that
             denies chroot) exits 125 naming `chroot(2)` denied with no
             rootfs mutation, and `podvm probe` maps its chroot leg to
             the same refusal; on a chroot-capable host every rung runs
             as before. Close issue 12 with a comment showing both
             outputs and the probe-gated refusal as the guard that stops
             recurrence.

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


---

### T-1339 Enter the namespace rung where the probe permits it

Source:      issue 59, beta.7 drive 2026-09-25 (`README.md:56-62`
             promised it, `run` never did); `crates/podvm-enter/src/lib.rs:47-61`
             (`ENTERED_RUNG` is `Chroot` on every machine),
             `pod_hostprobe::select` (returns
             `Rung::Namespace`), `crates/podvm/src/system/df_info.rs:213-216`
             (asserts the two differ)
Category:    enter
Priority:    P1
Effort:      L
Status:      done

Problem:     No TODO entry mentions the `namespace` rung, and nothing
             enters it. Where the probe succeeds at
             `unshare(CLONE_NEWNS)` and mount, `select.rs` returns
             `Rung::Namespace` and `run` still enters a plain chroot.
             The binary reports both facts (`system info` carries
             `.Rung` and `.EnteredRung` with the guard asserting they
             differ); the rung itself has no backlog entry.
Premise:     Read at file and line on 2026-09-25: the constant, the
             selection, and the assert are as cited. No production
             path calls `unshare` or `clone` with a `CLONE_NEW*`
             flag; the only `unshare` in the tree is the
             interposer's emulation. `docs/architecture.md:32-40`
             stays carefully worded and needs no change.
Approach:    Enter the rung the probe already selects: where the
             namespace legs hold, `unshare` with mount and ID maps,
             then the existing chroot sequence inside; where they do
             not, the chroot rung as today. The banner names the
             achieved rung with what it does not isolate, and
             `.EnteredRung` reports `namespace` where entered. The
             `system.rs` guard stays: the two facts must never be
             conflated again. The README correction of 2026-09-25
             (run enters chroot, both facts carried) stands until
             this lands, then reads the new behavior.
Decision:    Implement behind the probe, not beside it. No flag
             selects the rung; the legs do, the way every other rung
             is selected. A host that loses a leg between probe and
             entry falls back to chroot with the banner naming it,
             never failing a run the chroot rung could carry.
Out of scope: a network or pid namespace (mount and ID maps
             first), changing the probe legs, touching the machine
             tier.
Prove:       on a namespace-capable host `podvm probe` reports
             `namespace` and `podvm system info --format
             '{{.EnteredRung}}'` reports `namespace` after a run
             that isolates a mount the host cannot see; on a denied
             host both report `chroot` as today.

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

---

### T-1407 Anchor userland paths inside the rootfs or refuse

Source:      issues 69 and 70; `crates/podvm-enter/src/userland.rs`;
             `crates/podvm-interpose/src/map.rs`
Category:    enter
Priority:    P0
Effort:      L
Status:      done

Problem:     On the userland rung an absolute guest path resolves on the
             host. A write lands on the host tree. A shell grandchild
             execs host binaries and mixes libc. The package-install
             warning in issue 69 is host mutation through the runtime.
Premise:     The rung states the pass-through in `userland.rs:15`. An
             empty map table rewrites nothing (`map.rs:68`). Nothing in
             `crates/podvm/src` sets `PODVM_MAPS` for a userland
             run (measured by grep: zero hits). `host_env` rewrites
             `LD_LIBRARY_PATH` (`userland.rs:257`) and leaves `PATH` to
             the caller. First-level lookup is image-side through
             `resolve_payload` (`ladder.rs:148`).
Approach:    Set a root-anchoring `PODVM_MAPS` for every userland run,
             minus the never-rewrite paths that T-0706 and T-0707 name.
             Refuse with a named errno where a call cannot be
             virtualized. Add an explicit `--unsafe-host-paths` opt-in
             for callers that want the current pass-through.
Decision:    Refusal beats a silent host write. The table is owned by
             `run` and set before the payload starts. No caller edits
             the table by hand.
Prove:       `cargo test -p podvm-enter` exits 0 with the new unit
             guards. A tracked drive on a chroot-denying fixture reads
             back a rootfs marker through an absolute path, writes no
             host `/tmp` file, and runs `sh -c` compound commands with
             image binaries only.

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

**Open 2026-09-30 (history).** Filed from issues 69 and 70. Read the current
source before implementation. The two issues close together; keep
one acceptance clause per issue.

---

### T-1408 Resolve the loader symlink inside the rootfs

Source:      issue 73; `crates/podvm-enter/src/userland.rs:191`;
             `crates/podvm-enter/src/memfd.rs:49`;
             `crates/podvm-enter/src/abi/search.rs:167`
Category:    enter
Priority:    P1
Effort:      S
Status:      done

Problem:     Debian's `/lib64/ld-linux-x86-64.so.2` is a symlink to an
             absolute target. The loader check follows it on the host,
             where it dangles. Debian and gcc images refuse at 125 on
             the userland rung.
Premise:     `is_executable` uses metadata, which follows symlinks
             (`memfd.rs:49`). `resolve_in` already walks
             rootfs-contained links with a bound (`abi.rs:713`).
             `loader_plan` does not use it.
Approach:    Resolve `interp_guest` through `resolve_in`, bounded and
             staying inside the rootfs, then run `is_executable` on the
             result. Keep the refusal naming the guest path where it
             still fails.
Decision:    The rootfs is the resolution root for every guest path on
             this rung. The host root is never a fallback.
Prove:       `cargo test -p podvm-enter` exits 0. A tracked drive runs
             `public.ecr.aws/debian/debian:bookworm-slim echo hi` and
             `public.ecr.aws/docker/library/gcc:13-bookworm gcc --version`
             on a chroot-denying fixture. Both images join
             `experiments/356-no-chroot-rung.sh`.

**Done 2026-09-30.** `loader_plan` resolves the guest interpreter
through `resolve_in` (bounded, rootfs-contained) before
`is_executable`; the refusal still names the guest path.
`cargo test -p podvm-enter` exits 0 with the absolute-link loader
fixture test. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): debian runs
`sh` on the fixture (clause 4) and `gcc --version` answers on the gcc
image (clause 19), both recorded with their pulled digests.

**Open 2026-09-30 (history).** Filed from issue 73. Read the current source
before implementation.

---

### T-1409 Answer the payload-visible exe on the loader family

Source:      issue 74; `crates/podvm-interpose/src/path_calls.rs:63`;
             `crates/podvm-enter/src/ladder.rs:187`;
             `crates/podvm/src/run/foreground.rs:259`
Category:    enter
Priority:    P1
Effort:      M
Status:      done

Problem:     Entering through the loader leaves `/proc/self/exe`
             pointing at the loader. Go reads `GOROOT` from
             `os.Executable` and fails. The same hazard hits any
             self-locating runtime.
Premise:     The exe emulation answers only where the real call fails
             with `ENOENT` (`lib.rs:1256`). On the userland rung the
             host `/proc` is mounted, so the real `readlink` succeeds
             with the loader path and `PODVM_GUEST_EXE` (`run.rs:798`)
             is never consulted. `ladder.rs:187` documents the
             chroot-only assumption.
Approach:    On loader-family entries answer `/proc/self/exe` and the
             `readlinkat`, `open`, and `stat` shapes from
             `PODVM_GUEST_EXE` even where the real call succeeds.
             State the substitution in the banner.
Decision:    The payload sees its own path, not the loader's. The
             emulation is named, never silent.
Prove:       `cargo test -p podvm-enter` exits 0. A tracked drive runs
             `go version`, `go env GOROOT`, and a hello build on
             `public.ecr.aws/docker/library/golang:1.24.7-bookworm`
             over a chroot-denying fixture with no extra `-e`.

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

**Open 2026-09-30 (history).** Filed from issue 74. Read the current source
before implementation.

---

### T-1410 Exec through the no-chroot family

Source:      issue 80; `crates/podvm/src/exec/command.rs:208`;
             `crates/podvm/src/run/prepare.rs:76`; [T-0505](enter.md)
Category:    enter
Priority:    P1
Effort:      L
Status:      done

Problem:     `run` works on a chroot-denied host but `exec` on the same
             extracted rootfs fails with `EPERM`. `exec` only knows the
             chroot path.
Premise:     `exec` calls `podvm_enter::run` directly (`exec.rs:444`).
             It never calls `decide_entry`, which is the `run` path
             (`run.rs:1078`). T-1317 scoped `exec` out deliberately and
             left the helper one call away.
Approach:    Route `exec` through `decide_entry` and the loader and
             memfd drive. Keep the fresh-entry-shares-filesystem
             contract from T-0505. Refusals name the blocking call.
Decision:    `exec` and `run` share the entry decision. They keep
             separate banners and separate records.
Prove:       `cargo test -p podvm` exits 0. A tracked drive
             extracts `public.ecr.aws/docker/library/alpine:3.20` and
             runs `exec echo hi` with rc 0 on a chroot-denying fixture.

**Done 2026-09-30.** Both exec paths (container and image) route
through `decide_entry` and the loader and memfd drive, keeping the
fresh-entry-shares-filesystem contract with separate banners and
records; refusals name the blocking call. `cargo test -p podvm`
exits 0. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 15
extracts the image and runs image-exec and container-exec at 0 on the
fixture.

**Open 2026-09-30 (history).** Filed from issue 80. Read the current source
before implementation.

---

### T-1411 A detached launcher for the no-chroot family

Source:      issue 72; `crates/podvm/src/run/prepare.rs:77`;
             `crates/podvm/src/lifecycle.rs:213`; [T-1317](enter.md)
Category:    enter
Priority:    P1
Effort:      L
Status:      done

Problem:     `run -d` and `create` refuse where chroot is denied, so
             `start`, `ps`, `logs`, `stop`, `rm`, `wait`, `kill`,
             `restart`, and `cp` have nothing to act on. `ps` prints an
             empty table, which reads as no containers rather than an
             unavailable mode.
Premise:     The refusal is deliberate (`run.rs:1079`). `start_one`
             refuses at the chroot gate (`lifecycle.rs:213`). T-1317
             records `run -d` staying chroot-gated as the decided last
             resort. The pidfd and waitid supervision primitives are
             present.
Approach:    Drive the userland loader family detached: fork, hold the
             pidfd, record the container, keep the foreground path
             unchanged. Where a call genuinely blocks detached
             operation, the refusal names the call and the affected
             verbs.
Decision:    This entry reverses T-1317's last-resort decision in
             writing once the drive passes. Until then the refusal
             stays.
Prove:       `cargo test --workspace` exits 0. A tracked drive runs
             `run -d`, `ps`, `logs`, `stop`, and `rm`, plus `create`
             and `start`, on one container over a chroot-denying
             fixture.

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

**Open 2026-09-30 (history).** Filed from issue 72. Read the current source
before implementation.

---

### T-1412 A missing workdir refuses naming the path

Source:      issue 81; `crates/podvm-enter/src/launch.rs:178`;
             `crates/podvm/src/run/parse.rs:179`
Category:    enter
Priority:    P1
Effort:      S
Status:      done

Problem:     `run -w` to a missing directory is accepted with exit 0.
             The payload starts in the rootfs root. A script that
             depends on the working directory runs in the wrong place
             and reads success.
Premise:     Both rungs discard the `chdir` error: the chroot arm runs
             from `/` saying so, and the no-chroot arm runs from the
             rootfs with the banner naming the start (`lib.rs:1009`).
             The flag parses with no validation (`run.rs:301`).
Approach:    Check the workdir before the fork on both rungs. Refuse
             naming the path and the image. Keep `-w /tmp` printing
             `/tmp` as the passing control.
Decision:    Refusal beats a silent wrong directory. docker creates the
             directory; podvm refuses and names it, and the refusal
             says so.
Prove:       `cargo test -p podvm-enter` exits 0. A tracked drive
             refuses `-w /no/such/dir` with the path named and prints
             `/tmp` for `-w /tmp`.

**Done 2026-09-30.** The workdir is checked before the fork on both
rungs and both exec paths (`check_workdir`: resolved under the
rootfs, refused naming the path and the image, noting that docker
creates the directory while podvm refuses); the child race arrives
as a named step-11 error instead of a silent wrong directory.
`cargo test -p podvm-enter` exits 0. `sh experiments/356-no-chroot-rung.sh`
exits 0 ([the drive](../experiments/results/no-chroot-rung.txt)):
clause 10 refuses the missing directory on the fixture and the sane
host and prints exactly `/tmp` for the control.

**Open 2026-09-30 (history).** Filed from issue 81. Read the current source
before implementation.

---

### T-1413 A script refusal names the file and its shebang

Source:      issue 78; `crates/podvm/src/lifecycle/entry.rs:217`;
             `crates/podvm-enter/src/memfd.rs:147`;
             `crates/podvm-enter/src/ladder.rs:148`
Category:    enter
Priority:    P2
Effort:      S
Status:      done

Problem:     `node:*-alpine` refuses as a `#!` script without naming the
             file or its shebang. The caller cannot tell whether podvm
             picked the entrypoint, a wrapper, or the wrong binary.
Premise:     `eligible` returns the unit variant `RoutePastScript`
             (`memfd.rs:147`), so path and first line are already
             dropped. `payload_bytes` resolves through `resolve_payload`
             (`ladder.rs:148`) but returns bytes only. The refusal
             (`lifecycle.rs:1779`) cannot name what it never received.
Approach:    Thread the resolved path and first line into the
             `no_chroot` message. Longer term, route the interpreter
             inside the rootfs where the map table covers it; that half
             belongs to T-1407.
Decision:    The refusal names the exact payload path and its `#!`
             line. An anonymous refusal is a defect in the message, not
             a separate capability.
Prove:       `cargo test -p podvm-enter` exits 0. A tracked drive shows
             the node refusal naming the resolved file and its `#!`
             line, or the payload runs.

**Done 2026-09-30.** `RoutePastScript` carries the resolved path and
first shebang line into the `no_chroot` refusal, so the node payload
refuses naming the exact file and its `#!` line instead of
anonymously. `cargo test -p podvm-enter` exits 0.
`sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 13
refuses at 126 naming `/hello.sh` and its shebang. Routing the
interpreter inside the rootfs stays with T-1407.

**Open 2026-09-30 (history).** Filed from issue 78. Read the current source
before implementation.

### T-1604 Fix the detached-stdio serial mutex and the 1-of-3 detached start failure

Source:      CI run 36894578988, job 110478236087, 2026-10-01;
             `crates/podvm/tests/detached_stdio.rs` `:138`, `:292`;
             `experiments/340-detached-stdio.sh`
Category:    enter
Priority:    P1
Effort:      M
Status:      partial

Problem:     `cargo test --workspace` exits 101 on `detached_stdio` with
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
Premise:     A serialisation mutex must not propagate one test's panic
             into the next. `unwrap()` on a poisoned lock makes the
             second failure a report of the first, which is why the two
             rows read as one confusing cascade instead of one clear
             failure. The underlying 1-of-3 failure is engine- or
             runner-dependent and identical on both toolchains, so it is
             not a Rust 1.99 regression and not a Batch 3 change.
Approach:    Two fixes, because the cascade hides the real failure. First,
             make the serialisation mutex poison-tolerant at `:138`,
             `:167`, and `:196`, so a failure in one test is reported as
             itself and the next test still runs. Second, find and fix
             why two of three detached starts did not run on a loaded
             runner; drive `experiments/340-detached-stdio.sh` in the
             Linux lane under CI-like conditions before changing the
             assertion, because a test may not depend on a scheduling
             order it cannot control
             (`docs/methodology/authoring.md`, entry fields).
Decision:    The cascade is the reporting defect and the 1-of-3 is the
             product defect. Fix the mutex first so the second is
             measurable, then measure the second. Do not weaken the
             assertion at `:292` to accept 1 of 3: that would record a
             green run over a real failure.
Prove:       `cargo test -p podvm --test detached_stdio` passes in
             the Linux lane, run twice; a deliberately failing test in
             the same binary does not poison its neighbours (the plant
             this task owes); the saved lane log shows all three
             detached starts running;
             `./target/release/podvm-gate` exits 0.

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
(`.github/workflows/gate.yml:230-243`), mirroring the static-build job.
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
---

### T-1647 Stream a payload into a memfd: the 128 MiB read ceiling refuses large runtimes

Source:      issue 90; issue 114 section 3.3 and section 6 item 6;
             `crates/podvm-enter/src/ladder.rs:238`;
             `crates/podvm-enter/src/abi/elf.rs:10`;
             `crates/podvm-enter/src/memfd.rs:108`;
             A-V-04 of PodUtils/podcrates#39
Category:    enter
Priority:    P1
Effort:      M
Status:      open

Problem:     On a chroot-denied host podvm refuses a payload over
             134217728 bytes at 125 before it runs. `node` in node:alpine
             is 149234536 bytes, and a zig toolchain binary is 172641672
             bytes. `doctor` and `probe` do not report the cap, so the
             image is pulled first and the run fails after.
Premise:     Two 128 MiB ceilings read the whole file into memory.
             `Elf::read` (`abi.rs:157`) serves the tier classifier
             (`crates/podvm/src/interpose.rs:184`). `payload_bytes`
             (`ladder.rs:238`) serves `decide_entry`
             (`crates/podvm/src/lifecycle/entry.rs:297`), so the loader
             family refuses too, though it never stages the bytes.
             `memfd::stage` (`memfd.rs:108`) takes one contiguous buffer.
             `memfd::eligible` (`memfd.rs:262`) reads the ELF header and
             the program headers, not the whole file.
Approach:    Classify from a bounded read of the ELF header and its
             tables. Stage the memfd family's payload by a copy from the
             file into the memfd in bounded chunks, and hash during the
             copy where a digest is needed. Bound the bytes held in
             memory, not the file size. Where a size limit must stay,
             report it in `doctor` and `probe` and refuse before the pull.
Decision:    File size is not a refusal reason; memory held is bounded.
             A static runtime that locates its install tree through
             `/proc/self/exe` cannot run from a memfd (issue 90 measured
             zig). That case is out of this entry: T-1648 refuses it by
             name and T-1652 serves it.
Prove:       `cargo test -p podvm-enter` exits 0 with a test that
             classifies and stages a sparse 200 MiB fixture through a
             bounded buffer. `cargo test -p podvm` exits 0.
             `sh experiments/356-no-chroot-rung.sh` exits 0 with a clause
             that runs `node -e 'console.log(1)'` from the node image
             with `--entrypoint node`, past the size check.

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

---

### T-1648 A static Go payload runs into a GOROOT error instead of a named refusal

Source:      issue 91; issue 114 section 3.4;
             `crates/podvm/src/lifecycle/entry.rs:333`;
             `crates/podvm/src/lifecycle/entry.rs:357`;
             `crates/podvm/src/interpose.rs:355`;
             `experiments/356-no-chroot-rung.sh:481`;
             A-V-04 of PodUtils/podcrates#39
Category:    enter
Priority:    P0
Effort:      S
Status:      open

Problem:     On a chroot-denied host `go version` from the golang image
             exits 2 with the payload's own error: "cannot find GOROOT
             directory". The error blames the image. The real reason
             prints only under `--verbose`. The payload sees the host
             tree: `go build -o /tmp/gohello` wrote a 2238818-byte file
             into the host `/tmp`. That is a host write by a container
             payload, with the cause hidden.
Premise:     `decide_entry` sends every static payload to the memfd
             family (`lifecycle.rs:1895`), where no interposer loads. Its
             banner says that absolute paths resolve on the host
             (`lifecycle.rs:1919`), and the quiet default of T-1416 hides
             the banner. `interpose::apply` records the Go decline as a
             banner note (`crates/podvm/src/interpose.rs:355`),
             which the quiet default also hides. Clause 18 of experiment
             356 pins exit 2 naming GOROOT as the boundary
             (`356-no-chroot-rung.sh:481`).
Approach:    On a chroot-denied host, refuse a static payload at 125
             before the exec, unless `--unsafe-host-paths` opts in. That
             is T-1407's flag for this exact host pass-through. The
             refusal says the payload is static, says its absolute paths
             resolve on the host, and names the routes: a host with
             chroot, the opt-in, and the rungs of T-1651 and T-1652 where
             they exist. Under the opt-in, the host-root line prints even
             under `-q`. Every clause of experiment 356 that drives the
             memfd family passes the opt-in. Clause 18 changes from exit
             2 to exit 125.
Decision:    Refusal beats a silent host write, T-1407's rule applied to
             the memfd family. podvm does not inject `GOROOT`: clause 18
             already refused payload-specific magic. The measured
             host-path `GOROOT` recipe stays a caller choice under the
             opt-in.
Prove:       `cargo test -p podvm` exits 0 with a test: a static
             fixture payload refuses at 125 without the opt-in and
             enters the memfd family with it.
             `sh experiments/356-no-chroot-rung.sh` exits 0 with clause 18
             changed: `go version` from the golang image refuses at 125
             naming the static reason, and never exits 2 naming GOROOT.

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

---

### T-1651 An emulate tier on a prefix-preserving, fail-closed qemu-user for static payloads

Source:      issue 94; issue 114 section 3.1 and section 4;
             `crates/podvm/src/tier.rs:26`;
             `crates/podvm-probe/src/report/document.rs:23`;
             `crates/podvm-extract/src/safety.rs:261`; [T-0506](enter.md)
Category:    enter
Priority:    P2
Effort:      XL
Status:      open

Problem:     On a host that denies chroot, user namespaces, mount and
             ptrace, a static payload has no rung that keeps it inside
             the image. qemu-user with `-L` runs one process in the image
             and leaks in three ways. A child it execs runs from the
             host. A path absent from the rootfs is read from the host.
             An absolute link in the image resolves on the host.
Premise:     `--podvm-tier` takes `machine` or `chroot` only
             (`tier.rs:26`). The probe marks an emulator's answers in
             `measured_by` (`report.rs:451`), the T-0506 rule. In qemu,
             `do_execve` hands the raw guest path to the host, and
             `util/path.c` `path()` returns the unprefixed host path
             after an `access()` miss. `openat2(RESOLVE_IN_ROOT)` works
             unprivileged on the errand host's kernel 7.2.2, and podvm
             already wraps it in `podvm_extract::safety`
             (`safety.rs:261`).
Approach:    Add an `emulate` tier. podvm invokes a qemu-user as its own
             process, patched in three places. `do_execve` applies the
             prefix and re-execs the emulator with the same `-L`.
             `path()` fails closed with `ENOENT`. Path resolution, `stat`,
             `access` and `readlink` included, goes through a root
             descriptor with `RESOLVE_IN_ROOT`. The tier answers
             `/proc/self/exe` with the guest path (T-1409's rule). The
             banner names the tier, and the probe marks its answers as
             the emulator's (T-0506).
Decision:    The emulate tier is a rung the caller selects or the ladder
             names, never a hidden fallback. qemu stays a separate GPL
             process, the boundary the machine tier's `qemu-system`
             already keeps. podvm refuses an unpatched qemu, because its
             fallback serves host files. For a native static payload
             T-1652 is the first choice; this tier covers other
             architectures and payloads whose exec or threads that loader
             cannot follow. The cost is an estimate of about 2x native
             (sandhome measured 0.421 s against 0.845 s on one payload).
Prove:       `cargo test -p podvm` exits 0 with tests for the tier
             value and its banner. `sh experiments/356-no-chroot-rung.sh`
             exits 0 with an emulate clause: `go version` from the golang
             image and a shell pipeline exit 0 under the tier, a path
             absent from the rootfs is refused and not read from the
             host, and `podvm probe` under the tier reports
             `measured_by.emulated` true.

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

---

### T-1652 A native-static rung: userland execve, Syscall User Dispatch and openat2(RESOLVE_IN_ROOT)

Source:      issue 109; issue 114 section 1, section 4 and section 6
             item 1; `crates/podvm-probe/src/select.rs:35`;
             `pod_hostprobe::probes`;
             `crates/podvm-enter/src/memfd.rs:108`;
             A-V-04, G-09 of PodUtils/podcrates#39
Category:    enter
Priority:    P2
Effort:      XL
Status:      open

Problem:     On the errand host the supervise rung is out of reach. A
             tracer reads a path argument through `process_vm_readv`,
             which is `EPERM`, or `/proc/<pid>/mem`, which is `EACCES`.
             A static payload therefore has no native rung that keeps it
             inside the image.
Premise:     Syscall User Dispatch (Linux 5.11 and later) delivers
             `SIGSYS` to a handler in the payload's own address space.
             The handler reads the path argument with a plain
             dereference. Issue 109 measured the trap and an in-root open
             on kernel 7.2.2 as uid 966. Dispatch does not survive
             `fork`, `clone` or `execve`: a forked child did not trap. The
             rung list (`select.rs:35`) has no such rung. The probe's
             supervise leg measures `process_vm_readv` (`probes.rs:266`).
             The memfd driver stages a payload (`memfd.rs:108`).
Approach:    Build the rung from four parts. A userland execve loader maps
             the static payload into its own address space and jumps to
             it. `PR_SET_SYSCALL_USER_DISPATCH` with
             `PR_SYS_DISPATCH_EXCLUSIVE_ON` makes the loader's own code
             the trusted region, so every payload syscall dispatches. The
             handler serves path calls through `openat2(rootfd, path,
             RESOLVE_IN_ROOT)`. The handler intercepts `clone` and `fork`
             to arm dispatch in the child, and `execve` to load again.
             Add a probe leg for the `prctl`, a rung word, and
             `measured_by.user_dispatch`. Stage the bytes through T-1647.
Decision:    For a native static payload this rung is the first choice
             over T-1651: native speed, no extra binary, no privilege. A
             refused `prctl` or an older kernel falls back to the emulate
             tier by name, never silently. Dynamic payloads stay on the
             loader family.
Prove:       `cargo test -p podvm-enter` and `cargo test -p podvm-probe`
             exit 0 with tests for the trap, the in-root open and the
             re-arm in a child. `sh experiments/356-no-chroot-rung.sh`
             exits 0 with a native-static clause: `go version` and a
             `go build` of a hello program exit 0 from the golang image,
             the probe reports `measured_by.user_dispatch` true, and with
             the `prctl` refused the run names the emulate tier.

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

---

### T-1653 `-v` volumes on a host with no /dev/fuse and no mount

Source:      issue 130 (130 reposts the hidden issue 104); issue 114
             section 3.6; `crates/podvm/src/parity.rs:221`;
             `crates/podvm-enter/src/userland.rs:313`;
             `crates/podvm-interpose/src/map/rewrite.rs:75`;
             A-V-06, G-07 of PodUtils/podcrates#39
Category:    enter
Priority:    P2
Effort:      L
Status:      open

Problem:     `run -v` is refused on every rung. The errand host has no
             `/dev/fuse` and denies `mount(2)`, so podvm can make neither
             an overlay nor a bind mount. A payload has no way to see a
             host directory at a guest path.
Premise:     The parity table refuses `-v, --volume` with status None:
             "a volume would be a copy pretending to be a mount"
             (`parity.rs:335`). The userland table is one pair, `/` to
             the rootfs (`userland.rs:313`). The interposer takes the
             longest matching prefix (`map.rs:325`), so a pair for a guest
             directory wins over the root pair. T-0703 drove a mapped pair
             end to end and left `-v` out of scope.
Approach:    On the loader family, turn each `-v HOST:GUEST` into a
             `GUEST:HOST` pair in the `PODVM_MAPS` table beside the root
             pair. Apply the T-1407 spelling checks to both sides. Refuse
             `:ro` by name until podvm enforces it. The memfd family has
             no interposer, so it keeps the refusal until T-1652 maps a
             volume prefix to a host directory descriptor. The parity row
             moves from None to Degraded and names the family that
             serves it.
Decision:    A volume on this host is a path rewrite. It is not a copy and
             not an overlay. A write lands in the host directory, as with
             a docker bind mount. The two-root view of issue 114 copies
             executables, which is the copy the parity row refuses, so
             podvm does not use it. A payload execs from a volume only
             where the host mount allows exec, and the banner names a
             noexec source.
Prove:       `cargo test -p podvm-enter` and `cargo test -p podvm`
             exit 0 with table tests for a volume pair and a refused
             spelling. `sh experiments/356-no-chroot-rung.sh` exits 0 with
             a volume clause on the fixture: a host file reads back at
             the guest path, and a guest write lands in the host
             directory. A `:ro` volume gives `EROFS` for a guest write and leaves the
             host file unchanged, once the read-only layer of T-1655 exists.

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

---

### T-1657 A syscall-interpreting tier: the gVisor capability without user namespaces

Source:      issue 169; #193; the owner's ruling of 2026-11-06
Category:    enter
Priority:    P2
Effort:      L
Status:      open

Problem:     gVisor's `runsc` answers a payload's syscalls in user space, so
             the payload does not reach the host kernel for files. On the
             target host `runsc` cannot start: it needs a user namespace,
             mount and ptrace, and the host denies all three. podvm has no
             tier that gives the same view: a fixed kernel identity, a
             private `/proc`, a sized `/tmp` and a placed `/workspace` that
             writes through.
Premise:     T-1652 builds the native-static rung: a userland execve loader
             with Syscall User Dispatch, whose handler serves path calls
             through `openat2(RESOLVE_IN_ROOT)`. T-1651 is the emulate tier
             for foreign and other payloads. `--podvm-tier` takes `machine`
             or `chroot` only (`crates/podvm/src/tier.rs:22`). The rung list
             is `crates/podvm-probe/src/select.rs:35`. The issue names the
             podcrates crate `pod-sud` (wave M7).
Approach:    On top of the T-1652 handler, answer `uname` with a fixed
             string that names the tier. Serve a private `/proc` view for
             the payload's own entries. Serve `/tmp` from a store directory
             with a size cap. Place `/workspace` as a path-map entry whose
             writes reach the host directory, and plant the F14 trap: a
             write that lands in a private overlay fails the test. Resolve
             `..`, absolute links and a missing file in the root or fail
             closed. Where a host grants user namespaces, offer `runsc` as
             an optional backend from the namespace rung.
Decision:    An in-process handler shares the payload's address space. It
             is a compatibility layer, not a boundary, and the banner says
             so, as it does for the interposer. podvm does not copy gVisor's
             network stack. The optional `runsc` backend is not the answer
             for the target host. Build on `pod-sud` once podcrates
             releases it.
Prove:       `cargo test -p podvm-enter` exits 0 with tests for the `uname`
             answer, the `/tmp` cap and fail-closed resolution of `..`, an
             absolute link and a missing file.
             `sh experiments/413-syscall-tier.sh` exits 0 on a
             chroot-denied fixture: a static payload reads `/etc/os-release`
             from the image with no ptrace and no qemu, its write to
             `/workspace` is on the host after the run, `uname -r` names the
             tier, and the banner says the layer is not a boundary.

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

---

### T-1658 A confine layer: Landlock, seccomp and no_new_privs on every rootfs tier

Source:      issue 170; #193; the owner's ruling of 2026-11-06;
             A-V-01 of PodUtils/podcrates#39
Category:    enter
Priority:    P2
Effort:      L
Status:      open

Problem:     bwrap and bailey confine a host process with user and mount
             namespaces, which the target host denies. Landlock needs no
             privilege, but podvm applies no Landlock ruleset, no seccomp
             deny filter and no built environment to a payload. A profile
             cannot say "read these paths, write these, connect to port 443
             only".
Premise:     The probe measures `landlock_create_ruleset(VERSION)`
             (`pod_hostprobe::probes`, body at
             `probes.rs:1331`) and sets `PR_SET_NO_NEW_PRIVS` in a child
             (`probes.rs:1287`). The Landlock and seccomp sweeps are T-0903
             and T-0902. Landlock domains stack, so a payload can add a
             ruleset over the host's, but whether the target host's outer
             policy allows a nested ruleset is not measured. The issue
             names the podcrates crate `pod-confine` (wave M7).
Approach:    Before the exec, set `PR_SET_NO_NEW_PRIVS`, add a Landlock
             ruleset from the profile's read, write and execute lists and
             TCP ports (ABI 4 or later for the network rules), load a
             seccomp deny filter for namespace, mount, ptrace, keyctl, bpf
             and similar calls, build the environment, and set rlimits.
             Report the Landlock ABI and the nested-ruleset answer in
             `podvm probe`. Place `/workspace` through the path map, where
             landrun could not (F30). Ship the bailey policy of issue 170
             as a built-in `errand` profile.
Decision:    Where Landlock is absent, a profile that asks for it refuses by
             name; it never runs unconfined. The default-route trap (F2)
             and the trust-parent trap (F3) are causes to probe and name
             before the start. Build on `pod-confine` once podcrates
             releases it.
Prove:       `cargo test -p podvm-enter` and `cargo test -p podvm-probe`
             exit 0 with tests for the ruleset builder and the probe's ABI
             answer. `sh experiments/414-confine.sh` exits 0 on a host that
             grants Landlock: a profile with `fs.read` and `net.ports =
             [443]` makes `cat /etc/hostname` fail with `EACCES`, a connect
             to port 22 fail with `EACCES` and port 443 pass;
             `/proc/self/status` shows `NoNewPrivs 1` and `Seccomp 2`; with
             Landlock absent the profile refuses by name.

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
T-108). The peer asks for P0. The priority stays P2, because nothing in
podvm is wrong without the layer (the priority rules of `TODO/INDEX.md`).
The owner can change it.

### T-1688 The memfd rung measures whether a memfd executes, names `vm.memfd_noexec`, and refuses before staging

Source:      issue 212; T-1667 (the doctor row shape); T-1003 (the fd exec)
Category:    enter
Priority:    P2
Effort:      M
Status:      done

Problem:     `kernel_takes_memfd()` proves that `memfd_create` answers, not
             that the descriptor executes. Under `vm.memfd_noexec=1` the
             ladder promises the memfd rung and the launch fails at step 5,
             "execveat of the memfd". Under `vm.memfd_noexec=2` the entry
             refuses with a text that does not name the sysctl. `podvm
             doctor` has no row for it.
Premise:     The probe is `crates/podvm-enter/src/memfd.rs:90`: it creates
             and closes a memfd and never executes one. The callers are
             `crates/podvm/src/ladder.rs:202` and
             `crates/podvm/src/lifecycle/entry.rs:336`. The late step is
             `crates/podvm-enter/src/launch.rs:349`. The census row
             `memfd_create+exec` of `pod_hostprobe::probes` runs a shell
             script through `/proc/self/fd/N`, not `execveat` of a static
             payload, and podvm does not read it. Under value 2, `MFD_EXEC`
             is denied too, so "pass `MFD_EXEC`" (the issue) fixes value 1
             only.
Approach:    Add one probe in `podvm-enter`: create a memfd, write a small
             static trampoline, fork, `execveat` it, read the exit status.
             It returns `Exec`, `NoExec` (with the sysctl value when
             `/proc/sys/vm/memfd_noexec` is readable) or `NoMemfd`. The
             doctor row `memfd_exec`, the ladder and the entry check read
             this one probe. The refusal comes before staging and names the
             sysctl, with a fix line for each value: 1 allows `MFD_EXEC`; 2
             needs the sysctl set to 0 or another rung.
Decision:    The doctor row and the refusal read one probe, so they cannot
             disagree. A denial with no readable sysctl reads as "denied,
             cause unknown", not as the sysctl.
Prove:       `cargo test -p podvm-enter -p podvm` exits 0 with tests for the
             pure mapping (measured result and sysctl value to the answer
             and the fix line), and `podvm doctor` prints the row
             `memfd_exec` on the cloud host.

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

**Filed 2026-11-09.** From issue 212 (the owner). The triage of
2026-11-09 confirmed each claim at `2be31b2` and corrected two: the census
row exists in `pod-hostprobe` but tests another exec path, and `MFD_EXEC`
does not help under value 2.
