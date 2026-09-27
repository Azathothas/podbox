#!/usr/bin/env bash
# podssh's end-to-end proof. Run it from anywhere:
#
#   ./crates/podbox-ssh/tests/e2e.sh                 # builds podssh, runs everything runnable
#   PODSSH=/path/to/podssh ./crates/podbox-ssh/tests/e2e.sh
#
# It is a shell script and not a `#[test]` on purpose. Every case needs a
# listener, a relay process, a server process and a client, and the interesting
# failure of 2026-09-26 -- a pump that blocked on standard input before it ever
# read the relay -- hung a real `ssh` under a real `ProxyCommand` and passed a
# pipe. A test that talks to the real clients is the only one that would have
# caught it.
#
# ⛔ Every case is over a unix socket, because a build host may forbid binding a
# TCP port. The transport that matters is the one between the two podssh
# processes; the socket underneath it is not what is under test.
#
# Exit: 0 every runnable case passed, 1 a case failed, 2 could not run.

set -u

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/podssh-e2e.XXXXXX")
BIN="${PODSSH:-}"
PASS=0
FAIL=0
SKIP=0
TIMEOUT=${PODSSH_E2E_TIMEOUT:-30}

cleanup() {
    [ -n "${RELAY_PID:-}" ] && kill "$RELAY_PID" 2>/dev/null
    [ -n "${SERVE_PID:-}" ] && kill "$SERVE_PID" 2>/dev/null
    rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

ok()   { PASS=$((PASS + 1)); printf 'ok       %s\n' "$1"; }
bad()  { FAIL=$((FAIL + 1)); printf 'FAILED   %s: %s\n' "$1" "$2"; }
skip() { SKIP=$((SKIP + 1)); printf 'skip     %s: %s\n' "$1" "$2"; }

need() { command -v "$1" >/dev/null 2>&1; }

if [ -z "$BIN" ]; then
    if ! need cargo; then
        echo "podssh-e2e: no \$PODSSH and no cargo; pass PODSSH=/path/to/podssh" >&2
        exit 2
    fi
    ( cd "$ROOT" && cargo build -p podbox-ssh ) || exit 2
    BIN="$ROOT/target/debug/podssh"
fi
if [ ! -x "$BIN" ]; then
    echo "podssh-e2e: $BIN is not executable" >&2
    exit 2
fi
BIN=$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")

echo "podssh-e2e: binary $BIN"
echo "podssh-e2e: workdir $WORK"
"$BIN" version || true

start_relay() {
    local name=$1 extra=$2
    "$BIN" relay --listen "unix://$WORK/$name.sock" --key SECRET $extra \
        >"$WORK/$name.relay.log" 2>&1 &
    RELAY_PID=$!
    local i=0
    while [ ! -S "$WORK/$name.sock" ]; do
        i=$((i + 1))
        if [ "$i" -gt 100 ]; then
            bad "$name" "relay never created its socket"
            return 1
        fi
        sleep 0.05
    done
}

stop() {
    [ -n "${RELAY_PID:-}" ] && kill "$RELAY_PID" 2>/dev/null
    [ -n "${SERVE_PID:-}" ] && kill "$SERVE_PID" 2>/dev/null
    wait 2>/dev/null
    RELAY_PID=
    SERVE_PID=
}

# A pure byte pipe: `cat` echoes whatever the client writes, so the case proves
# the transport carried bytes both ways and that the half-close ended both
# processes rather than hanging them.
case_pipe() {
    local name=$1 extra=$2 url=$3 ca=${4:-}
    local ca_arg=
    [ -n "$ca" ] && ca_arg="--tls-ca $ca"
    if ! start_relay "$name" "$extra"; then return; fi
    "$BIN" serve --relay "$url" --name agent1 --auth SECRET $ca_arg \
        --server cat --once >"$WORK/$name.serve.log" 2>&1 &
    SERVE_PID=$!
    sleep 0.2
    # ⛔ The trailing `sleep` is the test and not padding. Closing the client's
    # standard input is what tells the relay the session is over, so a bare
    # `printf` races the echo back and reads as a transport that dropped bytes.
    # A real ProxyCommand's stdin stays open for the session's whole lifetime,
    # which is what this holds open; the first version of this case failed on
    # ws, tls and wss and had nothing to do with any of the three.
    local out
    out=$( { printf 'podssh-%s-payload' "$name"; sleep 1; } | timeout "$TIMEOUT" \
        "$BIN" connect --relay "$url" --name agent1 --auth SECRET $ca_arg 2>"$WORK/$name.err") \
        && [ "$out" = "podssh-$name-payload" ] \
        && ok "$name" \
        || bad "$name" "out='$out' $(tail -1 "$WORK/$name.err" 2>/dev/null)"
    stop
}

# The case the 2026-09-26 defect was found in: a real `ssh` process driving
# `podssh connect` as its ProxyCommand into a real `sshd -i`. The pipe case
# above passed while this one hung, so this is the one that must be here.
case_ssh() {
    local name=$1 extra=$2 url=$3 ca=${4:-}
    local ca_arg=
    [ -n "$ca" ] && ca_arg="--tls-ca $ca"
    if ! need ssh || ! need sshd; then
        skip "ssh/$name" "no ssh or sshd on PATH"
        return
    fi
    if ! start_relay "$name" "$extra"; then return; fi
    "$BIN" serve --relay "$url" --name agent1 --auth SECRET $ca_arg \
        --server "sshd -i -e -f $WORK/sshd_config" --once \
        >"$WORK/$name.serve.log" 2>&1 &
    SERVE_PID=$!
    sleep 0.3
    local out
    out=$(timeout "$TIMEOUT" ssh \
        -o "ProxyCommand=$BIN connect --relay $url --name agent1 --auth SECRET $ca_arg" \
        -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -o BatchMode=yes -o IdentitiesOnly=yes -i "$WORK/user_ed25519" \
        agent@agent1 "echo ssh-$name-ok" 2>"$WORK/$name.err") \
        && [ "$out" = "ssh-$name-ok" ] \
        && ok "ssh/$name" \
        || bad "ssh/$name" "out='$out' rc=$? $(tail -1 "$WORK/$name.err" 2>/dev/null)"
    stop
}

# --- fixtures ---------------------------------------------------------------

# A host key and a user key for the sshd cases. An empty passphrase because
# `-o BatchMode=yes` must not reach a prompt. A valid `getpwnam` for the current
# user is the one thing this cannot conjure: a normal host has /etc/passwd, and
# a build cage without one sets SANDSSH_PASSWD/LD_PRELOAD the way this project's
# own lanes already do.
ssh-keygen -q -t ed25519 -N '' -f "$WORK/host_ed25519" >/dev/null 2>&1
ssh-keygen -q -t ed25519 -N '' -f "$WORK/user_ed25519" >/dev/null 2>&1
cp "$WORK/user_ed25519.pub" "$WORK/authorized_keys"
cat >"$WORK/sshd_config" <<EOF
HostKey $WORK/host_ed25519
PidFile none
AuthorizedKeysFile $WORK/authorized_keys
PasswordAuthentication no
UsePAM no
StrictModes no
LogLevel ERROR
EOF

# A CA and a leaf, so the TLS cases exercise verification rather than
# --insecure. A self-signed leaf is rejected by rustls as CaUsedAsEndEntity,
# which is correct and was how this was first got wrong.
HAVE_TLS=0
if need openssl; then
    openssl req -x509 -newkey ed25519 -keyout "$WORK/ca.key" -out "$WORK/ca.crt" \
        -days 2 -nodes -subj "/CN=podssh-e2e-ca" \
        -addext "basicConstraints=critical,CA:TRUE" >/dev/null 2>&1
    openssl req -newkey ed25519 -keyout "$WORK/leaf.key" -out "$WORK/leaf.csr" \
        -nodes -subj "/CN=localhost" >/dev/null 2>&1
    printf 'subjectAltName=DNS:localhost\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth\n' \
        >"$WORK/leaf.ext"
    openssl x509 -req -in "$WORK/leaf.csr" -CA "$WORK/ca.crt" -CAkey "$WORK/ca.key" \
        -CAcreateserial -out "$WORK/leaf.crt" -days 2 -extfile "$WORK/leaf.ext" >/dev/null 2>&1
    [ -s "$WORK/leaf.crt" ] && HAVE_TLS=1
fi

# --- the matrix -------------------------------------------------------------

case_pipe raw   ""                          "unix://$WORK/raw.sock"
case_ssh  raw   ""                          "unix://$WORK/raw.sock"

case_pipe ws    "--ws"                      "ws+unix://$WORK/ws.sock"
case_ssh  ws    "--ws"                      "ws+unix://$WORK/ws.sock"

if [ "$HAVE_TLS" = 1 ]; then
    case_pipe tls  "--tls-cert $WORK/leaf.crt --tls-key $WORK/leaf.key" \
        "tls+unix://$WORK/tls.sock" "$WORK/ca.crt"
    case_ssh  tls  "--tls-cert $WORK/leaf.crt --tls-key $WORK/leaf.key" \
        "tls+unix://$WORK/tls.sock" "$WORK/ca.crt"

    case_pipe wss  "--ws --tls-cert $WORK/leaf.crt --tls-key $WORK/leaf.key" \
        "wss+unix://$WORK/wss.sock" "$WORK/ca.crt"
    case_ssh  wss  "--ws --tls-cert $WORK/leaf.crt --tls-key $WORK/leaf.key" \
        "wss+unix://$WORK/wss.sock" "$WORK/ca.crt"
else
    skip "tls" "no openssl to make a CA and a leaf"
    skip "wss" "no openssl to make a CA and a leaf"
fi

# `podbox ssh` is the same code behind the podbox verb, so it must answer the
# same way the standalone binary does. `podbox` is a second package in the same
# workspace, so it is found beside the binary under test when both are built;
# a missing one is a skip and never a failure.
PODBOX=""
for cand in "$(dirname "$BIN")/podbox" \
            "$ROOT/target/debug/podbox" \
            "$ROOT/target/x86_64-unknown-linux-gnu/debug/podbox"; do
    [ -x "$cand" ] && PODBOX=$cand && break
done
if [ -n "$PODBOX" ]; then
    if "$PODBOX" ssh version >/dev/null 2>&1; then
        ok "podbox-ssh-alias"
    else
        bad "podbox-ssh-alias" "$PODBOX ssh version exited non-zero"
    fi
else
    skip "podbox-ssh-alias" "podbox not built"
fi

# ⛔ The interop cases. `sandssh` is the tree podssh's transport comes from, and
# a protocol that only talks to itself is a fork of it, not a replacement. The
# sandssh tree is not in this repository, so a missing one is a skip and not a
# failure; where it is present, all three directions must pass. Each direction
# is a different claim: our relay is not ours alone, our client is not ours
# alone, and our server is not ours alone.
SANDSSH_TREE=${SANDSSH_TREE:-$ROOT/../sandssh}
if [ -f "$SANDSSH_TREE/bin/sandssh" ] && [ -f "$SANDSSH_TREE/relay/sandssh-relay.py" ]; then
    SS="$SANDSSH_TREE/bin/sandssh"

    # A: sandssh's own unmodified relay, podssh on both ends. The relay is the
    # foreign half here, and it is the half that decides whether the wire form
    # is really shared.
    python3 "$SANDSSH_TREE/relay/sandssh-relay.py" \
        --listen "$WORK/ssrelay.sock" --key SECRET >"$WORK/ssrelay.log" 2>&1 &
    RELAY_PID=$!
    i=0
    while [ ! -S "$WORK/ssrelay.sock" ]; do
        i=$((i + 1))
        [ "$i" -gt 100 ] && break
        sleep 0.05
    done
    "$BIN" serve --relay "unix://$WORK/ssrelay.sock" --name agent1 --auth SECRET \
        --protocol sandssh1 --server "sshd -i -e -f $WORK/sshd_config" --once \
        >"$WORK/interopA.serve.log" 2>&1 &
    SERVE_PID=$!
    sleep 0.8
    out=$(timeout "$TIMEOUT" ssh \
        -o "ProxyCommand=$BIN connect --relay unix://$WORK/ssrelay.sock --name agent1 --auth SECRET --protocol sandssh1" \
        -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -o BatchMode=yes -o IdentitiesOnly=yes -i "$WORK/user_ed25519" \
        agent@agent1 'echo interop-sandssh-relay-ok' 2>"$WORK/interopA.err")
    if [ "$out" = "interop-sandssh-relay-ok" ]; then
        ok "interop/sandssh-relay"
    else
        bad "interop/sandssh-relay" "out='$out' $(tail -1 "$WORK/interopA.err" 2>/dev/null)"
    fi
    stop

    # B: podssh's relay with sandssh's client. The client is the foreign half.
    if ! start_relay "interopB" ""; then :; else
        "$BIN" serve --relay "unix://$WORK/interopB.sock" --name agent1 --auth SECRET \
            --protocol sandssh1 --server "sshd -i -e -f $WORK/sshd_config" --once \
            >"$WORK/interopB.serve.log" 2>&1 &
        SERVE_PID=$!
        sleep 0.8
        out=$(timeout "$TIMEOUT" ssh \
            -o "ProxyCommand=python3 $SS connect --relay unix://$WORK/interopB.sock --name agent1 --auth SECRET" \
            -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
            -o BatchMode=yes -o IdentitiesOnly=yes -i "$WORK/user_ed25519" \
            agent@agent1 'echo interop-sandssh-client-ok' 2>"$WORK/interopB.err")
        if [ "$out" = "interop-sandssh-client-ok" ]; then
            ok "interop/sandssh-client"
        else
            bad "interop/sandssh-client" "out='$out' $(tail -1 "$WORK/interopB.err" 2>/dev/null)"
        fi
        stop
    fi

    # C: sandssh's server under podssh's relay and client.
    printf '#!/bin/sh\nexec sshd -i -e -f %s/sshd_config\n' "$WORK" >"$WORK/sshd-wrapper.sh"
    chmod +x "$WORK/sshd-wrapper.sh"
    if ! start_relay "interopC" ""; then :; else
        timeout "$TIMEOUT" python3 "$SS" serve --relay "unix://$WORK/interopC.sock" \
            --name agent1 --auth SECRET --command "$WORK/sshd-wrapper.sh" \
            >"$WORK/interopC.serve.log" 2>&1 &
        SERVE_PID=$!
        sleep 1.5
        out=$(timeout "$TIMEOUT" ssh \
            -o "ProxyCommand=$BIN connect --relay unix://$WORK/interopC.sock --name agent1 --auth SECRET" \
            -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
            -o BatchMode=yes -o IdentitiesOnly=yes -i "$WORK/user_ed25519" \
            agent@agent1 'echo interop-sandssh-serve-ok' 2>"$WORK/interopC.err")
        if [ "$out" = "interop-sandssh-serve-ok" ]; then
            ok "interop/sandssh-serve"
        else
            bad "interop/sandssh-serve" "out='$out' $(tail -1 "$WORK/interopC.err" 2>/dev/null)"
        fi
        stop
    fi
else
    skip "interop" "no sandssh tree beside this repository (set SANDSSH_TREE)"
fi

printf '\npodssh-e2e: %d passed, %d failed, %d skipped\n' "$PASS" "$FAIL" "$SKIP"
[ "$FAIL" -eq 0 ] || exit 1
exit 0
