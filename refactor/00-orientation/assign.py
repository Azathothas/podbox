import os

SEP = chr(92)
rows = []
for d in ("experiments", "scripts"):
    for name in sorted(os.listdir(d)):
        p = os.path.join(d, name)
        if os.path.isfile(p) and name.endswith((".sh", ".py", ".ps1")):
            rows.append((p.replace(SEP, "/"), sum(1 for _ in open(p, "rb"))))
rows.sort()
total = sum(n for _, n in rows)
G = 10
groups = [[] for _ in range(G)]
load = [0] * G
for p, n in sorted(rows, key=lambda x: -x[1]):
    i = load.index(min(load))
    groups[i].append((p, n))
    load[i] += n

lines = [
    "# Assignment map: group -> scripts",
    "",
    "Total scripts: %d, total lines: %d, groups: %d" % (len(rows), total, G),
    "",
]
for i, g in enumerate(groups, 1):
    lines.append("## Group %d (%d scripts, %d lines)" % (i, len(g), load[i - 1]))
    for p, n in sorted(g):
        lines.append("  %5d  %s" % (n, p))
    lines.append("")
open("refactor/00-orientation/assignment-map.md", "w").write("\n".join(lines))
print("wrote assignment map")
for i, g in enumerate(groups, 1):
    print(i, len(g), load[i - 1])