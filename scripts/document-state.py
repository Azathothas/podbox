#!/usr/bin/env python3
"""Compat shim for the source-state port (T-1564). The page lives in the
document-state binary in crates/podbox-release; this file only execs it
with the same arguments, so the record gate's document check and the
py_compile gate step keep working."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
CANDIDATES = (
    ROOT / "target" / "release" / ("document-state" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("document-state" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("document-state" + SUFFIX),
    ROOT / "target" / "debug" / ("document-state" + SUFFIX),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("document-state: the binary is not built; cargo build -p podbox-release",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
