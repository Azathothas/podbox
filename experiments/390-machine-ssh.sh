#!/bin/bash
# Question: does `podbox machine ssh` open a real SSH session in a Linux
# guest podbox boots, run a real command there, and report its exit code?
#
# TODO/podssh.md T-1404 (the machine arm). A static dropbear built from a
# pinned commit serves on the guest's serial line through an owned static
# bridge: dropbear's inetd path calls getpeername on stdin and exits where
# that is not a socket (diag5/diag6: exit 1 with no banner on pipes and
# ptys, a banner on a socket pair; svr-main.c main_inetd into netio.c
# get_socket_address), and a serial line is never a socket. Patching the
# server would fork a security boundary the tree already watched rot, so
# the server stays pristine and experiments/lib/machine-bridge.c adapts:
# one socket pair, the server on one end, the other spliced to the tty.
# The guest is the 147-pinned Alpine rootfs plus kernel, assembled with
# the podvm-guest library, with dropbear, the bridge, a dropbearkey-native
# host key (the pinned server exits 1 with no banner on an ssh-keygen key
# even on a socket: diag8) and the authorized key baked in. The lane-built podbox boots it with QEMU,
# speaks real SSH over the serial socket through the lane-built proxy,
# and exits with the guest's code. Help paths print usage with 0, error
# paths refuse with 125, the short timeout run proves the wait is
# bounded, and nothing of the run remains.
#
# Exit 0 every clause held, 1 a clause disagreed, 2 the lane could not run.
set -u

cd /work || exit 2
# The checkout this job runs from: the wrapper executes this file at
# /in/job.sh with /work as the working directory, so $0-relative lookup
# would land in /in. The results path below is already checkout-relative
# for the same reason.
LIB="$PWD/experiments/lib/podvm-guest.sh"
[ -f "$LIB" ] || exit 2
# Absolute: the dropbear build below runs in its own source directory, and
# a relative results path under a changed directory loses the verdict with
# a bare redirect error (measured twice in the T-1404 diagnostic series).
OUT="$PWD/experiments/results/machine-ssh.txt"
mkdir -p experiments/results /out 2>/dev/null || exit 2
work=$(mktemp -d) || exit 2
fail=0
# Partial evidence always comes home: an early exit still leaves the
# conditions and whatever clauses ran in /out beside the job log, and the
# workdir leaves with the run either way.
trap 'cp "$OUT" /out/ 2>/dev/null || true; rm -rf "$work"' EXIT HUP INT TERM
STORE="$work/store"
export PODBOX_STORE="$STORE"
DBSRC="$work/dropbear-src"

# ⭐ Every input pinned, as in 146/147.
ALPINE_REF='public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764'
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KERNEL_SHA256="6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51"
DROPBEAR_COMMIT="59870ad43153fe8d4f1c96f5d5752116c94f31ff"
DROPBEAR_ORIGIN="https://github.com/mkj/dropbear"
CURL_TIMEOUT=60

say() { printf '%s\n' "$*" >>"$OUT"; }
miss() { fail=1; say "  FAIL: $1"; }
ok() { say "  ok: $1"; }

{
  echo "== conditions"
  printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel       %s\n' "$(uname -r)"
  printf 'image             %s\n' "$ALPINE_REF"
  printf 'kernel            %s\n' "$KURL"
  printf 'dropbear          %s %s\n' "$DROPBEAR_ORIGIN" "$DROPBEAR_COMMIT"
  echo
} >"$OUT"

bash scripts/common/bootstrap-env.sh rust cc zig openssh tools qemu vmtools >"$work/bootstrap.log" 2>&1 || { echo "SKIP: bootstrap failed" >&2; exit 2; }
for t in qemu-system-x86_64 cpio python3 curl sha256sum timeout ssh ssh-keygen file make cc zig git; do
  command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

# shellcheck source=lib/podvm-guest.sh
. "$LIB"

say "== 1. the lane-built binaries and the help paths"
if timeout 25m cargo build -p podbox-cli -p podbox-ssh --bins >>"$OUT" 2>&1; then
  ok "cargo build -p podbox-cli -p podbox-ssh --bins exit 0"
else
  say "  cargo build FAILED"
  exit 2
