# Reference sweep, 2026-09-11

⭐ **Eleven repositories, read under
[`../methodology/references.md`](../methodology/references.md).** This page is
the findings file. The usable output is the entries in
[`../../TODO/`](../../TODO/), because an entry names what to do and which
reference to open at which line. The corpus is
[`../../references/`](../../references/) and it is tracked.

---

## ⛔ What this sweep did NOT establish

Read this first. A reader who reaches the verdicts first has stopped reading.

| this was not established | why it matters |
| --- | --- |
| **Nothing here was measured on podbox's target.** Every number below was taken on somebody else's host: Linux 6.18.39, AMD Ryzen 7 7700, 16 threads, 30 GiB. ⛔ Reproduce the mechanism and expect the number to differ | a number without its conditions is a rumour, and these conditions are not ours |
| **No tool in the survey was run.** The six VM tools were read, not executed. Their verdicts are `[S]`, established from source and tracker, never `[V]` | a README is evidence of intent, not of behaviour |
| **The four research trees describe three DIFFERENT instances** of the target class, not one. Their identity maps, their filter contents and their network rules disagree with each other | a contract read from one of them and hard-coded here would be wrong on the other two |
| **No licence was checked by a lawyer.** Each determination is the repository's own statement, read in its tree | the determination is what the tree says, not advice |
| **The trackers of the four research trees are empty**, so no maintainer ruling was available for any of them | the richest source the procedure names produced nothing for four of eleven |
| **`vml`'s vendored tree was not read.** 19,402 files, almost all of them vendored crates | the row below rests on its README, its `src/`, and a four-item tracker |

⚠ **Conditions.** One session, 2026-09-11, on a Windows host reading a
tracked corpus. The `gh` route answered for all eleven fetches and every
`PROVENANCE.md` records **0 gaps**, discussions included.

## ⛔ How many claims the previous revision got wrong

⭐ **It is the only honest estimate of how many are still wrong.** The reading
this sweep replaces made **eight** claims that its own sources contradict. Six
were corrected here and two were confirmed on a second look.

| # | the previous claim | what the source says |
| --- | --- | --- |
| 1 | `Azathothas/sandbox-insights` is unlicensed, ⛔ do not vendor | its `tree/LICENSE` carries the full Zero-Clause BSD text. ⭐ **0BSD and vendorable.** The badge reads `NOASSERTION` because the heading is not the canonical string |
| 2 | `qemu-rs` is MIT and vendorable | `tree/Cargo.toml:7` declares **`GPL-2.0-or-later`**, inherited by both member crates. ⛔ Refused |
| 3 | `cubic` is Apache-2.0 | `tree/Cargo.toml` declares **`MIT OR Apache-2.0`** and both files are present. More permissive, not less |
| 4 | the five VM tools are "microVM managers" | ⛔ **None of them is a microVM manager for a capability-denied host.** Three require hardware acceleration, one is a configuration manager, one is an emulator engine |
| 5 | the nix experiment filled `/dev/urandom` with 1 MiB from `getrandom(2)` | its chroot builder writes **4 KiB copied from the host's `/dev/urandom`**. podbox's own 1 MiB from `getrandom(2)` is a different and better choice, and the two must not be described as the same fix |
| 6 | the four references were "studied" | five markdown files were opened once. Zero scripts and zero logs were opened, and the evidence is in the scripts and the logs |
| 7 | `REPORT.md` over-claims a nix 2.3.18 result its log does not carry | ⭐ **confirmed on a second look, and the previous doubt was wrong.** `logs/04c` says "skipped"; `logs/06-pty-shim-poc.log` carries it |
| 8 | the TCG tax is a single multiplier | confirmed as a defect in the sources themselves. See finding F1 |

## Route the reader

| a reader with | reads |
| --- | --- |
| two minutes | the verdict table |
| ten minutes | findings F1 to F6 |
| the implementation to do | the entry named in each finding, in `TODO/` |
| a reason to distrust this page | the provenance table, then the logs it names |

---

## Provenance, and the depth reached

⛔ **Depth is stated per reference.** A reference read at one depth cannot
support a claim that needs another.

