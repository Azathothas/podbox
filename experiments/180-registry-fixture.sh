#!/usr/bin/env bash
# Question: can podbox pull the four registry endpoints from a loopback
# fixture with ALL outbound network blocked, so the acceptance stops
# depending on somebody else's quota?
#
# TODO/image.md T-0206.
#
# The fixture is one pinned zot binary plus a config, a seeded storage dir,
# a generated cert and an htpasswd file, all staged host-side; the Prove
# runs inside one `--network=none` driver container where zot and podbox
# meet on container loopback. Two zot configs: open (the podbox pull
# clauses) and htpasswd-required (raw-HTTP 401/200 clauses, the shape
# TODO/image.md T-0209 later drives with credentials; podbox itself is
# anonymous-only until then, so no podbox clause touches the required
# config).
#
#   ./180-registry-fixture.sh
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/registry-fixture.txt"
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep180-work"; rm -rf "$WORK"; mkdir -p "$WORK/w" || exit 2 ;;
esac

# ⭐ THE PIN. One release binary, verified every run: the size against the
# release metadata and the bytes against the release checksums before
# anything executes them.
ZOT_VERSION="v2.1.21"
ZOT_BIN_URL="https://github.com/project-zot/zot/releases/download/v2.1.21/zot-linux-amd64-minimal"
ZOT_BIN_SHA256="d2422616a28dbae10a92c1df9daa23980e2f7f3deb0079968b928f23492196cb"
ZOT_BIN_BYTES="85459246"
ZOT_SUMS_URL="https://github.com/project-zot/zot/releases/download/v2.1.21/checksums.sha256.txt"
# Direct first, then the proxy this lane needs for github.com payloads
# (measured 2026-09-22: api.github.com answers directly, the download does
# not, and the proxy serves it).
ZOT_PROXY="https://api.rv.pkgforge.dev/"

OPEN_PORT="5000"
REQ_PORT="5443"
REPO_NAME="fixture"
REPO_TAG="t1"
FIX_USER="fixture"

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
	printf 'network           none (container loopback only; asserted in clause iso-1)\n'
	echo
} >"$WORK/report"

fail=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { say "  ok $1"; }
refuse() { say "  FAIL: $1"; fail=1; }

# ------------------------------------------------------- the podbox binary
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
# ⛔ `-f` and not `-x` on a non-native lane: NTFS carries no POSIX mode bit,
# so a lane-built ELF arrives without it and still executes once staged
# into a container (the driver chmods its mounts). `-x` is the check where
# the binary executes here.
if [ "$NATIVE" -eq 1 ]; then [ -x "$BIN" ] || BIN=""; else [ -f "$BIN" ] || BIN=""; fi
if [ -z "$BIN" ]; then
	say "== podbox binary: not staged, building it in the lane"
	cat >"$WORK/pb-build-job.sh" <<'JOB_EOF'
#!/bin/sh
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cargo build --release --target x86_64-unknown-linux-musl --manifest-path "$PWD/Cargo.toml" || exit 2
cp "/work/target/x86_64-unknown-linux-musl/release/podbox" /out/podbox || exit 2
echo "== podbox staged"
exit 0
JOB_EOF
	mkdir -p "$WORK/stage" || exit 2
	if PODBOX_ARTIFACTS="$WORK/stage" sh "$REPO/scripts/windows/run-in-base.sh" "$WORK/pb-build-job.sh" >"$WORK/pb-build.log" 2>&1; then
		BIN="$WORK/stage/podbox"
	else
		echo "SKIP: no podbox binary (set PODBOX_BIN or repair the lane build)" >&2
		tail -n 10 "$WORK/pb-build.log" >&2
		exit 2
	fi
fi
if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || { echo "SKIP: podbox binary is not executable: $BIN" >&2; exit 2; }
else
	[ -f "$BIN" ] && [ "$(wc -c <"$BIN" | tr -d ' ')" != "0" ] || { echo "SKIP: podbox binary is missing or empty: $BIN" >&2; exit 2; }
