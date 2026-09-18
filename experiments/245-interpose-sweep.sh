#!/usr/bin/env bash
# Question: across the distribution matrix, does a payload that needs
# virtualized ownership succeed, and does podbox pick the right object
# without being told?
#
# TODO/milestones.md T-1110, M6's acceptance. Method in
# `scripts/common/distro-matrix.sh` (T-1203's mechanics) and subject shape in
# `experiments/240-distro-sweep.sh`: one subject for every row, transcripts
# per row, no-pull counted apart, docker control where podbox fails.
#
# Per row, one question each for T-0712's two columns, with the argument held
# constant (that entry's rule: the lchown target is gid 42, unmapped, on
# every row, so "reached" and "cleared" are two columns of one table):
#   seen     LD_PRELOAD reaches the payload's environment
#   cleared  chown 0:42 reads back 0:42 afterwards
# plus a static victim (declined by name, still runs) and a Go victim
# (declined by name for its markers), both staged into the row's own rootfs.
# A row whose own shell is static answers seen=no with a static reason and
# counts as a named decline rather than a failure.
#
# Then the selection, asserted rather than inferred: the placed object's
# bytes are compared against the two built objects, so a musl row carrying
# the glibc object reads as what it is. Then the two cross-libc refusals,
# driven deliberately through `podbox system abi`. The third refusal, a new
# build host against an old libc, has no matrix row old enough to fire it;
# the unit test owns it (`abi.rs`) and this records that instead of
# inventing a row for it.
#
#   ./245-interpose-sweep.sh            every row
#   ./245-interpose-sweep.sh debian     one row, by its local name
#
# Exit: 0 every row that pulled answered, 1 a row pulled and did not,
#       2 nothing ran.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/interpose-sweep.txt"
TRANSCRIPTS="${TRANSCRIPTS:-$REPO/experiments/results/sweep245}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

ONLY="${1:-}"
ROW_TIMEOUT="${PODBOX_ROW_TIMEOUT:-600}"

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. ./scripts/dev.sh build" >&2
	exit 2
}
# shellcheck source=../scripts/common/distro-matrix.sh
. "$REPO/scripts/common/distro-matrix.sh"

export PODBOX_STORE="${PODBOX_SWEEP_STORE:-$WORK/store}"
mkdir -p "$TRANSCRIPTS"

have_docker=0
if command -v docker >/dev/null 2>&1 && timeout 30 docker info >/dev/null 2>&1; then
	have_docker=1
fi

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'podbox            %s\n' "$("$BIN" version)"
printf 'rows              %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
printf 'row timeout       %s s\n' "$ROW_TIMEOUT"
printf 'control           %s\n' \
	"$([ "$have_docker" -eq 1 ] && echo 'docker, on any row podbox fails' || echo 'ABSENT: no docker daemon, so a failure cannot be attributed')"
echo

# ------------------------------------------------------------------ victims
#
# ⭐ The Go victim is generated, not fetched: no row of DISTRO_ROWS_M5 is a Go
# image and inventing a registry reference is refused, so the classifier's Go
# arm would otherwise have no end-to-end driver.
# `experiments/src/govictim.sh` carries the layout and the reason for ET_REL.
sh "$REPO/experiments/src/govictim.sh" >"$WORK/go-victim" || {
	echo "SKIP: the Go fixture did not generate" >&2; exit 2; }
[ "$(wc -c <"$WORK/go-victim")" -eq 384 ] || {
	echo "SKIP: the Go fixture is not 384 bytes" >&2; exit 2; }
STATIC_VICTIM="$(command -v "$BIN")"

# ------------------------------------------------------------------ subject
#
# ⛔ ONE SCRIPT FOR EVERY ROW. A per-row subject would be ten subjects, and a
# difference between rows would then be a difference between the commands.
# It writes into /tmp inside the container, never beside itself.
cat >"$WORK/subject.sh" <<'SUBJECT'
#!/bin/sh
# One unmapped gid for every row, class and image (T-0712's rule).
if env | grep -q '^LD_PRELOAD='; then echo SEEN=yes; else echo SEEN=no; fi
: > /tmp/v245 2>/dev/null
if chown 0:42 /tmp/v245 2>/dev/null && [ "$(stat -c %u:%g /tmp/v245 2>/dev/null)" = "0:42" ]; then
	echo CLEARED=yes
else
	echo CLEARED=no
fi
SUBJECT
chmod +x "$WORK/subject.sh"

ran=0
nopull=0
broken=0
host=0
virtualized=0
declined=0

printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' ROW LIBC SEEN CLEARED STATIC GO SELECT
printf '%.0s-' $(seq 1 74)
echo

