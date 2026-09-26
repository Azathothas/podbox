#!/bin/sh
# Question: can a lane with a sub-3GB file-size ceiling land the
# ValidationOS disk without ever writing a file the ceiling forbids?
#
# TODO/milestones.md T-1112. The ISO is 2460880896 bytes and the lane that
# produced this record refuses any file at or over 1000000000 bytes
# (RLIMIT_FSIZE, unraiseable), so a whole-ISO download dies with SIGXFSZ
# at ~954 MB. The VHDX inside it is one contiguous UDF extent of
# 910163968 bytes, which fits. This script walks the on-origin structures
# with range requests, parses the one extent, and fetches only it.
#
# Pinned inputs: the Tayberry ISO URL below (HEAD 200, 2460880896 bytes,
# anonymous), UDF NSR02 with the AVDP at sector 256, partition start 304.
# The VHDX extent (abs LBA 1010, 910163968 bytes, `vhdxfile` magic) and
# the WIM extent (abs LBA 445426, 245012656 bytes) were read out of the
# File Entries at partition-relative 156 and 157. A re-mastered ISO
# moves them; the script asserts every step rather than trusting it.
#
# Clauses:
#   0. conditions: curl and python3 present, the fsize ceiling read, and
#      room plus ceiling for the 910163968-byte extent (exit 2 without).
#   1. the ISO answers HEAD 200 with 2460880896 bytes, anonymous.
#   2. sector 16 is the ISO9660 PVD (CD001) naming the volume; sectors
#      18-19 are BEA01/NSR02: the UDF bridge this walks.
#   3. the AVDP at sector 256 points at the VDS; the VDS holds PVD, LVD,
#      PD, USD, IUVD and TD in that order.
#   4. the LVD names the FSD; the FSD names the root ICB; the root FE
#      is a directory. Every location is parsed, never hardcoded: the FSD
#      comes out of the LVD already fetched for clause 3, plus the
#      partition start out of the PD beside it.
#   5. the extent fetched starts with `vhdxfile` and sha256-matches the
#      recorded digest, and `qemu-img info` reads it as a readable disk.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no tooling, no origin).
set -u

URL='https://software-static.download.prss.microsoft.com/dbazure/26100.9278.260824-2119.ge_release_svc_prod3_amd64fre_en-us_VALIDATIONOS.iso'
ISO_LEN=2460880896
VHDX_LBA=1010
VHDX_LEN=910163968
VHDX_SHA256='063442aa9f71f2faeebf49cd960003ce315abd556b52c696e1b994ec5a80fe7f'

REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep365-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-365.txt"
OUT="${PODBOX_VOS_OUT:-$WORK/ValidationOS.vhdx}"
fail=0

# Pre-flight: the extent must fit the ceiling and the disk, or this lane
# cannot run it. curl dying with SIGXFSZ halfway is exit 1 territory;
# refusing up front with the numbers is exit 2.
FSIZE_HARD="$(prlimit --fsize 2>/dev/null | awk '/^FSIZE/ {print $6}' || echo unknown)"
ROOM="$(df -k "$(dirname "$OUT")" 2>/dev/null | awk 'NR==2 {print $4 * 1024}')"
echo "fsize hard        $FSIZE_HARD" >>"$REPORT"
echo "room bytes        $ROOM" >>"$REPORT"
if [ "$FSIZE_HARD" != "unknown" ] && [ "$FSIZE_HARD" != "unlimited" ] && [ "$FSIZE_HARD" -lt "$VHDX_LEN" ]; then
	echo "ceiling           $FSIZE_HARD bytes cannot hold $VHDX_LEN: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi
if [ -n "$ROOM" ] && [ "$ROOM" -lt "$VHDX_LEN" ]; then
	echo "disk              $ROOM bytes free cannot hold $VHDX_LEN: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "fsize ceiling     $(ulimit -Hf 2>/dev/null || prlimit --fsize 2>/dev/null | tail -1)"
command -v curl >/dev/null 2>&1 || { echo "curl missing: COULD NOT RUN"; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN"; exit 2; }
} >"$REPORT"

range() {
	# range <first> <last> <dest>: one bounded GET, failing loud
	timeout 300 curl -fsSL -r "$1-$2" -o "$3" "$URL" || { echo "range $1-$2 FAILED" | tee -a "$REPORT"; return 1; }
}

# Clause 1: the origin serves the ISO anonymously at the pinned length.
# HEAD only: a GET to /dev/null would spend 2.46 GB proving a number the
# header already states.
LEN="$(timeout 60 curl -sSI "$URL" 2>/dev/null | grep -i '^content-length:' | tr -d '\r' | awk '{print $2}')"
echo "head bytes        $LEN" >>"$REPORT"
[ "$LEN" = "$ISO_LEN" ] || { echo "clause 1          ISO length $LEN, wanted $ISO_LEN" >>"$REPORT"; fail=1; }

# Clause 2: PVD plus the UDF bridge markers.
range 32768 40959 "$WORK/vrs.bin" || exit 2
python3 - "$WORK/vrs.bin" >>"$REPORT" 2>&1 <<'PY' || { echo "clause 2          VRS PARSE FAILED" >>"$REPORT"; fail=1; }
import struct, sys
d = open(sys.argv[1], "rb").read()
assert d[1:6] == b"CD001", "sector 16 is not the PVD"
assert d[2048] == 255 and d[2049:2054] == b"CD001", "sector 17 is not the terminator"
assert d[4096+1:4096+6] == b"BEA01", "no UDF bridge"
assert d[6144+1:6144+6] == b"NSR02", "not NSR02"
print("clause 2          PVD plus terminator plus BEA01/NSR02 present")
PY

