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
# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. Fetches happen before any timed clause, every
# run is bounded and `--rm`, mounts are read-only from declared roots, and
# `--privileged` and `--cap-add` are refused outright. On a lane where the
# Linux podbox binary does not execute, every `$BIN` call runs staged inside
# a driver container (the M5 debian row) with the store on shared scratch,
# under `--cap-drop=CHOWN` so a cleared chown still measures the interposer
# rather than the kernel; what each row asserts is unchanged.
#
# Exit: 0 every row that pulled answered, 1 a row pulled and did not,
#       2 nothing ran.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
TRANSCRIPTS="${TRANSCRIPTS:-$REPO/experiments/results/sweep245}"
OUT="${OUT:-$REPO/experiments/results/interpose-sweep.txt}"

ONLY="${1:-}"
ROW_TIMEOUT="${PODBOX_ROW_TIMEOUT:-600}"

# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there.
case "$(uname -s)" in
  Linux) NATIVE=1 ;;
  *)     NATIVE=0 ;;
esac

# ⭐ Mount sources must be Windows paths on a non-native lane: the helper
# hands them to a Windows engine binary verbatim, and a /tmp/... spelling
# names nothing there. The lane's scratch therefore lives under the checkout
# (ignored by the experiments/.*/ rule) instead of under /tmp.
if [ "$NATIVE" -eq 1 ]; then
  WORK="$(mktemp -d)"
else
  WORK="$REPO/experiments/.sweep245-work"
  rm -rf "$WORK"
  mkdir -p "$WORK" || exit 2
fi
trap 'rm -rf "$WORK"' EXIT INT TERM

# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
# ⭐ Soft: a Linux lane without an engine still drives podbox natively and
# only loses the control. Anywhere else the driver IS the lane, so no
# engine means nothing can run.
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$NATIVE" -eq 1 ] || [ "$HAVE_ENGINE" -eq 1 ] || {
  echo "SKIP: no engine (a docker daemon or host podman) on a lane where the podbox binary does not execute" >&2
  exit 2
}

# The driver: the M5 debian row, pinned. It hosts the staged podbox binary
# for every `$BIN` call on a non-native lane.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
# ⭐ The wall, rebuilt around podbox. The driver is rootful, so a payload
# chown would succeed natively and every row would clear for no reason;
# without CHOWN the interposer is the only way 0:42 reads back.
DRIVER_CAPS='--cap-drop=CHOWN'
# The driver's bound covers a first pull plus the row's runs; the row's own
# runs carry ROW_TIMEOUT inside, as before.
OUTER="$((ROW_TIMEOUT + 900))"
STAGE="$WORK/stage"

# The object file for a word, honouring the environment overrides above.
objfile() { # gnu|musl -> path on stdout
  case "$1" in
    gnu) printf '%s' "$GNU_SO" ;;
    musl) printf '%s' "$MUSL_SO" ;;
  esac
}

# ⭐ The refusal check travels with the binary: directly where it
# executes, staged beside its two inputs elsewhere.
abi_check() { # WORD OBJPATH LIBCPATH ERRFILE -> 0 admitted
  if [ "$NATIVE" -eq 1 ]; then
    "$BIN" system abi "$2" "$3" >/dev/null 2>"$4"
  else
    eng_mount "$STAGE/podbox" /pb && eng_mount "$STAGE/$1.so" /obj && eng_mount "$3" /lc || {
      echo "STAGE MOUNT FAILED" >"$4"
      eng_clear
      return 1
    }
    eng_run 60 "$DRIVER" "" -- /pb system abi /obj /lc >/dev/null 2>"$4"
    r=$?
    eng_clear
    return "$r"
  fi
}

# The two interpose objects the selection is compared against. Environment
# overrides point at guest build artifacts on lanes where the tree never
# built them itself; the defaults are what a Linux lane builds in place.
GNU_SO="${GNU_SO:-$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so}"
MUSL_SO="${MUSL_SO:-$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so}"

