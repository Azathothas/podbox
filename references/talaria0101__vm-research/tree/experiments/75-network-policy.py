#!/usr/bin/env python3
# Question: 11- found "no TCP bind, no loopback connect" — but seccomp cannot
# inspect sockaddr contents, so a number-based filter CANNOT be the mechanism.
# Map the actual network policy precisely: which family/type/port/address
# combinations are denied, and by which errno pattern, to identify the
# enforcing mechanism and its exact boundary.
import socket, sys, time, platform

LOG = "/workspace/vm-research/experiments/logs/75-network-policy.log"
out = open(LOG, "a")

def t(label, fn):
    try:
        fn(); out.write(f"{label}: OK\n")
    except OSError as e:
        out.write(f"{label}: DENIED errno={e.errno} ({e.strerror})\n")

def bind_tcp(ip, port):
    def f():
        s = socket.socket(); s.bind((ip, port)); s.listen(1); s.close()
    return f

def connect_tcp(ip, port):
    def f():
        s = socket.socket(); s.settimeout(6); s.connect((ip, port)); s.close()
    return f

def bind_udp(ip, port):
    def f():
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM); s.bind((ip, port)); s.close()
    return f

out.write("## conditions\n")
out.write(f"date: {time.strftime('%FT%TZ', time.gmtime())}\n")
out.write(f"host: {platform.platform()}\n\n")

for port in (80, 443, 8080, 65002):
    t(f"bind TCP 127.0.0.1:{port}", bind_tcp("127.0.0.1", port))
t("bind TCP 0.0.0.0:65003", bind_tcp("0.0.0.0", 65003))
t("bind TCP 192.168.1.64:65004 (host wlan IP)", bind_tcp("192.168.1.64", 65004))

for host, port, label in [
    ("127.0.0.1", 65005, "connect TCP 127.0.0.1:65005 (own listener)"),
    ("192.168.1.64", 443, "connect TCP 192.168.1.64:443 (host wlan IP, closed port)"),
    ("10.0.2.2", 443, "connect TCP 10.0.2.2:443 (private RFC1918)"),
    ("151.101.194.132", 80, "connect TCP fastly:80 (http)"),
    ("151.101.194.132", 443, "connect TCP fastly:443 (https)"),
    ("1.1.1.1", 80, "connect TCP 1.1.1.1:80"),
    ("1.1.1.1", 443, "connect TCP 1.1.1.1:443"),
]:
    t(label, connect_tcp(host, port))

t("bind UDP 127.0.0.1:65006", bind_udp("127.0.0.1", 65006))
t("bind UDP 0.0.0.0:65007", bind_udp("0.0.0.0", 65007))

def unix_bind():
    import os
    p = "/workspace/vm-research/experiments/work/75-test.sock"
    if os.path.exists(p): os.remove(p)
    s = socket.socket(socket.AF_UNIX); s.bind(p); s.close(); os.remove(p)
t("bind AF_UNIX path", unix_bind)

def icmp_raw():
    s = socket.socket(socket.AF_INET, socket.SOCK_RAW, socket.IPPROTO_ICMP); s.close()
t("SOCK_RAW IPPROTO_ICMP", icmp_raw)

out.write("\ninterpretation: see docs/seccomp.md section 'network policy mechanism'\n")
out.close()
print(open(LOG).read())
