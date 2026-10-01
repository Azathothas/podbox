# Plan: a self-contained brief for any agent picking up this refactor

Paste this into a fresh session, or read it end to end before touching the
repository. It is written so that an agent with no prior context, in a new
context window, on a Windows machine, can orient itself and start the first
task correctly.

---

## Your task in one paragraph

You are retiring 121 shell, Python, and PowerShell scripts from the `podbox`
repository by re-expressing their measurements as native Rust. The work is
already planned and audited across seven rounds. Your job is to execute it, one
task at a time, in the order this brief gives, and to prove each one.

podbox is a Linux container runtime that runs payloads where namespaces, mounts,
devices, and ownership changes can be denied. It also has a machine tier and an
SSH transport. It measures operations and reports the mechanism it enters.

The repository is at `C:\Users\AjamX\Downloads\podbox`. You are on Windows with
Git Bash as your shell.

---

## Start here, in this order

Read all five before writing anything.

1. **[INDEX.md](INDEX.md)** : the consolidated plan. It states the goal, the
   host findings, the corrections, the open decisions, the 100 tasks, and the
   batch structure. **This is your orientation.** If you read only one file,
   read this one.
2. **[06-entries/PLAN.md](06-entries/PLAN.md)** : the master plan from the
   audit. It carries the citations, the verdict distribution over the 121
   scripts, the shape of the target (four new crates, not eighteen), and the
   six waves.
3. **[TODO/RULES.md](../TODO/RULES.md)** and **[TODO/INDEX.md](../TODO/INDEX.md)**:
   the repository's own record rules and its entry format. The index's category
   table at roughly lines 42 to 57 is the authority on which crate owns a
   subject. The highest existing task id is T-1423, so new work starts at
   T-1501.
4. **[AGENTS.md](../AGENTS.md)** and **[docs/conventions/prose.md](../docs/conventions/prose.md)**:
   the repository rules and the prose rules.
5. **[recon-c.md](recon-c.md)** : the 100-task decomposition with its dependency
   graph. Section 1 is the task table, section 2 the dependencies. This is the
   file you take work from.

Two more, when your task needs them: **[recon-b.md](recon-b.md)** for the
blockers on your wave, and **[docs/methodology/gate.md](../docs/methodology/gate.md)**
for what the gate checks.

---

## The rules that will cost you if you break them

- **Never call `wsl.exe` with a payload argument.** [docs/containers.md](../docs/containers.md)
  carries the reason: a payload handed to `wsl.exe` is expanded before the guest
  reads it, and the guest parses the result a second time. Use `wsl-toolkit
  --instance podbox` or the `wslc` path below.
- **Run `py scripts/check-todo.py` before and after every change.** It is the
  record gate. It is currently green. A red gate is a stop, not a warning.
- **Read every exit code without a pipe.** A command piped into another command
  reports the pipe's status, so a failure reads as success. ⛔ **This applies
  inside your job script too.** A job ending in `cargo test ... | tail -8`
  reports `tail`'s status, and you will read `rc=0` over a red suite. Capture to
  a file and read the cargo exit code, or drop the pipe.
- **Land a plant with every new check, in the same change.** A green check with
  no failure test is insufficient.
- **Do not narrow a failing run until it passes.** No skipping the failing case,
  no deleting the test, no weakening the assertion.
- **Never commit unless the operator asks.** Do not run `git add` or
  `git commit`.
- **Prose: ASCII, no em dash.** The markers `⛔`, `⭐`, `⚠`, `✅`, `❌` have fixed
  meanings.
- **Record an unknown value as a dash.** Label an estimate each time it appears.

---

## What you can and cannot do on this host

This is measured, not assumed. Measurements dated 2026-10-01.

| you want to | can you, natively |
| --- | --- |
| write Rust or markdown | **yes**, always |
| `cargo check` for the host target | **yes**, exit 0 |
| `cargo check --target x86_64-unknown-linux-gnu` | **yes** for `podbox-probe`, `podbox-enter`, `podbox-windows` |
| `cargo test` | **no**, `linker 'cc' not found` |
| build for a Windows target | **no**, the source is Unix-only at the type level |