fi
say "== podbox: staged musl binary ($(wc -c <"$BIN" | tr -d ' ') bytes; path omitted: host checkouts differ per lane, version read back inside the driver)"

# ------------------------------------------------------- the zot binary
say "== zot pin"
mkdir -p "$WORK/zot" || exit 2
if [ ! -f "$WORK/zot/zot" ] || [ "$(wc -c <"$WORK/zot/zot" | tr -d ' ')" != "$ZOT_BIN_BYTES" ] || [ "$(sha256sum "$WORK/zot/zot" | cut -d' ' -f1)" != "$ZOT_BIN_SHA256" ]; then
	rm -f "$WORK/zot/zot"
	fetched=0
	for base in "" "$ZOT_PROXY"; do
		if curl -sS --max-time 600 -H 'User-Agent: curl/8.21.0' -o "$WORK/zot/zot" "${base}${ZOT_BIN_URL}" 2>/dev/null; then
			# ⛔ curl without -f exits 0 on an HTTP error page, so a
			# refused direct fetch reads as a download. The size gates
			# the break; the checksum below gates the run.
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
# ⛔ The baked hash is tied to the release, not only to itself: the release
# checksums file must carry the same line, fetched beside the binary.
for base in "" "$ZOT_PROXY"; do
	if curl -sS --max-time 60 -H 'User-Agent: curl/8.21.0' -o "$WORK/zot/checksums.sha256.txt" "${base}${ZOT_SUMS_URL}" 2>/dev/null && [ -s "$WORK/zot/checksums.sha256.txt" ]; then
		break
	fi
done
grep -q "$ZOT_BIN_SHA256.*zot-linux-amd64-minimal" "$WORK/zot/checksums.sha256.txt" 2>/dev/null || { echo "SKIP: release checksums do not carry the pinned line" >&2; exit 2; }
pass "zot $ZOT_VERSION, $ZOT_BIN_BYTES bytes, sha256 matches the release line"

