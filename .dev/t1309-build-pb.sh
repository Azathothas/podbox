#!/bin/sh
# Lane build for the prove (ignored, not committed). Builds BOTH interpose
# objects first (build.rs only embeds what this leaves behind; placeholders
# ride silent and every payload runs bare), then the musl release binary,
# then asserts the binary carries three ELFs (itself plus both objects)
# before it rides home through /out.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== interpose objects first"
./scripts/build-interpose.sh || exit 1
echo "== musl release build"
./scripts/dev.sh build || exit 1
BIN="target/x86_64-unknown-linux-musl/release/podbox"
[ -x "$BIN" ] || { echo "BUILD_MISSING: no $BIN"; exit 1; }
"$BIN" version 2>&1 | head -2
ELFS=$(python3 -c "d=open('$BIN','rb').read();print(d.count(b'\x7fELF'))" 2>/dev/null || echo 0)
echo "ELF_MAGICS=$ELFS"
[ "$ELFS" -ge 3 ] || { echo "EMBED_MISSING: binary carries $ELFS ELF objects, want 3"; exit 1; }
cp "$BIN" /out/podbox || exit 1
ls -la /out/
echo "== build job done"