fi
PODBOX_BIN=$(ls -d target/*/debug/podbox 2>/dev/null | head -n 1)
PROXY_BIN=$(ls -d target/*/debug/proxy 2>/dev/null | head -n 1)
if [ -z "${PODBOX_BIN:-}" ] || [ -z "${PROXY_BIN:-}" ]; then
  say "  podbox or proxy binary missing"
  exit 2
fi
mkdir -p "$work/bin"
cp "$PODBOX_BIN" "$PROXY_BIN" "$work/bin/"
PODBOX="$work/bin/podbox"
export PATH="$work/bin:$PATH"
ok "podbox learns its version: $("$PODBOX" version)"
for path in "machine" "machine -h" "machine ssh -h"; do
  # shellcheck disable=SC2086
  if timeout 60 $PODBOX $path >"$work/help.out" 2>&1; then
    code=0
  else
    code=$?
  fi
  if [ "$code" -eq 0 ] && grep -q "^usage:" "$work/help.out"; then
    ok "podbox $path: usage with 0"
  else
    miss "podbox $path (exit $code)"
  fi
done
timeout 60 $PODBOX machine ssh >"$work/noflags.out" 2>"$work/noflags.err"
ncode=$?
if [ "$ncode" -eq 125 ] && grep -q "\-\-kernel" "$work/noflags.err"; then
  ok "machine ssh with no flags refused with 125 naming --kernel"
else
  miss "machine ssh no-flags FAILED (exit $ncode)"
fi
timeout 60 $PODBOX machine ssh --kernel "$work/no-such-kernel" \
  --initramfs "$work/no-such-initramfs" --user-key "$work/no-such-key" \
  >"$work/missing.out" 2>"$work/missing.err"
mcode=$?
if [ "$mcode" -eq 125 ] && grep -q "is not a file" "$work/missing.err"; then
  ok "machine ssh with missing files refused with 125 naming the path"
else
  miss "machine ssh missing-file FAILED (exit $mcode)"
fi
timeout 60 $PODBOX machine ssh --podbox-timeout 0 >"$work/t0.out" 2>"$work/t0.err"
tcode=$?
if [ "$tcode" -eq 125 ] && grep -q "positive count" "$work/t0.err"; then
  ok "machine ssh --podbox-timeout 0 refused with 125"
else
  miss "machine ssh timeout-0 FAILED (exit $tcode)"
fi
timeout 60 $PODBOX machine ssh --bogus >"$work/bogus.out" 2>"$work/bogus.err"
bcode=$?
if [ "$bcode" -eq 125 ] && grep -q "not a flag" "$work/bogus.err"; then
  ok "machine ssh --bogus refused with 125"
else
  miss "machine ssh --bogus FAILED (exit $bcode)"
fi

say ""
say "== 2. a static dropbear from the pinned commit"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 1"
else
git init --quiet "$DBSRC" 2>/dev/null || { miss "cannot init dropbear work"; }
if [ "$fail" -eq 0 ]; then
  git -C "$DBSRC" remote add origin "$DROPBEAR_ORIGIN" 2>/dev/null || true
  if git -C "$DBSRC" fetch --quiet --depth 1 origin "$DROPBEAR_COMMIT" >>"$OUT" 2>&1 \
      && git -C "$DBSRC" checkout --quiet FETCH_HEAD >>"$OUT" 2>&1; then
    ok "dropbear source at $(git -C "$DBSRC" rev-parse HEAD)"
  else
    miss "cannot fetch dropbear $DROPBEAR_COMMIT"
  fi
