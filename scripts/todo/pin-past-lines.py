#!/usr/bin/env python3
"""Pin each line number of a past tree to its commit (T-1680).

`podvm-todo check` reports a bare `:N` past the end of the file that it
follows, and a plain "line N" after a citation: each is a reading of the
tree of its time. Each becomes "line N at `COMMIT`" (or "lines A-B at
`COMMIT`"), COMMIT the oldest commit whose text of the record file holds
the line. pod-todo reads such a number as a reading of a past tree.

Usage: pin-past-lines.py ROOT. It prints each change.
"""

import re
import subprocess
import sys
from pathlib import Path


def run(root, *args):
    r = subprocess.run(args, cwd=root, capture_output=True, text=True)
    return r.stdout + r.stderr


def commit_of(root, rel, snippet):
    out = run(root, "git", "log", "--format=%h", "--abbrev=12", "-S", snippet, "--", rel).split()
    return out[-1] if out else None


def main(argv):
    root = Path(argv[0])
    report = run(root, "./target/x86_64-unknown-linux-musl/release/podvm-todo", "check")
    items = set()
    for line in report.splitlines():
        m = re.match(r"^(\S+?):(\d+): `(:\d+(?:-\d+)?)` after `", line)
        if m:
            items.add((m.group(1), int(m.group(2)), "bare", m.group(3)))
            continue
        m = re.match(r'^(\S+?):(\d+): "(lines? [\d-]+)" follows a citation', line)
        if m:
            items.add((m.group(1), int(m.group(2)), "plain", m.group(3)))
    done = 0
    for rel, n, kind, tok in sorted(items):
        p = root / rel
        lines = p.read_text().split("\n")
        text = lines[n - 1]
        snippet = text.strip()[:60]
        c = commit_of(root, rel, snippet)
        if not c:
            print(f"{rel}:{n}: no commit holds {snippet!r}", file=sys.stderr)
            continue
        if kind == "bare":
            nums = tok[1:]
            word = "lines" if "-" in nums else "line"
            new = text.replace(f"`{tok}`", f"{word} {nums} at `{c}`")
        else:
            new = re.sub(re.escape(tok) + r"(?! at `)", f"{tok} at `{c}`", text, count=1)
        if new == text:
            print(f"{rel}:{n}: {tok} not found on the line", file=sys.stderr)
            continue
        lines[n - 1] = new
        p.write_text("\n".join(lines))
        done += 1
        print(f"{rel}:{n}: {tok} -> at {c}")
    print(f"pin-past-lines: {done} numbers pinned")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
