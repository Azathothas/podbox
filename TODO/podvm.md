# podvm

The machine tier. podbox runs a payload on the host kernel; `podvm` runs one on
a kernel of its own, and it answers to the same verbs, flags and exit codes.

⛔ **It is a mode of podbox and not a second product.** One binary, one parity
table, one set of exit codes. A second product means two parity tables, and a
flag that means one thing in one and another thing in the other is the defect
[cli.md](cli.md) T-0803 already rules on for the `docker` name.

⭐ **Read this first, because it is the finding that shapes every entry here.**
Two restricted runtimes were studied and **they do not have the same walls**:

| | the runtime `podvm` was specified against | the runtime podbox was specified against |
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
  [T-1111](milestones.md) drives through podbox.

---

### T-1301 The machine tier is probed leg by leg, and a present file is not a working one

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 0 and 3; [probe.md](probe.md) T-0103
Category:    podvm
Priority:    P0
Effort:      M
Status:      done

Problem:     podbox reports a rung and never claims a stronger one than it
             achieved. The machine tier has no probe at all, so `podvm` today
             could only assert it. ⛔ **Every way to get this wrong produces a
             false report rather than an error**, which is the class
             [`../docs/methodology/reviews.md`](../docs/methodology/reviews.md)
             exists for.
Premise:     ⭐ **podbox already holds the discipline and two of the legs.**
             [T-0103](probe.md) probes creation and attachment separately and is
             done; [T-0102](probe.md) separates a filtered syscall from an
             executed one with a bogus argument; [T-0109](probe.md) keeps
             "could not run" distinct from "denied". The machine tier adds legs
             and reuses all three rules.
             ⚠ The specification measures `/dev/kvm` ABSENT and uncreatable on
             its own target, and the emulator therefore runs under TCG. ⛔ A
             boundary under TCG is the emulator process and not hardware, so the
             banner may never say hardware isolation.
Approach:    Add a `machine` group to `podbox probe`, with one leg per fact and
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
Decision:    The tier is refused when any leg is missing, and the refusal names
             the leg. ⛔ Not a reduced machine tier: the ladder in
             [`../docs/architecture.md`](../docs/architecture.md) says a weaker
             mode never satisfies a stronger request, and a machine tier with no
             acceleration and no network is two different products depending on
             which leg failed.
Prove:       `podbox probe --json | jq -e '.tiers.machine.legs | length >= 5 and (map(select(.verdict == null)) | length == 0)'`

**Done 2026-09-21.** Six legs in `Group::Machine`
(`crates/podbox-probe/src/probes.rs`, `MACHINE_LEGS`), one verdict per
leg, read back through `crates/podbox-probe/src/machine.rs`, which refuses
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

---

### T-1302 One binary, one parity table, and a VM-only flag that cannot collide

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document; [cli.md](cli.md) T-0803, T-0808
Category:    podvm
Priority:    P1
Effort:      M
Status:      done

Problem:     `podvm` has to feel like `podman` and `docker`, because its audience
             is automated and an agent reaches for the verbs it knows. ⛔ A
             second binary is a second parity table, and
             [cli.md](cli.md) T-0808 already drives 141 rows through the shipped
             binary because a flag accepted and unlisted is invisible from the
             table's side.
Premise:     podbox already answers to more than one name:
             [cli.md](cli.md) T-0803 rules that podbox refuses the `docker` name
             where a working daemon is reachable unless a flag says otherwise.
             The mechanism for a third name therefore exists and is ruled on.
