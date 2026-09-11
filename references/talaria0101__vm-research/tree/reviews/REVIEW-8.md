# REVIEW 8 — adversarial deep-dive: the three-route comparison under attack

Date: 2026-09-10 · Method: attack `docs/comparison.md` and paper §5.8 as a
hostile reviewer would: mischaracterization checks, fairness checks,
alternative-explanation checks, and a live re-execution of the two cheapest
contested claims.

## A. Mischaracterization attacks

**A1. "podbox-style" is partially the author's construction.**
container-research's `podbox` is a *design* (TOOL.md build order, Rust,
mode ladder), not a shipped tool; my comparison grades a route whose
implementation does not exist. Mitigation verified: the comparison says
"podbox-style (container-research)" and routes every podbox claim through
**[C]** tags tied to their committed claims (N/F/M), never to podbox
behavior. Their own §0.5/§0 disclaimers (M never verified on a runnable
machine; lilipod patch not a seed) are consistent with this treatment.
**Verdict: fairly labeled — kept.**

**A2. "The M-mechanism agreement is overstated."**
Their allowlist is `{/tmp, /dev/shm, /workspace, /state}`. My probe set
tested exactly those four (all OK) plus `/etc`, `/usr` (denied) and
`/var/tmp` (absent — not a denial data point). One subtle difference found:
`/var/tmp` is absent on this instance, and cratera's failed
`/var/tmp/cratera` mkdir (40-) is consistent with absence rather than
denial — the comparison does not claim otherwise. **Verdict: accurate, with
the /var/tmp absence now stated explicitly here.**

**A3. "Same runtime class" is asserted, not proven.**
Evidence for same-class: identical ID map (`0 1000 1`), identical seccomp
signature incl. the clone3 gap, identical Landlock allowlist including the
unusual `/state` path (my `/state` exists and is writable — a distinctive
marker), identical `/proc/pid/mem` EACCES. Counter-evidence: none found.
The wording is "same runtime class", not "the same instance" — defensible.
**Kept, wording unchanged.**

## B. Fairness attacks

**B1. The 20–25× ratio pairs unlike things.**
Native runs and TCG runs were taken at different times under different
load. Like-for-like same-day pairings: host 721.4–754.8 vs microvm-alpine
35.1 → 20.5–21.5×; chroot-native 845.9–866.8 vs microvm-alpine 35.1 →
24.1–24.7×. Cross-day worst case (866.8/18.6) reaches 46.6× but that
18.6 run was load-perturbed (its own log shows the perturbation). The
documents now state **20–25× (like-for-like)** with the outlier noted
(REVIEW-7). Honest.

**B2. chroot "no in-guest gcc installs over network" is wrong.**
E-76 shows apk installing gcc 172 MiB *inside* the chroot over the network.
The comparison does not claim otherwise; it lists in-chroot toolchain
installs as a strength. ✓

**B3. "chroot escape preconditions hold" — did you demonstrate an escape?**
No. E-76 demonstrates the *precondition* (second `chroot()` succeeds for
uid 0 with API availability), and the paper says "preconditions hold",
not "escape demonstrated". The distinction is deliberate: a full escape
demo requires a writable dir staged inside plus a dirfd strategy; the
precondition is the load-bearing security fact. **Wording audited,
accurate.**

**B4. "podvm contains hostile workloads" — TCG guests share the emulator
process.** True and unstated in §5.8: a qemu TCG process is a single tenant
process; a guest kernel compromise escapes into *qemu's* process context
(no KVM means no hardware boundary — TCG is pure emulation, so the guest
kernel runs as userspace code inside qemu). This must be said: under TCG,
"isolation" is qemu-process isolation plus emulation fidelity, **not**
hardware-enforced isolation. **Paper §5.8 and comparison §1 amended** (see
commit) to add: "under TCG the boundary is the emulator process, not
hardware — hardware enforcement is precisely what this sandbox denies."

## C. Alternative-explanation checks

**C1. Is the clone3 gap perhaps not a gap but intended?** Irrelevant to the
comparison's truth value: either way the *capability* is measurably present
today and absent for `unshare`. The 60-/77- pair documents the delta.
**Stands.**

**C2. Could the :80 relaxation be an artifact of my earlier test being
wrong?** No: the earlier denial was measured twice (host curl rc=7, python
EPERM, 71- in-guest "Network unreachable" via slirp), the later permission
twice (python OK, host curl 200). Two independent stacks, four probes, both
states committed. **Stands as a policy change.**

**C3. Is microvm-alpine's 35.1 cherry-picked over pc-alpine's 31.4?**
Both reported; the recommendation prefers microvm for boot+capability
reasons and reports the pc number adjacent. **Stands.**

## D. Live re-execution performed during this review

- `nsvalue2` (70- binary rebuilt from the committed embedded source):
  `REPLY RECEIVED` ×2 — clone3 gap + fabric still functional at review
  time.
- `census` (60- binary): `fsopen` EPERM confirmed still patched; clone3
  still open.
- chroot appliance: gcc compile + native bench re-ran in REVIEW-7's
  re-derivation (845.9/866.8).

## Verdict

The comparison survives adversarial reading with two wording amendments
(TCG-boundary honesty; ratio interval). The strongest true statement it
supports: **under TCG, podvm is the only kernel-boundary route, at a
measured 20–25× compute price; podbox-style namespaces are the only
native-speed confinement and depend on a filter gap the operator has
already begun closing; chroot is native speed with no boundary at all.**
