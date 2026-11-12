#!/bin/sh
# Question: does `run --podvm-tier=runsc` run a payload under gVisor where
# the host grants user namespaces, and refuse by name where it does not?
#
# TODO/enter.md T-1729. podvm writes an OCI bundle around the container's
# own tree and starts `runsc --rootless run` (crates/podvm/src/runsc.rs).
#
# Pinned input: gVisor release 20261102 for x86_64 (the SHA-512 below),
# runsc with its sidecars of gvisor-bin/, or PODVM_436_RUNSC names a
# runsc; alpine 3.20 of mirror.gcr.io.
#
# Clauses:
#   1. `uname -r` names gVisor, and the exit code of the payload is the
#      exit code of the run.
#   2. a write to a `-v` volume is on the host after the run, and stdin
#      reaches the payload.
#   3. `--read-only` and a `:ro` volume refuse a write with EROFS.
#   4. the container's tree and its bundle are gone after the run.
#   5. with no runsc (PODVM_RUNSC names no file), the tier refuses at 125
#      and names T-1729.
#   6. where unshare(2) is denied (experiments/deny-unshare.c), the tier
#      refuses at 125 and names the user namespaces.
#   7. `exec --podvm-tier=runsc` refuses by name.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep436-work"
OUT="$REPO/experiments/results/runsc.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMG="${PODVM_436_IMAGE:-mirror.gcr.io/library/alpine:3.20}"
REL=20261102
SUM=2e6671bb2d033b871c5811ab57e16ac570988b0e0054c525f911cfaf191bab3ca4efb08e0a98daa4b9e7d248f9e5776ef2ef2f8a6b82d829646a42924c98bb05
mkdir -p "$WORK" || exit 2
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
[ "$(uname -m)" = x86_64 ] || [ -n "${PODVM_436_RUNSC:-}" ] || { echo "the pinned runsc is x86_64: set PODVM_436_RUNSC: COULD NOT RUN" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-unshare" "$REPO/experiments/deny-unshare.c" || { echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }

RUNSC="${PODVM_436_RUNSC:-$WORK/gvisor/runsc}"
if [ ! -x "$RUNSC" ]; then
	mkdir -p "$WORK/gvisor" || exit 2
	curl -qfsSL -o "$WORK/gvisor.tar.bz2" "https://storage.googleapis.com/gvisor/releases/release/$REL/x86_64/gvisor.tar.bz2" || { echo "gVisor download FAILED: COULD NOT RUN" >&2; exit 2; }
	echo "$SUM  $WORK/gvisor.tar.bz2" | sha512sum -c - >/dev/null || { echo "gVisor SHA-512 MISMATCH: COULD NOT RUN" >&2; exit 2; }
	tar -xjf "$WORK/gvisor.tar.bz2" -C "$WORK/gvisor" runsc gvisor-bin || { echo "gVisor unpack FAILED: COULD NOT RUN" >&2; exit 2; }
fi
export PODVM_RUNSC="$RUNSC"
timeout 600 "$PB" pull "$IMG" >"$WORK/pull.log" 2>&1 || { echo "pull FAILED: COULD NOT RUN" >&2; tail -2 "$WORK/pull.log" >&2; exit 2; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "runsc             $("$RUNSC" --version 2>/dev/null | head -1)"
note "image             $IMG"
note ""

VOL="$WORK/vol"; rm -rf "$VOL"; mkdir -p "$VOL"
run() { timeout 300 "$PB" run --rm --podvm-tier=runsc "$@"; }

note "== clause 1  uname -r and the exit code"
r=$(run "$IMG" sh -c 'uname -r; exit 3' 2>"$WORK/e1"); rc=$?
note "  uname -r: $r, exit $rc"
case "$r" in *gvisor*) ;; *) bad "uname -r does not name gVisor" ;; esac
[ "$rc" -eq 3 ] || bad "the exit code of the payload was not the run's"

note "== clause 2  a volume write and stdin"
r=$(echo in436 | run -v "$VOL:/workspace" "$IMG" sh -c 'read l; echo "$l" > /workspace/w.txt; echo ok' 2>"$WORK/e2"); rc=$?
note "  exit $rc, host file: $(cat "$VOL/w.txt" 2>/dev/null || echo -)"
[ "$rc" -eq 0 ] && [ "$r" = ok ] || bad "the run did not answer ok"
[ "$(cat "$VOL/w.txt" 2>/dev/null)" = in436 ] || bad "the write with stdin's line is not on the host"

note "== clause 3  --read-only and :ro"
r=$(run --read-only -v "$VOL:/ro:ro" "$IMG" sh -c 'touch /x 2>&1; touch /ro/y 2>&1; true' 2>"$WORK/e3")
note "$(echo "$r" | sed 's/^/  /')"
[ "$(echo "$r" | grep -c 'Read-only file system')" -eq 2 ] || bad "a write was not refused with EROFS"
[ -e "$VOL/y" ] && bad "a write under :ro is on the host"

note "== clause 4  nothing of the run stays"
left=$(ls -A "$PODVM_STORE/trees" 2>/dev/null; ls -A "$PODVM_STORE/runsc" 2>/dev/null | grep -v '^state$')
note "  left: ${left:-none}"
[ -z "$left" ] || bad "a tree or a bundle stayed after the run"

note "== clause 5  no runsc"
PODVM_RUNSC=/nonexistent/runsc timeout 60 "$PB" run --rm --podvm-tier=runsc "$IMG" true 2>"$WORK/e5"; rc=$?
note "  exit $rc: $(cut -c1-160 "$WORK/e5")"
[ "$rc" -eq 125 ] && grep -q "T-1729" "$WORK/e5" || bad "no runsc was not refused at 125 by name"

note "== clause 6  no user namespaces"
"$WORK/deny-unshare" timeout 60 "$PB" run --rm --podvm-tier=runsc "$IMG" true 2>"$WORK/e6"; rc=$?
note "  exit $rc: $(cut -c1-160 "$WORK/e6")"
[ "$rc" -eq 125 ] && grep -q "denies user namespaces" "$WORK/e6" || bad "a host with no user namespaces was not refused at 125 by name"

note "== clause 7  exec"
timeout 60 "$PB" exec --podvm-tier=runsc x true 2>"$WORK/e7"; rc=$?
note "  exit $rc: $(cut -c1-160 "$WORK/e7")"
grep -q "runsc serve run only" "$WORK/e7" || bad "exec --podvm-tier=runsc was not refused by name"
note ""
# runsc mounts null-netns in its state root where it runs as root.
umount "$PODVM_STORE/runsc/state/null-netns" 2>/dev/null

if [ "$fail" -eq 0 ]; then
	note "verdict: THE RUNSC TIER RUNS AND REFUSES BY NAME"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
