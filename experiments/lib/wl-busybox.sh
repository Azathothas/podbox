#!/bin/sh
# One workload, named by $1, in a fresh directory under $WL_DIR (default
# /tmp) of whatever sandbox this runs in. Prints `inner_ms=N` only when the
# workload succeeded. Busybox only, so each engine runs the same image with
# no package installed in it (experiment 433, TODO/machine.md T-1717). The
# shapes follow `research/experiments/workload.sh` of Azathothas/errand-box
# at 1ad2fc1, without python3, git and gcc:
#   spawn   300 fork+exec of /bin/true
#   files   write, read back and stat 3000 files of 512 bytes
#   cpu     a 3e6-iteration awk loop
# The clock is /proc/uptime (10 ms steps): busybox date has no %N.
set -u
d="${WL_DIR:-/tmp}/wl.$$"; rm -rf "$d"; mkdir -p "$d"; cd "$d" || exit 1
now() { read -r up _ </proc/uptime; echo "${up%.*}${up#*.}"; }
t0=$(now)
(
set -e
case "$1" in
  spawn) i=0; while [ $i -lt 300 ]; do /bin/true; i=$((i+1)); done ;;
  files) b=$(printf '%512s' x)
         i=0; while [ $i -lt 3000 ]; do printf '%s' "$b" > f$i; i=$((i+1)); done
         n=$(cat f* | wc -c); [ "$n" -eq $((3000 * 512)) ]
         i=0; while [ $i -lt 3000 ]; do [ -f f$i ]; i=$((i+1)); done ;;
  cpu) awk 'BEGIN { s = 0; for (i = 0; i < 3000000; i++) s += i * i; if (s <= 0) exit 1 }' ;;
  *) echo "unknown workload $1" >&2; exit 2 ;;
esac
) >"$d.err" 2>&1
code=$?
t1=$(now)
cd / && rm -rf "$d"
if [ $code -eq 0 ]; then echo "inner_ms=$(( (t1 - t0) * 10 ))"; else echo "workload $1 failed: $(head -3 "$d.err")" >&2; fi
rm -f "$d.err"
exit $code
