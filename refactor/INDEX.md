# Refactor index: implementing the shell retirement

**Read this file first.** It is the entry point for implementation. It states
what podbox is doing, what the goal is, which host proves what, what the
analyses corrected, what an implementor must decide, and which task to take.

## 1. The goal, in one paragraph

podbox is a Linux container runtime that runs payloads where namespaces,
mounts, devices, and ownership changes can be denied. It has a machine tier and
an SSH transport. It measures operations and reports the mechanism it enters.
The repository also carries 121 shell, Python, and PowerShell scripts under
`experiments/` and `scripts/`, totalling 29,153 lines. A seven-round audit
concluded that most of those scripts are measurements that belong in native Rust
rather than in a shell. This refactor retires them: it deletes what Rust already
asserts, converts the rest to Rust tests or Rust tooling, and leaves in shell
only the 29 scripts that need a real engine, a device, or an OS facility.

The plan is [06-entries/PLAN.md](06-entries/PLAN.md). Read it before any entry.
It carries the citations, the verdict distribution, and the wave reasoning. This
file carries the task list, the host split, the corrections, the open decisions,
and the dispatch order. Where the two disagree, PLAN.md is the plan and this
file is the correction record: section 4 lists every disagreement.

## 2. Repository rules an implementor needs before touching anything

These are not optional and this file is the only place they are collected.

- Run `py scripts/check-todo.py` before and after every change. It is the record
  gate. It exits 0 on a clean tree and it is currently **green**.
- **Never call `wsl.exe` with a payload argument.** [docs/containers.md](../docs/containers.md)
  carries the rule and the reason: a payload handed to `wsl.exe` as an argument
  is expanded before the guest reads it, and the guest then parses the result a
  second time. Use `wsl-toolkit --instance podbox` or the commands in section 5.
- Every other repository is read only. Fix carried source in this tree.
- Use CodeGraph before a source search. Prose can be searched directly.
- Read the linked rule and the current source. A previous record is a claim to
  verify.
- Record an unknown value as a dash. Label an estimate each time it appears.
- Read exit codes without a pipe. Bound network and process waits.
- Work ASD-STE100 prose. ASCII where possible. **No em dash.** The markers
  `⛔`, `⭐`, `⚠`, `✅`, `❌` have fixed meanings in
  [docs/conventions/prose.md](../docs/conventions/prose.md).
- Attribute commits and release notes to the operator alone.
- Keep credentials out of source, output, and commit messages.
- Give each experiment a unique number and keep its script and its result.
- Add a plant with each new check. A green check without a failure test is
  insufficient. See [scripts/plant.sh](../scripts/plant.sh) and
  [docs/methodology/gate.md](../docs/methodology/gate.md).

Full rules: [AGENTS.md](../AGENTS.md), [TODO/RULES.md](../TODO/RULES.md),
[docs/conventions/code.md](../docs/conventions/code.md),
[docs/conventions/forbidden-patterns.md](../docs/conventions/forbidden-patterns.md),
[docs/methodology/authoring.md](../docs/methodology/authoring.md).

## 3. The sources this file consolidates

| file | what it holds | author |
| --- | --- | --- |
| [06-entries/PLAN.md](06-entries/PLAN.md) | the master plan | the seven-round audit |
| [06-entries/T-R000.md](06-entries/T-R000.md) through [T-R006.md](06-entries/T-R006.md) | seven wave entries, one per wave | the audit |
| [06-entries/verdict-ledger.tsv](06-entries/verdict-ledger.tsv) | 121 scripts with verdict and source report line | the audit |
| [00-orientation/](00-orientation/) through [07-verify/](07-verify/) | the audit's own rounds and its verification pass | the audit |
| [recon-a.md](recon-a.md) | host feasibility, 66 work units | this session |
| [recon-b.md](recon-b.md) | blockers, 30 classified | this session |
| [recon-c.md](recon-c.md) | 100-task decomposition, dependency graph, batches | this session |
| [recon-wslc.md](recon-wslc.md) | the `wslc` evaluation, superseded in part | this session |
| [compare-toolkit-1.md](compare-toolkit-1.md) | the non-root capability audit | this session |
| [compare-toolkit-2.md](compare-toolkit-2.md) | the lane comparison and the routing rules | this session |
| [review-index-1.md](review-index-1.md) | review of this file, claim by claim | this session |
| [review-index-2.md](review-index-2.md) | review of this file as a dispatch document | this session |

## 4. The host, as measured

The plan closes seven entries with "Nothing compiles on the Windows host:
`cargo` stops at `linker 'cc' not found`." That is true of bare `cargo test` on
this machine. It is not a statement about what the host can prove, and taking
it as one costs the schedule.

All measurements below were taken on 2026-10-01 on this machine.

| measurement | result |
| --- | --- |
| `cargo check -p podbox-probe` | exit 0 |
| `cargo check --target x86_64-unknown-linux-gnu` on `podbox-probe`, `podbox-enter`, `podbox-windows` | exit 0, 2.36 s |
| `cargo test -p podbox-probe --no-run` | exit 101, `linker 'cc' not found` |
| `cargo check --target x86_64-pc-windows-gnu` | exit 101, `E0433: cannot find unix in os`, `E0599: no method named pre_exec` at `crates/podbox-probe/src/sys.rs:815` |
| `sh scripts/windows/run-in-base.sh` with a probe job | exit 0, `Linux 7.2.0-WSL2-STABLE`, `/usr/bin/cc` present |
| `cargo test -p podbox-probe` on the `wslc` lane, measured earlier in this session | 108 passed, 0 failed, exit 0 |

