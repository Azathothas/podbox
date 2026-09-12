# History

This directory holds superseded project records and review evidence. These
files explain earlier states; they do not override `README.md`, `AGENTS.md`, or
`TODO/PROGRESS.md`.

| Record | Contents |
| --- | --- |
| [`migration-2026-09-11.md`](migration-2026-09-11.md) | Source, template, corpus, review, and validation evidence for the repository migration |
| [`2026-09-11-reference-sweep.md`](2026-09-11-reference-sweep.md) | The sweep of eleven references: what it did not establish, the depth reached per tree, the verdicts, and the six findings |
| [`2026-09-12-store-lock-race-dead-ends.md`](2026-09-12-store-lock-race-dead-ends.md) | [T-0215](../../TODO/image.md)'s superseded `Premise`: five closed mechanisms, the clause series behind them, and the three wrong readings of the fork control |
| [`source-progress-ea5b671.md`](source-progress-ea5b671.md) | Complete live record at the final source revision before migration |
| [`sessions/`](sessions/) | Superseded source-project session summaries |
| [`reviews/`](reviews/) | Focused review outcomes for migrated changes |

---

## ⭐ Claims this project published and later withdrew

⛔ **Read this before trusting a sentence in any document here.**
[`../methodology/history.md`](../methodology/history.md) asks for this list on
the front page, and the reason is blunt: a reader who trusts a page without
checking it trusts sentences that are wrong.

⚠ Every row names where the correction lives. The claim keeps its original
wording in the page that made it, per that methodology's append-never-edit rule.

| the claim | what took it away | where it lives now |
| --- | --- | --- |
| A musl-linked object would fail to load into a glibc payload for symbol or struct reasons | `experiments/60-interposer-libc.sh` measured the SONAME as the discriminator instead | [T-0702](../../TODO/interpose.md) `Premise` |
| `150-image-acquisition.sh` could not leave Docker Hub, because it compares podbox's digest with docker's | docker pulls from `ghcr.io` perfectly well, checked on this host 2026-09-09 | [T-0206](../../TODO/image.md) |
| A `build.rs` that runs `scripts/build-interpose.sh` would deadlock as a cargo inside a cargo | `experiments/158-interpose-embedding.sh` ran it in two shapes and both completed. ⚠ Refused anyway, because it would fire elsewhere | [T-0702](../../TODO/interpose.md) |
| The store lock race was a misdirected `close` leaving a description open | Observation, not a rate: at every capture the process held no description on the inode | [`2026-09-12-store-lock-race-dead-ends.md`](2026-09-12-store-lock-race-dead-ends.md) |
| A concurrent fork was NOT a necessary condition for that race | The fork control's skip list was short of the code three times; completing it reversed the verdict to 0 of 20 | the same page |
| The race was a forked child holding a lock descriptor, in any of five shapes | All five were closed and the rate never moved. The refusal had no holder at all | the same page, and [T-0215](../../TODO/image.md)'s `Done` record |
| A lock handed to a payload is free the instant the payload's descriptor closes | The first draft of that guard asserted it and failed 9 and 13 of 30 | [T-0215](../../TODO/image.md)'s `Done` record |

⭐ **The last two rows are one lesson twice**: a control that has never been
seen to fail, and an assertion that claims more than the design provides, both
read as evidence until something makes them go red on purpose.
