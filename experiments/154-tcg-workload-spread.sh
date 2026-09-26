#!/usr/bin/env bash
# Question: what does the machine tier cost per workload class, with the
# same checksum on every platform?
#
# TODO/podvm.md T-1308 (one TCG number is a claim about one benchmark).
# Every source that states one multiplier states a different one (3x to
# 21x on the same host class), so this prints a row per workload class
# with its checksum and its ratio, and refuses the run where two platforms
# disagree on a checksum.
#
# Shape, following 146: the pinned image through podbox, four static
# payloads staged into the rootfs beside it, the lib's base-plus-extras
# assembly unchanged, and a batch /init that runs every payload three
# times, prints, and powers off. No serial driver: the console log carries
# everything. The lib is untouched, so 146/147's evidence stands.
#
# Controls, same-day and same-run: native on the host, the chroot tier
# through podbox (scratch store, payloads staged into its rootfs), and the
# TCG guest. One payload binary per class runs on all three, so equal
# checksums are the control that every platform computed the same thing.
# The I/O backing differs per platform and is named in the conditions.
#
# Clauses:
#   0. conditions and the four static payloads with pinned checksums;
#   1. host runs, three per class;
#   2. chroot runs through podbox, three per class;
#   3. guest assembly, batch boot, console parsed;
#   4. one row per class: medians, ratios, checksum agreement. Exit 1
#      where two platforms disagree on a checksum.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and the registry fetch is podbox's own client. On a Windows
# host run it inside the base with PODBOX_BIN set to a guest build
# artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash. No `read -t`, no coproc, no
# arrays: bounded waits are `timeout`.
#
# Exit: 0 every row printed with agreeing checksums, 1 a checksum
# disagreed or a run failed, 2 a tool, the binary or an input could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/tcg-workload-spread.txt"
WORK="$REPO/experiments/.sweep154-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

# ⭐ Every input pinned, as in 146.
ALPINE_REF='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
CURL_TIMEOUT="${PODBOX_154_CURL_TIMEOUT:-60}"
BOOT_TIMEOUT="${PODBOX_154_TIMEOUT:-600}"
RUN_TIMEOUT=300
# ⛔ Pinned beside the invocations, not inside them: the conditions block
# prints these, so a re-drive that changes a flag cannot silently compare
# against the old report (TODO/podvm.md T-1308).
QEMU_FLAGS="-M pc,acpi=off -m 256 -nographic -no-reboot -accel tcg,thread=multi"
BENCH_CFLAGS="-O2 -static"

fail=0
fails=0
driven=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { driven=$((driven + 1)); say "  ok: $1"; }
miss() { fail=1; fails=$((fails + 1)); say "  FAIL: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
for t in qemu-system-x86_64 cpio python3 curl sha256sum timeout gcc file awk sort; do
	command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

# shellcheck source=lib/podvm-guest.sh
. "$HERE/lib/podvm-guest.sh"

PAYLOADDIR="$WORK/payloads"
mkdir -p "$PAYLOADDIR" || exit 2
for w in int sys mem io; do
	# shellcheck disable=SC2086 # the flags are a pinned constant, split on purpose.
	gcc $BENCH_CFLAGS -o "$PAYLOADDIR/bench-$w" "$HERE/154-bench-$w.c" || {
		echo "SKIP: bench-$w did not build static" >&2; exit 2;
	}
	file "$PAYLOADDIR/bench-$w" | grep -q "statically linked" || {
		echo "SKIP: bench-$w is not static" >&2; exit 2;
	}
done

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'qemu              %s\n' "$(qemu-system-x86_64 --version | head -1)"
	printf 'gcc               %s\n' "$(gcc --version | head -1)"
	printf 'image             %s\n' "$ALPINE_REF"
	printf 'kernel            %s (sha256 %s)\n' "$KURL" "$KERNEL_SHA256"
	printf 'qemu flags        %s\n' "$QEMU_FLAGS"
	printf 'payload cflags    gcc %s\n' "$BENCH_CFLAGS"
	printf 'host io backing   %s\n' "$(df -T "$WORK" | tail -1)"
	for w in int sys mem io; do
		printf 'payload bench-%s  %s\n' "$w" "$(sha256sum <"$PAYLOADDIR/bench-$w" | cut -d' ' -f1)"
	done
	echo
} >"$WORK/report"

