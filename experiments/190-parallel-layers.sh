#!/usr/bin/env bash
# Question: does the bounded pool fetch a multi-layer image sooner than the
# sequential loop, and does a failure in one worker cancel the rest without
# leaving a partial store?
#
# TODO/image.md T-0207.
#
# The fixture is the T-0206 shape: one pinned zot binary serving a seeded
# multi-layer repo on container loopback, with ALL outbound network blocked,
# so the measurement spends nobody's quota. Two podbox binaries meet it
# there: the worktree's (bounded pool) and one built with pull.rs at HEAD
# (the sequential loop this entry replaces), so the wall times are two
# shapes of the same code rather than a model of one of them.
#
#   ./190-parallel-layers.sh
#   PODBOX_BIN=/path/to/podbox PODBOX_BIN_SEQ=/path/to/podbox-seq ./190-parallel-layers.sh
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/parallel-layers.txt"
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep190-work"; rm -rf "$WORK"; mkdir -p "$WORK/w" || exit 2 ;;
esac

# The pin, shared with 180-registry-fixture.sh: one release binary, verified
# every run against the release metadata and the release checksums line.
ZOT_VERSION="v2.1.21"
ZOT_BIN_URL="https://github.com/project-zot/zot/releases/download/v2.1.21/zot-linux-amd64-minimal"
ZOT_BIN_SHA256="d2422616a28dbae10a92c1df9daa23980e2f7f3deb0079968b928f23492196cb"
ZOT_BIN_BYTES="85459246"
ZOT_SUMS_URL="https://github.com/project-zot/zot/releases/download/v2.1.21/checksums.sha256.txt"
ZOT_PROXY="https://api.rv.pkgforge.dev/"

OPEN_PORT="5000"
REPO_NAME="multi"
REPO_TAG="layers"
LAYERS="8"
LAYER_BYTES="6291456"
POISON_INDEX="3"

# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }

cleanup() {
	eng_cleanup
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'engine            %s %s\n' "$(engine_describe)" "(fixtures: $ENGINE_NAME)"
	printf 'zot               %s %s bytes sha256:%s\n' "$ZOT_VERSION" "$ZOT_BIN_BYTES" "$ZOT_BIN_SHA256"
	printf 'image             %s:%s, %s layers of %s bytes (random, incompressible)\n' "$REPO_NAME" "$REPO_TAG" "$LAYERS" "$LAYER_BYTES"
	printf 'network           none (container loopback only; asserted in clause iso-1)\n'
	echo
} >"$WORK/report"

fail=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { say "  ok $1"; }
refuse() { say "  FAIL: $1"; fail=1; }

# ------------------------------------------------------- the podbox binaries
# Two shapes of the same code: the worktree's pool and HEAD's sequential
# loop. Both are lane-built; the lane copy carries the worktree change, and
# each job asserts which shape it holds before building.
lane_build() {
	_name="$1"; _shape="$2"; _dest="$3"
	cat >"$WORK/${_name}-job.sh" <<JOB_EOF
#!/bin/sh
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
case "$_shape" in
new)
  if git diff --quiet -- crates/podbox-image/src/pull.rs; then
    echo "190: lane copy carries no worktree change to pull.rs" >&2
    exit 2
  fi
  ;;
seq)
  git show HEAD:crates/podbox-image/src/pull.rs > crates/podbox-image/src/pull.rs || exit 2
  if git diff --quiet -- crates/podbox-image/src/pull.rs; then
    echo "190: HEAD shape staged"
  else
    echo "190: HEAD shape did not stage" >&2
    exit 2
  fi
  ;;
esac
cargo build --release --target x86_64-unknown-linux-musl --manifest-path "\$PWD/Cargo.toml" || exit 2
cp "/work/target/x86_64-unknown-linux-musl/release/podbox" /out/podbox || exit 2
echo "== podbox staged ($_shape)"
exit 0
JOB_EOF
	mkdir -p "$WORK/stage-$_name" || exit 2
	if PODBOX_ARTIFACTS="$WORK/stage-$_name" sh "$REPO/scripts/windows/run-in-base.sh" "$WORK/${_name}-job.sh" >"$WORK/${_name}.log" 2>&1; then
		cp "$WORK/stage-$_name/podbox" "$_dest" || exit 2
	else
		echo "SKIP: lane build of the $_shape binary failed" >&2
		tail -n 10 "$WORK/${_name}.log" >&2
		exit 2
	fi
}

