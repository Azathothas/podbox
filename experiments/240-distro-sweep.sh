#!/usr/bin/env bash
# Question: on ten distributions, does `podbox run` install a C TOOLCHAIN
# through the distribution's own package manager and build and run a program
# with it, with no user-supplied fixups?
#
# TODO/milestones.md T-1106, M5's acceptance. TODO/complete.md T-0401 to T-0411
# are the fixups it exercises.
#
# ⭐ WHY A C TOOLCHAIN AND NOT A TRIVIAL PACKAGE. It exercises the ownership
# wall, the resolver, the keyring and the sandbox user AT ONCE, and a two-file
# project catches a toolchain that installed and cannot link. `apk add curl`
# would pass with half of M5 missing.
#
# ⭐ EVERY MEMBER OF THE SET IS THERE BECAUSE IT BROKE SOMETHING:
#
#   alpine         musl, apk, no nsswitch.conf at all              (T-0410)
#   debian         apt, the _apt sandbox user, http:// sources     (T-0407, T-0411)
#   ubuntu         the same, one release apart
#   archlinux      pacman's DownloadUser, and the keyring          (T-0406)
#   almalinux      dnf, and a glibc rootfs with no ca bundle
#   rocky          the libvirt gateway baked into resolv.conf      (T-0402)
#   rocky-minimal  microdnf, which is a different binary
#   fedora         dnf, newest glibc
#   opensuse-leap  zypper, whose repos.d is REGENERATED            (T-0408)
#   voidlinux-musl xbps, and ownership errors from the unpacker    (T-0409)
#
# ⛔ THE MECHANICS ARE T-1203'S, NOT A SECOND RUNNER. The pinning, the `no-pull`
# row, the exit-2-when-nothing-ran rule and the reference qualification are in
# `scripts/common/distro-matrix.sh`, which `125-across-distributions.sh` also
# sources. This file supplies the subject and reads the M5 row list.
#
# ⛔ NO DOCKER HUB. ghcr.io, public.ecr.aws and the distributions' own
# registries. Docker Hub's rate limit is attributed to a shared address in this
# environment and a limited pull reads as a broken registry.
#
# ⭐ **A ROW THAT FAILS IS RUN AGAIN UNDER THE ENGINE, WITH THE SAME BYTES AND NO
# HELP.** That control is what separates "podbox is missing a fixup" from "this
# machine cannot do it either", and both are real answers with different next
# moves. Measured on 2026-09-09: this host intercepts TLS, and `apk add gcc`
# inside `alpine` failed under docker exactly as it did under podbox until the
# completion layer installed the machine's announced CA bundle. A row where
# docker fails the same way reads `host` and does not fail the sweep; the
# transcript carries docker'"'"'s output beside podbox'"'"'s so a reader can check it.
#
#   ./240-distro-sweep.sh            every row
#   ./240-distro-sweep.sh alpine     one row, by its local name
#
# Exit: 0 every row that pulled built and ran the program, 1 a row pulled and
#       did not, 2 nothing ran.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="${OUT:-$REPO/experiments/results/distro-sweep.txt}"
TRANSCRIPTS="${TRANSCRIPTS:-$REPO/experiments/results/sweep}"
# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there. The lane's scratch
# lives under the checkout on a non-native lane: mount sources must be
# Windows paths there (245's rule), and /tmp/... names nothing.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep240-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2 ;;
esac
trap 'rm -rf "$WORK"' EXIT INT TERM

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. The rows, the subject and what each row
# asserts are unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
# The rows cannot run without the binary: natively, or staged in the
# driver. The control needs the engine on either lane.
if [ "$NATIVE" -eq 0 ]; then
	[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine on a lane where the podbox binary does not execute" >&2; exit 2; }
	[ -r "$BIN" ] || {
		echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2
		exit 2
	}
fi

# The driver: the M5 debian row, pinned. It hosts the staged podbox binary
# for every `$BIN` call on a non-native lane. No capability wall here: 240
# measures real installs, and 245's CHOWN wall belongs to the interposer.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
STAGE="$WORK/stage"

ONLY="${1:-}"
# ⚠ Generous, and bounded. A dnf transaction over a cold cache is minutes, and
# an unbounded wait is the failure RULES.md section 8 exists to prevent.
ROW_TIMEOUT="${PODBOX_ROW_TIMEOUT:-1200}"
# The driver's bound covers a first pull plus the row's run; the control
# runs on the host engine, outside it.
OUTER="$((ROW_TIMEOUT + 900))"

if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || {
		echo "SKIP: $BIN is not an executable. ./scripts/dev.sh build" >&2
		exit 2
	}
fi
# shellcheck source=../scripts/common/distro-matrix.sh
. "$REPO/scripts/common/distro-matrix.sh"

export PODBOX_STORE="${PODBOX_SWEEP_STORE:-$WORK/store}"
mkdir -p "$TRANSCRIPTS"

# ⛔ DETECTED BEFORE THE HEADER THAT REPORTS IT. It was below the loop, so under
# `set -u` the conditions block read an unset variable, printed
# `have_docker: unbound variable` and then an EMPTY `control` line -- which
# reads as "no control" on a run that had one.
# have_docker is HAVE_ENGINE now: the helper picked a daemon or host
# podman above, and the warning stays because the rule it states still holds.

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
if [ "$NATIVE" -eq 1 ]; then
	_ver="$("$BIN" version)"
	_entered="$("$BIN" system info --format '{{.EnteredRung}}')"
	_permits="$("$BIN" system info --format '{{.Rung}}')"
else
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE"
	cp "$BIN" "$STAGE/podbox"
	eng_mount "$STAGE/podbox" /pb || exit 2
	_ver="$(eng_run 60 "$DRIVER" "" -- /pb version 2>&1)"
	_entered="$(eng_run 60 "$DRIVER" "" -- /pb system info --format '{{.EnteredRung}}' 2>&1)"
	_permits="$(eng_run 60 "$DRIVER" "" -- /pb system info --format '{{.Rung}}' 2>&1)"
	eng_clear
fi
printf 'podbox            %s\n' "$_ver"
# ⛔ TWO ANSWERS. This printed `.Rung`, which is the rung THIS MACHINE PERMITS,
# under the label "rung podbox uses" -- so on this host the header said
# `namespace` for a sequence that chroots and creates no namespace, which is
# exactly what TODO/cli.md T-0804 was opened for. `.EnteredRung` is
# `podbox_enter::ENTERED_RUNG`, the sequence podbox actually performs.
printf 'rung podbox enters %s\n' "$_entered"
printf 'rung this machine  %s   (permits, and podbox does not use it)\n' \
	"$_permits"
printf 'rows              %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
printf 'row timeout       %s s\n' "$ROW_TIMEOUT"
printf 'store             %s\n' "\$PODBOX_STORE (a fresh one per run unless set)"
printf 'transcripts       %s\n' "experiments/results/sweep/"
printf 'control           %s\n' \
	"$([ "$HAVE_ENGINE" -eq 1 ] && echo "$ENGINE_NAME, on any row podbox fails" || echo 'ABSENT: no engine, so a failure cannot be attributed')"
echo

# ------------------------------------------------------------------ the subject
#
# ⛔ ONE SCRIPT FOR EVERY ROW, and it detects the package manager rather than
# being told which one. A per-row command list would be ten subjects, and a
# difference between rows would then be a difference between the commands.
#
# ⚠ It writes into /tmp inside the container, never beside itself: the rootfs is
# content-addressed and shared between containers (TODO/image.md T-0204), and a
# build artefact left in it is one the next row inherits.
cat >"$WORK/subject.sh" <<'SUBJECT'
#!/bin/sh
# The M5 subject. Print one KEY=VALUE per line; the runner reads those.
set -u
say() { printf '%s\n' "$*"; }

# 1. what is here, before anything is installed.
say "ID=$(id -u):$(id -g)"
if [ -e /dev/null ] && [ ! -s /dev/null ]; then say "DEVNULL=usable"; else say "DEVNULL=MISSING"; fi
echo probe >/dev/null 2>&1 && say "DEVNULL_WRITE=ok" || say "DEVNULL_WRITE=failed"
if [ -r /etc/resolv.conf ]; then
  say "RESOLVER=$(grep -m1 '^nameserver' /etc/resolv.conf 2>/dev/null | awk '{print $2}')"
else
  say "RESOLVER=absent"
fi
if [ -r /etc/nsswitch.conf ]; then
  say "NSSWITCH=$(grep -E '^[[:space:]]*passwd' /etc/nsswitch.conf | head -1 | tr -s ' \t' ' ' | sed 's/^ *passwd: *//')"
else
  say "NSSWITCH=absent"
fi
# ⭐ T-0404's own question, and it is asked with the tool the DISTRIBUTION
# ships rather than by reading the file: `getent` goes through NSS on glibc,
# which is the whole of T-0410.
if command -v getent >/dev/null 2>&1; then
  say "GETENT_ROOT=$(getent passwd 0 2>/dev/null | cut -d: -f1)"
else
  say "GETENT_ROOT=no-getent"
fi

# 2. the package manager, detected.
PM=none
for p in apk apt-get pacman dnf microdnf zypper xbps-install; do
  command -v "$p" >/dev/null 2>&1 && { PM="$p"; break; }
done
say "PM=$PM"

# 3. install a C toolchain with it. ⚠ Every one is non-interactive by flag:
# a prompt with no terminal behind it is a hang, and a hang costs the run.
install_rc=0
case "$PM" in
apk)      apk add --no-cache gcc musl-dev >/tmp/inst.log 2>&1 || install_rc=$? ;;
apt-get)  DEBIAN_FRONTEND=noninteractive apt-get update -qq >/tmp/inst.log 2>&1 &&
          DEBIAN_FRONTEND=noninteractive apt-get install -y -qq gcc libc6-dev >>/tmp/inst.log 2>&1 || install_rc=$? ;;
