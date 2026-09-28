#!/bin/bash
# Question: does an interactive SSH session work with no pty anywhere?
#
# TODO/podssh.md T-1402 (the interactive proof). A real `sshd` runs as a
# daemon on loopback with `ForceCommand` set to the lane-built `shell`
# binary; a real `ssh` with piped descriptors drives one session through
# echo, editing, history, state, a signal, and the exit code, refuses an
# exec request, and records what a subsystem request reaches. The session
# asserts its own descriptors are not a terminal.
#
# Needle rule: the discipline echoes every typed byte, so a needle
# present in the input echo proves nothing. Every output needle below is
# a computed marker whose expanded form never occurs in the typed bytes.
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="experiments/results/interactive-shell.txt"
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

bash scripts/common/bootstrap-env.sh zig openssh tools >"$work/bootstrap.log" 2>&1 || exit 2
mkdir -p /run/sshd || exit 2
command -v ssh >>"$OUT" 2>&1 || exit 2
command -v sshd >>"$OUT" 2>&1 || exit 2
command -v ssh-keygen >>"$OUT" 2>&1 || exit 2

# Clause 1: the lane-built shell binary exists.
if timeout 25m cargo build -p podbox-ssh --bin shell >>"$OUT" 2>&1; then
  say "clause 1          cargo build -p podbox-ssh --bin shell exit 0"
else
  say "clause 1          cargo build FAILED"
  exit 2
