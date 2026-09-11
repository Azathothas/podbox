#!/usr/bin/env python3
"""netpolicy.py — characterize the network envelope: which bind/connect
operations are permitted, and what mechanism the denials imply.

A seccomp filter evaluates the syscall number and the argument
registers. It cannot dereference the sockaddr pointer, so it cannot
distinguish 127.0.0.1 from a public address. Denials that are
address-, port- and family-aware therefore belong to a mechanism below
the syscall boundary: a cgroup-BPF SOCK_ADDR program family attached to
the sandbox cgroup (BPF_CGROUP_INET4_BIND / _CONNECT), which can read
the full sockaddr, return EPERM, and be reconfigured live.

This script measures; it does not guess. Every row prints the errno.
Exit 0 always: denials are the result.
"""
import socket
import sys
import os

def verdict(name, fn):
    try:
        r = fn()
        print(f"{name:44s} OK" + (f" ({r})" if r else ""))
    except OSError as e:
        print(f"{name:44s} errno={e.errno} ({e.strerror})")
    except Exception as e:  # noqa: BLE001
        print(f"{name:44s} {type(e).__name__}: {e}")



def bind(fam, typ, addr):
    def f():
        s = socket.socket(fam, typ)
        try:
            s.bind(addr)
        finally:
            s.close()
        return ""
    return f

def connect_tcp(addr, timeout=3.0):
    def f():
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(timeout)
        try:
            s.connect(addr)
            s.close()
            return "connected"
        except ConnectionRefusedError:
            s.close()
            return "ECONNREFUSED — kernel answered: policy allowed the connect"
        finally:
            pass
    return f

def unix_bind():
    def f():
        p = "/tmp/.netpolicy-sock"
        try:
            os.unlink(p)
        except FileNotFoundError:
            pass
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.bind(p)
        s.close()
        os.unlink(p)
        return ""
    return f

def main():
    print("## network policy census (errno per row; denials are results)")
    print("## date:", os.popen("date -u +%FT%TZ").read().strip())
    print()
    print("### bind")
    verdict("bind tcp 127.0.0.1:0 (loopback)", bind(socket.AF_INET, socket.SOCK_STREAM, ("127.0.0.1", 0)))
    verdict("bind tcp 0.0.0.0:0 (wildcard)", bind(socket.AF_INET, socket.SOCK_STREAM, ("0.0.0.0", 0)))
    verdict("bind udp 127.0.0.1:0 (loopback)", bind(socket.AF_INET, socket.SOCK_DGRAM, ("127.0.0.1", 0)))
    verdict("bind udp 0.0.0.0:0 (wildcard)", bind(socket.AF_INET, socket.SOCK_DGRAM, ("0.0.0.0", 0)))
    verdict("bind af_unix", lambda: (unix_bind(), "")[1])
    print()
    print("### connect (TCP)")
    verdict("connect 127.0.0.1:443 (loopback)", connect_tcp(("127.0.0.1", 443)))
    verdict("connect 127.0.0.1:80 (loopback)", connect_tcp(("127.0.0.1", 80)))
    verdict("connect external :443", connect_tcp(("140.82.121.4", 443)))
    verdict("connect external :80", connect_tcp(("140.82.121.4", 80)))
    return 0

if __name__ == "__main__":
    sys.exit(main())
