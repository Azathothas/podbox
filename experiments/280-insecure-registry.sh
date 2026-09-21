#!/usr/bin/env bash
# Question: can podbox reach a local registry that speaks plain HTTP, or HTTPS
# with a certificate nothing trusts, WITHOUT losing the refusal that
# TODO/image.md T-0201 exists for?
#
# TODO/image.md T-0213.
#
# ⭐ THE TWO HALVES ARE THE WHOLE POINT AND THEY PULL IN OPPOSITE DIRECTIONS.
# T-0201 refused a plain-HTTP FALLBACK because tcp/80 is black-holed on the
# runtimes podbox targets, so an automatic downgrade hangs instead of failing.
# That finding stands and clauses 1 and 4 assert it. What it never justified is
# refusing a registry the caller explicitly named, which is the ordinary case of
# a registry on loopback; clauses 2, 3 and 5 assert that one.
#
# ⛔ EVERY DOWNGRADE IS ANNOUNCED. Clause 6 reads the announcement back, because
# an agent cannot notice that its transport was downgraded the way a person can,
# and an unannounced downgrade is the exact class of dishonesty podbox exists to
# refuse.
#
#   ./280-insecure-registry.sh
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/insecure-registry.txt"
# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there. The lane's scratch
# lives under the checkout on a non-native lane: mount sources must be
# Windows paths there (245's rule), and /tmp/... names nothing.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep280-work"; rm -rf "$WORK"; mkdir -p "$WORK/w" || exit 2 ;;
esac

HTTP_NAME="podbox-x280-http-$$"
TLS_NAME="podbox-x280-tls-$$"
HTTP_PORT="${PODBOX_X280_HTTP_PORT:-5000}"
TLS_PORT="${PODBOX_X280_TLS_PORT:-5443}"

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. What each clause asserts is unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }

