# Session summary, 2026-10-02

The second session of 2026-10-02. It took the whole open work order to the
end of the batch and stopped where a human's next decision is not needed.

## What moved

| Entry | Before | After | Evidence |
| --- | --- | --- | --- |
| T-1601 | open | done | CI run 36974640619, static-build job green |
| T-1602 | open | done | lane clippy 0, 38 suites green on 1.99 |
| T-1603 | open | done | CI run 36974640686, scanner ran; scope corrected |
| T-1604 | open | partial | CI run 36974640619 named the exit |
| T-1605 | open | partial | tag `v0.1.0-beta.13` pushed; nightly ran red |
| T-1606 | open | partial | map repointed, corpus untouched |
| T-1607 | open | partial | 17 filed, the other 29 not |
| T-1608 | open | done | `check-one-home.sh` exits 0 |
| T-1611 | open | done | `check-docs.sh` exits 0 at 1136 links |
| T-1612 | open | done | two reopened, two limits recorded |
| T-1613, T-1621 through T-1640 | new | filed, one done | 250 rows, gate exit 0 |
| T-1641 | new | open | KVM diagnosis, three defects |
| T-1350 | partial | partial | three guarded runs, all red |

Counts: 226 rows became 250. The record went from 10 open to 23 open, 9
partial, 0 blocked, 218 done, and the gate exits 0.

## The two commits

- `1a87b10` Clear the CI remainder and file the unbatched plan rows.
  48 files, +2217 / -624.
- `8dbad80` Read CI's own log: the tag fired, and three findings own the
  reds. 9 files, +261 / -40.

Tag `v0.1.0-beta.13` on `1a87b10`. `TODO/RULES.md` section 2 grants the
push; no force, no history rewrite.

## Measured, on this host

| Row | Evidence |
| --- | --- |
| record gate | `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exit 0, 250 rows |
| counts | `podbox-count.exe` exit 0, 250 items: 23 open, 9 partial, 0 blocked, 218 done |
| lane, 1.99 | `experiments/results/ci-lint-1.99.txt`: fmt 0, fmt-interpose 0, clippy 0, test 0, 38 passing suites, zero `FAILED` |
| interposer, 1.99 | `experiments/results/interpose-1.99-objects.txt` exit 0: musl 350184 bytes, glibc 330264, 112 exports each |
| toolchain | host `rustc 1.98.0`; the lane is `rustc 1.99.0` |
| CI | run 36974640619 on `1a87b10`: static build green, fmt and clippy green, two jobs red |
| secrets scan | run 36974640686: 34993 chunks, 375 MB, 19.0 s, `verified_secrets: 0`, `unverified_secrets: 122` |
| KVM, guarded | `experiments/results/kvm-guest-2026-10-02.txt` and `-third.txt`, three attempts |
| maintained checks | one-home 0, docs 0, markers 0, placeholders 0, twins 0, attribution 0 |
| retained jobs | `wsl-toolkit gc --json` empty at session close |

## Three defects this session found that are not its own work

1. **`scripts/release-licenses.py` searched two fixed layouts.** The
   nightly builds each leg into `target/<triple>/release/`, so all six
   legs exited 2 with the shim's own "not built" message. Fixed, and the
   triple list is checked against the nightly matrix rather than kept in
   step by hand. This is why T-1605 is partial rather than done.
2. **`crates/podbox-cli/tests/store_gates.rs:169`** tripped a 1.99
   `clippy::bool_comparison` deny and was the only clippy error left in
   the tree. Fixed to the negation.
3. **The T-1604 plant's control failed.** A deliberately failing test
   left its neighbours green with the fix in place, and the control that
   restored `unwrap()` did not reproduce the cascade, because a thread
   already parked on a poisoned mutex is handed the lock. The green is
   not evidence and is not claimed.

## Three the record still owns

T-1641 holds the KVM findings: `setup` writes into the vendor's backing
image with no read-only overlay; a run that reaches its timeout orphans
an emulator that neither the proof's selector nor the watchdog can
select; and the proof's seam step swallows a non-zero exit so a failing
seam never records a miss. T-1641 also corrects
`experiments/lib/kvm-guest-base.sh:158-161`, which claims the serial
watcher says where a hang stops. It cannot: the 2026-09-30 third run has
the same BDS-only serial ending and its `ver` had already succeeded.

## What this session could not do

- The KVM guest run does not pass. `windows setup` completes on the same
  firmware and the same command line; `run ver` hangs after the firmware
  handoff. T-1641 owns the next step, not this session.
- `scripts/build-interpose.sh` resolves `target/release/`, which the
  musl build target never fills, so it exits 2 in a lane and would in CI.
  The nightly works around it by copying the binary by hand. It predates
  this batch and has no entry of its own yet; it is named in T-1601.
- Plants 32e and 32f were not run. The harness links the crate, which
  this Windows host cannot do.
- The secrets workflow has run once, red, and the corrected scope has not
  run yet. Its own log is in T-1603.