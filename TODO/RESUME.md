**Resume**

The cold start of a session, on 2026-11-13. This page gives no work order:
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
| [T-1662](net.md) | the lab guest, its probes and the judge (podcrates@T-345 to podcrates@T-347) |
| [T-1667](cli.md) | the doctor's check of labs in a guest, with T-1662 |
| [T-1740](enter.md) | a cheaper mechanism for the syscall tier |
| [T-1749](enter.md) | the root with no copy (COORD-071 for the namespace rung; the loader family is podvm's) |
| [T-1350](gate.md) | the wrapper of the parked Windows lane |

## Facts that save time

* KVM is free (the owner, 2026-11-09). A subagent starts a guest only
  when its brief says so. One guest runs at a time, with a timeout, and
  no emulator stays after the run (`AGENTS.md`, sections 2 and 7).
* The cloud host has no `/dev/kvm`. T-1682 runs the Linux guest on a
  hosted runner.
* The Windows lane on wsl-toolkit is parked (`docs/containers.md`). A
  cloud session does not need it.
* The files that podcrates imported are frozen until podvm takes the
  crate (`AGENTS.md`, section 9, item 4). The hand-over is done (T-1681),
  and the manifests of podvm name each `pod-*` crate at `v0.45.0`
  (COORD-083, issue 317). A fix of moved code goes to podcrates as a
  COORD issue.
* `refactor/` is not on `main`. It is on the branch
  `archive/podbox-worktree` (`docs/history/worktree-archive.md`). T-1673
  reads the 29 plan rows from it.
* The earlier text of this page is in
  [resume-before-2026-11-10.txt](../docs/history/resume-before-2026-11-10.txt).
