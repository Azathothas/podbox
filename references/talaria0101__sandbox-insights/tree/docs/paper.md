# The Shape of a Restricted Runtime

**Mechanism attribution for a Linux sandbox that looks maximally
privileged and is pervasively denied — four walls, what still runs,
and design lessons for tools that must live there**

---

## Abstract

A class of Linux environments — agent sandboxes, hardened CI
executors, locked-down build farms — presents a process with uid 0 and
a full capability set while refusing most of what the container,
virtualization and debugging stacks are built on. Failures there
mislead, because the errno or error string surfaces layers above the
kernel decision and several mechanisms produce overlapping symptoms.
We characterize one production instance of this class with a
composable instrument suite and reduce its behaviour to four
mechanisms with distinct signatures: a user namespace with a partial
ID map (**N**), a syscall-number denylist seccomp filter (**F**), a
path-scoped write policy (**M**), and an address-aware network policy
(**P**). One technique attributes any denial between them at a cost of
one syscall: call the suspect syscall with an argument the kernel
would reject inside its own body — a path- or id-shaped errno means it
executed, `EPERM` means a filter refused it before entry. We document
the four walls where outside tooling stops (unmapped-ID ownership,
Go's ambient `setgroups`, interposition reach, and the
namespace-without-mounts split), each with a runnable reproduction;
two filter gaps in the denylist, one of which grants real in-process
namespace privileges while the adjacent gap was closed live under
observation; and the capability inventory that survives: qemu TCG
booting a pinned kernel to userspace in seconds with driven exec and
checksum-validated in-guest compute, chroot appliances at native
speed, and a complete ptrace-free tracing tier built on seccomp user
notification — all six of whose primitives we prove live on a host
where the ptrace syscall class is denied outright. We close with
design rules for tools in this class, of which the first is that
silent degradation is not a weaker behaviour but a false one. Every
claim is produced by a numbered, committed script; the repository is
the artefact.

---

## 1. Introduction

Run a container tool inside this class of sandbox and it fails. Run a
microVM manager and it fails. Run a debugger and it fails. The
interesting question is not *that* they fail but *which kernel
mechanism refused them*, because the failures present errnos that
point elsewhere — an `EINVAL` that is a mapping check, an `operation
not permitted` produced by three different refusals, a `no such file
or directory` from a process that can see the file — and because
fixes derived from the wrong mechanism cannot work.

The environment's defining property is a contradiction in what the
process can see about itself. It is uid 0 with every capability bit
set; nothing in `id` or `/proc/self/status` suggests a restriction.
At the same time most of what container tooling does — mount, mknod,
unshare, ptrace — answers `EPERM`, and the write surface is a
four-directory allowlist. Diagnosis is treacherous in proportion to
how many mechanisms produce the same symptom.

This work makes three contributions:

1. **An attribution method** (§4) and its instrument library: the
   bogus-argument discriminator, the probes that measure nothing, and
   a four-mechanism model whose members can be composed and removed
   independently.
2. **The four walls** (§5): the recurring failure points of outside
   tooling on this class, each with a committed reproduction, and the
   fixes that actually clear them.
3. **What still runs** (§6): a measured inventory of the capability
   tier that survives — software-emulation microVMs with driven exec,
   chroot appliances, in-process namespace privileges through a filter
   gap, and a full ptrace-free tracing tier — with the design rules
   (§7) that fall out of how these tools fail.

The setting for all measurements is one production instance of the
class (kernel 6.18.39, x86-64, AMD Ryzen 7 7700, 16 hardware threads,
30 GiB RAM), re-measured continuously; where a property of the
instance changed during the work, both states are committed. The
repository is the artefact: every section cites the experiment that
produced its numbers.

## 2. The environment, measured

Table 1 is the contract. "Measured" means produced by
`experiments/10-` (mechanism census), `15-` (network), `20-`
(attribution), `40-` (namespace gap), `45-` (tracing primitives) with
committed logs.

**Table 1 — the runtime contract.**

