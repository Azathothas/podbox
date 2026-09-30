#!/bin/bash
# Question: does every stalled SSH operation end inside its bound, does a
# dropped node socket redial and pair again, and do the workers clean up?
#
# TODO/podssh.md T-1406 (bound DNS and writes, node reconnect). The
# simultaneous-session proof in 387 covers sessions that stay up; this
# drive covers the operations that do not: a stalled resolver, a peer that
# stops reading, and a relay that drops the node socket mid-service.
#
# Loopback only: the fault tests stall injected closures and unread
# sockets, and the reconnect proof runs the lane-built node and operator
# against the crate's loopback fake relay. No live relay, no KVM guest,
# no traffic past 127.0.0.1. Clause and hygiene shape mirrors 387: the
# results file carries statuses, counts, and durations, never a token
# (no real token exists on this path; the fixed test constants are
# grepped out of the log the same way).
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="experiments/results/ssh-liveness.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT HUP INT TERM
fail=0

say() { echo "$@" >>"$OUT"; }

{
  echo "== conditions"
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  git rev-parse --short HEAD
  cargo --version
  rustc --version
} >"$OUT"

bash scripts/common/bootstrap-env.sh rust cc zig tools openssh >"$work/bootstrap.log" 2>&1 || exit 2
mkdir -p /run/sshd || exit 2
command -v timeout >>"$OUT" 2>&1 || exit 2

# Clause 1: the lane-built binaries and test harness exist.
start=$(date +%s)
if timeout 25m cargo build -p podbox-ssh --bins >>"$OUT" 2>&1; then
  say "clause 1          cargo build -p podbox-ssh --bins exit 0 in $(( $(date +%s) - start ))s"
else
  say "clause 1          cargo build FAILED"
  exit 2
fi

# Clause 2: a stalled resolver ends within its bound. The fault test
# stalls the resolution seam (no real DNS stall is loopback-reachable)
# and asserts the 1 s bound both sides: waited out, then failed loud.
start=$(date +%s)
if timeout 10m cargo test -p podbox-ssh --lib transport::tests::stalled_resolver \
  >"$work/clause2.log" 2>&1; then
  say "clause 2          stalled resolver ends within its bound in $(( $(date +%s) - start ))s"
else
  say "clause 2          stalled-resolver fault test FAILED"
  tail -20 "$work/clause2.log" >>"$OUT" 2>/dev/null || true
  fail=1
fi

# Clause 3: a non-reading peer ends writes within the bound, on Unix,
# TCP, pipe, and frame legs, naming the leg at the guarded actions.
start=$(date +%s)
if timeout 10m cargo test -p podbox-ssh --lib -- \
  transport::tests::non_reading_unix_peer_ends_writes_within_the_bound \
  transport::tests::non_reading_tcp_peer_ends_writes_within_the_bound \
  transport::tests::pipe_write_names_its_leg_on_a_stall \
  ws::tests::stalled_frame_send_ends_within_the_write_bound_naming_the_leg \
  >"$work/clause3.log" 2>&1; then
  say "clause 3          stalled writes end within their bounds in $(( $(date +%s) - start ))s"
else
  say "clause 3          stalled-write fault tests FAILED"
  tail -20 "$work/clause3.log" >>"$OUT" 2>/dev/null || true
  fail=1
fi

# Clause 4: resolver and pipe workers leave nothing behind across
# twenty iterations: the kernel thread count settles to zero.
start=$(date +%s)
if timeout 10m cargo test -p podbox-ssh --lib \
  transport::tests::resolver_workers_do_not_accumulate_across_iterations \
  >"$work/clause4.log" 2>&1; then
  say "clause 4          worker cleanup holds in $(( $(date +%s) - start ))s"
else
  say "clause 4          worker-cleanup fault test FAILED"
  tail -20 "$work/clause4.log" >>"$OUT" 2>/dev/null || true
  fail=1
fi

# Clause 5: reconnect pairing. The fake relay drops the first node socket
# after its first completed session; the non---once node redials the same
# name, a second operator session pairs on the new socket, both sessions
# complete, the count proves exactly one redial, and the port rebinds.
start=$(date +%s)
if timeout 15m cargo test -p podbox-ssh --test mux_two_client \
  node_redial_pairs_a_second_session_after_the_first_socket_drops \
  >"$work/clause5.log" 2>&1; then
  say "clause 5          reconnect pairing holds in $(( $(date +%s) - start ))s"
else
  say "clause 5          reconnect proof FAILED"
  tail -30 "$work/clause5.log" >>"$OUT" 2>/dev/null || true
  fail=1
fi

# Clause 6: no token in the committed log. The fixed test constants are
# grepped the way 387 greps real tokens: a match fails the run.
for t in "node-tok-z" "conn-tok-z"; do
  if grep -qF "$t" "$OUT"; then
    say "clause 6          TEST CONSTANT LEAK IN RESULTS LOG"
    fail=1
  fi
done
if [ "$fail" -eq 0 ]; then
  say "clause 6          no token in the committed log"
fi

# Clause 7: no residue. No lane-built node or operator process survives
# the suite, scanned through /proc so no tool is assumed.
leftover=0
for pid_dir in /proc/[0-9]*; do
  if tr '\0' ' ' <"$pid_dir/cmdline" 2>/dev/null | grep -q "target/.*/debug/\(node\|operator\)"; then
    say "clause 7          leftover process: $pid_dir $(tr '\0' ' ' <"$pid_dir/cmdline" 2>/dev/null)"
    leftover=1
  fi
done
if [ "$leftover" -eq 0 ]; then
  say "clause 7          no node or operator process left behind"
else
  fail=1
fi

{
  echo ""
  if [ "$fail" -eq 0 ]; then echo "verdict           STALLED OPS END BOUNDED, NODE REDIALS AND PAIRS"; else echo "verdict           SSH LIVENESS PROOF OPEN"; fi
} >>"$OUT"
cp "$OUT" /out/ 2>/dev/null || true
cat "$OUT"
[ "$fail" -eq 0 ]
