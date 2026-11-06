#!/bin/sh
# Rocky guard run for T-1309 (ignored, not committed). The lane loader
# hashes the default first, so the guard cannot fire there; rocky's
# loader hashes the compat first, so the guard fires there on the
# planted (unversioned) code and passes on the fix. Toolchain via
# rustup (deterministic version), crate mounted live read-only, cargo
# home and target dir on /tmp so the mount stays clean.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
CRATE="C:/Users/AjamX/Downloads/podbox/crates/podbox-interpose"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== interpose unit suite on rocky payload libc"
timeout 1500 podman run --rm -v "$CRATE:/c:ro" "$ROCKY" /bin/sh -c \
  "dnf install -y gcc 2>&1 | tail -5; command -v gcc cc || ls /usr/bin/*cc* 2>&1
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.98.1 2>&1 | tail -1
   export PATH=\"\$HOME/.cargo/bin:\$PATH\" CARGO_HOME=/tmp/ch CARGO_TARGET_DIR=/tmp/t
   rustc -V
   cargo test --manifest-path /c/Cargo.toml 2>&1 | tail -22" \
  >"$W/v14guard.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v14guard.out"
