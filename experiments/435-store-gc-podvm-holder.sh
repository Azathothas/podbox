#!/bin/sh
# Question: does a store GC skip an image because podvm itself holds the
# image lock across its own exec, with no container record to stop it first?
#
# TODO/image.md T-0204. `podvm image prune` and `podvm rmi` read the
# container records first (`is referenced by container NAME`), and the
# `flock(2)` of the image lock last (`is in use by a running container`).
# While a container runs, its record always answers first, so a plain run
# never reaches the lock. This script removes the record table while a
# detached container runs. Then only the lock that `Lock::hand_to_payload`
# gave to the payload can stop the GC. Experiment 160 drives the same skip
# with flock(1) as the holder.
#
# Pinned input: ALPINE of experiments/356-no-chroot-rung.sh (alpine 3.20 by
# digest). PODVM_435_MIRROR=1 takes the same digest from mirror.gcr.io.
#
# Clauses:
#   1. a detached run of `sleep` starts, and the payload (`sleep`) holds
#      the image lock fd after its exec. The podvm launcher holds the same
#      open file description for the life of the container.
#   2. with the record table, prune names the container
#      (`is referenced by container gc-probe`) and keeps the image.
#   3. with no record table, prune exits 0, prints
#      `skipped: NAME is in use by a running container`, and keeps the image.
#   4. with no record table, rmi exits 125 and says
#      `is in use by a running container`, and the image stays.
#   5. with the record table back, `rm -f gc-probe` ends the payload, and
#      prune then deletes the image.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run (no binary, no image).
set -u

REPO="$(pwd)"
BIN="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
OUT="$REPO/experiments/results/store-gc-podvm-holder.txt"
WORK="$(mktemp -d)"
STORE="$WORK/store"
export PODVM_STORE="$STORE"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
[ "${PODVM_435_MIRROR:-0}" = 1 ] && ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
TABLE="$STORE/containers.json"
ASIDE="$WORK/containers.json.aside"
cleanup() {
	[ -f "$ASIDE" ] && mv "$ASIDE" "$TABLE"
	"$BIN" rm -f gc-probe >/dev/null 2>&1
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

[ -x "$BIN" ] || { echo "SKIP: $BIN is not built: cargo build --release -p podvm" >&2; exit 2; }
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$BIN" version 2>/dev/null | head -1)"
note "image             $ALPINE"
note ""

timeout 600 "$BIN" pull -q "$ALPINE" >"$WORK/pull.log" 2>&1 || {
	note "pull FAILED: COULD NOT RUN"
	sed 's/^/  /' "$WORK/pull.log" | tee -a "$OUT"
	exit 2
}
NAME=$("$BIN" images --format '{{.Repository}}:{{.Tag}}' 2>/dev/null | head -1)
[ -n "$NAME" ] || { note "no image after the pull: COULD NOT RUN"; exit 2; }

# ---- clause 1: the payload holds the lock --------------------------------
note "== clause 1  the holder of the image lock is the payload"
timeout 120 "$BIN" run -d --name gc-probe "$ALPINE" sleep 120 >"$WORK/run.log" 2>&1
rc=$?
note "  run -d exited $rc  (want 0)"
[ "$rc" -eq 0 ] || { bad "the detached run did not start"; sed 's/^/    /' "$WORK/run.log" | tee -a "$OUT"; }
LOCK=$(ls "$STORE"/locks/*.lock 2>/dev/null | head -1)
holders=""
i=0
while [ "$i" -lt 50 ] && [ -z "$holders" ]; do
	for fd in /proc/[0-9]*/fd/*; do
		[ "$(readlink "$fd" 2>/dev/null)" = "$LOCK" ] || continue
		p=${fd#/proc/}; p=${p%%/*}
		holders="$holders $p:$(tr '\0' ' ' <"/proc/$p/cmdline" 2>/dev/null | sed 's#^[^ ]*/##' | cut -c1-40)"
	done
	[ -n "$holders" ] || { sleep 0.1; i=$((i + 1)); }
done
note "  lock file         ${LOCK#"$STORE"/}"
note "  holders           ${holders:- none}"
case "$holders" in
*":sleep 120"*) ;;
*) bad "the payload does not hold the image lock" ;;
esac
note ""

# ---- clause 2: the record answers first ----------------------------------
note "== clause 2  with the record table, prune names the container"
timeout 120 "$BIN" image prune -a -f >"$WORK/p2.log" 2>&1
rc=$?
sed 's/^/    /' "$WORK/p2.log" | tee -a "$OUT"
note "  prune exited $rc  (want 0)"
grep -q "is referenced by container gc-probe" "$WORK/p2.log" || bad "prune did not name gc-probe"
"$BIN" inspect "$ALPINE" >/dev/null 2>&1 || bad "the image is gone"
note ""

# ---- clauses 3 and 4: the lock is the last word --------------------------
mv "$TABLE" "$ASIDE" || { note "no record table at ${TABLE#"$WORK"/}: COULD NOT RUN"; exit 2; }
note "== clause 3  with no record table, prune skips the held image"
timeout 120 "$BIN" image prune -a -f >"$WORK/p3.log" 2>&1
rc=$?
sed 's/^/    /' "$WORK/p3.log" | tee -a "$OUT"
note "  prune exited $rc  (want 0)"
[ "$rc" -eq 0 ] || bad "prune exited $rc"
grep -q "skipped: .* is in use by a running container" "$WORK/p3.log" || bad "prune did not skip the held image by name"
grep -q "is referenced by container" "$WORK/p3.log" && bad "a record answered, so the lock was not read"
"$BIN" inspect "$ALPINE" >/dev/null 2>&1 || bad "the image is gone"
note ""

note "== clause 4  with no record table, rmi refuses the held image"
timeout 120 "$BIN" rmi "$NAME" >"$WORK/r4.log" 2>&1
rc=$?
sed 's/^/    /' "$WORK/r4.log" | tee -a "$OUT"
note "  rmi exited $rc  (want 125)"
[ "$rc" -eq 125 ] || bad "rmi exited $rc"
grep -q "is in use by a running container" "$WORK/r4.log" || bad "rmi did not name the hold"
"$BIN" inspect "$ALPINE" >/dev/null 2>&1 || bad "the image is gone"
mv "$ASIDE" "$TABLE"
note ""

# ---- clause 5: the release ------------------------------------------------
note "== clause 5  after rm -f, prune deletes the image"
timeout 120 "$BIN" rm -f gc-probe >"$WORK/rm.log" 2>&1
rc=$?
note "  rm -f exited $rc  (want 0)"
[ "$rc" -eq 0 ] || bad "rm -f exited $rc"
timeout 120 "$BIN" image prune -a -f >"$WORK/p5.log" 2>&1
rc=$?
sed 's/^/    /' "$WORK/p5.log" | tee -a "$OUT"
note "  prune exited $rc  (want 0)"
grep -q "^deleted: " "$WORK/p5.log" || bad "prune deleted nothing after the release"
"$BIN" inspect "$ALPINE" >/dev/null 2>&1 && bad "the image is still there"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: PODVM HOLDS THE IMAGE LOCK ACROSS ITS EXEC, AND THE GC SKIPS IT"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
