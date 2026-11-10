# Container lanes

This page gives the procedure for the Linux lane and the parked Windows
lane. A cloud session is the Linux lane ([decisions](decisions.md),
2026-11-06). The Windows lane is parked: it stays correct for
wsl-toolkit 9, and no session needs it.

## Select the lane

Run the session entry point from the repository root:

```sh
sh scripts/cloud-setup.sh
cargo build --release -p podvm-gate
./target/x86_64-unknown-linux-musl/release/podvm-dev session
```

It detects the host and selects one lane. Do not select a lane from memory.

| Host | Work location |
| --- | --- |
| Cloud session or Linux host | The checkout. `podvm-dev` starts the environment and build. |
| Linux container | The checkout. Setup runs again because the container can be new. |
| Windows (parked) | Record checks run on the host. Linux builds and tests run in a job container in `wsl-toolkit-podvm`. |

Use `sh scripts/common/check-gate.sh --fast` for an early host check.
Use `--strict` for the final host gate, including the platform twins.
`podvm-dev status` reports the native Linux background build. It does
not report Windows base status.

The `wsl-toolkit` binary owns its command reference. Read its installed
manual before you use a command that this page does not show:

```powershell
wsl-toolkit version
wsl-toolkit man --no-pager
```

The earlier Windows procedure is in
[`history/containers-before-toolkit-6.txt`](history/containers-before-toolkit-6.txt).
It records earlier tool behavior and is not a current instruction.

## Windows base

The rest of this page applies to the parked Windows lane. It targets
wsl-toolkit 9.0.1.

The instance name is `podvm`. The distribution name is
`wsl-toolkit-podvm`. Every manual command must name the instance.

```powershell
wsl-toolkit --instance podvm base status --probe
wsl-toolkit --instance podvm base ensure --probe
```

`base ensure` needs a working host container engine when it creates the
distribution from an image. Record the state of the host engine before you
start it. Restore that state when the session ends. Keep the base: it holds
the image cache and the job records across sessions.

The base status command reports drive access and cgroup support. On the
recorded Windows host, the 2026-09-28 probe reported read-only drive automounts and
no cgroup delegation for the base account. The engine accepted memory and
CPU limits but did not enforce them. Read the current probe before you make
a limit claim on another host. An explicit `base grant --mode rw` can give a
directory write access; the automount setting does not cover that grant.

Never call `wsl.exe` from this project. A direct call can alter another
distribution. Use `wsl-toolkit --instance podvm` for this base.

## Run a Linux job from Windows

```powershell
sh scripts/windows/run-in-base.sh
sh scripts/windows/run-in-base.sh JOB.sh
```

The first command runs the full Linux check. The second runs `JOB.sh`
inside a copied checkout at `/work`. The wrapper sends the job as
`--input job.sh=FILE`; it runs at `/in/job.sh`. Two calls from one checkout
have separate input files.
The wrapper runs a job with Bash when its first line names Bash at
`/usr/bin/env` or `/bin/bash`. It runs other shell jobs with `sh`.
A job that finds the checkout from `$0` must handle `/in/job.sh` and
use the working directory, `/work`, for that form.

The wrapper removes CRLF from the job file before it sends it. The tool
copies other input files byte for byte. The workspace copy restores
executable modes from the git index and from shebangs. A new shell script
must still have its intended mode in git before commit.

The wrapper excludes `codegraph.db`, its sidecars, `daemon.log`,
`daemon.pid`, `target`, `.dev`, and `.tmp`. It keeps `.git` and
`references/`: checks read the index and `references/MANIFEST.tsv`.
Do not exclude every log file; tracked result logs are evidence. The
captured corpus left the tree on 2026-11-06 ([T-1606](../TODO/machine.md)).

`--container-lifecycle ephemeral` removes the container and guest job
directory after the run. The host record goes too (wsl-toolkit 8+); `wait ID`
and `logs ID --follow` answer for 10 minutes after. The 2026-09-28 drive
below, on 6.0.0, found three host job directories after two successful jobs
and one caller path probe, and none after job-specific collection.

A job writes artifacts into `/out`. Set `PODVM_ARTIFACTS` to copy them
back to a host directory. A file written only in `/work` stays in the
copied workspace and is not an output for the host.
The default check exports its transcript, podvm, the four SSH helpers,
and the build record when artifacts are requested.

```powershell
$env:PODVM_ARTIFACTS = 'artifacts'
sh scripts/windows/run-in-base.sh JOB.sh
```

[`experiments/384-windows-lane-v6.sh`](../experiments/384-windows-lane-v6.sh)
drives two jobs against a digest-pinned image. Its
[`result`](../experiments/results/windows-lane-v6.txt) records the tool
version, host, exits, mode check, separate payloads, and cleanup result.

## Inspect and collect jobs

```powershell
wsl-toolkit --instance podvm resources
wsl-toolkit --instance podvm gc --json
wsl-toolkit --instance podvm gc --job JOB_ID --apply
```

`gc` reports what it would remove until `--apply` is present. Collect a
job only after its evidence is saved. `gc --job JOB_ID --apply` is for a
persistent job; an ephemeral lane job leaves nothing to collect and the
command exits 2. Use its ID to leave other jobs alone.
The report includes containers, guest directories, host directories, and
ended base sessions. `crates/podvm-gate/src/main.rs` check 29 reads the JSON
report and refuses a kept item. A running job stays live unless a caller
stops it. `wait ID --timeout D` stops the wait after D; it does not stop
the job.

`gc` removes job resources. It does not shrink the base virtual disk.
`base remove` removes the persistent distribution and its cache. Check
the base state and its work before any removal.

## Host commands and paths

Path conversion may stay on for `scripts/windows/run-in-base.sh`.
wsl-toolkit 8+ also reads MSYS-spelled host paths when
`MSYS_NO_PATHCONV=1` and `MSYS2_ARG_CONV_EXCL='*'` are exported. A command string that names a guest path can
be rewritten by Git Bash before the guest reads it. Use `--script` or
`--command-base64` for a command with paths or quoting.

On the recorded Windows host, `python3` resolves to a Store stub. Run host Python
checks with `py`. Inside a Linux job, use the installed Linux interpreter.
PowerShell can give a generated script CRLF endings. Write an LF script, or
send it through the wrapper's job-file path.

Experiments that need an engine on the Windows host use
[`experiments/lib/engine.sh`](../experiments/lib/engine.sh). It records the
engine, bounds its calls, requires digest-pinned images, and limits mounts.
A job container in the base is not a place to start a second daemon.

## End the session

Read `resources` and `gc --json`. Collect this session's ended jobs by
ID after their results are saved. Keep the base. Stop the host engine only
if this session started it. Run the record gate again and confirm the
repository is clean after commit.
