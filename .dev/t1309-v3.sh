#!/bin/sh
# Scratch v3 probe for T-1309 (ignored, not committed). One rocky container,
# sequential steps: install gcc BARE (control dnf works), build the v3 shim
# from /w/rpshim.c, then four makecache shapes against the fixed URL.
# Evidence lands on /w (host .dev/t1309-w). Never edit while running.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== build the v3 shim with a bare dnf gcc install"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "dnf install -y -q gcc 2>&1 | tail -2; gcc -shared -fPIC -o /w/rpshim.so /w/rpshim.c && echo SHIM_BUILD_OK && ls -la /w/rpshim.so" \
  >"$W/v3build.out" 2>&1
echo "BUILD_RC=$?"
tail -4 "$W/v3build.out"
echo "== run A: shim first, interpose second (measures interpose)"
: >"$W/rpshim.log"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "LD_PRELOAD=/w/rpshim.so:/w/interpose.so dnf makecache $FIX >/tmp/a.log 2>&1; echo A_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/a.log | head -3" \
  >"$W/v3A.out" 2>&1
echo "A_RC=$?"
cat "$W/v3A.out"
echo "== /w/rpshim.log after A"
cat "$W/rpshim.log"
