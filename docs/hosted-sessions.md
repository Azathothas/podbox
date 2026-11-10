# Hosted sessions

podvm is worked in Claude Code cloud sessions ([decisions](decisions.md), 2026-11-06).
`AGENTS.md`, section 8, gives the start procedure.
Run `sh scripts/cloud-setup.sh` to install zig and the toolchain and to report the host.
Do not assume the clone has full history or the default branch.
Run `git fetch --unshallow origin main` before a `git log` over a range.
Check the current branch, source state, and available disk space.

Read the required project orientation before changing the tree.
Use the project build and gate commands (`AGENTS.md`, section 12).
Keep network calls and child waits bounded and non-interactive.
The egress is an HTTPS proxy with an allowlist. Do not go around it.
The cloud host has no `/dev/kvm`; a guest runs under TCG there.

Set the operator's Git identity and project hook before committing
(`.claude/session-start.sh` does both).
Do not add a tool attribution.
Commit verified work in logical changes. Use the authorized remote route.
A transient machine can lose uncommitted files.

Save proof and handoff in tracked files.
A cold reader must not need an ignored cache or session transcript.
Do not treat a previous host's result as a measurement of this host.

[containers](containers.md) owns lane setup and cleanup.
[sessions](methodology/sessions.md) owns the session procedure.
[git](conventions/git.md) owns identity and publication.