| facility | state | mechanism |
|---|---|---|
| identity | uid 0, full capability mask `000001ffffffffff` | root in a userns mapping one id (`0 0 1` at measurement; an earlier revision mapped `0 → 1000`) |
| supplementary groups | `groups=0,65534` | 65534 = overflow gid: an unmapped group is the tell |
| `chown`/`lchown`/`setuid` outside the map | `EINVAL` | N — a mapping check, not policy |
| `setgroups(0)` | `EPERM` | N — `setgroups: deny` is part of the userns contract |
| `mount`, `umount2`, `pivot_root`, `fsopen`, `open_tree`, `move_mount`, `unshare`, `setns`, `ptrace`, `bpf`, `keyctl`, `open_by_handle_at`, `process_vm_readv/writev` | `EPERM` pre-execution | F |
| `clone`/`clone3` with namespace flags | **execute** | F's gap (§6.3) |
| writes | allowlist `{/tmp, /dev/shm, /workspace, /state}` only | M — path-scoped; a filter cannot dereference paths, so this is a distinct mechanism by construction |
| `/proc/self/mem` | readable, not writable | M/provenance |
| `uid_map`/`gid_map` writes | `EPERM` for everyone, parent and child alike | `map_write()` checks `file_ns_capable()` against the procfs **mount's** userns |
| network | TCP bind denied everywhere; UDP and AF_UNIX bind allowed; loopback TCP connect allowed at measurement (was denied in an earlier revision); external egress allowed | P — address/port/family-aware `EPERM` = cgroup-BPF `SOCK_ADDR` class, reconfigurable live |
| `/dev/kvm`, `/dev/net/tun` | nodes absent; host drivers present (`/proc/misc`) | node exposure, not hardware; every tenant-side path to the node measured closed |
| `RLIMIT_FSIZE` | 500 MiB/file | guest images above it die `SIGXFSZ` |
| `/tmp` | 64 MiB tmpfs | a full `/tmp` makes writers lose output **silently** |
| available | RWX `mmap`/`mprotect`; `memfd_create`; Landlock ABI; `openat2`; `io_uring`; `chroot`; outbound TCP/UDP/DNS; seccomp **user-notification** | the capability tier of §6 |

Three structural facts drive everything downstream. First, the
capability mask is what a process gets as root in a user namespace —
by construction — and says nothing about which namespace each
capability is scoped to; most capability checks here fail not for
lack of the bit but because the object's owner is unmapped or the
object's namespace is not ours. Second, the filter is a denylist of
syscall **numbers**, so it blocks the legacy door to an operation and
admits the modern door unless separately named — the source of both
the namespace gap and its live patching (§6.3). Third, `/proc` is a
mount from the initial user namespace, which no tenant capability
applies to; that single provenance fact is what keeps namespace
privileges in-process and uid maps unwritable.

## 3. Method

Three rules, enforced by the instruments themselves.

**An experiment is a file.** Every measurement lives in a numbered,
runnable script (`experiments/NN-*.sh`) with pinned inputs (URL +
SHA-256), a conditions block (host, kernel, date, versions, identity)
on every log, and exit codes that distinguish *ran and held* (0),
*ran and the thing under test failed or a committed negative result
was taken* (1), and *could not run here* (2). Negative results are
committed with the same rigour as positive ones.

**A probe reports the operation's verdict.** Each probe runs in a
disposable child; the child prints its own verdict and errno (a
parent reporting "the child's" errno reports its own, which is always
0 — our first census draft had exactly that bug, and the class it
represents — a harness measuring itself — recurs wherever verdicts
are taken from exit codes, leftover fixtures, or shared temp paths).
"Could not run" is structurally distinct from "denied": a missing
fixture must never read as a policy.

**The instrument must be able to say it is broken.** Attribution
probes carry controls that must answer with argument-shaped errnos
(`pidfd_getfd(-1,-1)` → `EBADF`, `kcmp(-1,-1,…)`, `ESRCH`); if a
control answers `EPERM`, the prober's world model is wrong, not the
host's.

Claims are tagged **[V]** (verified in this tree, runnable), **[S]**
(established from pinned source read), and **[C]** (cross-checked
against another instance of the class). A number without its
conditions is a rumour.

## 4. Attribution

### 4.1 The four mechanisms

