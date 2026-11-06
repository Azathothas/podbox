"""Build the authoritative per-script verdict ledger from the ten group reports.

Round 2 of the meta review settled that the group BODIES are authoritative and
the header tables are not. Six groups' headers disagree with their own bodies.

This reads the body verdict line for each script and pairs it with the script
path, handling both report shapes:
  - `**Verdict: RUST-TEST.** ...` under a script heading
  - `### Verdict: RUST-TOOL` as a heading itself (group 3)
"""

import os
import re

VERDICT = re.compile(r"(?:\*\*|^#{2,4} )Verdict:\s*(.+?)\s*\.?\s*(\*\*)?\s*$")
PATH = re.compile(r"(experiments|scripts)/[A-Za-z0-9._-]+")
NORMAL = re.compile(r"^(DELETE|RUST-TEST|RUST-TOOL|KEEP-SHELL|SPLIT)\b")


def normalise(text):
    m = NORMAL.match(text.strip())
    if m:
        return m.group(1)
    for name in ("KEEP-SHELL", "RUST-TOOL", "RUST-TEST", "DELETE", "SPLIT"):
        if text.strip().upper().startswith(name):
            return name
    return text.strip()[:60]


def read_group(n):
    """Return [(script_path, verdict, report_line)] for one group report."""
    path = "refactor/01-audit/group-%d.md" % n
    lines = open(path, "rb").read().decode("utf-8", "replace").splitlines()
    out = []
    pending = None
    for i, line in enumerate(lines):
        # A heading that names a script becomes the pending subject. Headings
        # that name no script leave the pending subject alone, so a `## Findings`
        # section does not steal the next verdict.
        if line.startswith("#"):
            names = PATH.findall(line)
            if names:
                pending = PATH.search(line).group(0)
        v = VERDICT.search(line)
        if v and pending:
            out.append((pending, normalise(v.group(1)), i + 1))
            pending = None
    return out


ledger = {}
for n in range(1, 11):
    for key, verdict, line in read_group(n):
        if key in ledger:
            print("DUP %s group-%d:%d already have group-%d:%d" % (
                key, n, line, ledger[key][0], ledger[key][2]))
            continue
        ledger[key] = (n, verdict, line)

# Every script the assignment map knows about must appear.
expected = set()
for d in ("experiments", "scripts"):
    for name in sorted(os.listdir(d)):
        if os.path.isfile(os.path.join(d, name)) and name.endswith((".sh", ".py", ".ps1")):
            expected.add(("%s/%s" % (d, name)).replace(os.sep, "/"))

print("expected scripts: %d" % len(expected))
print("ledger entries:   %d" % len(ledger))
missing = sorted(expected - set(ledger))
extra = sorted(set(ledger) - expected)
print("missing from ledger: %d" % len(missing))
for k in missing:
    print("  MISSING %s" % k)
print("not in the map: %d" % len(extra))
for k in extra:
    print("  EXTRA   %s" % k)

# Six of the ten reports mis-state a script's directory. A group report writes
# `experiments/verify-release.sh` for a file that lives in `scripts/`. Repair
# any key that is not a real path against the assignment map's directory.
for key in list(ledger):
    if key in expected:
        continue
    for d in ("scripts", "experiments"):
        if key.startswith("experiments/") and ("%s/%s" % (d, key.split("/", 1)[1])) in expected:
            fixed = "%s/%s" % (d, key.split("/", 1)[1])
            print("REPAIR %s -> %s" % (key, fixed))
            ledger[fixed] = ledger.pop(key)
            break

counts = {}
for _, (n, v, line) in sorted(ledger.items()):
    counts[v] = counts.get(v, 0) + 1
print("\nverdict counts: %s  total %d" % (counts, sum(counts.values())))

with open("refactor/06-entries/verdict-ledger.tsv", "w") as fh:
    fh.write("script\tgroup\tverdict\treport_line\n")
    for key, (n, v, line) in sorted(ledger.items()):
        fh.write("%s\t%d\t%s\tgroup-%d.md:%d\n" % (key, n, v, n, line))
print("wrote refactor/06-entries/verdict-ledger.tsv")