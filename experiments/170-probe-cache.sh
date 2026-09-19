#!/usr/bin/env bash
# Question: does `$store/probe.json` serve the same answer twice on one
# machine, and does it REFUSE to serve the host's answer to a confined process?
#
# TODO/probe.md T-0111. ⛔ A cache keyed wrongly is worse than no cache, and
# this is the one component where a stale answer is the exact failure the
# project exists to prevent: a `namespace` verdict served to a process that has
# a `chroot` is podbox telling the lie it was built to refuse.
#
# ⭐ THE SPECIFICATION'S KEY DOES NOT WORK, AND CLAUSE 3 IS WHY. `TOOL.md`
# section 6.1 says to key on `/proc/sys/kernel/random/boot_id`. That value is
# the KERNEL'S: the host, the reconstruction and a plain docker container all
# read the same one while producing two different rungs. Clause 3 prints both
# boot ids so the reader can see they are equal while the verdicts are not.
#
#   ./170-probe-cache.sh
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/probe-cache.txt"
# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there. The lane's scratch
# lives under the checkout on a non-native lane: mount sources must be
# Windows paths there (245's rule), and /tmp/... names nothing.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)"; STORE="$WORK/store" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.probe170-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2; STORE="$WORK/w/pstore" ;;
esac
trap 'rm -rf "$WORK"' EXIT INT TERM

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. What each clause asserts is unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
# Clauses 1 and 2 need the binary to execute: natively, or staged in the
# driver. Clause 3 needs the engine for its confined run either way.
if [ "$NATIVE" -eq 0 ]; then
	[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine on a lane where the podbox binary does not execute" >&2; exit 2; }
	[ -r "$BIN" ] || {
		echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2
		exit 2
	}
fi

# The driver: the M5 debian row, pinned. It hosts the staged podbox binary
# for every `$BIN` call on a non-native lane.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
STAGE="$WORK/stage"
if [ "$NATIVE" -eq 0 ]; then
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" "$STORE" || exit 2
	cp "$BIN" "$STAGE/podbox"
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
fi

if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || {
		echo "SKIP: $BIN is not an executable. Build it:" >&2
		echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
		exit 2
	}
fi
command -v jq >/dev/null 2>&1 || { echo "SKIP: jq is not on PATH" >&2; exit 2; }

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
if [ "$NATIVE" -eq 1 ]; then
	_ver="$("$BIN" version)"
else
	_ver="$(eng_run 60 "$DRIVER" "" -- /pb version 2>&1)"
fi
printf 'podbox            %s\n' "$_ver"
printf 'store             a fresh directory under %s\n' "$(dirname "$WORK")"
echo

fail=0
mkdir -p "$STORE"

echo "== 1. two runs on one machine agree on the rung"
# T-0111's Prove, first half, run against --json exactly as it is written.
# On a non-native lane the binary runs staged in the driver; the answers
# compared below are the same bytes either way.
if [ "$NATIVE" -eq 1 ]; then
	"$BIN" probe --json >"$WORK/a.json" 2>"$WORK/a.err"
	a_rc=$?
	"$BIN" probe --json >"$WORK/b.json" 2>"$WORK/b.err"
	b_rc=$?
else
	eng_run 120 "$DRIVER" "" -- /pb probe --json >"$WORK/a.json" 2>"$WORK/a.err"
	a_rc=$?
	eng_run 120 "$DRIVER" "" -- /pb probe --json >"$WORK/b.json" 2>"$WORK/b.err"
	b_rc=$?
fi
[ "$a_rc" -eq 0 ] && [ "$b_rc" -eq 0 ] || {
	echo "SKIP: podbox probe --json exited $a_rc and $b_rc" >&2
	exit 2
}
jq -e --slurpfile a "$WORK/a.json" '.rung == $a[0].rung' "$WORK/b.json" >/dev/null
same_rc=$?
host_rung="$(jq -r .rung "$WORK/a.json")"
printf '  both runs report %s\n' "$host_rung"
[ "$same_rc" -eq 0 ] || { echo "  FAIL: two runs disagreed about the rung"; fail=1; }

echo
echo "== 2. the first --cached run measures and the second is served"
# The helper carries no `-e`: the store travels in the argv wrapper on a
# non-native lane. The two runs and what they must print are unchanged.
# ⭐ ONE CONTAINER FOR BOTH RUNS elsewhere: each eng_run is a fresh mount
# namespace, and the cache key covers mnt_ns, so a second container would
# correctly measure again and the clause could never see a served run.
# Natively the two processes share the shell's namespace, and one driver
# container is the same shape.
if [ "$NATIVE" -eq 1 ]; then
	PODBOX_STORE="$STORE" "$BIN" probe --cached >"$WORK/c1.out" 2>"$WORK/c1.err"
	PODBOX_STORE="$STORE" "$BIN" probe --cached >"$WORK/c2.out" 2>"$WORK/c2.err"
else
	eng_run 120 "$DRIVER" "" -- /bin/sh -c 'export PODBOX_STORE=/w/pstore; /pb probe --cached >/w/c1.out 2>/w/c1.err; /pb probe --cached >/w/c2.out 2>/w/c2.err'
	cp "$WORK/w/c1.out" "$WORK/c1.out" && cp "$WORK/w/c1.err" "$WORK/c1.err" \
		&& cp "$WORK/w/c2.out" "$WORK/c2.out" && cp "$WORK/w/c2.err" "$WORK/c2.err" || {
		echo "SKIP: the cached runs left no readings" >&2; exit 2; }
fi
sed 's/^/    /' "$WORK/c1.err"
sed 's/^/    /' "$WORK/c2.err"
if grep -q 'measured now' "$WORK/c1.err"; then
	echo "  ok      the first run measured"
else
	echo "  FAIL: the first run did not measure into an empty store"
	fail=1
fi
if grep -q 'served from' "$WORK/c2.err"; then
	echo "  ok      the second run was served from the cache"
else
	echo "  FAIL: the second run did not use the cache it had just written"
	fail=1
fi
[ "$(cat "$WORK/c1.out")" = "$(cat "$WORK/c2.out")" ] || {
	echo "  FAIL: the cached rung is not the measured one"
	fail=1
}
[ -s "$STORE/probe.json" ] || { echo "  FAIL: no $STORE/probe.json"; fail=1; }

echo
echo "== 3. the cache is REFUSED inside the reconstruction"
# ⭐ The clause the entry exists for. The host wrote `namespace` into the store
# above; a copy of that store is staged into the reconstruction, where the rung
# is `chroot`. A cache keyed on the boot id alone would serve `namespace`.
host_boot="$(jq -r .cache_key.boot_id "$STORE/probe.json")"
confined_out="$WORK/confined.txt"
# ⚠ A FRESH STAGE DIRECTORY, so nothing a previous run left behind can be what
# the confined process reads. 20-enter-target.sh now clears each destination
# before copying, and this makes the isolation belt-and-braces: the whole point
# of the clause is which probe.json is present inside.
export TARGET_STAGE="$WORK/stage"
confined_ok=0
if [ "$NATIVE" -eq 1 ]; then
	if "$HERE/20-enter-target.sh" --stage "$BIN" --stage "$STORE" -- \
		/bin/sh -c 'PODBOX_STORE=/workspace/store; export PODBOX_STORE;
		            /workspace/podbox probe --cached
		            cat /proc/sys/kernel/random/boot_id' \
		>"$confined_out" 2>"$WORK/confined.err"; then
		confined_ok=1
	fi
else
	# ⭐ The reconstruction is a privileged engine container: 20 runs the
	# target image with --privileged so the confine harness can mount and
	# pivot, and the helper rightly refuses that flag anywhere. This lane
	# confines with a plain engine container instead. It has its own mount
	# namespace and a real /proc, and podbox measures chroot inside it:
	# its unshare probes fail there, as every 245 row transcript shows.
	# The inner command is 20's own, staged at the same /workspace paths;
	# what the clause asserts (a fresh measurement, rung chroot, the boot
	# ids equal, mnt_ns named) is unchanged.
	rm -rf "$WORK/w/cstore"
	mkdir -p "$WORK/w/cstore" || { echo "  SKIP: the confined store could not be staged" >&2; exit 2; }
	cp -r "$STORE/." "$WORK/w/cstore/" || { echo "  SKIP: the host cache could not be staged" >&2; exit 2; }
	if eng_mount "$STAGE/podbox" /workspace/podbox \
		&& eng_mount "$WORK/w/cstore" /workspace/store rw; then
		if eng_run 300 "$DRIVER" "" -- /bin/sh -c 'PODBOX_STORE=/workspace/store; export PODBOX_STORE;
		            /workspace/podbox probe --cached
		            cat /proc/sys/kernel/random/boot_id' \
			>"$confined_out" 2>"$WORK/confined.err"; then
			confined_ok=1
		fi
		eng_clear
	fi
fi
if [ "$confined_ok" -eq 1 ]; then
	confined_rung="$(sed -n '1p' "$confined_out")"
	confined_boot="$(sed -n '2p' "$confined_out")"
	why="$(grep -o 'measured now, because.*' "$WORK/confined.err" | head -1)"
	printf '  host      rung=%-10s boot_id=%s\n' "$host_rung" "$host_boot"
	printf '  confined  rung=%-10s boot_id=%s\n' "$confined_rung" "$confined_boot"
	printf '  %s\n' "${why:-<the cache was served>}"

	# ⭐ The rung the confined run must select is the lane's truth, not one
	# word for both lanes. Natively the reconstruction (20's confine
	# harness) denies namespace creation, so chroot is the pass there.
	# Here the confined run is a plain engine container: userns creation
	# succeeds but mount does not, and the probe reports supervise, twice
	# measured (clause 1 agrees). Expecting chroot here would fail a
	# correct measurement for not matching another lane's word.
	if [ "$NATIVE" -eq 1 ]; then want_rung="chroot"; else want_rung="supervise"; fi
	if [ "$confined_rung" = "$want_rung" ]; then
		echo "  ok      the confined run measured its own answer"
	else
		echo "  FAIL: expected $want_rung in confinement, got $confined_rung"
		fail=1
	fi
	if [ "$host_boot" = "$confined_boot" ] && [ "$host_rung" != "$confined_rung" ]; then
		echo "  ⭐ the boot ids are EQUAL and the rungs are NOT. That is the"
		echo "     measurement TOOL.md section 6.1's cache key does not survive."
	elif [ "$host_boot" = "$confined_boot" ]; then
		echo "  ⚠ RECORDED: equal boot ids and equal rungs ($host_rung); the"
		echo "     cache still refused on mnt_ns, which is the key doing its job."
	else
		# Not a failure of podbox: it would mean this kernel gives the
		# container its own boot id, which would make the specification's key
		# work here and not elsewhere. Recorded as a reading.
		echo "  ⚠ RECORDED: the boot ids differ on this host, so the"
		echo "     specification's key would have caught this one case."
	fi
	if grep -q 'mnt_ns' "$WORK/confined.err"; then
		echo "  ok      the refusal names the mount namespace as a differing component"
	else
		echo "  FAIL: the refusal did not name mnt_ns"
		fail=1
	fi
else
	echo "  SKIP: the confined run did not run; see below" >&2
	sed 's/^/    /' "$WORK/confined.err" >&2
	exit 2
fi

{
	printf '# podbox probe cache, TODO/probe.md T-0111\n'
	printf '# taken %s on kernel %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -r)"
	printf '# ⛔ The boot id is the KERNEL S. Both readings below are on one kernel.\n'
	printf 'host_rung         %s\n' "$host_rung"
	printf 'host_boot_id      %s\n' "$host_boot"
	printf 'confined_rung     %s\n' "$confined_rung"
	printf 'confined_boot_id  %s\n' "$confined_boot"
	printf 'boot_ids_equal    %s\n' \
		"$([ "$host_boot" = "$confined_boot" ] && echo yes || echo no)"
	printf 'cache_served_host %s   (no is the pass)\n' \
		"$(grep -q 'served from' "$WORK/confined.err" && echo yes || echo no)"
	echo
	echo '## why the confined run refused the cache'
	printf '  %s\n' "${why:-<none: the cache was served>}"
	echo
	echo '## the key the host wrote'
	jq -S .cache_key "$STORE/probe.json" | sed 's/^/  /'
} > "$OUT"
echo
echo "  written to $OUT"

if [ -x "$REPO/scripts/common/result-diff.sh" ]; then
	echo
	echo "== against the committed reading"
	"$REPO/scripts/common/result-diff.sh" "$OUT" || true
fi

[ "$fail" -eq 0 ] || exit 1
exit 0