**The source is Unix-only at the type level.** The third row is a typecheck
failure, not a link failure, so no linker change moves it. Files using
`std::os::unix`, `libc::`, `nix::`, `pre_exec`, `flock(`, or `syscall(`, counted
per crate over `crates/<crate>/src`: enter 8/10, ssh 8/14, cli 6/30, image 6/19,
complete 5/8, probe 2/17, extract 2/9, windows 2/7, supervise 1/3. Eight of
nine crates. `podbox-supervise` at 1/3 is the only one a single `mod` could gate,
and its one file is `src/launcher.rs`, which every other crate depends on
transitively.

So three things must be kept apart, and the distinction is what decides what
parallel agents can do:

- **authoring.** Writing Rust or markdown. Native. Always available.
- **compiling.** `cargo check` works natively for the host target, and for three
  crates against `x86_64-unknown-linux-gnu`.
- **executing a test binary.** Needs a Linux container. Section 5 gives the two.

A pure unit test can be written natively and proven on a lane. That is the
normal case in this plan, and it is why a fleet of authors is worth running even
while no lane is healthy.

## 5. The two lanes

### 5.1 The repository's own lane, which is the default

```sh
sh scripts/windows/run-in-base.sh JOB.sh
```

It **copies** the tree into a disposable container inside the distribution
`wsl-toolkit-podbox` and runs the job there, and hands evidence back through
`/out`. Measured 2026-10-01: exit 0, Linux 7.2.0-WSL2-STABLE, a real
`/usr/bin/cc`. It copies 343.2 MiB across 12,047 entries, so budget minutes,
not seconds.

Use this lane by default. It is the repo's own, it is documented, and it has
the property that matters most here: it does not mutate the working tree.
⛔ **Run `wsl-toolkit` from PowerShell, not Git Bash.** MSYS rewrites a guest
path such as `--dir /workspaces/proj` into `C:/Program Files/Git/...` and the
tool refuses the rewrite. Read
[scripts/windows/run-in-base.sh](../scripts/windows/run-in-base.sh) in full
before relying on it.

⛔ **⛔ Do not add `--exclude` for anything.** The wrapper already excludes
`codegraph.db` and its WAL and SHM sidecars, `daemon.log`, `daemon.pid`,
`target`, `.dev`, and `.tmp`, by name, at its `EXCLUDES` line. Measured: adding
`--exclude references` or `--exclude experiments` turns the record gate red,
`references` giving 12 "does not exist" errors from `TODO/reference-map.md` and
`experiments` a `FileNotFoundError` on `experiments/110-bloat-delta.sh`. The
only large safe exclusion left is none. The 343 MiB copy is the correct number.

⛔ **`--max-bytes` is 1.0 GiB and the tree is near it.** A hand-rolled
`wsl-toolkit run --workspace` over this checkout is **refused**:
`workspace refused: the workspace passes 1.0 GiB at
references/mhx__dwarfs/api/releases.json`, exit 2. The wrapper is the supported
entry point, because it applies the exclusions. Do not build your own call.

⛔ **The job container is not bootstrapped, and the bootstrap is the real cost.**
A bare job has `cc` and `ssh` but **no `zig` and no `jq`**.
`.cargo/config.toml` routes the C compiler to `scripts/zig-cc.sh`, so any crate
pulling `ring` fails at `cc-rs`. Note that `podbox-extract` is **not** exempt:
`Cargo.toml:18` pulls `podbox-image`, which pulls `rustls` with the `ring`
feature. Only `podbox-probe` is bootstrap-free. CI avoids this by running
`./scripts/common/bootstrap-env.sh rust cc zig tools openssh` first, at
`.github/workflows/gate.yml:197`. ⛔ **That bootstrap cannot run as a non-root
user.** It pins `ZIG_PREFIX="/opt/zig"` at its line 71, neither `/opt` nor
`/usr/local/bin` is writable by uid 1000, and there is no `sudo`. Measured:
`FAILED tools`, `FAILED openssh`, `FAILED zig`, exit 1, with
`mv: cannot move ... to '/opt/zig': Permission denied`. So a non-root job must
install zig into `$HOME` itself, at about 53 MB and 25 s **per job**, not
shareable, because each job gets a fresh copy of the tree. As root the
bootstrap works and `cargo test -p podbox-complete` reports **51 passed, 0
failed**.

⛔ **The lane runs as uid 0 by default, and that is now known not to matter.**
Measured `UID=0`. The concern was that tests asserting the `mknod`-denied arm
would fail under root. They do not: root inside this container lacks
`CAP_MKNOD` (`CapEff: 0x800405fb`, bit 27 clear) and `mknod` is refused even
inside `unshare -r` with all capabilities, so **root and uid 1000 take the same
arm** and both pass 51 of 51. The denial sits below the capability layer, not in
the uid. ⛔ **Section 10's uid-0 list is withdrawn: there is no uid-0 blocked
task on this lane.**

⛔ **It was down and needed repair before that measurement.** The engine's
cached boot id did not match the current boot, so every container was refused
before it started. The repair is the tool's own, and it cost nothing:

```sh
wsl-toolkit --instance podbox base ensure --repair
```

After it, `base status --probe` reports `usable true` and podman 6.1.2. The base
has **no cgroup delegation**, so a memory or CPU limit is accepted and not
enforced. That is recorded in `TODO/PROGRESS.md` and is not this plan's work.

⛔ **The lane leaves debt that turns the record gate red.** Every job leaves a
retained job, and `scripts/check-todo.py` fails while any is kept. If the gate
goes red with `wsl-toolkit: lane job still kept`, clear it:

```sh
wsl-toolkit --instance podbox gc --job JOB_ID --apply
```

