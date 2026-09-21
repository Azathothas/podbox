#!/usr/bin/env bash
# Question: does one command cross the serial line and come back with its
# own status, while a wrong status, a deadline and a forged marker each
# report distinctly?
#
# TODO/podvm.md T-1304 (the exec protocol; the image wrapping is T-1303's
# 146, whose assembly this reuses from experiments/lib/podvm-guest.sh).
#
# The wire, ruled in the entry: every marker carries a per-command 128-bit
# nonce and matches line-anchored and exact. A payload can print anything,
# so a marker without a nonce is a shape collision waiting for content.
#
# Clauses over pinned inputs:
#   0. the conditions: binary, kernel hash, image digest, guest tools,
#      deadlines;
#   1. the assembly: the pinned image through podbox, the lib's base plus
#      an /init that prints VMR-GUEST-READY and execs a shell;
#   2. readiness: the marker, then a nonce round-trip. Feeding the console
#      before the shell listens is lossy, so the driver retries until the
#      guest answers rather than sleeping;
#   3. a zero status comes back zero;
#   4. a non-zero status comes back with its number;
#   5. a marker-shaped line with the wrong nonce does not end the command;
#   6. an overlong command reports a deadline, which is neither a status
#      nor silence.
#
# Runs on Linux, native or in a job container: qemu, cpio and python3 come
# from the guest package manager and are recorded in the conditions. On a
# Windows host run it inside the base with PODBOX_BIN set to a guest build
# artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash. No `read -t`, no coproc, no
# arrays: bounded waits are `timeout` and deadline loops.
#
# Exit: 0 every clause green, 1 a clause failed, 2 a tool, the binary or an
# input could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/podvm-exec.txt"
WORK="$REPO/experiments/.sweep147-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

# ⭐ Every input pinned, as in 146.
ALPINE_REF='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
BOOT_DEADLINE="${PODBOX_147_BOOT:-120}"
CMD_DEADLINE="${PODBOX_147_CMD:-20}"

fail=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
miss() { fail=1; say "  FAIL: $1"; }
ok() { say "  ok: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
for t in qemu-system-x86_64 cpio python3 curl sha256sum timeout mkfifo stdbuf; do
	command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

# shellcheck source=lib/podvm-guest.sh
. "$HERE/lib/podvm-guest.sh"

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'qemu              %s\n' "$(qemu-system-x86_64 --version | head -1)"
	printf 'image             %s\n' "$ALPINE_REF"
	printf 'kernel            %s (sha256 %s)\n' "$KURL" "$KERNEL_SHA256"
	printf 'boot deadline     %s s\n' "$BOOT_DEADLINE"
	printf 'command deadline  %s s\n' "$CMD_DEADLINE"
	echo
} >"$WORK/report"

# ⛔ `-N16`: od reads to EOF and urandom has none, so an unbounded od is
# the unbounded wait RULES.md section 8 forbids. Sixteen bytes is the
# ruled 128-bit nonce.
nonce() { od -An -N16 -tx1 /dev/urandom | tr -d ' \n'; }

say "== 1. the assembly"
"$BIN" pull "$ALPINE_REF" >"$WORK/pull.log" 2>&1 || {
	tail -3 "$WORK/pull.log" >>"$WORK/report"
	miss "the pull failed"
}
ROOTFS=""
if [ "$fail" -eq 0 ]; then
	ROOTFS="$("$BIN" extract "$ALPINE_REF" 2>"$WORK/extract.log")"
	[ -n "$ROOTFS" ] && [ -d "$ROOTFS" ] || {
		tail -3 "$WORK/extract.log" >>"$WORK/report"
		miss "extract printed no rootfs directory"
	}
fi
if [ "$fail" -eq 0 ]; then
	cat >"$WORK/init-shell" <<'INITEOF'