| | mechanism | contents | signature |
|---|---|---|---|
| **N** | user namespace | one-entry ID map; `setgroups` deny; an unmapped supplementary group; **a mount namespace owned by the parent userns** | `EINVAL` from credential ops on unmapped ids; `EACCES` on unmapped owners; `EPERM` where the check is initial-ns-scoped (`mknod` of real device numbers) |
| **F** | seccomp filter | the denylist of Table 1 | `EPERM` **pre-execution**, discriminated by §4.2 |
| **M** | path-scoped write policy | the four-directory allowlist; `/proc/self/mem` write; procfs provenance | denials a filter cannot produce (it cannot read pathnames); Landlock ABI present; content is operator policy, reconfigurable |
| **P** | network policy | TCP bind; (earlier revisions: loopback connect, :80 egress) | address/port/family-aware `EPERM`; live reconfiguration |

Two clarifications the model depends on. `clone(CLONE_NEWNS)`
succeeds while `unshare(CLONE_NEWNS)` is refused: **the mount
namespace is obtainable and the mounts are not** — any account that
explains mount failures by namespace ownership predicts the opposite
of what happens. And M is not a variant of N: N explains unwritable
directories whose owner is unmapped, but on this instance only some
writable-owner directories accept writes, which no capability- or
mapping-based mechanism explains — the split is path policy.

### 4.2 The bogus-argument discriminator

A seccomp filter evaluates the syscall number and six argument
registers. It **cannot dereference a pointer**, and it runs **before
the syscall body**. Call the suspect syscall with an argument the
kernel would reject inside its own body:

- a path- or id-shaped errno (`ENOENT`, `EBADF`, `ESRCH`, `EINVAL`,
  `EFAULT`) → the syscall executed; any denial is kernel-internal;
- `EPERM` for the same argument → refused before entry: the filter
  named the syscall.

One syscall per question, no privilege, no access to the filter
needed. On this instance it settles the whole mount family —
including the new-API `fsopen`/`open_tree`/`move_mount` — plus
`unshare`, `setns`, `ptrace`, and `process_vm_readv` as filtered
pre-entry, with `clone3`, `openat2`, `io_uring`, `kcmp` and
`pidfd_getfd` executing (experiment 20-). The controls matter as much
as the probes: a discriminator that can no longer see the difference
between the two errno classes must say so.

