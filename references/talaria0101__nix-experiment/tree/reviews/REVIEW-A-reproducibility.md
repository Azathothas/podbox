# Review A — reproducibility & re-derivation

Lens: can every load-bearing claim be re-derived from the committed state, from
scratch, by a stranger? Method: clean-room re-run of the full pipeline
(fresh rootfs via `scripts/chroot-create.sh`, fresh store, full
`scripts/chroot-run.sh`), plus independent re-execution of the primitive
probes. Not a summary: each finding below was *executed* during this review.

## Procedure

```
mv chroot/rootfs chroot/rootfs.prev          # remove prior state
scripts/fetch-nix-2.3-tarball.sh             # re-derive the Hydra product fetch
scripts/chroot-create.sh                     # fresh rootfs from committed script
scripts/chroot-run.sh                        # full pipeline, fresh store
```

(logs/04-chroot-create.log, logs/04c-chroot-run.log — final versions in this
commit are from the clean-room run)

## Findings & fixes applied during this review

| # | Finding | Severity | Fix |
|---|---------|----------|-----|
| A1 | `chroot-create.sh` required `chroot/dl/nix-2.2.2-x86_64-linux.tar.bz2` but had no step to obtain it (worked only with leftover state) | blocker | auto-download from `nixos.org/releases/nix/nix-2.2.2/` (re-verified HTTP 200 during the run) |
| A2 | `fetch-nix-2.3-tarball.sh` parsed the Hydra product path with the wrong index (`/nix/store` → HASH="store") | blocker | `split('/')[3].split('-')[0]`; re-run re-fetched the 27 MB product from cache.nixos.org |
| A3 | `/root/fastfetch.nix` was hand-placed, not materialized by the script | blocker | heredoc embedded in `chroot-run.sh` |
| A4 | embedded heredoc contained an apostrophe ("chroot's"), truncating the single-quoted `sh -c` payload (`error: syntax error … unexpected $end`) | blocker | comment reworded |
| A5 | shim/harness deployment aborted on missing `usr/lib` dir (`set -e`) | minor | `mkdir -p` up front + `\|\| true` on optional gcc steps; re-run against live rootfs deploys cleanly |
| A6 | shim PoC section depended on the store's patchelf (only present after the fastfetch build) and manual host patchelf | minor | harness is now patched by `patchelf` at create time (host patchelf exists: `/usr/bin/patchelf`); run-script section simplified to execution |

## Re-derivation results (clean room, this review)

- fresh rootfs: 30 store paths (2.2.2 closure) + 2.3.18 side-by-side + shim +
  patched harness — all from committed scripts
- `nix-store --load-db < /nix/.reginfo` → DB-REGISTERED
- `fetchTarball` nixpkgs-22.05 over TLS → eval → `hello-forced-local-2.12.drv`
- **local compile** (autoconf/make, testsuite 5 PASS / 2 SKIP) →
  `Hello, world!`
- **fastfetch 1.12.2** from source → runs, prints version
- pty-shim PoC: without shim `posix_openpt: No such file or directory`; with
  shim `emulated slave path = /dev/.nix-pty-emu/0` →
  `pty-shim-works-from-execed-child` → EOF → `poc exit: 0`
- `/proc/self/exe` shim: `nix-instantiate (Nix) 2.3.18` prints (previously
  aborted)
- negative test: sandboxed build → `setgroups failed: Operation not permitted`
  (unchanged, expected)

## Primitive re-verification

- seccomp probe re-compiled and re-run: `unshare/mount/ptrace/setgroups` EPERM;
  `chroot` OK; matches `notes/seccomp-probe-output.txt`
- `mknod` allowlist re-swept (majors 0,1,2,3,4,5,6,7,9,10,11,13,14,21,29,81,
  108,136,166,180,202,226,234,254,511 at minor 0): only char 0:0 — matches
- nix 2.2.2 tarball URL re-checked: HTTP 200
- Hydra product for 2.3.18 re-resolved: same store path
  (`y6iffbh74kjb2nrp44ivm05ddyqsrswf-…`)

## Verdict

Pipeline is reproducible from the committed state. All found gaps were fixed
in the same commit series; the final clean-room log in `logs/04c-chroot-run.log`
is from the fixed scripts.