# ⭐ Readability, not executability, is what is asked on a non-native lane:
# the binary runs staged inside the driver there.
case "$(uname -s)" in
  Linux) [ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. ./scripts/dev.sh build" >&2
	exit 2
} ;;
  *) [ -r "$BIN" ] || {
	echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2
	exit 2
} ;;
esac
# shellcheck source=../scripts/common/distro-matrix.sh
. "$REPO/scripts/common/distro-matrix.sh"

export PODBOX_STORE="${PODBOX_SWEEP_STORE:-$WORK/store}"
mkdir -p "$TRANSCRIPTS"


echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
# The version travels with the binary: directly where it executes,
# staged inside the already-pulled driver elsewhere.
if [ "$NATIVE" -eq 1 ]; then
  _ver="$("$BIN" version)"
else
  eng_pull "$DRIVER" || exit 2
  mkdir -p "$STAGE"
  cp "$BIN" "$STAGE/podbox"
  eng_mount "$STAGE/podbox" /pb || exit 2
  _ver="$(eng_run 60 "$DRIVER" "" -- /pb version 2>&1)"
  eng_clear
fi
printf 'podbox            %s\n' "$_ver"
printf 'rows              %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
printf 'row timeout       %s s\n' "$ROW_TIMEOUT"
printf 'control           %s\n' \
	"$([ "$HAVE_ENGINE" -eq 1 ] && echo "$ENGINE_NAME, on any row podbox fails" || echo 'ABSENT: no engine, so a failure cannot be attributed')"
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

# ⭐ The row's whole podbox sequence in one driver call. The driver
# cannot receive the subject as argv (it arrives through two shells), so
# both scripts travel as staged files and the row driver reads them
# inside. Stdout carries only stage failures; every reading lands in
# /w/row-NAME/ on the shared scratch.
cat >"$WORK/row-driver.sh" <<'ROWDRIVER_EOF'
#!/bin/sh
# row-driver.sh PINNED NAME ROW_TIMEOUT - one matrix row in the driver.
set -u
export PODBOX_STORE=/w/store
pinned="$1"
name="$2"
to="$3"
d="/w/row-$name"
mkdir -p "$d" || exit 6
/pb pull "$pinned" >"$d/pull.log" 2>&1 || exit 3
/pb extract "$pinned" >/dev/null 2>&1 || exit 4
root="$(/pb inspect --format '{{.RootfsPath}}' "$pinned" 2>/dev/null)" || exit 5
[ -n "$root" ] || exit 5
cp /pb "$root/podbox-static" && cp /govictim "$root/go-victim" \
  && chmod 755 "$root/podbox-static" "$root/go-victim" || exit 6
timeout "$to" /pb run "$pinned" /bin/sh -c "$(cat /subj/subject.sh)" \
  >"$d/out" 2>"$d/err"
echo "$?" >"$d/rowrc"
timeout "$to" /pb run "$pinned" /podbox-static --version >"$d/sout" 2>&1
echo "$?" >"$d/src"
timeout "$to" /pb run "$pinned" /go-victim >"$d/gout" 2>&1
echo "$?" >"$d/grc"
if [ -f "$root/.podbox/interpose.so" ]; then
  sha256sum "$root/.podbox/interpose.so" | cut -d' ' -f1 >"$d/selsha"
else
  echo absent >"$d/selsha"
fi
exit 0
ROWDRIVER_EOF

ran=0
nopull=0
broken=0
host=0
virtualized=0
declined=0

printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' ROW LIBC SEEN CLEARED STATIC GO SELECT
printf '%.0s-' $(seq 1 74)
echo

# ⭐ Staged once: the same files serve every row, and the mounts stay
# put until the loop is done. The objects stay on the host; only the
# binary enters the driver, because only it has to execute there. /w is
# the shared scratch the driver writes its readings to, read-write.
if [ "$NATIVE" -eq 0 ]; then
	mkdir -p "$WORK/w" "$STAGE"
	cp "$BIN" "$STAGE/podbox"
	[ -f "$GNU_SO" ] && cp "$GNU_SO" "$STAGE/gnu.so"
	[ -f "$MUSL_SO" ] && cp "$MUSL_SO" "$STAGE/musl.so"
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_mount "$WORK/go-victim" /govictim \
		&& eng_mount "$WORK/row-driver.sh" /drv/row.sh \
		&& eng_mount "$WORK/subject.sh" /subj/subject.sh \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
