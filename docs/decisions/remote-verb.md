# SSH command placement

Status: adopted and implemented.

`podbox remote ssh` operates across an existing machine boundary.
It dispatches `serve`, `connect`, and `forward` through the SSH helpers.
`podbox machine ssh` boots the specified Linux guest and reaches its server
through the serial transport.

A bare `podbox ssh` returns 125 and names the supported command group.
There is no `local` group, remote fetch verb, relay server verb, or probe verb.
Do not advertise a proposed group as an implemented command.

The CLI dispatch and parity table own the command surface.
[T-1404](../../TODO/podssh.md) records the live command proof.
[Limits](../limits.md) records the untested conditions.
