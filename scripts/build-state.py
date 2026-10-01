#!/usr/bin/env python3
"""Compat shim for the build-state port (T-1559). The logic lives in the
podbox-buildstate binary in crates/podbox-buildstate; this file only execs
it with the same arguments, so dev.rs call sites, the py_compile gate step,
and local callers keep working. experiments/393-build-freshness.py drives
the binary directly and proves the behavior, not this shim."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
CANDIDATES = (
    ROOT / "target" / "release" / ("podbox-buildstate" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("podbox-buildstate" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("podbox-buildstate" + SUFFIX),
    ROOT / "target" / "debug" / ("podbox-buildstate" + SUFFIX),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("build-state: podbox-buildstate is not built; cargo build -p podbox-buildstate",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
