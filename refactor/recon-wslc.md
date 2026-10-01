# Recon: `wslc`, the WSL container feature

> **⛔ SUPERSEDED IN PART. Read [INDEX.md](INDEX.md) section 5.2 instead.**
>
> This report was written when `wslc` was believed **absent** from this machine,
> because `which wslc` fails and the Git Bash `PATH` does not carry it. It is
> **installed**, at `C:\Program Files\WSL\wslc.exe`, version **3.0.1.0**, and
> the operator supplied that path. Every statement below that says `wslc` is
> not installed, is absent, or would need a WSL upgrade is **wrong**: the
> affected lines are 171, 307, 334, and 407.
>
> What the research got right, and what still holds: `wslc` is a real,
> generally available Microsoft Linux container CLI, it is closer to docker and
> podman at the command line than to `wsl-toolkit`, and it is MIT licensed.
> What it got wrong on the merits: `wslc` **is** a better fit than this report
> concluded, because it bind-mounts the checkout and runs the suite in about 40
> seconds where the repository's own lane copies 343 MiB first.
>
> The recommendation below stands inverted: **use `wslc` for bounded read-only
> proofs, with the two limits INDEX.md section 5.2 records** (uid 0, and it
> wedges if a job is killed). The two limitations this report could not
> establish, because it never ran the binary, are now measured.

## Result

`wslc` is real, shipped by Microsoft, and generally available. It is a Linux
container CLI that came with WSL 3.0 on 2026-09-29. The operator's description
of it as "very similar to docker/podman" is correct at the command line and
wrong at the isolation model. The difference decides this document.

Recommendation: **MONITOR**. Do not adopt it for the current work. The reason
is in section 5, and it is not the usual one.

This is a research record. It ran no `wslc` command, because `wslc.exe` is not
installed on this machine. Every claim is read from a source or marked as an
inference.

## Sources

Every external claim below carries its URL. A claim without one is marked
`INFERENCE`, and an inference is not evidence.

| Ref | Source |
| --- | --- |
| S1 | <https://learn.microsoft.com/en-us/windows/wsl/wsl-container> |
| S2 | <https://learn.microsoft.com/en-us/windows/wsl/tutorials/wsl-containers> |
| S3 | <https://devblogs.microsoft.com/commandline/wslc-architecture-deep-dive/> |
| S4 | <https://devblogs.microsoft.com/commandline/wsl-container-is-now-available-for-public-preview/> |
| S5 | <https://blogs.windows.com/windowsdeveloper/2026/09/29/wsl-containers-now-generally-available/> |
| S6 | <https://github.com/microsoft/WSL/releases/tag/3.0.1> |
| S7 | <https://github.com/microsoft/WSL/releases> |
| S8 | <https://raw.githubusercontent.com/microsoft/WSL/master/LICENSE> |
| S9 | <https://github.com/microsoft/WSL/issues?q=wslc+privileged> |
| S10 | <https://github.com/microsoft/WSL/issues?q=is%3Aissue+label%3A%22feature+wslc%22> |
| S11 | <https://wslcontainers.com/> |

S11 is a third party. It describes `wslc` 2.9.4.0, which predates GA. It is
used here for the shape of the flag surface and for nothing else. Every claim
taken from it is marked `THIRD PARTY` and dated to the version it tested.

## 1. What `wslc` is

### The binary

- **Name and spelling.** `wslc.exe`. A separate executable, not `wsl.exe` under
  another name and not a `wsl.exe` subcommand. S1: "WSL now includes `wslc.exe`
  as a built in binary." S4: "you will get a new binary on your path: `wslc.exe`".
- **Invocation.** `wslc <verb>`, for example `wslc run --rm -it ubuntu:latest
  bash`, `wslc image list`, `wslc container list`, `wslc build -t name .`. S1
  lists the same verbs.
- **Alias.** `container.exe` runs the same binary. S5: "or use its built-in alias
  `container.exe` to run the same familiar container commands".
