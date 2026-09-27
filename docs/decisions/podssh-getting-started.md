# podssh, from zero to a session

A walk from nothing to a working `ssh` into a machine that cannot be dialled.
Every step here was run, and the reading that proved each one is named.

## 0. What you need, and what you do not

You need: podbox, a relay you can reach, and an ssh server on the far side.
You do **not** need: a port forwarded, anything listening on the agent, a VPN,
or a client that is not OpenSSH. If a plan involves any of those, it is the
wrong plan.

## 1. See what this machine can do

```sh
podbox remote ssh probe                  # one JSON document, on stdout
podbox remote ssh probe --json           # the same; the flag is explicit
```

⛔ **`probe` PRINTS JSON AND NOTHING ELSE, AND THAT IS THE WHOLE OUTPUT.** The
first version of this guide showed it as a table, which was written before the
verb was run, and the table does not exist. `--json` is accepted and is the
default rather than a switch, so a script can pass it either way.

This is the step that saves the most time, because the two failure modes look
identical from the outside and are not the same. A cage with no `/etc/passwd`
and a cage with a dead resolver both produce "ssh hangs", and the fix for one
is a shim and for the other is a resolver.

The probe answers, separately:

| row | what it means when it says `denied` |
| --- | --- |
| `system-resolver` | the libc resolver cannot answer; the DoH resolver is used instead |
| `egress-proxy` absent | there is no ambient proxy, so only direct egress is possible |
| `bind-listener` | `bind(2)` is refused, which is why nothing here listens |
| `pty` | no `/dev/ptmx`, so a full-screen TUI cannot run on this side |

⛔ **A row that says `skip` was not measured and is not a failure.** The
difference matters: `denied` is the kernel saying no, and `skip` is podssh
saying it could not ask. A tool that draws those as the same thing teaches
its reader to retry something that cannot change.

## 2. Mint a relay token, if the relay needs one

`tcp.ssh.relay.ajam.dev` is self-service:

```sh
export PODSSH_RELAY_TOKEN=$(curl -sS -X POST -d '{}' \
  https://tcp.ssh.relay.ajam.dev/v1/mint \
  | tr -d '\n' | sed 's/.*"token" *: *"\([^"]*\)".*/\1/')
```

⛔ **`tr -d '\n'` IS NOT OPTIONAL.** The response is pretty-printed, and `sed`
matches line by line, so without it the pattern never matches on the line that
carries the token and the substitution silently passes the WHOLE BODY
through. Measured: the extraction above returned a 159-character string
beginning `{` before the `tr` was added, and a 92-character token after. A
command substitution that quietly yields the wrong thing is worse than one that
fails, because the next command reports a 403 and names the token as wrong.

`jq -r .token` is the alternative where `jq` exists, and
`python3 -c "import json,sys; print(json.load(sys.stdin)['token'])"` is the
one that needs nothing but the interpreter most cages already have.

The token expires, at most 72 hours out. ⛔ **It is a credential: keep it in
the environment, never in a repository.** `PODSSH_RELAY_TOKEN` is the
intended way in, and `.gitignore` covers the two files that a manual
`curl > file` would create.

A relay you run yourself needs no token:

```sh
podssh relay --listen 0.0.0.0:8443        # on any host with a public address
```

## 3. Get a session to something that is already listening

The quickest proof, and it needs no agent at all:

```sh
podssh forward --relay "wss://tcp.ssh.relay.ajam.dev/connect/ssh.github.com/443" \
               --target ssh.github.com:443
```

That is a raw byte pipe: whatever you type goes to port 443 of that host.
`--expect-banner SSH-2.0-` makes it refuse to splice onto a service that is
not what you asked for, which matters because a relay that answers and then
serves you something else looks like a transport fault and is not.

## 4. Reach a machine that cannot be dialled

The agent side, on the machine you want into. It dials out and waits:

```sh
podssh serve --relay wss://relay.example:443 --name agent1 --auth "$SECRET"
```

