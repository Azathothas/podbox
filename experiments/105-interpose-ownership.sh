#!/usr/bin/env bash
# Question: does podbox's own interposer clear the ownership wall, and does it
# stay quiet on a machine that has no wall?
#
# TODO/interpose.md T-0701 (the cdylib's build constraints) and T-0704
# (ownership virtualization). This is the half a path interposer does not
# have: `references/VHSgunzo__pathmap/tree/path-mapping.c:1240-1241` routes
# `chown` through the same path-rewriting macro as everything else and passes
# the ids through untouched, so a payload's `chown 0:42` fails identically with
# and without it loaded.
#
# Six checks:
#
#   A. the EXPORTED SET. T-0701 constraint 1: exactly the entry points, and no
#      `rust_eh_personality`. Asserted against `interpose.map` itself, so the
#      version script and the `#[no_mangle]` list cannot drift apart.
#   B. `struct stat` AND `struct statx` OFFSETS, under both libcs, by `offsetof`
#      rather than by reading a header. `src/lib.rs` carries all of them, and a
#      check that asserted only the first struct would have left unmeasured the
#      one that actually answers a modern glibc payload.
#   C. THE CONTROL, and it comes first: on a machine that CAN chown, podbox must
#      change nothing and write no memo. An interposer that reported the
#      caller's intent where the kernel would have done the real thing is weaker
#      than the bare chroot it replaces.
#   D. THE WALL, glibc. `--cap-drop=CHOWN` is root without `CAP_CHOWN`, which is
#      the same refusal this runtime gives for an unmapped id. Without the
#      object the chown fails; with it the chown succeeds and `stat` reports
#      what the payload meant.
#   E. THE WALL, musl, with the musl-linked object, because T-0702 is that one
#      object per libc is required and an arm that only ran under glibc would
#      not have tested that.
#   F. THE PAIR podbox MUST REFUSE, and it is podbox's own object rather than
#      the C reference interposer 80-interposer-abi.sh uses: this object is
#      built on glibc 2.39 and imports `dlsym@GLIBC_2.34`, so a 2.31 payload
#      cannot load it. T-0709's reader says so from ELF, with nothing loaded,
#      and the loader is asked afterwards to agree.
#
# WHY NOT THIS HOST DIRECTLY. It grants real ownership -- `podbox probe` reads
# `ownership: real` -- so `chown 0:42` SUCCEEDS here and the memo path never
# runs. A check that passed for that reason would be measuring the kernel.
# `--cap-drop=CHOWN` removes exactly the capability and nothing else.
#
# Inputs pinned: the glibc image and the too-old glibc image by manifest
# digest, the musl image by the same digest the M5 alpine row pins (qualified
# at `public.ecr.aws`, never Docker Hub), and the objects
# `scripts/build-interpose.sh` produces from this tree, or `GNU_SO` and
# `MUSL_SO` in the environment pointing at a guest build's artifacts. The
# podbox binary for check F is `PODBOX_BIN` (a Linux binary: on a Linux lane
# it runs directly, anywhere else staged inside the glibc payload image,
# which is statically linked enough to need nothing from it but a kernel).
#
# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. Every container call carries a timeout, runs
# `--rm`, mounts read-only, and refuses `--privileged`, unpinned images and
# mount sources outside this tree.
#
# Exit: 0 every check that ran matched, 1 one did not, 2 a check could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
CRATE="$ROOT/crates/podbox-interpose"
OUT="${OUT:-$HERE/.ownership}"
rm -rf "$OUT"
mkdir -p "$OUT"

# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$ROOT"
ENGINE_WORK="$OUT"
engine_pick || exit 2
trap eng_cleanup EXIT INT TERM

