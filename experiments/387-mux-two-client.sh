#!/bin/bash
# Question: do concurrent authenticated SSH sessions share one podbox-ssh
# node connection against the live multiplexed relay?
#
# TODO/podssh.md T-1403 (the two-client proof) and T-1401 (the relay
# landing over the proved transport). The relay is the live
# tcp.ssh.relay.ajam.dev at the version /health reports; the node and all
# operators are the lane-built podbox-ssh binaries; every SSH client is a
# real `ssh` against a real `sshd -i` the node spawns per session. Three
# sessions run, two concurrent at a time: A spans the whole drive while B
# moves bytes and a third session passes exit 42.
#
# ⛔ Tokens travel in environment, in a work-dir file, and in local argv
# (the node flag and the ssh ProxyCommand line, visible in the lane's own
# process list and nowhere else). The results file carries statuses,
# counts, and hashes, never a token: the script greps for each token
# before it verdicts, and a match fails the run.
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="experiments/results/mux-two-client.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT HUP INT TERM
RELAY="https://tcp.ssh.relay.ajam.dev"
WSS="wss://tcp.ssh.relay.ajam.dev"
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

bash scripts/common/bootstrap-env.sh zig openssh tools >"$work/bootstrap.log" 2>&1 || exit 2
# Debian's sshd demands the privilege separation directory even in inetd
# mode, and the node probes sshd at startup. Same line as the crate e2e.
mkdir -p /run/sshd || exit 2
command -v curl >>"$OUT" 2>&1 || exit 2
command -v ssh >>"$OUT" 2>&1 || exit 2
command -v sshd >>"$OUT" 2>&1 || exit 2
command -v ssh-keygen >>"$OUT" 2>&1 || exit 2
command -v jq >>"$OUT" 2>&1 || exit 2
say "relay             $RELAY"
curl -sS -m 20 "$RELAY/health?detail=1" -o "$work/health.json" >>"$OUT" 2>&1 || exit 2
say "relay version     $(jq -r .version "$work/health.json" 2>/dev/null || echo unknown)"

# Clause 1: the lane-built node and operator exist.
if timeout 25m cargo build -p podbox-ssh --bins >>"$OUT" 2>&1; then
  say "clause 1          cargo build -p podbox-ssh --bins exit 0"
else
  say "clause 1          cargo build FAILED"
  exit 2
fi
NODE_BIN=$(ls -d target/*/debug/node 2>/dev/null | head -n 1)
OPERATOR_BIN=$(ls -d target/*/debug/operator 2>/dev/null | head -n 1)
if [ -z "${NODE_BIN:-}" ] || [ -z "${OPERATOR_BIN:-}" ]; then
  say "clause 1          node or operator binary missing"
  exit 2
fi

# Clause 2: mint an ephemeral pair. The response carries the tokens and is
# never copied to the results file.
http=$(curl -sS -m 25 -X POST "$RELAY/v1/pair" -H "content-type: application/json" -d '{}' \
  -o "$work/pair.json" -w '%{http_code}') || exit 2
if [ "$http" != "200" ]; then
  say "clause 2          pair mint answered $http, not 200"
  exit 2
fi
NAME=$(jq -r .name "$work/pair.json")
NODE_TOKEN=$(jq -r .node_token "$work/pair.json")
CONNECT_TOKEN=$(jq -r .connect_token "$work/pair.json")
STOP_TOKEN=$(jq -r .stop_token "$work/pair.json")
if [ -z "${NAME:-}" ] || [ -z "${NODE_TOKEN:-}" ] || [ -z "${CONNECT_TOKEN:-}" ] || [ -z "${STOP_TOKEN:-}" ]; then
  say "clause 2          pair response missed a field"
  exit 2
fi
say "clause 2          pair minted (name withheld from committed log: see node log counts)"