`sshd -i` is started on the relay's bytes and **nothing listens on a port**.
Where `chroot(2)` is denied that server cannot be `sshd` at all, so:

```sh
./scripts/build-dropbear.sh dist/dropbear     # from a podbox checkout
dist/dropbear/dropbear -i -E -s -g -F -r hostkey -D akdir
```

and point `--server` at it. The operator then runs the whole ssh experience:

```sh
ssh -o "ProxyCommand=podssh connect --relay wss://relay.example:443 \
         --name agent1 --auth $SECRET" root@agent1
```

## 5. Make it a name, so nothing is hand-written

```sh
podssh config --name agent1 --relay wss://relay.example/connect/host/22 --user root
```

⛔ **THE FLAGS ARE `--name` AND `--relay`, NOT POSITIONAL ARGUMENTS.** The
first version of this step said `podssh config agent1 agent1.host.example 22`,
which was written from the shape of `ssh` rather than from the verb, and it
fails with `--name is required` on a name it was just given. Every verb here
takes flags.

The stanza it prints has an **absolute** path to the binary, because `ssh`
runs a `ProxyCommand` through a login shell whose `PATH` is not yours and a
bare `podssh` is the most common reason a generated config does not work.
That was a real defect, not a documentation nicety: the generator emitted a
bare name, and it was found by pasting the stanza this guide tells a reader
to paste.

The stanza also picks the VERB from the relay's shape, because a relay is
one of two things. A relay whose path is `/connect/<host>/<port>` dials the
target itself, so the stanza says `forward --target <host>:<port>`; anything
else is a rendezvous and the stanza says `connect --name <name>`. Getting that
wrong produced a stanza that failed with `--target is required`, naming a flag
the reader never typed.

From then on:

```sh
ssh agent1                       # exec
scp ./file agent1:/app/          # files
rsync -a ./dir agent1:/app/      # and this
```

A generated stanza for the operator's relay, carrying a real session, is in
the branch's proof: `ssh -F <stanza> vm 'id -un'` against a live ephemeral VM
returns `root` and exits 0.

## 6. When it does not work, the error says which hop

Every refusal is reported by relay and in its own words, because "all relays
failed" is the least useful sentence a transport can produce:

```
podssh: chain: every relay failed:
  egress proxy 169.254.169.1:44099 [refused] answered 403 not on the egress allowlist
  relay wss://tcp-1.../connect/a/22 [protocol] answered 502 Bad Gateway
  relay wss://tcp-3.../connect/a/22 [auth] relay: missing or wrong token
```

Read it top down. The first refusal is usually the informative one: a `403`
from the egress says the POLICY is the wall, and a `502` from a relay says
that relay's upstream is unreachable, which is a different problem with a
different fix.

## The four failures worth knowing by name

Each was measured, and each looks like something else.

1. **Exit 255 with no output at all.** The session never started. Run
   `probe`; the answer is in there.
2. **"Connection timed out during banner exchange".** The tunnel opened and
   the server's greeting never arrived. Almost always a relay that answered
   and went quiet, which the chain handles by trying the next one; if it
   survives every relay, suspect the target.
3. **"Permission denied (publickey)" from a server that should accept you.**
   The server cannot resolve the ACCOUNT. A cage has no `/etc/passwd`, so
   the name is not there even though the key is fine, and the message says
   nothing about the account. `LD_PRELOAD=./fakepwd.so` is the fix.
4. **A relay that worked yesterday and not today.** Shared public relays are
   perishable: one of the best on 2026-09-26 was dead within three hours.
   This is not a bug and it is why `probe` re-measures rather than trusting a
   list.

## Where this stops, honestly

A **full-screen TUI cannot run** in a cage, because there is no `/dev/ptmx`
and nothing in userspace can create one. One-shot commands, `scp` and
`rsync` are the covered cases today. `TODO/podssh.md` T-1402 and T-1403 are
the userspace line discipline for the interactive case, and they are the first
thing to pick up if you are continuing this.
