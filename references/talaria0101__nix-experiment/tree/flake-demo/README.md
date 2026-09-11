Demo from attempt 02: a plain path flake with `inputs.nixpkgs.url = "github:..."`
builds fine with static nix 2.35.2 on the HOST (relocated store, substitute-only;
`--store $PWD/store-demo/nix-store`). `nix build .#hello` produced
`result -> /nix/store/...-hello-2.12.1` (see logs/02-nix-prebuild-*.log).
The artifact itself cannot execute on the host (no /nix path) - run it in the
chroot instead (attempt 04).
