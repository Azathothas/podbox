#!/bin/sh
# Question: does the channel of a guest of the machine tier
# (`--podvm-channel HOST_SOCKET:127.0.0.1:PORT` with `--podvm-net none`)
# reach the host socket with 9p volumes beside it, and through a proxy
# variable of the payload?
#
# TODO/machine.md T-1778 (PodUtils/podrunner#87) and T-1751. The host
# socket is a small HTTP server of python3 on a unix socket; each request
# that reaches it is one line of its log.
#
# Pinned inputs:
#   OCI     alpine 3.20 of the store PODVM_STORE (default the store of
#           experiment 443), pulled by `podvm pull` where it is missing
#   podvm   PODVM_444_BIN (default the release build of the workspace)
#
# Clauses:
#   1. no volume: wget of http://127.0.0.1:8080/ in the guest gets the
#      answer of the host socket, and /sys/class/net holds `lo` only.
#   2. the run of podrunner's microvm kind: two 9p volumes (one `:ro`),
#      `-e http_proxy=http://127.0.0.1:3128` and the channel on 3128; a
#      user that is not root fetches http://broker.podrunner.test/ through
#      the proxy variable, gets the answer, and reads the `:ro` volume; the
#      host socket logs the request with its full URL.
#   3. with the default network, the proxy variable is still the gateway
#      10.0.2.2 (T-1665).
#   4. no emulator of the runs stays.
#
# Each run has a bound of 300 s. Exit: 0 each clause matched, 1 one or
# more failed, 2 the lane could not run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PB="${PODVM_444_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_STORE:-$REPO/.work/qa/store}"
A=mirror.gcr.io/library/alpine:3.20
W="$(mktemp -d)"
trap 'kill "$SRV" 2>/dev/null; rm -rf "$W"' EXIT INT TERM

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
mkdir -p "$PODVM_STORE" && chmod 700 "$PODVM_STORE"
"$PB" image inspect "$A" >/dev/null 2>&1 || "$PB" pull -q "$A" >/dev/null 2>&1 ||
	{ echo "alpine 3.20 does not pull: COULD NOT RUN" >&2; exit 2; }

cat >"$W/srv.py" <<'EOF'
import socketserver, http.server, sys
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        b = b"answer-of-the-host-444\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)
    def log_message(self, *a):
        sys.stderr.write("request %s\n" % self.path)
        sys.stderr.flush()
class S(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    pass
S(sys.argv[1], H).serve_forever()
EOF
python3 "$W/srv.py" "$W/s" 2>"$W/srv.log" &
SRV=$!
i=0
while [ ! -S "$W/s" ] && [ $i -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
[ -S "$W/s" ] || { echo "the host socket does not listen: COULD NOT RUN" >&2; exit 2; }
mkdir -p "$W/cred" "$W/work"
echo cred-444 >"$W/cred/token"
chmod -R a+rX "$W"

FAIL=0
verdict() { # clause, test, what
	if eval "$2"; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; FAIL=1; fi
}
M="--podvm-tier=machine"

out=$(timeout 300 "$PB" run --rm $M --podvm-net none --podvm-channel "$W/s:127.0.0.1:8080" "$A" \
	sh -c 'wget -qO- http://127.0.0.1:8080/one; ls /sys/class/net' 2>"$W/e1")
printf '%s\n' "$out" | sed 's/^/  1| /'
verdict 1 '[ "$out" = "answer-of-the-host-444
lo" ] && grep -q "request /one" "$W/srv.log"' "the channel alone reaches the host socket; the guest has lo only"

out=$(timeout 300 "$PB" run --rm $M --podvm-net none --podvm-channel "$W/s:127.0.0.1:3128" \
	-v "$W/cred:/mnt/podrunner-cred:ro" -v "$W/work:/podrunner-work" \
	-e http_proxy=http://127.0.0.1:3128 "$A" \
	sh -c 'adduser -D runner >/dev/null 2>&1; su runner -c "wget -qO- http://broker.podrunner.test/two; cat /mnt/podrunner-cred/token"' 2>"$W/e2")
printf '%s\n' "$out" | sed 's/^/  2| /'
verdict 2 '[ "$out" = "answer-of-the-host-444
cred-444" ] && grep -q "request http://broker.podrunner.test/two" "$W/srv.log"' \
	"two 9p volumes and a loopback proxy variable: a user that is not root reaches the host socket"

out=$(timeout 300 "$PB" run --rm $M -e http_proxy=http://127.0.0.1:3128 "$A" sh -c 'echo "$http_proxy"' 2>"$W/e3")
printf '%s\n' "$out" | sed 's/^/  3| /'
verdict 3 '[ "$out" = "http://10.0.2.2:3128" ]' "with the default network the proxy variable is the gateway"

sleep 1
left=$(pgrep -f "qemu-system" 2>/dev/null | wc -l)
verdict 4 '[ "$left" = 0 ]' "no emulator stays ($left)"

if [ $FAIL = 0 ]; then echo "THE CHANNEL OF A GUEST REACHES THE HOST SOCKET"; fi
exit $FAIL
