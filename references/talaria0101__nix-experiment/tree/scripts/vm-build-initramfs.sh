#!/usr/bin/env bash
# Build an initramfs containing: busybox-static, static nix (containerbase build),
# virtio-net modules, CA certs. Inside the VM: full nix builds with sandboxing.
set -euo pipefail
cd "$(dirname "$0")/.."
STAGE=vm/stage
OUT=vm/initramfs.cpio.gz
NIX_BIN="${NIX_BIN:-/tmp/nixprebuild/2.35.2/bin/nix}"
NIXPKGS_REV="${NIXPKGS_REV:-ac62194c3917d5f474c1a844b6fd6da2db95077d}"   # nixos-25.05 tip

rm -rf "$STAGE"; mkdir -p "$STAGE"/{bin,dev/pts,etc/nix,etc/ssl/certs,etc/udhcpc,proc,sys,tmp,nix/store,nix/var/nix/db,root,modules,dev}

# busybox + applet symlinks
cp vm/root/bin/busybox.static "$STAGE/bin/busybox"; chmod +x "$STAGE/bin/busybox"
for a in sh ash mount umount insmod ls mkdir mknod mdev ifconfig ip route udhcpc cat echo sleep \
         poweroff hostname cp chmod env grep sed tar gzip true false test rm ln printf uname; do
  ln -sf busybox "$STAGE/bin/$a"
done

# static nix + config
cp "$NIX_BIN" "$STAGE/bin/nix"; chmod +x "$STAGE/bin/nix"
cat > "$STAGE/etc/nix/nix.conf" <<CONF
experimental-features = nix-command flakes
build-users-group =
sandbox = true
max-jobs = 8
CONF

# identity files (nix wants them present)
echo "root:x:0:0:root:/root:/bin/sh"  > "$STAGE/etc/passwd"
echo "root:x:0:"                      > "$STAGE/etc/group"
echo "nixvm"                          > "$STAGE/etc/hostname"
echo "nameserver 10.0.2.3"            > "$STAGE/etc/resolv.conf"

# udhcpc hook (busybox ships no default script)
cat > "$STAGE/etc/udhcpc.script" <<'EOS'
#!/bin/sh
case "$1" in
  deconfig) ifconfig "$interface" 0.0.0.0 up ;;
  bound|renew)
    ifconfig "$interface" "$ip" netmask "${subnet:-255.255.255.0}" up
    [ -n "${router:-}" ] && route add default gw "$router" dev "$interface"
    ;;
esac
EOS
chmod +x "$STAGE/etc/udhcpc.script"

# CA bundle for TLS
cp /etc/ssl/certs/ca-certificates.crt "$STAGE/etc/ssl/certs/"

# virtio-net module + deps
for m in kernel/net/core/failover.ko.gz kernel/drivers/net/net_failover.ko.gz kernel/drivers/net/virtio_net.ko.gz; do
  cp "vm/lib/modules/6.12.109-0-virt/$m" "$STAGE/modules/"
done

# /init — the whole experiment runs here
cat > "$STAGE/init" <<EOI
#!/bin/sh
export PATH=/bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
mkdir -p /dev/pts /dev/shm /tmp /nix/store /nix/var/nix/db /root
mount -t devpts devpts /dev/pts
mount -t tmpfs tmpfs /tmp
insmod /modules/failover.ko.gz
insmod /modules/net_failover.ko.gz
insmod /modules/virtio_net.ko.gz
ifconfig lo 127.0.0.1 up
hostname -F /etc/hostname
echo "[vm] waiting for eth0..."
n=0; while [ \$n -lt 20 ]; do ip link show eth0 >/dev/null 2>&1 && break; sleep 0.5; n=\$((n+1)); done
udhcpc -i eth0 -s /etc/udhcpc.script -n -q
echo "[vm] network: \$(ip -4 addr show eth0 | grep inet)"
export NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
export NIXPKGS_REV=$NIXPKGS_REV

echo "=== PROBE: sandbox facilities inside VM (all FAIL on the host) ==="
unshare -m true 2>/dev/null && echo "VM: mount-ns      OK" || echo "VM: mount-ns      FAIL"
unshare -U true 2>/dev/null  && echo "VM: userns        OK" || echo "VM: userns        FAIL"
test -e /dev/ptmx            && echo "VM: /dev/ptmx     present" || echo "VM: /dev/ptmx     MISSING"
echo "VM: seccomp status: \$(grep Seccomp /proc/self/status)"

echo "=== TEST 1: nix --version ==="
/bin/nix --version

echo "=== TEST 2: direct flake ref, substitute-or-build nixpkgs#hello ==="
mkdir -p /root/demo && cd /root/demo
cat > flake.nix <<'FLAKE'
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;
    packages.x86_64-linux.hello-local =
      nixpkgs.legacyPackages.x86_64-linux.hello.overrideAttrs (o: { name = "hello-local-2.12.1"; doCheck = false; });
    packages.x86_64-linux.default = self.packages.x86_64-linux.hello;
  };
}
FLAKE
cp flake.nix /root/demo/keep-flake.nix
/bin/nix build .#hello -L && echo "TEST2-BUILD-OK" && ./result/bin/hello | head -1

echo "=== TEST 3: forced LOCAL SANDBOXED BUILD (overrideAttrs -> not in cache) ==="
/bin/nix build .#hello-local -L && echo "TEST3-BUILD-OK" && ./result/bin/hello | head -1

echo "=== TEST 4: store diagnostics ==="
/bin/nix store info 2>/dev/null | head -12

echo "ALL-DONE"
poweroff -f
EOI
chmod +x "$STAGE/init"

# device nodes for the early console
mknod -m 622 "$STAGE/dev/console" c 5 1 2>/dev/null || true
mknod -m 666 "$STAGE/dev/null"    c 1 3 2>/dev/null || true

(cd "$STAGE" && find . | cpio -o -H newc --quiet | gzip -9) > "$OUT"
ls -la "$OUT"
