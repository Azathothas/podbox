## The task

Session of 2026-09-21, continued. The operator ruled on T-1213 (build
entry yes, narrow fixture-only escape yes) and the unit is implemented:
helper gained `eng_build` + `eng_privrun`, 10/20/130 converted with no
assertion changed, 10 exits 0, 20 enters, 130 exits 1 on lane findings.
Work order is TODO/PROGRESS.md: T-1213 is blocked on findings, next is
T-0805/T-0808. Push straight to main, no branches. Work unattended; the
operator reads the result later.

## The resume point

Commit the T-1213 conversion (helper, three scripts, probe-parity
evidence, entry, counts, record), gate green, push to main. Then start
T-0805/T-0808: read both entries in full first.

## In flight (uncommitted, tree otherwise clean)

```text
M TODO/gate.md                    (T-1213 implemented + runs + findings)
M TODO/PROGRESS.md                (ruling settled, work order, counts)
M TODO/RESUME.md                  (this file)
M experiments/lib/engine.sh       (+eng_build, +eng_privrun, +_in_roots,
                                    +ENG_ENTRYPOINT, bare-ID pin)
M experiments/10-build-target-image.sh (converted; exit 0)
M experiments/20-enter-target.sh       (converted; enters, N+F+M)
M experiments/130-probe-parity.sh      (converted; exit 1, lane findings)
M experiments/results/probe-parity.txt (12 matched, 4 differed, 0 missing)
```

Counts hold at 35 open, 3 partial, 3 blocked, 96 done (no status moved).
Gate was green before this work (`py scripts/check-todo.py` exit 0
unpiped; full Linux check 9 passed, 0 failed, 2 environmental skips).
Re-run the host gate unpiped after these edits; the Linux half needs a
fresh run-in-base pass because engine.sh and three scripts changed.

## Standing traps (paid for, do not rediscover)

- `python3` is a Store stub; gate runs as `py scripts/check-todo.py`.
- Never export `MSYS_NO_PATHCONV` globally; per-call prefixes only.
- Read exit codes unpiped; `cmd | tail` reports tail's status.
- Do not run heavy engine jobs concurrently; a busy podman starves
  `engine_pick` into exit 2.
- Never edit a running script: the shell reads ahead by byte offset.
- `eng_serve` ids captured in `$( )` never reach `eng_cleanup`; remove
  fixtures by name in the trap.
- `eng_pbrun` execs `/pb` with the words it is given; a wrapper travels
  through `eng_run` with positional words.
- This lane's jq ends every raw-output line with CRLF; command
  substitution strips only the trailing one. Strip `tr -d '\r'` on
  multi-line streams.
- This lane's OpenSSL rejects its own system config; `req -x509` needs an
  empty config file at a `winpath` spelling with conversion off for the
  call, and native openssl reads no msys spelling.
- 20's stdout IS the payload channel: silence `engine_pick` there (the
  line rode into 130's rung capture). Conditions on stderr still name
  the driver.
- The Go harness builds for the image arch from the engine, never the
  host (host go targets windows here); the C probe builds on native
  lanes only.
- confine resolves no bare names (`20 -- id` dies, `20 -- /bin/id`
  works); every real caller passes absolute paths.
- Fixture tags are local names: 10/20 run the census and the
  reconstruction by image ID, which `_pinned` accepts as content.
- Guest artifacts: `.dev/artifacts/artifacts/{podbox,gnu.so,musl.so}`.
- shellcheck SC1007 on the `CDPATH= ` idiom is a pre-existing
  false-positive pattern across all scripts.
- Machine state: `podman-machine-default` Running (this session
  restarted it after the T-1212 stop). Base `wsl-toolkit-podbox`
  repaired and usable, with one honest non-fatal note: no cgroup
  delegation.
- `Dockerfile.target` opens with an unpinned `FROM golang:1.24.7-bookworm`
  stage. Pinning it changes the build input; carried as a wart, not a fix.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order: T-1213 is
blocked on lane findings, so T-0805/T-0808 are next. Push straight to
main and create no branches. Work unattended; the operator reads the
result later. The guest lane still has no docker daemon (no NET_ADMIN),
so engine clauses run on host podman.
```
