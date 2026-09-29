#!/bin/bash
# Question: does the owned machine bridge build static, move bytes both
# ways, and exit with the server's status on every documented edge?
#
# TODO/podssh.md T-1404 (the machine arm). dropbear's inetd path needs a
# socket where the guest has a serial line, so the server stays pristine
# and experiments/lib/machine-bridge.c adapts: one socket pair, the
# server forked onto one end, the other spliced to the tty. This drive
# proves the bridge alone, host-side, with no guest in the loop: the
# static build with the lane zig, the usage refusal, bytes both ways and
# the server's status through a pty, the pinned server's banner across
# it, the failure edges (a tty that will not open, a server that
# will not exec, a server a signal takes), the raw mode the serial
# line needs (the device starts cooked and the bridge clears it: without
# that, output post-processing and canonical input mangle binary
# framing; VMIN and VTIME pinned beside the flags), and the half-close
# that releases the server where the tty dies instead of wedging it.
# The guest composition is
# experiments/390-machine-ssh.sh.
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="$PWD/experiments/results/machine-bridge.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
[ -d experiments/results ] || exit 2
: >"$OUT" || exit 2
work=$(mktemp -d) || exit 2
fail=0
trap 'echo "391: exiting with fail=$fail"; cp "$OUT" /out/ 2>/dev/null || true; rm -rf "$work"' EXIT HUP INT TERM
DBSRC="$work/dropbear-src"

DROPBEAR_COMMIT="59870ad43153fe8d4f1c96f5d5752116c94f31ff"
DROPBEAR_ORIGIN="https://github.com/mkj/dropbear"

say() { printf '%s\n' "$*" >>"$OUT"; }
miss() { fail=1; say "  FAIL: $1"; }
ok() { say "  ok: $1"; }
progress() { echo "391: $*" >&2; }

{
  echo "== conditions"
  printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel       %s\n' "$(uname -r)"
  printf 'dropbear          %s %s\n' "$DROPBEAR_ORIGIN" "$DROPBEAR_COMMIT"
  echo
} >"$OUT"

bash scripts/common/bootstrap-env.sh rust cc zig openssh tools qemu vmtools >"$work/bootstrap.log" 2>&1 || { echo "SKIP: bootstrap failed" >&2; exit 2; }
for t in python3 timeout file zig make cc git; do
  command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

say "== 1. the bridge builds static and refuses without args"
progress "building the bridge"
BRIDGE_SRC="$PWD/experiments/lib/machine-bridge.c"
[ -f "$BRIDGE_SRC" ] || { miss "the bridge source is not in the tree"; }
if [ "$fail" -eq 0 ]; then
  say "zig: $(zig version 2>/dev/null || echo MISSING)"
  if timeout 300 zig cc -target x86_64-linux-musl -static -O2 -Wall -Wextra -Werror \
      -o "$work/machine-bridge" "$BRIDGE_SRC" >>"$OUT" 2>&1; then
    ok "bridge built with zig $(zig version 2>/dev/null || echo unknown)"
  else
    miss "the bridge did not build"
  fi
fi
BRIDGE="$work/machine-bridge"
if [ "$fail" -eq 0 ]; then
  if [ -x "$BRIDGE" ] && file "$BRIDGE" | grep -q "statically linked"; then
    ok "bridge is statically linked: $(file "$BRIDGE" | cut -c1-120)"
  else
    miss "bridge is not a static binary"
  fi
fi
if timeout 10 "$BRIDGE" >"$work/usage.out" 2>&1; then
  miss "bridge with no args exited 0"
else
  ucode=$?
  if [ "$ucode" -eq 125 ] && grep -q "usage:" "$work/usage.out"; then
    ok "bridge with no args refuses with 125 and usage"
  else
    miss "bridge usage FAILED (exit $ucode)"
  fi
fi
if timeout 10 "$BRIDGE" onlyone >"$work/usage1.out" 2>&1; then
  miss "bridge with one arg exited 0"
else
  ucode1=$?
  if [ "$ucode1" -eq 125 ] && grep -q "usage:" "$work/usage1.out"; then
    ok "bridge with one arg refuses with 125 and usage"
  else
    miss "bridge one-arg usage FAILED (exit $ucode1)"
  fi
fi