pacman)   pacman -Sy --noconfirm --needed gcc >/tmp/inst.log 2>&1 || install_rc=$? ;;
dnf)      dnf install -y --setopt=install_weak_deps=False gcc glibc-devel >/tmp/inst.log 2>&1 || install_rc=$? ;;
microdnf) microdnf install -y gcc glibc-devel >/tmp/inst.log 2>&1 || install_rc=$? ;;
zypper)   zypper --non-interactive --gpg-auto-import-keys refresh >/tmp/inst.log 2>&1;
          zypper --non-interactive install gcc glibc-devel >>/tmp/inst.log 2>&1 || install_rc=$? ;;
xbps-install) xbps-install -Suy >/tmp/inst.log 2>&1;
          xbps-install -y gcc >>/tmp/inst.log 2>&1 || install_rc=$? ;;
*)        install_rc=127 ;;
esac
say "INSTALL_RC=$install_rc"
# ⚠ The tail of the log travels with the row, or a failure is a number with no
# cause attached to it.
say "INSTALL_TAIL=$(tail -3 /tmp/inst.log 2>/dev/null | tr '\n' ' ' | tr -s ' ' | cut -c1-220)"

# 4. ⛔ TWO FILES, so a toolchain that installed and cannot LINK is caught.
mkdir -p /tmp/p
cat >/tmp/p/helper.h <<'EOF'
int answer(void);
EOF
cat >/tmp/p/helper.c <<'EOF'
#include "helper.h"
int answer(void) { return 42; }
EOF
cat >/tmp/p/main.c <<'EOF'
#include <stdio.h>
#include "helper.h"
int main(void) { printf("BUILT_AND_RAN=%d\n", answer()); return 0; }
EOF
if command -v cc >/dev/null 2>&1; then CC=cc; else CC=gcc; fi
say "CC=$(command -v $CC 2>/dev/null || echo none)"
if command -v $CC >/dev/null 2>&1; then
  say "CC_VERSION=$($CC --version 2>&1 | head -1)"
  ( cd /tmp/p && $CC -O1 -o prog main.c helper.c ) >/tmp/build.log 2>&1
  say "BUILD_RC=$?"
  say "BUILD_TAIL=$(tail -2 /tmp/build.log 2>/dev/null | tr '\n' ' ' | cut -c1-200)"
  if [ -x /tmp/p/prog ]; then /tmp/p/prog; else say "BUILT_AND_RAN=no-binary"; fi