GNU_SO="${GNU_SO:-$CRATE/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so}"
MUSL_SO="${MUSL_SO:-$CRATE/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so}"
# The glibc payload is newer than the build host's, and that is not a
# convenience. This object is linked against the host's glibc 2.39 and imports
# `dlsym@GLIBC_2.34`, so it CANNOT be loaded into a 2.31 payload: check F puts
# the old one to podbox's own reader and requires it to be refused before
# anything is loaded, which is TODO/interpose.md T-0709.
GLIBC_PAYLOAD='quay.io/fedora/fedora@sha256:e78cd1a688cd079c23864f289a89a49a3f4ad66d817864e325e1d058310ee95c'
# Bare `ubuntu` with a pinned digest: no M5 row names this older libc, so it
# stays as it was. [T-1209](gate.md) owns the Hub mapping in general.
GLIBC_TOO_OLD='ubuntu@sha256:c664f8f86ed5a386b0a340d981b8f81714e21a8b9c73f658c4bea56aa179d54a'
MUSL_PAYLOAD='public.ecr.aws/docker/library/alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'host libc         %s\n' "$(ldd --version 2>&1 | head -1)"
printf 'zig               %s\n' "$(zig version 2>/dev/null || echo absent)"
printf 'engine            %s %s\n' "$ENGINE_NAME" "$(engine_describe 2>/dev/null || echo MISSING)"
printf 'glibc payload     %s\n' "$GLIBC_PAYLOAD"
printf 'glibc too old     %s\n' "$GLIBC_TOO_OLD"
printf 'musl payload      %s\n' "$MUSL_PAYLOAD"
echo

rc=0
could_not=0
fail() { printf 'FAIL %s\n' "$1"; rc=1; }
pass() { printf 'ok   %s\n' "$1"; }
cannot() { printf '  COULD NOT RUN: %s\n' "$1"; could_not=1; }

if [ ! -r "$GNU_SO" ] || [ ! -r "$MUSL_SO" ]; then
	echo "SKIP: the objects are not built. ./scripts/build-interpose.sh (or set GNU_SO and MUSL_SO)" >&2
	exit 2
fi

# The first engine run would otherwise spend its own timeout pulling: fetch
# both payload images before any clause is timed.
eng_pull "$GLIBC_PAYLOAD" || exit 2
eng_pull "$MUSL_PAYLOAD" || exit 2

echo "== A. the exported set, against the version script"
# The version script is the SOURCE of this list, and the object is what is
# compared with it: a name in `interpose.map` that `src/lib.rs` does not define
# is a silent non-interposition, and one the object exports that the script does
# not list is a symbol some other library in the payload's process would resolve
# to podbox.
awk '/global:/{g=1;next} /local:/{g=0} g && /;/{gsub(/[ \t;]/,"");if($0!="")print}' \
	"$CRATE/interpose.map" | sort >"$OUT/declared"
nm -D --defined-only "$GNU_SO" | awk '$2=="T"{print $3}' | sort >"$OUT/exported.gnu"
nm -D --defined-only "$MUSL_SO" | awk '$2=="T"{print $3}' | sort >"$OUT/exported.musl"
printf '  declared in interpose.map  %s\n' "$(wc -l <"$OUT/declared")"
printf '  exported by the gnu object %s\n' "$(wc -l <"$OUT/exported.gnu")"
printf '  exported by the musl object %s\n' "$(wc -l <"$OUT/exported.musl")"
for arm in gnu musl; do
	if diff -q "$OUT/declared" "$OUT/exported.$arm" >/dev/null; then
		pass "A: the $arm object exports exactly what interpose.map declares"
	else
		fail "A: the $arm object's exports differ from interpose.map"
		diff "$OUT/declared" "$OUT/exported.$arm" | sed 's/^/      /'
	fi
done
# T-0701's own Prove. A default Rust cdylib exports this into every process.
for arm in gnu musl; do
	so="$GNU_SO"
	[ "$arm" = musl ] && so="$MUSL_SO"
	n="$(nm -D --defined-only "$so" | grep -c 'rust_eh_personality')"
	printf '  rust_eh_personality in the %s object: %s\n' "$arm" "$n"
	[ "$n" -eq 0 ] || fail "A: the $arm object exports rust_eh_personality"
done
echo

echo "== B. struct stat offsets, by offsetof and under both libcs"
cat >"$OUT/off.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <sys/stat.h>
#include <stddef.h>
/* NOT <linux/stat.h>. musl's own <sys/stat.h> defines `struct statx` and the
 * kernel header then redefines it; glibc since 2.28 defines it there too under
 * _GNU_SOURCE. Two libcs declaring one kernel struct in two different headers
 * is exactly why podbox's object carries OFFSETS rather than a struct. */
