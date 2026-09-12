#!/usr/bin/env bash
# Question: is the store lock race reachable by a podbox process, or only by a
# test harness that runs many threads inside one process?
#
# TODO/image.md T-0215. The measurement that opened it is the first clause here:
# `cargo test --workspace` failed 5 of 12 runs on 2026-09-11, and that entry now
# names six store lock tests rather than the four it opened with. ⚠ The counts
# live in the entry and not in this comment, because a number written into a
# script goes stale the next time the script runs.
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
#   4. the only fork left is clone_fork                 keeps the shedding fork
#   5. the only fork left is Command::spawn             keeps the fork-then-exec window
#   6. clause 3, probe_cache and pull's forker as well  removes every fork this binary makes
#   7. the same suite on a second filesystem            changes where the locks live
#   8. the pre-registration window widened              makes one candidate window bigger
#   9. podbox's own spawn no longer sheds                puts one fd closure back as it was
#  10. the hook gutted, and its control asserted red   the control's own plant, and no rate
#  11. the test's own bare spawn given the hook       the last unshed fork in the process
#
# ⚠ CLAUSE 7 WAS BUILT FOR ONE IDEA AND HAS ALREADY RULED IT OUT. A refusal
# that clears at once with no holder anywhere the kernel reports one is what a
# release completing late would look like, so the clause moves every lock to
# another filesystem and changes nothing else: `scratch` in the store's tests
# resolves its directory through `std::env::temp_dir()`, which `TMPDIR` moves.
# ⛔ T-0215 carries what it found. The clause stays because a filesystem is
# worth re-varying on any host where this race is chased again.
#
# ⛔ CLAUSE 3 IS NOT THE FORK CONTROL AND CLAUSE 6 IS. `probe_cache` is a module
# of `podbox-image`, so its tests run in the SAME process as the store tests,
# and `resolve` calls `measure`, which runs the probe as one freshly forked
# child per probe. Skipping store.rs's own two forking tests therefore leaves
# the forking that TODO/image.md T-0211 names in place.
#
# ⛔ THE FORK CONTROL WAS WRONG TWICE AND THE SKIP LIST IS WHY THIS COMMENT IS
# LONG. First it skipped two tests and was read as skipping every fork.
# Then it added `probe_cache::` and was still short one: `pull` calls
# `probe_cache::resolve` once it is past the transport policy, so
# `naming_the_registry_insecure_gets_past_the_policy` forks under a name that
# says nothing about forking. ⚠ A SKIP LIST IS A CLAIM ABOUT THE CODE AND IT IS
# CHECKED BY READING THE CODE, not by the names of the tests. The four
# names below are every path to a fork in PODBOX-IMAGE'S TEST BINARY as of
# 2026-09-12: `clone_fork` in store.rs, `Command::spawn` in store.rs, and
# `podbox_probe::run` reached through `probe_cache::measure` from the
# `probe_cache` tests and from `pull`.
# ⚠ THE BINARY IS NAMED BECAUSE ANOTHER ONE FORKS TOO, and skipping a test in
# it would change nothing here. `podbox-probe`'s own test binary spawns
# `/bin/sh` in
# `a_spawn_through_the_hook_sheds_a_registered_fd_and_one_without_it_does_not`,
# in a different process, which cannot hold a descriptor on a store lock this
# binary opened. It adds cross-binary load and no fork to the failing process.
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
# ⛔ AND THIS ONE, WHICH IS WHY CLAUSE 6 WAS WRONG TWICE. `pull` calls
# `probe_cache::resolve` once it is past the transport policy, so the test that
# gets past that policy forks, and its name carries no hint of it. The other
# `pull` test is refused before that line and forks nothing.
# `crates/podbox-image/src/pull.rs` is where the order is visible.
FORKER_PULL="naming_the_registry_insecure_gets_past_the_policy"

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
	# ⛔ A READING TAKEN ON A MODIFIED TREE IS NOT A READING OF THAT COMMIT, and
	# every clause here exists to measure a change, so the tree is usually
	# modified. Printing the commit alone names a state that was not run.
	if git -C "$REPO" diff --quiet HEAD 2>/dev/null; then
		printf 'tree              clean at that commit\n'
	else
		printf 'tree              ⛔ MODIFIED. What ran is that commit PLUS:\n'
		git -C "$REPO" diff --stat HEAD 2>/dev/null | sed 's/^/                  /'
	fi
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
# ⛔ EVERY CLAUSE IS TAKEN TWICE, the subject included, in two passes named A
# and B.
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