else
  say "CC_VERSION=absent"
  say "BUILD_RC=127"
  say "BUILT_AND_RAN=no-compiler"
fi
SUBJECT
chmod +x "$WORK/subject.sh"

ran=0
nopull=0
broken=0
host=0
passed=0
fail=0

printf '%-14s %-8s %-13s %-9s %-7s %s\n' ROW LIBC PM INSTALL BUILD 'RAN'
printf '%.0s-' $(seq 1 74)
echo

# ⭐ The row's whole podbox sequence in one driver call. The subject
# arrives as a staged file and is read inside (245's rule: two shells
# re-parse argv between here and the driver). Stdout carries only stage
# failures; every reading lands in /w/row-NAME/ on the shared scratch.
cat >"$WORK/row-driver.sh" <<'ROWDRIVER_EOF'
#!/bin/sh
# row-driver.sh PINNED NAME ROW_TIMEOUT - one matrix row in the driver.
set -u
# ⭐ Container-local store: nothing on the host reads the store's bytes on
# this sweep (240 has no refusal loop), and the shared Windows-backed
# scratch cannot hold archlinux's case-colliding terminfo names (245's
# finding, measured 2026-09-19). Extraction runs on overlayfs here.
export PODBOX_STORE=/tmp/ostore
# The lane's CA announcement: the provisioned bundle, by the path the
# driver sees. Native lanes inherit the machine's own announcement.
if [ -f /w/cacert.pem ]; then
	export SSL_CERT_FILE=/w/cacert.pem
