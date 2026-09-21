#!/usr/bin/env python3
"""check-todo.py - the reader half of docs/methodology/work-todo.md's two scripts.

⛔ THIS IS THE GATE. It asserts, independently of the writer, that:

  1. every row in TODO/INDEX.md names an entry that exists, in the file the row
     links to, under a heading `### T-NNNN <title>`;
  2. every entry has a row;
  3. the status in the row and the `Status:` line in the entry agree;
  4. every count in INDEX.md's Counts block is what the rows actually say;
  5. every entry carries all ten fields authoring.md requires, and its `Prove`
     is a command;
  6. every reference named in TODO/reference-map.md resolves to a directory
     under references/ that has a PROVENANCE.md;
  7. every cited `path:line` in TODO/ resolves: the file exists and has at
     least that many lines;
  8. every relative markdown link from a TODO/ file resolves;
  9. every `T-NNNN` mentioned anywhere in TODO/ names an entry that exists;
 10. PROGRESS.md's count line is what the rows say. A file that quotes a number
     another file measures has to be checkable, so it is written as a fixed line
     this script parses rather than as prose that reads better;
 11. every cited `path:line` ANYWHERE this project wrote, not only under TODO/,
     resolves. The first defect this repository shipped was a README and eight
     crate doc comments naming a directory that did not exist, and checks 7 and
     8 could not see any of it because they read TODO/ alone;
 12. every cited path is TRACKED BY GIT, not merely present on this disk. A
     check that asks the filesystem agrees with whoever ran it last and
     disagrees with a fresh clone, which is the worst place for a disagreement
     to sit: the person who could fix it is the one being told nothing is wrong;
 13. the dangling links inside the verbatim methodology copy are exactly the
     recorded set. They point at template infrastructure this project did not
     adopt. Holding the count is what stops a tenth appearing unnoticed;
 14. every BARE path named in backticks anywhere this project wrote resolves to
     something git tracks. Check 11 sees only a `path:line`, and the citation
     that actually broke this repository carried no line number. Two exemptions,
     both narrow and both visible in the source: a path under
     `experiments/results/`, which is where a measurement not yet taken will
     land, and a line carrying the `known-absent` token, which is how a document
     names something deliberately not here;
 15. no build artefact or experiment scratch directory is tracked. An
     experiment that stages a rootfs or builds an object writes hundreds of
     somebody else's files next to the script, and one `git add -A` commits
     them. Measured on 2026-09-08: about 1400 files of a debian rootfs and 23
     cargo artefacts reached a commit this way, because `.gitignore` carried a
     LIST of scratch directory names rather than a rule;
 16. ⭐ every check above examined something. A check that runs zero assertions
     and a check whose assertions all pass produce the same exit code, and the
     first is the state this repository was actually in at its first commit.
     The coverage line below is printed on every run and a zero in it is a
     failure, not a note;
 17. the release binary's size ceiling is declared in exactly one place, and
     the committed baseline is under it. TODO/deps.md T-0910: a dependency that
     lands without a before-and-after number has not landed, and without a
     committed baseline there is no "before". The ceiling was a literal in the
     CI workflow with nothing behind it; a number in two files drifts, and the
     copy a reader trusts is the wrong one. ⚠ This check reads the COMMITTED
     evidence and never builds anything, so it runs on a fresh clone with no
     toolchain. The build-time half is the workflow calling the script;
 18. no experiment number carries two names. experiments/README.md rules that a
     number is never reused, because a citation of `30-` has to keep meaning
     what it meant, and nothing enforced it: sweeping every `Prove` against
     experiments/ at the close of M1 found FOUR entries naming a taken number,
     one of them M2's own acceptance. Each would have been discovered by
     whoever implemented it, mid-flight, which is how TODO/image.md T-0203's
     was found and what it cost. TODO/gate.md T-1205;
 19. every CI job that runs cargo installs the toolchain .cargo/config.toml
     says a cargo build needs. The component list in .github/workflows/ is a
     SECOND declaration of that, and it drifted: T-0201 pointed
     `CC_<target>` at scripts/zig-cc.sh, and the workflow's list was never
     given `zig`, so nine consecutive pushes to main died inside `ring`'s build
     script with "zig is not on PATH" while every local run stayed green.
     ⛔ The required set is DERIVED, never listed here: config.toml names the
     wrapper, and the wrapper names the component that installs it. TODO/gate.md
     T-1206;
 20. Docker-compatible exit codes are declared in exactly one source file.
 21. No `Prove` line pulls from the registry the acceptance may not use.
     TODO/gate.md T-1209: a `Prove` line IS the acceptance, so the rule that
     keeps it off Docker Hub applies to every one of them. A `Prove` block
     (the `Prove:` line and its indented continuations) may name no
     `docker.io/` reference and no unqualified image reference; every
     reference is a row of DISTRO_ROWS_M5 in
     `scripts/common/distro-matrix.sh`. A block re-reading
     `experiments/results/across/` is exempt: those readings were taken
     against DISTRO_ROWS_NSSWITCH digests and the same tag at another
     registry is another image. A bare row name handed to a matrix script
     is not an image reference: the script resolves it to a pinned digest
     internally.
 22. Every `done` entry opens its record with a bold `Done` paragraph on the
     first unindented line after `Prove`. TODO/gate.md T-1208: a closed entry
     nobody ran reads as done, and four of them did.

⛔ Read the exit code from this process, unpiped.
Exit: 0 everything agrees, 1 something disagrees, 2 could not run.

⚠ THIS SCRIPT IS NOT SELF-EVIDENT AND ITS CHECKS HAVE BEEN WRONG BEFORE.
`scripts/plant.sh` breaks each one on purpose and asserts that it goes red with
its own message. An assertion nobody has seen fail is not an assertion. Add a
check here and a case there in the same change.

Python rather than shell because the checks are string and arithmetic work, and
check 4 is the arithmetic this script exists to stop being done by hand.
docs/methodology/work-todo.md names a pair of scripts and not a language.
"""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TODO = os.path.join(ROOT, "TODO")
REFS = os.path.join(ROOT, "references")

