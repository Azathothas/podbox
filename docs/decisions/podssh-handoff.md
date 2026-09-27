# podssh handoff

**Written 2026-09-27.** This is the page a later session reads first. It says
what is done, what is proved, what is not, and what is blocked on whom.

## What podssh is

`podbox remote ssh`, and the same code as a standalone `podssh`. It carries a
real `ssh` session to a machine that cannot be dialled, with **no custom
client at either end of the encrypted session**: the agent side is OpenSSH
and the far side is a stock ssh server. That is what removes the vendor lock-in
that gsocket and tailcat have, and it is why `scp`, `rsync`, `git` and every
editor that speaks ssh keep working without podssh knowing they exist.

Two shapes, and they are not the same thing:

- **rendezvous** (`serve` on the agent, `connect` on the operator). Two
  sandboxed machines meet through a relay. Neither listens on a port.
- **forward** (`forward`, and `connect` with no relay). Reach one fixed
  `host:port` through the egress. No rendezvous at all.

Every transport is a URL scheme: `tcp`, `tls`, `ws`, `wss`, `unix`, `exec`,
with an HTTP `CONNECT` proxy or SOCKS5 in front of any of them. `exec://` is
the escape hatch: any command that carries a byte pipe is a transport without
a line of podssh changing.

## Proved, with the artifact that shows it

| claim | where |
| --- | --- |
| 13 of 13 e2e cases, 0 failed, 0 skipped, in the reference cage | `crates/podbox-ssh/tests/e2e.sh` |
| four transports, each against a byte pipe AND a real `ssh` client through a real `dropbear -i` | same |
| three sandssh interop directions, sandssh tree unchanged | same |
| a full pubkey session over a rendezvous, in a cage with no pty, no `bind`, no `chroot`, no `/var`, no `/etc/passwd`, no UDP, no working resolver. ssh exit 0, read from the ssh process | `docs/decisions/ssh-server-in-a-cage.md` |
| a relay failover chain, with every refusal reported per hop | `crates/podbox-ssh/src/chain.rs` |
| 53 unit tests | `cargo test -p podbox-ssh` |

The reference cage is the reason most of this exists and its shape is worth
stating: uid 0, `/etc/passwd` absent, `/etc` read-only, no `/dev/ptmx`, no
`/dev/pts`, `bind(2)` denied with EACCES, `chroot(2)` denied, `unshare`
denied, all UDP refused, and egress through a CONNECT proxy whose port changes
every session.

## NOT proved, and a later session should not assume it

- **`scp` over the rendezvous was not run.** `scp` was proved over the relay
  path against a live ephemeral VM, and the rendezvous carries the same byte
  pipe, but the two facts have not been multiplied into one run. This is the
  cheapest open item in the file.
- **No pty anywhere.** The cage has no `/dev/ptmx` and neither host can create
  one, so a full-screen TUI cannot run. T-1402 and T-1403 are the open entries
  for the userspace line discipline that covers the interactive case.
- **The ajam relay has never been used.** `tcp.ssh.relay.ajam.dev` answers
  `403 relay: missing or wrong token` on every forward path and no token is
  in the tree. The transport, the `X-Relay-Token` header plumbing and the
  failover underneath it are all in place; a token is the only thing missing.
  See "Blocked" below.
- **TURN is not wired in.** The crate has no TURN client. `TURN` needs a
  credential no account here can mint, and `exec://` is the documented route
  once one exists. The catalog records the endpoints so a later session does
  not rediscover them.
- **The three deep review passes on this branch have not been run.** The work
  is committed and tested, not reviewed. That is the first thing a later
  session should do, and the three questions are in `TODO/RULES.md` section 10.

## Decisions that are settled and are not to be relitigated

- **The verb is `podbox remote ssh`.** `docs/decisions/remote-verb.md`. `remote`
  is a namespace, not a synonym: `remote fetch`, `remote download` and
  `remote wget` are the members that were named, and each would otherwise want
  its own top-level verb. `local` is the counterpart. `machine ssh` is
  podman parity and a **different axis**, so it stays separate. `podbox ssh` is
  refused with the group named and exit 125, and the e2e asserts both halves.
- **The ssh server a cage can run is dropbear, built dynamically.**
  `docs/decisions/ssh-server-in-a-cage.md`. `sshd` cannot run where chroot is
  denied, and no configuration changes that. A static dropbear cannot see the
  passwd database. `detect()` probes a candidate by starting it.
- **Relay ordering is by measured first-byte latency, not by discovery order.**
  0.67s, 0.76s, 4.34s to the same banner. The slowest was first in the first
  version of the list, which is a latency bug, not a style question.
- **A relay token travels in a header, not a query.** A query string is written
  to every access log on the way; a header is not. The spec parser takes
  `?header=Name:Value` and validates both halves, because a token is
  operator-supplied text that lands in a request line and a newline there
  would smuggle a second header in.
- **No token is committed anywhere.** It arrives in `PODSSH_RELAY_TOKEN`, and
  the operator's relay is tried LAST rather than first so a session an
  uncredentialed relay can serve never waits for a credential that may be
  absent.

## Blocked, and on whom

**One thing, and it is a token.** `tcp.ssh.relay.ajam.dev` needs a forward
token in `X-Relay-Token`. Without it every forward path answers 403 and
`/health?detail=1` still reports `"allow": "any public target"`, so the
service is live and the credential is the only gap. What to do when it
arrives:

```sh
export PODSSH_RELAY_TOKEN=...            # never committed, never logged
podbox remote ssh probe --json           # which relays answer
podbox remote ssh connect --relay "wss://tcp.ssh.relay.ajam.dev/connect/<host>/<port>" ...
```

The e2e's `PODSSH_E2E_SSHD` and the relay's `?family=`, `?path=`, `?dial=`
knobs are documented at `tcp.ssh.relay.ajam.dev/index.md`, which is fetched
over the egress proxy because DNS is dead in the cage.

## What a later session should do first

1. **Run the three deep review passes** over every file this branch touched.
   The questions are in `TODO/RULES.md` section 10 and they are three
   different questions, not one pass three times.
2. **Prove `scp` over the rendezvous**, which is one command against the
   existing `e2e.sh` and the last cheap unclosed claim.
3. **Get the token**, then run the transport matrix against the ajam relay and
   record the reading in `experiments/results/`.
4. **T-1402 and T-1403** are the interactive half and are the reason a
   first-class `ssh` experience is not finished: the transport carries a
   one-shot command today and the line discipline is what makes an operator's
   session feel native.
