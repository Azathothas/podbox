#!/bin/sh
# Question: does the shipped binary answer issue 60's curated docker
# surface row by row -- every one of the 46 `run` flags and 10 verbs
# with the reason from its parity row, and the promoted flags with real
# behavior?
#
# TODO/cli.md T-0801 (drives the issue-60 rows through the binary).
#
# Clauses over the table read out of the binary itself:
#   1. every curated flag the table refuses: `run --rm <flag>` exits
#      with the flag-error code and names status None on stderr
#      (`--device` is not among them: it maps since T-0501, clause 5);
#   2. every curated verb: `podbox <verb>` exits with the runtime-error
#      code and prints the verb row's note;
#   3. the promoted flags end to end against the pinned image below:
#      `--env-file` sets variables (later `-e` wins), `--label`,
#      `--attach` and `--expose` are admitted stubs, `--log-driver`
#      takes `json-file` and refuses any other driver naming it, and
#      `--strict` refuses the stubs naming them;
#   4. `create` inherits the same surface through run's parser;
#   5. `--device` shapes: it maps since T-0501 (the serve is 363's),
#      and a malformed shape is a flag error naming it.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and only clause 3 needs a registry. On a Windows host run
# it inside the base with the checkout as the working directory.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep355-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-355.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "image input       $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain"
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
echo "podbox            $("$PB" version)" >>"$REPORT"

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$PB" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
FLAG=$PODBOX_EXIT_FLAG_ERROR
RUN_ERR=$PODBOX_EXIT_RUNTIME_ERROR

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"
fail=0
refused=0
admitted=0

# Clause 1: the 40 refused flags. Each must exit with the flag-error
# code and name status None, before any image work happens.
# (`--log-driver` is not among them: it takes `json-file`, clause 3.
# `--device` is not among them either: it maps since T-0501, clause 5.)
for f in --blkio-weight --cgroup-parent --cidfile --cpu-period \
    --cpu-quota --cpu-shares --cpuset-cpus --detach-keys \
    --device-cgroup-rule --disable-content-trust --dns --dns-option \
    --dns-search --domainname --gpus --group-add --health-cmd --init \
    --ipc --isolation --link --log-opt --mac-address \
    --mount --oom-kill-disable --pid --pids-limit --read-only --runtime \
    --security-opt --shm-size --stop-signal --stop-timeout --sysctl \
    --tmpfs --ulimit --userns --uts --volume-driver --volumes-from; do
	out="$WORK/refusal.txt"
	timeout 120 "$PB" run --rm "$f" "$ALPINE" true >"$out" 2>&1
	rc=$?
	if [ "$rc" -eq "$FLAG" ] && grep -q "status None" "$out"; then
		refused=$((refused + 1))
	else
		echo "FAIL: run $f exited $rc (want $FLAG with status None):" >>"$REPORT"
		sed 's/^/  /' "$out" >>"$REPORT"
		fail=1
	fi
done
echo "clause-1 refused-with-reason  $refused/40" >>"$REPORT"

# Clause 2: the 10 verbs. Each must exit with the runtime-error code
# and print its row's note.
for v in manifest node plugin scan secret service stack trust checkpoint config; do
	out="$WORK/verb.txt"
	timeout 120 "$PB" "$v" >"$out" 2>&1
	rc=$?
	if [ "$rc" -eq "$RUN_ERR" ] && grep -q "podbox: $v: " "$out"; then
		admitted=$((admitted + 1))
	else
		echo "FAIL: verb $v exited $rc (want $RUN_ERR with its note):" >>"$REPORT"
		sed 's/^/  /' "$out" >>"$REPORT"
		fail=1
	fi
done
echo "clause-2 verbs-refused        $admitted/10" >>"$REPORT"

# Clause 3 needs the image. A pull that fails is the lane, not the
# binary: exit 2, exactly as 354 treats an unbuilt lane.
if ! timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1; then
	echo "pull FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi

