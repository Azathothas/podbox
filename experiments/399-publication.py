#!/usr/bin/env python3
"""Compat shim for the publication port (T-1567). The acceptance lives in
the podbox-publish binary in crates/podbox-release; this file only execs
it with the same arguments, so the py_compile gate step keeps working. No
remote writes: signature content verification stays with podbox-verify."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
CANDIDATES = (
    ROOT / "target" / "release" / ("podbox-publish" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("podbox-publish" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("podbox-publish" + SUFFIX),
    ROOT / "target" / "debug" / ("podbox-publish" + SUFFIX),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("publication: podbox-publish is not built; cargo build -p podbox-release",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
