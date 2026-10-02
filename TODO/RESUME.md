# Resume

Session 2026-10-02. Tree `main` clean and pushed. Record gate exits 0:
226 entries, 10 open, 3 partial, 0 blocked, 213 done.

This file was rewritten at the end of the session because four read-only
audits found it three commits stale, carrying three mutually inconsistent
counts and still listing two completed entries as open blockers. It is
the cold-start handoff; if it disagrees with the gate, the gate wins.

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
   T-1609 watchdog. Everything the proof needs is installed into
   `wsl-toolkit-podbox`; the Windows host qemu is never touched.

## Run these first

The AGENTS.md start command targets `./target/release/`, which does not
exist on this host. What runs here is the debug binary, and it is what
the gate and every Prove clause on this host must use:

```sh
./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe
```

`cargo build --release -p podbox-gate` does not fill `target/release/`
here because `.cargo/config.toml` sets the musl build target; zig is on
PATH so it can be built, but it lands under
`target/x86_64-unknown-linux-musl/release/`.

Two checks are red on arrival and each has an owner. Neither is a
surprise and neither blocks the work order:

- `check-one-home.sh` exits 1 on a duplicated sentence in two Batch 3
  Done paragraphs. T-1608.
- `check-no-secrets.sh --public` exits 1 on `/home/toolkit` in the lane
  runner. T-1603.

## Next action

T-1601, in `TODO/interpose.md`. Rust 1.99 rejects the `open`
interposition at `crates/podbox-interpose/src/lib.rs:2144` with a
deny-by-default lint, and that one failure stops the static build, the
interposer tests, and plants 32e and 32f with it. Allow the lint at the
definition, forward-compatible through `unknown_lints`. Do not pin the
toolchain.

This cannot be reproduced on this host: `rustc -V` is 1.98.0 and 1.99 is
not installed. The lint is a CI observation. Do not close the entry on
a local green run, because a local run has no lint to fire.

Then T-1602, T-1603, T-1604, which clear the other three red CI jobs,
then T-1608 and T-1611, then T-1612, T-1607, T-1606, T-1605.

## Open work

The work order lives in `TODO/PROGRESS.md` and only there;
`docs/methodology/work-todo.md` forbids a second one. The ten open rows
are T-1601 through T-1612 as listed in `TODO/INDEX.md`. Three carry a
condition a previous session found and a new agent would otherwise
rediscover:

- **T-1601** cannot be reproduced here. `rustc -V` is 1.98.0 and 1.99 is
  not installed, so a local green run has no lint to fire. Prove it in
  the Linux lane.
- **T-1602**'s Premise was corrected on 2026-10-02: its consumer tests
  exist and read all four fields. The lint fires in `parallel_layers`
  alone. Do not port anything that is already written.
- **T-1603** must land the deletion and the trufflehog workflow in one
  commit. Two commits leave the repository with no secrets scanning in
  between, and pushes to `main` are authorized.

## Partial entries

T-1350 can now run: T-1609 built and wired the watchdog, T-1610 repaired
the base and installed qemu and OVMF into it. Run it with both
`--accept-host-risk` and `--unattended`.

Before any guest run, check `/dev/kvm`. The base shipped it as
`crw------- root root` and the guest account is uid 1000, so it was
changed by hand to `crw-rw---- root kvm`. That is per boot: the base's
own tmpfiles rule replays `z /dev/kvm 0666 - kvm -`. After a reboot,
reapply it or run the guest as root.

T-1112 stays partial: its KVM leg rides the same run, and its ReactOS
leg needs a redistributable base that does not exist anywhere in the
tree. No task owns that acquisition and no standing decision covers it.
It is the one remaining item a human must decide.

## Measured state

| Row | Evidence |
| --- | --- |
| record gate | exit 0, 226 rows, 10 open, 3 partial, 0 blocked, 213 done |
| KVM watchdog | `experiments/results/kvm-watchdog.txt`, three runs, three arms each, all green |
| KVM bound | `experiments/results/kvm-watchdog-bound.txt`, a 3 s bound reports rc 124 and removes the emulator |
| base packages | `experiments/results/kvm-base-provision.txt` |
| CI | four jobs red at `3bd36f0`; run 36956512663 was in progress at session end |
| toolchain | `rustc 1.98.0`; 1.99 not installed locally |
| retained jobs | none; `wsl-toolkit gc --json` empty at session end |

## Untracked working material

`refactor/` stays untracked by `.gitignore:152`. It holds the refactor
plan, including the 21 unbatched rows T-1607 owns, and they cannot be
read from a fresh clone.

`refactor/DEFERRALS.md` is superseded. Its four sections live in tracked
entries. Do not treat it as the record.

## Next prompt

```
Continue podbox from TODO/RESUME.md. Read TODO/RULES.md section 11
before acting; five standing decisions from 2026-10-02 bind this work.
The gate that runs on this host is
./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe, not the one under
target/release. Take T-1601 from TODO/interpose.md and implement it;
its lint cannot be reproduced here, so prove it in the Linux lane.
```