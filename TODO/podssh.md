# podssh

This page holds the SSH work from podbox pull requests 66 and 67. Neither
pull request is in `main`. Use their captured heads
`fd461ee7` and `66b6fa10` when checking a claim. The full
identifiers are in the pull request records.
The second changes the first and is the candidate for the next pass.

The source comparison and the two captured reference trees are in
[`docs/history/references/ssh-relay-2026-09-28.md`](../docs/history/references/ssh-relay-2026-09-28.md).
The current work order is [PROGRESS.md](PROGRESS.md).

---

### T-1401 Prove the SSH transport and server in the current tree

Source:      podbox pull requests 66 and 67; sandssh at
             `4fc7f8cc`; dropssh at `0aafa21d`
Category:    podssh
Priority:    P1
Effort:      L
Status:      open

Problem:     podbox has no SSH transport on `main`. Pull request 67 adds one,
             but its current check set has three red jobs. Pull request 66
             conflicts with `main` and carries an earlier form of the same
             work. No SSH claim from either is a release claim yet.
Premise:     The pull request 67 source uses a byte stream under an SSH
             server and uses a real SSH client in its end-to-end test. Its
             proposed build needs a passwd shim file that the CI job did not
             find. The crate lint and document checks also fail. The current
             dropssh source has a separate, concurrent relay path; a simple
             pair test does not prove that path.
Approach:    Take pull request 67 as a source candidate. Reconcile its
             transport, server probe, test fixtures, and build script on
             `main`. Keep the SSH encryption and user check in a real SSH
             implementation. Make the passwd shim an explicit, tracked build
             input with its licence and link model checked. Fix the three red
             jobs and add a fresh-clone build. Use pull request 66 only for
             evidence of decisions that are absent from 67.
Decision:    Do not merge either head as it stands. A server started in a
             restricted host must be probed on the same socket shape it will
             receive in service. A dynamic server may use a libc shim; a
             static server cannot claim that the shim took effect.
Reconciled:  Two review passes read both heads at their recorded commits on
             2026-09-28. Pull request 67 is the base. It carries the
             socketpair server probe, the forward chain, the target parser,
             the cage resolver, the error table, the dropbear build, and the
             remote dispatch. Pull request 66 is an earlier subset with no
             check runs. It contributes no unique file. Three jobs are red on
             pull request 67. The cage job finds no tracked passwd shim. The
             lint job fails on a redundant boolean comparison. The same line
             misclassifies hosts with no dot in the name. The document job
             reports two shell-unsafe placeholders and four orphan pages. The
             committed end-to-end reading shows 12 cases under the old alias
             name. The head defines 13 cases. The rename cases have no
             committed run. Every case runs against the one-pair relay. No
             case proves the concurrent route. This session lands the
             transport and server without the relay and remote group, beside
             the machine arm. The relay and remote group stay deferred.
Prove:       `cargo test -p podbox-ssh` and
             `sh scripts/common/check-gate.sh --strict` exit 0, and a
             bounded end-to-end drive makes an SSH client run a command
             through a relay with the server on a socketpair.

### T-1402 Provide an interactive session without a pty

Source:      podbox pull request 67, proposed T-1402; sandssh at
             `4fc7f8cc`
Category:    podssh
Priority:    P1
Effort:      L
Status:      open

Problem:     A successful one-shot command does not make an interactive SSH
             session useful on a host with no pty. Echo, line editing, job
             signals, and command state need separate proof.
Premise:     The sandssh README now points to a separate sandhome tree for
             the line discipline; the captured sandssh tree does not carry
             that shell. The pull request 67 source has no server shell
             variant. Its end-to-end cases can pass while an interactive
             login is absent.
Approach:    Add a session layer above the byte transport. Drive a real SSH
             client through an interactive login without `/dev/ptmx`; assert
             echo, editing, history, state across commands, and a signal sent
             to the command process group. Keep the SSH protocol in the SSH
             server.
Decision:    The session layer must say which terminal operations it cannot
             support. Do not report a full pty when no pty exists. Reuse the
             server path proved by T-1401.