int main(void){
  printf("sizeof=%zu dev=%zu ino=%zu mode=%zu uid=%zu gid=%zu\n",
    sizeof(struct stat), offsetof(struct stat, st_dev), offsetof(struct stat, st_ino),
    offsetof(struct stat, st_mode), offsetof(struct stat, st_uid),
    offsetof(struct stat, st_gid));
  /* statx too, and it is not a duplicate: coreutils' `stat` on a modern
   * glibc asks statx(2) and never reaches `stat`. src/lib.rs carries these
   * six numbers as well, and a check that asserted only the first struct
   * would have left the ones that actually answer a glibc payload unmeasured. */
  printf("statx sizeof=%zu mask=%zu uid=%zu gid=%zu ino=%zu devmaj=%zu devmin=%zu\n",
    sizeof(struct statx), offsetof(struct statx, stx_mask),
    offsetof(struct statx, stx_uid), offsetof(struct statx, stx_gid),
    offsetof(struct statx, stx_ino), offsetof(struct statx, stx_dev_major),
    offsetof(struct statx, stx_dev_minor));
  return 0;
}
EOF
g_off=""
m_off=""
# NTFS carries no POSIX mode, and chmod on this checkout is a silent no-op
# (measured 2026-09-19), so a binary linked on the Windows host must be
# copied to a filesystem that honours modes before it can run. The runs
# below copy it inside the container for exactly that reason.
if cc -O1 -o "$OUT/off_g" "$OUT/off.c" 2>/dev/null; then
	:
elif command -v zig >/dev/null 2>&1 &&
	ZIG_TARGET=x86_64-linux-gnu.2.17 "$ROOT/scripts/zig-cc.sh" -O1 \
		-o "$OUT/off_g" "$OUT/off.c" 2>/dev/null; then
	:
else
	cannot "the glibc offsetof program did not build (needs cc or zig)"
fi
if command -v zig >/dev/null 2>&1 &&
	ZIG_TARGET=x86_64-linux-musl "$ROOT/scripts/zig-cc.sh" -O1 \
		-o "$OUT/off_m" "$OUT/off.c" 2>/dev/null; then
	:
else
	cannot "the musl offsetof program did not build (needs zig)"
fi
# A Linux host runs both directly. Anywhere else they run staged inside the
# matching image, because an ELF built here does not execute there. The copy
# inside the container is load-bearing on Windows lanes: see above.
if [ -f "$OUT/off_g" ] && [ -z "$g_off" ]; then
	case "$(uname -s)" in
	Linux) g_off="$("$OUT/off_g" | tr '\n' '|')" ;;
	*)
		if eng_mount "$OUT/off_g" /off-src; then
			g_off="$(eng_run 60 "$GLIBC_PAYLOAD" "" -- /bin/sh -c 'cp /off-src /tmp/off && chmod +x /tmp/off && exec /tmp/off' 2>"$OUT/off_g.err" | tr '\n' '|')" || g_off=""
			[ -n "$g_off" ] || sed 's/^/  engine: /' "$OUT/off_g.err"
		fi
		eng_clear
		;;
	esac
fi
if [ -f "$OUT/off_m" ] && [ -z "$m_off" ]; then
	case "$(uname -s)" in
	Linux) m_off="$("$OUT/off_m" | tr '\n' '|')" ;;
	*)
		if eng_mount "$OUT/off_m" /off-src; then
			m_off="$(eng_run 60 "$MUSL_PAYLOAD" "" -- /bin/sh -c 'cp /off-src /tmp/off && chmod +x /tmp/off && exec /tmp/off' 2>"$OUT/off_m.err" | tr '\n' '|')" || m_off=""
			[ -n "$m_off" ] || sed 's/^/  engine: /' "$OUT/off_m.err"
		fi
		eng_clear
		;;
	esac
fi
printf '  glibc  %s\n' "${g_off:-COULD NOT BUILD OR RUN}"
printf '  musl   %s\n' "${m_off:-COULD NOT BUILD OR RUN}"
# The numbers `crates/podbox-interpose/src/lib.rs` carries, asserted here
# rather than believed. They agree on x86_64 and that is this architecture's
# property, not a general one: T-0702's premise is that struct layout is what a
# preload cannot bridge.
WANT='sizeof=144 dev=0 ino=8 mode=24 uid=28 gid=32|statx sizeof=256 mask=0 uid=20 gid=24 ino=32 devmaj=136 devmin=140|'
if [ -z "$g_off" ] || [ -z "$m_off" ]; then
	cannot "one of the two offsetof programs did not build or run"
elif [ "$g_off" = "$WANT" ] && [ "$m_off" = "$WANT" ]; then
	pass "B: both libcs agree on BOTH structs, and with the offsets src/lib.rs carries"
