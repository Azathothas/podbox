#!/usr/bin/env python3
"""Record build inputs and verify the output bytes. See TODO/gate.md T-1349."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

SCHEMA = "podbox-build/1"
DEFAULT_ROOT = Path(__file__).resolve().parent.parent
OUTPUT_NAMES = ('podbox', 'node', 'operator', 'proxy', 'shell')


def build_commit(root):
    if 'PODBOX_BUILD_COMMIT' in os.environ:
        return os.environ['PODBOX_BUILD_COMMIT']
    head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root,
        capture_output=True, text=True, encoding='utf-8', timeout=10)
    if head.returncode:
        return 'unknown'
    dirty = subprocess.run(['git', 'status', '--porcelain'], cwd=root,
        capture_output=True, text=True, encoding='utf-8', timeout=10, check=True)
    return head.stdout.strip() + ('-dirty' if dirty.stdout.strip() else '')


def outputs(binary):
    return {name: digest(binary if name == 'podbox' else binary.parent / name)
            for name in OUTPUT_NAMES}


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inputs(root, target, tools=True):
    """Hash names and bytes. File timestamps do not establish freshness."""
    paths = set()
    for name in ("crates", "vendor", "scripts", ".cargo"):
        base = root / name
        if base.exists():
            for path in base.rglob("*"):
                relative = path.relative_to(root)
                if path.is_file() and not any(
                    part in ("target", "__pycache__") for part in relative.parts
                ):
                    paths.add(relative.as_posix())
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        if (root / name).is_file():
            paths.add(name)
    # These bytes are inputs to the CLI build script.
    for libc in ("gnu", "musl"):
        name = (
            "crates/podbox-interpose/target/x86_64-unknown-linux-"
            f"{libc}/release/libpodbox_interpose.so"
        )
        if (root / name).is_file():
            paths.add(name)
    values = {name: digest(root / name) for name in sorted(paths)}
    conditions = {"target": target}
    keys = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CC", "AR", "TARGETS"}
    keys.update(key for key in os.environ if key.startswith(("CARGO_TARGET_", "CC_", "AR_")))
    for key in sorted(keys):
        conditions[key] = os.environ.get(key, "")
    if tools:
        conditions['build_commit'] = build_commit(root)
        for tool in ("rustc", "cargo", "zig"):
            result = subprocess.run(
                [tool, "version" if tool == "zig" else "--version"],
                capture_output=True, text=True, timeout=10, check=True,
            )
            conditions[tool] = result.stdout.strip()
    data = json.dumps([values, conditions], sort_keys=True).encode()
    return hashlib.sha256(data).hexdigest()


def state(root, target, binary, tools=True):
    if not binary.is_file():
        return "absent", 2
    record = root / ".dev/build-state.json"
    if not record.is_file():
        return "stale: no build record", 1
    try:
        saved = json.loads(record.read_text(encoding="utf-8"))
    except (ValueError, OSError):
        return "stale: unreadable build record", 1
    if not isinstance(saved, dict) or saved.get("schema") != SCHEMA:
        return "stale: build record schema differs", 1
    if saved.get("inputs") != inputs(root, target, tools):
        return "stale: build inputs changed", 1
    try:
        current_outputs = outputs(binary)
    except FileNotFoundError:
        return 'absent: a workspace executable is missing', 2
    if saved.get("outputs") != current_outputs:
        return "stale: binary bytes changed", 1
    return "ready", 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("inputs", "record", "status", "commit"))
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--target", default="x86_64-unknown-linux-musl")
    parser.add_argument("--binary", type=Path)
    options = parser.parse_args()
    root = options.root.resolve()
    binary = options.binary or root / "target" / options.target / "release/podbox"
    try:
        if options.action == 'commit':
            print(build_commit(root))
        elif options.action == "inputs":
            print(inputs(root, options.target))
        elif options.action == "record":
            result = {
                "schema": SCHEMA, "inputs": inputs(root, options.target),
                "outputs": outputs(binary),
            }
            destination = root / ".dev/build-state.json"
            destination.parent.mkdir(parents=True, exist_ok=True)
            temporary = destination.with_suffix(".new")
            temporary.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
            temporary.replace(destination)
        else:
            message, code = state(root, options.target, binary)
            print(message)
            return code
    except (OSError, subprocess.SubprocessError) as error:
        print(f"build-state: cannot measure the build: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