- **Delivery.** It ships inside WSL. There is no separate engine to install. S2:
  "`wslc.exe` is included with WSL, so there's no separate engine to install."
  It requires WSL 2.9.3 or later. S1 and S2 both state the floor.
- **Version check.** `wslc version`. S2.

### What it runs

Linux containers, not Linux virtual machines. Every container in a session
shares one utility VM. S2: "you have now built and run a Linux container on
Windows using `wslc.exe`, powered by WSL 2".

### Its place beside `wsl.exe`

Three separate programs share a service:

- `wslservice.exe` is the privileged Windows service. S3: "client processes call
  into 'wslservice.exe', which is a privileged Windows service. That service has
  the capability to create virtual machines (via HCS)".
- `wslcsession.exe` is a new per-user child process. S3: "wslservice.exe does not
  retain ownership of the virtual machine. Instead, it creates a child process,
  `wslcsession.exe`, which runs on behalf of the calling user and will perform
  all session operations (creating containers, mounting directories, binding
  networking ports, etc) on behalf of the user."
- `wslc.exe` is the client. S3: "a new Linux container platform comes with
  multiple architectures changes compared to WSL".

The session process is the difference that matters. S3 gives its reason:
"strong isolation between sessions, since they live in different processes, but
also reinforced security boundaries, since sessions operations are executed in a
less privileged process than wslservice.exe".

`wsl.exe` is not replaced and is not affected. A WSL distribution and a `wslc`
session are different objects with different storage.

### Storage

S3 names two volume kinds and one session store.

- A session VHD per session, at `%AppData%\Local\wslc\sessions`. S3: "these VHDs
  are stored in `%AppData%\Local\wslc\sessions`". S5 adds "a configurable
  storage path for the default wslc session".
- A Windows path volume becomes a virtiofs share. S3 gives the command
  `wslc container run -v C:\Windows\System32\drivers\etc:/volume -it debian:latest
  ls /volume` and says "these volumes are implemented by mounting virtiofs shares
  inside the Linux virtual machine, and making them available to the container.
  Inside the virtual machine, these mounts are created under `/mnt`, and then
  attached to the container as bind mounts".
