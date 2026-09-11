# REVIEW 1 — the mechanics sweep: every experiment re-run from clean state

Date: 2026-09-10. Method: `rm -rf work experiments/logs`, then run all
eleven experiments in sequence with their exit codes read unpiped; for
every failure, instrument first, conclude second. This is the "what
happens when a stranger runs the tree" pass.

## Findings, each reproduced before the fix

| # | finding | resolution |
| --- | --- | --- |
| 1 | **`55-` could hang forever.** The driver opened the serial FIFOs blocking and pumped with a blocking `os.read`; any early qemu death left the driver waiting on a FIFO nobody would write — the run hit the outer 10-minute kill with a log that stopped after the host benches. A driver that can block forever does not own the guest's lifecycle, it is hostage to it. | all fifo fds opened `O_NONBLOCK`; the pump `select`s with a 0.1 s tick under a hard per-phase deadline; `write_in` retries `EAGAIN` with a bounded loop |
| 2 | **Readiness was a coin flip.** The nonce echo can be the guest tty's *echo of the typed line*, which proves nothing about the shell being live; the exec line typed before the shell reads it is lost, and the run failed at the begin marker one run in three. | readiness now requires the nonce echo **and then the shell prompt** (`[#$] $`) — the prompt is the shell's own statement that it consumed everything before it |
| 3 | **The prompt regex was wrong**, which is how finding 2 surfaced: the busybox ash prompt is `~ # `, not the `/ # ` first assumed. Found from the failure dump, not from reading code. | prompt regex corrected; failure dumps added to *every* phase so the next mismatch reads its own evidence |
| 4 | **Markers were CRLF-fragile.** The guest tty emits `\r\n`; Python's `$` does not match before `\r`, so `^TAG B$` matched only when the line discipline happened to deliver bare `\n`. Earlier green runs were luck, which is exactly the class of pass this lens exists to catch. | all marker regexes CR-tolerant (`\r?$`) |
| 5 | **A backgrounded qemu outlived its killed parent** — the first hang left an orphan holding fifos and 256 MiB. | the script traps EXIT and kills its qemu on every path |
| 6 | **The end-marker regex escaped its backslashes twice** (`\\r` in a raw string = a literal backslash), found because the begin-marker fix worked and the end-marker fix did not. | corrected; the pair now reads identically |
| 7 | `50-` measured *time-to-timeout*, not time-to-boot: on `pc,acpi=off` the guest halts and qemu never exits, so the wall clock was the kill timer. | the driver owns the lifecycle: poll for the marker, timestamp it, kill qemu; the log now says `time-to-marker` |
| 8 | Minor: `census.c`'s whiteout fixture survived a successful probe and poisoned the next run with `EEXIST` (a fixture left behind is not evidence, it is a trap); the first census draft printed the parent's errno (`0`) for the child's syscalls. | fixtures cleaned on success; the child prints its own verdict |

## Gate

All eleven experiments re-run clean after the fixes: ten rc=0, `55-`
three consecutive rc=0 before the sweep was closed. No qemu orphans
left (`pgrep` clean).

## What would have made this pass fire sooner

A gate that runs the tree twice in a row from clean state would have
caught findings 2 and 4 (both are flake-or-luck classes) without the
manual sweep. The experiment README now asks for one immediate re-run
of any experiment that was touched, before its log is trusted.
