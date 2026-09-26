#!/bin/sh
# Question: do the root-listing legs and the ptmx legs run and report
# their own errno, and does a chroot run name the missing procfs?
#
# TODO/complete.md T-0414 (the root-listing clause and the `/dev/ptmx`
# clause) and TODO/complete.md T-0413 (the proc-absence clause: the banner
# names the missing procfs, and a process-substitution payload fails naming
# `/dev/fd` beside it). T-0413 rules no static fixture ships: nothing static
# can carry `/proc/self/fd` semantics, so absent plus named is the answer.
#
# The binary is `$PODBOX_BIN`, a lane-built podbox (the legs are its
# code). A non-native lane stages it through the M5 debian driver, the
# experiments/270 pattern: the probe runs inside the driver container,
# and the conditions block says so.
#
#   ./155-proc-absence.sh
#
# Exit: 0 every clause held, 1 one did not, 2 a leg could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/proc-absence.txt"
WORK="$REPO/experiments/.sweep155-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'eng_cleanup 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

fail=0
leg_not_run() {
	echo "  $1: could not run ($2)" >>"$WORK/report"
	echo 2 >"$WORK/worst"
}

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
case "$(uname -s)" in
# The artifact is a Linux binary: executing it here would prove nothing
# about the binary and only that this host is not Linux. Readability is
# the check; the driver runs it below.
Linux) echo "podbox            $($BIN version 2>/dev/null || echo MISSING)" ;;
*) if [ -r "$BIN" ]; then echo "podbox            lane-built artifact, runs in the driver below"; else echo "podbox            MISSING ($BIN unreadable)"; fi ;;
esac
} >"$WORK/report"

command -v jq >/dev/null 2>&1 || { echo "jq is missing" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
# The engine drives the driver lane only, and `LANE` below is native on
# every Linux machine: a native lane runs each clause directly and needs
# no daemon. Requiring one here exits 2 on machines that run podbox
# natively without docker (a lane job container holds no NET_ADMIN and
# cannot start dockerd, measured 2026-09-19 in `experiments/lib/engine.sh`).
case "$(uname -s)" in
Linux)
	echo "engine            none needed on the native lane" >>"$WORK/report"
	;;
*)
	[ "$HAVE_ENGINE" -eq 1 ] || { echo "engine            none answered" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	;;
esac

# The driver hosts the staged binary on a non-native lane; natively the
# binary executes where it stands.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
case "$(uname -s)" in
Linux) BIN_RUN="$BIN"; [ -x "$BIN_RUN" ] || { echo "binary $BIN is not executable" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; } ;;
*)
	[ -r "$BIN" ] || { echo "binary $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	STAGE="$WORK/stage"
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" || exit 2
	cp "$BIN" "$STAGE/podbox" || exit 2
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_volmount "x155-$$" /v rw \
		|| { echo "driver inputs could not be staged" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	eng_pb "$WORK/pb-shim" 120 "$DRIVER" || exit 2
	BIN_RUN="$WORK/pb-shim"
	;;
esac

# rows FILE: the three root-listing verdicts plus the ptmx pair, by name.
# FILE travels through cygpath: jq here is a native Windows program and
# reads an MSYS /c/... path as C:\c\... (measured 2026-09-22, the same
# mangling the 95 script met on its build context).
show_rows() {
	_f="$1"
	if command -v cygpath >/dev/null 2>&1; then
		_f="$(cygpath -w "$1")"
	fi
	jq -c -r '.probes[] | select(.name == "readdir(/)" or .name == "open(/bin, O_RDONLY) by name" or .name == "creat(/, O_CREAT|O_EXCL)" or .name == "stat(/dev/ptmx)" or .name == "open(/dev/ptmx, O_RDWR)") | "\(.name) \(.verdict) errno=\(.errno_name // "-")"' "$_f" 2>/dev/null
}

echo "" >>"$WORK/report"
echo "== 1. the root-listing legs run and report" >>"$WORK/report"
if ! timeout 120 $BIN_RUN probe --json >"$WORK/probe.json" 2>"$WORK/probe.err" || ! show_rows "$WORK/probe.json" >"$WORK/rows" || [ ! -s "$WORK/rows" ]; then
	leg_not_run "probe --json" "no rows; stderr tail:"
	tail -5 "$WORK/probe.err" >>"$WORK/report"
else
	cat "$WORK/rows" >>"$WORK/report"
	for leg in "readdir(/)" "open(/bin, O_RDONLY) by name" "creat(/, O_CREAT|O_EXCL)"; do
		grep -q -F "$leg" "$WORK/rows" || { echo "  MISSING leg: $leg" >>"$WORK/report"; fail=1; }
	done
fi

echo "" >>"$WORK/report"
echo "== 2. the ptmx legs run and report" >>"$WORK/report"
for leg in "stat(/dev/ptmx)" "open(/dev/ptmx, O_RDWR)"; do
	grep -q -F "$leg" "$WORK/rows" 2>/dev/null || { echo "  MISSING leg: $leg" >>"$WORK/report"; fail=1; }
done
echo "  both ptmx rows present above" >>"$WORK/report"

echo "" >>"$WORK/report"
echo "== 3. a denied arm, where the machine stages one" >>"$WORK/report"
# An unprivileged run shows creat refused with its errno on a machine
# that permits everything to root. setpriv may be absent either place;
# that is a note, not a failure: the legs above already ran.
case "$(uname -s)" in
Linux) LANE=native ;;
*) LANE=driver ;;
esac
PRIV=""
if [ "$LANE" = "native" ]; then
	if command -v setpriv >/dev/null 2>&1; then
		PRIV="setpriv --reuid=nobody --regid=nogroup --clear-groups"
	else
		echo "  no setpriv on this host: the denied arm is not staged here" >>"$WORK/report"
	fi
