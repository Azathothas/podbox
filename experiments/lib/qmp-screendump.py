#!/usr/bin/env python3
"""One screenshot of a QEMU guest through its QMP socket.

Usage: qmp-screendump.py SOCKET OUT.ppm

TODO/gate.md T-1641: the watcher of experiment 392 calls it while a
Windows guest runs, so a run that does not power off leaves pictures of
the screen. It holds the socket for one command only: podvm connects to
the same socket to stop the guest. Exit 0 with the file written, 1
otherwise.
"""
import json
import socket
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    sock, out = sys.argv[1], sys.argv[2]
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(5)
        s.connect(sock)
        f = s.makefile("rw")
        f.readline()  # the greeting
        for cmd in ({"execute": "qmp_capabilities"},
                    {"execute": "screendump", "arguments": {"filename": out}}):
            f.write(json.dumps(cmd) + "\n")
            f.flush()
            while True:
                reply = json.loads(f.readline())
                if "return" in reply or "error" in reply:
                    break
            if "error" in reply:
                print(reply["error"], file=sys.stderr)
                return 1
        s.close()
    except (OSError, ValueError) as e:
        print(e, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
