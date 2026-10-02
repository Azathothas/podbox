#!/usr/bin/env sh
# build-lane.sh - refuse a Linux-target cargo build or test that was launched
# on a Windows host, where it cannot work.
#
# ⛔ WHY THIS EXISTS. `.cargo/config.toml` sets `[build] target =
# "x86_64-unknown-linux-musl"`, so a bare `cargo build` compiles a LINUX
# target on every host, including a Windows one. `docs/containers.md:33` says
# Linux builds and tests run in a job container in `wsl-toolkit-podbox`, and
# `scripts/dev-lane.sh` is the one lane runner. Running the build on the host
# instead produces `ring`'s build script invoking `scripts/zig-cc.sh` through
# a Windows process spawner: `os error 193`, "%1 is not a valid Win32
# application".
#
# ⛔ THE FAILURE MODE THIS EXISTS TO STOP IS NOT THE ERROR. That error is
# loud and harmless on its own. The damage is what an agent does with it:
# measured 2026-10-02, an agent reported the build as impossible on the host,
# confirmed the failure reproduced on a clean tree, and wrote "pre-existing,
# not introduced here" into three records. Every observation was true and the
# conclusion was worthless, because the build had been run in the wrong
# place. Through the lane it finishes in 55 s. A reproducible failure in the
# wrong place is still the wrong place, so the check fires on the PLACE, not
# on the error.
#
# ⚠ `cargo check` and `cargo clippy` are NOT refused. They never link, so on a
# Windows host they legitimately compile the Linux target and pass. The gate
# itself depends on that: `scripts/common/check-gate.sh` builds
# `x86_64-pc-windows-msvc` for its own use, and `check-gate.sh --fast` is a
# documented host check. Refusing check or clippy would make the tree
# unverifiable on the host to prevent a mistake on the host.
#
# What this does NOT do, and why that is a decision rather than a gap: it
# refuses the mistake. It cannot make the right command easy to find, and a
# reader still has to reach for `dev-lane.sh` on their own. A guard is the
# cheap half; the pointer in AGENTS.md is the other half and neither
# replaces the other.
#
# Exit: 0 the command is allowed here, 1 the command is refused,
# 2 the environment could not be judged.
#
# SPDX-License-Identifier: 0BSD

set -u

usage() {
	cat >&2 <<'USAGE'
usage: check-build-lane.sh -- <cargo command...>

Reads a cargo command line on stdin or after `--` and exits non-zero when a
command would build or test a Linux target on a Windows host.

  sh scripts/common/check-build-lane.sh -- cargo test --workspace
  cargo test --workspace | sh scripts/common/check-build-lane.sh -- cargo test --workspace
USAGE
}

case "${1:-}" in
-h | --help)
	usage
	exit 0
	;;
