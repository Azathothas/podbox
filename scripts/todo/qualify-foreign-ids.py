#!/usr/bin/env python3
"""Write each id of another repository's record as REPO@T-NNN (T-1680).

podvm's ids have four digits. An id `T-NNN` of three digits in the record
is an id of a sibling's record: podcrates, podnet, podssh or podrunner. Its
repository is the one that its line names nearest before it, else after
it, else its paragraph before it, else after it; podcrates where none is
named. An id in a code span stays: it is a part of a command. pod-todo reads `REPO@T-NNN` as an id of another record.

Usage: qualify-foreign-ids.py [--dry-run] ROOT. It prints each change.
"""

import re
import sys
from pathlib import Path

REPOS = ["podcrates", "podnet", "podssh", "podrunner"]
ID = re.compile(r"(?<![A-Za-z0-9@-])T-(\d{3})(?![0-9A-Za-z])")
NAME = re.compile(r"\b(" + "|".join(REPOS) + r")\b", re.I)


def files(root):
    out = sorted((root / "TODO").glob("*.md"))
    out += sorted((root / "docs").glob("*.md"))
    return out


def main(argv):
    dry = "--dry-run" in argv
    root = Path([a for a in argv if a != "--dry-run"][0])
    total = 0
    for p in files(root):
        text = p.read_text()
        paras = re.split(r"(\n\s*\n)", text)
        out = []
        for para in paras:
            def sub(m):
                nonlocal total
                ls = para.rfind("\n", 0, m.start()) + 1
                le = para.find("\n", m.end())
                le = len(para) if le < 0 else le
                if para[ls:m.start()].count("`") % 2:
                    print(f"{p.relative_to(root)}: {m.group(0)} in a code span: kept", file=sys.stderr)
                    return m.group(0)
                same_before = [x.group(1).lower() for x in NAME.finditer(para, ls, m.start())]
                same_after = [x.group(1).lower() for x in NAME.finditer(para, m.end(), le)]
                before = [x.group(1).lower() for x in NAME.finditer(para, 0, m.start())]
                after = [x.group(1).lower() for x in NAME.finditer(para, m.end())]
                picks = (same_before[-1:] or same_after[:1] or before[-1:] or after[:1] or ["podcrates"])
                repo = picks[0]
                total += 1
                line = para[:m.start()].count("\n")
                print(f"{p.relative_to(root)}: {m.group(0)} -> {repo}@{m.group(0)}")
                return f"{repo}@{m.group(0)}"
            out.append(ID.sub(sub, para))
        new = "".join(out)
        if new != text and not dry:
            p.write_text(new)
    print(f"qualify-foreign-ids: {total} ids")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