cleanup() {
	# ⛔ By name, not only by registration: ids eng_serve prints are
	# captured in $( ), and a subshell discards the assignment, so
	# eng_cleanup never learns them. A run that dies after serving would
	# otherwise leave its fixtures holding the ports. Measured
	# 2026-09-21: the rerun's start failed on the first run's still-Up
	# fixture.
	eng_rm "$HTTP_NAME" "$TLS_NAME"
	eng_cleanup
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

# The driver: the M5 debian row, pinned. It hosts the staged podbox binary
# for every podbox call on a non-native lane.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
# The fixture registry: ECR's mirror of the distribution image, pinned to the
# INDEX digest so every lane resolves the same manifest.
REGISTRY_IMG='public.ecr.aws/docker/library/registry@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373'
STAGE="$WORK/stage"
if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || {
		echo "SKIP: $BIN is not an executable. Build it:" >&2
		echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
		exit 2
	}
	BIN_RUN="$BIN"
	# The store and the driver-spelled config coincide where podbox runs.
	PSTORE="$WORK/store"
	CONF_INNER="$WORK/registries.conf"
else
	[ -r "$BIN" ] || {
		echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2
		exit 2
	}
	command -v zig >/dev/null 2>&1 || {
		echo "SKIP: zig is not on PATH; the driver-side forward is built from $HERE/lib/tcpfwd.c" >&2
		exit 2
	}
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" || exit 2
	cp "$BIN" "$STAGE/podbox"
	# ⭐ The forward beside podbox, built from a pinned input. The driver
	# cannot reach the published fixtures at `localhost`: its /etc/hosts
	# pins that name to 127.0.0.1 and no unprivileged call rewrites it, so
	# a static forward listens on driver loopback and dials the host's
	# published ports. `experiments/lib/tcpfwd.c` is the source; zig and
	# its version are printed in the conditions block.
	zig cc -target x86_64-linux-musl -static -O2 -s "$HERE/lib/tcpfwd.c" \
		-o "$STAGE/tcpfwd" 2>"$WORK/tcpfwd.err" || {
		echo "SKIP: the driver-side forward did not build" >&2
		sed 's/^/    /' "$WORK/tcpfwd.err" >&2
		exit 2
	}
	chmod +x "$STAGE/tcpfwd"
	# The generated wrapper, never edited: forwards up, then podbox with
	# the sweep's store and its driver-spelled inputs from $1, $2, $3.
	# Positional, because eng_run carries no `-e`: the store, the config
	# file and the insecure list cross as words, and the caller's
	# PODBOX_INSECURE_REGISTRIES reaches the wrapper through pb() below.
	cat >"$WORK/pb280.sh" <<PB_EOF
#!/bin/sh
# Generated by 280-insecure-registry.sh. Forwards driver loopback at the
# fixture ports to the published host ports, then podbox.
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
export PODBOX_STORE="\$1" PODBOX_CONFIG="\$2"
[ -n "\${3:-}" ] && export PODBOX_INSECURE_REGISTRIES="\$3"
shift 3
/drv/tcpfwd $HTTP_PORT host.containers.internal $HTTP_PORT &
/drv/tcpfwd $TLS_PORT host.containers.internal $TLS_PORT &
exec /pb "\$@"
PB_EOF
	chmod +x "$WORK/pb280.sh"
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$STAGE/tcpfwd" /drv/tcpfwd \
		&& eng_mount "$WORK/pb280.sh" /drv/pb280.sh \
		&& eng_mount "$WORK/w" /w rw \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
	eng_pb "$WORK/pb-shim" 120 "$DRIVER" || exit 2
	BIN_RUN="$WORK/pb-shim"
	# The store lives on the shared scratch (pulls only; no extract lands
	# here, so the Windows case-insensitivity 245 measured is not in play)
	# and the config beside it, both visible from the host side.
	PSTORE="$WORK/w/store"
	CONF_INNER="/w/registries.conf"
fi
export PODBOX_STORE="$PSTORE"
export PODBOX_CONFIG="$WORK/w/registries.conf"
[ "$NATIVE" -eq 1 ] && PODBOX_CONFIG="$WORK/registries.conf"

# pb TIMEOUT ARGS... - podbox with the sweep's store: directly where it
# executes, beside the forwards in the driver elsewhere. Timeouts stay per
# call site, as before. The store, the driver-spelled config and the
# insecure list cross into the driver as positional words to the generated
# wrapper (eng_run carries no `-e`); the config FILE is written host-side
# into the shared scratch. eng_pbrun cannot drive this script: it execs
# /pb with the words it is given, so handing it the wrapper would run the
# wrapper's path AS podbox arguments. Measured 2026-09-21: every clause
# exited with podbox refusing `/bin/sh` as a command.
pb() {
	_t="$1"; shift
	if [ "$NATIVE" -eq 1 ]; then
		timeout "$_t" "$BIN" "$@"
	else
		eng_run "$_t" "$DRIVER" "" -- /bin/sh /drv/pb280.sh \
			"$PSTORE" "$CONF_INNER" "${PODBOX_INSECURE_REGISTRIES:-}" "$@"
	fi
}

[ -x "$BIN_RUN" ] || {
	echo "SKIP: cannot read podbox's exit-code table; is jq installed and the binary built?" >&2
	exit 2
}
command -v openssl >/dev/null 2>&1 || { echo "SKIP: openssl is not on PATH" >&2; exit 2; }

# ⚠ The store and the policy come from this script and nothing else, so a
# caller's own configuration cannot decide what is measured.
# ⭐ The exit codes are DATA, read out of the binary rather than written here.
# TODO/cli.md T-0802 and scripts/common/exit-codes.sh: six clauses across four
# experiments had `[ "$rc" -eq 2 ]` in them and all went red at once the day
# docker's codes were measured.
. "$REPO/scripts/common/exit-codes.sh"
# Read out of the binary; on a lane where it does not execute, the
# generated shim runs that one call staged in the driver.
podbox_exit_codes "$BIN_RUN" || {
	echo "SKIP: cannot read podbox's exit-code table; is jq installed and the binary built?" >&2
	exit 2
}

unset PODBOX_INSECURE_REGISTRIES
: >"$PODBOX_CONFIG"

# ⭐ The proxy is deliberately LEFT SET where the environment sets one. Clause 3
# is only meaningful with it: the first build of this feature pulled a loopback
# registry through the environment's proxy and got HTTP 405, which is a proxy
# refusing a non-CONNECT request and reads as a broken registry.
PROXY_SET=no
[ -n "${HTTPS_PROXY:-}${https_proxy:-}" ] && PROXY_SET=yes
unset NO_PROXY no_proxy

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$(pb 60 version)"
	printf 'engine            %s %s\n' "$(engine_describe)" "(fixtures: $ENGINE_NAME)"
	printf 'proxy in env      %s (clause 3 is only meaningful when yes)\n' "$PROXY_SET"
	if [ "$NATIVE" -eq 0 ]; then
		printf 'forward           tcpfwd.c built with %s\n' "$(zig version 2>/dev/null || echo unknown)"
	fi
	echo
} >"$WORK/report"