# ⭐ CLAUSES 8 AND 9 MUTATE THE SOURCE, THE SAME WAY `plant.sh` DOES, AND
# NEITHER MUTATION EVER REACHES THE TREE.
#
# ⛔ Every clause above this one SUBTRACTS a test. These two change the CODE,
# because the two remaining candidates are not properties of the test selection:
#
#   8. `Lock::try_acquire` opens the fd, locks it, and only THEN registers it
#      with `sys::close_in_children`. A fork landing between the open and the
#      registration leaves a child holding a lock fd that is in no table, so
#      nothing can shed it. That window is a few instructions wide, so WIDEN it:
#      a window that is the mechanism gets worse when it is made longer.
#   9. `run_payload` spawns through `sys::shed_after_fork`. Take the hook off
#      and the tree is back to the state where libstd's fork carried every lock
#      fd into the child. ⭐ This is the A of the A/B whose B is the subject,
#      and it is a clause rather than a hand-edit so that both legs land in one
#      evidence file with one conditions block.
#
# ⚠ An amplification is READ IN THE OPPOSITE DIRECTION from a subtraction, and
# the reading block below keeps them apart.
#
#   $1 clause  $2 pass  $3 label  $4 file, relative to the repository
#   $5 the anchor, matched as a whole line  $6 `after` or `replace`  $7 the line
mutate_and_measure() {
	n="$1"
	pass="$2"
	label="$3"
	src="$REPO/$4"
	anchor="$5"
	how="$6"
	line="$7"
	hits=$(grep -c -F -x -- "$anchor" "$src")
	if [ "$hits" != "1" ]; then
		say "== clause $n pass $pass  $label"
		say "  SKIP: the anchor matched $hits times in $4, not once, so this"
		say "  clause cannot say where it mutated. Re-read the function."
		say ""
		return
	fi
	cp "$src" "$WORK/mutated.orig"
	# ⛔ Restored however this script ends, including a kill. The name is set
	# BEFORE the trap that reads it, so a kill between the two lines cannot leave
	# the trap referring to an unset variable.
	MUTATED="$src"
	trap 'cp "$WORK/mutated.orig" "$MUTATED" 2>/dev/null' EXIT HUP INT TERM
	awk -v a="$anchor" -v l="$line" -v how="$how" '
		$0 == a { if (how == "replace") { print l; next } print; print l; next }
		{ print }
	' "$WORK/mutated.orig" >"$src"
	say "== clause $n pass $pass  $label"
	# ⛔ BOTH LINES, and the anchor alone is not enough. A reading that names
	# what was mutated and not what it became cannot be checked by a reader, and
	# reading the COMMAND rather than the label is what caught three wrong
	# verdicts in TODO/image.md T-0215.
	say "  mutation         $4"
	say "    $how, at:      $anchor"
	say "    with:          $line"
	if ! cargo test --workspace --no-run >"$WORK/mutate.log" 2>&1; then
		say "  SKIP: the mutated tree did not build"
		tail -10 "$WORK/mutate.log" | sed 's/^/  /' >>"$REPORT"
		say ""
		cp "$WORK/mutated.orig" "$src"
		cargo test --workspace --no-run >>"$WORK/mutate.log" 2>&1
		return
	fi
	measure "$n" "$pass" "$label" "$CONTROL_RUNS"
	cp "$WORK/mutated.orig" "$src"
	# ⚠ Rebuilt back to the unmutated tree, so a clause after this one does not
	# measure the mutation.
	cargo test --workspace --no-run >>"$WORK/mutate.log" 2>&1
}

