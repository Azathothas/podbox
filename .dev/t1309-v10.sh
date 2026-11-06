#!/bin/sh
# T-1309 void-check (ignored, not committed). Re-runs the V6 shape
# (minimal shim, default mirrors) but KEEPS the shim's own log lines:
# did the wrapper fire, and with what result?
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== V6-shape rerun with full log"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1
   gcc -shared -fPIC -o /w/minshim.so /w/minshim.c && echo MIN_BUILD_OK
   LD_PRELOAD=/w/minshim.so dnf makecache >/tmp/m.log 2>&1; echo M_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/m.log | head -2
   echo '-- shim lines:'; grep -a 'minrealpath' /tmp/m.log | head -5; echo SHIMGREP_RC=\$?" \
  >"$W/v10void.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v10void.out"
