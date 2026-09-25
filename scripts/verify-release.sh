#!/bin/sh
# verify-release.sh - the downloader's half of TODO/packaging.md T-1328.
#
# Usage: sh scripts/verify-release.sh TAG ARCH
#   TAG is a nightly tag (v0.1.0-beta.N), ARCH one of the seven matrix
#   names (x86_64, aarch64, riscv64gc, loongarch64, armv7, i686,
#   powerpc64le).
#
# Fetches podbox-ARCH and podbox-ARCH.sigstore from the tag's nightly
# pre-release and verifies the bundle with `cosign verify-blob` against
# the publish workflow's identity. Needs `cosign`, `gh` and the network,
# so it exits 2 without them: "could not run" is the third state
# everywhere in this tree.
#
# Exit: 0 the bundle verifies and names the workflow identity, 1 the
# bytes or the identity did not verify, 2 it could not run.
set -u

TAG="${1:-}"
ARCH="${2:-}"
[ -n "$TAG" ] && [ -n "$ARCH" ] || { echo "verify-release: usage: verify-release.sh TAG ARCH" >&2; exit 2; }
command -v cosign >/dev/null 2>&1 || { echo "verify-release: cosign is not installed" >&2; exit 2; }
command -v gh >/dev/null 2>&1 || { echo "verify-release: gh is not installed" >&2; exit 2; }

REPO="${GH_REPO:-Azathothas/podbox}"
WORK="$(mktemp -d)" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

gh release download "$TAG" --repo "$REPO" --dir "$WORK" \
  --pattern "podbox-$ARCH" --pattern "podbox-$ARCH.sigstore" \
  || { echo "verify-release: the nightly has no podbox-$ARCH assets for $TAG" >&2; exit 2; }

# The identity the publish job signs as: the nightly workflow at the
# tag being verified, keyless through the job's OIDC. The workflow
# answers version tags alone, so a bundle for TAG carries the tag's
# ref, never the main branch. A bundle from any other identity fails
# closed here rather than verifying as "some signature".
cosign verify-blob \
  --bundle "$WORK/podbox-$ARCH.sigstore" \
  --certificate-identity "https://github.com/$REPO/.github/workflows/nightly.yml@refs/tags/$TAG" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  "$WORK/podbox-$ARCH"
