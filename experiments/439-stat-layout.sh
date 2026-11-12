#!/bin/sh
# Question: where are the fields of `struct stat`, `struct stat64` and
# `struct statx` on aarch64 under glibc and musl, against x86_64?
#
# TODO/interpose.md T-1721 (the interposer for aarch64), as experiment 105
# measured x86_64. A C program prints offsetof and sizeof of each field
# that the interposer reads or writes; zig cc builds it for each target,
# and qemu-aarch64-static runs the aarch64 ones (glibc with the loader of
# an arm64 debian image as its sysroot).
#
# Inputs: zig (PODVM_ZIG, else zig on PATH), qemu-aarch64-static, and an
# arm64 rootfs for glibc (PODVM_439_SYSROOT; podvm extract --platform
# linux/arm64 debian:bookworm-slim gives one).
#
# Clauses:
#   1. each of the four programs runs and prints its table.
#   2. the two aarch64 tables agree with each other (the generic layout).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep439-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/stat-layout.txt"
ZIG="${PODVM_ZIG:-zig}"
SYSROOT="${PODVM_439_SYSROOT:-}"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
trap 'rm -rf "$WORK"' EXIT INT TERM

command -v "$ZIG" >/dev/null 2>&1 || { echo "zig missing: COULD NOT RUN" >&2; exit 2; }
command -v qemu-aarch64-static >/dev/null 2>&1 || { echo "qemu-aarch64-static missing: COULD NOT RUN" >&2; exit 2; }
[ -n "$SYSROOT" ] && [ -e "$SYSROOT/lib/ld-linux-aarch64.so.1" ] || { echo "PODVM_439_SYSROOT names no arm64 glibc rootfs: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/off.c" <<'C'
#define _GNU_SOURCE
#include <stdio.h>
#include <stddef.h>
#include <sys/stat.h>
#include <fcntl.h>
#define P(T,f) printf(#T "." #f " %zu %zu\n", offsetof(struct T,f), sizeof(((struct T*)0)->f))
int main(void){
 printf("stat size %zu\n", sizeof(struct stat));
 P(stat,st_dev);P(stat,st_ino);P(stat,st_mode);P(stat,st_nlink);P(stat,st_uid);P(stat,st_gid);P(stat,st_rdev);P(stat,st_size);
 printf("statx size %zu\n", sizeof(struct statx));
 P(statx,stx_mode);P(statx,stx_uid);P(statx,stx_gid);P(statx,stx_ino);P(statx,stx_rdev_major);P(statx,stx_dev_major);
 return 0;}
C

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "zig               $("$ZIG" version 2>/dev/null)"
note "qemu              $(qemu-aarch64-static --version | head -1)"
note ""
for t in x86_64-linux-gnu x86_64-linux-musl aarch64-linux-gnu aarch64-linux-musl; do
	note "== $t"
	"$ZIG" cc -target "$t" -o "$WORK/off-$t" "$WORK/off.c" 2>"$WORK/cc.err" || { bad "$t does not build"; continue; }
	case $t in
		x86_64-*) "$WORK/off-$t" >"$WORK/$t.txt" ;;
		aarch64-linux-gnu) qemu-aarch64-static -L "$SYSROOT" "$WORK/off-$t" >"$WORK/$t.txt" ;;
		*) qemu-aarch64-static "$WORK/off-$t" >"$WORK/$t.txt" ;;
	esac || bad "$t does not run"
	sed 's/^/  /' "$WORK/$t.txt" | tee -a "$OUT"
done
note ""
cmp -s "$WORK/aarch64-linux-gnu.txt" "$WORK/aarch64-linux-musl.txt" || bad "glibc and musl disagree on aarch64"

if [ "$fail" -eq 0 ]; then
	note "verdict: THE LAYOUTS ARE ABOVE; AARCH64 HAS ONE LAYOUT UNDER BOTH LIBCS"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
