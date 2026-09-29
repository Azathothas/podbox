#!/bin/bash
# Question: do the `podbox remote ssh` verbs drive real commands with real
# exit codes on both placements, and does every help and error path answer?
#
# TODO/podssh.md T-1404 (the remote dispatch). The loopback sshd proves the
# direct placement: `remote ssh forward` as an ssh ProxyCommand runs a real
# command and passes exit 42 through, twice over. The live r12 relay proves
# the relay placement: `remote ssh serve` registers the node and one real
# ssh session through `remote ssh connect` runs a real command and passes
# its exit code through. Help paths print usage with 0, error paths refuse
# with 125, and no token reaches the log.
#
# What this drive does not cover, stated: concurrent sessions over the
# relay stay with 387 (T-1403 proves two clients on one node connection,
# re-driven green this same night). This drive holds one relay session
# per pair on purpose: the relay shed load all night (503 at upgrade,
# 1011 backpressure, 1003 unknown-session across three runs), and a proof
# that needs a healthy relay hour is a proof about the hour, not the arm.
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="experiments/results/remote-ssh.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
trap 'kill "$SERVE_PID" "$SSHD_PID" 2>/dev/null; cp "$OUT" /out/ 2>/dev/null || true; rm -rf "$work"' EXIT HUP INT TERM
SERVE_PID=""
SSHD_PID=""
fail=0
RELAY="https://tcp.ssh.relay.ajam.dev"
WSS="wss://tcp.ssh.relay.ajam.dev"

say() { echo "$@" >>"$OUT"; }

{
  echo "== conditions"
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  git rev-parse --short HEAD
  cargo --version
  rustc --version
} >"$OUT"

bash scripts/common/bootstrap-env.sh rust cc zig openssh tools >"$work/bootstrap.log" 2>&1 || exit 2
mkdir -p /run/sshd || exit 2
command -v ssh >>"$OUT" 2>&1 || exit 2
command -v sshd >>"$OUT" 2>&1 || exit 2
command -v ssh-keygen >>"$OUT" 2>&1 || exit 2
command -v curl >>"$OUT" 2>&1 || exit 2
command -v jq >>"$OUT" 2>&1 || exit 2

# Clause 1: the lane-built podbox and ssh helper binaries exist.
if timeout 25m cargo build -p podbox-cli -p podbox-ssh --bins >>"$OUT" 2>&1; then
  say "clause 1          cargo build -p podbox-cli -p podbox-ssh --bins exit 0"
else
  say "clause 1          cargo build FAILED"
  exit 2
fi
PODBOX_BIN=$(ls -d target/*/debug/podbox 2>/dev/null | head -n 1)
PROXY_BIN=$(ls -d target/*/debug/proxy 2>/dev/null | head -n 1)
OPERATOR_BIN=$(ls -d target/*/debug/operator 2>/dev/null | head -n 1)
NODE_BIN=$(ls -d target/*/debug/node 2>/dev/null | head -n 1)
if [ -z "${PODBOX_BIN:-}" ] || [ -z "${PROXY_BIN:-}" ] \
    || [ -z "${OPERATOR_BIN:-}" ] || [ -z "${NODE_BIN:-}" ]; then
  say "clause 1          a binary is missing"
  exit 2
fi
# The group resolves its helpers beside the binary first: run the drive
# from a directory holding all four, so the sibling arm is the one proved.
mkdir -p "$work/bin"
cp "$PODBOX_BIN" "$PROXY_BIN" "$OPERATOR_BIN" "$NODE_BIN" "$work/bin/"
PODBOX="$work/bin/podbox"

# Clause 2: every help path prints usage with 0, and the bare old spelling
# answers with the group instead of "unknown command".
for path in "remote" "remote -h" "remote ssh" "remote ssh -h" \
    "remote ssh serve -h" "remote ssh connect -h" "remote ssh forward -h" \
    "machine" "machine ssh -h"; do
  # shellcheck disable=SC2086
  if timeout 60 $PODBOX $path >"$work/help.out" 2>&1; then
    code=0
  else
    code=$?
  fi
  if [ "$code" -eq 0 ] && grep -q "^usage:" "$work/help.out"; then
    say "clause 2          podbox $path: usage with 0"
  else
    say "clause 2          podbox $path FAILED (exit $code)"
    fail=1
  fi
done
timeout 60 $PODBOX ssh >"$work/ssh.out" 2>"$work/ssh.err"
scode=$?
if [ "$scode" -eq 125 ] && grep -q "podbox remote ssh" "$work/ssh.err"; then
  say "clause 2          podbox ssh refused with 125 naming remote ssh"
else
  say "clause 2          podbox ssh FAILED (exit $scode)"
  fail=1
fi
timeout 60 $PODBOX remote relay >"$work/relay.out" 2>"$work/relay.err"
rcode=$?
if [ "$rcode" -eq 125 ] && grep -q "no such member" "$work/relay.err"; then
  say "clause 2          podbox remote relay refused with 125 naming the member"