BIN_NEW="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
BIN_SEQ="${PODBOX_BIN_SEQ:-}"
if [ "$NATIVE" -eq 1 ]; then [ -x "$BIN_NEW" ] || BIN_NEW=""; else [ -f "$BIN_NEW" ] || BIN_NEW=""; fi
if [ -z "$BIN_NEW" ]; then
	say "== podbox (pool): not staged, building it in the lane"
	lane_build pb-new new "$WORK/podbox-new" || exit 2
	BIN_NEW="$WORK/podbox-new"
fi
if [ -z "$BIN_SEQ" ]; then
	say "== podbox (sequential): building pull.rs at HEAD in the lane"
	lane_build pb-seq seq "$WORK/podbox-seq" || exit 2
	BIN_SEQ="$WORK/podbox-seq"
fi
if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN_NEW" ] || { echo "SKIP: pool binary is not executable: $BIN_NEW" >&2; exit 2; }
	[ -x "$BIN_SEQ" ] || { echo "SKIP: sequential binary is not executable: $BIN_SEQ" >&2; exit 2; }
else
	[ -f "$BIN_NEW" ] && [ "$(wc -c <"$BIN_NEW" | tr -d ' ')" != "0" ] || { echo "SKIP: pool binary missing: $BIN_NEW" >&2; exit 2; }
	[ -f "$BIN_SEQ" ] && [ "$(wc -c <"$BIN_SEQ" | tr -d ' ')" != "0" ] || { echo "SKIP: sequential binary missing: $BIN_SEQ" >&2; exit 2; }
fi
say "== podbox: pool ($(wc -c <"$BIN_NEW" | tr -d ' ') bytes) and sequential ($(wc -c <"$BIN_SEQ" | tr -d ' ') bytes) staged"

# ------------------------------------------------------- the zot binary
say "== zot pin"
mkdir -p "$WORK/zot" || exit 2
if [ ! -f "$WORK/zot/zot" ] || [ "$(wc -c <"$WORK/zot/zot" | tr -d ' ')" != "$ZOT_BIN_BYTES" ] || [ "$(sha256sum "$WORK/zot/zot" | cut -d' ' -f1)" != "$ZOT_BIN_SHA256" ]; then
	rm -f "$WORK/zot/zot"
	fetched=0
	for base in "" "$ZOT_PROXY"; do
		if curl -sS --max-time 600 -H 'User-Agent: curl/8.21.0' -o "$WORK/zot/zot" "${base}${ZOT_BIN_URL}" 2>/dev/null; then
			# curl without -f exits 0 on an HTTP error page, so the size
			# gates the break and the checksum below gates the run.
			if [ "$(wc -c <"$WORK/zot/zot" | tr -d ' ')" = "$ZOT_BIN_BYTES" ]; then
				fetched=1
				break
			fi
		fi
	done
	[ "$fetched" -eq 1 ] || { echo "SKIP: zot binary unreachable (direct and proxy)" >&2; exit 2; }
fi
[ "$(wc -c <"$WORK/zot/zot" | tr -d ' ')" = "$ZOT_BIN_BYTES" ] || { echo "SKIP: zot binary has the wrong size" >&2; exit 2; }
[ "$(sha256sum "$WORK/zot/zot" | cut -d' ' -f1)" = "$ZOT_BIN_SHA256" ] || { echo "SKIP: zot binary fails its checksum" >&2; exit 2; }
for base in "" "$ZOT_PROXY"; do
	if curl -sS --max-time 60 -H 'User-Agent: curl/8.21.0' -o "$WORK/zot/checksums.sha256.txt" "${base}${ZOT_SUMS_URL}" 2>/dev/null && [ -s "$WORK/zot/checksums.sha256.txt" ]; then
		break
	fi
done
grep -q "$ZOT_BIN_SHA256.*zot-linux-amd64-minimal" "$WORK/zot/checksums.sha256.txt" 2>/dev/null || { echo "SKIP: release checksums do not carry the pinned line" >&2; exit 2; }
pass "zot $ZOT_VERSION, $ZOT_BIN_BYTES bytes, sha256 matches the release line"

