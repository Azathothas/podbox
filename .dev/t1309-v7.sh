#!/bin/sh
# Scratch bisect runs for T-1309 (ignored, not committed). Builds S1 (ctor)
# and S2 (first-call file logging, no ctor), drives makecache under each.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== build S1 S2, run S1 then S2"
timeout 1200 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1
   gcc -shared -fPIC -o /w/s1.so /w/s1shim.c && gcc -shared -fPIC -o /w/s2.so /w/s2shim.c && echo BISECT_BUILD_OK
   echo '-- S1 (ctor) makecache:'; LD_PRELOAD=/w/s1.so dnf makecache >/tmp/s1.log 2>&1; echo S1_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/s1.log | head -2
   grep -h 's1(' /tmp/s1.log | head -3
   echo '-- S2 (first-call log, no ctor) makecache:'; LD_PRELOAD=/w/s2.so dnf makecache >/tmp/s2.log 2>&1; echo S2_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/s2.log | head -2
   grep -h 's2(' /tmp/s2.log | head -3" \
  >"$W/v7bis.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v7bis.out"