# ------------------------------------------------------- the seeded storage
# One repo, hand-crafted bytes, no network: config + one layer + manifest,
# each file named by its own sha256, plus the tag index and the layout file.
say "== seed"
seed_repo() {
	_root="$1"; _repo="$2"; _tag="$3"
	_d="$_root/$_repo"
	mkdir -p "$_d/blobs/sha256" || return 1
	_w="$WORK/seed-tmp"; rm -rf "$_w"; mkdir -p "$_w/root" || return 1
	printf 'fixture-one\n' >"$_w/root/one.txt"
	printf 'fixture-two\n' >"$_w/root/two.txt"
	(cd "$_w/root" && tar -cf "$_w/layer.tar" ./one.txt ./two.txt) || return 1
	_diff="$(sha256sum "$_w/layer.tar" | cut -d' ' -f1)" || return 1
	gzip -n -c "$_w/layer.tar" >"$_d/blobs/sha256/layer.tmp" || return 1
	_lhex="$(sha256sum "$_d/blobs/sha256/layer.tmp" | cut -d' ' -f1)" || return 1
	_lsize="$(wc -c <"$_d/blobs/sha256/layer.tmp" | tr -d ' ')" || return 1
	mv "$_d/blobs/sha256/layer.tmp" "$_d/blobs/sha256/$_lhex" || return 1
	printf '{"architecture":"amd64","os":"linux","rootfs":{"type":"layers","diff_ids":["sha256:%s"]}}' "$_diff" >"$_d/blobs/sha256/config.tmp" || return 1
	_chex="$(sha256sum "$_d/blobs/sha256/config.tmp" | cut -d' ' -f1)" || return 1
	_csize="$(wc -c <"$_d/blobs/sha256/config.tmp" | tr -d ' ')" || return 1
	mv "$_d/blobs/sha256/config.tmp" "$_d/blobs/sha256/$_chex" || return 1
	printf '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:%s","size":%s},"layers":[{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:%s","size":%s}]}' "$_chex" "$_csize" "$_lhex" "$_lsize" >"$_d/blobs/sha256/manifest.tmp" || return 1
	_mhex="$(sha256sum "$_d/blobs/sha256/manifest.tmp" | cut -d' ' -f1)" || return 1
	_msize="$(wc -c <"$_d/blobs/sha256/manifest.tmp" | tr -d ' ')" || return 1
	mv "$_d/blobs/sha256/manifest.tmp" "$_d/blobs/sha256/$_mhex" || return 1
	printf '{"schemaVersion":2,"manifests":[{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:%s","size":%s,"annotations":{"org.opencontainers.image.ref.name":"%s"}}]}' "$_mhex" "$_msize" "$_tag" >"$_d/index.json" || return 1
	printf '{"imageLayoutVersion":"1.0.0"}' >"$_d/oci-layout" || return 1
	rm -rf "$_w" || true
	printf '%s' "$_mhex"
	return 0
}
# Two storage dirs: zot locks one cache.db per root, so the open and the
# required servers never share one (measured 2026-09-22: the second serve
# dies on the first's lock).
SEED_OPEN="$(seed_repo "$WORK/w/storage-open" "$REPO_NAME" "$REPO_TAG")" || { echo "SKIP: seed failed" >&2; exit 2; }
SEED_REQ="$(seed_repo "$WORK/w/storage-required" "$REPO_NAME" "$REPO_TAG")" || { echo "SKIP: seed failed" >&2; exit 2; }
# ⛔ The two digests legitimately differ: tar headers carry the files'
# mtimes, so two seed runs minutes apart are different bytes. Each side of
# the driver uses its own root's digest, baked below as SEED_OPEN/SEED_REQ.
pass "seeded $REPO_NAME:$REPO_TAG open sha256:$SEED_OPEN, required sha256:$SEED_REQ"

# ------------------------------------------------------- configs, cert, password
say "== configs"
# ⛔ Absolute container paths: the shipped examples use CWD-relative paths,
# which break when the driver's working directory differs.
cat >"$WORK/w/config-open.json" <<'EOF'
{
  "distSpecVersion": "1.1.1",
  "storage": { "rootDirectory": "/w/storage-open" },
  "http": {
    "address": "127.0.0.1",
    "port": "5000",
    "realm": "zot",
    "tls": { "cert": "/w/server.cert", "key": "/w/server.key" }
  },
  "log": { "level": "error" }
}
EOF
cat >"$WORK/w/config-required.json" <<'EOF'
{
  "distSpecVersion": "1.1.1",
  "storage": { "rootDirectory": "/w/storage-required" },
  "http": {
    "address": "127.0.0.1",
    "port": "5443",
    "realm": "zot",
    "tls": { "cert": "/w/server.cert", "key": "/w/server.key" },
    "auth": { "htpasswd": { "path": "/w/htpasswd" } }
  },
  "log": { "level": "error" }
}
EOF
# ⛔ A password per run, never printed, never committed: it lives in WORK,
# which the EXIT trap removes, and the last clause asserts it is absent
# from the result file.
FIX_PASS="$(head -c16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
printf '%s' "$FIX_PASS" >"$WORK/w/.pass"
FIX_SALT="$(head -c8 /dev/urandom | od -An -tx1 | tr -d ' \n' | cut -c1-16)"
FIX_HASH="$(openssl passwd -5 -salt "$FIX_SALT" "$FIX_PASS")" || { echo "SKIP: htpasswd hash failed" >&2; exit 2; }
printf '%s:%s\n' "$FIX_USER" "$FIX_HASH" >"$WORK/w/htpasswd"
# This lane's OpenSSL reads a system config its own build rejects, so the
# certificate is built against an empty config file named in the Windows
# spelling a native binary reads (the 280 shape), with conversion off for
# the call or -subj arrives as a path.
: >"$WORK/w/empty.cnf"
_WINWORK="$(winpath "$WORK/w")"
_OLD_NO_PATHCONV="${MSYS_NO_PATHCONV:-}"
_OLD_ARG_CONV_EXCL="${MSYS2_ARG_CONV_EXCL:-}"
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
OPENSSL_CONF="$_WINWORK/empty.cnf" openssl req -x509 -newkey rsa:2048 -keyout "$_WINWORK/server.key" -out "$_WINWORK/server.cert" -days 2 -nodes -subj "/CN=127.0.0.1" -addext "subjectAltName=IP:127.0.0.1" >"$WORK/openssl.log" 2>&1 || { echo "SKIP: certificate failed" >&2; tail -n 5 "$WORK/openssl.log" >&2; exit 2; }
export MSYS_NO_PATHCONV="$_OLD_NO_PATHCONV" MSYS2_ARG_CONV_EXCL="$_OLD_ARG_CONV_EXCL"
[ -s "$WORK/w/server.cert" ] && [ -s "$WORK/w/server.key" ] || { echo "SKIP: certificate came out empty" >&2; exit 2; }
pass "configs written, cert carries SAN IP:127.0.0.1, htpasswd holds one bcrypt-or-crypt user"

