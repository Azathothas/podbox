#!/bin/sh
# Question: does the pinned guest kernel boot `-M microvm` every time, with
# a virtio-mmio NIC and a 9p device, and how long does a boot take against
# `-M pc,acpi=off`?
#
# TODO/machine.md T-1723 (issue 168 counts twenty boots for each flag
# set). The kernel, its 9p modules and the base archive of alpine come from
# a podvm store: one `run --podvm-tier=machine` fills it. The init of each
# boot loads the 9p modules with busybox insmod, mounts the tag, writes a
# file with the number of the boot, and resets the machine.
#
# Clauses:
#   1. PODVM_440_BOOTS boots (20 by default) of `-M microvm` each print the
#      marker, see the two virtio devices (net and 9p), and write their
#      file through 9p onto the host.
#   2. the same count of `-M pc,acpi=off` boots each print the marker.
#   3. the wall time of each set is printed (not a clause: a measurement).
#   4. no emulator stays after the boots.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep440-work"
rm -rf "$WORK"; mkdir -p "$WORK/share" || exit 2
OUT="$REPO/experiments/results/microvm-machine.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
BOOTS=${PODVM_440_BOOTS:-20}
IMG="${PODVM_440_IMAGE:-mirror.gcr.io/library/alpine:3.20}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v qemu-system-x86_64 >/dev/null 2>&1 || { echo "qemu-system-x86_64 missing: COULD NOT RUN" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then ACCEL=kvm; CPU=host; else ACCEL=tcg; CPU=max; fi
timeout 900 "$PB" run --rm --podvm-tier=machine "$IMG" true >"$WORK/fill.log" 2>&1 || { echo "the store did not fill: COULD NOT RUN" >&2; tail -3 "$WORK/fill.log" >&2; exit 2; }
M="$PODVM_STORE/machine"
KERNEL=$(ls "$M"/vmlinuz-* 2>/dev/null | head -1)
BASE=$(ls "$M"/base-*.cpio 2>/dev/null | head -1)
[ -f "$KERNEL" ] && [ -f "$BASE" ] || { echo "no kernel or base archive in the store: COULD NOT RUN" >&2; exit 2; }

# The init of a boot: the marker, the virtio devices, the 9p write.
cat >"$WORK/init" <<'EOF'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
n=$(sed 's/.*podvm440=\([0-9]*\).*/\1/' /proc/cmdline)
echo "PODVM440-UP $n devices=$(ls /sys/bus/virtio/devices 2>/dev/null | wc -l)"
if [ -d /m ]; then
	for k in netfs 9pnet 9pnet_virtio 9p; do insmod /m/$k.ko 2>/dev/null; done
	mkdir -p /share && mount -t 9p -o trans=virtio,version=9p2000.L podvm0 /share && echo "boot $n" > /share/boot-$n && echo "PODVM440-9P $n"
fi
reboot -f
EOF
python3 - "$WORK/init" "$M" "$WORK/init.cpio" <<'EOF'
import sys, glob, os
init, mdir, out = sys.argv[1:]
def ent(name, mode, body, ino):
    n = name.encode() + b'\0'
    h = '070701' + ''.join('%08X' % x for x in [ino, mode, 0, 0, 1, 0, len(body), 0, 0, 0, 0, len(n), 0])
    b = h.encode() + n
    b += b'\0' * ((4 - len(b) % 4) % 4)
    return b + body + b'\0' * ((4 - len(body) % 4) % 4)
parts = [ent('podvm-init', 0o100755, open(init, 'rb').read(), 900001), ent('m', 0o040755, b'', 900002)]
ino = 900003
for k in ['netfs', '9pnet', '9pnet_virtio', '9p']:
    f = glob.glob(os.path.join(mdir, k + '-*.ko'))[0]
    parts.append(ent('m/' + k + '.ko', 0o100644, open(f, 'rb').read(), ino)); ino += 1
parts.append(ent('TRAILER!!!', 0, b'', 0))
open(out, 'wb').write(b''.join(parts))
EOF
cat "$BASE" "$WORK/init.cpio" >"$WORK/rd.cpio" || exit 2

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "emulator          $(qemu-system-x86_64 --version | head -1)"
note "accelerator       $ACCEL"
note "guest kernel      $(basename "$KERNEL") (the pin of alpine-netboot)"
note "boots             $BOOTS of each machine"
note ""

boot() { # machine n -> the console of the boot in $WORK/boot.out
	m=$1; n=$2
	case $m in
	microvm) dev="-netdev user,id=n0 -device virtio-net-device,netdev=n0 -fsdev local,id=f0,path=$WORK/share,security_model=none -device virtio-9p-device,fsdev=f0,mount_tag=podvm0" ;;
	*) dev="-nic user,model=virtio-net-pci -virtfs local,path=$WORK/share,mount_tag=podvm0,security_model=none" ;;
	esac
	# shellcheck disable=SC2086
	timeout 120 qemu-system-x86_64 -M "$m" -nodefaults -no-user-config -display none \
		-serial stdio -no-reboot -accel "$ACCEL" -cpu "$CPU" -m 512M -smp 1 \
		-kernel "$KERNEL" -initrd "$WORK/rd.cpio" \
		-append "console=ttyS0 quiet loglevel=0 panic=-1 rdinit=/podvm-init podvm440=$n" \
		$dev >"$WORK/boot.out" 2>&1
}

for m in microvm "pc,acpi=off"; do
	up=0; nine=0; devs=0; hung=""
	t0=$(date +%s%N)
	i=1
	while [ "$i" -le "$BOOTS" ]; do
		boot "$m" "$i"
		rc=$?
		if [ "$rc" -eq 124 ]; then
			if grep -q "^PODVM440-UP $i " "$WORK/boot.out"; then hung="$hung $i(after-up)"; else hung="$hung $i(before-up)"; fi
			cp "$WORK/boot.out" "$WORK/hung-$(echo "$m" | cut -d, -f1)-$i.out"
		fi
		grep -q "^PODVM440-UP $i devices=2" "$WORK/boot.out" && devs=$((devs + 1))
		grep -q "^PODVM440-UP $i " "$WORK/boot.out" && up=$((up + 1))
		grep -q "^PODVM440-9P $i" "$WORK/boot.out" && [ -f "$WORK/share/boot-$i" ] && nine=$((nine + 1))
		rm -f "$WORK/share/boot-$i"
		i=$((i + 1))
	done
	t1=$(date +%s%N)
	note "== -M $m"
	note "  up $up of $BOOTS, two virtio devices $devs, a 9p write on the host $nine"
	note "  hung at the timeout of 120 s:${hung:- none}"
	note "  wall time ${BOOTS} boots: $(( (t1 - t0) / 1000000 )) ms, $(( (t1 - t0) / 1000000 / BOOTS )) ms each"
	[ "$up" -eq "$BOOTS" ] || bad "-M $m booted $up of $BOOTS times"
	[ -z "$hung" ] || bad "-M $m hung:$hung"
	if [ "$m" = microvm ]; then
		[ "$devs" -eq "$BOOTS" ] || bad "-M microvm saw the two virtio devices $devs of $BOOTS times"
		[ "$nine" -eq "$BOOTS" ] || bad "-M microvm wrote through 9p $nine of $BOOTS times"
	fi
done
note ""
pgrep -f "podvm440=" >/dev/null 2>&1 && bad "an emulator stayed after the boots"

if [ "$fail" -eq 0 ]; then
	note "verdict: THE PINNED KERNEL BOOTS -M microvm EACH TIME"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
