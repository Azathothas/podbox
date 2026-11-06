#!/bin/sh
# Scratch: replicate 146 clause 6 on the host. Not committed.
set -u
T="$(mktemp -d)"; trap 'rm -rf "$T"; kill "$STALLPID" 2>/dev/null || true' EXIT
cat >"$T/stall.py" <<'PYEOF'
import socket, time
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 0))
s.listen(1)
print(s.getsockname()[1], flush=True)
c, _ = s.accept()
time.sleep(30)
PYEOF
rm -f "$T/stall.port"
py "$T/stall.py" >"$T/stall.port" 2>"$T/stall.err" &
STALLPID=$!
PORT=""
i=0
while [ "$i" -lt 25 ] && [ ! -s "$T/stall.port" ]; do
	sleep 0.2
	i=$((i + 1))
done
[ -s "$T/stall.port" ] && PORT="$(cat "$T/stall.port")"
echo "PORT=$PORT"
if [ -z "$PORT" ]; then
	echo "NO-PORT"; cat "$T/stall.err"
else
	start=$(date +%s)
	timeout 10 curl -fsSL --max-time 2 "http://127.0.0.1:$PORT/" -o "$T/stall.out" 2>"$T/curl.err"
	rc=$?
	end=$(date +%s)
	echo "rc=$rc elapsed=$((end - start))s"
fi
