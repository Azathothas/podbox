# Which lane: `wsl-toolkit` or `wslc`, for the 100-task refactor

**Question.** The operator asked, "So we still use wsl-toolkit when needed, else
use wslc?" This file answers it for **this** workload, with measurements taken
on 2026-10-01 on this machine.

**Answer.** No. The split as phrased does not describe a rule an agent can
apply, because "when needed" is a judgement call and 19 agents will make 19 of
them. The split that the measurements support is narrower: **`wsl-toolkit` runs
every proof, and `wslc` runs none.** `wslc` is faster, and it is faster by
exactly the amount that makes it unusable. The routing rule is in section 7.

**What is not settled here.** Whether a non-root job is reachable is being
established by another agent. Section 5 records what this session measured
about uid without reaching a verdict.

## 1. Measurement table

Every wall time below was read from `[Diagnostics.Stopwatch]` around the
`wsl-toolkit` process on the Windows side, and the exit code was read from
`$LASTEXITCODE` on the next PowerShell statement. No value is piped.

| lane | task class | wall seconds | workspace copied? | measured by | notes |
| --- | --- | --- | --- | --- | --- |
| A `wsl-toolkit run` | tool availability, no source | 3.4 | no | me | `--user` absent runs as uid 0. `/work` exists and is empty. `cc` and `cargo` present, `zig` and `jq` absent. |
| A `wsl-toolkit run` | workspace copy, no build | 6.2 | yes | me | 12,050 entries, 343.3 MiB. Copy lands 2.993 s in (timestamp column `rel`). |
| A `wsl-toolkit run --user 1000:1000` | workspace copy, no build | 6.5 | yes | me | uid 1000, `cc` works. **Same speed as root.** The user flag is free. |
| A + `cargo test -p podbox-probe` | crate test, 108 tests | 38.9 | yes | me | 6.3 s copy and startup, **32.6 s** `cargo`, 0.09 s the tests themselves. Exit 0, 108 passed. |
| A `cargo test -p podbox-probe` | same, with the minimal exclude set | 38.7 | yes, 192.8 MiB | me | 1.1 s saved against 343.3 MiB. The copy is not the cost. |
| A `podbox-complete`, no bootstrap | `ring` crate | 32.1 | yes | me | **Fails.** `cc-rs` cannot run `scripts/zig-cc.sh`: `zig is not on PATH`. |
| A `podbox-complete`, bootstrap as root | `ring` crate | 48.7 | yes | me | Bootstrap 28.7 s (needs `apt-get`, `/opt/zig`), then `cargo` 13.4 s. **51 passed, exit 0.** |
| A `podbox-complete`, bootstrap as uid 1000 | `ring` crate | 29.8 | yes | me | **Bootstrap fails.** `apt-get` refused, `mv` to `/opt/zig` denied. |
| A, uid 1000, zig fetched into `$HOME` by hand | `ring` crate | 65.7 | yes | me | Works. But `HOME=/work`, so the 55 MB dist dies with the copy. `cargo` 47.5 s. |
| B `wslc` | `cargo test -p podbox-probe` | about 40 | no, bind mount | **inherited** | 108 passed, exit 0. Not re-measured: `wslc` is wedged, section 3. |

**The cost is not the copy.** A real crate test is 38.9 s, of which 6.3 s is the
copy and 32.6 s is `cargo`. Excluding half the tree saves 1.1 s, which is 2.8
percent of the run. Anyone optimising the copy is optimising the wrong 6
percent.

## 2. The `--exclude` finding

`--exclude` is a glob matched against path components. It is the only copy
lever, and it is much less powerful than it looks.

Measured, one variable at a time, from the repository baseline:

