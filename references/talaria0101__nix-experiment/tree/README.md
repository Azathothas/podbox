# nix-experiment

Can Nix run in this restricted sandbox (NoNewPrivs, seccomp filter, no
/dev/ptmx, root dir locked down)? Spoiler: yes — with chroot + nix 2.2.2.

**Start reading: [REPORT.md](REPORT.md)**

Layout:
- `REPORT.md` — the whole journey, all findings, final solution
- `scripts/` — idempotent scripts for every attempt (nix-portable patch, VM detour, final chroot); the seccomp probe lives in `notes/`
- `patches/` — byte-size-preserving nix-portable launcher patch
- `logs/` — raw unedited output of every attempt
- `notes/` — seccomp probe source + syscall matrix
- `flake-demo/` — host-side path-flake demo (static nix 2.35.2)
- `reviews/` — three deep reviews (reproducibility, claim-scope, consistency); findings were fixed in-tree

Final solution (no VMs, no namespaces, no mounts — plain `chroot(2)`):

```sh
scripts/chroot-create.sh   # assemble rootfs: nix 2.2.2 official tarball + busybox-static + fake /dev
scripts/chroot-run.sh      # register store DB, fetch nixpkgs-22.05, compile hello, run it -> "Hello, world!"
```

## License

0BSD (see [LICENSE](LICENSE)).