#!/bin/sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
echo VMR-GUEST-READY
exec /bin/sh
INITEOF
	podvm_base "$ROOTFS" "$WORK/base.cpio" || miss "the base cpio did not build"
	podvm_extras "$WORK/init-shell" "$WORK/extras.cpio" || miss "the extras archive did not build"
	podvm_concat "$WORK/base.cpio" "$WORK/extras.cpio" "$WORK/full.cpio" || miss "concat failed"
	ok "assembly: $(wc -c <"$WORK/full.cpio") bytes"
fi
if [ "$fail" -eq 0 ]; then
	if ! curl -fsSL -o "$WORK/vmlinuz-virt" "$KURL"; then
		miss "the kernel did not fetch"
	else
		printf '%s  %s\n' "$KERNEL_SHA256" "$WORK/vmlinuz-virt" | sha256sum -c - >>"$WORK/report" 2>&1 || {
			miss "the kernel hash does not match the pin"
		}
	fi
fi

# The guest and its serial pair. The parent holds both ends open before
# qemu starts: the pipe backend opens existing fifos and blocks otherwise
# (the spec, measured). Writes go to .in, reads come from .out.
SER="$WORK/serial"
QEMU=""
READER=""
cleanup() {
	[ -n "$QEMU" ] && kill "$QEMU" 2>/dev/null
	[ -n "$READER" ] && kill "$READER" 2>/dev/null
	exec 7>&- 2>/dev/null || true
	exec 8>&- 2>/dev/null || true
}

# A failed boot leaves its logs beside the transcript: a red run with no
# guest log is a verdict nobody can debug. A green run removes them, so no
# stale failure ships beside a pass.
keep_logs() {
	cp "$WORK/guest.log" "$(dirname "$OUT")/podvm-exec-guest.log" 2>/dev/null || true
	cp "$WORK/qemu.log" "$(dirname "$OUT")/podvm-exec-qemu.log" 2>/dev/null || true
	say "  qemu's last lines:"
	tail -8 "$WORK/qemu.log" 2>/dev/null >>"$WORK/report"
	say "  guest's last lines:"
	tail -8 "$WORK/guest.log" 2>/dev/null >>"$WORK/report"
}
trap 'cleanup; rm -rf "$WORK"' EXIT INT TERM

send() { printf '%s\n' "$1" >&7; }

# wait_for PATTERN DEADLINE WHAT: poll the guest log until a line matches,
# or report the deadline. The two states never read as each other.
wait_for() {
	_i=0
	while [ "$_i" -lt "$2" ]; do
		if grep -aq "$1" "$WORK/guest.log" 2>/dev/null; then
			return 0
		fi
		sleep 1
		_i=$((_i + 1))
	done
	miss "deadline ($2 s) waiting for $3"
	return 1
}

if [ "$fail" -eq 0 ]; then
say ""
say "== 2. boot and readiness"
mkfifo "$SER.in" "$SER.out" || { miss "no fifos"; }
# ⛔ Both ends held O_RDWR before qemu starts: the opens never block and a
# backend that opens non-blocking never meets ENXIO. A reader that opens
# the fifo only when it gets scheduled is a race with qemu's own open.
exec 7<>"$SER.in" || { miss "the input fifo did not open"; }
exec 8<>"$SER.out" || { miss "the output fifo did not open"; }
# ⛔ A serial console speaks CRLF, and every line-anchored match below would
# miss past a trailing carriage return. The translation happens once, at the
# reader, so no clause has to know about it. `stdbuf -o0`: tr block-buffers
# a file target and would hold the marker past the deadline; unbuffered it
# delivers every byte as it arrives.
cat <&8 2>/dev/null | stdbuf -o0 tr -d '\r' >"$WORK/guest.log" 2>/dev/null & READER=$!
timeout -s KILL 600 qemu-system-x86_64 \
	-M pc,acpi=off -m 256 -display none -monitor none -no-reboot \
	-accel tcg,thread=multi \
	-kernel "$WORK/vmlinuz-virt" \
	-initrd "$WORK/full.cpio" \
	-append "console=ttyS0 panic=-1" \
	-serial "pipe:$SER" \
	>"$WORK/qemu.log" 2>&1 & QEMU=$!
