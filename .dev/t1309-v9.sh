#!/bin/sh
# De-confounding for T-1309 (ignored, not committed). ONE container,
# alternating minimal / S3 / minimal / S3 against baseos-fixed, plus the
# dynamic-symbol nature of realpath. If minimal passes 2x and S3 fails 2x
# the bisect stands; any mix means intermittent infrastructure.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== alternate minimal/S3 in one container, baseos-fixed"
timeout 1500 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc binutils 2>&1 | tail -1
   gcc -shared -fPIC -o /w/minshim.so /w/minshim.c && gcc -shared -fPIC -o /w/s3.so /w/s3shim.c && echo ALT_BUILD_OK
   nm -D /lib64/libc.so.6 | grep -E ' realpath|__realpath' | head -5
   for shape in min s3 min s3; do
     if [ \"\$shape\" = min ]; then SO=/w/minshim.so; else SO=/w/s3.so; fi
     echo \"-- shape \$shape (\$SO):\"; rm -rf /var/cache/dnf
     LD_PRELOAD=\$SO dnf makecache $FIX >/tmp/\${shape}.log 2>&1
     echo \${shape}_RC=\$?; grep -E 'Metadata|Error|Invalid' /tmp/\${shape}.log | head -1
     grep -hE '^(minrealpath|s3)\\(pid' /tmp/\${shape}.log | head -2
   done" \
  >"$W/v9alt.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v9alt.out"
