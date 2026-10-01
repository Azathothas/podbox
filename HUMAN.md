# Operator procedure

[PROGRESS](TODO/PROGRESS.md) gives the current work order.
[RESUME](TODO/RESUME.md) gives unfinished work.
[INDEX](TODO/INDEX.md) gives each acceptance command and status.

## Windows session

Run the session entry point from the checkout.
It selects the `podbox` base and waits for its probe.

```powershell
cargo build --release -p podbox-gate
./target/release/podbox-dev session
./target/release/podbox-gate
wsl-toolkit --instance podbox base status --probe
sh scripts/windows/run-in-base.sh
```

[Container procedures](docs/containers.md) describe artifact return and job collection.
Read each exit code. A skipped check does not prove its subject.

## Linux session

Run `./target/release/podbox-dev session`.
The environment and build start while you read the task.
Run `./target/release/podbox-dev status` before using the build.
Run `./target/release/podbox-dev check` before commit.

## Decisions and input

The operator controls image terms, credentials, remote publication, and
changes to shared host resources. Existing session authorization remains valid.
Task entries carry accepted choices. Do not ask for a choice that an entry settles.

For the KVM proof, supply a current Linux binary and a local licensed image:

```powershell
py scripts/windows/kvm-guest.py --accept-host-risk --binary .dev/artifacts/podbox --image IMAGE.vhdx
```

The image stays outside the repository. The proof validates its pinned digest.
The script reports a missing input as exit 2.
On 2026-09-30 a KVM proof run left a guest emulator that SIGKILL did not
remove, and the Windows host then failed. Run the proof only while you are
present. Without `--accept-host-risk`, the driver refuses with exit 2.

## Check a result

Read the script, its conditions, and its result together.
A historical result proves that revision under those conditions.
A status of done applies to the entry's acceptance clauses.
[Current limits](docs/limits.md) name unproved or restricted behavior.