# ------------------------------------------------------- the seeded storage
# Eight random layers, one config, one manifest, the tag index and the layout
# file. The bytes are random so they do not compress: the measurement is
# transfer time, not gzip time. No network anywhere in the seeding.
say "== seed"
SEED_ROOT="$WORK/w/storage"
seed_repo() {
	_root="$1"; _repo="$2"; _tag="$3"; _n="$4"; _bytes="$5"; _order="$6"
	_d="$_root/$_repo"
	mkdir -p "$_d/blobs/sha256" || return 1
	: >"$_order" || return 1
	_layers=""; _diffs=""; _i=0
	while [ "$_i" -lt "$_n" ]; do
		head -c "$_bytes" /dev/urandom >"$_d/blobs/sha256/raw.$_i" 2>/dev/null || return 1
		_dhex="$(sha256sum "$_d/blobs/sha256/raw.$_i" | cut -d' ' -f1)" || return 1
		gzip -n -c "$_d/blobs/sha256/raw.$_i" >"$_d/blobs/sha256/layer.$_i.tmp" || return 1
		rm -f "$_d/blobs/sha256/raw.$_i" || return 1
		_lhex="$(sha256sum "$_d/blobs/sha256/layer.$_i.tmp" | cut -d' ' -f1)" || return 1
		_lsize="$(wc -c <"$_d/blobs/sha256/layer.$_i.tmp" | tr -d ' ')" || return 1
		mv "$_d/blobs/sha256/layer.$_i.tmp" "$_d/blobs/sha256/$_lhex" || return 1
		printf '%s\n' "$_lhex" >>"$_order" || return 1
		_layers="$_layers{\"mediaType\":\"application/vnd.oci.image.layer.v1.tar+gzip\",\"digest\":\"sha256:$_lhex\",\"size\":$_lsize},"
		_diffs="$_diffs\"sha256:$_dhex\","
		_i=$((_i + 1))
	done
	_layers="$(printf '%s' "$_layers" | sed 's/,$//')"
	_diffs="$(printf '%s' "$_diffs" | sed 's/,$//')"
	printf '{"architecture":"amd64","os":"linux","rootfs":{"type":"layers","diff_ids":[%s]}}' "$_diffs" >"$_d/blobs/sha256/config.tmp" || return 1
	_chex="$(sha256sum "$_d/blobs/sha256/config.tmp" | cut -d' ' -f1)" || return 1
	_csize="$(wc -c <"$_d/blobs/sha256/config.tmp" | tr -d ' ')" || return 1
	mv "$_d/blobs/sha256/config.tmp" "$_d/blobs/sha256/$_chex" || return 1
	printf '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:%s","size":%s},"layers":[%s]}' "$_chex" "$_csize" "$_layers" >"$_d/blobs/sha256/manifest.tmp" || return 1
	_mhex="$(sha256sum "$_d/blobs/sha256/manifest.tmp" | cut -d' ' -f1)" || return 1
	_msize="$(wc -c <"$_d/blobs/sha256/manifest.tmp" | tr -d ' ')" || return 1
	mv "$_d/blobs/sha256/manifest.tmp" "$_d/blobs/sha256/$_mhex" || return 1
	printf '{"schemaVersion":2,"manifests":[{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:%s","size":%s,"annotations":{"org.opencontainers.image.ref.name":"%s"}}]}' "$_mhex" "$_msize" "$_tag" >"$_d/index.json" || return 1
	printf '{"imageLayoutVersion":"1.0.0"}' >"$_d/oci-layout" || return 1
	printf '%s' "$_mhex"
	return 0
}
SEED_MANIFEST="$(seed_repo "$SEED_ROOT" "$REPO_NAME" "$REPO_TAG" "$LAYERS" "$LAYER_BYTES" "$WORK/w/layer-order.txt")" || { echo "SKIP: seed failed" >&2; exit 2; }
# Manifest order of the layer shorts, and the poisoned layer's full digest,
# for the driver's order and failure clauses. The order file carries one full
# hex per line in the order the manifest names them; glob order is not it.
POISON_HEX="$(sed -n "$((POISON_INDEX + 1))p" "$WORK/w/layer-order.txt" | cut -c1-12)"
[ -n "$POISON_HEX" ] || { echo "SKIP: poisoned layer not found in the seed" >&2; exit 2; }
POISON_FULL=""
for b in "$SEED_ROOT/$REPO_NAME/blobs/sha256/"*; do
	n=$(basename "$b")
	case "$n" in "$POISON_HEX"*) POISON_FULL="$n"; break ;; esac
