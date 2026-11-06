#!/bin/sh
# Scratch v4 probe for T-1309 (ignored, not committed). Rebuilds the shim
# from /w/rpshim.c (v4: dladdr + stat logging), then run A2 (shim+interpose)
# and run B2 (shim only, libc direct). Evidence on /w.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== build v4 shim (bare dnf gcc already installed? install if missing)"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1; gcc -shared -fPIC -o /w/rpshim.so /w/rpshim.c && echo SHIM_BUILD_OK" \
  >"$W/v4build.out" 2>&1
echo "BUILD_OUTER_RC=$?"
tail -2 "$W/v4build.out"
echo "== run A2: shim+interpose"
: >"$W/rpshim.log"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "LD_PRELOAD=/w/rpshim.so:/w/interpose.so dnf makecache $FIX >/tmp/a.log 2>&1; echo A_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/a.log | head -3" \
  >"$W/v4A.out" 2>&1
echo "A_OUTER_RC=$?"
cat "$W/v4A.out"
echo "== log after A2"
cat "$W/rpshim.log"
echo "== run B2: shim only"
: >"$W/rpshim.log"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "LD_PRELOAD=/w/rpshim.so dnf makecache $FIX >/tmp/b.log 2>&1; echo B_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/b.log | head -3" \
  >"$W/v4B.out" 2>&1
echo "B_OUTER_RC=$?"
cat "$W/v4B.out"
echo "== log after B2"
cat "$W/rpshim.log"