say ""
say "== 2. bytes both ways and the server's exit code, through a pty"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 1"
else
progress "fake-server case starting"
timeout 60 python3 - "$BRIDGE" "$work/t2.out" "$work/t2.report" <<'PYEOF'
import os, pty, select, sys, time, tty
bridge, outp, reportp = sys.argv[1], sys.argv[2], sys.argv[3]
m, s = pty.openpty()
tty.setraw(s)
slavename = os.ttyname(s)
pid = os.fork()
if pid == 0:
    os.setsid()
    os.dup2(s, 0); os.dup2(s, 1)
    os.close(m); os.close(s)
    os.execv(bridge, [bridge, slavename, "/bin/sh", "-c", "echo bridge-42; exit 42"])
    os._exit(127)
os.close(s)
# The tty end is the pty slave itself: master bytes travel tty to
# socket, server bytes travel socket to tty, both observed here. Raw
# mode keeps the line discipline from echoing our bytes back at us.
os.write(m, b"ignored\n")
data = b""
t0 = time.monotonic()
while time.monotonic() - t0 < 10.0:
    r, _, _ = select.select([m], [], [], 1.0)
    if r:
        try:
            chunk = os.read(m, 65536)
        except OSError:
            break
        if not chunk:
            break
        data += chunk
open(outp, "wb").write(data)
_, status = os.waitpid(pid, os.WNOHANG)
if status == 0:
    try:
        os.kill(pid, 9)
    except OSError:
        pass
    _, status = os.waitpid(pid, 0)
code = os.waitstatus_to_exitcode(status)
open(reportp, "w").write(f"bridge-exit={code}\n")
PYEOF
t2code=$(sed -n 's/^bridge-exit=//p' "$work/t2.report" | head -1)
t2out=$(tr -d '\0\r\n' <"$work/t2.out" 2>/dev/null || true)
say "fake-server case: bridge-exit=$t2code out=[$t2out]"
if [ "$t2code" = "42" ] && [ "$t2out" = "bridge-42" ]; then
  ok "the bridge moved the server's bytes and exited with its status 42"
else
  miss "the fake-server case FAILED"
fi
progress "pipelined case starting"
timeout 60 python3 - "$BRIDGE" "$work/t2b.report" <<'PYEOF'
import os, pty, select, sys, time, tty
bridge, reportp = sys.argv[1], sys.argv[2]
m, s = pty.openpty()
tty.setraw(s)
slavename = os.ttyname(s)
pid = os.fork()
if pid == 0:
    os.setsid()
    os.dup2(s, 0); os.dup2(s, 1)
    os.close(m); os.close(s)
    # The server dies at once with 42. The feeder keeps writing past
    # the death: a megabyte the dead server never reads. The bridge
    # must still report 42, not its own write failure.
    os.execv(bridge, [bridge, slavename, "/bin/sh", "-c", "exit 42"])
    os._exit(127)
os.close(s)
sent = 0
t0 = time.monotonic()
os.set_blocking(m, False)
while time.monotonic() - t0 < 10.0:
    r, w, _ = select.select([m], [m], [], 1.0)
    if m in w and sent < 1024 * 1024:
        try:
            sent += os.write(m, b"x" * 65536)
        except OSError:
            pass
    if m in r:
        try:
            if not os.read(m, 65536):
                break
        except OSError:
            break
_, status = os.waitpid(pid, 0)
code = os.waitstatus_to_exitcode(status)
open(reportp, "w").write(f"bridge-exit={code} sent={sent}\n")
PYEOF
t2bcode=$(sed -n 's/^bridge-exit=//p' "$work/t2b.report" | head -1 | cut -d' ' -f1)
t2bsent=$(sed -n 's/^bridge-exit=[0-9]* sent=//p' "$work/t2b.report" | head -1)
say "pipelined case: bridge-exit=$t2bcode bytes-fed=$t2bsent"
if [ "$t2bcode" = "42" ] && [ "${t2bsent:-0}" -gt 0 ]; then
  ok "bytes past the server's death still report the server's status 42"
else
  miss "the pipelined case FAILED"
fi
fi

say ""
say "== 3. the pinned server behind the bridge, banner through a pty tty"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 2"
else
progress "building dropbear for the banner case"
git init --quiet "$DBSRC" 2>/dev/null || { miss "cannot init dropbear work"; }
if [ "$fail" -eq 0 ]; then
  git -C "$DBSRC" remote add origin "$DROPBEAR_ORIGIN" 2>/dev/null || true
  if timeout 120 git -C "$DBSRC" fetch --quiet --depth 1 origin "$DROPBEAR_COMMIT" >>"$OUT" 2>&1 \
      && git -C "$DBSRC" checkout --quiet FETCH_HEAD >>"$OUT" 2>&1; then
    ok "dropbear source at $(git -C "$DBSRC" rev-parse HEAD)"
  else
    miss "cannot fetch dropbear $DROPBEAR_COMMIT"
  fi
