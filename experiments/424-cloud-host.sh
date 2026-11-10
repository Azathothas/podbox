#!/bin/sh
# 424-cloud-host.sh - what does the host of a cloud session grant?
#
# TODO/probe.md T-1670. The question: which mechanisms of podvm's ladder
# does the host of a Claude Code cloud session permit, and which
# registries answer through its egress proxy? `podvm probe --json` and
# `podvm doctor` measure most mechanisms; this script adds the rows that
# they lack: Syscall User Dispatch, openat2(RESOLVE_IN_ROOT), a request to
# each registry of DISTRO_ROWS_M5, and the free blocks and inodes.
#
#   sh experiments/424-cloud-host.sh [PODVM]
#
# PODVM is the podvm binary (default: the release build of this tree).
# Exit codes: 0 each row was measured (a denied mechanism is a measured
# answer, not a failure), 1 a row could not be measured, 2 the script could
# not run (no C compiler, not Linux). A missing observation is never a
# denial.
set -u

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/.." && pwd)
podvm=${1:-$repo/target/x86_64-unknown-linux-musl/release/podvm}
[ "$(uname -s)" = Linux ] || { echo "424: Linux only" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "424: no C compiler (cc)" >&2; exit 2; }
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT
rc=0
row() { printf '%-34s %s\n' "$1" "$2"; }
miss() { row "$1" "NOT MEASURED: $2"; rc=1; }

echo "== conditions"
row "date" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
row "kernel" "$(uname -r) $(uname -m)"
row "uid" "$(id -u)"
row "cc" "$(cc --version 2>/dev/null | head -n 1)"
row "podvm" "$("$podvm" version 2>/dev/null || echo '- (not built)')"
if [ -n "${HTTPS_PROXY:-}" ]; then row "egress" "through HTTPS_PROXY"; else row "egress" "direct"; fi
echo

echo "== 1. devices and kernel switches"
for d in /dev/kvm /dev/fuse /dev/ptmx /dev/net/tun; do
	if [ -e "$d" ]; then row "$d" "present"; else row "$d" "absent"; fi
done
row "seccomp (this shell)" "$(awk '/^Seccomp:/ {print $2}' /proc/self/status)"
for f in /proc/sys/kernel/unprivileged_userns_clone /proc/sys/user/max_user_namespaces /proc/sys/vm/memfd_noexec /proc/sys/kernel/yama/ptrace_scope; do
	if [ -r "$f" ]; then row "$f" "$(cat "$f")"; else row "$f" "-"; fi
done
echo

echo "== 2. namespaces and chroot"
if unshare -U true 2>/dev/null; then row "unshare -U" "allowed"; else row "unshare -U" "denied"; fi
if unshare -Urm true 2>/dev/null; then row "unshare -Urm" "allowed"; else row "unshare -Urm" "denied"; fi
if chroot / true 2>/dev/null; then row "chroot / (this uid)" "allowed"; else row "chroot / (this uid)" "denied"; fi
echo

echo "== 3. Syscall User Dispatch and openat2(RESOLVE_IN_ROOT)"
cat >"$work/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>
#ifndef PR_SET_SYSCALL_USER_DISPATCH
#define PR_SET_SYSCALL_USER_DISPATCH 59
#endif
#ifndef PR_SYS_DISPATCH_ON
#define PR_SYS_DISPATCH_ON 1
#define PR_SYS_DISPATCH_OFF 0
#endif
struct oh { unsigned long long flags, mode, resolve; };
int main(void) {
	static char selector = 0; /* SYSCALL_DISPATCH_FILTER_ALLOW */
	long r = prctl(PR_SET_SYSCALL_USER_DISPATCH, PR_SYS_DISPATCH_ON, 0, 0, &selector);
	if (r == 0) {
		prctl(PR_SET_SYSCALL_USER_DISPATCH, PR_SYS_DISPATCH_OFF, 0, 0, 0);
		printf("sud allowed\n");
	} else {
		printf("sud denied errno=%d (%s)\n", errno, strerror(errno));
	}
	int dir = open("/", O_RDONLY | O_DIRECTORY);
	struct oh how = { O_RDONLY, 0, 0x10 /* RESOLVE_IN_ROOT */ };
	long fd = syscall(437 /* openat2 */, dir, "etc/../../etc/hostname", &how, sizeof how);
	if (fd >= 0) printf("openat2 allowed\n");
	else printf("openat2 denied errno=%d (%s)\n", errno, strerror(errno));
	return 0;
}
EOF
if cc -O2 -o "$work/probe" "$work/probe.c" 2>"$work/cc.txt"; then
	"$work/probe" >"$work/probe.txt" 2>&1
	row "prctl(SYSCALL_USER_DISPATCH)" "$(sed -n 's/^sud //p' "$work/probe.txt")"
	row "openat2(RESOLVE_IN_ROOT)" "$(sed -n 's/^openat2 //p' "$work/probe.txt")"
else
	miss "SUD and openat2" "the probe did not compile: $(head -n 1 "$work/cc.txt")"
fi
echo

echo "== 4. registries through the egress (GET /v2/; 200 or 401 is reachable)"
. "$repo/scripts/common/distro-matrix.sh"
seen=" "
for line in $DISTRO_ROWS_M5; do
	reg=${line%%/*}
	case "$seen" in *" $reg "*) continue ;; esac
	seen="$seen$reg "
	code=$(curl -qsS -o /dev/null -w '%{http_code}' --max-time 20 "https://$reg/v2/" 2>/dev/null) || code="-"
	row "$reg" "$code"
done
for reg in registry-1.docker.io mirror.gcr.io; do
	code=$(curl -qsS -o /dev/null -w '%{http_code}' --max-time 20 "https://$reg/v2/" 2>/dev/null) || code="-"
	row "$reg" "$code"
done
echo

echo "== 5. space"
df -Pk "$repo" | awk 'NR==2 {printf "%-34s %s KiB free of %s\n", "blocks at the repository", $4, $2}'
df -Pi "$repo" | awk 'NR==2 {printf "%-34s %s free of %s\n", "inodes at the repository", $4, $2}'
echo

echo "== 6. podvm doctor"
if [ -x "$podvm" ]; then
	"$podvm" doctor 2>&1
	row "doctor exit" "$?"
else
	miss "podvm doctor" "no podvm binary at $podvm"
fi
exit "$rc"
