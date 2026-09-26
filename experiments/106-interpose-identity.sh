#!/usr/bin/env bash
# Question: does the identity tier refuse honestly by default, lie only under
# `--user`, and say the same thing in the banner as the call did?
#
# TODO/interpose.md T-0711. The operator ruled answer 3: `setuid` and friends
# fail as the runtime fails them by default, and `--user` turns on the fakeroot
# behaviour for a payload that needs it. The flag marks the run degraded, so
# `--strict` refuses it, and the banner names it wherever the tier loads.
#
# Seven clauses:
#   A. THE CONTROL FIRST. Without `PODBOX_IDENTITY` the object changes
#      nothing: the victim's answers match the bare run exactly. An
#      interposer that faked where the kernel would have granted is weaker
#      than the bare chroot it replaces.
#   B. THE FAKE, glibc. With `PODBOX_IDENTITY=1000:100` the setters answer 0
#      and the getters answer the record, on a dynamic glibc payload.
#   C. THE FAKE, musl, with the musl-linked object. One libc's arm would not
#      have tested the other object (TODO/interpose.md T-0702). C0 first:
#      with no flag the honest musl object must report what the kernel
#      says, checked against clause A's bare lines on the same kernel.
#      The victim cannot run on the glibc job container directly (its
#      interpreter is the absent musl loader), so both arms run staged
#      inside the alpine rootfs through podbox itself.
#   D. END TO END through podbox: `--user 1000:100` makes `id -u` answer
#      1000 on both libcs, the banner names the faked identity, and without
#      the flag the same payload answers 0.
#   E. `--strict` refuses a `--user` run, naming the flag, because the parity
#      table carries `-u, --user` as Degraded.
#   F. A declined tier carries no memo: a static payload with `--user` still
#      runs, and the decline names the flag as honoured by nothing.
#   G. THE ERRNO-BY-CALL READING (T-0711): the honest object reports each
#      refused setter with its call and errno on the denying machine at
#      hand; recorded, never asserted absolutely (errnos vary by wall).
#
# ⛔ No `setuid` outcome is asserted against the kernel: on a machine that CAN
# change ids the honest call succeeds, on podbox's target it fails, so the
# assertion is bare-against-loaded (A) and record-against-request (B/C/D),
# never an absolute rc.
#
# Inputs pinned: the victims are built here from the included C source (host
# `cc` for glibc, `scripts/zig-cc.sh` for musl); the two images by manifest
# digest (the M5 alpine row and the fedora row 105 uses) for the podbox-driven
# clauses D, E and F; the objects `scripts/build-interpose.sh` produces from
# this tree; and the podbox binary under test (which is itself the static
# payload clause F drives).
#
# Exit: 0 every check that ran matched, 1 one did not, 2 a check could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
CRATE="$ROOT/crates/podbox-interpose"
OUT="$ROOT/experiments/results/interpose-identity.txt"
WORK="$(mktemp -d)"

cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

GNU_SO="$CRATE/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so"
MUSL_SO="$CRATE/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so"
BIN="${PODBOX_BIN:-$ROOT/target/x86_64-unknown-linux-musl/release/podbox}"
GLIBC_IMG='quay.io/fedora/fedora@sha256:e78cd1a688cd079c23864f289a89a49a3f4ad66d817864e325e1d058310ee95c'
MUSL_IMG='public.ecr.aws/docker/library/alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'host arch         %s\n' "$(uname -m)"
printf 'zig               %s\n' "$(zig version 2>/dev/null || echo absent)"
printf 'cc                %s\n' "$(cc --version 2>/dev/null | head -1 || echo absent)"
_dv="$(docker version --format '{{.Server.Version}}' 2>/dev/null)"
printf 'docker            %s\n' "${_dv:-MISSING}"
printf 'podbox            %s\n' "$("$BIN" version 2>/dev/null || echo MISSING)"
printf 'glibc image       %s\n' "$GLIBC_IMG"
printf 'musl image        %s\n' "$MUSL_IMG"
echo

