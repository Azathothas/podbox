# Operator procedure

[PROGRESS](TODO/PROGRESS.md) gives the current work order.
[RESUME](TODO/RESUME.md) gives unfinished work.
[INDEX](TODO/INDEX.md) gives each acceptance command and status.
[The decisions](docs/decisions.md) give each ruling of the owner and the open questions.

## Cloud or Linux session

podvm is worked in Claude Code cloud sessions. `AGENTS.md`, section 8,
gives the start procedure.

```sh
sh scripts/cloud-setup.sh
export PATH="$HOME/.local/podvm/zig-x86_64-linux-0.16.0:$PATH"
cargo build --release -p podvm-gate -p podvm-release
./target/x86_64-unknown-linux-musl/release/podvm-gate
```

Run `./target/x86_64-unknown-linux-musl/release/podvm-dev session` for the host report.
Run `podvm-dev status` before using the build.
Run the checks of `AGENTS.md`, section 12, before commit.
Read each exit code. A skipped check does not prove its subject.

## Windows session (parked)

The Windows lane on wsl-toolkit is parked since 2026-11-06. It stays
correct for wsl-toolkit 9.
[Container procedures](docs/containers.md) describe the base, artifact
return, and job collection.

## Decisions and input

The operator controls image terms, credentials, remote publication, and
changes to shared host resources. Existing session authorization remains valid.
Task entries carry accepted choices. Do not ask for a choice that an entry settles.

KVM is free since 2026-11-09 (`AGENTS.md`, section 2, gives the guest rules).
The cloud host has no `/dev/kvm`.

The KVM proof of the Windows guest (`experiments/392-kvm-guest.sh`) runs
only on the Windows host of the parked lane: it needs the `py` launcher
and a local licensed image, and elsewhere it exits 2.

```powershell
py scripts/windows/kvm-guest.py --accept-host-risk --binary .dev/artifacts/podvm --image IMAGE.vhdx
```

The image stays outside the repository. The proof validates its pinned digest.
The script reports a missing input as exit 2.
On 2026-09-30 a KVM proof run on the Windows base left a guest emulator
that SIGKILL did not remove, and the Windows host then failed. Without
`--accept-host-risk`, the driver refuses with exit 2.
[Limits](docs/limits.md) gives the watchdog and the other guards.
T-1682 owns a hosted-runner workflow for the KVM leg of the Linux guest.

## Check a result

Read the script, its conditions, and its result together.
A historical result proves that revision under those conditions.
A status of done applies to the entry's acceptance clauses.
[Current limits](docs/limits.md) name unproved or restricted behavior.