fail=0
skipped=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }

# ------------------------------------------------------------ the fixtures
eng_rm "$HTTP_NAME" "$TLS_NAME"
HID="$(eng_serve "$HTTP_NAME" "$REGISTRY_IMG" "$HTTP_PORT:5000" "" --)" || {
	echo "SKIP: could not create a plain-HTTP registry on $HTTP_PORT" >&2
	exit 2
}
eng_start "$HID" >/dev/null 2>&1 || {
	echo "SKIP: could not start a plain-HTTP registry:2 on $HTTP_PORT" >&2
	exit 2
}
mkdir -p "$WORK/certs"
# ⛔ The config travels as a real file with the spelling openssl reads.
# Measured 2026-09-21 on the Windows lane, four failures in a row. First,
# the installed OpenSSL reads a system config whose v3_ca section names an
# option this build does not know, so `req -x509` dies and the TLS fixture
# starts with no certificate. Second, `OPENSSL_CONF=/dev/null` is no fix:
# a native Windows binary cannot open /dev/null. Third, an empty file
# under /tmp is no fix either: that /tmp is the shell's, and openssl
# never sees it. So the script writes an empty config beside the
# certificate and names it the way the engine wants paths (`winpath`
# from lib/engine.sh): every input here travels on the command line, so
# an empty config changes nothing but the broken lookup. Fourth, openssl
# itself is a native Windows binary: it reads neither the shell's /tmp
# nor its /c/... spellings, so the key and certificate paths travel in
# the engine's spelling too. And
# `-subj "/CN=..."` looks like a POSIX path, so MSYS rewrites it into
# `C:/Program Files/Git/...` unless conversion is off for this one call.
# Either failure leaves an empty certs directory, the registry exits 1 on
# `open /certs/domain.crt`, and clause 5 SKIPs on a missing fixture
# rather than on the product.
: >"$WORK/empty.cnf"
CERTS_WIN="$(winpath "$WORK/certs")"
MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' OPENSSL_CONF="$(winpath "$WORK/empty.cnf")" \
	openssl req -newkey rsa:2048 -nodes -keyout "$CERTS_WIN/domain.key" -x509 -days 2 \
	-out "$CERTS_WIN/domain.crt" -subj "/CN=localhost" \
	-addext "subjectAltName=DNS:localhost,IP:127.0.0.1" >/dev/null 2>&1 \
	|| { echo "SKIP: the TLS fixture's certificate could not be made" >&2; exit 2; }
[ -s "$WORK/certs/domain.crt" ] && [ -s "$WORK/certs/domain.key" ] || {
	echo "SKIP: the TLS fixture's certificate came out empty" >&2
	exit 2
}
if [ "$NATIVE" -eq 1 ]; then
	TID="$(eng_serve "$TLS_NAME" "$REGISTRY_IMG" "$TLS_PORT:443" \
		"REGISTRY_HTTP_ADDR=0.0.0.0:443 REGISTRY_HTTP_TLS_CERTIFICATE=/certs/domain.crt REGISTRY_HTTP_TLS_KEY=/certs/domain.key" \
		--)" || {
		echo "SKIP: could not create a TLS registry:2 on $TLS_PORT" >&2
		exit 2
	}
else
	eng_mount "$WORK/certs" /certs || {
		echo "SKIP: the TLS fixture's certificate could not be staged" >&2
		exit 2
	}
	TID="$(eng_serve "$TLS_NAME" "$REGISTRY_IMG" "$TLS_PORT:443" \
		"REGISTRY_HTTP_ADDR=0.0.0.0:443 REGISTRY_HTTP_TLS_CERTIFICATE=/certs/domain.crt REGISTRY_HTTP_TLS_KEY=/certs/domain.key" \
		--)" || {
		echo "SKIP: could not create a TLS registry:2 on $TLS_PORT" >&2
		exit 2
	}
	eng_clear
	# ⛔ The driver inputs are staged again: eng_clear above dropped them
	# with the /certs staging, and every pb() call below needs /pb, the
	# forward and the wrapper. Measured 2026-09-21: without this every
	# clause exits 127 with `exec: /pb: not found`.
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$STAGE/tcpfwd" /drv/tcpfwd \
		&& eng_mount "$WORK/pb280.sh" /drv/pb280.sh \
		&& eng_mount "$WORK/w" /w rw \
		|| { echo "SKIP: the driver inputs could not be re-staged" >&2; exit 2; }
