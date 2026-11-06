#!/bin/sh
# Scratch v3 control for T-1309 (ignored, not committed). Shim ONLY, no
# interpose: the shim's dlsym(RTLD_NEXT) now reaches libc directly, so the
# log shows what libc itself answers for the same two paths.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== run B: shim only (measures libc directly)"
: >"$W/rpshim.log"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "LD_PRELOAD=/w/rpshim.so dnf makecache $FIX >/tmp/b.log 2>&1; echo B_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/b.log | head -3" \
  >"$W/v3B.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v3B.out"
echo "== /w/rpshim.log after B"
cat "$W/rpshim.log"
