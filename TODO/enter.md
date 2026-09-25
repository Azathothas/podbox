# enter

`crates/podbox-enter`. `TOOL.md` section 6.5, milestone M3.

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
Status:      partial 2026-09-09

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
Prove:       `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'echo hi' | grep -qx hi`

**Done 2026-09-09.** `crates/podbox-enter/src/lib.rs`. Every buffer the child
touches after the `chroot`, the argv, the environment, the working directory and
the candidate program paths, is built **before** the fork, because between
`clone` and `execve` only async-signal-safe work is permitted and nothing there
may allocate.

⚠ **stdio is inherited rather than re-opened, deliberately**, and that is what
makes `podbox run <image> cmd | consumer` give the consumer the payload's bytes.
The `/dev/null` trap this entry names is real and is why podbox never *closes*
stdio: a child with none opens `/dev/null`, which an extracted rootfs does not
have. ⛔ `--device` and the PTY pair are **not implemented**; they are
[T-0503](#t-0503-probe-devptmx-and-refuse--t-by-name-where-it-is-absent)'s and
M4's, and `-t` is refused by name rather than degraded.

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
`crates/podbox-enter`'s device plan. Risk if wrong is a docker flag
answered as an omission rather than a decision. Prove is one run
mapping a host device and reading it inside, with the refusal arm
(`experiments/355-parity-curated.sh` clause 1, `--device` at 125
naming status None) beside it.

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
Prove:       `env -i "$(command -v podbox)" run --rm public.ecr.aws/docker/library/alpine:3.20 /bin/sh -c 'echo ok' | grep -qx ok`

**Done 2026-09-09.** `podbox_enter::run` resolves the program **after**
`fchdir`, `chroot(".")` and `chdir("/")`, in the process that did them.
`Plan::program_candidates` builds the list before the fork and the child tries
each with `execve`; nothing is resolved in the parent.

⭐ Driven: `podbox run <image> sh -c 'echo onpath'` finds `sh` along the image's
own `PATH` **inside the new root**, and `experiments/300-run.sh` clause 4 asserts
it. A parent-resolved path would have named the outer tree, which is gone.

⚠ The relative-store hazard this entry names is closed from the other side too:
`Store::default_root` is computed before any environment change, and clause 7 of
`300-run.sh` runs with `PODBOX_STORE` set to a path inside `/workspace` and
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
Prove:       `podbox probe --json | jq -e 'has("ptmx")' && { podbox run --rm -t public.ecr.aws/docker/library/alpine:3.20 true || podbox run --rm -t public.ecr.aws/docker/library/alpine:3.20 true 2>&1 | grep -q 'ptmx'; }`

**Done 2026-09-22.** Both verbs ask one shared predicate
(`podbox-probe/src/probes.rs` `ptmx_usable`): only the OPEN row with `Ok`
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
question is now one command on the machine that matters: `podbox probe --json |
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
$ fixture podbox run --rm -t ALPINE true; echo $?
podbox run: -t was asked for and /dev/ptmx is not usable on this machine [...]
podbox run: chroot(2) is denied on this machine as well [...]
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
Prove:       `ln -sfn "$(podbox inspect --format '{{.RootfsPath}}' public.ecr.aws/docker/library/alpine:3.20)" /tmp/rootlink && ! podbox run --rm --rootfs /tmp/rootlink public.ecr.aws/docker/library/alpine:3.20 true`

**Done 2026-09-09.** `podbox_enter::RootDir::open` `lstat`s with
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

Problem:     `docker exec` enters the container's namespaces. podbox has none to
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
             grandchildren that reparent are outside podbox's reach.
Decision:    A fresh chroot rather than refusing `exec`. `exec` is load-bearing
             for M4's lifecycle test and for every agent workflow, and the
             filesystem-only sharing is enough for the overwhelming majority of
             uses provided it is stated.
Prove:       `./experiments/320-cli-contract.sh` clause 4 and `./experiments/300-run.sh` clause 8: exec's stdout is the payload's, the banner says it is a fresh chroot, and `inspect --format '{{.Exec.Shares}}'` reads `filesystem`

**Done, 2026-09-09.** `crates/podbox-cli/src/exec.rs`, `Exec.Mode` and
`Exec.Shares` on `inspect`, and clause 8 of `experiments/300-run.sh`.

⛔ **The `Prove` above was rewritten, and the original is here because the
rewrite is the finding.** It read

```
podbox run -d --name execprobe alpine:latest sleep 30 && podbox exec execprobe ... && podbox rm -f execprobe
```

and every verb in it except `exec` is M4's: `run -d`, `--name` and `rm` need
container state, which [T-1105](milestones.md) creates. ⛔ The work order puts
this entry BEFORE M4 because M4 needs it, so a `Prove` that needs M4 could never
have run in the order it was written for. The mechanism does not depend on that
half: `exec` re-enters a rootfs, and today a rootfs is named by an image
reference and at M4 by a container name that resolves to the same directory.

⭐ **The degradation is stated in three places and they cannot disagree**,
because all three read one pair of constants in `crates/podbox-cli/src/images.rs`:
the banner on every `exec`, `podbox inspect --format '{{.Exec.Mode}}'` for a
program, and the `Degraded` row for the verb in [T-0801](cli.md)'s parity table.
A unit test asserts the banner contains what `inspect` reports, and clause 8
asserts it on the shipped binary.

⛔ **`exec` never pulls and never extracts.** A verb that creates the thing it
claims to attach to is exactly the lie `TOOL.md` section 4.1 forbids, so an
image the store does not hold, or holds and has never extracted, is a refusal
naming `podbox run`. ⚠ It also has no default command: an image's `Cmd` is what
`run` starts, and re-running it from `exec` is a process nobody asked for.

⛔ **`--format` had to learn a dotted name for this.** `{{.Exec.Shares}}` was
refused as an unsupported traversal, because the walker rejected any `.` inside
a name. A dotted name is now ONE registered name, so an unregistered one is
still refused rather than resolving half of itself and rendering a blank.

---

### T-0506 A foreign-architecture container, and never a rung measured by the emulator

Source:      Found on 2026-09-09 by running `podbox probe` under `qemu-aarch64`, while closing [T-0911](deps.md)
Category:    enter
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     ⛔ **A probe run under `qemu-user` measures the emulator, and
             printing its rung as the machine's is the exact lie podbox exists
             to refuse.** Measured on 2026-09-09,
             `experiments/results/multiarch.txt` clause 4: `qemu-aarch64`
             answers `EINVAL` to `clone(CLONE_NEWNS)`, `ENOSYS` to `fsmount` and
             `move_mount`, and reports `Seccomp: 0` and `NoNewPrivs: 0` however
             the host is confined. Every one of those is a statement about QEMU.
             ⚠ It was harmless while podbox ran only where it was built. The
             multiarch work makes it reachable by an ordinary user: `podbox run`
             on a `linux/arm64` image on an amd64 host executes its payload,
             and any `podbox probe` inside it, under precisely this emulator.
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
                2026-09-09, and ⛔ **disclose the write**: podbox put a file in
                the payload's tree, which is a thing the payload can see, and
                the honesty rules do not have an exception for convenience.
             4. Where no interpreter is registered at all, **refuse** with exit
                125, naming the host platform, the image platform, whether
                `binfmt_misc` is mounted, and what would fix it. ⛔ Never exec
                into a bare `Exec format error`.
             5. ⛔ **Mark the probe answer as the emulator's** wherever podbox
                runs under one, in the evidence block and in `--json`, and
                refuse to write it into `$store/probe.json` under a key that
                does not name the emulator. [T-0111](probe.md)'s cache key is
                about confinement; this adds the interpreter.
Decision:    Report and refuse before falling back. A runtime that guesses which
             architecture a payload wanted has the same problem as one that
             guesses a rung. ⚠ The copy-in is deliberate and disclosed rather
             than refused, because refusing it would make podbox useless on
             every machine whose binfmt registration predates `F`, which is most
             of them.
Prove:       `podbox run --rm --platform linux/arm64 <image> /bin/true` succeeds where an interpreter is registered and exits 125 naming both platforms where none is, and `podbox probe --json` run inside it carries the interpreter in `measured_by`

**Done 2026-09-09.** `crates/podbox-enter/src/binfmt.rs` and the `run` verb,
driven by `experiments/300-run.sh` clause 5 and
`experiments/260-multiarch.sh` clause 5.

| | |
| --- | --- |
| `podbox run --platform linux/arm64 <image> /bin/uname -m`, on an amd64 host | **`aarch64`**, exit 0 |
| what podbox said on stderr | it named the image's platform, the host's, and the interpreter, and that the binfmt `F` flag makes it reachable inside the chroot |
| `--platform linux/riscv64`, nothing registered | exit **125**, naming the ELF machine it looked for, how many registrations are enabled, and that installing `qemu-user-static` is what docker and podman rely on too |

⛔ **podbox reads the registrations and never writes one.** Registering an
interpreter is a machine-wide change and podbox is not the machine's owner.
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
of the ELF header, so podbox answers "is there an interpreter for THIS image"
rather than "is there any interpreter at all". A magic too short to reach byte
18 does not select on the architecture, and podbox treats that as no match
rather than as a match.

**Point 5 done, 2026-09-09.** `crates/podbox-probe/src/interp.rs`,
`measured_by` in `report::document`, a line in the banner, and `interpreter` as
the eighth component of the cache key. Driven by
`experiments/260-multiarch.sh` clause 6.

| | |
| --- | --- |
| `measured_by.emulated`, aarch64 podbox under qemu on this amd64 host | **`true`** |
| `measured_by.interpreter` and `cache_key.interpreter` | both `/usr/bin/qemu-aarch64-static` |
| what the banner says, beside the rung | `⛔ MEASURED BY /usr/bin/qemu-aarch64-static, NOT BY THIS MACHINE` |
| that answer offered to a native podbox sharing the store | refused: `the instrument changed` |

⭐ **What can be detected was measured rather than assumed**, by running the
arm64 rootfs's own tools under `qemu-aarch64-static`:

| asked | under the emulator | usable |
| --- | --- | --- |
| `/proc/cpuinfo` | `model name: ARMv8 Processor rev 0 (v8l)` | ⛔ no, emulated |
| `readlink /proc/self/exe` | the guest binary's own path | ⛔ no, emulated |
| `/proc/self/maps` | guest mappings only; qemu's own are filtered out | ⛔ no |
| `ls /proc/sys/fs/binfmt_misc` | the **host's** registrations | ⭐ yes |

⛔ **So the claim is conditional and says so.** podbox cannot prove it is
running natively; what it establishes is that this machine routes binaries of
podbox's **own** ELF machine through an interpreter. The other state is
`NoEvidence`, it carries the list of what was checked, and it is never printed
as "measured natively": `TODO/probe.md` T-0109 rule 1 forbids collapsing "no
evidence" into an answer.

⚠ **Two routes in, and only one leaves a `binfmt_misc` trace.** An explicit
`qemu-aarch64-static ./podbox` involves no registration at all, so the
environment is checked for the variables qemu-user reads. That is weaker
evidence and is reported as its own sentence rather than merged with the first.

⛔ **The reader moved to `podbox-probe` rather than being copied.**
`podbox-enter::binfmt` asks "can this machine execute a FOREIGN image"; this
asks the mirror question about podbox's OWN architecture. One parser answers
both, and `podbox-enter` re-exports it.

⚠ **A defect this found in `experiments/260-multiarch.sh`**: clause 4's reading
depended on whatever binfmt state the machine happened to be in, because
`podbox probe` re-execs itself once per probe and an explicitly-invoked aarch64
podbox cannot start its own children without a registration. The rung read
`namespace` on a machine that had one and `unsupported` on a machine that did
not, and the committed reading recorded only the first. The registration is now
made once, before clause 4, and the trap removes it.

---

### T-1317 A chroot-denied host gets an up-front refusal naming chroot, not a 125 after the work

Source:      issue 12, client beta testing 2026-09-22 (every rung dies at
             `chroot(".")` EPERM while probe selects `interpose`);
             `crates/podbox-enter/src/lib.rs` (fchdir/chroot/chdir entry
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
Prove:       `podbox run` on a chroot-denied host (or a fixture that
             denies chroot) exits 125 naming `chroot(2)` denied with no
             rootfs mutation, and `podbox probe` maps its chroot leg to
             the same refusal; on a chroot-capable host every rung runs
             as before. Close issue 12 with a comment showing both
             outputs and the probe-gated refusal as the guard that stops
             recurrence.

**Done 2026-09-23.** Up-front refusal, as decided. One helper,
`lifecycle::ensure_chroot_usable` (`crates/podbox-cli/src/lifecycle.rs`),
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
C launcher denying `chroot(2)` with EPERM, then execing podbox),
`public.ecr.aws/docker/library/alpine:3.20`:

```
$ podbox probe            # unfiltered:  chroot=ok
$ fixture podbox probe    # filtered:    chroot=EPERM
$ fixture podbox run --rm IMG echo hi; echo $?
podbox run: chroot(2) is denied on this machine, so the chroot tier
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
`crates/podbox-enter` entry sequence, the `lifecycle` gate, and the
T-1003 ladder. Risk if wrong is weaker isolation stated as equal.
Prove is `run --rm alpine echo hi` at exit 0 with the banner naming
the rung on the chroot-denying fixture, beside the current refusal
test as the strict arm, with no rootfs mutation on the refusal path.

**Done, 2026-09-25.** Both no-chroot families run, decided in
`lifecycle::decide_entry` before any fixup mutates the rootfs:
dynamic payloads through the image's own loader with the image's
library directories and the interposer preloaded by host path
(`podbox-enter/src/userland.rs` `loader_plan`, `host_env`), static
payloads from a staged memfd exactly as the ladder stages it
(`Family::Memfd`). `Rung::Userland` orders after chroot and never
claims isolation: the banner, `PODBOX_ACTIVE_MODE=userland` and
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
$ fixture podbox run --rm DEBIAN sh -c 'echo $PODBOX_ACTIVE_MODE'; echo $?
entering without chroot on the loader family: [...] runs [...]/usr/bin/sh [...]
userland
0
$ fixture podbox run --rm ALPINE sh -c 'echo $PODBOX_ACTIVE_MODE'; echo $?
entering without chroot on the loader family: [...] runs [...]/bin/busybox
[...] the invoked name "sh" rides as the applet [...]
userland
0
$ fixture podbox run --rm static-hello:1 /hello; echo $?
entering without chroot on the memfd family: [...]
static-hi
0
$ fixture podbox run --rm --strict ALPINE true; echo $?
podbox run: --strict, and this run is degraded in 6 way(s). [...]
125
$ fixture podbox run --rm ALPINE /nonexistent-probe-target; echo $?
podbox run: chroot(2) is denied [...] no no-chroot family runs [...]
125         (rootfs byte-identical after)
```

Forced `PODBOX_MODE=memfd` enters the static hello at 0, refuses
over dynamic alpine naming the unforced family at 125, and forced
`fuse` refuses naming the force at 125. Unit guards:
`userland.rs` (loader plan, lib dirs, host env, invocation name,
applet argv), `lifecycle.rs` (rung decision), `ladder.rs` (forced
no-chroot drives and refusals), `system.rs` (`.EnteredRung`
`userland` where chroot is denied); lane `cargo test` green over
the three crates beside the drive (podbox-cli 143, podbox-enter 64,
podbox-probe 97 passed, 0 failed).


---

### T-1339 Enter the namespace rung where the probe permits it

Source:      issue 59, beta.7 drive 2026-09-25 (`README.md:56-62`
             promised it, `run` never did); `crates/podbox-enter/src/lib.rs:47-61`
             (`ENTERED_RUNG` is `Chroot` on every machine),
             `crates/podbox-probe/src/select.rs:138-155` (returns
             `Rung::Namespace`), `crates/podbox-cli/src/system.rs:428-431`
             (asserts the two differ)
Category:    enter
Priority:    P1
Effort:      L
Status:      open

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
Prove:       on a namespace-capable host `podbox probe` reports
             `namespace` and `podbox system info --format
             '{{.EnteredRung}}'` reports `namespace` after a run
             that isolates a mount the host cannot see; on a denied
             host both report `chroot` as today.