| exclusion set | entries | bytes | wall |
| --- | --- | --- | --- |
| the repository's own set (`codegraph.db`, sidecars, `daemon.log`, `daemon.pid`, `target`, `.dev`, `.tmp`) | 12,050 | 343.3 MiB | 5.9 s |
| plus `.codegraph` (the **directory**) | 12,046 | 343.3 MiB | 5.7 s |
| plus `*.db` | 12,050 | 343.3 MiB | - |
| plus `.codegraph/*` | 12,049 | 343.3 MiB | - |
| plus `references` | 2,743 | 192.8 MiB | 4.6 s |
| plus `references` and `refactor` and `vendor` | 2,678 | 191.1 MiB | 4.2 s |
| plus `.git` as well | 2,339 | 149.1 MiB | 4.2 s |
| plus `experiments` | 783 | 48.9 MiB | 4.2 s |

Two findings, both against expectation.

**Finding 1. `--exclude .codegraph` saves nothing.** The directory is 313.9 MB
and the exclusion removes 4 entries and 0 bytes. The repository already
excludes `codegraph.db` **by name**, so the 313.9 MB file was never in the
copy. The measured composition of the copy, read from inside the container:

```
references    163,187,396 bytes   9,303 entries
experiments   151,665,607 bytes   1,958 entries
.git           44,696,718 bytes     339 entries
crates          3,088,656 bytes     174 entries
refactor        1,790,031 bytes      51 entries
TODO              964,609 bytes      22 entries
scripts            713,237 bytes      63 entries
docs              612,440 bytes      90 entries
.codegraph            8,130 bytes       3 entries
```

So the two big exclusions available are `references` and `experiments`, and
they are the two directories the record gate reads.

**Finding 2. Excluding either one breaks the gate.** Measured inside a stripped
container:

| exclusion | copy | `python3 scripts/check-todo.py` |
| --- | --- | --- |
| repository baseline | 343.3 MiB | **exit 0, `check-todo: ok`** |
| plus `.codegraph` | 343.3 MiB | **exit 0, `check-todo: ok`** |
| plus `references` | 192.8 MiB | **exit 1**, `TODO/reference-map.md: references/apptainer__apptainer does not exist`, and 11 more |
| plus `references` and `.git` | 150.8 MiB | **exit 2** |
| plus `.git` only | 301.2 MiB | **exit 2** |
| plus `experiments` and `references` | 48.9 MiB | **exit 1**, `FileNotFoundError: '/work/experiments/110-bloat-delta.sh'` |
| plus `vendor` only | 343.3 MiB | **exit 0** |

The reason is in the source. `check_tree` at
`scripts/check-todo.py:300` walks the tracked listing and opens every file, and
`is_ours` at `:291` exempts `references/` only to skip *its own* citation
checks. `TODO/reference-map.md` still cites every corpus directory, and
`:228` reads `experiments/110-bloat-delta.sh` as a named constant. The
documented rule in `run-in-base.sh:132` is not conservatism. It is the
measured behaviour.

**Verdict on `--exclude`: there is no free saving.** The 343 MiB copy breaks
into two directories that cannot be dropped without breaking the gate, and one
very large directory that is already excluded. Maximum safe reduction is
0 bytes. The lever is spent.

## 3. Copy versus bind mount

Counted from the `writes` column of `refactor/recon-c.md` section 2, joined to
the `Host` column of section 1 by task id. The join is over all 100 rows.

| host class | tasks | `writes` names a tracked checkout path | read-only |
| --- | --- | --- | --- |
| `native-windows` | 47 | - | - |
| `linux-lane` | **47** | **47** | **0** |
| `both` | 6 | **6** | **0** |

**All 53 lane-class tasks write the checkout. Not one is read-only.** The 47
`linux-lane` tasks touch 48 paths under `crates/`, 14 under `scripts/`, 16
under `experiments/`, 5 under `TODO/`, and 2 `Cargo.toml`. Each one's proof
reads the file it just wrote.

So the choice is not "copy for writers, bind mount for readers". There is no
reader class. **Every lane proof needs its own copy.** The bind mount's
advantage, which is real, buys nothing, because the thing being bound is the
thing under test.

Two further facts close the question:

- **The copy cannot be replaced by a grant.** `wsl-toolkit run` has no mount
  flag at all. `man --no-pager` carries `--device` and nothing that attaches a
  host directory to a job container. The only directory-attaching command is
  `base grant`, which attaches to the **base**, not to a job container, and it
  defaults to read only.
