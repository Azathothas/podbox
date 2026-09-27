# podbox-ssh

`podssh`: ssh between two machines that can each reach a rendezvous, where
neither machine has to listen on a port, and the ssh client on the operator
side is a real `ssh`.

```sh
# the agent. It dials out and registers; nothing listens here.
podssh serve --relay wss://relay.example:443 --name agent1 --auth "$SECRET"

# the operator. This is the whole ssh experience.
ssh -o "ProxyCommand=podssh connect --relay wss://relay.example:443 \
         --name agent1 --auth $SECRET" agent@agent1
```

`podbox ssh <command>` is the same binary behind the podbox verb, so it takes
the same subcommands and answers the same way.

## Why it is not a container trick

`sandssh` splices a byte pipe through a relay and runs `dropbear -i` under it;
`podbox` runs a payload on the host kernel and needs no daemon. `podssh` takes
the transport idea from the first and the "it is just a program" idea from the
second, and adds nothing that has to be installed on the far side:

- **No listening socket on the agent.** `serve` dials the relay and waits. A
  machine behind NAT, a firewall that allows only one direction, or a cage
  that forbids `bind` can still be reached.
- **No vendor.** A relay is a URL. `tcp://`, `tls://`, `ws://`, `wss://`,
  `unix://` and `exec://<command>` all work, as does an HTTP `CONNECT` proxy
  and SOCKS5 in front of any of them. `exec://` is the escape hatch: any
  command that carries a byte pipe (TURN, gsocket, `cloudflared`, `ssh -W`) is
  a transport without a line of podssh changing.
- **No new client.** The operator types `ssh`. `podssh connect` is a
  `ProxyCommand`, which is the mechanism ssh has always had for this.
- **No ssh server of its own.** `serve` starts whichever one the host has:
  `sshd -i` by default, an explicit `--server` when the host has something
  else, or `--forward host:port` to reach a server that is already running.

## Subcommands

| | |
|---|---|
| `probe` | what this machine can egress, and which candidate relays answer. `--json` for one document |
| `relay` | run the rendezvous. Two names pair; the relay never sees the ssh session's keys |
| `serve` | agent side: start an ssh server here and register with a relay |
| `connect` | operator side: the `ProxyCommand` |
| `forward` | reach one fixed `host:port` through the egress proxy, no relay and no agent |
| `config` | print an `~/.ssh/config` snippet for a name |
| `selftest` | the in-process protocol and framing checks |
| `version` | the version |

## The relay protocol

`sandssh`'s, so the two interoperate; `--protocol` selects it.

```
PODSSH1 <n|c> <name>\n<key>\n     ->  OK\n     (podssh1)
SANDSSH1 <n|c> <name>\n<key>\n    ->  OK\n     (sandssh1)
```

After `OK` the relay is a byte pipe. The relay does not terminate ssh, does
not hold a key, and has no session state beyond the two sockets it is splicing.

## Proof

`tests/e2e.sh` (driven by `experiments/369-podssh-e2e.sh`) runs the transport
matrix, each case against a byte pipe **and** against a real `ssh` client
through a real `sshd -i`, then three interop cases against the `sandssh` tree
unchanged: its relay under podssh on both ends, its client against podssh's
relay and server, and its server under podssh's relay and client. The committed
reading is `experiments/results/podssh-e2e.txt` (12 of 12, none skipped); the
public-relay reading is `experiments/results/podssh-egress.txt`.

## What it is not

- Not a relay network. Point it at one you run, or at any endpoint that speaks
  the protocol above.
- Not a substitute for host key verification. `--insecure` exists and is
  named; the default verifies TLS peers against the system roots.
- Not a way to reach a machine that can dial nothing at all. It removes the
  need to *listen*, not the need to *reach*.
