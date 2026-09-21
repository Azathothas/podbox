## The task

Session of 2026-09-21, continued. T-1213 is committed as 5cfe7ea and
pushed to main: helper gained `eng_build` + `eng_privrun`, 10/20/130
converted with no assertion changed, 10 exits 0, 20 enters, 130 exits
1 on lane findings. The commit gate failed three Linux runs on
store-suite contention (a different two_* victim each time; the slot
pool is process-wide and libtest shares it) before the fourth came
back fully green, with a 97/97 serial confirmation beside it. T-1213
stays blocked on lane findings. Next is T-0805/T-0808. Push straight
to main, no branches. Work unattended; the operator reads the result
later.

## The resume point

T-1213 is on main (5cfe7ea). Start T-0805/T-0808: read both entries
in full first. The podman machine is stopped at close-out; restart it
before engine work. The contention fix (serial store tests or scoped
slots) belongs to the T-0211/T-0215 family and is still unauthored.

## State (committed, tree clean)

```text
5cfe7ea  experiments: convert target-image pair and probe consumer
         to lib/engine.sh (T-1213, blocked) — pushed to main
```

Counts hold at 35 open, 3 partial, 3 blocked, 96 done (no status moved).
Host gate green (`py scripts/check-todo.py` exit 0 unpiped); Linux gate
green on the fourth attempt (EXIT:0 unpiped) after three contention
reds, all named in TODO/PROGRESS.md.

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
- Machine state: `podman-machine-default` stopped at close-out (this
  session found it stopped, ran the engine work, and stopped it again).
  Base `wsl-toolkit-podbox` repaired and usable, with one honest
  non-fatal note: no cgroup delegation.
- `Dockerfile.target` opens with an unpinned `FROM golang:1.24.7-bookworm`
  stage. Pinning it changes the build input; carried as a wart, not a fix.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
T-1213 is on main: eng_build + eng_privrun are in, 10/20/130 are
converted. This is a continuous session: work until manually stopped,
finishing as many entries as possible, spawning subagents where work
is independent. First verify CI on main (a red CI is the top
priority), then gc the kept wsl-toolkit job containers, then work the
full TODO/PROGRESS.md order in listed order starting at T-0805/T-0808
(read both entries in full first). The store-contention fix is
authorised: author the T-0211/T-0215-family entry, then implement it,
in its own change. Push straight to main and create no branches. The
guest lane still has no docker daemon (no NET_ADMIN), so engine
clauses run on host podman; the podman machine is stopped, restart it
before engine work.
```