Approach:    `podvm` is `podbox` invoked under that name, or `podbox` with the
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
Decision:    ⭐ **Taken on 2026-09-12: BOTH, and the flag is the primitive.**
             `--podbox-tier=machine` selects the tier, and `podvm` is a third
             name in `crates/podbox-cli/src/names.rs`'s `ALIASES` whose only
             effect is to change that flag's DEFAULT. There is one
             implementation and one parity table; the table lists the flag, and
             the name is an entry point to it rather than a second code path,
             which is what [cli.md](cli.md) T-0808 needs.
             ⛔ **The collision rule, because `podvm --podbox-tier=chroot` is
             otherwise undefined: the explicit flag wins over `argv[0]`, and
             podbox states the tier it selected whenever the two disagree.**
             The tier is not cosmetic. This entry's own `Approach` names
             `--memory` as enforced by the machine tier and not by the chroot
             tier, so a caller who cannot tell which tier ran cannot tell
             whether a limit was honoured.
             ⚠ **Neither shape alone survives this entry's own `Prove`**, which
             asks that `podbox --help` AND `podvm --help` list one set of verbs
             and that the parity experiment drive every row **from the flag**.
             A name with no flag has nothing to drive; a flag with no name
             leaves `podvm --help` unanswerable.
             ⚠ **T-0803's refusal rule does not extend here.** That ruling is
             about taking another tool's name where that tool works: `docker` is
             refused where a daemon answers. `podvm` is podbox's own name, so
             there is no foreign daemon to check for and no refusal to make.
             The banner obligation is the tier, not the name.
             ⚠ **The prefix is fixed by this decision** so an implementer does
             not reopen it: `--podbox-` is a prefix neither docker nor podman
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
             ⭐ **podbox takes the repeatable-flag shape**: one token per
             occurrence, repeated as many times as needed, and no splitting of
             anything. ⚠ **Rejected:** splitting on whitespace with a shell word
             splitter, which is what that report suggests as its first option.
             It is friendlier to type and it puts a quoting parser between the
             caller and the emulator, so a mis-parse becomes podbox's bug in a
             place podbox cannot test exhaustively. ⛔ docker and podman both
             take the repeatable form for this class of flag, and
             [cli.md](cli.md) is the parity rule that settles ties.
Prove:       `podbox --help` and `podvm --help` list one set of verbs, and `./experiments/145-podvm-parity.sh` drives every row from the flag rather than from the table

**Done 2026-09-21.** One flag and one name, in one implementation
(`crates/podbox-cli/src/tier.rs`): `--podbox-tier=machine|chroot` on `run`
and `exec` (which also serves `create`), and `podvm` as a third entry in
`ALIASES` whose only effect is to default that flag to machine. The
explicit flag wins over `argv[0]`, and `podvm --podbox-tier=chroot` states
the override and runs the chroot tier. The machine tier assesses T-1301's
legs before anything is pulled and refuses naming every missing one;
where it holds, podbox says the guest driver arrives with T-1303/T-1304
rather than running something else. `--podbox-qemu-arg` is repeatable,
one token per occurrence, never split, machine-tier-only and refused
elsewhere. `podbox --help` and `podvm --help` print one text.
`experiments/145-podvm-parity.sh` exits 0 on a lane-built binary: 19
driven, 0 mismatches (`experiments/results/podvm-parity.txt`).

Decisions taken inside the ruled shape: the flag takes exactly
`machine|chroot`, because those are the two tiers this binary knows; the
qemu arguments are parsed, tier-gated and carried for T-1303's driver, and
the collection contract (no splitting) is pinned by a parser unit test
rather than by a drive. The 325 driver needed no change: a bare new flag
reaches a message without a refusal marker, which clause 1 counts as
driven.

---

### T-1303 The image is a rootfs directory, and an initramfs with no console is a silent machine

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 1 and 2
Category:    podvm
Priority:    P1
Effort:      L
Status:      done

Problem:     podbox's image path ends in an extracted rootfs on disk
             ([extract.md](extract.md)), and the machine tier needs that same
             rootfs wrapped so a kernel can start in it. The wrapping is where
             the specification records the expensive failures.
Premise:     ⭐ **Three measured traps, each of which produces a machine that
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
Approach:    Reuse [extract.md](extract.md)'s output as the image. Build the
             initramfs from it with the console node appended, and keep the
             assembly in one script under `experiments/` so the measurement
             ships with it.
             ⛔ The ownership rule does not change: extraction stays
             ownership-neutral and the intended metadata stays in the sidecar,
             because the wall that forces it is the kernel's mapping check and
             not a property of the destination.
Decision:    A rootfs directory, not a disk image. ⚠ A disk image needs a
             filesystem the host can make, and the runtimes studied refuse
             `mount`, so the host cannot populate one it cannot attach.
Prove:       `./experiments/146-podvm-initramfs.sh` asserts the guest prints its ready marker and that the console node is present in the unpacked archive

**Done 2026-09-21.** `experiments/146-podvm-initramfs.sh` exits 0 on a
lane-built binary: the pinned Alpine image pulls and extracts through
podbox, the rootfs archives as newc cpio, an appended raw archive carries
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