| repository | commit | files | depth reached |
| --- | --- | --- | --- |
| `talaria0101/sandbox-insights` | `0889f5f` | 54 | ⭐ all four passes. Every document, every experiment script, every log |
| `talaria0101/vm-research` | `7697b9b` | 98 | passes 1 to 3. All six documents, the crossvalidation and serial-protocol scripts, ten logs. ⚠ The 38 subject scripts were read by their README rows, not opened one by one |
| `talaria0101/nix-experiment` | `d8835f2` | 51 | ⭐ all four passes. Report, shim design, three reviews, the chroot builder, the probe output, the decisive logs |
| `Azathothas/sandbox-insights` | `bcf415c` | 34 | passes 1, 2 and 4. Every prose document. ⚠ The paper, the four PowerShell experiments and the evidence ledger were not opened |
| `Azathothas/memfd-ng` | `5da5803` | — | pass 1 only. Metadata and licence. ⚠ Its source is unread; T-0909 is where that is owed |
| `hust-open-atom-club/Vex` | — | 89 | passes 1 and 3. README and tracker, 13 issues and 21 pull requests |
| `cubic-vm/cubic` | — | 211 | passes 1 and 3. README and tracker, 87 issues and 477 pull requests |
| `Obirvalger/vml` | — | 19,402 | pass 1 only. README and tracker. ⚠ Its `vendor/` is untouched |
| `gevico/tcg-rs` | — | 140 | passes 1 and 4. README and crate table. Empty tracker |
| `qemu-rs/qemu-rs` | — | 63 | passes 1 and 3. Licence, README, and the three decisive issues |
| `carlbomsdata/winquick` | — | 263 | pass 1 only. Metadata and licence |

---

## Verdicts

| repository | verdict | the reason, in one line |
| --- | --- | --- |
| `talaria0101/sandbox-insights` | ⭐ **adopt** | the probe discipline, the discriminator and its controls, and the four walls |
| `talaria0101/vm-research` | ⭐ **adopt** | the `podvm` machine tier, the serial exec protocol, and the fleet and fork mechanisms |
| `talaria0101/nix-experiment` | ⭐ **adopt** | the litmus payload, the version forensics, and the two walls podbox has not closed |
| `Azathothas/sandbox-insights` | ⭐ **adopt** | the seven-plane model, the layer contract, and twelve open questions with closure tests |
| `Azathothas/memfd-ng` | **filed elsewhere** | [deps.md](../../TODO/deps.md) T-0909 owns it |
| `cubic-vm/cubic` | ⭐ **anti-pattern exhibit** | a shipped argument-splitting defect podbox would repeat. Also one mechanism worth adopting |
| `hust-open-atom-club/Vex` | **confirms** | a Docker-like verb set over QEMU configurations. Independent evidence for the parity rule |
| `Obirvalger/vml` | **confirms** | machine-as-a-directory, and hardware acceleration assumed throughout |
| `gevico/tcg-rs` | **filed elsewhere** | [podvm.md](../../TODO/podvm.md) T-1307. Interesting and not usable yet |
| `qemu-rs/qemu-rs` | ⛔ **refused** | GPL-2.0-or-later, a plugin binding rather than a manager, and an unresolved maintenance question |
| `carlbomsdata/winquick` | **filed elsewhere** | [milestones.md](../../TODO/milestones.md) T-1112 owns it |

---

## The findings

### F1. ⛔ A single TCG multiplier is a claim about one benchmark

⭐ **This is the most load-bearing correction the sweep produced**, because
[podvm.md](../../TODO/podvm.md) exists to decide when the machine tier is worth
its cost.

Four measurements of "the TCG tax" on one host class:

| workload | host | guest | ratio | source |
| --- | --- | --- | --- | --- |
| md5 of 16 MiB | 0.026 s | 0.07 to 0.08 s | **~3×** | vm-research `logs/66-tcg-benchmarks.log` |
| tight integer loop, 30 M iterations | 854.1 Mops/s | 274.7 Mops/s | **3.1×** | sandbox-insights `logs/55-tcg-exec-and-bench.log` |
| dependent double chain | 708.8 Mops/s | 78.4 Mops/s | **9.0×** | the same log |
| xorshift32 + double + FNV, 30 M | 721.4 to 754.8 Mops/s | 31.4 to 35.3 Mops/s | **~21×** | vm-research `logs/72-bench-matrix.log` |

⛔ **The range is 3× to 21× across four workloads on one host class.** Yet
`vm-research/docs/comparison.md` states "TCG: ~20-25× under native" as the
speed row of its decision table, and repeats "you pay 20-25× compute" in its
recommendation. Both of its own figures are correct and they measure different
things.

⭐ **Two of the sources already say so.** sandbox-insights writes that "any
single-number TCG multiplier is a claim about a benchmark and a moment, not
about emulation". `Azathothas/sandbox-insights` files it as open question **O8**
with the closure test: run integer, syscall, memory-bandwidth, compilation and
I/O workloads, and report distributions with same-day controls.

⚠ **Every checksum matched across every platform in both trees** (`69d0fd33`
and `165be307`), so none of these is measuring a different computation. The
spread is real.

→ [podvm.md](../../TODO/podvm.md) T-1308.

### F2. ⛔ The target contract moved three times under observation

The four trees describe **three different instances** of the class, and two of
them changed while being measured.

