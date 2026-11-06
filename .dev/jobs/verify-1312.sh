#!/bin/sh
# Lane job: T-1312 verification. Bootstrap, build both objects with the new
# ceiling assertion, dump the version needs, then fmt/clippy/unit for the
# crate. Streams everything (a redirected death hides).
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== build-interpose (with the T-1312 ceiling)"
./scripts/build-interpose.sh || exit 2
for t in x86_64-unknown-linux-gnu x86_64-unknown-linux-musl; do
  so="crates/podbox-interpose/target/$t/release/libpodbox_interpose.so"
  echo "== $t newest need"
  readelf -VW "$so" | grep -oE 'GLIBC_[0-9.]+' | sort -uV | tail -3
  echo "== $t UND dlsym/gettid versions"
  objdump -T "$so" | grep -E 'dlsym|gettid' || echo "no dlsym/gettid UND"
  echo "== $t dynamic gettid definition"
  objdump -T "$so" | grep -E ' gettid' || echo "gettid not dynamic (local, as intended)"
  echo "== $t UND gettid must be gone (resolved inside the object)"
  readelf -sW "$so" | awk '$7=="UND" && /gettid/ {found=1} END {exit found}' \
    && echo "gettid UND absent" || { echo "FAIL: gettid still undefined"; exit 2; }
  echo "== $t export set equals interpose.map"
  nm -D --defined-only "$so" | awk '$2=="T" {print $3}' | sort -u > /tmp/exp.txt
  grep -E '^[[:space:]]+[A-Za-z_][A-Za-z_0-9]*;$' crates/podbox-interpose/interpose.map | tr -d ' ;' | sort -u > /tmp/map.txt
  comm -23 /tmp/exp.txt /tmp/map.txt > /tmp/extra.txt
  comm -13 /tmp/exp.txt /tmp/map.txt > /tmp/missing.txt
  if [ -s /tmp/extra.txt ] || [ -s /tmp/missing.txt ]; then
    echo "FAIL: export set differs"; cat /tmp/extra.txt /tmp/missing.txt; exit 2
  fi
  echo "exports agree both ways"
done
echo "== fmt"
cargo fmt --manifest-path crates/podbox-interpose/Cargo.toml -- --check || exit 2
echo "== clippy"
RUSTFLAGS='-C target-feature=-crt-static' cargo clippy --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --all-targets -- -D warnings || exit 2
echo "== unit"
RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu || exit 2
echo JOB_DONE