The last row is the important one. `cargo check --target x86_64-pc-windows-gnu`
fails with `E0433: cannot find unix in os` and `E0599: no method named
pre_exec` at `crates/podbox-probe/src/sys.rs:815`. That is a **typecheck**
failure, so no linker change moves it. Eight of nine crates use Unix-only APIs;
`podbox-supervise` is the only one a single `mod` could gate, and its one file
is `src/launcher.rs`, which everything depends on.

So: **authoring is always native, proving needs a Linux container.** A pure
unit test can be written on Windows and proven on a lane. That is the normal
case.

### The lane: one command, use it

**Run every proof through [lane.sh](lane.sh).** Do not call `wsl-toolkit` by
hand, and do not call `wslc` at all.

```sh
sh refactor/lane.sh JOB.sh              # the job, with the toolset
sh refactor/lane.sh JOB.sh --user       # the job, as the unprivileged account
sh refactor/lane.sh --check             # is the host ready
```

⛔ **Run it from Git Bash.** The script is the Windows half of a job and calls
`wsl-toolkit` from a shell that does not rewrite guest paths. MSYS turns
`--dir /workspaces/x` into `C:/Program Files/Git/workspaces/x` and the tool
refuses the rewrite.

⛔ **Never call `wsl.exe` with a payload argument** (`docs/containers.md`), and
⛔ **never `wsl --shutdown`**, which breaks every WSL instance on the machine.

Seven traps, all found by running them. `lane.sh` closes all seven:

1. The bare job image has **no `zig` and no `jq`**, so any crate pulling `ring`
   fails at `cc-rs`. Only `podbox-probe` is exempt; `podbox-extract` is not,
   because `Cargo.toml:18` pulls `podbox-image`.
2. The bootstrap **cannot run as non-root**, because it pins
   `ZIG_PREFIX=/opt/zig` and is root's. `lane.sh` bootstraps as root, lays down
   one passwordless sudo rule, then drops. **Measured: uid 1000,
   `zig 0.16.0`, `GATE_OK`, 51 tests passed.**
3. The copy is **root-owned**, so non-root cannot write it and git reports
   `fatal: detected dubious ownership`. The runner chowns the tree to the
   account, because a job that cannot write the code it tests proves nothing.
4. `codegraph.db` is excluded **by name**, never the `.codegraph` directory.
5. `PATH` is **prepended**, never replaced, or `cargo: not found` and exit 127.
6. The privilege drop is a **script file**, not a nested quoted string. The
   quoted form died silently, exit 2, with nothing on either stream.
7. The drop is **not `exec`**, or the job's exit code is lost.

Measured on 2026-10-01: the default path ran the acceptance job in 44.1 s with
`GATE_OK` and 51 tests passed; `--user` ran it in 56.3 s at uid 1000 with
passwordless sudo and the same result; 4 concurrent jobs all exited 0 in 28 to
29 s each and left the record gate green.

**What the tool is for, in one line:** it copies the tree into a disposable
Linux container, installs the toolset, runs your job as root or as the
unprivileged account, then removes the retained job so the record gate stays
green for the next agent.

If you must call the tool by hand: read `wsl-toolkit man --no-pager`, not
`--help`. A `--help` list was truncated on this machine once and cost a wrong
conclusion about `run --user`, which does exist. Read `$LASTEXITCODE` from
PowerShell. Never add `--exclude` for `references`, `experiments`, `.git`, or
`crates`; each turns the record gate red.

⛔ **`wslc` is not the alternative.** Microsoft ships it at
`C:\Program Files\WSL\wslc.exe`, and it measured the same speed, cannot take
a copy, cannot help because all 53 lane tasks write a tracked path, and is
**wedged**: a killed job locks the shared image store, so every later call
fails with `ERROR_SHARING_VIOLATION` and **one dead job blocks every other
agent**. `wslc system session terminate` and `wsl.exe --shutdown` were both
tried and neither worked.

⛔ **A missing `sshd` is not a uid problem.** The tests in
`crates/podbox-ssh/tests/` drive a real `sshd`, and
`crates/podbox-ssh/tests/common.rs:41` **panics** without it. The job image has
`ssh` but not `sshd`. Install `openssh-server` in the job or run those tests in
CI. Do not rewrite the test.


