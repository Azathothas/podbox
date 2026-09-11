#!/usr/bin/env bash
# Optional: fetch the nix 2.3.18 binary tarball (pty-era closure, used only by
# the shim PoC). The tarball is no longer on nixos.org/releases; it lives as a
# Hydra build product in the binary cache. Resolve the store path from the
# Hydra API and substitute it with the static 2.35 nix (pure download).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p chroot/dl
HASH=$(curl -sL "https://hydra.nixos.org/job/nix/maintenance-2.3/binaryTarball.x86_64-linux/latest" \
  -H "Accept: application/json" \
  | python3 -c "import json,sys; b=json.load(sys.stdin); print(list(b['buildproducts'].values())[0]['path'].split('/')[3].split('-')[0])")
echo "store path: /nix/store/$HASH-nix-binary-tarball-2.3.18"
export NIX_CONFIG="experimental-features = nix-command flakes build-users-group ="
STORE="$PWD/store-demo/nix-store"; mkdir -p "$STORE"
NIX=/tmp/nixprebuild/2.35.2/bin/nix
[ -x "$NIX" ] || { mkdir -p /tmp/nixprebuild; tar xJf nix-prebuild-2.35.2-x86_64.tar.xz -C /tmp/nixprebuild --no-same-owner; }
"$NIX" --store "$STORE" build /nix/store/$HASH-nix-binary-tarball-2.3.18
cp "$STORE/nix/store/$HASH-nix-binary-tarball-2.3.18/nix-2.3.18-x86_64-linux.tar.xz" chroot/dl/
ls -la chroot/dl/nix-2.3.18-x86_64-linux.tar.xz
