#!/bin/sh
# Question: does a SIGINT/SIGTERM to a foreground `run` reach the payload,
# and does one to a detached launcher reach it too?
#
# TODO/supervise.md T-1335. `stop`/`kill` reach the payload by pidfd, but
# the inverse -- the waiter itself receiving the shutdown signals (Ctrl-C
# on a foreground `run`, a TERM to the setsid'd launcher) -- had no
# specified behavior and no entry said whether the payload is forwarded
# the signal or dies with the waiter.
#
# ⭐ WHAT THIS DRIVES. Both waiters block SIGINT/SIGTERM and read them
# from a signalfd beside the payload's pidfd in one `ppoll`: the
# foreground waiter (`podbox-enter` `wait_forwarding`, naming each
# forward and a signaled end on stderr) and the setsid'd launcher
# (`podbox-supervise` `serve`, silent, the signaled exit in the table
# record). A payload that traps or ignores the signal is waited on, not
# killed: killing it would be `stop`'s decision made inside `run`.
#
# ⚠ THE SHELL BEQUEATHS AN IGNORED SIGINT. A `&`-backgrounded shell
# without job control ignores SIGINT, and SIG_IGN survives exec, so a
# payload started that way never dies on the signal -- and neither does
# the waiter, which inherits the same disposition. A POSIX `trap -` cannot
# reset an ignore that was inherited. Every foreground leg below runs
# through `env --default-signal=INT,QUIT`, which resets the bequeathed
# ignore to default before exec, the way a real terminal does. That is a
# property of the harness, not of podbox: no line of podbox resets a
# caller-bequeathed ignore, and whether a foreground `run` should is
# recorded in T-1335 as follow-up work, not expanded here.
#
# ⚠ THE PAYLOAD IS READY WHEN ITS ARGV IS VISIBLE. Killing podbox during
# extraction tests nothing, because there is no payload yet. A substring
# match also matches podbox's own argv (`podbox run IMG sleep 30`) from
# the instant it starts, so the waiter matches a full command line that
# IS `sleep 30`, which is the payload's exact argv and nothing else, with
# a pre-clean so a stale orphan cannot satisfy it either.
#
#   ./351-signal-forward.sh
#
# Exit: 0 every clause forwarded (or waited) as specified, 1 one did not,
#       2 could not run.
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash (the `325-` script carries the
# same note). Dash has no `pipefail`, and the option error would exit
# the whole job before a clause runs.
#
# ⛔ THE LANE SHAPE: a bare `run-in-base.sh` with this script as its
# argument stages this file at the checkout root as `.podbox-job.sh` and
# runs `sh` on it with cwd `/`, so `$0`'s directory is `/work` (never
# `experiments/`) and the wrapper's own `cd /work` runs only for ITS
# steps, not the job. The committed script therefore does NOT build:
# the lane job that drives it builds first and exports PODBOX_BIN at
# the guest path. A NATIVE run (`./experiments/351-signal-forward.sh`
# on Linux) builds the default first with
# `cargo build --release --target x86_64-unknown-linux-musl`, or sets
# PODBOX_BIN itself.
set -u
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
# ⚠ In the lane `$0` is `/work/.podbox-job.sh`, so HERE is /work and REPO
# is `/`: BIN must NOT derive from REPO there. PODBOX_BIN is the lane
# job's export; the /work default covers a manual lane `sh` only.
BIN="${PODBOX_BIN:-/work/target/x86_64-unknown-linux-musl/release/podbox}"
export PODBOX_STORE="${PODBOX_STORE:-/tmp/pb-351-store}"
# ⚠ OUT is /out-absolute where the lane mounts artifacts, else the
# checkout's results dir natively. A results file that lands only in the
# lane workspace dies with the container; /out is what `--artifacts`
# copies back (docs/containers.md). Native OUT keeps the committed path
# the entry's Prove cites.
if [ -d /out ]; then
  OUT="/out/signal-forward.txt"
else
  OUT="$REPO/experiments/results/signal-forward.txt"
fi
mkdir -p "$(dirname -- "$OUT")" || exit 2
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

IMAGE="${PODBOX_RUN_IMAGE:-public.ecr.aws/docker/library/alpine:3.20}"

fail=0
say() { printf '%s\n' "$*" | tee -a "$OUT"; }

need() {
  command -v "$1" >/dev/null 2>&1 || { say "MISSING-TOOL-$1"; exit 2; }
}
need setsid
need env
need pgrep
need pkill

: >"$OUT"
say "== T-1335: a shutdown signal to a waiter reaches the payload"
say "== taken $(date -u +%Y-%m-%dT%H:%M:%SZ) on $(uname -sr 2>/dev/null || echo unknown)"
say "== conditions"
say "bin: $BIN"
"$BIN" version 2>/dev/null | head -1 | tee -a "$OUT" || { say "NO-BINARY"; exit 2; }
command -v timeout >/dev/null 2>&1 || { say "MISSING-TOOL-timeout"; exit 2; }

say "== pull"
timeout 120 "$BIN" pull "$IMAGE" >>"$OUT" 2>&1 || { say "PULL-FAILED"; exit 2; }

# The payload is ready when a process whose FULL command line IS
# `sleep 30` exists: the payload's exact argv and nothing else (a
# substring would also match podbox's own `run` argv from start).
wait_payload() {
  n=0
  while [ "$n" -lt 30 ]; do
    pgrep -f '^sleep 30$' >/dev/null 2>&1 && return 0
    sleep 1
    n=$((n + 1))
  done
  return 1
}

# No stale payload may satisfy the waiter above.
clean_sleep() {
  pkill -9 -f '^sleep 30$' 2>/dev/null || true
  n=0
  while [ "$n" -lt 10 ]; do
    pgrep -f '^sleep 30$' >/dev/null 2>&1 || return 0
    sleep 1
    n=$((n + 1))
  done
  return 1
}

no_orphan() {
  sleep 1
  if pgrep -f '^sleep 30$' >/dev/null 2>&1; then say "ORPHAN-LEFT (bad)"; fail=1; else say "NO-ORPHAN-OK"; fi
}

say "== foreground TERM is forwarded and named, exit 143"
clean_sleep || { say "STALE-SLEEP (bad)"; fail=1; }
setsid env --default-signal=INT,QUIT "$BIN" run "$IMAGE" sleep 30 >"$WORK/term-out.txt" 2>"$WORK/term-err.txt" &
PBPID=$!
wait_payload || { say "PAYLOAD-NEVER-STARTED (bad)"; fail=1; }
kill -TERM "$PBPID"
wait "$PBPID"
RC=$?
say "term-rc=$RC"
cat "$WORK/term-err.txt" >>"$OUT" 2>/dev/null || true
if [ "$RC" -eq 143 ] && grep -q 'forwarded SIGTERM to the payload' "$WORK/term-err.txt" && grep -q 'died on SIGTERM: exit 143' "$WORK/term-err.txt"; then say "TERM-FORWARDED-OK"; else say "TERM-BAD"; fail=1; fi
no_orphan

say "== foreground INT is forwarded and named, the payload decides the exit"
clean_sleep || { say "STALE-SLEEP (bad)"; fail=1; }
setsid env --default-signal=INT,QUIT "$BIN" run "$IMAGE" sleep 30 >"$WORK/int-out.txt" 2>"$WORK/int-err.txt" &
PBPID=$!
wait_payload || { say "PAYLOAD-NEVER-STARTED (bad)"; fail=1; }
kill -INT "$PBPID"
wait "$PBPID"
RC=$?
say "int-rc=$RC"
cat "$WORK/int-err.txt" >>"$OUT" 2>/dev/null || true
# SIGINT is forwarded, never re-raised: the waiter stays to reap, and the
# exit is the payload's own. Busybox `sleep` dies on the default
# disposition, so 130 with both lines. A trapping payload would outlive
# it; the trap clause below pins that shape.
if [ "$RC" -eq 130 ] && grep -q 'forwarded SIGINT to the payload' "$WORK/int-err.txt" && grep -q 'died on SIGINT: exit 130' "$WORK/int-err.txt"; then say "INT-FORWARDED-OK"; else say "INT-BAD"; fail=1; fi
no_orphan

say "== a clean exit stays quiet"
timeout 60 "$BIN" run "$IMAGE" true >"$WORK/true-out.txt" 2>"$WORK/true-err.txt"
RC=$?
if [ "$RC" -eq 0 ] && ! grep -q 'forwarded\|died on' "$WORK/true-err.txt"; then say "QUIET-EXIT-OK"; else say "QUIET-BAD"; fail=1; fi

say "== the launcher forwards its own TERM, exit 143 recorded"
timeout 30 "$BIN" create --name fw1 "$IMAGE" sleep 30 >>"$OUT" 2>&1 || { say "CREATE-FW1-FAILED"; exit 1; }
timeout 60 "$BIN" start fw1 >>"$OUT" 2>&1 || { say "START-FW1-FAILED"; exit 1; }
LPID=$(timeout 30 "$BIN" inspect --format '{{.LauncherPid}}' fw1 2>/dev/null) || { say "NO-LAUNCHER (bad)"; fail=1; }
say "launcher-pid=$LPID"
kill -TERM "$LPID" 2>/dev/null || { say "KILL-LAUNCHER-FAILED (bad)"; fail=1; }
timeout 60 "$BIN" wait fw1 >"$WORK/fw-wait.txt" 2>&1
RC=$?
say "launcher-wait-rc=$RC"
cat "$WORK/fw-wait.txt" >>"$OUT" 2>/dev/null || true
if [ "$RC" -eq 0 ] && grep -qx '143' "$WORK/fw-wait.txt"; then say "LAUNCHER-FORWARDED-OK"; else say "LAUNCHER-BAD"; fail=1; fi
timeout 30 "$BIN" rm fw1 >>"$OUT" 2>&1 || true

say "== a payload that ignores TERM is waited on, not killed"
clean_sleep || { say "STALE-SLEEP (bad)"; fail=1; }
setsid env --default-signal=INT,QUIT "$BIN" run "$IMAGE" sh -c 'trap "" TERM; exec sleep 30' >/dev/null 2>"$WORK/trap-err.txt" &
PBPID=$!
wait_payload || { say "PAYLOAD-NEVER-STARTED (bad)"; fail=1; }
kill -TERM "$PBPID"
sleep 6
if kill -0 "$PBPID" 2>/dev/null; then say "TRAP-WAITED-OK"; else say "TRAP-DIED (bad)"; fail=1; fi
grep -q 'forwarded SIGTERM to the payload' "$WORK/trap-err.txt" && say "TRAP-FORWARD-LINE-OK" || { say "TRAP-LINE-BAD"; fail=1; }
kill -KILL "$PBPID" 2>/dev/null || true
wait "$PBPID" 2>/dev/null || true
pkill -9 -f '^sleep 30$' 2>/dev/null || true
sleep 1
if pgrep -f '^sleep 30$' >/dev/null 2>&1; then say "ORPHAN-LEFT (bad)"; fail=1; else say "NO-ORPHAN-OK"; fi

say "== verdict fail=$fail"
exit "$fail"