fi
if [ -n "$PRIV" ]; then
	# shellcheck disable=SC2086
	if timeout 120 $PRIV $BIN_RUN probe --json >"$WORK/unpriv.json" 2>/dev/null && show_rows "$WORK/unpriv.json" >"$WORK/urows" && [ -s "$WORK/urows" ]; then
		cat "$WORK/urows" >>"$WORK/report"
	else
		echo "  unprivileged run gave no rows" >>"$WORK/report"
	fi
elif [ "$LANE" = "driver" ]; then
	if eng_run 120 "$DRIVER" "" -- /bin/sh -c 'command -v setpriv >/dev/null 2>&1'; then
		if eng_run 120 "$DRIVER" "" -- /bin/sh -c 'exec setpriv --reuid=nobody --regid=nogroup --clear-groups /pb probe --json' >"$WORK/unpriv.json" 2>/dev/null && show_rows "$WORK/unpriv.json" >"$WORK/urows" && [ -s "$WORK/urows" ]; then
			cat "$WORK/urows" >>"$WORK/report"
		else
			echo "  unprivileged run gave no rows" >>"$WORK/report"
		fi
	else
		echo "  no setpriv in the driver: the denied arm is not staged here" >>"$WORK/report"
	fi
else
	echo "  no setpriv on this host: the denied arm is not staged here" >>"$WORK/report"
fi

# TODO/complete.md T-0413 (the proc-absence clause). One image with bash:
# process substitution is a bash feature, and the M5 debian row is pinned in
# scripts/common/distro-matrix.sh.
#
# T-0413 emulation: the interposer answers pipe descriptors, the resolved
# exe path and mount-table reads where exactly answerable, and refuses the
# rest. So this clause drives both halves: a process-substitution payload
# that now SUCCEEDS through the emulation (4a), the failure that remains
# where emulation cannot apply (4b: a live-interface file), the exe
# passthrough (4c), the generated mount table (4d), the pipe-descriptor
# readlink shape (4e), and the standard stream spelling (4f). Each names
# what answered: the banner for the emulation, `/dev/fd` beside the banner
# for the failure.
# A fresh container-local store; readings land in /w on the shared scratch.
echo "" >>"$WORK/report"
echo "== 4. the chroot run through /proc emulation (T-0413)" >>"$WORK/report"
RUN_IMAGE='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
if [ "$LANE" = "native" ]; then
	STORE155="$WORK/store155"
	rm -rf "$STORE155"
	if PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" pull "$RUN_IMAGE" >"$WORK/pull155.log" 2>&1; then
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'cat <(echo hi)' >"$WORK/run155.out" 2>"$WORK/run155.err"
		echo "$?" >"$WORK/run155.rc"
		# T-0413 emulation arms beside the success arm: the failure where
		# emulation cannot apply, the exe passthrough, the generated mount
		# table and the pipe-descriptor readlink shape.
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'cat /proc/cpuinfo' >"$WORK/run155-cpuinfo.out" 2>"$WORK/run155-cpuinfo.err"
		echo "$?" >"$WORK/run155-cpuinfo.rc"
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'readlink /proc/self/exe' >"$WORK/run155-exe.out" 2>"$WORK/run155-exe.err"
		echo "$?" >"$WORK/run155-exe.rc"
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'cat /proc/mounts' >"$WORK/run155-mounts.out" 2>"$WORK/run155-mounts.err"
		echo "$?" >"$WORK/run155-mounts.rc"
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'echo hi | readlink /proc/self/fd/0' >"$WORK/run155-fdzero.out" 2>"$WORK/run155-fdzero.err"
		echo "$?" >"$WORK/run155-fdzero.rc"
		PODBOX_STORE="$STORE155" timeout 300 "$BIN_RUN" run --rm "$RUN_IMAGE" /bin/bash -c 'echo hi | cat /dev/stdin' >"$WORK/run155-stdin.out" 2>"$WORK/run155-stdin.err"
		echo "$?" >"$WORK/run155-stdin.rc"
	else
		leg_not_run "podbox pull $RUN_IMAGE" "see $WORK/pull155.log"
	fi
