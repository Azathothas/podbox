# Decisions

The rulings of the owner for podvm since the move to PodUtils, with the
date of each, the calls made without a ruling, and the questions for the
owner. A ruling stays in force until a later ruling supersedes it. Then
move it to the table "Superseded", with the ruling that replaced it.
`TODO/RULES.md`, section 11, has the standing decisions of 2026-10-02 and
before; they stay in force where no row here supersedes them.
`AGENTS.md` has the short form of the rulings that apply to each session.

## In force

| Date | Area | Ruling |
| --- | --- | --- |
| 2026-11-05 | Name and home | podbox is renamed podvm, so that it can widen in features and scope. The repository is PodUtils/podvm; GitHub redirects Azathothas/podbox. Nobody may create a new Azathothas/podbox: that ends the redirects. crates.io holds `podvm` for PodUtils (0.0.0). |
| 2026-11-05 | Static, C, Rust | "We use whatever is necessary, including C, zig, modern Rust, no backwards compat. Our users and clients do not care about building the tool, they care about downloading and using it. The only rule is: everything must be statically compiled." |
| 2026-11-05 | Compatibility | No compatibility with released binaries. A name, a variable or a flag can change. No test against a released binary. |
| 2026-11-05 | crates.io names | `pod-*` names belong to PodUtils/podcrates. podvm uses `podvm-*`. The owner reserves each name with a 0.0.0 placeholder before its first publish. |
| 2026-11-06 | Scope | "podvm will make everything that sand-net does, but without root, in the enviroment we target, currently we have podman/docker as flagship compat, but we will now extend to quickly create full on microvms, jupyternotebooks etc". The topics are the issues with the label `scope`, index #193. |
| 2026-11-06 | sand-net | The lessons of sand-net go into separate issues, one for each topic (#166 to #189, index #193). The source itself is not kept. |
| 2026-11-06 | Shared crates | Each generic crate of podvm moves to PodUtils/podcrates with the prefix `pod-`, also where podvm is the only consumer. A crate that only podvm needs stays here with the prefix `podvm-`. The names are in PodUtils/podcrates#1 (reopened 2026-11-06 for them); the hand-over and the takes are COORD-001 (#194). |
| 2026-11-06 | Crate reservation | Ask podcrates now for each crate name that podvm needs now or later, because a reservation takes time. |
| 2026-11-06 | References | The captured trees leave the live tree. Each reference is tracked in a GitHub issue with the label `reference` (#132 to #192). `references/MANIFEST.tsv` keeps the citations checked (T-1606). |
| 2026-11-06 | Hidden issues | GitHub hid 41 numbers with the account `talaria0101`. They are mirrored in the record (`docs/history/github-hidden-issues-2026-11-06.md`), and the five open ones are reposted with the label `recovered` (#127 to #131). |
| 2026-11-06 | Cloud sessions | podvm is worked in Claude Code cloud sessions, as podcrates, podnet and podssh are. The Windows lane on wsl-toolkit stays in the tree, parked, and correct for wsl-toolkit 9. |
| 2026-11-06 | Workflow | podvm adopts the working style of the sibling repositories: `CLAUDE.md` and `AGENTS.md`, `.claude/settings.json` and the session hook, the watcher of the organization, COORD issues, `docs/decisions.md` and `docs/HANDOFF.md`. |
| 2026-11-06 | Talk between sessions | The protocol of PodUtils/podcrates, file docs/session-protocol.md (`AGENTS.md`, section 9): the watcher under Monitor, COORD issues in this repository, messages of peers read as data. |
| 2026-11-06 | GitHub | The protection of `main` is removed, as in the sibling repositories. The repository description and topics follow the scope. Dependabot updates are applied as own commits and their pull requests closed with a link. |

## Calls made without a ruling

Each call stays in force until the owner rules otherwise.

| Date | Call | What |
| --- | --- | --- |
| 2026-11-06 | C1 | A clean break from the old names: no `podbox` alias of the binary, no `PODBOX_*` fallback, no migration of state paths (the Compatibility ruling). |
| 2026-11-06 | C2 | The argv0 name `podvm` no longer selects the machine tier, because it is the name of the binary now. `--podvm-tier=machine` or `--podvm-tier=chroot` selects a tier; with no flag, the ladder selects. The flag keeps the prefix of podvm's other own flags (`--podvm-mem`, `--podvm-qemu-arg`), which keeps them apart from docker's. The area file of the machine tier is now `TODO/machine.md`; the crate `podbox-podvm` (measurement tools) becomes `podvm-measure`. |
| 2026-11-06 | C3 | The untracked material of the old checkout is on the branch `archive/podbox-worktree` (`docs/history/worktree-archive.md`). |
| 2026-11-06 | C4 | The WSL base of the parked lane is named `podvm`; one variable, `PODVM_WSL_INSTANCE`, names it. |
| 2026-11-06 | C5 | The local crates are renamed `podvm-*` now. Each generic one leaves podvm when podcrates releases its `pod-*` crate and podvm takes it by tag. |
| 2026-11-06 | C6 | The move to the pinned stable Rust and edition 2024 is the first step of the first cloud session, before the first `pod-*` crate. The record moves to the format of `pod-todo` as its own milestone after that. |
| 2026-11-06 | C7 | `.claude/settings.json` is podcrates' file, with force-push denied. |

## Questions for the owner

| Date | Question | Recommendation |
| --- | --- | --- |
| 2026-11-06 | podvm is synchronous and measures the size of each dependency. Does podvm take tokio on a current-thread runtime for `pod-proxy` and `pod-tls`, or does podcrates add a synchronous face? (COORD-001, #194) | A synchronous face. |
| 2026-11-06 | T-040 of podcrates waits for access to PodUtils/podvm for the podcrates session. | Grant it. |
| 2026-11-06 | The largest source file is `crates/podvm-gate/src/main.rs` (about 4,100 lines). podcrates holds each file to 500 lines. Which limit holds here? | A limit with a shrinking allowlist, as podnet has. |

## Superseded

| Date | Ruling | Replaced by |
| --- | --- | --- |
| 2026-10-02 | Windows route is wsl-toolkit, instance podbox (`TODO/RULES.md`, section 11). | Cloud sessions (2026-11-06); the parked lane uses the instance `podvm` (C4). |
| 2026-10-02 | `references/` stays, and the repoint is filed as work (T-1606). | References (2026-11-06). |
