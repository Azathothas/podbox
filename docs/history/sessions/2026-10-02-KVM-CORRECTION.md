# Session summary: KVM correction, T-1604 tier skip, CI interposer

Session 2026-10-02, third pass, worked unattended. Started from
`TODO/RESUME.md` at `fd8001f` on `main`.

## What this session found

The work order named three defects in T-1641. Two of the three causes
were read in source and found to be wrong, and one rested on evidence
that is filed nowhere. The defects were real; the stated mechanisms were
not. The record now carries the correction and the refutations, with the
file and line that settle each one.

| Claim | Verdict | Evidence |
| --- | --- | --- |
| `overlay_argv` passes no read-only backing | refuted | `crates/podbox-windows/src/plan.rs:195-207` passes `-F` and `-b` |
| `setup` writes into the pinned VHDX | refuted | every write targets `mailbox`, `root`, or `monitor`; `base` reaches only `overlay_argv`'s `-b`, and there is no `commit`, `map`, or `convert` in the crate |
| `lib.rs` discards the reaped child's exit status | confirmed | `lib.rs:241`, `:244` |
| nothing verifies the emulator died | confirmed | `lib.rs:239-243`, `:337-338` |
| the orphan is unselectable by proof and watchdog | refuted | `experiments/lib/kvm-owned.sh:5-6` selects on `ps` text; `scripts/windows/kvm-watchdog.py:108-121` on `/proc/PID/exe` and `cmdline`; neither reads the parent |
| the seam swallows a non-zero exit | refuted | `experiments/lib/kvm-guest-base.sh:191` calls `miss`, which sets `fail=1` at `:21` |
| `scode=$?` is dead | confirmed | `experiments/lib/kvm-guest-base.sh:183`; `$?` inside `then` is the status of the `if` test |
| the seam is silently skipped after an earlier failure | confirmed | `experiments/lib/kvm-guest-base.sh:181`; the 2026-09-30 third result lines 49-50 |
| the orphan ran 8m43s with `parent` NULL | not verifiable | prose only, in `TODO/gate.md` and `TODO/RESUME.md:75-78`; no result file records it |

⚠ **The Approach the entry first prescribed would have caused the defect
it names.** It asked for a `qemu-img commit` into the file that carries
the backing pointer, which folds the backing in and makes the
provisioned image overwrite the vendor file. The correction commits into
a separate standalone file.

## What changed

| File | Change |
| --- | --- |
| `crates/podbox-windows/src/plan.rs` | `commit_argv`, a pure function beside `overlay_argv`, so the commit is testable without qemu; `Plan::root`'s comment corrected, it is not always per-run |
| `crates/podbox-windows/src/lib.rs` | `Stop` enum: `Confirmed` only from an observed reap, `Attempted` otherwise; `Error::Timeout` carries it and no longer asserts a stop nobody checked; `spawn_emulator` sets `process_group(0)` before spawn; `stop_emulator` sends QMP `quit`, then SIGKILL to the group, then confirms by reaping; `provision` commits the scratch overlay into a separate `out` |
| `crates/podbox-windows/src/dos.rs` | same process group and stop discipline; forced by the `Error::Timeout` signature |
| `crates/podbox-cli/src/windows/setup.rs` | `plan.root` left in the scratch directory; `out` is a separate commit target |
| `crates/podbox-cli/src/windows/dos.rs` | the one missed call site, which still matched `Error::Timeout(d)` and would not compile |
| `experiments/lib/kvm-guest-base.sh` | the seam reads its exit status before any test; every guarded step says it was skipped; the serial watcher's comment corrected to what it is not evidence of |
| `crates/podbox-cli/tests/detached_stdio.rs` | the tier skip, requiring exit 126 AND both refusal fragments |
| `.github/workflows/gate.yml` | the interposer is built before `cargo test --workspace` |
| `scripts/build-interpose.sh` | resolves the binary in `target/release/` or any per-triple directory |
| `TODO/gate.md` | T-1641's premise, approach, decision, prove clause corrected, and the refutations filed |

## Measured

| Row | Evidence |
| --- | --- |
| record gate | exit 0, 250 rows, 250 entries, 23 open, 9 partial, 0 blocked, 218 done |
| `cargo fmt --all --check` | exit 0, after formatting three hunks the writer left drifting |
| `cargo clippy -p podbox-windows --all-targets` | exit 0, no warnings; `stop` has one caller at `lib.rs:335`, so no dead code was silenced |
| `cargo check -p podbox-windows --all-targets` | exit 0 |
| `check-docs`, `check-markers`, `check-placeholders`, `check-control-bytes` | exit 0 each |
| `cargo fmt --manifest-path crates/podbox-interpose/Cargo.toml -- --check` | exit 0 |
| CI workflow | `yaml.safe_load` parses; step order is bootstrap, interposer, workspace tests |
| `build-interpose.sh` | 5 cases on a fake tree, all as expected; the pre-fix script exits 2 on the same musl-only tree, which is the control |
| tier predicate | 9 drives against the real CI stderr and its negatives, green; removing the exit-code guard turns 2 red |
| diff | 10 files, 852 insertions, 141 deletions |
| toolchain | host `rustc 1.98.0 (88d9e12ae 2026-08-18)`, `channel = "stable"` |

## Not run, and why

⛔ **The first draft of this section said both proofs were blocked by the
host, and that was wrong. Both run in the Linux lane. This session built
on the Windows host, where `ring`'s build script invokes
`scripts/zig-cc.sh` through a Windows process spawner and fails with
`%1 is not a valid Win32 application (os error 193)`. The failure was
real, it reproduced on the clean tree, and it was still the wrong
measurement: `docs/containers.md:33` says Linux builds run in a job
container in `wsl-toolkit-podbox`, and `scripts/dev-lane.sh` is the one
lane runner.** Reported as a host limit, a build-location mistake read
as a platform limitation.

Corrected, measured in the lane on 2026-10-02:

| Row | Evidence |
| --- | --- |
| Lane build | `sh scripts/dev-lane.sh run .tmp/probe-build.sh`, job `d1e71f0605f63e9f`, exit 0, 104.2 s wall |
| Toolchain there | `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0`, zig 0.16.0 at `/usr/local/bin/zig`, gcc 12, musl target present |
| The binary the KVM proof needs | `target/x86_64-unknown-linux-musl/release/podbox`, 3277880 bytes, `Finished release profile in 55.16s`, build exit 0 |
| Job collected | `wsl-toolkit --instance podbox gc --job d1e71f0605f63e9f --apply`, "ledger compacted to 0 open record(s)", exit 0 |
| Lane linter | refused my first job because it piped cargo into `tail`, because a pipe reports the pipe's status. The refusal was correct and is the rule in `scripts/dev-lane.sh` |

So the remaining acceptance for T-1641 and T-1604 is unchanged and is
only a lane run; the binary it needs already builds. Neither proof was
run this session.

- No subagent ran a guest. No subagent edited a `TODO/` row or ran
  `podbox-count`.