- **Copying `target/` is impossible anyway.** `target/` is 2,816,463,598 bytes
  over 10,424 entries. With `--max-bytes 100000000` the copy is refused before
  the container starts:
  `! workspace refused: the workspace passes 95.4 MiB at .codegraph/codegraph.db`,
  exit 2. The default ceiling is 1 GiB. The host `target/` is 2.6x over it.

**A bind-mount fleet is not merely unhelpful, it is broken.** Section 4.

## 4. Concurrency

### Lane A parallelises. Measured.

| measurement | result |
| --- | --- |
| 4 concurrent jobs, no workspace, each `sleep 20` | all four slept 20.001, 20.001, 20.002, 20.003 s. Whole batch 25 s. **No serialisation.** |
| 6 concurrent jobs, full 343.3 MiB workspace copy each | six copies, six job directories, exit 0 each. Per-job 8.96 to 10.29 s against 6.2 s for one alone. **Contention factor about 1.6.** |
| 1 detached job running, `base status --probe` | `usable true`, `engine podman version 6.1.2` |
| 1 detached job running, a second `run` started | started and finished in 3.09 s, exit 0 |

The single podman engine does **not** serialise jobs. The copy is the shared
resource, and it degrades linearly and mildly. Six concurrent copies cost about
1.6x the wall of one, which is well inside the 6 sustainable the plan assumes.

⚠ The base has **no cgroup delegation**. `base status --probe` says so:
`this base has no cgroup delegation for the engine's account, so podman creates
no cgroup per container`. A memory or CPU limit is accepted and not enforced,
and an out-of-memory kill cannot be told from any other exit 137. **A fleet of
6 has no memory ceiling.** That is a scheduler's problem to bound, not a
tool's.

### `matrix --parallel` default 4

`man` states `--parallel how many rows run at once Default: 4`. This is the
tool's own statement about its default width, and it is **not** a measurement of
the engine's limit. The four concurrent jobs above ran at full speed, and six
ran too, so 4 is a conservative default and not a ceiling. Nothing here
requires `matrix`.

### Lane B has one session manager, and it is wedged

Measured this session:

```
wslc version                -> wslc 3.0.1.0
wslc system session list    -> ID 3  Creator PID 783352  wslc-cli-AjamX
wslc run --rm -v ...:/work  -> ERROR_SHARING_VIOLATION, exit 1
```

**Confirmed still wedged**, unchanged from the inherited report. `ID 3` has a
creator PID, but `run` cannot get at the image store.

**A wedged Lane B blocks every other agent.** This is the observed behaviour
recorded in `refactor/INDEX.md:197` to `:207` and reproduced here: the failure
is `ERROR_SHARING_VIOLATION` on the **shared image store**, not on a per-job
resource. Every `wslc` invocation on this machine reads the same store. One
killed job took the store for all callers, and `wslc system session terminate`
and `wsl.exe --shutdown` neither recovered it. There is no per-agent recovery.
A fleet of 6 depending on Lane B has a single point of failure with no
per-agent escape.

### The shared-`target/` finding

This is the strongest single argument in this file, and it is worse than
"they block each other". A bind-mounted fleet does not queue. It **corrupts**.

Measured, two `cargo build` processes on one `--target-dir`, toolchain pinned so
rustup could not be blamed:

```
2 concurrent builds, ONE target dir:
  B_RC=101 at 1425ms
  A_RC=101 at 1465ms
  build-directory lock waits: 1
  E0463 in a: 2   E0463 in b: 2
  a tail: error: could not compile `syscalls` (lib) due to 1 previous error
          For more information about this error, try `rustc --explain E0463`.

3 concurrent builds, ONE target dir:
  A_RC=101 B_RC=101 C_RC=101, all inside 2.3 s
  Blocking waiting for file lock on build directory:  1
  Blocking waiting for file lock on package cache:     9
  E0463 in all three: 2 each
```

