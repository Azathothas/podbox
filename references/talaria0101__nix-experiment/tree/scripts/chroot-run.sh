#!/usr/bin/env bash
# Run the whole nix experiment inside the chroot. No namespaces, no mounts,
# no VM: plain chroot(2), which this sandbox permits.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=chroot/rootfs
REV="${NIXPKGS_REV:-22.05}"  # last nixpkgs evaluating on nix 2.2.x
exec chroot "$ROOT" /bin/sh -c '
set -e
export PATH=/bin:/usr/bin
export HOME=/root USER=root LOGNAME=root REV='"$REV"'
export NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
cd /

echo "=== sanity ==="
id && uname -a && ls -la /nix/.reginfo

NIXBIN=$(echo /nix/store/*-nix-2.2.2/bin)
export PATH="$NIXBIN:$PATH"
echo "nix 2.2.2 at: $NIXBIN"
nix-store --version 2>&1 | grep -v "GC Warning" || true
nix-instantiate --version 2>&1 | grep -v "GC Warning" || true

echo "=== register binary-tarball closure in the nix DB ==="
nix-store --load-db < /nix/.reginfo && echo DB-REGISTERED

echo "=== nixpkgs-22.05 eval via fetchTarball (needs TLS+DNS) ==="
cat > /root/hello-local.nix <<NIXEOF
with import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/refs/tags/22.05.tar.gz") {};
hello.overrideAttrs (o: { pname = "hello-forced-local"; dontPatchELF = true; dontRewriteSymlinks = true; dontPatchShebangs = true; noAuditTmpdir = true; })
NIXEOF
nix-instantiate /root/hello-local.nix

echo "=== BUILD (local compile; sandbox=false, pipe-based builder output) ==="
nix-build /root/hello-local.nix -o /root/result

echo "=== RUN the artifact inside the chroot ==="
/root/result/bin/hello

echo "=== fastfetch: nix-build from source (cmake) and run ==="
cat > /root/fastfetch.nix <<NIXEOF
# fastfetch built from source with nix 2.2.2 + nixpkgs 22.05 inside this chroot.
# The four dont* attrs skip fixup hooks that use bash process substitution
# (< <(...)), which needs /proc/self/fd - unavailable in this chroot.
with import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/refs/tags/22.05.tar.gz") {};
stdenv.mkDerivation rec {
  pname = "fastfetch";
  version = "1.12.2";
  src = fetchurl {
    url = "https://github.com/fastfetch-cli/fastfetch/archive/refs/tags/\${version}.tar.gz";
    sha256 = "1c3s0b2jxz7v4pckif4661vlj1mljn07w6xyvkznwc5aw16kimz3";
  };
  nativeBuildInputs = [ cmake ];
  buildInputs = [ ];
  cmakeFlags = [ "-DCMAKE_BUILD_TYPE=Release" ];
  dontPatchELF = true;
  dontRewriteSymlinks = true;
  dontPatchShebangs = true;
  noAuditTmpdir = true;
}
NIXEOF
nix-build /root/fastfetch.nix -o /root/fastfetch-result
/root/fastfetch-result/bin/fastfetch --structure OS:Kernel:Uptime:Shell:Locale:Colors
/root/fastfetch-result/bin/fastfetch -v

echo "=== pty-shim PoC: emulated pty with/without shim (needs store patchelf from fastfetch build) ==="
if [ -x /root/ptyharness ]; then
  echo "--- without shim:"
  (LD_LIBRARY_PATH=/hostlib /root/ptyharness 2>&1 | head -1) || true
  echo "--- with shim:"
  LD_PRELOAD=/usr/lib/nix-pty-shim.so LD_LIBRARY_PATH=/hostlib /root/ptyharness
  echo "poc exit: $?"
  echo "--- /proc/self/exe shim on real nix 2.3.18 (if closure present):"
  N18=$(ls -d /nix/store/*-nix-2.3.18/bin 2>/dev/null | head -1)
  if [ -n "$N18" ]; then
    LD_PRELOAD=/usr/lib/nix-pty-shim.so NIX_SHIM_EXE=$N18/nix-instantiate $N18/nix-instantiate --version 2>&1 | grep -v "GC Warning" || true
  else
    echo "(2.3.18 closure not present; skipped)"
  fi
else
  echo "(store patchelf not available yet; skipped)"
fi

echo "=== negative test: sandboxed build must fail (mount ns blocked) ==="
cat > /root/hello-sandbox.nix <<NIXEOF
with import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/refs/tags/22.05.tar.gz") {};
hello.overrideAttrs (o: { pname = "hello-sandbox-attempt"; })
NIXEOF
(nix-build /root/hello-sandbox.nix -o /root/result2 --option sandbox true 2>&1 | tail -4) || echo ">>> sandboxed build failed as expected (mount/unshare EPERM)"
echo "CHROOT-EXPERIMENT-DONE"
'