fi

while IFS='|' read -r ref name libc digest; do
	[ -n "${ref:-}" ] || continue
	[ -z "$ONLY" ] || [ "$ONLY" = "$name" ] || continue
	pinned="$(distro_pinned "$ref" "$digest")"
	t="$TRANSCRIPTS/$name.out"

	# ⭐ Non-native lane: one driver call runs the row's whole podbox
	# sequence (pull, extract, inspect, victims, three runs, selection
	# hash) and normalizes every reading to the same names the native
	# path writes, so the transcript and every check below are shared.
	if [ "$NATIVE" -eq 0 ]; then
		d="$WORK/w/row-$name"
		drc_out="$(eng_run "$OUTER" "$DRIVER" "$DRIVER_CAPS" -- /bin/sh /drv/row.sh "$pinned" "$name" "$ROW_TIMEOUT" 2>&1)"; drc=$?
		case "$drc" in
			0) ;;
			3)
				printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - 'no-pull'
				{
					echo "no-pull $pinned"
					tail -3 "$d/pull.log"
				} >"$t"
				nopull=$((nopull + 1))
				continue ;;
			4|5|6)
				printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - 'no rootfs'
				echo "driver stage $drc for $pinned" >"$t"
				nopull=$((nopull + 1))
				continue ;;
			*)
				printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - "driver rc=$drc"
				{
					echo "driver rc=$drc for $pinned"
					printf '%s\n' "$drc_out" | tail -5
				} >"$t"
				broken=$((broken + 1))
				continue ;;
		esac
		cp_ok=1
		for f in out err; do
			cp "$d/$f" "$WORK/$f.$name" 2>/dev/null || cp_ok=0
		done
		if [ "$cp_ok" -eq 0 ]; then
			printf '%-14s %-8s %-6s %-9s %-8s %-8s %s\n' "$name" "$libc" - - - - 'driver files missing'
			echo "driver files missing for $pinned" >"$t"
			broken=$((broken + 1))
			continue
		fi
		row_rc="$(cat "$d/rowrc")"
		sout="$(cat "$d/sout")"
		src="$(cat "$d/src")"
		gout="$(cat "$d/gout")"
		grc="$(cat "$d/grc")"
	else
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
	fi
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
	if [ "$NATIVE" -eq 1 ]; then
		sout="$(timeout "$ROW_TIMEOUT" "$BIN" run "$pinned" /podbox-static --version 2>&1)"
		src=$?
	fi
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
	if [ "$NATIVE" -eq 1 ]; then
		gout="$(timeout "$ROW_TIMEOUT" "$BIN" run "$pinned" /go-victim 2>&1)"
		grc=$?
	fi
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
		[ -f "$(objfile "$obj")" ] || objects_built=0
	done
	if [ "$objects_built" -eq 1 ]; then
		if [ "$NATIVE" -eq 1 ]; then
			if [ -f "$ROOT/.podbox/interpose.so" ]; then
				for obj in gnu musl; do
					if cmp -s "$ROOT/.podbox/interpose.so" "$(objfile "$obj")"; then
						sel="$obj"
					fi
				done
			fi
		else
			# ⭐ Byte equality by digest: the placed object's sha256 travels
			# out in the row's files, and no pipe below ever carries a binary.
			selsha="$(cat "$d/selsha" 2>/dev/null || echo absent)"
			for obj in gnu musl; do
				if [ "$selsha" != "absent" ] && [ "$(sha256sum "$(objfile "$obj")" | cut -d' ' -f1)" = "$selsha" ]; then
					sel="$obj"
				fi
			done
		fi
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
	# ⛔ A row the payload did not clear fails here unless the engine fails it
	# too, which is the control that attributes the failure (240's rule).
	# Without an engine the failure counts as the host's: podbox cannot prove
	# otherwise, and a failure it cannot attribute is not one it owns.
	# The subject travels as a staged file here rather than as argv: the
	# helper can bind-mount, so the two shells the argv form crosses are gone.
	# ⛔ A row the payload did not clear fails here unless the engine fails it
	# too, which is the control that attributes the failure (240's rule).
	# Without an engine the failure counts as the host's: podbox cannot prove
	# otherwise, and a failure it cannot attribute is not one it owns.
	# The subject travels as a staged file here rather than as argv: the
	# helper can bind-mount, so the two shells the argv form crosses are gone.
	# ⭐ Static-declined rows skip the control. The tier declined by name
	# and the wall blocks the native chown, so a wall-less control would
	# succeed and blame podbox for a correct decline; the decline above
	# already attributes it. Dynamic rows keep the control: there the
	# engine succeeding where podbox failed names the owner.
	if [ "$cleared" != "yes" ]; then
		if [ "$seen" != "yes" ]; then
			printf '%-14s   static decline under the wall: recorded as the host.\n' "$name"
			host=$((host + 1))
		elif [ "$HAVE_ENGINE" -eq 0 ]; then
			printf '%-14s   ⚠ no engine control: recorded as the host.\n' "$name"
			host=$((host + 1))
		elif eng_mount "$WORK/subject.sh" /subj.sh; then
			if eng_run "$ROW_TIMEOUT" "$pinned" "" -- /bin/sh /subj.sh \
				>"$WORK/dout.$name" 2>/dev/null \
			&& grep -q '^CLEARED=yes' "$WORK/dout.$name"; then
				printf '%-14s   ⛔ podbox failed and %s SUCCEEDED on the same image.\n' "$name" "$ENGINE_NAME"
				broken=$((broken + 1))
			else
				printf '%-14s   ⚠ the host: %s failed here too.\n' "$name" "$ENGINE_NAME"
				host=$((host + 1))
			fi
		eng_clear
		else
			printf '%-14s   ⛔ the control could not be staged.\n' "$name"
			broken=$((broken + 1))
		fi
	fi
