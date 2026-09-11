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
Status:      open

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
                [RULES.md](RULES.md) section 8.
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

---

### T-1302 One binary, one parity table, and a VM-only flag that cannot collide

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document; [cli.md](cli.md) T-0803, T-0808
Category:    podvm
Priority:    P1
Effort:      M
Status:      open

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
Decision:    Not taken. ⚠ Two candidate shapes and the trade is real: a tier
             flag on one command line reads naturally to a person, and an argv0
             alias is what an agent's existing scripts already produce. They are
             not exclusive, and the entry should rule whether BOTH are offered
             or only one.
Prove:       `podbox --help` and `podvm --help` list one set of verbs, and `./experiments/145-podvm-parity.sh` drives every row from the flag rather than from the table

---

### T-1303 The image is a rootfs directory, and an initramfs with no console is a silent machine

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 1 and 2
Category:    podvm
Priority:    P1
Effort:      L
Status:      open

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

---

### T-1304 The exec protocol is a serial pair with a nonce, and every wait is bounded

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 4
Category:    podvm
Priority:    P1
Effort:      L
Status:      open

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
Decision:    Not taken. ⚠ The marker format is a wire contract and a payload can
             print anything, including a line that looks like a marker. The
             entry must rule how a marker is made unforgeable: a per-command
             nonce is the obvious answer and it should be written down rather
             than assumed.
Prove:       `./experiments/147-podvm-exec.sh` asserts a zero status, a non-zero status and a deadline are each reported distinctly, and that a payload printing a marker-shaped line does not end the command

---

### T-1305 The fleet and the fork, and the file-size ceiling that bounds both

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, sections 3 and 5
Category:    podvm
Priority:    P2
Effort:      L
Status:      open

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
Decision:    Not taken. ⚠ The question is whether a fleet is podbox's job at all.
             `docker` has no fleet verb, so a fleet is an extension, and this
             file's opening rule says an extension may not confuse an agent that
             knows docker. ⛔ Rule it explicitly rather than growing one verb at
             a time.
Prove:       `./experiments/148-podvm-fleet.sh` asserts a guest over the file-size ceiling is refused BEFORE it starts, with the ceiling in the message

---

### T-1306 The non-goals are refusals the code makes, not notes in a document

Source:      `https://github.com/talaria0101/vm-research`, its podvm-spec document, section 7
Category:    podvm
Priority:    P1
Effort:      M
Status:      open

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
Prove:       `./experiments/149-podvm-non-goals.sh` asserts each refusal names its leg and its errno, and that a leg the host permits turns the refusal off