done
[ -n "$POISON_FULL" ] || { echo "SKIP: poisoned layer not found in the seed" >&2; exit 2; }
STORED_BYTES="$(du -cb "$SEED_ROOT/$REPO_NAME/blobs/sha256/" | tail -n 1 | cut -f1)"
pass "seeded $REPO_NAME:$REPO_TAG sha256:$SEED_MANIFEST, $LAYERS layers, $STORED_BYTES stored bytes"

# ------------------------------------------------------- config and cert
say "== config and cert"
cat >"$WORK/w/config-open.json" <<'EOF'
{
  "distSpecVersion": "1.1.1",
  "storage": { "rootDirectory": "/w/storage" },
  "http": {
    "address": "127.0.0.1",
    "port": "5000",
    "realm": "zot",
    "tls": { "cert": "/w/server.cert", "key": "/w/server.key" }
  },
  "log": { "level": "error" }
}
EOF
: >"$WORK/w/empty.cnf"
_WINWORK="$(winpath "$WORK/w")"
_OLD_NO_PATHCONV="${MSYS_NO_PATHCONV:-}"
_OLD_ARG_CONV_EXCL="${MSYS2_ARG_CONV_EXCL:-}"
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
OPENSSL_CONF="$_WINWORK/empty.cnf" openssl req -x509 -newkey rsa:2048 -keyout "$_WINWORK/server.key" -out "$_WINWORK/server.cert" -days 2 -nodes -subj "/CN=127.0.0.1" -addext "subjectAltName=IP:127.0.0.1" >"$WORK/openssl.log" 2>&1 || { echo "SKIP: certificate failed" >&2; tail -n 5 "$WORK/openssl.log" >&2; exit 2; }
export MSYS_NO_PATHCONV="$_OLD_NO_PATHCONV" MSYS2_ARG_CONV_EXCL="$_OLD_ARG_CONV_EXCL"
[ -s "$WORK/w/server.cert" ] && [ -s "$WORK/w/server.key" ] || { echo "SKIP: certificate came out empty" >&2; exit 2; }
pass "config written, cert carries SAN IP:127.0.0.1"

# ------------------------------------------------------- the driver image
say "== driver image"
mkdir -p "$WORK/ctx" || exit 2
_BUILD_OUT="$(eng_build 600 podbox-x190-driver "$REPO/experiments/180-driver.Dockerfile" "$WORK/ctx" --)" || { echo "SKIP: driver image did not build" >&2; exit 2; }
DRIVER_ID="$(printf '%s' "$_BUILD_OUT" | tail -n 1)"
pass "driver $DRIVER_ID"

# ------------------------------------------------------- the driver script
# Literals baked in (digests, ports, repo, order); nothing secret travels.
say "== driver run (network none)"
# One short digest per line, in manifest order.
cut -c1-12 "$WORK/w/layer-order.txt" >"$WORK/w/order.txt" || exit 2
cat >"$WORK/w/driver.sh" <<DRIVER_EOF
#!/bin/sh
# 190 inside the none-network container: zot and two podbox shapes on loopback.
set -u
SEED="$SEED_MANIFEST"
POISON="$POISON_FULL"
PORT_OPEN="$OPEN_PORT"
REPO="$REPO_NAME"
TAG="$REPO_TAG"
fails=0
ok() { printf '  ok %s\n' "\$1"; }
bad() { printf '  FAIL: %s\n' "\$1"; fails=\$((fails + 1)); }
chmod +x /zb /pb /pb-seq 2>/dev/null || true
echo "-- staged binaries"
printf '  pool: %s\n' "\$(/pb version 2>/dev/null || echo MISSING)"
printf '  sequential: %s\n' "\$(/pb-seq version 2>/dev/null || echo MISSING)"
/zb --help >/dev/null 2>&1 || { bad "staged zot does not execute"; }

