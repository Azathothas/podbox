#!/bin/sh
# Question: under TCG with user-mode networking, does a guest built from
# a podbox-extracted rootfs reach the host through UDP, and does the
# host reach the guest through a forwarded port?
#
# TODO/podvm.md T-1306 (the one promotion: guest networking).
#
# Clauses over pinned inputs:
#   0. conditions: binary, kernel and modloop hashes, image digest,
#      qemu, guest tools.
#   1. pull and extract the pinned image through the lane-built binary.
#   2. the guest applets are in the image's busybox (ip, nc, sleep,
#      insmod and friends): read out of the binary's strings, not
#      assumed.
#   3. the kernel and the modloop fetch against their pins: the netboot
#      kernel keeps NIC drivers in the modloop, so virtio_net arrives
#      as a module rather than an assumption.
#   4. the assembly: base archive, an extras archive carrying the
#      console node and a /init that loads the modules, brings up the
#      first non-loopback interface, sends its token to the host,
#      waits for the reply, prints what arrived, powers off, plus a
#      modules archive beside the extras one.
#   5. guest to host: the token arrives at the host listener, which is
#      up before the boot on a port it chose itself.
#   6. host to guest: the reply forwarded into the guest is printed on
#      the guest console.
#
# Exit: 0 both datagrams crossed, 1 the assembly or either direction
# failed, 2 a tool, the binary or an input could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/guest-usernet.txt"
WORK="$REPO/experiments/.sweep361-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

# shellcheck source=lib/podvm-guest.sh
. "$HERE/lib/podvm-guest.sh"

ALPINE_REF='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
MURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/modloop-virt"
MODLOOP_SHA256="9a5252fa3e49f908fa7bc796a38b7ce57c9a930c91670324278a352effa5c30f"
BOOT_TIMEOUT="${PODBOX_361_TIMEOUT:-240}"
CURL_TIMEOUT="${PODBOX_361_CURL_TIMEOUT:-60}"

fail=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
miss() { fail=1; say "  FAIL: $1"; }
ok() { say "  ok: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	say "qemu present"
else
	say "qemu absent: installing qemu-system-x86"
	if timeout 600 apt-get update >>"$WORK/apt.log" 2>&1 \
		&& timeout 1200 apt-get install -y qemu-system-x86 cpio squashfs-tools >>"$WORK/apt.log" 2>&1; then
		say "qemu installed"
	else
		echo "SKIP: qemu install failed" >&2
		exit 2
	fi
fi
for t in cpio python3 curl sha256sum timeout unsquashfs; do
	command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'qemu              %s\n' "$(qemu-system-x86_64 --version | head -1)"
	printf 'image             %s\n' "$ALPINE_REF"
	printf 'kernel            %s (sha256 %s)\n' "$KURL" "$KERNEL_SHA256"
	printf 'modloop           %s (sha256 %s)\n' "$MURL" "$MODLOOP_SHA256"
	echo
} >"$WORK/report"

say "== 1. pull and extract the pinned image"
if ! "$BIN" pull "$ALPINE_REF" >"$WORK/pull.log" 2>&1; then
	say "  no-pull (base image):"
	tail -3 "$WORK/pull.log" >>"$WORK/report"
	miss "the pull failed"
else
	ROOTFS="$("$BIN" extract "$ALPINE_REF" 2>"$WORK/extract.log")"
	if [ -z "$ROOTFS" ] || [ ! -d "$ROOTFS" ]; then
		tail -3 "$WORK/extract.log" >>"$WORK/report"
		miss "extract printed no rootfs directory"
	else
		ok "rootfs at $ROOTFS"
	fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 2. the guest applets are in the image's busybox"
# Read out of the binary's own string table, not assumed: applet names
# ride as NUL-separated words, so non-alphanumerics become newlines and
# whole-line match decides. No binutils needed.
if [ ! -f "$ROOTFS/bin/busybox" ]; then
	miss "no busybox in the rootfs"
else
	for app in ip nc sleep poweroff mount head cat insmod dmesg zcat ls grep; do
		if tr -c '[:alnum:]' '\n' <"$ROOTFS/bin/busybox" 2>/dev/null | grep -qx "$app"; then
			ok "applet present: $app"
		else
			miss "applet missing from busybox: $app"
		fi
	done
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 3. the kernel and the modloop fetch against their pins"
if ! curl -fsSL --max-time "$CURL_TIMEOUT" -o "$WORK/vmlinuz-virt" "$KURL"; then
	miss "the kernel did not fetch"
else
	printf '%s  %s\n' "$KERNEL_SHA256" "$WORK/vmlinuz-virt" | sha256sum -c - >>"$WORK/report" 2>&1 || {
		miss "the kernel hash does not match the pin"
	}