---

### T-1304 The exec protocol is a serial pair with a nonce, and every wait is bounded

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 4
Category:    podvm
Priority:    P1
Effort:      L
Status:      done

Problem:     `podbox exec` hands a command to a process. The machine tier has no
             process to hand it to: the command crosses a serial line into a
             shell that may not be listening yet.
Premise:     ⭐ **The readiness handshake is the part that cannot be skipped.**
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
Approach:    One shot per command: a begin marker, the command, an end marker
             carrying the status, matched line-anchored. The status crosses the
             line as text and is then the process's own exit code, which is what
             [cli.md](cli.md)'s exit-code contract requires.
             ⛔ Every read has a deadline and the deadline is reported, per
             [RULES.md](RULES.md) section 8. A machine that stopped answering and
             a command that is still running are different states.
Decision:    ⭐ **Taken 2026-09-21: a per-command 128-bit nonce in every
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
Prove:       `./experiments/147-podvm-exec.sh` asserts a zero status, a non-zero status and a deadline are each reported distinctly, and that a payload printing a marker-shaped line does not end the command

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
`crates/podbox-complete/src/devices.rs:218` records. A bare `exit 3`
ends the very shell that must print the end marker, so the command
runs in a subshell and the reporter outlives it. The end-marker
pattern accepts the exact line and rejects six forgery shapes,
checked host-side against the same pattern text. The kernel pin stays
measured: the mirror serves 6.12.110 and the reference names 6.12.94.

Residual, not in this change: `curl -fsSL` in 146 and 147 carries no
`--max-time`, so a stalled mirror is the one network wait without a
ceiling. It predates this entry (146 introduced it) and changing 146
would invalidate T-1303's evidence, so it needs its own entry.

---

### T-1305 The fleet and the fork, and the file-size ceiling that bounds both

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 3 and 5
Category:    podvm
Priority:    P2
Effort:      L
Status:      done

Problem:     The machine tier's value over the chroot tier is that a guest can be
             copied, restarted and thrown away. Neither copying nor restarting
             is free, and the ceiling that bounds them is not the one a reader
             expects.
Premise:     ⚠ **`RLIMIT_FSIZE` bounds a single file, and a guest's memory image
             is a single file.** The specification measures a 500 MiB per-file
             ceiling on its target and a guest above it dying from `SIGXFSZ`.
             ⛔ That is a different resource from the writable allowance
             [RULES.md](RULES.md) section 8 already covers, and checking one does
             not check the other: a machine with terabytes free still kills a
             guest whose memory image crosses the per-file limit.
             ⚠ Every figure in the specification is that host's. Reproduce the
             mechanism and re-measure.