else
	cat >"$WORK/row155.sh" <<'ROW155_EOF'
#!/bin/sh
# One chroot run in the driver: pull, then a process-substitution payload.
# Stdout carries only stage failures; the readings land in /w/row155/.
set -u
export PODBOX_STORE=/tmp/ostore155
rm -rf /tmp/ostore155
if [ -f /w/cacert.pem ]; then
	export SSL_CERT_FILE=/w/cacert.pem
fi
d=/w/row155
mkdir -p "$d" || exit 6
rm -f "$d/out" "$d/err" "$d/rc"
/pb pull "$1" >"$d/pull.log" 2>&1 || exit 3
timeout 300 /pb run --rm "$1" /bin/bash -c 'cat <(echo hi)' >"$d/out" 2>"$d/err"
echo "$?" >"$d/rc"
exit 0
ROW155_EOF
	if [ ! -f "$WORK/w/cacert.pem" ]; then
		eng_run 600 "$DRIVER" "" -- /bin/sh -c 'DEBIAN_FRONTEND=noninteractive apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ca-certificates && cp /etc/ssl/certs/ca-certificates.crt /w/cacert.pem && test -s /w/cacert.pem' \
			>/dev/null 2>"$WORK/ca-install155.log" || leg_not_run "CA provision" "see $WORK/ca-install155.log"
	fi
	if [ ! -s "$WORK/worst" ]; then
		eng_mount "$WORK/row155.sh" /drv/row155.sh || leg_not_run "stage row155.sh" "mount refused"
	fi
	if [ ! -s "$WORK/worst" ]; then
		if eng_run 600 "$DRIVER" "" -- /bin/sh /drv/row155.sh "$RUN_IMAGE" 2>"$WORK/row155.stage"; then
			cp "$WORK/w/row155/out" "$WORK/run155.out" && cp "$WORK/w/row155/err" "$WORK/run155.err" && cp "$WORK/w/row155/rc" "$WORK/run155.rc" || leg_not_run "read back row155" "copy failed"
		else
			leg_not_run "row155 driver" "see $WORK/row155.stage"
		fi
	fi
fi
if [ -f "$WORK/run155.rc" ]; then
	rc="$(cat "$WORK/run155.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	case "$rc" in
	0) echo "  process substitution succeeded through the emulation" >>"$WORK/report" ;;
	*) echo "  FAIL: the process-substitution payload exited $rc; the pipe-descriptor emulation did not answer" >>"$WORK/report"; fail=1 ;;
	esac
	if [ "$(cat "$WORK/run155.out" 2>/dev/null)" = "hi" ]; then
		echo "  payload printed hi" >>"$WORK/report"
	else
		echo "  FAIL: the payload printed no hi; the pipe carried nothing" >>"$WORK/report"
		fail=1
	fi
	if grep -q 'Interpose.Emulated.procfs' "$WORK/run155.err" 2>/dev/null; then
		echo "  banner names the procfs emulation" >>"$WORK/report"
	else
		echo "  FAIL: the banner does not name Interpose.Emulated.procfs" >>"$WORK/report"
		fail=1
	fi
fi