--self-test)
	# ⛔ THE PLANT, AND IT IS THE ONLY THING THAT KEEPS THIS CHECK HONEST.
	# A guard nobody has seen refuse is a guard nobody knows works, and one
	# that only ever ran on the Windows host it was written for is a guard
	# that could pass because its host test is wrong rather than because it
	# is right. That is not hypothetical: the first revision of this file
	# compared `"$uname_out" != "MINGW"*` inside `[ ]`, where the quoted word
	# leaves `*` literal, so the comparison never matched and EVERY Windows
	# host took the allow branch. It shipped that way and reported this
	# session's exact offending command as allowed.
	#
	# So the cases below are driven through a stubbed `uname`, on this host,
	# for BOTH hosts. Every refusal case is read for its exit code and no pipe
	# sits between the check and the reading. Exit 0 is not enough: a run
	# that prints nothing and refuses nothing is reported as a failure here,
	# because "quietly allowed" is the failure this check is most prone to.
	self_test() {
		failures=0
		stub=$(mktemp -d) || {
			printf 'check-build-lane: cannot make a temp dir\n' >&2
			exit 2
		}
		printf '#!/bin/sh\necho %s\n' "$1" >"$stub/uname"
		chmod +x "$stub/uname"
		run() {
			PATH="$stub:$PATH" "$0" -- "$2" >/dev/null 2>&1
			echo $?
		}
		want() {
			got=$1
			if [ "$got" = "$2" ]; then
				printf '  ok    %s (%s)\n' "$3" "$got"
			else
				printf '  FAIL  %s: exit %s, want %s\n' "$3" "$got" "$2" >&2
				failures=$((failures + 1))
			fi
		}
		printf 'build-lane self-test, host %s\n' "$1"
		want "$(run "$1" 'cargo build -p podbox-cli')" "$2" 'build without --target'
		want "$(run "$1" 'cargo test --workspace')" "$2" 'cargo test'
		want "$(run "$1" 'cargo build --release -p podbox-cli --target x86_64-unknown-linux-musl')" \
			"$2" 'build with a Linux --target'
		want "$(run "$1" 'cargo check -p podbox-windows')" 0 'cargo check'
		want "$(run "$1" 'cargo clippy -p podbox-windows')" 0 'cargo clippy'
		want "$(run "$1" 'cargo build --target x86_64-pc-windows-msvc -p podbox-gate')" 0 \
			'build with a Windows --target'
		want "$(run "$1" 'cargo fmt --all')" 0 'cargo fmt'
		want "$(run "$1" 'cargo tree')" 0 'cargo tree'
		rm -rf "$stub"
		return "$failures"
	}
	total=0
	# A Linux host allows everything, which is what keeps the CI workflows
	# valid: gate.yml:45 runs a bare `cargo build --release -p podbox-gate`.
	self_test Linux 0 || total=$((total + 1))
	# A Windows host refuses a Linux build and allows the rest.
	self_test MINGW64_NT-10.0-26200 1 || total=$((total + 1))
	if [ "$total" -ne 0 ]; then
		printf 'check-build-lane: self-test failed for %s host(s)\n' "$total" >&2
		exit 1
	fi
	printf 'build-lane ok: a Windows host refuses a Linux build, a Linux host allows it\n'
	exit 0
	;;
esac

# ---------------------------------------------------------------- the host --
# Uname decides, and `uname` is read from a file, never from a pipeline, so
# the exit code belongs to this command and not to the last stage of a pipe.
uname_out=$(uname -s 2>/dev/null)
uname_rc=$?
if [ "$uname_rc" -ne 0 ] || [ -z "$uname_out" ]; then
	printf 'check-build-lane: uname did not answer; the host is unknown and\n' >&2
	printf '  this check will not guess. Report the host by hand.\n' >&2
	exit 2
fi

# ⚠ `case`, not `[ "$x" != "MINGW"* ]`. A `case` pattern globs; the same
# pattern inside `[ ]` has its word quoted and its `*` is literal, so the
# comparison never matches and every Windows host silently took the
# not-a-Windows-host branch below. That defect shipped in the first
# revision of this check and was caught by the control it now documents: the
# guard reported the offending command as allowed, on the machine it was
# written for. `POSIXLY_CORRECT` changes `[` to match globs and masks it
# further, which is why `case` is used here and in `build-interpose.sh`.
case "$uname_out" in
MINGW* | MSYS* | CYGWIN* | Windows_NT)
	# A Windows host. Carry on.
	;;
*)
	# Not a Windows host. Every cargo command is allowed, which includes the
	# `cargo build --release -p podbox-gate` the workflows run on a Linux
	# runner. Exiting here is what keeps those workflows valid.
	exit 0
	;;
esac

# ------------------------------------------------------------- the command --
args=""
if [ "${1:-}" = "--" ]; then
	shift
fi
if [ "$#" -gt 0 ]; then
	args="$*"
fi
if [ -z "$args" ]; then
	args=$(cat)
fi

