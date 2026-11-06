# Hidden issues of PodUtils/podvm: recovery and TODO map

Generated 2026-11-06. Every missing number (41: 10-18, 29-38, 63, 65, 67, 95-108, 110-113, 115) returns HTTP 404 from REST and NOT_FOUND from GraphQL; none returns 410. Numbers 1-9, 61, 62, 64, 66 and 116-126 are pull requests that still resolve; nothing exists above 126.

**Likely cause: hidden, not deleted.** All 22 numbers whose metadata was recovered were authored by `talaria0101`, and `gh api users/talaria0101` now returns 404 as well. GitHub hides every issue and PR of a suspended or flagged account (404 everywhere) instead of deleting them; a deleted account would have been reattributed to `ghost`. If the account is reinstated, they should reappear. The 61 surviving issues are by `Nemo-010` (60) and `Azathothas` (1).

Full records, bodies included: `recovered-issues.json` beside this file.

## Counts

- Issues, exact title + full body + metadata: 20 (10-18, 29-38, 65)
- Pull requests, exact title + full body: 2 (63, 67)
- Issues reconstructed from cross-references only (no title, no body): 5 (95, 96, 97, 104, 105)
- Unknown kind, nothing recovered: 14 (98-103, 106-108, 110-113, 115)
- Comment text: not recoverable for any of them (counts only).

## Table

