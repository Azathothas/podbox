# REVIEW 3 — adversarial: attack the instruments and their interpretations

Date: 2026-09-10. Method: for each load-bearing interpretation, try to
construct an alternative explanation the instrument cannot exclude,
then either refute it with a probe or soften the claim. Three attacks
landed; two hardened the instruments, one sharpened a sentence.

## A. The bogus-argument discriminator

**Attack A1: "EPERM pre-entry proves seccomp — but could an LSM deny
before the path resolution?"** Landlock's hooks fire after path lookup
(a bogus target would answer ENOENT first, since the lookup fails
before `security_sb_mount` runs); a BPF-LSM could in principle deny at
entry. The instrument cannot exclude every conceivable entry layer —
but it can check whether the denial layer is *selective*: a uniform
entry-denial layer would deny everything, not a list.

*Probe added:* entry-layer controls (`membarrier(bogus)` → EINVAL,
`mprotect(0,0,0)` → OK) — plain syscalls with no plausible denylist
entry reach the kernel and answer argument-shaped errnos. Combined
with the measured `Seccomp: 2` and the observed live patching of the
list (fsopen closed between sessions while the adjacent gap survived),
the number-filter attribution stands. **Wording hardened** in
`docs/attribution.md`: pre-entry EPERM is attributed to the syscall-
number filter as the one mechanism *known to be present* that denies
at that layer, with the entry controls as the standing check that the
layer is selective, not uniform.

**Attack A2: "The controls only prove the prober discriminates; they
don't prove the denied rows would answer argument-shaped errnos if
unfiltered."** True and unfixable on a host where the filter cannot be
removed — which is exactly why the methodology's removal-based
attribution (compose the mechanisms on a host you control) exists.
The document says this; no change.

## B. The network-policy mechanism claim

**Attack B1: "'cgroup-BPF SOCK_ADDR class' is an inference from a
signature, not a measured attachment."** Correct, and the documents
now say so in exactly those terms: the *measured* facts are address-,
port- and family-aware denials returning EPERM with live
reconfigurability — which seccomp cannot produce (it cannot read the
sockaddr) and which match one known mechanism class. The claim is
"the signature identifies the class", never "we observed the
attachment". `docs/environment.md` mechanism P and paper Table 1
checked sentence-by-sentence against this standard; the inet6 probes
(REVIEW-2 #3) extended the family-awareness evidence.

## C. The ownership wall

**Attack C1: "EINVAL from tar's chown — how do you know it is the
mapping check and not the filter denying chown pre-entry?"** Three
committed facts close it: chown to a *mapped* id succeeds (so the
syscall is not filtered); chown to an unmapped id answers EINVAL, not
EPERM (a filter refusing a syscall name answers EPERM for every
argument, mapped or not); and the kernel's translation of unmapped
ids to INVALID_UID/GID is documented behaviour [S]. The census's
chown row and experiment 25- carry the pair. No change needed; the
row was already worded to carry it.

## D. The near-native TCG integer run

**Attack D1: "835 Mops/s in-guest is impossible for emulation — is
the bench really running in the guest?"** A fair challenge, and the
checksum discipline is the answer: the guest's bench prints the
host-identical checksum over 30M iterations whose output the serial
protocol carried back through the guest console. The mechanism is
also plausible as stated: TCG translates guest instructions to host
instructions once per translation block, so a tight register-only
loop re-executes translated code at near host speed — what TCG
cannot hide is translation cost (boot, cold paths) and unoptimized
dependency chains (the FP-heavy 6.9–12× stands as the honest
emulation tax). The paper now states both halves with the measured
range instead of the earlier single pairing.

## E. What the attacks did not shake

- The four-walls structure: each wall's errno signature and fix are
  backed by a reproduction that runs the *fix* and shows it working
  (ownership-neutral extraction, `NoSetGroups`, the entry controls,
  copy-in seeding).
- The namespace-gap boundary: the exec-drops-caps observation is
  inside the same committed log as the privilege demonstration — the
  claim and its boundary cannot be quoted separately.
- The tracing tier: every primitive's proof includes a consumer on
  the tracee side (the tracee read 4242; the tracee read the injected
  fd), not just tracer-side mechanics.

## Verdict

Two instruments hardened (entry controls; inet6 probes), one
attribution sentence hardened (A1), one claim's wording brought back
inside its evidence (B1). Assume more remain.
