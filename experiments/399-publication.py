#!/usr/bin/env python3
"""Compat shim for the publication port (T-1567). The acceptance lives in
the podvm-publish binary in crates/podvm-release; this file only execs
it with the same arguments, so the py_compile gate step keeps working. No
remote writes: signature content verification stays with podvm-verify."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
CANDIDATES = (
    ROOT / "target" / "release" / ("podvm-publish" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("podvm-publish" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("podvm-publish" + SUFFIX),
    ROOT / "target" / "debug" / ("podvm-publish" + SUFFIX),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("publication: podvm-publish is not built; cargo build -p podvm-release",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
