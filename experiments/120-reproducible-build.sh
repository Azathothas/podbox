#!/bin/sh
# Question: do two builds of one commit on one toolchain produce the same
# bytes, and does the binary report the inputs that produced it?
#
# TODO/packaging.md T-1004. The entry asks for reporting first
# (`version --verbose`: commit, toolchain, target, interposer digests,
# static mode) and a measured reproducibility verdict second. Where the
# bytes differ, the record names what differed rather than dropping the
# claim.
#
# The builds run in the lane (Linux, musl, zig): this host cannot link
# the target. The lane job travels as a heredoc below, so the measurement
# ships as one committed script; `run-in-base.sh` strips carriage
# returns from the staged job before it runs.
#
#   ./120-reproducible-build.sh
#
# Exit: 0 the bytes match, 1 they do not, 2 the builds could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/reproducible-build.txt"
WORK="$REPO/experiments/.sweep120-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT HUP INT TERM

# ⛔ No MSYS path-conversion exports here. This script hands /tmp paths
# to `run-in-base.sh`, and disabling conversion makes the lane resolve
# them against C:\tmp instead of this shell's /tmp (measured 2026-09-22:
# the wrapper arrived as C:\tmp\tmp.XXX\wrapper.lf.sh and the job
# exited 2). Nothing below calls a Windows binary with a path directly,
# so nothing needs the exclusion.

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host              $(uname -srm)"
echo "commit            $(git -C "$REPO" rev-parse HEAD 2>/dev/null || echo unknown)"
} >"$WORK/report"

command -v wsl-toolkit >/dev/null 2>&1 || {
	echo "lane              wsl-toolkit ABSENT" >>"$WORK/report"
	echo "verdict           COULD NOT RUN: no lane" >>"$WORK/report"
	cp "$WORK/report" "$OUT"
	exit 2
}

# The lane job. Two release builds of the same tree in one container, so
# the toolchain and the sources are identical by construction; only the
# build itself may differ. `clean -p` forces the crate (and build.rs)
# to rerun while the dependencies stay compiled.
cat >"$WORK/job.sh" <<'JOBEOF'
#!/bin/sh
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/build-interpose.sh || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 2
BIN=target/x86_64-unknown-linux-musl/release/podbox
[ -x "$BIN" ] || exit 2
mkdir -p /out || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha1
"$BIN" version --verbose > /out/verbose1 || exit 2
cp "$BIN" /out/podbox1 || exit 2
cargo clean --target x86_64-unknown-linux-musl -p podbox-cli || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 2
sha256sum "$BIN" | cut -d' ' -f1 > /out/sha2
"$BIN" version --verbose > /out/verbose2 || exit 2
cp "$BIN" /out/podbox2 || exit 2
echo "== lane builds done"
JOBEOF

if ! PODBOX_ARTIFACTS="$WORK/artifacts" sh "$REPO/scripts/windows/run-in-base.sh" "$WORK/job.sh" >"$WORK/lane.log" 2>&1; then
	echo "lane              job failed; error lines then tail:" >>"$WORK/report"
	grep -a -m5 -i "error\|FAILED\|panicked" "$WORK/lane.log" >>"$WORK/report"
	tail -15 "$WORK/lane.log" >>"$WORK/report"
	echo "verdict           COULD NOT RUN: lane build failed" >>"$WORK/report"
	cp "$WORK/report" "$OUT"
	exit 2
fi
for f in sha1 sha2 verbose1 verbose2 podbox1 podbox2; do
	if [ ! -f "$WORK/artifacts/$f" ]; then
		echo "lane              artifact $f missing" >>"$WORK/report"
		echo "verdict           COULD NOT RUN: incomplete artifacts" >>"$WORK/report"
		cp "$WORK/report" "$OUT"
		exit 2
	fi
done

{
echo ""
echo "== the record the binary reports"
cat "$WORK/artifacts/verbose1"
echo ""
echo "== build 1           $(cat "$WORK/artifacts/sha1")"
echo "== build 2           $(cat "$WORK/artifacts/sha2")"
} >>"$WORK/report"

if ! cmp -s "$WORK/artifacts/verbose1" "$WORK/artifacts/verbose2"; then
	echo "== the two binaries DISAGREE about their inputs" >>"$WORK/report"
fi
if cmp -s "$WORK/artifacts/podbox1" "$WORK/artifacts/podbox2"; then
	echo "verdict           MATCH: two builds, one byte stream" >>"$WORK/report"
	rc=0
else
	echo "== first differing bytes (offset, build 1, build 2)" >>"$WORK/report"
	cmp -l "$WORK/artifacts/podbox1" "$WORK/artifacts/podbox2" 2>/dev/null | head -5 >>"$WORK/report"
	echo "verdict           MISMATCH: the bytes differ, offsets above" >>"$WORK/report"
	rc=1
fi
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo "written to ${OUT#"$REPO"/}"
exit "$rc"
