# Resume

The cold start of a session, on 2026-11-11. This page gives no work order:
[PROGRESS](PROGRESS.md) has the only one.

## Start

1. Do the steps of `AGENTS.md`, section 8. They include the session hook,
   the handoff (`docs/HANDOFF.md`), the decisions (`docs/decisions.md`),
   the last CI runs, the watcher of the organization and
   `sh scripts/cloud-setup.sh`.
2. Build the record tools and run the gate. The workspace builds the
   target `x86_64-unknown-linux-musl` (`.cargo/config.toml`), and the C
   code builds with zig from `sh scripts/cloud-setup.sh`:

   ```sh
   export PATH="$HOME/.local/podvm/zig-x86_64-linux-0.16.0:$PATH"
   cargo build --release -p podvm-gate -p podvm-release
   ./target/x86_64-unknown-linux-musl/release/podvm-gate
   ```

   The gate also runs `document-state` of `podvm-release`. Without that
   binary, it reports that `docs/runtime-state.md` differs.
3. Read the questions for the owner in `docs/decisions.md`. Then take
   the first entry of the work order in [PROGRESS](PROGRESS.md) that is
   not done (the ruling "podvm resumes", 2026-11-11).

## Unfinished state

No change is in progress. Each partial entry names its remaining
clauses in its own record:

| Entry | Remaining clause |
| --- | --- |
| [T-1653](enter.md) | `-v HOST:GUEST:ro` gives `EROFS`; the read-only layer of T-1655 |
| [T-1604](enter.md) | the plant, a second lane run, the saved lane log |
| [T-0211](image.md) | experiment 157: each mutation turns one test red |
| [T-0204](image.md) | the Prove reads the lock skip with podvm as the holder |
| [T-1641](gate.md) | one bounded KVM run with the binary of the current tree |
| [T-1350](gate.md) | one successful run of `experiments/392-kvm-guest.sh` |
| [T-1112](milestones.md) | the KVM leg, on the same run |
| [T-1406](podssh.md) | node redial pairing against the live relay |

## Facts that save time

* KVM is free (the owner, 2026-11-09). A subagent starts a guest only
  when its brief says so. One guest runs at a time, with a timeout, and
  no emulator stays after the run (`AGENTS.md`, sections 2 and 7).
* The cloud host has no `/dev/kvm`. T-1682 runs the Linux guest on a
  hosted runner.
* The Windows lane on wsl-toolkit is parked (`docs/containers.md`). A
  cloud session does not need it.
* The files that podcrates imported are frozen until podvm takes the
  crate (`AGENTS.md`, section 9, item 4). podvm took each of its 11 crates
  at `v0.15.0` (T-1716). A fix of moved code goes to podcrates as a
  COORD issue.
* `refactor/` is not on `main`. It is on the branch
  `archive/podbox-worktree` (`docs/history/worktree-archive.md`). T-1673
  reads the 29 plan rows from it.
* The earlier text of this page is in
  [resume-before-2026-11-10.txt](../docs/history/resume-before-2026-11-10.txt).