### 5.2 `wslc`, which is faster but has two limits

Microsoft ships a Linux container CLI inside WSL, `wslc` 3.0.1.0, at
`C:\Program Files\WSL\wslc.exe`. It is **not on the Git Bash PATH**; invoke it
by full path. It is a separate executable, not a `wsl.exe` alias, and it is
closer to docker and podman at the command line than to `wsl-toolkit`.

```sh
MSYS_NO_PATHCONV=1 "/c/Program Files/WSL/wslc.exe" run --rm \
  -v "C:/Users/AjamX/Downloads/podbox:/work" -w /work \
  docker.io/library/rust:1.98.1-bookworm sh -c 'JOB_COMMAND'
```

- `MSYS_NO_PATHCONV=1` is **required**. Without it Git Bash rewrites `-w /work`
  into `C:/Program Files/Git/work` and the call fails with `Error code: E_FAIL`.
- The volume takes a Windows path on the left and a POSIX path on the right.
- It runs a container as **uid 0** by default, takes `-u/--user`, has a real
  `/usr/bin/cc`, and **bind-mounts** the checkout.
- It has no `--privileged`, no `--cap-add` or `--cap-drop`, and no seccomp,
  apparmor, or userns flag. `wslc run --help` and `wslc create --help` confirm
  the absence. Whether it offers user, mount, or PID namespaces is undocumented
  by Microsoft and was not established. ⛔ **Its `-u` is unmeasured**: every
  attempt since it wedged failed with `ERROR_SHARING_VIOLATION`, so do not
  assume `-u 1000:1000` works there.

⛔ **Limit one, uid 0.** `wsl-toolkit run` runs as the image's default user,
which is root. `--user 1000:1000` is available and works, but it buys nothing
for this plan: root and uid 1000 take the same `mknod` arm here, so both pass
every test. `--user toolkit` by **name** fails with
`unable to find user toolkit: no matching entries in passwd file`; use the
numeric form. A non-root job also cannot run the bootstrap, so it is strictly
worse than root for this workload. Keep root.

⛔ **Limit two, it wedges.** When a `wslc` job is killed rather than allowed to
finish, the session is left holding the **image store**, and every later call
fails with `ERROR_SHARING_VIOLATION`. Measured again on 2026-10-01 after agent
runs: `wslc run -u 1000:1000` and `wslc images` both fail, while `wslc --version`
still answers. `wslc system session terminate --help` documents **no id option**;
it terminates the default session, which was not the one at fault.
`wsl.exe --shutdown` did not clear it either, and the skill for that tool
forbids it outright because it breaks every WSL instance on the machine. The
holder is the `vmmemwslc-cli-AjamX` VM and not a registered distribution.

⛔ **It is still wedged, and the failure is shared.** The lock is on the image
store, so **one wedged job blocks every other agent** and there is no per-agent
recovery. For a fleet of 6 to 19 that is disqualifying on its own.

### 5.3 What the comparison settled

Measured 2026-10-01 by two independent agents. The full reports are
[compare-toolkit-1.md](compare-toolkit-1.md) and
[compare-toolkit-2.md](compare-toolkit-2.md).

| question | measured answer |
| --- | --- |
| Lane A total, `cargo test -p podbox-probe` | **38.9 s**: 6.3 s copy and startup, 32.6 s `cargo`, 0.09 s the 108 tests |
| Lane B, the same test | **about 40 s**, inherited from an earlier run, not re-measured |
| non-root versus root, same proof | 48.1 s versus 52.95 s wall, 2 samples, **no difference either way** |
| is `--exclude` a cost lever | **spent**. Safe reduction is 0 bytes; every large exclusion turns the gate red |
| do lane tasks need the copy | **all 53 of them**, counting the 47 `linux-lane` and 6 `both` tasks. Each proof writes a tracked path then reads it. **Zero are read-only**, so a bind mount buys nothing |
| Lane A concurrency | **it parallelises.** 4 concurrent jobs all slept their full 20 s; 6 concurrent 343 MiB copies ran 8.9 to 10.3 s each against 6.2 s for one, about 1.6x contention, no serialisation |
| shared `target/` | **worse than blocking.** Two `cargo build`s on one target directory both fail `E0463` could not find crate. Each Lane A job already has its own `target/`, because the wrapper excludes it from the copy |
| one dead job's blast radius | 20 leftover jobs produced **104 `check-todo` problems**, about 5 per job, because `check-todo.py:1339` emits one error per retained item with no scoping. A neighbour's dead job fails everyone else's landing proof |

⛔ **So the answer to "wsl-toolkit when needed, else wslc" is no: `wslc` runs
zero proofs.** It is the same speed, it cannot take a copy, its one advantage
does not apply because no task here is read-only, and one killed job takes down
the whole fleet. **Use Lane A for everything.** `wslc` stays useful as a fast
read-only probe of a base image, and that is all.

### 5.4 The routing rules

Mechanically applicable. An agent applies these without judgement.

- **R1.** Every proof runs on Lane A through `sh scripts/windows/run-in-base.sh
  JOB.sh`. There is no `wslc` fallback.
- **R2.** Pass no `--workspace` only when the job script cites no tracked path.
- **R3.** Never add `--exclude` for `references`, `experiments`, `.git`,
  `crates`, or `target`. The wrapper already excludes `target`.
- **R4.** ⛔ **Collect the job by its full 16-hex id before running the record
  gate.** `gc --job` needs it. The 12-character form printed in the log header
  exits 0 and **removes nothing**, which is a silent failure. Get the id from
  the `--json` answer: `wtk-<16hex> (<12hex>)`.
  `wsl-toolkit --instance podbox gc --job JOB_ID --apply`, or
  `wsl-toolkit --instance podbox gc --apply` for the lot.