# ------------------------------------------------------- the driver image
say "== driver image"
mkdir -p "$WORK/ctx" || exit 2
# ⛔ The build log rides stdout beside the id, so the id is the last line,
# not the whole output; the exit code still belongs to the build.
_BUILD_OUT="$(eng_build 600 podbox-x180-driver "$REPO/experiments/180-driver.Dockerfile" "$WORK/ctx" --)" || { echo "SKIP: driver image did not build" >&2; exit 2; }
DRIVER_ID="$(printf '%s' "$_BUILD_OUT" | tail -n 1)"
pass "driver $DRIVER_ID"

# ------------------------------------------------------- the driver script
# Written with literals baked in (digests, ports, repo); the password is
# read from /w/.pass inside, never baked and never printed.
say "== driver run (network none)"
cat >"$WORK/w/driver.sh" <<DRIVER_EOF
#!/bin/sh
# 180 inside the none-network container: zot and podbox on loopback alone.
set -u
SEED="$SEED_OPEN"
SEED_REQ="$SEED_REQ"
USER="$FIX_USER"
PORT_OPEN="$OPEN_PORT"
PORT_REQ="$REQ_PORT"
REPO="$REPO_NAME"
TAG="$REPO_TAG"
fails=0
ok() { printf '  ok %s\n' "\$1"; }
bad() { printf '  FAIL: %s\n' "\$1"; fails=\$((fails + 1)); }
chmod +x /zb 2>/dev/null || true
echo "-- staged binaries"
printf '  podbox: %s\n' "\$(/pb version 2>/dev/null || echo MISSING)"
printf '  zot: binary executes (pin verified host-side; zot has no version subcommand carrying the release tag)\n'
/zb --help >/dev/null 2>&1 || { bad "staged zot does not execute"; }

echo "-- isolation (the Prove's block, asserted not assumed)"
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

echo "-- zot verify (the binary runs)"
if /zb verify /w/config-open.json >/dev/null 2>&1; then ok "cfg-1: open config verifies"; else bad "cfg-1: open config rejected"; fi
if /zb verify /w/config-required.json >/dev/null 2>&1; then ok "cfg-2: required config verifies"; else bad "cfg-2: required config rejected"; fi

echo "-- open server on 127.0.0.1:\$PORT_OPEN"
/zb serve /w/config-open.json >/w/zot-open.log 2>&1 &
ZOT_OPEN=\$!
poll=0
while [ "\$poll" -lt 20 ]; do
	if curl -sS --max-time 5 --cacert /w/server.cert -o /dev/null "https://127.0.0.1:\$PORT_OPEN/v2/" 2>/dev/null; then break; fi
	poll=\$((poll + 1)); sleep 1