done <<EOF_ROWS
$DISTRO_ROWS_M5
EOF_ROWS

# Loop mounts go back; one-shot runs need no trap, and the
# control and refusal arms clear their own.
eng_clear

echo
echo "== the two cross-libc refusals, driven deliberately"
# The matrix rows' own files: alpine's loader IS its libc, debian's libc is
# found by search, both from outside with this host's tools (100-'s rule).
refuse_fail=0
for pair in "gnu musl ld-musl-*.so.1" "musl glibc libc.so.6"; do
	set -- $pair
	obj="$(objfile "$1")"
	pat="$3"
	# ⭐ First host-readable match in sorted order, never whatever `find`
	# returns first: an absolute link of another rootfs (void's
	# `ld-musl-*.so.1` -> `/usr/lib64/libc.so`, measured 2026-09-19)
	# dangles on the staging host, so staging it can only stage a
	# failure, and the "refusal" that prints proves nothing. A match the
	# host cannot read, or one resolving outside the checkout, is not
	# the rootfs's file and counts as missing, honestly.
	find "$WORK" -path '*store*' -name "$pat" 2>/dev/null | sort >"$WORK/cands.$1$2"
	found=""
	seen=0
	while IFS= read -r cand; do
		[ -n "$cand" ] || continue
		seen=$((seen + 1))
		[ -e "$cand" ] || continue
		rcand="$(readlink -f "$cand" 2>/dev/null)" || continue
		case "$rcand" in
		"$WORK"/*) found="$cand"; break ;;
		esac
	done <"$WORK/cands.$1$2"
	rm -f "$WORK/cands.$1$2"
	if [ ! -f "$obj" ]; then
		echo "  SKIP $1 object against $2 libc: object not built"
	elif [ -z "$found" ] && [ "$seen" -eq 0 ]; then
		echo "  SKIP $1 object against $2 libc: no $pat extracted"
	elif [ -z "$found" ]; then
		echo "  SKIP $1 object against $2 libc: no usable $pat: $seen match(es), none readable from this lane"
	elif abi_check "$1" "$obj" "$found" "$WORK/refuse.$1$2"; then
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
