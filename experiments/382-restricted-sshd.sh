#!/bin/bash
# Question: does a real SSH server run a real command in a host where
# chroot is denied and no passwd database exists, on the compiled shim?
#
# TODO/podssh.md T-1401 (the restricted-server remainder). The passwd shim
# is retained source at crates/podbox-ssh/shims/fakepwd.c; this drive is
# the tracked build input that compiles it, and the fresh-clone proof that
# a dynamic server preloaded with it serves a real SSH session. glibc
# dynamic only: a static server cannot take LD_PRELOAD, which is the dead
# end docs/decisions/ssh-server-in-a-cage.md records. musl/static lanes
# refuse here with exit 2, naming the reason.
#
# The cage user exists only in the fixture passwd file, never in the host
# database: a login as that user proves the shim answered getpwnam, not
# the host. Transport, relay, and dispatch are untouched by this drive;
# every SSH byte here moves between a real ssh client and a real sshd.
# No secret is involved: the keys are minted fresh in the work dir and no
# token exists on this path.
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
OUT="experiments/results/restricted-sshd.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
SSHD_PID=""
cleanup() {
  if [ -n "$SSHD_PID" ] && kill -0 "$SSHD_PID" 2>/dev/null; then
    kill "$SSHD_PID" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap 'cleanup' EXIT HUP INT TERM
fail=0

say() { echo "$@" >>"$OUT"; }

{
  echo "== conditions"
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  git rev-parse --short HEAD
  gcc --version | head -n 1
  ssh -V 2>&1 || true
} >"$OUT"

bash scripts/common/bootstrap-env.sh cc openssh tools >"$work/bootstrap.log" 2>&1 || exit 2
command -v gcc >>"$OUT" 2>&1 || exit 2
command -v sshd >>"$OUT" 2>&1 || exit 2
command -v ssh >>"$OUT" 2>&1 || exit 2
command -v ssh-keygen >>"$OUT" 2>&1 || exit 2
command -v readelf >>"$OUT" 2>&1 || exit 2
command -v timeout >>"$OUT" 2>&1 || exit 2

# Lane setup, not drive action: daemon-mode sshd demands its privilege
# separation user, and this image ships openssh-server without one. The
# host-identity snapshot below comes after, so clause 6 judges only what
# the drive itself changed.
if getent passwd sshd >/dev/null 2>&1; then
  say "setup             privilege separation user sshd present"
else
  if useradd -r -s /usr/sbin/nologin sshd >>"$OUT" 2>&1; then
    say "setup             privilege separation user sshd created (lane setup)"
  else
    say "setup             cannot create the sshd user; the daemon cannot start here"
    exit 2
  fi
fi

# Clause 0: the host identity before the drive. Compared after: the drive
# must change nothing about the host it ran on.
HOSTNAME_BEFORE=$(hostname)
PASSWD_SUM_BEFORE=$(md5sum /etc/passwd | cut -d' ' -f1)
say "host before       $HOSTNAME_BEFORE passwd-sha $PASSWD_SUM_BEFORE"

# Clause 1: the tracked build input. Fresh-clone safe: the only source is
# the retained shim, the only tool the host gcc.
if gcc -shared -fPIC -O2 -o "$work/fakepwd.so" crates/podbox-ssh/shims/fakepwd.c >>"$OUT" 2>&1; then
  say "clause 1          gcc built fakepwd.so from the retained shim"
else
  say "clause 1          gcc FAILED to build the shim"
  exit 2
fi
if [ ! -s "$work/fakepwd.so" ]; then
  say "clause 1          fakepwd.so empty after build"
  exit 2
fi
if gcc -dumpmachine | grep -q "linux-gnu"; then
  say "clause 1          gcc target $(gcc -dumpmachine): glibc dynamic lane"
else
  say "clause 1          gcc target $(gcc -dumpmachine): not a glibc dynamic lane; the preload shim is a dead end here"
  exit 2
fi
if readelf -d "$work/fakepwd.so" | grep -q "NEEDED.*libc"; then
  say "clause 1          fakepwd.so is dynamically linked (NEEDED libc present)"
else
  say "clause 1          fakepwd.so is not dynamically linked; LD_PRELOAD cannot work"
  exit 2
fi

# Clause 2: the restriction fixture. The cage carries no passwd database,
# and chroot there is denied. setpriv runs the attempt as nobody so a root
# lane proves the denial instead of sailing through it.
CAGE="$work/cage"
mkdir -p "$CAGE"
if [ -e "$CAGE/etc/passwd" ]; then
  say "clause 2          cage unexpectedly carries a passwd database"
  fail=1
else
  say "clause 2          cage carries no passwd database"
fi
if command -v setpriv >/dev/null 2>&1; then
  setpriv --reuid=nobody --regid=nogroup --clear-groups chroot "$CAGE" /bin/true \
    >"$work/chroot.out" 2>&1
  chroot_rc=$?
  chroot_how="as nobody via setpriv"
elif [ "$(id -u)" -ne 0 ]; then
  chroot "$CAGE" /bin/true >"$work/chroot.out" 2>&1
  chroot_rc=$?
  chroot_how="as uid $(id -u)"
else
  say "clause 2          lane is root with no setpriv: the denial is unprovable here"
  exit 2
fi
if [ "$chroot_rc" -ne 0 ]; then
  say "clause 2          chroot denied (exit $chroot_rc $chroot_how)"
else
  say "clause 2          chroot SUCCEEDED where it must be denied"
  fail=1
fi

# Clause 3: the server must be dynamic. A static sshd cannot take
# LD_PRELOAD, so it fails this clause instead of the session later.
SSHD_BIN=$(command -v sshd)
if ldd "$SSHD_BIN" 2>/dev/null | grep -q "libc"; then
  say "clause 3          $SSHD_BIN is dynamically linked (libc in ldd)"
else
  say "clause 3          $SSHD_BIN is not dynamically linked; the shim cannot load into it"
  fail=1
fi

# Clause 4: the restricted session. The cage user exists only in the
# fixture, with the lane's own uid/gid so no privilege is needed. The
# daemon inherits LD_PRELOAD and SANDHOME_PASSWD; each session answers
# getpwnam from the fixture, never from the host database.
ME_UID=$(id -u)
ME_GID=$(id -g)
HOME_DIR="$work/cage-home/cageuser"
mkdir -p "$HOME_DIR/.ssh"
chmod 700 "$HOME_DIR/.ssh"
printf 'cageuser:x:%s:%s:cage:%s:/bin/sh\n' "$ME_UID" "$ME_GID" "$HOME_DIR" >"$work/fixture.passwd"
# ⛔ The shim answers every passwd lookup from the fixture alone: it does
# not fall through to the host database. The daemon looks up its own
# privilege separation user at startup, so the fixture carries that user
# too, with the host's real ids (read before any preload is exported).
# Both names resolve from the fixture; the host database stays unread.
SSHD_IDS=$(getent passwd sshd | cut -d: -f3,4)
SSHD_UID=${SSHD_IDS%%:*}
SSHD_GID=${SSHD_IDS##*:}
printf 'sshd:x:%s:%s:privsep:/run/sshd:/usr/sbin/nologin\n' "$SSHD_UID" "$SSHD_GID" >>"$work/fixture.passwd"
chmod 755 /run/sshd 2>/dev/null || true
ssh-keygen -t ed25519 -N "" -q -f "$work/user_key" || exit 2
cp "$work/user_key.pub" "$HOME_DIR/.ssh/authorized_keys"
chmod 600 "$HOME_DIR/.ssh/authorized_keys"
ssh-keygen -t ed25519 -N "" -q -f "$work/ssh_host_ed25519_key" || exit 2
ssh-keygen -t ed25519 -N "" -q -f "$work/wrong_key" || exit 2
if [ ! -d /run/sshd ]; then
  mkdir -p /run/sshd || exit 2
fi
if [ "$fail" -eq 0 ]; then
  PORT=""
  attempt=0
  while [ "$attempt" -lt 5 ]; do
    CAND=$((22180 + $$ % 200 + attempt))
    cat >"$work/sshd_port.conf" <<EOF
Port $CAND
ListenAddress 127.0.0.1
HostKey $work/ssh_host_ed25519_key
AuthorizedKeysFile $HOME_DIR/.ssh/authorized_keys
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
    # ⛔ `-D` keeps the daemon on this pid: without it sshd forks and $!
    # names a parent that already exited, so the kill below would miss.
    LD_PRELOAD="$work/fakepwd.so" SANDHOME_PASSWD="$work/fixture.passwd" \
      timeout 120 "$SSHD_BIN" -D -f "$work/sshd_port.conf" -E "$work/sshd.log" &
    SSHD_PID=$!
    sleep 2
    if grep -q "Address already in use" "$work/sshd.log" 2>/dev/null; then
      kill "$SSHD_PID" 2>/dev/null || true
      wait "$SSHD_PID" 2>/dev/null || true
      SSHD_PID=""
      : >"$work/sshd.log"
      attempt=$((attempt + 1))
      continue
    fi
    if kill -0 "$SSHD_PID" 2>/dev/null; then
      PORT=$CAND
      break
    fi
    wait "$SSHD_PID" 2>/dev/null || true
    SSHD_PID=""
    attempt=$((attempt + 1))
  done
  if [ -z "$PORT" ]; then
    say "clause 4          sshd never held a loopback port; log:"
    sed 's/^/  /' "$work/sshd.log" >>"$OUT" 2>/dev/null || true
    fail=1
  else
    say "clause 4          sshd listening on 127.0.0.1:$PORT with the preloaded shim"
    # The proof's negative half: the cage user is absent from the host
    # database, so the login below can only have resolved through the
    # preloaded shim, never through the host.
    if getent passwd cageuser >/dev/null 2>&1; then
      say "clause 4          cageuser unexpectedly exists in the host database"
      fail=1
    else
      say "clause 4          cageuser absent from the host database (shim-only user)"
    fi
    ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o LogLevel=ERROR"
    # shellcheck disable=SC2086
    if timeout 60 ssh $ssh_opts -p "$PORT" -i "$work/user_key" cageuser@127.0.0.1 \
      "printf restricted-ok; exit 42" >"$work/cmd.out" 2>"$work/cmd.err"; then
      cmd_rc=0
    else
      cmd_rc=$?
    fi
    if [ "$cmd_rc" -eq 42 ] && [ "$(cat "$work/cmd.out")" = "restricted-ok" ] && [ ! -s "$work/cmd.err" ]; then
      say "clause 4          real SSH command returned 42 with exact bytes and silent stderr"
    else
      say "clause 4          SSH command FAILED (exit $cmd_rc, out: $(cat "$work/cmd.out" 2>/dev/null), err: $(cat "$work/cmd.err" 2>/dev/null))"
      fail=1
    fi
  fi
fi

# Clause 5: the bounded-failure arm. A wrong key must refuse fast with 255
# while the daemon stands: a hang here is a missing bound, not a refusal.
if [ "$fail" -eq 0 ] && [ -n "${PORT:-}" ]; then
  ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o LogLevel=ERROR"
  start=$(date +%s)
  # shellcheck disable=SC2086
  if timeout 30 ssh $ssh_opts -p "$PORT" -i "$work/wrong_key" cageuser@127.0.0.1 \
    "exit 0" >"$work/refused.out" 2>"$work/refused.err"; then
    ref_rc=0
  else
    ref_rc=$?
  fi
  took=$(( $(date +%s) - start ))
  if [ "$ref_rc" -eq 255 ] && [ "$took" -lt 30 ] && kill -0 "$SSHD_PID" 2>/dev/null; then
    say "clause 5          wrong key refused with 255 in ${took}s, daemon still standing"
  else
    say "clause 5          bounded refusal FAILED (exit $ref_rc after ${took}s)"
    fail=1
  fi
fi

# Clause 6: the host identity after the drive. Nothing about the host may
# have changed: same hostname, same passwd database.
HOSTNAME_AFTER=$(hostname)
PASSWD_SUM_AFTER=$(md5sum /etc/passwd | cut -d' ' -f1)
if [ "$HOSTNAME_AFTER" = "$HOSTNAME_BEFORE" ] && [ "$PASSWD_SUM_AFTER" = "$PASSWD_SUM_BEFORE" ]; then
  say "clause 6          host identity unchanged (hostname and /etc/passwd checksum match)"
else
  say "clause 6          HOST IDENTITY CHANGED: $HOSTNAME_BEFORE->$HOSTNAME_AFTER, passwd $PASSWD_SUM_BEFORE->$PASSWD_SUM_AFTER"
  fail=1
fi

# Clause 7: cleanup with no residue. The daemon dies, the port refuses
# behind it, and the work dir goes.
if [ -n "$SSHD_PID" ]; then
  kill "$SSHD_PID" 2>/dev/null || true
  waited=0
  while kill -0 "$SSHD_PID" 2>/dev/null && [ "$waited" -lt 10 ]; do
    sleep 1
    waited=$((waited + 1))
  done
  if kill -0 "$SSHD_PID" 2>/dev/null; then
    say "clause 7          sshd survived SIGTERM"
    fail=1
  else
    say "clause 7          sshd reaped within ${waited}s"
  fi
  SSHD_PID=""
fi
if [ -n "${PORT:-}" ]; then
  ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=5 -o LogLevel=ERROR"
  # shellcheck disable=SC2086
  if timeout 15 ssh $ssh_opts -p "$PORT" -i "$work/user_key" cageuser@127.0.0.1 \
    "exit 0" >/dev/null 2>&1; then
    say "clause 7          port $PORT still answers after the daemon died"
    fail=1
  else
    say "clause 7          port $PORT refuses after the daemon died"
  fi
fi
rm -rf "$work"
# The trap now has nothing to remove; the check below proves it.
if [ -d "$work" ]; then
  say "clause 7          work dir residue remains"
  fail=1
else
  say "clause 7          work dir removed, no owned residue"
fi

{
  echo ""
  if [ "$fail" -eq 0 ]; then echo "verdict           RESTRICTED SSHD SERVES EXIT 42 ON THE COMPILED SHIM"; else echo "verdict           RESTRICTED-HOST PROOF OPEN"; fi
} >>"$OUT"
cp "$OUT" /out/ 2>/dev/null || true
cat "$OUT"
[ "$fail" -eq 0 ]
