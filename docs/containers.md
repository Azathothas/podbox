# containers.md

Measuring something this machine cannot measure, in a machine you throw away
afterwards.

⭐ **A procedure, not a dependency.** Nothing in [`../scripts/`](../scripts/)
needs a container and no gate does. A check that can use one **exits 2** when
there is none, because "could not run" and "failed" are different facts. A host
with no container engine is not a failing build.

---

## ⚠ What this page did not establish

⛔ **Almost every trap below was reported by somebody else and is not a
measurement taken for this page.** Each was paid for by a real session on a
real machine, none of which this repository can see, and this repository has
no host of its own to check them against. Two exceptions, both stated where
they appear: the raw-endpoint symlink behaviour was measured on 2026-08-30, and
the NAT default was confirmed on the one Windows 11 machine this was written
on.

⭐ **So treat the mechanisms as a list of things to check for, not as findings.**
[`methodology/experiments.md`](methodology/experiments.md) is what turns one
into the other, and ⛔ a number taken from here without re-measuring is a
number with no conditions.

---

## When this is the right answer

A newer browser than the one installed. A libc this host does not use. A
filesystem it does not have. A kernel feature that has to be registered and
then unregistered. Each needs a different machine, and waiting for CI to be
that machine costs minutes per question.

⚠ **When it is the wrong answer:** anything the host can already do. A
throwaway machine is not a way to avoid reading what is installed. Run the
probe first.

```bash
sh scripts/doctor/doctor.sh
```

---

## The tool