- A VHD volume is a real ext4 disk. S3: "`wslc volume create --driver vhd
  -o SizeBytes=200000000 my-volume my-volume`" produces `/dev/sdf ext4`.

### Networking

S3 names the model: Consomme. "all the Linux virtual machines' traffic is sent as
ethernet frames to a virtio queue, which is then read by a Windows process
running on behalf of the user". That process answers DNS and routes UDP and TCP.

S3 states the benefit, and it is a host claim rather than a container claim:
"the traffic that's routed out of the virtual machine is sent on behalf of the
user owning the wslc session, so traffic flows as it was emitted by a regular
Windows process, which provides extensive compatibility with VPNs and firewalls".

### Isolation model

This is the part that decides section 5, so it is separated into what is written
and what I could not read.

**Written.** The container sits inside one WSL 2 utility VM. S2 confirms the
container runs "on the WSL 2 Linux kernel". S3 says the plugin is a Plan9 host
plugin and that virtiofs replaces it for container volumes. Sessions are
separated by process, not by a hypervisor boundary per container.

**Not written, and I looked.** No Microsoft source in S1 through S5 states
which Linux namespaces a `wslc` container receives. There is no table of
namespaces, mounts, devices, and capabilities. The operator asked for that table.
It does not exist in the sources I read.

**Read from the issue tracker instead.** S9 shows the default is restrictive and
that the escape hatches are not built:

- #41503, open, label `feature wslc`: "wslc: containers can't use AF_VSOCK:
  seccomp blocks socket(AF_VSOCK) with no `--security-opt`/`--privileged` escape
  hatch". A seccomp filter is present, and there is no documented way past it.
- #41597, closed as abandoned: "wslc: add USB device passthrough (`--usb`,
  `--privileged`)".
- #41174, closed as abandoned: "Add privileged container support for FUSE
  workloads".
- #41681, open, in progress: "Add `--cap-add` and `--cap-drop` to `wslc run` and
  create for both cli and sdk". Capability control did not exist.
- #41181, open, label `feature wslc`: "Privileged container for fuse mount".

The pattern is consistent across five issues. `--privileged` is not implemented
and two requests for it were closed. `--cap-add` and `--cap-drop` were still
being written. S4 named the goal "containing a Linux process' access to
resources on the host", but no source states which host resources are denied.

**The gap that matters most for podbox is FUSE.** `docs/limits.md` records that
"Forced FUSE mode enters through a rung-complete read-only server". A read-only
FUSE server needs `fusermount` or `mount(2)` inside the container. Issue #41181
is a request for exactly that and it is open. `THIRD PARTY` S11, at 2.9.4.0,
also records that the FUSE paths were not available.

I did not confirm the namespace set, the device set, or the ownership
behaviour. I am reporting the absence of documentation, not a capability list.

### Maturity

Generally available, as of 2026-09-29.

- S6, the 3.0.1 release body: "WSLc is generally available."
- S5, dated September 29, 2026: "a new feature in WSL: WSL containers, which is
  generally available today". The post is by a Windows Platform corporate
  vice president.
- S6 also carries the cleanup commit "Remove references to WSLC being in preview
  by Blue in #41704", so the source no longer calls it preview.

The public preview was 2026-06-29. S4 is dated June 29th, 2026 and ends "We aim
to make this feature generally available in the upcoming fall of 2026." GA
landed about three months later.

Version on this machine: WSL 3.0.1.0, kernel 6.18.4.0-1. `wslc.exe` is absent
from `C:\Windows\System32`, from `C:\Users\AjamX\bin`, and from scoop shims. The
only `wsl`-family files present are `wsl.exe`, `wslapi.dll`, and `wslconfig.exe`.
A Microsoft Store or `wsl --update` WSL 3.0 install is what carries the binary.

### Licence

MIT, from S8: "MIT License. Copyright (c) Microsoft Corporation."

podbox is 0BSD. `Cargo.toml:26` states `license = "0BSD"` and `LICENSE` carries
the 0BSD text. `references/Azathothas__container-research/tree/TOOL.md` section
"Licence" states the rule directly: "MIT and Apache-2.0 code can be vendored and
redistributed inside a 0BSD project, and its notice stays with the files it
covers".

**INFERENCE.** podbox would not vendor `wslc` source, because `wslc` is a
Windows binary that requires `wslservice.exe` and HCS. A licence question does
not arise. The obligation that does arise is the same one `wsl-toolkit` already
carries: record the external tool in `TODO/reference-map.md` with its version and
its licence, per the tree procedure.

### Against `wsl-toolkit`

Overlap is partial and runs in one direction.

| | `wsl-toolkit` 6.0.0 | `wslc` |
| --- | --- | --- |
| Origin | third party, `C:\Users\AjamX\bin` | Microsoft, inside WSL |
| Provides | a named WSL distribution, a rootless Podman, a toolset, Windows directory grants | a container CLI and a container API |
| Owns a distribution | yes, `wsl-toolkit-podbox` | no, sessions use their own VHD |
| Rootless engine | yes, Podman | n/a, there is no daemon |
| Windows job runner | yes, `run-in-base.sh` is its caller | no |
| Licence read | not established here | MIT (S8) |

`docs/containers.md` states what `wsl-toolkit` does for this project: "The
`wsl-toolkit` binary owns its command reference", it names the instance
`podbox`, and it holds the image cache and the job records across sessions.

**INFERENCE.** `wslc` complements and does not replace. `wsl-toolkit` is a
development base with a job runner. `wslc` is a container runtime. A `wslc`
container replaces the Podman container inside a `wsl-toolkit` base, if anything,
and the base itself is a distribution that `wslc` does not manage. S3 says
nothing about creating or importing distributions.

The one place `wslc` could take work is the container that
`scripts/windows/run-in-base.sh` starts for a Linux job. Section 4 says why that
does not pay.

## 2. podbox's contract

`README.md` states it: "podbox runs Linux payloads on hosts that can deny
namespaces, mounts, device creation, and ownership changes. It accepts Docker and
Podman commands and reports the behavior it can provide."

`TOOL.md` section 2.0 makes the target a floor: "`podbox` must work in an
environment **more restricted than this one**, and degrade honestly rather than
break when it meets one." It lists what must be a probe result and never an
assumption, including "whether `clone` with namespace flags is permitted at all,
whether `chroot` is permitted, whether `/dev/ptmx` exists, whether `seccomp` can
be stacked".

`TOOL.md` section 2.2 names the four walls: ownership (`chown` to an unmapped
id returns `EINVAL`), credentials (`setgroups` is `EPERM`), interposition reach
(`LD_PRELOAD` never sees a Go program's `lchown`), and mounts.

`docs/limits.md` is the honest record. The first line of the host section:
"A chroot changes paths. It does not provide process or network isolation."

### Which claims depend on a Linux engine podbox does not own

| Claim | Source | Engine required |
| --- | --- | --- |
| Image acquisition and layer extraction | `docs/architecture.md` | none, podbox does it |
| `chroot` rung | `docs/architecture.md` | a Linux kernel and permission |
| namespace rung, mount namespace only | `docs/architecture.md` | kernel namespaces, often refused |
| userland and interpose rungs | `docs/architecture.md` | no engine, that is the point |
| machine tier, KVM TCG DOS | `docs/architecture.md` | QEMU and nested virtualization |
| SSH transport | `docs/architecture.md` | none |

podbox's own rungs are the claim. A `wslc` container is a different and stronger
machine, so it does not validate any of them. `docs/limits.md` says so for
KVM already: "An earlier KVM success does not establish current reliability."

### What `wslc` would replace, complement, or compete with

**Replace: nothing.** podbox has no container engine. The parity table at
`crates/podbox-cli/src/parity.rs:232-537` holds 265 rows, verified by counting
`    Row {` in that file. No row names `wslc`.

**Compete with: the parity table, and this is the important one.**
`TOOL.md` section 2.0 requires podbox to "answer to `docker` and `podman` on
PATH". The table is where podbox states the gap between what docker accepts and
what podbox grants. If `container.exe` is on PATH on a Windows host, it answers
to `container`, and podbox's honesty story is a manual opt-in to the weaker tool
on a host that has a stronger one. That is a product argument, not a technical
one, and it is the only place `wslc` touches podbox's contract.

**Complement: the guest image.** A `wslc` container could host a Linux payload
for a user who has WSL and not podbox. That is a new entry point, and it is
outside the current work.

**Name the three KEEP-SHELL subjects that name `wsl-toolkit`.**
`refactor/06-entries/PLAN.md` section 2 records 29 KEEP-SHELL verdicts that "need
QEMU or an emulator, `wsl-toolkit` (3), a live registry or relay". Those three
scripts keep their justification. `wslc` does not own a distribution, so it
cannot serve them.

## 3. The current work

`refactor/06-entries/PLAN.md` retires 121 shell, Python, and PowerShell scripts
across six waves. Wave 0 records fixes, wave 1 deletes what is already in Rust,
wave 2 fills inline gaps, wave 3 adds the first `tests/` directories, and waves
4
and 5 add four new crates.

Section 6 of the plan names what blocks the work on this host. One row is:
"`cargo test --workspace` on this host", blocked by "`linker 'cc' not found`",
cleared by "the Linux base lane, `sh scripts/windows/run-in-base.sh`".

### The Linux lane is not on the critical path

`.github/workflows/gate.yml` runs all four jobs on `ubuntu-latest`. A count of
`windows-latest` and `run-in-base` in that file returns 0. The gate never runs
on Windows, so no wave is blocked in CI by the base.

The base lane is a local convenience on a Windows host. It is where a developer
on this machine runs `cargo test --workspace` and the full Linux check. The plan
needs it. Nothing in the six waves becomes feasible without it, and nothing in
the six waves is proven by it either.

### What `wslc` would change about that lane

`run-in-base.sh` copies the tree into a disposable container in
`wsl-toolkit-podbox` and runs the job there. A `wslc` container would do the same
copy-and-run. Four differences follow from section 1.

1. `wslc` is not installed on this machine. Adopting it means a WSL upgrade, and
   `docs/containers.md` records the state this tree is in: "The operator removed
   the WSL distributions and stopped the processes" in the 2026-09-30 recovery,
   per `TODO/PROGRESS.md`. The base was rebuilt with `wsl-toolkit --instance
   podbox base ensure --probe` and holds the image cache across sessions.
2. `wslc` has no cgroup delegation story for this project. `docs/limits.md`
   records the current base state: "On the recorded host, engine memory and CPU
   limits are accepted but not enforced." A `wslc` session takes `CpuCount` and
   `MemoryMB` through `SessionSettings` (S1), and no source states whether a
   `wslc` session enforces either. Swapping engines does not fix an
   unenforced limit, and it adds a second unenforced claim to probe.
3. `wslc` has no job-record and garbage-collection surface. `docs/containers.md`
   depends on both: "`wsl-toolkit --instance podbox gc --job JOB_ID --apply`"
   and "The host job directory keeps the transcript until `gc` removes it".
4. The plan's blocked row is a Rust registry fixture, not a missing engine. Its
   stated blocker is "there is no Rust OCI registry fixture in this tree", and
   the row that clears it is "build a Rust registry fixture first". `wslc` does
   not change that.

**INFERENCE.** `wslc` would make the lane neither faster nor more reliable in a
way this repository can measure, and it would not be unnecessary. The cost is a
tool swap with no measurement behind it. The plan does not ask for the swap.

## 4. Adoption cost

| Cost | Detail |
| --- | --- |
| Not installed | `wslc.exe` is absent on this host. WSL 3.0.1.0 is present and does not carry it. |
| A new external tool | a second container engine name in a tree that names one, against `docs/agent-tooling.md`: "Use an installed tool before you add another implementation." |
| A new failure surface | every `wslc` open issue becomes a dependency question. #41503 shows a seccomp profile with no escape hatch. |
| Licence | MIT is compatible with 0BSD. This cost is zero. |
| Version pinning | `wslc` versions with WSL, not with a manifest. A `wsl --update` can change the engine under the repository. |
| Provenance record | a new row for `TODO/reference-map.md`, per the licence rule. |

The version pinning row is the one that costs the most, and it is the same shape
as the KVM problem. `docs/limits.md` records: "Nested KVM in the Windows base can
stop the Windows host. On 2026-09-30 a KVM proof left an emulator that SIGKILL
did not remove, and the host then failed." `TODO/PROGRESS.md` records the same
event and names the recovery.

`wslc` cannot stop a host the way nested KVM did. It is a different risk and it
should not be written as the same one. The real point is narrower: this
repository was hurt by a host-level primitive it had adopted, and `wslc` is
another host-level primitive with a short public history. `docs/limits.md` also
records open host WSL defects under the same product:
`[WSL 2.9 regression] Rootless Docker/Podman cannot start any container`,
issue #41492, open at the time of this writing.

**The maturity argument is weak here, and I should not lean on it.** `wslc` is
GA, not preview. S6 says so in the release body. An argument that rests on "it is
a preview feature" would be wrong, and the operator's framing in the assignment
was that it was new. It is new. It is not preview. The reason for the
recommendation is the isolation model in section 1, plus the fact that the
current work does not need a container engine at all.

## 5. Recommendation

**MONITOR.**

### What `wslc` would have to be before adoption

For the parity table, the contract in `TOOL.md` section 2.0, and podbox's
honesty rule, the container has to let a payload reach the four walls and a
denial. A payload that cannot call `mount(2)`, cannot use FUSE, cannot run
`fusermount`, and cannot be granted capabilities is a container podbox cannot
use, because every podbox rung is a statement about what a payload could not do.

Specifically, these would have to hold, and none holds today:

1. `TOOL.md` section 2.1 mechanism N, a user namespace with a single-entry map
   and `setgroups` denied, must be reachable or refusable with podbox's own
   errno discipline.
2. A FUSE path must exist, for the read-only forced-FUSE mode in
   `docs/limits.md`. Issue #41181 is that request and it is open.
3. `--cap-add` and `--cap-drop` must ship, for issue #41681, so a capability can
   be added or denied by name rather than by inference from an error.
4. `--privileged` must exist, or its absence must be documented as a supported
   refusal. Issues #41597 and #41174 were closed as abandoned.

### What would reverse the decision

Any one of these moves MONITOR to ADOPT-LATER:

- Microsoft publishes a namespace, device, and capability table for `wslc`
  containers. The absence is the load-bearing gap in this document.
- Issue #41181 closes with a FUSE path, and #41681 ships capability control.
- A podbox rung becomes unavailable in the `wsl-toolkit` base and the failure is
  traced to the base rather than to the host. That would be a real trigger,
  because it would be a measurement rather than a forecast.
- The `wsl-toolkit` project stops. Then this is a successor question and the
  scope is a migration.

### What would confirm the rejection

A `wslc` container cannot host a forced-FUSE podbox payload and cannot be
measured to say why. The `wslc` seccomp profile at #41503 suggests it.

### What is not recommended

Do not add `wslc` to `docs/containers.md` as a lane, and do not add it to
`docs/agent-tooling.md` as an installed tool. It is not installed, and
`docs/agent-tooling.md` requires the opposite: "Run its version or help command."
until a version has been read.

## 6. Three facts this document could not establish

1. **The namespace, mount, device, and ownership set of a `wslc` container.** No
   Microsoft source in S1 through S5 states it. The only direct evidence is
   indirect: issue #41503 reports a seccomp filter, and #41681, #41597, #41174,
   and #41181 report missing capability and privilege controls. I read the
   absence of documentation. I did not read a specification, and I did not run
   the binary.
2. **Whether a `wslc` session enforces `CpuCount` and `MemoryMB`.** S1 shows both
   on `SessionSettings`. No source states enforcement. The question matters
   because `docs/limits.md` records that the current base accepts limits and does
   not enforce them, and an unenforced limit on two engines is worse than one.
3. **Whether `wslc` sessions and the `wsl-toolkit-podbox` distribution can
   coexist on one host after the 2026-09-30 recovery.** `TODO/PROGRESS.md`
   records that the operator removed the WSL distributions during that recovery
   and rebuilt one. S3 describes per-session VHDs at
   `%AppData%\Local\wslc\sessions` and says nothing about distributions. The
   question was not tested and was not safe to test, because
   `docs/containers.md` states "Never call `wsl.exe` from this project" and the
   only way to check is `wsl --list`.

## Scope

Read: `README.md`, `docs/architecture.md`, `docs/containers.md`,
`docs/limits.md`, `docs/agent-tooling.md`, `docs/conventions/prose.md`,
`TODO/PROGRESS.md`, `TODO/INDEX.md:38-60`, `Cargo.toml`, `LICENSE`,
`refactor/06-entries/PLAN.md` in full,
`references/Azathothas__container-research/tree/TOOL.md` sections 2.0, 2.1, 2.2,
Licence, 3.5, and the heading index, `.github/workflows/gate.yml`, and
`crates/podbox-cli/src/parity.rs`.

Not read: `TOOL.md` sections 4 through 6 in full, the other `TODO/` entries, the
ten group reports under `refactor/01-audit/`, and the experiments corpus.

Ran: no `wslc` command, no install, no commit. Verified the 265-row parity count
and the `gate.yml` runner count with a direct count. Fetched S1 through S11.
Ran `scripts/common/check-docs.sh` and `scripts/common/check-markers.sh`. The
marker check first reported two non-ASCII characters in this file, an em dash
in a quoted issue title and an accented letter in a product name. Both are
corrected. The check also reports eight in `refactor/07-verify/verify-waves.md`,
which this file did not touch and does not own.