fi
SHELL_BIN=$(ls -d target/*/debug/shell 2>/dev/null | head -n 1)
if [ -z "${SHELL_BIN:-}" ]; then
  say "clause 1          shell binary missing"
  exit 2
fi
SHELL_BIN_ABS="/work/$SHELL_BIN"
SSHD_BIN=$(command -v sshd) || exit 2

# Clause 2: a key-only sshd daemon on loopback with the session server
# forced. The config mirrors what `write_sshd_config` generates.
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
"$SSHD_BIN" -f "$work/sshd_config" -p 2222 \
  -o "ForceCommand=$SHELL_BIN_ABS --shell sh" -E "$work/sshd.log" || exit 2
SSHD_PID=$(cat "$work/sshd.pid" 2>/dev/null || true)
poll=0
while [ $poll -lt 30 ]; do
  # No command: an interactive session that ends at once on empty stdin.
  # A one-shot command would exercise the refusal path, not readiness.
  if printf '' | timeout 10 ssh -o BatchMode=yes -o StrictHostKeyChecking=no \
      -o UserKnownHostsFile=/dev/null -o ConnectTimeout=5 -o LogLevel=ERROR \
      -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost 2>/dev/null; then
    break
  fi
  sleep 1
  poll=$((poll + 1))
done
if [ "$poll" -ge 30 ]; then
  say "clause 2          sshd daemon never answered"
  exit 2
fi
say "clause 2          sshd daemon answers on 2222 with ForceCommand shell"

ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o LogLevel=ERROR"

# Clause 3: prompt, echo, and the premise. Descriptors inside the
# session are pipes, not a terminal. The shell-ran marker is computed:
# only expansion produces `out=hello-session`.
printf 'test -t 0; echo stdin_tty=$?\ntest -t 1; echo stdout_tty=$?\necho hello-session\necho out=$(echo hello-session)\n' >"$work/in3.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in3.txt" >"$work/out3.txt" 2>"$work/err3.txt"
c3=$?
if [ "$c3" -eq 0 ] && [ "$(head -c 2 "$work/out3.txt")" = "\$ " ] \
    && grep -qF 'stdin_tty=1' "$work/out3.txt" \
    && grep -qF 'stdout_tty=1' "$work/out3.txt" \
    && grep -qF 'out=hello-session' "$work/out3.txt"; then
  say "clause 3          prompt, echo, and no terminal on either descriptor"
else
  say "clause 3          FAILED (exit $c3)"
  fail=1
fi

# Clause 4: editing repairs a typo. The repaired marker QZY7 reaches the
# shell; the as-typed QZX7X never runs as a command.
printf 'echo QZX7X\177\177\177Y7\n' >"$work/in4.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in4.txt" >"$work/out4.txt" 2>/dev/null
if grep -qF 'QZY7' "$work/out4.txt"; then
  say "clause 4          editing repaired the typo (shell ran QZY7)"
else
  say "clause 4          FAILED"
  fail=1
fi

# Clause 5: history recalls. Up brings the line back; Enter runs it again.
# The escape is octal: POSIX printf owns no \x form. The marker is
# computed: the expanded `recall-2` never occurs in the typed bytes, so
# two sightings are two shell runs.
printf 'echo recall-$((1+1))\n\033[A\n' >"$work/in5.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in5.txt" >"$work/out5.txt" 2>/dev/null
if [ "$(grep -cF 'recall-2' "$work/out5.txt")" -ge 2 ]; then
  say "clause 5          history recalled and re-ran the command"
else
  say "clause 5          FAILED"
  fail=1
fi

# Clause 6: state persists. A variable and a directory survive across lines.
# Both markers are computed: the typed bytes hold `$podbox_mark` and
# `$(pwd)`, never the expanded `got-42` or `dir=/tmp`.
printf 'podbox_mark=42\necho got-$podbox_mark\ncd /tmp\necho dir=$(pwd)\n' >"$work/in6.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in6.txt" >"$work/out6.txt" 2>/dev/null
if grep -qF 'got-42' "$work/out6.txt" && grep -qF 'dir=/tmp' "$work/out6.txt"; then
  say "clause 6          variable and directory persisted across commands"
else
  say "clause 6          FAILED"
  fail=1
fi

# Clause 7: the signal kills the command, not the shell. The shell traps
# INT with a handler, so it survives; the foreground sleep is reset to
# the default in the child, so the group SIGINT kills the command and
# nothing else. The handler (not an ignore) is load-bearing: ignored
# dispositions are inherited across fork and exec, so under `trap '' INT`
# the sleep would ignore the signal too and run its full course, and no
# inner `trap - INT` undoes an ignore inherited on entry (both measured
# 2026-09-28: 30 s sleeps survive either way; the untrapped session ends
# 130). Input flows through a FIFO, staggered on the shell's own output:
# the `started` marker (carriage-pinned, so the input echo cannot
# satisfy it) proves the shell reached the sleep line, and a five-second
# settle lets the sleep exec before Ctrl-C lands on the group. The alive
# marker is computed, so only the shell running the line produces it;
# the 25 s bound against the 60 s sleep convicts a surviving sleep.
mkfifo "$work/in7.fifo" || exit 2
cr=$(printf '\r')
# shellcheck disable=SC2086
timeout 120 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in7.fifo" >"$work/out7.txt" 2>/dev/null &
c7pid=$!
exec 9>"$work/in7.fifo"
start=$(date +%s)
printf '%s\n' "trap 'echo outer-trapped' INT" >&9
printf '%s\n' 'echo started; sleep 60' >&9
poll=0
while [ $poll -lt 30 ] && ! grep -qF "started$cr" "$work/out7.txt" 2>/dev/null; do
  sleep 1
  poll=$((poll + 1))
done
# Five seconds, matching the integration test's proven settle: the marker
# proves the shell wrote `started`, and only the exec of `sleep` after it
# puts a victim in the group for Ctrl-C.
sleep 5
printf '\003' >&9
printf '%s\n' 'echo alive-$((40+2))' >&9
exec 9>&-
wait "$c7pid"
c7=$?
elapsed=$(( $(date +%s) - start ))
if grep -qF "outer-trapped$cr" "$work/out7.txt" \
    && grep -qF "alive-42$cr" "$work/out7.txt" && [ "$elapsed" -lt 30 ]; then
  say "clause 7          SIGINT killed sleep in ${elapsed}s, shell survived"
else
  say "clause 7          FAILED (exit $c7, ${elapsed}s)"
  fail=1
fi

# Clause 8: the shell's exit code passes through.
printf 'exit 7\n' >"$work/in8.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in8.txt" >/dev/null 2>/dev/null
if [ "$?" -eq 7 ]; then
  say "clause 8          exit 7 passed through"
else
  say "clause 8          FAILED"
  fail=1
fi

# Clause 9: a one-shot command is refused naming the variable, and the
# refused command prints nothing.
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  "echo hi" >"$work/out9.txt" 2>"$work/err9.txt"
c9=$?
if [ "$c9" -eq 125 ] && grep -qF 'SSH_ORIGINAL_COMMAND' "$work/err9.txt" \
    && [ ! -s "$work/out9.txt" ]; then
  say "clause 9          exec refused with SSH_ORIGINAL_COMMAND named, stdout empty"
else
  say "clause 9          FAILED (exit $c9)"
  fail=1
fi

# Clause 10 (record only): what does a subsystem request reach? Measured
# 2026-09-28: it never reaches the shell. The generated configuration
# defines no subsystems, so `sshd` itself refuses `ssh -s sftp` with
# "subsystem request failed" and a nonzero exit. The refusal catalogue
# records this split: exec-form requests are refused by the shell naming
# SSH_ORIGINAL_COMMAND, subsystem requests by the daemon.
printf 'echo subsystem-probe-marker\n' >"$work/in10.txt"
# shellcheck disable=SC2086
timeout 20 ssh $ssh_opts -s -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  sftp <"$work/in10.txt" >"$work/out10.txt" 2>"$work/err10.txt"
c10=$?
say "clause 10         subsystem sftp exit $c10, stdout bytes $(wc -c <"$work/out10.txt") (record only)"
{
  echo "--- out10"
  cat "$work/out10.txt"
  echo "--- err10"
  cat "$work/err10.txt"
} >>"$OUT"

# Clause 11: without any trap, the group SIGINT kills the shell and its
# foreground sleep together, and the session reports 128+2. Delivery to
# the group is what this proves; selectivity is clause 7. The Ctrl-C byte
# may land before or after the sleep execs; both victims hold the default
# disposition, so either death reports 130 either way.
printf 'sleep 20\n\003' >"$work/in11.txt"
# shellcheck disable=SC2086
timeout 60 ssh $ssh_opts -i "$work/user_key" -l "$USER_NAME" -p 2222 localhost \
  <"$work/in11.txt" >"$work/out11.txt" 2>/dev/null
c11=$?
if [ "$c11" -eq 130 ]; then
  say "clause 11         untrapped SIGINT ended the session with 130"
else
  say "clause 11         FAILED (exit $c11)"
  fail=1
fi

kill "$SSHD_PID" 2>/dev/null || true
wait 2>/dev/null || true
{
  echo ""
  if [ "$fail" -eq 0 ]; then echo "verdict           INTERACTIVE SESSION WITHOUT PTY HOLDS"; else echo "verdict           INTERACTIVE PROOF OPEN"; fi
} >>"$OUT"
# Every clause output travels with the record (no keys: $work keeps those).
cp "$OUT" /out/ 2>/dev/null || true
for f in "$work"/out*.txt "$work"/err*.txt; do
  cp "$f" /out/ 2>/dev/null || true
done
cat "$OUT"
[ "$fail" -eq 0 ]