fi
if [ "$fail" -eq 0 ]; then
  # The guest is a real kernel, not a seccomp-filtered cage, so setgroups
  # works there and the cage patch stays out: fewer moving parts. Static
  # because the guest carries no loader but the kernel: a dynamic binary
  # built here would not execute there. No zlib: one less library to place.
  # Scope against docs/decisions/ssh-server-in-a-cage.md, which orders
  # DYNAMIC for the opposite situation: a cage with no passwd database,
  # where only an LD_PRELOAD shim can supply the accounts and a static
  # binary cannot see them. The guest carries root in its own
  # /etc/passwd (asserted below), so static links fine here.
  # Subshells, never a bare cd: the log path above is absolute, but a leg
  # that forgets its way back would still strand the relative tool paths,
  # so the directory never changes here. Every leg records its own exit.
  c1=0; m1=0; c2=0; m2=0; dd=0
  (cd "$DBSRC" && ./configure --enable-static --disable-zlib --disable-pam >>"$work/dropbear-configure.log" 2>&1) || c1=$?
  if [ "$c1" -eq 0 ]; then
    (cd "$DBSRC" && make dropbear dropbearkey STATIC=1 >>"$work/dropbear-make.log" 2>&1) || m1=$?
  fi
  if [ "$c1" -eq 0 ] && [ "$m1" -eq 0 ]; then
    ok "static dropbear built with $(cc --version | head -1)"
  else
    (cd "$DBSRC" && make distclean >>"$work/dropbear-make.log" 2>&1) || dd=$?
    (cd "$DBSRC" && CC="zig cc -target x86_64-linux-musl" ./configure --enable-static --disable-zlib --disable-pam >>"$work/dropbear-configure.log" 2>&1) || c2=$?
    if [ "$c2" -eq 0 ]; then
      (cd "$DBSRC" && make dropbear dropbearkey STATIC=1 CC="zig cc -target x86_64-linux-musl" >>"$work/dropbear-make.log" 2>&1) || m2=$?
    fi
    if [ "$c2" -eq 0 ] && [ "$m2" -eq 0 ]; then
      ok "static dropbear built with zig cc musl after gcc failed (gcc-configure=$c1 gcc-make=$m1)"
    else
      miss "static dropbear did not build (gcc-configure=$c1 gcc-make=$m1 distclean=$dd zig-configure=$c2 zig-make=$m2; see dropbear-make.log)"
    fi
  fi
fi
DROPBEAR="$DBSRC/dropbear"
DROPBEARKEY="$DBSRC/dropbearkey"
if [ "$fail" -eq 0 ]; then
  if [ -x "$DROPBEAR" ] && file "$DROPBEAR" | grep -q "statically linked"; then
    ok "dropbear is statically linked: $(file "$DROPBEAR" | cut -c1-120)"
  else
    miss "dropbear is not a static binary"
  fi
  [ -x "$DROPBEARKEY" ] || miss "dropbearkey did not build"
fi
fi

say ""
say "== 3. the socket bridge (owned, static, beside the pristine server)"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 2"
else
# dropbear's inetd path needs a socket where the guest has a serial line
# (diag5/diag6: exit 1 with no banner on pipes and ptys, a banner on a
# socket pair; svr-main.c main_inetd into netio.c get_socket_address).
# The server stays pristine and this owned bridge adapts: one socket pair,
# the server forked onto one end, the other spliced against the tty.
BRIDGE_SRC="$PWD/experiments/lib/machine-bridge.c"
[ -f "$BRIDGE_SRC" ] || { miss "the bridge source is not in the tree"; }
if [ "$fail" -eq 0 ]; then
  say "zig: $(zig version 2>/dev/null || echo MISSING)"
  if zig cc -target x86_64-linux-musl -static -O2 -Wall -Wextra -Werror \
      -o "$work/machine-bridge" "$BRIDGE_SRC" >>"$OUT" 2>&1; then
    ok "static bridge built with zig $(zig version 2>/dev/null || echo unknown)"
  else
    miss "the static bridge did not build"
  fi
fi
if [ "$fail" -eq 0 ]; then
  if [ -x "$work/machine-bridge" ] && file "$work/machine-bridge" | grep -q "statically linked"; then
    ok "bridge is statically linked: $(file "$work/machine-bridge" | cut -c1-120)"
  else
    miss "bridge is not a static binary"
  fi
fi
fi

say ""
say "== 4. the guest assembly"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 3"
else
"$PODBOX" pull "$ALPINE_REF" >"$work/pull.log" 2>&1 || miss "the pull failed"
ROOTFS=""
if [ "$fail" -eq 0 ]; then
  ROOTFS="$("$PODBOX" extract "$ALPINE_REF" 2>"$work/extract.log")"
  [ -n "$ROOTFS" ] && [ -d "$ROOTFS" ] || miss "extract printed no rootfs directory"
