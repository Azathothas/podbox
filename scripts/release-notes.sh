#!/bin/sh
# release-notes.sh - the nightly release notes TODO/packaging.md T-1334 needs.
#
# Usage: sh scripts/release-notes.sh TAG
#
# The notes carry two facts a downloader otherwise cannot get: the gate
# state of the commit TAG names, and the reproducibility boundary of the
# bytes beside them. Both are computed, never copied: the gate state is
# this workflow's own `gate` conclusion on the tagged commit (read back
# through the release API's caller, `gh`), and the boundary is T-1004's
# intra-host claim with its condition stated.
#
# Needs `gh` and the network, so it exits 2 on a machine without them:
# "could not run" is the third state everywhere in this tree, never a
# pass and never a failure.
#
# Exit: 0 the notes printed, 2 `gh` or the network could not answer.
set -u

TAG="${1:-}"
[ -n "$TAG" ] || { echo "release-notes: usage: release-notes.sh TAG" >&2; exit 2; }
command -v gh >/dev/null 2>&1 || { echo "release-notes: gh is not installed" >&2; exit 2; }

REPO="${GH_REPO:-Azathothas/podbox}"
COMMIT="$(git rev-parse "$TAG^{commit}" 2>/dev/null)" || { echo "release-notes: no commit for $TAG" >&2; exit 2; }

# The gate conclusion on the tagged commit: the newest completed `gate`
# run on main whose head SHA is the commit. A tag never moves, so the
# newest completed run for the commit is the state of the commit.
GATE="$(gh run list --repo "$REPO" --workflow gate.yml --branch main \
  --status completed --limit 20 --json headSha,conclusion,url \
  --jq "[.[] | select(.headSha == \"$COMMIT\")][0] | \"\\(.conclusion // \"none\") \\(.url // \"\")\"" 2>/dev/null)" \
  || { echo "release-notes: the gate runs could not be read" >&2; exit 2; }

cat <<EOF
Nightly pre-release: all seven claimed archs built as static binaries, each smoke-tested on its own arch (version, both interposer digests, crt-static). The full acceptance stays host-arch. Each binary carries the x86_64 interposer pair, so interposition declines by name off x86_64 until per-arch objects exist (TODO/interpose.md T-1327).

Build commit: $COMMIT
Gate on that commit: $GATE

Reproducibility boundary: byte-identical within one host and toolchain; cross-host builds differ in the embedded glibc interposer object (its digest is in \`podbox version --verbose\`, T-1004). A downloader re-deriving these bytes needs the same host glibc the release builder linked against, not only the same rustc and zig.
EOF