else
	fail "B: the offsets differ from the ones src/lib.rs carries [$WANT]"
fi
echo

# subject SO IMAGE CAPS -> prints "<rc-of-chown> <what stat reports> <memo bytes>"
# ⭐ T-0710: the memo is handed as a descriptor, as podbox hands it at spawn.
# Where the payload preloads the object the harness opens `/.podbox/` (inside
# this test container, standing in for the host file podbox holds) read-write
# and hands fd 9 with `PODBOX_MEMO_FD=9`; bare runs hand nothing and write
# no memo. Read-write (`<>`) rather than append (`>>`): the interposer reads
# through the same descriptor it writes, and a write-only one answers `EBADF`
# on the read. The object seeks to the end before each record, so the position
# is exact either way. Fd 9 and not 17: the old glibc payload's `/bin/sh` is
# dash, which redirects single-digit descriptors only; podbox itself hands 17
# by `dup2` before the `exec` and no shell is involved there.
subject() {
	local so="$1" img="$2" caps="$3" preload="$4"
	eng_mount "$so" /i.so || return 1
	eng_run 180 "$img" "$caps" -- /bin/sh -c "
    case ' ${preload} ' in
      *LD_PRELOAD*)
        mkdir -p /.podbox
        : >/.podbox/ownership.memo
        exec 9<>/.podbox/ownership.memo
        export PODBOX_MEMO_FD=9
        ;;
    esac
    ${preload}
    : >/tmp/f
    chown 0:42 /tmp/f 2>/tmp/e; echo \"CHOWN_RC=\$?\"
    echo \"CHOWN_ERR=\$(head -1 /tmp/e 2>/dev/null | cut -c1-80)\"
    echo \"STAT=\$(stat -c %u:%g /tmp/f 2>/dev/null)\"
    echo \"MEMO=\$( [ -f /.podbox/ownership.memo ] && wc -c </.podbox/ownership.memo || echo none )\"
  " 2>&1
}

echo "== C. the control: a machine that CAN chown, with the object loaded"
# FIRST. An interposer that reported the caller's intent where the kernel
# would have done the real thing is weaker than the bare chroot it replaces, and
# every arm below would pass for that reason.
c_out="$(subject "$GNU_SO" "$GLIBC_PAYLOAD" "" 'export LD_PRELOAD=/i.so')"
printf '%s\n' "$c_out" | sed 's/^/  /'
c_stat="$(printf '%s' "$c_out" | sed -n 's/^STAT=//p')"
c_memo="$(printf '%s' "$c_out" | sed -n 's/^MEMO=//p')"
# ⭐ T-0710: the harness hands an empty host file; "wrote no memo" is no
# records, so none (no file, bare runs) or 0 (empty file, real chown needed no
# record) both pass. Anything else is a record where the kernel did the work.
if [ "$c_stat" = "0:42" ] && { [ "$c_memo" = "none" ] || [ "$c_memo" = "0" ]; }; then
	pass "C: the real chown worked and podbox wrote no memo"
else
	fail "C: stat says [$c_stat] and the memo is [$c_memo]; it should be 0:42 and none or 0"
fi
echo

echo "== D. the wall, glibc: root WITHOUT CAP_CHOWN"
# `--cap-drop=CHOWN` refuses the call with EPERM, which is one of the two
# errnos T-0704 names. The runtime podbox targets answers EINVAL for an unmapped
# id, and the object swallows both; fakeroot tests EPERM alone at eight sites
# and would return the error on every one of them here.
d_bare="$(subject "$GNU_SO" "$GLIBC_PAYLOAD" "--cap-drop=CHOWN" 'true')"
printf '  without the object:\n'
printf '%s\n' "$d_bare" | sed 's/^/    /'
d_pre="$(subject "$GNU_SO" "$GLIBC_PAYLOAD" "--cap-drop=CHOWN" 'export LD_PRELOAD=/i.so')"
printf '  with the object:\n'
printf '%s\n' "$d_pre" | sed 's/^/    /'
bare_rc="$(printf '%s' "$d_bare" | sed -n 's/^CHOWN_RC=//p')"
pre_rc="$(printf '%s' "$d_pre" | sed -n 's/^CHOWN_RC=//p')"
pre_stat="$(printf '%s' "$d_pre" | sed -n 's/^STAT=//p')"
if [ "$bare_rc" = "0" ]; then
	cannot "the control chown SUCCEEDED without CAP_CHOWN, so there is no wall here"
