# Review C — consistency router

Lens: does every part of the repo agree with every other part — references,
logs, ignore rules, identity, publishability? Method: programmatic
cross-checks (listed below), each executed during this review. Findings were
fixed in-tree before publishing.

## Cross-reference checks

- **C1 — every path mentioned in README/REPORT/docs resolves.** 28 referenced
  files checked (scripts, patches, notes, logs, docs, flake-demo): all
  present. (Before fixes: README/REPORT implied a `scripts/00-seccomp-probe`
  that actually lives in `notes/` — wording corrected.)
- **C2 — logs carry the claimed markers.** Grepped each claimed outcome in
  its log: `Hello, world!` (04c), `fastfetch 1.12.2` (04c, 05),
  `setgroups failed` (04c), `pty-shim-works-from-execed-child` (06),
  `unshare failed: EPERM` (04a), `ptrace(TRACEME)` + `opening pseudoterminal
  master` (01). All present; final `04c` log is from the clean-room run.

## Inventory & ignore-rule checks

- **C3 — BLOCKER: `git add -A` had swept the Review-A backup
  `chroot/rootfs.prev` into the index: 144,609 tracked files (~370 MiB
  pack), including binary store paths.** Root cause: `.gitignore` covered
  `chroot/rootfs/` but not the `rootfs.prev` sibling. Fix: ignore
  `chroot/rootfs*/`, drop the backup, rebuild the branch as a single clean
  commit (the per-fix history is preserved in the review documents), expire
  reflogs, `gc --prune=now`. Post-fix: 50 tracked files, `.git` = 209 KiB,
  largest file 16 KiB.
- **C4 — built artifact `chroot/nix-pty-shim.so` was staged** while only the
  source belongs in-tree (the create script builds it). Untracked + ignored.

## Content consistency

- **C5 — capability matrix vs docs:** the "modern flakes inside chroot ✗"
  cell could read as "impossible"; re-scoped to "as built" with a pointer to
  the shim blueprint (see Review B).
- **C6 — emoji/typographic claims:** fastfetch output is described as
  proc-less-truthful; the final log shows exactly OS/Kernel/Uptime/Shell/
  Locale + version — matches.
- **C7 — instructions follow-through:** `scripts/chroot-create.sh` +
  `scripts/chroot-run.sh` alone reproduce the headline results (verified in
  Review A's clean room; no manual state required).

## Publishing checks

- Commit identity: `Talaria <324092415+talaria0101@users.noreply.github.com>`
  (repository bot account) — no personal authorship anywhere in history.
- Secret scan over tracked files: no credentials/tokens (hits were binary
  libc/cmake strings from the pre-cleanup index; absent after C3).
- License: 0BSD `LICENSE` added; README links it.
- No pull request opened (publishing was requested; nothing else).

## Verdict

One publish-blocking defect (C3) found and fixed by history rebuild;
everything else consistent. The published tree is the reviewed tree.
