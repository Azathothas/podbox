#!/usr/bin/env python3
"""Move the record of TODO/ to the format of podcrates' pod-todo (T-1672).

The old entry: `### T-NNNN Title`, ten fields `Name:` with continuation
lines indented, then dated paragraphs (`**Done 2026-11-11.** ...`). The new
entry: `# T-NNNN: Title`, the header fields `**Name:** value` (Source,
Category, Milestone, Priority, Effort, Status), then `## Problem`,
`## Premise`, `## Approach`, `## Decision`, `## Prove` and, by the lead word
of each dated paragraph, `## Blocker`, `## Correction` and `## Done`.

The rules of the move (docs/decisions.md, "The record in the format of
pod-todo"):
* Milestone is `none`: podvm's milestones are entries of their own.
* The date of a `Status:` line goes; each done or partial entry keeps a date
  in its `## Done`.
* Effort XL becomes L, and `## Correction` says so: pod-todo has no XL.
* A paragraph led by Done or Partial goes to `## Done` where the status is
  done or partial; a paragraph led by Blocked goes to `## Blocker` where the
  status is blocked; each other dated paragraph goes to `## Correction`. A
  paragraph with no lead word of its own joins the paragraph before it.
* An unindented paragraph between two fields joins the field before it.
* The H1 of an area file becomes a line `Area: NAME.`, because each H1 of
  an area file is an entry for pod-todo.
* The separators `---` between entries go.
* The status cell of a row loses its bold, and the H1 of RESUME.md and
  reference-map.md becomes a bold line.

Usage: convert-to-pod-todo.py [--check] [ROOT]. Without --check, the files
are rewritten. Each run parses the old record and the new text and compares
them field by field (whitespace folded); it prints the count of fields
lost and exits 1 when it is not 0.
"""

import re
import sys
from pathlib import Path

FIELDS = ["Source", "Category", "Priority", "Effort", "Status",
          "Problem", "Premise", "Approach", "Decision", "Prove"]
HEADER = ["Source", "Category", "Milestone", "Priority", "Effort", "Status"]
BODY = ["Problem", "Premise", "Approach", "Decision", "Prove"]
SECTIONS = ["Problem", "Premise", "Approach", "Decision", "Prove",
            "Blocker", "Start condition", "Correction", "Done"]
NOT_AREAS = {"INDEX.md", "PROGRESS.md", "RULES.md", "RESUME.md", "issues.md",
             "reference-map.md"}
FIELD_RE = re.compile(r"^(" + "|".join(FIELDS) + r"):( +)(.*)$")
HEAD_RE = re.compile(r"^### (T-\d{4}) (.*)$")
LEAD_RE = re.compile(r"^\*\*([A-Z][A-Za-z-]*)")
DATE_RE = re.compile(r"\d{4}-\d{2}-\d{2}")
DONE_LEADS = {"Done", "Partial", "Closed"}
DATED_LEADS = {"Done", "Partial", "Closed", "Blocked", "Unblocked", "Filed",
               "Amended", "Open", "Reopened", "Ruling", "Recorded", "Measured",
               "Decided", "Settled", "Corrected", "Disproved"}


def fold(text):
    return " ".join(text.split())


def parse_old(lines):
    """The fields and the dated paragraphs of one old entry (its lines after
    the heading)."""
    fields = {}
    order = []
    cur = None
    trailer = []
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        m = FIELD_RE.match(line)
        if m and not trailer:
            cur = m.group(1)
            width = len(cur) + 1 + len(m.group(2))
            if cur in fields:
                fields[cur]["lines"].append("")
                fields[cur]["lines"].append(m.group(3))
            else:
                fields[cur] = {"width": width, "lines": [m.group(3)]}
                order.append(cur)
            i += 1
            continue
        if trailer:
            trailer.append(line)
            i += 1
            continue
        if cur is None:
            if line.strip():
                raise ValueError(f"text before the first field: {line!r}")
            i += 1
            continue
        if not line.strip() or line[0] in " \t":
            width = fields[cur]["width"]
            lead = len(line) - len(line.lstrip(" "))
            fields[cur]["lines"].append(line[min(width, lead):])
            i += 1
            continue
        # An unindented line: inside the fields where a field that is not
        # given yet follows it (a trailer can hold a line `Prove: ...`).
        later = {m.group(1) for m in map(FIELD_RE.match, lines[i + 1:]) if m}
        if later - set(fields):
            fields[cur]["lines"].append(line)
            i += 1
            continue
        trailer.append(line)
        i += 1
    for f in fields.values():
        while f["lines"] and not f["lines"][-1].strip():
            f["lines"].pop()
    while trailer and (not trailer[-1].strip() or trailer[-1].strip() == "---"):
        trailer.pop()
    return fields, order, trailer


