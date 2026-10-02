# Resume

Session 2026-10-02, decision round. Tree `main` at `b9ed3ac`, pushed,
clean apart from this session's own record change. Record gate exits 0
on the landed tree: 220 entries, 6 open, 3 partial, 0 blocked, 211
done.

This session asked the operator about every open, pending, or blocked
item the previous session reported, settled all ten, and wrote the
answers into the record. It implemented none of the settled work.
Seven entries are open and unstarted.

## Read before acting

`TODO/RULES.md` section 11 carries four standing decisions the
operator made on 2026-10-02, and section 2 carries the read-write
grant. They change how the work is done, not only what it is, and
they settle the questions this round was asked about:

1. Build compatibility is not a goal. Users take a published binary.
   Take whatever toolchain feature unlocks the task, nightly included.
   Fix a toolchain rejection; do not ask for a pin.
2. Dead code is a fault of the reader until proven otherwise. Never
   delete a field to quiet a lint.
3. No deferrals. Work needing a human becomes a tracked task with a
   clearing condition. Batch for a later queue, then finish it.
4. Read and write on this repository is authorized. Not another
   repository, not a force-push.

Do not ask these again. They are answered.

## Next action

T-1601, in `TODO/interpose.md`. Rust 1.99 rejects the `open`
interposition at `crates/podbox-interpose/src/lib.rs:2144` with a
deny-by-default lint, and that failure stops the static build, the
interposer tests, and plants 32e and 32f with it. The entry says what
to do: allow the lint at the definition, forward-compatible through
`unknown_lints`. Do not pin `rust-toolchain.toml`.

Then T-1602, T-1603, and T-1604, which clear the other three red CI
jobs. Then T-1607, T-1606, and T-1605.

## What changed this session

No source. Seven new entries, all `open`: T-1601 through T-1607. Two
live documents rewritten: the CI remainder section of
`TODO/PROGRESS.md` and this file. `TODO/RULES.md` gained the section 11
decisions and the read-write grant in section 2.

## Measured state

| row | evidence |
| --- | --- |
| record gate | `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0, 220 rows |
| counts | `podbox-count` wrote 220 items: 6 open, 3 partial, 0 blocked, 211 done |
| CI | run 36894578988 at `b9ed3ac`, all four jobs red; causes read off that run's own log |
| secrets check | `sh scripts/common/check-no-secrets.sh --public` exits 1 on `scripts/dev-lane.sh:205`, reproduced locally |
| clippy | `registry.rs:30`, four unread fields, on the `parallel_layers` test target |
| tests | `detached_stdio` 1 passed, 2 failed; `:138` is a PoisonError cascade from `:292` |
| host release build | not available here: `.cargo/config.toml` sets the musl target, which needs `scripts/zig-cc.sh` |

## Untracked working material

`refactor/` stays untracked on purpose (`.gitignore:147`).
`refactor/DEFERRALS.md` is superseded: its four sections now live in
tracked entries. Do not treat it as the record.

The plan's unbatched rows are 21, not the 22 both planning documents
claim. T-1607 records the corrected figure and owns the queue.

## Known open work

| id | subject | blocking |
| --- | --- | --- |
| T-1601 | 1.99 interpose lint | needs a Linux lane run to re-verify |
| T-1602 | registry fixture dead code | needs `acquisition.rs` and `store_digest.rs` |
| T-1603 | remove the secrets check, add trufflehog | touches 7 call sites; needs a new pinned workflow |
| T-1604 | detached stdio cascade | needs a Linux lane run |
| T-1605 | live version tag proof | authorized; no operator action needed |
| T-1606 | repoint 47 `references/` citations | blocks the corpus deletion |
| T-1607 | file 21 unbatched rows | mechanical |

T-1350 and T-1112 stay `partial` on one condition no decision removes:
an operator-present KVM session accepting host risk. Do not start a
KVM guest unattended.

## Next prompt

```
Continue podbox from TODO/RESUME.md. Read TODO/RULES.md section 11
before acting; four standing decisions from 2026-10-02 bind this work.
Take T-1601 from TODO/interpose.md and implement it. The record gate
must exit 0 before you close.
```