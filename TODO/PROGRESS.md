# Progress

## State

180 entries: 1 open, 1 partial, 0 blocked, 178 done.

T-1403 and T-1401 closed on the relay landing. The multiplexed
`reverse-v1` legs live in `crates/podbox-ssh` with the fake-relay
suite and the bounded two-client proof against the live relay, over
the transport T-1401 proved. Pull requests 66 and 67 are open;
neither is on `main`. What is left of them is in [podssh.md](podssh.md)
as T-1404. Pull request 67 is the base: pull request 66 is
an earlier subset with no check runs and contributes no unique file.
Pull request 67 carries three red jobs (cage build with no tracked
shim, one lint denial, one document denial), so it is not ready to
merge.

T-1402 closed on the session landing. The server-side line discipline
lives in `crates/podbox-ssh` (`session.rs`, the `shell` ForceCommand
server) with the fake-pty-free interactive suite and the bounded drive
against a real daemon. What is left of the two pull requests is the
T-1404 remote arm.

T-1112 is unblocked and still unproved. The host KVM node answers API
version 12, QEMU 11.1.1 with `qemu-img` installs from Arch extra, and the
ValidationOS disk is installed outside the tree with a matching digest.
What remains is the KVM guest run.

T-1112 remains partial. Its Windows guest arms ran under `tcg`; its
`kvm` arm needs a licensed image on a KVM host, under the entry's
accept-terms gate. The entry names the exact proof that will close it.

## Baseline and current lane

This session started 2026-09-28T19:37:38Z from `main` at `9d4bb55`,
with the T-1403 relay work implemented on disk and uncommitted. The
host is Windows
`MINGW64_NT-10.0-26200`. The selected lane is the
`wsl-toolkit-podbox` base, with kernel `7.2.0-WSL2-STABLE` and
`podman 6.1.2`. The base holds the image cache and the job records
across sessions. Its status reports no cgroup delegation, so a
memory or CPU limit accepted by the engine is not proof that the limit
was enforced.

`py scripts/check-todo.py` was green at the start: 171 entries, 0
open, 1 partial, 170 done. `sh scripts/common/check-gate.sh --fast`
was green: 10 passed, 0 failed, 1 skipped. On this Windows host,
`sh scripts/dev.sh status` answers `unknown`; use
`wsl-toolkit --instance podbox base status --probe` for the lane.
Run `sh scripts/windows/run-in-base.sh` for the Linux build and test
gate. [`docs/containers.md`](../docs/containers.md) holds the
current procedure and the installed manual owns the flag reference.

## This session

T-1403 landed the relay protocol split and the two-client proof. The
multiplexed `reverse-v1` legs live in `crates/podbox-ssh` (`ws.rs`
framing, `tls.rs` verify-always client, `mux.rs` node and operator with
the specification and its R12 line cites, `node` and `operator`
binaries). The one-pair rendezvous stays out of the crate for local
paths and tests.

The lane proved it three ways: `cargo test -p podbox-ssh` exits 0 (63
unit and binary tests, 14 relay tests with a fake relay, 3 proxy
end-to-end tests); three mutation breaks each turn their own test red
(session-id validation, relay-fed control id check, token charset);
`experiments/387-mux-two-client.sh` exits 0 against the live r12 relay
with pair 200, connect-token status 200 against node-token 403, one
node online, a 200000-byte exact round trip, exit 42 passthrough, the
first session surviving the others, silent client stderrs, stop 200
with status 403 after, 3 sessions closed, no token in the log, and the
two-clients verdict in
[`experiments/results/mux-two-client.txt`](../experiments/results/mux-two-client.txt).

Three review passes read the tree against itself (socket, error, and
exit-code sweep; guard-to-test audit; cite-and-clause audit). What
they found landed before the commit: usage lines on usage failures,
the `Lonely` once-exit-1 path, refused-apart-from-completed counting,
honest 1009-arm comments, three R12 cite corrections, and the gate
alphabet fixes (arrows to `->`, runtime-built id fixtures, the
ASCII-folded relay capture with two markdown links). What they found
that stays as a stated limit: writes and DNS resolve without a
timeout, so a relay that stops reading wedges the loop; frame sizes
rest on read sizing with the chunking loops as defense-in-depth; node
redial pairing stays unmeasured.

T-1401 closed on the landing: no transport change was needed, and its
named trio (lane suite, host strict gate, socketpair relay drives)
holds. One commit carries T-1403, T-1401, and the record.

T-1402 landed the interactive session without a pty. The discipline
(`crates/podbox-ssh/src/session.rs`) echoes, edits, recalls capped
history, keeps state in the supervised shell, signals the shell's
process group, and prints a static prompt; the `shell` binary serves
the session under `ForceCommand` and refuses exec requests naming
`SSH_ORIGINAL_COMMAND` with exit 125. The lane proves it four ways:
`cargo test -p podbox-ssh` exits 0 (82 lib, 12 binary, 14 relay, 12
interactive, 3 proxy end-to-end tests); three mutation breaks each turn
their own test red (line cap, CR-LF swallow, group-kill sign);
`experiments/388-interactive-shell.sh` exits 0 against a real daemon
with ten asserting clauses and the interactive verdict in
[`experiments/results/interactive-shell.txt`](../experiments/results/interactive-shell.txt);
the host strict gate passes 11 checks with no skip. Three review
passes read the tree (guard-to-test, needle-soundness, honesty audit)
and their findings landed: the live-stdin teardown, the idle-loop
sleep, the narrowed refusal catalogue, computed output markers, the
trap-handler selective kill, teardown and drain gap tests, and the
untrapped-130 drive clause. What the reviews refused stays stated as a
limit in the module docs, not as a gap in the proof.

## Verification

The lane suite is green on the committed tree: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and
`cargo test -p podbox-ssh` (82 lib, 12 binary, 14 relay, 12
interactive, 3 proxy end-to-end tests) all exit 0. Three session
mutations each turn their own test red with the tree restored
byte-identical after. The live relay drive
`experiments/387-mux-two-client.sh` exits 0 at 2026-09-28T20:04:16Z
with the two-clients verdict and no token in the log. The live session
drive `experiments/388-interactive-shell.sh` exits 0 at
2026-09-28T22:25:02Z with the interactive verdict and no secret in the
log. The host strict gate passes 11 checks with no skip. The base
cleanup report is empty after the last drive. The beta.9 release head
`05d153a` passes all four hosted gate jobs, and issue 68 is closed.

## Work order

1. T-1402: done on the session landing, proof above.
2. T-1404: add the remote SSH arm after the relay holds. The machine
   dispatch and refusal are landed; the positive path needs a guest SSH
   endpoint.
3. T-1112: run the KVM guest. The host node, QEMU 11.1.1, and the
   installed disk are all measured; the accept-terms gate from the entry
   still applies.

To completion: the T-1404 remote arm and the T-1112 guest run close in
that order, each with parallel review passes and the full
gate green before it commits. Release prep follows the packaging
entries (version, changelog, signed artefacts). The operator
authorized the closing acts on this repository: pull requests 66 and 67
close as superseded once their work is on `main`, and the beta tags
and publishes. No session touches any other repository; that boundary
stands.

## Open questions

Podbox speaks both relay protocols, split by use: multiplexed reverse
for remote paths, one-pair rendezvous for local paths. Sessions may mint
ephemeral self-service tokens and commit redacted logs. T-1403 records
the tests that settle the remainder, including node redial pairing.
T-1112 still needs the KVM guest run named in its entry. Writes and DNS
have no timeout on the relay legs; a relay that stops reading wedges
the loop, which the entry states as a limit.