- **R5.** If `wsl-toolkit --instance podbox base status --probe` does not report
  `usable true`, run `wsl-toolkit --instance podbox base ensure --repair`.
- **R6.** ⛔ **Never kill a Lane A job.** Use `wsl-toolkit --instance podbox stop
  JOB_ID`. Killing the client leaves an open record that only R4 clears.
- **R7.** Run `wsl-toolkit` from PowerShell, and read `$LASTEXITCODE` on the
  line after the command.

## 6. What the analyses corrected

Recon B classified 30 blockers as 21 `TREE`, 2 `HOST`, 3 `SELF-REFERENTIAL`,
3 `UNRESOLVED-DECISION`, and 6 `FALSE`, with 2 rows overlapping. Recon A and C
found more. The findings below change the plan. Each carries the command that
settles it.

| # | finding | evidence | consequence |
| --- | --- | --- | --- |
| 1 | **There are seven waves, not six.** `T-R006.md` exists and carries the twelve DELETE scripts. `PLAN.md` section 8 says `T-R000.md` through `T-R005.md`. `T-R005.md` and `T-R006.md` each call themselves the sixth. | `ls refactor/06-entries/T-R00*.md` returns 7 files | An implementor following section 8 drops a wave. Fix PLAN.md sections 5 and 8. |
| 2 | **Two size ceilings die with the files that declare them, and no entry notices.** `scripts/check-todo.py:228` reads `^CEILING_BYTES=(\d+)$` out of `experiments/110-bloat-delta.sh`; check 23 reads `INTERPOSE_CEILING_BYTES` out of `scripts/build-interpose.sh:48`. Wave 4 deletes both files. | the regex matches `CEILING_BYTES=8000000` and does not match `pub const CEILING_BYTES: u64 = 8000000;` | The first wave-4 commit turns the gate red through checks 17 and 23, and the entry's own `Prove` reports it. This is decision **D-1**. |
| 3 | **Wave 4's replacement `Prove` cannot run.** `./target/release/podbox-gate` is the right shape of command, but the `todo` job runs zero cargo commands and has no `bootstrap-env.sh`, so the binary does not exist on that runner. | `awk '/^  todo:/,/^  build:/' .github/workflows/gate.yml \| grep -c 'cargo\|bootstrap-env'` returns 0 | T-R004 names the self-reference and supplies a fix that inherits it. Add a build step to `todo`, or move the check into the `build` job. |
| 4 | **The `110` inversion: wave 4 deletes the file's readers while wave 5 moves the file.** | `scripts/check-todo.py:228`; `scripts/plant.sh:51` and `:191`; `.github/workflows/gate.yml:131`; and 10 `Prove` lines in `TODO/deps.md`, measured with `grep '110-bloat-delta' TODO/deps.md \| grep -c Prove` | The waves cannot land in either order. Hoist `110` into wave 4. This is task **T-1558** and decision **D-2**. |
| 5 | **A second workflow consumes four wave-4 and wave-5 subjects.** `.github/workflows/nightly.yml` runs `build-interpose.sh` at `:84`, `nightly-smoke.sh` at `:103`, `package-ssh.sh` at `:112`, and `release-notes.sh` at `:179`. | those four lines | Every entry treats "the full gate" as the four jobs in `gate.yml`. The nightly breakage is silent until a release attempt. |
| 6 | **26 retirement scripts are in no wave.** 22 `SPLIT` and 4 `RUST-TEST` are named in none of the seven entries. | ledger basename set minus the seven entries: 48 unmatched, of which 21 are KEEP-SHELL and legitimately absent | `PLAN.md` claims 81 scripts retire. 26 of them have no implementation. This is decision **D-7**. |
| 7 | **The ledger reconciles with the plan's table in three of five rows, not all five.** After VC-2 and VC-3 are applied, the ledger gives KEEP-SHELL 29, SPLIT 35, RUST-TOOL 27, RUST-TEST 19, DELETE 11 against the plan's 29, 36, 27, 18, 11. | the tallies computed from `verdict-ledger.tsv` with VC-2 and VC-3 applied | Two rows are one script apart, not four. The earlier claim in this file that four rows moved was wrong. One script is unaccounted for and the ledger needs one row corrected. |
| 8 | **T-R002's "source defect" is not a defect.** `experiments/results/store-gc.txt` records `prune_skipped 1`, `crates/podbox-cli/src/images.rs:961-986` returns 0 on the success arm, and `experiments/160-store-gc.sh:138-139` asserts both 0 and `skipped:`. All three agree. | those three reads | Drop the defect framing. The clause is already satisfied and its test can be written straight. |
| 9 | **Three more stale citations of the class wave 0 exists to fix.** `crates/podbox-cli/src/system.rs` has `mod tests` at 604, not 603, and `fn abi` at 157, not 191-214. `crates/podbox-cli/src/parity.rs` has `mod tests` at 707, not 706. | `grep -n 'mod tests' crates/podbox-cli/src/system.rs crates/podbox-cli/src/parity.rs` | Add to wave 0's list. |
| 10 | **`PLAN.md` names a file that does not exist.** `crates/podbox-supervise/src/nongoals.rs`; the file is `crates/podbox-probe/src/nongoals.rs`. | `ls crates/podbox-supervise/src/` returns `launcher.rs`, `lib.rs`, `table.rs` | T-R002 corrected three such paths and missed this one. |
| 11 | **Concurrent `TODO/` edits are unsafe.** `scripts/todo-count.py` rewrites one Counts block from all 203 rows, and `check_tree` then reads the whole tree. | recon C section 4.5 | 41 of the 100 tasks write a `TODO/` file and must serialise behind one writer. This is the hard cap on fleet width. |
| 12 | **The plan is untracked, so no citation in it is held.** | `git ls-files refactor \| wc -l` returns 0; `git status --porcelain refactor` returns `?? refactor/` | `check_tree` reads tracked files only. A records decision, not code. |
| 13 | **`recon-c.md`'s task rows are broken markdown links.** They link to `TODO/<category>.md` entries that do not exist yet, because the tasks have not been filed. | `scripts/common/check-docs.sh` reports the broken links | Expected before filing. The task ids and the `Host` column are what this plan needs from that file, and both are readable. |

