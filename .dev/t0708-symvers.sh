#!/bin/sh
# Symbol-version probe for T-0708, second pass (ignored, not committed).
# First pass probed the libc.so linker script (a text file) with a pattern
# that needs a trailing space objdump never prints. This one resolves
# libc.so.6 and matches the symbol at end-of-line with its version.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh cc || exit 2
LIBC="$(cc -print-file-name=libc.so.6 2>/dev/null)"
echo "libc: $LIBC"
file "$LIBC" | cut -c1-120
for s in mknod mknod64 mount umount2 unshare clone open open64 __xmknod __xmknod64; do
  echo "== $s"
  objdump -T "$LIBC" 2>&1 | grep -E " ${s}(@|\$)" | head -n 6
done
echo "== probe done"