def paragraphs(trailer):
    """The dated paragraphs: (lead word or None, lines)."""
    chunks = []
    cur = None
    prev_blank = True
    fence = False
    for line in trailer:
        if line.lstrip().startswith("```"):
            fence = not fence
        m = LEAD_RE.match(line) if not fence else None
        if m and prev_blank and (m.group(1) in DATED_LEADS or cur is None):
            cur = [m.group(1) if m.group(1) in DATED_LEADS else None, [line]]
            chunks.append(cur)
        elif cur is None:
            cur = [None, [line]]
            chunks.append(cur)
        else:
            cur[1].append(line)
        prev_blank = not line.strip()
    for c in chunks:
        while c[1] and not c[1][-1].strip():
            c[1].pop()
    return [c for c in chunks if c[1]]


def convert_entry(tid, title, lines, problems):
    fields, _order, trailer = parse_old(lines)
    for f in FIELDS:
        if f not in fields:
            problems.append(f"{tid}: no field {f}")
            fields[f] = {"width": 13, "lines": ["-"]}
    status_line = fold("\n".join(fields["Status"]["lines"]))
    status = status_line.split(" ")[0].strip("*")
    effort = fold("\n".join(fields["Effort"]["lines"]))
    notes = []
    if effort == "XL":
        effort = "L"
        notes.append(f"The effort of {tid} was XL before call C17 of "
                     "`docs/decisions.md`; pod-todo has no XL.")
    sections = {s: [] for s in SECTIONS}
    last = "Correction"
    for lead, body in paragraphs(trailer):
        if lead in DONE_LEADS and status in ("done", "partial"):
            last = "Done"
        elif lead == "Blocked" and status == "blocked":
            last = "Blocker"
        elif lead is not None:
            last = "Correction"
        if sections[last]:
            sections[last].append("")
        sections[last].extend(body)
    if notes:
        if sections["Correction"]:
            sections["Correction"].append("")
        sections["Correction"].extend(notes)
    date = DATE_RE.search(status_line)
    synthetic = []
    if status in ("done", "partial") and date and date.group(0) not in "\n".join(sections["Done"]):
        lead = "Done" if status == "done" else "Partial"
        line = f"**{lead} {date.group(0)}.** The date of the status line of the old record."
        synthetic.append(line)
        sections["Done"] = [line] + ([""] + sections["Done"] if sections["Done"] else [])
    for b in BODY:
        sections[b] = fields[b]["lines"]
    head = {
        "Source": fields["Source"]["lines"],
        "Category": fields["Category"]["lines"],
        "Milestone": ["none"],
        "Priority": fields["Priority"]["lines"],
        "Effort": [effort],
        "Status": [status],
    }
    out = [f"# {tid}: {title}", ""]
    for h in HEADER:
        v = head[h]
        out.append(f"**{h}:** {v[0].strip()}".rstrip())
        out.extend("  " + x.strip() for x in v[1:] if x.strip())
    for s in SECTIONS:
        if not sections[s]:
            continue
        out.extend(["", f"## {s}", ""])
        out.extend(sections[s])
    out.append("")
    sections["_old"] = [fold("\n".join(body)) for _lead, body in paragraphs(trailer)]
    sections["_synthetic"] = synthetic + notes
    return out, fields, status, effort, sections


def parse_new(text):
    """The entries of a new area file, read as pod-todo reads them."""
    entries = {}
    cur = None
    fence = False
    in_header = False
    for line in text.split("\n"):
        if line.lstrip().startswith("```"):
            fence = not fence
        if not fence and line.startswith("# "):
            tid, _, title = line[2:].partition(": ")
            cur = {"title": title, "fields": {}, "sections": {}, "order": []}
            entries[tid] = cur
            in_header = True
            last = None
            continue
        if cur is None:
            continue
        if not fence and line.startswith("## "):
            name = line[3:].strip()
            cur["sections"][name] = []
            cur["order"].append(name)
            in_header = False
            continue
        if in_header:
            m = re.match(r"^\*\*([A-Za-z ]+):\*\* ?(.*)$", line)
            if m:
                cur["fields"][m.group(1)] = m.group(2)
                last = m.group(1)
            elif line.strip() and last:
                cur["fields"][last] += " " + line.strip()
        elif cur["order"]:
            cur["sections"][cur["order"][-1]].append(line)
    return entries