The same asymmetry, read from the other side, explains why certain
*messages* discriminate: bubblewrap's `Failed to make / slave:
Operation not permitted` proves its unconditional `clone(CLONE_NEWNS)`
succeeded (the mount is only reachable after), while a refused clone
prints `Creating new namespace failed` — two different walls, two
different strings, reproducible by composing the mechanisms
independently [V].

### 4.3 Probes that measure nothing

- `mknod(S_IFCHR, 0)` is a **whiteout**; the kernel exempts it from
  the capability check. It succeeds where a real device number fails,
  and that pair — same path, same directory — proves the denial is
  capability-based, because no path policy distinguishes device
  numbers. A device-node probe that uses dev 0 has measured its own
  assumption.
- `unshare(CLONE_NEWUSER)` from Go returns `EINVAL` **unconditionally**
  — the kernel refuses it for multithreaded callers and the Go runtime
  is always multithreaded. A Go verdict on that flag is an artefact of
  the prober's language. Use a single-threaded helper or `clone`.
- Any verdict taken from a subject's exit code alone: real tools exist
  whose `doctor` exits 0 on failure. Grep the log for the decisive
  line.
- An errno printed by a parent for a child's syscall (always 0), or a
  missing fixture read as a denial.

### 4.4 Attribution by removal

When one probe cannot separate two mechanisms, compose the mechanisms
independently — a userns with the target's map; a filter with
per-syscall toggles; a Landlock ruleset scoped to the allowlist — and
remove one denial at a time. A denial observed under two mechanisms
tells you nothing; a denial that survives removing one of them is
attributed. On a host you control this is minutes of work, and it is
the difference between "fails under the sandbox" and "fails because
of rule X".

## 5. The four walls

Every tool we ran on this class stops at one of four places. They have
different fixes, and three of the four present messages that mislead.

### 5.1 Ownership: `chown` to an unmapped ID

The single most consequential wall. In a user namespace an ID with no
mapping translates to INVALID_UID/GID, and the kernel reports that as
`EINVAL` — where a permission problem is expected and privilege is
the attempted remedy. No privilege clears it; it is a mapping
failure:

```text
tar: ./etc/shadow: Cannot change ownership to uid 0, gid 42: Invalid argument
```

reproduced against the real pinned Alpine minirootfs and with a
synthetic archive (experiment 25-) [V]. The wall's tenants are every
extractor that restores ownership by default: GNU tar as root (the
unpacker behind several container tools), Go unpackers issuing
`lchown` as direct syscalls, containers/storage-class layer appliers,
and package managers that chown a download directory to a dedicated
user (pacman's `DownloadUser` directive; the first casualty is
usually `/etc/shadow`, gid 42) [V for tar and the synthetic case;
[S] for the Go and pacman code paths].

The fix — extract ownership-neutrally — is real but not free: the
resulting tree's ownership differs from the image's, which changes
the behaviour of anything that checks. The better form records the
intended metadata in a sidecar keyed by path and applies it only
where the target ID is mapped: faithful re-export, honest answers,
and a diagnostic that names the gap, without pretending the kernel
checks were satisfied.

### 5.2 Credentials: Go's ambient `setgroups`

`os/exec` issues `setgroups(2)` in the child whenever
`SysProcAttr.Credential` is non-nil, unless
`Credential.NoSetGroups` is set or a `GidMappings` guard applies
(`syscall/exec_linux.go`, Go 1.24 [S]). Under N, `setgroups` is
denied, so the spawn fails — and here is the expensive part: a spawn
setting namespace clone flags **and** a `Credential` fails with the
identical string whether the refused call was the clone or the
setgroups, because the clone runs first, the setgroups runs in the
child before `execve`, and either failure surfaces as
`fork/exec ...: operation not permitted` [V, experiment 30- — the
ten-row matrix, including the combined row that demonstrates the
ambiguity]. Entire wrong architectures ("namespaces are denied, give
up on isolation") have been built on that message.

The fix is one field: `Credential.NoSetGroups = true` — a no-op on
unrestricted hosts. Note also the guard's trap:
`GidMappingsEnableSetgroups: true` without any `GidMappings` does
**not** take the guard's exit (the first conjunct is false) — a shape
that shipped in a real container tool [S].

### 5.3 Interposition reach: coverage is a property of the payload

One shim, three payload classes, two syscalls each (experiment 35-)
[V]:

| payload | seen by an `LD_PRELOAD` shim |
|---|---|
| dynamically linked C | yes |
| statically linked | no — no loader, no preload at all |
| Go | no — raw syscalls, with or without cgo |

Classification (`PT_INTERP`, Go build markers) is therefore mandatory
before promising coverage, and it stays advisory: no ELF property
proves raw-syscall freedom. In the same run, the intercepted
`lchown(path, 0, 42)` still answers `EINVAL`: **seeing a call is not
clearing it**. Path virtualization and ownership virtualization are
two jobs the word "interposition" hides; only the second clears
§5.1, and no interposer in any language reaches Go's version of it.
A per-process bind *view* without any mount privilege is genuinely
available to this tier for dynamic C — and is invisible to every
payload the tier does not reach, which is why a tool offering it must
say which payloads got it.

### 5.4 Mounts: available namespace, unusable mount

`clone(CLONE_NEWNS)` succeeds; `mount(2)` is refused, in the fresh
namespace too [V]. Bubblewrap's `MS_SLAVE` propagation, overlay
drivers, `pivot_root`, FUSE (which additionally needs a `/dev/fuse`
node this runtime cannot create) — all unsatisfiable regardless of
namespace acrobatics. The wall's extent is finer than "mounts fail"
— the new mount API can split creation from attachment, and which
halves are open is revisable between revisions of the same sandbox —
but the standing rule is not: **probe creation and attachment
separately**, because a runtime that asks only the old-API question
learns one syscall's worth less than the new-API question would have
told it.

The measured replacements: copy-in seeding (a chroot appliance
receives files by `cp`, not bind), ownership-neutral extraction for
anything that would have been a layer apply, and byte-written device
nodes for guest images (§6.1).

## 6. What still runs

### 6.1 Software-emulation microVMs

KVM is unreachable (Table 1; every tenant-side path to the node
measured closed), and software emulation is not. On this instance
[all V, experiments 50-/55-/60-]:

- a pinned Alpine `linux-virt` 6.12.94 kernel **boots to guest
  userspace in 2.0–5.6 s** under `qemu -accel tcg,thread=multi -M
  pc,acpi=off` (range across runs, load-dependent; conditions in the
  log);
- the guest is **driven**: FIFO-pair serial transport (both ends held
  `O_RDWR` before qemu starts — the `pipe:` backend opens *existing*
  fifos), readiness by nonce echo, marker-bracketed single-shot execs
  with **line-anchored** markers (unanchored matches hit the echoed
  typed command — this cost one debugging cycle) and exit-code
  propagation;
- **in-guest compute is cross-validated by checksum**: the same
  `bench.c` runs on host and guest, and both its integer bench and an
  FP-heavy variant print identical checksums on both sides before any
  speed is compared. Measured TCG tax: **1.0–3.2×** for the
  integer-dominated bench across runs — tight register-only loops can
  run near native under TCG's code cache — and **6.9–12×** for a
  dependent double chain, consistently. The tax is *workload-shaped*
  and *run-shaped*, and any single-number TCG multiplier is a claim
  about a benchmark and a moment, not about emulation;
- initramfs **device nodes are written byte-wise** into the newc cpio
  (`mkrootfs.py`): the host cannot `mknod`, but the guest kernel's
  own unpacker can, and without a `/dev/console` node PID 1 is
  silent;
- the driver owns the guest lifecycle: on `pc,acpi=off` the guest's
  poweroff **halts the vCPU instead of exiting qemu**, so completion
  is a serial-log marker followed by a kill, never an exit wait;
- guest image sizes respect `RLIMIT_FSIZE` (500 MiB/file); guest
  egress runs over slirp; TCP host-forwards are impossible (mechanism
  P), so guest↔guest and host↔guest data move over UDP forwards or
  the serial channel.

### 6.2 chroot appliances: native speed, zero boundary

`chroot(2)` is unfiltered and needs only `CAP_SYS_CHROOT` in the
tenant's own userns. A pinned Alpine minirootfs, extracted
ownership-neutrally, entered by plain chroot, runs the same benchmark
at **native speed with an identical checksum** — 817.1 host vs
880.0 chroot Mops/s in the committed pairing (experiment 60-) [V]. The appliance
ships its own `/etc/passwd`, which the sandbox lacks; `ps` inside
shows an empty table (no `/proc`; `mount` is filtered); `/dev/null`
may not exist as a device (no mknod, no devtmpfs); and **there is no
kernel boundary** — the workload shares the tenant's kernel, PID
space and namespaces, and the classic double-chroot escape
*precondition* holds for uid 0 (the run proves the precondition; it
deliberately does not demonstrate an escape).

### 6.3 Filter gaps: real privileges, live lifecycle

The denylist blocks the legacy door and leaves the modern door open
where it was never named. Measured on this instance:

- **`clone3` with namespace flags executes** (`clone3(NULL)` answers
  `EINVAL` — the kernel body rejected the argument). A process
  entering `CLONE_NEWUSER|CLONE_NEWNET` **owns** those namespaces,
  and owner-namespace capability checks pass for it, in-process:
  raw-ICMP sockets, raw-netlink veth pairs, private `CLONE_NEWPID`
  process trees (the child reports PID 1) — all in the child, where
  the parent's identical operations answer `EPERM` (experiment 40-)
  [V].
- **The boundary is measured, not assumed**: `uid_map` stays empty
  (the procfs-provenance rule of Table 1), namespace children run
  with an unmapped uid, and `execve` **drops** the capability set
  granted at namespace creation. The gap buys in-process privileges —
  private fabrics, private PID trees — not a container runtime.
- **The lifecycle is live.** An adjacent gap — the new mount API —
  was open in an earlier revision of this same instance and is closed
  in the current one, under continuous re-measurement; the
  clone-family gap outlived the same patch cycle. Both states are
  committed. Consequence for tool design: anything built on a gap
  **probes the gap every run**, cheaply, with the §4.2 discriminator,
  and treats closure as a committed negative result — not a failure
  of the tool.

### 6.4 A tracing tier without ptrace

The ptrace syscall class is denied pre-entry here — vanilla strace
answers `PTRACE_TRACEME: Operation not permitted` and dies [V] — and
UML-class guests, which intercept syscalls through ptrace, die at
their own self-checks for the same reason. The tracee can, however,
**seccomp itself**. All six primitives of a tracer built on seccomp
user notification are proven live on this host (experiment 45-)
[V]:

1. self-install of `SECCOMP_FILTER_FLAG_NEW_LISTENER`;
2. listener handoff to the tracer over a unix socket (`SCM_RIGHTS`);
3. `NOTIF_RECV`/`NOTIF_SEND` with `USER_NOTIF_FLAG_CONTINUE`
   shepherding every syscall, unprivileged;
4. forged return values — the logged tracee observed
   `getuid() == 4242`;
5. `SECCOMP_IOCTL_NOTIF_ADDFD` installing a *real* fd into the
   tracee — the logged tracee read it;
6. `/proc/<pid>/mem` serving memory arguments while the tracee is
   blocked in the notification — stable by construction (the tracee
   cannot run), remote pointer = file offset, no TOCTOU window.

Four implementation rules each cost a debugging cycle and are now
part of the instrument: **the handoff path must be exempt from the
tracee's own filter** (a filter that notifies on `sendmsg` deadlocks
the handoff — the message never leaves a tracee blocked in the
notification queue); **`TSYNC` and `NEW_LISTENER` are mutually
exclusive** (EINVAL — a listener belongs to one thread's filter); **a
support probe requires proof of liveness** (the handoff *and* an
actual notification within a timeout — a filter that falls through to
`RET ALLOW` on an architecture mismatch silently traces nothing); and
**the driver drains until the tracee exits** (one unanswered
notification blocks the tracee in that syscall forever).

The tier's boundary is probed and committed: an outer `ERRNO` filter
outranks `USER_NOTIF` in kernel action precedence, so syscalls the
sandbox denies — the ptrace class — **never notify and can never be
mediated**. A tracer in this tier must say which syscalls it covers,
not assume its listener implies coverage.

## 7. Design lessons

1. **Report the mode achieved; never let a weaker mode satisfy a
   stronger request.** The honest ladder here: machine (TCG) →
   appliance (chroot) → supervise (seccomp-notif) → interpose. Each
   mode's ceiling is measured (§6); a tool that accepts a request for
   isolation and delivers an appliance is not degrading, it is lying.
   And the TCG boundary must be described for what it is — the
   emulator process, not hardware.
2. **Degradation must be chosen and loud.** Two tools can implement
   the same tier with opposite fallbacks: one refuses the mode and
   names the reason; the other continues the syscall and reports
   success for mediation it never performed. The second is not weaker,
   it is false. `supervise` is the tier where silent failure is
   structural (its argument channel can vanish while its listener
   works), so its three legs — listener, ADDFD, argument read — must
   be probed together, and the tier refused if any is missing.
3. **Probe the contract at runtime.** The filter and the network
   policy change on a day scale (§6.3). Anything built on a gap probes
   it every run; every document that states a gap states its date.
4. **Classify payloads before promising anything.** Coverage is a
   property of the payload (§5.3); the honest default when unsure is
   "not reached", stated.
5. **Diagnostics name operation, errno, mechanism, remedy.** In this
   environment the raw errno misinforms most of the time (`EINVAL`
   from a mapping check; one string from three refusals; `-20` from
   libarchive is a status code, not an errno [S]). A one-line mode
   banner — mode, why, what is not provided — plus failures of the
   shape *operation (args): errno / mechanism, with the file that
   proves it / remedy* prevents the misdiagnosis class this paper
   documents.
6. **Ship the census.** A tool for this class should carry the
   probe discipline of §3 as its own preflight — disposable children,
   operation-named verdicts, preserved errno semantics, discriminators
   with controls. It converts "doesn't work here" bug reports into
   attribution rows.
7. **Complete the environment loudly.** Missing `/etc/passwd`,
   missing `/dev/null`, byte-written device nodes, static `/proc`
   fixtures — each has a measured recipe here, and each should log
   what it served. A silent completion is indistinguishable from a
   false claim about the environment.

## 8. Limitations

Stated first, because a reader skimming for conclusions will not
reach an appendix.

- **One instance, continuously re-measured.** The numbers are this
  machine's; the contract moves (two live changes observed during
  this work alone). The instruments are the durable part; the tables
  are dated.
- **The filter is inferred, never dumped.** Only probed entries are
  known; a rule nothing exercises is invisible. Every "unfiltered"
  verdict means "unfiltered among the probes run".
- **The TCG multipliers are benchmark-shaped** (3.2× integer, 12×
  dependent-FP here). They bound nothing about KVM, which this host
  cannot measure; no KVM baseline exists inside the sandbox.
- **The chroot escape is a precondition claim.** The logged run
  proves a second chroot succeeds for uid 0; it does not demonstrate
  a full escape, which needs a staged dirfd strategy beyond this
  paper's scope.
- **How many claims a previous revision got wrong is the only honest
  estimate of how many are still wrong.** This repository's own
  first-draft census shipped a parent-reported errno (always 0), a
  whiteout fixture that poisoned its next run, and a marker parser
  that matched echoed commands; all were found by running the
  instruments and are documented in the review passes. Assume more
  remain.

## 9. The artefact

The repository is the paper's evidence and its reusable part:

```text
docs/          environment.md (the contract) · attribution.md (the
               method) · walls.md (the four walls) · recipes.md (what
               runs, exactly) · design-lessons.md · reviews/
