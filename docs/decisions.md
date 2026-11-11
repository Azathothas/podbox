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
| 2026-11-06 | File size | Each Rust source file outside `vendor/` has at most 500 lines, with no allowlist, as in podcrates. Every larger file is split now, before other work (the owner: "500, split everything now"). T-1674. |
| 2026-11-06 | Synchronous face | podvm takes `pod-tls` with `default-features = false` and `pod-proxy` with its feature `blocking`, as podcrates built them (PodUtils/podcrates#4, its call C20); no tokio in podvm's own code. |
| 2026-11-09 | KVM | The owner lifts the KVM deferral of podvm fully: a session can start a KVM guest where `/dev/kvm` exists (podcrates docs/decisions.md, 2026-11-09, Q-A6). T-1112, T-1350, T-1641 and T-1682 are free. |
| 2026-11-09 | Releases | "everyone can release betas whenever needed/fits, but no crates io release". podvm can cut a beta when it fits; no crate goes to crates.io now. |
| 2026-11-09 | New projects | Each project that PodUtils/podcrates#35 and #36 propose gets its own repository of PodUtils; `podrunner` runs podvm as a unit kind. podvm does not grow those projects. |
| 2026-11-09 | Test accounts | "Not now". No Cloudflare, Zero Trust or GitHub test account as a CI secret. A live step that needs one waits; offline tests stay. |
| 2026-11-09 | Workflow dispatch | A session dispatches the workflows of this repository itself, with the GitHub tool `actions_run_trigger` (the owner's account). The issues `[gh action] requested` and COORD-005 (#198) are superseded. |
| 2026-11-09 | Priority | "priority is first podvm, podnet, podssh projects, cfb & githome can wait". The entries for the asks of cfb and githome (T-1694 to T-1696 and the clauses of PodUtils/podcrates#39) stay, after podvm's own work order. The new repositories of PodUtils stay empty. |
| 2026-11-10 | Findings | "whenever some flaw/nitpick/bug is found, if it is small, fix it right then & there otherwise file a proper entry as a followup task, and not just print it in chat and lose it" (podcrates docs/decisions.md, 2026-11-10). It applies to podvm too: each subagent brief carries the rule. |
| 2026-11-10 | References closed | "the ruling that reference need to stay open is wrong, for instance #190 is already done by parent/sibling projects". A `reference` issue closes when its lesson is taken, in podvm or in a sibling repository, or when it has no remaining use. It stays open only while podvm work still needs it. This narrows the row "References" of 2026-11-06. |
| 2026-11-10 | podrunner | "Add https://github.com/PodUtils/podrunner as rw ... create issues ... for everything you must do (add as issues + task entries) before podrunner can come into fruition", from the design review of PodUtils/podcrates#47 that the owner gave. podrunner gets issues; podvm gets issues and entries for its part; the crates get COORD issues. The row "podrunner online" of 2026-11-11 answers DR-1 and DR-2 of that review. PodUtils/podcrates#47 itself stays as it is ("leave #47 alone", podcrates, 2026-11-10). |
| 2026-11-11 | podrunner online | "`podrunner` has been attached so you can start a direct session for it, it needs to quickly cpy/adapt the working style, settings, discipline etc" (podcrates docs/decisions.md, 2026-11-11, and its call C49). DR-1 (PodUtils/podrunner#2) and DR-2 (#3) are yes: the plan of PodUtils/podrunner#1 is the plan, and PodUtils/podrunner holds the code. A lead session of PodUtils/podrunner runs. It writes to podvm by a COORD issue of podrunner titled `(to podvm)`. podvm's entries T-1704 to T-1715 have no wait for DR-1 and DR-2 now. The other decisions of the review (DR-4 to DR-17, PodUtils/podrunner#4 to #8) belong to the podrunner session. `pod-dockerapi` comes with phase 3 of podrunner (C49). |
| 2026-11-11 | podvm resumes | The owner started a podvm session on 2026-11-11 and answered Q2 "Resume the order": podvm's work order of `TODO/PROGRESS.md` resumes from item 1, after T-1716 takes podcrates `v0.15.0`. podvm answers COORD issues again. It supersedes "podvm paused" of 2026-11-10. podcrates' row "The sessions of 2026-11-11" calls podvm "on break"; COORD-023 tells podcrates that podvm resumed. |
| 2026-11-11 | GitHub footer | The owner answered Q3 "Footer on posts only". A GitHub issue, comment or review that a cloud session writes ends with the footer that the cloud session requires ("Generated by Claude Code"). A commit, a tag, a release and a file of the repository carry no attribution; `.githooks/commit-msg` refuses it in a commit. `AGENTS.md`, section 3, has the rule. |
| 2026-11-11 | Fuzz targets | The owner answered Q4 "COORD to podcrates". The fuzz targets of the parsers that live in podcrates (`pod-oci`, `pod-extract`, `pod-enter`) go to podcrates as a COORD issue (COORD-023). T-1684 keeps the ELF reader of the interposer and the watch of `vendor/`. |
| 2026-11-11 | T-1696 dropped | The owner answered Q5 "Drop T-1696". podvm builds no `--secret` flag and no read-once mode. podrunner stages the credential files of the official runner itself, at mode 0600 and with no argv and no environment, and removes them at `destroy` (G-03 as DR-6 restates it, PodUtils/podrunner#6). The ask A-V-11 of PodUtils/podcrates#39 is not podvm's. |

## Calls made without a ruling

Each call stays in force until the owner rules otherwise.

| Date | Call | What |
| --- | --- | --- |
| 2026-11-06 | C1 | A clean break from the old names: no `podbox` alias of the binary, no `PODBOX_*` fallback, no migration of state paths (the Compatibility ruling). |
| 2026-11-06 | C2 | The argv0 name `podvm` no longer selects the machine tier, because it is the name of the binary now. `--podvm-tier=machine` selects the machine tier; with no flag, the ladder selects (call C14 removed the value `chroot`). The flag keeps the prefix of podvm's other own flags (`--podvm-mem`, `--podvm-qemu-arg`), which keeps them apart from docker's. The area file of the machine tier is now `TODO/machine.md`; the crate `podbox-podvm` (measurement tools) becomes `podvm-measure`. |
| 2026-11-06 | C3 | The untracked material of the old checkout is on the branch `archive/podbox-worktree` (`docs/history/worktree-archive.md`). |
| 2026-11-06 | C4 | The WSL base of the parked lane is named `podvm`; one variable, `PODVM_WSL_INSTANCE`, names it. |
| 2026-11-06 | C5 | The local crates are renamed `podvm-*` now. Each generic one leaves podvm when podcrates releases its `pod-*` crate and podvm takes it by tag. |
| 2026-11-06 | C6 | The move to the pinned stable Rust and edition 2024 is the first step of the first cloud session, before the first `pod-*` crate. The record moves to the format of `pod-todo` as its own milestone after that. |
| 2026-11-06 | C7 | `.claude/settings.json` is podcrates' file, with force-push denied. |
| 2026-11-06 | C8 | The variables that a payload and podvm-interpose read (`PODVM_MODE`, `PODVM_MAPS` and the rest) become settings of `pod-enter` and `pod-supervise` that podvm fills; no crate hard-codes a `PODVM_` name. podvm-interpose, the memo format and the download of a guest base image stay in podvm (COORD-004, #197). |
| 2026-11-06 | C9 | podvm takes `pod-tls` 0.4.0 with `default-features = false` and accepts its three changes of behaviour: podvm-ssh trusts the first usable host bundle and the compiled-in roots together; a bundle with a block that is not valid base64 is skipped whole; the TLS handshake of the relay runs in `Tls::connect`, before the WebSocket upgrade, with the write bound set before it. The hint of `pod-tls` that names a CA file "which replace podvm's roots" is podcrates' known gap (PodUtils/podcrates#7); podvm waits for its fix. |
| 2026-11-06 | C10 | podvm takes `pod-proxy` with feature `blocking` for each HTTP request and both relay legs (PodUtils/podcrates#9 and #13), under the ruling "Synchronous face": the private current-thread runtime of `pod-proxy` brings tokio into the binaries, never into podvm's own code. ureq leaves; the release `podvm` shrinks by 36 KiB, `node` grows by 236 KiB. An unusable proxy variable fails the dial with a named error; a proxy at a command (`+exec:`) is refused. `HTTP_PROXY` is not read for a plain-HTTP insecure registry until podcrates T-075; each request takes a new connection until podcrates T-077. This call replaced its first form ("never into the `podvm` binary") on the same day, when the registry moved to `pod-proxy`. |
| 2026-11-06 | C11 | podvm takes `pod-hostprobe` 0.6.0 (PodUtils/podcrates#17): `podvm-probe` keeps its report, `exit`, `json`, `nongoals` and its settings, and re-exports the crate's modules, so each other crate keeps its `podvm_probe::X` paths. podvm accepts that the noexec detail says "move the directory off this filesystem", that `podvm-probe` pulls `pod-core`, and the glue that maps a `Failure` back to podvm's errors. Since podcrates v0.7.0 (T-081) the glue reads `Failure::io_cause()`, which walks the whole chain, in place of a downcast of the first source. The workspace pins of `syscalls` and `linux-raw-sys` leave with the syscall layer. |
| 2026-11-09 | C12 | `podvm stop -t 0` sends SIGKILL at once, as docker does (T-1691, issue 215). A negative value stays refused. |
| 2026-11-09 | C13 | On an open row of the probe JSON, the non-goal `remedy` is an empty string; the key stays (T-1689, issue 213). |
| 2026-11-11 | C14 | `--podvm-tier` takes `machine` only (T-1712, form a, issue 237). The value `chroot` had no effect since C2: a caller that asked for chroot got the rung of the ladder with exit 0. It is a flag error now, and the refusal names the value and `.EnteredRung`. A caller reads the entered rung from `system info` (T-1713) and needs no force. Form b (a forced chroot that refuses where chroot is denied) waits for a caller that needs it. |

## Questions for the owner

No question is open on 2026-11-11.

The seven questions of 2026-11-10 (Q1 to Q7, `git show 00c189b:docs/decisions.md`)
are settled on 2026-11-11. Q1: DR-1 and DR-2 are yes (the row "podrunner
online"); the other decisions of the review belong to the podrunner
session. Q2: the row "podvm resumes". Q3: the row "GitHub footer". Q4: the
row "Fuzz targets". Q5: the row "T-1696 dropped". Q6: `pod-dockerapi`
comes with phase 3 of podrunner (podcrates C49). Q7: the cleaned copy of
the design review is the work of the podrunner session
(PodUtils/podrunner#14).

The three questions of 2026-11-06 are settled: the synchronous face
(podcrates gave it, PodUtils/podcrates#4), the access of T-040 (podcrates
reads the public repository), and the file-size limit (the row "File
size" above).

## Superseded

| Date | Ruling | Replaced by |
| --- | --- | --- |
| 2026-11-10 | podvm paused: the owner paused podvm's own work so that podssh and podnet finish first ("podvm has been shut down so work on podssh/podnet can finish up quicker", podcrates docs/decisions.md, 2026-11-10). The work order of `TODO/PROGRESS.md` waited for the owner's word. | 2026-11-11, podvm resumes. |
| 2026-10-02 | Windows route is wsl-toolkit, instance podbox (`TODO/RULES.md`, section 11). | Cloud sessions (2026-11-06); the parked lane uses the instance `podvm` (C4). |
| 2026-10-02 | `references/` stays, and the repoint is filed as work (T-1606). | References (2026-11-06). |
| 2026-11-06 | GitHub Actions is disabled for the account that the podcrates session pushes with, so a push there starts no workflow. Until the owner resolves it, the owner's account dispatches podcrates' `ci.yml` and `release.yml` by `workflow_dispatch` (COORD-005). | 2026-11-09, Workflow dispatch. |
| 2026-11-06 | A session that needs a workflow run it cannot start opens an issue titled `[gh action] requested` with the exact `gh` commands in its body; the owner's local agent runs them and writes back the run ids. | 2026-11-09, Workflow dispatch. |
| 2026-10-02 | KVM and Windows-guest work is deferred; no session starts a KVM guest (`TODO/RULES.md`, section 11). | 2026-11-09, KVM. |
| 2026-10-02 | An unattended KVM guest run is permitted on one host only, with a watchdog outside the guest that removes an emulator that outlives its bound (`TODO/RULES.md`, section 11). | 2026-11-09, KVM. |
