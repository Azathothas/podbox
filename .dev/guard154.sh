#!/bin/sh
# Scratch guard-mutation for 154 section 4: functions copied verbatim from
# experiments/154-tcg-workload-spread.sh, synthetic logs, assert pass/refuse.
# Not committed; .dev is ignored.
set -u
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT

elap() {
	n=$(grep -c "^workload=$1 " "$2" 2>/dev/null); n=${n:-0}
	[ "$n" -eq 3 ] || return 0
	grep -h "^workload=$1 " "$2" 2>/dev/null | sed -E 's/.* elapsed=([0-9.]+).*/\1/' | sort -n | sed -n '2p'
}
sums() {
	grep -h "^workload=$1 " "$2" 2>/dev/null | sed -E 's/.* checksum=([0-9a-f]+).*/\1/' | sort -u
}
ratio() {
	awk -v a="$1" -v b="$2" 'BEGIN { if (b > 0 && a > 0) printf "%.1f", a/b; else printf "-" }'
}
# row LOGW: replicate the section-4 body for class w across three platform logs.
row() {
	w="$1"; shift
	he="$(elap "$w" "$1")"; ce="$(elap "$w" "$2")"; ge="$(elap "$w" "$3")"
	hc="$(sums "$w" "$1" | tr '\n' ' ')"; cc="$(sums "$w" "$2" | tr '\n' ' ')"
	gc="$(sums "$w" "$3" | tr '\n' ' ')"
	all="$(printf '%s\n%s\n%s\n' "$hc" "$cc" "$gc" | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ')"
	n=$(printf '%s' "$all" | wc -w)
	if [ "$n" -eq 1 ] && [ -n "$he" ] && [ -n "$ce" ] && [ -n "$ge" ]; then
		echo "PASS $w elap=$he/$ce/$ge sums=$all"
	else
		echo "REFUSE $w host=[$he/$hc] chroot=[$ce/$cc] guest=[$ge/$gc]"
	fi
}
mk() { # mk NAME LINE [LINE...]: write lines to $T/NAME
	name="$1"; shift
	printf '%s\n' "$@" >"$T/$name"
}
good='workload=int iters=100 mops=1.0 checksum=aaaa acc=1.0e+00 elapsed=0.150'
good2='workload=int iters=100 mops=1.0 checksum=aaaa acc=1.0e+00 elapsed=0.160'
good3='workload=int iters=100 mops=1.0 checksum=aaaa acc=1.0e+00 elapsed=0.140'
bad='workload=int iters=100 mops=1.0 checksum=bbbb acc=1.0e+00 elapsed=0.150'
err='workload=io error=open-write'

mk h1 "$good" "$good2" "$good3"
mk c1 "$good" "$good2" "$good3"
mk g1 "$good" "$good2" "$good3"
echo "1 $(row int "$T/h1" "$T/c1" "$T/g1")  # want PASS, median 0.150"

mk h2 "$good" "$good2" "$good3"
mk c2 "$good" "$good2" "$bad"
mk g2 "$good" "$good2" "$good3"
echo "2 $(row int "$T/h2" "$T/c2" "$T/g2")  # want REFUSE (checksum disagree)"

mk h3 "$good" "$good2"
mk c3 "$good" "$good2" "$good3"
mk g3 "$good" "$good2" "$good3"
echo "3 $(row int "$T/h3" "$T/c3" "$T/g3")  # want REFUSE (short count)"

mk h4 "$good" "$good2" "$good3"
mk c4 "$good" "$good2" "$good3"
: >"$T/g4"
echo "4 $(row int "$T/h4" "$T/c4" "$T/g4")  # want REFUSE (empty guest)"

mk h5 'workload=io mib=64 mibs=1.0 checksum=cccc elapsed=0.080' \
      'workload=io mib=64 mibs=1.1 checksum=cccc elapsed=0.090' \
      'workload=io mib=64 mibs=0.9 checksum=cccc elapsed=0.070'
mk c5 'workload=io mib=64 mibs=1.0 checksum=cccc elapsed=0.080' \
      'workload=io mib=64 mibs=1.1 checksum=cccc elapsed=0.090' "$err"
mk g5 'workload=io mib=64 mibs=1.0 checksum=cccc elapsed=0.080' \
      'workload=io mib=64 mibs=1.1 checksum=cccc elapsed=0.090' \
      'workload=io mib=64 mibs=0.9 checksum=cccc elapsed=0.070'
echo "5 $(row io "$T/h5" "$T/c5" "$T/g5")  # want REFUSE (2 good + 1 error)"

mk h6 'workload=io mib=64 mibs=1.0 checksum=cccc elapsed=0.080' \
      'workload=io mib=64 mibs=1.1 checksum=cccc elapsed=0.090' \
      'workload=io mib=64 mibs=0.9 checksum=cccc elapsed=0.070'
mk c6 "$good" "$good2" "$good3"
mk g6 "$good" "$good2" "$good3"
echo "6 $(row io "$T/h6" "$T/c6" "$T/g6")  # want REFUSE (class absent)"