fi
pinned="$1"
name="$2"
to="$3"
d="/w/row-$name"
mkdir -p "$d" || exit 6
rm -rf /tmp/ostore
/pb pull "$pinned" >"$d/pull.log" 2>&1 || exit 3
timeout "$to" /pb run --rm "$pinned" /bin/sh -c "$(cat /subj/subject.sh)" >"$d/out" 2>"$d/err"
echo "$?" >"$d/rowrc"
exit 0
ROWDRIVER_EOF

# ⭐ Staged once: the same files serve every row. The objects stay on the
# host; only the binary enters the driver, because only it has to execute
# there. /w is the shared scratch the driver writes its readings to.
if [ "$NATIVE" -eq 0 ]; then
	mkdir -p "$WORK/w" "$STAGE"
	cp "$BIN" "$STAGE/podbox"
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_mount "$WORK/subject.sh" /subj/subject.sh \
		&& eng_mount "$WORK/row-driver.sh" /drv/row.sh \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
fi

# ⭐ The driver announces the standard roots. podbox-complete upgrades
# package sources to https where the mirror speaks it (debian, ubuntu,
# almalinux, rocky: measured 2026-09-19), and appends THIS MACHINE's CA
# bundle into the rootfs -- but only one the machine announced through
# $SSL_CERT_FILE / $CURL_CA_BUNDLE / $REQUESTS_CA_BUNDLE. A bare
# bookworm-slim announces nothing and ships no bundle, so every https
# fetch died with "No system certificates available" while the pristine
# control (still http) succeeded: five ⛔ lines that blamed podbox for a
# missing lane provision.
# ⭐ The bundle travels as a FILE, not an install: containers are removed
# after each run, so an apt install per row would cost minutes ten times
# over. One install, copied to the shared scratch, announced by path from
# every row driver after it. A normal Linux host announces by existing;
# this is the same announcement, provisioned once.
if [ "$NATIVE" -eq 0 ]; then
	eng_run 600 "$DRIVER" "" -- /bin/sh -c 'DEBIAN_FRONTEND=noninteractive apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ca-certificates && cp /etc/ssl/certs/ca-certificates.crt /w/cacert.pem && test -s /w/cacert.pem' \
		>/dev/null 2>"$WORK/ca-install.log" || {
		echo "SKIP: the CA bundle did not install; see $WORK/ca-install.log" >&2
		exit 2
	}
fi

