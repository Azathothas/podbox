# Resume

Current assignment: publish the reconciled audit and the beta.10 release.
Read AGENTS.md and run the session entry point before work.

The audit and the recovery fixes are committed to main in this session.
If `origin/main` does not contain them, push main without force after the
full host and Linux checks pass. Wait for the exact-commit `gate` run.

After a green gate, push tag `v0.1.0-beta.10` on that commit. The
`nightly` workflow publishes seven binaries and seven SSH archives with
checksums and signatures. Verify with
`py experiments/399-publication.py --release v0.1.0-beta.10` and
`sh scripts/verify-release.sh v0.1.0-beta.10 x86_64 ssh`.
Close T-1405 only when both pass.

Then delete `publish/20260926T052210Z` on origin and run
`py experiments/395-reconcile-repository.py --expect-deleted`.

Do not start a KVM guest. T-1350 and T-1112 need the operator present.
The permanent work order is in PROGRESS.
