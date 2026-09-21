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
# Accumulated `SRC:DST:MODE` bindings for the next eng_run, where SRC is a
# host path staged by eng_mount or a volume name staged by eng_volmount.
# Cleared after it.
ENG_MOUNTS=""
# Container ids eng_create registered, space-separated, for eng_cleanup.
ENG_CIDS=""
# Named volumes eng_volmount registered, space-separated, for eng_cleanup.
ENG_VOLS=""

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

# eng_volmount NAME DST [rw] - stage a NAMED VOLUME binding for the next
# eng_run, and register NAME for eng_cleanup. For engine-local persistent
# state: a podbox store that must survive `--rm` one-shots on a filesystem
# the host cannot share (a Windows-backed bind is case-insensitive, and an
# archlinux extract does not survive one). NAME is a volume name, never a
# path: it names nothing on the operator's disk, so unlike eng_mount it
# needs no declared root, and a second run with the same NAME reuses the
# volume rather than starting clean. DST must be absolute; read-only unless
# the third argument is exactly `rw`.
eng_volmount() {
	[ $# -ge 2 ] || return 1
	_vn="$1"
	_vd="$2"
	_vm="ro"
	case "$_vn" in
	"" | .* | -* | *[!A-Za-z0-9_.-]*) _refuse "volume name is not a name: $_vn" || return 1 ;;
	esac
	case "$_vd" in
	/*) ;;
	*) _refuse "mount destination is not absolute: $_vd" || return 1 ;;
	esac
	if [ "${3:-}" = "rw" ]; then _vm="rw"; fi
	ENG_MOUNTS="$ENG_MOUNTS$_vn:$_vd:$_vm "
	case " $ENG_VOLS " in
	*" $_vn "*) ;;
	*) ENG_VOLS="$ENG_VOLS$_vn " ;;
	esac
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

# eng_serve NAME IMAGE PORTS ENV -- CMD... - a fixture server that outlives
# one call. Prints the id on stdout and nothing else, and registers it for
# eng_cleanup. NAME is a fixed fixture name (`podbox-x280-http-<pid>`), so a
# re-run can pre-clean a previous run's fixture by name. PORTS is
# `HPORT:CPORT` words published as given (fixtures serve the test alone, on
# ports the caller chose). ENV is `K=V` words passed through `-e`; values
# with spaces are unsupported, keep them out. Mounts come from eng_mount as
# with eng_run. CMD may be empty where the image's own command serves.
eng_serve() {
	[ $# -ge 5 ] || return 1
	_n="$1"; _img="$2"; _ports="$3"; _env="$4"
	shift 4
	[ "$1" = "--" ] || return 1
	shift
	[ -n "$ENGINE_BIN" ] || return 1
	_pinned "$_img" || return 1
	_no_priv "$@" || return 1
	case "$_n" in
	"" | .* | -* | *[!A-Za-z0-9_.-]*) _refuse "fixture name is not a name: $_n" || return 1 ;;
	esac
	_flags=""
	# shellcheck disable=SC2086
	for _p in $_ports; do
		case "$_p" in
		[0-9]*:[0-9]*) ;;
		*) _refuse "port mapping is not HPORT:CPORT: $_p" || return 1 ;;
		esac
		_flags="$_flags -p $_p"
	done
	# shellcheck disable=SC2086
	for _e in $_env; do
		case "$_e" in
		*=*) ;;
		*) _refuse "env is not K=V: $_e" || return 1 ;;
		esac
		_flags="$_flags -e $_e"
	done
	# shellcheck disable=SC2086
	for _m1 in $ENG_MOUNTS; do
		_flags="$_flags -v $_m1"
	done
	# shellcheck disable=SC2086
	_cid="$(MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 120 "$ENGINE_BIN" create --name "$_n" $_flags "$_img" "$@")"
	[ -n "$_cid" ] || return 1
	ENG_CIDS="$ENG_CIDS$_cid "
	printf '%s\n' "$_cid"
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

# eng_cleanup - remove everything eng_create registered, then every volume
# eng_volmount registered. Callers using either add this to their EXIT trap.
eng_cleanup() {
	if [ -n "$ENG_CIDS" ]; then
		# shellcheck disable=SC2086
		eng_rm $ENG_CIDS
	fi
	if [ -n "$ENG_VOLS" ] && [ -n "$ENGINE_BIN" ]; then
		# shellcheck disable=SC2086
		for _v in $ENG_VOLS; do
			MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 60 "$ENGINE_BIN" volume rm "$_v" >/dev/null 2>&1 || true
		done
		ENG_VOLS=""
	fi
}

# eng_pbrun TIMEOUT IMAGE STORE [VAR=VAL ...] -- ARGS... - podbox staged in
# the driver with STORE as its PODBOX_STORE. The caller staged /pb (and any
# mounts) first; STORE is a /w path on lanes that share the store with the
# host and a container-local path where nothing host-side reads it. Further
# variables (a driver-spelled config path, an insecure-registry list) travel
# the same way: VAR=VAL words before the `--`, refused where a value carries
# a quote or a space. The helper carries no `-e`, so the store travels in
# the argv wrapper, whose text crosses shells intact on both lanes.
# Timeouts stay per call site.
eng_pbrun() {
	[ $# -ge 4 ] || return 1
	_t="$1"; _img="$2"; _store="$3"
	shift 3
	_pfx=""
	while [ $# -gt 0 ] && [ "$1" != "--" ]; do
		case "$1" in
		*=*)
			_v="${1%%=*}"; _val="${1#*=}"
			case "$_v" in "" | [0-9]* | *[!A-Za-z0-9_]*) _refuse "env name is not a name: $_v" || return 1 ;; esac
			case "$_val" in *"'"* | *" "*) _refuse "env value crosses no shell intact: $_v" || return 1 ;; esac
			_pfx="$_pfx$_v=$_val " ;;
		*) _refuse "not VAR=VAL and not --: $1" || return 1 ;;
		esac
		shift
	done
	[ "${1:-}" = "--" ] || return 1
	shift
	_script="$_pfx"PODBOX_STORE="$_store"' exec /pb "$@"'
	eng_run "$_t" "$_img" "" -- /bin/sh -c "$_script" _ "$@"
}

# eng_pull [--allow-tag] IMAGE - fetch before a create that does not pull.
# Tags are refused by default: a moving tag measures a different thing
# each week. --allow-tag is the explicit escape for the one case where
# the tag IS the subject and the script handles the race itself (150's
# digest parity, with its re-pull protocol and printed resolved digests).
# An allowlist flag, greppable at the call site, never a quiet default.
eng_pull() {
	_tagok=0
	if [ "${1:-}" = "--allow-tag" ]; then _tagok=1; shift; fi
	[ $# -eq 1 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	if [ "$_tagok" -eq 0 ]; then _pinned "$1" || return 1; fi
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 300 "$ENGINE_BIN" pull "$1"
}

# eng_tag SRC DST - point a second name at an image, for fixture setup.
# Both sides are engine references; pinning is the caller's business here
# because a fixture tag is a local name by definition.
eng_tag() {
	[ $# -eq 2 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 120 "$ENGINE_BIN" tag "$1" "$2"
}

# eng_push REF - push to a fixture registry. The experiments that push
# only ever push their own loopback fixtures, whose certificate nothing
# trusts; on a podman lane the daemon cert store is unreachable from
# here, so verification is skipped for exactly that case, loudly in the
# caller's transcript. A docker lane pushes plainly and trusts through
# its own certs.d, as before.
eng_push() {
	[ $# -eq 1 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	case "$ENGINE_BIN" in
	docker) MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 300 "$ENGINE_BIN" push "$1" ;;
	podman) MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 300 "$ENGINE_BIN" push --tls-verify=false "$1" ;;
	*) _refuse "no engine picked" || return 1 ;;
	esac
}

# eng_image_inspect REF FORMAT - one engine-side record, for parity checks
# against podbox's own. --format is spelled the same on both engines.
eng_image_inspect() {
	[ $# -eq 2 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 60 "$ENGINE_BIN" image inspect --format "$2" "$1"
}

# eng_start CID - start a created container and return once it is running.
# For fixtures that must serve across calls (a registry); one-shot runs
# use eng_run. The id stays registered for eng_cleanup.
eng_start() {
	[ $# -eq 1 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' timeout 60 "$ENGINE_BIN" start "$1"
}

# eng_pb SHIM TIMEOUT IMAGE - write an executable that forwards to the
# staged /pb in the driver, for readers that need a PATH rather than a
# function: `podbox_exit_codes` runs `$BIN system info`, and multicall
# clauses exec names. The shim carries no mounts (readers needing files
# use eng_run directly) and forwards the exit code; timeout bounds it as
# eng_run would. Generated, never edited: the five lines below are the
# only second spelling of the run line, and they are written here once.
eng_pb() {
	[ $# -eq 3 ] || return 1
	[ -n "$ENGINE_BIN" ] || return 1
	[ -n "$ENGINE_WORK" ] || return 1
	_pb="$ENGINE_WORK/stage/podbox"
	[ -f "$_pb" ] || _refuse "no staged podbox at $_pb" || return 1
	cat >"$1" <<EOF_PB
#!/bin/sh
# Generated by eng_pb. Forwards to the staged podbox; takes no mounts.
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
exec timeout $2 "$ENGINE_BIN" run --rm -v "$(winpath "$_pb"):/pb:ro" "$3" /pb "\$@"
EOF_PB
	chmod +x "$1"
}