while IFS='|' read -r ref name libc digest; do
	[ -n "${ref:-}" ] || continue
	[ -z "$ONLY" ] || [ "$ONLY" = "$name" ] || continue
	pinned="$(distro_pinned "$ref" "$digest")"
	t="$TRANSCRIPTS/$name.out"

	# ⭐ Non-native lane: one driver call runs the row's pull and run and
	# normalizes every reading to the names the native path writes, so the
	# transcript and every check below are shared.
	if [ "$NATIVE" -eq 0 ]; then
		d="$WORK/w/row-$name"
		drc_out="$(eng_run "$OUTER" "$DRIVER" "" -- /bin/sh /drv/row.sh "$pinned" "$name" "$ROW_TIMEOUT" 2>&1)"; drc=$?
		case "$drc" in
		0)
			cp "$d/pull.log" "$WORK/pull.$name" && cp "$d/out" "$WORK/out.$name" && cp "$d/err" "$WORK/err.$name" || {
				printf '%-14s %-8s %-13s %-9s %-7s %s\n' "$name" "$libc" - - - 'driver files missing'
				echo "driver files missing for $pinned" >"$t"
				broken=$((broken + 1))
				fail=1
				continue
			}
			row_rc="$(cat "$d/rowrc")" ;;
		3)
			printf '%-14s %-8s %-13s %-9s %-7s %s\n' "$name" "$libc" - no-pull - -
			{
				echo "no-pull $pinned"
				tail -3 "$d/pull.log"
			} >"$t"
			nopull=$((nopull + 1))
			continue ;;
		*)
			printf '%-14s %-8s %-13s %-9s %-7s %s\n' "$name" "$libc" - - - "driver rc=$drc"
			{
				echo "driver rc=$drc for $pinned"
				printf '%s\n' "$drc_out" | tail -5
			} >"$t"
			broken=$((broken + 1))
			fail=1
			continue ;;
		esac
	elif ! timeout 900 "$BIN" pull "$pinned" >"$WORK/pull.$name" 2>&1; then
		printf '%-14s %-8s %-13s %-9s %-7s %s\n' "$name" "$libc" - no-pull - -
		{
			echo "no-pull $pinned"
			tail -3 "$WORK/pull.$name"
		} >"$t"
		nopull=$((nopull + 1))
		continue
	fi

	# ⚠ THE SUBJECT TRAVELS AS ONE ARGV ELEMENT, not as a file. podbox cannot
	# bind-mount -- that is the premise of the whole runtime -- so `-v` is not
	# available, and `create` plus `cp` plus `start` would be three more verbs
	# between the question and the answer. `sh -c "$(cat ...)"` passes the script
	# through `execve`'s argument vector, which the shell does not re-parse.
	#
	# ⛔ `--rm`: each row extracts a rootfs and then installs a toolchain into
	# it, and ten of those is gigabytes. The rootfs is content-addressed and
	# shared, so leaving it would also carry one row's installed packages into
	# any later run against the same digest. On the driver lane the run
	# already happened above; this is the native lane's.
	if [ "$NATIVE" -eq 1 ]; then
		timeout "$ROW_TIMEOUT" "$BIN" run --rm "$pinned" /bin/sh -c "$(cat "$WORK/subject.sh")" \
			>"$WORK/out.$name" 2>"$WORK/err.$name"
		row_rc=$?
	fi
	{
		printf '# %s\n# pinned %s\n# podbox run exit %s\n' "$name" "$pinned" "$row_rc"
		# ⚠ THE PULL TRANSCRIPT TRAVELS WITH EVERY ROW, not only with a row that
		# failed to pull. T-0214's retry fires inside a pull that then succeeds,
		# and a transcript kept only on failure can never show it: the run that
		# needed it is exactly the run that no longer fails.
		echo "--- the pull"
		tail -4 "$WORK/pull.$name" | cut -c1-200
		echo "--- stdout"
		cat "$WORK/out.$name"
		echo "--- the completion layer said"
		grep 'podbox: complete:' "$WORK/err.$name" | cut -c1-200
		echo "--- the last of stderr"
		grep -v 'podbox: complete:' "$WORK/err.$name" | tail -5 | cut -c1-200
	} >"$t"

	get() { sed -n "s/^$1=//p" "$WORK/out.$name" | head -1; }
	# ⛔ The control, run only where podbox did not reach 42, because it costs a
	# second install of a toolchain and answers nothing on a row that passed.
	control() {
		if [ "$HAVE_ENGINE" -eq 0 ]; then
			printf '%s' "-"
			return
		fi
		# The subject travels as argv, exactly as it did under docker: the
		# argv probe on host podman carries spaces, quotes and newlines
		# intact, measured 2026-09-19.
		eng_run "$ROW_TIMEOUT" "$pinned" "" -- /bin/sh -c "$(cat "$WORK/subject.sh")" \
			>"$WORK/dout.$name" 2>"$WORK/derr.$name"
		{
			echo "--- the control: the same subject under $ENGINE_NAME, no help"
			cat "$WORK/dout.$name"
			tail -3 "$WORK/derr.$name"
		} >>"$t"
		sed -n 's/^BUILT_AND_RAN=//p' "$WORK/dout.$name" | head -1
	}
	pm="$(get PM)"
	inst="$(get INSTALL_RC)"
	build="$(get BUILD_RC)"
	built="$(get BUILT_AND_RAN)"
	if [ -z "$pm" ]; then
		printf '%-14s %-8s %-13s %-9s %-7s %s\n' "$name" "$libc" - "rc=$row_rc" - 'subject did not run'
		broken=$((broken + 1))
		fail=1
		continue
	fi
	ran=$((ran + 1))
	[ "$built" = "42" ] && passed=$((passed + 1))
	printf '%-14s %-8s %-13s %-9s %-7s %s\n' \
		"$name" "$libc" "${pm:--}" "${inst:--}" "${build:--}" "${built:--}"
	# ⛔ THE ROW PASSES ONLY IF THE PROGRAM RAN. An install that reported 0 and a
	# compiler that cannot link is the failure a two-file project exists to see.
	if [ "$built" != "42" ]; then
		dbuilt="$(control)"
		if [ "$dbuilt" = "42" ]; then
			printf '%-14s   ⛔ podbox failed and %s SUCCEEDED on the same image.\n' "$name" "$ENGINE_NAME"
			printf '%-14s      %s\n' "" "$(get INSTALL_TAIL | cut -c1-110)"
			fail=1
		else
			printf '%-14s   ⚠ the host: %s failed here too (BUILT_AND_RAN=%s).\n' \
				"$name" "$ENGINE_NAME" "${dbuilt:--}"
			printf '%-14s      %s\n' "" "$(get INSTALL_TAIL | cut -c1-110)"
			host=$((host + 1))
		fi
	fi