# ⛔ A qemu that dies at startup is a fast failure, not a 120 s boot
# deadline. The wait below cannot tell them apart, so this does.
sleep 3
if ! kill -0 "$QEMU" 2>/dev/null; then
	wait "$QEMU"
	miss "qemu exited at startup with rc=$?"
fi
if wait_for "^VMR-GUEST-READY$" "$BOOT_DEADLINE" "the ready marker"; then
	ok "the guest booted"
	# ⛔ Retried, not slept: feeding the console before the shell listens
	# is lossy, so the driver re-sends until the round-trip answers. A
	# single send here is the fixed sleep RULES.md section 8 forbids.
	HS="$(nonce)"
	_i=0
	while [ "$_i" -lt 60 ]; do
		send "echo HS-$HS"
		if grep -aq "^HS-$HS$" "$WORK/guest.log" 2>/dev/null; then
			break
		fi
		sleep 2
		_i=$((_i + 1))
	done
	if grep -aq "^HS-$HS$" "$WORK/guest.log" 2>/dev/null; then
		ok "the shell answers: readiness is a round-trip, not a sleep"
	else
		miss "deadline (120 s) waiting for the handshake echo"
	fi
fi
[ "$fail" -eq 0 ] || keep_logs
fi

# run_case NAME CMD DEADLINE: one command across the line. The nonce is
# fresh per command; the end marker is matched line-anchored and exact.
# ⛔ The command runs in a subshell: a bare `exit 3` would end the very
# shell that must print the end marker, so without the parens the reporter
# dies with the command and every shell-ending status reads as DEADLINE.
# Prints the reported status, or DEADLINE where the wait ran out.
run_case() {
	_N="$(nonce)"
	send "echo VMR-BEGIN-$_N"
	send "( $2 )"
	send "echo VMR-END-$_N \$?"
	_i=0
	while [ "$_i" -lt "$3" ]; do
		_line="$(grep -a "^VMR-END-$_N [0-9][0-9]*$" "$WORK/guest.log" 2>/dev/null | head -1)"
		if [ -n "$_line" ]; then
			printf '%s' "$_line" | sed "s/^VMR-END-$_N //"
			return 0
		fi
		sleep 1
		_i=$((_i + 1))
	done
	printf 'DEADLINE'
	return 1
}

if [ "$fail" -eq 0 ]; then
say ""
say "== 3. a zero status comes back zero"
_got="$(run_case zero true "$CMD_DEADLINE")"
if [ "$_got" = "0" ]; then
	ok "true reported 0"
else
	miss "true reported $_got"
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 4. a non-zero status comes back with its number"
_got="$(run_case nonzero "exit 3" "$CMD_DEADLINE")"
if [ "$_got" = "3" ]; then
	ok "exit 3 reported 3"
else
	miss "exit 3 reported $_got"
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 5. a marker-shaped line with the wrong nonce does not end the command"
_got="$(run_case forgery "echo VMR-END-0123456789abcdef 0; exit 7" "$CMD_DEADLINE")"
if [ "$_got" = "7" ]; then
	ok "the forged line was ignored and the real status 7 reported"
else
	miss "the forgery reported $_got instead of 7"
fi
fi

if [ "$fail" -eq 0 ]; then
say ""
say "== 6. an overlong command reports a deadline"
_got="$(run_case deadline "sleep 30" 8)"
if [ "$_got" = "DEADLINE" ]; then
	ok "sleep 30 past an 8 s deadline reported DEADLINE, which is no status"
else
	miss "the overlong command reported $_got"
fi
fi

cleanup
QEMU=""
READER=""

say ""
if [ "$fail" -eq 0 ]; then
	say "147 acceptance: statuses cross the line, and only the line"
	rm -f "$(dirname "$OUT")/podvm-exec-guest.log" "$(dirname "$OUT")/podvm-exec-qemu.log"
else
	say "147 acceptance: FAILED"
fi
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
