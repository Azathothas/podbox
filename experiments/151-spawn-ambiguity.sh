#!/usr/bin/env bash
# Question: behind one identical Go spawn failure, does podbox name the
# refused call instead of repeating the payload's ambiguous string?
#
# TODO/cli.md T-0809 (wall 2 of the sandbox-insights walls document).
#
# Two Go payloads spawn /bin/true: one with namespace clone flags and no
# credential, one with a credential and no clone flags. Where the kernel
# refuses the call, both fail with the byte-identical string
# `fork/exec /bin/true: operation not permitted`. `podbox probe` measures
# both legs itself, so its evidence names the refused call per context
# instead of echoing the string.
#
# Exit: 0 every runnable clause held, 1 one of them did not, 2 could not run.
#
# ⚠ `set -u` and no `pipefail`: this script runs under `sh` in a Linux job
# container, where `sh` is dash and `pipefail` is not an option it knows.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/spawn-ambiguity.txt"
WORK="$REPO/experiments/.sweep151-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

fail=0
skipped=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
command -v go >/dev/null 2>&1 || { echo "SKIP: go is not on PATH" >&2; exit 2; }
command -v unshare >/dev/null 2>&1 || { echo "SKIP: unshare is not on PATH" >&2; exit 2; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'go                %s\n' "$(go version)"
	printf 'setgroups here    %s\n' "$(cat /proc/self/setgroups 2>/dev/null || echo UNREADABLE)"
	printf 'setgroups unshared %s\n' "$(unshare -Ur cat /proc/self/setgroups 2>/dev/null || echo UNUSABLE)"
	echo
} >"$WORK/report"

# The payloads travel as files. A prose payload through a shell needs the
# file channel (docs/conventions/shell.md section 1).
cat >"$WORK/spawn-clone.go" <<'GO_EOF'
package main

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"
)

func main() {
	cmd := exec.Command("/bin/true")
	cmd.SysProcAttr = &syscall.SysProcAttr{Cloneflags: syscall.CLONE_NEWNS}
	if err := cmd.Run(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
GO_EOF
cat >"$WORK/spawn-cred.go" <<'GO_EOF'
package main

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"
)

func main() {
	cmd := exec.Command("/bin/true")
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Credential: &syscall.Credential{Uid: 0, Gid: 0, Groups: []uint32{0}},
	}
	if err := cmd.Run(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
GO_EOF

(cd "$WORK" && go mod init sweep151 >/dev/null 2>&1 && CGO_ENABLED=0 go build -o spawn-clone spawn-clone.go) || {
	echo "SKIP: the clone payload did not build" >&2
	exit 2
}
(cd "$WORK" && CGO_ENABLED=0 go build -o spawn-cred spawn-cred.go) || {
	echo "SKIP: the credential payload did not build" >&2
	exit 2
}

AMBIGUOUS="fork/exec /bin/true: operation not permitted"

# ------------------------------------------------------------------ A, clone
say "== A. the clone leg, in this context"
timeout 60 "$BIN" probe >"$WORK/probe-plain.out" 2>"$WORK/probe-plain.err" || true
if grep -q "clone(CLONE_NEWNS) here: denied" "$WORK/probe-plain.err"; then
	out_a="$(timeout 60 "$WORK/spawn-clone" 2>&1)"; rc_a=$?
	say "  spawn-clone rc=$rc_a out=[$out_a]"
	[ "$rc_a" -ne 0 ] || { say "  FAIL: the clone payload ran where clone is denied"; fail=1; }
	[ "$out_a" = "$AMBIGUOUS" ] || { say "  FAIL: not the ambiguous string"; fail=1; }
	if grep -q "meets the clone denial" "$WORK/probe-plain.err"; then
		say "  podbox attributes the string to the clone: yes"
	else
		say "  FAIL: podbox does not attribute the string to the clone"
		fail=1
	fi
else
	say "  SKIP: clone(CLONE_NEWNS) is not denied in this context"
	skipped=1
fi

# ------------------------------------------------------------- B, setgroups
say ""
say "== B. the setgroups leg, under unshare"
sg_u="$(unshare -Ur cat /proc/self/setgroups 2>/dev/null || echo UNUSABLE)"
say "  setgroups under unshare: $sg_u"
if [ "$sg_u" = "deny" ]; then
	out_b="$(timeout 60 unshare -Ur "$WORK/spawn-cred" 2>&1)"; rc_b=$?
	say "  spawn-cred rc=$rc_b out=[$out_b]"
	[ "$rc_b" -ne 0 ] || { say "  FAIL: the credential payload ran where setgroups is denied"; fail=1; }
	[ "$out_b" = "$AMBIGUOUS" ] || { say "  FAIL: not the ambiguous string"; fail=1; }
	if [ -n "${out_a:-}" ] && [ "${out_a:-}" != "$out_b" ]; then
		say "  FAIL: the two strings differ"
		fail=1
	else
		say "  one string behind both refusals: yes"
	fi
	timeout 120 unshare -Ur "$BIN" probe >"$WORK/probe-unshare.out" 2>"$WORK/probe-unshare.err" || true
	if grep -q "meets the setgroups denial" "$WORK/probe-unshare.err" && grep -q "Credential{NoSetGroups: true}" "$WORK/probe-unshare.err"; then
		say "  podbox attributes the string to setgroups and names the field: yes"
	else
		say "  FAIL: podbox does not attribute the string to setgroups"
		fail=1
	fi
else
	say "  SKIP: unshare does not deny setgroups here"
	skipped=1
fi

# --------------------------------------- C, control: the binary is well formed
say ""
say "== C. control: the credential payload where setgroups is allowed"
if [ "$(cat /proc/self/setgroups 2>/dev/null || echo UNREADABLE)" = "allow" ]; then
	timeout 60 "$WORK/spawn-cred" >/dev/null 2>&1; rc_c=$?
	say "  spawn-cred rc=$rc_c (0 proves the failure is the denial, not the binary)"
	[ "$rc_c" -eq 0 ] || { say "  FAIL: the credential payload fails where nothing denies it"; fail=1; }
else
	say "  SKIP: setgroups is not allowed in this context"
	skipped=1
fi

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
[ "$skipped" -eq 0 ] || exit 2
exit 0
