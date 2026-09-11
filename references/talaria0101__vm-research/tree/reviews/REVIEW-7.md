# REVIEW 7 — numeric re-derivation pass over rounds 3–5 content
# (comparison, protocol, fork, policy, spec, paper v2)

Date: 2026-09-10 · Method: every number in the touched documents re-derived
from raw logs; every quoted log line re-grepped; ratio claims recomputed in
an independent calculator (python), not copied from prose.

## Numbers re-derived

| Claim (document) | Re-derivation | Verdict |
|---|---|---|
| chroot native 845.9/866.8 Mops/s (§5.8, comparison §2.1) | `grep 'Mops' logs/76-chroot-route.log` → exact match, checksum 165be307 | ✓ |
| microvm-alpine 35.1 Mops/s ×2 (§5.5, comparison) | `awk '$4'` on 72- matrix log → 35.1, 35.1 | ✓ |
| host baseline 721.4/754.8 (§5.5) | same method → match | ✓ |
| TCG/native ratio "20–25×" (§5.5, comparison §1) | recomputed: like-for-like 20.4×–24.7×; cross-day outliers extend to ~28× (loaded runs 18.6–19.0 Mops/s). **corrected prose from "~21×"/"~24×" to "20–25×" with the outlier noted here** | fixed |
| fork restore ≤2.1 s, snapshot 1.02 s / 76,331,612 B (§5.5, spec §5) | logs/74- exact lines re-read; method limitation (2 s tick bound) re-stated | ✓ |
| exec protocol ready-in-8 s + 5 execs incl. compile (§6, spec §4) | logs/73-protocol-driver.log re-read: ready True (16300 B in 8s), COMPILE-OK, PROTOCOL-POC-DONE | ✓ |
| fsopen patched between 09-09 and 09-10 (§5.2) | 60- log: `fsopen(tmpfs) rc=3` (09-09); re-census 09-10: `rc=-1 EPERM` ×2; clone3 gap re-tested live: `REPLY RECEIVED` ×2 — both states re-verified **today**, not from memory | ✓ |
| write allowlist {/tmp,/dev/shm,/workspace,/state} (§5.6, seccomp.md) | 77- probe: OK/OK/OK/OK; /etc,/usr DENIED; /var/tmp absent | ✓ |

## Defects found → fixed in this pass

1. **Ratio prose imprecision** ("~21×", "~24×" in different places for the
   same measurement family). Replaced everywhere with the honest interval
   **20–25×** plus the outlier note. No table numbers changed.
2. **chroot extraction fragility** surfaced as an experiment failure (76-
   rc=2 "appliance missing"): the appliance extractor aborted on gid-42
   chown without `--no-same-owner`, leaving a broken /bin/sh — and 70-'s
   earlier "CHROOT-OK" had silently tolerated the same broken appliance
   because its success marker printed before the exec attempt. Both
   extractors now use `--no-same-owner`; 70-'s re-run still passes
   (REPLY RECEIVED ×2 re-verified).
3. **Revealed in re-run**: the chroot appliance's `/dev/null` is a *regular
   file* (no mknod; the redirect test rc=0 hides it) — added to §2.1 of the
   comparison as a stated limitation rather than an accident.

## Log-hygiene

- 75- policy log re-read in full: every line is a probe result + conditions;
  no session data.
- 73- protocol log: the transcript is complete (ready/handshake/5 execs);
  the first exec's `rc=None` extraction artifact on a multi-line `ls`
  output is documented in the log itself and in REVIEW-6.

## Verdict

All quantitative claims in the touched documents now re-derive from
committed logs; the one imprecise ratio family was converted to an interval
with its derivation. Two extraction robustness fixes landed. Ready to push.