# 4b. Where emulation cannot apply the failure stands and names /proc: a
# live-interface file no fixture may carry.
echo "" >>"$WORK/report"
echo "== 4b. what emulation cannot carry still fails naming /proc" >>"$WORK/report"
if [ -f "$WORK/run155-cpuinfo.rc" ]; then
	rc="$(cat "$WORK/run155-cpuinfo.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	case "$rc" in
	0) echo "  FAIL: reading /proc/cpuinfo succeeded; /proc may be mounted where none was expected" >>"$WORK/report"; fail=1 ;;
	*) echo "  live-interface read fails as it must" >>"$WORK/report" ;;
	esac
	if grep -q 'no /proc is mounted' "$WORK/run155-cpuinfo.err" 2>/dev/null; then
		echo "  banner names the missing procfs beside the failure" >>"$WORK/report"
	else
		echo "  FAIL: the banner does not name the missing procfs" >>"$WORK/report"
		fail=1
	fi
else
	echo "  not staged on this lane (native-only arm)" >>"$WORK/report"
fi

# 4c. The exe passthrough: the resolved guest path, as the kernel would print it.
echo "" >>"$WORK/report"
echo "== 4c. /proc/self/exe answers the resolved guest path" >>"$WORK/report"
if [ -f "$WORK/run155-exe.rc" ]; then
	rc="$(cat "$WORK/run155-exe.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	[ "$rc" = "0" ] || { echo "  FAIL: the exe readlink exited $rc" >>"$WORK/report"; fail=1; }
	if [ "$(cat "$WORK/run155-exe.out" 2>/dev/null)" = "/usr/bin/bash" ]; then
		echo "  exe answers /usr/bin/bash" >>"$WORK/report"
	else
		echo "  FAIL: the exe answer is not the resolved guest path: $(cat "$WORK/run155-exe.out" 2>/dev/null)" >>"$WORK/report"
		fail=1
	fi
else
	echo "  not staged on this lane (native-only arm)" >>"$WORK/report"
fi

# 4d. The generated mount table: the / line from live topology.
echo "" >>"$WORK/report"
echo "== 4d. /proc/mounts serves the generated fixture" >>"$WORK/report"
if [ -f "$WORK/run155-mounts.rc" ]; then
	rc="$(cat "$WORK/run155-mounts.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	[ "$rc" = "0" ] || { echo "  FAIL: reading /proc/mounts exited $rc" >>"$WORK/report"; fail=1; }
	if grep -q '^podbox / ' "$WORK/run155-mounts.out" 2>/dev/null; then
		echo "  fixture names the podbox root: $(grep '^podbox / ' "$WORK/run155-mounts.out" | head -1)" >>"$WORK/report"
	else
		echo "  FAIL: no podbox root line in the served table" >>"$WORK/report"
		fail=1
	fi
else
	echo "  not staged on this lane (native-only arm)" >>"$WORK/report"
fi

# 4e. The pipe-descriptor readlink shape: pipe:[ino], exactly.
echo "" >>"$WORK/report"
echo "== 4e. a pipe descriptor readlinks as pipe:[ino]" >>"$WORK/report"
if [ -f "$WORK/run155-fdzero.rc" ]; then
	rc="$(cat "$WORK/run155-fdzero.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	[ "$rc" = "0" ] || { echo "  FAIL: the fd readlink exited $rc" >>"$WORK/report"; fail=1; }
	if grep -q -E '^pipe:\[[0-9]+\]$' "$WORK/run155-fdzero.out" 2>/dev/null; then
		echo "  fd 0 readlinks as $(cat "$WORK/run155-fdzero.out")" >>"$WORK/report"
	else
		echo "  FAIL: no pipe:[ino] shape: $(cat "$WORK/run155-fdzero.out" 2>/dev/null)" >>"$WORK/report"
		fail=1
	fi
else
	echo "  not staged on this lane (native-only arm)" >>"$WORK/report"
fi

# 4f. The standard stream spelling opens the descriptor: cat reads stdin
# through /dev/stdin exactly as through fd 0.
echo "" >>"$WORK/report"
echo "== 4f. /dev/stdin opens descriptor 0" >>"$WORK/report"
if [ -f "$WORK/run155-stdin.rc" ]; then
	rc="$(cat "$WORK/run155-stdin.rc")"
	echo "  payload exit: $rc" >>"$WORK/report"
	[ "$rc" = "0" ] || { echo "  FAIL: cat /dev/stdin exited $rc" >>"$WORK/report"; fail=1; }
	if [ "$(cat "$WORK/run155-stdin.out" 2>/dev/null)" = "hi" ]; then
		echo "  cat /dev/stdin printed hi" >>"$WORK/report"
	else
		echo "  FAIL: cat /dev/stdin printed no hi" >>"$WORK/report"
		fail=1
	fi