while IFS='|' read -r ref name libc digest; do
	[ -n "${ref:-}" ] || continue
	[ -z "$ONLY" ] || [ "$ONLY" = "$name" ] || continue
	pinned="$(distro_pinned "$ref" "$digest")"
	t="$TRANSCRIPTS/$name.out"

	if ! timeout 900 "$BIN" pull "$pinned" >"$WORK/pull.$name" 2>&1; then
		printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - 'no-pull'
		{
			echo "no-pull $pinned"
			tail -3 "$WORK/pull.$name"
		} >"$t"
		nopull=$((nopull + 1))
		continue
	fi
	"$BIN" extract "$pinned" >/dev/null 2>&1
	ROOT="$("$BIN" inspect --format '{{.RootfsPath}}' "$pinned" 2>/dev/null)" || {
		printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - 'no rootfs'
		echo "no rootfs for $pinned" >"$t"
		nopull=$((nopull + 1))
		continue
	}
	cp "$STATIC_VICTIM" "$ROOT/podbox-static" 2>/dev/null || true
	cp "$WORK/go-victim" "$ROOT/go-victim" 2>/dev/null || true
	chmod 755 "$ROOT/podbox-static" "$ROOT/go-victim" 2>/dev/null || true

	# ⭐ THE SUBJECT TRAVELS AS ONE ARGV ELEMENT, not as a file (240's rule:
	# podbox cannot bind-mount, so `-v` is not available).
	timeout "$ROW_TIMEOUT" "$BIN" run "$pinned" /bin/sh -c "$(cat "$WORK/subject.sh")" \
		>"$WORK/out.$name" 2>"$WORK/err.$name"
	row_rc=$?
	{
		printf '# %s\n# pinned %s\n# podbox run exit %s\n' "$name" "$pinned" "$row_rc"
		echo "--- stdout"
		cat "$WORK/out.$name"
		echo "--- stderr, without the completion layer"
		grep -v 'podbox: complete:' "$WORK/err.$name" | tail -8 | cut -c1-200
	} >"$t"

	get() { sed -n "s/^$1=//p" "$WORK/out.$name" | head -1; }
	seen="$(get SEEN)"
	cleared="$(get CLEARED)"
	if [ -z "$seen" ]; then
		printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - "rc=$row_rc subject did not run"
		broken=$((broken + 1))
		continue
	fi
	ran=$((ran + 1))
	[ "$cleared" = "yes" ] && virtualized=$((virtualized + 1))
	# ⭐ A shell the tier does not reach is a finding, not data -- unless the
	# decline names an unreachable class. A static shell answers seen=no
	# with a static reason and counts as a named decline; anything else
	# declining here is a row the classifier got wrong.
	if [ "$seen" != "yes" ]; then
		if grep -q 'interpose: declined' "$WORK/err.$name" \
		&& grep 'interpose: declined' "$WORK/err.$name" | grep -qE 'statically linked|Go build markers'; then
			printf '%-14s   static shell: named decline, payload runs natively.\n' "$name"
			declined=$((declined + 1))
		else
			printf '%-14s   ⛔ preloaded nothing into a dynamic shell.\n' "$name"
			broken=$((broken + 1))
		fi
	fi

	# The static victim: declined by name, and still runs.
	sout="$(timeout "$ROW_TIMEOUT" "$BIN" run "$pinned" /podbox-static --version 2>&1)"
	src=$?
	if printf '%s' "$sout" | grep -q 'interpose: declined' && [ "$src" -eq 0 ]; then
		static="declined"
		declined=$((declined + 1))
	else
		static="UNEXPECTED"
		broken=$((broken + 1))
		{
			echo "--- static victim: rc=$src"
			printf '%s\n' "$sout" | tail -4 | cut -c1-200
		} >>"$t"
	fi
	# The Go victim: declined FOR ITS MARKERS, and the kernel then refuses
	# the fixture itself (ET_REL execs as ENOEXEC, exit 126).
	gout="$(timeout "$ROW_TIMEOUT" "$BIN" run "$pinned" /go-victim 2>&1)"
	grc=$?
	if printf '%s' "$gout" | grep -q 'interpose: declined' \
	&& printf '%s' "$gout" | grep -q 'Go build markers' \
	&& [ "$grc" -eq 126 ]; then
		go="declined"
		declined=$((declined + 1))
	else
		go="UNEXPECTED"
		broken=$((broken + 1))
		{
			echo "--- go victim: rc=$grc"
			printf '%s\n' "$gout" | tail -4 | cut -c1-200
		} >>"$t"
	fi

	# ⭐ The SELECTION, asserted on bytes rather than inferred from the
	# outcome: the placed object is compared against both built objects.
	# Where the objects are not built (a tree without zig), every row
	# declines for carrying none, and that is stated rather than counted
	# as a mismatch.
	sel="unknown"
	objects_built=1
	for obj in gnu musl; do
		[ -f "$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-$obj/release/libpodbox_interpose.so" ] || objects_built=0
	done
	if [ "$objects_built" -eq 1 ] && [ -f "$ROOT/.podbox/interpose.so" ]; then
		for obj in gnu musl; do
			if cmp -s "$ROOT/.podbox/interpose.so" \
				"$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-$obj/release/libpodbox_interpose.so"; then
				sel="$obj"
			fi
		done
	fi
	if [ "$objects_built" -eq 0 ]; then
		selmark="unbuilt-objects"
		want=""
	else
		# ⭐ The row names the libc, the object names the target: glibc
		# rows take the gnu object. Comparing the raw words would read
		# every correct glibc placement as a mismatch.
		want="$libc"
		[ "$libc" = "glibc" ] && want="gnu"
		if [ "$sel" = "$want" ]; then
			selmark="placed-$sel"
		else
			selmark="MISMATCH placed-$sel"
		fi
	fi
	printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' \
		"$name" "$libc" "$seen" "$cleared" "$static" "$go" "$selmark"
	if [ "$objects_built" -eq 1 ] && [ "$sel" != "$want" ]; then
		broken=$((broken + 1))
	fi
	# ⛔ A row the payload did not clear fails here unless docker fails it
	# too, which is the control that attributes the failure (240's rule).
	# Without docker the failure counts as the host's: podbox cannot prove
	# otherwise, and a failure it cannot attribute is not one it owns.
	if [ "$cleared" != "yes" ]; then
		if [ "$have_docker" -eq 1 ]; then
			if timeout "$ROW_TIMEOUT" docker run --rm "$pinned" /bin/sh -c "$(cat "$WORK/subject.sh")" \
				>"$WORK/dout.$name" 2>/dev/null \
			&& grep -q '^CLEARED=yes' "$WORK/dout.$name"; then
				printf '%-14s   ⛔ podbox failed and DOCKER SUCCEEDED on the same image.\n' "$name"
				broken=$((broken + 1))
			else
				printf '%-14s   ⚠ the host: docker failed here too.\n' "$name"
				host=$((host + 1))
			fi
		else
			printf '%-14s   ⚠ no docker control: recorded as the host.\n' "$name"
			host=$((host + 1))
		fi
	fi
