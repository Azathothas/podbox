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
# Exit: 0 the notes printed, 2 the exact-commit gate or required read is absent.
set -u

TAG="${1:-}"
[ -n "$TAG" ] || { echo "release-notes: usage: release-notes.sh TAG" >&2; exit 2; }
command -v gh >/dev/null 2>&1 || { echo "release-notes: gh is not installed" >&2; exit 2; }

REPO="${GH_REPO:-Azathothas/podbox}"
COMMIT="$(git rev-parse "$TAG^{commit}" 2>/dev/null)" || { echo "release-notes: no commit for $TAG" >&2; exit 2; }

# The gate conclusion on the tagged commit: the newest completed `gate`
# run on main whose head SHA is the commit. A tag never moves, so the
# newest completed run for the commit is the state of the commit.
GATE="$(timeout 120 gh run list --repo "$REPO" --workflow gate.yml --branch main \
  --status completed --limit 20 --json headSha,conclusion,url \
  --jq "[.[] | select(.headSha == \"$COMMIT\")][0] | \"\\(.conclusion // \"none\") \\(.url // \"\")\"" 2>/dev/null)" \
  || { echo "release-notes: the gate runs could not be read" >&2; exit 2; }
case "$GATE" in
success\ *) ;;
*) echo "release-notes: no successful main gate for the exact build commit" >&2; exit 2 ;;
esac

cat <<EOF
Nightly pre-release: seven static CLI binaries passed their architecture smoke checks.
The smoke checks version, target, interposer digests, static linkage, image pull, and extraction.
The full runtime acceptance uses the host architecture.
Each binary embeds the x86_64 interposer pair. Other architectures refuse that interposition path until compatible objects exist.

Build commit: $COMMIT
Gate on that commit: $GATE

Reproducibility boundary: byte-identical within one host and toolchain.
Cross-host builds differ in the embedded glibc interposer object.
Its digest is in \`podbox version --verbose\`.
Reproduction requires the builder's host glibc as well as its Rust and Zig toolchains.
EOF
if git cat-file -e "$COMMIT:scripts/package-ssh.sh" 2>/dev/null; then
  cat <<EOF

Each architecture also has a signed podbox-ssh archive and checksum.
It contains node, operator, proxy, shell, and the locked registry package licence texts.
Place compatible helpers beside podbox or on PATH. Supply the required external SSH server where the selected path needs it.
Helper smoke checks static ELF linkage and usage status. It does not prove every architecture's live SSH session.

Current runtime limits and incomplete guest and server acceptance:
https://github.com/$REPO/blob/$COMMIT/docs/limits.md
EOF
fi