Two things were fixed rather than merely recorded. The record gate was **red**
on five retained `wsl-toolkit` lane jobs, which blocks wave 0's own `Prove`; the
jobs were removed with `wsl-toolkit --instance podbox gc --job JOB_ID --apply` and
`py scripts/check-todo.py` now exits 0. The base's stale run state was repaired
with `wsl-toolkit --instance podbox base ensure --repair`, as section 5.1
records.

## 7. Decisions that must be made before any code

These are not an implementor's to invent. Each is recorded with the options and
their costs so the decision is a choice rather than a guess. Nothing in
Batch 0 may be skipped, and Batches 1 through 4 all depend on part of this.

**D-1. Where the two size ceilings live.** Check 17 needs a file containing
`CEILING_BYTES=<digits>` and check 23 a file containing
`INTERPOSE_CEILING_BYTES=<digits>`, because both regexes are anchored to shell
grammar and wave 4 deletes both declaring files.
- (a) the ported binaries print the value on a flag and both checks are
  rewritten. This changes what the gate accepts, which `T-R004` forbids without
  a decision of its own.
- (b) the ported binaries keep a machine-readable declaration file, and check
  17's own "declared in exactly one place" rule holds against it.
- (c) the two ceiling files stay in shell as declaration only, and only the
  measurement moves to Rust.

⛔ **This file assumes (c)** because it is the smallest change that preserves the
check. An implementor who prefers (a) or (b) records the reason in T-R004's
`Decision` field and says why it is not a redesign.

**D-2. The `110` hoist.** `experiments/110-bloat-delta.sh` is read by
`check-todo.py:228`, by `scripts/plant.sh:51` and `:191`, and by `gate.yml:131`,
and it is moved to `podbox-release` in wave 5 while wave 4 deletes the first two
readers. Either `110` moves into wave 4 with its readers, or the ceiling check is
retired with a recorded reason. There is no third order that leaves the gate
green.

**D-3. Wave 2's plant mechanism.** `scripts/plant.sh:51` `FILES` names no
source file that any of wave 2's 13 clauses targets; it carries three `crates/`
paths, `podbox-supervise/src/lib.rs`, `podbox-cli/src/parity.rs`, and
`podbox-cli/src/run.rs`. Adding the 13 target files widens the guard at
`plant.sh:74` that refuses to start on a dirty tree, and its backup and restore
steps iterate every entry. Either each plant joins `FILES`, or wave 2's plants
run from a separate script. Recon B says do not start wave 2 first. ⛔ **This
blocks 13 tasks in Batch 1.**

**D-4. Wave 1's `Prove`.** T-R001 proves the two deletions with
`cargo test --workspace`, which reaches no clause of either deleted script: the
six `220` tests are unit tests in `podbox-extract` and the twelve `388` tests
are in `podbox-ssh`. Either the prose clause-to-test table is accepted as the
closure record and the `Prove` line says so, or the scripts are deleted only
after their clauses have plants.

**D-5. The `podbox-cli` test shape.** `cargo metadata` reports `podbox-cli`
targets as `bin` and `custom-build` with **no** `lib`, so every
`crates/podbox-cli/tests/*.rs` is a `CARGO_BIN_EXE_podbox` black-box driver
unless the crate gains a `[lib]`. T-R003 picks black-box in prose, but three of
its five `podbox-cli` test files assert internal state that a black-box driver
cannot reach. Name the shape per file, and do not add `[lib]` silently: it is a
new export surface and a new release artefact. ⛔ **This blocks 4 of the 7
agents in Batch 4.**

**D-6. Who writes the performance ceilings.** Check 30 reads
`experiments/perf-ceilings.tsv` and three result files.
`experiments/360-perf-harness.sh` is a wave-4 subject and no entry names the
binary that writes them.

**D-7. The 26 unassigned scripts.** Either a wave takes them or the plan's
retirement claim drops from 81 to 55. The second is a smaller truth and costs
one sentence in PLAN.md.

**D-8. Is `T-R006` authorised.** It deletes eleven scripts. `T-R006`'s own
table lists eleven while its title and `Source` say twelve; the twelfth is
`162`, refuted by VC-2 and correctly retained. The plan's section 8 does not
list the entry at all, so a reader following the plan never reaches it.

**D-9. WITHDRAWN.** This decision existed only to name the ninth task that a
root container could not prove. Measurement refuted the premise: root lacks
`CAP_MKNOD` here, so root and uid 1000 pass the same tests. **No task is blocked
on uid.** Section 10 states what replaces it, which is the missing `sshd` in the
job image.

**D-10. Where the 27 KEEP-SHELL scripts live.** End state point 1 deletes
`experiments/` entirely, and 27 scripts are KEEP-SHELL: they need a real engine,
a device, an OS facility, or a live guest. They cannot become Rust and they
cannot stay where they are. Each must **move** to `scripts/`, rewritten,
extended, and improved. The plan assigns them no wave and names no
destination. `scripts/` already holds the gate's own checks, so the move is
mechanical in shape but not in content: these are measurements, and the gate's
`scripts/common/check-*.sh` are checks. **Record the split in
`docs/containers.md`, which already carries the exclusion table, before the
first move.**

