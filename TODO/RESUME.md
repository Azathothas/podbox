# Resume

Session 2026-10-02, second pass, worked unattended to the end of the
batch. Tree `main` clean and pushed. Record gate exits 0:
250 entries, 23 open, 9 partial, 0 blocked, 218 done.

The session before this one left ten open rows and a work order. This
one took every one of them to a terminal or a named remaining clause,
and filed the twenty-two rows the refactor plan's unbatched set needed.
The full account is
[SESSION-SUMMARY-2026-10-02-UNATTENDED](SESSION-SUMMARY-2026-10-02-UNATTENDED.md).

## Read before acting

`TODO/RULES.md` section 11 carries the standing decisions and section 2
the read-write grant. Do not re-ask any of these.

1. Build compatibility is not a goal. Take whatever toolchain feature
   unlocks the task, nightly included. Fix a toolchain rejection; do not
   ask for a pin.
2. Dead code is a fault of the reader until proven otherwise. Never
   delete a field to quiet a lint.
3. No deferrals. Work needing a human becomes a tracked task with a
   clearing condition. Batch for a later queue, then finish it.
4. Read and write on this repository is authorized, tags and releases
   included. Not another repository, not a force-push.
5. An unattended KVM guest run is permitted on this host under the
   T-1609 watchdog. The Windows host qemu is never touched.

## Run these first

The AGENTS.md start command targets `./target/release/`, which does not
exist on this host. What runs here is the debug binary, and it is what
the gate and every Prove clause must use:

```sh
./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe
```

`scripts/release-licenses.py` and `scripts/build-state.py` search the
per-triple build directories as well as `target/release/`, so the
musl output is found on this host.

## What is green and what is not

Green on `main` at `3470982`, from CI run 36975394409 and the secrets
run 36975394456: the static build, fmt, clippy, the shell and Python
steps, the record gate, and the trufflehog scan of the working tree and
the whole history. Green locally: the maintained repository checks, all
nine of them.

**Red, one row.** `workspace and interposer tests` fails on
`detached_start_returns_before_the_payload_ends`, and T-1604 owns it.
The failure is not a flake. The start exits 126 in five seconds because
`chroot(2)` is denied and the CI job that runs the workspace tests does
not build the interposer objects, so the binary under test carries no
glibc interposer and the no-chroot family declines. The refusal is
correct for the tier; the assertion is not conditional on the tier. Fix
it the way `tests/store_gates.rs` already skips where a host refuses
namespaces, and decide with T-1641 whether that CI job should build the
interposer first.

## Next action

T-1641, in `TODO/gate.md`. It is the only P0 open entry and it holds
three defects the KVM diagnosis found, all read in source and none with
an entry until now.

1. `setup` writes into the vendor's backing image.
   `overlay_argv` passes no read-only backing, so the provisioner writes
   into the pinned VHDX itself and the guest hard-powers-off. Give
   `setup` its own overlay as `root` and commit it to a fresh
   read-only-backed image, the discipline `stage` already uses.
2. A run that reaches its timeout orphans its emulator. Measured live:
   the emulator outlived the kill by 8m43s of CPU with `commandline`
   and `parent` both NULL in `/proc`, so neither
   `experiments/lib/kvm-owned.sh` nor the watchdog can select it, and
   `lib.rs` discards the reaped child's exit status.
3. The proof's seam step is wrapped `if timeout 600 ...; then scode=$?`,
   so a non-zero exit never reaches an `ok:` or a `miss:` and a failing
   seam leaves the verdict to someone else.

Then T-1604, then the rest of the filed queue.

## Measured state

| Row | Evidence |
| --- | --- |
| record gate | exit 0, 250 rows, 23 open, 9 partial, 0 blocked, 218 done |
| lane, Rust 1.99 | [ci-lint-1.99](../experiments/results/ci-lint-1.99.txt): fmt 0, fmt-interpose 0, clippy 0, test 0, 38 suites |
| interposer, 1.99 | [interpose-1.99-objects](../experiments/results/interpose-1.99-objects.txt) exit 0 |
| CI | run 36975394409 on `8dbad80`: three of four jobs green |
| secrets | run 36975394456 green, tree and history |
| nightly | runs 36974650357 and 36974650815 red on six legs; the shim is fixed, the next tag proves it |
| KVM | [attempt 1](../experiments/results/kvm-guest-2026-10-02.txt) and [attempt 3](../experiments/results/kvm-guest-2026-10-02-third.txt) |
| toolchain | host `rustc 1.98.0`; the lane is 1.99.0 |
| retained jobs | none; `wsl-toolkit gc --json` empty at the close |

## Notes that will save time

- `/dev/kvm` needed no reapply this session. `qemu-img` did need
  installing into `wsl-toolkit-podbox`, and it ships in a separate Arch
  package that T-1610's Prove clause does not name.
- `scripts/build-interpose.sh` resolves
  `target/release/podbox-interpose-build` and finds nothing, because
  `.cargo/config.toml` sets the musl build target. It exits 2 in a lane
  and would in CI; the nightly works around it by copying the binary by
  hand. It predates this batch, it has no entry of its own yet, and it
  is named in T-1601's record.
- The serial log is not a hang location. T-1641 records the measurement
  and the comment in `experiments/lib/kvm-guest-base.sh` that claims
  otherwise.
- `refactor/` stays untracked by `.gitignore:152`. T-1613 filed the
  plan's named 21 rows into `TODO/`; the other 29 are still unfiled and
  T-1607 owns them.

## Next prompt

```
Continue podbox from TODO/RESUME.md and work unattended to completion.
Do not stop to ask about anything already settled: TODO/RULES.md section
11 carries the standing decisions and section 2 the read-write grant.

Gate first, with the binary that exists on this host:
./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe
(target/release/ does not; .cargo/config.toml sets the musl target.)

Use subagents and swarms; TODO/RULES.md section 12 owns the rules. The
open work order is T-1641 first, then T-1604, then the rows T-1613
filed, one per file they touch, dispatched the same way: one writer per
file per batch, one agent per disjoint group, a subagent reports and
does not close.

Two things the record names and this session did not do. T-1604's
detached-stdio assertion is not conditional on the tier: it must skip
with a named reason where the no-chroot family legitimately declines,
and decide with T-1641 whether the CI job should build the interposer
before the workspace tests. T-1641's three defects are the KVM guest
hang: setup writes into the vendor's backing image, a timed-out run
orphans an emulator nothing can select, and the proof's seam step
swallows a non-zero exit.

A subagent reports and does not close. Closing an entry, writing its
Done paragraph, and running podbox-count belong to one writer at the
end. Never git add -A while podbox-plant or any --script tool is
running; stage explicit paths. No subagent starts a KVM guest.

Finish the batch, then keep taking the work order until the record is
green, and print the next prompt at the close. Save the measured summary
beside the record before you stop.
```