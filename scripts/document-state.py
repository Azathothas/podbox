#!/usr/bin/env python3
"""Generate the source state page. T-1348. This does not prove runtime behavior."""

import argparse
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
DESTINATION = ROOT / "docs/runtime-state.md"


def enumeration(path, name):
    text = (ROOT / path).read_text(encoding="utf-8")
    match = re.search(r"pub enum " + re.escape(name) + r"\s*\{(.*?)\n\}", text, re.S)
    if not match:
        raise ValueError(f"cannot read {name} from {path}")
    values = re.findall(r"^\s*([A-Z][A-Za-z0-9_]*)\s*,", match.group(1), re.M)
    if not values:
        raise ValueError(f"no values for {name}")
    return values


def render():
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    workspace = manifest["workspace"]
    registry = set()
    for member in workspace["members"]:
        crate = tomllib.loads((ROOT / member / "Cargo.toml").read_text(encoding="utf-8"))
        for name, definition in crate.get("dependencies", {}).items():
            if isinstance(definition, str) or "path" not in definition:
                registry.add(name)
    lines = [
        "# Source state", "",
        "This page is generated from the tracked manifests and source declarations.",
        "It records the build surface. Runtime proof and limits live in",
        "[PROGRESS](../TODO/PROGRESS.md) and [Limits](limits.md).", "",
        "Regenerate with `py scripts/document-state.py --write` on Windows.",
        "Use `python3` on Linux. The record gate rejects a changed snapshot.", "",
        "## Workspace", "",
        f"Declared version: `{workspace['package']['version']}`.", "",
        "| Member | Manifest |", "| --- | --- |",
    ]
    for member in workspace["members"]:
        lines.append(f"| `{member.rsplit('/', 1)[-1]}` | [{member}/Cargo.toml](../{member}/Cargo.toml) |")
    lines += ["", "Separate crate: `crates/podbox-interpose`.", "", "## Mechanism declarations", ""]
    for path, name in (
        ("crates/podbox-probe/src/select.rs", "Rung"),
        ("crates/podbox-cli/src/tier.rs", "Tier"),
        ("crates/podbox-enter/src/ladder.rs", "Mode"),
    ):
        lines.append(f"`{name}` in [{path}](../{path}): " + ", ".join(f"`{value}`" for value in enumeration(path, name)) + ".")
        lines.append("")
    lines += ["## SSH helper sources", "", "| Executable | Source |", "| --- | --- |"]
    for source in sorted((ROOT / "crates/podbox-ssh/src/bin").glob("*.rs")):
        path = source.relative_to(ROOT).as_posix()
        lines.append(f"| `{source.stem}` | [{path}](../{path}) |")
    lines += ["", "## Declared direct registry dependencies", "", "| Dependency | Locked versions |", "| --- | --- |"]
    for name in sorted(registry):
        versions = sorted({p["version"] for p in lock["package"] if p["name"] == name and "source" in p})
        if not versions:
            raise ValueError(f"no registry lock entry for {name}")
        lines.append(f"| `{name}` | " + ", ".join(f"`{version}`" for version in versions) + " |")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    options = parser.parse_args()
    try:
        expected = render()
        if options.write:
            DESTINATION.write_text(expected, encoding="utf-8", newline="\n")
            print("document-state: wrote docs/runtime-state.md")
        elif not DESTINATION.is_file() or DESTINATION.read_text(encoding="utf-8") != expected:
            print("document-state: source state differs; regenerate docs/runtime-state.md", file=sys.stderr)
            return 1
        else:
            print("document-state: ok")
    except (OSError, ValueError, KeyError) as error:
        print(f"document-state: cannot read source state: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
