#!/bin/sh
# Question: does podvm's static probe write a report inside images that
# have no shell or no interpreter, on the ladder's rung and in a guest of
# the machine tier, and does `podvm compare` fail only on an unexplained
# difference in an invariant section?
#
# TODO/probe.md T-1666 (issue 186). The probe is podvm's own static binary:
# `probe-inside` copies it into the container's own tree (T-1709), or into
# the guest, and runs it as the payload.
#
# Rows (the images): ALPINE, the musl row of DISTRO_ROWS_M5 (alpine 3.20 by
# digest); BUSYBOX (busybox:latest by tag); DISTROLESS
# (gcr.io/distroless/static-debian12 by tag). Each on the rung that the
# ladder selects here, ALPINE also on the chroot-denying fixture of
# experiment 356 (the ladder's rung there, and the syscall tier of T-1657)
# and in a guest of the machine tier. The emulate and incus tiers need a
# qemu-user and an incusd, which this lane does not install.
# PODVM_422_MIRROR=1 takes the Docker-library images from mirror.gcr.io.
#
# Clauses:
#   1. probe-inside exits 0 in each row, and each report holds 50 records
#      or more, each with a verdict of the vocabulary.
#   2. two reports of one tier compare clean under --strict (exit 0).
#   3. a planted escape (escape-surface host-store ALLOW) fails --strict
#      (exit 1) and the line names it UNEXPLAINED.
#   4. the chroot rung against the machine tier compares clean under
#      --strict with experiments/probe-inside-expected.tsv, which gives a
#      cause for each difference in an invariant section.
#   5. the matrix of the verdicts of each section is saved.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep422-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/probe-inside.txt"
SAVE="$REPO/experiments/results/probe-inside"
EXPECTED="$REPO/experiments/probe-inside-expected.tsv"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
BUSYBOX='public.ecr.aws/docker/library/busybox:latest'
DISTROLESS='gcr.io/distroless/static-debian12'
if [ "${PODVM_422_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
	BUSYBOX='mirror.gcr.io/library/busybox:latest'
fi
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "/dev/kvm          $([ -c /dev/kvm ] && echo present || echo absent)"
for i in "$ALPINE" "$BUSYBOX" "$DISTROLESS"; do
	timeout 600 "$PB" pull -q "$i" >"$WORK/pull.log" 2>&1 ||
		{ note "pull of $i FAILED: COULD NOT RUN"; sed 's/^/  /' "$WORK/pull.log"; exit 2; }
	note "image             $i $("$PB" inspect --format '{{.Id}}' "$i" 2>/dev/null)"
done
note ""

# One row: NAME, then the command that writes the report to $WORK/NAME.tsv.
row() {
	name="$1"; shift
	timeout 900 "$@" >"$WORK/$name.log" 2>&1
	rc=$?
	n=$(grep -c "$(printf '\t')" "$WORK/$name.tsv" 2>/dev/null || echo 0)
	badv=$(awk -F '\t' '!/^#/ && NF >= 3 && $3 !~ /^(ALLOW|DENY|ABSENT|UNKNOWN|TIMEOUT|UNRESOLVED|REFUSED|DROPPED|EXHAUSTED)$/' "$WORK/$name.tsv" 2>/dev/null | wc -l)
	note "  $name: exit $rc, $n records, $(grep '^# records' "$WORK/$name.tsv" 2>/dev/null | cut -d: -f2-)"
	[ "$rc" -eq 0 ] || { bad "$name exited $rc"; tail -3 "$WORK/$name.log" | sed 's/^/      /' | tee -a "$OUT"; }
	[ "$n" -ge 50 ] || bad "$name holds $n records"
	[ "$badv" -eq 0 ] || bad "$name holds $badv lines with a verdict outside the vocabulary"
}

note "== clause 1  a report in each row"
row alpine-ladder "$PB" probe-inside -o "$WORK/alpine-ladder.tsv" "$ALPINE"
row alpine-ladder-2 "$PB" probe-inside -o "$WORK/alpine-ladder-2.tsv" "$ALPINE"
row busybox-ladder "$PB" probe-inside -o "$WORK/busybox-ladder.tsv" "$BUSYBOX"
row distroless-ladder "$PB" probe-inside -o "$WORK/distroless-ladder.tsv" "$DISTROLESS"
row alpine-fixture unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" probe-inside -o "$WORK/alpine-fixture.tsv" "$ALPINE"
row alpine-syscall unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" probe-inside --podvm-tier syscall -o "$WORK/alpine-syscall.tsv" "$ALPINE"
row alpine-machine "$PB" probe-inside --podvm-tier machine -o "$WORK/alpine-machine.tsv" "$ALPINE"
note ""

note "== clause 2  two reports of one tier"
"$PB" compare "$WORK/alpine-ladder.tsv" --reference "$WORK/alpine-ladder-2.tsv" --strict >"$WORK/c2.out" 2>&1
rc=$?
note "  exit $rc  (want 0): $(head -1 "$WORK/c2.out")"
[ "$rc" -eq 0 ] || { bad "two reports of one tier failed --strict"; sed 's/^/    /' "$WORK/c2.out" | tee -a "$OUT"; }
note ""

note "== clause 3  a planted escape"
sed 's/^escape-surface\thost-store\t[A-Z]*\t.*/escape-surface\thost-store\tALLOW\topened (planted)/' "$WORK/alpine-ladder.tsv" >"$WORK/planted.tsv"
"$PB" compare "$WORK/planted.tsv" --reference "$WORK/alpine-ladder.tsv" --strict >"$WORK/c3.out" 2>&1
rc=$?
note "  exit $rc  (want 1)"
sed 's/^/    /' "$WORK/c3.out" | tee -a "$OUT"
[ "$rc" -eq 1 ] || bad "the planted escape did not fail --strict"
grep -q "host-store.*UNEXPLAINED" "$WORK/c3.out" || bad "the line does not name the escape UNEXPLAINED"
note ""

note "== clause 4  the chroot rung against the machine tier, with the causes"
"$PB" compare "$WORK/alpine-machine.tsv" --reference "$WORK/alpine-ladder.tsv" --expected "$EXPECTED" --strict >"$WORK/c4.out" 2>&1
rc=$?
note "  exit $rc  (want 0)"
sed 's/^/    /' "$WORK/c4.out" | tee -a "$OUT"
[ "$rc" -eq 0 ] || bad "a difference of the machine tier has no cause in $EXPECTED"
note ""

note "== clause 5  the matrix"
rows="alpine-ladder busybox-ladder distroless-ladder alpine-fixture alpine-syscall alpine-machine"
for r in $rows; do
	[ -f "$WORK/$r.tsv" ] || continue
	# Each report, saved without its env section (the variables of the
	# host do not belong in the repository) and with the work path cut.
	mkdir -p "$SAVE"
	grep -v "^env$(printf '\t')" "$WORK/$r.tsv" | sed "s#$WORK#WORK#g" >"$SAVE/$r.tsv"
done
{
	printf '%-16s' "section"
	for r in $rows; do printf ' %-18s' "$r"; done
	printf '\n'
	for s in identity escalation escape-surface procfs raw-socket proxy dns tcp-direct tcp-direct-ip udp-direct loopback metadata; do
		printf '%-16s' "$s"
		for r in $rows; do
			c=$(awk -F '\t' -v s="$s" '$1 == s {print $3}' "$WORK/$r.tsv" 2>/dev/null | sort | uniq -c | awk '{printf "%s%s:%s", (NR>1?",":""), (substr($2,1,2)=="UN" && $2=="UNRESOLVED" ? "UR" : substr($2,1,2)), $1}')
			printf ' %-18s' "${c:--}"
		done
		printf '\n'
	done
	printf '\nAL ALLOW, DE DENY, AB ABSENT, UN UNKNOWN, TI TIMEOUT, UR UNRESOLVED, RE REFUSED, DR DROPPED, EX EXHAUSTED (the first two letters; the reports under experiments/results/probe-inside/ hold each verdict)\n'
} | tee -a "$OUT"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE PROBE WRITES A REPORT IN EACH ROW, AND COMPARE FAILS ONLY ON AN UNEXPLAINED ESCAPE"
else
	note "verdict: A CLAUSE DISAGREED"
fi
sed -i "s#$WORK#WORK#g" "$OUT"
pkill -f "qemu-system.*podvm-init" 2>/dev/null
rm -rf "$WORK"
exit "$fail"