rc=0
could_not=0
fail() { printf 'FAIL %s\n' "$1"; rc=1; }
pass() { printf 'ok   %s\n' "$1"; }
cannot() { printf '  COULD NOT RUN: %s\n' "$1"; could_not=1; }

if [ ! -r "$GNU_SO" ] || [ ! -r "$MUSL_SO" ]; then
	echo "SKIP: the objects are not built. ./scripts/build-interpose.sh" >&2
	exit 2
fi

# The victim: one dynamic binary per libc, calling the setters and then
# reading the getters. Built here so the assertions below compare the
# object's answers, not a tool the image ships.
cat >"$WORK/victim.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <unistd.h>
#include <grp.h>
#include <sys/types.h>
int main(void) {
	int rc_u, rc_g, rc_s, e_u, e_g, e_s;
	int rc_eu, rc_eg, rc_ru, rc_rg, e_eu, e_eg, e_ru, e_rg;
	gid_t put[2] = {100, 101};
	gid_t got[8];
	int ng;
	unsigned r, e, s, gr, ge, gs;
	errno = 0; rc_u = setuid(1000); e_u = errno;
	errno = 0; rc_g = setgid(100); e_g = errno;
	errno = 0; rc_s = setgroups(2, put); e_s = errno;
	errno = 0; rc_eu = seteuid(1000); e_eu = errno;
	errno = 0; rc_eg = setegid(100); e_eg = errno;
	errno = 0; rc_ru = setresuid(-1, 1000, -1); e_ru = errno;
	errno = 0; rc_rg = setresgid(-1, 100, -1); e_rg = errno;
	ng = getgroups(8, got);
	getresuid(&r, &e, &s);
	getresgid(&gr, &ge, &gs);
	printf("SETUID_RC=%d ERR=%d\n", rc_u, e_u);
	printf("SETGID_RC=%d ERR=%d\n", rc_g, e_g);
	printf("SETGROUPS_RC=%d ERR=%d\n", rc_s, e_s);
	printf("SETEUID_RC=%d ERR=%d\n", rc_eu, e_eu);
	printf("SETEGID_RC=%d ERR=%d\n", rc_eg, e_eg);
	printf("SETRESUID_RC=%d ERR=%d\n", rc_ru, e_ru);
	printf("SETRESGID_RC=%d ERR=%d\n", rc_rg, e_rg);
	printf("GETUID=%u GETEUID=%u\n", getuid(), geteuid());
	printf("GETGID=%u GETEGID=%u\n", getgid(), getegid());
	printf("RESUID=%u:%u:%u\n", r, e, s);
	printf("RESGID=%u:%u:%u\n", gr, ge, gs);
	printf("GROUPS_N=%d GROUPS_0=%u GROUPS_1=%u\n", ng,
		ng > 0 ? (unsigned)got[0] : 999999,
		ng > 1 ? (unsigned)got[1] : 999999);
	/* ⭐ Partial updates run LAST, because they move the record the lines
	 * above assert. A `-1` effective leaves the effective where it is, and
	 * the saved follows the (new) effective, which is the kernel's own
	 * rule: after setuid(1000) the record reads 1000:1000:1000, so
	 * setreuid(2000,-1) must report 2000:1000:1000 and the group twin
	 * 200:100:100. Deterministic with or without a wall, because the
	 * record runs on both the granted and the refused path. */
	errno = 0; rc_u = setreuid(2000, -1); e_u = errno;
	errno = 0; rc_g = setregid(200, -1); e_g = errno;
	getresuid(&r, &e, &s);
	getresgid(&gr, &ge, &gs);
	printf("SETREUID_RC=%d ERR=%d\n", rc_u, e_u);
	printf("SETREGID_RC=%d ERR=%d\n", rc_g, e_g);
	printf("RERESUID=%u:%u:%u\n", r, e, s);
	printf("RERESGID=%u:%u:%u\n", gr, ge, gs);
	return 0;
}
EOF

