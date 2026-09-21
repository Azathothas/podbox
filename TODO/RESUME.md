## The task

Session of 2026-09-21, continued. T-1212 is committed and pushed
(`1d9ed91`). T-1213 analysed without touching a script: `20` needs
`--privileged` (helper refuses it by design) and `10` needs a build
entry the helper does not have. Work order is TODO/PROGRESS.md: T-1213
is blocked on an operator ruling, next is T-0805/T-0808. Push straight
to main, no branches. Work unattended; the operator reads the result
later.

## The resume point

Commit the T-1213 `blocked` state (entry, counts, record), gate green,
push to main. Then start T-0805/T-0808: read both entries in full first.
The open operator question (helper build entry + privileged escape)
is asked in the final message and recorded in TODO/PROGRESS.md; do not
implement either without the ruling.

## In flight (uncommitted, tree otherwise clean)

```text
M TODO/gate.md       (T-1213 blocked + three routes + clearer)
M TODO/PROGRESS.md   (counts 35/3/3/96, work order, open question)
M TODO/RESUME.md     (this file)
M TODO/INDEX.md      (35 open, 3 partial, 3 blocked, 96 done)
```

Gate was green at the T-1212 push (`py scripts/check-todo.py` exit 0
read unpiped; full Linux check 9 passed, 0 failed, 2 environmental
skips). Re-run both after these edits, unpiped, before the push.

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
- Guest artifacts: `.dev/artifacts/artifacts/{podbox,gnu.so,musl.so}`.
- shellcheck SC1007 on the `CDPATH= ` idiom is a pre-existing
  false-positive pattern across all scripts.
- Machine state: `podman-machine-default` Running (this session started
  it from stopped; stop it when no entry needs host podman). Base
  `wsl-toolkit-podbox` repaired and usable, with one honest non-fatal
  note: no cgroup delegation.
- T-1213's wall: `20-enter-target.sh:134` `--privileged` vs the helper's
  `_no_priv` refusal; `10-build-target-image.sh:20` `docker build` vs no
  build entry. Both need the operator, neither needs rediscovery.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order: T-1213 is
blocked on an operator ruling (helper build entry plus privileged
escape), so T-0805/T-0808 are next after the ruling or in parallel with
it. Push straight to main and create no branches. Work unattended; the
operator reads the result later. The guest lane still has no docker
daemon (no NET_ADMIN), so engine clauses run on host podman.
```
