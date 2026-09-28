#!/bin/bash
# Question: does the SSH partial hold in the lane: the podbox-ssh suite
# (unit plus real-client e2e) green, and the machine group driving its
# usage, refusal, and manual section through the built binary?
#
# TODO/podssh.md T-1401 (partial, no relay, no remote group) and T-1404
# (machine arm). The relay and remote group stay deferred: nothing here
# dials one. Exit 0 every clause held, 1 a clause disagreed, 2 the lane
# could not run.
set -u

cd /work || exit 2
OUT="experiments/results/podssh-partial.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT HUP INT TERM
{
  echo "== conditions"
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  git rev-parse --short HEAD
  cargo --version
  rustc --version
} >"$OUT"

fail=0

# Clause 1: the SSH binaries the e2e drives exist, installed here when
# they do not. Install output stays in the work log: the results file
# carries prose-safe lines only.
bash scripts/common/bootstrap-env.sh zig openssh >"$work/bootstrap.log" 2>&1 || exit 2
command -v sshd >>"$OUT" 2>&1 || exit 2
command -v ssh >>"$OUT" 2>&1 || exit 2
command -v ssh-keygen >>"$OUT" 2>&1 || exit 2
echo "clause 1          sshd, ssh, ssh-keygen on PATH" >>"$OUT"

# Clause 2: the crate suite, unit plus real-client e2e.
if timeout 20m cargo test -p podbox-ssh >>"$OUT" 2>&1; then
  echo "clause 2          cargo test -p podbox-ssh exit 0" >>"$OUT"
else
  echo "clause 2          cargo test -p podbox-ssh FAILED" >>"$OUT"
  fail=1
fi

# Clause 3: the CLI builds, so the group below drives the shipped shape.
# (The zig toolchain arrived with clause 1.)
if timeout 25m cargo build -p podbox-cli >>"$OUT" 2>&1; then
  echo "clause 3          cargo build -p podbox-cli exit 0" >>"$OUT"
else
  echo "clause 3          cargo build -p podbox-cli FAILED" >>"$OUT"
  fail=1
fi

# Clause 4: the machine surface through the built binary.
BIN=$(ls -d target/*/debug/podbox 2>/dev/null | head -n 1)
if [ -z "${BIN:-}" ] || [ ! -x "$BIN" ]; then
  echo "clause 4          no built podbox binary" >>"$OUT"
  fail=1
else
  echo "binary            $BIN" >>"$OUT"
  check() {
    # check <name> <want-code> <want-text> <args...>: run and judge.
    name="$1"; want="$2"; text="$3"; shift 3
    got="$("$BIN" "$@" 2>&1)"; code=$?
    if [ "$code" -eq "$want" ] && printf '%s' "$got" | grep -qF "$text"; then
      echo "ok                $name" >>"$OUT"
    else
      echo "FAIL              $name: exit $code, output: $got" >>"$OUT"
      fail=1
    fi
  }
  check "machine usage" 0 "usage: podbox machine" machine
  check "machine help" 0 "usage: podbox machine" machine --help
  check "machine ssh refusal" 125 "no machine guest carries an SSH server" machine ssh
  check "machine ssh guest refusal" 125 "guest: no machine guest" machine ssh guest
  check "machine ssh help" 0 "usage: podbox machine ssh" machine ssh --help
  check "machine unknown member" 125 "not implemented yet" machine relay
  check "machine bogus flag" 125 "unknown option" machine --bogus
  check "man machine" 0 "usage: podbox machine" man machine --no-pager
fi

{
  echo ""
  if [ "$fail" -eq 0 ]; then echo "verdict           PODSSH PARTIAL HOLDS"; else echo "verdict           PODSSH PARTIAL OPEN"; fi
} >>"$OUT"
cp "$OUT" /out/ 2>/dev/null || true
cp Cargo.lock /out/Cargo.lock 2>/dev/null || true
cat "$OUT"
[ "$fail" -eq 0 ]