# ⭐ CLAUSE 10. THE CONTROL'S OWN PLANT, AND IT MEASURES NO RATE.
#
# ⛔ Clause 9's null result is worth exactly as much as the proof that
# `sys::shed_after_fork` fires, and that proof is one test. A control nobody has
# seen fail is not a control, which is `scripts/plant.sh`'s whole argument
# applied to a test rather than to a gate check.
#
# ⚠ It mutates the hook's body to nothing and asserts the control goes RED. A
# control that stays green with the hook gutted is measuring something else.
plant_the_hook_control() {
	pass="$1"
	src="$REPO/crates/podbox-probe/src/sys.rs"
	anchor="            shed_registered_fds();"
	test_name="a_spawn_through_the_hook"
	hits=$(grep -c -F -x -- "$anchor" "$src")
	say "== clause 10 pass $pass  the hook gutted, and its control has to notice"
	if [ "$hits" != "1" ]; then
		say "  SKIP: the anchor matched $hits times, not once. The pre_exec body"
		say "  moved, so this plant cannot say what it mutated."
		say ""
		return
	fi
	cp "$src" "$WORK/mutated.orig"
	MUTATED="$src"
	trap 'cp "$WORK/mutated.orig" "$MUTATED" 2>/dev/null' EXIT HUP INT TERM
	awk -v a="$anchor" '{ if ($0 == a) print "            let _ = 0;"; else print }' 		"$WORK/mutated.orig" >"$src"
	say "  mutation         crates/podbox-probe/src/sys.rs"
	say "    replace, at:   $anchor"
	say "    with:          let _ = 0;"
	say "  command          cargo test -p podbox-probe $test_name"
	# ⛔ The code is read from the process that produced it, unpiped.
	cargo test -p podbox-probe "$test_name" >"$WORK/plant10.log" 2>&1
	rc=$?
	cp "$WORK/mutated.orig" "$src"
	say "  exit             $rc"
	if [ "$rc" -eq 0 ]; then
		say "  ⛔ THE CONTROL STAYED GREEN WITH THE HOOK GUTTED, so it proves"
		say "  nothing and every clause that rests on it rests on nothing."
		fail=1
	else
		say "  ⭐ the control went red, so it is watching the hook and not the air"
	fi
	grep -E "assertion|panicked at|the hook is installed" "$WORK/plant10.log" |
		head -6 | sed 's/^/    /' >>"$REPORT"
	say ""
	# ⚠ Rebuilt unmutated, so a clause after this one does not measure the plant.
	cargo test --workspace --no-run >>"$WORK/plant10.log" 2>&1
}

for c in $CLAUSES; do
	for p in A B; do
		case "$c" in
		# ⛔ THE SUBJECT IS TAKEN TWICE AS WELL, and it was taken once until
		# 2026-09-12. While nothing is being changed, its reproduction is not
		# the claim under test and one pass is enough. The moment a candidate
		# fix is in the tree the subject BECOMES the claim, and a subject that
		# reads 0 once says no more than a control that reads 0 once: this
		# suite's own rate has read anywhere from 2 to 10 of 12.
		1) measure 1 "$p" "the suite as the gate runs it. THE SUBJECT" "$RUNS" ;;
		2) measure 2 "$p" "one test thread per binary. Removes threads in one process" \
			"$CONTROL_RUNS" --test-threads=1 ;;
		3) measure 3 "$p" "store.rs's two forking tests skipped. NOT the fork control" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" --skip "$FORKER_SPAWN" ;;
		4) measure 4 "$p" "the ONLY fork left is store.rs's clone_fork" \
			"$CONTROL_RUNS" --skip "$FORKER_SPAWN" \
			--skip "$FORKER_PROBE" --skip "$FORKER_PULL" ;;
		5) measure 5 "$p" "the ONLY fork left is store.rs's Command::spawn" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" \
			--skip "$FORKER_PROBE" --skip "$FORKER_PULL" ;;
		6) measure 6 "$p" "every forking test skipped. THE FORK CONTROL" \
			"$CONTROL_RUNS" --skip "$FORKER_CLONE" --skip "$FORKER_SPAWN" \
			--skip "$FORKER_PROBE" --skip "$FORKER_PULL" ;;
		7) second_filesystem "$p" ;;
		8) mutate_and_measure 8 "$p" "the pre-registration window widened to 200 us" 			"crates/podbox-image/src/store.rs" 			"        let fd = Lock::open(path)?;" after 			"        std::thread::sleep(std::time::Duration::from_micros(200));" ;;
		9) mutate_and_measure 9 "$p" "podbox's own spawn no longer sheds" 			"crates/podbox-probe/src/probes.rs" 			"    match sys::shed_after_fork(&mut cmd).status() {" replace 			"    match cmd.status() {" ;;
		# ⚠ A plant is deterministic, so one pass is the whole of it. Every
		# clause that measures a RATE is taken twice; this one measures none.
		10) [ "$p" = A ] && plant_the_hook_control A ;;
		11) mutate_and_measure 11 "$p" "the bare spawn in the test given the hook too" "crates/podbox-image/src/store.rs" '        let mut child = std::process::Command::new("/bin/sh")' replace '        let mut child = sys::shed_after_fork(&mut std::process::Command::new("/bin/sh"))' ;;
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

