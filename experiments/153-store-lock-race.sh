#!/usr/bin/env bash
# Question: is the store lock race reachable by a podbox process, or only by a
# test harness that runs many threads inside one process?
#
# TODO/image.md T-0215. The measurement that opened it is the first clause here:
# `cargo test --workspace` failed 5 of 12 runs on 2026-09-11, across four store
# lock tests, while the same tests alone passed 30 of 30.
#
# ⛔ THE ANSWER DECIDES WHERE THE FIX GOES, so this script measures before
# anything is changed. A race that only many threads in one process can produce
# is a test defect and is fixed in the test. A race two podbox processes can
# produce is a P0 in the product, because `in_use` is what `prune` asks before
# it deletes the blobs a running container executes out of.
#
# ⭐ EVERY CLAUSE RUNS THE SAME SUITE AND CHANGES ONE THING. Clause 1 is the
# subject. Each clause after it removes one candidate condition and holds the
# rest still: the same crates, the same binaries, the same cross-binary load.
#
#   0. the instrument's own positive control            without it a none means nothing
#   1. the suite as the gate runs it                    the subject
#   2. one test thread per binary                       removes threads in one process
#   3. neither of store.rs's two forking tests          removes TWO forks, not every one
#   4. the clone_fork forker alone                      keeps the shedding fork
#   5. the Command::spawn forker alone                  keeps the fork-then-exec window
#   6. clause 3 and probe_cache's tests as well         removes every fork this binary makes
#   7. the same suite on a second filesystem            changes where the locks live
#
# ⚠ CLAUSE 7 TESTS A HYPOTHESIS AND NOTHING IN THIS TREE SUPPORTS IT YET.
# Every captured failure clears microseconds later with no holder anywhere the
# kernel reports one, which is what a release that completes late would look
# like. `scratch` in the store's tests resolves its directory through
# `std::env::temp_dir()`, so `TMPDIR` moves every lock to another filesystem
# and changes nothing else. ⛔ If the two filesystems disagree the cause is
# below podbox; if they agree, this clause has ruled the idea out, which is
# worth the same.
#
# ⛔ CLAUSE 3 IS NOT THE FORK CONTROL AND CLAUSE 6 IS. `probe_cache` is a module
# of `podbox-image`, so its tests run in the SAME process as the store tests,
# and `resolve` calls `measure`, which runs the probe as one freshly forked
# child per probe. Skipping store.rs's own two forking tests therefore leaves
# the forking that TODO/image.md T-0211 names in place. Reading clause 3 as
# "no fork" was this script's own first mistake and the comment stays so nobody
# makes it twice.
#
# ⚠ THE PRODUCT SHAPE IS NOT MEASURED AGAIN HERE. The process that holds an
# image lock for a container's life is the detached launcher, and
# `230-lifecycle-loop.sh` clause 3 already reads `/proc/<launcher>/task` of a
# real one. `results/lifecycle-loop.txt` carries the reading. One fact, one
# home.
#
#   ./153-store-lock-race.sh
#   PODBOX_RACE_RUNS=12 ./153-store-lock-race.sh
#   PODBOX_RACE_CLAUSES="1 2" ./153-store-lock-race.sh
#
# Exit: 0 the measurement ran and no run failed, 1 it ran and a run failed,
#       2 it could not run.
#
# ⛔ It reports the count even when the count is zero. A racy check that
# happened to pass is not a check that passed.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/store-lock-race.txt"
RUNS="${PODBOX_RACE_RUNS:-12}"
CONTROL_RUNS="${PODBOX_RACE_CONTROL_RUNS:-$RUNS}"
CLAUSES="${PODBOX_RACE_CLAUSES:-0 1 2 3 4 5 6 7}"
WORK="$(mktemp -d)"

# ⚠ A failing run's log is the evidence and it is kept. The assertion and the
# panicking thread appear nowhere else.
KEEP="$WORK/logs"
mkdir -p "$KEEP"

FORKER_CLONE="a_fork_while_the_lock_is_held_does_not_extend_it"
FORKER_SPAWN="a_spawned_process_does_not_inherit_the_lock"
# ⚠ The module, not one test. Every `probe_cache` test calls `resolve`, and a
# cache miss there runs the probe, which forks.
FORKER_PROBE="probe_cache::"

