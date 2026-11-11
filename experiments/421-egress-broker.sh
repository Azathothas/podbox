#!/bin/sh
# Question: does a payload reach a proxy on the host's loopback, on a
# rootfs tier with the variable unchanged and in a guest of the machine
# tier at the gateway, with its credentials intact?
#
# TODO/net.md T-1665 (issue 184). The proxy is a small HTTP proxy of this
# script on 127.0.0.1 (python3): it answers 200 and the body
# `proxied-421` to a GET with the right Basic credential, and 407 to
# each other request. The password holds an `@`.
#
# Pinned input: ALPINE of experiment 356 (alpine 3.20 by digest).
# PODVM_421_MIRROR=1 takes the same digest from mirror.gcr.io.
#
# The broker clauses run the static busybox of busybox:musl on the syscall
# tier (T-1657) on the chroot-denying fixture of experiment 356, and alpine
# in a TCG guest of the machine tier. busybox:musl has no TLS client, so the
# broker allows port 80 beside 443 and the fetch of the allowed host is
# plain HTTP. PODVM_421_MIRROR=1 takes busybox:musl from mirror.gcr.io too.
#
# Clauses:
#   1. a rootfs-tier payload (the ladder's rung) fetches through the
#      unchanged variable: 200, `proxied-421`.
#   2. a machine-tier guest sees the proxy at 10.0.2.2 with the scheme,
#      the credentials and the port intact, and fetches: 200.
#   3. without the credential the proxy answers 407, so clauses 1 and 2
#      prove the credential arrived.
#   4. a proxy URL with no port refuses the machine-tier run by name,
#      and the refusal does not hold the password.
#   5. `--network broker --ports 80,443 --allow example.com` on the syscall
#      tier: 200 for the allowed host, 403 for port 22 and for 127.0.0.1,
#      407 without the run's credential, and a payload that ignores the
#      variables cannot connect (EACCES).
#   6. the same on the machine tier: 200, 403 for port 22, and a direct
#      connect is refused (the guest network is restricted).
#   7. with no holding mechanism (the ladder's rung) `--network broker`
#      refuses by name (exit 125).
#   8. on the syscall tier, `/etc/hosts` maps the allowed name to a fake
#      address 198.18.0.N, and a payload that ignores the variables
#      reaches the allowed host through the broker (the handler turns its
#      connect into a CONNECT tunnel); port 22 of the same name is refused
#      (EACCES, from the broker's 403).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep421-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/egress-broker.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
[ "${PODVM_421_MIRROR:-0}" = 1 ] && ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
PROXY_PID=""
trap '[ -n "$PROXY_PID" ] && kill "$PROXY_PID" 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }

# A fresh credential of this run, with an `@` in the password.
SECRET="p@ss-$(od -An -N6 -tx1 /dev/urandom | tr -d ' \n')"
cat >"$WORK/proxy.py" <<'EOF'
import base64, http.server, os, sys
want = "Basic " + base64.b64encode(("u421:" + os.environ["SECRET"]).encode()).decode()
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get("Proxy-Authorization") != want:
            self.send_response(407)
            self.send_header("Proxy-Authenticate", 'Basic realm="421"')
            self.end_headers()
            return
        body = b"proxied-421\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *a):
        sys.stderr.write("proxy: %s\n" % (a[0] % a[1:]))
s = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
print(s.server_address[1], flush=True)
s.serve_forever()
EOF
SECRET="$SECRET" python3 "$WORK/proxy.py" >"$WORK/port" 2>"$WORK/proxy.log" &
PROXY_PID=$!
i=0
while [ ! -s "$WORK/port" ] && [ "$i" -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
PORT=$(cat "$WORK/port")
[ -n "$PORT" ] || { note "the proxy did not start: COULD NOT RUN"; exit 2; }
URL="http://u421:$SECRET@127.0.0.1:$PORT"

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "proxy             http://u421:CRED@127.0.0.1:PORT (a password of this run with an @)"
timeout 600 "$PB" pull -q "$ALPINE" >"$WORK/pull.log" 2>&1 ||
	{ note "pull FAILED: COULD NOT RUN"; sed 's/^/  /' "$WORK/pull.log"; exit 2; }
note ""

FETCH='wget -q -O - http://podvm-421.invalid/ 2>&1; echo "proxy-seen=$(echo "$http_proxy" | sed "s#//.*@#//CRED@#")"'

note "== clause 1  a rootfs-tier payload, the variable unchanged"
timeout 300 "$PB" run --rm -e http_proxy="$URL" "$ALPINE" sh -c "$FETCH" >"$WORK/c1.out" 2>"$WORK/c1.err"
rc=$?
sed "s#$PORT#PORT#g" "$WORK/c1.out" | sed 's/^/    /' | tee -a "$OUT"
[ "$rc" -eq 0 ] || bad "the run exited $rc"
grep -qx "proxied-421" "$WORK/c1.out" || bad "the payload did not fetch through the proxy"
grep -q "proxy-seen=http://CRED@127.0.0.1:$PORT" "$WORK/c1.out" || bad "the variable changed on the rootfs tier"
note ""

note "== clause 2  a machine-tier guest, the proxy at the gateway"
timeout 600 "$PB" run --rm --podvm-tier=machine -e http_proxy="$URL" "$ALPINE" sh -c "$FETCH" >"$WORK/c2.out" 2>"$WORK/c2.err"
rc=$?
sed "s#$PORT#PORT#g" "$WORK/c2.out" | sed 's/^/    /' | tee -a "$OUT"
[ "$rc" -eq 0 ] || { bad "the guest run exited $rc"; tail -3 "$WORK/c2.err" | sed 's/^/      /' | tee -a "$OUT"; }
grep -qx "proxied-421" "$WORK/c2.out" || bad "the guest did not fetch through the proxy"
grep -q "proxy-seen=http://CRED@10.0.2.2:$PORT" "$WORK/c2.out" || bad "the guest does not see the proxy at the gateway"
note ""

note "== clause 3  the credential is what passed"
timeout 300 "$PB" run --rm -e http_proxy="http://127.0.0.1:$PORT" "$ALPINE" sh -c 'wget -q -O - http://podvm-421.invalid/ 2>&1 | head -1' >"$WORK/c3.out" 2>&1
note "    without it: $(head -1 "$WORK/c3.out")"
grep -q "407" "$WORK/c3.out" || bad "the proxy did not answer 407 without the credential"
grep -c '" 200 ' "$WORK/proxy.log" | sed 's/^/    200 answers of the proxy: /' | tee -a "$OUT"
note ""

note "== clause 4  a URL with no port refuses the guest by name"
timeout 120 "$PB" run --rm --podvm-tier=machine -e http_proxy="http://u421:$SECRET@127.0.0.1" "$ALPINE" true >"$WORK/c4.out" 2>&1
rc=$?
note "  exit $rc: $(grep -m1 'http_proxy' "$WORK/c4.out")"
[ "$rc" -eq 125 ] || bad "the run exited $rc"
grep -q "http_proxy cannot be given to the guest" "$WORK/c4.out" || bad "the refusal does not name the variable"
grep -q "$SECRET" "$WORK/c4.out" && bad "the refusal holds the password"
note ""

BUSYBOX='public.ecr.aws/docker/library/busybox:musl'
[ "${PODVM_421_MIRROR:-0}" = 1 ] && BUSYBOX='mirror.gcr.io/library/busybox:musl'
timeout 600 "$PB" pull -q "$BUSYBOX" >"$WORK/pull2.log" 2>&1 ||
	{ note "pull of $BUSYBOX FAILED: COULD NOT RUN"; exit 2; }
command -v cc >/dev/null 2>&1 || { note "cc missing: COULD NOT RUN"; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ note "fixture COMPILE FAILED: COULD NOT RUN"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF2'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF2
BROKER='
echo "proxy-seen=$(echo "$http_proxy" | sed "s#//.*@#//CRED@#")"
echo "allowed: $(wget -q -T 10 -O - http://example.com/ 2>&1 | grep -o "<title>[^<]*</title>" | head -1)"
echo "port22: $(wget -q -T 10 -O - http://example.com:22/ 2>&1 | head -1)"
echo "loopback: $(wget -q -T 10 -O - http://127.0.0.1:80/ 2>&1 | head -1)"
echo "nocred: $(http_proxy=$(echo "$http_proxy" | sed "s#//.*@#//#") wget -q -T 10 -O - http://example.com/ 2>&1 | head -1)"
echo "direct: $(http_proxy= wget -q -T 5 -O - http://1.1.1.1/ 2>&1 | head -1)"
echo "fake: $(grep -m1 "[[:space:]]example.com" /etc/hosts)"
echo "ignores: $(http_proxy= wget -q -T 10 -O - http://example.com/ 2>&1 | grep -o "<title>[^<]*</title>" | head -1)"
echo "ignores22: $(http_proxy= wget -q -T 10 -O - http://example.com:22/ 2>&1 | head -1)"'

note "== clause 5  the broker on the syscall tier"
unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" run --rm --podvm-tier=syscall --network broker --ports 80,443 --allow example.com "$BUSYBOX" sh -c "$BROKER" >"$WORK/c5.out" 2>"$WORK/c5.err"
rc=$?
sed "s#$PORT#PORT#g" "$WORK/c5.out" | sed 's/^/    /' | tee -a "$OUT"
[ "$rc" -eq 0 ] || { bad "the run exited $rc"; tail -3 "$WORK/c5.err" | sed 's/^/      /' | tee -a "$OUT"; }
grep -q "^allowed: <title>Example Domain</title>" "$WORK/c5.out" || bad "the allowed host did not answer 200"
grep -q "^port22: .*403" "$WORK/c5.out" || bad "port 22 is not 403"
grep -q "^loopback: .*403" "$WORK/c5.out" || bad "127.0.0.1 is not 403"
grep -q "^nocred: .*407" "$WORK/c5.out" || bad "no credential is not 407"
grep -q "^direct: .*Permission denied" "$WORK/c5.out" || bad "a direct connect was not refused"
note "== clause 8  a payload that ignores the variables, on the syscall tier"
grep -q "^fake: 198\.18\.0\.1[[:space:]]*example\.com" "$WORK/c5.out" || bad "/etc/hosts does not map example.com to 198.18.0.1"
grep -q "^ignores: <title>Example Domain</title>" "$WORK/c5.out" || bad "the payload that ignores the variables did not reach the allowed host"
grep -q "^ignores22: .*Permission denied" "$WORK/c5.out" || bad "port 22 through the fake address was not refused"
note ""

note "== clause 6  the broker on the machine tier"
timeout 600 "$PB" run --rm --podvm-tier=machine --timeout 300 --network broker --ports 80,443 --allow example.com "$ALPINE" sh -c "$BROKER" >"$WORK/c6.out" 2>"$WORK/c6.err"
rc=$?
sed 's/^/    /' "$WORK/c6.out" | tee -a "$OUT"
[ "$rc" -eq 0 ] || { bad "the guest run exited $rc"; tail -3 "$WORK/c6.err" | sed 's/^/      /' | tee -a "$OUT"; }
grep -q "^proxy-seen=http://CRED@10.0.2.100:3128" "$WORK/c6.out" || bad "the guest does not see the broker at its forward"
grep -q "^allowed: <title>Example Domain</title>" "$WORK/c6.out" || bad "the allowed host did not answer 200 in the guest"
grep -q "^port22: .*403" "$WORK/c6.out" || bad "port 22 is not 403 in the guest"
grep -q "^direct: .*refused" "$WORK/c6.out" || bad "a direct connect of the guest was not refused"
note ""

note "== clause 7  no holding mechanism"
timeout 300 "$PB" run --rm --network broker --allow example.com "$ALPINE" true >"$WORK/c7.out" 2>&1
rc=$?
note "  exit $rc: $(grep -m1 'network broker' "$WORK/c7.out" | cut -c1-160)"
[ "$rc" -eq 125 ] || bad "the run exited $rc"
grep -q "needs a mechanism that holds the payload" "$WORK/c7.out" || bad "the refusal does not name the holding mechanism"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: A LOOPBACK PROXY REACHES THE PAYLOAD ON EACH TIER WITH ITS CREDENTIAL, AND THE BROKER HOLDS IT"
else
	note "verdict: A CLAUSE DISAGREED"
fi
pkill -f "qemu-system.*podvm-init" 2>/dev/null
exit "$fail"
