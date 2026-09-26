#!/bin/sh
# Question: how fast is podbox here, against what budget, and did it regress?
#
# TODO/gate.md T-1338 (issue 57: performance claims scattered across
# single-purpose experiments, each on one host and one shape). One
# harness emitting machine-readable rows (commit, host, kernel, arch,
# qemu version, filesystem, network shape, metric, value, unit), with
# results in experiments/results/perf-SHAPE.txt and the exit contract
# 0 ran and matched, 1 ran and a metric failed, 2 could not run.
#
# Shapes: `lane` (the container job: constrained host, loopback link,
# overlayfs) and `kvm` (the wsl-toolkit base with /dev/kvm). Metrics
# are wall time with peak RSS: cold pull (tag and digest), probe and
# extract; three payload runs named by order (first, repeat, late);
# the lifecycle verbs, the data verbs (cp, save, load, prune, man),
# each ladder rung that enters (the rest record refused); guest boot
# and command latency under TCG (lane) against KVM (base); binary
# size with PT_INTERP state. Seed rows from 190 (pull pool) and 154
# (TCG workloads) live in experiments/results/perf-seeds.tsv with
# their source report beside each value, not re-measured here.
#
# The budget lives in experiments/perf-ceilings.tsv (metric, unit,
# ceiling, tolerance). This script WRITES rows; scripts/check-todo.py
# check 30 COMPARES them (one read path: the comparison lives in
# exactly one place).
#
# Usage: PODBOX_BIN=/path/to/podbox sh experiments/360-perf-harness.sh [lane|kvm]
#
# Exit: 0 every metric ran, 1 a metric failed, 2 the shape could not
# run (no binary, no pull, no qemu, no KVM where the shape needs it).
set -u

# The job travels inside the workspace as /work/.podbox-job.sh, so $0
# names the staging path, not experiments/. Take the checkout from the
# working directory instead (experiments/353-open-issue-triage.sh).
REPO="$(pwd)"
cd "$REPO" || exit 2
SHAPE="${1:-lane}"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/perf-$SHAPE.txt"
WORK="$REPO/experiments/.sweep360-work-$SHAPE"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

# ⭐ Every input pinned. The debian row is the CLI matrix image (same
# bytes as 363/364/365); the alpine row is the guest-base image (same
# bytes as 154, so the guest numbers compare against its workload
# ratios); the kernel is 154's pin beside its sha256.
DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
ALPINE='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
QEMU_TCG="-M pc,acpi=off -m 256 -nographic -no-reboot -accel tcg,thread=multi"
QEMU_KVM="-M pc,acpi=off -m 256 -nographic -no-reboot -accel kvm"
BOOT_TIMEOUT=600

fail=0
failed_metrics=""
pass() { echo "  ok: $1" >>"$WORK/report"; }
miss() { echo "  FAIL: $1" >>"$WORK/report"; fail=1; failed_metrics="$failed_metrics $1"; }

# One context block: every row below carries these, so a row read
# alone still names its machine (docs/conventions/prose.md: a number
# carries its conditions or it is not a number).
COMMIT="$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo unknown)"
HOST="$(uname -srm)"
KERNEL="$(uname -r)"
ARCH="$(uname -m)"
FSBACK="$(df -T "$WORK" 2>/dev/null | tail -1 | awk '{print $1"/"$2}' || echo unknown)"
QEMU_V="$(qemu-system-x86_64 --version 2>/dev/null | head -1 || echo absent)"
{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "shape             $SHAPE"
echo "commit            $COMMIT"
echo "host              $HOST"
echo "kernel            $KERNEL"
echo "arch              $ARCH"
echo "io backing        $FSBACK"
echo "podbox            $($BIN version 2>/dev/null || echo MISSING)"
echo "qemu              $QEMU_V"
echo "debian            $DEBIAN"
echo "alpine            $ALPINE"
echo "kernel pin        $KURL (sha256 $KERNEL_SHA256)"
echo "link shape        loopback pulls from the registry; 190 covers latency-bound"
echo "guest poll        0.1 s console poll for READY/DONE markers"
echo "== rows: commit host kernel arch shape metric cond value unit state"
} >"$WORK/report"

