#!/usr/bin/env bash
# Question: none. This is a build step, not a measurement.
#
# Builds crates/podbox-interpose as an LD_PRELOAD cdylib, once per libc.
#
# ⛔ RUSTFLAGS, NOT A CRATE-LOCAL .cargo/config.toml. Cargo MERGES config files
# up the directory tree and appends the parent's rustflags AFTER the child's,
# so a crate-local `-C target-feature=-crt-static` is followed by the root's
# `+crt-static` and loses. The RUSTFLAGS environment variable replaces the
# config value outright, which is why this is a script and not a config file.
# Measured by experiments/60-interposer-libc.sh.
#
# ⛔ ONE OBJECT PER LIBC. A preloaded object is loaded by the payload's own
# dynamic loader and resolves its own imports against the payload's libc, so a
# musl-linked object cannot be preloaded into a glibc payload. podbox embeds
# both and selects on the payload's PT_INTERP (TOOL.md section 6.7, TODO/interpose.md
# T-0702).
#
# Exit: 0 every requested target built, 1 a build failed, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
CRATE="$ROOT/crates/podbox-interpose"

command -v cargo >/dev/null 2>&1 || { echo "SKIP: cargo not on PATH" >&2; exit 2; }
[ -d "$CRATE" ] || { echo "SKIP: $CRATE is absent" >&2; exit 2; }

TARGETS="${TARGETS:-x86_64-unknown-linux-musl x86_64-unknown-linux-gnu}"
# ⛔ **THE MUSL TARGET NEEDS `zig cc` AS THE LINKER, AND WITHOUT IT THIS SCRIPT
# PRODUCES TWO GLIBC OBJECTS.** Measured on 2026-09-09: with the default linker
# the `x86_64-unknown-linux-musl` object records `libc.so.6`, `libgcc_s.so.1`
# and `ld-linux-x86-64.so.2` in DT_NEEDED, which is byte-for-byte the wrong
# answer to "one object per libc" and reads as success because the build exits 0.
# `rustc` passes `-lgcc_s` on the musl target even under `panic = "abort"`, and
# `musl-tools` ships no musl-linked `libgcc_s.so.1`, so the host `cc` links it
# against the host's glibc. `zig cc` carries its own `compiler-rt`.
# `scripts/zig-cc.sh` records the whole determination.
ZIG_LINKER="$ROOT/scripts/zig-cc.sh"

# What DT_NEEDED must name, per target. ⛔ Asserted rather than assumed: the
# failure above is silent, and a build that exits 0 having produced the wrong
# object is the shape TODO/interpose.md T-0702 exists to prevent.
want_libc() {
  case "$1" in
  *-musl | *-musl*) printf 'libc.so' ;;
  *) printf 'libc.so.6' ;;
  esac
}

