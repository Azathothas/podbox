# Experiments

Each numbered script owns a repeatable measurement. Its number is permanent.
Do not reuse a number. Run a script from the repository root and read its own
exit code before reading or filtering its output.

| Exit | Meaning |
| --- | --- |
| 0 | The test ran and matched its assertions |
| 1 | The test ran and failed an assertion |
| 2 | Required conditions were absent; the test could not run |

Use [the experiment procedure](../docs/methodology/experiments.md).
Read the script for its pinned inputs, host conditions, timeout, and cleanup.
A historical result applies to its printed revision and host. A denied
operation is evidence only if the script reached and tested that operation.

The initial target reconstruction uses `10-build-target-image.sh`,
`20-enter-target.sh`, and `30-attribution-census.sh`. The retained research
describes a measured floor. Probe the current host before a capability claim.
Later scripts test podbox behavior. Their owning task gives the acceptance.

| Path | Purpose |
| --- | --- |
| `results/` | Tracked measurements, including failures |
| `lib/` | Shared drivers; `engine.sh` selects the available engine |
| `Dockerfile.target` | Pinned reconstruction image |
| `targetfs.sh` | Reconstruction entry point; do not run it on the host |
| `src/` | Programs used by the language measurements |

## Repository audit proofs

| Script | Required conditions and result |
| --- | --- |
| `392-kvm-guest.sh` | Windows toolkit base, explicit Linux binary, pinned operator disk; guest version, exit 42, run path, and cleanup. NOT PASSING: the 2026-09-30 KVM runs failed (`TODO/PROGRESS.md`); this row is historical until an operator-present run re-takes it |
| `393-build-freshness.py` | Python; fixed fixture changes with equal size and time, changed output, and missing output |
| `394-ssh-package.sh` | A native static release build; complete archive and named missing-helper and ELF refusals |
| `395-reconcile-repository.py` | Git refs; patch identity, range comparison, and optional deleted-branch check |
| `396-audit-linux.sh` | Copied toolkit job; full Linux check, clean snapshot, full plants, and exported artifacts |
| `397-exported-build.py` | Export directory; all five executable byte digests agree with the saved build record |
| podbox-smoke --diagnostics | Both gate runners; complete failed output, lost-output mutations, JSON, and skip states |
| `399-publication.py` | Current main commit, deleted branch, release workflow, and complete asset set |
| `400-kvm-cleanup.py` | Linux controlled processes; select the owned emulator, exclude the observer, and preserve another process |

For Windows builds, set `PODBOX_ARTIFACTS` and run
`sh scripts/windows/run-in-base.sh`. This runs the full check in a copy and
returns `linux-check.txt`, five binaries, and their build record on success.
The explicit KVM command is:

```sh
sh experiments/392-kvm-guest.sh --accept-host-risk --binary BINARY --image IMAGE
```

Nested KVM stopped the Windows host on 2026-09-30, so `--accept-host-risk`
is required and was for a long time operator-only. The operator permitted
an unattended run on 2026-10-02: pass `--unattended` as well, and the
driver holds a T-1609 watchdog on the session, refusing to start unless
that watchdog answers first. The driver also refuses to start beside
another emulator or with less than 6144 MiB available in the base.
The image length and digest are checked by the tracked driver. Keep the
licensed disk outside Git. Save the report before collecting the owned job.
Do not make a future test depend on `.tmp` or an expired toolkit transcript.
The earlier experiment guide is retained in
[history](../docs/history/experiments-README.md-before-2026-09-30.txt).
