#!/usr/bin/env bash
# Question: does `podssh` carry a real ssh session over every transport it
# claims, driven by a real `ssh` client, with the agent opening no listening
# socket at all?
#
# TODO/podssh.md T-1401. The answer this proves is the whole product claim, so
# it is a script and not a unit test: every case needs a relay process, a
# server process and a client, and the defect this entry closes was found only
# when a real `ssh` drove `podssh connect` as its `ProxyCommand`. The
# byte-pipe case passed at the same time the ssh case hung, so a unit test
# would have reported the transport working while the product did not.
#
# ⛔ Every case is over a unix socket. A build cage may forbid binding a TCP
# port, and the socket under the transport is not what is under test.
#
# The committed reading is experiments/results/podssh-e2e.txt.
#
#   ./369-podssh-e2e.sh                 build podssh and run the matrix
#   PODSSH=/path/to/podssh ./369-...    run a binary already built
#
# Exit: 0 every case passed, 1 a case failed, 2 could not run.

set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)

if [ -z "${PODSSH:-}" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "369-podssh-e2e: pass PODSSH=/path/to/podssh, or have cargo" >&2
        exit 2
    fi
    ( cd "$ROOT" && cargo build -p podbox-ssh ) || exit 2
    PODSSH="$ROOT/target/debug/podssh"
fi
[ -x "$PODSSH" ] || { echo "369-podssh-e2e: $PODSSH is not executable" >&2; exit 2; }

exec sh "$ROOT/crates/podbox-ssh/tests/e2e.sh"