command -v cargo >/dev/null 2>&1 || {
	echo "SKIP: no cargo on PATH. This measurement is the workspace test suite." >&2
	exit 2
}
[ -f "$REPO/Cargo.toml" ] || {
	echo "SKIP: $REPO holds no workspace" >&2
	exit 2
}

fail=0
REPORT="$WORK/report"
say() { printf '%s\n' "$*" >>"$REPORT"; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -sr)"
	printf 'nproc             %s\n' "$(nproc 2>/dev/null || echo -)"
	printf 'cargo             %s\n' "$(cargo --version 2>/dev/null || echo -)"
	printf 'rustc             %s\n' "$(rustc --version 2>/dev/null || echo -)"
	printf 'commit            %s\n' "$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo -)"
	printf 'runs per clause   %s subject, %s control\n' "$RUNS" "$CONTROL_RUNS"
	printf 'clauses           %s\n' "$CLAUSES"
	echo
} >"$REPORT"

# ⛔ The binaries are compiled ONCE, before any clause, so no clause carries a
# build's load into the run it is timing.
echo "compiling the test binaries" >&2
cargo test --workspace --no-run >"$WORK/build.log" 2>&1
brc=$?
if [ "$brc" -ne 0 ]; then
	say "== could not run"
	say "  cargo test --workspace --no-run exited $brc"
	tail -20 "$WORK/build.log" | sed 's/^/  /' >>"$REPORT"
	cat "$REPORT"
	exit 2
fi

# ⭐ ONE RUNNER FOR EVERY CLAUSE. The clauses differ by their arguments alone,
# so none of them can measure a different suite from the others.
# ⛔ EVERY CONTROL IS TAKEN TWICE, in two passes named A and B.
# `docs/methodology/experiments.md`: a control run once is a coincidence you
# have not noticed yet. That is not a general caution here, it is what this
# script measured: clause 6 read 3 of 12 in one pass and 0 of 12 in the next,
# so a reading taken from either pass alone would have named a cause the other
# pass refuses.
#
#   $1 clause number   $2 pass letter   $3 what it changes   $4 run count
#   $5.. libtest arguments
measure() {
	n="$1"
	pass="$2"
	label="$3"
	runs="$4"
	shift 4
	failed=0
	total=0
	names="$WORK/names.$n$pass"
	: >"$names"
	: >"$WORK/asserts.$n$pass"

	say "== clause $n pass $pass  $label"
	if [ "$#" -gt 0 ]; then
		say "  command          cargo test --workspace -- $*"
	else
		say "  command          cargo test --workspace"
	fi
	say "  runs             $runs"

	i=1
	while [ "$i" -le "$runs" ]; do
		log="$KEEP/clause$n$pass.run$i.log"
		printf 'clause %s pass %s run %s of %s\n' "$n" "$pass" "$i" "$runs" >&2
		s=$(date +%s)
		if [ "$#" -gt 0 ]; then
			cargo test --workspace -- "$@" >"$log" 2>&1
		else
			cargo test --workspace >"$log" 2>&1
		fi
		# ⛔ The code is read here, from the process that produced it, before
		# anything else can touch it.
		rc=$?
		e=$(date +%s)
		total=$((total + e - s))
		if [ "$rc" -ne 0 ]; then
			failed=$((failed + 1))
			grep -oE '^---- [^ ]+ stdout ----' "$log" |
				sed 's/^---- //' | sed 's/ stdout ----$//' |
				sed "s/^/run $i /" >>"$names"
			# ⚠ Twenty lines, because the assertion message carries the
			# instrument's `/proc/self/fd` and `/proc/locks` rows and a
			# three-line window cut off the part that names the holder.
			grep -A20 'panicked at' "$log" | sed "s/^/  run $i  /" >>"$WORK/asserts.$n$pass"
		else
			rm -f "$log"
		fi
		i=$((i + 1))
	done

	say "  failed runs      $failed of $runs"
	say "  seconds          $total total for $runs runs"
	if [ -s "$names" ]; then
		say "  the failing tests, by run:"
		sed 's/^/    /' "$names" >>"$REPORT"
		say "  distinct tests:"
		awk '{ print $3 }' "$names" | sort | uniq -c | sed 's/^/    /' >>"$REPORT"
		say "  the assertions:"
		head -160 "$WORK/asserts.$n$pass" >>"$REPORT"
	else
		say "  the failing tests: none"
	fi
	say ""
	eval "CLAUSE${n}_${pass}_FAILED=$failed"
	eval "CLAUSE${n}_RUNS=$runs"
}