fi
if [ "$fail" -eq 0 ]; then
  grep -q '^root:' "$ROOTFS/etc/passwd" || miss "the rootfs has no root account"
  mkdir -p "$ROOTFS/sbin" "$ROOTFS/etc/dropbear" "$ROOTFS/root/.ssh" || miss "cannot stage the server paths"
  cp "$DROPBEAR" "$ROOTFS/sbin/dropbear" && chmod 755 "$ROOTFS/sbin/dropbear" || miss "cannot place dropbear"
  cp "$work/machine-bridge" "$ROOTFS/sbin/machine-bridge" && chmod 755 "$ROOTFS/sbin/machine-bridge" || miss "cannot place the bridge"
  # The host key is baked in dropbear's native format, not OpenSSH's: the
  # pinned server exits 1 with no banner on an ssh-keygen key even where
  # the stdin is a socket (diag8), and serves on a dropbearkey one. The
  # user key stays ssh-keygen: the lane client reads its own format, and
  # the server reads the OpenSSH public line natively.
  "$DROPBEARKEY" -t ed25519 -f "$work/host_key" >>"$OUT" 2>&1 || { miss "dropbearkey did not write a host key"; }
  ssh-keygen -t ed25519 -N "" -q -f "$work/user_key" || exit 2
  cp "$work/host_key" "$ROOTFS/etc/dropbear/hostkey" && chmod 600 "$ROOTFS/etc/dropbear/hostkey" || miss "cannot place the host key"
  cp "$work/user_key.pub" "$ROOTFS/root/.ssh/authorized_keys" && chmod 600 "$ROOTFS/root/.ssh/authorized_keys" || miss "cannot place the authorized key"
  cat >"$work/init-ssh" <<'INITEOF'
#!/bin/sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
# The bridge gives the server the socket its inetd path needs, on the
# serial line the guest actually has (clause 3). The server keeps its
# session; the bridge only moves bytes and exits with its status.
exec /sbin/machine-bridge /dev/ttyS0 /sbin/dropbear -i -s -g -r /etc/dropbear/hostkey
INITEOF
  podvm_base "$ROOTFS" "$work/base.cpio" || miss "the base cpio did not build"
  podvm_extras "$work/init-ssh" "$work/extras.cpio" || miss "the extras archive did not build"
  if [ ! -e "$ROOTFS/dev/ttyS0" ]; then
    say "  the base carries no dev/ttyS0: appending the node archive"
    # The serial line needs its node and the base carries none: bytes in
    # the newc shape, not a device the host creates (podvm-guest.sh).
    python3 - "$work/nodes.cpio" <<'PYEOF' || miss "the ttyS0 node archive did not build"
import sys
out = sys.argv[1]
def newc(name, mode, filesize=0, rdevmaj=0, rdevmin=0, data=b""):
    name_b = name.encode() + b"\0"
    head = ["070701", "00000000", format(mode, "08x"), "00000000",
            "00000000", "00000001", "00000000", format(filesize, "08x"),
            "00000000", "00000000", format(rdevmaj, "08x"),
            format(rdevmin, "08x"), format(len(name_b), "08x"), "00000000"]
    rec = "".join(head).encode() + name_b
    rec += b"\0" * ((4 - len(rec) % 4) % 4)
    rec += data + b"\0" * ((4 - len(data) % 4) % 4)
    return rec
rec = newc("dev", 0o040755)
rec += newc("dev/ttyS0", 0o020600, rdevmaj=4, rdevmin=64)
rec += newc("TRAILER!!!", 0)
open(out, "wb").write(rec)
PYEOF
    [ -s "$work/nodes.cpio" ] || miss "the ttyS0 node archive is empty"
    cat "$work/base.cpio" "$work/extras.cpio" "$work/nodes.cpio" >"$work/full.cpio" || miss "concat failed"
  else
    say "  the base carries dev/ttyS0 already"
    podvm_concat "$work/base.cpio" "$work/extras.cpio" "$work/full.cpio" || miss "concat failed"
  fi
  ok "assembly: $(wc -c <"$work/full.cpio") bytes"
  # The archives are asserted per piece, not assumed: `cpio -t` stops at
  # the first TRAILER, so the concatenated image cannot be listed as one
  # (podvm-guest.sh). A missing byte fails this clause, not the boot.
  for need in "sbin/dropbear" "sbin/machine-bridge" "etc/dropbear/hostkey" "root/.ssh/authorized_keys"; do
    if cpio -t <"$work/base.cpio" 2>/dev/null | grep -qx "$need"; then
      ok "the base archive carries $need"
    else
      miss "the base archive misses $need"
    fi
  done
  for need in "init" "dev/console"; do
    if cpio -t <"$work/extras.cpio" 2>/dev/null | grep -qx "$need"; then
      ok "the extras archive carries $need"
    else
      miss "the extras archive misses $need"
    fi
  done
  if [ ! -e "$ROOTFS/dev/ttyS0" ]; then
    if cpio -t <"$work/nodes.cpio" 2>/dev/null | grep -qx "dev/ttyS0"; then
      ok "the node archive carries dev/ttyS0"
    else
      miss "the node archive misses dev/ttyS0"
    fi
  fi
