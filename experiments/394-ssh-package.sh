#!/bin/sh
# Does helper packaging reject a missing helper and invalid ELF bytes?
# Inputs: the just-built native musl workspace, plus two controlled mutations.
# Exit: 0 matched, 1 failed, 2 could not run. T-1405.
set -u
ROOT=$(pwd)
[ -f "$ROOT/scripts/package-ssh.sh" ] || exit 2
BIN_DIR="$ROOT/target/x86_64-unknown-linux-musl/release"
[ -x "$BIN_DIR/node" ] || exit 2
WORK=$(mktemp -d) || exit 2
trap 'rm -rf "${WORK:?}"' EXIT HUP INT TERM
echo '== conditions'
date -u +%Y-%m-%dT%H:%M:%SZ
uname -sr
git rev-parse HEAD
echo 'scope: native helper archive, usage refusals, and package guard mutations'
for helper in node operator proxy shell; do cp "$BIN_DIR/$helper" "$WORK/$helper" || exit 1; done
sh scripts/package-ssh.sh "$WORK" x86_64 || exit "$?"
mv "$WORK/node" "$WORK/node.saved" || exit 1
sh scripts/package-ssh.sh "$WORK" x86_64 >"$WORK/missing.txt" 2>&1
rc=$?
[ "$rc" -eq 1 ] && grep -q 'missing executable: node' "$WORK/missing.txt" || exit 1
echo 'ok: missing node rejected with its own message'
mv "$WORK/node.saved" "$WORK/node" || exit 1
printf 'invalid ELF fixture\n' >"$WORK/node"
sh scripts/package-ssh.sh "$WORK" x86_64 >"$WORK/invalid.txt" 2>&1
rc=$?
[ "$rc" -eq 1 ] && grep -q 'invalid ELF: node' "$WORK/invalid.txt" || exit 1
echo 'ok: invalid node bytes rejected with their own message'
tar -tzf dist/podbox-ssh-x86_64.tar.gz >"$WORK/members.txt" || exit 1
for helper in node operator proxy shell; do
    grep -qx "./$helper" "$WORK/members.txt" || exit 1
done
echo 'verdict SSH-PACKAGE-OK'