else
	echo "  not staged on this lane (native-only arm)" >>"$WORK/report"
fi

# 5. T-0414: a tree copy that must list an unreadable directory refuses
# naming the listing, not a missing file. Staged as nobody (the lane runs
# as root, for whom no listing fails): the walked directory allows
# traversal (711) but denies listing, so metadata succeeds and the walk's
# read_dir is what fails with EACCES.
echo "" >>"$WORK/report"
echo "== 5. a denied listing refuses naming readdir (T-0414)" >>"$WORK/report"
if [ "$LANE" = "native" ] && command -v setpriv >/dev/null 2>&1; then
	NOBODY_STORE="$WORK/nobody-store"
	NOBODY_HOME="$WORK/nobody-home"
	rm -rf "$NOBODY_STORE" "$NOBODY_HOME" /tmp/pb155-nolist
	mkdir -p "$NOBODY_STORE" "$NOBODY_HOME" /tmp/pb155-nolist/inner || exit 2
	echo data > /tmp/pb155-nolist/inner/f.txt
	chmod 711 /tmp/pb155-nolist
	chmod 755 "$NOBODY_STORE" "$NOBODY_HOME"
	PRIV="setpriv --reuid=65534 --regid=65534 --clear-groups"
	# ⚠ A real unprivileged user brings its own home: without HOME the
	# binary reads root's config path and dies on it before any pull
	# (measured 2026-09-26: 125 naming /root/.config), which stages a
	# config failure rather than the denied listing.
	# ⚠ The pull and create run with the privilege those operations
	# need (create enters through chroot, which nobody is denied), and
	# only the copy sheds privilege: everything before the tree walk
	# is a metadata read, so the walk's listing is what fails first.
	# ⚠ The store goes to nobody AFTER the privileged pair: root's
	# pull and create leave a root-owned lock file behind, and a copy
	# that cannot open the lock dies 125 naming EACCES before any
	# listing (measured 2026-09-26).
	if env HOME="$NOBODY_HOME" PODBOX_STORE="$NOBODY_STORE" "$BIN_RUN" pull "$RUN_IMAGE" >"$WORK/pull155-nobody.log" 2>&1 \
	&& env HOME="$NOBODY_HOME" PODBOX_STORE="$NOBODY_STORE" "$BIN_RUN" create --name nolistc "$RUN_IMAGE" >/dev/null 2>&1 \
	&& chown -R 65534:65534 "$NOBODY_STORE" "$NOBODY_HOME" 2>/dev/null; then
		# shellcheck disable=SC2086
		$PRIV env HOME="$NOBODY_HOME" PODBOX_STORE="$NOBODY_STORE" "$BIN_RUN" cp -r /tmp/pb155-nolist nolistc:/dst >"$WORK/cp155.out" 2>"$WORK/cp155.err"
		echo "$?" >"$WORK/cp155.rc"
		rc="$(cat "$WORK/cp155.rc")"
		echo "  cp exit: $rc" >>"$WORK/report"
		[ "$rc" = "125" ] || { echo "  FAIL: the denied listing exited $rc, not docker's 125" >>"$WORK/report"; fail=1; }
		if grep -q 'cannot list /tmp/pb155-nolist' "$WORK/cp155.err" 2>/dev/null && grep -q -i 'permission denied' "$WORK/cp155.err" 2>/dev/null; then
			echo "  refusal names the listing and EACCES: $(head -1 "$WORK/cp155.err")" >>"$WORK/report"
		else
			echo "  FAIL: the refusal names neither the listing nor EACCES" >>"$WORK/report"
			fail=1
		fi
	else
		leg_not_run "pull+create for the nobody copy" "see $WORK/pull155-nobody.log"
	fi
	rm -rf /tmp/pb155-nolist
else
	echo "  not staged on this lane (needs native setpriv)" >>"$WORK/report"
fi

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo ""
echo "written to ${OUT#"$REPO"/}"
[ -s "$WORK/worst" ] && exit 2
[ "$fail" -eq 0 ] || exit 1
exit 0
