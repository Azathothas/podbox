# REVIEW 2 — adversarial pass: claims attacked, reproducibility tested

Date: 2026-09-09 · Posture: try to break the repo's own claims. Also the
hygiene pass (secrets, pairing, conditions, leanness).

## Hygiene results

| Check | Result |
|---|---|
| Secrets in tracked tree | none (grep `ghp_/gho_/github_pat/Authorization/token` over tracked .log/.md/.sh: only hits inside gitignored `work/lima-src`, upstream docs) |
| Experiment ↔ log pairing | all 22 numbers have both script and log (10, 11, 20–21, 30–45, 50–51) |
| Conditions blocks | every `logs/*.log` starts with `## conditions` |
| Leanness | tracked tree ≈ 120 KB of logs + text; `work/` (gigabytes of binaries/images) gitignored and verified ignored |
| Reproducibility | `21-` re-run from clean shell: rc=0, **1.41 s wall** (first logged run: 3.25 s; variance = warm page cache and TCG TB reuse, conditions identical otherwise) |

## Attacks and what they found

1. **"Five runnables" was a miscount.** lima reaches qemu but never completes a
   boot — it is not runnable. Bottom line corrected to **four end-to-end
   runnables** (pc, microvm, smolBSD, nanos) with lima's exact stop point
   stated.
2. **pullrun vm-mode evidence re-checked.** The log does contain the decisive
   line: `rpc error: ... stage kernel pullrun/kernel-asahi:6...` (pull of the
   kernel image fails with UNAUTHORIZED), after which the firecracker chain of
   30- applies. The row stands.
3. **clawk [S] anchor re-checked.** `logs/36-` contains the binary's own help
   line naming `--provider ... firecracker` as the Linux path, so the [S] tag
   is anchored in committed evidence, not memory. Row stands.
4. **[S]+[V] chains audited.** Every [S] quote in 31/32/37/38/39 carries its
   pinned source URL and the grep output in the log; every [V] half cites a
   line from `10-`. No row relies on an unquoted upstream claim.
5. **Number stability rule applied.** The 21- re-run produced a different wall
   time (1.41 s vs 3.25 s). Per methodology the original number stays attached
   to the original log; this review records the re-run here rather than
   overwriting. Anyone quoting "TCG microvm boot time" must quote both
   conditions (cold-ish first run vs warm re-run).

## Verdict

Claims survive the pass with two corrections applied (miscount; evidence
locality already fixed in REVIEW-1). The repo now contains no claim whose
proof is outside the tracked tree.