fi
eng_start "$TID" >/dev/null 2>&1 || {
	echo "SKIP: could not start a TLS registry:2 on $TLS_PORT" >&2
	exit 2
}

# ⚠ A bounded wait, never an unbounded one: RULES.md section 8.
for _ in $(seq 1 30); do
	curl -sS --noproxy '*' -o /dev/null "http://localhost:$HTTP_PORT/v2/" 2>/dev/null && break
	sleep 1
done
for _ in $(seq 1 30); do
	curl -sSk --noproxy '*' -o /dev/null "https://localhost:$TLS_PORT/v2/" 2>/dev/null && break
	sleep 1
done

# Something to pull. ⚠ Any image the engine already holds; the question is the
# transport and not the payload.
SRC="${PODBOX_X280_SOURCE:-ghcr.io/pkgforge-dev/archlinux:latest}"
eng_pull --allow-tag "$SRC" >/dev/null 2>&1
eng_tag "$SRC" "localhost:$HTTP_PORT/x280:t" >/dev/null 2>&1
eng_push "localhost:$HTTP_PORT/x280:t" >/dev/null 2>&1 || {
	echo "SKIP: could not push a fixture image into the plain-HTTP registry" >&2
	exit 2
}
if [ "$ENGINE_NAME" = docker ]; then
	mkdir -p "/etc/docker/certs.d/localhost:$TLS_PORT"
	cp "$WORK/certs/domain.crt" "/etc/docker/certs.d/localhost:$TLS_PORT/ca.crt" 2>/dev/null
fi
eng_tag "$SRC" "localhost:$TLS_PORT/x280:t" >/dev/null 2>&1
eng_push "localhost:$TLS_PORT/x280:t" >/dev/null 2>&1
tls_pushed=$?

