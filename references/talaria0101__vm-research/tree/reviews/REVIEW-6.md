# REVIEW 6 — spec, protocol PoC, fork latency, policy revision

Date: 2026-09-10 · Scope: everything added after REVIEW-5 (73- protocol,
74- fork latency, 75- policy matrix, docs/podvm-spec.md, AGENTS.md rewrite,
paper v2).

## Verified against logs

| Claim | Evidence |
|---|---|
| exec protocol: ready in 8 s; marker-bracketed execs with rc propagation | `logs/73-protocol-driver.log`: ready True (16300 B); `[exec rc=0]` ×5 incl. in-guest `COMPILE-OK` and bench run |
| in-guest benchmark via protocol | transcript (73-protocol-driver.log); earlier 71- log corroborates (29.9 Mops/s on the apk-resident variant) |
| fork restore ≤ 2.1 s | `clone-1..3: first-tick uptime=6.67/6.67/6.65` vs snapshot `U0=4.59` → 2.06–2.08 s, bounded below by the 2 s tick interval; snapshot 1.02 s / 76,331,612 B (74-) |
| network policy revision | 75-: `192.168.1.64:443 → ECONNREFUSED` (policy-allowed), `fastly:80 → OK` (relaxed), binds still EPERM |
| nspriv fabric | 70-: `REPLY RECEIVED` on both loopback and 10.200.0.2 across the private veth |

## Issues found and fixed during this review

1. **`tools/podvm` draft removed.** The half-built tool duplicated logic the
   spec now states declaratively; its untested `cmd_run` had a redirect bug.
   Per the research mandate, the deliverable is `docs/podvm-spec.md`
   (evidence-cited) + PoC experiments, not a product.
2. **73- root cause documented**: qemu subshell `cd`s then used a relative
   `-kernel vmlinuz-virt` → instant death ("could not open kernel file") —
   this, not the fifo transport, was the earlier ready:False. Absolute
   kernel path; fifo pairs pre-created with an `O_RDWR` holder (qemu's
   `pipe:` backend opens existing fifos and blocks otherwise).
3. **Paper §5.5 numbers re-derived**: 35.1/35.1 (microvm), 31.4/35.1 (pc),
   721.4/754.8 (host) — all match logs; checksum `165be307` single-valued
   across every platform.
4. **TLS-egress claim revised to a two-measurement statement**: denied
   early-session (71- era), permitted after live relaxation (75- era); both
   logs committed. A paper claim that silently outlives its policy would
   have been wrong within one day — the two-condition statement is the
   durable form.

## Residual notes

- 73- transcript's first exec shows `rc=None` extraction on a multi-line
  output (marker regex vs wrapped ls output) — the transcript carries the
  full data; noted for implementers: anchor markers at line boundaries.
- 74- restore latency is tick-bounded (2 s interval): the number is an upper
  bound; true restore is likely sub-second. Stated as such in the paper.

## Verdict

Spec claims are evidence-backed; protocol, fork, and policy claims verified
line-by-line. The repository now supports an implementer end-to-end: environment
contract, working PoC protocols with committed transcripts, failure
signatures, and the design constraints derived from measurements.