HAVE_GLIBC_VICTIM=0
HAVE_MUSL_VICTIM=0
if cc -O1 -o "$WORK/victim-glibc" "$WORK/victim.c" 2>"$WORK/build-glibc.log"; then
	HAVE_GLIBC_VICTIM=1
elif command -v zig >/dev/null 2>&1 &&
	ZIG_TARGET=x86_64-linux-gnu.2.17 "$ROOT/scripts/zig-cc.sh" -O1 \
		-o "$WORK/victim-glibc" "$WORK/victim.c" 2>"$WORK/build-glibc.log"; then
	HAVE_GLIBC_VICTIM=1
else
	cannot "the glibc victim did not build; see $WORK/build-glibc.log"
fi
if command -v zig >/dev/null 2>&1 &&
	ZIG_TARGET=x86_64-linux-musl "$ROOT/scripts/zig-cc.sh" -O1 -dynamic \
		-o "$WORK/victim-musl" "$WORK/victim.c" 2>"$WORK/build-musl.log"; then
	HAVE_MUSL_VICTIM=1
else
	cannot "the musl victim did not build (needs zig); see $WORK/build-musl.log"
fi
# ⛔ A static victim cannot be interposed at all: asserting anything about it
# would measure the loader's silence, not this object.
for arm in glibc musl; do
	if [ "$arm" = glibc ]; then h="$HAVE_GLIBC_VICTIM"; else h="$HAVE_MUSL_VICTIM"; fi
	if [ "$h" -eq 1 ]; then
		printf '  victim-%s  %s\n' "$arm" "$(file -b "$WORK/victim-$arm" | cut -c1-80)"
		if file -b "$WORK/victim-$arm" | grep -q 'dynamically linked'; then
			pass "victim-$arm is dynamic, so the object can reach it"
		else
			fail "victim-$arm is not dynamically linked and cannot be interposed"
			if [ "$arm" = glibc ]; then HAVE_GLIBC_VICTIM=0; else HAVE_MUSL_VICTIM=0; fi
		fi
	fi
done
echo

# subject SO VICTIM IDENTITY PRELOAD -> the victim's report lines.
# PRELOAD is `export LD_PRELOAD=/i.so` for a loaded run and `true` for the
# bare control, which is 105's own shape: the same binary, the same environ
# apart from the variable, so any difference in the report is the object's.
#
# ⚠ The victim runs DIRECTLY, not under docker. The job container cannot run
# a docker daemon (measured 2026-09-19: dockerd fails creating the DOCKER
# NAT chain, `iptables ... Permission denied (you must be root)`, so the
# outer container holds no NET_ADMIN), and these clauses need none: the
# fake answers the record whether the kernel grants or refuses, and the
# control compares bare against loaded rather than against the kernel.
# A clause asserting an absolute honest-failure rc would need a wall and
# could not run here; none does, by design (see the header).
subject() {
	local so="$1" vic="$2" ident="$3" preload="$4" ldarg=
	[ "$preload" = yes ] && ldarg="LD_PRELOAD=$so"
	# shellcheck disable=SC2086
	if [ -n "$ident" ]; then
		timeout 120 env "PODBOX_IDENTITY=$ident" $ldarg "$vic" 2>"$WORK/diag"
	else
		timeout 120 env $ldarg "$vic" 2>"$WORK/diag"
	fi
	echo "DIAG=$(head -1 "$WORK/diag" 2>/dev/null | cut -c1-100)"
}

if [ "$HAVE_GLIBC_VICTIM" -eq 1 ]; then
	echo "== A. the control: without PODBOX_IDENTITY the object changes nothing"
	a_bare="$(subject "$GNU_SO" "$WORK/victim-glibc" '' no)"
	printf '%s\n' "$a_bare" | sed 's/^/  bare: /'
	a_pre="$(subject "$GNU_SO" "$WORK/victim-glibc" '' yes)"
	printf '%s\n' "$a_pre" | sed 's/^/  loaded: /'
	if [ "$(printf '%s' "$a_bare" | grep -v '^DIAG=')" = "$(printf '%s' "$a_pre" | grep -v '^DIAG=')" ]; then
		pass "A: the loaded victim reports exactly what the bare one does"
	else
		fail "A: the loaded victim's report differs from the bare one"
	fi
	echo