# ⭐ CLAUSE 0 IS THE INSTRUMENT'S OWN POSITIVE CONTROL, and every "none" the
# other clauses print is worthless without it. It holds a lock, asks the
# instrument, and puts the answer in the evidence, so a reader can tell an
# empty world from a blind probe.
instrument_control() {
	say "== clause 0  the instrument's positive control"
	say "  command          cargo test -p podbox-image the_t_0215_instrument -- --nocapture"
	cargo test -p podbox-image the_t_0215_instrument -- --nocapture \
		>"$KEEP/clause0.log" 2>&1
	rc=$?
	say "  exit             $rc"
	# ⚠ The `^ +[0-9]+:` arm is the kernel's own lock rows, and leaving it out
	# dropped the very lines that prove the instrument is not blind.
	grep -E "T-0215 positive control|^    fd |/proc/locks|inode |descriptions on it|^ +[0-9]+: (FLOCK|POSIX|OFDLCK)" \
		"$KEEP/clause0.log" | sed 's/^/    /' >>"$REPORT"
	if [ "$rc" -ne 0 ]; then
		# ⚠ No backticks in this string: it is double quoted, and a shell
		# would run what is inside them.
		say "  ⛔ THE INSTRUMENT IS BLIND, so every none below says nothing."
		fail=1
	fi
	say ""
}

# ⭐ CLAUSE 7. The same suite with every lock on a second filesystem, and
# nothing else changed. ⛔ It refuses to report a comparison it did not make:
# where the two paths are the same filesystem type there is no second
# filesystem, and saying so is the honest answer.
second_filesystem() {
	pass="$1"
	alt="${PODBOX_RACE_ALT_TMP:-/dev/shm}"
	here_fs="$(stat -f -c %T "${TMPDIR:-/tmp}" 2>/dev/null || echo -)"
	alt_fs="$(stat -f -c %T "$alt" 2>/dev/null || echo -)"
	if [ ! -d "$alt" ] || [ ! -w "$alt" ]; then
		say "== clause 7 pass $pass  a second filesystem"
		say "  SKIP: $alt is not a writable directory here"
		say ""
		return
	fi
	if [ "$here_fs" = "$alt_fs" ]; then
		say "== clause 7 pass $pass  a second filesystem"
		say "  SKIP: ${TMPDIR:-/tmp} and $alt are both $here_fs, so this clause"
		say "  would change nothing. PODBOX_RACE_ALT_TMP names another path."
		say ""
		return
	fi
	work="$alt/podbox-race-$$"
	mkdir -p "$work" || {
		say "== clause 7 pass $pass  a second filesystem"
		say "  SKIP: cannot create $work"
		say ""
		return
	}
	say "  ⚠ clause 7 moves TMPDIR from ${TMPDIR:-/tmp} ($here_fs) to $work ($alt_fs)"
	TMPDIR="$work" measure 7 "$pass" \
		"every lock on $alt_fs instead of $here_fs" "$CONTROL_RUNS"
	rm -rf "$work"
}

for c in $CLAUSES; do
	for p in A B; do
		case "$c" in
		# ⚠ The subject is taken once. That it reproduces is not the claim
		# under test; the controls are, and each of them is taken twice.
		1) [ "$p" = A ] && measure 1 A "the suite as the gate runs it. THE SUBJECT" "$RUNS" ;;
		2) measure 2 "$p" "one test thread per binary. Removes threads in one process" \
			"$CONTROL_RUNS" --test-threads=1 ;;
		3) measure 3 "$p" "store.rs's two forking tests skipped. NOT the fork control" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" --skip "$FORKER_SPAWN" ;;
		4) measure 4 "$p" "the clone_fork forker kept, the spawning one skipped" \
			"$CONTROL_RUNS" --skip "$FORKER_SPAWN" ;;
		5) measure 5 "$p" "the spawning forker kept, the clone_fork one skipped" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" ;;
		6) measure 6 "$p" "probe_cache skipped as well. THE FORK CONTROL" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" --skip "$FORKER_SPAWN" \
			--skip "$FORKER_PROBE" ;;
		7) second_filesystem "$p" ;;
		0) [ "$p" = A ] && instrument_control ;;
		*)
			if [ "$p" = A ]; then
				say "== clause $c is not one this script has"
				say ""
			fi
			;;
		esac
	done