scripts/       census.c · attribute.c · netpolicy.py · nspriv.c ·
               senotif_probe.c · spawn_matrix.go · ldpreload/ ·
               libfakepasswd.c · bench.c, bench_fp.c · mkrootfs.py ·
               lib.sh (pins + conditions)
experiments/   numbered scripts + committed logs (the evidence)
```

Every instrument is standalone, self-asserting, and prints its
conditions. The census (10-) is the first thing any session here
runs, because the host is re-audited by its operator continuously and
yesterday's contract is today's hypothesis.

## References

1. Linux kernel source: `fs/namespace.c` (`may_mount()`), `fs/fsopen.c`
   (new mount API), `fs/proc/array.c` and user_namespace map writes
   (`map_write()`, `file_ns_capable()`), `security/landlock/`
   (ruleset ABI), `kernel/seccomp.c` (user-notification, action
   precedence), `mm/shmem` whiteout handling for `mknod(0:0)`.
2. Go standard library: `syscall/exec_linux.go` — the `setgroups`
   guard and `Credential.NoSetGroups`.
3. seccomp user notification: `Documentation/userspace-api/seccomp_filter.rst`
   (`SECCOMP_IOCTL_NOTIF_RECV/SEND`, `SECCOMP_IOCTL_NOTIF_ADDFD`,
   `SECCOMP_USER_NOTIF_FLAG_CONTINUE`).
4. QEMU — https://www.qemu.org (TCG; `-M pc`, `-serial pipe:`).
5. Alpine Linux — https://alpinelinux.org (netboot `linux-virt`
   kernel, `busybox-static`, minirootfs; `apk-tools`).
6. strace — https://strace.io (ptrace-based tracing; the ptrace
   denial signature on this class is its own startup diagnostic).
7. GNU tar — https://www.gnu.org/software/tar/ (`--no-same-owner`,
   `--no-same-permissions`).
8. bubblewrap — https://github.com/containers/bubblewrap
   (unconditional `clone(CLONE_NEWNS)`; the discriminating mount
   message).
9. Podman / containers/storage — https://github.com/containers (vfs
   graph driver; layer application).
10. pacman — https://gitlab.archlinux.org/pacman/pacman
    (`DownloadUser`, `CheckSpace`).
11. libarchive — https://www.libarchive.org (`ARCHIVE_WARN` and the
    status-code-vs-errno trap).
12. Firecracker, libkrun, Kata Containers, Lima — the KVM-dependent
    tier whose dependency chain this environment forecloses
    (`/dev/kvm` node exposure).
13. User-mode Linux — https://user-mode-linux.sourceforge.net
    (ptrace-intercepting guests).
14. udocker, fakechroot, PRoot, fakeroot, ruri — the
    interposition/chroot lineage whose reach this paper's walls bound.
15. Landlock — https://www.landlock.io (the ABI whose presence makes
    mechanism M self-applicable by a tenant).
