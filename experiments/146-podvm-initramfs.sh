#!/usr/bin/env bash
# Question: does a podbox-extracted rootfs boot under TCG when wrapped as an
# initramfs with an appended console node, and print its ready marker?
#
# TODO/podvm.md T-1303 (the image wrapping; the emulator driver is T-1302's
# flag and T-1304's protocol, and neither is driven here).
#
# Clauses over pinned inputs:
#   0. the conditions: binary, kernel hash, image digest, guest tools;
#   1. pull and extract the pinned image through the lane-built binary, which
#      prints the rootfs path;
#   2. the base archive: the rootfs as newc cpio, uncompressed;
#   3. the appended archive: dev/, dev/console 5:1 and the /init override,
#      written by this script's own newc writer. No host mknod and no
#      privilege: the record is bytes, and archives concatenate;
#   4. the console node is in the appended archive (cpio -t stops at the
#      base archive's TRAILER while the kernel's unpacker keeps going, so
#      the boot in clause 5 is what proves the full assembly);
#   5. the pinned kernel boots the assembly under TCG to VMR-GUEST-READY.
#
# Runs on Linux, native or in a job container: qemu, cpio and python3 come
# from the guest package manager and are recorded in the conditions. On a
# Windows host run it inside the base with PODBOX_BIN set to a guest build
# artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash.
#
# Exit: 0 the guest printed the marker and the archive carries the node,
# 1 the assembly or the boot failed, 2 a tool, the binary or an input
# could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/podvm-initramfs.txt"
WORK="$REPO/experiments/.sweep146-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

# ⭐ Every input pinned. A registry that moves a tag is a different
# measurement wearing the same name.
ALPINE_REF='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
BOOT_TIMEOUT="${PODBOX_146_TIMEOUT:-180}"

fail=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
miss() { fail=1; say "  FAIL: $1"; }
ok() { say "  ok: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
for t in qemu-system-x86_64 cpio python3 curl sha256sum timeout; do
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
	printf 'boot timeout      %s s\n' "$BOOT_TIMEOUT"
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
		# ⛔ Resolved as the guest resolves it, not as the host does. The
		# image's links are absolute (`sh -> /bin/busybox`), so a host
		# `-x` follows them out of the rootfs and reports the host's
		# filesystem instead of the guest's.
		shtarget="$ROOTFS/bin/sh"
		if [ -L "$shtarget" ]; then
			link="$(readlink "$shtarget")"
			case "$link" in
			/*) shtarget="$ROOTFS$link" ;;
			*) shtarget="$ROOTFS/bin/$link" ;;
			esac
		fi
		if [ -x "$shtarget" ]; then
			ok "the rootfs carries a guest-executable /bin/sh"
		else
			miss "no guest-executable /bin/sh in the rootfs"
			say "  rootfs/bin lists as:"
			ls -la "$ROOTFS/bin" 2>&1 | head -8 >>"$WORK/report"
			say "  extract said:"
			tail -5 "$WORK/extract.log" >>"$WORK/report"
		fi
	fi
fi
[ "$fail" -eq 0 ] || say "  the image never arrived, so clauses 2 to 5 do not run"
if [ "$fail" -eq 0 ]; then
say ""
say "== 2. the base archive"
(cd "$ROOTFS" && find . -print0 | cpio --quiet -o -H newc --null >"$WORK/base.cpio") || {
	miss "the base cpio did not build"
}
say "  base: $(wc -c <"$WORK/base.cpio") bytes"

say ""
say "== 3. the appended archive: console node and init override"
cat >"$WORK/init-override" <<'INITEOF'
#!/bin/sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
echo VMR-GUEST-READY
poweroff -f
INITEOF
python3 - "$WORK" <<'PYEOF'
import sys
d = sys.argv[1]
def newc(name, mode, filesize=0, rdevmaj=0, rdevmin=0, data=b""):
    name_b = name.encode() + b"\0"
    head = ["070701", "00000000", format(mode, "08x"), "00000000",
            "00000000", "00000001", "00000000", format(filesize, "08x"),
            "00000000", "00000000", format(rdevmaj, "08x"),
            format(rdevmin, "08x"), format(len(name_b), "08x"), "00000000"]
    out = "".join(head).encode() + name_b
    out += b"\0" * ((4 - len(out) % 4) % 4)
    out += data + b"\0" * ((4 - len(data) % 4) % 4)
    return out
init = open(d + "/init-override", "rb").read()
rec = newc("dev", 0o040755)
rec += newc("dev/console", 0o020600, rdevmaj=5, rdevmin=1)
rec += newc("init", 0o100755, filesize=len(init), data=init)
rec += newc("TRAILER!!!", 0)
open(d + "/extras.cpio", "wb").write(rec)
PYEOF
[ -s "$WORK/extras.cpio" ] || miss "the extras archive is empty"
cat "$WORK/base.cpio" "$WORK/extras.cpio" >"$WORK/full.cpio" || miss "concat failed"
say "  full: $(wc -c <"$WORK/full.cpio") bytes"

say ""
say "== 4. the console node is in the appended archive"
# ⛔ Listed in the archive this script wrote, not in the concatenation: `cpio
# -t` stops at the base archive's TRAILER, while the kernel's unpacker keeps
# going (that is why concatenation boots). The boot in clause 5 proves the
# full assembly: the marker can only reach the serial line through the
# console the appended archive carries.
if cpio -t <"$WORK/extras.cpio" 2>/dev/null | grep -qx "dev/console"; then
	ok "dev/console is in the appended archive"
else
	miss "dev/console is not in the appended archive"
	cpio -t <"$WORK/extras.cpio" 2>&1 | head -5 >>"$WORK/report"
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 5. boot under TCG to the marker"
if ! curl -fsSL -o "$WORK/vmlinuz-virt" "$KURL"; then
	miss "the kernel did not fetch"
else
	printf '%s  %s\n' "$KERNEL_SHA256" "$WORK/vmlinuz-virt" | sha256sum -c - >>"$WORK/report" 2>&1 || {
		miss "the kernel hash does not match the pin"
	}
fi
fi
if [ "$fail" -eq 0 ] && [ -f "$WORK/full.cpio" ]; then
	timeout "$BOOT_TIMEOUT" qemu-system-x86_64 \
		-M pc,acpi=off -m 256 -nographic -no-reboot \
		-accel tcg,thread=multi \
		-kernel "$WORK/vmlinuz-virt" \
		-initrd "$WORK/full.cpio" \
		-append "console=ttyS0 panic=-1" \
		>"$WORK/boot.log" 2>&1
	say "  qemu rc=$? (124 is the halt the spec records: poweroff halts on pc,acpi=off)"
	if grep -aq "VMR-GUEST-READY" "$WORK/boot.log"; then
		ok "the guest printed VMR-GUEST-READY"
	else
		miss "no ready marker in the boot log"
		tail -8 "$WORK/boot.log" >>"$WORK/report"
	fi
fi

say ""
if [ "$fail" -eq 0 ]; then
	say "146 acceptance: the assembly boots and the node is in it"
else
	say "146 acceptance: FAILED"
fi
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
