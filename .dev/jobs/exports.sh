#!/bin/sh
# Lane job: is gettid dynamically exported, and does the export set still
# match interpose.map both ways (105 check A's question)?
set -u
cd /work || exit 2
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
echo "== defined dynamic TEXT symbols not in interpose.map"
nm -D --defined-only "$SO" | awk '$2=="T"' | awk '{print $3}' | sort > /tmp/exp.txt
grep -E '^[a-z_0-9]+;$' crates/podbox-interpose/interpose.map | tr -d ';' | sort > /tmp/map.txt
comm -23 /tmp/exp.txt /tmp/map.txt
echo "== end (empty means the sets agree)"
echo "== gettid entries, all tables"
nm "$SO" | grep -i gettid || echo "no gettid in symtab"
nm -D "$SO" | grep -i gettid || echo "no gettid in dynsym"
echo JOB_DONE
