# GitHub issues hidden on 2026-11-06

GitHub hid 41 issue and pull request numbers of PodUtils/podvm (formerly
Azathothas/podbox) together with the account that wrote them, `talaria0101`.
Each number answers 404 from the REST and GraphQL APIs, not 410, so GitHub
hid them and did not delete them. If GitHub restores the account, they can
come back. The account `talaria0101` also owned five of the captured
reference trees; their upstreams answer 404 for the same reason.

The issue events API still serves the events of a hidden issue, and each
close event carries the whole issue. That returned 20 issues and 2 pull
requests exactly. The bodies of surviving issues name five more, which were
open when GitHub hid them; podvm reposted those five with the label
`recovered`. No comment text came back for any number.

The full records, with bodies, are on the branch `archive/podbox-worktree`
in `github-2026-11-06/recovered-issues.json`. The same directory holds a
snapshot of every issue and pull request that still resolved on 2026-11-06.

```sh
git fetch origin archive/podbox-worktree
git show origin/archive/podbox-worktree:github-2026-11-06/recovered-issues.json
```

Citations of these numbers in `TODO/`, `docs/` and code comments stay as
written. Each one names the number that the issue had.

## The records

`exact` is the title and body from a close event. `reconstructed` is a
statement rebuilt from what surviving issues say about the number.

| # | Kind | Recovered | Title or statement | TODO entries | Now |
| --- | --- | --- | --- | --- | --- |
| 10 | issue | exact | extract does not re-verify blob digests: a tampered store blob extracts cleanly (invariant gap, not a threat-model bypass) | T-1315 (done) | closed |
| 11 | issue | exact | committed Cargo.lock is stale vs the manifests: every fresh build dirties the tree, so the binary self-reports -dirty (explains the beta... | T-1316 (done) | closed |
| 12 | issue | exact | no entry path exists when chroot(2) itself is denied: probe says interpose is permitted but every rung, including forced memfd, dies at c... | T-1317 (done) | closed |
| 13 | issue | exact | release integrity: beta.1's build commit failed the repo's own gate, and the released bytes do not reproduce outside the lane while the n... | T-1334 (done) | closed |
| 14 | issue | exact | podbox cp cannot touch an image's extracted rootfs, so restricted hosts (and any pre-start workflow) have no checked way to stage or retr... | T-1323 (done) | closed |
| 15 | issue | exact | no store-health verb (verify) and no pull provenance: digests are enforced per-read but nothing sweeps the store or records where bytes c... | T-1321 (done) | closed |
| 16 | issue | exact | four parity-table gaps have no TODO entries: save/load/import, logs -f, cp directories, launcher signal forwarding | T-1318 (done), T-1320 (done), T-1323 (done), T-1335 (done) | closed |
| 17 | issue | exact | experiments/210 misclassifies registry HTTP 429 as a store-contract failure, and its prune check cannot distinguish a chroot-denied host... | T-0210 (done) | closed |
| 18 | issue | exact | README quick start omits build-interpose.sh, so the first-run binary declines every dynamic payload; and no doc maps probe output to expe... | T-1324 (done) | closed |
| 29 | issue | exact | [T-1112] Deferred: Windows guest refused by name instead of implemented | T-1112 (partial) | closed |
| 30 | issue | exact | [T-1317] Deferred: chroot-denied hosts refused up front, no no-chroot rung | T-1317 (done) | closed |
| 31 | issue | exact | [T-1003] Partial: launch ladder sketched rungs and missing embedded rootfs | T-1003 (done) | closed |
| 32 | issue | exact | [T-0413] Partial: no proc fixture, banner only | T-0413 (done) | closed |
| 33 | issue | exact | [T-0414] Partial: probe legs landed, enumeration remedy not taken | T-0414 (done) | closed |
| 34 | issue | exact | [T-0606] Deferred: supervise tier refused whole, no degraded fallback | T-0606 (done) | closed |
| 35 | issue | exact | [T-1301] Deferred: machine tier needs all legs, no TCG without KVM and tun | T-1301 (done) | closed |
| 36 | issue | exact | [T-1306] Deferred: non-goals as refusals, none implemented | T-1306 (done) | closed |
| 37 | issue | exact | [CLI] Beta.7 on chroot-denied host: stale logs note plus refusal ordering masks ptmx | T-0503 (done) | closed |
| 38 | issue | exact | [QOL] Doctor checklist, disk usage, and log tail are missing | T-1337 (done) | closed |
| 63 | pr | exact | T-1112: portable DOS guest under TCG on the machine tier | T-1112 (partial) | closed |
| 65 | issue | exact | FR: streaming and chunked acquisition for lanes with a file-size ceiling (T-1112 follow-up) | T-1342 (done), T-1112 (partial) | closed |
| 67 | pr | exact | podssh: `podbox remote ssh`, a relay failover chain, and a server probe | T-1401 (done), T-1402 (done), T-1403 (done), T-1404 (done), T-1405 (done), T-1406 (partial) | closed |
| 95 | issue | reconstructed | Child-side consequence of #87/#89: a dynamic grandchild exec'd by the payload resolves PT_INTERP in the kernel, re-enters through the HOS... | T-1407 (done), T-1408 (done), T-1410 (done) | reposted as #127 |
| 96 | issue | reconstructed | Corrected mechanism for the glibc-refusal class of #87: the absolute lib64/ld-linux-x86-64.so.2 symlink is resolved on the host and decid... | T-1408 (done) | reposted as #128 |
| 97 | issue | reconstructed | The libc ABI union is incomplete (ld-linux-x86-64.so.2 __tls_get_addr, libdl.so.2 dlsym), so glibc payloads are refused; the caller passe... | T-0708 (done) | reposted as #129 |
| 104 | issue | reconstructed | No -v / volumes on the constrained host: no /dev/fuse and no mount, so no overlay; #114 names a two-root view model (copy regular executa... | - | reposted as #130 |
| 105 | issue | reconstructed | Relative-path double prefix when the cwd is already inside the rootfs: map.rs absolutize joins the real getcwd and prepare/rewrite prefix... | T-1407 (done) | reposted as #131 |

No trace came back for 98, 99, 100, 101, 102, 103, 106, 107, 108, 110, 111, 112, 113, 115. The kind of each is unknown.
Numbers 1 to 9, 61, 62, 64, 66 and 116 to 126 are pull requests that still
resolve.
