#!/usr/bin/env python3
"""Does the freshness check reject changed inputs and outputs? T-1349, driver
moved to the podbox-buildstate binary in T-1559. The fixture, the verdicts,
and the exit codes are unchanged; only the driver changed, from a module
import to the shipped CLI, so this run proves the binary, not a shim."""

import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent.parent
CANDIDATES = (
    ROOT / "target" / "release" / "podbox-buildstate",
    ROOT / "target" / "x86_64-unknown-linux-musl" / "release" / "podbox-buildstate",
    ROOT / "target" / "debug" / "podbox-buildstate",
)
OUTPUT_NAMES = ("podbox", "node", "operator", "proxy", "shell")


def binary():
    for candidate in CANDIDATES:
        if candidate.is_file():
            return candidate
    print("SKIP: podbox-buildstate is not built; cargo build -p podbox-buildstate",
          file=sys.stderr)
    raise SystemExit(2)


def run(binary_path, *arguments):
    return subprocess.run([str(binary_path)] + list(arguments), capture_output=True,
                          text=True, encoding="utf-8", timeout=120)


def main():
    binary_path = binary()
    print("== conditions")
    print(datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))
    print(f"host: {platform.system()} {platform.machine()}; Python {platform.python_version()}")
    print("inputs: fixed fixture-v1/fixture-v2 bytes; saved timestamps restored")
    print("scope: source and output classification; no runtime execution")
    with tempfile.TemporaryDirectory(prefix="podbox-freshness-") as directory:
        root = Path(directory)
        files = (
            "crates/podbox-cli/src/main.rs", "crates/podbox-cli/Cargo.toml",
            "crates/podbox-ssh/shims/fakepwd.c", "scripts/build-interpose.sh",
            "vendor/userland-execve/Cargo.toml", "Cargo.lock",
            "crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so",
        )
        for name in files:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"fixture-v1\n")
        exe = root / "podbox"
        exe.write_bytes(b"fixture-output\n")
        for name in OUTPUT_NAMES[1:]:
            (root / name).write_bytes(b"fixture-output\n")
        record = run(binary_path, "record", "--root", str(root),
                     "--target", "fixture-target", "--binary", str(exe))
        assert record.returncode == 0, record.stderr
        saved = json.loads((root / ".dev" / "build-state.json").read_text(encoding="utf-8"))
        assert saved["schema"] == "podbox-build/1", saved
        assert set(saved["outputs"]) == set(OUTPUT_NAMES), saved["outputs"]
        state = run(binary_path, "status", "--root", str(root),
                    "--target", "fixture-target", "--binary", str(exe))
        assert (state.stdout.strip(), state.returncode) == ("ready", 0), (state.stdout, state.stderr)
        print("ok: unchanged fixture is ready")
        for name in files:
            path = root / name
            before = path.stat()
            original = path.read_bytes()
            path.write_bytes(b"fixture-v2\n")
            os.utime(path, ns=(before.st_atime_ns, before.st_mtime_ns))
            state = run(binary_path, "status", "--root", str(root),
                        "--target", "fixture-target", "--binary", str(exe))
            assert state.returncode == 1, name
            path.write_bytes(original)
            print(f"ok: changed bytes with the same timestamp: {name}")
        exe.write_bytes(b"different-output\n")
        state = run(binary_path, "status", "--root", str(root),
                    "--target", "fixture-target", "--binary", str(exe))
        assert state.returncode == 1, (state.stdout, state.stderr)
        print("ok: changed output bytes are stale")
        exe.write_bytes(b"fixture-output\n")
        for name in OUTPUT_NAMES[1:]:
            helper = root / name
            helper.write_bytes(b"changed-helper\n")
            state = run(binary_path, "status", "--root", str(root),
                        "--target", "fixture-target", "--binary", str(exe))
            assert state.returncode == 1, name
            helper.write_bytes(b"fixture-output\n")
            print(f"ok: changed helper bytes are stale: {name}")
        exe.unlink()
        state = run(binary_path, "status", "--root", str(root),
                    "--target", "fixture-target", "--binary", str(exe))
        assert state.returncode == 2, (state.stdout, state.stderr)
        print("ok: missing output is absent")
        print("verdict BUILD-FRESHNESS-OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