# The corpus and imported methodology are somebody else's text. Their internal
# citations are not this project's to hold. Project-authored documents beside
# those imported directories are checked like every other project file.
CORPUS_PREFIX = "references/"
VERBATIM_PREFIXES = (
    "docs/conventions/",
    "docs/methodology/",
    "docs/security/",
)

# Current template infrastructure resolves every imported methodology link.
# Holding zero is what stops a new dangling link from becoming accepted noise.
KNOWN_VERBATIM_DANGLING = 0

# Files this project wrote that are neither markdown nor under TODO/, and that
# carry citations worth resolving.
SOURCE_SUFFIXES = (".rs", ".sh", ".py", ".toml", ".yml", ".yaml")

# ⭐ The first of check 14's two exemptions, and it is narrow on purpose.
# `experiments/results/` is where a measurement lands, so an entry whose work is
# to take that measurement cites a file that does not exist yet. That is a
# forward reference and it is the point of the entry. Everything else that
# names a path must name one the tree has: the defect this repository shipped
# first was a README naming `TODO/`, which carried no line number and which a
# `path:line` check therefore could not see.
FORWARD_REF_PREFIX = "experiments/results/"

# ⭐ The second and last of check 14's exemptions, per LINE rather than
# per file, so it cannot be turned on for a whole document by accident. A line
# carrying this token names a path that is deliberately absent, and naming it
# is the point. The token is greppable, so every exemption in the
# tree can be listed in one command:
#     git grep -n 'known-absent'
KNOWN_ABSENT = "<!-- known-absent -->"

# ⛔ Check 15. Nothing under these may be tracked. `experiments/results/` is the
# exception and is the evidence, so it is deliberately NOT a scratch prefix:
# every scratch directory is a DOT directory under experiments/.
SCRATCH = re.compile(
    r"^(experiments/\.[^/]+/|crates/[^/]+/target/|target/|.*/__pycache__/)"
)

FIELDS = [
    "Source", "Category", "Priority", "Effort", "Status",
    "Problem", "Premise", "Approach", "Decision", "Prove",
]
STATUSES = ["open", "partial", "blocked", "done"]
PRIORITIES = ["P0", "P1", "P2", "P3"]

# A citation this script resolves. Anchored at a top-level directory of the
# repository so that prose mentioning `foo.c:12` in the abstract is not read as
# a path. The trailing `[).,;]?` is stripped by the capture group ending.
CITE = re.compile(
    r"(?<![\w/.-])((?:references|crates|experiments|scripts|docs|TODO)"
    r"/[A-Za-z0-9._+@/-]+?):(\d+)(?:-(\d+))?(?![\w.-])"
)
ROW = re.compile(
    r"^\|\s*\[(T-\d{4})\]\(([^)]+)\)\s*\|\s*(P\d)\s*\|\s*([a-z-]+)\s*\|"
    r"\s*\*{0,2}([a-z]+)\*{0,2}\s*\|\s*(.+?)\s*\|\s*$"
)
HEADING = re.compile(r"^### (T-\d{4}) +(\S.*)$")
# A path named in backticks with no line number. This is the shape the first
# defect took, and a `path:line` matcher cannot see it.
BARE = re.compile(
    r"`((?:references|crates|experiments|scripts|docs|TODO)"
    r"/[A-Za-z0-9._+@/-]*)`"
)
LINK = re.compile(r"\[[^\]]*\]\(([^)#][^)]*?)(?:#[^)]*)?\)")

errors = []
warnings = []

# ⭐ Every check increments its counter. A zero is a check that examined
# nothing, which is reported as a failure rather than as a clean run.
seen = {
    "rows": 0, "entries": 0, "fields": 0, "counts": 0, "corpus": 0,
    "todo_citations": 0, "todo_links": 0, "crossrefs": 0,
    "tree_citations": 0, "tree_links": 0, "bare_citations": 0,
    "size_ceiling": 0, "experiment_numbers": 0, "ci_components": 0,
    "exit_codes": 0, "prove_registry": 0, "closure_records": 0,
}

# ⛔ Check 17. The one file allowed to declare the release binary's ceiling, and
# the committed evidence it is checked against. Both are read as text; nothing
# is built, so this runs on a fresh clone.
CEILING_SCRIPT = "experiments/110-bloat-delta.sh"
CEILING_DECL = re.compile(r"^CEILING_BYTES=(\d+)$", re.M)
BLOAT_BASELINE = "experiments/results/bloat-baseline.txt"
BLOAT_TOTAL = re.compile(r"^total_bytes (\d+)$", re.M)

# ⛔ Check 18. An experiment script named anywhere this project wrote, or
# present in the tracked listing. `experiments/README.md` rules that a number is
# never reused, because a citation of `30-` has to keep meaning what it meant.
# ⚠ The number is the subject and the name is the evidence, so both are
# captured: the finding is one number carrying two names, which is not visible
# from either half alone.
EXPERIMENT = re.compile(
    r"experiments/(\d+)-([A-Za-z0-9._+-]+)\.sh"
)