# Clause 3: the token roles hold. Status answers the connect token and
# refuses the node token; nothing here prints either token.
st=$(curl -sS -m 20 -o "$work/st1.json" -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $CONNECT_TOKEN") || exit 2
st_node=$(curl -sS -m 20 -o /dev/null -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $NODE_TOKEN") || exit 2
if [ "$st" = "200" ] && [ "$st_node" = "403" ]; then
  say "clause 3          status: connect-token $st, node-token $st_node"
else
  say "clause 3          status role split FAILED: connect $st, node $st_node"
  fail=1
fi

# Clause 4: one node connection registers. The server is detected, not
# injected: runtime and authorized keys below are the production default.
RT="$work/rt"
mkdir -p "$RT" && chmod 700 "$RT"
ssh-keygen -t ed25519 -N "" -q -f "$work/user_key" || exit 2
cp "$work/user_key.pub" "$work/authorized_keys"
export PODSSH_RUNTIME="$RT" PODSSH_AUTHORIZED_KEYS="$work/authorized_keys"
export PODSSH_RELAY="$WSS" PODSSH_NAME="$NAME" PODSSH_NODE_TOKEN="$NODE_TOKEN"
export PODSSH_CONNECT_TOKEN="$CONNECT_TOKEN"
"$NODE_BIN" --relay "$WSS" --name "$NAME" --node-token "$NODE_TOKEN" >"$work/node.log" 2>&1 &
NODE_PID=$!
poll=0
while [ $poll -lt 60 ]; do
  st=$(curl -sS -m 20 -o "$work/st2.json" -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $CONNECT_TOKEN") || break
  online=$(jq -r .online "$work/st2.json" 2>/dev/null || echo "?")
  [ "$online" = "true" ] && break
  sleep 2
  poll=$((poll + 1))
done
if [ "${online:-?}" = "true" ]; then
  say "clause 4          one node connection registered (status online)"
else
  say "clause 4          node never showed online (status $st)"
  fail=1
fi

# Clause 5: concurrent authenticated sessions. A runs a marker, sleeps
# across the rest, then runs a second marker: the second half proves the
# first session survived the others' close. B moves 200000 bytes both ways
# at once; a third session passes exit 42 through.
USER_NAME=$(id -un)
OPERATOR_PROXY="env PODSSH_RELAY=$WSS PODSSH_NAME=$NAME PODSSH_CONNECT_TOKEN=$CONNECT_TOKEN $OPERATOR_BIN"
ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o LogLevel=ERROR"
if [ "$fail" -eq 0 ]; then
  # shellcheck disable=SC2086
  ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -o ProxyCommand="$OPERATOR_PROXY" localhost \
    "printf live-A1; sleep 8; printf live-A2" >"$work/a.out" 2>"$work/a.err" &
  A_PID=$!
  sleep 2
  head -c 200000 /dev/urandom >"$work/big.bin" || exit 2
  # shellcheck disable=SC2086
  ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -o ProxyCommand="$OPERATOR_PROXY" localhost \
    "cat" <"$work/big.bin" >"$work/big.out" 2>"$work/b.err" &
  B_PID=$!
  wait "$B_PID"
  bcode=$?
  if [ "$bcode" -eq 0 ] && cmp -s "$work/big.bin" "$work/big.out"; then
    say "clause 5a         200000-byte round trip exact through session B"
  else
    say "clause 5a         200000-byte round trip FAILED (exit $bcode)"
    fail=1
  fi
  # shellcheck disable=SC2086
  ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -o ProxyCommand="$OPERATOR_PROXY" localhost \
    "exit 42" >/dev/null 2>"$work/b42.err"
  b42=$?
  if [ "$b42" -eq 42 ]; then
    say "clause 5b         exit 42 passed through"
  else
    say "clause 5b         exit code FAILED: got $b42"
    fail=1
  fi
  wait "$A_PID"
  acode=$?
  if [ "$acode" -eq 0 ] && [ "$(cat "$work/a.out")" = "live-A1live-A2" ]; then
    say "clause 5c         session A survived B's close with exact bytes"
  else
    say "clause 5c         session A FAILED (exit $acode, out: $(cat "$work/a.out" 2>/dev/null))"
    fail=1
  fi
  if [ -s "$work/a.err" ] || [ -s "$work/b.err" ] || [ -s "$work/b42.err" ]; then
    say "clause 5d         a client stderr not empty"
    fail=1
  else
    say "clause 5d         all clients silent on stderr"
  fi
fi

# Clause 6: stop the pair and prove it is gone. Status after stop is 403.
stopcode=$(curl -sS -m 25 -X POST "$RELAY/v1/stop/$NAME" -H "X-Relay-Token: $STOP_TOKEN" -o /dev/null -w '%{http_code}') || stopcode="curl-failed"
after=$(curl -sS -m 20 -o /dev/null -w '%{http_code}' "$RELAY/v1/status/$NAME" -H "X-Relay-Token: $CONNECT_TOKEN") || after="curl-failed"
if [ "$stopcode" = "200" ] && [ "$after" = "403" ]; then
  say "clause 6          stop 200, status after stop 403"
else
  say "clause 6          stop lifecycle FAILED: stop $stopcode, after $after"
  fail=1
fi
kill "$NODE_PID" 2>/dev/null || true
wait "$NODE_PID" 2>/dev/null || true
completed=$(grep -c "session closed" "$work/node.log" 2>/dev/null || true)
if [ "$fail" -eq 0 ]; then
  if [ "$completed" = "3" ]; then
    say "node sessions     3 closed, none left behind"
  else
    say "node sessions     $completed closed, expected 3"
    fail=1
  fi
else
  say "node sessions     $completed closed (earlier clause already failed)"
fi
grep -v "^$" "$work/node.log" | sed "s/$NAME/PAIR-NAME/g" >>"$OUT" 2>/dev/null || true

# Clause 7: no token in the committed log. The check runs before the
# verdict, and a match fails the run: the file must not be committed then.
for t in "$NODE_TOKEN" "$CONNECT_TOKEN" "$STOP_TOKEN"; do
  if grep -qF "$t" "$OUT"; then
    say "clause 7          TOKEN LEAK IN RESULTS LOG"
    fail=1
  fi
done
if [ "$fail" -eq 0 ]; then
  say "clause 7          no token in the committed log"
fi

{
  echo ""
  if [ "$fail" -eq 0 ]; then echo "verdict           TWO CLIENTS ON ONE NODE CONNECTION HOLD"; else echo "verdict           TWO-CLIENT PROOF OPEN"; fi
} >>"$OUT"
cp "$OUT" /out/ 2>/dev/null || true
cat "$OUT"
[ "$fail" -eq 0 ]