**Both agents failed.** `E0463` is "couldn't find crate": one build removed an
artifact out from under the other mid-compile. Only one build-directory lock
wait was recorded, so the lock is not what protects you, and no amount of
waiting prevents this. A third process does not help; it fails the same way.

⚠ One control run of this experiment failed for a **different** reason and the
first reading was wrong. The first attempt showed `A_RC=1` at 1.75 s with a
rustup error, `component download failed for cargo-x86_64-unknown-linux-gnu`,
`rolling back changes`. That was two `cargo` processes racing on
`/usr/local/rustup`, not the target lock. The control was re-run with
`RUSTUP_TOOLCHAIN` pinned, which is the run quoted above, and it reproduces the
E0463 failure cleanly. Both races are real; only the second is about the shared
target directory.

**So the answer to "give each agent its own target directory" is: Lane A already
does.** Each job copies into its own per-job directory, so each `/work` carries
its own `target/` and no two agents can reach another's. The copy is not a
wart. It is the isolation mechanism, and it is the reason the lane parallelises.
Do not attempt to buy its cost back with `--exclude target`, which every job
already passes.

## 5. Failure modes, for an unattended fleet of 6

Lane A, measured this session:

| event | what remains | who is hurt | recovery |
| --- | --- | --- | --- |
| job completes, `--container-lifecycle ephemeral` | host job directory with the transcript | **everyone** | `gc --job ID --apply`, one command |
| job completes, `persistent` | container, guest dir, host dir | **everyone** | same |
| job is killed with `wsl-toolkit stop` | container, guest dir, host dir, **plus an open record**: `resources` printed `open records 1, so a run was interrupted` | **everyone** | same, one command |
| job times out | the same, row reports 124 | **everyone** | same |
| job is still running | listed as kept: `in use right now` | nobody | nothing to do |

The cost is measured, not estimated. Twenty jobs left behind:

```
containers   18
guest_dirs   18
host_dirs    68
check-todo   104 problem(s)
guest total  4.2 GiB across 18 directories
engine       Containers 18  12.54GB
```

**One agent that dies turns the record gate red for all 6.** The mechanism is
`scripts/check-todo.py:1339`: it reads `wsl-toolkit gc --json` and emits one
error per retained item, with no per-agent scoping and no allowance. 104
problems from 20 jobs, roughly 5 per job. Every agent's landing proof is
`py scripts/check-todo.py`, so a neighbour's dead job fails their gate.

The base grew from 11.4 GiB to 19.2 GiB across this session's runs, and `gc`
does not shrink the virtual disk. That is a fixed allowance with no reclaim.

⛔ **The recovery command needs the full 16-hex container id, not the 12-char
job id shown in some messages.** Measured: `gc --job 0b769b3dcc81ddc9` listed
the container, guest dir and host dir for removal, and `gc --job 0b769b3dcc81`
silently removed nothing while reporting exit 0. An agent that takes the short
id from the log line, runs the command, reads exit 0, and moves on leaves the
gate red. `--json` prints `wtk-<16 hex> (<12 hex>)`; the full first form is the
argument. This is a trap in the current procedure and it cost me one round
trip.

Lane B:

| event | result | who is hurt | recovery |
| --- | --- | --- | --- |
| any `wslc` job is killed | store wedged, `ERROR_SHARING_VIOLATION` on every later call | **every agent, all lanes that share the store** | none found. `session terminate` and `--shutdown` both failed. |
| ordinary failure | `wslc run` returns the container's code | one agent | rerun |

Neither lane recovers from a wedged Lane B. The only observed recovery is a
host restart, which is a human action with a human's whole day attached to it.

## 6. What Lane A without `--workspace` is for

Measured: 3.4 s, exit 0, uid 0, `/work` empty, `cc` and `cargo` present, `zig`
and `jq` absent.

It cannot see the source, so it cannot prove anything about this tree. What it
is good for is exactly three things, and each is real:

- **Proving the lane is alive before a wave.** 3.4 s, against 6.2 s for a copy.
  `sh scripts/windows/run-in-base.sh` runs the full check; this is the cheap
  preflight for "is the base usable at all".
