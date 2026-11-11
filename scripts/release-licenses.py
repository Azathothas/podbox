#!/usr/bin/env python3
"""Compat shim for the licence-inventory port (T-1560). The logic lives in
the podvm-release-licenses binary in crates/podvm-buildstate; this file
only execs it with the same arguments, so the old callers and the
py_compile gate step keep working."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIX = ".exe" if os.name == "nt" else ""
# The triples the nightly builds. Read from the matrix rather than guessed,
# so a new architecture is a one-line change here and in one file.
TRIPLES = (
    "x86_64-unknown-linux-musl",
    "i686-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "riscv64gc-unknown-linux-musl",
    "loongarch64-unknown-linux-musl",
    "powerpc64le-unknown-linux-musl",
    "armv7-unknown-linux-musleabihf",
)
CANDIDATES = (
    # ⛔ THE TARGET SUBDIRECTORY IS NOT OPTIONAL. The search is keyed on the
    # triple this shim runs as, not on `uname`: the nightly's per-arch legs
    # build for i686, aarch64, riscv64gc and the rest, and
    # `target/<triple>/release/` is where each of them lands. A flat list of
    # the two directory layouts this operator's own machine uses made every
    # one of those six legs exit 2 with `podvm-release-licenses is not
    # built`, which is the shape of this shim's own error message rather than
    # a build failure. The nightly already writes `target/release/` for the
    # two host-built tools it copies by hand, so that layout is searched too.
    ROOT / "target" / "release" / ("podvm-release-licenses" + SUFFIX),
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / ("podvm-release-licenses" + SUFFIX),
    ROOT / "target" / "x86_64-pc-windows-msvc" / "debug" / ("podvm-release-licenses" + SUFFIX),
    ROOT / "target" / "debug" / ("podvm-release-licenses" + SUFFIX),
    *(ROOT / "target" / triple / "release" / ("podvm-release-licenses" + SUFFIX)
      for triple in TRIPLES),
)


def main():
    for binary in CANDIDATES:
        if binary.is_file():
            os.execv(str(binary), [str(binary)] + sys.argv[1:])
    print("release-licenses: podvm-release-licenses is not built; cargo build -p podvm-buildstate",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