else
  say "clause 2          podbox remote relay FAILED (exit $rcode)"
  fail=1
fi
timeout 60 $PODBOX remote --bogus >"$work/flag.out" 2>"$work/flag.err"
fcode=$?
if [ "$fcode" -eq 125 ] && grep -q "parity table" "$work/flag.err"; then
  say "clause 2          podbox remote --bogus refused with 125 by the table"
else
  say "clause 2          podbox remote --bogus FAILED (exit $fcode)"
  fail=1
fi
timeout 60 $PODBOX remote ssh --bogus >"$work/sshdash.out" 2>"$work/sshdash.err"
sdcode=$?
if [ "$sdcode" -eq 125 ] && grep -q "is not a flag" "$work/sshdash.err"; then
  say "clause 2          podbox remote ssh --bogus refused with 125 as a flag, not a command"
else
  say "clause 2          podbox remote ssh --bogus FAILED (exit $sdcode)"
  fail=1
fi

# Clause 3: the direct placement. A key-only sshd daemon on loopback; the
# forward arm as ProxyCommand; a real command and exit 42 through it.
USER_NAME=$(id -un)
RT="$work/rt" && mkdir -p "$RT" && chmod 700 "$RT"
ssh-keygen -t ed25519 -N "" -q -f "$work/host_key" || exit 2
ssh-keygen -t ed25519 -N "" -q -f "$work/user_key" || exit 2
cp "$work/user_key.pub" "$work/authorized_keys"
cat >"$work/sshd_config" <<EOF
HostKey $work/host_key
AuthorizedKeysFile $work/authorized_keys
PidFile $work/sshd.pid
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
PermitRootLogin prohibit-password
UsePAM no
StrictModes no
PrintMotd no
PermitUserEnvironment no
EOF
chmod 600 "$work/sshd_config"
"$(command -v sshd)" -f "$work/sshd_config" -p 2223 -E "$work/sshd.log" || exit 2
SSHD_PID=$(cat "$work/sshd.pid" 2>/dev/null || true)
poll=0
while [ $poll -lt 30 ]; do
  if printf '' | timeout 10 ssh -o BatchMode=yes -o StrictHostKeyChecking=no \
      -o UserKnownHostsFile=/dev/null -o ConnectTimeout=5 -o LogLevel=ERROR \
      -i "$work/user_key" -l "$USER_NAME" -p 2223 localhost 2>/dev/null; then
    break
  fi
  sleep 1
  poll=$((poll + 1))
done
if [ "$poll" -ge 30 ]; then
  say "clause 3          sshd daemon never answered"
  exit 2
