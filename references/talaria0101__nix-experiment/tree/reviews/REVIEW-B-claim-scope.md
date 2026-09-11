# Review B — claim scope

Lens: for every non-trivial claim in REPORT.md / docs/, is it *stated at the
scope the evidence supports*? Method: each claim was traced to either an
executable check run during this review (output quoted), the raw logs, or was
re-scoped/re-worded. Not a summary: corrections were applied to the documents
in this same commit.

## Claims verified during this review (evidence)

| Claim | Check | Result |
|---|---|---|
| nixpkgs minver gates: 22.05 accepts 2.2; 22.11 requires 2.3; 23.11 requires 2.3 | fetched `lib/minver.nix` at each tag | `"2.2"` / `"2.3"` / `"2.3"` — matches REPORT |
| Nix 2.2.2 is pipe-based, 2.3.x is pty-based | `grep -c posix_openpt` on `src/libstore/build.cc` at tags 2.2.2 and 2.3.10 | 0 vs 1 — matches |
| 23.11 lib needs Nix ≥ 2.4 regardless of its "2.3" gate | read `lib/strings.nix` line ~31: `inherit (builtins) … isPath …` | matches REPORT's fastfetch note |
| chroot(2) permitted; unshare/mount/ptrace/setgroups blocked | re-ran seccomp probe + chroot probe | matches notes |
| mknod restricted to char 0:0 | re-swept 24 majors at minor 0 | only 0:0 — scope in REPORT correctly limited to majors probed |
| nix 2.2.2 tarball still served | `curl -I nixos.org/releases/nix/nix-2.2.2/…` | 200 |
| 2.3.18 tarball via Hydra product | re-resolved via API during Review A | same store path `y6iffbh…` |
| "2.3.18 build with shim executed the builder and produced correct output" | output file `zflpw…-pty-shim-poc`, 15 bytes `pty-shim-works` | true; scope correctly excludes goal completion |
| shim harness result | logs/06 + in-run section | without shim ENOENT, with shim full flow, exit 0 |

## Corrections applied (claims that outran the evidence)

| # | Claim (before) | Problem | Correction (after) |
|---|---|---|---|
| B1 | "the perl hook inherits the goal's pty fds" (docs §3.2) | `HookInstance` has its *own* pipe (`HookInstance::builderOut`), not the goal pty; the actor closing the master in the PoC logs was not positively identified | reworded to "a short-lived forked helper … exact role not fully disambiguated", with the HookInstance fact noted |
| B2 | W3 stated as verified breakage ("a 2.35 store visit permanently upgrades the DB and older Nix refuses") | never tested; an attempt during this review was inconclusive (WAL/registration subtleties) | reworded to design rationale: schema migration is one-way by design; per-version chroots sidestep it; breakage not independently tested |
| B3 | "22.05 is the last Nixpkgs that accepts Nix 2.2" | true only across release *branches*; minver evidence added | reworded + upstream minver values quoted inline |
| B4 | bare-flake-ref failure attributed to a specific code path | mechanism inferred; exact throw site not pinned (no ptrace) | marked "consistent with …; exact throw site not pinned" |
| B5 | capability matrix cell "modern flakes inside chroot ✗ (static 2.35 …)" | could read as "impossible" | now says "as built" and points at the dynamic+shim blueprint |

## Claims deliberately *not* made (and correctly absent)

- No claim that sandboxed builds can work (they cannot under this filter).
- No claim that the shim makes latest Nix fully functional — §3 lists the
  open goal-completion integration explicitly.
- No claim about `urandom` cryptographic quality — flagged as a finite pool.

## Verdict

All checked claims are now scoped to their evidence. The two strongest
statements in the repo — "nix 2.2.2 + nixpkgs 22.05 is the newest release
pair that works end-to-end here" and "the shim emulates the pty API nix
needs" — are each backed by a from-scratch re-derivation (Review A) and the
PoC log respectively.