⛔ **This repository ships none.** [`agent-tooling.md`](agent-tooling.md) says
where they live and why they are not here. For a Windows host driving WSL2 that
is `wsl-toolkit` in
[`Azathothas/ToolKit`](https://github.com/Azathothas/ToolKit); for a POSIX host
it is a container engine you already have.

⚠ **The two are not twins and must not be treated as one.** A WSL distro runner
and a container engine solve different problems with different interfaces. Pick
by what the host has, and say in the write-up which one produced a number.

### ⛔ What the tool is, and how to call it, is not written here

**This page owns the PROCEDURE. Upstream owns the TOOL.** Nothing below
describes a flag, a subcommand, a product shape or a version. The split is
deliberate. A page that copies flags goes stale without anybody editing it.

Upstream ships one Windows executable. It generates its manual from the
commands it holds. The retired two-product wording lives in
[`history/upstream-tool-shape.md`](history/upstream-tool-shape.md).

⭐ **Upstream ships one agent skill per use, and each skill follows the
executable's own manual.** Read the one you need before you run anything.
When a link below stops resolving, list the directory that holds them:
[`skills/`](https://github.com/Azathothas/ToolKit/tree/main/skills).

| skill | answers |
| --- | --- |
| [`skills/wsl-toolkit`](https://github.com/Azathothas/ToolKit/raw/main/skills/wsl-toolkit/SKILL.md) | build and operate a base: what a base is, how to make one, how to grant it a directory, how to run a command in it |
| [`skills/wsl-toolkit-agents`](https://github.com/Azathothas/ToolKit/raw/main/skills/wsl-toolkit-agents/SKILL.md) | drive a coding agent inside a base, and read back the model and the effort it started on |
| [`skills/text-tool`](https://github.com/Azathothas/ToolKit/raw/main/skills/text-tool/SKILL.md) | write and edit a file from a shell that mangles the payload. [`conventions/shell.md`](conventions/shell.md) section 1 states why that matters. |

⚠ **Those three links name a branch, and every other fetch on this page is
pinned.** A page a reader opens is read. A pinned fetch is code that runs.
Pin what runs. Read what is current.

⚠ **Ask the binary for its version and its manual.** A version in a document
was true once. The tool reports what it holds today.

```powershell
wsl-toolkit --version
wsl-toolkit man --no-pager
```

⭐ **It also answers whether a machine can run an isolated Linux job at all**,
in one command, rather than leaving a session to infer it from three.
[`hosted-sessions.md`](hosted-sessions.md) is why that matters: the commonest
false claim about a provisioned machine is that it cannot do something whose
daemon was merely never started.

---

## ⭐ The procedure on a Windows host

⛔ **Never call `wsl.exe`, and never write another wrapper for it.** A payload
handed to `wsl.exe` as an argument is expanded before the guest sees it, and
the guest then parses the result a second time. `wsl-toolkit` sends every
payload on stdin or as a file. ⛔ `wsl --shutdown` is machine-wide: it stops
every distribution on the host, including the one that holds somebody else's
container engine.

⭐ **One instance, and it is `podbox`.** The flag `--instance podbox` names the
distribution `wsl-toolkit-podbox` and gives it its own state directory. Every
call names it: [`../scripts/windows/run-in-base.sh`](../scripts/windows/run-in-base.sh)
defaults `PODBOX_WSL_INSTANCE` to `podbox`, and a manual call passes the flag.

**The distribution persists and a container inside it does not.** The base
holds the engine, the image cache and the job records across sessions. Every
job runs in a container that is removed when it exits, so nothing a job
installs is there next time. That split is deliberate: it is the same shape the
hosted Linux environment has, so one set of scripts serves both.

### The three lanes

| the host | where the work runs |
| --- | --- |
| Linux | directly. [`../scripts/common/bootstrap-env.sh`](../scripts/common/bootstrap-env.sh) installs what is missing. |
| a container | the same as Linux. The container is already the machine. |
| Windows | the record and document checks run on the host. The build, the tests and every Linux measurement run in a container inside `wsl-toolkit-podbox`. |

⛔ **`scripts/session-start.sh` picks the lane. Do not pick it by hand.** It
reports the host, the time and the tools, then runs the right bootstrap. A
session that guesses the lane is a session that runs the Debian bootstrap
against an Arch base.

### Bringing the base up

```powershell
wsl-toolkit --instance podbox base status --probe
wsl-toolkit --instance podbox base ensure --probe
```

**`base ensure` needs a container engine on the Windows host**, because it
builds the distribution from an OCI image. Measured on 2026-09-11: with
`podman-machine-default` stopped, `base ensure` exited 2 and named the engine
rather than the image. `podman machine start` cleared it.

⛔ **That machine is not this project's.** Start it, use it, and put it back to
the state it was found in. `podman machine list --format json` reports
`Running` before and after.

### Running a job

```powershell
wsl-toolkit --instance podbox run --image docker.io/library/rust:1.98.1-bookworm --workspace . --exclude codegraph.db --exclude target --exclude .dev --script .\job.sh --timeout 45m --tick 120s
```

| the choice | why |
| --- | --- |
| `--exclude codegraph.db` | the local index is 230 MiB and is not an input to anything. ⛔ **The FILE, never the `.codegraph` directory**: that name also matches a tracked corpus file, and a copy one file short reads as a dirty tree in the guest |
| ⭐ `--exclude codegraph.db-wal`, `codegraph.db-shm`, `daemon.log`, `daemon.pid` | the live index's sidecars, and a file that GROWS during the copy breaks it. ⛔ By name, never `*.log`: 82 tracked corpus logs carry that suffix and each one is evidence |
| `--exclude target` | build output, and it is the wrong architecture on a Windows host |
| `--exclude .dev` | the background build's own log and state |
| `.git` is **kept** | `check-attribution`, `check-markers` and `restore-modes` all read the index |
| `references/` is **kept** | `check-todo.py` resolves every cited path and line in it |
| `--script`, never `-c` | the file travels as bytes. A command travels through two parsers. |
| ⭐ `--artifacts DIR` | what the job leaves in `/out` is copied to `DIR` on this machine. **The container is removed when it exits**, so a measurement that writes only into the workspace has written into something nobody can read. `PODBOX_ARTIFACTS` is how [`../scripts/windows/run-in-base.sh`](../scripts/windows/run-in-base.sh) passes it |

**Measured on 2026-09-11: the workspace is 8,406 entries and 164.6 MiB, and
the copy took about 4 s.** The image pull is the larger cost on a cold base.

### ⭐ Running engine clauses from the Windows host

A job container cannot start a docker daemon: it holds no `NET_ADMIN`, and
`dockerd` fails creating the `DOCKER` chain with `iptables ... Permission
denied`, measured 2026-09-19. Experiment clauses that need an engine do not
run there. They run on the host's own podman machine, which is rootful and
honours `--cap-drop`, measured the same day: a `--cap-drop=CHOWN` payload
fails `EPERM` with exit 1 and unchanged ownership.

[`../experiments/lib/engine.sh`](../experiments/lib/engine.sh) is the one way
scripts reach either engine. It prefers a docker daemon where one answers and
falls back to host podman, and the conditions block names which one drove.
What it enforces, because the workstation is shared:

- no `--privileged` and no `--cap-add`, refused outright;
- digest-pinned images only (`@sha256:`), refused otherwise;
- mounts read-only, and only from the checkout or the caller's scratch;
- a timeout on every call: runs take it as an argument, and create, copy
  and pull carry fixed bounds;
- created containers registered for removal by the caller's own trap.
  `wsl-toolkit gc` never sees these containers, so a script that leaves one
  behind leaves it for somebody else's disk.

Three traps belong to this lane. A Git Bash `/c/...` path arrives at the
Windows podman binary as garbage: spell host sources `C:/...` (the helper
translates) and carry `MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'` on every
engine call, which the helper does. `chmod` on this checkout is a silent
no-op, measured 2026-09-19, so a binary linked here runs only after a copy
to a filesystem that honours modes. `wsl-toolkit base exec` carries simple
commands reliably and mangles argument-carrying ones: a Linux binary off the
Windows automount receives shortened `argv`, and even staged into the base
the same call answered usage twice against five genuine runs, measured
2026-09-19. Commands that must receive file arguments run inside a container
through `engine.sh` instead, where `argv` arrives intact. The podman
machine stays running as found: never stop it, never prune without naming
what goes. A first image pull outlasts a clause timeout, so scripts fetch
their images before any timed clause starts rather than inside it.

### ⛔ Nine traps this host produced, five on 2026-09-11, two on 2026-09-12 and two on 2026-09-22

- ⛔ **A Windows checkout carries no executable bit, so every script arrives
  unrunnable.** Measured on 2026-09-11: **396 of 396** had to be repaired. NTFS holds no POSIX mode and `core.fileMode` is false there.
  The first failure reads `./scripts/common/bootstrap-env.sh: Permission
  denied`, which names the script and not the transfer.
  ⭐ [`../scripts/common/restore-modes.sh`](../scripts/common/restore-modes.sh)
  repairs it from the git index, which is the only record of which files are
  meant to run. ⛔ Never `chmod -R +x`: that marks data executable and nothing
  reports it.
- ⛔ **`/mnt/c` is mounted inside the base and it is writable.** A delete there
  destroys the real checkout on Windows. Read from it. Never write to it, and
  never point a build's output at it.
- ⚠ **A payload sent from PowerShell arrives with CRLF.** A POSIX shell then
  reads the carriage return as part of the last word, and `2>/dev/null` becomes
  a file called `/dev/null` followed by a carriage return. Write the payload
  with LF, or send it as a file.
- ⚠ **`python3` on this host is a Microsoft Store stub.** It prints an
  installation notice and exits without running anything. The real interpreter
  is `py`, at 3.13.15. A check that shells out to `python3` on Windows measures
  the stub.
- ⚠ **Python reads a script from stdin using the host code page, not UTF-8.**
  A marker character in the source is mangled before the program starts.
  Export `PYTHONIOENCODING=utf-8`, or pass a file.
- ⛔ **A file that grows while the workspace is copied stops the copy, and the
  error names the archiver rather than the file.** Measured on 2026-09-12 while
  the CodeGraph daemon was indexing: `! archive/tar: write too long`, and the
  job exited 2 in **475 ms**. A tar member's size goes into its header before
  its bytes are read, so a file that grows in between overruns what was
  declared. ⭐ The exclusions above cover the four sidecars the daemon writes,
  which are the only files left in the copy that anything writes in the
  background. ⚠ **The cause was not isolated to one of the four**, and the
  repair does not need it to be.
- ⛔ **STOPPING THE WRAPPER ON THE WINDOWS SIDE DOES NOT STOP THE JOB IN THE
  GUEST.** Measured on 2026-09-12: a background
  [`../scripts/windows/run-in-base.sh`](../scripts/windows/run-in-base.sh) was
  killed three minutes in, and its container ran to completion nine minutes
  later and kept writing to the redirect it had inherited. ⚠ **A second job
  launched into the same log then interleaves with the first**, and the reader
  cannot tell them apart: the conditions block of one run was read beside the
  tail of the other, and the clause list did not match the job that was asked
  for. ⭐ Give every job its OWN log path and its own `PODBOX_ARTIFACTS`
  directory, and read the artifact rather than the console. ⛔ Check
  `wsl-toolkit --instance podbox resources` for a container still `Up` before
  believing a job has ended.
- ⛔ **A new script arrives unrunnable even after `restore-modes.sh`, because
  that repair reads the git INDEX.** Measured on 2026-09-12: a new
  `experiments/` script that was written but not staged failed as
  `./experiments/153-store-lock-race.sh: Permission denied`, rc **126**, with
  the mode repair reporting 396 of 396 files fixed in the same run. ⭐ Stage it
  with its bit before the job: `git add PATH && git update-index --chmod=+x PATH`.
  ⚠ Running it as `sh PATH` hides the missing bit rather than repairing it, and
  the bit is what the tree has to carry.
- ⛔ **Two `run-in-base.sh` jobs from one checkout stage through one job
  file, so the second job drives the first job's script.** Measured on
  2026-09-22: two lane jobs launched together both stage through
  `$ROOT/.podbox-job.sh`, and both containers drove `146` while one was
  asked for `147`. Both transcripts name 146, so the evidence stayed
  honest, but the 147 re-drive never happened and was run again alone.
  ⭐ One lane job at a time per checkout. A staging path unique per
  invocation would make the race impossible; until one ships, serial
  jobs are the rule.
- ⛔ **A file named `NUL` in the checkout breaks the lane workspace copy
  and is invisible to git.** Measured on 2026-09-22: a 99-byte `./NUL`
  appeared during an engine drive, and two lane jobs exited 2 with
  `workspace refused: NUL shrank while it was being read (0 of 99
  bytes)`. Windows opens the name as the null device, so the copy reads
  0 bytes of a 99-byte header; `.gitignore` already ignores the
  reserved names, so `git status` never shows it. ⭐ Repair is
  `rm -f ./NUL` from Git Bash. What would reopen it is the creator,
  which is not isolated: the `.gitignore` comment blames a shell not
  mapping `2>/dev/null`, and no shell in the driven chain names one.

### ⛔ Decommissioning

A session leaves the machine as it found it.

| what | what to do at the end |
| --- | --- |
| the base distribution | ⭐ **keep it.** It is persistent on purpose, and rebuilding it costs about 31 s plus the rootfs pull. |
| a job's containers and directories | `wsl-toolkit --instance podbox gc --apply --older-than 24h` |
| `podman-machine-default` | stop it, if this session started it |
| anything else on the machine | ⛔ not this session's to touch |

```powershell
wsl-toolkit --instance podbox resources
wsl-toolkit --instance podbox gc --apply --older-than 24h
```

**`gc` reports by default and `--apply` acts.** It spares a container that is
running and anything belonging to an open job record, so a job still in flight
survives a collection somebody else started.

---

## ⛔ Pin it, and verify the bytes before anything executes

A wrapper that fetches code and runs it **is a supply chain**, and it gets the
same discipline as a pinned third-party action, for the same reason: a moving
reference runs code nobody reviewed.

| the rule | what it prevents |
| --- | --- |
| ⛔ **a commit or a release tag, never a branch** | a fetch that runs whatever `main` says on the day it runs |
| ⛔ **a digest, checked before execution** | a mismatch is a hard stop, the cached copy is deleted, and nothing runs |
| ⛔ **no network and no verified cache is an ERROR** | a silent skip that falls back to whatever is on disk |
| ⚠ **a release tag beats a commit** | a commit names a TREE and the file at a path in it is whatever happened to be there. A release names an artefact that was built, tested and published on purpose, and it carries its own digests. |

⚠ **Read the digest from the endpoint you will fetch from, never from a working
tree.** A repository that stores `.ps1` with CRLF in a checkout and LF in the
index gives a locally computed digest that disagrees with what the raw endpoint
serves. It fails closed, which is safe and takes an hour to work out.

### ⭐ Three tiers, and they answer three different questions

⛔ **They are not stronger and weaker versions of one check.** Each answers a
question the others cannot, which is why a caller can hold all three at once and
why dropping one is a decision rather than a simplification.

| what you have | what it proves | what it cannot say |
| --- | --- | --- |
| a sums file published with the artefact | ⚠ **transport.** The bytes that arrived are the bytes that were uploaded. | anything about who uploaded them: whoever could replace one file could replace both |
| ⭐ a digest the caller obtained separately and reviewed | these are the exact bytes somebody looked at | nothing about later releases. It pins one revision, deliberately. |
| ⭐ a signature verifiable against the publishing workflow's own identity | the artefact was produced by that workflow, which nothing outside it can imitate | that the contents are correct. A signed defect is still a defect. |

⚠ **The third tier costs a network lookup at verify time**, because a keyless
signature is checked against a public transparency log rather than a key the
caller holds. An offline verifier cannot use it. That is a real trade and it is
the reason the other two do not go away.

⚠ **Verification that is optional is verification that has to say what it did.**
A step that could not check and stayed quiet is indistinguishable from one that
checked and passed, which is the defect class this whole page is about. Expect
a tool to report all four outcomes: verified, nothing published to verify
against, no verifier installed, and failed.

### ⚠ Four traps in this shape, each paid for

- ⛔ **A copy sitting beside the launcher can win over the pin, silently.** One
  launcher resolved an explicit local path, then a sibling file, then the pinned
  ref, first hit wins. A caller passing both a commit and a digest ran the stale
  sibling and verified nothing. ⭐ Keep the fetch directory holding the launcher
  alone, or use a launcher that makes an explicit ref win.
- ⛔ **A symlink at an old path is worse than a 404.** A raw-content endpoint
  serves a symlink's own TARGET STRING with HTTP 200, so an old URL answers a
  successful-looking line of text that no interpreter can run and no digest
  check explains. A 404 is loud; that is not. ⚠ Measured on 2026-08-30 against
  a tracked symlink in a large public repository: **HTTP 200, 17 bytes, and the
  body was the relative path the link points at.**
- ⚠ **A pinned consumer does not get your fix.** Pinning protects a caller from
  a change it did not review, which is exactly why it withholds one it wanted.
  Bumping a pin is a deliberate act in the consumer's own repository.
- ⚠ **The pin's owner is not always the tool's owner.** Write down who decides
  when the pin moves, next to the pin.
- ⭐ **A digest nobody has to paste is a digest nobody pastes wrong.** The
  version of this shape that a person maintains by hand asks them to copy a
  forty-character revision and a sixty-four-character digest, and to do it again
  whenever either moves. Every one of those is a place to paste the wrong
  string, and a wrong digest fails closed in a way that takes an hour to
  diagnose. ⚠ A tool that resolves a moving reference to an immutable one
  **once** and records both in a lock file the project commits keeps the pin
  rule intact and removes the typing. ⛔ What it must never do is fetch the
  moving reference itself.

⚠ **A fetch with more than one route is not a weaker fetch.** Where a raw
endpoint, an API endpoint and a read-only proxy serve the same object, the
digest check holds whichever answered to the same bytes, so a fallback is the
same object over another road rather than a lesser copy.
[`hosted-sessions.md`](hosted-sessions.md) carries the measurement for the
proxy and the trap in its user-agent handling. ⛔ A route that fired and said
nothing is a route nobody knows fired: name the host that answered.

---

## Getting a command in

⛔ **A command sent as text is parsed twice.** The calling shell expands it
before the guest ever sees it: a dollar sign expands in transit, a backtick
opens a substitution, and Windows PowerShell 5.1 drops a double quote out of a
child process's argument list before the script runs. ⭐ Send a **file**, or
send **base64**, which has no character any shell touches.

⛔ **Do not hand a job payload to the platform command yourself.** A payload
handed to `wsl.exe` as an argument expands before the guest sees it. The guest
parses the result again. [`../scripts/windows/run-in-base.sh`](../scripts/windows/run-in-base.sh)
sends the wrapper as a file for this reason.

⚠ **Write the script with LF endings.** A byte-exact channel will not repair
anybody's payload, so a CRLF script makes a POSIX shell read the carriage
return as part of the last word on every line. A here-string written on a
Windows host is CRLF unless something says otherwise.

⚠ **Pass values as assignments, never by substituting them into the script.** A
value carrying a slash, an ampersand or a quote corrupts a substitution and
usually produces a script that still runs.

⚠ **The default shell in a slim image is not bash.** A process-substitution or
a `/dev/tcp` builtin is not there, and neither are the tools a full image has.
Name what the guest needs; do not assume it.

---

## Reaching this host from inside the guest

⛔ **In the default NAT mode the guest does not reach the host's loopback.**
`localhost` inside the guest is the guest, and the failure is silent: a fixture
bound to loopback never receives a connection and nothing on either side says
why.

| the mode | what the guest reaches |
| --- | --- |
| NAT, the default, and what this was written on | the host's address on the virtual adapter. Ask the tool; it changes. `wsl-toolkit hostaddress` prints it. |
| mirrored | the host's own loopback, so a caller's branch for this disappears |
| bridged | the guest is on the LAN, and which host address it reaches is a choice rather than a lookup |

⛔ **Read the address at run time, never record it.** It is assigned and it
changes. A number written into a document is a number that will be wrong.

⚠ **Do not forward a port to work around this.** On Windows that needs an
elevated session and leaves a rule behind after the tool exits. Bind the host
service to the address the guest can reach instead. The unspecified address is
not the answer, because it accepts the LAN as well.

---

## ⛔ Bound anything that can wait forever, and make silence visible

A guest command that hangs looks exactly like a guest command that is working,
and a line-oriented reader shows nothing at all while a large download is
visibly progressing, because the downloader redraws one line and emits no
newline for minutes.

⭐ **Every row below is a hazard, not an assignment.** The tool ships each
shape with its own flags. Check the manual before you build one. A caller who
builds them again owns a second copy of a solved problem.

| the shape | why it is needed |
| --- | --- |
| ⭐ **a heartbeat on SILENCE, not on a timer** | a chatty command produces none, and a quiet one says how long it has been quiet and whether the guest is still alive. "The command is quiet" and "the machine is gone" are the two states you could not otherwise tell apart. |
| **a carriage return terminates a line** | otherwise a progress counter is invisible for the whole download |
| **an unterminated line shown after a short wait** | a prompt waiting on input looks exactly like work, forever |
| **a timeout on the command, with its own exit code** | otherwise a person eventually kills it and reads raw logs to find out what happened |
| ⚠ **an off switch** | relaying makes the guest's output a pipe, so an application that block-buffers off a terminal will buffer. A caller parsing bytes needs the unrelayed form. |

⚠ **Nothing is injected into the guest for this.** Every figure is one the host
already holds, plus a read-only query about the machine's state. A guest with
no userspace to speak of still needs to be watchable.

⛔ **A guest command that can hang carries its own timeout too.** A build that
runs for an hour is legitimate, so the tool cannot bound the command by
default; the script inside can.

---

## Decommissioning, which is not optional

Every machine is removed in the run that made it, and the run **reads the state
back** rather than assuming.

⛔ **A destructive step that prints success without checking is the row
[`conventions/forbidden-patterns.md`](conventions/forbidden-patterns.md)
carries about a delete that reported success.** A file something holds open
reads as a file that has gone.

⚠ **A cancelled run leaves things behind**, because cleanup is a `finally` and
a hard interrupt does not run one. Expect a registered machine and a rootfs
image of several hundred megabytes. List before you start and list after you
finish; a purge is what removes them.

⚠ **A listing shows a machine that is running right now the same way it shows
an orphan.** Read the timestamp before purging.

⛔ **Never take the whole subsystem down to clean up after one guest.** On
Windows the tempting command is machine-wide: it stops every distro including
the one the container engine runs in, which takes the engine down with it.

---

## ⚠ Two traps that belong to the kernel, not the tool

⛔ **A guest's lifetime is not the kernel's lifetime.** Terminating or
unregistering a distro restarts or removes its userspace while the shared
kernel keeps running. Binary-format registrations, loaded modules and pinned
superblocks survive, so a throwaway machine used to reproduce a kernel-level
condition can read state hours old and answer confidently.

⛔ **A foreign-architecture rootfs may RUN rather than fail**, which is the
worse outcome. That shared kernel carries whatever emulator handlers anything
on the machine registered, and the flag that keeps an interpreter open means a
rootfs for another architecture boots and answers that architecture's name.
⭐ Name the platform on every pull and every create: the local image store is
keyed by tag and not by architecture, so one pull for another architecture
repoints the shared tag and every later unqualified pull hands back the wrong
image. That is the row
[`conventions/forbidden-patterns.md`](conventions/forbidden-patterns.md)
carries about a cache keyed without the variant.

---

## Images cost more disk than they look like they do

⚠ **The cost is dominated by a fixed floor rather than by a multiple of the
input**, so a small rootfs does not produce a small disk. Measure free space
before importing and refuse rather than leaving a half-written disk and a
registered machine that does not work.

⚠ **This repository states no number here**, because the floor is the host's
and this page cannot see the host.
[`methodology/experiments.md`](methodology/experiments.md) is how to take the
measurement and what it owes: the machine, the day, the versions.

---

## Leaving the engine as it was found

A session that pulls an image or creates a volume removes it. The engine is
shared with everything else on the machine and a session's leftovers are
somebody else's disk.

⚠ **Reclaimable space at a large fraction of total size means something stopped
cleaning up after itself.** A named volume with no container attached is not
necessarily garbage, which is why the honest output is a count and a size
rather than a recommendation. ⛔ Reclaiming somebody's disk is their decision.
