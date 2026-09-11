# Open questions and closure experiments

These entries are research obligations. Each names the experiment required to
turn an uncertainty into a bounded claim.

## O1. Re-runnable path-policy attribution

The recorded path matrix demonstrates path-dependent verdicts beyond ordinary
DAC, but it does not identify one policy implementation with a clean local
reconstruction.

Close by running the complete write/open/link/rename matrix on a Linux host with
the candidate policy enabled and disabled. Require positive and negative controls,
record policy ABI and boot identity, and reject attribution if another LSM remains
an equally plausible cause.

## O2. PTY behavior before root entry

The availability and semantics of an outer PTY before `chroot` have not been
captured with a complete round trip.

Close with `stat`, open, `grantpt`, `unlockpt`, child I/O, and a payload that reopens
`/dev/pts/N` after the root switch. Record which descriptors survive and which
path-dependent operations fail.

## O3. Complete runtime implementation

The route architecture is `DESIGN-ONLY`; no front end currently satisfies all
milestones in `docs/architecture.md`.

Close one milestone at a time. A milestone requires positive and negative
witnesses, a driven end-to-end path, stable logs, and the three review lenses.
Paper prose is never implementation evidence.

## O4. Complete ownership interposition

Path virtualization and ownership virtualization are separately understood, but
one small runtime has not demonstrated both across a representative package matrix.

Close with a sidecar-backed interposer and tests for chown/stat round trips, fork
safety, the `*at` family, rename/link behavior, static and direct-syscall refusal,
and at least five package managers.

## O5. Seccomp-notify parity and TOCTOU boundaries

The four-leg backend works for selected operations, but complete parity and race
analysis remain open.

Close with notification-ID validity checks before and after work, dirfd/cwd
mutation fixtures, multithreaded tracees, signal races, state-pool exhaustion,
fd-injection failures, and output comparison against ptrace on a capable host.

## O6. Compat and non-native tracees

The current design assumes native x86_64 or aarch64 layouts. Compat syscall tables
and 32-bit iovec layouts are not covered.

Close with explicit 32-bit fixtures, architecture-tagged event records, and parity
tests. Otherwise preserve a tested refusal.

## O7. Same-host KVM baseline

The local dataset compares native and chroot execution with QEMU/TCG. It has no
same-host KVM sample using the same guest artifacts.

Close with identical kernel, initramfs, machine profile, benchmark binary, sample
count, and host-load controls on a host where `/dev/kvm` is actually usable.

## O8. Generality of the 20-25x interval

The interval describes one CPU-oriented benchmark. It is not a general emulator
slowdown factor.

Close with integer, syscall, memory-bandwidth, compilation, and I/O workloads.
Report distributions and same-day native, chroot, TCG, and KVM controls.

## O9. Policy churn cadence

Syscall and network verdicts changed during captured observation windows. Their
trigger and cadence are unknown.

Close with a bounded heartbeat that records boot identity and emits output only
when a canary vector changes. Do not infer policy intent from timing.

## O10. Chroot escape demonstration

Repeated root switching established classic escape preconditions, but the local
evidence does not include a complete escape demonstration.

Close only inside a disposable fixture with no sensitive parent paths. Keep all
current wording at “escape preconditions” until the final outside-path read is
demonstrated and logged.

## O11. Tracing overhead on extraction storms

Full seccomp-notification tracing of a filesystem extraction workload exceeded the
bounded experiment window. The cost has not been decomposed.

Close by measuring notification rate, argument-read cost, open emulation, report
aggregation, storage latency, and passthrough separately under the same fixture.

## O12. Structured-event backend contract

The architecture requires begin, syscall, signal, exit, and summary records, but a
single backend-parity fixture has not yet validated ordering and termination under
normal exit, signal death, exec, and supervisor failure.

Close with a schema validator, an ordered event oracle, and both ptrace and
seccomp-notification runs. Until then, parity remains `OPEN`.
