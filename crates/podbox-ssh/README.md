# podbox-ssh

`podssh` carries a real `ssh` session to a machine that cannot be dialled.
The client on the operator side is a real OpenSSH and the server on the far
side is a real ssh server, so **nothing custom terminates the encrypted
session**. That is what removes the vendor lock-in, and it is why `scp`,
`rsync`, `git`, `sshfs` and every editor that speaks ssh keep working
without podssh knowing they exist.

It is reached as `podbox remote ssh` or as the standalone `podssh`. The two
are one implementation: `crates/podbox-ssh/src/cli.rs` holds the verbs and
both entry points dispatch to it, so a `ProxyCommand` written against one
works against the other.

## Two shapes, and they are not the same thing

A relay is one of two things, and a tool that has only one of them is half a
tool.

**A rendezvous.** Two named peers each dial the relay and it pairs them.
Neither side listens on a port. This is `podssh relay` (or the
`sandssh-relay.py` it interoperates with) plus `serve` on the agent and
`connect` on the operator.

**A forward.** The relay dials the target itself and the client only has to
reach the relay. There is nobody to pair with, so `--name` and `--auth` have
no meaning. This is `forward --relay wss://host/connect/<host>/<port>`.

```sh
# a rendezvous, nothing listening anywhere
podssh serve --relay wss://relay.example:443 --name agent1 --auth "$SECRET"
ssh -o "ProxyCommand=podssh connect --relay wss://relay.example:443 \
         --name agent1 --auth $SECRET" root@agent1 'uname -a'

# a forward, through a relay that dials the target itself
ssh -o "ProxyCommand=podssh forward --relay wss://relay.example/connect/host/22 \
         --target host:22" root@host 'uname -a'
```

## Subcommands

| | |
|---|---|
| `probe` | what this machine can egress, and which candidate relays answer. `--json` for one document |
| `relay` | run the rendezvous. Two names pair; the relay never sees the ssh keys |
| `serve` | agent side: start an ssh server here and register with a relay |
| `connect` | operator side: the `ProxyCommand` for a rendezvous |
| `forward` | reach one fixed `host:port`, optionally THROUGH a relay transport |
| `config` | print an `~/.ssh/config` stanza, so `ssh <name>` works with nothing hand-written |
| `selftest` | the in-process protocol and framing checks |
| `version` | the version |

## Transports

A relay is a URL and the scheme names the transport: `tcp://`, `tls://`,
`ws://`, `wss://`, `unix://` and `exec://<command>`. An HTTP `CONNECT` proxy
and SOCKS5 can sit in front of any of them, and a cage's egress proxy is
found in the environment.

`exec://` is the escape hatch and it is the one to reach for first when
something new has to work: **any command that carries a byte pipe is a
transport without a line of podssh changing.** TURN, `cloudflared`, `gsocket`
and `ssh -W` are all `exec://` and none of them needs a podssh release.

```sh
# TURN, or anything else, with no podssh change at all
ssh -o "ProxyCommand=podssh forward --relay 'exec://gsocket -k on' --target host:22" root@host
```

## An authenticated relay

A relay that needs a credential takes it in a **header**, not a query string,
because a query string is written to every access log on the way and a header
is not. The spec carries it as `?header=Name:Value`, and both halves are
validated: the name must be an RFC 7230 token and the value may not carry a
newline, because a token is operator-supplied text that lands in a request
line and a newline there smuggles a second header in.

```sh
# Self-service for tcp.ssh.relay.ajam.dev. Tokens expire, at most 72h out.
export PODSSH_RELAY_TOKEN=$(curl -sS -X POST -d '{}' \
  https://tcp.ssh.relay.ajam.dev/v1/mint | sed 's/.*"token":"\([^"]*\)".*/\1/')
ssh -o "ProxyCommand=podssh forward \
  --relay \"wss://tcp.ssh.relay.ajam.dev/connect/host/22?header=X-Relay-Token:$PODSSH_RELAY_TOKEN\" \
  --target host:22" root@host
```

⛔ **A token is a credential and no token is in this repository.** It arrives
in `PODSSH_RELAY_TOKEN` or in the `?header=` on the spec, and
`.gitignore` names the patterns that keep it out. The operator's relay is
tried **last** in the candidate order, so a session an uncredentialed relay
can serve never waits for a credential that may be absent.

## The ssh server, and why it is not sshd

`serve` starts whichever ssh server the host has. On an ordinary machine
`sshd -i` is right. On a machine that forbids `chroot(2)` it cannot work at
all, and the failure is quiet enough to mislead:

| step | result |
| --- | --- |
| `sshd -i -t -f <config>` | **exits 0** |
| `sshd -i` at runtime | `Privilege separation user nobody does not exist` |
| with a `nobody` entry | `Missing privilege separation directory: /var/chroot/ssh` |
| with `ChrootDirectory` moved | no effect, a different hardcoded path |
| with `UsePrivilegeSeparation no` | deprecated and **ignored** since OpenSSH 8.4 |

So `detect` **starts** each candidate on a throwaway socketpair rather than
trusting that it is on PATH, and reports what it said. The probe is the
smallest thing that is true: a server that exits inside the window is a
failure and its first line is the diagnosis, a server still running is a
pass, and it does **not** establish that the server speaks ssh.

What works where `sshd` cannot is **dropbear, built dynamically**, and the
builder is `scripts/build-dropbear.sh`:

```sh
./scripts/build-dropbear.sh dist/dropbear
# dropbear, dropbearkey, fakepwd.so, SHA256SUMS, BUILDINFO
```

Two measured facts make it that way, and `BUILDINFO` says both:

- a **statically linked** dropbear carries its own libc, so `LD_PRELOAD`
  cannot reach it, its passwd lookups fail against a cage with no
  `/etc/passwd`, and it logs `Login attempt for nonexistent user` for a
  user that is there;
- a seccomp-filtered cage **denies `setgroups(2)`**, so dropbear's
  `initgroups()` always fails and it exits on every login. `setgid` stays
  **fatal**, because `setgid` is the call that changes the group and its
  failure is a real privilege problem; a denied `initgroups` leaves a
  session with correct uid and gid and merely no supplementary groups.

`.github/workflows/dropbear.yml` builds it, asserts the three properties that
fail silently, and then **proves a real session** through the artefact on
every run, because a binary that compiles and cannot log anyone in is the
failure this exists to prevent.

## What this is not

- Not a relay network. Point it at one you run, or at any endpoint that
  speaks a transport above.
- Not a substitute for host key verification. `--insecure` exists and is
  named; the default verifies TLS peers against the system roots.
- Not a way to reach a machine that can dial nothing at all. It removes the
  need to **listen**, not the need to **reach**.
- Not interactive yet, and this is the honest limit. A cage has no
  `/dev/ptmx` and neither host can create one, so a full-screen TUI cannot
  run. `TODO/podssh.md` T-1402 and T-1403 are the userspace line
  discipline that covers the interactive case; a one-shot command works now.

## Where to read next

- `docs/decisions/remote-verb.md` -- why the verb is `podbox remote ssh`.
- `docs/decisions/ssh-server-in-a-cage.md` -- the five-step `sshd`
  measurement, and what was not proved.
- `docs/decisions/podssh-handoff.md` -- **start here**: what is proved, what
  is not, and what is blocked on whom.
- `TODO/podssh.md` -- the entries, T-1401 done and T-1402 to T-1404 open.