Approach:    Read `RLIMIT_FSIZE` in the probe ([T-1301](#t-1301-the-machine-tier-is-probed-leg-by-leg-and-a-present-file-is-not-a-working-one)),
             and refuse a guest whose configured memory would cross it, before it
             starts, naming the number. Fork is a quiesce, a copy of the memory
             image and a restore into a second guest.
Decision:    Ruled 2026-09-21 under the operator steer that podbox works on
             every environment with as many features and fallbacks as it can
             honestly carry. The fleet IS podbox's job, spelled with the verbs
             it already has: `run`, `ps`, `stop`, `rm`, `images` and `logs`
             over machine-tier guests. No new fleet verb: `docker` has none,
             so one here is the extension this file's opening rule forbids,
             and a second verb family is a second parity table
             [cli.md](cli.md) T-0808 would have to drive. Fork (quiesce, copy
             the memory image, restore into a second guest) is NOT this
             entry: no guest driver ships yet (T-1302's holds-branch still
             refuses), so there is nothing to fork; it stays future work for
             when the driver lands. What this entry ships is the bound both
             fleet operations share: `--podbox-mem` (machine-tier only,
             `--podbox-`-prefixed per T-1302, refused elsewhere), judged
             against `RLIMIT_FSIZE` BEFORE the legs, so an over-ceiling guest
             is refused naming both numbers on every machine. The `--memory`
             trap T-1302 names closes per tier: chroot keeps refusing
             `-m`/`--memory` (no cgroup to enforce with), machine enforces
             `--podbox-mem` against the file ceiling, and neither tier
             accepts a limit it silently drops.
             ⚠ Rejected: a `fleet`/`fork` verb family (a second table), and
             no fleet ruling at all (the Problem this entry states would stay
             open silently).
Prove:       `./experiments/148-podvm-fleet.sh` asserts a guest over the file-size ceiling is refused BEFORE it starts, with the ceiling in the message

**Done 2026-09-21.** `--podbox-mem` on `run` and `exec` (`create` rides
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

---

### T-1306 The non-goals are refusals the code makes, not notes in a document

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 7
Category:    podvm
Priority:    P1
Effort:      M
Status:      done

Problem:     The specification lists designs that were tried and do not work on
             its target: a TCP listener of any kind, anything KVM-accelerated, a
             runc-style OCI container, a user-mode Linux guest, an identity
             switch through `uid_map`, and a guest file over the size ceiling.
             ⛔ **A list of non-goals in a document is a list the next session
             re-discovers.** podbox's own rule is that a limit stated as a fact
             is inherited as a fact, so each of these has to be a probe result
             and a named refusal rather than a sentence.
Premise:     ⭐ **This is the entry that keeps the mode honest on a runtime that
             is not the one it was specified against.** Each non-goal above is
             blocked by a MECHANISM on that host, and every mechanism is
             something [probe.md](probe.md) can ask about. A mechanism that is
             open on another runtime turns a non-goal back into a goal, and a
             hard-coded refusal would hide that.
             ⚠ The specification itself records the contract moving under it: a
             network rule that denied a port early in a session and permitted it
             later. ⛔ So a refusal is re-probed per run and never cached across
             one.
Approach:    One probe leg per non-goal, and a refusal that names the leg, the
             errno and the remedy where there is one. The four-part diagnostic
             [cli.md](cli.md) T-0805 defines is the shape, and this reuses it
             rather than inventing a second one.
Decision:    A non-goal is a measured verdict with a date, never a constant.
             ⛔ Where the mechanism is genuinely unavailable the refusal says
             which leg failed, so a reader can tell "this runtime refuses it"
             from "podbox does not implement it".
             Ruled 2026-09-22 on the one fork the Approach leaves open: ONE
             new leg, five cited. Only the TCP bind had no measurement; the
             other five mechanisms already have probe rows (the kvm open, the
             UTS and user namespace rows, ptrace, setuid/setgroups, the fsize
             bound), and a second row measuring the same denial is the
             value-in-two-places drift the forbidden patterns refuse. The
             per-non-goal verdict lives in the new assessment, not in new
             legs. Cross-run staleness is bounded by the probe cache key
             (T-0111), like every other verdict in the document.
Prove:       `./experiments/149-podvm-non-goals.sh` asserts each refusal names its leg and its errno, and that a leg the host permits turns the refusal off

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

---

### T-1307 The five Rust VM tools, ruled one by one, so nobody surveys them again

Source:      [reference-map.md](reference-map.md); the sweep at
             [`../docs/history/2026-09-11-reference-sweep.md`](../docs/history/2026-09-11-reference-sweep.md)
Category:    podvm
Priority:    P2
Effort:      S
Status:      done

Problem:     Five Rust projects were named as candidate microVM managers for the
             machine tier. **A candidate list nobody rules on is a list the
             next session surveys again.** Each needs one verdict, with the
             reason recorded, so the survey is paid for once.
Premise:     **Read on 2026-09-11, and not one of the five is a microVM
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
Approach:    One row per tool in [reference-map.md](reference-map.md), which is
             written, plus the two mechanisms that survive the reading:
             1. **ask the emulator which accelerator works, never assume
                one.** `cubic` reached this independently: its pull request #538
                is `feat: ask qemu which accelerator works on the host` and its
                issue #6 is `Only use KVM if available`. T-1301 is where podbox
                does it;
             2. the named-configuration shape from `Vex`, which is what a
                `podvm` machine profile is, and which T-1303 already carries.
             Take no code from any of the five. Four permit it and the fifth
             does not, and none of the four has a line podbox needs.
Decision:    `Vex` **confirms** and `vml` **confirms**: independent evidence
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
Prove:       `./scripts/check-todo.py` resolves every row, and each of the five names its verdict and the tree line that settles its licence

**Done 2026-09-22.** Every row of [reference-map.md](reference-map.md)
names its verdict and the tree line that settles it, and
`./scripts/check-todo.py` resolves every row green. `Vex` **confirms**
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

---

### T-1308 One TCG number is a claim about one benchmark, and the range is 3x to 21x

Source:      `references/talaria0101__vm-research/tree/experiments/logs/66-tcg-benchmarks.log`
             and `references/talaria0101__vm-research/tree/experiments/logs/72-bench-matrix.log`;
             `references/talaria0101__sandbox-insights/tree/experiments/logs/55-tcg-exec-and-bench.log`;
             `references/Azathothas__sandbox-insights/tree/docs/open-questions.md`
Category:    podvm
Priority:    P1
Effort:      M
Status:      open

Problem:     [T-1301](podvm.md) selects the machine tier and the selection is a
             cost decision, so podbox will have to tell an operator what the
             tier costs. **Every source that states one number states a
             different one**, and a reader who takes any of them as "the TCG
             tax" will be wrong by up to seven times.
Premise:     **Measured, four times, on one host class, and the spread is
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
Approach:    Adopt that closure test rather than inventing one: integer,
             syscall, memory-bandwidth, compilation and I/O workloads, reported
             as a distribution rather than a point, with same-day native and
             chroot controls taken on the same host in the same run.
             Every row prints a checksum and the run is refused if two
             platforms disagree, because a platform that computed something else
             must say so rather than look fast.
             podbox quotes the **range and the workload**, never a single
             figure: the four-part diagnostic [cli.md](cli.md) T-0805 defines
             has room for it.
Decision:    **podbox never prints a bare multiplier.** Where the machine tier
             is selected the banner names the workload class and the measured
             range for it, or it names nothing. **Rejected:** carrying the
             "20-25x" figure as a default, which is a real measurement of
             somebody else's benchmark on somebody else's host and would be read
             here as a property of emulation.
Prove:       `./experiments/154-tcg-workload-spread.sh` prints one row per workload class with its checksum and its ratio, and exits 1 if any two platforms disagree on a checksum

---

### T-1313 Every experiment fetch carries its own ceiling, and a stalled origin proves it

Source:      `TODO/podvm.md` T-1304 residual; `experiments/146-podvm-initramfs.sh:153`,
             `experiments/147-podvm-exec.sh:119`, `experiments/152-nix-acceptance.sh:94`
Category:    podvm
Priority:    P1
Effort:      M
Status:      open

Problem:     Three experiment fetches wait on the network with no ceiling:
             the kernel fetch in 146 and 147 (`curl -fsSL`, no `--max-time`,
             no `timeout` wrapper) and the nix tarball fetch in 152. A
             stalled mirror turns any of the three into a hang no clause
             names. [RULES.md](RULES.md) section 8 gives every
             network-touching command a timeout, and podbox inherits that as
             a product requirement; the project's own scripts are not exempt.
Premise:     Every OTHER wait in 146/147 is already bounded (146 boots under
             `timeout BOOT_TIMEOUT`, 147 carries boot, command and handshake
             deadlines), and the tree already bounds fetches both ways:
             `timeout 300 curl` in `experiments/290-microvm.sh:91` and
             `--max-time 60` in `scripts/common/mine-repo.sh:416`. The fix is
             one flag per fetch, not a new mechanism.
             `experiments/280-insecure-registry.sh:287,291` needs no change:
             loopback health polls inside retry loops, where the loop is the
             bound.
Approach:    `--max-time` on all three fetches, default 60, env-overridable
             per script (`PODBOX_146_CURL_TIMEOUT` and kin, following the
             `BOOT_TIMEOUT` pattern already in 146/147) for a slow mirror.
             A stalled-origin clause in 146 proves the bound bites: a
             localhost server that accepts and never answers, fetched with a
             2 s ceiling, must exit 28 inside 10 s. A bound nobody drove red
             is a comment, not a ceiling. Changing 146 re-drives T-1303's
             evidence and changing 152 re-drives T-1111's, so all three
             scripts re-drive green in the same change.
Decision:    Recommendation: `curl --max-time`, not the `timeout` wrapper.
             The ceiling travels with the transfer rather than with the
             process, and 146 already gates on `curl`. Ruled out: leaving
             152 out because its re-drive is heavy (that leaves the heaviest
             fetch unbounded, which is the one that most needs the ceiling).
Prove:       `./experiments/146-podvm-initramfs.sh` exits 0 on a lane-built binary with the stalled-origin clause green