| # | kind | confidence | state | title (or reconstructed summary) | TODO task (status) | recommend |
|---|---|---|---|---|---|---|
| 10 | issue | exact | closed 2026-09-25 | extract does not re-verify blob digests: a tampered store blob extracts cleanly (invariant gap, not a threat-model bypass) | T-1315 (done) | repost-closed-for-record |
| 11 | issue | exact | closed 2026-09-25 | committed Cargo.lock is stale vs the manifests: every fresh build dirties the tree, so the binary self-reports -dirty (explains the beta asset) | T-1316 (done) | repost-closed-for-record |
| 12 | issue | exact | closed 2026-09-25 | no entry path exists when chroot(2) itself is denied: probe says interpose is permitted but every rung, including forced memfd, dies at chroot EPERM | T-1317 (done) | repost-closed-for-record |
| 13 | issue | exact | closed 2026-09-25 | release integrity: beta.1's build commit failed the repo's own gate, and the released bytes do not reproduce outside the lane while the notes read as if they do | T-1334 (done) | repost-closed-for-record |
| 14 | issue | exact | closed 2026-09-23 | podbox cp cannot touch an image's extracted rootfs, so restricted hosts (and any pre-start workflow) have no checked way to stage or retrieve files | T-1323 (done) | repost-closed-for-record |
| 15 | issue | exact | closed 2026-09-25 | no store-health verb (verify) and no pull provenance: digests are enforced per-read but nothing sweeps the store or records where bytes came from | T-1321 (done) | repost-closed-for-record |
| 16 | issue | exact | closed 2026-09-25 | four parity-table gaps have no TODO entries: save/load/import, logs -f, cp directories, launcher signal forwarding | T-1318 (done), T-1320 (done), T-1323 (done), T-1335 (done) | repost-closed-for-record |
| 17 | issue | exact | closed 2026-09-25 | experiments/210 misclassifies registry HTTP 429 as a store-contract failure, and its prune check cannot distinguish a chroot-denied host from a deleted rootfs | T-0210 (done) | repost-closed-for-record |
| 18 | issue | exact | closed 2026-09-23 | README quick start omits build-interpose.sh, so the first-run binary declines every dynamic payload; and no doc maps probe output to expected run behavior | T-1324 (done) | repost-closed-for-record |
| 29 | issue | exact | closed 2026-09-27 | [T-1112] Deferred: Windows guest refused by name instead of implemented | T-1112 (partial) | repost-closed-for-record |
| 30 | issue | exact | closed 2026-09-25 | [T-1317] Deferred: chroot-denied hosts refused up front, no no-chroot rung | T-1317 (done) | repost-closed-for-record |
| 31 | issue | exact | closed 2026-09-25 | [T-1003] Partial: launch ladder sketched rungs and missing embedded rootfs | T-1003 (done) | repost-closed-for-record |
| 32 | issue | exact | closed 2026-09-26 | [T-0413] Partial: no proc fixture, banner only | T-0413 (done) | repost-closed-for-record |
| 33 | issue | exact | closed 2026-09-26 | [T-0414] Partial: probe legs landed, enumeration remedy not taken | T-0414 (done) | repost-closed-for-record |
| 34 | issue | exact | closed 2026-09-25 | [T-0606] Deferred: supervise tier refused whole, no degraded fallback | T-0606 (done) | repost-closed-for-record |
| 35 | issue | exact | closed 2026-09-25 | [T-1301] Deferred: machine tier needs all legs, no TCG without KVM and tun | T-1301 (done) | repost-closed-for-record |
| 36 | issue | exact | closed 2026-09-26 | [T-1306] Deferred: non-goals as refusals, none implemented | T-1306 (done) | repost-closed-for-record |
| 37 | issue | exact | closed 2026-09-25 | [CLI] Beta.7 on chroot-denied host: stale logs note plus refusal ordering masks ptmx | T-0503 (done) | repost-closed-for-record |
| 38 | issue | exact | closed 2026-09-26 | [QOL] Doctor checklist, disk usage, and log tail are missing | T-1337 (done) | repost-closed-for-record |
| 63 | pr | exact | closed 2026-09-27 | T-1112: portable DOS guest under TCG on the machine tier | T-1112 (partial) | drop |
| 65 | issue | exact | closed 2026-09-27 | FR: streaming and chunked acquisition for lanes with a file-size ceiling (T-1112 follow-up) | T-1342 (done), T-1112 (partial) | repost-closed-for-record |
| 67 | pr | exact | closed 2026-09-28 | podssh: `podbox remote ssh`, a relay failover chain, and a server probe | T-1401 (done), T-1402 (done), T-1403 (done), T-1404 (done), T-1405 (done), T-1406 (partial) | drop |
| 95 | issue | reconstructed | open (inferred) | Child-side consequence of #87/#89: a dynamic grandchild exec'd by the payload resolves PT_INTERP in the kernel, re-enters through the HOST loader and ... | T-1407 (done), T-1408 (done), T-1410 (done) | repost-open |
| 96 | issue | reconstructed | open (inferred) | Corrected mechanism for the glibc-refusal class of #87: the absolute lib64/ld-linux-x86-64.so.2 symlink is resolved on the host and decides which libc... | T-1408 (done) | repost-open |
| 97 | issue | reconstructed | open (inferred) | The libc ABI union is incomplete (ld-linux-x86-64.so.2 __tls_get_addr, libdl.so.2 dlsym), so glibc payloads are refused; the caller passes only libgcc... | T-0708 (done) | repost-open |
| 98 | unknown | none | ? |  | none | drop |
| 99 | unknown | none | ? |  | none | drop |
| 100 | unknown | none | ? |  | none | drop |
| 101 | unknown | none | ? |  | none | drop |
| 102 | unknown | none | ? |  | none | drop |
| 103 | unknown | none | ? |  | none | drop |
| 104 | issue | reconstructed | open (inferred) | No -v / volumes on the constrained host: no /dev/fuse and no mount, so no overlay; #114 names a two-root view model (copy regular executables, symlink... | none | repost-open |
| 105 | issue | reconstructed | open (inferred) | Relative-path double prefix when the cwd is already inside the rootfs: map.rs absolutize joins the real getcwd and prepare/rewrite prefix again, givin... | T-1407 (done) | repost-open |
| 106 | unknown | none | ? |  | none | drop |
| 107 | unknown | none | ? |  | none | drop |
| 108 | unknown | none | ? |  | none | drop |
| 110 | unknown | none | ? |  | none | drop |
| 111 | unknown | none | ? |  | none | drop |
| 112 | unknown | none | ? |  | none | drop |
| 113 | unknown | none | ? |  | none | drop |
| 115 | unknown | none | ? |  | none | drop |

## Still-open work

- #95, #96, #97, #104, #105 were most likely open when hidden, and no TODO entry cites them. They belong to the still-open cluster #86-#94, #109, #114 (userland rung on the chroot-denied "errand" host), which also has no TODO entry: the newest ids (T-1407..T-1421) were filed from issues 69-85 and stop there.
- #29 is closed, but its task T-1112 ("A disposable guest that is not Linux") is still **partial**; #63 and #65 also point at T-1112.
- #67 maps to the podssh entries; T-1406 is still **partial**, T-1401..T-1405 are done.
- Every other recovered issue is closed and its TODO task is done.

## Dangling citations in the repository

