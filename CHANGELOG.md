# Changelog

Current behavior and limits are in [README](README.md) and
[Limits](docs/limits.md). Task status is in [INDEX](TODO/INDEX.md).
Earlier repository change records are retained in
[the captured changelog](docs/history/CHANGELOG.md-before-2026-09-30.txt).
They describe their recorded revision, not the current build.

## Unreleased: 0.1.0-beta.12

- The release tag names the workspace version again. Beta.11's binary
  reports beta.10; the nightly build now refuses a mismatched tag
  before one arch builds. See T-1423.

## 0.1.0-beta.11

Superseded: its binary reports 0.1.0-beta.10. Beta.12 carries the
same source with the version string corrected. See T-1423.

- The no-chroot loader family is complete: anchored maps, loader-symlink
  resolution, payload-visible exe, image and container exec, a detached
  launcher, and script refusal naming the file and its shebang. See
  T-1407 through T-1413.
- Exit codes follow docker: absent reads 127, not invocable reads 126,
  podbox failures stay 125. Strict classifies degradations. Runs are
  quiet by default with a verbose banner. Inspect carries the OCI
  config. See T-1414 through T-1417.
- Store gates refuse on a noexec store, verify and heal blobs, and name
  the host once on transport errors. See T-1418 through T-1420.
- A foreground run leaves a container record. See T-1421.
- SSH carries DNS and write deadlines with reconnect, and the compiled
  shim serves exit 42 to a restricted sshd. See T-1406 and T-1401.
- The ladder runs FUSE, tmpfs, cache, rundir, and memfd rungs with an
  OCI-tarball pack flow. See T-1003 and T-1411.
- The gate plants its kept-session check with a batch-file vehicle on
  Windows. See T-1422.
- Known limits: the static golang binary answers exit 2 naming GOROOT;
  the full exit-code drive needs an engine; KVM and ReactOS stay
  deferred to an operator-present run. See T-1409, T-1414, T-1350,
  and T-1112.

## 0.1.0-beta.10

- The source-state page is generated from manifests, enum declarations, and helper entry points. The gate rejects a changed snapshot.
- Development builds verify input bytes, conditions, and all five output binaries. Interposer objects are built before the CLI.
- The Windows job can export the CLI, helpers, and build record. The KVM proof takes explicit binary and disk inputs.
- Release packaging adds four static SSH helpers, dependency licence texts, a checksum, and a signature for each target.
- Current documents separate source behavior from historical proof. T-1003 and T-1401 are partial because their original acceptance is incomplete.
- The read-only pg-toolkit review supplied the build-freshness method. No runtime code from that tree is linked.
- `podbox windows run` no longer pipes the emulator's output without reading it. A noisy emulator could block and read as a guest timeout. A failed run now names the end of `emulator.log`. See T-1112.
- The KVM proof driver refuses without `--accept-host-risk`, beside another emulator, or below 6144 MiB available. A 2026-09-30 run preceded a Windows host failure. See T-1350.

The full Linux gate and fault plants pass. KVM repetition and publication are
being checked. [PROGRESS](TODO/PROGRESS.md) owns their current state.

## 0.1.0-beta.9

The published assets contain the standalone CLI. The source now has relay,
interactive, remote, and machine SSH paths. The later source commits were
not part of the beta.9 assets. Helpers must be supplied separately for that
release. See the captured changelog for the earlier version sequence.