elif [ "$pre_rc" = "0" ] && [ "$pre_stat" = "0:42" ]; then
	pass "D: the bare chown failed (rc=$bare_rc) and podbox's answered 0:42"
else
	fail "D: with the object the chown exited $pre_rc and stat says [$pre_stat]"
fi
echo

echo "== E. the wall, musl, with the musl-linked object"
e_bare="$(subject "$MUSL_SO" "$MUSL_PAYLOAD" "--cap-drop=CHOWN" 'true')"
printf '  without the object:\n'
printf '%s\n' "$e_bare" | sed 's/^/    /'
e_pre="$(subject "$MUSL_SO" "$MUSL_PAYLOAD" "--cap-drop=CHOWN" 'export LD_PRELOAD=/i.so')"
printf '  with the object:\n'
printf '%s\n' "$e_pre" | sed 's/^/    /'
e_bare_rc="$(printf '%s' "$e_bare" | sed -n 's/^CHOWN_RC=//p')"
e_pre_rc="$(printf '%s' "$e_pre" | sed -n 's/^CHOWN_RC=//p')"
e_pre_stat="$(printf '%s' "$e_pre" | sed -n 's/^STAT=//p')"
if [ "$e_bare_rc" = "0" ]; then
	cannot "the musl control chown SUCCEEDED without CAP_CHOWN"
elif [ "$e_pre_rc" = "0" ] && [ "$e_pre_stat" = "0:42" ]; then
	pass "E: the musl object answered 0:42 where the bare chown failed (rc=$e_bare_rc)"
else
	fail "E: with the musl object the chown exited $e_pre_rc and stat says [$e_pre_stat]"
fi
echo

echo "== F. the pair podbox must refuse BEFORE loading anything"
# TODO/interpose.md T-0709, against podbox's OWN object rather than the C
# reference interposer 80-interposer-abi.sh uses. The object is built on glibc
# 2.39 and imports `dlsym@GLIBC_2.34`; the older payload declares up to
# GLIBC_2.31. The reader must say so from ELF, with nothing loaded, and the
# loader must agree when the pair is forced.
BIN="${PODBOX_BIN:-$ROOT/target/x86_64-unknown-linux-musl/release/podbox}"
# The reader runs where a Linux binary executes. On a Linux lane that is
# here. Anywhere else it runs staged inside the glibc payload image: the
# release binary is statically linked, so it needs nothing from the image
# but a kernel, and the base-exec channel proved too flaky for argv (usage
# answers mixed with genuine ones across identical calls).
_can_read=""
_can_engine=""
case "$(uname -s)" in
Linux)
	if [ -x "$BIN" ]; then
		_can_read="1"
	else
		cannot "$BIN is not an executable (set PODBOX_BIN), so the reader cannot be asked"
	fi
	;;
*)
	if [ -r "$BIN" ]; then
		_can_engine="1"
	else
		cannot "PODBOX_BIN is not readable (set it to a guest build artifact), so the reader cannot be asked"
	fi
	;;
esac
if [ -n "$_can_read" ] || [ -n "$_can_engine" ]; then
	cid="$(eng_create "$GLIBC_TOO_OLD" -- /bin/true)" || {
		cannot "the old payload image could not be created"
		cid=""
	}
	if [ -n "$cid" ] && eng_cp "$cid" /lib/x86_64-linux-gnu/libc.so.6 "$OUT/old-libc.so.6" >/dev/null 2>&1 && [ -s "$OUT/old-libc.so.6" ]; then
		eng_rm "$cid"
		if [ -n "$_can_read" ]; then
			f_out="$("$BIN" system abi "$GNU_SO" "$OUT/old-libc.so.6" 2>&1)"
			f_rc=$?
		elif eng_mount "$BIN" /pb && eng_mount "$GNU_SO" /obj && eng_mount "$OUT/old-libc.so.6" /lc; then
			f_out="$(eng_run 60 "$GLIBC_PAYLOAD" "" -- /pb system abi /obj /lc 2>&1)"
			f_rc=$?
			eng_clear
		else
			cannot "the reader inputs could not be staged"
			f_out=""
			f_rc=2
		fi
		printf '  podbox system abi: rc=%s %s\n' "$f_rc" \
			"$(printf '%s' "$f_out" | head -1 | sed "s#$ROOT/##g" | cut -c1-100)"
		# subject() stages the object itself; staging it here too would only
		# double the same destination.
		f_loader="$(subject "$GNU_SO" "$GLIBC_TOO_OLD" "" 'export LD_PRELOAD=/i.so' |
			grep -oE "GLIBC_[0-9.]+.? not found" | head -1)"
		printf '  and the loader:    %s\n' "${f_loader:-it loaded}"
		if [ "$f_rc" = 1 ] && [ -n "$f_loader" ]; then
			pass "F: the reader refused the pair from ELF and the loader agreed"
		elif [ "$f_rc" = 1 ]; then
			fail "F: the reader refused it and the loader did not"
		else
			fail "F: the reader answered rc=$f_rc for a pair the loader refuses"
		fi
	else
		[ -n "$cid" ] && eng_rm "$cid"
		cannot "the old payload's libc could not be copied out"
	fi