fi
# The netboot kernel keeps NIC drivers in the modloop, not in the
# image: virtio_pci binds (seen on the console) but no interface
# appears, because virtio_net is a module. The pin below was measured
# on the lane the way the kernel pin was, not inherited.
if ! curl -fsSL --max-time "$CURL_TIMEOUT" -o "$WORK/modloop-virt" "$MURL"; then
	miss "the modloop did not fetch"
else
	printf '%s  %s\n' "$MODLOOP_SHA256" "$WORK/modloop-virt" | sha256sum -c - >>"$WORK/report" 2>&1 || {
		miss "the modloop hash does not match the pin"
	}
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 4. the assembly with the networking /init"
podvm_base "$ROOTFS" "$WORK/base.cpio" || miss "the base cpio did not build"
# Only the modules the guest loads, in dependency order (failover,
# net_failover, virtio_net: the kernel refuses unknown symbols
# otherwise, measured link by link); the whole modloop unpacked is
# a hundred megabytes the allowance does not owe. The sed strips the
# squashfs-root/ display prefix -l prints: passed through, the extract
# "succeeds" writing nothing (measured: 0 files, 1 directory).
MODPATHS="$(unsquashfs -l "$WORK/modloop-virt" 2>/dev/null | grep -E '/(virtio_net|virtio_ring|net_failover|failover)\.ko$' | sed 's|^squashfs-root/||' || true)"
if [ -z "$MODPATHS" ]; then
	miss "no virtio_net.ko in the modloop"
	unsquashfs -l "$WORK/modloop-virt" 2>/dev/null | grep -E 'drivers/(net|virtio)/' | head -10 >>"$WORK/report" || true
else
	say "  modloop carries: $(printf '%s' "$MODPATHS" | tr '\n' ' ')"
	mkdir -p "$WORK/modules/lib/modules" || exit 2
	for mp in $MODPATHS; do
		if unsquashfs -f -d "$WORK/modules" "$WORK/modloop-virt" "$mp" >>"$WORK/report" 2>&1; then
			ok "extracted $mp"
		else
			miss "could not extract $mp"
		fi
	done
	# Found by name, not by versioned path: the release's module
	# directory carries the kernel version, which the pin does not.
	find "$WORK/modules" \( -name 'virtio_net.ko' -o -name 'virtio_ring.ko' -o -name 'net_failover.ko' -o -name 'failover.ko' \) \
		-exec cp {} "$WORK/modules/lib/modules/" \; 2>/dev/null || true
	if [ -f "$WORK/modules/lib/modules/virtio_net.ko" ]; then
		ok "virtio_net.ko staged for the guest"
	else
		miss "virtio_net.ko did not land in the staging"
		find "$WORK/modules" -name '*.ko' | head -10 >>"$WORK/report" || true
	fi
fi
# Ephemeral ports, chosen by binding, not by guessing: the listener
# owns LPORT for the whole run, HPORT is bound to prove it is free.
cat >"$WORK/ports.py" <<'PYEOF'
import socket
a = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
a.bind(("0.0.0.0", 0))
b = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
b.bind(("127.0.0.1", 0))
print(a.getsockname()[1], b.getsockname()[1], flush=True)
PYEOF
PORTS="$(python3 "$WORK/ports.py")"
LPORT="${PORTS%% *}"
HPORT="${PORTS##* }"
GPORT=4099
say "  host listener port $LPORT, forwarded host port $HPORT, guest port $GPORT"
cat >"$WORK/init-override" <<INITEOF
#!/bin/sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
echo "NET-DEVS: \$(ls /sys/class/net 2>/dev/null | tr '\n' ' ')"
echo "NET-CFG: \$(zcat /proc/config.gz 2>/dev/null | grep -E 'VIRTIO_NET|E1000|8139' | tr '\n' ' ')"
for ko in /lib/modules/failover.ko /lib/modules/net_failover.ko /lib/modules/virtio_ring.ko /lib/modules/virtio_net.ko; do
	if [ -f "\$ko" ]; then
		echo "MOD-TRY: \$ko"
		insmod "\$ko" && echo "MOD-OK: \$ko" || echo "MOD-FAIL: \$ko rc=\$?"
	fi
done
echo "MOD-DMESG: \$(dmesg 2>/dev/null | grep -iE 'failover|virtio' | tr '\n' '|')"
DEV=""
for d in /sys/class/net/*; do
	[ -d "\$d" ] || continue
	b="\${d##*/}"
	[ "\$b" = lo ] && continue
	DEV="\$b"; break
done
if [ -z "\$DEV" ]; then
	echo "NET-NODEV: no non-loopback interface; virtio lines follow"
	dmesg 2>/dev/null | grep -i virtio | head -5
else
	ip link set \$DEV up
	ip addr add 10.0.2.15/24 dev \$DEV
	ip route add default via 10.0.2.2
fi
echo VMR-GUEST-READY
nc -l -u -p $GPORT >/in.txt 2>/dev/null &
echo "VMR-NET-PROBE" | nc -u -w5 10.0.2.2 $LPORT
i=0
while [ \$i -lt 30 ]; do
	grep -q VMR-NET-REPLY /in.txt 2>/dev/null && break
	sleep 1
	i=\$((i + 1))
