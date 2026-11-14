Area: machine.

Record semantics: [task rules](RULES.md#5-entry-closure).


The machine tier. The chroot tier runs a payload on the host kernel; this tier runs one on
a kernel of its own, and it answers to the same verbs, flags and exit codes.

⛔ **It is a mode of podvm and not a second product.** One binary, one parity
table, one set of exit codes. A second product means two parity tables, and a
flag that means one thing in one and another thing in the other is the defect
[cli.md](cli.md) T-0803 already rules on for the `docker` name.

⭐ **Read this first, because it is the finding that shapes every entry here.**
Two restricted runtimes were studied and **they do not have the same walls**:

| | the runtime the machine tier was specified against | the runtime podvm was specified against |
| --- | --- | --- |
| `clone`/`clone3` with `NEWUSER`, `NEWNS`, `NEWPID`, `NEWNET` | work | `EPERM` |
| the new mount API, `fsopen` and `fsmount` | work | refused before entry |
| `move_mount` | `EPERM` | `EPERM` |
| `chroot(2)` | available | ⭐ available, and it is the whole solution there |
| `/dev/kvm` | absent and uncreatable | absent |
| `/dev/ptmx` | not stated | absent, and no `devpts` can be mounted |

⛔ **So neither set of walls may be compiled in.** A mode that assumes the
namespace gap is open is wrong on the runtime that refuses `unshare`, and a mode
that assumes it is closed gives up a tier it could have offered. Every entry
below probes. `AGENTS.md`'s routing row already says this about `TOOL.md`'s
floor, and the machine tier is where it costs the most to forget.

Sources, none of which is in the corpus. Each is read at the URL and the licence
determination is in [reference-map.md](reference-map.md):

- `https://github.com/talaria0101/vm-research`, and its podvm-spec document,
  which is the specification these entries derive from;
- `https://github.com/talaria0101/sandbox-insights`, the four walls and the mode
  ladder;
- `https://github.com/talaria0101/nix-experiment`, a real payload driven to
  completion on the second runtime, which is the acceptance
  [T-1111](milestones.md) drives through podvm.

# T-1301: The machine tier is probed leg by leg, and a present file is not a working one

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 0 and 3; [probe.md](probe.md) T-0103
**Category:** machine
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

podvm reports a rung and never claims a stronger one than it
achieved. The machine tier has no probe at all, so the machine tier today
could only assert it. ⛔ **Every way to get this wrong produces a
false report rather than an error**, which is the class
[`../docs/methodology/reviews.md`](../docs/methodology/reviews.md)
exists for.

## Premise

⭐ **podvm already holds the discipline and two of the legs.**
[T-0103](probe.md) probes creation and attachment separately and is
done; [T-0102](probe.md) separates a filtered syscall from an
executed one with a bogus argument; [T-0109](probe.md) keeps
"could not run" distinct from "denied". The machine tier adds legs
and reuses all three rules.
⚠ The specification measures `/dev/kvm` ABSENT and uncreatable on
its own target, and the emulator therefore runs under TCG. ⛔ A
boundary under TCG is the emulator process and not hardware, so the
banner may never say hardware isolation.

## Approach

Add a `machine` group to `podvm probe`, with one leg per fact and
a verdict per leg:
1. an emulator binary that runs and answers its own version;
2. `/dev/kvm` OPENED, not stat-ed. A node that exists and returns
   `EACCES` on open is not acceleration.
3. `RLIMIT_FSIZE`, read with `getrlimit`, because it bounds the
   memory image and a guest over it dies with `SIGXFSZ`;
4. `/dev/net/tun`, opened the same way;
5. the writable space for an image, blocks and inodes both, per
   [RULES.md](RULES.md) section 8;
6. ⭐ **ask the emulator which accelerator it will actually use**,
   rather than deciding from leg 2. `qemu-system-x86_64 -accel
   help` lists what this build supports, and an accelerator a
   build was compiled without is as absent as a missing node.
   ⚠ **`references/cubic-vm__cubic` reached this independently**,
   which is what makes it worth copying: its pull request #538 is
   `feat: ask qemu which accelerator works on the host` and its
   issue #6 is `Only use KVM if available`. That project has no
   connection to this one and no restricted host to defend
   against, and it still ended up probing rather than assuming.
⛔ Never infer a leg from another leg. The specification records a
runtime where the host drivers are listed in `/proc/misc` and the
device nodes are absent and uncreatable, so the driver's presence
says nothing about the caller's access.

## Decision

The tier is refused when any leg is missing, and the refusal names
the leg. ⛔ Not a reduced machine tier: the ladder in
[`../docs/architecture.md`](../docs/architecture.md) says a weaker
mode never satisfies a stronger request, and a machine tier with no
acceleration and no network is two different products depending on
which leg failed.

## Prove

`experiments/357-tcg-profile.sh` exits 0 (report in
`experiments/results/tcg-profile.txt`): `tiers.machine.refusal`
null with `profile` `tcg` on a kvm-less host whose emulator
lists tcg, the TCG holds message at 125 under both names,
refusal naming the emulator with qemu hidden, and 146
re-driven with the same binary booting to VMR-GUEST-READY.

## Done

**Done 2026-09-21.** Six legs in `Group::Machine`
(`pod_hostprobe::probes`, `MACHINE_LEGS`), one verdict per
leg, read back through `pod_hostprobe::machine`, which refuses
the tier naming every missing leg. `report::document` carries
`tiers.machine.legs` with a null verdict where a row is absent, so a
document written before the legs existed cannot read as a measured tier;
the stderr evidence lists the legs with the same refusal. The Prove exits
0 against a lane-built binary: 6 legs, no null verdict.

| leg | verdict on the lane |
| --- | --- |
| `qemu-system-x86_64 --version` | skip, no emulator on PATH (ENOENT) |
| `open(/dev/kvm, O_RDWR)` | denied ENOENT |
| `prlimit(RLIMIT_FSIZE)` | ok, `cur=infinity max=infinity` |
| `open(/dev/net/tun, O_RDWR)` | denied ENOENT |
| `image space (statfs .)` | ok, blocks and inodes free named |
| `qemu-system-x86_64 -accel help` | skip, no emulator on PATH (ENOENT) |

The review caught a wrong constant before it shipped: `RLIMIT_FSIZE` was
written as 7, which is `RLIMIT_NOFILE`, and the leg reported the
descriptor ceiling as a file size. The kernel's own headers settle it
(`RLIMIT_FSIZE` 1, `RLIMIT_NOFILE` 7, measured in the lane), the constant
is 1, and the re-drive above ran on the fixed code. Every non-ok leg on
that machine carries its errno, so T-0101's invariant holds.

**Partial, 2026-09-25.** Issue 35 reopens this entry: six legs and
a refuse-where-any-missing rule ship, so on hosts without
`/dev/kvm` or `/dev/net/tun` the whole machine tier is refused even
where QEMU TCG is present. Split the legs into required (emulator
present, `tcg` in the accelerator list, space, fsize), accelerated
(`kvm`) and networked (`tun`): TCG with user-mode networking runs
with a banner stating no hardware isolation and no tun, and refusal
stays only where no accelerator at all or no space holds. The
Premise already states a TCG boundary is the emulator process, not
hardware, and the reference refuses TCG as a different product, so
the banner must never claim hardware isolation under TCG. Measured
on the lane 2026-09-25
(`experiments/results/triage-353.txt` clauses `35-tier-machine`
and `35-podvm`): refusal names the missing legs at 125 on a host
whose only gap is the emulator and the two nodes; the `podvm`
subverb spelling is refused by name per T-1302 (the name is
`argv[0]`, not a verb). Fix area is `MACHINE_LEGS` in
`pod_hostprobe::probes` with the `machine.rs`
assessor. Risk if wrong is emulated execution claimed as hardware
isolated. Prove is `tiers.machine.refusal` null for the TCG
profile on a kvm-less but tcg-capable host, with the initramfs
boot and its ready marker through TCG.

**Done, 2026-09-25.** The legs split into required, accelerated and
networked without adding or renaming a row: `machine.rs` establishes
`full` where every leg holds and the emulator lists `kvm`, `tcg`
where the emulator, its `tcg` accelerator, the file-size bound and
image space hold without kvm or tun, and no profile elsewhere. The
accelerator list is read word-wise from the accel leg's own reason,
and kvm needs the list entry and the opened node together: neither
half infers the other. `tiers.machine` carries `profile`
(`full`, `tcg`, null) beside `refusal`, which is null where a
profile holds and otherwise names the legs blocking every profile
with the kvm leg beside them; the evidence prints the TCG boundary
(the emulator process, never hardware isolation) and the tun shape
(real tun, or user-mode networking) on every TCG run. `enter_machine`
prints the TCG holds message at 125 under both names, still 125
where the tier holds without a guest driver, and the unchanged
refusal where no profile holds. Three designs were weighed before
testing (a separate tcg_refusal key; a required-only weakening; the
profile split): the first fails the Prove as written (it demands
`refusal` null), the second drops the accelerated and networked
distinction the entry requires, so the split shipped. Prove is
`experiments/357-tcg-profile.sh` (exit 0,
`experiments/results/tcg-profile.txt`) on the lane with qemu
7.2.22 installed, kvm and tun absent:

```
$ podvm probe --json | jq -e '.tiers.machine.refusal == null'
0
$ jq -r '.tiers.machine.profile' probe.json
tcg
$ podvm run --podvm-tier=machine never-pulled-357:tag /bin/true; echo $?
podvm run: the machine tier runs TCG on this machine: the emulator
process is the boundary, not hardware isolation. /dev/net/tun is
missing, so guests use user-mode networking. [...]
125
$ podvm run never-pulled-357:tag /bin/true; echo $?
podvm: invoked as `podvm`. [...]
podvm run: the machine tier runs TCG on this machine: [...]
125
$ PATH=<empty> PODVM_STORE=<fresh> podvm run --podvm-tier=machine [...]; echo $?
podvm run: machine tier refused: qemu-system-x86_64 --version=skip:
no qemu-system-x86_64 on PATH [...] open(/dev/kvm, O_RDWR)=ENOENT [...]
125
```

and 146 re-driven inside with the same binary boots to
`VMR-GUEST-READY` under TCG (clause 5 green). Unit guards: the
`machine.rs` profile matrix (full, kvm-denied, tun-denied,
accel-without-tcg, node-without-listing, skipped emulator, absent
row, word-wise list parsing), the report document and evidence
tests, `tier.rs` still 125 on any machine; lane suites green beside
the drive (podvm 143, podvm-probe 101 passed, 0 failed). Two
limits ride named, not silent: 145 and 148 assert the refusal text
and run on qemu-less lanes, so a qemu host would need their TCG
arms; and a kvm node opening under an emulator that does not list
kvm is TCG, never full.

# T-1302: One binary, one parity table, and a VM-only flag that cannot collide

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document; [cli.md](cli.md) T-0803, T-0808
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`podvm` has to feel like `podman` and `docker`, because its audience
is automated and an agent reaches for the verbs it knows. ⛔ A
second binary is a second parity table, and
[cli.md](cli.md) T-0808 already drives 141 rows through the shipped
binary because a flag accepted and unlisted is invisible from the
table's side.

## Premise

podvm already answers to more than one name:
[cli.md](cli.md) T-0803 rules that podvm refuses the `docker` name
where a working daemon is reachable unless a flag says otherwise.
The mechanism for a third name therefore exists and is ruled on.

## Approach

`podvm` is `podvm` invoked under that name, or `podvm` with the
machine tier selected explicitly. The verbs do not change: `run`,
`exec`, `ps`, `rm`, `images`, `logs`, `stop`.
⛔ **A flag that only the machine tier has carries a prefix that
neither `docker` nor `podman` uses**, so a flag cannot mean one
thing here and another there. A flag both tiers share keeps
docker's spelling and docker's meaning.
⚠ `--memory` is the trap in this shape and it is worth naming: the
machine tier can enforce it, and the chroot tier cannot. A flag
that is enforced in one tier and accepted in the other is the
silent degradation the honesty rules forbid, so the tier that
cannot enforce it says so.

## Decision

⭐ **Taken on 2026-09-12: BOTH, and the flag is the primitive.**
`--podvm-tier=machine` selects the tier, and `podvm` is a third
name in `crates/podvm/src/names.rs`'s `ALIASES` whose only
effect is to change that flag's DEFAULT. There is one
implementation and one parity table; the table lists the flag, and
the name is an entry point to it rather than a second code path,
which is what [cli.md](cli.md) T-0808 needs.
⛔ **The collision rule, because `podvm --podvm-tier=chroot` is
otherwise undefined: the explicit flag wins over `argv[0]`, and
podvm states the tier it selected whenever the two disagree.**
The tier is not cosmetic. This entry's own `Approach` names
`--memory` as enforced by the machine tier and not by the chroot
tier, so a caller who cannot tell which tier ran cannot tell
whether a limit was honoured.
⚠ **Neither shape alone survives this entry's own `Prove`**, which
asks that `podvm --help` AND `podvm --help` list one set of verbs
and that the parity experiment drive every row **from the flag**.
A name with no flag has nothing to drive; a flag with no name
leaves `podvm --help` unanswerable. (Superseded at the rename, 2026-11-06.)
⚠ **T-0803's refusal rule does not extend here.** That ruling is
about taking another tool's name where that tool works: `docker` is
refused where a daemon answers. `podvm` is podvm's own name, so
there is no foreign daemon to check for and no refusal to make.
The banner obligation is the tier, not the name.
⚠ **The prefix is fixed by this decision** so an implementer does
not reopen it: `--podvm-` is a prefix neither docker nor podman
uses, which is the rule the `Approach` above already states.
⛔ **One thing IS settled, and it is about the escape hatch rather
than the tier flag.** A machine tier needs a way to pass an
argument straight to the emulator, and
`references/cubic-vm__cubic` ships the defect that shape invites:
its open issue #448 records that the passthrough **splits its
string on a single space**, so any argument whose value contains a
space is torn into pieces and the emulator rejects the result. The
report names the splitting line in that project's own source, and
the user-visible effect is that quoting on the command line does
not survive.
⭐ **podvm takes the repeatable-flag shape**: one token per
occurrence, repeated as many times as needed, and no splitting of
anything. ⚠ **Rejected:** splitting on whitespace with a shell word
splitter, which is what that report suggests as its first option.
It is friendlier to type and it puts a quoting parser between the
caller and the emulator, so a mis-parse becomes podvm's bug in a
place podvm cannot test exhaustively. ⛔ docker and podman both
take the repeatable form for this class of flag, and
[cli.md](cli.md) is the parity rule that settles ties.

## Prove

`podvm --help` and `podvm --help` list one set of verbs, and `./experiments/145-podvm-parity.sh` drives every row from the flag rather than from the table

## Done

**Done 2026-09-21.** One flag and one name, in one implementation
(`crates/podvm/src/tier.rs`): `--podvm-tier=machine|chroot` on `run`
and `exec` (which also serves `create`), and `podvm` as a third entry in
`ALIASES` whose only effect is to default that flag to machine. The
explicit flag wins over `argv[0]`, and `podvm --podvm-tier=chroot` states
the override and runs the chroot tier. The machine tier assesses T-1301's
legs before anything is pulled and refuses naming every missing one;
where it holds, podvm says the guest driver arrives with T-1303/T-1304
rather than running something else. `--podvm-qemu-arg` is repeatable,
one token per occurrence, never split, machine-tier-only and refused
elsewhere. `podvm --help` and `podvm --help` print one text. Superseded at the rename of 2026-11-06: `podvm` is now the binary's own name, the argv0 default is gone, and only `--podvm-tier` selects the tier; `podvm-parity` drives the help parity over `podvm`, `docker` and `podman`.
`experiments/145-podvm-parity.sh` exits 0 on a lane-built binary: 19
driven, 0 mismatches (`experiments/results/podvm-parity.txt`).

Decisions taken inside the ruled shape: the flag takes exactly
`machine|chroot`, because those are the two tiers this binary knows; the
qemu arguments are parsed, tier-gated and carried for T-1303's driver, and
the collection contract (no splitting) is pinned by a parser unit test
rather than by a drive. The 325 driver needed no change: a bare new flag
reaches a message without a refusal marker, which clause 1 counts as
driven.

# T-1303: The image is a rootfs directory, and an initramfs with no console is a silent machine

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 1 and 2
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

podvm's image path ends in an extracted rootfs on disk
([extract.md](extract.md)), and the machine tier needs that same
rootfs wrapped so a kernel can start in it. The wrapping is where
the specification records the expensive failures.

## Premise

⭐ **Three measured traps, each of which produces a machine that
looks broken rather than one that reports a problem.**
1. A guest with no console device is a PID 1 that runs and says
   nothing. The specification records an initramfs that shipped an
   empty `/dev`, and the fix is a second raw cpio archive appended
   after the base one, carrying the console node.
2. A module that the guest needs may not load with the tool the
   guest has: busybox `insmod` fails on an unresolved dependency
   and says nothing, so the module tree ships and `modprobe` loads
   it.
3. The machine profile decides whether userspace output arrives at
   all. One profile delivers kernel messages to the serial port and
   drops userspace writes to it.
⚠ Every one of these is a reading of somebody else's measurement on
somebody else's host. ⛔ Reproduce the MECHANISM here and expect the
numbers to differ, per `AGENTS.md`'s "verify, do not accept".

## Approach

Reuse [extract.md](extract.md)'s output as the image. Build the
initramfs from it with the console node appended, and keep the
assembly in one script under `experiments/` so the measurement
ships with it.
⛔ The ownership rule does not change: extraction stays
ownership-neutral and the intended metadata stays in the sidecar,
because the wall that forces it is the kernel's mapping check and
not a property of the destination.

## Decision

A rootfs directory, not a disk image. ⚠ A disk image needs a
filesystem the host can make, and the runtimes studied refuse
`mount`, so the host cannot populate one it cannot attach.

## Prove

`./experiments/146-podvm-initramfs.sh` asserts the guest prints its ready marker and that the console node is present in the unpacked archive

## Done

**Done 2026-09-21.** `experiments/146-podvm-initramfs.sh` exits 0 on a
lane-built binary: the pinned Alpine image pulls and extracts through
podvm, the rootfs archives as newc cpio, an appended raw archive carries
`dev/`, `dev/console` 5:1 and the `/init` override, and the pinned kernel
boots the assembly under TCG to `VMR-GUEST-READY`
(`experiments/results/podvm-initramfs.txt`).

| clause | verdict on the lane |
| --- | --- |
| pull + extract `alpine@sha256:3e9b4b…` | ok, guest-executable `/bin/sh` |
| base archive | ok, 8,365,568 bytes |
| appended archive | ok, 8,366,168 bytes assembled |
| console node | ok, `dev/console` in the appended archive |
| boot `vmlinuz-virt` (sha256 `6b58e5d7…`), qemu 7.2.22 TCG | ok, marker printed; qemu rc 124, the halt the spec records |

Three findings on the way, each in the script now. The reference pin for
the kernel is stale: its tree names 6.12.94 and the mirror serves
6.12.110, so the pin here is measured, not inherited. A host `-x` test
lies about absolute symlinks: the image's `sh` points at `/bin/busybox`,
which the guest resolves inside the rootfs and the host resolves outside
it, so the check resolves the link as the guest does. `cpio -t` stops at
the base archive's TRAILER while the kernel's unpacker keeps going, so
the node is listed in the appended archive and the boot proves the full
assembly. The newc writer is this script's own: no host mknod, no
privilege.

# T-1304: The exec protocol is a serial pair with a nonce, and every wait is bounded

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 4
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

`podvm exec` hands a command to a process. The machine tier has no
process to hand it to: the command crosses a serial line into a
shell that may not be listening yet.

## Premise

⭐ **The readiness handshake is the part that cannot be skipped.**
The specification records that feeding the console before it is up
is lossy, so the driver retries a newline plus a nonce until the
guest echoes that nonce back. ⛔ A driver that assumes the guest is
ready after a fixed sleep is the unbounded wait
[RULES.md](RULES.md) section 8 forbids, wearing a timer.
⚠ Two more, both measured there: the parent must hold both ends of
the serial pair open before the emulator starts, because the
emulator opens an existing pipe and blocks otherwise; and a guest
compiler must be invoked by absolute path, because it resolves its
own helpers from `argv[0]`.
⚠ A guest that powers off on one machine profile halts instead of
exiting, so the driver treats the halt message plus its own
completion marker as completion and then kills the emulator.

## Approach

One shot per command: a begin marker, the command, an end marker
carrying the status, matched line-anchored. The status crosses the
line as text and is then the process's own exit code, which is what
[cli.md](cli.md)'s exit-code contract requires.
⛔ Every read has a deadline and the deadline is reported, per
[RULES.md](RULES.md) section 8. A machine that stopped answering and
a command that is still running are different states.

## Decision

⭐ **Taken 2026-09-21: a per-command 128-bit nonce in every
marker, matched line-anchored and exact.** `VMR-BEGIN-<nonce>`
opens, `VMR-END-<nonce> <status>` closes, and the driver takes
the first exact line. The nonce comes from the host's entropy
per command and is never reused. What this defeats is the stated
threat: a payload printing a marker-SHAPED line, which a fixed
marker cannot tell from the real one. What it does not defeat is
a guest that reads the nonce off its own stdin and forges the
exact line: the nonce crosses the same serial line as the
command, so a hostile guest is outside this model, and the entry
says so rather than implying otherwise.
⚠ **Rejected:** fixed markers (the shape a payload collides
with by accident) and bare sequence numbers (the same shape
with arithmetic).

## Prove

`./experiments/147-podvm-exec.sh` asserts a zero status, a non-zero status and a deadline are each reported distinctly, and that a payload printing a marker-shaped line does not end the command

## Done

**Done 2026-09-21.** `experiments/147-podvm-exec.sh` exits 0 on a
lane-built binary: the guest boots, the shell answers the handshake,
`true` reports 0, `exit 3` reports 3, a marker-shaped line with the
wrong nonce is ignored with the real 7 reported, and `sleep 30` past
an 8 s deadline reports DEADLINE, which is no status
(`experiments/results/podvm-exec.txt`). The assembly moves to
`experiments/lib/podvm-guest.sh`, shared with 146; 146 re-driven on
the refactored tree exits 0 with evidence identical bar the date line,
so the committed `podvm-initramfs.txt` is that re-drive.

Five findings on the way, each in the script now. The serial speaks
CRLF, so the reader strips it once (`tr -d '\r'`) and no anchored
match has to know. `tr` block-buffers a file target, so the reader
runs unbuffered (`stdbuf -o0`). The pipe backend opens existing fifos
and blocks otherwise, so the parent holds both ends O_RDWR before qemu
starts. `od` reads to EOF and urandom has none, so the nonce reader
takes `-N16`: without it the first handshake hung unboundedly, which
is the shell form of the urandom trap
`src/devices.rs:218` of `pod-rootfs` (PodUtils/podcrates) records. A bare `exit 3`
ends the very shell that must print the end marker, so the command
runs in a subshell and the reporter outlives it. The end-marker
pattern accepts the exact line and rejects six forgery shapes,
checked host-side against the same pattern text. The kernel pin stays
measured: the mirror serves 6.12.110 and the reference names 6.12.94.

Residual, not in this change: `curl -fsSL` in 146 and 147 carries no
`--max-time`, so a stalled mirror is the one network wait without a
ceiling. It predates this entry (146 introduced it) and changing 146
would invalidate T-1303's evidence, so it needs its own entry.

# T-1305: The fleet and the fork, and the file-size ceiling that bounds both

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 3 and 5
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The machine tier's value over the chroot tier is that a guest can be
copied, restarted and thrown away. Neither copying nor restarting
is free, and the ceiling that bounds them is not the one a reader
expects.

## Premise

⚠ **`RLIMIT_FSIZE` bounds a single file, and a guest's memory image
is a single file.** The specification measures a 500 MiB per-file
ceiling on its target and a guest above it dying from `SIGXFSZ`.
⛔ That is a different resource from the writable allowance
[RULES.md](RULES.md) section 8 already covers, and checking one does
not check the other: a machine with terabytes free still kills a
guest whose memory image crosses the per-file limit.
⚠ Every figure in the specification is that host's. Reproduce the
mechanism and re-measure.

## Approach

Read `RLIMIT_FSIZE` in the probe ([T-1301](#t-1301-the-machine-tier-is-probed-leg-by-leg-and-a-present-file-is-not-a-working-one)),
and refuse a guest whose configured memory would cross it, before it
starts, naming the number. Fork is a quiesce, a copy of the memory
image and a restore into a second guest.

## Decision

Ruled 2026-09-21 under the operator steer that podvm works on
every environment with as many features and fallbacks as it can
honestly carry. The fleet IS podvm's job, spelled with the verbs
it already has: `run`, `ps`, `stop`, `rm`, `images` and `logs`
over machine-tier guests. No new fleet verb: `docker` has none,
so one here is the extension this file's opening rule forbids,
and a second verb family is a second parity table
[cli.md](cli.md) T-0808 would have to drive. Fork (quiesce, copy
the memory image, restore into a second guest) is NOT this
entry: no guest driver ships yet (T-1302's holds-branch still
refuses), so there is nothing to fork; it stays future work for
when the driver lands. What this entry ships is the bound both
fleet operations share: `--podvm-mem` (machine-tier only,
`--podvm-`-prefixed per T-1302, refused elsewhere), judged
against `RLIMIT_FSIZE` BEFORE the legs, so an over-ceiling guest
is refused naming both numbers on every machine. The `--memory`
trap T-1302 names closes per tier: chroot keeps refusing
`-m`/`--memory` (no cgroup to enforce with), machine enforces
`--podvm-mem` against the file ceiling, and neither tier
accepts a limit it silently drops.
⚠ Rejected: a `fleet`/`fork` verb family (a second table), and
no fleet ruling at all (the Problem this entry states would stay
open silently).

## Prove

`./experiments/148-podvm-fleet.sh` asserts a guest over the file-size ceiling is refused BEFORE it starts, with the ceiling in the message

## Done

**Done 2026-09-21.** `--podvm-mem` on `run` and `exec` (`create` rides
`run`'s parser through `parity::rows_of`): bytes with an optional K/M/G/T
suffix (`tier::parse_mem`, zero and overflow refused), machine-tier-only
and refused elsewhere as needs-machine, judged in `enter_machine` BEFORE
the legs against `prlimit(RLIMIT_FSIZE)` with both numbers in the refusal
at exit 125. `experiments/148-podvm-fleet.sh` exits 0 on a lane-built
binary: 10 driven, 0 mismatches
(`experiments/results/podvm-fleet.txt`). The lane's natural ceiling is
infinity (conditions print `(-1, -1)`, python's infinity), so the script
lowers the limit to 1 GiB in the driven child only (python3 setrlimit +
exec, asserted on take): 4 GiB refused naming ceiling 1073741824 and guest
4294967296, 1 MiB passes through to the legs verdict with no ceiling
language, three not-a-size spellings are flag errors, and `exec` mirrors
`run`. Unit tests pin the spelling matrix, the ceiling comparison
(including infinity never refusing), and both parsers in both spellings;
`every_flag_the_table_admits_is_handled_by_this_parser` holds for the two
new rows on both verbs. 145 re-driven green unchanged (19 driven, 0
mismatches); 325 re-driven green at 160 rows, 199 driven, 0 mismatches,
which also retires the staleness T-1302 left in T-0808's record below.

# T-1306: The non-goals are refusals the code makes, not notes in a document

**Source:** `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 7
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

The specification lists designs that were tried and do not work on
its target: a TCP listener of any kind, anything KVM-accelerated, a
runc-style OCI container, a user-mode Linux guest, an identity
switch through `uid_map`, and a guest file over the size ceiling.
⛔ **A list of non-goals in a document is a list the next session
re-discovers.** podvm's own rule is that a limit stated as a fact
is inherited as a fact, so each of these has to be a probe result
and a named refusal rather than a sentence.

## Premise

⭐ **This is the entry that keeps the mode honest on a runtime that
is not the one it was specified against.** Each non-goal above is
blocked by a MECHANISM on that host, and every mechanism is
something [probe.md](probe.md) can ask about. A mechanism that is
open on another runtime turns a non-goal back into a goal, and a
hard-coded refusal would hide that.
⚠ The specification itself records the contract moving under it: a
network rule that denied a port early in a session and permitted it
later. ⛔ So a refusal is re-probed per run and never cached across
one.

## Approach

One probe leg per non-goal, and a refusal that names the leg, the
errno and the remedy where there is one. The four-part diagnostic
[cli.md](cli.md) T-0805 defines is the shape, and this reuses it
rather than inventing a second one.

## Decision

A non-goal is a measured verdict with a date, never a constant.
⛔ Where the mechanism is genuinely unavailable the refusal says
which leg failed, so a reader can tell "this runtime refuses it"
from "podvm does not implement it".
Ruled 2026-09-22 on the one fork the Approach leaves open: ONE
new leg, five cited. Only the TCP bind had no measurement; the
other five mechanisms already have probe rows (the kvm open, the
UTS and user namespace rows, ptrace, setuid/setgroups, the fsize
bound), and a second row measuring the same denial is the
value-in-two-places drift the forbidden patterns refuse. The
per-non-goal verdict lives in the new assessment, not in new
legs. Cross-run staleness is bounded by the probe cache key
(T-0111), like every other verdict in the document.

## Prove

`./experiments/149-podvm-non-goals.sh` asserts each refusal names its leg and its errno, and that a leg the host permits turns the refusal off

## Done

**Done 2026-09-22.** One new census row (`bind(127.0.0.1:0)+listen`:
socket, bind port 0, listen once, close; the family in native order for
the big-endian targets) plus `nongoals::assess`: six stances over cited
rows, refused with leg, errno and remedy, open with no refusal language,
unestablished where the rows cannot say. The document carries
`non_goals[]` and the stderr evidence a block beside the machine one.
`experiments/149-podvm-non-goals.sh` exits 0 on a lane-built binary: 14
driven, 0 mismatches (`experiments/results/podvm-non-goals.txt`). Lane
stances: tcp open, kvm refused naming
`open(/dev/kvm, O_RDWR)=ENOENT`, runc refused naming its errno, uml,
uid_map and file open (ptrace permitted, setuid works as root, no finite
ceiling there). Five assessment unit tests and two report tests; 81 of 81
probe tests green in the lane.

**Partial, 2026-09-25.** Issue 36 reopens this entry: each non-goal
is a refusal the code makes, and none is implemented. Promote one
non-goal instead of banking all six as permanent refusals. Smallest
is guest networking through the user-mode hostfwd the TCP remedy
already names, or the TCG path from T-1301 that makes the kvm
refusal conditional. Fix area is the machine and non-goal
assessment with `149-podvm-non-goals.sh`. Risk if wrong is a new
implementation that weakens the honesty rules: the banner states
what is emulated. Prove is `149` extended with a positive arm
where the promoted goal runs (a guest reaching the host through
UDP hostfwd, for example) with the remaining refusals unchanged.

**Done 2026-09-26.** The promoted goal is guest user-mode
networking: the kvm node stays refused, and its remedy (TCG
instead of KVM) is now executable. `experiments/361-guest-usernet.sh`
builds a TCG guest from the podvm-extracted alpine rootfs with a
pinned kernel and modloop, loads the `failover`, `net_failover`
and `virtio_net` module chain the netboot kernel keeps out of the
image, brings up the first non-loopback interface, and crosses
datagrams both ways: the guest token reaches a host listener, and
a host reply forwarded into the guest prints on the guest
console. `149-podvm-non-goals.sh` gains the positive arm (clause
6 runs 361 for its exit code); its stance clauses are unchanged,
and the kvm node still refuses naming ENOENT beside them.

Driven on the lane (`rust:1.98.1-bookworm` job container, kernel
`7.2.0-WSL2-STABLE`, qemu 7.2.22 installed where absent):
149 exits 0 with 15 driven 0 mismatches
(`experiments/results/podvm-non-goals.txt`), 361 exits 0
(`experiments/results/guest-usernet.txt`): both applets and both
pins hold, three modules staged, `HOST-GOT: VMR-NET-PROBE` on the
host and the reply on the guest console. The full `podvm-probe`
suite is green on the same drive (107 passed, 0 failed). Three
findings paid for on the way, each in the script: the netboot
kernel carries no NIC driver (virtio_pci binds, no interface);
unsquashfs `-l` prints a `squashfs-root/` prefix its extractor
refuses (measured: 0 files); the module chain resolves link by
link (failover, then net_failover, then virtio_net). No product
code changed: the kvm remedy already names TCG, the TCG banner
already states user-mode networking, and the assessment already
turns a non-goal off where its mechanism works. What is new is
the measured capability with a standing arm that re-proves it.

# T-1307: The five Rust VM tools, ruled one by one, so nobody surveys them again

**Source:** [reference-map.md](reference-map.md); the sweep at
  [`../docs/history/2026-09-11-reference-sweep.md`](../docs/history/2026-09-11-reference-sweep.md)
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

Five Rust projects were named as candidate microVM managers for the
machine tier. **A candidate list nobody rules on is a list the
next session surveys again.** Each needs one verdict, with the
reason recorded, so the survey is paid for once.

## Premise

**Read on 2026-09-11, and not one of the five is a microVM
manager for a capability-denied host.**
`references/hust-open-atom-club__Vex` is a Docker-like command line
that saves, shares and launches named `qemu-system-*`
configurations. It composes command lines and boots nothing itself.
`references/cubic-vm__cubic` boots official distribution cloud
images with `cloud-init`, and it **accelerates every machine with
KVM, Hypervisor or WHPX** - the one facility the target denies.
`references/Obirvalger__vml` presents machines as directories with
a `vml.toml` and requires `kvm` plus `rsync`, `socat` and
`cloud-localds`.
`references/gevico__tcg-rs` is a Rust reimplementation of QEMU's
TCG: a RISC-V guest translated to x86-64 host code, with 816 tests
and a differential test against QEMU. RISC-V guest only, so it
cannot carry an x86-64 payload today.
`references/qemu-rs__qemu-rs` is **not a manager at all**: it is a
Rust binding to QEMU's TCG **plugin** C API. And
`references/qemu-rs__qemu-rs/tree/Cargo.toml:7` declares
`GPL-2.0-or-later`, which `TOOL.md` section 3.3 already settles
against for this tree.
Correction 2026-09-22, read against the tree: the `cubic`
sentence above overstates. `tree/src/qemu/qemu_accelerator.rs:47-64`
defaults to one accelerator per host OS (kvm, hvf, whpx, nvmm,
else tcg), and `tree/src/actions/start_instance_action.rs:177-193`
falls back to software tcg on `Accel::Off`, arch mismatch, or a
failed probe. Unaccelerated machines exist there; the verdict
below stands unchanged.

## Approach

One row per tool in [reference-map.md](reference-map.md), which is
written, plus the two mechanisms that survive the reading:
1. **ask the emulator which accelerator works, never assume
   one.** `cubic` reached this independently: its pull request #538
   is `feat: ask qemu which accelerator works on the host` and its
   issue #6 is `Only use KVM if available`. T-1301 is where podvm
   does it;
2. the named-configuration shape from `Vex`, which is what a
   machine-tier profile is, and which T-1303 already carries.
Take no code from any of the five. Four permit it and the fifth
does not, and none of the four has a line podvm needs.

## Decision

`Vex` **confirms** and `vml` **confirms**: independent evidence
for the parity rule and for the machine-as-a-directory shape, no
new work. `cubic` is an **anti-pattern exhibit**, kept for T-1302's
defect. `tcg-rs` is **filed** here, below. `qemu-rs` is
**refused**, and the licence alone settles it.
**`tcg-rs` is filed rather than refused**, and the reason is
worth keeping: the target permits RWX `mmap`/`mprotect`, which is
exactly what a translator's code buffer needs, so a pure-Rust TCG
could in principle run where QEMU runs and would remove the QEMU
dependency entirely. It is a RISC-V guest today, so it is years
from useful here. Revisit only when it carries an x86-64 guest.

## Prove

`./target/release/podvm-gate` resolves every row, and each of the five names its verdict and the tree line that settles its licence

## Done

**Done 2026-09-22.** Every row of [reference-map.md](reference-map.md)
names its verdict and the tree line that settles it, and
`./scripts/check-todo.py` (now `podvm-gate`) resolves every row green. `Vex` **confirms**
(`tree/src/commands/exec.rs:39` builds the qemu command line and spawns
it: a composer, not a manager). `vml` **confirms**
(`tree/README.md:20-26` names kvm, rsync, socat and cloud-localds).
`cubic` is the **anti-pattern exhibit** (T-1302's space-splitting defect;
the acceleration premise above is corrected underneath it).
`tcg-rs` is **filed** (`tree/README.md:6-8`: a Rust TCG with a RISC-V
guest only). `qemu-rs` is **refused** (`tree/Cargo.toml:7` declares
`GPL-2.0-or-later`, and it binds the plugin API rather than managing
anything). No row moved and no verdict changed; the one overstatement
found on the way is corrected above with its lines.

# T-1308: One TCG number is a claim about one benchmark, and the range is 3x to 21x

**Source:** `references/talaria0101__vm-research/tree/experiments/logs/66-tcg-benchmarks.log`
  and `references/talaria0101__vm-research/tree/experiments/logs/72-bench-matrix.log`;
  `references/talaria0101__sandbox-insights/tree/experiments/logs/55-tcg-exec-and-bench.log`;
  `references/Azathothas__sandbox-insights/tree/docs/open-questions.md`
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

[T-1301](machine.md) selects the machine tier and the selection is a
cost decision, so podvm will have to tell an operator what the
tier costs. **Every source that states one number states a
different one**, and a reader who takes any of them as "the TCG
tax" will be wrong by up to seven times.

## Premise

**Measured, four times, on one host class, and the spread is
real.** Every checksum matched on every platform in both trees, so
none of these measures a different computation.
md5 of 16 MiB: host 0.026 s against guest 0.07 to 0.08 s, which is
about **3x**.
A tight integer loop of 30 M iterations: host 854.1 Mops/s against
guest 274.7 Mops/s, which is **3.1x**.
A dependent double chain: host 708.8 Mops/s against guest
78.4 Mops/s, which is **9.0x**.
xorshift32 with double accumulation and FNV mixing, 30 M
iterations: host 721.4 to 754.8 Mops/s against guest 31.4 to
35.3 Mops/s, which is about **21x**.
**A document in that corpus states "20-25x" as its decision-table
speed row and repeats it in its recommendation**, from the last
measurement alone. Both of its own numbers are right and they
measure different things.
Two of the four trees already say so. One writes that any
single-number multiplier is "a claim about a benchmark and a
moment, not about emulation". The other files it as an open
question and names the closure test.

## Approach

Adopt that closure test rather than inventing one: integer,
syscall, memory-bandwidth and I/O workloads, reported
as a distribution rather than a point, with same-day native and
chroot controls taken on the same host in the same run.
Compilation is a named gap, not a fifth row: the guest
carries no toolchain and has no network to fetch one, so no
compiler runs inside it. Carrying a static compiler into the
extras archive is future work; until then the spread rests on
the four classes one binary can cover on all three platforms.
Every row prints a checksum and the run is refused if two
platforms disagree, because a platform that computed something else
must say so rather than look fast.
podvm quotes the **range and the workload**, never a single
figure: the four-part diagnostic [cli.md](cli.md) T-0805 defines
has room for it.

## Decision

**podvm never prints a bare multiplier.** Where the machine tier
is selected the banner names the workload class and the measured
range for it, or it names nothing. **Rejected:** carrying the
"20-25x" figure as a default, which is a real measurement of
somebody else's benchmark on somebody else's host and would be read
here as a property of emulation.

## Prove

`./experiments/154-tcg-workload-spread.sh` prints one row per workload class with its checksum and its ratio, and exits 1 if any two platforms disagree on a checksum

## Done

**Done 2026-09-22.** `experiments/154-tcg-workload-spread.sh` exits 0
             on a lane-built binary: 14 driven, 0 mismatches
             (`experiments/results/tcg-workload-spread.txt`). One static
             binary per class runs on all three platforms, medians of three
             runs: int host 0.153 s, chroot 0.155 s (1.0x), guest 1.136 s
             (7.4x); syscall host 0.267 s, chroot 0.270 s (1.0x), guest
             3.060 s (11.5x); memory host 1.094 s, chroot 1.118 s (1.0x),
             guest 1.588 s (1.5x); file I/O host 0.087 s, chroot 0.092 s
             (1.1x), guest 0.453 s (5.2x). Every checksum agrees on every
             platform, so no platform computes a different thing. The chroot
             tier costs nothing measurable here (1.0 to 1.1x on all four
             classes). The TCG spread runs 1.5x (memory) to 11.5x
             (syscall): syscall-heavy payloads pay most, and no single
             multiplier describes the tier. That confirms the Decision: the
             banner names the class and its range, never a bare figure.
             Conditions: lane base `rust:1.98.1-bookworm`, host kernel
             7.2.0-WSL2-STABLE, qemu 7.2.22 with
             `-M pc,acpi=off -m 256 -accel tcg,thread=multi`, gcc 12.2.0
             with `-O2 -static`, the pinned image and kernel above. The
             guest file-I/O leg runs against initramfs (RAM): no disk
             device passes to qemu, so that row compares filesystems, not
             disks. Two notes stay open: an `io error=` line counts as a
             run in section 2 while section 4 still refuses it
             (report-internal only, the exit stays 1), and the conditions
             block does not repeat the qemu flags or CFLAGS (both pinned
             in the script). The results file was recovered from the kept
             job transcript: the run left no `PODVM_ARTIFACTS`, so the raw
             per-run logs stayed in the guest job directory. A measurement
             job always names `PODVM_ARTIFACTS`.

**Partial, 2026-09-25.** Issue 53 reopens this entry on its own
two notes: an `io error=` line counts as a run in section 2 while
section 4 still refuses it (report-internal only, the exit stays
1), and the conditions block does not repeat the qemu flags or
CFLAGS. Make section 2 count an `io error=` line the way section
4 does, or say in both why not, and repeat the flags in the
conditions block. Fix area is the measurement script with
`experiments/results`. Risk if wrong is a reader adding two
numbers that should agree and getting a total that does not.
Prove is the re-driven `154` with both tallies agreeing and the
flags in the conditions.

**Done 2026-09-26.** `experiments/154-tcg-workload-spread.sh`
exits 0 on the lane (`rust:1.98.1-bookworm` job container in
`wsl-toolkit-podvm`, host kernel `7.2.0-WSL2-STABLE`, lane-built
musl release binary): 14 driven, 0 mismatches
(`experiments/results/tcg-workload-spread.txt`). Section 1 now
counts `^workload=` lines requiring 3, exactly like sections 2
and 4, so an `io error=` line beside a zero exit can no longer
pass one tally and fail the other; the tallies agree by
construction. The conditions block prints the pinned
`QEMU_FLAGS` (`-M pc,acpi=off -m 256 -nographic -no-reboot
-accel tcg,thread=multi`) and `BENCH_CFLAGS` (`-O2 -static`)
beside the invocations that use them, so a re-drive that
changes a flag cannot silently compare against this report.
Rows: int host 0.155 s chroot 0.148 s (1.0x) guest 1.168 s
(7.5x); sys host 0.288 s chroot 0.257 s (0.9x) guest 3.093 s
(10.7x); mem host 1.131 s chroot 1.106 s (1.0x) guest 1.557 s
(1.4x); io host 0.094 s chroot 0.095 s (1.0x) guest 0.457 s
(4.9x). Every checksum agrees on every platform. The chroot
tier still costs nothing measurable (1.0x or better on three
classes, 1.0x on the fourth). The TCG spread on this run is
1.4x (memory) to 10.7x (syscall), confirming the Decision
again: no single multiplier describes the tier.

# T-1313: Every experiment fetch carries its own ceiling, and a stalled origin proves it

**Source:** `TODO/machine.md` T-1304 residual; `experiments/146-podvm-initramfs.sh:153`,
  `experiments/147-podvm-exec.sh:119`, `experiments/152-nix-acceptance.sh:94`
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Three experiment fetches wait on the network with no ceiling:
the kernel fetch in 146 and 147 (`curl -fsSL`, no `--max-time`,
no `timeout` wrapper) and the nix tarball fetch in 152. A
stalled mirror turns any of the three into a hang no clause
names. [RULES.md](RULES.md) section 8 gives every
network-touching command a timeout, and podvm inherits that as
a product requirement; the project's own scripts are not exempt.

## Premise

Every OTHER wait in 146/147 is already bounded (146 boots under
`timeout BOOT_TIMEOUT`, 147 carries boot, command and handshake
deadlines), and the tree already bounds fetches both ways:
`timeout 300 curl` in `experiments/290-microvm.sh:91` and
`--max-time 60` in `scripts/common/mine-repo.sh:416`. The fix is
one flag per fetch, not a new mechanism.
`experiments/280-insecure-registry.sh:287,291` needs no change:
loopback health polls inside retry loops, where the loop is the
bound.

## Approach

`--max-time` on all three fetches, default 60, env-overridable
per script (`PODVM_146_CURL_TIMEOUT` and kin, following the
`BOOT_TIMEOUT` pattern already in 146/147) for a slow mirror.
A stalled-origin clause in 146 proves the bound bites: a
localhost server that accepts and never answers, fetched with a
2 s ceiling, must exit 28 inside 10 s. A bound nobody drove red
is a comment, not a ceiling. Changing 146 re-drives T-1303's
evidence and changing 152 re-drives T-1111's, so all three
scripts re-drive green in the same change.

## Decision

Recommendation: `curl --max-time`, not the `timeout` wrapper.
The ceiling travels with the transfer rather than with the
process, and 146 already gates on `curl`. Ruled out: leaving
152 out because its re-drive is heavy (that leaves the heaviest
fetch unbounded, which is the one that most needs the ceiling).

## Prove

`./experiments/146-podvm-initramfs.sh` exits 0 on a lane-built binary with the stalled-origin clause green

## Done

**Done 2026-09-22.** `--max-time` on all three fetches, default 60,
             env-overridable per script (`PODVM_146_CURL_TIMEOUT`,
             `PODVM_147_CURL_TIMEOUT`, `PODVM_NIX_CURL_TIMEOUT`),
             with the ceiling in each conditions block. 146 gains clause 6:
             a localhost python server that accepts and never answers,
             fetched with a 2 s ceiling under `timeout 10`, exits 28. All
             three re-drive green in this change: 146 exits 0 on a
             lane-built binary with clause 6 green
             (`experiments/results/podvm-initramfs.txt`), 147 exits 0 on a
             lane-built binary with every status distinct
             (`experiments/results/podvm-exec.txt`), and 152 exits 0 on the
             Windows host against host podman 6.1.2 with all seven rows
             green (`experiments/results/nix-acceptance.txt`). T-1303's and
             T-1111's evidence stand re-driven. Two findings belong to the
             driving, not the code: two `run-in-base.sh` jobs from one
             checkout stage through one job file, so both drove 146 while
             one was asked for 147 (recorded as the eighth trap in
             [`../docs/containers.md`](../docs/containers.md); 147 was
             re-run alone), and a measurement job always names
             `PODVM_ARTIFACTS` (146's binary rode home that way and drove
             152 on the host). One pre-existing wart, untouched: the
             subject prints `/bin/sh: 18: echo: echo: I/O error` where
             `grep -qm1` closes the pipe early; every row stays green.

# T-1568: Port `experiments/145-podvm-parity.sh` to `podvm-parity`

**Source:** `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:55`;
  `experiments/145-podvm-parity.sh`; `TODO/machine.md` T-1302
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

The help parity proof is shell. The pure half asserts that
`podvm --help` and `podvm --help` list one set of verbs, and
that half becomes a unit test; the live-payload rows stay
driven, by the binary the same way the script drove them.
Nothing here pulls an image.

## Premise

The exit codes are data, read out of the binary like the
script reads them out of the exit-codes helper: a case the
table does not carry is a failure to read it, never a guess.
The name routes on the basename of argv[0], so a copy named
`podvm` drives exactly like the script's symlink, portably.

## Approach

Port the proof to binary `podvm-parity` with no
dependencies, parsing the parity and exit-code answers
in-process instead of `jq`. Keep the experiment path as a
compat shim that execs the binary, so T-1302 keeps its path.

## Decision

The script keeps its path and its exit codes; the binary
owns the clauses, the copy it stages, and the report.

## Prove

`cargo test -p podvm-measure` green in the lane; the proof
drives the shipped binary; `./target/release/podvm-gate`
exits 0.

## Done

**Done 2026-10-01.** The `podvm-measure` crate carries `podvm-parity`
and `podvm-workload` with no dependencies, and both experiment
paths stay as exec shims. Lane proof
([podvm-crate-proof](../experiments/results/podvm-crate-proof.txt)):
unit tests 3 and 4 passed; retired script and shim both exit 1 over
the same CLI with identical counts (14 driven, 5 mismatches);
verdicts agree retired-vs-binary; both shims reach their binaries;
clippy and fmt clean. Plant
([transcript](../experiments/results/podvm-crate-plant.txt)): unknown
flags exit 2, an unreadable exit-code table exits 2, the median unit
proof holds. Qualification: chroot rows intermittently report short
counts with rc 0 while checksums agree where runs complete, and the
guest half never reaches its done marker in the lane; both behaviours
are environmental and identical retired-vs-binary, and stay owned by
T-1302 and T-1308. The record gate exits 0 on the landed tree.

# T-1569: Port `experiments/154-tcg-workload-spread.sh` to `podvm-workload`

**Source:** `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:55`;
  `experiments/154-tcg-workload-spread.sh`; `TODO/machine.md` T-1308
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

The workload spread measurement is shell. One payload binary
per class runs on three platforms, so equal checksums are the
control that every platform computed the same thing. It moves
into the `podvm-measure` crate beside `podvm-parity`.

## Premise

The pins are the contract: the image ref, the kernel URL and
hash, the qemu flags, and the payload flags print in the
conditions. The guest assembly is ported, not shelled out
to: the base is a `find`/`cpio` pipeline driven without a
shell, and the extras archive is a byte-exact newc writer. A
median of fewer than three runs is a wrong number, not an
approximation.

## Approach

Port the measurement to binary `podvm-workload` with
no dependencies. Keep the experiment path as a compat shim
that execs the binary, so T-1308 keeps its path.

## Decision

The script keeps its path, its pins, and its exit codes; the
binary owns the runs, the assembly, and the rows.

## Prove

`cargo test -p podvm-measure` green in the lane;
`./target/release/podvm-gate` exits 0.

## Done

**Done 2026-10-01.** Alongside `podvm-parity`, the dependency-free
`podvm-workload` binary joined the `podvm-measure` crate, and
T-1308 keeps its path through an exec shim. Lane proof
([podvm-crate-proof](../experiments/results/podvm-crate-proof.txt),
section 5, rows in
[tcg-workload-spread](../experiments/results/tcg-workload-spread.txt)):
the workload drives to a verdict (exit 1, 6 driven, 6 mismatches)
with the pins printed in the conditions; host and chroot checksums
agree where runs complete. Qualification: chroot int and sys rows run
short intermittently with rc 0, and the guest half never reaches its
done marker in the lane; both are environmental, identical
retired-vs-binary, and stay owned by T-1308. The record gate exits 0
on the landed tree.

# T-1606: Repoint all 47 `references/` citations before deleting the corpus

**Source:** End-state point 3; `refactor/DEFERRALS.md` section 3;
  `TODO/reference-map.md`; `crates/podvm-gate/src/main.rs`
  `check_tree`
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

`references/` is 46 repositories and 168 MB of other
people's source, tracked in this repository and cited 47
times from `TODO/reference-map.md` and the live documents.
The end state calls for deleting it. Deleting it today turns
the record gate red, because `check_tree` reads every cited
path and line and fails when one does not resolve.

## Premise

The corpus is evidence, not dead weight: every citation is a
recorded source and licence decision, and the gate validates
that each cited line still exists. Removing the corpus
without repointing would delete the evidence and the
validation together, and the gate would catch the loss only
as an unresolved link rather than as a missing decision.

## Approach

Keep `references/` for now and repoint first. Each of the
47 citations moves to podvm's own source or is dropped
with a recorded reason, and the gate stays green through the
move rather than after it. File the repoint as its own
reviewable batch, then delete the corpus as a separate
change once the gate is green without it. Do not delete
first and fix citations after: that is a red tree in the
middle.

## Decision

Repoint then delete, on the operator's answer of
2026-10-02 that `references/` stays and the repoint is filed
as work. The corpus is retained deliberately until this
entry closes, and its retention is a decision with a
clearing condition rather than an omission.

## Prove

`grep -rn "references/" TODO/ docs/` returns no citation
that `check_tree` resolves outside the corpus, or every
remaining hit is recorded with its reason; `./target/release/podvm-gate`
exits 0 with `references/` still tracked; after the
deletion lands, the gate exits 0 with the corpus absent and
`git status` shows no tracked file referencing it.

## Done

**Done 2026-11-06, under a new ruling.** The operator ruled on
2026-11-06 that the corpus leaves the live tree and that each reference is
tracked in a GitHub issue with the label `reference` instead. That replaces
the repoint above: the citations stay as written and are checked against a
manifest. The 46 issues are #132 to #192; `references/README.md` maps each
tree to its issue and records the state of its upstream.

`references/MANIFEST.tsv` lists the 7,974 files of the 46 trees with their
line counts, read from the blobs of commit `452d792`, and counted as
`count_lines_bytes` counts them. `crates/podvm-gate/src/main.rs` resolves
each `references/` citation against it: the `path:line` citations of
`check_tree` and of the TODO pass (`corpus_cite_problem`), the bare names
(`corpus_names`), and the trees of the reference map. An empty or missing
manifest is its own problem. Plant 6 now expects "and the map does not name
it", and the new plant 6b cites a corpus path the manifest does not list and
turns the gate red with "MANIFEST.tsv lists no such file".

The two bindings above are cleared. The template notice is now a tracked
file, `docs/notices/Azathothas__TEMPLATE-LICENSE.txt`, which
`THIRD_PARTY.md` links. The 155 mechanism citations resolve through the
manifest, so no podvm-side finding has to be written before the deletion.
Three experiments read corpus files at run time
(`experiments/20-enter-target.sh`, `experiments/80-interposer-abi.sh`,
`experiments/100-interpose-symbols.sh`); each now reads them from commit
`452d792` when the tree is absent. `.gitattributes`, `.gitignore` and
`.github/dependabot.yml` drop their corpus rules, and
`docs/methodology/references.md` states the new method.

Proof: with `references/` holding only `MANIFEST.tsv` and `README.md`,
`./target/release/podvm-gate` exits 0 at `corpus=46`, and `podvm-plant`
reports plants 6 and 6b caught. The last clause of the Prove, "no tracked
file referencing it", is superseded by the ruling: the citations into
`references/` stay by design.

**Partial 2026-10-02, and the entry's "47" was the wrong number twice.**
The reference map is repointed; the rest of the corpus is not freed, and
the clause cannot be met as written.

`grep -rn "references/" TODO/ docs/` returns 198 hits outside
`docs/history/`, and 156 of them are in superseded records that
`docs/methodology/history.md` requires be kept verbatim. Of the rest,
about 155 are `path:line` citations into third-party **source**, spread
across seventeen `TODO/*.md` files, and they resolve through
`check_document_state`. Repointing those is not a rewrite: each one
records a mechanism podvm adopted, and the honest move is to write the
finding into podvm's own record first and cite that. That is roughly an
order of magnitude larger than what this entry scoped, and it is the
work the entry should have been scoped to.

What was done. `TODO/reference-map.md` now states each licence
determination as a self-contained sentence with the file it was read
from, so the decision no longer needs the captured bytes; the orphan
pg-toolkit row, which sat in a stray one-row table after a blank line,
is merged into the main table, which is what `corpus=46` checks. Each
of the nine captures with no top-level licence file now says so instead
of implying one was read. `git status --porcelain references/` is empty:
no file under the corpus was touched, and the corpus stays tracked.
`./target/x86_64-pc-windows-msvc/debug/podvm-gate.exe` exits 0 at
`corpus=46`.

**Two corrections to the entry's premise, both measured on disk.** `ls
-d references/*/` is 46, not 47; the map held 46 rows plus one orphan,
and a markdown accident rather than a 47th citation. And none of them
ever cited a line of source: every one resolved to `tree/LICENSE`,
`tree/COPYING`, a manifest line, or the repository API capture, which
are licence determinations and carry no mechanism. So the instruction to
move each citation to podvm's own source has no answer for any of the
47, because nothing in `crates/` carries "fakeroot is GPL-3.0".

**The corpus cannot be deleted while the notice stays.** `THIRD_PARTY.md`
links `references/Azathothas__TEMPLATE/tree/LICENSE` as the retained
0BSD notice, and a licence obligation needs the notice as a file. That
citation is recorded, with its clearing condition, as the one binding
the corpus by licence rather than by evidence. The clearing condition
for the whole entry is therefore not "the 47 are moved": it is all the
`path:line` mechanism citations given a recorded podvm-side finding,
plus a licence decision on the template notice.

An attempt worth recording, because it cost a gate run: stripping the
`references/` prefix from the map's Tree cells turned the gate to 49
problems. The corpus registry at `crates/podvm-gate/src/main.rs` reads
that column to find every on-disk tree, so the path is load-bearing
infrastructure and not decoration.

# T-1626: Record that `149` clause 6 has no Rust arm and stays shell pending the guest fixture

**Source:** `refactor/recon-c.md:173`;
  `refactor/06-entries/PLAN.md:124`
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

Clause 6 of this script shells to another script under a
1500-second bound. No Rust arm exists, so the script is not
deletable and the tree does not say why.

## Premise

The other clauses of `149` are converted and their tests are
named in the source. This clause is the exception and it is
the one a reader counts when deciding whether the script is
finished.

## Approach

Record the clause, its bound, and the guest fixture it
waits on. Name the entry that owns the fixture.

## Decision

Record against the current guest entries rather than
opening a new one, so the gap has one owner.

## Prove

`grep -n "149-podvm-non-goals" TODO/machine.md` names clause
6 and the entry that clears it.

## Correction

**Recorded 2026-10-02.** Clause 6 of
`experiments/149-podvm-non-goals.sh:183-193` has no Rust arm and the
script is not deletable. The clause tests `[ -x
"$REPO/experiments/361-guest-usernet.sh" ]`, then runs that script
under `timeout 1500` with `PODVM_BIN` exported, writes its output to
`$WORK/361.log`, and appends the whole of it to the report on failure.
It owns the exit code and nothing else, so a red 361 reads here as a
missed promotion rather than a stance regression, which is what the
clause's own comment says. Clauses 0 through 5 are the converted
half: they read `podvm probe --json` and its stderr evidence, and
their tests are
`crates/podvm-probe/src/nongoals.rs:252-318` (five tests, `mod tests`
at `:227`) and `crates/podvm-probe/src/report/tests.rs:312` and line 1087 at `17569d355192`.
The audit calls the clause a claim about a different script
(`refactor/01-audit/group-9.md:117`), and the ledger keeps 149 at
SPLIT and 361 at KEEP-SHELL
(`refactor/06-entries/verdict-ledger.tsv:15` and `:65`).

The fixture clause 6 waits on is a booting guest, and no Rust test in
the tree builds one. `experiments/361-guest-usernet.sh` pulls the
pinned alpine image through the binary, fetches the netboot kernel and
its modloop against their pins, assembles the base and extras
archives, and drives `qemu-system-x86_64` with
`-netdev user,id=u0,hostfwd=udp::$HPORT-:$GPORT`
(`experiments/361-guest-usernet.sh:293-299`) so a token crosses guest
to host and a reply is forwarded back. The only Rust in this tree that
starts an emulator is `crates/podvm-measure/src/workload/drive.rs:323`, and
it runs TCG with a serial console and no forwarding, so it cannot
stand in for the fixture.

⛔ **T-1306 owns the fixture, and it is the entry that clears it.** Its
Done record at line 606 above is what built 361 and what carries its
pins, its result file and its conditions. The clause is removed only
when that entry's guest path has a Rust arm, which is new work and
not this entry's. The one thing that would remove the clause sooner is
a Rust guest-networking fixture, and
`refactor/06-entries/PLAN.md:154` states the shape such work takes: a
Rust fixture first, as its own wave-0-adjacent entry.

⚠ **One correction to the source above.** The plan's row for the
`149` script at `refactor/06-entries/PLAN.md:124` cites
a file nongoals.rs in `crates/podvm-supervise/src/` <!-- known-absent --> with
a line range. That path was never on disk.
`crates/podvm-supervise/src/` holds `launcher.rs`, `lib.rs` and
`table.rs` and no non-goals module. The tests are in
`crates/podvm-probe/src/nongoals.rs`, and the six row names they
assert are hard-coded at `experiments/149-podvm-non-goals.sh:95-100`.

## Done

**Done 2026-10-02.** The entry records that clause 6 of
`experiments/149-podvm-non-goals.sh:183-193` has no Rust arm while clauses
0 through 5 are converted and tested at
`crates/podvm-probe/src/nongoals.rs:252-318` and
`crates/podvm-probe/src/report/tests.rs:312` and line 1087 at `17569d355192`. Clause 6 runs
`experiments/361-guest-usernet.sh` under `timeout 1500` and owns that exit
code alone, so the `149` script is not deletable. The fixture is a booting
guest with UDP forwarding (`experiments/361-guest-usernet.sh:293-299`), no
Rust test builds one, and T-1306 owns it: its Done record at line 606 at `98112cb974e3`
built 361 and the ledger keeps 149 at SPLIT and 361 at KEEP-SHELL
(`refactor/06-entries/verdict-ledger.tsv:15` and `:65`). One correction to
the cited source: `refactor/06-entries/PLAN.md:124` names
a file nongoals.rs in `crates/podvm-supervise/src/` <!-- known-absent -->, a path
never on disk. The
proof is `grep -n "149-podvm-non-goals" TODO/machine.md`, now returning 9
matching lines.

# T-1656: MicroVMs: a fast disposable guest from an OCI image, on KVM or TCG

**Source:** issue 168; #193; the owner's ruling of 2026-11-06;
  A-V-08, G-02 of PodUtils/podcrates#39
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The owner's scope says podvm will "quickly create full on
microvms". `podvm run --podvm-tier=machine` probes the tier and
then refuses: running a guest from `run` is not wired. sand-net
booted a QEMU `microvm` machine from an OCI image and returned
the exit status of one command.

## Premise

`enter_machine` (`crates/podvm/src/tier.rs:112`) assesses the
machine profile and exits 125 with "running a guest arrives
with TODO/machine.md T-1303 and T-1304" (`tier.rs:138-141`).
T-1303 builds the initramfs from an extracted rootfs with an
appended console archive, and T-1304 is the serial exec
protocol with a nonce; both closed on experiments, not on the
`run` path. The TCG cost is 3x to 21x (T-1308). User-mode
networking already carried a guest in
`experiments/361-guest-usernet.sh:299`. The issue names the
podcrates crate `pod-microvm` (podcrates wave M7, PodUtils
podcrates issue 3).

## Approach

Wire `run --podvm-tier=machine` to boot a guest. Cache the base
archive by image digest and append only the per-run archive
(`/init`, the project, `resolv.conf`). Pin a small virtio-mmio
kernel by digest. Count boots for each machine flag set and
record which flags boot reliably. Give the guest QEMU
user-mode networking. Return the exit status through the
T-1304 markers. Carry `/workspace` back through 9p
(`-virtfs local`) or a userland block image, else say it is a
copy. Separate stdout and stderr on a second serial or
virtio-console port. Measure cold and warm time to the ready
marker under KVM and under TCG. Build on `pod-microvm` once
podcrates releases it, and do not wait for it.

## Decision

The banner names the boundary: the emulator process under TCG,
never hardware. One boot proves nothing about the next: the
acceptance counts twenty. vsock needs `/dev/vhost-vsock`, so
the second channel is a serial or virtio-console port. The
owner lifted the KVM deferral on 2026-11-09 (`docs/decisions.md`). A
guest runs under KVM where `/dev/kvm` opens, and the TCG leg is the
proof where it does not.

## Prove

`cargo test -p podvm` exits 0 with tests for the archive cache
key and the exit marker parse.
`sh experiments/412-microvm-boot.sh` exits 0 on a TCG host:
`podvm run --podvm-tier=machine IMAGE sh -c 'uname -r; exit 7'`
prints the guest kernel and exits 7, twenty boots in a row
reach the command, a second run reuses the base archive, the
guest fetches a URL through the host's loopback proxy, and a
file written to `/workspace` is on the host or the run says it
is a copy. A run that asks for KVM, with `/dev/kvm` closed to the user,
refuses by name and does not boot under TCG.

## Correction

**Filed 2026-11-06.** Quoted from issue 168, measured on an 8 vCPU Incus
guest with `/dev/kvm` and QEMU 10.0.13:

```text
qemu-system-x86_64 -enable-kvm -M microvm,rtc=on,pit=off,pic=off -cpu host
  -m MEM -smp 1 -nodefaults -no-user-config -nographic -serial stdio -no-reboot
  -kernel vmlinux -initrd initrd -append "console=ttyS0 earlyprintk=serial,ttyS0
  reboot=k panic=1 tsc=reliable sand.cmd=BASE64 ..."
F15: virtkit 6.18 under -M microvm,rtc=on crashed (divide error in
     kmem_cache_init_late) or hung in 5 of 6 boots; with pit=off,pic=off and
     tsc=reliable, 24 of 24 booted; which setting matters was not isolated
F16: console=ttyS0 alone printed nothing; earlyprintk=serial,ttyS0 gave all
memory at least 512 MiB; timeout 600 s; exit 7 came back as 7
limits: no shared directory, so /workspace writes never return; stdout and
     stderr merged on one serial line; no stdin
not built: Firecracker, Cloud Hypervisor (same kernel boots on QEMU
     microvm), Kata (973 MB bundle), microsandbox and smolvm (need /dev/kvm)
```

**Amended 2026-11-09.** The asks A-V-08 and G-02 of PodUtils/podcrates#39
(COORD-015, a message from a peer session) want a microVM from an OCI image
through `run`, KVM for a user in the group `kvm`, and a unit kind for each
label with no fallback to a weaker kind. The Approach holds the microVM
through `run`. Two clauses are new. A run that asks for the machine tier
never falls back to a rootfs tier. A run that asks for KVM never falls back
to TCG: when the user cannot open `/dev/kvm` for read and write, the run
refuses by name, and the message names the group `kvm`. The spelling of the
request for KVM is the implementer's choice, recorded in this entry. The
Decision text about the KVM deferral is corrected in place, because the
owner lifted the deferral.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC sections 6.6, 9.1, 10.1 and its question DR-5) uses a microVM
as the only unit that serves an untrusted job. Five clauses are new.
First, a guest can run with no network device and one host channel, a Unix
socket on the host bridged to a guest port (a virtio-console or serial
port; vsock where `/dev/vhost-vsock` opens), so a host broker is the only
way out. Second, the files that the caller stages for the guest (the
credential of the runner, PodUtils/podrunner#6) reach the guest's tmpfs,
and no per-run archive on disk keeps them after the boot. Third, a volume on this
tier is read-only 9p or a block image where the caller asks for read-only.
Fourth, `--timeout` and `rm -f` end the emulator, and no emulator stays
after the run (`AGENTS.md`, section 2). Fifth, the report of T-1695 names
the boundary (`kvm` or `tcg`) and the boot id of the guest. The acceptance
adds a guest that runs `--version` of a glibc .NET payload. The KVM leg
runs where `/dev/kvm` opens (T-1682). The position of this entry in the
work order stays as it is: if the owner answers DR-5 with "wait", this
entry blocks the `untrusted` tier of podrunner.

**Amended 2026-11-11, by the owner's answers to DR-5 and to its place.**
podrunner builds no microVM driver of its own: it waits for this entry, and
its `microvm` unit kind is an adapter over podvm (DR-5, PodUtils/podrunner#5,
relayed by PodUtils/podcrates#56). The owner answered "podrunner needs
first" for the place of this entry: it comes after the runner fixes on the
userland rung and after T-1682, before T-1652 (`TODO/PROGRESS.md`, item
3). T-1717 and T-1718 (incus) come after it.

The effort of T-1656 was XL before call C17 of `docs/decisions.md`; pod-todo has no XL.

## Done

**Done 2026-11-11.** Each clause of the Prove has a recorded run, as the
paragraphs below give: `cargo test -p podvm` with the tests of the cache
key and the marker parse; `PODVM_412_MIRROR=1 sh
experiments/412-microvm-boot.sh` exits 0 under TCG
([the run](../experiments/results/microvm-boot.txt)) with `uname -r; exit
7`, 20 of 20 boots, the cached base archive, the fetch through the host's
loopback proxy, `/workspace` as a copy that the run names, stderr apart,
stdin, and the refusal of a KVM ask with no `/dev/kvm`; under KVM on a
hosted runner a run takes about 0.6 s (kvm.yml run 42973225343). The
banner names the boundary under TCG. The `microvm` machine type, a shared
`/workspace` and an interactive stdin, which the Prove does not ask for,
are T-1723.

**Partial, 2026-11-11.** `run --podvm-tier=machine IMAGE CMD` boots a
guest (`crates/podvm/src/machine/guest.rs`, `crates/podvm/src/run/machine.rs`).
The rootfs becomes a `newc` base archive, cached in the store by the
manifest digest and a format number; each run copies it and appends a run
archive with `/podvm-init` and the console nodes. QEMU (`-M pc,acpi=off`,
`-no-reboot`, user-mode NIC) boots the pinned `vmlinuz-virt` of Alpine
3.22.6, which podvm fetches and checks against its SHA-256
(`crates/podvm/src/machine/guest/kernel.rs`). The exit code comes back
through markers with a nonce of 128 bits. KVM where `/dev/kvm` opens, TCG
elsewhere; `PODVM_MACHINE_ACCEL=kvm` refuses by name with no fall back
(call C16). `cargo test -p podvm` passes with the tests of the cache key,
the archive bytes, the accelerator rule, the marker parse and the init
quoting. `PODVM_412_MIRROR=1 sh experiments/412-microvm-boot.sh` exits 0
on the cloud host under TCG ([the run](../experiments/results/microvm-boot.txt)):
`uname -r; exit 7` prints `6.12.110-0-virt` and exits 7, a second run takes
the base archive from the cache, 20 of 20 boots reach the command (48 s,
about 2.4 s for each), and the KVM ask refuses with 125.

The guest network, the same day: the pinned kernel keeps `virtio_net` in
a module. `boot/initramfs-virt` of the same archive carries
`failover.ko`, `net_failover.ko` and `virtio_net.ko`, uncompressed; podvm
takes the three from it, each checked against its own SHA-256, and keeps
them in the store. The run archive carries podvm itself and the modules;
`/podvm-init` runs `podvm guest-net`
(`crates/podvm/src/machine/guest/net.rs`), which loads them with
`finit_module(2)`, configures `lo`, `eth0` (10.0.2.15/24) and the default
route through 10.0.2.2 with ioctls, and writes `/etc/resolv.conf`
(10.0.2.3). So no tool of the image is needed. Clause 5 of experiment 412
passes: `-e http_proxy=http://10.0.2.2:PORT` sends busybox wget to a
stand-in proxy on the host's loopback, which answers the guest.

Measured and left: Firecracker's CI kernel 6.1.102 (virtio-mmio and
virtio-net built in) for `-M microvm` did not boot under TCG. With
`pit=off,pic=off` it hung after "ACPI: Core revision"; with the PIT on and
ACPI it stopped in `kmem_cache_init_late` with a divide error (F15 of
issue 168); with ACPI off it booted, but it found no virtio-mmio device,
because it has no `CONFIG_VIRTIO_MMIO_CMDLINE_DEVICES`. `tsc_early_khz`
and `lpj` did not help. The machine stays `-M pc,acpi=off`.

`/workspace`, the same day: on the machine tier `run -v HOST:GUEST`
copies the host tree into the run archive at GUEST
(`Newc::tree_at` in `crates/podvm/src/machine/guest/cpio.rs`), and the run
says on stderr that the volume is a copy whose writes do not return to
the host, as the Prove allows. The 9p modules are not in
`initramfs-virt` (only in the xz modloop), and `virtiofs.ko` needs a
`virtiofsd` on the host, so a shared directory waits. Clause 6 of
experiment 412 passes: the guest reads a host file in `/workspace`, the
run names the copy, and the guest's write is not on the host.

stderr, the same day: a second serial port (`ttyS1`) is a socket
chardev with `server=on,wait=on` under the temporary directory, so QEMU
starts the guest only when podvm has connected; `/podvm-init` sends the
command's stderr there, and podvm copies it to its own stderr without the
tty's `\r` (`crates/podvm/src/machine/guest/stderr.rs`). Clause 7 of
experiment 412 passes: `echo out; echo err >&2; exit 4` gives `out` on
stdout alone, `err` on stderr, and exit 4.

stdin, the same day: where podvm's stdin is a pipe or a file, podvm reads
it (at most 64 MiB) into the run archive as `/podvm/stdin`, and
`/podvm-init` gives it to the command; else the command reads
`/dev/null`. A terminal does not reach the guest, so no interactive
stdin. Clause 8 of experiment 412 passes: 100000 random bytes give the
same SHA-256 in the guest.

Times under KVM: kvm.yml run 42973225343 ran experiment 412 on a hosted
runner ([the transcript](../experiments/results/microvm-boot-kvm.txt)):
the first run, with the kernel fetch and the base archive, took 3 s; 20
runs in a row took 12 s, about 0.6 s from `run` to the exit code; under
TCG on the cloud host the same 20 took 48 to 59 s.

Remaining then: the `microvm` machine type (Firecracker's kernel did not
boot under TCG; under KVM not tried); a shared `/workspace` (9p or
virtiofs); an interactive stdin. They are T-1723.

# T-1682: A `kvm.yml` workflow that runs the machine tier's Linux guest on a hosted runner

**Source:** the migration of 2026-11-06 to cloud sessions; `TODO/RULES.md:139`
  (the KVM deferral lifted); `experiments/392-kvm-guest.sh:1`;
  `crates/podvm/src/tier.rs:112`;
  A-V-08 of PodUtils/podcrates#39
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

The KVM proof of the machine tier has run only on the
operator's host. One run there left an emulator that SIGKILL
did not remove, and the host failed later. A cloud session
has no `/dev/kvm` (issue 114). No workflow runs the Linux guest
under KVM, so the KVM leg has no place to run that cannot harm
a host.

## Premise

GitHub's hosted Linux runners expose `/dev/kvm` to a job that
enables it with a udev rule; this is a documented runner
property, not measured here. The KVM guest experiment is
`experiments/392-kvm-guest.sh`. The machine tier's entry point
is `enter_machine` (`crates/podvm/src/tier.rs:112`). The
owner lifted the KVM deferral of 2026-10-02 on 2026-11-09
(`TODO/RULES.md:139`, `docs/decisions.md`).

## Approach

Add `.github/workflows/kvm.yml`, dispatched by hand. It checks
for `/dev/kvm`, builds podvm, runs the Linux guest experiment
with a job timeout and the T-1609 watchdog, and uploads the
transcript. A missing `/dev/kvm` exits 2, not red as denied.

## Decision

The owner lifted the deferral on 2026-11-09, so a session can add and
run the workflow. A hosted runner is the first choice over any
session host.

## Prove

`gh workflow run kvm.yml` starts a run whose guest experiment
exits 0, with the transcript saved under
`experiments/results/`, and `./target/x86_64-unknown-linux-musl/release/podvm-gate`
exits 0 with the result cited.

## Correction

**Blocked 2026-11-06, cleared 2026-11-09.** The blocker was the KVM
deferral of 2026-10-02 (`TODO/RULES.md` section 11). The owner lifted it
on 2026-11-09 (`docs/decisions.md`, row "KVM"). Nothing blocks this entry.

**Amended 2026-11-09.** The ask A-V-08 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) names KVM for a user in the group `kvm`. This
entry proves KVM on a hosted runner, where the udev rule gives the job
access to `/dev/kvm`. The clauses do not change. The Status moves from
`blocked` to `open`, because the deferral is lifted.

## Done

**Done 2026-11-11.** `.github/workflows/kvm.yml` (dispatched by hand) gives
the job `/dev/kvm` with a udev rule and a `chmod` (a job without it exits
2 before any guest), installs `qemu-system-x86` alone, and runs
experiment 390 with `PODVM_390_WANT_KVM=1`: the run must name the `kvm`
accelerator, which `podvm machine ssh` now prints on stderr. The job fails
where an emulator stays after the run, and it uploads the transcript.
Run 42972037125 at `aa25ab2` is green
([the transcript](../experiments/results/machine-ssh-kvm.txt)): the
guest booted under KVM, ran the command with exit 0 as root, passed exit
42 through, and left no process and no run directory. Six earlier runs
found the steps: no `/dev/kvm` mode at first; `qemu-user-static` failed
to install; and the guest's dropbear refused the key with "Permission
denied (publickey)", because a runner builds as a user and the base
archive gave `/root/.ssh` that uid (`podvm_base` writes 0:0 now;
`PODVM_MACHINE_SSH_LOG=DEBUG1` gave the reason).

# T-1702: The TCG guest of `podvm-workload` prints no console byte in 600 s on the cloud host

**Source:** T-1687 (its run of 2026-11-10);
  `crates/podvm-measure/src/workload/drive.rs:352`
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

On the cloud host (no `/dev/kvm`), clause 3 of
`podvm-workload` boots the pinned Alpine kernel under TCG with
the assembled initramfs. qemu did not return in 600 s, and
the console file held 0 bytes, so no row of the guest
column was measured. A kernel under TCG prints its first
console lines in seconds, so 0 bytes points at the console
path or the qemu arguments, not at speed.

## Premise

The flags are `-M pc,acpi=off -m 256 -nographic -no-reboot
-accel tcg,thread=multi`; the kernel hash matched the pin and
the assembly was 11883652 bytes. qemu 8.2.2 of Ubuntu 24.04.

## Approach

Boot the same kernel and initramfs by hand with the same
flags and `console=ttyS0`, read what reaches the console, and
compare with the arguments the tool passes. Fix the cause in
the tool, or record the host condition that stops it.

## Decision

A guest that prints nothing is not a slow guest; the tool
names the cause before it reports a time.

## Prove

`PODVM_WORKLOAD_MIRROR=1 podvm-workload` on the cloud host
reaches the done marker of clause 3 and exits 0, or the
entry records the measured host condition that stops it.

## Correction

**Filed 2026-11-10.** Found by the run of T-1687.

## Done

**Done 2026-11-10.** The Premise is refuted: the arguments were right,
and the kernel printed to the console within seconds (16536 bytes in
2.7 s by hand). Two faults of the tool hid it. The guest init ended with
`poweroff -f`, which halts on `-M pc,acpi=off` ("Power off not
available: System halted instead"), so qemu never exited. And
`run_capture` ran qemu through `wait_with_output`: at the timeout it
dropped the pipes, so the console was lost, and it never killed qemu.
Now `GUEST_INIT` ends with `reboot -f`, so `-no-reboot` makes qemu exit 0,
and the new `run_to_file` writes the console to `boot.log` and kills the
child at the timeout. `cargo test -p podvm-measure` exits 0 with
`guest_init_resets_so_qemu_exits` and
`run_to_file_keeps_output_and_kills_at_the_timeout`.
`PODVM_WORKLOAD_MIRROR=1 podvm-workload` exits 0 in 61 s: the guest
reaches `VMR-BENCH-DONE`, clause 4 has 14 driven and 0 mismatches, and
the checksums of all four classes agree ([the run](../experiments/results/tcg-workload-spread.txt),
[the hand boot](../experiments/results/tcg-console-1702.txt)). No qemu
process stays after either run.

# T-1717: Study incus without root and without heavy overhead, with errand-box, before an incus back end

**Source:** PodUtils/podcrates#56 (COORD-031, the owner's word relayed:
  "it must also support/have an adapter for incus since it seems
  to be best of both microvms & dockers"); the owner's answer of
  2026-11-11 ("study, and also study this:
  https://github.com/Azathothas/errand-box, find a way to use
  incus without root & without heavy overhead/performance loss,
  then api back end"); Azathothas/errand-box at `1ad2fc1`:
  `errand-box:docs/research/rootless.md`,
  `errand-box:docs/research/engines.md` and
  `errand-box:docs/incus-backend.md`
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

podvm has no incus back end. incus runs system containers
and virtual machines through one REST API on a local Unix
socket, but a standard install runs `incusd` as root, and
podvm's hosts give no root. Nobody in PodUtils has measured
which route gives incus to podvm without root and without a
large loss of speed.

## Premise

errand-box runs each session in an incus system container
and ranks two readings of "without root"
(`errand-box:docs/research/rootless.md`, measured 2026-10-23
on its own host). A: root prepares the host once (subordinate id ranges,
a btrfs file system that the user owns, cgroup delegation);
then `incusd` 6.0.6 runs as root of a user namespace at 0.6
to 1.1 times the cost of runc. It loses the disk quota, a new
instance takes 0.9 to 3.5 s in place of 0.5 s, a proxy device
cannot reach the host's loopback, and on cgroup v1 with no
init the cgroup delegation is a root step at each boot. B: no
root ever: no subordinate ids, so only a guest kernel without
KVM (User-Mode Linux or QEMU's TCG) runs a full system, at 18
to 53 times the cost of runc. Its scripts are under
`errand-box:research/experiments/rootless/` and
`errand-box:research/experiments/x92-rootless-cost.sh`. The cloud host of
2026-11-11 is uid 0 in a container, with user namespaces and
`/dev/fuse` and no `/dev/kvm` (`sh scripts/cloud-setup.sh`).
T-1656 is podvm's own route of reading B.

## Approach

Record errand-box as a reference issue (label `reference`,
`docs/methodology/references.md`). Read its rootless pages,
its survey (`errand-box:docs/research/rootless-survey.md`) and
its incus client specification. Rerun route A on the cloud host as a
non-root user with a new tracked script under `experiments/`
(`NNN-incus-rootless.sh`) and a pinned incus
version, and measure the cost against runc and against
podvm's own rungs with the workloads of errand-box. Answer
each question with a measurement or a cited source: the host
preconditions of each route (subordinate ids, `newuidmap`,
cgroup delegation, a storage pool that the user owns,
`/dev/kvm` for a VM); whether podvm can start and own an
`incusd` of its own (a static build of `incusd` and liblxc
with zig) or must use the socket of an `incusd` of the host;
which docker verbs and OCI image pulls map to the API; and
which part is generic enough to be a podcrates crate.

## Decision

The study gives the form of T-1718; it adds no code to the
podvm binary. A route that needs root at run time is recorded
with its reason and is not taken without a ruling of the
owner. podvm stays one static binary.

## Prove

A new page `incus.md` under `docs/` names each route with
its preconditions and its measured cost, and cites each
source. The new script `NNN-incus-rootless.sh` under
`experiments/` exits 0 (or 2 with the missing precondition
named) and saves its run under `experiments/results/`.

## Correction

**Filed 2026-11-11.** From the owner's ask relayed by COORD-031 and the
owner's answer in the session of 2026-11-11.

## Done

**Done 2026-11-11.** [The page](../docs/incus.md) names each route with
its preconditions, its measured cost and its source, and gives the form of
T-1718. `sh experiments/433-incus-rootless.sh` (with
`PODVM_433_MIRROR=1`) exits 0 on the cloud host as root, which is the
one preparation of route A ([the run](../experiments/results/incus-rootless.txt)):
incusd 6.0.0 runs as root of a user's own user namespace with the `lxc`
driver; an alpine 3.21 system container runs `/sbin/init` as an id of the
user's range; rootless incus runs the busybox workloads at 0.6 to 0.9
times podvm's userland rung (spawn 100 against 150 ms, files 450 against
730, cpu 1310 against 1510); a new instance takes about 0.4 to 0.5 s and
`exec` 48 ms; podvm's extracted OCI image imports as a unified tarball and
launches with its command as PID 1 (`lxc.init.cmd`) and its files owned by
root; nothing of the user or its id range stays. errand-box is reference
issue 251. The incus VM driver was not tried: no `/dev/kvm` here.

# T-1718: An incus back end: podvm drives incus over its REST API

**Source:** PodUtils/podcrates#56 (COORD-031); the owner's answer of
  2026-11-11 ("... then api back end"); T-1717
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The owner asks that podvm support incus or have an adapter
for it, "since it seems to be best of both microvms &
dockers". podvm has no back end that drives an incus.

## Premise

incus serves its REST API on a local Unix socket. podvm is one
static musl binary and speaks HTTP through `pod-proxy`. A
client of the incus API is generic: podcrates offers to hold
it as a crate (COORD-031). T-1717 gives the route and its
preconditions.

## Approach

Build the back end that T-1717 names: a client of the incus
REST API (the generic part as a podcrates crate, asked by a
COORD issue `(to podcrates)`), the map of `run`, `exec`, `ps`,
`stop`, `rm` and `pull` to incus instances and images, the
selection of the back end by the ladder and by
`--podvm-tier`, and a refusal by name where the host gives no
usable incus. The banner and `.EnteredRung` name the back end.

## Decision

incus is a host service, as KVM is: podvm drives it and never
installs it as root. Each refusal names the missing
precondition.

## Prove

`cargo test --workspace` exits 0 with tests of the client
against a stand-in API server on a Unix socket. An experiment
runs the pinned image `ALPINE` of
`experiments/356-no-chroot-rung.sh` through the back end where
an `incusd` answers, and refuses by name where none answers.

## Correction

**Blocked 2026-11-11.** Blocker: T-1717 (the study gives the route).
Clears when: T-1717 is done and names the route.

**Unblocked 2026-11-11.** T-1717 is done. The route is A of
[the page](../docs/incus.md): podvm drives an incusd of the user's own
namespace (the host's package, prepared by root one time), imports its own
OCI pulls as unified tarballs, and names the image's command with
`lxc.init.cmd`. The page gives the map of the verbs and what the first
back end refuses (`-p`: no network in the namespace).

The effort of T-1718 was XL before call C17 of `docs/decisions.md`; pod-todo has no XL.

## Done

**Done 2026-11-11.** `run --podvm-tier=incus` drives an incusd over its
REST API (`crates/podvm/src/incus/`: the client on the unix socket, the
image, the run). `cargo test -p podvm incus::` exits 0 with seven tests,
one of them a whole run against a stand-in API server on a unix socket
(import, alias, create with `lxc.init.cmd`, push, start, exec, delete).
`PODVM_434_MIRROR=1 sh experiments/434-incus-tier.sh` exits 0 as root on
the cloud host ([the run](../experiments/results/incus-tier.txt)) with
the pinned image `ALPINE` of experiment 356: against the incusd of route
A, a run gives `hello` and 0, exit 7, stdout and stderr apart, piped
stdin, `-e` and `-w`; PID 1 is podvm's hold and `/etc/shadow` is `0:42`
as the image intended; a second run takes the image from incus; `create`
and `exec` refuse the tier by name; no instance and no process stays; and
with no incusd a run exits 125 and names the socket. The first run found
two defects, both fixed: incus refuses a `creation_date` of 0, and an
image with each file owned by 0 lost the image's groups. The rest of the
Approach (the verbs of an instance that stays, `-v`, the ladder's
selection, `.EnteredRung`) is T-1722. podcrates takes the client as
`pod-incus` (PodUtils/podcrates#63, COORD-038 of podcrates).

# T-1722: The incus back end: the verbs of an instance that stays, volumes and the ladder

**Source:** T-1718 (the rest of its Approach); docs/incus.md
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** L
**Status:** done

## Problem

`--podvm-tier=incus` serves `run` only. `create`, `start`,
`exec`, `ps`, `stop`, `rm` and `logs` refuse it, `-v` is
refused, the ladder never selects incus, and
`system info --format '{{.EnteredRung}}'` does not name it.
Each run pushes the podvm binary into the new instance.

## Premise

T-1718 has the client, the image and the run
(`crates/podvm/src/incus/`). incus keeps an instance by name,
and a container record of podvm can name it. A `disk` device
maps a host path into an instance.

## Approach

Keep a container record for an incus instance (its name and
its socket); map `create`, `start`, `exec`, `ps`, `stop`,
`rm` and `logs` to the API; map `-v HOST:GUEST[:ro]` to a
`disk` device; let the ladder select incus where an incusd
answers and the user asks for it by configuration; name the
tier in the banner and in `.EnteredRung`; push podvm one time
for each image, not for each run.

## Decision

incus stays a host service: podvm never installs or starts
it as root.

## Prove

`cargo test -p podvm incus::` exits 0 with tests of each verb
against the stand-in server, and a new experiment runs
`create`, `exec`, `ps`, `stop`, `rm` and `-v` against the
incusd of route A with exit 0.

## Correction

**Filed 2026-11-11.** From the rest of T-1718.

## Done

**Done 2026-11-13.** The last clauses. `tier = incus` in `$store/config`
selects the incus tier with no flag (`crates/podvm/src/incus/config.rs`):
`run` and `create` take it where the incusd socket is there and say so
where it is not; `system info` names `incus` as `.EnteredRung`; `exec`,
`start`, `stop`, `rm`, `logs` and `inspect` take it for a name that is an
instance of podvm in incus; an explicit `--podvm-tier` wins, and another
value of `tier` is refused by name. `logs -f` follows the log until the
instance stops, and `inspect` gives the state as JSON or through
`--format` (`crates/podvm/src/incus/inspect.rs`). incus 6.0.0 of the
Ubuntu archive is installed on the cloud host (the ruling "Missing
tools"). `PODVM_434_MIRROR=1 sh experiments/434-incus-tier.sh` exits 0
([the run](../experiments/results/incus-tier.txt)) with the new clause 13
and each earlier clause. `cargo test -p podvm` passes with the tests of
`incus::config`.

**Partial 2026-11-12, the volumes.** `run --podvm-tier=incus -v
HOST:GUEST[:ro]` adds one `disk` device for each volume to the new
instance (`disk_devices` of `crates/podvm/src/incus/run.rs`; `:ro` sets
`readonly`). The test of the stand-in server checks the devices of the
request. `sh experiments/434-incus-tier.sh` exits 0 against the incusd of
route A on this host with the new clause 11: the command reads a host file,
its write is on the host after the run, and a `:ro` volume refuses a write
([the run](../experiments/results/incus-tier.txt)). Under route A the root
of the instance is an id of the user's range, so a directory of mode 0755
of the user is read-only to it (measured: "Permission denied"); the clause
uses mode 0777. Clause 8 now takes the refusal of `exec` that names each
tier of `run` only.

**Partial 2026-11-12, podvm in the image.** The unified tarball carries
podvm at `/.podvm/podvm` (owner 0, mode 0755), and the alias names the
first 16 hex digits of podvm's own SHA-256 (`podvm-HEX16-PODVM16-v3`), so
a new podvm build imports the image again; a run pushes only its stdin.
The tests of `crates/podvm/src/incus/tests.rs` hold the alias, the entry
of the tarball and a run with no push of podvm. Experiment 434 now runs
the release binary and exits 0: PID 1 is still `/.podvm/podvm
guest-hold`, and a run of `true` takes 1 070 ms (median of 5). 

**Partial 2026-11-12, the verbs.** Decision (this session's call,
AGENTS.md section 10): incusd keeps the state of an instance that stays,
and podvm keeps no record of it in its container table, whose `reconcile`
calls a running record with no launcher lock dead. `run -d` and `create`
with the tier make an instance labelled `user.podvm=1` (and the image in
`user.podvm.image`) whose init is `podvm guest-hold --run`: it runs the
command of `/.podvm/cmd`, appends its output to `/.podvm/log`, writes its
code to `/.podvm/exit`, passes a stop signal to it, and ends with its code
(`crates/podvm/src/incus.rs`, `crates/podvm/src/incus/verbs.rs`). `ps
[-a]`, `start`, `stop`, `rm [-f]`, `logs` and `exec` with
`--podvm-tier=incus` call the API on the instance of that name and refuse
an instance with no label. `sh experiments/434-incus-tier.sh` exits 0 with
the new clause 12 (each verb, an instance that stops when its command
ends, nothing left) and clause 8 rewritten (`exec` on no instance exits
125 and names it). Remaining: the ladder selects incus by configuration and
`.EnteredRung` names it; `logs -f` and `inspect`.

# T-1723: The microVM of run: the microvm machine, a shared /workspace and an interactive stdin

**Source:** T-1656 (the rest of its Approach); issue 168
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** L
**Status:** done

## Problem

`run --podvm-tier=machine` boots `-M pc,acpi=off` with the
Alpine `vmlinuz-virt`. `-v` is a copy whose writes do not
return to the host, and a terminal on stdin does not reach the
guest.

## Premise

T-1656 measured that Firecracker's CI kernel 6.1.102 for `-M
microvm` did not boot under TCG; under KVM it was not tried.
The 9p modules are not in `initramfs-virt` (only in the xz
modloop), and `virtiofs.ko` needs a `virtiofsd` on the host.
`kvm.yml` (T-1682) gives a runner with `/dev/kvm`.

## Approach

Under KVM, try `-M microvm` with a pinned virtio-mmio kernel
and count twenty boots for each flag set, as issue 168 does;
keep `-M pc` where it does not boot. Take the 9p modules of
the modloop (by SHA-256) and mount `-v` with `-virtfs local`,
so that a write returns to the host. Carry a terminal on a
virtio-console port in raw mode.

## Decision

The TCG leg stays `-M pc` until a kernel boots there twenty
times in a row.

## Prove

`cargo test -p podvm` exits 0, and `sh experiments/412-microvm-boot.sh`
exits 0 in `kvm.yml` with new clauses: twenty boots of `-M
microvm` under KVM, a write in a `-v` directory that is on the
host after the run, and a command that reads a terminal.

## Correction

**Filed 2026-11-11.** From the rest of T-1656. Measured 2026-11-12: the
9p modules of the pinned kernel 6.12.110-0-virt are `netfs`, `9pnet`,
`9pnet_virtio` and `9p` (their `modules.dep` of the modloop), and none is
built in. Measured the same day under TCG on this host, with
`sh experiments/440-microvm-machine.sh` (20 boots of each machine; [the
run](../experiments/results/microvm-machine.txt)): the pinned kernel
boots `-M microvm` (ACPI on: with `acpi=off` it found no device and hung)
17 of 20 times, and the 3 others hang before the first console line;
`-M pc,acpi=off` boots 20 of 20 times, 3.0 s each. On microvm the NIC is
`virtio-net-device` and a volume `-fsdev` with `virtio-9p-device`
(`-virtfs` asks for PCI), and only the first serial port is wired, so
stderr apart needs a virtio console there. The TCG leg stays `-M pc`.

## Done

**Done 2026-11-12.** `kvm.yml` run 43210445190 (at `e8f5d131`, hosted
`ubuntu-latest` with `/dev/kvm`): experiment 412 exits 0 under KVM with
clause 6 (the 9p volume, also in run 43205956782) and clause 11 (the
terminal), and experiment 440 exits 0: `-M microvm` boots 20 of 20 times
with the two virtio devices and a 9p write each time, 1 808 ms a boot;
`-M pc,acpi=off` boots 20 of 20 times, 561 ms a boot. Decision (this
session's call, AGENTS.md section 10): `run --podvm-tier=machine` keeps
`-M pc,acpi=off` on both legs. Under KVM the pinned kernel boots it three
times faster than microvm, and under TCG microvm hangs in 3 of 20 boots;
microvm would also need a virtio console for stderr apart. `cargo test -p
podvm` exits 0.

**Partial 2026-11-12, the volumes.** `-v HOST:GUEST[:ro]` of `run
--podvm-tier=machine` is a 9p mount of the host directory. podvm reads
the four modules out of the pinned modloop (its SHA-256 is a fact of
`alpine-netboot`) with its own reader of squashfs and xz
(`crates/podvm/src/machine/guest/squashfs.rs`; its tests read a fixture of
`mksquashfs`, and the pinned modloop gives the same bytes as
`unsquashfs`). The guest loads them and mounts each tag before the `cd`
(`podvm guest-9p`, `crates/podvm/src/machine/guest/vol.rs`); the host
passes `-virtfs local,...,security_model=none` (`readonly=on` for
`:ro`). A failed mount ends the run with 125 before the command. A kernel
of `PODVM_MACHINE_KERNEL` keeps the copy, and the run says so. `sh
experiments/412-microvm-boot.sh` exits 0 under TCG on this host with the
new clause 6: the guest reads a host file with `-w` in the volume, its
write is on the host after the run, and a write in a `:ro` volume fails
with EROFS ([the run](../experiments/results/microvm-boot.txt)).
Remaining: `-M microvm` under KVM (twenty boots), a terminal on stdin,
and the run of 412 in `kvm.yml`.

**Partial 2026-11-12, the terminal.** `run -t --podvm-tier=machine` gives
the emulator a `virtconsole` on a unix socket of the run
(`crates/podvm/src/machine/guest/tty.rs`; `virtio_console` is built into
the pinned kernel). In the guest `podvm guest-tty` starts a session with
`/dev/hvc0` as its controlling terminal, sets the size of the host
terminal, and execs the command; on the host podvm puts its terminal in
raw mode and copies stdin and stdout through the socket. `sh
experiments/412-microvm-boot.sh` exits 0 under TCG with the new clause 11:
under script(1) with 33 rows and 99 columns, `[ -t 0 ]` and `[ -t 1 ]`
are true, `stty size` gives `33 99`, a typed line reaches `read`, and the
exit code 6 comes back. Remaining: `-M microvm` under KVM (twenty boots,
experiment 440 in `kvm.yml`) and the run of 412 in `kvm.yml`.

# T-1727: Take podcrates v0.21.0: `pod-incus` replaces the incus client of T-1718

**Source:** PodUtils/podcrates#65 (COORD-040 of podcrates) and #63
  (COORD-038), messages from a peer session; podvm#253
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

podvm keeps its own client of the incus REST API
(the module `incus::client` of T-1718, gone since this entry). podcrates
`v0.21.0` (`ada3699`) has `pod-incus`, made from podvm's ask.

## Premise

The README of `pod-incus` maps each call of podvm's client to a
call of the crate. The crate reads no variable of the
environment: `Settings::new(INCUS_DIR.join("unix.socket"))`.

## Approach

Move each pin of `pod-*` to `v0.21.0`, replace `incus/client.rs`
with `pod-incus` (`server`, `alias`, `import_image`,
`create_alias`, `create_instance`, `start`, `put_file`,
`exec_recorded`, `stop`, `delete_instance`), keep the image
conversion and the run in podvm, and answer on podcrates#63
with the commit.

## Decision

podvm keeps no second client of the incus API.

## Prove

`cargo test -p podvm incus::` exits 0 against the stand-in
server, and `PODVM_434_MIRROR=1 sh experiments/434-incus-tier.sh`
exits 0 as root on the cloud host.

## Correction

**Filed 2026-11-11.** From PodUtils/podcrates#65.

## Done

**Done 2026-11-11.** Each `pod-*` pin is at `v0.21.0`; the incus tier calls
`pod-incus` (`server` and `Server::check`, `alias`, `import_image`,
`create_instance`, `put_file`, `start`, `exec_recorded`, `stop`,
`delete_instance`) on a runtime of one thread (`crates/podvm/src/incus/run.rs`),
and `incus/client.rs` is gone. `cargo test -p podvm incus::` exits 0 against
the stand-in server (which answers as incus does: the extensions of
`NEEDED`, the logs under `logs/exec-output/` of the instance), and
`PODVM_434_MIRROR=1 sh experiments/434-incus-tier.sh` exits 0 as root on the
cloud host ([the run](../experiments/results/incus-tier.txt), each of its
ten clauses). The `--strict` gap of the loader family names T-1651 now: a
static child takes the native-static rung where it arms (T-1724).

# T-1728: Without `-t`, a command of the machine tier writes to a terminal

**Source:** Found by T-1654 (experiment 410 compares `ls /workspace` across the profiles)
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

The guest init ran the command with stdout on the console
(`/dev/ttyS0`) and stderr on `/dev/ttyS1`. A serial port is a
terminal, so `[ -t 1 ]` was true without `-t`, and busybox `ls`
wrote colour escapes into podvm's stdout. docker without `-t`
gives a pipe, and a script that reads the output gets other
bytes.

## Premise

`crates/podvm/src/machine/guest/init.rs`, `script`: `exec
2>/dev/ttyS1` and the command with no redirection of stdout.
Measured on 2026-11-11: `run --podvm-tier=machine alpine sh -c
'[ -t 1 ] && echo tty'` printed `tty` with podvm's stdout on a
pipe.

## Approach

The init makes two FIFOs. The command writes to them, and one
`cat` copies each FIFO to its serial port. The script waits for
both copies before it writes the exit marker.

## Decision

A FIFO and not a pipe of the shell: the shell keeps the exit
code of the command without `pipefail`.

## Prove

`sh experiments/412-microvm-boot.sh` exits 0 with clause 10: `[ -t 1 ]` and `[ -t 2 ]` are false in the guest, and `ls` writes no colour escape

## Correction

**Filed 2026-11-11.** Found while T-1654 compared the output of the
profiles.

## Done

**Done 2026-11-11.** `PODVM_412_MIRROR=1 sh experiments/412-microvm-boot.sh`
exits 0 under TCG on the cloud host, each of its ten clauses
([the run](../experiments/results/microvm-boot.txt)): clause 10 reads
`out-pipe` and `err-pipe`, and clause 7 still reads stderr apart and exit
4. The unit test `the_command_writes_to_fifos_not_to_a_terminal` checks
the script.

# T-1736: A test of the refusal of `windows --dos` with no DOS base image

**Source:** row 1529 of the refactor plan (`refactor/recon-c.md` on the
  branch `archive/podbox-worktree`), filed by [T-1673](gate.md)
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** done

## Problem

The DOS flavor of the Windows verbs refuses by name where no DOS base
image exists (`crates/podvm/src/windows/dos.rs:58-66`). No test proves
the refusal, so a change can turn it into a boot with no base, or a
fetch.

## Premise

The other refusals of the Windows verbs have tests
(`crates/podvm/src/lifecycle/tests/fields_and_copy.rs`, and the test of a
missing named image in `crates/podvm/src/windows/mod.rs`).

## Approach

Point the base cache at an empty directory and run the DOS path. Assert
the exit code 125, the name of the missing base in the message, and
that no file is fetched or written.

## Decision

The test needs no QEMU and no network.

## Prove

`cargo test -p podvm a_dos_run_with_no_base_is_refused_by_name` exits 0.

## Done

**Done 2026-11-11.** `cargo test -p podvm a_dos_run_with_no_base_is_refused_by_name`
exits 0 (`crates/podvm/tests/dos_refusal.rs`): `podvm windows run --guest
dos ver` with `PODVM_DOS_BASE` at a path that does not exist exits 125,
names the path in "no DOS base image at", and leaves its `HOME` empty. The
test runs the binary, so the variable stays in the child.

# T-1750: `exec` into a running guest of the machine tier

**Source:** PodUtils/podcrates#124 (the asks of podswarm to podvm, a message from a
  peer session; PodUtils/podcrates#107 is its tracker), ask V-6
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** L
**Status:** open

## Problem

podswarm keeps one warm sandbox for each slot and runs each attempt with
`exec`. The machine tier has no warm form, so each attempt boots a guest.

## Premise

The machine tier boots one guest for each `run` and reads the exit code
through markers (T-1656). `run -d` and `exec` exist for the rootfs rungs
(`pod-supervise`), not for a guest.

## Approach

`run -d --podvm-tier=machine` keeps the guest and its emulator under the
launcher. `exec` sends the command over one host channel (a
`virtio-serial` port on a Unix socket) to an agent in the guest init, and
reads its exit code back.

## Decision

The channel is the one of T-1751; one design serves both.

## Prove

`cargo test -p podvm` exits 0, with a test that `exec` into a running
guest gives the exit code of its command (under TCG on the cloud host)

# T-1751: The guest plan of shield `guest`: no host file in the guest, no network card, one serial channel

**Source:** PodUtils/podcrates#124 (the asks of podswarm to podvm, a message from a
  peer session; PodUtils/podcrates#107 is its tracker), ask V-7
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

podswarm's shield `guest` requires that no file name, file content, argv
value or environment value of a run appears in a host file or in the argv
of a host process. The machine tier uses 9p volumes, a run archive on the
host and QEMU user-mode network.

## Premise

QEMU can boot with `-nic none`, one `virtio-serial` port on a `chardev`
Unix socket in a per-run directory of mode 0700, and one raw scratch
disk. `-overcommit mem-lock=on` needs `RLIMIT_MEMLOCK`.

## Approach

A plan of the machine tier for this shield: the run data go into the
guest through the `chardev` socket only, no 9p, no virtiofs, no host
stdin file, `-nic none`, a raw scratch disk, and `mem-lock=on` where the
limit allows it.

## Decision

podvm runs the canary tests itself. podswarm's tests E-19 to E-21 are its
adoption trial, not conditions of this entry.

**2026-11-13.** The run comes as a tar bundle on podvm's stdin
(`--podvm-sealed`): podvm's argv then holds no value of the run. The
guest gets the bundle as the first stream of `pod-link` over the one
port `podvm.ch`. The guest's root is RAM. podvm adds no raw scratch disk:
a disk that the guest writes is a host file that holds values of the
run, and the shield forbids that.

## Prove

`sh experiments/445-sealed-guest.sh` exits 0 as root: a run with a canary in a file name, a file body, an argv element and a variable leaves no canary in the argv or the environment of a host process, in the per-run directory or in the state of podvm, and `/sys/class/net` in the guest holds `lo` only; `cargo test -p podvm` exits 0

## Correction

**2026-11-13.** The Prove named a test of `cargo test -p podvm`. The
canary test needs QEMU and root (it reads `/proc/PID/environ` of each
process), so it is experiment 445. `cargo test -p podvm` holds the
bundle, the fragment of the run and the refusals that name no value.

## Done

**Done 2026-11-13.** podswarm's half (ask V-7). `run
--podvm-tier=machine --podvm-sealed IMAGE` reads the bundle from stdin
(`.podvm-run.json` with `argv`, `env` and `workdir`, and the files of the
guest; `crates/podvm/src/machine/guest/sealed.rs`) and keeps it in
memory. The initrd on the host holds the image and a generic init; the
guest has no NIC, no 9p volume and no stdin file. `podvm guest-channel
--sealed` fetches the bundle as the first stream, unpacks it into the
guest's root and writes the fragment of the run, which the init sources
after the image's command. A command, `-e`, `-w`, `-v`, `-t` and
`--network broker` are refused with no value of the bundle in the words.
`-overcommit mem-lock=on` is set where `RLIMIT_MEMLOCK` holds the guest's
memory and 64 MiB; elsewhere podvm names the limit (8 MiB on the cloud
host). Experiment 445 passes its 6 clauses on the cloud host (TCG), as
root: the guest reads the file by its canary name, the variable, the argv
and the working directory, and exits 7; during the run (with a control
that reads the emulator's environment) and after it, no canary is in the
argv or the environment of a host process or in a file name or body
under the store and TMPDIR; `/sys/class/net` holds `lo` only; no
emulator stays. A first run of clause 6 matched a process with
`qemu-system` in its argv that a second look did not find; the clause now
names each match. The tests `the_bundle_names_the_run_and_its_files` and
`a_bad_bundle_refuses_and_names_no_value` hold the bundle.

**The first part, 2026-11-13.** The part that PodUtils/podrunner#83 asks for, so
that podrunner's `microvm` kind can run an untrusted job: `--podvm-net
none` boots the guest with `-nic none` and no NIC module, and
`--podvm-channel HOST_SOCKET:127.0.0.1:PORT` gives the guest one
`virtio-serial` port, `podvm.ch`, on a chardev socket of the run
(`crates/podvm/src/machine/guest/channel.rs`). The socket is in a
directory of its own, mode 0700: QEMU's chardev server takes its first
client, and no other user may be that client. In the guest, `podvm
guest-channel PORT` listens on the loopback port and opens one stream of
`pod-link` for each connection; on the host, podvm dials `HOST_SOCKET` for
each stream. Both flags are refused on each other tier, and `--podvm-net
none` with `--network broker` is refused. In a TCG guest on the cloud
host, `ip -o link` shows `lo` only, and two connections to
`127.0.0.1:8080` reach a Unix socket of the host and its answers, within
the bound; no emulator stays after the run. The test
`the_flag_names_a_host_socket_and_a_port_of_the_loopback` holds the parse
and the arguments of the emulator.


# T-1776: Each file of a guest of the machine tier belongs to root

**Source:** PodUtils/podrunner#81 (COORD-021 of podrunner, for PodUtils/podrunner@T-037)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

In a guest of the machine tier, each file of the image belongs to root:
`/home/runner` of podrunner's runner image is `root:root 750`, so the user
`runner` cannot run `/home/runner/run.sh` (exit 126). The rootfs rungs keep
the owners.

## Premise

The base archive (`newc`) writes 0 in the owner and the group of each
entry. The extraction keeps the ids of its own process and records the
image's ids in its sidecar, `.meta.jsonl` beside the rootfs.

## Approach

Write the owner and the group of the sidecar in each entry of the base
archive, and bump the format of the archive so that no cached archive is
used again.

## Decision

An entry that the sidecar does not name stays root's.

## Prove

`cargo test -p podvm the_base_archive_keeps_the_owners_of_the_image` passes, and a guest of the machine tier shows the owner of the image

## Done

**Done 2026-11-13.** `Newc::owners_from` in
`crates/podvm/src/machine/guest/cpio.rs` reads the sidecar, and each entry
of the base archive carries its uid and gid (`BASE_FORMAT` 3 in
`crates/podvm/src/machine/guest.rs`). In a TCG guest on the cloud host,
`run --rm --podvm-tier=machine alpine stat -c "%u:%g" /etc/shadow` prints
`0:42` (the group `shadow` of the image; `0:0` before), within the bound,
and no emulator stays after the run. The test
`the_base_archive_keeps_the_owners_of_the_image` holds the header fields.

# T-1777: The incus tier takes no stdin for an instance that stays

**Source:** PodUtils/podrunner#85 (COORD-025 of podrunner, for PodUtils/podrunner@T-105)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

podrunner's `incus` kind runs one job in one instance of a name that it
chooses, and gives the runner its credential files through stdin, never in
argv or the environment. `run -d --name` of the incus tier makes the named
instance, but takes no stdin; the foreground `run` takes stdin, but has a
random name and no label of podvm.

## Premise

The foreground run pushes stdin to `/.podvm/stdin` (mode 0600, owner 0).
`detached` pushes only `/.podvm/cmd`, and `guest-hold --run` starts the
command with its stdin on `/dev/null`.

## Approach

With `-i`, `run -d` pushes the piped stdin to `/.podvm/stdin` before the
start, and `guest-hold` gives the file to the command as its stdin, then
removes it.

## Decision

Without `-i`, `run -d` pushes no stdin, as docker's `run -d` reads none.

## Prove

`sh experiments/434-incus-tier.sh` exits 0, with its clause 14: the command reads the piped stdin, the file is gone once the command holds it, and the recorded command does not carry it

## Done

**Done 2026-11-13.** `-i` of `run` sets `Opts::interactive`, which goes to
`Detach::stdin` (`crates/podvm/src/incus/verbs.rs`): `detached` pushes the
piped stdin to `/.podvm/stdin` (`STDIN` of `crates/podvm/src/incus.rs`),
root's, mode 0600, before the start; `start_cmd` of `guest-hold` opens it
as the command's stdin and removes it once the command holds it. On the
cloud host, with the incusd of route A, clause 14 of experiment 434 gives
`logs` = `sekret434 gone`, and `inspect --format '{{.Cmd}}'` does not hold
the secret. The parity row of `-i` for `run` is Native now.


# T-1778: The channel of a guest gets no request through a loopback proxy variable

**Source:** PodUtils/podrunner#87 (COORD-027 of podrunner, for PodUtils/podrunner@T-037)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

podrunner's `microvm` kind runs `--podvm-net none --podvm-channel
SOCK:127.0.0.1:3128` with two 9p volumes and `-e
http_proxy=http://127.0.0.1:3128`. The official runner in the guest
reaches no proxy, and the host socket logs no connection. The same run
with `curl -x http://127.0.0.1:8080` and no volume reaches the socket.

## Premise

podrunner names the 9p volumes as a possible cause. On the cloud host,
the run of form 2 with `wget` gives "can't connect to remote host
(10.0.2.2): Network unreachable": the rewrite of T-1665 turns the
loopback host of each proxy variable into the gateway 10.0.2.2 of the
user-mode network, and a guest with no NIC has no gateway. The 9p
volumes are not the cause.

## Approach

With `--podvm-net none`, the machine tier keeps the proxy variables as
the caller gives them: the guest's loopback is where the channel
listens.

## Decision

With a NIC, the rewrite of T-1665 stays.

## Prove

`sh experiments/444-guest-channel.sh` exits 0: the channel alone, the channel with two 9p volumes and a loopback proxy variable for a user that is not root, the rewrite with the default network, and no emulator that stays

## Done

**Done 2026-11-13.** `machine_tier` (`crates/podvm/src/run/machine.rs`)
calls `rewrite_env` only where the guest has a NIC. Experiment 444 on
the cloud host (TCG) passes its 4 clauses: in clause 2, the user `runner`
fetches `http://broker.podrunner.test/two` through `http_proxy`, the host
socket logs the full URL, and the `:ro` volume reads.

# T-1779: The incus tier takes `--podvm-net none` and `--podvm-channel`

**Source:** PodUtils/podrunner#88 (COORD-028 of podrunner, for PodUtils/podrunner@T-109); PodUtils/podrunner#90 (COORD-030)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

podrunner's `incus` kind runs one instance for each job. A `trusted` job
needs its egress held: each connection must pass podrunner's egress
module. An instance has the NIC of the default profile, and the incus
tier refused `--podvm-net none` and `--podvm-channel`.

## Premise

A device of the instance masks a device of the profile with the same
name; a device of type `none` gives no device. An incus `proxy` device
with `listen=tcp:127.0.0.1:PORT`, `connect=unix:HOST_SOCKET` and
`bind=instance` carries each connection in the instance to a socket of
the host. pod-incus has no update of an instance.

## Approach

`--podvm-net none` masks `eth0` with a device of type `none`; after the
create, podvm reads the expanded devices and, where a NIC of another
name stays, makes the instance again with that name masked too.
`--podvm-channel` adds the device `podvm-channel`. podvm's init of the
instance (`guest-hold`) brings `lo` up.

## Decision

The flags serve `run`, `run -d` and `create`, as podrunner asks.

## Prove

`sh experiments/434-incus-tier.sh` exits 0 as root, with its clause 15: with `--podvm-net none` the instance shows `lo` only, and a run and a `run -d -i` with `--podvm-channel` reach the host socket, which logs each request; `cargo test -p podvm` exits 0

## Correction

**2026-11-13.** The channel was an incus `proxy` device. On podrunner's
hosted runner (Ubuntu 24.04, kernel 7.0, the packaged incus 6.0.0 as
root), the start of the instance failed: "Error occurred when starting
proxy device: ... Failed to safely open namespace file descriptor based
on pidfd" (PodUtils/podrunner#90). The channel is now a `disk` device
that mounts the host socket at `/.podvm/channel.sock`; podvm pushes the
port to `/.podvm/channel.port` before the start, and `guest-hold` binds
`127.0.0.1:PORT` before any command starts, writes
`/.podvm/channel.ready`, and relays each connection to the socket. A
foreground run waits for the ready file before its `exec`. The root of
the instance must be able to connect to the socket file: the caller
gives the file the mode, inside a directory that guards it.

## Done

**Done 2026-11-13.** `crates/podvm/src/incus/net.rs` makes the mask and
the proxy device, and `create` checks the expanded devices; both paths of
the tier (`run_in`, `detached`) use it. `guest-hold` brings `lo` up: the
first run of clause 15 gave "Network unreachable" for 127.0.0.1, as no
init of the instance brought `lo` up. On the cloud host, with the
incusd of route A, clause 15 gives `lo` only with `--podvm-net none`,
`host434` from the host socket for the run and for `run -d -i`, and the
socket logs `/run434` and `/detach434`. The incusd of route A makes no
bridge (its user namespace), so its default profile has no NIC: the mask
of a real NIC of the profile is not measured on the cloud host; the test
`no_nic_masks_the_profile_and_the_channel_is_a_socket_device` holds the
device forms.

**Done again 2026-11-13, after the correction.** Clause 15 of
experiment 434 passes with the relay (socket mode 0666, in a directory
of the test user). By hand, with a packaged incusd 6.0.0 started as root
on the cloud host (kernel 6.18) and `incus admin init --minimal` (a NIC
`eth0` on `incusbr0` in the default profile): a run with no flag shows
`eth0 lo`, and with `--podvm-net none` `lo` only; with the channel, a
run and a `run -d -i` reach the host socket (it logs `/run` and
`/detach`), and `wget` of 1.1.1.1 gets "Network unreachable". The
`proxy` device of beta.12 also worked on that kernel, so the failure of
PodUtils/podrunner#90 is of podrunner's host; the relay needs no
`forkproxy`.

# T-1780: A guest of the machine tier runs a container engine

**Source:** PodUtils/podrunner#92 (COORD-032 of podrunner, for PodUtils/podrunner@T-103)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

podrunner runs `services:`, `docker build` and Docker-in-Docker on a
container engine inside a guest of the machine tier. The guest loads no
module of a bridge, `veth`, netfilter or overlayfs, and the init mounts
no cgroup2: Docker 27.5.1 runs only with the driver `vfs` and with no
network of its own.

## Premise

The pinned kernel (Alpine `linux-virt` 6.12.110) has each part as a
module of its modloop, which podvm pins by its SHA-256; none is built
in. The modules are plain `.ko` files that `finit_module(2)` loads, as
the 9p modules of T-1723.

## Approach

`--podvm-engine` (machine tier): podvm takes the modules of a list and
each module that `modules.dep` of the modloop names for them, in the
order of their symbols, into the store once, and the run archive carries
them with a list; the init loads the list with `podvm guest-net --load`.
The init mounts cgroup2 at `/sys/fs/cgroup` in each guest.

## Decision

The modules load only with the flag: some 30 modules cost each boot.
podvm adds no `iptables` to the guest: the image of the engine brings it,
as podrunner's image does.

## Prove

`sh experiments/447-guest-engine.sh` exits 0: with `--podvm-engine` and no NIC, the static Docker 27.5.1 runs with overlay2 on cgroup2, a container on a bridge network resolves and pings another by its name, and a mapped port reaches its container; without the flag the guest has no bridge module and cgroup2 is mounted

## Done

**Done 2026-11-13.** `crates/podvm/src/machine/guest/engine.rs` names
the modules and resolves their order from `modules.dep`; `ensure` takes
them out of the pinned modloop into `machine/` of the store with a list
(`fetch_modloop` and `modloop_kver` of `kernel.rs`). `crc32c_generic`
comes first: `libcrc32c` has a soft dependency on the algorithm that
`modules.dep` does not name, and its load failed with ENOENT before it.
The init mounts cgroup2 and runs `podvm guest-net --load`, which also
brings `lo` up in each guest that carries podvm: before, a guest with no
NIC and no channel had `lo` down, and a mapped port at 127.0.0.1 was
unreachable. Experiment 447 on the cloud host (TCG) passes its 5
clauses: `info overlay2 2`, a bridge network, `nslookup srv` and `ping
srv` from a second container, `nc 127.0.0.1 8080` gives `mapped`, and no
emulator stays. Experiments 444 and 445 still pass. The test
`the_closure_puts_each_dependency_first_once` holds the order.


# T-1803: The `--report` of a machine-tier run reads as a refusal with no rung, image or exit code

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), O-01 (RUNS-03 second half, MACH-04; verify, "New observation")
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`podvm run --podvm-tier=machine --report rep1.json alpine:3.20 sh -c 'exit 7'`
writes `"rung": null, "image": null, "image_digest": null,
"exit_code": null, "refused": 7`, and no CPU or RSS. A payload that exits
0 gives `"refused": 0`. A machine timeout gives `timed_out: false` and
`timeout_s: null`. The schema says `refused` is null where the payload
ran. The report is schema-valid, so a consumer cannot detect the fault.

## Premise

A guest run returns its code through the `Err` arm of the preparation
(`crates/podvm/src/run/prepare.rs:84`, `return Err(f(...))`, with
`machine_tier` at `crates/podvm/src/run/machine.rs:94`), so `prepared`
(`crates/podvm/src/run/foreground.rs:145`) is never reached and
`Report::end` treats the run as not started.

## Approach

Return a third outcome from the preparation: prepared for the rootfs
path, a guest run with its code, or a refusal with its code. Fill the
report from the parsed options and the acquired record before the
dispatch, set `ran` when the guest payload starts, and set `timed_out`
and the signal from the result of the driver. The CPU and RSS fields of
a guest are `null` with the reason in the schema description.

## Decision

The report describes the run that the caller asked for, on each tier.

## Prove

`PODVM_443_ONLY=machine sh experiments/443-qa-surface.sh` exits 0 on a host with `/dev/kvm`, with rows that a machine run of `exit 7` writes `exit_code: 7`, `refused: null`, `rung` and `image`, and that a machine `--timeout 2` writes `timed_out: true`

## Correction

**Amended, 2026-11-14.** No third outcome of the preparation: a guest
tier notes its run (`report::note_guest`, a slot of the thread, as the
parse of T-1802 notes its command) just before the guest boots, and
`run` gives it to the report after the run; a refusal before that line
stays a refusal.

## Done

**Done 2026-11-14.** `Report::guest` sets the tier as `rung`, the image
and its digest, `timeout_s`, `timed_out` (137 with a timeout, T-1809)
and `ran`, so `end` gives the payload's exit code and signal; the CPU
time and the resident set stay null for a guest, with `measured_by`
naming the tier. The machine and incus tiers note their run
(`run/machine.rs`, `note_guest_run`). By hand under TCG: `exit 7` gives
`rung: machine`, the image and its digest, `exit_code: 7`, `refused:
null`; `--timeout 2 sleep 30` gives `exit_code: 137`, `signal: 9`,
`timed_out: true`, `timeout_s: 2`; an empty sealed bundle stays
`refused: 125`; each is valid against `run-report.v1.schema.json`.
Experiment 443, section machine: 19 pass, 0 fail, with the rows
`machine.report` and `machine.report-timeout`, under TCG on the cloud
host, which has no `/dev/kvm`.


# T-1808: The machine tier accepts `--podvm-confine` and eight hardening flags and drops them

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), F-08 (MACH-01, blocker) and F-09 (RUNS-02, MACH-02; verify items 6a and 8 and V-5)
**Category:** machine
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

`podvm run --podvm-tier=machine --podvm-confine 'read=/tmp;exec=/bin' alpine:3.20 sh -c 'head -1 /etc/passwd'`
prints the line and exits 0; the help says the machine tier refuses the
layer by name. In the guest, `-u 1000` and `-u nosuchuser` run as uid 0,
`--read-only` permits a write to `/etc`, `--tmpfs /mnt/x` makes nothing,
`--ulimit nofile=512:512` leaves 1024, `--security-opt no-new-privileges`
and `--cap-drop ALL` leave `NoNewPrivs: 0` and the full capability set,
`--add-host` adds no line, and `--label a=b` gives `labels: []`. No line
on stderr says so, and plain `--strict` exits 0. The payload runs as root
with each capability where the caller asked otherwise.

## Premise

`machine_tier` (`crates/podvm/src/run/machine.rs:94`) builds the guest
request from argv, env, workdir, the volumes and the machine flags only
(`crates/podvm/src/run/machine.rs:209`). It never reads `o.confine`,
`o.user`, `o.add_hosts`, `o.read_only`, `o.tmpfs` or `o.hardening`, and the
machine path returns before `confine::decide`, which is only on the
rootfs path. The incus tier already refuses its unsupported flags by
name.

## Approach

Refuse each of `--podvm-confine`, `--user`, `--add-host`, `--read-only`,
`--tmpfs`, `--ulimit`, `--security-opt`, `--cap-drop` and `--label` on the
machine tier by name with exit 125, before the boot, unless a later
entry implements it in the guest init with a proof. `--strict` classes
each refused flag. T-1854 adds the test that each option of the run is
honoured or refused on each tier.

## Decision

A refusal now; an implementation in the guest (user, hosts, limits,
no-new-privs) is a separate entry when a caller asks. The QA patch
0003 is a candidate for the refusals, to be read before use.

## Prove

`cargo test -p podvm --bin podvm machine_refuses` exits 0, with a test that each of the nine flags with `--podvm-tier=machine` exits 125 and names the flag before any boot

## Done

**Done 2026-11-14.** `run/machine.rs`: `unhonoured_flags` names each of
the nine flags that a run gives, and `machine_tier` refuses them by name
with exit 125 before `prepare_image`, so before any pull or boot.
`--security-opt no-new-privileges` and `--cap-drop ALL` set one field
of the record, so the message names both. The test
`machine_refuses_each_unhonoured_flag` parses each flag with
`--podvm-tier=machine` and checks the name, and that a run with none of
them names nothing. By hand with the built binary and an image that the
store does not have: each of the nine exits 125 and its first line says
`machine tier refused: the guest does not honour FLAG yet`.
`docs/limits.md` states the rule; the help of `--podvm-confine` already
said that the machine tier refuses it. Experiment 443, section machine,
has the rows `machine.refuse-*`; the section gave 15 pass, 0 fail, with
four guests under TCG, and no emulator stayed. The incus tier is not changed here:
T-1854 tests each option on each tier.


# T-1809: `--timeout` on the machine tier kills 10 s late and exits 125 in place of 137

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), F-10 (RUNS-04, MACH-03; verify item 7)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`podvm run --podvm-tier=machine --timeout 2 alpine:3.20 sleep 30` took
12.09 s and 12.10 s and exited 125 with "the guest gave no exit marker
before the timeout; qemu: ... QEMU waiting for connection". The rootfs
rung exits 137 at 2.07 s. The help says the payload and its tree are
killed with SIGKILL at the bound.

## Premise

`drive` sets the deadline (`crates/podvm/src/machine/guest/drive.rs:43`);
after it, the loop breaks and `wait_bounded` waits a fixed 10 s
(`crates/podvm/src/machine/guest/drive.rs:118`) before the kill, and a
run with no marker returns `EXIT_RUNTIME_ERROR`.

## Approach

At the deadline, kill the process group of the emulator and return 137
with the reason `wall`; keep the bounded wait only for the reap after the
kill. Name the timeout on stderr, not the chardev of QEMU.

## Decision

The machine tier keeps the contract of `--timeout` of the other rungs.
No emulator stays after the run (`AGENTS.md`, section 7).

## Prove

`PODVM_443_ONLY=machine sh experiments/443-qa-surface.sh` exits 0 on a host with `/dev/kvm`, with a row that `--timeout 2 ... sleep 30` exits 137 in less than 4 s and leaves no `qemu-system` process

## Done

**Done 2026-11-14.** `drive` (`machine/guest/drive.rs`) kills the
process group of the emulator at once where no exit marker came (the
deadline, or an emulator that ended), and keeps the wait of 10 s only
after a marker; a run past `--timeout` exits 137 with "the payload ran
past --timeout N s, and the guest was killed". By hand under TCG on the
cloud host: `run --rm --podvm-tier=machine --timeout 2 alpine:3.20 sleep
30` exits 137 after 2.03 s, and no `qemu-system` stays; `sh -c 'exit 3'`
still gives 3. Experiment 443 has the row `machine.timeout`. The Prove
names a host with `/dev/kvm`; the cloud host has none, and the row runs
under TCG there, which is the slower case.


# T-1810: A sealed bundle member that leaves the guest root is skipped and the run exits 0

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), F-11 (MACH-05; verify item 10), MACH-13 items 3 and 4
**Category:** machine
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

`podvm run --podvm-tier=machine --podvm-sealed alpine:3.20 < bundle.tar`
with a member `../mach-escape-clean.txt` exits 0; the member is neither
written nor named. `d/../../mach-deep.txt` is the same. An absolute
member `/mach-abs.txt` is accepted and lands at `/mach-abs.txt` in the
guest. Two nits of the same path: the files of the bundle carry an mtime
of 1970, and an empty pipe gets "reads the bundle of the run from stdin,
a pipe or a file", which does not say why it refused.

## Premise

`check` (`crates/podvm/src/machine/guest/sealed.rs:74`) validates only the
JSON; `unpack` (`crates/podvm/src/machine/guest/sealed.rs:99`) calls
`unpack_in` and ignores its `false` result
(`crates/podvm/src/machine/guest/sealed.rs:114`). The tar headers
that podvm writes for the guest have an mtime of 0.

## Approach

In `check`, refuse each member whose path has `..`, starts with `/`, or
has a component that is not normal, with exit 125, naming no bundle
value. Treat a `false` of `unpack_in` as an error. Keep the mtime of the
member. Say "the bundle is empty" for an empty stream.

## Decision

A sealed run fails loudly on a bad member; it never runs with a member
left out. The comment at `sealed.rs` line 96 is corrected to the new
check.

## Prove

`cargo test -p podvm --bin podvm sealed_members` exits 0, with tests that `../x`, `d/../../x` and `/x` members are refused with exit 125 and that an empty bundle names the reason

## Done

**Done 2026-11-14.** `machine/guest/sealed.rs`: `check` refuses, on the
host and before the boot, each member whose path has `..`, a leading
`/` or a component that is not a plain name (`inside`), and names no
member; `unpack` treats a member that `tar` did not land as an error;
an empty stream reads "the bundle is empty: stdin gave no bytes", and
a stdin that is not a pipe or a file says so. The test
`sealed_members_that_leave_the_root_refuse_the_bundle` refuses
`../canary-escape`, `d/../../canary-deep`, `/canary-abs` and `..` with
no `canary` in the message, lands `d/plain` and `./dot`, and keeps the
mtime of the header (1 700 000 000). By hand: a bundle with
`../mach-escape.txt` exits 125 at once, and an empty pipe exits 125 with
the new reason. The mtime of 1970 of the report: `unpack` keeps the
mtime of each header, and a `tarfile.TarInfo` of Python has mtime 0
when the writer sets none, which is the likely source. Experiment 445
passes its 6 clauses under TCG; experiment 443 has the row
`machine.sealed-escape`.


# T-1823: An early end of the guest init reads "the guest gave no exit marker" with the socket of QEMU

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), CODE-06, RUNS-12, RUNS-13, RUNS-20, MACH-13 item 2, `findings/code.md` "Not covered" (a full `/tmp`)
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

On the machine tier, `-w /no/such/dir` and `-w /etc/hostname` exit 125
with "the guest gave no exit marker; qemu: ... -chardev socket ... QEMU
waiting for connection"; the rootfs rung makes the missing directory, as
docker does, and refuses a file by name. `--platform linux/arm64` boots
an x86_64 guest and gives the same text; the rootfs rung names the
architecture. `--entrypoint /no/such` gives exit 127 with only the line
of the guest shell. A full `/tmp` gives the same text. Each message
prints a host path `/tmp/podvm-guest-ID.err`.

## Premise

The guest init does `cd W || exit 125`
(`crates/podvm/src/machine/guest/init.rs:76`) and `. FRAGMENT || exit 125`
(`crates/podvm/src/machine/guest/init.rs:108`), and leaves without the
exit marker; `drive` reports each early end as a missing marker and
appends the stderr of QEMU. `prepare_image`
(`crates/podvm/src/run/machine.rs:37`) does not make the workdir and does
not compare the platform of the image with the guest.

## Approach

Before the boot: compare the platform of the image with the architecture
of the guest kernel and refuse by name; make a missing workdir in the
tree and refuse a file by name; check that the entrypoint resolves in
the tree. In the init: print the exit marker with a podvm reason before
each early exit. In `drive`: say "the emulator did not start" or "the
guest ended before the payload" and print the chardev path only under
`--verbose`.

## Decision

The rootfs rung and the machine tier give one answer for one input,
where the tree on the host can tell.

## Prove

`PODVM_443_ONLY=machine sh experiments/443-qa-surface.sh` exits 0 on a host with `/dev/kvm`, with rows for `-w /no/such/dir` (prints the directory, exit 0), `-w /etc/hostname`, `--platform linux/arm64` and `--entrypoint /no/such` (each refused by name, no "exit marker" text)


# T-1846: A failed incus run leaves its instance, `ps` omits instances, `rm` calls an ERROR instance running, and `system info` takes `tier = bogus`

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), MACH-09, MACH-10 (tester-reported; the host could not start an instance)
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

`podvm run --podvm-tier=incus alpine:3.20 true` exits 125 at the start of
the instance, and `incus list` then shows `podvm-42b540560a8d,ERROR`; a
second run leaves `incrun,ERROR`. With `tier = incus` in the
configuration, `podvm ps -a` omits a stopped instance. `podvm rm inc1`
on an instance in state ERROR says "the instance runs". With
`tier = bogus`, `system info` prints `supervise userland` and exits 0,
while `run` refuses the value.

## Premise

The error path of the incus run does not delete the instance. `ps` lists
the table of `pod-supervise` only. The text "the instance runs" is
`crates/podvm/src/incus/verbs.rs:280`, for each state that is not
stopped. `system info` does not read the configuration as `run` does.

## Approach

Delete the instance on each error after its creation, and name it in the
refusal if the delete fails. Merge the instances with the podvm label
into `ps` when the configuration selects incus. Name the state in the
`rm` refusal. Validate the configuration in `system info` as `run` does.

## Decision

docs/incus.md says that a run deletes its instance; a failed run too.

## Prove

`cargo test -p podvm --bin podvm incus_verbs` exits 0, with tests against the fake incus API of the tests: a failed start sends the DELETE, `ps` lists a labelled instance, `rm` of an ERROR instance names ERROR, and `system info` with `tier = bogus` exits 125


# T-1849: `run` does not say that the machine tier needs `qemu-system`, and the guest sees less memory than `--podvm-mem` asks

**Source:** the external QA report of `v0.2.0-beta.14` (2026-11-14), benchmark P2 and P6 (`benchmarks/compare.md`, part C)
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** open

## Problem

`run --podvm-tier=machine` without `qemu-system-x86_64` exits 125 with one
line; the help of `--podvm-tier` and the README ("Each binary is static:
it needs no library of the host") do not name the emulator. `doctor`
checks it. `--podvm-mem 256M` gives a guest `MemTotal` of 223 016 kB
(85.1 %), 512M gives 91.7 %, 1G gives 94.9 %.

## Premise

The guest kernel keeps a reserve; the help of `--podvm-mem` does not say
that the value is the memory of the virtual machine, not the memory free
for the payload.

## Approach

Name `qemu-system-ARCH` and `podvm doctor` in the help of `--podvm-tier`
for `machine`. Say in the help of `--podvm-mem` and in `docs/limits.md`
that the guest kernel keeps a part of the size.

## Decision

The value stays the size of the virtual machine, as QEMU's `-m` is.

## Prove

`cargo test -p podvm --bin podvm machine_help` exits 0, with a test that `run --help` names `qemu-system` and `doctor` beside `--podvm-tier` and says what `--podvm-mem` sizes


# T-1861: `podvm run windows:NAME` has no image name, no catalog and no `pull`, `images` or `rmi`

**Source:** the Windows guest review of 2026-11-14, section 5, item W2
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** partial

## Problem

A Windows guest takes a path, not a name. `crates/podvm/src/windows/mod.rs`
has `resolve_image` (:76) and `is_disk_request` (:179); the positional
argument is a path, a disk suffix, or an OCI-shaped token that only names a
banner (:136-139). `windows fetch --url` takes a raw disk URL, and the
DOS base comes from `experiments/369-windows-tcg-dos.sh`. No pin table is in
the binary. A user cannot type `windows:11`.

## Premise

The owner accepts the vendor terms (`docs/decisions.md`, 2026-11-14, "Windows
media"), so podvm can fetch, verify and keep the media itself. The Microsoft
evaluation ISOs have static CDN URLs with a fixed size and sha256. The review
checked on 2026-11-14 that the URLs of Windows 11 Enterprise Evaluation and of
Windows Server 2025 Evaluation answer `HEAD` with the pinned length. The pins
come from dockur/windows `src/define.sh`. The sha256 of each entry is taken
again from the download, not from the review.

## Approach

Add a catalog to `crates/podvm-windows`: one row for each of `validationos`
(alias `vos`), `11`, `2025`, `2022`, `2019`, `10` and `dos`, with URL, size,
sha256, build, kind (disk, ISO or zip) and a ceiling of size plus 1 percent.
Add the store layout `store/windows/{iso,images,virtio}` of the review (4.3).
Add the verbs `podvm pull NAME`, `podvm images` and `podvm rmi NAME` for the
Windows names. Make `podvm run windows:NAME` route to the Windows driver with
no `--platform` and no `--podvm-tier`. An unknown name is refused by name with
the list of known names. `windows:dos` fetches the pinned FreeDOS 1.4 LiteUSB
zip itself and writes the 32 MiB base (sha256 pins from experiment 369), so
`dos.rs` no longer needs a script. The download uses the verified fetch of
T-1862.

## Decision

A tag is the Windows release, as a docker tag is. `windows:validationos` and
`windows:dos` work under TCG. A full Windows image installs only where
T-1867 allows it. Language is a later flag with the default en-US.

## Prove

`cargo test -p podvm-windows catalog` exits 0, with new tests: the pin table

## Done

**Partial 2026-11-14.** The catalog and the name: `podvm-windows`
`catalog.rs` holds seven names (`dos`, `validationos`/`vos`, `11`,
`2025`, `2022`, `2019`, `10`) with URL, size, sha256, kind, store file
and a ceiling of size plus 1 percent; the pins of the evaluation ISOs
are dockur/windows `src/define.sh` (each URL answered `HEAD` with its
size, 2026-11-14), FreeDOS is experiment 369, and Validation OS
26100.9818 was hashed by podvm over its 2 509 025 280 bytes on
2026-11-14. `run windows:NAME` takes the door of the Windows driver with
no `--podvm-tier` and no `--platform` (`windows::should_drive`, `named`);
`windows:dos` fetches the pinned zip, checks it, writes the 32 MiB base
and boots (`windows/pull.rs`); an unknown name is refused with the list
of names. Tests: `catalog_pins_parse_and_each_name_resolves`,
`catalog_refuses_an_unknown_name_with_the_list`,
`catalog_zip_member_reads_a_stored_member_and_refuses_the_rest` and
`windows_name_takes_the_driver_door_with_no_flag`. By hand on the cloud
host with a fresh home: `podvm run windows:dos ver` fetched the zip,
wrote the base and printed `FreeCom version 0.86` with exit 0 in 62 s
under TCG; a second run took the base; `windows:11` is refused by name
with T-1867 and the store path; `windows:95` lists the names; no
emulator stays. The clauses that remain: `pull`, `images` and `rmi` for
the Windows names, the disk of `validationos` (T-1863) and of a setup
ISO (T-1867), and the experiment of the Prove.
parses, each name resolves, an unknown name is refused with the list, and the
refusal text names the store path. `cargo test -p podvm --bin podvm
windows_name` exits 0, with new tests that `run windows:dos` and `run
windows:11` reach the Windows driver with no tier flag. A new experiment (the
next free number) pulls `windows:dos` end to end on the cloud host, with
`podvm images` listing it, and runs `ver` under TCG with exit 0.


# T-1862: The download has no resume, no free-disk check and no `RLIMIT_FSIZE` check

**Source:** the Windows guest review of 2026-11-14, section 5, item W3
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** open

## Problem

`windows fetch` is one HTTPS GET (`crates/podvm/src/windows/setup.rs:86-141`,
`DEFAULT_MAX_BYTES` 8 GiB at :17). It has no Range request and no resume. A
download of 2 to 8 GB that breaks starts again. Without `--sha256` the file is
bounded but not verified. The ceiling is one value for each image. The shared
`pod_guest::fetch::store` of podcrates v0.42.0 holds the bounded store.

## Premise

The static CDN answers `Accept-Ranges: bytes` (the review checked it on
2026-11-14 for the Windows 11 and Server 2025 ISOs and for the ValidationOS
ISO with `206`). A 64 GiB sparse qcow2 can hit `RLIMIT_FSIZE` late.

## Approach

Add Range requests with a `.part` file, resume after a break, a check of the
size against `Content-Length`, a sha256 check at the end, and a ceiling for
each catalog entry. Before the first byte, check the free disk and
`RLIMIT_FSIZE` and refuse by name. Print one progress line (bytes and rate).
The shared `fetch` belongs to podcrates. podvm asks podcrates by a COORD issue
for Range and resume in `pod_guest::fetch::store`, and writes a thin podvm
wrapper for the progress line and the pre-checks. Until the tag arrives, the
wrapper keeps the present behavior.

## Decision

The Range and resume logic is shared code, so it lives in podcrates. The
progress line and the pre-checks live in podvm.

## Prove

`cargo test -p podvm-windows download` exits 0, with new tests against a local
Range server (the pattern of experiment 370, clause 4): a break in the middle
then a resume gives the right digest, a sha256 mismatch removes the file, a
declared length above the ceiling is refused with no file left, and a
`RLIMIT_FSIZE` below the size is refused by name before the first byte.


# T-1863: `windows:validationos` has no fetch by range, and the extent of the VHDX is a fixed sector

**Source:** the Windows guest review of 2026-11-14, section 5, item W4
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** open

## Problem

`experiments/371-validationos-stream.sh` reads the UDF structure of the
Microsoft ISO and fetches only the VHDX extent. `.github/workflows/kvm.yml`
(job `windows`) fixes the extent: sector 1010, 910163968 bytes, for build
26100.9278. The review checked on 2026-11-14 that `aka.ms/DownloadValidationOS`
now redirects to build 26100.9818 (2,509,025,280 bytes; the 9278 ISO is
2,460,880,896 bytes). A new ISO moves the extent. podvm has no UDF reader in
Rust.

## Premise

winquick `src/udf.rs` (470 lines, Apache-2.0) reads the UDF directory. The
captured copy is in the references (issue 149, `references/README.md`). The
UDF layout of the ISO can change in each build, so a fixed sector is wrong.

## Approach

Add a UDF reader in Rust and use it for `windows:validationos`: read the
directory by Range requests, find `ValidationOS.vhdx`, fetch only the extent
(about 0.9 GB, not 2.5 GB), verify the sha256, store it. Resolve the build
from the pin in the catalog or from the `Location` of the `aka.ms` redirect.
Prefer a crate `pod-udf` in podcrates: podvm asks podcrates by a COORD issue.
If podcrates declines, vendor `udf.rs` under `vendor/` with its notice in
`THIRD_PARTY.md` (Vendoring rules, `docs/methodology/vendoring.md`).

## Decision

The reader belongs to podcrates first, as it is generic. Vendoring is the
fallback and needs the notice.

## Prove

`cargo test -p podvm-windows udf` exits 0, with new tests on a small
synthetic UDF image: the file is found, the extent and length are right, and
a missing file is refused by name. A new experiment (the next free number) does
a live range test against the 9278 ISO and the 9818 ISO and prints extent,
length and sha256 for each. It starts no emulator and runs on the cloud host.


# T-1864: The agent of a Windows guest runs the command with `call`, so a payload that ends in `exit` times out

**Source:** the Windows guest review of 2026-11-14, section 5, item W6
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** open

## Problem

`pod-guest` `agent.rs` (`AGENT_CMD`, podcrates v0.42.0) runs the command
through `call`. A payload that ends in `exit` ends the agent, and the run
runs to the timeout. The run on 2026-09 measured it and kept it as intended
behavior (`experiments/results/windows-guest.txt`: a payload that ends in
`exit` runs to the 300 s timeout and is refused). winquick runs a child
`cmd /c`, so `exit` cannot kill the agent.

## Premise

A user expects `cmd /c exit 42` and a payload with `exit` to return their
code, as in CI. A timeout is a wrong answer for a correct command.

## Approach

Ask podcrates by a COORD issue to run the command in a child `cmd /c`, to add
a `WQREADY` marker, and to add a keep-alive mode with no shutdown for the
later warm and `exec` use. podvm takes the new tag and updates the result
file. The stdin of the command stays NUL.

## Decision

The change is in `pod-guest`, so podcrates owns it. podvm owns the
experiment and the record.

## Prove

A new experiment (the next free number) runs the case "payload ends in `exit`"
on the ValidationOS image under TCG on the cloud host, and it returns the code
of the payload and not a timeout. `cmd /c exit 42` still gives 42. The result
goes under `experiments/results/`.


# T-1865: The plan of a Windows guest has no NIC, no CD-ROM, no USB media, no VNC socket and no serial channel

**Source:** the Windows guest review of 2026-11-14, section 5, item W9
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** open

## Problem

The boot line of `pod-guest` `plan.rs` (podcrates v0.42.0) is fixed: `-M q35`,
two OVMF pflash drives, a qcow2 overlay on NVMe, a mailbox disk on NVMe,
`-vga std -display none -nic none`, QMP on a socket. It has no option for a
user-mode NIC with host forwards, virtio devices, an install ISO as CD-ROM,
USB mass storage, a VNC socket, a serial channel for a guest agent or Hyper-V
enlightenments. `--podvm-qemu-arg` is the only way in.

## Premise

The install builder (T-1867) needs a NIC, a CD-ROM, USB media and USB input.
The guest agent (T-1868) needs a virtio-serial channel. ValidationOS has no
network driver, so its run keeps `-nic none`.

## Approach

Ask podcrates by a COORD issue for plan options: NIC (`user` with `hostfwd`),
virtio devices, CD-ROM and USB media, a VNC socket, a serial channel and the
Hyper-V flags of T-1877. `-nic none` stays the default of the mailbox run. In
podvm, pass the options from the flags. T-1867 may use `--podvm-qemu-arg` until
the tag arrives.

## Decision

The plan is in `pod-guest`, so podcrates owns it.

## Prove

`cargo test -p podvm-windows plan_options` exits 0, with new tests of the
argv for each option and a test that fails when a NIC device appears in the
argv of a ValidationOS run (the check that winquick has). `podvm windows run`
of ValidationOS keeps `-nic none`.


# T-1866: There is no generator of the unattended answer file or of the USB FAT media

**Source:** the Windows guest review of 2026-11-14, section 5, item W8a
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** partial

## Problem

A full Windows install needs an `autounattend.xml` and media on which
Setup finds it. podvm has neither. `pod-guest/src/fat16.rs` builds a FAT16
volume in Rust (16 MiB behind an MBR) and needs no host tool.

## Premise

Setup finds the answer file at the root of a removable drive or a CD. The
plan is a USB mass storage FAT image (`-device usb-storage`). The review did
not run this; T-1867 proves it. The LabConfig bypass for Windows 11
(`BypassTPMCheck`, `BypassSecureBootCheck`, `BypassRAMCheck`) is taken from
dockur/windows `assets/win11x64.xml` lines 93-106 and is not tested here.

## Approach

Write pure functions in `crates/podvm-windows` that build the answer file
from a template for 11, 2025 and 10: GPT layout for UEFI, edition by name,
locale, a local account `podvm` with AutoLogon for the first logon, OOBE
skipped, the LabConfig bypass for 11, RDP on (`fDenyTSConnections=0` and the
firewall group), no sleep, and a `FirstLogonCommands` chain that ends with a
`READY` marker and a shutdown. The password is random for each machine, and is
never placed in argv or a log (`docs/decisions.md` and AGENTS.md section 7).
Write the XML into a FAT image with the writer of `fat16`.

## Decision

The template is in the binary and the user does not edit it. The password
goes to the machine record with mode 0600 (T-1869 prints it on request).

## Prove

`cargo test -p podvm-windows answer_file` exits 0, with new tests: golden

## Done

**Partial 2026-11-14.** The answer file: `podvm-windows/src/answer.rs`,
a pure `xml(&Answer)` for Windows 11, Server 2025 and Windows 10 (GPT
for UEFI, the image by its index, the locale, one administrator `podvm`
with one automatic logon, the out-of-box screens skipped, Remote Desktop
and its firewall group on, no sleep, and a first logon that writes
`PODVM-WINDOWS-READY` on COM1 and shuts down; the LabConfig keys for 11
only), and `password()`, 20 characters from `/dev/urandom`. Tests:
`answer_file_is_well_formed_for_each_target` (each element closes; the
index, the marker, Remote Desktop; LabConfig for 11 only; the password
escaped and in its two places only) and
`answer_file_password_is_random_and_complex`. The index of the server
image (2, the Standard edition with the Desktop Experience) is an
estimate until T-1867 reads the WIM. The clause that remains: the USB
FAT media, which needs the long name `autounattend.xml` that `fat16` of
`pod-guest` does not write (8.3 only): COORD-109 (issue 360) asks
podcrates; and the golden files, the round trip through `fat16` and the
pinned `-device usb-storage` line of the Prove come with it.
files for 11, 2025 and 10, XML well-formedness, a round trip of the FAT image
through the reader of `fat16`, a test that the password is not in any argv, and
a test that pins the `-device usb-storage` line.


# T-1867: `podvm pull windows:11` cannot install a full Windows guest

**Source:** the Windows guest review of 2026-11-14, section 5, item W8b
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** open

## Problem

There is no route for a full Windows 10, 11 or Server install
(`crates/podvm/src/windows/mod.rs:57-69` and `:121-130` refuse what it
does not route). The setup of today (`pod_guest::provision`) only prepares
the ValidationOS image and sleeps the whole wait with no readiness check.

## Premise

The install uses devices with inbox Windows drivers: q35, OVMF, NVMe root
disk, the ISO as `ide-cd`, an `e1000e` NIC on slirp, `-vga std`, and
`qemu-xhci` with `usb-kbd` and `usb-tablet`. The stock ISO asks "Press any key
to boot from CD or DVD" under UEFI; the plan sends keys through QMP every 500 ms
for the first 20 s (`qmp::Monitor::keys` exists). Install time under KVM is
15 to 40 min (estimate). Under TCG it is hours (estimate). The review ran none
of this. The "press any key", the answer-file discovery on USB and the
LabConfig bypass on the current 25H2 build are taken from other tools and may
need an ISO repack.

## Approach

Build the installer on T-1865 (or `--podvm-qemu-arg` until then) and T-1866:
boot the ISO with the answer media, send the keys, watch the guest from
outside (QMP `screendump` every 30 s and the `READY` marker), print the
phases `copying files`, `installing` and `first logon` with elapsed time,
apply a hard timeout, then `qemu-img convert` the disk into `golden.qcow2`
with no backing file. The ISO stays in the cache. Refuse TCG by default and
name `--accel tcg` and the expected time in the refusal. Check the disk and
`RLIMIT_FSIZE` before the first write (T-1862). Disk 64 GiB sparse, 4 GiB
and 2 CPUs by default, all overridable.

## Decision

`podvm pull windows:2025` is the first proof (it has no TPM check), then
`windows:11`. ValidationOS and DOS stay allowed under TCG.

## Prove

A new experiment (the next free number) on `kvm.yml` (T-1682) runs `podvm pull
windows:2025` and then `windows:11`. It records the wall clock of each, checks
that `golden.qcow2` has no backing file, that the digest of the vendor ISO is
unchanged, that no emulator is left, and that a refusal under TCG names
`--accel tcg`. Record the first real install time; until then it is a dash.
The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`. `cargo test -p podvm-windows install_plan` exits 0, with new tests
of the phase lines and of the TCG refusal.


# T-1868: A full Windows guest has no `exec`, `cp`, graceful `stop` or readiness probe

**Source:** the Windows guest review of 2026-11-14, section 5, item W10
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

The mailbox runs one command for each boot. A running full Windows guest has
no way to run a command, copy a file or stop in a graceful way.

## Premise

The QEMU Guest Agent offers `guest-ping`, `guest-exec`, `guest-exec-status`,
`guest-file-open`, `guest-file-read`, `guest-file-write`, `guest-file-close`,
`guest-get-osinfo` and `guest-shutdown`. The virtio-win tools install it as a
service, and it talks over virtio-serial. ValidationOS cannot run it (no
Windows Installer service, no PnP service), so ValidationOS keeps the
mailbox. `pod-guest/src/qmp.rs` is a client of the same style.

## Approach

Add the virtio-win guest tools to the install chain of T-1867 (a pinned
virtio-win ISO in the catalog, as a third CD). Write a JSON client of the agent
over a Unix socket (a crate `pod-qga` in podcrates, or in podvm). Add `podvm
exec NAME -- CMD` (exit code passes through, 0 to 124), `podvm cp` in both
directions, a graceful `stop` and the `guest-ping` readiness probe. The first
version needs the socket of T-1865.

## Decision

The client is generic, so podvm asks podcrates by a COORD issue first. If
podcrates declines, podvm owns it.

## Prove

`cargo test -p podvm-windows qga` exits 0, with new tests against a fake
socket. On the golden image of T-1867: `exec -- ver` returns the exit code,
a `cp` round trip keeps the sha256, and `stop` is graceful. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`.


# T-1869: A Windows guest has no `-p`, no RDP check, no VNC, no screenshot and no `creds`

**Source:** the Windows guest review of 2026-11-14, section 5, item W11
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

A guest started by `podvm windows run` has `-nic none` and `-display none`.
It publishes no port and offers no desktop.

## Premise

QEMU user-mode networking needs no root and no tun. `-p HOST:GUEST` becomes
`hostfwd=tcp:127.0.0.1:HOST-:GUEST`. The review (6, item 14) notes that the
guest reaches the host loopback at 10.0.2.2 as in the Linux machine tier
(experiment 421); whether that stays on for Windows is open. Slirp gives no
ICMP echo to the internet, so `ping` fails while `curl` works. This is not
measured here.

## Approach

Add `-p` (loopback by default, `0.0.0.0:` only if written, a free port when no
host port is named), `podvm port NAME PORT`, the readiness chain (agent
answers, TCP 3389 accepts, X.224 confirm that podvm sends itself), `podvm
windows rdp`, `vnc` (a Unix socket), `screenshot` (QMP `screendump`) and
`creds`. The run prints a hint line and never the password. Full Windows gets
a user-mode NIC and `--podvm-net none` turns it off, as for the Linux machine
tier. Document the `ping` oddity in `docs/limits.md`. Decide the policy for
10.0.2.2 in this entry and record it in `docs/decisions.md`.

## Decision

The default is no published port and no host folder.

## Prove

After `run -d -p 3389:3389 windows:2025` the host sees the RDP confirm,
`podvm port` prints the host port, and `--podvm-net none` makes an outbound
`curl` in the guest fail. `cargo test -p podvm windows_ports` exits 0, with new
tests of the `hostfwd` line and of the loopback default. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`. It needs
T-1865, T-1867 and T-1870.


# T-1870: A Windows guest has no lifecycle: no `run -d`, `ps`, `stop`, `start`, `rm`, `logs` or `--name`

**Source:** the Windows guest review of 2026-11-14, section 5, item W7
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** open

## Problem

Detached `run` and `create` are refused for a Windows guest
(`crates/podvm/src/windows/mod.rs:57-69`, `:121-130`). The guest has no
record, no pidfile and no lifetime. Each run is one boot, one command and
one removal (`RunGuard`).

## Premise

A detached machine without a lifetime is a leak. The emulator runs in its own
process group today, and the stop sequence is QMP `quit`, SIGKILL to the group
and a bounded reap, with the report `Confirmed` or `Attempted`. A pidfile
needs the start time, because a pid can be reused. DOS and ValidationOS can
prove the lifecycle under TCG on a cloud session, so this item can start
before T-1867.

## Approach

Add machine records under `store/machines/NAME/{machine.json,disk.qcow2,
vars.fd,qmp.sock,log}`. Add `run -d`, `--name`, `ps [-a]`, `stop`, `start`,
`rm [-f]` and `logs [-f]`. A launcher owns the process group and writes a
pidfile with the start time. `--podvm-timeout` on a `run -d` is the maximum
lifetime (0 is refused, the default is 24 h, estimate). `stop` and `rm -f` end
the group, and `rm` checks that no process of the machine stays. Reuse the
ideas of `pod-supervise`.

## Decision

Podvm owns the lifecycle. A machine is an overlay on the golden image, as a
container is on an image.

## Prove

A new experiment (the next free number): `run -d` a DOS or ValidationOS guest
under TCG on the cloud host, `ps` shows it, `stop` ends it, `rm` removes it,
then `pgrep qemu-system` is empty and the run directory is gone. A plant that
leaves the emulator makes the check fail (`podvm-plant`).
`cargo test -p podvm windows_lifecycle` exits 0, with new tests of the
record and the pidfile start-time check.


# T-1871: `podvm windows doctor` is thin

**Source:** the Windows guest review of 2026-11-14, section 5, item W14
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** partial

## Problem

`doctor` (`crates/podvm/src/windows/doctor.rs`) prints the profile, the
accelerator, the tool and firmware paths and the base image state. It has no
QEMU version check, no firmware-pair check, no free-disk check, no
`RLIMIT_FSIZE` check, no nested-virtualization note, no store state, no image
age check and no "next command" line. The first failure of a new user is "qemu
is not on this machine".

## Premise

winquick `doctor` checks the host architecture, the accelerator, the QEMU
version (it flags QEMU below 11, for NVMe state migration), the UEFI firmware
pair ("firmware from the wrong host is worse than none"), the free disk and
whether the host is a VM. The 2026-09-30 and 2026-10-02 KVM runs under WSL
failed or hung. The Microsoft evaluation media run for 90 days (client) or
180 days (Server): a vendor statement, not checked here.

## Approach

Add the checks above, each with its own exit code and a line that names the
next command (for example the apt, dnf or apk line for QEMU). Say "this host is
itself a VM" as a note. Warn when a golden image nears the end of the
evaluation period (the date is in `meta.json`). Make `PODVM_MACHINE_ACCEL=kvm`
work for `podvm windows` as for the Linux guest (C16): `kvm` with no
`/dev/kvm` refuses by name and names the `kvm` group, and it never falls back
to TCG without a word. The verb stays reachable as `podvm doctor`.

## Decision

The doctor refuses where `run` refuses, and shares `plan::accelerator`.

## Prove

`cargo test -p podvm windows_doctor` exits 0, with new tests for each check

## Done

**Partial 2026-11-14.** `windows/doctor_more.rs`: the QEMU version
(with a note below 11, for the warm start of T-1874), a missing emulator
as a failure with exit 1 and the install line for apt, dnf and apk, the
firmware pair, the free disk and the `RLIMIT_FSIZE` of the store of the
catalog, a note where the host is itself a VM, the state of each name of
the catalog, and a "next" line. The doctor found a defect: on the cloud
host (Ubuntu 24.04) `plan::firmware` found no OVMF, because Debian and
Ubuntu name it `OVMF_CODE_4M.fd`; the search is now by pairs of one
build (`windows_doctor_firmware_is_a_pair_of_one_directory`), and the
host's `/usr/share/OVMF/OVMF_{CODE,VARS}_4M.fd` is found. Tests:
`windows_doctor_reads_the_version_of_an_emulator` and the pair test. By
hand: exit 0 with each line on the cloud host, and exit 1 with the
install line where `PODVM_QEMU` names no emulator. The clauses that
remain: the end of the evaluation period of a golden image (T-1867
writes its `meta.json`), and `PODVM_MACHINE_ACCEL=kvm` for `podvm
windows`.
and for the `kvm` refusal. A plant that breaks one check turns the gate red
(`podvm-plant`).


# T-1872: A Windows guest has no `-v`: no way to bring a folder in or out

**Source:** the Windows guest review of 2026-11-14, section 5, item W12
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

There is no shared folder, and no copy-in or copy-out for a Windows guest.
winquick `-w` copies a directory into a fresh filesystem image that becomes
`C:\workspace`, and `-a` collects named files. The machine tier does the same
for Linux and the run says that it is a copy.

## Premise

A live share has three candidates and none is proven: slirp `smb=DIR` needs
a host `smbd` (not static); virtiofs needs `virtiofsd` and the WinFsp driver,
and `virtiofsd` needs namespaces that the target hosts deny; an in-process
WebDAV server on the slirp gateway needs the guest `WebClient` service and has
a Windows file size limit that the review did not check. 9p does not work on
Windows.

## Approach

First, add `-v HOST:C:/path[:ro]` as a one-way copy disk. The run says that it
is a copy, and `:ro` is honest. Second, run an experiment to choose a live
share (WebDAV in podvm, virtiofs or smb) with a write-back test. Do not
promise live sync before that proof. `podvm cp` of T-1868 covers both
directions with no share.

## Decision

The copy comes first. The live share waits for the experiment.

## Prove

`cargo test -p podvm windows_volume` exits 0, with new tests of the flag parse
and of the refusal of a path that escapes. Copy, on a guest of the cloud host:
a file appears in the guest, a write in the guest does not appear on the host,
and the run says that it is a copy. Live share: a new experiment (the next free number) shows that a guest
write lands on the host with the right owner. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`.


# T-1873: `windows setup` sleeps the whole wait with no readiness check

**Source:** the Windows guest review of 2026-11-14, section 5, item W5
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** open

## Problem

`pod_guest::provision` sleeps the whole `wait` (default 240 s,
`crates/podvm/src/windows/setup.rs:190`) with no readiness check, then types
a drive-scan line through QMP `send-key` (8 ms a key, US layout). Setup takes
at least `wait` seconds even when Windows is ready in 20 s. The record flags the
same ambiguity (`TODO/gate.md`, T-1641 Correction); the hosted run 43148947380
took about 10 minutes.

## Premise

winquick sets up offline in about a minute: it writes `wqagent.cmd` into
`System32` with `ntfscp` at a byte offset and sets `Command Processor\AutoRun`
with `hivexsh`, with no boot. The review found a conflict: winquick says
`AutoRun` works on ValidationOS ARM64, and `pod-guest/src/agent.rs` says that on
the x64 image `AutoRun`, `Run` and `RunOnce` do not fire and an `onstart` task
as SYSTEM does. An x64 measurement is needed first. `ntfscp` and `hivexsh` are C
and can build static with zig; a small Rust NTFS overwrite of the same size is
the other way.

## Approach

Part (a): replace the sleep by a readiness poll (mailbox marker, serial text
or a change of the screen) and cut the time of `setup`. Part (b): measure whether
`AutoRun` fires on the x64 image, then evaluate the offline write. The
provision code is in `pod-guest`, so podvm asks podcrates by a COORD issue; the
NTFS and hive writer is a new shared crate.

## Decision

Part (b) starts only after the x64 `AutoRun` proof.

## Prove

`cargo test -p podvm-windows provision_ready` exits 0, with new tests of the readiness poll (a fake console and a fake mailbox). `sh experiments/392-kvm-guest.sh` records, for (a), a setup time under 90 s on KVM (estimate of the
target). (b) A new experiment shows that `AutoRun` fires or does not fire on the
x64 image. If it fires, setup takes under 60 s with no boot, and then `ver` and
exit 42 pass; the offline write has a unit test on an image from the hosted
runner. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`.


# T-1874: Each Windows run is a full boot: there is no warm start

**Source:** the Windows guest review of 2026-11-14, section 5, item W13
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** open

## Problem

Every run boots Windows: 29 s under TCG (measured 2026-09 with a harness);
under KVM the time is a dash. winquick restores a frozen guest in 0.23 to
0.31 s (Apple M4 Pro, HVF, QEMU 11.1).

## Premise

winquick freezes a prepared guest with `migrate` to a file, and each run
restores it with `-incoming file:` over a clone of the overlay. It keeps a
fingerprint of version, base, agent, QEMU, firmware, RAM and CPU, and it
remounts the mailbox volume by GUID to defeat the FAT cache. It needs QEMU 11
for the NVMe device state. The hosted runner has QEMU 10.2.1. x86 KVM is not
measured.

## Approach

Ask podcrates by a COORD issue for the mechanism: freeze, restore, the
fingerprint and the fallback to a cold boot. Start with ValidationOS. podvm
adds the flags and the doctor note about QEMU 11. Depends on T-1864 (keep-alive
mode).

## Decision

A wrong or stale state always falls back to a cold boot, never to a wrong
answer.

## Prove

A new experiment (the next free number) on `kvm.yml` runs 50 `cmd /c ver` runs
and records the p95. A changed QEMU or base invalidates the state, and a
wrong state falls back to a cold boot. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`. It needs QEMU 11.


# T-1875: podvm needs `qemu-system-x86_64`, `qemu-img` and OVMF on the host

**Source:** the Windows guest review of 2026-11-14, section 5, item W15
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** open

## Problem

`crates/podvm/src/windows/plan.rs:137-165` refuses by name when QEMU, `qemu-img`
or an OVMF pair is missing. This is the first failure of a new user, and it
breaks "download and use". The rule of the owner is that each released
binary is static.

## Premise

No pinned static `qemu-system-x86_64` source is in the record. A build with zig
is large. A vendor static build may exist; the review did not verify one. Until
a source is found, `podvm windows doctor` (T-1871) prints the install line.

## Approach

Find and pin a source of static `qemu-system-x86_64`, `qemu-img` and edk2
firmware. Fetch them into the store when the host has none, with a sha256
check. Record the source in `THIRD_PARTY.md`. If the answer is a build, write
the build as a tracked script. If a shared crate is the right home, ask
podcrates by a COORD issue.

## Decision

The host QEMU wins when it is present and passes the doctor checks.

## Prove

A clean container with no QEMU: `podvm run windows:dos` works under TCG, and
`podvm windows doctor` names the store copy. A new experiment (the next free
number) runs it and prints the sha256 of each fetched tool.


# T-1876: Nothing checks the catalog pins for drift

**Source:** the Windows guest review of 2026-11-14, section 5, item W16
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** open

## Problem

The pins (T-1861) point at Microsoft URLs that rot. `aka.ms/DownloadValidationOS`
moved from build 9278 to 9818 between the record and 2026-11-14. Old ISO URLs
can disappear.

## Premise

A `HEAD` request tells the size and the redirect target. A scheduled job can
compare them with the pins and open an issue on drift. How podvm refreshes a
catalog without a release (a signature, a pin in the binary or a git tag) is not
decided.

## Approach

Add a scheduled workflow that sends `HEAD` to each pinned URL, compares the
size and the `aka.ms` target, and opens an issue on a difference. Write a
proposal for the refresh path as a question for the owner in
`docs/decisions.md`, with a recommendation (AGENTS.md section 10).

## Decision

The job reads only. It never changes a pin.

## Prove

`sh scripts/common/check-catalog-drift.sh --plant` exits 1 for a planted URL
and `sh scripts/common/check-catalog-drift.sh` exits 0 on the live catalog. The
script is new and this entry adds it. The workflow runs it on a schedule.


# T-1877: A Windows desktop has no Hyper-V enlightenments, and no one measured its boot time

**Source:** the Windows guest review of 2026-11-14, section 5, item W17
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** open

## Problem

The CPU flag is `-cpu host` under KVM and nothing else. The boot to RDP time is
a dash.

## Premise

The review proposes `-cpu host,hv_relaxed,hv_vapic,hv_spinlocks=0x1fff,hv_time`.
It is not tested. Other timer and `-rtc` settings may also help. Under TCG
`-cpu host` is never used (this part is done).

## Approach

Add the flags through the plan options of T-1865. Measure the boot to RDP time
with and without them on the hosted runner. Keep the flags only if the time
drops.

## Decision

The decision follows the measurement.

## Prove

`sh experiments/NNN-windows-hyperv.sh` (new; this entry adds it with the next
free number) does an A/B run of the boot to RDP time with and without the flags
and prints both. The proof runs on `kvm.yml` (T-1682), because a cloud session has no `/dev/kvm`. It needs T-1867 and
T-1869.


# T-1878: `podvm run -it windows:11 -- powershell`, streamed logs, a web viewer and Windows ARM64 are not planned in detail

**Source:** the Windows guest review of 2026-11-14, section 5, item W18
**Category:** machine
**Milestone:** none
**Priority:** P3
**Effort:** L
**Status:** open

## Problem

`run` returns the output at the end. There is no PTY, no streaming, no web
viewer. A Windows guest follows the host architecture (`windows/mod.rs:121`),
so an arm64 host has no media.

## Premise

A PTY needs an own guest agent: a small Windows executable cross-built with
`x86_64-pc-windows-gnu` through zig and embedded with `include_bytes!`. A web
viewer is a separate crate (HTTP and noVNC). Windows 11 ARM64 media is another
project for dockur. `arm.yml` exists for an arm64 runner.

## Approach

Split this entry into its own entries when T-1867 and T-1868 are done: one for
the PTY and the streamed logs (podcrates for the agent), one for the web
viewer, one for ARM64. Each has its own proof.

## Decision

This entry is a holder. It closes when each part has an entry.

## Prove

`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0 and the record
holds one entry for each part, each with a Prove that a session can run.


# T-1879: The messages of `dos.rs` and `doctor.rs` and the premise of T-1112 still say that podvm fetches no image

**Source:** the Windows guest review of 2026-11-14, section 5, item W1
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

The ruling of 2026-11-14 ("Windows media", `docs/decisions.md`), the paragraph
of `docs/limits.md` and the header of `experiments/370-windows-guest.sh` are
corrected on `main`. These texts stay. `crates/podvm/src/windows/dos.rs:58-66`
prints "No base is fetched or committed here" and sends the user to
`experiments/369-windows-tcg-dos.sh`. `crates/podvm/src/windows/doctor.rs:63`
prints that the script "fetches FreeDOS". The Premise of T-1112
(`TODO/milestones.md`) says that the licensed input "is supplied explicitly".
T-1112 is `done`, but its acceptance command needs a user who holds a VHDX, so
the goal for a user is not met.

## Premise

podvm fetches the FreeDOS base itself after T-1861. Until then, the messages
must not say that podvm cannot fetch. They must not tell a user to bring an
image for a reason of license.

## Approach

Rewrite the two messages so that they name the command that creates the base
(`podvm pull windows:dos` after T-1861, and the script before). Add a
`## Correction` section below the Premise of T-1112 that says that the owner
accepts the vendor terms and that the user goal is the work of T-1861 to
T-1878. Keep the title and the status of T-1112.

## Decision

Do the text now. Do not wait for T-1861: name the script until the verb
exists.

## Prove

`cargo test -p podvm --test dos_refusal` exits 0, with the test of T-1736
updated to the new text. `grep -rn "No base is fetched" crates` finds nothing.
`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0, and its plant
for the removed sentence turns red when the sentence returns.

## Done

**Done 2026-11-14.** The message of `dos.rs` is gone: a DOS run fetches
the FreeDOS base of the catalog (T-1861), and the doctor says that
`podvm run windows:dos` fetches it. T-1112 has a `## Correction` that
names the ruling and T-1861 to T-1878. `podvm-gate` refuses the retired
sentence in a Rust source of `crates/` (`gate/retired.rs`); the plant
case 27d appends it to `dos.rs`, and the gate exits 1 with "a retired
sentence" (run by hand on this host), and 0 after the restore.
`cargo test -p podvm --test dos_refusal` exits 0, and `grep -rn "No base
is fetched" crates` finds only the check in `podvm-gate`.


# T-1884: A machine run that podvm ends on SIGTERM leaves its emulator running

**Source:** the check of T-1817 (2026-11-14)
**Category:** machine
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`printf 'x\n' | timeout 400 podvm run --rm --podvm-tier=machine -t
alpine:3.20 cat` with beta.17 hung, `timeout` sent SIGTERM to podvm at
400 s, podvm exited 124, and a `qemu-system-x86_64` process of that run
was still running afterwards (pid 7795 on the cloud host, ended by hand).
`AGENTS.md`, section 2: no emulator stays after the run.

## Premise

The machine tier starts QEMU in a process group of its own, and only the
normal end of the run (`drive`, its bounded wait and kill) ends it; a
signal to podvm does not reach that group.

## Approach

On SIGTERM, SIGINT and SIGHUP during a machine run, kill the process
group of the emulator, reap it within a bound, and exit 128 plus the
signal; a supervisor of the emulator that dies with podvm (a pidfd and
`PR_SET_PDEATHSIG` in the child) closes the case of SIGKILL.

## Decision

An emulator never outlives the podvm that started it.

## Prove

On the cloud host, `timeout 30 podvm run --rm --podvm-tier=machine mirror.gcr.io/library/alpine:3.20 sleep 300` exits 124 and `pgrep qemu-system` then finds nothing, and the same with `timeout -s KILL`

## Done

**Done 2026-11-14.** `machine::die_with_podvm` sets `PR_SET_PDEATHSIG`
to SIGKILL in the child of the emulator, and refuses the start where
podvm is already gone; `drive` (`machine/guest/drive.rs`) and `machine
ssh` (`machine/ssh/qemu.rs`) take it. Both spawn from the thread that
waits for the emulator until the end of podvm. By hand on the cloud
host, with `run --rm --podvm-tier=machine alpine:3.20 sleep 300`:
`timeout -s TERM 25` gives 124 and `timeout -s KILL 25` gives 137, and
no `qemu-system` stays after either; beta.17 left one after the KILL
(ended by hand). The signal and the bounded reap of the Approach are
not needed: the kernel ends the emulator with podvm.


# T-1885: Piped input to a machine run with `-t` is lost before the guest terminal opens

**Source:** the check of T-1817 (2026-11-14)
**Category:** machine
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** open

## Problem

`printf 'machine-in\n' | podvm run --rm --podvm-tier=machine -t
alpine:3.20 cat` ends in 3 s with exit 0 and prints no `machine-in`;
without `-t` the same run prints it. beta.17 printed nothing either (it
hung).

## Premise

The pump of `machine/guest/tty.rs` connects to the socket of the
console as soon as QEMU listens and writes stdin at once, before
`podvm guest-tty` in the guest opens `/dev/hvc0`; the bytes and the
VEOF of T-1817 reach a console that no reader holds.

## Approach

Hold stdin until the guest says that its terminal is open (a marker that
`guest-tty` writes on the console, which the pump waits for), then
relay.

## Decision

A `-t` run gets each byte of its stdin, as a run without `-t` does.

## Prove

`printf 'machine-in\n' | podvm run --rm --podvm-tier=machine -t mirror.gcr.io/library/alpine:3.20 cat` prints `machine-in` and exits 0 under TCG on the cloud host