# A `--target` flag means the caller chose the target on purpose. On a
# Windows host a Linux `--target` is still refused below, because naming it is
# exactly what this session's author did and got wrong; a Windows `--target`
# is the one build the host should run, and it is let through.
printf '%s\n' "$args" | grep -q -- '--target' && explicit_target=1 || explicit_target=0
if ! printf '%s\n' "$args" | grep -qE '(^|[[:space:]/])(build|test|run|bench)([[:space:]]|$)'; then
	exit 0
fi

if [ "$explicit_target" -eq 1 ]; then
	# `--target <not a Linux triple>` is the one build a Windows host should
	# run: the gate itself is built as
	# `cargo build --target x86_64-pc-windows-msvc -p podbox-gate`, and
	# `scripts/common/check-gate.sh` depends on that working on this host.
	# Naming a Windows target on purpose is not the mistake this check
	# exists to stop, so it is allowed through here and refused only below
	# when the target actually is a Linux one.
	named=$(printf '%s\n' "$args" | awk '
		{ for (i = 1; i < NF; i++) if ($i == "--target") print $(i + 1) }
	')
	case "$named" in
	linux* | *-linux-*)
		printf 'check-build-lane: refused. This is a Windows host and\n' >&2
		printf '  --target %s names a Linux target outright.\n' "$named" >&2
		printf '  Linux builds and tests run in the base; see\n' >&2
		printf '  docs/containers.md, and sh scripts/dev-lane.sh run JOB.sh\n' >&2
		printf '  is the lane runner.\n' >&2
		exit 1
		;;
	esac
	# A Windows triple, or no `--target` value after the flag. Either way the
	# caller named a target and it is not a Linux one. Carry on.
	exit 0
fi

target=""
if [ -f .cargo/config.toml ]; then
	# The `[build] target` line, read from the file. A commented-out line is
	# not a setting and is skipped, which is why the match anchors on the
	# first character of the line.
	target=$(awk '
		/^\[/            { in_build = ($0 ~ /^\[build\][[:space:]]*$/) }
		in_build && /^[[:space:]]*target[[:space:]]*=/ {
			sub(/^[^=]*=[[:space:]]*/, "")
			gsub(/["[:space:]]/, "")
			print
			exit
		}
	' .cargo/config.toml 2>/dev/null)
fi

if [ -z "$target" ]; then
	# No `[build] target`. Cargo would use the host triple, which on Windows
	# is a Windows triple and does not need the lane. Nothing to refuse.
	exit 0
fi

case "$target" in
linux* | *-linux-* | *linux-*) ;;
*)
	# The pinned target is not a Linux target. It is a Windows host building
	# for Windows, which is the one correct case.
	exit 0
	;;
esac

printf 'check-build-lane: refused. This is a Windows host (%s) and\n' "$uname_out" >&2
printf '  .cargo/config.toml sets [build] target = %s, so this cargo\n' "$target" >&2
printf '  command would build a Linux target here.\n' >&2
printf '\n' >&2
if [ "$explicit_target" -eq 1 ]; then
	printf '  It also passed --target, so the Linux target was named outright.\n' >&2
fi
printf '  Linux builds and tests run in the base. See docs/containers.md.\n' >&2
printf '\n' >&2
printf '  sh scripts/dev-lane.sh run JOB.sh --artifacts DIR\n' >&2
printf '\n' >&2
printf '  A C dependency builds through scripts/zig-cc.sh, which a Windows\n' >&2
printf '  process spawner cannot execute: os error 193, "%%1 is not a valid\n' >&2
printf '  Win32 application". That error is NOT a host limitation. The same\n' >&2
printf '  build finished in 55 s through the lane on 2026-10-02.\n' >&2
printf '\n' >&2
printf '  cargo check and cargo clippy are allowed here on purpose: they never\n' >&2
printf '  link, so they compile the Linux target and pass on this host.\n' >&2
printf '\n' >&2
printf '  refused: %s\n' "$args" >&2
exit 1