Studied:     Two terminal-pair tools were read at pinned commits on
             2026-09-28. faketty allocates two real pairs around the child.
             fakepty allocates one real pair and prints at exit. Both fail
             where no pair device exists. Both answer none of the five
             asserts. Both are refused as mechanisms. faketty contributes
             its test shape. fakepty contributes its trap catalogue. The
             session layer stays a server-side line discipline above the
             transport, with a refusal catalogue that names each unsupported
             operation.
Prove:       `cargo test -p podbox-ssh` exits 0, and a bounded interactive
             drive verifies editing and interruption through a real SSH
             client with no pty device.

### T-1403 Prove concurrent sessions on one relay connection

Source:      dropssh at `0aafa21d`;
             sandssh at `4fc7f8cc`;
             podbox pull request 67
Category:    podssh
Priority:    P1
Effort:      M
Status:      open

Problem:     The relay path in pull request 67 pairs one node and one client.
             It does not prove two independent operator sessions on one node
             connection. A second login can wait while the first is active.
Premise:     The captured dropssh `src/serve.c` has a session table and one
             relay reader; its `tests/mux-probe.py` exercises the frame
             direction and a malformed node frame. The captured sandssh
             Python relay pairs one node socket with one client and then
             splices it. These are two different protocols.
Approach:    Specify the relay protocol before changing the transport. Test
             the operator and node frame directions separately. Drive two
             concurrent authenticated SSH clients through one registered node
             connection, transfer data in both, then close one while the
             other continues. Bound all waits and prove cleanup.
Decision:    Interoperability must name the exact relay protocol and version.
             A passing test against the simple Python relay does not prove
             the multiplexed route.
Decided:     Both protocols stay, split by use (operator, 2026-09-28).
             The multiplexed reverse path serves remote use against the
             r12 relay: identifier-prefixed frames, exact close table,
             64 sessions, 64 KiB frames, 64 MiB sessions. The one-pair
             rendezvous serves local use and tests. Both pull requests
             speak the one-pair form today. dropssh proved two concurrent
             sessions live on 2026-09-28. Self-service pairing works from
             this host, measured on 2026-09-28: pair 200, connect-token
             status 200, node-token status 403, stop 200, status after
             stop 403. Tokens stay redacted in committed logs. What
             remains unmeasured: node redial pairing, which no document
             states.
Prove:       `cargo test -p podbox-ssh` exits 0, and a bounded two-client
             drive observes one node connection and two completed sessions.

### T-1404 Add the remote and machine SSH verbs after the transport holds

Source:      podbox pull request 67, proposed T-1404; [podvm.md](podvm.md)
             T-1302 and T-1304
Category:    podssh
Priority:    P2
Effort:      M
Status:      open

Problem:     The remote and machine sites have different server placement.
             The current `main` has neither SSH verb. Pull request 67
             dispatches remote SSH, but does not dispatch machine SSH.
Premise:     Pull request 67's `remote_group` calls the SSH crate and its
             parity table has a remote SSH row. Its help also names
             machine SSH, while its main dispatch has no matching arm.
             The document gate reports its decision page as orphaned.
Approach:    Bring the remote dispatch and parity row into `main` with
             T-1401, then add the machine arm after T-1403 passes.
             Probe the server at the selected far end. Prove each help path,
             one command, exit code, and error path by driving the built
             binary. Keep a real SSH client on the operator side.
Decision:    The remote and machine verbs share transport code but keep
             distinct server placement. Do not make a help row stand in for
             a runnable arm.
Scoped:      The remote half exists in pull request 67 as one dispatch
             arm, read at its head on 2026-09-28. The help text advertises
             relay, probe, local, and machine members with no arms. The
             operator defers the remote group with the relay. The machine
             arm lands now. Machine means podman parity: a shell in a guest
             that podbox itself runs.
Prove:       `cargo test --workspace` and
             `sh scripts/common/check-gate.sh --strict` exit 0, and both
             CLI paths drive a real command and report its exit code.
