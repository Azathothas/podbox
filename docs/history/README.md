# History

This directory holds superseded project records and review evidence. These
files explain earlier states; they do not override `README.md`, `AGENTS.md`, or
`TODO/PROGRESS.md`.

| Record | Contents |
| --- | --- |
| [Repository audit review](reviews/2026-09-30-repository-audit.md) | Source paths, failure tests, and cold-start evidence |
| [Earlier changelog](CHANGELOG.md-before-2026-09-30.txt) | Version and task change records before reconciliation |
| [Earlier experiment guide](experiments-README.md-before-2026-09-30.txt) | Superseded script catalogue and host claims |
| [Repository audit source comparison](../../experiments/results/publish-branch-comparison.txt) | Publish-branch patch identity and context differences |
| Documents before the audit | Commit `005638d` on main; read a page with `git show 005638d:PATH` |
| [Lane proofs of 2026-09-23](lane-proofs-2026-09-23/README.md) | Seven closure scripts that ran from the ignored `.tmp/` directory, copied unchanged |
| [Earlier T-1003 record](audit-before-2026-09-30/TODO/packaging.txt) | Earlier scoped closure and the omitted ladder work |
| [Earlier T-1112 record](audit-before-2026-09-30/TODO/milestones.txt) | Guest implementation stages before the current acceptance record |
| [pg-toolkit audit](references/pg-toolkit-2026-09-30.md) | Selected source, read depth, adopted mechanism, and remaining candidates |
| [Retired template option](lean-adoption-before-2026-09-30.txt) | An alternative work model that does not apply to this project |
| [`migration-2026-09-11.md`](migration-2026-09-11.md) | Source, template, corpus, review, and validation evidence for the repository migration |
| [`2026-09-11-reference-sweep.md`](2026-09-11-reference-sweep.md) | The sweep of eleven references: what it did not establish, the depth reached per tree, the verdicts, and the six findings |
| [`2026-09-12-store-lock-race-dead-ends.md`](2026-09-12-store-lock-race-dead-ends.md) | [T-0215](../../TODO/image.md)'s superseded `Premise`: five closed mechanisms, the clause series behind them, and the three wrong readings of the fork control |
| [`source-progress-ea5b671.md`](source-progress-ea5b671.md) | Complete live record at the final source revision before migration |
| [`session-2026-09-25-to-27.md`](session-2026-09-25-to-27.md) | The continuous session's superseded record: triage narrative, triage table, finished work order, closed in-progress narrative, verbatim |
| [`upstream-tool-shape.md`](upstream-tool-shape.md) | Retired two-product wording for `wsl-toolkit`, kept verbatim with what took it away |
| [`containers-before-toolkit-6.txt`](containers-before-toolkit-6.txt) | Windows lane procedure before `wsl-toolkit` 6.0.0, kept verbatim |
| [`progress-before-2026-09-28.txt`](progress-before-2026-09-28.txt) | Superseded session state and work order, kept verbatim |
| [`progress-before-t1403.txt`](progress-before-t1403.txt) | Superseded lane, source-capture, and partial record displaced by the T-1403 relay landing, kept verbatim |
| [`index-order-before-2026-09-28.txt`](index-order-before-2026-09-28.txt) | Superseded priority argument, kept verbatim |
| [`reference-discussion-gap-before-2026-09-28.txt`](reference-discussion-gap-before-2026-09-28.txt) | Superseded claim that no reference had Discussions data, kept verbatim |
| [SSH relay source check](references/ssh-relay-2026-09-28.md) | SSH relay source comparison and pull request findings |
| [Reverse relay protocol (r12)](references/relay-index-2026-09-28-r12.md) | Live relay protocol document captured 2026-09-28, the authority for the `reverse-v1` spec |
| [`t1112-kvm-blocker-before-2026-09-28.txt`](t1112-kvm-blocker-before-2026-09-28.txt) | [T-1112](../../TODO/milestones.md)'s superseded KVM blocker text |
| [`reviews/2026-09-28-windows-lane.md`](reviews/2026-09-28-windows-lane.md) | Three review passes over the Windows lane and work record |
| [`sessions/`](sessions/) | Superseded source-project session summaries |
| [`reviews/`](reviews/) | Focused review outcomes for migrated changes |
| [Session summary 2026-09-28](../../TODO/SESSION-SUMMARY-2026-09-28.md) and [SSH session summary](../../TODO/SESSION-SUMMARY-2026-09-28-SSH.md) | Final checks, change size, remote and machine state; the SSH reconcile, partial, machine arm, and KVM unblock |

---

## ⭐ Claims this project published and later withdrew

⛔ **Read this before trusting a sentence in any document here.**
[`../methodology/history.md`](../methodology/history.md) asks for this list on
the front page, and the reason is blunt: a reader who trusts a page without
checking it trusts sentences that are wrong.

Each row names the current correction. Required earlier wording is retained
in history or its captured task record. Live text is corrected in place.

| the claim | what took it away | where it lives now |
| --- | --- | --- |
| A musl-linked object would fail to load into a glibc payload for symbol or struct reasons | experiments/60-interposer-libc.sh measured the SONAME as the discriminator instead | [T-0702](../../TODO/interpose.md) `Premise` |
| `150-image-acquisition.sh` could not leave Docker Hub, because it compares podbox's digest with docker's | docker pulls from `ghcr.io` perfectly well, checked on this host 2026-09-09 | [T-0206](../../TODO/image.md) |
| A `build.rs` that runs `scripts/build-interpose.sh` would deadlock as a cargo inside a cargo | `experiments/158-interpose-embedding.sh` <!-- known-absent --> ran it in two shapes and both completed. ⚠ Refused anyway, because it would fire elsewhere | [T-0702](../../TODO/interpose.md) |
| The store lock race was a misdirected `close` leaving a description open | Observation, not a rate: at every capture the process held no description on the inode | [`2026-09-12-store-lock-race-dead-ends.md`](2026-09-12-store-lock-race-dead-ends.md) |
| A concurrent fork was NOT a necessary condition for that race | The fork control's skip list was short of the code three times; completing it reversed the verdict to 0 of 20 | the same page |
| The race was a forked child holding a lock descriptor, in any of five shapes | All five were closed and the rate never moved. The refusal had no holder at all | the same page, and [T-0215](../../TODO/image.md)'s `Done` record |
| A lock handed to a payload is free the instant the payload's descriptor closes | The first draft of that guard asserted it and failed 9 and 13 of 30 | [T-0215](../../TODO/image.md)'s `Done` record |
| `wsl-toolkit` ships as two products, a script and a compiled carrier with a launcher | Upstream deleted the PowerShell product and its launcher; the binary on this host answers as one product | [`upstream-tool-shape.md`](upstream-tool-shape.md) |
| No reachable machine holds `/dev/kvm` | `experiments/385-kvm-open.sh` opens the node and reads API version 12 on this host 2026-09-28 | [T-1112](../../TODO/milestones.md) |

⭐ **The last two rows are one lesson twice**: a control that has never been
seen to fail, and an assertion that claims more than the design provides, both
read as evidence until something makes them go red on purpose.