done
echo "GUEST-GOT:"
cat /in.txt 2>/dev/null
poweroff -f
INITEOF
podvm_extras "$WORK/init-override" "$WORK/extras.cpio" || miss "the extras archive did not build"
# The modules ride as a third archive beside the extras one: the
# shared lib packs exactly one /init, and this stays its caller.
cat >"$WORK/mkmodules.py" <<'PYEOF'
import glob, os, sys
work = sys.argv[1]
def newc(name, mode, data=b""):
    name_b = name.encode() + b"\0"
    head = ["070701", "00000000", format(mode, "08x"), "00000000",
            "00000000", "00000001", "00000000", format(len(data), "08x"),
            "00000000", "00000000", "00000000", "00000000",
            format(len(name_b), "08x"), "00000000"]
    rec = "".join(head).encode() + name_b
    rec += b"\0" * ((4 - len(rec) % 4) % 4)
    rec += data + b"\0" * ((4 - len(data) % 4) % 4)
    return rec
rec = newc("lib", 0o040755)
rec += newc("lib/modules", 0o040755)
kos = sorted(glob.glob(os.path.join(work, "modules/lib/modules/*.ko")))
for ko in kos:
    with open(ko, "rb") as fh:
        rec += newc("lib/modules/" + os.path.basename(ko), 0o100644, fh.read())
rec += newc("TRAILER!!!", 0)
open(os.path.join(work, "modules.cpio"), "wb").write(rec)
print(len(kos))
PYEOF
NKO="$(python3 "$WORK/mkmodules.py" "$WORK")"
say "  modules archive: $NKO objects"
[ "$NKO" -ge 1 ] || miss "the modules archive is empty"
podvm_concat "$WORK/base.cpio" "$WORK/extras.cpio" "$WORK/stage.cpio" || miss "concat failed"
podvm_concat "$WORK/stage.cpio" "$WORK/modules.cpio" "$WORK/full.cpio" || miss "concat failed"
say "  full: $(wc -c <"$WORK/full.cpio") bytes"
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 5+6. boot with user-mode networking; datagrams both ways"
cat >"$WORK/listen.py" <<PYEOF
import socket, sys
lport, hport = int(sys.argv[1]), int(sys.argv[2])
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.bind(("0.0.0.0", lport))
s.settimeout(120)
try:
    data, _ = s.recvfrom(1024)
except socket.timeout:
    print("TIMEOUT waiting for the guest token")
    sys.exit(1)
print("HOST-GOT:", data.decode(errors="replace").strip(), flush=True)
out = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
out.sendto(b"VMR-NET-REPLY", ("127.0.0.1", hport))
print("HOST-SENT reply", flush=True)
PYEOF
python3 "$WORK/listen.py" "$LPORT" "$HPORT" >"$WORK/net.log" 2>&1 &
LISTENPID=$!
timeout "$BOOT_TIMEOUT" qemu-system-x86_64 \
	-M pc,acpi=off -m 256 -nographic -no-reboot \
	-accel tcg,thread=multi \
	-kernel "$WORK/vmlinuz-virt" \
	-initrd "$WORK/full.cpio" \
	-append "console=ttyS0 panic=-1" \
	-netdev "user,id=u0,hostfwd=udp::$HPORT-:$GPORT" \
	-device virtio-net-pci,netdev=u0 \
	>"$WORK/boot.log" 2>&1
say "  qemu rc=$? (124 is the halt the spec records: poweroff halts on pc,acpi=off)"
wait "$LISTENPID"
say "  listener rc=$? (0 is the token received and the reply sent)"
grep -aq "VMR-GUEST-READY" "$WORK/boot.log" && ok "the guest printed VMR-GUEST-READY" || miss "no ready marker in the boot log"
if grep -aq "HOST-GOT: VMR-NET-PROBE" "$WORK/net.log"; then
	ok "clause 5: the guest token reached the host"
else
	miss "clause 5: the guest token never arrived"
	tail -5 "$WORK/net.log" >>"$WORK/report"
fi
if grep -aq "GUEST-GOT:" "$WORK/boot.log" && grep -A2 -a "GUEST-GOT:" "$WORK/boot.log" | grep -aq "VMR-NET-REPLY"; then
	ok "clause 6: the forwarded reply reached the guest console"
else
	miss "clause 6: the guest printed no reply"
	tail -8 "$WORK/boot.log" >>"$WORK/report"
	grep -a -E 'MOD-|NET-' "$WORK/boot.log" | head -20 >>"$WORK/report" || true
fi
fi

say ""
if [ "$fail" -eq 0 ]; then
	say "361 acceptance: datagrams crossed both ways under TCG user-mode networking"
else
	say "361 acceptance: FAILED"
fi
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
