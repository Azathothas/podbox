#!/bin/sh
# Scratch isolation matrix for T-1309 (ignored, not committed). Plain rocky
# containers on host podman, no podbox, no interposer unless named.
# C: bare + fixed URL (the control the entry premise claims passes).
# F: bare + default mirrors (mirror-shape control).
# E: shim-only + default mirrors (is the shim itself the trigger?).
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== run C: bare, fixed URL"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "dnf makecache $FIX >/tmp/c.log 2>&1; echo C_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/c.log | head -3" \
  >"$W/v5C.out" 2>&1
echo "C_OUTER_RC=$?"
cat "$W/v5C.out"
echo "== run F: bare, default mirrors"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "dnf makecache >/tmp/f.log 2>&1; echo F_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/f.log | head -3" \
  >"$W/v5F.out" 2>&1
echo "F_OUTER_RC=$?"
cat "$W/v5F.out"
echo "== run E: shim-only, default mirrors"
: >"$W/rpshim.log"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "LD_PRELOAD=/w/rpshim.so dnf makecache >/tmp/e.log 2>&1; echo E_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/e.log | head -3" \
  >"$W/v5E.out" 2>&1
echo "E_OUTER_RC=$?"
cat "$W/v5E.out"
echo "== shim log after E (first 8 lines)"
head -8 "$W/rpshim.log"
