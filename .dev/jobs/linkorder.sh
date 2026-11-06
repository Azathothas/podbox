#!/bin/sh
# Lane job: stub version state + full gnu link order.
set -u
cd /work || exit 2
echo "== stub defines"
readelf -sW crates/podbox-interpose/target/dl-stub/libdl-stub.so 2>/dev/null | grep -w dlsym || echo "stub not built yet"
readelf -VW crates/podbox-interpose/target/dl-stub/libdl-stub.so 2>/dev/null | grep -A3 "Version definition" | head -8
echo "== rebuild stub verbosely"
STUBDIR=/work/crates/podbox-interpose/target/dl-stub
mkdir -p "$STUBDIR"
cc -shared -nostdlib -Wl,--version-script=/work/crates/podbox-interpose/dl-stub.map -Wl,-soname,libdl.so.2 -o "$STUBDIR/libdl-stub.so" /work/crates/podbox-interpose/dl-stub.S -v 2>&1 | tail -3
echo "== stub defines now"
readelf -sW "$STUBDIR/libdl-stub.so" | grep -w dlsym
echo "== full gnu link order (positions of stub, -lc, objects)"
cd /work/crates/podbox-interpose || exit 2
touch src/real.rs
RUSTFLAGS="-C target-feature=-crt-static -C link-arg=-Wl,--no-as-needed -C link-arg=$STUBDIR/libdl-stub.so -C link-arg=-Wl,--as-needed -C link-arg=-Wl,--version-script=/work/crates/podbox-interpose/interpose.map" \
  cargo build --release --target x86_64-unknown-linux-gnu -v > /tmp/link2.log 2>&1
echo "build_rc=$?"
grep -oE 'libdl-stub\.so[^ "]*|"[^ "]*cc"[^ ]*|[^ ]*\.o"|-lc |-ldl|--no-as-needed|rustc[^ ]*\.rlib' /tmp/link2.log | head -5
echo "== link line positions (character offsets)"
grep -o 'Running.*cc.*-o .*libpodbox.*' /tmp/link2.log | head -1 > /tmp/ccline.txt || true
if [ -s /tmp/ccline.txt ]; then
  for tok in 'libdl-stub' ' -lc ' 'libpodbox_interpose.*\.o' 'libstd-' '--no-as-needed'; do
    printf '%s at: ' "$tok"
    grep -o -b "$tok" /tmp/ccline.txt | head -2 | cut -d: -f1 | tr '\n' ' '
    echo
  done
else
  echo "no cc line found; tail:"
  tail -8 /tmp/link2.log
fi
echo JOB_DONE