## 8. The tasks

**100 tasks, `T-1501` through `T-1600`.** The highest existing id is T-1423 at
`TODO/INDEX.md:266`, and `docs/methodology/authoring.md` says "Use a free task
ID." T-R000 through T-R006 are `refactor/` documents and do not occupy the
`TODO/` id space, so they are cited as parents rather than reused as ids.

The full table with one-line summaries, the `Host` class, `depends_on`,
`writes`, and `conflicts_with` is [recon-c.md](recon-c.md) sections 1 and 2, and
that file is the authority. The table below is the shape, with the host split
recomputed from that file's `Host` column on 2026-10-01.

| parent | task ids | count | native | lane | both |
| --- | --- | --- | --- | --- | --- |
| T-R000 wave 0, record fixes | T-1501 to T-1514 | 14 | 12 | 0 | 2 |
| T-R001 wave 1, deletions of converted scripts | T-1515, T-1516 | 2 | 0 | 2 | 0 |
| T-R001 wave 1, the ten recorded deletions | T-1540 to T-1549 | 10 | 10 | 0 | 0 |
| T-R002 wave 2, inline pure gaps | T-1517 to T-1529 | 13 | 0 | 13 | 0 |
| T-R003 wave 3, `tests/` directories and the fixture | T-1530 to T-1539 | 10 | 0 | 9 | 1 |
| T-R004 wave 4, `crates/podbox-gate` | T-1550 to T-1558 | 9 | 2 | 6 | 1 |
| T-R005 wave 5, the three tool crates | T-1559 to T-1569 | 11 | 0 | 11 | 0 |
| T-R006 wave 6, the DELETE scripts | T-1570 to T-1580 | 11 | 3 | 6 | 2 |
| decisions and cut set | T-1581 to T-1600 | 20 | 20 | 0 | 0 |
| **total** | | **100** | **47** | **47** | **6** |

⛔ **An earlier draft of this file said 99 tasks split 42 native, 51 lane, 7
both. Both the count and every split were wrong.** The figures above were
computed by reading the `Host` column of all 100 rows in recon-c, and two
independent reviews recomputed them and agreed. The six `both` rows are
T-1508, T-1511, T-1532, T-1551, T-1578, and T-1579; they are a useful check
because six is small enough that a miscount shows. **Use the per-row `Host`
value, never this table's totals, when dispatching one task.**

**41 of the 100 tasks write a `TODO/` file** and must serialise behind one
writer, per finding 11. The file that serialises is `TODO/INDEX.md`, because
`scripts/todo-count.py` rewrites one Counts block from all 203 rows. The other
59 write no `TODO/` file.

## 9. Dispatch

A wave is not a unit of independence. Wave 4 alone is eight binaries sharing a
crate, a grammar module, a `Cargo.toml` row, a workflow, and six `TODO/` files.
The work decomposes into 100 tasks and 43 batches, of which five matter.

**Batch 0, decisions and the cut set. 1 agent, serial, 3 tasks.**
T-1581, T-1558, T-1551. These unblock most of the rest and they are the
cheapest work on a clean tree. ⛔ **Nothing else starts until this lands**,
because findings 2 and 4 and decisions D-1 and D-2 are all in here.

**Batch 1, records and deletions. 8 agents, 28 tasks.**
The 14 wave-0 corrections grouped by the file each writes, the 11 wave-6
deletions, the 10 recorded deletions, and 13 wave-2 unit tests. Finding 11 and
decision D-3 apply: the tasks writing a `TODO/` file serialise behind one
writer, so 5 of the 8 agents run fully parallel and the rest queue.
⛔ **D-3 must be settled before the 13 wave-2 tasks start**, per recon B.

**Batch 2, the gate crate. 1 agent, serial, 8 tasks.**
T-1552 through T-1557, T-1577, T-1578. **This is the serial core.** One agent,
because they share a crate, a `Cargo.toml` row, `plant.sh`, and each one's
`Prove` is the record gate the previous one is deleting. It cannot be split: the
port is behaviour-preserving and the only proof of preservation is a
before-and-after diff of the same tree.

**Batch 3, the tool crates. 4 agents, 11 tasks.**
After Batch 2, split by crate: buildstate, release split per file, podvm. The
release tasks that need the `110` hoist and the `157` re-anchor wait for Batch 0.

**Batch 4, integration tests and the registry fixture. 7 agents, 14 tasks.**
T-1530 and T-1531 serial, the fixture before its two consumers, then the
`podbox-image` and `podbox-cli` test files. **This batch can start before Batch
2**; its `writes` intersect Batch 2's in zero paths. It is the one place the
plan's parallelism is real, and it closes the blocker `PLAN.md` section 6 names
as blocking `crates/podbox-image/tests/store_digest.rs` and every
registry-backed test. **If one session is wanted, Batch 4 alone is a complete,
shippable unit.** D-5 must be settled before its four `podbox-cli` agents start.

### The tasks no batch covers

⛔ **22 of the 100 tasks are in no batch above**, and an agent that reads only
this section will not know that. They are:

- **T-1550** and **T-1582 to T-1600**, which are 20 decision records. They
  belong to Batch 0 as it grows; Batch 0 is named as 3 tasks because those 3 are
  the cut set, and the other 19 are the per-wave decisions an implementor writes
  into their own entry. They are authoring, they are native, and they are not
  independent of the wave they document.