fi
if [ "$fail" -eq 0 ]; then
  c1=0; m1=0
  (cd "$DBSRC" && ./configure --enable-static --disable-zlib --disable-pam >>"$work/dropbear-configure.log" 2>&1) || c1=$?
  if [ "$c1" -eq 0 ]; then
    (cd "$DBSRC" && make dropbear dropbearkey STATIC=1 >>"$work/dropbear-make.log" 2>&1) || m1=$?
  fi
  if [ "$c1" -eq 0 ] && [ "$m1" -eq 0 ]; then
    ok "static dropbear built (gcc-configure=$c1 gcc-make=$m1)"
  else
    miss "static dropbear did not build (gcc-configure=$c1 gcc-make=$m1)"
  fi
fi
if [ "$fail" -eq 0 ]; then
"$DBSRC/dropbearkey" -t ed25519 -f "$work/hostkey" >>"$OUT" 2>&1 || exit 2
progress "banner case starting"
timeout 60 python3 - "$BRIDGE" "$DBSRC/dropbear" "$work/hostkey" "$work/t3.out" "$work/t3.report" <<'PYEOF'
import os, pty, select, sys, time, tty
bridge, server, key, outp, reportp = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
m, s = pty.openpty()
tty.setraw(s)
pid = os.fork()
if pid == 0:
    os.setsid()
    os.execv(bridge, [bridge, os.ttyname(s), server, "-i", "-s", "-g", "-r", key])
    os._exit(127)
os.close(s)
os.write(m, b"SSH-2.0-391-probe\r\n")
data = b""
t0 = time.monotonic()
while time.monotonic() - t0 < 10.0:
    r, _, _ = select.select([m], [], [], 1.0)
    if r:
        try:
            chunk = os.read(m, 65536)
        except OSError:
            break
        if not chunk:
            break
        data += chunk
        if b"SSH-2.0-dropbear" in data:
            break
open(outp, "wb").write(data)
_, status = os.waitpid(pid, os.WNOHANG)
if status == 0:
    alive = "running"
else:
    alive = "exited"
open(reportp, "w").write(f"bridge-state={alive}\n")
if alive == "running":
    try:
        os.kill(pid, 9)
    except OSError:
        pass
    os.waitpid(pid, 0)
PYEOF
t3out=$(head -c 120 "$work/t3.out" 2>/dev/null | LC_ALL=C tr -cd '[:print:]' | head -c 80 || true)
t3state=$(sed -n 's/^bridge-state=//p' "$work/t3.report" | head -1)
say "banner case: out-head=[$t3out] bridge-state=$t3state"
if printf '%s' "$t3out" | grep -q "SSH-2.0-dropbear"; then
  ok "the pinned server's banner crossed the bridge on a pty line"
else
  miss "the banner case FAILED"
fi
fi
fi

say ""
say "== 4. the failure edges: bad tty, bad server, signalled server"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 3"
else
# A tty that will not open ends the bridge before the server exists.
timeout 10 "$BRIDGE" /no/such/tty /bin/true >"$work/t4.out" 2>&1
t4=$?
if [ "$t4" -eq 125 ]; then
  ok "an unopenable tty refuses with 125"
else
  miss "bad tty FAILED (exit $t4)"
fi
# A server that will not exec ends it with the exec status, not silence:
# the socket EOFs at once and the wait reports 127. /dev/null as the tty
# doubles as the no-discipline proof: there is nothing to clear, so the
# splice runs as it is instead of refusing.
timeout 10 "$BRIDGE" /dev/null /no/such/server >"$work/t5.out" 2>&1
t5=$?
if [ "$t5" -eq 127 ]; then
  ok "an unexecable server exits 127"
else
  miss "bad server FAILED (exit $t5)"
fi
# A signal taking the server reads as 128 plus the signal.
timeout 10 "$BRIDGE" /dev/null /bin/sh -c 'kill -9 $$' >"$work/t6.out" 2>&1
t6=$?
if [ "$t6" -eq 137 ]; then
  ok "a signalled server exits 137"
else
  miss "signalled server FAILED (exit $t6)"
fi
fi

