#!/bin/sh
# Question: what does the syscall tier cost against the loader family for a
# dynamic program, where chroot is denied?
#
# TODO/enter.md T-1740. A raw system call of a dynamic program passes the
# interposer of the loader family (the default); the syscall tier gives
# each call to podvm's handler (T-1731), at the price of a SIGSYS and a
# return for each call. This prints the wall time of three workloads of
# alpine's dynamic busybox on each, on one host.
#
# Fixture: the chroot-denying wrapper of experiment 356. Pinned input:
# alpine 3.20 of mirror.gcr.io (PODVM_438_IMAGE overrides).
#
# Clauses:
#   1. each workload exits 0 on each (the default enters the userland
#      rung's loader family; the other enters the syscall tier).
#   2. the wall times are printed: `exec` (2000 runs of cat), `io` (md5sum
#      of the image's files, 20 times) and `calls` (dd bs=1 of 200000
#      bytes, two system calls for each byte).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep438-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/raw-syscall-cost.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMG="${PODVM_438_IMAGE:-mirror.gcr.io/library/alpine:3.20}"
export PODVM_STORE="${PODVM_438_STORE:-$WORK/store}"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
trap 'rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" || { echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
timeout 600 "$PB" pull "$IMG" >"$WORK/pull.log" 2>&1 || { echo "pull FAILED: COULD NOT RUN" >&2; tail -2 "$WORK/pull.log" >&2; exit 2; }

EXEC='i=0; while [ $i -lt 2000 ]; do cat /etc/alpine-release >/dev/null; i=$((i+1)); done'
IO='i=0; while [ $i -lt 20 ]; do find /usr /lib /etc /bin /sbin -type f | xargs md5sum >/dev/null; i=$((i+1)); done'
CALLS='dd if=/dev/zero of=/dev/null bs=1 count=200000 2>/dev/null'

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "host CPUs         $(nproc)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note "image             $IMG"
note ""

note "== clauses 1 and 2  the wall time (ms) of each workload"
note "  workload  loader-family  syscall-tier  rung of the default"
for w in exec io calls; do
	case $w in exec) cmd=$EXEC ;; io) cmd=$IO ;; calls) cmd=$CALLS ;; esac
	row="  $w"
	for tier in "" "--podvm-tier=syscall"; do
		t0=$(date +%s%N)
		timeout 900 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" run --rm --report "$WORK/r.json" $tier "$IMG" sh -c "$cmd" >/dev/null 2>"$WORK/err"
		rc=$?
		t1=$(date +%s%N)
		[ "$rc" -eq 0 ] || { bad "$w ${tier:-(default)} exited $rc"; tail -2 "$WORK/err" | sed 's/^/    /' | tee -a "$OUT"; }
		row="$row  $(( (t1 - t0) / 1000000 ))"
		[ -z "$tier" ] && rung=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["rung"])' "$WORK/r.json" 2>/dev/null)
	done
	note "$row  ${rung:--}"
done
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: EACH WORKLOAD RUNS ON EACH; THE TIMES ARE ABOVE"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