- **T-1571**, which recon-c places in wave 6 but which is not one of the
  eleven deletions the batch lists. Confirm its parent in
  [recon-c.md](recon-c.md) section 1 before dispatching it.

**No agent may take a task that no batch names.** Take one from a named batch,
or file the missing batch here first.

### Fleet width

**19 concurrent agents on a clean tree**, reachable only during the wave-0 and
wave-6 window. After the record-gate and workflow serialisations, sustainable
concurrency is **6**, and it never recovers above 6 in waves 2, 3, and 5. The
floor is **1** for all of Batch 2.

The serial core is 11 tasks, 11 percent of the work, gating 74 of the remaining
88. **89 percent of the work sits outside it**, and most of that can run 6-wide.

## 12. The road to the end state

The operator has fixed the end state. These six points are not negotiable and
this refactor is one stage of reaching them. They are recorded here so every
wave lands work that moves toward them, and so a later reader does not have to
rediscover the intent.

| # | end state | where the plan already serves it |
| --- | --- | --- |
| 1 | **All of `experiments/` is deleted.** Usable scripts move to `scripts/`, rewritten, extended, and improved. The rest become Rust. | Waves 0 to 5 delete and convert. ⛔ **The 27 KEEP-SHELL scripts have no home.** They stay in shell and `experiments/` is deleted, so each must **move** to `scripts/`. The plan never says this. That is decision **D-10**. |
| 2 | **The whole `refactor/` directory is deleted** when the refactor is done. | This file and `PLAN.md` are the last things standing. Everything they assert must be carried into `TODO/` before they go. |
| 3 | **All of `references/` is deleted.** | 46 external repositories, 168 MB, 47 citations from `TODO/reference-map.md`. ⛔ **The record gate reads it.** `check_tree` opens every tracked source file and resolves every cited path and line, and `docs/containers.md:88` records that the checks read the index and the cited source lines. Deleting `references/` turns the gate red until every citation in the tree is repointed at podbox's own source or dropped with a recorded reason. |
| 4 | **`vendor/` is natively rewritten as our own crate.** | `vendor/userland-execve` is 512 lines across 6 files, MIT, and **is not linked**: it is outside the workspace, uncompiled, with `goblin` and `nix` patched out per T-0909 and T-1003. `THIRD_PARTY.md:36` says so. The rewrite is small, bounded, and depends on nothing that runs today, so it can go **early**. |
| 5 | **`podbox-ssh` is purged and integrated** into `podbox remote ssh`, `podbox local ssh`, `podbox machine ssh`. | ⛔ **Two of the three verbs already exist.** `podbox remote ssh` and `podbox machine ssh` are dispatched from `crates/podbox-cli/src/main.rs:153-161` and documented in the `ssh_refusal` text at `:280-292`. **`podbox local ssh` does not exist**: there is no `local` group in `main.rs`. So the work is the `local` verb plus folding `crates/podbox-ssh` (9,160 lines) into the CLI. |
| 6 | **GitHub issues are mined, and the whole `TODO/INDEX.md` and its entries are rewritten toward closing them.** | 203 entries today. This is a re-planning exercise, not a wave, and it needs the issue list before it can be scoped. |

### Ordering

⛔ **Point 4 goes first of the six, and point 6 goes last.** Point 4 is
independent of everything else, it is 512 lines, and it removes a licence
obligation from `THIRD_PARTY.md`. Point 6 rewrites the plan this file carries,
so it cannot be scoped until the plan has run.

Points 1 and 3 are the expensive ones and they are **the same work seen twice**:
point 1 empties `experiments/`, and point 3 empties `references/`, and both turn
the record gate red the moment they land. Neither is a deletion anyone can do
alone.

⛔ **Point 2 is the last of all, and it is the one that gets forgotten.** By the
time `refactor/` is deleted, `T-R000.md` through `T-R006.md`, the 121-row
verdict ledger, and the 100 tasks must all have been carried into `TODO/`, or
the reasoning that produced the code is gone with the code's own history.

### What the seven waves do for these six points

| wave | serves | leaves owing |
| --- | --- | --- |
| 0, record fixes | 2, 6 | nothing |
| 1, delete what is Rust already | 1 | the D-10 moves |
| 2, inline pure gaps | 1 | nothing |
| 3, `tests/` and the fixture | 1, 5 | the `local ssh` verb |
| 4, `crates/podbox-gate` | 1 | the KEEP-SHELL moves |
| 5, three tool crates | 1 | the `394`/`397` assignment, D-2 |
| 6, the DELETE scripts | 1 | nothing |

⛔ **No wave in the current plan serves point 3, 4, or 5.** They are not
scheduled. This section names them so the gap is visible rather than assumed
away, and the next planning round owns filing them as tasks.

## 13. The lane runner

**Use [lane.sh](lane.sh).** It is the one command an implementor runs, and it
exists because the raw lane has seven traps that each cost an agent a session.
All seven were found by running them, not by reading about them.

```sh
sh refactor/lane.sh JOB.sh              # the job, with the toolset
sh refactor/lane.sh JOB.sh --user       # the job, as the unprivileged account
sh refactor/lane.sh JOB.sh --no-bootstrap   # skip the toolset install
sh refactor/lane.sh --check             # is the host ready
sh refactor/lane.sh --preflight         # --check plus one probe run
sh refactor/lane.sh --print-command     # show the call, run nothing
sh refactor/lane.sh JOB.sh --dump-payload   # show the generated payload
```

⛔ **Run it from Git Bash**, because the script is the Windows half of a job and
calls `wsl-toolkit` from a shell that does not rewrite guest paths. The tool
must never see a Git-Bash-rewritten path.

### Measured, on 2026-10-01

