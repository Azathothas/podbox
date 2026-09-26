# Code map

## Rust crates

| Path | Responsibility |
| --- | --- |
| `crates/podbox-cli` | Command parsing, parity tables, output, orchestration, and lifecycle verbs |
| `crates/podbox-probe` | Raw syscalls, capability findings, rung selection, reports, and shared exit codes |
| `crates/podbox-image` | OCI references, registries, TLS, digest verification, platforms, and the local store |
| `crates/podbox-extract` | Safe layer application, whiteouts, ownership sidecars, and rootfs removal |
| `crates/podbox-complete` | Device shims, account files, package-manager repairs, DNS, hosts, and CA completion |
| `crates/podbox-enter` | Root change, executable resolution, terminal setup, ABI handling, and foreign interpreters |
| `crates/podbox-supervise` | Launcher records, logs, pidfds, signals, and lifecycle state |
| `crates/podbox-windows` | The disposable Windows guest: `fat16.rs` builds and reads the mailbox in process, `agent.rs` holds the two `cmd.exe` scripts and the protocol, `qmp.rs` speaks to the emulator's monitor, `fetch.rs` bounds and verifies a base image, `plan.rs` builds the argv, `lib.rs` drives provisioning and a run |
| `crates/podbox-interpose` | Separate dependency-free glibc/musl preload object for ownership compatibility |

`podbox-interpose` is intentionally excluded from the workspace because it is
loaded into another process and must not inherit workspace dependency or
feature unification. `scripts/build-interpose.sh` builds and inspects both
objects.

## Operational surfaces

| Path | Responsibility |
| --- | --- |
| `scripts/session-start.sh` | The one command a session runs first: machine, UTC instant, tools, lane, then that lane's setup |
| `scripts/dev.sh` | Fast local build and complete contributor check |
| `scripts/windows/run-in-base.sh` | The Windows half of `dev.sh`: one Linux job in a disposable container inside `wsl-toolkit-podbox` |
| `crates/podbox-cli/src/windows/` | The `podbox windows` verb, one module per question: `args` the flag surface, `plan` the paths and the accelerator, `doctor` the report, `setup` acquisition and the one provisioning boot, `run` the boot whose exit status is the guest's, `mod` the dispatch and the `run` seam |
| `scripts/common/restore-modes.sh` | Executable-bit repair, read from the git index, for a tree copied off a filesystem with no mode bit |
| `scripts/check-todo.py` | Independent TODO, citation, corpus, and gate consistency reader |
| `scripts/todo-count.py` | TODO status writer and count derivation |
| `scripts/plant.sh` | Mutation harness proving the project-specific gate can fail |
| `scripts/common/` | Maintained repository checks in shell and PowerShell |
| `experiments/` | Host measurements and milestone acceptance drivers |
| `.github/workflows/gate.yml` | Hosted Rust, TODO, document, security, and artifact gate |
| `.github/workflows/remote-items.yml` | Read-only scheduled review of open issues and pull requests |

## Records and evidence

| Path | Responsibility |
| --- | --- |
| `TODO/INDEX.md` | Every tracked entry and derived status totals |
| `TODO/PROGRESS.md` | Current state, baseline, work order, and operator questions |
| `TODO/reference-map.md` | Corpus provenance and license determinations |
| `experiments/results/` | Immutable raw captures produced by experiment scripts |
| `docs/history/` | Superseded reports, migration records, and review outcomes |
| `THIRD_PARTY.md` | Notice for copied and redistributed material |
