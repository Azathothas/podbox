#!/bin/sh
# T-1309 dispatch proof v12 (ignored, not committed). Builds v12shim.c and
# drives fixed-baseos makecache under it, keeping the v12 comparison lines.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== v12 dispatch proof"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1
   gcc -shared -fPIC -o /w/v12.so /w/v12shim.c && echo V12_BUILD_OK
   rm -rf /var/cache/dnf
   LD_PRELOAD=/w/v12.so dnf makecache $FIX >/tmp/v12.log 2>&1; echo V12_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/v12.log | head -2
   echo '-- v12 lines:'; grep -a 'v12(' /tmp/v12.log | head -4" \
  >"$W/v12.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v12.out"