Each line names a number that no longer resolves (`git grep` over tracked files, vendor/ and references/ excluded, other projects' PR/issue numbers filtered out). Most are TODO `Source: issue N` lines and "Close issue N with a comment" acceptance steps.

```
TODO/INDEX.md:57:| [podssh](podssh.md) | proposed SSH crate | the SSH work in pull requests 66 and 67 |
TODO/PROGRESS.md:83:machine command dispatch. Pull requests 66 and 67 are closed.
TODO/SESSION-SUMMARY-2026-09-28-SSH.md:30:- Four review verdicts (PR 66, PR 67, faketty, fakepty) verified against
TODO/SESSION-SUMMARY-2026-09-28.md:14:| CI | `main` gate passed at `5701a8b` | Gate passed all four jobs at `81fec0c`; PR 67 has three failed checks, PR 66 has no checks
TODO/cli.md:915:Source:      issues 14 and 16, client beta testing 2026-09-22 (`cp`
TODO/cli.md:950:             escapes); `podbox cp --help` documents both. Close issues 14
TODO/cli.md:1272:Source:      issue 38, beta.7 drive 2026-09-25 (no `doctor`, no `df`,
TODO/cli.md:1419:With all three verbs driven and 364 fully green, issue 38
TODO/complete.md:1011:**Partial, 2026-09-25.** Issue 32 reopens this entry: the banner
TODO/complete.md:1159:**Partial, 2026-09-25.** Issue 33 reopens this entry on the remedy:
TODO/complete.md:1350:Source:      issues 18 and 22, client beta testing 2026-09-22 (quick
TODO/complete.md:1385:             a dynamic payload green. Close issues 18
TODO/deps.md:903:Source:      issue 11, client beta testing 2026-09-22 (every fresh build
TODO/deps.md:933:             reports the bare commit. Close issue 11 with a comment
TODO/enter.md:238:**Partial, 2026-09-25.** Issue 37 reopens the ordering half: where
TODO/enter.md:517:Source:      issue 12, client beta testing 2026-09-22 (every rung dies at
TODO/enter.md:554:             as before. Close issue 12 with a comment showing both
TODO/enter.md:606:**Partial, 2026-09-25.** Issue 30 reopens this entry: the up-front
TODO/extract.md:430:Source:      issue 10, client beta testing 2026-09-22 (tampered store blob
TODO/extract.md:468:             with the tampered bytes never written). Close issue 10 with
TODO/image.md:1039:Follow-up 2026-09-23 (issue 17, client beta testing): the harness
TODO/image.md:1053:classified as quota. Close issue 17 with a comment showing the
TODO/image.md:1946:Source:      issue 16, client beta testing 2026-09-22 (registry-only
TODO/image.md:1977:             rootfs tar with the digest recorded. Close issue 16
TODO/image.md:2043:Source:      issue 15, client beta testing 2026-09-22 (no `verify`, no
TODO/image.md:2076:             six fields. Close issue 15 with a comment showing both runs
TODO/image.md:2234:Source:      Issue 65 (streaming and chunked acquisition): the
TODO/packaging.md:601:Source:      issue 13, client beta testing 2026-09-22 (beta.1 build
TODO/packaging.md:636:             Close issue 13 with a comment showing the notes and the
TODO/packaging.md:644:on the exact build commit. Issue 13 is closed, as checked through the
TODO/podssh.md:8:Pull requests 66 and 67 are closed. Their captured heads are historical inputs.
TODO/podssh.md:18:Source:      pull requests 66 and 67 at their captured heads; current SSH source
TODO/podssh.md:116:Source:      captured pull requests 66 and 67; crates/podbox-cli/src/remote/mod.rs and machine.rs
TODO/podvm.md:131:**Partial, 2026-09-25.** Issue 35 reopens this entry: six legs and
TODO/podvm.md:594:**Partial, 2026-09-25.** Issue 36 reopens this entry: each non-goal
TODO/supervise.md:436:**Partial, 2026-09-25.** Issue 34 reopens this entry: the tier
TODO/supervise.md:644:Source:      issue 16, client beta testing 2026-09-22 (automated
TODO/supervise.md:667:             Close issue 16 (logs third) with a comment showing the run
TODO/supervise.md:727:Source:      issue 16, client beta testing 2026-09-22 (Ctrl-C on
TODO/supervise.md:759:             issue 16 (signal third) with a comment showing the run
crates/podbox-cli/src/doctor.rs:3://! [`TODO/cli.md`](../../../TODO/cli.md) T-1337 (issue 38). `probe` prints
crates/podbox-cli/src/parity.rs:481:    // pull request 67 onto the transport main proves: serve registers with
crates/podbox-cli/src/remote/mod.rs:12://! The one member today is `ssh`, brought from podbox pull request 67
crates/podbox-ssh/src/lib.rs:14://! ⭐ **This crate takes the mechanism of podbox pull request 67 and drops
docs/history/CHANGELOG.md-before-2026-09-30.txt:38:lifecycle verifies with zero residue, and pull requests 66 and 67
docs/history/CHANGELOG.md-before-2026-09-30.txt:62:run directory on drop, and issues 29, 58 and 65 are closed with
docs/history/CHANGELOG.md-before-2026-09-30.txt:96:Reviewing this branch against PR 63, an independent answer to the same
docs/history/audit-before-2026-09-30/TODO/milestones.txt:919:**Partial, 2026-09-25.** Issue 29 reopens this entry: the refusal arm
docs/history/audit-before-2026-09-30/TODO/milestones.txt:1049:sibling effort (PR 63).** ⛔ The two efforts answered the same entry in
docs/history/audit-before-2026-09-30/TODO/milestones.txt:1050:different currencies and the entry is only satisfied by both: PR 63 ran
docs/history/audit-before-2026-09-30/TODO/milestones.txt:1145:binary builds here.** The sibling FreeDOS effort (PR 63) landed as a
docs/history/audit-before-2026-09-30/TODO/packaging.txt:280:**Partial, 2026-09-25.** Issue 31 reopens this entry on its own
docs/history/audit-before-2026-09-30/TODO/packaging.txt:744:Source:      issue 13, client beta testing 2026-09-22 (beta.1 build
docs/history/audit-before-2026-09-30/TODO/packaging.txt:779:             Close issue 13 with a comment showing the notes and the
docs/history/audit-before-2026-09-30/TODO/packaging.txt:787:commit is red says so in its own notes. Issue 13 stays open for
docs/history/audit-before-2026-09-30/TODO/podssh.txt:3:This page holds the SSH work from podbox pull requests 66 and 67. Neither
docs/history/audit-before-2026-09-30/TODO/podssh.txt:24:Problem:     podbox has no SSH transport on `main`. Pull request 67 adds one,
docs/history/audit-before-2026-09-30/TODO/podssh.txt:25:             but its current check set has three red jobs. Pull request 66
docs/history/audit-before-2026-09-30/TODO/podssh.txt:28:Premise:     The pull request 67 source uses a byte stream under an SSH
docs/history/audit-before-2026-09-30/TODO/podssh.txt:34:Approach:    Take pull request 67 as a source candidate. Reconcile its
docs/history/audit-before-2026-09-30/TODO/podssh.txt:39:             jobs and add a fresh-clone build. Use pull request 66 only for
docs/history/audit-before-2026-09-30/TODO/podssh.txt:46:             2026-09-28. Pull request 67 is the base. It carries the
docs/history/audit-before-2026-09-30/TODO/podssh.txt:49:             remote dispatch. Pull request 66 is an earlier subset with no
docs/history/audit-before-2026-09-30/TODO/podssh.txt:51:             pull request 67. The cage job finds no tracked passwd shim. The
docs/history/audit-before-2026-09-30/TODO/podssh.txt:77:connection hold). The three red jobs from pull request 67 stay
docs/history/audit-before-2026-09-30/TODO/podssh.txt:96:             that shell. The pull request 67 source has no server shell
docs/history/audit-before-2026-09-30/TODO/podssh.txt:164:             podbox pull request 67
docs/history/audit-before-2026-09-30/TODO/podssh.txt:170:Problem:     The relay path in pull request 67 pairs one node and one client.
docs/history/audit-before-2026-09-30/TODO/podssh.txt:238:Source:      podbox pull request 67, proposed T-1404; [podvm.md](podvm.md)
docs/history/audit-before-2026-09-30/TODO/podssh.txt:246:             The current `main` has neither SSH verb. Pull request 67
docs/history/audit-before-2026-09-30/TODO/podssh.txt:248:Premise:     Pull request 67's `remote_group` calls the SSH crate and its
docs/history/audit-before-2026-09-30/TODO/podssh.txt:269:Scoped:      The remote half exists in pull request 67 as one dispatch
docs/history/progress-before-2026-09-28.txt:128:issues 29, 58 and 65; the beta.8 tag shipped the nightly
docs/history/progress-before-t1403.txt:61:The 2026-09-28 review session reconciled pull requests 66 and 67 through
docs/history/references/ssh-relay-2026-09-28.md:15:| podbox PR 66 | `fd461ee7` | changed-file list, task page, and pull request state | podbox |
docs/history/references/ssh-relay-2026-09-28.md:16:| podbox PR 67 | `66b6fa10` | changed-file list, task page, key source paths, CI results | podbox |
docs/history/references/ssh-relay-2026-09-28.md:46:4. PR 67 accepts `SANDSSH1` as a protocol alias, uses a socketpair for a
docs/history/references/ssh-relay-2026-09-28.md:53:5. PR 67 is mergeable but its build, lint, and document jobs fail. The
docs/history/references/ssh-relay-2026-09-28.md:57:   orphan pages. The other three jobs pass. PR 66 has no check result and
docs/history/references/ssh-relay-2026-09-28.md:65:   portable glibc builds and name resolution. PR 67's proposed SSH client
docs/history/references/ssh-relay-2026-09-28.md:72:Use PR 67 as code to inspect for T-1401. Keep T-1402, T-1403, and T-1404
docs/history/reviews/2026-09-28-windows-lane.md:20:text and code: PR 67 dispatches `remote ssh` and has its parity row,
docs/history/reviews/2026-09-30-repository-audit.md:57:Issues 13 and 26 are closed; their pending-comment text was corrected.
docs/history/session-2026-09-25-to-27.md:275:verified on the lane. Next is closing issues 29, 58 and 65,
experiments/353-open-issue-triage.sh:6:# triage of open issues 29-38 and 49-60 plus dependabot PR 9. Each clause
experiments/353-open-issue-triage.sh:86:# Issue 29: a windows guest is refused by name, nothing fetched.
experiments/353-open-issue-triage.sh:89:# Issue 30: chroot gate on run, memfd force, create, start.
experiments/353-open-issue-triage.sh:97:# Issue 31: ladder rungs report through the gate on this lane.
experiments/353-open-issue-triage.sh:102:# Issues 32/33/34/35/36: probe legs read back from the saved JSON.
experiments/353-open-issue-triage.sh:105:echo "== probe-legs (issues 32-36)"
experiments/353-open-issue-triage.sh:130:# Issue 35: machine tier entry spellings.
experiments/353-open-issue-triage.sh:141:# Issue 38: doctor, df, tail/since absent from help.
experiments/364-qol.sh:4:# TODO/cli.md T-1337 (issue 38: no `doctor`, no `df`, `logs` without
experiments/372-pull-ceiling.sh:4:# TODO/image.md T-1342 (issue 65, second half: the `windows fetch` half
```

Gitignored files also cite them: `.dev/msgF.txt` (issues 10-28 -> T-1315..T-1336), `.dev/triage353c/job.log` (issues 32-36), `refactor/01-audit/group-4.md` (issues 29-38).

Surviving GitHub issues that cite hidden ones: #114 (#95, #96, #97, #104, #105), #109 (#95), #89 (#95), #87 (#95, #96), #86 (#105); closed duplicates #39-#48 cite #29-#38; #27 and #48 cite #16; #49 cites #32; #58 and #59 cite #30 and #35; #64 cites #63; #68 cites #34.

## Which sources worked

- **Worked:** `gh api repos/PodUtils/podvm/issues/events --paginate` (170 events from 2026-09-11). Each `closed` event embeds the full issue object (title, body, author, labels, state, dates, comment count), and the feed still serves events for hidden numbers. This gave all 22 exact records. Only numbers that had an event appear, so the never-closed 95-115 got nothing here.
- **Worked partially:** bodies of surviving issues, which cross-reference #95-#105 (the 5 reconstructions and the creation-time windows).
- **No help:** `repos/PodUtils/podvm/events` and `repos/Azathothas/podbox/events` (same feed, back to 2026-10-09 only, no IssuesEvent); `users/Azathothas/events` and `/events/public` (back to 2026-11-03 only, other repos); the repo issue-comment listing (omits hidden issues); timelines of surviving issues (cross-references to surviving issues only); search API; GraphQL; `Llaberry/podbox` (exists, 6 closed issues of its own, separate numbering); git log (cites issues 6, 9, 17, 60 only); the Claude Code transcripts under `C:\Users\AjamX\.claude\projects\C--Users-AjamX-Downloads-podbox` (no podbox issue numbers or bodies); `.dev/`, `.tmp/`, `refactor/`, `experiments/.*/` (citations only, no bodies).
