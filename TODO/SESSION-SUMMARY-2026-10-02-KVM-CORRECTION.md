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

- **`cargo test --workspace` did not run here.** `ring`'s build script
  invokes `scripts/zig-cc.sh` through a Windows process spawner, which
  cannot execute a shell script: `%1 is not a valid Win32 application
  (os error 193)`. Verified on the clean tree with the edits stashed, so
  it is pre-existing and not introduced here.
- **The KVM proof did not run.** It needs a musl static `podbox` binary,
  and the same `ring` failure is what blocks that build on this host. The
  image is present and correct: `/c/Users/AjamX/podbox-images/ValidationOS.vhdx`,
  910163968 bytes, the pinned length. The binaries under `.dev/artifacts*`
  are from 2026-09-30 and predate every change in this session, so a run
  with any of them would prove nothing. T-1641 therefore closes partial,
  with the lane as its named remaining acceptance.
- No subagent ran a guest. No subagent edited a `TODO/` row or ran
  `podbox-count`.