#!/usr/bin/env python3
"""Compat shim for the licence-inventory port (T-1560). The logic lives in
the podbox-release-licenses binary in crates/podbox-buildstate; this file
only execs it with the same arguments, so scripts/package-ssh.sh and the
py_compile gate step keep working."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
CANDIDATES = (
    ROOT / "target" / "release" / ("podbox-release-licenses" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("podbox-release-licenses" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("podbox-release-licenses" + SUFFIX),
    ROOT / "target" / "debug" / ("podbox-release-licenses" + SUFFIX),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("release-licenses: podbox-release-licenses is not built; cargo build -p podbox-buildstate",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