| run | result | wall |
| --- | --- | --- |
| default path, the acceptance job | `GATE_OK`, `51 passed`, `PROOF_JOB_OK`, exit 0 | 44.1 s |
| `--user`, the acceptance job | uid 1000, `zig 0.16.0`, `jq 1.6`, `SUDO_NOPASSWD`, `OPT_WRITABLE_VIA_SUDO`, `GATE_OK`, `51 passed`, `USER_JOB_OK`, exit 0 | 56.3 s |
| 4 concurrent jobs, default path | all four exit 0, 28.1 to 29.0 s, `CONC_GATE_OK` in every one | 31 s wall |
| argument handling | no job, bad job, unknown option: exit 2 each, read without a pipe | instant |

**The runner collects its own job.** After each run it reads the full 16-hex id
out of the `--json` answer and removes it, so a job does not leave the record
gate red for the next agent. Measured: 4 concurrent jobs left the gate green.

### The seven traps it closes

1. **The bare job image has no `zig` and no `jq`**, so every crate that pulls
   `ring` fails at `cc-rs` before a test runs. Only `podbox-probe` is exempt;
   `podbox-extract` is not, because `Cargo.toml:18` pulls `podbox-image`.
   `lane.sh` bootstraps first.
2. **The bootstrap cannot run as a non-root user.** It pins
   `ZIG_PREFIX="/opt/zig"`, which is root's, and reports `FAILED zig` with
   `mv: cannot move ... to '/opt/zig': Permission denied`. `lane.sh` therefore
   bootstraps as root, then lays down one passwordless sudo rule, then drops.
3. **The copy is root-owned**, so a non-root user cannot write `/work` and git
   refuses it: `fatal: detected dubious ownership in repository at '/work'`.
   That turns `check-todo.py` into `git ls-files failed ... This is not a
   pass`. `lane.sh` hands the tree to the account rather than adding a
   safe.directory exception, because a job that cannot write the code it tests
   proves nothing.
4. **`codegraph.db` must be excluded by name**, never the `.codegraph`
   directory, or the copy drops a tracked corpus file and the guest reads the
   tree as dirty.
5. **`PATH` must be prepended, never replaced.** cargo and rustup live in the
   image's `CARGO_HOME=/usr/local/cargo/bin`. Replacing PATH gave
   `cargo: not found` and exit 127.
6. **The drop must be a script file, not a nested quoted string.** The escaped
   `sh -c "..."` payload died silently after the drop line, exit 2, with nothing
   on either stream. `lane.sh` sends the drop as a second `--input`.
7. **`exec` loses the job's exit code.** `exec setpriv` replaced the payload
   process and the tool reported the drop's status, not the job's. `lane.sh`
   captures the code and exits with it.

## 10. Proofs

Every proof is a runnable command, on Lane A, per rule R1 in section 5.4.

⛔ **The uid-0 blocked-task list is WITHDRAWN.** An earlier draft of this file
named nine tasks that could not be proven because the lane runs as root. That
was wrong, and measurement is what refuted it: root inside this container lacks
`CAP_MKNOD` (`CapEff: 0x800405fb`, bit 27 clear), and `mknod` is refused even
inside `unshare -r` with all capabilities, so root and uid 1000 reach the same
arm and both pass `podbox-complete` 51 of 51. `devices.rs` lines 603 and 629 are
**assertions inside tests** at 591 and 615, not tests of their own. **All 100
tasks can be proven on Lane A.** The one real gap is different and narrower:
`crates/podbox-ssh/tests/common.rs:41` panics when `sshd` is absent, and the
job image has `ssh` but not `sshd`. That is a missing tool, not a uid. Install
`openssh-server` in the job or run the SSH tests in CI.

⛔ **`cargo test --workspace` does not reach `crates/podbox-interpose`.** It is
excluded at `Cargo.toml:20`. Its proof is the separate command at `gate.yml:213`:
`cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target
x86_64-unknown-linux-gnu`. Measured: 48 tests, and it needs the bootstrap
because it compiles C.

⛔ **A `Prove` that names a deleted file is the defect wave 0 exists to prevent.**
`scripts/check-todo.py` treats a bare `experiments/...` citation as a
tracked-file reference and turns red when the file is gone, but a `sh
experiments/...` or `./experiments/...` citation is **not** caught in either
direction. Grep both forms before and after every deletion.
`scripts/plant.sh` alone is named in 37 places under `TODO/` and 10 under
`docs/`, and 11 counting `docs/` and `.github/` together, of which 13 are
`Prove` lines in 2 files, `TODO/gate.md` and `TODO/deps.md`. A `sh`-prefixed
citation left behind leaves the gate green with a dead reference.

## 11. What an implementor does next

1. Read this file, then [06-entries/PLAN.md](06-entries/PLAN.md), then the one
   `T-R00N.md` that owns your task.
2. Read [recon-b.md](recon-b.md) for the blockers on your wave. Findings 2, 3,
   and 4 in section 6 are in no entry and will stop you.
3. Find your task's row in [recon-c.md](recon-c.md) section 1. Read its `Host`,
   and section 2 for `depends_on`, `writes`, and `conflicts_with`.
4. Take work from your batch. Do not start across batches. Do not start a
   blocked task before its decision.
5. Land with a plant, in the same change. A green check without a failure test
   is insufficient.
6. Run `py scripts/check-todo.py` and read its exit code without a pipe.
7. Record what you could not do and what still owes, in the task entry. Do not
   narrow a failing run until it passes, and do not skip a failing case.

If you find that a task's row is missing, wrong, or ambiguous, that is a
finding: record it against the entry and move to a task you can execute. Do not
invent a decision this file has left open.