echo "-- isolation (asserted, not assumed)"
if grep -q '^[^I ][^ ]*  *00000000' /proc/net/route 2>/dev/null; then
	bad "iso-1: a default route exists: \$(cat /proc/net/route)"
else
	ok "iso-1: no default route (header only or loopback destinations)"
fi
if getent hosts index.docker.io >/dev/null 2>&1; then
	bad "iso-2: the WAN resolves from inside"
else
	ok "iso-2: no name resolves from inside"
fi

echo "-- zot on 127.0.0.1:\$PORT_OPEN"
/zb verify /w/config-open.json >/dev/null 2>&1 || { bad "cfg-1: open config rejected"; }
ok "cfg-1: open config verifies"
serve_zot() {
	/zb serve /w/config-open.json >/w/zot.log 2>&1 &
	ZOT_PID=\$!
	poll=0
	while [ "\$poll" -lt 20 ]; do
		if curl -sS --max-time 5 --cacert /w/server.cert -o /dev/null "https://127.0.0.1:\$PORT_OPEN/v2/" 2>/dev/null; then break; fi
		poll=\$((poll + 1)); sleep 1
	done
}
stop_zot() {
	kill "\$ZOT_PID" 2>/dev/null || true
	wait "\$ZOT_PID" 2>/dev/null || true
	rm -f /w/storage/cache.db || true
}
serve_zot
if curl -sS --max-time 15 --cacert /w/server.cert -H "Accept: application/vnd.oci.image.manifest.v1+json" "https://127.0.0.1:\$PORT_OPEN/v2/\$REPO/manifests/\$TAG" -o /w/got-tag.json 2>/dev/null && [ "\$(sha256sum /w/got-tag.json | cut -d' ' -f1)" = "\$SEED" ]; then
	ok "seed-1: manifest by tag is the seeded bytes"
else
	bad "seed-1: manifest by tag failed or differs from the seed"
fi

# The order check, shared by the timing and failure legs: every
# 'Pull complete' and 'Already exists' line of a pull log names its layer in
# manifest order. /w/order.txt carries that order, one short digest a line.
ordered() {
	awk -v order=/w/order.txt '
	BEGIN {
		n = 0
		while ((getline line < order) > 0) { want[++n] = line }
		last = 0; seen = 0
	}
	/: (Pull complete|Already exists)/ {
		short = \$1; sub(/:$/, "", short); seen++
		found = 0
		for (i = 1; i <= n; i++) {
			if (want[i] == short) { found = i; break }
		}
		if (found == 0 || found <= last) { print "out of order: " \$0; exit 1 }
		last = found
	}
	END { if (seen == 0) { print "no transcript lines"; exit 1 } }
	' "\$1"
}

echo "-- timing: sequential shape, cold store each leg"
REF="127.0.0.1:\$PORT_OPEN/\$REPO:\$TAG"
for run in 1 2; do
	rm -rf /w/store-seq && mkdir -p /w/store-seq
	t0=\$(date +%s%N)
	SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-seq /pb-seq pull "\$REF" >/w/pull-seq-\$run.log 2>&1
	rc=\$?
	t1=\$(date +%s%N)
	printf '  time-seq-%s %s\n' "\$run" "\$((t1 - t0))"
	if [ "\$rc" -ne 0 ]; then
		bad "time-seq-\$run: sequential pull exits \$rc, not 0"
	else
		GOT="\$(SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-seq /pb-seq inspect --format '{{.Digest}}' "\$REF" 2>/dev/null)"
		if [ "\$GOT" = "sha256:\$SEED" ]; then
			ok "time-seq-\$run: exits 0 and inspects to the seeded digest"
		else
			bad "time-seq-\$run: digest \$GOT is not sha256:\$SEED"
		fi
	fi
done

