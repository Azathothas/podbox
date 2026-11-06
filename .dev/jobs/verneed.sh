#!/bin/sh
# Lane job: which versioned symbols does the glibc interposer need?
# Prints every NEEDED version above GLIBC_2.27 with the symbols behind it.
set -u
cd /work || exit 2
mkdir -p /out || exit 2
{
echo "== rebuild the glibc object at the pushed head"
./scripts/build-interpose.sh || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
echo "== file"
file "$SO"
echo "== NEEDED"
readelf -dW "$SO" | grep -E 'NEEDED|SONAME'
echo "== version needs above GLIBC_2.27"
readelf -VW "$SO" | grep -oE 'GLIBC_[0-9.]+' | sort -uV
echo "== undefined dynamic symbols with their versions"
readelf --dyn-syms -W "$SO" | awk '$7=="UND" {print $8}' | sort -u | head -60
} > /out/verneed.log 2>&1
rc=$?
tail -40 /out/verneed.log
echo "JOB_RC=$rc"
exit "$rc"