# ⛔ Check 19. The toolchain a cargo build needs is declared once, in
# `.cargo/config.toml`, and CI carries a second declaration of it as a list of
# bootstrap components. The two drifted for nine commits. Nothing below is a
# list of components: config.toml names a wrapper, the wrapper names the
# component that installs it, and this check holds CI to the union.
CARGO_CONFIG = ".cargo/config.toml"
WORKFLOWS = ".github/workflows/"
# `CC_x86_64_unknown_linux_musl = { value = "scripts/zig-cc.sh", ... }`
TOOLCHAIN_DECL = re.compile(
    r"^\s*(?:CC|AR|LD|CXX)_[A-Za-z0-9_]+\s*=\s*\{[^}]*?"
    r"value\s*=\s*\"([^\"]+)\"", re.M)
# A wrapper says how to install itself, and that sentence is the declaration.
COMPONENT_DECL = re.compile(r"bootstrap-env\.sh\s+([a-z][a-z0-9-]*)")
# One job, at exactly two spaces under `jobs:`.
JOB_HEAD = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
# A cargo invocation in a `run:` block. `cargo` as a word, not as a path
# component, so `~/.cargo/bin` on a PATH line is not one.
CARGO_CALL = re.compile(r"(?<![\w/.-])cargo\s+[a-z]")
# A repository script named in a job, for one level of indirection: a step that
# runs `./experiments/110-bloat-delta.sh ci` runs cargo without saying so.
REPO_SCRIPT = re.compile(r"\./((?:scripts|experiments)/[A-Za-z0-9._/-]+\.sh)")
BOOTSTRAP_CALL = re.compile(r"bootstrap-env\.sh([^\n#|&;]*)")


def err(where, msg):
    errors.append(f"{where}: {msg}")


