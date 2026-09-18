# Upstream tool shape

The retired claim that `wsl-toolkit` ships as two products. Upstream deleted the PowerShell product and its launcher. The binary that remains is one executable. This page keeps the superseded wording verbatim. The live pages state what is true now.

## Retired wording from `docs/containers.md`

> ⭐ **It is two products now, and a caller gets the compiled one by default.**
> Upstream ships a PowerShell script and an executable that carries that same
> script inside itself and adds to it, and its launcher resolves the executable
> first. ⚠ **A page here that names its flags is a page that goes stale without
> anybody editing it**, so this one does not: read the tool's own documentation
> at the link. What matters at this level is that the two exist, that a caller
> can ask for either, and that "the version I ran" is now a question with two
> answers.

## Retired wording from `docs/agent-tooling.md`

> `wsl-toolkit` surveys the host, owns one WSL distro with a container engine in it, runs a command in a container or a set of them, and removes what it made. ⭐ Two products, one of them compiled.

## What took it away

Upstream `Azathothas/TEMPLATE` `docs/containers.md` states that the page used to name two products and that upstream deleted the PowerShell product and its launcher. The three agent skills name one executable each. The binary on this host answers as one product:

```text
wsl-toolkit 4.0.0
```

`wsl-toolkit --help` lists one command set with no launcher. `wsl-toolkit man --no-pager` generates the manual from the commands the binary holds. The correction lands in `docs/containers.md` and `docs/agent-tooling.md` in the same change as this page. The date is read from the machine at write time and recorded in the commit message.
