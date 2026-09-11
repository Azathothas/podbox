# Sandbox Insights

An agent-first field manual for diagnosing and designing execution environments
when Linux exposes root-shaped identity but independently restricts namespaces,
syscalls, paths, networks, devices, resources, and virtualization.

The central result is simple:

> Treat privilege as a measured capability vector, then select an execution
> route from the semantics the caller actually requires.

The repository develops that result through reproducible diagnostic rules,
captured observations, failure cases, derived calculations, and an implementation
architecture. It is standalone: every claim used by the local audit resolves to
a Markdown or data file in this tree.

## Start here

Agents should read in this order:

1. [`AGENTS.md`](AGENTS.md) - operating rules and task router.
2. [`docs/capability-model.md`](docs/capability-model.md) - the complete systems
   model and expensive-to-rediscover techniques.
3. [`docs/observations.md`](docs/observations.md) - the recorded evidence and
   measurement conditions.
4. [`docs/failure-field-guide.md`](docs/failure-field-guide.md) - symptom to
   discriminator to action.
5. [`docs/architecture.md`](docs/architecture.md) - a buildable runtime design and
   milestone order.

Markdown is canonical. The paper PDF and LaTeX are derived exports for human
review and submission workflows; they are never the primary source of truth.

## Result

The design has three execution routes and one observation plane:

| Need | Route | Honest boundary |
|---|---|---|
| trusted workload, maximum speed | chroot compatibility | path switch only; no isolation claim |
| native-speed confinement | shared-kernel namespaces + seccomp + path policy | exactly the controls proven by live probes |
| hostile workload, snapshots, or fleets | QEMU/TCG | separate guest kernel inside an emulator process |
| syscall events, reports, or intervention | ptrace or seccomp notification | backend-specific visibility and parity limits |

A weaker route may satisfy an explicit compatibility request. It must never
satisfy a stronger isolation requirement.

## Repository map

| Path | Purpose |
|---|---|
| `AGENTS.md` | primary instructions and routing for agents |
| `docs/observations.md` | self-contained observation notebook and conditions |
| `docs/capability-model.md` | seven-plane model and hard-won mechanisms |
| `docs/evidence-model.md` | local evidence classes and publication rules |
| `docs/research-method.md` | probe, experiment, and review method |
| `docs/architecture.md` | unified runtime design and acceptance gates |
| `docs/failure-field-guide.md` | operational diagnostic lookup table |
| `docs/open-questions.md` | unresolved experiments with explicit closure tests |
| `experiments/` | local audits, machine-readable evidence, data, and logs |
| `scripts/` | one-command validation and paper export tooling |
| `reviews/` | independent review passes and release audit |
| `paper/paper.md` | canonical long-form paper |
| `paper/main.tex` | generated secondary LaTeX export |
| `paper/sandbox-insights.pdf` | generated secondary PDF export |

## Validate everything

From the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-gate.ps1
```

The gate checks the local artifact inventory, every evidence-ledger anchor,
cross-document mechanism consistency, derived benchmark arithmetic, Markdown
links, paper generation, PDF text extraction, metadata, and page rendering.

## Status and limits

This is a research and design repository, not a finished runtime. Captured Linux
observations are preserved with their conditions; the Windows authoring host can
audit them and recompute derived results but cannot rerun kernel-specific probes.

The results do not imply that every sandbox has the same policy. Recorded syscall
and network verdicts changed over time, so any implementation must probe the live
environment and cache results by policy epoch.

## License

All content in this repository is released under the
[Zero-Clause BSD license](LICENSE).
