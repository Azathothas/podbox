#!/usr/bin/env python3
"""Does the freshness check reject changed inputs and outputs? T-1349."""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
from datetime import datetime, timezone
import platform

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("build_state", ROOT / "scripts/build-state.py")
build_state = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build_state)


def main():
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
        binary = root / "podbox"
        binary.write_bytes(b"fixture-output\n")
        for name in build_state.OUTPUT_NAMES[1:]:
            (root / name).write_bytes(b'fixture-output\n')
        record = root / ".dev/build-state.json"
        record.parent.mkdir()
        record.write_text(json.dumps({
            "schema": build_state.SCHEMA,
            "inputs": build_state.inputs(root, "fixture-target", tools=False),
            "outputs": build_state.outputs(binary),
        }), encoding="utf-8")
        assert build_state.state(root, "fixture-target", binary, tools=False) == ("ready", 0)
        print("ok: unchanged fixture is ready")
        for name in files:
            path = root / name
            before = path.stat()
            original = path.read_bytes()
            path.write_bytes(b"fixture-v2\n")
            os.utime(path, ns=(before.st_atime_ns, before.st_mtime_ns))
            assert build_state.state(root, "fixture-target", binary, tools=False)[1] == 1, name
            path.write_bytes(original)
            print(f"ok: changed bytes with the same timestamp: {name}")
        binary.write_bytes(b"different-output\n")
        assert build_state.state(root, "fixture-target", binary, tools=False)[1] == 1
        print("ok: changed output bytes are stale")
        binary.write_bytes(b'fixture-output\n')
        for name in build_state.OUTPUT_NAMES[1:]:
            helper = root / name
            helper.write_bytes(b'changed-helper\n')
            assert build_state.state(root, 'fixture-target', binary, tools=False)[1] == 1
            helper.write_bytes(b'fixture-output\n')
            print(f'ok: changed helper bytes are stale: {name}')
        binary.unlink()
        assert build_state.state(root, "fixture-target", binary, tools=False)[1] == 2
        print("ok: missing output is absent")
        print("verdict BUILD-FRESHNESS-OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