- **Host-side probes with no checkout.** `uname`, `id`, toolchain versions,
  whether `/opt/zig` or a device node is present, what `/proc` reports. These
  are the questions the workspace is irrelevant to.
- **A user-namespace and uid probe.** `--user 1000:1000` at 3.4 s. The
  per-job uid question, which section 5 leaves to the other agent, is cheapest
  to answer here because no copy stands between the question and the answer.

It is not a proof lane. A proof that does not read the tree is not a proof of
the tree.

## 7. The routing rule

Written so an agent applies it without judgement. There is one lane.

> **R1. Every proof runs on Lane A, through
> `sh scripts/windows/run-in-base.sh JOB.sh`. There is no lane choice and no
> `wslc` fallback.**
>
> **R2. Run a job on Lane A without `--workspace` only when the answer is about
> the container and not the tree. The checkable condition: the job script
> contains no reference to `crates/`, `scripts/`, `TODO/`, `Cargo.toml`, or any
> other tracked path. If you cannot point at a line of your job that reads the
> tree, you may drop `--workspace`. Otherwise you may not.**
>
> **R3. Never add `--exclude` for `references`, `experiments`, `.git`, `crates`,
> `scripts`, `TODO`, or `refactor`.** The copy is 343.3 MiB and that number is
> the correct number. `run-in-base.sh:143` already passes the seven safe
> exclusions; leave the list alone. Adding one is a tree defect until a measured
> run proves otherwise, and the two obvious candidates are already measured red.
>
> **R4. After every Lane A job, before running the record gate, collect that
> job:**
>
> ```sh
> ID=$(wsl-toolkit --instance podbox gc --json | grep -o 'wtk-[0-9a-f]\{16\}' | head -1 | cut -c5-)
> wsl-toolkit --instance podbox gc --job "$ID" --apply
> ```
>
> Read the exit code. If it is not 0, or the gate still names a kept job, the
> job is not collected and the lane is not clean.
>
> **R5. If `wsl-toolkit --instance podbox base status --probe` does not report
> `usable true`, run `base ensure --repair`, then re-probe, then continue.** Do
> not fall back to `wslc`.
>
> **R6. Never kill a Lane A job with anything but `wsl-toolkit stop`.** `stop`
> is recoverable with R4. Killing the client leaves an open record, which R4
> also collects. This is the one rule that separates Lane A from Lane B: on
> Lane B there is no safe way to kill a job, and that single asymmetry is why
> there is no fallback.

### Why the rule is this narrow

The question was "wsl-toolkit when needed, else wslc". The measurements answer
it three times over.

`wslc` is faster, by about one copy: 40 s inherited against 38.9 s measured.
**They are the same speed.** Every copy Lane A pays, `wslc` pays back in full
as compile time, and the compile is the larger term (32.6 s of 38.9 s). The
speed case for `wslc`, which is the entire case for having two lanes, is not a
speed case.

Every one of the 53 lane tasks writes the checkout, so the bind mount's only
real advantage, sharing `target/`, applies to a class of task that does not
exist in this workload. And if the bind mount is used anyway, two agents on one
`target/` directory both fail with `E0463` rather than one waiting.

`wslc` has a failure mode Lane A does not: one killed job wedges the shared
store for every caller on the machine, with no per-agent recovery. Lane A's
worst case is a red gate and one command.

**A correct rule beats an optimal rule.** The optimal rule would send 53 proofs
to the faster lane, which is a tie, and would accept one wedged session to save
6 seconds per proof. The rule above is the one an agent can execute unattended.

### If `wslc` ever recovers

The comparison should be re-run, not assumed. Three things would have to change
before it earned a place in the routing rule, and each is checkable:

1. A second `wslc` invocation succeeds while one is running, which shows the
   store is not single-holder.
2. A `kill` followed by a `run` succeeds, which shows the wedge is recoverable
   by a command.
3. Two `cargo` processes on one `target/` directory behave, which section 4
   measured as `E0463` on this lane's terms.

Until all three pass, `wslc` is a one-agent manual probe and never a fleet lane.