fi

# G. THE ERRNO-BY-CALL READING (TODO/interpose.md T-0711). The honest
# object reports each refused setter with its call and errno; this
# clause prints that table on the denying machine at hand and records
# it, asserting only that the victim ran. Errnos vary by wall (EPERM
# where the id is mapped but denied, EINVAL where it is unmapped), so
# no absolute value is asserted here by design (see the header).
if [ "$HAVE_GLIBC_VICTIM" -eq 1 ]; then
	echo "== G. the errno-by-call reading on this denying machine (recorded, not asserted)"
	timeout 120 env "LD_PRELOAD=$GNU_SO" "$WORK/victim-glibc" >"$WORK/g-table.txt" 2>"$WORK/g-diag.txt"
	if grep -q "^SETUID_RC=" "$WORK/g-table.txt" 2>/dev/null; then
		pass "G: the victim ran under the honest object"
		echo "  -- rc and errno per call, stdout:"
		grep -h "^SET.*_RC=" "$WORK/g-table.txt" | sed 's/^/  /'
		echo "  -- the refusal text, stderr:"
		grep -h "failed with errno" "$WORK/g-diag.txt" | sed 's/^/  /'
	else
		fail "G: the victim did not run under the honest object"
	fi
	echo
fi

expect_fake() {
	# $1 is the victim report. Every setter answered 0 and every getter
	# answers the requested record. ⛔ The victim prints getters in pairs
	# per line, so a start-of-line anchor misses the second of each pair:
	# the match is whitespace-or-edge delimited instead.
	local out="$1" bad=0
	for want in SETUID_RC=0 SETGID_RC=0 SETGROUPS_RC=0 \
		GETUID=1000 GETEUID=1000 GETGID=100 GETEGID=100 \
		RESUID=1000:1000:1000 RESGID=100:100:100 \
		GROUPS_N=2 GROUPS_0=100 GROUPS_1=101 \
		SETREUID_RC=0 SETREGID_RC=0 \
		RERESUID=2000:1000:1000 RERESGID=200:100:100; do
		if ! printf '%s\n' "$out" | grep -Eq "(^| )${want}($| )"; then
			printf '  missing: %s\n' "$want"
			bad=1
		fi
	done
	return "$bad"
}

if [ "$HAVE_GLIBC_VICTIM" -eq 1 ]; then
	echo "== B. the fake, glibc: PODBOX_IDENTITY=1000:100"
	b_out="$(subject "$GNU_SO" "$WORK/victim-glibc" '1000:100' yes)"
	printf '%s\n' "$b_out" | sed 's/^/  /'
	if expect_fake "$b_out"; then
		pass "B: the setters answer 0 and the getters answer 1000:100"
	else
		fail "B: the record does not answer what was requested"
	fi
	echo
fi

