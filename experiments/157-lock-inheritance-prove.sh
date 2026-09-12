#!/usr/bin/env bash
# Question: do T-0211's two tests pass repeatably, and does each mutation turn
# exactly ONE of them red?
#
# TODO/image.md T-0211. It was moved from `done` to `partial` on 2026-09-11 for
# one reason: it recorded no run, and both tests its `Prove` names are in
# T-0215's measured-intermittent set. ⛔ A SINGLE GREEN RUN CANNOT CLOSE IT.
# This script is what a closing record needs instead: a pass count out of a
# stated number of attempts, plus the two mutations run rather than described.
#
# ⭐ EACH TEST IS RUN ALONE, WHICH IS THE SHAPE THE `Prove` NAMES.
# `cargo test -p podbox-image NAME` runs one test in its own process, so the
# many-threads-in-one-process condition T-0215 measured is absent by
# construction. ⚠ That is the point and it is also the limit: this script
# cannot say the suite is sound, only that these two tests are. T-0215 owns the
# suite.
#
# ⛔ THE MUTATIONS ARE THE HALF THAT ASSERTS INDEPENDENCE. Removing the fork
# defence must redden the fork test and leave the exec test green, and removing
# the exec defence must do the reverse. A mutation that reddens both would mean
# one mechanism written twice.
#
#   ./157-lock-inheritance-prove.sh
#   PODBOX_PROVE_RUNS=30 ./157-lock-inheritance-prove.sh
#
# Exit: 0 every attempt passed and both mutations were caught exactly once,
#       1 an attempt failed or a mutation was not caught, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/lock-inheritance-prove.txt"
RUNS="${PODBOX_PROVE_RUNS:-30}"
WORK="$(mktemp -d)"
BACKUP="$WORK/backup"

FORK_TEST="a_fork_while_the_lock_is_held_does_not_extend_it"
EXEC_TEST="a_spawned_process_does_not_inherit_the_lock"
SUBJECT="crates/podbox-image/src/store.rs"

command -v cargo >/dev/null 2>&1 || {
	echo "SKIP: no cargo on PATH." >&2
	exit 2
}
[ -f "$REPO/$SUBJECT" ] || {
	echo "SKIP: $SUBJECT is not here, so this is not the podbox tree." >&2
	exit 2
}

# ⛔ REFUSE TO START ON A DIRTY SUBJECT. The mutations below edit this one file
# and put it back from a copy. An uncommitted change in it would be restored
# into a state nobody asked for, and `scripts/plant.sh` refuses for the same
# reason.
if ! git -C "$REPO" diff --quiet -- "$SUBJECT" ||
	! git -C "$REPO" diff --cached --quiet -- "$SUBJECT"; then
	echo "SKIP: $SUBJECT has uncommitted changes. This script must own it." >&2
	git -C "$REPO" status --porcelain -- "$SUBJECT" >&2
	exit 2
fi

mkdir -p "$BACKUP/$(dirname "$SUBJECT")"
cp "$REPO/$SUBJECT" "$BACKUP/$SUBJECT"
# ⛔ Restore from the COPY and never `git checkout --`: with a mutation staged,
# checkout restores the index, which is the mutation. `scripts/plant.sh` rule 2
# is the same lesson.
restore() { cp "$BACKUP/$SUBJECT" "$REPO/$SUBJECT"; }
trap 'restore; rm -rf "$WORK"' EXIT INT TERM

fail=0
REPORT="$WORK/report"
say() { printf '%s\n' "$*" >>"$REPORT"; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -sr)"
	printf 'nproc             %s\n' "$(nproc 2>/dev/null || echo -)"
	printf 'cargo             %s\n' "$(cargo --version 2>/dev/null || echo -)"
	printf 'commit            %s\n' "$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo -)"
	printf 'attempts          %s per test\n' "$RUNS"
	echo
} >"$REPORT"

#   $1 test name   prints the exit code, nothing else
one() {
	cargo test -p podbox-image "$1" >"$WORK/one.log" 2>&1
	# ⛔ Read here, from the process that produced it.
	echo $?
}

say "== clause 1  each test alone, $RUNS attempts each"
for t in "$FORK_TEST" "$EXEC_TEST"; do
	passed=0
	i=1
	while [ "$i" -le "$RUNS" ]; do
		printf 'clause 1 %s attempt %s of %s\n' "$t" "$i" "$RUNS" >&2
		rc=$(one "$t")
		if [ "$rc" -eq 0 ]; then
			passed=$((passed + 1))
		else
			cp "$WORK/one.log" "$WORK/fail.$t.$i.log"
		fi
		i=$((i + 1))
	done
	say "  $t"
	say "    passed           $passed of $RUNS"
	if [ "$passed" -ne "$RUNS" ]; then
		say "    ⛔ NOT REPEATABLE, so this is not a closing record"
		fail=1
	fi
done
say ""

# ⭐ ONE MUTATION AT A TIME, and each is asserted to LAND before it is read.
# A mutation whose pattern matched nothing exits 0 exactly like a defence that
# held, which is the defect `scripts/plant.sh` exists to prevent.
#   $1 label   $2 sed script   $3 the test that must go red   $4 the one that must stay green
mutate() {
	label="$1"
	script="$2"
	must_red="$3"
	must_green="$4"
	say "== $label"
	restore
	before="$(git -C "$REPO" hash-object "$REPO/$SUBJECT")"
	sed -i -E "$script" "$REPO/$SUBJECT"
	after="$(git -C "$REPO" hash-object "$REPO/$SUBJECT")"
	if [ "$before" = "$after" ]; then
		say "  ⛔ THE MUTATION DID NOT LAND, so nothing below was measured"
		say "  sed:  $script"
		fail=1
		restore
		say ""
		return
	fi
	red=$(one "$must_red")
	green=$(one "$must_green")
	say "  $must_red exited $red  (want non-zero)"
	say "  $must_green exited $green  (want 0)"
	[ "$red" -ne 0 ] || {
		say "  ⛔ the mutation did not redden its own test"
		fail=1
	}
	[ "$green" -eq 0 ] || {
		say "  ⛔ the mutation reddened the OTHER test as well, so the two"
		say "     defences are not independent"
		fail=1
	}
	restore
	say ""
}

# ⚠ The fork defence is the registration line in `Store::hold`. Deleting the
# `if !sys::close_in_children(...)` guard would leave an unbalanced block, so
# the call is turned into one that registers nothing and still returns true.
mutate "clause 2  the fork defence removed from Store::hold" \
	's/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/' \
	"$FORK_TEST" "$EXEC_TEST"

# ⚠ The exec defence is the one flag in `Lock::open`.
mutate "clause 3  O_CLOEXEC removed from Lock::open" \
	's/let flags = sys::O_RDWR \| sys::O_CREAT \| sys::O_CLOEXEC;/let flags = sys::O_RDWR | sys::O_CREAT;/' \
	"$EXEC_TEST" "$FORK_TEST"

say "== the tree afterwards"
restore
if git -C "$REPO" diff --quiet -- "$SUBJECT"; then
	say "  $SUBJECT is back as it was"
else
	say "  ⛔ $SUBJECT IS NOT BACK AS IT WAS"
	fail=1
fi

cat "$REPORT"
mkdir -p "$(dirname "$OUT")"
cp "$REPORT" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
if [ -d /out ]; then
	cp "$REPORT" /out/lock-inheritance-prove.txt 2>/dev/null
fi
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
exit 0