def read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def tracked_files():
    """Every path git tracks, relative to ROOT.

    ⛔ git, not os.walk. A citation that resolves on this disk and not in a
    fresh clone is the defect this check exists for, and only git can tell
    them apart. If git cannot answer, this check did not run, and a check that
    did not run does not report success: main() exits 2.
    """
    r = subprocess.run(["git", "-C", ROOT, "ls-files", "-z"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return None
    return {p for p in r.stdout.split("\0") if p}


def is_ours(rel):
    """True for a path this project wrote, so its citations are ours to hold."""
    if rel.startswith(CORPUS_PREFIX):
        return False
    if rel.startswith(VERBATIM_PREFIXES):
        return False
    return True


def check_tree(files):
    """Checks 11 to 14: citations and links outside TODO/."""
    dangling_verbatim = 0
    for rel in sorted(files):
        if rel.startswith(CORPUS_PREFIX):
            continue
        ours = is_ours(rel)
        is_md = rel.endswith(".md")
        if not is_md and not rel.endswith(SOURCE_SUFFIXES):
            continue
        path = os.path.join(ROOT, rel)
        try:
            text = read(path)
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if ours:
                for m in CITE.finditer(line):
                    cited, start, end = m.group(1), int(m.group(2)), m.group(3)
                    where = f"{rel}:{n}"
                    if rel.startswith("TODO/"):
                        continue  # check 7 already owns these
                    seen["tree_citations"] += 1
                    target = os.path.join(ROOT, cited)
                    if not os.path.isfile(target):
                        err(where, f"cites {cited}:{start}, and that file does not exist")
                        continue
                    if cited not in files:
                        err(where, f"cites {cited}, which exists here and is NOT "
                                   f"tracked by git, so a fresh clone does not have it")
                        continue
                    with open(target, "rb") as fh:
                        count = sum(1 for _ in fh)
                    last = int(end) if end else start
                    if last > count:
                        err(where, f"cites {cited}:{start}, and that file has {count} lines")
            if ours and KNOWN_ABSENT not in line:
                for m in BARE.finditer(line):
                    cited = m.group(1)
                    seen["bare_citations"] += 1
                    if cited.startswith(FORWARD_REF_PREFIX):
                        continue
                    stem = cited.rstrip("/")
                    if stem in files:
                        continue
                    if any(f.startswith(stem + "/") for f in files):
                        continue
                    err(f"{rel}:{n}",
                        f"names `{cited}`, and git tracks no such file or "
                        f"directory. A citation that resolves on one disk and "
                        f"not in a fresh clone is the defect this check exists "
                        f"for.")
            if not is_md:
                continue
            for m in LINK.finditer(line):
                href = m.group(1).strip().split("#")[0]
                if not href or href.startswith(("http://", "https://", "mailto:")):
                    continue
                target = os.path.normpath(os.path.join(os.path.dirname(path), href))
                exists = os.path.exists(target)
                if ours:
                    if rel.startswith("TODO/"):
                        continue  # check 8 already owns these
                    seen["tree_links"] += 1
                    if not exists:
                        err(f"{rel}:{n}", f"link target {href} does not resolve")
                    else:
                        # ⛔ `git ls-files` answers with forward slashes on
                        # every platform and `os.path.relpath` answers with the
                        # host separator. On Windows the two never matched, so
                        # every link in the tree was reported as untracked: 203
                        # false problems, measured on 2026-09-11. The separator
                        # is normalised to git's.
                        trel = os.path.relpath(target, ROOT).replace(os.sep, "/")
                        if os.path.isfile(target) and trel not in files:
                            err(f"{rel}:{n}",
                                f"links to {href}, which exists here and is NOT "
                                f"tracked by git")
                elif not exists:
                    dangling_verbatim += 1
    if dangling_verbatim != KNOWN_VERBATIM_DANGLING:
        err("docs/", f"{dangling_verbatim} dangling link(s) in the imported "
                     f"methodology; expected {KNOWN_VERBATIM_DANGLING}. "
                     f"Reconcile the import or provide the missing target.")


def check_size_ceiling(files):
    """Check 17: one declaration of the ceiling, and a baseline under it."""
    if CEILING_SCRIPT not in files:
        err(CEILING_SCRIPT, "does not exist or is not tracked, so the release "
                            "binary has no declared ceiling. TODO/deps.md T-0910.")
        return
    seen["size_ceiling"] += 1
    m = CEILING_DECL.search(read(os.path.join(ROOT, CEILING_SCRIPT)))
    if not m:
        err(CEILING_SCRIPT, "declares no `CEILING_BYTES=<n>` line. That line is "
                            "the ceiling's one home; without it every other file "
                            "naming a size is unanchored.")
        return
    ceiling = m.group(1)

    # ⛔ Nowhere else. A value in two places drifts, and a workflow that carries
    # its own copy passes a build the script would have refused. `.txt` results
    # under experiments/results/ are written BY the script and are not scanned:
    # they are readings, and a reading quotes the ceiling it was taken against.
    for rel in sorted(files):
        if rel == CEILING_SCRIPT or not is_ours(rel):
            continue
        if not rel.endswith(".md") and not rel.endswith(SOURCE_SUFFIXES):
            continue
        try:
            text = read(os.path.join(ROOT, rel))
        except (OSError, UnicodeDecodeError):
            continue
        seen["size_ceiling"] += 1
        for n, line in enumerate(text.splitlines(), 1):
            if re.search(rf"(?<!\d){ceiling}(?!\d)", line):
                err(f"{rel}:{n}",
                    f"names the binary size ceiling {ceiling} itself. It is "
                    f"declared in {CEILING_SCRIPT} and nowhere else; call that "
                    f"script instead of copying its number.")

    # The committed "before". Read, never rebuilt: this has to work on a clone
    # with no toolchain, and the number a dependency is measured against is the
    # one in the tree rather than the one this machine happens to produce.
    if BLOAT_BASELINE not in files:
        err(BLOAT_BASELINE,
            "is not tracked. TODO/deps.md T-0910: without a committed baseline "
            "there is no `before`, and a dependency that lands without a "
            "before-and-after number has not landed. Take it with "
            "`./experiments/110-bloat-delta.sh baseline`.")
        return
    seen["size_ceiling"] += 1
    b = BLOAT_TOTAL.search(read(os.path.join(ROOT, BLOAT_BASELINE)))
    if not b:
        err(BLOAT_BASELINE, "carries no `total_bytes <n>` line, so the baseline "
                            "cannot be compared with anything.")
        return

    # ⭐ TODO/gate.md T-1204. EVERY committed reading, not the baseline alone.
    # The baseline is deliberately the "before" and stops being the shipping
    # binary the moment a milestone lands, so a gate that reads only it holds a
    # number the ceiling was never about: M1 moved the artefact from 496,184 to
    # 2,130,672 bytes and this check stayed green throughout.
    #
    # ⚠ Every reading, and never "the newest by date". A date inside a file is a
    # string this would have to parse and rank, and a stale file somebody forgot
    # to delete would then silently outrank a real one. Asserting all of them
    # needs no ordering and fails on the same file either way.
    limit = int(ceiling)
    readings = sorted(f for f in files
                      if f.startswith("experiments/results/bloat-")
                      and f.endswith(".txt"))
    for rel in readings:
        m = BLOAT_TOTAL.search(read(os.path.join(ROOT, rel)))
        if not m:
            # ⚠ Not an error. An arm that could not run records that it could
            # not, and TODO/deps.md T-0910 rules a skip is not a pass; it is
            # also not a size to hold.
            continue
        seen["size_ceiling"] += 1
        total = int(m.group(1))
        if total >= limit:
            err(rel,
                f"records total_bytes {total}, which is at or over the ceiling "
                f"of {limit} declared in {CEILING_SCRIPT}. Raise the ceiling "
                f"deliberately, with the delta that justifies it committed "
                f"beside the change.")


def check_experiment_numbers(files):
    """Check 18: one experiment number carries one name.

    ⛔ Text and the tracked listing only. Nothing is executed and nothing is
    built, so this runs on a clone with no toolchain, which is this script's own
    constraint. TODO/gate.md T-1205.

    ⚠ A line carrying the `known-absent` token is skipped, exactly as check 14
    skips it. T-1205's own Premise records the collisions it found, old number
    beside new, and a check that read that table would report the defect the
    table exists to describe.
    """
    # number -> {name: the first place it was seen}
    numbers = {}

    def record(num, name, where):
        seen["experiment_numbers"] += 1
        numbers.setdefault(num, {}).setdefault(name, where)

    # The listing first, so a file on disk is the place a collision is reported
    # against rather than whichever document happened to sort first.
    for rel in sorted(files):
        m = EXPERIMENT.fullmatch(rel)
        if m:
            record(m.group(1), m.group(2), rel)

    for rel in sorted(files):
        if not is_ours(rel):
            continue
        if not rel.endswith(".md") and not rel.endswith(SOURCE_SUFFIXES):
            continue
        try:
            text = read(os.path.join(ROOT, rel))
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if KNOWN_ABSENT in line:
                continue
            for m in EXPERIMENT.finditer(line):
                record(m.group(1), m.group(2), f"{rel}:{n}")

    for num in sorted(numbers, key=int):
        names = numbers[num]
        if len(names) < 2:
            continue
        listed = ", ".join(f"`{num}-{nm}.sh` ({w})"
                           for nm, w in sorted(names.items()))
        err(f"experiments/{num}-",
            f"experiment number {num} carries {len(names)} names: {listed}. "
            f"experiments/README.md rules that a number is never reused, "
            f"because a citation of it has to keep meaning what it meant. "
            f"Give the new one a free number. TODO/gate.md T-1205.")


def cargo_toolchain_components():
    """Component -> the wrapper that declared it, for check 19.

    ⛔ DERIVED IN TWO HOPS AND HARD-CODED IN NEITHER. `.cargo/config.toml` names
    a program for `CC_<target>`; that program says which
    `bootstrap-env.sh` component installs it. A component added to the config
    is therefore required of CI the moment it is added, with nothing here to
    update. A wrapper that names no component contributes none, which is
    correct: `scripts/zig-ar.sh` is `exec zig ar` and installs nothing on its
    own.
    """
    comps = {}
    try:
        text = read(os.path.join(ROOT, CARGO_CONFIG))
    except OSError:
        return comps
    for m in TOOLCHAIN_DECL.finditer(text):
        wrapper = m.group(1)
        try:
            body = read(os.path.join(ROOT, wrapper))
        except (OSError, UnicodeDecodeError):
            continue
        for c in COMPONENT_DECL.findall(body):
            comps.setdefault(c, wrapper)
    return comps


def check_ci_components(files):
    """Check 19: a CI job that runs cargo bootstraps what cargo needs.

    ⛔ Text only. Nothing is executed and no YAML library is imported, so this
    runs on a clone with no toolchain, which is this script's own constraint.

    ⚠ ONE LEVEL OF INDIRECTION, and it is not decoration: the step that broke
    first was `cargo build`, but `./experiments/110-bloat-delta.sh ci` and
    `./scripts/build-interpose.sh` run cargo without the word appearing in the
    workflow at all, so a job could satisfy a word match and still be the job
    that fails. A script named in a job is read and its cargo calls count as
    the job's.

    ⚠ COMMENT-ONLY LINES ARE DROPPED FIRST. This workflow explains itself at
    length, and a comment saying why a cargo build needs something is not a
    cargo build. Reading one as a step would make the indirection arm above
    unreachable, and unreachable is how a check stops being one.
    """
    required = cargo_toolchain_components()
    for rel in sorted(f for f in files if f.startswith(WORKFLOWS)):
        try:
            text = read(os.path.join(ROOT, rel))
        except (OSError, UnicodeDecodeError):
            continue
        jobs, name, buf, in_jobs = {}, None, [], False
        for line in text.splitlines():
            if line.rstrip() == "jobs:":
                in_jobs = True
                continue
            if not in_jobs:
                continue
            m = JOB_HEAD.match(line)
            if m:
                if name:
                    jobs[name] = "\n".join(buf)
                name, buf = m.group(1), []
                continue
            if name:
                buf.append(line)
        if name:
            jobs[name] = "\n".join(buf)

        for job in sorted(jobs):
            body = "\n".join(ln for ln in jobs[job].splitlines()
                             if not ln.lstrip().startswith("#"))
            runs_cargo = bool(CARGO_CALL.search(body))
            via = None
            for script in sorted(set(REPO_SCRIPT.findall(body))):
                if script not in files:
                    continue
                try:
                    body_of = "\n".join(
                        ln for ln in read(os.path.join(ROOT, script)).splitlines()
                        if not ln.lstrip().startswith("#"))
                except (OSError, UnicodeDecodeError):
                    continue
                if CARGO_CALL.search(body_of):
                    runs_cargo, via = True, script
                    break
            if not runs_cargo:
                continue
            have = set()
            for m in BOOTSTRAP_CALL.finditer(body):
                have.update(a for a in m.group(1).split() if not a.startswith("-"))
            through = f" (through `{via}`)" if via and not CARGO_CALL.search(body) else ""
            for comp in sorted(required):
                seen["ci_components"] += 1
                if comp in have:
                    continue
                err(f"{rel}",
                    f"job `{job}` runs cargo{through} and its "
                    f"`bootstrap-env.sh` list does not carry `{comp}`. "
                    f"`{CARGO_CONFIG}` points a cargo build at "
                    f"`{required[comp]}`, which says it is installed by "
                    f"`bootstrap-env.sh {comp}`, so the job dies inside a "
                    f"dependency's build script rather than on its own step. "
                    f"TODO/gate.md T-1206.")


EXIT_CODE_HOME = "crates/podbox-probe/src/exit.rs"
EXIT_CODE_DECL = re.compile(
    r"^\s*pub const (EXIT_[A-Z_]+)\s*:\s*i32\s*=\s*(\d+)\s*;", re.M)


def check_exit_codes(files):
    """Check 20: docker's exit codes are declared in exactly one file.

    ⭐ TODO/cli.md T-0802. These numbers are the first thing an automated caller
    reads, and they had been written out in FOUR files of one binary --
    `podbox-image::error`, `podbox-cli::main`, `podbox-enter` and
    `podbox-supervise`. Two of the copies had already diverged: a correction
    measured against docker landed in one and the others kept the old value, so
    two verbs of one binary disagreed about what a flag error is.

    ⛔ Text only, and it looks for the DECLARATION rather than the number. A
    `pub use podbox_probe::exit::EXIT_RUNTIME_ERROR` is how a crate is supposed
    to get one and is not a declaration; `pub const EXIT_RUNTIME_ERROR: i32 =
    125;` is. ⚠ A bare `125` in a match arm or a message is not matched either:
    the defect is a second SOURCE of the value, not a second mention of it.

    ⚠ The home file is named here and nowhere else, so moving it is one edit.
    """
    for rel in sorted(f for f in files
                      if f.startswith("crates/") and f.endswith(".rs")):
        if rel == EXIT_CODE_HOME:
            continue
        try:
            text = read(os.path.join(ROOT, rel))
        except (OSError, UnicodeDecodeError):
            continue
        for m in EXIT_CODE_DECL.finditer(text):
            seen["exit_codes"] += 1
            n = text[:m.start()].count("\n") + 1
            err(f"{rel}:{n}",
                f"declares `{m.group(1)} = {m.group(2)}` and `{EXIT_CODE_HOME}` "
                f"already holds docker's exit codes. A second declaration is how "
                f"two verbs of one binary came to disagree about what a flag "
                f"error is: `pub use podbox_probe::exit::{m.group(1)};` is the "
                f"way to have it. TODO/cli.md T-0802.")
    # ⚠ Counted even when nothing is wrong, so a check that matches nothing is
    # distinguishable from one that ran: the home file's own declarations.
    try:
        seen["exit_codes"] += len(
            EXIT_CODE_DECL.findall(read(os.path.join(ROOT, EXIT_CODE_HOME))))
    except (OSError, UnicodeDecodeError):
        err(EXIT_CODE_HOME,
            "is where docker's exit codes live and it is not readable. "
            "TODO/cli.md T-0802.")


# ⛔ Check 21. No Prove line pulls from the registry the acceptance may not
# use. TODO/gate.md T-1209: a Prove line IS the acceptance, because RULES.md
# section 5 closes an entry on that command actually run, so the rule that
# keeps the acceptance off Docker Hub applies to every one of them.
#
# A Prove BLOCK is the `Prove:` line and its indented continuations: T-0301
# carries its second command there, and a matcher reading the first line
# alone misses it. Lines carrying the `known-absent` token are skipped, as
# checks 14 and 18 skip them; a block re-reading
# `experiments/results/across/` is exempt, because those readings were taken
# against DISTRO_ROWS_NSSWITCH digests and the same tag at another registry
# is another image. A bare row name handed to a matrix script (T-0412's
# `240-distro-sweep.sh opensuse-leap`) is not an image reference: the script
# resolves it to a pinned digest internally. That carve-out covers the
# unqualified arm only; a `docker.io/` reference is a Hub pull wherever it
# stands.
PROVE_FIELD = re.compile(r"^Prove:")
PROVE_ENTRY = re.compile(r"^### (T-\d{4})")
PROVE_UNQUALIFIED = re.compile(
    r"(?<![\w./:-])(?:(?:voidlinux/voidlinux-musl|rockylinux/rockylinux"
    r"|opensuse/leap|library/[A-Za-z0-9._-]+|golang:[A-Za-z0-9._-]+"
    r"|alpine|debian|ubuntu|archlinux|fedora|almalinux|rocky-minimal"
    r"|rocky|opensuse-leap|voidlinux-musl)(?::[A-Za-z0-9._-]+)?)"
    r"(?![\w./:-])"
)
PROVE_ACROSS = "experiments/results/across/"


def check_prove_registry():
    """Check 21: Prove blocks name no Docker Hub reference."""
    for name in sorted(os.listdir(TODO)):
        if not name.endswith(".md"):
            continue
        lines = read(os.path.join(TODO, name)).splitlines()
        heads = []
        for i, ln in enumerate(lines):
            m = PROVE_ENTRY.match(ln)
            if m:
                heads.append((i, m.group(1)))
        for idx, (li, tid) in enumerate(heads):
            end = heads[idx + 1][0] if idx + 1 < len(heads) else len(lines)
            start = None
            for j in range(li, end):
                if PROVE_FIELD.match(lines[j]):
                    start = j
                    break
            if start is None:
                continue
            j = start + 1
            while j < end and lines[j][:1] in (" ", "\t"):
                j += 1
            seen["prove_registry"] += 1
            block = lines[start:j]
            if any(PROVE_ACROSS in ln for ln in block):
                continue
            for k, ln in enumerate(block):
                if KNOWN_ABSENT in ln:
                    continue
                where = f"TODO/{name}:{start + k + 1}"
                m = re.search(r"docker\.io/\S*", ln)
                if m:
                    err(where,
                        f"({tid}) Prove names `{m.group(0)}`, which pulls "
                        f"from Docker Hub. Every Prove reference is a row "
                        f"of DISTRO_ROWS_M5 in "
                        f"`scripts/common/distro-matrix.sh`, named by its "
                        f"fully qualified reference. TODO/gate.md T-1209.")
                    continue
                m = PROVE_UNQUALIFIED.search(ln)
                while m:
                    # A matrix row handed to a script is resolved to a pinned
                    # digest inside it, so it is not an image reference.
                    if re.search(r"\.sh\s+$", ln[:m.start()]):
                        m = PROVE_UNQUALIFIED.search(ln, m.end())
                        continue
                    err(where,
                        f"({tid}) Prove names an unqualified image "
                        f"reference `{m.group(0)}`, which resolves through "
                        f"the engine's shortname aliases to Docker Hub. "
                        f"Every Prove reference is a row of DISTRO_ROWS_M5 "
                        f"in `scripts/common/distro-matrix.sh`, named by "
                        f"its fully qualified reference. "
                        f"TODO/gate.md T-1209.")
                    break


DONE_OPEN = re.compile(r"^\*\*Done")


def check_closure_records(entries):
    """Check 22: every done entry opens its record with a bold Done paragraph.

    The first unindented, non-blank line after the Prove block has to open
    with `**Done`. TODO/gate.md T-1208 is the ruling; RULES.md section 5 is
    the shape it states.
    """
    for tid in sorted(entries):
        e = entries[tid]
        m = re.search(r"^Status: +(\S.*)$", e["body"], re.M)
        if not m:
            continue  # check 5 reports the missing field
        if m.group(1).strip("*").split()[0] != "done":
            continue
        seen["closure_records"] += 1
        where = f"TODO/{e['file']}:{e['line']}"
        lines = e["body"].splitlines()
        start = None
        for i, ln in enumerate(lines):
            if re.match(r"^Prove: +", ln):
                start = i
                break
        if start is None:
            continue  # check 5 reports the missing field
        j = start + 1
        while j < len(lines) and (not lines[j].strip() or lines[j][:1] in (" ", "\t")):
            j += 1
        if j >= len(lines) or not DONE_OPEN.match(lines[j]):
            err(where,
                f"({tid}) closes without a recorded run: the first unindented "
                f"line after Prove does not open with `**Done`. "
                f"TODO/gate.md T-1208.")


def main():
    if not os.path.isdir(TODO):
        print("check-todo: TODO/ does not exist", file=sys.stderr)
        return 2
    index_path = os.path.join(TODO, "INDEX.md")
    if not os.path.isfile(index_path):
        print("check-todo: TODO/INDEX.md does not exist", file=sys.stderr)
        return 2

    index = read(index_path)

    # -- 1. the rows ---------------------------------------------------------
    rows = {}
    for n, line in enumerate(index.splitlines(), 1):
        if not line.startswith("| [T-"):
            continue
        m = ROW.match(line)
        if not m:
            err(f"TODO/INDEX.md:{n}", f"row does not parse: {line.strip()}")
            continue
        tid, target, prio, cat, status, title = m.groups()
        if tid in rows:
            err(f"TODO/INDEX.md:{n}", f"{tid} appears twice")
        if status not in STATUSES:
            err(f"TODO/INDEX.md:{n}", f"{tid} status {status!r} is not one of {STATUSES}")
        if prio not in PRIORITIES:
            err(f"TODO/INDEX.md:{n}", f"{tid} priority {prio!r} is not one of {PRIORITIES}")
        rows[tid] = dict(target=target, prio=prio, cat=cat, status=status,
                         title=title, line=n)
        seen["rows"] += 1

    if not rows:
        err("TODO/INDEX.md", "no entry rows found")

    # -- 2. the entries ------------------------------------------------------
    entries = {}
    for name in sorted(os.listdir(TODO)):
        if not name.endswith(".md") or name in ("INDEX.md", "PROGRESS.md",
                                                "RULES.md", "reference-map.md"):
            continue
        path = os.path.join(TODO, name)
        text = read(path)
        lines = text.splitlines()
        heads = [(i, HEADING.match(ln)) for i, ln in enumerate(lines)]
        heads = [(i, m.group(1), m.group(2)) for i, m in heads if m]
        for idx, (i, tid, title) in enumerate(heads):
            end = heads[idx + 1][0] if idx + 1 < len(heads) else len(lines)
            body = "\n".join(lines[i:end])
            if tid in entries:
                err(f"TODO/{name}:{i + 1}", f"{tid} is defined twice")
            entries[tid] = dict(file=name, line=i + 1, title=title, body=body)
            seen["entries"] += 1

    # -- cross-check ---------------------------------------------------------
    for tid, row in sorted(rows.items()):
        e = entries.get(tid)
        if e is None:
            err(f"TODO/INDEX.md:{row['line']}", f"{tid} has a row and no entry")
            continue
        if e["file"] != row["target"]:
            err(f"TODO/INDEX.md:{row['line']}",
                f"{tid} row links to {row['target']} and the entry is in {e['file']}")
        if e["title"] != row["title"]:
            err(f"TODO/INDEX.md:{row['line']}",
                f"{tid} row title {row['title']!r} != entry title {e['title']!r}")
    for tid, e in sorted(entries.items()):
        if tid not in rows:
            err(f"TODO/{e['file']}:{e['line']}", f"{tid} has an entry and no row")

    # -- 3, 5. the fields ----------------------------------------------------
    for tid, e in sorted(entries.items()):
        where = f"TODO/{e['file']}:{e['line']}"
        for field in FIELDS:
            m = re.search(rf"^{field}: +(\S.*)$", e["body"], re.M)
            if not m:
                err(where, f"{tid} has no `{field}:` field")
                continue
            value = m.group(1).strip()
            seen["fields"] += 1
            if field == "Status":
                status = value.strip("*").split()[0]
                if status not in STATUSES:
                    err(where, f"{tid} Status {status!r} is not one of {STATUSES}")
                elif tid in rows and status != rows[tid]["status"]:
                    err(where,
                        f"{tid} entry status {status!r} disagrees with the index "
                        f"row's {rows[tid]['status']!r}")
            if field == "Category" and tid in rows and value != rows[tid]["cat"]:
                err(where, f"{tid} Category {value!r} disagrees with the row's "
                           f"{rows[tid]['cat']!r}")
            if field == "Priority" and tid in rows and value != rows[tid]["prio"]:
                err(where, f"{tid} Priority {value!r} disagrees with the row's "
                           f"{rows[tid]['prio']!r}")
        # `Prove` has to be a command, per authoring.md section 6.
        m = re.search(r"^Prove: +(.*)$", e["body"], re.M)
        if m and not re.search(r"```|`[^`]+`|^\s{2,}\S", m.group(1)):
            err(where, f"{tid} Prove is prose, not a command: {m.group(1)[:60]!r}")

    # -- 4. the counts -------------------------------------------------------
    derived = {s: 0 for s in STATUSES}
    per_prio = {p: {s: 0 for s in STATUSES} for p in PRIORITIES}
    for row in rows.values():
        derived[row["status"]] += 1
        per_prio[row["prio"]][row["status"]] += 1
    total = len(rows)

    seen["counts"] = 1 + len(PRIORITIES)
    want = (f"{total} items: {derived['open']} open, {derived['partial']} partial, "
            f"{derived['blocked']} blocked, {derived['done']} done.")
    if want not in index:
        err("TODO/INDEX.md", f"the totals line is not the rows. Expected exactly:\n    {want}")

    for p in PRIORITIES:
        c = per_prio[p]
        t = sum(c.values())
        want_row = (f"| {p} | {c['open']} | {c['partial']} | {c['blocked']} | "
                    f"{c['done']} | {t} |")
        if want_row not in index:
            err("TODO/INDEX.md", f"the {p} count row is not the rows. Expected exactly:\n    {want_row}")
    want_all = (f"| **All** | **{derived['open']}** | **{derived['partial']}** | "
                f"**{derived['blocked']}** | **{derived['done']}** | **{total}** |")
    if want_all not in index:
        err("TODO/INDEX.md", f"the All count row is not the rows. Expected exactly:\n    {want_all}")

    # -- 6. the corpus -------------------------------------------------------
    map_path = os.path.join(TODO, "reference-map.md")
    if not os.path.isfile(map_path):
        err("TODO/reference-map.md", "does not exist")
    else:
        named = set(re.findall(r"`references/([A-Za-z0-9._+-]+)`", read(map_path)))
        if not named:
            err("TODO/reference-map.md", "names no reference under references/")
        for tree in sorted(named):
            d = os.path.join(REFS, tree)
            seen["corpus"] += 1
            if not os.path.isdir(d):
                err("TODO/reference-map.md", f"references/{tree} does not exist")
            elif not os.path.isfile(os.path.join(d, "PROVENANCE.md")):
                err("TODO/reference-map.md", f"references/{tree} has no PROVENANCE.md")
        on_disk = {d for d in os.listdir(REFS)
                   if os.path.isdir(os.path.join(REFS, d))} if os.path.isdir(REFS) else set()
        for tree in sorted(on_disk - named):
            err("TODO/reference-map.md",
                f"references/{tree} is on disk and the map does not name it")

    # -- 7, 8. citations and links ------------------------------------------
    for name in sorted(os.listdir(TODO)):
        if not name.endswith(".md"):
            continue
        path = os.path.join(TODO, name)
        for n, line in enumerate(read(path).splitlines(), 1):
            for m in CITE.finditer(line):
                rel, start, end = m.group(1), int(m.group(2)), m.group(3)
                seen["todo_citations"] += 1
                target = os.path.join(ROOT, rel)
                where = f"TODO/{name}:{n}"
                if not os.path.isfile(target):
                    err(where, f"cites {rel}:{start}, and that file does not exist")
                    continue
                with open(target, "rb") as fh:
                    count = sum(1 for _ in fh)
                last = int(end) if end else start
                if last > count:
                    err(where, f"cites {rel}:{m.group(0).split(':', 1)[1]}, "
                               f"and that file has {count} lines")
            for m in LINK.finditer(line):
                href = m.group(1).strip()
                if href.startswith(("http://", "https://", "mailto:")):
                    continue
                seen["todo_links"] += 1
                target = os.path.normpath(os.path.join(TODO, href.split(":")[0]))
                if not os.path.exists(target):
                    err(f"TODO/{name}:{n}", f"link target {href} does not resolve")

    # -- 9. cross-references ------------------------------------------------
    for name in sorted(os.listdir(TODO)):
        if not name.endswith(".md"):
            continue
        for n, line in enumerate(read(os.path.join(TODO, name)).splitlines(), 1):
            if line.startswith("| [T-") or line.startswith("### T-"):
                continue
            for tid in set(re.findall(r"\bT-\d{4}\b", line)):
                seen["crossrefs"] += 1
                if tid not in entries:
                    err(f"TODO/{name}:{n}", f"names {tid}, which is not an entry")

    # -- 10. the record's count line -----------------------------------------
    prog_path = os.path.join(TODO, "PROGRESS.md")
    if not os.path.isfile(prog_path):
        err("TODO/PROGRESS.md", "does not exist")
    else:
        want_prog = (f"{total} entries: {derived['open']} open, "
                     f"{derived['partial']} partial, {derived['blocked']} blocked, "
                     f"{derived['done']} done.")
        if want_prog not in read(prog_path):
            err("TODO/PROGRESS.md",
                f"the count line is not the rows. Expected exactly:\n    {want_prog}")

    # -- 11 to 14. the rest of the tree --------------------------------------
    files = tracked_files()
    if files is None:
        print("check-todo: `git ls-files` failed, so checks 11 to 15 could not "
              "run. This is not a pass.", file=sys.stderr)
        return 2
    check_tree(files)

    # -- 15. no artefact or scratch is tracked -------------------------------
    for rel in sorted(files):
        if SCRATCH.match(rel):
            err(rel, "is a build artefact or experiment scratch and is tracked. "
                     "Untrack it and make .gitignore carry a rule rather than a "
                     "list of names.")

    # -- 17. the size ceiling and its committed baseline ---------------------
    check_size_ceiling(files)

    # -- 18. one experiment number, one name ---------------------------------
    check_experiment_numbers(files)

    # -- 19. CI installs what a cargo build needs ----------------------------
    check_ci_components(files)
    check_exit_codes(files)

    # -- 21. Prove lines stay off Docker Hub ---------------------------------
    check_prove_registry()

    # -- 22. A closed entry carries its recorded run -------------------------
    check_closure_records(entries)

    # -- 16. coverage --------------------------------------------------------
    # ⭐ A check that examined nothing reports success otherwise, which is the
    # exact state this repository shipped its first commit in.
    empty = [k for k, v in seen.items() if v == 0]
    for k in empty:
        err("check-todo", f"the {k!r} check examined nothing. Either the tree "
                          f"lost every case or the check stopped matching; both "
                          f"are failures.")

    # -- report --------------------------------------------------------------
    print(f"check-todo: {len(rows)} rows, {len(entries)} entries, "
          f"{derived['open']} open, {derived['partial']} partial, "
          f"{derived['blocked']} blocked, {derived['done']} done")
    print("check-todo: coverage " +
          " ".join(f"{k}={v}" for k, v in sorted(seen.items())))
    for w in warnings:
        print(f"  warn {w}")
    if errors:
        print(f"check-todo: {len(errors)} problem(s)", file=sys.stderr)
        for e in errors:
            print(f"  {e}", file=sys.stderr)
        return 1
    print("check-todo: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
