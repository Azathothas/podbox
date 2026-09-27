# `podbox remote ssh`, and why not `podbox ssh`

**Status:** decided by the operator, 2026-09-27. Not open.

## The question

podssh is a tool for reaching a machine that is not the one you are sitting
on. An earlier branch carried it as `podbox ssh`. The operator's decision is
`podbox remote ssh`, and this page records why, because the reasoning
generalises and the next person to add a verb will ask the same question.

## The decision

```sh
podbox remote ssh <command> [options]   # reach another machine
podbox local  <command> [options]       # act on THIS machine
podbox machine ssh <command> [options]  # a guest podbox itself runs
```

`remote` and `local` are a pair, and they are the axis the question was
really about: **does this verb act across a machine boundary or not.**

## Why `remote` and not a bare `ssh`

The operator named three shapes, and this is what settles them.

**`podbox remote ssh` is right**, because `remote` is a namespace and not a
synonym. A namespace takes more than one member, and the operator named the
members that will arrive: `remote fetch`, `remote download`, `remote wget`.
Every one of those is "get something from, or put something on, a machine that
is not this one", and each would otherwise want its own top-level verb. Under
`ssh` alone they have no home; under `remote` they are subcommands and the top
level stays a container language.

A bare `podbox ssh` also collides with the reading an agent brings. Thousands
of agents reach for `docker` because it is the only container language they
know, and a top-level verb that is not a docker verb is one they will not
guess. The repo's own rule is that a tool needing its user to learn its
differences has replaced nothing. `remote` reads as a group, so an agent that
runs `podbox ssh` gets a refusal that names the group and lists its members,
rather than a verb whose meaning has to be inferred.

**`podbox local ssh` for the local ops** is right for the same reason, and it
is the half that makes `remote` mean something: a namespace with one member
does not distinguish. `local` is what gives `remote` a counterpart, and it is
where the verbs that act on this machine go when they need names of their own.

**`podbox machine ssh` is podman parity, and it is a different axis.** A
`podman machine` is a guest VM that podman itself runs and manages, so
`podman machine ssh` means "ssh into the VM podman started for me". That is
not the same as `remote ssh`, which is "ssh into a machine somebody else
started, over whatever network will carry it". The operator settled
`machine ssh` separately, so it does not compete with `remote ssh` and must
not be folded into it: folding them would make one of the two mean two
things.

## What this costs, stated plainly

Renaming a verb breaks every script that used the old one. `podbox ssh` has
existed only in unmerged branches, so today's cost is one line in a commit.
The cost of NOT deciding is a `remote` group invented later under pressure,
when `fetch` has already shipped as a top-level verb and has to be deprecated.
Deciding now is cheaper.

## How the refusal is worded

`podbox ssh` must be a useful answer, not "unknown command", because an agent
that typed it is asking a real question. The message names the group and lists
its members, the same way the parity table already answers a docker verb
podbox does not have. That is the house rule, not a new one, and it exits 125:
a verb podbox has and refuses is podbox failing to run the caller's command.

## The one-line summary

`remote` is a namespace because more than one verb needs one, `local` is its
counterpart so the axis is visible, and `machine` is a different axis
entirely and stays separate.
