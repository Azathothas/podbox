#!/bin/sh
# Post-map-fix export check for T-0705 (ignored, not committed): both
# interpose objects must dynamically export getcwd and
# get_current_dir_name after the interpose.map addition.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/build-interpose.sh || exit 1
fail=0
for t in x86_64-unknown-linux-musl x86_64-unknown-linux-gnu; do
  so="crates/podbox-interpose/target/$t/release/libpodbox_interpose.so"
  echo "== $t"
  nm -D "$so" | grep -E " (getcwd|get_current_dir_name)$" || { echo "MISSING export in $t"; fail=1; }
done
[ "$fail" -eq 0 ] && echo "== exports ok"
exit "$fail"
