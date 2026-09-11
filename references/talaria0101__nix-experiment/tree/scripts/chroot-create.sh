#!/usr/bin/env bash
# Assemble a minimal rootfs hosting a REAL /nix:
#  - official nix 2.3.18 binary tarball (glibc-dynamic; last nix whose builder
#    output uses a pipe instead of a pty -> builds work without /dev/ptmx)
#  - containerbase static nix 2.35.2 (modern eval/substitute; no store libs)
#  - busybox-static (musl, static) for shell/coreutils
# The chroot makes /nix/... paths real; host root dir restrictions (no mkdir,
# no readdir, no /dev/ptmx) never apply inside.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=chroot/rootfs
TARBALL=chroot/dl/nix-2.2.2-x86_64-linux.tar.bz2
mkdir -p chroot/dl "$ROOT"/{bin,usr/bin,etc/nix,etc/ssl/certs,dev,proc,sys,tmp,root,nix,var,store235}

# --- busybox-static ----------------------------------------------------------
if [ ! -f chroot/dl/bin/busybox.static ]; then
  curl -sL -o chroot/dl/busybox-static.apk \
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/main/x86_64/$(curl -s https://dl-cdn.alpinelinux.org/alpine/v3.22/main/x86_64/ \
      | grep -oE 'busybox-static-[^"<>]*\.apk' | head -1)"
  tar xzf chroot/dl/busybox-static.apk -C chroot/dl bin/busybox.static \
    --warning=no-unknown-keyword 2>/dev/null
fi
cp chroot/dl/bin/busybox.static "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
for a in sh ash ls cat cp rm ln mkdir mknod chmod env grep sed tar uname id printf test true false tail head wc sleep setsid ps; do
  ln -sf busybox "$ROOT/bin/$a"
done

# --- official nix 2.2.2 binary tarball --------------------------------------
if [ ! -f "$TARBALL" ]; then
  echo "downloading nix 2.2.2 binary tarball..."
  curl -sL -o "$TARBALL" "https://nixos.org/releases/nix/nix-2.2.2/nix-2.2.2-x86_64-linux.tar.bz2"
fi
rm -rf "$ROOT/nix"
mkdir -p "$ROOT/nix"
tar xjf "$TARBALL" -C "$ROOT/nix" --strip-components=1 nix-2.2.2-x86_64-linux/store
tar xjOf "$TARBALL" nix-2.2.2-x86_64-linux/.reginfo > "$ROOT/nix/.reginfo"
echo "store paths extracted: $(ls "$ROOT/nix/store" | wc -l)"

# --- containerbase static nix 2.35.2 -----------------------------------------
if [ ! -f /tmp/nixprebuild/2.35.2/bin/nix ]; then
  mkdir -p /tmp/nixprebuild
  tar xJf nix-prebuild-2.35.2-x86_64.tar.xz -C /tmp/nixprebuild --no-same-owner
fi
cp /tmp/nixprebuild/2.35.2/bin/nix "$ROOT/usr/bin/nix"; chmod 755 "$ROOT/usr/bin/nix"

# --- scaffolding -------------------------------------------------------------
echo "root:x:0:0:root:/root:/bin/sh" > "$ROOT/etc/passwd"
echo "root:x:0:"                     > "$ROOT/etc/group"
cp /etc/resolv.conf                  "$ROOT/etc/resolv.conf"
printf "hosts: files dns\n"        > "$ROOT/etc/nsswitch.conf"
cat > "$ROOT/etc/nix/nix.conf" <<CONF
build-users-group =
sandbox = false
binary-caches = https://cache.nixos.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
CONF
cp /etc/ssl/certs/ca-certificates.crt "$ROOT/etc/ssl/certs/"
chmod 1777 "$ROOT/tmp"

# /dev entries: seccomp allows mknod only for char 0:0 (overlay whiteout),
# so real device nodes for null/zero/random are impossible. Regular files
# stand in for them:
#   null, zero, full, tty, console -> empty files (reads=EOF, writes append;
#     good enough for shell/gcc/make in practice)
#   random, urandom -> 4 KiB of genuine host entropy. nix 2.2.2's libstdc++
#     (gcc-7.3) std::random_device opens /dev/urandom via fopen() and reads
#     a few bytes; openssl RAND_poll likewise. An EMPTY file made them throw
#     "random_device could not be read". A finite pool avoids that. Not
#     cryptographically sound (same bytes every open, 4 KiB cap) but fine
#     here: everything security-critical on modern glibc/openssl uses
#     getrandom(2), which this kernel/seccomp allows.
for f in null zero full tty console; do
  [ -e "$ROOT/dev/$f" ] || : > "$ROOT/dev/$f"
done
dd if=/dev/urandom of="$ROOT/dev/urandom" bs=4096 count=1 status=none
cp "$ROOT/dev/urandom" "$ROOT/dev/random"
chmod 666 "$ROOT/dev/"*

# --- pty-shim prototype tooling (optional diagnostics; see docs/latest-nix-shim.md)
if command -v gcc >/dev/null 2>&1; then
  mkdir -p "$ROOT/usr/lib" "$ROOT/hostlib" "$ROOT/root"
  gcc -shared -fPIC -O2 -o "$ROOT/usr/lib/nix-pty-shim.so" scripts/nix-pty-shim.c -lpthread || true
  gcc -O0 -o "$ROOT/root/ptyharness" scripts/ptyharness.c || true
  mkdir -p "$ROOT/hostlib"
  cp -L /lib64/ld-linux-x86-64.so.2 "$ROOT/hostlib/" 2>/dev/null || true
  cp -L /lib64/libc.so.6 "$ROOT/hostlib/" 2>/dev/null || true
  for l in $(ldd "$ROOT/root/ptyharness" | awk '{print $3}' | grep -E "lib(m|dl|pthread)"); do
    cp -L "$l" "$ROOT/hostlib/" 2>/dev/null || true
  done
  if command -v patchelf >/dev/null 2>&1; then
    patchelf --set-interpreter /hostlib/ld-linux-x86-64.so.2 "$ROOT/root/ptyharness"
    patchelf --set-rpath /hostlib "$ROOT/root/ptyharness"
  fi
  echo "pty-shim + harness deployed"
fi
# optional: nix 2.3.18 closure side-by-side (for the shim PoC on a real pty-era nix)
T2318=chroot/dl/nix-2.3.18-x86_64-linux.tar.xz
if [ -f "$T2318" ]; then
  tar xJf "$T2318" -C chroot/dl nix-2.3.18-x86_64-linux/store 2>/dev/null
  cp -r chroot/dl/nix-2.3.18-x86_64-linux/store/* "$ROOT/nix/store/"
  echo "nix 2.3.18 closure deployed (side-by-side, no db registration)"
fi

echo "rootfs ready at $ROOT:"
du -sh "$ROOT"
