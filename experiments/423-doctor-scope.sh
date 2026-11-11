#!/bin/sh
# Question: does `podvm doctor` give each capability of the widened scope
# one of three answers (ok, MISSING with a fix, cannot be fixed on this
# host), proved by a run where it can be, and does `--strict` exit 1 where
# one is MISSING?
#
# TODO/cli.md T-1667 (issue 188). Both runs go through the fixture of
# experiment 356 (user and mount namespaces, /dev/pts hidden, chroot(2)
# denied); the second run also denies prctl(PR_SET_SYSCALL_USER_DISPATCH)
# (`--deny-dispatch`), so the native-static rung cannot be had there.
# INCUS_DIR names a directory with no incusd, so scope.incus is MISSING.
#
# Clauses:
#   1. each of the ten scope capabilities has one line, with ok, MISSING
#      or cannot, and each MISSING has a fix line.
#   2. scope.incus is MISSING; `doctor` exits 0 and `doctor --strict`
#      exits 1.
#   3. with dispatch denied, scope.native-static and scope.syscall-tier
#      read "cannot be fixed on this host"; with it allowed, they read ok
#      (a payload entered).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep423-work"
rm -rf "$WORK"; mkdir -p "$WORK/store" "$WORK/no-incus" || exit 2
OUT="$REPO/experiments/results/doctor-scope.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="$WORK/store"
export INCUS_DIR="$WORK/no-incus"
unset PODVM_INCUS_SOCKET
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
NAMES="native-static syscall-tier openat2 landlock user-net emulate incus proxy space assets"

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
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note ""

doc() {
	name="$1"; shift
	unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$@" "$PB" doctor >"$WORK/$name.out" 2>&1
	rc=$?
	unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$@" "$PB" doctor --strict >"$WORK/$name-strict.out" 2>&1
	src=$?
	note "== $name: doctor exit $rc, doctor --strict exit $src"
	grep -E "^(ok|MISSING|cannot) scope\.|^fix: scope\." "$WORK/$name.out" | sed "s#$WORK#WORK#g" | sed 's/^/    /' | tee -a "$OUT"
}

doc fixture
note "== clause 1  one answer for each capability"
for n in $NAMES; do
	c=$(grep -cE "^(ok|MISSING|cannot) scope\.$n:" "$WORK/fixture.out")
	[ "$c" -eq 1 ] || bad "scope.$n has $c answer lines"
	if grep -qE "^MISSING scope\.$n:" "$WORK/fixture.out"; then
		grep -q "^fix: scope\.$n:" "$WORK/fixture.out" || bad "scope.$n is MISSING with no fix line"
	fi
done
note "== clause 2  MISSING and --strict"
grep -q "^MISSING scope.incus:" "$WORK/fixture.out" || bad "scope.incus is not MISSING"
[ "$rc" -eq 0 ] || bad "doctor exited $rc with only a MISSING"
[ "$src" -eq 1 ] || bad "doctor --strict exited $src with a MISSING"
note "== clause 3  the rung by a run"
grep -q "^ok scope.native-static: a static payload entered" "$WORK/fixture.out" || bad "the native-static rung did not enter on the fixture"
note ""

doc fixture-deny-dispatch --deny-dispatch
grep -q "^cannot scope.native-static: cannot be fixed on this host" "$WORK/fixture-deny-dispatch.out" || bad "with dispatch denied, scope.native-static is not cannot"
grep -q "^cannot scope.syscall-tier: cannot be fixed on this host" "$WORK/fixture-deny-dispatch.out" || bad "with dispatch denied, scope.syscall-tier is not cannot"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: EACH SCOPE CAPABILITY HAS ONE OF THE THREE ANSWERS, AND --strict FAILS ON A MISSING"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