rc=0
for t in $TARGETS; do
  if ! rustup target list --installed 2>/dev/null | grep -qx "$t"; then
    printf 'SKIP %s: target not installed\n' "$t" >&2
    continue
  fi
  printf '== %s\n' "$t"
  link=""
  case "$t" in
  *-musl | *-musl*)
    if command -v zig >/dev/null 2>&1 && [ -x "$ZIG_LINKER" ]; then
      link="-C linker=$ZIG_LINKER"
      printf '   linker: scripts/zig-cc.sh (zig %s)\n' "$(zig version 2>/dev/null)"
    else
      printf 'SKIP %s: no zig, and the default linker produces a GLIBC object\n' "$t" >&2
      printf '      ./scripts/common/bootstrap-env.sh zig\n' >&2
      rc=2
      continue
    fi
    ;;
  *-gnu*)
    # ⛔ THE `libdl` IMPORT LIBRARY, T-1312's second half. The `dlsym`
    # reference must bind the LIBRARY that defines it on old systems, and
    # on a modern host that is not `libc.so.6`: before the 2.34 merge
    # `dlsym` lived in `libdl`, and the host's `libdl.so` is a linker
    # script into `libc`, so a plain `-ldl` records `libc.so.6` (or vanishes
    # under `--as-needed`) and the old loader answers `symbol dlsym version
    # GLIBC_2.2.5 not defined in file libc.so.6`, measured in the 152
    # REGISTER row on 2026-09-21. `dl-stub.S` carries `SONAME libdl.so.2`
    # with no implementation, so the reference binds `libdl.so.2`, which
    # glibc keeps as a compatibility object on both sides of the merge.
    # musl has no `libdl` (its libc serves `dlsym` directly), so this stays
    # on the glibc arm the way `zig cc` stays on the musl one.
    stub_dir="$CRATE/target/dl-stub"
    mkdir -p "$stub_dir" || { printf 'SKIP %s: no stub directory\n' "$t" >&2; rc=2; continue; }
    stub="$stub_dir/libdl-stub.so"
    if cc -shared -nostdlib -Wl,--version-script="$CRATE/dl-stub.map" -Wl,-soname,libdl.so.2 -o "$stub" "$CRATE/dl-stub.S"; then
      printf '   stub: %s (SONAME %s)\n' "$stub" \
        "$(readelf -dW "$stub" 2>/dev/null | awk -F'[][]' '/SONAME/{print $2}')"
    else
      printf 'FAIL %s: the libdl stub did not build\n' "$t" >&2
      rc=1
      continue
    fi
    # ⛔ PREPENDED THROUGH A LINKER WRAPPER, not `-C link-arg`. User link
    # arguments travel after the objects, so by the time the stub (or a
    # `-ldl`) is seen the `dlsym` reference is already bound to `libc.so.6`;
    # `--as-needed` drops the latecomer outright. First on the line the
    # stub is both kept and searched first. `PODBOX_DL_STUB` carries the
    # path because a `-C linker` takes a program, not a command line, which
    # is the same reason `zig-cc.sh` is a script.
    link="-C linker=$ROOT/scripts/gnu-link-stub.sh"
    export PODBOX_DL_STUB="$stub"
    ;;
  esac
  so="$CRATE/target/$t/release/libpodbox_interpose.so"
  # ⛔ THE VERSION SCRIPT, T-0701 constraint 1. Without it a Rust cdylib exports
  # `rust_eh_personality` and the rest of its runtime into every process it is
  # loaded into, and this object is loaded into every process of a container.
  # ⚠ An ABSOLUTE path: the build runs with `$CRATE` as its working directory
  # and the linker is invoked from somewhere else again.
  vs="-C link-arg=-Wl,--version-script=$CRATE/interpose.map"
  if ( cd "$CRATE" && RUSTFLAGS="-C target-feature=-crt-static $link $vs" \
        cargo build --release --target "$t" ); then
    ls -l "$so"
    needed="$(readelf -dW "$so" 2>/dev/null | awk -F'[][]' '/NEEDED/{print $2}' | tr '\n' ' ')"
    printf '   DT_NEEDED: %s\n' "$needed"
    want="$(want_libc "$t")"
    # ⚠ An exact word. `libc.so.6` CONTAINS `libc.so`, so a substring test would
    # call a glibc object musl-linked, which is the very mistake being caught.
    case " $needed " in
    *" $want "*) printf '   ok: it names %s, so it is %s-linked\n' "$want" \
                   "$(case "$t" in *musl*) echo musl ;; *) echo glibc ;; esac)" ;;
    *) printf 'FAIL %s: DT_NEEDED is [%s] and must name %s\n' "$t" "$needed" "$want" >&2
       rc=1 ;;
    esac
    # ⛔ THE GLIBC CEILING, TODO/interpose.md T-1312. The object loads into
    # payloads back to glibc 2.27 (the nix closure's version, measured
    # 2026-09-21), so no needed version above it may enter. A new import the
    # build host stamps higher fails here with the version named, rather
    # than refusing somebody's container at startup.
    ceiling="GLIBC_2.27"
    newest="$(readelf -VW "$so" 2>/dev/null | grep -oE 'GLIBC_[0-9.]+' | sort -uV | tail -1)"
    if [ -n "$newest" ]; then
      hi="$(printf '%s\n%s\n' "$newest" "$ceiling" | sort -uV | tail -1)"
      if [ "$hi" != "$ceiling" ]; then
        printf 'FAIL %s: needs %s, above the %s ceiling\n' "$t" "$newest" "$ceiling" >&2
        rc=1
      else
        printf '   ok: newest need %s, within %s\n' "$newest" "$ceiling"
      fi
    else
      printf '   ok: no versioned needs\n'
    fi
    # ⛔ `gettid` MUST NOT LEAVE, T-1312. Rust std calls the libc wrapper,
    # which glibc only provides from 2.30, so the crate defines the number
    # itself as a bare assembly label: local binding by default, no dynamic
    # entry, no payload-visible export. The absence is asserted, not
    # assumed: a leaked `gettid` is a symbol some other library in the
    # payload's process resolves to podbox, and `105` check A compares the
    # export set both ways. Uppercase only: a localized entry stays in
    # `.dynsym` as `t`, which the loader binds inside the object and no
    # payload resolves.
    if nm -D "$so" 2>/dev/null | awk '$3=="gettid" && $2 ~ /^[A-Z]$/ {found=1} END {exit !found}'; then
      printf 'FAIL %s: gettid is dynamically exported\n' "$t" >&2
      rc=1
    else
      printf '   ok: gettid is not dynamically exported\n'
    fi
    # ⛔ `libdl` MUST BE NEEDED on glibc, T-1312. The pinned `dlsym` binds
    # whichever library the link names, and `libc.so.6` never defined that
    # version; `--as-needed` once dropped the flag silently, so the need is
    # asserted the same way the libc name is. The SONAME is `libdl.so.2`:
    # the `2` is that library's own version and has nothing to do with the
    # `libc.so.6` beside it.
    case "$t" in
    *-gnu*)
      case " $needed " in
      *" libdl.so.2 "*) printf '   ok: libdl.so.2 is needed\n' ;;
      *) printf 'FAIL %s: DT_NEEDED is [%s] and names no libdl\n' "$t" "$needed" >&2
         rc=1 ;;
      esac
      ;;
    esac
  else
    printf 'FAIL %s\n' "$t" >&2
    rc=1
  fi
done
exit "$rc"