# run PAYLOAD CWD OUTFILE: three bounded runs, raw lines appended.
bench3() {
	b="$1"; dir="$2"; out="$3"
	(cd "$dir" && for i in 1 2 3; do
		timeout "$RUN_TIMEOUT" "$b" >>"$out" 2>&1 || return 1
	done) || return 1
}

say "== 1. host runs"
mkdir -p "$WORK/host" || exit 2
for w in int sys mem io; do
	# ⛔ T-1308: every platform counts `^workload=` lines, never exits.
	# An `io error=` line beside a zero exit would pass an
	# exit-counted section and fail the line-counted row section, so
	# all three count lines and the tallies agree by construction.
	if bench3 "$PAYLOADDIR/bench-$w" "$WORK/host" "$WORK/host-$w.log"; then
		n=$(grep -c "^workload=$w " "$WORK/host-$w.log" 2>/dev/null); n=${n:-0}
		if [ "$n" -eq 3 ]; then
			pass "host bench-$w ran three times"
		else
			miss "host bench-$w ran $n of 3 times"
		fi
	else
		miss "host bench-$w failed"
	fi
done

say ""
say "== 2. chroot runs through podbox"
"$BIN" pull "$ALPINE_REF" >"$WORK/pull.log" 2>&1 || {
	miss "the pull failed"; tail -3 "$WORK/pull.log" >>"$WORK/report"
}
ROOTFS=""
if [ "$fail" -eq 0 ]; then
	ROOTFS="$("$BIN" extract "$ALPINE_REF" 2>"$WORK/extract.log")"
	[ -n "$ROOTFS" ] && [ -d "$ROOTFS" ] || {
		miss "no rootfs extracted"; ROOTFS=""
	}
fi
if [ -n "$ROOTFS" ]; then
	mkdir -p "$ROOTFS/bench" || exit 2
	cp "$PAYLOADDIR"/bench-int "$PAYLOADDIR"/bench-sys "$PAYLOADDIR"/bench-mem "$PAYLOADDIR"/bench-io "$ROOTFS/bench/" || exit 2
	printf 'chroot io backing %s\n' "$(df -T "$ROOTFS" | tail -1)" >>"$WORK/report"
	mkdir -p "$WORK/chroot" || exit 2
	for w in int sys mem io; do
		(cd "$WORK/chroot" && for i in 1 2 3; do
			timeout "$RUN_TIMEOUT" "$BIN" run --pull=never "$ALPINE_REF" "/bench/bench-$w" >>"$WORK/chroot-$w.log" 2>"$WORK/chroot-$w.err" || {
				echo "run $i rc=$?" >>"$WORK/chroot-$w.err"; break
			}
		done)
		n=$(grep -c "^workload=$w " "$WORK/chroot-$w.log" 2>/dev/null); n=${n:-0}
		if [ "$n" -eq 3 ]; then
			pass "chroot bench-$w ran through podbox three times"
		else
			miss "chroot bench-$w ran $n of 3 times"
			tail -3 "$WORK/chroot-$w.err" >>"$WORK/report" 2>/dev/null || true
		fi
	done
fi

say ""
say "== 3. guest assembly and batch boot"
if [ "$fail" -eq 0 ] && [ -n "$ROOTFS" ]; then
	podvm_base "$ROOTFS" "$WORK/base.cpio" || miss "the base cpio did not build"
fi
if [ "$fail" -eq 0 ]; then
	cat >"$WORK/init-batch" <<'INITEOF'
#!/bin/sh
echo VMR-BENCH-READY
cd /
for b in bench-int bench-sys bench-mem bench-io; do
	for i in 1 2 3; do
		/bench/$b
	done