done
if curl -sS --max-time 15 --cacert /w/server.cert -o /dev/null -w "%{http_code}" "https://127.0.0.1:\$PORT_OPEN/v2/" | grep -qx 200; then
	ok "open-1: GET /v2/ anon is 200"
else
	bad "open-1: GET /v2/ anon is not 200"
fi
curl -sS --max-time 15 --cacert /w/server.cert -H "Accept: application/vnd.oci.image.manifest.v1+json" "https://127.0.0.1:\$PORT_OPEN/v2/\$REPO/manifests/\$TAG" -o /w/got-tag.json -w "%{http_code}" > /w/code1 2>/dev/null
if [ "\$(cat /w/code1)" = "200" ] && [ "\$(sha256sum /w/got-tag.json | cut -d' ' -f1)" = "\$SEED" ]; then
	ok "open-2: manifest by tag is 200 and is the seeded bytes"
else
	bad "open-2: manifest by tag failed or differs from the seed"
fi
curl -sS --max-time 15 --cacert /w/server.cert "https://127.0.0.1:\$PORT_OPEN/v2/\$REPO/manifests/sha256:\$SEED" -o /w/got-digest.json -w "%{http_code}" > /w/code2 2>/dev/null
if [ "\$(cat /w/code2)" = "200" ] && cmp -s /w/got-tag.json /w/got-digest.json; then
	ok "open-3: manifest by digest is 200 and identical to by tag"
else
	bad "open-3: manifest by digest failed or differs"
fi
LAYER="\$(for b in /w/storage-open/\$REPO/blobs/sha256/*; do n=\$(basename "\$b"); case "\$n" in "\$SEED") ;; *) printf '%s' "\$n"; break;; esac; done)"
curl -sS --max-time 15 --cacert /w/server.cert "https://127.0.0.1:\$PORT_OPEN/v2/\$REPO/blobs/sha256:\$LAYER" -o /w/got-blob -w "%{http_code}" > /w/code3 2>/dev/null
if [ "\$(cat /w/code3)" = "200" ] && cmp -s /w/got-blob "/w/storage-open/\$REPO/blobs/sha256/\$LAYER"; then
	ok "open-4: blob is 200 and is the seeded bytes"
else
	bad "open-4: blob failed or differs from the seed"
fi
if SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/pbstore /pb pull "127.0.0.1:\$PORT_OPEN/\$REPO:\$TAG" >/w/pull-tag.log 2>&1; then
	GOT="\$(SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/pbstore /pb inspect --format '{{.Digest}}' "127.0.0.1:\$PORT_OPEN/\$REPO:\$TAG" 2>/dev/null)"
	if [ "\$GOT" = "sha256:\$SEED" ]; then
		ok "open-5: podbox pull tag exits 0 and inspects to the seeded digest"
	else
		bad "open-5: pulled digest \$GOT is not sha256:\$SEED"
	fi
else
	bad "open-5: podbox pull tag failed"
fi
if SSL_CERT_FILE=/w/server.cert PODBOX_STORE=/w/pbstore /pb pull "127.0.0.1:\$PORT_OPEN/\$REPO@sha256:\$SEED" >/w/pull-digest.log 2>&1; then
	ok "open-6: podbox pull by digest exits 0"
else
	bad "open-6: podbox pull by digest failed"
fi
kill "\$ZOT_OPEN" 2>/dev/null || true
wait "\$ZOT_OPEN" 2>/dev/null || true
rm -f /w/storage-open/cache.db || true

echo "-- required server on 127.0.0.1:\$PORT_REQ"
PASS="\$(cut -d: -f2 /w/.pass)"
/zb serve /w/config-required.json >/w/zot-req.log 2>&1 &
ZOT_REQ=\$!
poll=0
while [ "\$poll" -lt 20 ]; do
	if curl -sS --max-time 5 --cacert /w/server.cert -o /dev/null "https://127.0.0.1:\$PORT_REQ/v2/" 2>/dev/null; then break; fi
	poll=\$((poll + 1)); sleep 1
