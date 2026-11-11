#!/bin/sh
# Question: does `run --report PATH` leave one JSON document of the run,
# with the rung, the exit code, the peak resident set and the labels, and
# does `ps --filter label=` find the run?
#
# TODO/cli.md T-1695 (A-V-09, A-V-07 and G-06 of PodUtils/podcrates#39).
# Pinned input: busybox:musl of mirror.gcr.io (PODVM_426_IMAGE overrides).
#
# Clauses:
#   1. `run --label job=7 --report R.json IMG sh -c 'exit 3'` exits 3 and
#      leaves one valid JSON document: the rung, exit_code 3, signal null,
#      max_rss_kib above 0, wall_ms, and the label job=7.
#   2. `ps -a --filter label=job=7` lists the run, and label=job=8 does not.
#   3. a run that podvm refuses (an image that the store does not have,
#      with --pull never) still leaves its report: refused 125, exit_code
#      null, and no temporary file beside it.
#   4. no value of the environment is in a report (`-e SECRET=...`).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep426-work"
rm -rf "$WORK"; mkdir -p "$WORK/out" || exit 2
OUT="$REPO/experiments/results/run-report.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMG="${PODVM_426_IMAGE:-mirror.gcr.io/library/busybox:musl}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
trap 'rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
field() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); v=d
for k in sys.argv[2].split("."): v=v[int(k)] if isinstance(v,list) else v[k]
print(json.dumps(v))' "$1" "$2"; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "image             $IMG"
note ""
timeout 600 "$PB" pull "$IMG" >"$WORK/pull.log" 2>&1 || { note "pull FAILED: COULD NOT RUN"; tail -2 "$WORK/pull.log"; exit 2; }

note "== clause 1  the report of a run"
R="$WORK/out/r.json"
timeout 120 "$PB" run --label job=7 --report "$R" -e SECRET=s3cr3t-426 "$IMG" sh -c 'exit 3' >/dev/null 2>"$WORK/c1.err"
rc=$?
note "  exit $rc"
[ "$rc" -eq 3 ] || bad "the run exited $rc"
if python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$R" 2>/dev/null; then
	for k in rung exit_code signal max_rss_kib wall_ms labels user_ms; do
		note "  $k: $(field "$R" "$k")"
	done
	[ "$(field "$R" exit_code)" = 3 ] || bad "exit_code is not 3"
	[ "$(field "$R" signal)" = null ] || bad "signal is not null"
	[ "$(field "$R" rung)" != null ] || bad "the report names no rung"
	[ "$(field "$R" labels.0)" = '"job=7"' ] || bad "the label is not in the report"
	rss=$(field "$R" max_rss_kib)
	[ "$rss" != null ] && [ "$rss" -gt 0 ] || bad "max_rss_kib is not above 0"
else
	bad "no valid JSON document at the report path"
fi
note ""

note "== clause 2  ps finds the run by its label"
ids=$(timeout 60 "$PB" ps -a -q --filter label=job=7)
none=$(timeout 60 "$PB" ps -a -q --filter label=job=8)
note "  label=job=7: $(echo "$ids" | grep -c .) record(s); label=job=8: $(echo "$none" | grep -c .)"
[ -n "$ids" ] || bad "ps --filter label=job=7 lists nothing"
[ -z "$none" ] || bad "ps --filter label=job=8 lists a record"
note ""

note "== clause 3  a refusal leaves its report"
Q="$WORK/out/refused.json"
timeout 60 "$PB" run --pull never --report "$Q" localhost/podvm-426-absent:none true >/dev/null 2>"$WORK/c3.err"
rc=$?
note "  exit $rc; refused: $(field "$Q" refused 2>/dev/null); exit_code: $(field "$Q" exit_code 2>/dev/null)"
[ "$rc" -eq 125 ] || bad "the refused run exited $rc"
[ "$(field "$Q" refused 2>/dev/null)" = 125 ] || bad "the report of the refusal does not say refused 125"
[ "$(field "$Q" exit_code 2>/dev/null)" = null ] || bad "the refusal has an exit code"
[ "$(ls -A "$WORK/out" | grep -c '\.tmp$')" -eq 0 ] || bad "a temporary file stayed"
note ""

note "== clause 4  no value of the environment"
if grep -q "s3cr3t-426" "$R" 2>/dev/null; then bad "the report holds the value of SECRET"; else note "  the value of SECRET is not in the report"; fi
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: ONE REPORT FOR EACH RUN, WITH ITS LABELS, AND PS FINDS IT"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