if [ "$HAVE_MUSL_VICTIM" -eq 1 ] && [ -x "$BIN" ]; then
	echo "== C. the fake, musl, with the musl-linked object"
	# ⚠ The musl victim cannot run on this glibc job container directly:
	# its interpreter is the musl loader, which is absent here (a direct
	# run dies ENOENT naming the binary rather than the loader). It runs
	# staged inside the alpine rootfs instead, through podbox itself,
	# which also preloads the musl object it places there.
	export PODBOX_STORE="$WORK/store"
	unset PODBOX_DEFAULT_PLATFORM DOCKER_DEFAULT_PLATFORM
	# ⛔ `extract` does not pull: the store here is fresh, so the image is
	# fetched first the same way `run` would fetch it.
	"$BIN" pull "$MUSL_IMG" >/dev/null 2>"$WORK/c-pull.err" || {
		fail "C: pull failed"
		sed 's/^/  /' "$WORK/c-pull.err"
	}
	"$BIN" extract "$MUSL_IMG" >/dev/null 2>"$WORK/c-extract.err" || {
		fail "C: extract failed"
		sed 's/^/  /' "$WORK/c-extract.err"
	}
	c_root="$("$BIN" inspect --format '{{.RootfsPath}}' "$MUSL_IMG" 2>"$WORK/c-inspect.err")" || {
		fail "C: inspect --format {{.RootfsPath}} failed"
		sed 's/^/  /' "$WORK/c-inspect.err"
	}
	if [ -n "${c_root:-}" ]; then
		cp "$WORK/victim-musl" "$c_root/victim-musl" || fail "C: staging failed"
		# C0 first: no flag, so the honest object must report what the
		# kernel says, which is the same lines clause A's bare glibc run
		# printed, because it is the same kernel answering the same calls.
		# ⛔ No `--rm` on either run below: it removes the extracted rootfs
		# when the payload exits, and with it the staged victim the second
		# run needs. The store here is a temporary directory the trap
		# removes, so nothing leaks.
		c0_out="$("$BIN" run "$MUSL_IMG" /victim-musl 2>"$WORK/c0.err")"
		printf '%s\n' "$c0_out" | sed 's/^/  honest-musl: /'
		if [ -z "${a_bare:-}" ]; then
			cannot "C0: clause A did not run, so there is no bare report to compare"
		elif [ "$c0_out" = "$(printf '%s' "$a_bare" | grep -v '^DIAG=')" ]; then
			pass "C0: the honest musl object reports what the kernel says"
		else
			fail "C0: the honest musl report differs from the bare one"
			sed 's/^/  stderr: /' "$WORK/c0.err"
		fi
		c_out="$("$BIN" run --user 1000:100 "$MUSL_IMG" /victim-musl 2>"$WORK/c.err")"
		printf '%s\n' "$c_out" | sed 's/^/  /'
		if expect_fake "$c_out"; then
			pass "C: the musl object answers the same record"
		else
			fail "C: the musl object's record does not answer what was requested"
			sed 's/^/  stderr: /' "$WORK/c.err"
		fi
	fi
	echo
elif [ "$HAVE_MUSL_VICTIM" -eq 1 ]; then
	cannot "$BIN is not an executable, so clause C cannot run"
fi

