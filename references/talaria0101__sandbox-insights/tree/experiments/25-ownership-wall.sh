#!/usr/bin/env bash
# 25-ownership-wall.sh
#
# Question: does image/layer extraction fail on a MAPPING wall that no
# amount of granted privilege clears, and does it mislead as a
# permission problem?
#
# Method: extract a REAL distribution rootfs (pinned Alpine minirootfs,
# whose /etc/shadow ships root:shadow = gid 42) as uid 0 in a user
# namespace whose map covers one gid. tar restores ownership by default;
# chown/lchown to an unmapped gid is the kernel answering EINVAL from
# the mapping check — not a policy, not a missing capability.
# The differential: --no-same-owner extracts clean, and the extracted
# tree's ownership differs from the image's — the mitigation is not free.
#
# Inputs: pinned minirootfs tarball. Tools: tar, sha256sum.
# Exit: 0 measured · 1 unexpected outcome · 2 could not run.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/25-ownership-wall.log"
{
  conditions ownership-wall
  echo "tar: $(tar --version | head -1)"
  echo
} > "$OUT"

vfetch minirootfs.tar.gz "$PIN_MINIROOTFS_URL" "$PIN_MINIROOTFS_SHA" >>"$OUT" 2>&1 || { echo "pin fetch failed" >> "$OUT"; exit 2; }
T="$SI_WORK/ownwall"; rm -rf "$T"; mkdir -p "$T/as-is" "$T/no-owner"

rc=0
{
  echo "## the artefact: a real rootfs tarball, pinned"
  echo "## (its etc/shadow ships root:shadow = gid 42, the classic first casualty)"
  tar -tzf "$SI_WORK/minirootfs.tar.gz" | grep -E "^etc/shadow" || true
  echo
  echo "## extract as-is (tar restores ownership by default)"
  tar -xzf "$SI_WORK/minirootfs.tar.gz" -C "$T/as-is" 2>&1 | head -4
  echo "as-is rc: ${PIPESTATUS[0]}"
  echo
  echo "## extract ownership-neutrally"
  tar -xzf "$SI_WORK/minirootfs.tar.gz" --no-same-owner --no-same-permissions -C "$T/no-owner" 2>&1 | head -2
  echo "no-owner rc: ${PIPESTATUS[0]}"
  echo
  echo "## the ownership the two trees actually carry (etc/shadow, etc/passwd):"
  stat -c "%n %U:%G (%u:%g)" "$T/as-is/etc/shadow" 2>/dev/null || echo "as-is/etc/shadow: absent (extraction aborted before it)"
  stat -c "%n %U:%G (%u:%g)" "$T/no-owner/etc/shadow" 2>/dev/null
  echo
  echo "## a synthetic tarball with an unmapped gid reproduces the wall exactly:"
  python3 - <<'EOF'
import tarfile, io, os
# craft an archive entry with gid 42 WITHOUT any host chown (tarfile writes
# the header fields directly — the wall is in the extraction, not the creation)
buf = io.BytesIO()
t = tarfile.open(fileobj=buf, mode="w")
for name, gid in (("etc/", 0), ("etc/shadow", 42)):
    ti = tarfile.TarInfo(name)
    ti.type = tarfile.DIRTYPE if name.endswith("/") else tarfile.REGTYPE
    ti.size = 0; ti.uid = 0; ti.gid = gid; ti.uname = "root"; ti.gname = "shadow"
    t.addfile(ti)
t.close()
open("/tmp/.ownwall-g42.tar", "wb").write(buf.getvalue())
EOF
  rm -rf "$T/g42"; mkdir -p "$T/g42"
  tar -xf /tmp/.ownwall-g42.tar -C "$T/g42" 2>&1 | head -3
  echo "synthetic rc: ${PIPESTATUS[0]}"
  echo
  echo "## the mitigation's cost: files the image says are root:shadow land as root:root."
  echo "## anything that CHECKS ownership (package managers, ssh) now sees a different tree."
} 2>&1 | tee -a "$OUT"

# gate: as-is must have failed, no-owner must have succeeded
if grep -q "as-is rc: 0" "$OUT"; then echo "UNEXPECTED: as-is extraction succeeded" | tee -a "$OUT"; rc=1; fi
if ! grep -q "no-owner rc: 0" "$OUT"; then echo "UNEXPECTED: ownership-neutral extraction failed" | tee -a "$OUT"; rc=1; fi
echo
echo "log: $OUT"
exit $rc