printf '# comment\n\nFOO=from-file\nBAR=spaced value\nQ="quoted"\n' >"$WORK/t.env"
printf 'FOO=from-file\nBAR=spaced value\nQ="quoted"\nNOEQUALS\n' >"$WORK/bad.env"

step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 120 "$@" >"$out" 2>&1
	rc=$?
	echo "step $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "output:"
	sed 's/^/  /' "$out"
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { echo "step $name: wanted exit $want, got $rc" >>"$REPORT"; fail=1; }
}

# --env-file sets variables through the payload's own stdout.
step env-file-stdout 0 "$PB" run --rm --env-file "$WORK/t.env" "$ALPINE" sh -c 'echo $FOO'
grep -q "from-file" "$WORK/out-env-file-stdout.txt" || { echo "env-file value missing from payload stdout" >>"$REPORT"; fail=1; }
# A later -e wins over the file: the load is positional.
step env-file-override 0 "$PB" run --rm --env-file "$WORK/t.env" -e FOO=cli "$ALPINE" sh -c 'echo $FOO'
grep -q "^cli$" "$WORK/out-env-file-override.txt" || { echo "-e did not win over --env-file" >>"$REPORT"; fail=1; }
# The = form reads the same file.
step env-file-equals 0 "$PB" run --rm "--env-file=$WORK/t.env" "$ALPINE" sh -c 'echo $BAR'
grep -q "spaced value" "$WORK/out-env-file-equals.txt" || { echo "= form misread the file" >>"$REPORT"; fail=1; }
# A line without = and a missing file are flag errors, never silent env.
step env-file-badline "$FLAG" "$PB" run --rm --env-file "$WORK/bad.env" "$ALPINE" true
step env-file-missing "$FLAG" "$PB" run --rm --env-file "$WORK/absent.env" "$ALPINE" true
# The stub trio is admitted and runs.
step stub-label 0 "$PB" run --rm --label k=v "$ALPINE" true
step stub-attach 0 "$PB" run --rm --attach "$ALPINE" true
step stub-expose 0 "$PB" run --rm --expose "$ALPINE" true
# --strict refuses the stubs naming them.
step strict-label "$RUN_ERR" "$PB" run --rm --strict --label k=v "$ALPINE" true
grep -q "Stub" "$WORK/out-strict-label.txt" || { echo "--strict did not name the Stub status" >>"$REPORT"; fail=1; }
# --log-driver takes json-file (both spellings) and refuses the rest.
step log-driver-json 0 "$PB" run --rm --log-driver json-file "$ALPINE" true
step log-driver-json-eq 0 "$PB" run --rm --log-driver=json-file "$ALPINE" true
step log-driver-syslog "$FLAG" "$PB" run --rm --log-driver syslog "$ALPINE" true
grep -q "takes json-file" "$WORK/out-log-driver-syslog.txt" || { echo "--log-driver refusal did not name the value" >>"$REPORT"; fail=1; }

# Clause 4: create is served by run's parser, so it inherits the rows.
step create-env-file 0 "$PB" create --name c355 --env-file "$WORK/t.env" "$ALPINE" true
step create-label 0 "$PB" create --name c355b --label k=v "$ALPINE" true
step create-refused "$FLAG" "$PB" create --name c355c --read-only "$ALPINE" true
step rm-c355 0 "$PB" rm c355
step rm-c355b 0 "$PB" rm c355b

# Clause 5: `--device` maps since T-0501 (driven by 363), so it is not
# refused; but a malformed shape is a flag error naming it, before any
# image work happens.
step device-shape "$FLAG" "$PB" run --rm --device=/dev/zero:/a:/b:c "$ALPINE" true
grep -q "more than two colons" "$WORK/out-device-shape.txt" || { echo "--device shape refusal did not name the shape" >>"$REPORT"; fail=1; }
step device-perms "$FLAG" "$PB" run --rm --device=/dev/zero:/g:rx "$ALPINE" true
grep -q "perms take r, w and m only" "$WORK/out-device-perms.txt" || { echo "--device perms refusal did not name the perms" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           CURATED SURFACE COVERED"; else echo "verdict           CURATED SURFACE OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/parity-curated.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