## What this refactor is for

The operator has fixed the end state. This work is one stage of reaching it,
and you should know all six points so your task lands work that moves toward
them. Full detail is [INDEX.md](INDEX.md) section 12.

1. **`experiments/` is deleted entirely.** Usable scripts move to `scripts/`,
   rewritten and improved. The rest become Rust. ⛔ **27 KEEP-SHELL scripts have
   no home in the plan**; they must move, and that is decision D-10.
2. **`refactor/` is deleted** when the work is done. Everything these documents
   assert must be carried into `TODO/` first, or the reasoning is lost.
3. **`references/` is deleted.** 46 repositories, 168 MB. ⛔ The record gate
   reads it, so this cannot be done until every citation is repointed.
4. **`vendor/userland-execve` is rewritten as our own crate.** 512 lines, MIT,
   currently unlinked and outside the workspace. Small, bounded, and it goes
   early.
5. **`podbox-ssh` is purged** into `podbox remote ssh`, `podbox local ssh`, and
   `podbox machine ssh`. ⛔ **`remote ssh` and `machine ssh` already exist**;
   `local ssh` does not.
6. **GitHub issues are mined** and the whole `TODO/INDEX.md` and its entries are
   rewritten toward closing them. This comes last, because it re-plans the work
   this file carries.

⛔ **No wave in the current plan serves points 3, 4, or 5.** They are named so
the gap is visible. The next planning round files them.

## What is already done, and what is not

Done: 203 `TODO/` entries, 0 open, 2 partial, 201 done. The record gate is
green. Six of the 121 scripts are already fully converted to Rust and are
ready to delete in wave 1.

Not done: everything in the five batches below. The plan is not started.

⛔ **There are seven waves, not six.** `T-R006.md` carries the twelve DELETE
scripts and `PLAN.md` section 8 does not list it. Read INDEX.md section 6.

⛔ **Three blockers are in no entry and will stop you.** They are findings 2, 3,
and 4 in INDEX.md section 6: the two size ceilings die with the files that
declare them, wave 4's replacement `Prove` cannot run in the job that calls it,
and the `110` script is read by files wave 4 deletes while wave 5 moves it.
**Do not start wave 4 before Batch 0 lands.**

---

## Eight decisions are open. You may not invent them.

INDEX.md section 7 states each with its options and costs. D-1, the two size
ceilings. D-2, the `110` hoist. D-3, wave 2's plant mechanism, which blocks 13
tasks. D-4, wave 1's `Prove`. D-5, the `podbox-cli` test shape, which blocks 4
agents. D-6, who writes the performance ceilings. D-7, the 26 unassigned
scripts. D-8, whether `T-R006` is authorised. A ninth, naming the ninth
root-blocked task, was **withdrawn**: measurement showed the lane's root lacks
`CAP_MKNOD`, so no task is blocked on uid.

**If your task is blocked on a decision, you are blocked.** Move to another task
and record what you found. A decision you made up and wrote down is a defect,
not progress.

---

## The five batches

A wave is not a unit of independence. Wave 4 alone is eight binaries sharing a
crate, a grammar module, a `Cargo.toml` row, a workflow, and six `TODO/` files.
Take work by batch.

| batch | agents | tasks | what |
| --- | --- | --- | --- |
| **0** | 1, serial | 3 | the decisions and the cut set. **Nothing starts before this lands.** |
| **1** | 8 | 28 | the 14 wave-0 record corrections, the deletions, the wave-2 unit tests |
| **2** | 1, serial | 8 | `crates/podbox-gate`. **The serial core. Cannot be split.** |
| **3** | 4 | 11 | `podbox-buildstate`, `podbox-release`, `podbox-podvm` |
| **4** | 7 | 14 | the integration tests and the registry fixture. **Can start before batch 2.** |

Batch 0 is T-1581, T-1558, T-1551. Batch 2 is T-1552 through T-1557 plus T-1577
and T-1578. **If you want one session's worth of shippable work, Batch 4 alone
is a complete unit**: 14 tasks, 7 agents, no dependency on the gate crate, and it
closes the missing OCI registry fixture that `PLAN.md` section 6 names as
blocking `crates/podbox-image/tests/store_digest.rs`.