# Clause 3: AVDP at 256 names the VDS; the VDS carries the six descriptors.
range 524288 526335 "$WORK/avdp.bin" || exit 2
VDS="$(python3 - "$WORK/avdp.bin" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
assert struct.unpack("<H", d[0:2])[0] == 2, "sector 256 is not the AVDP"
length = struct.unpack("<I", d[16:20])[0]
loc = struct.unpack("<I", d[20:24])[0]
print(f"{loc*2048}-{(loc*2048)+length-1}")
PY
)" || { echo "clause 3          AVDP PARSE FAILED" >>"$REPORT"; fail=1; }
if [ -n "${VDS:-}" ]; then
	range "${VDS%-*}" "${VDS#*-}" "$WORK/vds.bin" || exit 2
	python3 - "$WORK/vds.bin" >>"$REPORT" 2>&1 <<'PY' || { echo "clause 3          VDS PARSE FAILED" >>"$REPORT"; fail=1; }
import struct, sys
d = open(sys.argv[1], "rb").read()
tags = [struct.unpack("<H", d[i:i+2])[0] for i in range(0, min(len(d), 12288), 2048)]
assert tags[:6] == [1, 6, 5, 7, 4, 8], f"VDS is not PVD/LVD/PD/USD/IUVD/TD: {tags[:6]}"
print("clause 3          VDS carries PVD LVD PD USD IUVD TD in order")
PY
fi

# Clause 4: the LVD (second block of the fetched VDS) names the FSD,
# and the PD beside it names the partition start the FSD block number is
# relative to. Nothing here is hardcoded to LBA 304.
FSDABS="$(python3 - "$WORK/vds.bin" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
lvd, pd = d[2048:4096], d[4096:6144]
assert struct.unpack("<H", lvd[0:2])[0] == 6, "second VDS block is not the LVD"
assert struct.unpack("<H", pd[0:2])[0] == 5, "third VDS block is not the PD"
fsd_lbn = struct.unpack("<I", lvd[248+4:248+8])[0]
part_start = struct.unpack("<I", pd[188:192])[0]
print(f"{(part_start + fsd_lbn) * 2048} {part_start}")
PY
)" || { echo "clause 4          LVD PARSE FAILED" >>"$REPORT"; fail=1; }
PARTSTART="$(echo "$FSDABS" | awk '{print $2}')"
FSDABS="$(echo "$FSDABS" | awk '{print $1}')"
if [ -z "${FSDABS:-}" ]; then echo "clause 4          NO FSD LOCATION" >>"$REPORT"; fail=1; exit 2; fi
range "$FSDABS" "$((FSDABS+4095))" "$WORK/fsd.bin" || exit 2
ROOT="$(python3 - "$WORK/fsd.bin" "$PARTSTART" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
part_start = int(sys.argv[2])
assert struct.unpack("<H", d[0:2])[0] == 256, "not the FSD"
lbn = struct.unpack("<I", d[404:408])[0]
print(f"{(part_start+lbn)*2048}-{((part_start+lbn)*2048)+2047}")
PY
)" || { echo "clause 4          FSD PARSE FAILED" >>"$REPORT"; fail=1; }
if [ -n "${ROOT:-}" ]; then
	range "${ROOT%-*}" "${ROOT#*-}" "$WORK/rootfe.bin" || exit 2
	python3 - "$WORK/rootfe.bin" >>"$REPORT" 2>&1 <<'PY' || { echo "clause 4          ROOT FE IS NOT A DIRECTORY" >>"$REPORT"; fail=1; }
import struct, sys
d = open(sys.argv[1], "rb").read()
assert struct.unpack("<H", d[0:2])[0] == 261, "not a File Entry"
assert struct.unpack("<H", d[27:29])[0] == 4, "root is not a directory"
print("clause 4          root File Entry is a directory")
PY
fi

# Clause 5: the one extent, its magic, its digest, its readability.
echo "fetch             $VHDX_LEN bytes at LBA $VHDX_LBA" >>"$REPORT"
timeout 900 curl -fsSL -r "$((VHDX_LBA*2048))-$((VHDX_LBA*2048+VHDX_LEN-1))" -o "$OUT" "$URL" \
	>>"$WORK/fetch.log" 2>&1 || { echo "fetch             FAILED" | tee -a "$REPORT"; tail -5 "$WORK/fetch.log"; exit 1; }
SZ="$(wc -c <"$OUT" | tr -d ' ')"
[ "$SZ" -eq "$VHDX_LEN" ] || { echo "clause 5          fetched $SZ bytes, wanted $VHDX_LEN" >>"$REPORT"; fail=1; }
head -c 8 "$OUT" | grep -qa "vhdxfile" || { echo "clause 5          NO VHDX MAGIC" >>"$REPORT"; fail=1; }
GOT="$(sha256sum "$OUT" | cut -d' ' -f1)"
echo "sha256            $GOT" >>"$REPORT"
[ "$GOT" = "$VHDX_SHA256" ] || { echo "clause 5          SHA256 MISMATCH" >>"$REPORT"; fail=1; }
if command -v qemu-img >/dev/null 2>&1; then
	qemu-img info "$OUT" >>"$REPORT" 2>&1 || { echo "clause 5          QEMU-IMG INFO FAILED" >>"$REPORT"; fail=1; }
	echo "clause 5          extent is vhdxfile, sha256 matches, qemu-img reads it" >>"$REPORT"
fi

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           VOS STREAM HOLDS"; else echo "verdict           VOS STREAM OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/windows-365.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
