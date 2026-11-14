#!/bin/sh
# Question: does podvm find the VHDX of each Validation OS ISO with its
# UDF reader, by range requests only, and does the extent that it names
# hold the disk?
#
# TODO/machine.md T-1863. `podvm windows fetch --url URL --extent
# ValidationOS.vhdx` reads the UDF directory of the ISO by HTTP Range
# requests (pod-udf of podcrates v0.45.0) and prints the offset and the
# length of the file. This script then fetches only that range with curl
# and hashes it as it arrives, so the digest does not depend on podvm's
# own download. It starts no emulator and writes no ISO to the disk.
#
# Pinned inputs:
#   9278    the ISO of build 26100.9278 (2 460 880 896 bytes), whose VHDX
#           experiment 371 measured by hand: sector 1010, 910 163 968
#           bytes, sha256 063442aa...fe7f
#   9818    the ISO of build 26100.9818 of the catalog (2 509 025 280
#           bytes)
#   podvm   PODVM_449_BIN (default the release build of the workspace)
#
# Clauses:
#   1. for each ISO, podvm prints an offset and a length, and both are
#      whole sectors of 2048 bytes;
#   2. the first 8 bytes of each extent are `vhdxfile`;
#   3. the extent of 9278 is the one that experiment 371 measured, and its
#      sha256 is the one of experiment 371;
#   4. the sha256 of each extent is printed, for the pin of the catalog.

set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="${PODVM_449_BIN:-$ROOT/target/x86_64-unknown-linux-musl/release/podvm}"
OUT="$ROOT/experiments/results/validationos-udf.txt"
BASE='https://software-static.download.prss.microsoft.com/dbazure'
U9278="$BASE/26100.9278.260824-2119.ge_release_svc_prod3_amd64fre_en-us_VALIDATIONOS.iso"
U9818="$BASE/26100.9818.261025-0959.ge_release_svc_prod3_amd64fre_en-us_VALIDATIONOS.iso"
SHA9278='063442aa9f71f2faeebf49cd960003ce315abd556b52c696e1b994ec5a80fe7f'

fail=0
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
: >"$OUT"
say() { printf '%s\n' "$*" | tee -a "$OUT"; }

for b in 9278 9818; do
	eval "url=\$U$b"
	line=$("$BIN" windows fetch --url "$url" --extent ValidationOS.vhdx)
	rc=$?
	set -- $line
	off=${1:-}; len=${2:-}
	if [ $rc -ne 0 ] || [ -z "$off" ] || [ -z "$len" ]; then
		say "FAIL $b: podvm named no extent (rc $rc)"; fail=1; continue
	fi
	whole=yes
	[ $((off % 2048)) -eq 0 ] && [ $((len % 2048)) -eq 0 ] || whole=no
	last=$((off + len - 1))
	magic=$(curl -qfsS -r "$off-$((off + 7))" "$url" | head -c 8)
	# A stream that breaks gives a wrong digest: the status of curl is
	# read from a file, since a pipe of sh hides it, and a break is tried
	# again, three times at most.
	sha=; try=0
	while [ -z "$sha" ] && [ $try -lt 3 ]; do
		try=$((try + 1))
		got=$({ curl -qfsS -r "$off-$last" "$url"; echo $? >"$TMP/rc"; } | sha256sum | cut -c1-64)
		[ "$(cat "$TMP/rc")" = 0 ] && sha=$got
	done
	[ -n "$sha" ] || { say "FAIL $b: the extent did not arrive whole in 3 tries"; fail=1; continue; }
	say "build $b: offset $off (sector $((off / 2048))), length $len, whole sectors $whole, magic $magic, sha256 $sha"
	[ "$whole" = yes ] || { say "FAIL $b: clause 1"; fail=1; }
	[ "$magic" = vhdxfile ] || { say "FAIL $b: clause 2"; fail=1; }
	if [ $b = 9278 ]; then
		[ "$off" = 2068480 ] && [ "$len" = 910163968 ] && [ "$sha" = "$SHA9278" ] \
			|| { say "FAIL 9278: clause 3, the extent of experiment 371 differs"; fail=1; }
	fi
done
[ $fail -eq 0 ] && say "PASS: each clause" || say "FAIL"
exit $fail