done <<EOF
$(printf '%s\n' "$DISTRO_ROWS_M5")
EOF

verdict_rc=0
distro_verdict "$ran" "$nopull" "$broken" || verdict_rc=$?
if [ "$host" -gt 0 ]; then
	printf '  ⚠ %s row(s) failed under podbox AND under %s on the same image,\n' "$host" "$ENGINE_NAME"
	printf '    so what they measure is this machine rather than either runtime.\n'
	printf '    Their transcripts carry both outputs.\n'
fi
printf '  transcripts in experiments/results/sweep/\n'

{
	printf '# M5 acceptance: a C toolchain through each distribution own package manager\n'
	printf '# TODO/milestones.md T-1106. Taken %s on kernel %s\n' \
		"$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -r)"
	printf '# ⛔ no docker.io: ghcr.io, public.ecr.aws and the distributions own registries\n'
	printf 'rows              %s\n' "$(distro_count_rows "$DISTRO_ROWS_M5")"
	printf 'ran               %s\n' "$ran"
	printf 'no_pull           %s\n' "$nopull"
	printf 'harness_failed    %s\n' "$broken"
	# ⛔ COUNTED IN THE LOOP, never by grepping the transcripts. Measured on
	# 2026-09-09: the transcript of a failing row also carries the engine
	# control's output, so `grep -l BUILT_AND_RAN=42` counted a row podbox
	# failed as a row podbox passed. The loop is not in a subshell here, so its
	# counters survive; 125-across-distributions.sh recounts from files because
	# its loop IS piped, and that difference is why this one does not.
	printf 'built_and_ran     %s\n' "$passed"
	printf 'host_not_runtime  %s\n' "$host"
} >"$OUT"

echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$verdict_rc" -eq 0 ] || exit "$verdict_rc"
[ "$fail" -eq 0 ] || exit 1
exit 0