echo "-- timing: pooled shape, cold store each leg"
for run in 1 2; do
	rm -rf /w/store-new && mkdir -p /w/store-new
	t0=\$(date +%s%N)
	SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-new /pb pull "\$REF" >/w/pull-new-\$run.log 2>&1
	rc=\$?
	t1=\$(date +%s%N)
	printf '  time-new-%s %s\n' "\$run" "\$((t1 - t0))"
	if [ "\$rc" -ne 0 ]; then
		bad "time-new-\$run: pooled pull exits \$rc, not 0"
	else
		GOT="\$(SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-new /pb inspect --format '{{.Digest}}' "\$REF" 2>/dev/null)"
		if [ "\$GOT" = "sha256:\$SEED" ]; then
			ok "time-new-\$run: exits 0 and inspects to the seeded digest"
		else
			bad "time-new-\$run: digest \$GOT is not sha256:\$SEED"
		fi
	fi
done

echo "-- order: pooled transcript against manifest order"
if ordered /w/pull-new-1.log && ordered /w/pull-new-2.log; then
	ok "order-1: both pooled transcripts list layers in manifest order"
else
	bad "order-1: a pooled transcript left manifest order"
fi

echo "-- failure injection: one bad blob cancels the rest"
stop_zot
PRE="\$(sha256sum "/w/storage/\$REPO/blobs/sha256/\$POISON" | cut -d' ' -f1)"
printf 'X' | dd of="/w/storage/\$REPO/blobs/sha256/\$POISON" bs=1 seek=100 count=1 conv=notrunc 2>/dev/null
if [ "\$(sha256sum "/w/storage/\$REPO/blobs/sha256/\$POISON" | cut -d' ' -f1)" != "\$PRE" ]; then
	ok "fail-0: the poisoned blob no longer matches its digest"
else
	bad "fail-0: the poison did not land (same sha before and after)"
fi
serve_zot
rm -rf /w/store-fail && mkdir -p /w/store-fail
SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-fail /pb pull "\$REF" >/w/pull-fail.log 2>&1
rc=\$?
printf '  fail-rc %s\n' "\$rc"
if [ "\$rc" -eq 0 ]; then
	bad "fail-1: pull of a poisoned image exits 0"
else
	ok "fail-1: pull of a poisoned image exits \$rc"
fi
if grep -q 'digest mismatch' /w/pull-fail.log 2>/dev/null; then
	ok "fail-2: the refusal names the digest mismatch"
else
	bad "fail-2: the refusal does not name the digest mismatch"
fi
LEFT="\$(find /w/store-fail/staging -name '*.partial' 2>/dev/null | wc -l | tr -d ' ')"
if [ "\$LEFT" = "0" ]; then
	ok "fail-3: zero staged files left behind"
else
	bad "fail-3: \$LEFT staged file(s) left behind"
fi
if ordered /w/pull-fail.log; then
	ok "fail-4: the failure transcript stays in manifest order"
else
	bad "fail-4: the failure transcript left manifest order"
fi
if SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/store-fail /pb inspect "\$REF" >/dev/null 2>&1; then
	bad "fail-5: the failed pull left a record behind"
else
	ok "fail-5: the failed pull recorded nothing"
fi
stop_zot

if [ "\$fails" -eq 0 ]; then echo "driver: all clauses held"; else echo "driver: \$fails clause(s) failed"; fi
exit "\$fails"
DRIVER_EOF
eng_mount "$WORK/zot/zot" /zb
eng_mount "$BIN_NEW" /pb
eng_mount "$BIN_SEQ" /pb-seq
eng_mount "$WORK/w" /w rw
# shellcheck disable=SC2034 # read by experiments/lib/engine.sh's eng_run
ENG_NETWORK=none
if eng_run 900 "$DRIVER_ID" "" -- /bin/sh /w/driver.sh >>"$WORK/report" 2>&1; then
	pass "driver run exits 0 with network none"
else
	refuse "driver run failed (network none)"
fi
eng_clear

# ------------------------------------------------------- the verdict
say "== verdict"
# ⛔ The driver must prove it ran: an empty driver script exits 0 having done
# nothing, and a verdict that only looks for FAIL lines reads that as green.
if grep -q '^driver: ' "$WORK/report"; then
	pass "driver ran to its verdict line"
else
	refuse "driver never reached its verdict line"
fi
if grep -q '^  FAIL' "$WORK/report"; then
	refuse "report carries FAIL lines"
fi
# The two wall-time shapes, restated beside the verdict they support.
say "== wall times (nanoseconds, cold store each leg)"
grep -h '^  time-' "$WORK/report" || refuse "no wall-time lines recorded"

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
exit 0