done
echo VMR-BENCH-DONE
poweroff -f
INITEOF
	podvm_extras "$WORK/init-batch" "$WORK/extras.cpio" || miss "the extras archive did not build"
	podvm_concat "$WORK/base.cpio" "$WORK/extras.cpio" "$WORK/full.cpio" || miss "concat failed"
	pass "assembly $(wc -c <"$WORK/full.cpio") bytes"
fi
if [ "$fail" -eq 0 ]; then
	if ! curl -fsSL --max-time "$CURL_TIMEOUT" -o "$WORK/vmlinuz-virt" "$KURL"; then
		miss "the kernel did not fetch"
	else
		printf '%s  %s\n' "$KERNEL_SHA256" "$WORK/vmlinuz-virt" | sha256sum -c - >>"$WORK/report" 2>&1 || {
			miss "the kernel hash does not match the pin"
		}
	fi
fi
if [ "$fail" -eq 0 ] && [ -f "$WORK/full.cpio" ]; then
	# shellcheck disable=SC2086 # the flags are a pinned constant, split on purpose.
	timeout "$BOOT_TIMEOUT" qemu-system-x86_64 $QEMU_FLAGS \
		-kernel "$WORK/vmlinuz-virt" \
		-initrd "$WORK/full.cpio" \
		-append "console=ttyS0 panic=-1" \
		>"$WORK/boot.log" 2>&1
	say "  qemu rc=$? (124 is the halt the spec records: poweroff halts on pc,acpi=off)"
	if grep -aq "VMR-BENCH-DONE" "$WORK/boot.log"; then
		pass "the guest ran the batch to VMR-BENCH-DONE"
	else
		miss "no done marker in the boot log"
		tail -8 "$WORK/boot.log" >>"$WORK/report"
	fi
fi

# elap WORKLOAD LOGFILE: the median of the three elapsed values, or empty
# where the log holds anything but three runs: a median of fewer runs is
# a wrong number, not an approximation.
elap() {
	# ⚠ `grep -c` prints the count AND exits 1 on zero matches, so an
	# `|| echo 0` guard prints a second zero and the test below reads
	# "0\n0" as an illegal number. Read the printed count, defaulting an
	# unreadable log to zero.
	n=$(grep -c "^workload=$1 " "$2" 2>/dev/null); n=${n:-0}
	[ "$n" -eq 3 ] || return 0
	grep -h "^workload=$1 " "$2" 2>/dev/null | sed -E 's/.* elapsed=([0-9.]+).*/\1/' | sort -n | sed -n '2p'
}
# sums WORKLOAD LOGFILE: every checksum printed for the class, one per line.
sums() {
	grep -h "^workload=$1 " "$2" 2>/dev/null | sed -E 's/.* checksum=([0-9a-f]+).*/\1/' | sort -u
}
# ratio A B: A/B to one decimal, or - where the numbers cannot divide.
ratio() {
	awk -v a="$1" -v b="$2" 'BEGIN { if (b > 0 && a > 0) printf "%.1f", a/b; else printf "-" }'
}

say ""
say "== 4. one row per class"
for w in int sys mem io; do
	he="$(elap "$w" "$WORK/host-$w.log")"
	ce="$(elap "$w" "$WORK/chroot-$w.log")"
	ge="$(elap "$w" "$WORK/boot.log")"
	hc="$(sums "$w" "$WORK/host-$w.log" | tr '\n' ' ')"
	cc="$(sums "$w" "$WORK/chroot-$w.log" | tr '\n' ' ')"
	gc="$(sums "$w" "$WORK/boot.log" | tr '\n' ' ')"
	all="$(printf '%s\n%s\n%s\n' "$hc" "$cc" "$gc" | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ')"
	n=$(printf '%s' "$all" | wc -w)
	if [ "$n" -eq 1 ] && [ -n "$he" ] && [ -n "$ce" ] && [ -n "$ge" ]; then
		pass "$w: host ${he}s chroot ${ce}s ($(ratio "$ce" "$he")x) guest ${ge}s ($(ratio "$ge" "$he")x) checksum $all"
	else
		miss "$w: host=[$he/$hc] chroot=[$ce/$cc] guest=[$ge/$gc]"
	fi
done

say ""
say "== counts: $driven driven, $fails mismatches"
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