def convert_area(text, problems):
    lines = text.split("\n")
    heads = [i for i, l in enumerate(lines) if HEAD_RE.match(l)]
    if not heads:
        return text, {}
    pre = lines[:heads[0]]
    while pre and pre[-1].strip() in ("", "---"):
        pre.pop()
    out = []
    for line in pre:
        if line.startswith("# "):
            out.append(f"Area: {line[2:].strip()}.")
        else:
            out.append(line)
    out.append("")
    old = {}
    for k, h in enumerate(heads):
        end = heads[k + 1] if k + 1 < len(heads) else len(lines)
        m = HEAD_RE.match(lines[h])
        tid, title = m.group(1), m.group(2)
        body = lines[h + 1:end]
        new, fields, status, effort, sections = convert_entry(tid, title, body, problems)
        old[tid] = (title, fields, status, effort, sections)
        out.extend(new)
    return "\n".join(out).rstrip("\n") + "\n", old


def compare(old, new_text, problems):
    """Each old field is in the new entry, whitespace folded. The count of
    fields lost."""
    new = parse_new(new_text)
    lost = 0
    for tid, (title, fields, status, effort, sections) in old.items():
        e = new.get(tid)
        if e is None:
            problems.append(f"{tid}: the entry is gone")
            lost += len(FIELDS)
            continue
        if e["title"] != title:
            problems.append(f"{tid}: the title changed")
            lost += 1
        for f in ("Source", "Category", "Priority"):
            if fold(e["fields"].get(f, "")) != fold("\n".join(fields[f]["lines"])):
                problems.append(f"{tid}: {f} changed")
                lost += 1
        if e["fields"].get("Status") != status:
            problems.append(f"{tid}: Status changed")
            lost += 1
        old_effort = fold("\n".join(fields["Effort"]["lines"]))
        if e["fields"].get("Effort") != effort or (old_effort != effort and "was XL" not in fold("\n".join(e["sections"].get("Correction", [])))):
            problems.append(f"{tid}: Effort changed with no note")
            lost += 1
        date = DATE_RE.search(fold("\n".join(fields["Status"]["lines"])))
        if date and date.group(0) not in "\n".join(sum(e["sections"].values(), [])):
            problems.append(f"{tid}: the date of the status is gone")
            lost += 1
        for b in BODY:
            if fold("\n".join(e["sections"].get(b, []))) != fold("\n".join(fields[b]["lines"])):
                problems.append(f"{tid}: {b} changed")
                lost += 1
        trailer_new = fold("\n".join(sum((e["sections"].get(s, []) for s in ("Blocker", "Correction", "Done")), [])))
        for para in sections["_old"]:
            if para not in trailer_new:
                problems.append(f"{tid}: a dated paragraph is gone: {para[:60]!r}")
                lost += 1
        # Nothing new but the notes of the move.
        extra = trailer_new
        for para in sections["_old"] + [fold(x) for x in sections["_synthetic"]]:
            extra = extra.replace(para, "", 1)
        if fold(extra):
            problems.append(f"{tid}: text that the old entry did not hold: {fold(extra)[:60]!r}")
            lost += 1
    return lost


def convert_index(text, efforts):
    out = []
    for line in text.split("\n"):
        m = re.match(r"^\| \[(T-\d{4})\]\(([^)]+)\) \| (P\d) \| ([^|]+) \| ([^|]+) \| (.*) \|$", line)
        if m:
            tid = m.group(1)
            out.append(f"| [{tid}]({m.group(2)}) | {m.group(3)} | {efforts[tid]} | none | "
                       f"{m.group(4).strip()} | {m.group(5).strip().strip('*')} | {m.group(6)} |")
            continue
        out.append(line)
    return "\n".join(out)


CITE_RE = re.compile(r"(TODO/([a-z-]+\.md)):(\d+)(?:-(\d+))?")


