#!/usr/bin/env python3
"""Check the five exported executables against their build record. T-1350."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path)
    options = parser.parse_args()
    lines = ["== conditions", datetime.now(timezone.utc).isoformat(),
             f"host={platform.system()} python={platform.python_version()}",
             "Inputs: five exports and podbox-build/1 record from the default Windows job.",
             "Scope: output integrity; this does not compare source or toolchain conditions."]
    code = 0
    try:
        record = json.loads((options.directory / "build-state.json").read_text(encoding="utf-8"))
        names = {"podbox", "node", "operator", "proxy", "shell"}
        if not isinstance(record, dict) or record.get("schema") != "podbox-build/1" or set(record.get("outputs", {})) != names:
            raise ValueError("build record schema or executable set differs")
        lines.append("recorded input digest=" + record["inputs"])
        for name in sorted(names):
            data = (options.directory / name).read_bytes()
            digest = hashlib.sha256(data).hexdigest()
            matched = digest == record["outputs"][name]
            lines.append(f"{name}: bytes={len(data)} sha256={digest} matched={matched}")
            if not matched:
                code = 1
    except (ValueError, KeyError, TypeError) as error:
        lines.append("FAIL: " + str(error))
        code = 1
    except OSError as error:
        lines.append("cannot run: " + error.strerror)
        code = 2
    lines.append(f"proof exit: {code}")
    report = "\n".join(lines) + "\n"
    if options.output:
        options.output.write_text(report, encoding="utf-8", newline="\n")
    print(report, end="")
    return code


if __name__ == "__main__":
    sys.exit(main())