say ""
say "== 5. the tty reaches raw mode under the bridge"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 4"
else
# The device starts cooked (the pre-assert proves the test is not
# vacuous) and the bridge clears the mangling bits itself: output
# post-processing, canonical input, echo, signals, input mapping.
progress "raw-mode case starting"
timeout 60 python3 - "$BRIDGE" "$work/t7.report" <<'PYEOF'
import os, pty, termios, time, sys
bridge, reportp = sys.argv[1], sys.argv[2]
m, s = pty.openpty()
pre = termios.tcgetattr(s)
# tcgetattr order is iflag, oflag, cflag, lflag: index with care, a
# swapped pair here once failed the suite with the bridge innocent.
pre_cooked = bool(pre[3] & termios.ICANON) and bool(pre[3] & termios.ECHO) and bool(pre[1] & termios.OPOST)
slavename = os.ttyname(s)
pid = os.fork()
if pid == 0:
    os.setsid()
    os.close(m); os.close(s)
    os.execv("/usr/bin/timeout", ["timeout", "-s", "KILL", "12", bridge, slavename, "/bin/sleep", "25"])
    os._exit(127)
time.sleep(3.0)
post = termios.tcgetattr(m)
iflag, oflag, cflag, lflag = post[0], post[1], post[2], post[3]
raw = not (lflag & (termios.ICANON | termios.ECHO | termios.ECHONL | termios.ISIG | termios.IEXTEN)) \
  and not (oflag & termios.OPOST) \
  and not (iflag & (termios.IGNBRK | termios.BRKINT | termios.PARMRK | termios.ISTRIP | termios.INLCR | termios.IGNCR | termios.ICRNL | termios.IXON)) \
  and (cflag & termios.CSIZE) == termios.CS8 and not (cflag & termios.PARENB)
# The control characters ride beside the flags: the bridge pins
# VMIN/VTIME (machine-bridge.c), and deleting that pin must redden this.
cc = post[6]
cc_ok = cc[termios.VMIN] == 1 and cc[termios.VTIME] == 0
open(reportp, "w").write(f"pre-cooked={pre_cooked} raw={raw} cc={cc_ok}\n")
os.waitpid(pid, 0)
PYEOF
t7=$(cat "$work/t7.report" 2>/dev/null || echo missing)
say "raw-mode case: $t7"
if [ "$t7" = "pre-cooked=True raw=True cc=True" ]; then
  ok "the device starts cooked and the bridge leaves it raw with VMIN/VTIME pinned"
else
  miss "the raw-mode case FAILED"
fi
fi

say ""
say "== 6. a dead tty half-closes the server instead of wedging it"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 5"
else
# The server echoes first so the splice is proved live both ways, then
# the tty side dies: the bridge must half-close the server's stdin so
# cat sees EOF and exits 0. Without the half-close cat wedges and this
# case hangs past its budget instead of reporting.
progress "half-close case starting"
timeout 60 python3 - "$BRIDGE" "$work/t8.report" <<'PYEOF'
import os, pty, select, sys, time, tty
bridge, reportp = sys.argv[1], sys.argv[2]
m, s = pty.openpty()
tty.setraw(s)
slavename = os.ttyname(s)
pid = os.fork()
if pid == 0:
    os.setsid()
    os.dup2(s, 0); os.dup2(s, 1)
    os.close(m); os.close(s)
    os.execv(bridge, [bridge, slavename, "/bin/cat"])
    os._exit(127)
os.close(s)
os.write(m, b"halfclose-probe\n")
got = b""
t0 = time.monotonic()
while time.monotonic() - t0 < 10.0 and b"halfclose-probe" not in got:
    r, _, _ = select.select([m], [], [], 1.0)
    if m in r:
        try:
            chunk = os.read(m, 65536)
        except OSError:
            chunk = b""
        if not chunk:
            break
        got += chunk
alive = b"halfclose-probe" in got
os.close(m)
code = None
t1 = time.monotonic()
while time.monotonic() - t1 < 15.0:
    done, status = os.waitpid(pid, os.WNOHANG)
    if done == pid:
        code = os.waitstatus_to_exitcode(status)
        break
    time.sleep(0.1)
if code is None:
    os.kill(pid, 9)
    os.waitpid(pid, 0)
    code = "hung"
open(reportp, "w").write(f"echo-alive={alive} bridge-exit={code}\n")
PYEOF
t8=$(cat "$work/t8.report" 2>/dev/null || echo missing)
say "half-close case: $t8"
if [ "$t8" = "echo-alive=True bridge-exit=0" ]; then
  ok "a dead tty releases the server with its status 0, no wedge"
else
  miss "the half-close case FAILED"
fi
fi

say ""
if [ "$fail" -eq 0 ]; then
  say "verdict           BRIDGE-OK: the bridge builds static, splices both ways, and reports every status"
else
  say "verdict           BRIDGE-FAIL"
fi
cp "$OUT" /out/ 2>/dev/null || true
exit "$fail"