if [ -x "$BIN" ]; then
	export PODBOX_STORE="$WORK/store"
	unset PODBOX_DEFAULT_PLATFORM DOCKER_DEFAULT_PLATFORM
	echo "== D. end to end: podbox run --user fakes id on both libcs"
	d_out="$("$BIN" run --rm --user 1000:100 "$MUSL_IMG" id -u 2>"$WORK/d.err")"
	d_rc=$?
	d_banner="$(cat "$WORK/d.err")"
	printf '  musl id -u: %s (rc=%s)\n' "$d_out" "$d_rc"
	printf '%s\n' "$d_banner" | sed 's/^/  banner: /'
	if [ "$d_out" = "1000" ] && [ "$d_rc" -eq 0 ]; then
		pass "D: musl id -u answers the requested 1000"
	else
		fail "D: musl id -u answered [$d_out] rc=$d_rc"
	fi
	if printf '%s' "$d_banner" | grep -q 'identity faked to 1000:100'; then
		pass "D2: the banner names the faked identity"
	else
		fail "D2: the banner does not name the faked identity"
	fi
	dg_out="$("$BIN" run --rm --user 1000:100 "$GLIBC_IMG" sh -c 'id -u; id -g' 2>"$WORK/dg.err")"
	printf '  glibc id: %s\n' "$(printf '%s' "$dg_out" | tr '\n' ' ')"
	if [ "$(printf '%s' "$dg_out" | sed -n '1p')" = "1000" ] &&
		[ "$(printf '%s' "$dg_out" | sed -n '2p')" = "100" ]; then
		pass "D3: glibc id answers 1000 and 100"
	else
		fail "D3: glibc id answered [$(printf '%s' "$dg_out" | tr '\n' '/')]"
	fi
	d0_out="$("$BIN" run --rm "$MUSL_IMG" id -u 2>"$WORK/d0.err")"
	printf '  no-flag id -u: %s\n' "$d0_out"
	if [ "$d0_out" = "0" ]; then
		pass "D4: without --user the payload is still uid 0"
	else
		fail "D4: without --user id -u answered [$d0_out]"
	fi
	echo

	echo "== E. --strict refuses the faked run"
	# ⚠ The chroot rung refuses under --strict on its own (the floor is a
	# namespace), so the assertion is the NAMING: --user must be among the
	# reasons, which is what stops the flag reading as honoured-but-silent.
	"$BIN" run --rm --strict --user 1000:100 "$MUSL_IMG" id -u \
		>"$WORK/e.out" 2>"$WORK/e.err"
	e_rc=$?
	printf '  rc=%s out=[%s]\n' "$e_rc" "$(cat "$WORK/e.out")"
	sed 's/^/  err: /' "$WORK/e.err"
	if [ "$e_rc" -ne 0 ] && grep -q '\-\-user' "$WORK/e.err"; then
		pass "E: --strict refuses, naming --user"
	else
		fail "E: rc=$e_rc and the refusal does not name --user"
	fi
	echo

	echo "== F. a declined tier carries no memo: static --user still runs"
	"$BIN" extract "$MUSL_IMG" >/dev/null 2>"$WORK/extract.err" || {
		fail "F: extract failed"
		sed 's/^/  /' "$WORK/extract.err"
	}
	f_root="$("$BIN" inspect --format '{{.RootfsPath}}' "$MUSL_IMG" 2>"$WORK/inspect.err")" || {
		fail "F: inspect --format {{.RootfsPath}} failed"
		sed 's/^/  /' "$WORK/inspect.err"
	}
	if [ -n "${f_root:-}" ]; then
		cp "$BIN" "$f_root/podbox" || fail "F: staging the static payload failed"
		f_out="$("$BIN" run --rm --user 1000:100 "$MUSL_IMG" /podbox --version 2>&1)"
		f_rc=$?
		printf '%s\n' "$f_out" | sed 's/^/  run output: /'
		if printf '%s\n' "$f_out" | grep -q 'interpose: declined'; then
			echo "ok   F: the static payload is declined"
		else
			fail "F: no 'interpose: declined' line"
		fi
		if printf '%s\n' "$f_out" | grep -q 'has no interposed payload to act through'; then
			pass "F2: the decline says --user changes nothing here"
		else
			fail "F2: the decline does not say --user changes nothing"
		fi
		if [ "$f_rc" -eq 0 ]; then
			echo "ok   F3: the declined payload still ran (exit 0)"
		else
			fail "F3: the run exited $f_rc; a decline must not fail the run"
		fi
	fi
	echo
else
	cannot "$BIN is not an executable, so clauses D, E and F cannot run"
fi

echo "== verdict"
if [ "$rc" -eq 0 ] && [ "$could_not" -eq 0 ]; then
	echo "  every check ran and matched. The identity tier refuses honestly by"
	echo "  default, fakes only under --user, and the banner says what the call did."
elif [ "$rc" -eq 0 ]; then
	echo "  every check that ran matched; one or more could not be taken here."
else
	echo "  see the FAIL lines above"
fi
[ "$rc" -ne 0 ] || [ "$could_not" -eq 0 ] || exit 2
exit "$rc"
# ⛔ No pipe to `tee` here: every stage of a pipeline runs in a subshell,
# so `exit "$rc"` after one would always see the initial 0 and a failing
# run would exit green. The report goes to the file first, then to stdout.
} >"$OUT" 2>&1
cat "$OUT"
