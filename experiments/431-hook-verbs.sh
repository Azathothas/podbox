#!/bin/sh
# Question: do the podvm calls of the four container-hook commands of the
# official runner (a job with `container:` and one container action) pass
# on the userland rung, where chroot is denied?
#
# TODO/cli.md T-1711 (issue 236; the hook contract of the podrunner SPEC,
# sections 7.2 and 10.1, its U19). podvm adds no hook: this replays the
# calls that podrunner's host side makes, as podvm verbs.
#
# Fixture: the chroot-denying wrapper of experiment 356 (unshare -Urm and
# deny-chroot). Pinned input: alpine 3.20 of mirror.gcr.io, a job image
# that differs from the runner's (PODVM_431_IMAGE overrides).
#
# Clauses:
#   1. `run -d --name job --entrypoint sleep` with the seven mounts of the
#      job-container table (/__w, /__w/_temp, /__w/_actions, /__w/_tool,
#      /__e read-only, /github/home, /github/workflow) from directories
#      under a runner root: exit 0.
#   2. a step: `exec -e K=V -w /__w/repo job sh -e /__w/_temp/step.sh`
#      sees K and its workdir, and its write to /__w lands in the runner
#      root.
#   3. a write to /__e fails with EROFS ("Read-only file system").
#   4. a container action: `run --rm` with the five mounts of the
#      container-action table and `-w /github/workspace` reads the
#      workspace, and its exit code is its own (7).
#   5. `logs job` holds the output of the entrypoint; `wait` of a detached
#      `sh -c 'exit 5'` gives 5.
#   6. the time of 20 `exec` steps is printed.
#   7. `rm -f job` leaves no process of the job.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep431-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/hook-verbs.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMG="${PODVM_431_IMAGE:-mirror.gcr.io/library/alpine:3.20}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
trap 'rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" || { echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF

timeout 600 "$PB" pull "$IMG" >"$WORK/pull.log" 2>&1 || { echo "pull FAILED: COULD NOT RUN" >&2; tail -2 "$WORK/pull.log" >&2; exit 2; }

# The runner root and its directories.
R="$WORK/runner"
mkdir -p "$R/w/repo" "$R/w/_temp/_runner_file_commands" "$R/w/_actions" "$R/w/_tool" "$R/externals" "$R/home" "$R/workflow"
echo "from-the-checkout" >"$R/w/repo/README"
cat >"$R/w/_temp/step.sh" <<'EOF'
echo "K=$K"
echo "pwd=$(pwd)"
echo "step" > /__w/repo/step.out
EOF

# The sequence runs in one shell inside the fixture: each call's exit code
# and output go to a file of its own.
cat >"$WORK/seq.sh" <<EOF
PB="$PB"; IMG="$IMG"; R="$R"; W="$WORK"
JOB="-v \$R/w:/__w -v \$R/w/_temp:/__w/_temp -v \$R/w/_actions:/__w/_actions -v \$R/w/_tool:/__w/_tool -v \$R/externals:/__e:ro -v \$R/home:/github/home -v \$R/workflow:/github/workflow"
ACT="-v \$R/w/repo:/github/workspace -v \$R/home:/github/home -v \$R/workflow:/github/workflow -v \$R/w/_temp/_runner_file_commands:/github/file_commands -v \$R/w/_temp:/__w/_temp"
\$PB run -d --name job --entrypoint sh \$JOB "\$IMG" -c 'echo job-up; exec sleep 9431' >"\$W/c1.out" 2>&1; echo \$? >"\$W/c1.rc"
\$PB exec -e K=V -w /__w/repo job sh -e /__w/_temp/step.sh >"\$W/c2.out" 2>&1; echo \$? >"\$W/c2.rc"
\$PB exec job sh -c 'echo x > /__e/w.txt' >"\$W/c3.out" 2>&1; echo \$? >"\$W/c3.rc"
\$PB run --rm \$ACT -w /github/workspace "\$IMG" sh -c 'cat README; exit 7' >"\$W/c4.out" 2>&1; echo \$? >"\$W/c4.rc"
\$PB logs job >"\$W/c5.out" 2>&1; echo \$? >"\$W/c5.rc"
\$PB run -d --name short "\$IMG" sh -c 'exit 5' >/dev/null 2>&1
\$PB wait short >"\$W/c5w.out" 2>&1; echo \$? >"\$W/c5w.rc"
\$PB rm -f short >/dev/null 2>&1
t0=\$(date +%s%N)
i=0; while [ \$i -lt 20 ]; do \$PB exec -w /__w/repo job true >/dev/null 2>&1 || echo "step \$i failed" >>"\$W/c6.err"; i=\$((i + 1)); done
t1=\$(date +%s%N)
echo \$(( (t1 - t0) / 1000000 )) >"\$W/c6.ms"
\$PB rm -f job >"\$W/c7.out" 2>&1; echo \$? >"\$W/c7.rc"
sleep 1
ps -eo pid,args | grep -c '[s]leep 9431' >"\$W/c7.left"
EOF
timeout 900 unshare -Urm sh "$WORK/wrap.sh" "$WORK" /bin/sh "$WORK/seq.sh" >"$WORK/seq.log" 2>&1

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note "image             $IMG"
note ""
rc() { cat "$WORK/$1.rc" 2>/dev/null || echo none; }

