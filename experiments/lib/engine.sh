#!/bin/sh
# engine.sh - one safe way for experiment scripts to reach a container engine.
#
# Sourced, never executed: `. "$HERE/lib/engine.sh"`. POSIX sh only, so both
# bash scripts and dash jobs can use it.
#
# Two lanes reach an engine from this tree. A Linux host or container talks to
# a docker daemon. A Windows host talks to its own podman machine, which runs
# rootful and honours `--cap-drop`, where the job container behind
# `scripts/windows/run-in-base.sh` holds no NET_ADMIN and cannot start dockerd
# (measured 2026-09-19). `engine_pick` prefers the daemon and falls back to
# host podman, and the conditions block of every caller names which one drove.
#
# The five things direct podman gets wrong on a workstation, and what this
# file does about each:
#
#   1. A privileged container escapes the experiment. Every entry point here
#      refuses `--privileged` and `--cap-add` outright.
#   2. A `-v` source writes to the operator's disk. `eng_mount` only binds
#      paths under the two roots the caller declares, and everything binds
#      read-only unless the source is caller-owned scratch with an explicit
#      `rw` third argument.
#   3. A created container outlives the run. `eng_create` registers every id
#      and `eng_cleanup` removes them; callers add it to their EXIT trap.
#      One-shot runs always pass `--rm` and never need the trap.
#   4. A moving tag measures a different thing each week. Images must carry
#      `@sha256:` or the call is refused.
#   5. A hang looks like work. Every engine call is bounded: runs take the
#      timeout as their first argument, and create, copy and pull carry
#      fixed bounds of their own.
#
# What this file deliberately does not do: start, stop or remove any machine,
# and never `podman machine` anything. The shared machine stays as found.
#
# Exit codes follow the tree convention: 0 ran, 1 the caller asked for
# something refused, 2 no engine answered.
set -u

# Set by engine_pick. ENGINE_BIN is `docker` or `podbox host podman` spelled
# as one command; on a Windows host that is the podman executable itself.
ENGINE_BIN=""
ENGINE_NAME=""
# Caller-owned roots for eng_mount sources. The caller sets both before the
# first eng_mount: ENGINE_REPO to the checkout, ENGINE_WORK to its scratch
# or artifact directory. Anything else as a mount source is refused.
ENGINE_REPO="${ENGINE_REPO:-}"
ENGINE_WORK="${ENGINE_WORK:-}"
# Accumulated `SRC:DST:MODE` bindings for the next eng_run. Cleared after it.
ENG_MOUNTS=""
# Container ids eng_create registered, space-separated, for eng_cleanup.
ENG_CIDS=""

# winpath P -> the spelling the engine wants. Git Bash hands a POSIX-looking
# /c/... path to a Windows binary that reads it as garbage, so on Windows
# lanes a leading single-letter root becomes `C:/...`. Elsewhere identity.
winpath() {
	case "$(uname -s)" in
	MINGW* | MSYS* | CYGWIN*)
		printf '%s' "$1" | sed 's,^/\([a-zA-Z]\)/,\1:/,'
		;;
	*)
		printf '%s' "$1"
		;;
	esac
}

