#!/bin/sh
# Scratch minimal-shim run for T-1309 (ignored, not committed). Builds
# minshim.c (forward-only realpath) and runs it shim-only + default
# mirrors, plus coreutils realpath under the same shim as a second subject.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== build minimal shim, run M (shim-only makecache) and coreutils probe"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1
   gcc -shared -fPIC -o /w/minshim.so /w/minshim.c && echo MIN_BUILD_OK
   echo '-- coreutils under minshim:'; LD_PRELOAD=/w/minshim.so realpath /tmp; echo COREUTILS_RC=\$?
   echo '-- makecache under minshim:'; LD_PRELOAD=/w/minshim.so dnf makecache >/tmp/m.log 2>&1; echo M_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/m.log | head -3" \
  >"$W/v6M.out" 2>&1
echo "M_OUTER_RC=$?"
cat "$W/v6M.out"