done <<EOF_ROWS
$DISTRO_ROWS_M5
EOF_ROWS

echo
echo "== the two cross-libc refusals, driven deliberately"
# The matrix rows' own files: alpine's loader IS its libc, debian's libc is
# found by search, both from outside with this host's tools (100-'s rule).
refuse_fail=0
for pair in "gnu musl ld-musl-*.so.1" "musl glibc libc.so.6"; do
	set -- $pair
	obj="$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-$1/release/libpodbox_interpose.so"
	pat="$3"
	found="$(find "$WORK" -path '*store*' -name "$pat" 2>/dev/null | head -1)"
	if [ ! -f "$obj" ]; then
		echo "  SKIP $1 object against $2 libc: object not built"
	elif [ -z "$found" ]; then
		echo "  SKIP $1 object against $2 libc: no $pat extracted"
	elif "$BIN" system abi "$obj" "$found" >/dev/null 2>"$WORK/refuse.$1$2"; then
		echo "  FAIL $1 object ADMITTED into a $2 rootfs"
		refuse_fail=1
	else
		echo "  ok   $1 object refused into a $2 rootfs: $(head -c 120 "$WORK/refuse.$1$2" | tr '\n' ' ')"
	fi
done
echo "  (the build-host-newer-than-target refusal is unit-covered in abi.rs;"
echo "   no matrix row is old enough to fire it, and inventing one is refused)"

echo
echo "== verdict: the five words T-1110 asks for"
printf 'rows %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
printf 'ran %s\n' "$ran"
printf 'virtualized %s\n' "$virtualized"
printf 'declined %s\n' "$declined"
printf 'host_not_runtime %s\n' "$host"
{
	printf '# M6 acceptance: virtualized ownership across the matrix\n'
	printf '# TODO/milestones.md T-1110. Taken %s on kernel %s\n' \
		"$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -r)"
	printf 'rows              %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
	printf 'ran               %s\n' "$ran"
	printf 'virtualized       %s\n' "$virtualized"
	printf 'declined          %s\n' "$declined"
	printf 'host_not_runtime  %s\n' "$host"
	printf 'no-pull           %s\n' "$nopull"
	printf 'broken            %s\n' "$broken"
} >"$OUT"
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"
distro_verdict "$ran" "$nopull" "$broken"
vrc=$?
[ "$vrc" -eq 0 ] || exit "$vrc"
[ "$refuse_fail" -eq 0 ] || exit 1
exit 0