fi
ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o LogLevel=ERROR"
MARK="forward-marker-$((RANDOM * RANDOM))"
# shellcheck disable=SC2086
out=$(timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" \
  -o ProxyCommand="$PODBOX remote ssh forward tcp 127.0.0.1 2223" localhost \
  "echo $MARK" 2>"$work/fwd.err")
fcode=$?
if [ "$fcode" -eq 0 ] && [ "$out" = "$MARK" ]; then
  say "clause 3          forward runs the far command with exit 0"
else
  say "clause 3          forward FAILED (exit $fcode, err: $(head -1 "$work/fwd.err" 2>/dev/null))"
  fail=1
fi
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" \
  -o ProxyCommand="$PODBOX remote ssh forward tcp 127.0.0.1 2223" localhost \
  "exit 42" 2>/dev/null
f42=$?
if [ "$f42" -eq 42 ]; then
  say "clause 3          forward passes exit 42 through"
else
  say "clause 3          forward exit code FAILED: got $f42"
  fail=1
fi
timeout 60 $PODBOX remote ssh forward >"$work/fwdempty.out" 2>"$work/fwdempty.err"
fecode=$?
if [ "$fecode" -eq 125 ] && grep -q "needs a target" "$work/fwdempty.err"; then
  say "clause 3          forward with no target refused with 125 naming the target"
else
  say "clause 3          forward empty FAILED (exit $fecode)"
  fail=1
fi
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" \
  -o ProxyCommand="$PODBOX remote ssh forward tcp 127.0.0.1 1" localhost \
  "echo unreachable" 2>/dev/null
fref=$?
if [ "$fref" -ne 0 ]; then
  say "clause 3          forward to a refused port fails loud (ssh exit $fref)"
else
  say "clause 3          forward to a refused port read as success"
  fail=1
fi

# Clause 4: the relay placement through the new verbs. Pair, serve, online,
# a real command and exit 42 through connect, stop, and status after stop.
if ! curl -sS -m 20 "$RELAY/health?detail=1" -o "$work/health.json" >>"$OUT" 2>&1; then
  say "clause 4          relay health unreachable: could not run"
  exit 2
fi
say "relay version     $(jq -r .version "$work/health.json" 2>/dev/null || echo unknown)"
http=$(curl -sS -m 25 -X POST "$RELAY/v1/pair" -H "content-type: application/json" -d '{}' \
  -o "$work/pair.json" -w '%{http_code}') || exit 2
if [ "$http" != "200" ]; then
  say "clause 4          pair mint answered $http, not 200"
  exit 2
fi
NAME=$(jq -r .name "$work/pair.json")
NODE_TOKEN=$(jq -r .node_token "$work/pair.json")
CONNECT_TOKEN=$(jq -r .connect_token "$work/pair.json")
STOP_TOKEN=$(jq -r .stop_token "$work/pair.json")
if [ -z "${NAME:-}" ] || [ -z "${NODE_TOKEN:-}" ] || [ -z "${CONNECT_TOKEN:-}" ] || [ -z "${STOP_TOKEN:-}" ]; then
  say "clause 4          pair response missed a field"
  exit 2
fi
say "clause 4          pair minted (name withheld: see counts below)"
export PODSSH_RUNTIME="$RT" PODSSH_AUTHORIZED_KEYS="$work/authorized_keys"
"$PODBOX" remote ssh serve --relay "$WSS" --name "$NAME" --node-token "$NODE_TOKEN" \
  >"$work/serve.log" 2>&1 &
SERVE_PID=$!
poll=0
online="?"
while [ $poll -lt 60 ]; do
  st=$(curl -sS -m 20 -o "$work/st.json" -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $CONNECT_TOKEN") || break
  online=$(jq -r .online "$work/st.json" 2>/dev/null || echo "?")
  [ "$online" = "true" ] && break
  sleep 2
  poll=$((poll + 1))
done
if [ "${online:-?}" != "true" ]; then
  say "clause 4          serve never showed online (status ${st:-?})"
  fail=1
else
  say "clause 4          serve registered (status online)"
  CONNECT_PROXY="$PODBOX remote ssh connect --relay $WSS --name $NAME --connect-token $CONNECT_TOKEN"
  # One session carries both proofs: the marker out and the status back.
  # The marker is computed (the expanded MARK2 never occurs in the typed
  # bytes), and the exit code rides the same session, so one dial proves
  # the command ran and its code passed through.
  MARK2="relay-marker-$((RANDOM * RANDOM))"
  # shellcheck disable=SC2086
  out2=$(timeout 90 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" \
    -o ProxyCommand="$CONNECT_PROXY" localhost "echo $MARK2; exit 42" 2>"$work/conn.err")
  ccode=$?
  if [ "$ccode" -eq 42 ] && [ "$out2" = "$MARK2" ]; then
    say "clause 4          connect runs the far command and passes exit 42 through"
  else
    say "clause 4          connect FAILED (exit $ccode, out: $out2, err: $(head -3 "$work/conn.err" 2>/dev/null | tr '\n' '|'))"
    fail=1
  fi
fi
stop=$(curl -sS -m 20 -o /dev/null -w '%{http_code}' -X POST "$RELAY/v1/stop/$NAME" -H "X-Relay-Token: $STOP_TOKEN") || stop="?"
after=$(curl -sS -m 20 -o /dev/null -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $CONNECT_TOKEN") || after="?"
if [ "$stop" = "200" ] && [ "$after" = "403" ]; then
  say "clause 4          stop 200 with status 403 after"
else
  say "clause 4          stop/after FAILED: stop $stop, after $after"
  fail=1
fi
if [ "$fail" -ne 0 ]; then
  # Failure diagnostics, redacted: the serve log never carries tokens
  # (the mux legs keep them in headers), but it does carry the pair
  # name, which stays out of the committed log like the tokens do.
  say "diagnostics     serve log error lines (name redacted):"
  grep -aiE "error|panic|reject|unavailable|closed" "$work/serve.log" 2>/dev/null \
    | sed "s/$NAME/NAME/g" | head -20 >>"$OUT" || true
  say "diagnostics     operator stderrs:"
  head -5 "$work/conn.err" 2>/dev/null >>"$OUT" || true
fi
kill "$SERVE_PID" 2>/dev/null
wait "$SERVE_PID" 2>/dev/null
SERVE_PID=""

# Clause 5: the parity table carries the new rows, and no token reached
# the committed log.
if "$PODBOX" system info --format '{{json .Parity}}' 2>/dev/null | grep -q '"remote ssh"'; then
  say "clause 5          parity carries the remote ssh rows"
else
  say "clause 5          parity misses the remote ssh rows"
  fail=1
fi
if grep -qF "$NODE_TOKEN" "$OUT" 2>/dev/null || grep -qF "$CONNECT_TOKEN" "$OUT" 2>/dev/null \
    || grep -qF "$STOP_TOKEN" "$OUT" 2>/dev/null || grep -qF "$NAME" "$OUT" 2>/dev/null; then
  say "clause 5          a token or the pair name reached the committed log"
  fail=1
else
  say "clause 5          no token and no pair name in the log"
fi

if [ "$fail" -eq 0 ]; then
  say "verdict           REMOTE-OK: serve and connect drive real commands with exit codes"
else
  say "verdict           REMOTE-FAIL"
fi
cp "$OUT" /out/ 2>/dev/null || true
exit "$fail"
