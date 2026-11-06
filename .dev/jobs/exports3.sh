#!/bin/sh
# Lane job: gettid must be neither undefined nor dynamic; the dynamic export
# set must equal interpose.map (105 check A's question, asked directly).
set -u
cd /work || exit 2
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
echo "== UND gettid/dlsym"
readelf -sW "$SO" | awk '$7=="UND" && (/gettid/ || /dlsym/)' || echo "neither undefined"
echo "== dynamic gettid"
nm -D "$SO" | grep -w gettid || echo "not dynamic"
echo "== export set vs map"
nm -D --defined-only "$SO" | awk '$2=="T" {print $3}' | sort -u > /tmp/exp.txt
awk '{for(i=1;i<=NF;i++) if($i~/^[A-Za-z_0-9]+;$/) {s=$i; sub(/;$/,"",s); print s}}' crates/podbox-interpose/interpose.map | sort -u > /tmp/map.txt
echo "--- in object, not in map:"
comm -23 /tmp/exp.txt /tmp/map.txt
echo "--- in map, not in object:"
comm -13 /tmp/exp.txt /tmp/map.txt
echo "== newest need"
readelf -VW "$SO" | grep -oE 'GLIBC_[0-9.]+' | sort -uV | tail -1
echo JOB_DONE