fi
if [ "$fail" -eq 0 ]; then
  if curl -fsSL --max-time "$CURL_TIMEOUT" -o "$work/vmlinuz-virt" "$KURL" \
      && printf '%s  %s\n' "$KERNEL_SHA256" "$work/vmlinuz-virt" | sha256sum -c - >>"$OUT" 2>&1; then
    ok "kernel matches the pin"
  else
    miss "the kernel did not fetch or match the pin"
  fi
fi
fi

say ""
say "== 5. one SSH command in the guest podbox boots"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 4"
else
# The typed bytes hold the arithmetic and the flag text; only the
# guest's expansions read `guest-42` and `PLACED`, so those markers prove
# the guest ran them. The placement line matches its own flag text on
# purpose: reading the guest's cmdline back says which kernel answered.
# One boot carries all three (an inetd-mode server answers once), and the
# exit-code passthrough takes a second boot by the same rule.
out=$(timeout 500 "$PODBOX" machine ssh --kernel "$work/vmlinuz-virt" \
  --initramfs "$work/full.cpio" --user-key "$work/user_key" \
  -- "echo guest-\$((40+2)); grep -q panic=-1 /proc/cmdline && echo PLACED; id -u" 2>"$work/ssh.err")
code=$?
if [ "$code" -eq 0 ] && [ "$out" = "$(printf 'guest-42\nPLACED\n0')" ]; then
  ok "the guest ran the commands with exit 0, placed itself, and is root"
else
  miss "the guest run FAILED (exit $code, out: $out, err: $(head -1 "$work/ssh.err" 2>/dev/null))"
fi
if [ "$fail" -eq 0 ]; then
  # The exit-code passthrough needs its own boot: an inetd-mode server
  # answers one connection, so one boot carries one status.
  timeout 500 "$PODBOX" machine ssh --kernel "$work/vmlinuz-virt" \
    --initramfs "$work/full.cpio" --user-key "$work/user_key" \
    -- exit 42 2>/dev/null
  c42=$?
  if [ "$c42" -eq 42 ]; then
    ok "exit 42 passed through"
  else
    miss "exit 42 FAILED: got $c42"
  fi
fi
fi

say ""
say "== 6. the wait is bounded and nothing remains"
if [ "$fail" -ne 0 ]; then
  say "  skipped past clause 5"
else
# One second stays under every measured floor (the fastest healthy run
# needs the emulator spawn plus a full guest boot, about two seconds)
# and above the spawn, so the deadline arm always fires here. Five
# seconds used to be that bound; the fixed path completes a healthy
# run inside it, which proved the bound stale rather than the wait
# unbounded.
timeout 60 "$PODBOX" machine ssh --kernel "$work/vmlinuz-virt" \
  --initramfs "$work/full.cpio" --user-key "$work/user_key" \
  --podbox-timeout 1 -- echo too-late 2>"$work/bound.err"
bound=$?
if [ "$bound" -eq 125 ] && grep -Eq "before the deadline|did not answer in time" "$work/bound.err"; then
  ok "the 1 s run stopped with 125, not a hang"
else
  miss "the bounded run FAILED (exit $bound): $(head -1 "$work/bound.err" 2>/dev/null)"
fi
leftovers=$(ls -d "${TMPDIR:-/tmp}"/podbox-machine-ssh-* 2>/dev/null || true)
if [ -z "$leftovers" ]; then
  ok "no per-run directory remains"
else
  miss "per-run residue: $leftovers"
fi
if pgrep -f 'podbox-machine-ss[h]' >/dev/null 2>&1; then
  miss "a run process remains"
else
  ok "no run process remains"
fi
if "$PODBOX" system info --format '{{json .Parity}}' 2>/dev/null | grep -q '"machine ssh"'; then
  ok "parity carries the machine ssh rows"
else
  miss "parity misses the machine ssh rows"
fi
fi

if [ "$fail" -eq 0 ]; then
  say "verdict           MACHINE-OK: serial SSH drives a real guest command with its exit code"
else
  say "verdict           MACHINE-FAIL"
fi
cp "$OUT" /out/ 2>/dev/null || true
exit "$fail"
