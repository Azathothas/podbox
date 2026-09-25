#!/bin/sh
# Question: which of the 22 open issues reproduce on this lane, with what
# command, output and exit code?
#
# TODO triage entry T-1337 (authored beside this script): the 2026-09-25
# triage of open issues 29-38 and 49-60 plus dependabot PR 9. Each clause
# below names its issue, runs the cited command with an isolated
# PODBOX_STORE, and prints the output with the exit code read from the
# process that produced it, never through a pipe.
#
# The binary is lane-built here (cargo build, debug): the issues were
# measured against beta.7, and this drive checks the same commands on the
# current tree. Pinned inputs: the alpine digest below (same bytes as
# experiments/163-ladder-drive.sh) and the rust image the lane runs.
#
#   sh experiments/353-open-issue-triage.sh
#
# Exit: 0 the drive completed and the report is written, 1 the binary
# could not be built, 2 the lane could not run (no network, no toolchain).
set -u

# The job travels inside the workspace as /work/.podbox-job.sh, so $0
# names the staging path, not experiments/. The wrapper already sits in
# /work: take the checkout from the working directory instead. A job
# script that resolves paths from its own $0 builds at / and fails.
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep353-work"
rm -rf "$WORK"; mkdir -p "$WORK/out" || exit 2
REPORT="$WORK/out/triage-353.txt"
PROBE_JSON="$WORK/out/probe.json"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "rustc             $(rustc --version 2>&1)"
echo "cargo             $(cargo --version 2>&1)"
echo "image input       $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain (zig, cc, musl target)"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc zig tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
echo "== build the debug binary"
if timeout 1800 cargo build >>"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/build.log"
	exit 1
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podbox"
[ -x "$PB" ] || { echo "binary missing after build: FAILED" | tee -a "$REPORT"; exit 1; }
echo "podbox            $($PB version 2>&1)" >>"$REPORT"
echo "podbox verbose    $($PB version --verbose 2>&1 | tr '\n' ' ')" >>"$REPORT"

run_case() {
	name="$1"; shift
	store="$WORK/store-$name"
	mkdir -p "$store" || exit 1
	out="$WORK/out-$name.txt"
	PODBOX_STORE="$store" timeout 120 "$@" >"$out" 2>&1
	rc=$?
	echo "clause $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "store             isolated $store"
	echo "exit              $rc"
	echo "output:"
	sed 's/^/  /' "$out"
	} >>"$REPORT"
}

# Clause PULL: one pull for every image-dependent clause below.
run_case "pull" "$PB" pull "$ALPINE"

# Issue 29: a windows guest is refused by name, nothing fetched.
run_case "29-platform" "$PB" run --platform windows/amd64 "$ALPINE" cmd /c ver

# Issue 30: chroot gate on run, memfd force, create, start.
run_case "30-probe-chroot" "$PB" probe --json
cp "$WORK/out-30-probe-chroot.txt" "$PROBE_JSON" 2>/dev/null || true
run_case "30-run" "$PB" run --rm "$ALPINE" echo hi
PODBOX_MODE=memfd run_case "30-memfd" "$PB" run --rm "$ALPINE" echo hi
run_case "30-create" "$PB" create --name triage30 "$ALPINE" true
run_case "30-start-missing" "$PB" start triage30

# Issue 31: ladder rungs report through the gate on this lane.
PODBOX_MODE=rundir run_case "31-rundir" "$PB" run --rm "$ALPINE" echo hi
PODBOX_MODE=bogus run_case "31-bogus" "$PB" run --rm "$ALPINE" echo hi
run_case "31-verbose" "$PB" version --verbose

# Issues 32/33/34/35/36: probe legs read back from the saved JSON.
{
echo ""
echo "== probe-legs (issues 32-36)"
python3 - "$PROBE_JSON" >>"$REPORT" 2>&1 <<'EOF'
import json, sys
raw = open(sys.argv[1]).read()
try:
    doc = json.loads(raw)
except Exception as e:
    print("probe json unreadable: %s" % e)
    sys.exit(0)
def walk(d, prefix=""):
    if isinstance(d, dict):
        for k, v in d.items():
            walk(v, prefix + "/" + k)
    else:
        print("  %s = %r" % (prefix, d))
census = doc.get("census") or doc.get("Census") or {}
print("census rows:")
walk(census)
for key in ("tiers", "Tiers", "non_goals", "nonGoals"):
    if key in doc:
        print("%s:" % key)
        walk(doc[key])
EOF
} || echo "python3 probe parse failed" >>"$REPORT"

# Issue 35: machine tier entry spellings.
run_case "35-tier-machine" "$PB" run --podbox-tier=machine --rm "$ALPINE" echo hi
run_case "35-podvm" "$PB" podvm triage35 run --rm "$ALPINE" echo hi

# Issue 37a: parity rows for logs base and -f.
run_case "37-parity" "$PB" system info --format '{{json .Parity}}'
run_case "37-logs-help" "$PB" logs --help

# Issue 37b: -t refusal ordering against the chroot gate.
run_case "37-tty" "$PB" run --rm -t "$ALPINE" true

# Issue 38: doctor, df, tail/since absent from help.
run_case "38-system-help" "$PB" system --help
run_case "38-logs-help" "$PB" logs --help

# Issue 54: logout parity row, login help.
run_case "54-logout" "$PB" logout example.com
run_case "54-login-help" "$PB" login --help

# Issue 55: --device has no parity row.
run_case "55-device-run" "$PB" run --rm --device /dev/urandom "$ALPINE" true
run_case "55-device-exec" "$PB" exec --device /dev/urandom triage30 true

# Issue 56: --log-driver has no parity row.
run_case "56-log-driver" "$PB" run --rm --log-driver json-file "$ALPINE" true

# Issue 59: probe rung against entered rung.
run_case "59-rung" "$PB" probe
run_case "59-entered" "$PB" system info --format '{{.Rung}} {{.EnteredRung}}'

# Issue 60: sample of the missing flags and verbs.
run_case "60-read-only" "$PB" run --rm --read-only "$ALPINE" true
run_case "60-mount" "$PB" run --rm --mount type=tmpfs,destination=/tmp "$ALPINE" true
run_case "60-env-file" "$PB" run --rm --env-file /dev/null "$ALPINE" true
run_case "60-manifest" "$PB" manifest inspect "$ALPINE"
run_case "60-service" "$PB" service ls

# Lifecycle loop where the lane permits entry (issues 19-21 shape).
run_case "lifecycle-run" "$PB" run --rm "$ALPINE" echo lane-hi
run_case "lifecycle-create" "$PB" create --name triageLC "$ALPINE" true
run_case "lifecycle-start" "$PB" start triageLC
run_case "lifecycle-exec" "$PB" exec triageLC echo exec-hi
run_case "lifecycle-ps" "$PB" ps
run_case "lifecycle-logs" "$PB" logs triageLC
run_case "lifecycle-stop" "$PB" stop triageLC
run_case "lifecycle-rm" "$PB" rm triageLC

{
echo ""
echo "verdict           DRIVE COMPLETE"
} >>"$REPORT"

if [ -d /out ]; then cp "$WORK/out/triage-353.txt" /out/ 2>/dev/null || true; fi
cat "$REPORT"
exit 0