done
if curl -sS --max-time 15 --cacert /w/server.cert -o /dev/null -w "%{http_code}" "https://127.0.0.1:\$PORT_REQ/v2/" | grep -qx 401; then
	if curl -sS --max-time 15 --cacert /w/server.cert -o /dev/null -D /w/hdrs "https://127.0.0.1:\$PORT_REQ/v2/" 2>/dev/null; then true; fi
	if grep -qi 'www-authenticate: basic' /w/hdrs 2>/dev/null; then
		ok "req-1: GET /v2/ anon is 401 with a Basic challenge"
	else
		bad "req-1: 401 without a Basic challenge"
	fi
else
	bad "req-1: GET /v2/ anon is not 401"
fi
if curl -sS --max-time 15 --cacert /w/server.cert -u "\$USER:\$PASS" -o /dev/null -w "%{http_code}" "https://127.0.0.1:\$PORT_REQ/v2/" | grep -qx 200; then
	ok "req-2: GET /v2/ authed is 200"
else
	bad "req-2: GET /v2/ authed is not 200"
fi
if curl -sS --max-time 15 --cacert /w/server.cert -o /dev/null -w "%{http_code}" -H "Accept: application/vnd.oci.image.manifest.v1+json" "https://127.0.0.1:\$PORT_REQ/v2/\$REPO/manifests/\$TAG" | grep -qx 401; then
	ok "req-3: manifest anon is 401"
else
	bad "req-3: manifest anon is not 401"
fi
curl -sS --max-time 15 --cacert /w/server.cert -u "\$USER:\$PASS" -H "Accept: application/vnd.oci.image.manifest.v1+json" "https://127.0.0.1:\$PORT_REQ/v2/\$REPO/manifests/\$TAG" -o /w/got-req.json -w "%{http_code}" > /w/code4 2>/dev/null
if [ "\$(cat /w/code4)" = "200" ] && [ "\$(sha256sum /w/got-req.json | cut -d' ' -f1)" = "\$SEED_REQ" ]; then
	ok "req-4: manifest authed is 200 and is the seeded bytes"
else
	bad "req-4: manifest authed failed or differs"
fi
if curl -sS --max-time 15 --cacert /w/server.cert -u "\$USER:wrong-password" -o /dev/null -w "%{http_code}" "https://127.0.0.1:\$PORT_REQ/v2/" | grep -qx 401; then
	ok "req-5: wrong password is 401"
else
	bad "req-5: wrong password is not 401"
fi
kill "\$ZOT_REQ" 2>/dev/null || true
wait "\$ZOT_REQ" 2>/dev/null || true

if [ "\$fails" -eq 0 ]; then echo "driver: all clauses held"; else echo "driver: \$fails clause(s) failed"; fi
exit "\$fails"
DRIVER_EOF
eng_mount "$WORK/zot/zot" /zb
eng_mount "$BIN" /pb
eng_mount "$WORK/w" /w rw
# shellcheck disable=SC2034 # read by experiments/lib/engine.sh's eng_run
ENG_NETWORK=none
if eng_run 600 "$DRIVER_ID" "" -- /bin/sh /w/driver.sh >>"$WORK/report" 2>&1; then
	pass "driver run exits 0 with network none"
else
	refuse "driver run failed (network none)"
fi
eng_clear

# ------------------------------------------------------- the verdict
say "== verdict"
if grep -q '^  FAIL' "$WORK/report"; then
	refuse "report carries FAIL lines"
fi
if grep -q -F "$FIX_PASS" "$WORK/report"; then
	refuse "the test password reached the report"
else
	pass "the test password is absent from the report"
fi

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
exit 0