# --------------------------------------------------------------------- 1
say "== 1. the default refuses an explicit http:// and NAMES the flag"
# ⛔ T-0201 kept. A refusal a caller cannot act on is a refusal that costs a
# session, so the message has to carry the remedy.
out="$(pb 120 pull "http://localhost:$HTTP_PORT/x280:t" 2>&1)"
rc=$?
say "  exit              $rc (want $PODBOX_EXIT_CLI_ERROR, the cli-error code)"
say "  names the flag    $(printf '%s' "$out" | grep -c -- "--insecure-registry localhost:$HTTP_PORT")"
[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || { say "  FAIL: expected $PODBOX_EXIT_CLI_ERROR"; fail=1; }
printf '%s' "$out" | grep -q -- "--insecure-registry localhost:$HTTP_PORT" || {
	say "  FAIL: the refusal does not name the flag that would permit it"
	fail=1
}

# --------------------------------------------------------------------- 2
say ""
say "== 2. the default refuses a certificate nothing trusts"
rm -rf "$PODBOX_STORE"
out="$(pb 300 pull "localhost:$TLS_PORT/x280:t" 2>&1)"
rc=$?
say "  exit              $rc (125 is a runtime failure)"
say "  says              $(printf '%s' "$out" | tr -d '\n' | grep -oE 'invalid peer certificate[^)]*' | head -1)"
[ "$rc" -ne 0 ] || { say "  FAIL: an untrusted certificate was accepted"; fail=1; }

# --------------------------------------------------------------------- 3
say ""
say "== 3. --insecure-registry pulls the whole image over plain HTTP"
rm -rf "$PODBOX_STORE"
out="$(pb 900 pull --insecure-registry "localhost:$HTTP_PORT" \
	"localhost:$HTTP_PORT/x280:t" 2>&1)"
rc=$?
say "  exit              $rc"
say "  layers pulled     $(printf '%s' "$out" | grep -c 'Pull complete')"
if [ "$rc" -ne 0 ]; then
	say "  FAIL: $(printf '%s' "$out" | tail -1 | cut -c1-90)"
	[ "$PROXY_SET" = yes ] && say "        ⚠ HTTP 405 here is the PROXY refusing a non-CONNECT request."
	fail=1
fi

# --------------------------------------------------------------------- 4
say ""
say "== 4. and it changed NOTHING for any other registry"
# ⛔ The clause that keeps clause 3 from being a global downgrade. Naming one
# registry insecure must not make podbox speak HTTP to a second one.
rm -rf "$PODBOX_STORE"
out="$(pb 120 pull --insecure-registry "localhost:$HTTP_PORT" \
	"http://localhost:$TLS_PORT/x280:t" 2>&1)"
rc=$?
say "  a DIFFERENT registry over http://: exit $rc"
[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || {
	say "  FAIL: naming one registry insecure permitted plain HTTP to another"
	fail=1
}

# --------------------------------------------------------------------- 5
say ""
say "== 5. --tls-verify=false reaches the self-signed registry"
if [ "$tls_pushed" -ne 0 ]; then
	say "  SKIP: the fixture image is not in the TLS registry"
	skipped=1
else
	rm -rf "$PODBOX_STORE"
	out="$(pb 900 pull --tls-verify=false "localhost:$TLS_PORT/x280:t" 2>&1)"
	rc=$?
	say "  exit              $rc"
	say "  layers pulled     $(printf '%s' "$out" | grep -c 'Pull complete')"
	[ "$rc" -eq 0 ] || { say "  FAIL: $(printf '%s' "$out" | tail -1 | cut -c1-90)"; fail=1; }

	# ⛔ And it does NOT permit plain HTTP: not verifying a certificate and not
	# having one are different asks.
	rm -rf "$PODBOX_STORE"
	out="$(pb 120 pull --tls-verify=false "http://localhost:$HTTP_PORT/x280:t" 2>&1)"
	rc=$?
	say "  --tls-verify=false with an http:// reference: exit $rc"
	[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || {
		say "  FAIL: --tls-verify=false permitted plain HTTP, which is a different ask"
		fail=1
	}
fi

# --------------------------------------------------------------------- 6
say ""
say "== 6. every downgrade is announced on stderr"
rm -rf "$PODBOX_STORE"
err="$(pb 900 pull --insecure-registry "localhost:$HTTP_PORT" \
	"localhost:$HTTP_PORT/x280:t" 2>&1 >/dev/null)"
said_insecure=$(printf '%s' "$err" | grep -c 'configured as an insecure registry')
said_http=$(printf '%s' "$err" | grep -c 'use http://')
say "  announced insecure          $said_insecure"
say "  announced the http fallback $said_http"
[ "$said_insecure" -ge 1 ] || { say "  FAIL: the downgrade was silent"; fail=1; }

# ⛔ And a NORMAL pull says none of it. A disclosure printed on every run is a
# disclosure nobody reads.
rm -rf "$PODBOX_STORE"
err="$(pb 900 pull "$SRC" 2>&1 >/dev/null)"
noise=$(printf '%s' "$err" | grep -c 'insecure registry')
say "  a normal pull says it       $noise time(s)"
[ "$noise" -eq 0 ] || { say "  FAIL: an ordinary pull printed a downgrade notice"; fail=1; }

# --------------------------------------------------------------------- 7
say ""
say "== 7. the environment and the config file reach the same place"
rm -rf "$PODBOX_STORE"
# ⛔ The status is captured into a variable BEFORE anything else runs. Reading
# `$?` after a `say` reads `say`'s status, which is AGENTS.md absolute 8
# and cost this very script a wrong green on its first run.
PODBOX_INSECURE_REGISTRIES="localhost:$HTTP_PORT" \
	pb 900 pull "localhost:$HTTP_PORT/x280:t" >/dev/null 2>&1
env_rc=$?
say "  \$PODBOX_INSECURE_REGISTRIES exit $env_rc"
[ "$env_rc" -eq 0 ] || { say "  FAIL: the environment variable did not permit the registry"; fail=1; }
printf '# a comment\nlocalhost:%s\n' "$HTTP_PORT" >"$PODBOX_CONFIG"
rm -rf "$PODBOX_STORE"
pb 900 pull "localhost:$HTTP_PORT/x280:t" >/dev/null 2>&1
cfg_rc=$?
say "  \$PODBOX_CONFIG file         exit $cfg_rc"
[ "$cfg_rc" -eq 0 ] || { say "  FAIL: the config file did not permit the registry"; fail=1; }

# ⛔ A bad line in the config is named with its line number, never skipped.
printf 'localhost:%s\nhttps://oops/\n' "$HTTP_PORT" >"$PODBOX_CONFIG"
out="$(pb 120 pull "localhost:$HTTP_PORT/x280:t" 2>&1)"
rc=$?
say "  a URL where a host belongs: exit $rc, $(printf '%s' "$out" | tr -d '\n' | grep -oE 'registries.conf:[0-9]+' | head -1)"
[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || { say "  FAIL: a bad config line was not refused as invalid input"; fail=1; }
: >"$PODBOX_CONFIG"

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
[ "$skipped" -eq 0 ] || exit 2
exit 0