done

# ⭐ THE READING, and it states what TWO AGREEING PASSES support and nothing
# more. A clause that did not run says so rather than being read as a zero.
#
# ⛔ A CONTROL WHOSE TWO PASSES DISAGREE RULES NOTHING, and this script says
# so rather than taking the convenient pass. Clause 6 read 3 of 12 and then
# 0 of 12 on 2026-09-12, which is the reason this block exists in this shape.
say "== what the clauses support"

#   $1 clause number   $2 what the clause removes
verdict() {
	n="$1"
	what="$2"
	eval "a=\${CLAUSE${n}_A_FAILED:-}"
	eval "b=\${CLAUSE${n}_B_FAILED:-}"
	runs="$(eval "echo \${CLAUSE${n}_RUNS:-0}")"
	if [ -z "$a" ] && [ -z "$b" ]; then
		return
	fi
	if [ -z "$a" ] || [ -z "$b" ]; then
		say "  $what: only one pass ran (${a}${b} of $runs). ⚠ One pass rules"
		say "  nothing: run it twice."
		return
	fi
	say "  $what: pass A $a of $runs, pass B $b of $runs"
	if [ "$a" -eq 0 ] && [ "$b" -eq 0 ]; then
		say "    both passes green, so what this clause removes is a NECESSARY"
		say "    condition as far as two passes of $runs can say."
	elif [ "$a" -gt 0 ] && [ "$b" -gt 0 ]; then
		say "    both passes red, so what this clause removes is NOT necessary."
	else
		say "    ⛔ THE TWO PASSES DISAGREE, so this clause rules nothing. A rate"
		say "    this unstable needs more runs than $runs before it is read."
	fi
}

c1="${CLAUSE1_A_FAILED:-}"
if [ -z "$c1" ]; then
	say "  clause 1 did not run, so nothing below has a subject to compare with"
elif [ "$c1" -eq 0 ]; then
	say "  the subject did not reproduce in ${CLAUSE1_RUNS:-0} runs, so no control"
	say "  below can subtract a condition from a failure that did not happen."
	say "  ⚠ That is not a green suite: 2026-09-11 measured 5 failures of 12."
else
	say "  the subject reproduced: $c1 of ${CLAUSE1_RUNS:-0} runs failed"
	# ⛔ No pass-by-pass history is quoted here. A number written into a script
	# goes stale the next time the script runs, and TODO/image.md T-0215 is the
	# one home for the series.
	say "  ⚠ The subject's own rate is unstable across passes, and T-0215 carries"
	say "  the series. A control that reads 0 once is therefore weaker evidence"
	say "  than it looks, and that is why each one below is taken twice."
	verdict 2 "threads in one process removed"
	verdict 3 "two of store.rs's forks removed"
	verdict 4 "the spawning forker removed"
	verdict 5 "the clone_fork forker removed"
	verdict 6 "every forking test removed"
	verdict 7 "the locks moved to a second filesystem"
fi
say ""
say "⚠ What this script cannot say: that it generalises. It is one machine on"
say "  one day, and the conditions block above says which."

# ⛔ The subject's failures are the exit code. A control's failures are data.
[ "${CLAUSE1_A_FAILED:-0}" -eq 0 ] || fail=1

cat "$REPORT"
mkdir -p "$(dirname "$OUT")"
cp "$REPORT" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

# ⚠ A disposable container keeps nothing, so a job hands its evidence back
# through /out when the caller asked for one.
if [ -d /out ]; then
	cp "$REPORT" /out/store-lock-race.txt 2>/dev/null
	if [ -n "$(ls -A "$KEEP" 2>/dev/null)" ]; then
		mkdir -p /out/store-lock-race-logs
		cp "$KEEP"/* /out/store-lock-race-logs/ 2>/dev/null
	fi
fi
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

echo "the failing runs' logs are in $KEEP"
[ "$fail" -eq 0 ] || exit 1
exit 0
