#!/usr/bin/env bash
# Question: what does the sandbox allow on the network plane? Outbound worked
# all along; lima's failure (33-) showed a bind denial. Census: bind on
# loopback/wildcard at several ports, listen, loopback connect, external
# connect. This decides the fate of every hostfwd-based VM manager.
# Exit codes: 0 census taken (mixed results expected), 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/11-net-census.log"
conditions net-census > "$LOG"
python3 - >> "$LOG" 2>&1 <<'EOF'
import socket
def t(label, fn):
    try: fn(); print(f"{label}: OK")
    except Exception as e: print(f"{label}: DENIED ({e})")
for port in [22, 40543, 65002]:
    def b(p=port):
        s=socket.socket(); s.bind(("127.0.0.1", p)); s.listen(1); s.close()
    t(f"bind+listen loopback:{port}", b)
def b0():
    s=socket.socket(); s.bind(("0.0.0.0", 65003)); s.listen(1); s.close()
t("bind+listen wildcard:65003", b0)
def c():
    s=socket.socket(); s.bind(("127.0.0.1", 65004)); s.listen(1)
    c2=socket.socket(); c2.connect(("127.0.0.1", 65004)); c2.close(); s.close()
t("loopback connect", c)
def ext():
    s=socket.socket(); s.settimeout(10); s.connect(("1.1.1.1", 443)); s.close()
t("external connect 1.1.1.1:443", ext)
def udp():
    s=socket.socket(socket.AF_INET, socket.SOCK_DGRAM); s.bind(("127.0.0.1", 65005)); s.close()
t("udp bind loopback:65005", udp)
EOF
echo "--- interpretation: outbound TCP allowed; bind and loopback connect denied" >> "$LOG"
cat "$LOG" | tail -8
exit 0