# basepath P -> where the base sees a Windows-host path. The base mounts this
# machine's drives read-only under /mnt, so `C:/...` becomes `/mnt/c/...`.
# Prints nothing and returns 1 where the path has no drive spelling.
basepath() {
	_w="$(winpath "$1")"
	case "$_w" in
	[A-Za-z]:/*) printf '%s' "$_w" | sed 's,^\([A-Za-z]\):,/mnt/\L\1,;s,//,/,g' ;;
	*) return 1 ;;
	esac
}

# engine_pick - choose the engine and prove it answers. Prefers a docker
# daemon where one is reachable, because the runs on record used one, and
# falls back to host podman. Prints the choice; returns 2 where nothing
# answers. PODBOX_ENGINE=docker|podman pins the choice instead of probing.
#
# Each probe runs three times: a machine under load answers slowly rather
# than never, and one timed-out probe is a coincidence, not a verdict.
engine_pick() {
	case "${PODBOX_ENGINE:-auto}" in
	docker | podman | auto) ;;
	*) _refuse "PODBOX_ENGINE takes docker, podman or auto, not ${PODBOX_ENGINE:-}" || return 1 ;;
	esac
	command -v timeout >/dev/null 2>&1 || return 2
	if [ "${PODBOX_ENGINE:-auto}" = "docker" ] || [ "${PODBOX_ENGINE:-auto}" = "auto" ]; then
		if command -v docker >/dev/null 2>&1 && _answers docker info; then
			ENGINE_BIN="docker"
			ENGINE_NAME="docker"
			printf 'engine            docker (daemon)\n'
			return 0
		fi
		[ "${PODBOX_ENGINE:-auto}" = "docker" ] && return 2
	fi
	if command -v podman >/dev/null 2>&1 && _answers podman info; then
		ENGINE_BIN="podman"
		ENGINE_NAME="podman"
		printf 'engine            podman (host machine)\n'
		return 0
	fi
	return 2
}

# _answers CMD... - true where the probe answers at least once in up to
# three tries. Anything slower reads as absent.
_answers() {
	_n=0
	while [ "$_n" -lt 3 ]; do
		# shellcheck disable=SC2086
		if timeout 15 "$@" >/dev/null 2>&1; then
			return 0
		fi
		_n=$((_n + 1))
	done
	return 1
}

# engine_describe - the version line for a conditions block. Stdout alone.
engine_describe() {
	case "$ENGINE_NAME" in
	docker) timeout 15 docker version --format '{{.Server.Version}}' 2>/dev/null ;;
	podman) timeout 15 podman --version 2>/dev/null ;;
	*) return 2 ;;
	esac
}

# _refuse WHY... - the single exit for a refused call. Always 1: a refusal is
# a caller bug, never a missing engine.
_refuse() {
	printf 'engine: refused: %s\n' "$*" >&2
	return 1
}

# _pinned IMAGE - refuse anything that is not digest-pinned.
_pinned() {
	case "$1" in
	*@sha256:*) return 0 ;;
	*) _refuse "image is not digest-pinned: $1" ;;
	esac
}

# _no_priv ARGS... - refuse the flags that escape the experiment.
_no_priv() {
	for a in "$@"; do
		case "$a" in
		--privileged* | --cap-add*) _refuse "never $a in an experiment run" || return 1 ;;
		esac
	done
	return 0
}

# eng_mount SRC DST [rw] - stage one binding for the next eng_run, cleared
# by eng_clear afterwards. SRC must exist under ENGINE_REPO or ENGINE_WORK; DST must be absolute. Read-only
# unless the third argument is exactly `rw` and SRC is under ENGINE_WORK.
eng_mount() {
	[ $# -ge 2 ] || return 1
	_raw_src="$1"
	_dst="$2"
	_mode="ro"
	if [ "${3:-}" = "rw" ]; then
		[ -n "$ENGINE_WORK" ] || return 1
		case "$(winpath "$_raw_src")/" in
		"$(winpath "$ENGINE_WORK")/"*) _mode="rw" ;;
		*) _refuse "writable mount outside the work directory: $_raw_src" || return 1 ;;
		esac
	fi
	[ -e "$_raw_src" ] || _refuse "mount source does not exist: $_raw_src" || return 1
	case "$_dst" in
	/*) ;;
	*) _refuse "mount destination is not absolute: $_dst" || return 1 ;;
	esac
	_ok=""
	if [ -n "$ENGINE_REPO" ]; then
		case "$(winpath "$_raw_src")/" in
		"$(winpath "$ENGINE_REPO")/"*) _ok="1" ;;
		esac
	fi
	if [ -n "$ENGINE_WORK" ]; then
		case "$(winpath "$_raw_src")/" in
		"$(winpath "$ENGINE_WORK")/"*) _ok="1" ;;
		esac
	fi
	[ -n "$_ok" ] || _refuse "mount source outside the declared roots: $_raw_src" || return 1
	ENG_MOUNTS="$ENG_MOUNTS$(winpath "$_raw_src"):$_dst:$_mode "
}

# eng_run TIMEOUT IMAGE CAPS -- CMD... - one container, removed afterwards.
# CAPS is "" or `--cap-drop=...` words; anything else in it is refused.
# Mounts come from eng_mount and stay staged until the caller runs
# eng_clear: a run inside `$( )` cannot clear them itself, because a
# subshell discards the assignment, so clearing is the caller's job.
# The container exit code is the exit code.
#
# Word splitting below is deliberate: neither checkout roots nor scratch
# paths in this tree contain spaces. Paths with spaces are unsupported;
# keep them out.
eng_run() {
	[ $# -ge 5 ] || return 1
	_t="$1"
	_img="$2"
	_caps="$3"
	shift 3
	[ "$1" = "--" ] || return 1
	shift
	[ -n "$ENGINE_BIN" ] || return 1
	_pinned "$_img" || return 1
	_no_priv "$@" || return 1
	for _c in $_caps; do
		case "$_c" in
		--cap-drop=*) ;;
		"") ;;
		*) _refuse "capability flag is not a drop: $_c" || return 1 ;;
		esac
	done
	_flags=""
	# shellcheck disable=SC2086
	for _m1 in $ENG_MOUNTS; do
		_flags="$_flags -v $_m1"
	done
	# shellcheck disable=SC2086
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout "$_t" "$ENGINE_BIN" run --rm $_caps $_flags "$_img" "$@"
	_rc=$?
	return "$_rc"
}

# eng_clear - forget staged mounts. Call after the run, in the same shell
# that staged them: a `$( )` around the run would discard the clearing.
eng_clear() {
	ENG_MOUNTS=""
}

# eng_create IMAGE -- CMD... - a container that outlives one call. Prints the
# id on stdout and nothing else, and registers it for eng_cleanup.
eng_create() {
	[ $# -ge 3 ] || return 1
	_img="$1"
	shift
	[ "$1" = "--" ] || return 1
	shift
	[ -n "$ENGINE_BIN" ] || return 1
	_pinned "$_img" || return 1
	_no_priv "$@" || return 1
	_cid="$(MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 120 "$ENGINE_BIN" create "$_img" "$@")"
	[ -n "$_cid" ] || return 1
	ENG_CIDS="$ENG_CIDS$_cid "
	printf '%s\n' "$_cid"
}

# eng_cp CID SRC DST - copy one file out. There is no `-L` spelling: podman
# has none and the pinned sources this tree copies are regular files, so a
# flag asking for link-following would be a second behaviour only one engine
# honours. An argument starting with `-` is refused rather than passed on.
eng_cp() {
	[ $# -eq 3 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	for _a in "$@"; do
		case "$_a" in
		-*) _refuse "eng_cp takes no flags: $_a" || return 1 ;;
		esac
	done
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 120 "$ENGINE_BIN" cp "$1:$2" "$(winpath "$3")"
}

# eng_rm CID... - remove and unregister. Never fails the run: cleanup runs
# where the interesting failure already happened.
eng_rm() {
	for _c in "$@"; do
		MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 60 "$ENGINE_BIN" rm -f "$_c" >/dev/null 2>&1 || true
		_keep=""
		for _k in $ENG_CIDS; do
			[ "$_k" = "$_c" ] || _keep="$_keep$_k "
		done
		ENG_CIDS="$_keep"
	done
}

# eng_cleanup - remove everything eng_create registered. Callers using
# eng_create add this to their EXIT trap.
eng_cleanup() {
	if [ -n "$ENG_CIDS" ]; then
		# shellcheck disable=SC2086
		eng_rm $ENG_CIDS
	fi
}

# eng_pull IMAGE - fetch a pinned image before a create that does not pull.
eng_pull() {
	[ $# -eq 1 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	_pinned "$1" || return 1
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 300 "$ENGINE_BIN" pull "$1"
}
