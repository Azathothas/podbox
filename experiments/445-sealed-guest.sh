#!/bin/sh
# Question: does a sealed run of the machine tier (`--podvm-sealed`) keep
# each value of the run out of the host: no canary in the argv or the
# environment of a host process, in a file of the run's directories or in
# the state of podvm, while the guest gets each value?
#
# TODO/machine.md T-1751 (ask V-7 of podswarm, PodUtils/podcrates#124).
# The bundle is a tar stream on podvm's stdin; a canary is in a file name,
# a file body, an argv element and a variable. Run as root: the scan reads
# /proc/PID/environ of each process.
#
# Pinned inputs:
#   OCI     alpine 3.20 of the store PODVM_STORE (default the store of
#           experiment 443)
#   podvm   PODVM_445_BIN (default the release build of the workspace)
#
# Clauses:
#   1. the guest gets each value: the command prints the file body (read
#      by its name), the variable and the argv canary, in the working
#      directory of the bundle; podvm exits with the command's code (7).
#   2. during the run, no canary in /proc/PID/cmdline or /proc/PID/environ
#      of any process, nor in a file under PODVM_STORE or TMPDIR (names
#      and bodies). The scan runs while the guest sleeps, and it must read
#      the environment of the emulator (a control).
#   3. after the run, the same scan finds no canary.
#   4. /sys/class/net in the guest holds `lo` only.
#   5. a sealed run refuses a command, -e and -v (exit 125) and names no
#      value of the bundle; a bundle with no .podvm-run.json is refused.
#   6. no emulator stays.
#
# Each run has a bound of 300 s. Exit: 0 each clause matched, 1 one or
# more failed, 2 the lane could not run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PB="${PODVM_445_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_STORE:-$REPO/.work/qa/store}"
A=mirror.gcr.io/library/alpine:3.20
W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT INT TERM

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
[ "$(id -u)" = 0 ] || { echo "the scan needs root: COULD NOT RUN" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
"$PB" image inspect "$A" >/dev/null 2>&1 || "$PB" pull -q "$A" >/dev/null 2>&1 ||
	{ echo "alpine 3.20 does not pull: COULD NOT RUN" >&2; exit 2; }

# The canaries: built from parts, so that no argv of this script holds one.
P=c4n4ry445
N="${P}name"; B="${P}body"; V="${P}var"; G="${P}argv"
python3 - "$W/bundle.tar" "$P" <<'EOF' || { echo "the bundle does not build: COULD NOT RUN" >&2; exit 2; }
import io, json, sys, tarfile
out, p = sys.argv[1], sys.argv[2]
run = {"argv": ["sh", "-c", 'cat ./' + p + 'name; echo "$CANARY"; echo "$0"; pwd; ls /sys/class/net; sleep 20; exit 7', p + "argv"],
       "env": ["CANARY=" + p + "var"], "workdir": "/sealed"}
with tarfile.open(out, "w") as t:
    def add(name, data, mode=0o644):
        i = tarfile.TarInfo(name); i.size = len(data); i.mode = mode
        t.addfile(i, io.BytesIO(data))
    d = tarfile.TarInfo("sealed"); d.type = tarfile.DIRTYPE; d.mode = 0o755; t.addfile(d)
    add(".podvm-run.json", json.dumps(run).encode())
    add("sealed/" + p + "name", (p + "body\n").encode())
EOF

FAIL=0
verdict() { # clause, test, what
	if eval "$2"; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; FAIL=1; fi
}
# The scan: each canary in the argv and the environment of each process
# (but this script's own, which builds them), and in the names and bodies
# of the files of the store and of TMPDIR.
# With an argument, the scan must also read the environment of the
# emulator: a scan that reads nothing finds nothing (a control).
scan() {
	hits=""
	seen=""
	for d in /proc/[0-9]*; do
		pid=${d#/proc/}
		[ "$pid" = $$ ] && continue
		for f in cmdline environ; do
			if { tr '\0' '\n' <"$d/$f" | grep -q "$P"; } 2>/dev/null; then
				hits="$hits $pid/$f($( { tr '\0' ' ' <"$d/cmdline"; } 2>/dev/null | cut -c1-60))"
			fi
		done
		if { grep -q qemu-system "$d/cmdline" && tr '\0' '\n' <"$d/environ" | grep -q '^PATH='; } 2>/dev/null; then
			seen=1
		fi
	done
	[ -n "${1:-}" ] && [ -z "$seen" ] && hits="$hits no-environ-of-the-emulator-read"
	for root in "$PODVM_STORE" "${TMPDIR:-/tmp}"; do
		n=$(find "$root" -path "$W" -prune -o -name "*$P*" -print 2>/dev/null | head -3)
		[ -n "$n" ] && hits="$hits names:$n"
		b=$(grep -rl --exclude-dir="$(basename "$W")" "$P" "$root" 2>/dev/null | head -3)
		[ -n "$b" ] && hits="$hits bodies:$b"
	done
	echo "$hits"
}

timeout 300 "$PB" run --rm --podvm-tier=machine --podvm-sealed "$A" <"$W/bundle.tar" >"$W/out" 2>"$W/err" &
RUN=$!
i=0
while ! grep -q "^lo" "$W/out" 2>/dev/null && [ $i -lt 2400 ]; do sleep 0.1; i=$((i + 1)); done
during=$(scan control)
wait $RUN
rc=$?
sed 's/^/  1| /' "$W/out"
sed 's/^/  1e| /' "$W/err"
verdict 1 '[ $rc = 7 ] && [ "$(sed -n 1p "$W/out")" = "$B" ] && [ "$(sed -n 2p "$W/out")" = "$V" ] && [ "$(sed -n 3p "$W/out")" = "$G" ] && [ "$(sed -n 4p "$W/out")" = /sealed ]' \
	"the guest gets the file by its name, the variable, the argv and the workdir; exit 7 ($rc)"
verdict 2 '[ -z "$during" ]' "during the run, no canary on the host:${during:- none}"
after=$(scan)
verdict 3 '[ -z "$after" ]' "after the run, no canary on the host:${after:- none}"
verdict 4 '[ "$(sed -n 5,6p "$W/out")" = lo ]' "the guest has lo only"

r1=$(timeout 60 "$PB" run --rm --podvm-tier=machine --podvm-sealed "$A" true <"$W/bundle.tar" 2>&1; echo "rc=$?")
r2=$(timeout 60 "$PB" run --rm --podvm-tier=machine --podvm-sealed -e X=1 "$A" <"$W/bundle.tar" 2>&1; echo "rc=$?")
r3=$(timeout 60 "$PB" run --rm --podvm-tier=machine --podvm-sealed -v "$W:/x" "$A" <"$W/bundle.tar" 2>&1; echo "rc=$?")
printf 'x' >"$W/not.tar"
r4=$(timeout 60 "$PB" run --rm --podvm-tier=machine --podvm-sealed "$A" <"$W/not.tar" 2>&1; echo "rc=$?")
echo "  5| $r1" "|" "$r2" "|" "$r3" "|" "$r4"
refused() { # each answer is 125 and names no canary
	for r in "$r1" "$r2" "$r3" "$r4"; do
		case "$r" in *"rc=125"*) ;; *) return 1 ;; esac
		case "$r" in *"$P"*) return 1 ;; esac
	done
}
verdict 5 refused \
	"a command, -e, -v and a bad bundle are refused (125), and no refusal names a value"

sleep 1
left=$(pgrep -af "qemu-system-" 2>/dev/null | cut -c1-120)
verdict 6 '[ -z "$left" ]' "no emulator stays (${left:-none})"

if [ $FAIL = 0 ]; then echo "A SEALED RUN LEAVES NO VALUE OF THE RUN ON THE HOST"; fi
exit $FAIL