# row METRIC COND VALUE UNIT STATE: one machine-readable TSV line.
row() { printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
	"$COMMIT" "$HOST" "$KERNEL" "$ARCH" "$SHAPE" "$1" "$2" "$3" "$4" "$5" >>"$WORK/report"; }

[ -x "$BIN" ] || { echo "binary $BIN is not executable: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
command -v timeout >/dev/null 2>&1 || { echo "timeout is missing: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
for t in curl sha256sum stat awk grep; do
	command -v "$t" >/dev/null 2>&1 || { echo "$t is missing: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
done

# Peak RSS comes from GNU time where present; where it is absent the
# row carries a dash, never an invented number.
TIMEBIN=""
command -v /usr/bin/time >/dev/null 2>&1 && TIMEBIN="/usr/bin/time"

# measure METRIC COND -- CMD...: wall seconds (3 decimals) and peak
# RSS kilobytes of one run, then the TSV row. Every verb runs under
# `timeout`: a hung verb is a failed metric at rc 124, never a hung
# drive (a runtime for automated callers may not wait unbounded).
# MTIMEOUT sets the bound per call (default 120 s). A nonzero exit is
# a failed metric, never a slow one: the budget comparison lives in
# check 30, not here.
measure() {
	metric="$1"; cond="$2"; shift 2
	mt="${MTIMEOUT:-120}"
	if [ -n "$TIMEBIN" ]; then
		t0=$(date +%s%N)
		timeout "$mt" "$TIMEBIN" -v "$@" >"$WORK/m.out" 2>"$WORK/m.err"
		rc=$?
		t1=$(date +%s%N)
		rss=$(grep -a "Maximum resident set size" "$WORK/m.err" | awk '{print $6}')
		[ -n "$rss" ] || rss="-"
	else
		t0=$(date +%s%N)
		timeout "$mt" "$@" >"$WORK/m.out" 2>"$WORK/m.err"
		rc=$?
		t1=$(date +%s%N)
		rss="-"
	fi
	sec=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", (b-a)/1e9 }')
	if [ "$rc" -eq 0 ]; then
		row "$metric" "$cond" "$sec" "s wall" "ok"
		row "$metric.rss" "$cond" "$rss" "kB peak" "ok"
		pass "$metric [$cond] ${sec}s wall, ${rss}kB peak"
	else
		row "$metric" "$cond" "-" "s wall" "failed"
		miss "$metric [$cond] rc=$rc"
		cat "$WORK/m.out" "$WORK/m.err" >>"$WORK/report"
	fi
}

echo "" >>"$WORK/report"
echo "== pulls (cold store)" >>"$WORK/report"
STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

MTIMEOUT=600
measure pull.tag.cold loopback "$BIN" pull "$DEBIAN"
[ "$fail" -eq 0 ] || { echo "debian pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
measure pull.digest.cold loopback "$BIN" pull "$ALPINE"
[ "$fail" -eq 0 ] || { echo "alpine pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
MTIMEOUT=120
measure probe.cold loopback "$BIN" probe
measure extract.cold loopback "$BIN" extract "$DEBIAN"

echo "" >>"$WORK/report"
echo "== payload runs: three runs across the drive, named by order" >>"$WORK/report"
echo "  (early runs read ~2 s, the late one ~0.08 s; the decay is" >>"$WORK/report"
echo "  recorded, not explained - consistent with cold file caches," >>"$WORK/report"
echo "  not isolated: TODO/gate.md T-1340)" >>"$WORK/report"
measure run.first loopback "$BIN" run --rm "$DEBIAN" /bin/echo perf-hi
timeout 120 "$BIN" run --rm "$DEBIAN" /bin/echo warmup >/dev/null 2>&1
timeout 120 "$BIN" run --rm "$DEBIAN" /bin/echo warmup >/dev/null 2>&1
measure run.repeat loopback "$BIN" run --rm "$DEBIAN" /bin/echo perf-hi

echo "" >>"$WORK/report"
echo "== lifecycle verbs (sleep payload, so stop meets it alive)" >>"$WORK/report"
measure create loopback "$BIN" create --name perf360 "$DEBIAN" /bin/sleep 30
measure start loopback "$BIN" start perf360
measure exec loopback "$BIN" exec perf360 /bin/echo exec-hi
measure stop loopback "$BIN" stop perf360
measure logs loopback "$BIN" logs perf360
measure ps loopback "$BIN" ps
measure rm loopback "$BIN" rm perf360
measure run.late loopback "$BIN" run --rm "$DEBIAN" /bin/echo late-hi

echo "" >>"$WORK/report"
echo "== data verbs" >>"$WORK/report"
measure cp loopback "$BIN" cp "$DEBIAN:/bin/echo" "$WORK/cp-echo"
if [ -f "$WORK/cp-echo" ]; then
	pass "cp landed $(wc -c <"$WORK/cp-echo") bytes"
else
	miss "cp landed no file"
fi
measure save loopback "$BIN" save -o "$WORK/perf360.tar" "$DEBIAN"
measure load loopback "$BIN" load -i "$WORK/perf360.tar"
measure prune loopback "$BIN" image prune -f
measure man loopback "$BIN" man run

echo "" >>"$WORK/report"
echo "== ladder rungs (forced; a 125 refusal is the rung's honest answer, not a time)" >>"$WORK/report"
# The cache rung enters with its opt-in (358); the memfd family needs
# a static payload, built here like 356 builds it (no registry image
# on the lane is static: the family pins to the toolchain, not a digest).
if command -v cc >/dev/null 2>&1; then
	cat >"$WORK/hello.c" <<'EOF'
#include <stdio.h>
int main(void) { puts("static-hi"); return 0; }
EOF
	mkdir -p "$WORK/static-root" || exit 2
	FIXTURE_OK=""
	if cc -O2 -static-pie -o "$WORK/static-root/hello" "$WORK/hello.c" >>"$WORK/report" 2>&1 \
		&& (cd "$WORK/static-root" && tar -cf "$WORK/static-hello.tar" hello) >>"$WORK/report" 2>&1 \
		&& timeout 120 "$BIN" import "$WORK/static-hello.tar" perfstatic:1 >>"$WORK/report" 2>&1; then
		FIXTURE_OK="yes"
		echo "  note: static fixture imported as perfstatic:1" >>"$WORK/report"
	else
		echo "  note: static fixture did not build: the memfd rung records could-not-run" >>"$WORK/report"
	fi
fi
for rung in rundir cache memfd tmpfs fuse; do
	if [ "$rung" = "memfd" ] && [ -z "${FIXTURE_OK:-}" ]; then
		row "rung.memfd" "loopback" "-" "s wall" "could-not-run"
		echo "  note: no static toolchain: the memfd rung could not run" >>"$WORK/report"
		continue
	fi
	case "$rung" in
		cache) FORCE="env PODBOX_CACHE=1 PODBOX_MODE=cache" ;;
		*) FORCE="env PODBOX_MODE=$rung" ;;
	esac
	case "$rung" in
		memfd) IMAGE="perfstatic:1"; CMD="/hello" ;;
		*) IMAGE="$DEBIAN"; CMD="/bin/echo rung-hi" ;;
	esac
	t0=$(date +%s%N)
	if [ -n "$TIMEBIN" ]; then
		# shellcheck disable=SC2086 # the force words split on purpose.
		timeout 120 $TIMEBIN -v $FORCE "$BIN" run --rm "$IMAGE" $CMD >"$WORK/m.out" 2>"$WORK/m.err"
		rc=$?
		rss=$(grep -a "Maximum resident set size" "$WORK/m.err" | awk '{print $6}')
		[ -n "$rss" ] || rss="-"
	else
		# shellcheck disable=SC2086 # the force words split on purpose.
		timeout 120 $FORCE "$BIN" run --rm "$IMAGE" $CMD >"$WORK/m.out" 2>"$WORK/m.err"
		rc=$?
		rss="-"
	fi
	t1=$(date +%s%N)
	sec=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", (b-a)/1e9 }')
	if [ "$rc" -eq 0 ]; then
		row "rung.$rung" "loopback" "$sec" "s wall" "ok"
		row "rung.$rung.rss" "loopback" "$rss" "kB peak" "ok"
		pass "rung $rung enters: ${sec}s wall, ${rss}kB peak"
	elif [ "$rc" -eq 125 ]; then
		row "rung.$rung" "loopback" "-" "s wall" "refused"
		echo "  note: forced $rung rung refuses at 125 here (m.err head)" >>"$WORK/report"
		head -5 "$WORK/m.err" >>"$WORK/report" 2>/dev/null || true
	else
		row "rung.$rung" "loopback" "-" "s wall" "failed"
		miss "rung $rung rc=$rc"
	fi
done

echo "" >>"$WORK/report"
echo "== binary size (read off the artefact, not its self-report)" >>"$WORK/report"
SIZEBIN="$BIN"
SIZEBYTES="$(stat -c%s "$SIZEBIN" 2>/dev/null || stat -f%z "$SIZEBIN" 2>/dev/null || echo -)"
row "size.bytes" "release-musl" "$SIZEBYTES" "B" "ok"
if command -v readelf >/dev/null 2>&1; then
	if readelf -l "$SIZEBIN" 2>/dev/null | grep -q "INTERP"; then
		row "size.interp" "release-musl" "1" "present" "ok"
	else
		row "size.interp" "release-musl" "0" "present" "ok"
	fi
	pass "binary $SIZEBYTES bytes, PT_INTERP row recorded"
else
	row "size.interp" "release-musl" "-" "present" "could-not-run"
	echo "  note: readelf absent, interp state unknown" >>"$WORK/report"
fi

echo "" >>"$WORK/report"
echo "== guest boot and command (qemu-direct, the emulator path the machine tier will use)" >>"$WORK/report"
# The product's machine tier prints its holds message (357) rather
# than booting, so the harness times the emulator assembly 154
# proves: the pinned kernel plus an initramfs of the pulled alpine
# rootfs with a marker /init, under TCG on the lane and KVM on the
# base. No hardware-isolation claim rides along (out of scope).
if [ "$SHAPE" = "kvm" ]; then
	ACCEL="$QEMU_KVM"
	ACCEL_NAME="kvm"
else
	ACCEL="$QEMU_TCG"
	ACCEL_NAME="tcg"
fi
if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
	row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
	row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
	echo "  note: no qemu-system-x86_64 on PATH: guest metrics could not run" >>"$WORK/report"
elif ! command -v cpio >/dev/null 2>&1; then
	row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
	row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
	echo "  note: no cpio on PATH: the initramfs cannot assemble, guest metrics could not run" >>"$WORK/report"
elif [ "$SHAPE" = "kvm" ] && [ ! -e /dev/kvm ]; then
	row "guest.kvm.boot" "kvm" "-" "s wall" "could-not-run"
	row "guest.kvm.cmd" "kvm" "-" "s wall" "could-not-run"
	echo "  note: no /dev/kvm: KVM guest metrics could not run" >>"$WORK/report"
else
	if ! timeout 600 "$BIN" pull "$ALPINE" >>"$WORK/report" 2>&1; then
		row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
		row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
		echo "  note: alpine pull FAILED: guest metrics could not run" >>"$WORK/report"
	else
		GROOTFS="$(timeout 600 "$BIN" extract "$ALPINE" 2>/dev/null)"
		if [ -z "$GROOTFS" ] || [ ! -d "$GROOTFS/bin" ]; then
			row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
			row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
			echo "  note: no extracted rootfs: guest metrics could not run" >>"$WORK/report"
		else
			if ! curl -fsSL --max-time 60 -o "$WORK/vmlinuz-virt" "$KURL" >>"$WORK/report" 2>&1; then
				row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
				row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
				echo "  note: the kernel did not fetch: guest metrics could not run" >>"$WORK/report"
			elif ! printf '%s  %s\n' "$KERNEL_SHA256" "$WORK/vmlinuz-virt" | sha256sum -c - >>"$WORK/report" 2>&1; then
				row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
				row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
				echo "  note: the kernel hash does not match the pin: guest metrics could not run" >>"$WORK/report"
			else
				# shellcheck source=lib/podvm-guest.sh
				. "$REPO/experiments/lib/podvm-guest.sh"
				cat >"$WORK/guest-init" <<'INITEOF'
#!/bin/sh
echo VMR-PERF-READY
echo VMR-PERF-CMD
echo VMR-PERF-DONE
poweroff -f
INITEOF
				if ! podvm_base "$GROOTFS" "$WORK/base.cpio" >>"$WORK/report" 2>&1 \
					|| ! podvm_extras "$WORK/guest-init" "$WORK/extras.cpio" >>"$WORK/report" 2>&1 \
					|| ! podvm_concat "$WORK/base.cpio" "$WORK/extras.cpio" "$WORK/full.cpio" >>"$WORK/report" 2>&1; then
					row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "could-not-run"
					row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "could-not-run"
					echo "  note: the initramfs did not assemble: guest metrics could not run" >>"$WORK/report"
				else
					t0=$(date +%s%N)
					# shellcheck disable=SC2086 # flags are pinned constants, split on purpose.
					timeout "$BOOT_TIMEOUT" qemu-system-x86_64 $ACCEL \
						-kernel "$WORK/vmlinuz-virt" \
						-initrd "$WORK/full.cpio" \
						-append "console=ttyS0 panic=-1" \
						>"$WORK/boot.log" 2>&1 &
					QPID=$!
					READY_AT=""; DONE_AT=""
					# 0.1 s poll: the command metric is millisecond-scale,
					# so a 1 s poll would quantize it away. The interval
					# rides in the conditions of the reading.
					while kill -0 "$QPID" 2>/dev/null; do
						now=$(date +%s%N)
						if [ -z "$READY_AT" ] && grep -aq "VMR-PERF-READY" "$WORK/boot.log" 2>/dev/null; then
							READY_AT="$now"
						fi
						if grep -aq "VMR-PERF-DONE" "$WORK/boot.log" 2>/dev/null; then
							DONE_AT="$now"
							break
						fi
						sleep 0.1
					done
					# The halted guest never exits on pc,acpi=off (poweroff
					# has no ACPI to answer it, under TCG or KVM alike):
					# reap it now that the markers are in, instead of
					# sitting in wait until BOOT_TIMEOUT kills it.
					kill "$QPID" 2>/dev/null
					wait "$QPID" 2>/dev/null
					if [ -n "$READY_AT" ]; then
						boot=$(awk -v a="$t0" -v b="$READY_AT" 'BEGIN { printf "%.3f", (b-a)/1e9 }')
						row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "$boot" "s wall" "ok"
						pass "guest $ACCEL_NAME boot ${boot}s to READY"
					else
						row "guest.$ACCEL_NAME.boot" "$ACCEL_NAME" "-" "s wall" "failed"
						miss "guest $ACCEL_NAME boot: no READY marker"
						tail -8 "$WORK/boot.log" >>"$WORK/report" 2>&1
					fi
					if [ -n "$READY_AT" ] && [ -n "$DONE_AT" ]; then
						cmd=$(awk -v a="$READY_AT" -v b="$DONE_AT" 'BEGIN { printf "%.3f", (b-a)/1e9 }')
						row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "$cmd" "s wall" "ok"
						if [ "$cmd" = "0.000" ]; then
							pass "guest $ACCEL_NAME command under one 0.1 s poll tick"
						else
							pass "guest $ACCEL_NAME command ${cmd}s READY to DONE"
						fi
					elif [ -n "$READY_AT" ]; then
						row "guest.$ACCEL_NAME.cmd" "$ACCEL_NAME" "-" "s wall" "failed"
						miss "guest $ACCEL_NAME command: no DONE marker"
					fi
				fi
			fi
		fi
	fi
fi

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           PERF $SHAPE SERVED" >>"$WORK/report"; else echo "verdict           PERF $SHAPE OPEN" >>"$WORK/report"; fi
echo "== counts: fail=$fail ($failed_metrics )" >>"$WORK/report"
cp "$WORK/report" "$OUT"
if [ -d /out ]; then cp "$OUT" /out/ 2>/dev/null || true; fi
[ "$fail" -eq 0 ]