def key(line):
    """The text of a line without the form of its field."""
    line = line.strip()
    m = FIELD_RE.match(line)
    if m:
        return m.group(3).strip()
    m = re.match(r"^\*\*(?:Source|Category|Milestone|Priority|Effort|Status):\*\* ?(.*)$", line)
    if m:
        return m.group(1).strip()
    m = HEAD_RE.match(line) or re.match(r"^# (T-\d{4}): (.*)$", line)
    if m:
        return m.group(1) + " " + m.group(2)
    return line


def entry_at(lines, n, heading):
    """The id of the entry that holds line n (1-based), or None."""
    tid = None
    for i in range(min(n, len(lines))):
        m = heading.match(lines[i])
        if m:
            tid = m.group(1)
    return tid


def new_line(old_lines, new_lines, n):
    """The line of new_lines that holds line n of old_lines, by its text in
    the same entry; None where it is gone."""
    if not 1 <= n <= len(old_lines):
        return None
    # A blank line (a citation that was stale before the move) goes with the
    # nearest line of text above it.
    back = 0
    while n - back > 1 and not old_lines[n - 1 - back].strip():
        back += 1
    if back:
        at = new_line(old_lines, new_lines, n - back)
        return None if at is None else at + back
    want = key(old_lines[n - 1])
    tid = entry_at(old_lines, n, HEAD_RE)
    new_head = re.compile(r"^# (T-\d{4}): ")
    best = None
    for i, l in enumerate(new_lines):
        if key(l) == want and want and entry_at(new_lines, i + 1, new_head) == tid:
            if best is None or abs(i + 1 - n) < abs(best - n):
                best = i + 1
    return best


def remap(root, olds, writes, problems):
    """Move each citation `TODO/FILE.md:N` of the tree to the new line of N.
    The files of docs/history/ are readings of their time and stay."""
    import subprocess
    files = subprocess.run(["git", "-C", str(root), "grep", "-lE", r"TODO/[a-z-]+\.md:[0-9]+"],
                           capture_output=True, text=True).stdout.split()
    moved = 0
    for rel in files:
        if rel.startswith("docs/history/"):
            continue
        p = root / rel
        text = writes.get(p, p.read_text())

        def sub(m):
            nonlocal moved
            name = m.group(2)
            old = olds.get(name)
            new = writes.get(root / "TODO" / name)
            if old is None or new is None:
                return m.group(0)
            ol, nl = old.split("\n"), new.split("\n")
            a = new_line(ol, nl, int(m.group(3)))
            b = new_line(ol, nl, int(m.group(4))) if m.group(4) else None
            if a is None or (m.group(4) and b is None):
                problems.append(f"{rel}: {m.group(0)} names a line that the move has no place for")
                return m.group(0)
            moved += 1
            return f"{m.group(1)}:{a}" + (f"-{b}" if b else "")

        new_text = CITE_RE.sub(sub, text)
        if new_text != text:
            writes[p] = new_text
    return moved


def main(argv):
    check = "--check" in argv
    args = [a for a in argv if a != "--check"]
    root = Path(args[0] if args else ".")
    todo = root / "TODO"
    problems = []
    lost = 0
    efforts = {}
    count = 0
    writes = {}
    olds = {}
    for p in sorted(todo.glob("*.md")):
        if p.name in NOT_AREAS:
            continue
        text = p.read_text()
        new, old = convert_area(text, problems)
        if not old:
            continue
        lost += compare(old, new, problems)
        olds[p.name] = text
        for tid, v in old.items():
            efforts[tid] = v[3]
        count += len(old)
        writes[p] = new
    # The H1 of a page of TODO/ that holds no entry: pod-todo reads each H1
    # of an area file as an entry.
    for name in ("RESUME.md", "reference-map.md"):
        p = todo / name
        if p.is_file():
            t = p.read_text()
            if t.startswith("# "):
                first, _, rest = t.partition("\n")
                writes[p] = f"**{first[2:].strip()}**\n{rest}"
    if count == 0:
        print("convert-to-pod-todo: 0 entries in the old format: the record is in the format of pod-todo")
        return 0
    index = todo / "INDEX.md"
    writes[index] = convert_index(index.read_text(), efforts)
    moved = remap(root, olds, writes, problems)
    for p in problems:
        print(p, file=sys.stderr)
    print(f"convert-to-pod-todo: {count} entries, {lost} fields lost, {moved} citations moved")
    if lost or problems:
        return 1
    if not check:
        for p, t in writes.items():
            p.write_text(t)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
