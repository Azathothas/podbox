# podssh

Record semantics: [task rules](RULES.md#5-entry-closure).


This page holds the current SSH tasks. Main contains the transport,
interactive session, multiplexed relay, and remote and machine dispatch.
Pull requests 66 and 67 are closed. Their captured heads are historical inputs.

The source comparison and the two captured reference trees are in
[`docs/history/references/ssh-relay-2026-09-28.md`](../docs/history/references/ssh-relay-2026-09-28.md).
The current work order is [PROGRESS.md](PROGRESS.md).

---

### T-1401 Prove the SSH transport and server in the current tree

Source:      pull requests 66 and 67 at their captured heads; current SSH source
Category:    podssh
Priority:    P1
Effort:      L
Status:      partial

Problem:     Transport is implemented, but the restricted-server build input
             and fresh restricted-host session proof are incomplete.
Premise:     Main has the socketpair server, target parser, byte transport,
             and authenticated test sessions. The passwd shim is retained
             source. No workspace or release build compiles or links it.
             A normal loopback server does not prove the restricted host.
Approach:    Retain the landed transport and tests. Prove the selected server
             where chroot is denied and no passwd database exists. Make any
             required shim an actual compiled input with its notice. Add a
             fresh-clone drive for command output, exit 42, bounded failure,
             unchanged host identity, and cleanup.
Decision:    Keep SSH encryption and user checks in a real server. Keep an
             explicit server command. Do not repeat the completed relay or
             command dispatch. T-1406 owns relay liveness and reconnect.
Prove:       `cargo test -p podbox-ssh` and
             `sh scripts/common/check-gate.sh --strict` exit 0. A tracked
             fresh-clone drive must run a real SSH command and return 42 in
             the restricted host with the compiled server prerequisites and
             no owned residue.

**Partial 2026-09-30.** The audit reopened the entry from its actual build
paths. The earlier authenticated transport and relay results remain evidence
for their stated host and revision. The retained C file alone does not prove
that its server prerequisite is built or supplied.

[Earlier source and proof](../docs/history/audit-before-2026-09-30/TODO/podssh.txt)
retain the original scope and closure. The missing restricted-server clause
is the remaining work in this entry.

### T-1402 Provide an interactive session without a pty

Source:      captured sandssh and sandhome trees; crates/podbox-ssh/src/session.rs
Category:    podssh
Priority:    P1
Effort:      L
Status:      done

Problem:     A one-shot command does not prove an interactive session without a pty.
Premise:     A real terminal pair is unavailable on the target host.
Approach:    Use a server-side line discipline with echo, editing, history, shell state, and process-group signals. Test a real SSH client.
Decision:    Refuse unsupported terminal operations. Do not claim a real pty. T-1401 owns restricted-server prerequisites.
Prove:       `cargo test -p podbox-ssh` exits 0; `sh experiments/388-interactive-shell.sh` verifies editing and interruption without a pty device.

**Done 2026-09-28.** The session layer and shell executable are implemented.
The current workspace test run passes the interactive, relay, and real proxy
tests. The earlier live session and three mutation results are retained in
[the original record](../docs/history/audit-before-2026-09-30/TODO/podssh.txt).
The current full test result is [saved](../experiments/results/repo-audit-linux.txt).
This closure does not prove the remaining restricted-host clause in T-1401.

----

### T-1403 Prove concurrent sessions on one relay connection

Source:      captured reverse-v1 relay specification; crates/podbox-ssh/src/mux.rs
Category:    podssh
Priority:    P1
Effort:      L
Status:      done

Problem:     Relay frames must keep concurrent sessions paired and preserve byte order.
Premise:     The relay is a byte shuttle. A real SSH server owns authentication.
Approach:    Implement node and operator legs, frame parsing, session pairing, and refusal paths. Test two clients against the live relay.
Decision:    Keep the pair protocol separate from local paths. T-1406 owns DNS and write deadlines and live reconnect.
Prove:       `cargo test -p podbox-ssh` and `sh experiments/387-mux-two-client.sh` exit 0 with independent sessions and exit codes.

**Done 2026-09-28.** The relay implementation is on main. The original live
two-client result is [saved](../experiments/results/mux-two-client.txt).
The current workspace run passes the frame, relay, and transport tests in
[the Linux report](../experiments/results/repo-audit-linux.txt).
The earlier implementation and mutation record is retained in
[history](../docs/history/audit-before-2026-09-30/TODO/podssh.txt).
Unbounded DNS and writes and unproved reconnect remain in T-1406.

----

### T-1404 Add the remote and machine SSH verbs after the transport holds

Source:      captured pull requests 66 and 67; crates/podbox-cli/src/remote/mod.rs and machine.rs
Category:    podssh
Priority:    P2
Effort:      L
Status:      done

Problem:     Before this entry, main had neither SSH command dispatch path.
Premise:     Remote uses a supplied far server. Machine uses a guest that podbox starts.
Approach:    Add runnable remote serve, connect, and forward arms and the machine SSH arm. Keep helper discovery and bounded refusals explicit.
Decision:    Share transport, but keep server placement distinct. Advertise only implemented arms.
Prove:       `cargo test --workspace` exits 0; `sh experiments/389-remote-ssh.sh`, `sh experiments/390-machine-ssh.sh`, and `sh experiments/391-machine-bridge.sh` return their own success verdicts.

**Done 2026-09-29.** Both verbs and the static machine bridge are on main.
The earlier live drives checked command output, exit 42, bounded refusal,
and cleanup. The captured record preserves the serial startup finding and
the banner-hold and tty fixes in
[history](../docs/history/audit-before-2026-09-30/TODO/podssh.txt).
The current workspace test suite passes in
[the Linux report](../experiments/results/repo-audit-linux.txt).
The machine environment fault arms were not reached in the earlier live run.
Its one-second timeout proved a bounded refusal, without attributing the
deadline to one of the two concurrent waits. The tracked drivers provide
the next reproduction; ignored diagnostic files are not required inputs.

----

### T-1405 Ship and smoke the SSH helper archive

Source:      repository audit 2026-09-30; current source and saved results
Category:    podssh
Priority:    P1
Effort:      M
Status:      done

Problem:     The beta.9 release ships podbox without its required node, operator, and proxy helpers.
Premise:     The source build emits four SSH binaries. CLI discovery needs exact helper names.
Approach:    Stage all four in one archive for each release target.
             Smoke usage failures and ELF linkage. Sign the archive and
             publish its checksum.
Decision:    Keep the existing standalone podbox asset. Install helpers beside it or on PATH.
Prove:       `sh scripts/package-ssh.sh RELEASE_DIR ARCH QEMU` exits 0; the release matrix publishes every helper archive with its digest and signature.

**Done 2026-09-30.** The release matrix publishes all seven helper archives
with digests and signatures. `py experiments/399-publication.py --release
v0.1.0-beta.10` reports PUBLICATION-OK at build commit `d6cb926` with 7
architectures and 42 required non-empty assets
([the matrix proof](../experiments/results/publication.txt)).
`sh scripts/verify-release.sh v0.1.0-beta.10 x86_64 ssh` reports Verified OK
([the signature proof](../experiments/results/ssh-release-beta10.txt)).
Native packaging proof stays in
[the Linux result](../experiments/results/repo-audit-linux.txt).

**Partial 2026-09-30 (history).** Native packaging passes the static ELF and usage
checks. The missing-helper and invalid-ELF controls fail with their own
messages. The archive includes actual licence texts for the locked package
set. [The Linux result](../experiments/results/repo-audit-linux.txt) records
the proof. The release matrix and published signatures remain acceptance.

----

### T-1406 Bound SSH DNS and writes and prove node reconnect

Source:      repository audit 2026-09-30; current source and saved results
Category:    podssh
Priority:    P1
Effort:      L
Status:      open

Problem:     DNS resolution and stream writes can wait without a bound. Node redial pairing has no live reconnect proof.
Premise:     node, operator, and session comments state the unbounded paths. Earlier relay proof covers simultaneous sessions only.
Approach:    Add explicit operation deadlines without unbounded worker accumulation. Test a stalled resolver and non-reading peer. Drive reconnect with two sessions.
Decision:    Keep the completed transport work scoped. This entry owns the remaining liveness and reconnect acceptance.
Prove:       `cargo test -p podbox-ssh` exits 0 with the new fault tests.
             A tracked drive must also end each stalled operation within
             its bound, prove reconnect pairing, and check worker cleanup.

**Open 2026-09-30.** This is remaining capability work. Read the current
source before implementation. The simultaneous-session result does not
prove these clauses.