fi
echo

echo "== G. past the scan ceiling: correct or refusal, never stale"
# TODO/interpose.md T-0710. The memo holds 4 MiB (131,072 records) of scan.
# An old pair recorded before the ceiling, overwritten past it, must not read
# back as the old owner: the lookup refuses and the real `stat` answers,
# marked degraded. The current code fails this check, which is why it is
# written before the fix rather than after.
#
# Shape, inside one `--cap-drop=CHOWN` container so the wall holds throughout:
# chown to 0:42 (records old), fill past the ceiling with zeros, chown the
# same file to 0:43 (records new, past the ceiling), then stat. Correct is
# 0:43, refusal is the real 0:0, stale is 0:42. The harness hands fd 9 as
# `subject` does (dash reaches single-digit descriptors only); podbox hands 17
# by `dup2` with no shell involved.
if eng_mount "$GNU_SO" /i.so; then
	g_out="$(eng_run 180 "$GLIBC_PAYLOAD" "--cap-drop=CHOWN" -- /bin/sh -c '
    mkdir -p /.podbox
    : >/.podbox/ownership.memo
    exec 9<>/.podbox/ownership.memo
    export PODBOX_MEMO_FD=9 LD_PRELOAD=/i.so
    : >/tmp/g
    chown 0:42 /tmp/g 2>/dev/null
    echo "OLD_STAT=$(stat -c %u:%g /tmp/g 2>/dev/null)"
    dd if=/dev/zero bs=1M count=5 >>/.podbox/ownership.memo 2>/dev/null
    chown 0:43 /tmp/g 2>/dev/null
    echo "NEW_STAT=$(stat -c %u:%g /tmp/g 2>/dev/null)"
    echo "MEMO_BYTES=$(wc -c </.podbox/ownership.memo)"
  ' 2>&1)"
	g_rc=$?
	eng_clear
	printf '%s\n' "$g_out" | sed 's/^/  /'
	g_old="$(printf '%s' "$g_out" | sed -n 's/^OLD_STAT=//p')"
	g_new="$(printf '%s' "$g_out" | sed -n 's/^NEW_STAT=//p')"
	g_bytes="$(printf '%s' "$g_out" | sed -n 's/^MEMO_BYTES=//p')"
	printf '  old stat %s, new stat %s, memo bytes %s\n' \
		"${g_old:-?}" "${g_new:-?}" "${g_bytes:-?}"
	if [ "$g_old" != "0:42" ]; then
		fail "G: the old chown did not record 0:42 (got [$g_old]); the wall may not hold here"
	elif [ "$g_new" = "0:42" ]; then
		fail "G: past the ceiling the memo answered stale 0:42 instead of correct 0:43 or refusal 0:0"
	elif [ "$g_new" = "0:43" ] || [ "$g_new" = "0:0" ]; then
		pass "G: past the ceiling the answer is $g_new (correct or refusal), never stale 0:42"
	else
		fail "G: past the ceiling the stat is [$g_new], which is neither correct 0:43 nor refusal 0:0"
	fi
else
	cannot "the object could not be staged for check G"
fi
echo

echo "== verdict"
if [ "$rc" -eq 0 ] && [ "$could_not" -eq 0 ]; then
	echo "  every check ran and matched. podbox's own object clears the ownership"
	echo "  wall and stays quiet where there is none."
elif [ "$rc" -eq 0 ]; then
	echo "  every check that ran matched; one or more could not be taken here."
else
	echo "  see the FAIL lines above"
fi
[ "$rc" -ne 0 ] || [ "$could_not" -eq 0 ] || exit 2
exit "$rc"