c1a="${CLAUSE1_A_FAILED:-}"
c1b="${CLAUSE1_B_FAILED:-}"
c1=$((${c1a:-0} + ${c1b:-0}))
if [ -z "$c1a" ] && [ -z "$c1b" ]; then
	say "  clause 1 did not run, so nothing below has a subject to compare with"
elif [ -z "$c1a" ] || [ -z "$c1b" ]; then
	say "  ⛔ ONLY ONE PASS OF THE SUBJECT RAN ($c1 of ${CLAUSE1_RUNS:-0}), so it"
	say "  rules nothing on its own, whichever way it read."
elif [ "$c1" -eq 0 ]; then
	say "  the subject did not reproduce: pass A $c1a and pass B $c1b of"
	say "  ${CLAUSE1_RUNS:-0} runs. No control below can subtract a condition from"
	say "  a failure that did not happen."
	say "  ⭐ Where a candidate fix is in the tree, THIS is the reading that"
	say "  supports it, and it needs both passes green to say anything."
	say "  ⚠ Where nothing was changed, it is not a green suite: 2026-09-11"
	say "  measured 5 failures of 12."
else
	say "  the subject reproduced: pass A $c1a and pass B $c1b of ${CLAUSE1_RUNS:-0}"
	# ⛔ No pass-by-pass history is quoted here. A number written into a script
	# goes stale the next time the script runs, and TODO/image.md T-0215 is the
	# one home for the series.
	say "  ⚠ The subject's own rate is unstable across passes, and T-0215 carries"
	say "  the series. A control that reads 0 once is therefore weaker evidence"
	say "  than it looks, and that is why each one below is taken twice."
	verdict 2 "threads in one process removed"
	verdict 3 "two of store.rs's forks removed"
	verdict 4 "every fork but store.rs's clone_fork removed"
	verdict 5 "every fork but store.rs's Command::spawn removed"
	say "  ⚠ CLAUSES 4 AND 5 REMOVE THREE PATHS OF FOUR, so they remove most of"
	say "  the fork VOLUME as well as the other paths. A green reading in either"
	say "  can mean the kept path is not the one, OR that one fork per run is too"
	say "  little exposure to reach a rate this low. Clause 6 removes the fourth"
	say "  as well, so 4 and 5 are read against 6 and not against clause 1."
	verdict 6 "every forking test removed"
	verdict 7 "the locks moved to a second filesystem"
	# ⛔ CLAUSE 8 IS NOT READ BY `verdict`, because that helper says "what this
	# clause REMOVES" and clause 8 removes nothing: it makes one window LONGER.
	# A subtraction and an amplification are read in opposite directions.
	a8="${CLAUSE8_A_FAILED:-}"
	b8="${CLAUSE8_B_FAILED:-}"
	if [ -n "$a8" ] && [ -n "$b8" ]; then
		say "  the pre-registration window widened: pass A $a8 of ${CLAUSE8_RUNS:-0},"
		say "  pass B $b8 of ${CLAUSE8_RUNS:-0}, against the subject's $c1a and $c1b"
		say "  in this same run."
		say "    ⭐ AMPLIFICATION, NOT SUBTRACTION. A window that is the mechanism"
		say "    gets WORSE when it is made longer. A reading at or below the"
		say "    subject's own rate says this window is not where the fd escapes."
	fi
	a9="${CLAUSE9_A_FAILED:-}"
	b9="${CLAUSE9_B_FAILED:-}"
	if [ -n "$a9" ] && [ -n "$b9" ]; then
		say "  podbox's own spawn no longer sheds: pass A $a9 of ${CLAUSE9_RUNS:-0},"
		say "  pass B $b9 of ${CLAUSE9_RUNS:-0}, against the subject's $c1a and $c1b"
		say "  in this same run."
		say "    ⭐ A FIX TAKEN BACK OUT, so it is read the other way again: a"
		say "    reading ABOVE the subject means the hook is doing work, and one"
		say "    level with the subject means it is not what this failure needs."
	fi
fi
say ""
say "⚠ What this script cannot say: that it generalises. It is one machine on"
say "  one day, and the conditions block above says which."

# ⛔ The subject's failures are the exit code. A control's failures are data.
[ "${CLAUSE1_A_FAILED:-0}" -eq 0 ] && [ "${CLAUSE1_B_FAILED:-0}" -eq 0 ] || fail=1

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
