# The ssh server a cage can actually run

**Status:** measured 2026-09-27 in the reference cage. Not a preference.

## The question

`podbox remote ssh serve` starts an ssh server on this machine and hands it
one socketpair, so the operator's `ssh` reaches it. Which server? The obvious
answer is the host's `sshd`, and in a cage that answer is wrong for reasons
that are not visible until measured.

## What was measured, in order

| step | result |
| --- | --- |
| `sshd` present | yes, OpenSSH 10.5p1 |
| `sshd -i -t -f <config>` with a hand-written config | exit 0 |
| `sshd -i` at runtime | `Privilege separation user nobody does not exist` |
| with a passwd shim supplying `nobody` | `Missing privilege separation directory: /var/chroot/ssh` |
| with `ChrootDirectory` moved | no effect: a different, hardcoded path |
| with `UsePrivilegeSeparation no` | deprecated and ignored in OpenSSH 10.5 |

`/var` does not exist in that cage, `/etc` is read-only, and `chroot(2)` is
denied, so the privsep sandbox cannot be provided at all.
`UsePrivilegeSeparation` stopped being settable in OpenSSH 8.4, which means
**no configuration makes a modern `sshd` run in a cage that forbids chroot.**

## Why this page exists

A config that passes `sshd -t` is not a config that runs. The first two
measurements said yes and the third said no, and the difference is a runtime
chroot the parser never checks. Anyone who concludes "sshd works here" from a
config file or a `--test` pass will be wrong, and will be wrong in a way that
looks like a podssh bug at the far end.

## The answer: dropbear, and the two things it needs

`dropbear -i` has no privsep chroot, no `/var` requirement and no
`/etc/passwd` requirement of its own. Two adjustments were needed.

**1. Build it DYNAMICALLY, not static.** A statically linked dropbear carries
its own libc, so `LD_PRELOAD` cannot reach it and its passwd lookups fail
against a cage that has no passwd database. The symptom is precise and
misleading:

```
Login attempt for nonexistent user from localhost:E
```

`root` exists. The binary cannot see that it does. `./configure
--disable-static-programs` makes the shim apply, and the same invocation then
authenticates.

**2. Tolerate a denied `setgroups(2)`.** A seccomp-filtered cage denies
`setgroups` at the syscall level, so dropbear's `initgroups()` always fails
there and `dropbear_exit("Error changing user group")` kills every login.
`setgid` is kept fatal and `initgroups` is not: `setgid` is the call that
actually changes the group, and a failure there is a real privilege problem,
while a denied `initgroups` leaves a session with correct uid and gid and
merely no supplementary groups. This is `sandssh`'s
`dropbear-setgroups-tolerance.patch`, re-applied because upstream moved the
code and the patch no longer applies cleanly.

## What was proved with it

A full public-key session, over a rendezvous, in a cage with no pty, no
`bind`, no `chroot`, no `/var`, no `/etc/passwd`, no UDP and no working
resolver:

```sh
ssh -o "ProxyCommand=podssh connect --relay unix:///tmp/r.sock \
         --name agent1 --auth SECRET" \
    -o BatchMode=yes -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null -i user_ed25519 root@agent1 \
    'echo DROPBEAR_RENDEZVOUS_OK; id -u; uname -s'
```

```
DROPBEAR_RENDEZVOUS_OK
0
Linux
```

ssh exit 0, read from the ssh process and not through a pipe. The server log
records `Pubkey auth succeeded for 'root'` and `Exited normally`.

## What was NOT proved, stated plainly

- **`scp` over the rendezvous was not run.** The transport is the same byte
  pipe and `scp` was proved separately over the relay path against a live VM,
  but the two facts have not been multiplied into one run.
- **No pty.** The session is non-interactive. A full-screen TUI still cannot
  run, because the cage has no `/dev/ptmx` and neither host can create one.
- **`Failed chdir '/root'` remains.** The user's home does not exist in the
  cage. It is a one-line refusal, not a session failure: the command ran and
  the exit code was 0.

## The consequence for the product

`--server` is not a convenience flag. On a machine with a working `sshd` the
default is right; on a machine like this cage the default cannot work and
naming a server is the only way in. The honest shape is therefore a **default
that probes and falls back**, and a message saying which server it chose and
why, rather than a default that fails with the far end's error attached.