| property | vm-research, 09-09 | sandbox-insights, 09-10 | nix-experiment, 09-11 |
| --- | --- | --- | --- |
| uid map | `0 1000 1` | `0 0 1` | uid 0, map not recorded |
| new mount API | ⭐ `fsopen`/`fsmount` **execute** | ⛔ refused pre-entry | not probed |
| loopback TCP connect | ⛔ `EPERM` | ⭐ allowed | not probed |
| external `:80` | ⛔ denied, then allowed mid-session | ⭐ allowed | not probed |
| `mknod` | char 0:0 only | char 0:0 only, real device denied | **char 0:0 only**, 24 majors swept |
| `/dev/ptmx` | not probed | not probed | ⛔ **absent, and devpts unmountable** |
| `readdir("/")` | not probed | not probed | ⛔ **`EACCES`** |
| `clone3` namespace flags | ⭐ execute | ⭐ execute | not probed |

⭐ **The mount-API gap closed between 09-09 and 09-10 while the clone-family gap
survived the same patch cycle.** Both states are committed in vm-research
`logs/60-` and `logs/77-`, and `logs/77` carries `fsopen: Operation not
permitted` against a script header that still says `fsopen/fsmount OK`.

⛔ **The engineering consequence is already AGENTS.md's rule** and this is
independent evidence for it: the runtime a specification describes is a floor.
Probe everything. Hard-code nothing. Cache by boot identity plus a canary
vector, and invalidate when any canary changes.

⚠ **Two properties appear only in the nix tree and podbox has never probed
them**: `readdir("/")` denied, and no `/dev/ptmx` anywhere.
→ [complete.md](../../TODO/complete.md) T-0414.

### F3. ⭐ The nix litmus test is a milestone, and its two walls are already entries

⭐ **The working answer that experiment reached is what podbox is**: a plain
`chroot(2)` over a hand-assembled rootfs, with regular-file device stand-ins,
a host CA bundle, and ownership-neutral extraction. Every route with a
namespace, a mount or a `ptrace` in it failed.

The version forensics, verified in that tree's own review B by reading
`lib/minver.nix` at each tag and by `grep -c posix_openpt` on `build.cc`:

- ⛔ **Nix ≥ 2.3.0 calls `posix_openpt()` in `startBuilder()` unconditionally.**
  No setting disables it. On a host with no `/dev/ptmx` **no Nix ≥ 2.3 can
  build**, on the host or in a chroot.
- Nix ≤ 2.2.2 captures builder output through a pipe.
- nixpkgs 22.05 gates at `"2.2"`; 22.11 gates at `"2.3"`; 23.11 additionally
  needs `builtins.isPath`, which is Nix ≥ 2.4.
- ⭐ So **nix 2.2.2 + nixpkgs 22.05** is the newest release pair whose whole
  pipeline runs with no pty.

Four `dont*` attributes are needed because there is no procfs, so `/dev/fd/N`
does not exist and bash process substitution (`done < <(find ...)`) breaks:
`dontPatchELF`, `dontRewriteSymlinks`, `dontPatchShebangs`, `noAuditTmpdir`. A
scan of all 38 setup hooks at 22.05 found exactly these four.

⚠ **The entropy correction.** That experiment wrote **4 KiB copied from the
host's `/dev/urandom`**, after an *empty* file made `std::random_device` in
gcc-7.3's libstdc++ throw during the first download. podbox writes 1 MiB from
`getrandom(2)` at `crates/podbox-complete/src/devices.rs:50`. ⭐ podbox's choice
is the better one, and the two are not the same fix.

⭐ **An `LD_PRELOAD` entropy shim was tried there and it did not work**, because
libstdc++ opens the file through `syscall()` and no libc interposer sees that.
This is wall 3 appearing in a real payload, and it is why the file is the fix.

→ [milestones.md](../../TODO/milestones.md) T-1113, the M8 milestone.

### F4. ⛔ None of the five VM tools is a microVM manager for this environment

⭐ **This answers the question the sweep was given.** None of them replaces
[podvm.md](../../TODO/podvm.md) T-1301 to T-1306, and none is a dependency.

| tool | what it actually is | why it does not transfer |
| --- | --- | --- |
| `Vex` | a Docker-like CLI that saves, shares and launches named `qemu-system-*` configurations | it composes command lines. It boots nothing itself and assumes a working host |
| `cubic` | boots official distribution cloud images with `cloud-init`, on Linux, macOS and Windows | ⛔ it accelerates **every** VM with KVM, Hypervisor or WHPX. Hardware acceleration is exactly what the target denies |
| `vml` | machines as directories with a `vml.toml`, `cloud-init` initialised | ⛔ requires `kvm`, plus `rsync`, `socat` and `cloud-localds` |
| `tcg-rs` | ⭐ a Rust reimplementation of QEMU's TCG: RISC-V guest to x86-64 host, JIT, 816 tests, difftest against QEMU | RISC-V guest only. podbox needs x86-64 guests |
| `qemu-rs` | Rust bindings to QEMU's **TCG plugin** C API | ⛔ not a manager at all, and GPL-2.0-or-later |