note "== clause 1  run -d with the seven mounts of the job container"
note "  exit $(rc c1)"
[ "$(rc c1)" = 0 ] || { bad "run -d exited $(rc c1)"; sed 's/^/    /' "$WORK/c1.out" | tail -3 | tee -a "$OUT"; }
note ""

note "== clause 2  a step with -e and -w"
note "  exit $(rc c2): $(grep -E '^(K|pwd)=' "$WORK/c2.out" | tr '\n' ' ')"
[ "$(rc c2)" = 0 ] || bad "the step exited $(rc c2)"
grep -q "^K=V$" "$WORK/c2.out" || bad "the step does not see K"
grep -q "^pwd=/__w/repo$" "$WORK/c2.out" || bad "the step is not in /__w/repo"
[ -f "$R/w/repo/step.out" ] || bad "the write to /__w did not land in the runner root"
note ""

note "== clause 3  /__e is read-only"
note "  exit $(rc c3): $(head -1 "$WORK/c3.out")"
[ "$(rc c3)" != 0 ] || bad "the write to /__e succeeded"
grep -q "Read-only file system" "$WORK/c3.out" || bad "the write to /__e did not fail with EROFS"
[ ! -f "$R/externals/w.txt" ] || bad "a file landed in the externals"
note ""

note "== clause 4  a container action"
note "  exit $(rc c4): $(grep -v '^podvm' "$WORK/c4.out" | head -1)"
[ "$(rc c4)" = 7 ] || bad "the action exited $(rc c4), not 7"
grep -q "from-the-checkout" "$WORK/c4.out" || bad "the action did not read /github/workspace"
note ""

note "== clause 5  logs and wait"
note "  logs: exit $(rc c5): $(head -1 "$WORK/c5.out"); wait: exit $(rc c5w): $(head -1 "$WORK/c5w.out")"
grep -q "job-up" "$WORK/c5.out" || bad "logs does not hold the output of the entrypoint"
grep -qx 5 "$WORK/c5w.out" || bad "wait did not give 5"
note ""

note "== clause 6  the time of 20 exec steps"
note "  20 steps: $(cat "$WORK/c6.ms" 2>/dev/null || echo -) ms$( [ -s "$WORK/c6.err" ] && echo "; $(wc -l <"$WORK/c6.err") failed")"
[ -s "$WORK/c6.err" ] && bad "a timed step failed"
note ""

note "== clause 7  rm -f leaves no process"
note "  exit $(rc c7); processes of the job left: $(cat "$WORK/c7.left" 2>/dev/null || echo -)"
[ "$(rc c7)" = 0 ] || bad "rm -f exited $(rc c7)"
[ "$(cat "$WORK/c7.left" 2>/dev/null)" = 0 ] || bad "a process of the job stayed"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: EACH CALL OF THE HOOK SEQUENCE PASSES ON THE USERLAND RUNG"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