⛔ **41 of the 100 tasks write a `TODO/` file** and must serialise behind one
writer. The file that serialises is `TODO/INDEX.md`, because
`scripts/todo-count.py` rewrites one Counts block from all 203 rows and
`check_tree` then reads the whole tree. Two agents each editing a different
`TODO/` file are both green and the merge is wrong. Within Batch 1, 5 of the 8
agents write no `TODO/` file and run fully parallel; the rest queue. Peak fleet
is 19 on a clean tree, during the wave-0 and wave-6 window only. Sustainable
concurrency after the record-gate and workflow serialisations is 6.

⛔ **22 of the 100 tasks are in no batch**: T-1550, T-1571, and T-1582 to
T-1600. Most are decision records belonging to whichever wave implements them.
**Do not take a task that no batch names.** Take one from a named batch, or file
the missing batch in INDEX.md first.

---

## How to take one task

1. Find your task's row in [recon-c.md](recon-c.md) section 1. Note its id, its
   one-line item, and its `Host` value. **Use the row's `Host`, not INDEX.md
   section 8's totals**, which are a shape and not a dispatch instruction.
2. Read section 2 of the same file for `depends_on`, `writes`, and
   `conflicts_with`. If anything in `depends_on` is open, you are blocked.
3. Read the `T-R00N.md` that is the task's parent, in full.
4. Read [recon-b.md](recon-b.md) and look up your wave. Six of its findings are
   `FALSE` claims in the plan, and four are `UNRESOLVED-DECISION` that map to D-3
   through D-6 above.
5. Read the current source. A previous record is a claim to verify, not a fact.
6. Do the work.
7. Land it with its plant.
8. Run the task's proof. `py scripts/check-todo.py` is native; `cargo test` is a
   lane; the interposer has its own command, `cargo test --manifest-path
   crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`,
   because `cargo test --workspace` skips that crate at `Cargo.toml:20`.
9. Update the task entry, `TODO/INDEX.md`, and the progress record with the
   change. Use `scripts/todo-count.py` to derive counts; do not edit them by
   hand.
10. Record what you could not do and what still owes.

---

## Two traps that produce a green gate with a broken reference

`scripts/check-todo.py` treats a **bare** `experiments/...` citation as a
tracked-file reference and turns red when the file is gone. A `sh
experiments/...` or `./experiments/...` citation is **not** caught in either
direction. So deleting a script and leaving a `sh`-prefixed citation behind
leaves the gate green with a dead reference.

`scripts/plant.sh` is named in 37 places under `TODO/` and 10 under `docs/`,
and 11 counting `docs/` and `.github/` together, of which 13 are `Prove` lines
in 2 files, `TODO/gate.md` and `TODO/deps.md`. Not 13 lines is the whole repair
set. Grep both citation forms before and after every deletion.

## Four lane rules you apply without judgement

Full form in [INDEX.md](INDEX.md) section 5.4. These four are the ones that bite.

- ⛔ **Collect the job by its full 16-hex id before running the record gate.**
  `wsl-toolkit --instance podbox gc --job JOB_ID --apply` needs it. The
  12-character form printed in the log header **exits 0 and removes nothing**,
  which is a silent failure. Get the id from the `--json` answer, which prints
  `wtk-<16hex> (<12hex>)`. The whole lot is
  `wsl-toolkit --instance podbox gc --apply`.
- ⛔ **Never kill a lane job.** Use `wsl-toolkit --instance podbox stop JOB_ID`.
  Killing the client leaves an open record only `gc` clears.
- ⛔ **Never add `--exclude`** for `references`, `experiments`, `.git`, or
  `crates`. Each turns the record gate red.
- ⛔ **A neighbour's dead job fails your landing proof.** 20 leftover jobs
  produced 104 `check-todo` problems, because `check-todo.py:1339` emits one
  error per retained item with no scoping. In a fleet, `gc` **before** you run
  the gate, not after.

---

## If you cannot finish

Say which part, name the blocker, and hand back what runs. A partial with a
precise remainder is a result; a silent narrowing is a defect. Do not mark work
complete while tests are red or a task is partial.

Do not commit. Do not push. The operator owns the repository's history.