⭐ **Two mechanisms are still worth taking.**

1. **Ask the emulator which accelerator works; never assume one.** `cubic`
   pull request #538 is `feat: ask qemu which accelerator works on the host`,
   and its issue #6 is `Only use KVM if available`. That is the probe-do-not-
   assume rule, reached independently by a project with no connection to this
   one. → T-1301.
2. ⛔ **`cubic` issue #448, open, is a defect podbox would repeat.** Its
   `--qemu-args` passthrough splits the string on a single space, so any
   argument whose value contains a space is torn apart and QEMU rejects the
   result. The report names `src/qemu/qemu_system.rs` line 154. podbox will need
   the same escape hatch. → T-1302.

⚠ **`tcg-rs` is filed, not refused.** The target permits RWX `mmap`/`mprotect`,
which is what a JIT needs, so a pure-Rust TCG could in principle run where QEMU
runs. It is years from useful for podbox and the row exists so no future
session re-derives that. → T-1307.

### F5. ⛔ `qemu-rs` is refused, and the tracker is why

Three facts, none visible in the code:

1. `tree/Cargo.toml:7` says `license = "GPL-2.0-or-later"`.
2. **Issue #38, `Maintaining QEMU-RS`, is open**, and issue #43 asks whether
   those maintenance problems "have been resolved" before QEMU 11.0 changes the
   plugin API again.
3. **Issue #49** is the upstream QEMU TCG-plugin maintainer proposing native
   Rust support in QEMU itself. A breaking change is already merged upstream,
   and a contributor replies that qemu-rs needs "major changes, maybe even a
   hard cut ditching all previous API versions". The same contributor states
   that the crate "does not currently free any of the data it ought to on TB
   cache flushes".

⭐ **This is what the tracker pass is for.** The licence alone settles it, and
without the tracker nobody would know the crate is mid-upheaval.

### F6. ⚠ Two defects in the sources' own fixtures

⭐ **Both matter because podbox will build the same fixtures.**

1. ⛔ **The interposition-reach fixture does not hold its arguments constant.**
   Its header and README both say the three victims run "the same two syscalls
   each". `victim_dyn.c:16` calls `lchown(path, 0, 42)`, an **unmapped** gid,
   while `victim_static.c:11` and `victim_go.go:16` call `lchown(path, 0, 0)`,
   a **mapped** one. The shim-visibility result stands, because no `SHIM:` line
   appears for the static and Go victims. ⛔ But `victim_static: lchown -> 0` in
   the log is **not** evidence that a static payload escapes the ownership wall,
   and a careless reader would take it as such. → [interpose.md](../../TODO/interpose.md) T-0712.
2. ⚠ **A chroot appliance's `/dev/null` claim is contradicted by its own log.**
   `60-chroot-appliance.sh` asserts that redirection fails because `/dev/null`
   cannot be a device. Its log shows the `|| echo " FAILS"` branch never fired,
   so the redirection **succeeded**. The paper hedges correctly with "may not
   exist"; the experiment header does not. ⭐ `Azathothas/sandbox-insights`
   states the real rule: if `/dev/null` is absent, shell redirection creates a
   **growing regular file** with that name, so validate the file type before
   the workload starts. → [complete.md](../../TODO/complete.md) T-0415.

---

## What transfers, and what must not

⭐ **Adopt mechanisms, not architectures.**

| transfers | does not transfer |
| --- | --- |
| the bogus-argument discriminator with standing controls | the four-mechanism model as a fixed list. `Azathothas/sandbox-insights` already extends it to **seven planes** |
| paired witnesses: `mknod` 0:0 beside a real device number | any single instance's filter contents |
| a disposable child per probe, and the CHILD prints its own errno | the numbers |
| exit 0 matched, 1 contradicted, 2 could not run | the PowerShell gate shape |
| the four-leg seccomp-notification contract, refused if any leg is missing | a fallback-per-call supervisor, which turns a dry run into mutation |
| the serial FIFO pair, nonce readiness, line-anchored CR-tolerant markers | the 20-25× figure |
| the layer contract: order, whiteouts, links, containment, preflight | `cloud-init`-based image assembly |
| the strict root-entry order: validate, open descriptors, `chroot`, `chdir`, resolve, `exec` | hardware acceleration in any form |

---

⛔ **Assume more remain.** The sweep corrected six claims from the reading it
replaces, one of which reversed a licence determination and one of which
reversed a vendorability determination. Four references were read at three
passes rather than four, one at a single pass, and `vml`'s vendored tree was
not opened at all.
