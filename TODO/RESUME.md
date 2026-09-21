## The task

Session of 2026-09-21. Work order is TODO/PROGRESS.md: close T-1212, then
T-1213 (10, 20, 130) as one conversion unit without changing what any script
asserts. Push straight to main, no branches. Work unattended; the operator
reads the result later. Engine clauses run on host podman (6.1.2, Windows
host, Git Bash); the guest lane still has no docker daemon (no NET_ADMIN).

## The resume point

T-1212 closed **blocked** and committed. Next is T-1213: read
`experiments/10-build-target-image.sh`, `experiments/20-enter-target.sh`
(needs a daemon today: exits 2 at line 39 without one) and
`experiments/130-probe-parity.sh:1-40`, then convert the three as one unit
through `experiments/lib/engine.sh`. Expect 130 to SKIP without a daemon;
that is a finding, never an edit to the rows.

## In flight (committed, tree clean, gate green)

```text
M TODO/gate.md                        (T-1212 blocked + six runs + repairs)
M TODO/PROGRESS.md                    (this session's record)
M TODO/RESUME.md                      (this file)
M TODO/INDEX.md                       (36 open, 3 partial, 2 blocked, 96 done)
M experiments/150-image-acquisition.sh   (converted last half; exit 1, F1)
M experiments/270-multiarch-image.sh     (converted last half; exit 1, F2)
M experiments/280-insecure-registry.sh   (converted + exit 0, all clauses)
M experiments/300-run.sh                 (converted; exit 2, cl5+cl7 SKIP)
M experiments/320-cli-contract.sh        (converted; exit 2, cl5 SKIP)
M experiments/330-exit-codes.sh          (converted; exit 2, docker half SKIP)
M experiments/lib/engine.sh              (unchanged this half)
A experiments/lib/tcpfwd.c               (new; driver-side TCP forward)
M experiments/results/cli-contract.txt   (320's green run)
M experiments/results/exit-codes.txt     (330's table-half run)
M experiments/results/image-acquisition.txt (150's run: digests_match no)
M experiments/results/insecure-registry.txt (280's 7-clause green run)
M experiments/results/multiarch-image.txt   (270's run: clauses 1-4 green)
M experiments/results/run.txt            (300's run, cl5+cl7 SKIP)
```

Gate: `py scripts/check-todo.py` exits 0 read unpiped. Linux half
(`sh scripts/windows/run-in-base.sh`) still owes a run in this change;
run it before the push if the base is up, else say so.

## Standing traps (paid for, do not rediscover)

- `python3` is a Store stub; gate runs as `py scripts/check-todo.py`.
- Never export `MSYS_NO_PATHCONV` globally; per-call prefixes only.
- Read exit codes unpiped; `cmd | tail` reports tail's status.
- Do not run heavy engine jobs concurrently; a busy podman starves
  `engine_pick` into exit 2.
- Never edit a running script: the shell reads ahead by byte offset, so a
  one-byte edit mid-run corrupts the run. Measured 2026-09-21 (`say` lost
  its `s` at 300's line 194; the run was repeated clean).
- `eng_serve` ids captured in `$( )` never reach `eng_cleanup`; remove
  fixtures by name in the trap.
- `eng_pbrun` execs `/pb` with the words it is given; a wrapper travels
  through `eng_run` with positional words.
- This lane's jq ends every raw-output line with CRLF; command
  substitution strips only the trailing one. Strip `tr -d '\r'` on
  multi-line streams before the bytes reach an assertion or a file.
- This lane's OpenSSL rejects its own system config; `req -x509` needs an
  empty config file at a `winpath` spelling with conversion off for the
  call, and native openssl reads no msys spelling (`/tmp`, `/c/...`).
- Guest artifacts: `.dev/artifacts/artifacts/{podbox,gnu.so,musl.so}`.
  `x280/x300/x320/x330-run*.log` are ignored scratch (`*.log`); delete
  them at the end.
- shellcheck SC1007 on the `CDPATH= ` idiom is a pre-existing
  false-positive pattern across all scripts; new helper lines are clean.
- Machine state at handoff: `podman-machine-default` Running (this
  session started it from stopped; stop it at the end if T-1213 does not
  need it). Base `wsl-toolkit-podbox` repaired (stale boot id) and
  usable, with one honest non-fatal note: no cgroup delegation, so a
  memory or cpu limit is accepted and not enforced.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order: T-1213 converts
10-build-target-image.sh, 20-enter-target.sh and 130-probe-parity.sh as one
unit through experiments/lib/engine.sh without changing what any script
asserts. Push straight to main and create no branches. Work unattended; the
operator reads the result later. The guest lane still has no docker daemon
(no NET_ADMIN), so engine clauses run on host podman and 130 will SKIP
without a daemon. The lock-table suite may need more than one pass.
```
