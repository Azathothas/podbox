# gate

Record semantics: [task rules](RULES.md#5-entry-closure).


`docs/methodology/gate.md` and `docs/methodology/experiments.md`. What makes a
check an assertion rather than a decoration, and what makes a number a
measurement rather than a property of one host.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ A check lands with the plant that proves it can fail, in the same change.

---

### T-1201 The gate reaches every file this project wrote

Source:      `docs/methodology/gate.md`; `docs/conventions/docs.md:1-20`
Category:    gate
Priority:    P0
Effort:      S
Status:      done 2026-09-08

Problem:     The first commit of this repository shipped a README and eight
             crate doc comments naming `TODO/`, `TODO/probe.md`,
             `TODO/interpose.md` and six more, none of which were in the tree.
             The check that would have caught it read `TODO/` alone, so it saw
             none of it, and the check itself was in the missing directory.
Premise:     ⭐ **Measured on this tree.** Two defect classes the gate could not
             see, both found by extending it:
             the citation form that broke the repository carried **no line
             number**, so a `path:line` matcher could never match it; and
             `os.path.isfile` asks this disk rather than a fresh clone, so an
             untracked file reads as present to whoever ran it last and absent
             to everybody else.
             ⛔ **The coverage counts are recorded nowhere, and that is a
             finding rather than an omission.** They are self-referential: this
             file's own citations are among the things counted, so writing the
             number down changes it. Measured while writing this entry, twice:
             `todo_links` moved from 261 to 262 because recording 261 added a
             link. `./scripts/check-todo.py` (now `podbox-gate`) prints the reading on every run,
             and no document copies it.
             What is fixed rather than measured, and safe to state: bare path
             citations were an **unchecked class** before this entry, and
             `tree_citations` counted **1**.
Approach:    Checks 11 to 15 of `crates/podbox-gate/src/main.rs`. Citations and links are
             resolved across every tracked file this project wrote, against
             `git ls-files` rather than the filesystem. `references/` and the
             verbatim methodology copy are excluded because their text is
             somebody else's; `AGENTS.md` is the one file under `docs/`
             that is written here and is checked like anything else.
             ⚠ Two exemptions, both narrow and both visible in the source: a
             path under `experiments/results/`, which is where a measurement not
             yet taken will land, and a line carrying the `known-absent` token,
             which is how a document names something deliberately not here.
             `git grep -n known-absent` lists every use.
Decision:    Resolve against git, not the disk. A check that agrees with
             whoever ran it last and disagrees with a fresh clone puts the
             disagreement where the one person who could fix it is the one being
             told nothing is wrong.
Prove:       `./target/release/podbox-gate && ./target/release/podbox-plant`

**Done.** `./scripts/check-todo.py` (now `podbox-gate`) exits 0 and prints a coverage line; the two
plants for these checks, cases 11 and 14 in scripts/plant.sh (now `podbox-plant`), both go red.

---

### T-1202 Every check is planted against, and a plant that stops reaching its subject says so

Source:      `docs/methodology/gate.md`; `docs/methodology/reviews.md`
Category:    gate
Priority:    P0
Effort:      M
Status:      done 2026-09-08

Problem:     A check that quietly matches nothing exits 0 exactly like a check
             whose assertions all passed, and the second is what a reader
             assumes they are looking at. M-1 verified six checks by breaking
             the tree by hand. That verification lived in a transcript, so it
             was worth nothing to the next session.
Premise:     ⭐ **Measured, and it found a real one on its first run.**
             scripts/plant.sh (now `podbox-plant`) plants a defect per case and asserts each makes
             the gate red **with that defect's own message**, and that the
             message was not already there. On its first run, over the fifteen
             cases it then had, case 6 landed its mutation and the gate
             stayed green: each corpus tree is named twice in
             `TODO/reference-map.md`, in the licence table and again in the
             verdicts table, so deleting one row left the other and the check
             never lost the tree. The plant now removes every mention.
             Result on 2026-09-08, after [T-1205](gate.md) added check 18 and its
             two cases: 20 plants caught, 0 missed, 3 controls
             quiet, 0 fired, over eighteen checks. ⛔ **Every check but one has
             a case, and the harness says which on every run.** Check 16, the
             coverage floor, has none: planting it means making a check examine
             nothing, which requires editing the gate's own matchers rather than
             the tree. Listing passing cases without naming the check that has
             none is the same vacuity this entry exists to remove.
             ⚠ The counts here are not held by anything and will move again.
             `./scripts/plant.sh` (now `podbox-plant`) prints the current pair on every run, and it
             is the answer; this line is a reading from one day.
             ⭐ **A second case rotted and guard 1 caught that too.** Case 3
             named `Status:      open` literally and stopped landing the moment
             `TODO/probe.md` had no open entry left, which is what closing M0
             did. It reported "the mutation did not land" rather than a silent
             green, and the case is status-agnostic now. Cases 4 and 10 took the
             same treatment for the counts, for the same reason.
Approach:    Four guards, each because the harness shape without it reports
             success while planting nothing:
             1. **the mutation must land**, asserted by hashing the file list
                before and after with `git hash-object`. A `sed` that matched
                nothing otherwise reads as a check that cannot fire;
             2. ⛔ **restore from a copy, never `git checkout --`.** Checkout
                restores from the **index**, so a staged plant survives it.
                Measured on 2026-09-08 in a scratch repository: with the plant
                staged, `git checkout -- f` left the plant in the worktree;
             3. **one file list**, iterated by both the backup and the restore,
                so a case that learns to touch a new file cannot leave it
                behind for every later case to measure;
             4. **controls counted apart from plants.** A control stays quiet, a
                plant goes red; adding them reports more caught than were.
Decision:    Assert the message, not the exit code. A gate already red for
             another reason exits 1 either way, so a case watching only the exit
             code passes vacuously the moment anything else breaks.
Prove:       `./target/release/podbox-plant`

**Done.** Exit 0: 20 plants caught, 0 missed; 3 controls quiet, 0 fired.
⚠ The pair moves whenever a check lands. `./scripts/plant.sh` (now `podbox-plant`) prints it, and
that is the answer; this line is a reading from the day it was taken.

---

### T-1203 A measurement taken on one host is a property of that host

Source:      `docs/methodology/experiments.md`; `TOOL.md` section 5 M5
Category:    gate
Priority:    P1
Effort:      M
Status:      done 2026-09-08

Problem:     Every number in `experiments/results/` was taken on one machine
             with one libc and one `nsswitch.conf`. An artefact that starts on
             every distribution and does the right thing on none of them reads
             as success to a single-host smoke test.
Premise:     ⭐ **Measured here, and it is why this entry exists.**
             `experiments/results/nsswitch-contract.txt` check B reads three
             pinned images: two name `files` and one ships no `nsswitch.conf`
             at all. Same question, three different answers, and podbox's
             behaviour has to be right on all three.
             ⭐ **And the runner has now taken the reading.** All eleven rows
             pulled and ran, four musl and seven glibc, alpine 3.10 through
             fedora 42. `experiments/results/across-distributions.txt`:

             ```
             alpine-3.22          musl   present-no-passwd-line  seen
             alpine-3.20          musl   present-no-passwd-line  seen
             alpine-3.10          musl   absent                  seen
             voidlinux-musl       musl   files                   seen
             debian-11            glibc  files                   seen
             debian-12            glibc  files                   seen
             ubuntu-20.04         glibc  files                   seen
             rockylinux-8         glibc  sss files systemd       seen
             opensuse-leap-15.6   glibc  compat                  probe-crashed-rc136
             fedora-42            glibc  files systemd           seen
             archlinux-latest     glibc  files systemd           seen
             ```

             ⚠ **Six distinct shapes of `nsswitch` across eleven rows**:
             `files`, `files systemd`, `sss files systemd`, `compat`, a file
             present with no `passwd` line, and no file at all. That is the
             whole argument for T-0410 probing rather than assuming.
             ⛔ **And one row where a static glibc binary does not run at all.**
             `rc136` is 128 plus 8, SIGFPE, on `opensuse-leap-15.6`, from a
             probe built on this host's glibc 2.39 and static-linked. That is a
             reading about static portability and not about `nsswitch`, which is
             why the runner reports `probe-crashed` rather than `not-seen`. It
             also means the `compat` row is untested for the passwd question:
             the probe died before answering.
Approach:    One runner, one command, one row per pinned distribution. The
             subject is a script mounted read only, the checkout is mounted read
             only, and every row gets the same bytes, so a difference between
             rows is a difference between distributions and nothing else.
             Four rules that make it a measurement rather than a survey:
             1. the manifest **digest** is what makes a number from three months
                ago comparable to one from today. A rolling tag measures a
                different thing each week and says so nowhere;
             2. ⚠ a row that could not be pulled reads `no-pull`, is counted
                apart from a row whose command ran and failed, and does not on
                its own make the run exit 1. "Unreachable" and "broken" need
                different next moves;
             3. ⛔ a run where **nothing** ran exits 2. Every row unreachable
                reads exactly like every row agreeing;
             4. fully qualify the reference before pulling. An unqualified name
                resolves through an engine's own shortname aliases to a
                different registry, where a Docker Hub digest does not exist,
                and that arrives as `manifest unknown`, which reads as a broken
                pin.
             ⚠ The subject must copy itself out of the read-only mount before
             running, and needs `HOME` set to a writable path: several of these
             images have no writable home for a non-root uid, and that failure
             looks like the subject's.
Decision:    A matrix, not a wider single-host test. The reading that matters is
             not "it fails on alpine 3.10"; it is an artefact that starts
             everywhere and works nowhere, which one host cannot show.
Prove:       `./experiments/125-across-distributions.sh` exits 0 or 1, never 2, and writes one transcript per row into `experiments/results/`

**Done.** Exit 0, eleven rows ran, eleven transcripts in
`experiments/results/across/`. Four defects in the instrument were found by the
first run and fixed in place: a combined `ls` glob that reported `unknown` for
every glibc row, a `tr -s ' '` that did not squeeze the tab voidlinux uses, a
`nsswitch` reading that folded "absent" into "present and silent", and a probe
crash reported as "not seen".

---

### T-1204 Check 17 holds the newest committed reading under the ceiling, not only the baseline

Source:      Found by reading check 17 against the tree after M1 landed
Category:    gate
Priority:    P1
Effort:      S
Status:      done 2026-09-09

Problem:     Check 17 reads `experiments/results/bloat-baseline.txt` and asserts
             its `total_bytes` is under the ceiling. That file is deliberately
             the "before": M0's artefact, 496,184 bytes. It is not the shipping
             binary any more, so the number the gate holds is not the number the
             ceiling exists to hold.
Premise:     ⭐ **Measured on 2026-09-08.** M1 moved the artefact from 496,184 to
             2,130,672 bytes, and `experiments/results/bloat-image.txt` records
             it. scripts/check-todo.py (now `podbox-gate`) went green throughout, because the file
             it reads did not change. ⚠ The ceiling is still enforced at BUILD
             time by `experiments/110-bloat-delta.sh`, which exits 1 over it, so
             nothing is currently unprotected: what is missing is the half that
             runs on a clone with no toolchain, which is the half check 17 was
             written to be.
Approach:    Read every `experiments/results/bloat-*.txt` rather than the
             baseline alone, and assert each `total_bytes` is under the ceiling.
             ⛔ Still no build: the check reads committed evidence, so it keeps
             working on a fresh clone with no cargo, which is
             `crates/podbox-gate/src/main.rs`'s own constraint.
             ⚠ The baseline keeps its own separate assertion, because a missing
             baseline means there is no "before" and `TODO/deps.md` T-0910 rules
             that a dependency without one has not landed.
Decision:    Every reading, not "the newest by date". A date inside a file is a
             string this script would have to parse and rank, and a stale file
             somebody forgot to delete would then silently outrank a real one.
             Asserting all of them needs no ordering and fails on the same file
             either way.
Prove:       `./target/release/podbox-plant` reports case 17d caught, planting a `total_bytes` over the ceiling into `experiments/results/bloat-image.txt`

**Done 2026-09-09.** Check 17 now reads every tracked
`experiments/results/bloat-*.txt` and asserts each `total_bytes` under the
ceiling, and plant case **17d** is the one that was missing rather than failing:
before it, a shipping binary over the ceiling was invisible to a clone with no
toolchain. 17b stays beside it, because the baseline's own assertion is about
there being a "before" at all and is a different claim.

⚠ **A reading with no `total_bytes` line is skipped rather than failed.** An arm
that could not run records that it could not, [T-0910](deps.md) rules a skip is
not a pass, and it is equally not a size to hold.

⚠ The `Prove` above said "19 caught" when it was authored. The harness is at 21,
because three cases have been added since for other checks, so the count is not
quoted: `AGENTS.md`'s rule that a value lives in one file applies to this
one too, and its home is scripts/plant.sh's own verdict line (now `podbox-plant`).

---

### T-1205 The gate holds experiment numbers unique, because four Proves already collide

Source:      Found by sweeping every `Prove` in `TODO/` against `experiments/` at the close of M1
Category:    gate
Priority:    P1
Effort:      S
Status:      done 2026-09-08

Problem:     `experiments/README.md` rules that a number is never reused,
             because a citation of `30-` has to keep meaning what it meant.
             Nothing enforces it, and four entries already name a number that is
             taken. Each will be discovered by whoever implements it, mid-flight,
             exactly as [T-0203](image.md)'s was.
Premise:     ⭐ **Measured on 2026-09-08**, by matching every
             `experiments/<n>-<name>.sh` named in a `Prove` against the files on
             disk:

             | the `Prove` named | the number was already | renumbered to |
             | --- | --- | --- |
             | `80-extract-path-safety.sh` ([T-1103](milestones.md), [T-0304](extract.md)) | `80-interposer-abi.sh` | `220-` | <!-- known-absent -->
             | `100-lifecycle-loop.sh` ([T-1105](milestones.md), [T-0607](supervise.md)) | `100-interpose-symbols.sh` | `230-` | <!-- known-absent -->
             | `130-distro-sweep.sh` ([T-1106](milestones.md)) | `130-probe-parity.sh` | `240-` | <!-- known-absent -->
             | `140-negative-tests.sh` ([T-1109](milestones.md)) | `140-space-precheck.sh` | `250-` | <!-- known-absent -->

             ⚠ Four more name a free number and are fine: `120-`, `85-`, `95-`,
             and the four M1 authored at `180-` to `210-`.
             ⛔ [T-1103](milestones.md)'s is the one that bit first: it is M2's
             own acceptance, and M2 is the session that closed this entry.
             ⭐ **The check reproduced the sweep.** Written before the
             renumbering and run against the tree that carried all four, it
             reported exactly these four numbers and named the same
             `file:line` on each. A hand sweep and a check that agree is the
             check having been tested against a real defect rather than a
             planted one.
             ⚠ The renumbering is an append, not a gap fill, and the new numbers
             keep `experiments/README.md`'s "order they run": M2 is `220-`, M4
             `230-`, M5 `240-`, M6 `250-`.
Approach:    A check in `crates/podbox-gate/src/main.rs` that collects the leading number
             of every `experiments/<n>-*.sh` named anywhere this project wrote,
             plus every such file on disk, and fails when one number carries two
             names. ⛔ It reads text and the filesystem listing only, so it keeps
             working on a clone with no toolchain, which is that script's own
             constraint.
             ⛔ The check and its plant land in the same change:
             `AGENTS.md` and [T-1202](gate.md). The plant renames a number
             into a collision and asserts the gate goes red with this check's own
             message.
             ⚠ Renumbering the four above is part of this entry, in the entries
             that name them, and each keeps its title.
Decision:    Enforce uniqueness rather than dropping the rule. The rule exists
             because a number in a closed entry is a citation somebody has
             already written down, and `experiments/README.md` says so; a rule
             worth writing and not worth checking is the kind that stops being
             believed.
Prove:       `./target/release/podbox-plant` exits 0 with cases 18a and 18b caught, each planting a duplicate experiment number

**Done, 2026-09-08.** Check 18 of scripts/check-todo.py (now `podbox-gate`), and cases 18a and
18b of scripts/plant.sh (now `podbox-plant`), in this change. `./scripts/plant.sh` (now `podbox-plant`) exits 0 with
**20 caught, 0 missed, 3 controls quiet**.

⭐ **The check was written before the renumbering and run against the tree that
still carried all four collisions.** It reported exactly the four this entry
records and named the same `file:line` on each. A hand sweep and a check that
agree independently is the check having been tested against a real defect as
well as a planted one, which is the half scripts/plant.sh (now `podbox-plant`) cannot supply.

⚠ **Two cases, not one, because the halves fail apart.** A number can be
claimed by a document promising a script (all four real collisions were this
shape) or by a second file arriving on disk. A case for one leaves the other
unseen, which is this harness's own founding defect.

⛔ **The planted number is read out of the listing at run time and never
written into scripts/plant.sh (now `podbox-plant`).** The gate reads that file like any other, so
a literal taken number there would put a second name on it and redden the clean
tree, exactly as two literal citations did on 2026-09-08 and as `CEILING_NUM`
would. The same trap, a third time, in the same file.

⚠ **A line carrying the `known-absent` token is skipped**, as check 14 skips
it. This entry's own Premise table names the four old numbers beside their new
ones, and a check that read that table would report the defect the table exists
to describe.

---

### T-1206 CI installs the toolchain the build config names, and nine commits proved nobody was holding it

Source:      Found on 2026-09-09 by reading the gate workflow's own logs after nine consecutive red runs on `main`
Category:    gate
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     `.cargo/config.toml` points `CC_<target>` at `scripts/zig-cc.sh`
             for every architecture podbox builds for, and
             `.github/workflows/gate.yml` carried a second, hand-written
             declaration of the same requirement as a `bootstrap-env.sh`
             component list. The two drifted, and nothing could see it.
Premise:     ⭐ **Measured on 2026-09-09** by reading the workflow logs of every
             run since the last green one:

             | | |
             | --- | --- |
             | last green run on `main` | `a4ab727`, 2026-09-08, which was M0's tip |
             | consecutive red runs after it | **9**, `702cc02` through `563df15` |
             | jobs red in each | `build` and `lint`; `todo` green throughout |
             | what both died on | `zig-cc.sh: zig is not on PATH`, inside `ring`'s build script, exit 101 |
             | what a local `./scripts/dev.sh check` (now `podbox-dev check`) said | green, on every one of them |

             ⛔ **The break was a merge, not a commit.** M1 landed `rustls` and
             its `ring`, which compiles C behind a build script, and
             [T-0201](image.md) pointed the C compiler at `scripts/zig-cc.sh`.
             The workflow's list was written before any of that and was never
             revisited, so consolidating M1, M2 and M3 onto `main` turned a list
             that had been complete into one that was short by one word.
             ⚠ **The local gate could not have caught it**: this container has
             zig installed, so every check a session runs passes. The only
             machine that disagrees is the runner, and the only signal is a log
             nobody was reading.
Approach:    Add `zig` to the two jobs, then hold the invariant rather than the
             value. Check 19 of `crates/podbox-gate/src/main.rs` derives the required
             component set in two hops and hard-codes it in neither:
             `.cargo/config.toml` names the wrapper program, and the wrapper
             names the `bootstrap-env.sh` component that installs it. Every job
             in `.github/workflows/` that runs cargo must carry that set.
             ⚠ **One level of indirection, because a word match is not enough**:
             `./experiments/110-bloat-delta.sh` and `./scripts/build-interpose.sh`
             run cargo without the word appearing in the workflow, so a script
             named in a job is read and its cargo calls count as the job's.
             ⚠ Comment-only lines are dropped before the match. This workflow
             explains itself at length, and prose about a cargo build is not one.
             ⛔ The check and its plants land in the same change: `AGENTS.md`
             and [T-1202](gate.md).
Decision:    Derive, do not list. A second list of components here would be the
             same defect one file further along, and the entry that filed it
             would be the one that reintroduced it. The two hops cost a file
             read each and mean that a toolchain added to `.cargo/config.toml`
             is required of CI the moment it is added, with nothing in the gate
             to update.
Prove:       `./target/release/podbox-plant` exits 0 with cases 19a and 19b caught, and the gate workflow is green on `main`

**Done, 2026-09-09.** Check 19 of scripts/check-todo.py (now `podbox-gate`), cases 19a and 19b of
scripts/plant.sh (now `podbox-plant`), and `zig` in both cargo-running jobs of
`.github/workflows/gate.yml`, in this change.

⭐ **The check was run against the tree that carried the defect.** With `zig`
removed from both lists it names `build` and `lint` by job, says which file
declares the requirement and which wrapper names the component, and exits 1.
That is the same finding the nine logs carry, reached without a runner.

⚠ **Two cases, because the two arms find a job by different evidence.** 19a is a
job that says `cargo` itself, which is the shape that actually broke; 19b is a
job that only names a script which runs it, which is the shape a word match
misses. An arm nothing exercises is an arm that has stopped working, which is
[T-1202](gate.md)'s finding in a second place.

⛔ **The planted component is read out of `.cargo/config.toml`'s wrapper at run
time and never written into scripts/plant.sh (now `podbox-plant`).** Naming it there would be a
third declaration of the value this check exists to keep in one place, which is
the trap `CEILING_NUM` and `TAKEN_EXP` were each written to avoid.

---

### T-1207 The one crate that runs inside other people's processes is the one the gate does not check

Source:      `Cargo.toml:14-20`; the retired dev.sh check driver
Category:    gate
Priority:    P1
Effort:      L
Status:      done

Problem:     ⛔ **`crates/podbox-interpose` is excluded from the workspace, and
             every step of `./target/release/podbox-dev check` is scoped to the
             workspace.** `cargo fmt --all`, `cargo clippy --workspace
             --all-targets -- -D warnings` and `cargo test --workspace` all
             stop at the exclusion, so the object that is loaded into other
             people's processes is the one artefact no gate step reads.
Premise:     The exclusion is right and is not what this entry proposes to
             change: `Cargo.toml:13-18` gives the reason, which is that a
             member would share this workspace's dependency unification and
             feature resolution, and that coupling is what
             [interpose.md](interpose.md) T-0701 exists to avoid.
             ⚠ Measured on 2026-09-09 and 2026-09-10: the crate needed four
             clippy fixes (manual C-string literals, `new_without_default`, two
             `question_mark` lints) and they were found by running clippy
             against it BY HAND. Nothing would have caught them on the next
             change, and nothing would catch the next four.
             ⛔ [T-0910](deps.md)'s size ceiling has the same shape. It reads
             the workspace binary; the two `.so` files are 266,632 and 286,056
             bytes and are held to no ceiling at all, while they are embedded
             in the binary that IS held to one.
             ⛔ **CORRECTED 2026-09-21 BY READING THE TREE: the `Problem` above no
             longer holds as written.** dev.sh check (now `podbox-dev check`) runs fmt, clippy
             with `-D warnings`, the interpose build and the interpose tests
             against the excluded crate, so Approach item 1 has landed in
             substance. What has not landed is item 2 (exported-symbol count
             against the version script at the gate), item 3 (per-libc size in
             T-0910's baseline) and item 4 (third-state reporting). The
             Ruled 2026-09-22: items 2 through 4 belong in `dev.sh check` (now `podbox-dev check`),
             each with its plant in the same change. The Decision below
             carries it; what is open is the implementation, now landed
             under the Prove.
             Measured 2026-09-23 in the lane: with the version script
             removed from the link both objects still export 112 names with
             no personality on this profile, so the script-drop threat the
             Approach names is dormant on this toolchain rather than live;
             both pins stay as drift guards across toolchains and profiles.
             The same run shows cargo never relinks on a map-only edit, so
             a changed map against a stale object is a real staleness the
             gate comparison catches, not a hypothetical one.
Approach:    A second scope, not a second gate:
             1. `dev.sh check` (now `podbox-dev check`) runs the same four steps against
                `crates/podbox-interpose` with its own target and linker, which
                `scripts/build-interpose.sh` already knows how to select;
             2. the object's exported-symbol count is checked against
                `interpose.map` at the gate rather than only in
                `experiments/105-interpose-ownership.sh`, because a version
                script that stops being applied produces a working object that
                exports hundreds of names into every process it enters;
             3. its size joins T-0910's committed baseline, per libc, since two
                objects are embedded in a binary with a declared ceiling and
                growth in them is invisible in the number that IS checked;
             4. ⚠ a step that cannot run -- no `zig`, no gnu target installed
                -- reports the third state and does not read as a failure,
                which is the rule the rest of the harness already follows.
Decision:    Not taken on the shape. ⚠ Whether this belongs in `dev.sh check` (now `podbox-dev check`)
             or in a `dev.sh check --all` (now `podbox-dev check --all`) matters: check is what a change
             passes before it is committed and it is currently about 30 s, and
             a second toolchain invocation on every commit is a cost the
             operator should rule on rather than inherit.
             Ruled 2026-09-22: items 2 through 4 belong in `dev.sh check` (now `podbox-dev check`),
             each with its plant in the same change. The per-commit toolchain
             cost is accepted; the entry stays open until all three land.
Prove:       `./target/release/podbox-interpose-build` exits 1 naming the export
             mismatch on a blinded comparison and on a bogus map global;
             `./experiments/110-bloat-delta.sh interpose` writes
             `experiments/results/bloat-interpose.txt` with both sizes under
             the declared ceiling; a full `./target/release/podbox-dev check` with the
             interpose build forced to SKIP reads SKIP and stays green;
             `./target/release/podbox-plant` exits 0 with the four new cases caught.
             The committed clippy-and-fmt plant spelling is superseded:
             item 1 landed in substance without it, recorded above.

**Done 2026-09-23.** Items 2 through 4 run in `dev.sh check` (now `podbox-dev check`), each
with its plant in the same change, and the full `plant.sh` (now `podbox-plant`) run on the
committed tree reads 31 caught, 0 missed, 3 controls quiet. Every run
below is taken.

Item 2 runs in `scripts/build-interpose.sh`: each object's
`nm -D --defined-only` `T` set against the `interpose.map` globals with
`rust_eh_personality` refused, beside the DT_NEEDED, version-ceiling,
gettid and libdl asserts. Lane 2026-09-23: 112 declared and 112
exported on both objects with no personality. Guard plant: the map
input blinded reads declared 0 against exported 112 and exits 1
naming the mismatch on both. Subject plant: a bogus map global reads
declared 113 against exported 112 with the diff naming it and exits
1; the link tolerates it and cargo never relinks on a map-only edit,
so a changed map against a stale object is the staleness this
catches. Check 24 holds the step with its plant case, proven red with
its own message on a trial run.
Item 3 runs at link time under the ceiling declared once in
`scripts/build-interpose.sh` and in the gate as check 23 over
`experiments/results/bloat-interpose.txt`, taken by
`experiments/110-bloat-delta.sh interpose` (lane 2026-09-23: musl
335320, gnu 315432, binary total 3503032, the total under the binary
ceiling and each object under the interpose ceiling). Check 23
plants mirror the proven 17b/17c shapes.
Item 4 runs in `dev.sh check` (now `podbox-dev check`): each step's own status, 0 passing, 2
reading SKIP with the step named, anything else FAILED; skips never
fail the run and a run that passed nothing is red. Plant through the
real gate with the interpose build forced to SKIP: 9 passed, 0
failed, 1 skipped, exit 0. Check 25 holds the arm with its plant
case, proven red with its own message on a trial run.
Full `dev.sh check` (now `podbox-dev check`) green in the lane: 10 passed, 0 failed, 0
skipped (fmt, clippy, interpose build, release build, 495 workspace
tests, 25 interpose tests, gate, markers).

**Done 2026-10-01 (the interpose-build port).** The builder is the
`podbox-interpose-build` binary in the gate crate
(`crates/podbox-gate/src/interpose_build.rs`), keeping the 112-name
export check against `interpose.map` for both targets and the stat/statx
offset check. `scripts/build-interpose.sh` stays as a declaration stub:
it keeps the `INTERPOSE_CEILING_BYTES=` line the gate reads and names
the binary. Lane proof builds both objects beside the retired script,
then the script becomes the stub in the same change; the red-run plant
is `experiments/results/interpose-build-port-plant.txt`.


---

### T-1208 A closed entry carries its recorded run, and the gate can see it

Source:      [RULES.md](RULES.md) section 5; the reconciliation of 2026-09-11
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     [RULES.md](RULES.md) section 5 says an entry closes in place with
             its `Prove` command actually run and the output recorded underneath.
             **Nothing asserts it.** `crates/podbox-gate/src/main.rs` checks the counts,
             the rows, the fields and every citation, and it does not check the
             one thing that makes a closed entry mean anything.
Premise:     **Measured by the closure-records instrument (deleted
             2026-10-01; superseded: the closure record is generated from
             `TODO/` state, not driven by a script), which is the
             instrument and not a reader.** Re-run on 2026-09-12 at `6008985`:
             131 entries, 85 closed, **85 carry a record of the run and 0 do
             not**. The one entry that carried a `Prove` line and nothing after
             it was T-0408 in [complete.md](complete.md), and it is reopened,
             which is why the count of the unrecorded is now zero.
             **The harder half is that the record has two shapes**, and a
             reader looking for one of them under-counts badly. 81 entries open
             the record with a bold `Done` paragraph. **Four** record the run in
             prose after the `Prove` line instead, naming the test, the driven
             command or the measured count: T-0204 in [image.md](image.md),
             T-1103 in [milestones.md](milestones.md), and T-0107 and T-0108 in
             [probe.md](probe.md).
             ⚠ **T-0801 in [cli.md](cli.md) is not one of them**, and an earlier
             reading of this said it was. Its record opens `**Done, 2026-09-09.**`,
             which is the bold shape. T-0211 was in the prose set and is now
             `partial`, so it is not a closed entry to convert.
             **Both shapes satisfy the rule as written.** Section 5 asks for
             the output recorded underneath and does not name a format. So the
             defect is not in those four entries; it is that the rule is not
             machine-checkable, and a rule nothing checks drifts.
             A first count of this got the answer badly wrong, which is the
             argument for a checked rule rather than a careful reader: a grep for
             the marker with a trailing space missed every record written
             `Done.` with no date, and reported 31 entries as unrecorded when the
             true number was one.
Approach:    Fix the rule first, then check it. Give section 5 the one stated
             shape the `Decision` names, amend the four prose entries into it in
             the same change, and then add the check:
             1. every entry whose `Status` is `done` has content after its
                `Prove` field;
             2. that content opens with the bold `Done` marker, which is the
                shape section 5 will state;
             3. the plant, in scripts/plant.sh (now `podbox-plant`): delete the record from one
                closed entry and assert the gate goes red naming that entry.
             A check with no plant is not a check, and [T-1202](gate.md)
             is the rule that says so.
Decision:    ⭐ **Taken on 2026-09-12: one shape, and it is the bold `Done`
             paragraph.** A closed entry's record opens with `**Done` on the
             first unindented line after `Prove`. The four prose entries are
             converted in the same change as the check.
             **The rejected option: accept any content after `Prove` and check
             only that it exists and cites something backticked.** It rewrites
             nothing, which is its whole appeal, and it fails the one test this
             entry exists for. "Cites something checkable" is a judgement a
             script has to approximate, so the check would pass a record that
             carries one backticked word and no run at all, and the approximation
             is a second rule nobody wrote down. A literal opening marker cannot
             drift and cannot be argued with. 81 of 85 entries already write it.
             ⚠ **The cost is stated rather than denied**: four entries that obey
             the rule as written get rewritten, and their prose records were not
             wrong when they were written.
             A date on the `Status` line is **not** part of this. 21 closed
             entries carry `done` with no date and 66 carry one; section 5 asks
             for neither, so making it a rule here would invent one.
Prove:       `./target/release/podbox-gate` reports a `closure_records` coverage count equal to the number of closed entries, and the plant for it goes red naming the entry whose record was removed

**Done 2026-09-21.** Check 22 in scripts/check-todo.py (now `podbox-gate`) (every `done` entry
opens its record with `**Done` on the first unindented line after `Prove`),
its plant case in scripts/plant.sh (now `podbox-plant`), the RULES.md section 5 shape sentence,
and five records converted (T-0204, T-1103, T-0107, T-0108, T-0505).

The check found five prose records, not the four this entry names: T-0505
(`enter.md`) kept its `**Done` paragraph after a `Prove`-rewrite note, so the
first unindented line after `Prove` was the note. All five open with `**Done`
now, with their content preserved. `check-todo.py` (now `podbox-gate`) reports
`closure_records=101` against 101 `done` entries. The plant run on the
committed tree: 27 caught, 0 missed, 3 controls quiet, full `dev.sh check` (now `podbox-dev check`)
green alongside.

---

### T-1209 Thirty-nine `Prove` lines pull from the one registry the acceptance may not use

Source:      Found while correcting [T-0702](interpose.md)'s `Prove` for the same defect
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     ⛔ **A `Prove` line IS the acceptance**, because [RULES.md](RULES.md)
             section 5 closes an entry on that command actually run. So the rule
             that keeps the acceptance off a quota-bearing registry applies to
             every one of them, and 39 of the 132 in `TODO/` break it.
             [T-0206](image.md) moved every `experiments/` script off Docker Hub
             on 2026-09-09 and recorded the registry each one uses. ⚠ **The
             `Prove` lines were not part of that sweep and nobody noticed**,
             because the entry's table names scripts and a `Prove` line is not a
             script.
             ⚠ What it costs is not a broken pull. It is a pull that answers
             `HTTP 429` and reads as a broken registry rather than as somebody
             else's quota, which is the reading [T-0206](image.md)'s `Problem`
             says teaches a session to ignore `exit 2`.
Premise:     ⭐ **Counted on 2026-09-12 over `TODO/*.md`: 39 of 132 `Prove`
             lines, in ten of the eighteen files.** 35 of them name an
             UNQUALIFIED reference and five name `docker.io/` outright, and the
             two sets overlap by one line.

             | the file | lines |
             | --- | --- |
             | [complete.md](complete.md) | 8 |
             | [interpose.md](interpose.md) | 6 |
             | [supervise.md](supervise.md) | 5 |
             | [cli.md](cli.md) , [image.md](image.md) | 4 each |
             | [enter.md](enter.md) , [extract.md](extract.md) , [milestones.md](milestones.md) | 3 each |
             | [packaging.md](packaging.md) | 2 |
             | [probe.md](probe.md) | 1 |

             ⛔ **The count above is short by four, measured 2026-09-19.**
             Scoped to the `Prove` line and its indented continuations, the
             sweep finds **43** lines that ever named a Hub reference, in the
             same ten files: `complete.md` carries 9 rather than 8, `enter.md`
             4 rather than 3, `extract.md` 4 rather than 3, and `probe.md` 2
             rather than 1. Two were corrected on 2026-09-12 ([T-0702 and
             T-0706](interpose.md)), so this entry owns the remaining **41**.
             The title keeps the number the entry was filed under.

             ⭐ **An unqualified reference is the larger half and it is the one
             a reader cannot see.** `alpine:latest` appears 50 times. It carries
             no registry, so it resolves through the engine's own shortname
             aliases, and `scripts/common/distro-matrix.sh` rule 4 already
             states what that costs: the name lands at a registry where the
             digest does not exist and the failure arrives as `manifest
             unknown`, which reads as a broken pin.
             ⛔ **The rule these break is not new and it is written twice.**
             `scripts/common/distro-matrix.sh` says of the M5 row list: no
             Docker Hub, every reference `ghcr.io`, `public.ecr.aws` or the
             distribution's own. [T-0206](image.md) records the same for the
             scripts.
             ⚠ **One legitimate exception exists and the check has to know it.**
             `DISTRO_ROWS_NSSWITCH` in the same file is deliberately on Docker
             Hub, because `experiments/results/across/` was measured against
             those digests and the same tag at another registry is another
             image. A `Prove` that re-reads one of those readings is not the
             same act as a `Prove` that pulls afresh.
Approach:    ⛔ **The rule before the sweep, and the check before the edits.** A
             replacement chosen per line is 39 judgements nobody can review; a
             stated mapping is one.
             1. state the mapping in one place: every `Prove` reference is a row
                of `DISTRO_ROWS_M5`, named by its fully qualified reference, and
                a `Prove` that needs an image with no row asks for a row rather
                than inventing a reference;
             2. ⚠ **`golang:alpine` in [T-0706](interpose.md) has no row and no
                equivalent**, and it is not a distribution. Its `Prove` needs a
                Go payload rather than that image, and the entry says why the
                payload is a Go one. Decide that there and not here;
             3. the check, in `crates/podbox-gate/src/main.rs`: a `Prove` line may name
                no unqualified image reference and no `docker.io/` reference;
                ⚠ **and `README.md`'s quick start names `alpine:latest` twice**,
                which is the same reference and not a `Prove` line. It is a
                reader's own quota rather than the acceptance's, so it is not
                this entry's defect; it is written down here so the next sweep
                does not find it and think nobody looked;
             4. the plant, in scripts/plant.sh (now `podbox-plant`): put `alpine:latest` into one
                `Prove` line and assert the gate goes red naming that entry.
             ⛔ A check with no plant is not a check, and [T-1202](gate.md) is
             the rule that says so.
             ⚠ **Not in one pass with the check.** The 41 edits change what 41
             acceptance commands assert, so they land as their own change with
             the mapping stated, and the check lands once nothing violates it.

             ⭐ **The mapping, stated once, applied 2026-09-19.** Every left
             side below is replaced by the right side, which is the
             `DISTRO_ROWS_M5` row in `scripts/common/distro-matrix.sh`:

             | the `Prove` lines named | the M5 row that replaces it |
             | --- | --- |
             | `alpine:latest`, bare `alpine` | `public.ecr.aws/docker/library/alpine:3.20` |
             | `debian:12`, `debian:latest`, `docker.io/library/debian:bookworm-slim` | `public.ecr.aws/debian/debian:bookworm-slim` |
             | `docker.io/rockylinux/rockylinux:9` | `quay.io/rockylinux/rockylinux:9` |
             | `docker.io/library/archlinux:latest` | `ghcr.io/pkgforge-dev/archlinux:latest` |
             | `docker.io/voidlinux/voidlinux-musl:latest`, `voidlinux/voidlinux-musl:latest` | `ghcr.io/void-linux/void-musl:latest` |
             | `registry.opensuse.org/opensuse/leap:latest` | `registry.opensuse.org/opensuse/leap:15.6` |

             41 violating lines swept, in the same ten files the Premise
             names, plus the [T-0408](complete.md) tag aligned to its row
             (same registry, `:latest` to `:15.6`). The two history notes in
             [T-0702 and T-0706](interpose.md) name the defect without the
             literal now, so their `Prove` lines stay fully checked. Step 2
             is settled without a new row: [T-0706](interpose.md) proves its
             Go row by unit test over the section markers, so no image is
             needed and none is invented.
Decision:    ⭐ **Taken on 2026-09-12: the sweep is one entry and not 41
             corrections spread through the entries that carry the lines.** A
             correction made inside each owning entry is made 41 times by 41
             sessions against 41 readings of the rule, and the rule is what
             drifted in the first place.
             ⚠ **[T-0702](interpose.md) and [T-0706](interpose.md) are the two
             exceptions and they are corrected already**, because the work order
             sent a session to run them and a `Prove` that cannot run is not a
             `Prove`. That is 2 of the 43; this entry owns the other 41.
             ⛔ **The rejected option: leave the lines and let the check warn.**
             A warning nobody has to clear is a comment, and the gate here is an
             assertion or it is decoration.
Prove:       `./target/release/podbox-gate` exits 0 with the new check in place, and `./target/release/podbox-plant` reddens it by name

**Done 2026-09-19.** The mapping, the sweep and the check, in two changes.
The sweep moved 41 violating `Prove` lines to their `DISTRO_ROWS_M5` rows in
the same ten files the Premise names, aligned the [T-0408](complete.md) tag
to its row, reworded the two history notes in [T-0702 and
T-0706](interpose.md) to name the defect without the literal so their
`Prove` lines stay fully checked, and stated the mapping and the corrected
count in this entry. The check and its plant landed together, per
[T-1202](gate.md): check 21 in `crates/podbox-gate/src/main.rs` with cases 21a and
21b in scripts/plant.sh (now `podbox-plant`), one per arm because the two spellings fail apart.

```
$ py scripts/check-todo.py; echo EXIT:$?  # historical 2026-09 run; the gate is now `podbox-gate`
check-todo: 136 rows, 136 entries, 38 open, 3 partial, 0 blocked, 95 done
check-todo: coverage bare_citations=1327 ci_components=3 corpus=41 counts=5 crossrefs=478 entries=136 exit_codes=5 experiment_numbers=471 fields=1360 prove_registry=136 rows=136 size_ceiling=232 todo_citations=101 todo_links=559 tree_citations=64 tree_links=342
check-todo: ok
EXIT:0
$ sh scripts/plant.sh; echo EXIT:$?  # historical 2026-09 run; plants are now `podbox-plant`
  plants   26 caught, 0 missed
  controls 3 quiet, 0 fired
  every check that was planted against went red with its own message.
EXIT:0
```

⚠ `plant.sh` (now `podbox-plant`) ran under a `python3` shim on PATH pointing at the real
interpreter, because this host's `python3` is a Microsoft Store stub that
exits without running anything. The shim lived for the one command and was
removed afterwards.

---

### T-1210 Convert the interpose engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     Three interpose scripts reach an engine without the one safe
             helper: `experiments/80-interposer-abi.sh` (13 docker calls),
             `experiments/100-interpose-symbols.sh` (8) and
             `experiments/245-interpose-sweep.sh` (11). A job container cannot
             start a docker daemon (no `NET_ADMIN`, measured 2026-09-19), so
             they exit 2 in the guest lane, and on a workstation their raw
             `docker run` calls carry no timeout, no pin check, no mount
             confinement and no privileged refusal.
Premise:     Counted on 2026-09-19 by matching `docker|podman` against each
             script: 80, 100 and 245 name the engine directly and never source
             `lib/engine.sh`, while only `105` does. `105` is the converted
             shape to copy: `engine_pick` preferring a daemon and falling back
             to host podman, the conditions block naming the driver, images
             fetched before any timed clause, every run bounded and `--rm`,
             mounts read-only from declared roots.
Approach:    One script at a time, in number order, through the helper:
             1. source `lib/engine.sh`, set `ENGINE_REPO` to the checkout and
                `ENGINE_WORK` to the script's scratch, `engine_pick` or exit 2;
             2. replace each `docker run|create|cp|pull` with the `eng_*`
                spelling, pinning every image at `@sha256:` and fetching before
                any timed clause;
             3. name the driver in the conditions block, as `105` does.
             Out of scope: what each script asserts. The conversion keeps every
             clause byte-identical apart from the engine spelling, so a red
             conversion is a broken conversion and not a new finding.
             ⛔ No `--privileged` and no `--cap-add` survive the move; the
             helper refuses both, and a clause needing one is rewritten to a
             `--cap-drop` wall or filed as blocked naming what would clear it.
Decision:    Convert, do not fork the helper per script. A second engine
             spelling is the copy that diverges, which is what `105` was
             written to stop.
Prove:       `./experiments/80-interposer-abi.sh`, `./experiments/100-interpose-symbols.sh`
             and `./experiments/245-interpose-sweep.sh` each exit 0 on host
             podman with the conditions block naming `podman (host machine)`,
             and `experiments/results/` carries the three runs.
**Done 2026-09-19.** All three convert through `lib/engine.sh` and exit 0
on host podman, each conditions block naming `podman (host machine)`.
`experiments/results/` carries the runs: `interpose-abi.txt` (new),
`interpose-symbols.txt`, and `interpose-sweep.txt` with per-row
transcripts in `experiments/results/sweep245/`. No clause changed apart
from the engine spelling; the row matrix, the verdict words and the exit
contract are untouched.

```
$ sh experiments/80-interposer-abi.sh; echo EXIT:$?
  == verdict: every check that ran matched.
EXIT:0
$ sh experiments/100-interpose-symbols.sh; echo EXIT:$?
  == verdict: every check that ran matched
EXIT:0
$ sh experiments/245-interpose-sweep.sh; echo EXIT:$?
  rows 10, ran 9, virtualized 9, declined 18, host_not_runtime 0
EXIT:0
```

Two findings from the first full sweep, one fixed and one carried. First,
the two cross-libc refusals printed vacuous oks: void's
`ld-musl-*.so.1` is an absolute link to `/usr/lib64/libc.so` (measured in
the pinned image), which dangles on the staging host, so the old
`find | head -1` order staged a mount failure and called it a refusal;
the early return then leaked `/pb` and `/obj` mounts, so the second pair
died on a duplicate destination. Fixed in the script: the loop takes the
first match in sorted order that the host can read and that resolves
inside the checkout, or skips honestly; the mount-failure path clears its
mounts, as the file's own comment already promised. The green run prints
both genuine DT_NEEDED refusals through `podbox system abi`.

Second, archlinux extracts nowhere on this lane: `podbox extract` stops
on `usr/share/terminfo/l/lft-pc850` against `L/LFT-PC850`, a case-only
collision the Windows-backed shared store cannot hold. Extracting the
same pinned image with a container-local store succeeds (4 layers, 33164
entries), so the trigger is the case-insensitive backing, not the image
and not the conversion. The table records it as `no rootfs` (no-pull 1,
broken 0); it clears on a case-sensitive store, where the 2026-09-18 run
reached 10 of 10.

⚠ One wart for a later entry: a cold host-podman connection answers
`podman info` slower than the 15 s probe budget, three times in a row, so
`engine_pick` exits 2 after about twenty idle minutes (measured twice:
6 probes answered FAIL, FAIL, FAIL, OK, OK, OK at 15, 15, 16, 4, 1 and
0 s). Warming with one `podman info` before the run clears it; all three
green runs above passed the pick on a warm engine.

---

### T-1211 Convert the distribution and probe engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     Four matrix scripts reach an engine without the helper:
             `experiments/90-nsswitch-contract.sh` (8 docker calls),
             `experiments/125-across-distributions.sh` (9),
             `experiments/170-probe-cache.sh` (1) and
             `experiments/240-distro-sweep.sh` (14). Same wall as T-1210: exit
             2 in the guest lane, unbounded and unconfined on a workstation.
Premise:     Counted on 2026-09-19 as T-1210's premise was: none of the four
             sources `lib/engine.sh`. `125` is the widest (eleven pinned rows)
             and `240` is M5's own sweep, so neither may change what it asserts
             in the move.
Approach:    As T-1210, one script at a time: source the helper, pin every
             image, fetch before timed clauses, bound every call, name the
             driver. Out of scope: the rows each matrix drives and what each
             row asserts. A matrix that changes its rows in the move measures
             something new, which is T-0712's defect in another costume.
Decision:    Convert, do not re-row. The matrices stay byte-identical apart
             from the engine spelling.
Prove:       `./experiments/90-nsswitch-contract.sh`, `./experiments/125-across-distributions.sh`,
             `./experiments/170-probe-cache.sh` and `./experiments/240-distro-sweep.sh`
             each exit 0 on host podman with the conditions block naming the
             driver, and `experiments/results/` carries the runs, including one
             transcript per row for `125` and `240`.
**Done 2026-09-22.** T-1309 is fixed and proven, so the two rows
read 42 and the sweep exits 0. The other three scripts never load
the interposer (no `run` or preload path in any of them, read by
grep), so their recorded runs stand on the fixed tree.

```
$ sh experiments/90-nsswitch-contract.sh; echo EXIT:$?
  == verdict: every check that ran matched
EXIT:0
$ sh experiments/125-across-distributions.sh; echo EXIT:$?
  11 row(s) produced a reading.
EXIT:0
$ sh experiments/170-probe-cache.sh; echo EXIT:$?
  ok the first run measured; ok the second run was served;
  ok the confined run measured its own answer;
  ok the refusal names the mount namespace
EXIT:0
$ sh experiments/240-distro-sweep.sh; echo EXIT:$?
  rows 10, ran 10, built_and_ran 10, host_not_runtime 0
EXIT:0
```

---

### T-1212 Convert the image, registry and CLI engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     Six scripts reach an engine without the helper:
             `experiments/150-image-acquisition.sh` (23 docker calls),
             `experiments/270-multiarch-image.sh` (1),
             `experiments/280-insecure-registry.sh` (16),
             `experiments/300-run.sh` (7),
             `experiments/320-cli-contract.sh` (21) and
             `experiments/330-exit-codes.sh` (27). `150`, `320` and `330` are
             the widest engine users in the tree; `330` owns docker's exit-code
             contract, so an engine difference there reads as a product defect.
Premise:     Counted on 2026-09-19 as T-1210's premise was. `280` stands up a
             registry fixture, so its conversion keeps the fixture's lifecycle
             (start, wait-on-condition, teardown) and only changes the engine
             spelling that drives it.
Approach:    As T-1210, one script at a time. Out of scope: exit codes,
             templates and registry behaviour each script asserts. `330` in
             particular keeps every expected code; a code that moves under host
             podman is reported as a finding about the engines, not edited into
             the expectation.
Decision:    Convert, do not re-assert. Where host podman and a daemon would
             answer differently, the conditions block names which one drove and
             the difference is a finding, never a silent edit.
Prove:       `./experiments/150-image-acquisition.sh`, `./experiments/270-multiarch-image.sh`,
             `./experiments/280-insecure-registry.sh`,
             `./experiments/320-cli-contract.sh` and `./experiments/330-exit-codes.sh`
             each exit 0 on host podman with the conditions block naming the
             driver; `./experiments/300-run.sh` exits 2 with zero FAILs and
             one SKIP (transcript at `TODO/gate.md:1037`), and
             `experiments/results/` carries the runs.
**Done 2026-09-23.** Every half the entry named now measures on the
base lane (`wsl-toolkit-podbox`, dockerd 29.8.1, kernel
7.2.0-WSL2-STABLE). `150` exits 0: podbox and docker agree on the
index digest `b2507f19…`, confirming the 2026-09-21 mismatch was
podman's child-digest reporting. `270` exits 0: T-0212's owner fixed
the clause-5a expectation the next day (it reads the cli-error code
from the binary: 1), and the fix holds here. `280` exits 0 with all
seven clauses green after the native-lane repair below. `320` and
`330` exit 0 with their daemon halves measured against docker
29.8.1 (every 330 comparison cell ok). `300` carries zero FAILs and
one SKIP: clause 7 runs (chroot inside, namespace outside) and the
riscv64 refusal half is unmeasurable where binfmt executes it. The
binary is the shipped `podbox 0.1.0`. `experiments/results/`
carries all six runs.

```
$ sh experiments/150-image-acquisition.sh; echo EXIT:$?
  == 1. podbox's digest against docker's, for the same tag
  podbox  sha256:b2507f1964270cab3cc190aa8df858521f6a7e45e969146a012877891d5dfc9b
  docker  sha256:b2507f1964270cab3cc190aa8df858521f6a7e45e969146a012877891d5dfc9b
  == 2. a second pull fetches nothing
  == 3. every stored blob hashes to the name it is stored under
  == 4. a plain-HTTP registry is refused rather than downgraded
EXIT:0
$ sh experiments/270-multiarch-image.sh; echo EXIT:$?
  == 1. no --platform pulls the platform podbox was BUILT for
  == 2. --platform takes a DIFFERENT manifest out of the same index
  == 3. the extracted rootfs really is that architecture
  == 4. an index that offers nothing for the ask says what it does offer
  == 5. a malformed --platform is a USAGE error, before any network
    exit              1 (1 is a cli error, TODO/cli.md T-0802)
EXIT:0
$ sh experiments/280-insecure-registry.sh; echo EXIT:$?
  == 1. the default refuses an explicit http:// and NAMES the flag
    exit              1 (want 1, the cli-error code)
    names the flag    1
  == 2. the default refuses a certificate nothing trusts
    exit              125 (125 is a runtime failure)
    says              invalid peer certificate: UnknownIssuer
  == 3. --insecure-registry pulls the whole image over plain HTTP
    exit              0
    layers pulled     4
  == 4. and it changed NOTHING for any other registry
    a DIFFERENT registry over http://: exit 1
  == 5. --tls-verify=false reaches the self-signed registry
    exit              0
    layers pulled     4
    --tls-verify=false with an http:// reference: exit 1
  == 6. every downgrade is announced on stderr
    announced insecure          7
    announced the http fallback 6
    a normal pull says it       0 time(s)
  == 7. the environment and the config file reach the same place
    $PODBOX_INSECURE_REGISTRIES exit 0
    $PODBOX_CONFIG file         exit 0
    a URL where a host belongs: exit 1, registries.conf:2
EXIT:0
$ sh experiments/300-run.sh; echo EXIT:$?
  == 1. T-1104's own acceptance: stdout [hi], exit 0, mode= on stderr
  == 2. T-0802: every payload code unaltered (0, 1, 42, 137, 127)
  == 3. a chroot into the image (both report Arch Linux; asserts readability)
  == 4. -e, -w and --entrypoint all answer (hello, /etc, ep, onpath)
  == 5. arm64 runs through qemu-aarch64-static, disclosed; riscv64 executes
     through its registered interpreter here, so the refusal half SKIPs
  == 6. --pull never names what the store holds, exit 125
  == 7. inside the reconstruction: stdout [hi], exit 0, rung chroot
     (this host selects namespace)
  == 8. exec is a fresh chroot on every channel
EXIT:2
$ sh experiments/320-cli-contract.sh; echo EXIT:$?
  == 1. the table is data: 164 rows, 57 verbs, four statuses, every row noted
  == 2. the table DECIDES: None refused with its reason, unlisted refused,
     a Stub accepted
  == 3. every None verb answers 125 with its own row: six of six
  == 4. `docker` and `podman` answer with payload and banner: hi, rc 0
  == 5. T-0803's ruling: a docker daemon here: reachable; install-names
     rc=125 with the reason; --force makes the link; every name a symlink
EXIT:0
$ sh experiments/330-exit-codes.sh; echo EXIT:$?
  == 0. the table as data: flag=125 cli=1 notfound=127 invoke=126 runtime=125
  == 1. five flag-parser rows ok against the table, docker columns ok
  == 2. four verb-refusal rows ok, docker columns ok
  == 3. four payload rows ok (42, 127, 126, 0), docker columns ok
  == 4. the bare name exits 0
  == 5. fixups never move the payload's code (7 and 7)
  == 6. --strict refuses with 125, StrictOk true
  docker 29.8.1: every comparison cell ok
EXIT:0
```

             Five conversion repairs, each found by running the converted
             script whole. First, `280`'s driver mounts were staged and then
             dropped by the `eng_clear` that released the `/certs` staging,
             so every clause exited 127 on `exec: /pb: not found`; the
             script re-stages them after the fixture block. Second, `280`'s
             `pb()` handed its wrapper to `eng_pbrun`, which execs `/pb`
             with the words it is given, so podbox refused `/bin/sh` as a
             command; the wrapper now takes the store, the config and the
             insecure list as positional words through `eng_run`.
             `eng_serve` ids captured in `$( )` never reach `eng_cleanup`
             (a subshell discards the assignment), so the trap removes the
             fixtures by name; a rerun's start failed on the first run's
             still-Up fixture before this. Third, the TLS fixture died on
             an empty certs directory four times: the lane's OpenSSL reads
             a system config with an unknown option, `/dev/null` is not
             openable by a native Windows binary, `/tmp` is the shell's
             and not openssl's, and native openssl reads no msys spelling.
             The script writes an empty config beside the certificate,
             names it and the key paths through `winpath`, and refuses
             honestly where the certificate comes out empty.
             One lane-toolchain repair outside any clause: this lane's jq
             ends every raw-output line with CRLF, and the shell's command
             substitution strips only the trailing one, so `320` clause 3
             measured five verbs as `restart\r` and failed them with "no
             such command". Python reads the table's values clean, the
             jq behaviour is measured at the command line, and the loop
             strips the transport bytes before podbox ever sees them. All
             six verbs answer 125 with their rows after it.
             Fourth, `280`'s TLS fixture never reached a native lane:
             `eng_serve` mounts only what `eng_mount` staged, and only the
             non-native branch staged `$WORK/certs`, so on a native lane
             the registry exited 1 on `open /certs/domain.crt`, the TLS
             push failed, and clause 5 SKIPped on a missing fixture. The
             mount is hoisted above the branch; clause 2 reading
             `UnknownIssuer` again confirms the registry serves.
             Fifth, `300`'s riscv64 refusal half assumes an unexecuted
             architecture without checking: a lane whose binfmt runs every
             offered architecture executes the payload (rc=0) instead of
             refusing it. The half SKIPs where the candidate executes, as
             the binfmt-absent and qemu-absent halves above it do.

---

### T-1213 Convert the target-image pair and its probe consumer to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/130-probe-parity.sh:1-40`
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     `experiments/10-build-target-image.sh` (9 docker calls) builds
             the image `experiments/20-enter-target.sh` (6) enters, and
             `experiments/130-probe-parity.sh` drives `20` for T-1101's
             acceptance. None sources the helper. `130` names no engine itself
             (zero `docker|podman` lines) and still cannot run where `20`
             cannot, which is why the three convert together or not at all.
Premise:     Counted on 2026-09-19 as T-1210's premise was. `130`'s header
             (lines 1-40) states the dependency in writing: all three rows
             through `20`, plus a `--refresh` path through `30-`.
Approach:    As T-1210: `10` builds through the helper (pinned base, bounded,
             no privileged), `20` enters through it, `130` inherits both and
             keeps its three rows and its `--refresh` shape. Out of scope: the
             rung each row expects. `130` compares verdict AND errno row for
             row against `attribute.txt`; the move changes neither side of the
             comparison.
Decision:    The three convert as one unit. Converting `130` without `20`
             tests nothing, and converting `20` without `10` builds nothing.
Prove:       `./experiments/10-build-target-image.sh`,
             `./experiments/20-enter-target.sh` and
             `./experiments/130-probe-parity.sh` each exit 0 on host podman
             with the conditions block naming the driver, and
             `experiments/results/probe-parity.txt` carries the run.
**Done 2026-09-23.** Both clearing conditions hold on the native
base lane (`wsl-toolkit-podbox`, kernel 7.2.0-WSL2-STABLE):
unconfined podbox selects `namespace`, and `130 --refresh`
re-captured `attribute.txt` (with `census.txt`) on the same machine,
after which `130` exits 0 with 16 matched and 0 differed. `10`
stands on its recorded host-podman build (image id
`9ed4f5c81452`, loaded byte-identical into the base: the conditions
block names the full id); no base rebuild belongs to it (the
podman-path build's apt hangs on port 80 while the docker path
fetches: measured 2026-09-23, three runs, and the Dockerfile is a
pinned input). `20` exits 0 natively (N+F+M, `/bin/id` answers
uid 0). The binary is the shipped `podbox 0.1.0` (3503032 bytes).
The observation on bare payload names stands.

```
$ sh experiments/10-build-target-image.sh; echo EXIT:$?
  == building container-research/target:1
  [2/2] COMMIT container-research/target:1
  --> 9ed4f5c81452
  == conditions
  engine            podman version 6.1.2
  image id          9ed4f5c81452 (short form; the full ID is in 10's console output, never committed)
  == what the image has, and has not
  go                go1.24.7
  gcc               12
  python3           3.11.2
  tar               1.34
  zstd              1.5.4
  rustc             MISSING (as on the target)
  docker on PATH    podman version 5.8.2
EXIT:0
$ sh experiments/20-enter-target.sh -- /bin/id; echo EXIT:$?
  uid=0 gid=0 groups=0,65534
EXIT:0
$ sh experiments/130-probe-parity.sh; echo EXIT:$?
  == 1. inside the reconstruction, the rung must be chroot
    got chroot
  == 2. unconfined, the rung must be namespace
    got namespace
  == 3. the attribution rows: 16 matched, 0 recorded, 0 differed, 0 missing
EXIT:0
```

             The helper carries two new entry points, both ruled, both
             with refusal paths probed: `eng_build` (bounded, roots-checked,
             never privileged, never pushed; prints the image id) and
             `eng_privrun` (pinned image, bounded, staged mounts, --rm,
             plus --privileged with seccomp=unconfined and -i, announcing
             on stderr on every call; caller-passed privilege flags stay
             refused). A bare 64-hex image ID counts as pinned: it names
             content, not a tag, and locally built fixtures only ever have
             that form. The `_in_roots` extraction was proved equivalent
             against the previous spelling over seven accept/refuse cases.
             Two conversion repairs: 20's stdout is the payload channel,
             so `engine_pick` is silenced there (its line rode into 130's
             rung capture); 20 builds the Go harness for the image's
             architecture from the engine, never the host's (host go
             targets windows on this lane), and skips the C probe where
             host gcc cannot target the container.
             One wart carried, not fixed: `Dockerfile.target` opens with
             an unpinned `FROM golang:1.24.7-bookworm` stage. Pinning it
             changes the build input, which is out of scope here.
             Commit gate, 2026-09-21: the conversion is done and
             recorded but uncommitted. The Linux gate failed three
             times on the store suite, a different `two_*` victim each
             time, all with the 16-lock-slot refusal: the slot pool is
             process-wide, libtest shares it across threads, and each
             `two_*` test holds two or more locks at once. No crate
             changed under it; [PROGRESS.md](PROGRESS.md) carries the
             mechanism and the runs. The serial run came back 97 of 97
             green, and the final full-gate attempt came back fully
             green with it, so the change commits with the three reds
             named. The contention fix itself belongs to the
             T-0211/T-0215 family.

---

### T-1325 The gate checks what entries claim: reachable flags and true parity notes

Source:      issues 25 and 23, client beta testing 2026-09-22 (T-0604's
             Prove names refused `--filter`; three parity notes stale);
             `TODO/supervise.md`,
             `crates/podbox-cli/src/parity.rs`
Category:    gate
Priority:    P1
Effort:      M
Status:      done 2026-09-23

Problem:     Two claim classes with no check. Closed entries' `Prove:`
             commands can name flags the parity table refuses (T-0604
             proves itself with `ps --filter`, a `None` row, rc 125: the
             command never ran as written; only `230`'s grep-over-`--format`
             drive is real), and the class was fixed once by hand, not
             swept (`TODO/interpose.md` records its own `-v` the same
             way). And parity notes go stale in the machine-readable
             contract with no equivalent of the citation check: `inspect`
             claims no containers post-M4, `system` claims prune missing
             and "docker has neither" (docker has both `system prune` and
             `events`), `run --restart` blames M4 for a missing policy.
Premise:     Measured by the reporter on the beta.3 asset and confirmed
             on this tree: the T-0604 line still carries `--filter`, all
             three notes read as reported. `parity::TABLE` is data, so
             both checks are cheap.
Approach:    Two checks in `check-todo.py` (now `podbox-gate`, or beside it), each with its
             plant in the same change: extract `podbox <verb> <flags>`
             tokens from each `Prove:` line and admit them against the
             table the way `admit` does at runtime; assert the three
             named notes (and, by shape, every note naming a milestone or
             a missing verb) against the verbs that exist. Fix T-0604's
             line (grep over `--format`, as 230 drives it) and the three
             notes in the same change. Out of scope: semantic truth of
             every note (the check owns reachability and named
             existentials, reviews own the rest).
Decision:    Checks with plants, fixes beside them. A recorded acceptance
             that never ran is the exact failure the gate exists to catch.
Prove:       `./target/release/podbox-plant` breaks each new check on purpose and asserts red
             with that defect's message; T-0604's rewritten line runs
             green verbatim; `system info --format '{{json .Parity}}`
             carries the three corrected notes. Close issues 25 and 23
             with comments showing the plant runs and the two checks as
             the guards that stop recurrence.

**Done, 2026-09-23.** Checks 26 (done Prove commands admit) and 27
(milestone-blame and missing-verb notes) in `crates/podbox-gate/src/main.rs`,
six plant cases plus one control in scripts/plant.sh (now `podbox-plant`), in the same
change. The sweep paid at once: besides T-0604's `--filter` it found
T-0804's `--network`/`--memory` (asserted success, red at runtime),
T-0501's `--device` (its own Done says not implemented), T-0107's
`--network` (rewritten as the refusal it asserts), and T-0204's
`-af` cluster. The `-a -f` fix then exposed a deeper inversion the
lane proved empirically: the skip line names the reference, so `!
prune | grep -q <reference>` is red whenever prune works; the line
now asserts the skip (`is referenced by container gc-probe`) and the
image's survival. T-0706's dead `run -v` one-liner became the 159
script invocation; T-0504's `! ... --rootfs` stands, exempt by the
documented `!` rule (it asserts refusal, and the spelling is the
trigger). Lane prove [`pb-w25-prove`](../docs/history/lane-proofs-2026-09-23/pb-w25-prove.txt), verdict `fail=0`:
every rewritten line green verbatim, 159 exits 0, the three notes in
`system info`, crate units and clippy clean. `plant.sh` (now `podbox-plant`) green after
the commit. The guards are the two checks themselves.

**Partial, 2026-09-25.** Issue 60 extends check 27: assert a
curated docker flag list covered, so a new parser flag cannot
land without a row and a missing row cannot pass as a refusal.
The check extension with its plant lands in the same change as
the T-0801 rows above, per the gate rule that a check and its
plant arrive together. Fix area is `crates/podbox-gate/src/main.rs` check
27 with scripts/plant.sh (now `podbox-plant`). Risk if wrong is a green gate over
an omission the table exists to prevent. Prove is `plant.sh` (now `podbox-plant`)
green with the new plant case red-first.

**Done, 2026-09-25.** Check 27 carries the curated arm:
`CURATED_RUN_FLAGS` (the issue's 46) must each resolve under
`run` and `CURATED_VERBS` (its 10) must each have a verb row,
whatever their status. Prove: `./target/release/podbox-plant` on the lane
exits 0 with 40 caught, 0 missed, 4 controls quiet, 0 fired,
including `27c a curated flag with no parity row red, and it
named it` (the plant deletes the `--read-only` row and the gate
goes red naming it). The in-binary twin is
`issue_60_curated_surface_stays_covered`. Guards are the check
itself and its plant, which arrived in the same change.

---

### T-1326 `check-markers.sh` builds its marker bytes portably across `/bin/sh`

Source:      issue 24, client beta testing 2026-09-22 (5550 false
             positives under dash); `scripts/common/check-markers.sh:167`
Category:    gate
Priority:    P1
Effort:      S
Status:      done 2026-09-23

Problem:     The marker constants are built with `M1=$(printf
             '\342\233\224')`, and dash's `printf` inside a command
             substitution mis-encodes bytes at or above 0x80, so the
             check does not recognise its own markers and reports every
             marker in the tree as illegal. `check-gate.sh` and CI run
             the sh half with `sh`, so a dash host sees the repo gate red
             with 5550 false positives while CI stays green.
Premise:     Measured by the reporter with byte-level instrumentation
             (dash emits `e2 81 81 9b 94` for want `e2 9b 94`), including
             the minimal repro and the two non-fixes (`\0ooo` reads as
             `\034` plus digits under dash; `printf '%b'` mis-encodes the
             same way).
Approach:    Build the bytes inside the awk pass (`BEGIN` with
             `sprintf("%c%c%c", ...)` under `LC_ALL=C`) or write them to
             a file once and read them back with `cat`; never
             `$(printf ...)` for non-ASCII bytes. Prove under both dash
             and bash. Out of scope: changing the five markers, changing
             what the check scans.
Decision:    The awk-`BEGIN` shape (no new files, one language already in
             use), unless the drive shows gawk absent where the check
             must run, in which case the file shape wins.
Prove:       `sh scripts/common/check-markers.sh` (dash) and
             `bash scripts/common/check-markers.sh` both exit 0 with the
             same count on this tree; a planted illegal byte fails under
             both. Close issue 24 with a comment showing both runs and
             the dual-shell drive as the guard that stops recurrence.

**Done, 2026-09-23.** The awk-`BEGIN` shape, as decided: the six
constants travel as decimal bytes and assemble via `split` plus
`sprintf("%c%c%c")` in `BEGIN` under `LC_ALL=C`; the shell never
builds non-ASCII. Lane prove [`pb-w26-prove`](../docs/history/lane-proofs-2026-09-23/pb-w26-prove.txt), verdict
`fail=0`: the old shape under dash exits 1 with 5628 false
positives; dash and bash both exit 0 with byte-identical output
(378 files, 5683 markers); a planted `U+00E9` fails under both,
named. The fixed check caught three live em dashes in
`crates/podbox-cli/src/lifecycle.rs` from this session's own T-1323
work, scrubbed in the same change. Finding, out of scope:
`scripts/doctor/doctor.sh:650-652` builds the same way for display
(a warning glyph, no gate comparison), so it mis-renders rather than
mis-fires; named here, not changed. The guard is the dual-shell
drive: any new `$(printf)` non-ASCII breaks dash first.


---

### T-1338 A performance harness with baselines and a regression gate

Source:      issue 57, 2026-09-25 (claims scattered across
             single-purpose experiments); `experiments/190-parallel-layers.sh`
             with `TODO/image.md` T-0207, `TODO/podvm.md` T-1308,
             `TODO/deps.md` T-0910, `TODO/packaging.md` T-1001
Category:    gate
Priority:    P2
Effort:      L
Status:      done

Problem:     podbox's performance claims live in single-purpose
             experiments, each on one host and one shape: the pull
             pool ratio (T-0207), the TCG guest overhead (T-1308),
             binary size and bloat (T-0910, T-1001). Nothing answers
             how fast podbox is here, against what budget, and
             whether today's commit regressed it.
Premise:     Read: the cited experiments each name one host and one
             shape, and no harness aggregates them. Seed numbers on
             record are the pull pool 1.78x on container loopback
             (T-0207, the latency-bound figure still missing) and the
             TCG guest 1.5x to 11.5x across four classes (T-1308).
Approach:    One harness under `experiments/` emitting
             machine-readable rows (commit, host, kernel, arch, qemu
             version, filesystem, network shape, metric, value,
             unit), with results in `experiments/results/perf-SHAPE.txt`
             and the exit contract 0 ok, 1 over budget, 2 could not
             run. Metrics are cold and warm wall time with peak RSS
             for probe, pull by tag and by digest, extract, store GC,
             run, create, start, exec, stop, ps, logs, cp, save, load,
             each ladder rung, guest boot and command latency under
             TCG against KVM, `man`, and binary size with PT_INTERP
             state. Shapes cover tiers and rungs, bare metal, VM,
             WSL2, container and the constrained host, all seven
             archs, offline, loopback, bandwidth-bound and
             latency-bound links, and tmpfs, ext4, overlayfs and
             NTFS-backed filesystems. Baselines commit per shape with
             the seed numbers folded in, and the gate compares the
             newest reading under a ceiling with a stated tolerance
             and fails on regression, recording could-not-run as its
             own state. No tuning before the baseline exists.
Decision:    The harness lands before any optimization follow-up.
             The gate check with its plant lands in the same change
             as the check. This entry closes when the harness, one
             baseline per shape, and the gate land; the first run
             files optimization follow-ups as new entries.
Out of scope: any claim of hardware isolation from a TCG number
             (every figure carries its rung, per T-1301 and T-1306),
             tuning.
Prove:       `./experiments/360-perf-harness.sh` exits 0 on the
             constrained sandbox and on one KVM-capable host emitting
             every metric above; the gate fails on a planted
             regression past tolerance and reports could-not-run as
             its own state.

**Done 2026-09-26.** One harness answers how fast podbox is here:
`experiments/360-perf-harness.sh` (shape `lane` or `kvm`) emits
ten-column TSV rows (commit, host, kernel, arch, shape, metric,
cond, value, unit, state) for cold pull by tag and digest, probe
and extract; three payload runs named by order; the lifecycle
verbs; cp, save, load, prune and man; each forced ladder rung
that enters (the rest record refused); guest boot and command
under TCG or KVM via the pinned kernel and a marker initramfs
assembled with `experiments/lib/podvm-guest.sh`; and binary
size with PT_INTERP state. Every verb runs under `timeout`, so
a hung verb is a failed metric, never a hung drive; every
missing tool is could-not-run with its name. The budget lives
in `experiments/perf-ceilings.tsv` (about twice the observed
lane maximum, tolerance 0.10; the size ceiling repeats
110's 8,000,000); the comparison lives only in check 30, which
holds every ok row under its ceiling and treats failed and
could-not-run as their own states. The Premise seed numbers are
superseded by the re-drives: the pool share is 0.394 loopback
and 0.469 latency-bound (2.53x and 2.13x speedups inverted so
the ceiling holds one way), and the TCG workload ratios are
7.5x int, 10.7x sys, 1.4x mem and 4.9x io; all folded as seed
rows with their source reports, not re-measured.
Prove: `360` exits 0 on the lane
(`experiments/results/perf-lane.txt`: the full matrix green,
rundir/cache/memfd enter, tmpfs/fuse refuse at 125) and on
the KVM base (`experiments/results/perf-kvm.txt`: KVM guest
boot 0.935 s at `experiments/results/perf-kvm.txt:133`, forced rungs refuse where
the extracted tree carries device nodes the rung will not
stage, memfd could-not-run with no toolchain). The harness
surfaced one real defect on its first run: `load` staged
blobs into `temp_dir()` and renamed them into the store,
which is EXDEV wherever /tmp and the store differ
(tmpfs /tmp against an overlayfs store on the lane). The fix
stages inside the store through `Store::stage` beside
`import`'s own call, holding the staging handle to commit;
unit guard `load_stages_inside_the_store` fails where the
staging path leaves the store root, and the re-driven `load`
row is green on both shapes. The run curve (first ~1.6 s,
repeat ~2 s, late ~0.08 s, on both shapes) is recorded under
order-named metrics with its cause isolated in T-1340. scripts/plant.sh (now `podbox-plant`) case 30 inflates the `run.repeat`
row past its ceiling and the gate goes red naming the
regression.

**D-6 verdict 2026-10-01 (the 360 subject stays shell).** The perf
harness is not trivially separable, so no seventh `podbox-perf` binary
lands in this crate. The writer pulls two pinned images, downloads a
guest kernel, assembles a marker initramfs, boots it under TCG or KVM
with console polling, and times every verb with peak RSS: that needs an
HTTP client, an OCI pull path, and guest orchestration no std-only
binary carries, and the comparison half already lives in the gate as
check 30. A port is follow-up work with its own entry, not a corner of
this one.

----

### T-1340 Isolate why early payload runs cost seconds and late ones do not

Source:      T-1338's first lane drive (`experiments/results/perf-lane.txt`):
             `run.first` 1.648 s, `run.repeat` 2.050 s, `run.late`
             0.084 s on the same store and image
Category:    gate
Priority:    P3
Effort:      M
Status:      done

Problem:     Three `run` timings across one harness drive decay
             twenty-fold (1.6 s, 2.1 s, 0.08 s) with no isolated
             cause. The harness records the curve honestly under
             order-named metrics, but a 2 s run nobody understands is
             a budget set by superstition: the ceilings in
             `experiments/perf-ceilings.tsv` cover it without
             explaining it.
Premise:     Read on 2026-09-26: the three rows with their conditions
             in the lane report above. First-payload fixups (completion
             staging, interpose writes) run once per image and cannot
             explain the second slow run; page-cache warmup fits the
             curve but is unisolated.
Approach:    Reproduce the decay in isolation (repeated runs against
             a hot and a cold store, caches dropped where the lane
             permits it), instrument which phase pays (probe, fixups,
             extract verification, payload start), then budget or fix
             what the instrument names. Three candidates before
             testing, test to refute.
Decision:    A cause first, then a number: no ceiling moves on a
             guess, and no optimization lands without the harness
             showing the gain on a re-drive.
Out of scope: changing any ceiling before the cause is isolated,
             tuning anything else.
Prove:       `./experiments/360-perf-harness.sh` re-driven with
             phase timing names the paying phase, and the ceilings
             move only on the isolated cause.

**Done 2026-09-27.** Cause isolated on the lane (lane-built musl
debug binary, kernel `7.2.0-WSL2-STABLE`):
`experiments/368-run-decay.sh` exits 0
(`experiments/results/run-decay-368.txt`), 11 of 11 predictions
held with 16-to-69-fold separation. The `--rm` arm in
`crates/podbox-cli/src/run.rs:754-784` deletes the extracted
rootfs where `referencing()`
(`crates/podbox-supervise/src/lib.rs:305-311`) answers empty, so
each keeper-less ephemeral run re-pays full extraction (a1
1.746 s with the tree present; a2 4.776 s and a3 4.778 s
re-extracting; rootfs absent after each). `create` re-extracts
once (4.764 s) and pins the tree through its container record
(`keeps the rootfs` on stderr, payload stdout clean); kept runs
cost 0.074 to 0.078 s (b1 through b3). `rm` leaves the tree
(present after, 0.002 s); the post-rm warm run costs 0.108 s
(c1, then deletes), the next re-pays extraction (c2 5.129 s).
The warm probe storm alone costs 0.059 s, which bounds every
flat per-run cost below 0.1 s and refutes a seconds-level
per-run mechanism. Verdicts: deletion confirmed; page-cache
warming and one-time steps (rehash `if !linked` at
`crates/podbox-complete/src/pkg.rs:780`, CA marker at
`crates/podbox-complete/src/pkg.rs:429`) plausible for the
~1.6 s first-run premium, unsettled between them; the settler
is a cache-drop run with keeper, which needs privilege the lane
lacks. The 360 rows reread through this: `run.repeat` stays
slow because the two warmups each delete the tree; `create`
(1.826 s lane, 2.138 s kvm) re-extracts and pins; `run.late`
(0.084 s lane, 0.090 s kvm) rides the pinned warm tree after
`rm`; `cp` 0.949 s fits a re-extraction after `run.late` deletes; the
kvm shape repeats the pattern with no exception. No ceiling
moves: every committed perf row holds under its ceiling with
the cause named, and the 5 s run budget covers
extraction-cost variance on this lane; redefining the run rows
with a keeper so they budget run cost is next work, not this
entry.

----

### T-1341 Budget the warm run cost beside the cold one

Source:      T-1340 Done (`experiments/results/run-decay-368.txt`):
             keeper-less runs re-pay extraction (1.746 to 5.129 s)
             while kept runs cost 0.074 to 0.108 s
Category:    gate
Priority:    P3
Effort:      S
Status:      done

Problem:     `run.first`, `run.repeat` and `run.late` all budget
             extraction variance under run names (5 s ceilings), so a
             run regression hides inside extraction noise and no row
             measures the warm steady-state cost the budget claims
             to hold.
Premise:     Measured 2026-09-27 (368): with a keeper alive three
             kept runs cost 0.074 to 0.078 s, and the warm probe
             storm alone costs 0.059 s.
Approach:    Additive rows only: one `run.kept` beside `start` in
             `experiments/360-perf-harness.sh`, measured with the
             lifecycle keeper alive, with a 0.25 s wall ceiling
             (about twice the observed lane maximum) and the
             sibling rss ceiling; re-drive the lane and the kvm
             shape.
Decision:    `first`, `repeat` and `late` keep their definitions
             and their history, and the plant 30 anchor
             (`run.repeat`) does not move.
Out of scope: redefining the old rows, moving any old ceiling.
Prove:       `360` exits 0 on the lane and on kvm with `run.kept`
             ok under its ceiling, `check-todo.py` (now `podbox-gate`) check 30 green,
             and `plant.sh` (now `podbox-plant`) case 30 still goes red naming
             `run.repeat`.

**Done 2026-09-27.** `run.kept` measured beside `start` in 360
with the lifecycle keeper alive, ceilings 0.25 s wall and the
sibling rss. `360` exits 0 on the lane (`run.kept` 0.030 s,
`experiments/results/perf-lane.txt`) and on the kvm base
(`run.kept` 0.060 s, `experiments/results/perf-kvm.txt`);
check 30 green on both shapes; the plant suite goes 42 caught
0 missed with case 30 red-naming `run.repeat`. The old rows
keep their definitions and their history.

----

### T-1343 The Windows lane uses toolkit 6 job inputs and checks every retained job

Source:      podbox issue 68; the installed `wsl-toolkit 6.0.0` manual;
             `scripts/windows/run-in-base.sh`; scripts/check-todo.py (now `podbox-gate`)
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     The Windows job wrapper put each caller's script at one shared
             path in the checkout. Two callers could replace that path during
             a copy. Check 29 read only three kinds of kept work and let an
             ended base session pass.
Premise:     The installed manual specifies `run --input NAME=FILE`,
             `--container-lifecycle ephemeral`, and a JSON cleanup report.
             The live run in
             `experiments/results/windows-lane-v6.txt` found that an
             ephemeral container still leaves its host job record.
Approach:    Send each caller's LF script as its own input. Let the toolkit
             restore executable modes during the tree copy. Use the ephemeral
             container life cycle. Read all four cleanup lists in JSON, and
             plant a kept session to prove the check fails.
Decision:    Keep CRLF removal for input bytes. Keep job records until their
             result is saved, then collect each by id. Do not remove a base
             that the next session needs.
Prove:       `sh experiments/384-windows-lane-v6.sh` and
             `./target/release/podbox-gate` exit 0; `./target/release/podbox-plant`
             reports case 29b red with a kept session.

**Done 2026-09-28.** Two simultaneous wrapper jobs read their own input,
found the executable mode, and returned zero. A third caller path probe
returned the expected no-binary code and named `/work`. The measurement
found three host job records after the containers ended. Job-specific
cleanup removed them.
Check 29 then failed against a planted live base session, named its id, and
passed after that session was collected. The full plant and Linux gate are
recorded in [PROGRESS.md](PROGRESS.md).

----

### T-1344 Experiment jobs find the checkout after the Windows input change

Source:      T-1343; the caller scan in `experiments/`; the installed
             `wsl-toolkit 6.0.0` input contract
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Experiment comments still said the Windows wrapper copied
             a script to the checkout root. That path no longer exists.
             Two scripts also resolved their repository path from the
             input file's location, which is outside the checkout.
Premise:     A source scan found twelve scripts that already take the
             checkout from the working directory. The other two
             derived it from `$0` for native runs. The wrapper now
             enters `/work` before it runs `/in/job.sh`.
Approach:    Amend each caller comment in place. In the two scripts,
             use the working directory for the Windows input and retain
             file-relative discovery for native runs. Derive the default
             binary path from the selected checkout.
Decision:    Keep the two invocation shapes explicit. Do not infer a
             checkout from the input file's parent directory.
Prove:       `sh -n experiments/351-signal-forward.sh` and
             `sh -n experiments/352-ascii-output.sh` exit 0;
             `sh experiments/384-windows-lane-v6.sh` exits 0 with
             two separate inputs and `/work` executable modes.

**Done 2026-09-28.** The scan found no old input-path reference in an
experiment caller except T-1343's negative check for the removed file.
All fifteen changed experiment scripts parsed under `sh -n`. The
352 caller path probe in 384 returned 2 as expected for an image with
no binary and named the correct `/work` binary path.

----

### T-1345 The Windows job wrapper runs a Bash caller with Bash

Source:      the live plant run on 2026-09-28;
             `scripts/windows/run-in-base.sh`; scripts/plant.sh (now `podbox-plant`)
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     The wrapper ran every caller file with `sh`. A Bash caller
             failed at `set -o pipefail` before it could run its checks.
Premise:     The job input keeps its first line. The plant script names
             Bash in that line. The default image carries Bash; the small
             test image runs POSIX callers with `sh`.
Approach:    Read the first line in the job container. Use Bash for either
             standard Bash path. Use `sh` for other shell input. Run the
             plant script through the wrapper and check its result.
Decision:    Select the interpreter from an explicit first line. Do not
             infer it from a file name. Keep POSIX jobs usable in images
             without Bash.
Prove:       `sh scripts/windows/run-in-base.sh ./target/release/podbox-plant` exits 0
             and reports every plant caught, with no missed case.

**Done 2026-09-28.** The first wrapper run failed with exit 2 at
`set -o pipefail`. The wrapper now selects Bash for a Bash first line.
The repeat result is recorded in [PROGRESS.md](PROGRESS.md).

----

### T-1346 The plant script finds the checkout from a Windows job input

Source:      the live plant run on 2026-09-28; scripts/plant.sh (now `podbox-plant`);
             `scripts/windows/run-in-base.sh`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     After the Bash caller started, the plant script took its
             checkout from `/in/job.sh` and looked for `/scripts`.
Premise:     The Windows wrapper enters `/work` before it starts an input.
             A native plant run still starts from its file in `scripts/`.
Approach:    For the exact input path, use the working directory as the
             checkout. Keep file-relative discovery for native runs.
             Drive the full plant suite through the Windows wrapper.
Decision:    Do not infer a checkout from an arbitrary input directory.
             Keep the native path for direct Linux use.
Prove:       `sh scripts/windows/run-in-base.sh ./target/release/podbox-plant` exits 0
             with every plant caught and no missed case.

**Done 2026-09-28.** The first Bash run reached the path guard and
returned 2 for `//scripts/check-todo.py` (now `podbox-gate`). The repeat result is in
[PROGRESS.md](PROGRESS.md).

----

### T-1347 Check parity notes against the milestone entries

Source:      the full plant run on 2026-09-28; scripts/check-todo.py (now `podbox-gate`);
             [milestones.md](milestones.md)
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Plant case 27a stayed green after a parity note blamed M4.
             Check 27 looked for one former sentence in PROGRESS to learn
             which milestones shipped. The current progress page has no
             such sentence, so the check treated none as shipped.
Premise:     Each milestone has its own entry and `Status:` field in
             `milestones.md`. The gate already parses those entries.
Approach:    Take shipped milestone numbers from done entries in that
             file. Compare a parity note's named number against that set.
             Run case 27a and the full plant suite.
Decision:    The milestone entry is the status source. The progress page
             carries the current work order and need not repeat that set.
Prove:       `sh scripts/windows/run-in-base.sh ./target/release/podbox-plant` exits 0
             with case 27a caught and no missed plant.

**Done 2026-09-28.** The first full run caught 42 plants and missed
27a. The corrected check reads the milestone entries. The repeat
result is in [PROGRESS.md](PROGRESS.md).

----

### T-1348 Check current documents against source declarations

Source:      repository audit 2026-09-30; current source and saved results
Category:    gate
Priority:    P1
Effort:      L
Status:      done

Problem:     Current pages disagree with manifests, mechanisms, and command dispatch.
Premise:     The earlier record gate checked links and counts but not these source facts.
Approach:    Generate the build surface. Correct the live pages. Add a mismatch check and plant.
Decision:    Source and measured proof take precedence over an earlier page.
Prove:       `python3 scripts/document-state.py` and `./target/release/podbox-plant` exit 0; a changed snapshot fails with its own message.

**Done 2026-09-30.** The source-state generator and check 31 are implemented.
The full Linux gate passed. Plant 31 produced its own mismatch message.
All 44 plants failed as required and four clean controls stayed quiet.
See [the Linux proof](../experiments/results/repo-audit-linux.txt) and
[the plant proof](../experiments/results/repo-audit-plants.txt).

----

### T-1349 Verify build freshness from input and output bytes

Source:      repository audit 2026-09-30; current source and saved results
Category:    gate
Priority:    P1
Effort:      M
Status:      done

Problem:     The old build stamp can accept changed files with equal size and time.
Premise:     pg-toolkit buildplan hashes input names, content, conditions, and output.
Approach:    Hash actual build inputs, including C shims and embedded objects. Reject changed output. Build objects before CLI.
Decision:    Keep a versioned local record. Do not use timestamps as proof.
Prove:       `python3 experiments/393-build-freshness.py` exits 0 with changed source and output refused; full dev check returns 0.

**Done 2026-09-30.** The record hashes input names, bytes, build conditions,
and all five executable outputs. The fixture rejects equal-size and
equal-time source changes and changed helper bytes. Interposer generation
precedes CLI builds. The full Linux gate passed.
See [the fixture proof](../experiments/results/build-freshness.txt) and
[the full check](../experiments/results/repo-audit-linux.txt).

----

### T-1350 Make Windows proofs use explicit inputs and return build artifacts

Source:      repository audit 2026-09-30; current source and saved results
Category:    gate
Priority:    P1
Effort:      M
Status:      partial

Problem:     The unpublished KVM driver reuses an experiment number and private paths. The default job does not export the built binary.
Premise:     The KVM host is the toolkit base. The Windows checkout is read-only there.
Approach:    Use explicit binary and image inputs, pinned image bytes, and owned scratch. Export the complete build on request.
Decision:    Preserve the base and supplied image. Save proof before collecting its session.
Prove:       `sh experiments/392-kvm-guest.sh --accept-host-risk --unattended --binary BINARY --image IMAGE` exits 0; the default wrapper exports podbox and four helpers; no owned job or scratch remains.

**Partial 2026-09-30.** Explicit inputs, unique experiment numbering,
owned runtime scratch, emulator cleanup, and artifact export are implemented.
The default wrapper passed all 12 steps and exported five executables.
[The wrapper result](../experiments/results/repo-audit-default.txt) and
[the byte check](../experiments/results/exported-build.txt) prove that path.
The fresh KVM repetitions failed. They are saved in
[the first result](../experiments/results/kvm-guest-2026-09-30-first.txt) and
[the second result](../experiments/results/kvm-guest-2026-09-30-second.txt).
The final driver uses a bound that includes both setup waits. Its shared
selector checks the executable field before it stops an owned emulator.
Experiment 400 proves that the observer and another process are excluded. Its current result is linked from
PROGRESS. A successful guest run remains acceptance; T-1112 owns the
current setup and guest startup faults.

**Unattended run 2026-10-02, three attempts, all red, and a new fault.**
The operator's 2026-10-02 authorisation was used: every attempt ran
`experiments/392-kvm-guest.sh --accept-host-risk --unattended`, so the
T-1609 watchdog held each session and the run never needed a person in
the room. The binary was built in the lane on `rustc 1.99.0`, musl
static, podbox 0.1.0-beta.12, SHA256
`e5d1d918a230a45fc06be6a78189a57e3d4bc132a7070f1072e28b542d8ce4aa`,
against the pinned 910163968-byte ValidationOS disk whose SHA256 matches
the recorded digest. All three attempts are saved:
[the first](../experiments/results/kvm-guest-2026-10-02.txt) and
[the third](../experiments/results/kvm-guest-2026-10-02-third.txt).

- Attempt 1 failed before the guest: `podbox windows: qemu-img is not on
  this machine`, exit 125. T-1610 had installed `qemu-system-x86`,
  `qemu-system-x86-firmware` and `edk2-ovmf`, and `qemu-img` ships in a
  **separate** Arch package the base did not have. `qemu-img 11.1.1-4` is
  now installed in `wsl-toolkit-podbox`; T-1610's Prove clause names
  three packages and this fourth one is not among them.
- Attempts 2 and 3 got further and then failed at a step no earlier run
  reached. `windows setup` completes, printing `INSTALLED D:`: the
  provision boots the same guest on the same firmware, types the
  installer into the console, and the guest runs it and powers itself
  off. The `run ver` step then hangs. The serial log stops at
  `BdsDxe: starting Boot0002 "UEFI QEMU NVMe Ctrl wqroot 1"`, which is
  firmware handing off to the Windows boot manager, and
  `podbox windows: the guest did not power off within 540s, so it was
  stopped` follows with exit 125.

What this rules out, measured rather than assumed: the accelerator is
genuinely in use, since the outside observer sees
`-accel kvm -cpu host`; the image is the pinned one; `/dev/kvm` is
`crw-rw---- root kvm` and the guest account is in group `kvm`; the
firmware is `edk2-ovmf 202608-1`, which is a package-file mtime of
2026-08-26, so it cannot have arrived between the passing 2026-09-29 run
and today; and T-1610's premise that the base had no OVMF is consistent,
because `TODO/PROGRESS.md` records that the base was rebuilt from
scratch on 2026-09-30 after the host failure.

**The serial log is not a hang location, and the record said otherwise.**
`experiments/lib/kvm-guest-base.sh:158-161` claims the serial watcher
exists so a future hang can say where it stops. It cannot: the
2026-09-30 third run has the same BDS-only serial ending and its `ver`
had already succeeded. Windows writes nothing to serial after the
firmware handoff. That comment is a claim the saved results contradict,
and a future session reading it will look in the wrong place.

The next measurement is in the entry that owns the defect, and the
distinguishing fact is already known: `windows setup` boots a Windows
guest to completion on this firmware, so a hang that only a later boot
sees is not firmware and not the image. It is the per-run boot
specifically, and the run plan and the provision plan differ in what they
put on the command line. The entry stays `partial` because a successful
guest run remains acceptance, and T-1112's KVM leg rides it.

**Host failure 2026-09-30.** The third run left a KVM emulator that SIGKILL
did not remove. A later session of the same agent stopped the Windows host,
and the operator removed the WSL distributions. The base enforces no memory
limit. The driver now refuses without `--accept-host-risk`, beside another
emulator, or with less than 6144 MiB available. The operator permitted an
unattended run on 2026-10-02, so the next run adds `--unattended` and holds
a T-1609 watchdog on the session rather than needing a person in the room.

⛔ **DEFERRED BY THE OPERATOR, 2026-10-02. Do not start a KVM guest for
this entry until the operator lifts it.** The driver, the watchdog, the
base and the image are all in place and stay in place; what is set aside
is driving them again. `TODO/RULES.md` section 11 carries the decision
and its clearing condition: the operator saying the KVM work resumes.
Not elapsed time, and not a green record. The entry stays `partial` with
its measured state above: three runs on 2026-10-02, `setup` completing
with `INSTALLED D:`, and `run ver` hanging after the firmware handoff,
which no fix has yet been shown to explain. T-1641 and T-1112 are
deferred with it.

----

### T-1351 Keep the failed check's complete diagnostic

Source:      repository audit; scripts/common/check-gate.sh and check-gate.ps1
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     The runner prints the first twelve lines and removes the log.
             A later failure message is lost. The PowerShell twin path
             also classifies an unavailable check as a failed check.
Premise:     The audit's twin failure starts with successful doctor fields.
Approach:    Print the complete failed output before cleanup. Test a message
             after line twelve. Keep exit 2 as skipped on both runners.
Decision:    Preserve machine JSON. Do not make a missing diagnostic a pass.
Prove:       `./target/release/podbox-smoke --diagnostics --powershell` exits 0
             with both runners, lost-output mutations, clean controls,
             JSON verdicts, and skipped twin checks.

**Done 2026-09-30.** Both runners retain the late diagnostic. The mutation
restores truncation and loses that message. Clean controls and JSON verdicts
pass. Both runners report an unavailable twin as skipped.
[The proof](../experiments/results/gate-diagnostics.txt) records both platforms.

**Done 2026-10-01 (the smoke port).** The drive is the `podbox-smoke`
binary's `--diagnostics` verb in the gate crate
(`crates/podbox-gate/src/smoke.rs`), with `--powershell` for the Windows
runner. Lane proof runs the drive green beside the retired script, then
deletes experiments/398-gate-diagnostics.py in the same change; the
red-run plant is `experiments/results/smoke-port-plant.txt`.

----

### T-1422 Plant the kept-session check where the OS execs batch files

Source:      this session's plant run; scripts/plant.sh (now `podbox-plant`)
Category:    gate
Priority:    P2
Effort:      S
Status:      done

Problem:     Plant case 29b misses on Windows. Its mockbin vehicle is an
             extensionless script, and Windows `CreateProcess` rejects it
             with WinError 193, so `check-todo.py` (now `podbox-gate`) records "the lane-job
             ledger could not be read" and never names `plant-session`.
             The gate check itself works: it named every real kept job
             this session.
Premise:     `check-todo.py` (now `podbox-gate`) invokes `wsl-toolkit` through `subprocess`
             with no shell, so the mock must resolve to a vehicle the OS
             can exec. `shutil.which` prefers the runnable form per OS.
Approach:    Ship a `.bat` twin of the mock beside the extensionless one
             and leave the assertion (red, names `plant-session`, base
             quiet) unchanged.
Decision:    Both vehicles stay. The plant asserts the gate, not the OS.
Prove:       `./target/release/podbox-plant` exits 0 with 44 caught, 0 missed,
             and 4 quiet controls.

**Done 2026-09-30.** Both mocks ship; `which` takes the `.bat` on
Windows and the extensionless script elsewhere. The plant run is green
end to end ([the log](../experiments/results/plant-2026-09-30.txt)).
The minimal repro is an extensionless mock through `subprocess.run`
raising where the `.bat` twin returns the ledger JSON.

### T-1603 Remove `check-no-secrets` and add a pinned trufflehog scan workflow

Source:      Operator decision 2026-10-02; CI run 36894578988 job
             110478236087; `scripts/dev-lane.sh` `:205`;
             `docs/security/secrets.md`
Category:    gate
Priority:    P0
Effort:      M
Status:      done

Problem:     `check-no-secrets.sh --public` exits 1 on
             `scripts/dev-lane.sh:205`, `export
             HOME="${LANE_HOME:-/home/toolkit}"`, read as an absolute home
             path. That literal is a lane runner's default `HOME`, not a
             credential. A shape-matching regex standing in for a
             secrets scanner produces this class of false positive
             permanently, and narrowing its patterns one at a time is
             unpaid work that never ends.
Premise:     Reproduced on the current tree, not carried from a report:
             `sh scripts/common/check-no-secrets.sh --public` exits 1 with
             two categories matched, one being this line. The check's own
             text already says a false positive should be handled by
             narrowing the pattern "rather than switching the check off",
             and the operator has now overruled that for this case.
Approach:    Remove the check and replace it with a real scanner. Call
             sites, all of which must be unwired or the tree keeps a
             check nothing runs: `.github/workflows/gate.yml:89`;
             `scripts/common/check-gate.sh:112` and `:123`;
             `scripts/common/check-gate.ps1:111` and `:118`;
             `crates/podbox-gate/src/smoke.rs:44`;
             `scripts/common/check-twins.sh:303` and `:304`. Delete
             the shell check and its PowerShell twin, which exist only
             to be twins of each other; both names are gone from the
             index as of 2026-10-02 and this paragraph keeps no citation
             to a file a fresh clone cannot resolve. Add a trufflehog
             workflow
             pinned to a commit SHA like every other action in this
             repository, scanning the working tree and the history.
             Update `docs/security/secrets.md`, which currently names the
             shell script as the review mechanism. Update the comments
             at `crates/podbox-ssh/src/mux.rs:1151` and
             `crates/podbox-ssh/tests/mux_two_client.rs:1290`, which
             cite the check as the reason for their placeholder shapes,
             and `experiments/270-multiarch-image.sh:117`.
Decision:    Scanner, not pattern. GitHub Actions runs its own scoped
             ephemeral token; the operator directed this on 2026-10-02.
             A grep cannot tell a checksum from a key, and this tree
             carries both by design. Do not keep the scripts as dead
             weight: check-twins.sh asserts on the pair, so leaving them
             means either a failing twins check or a check comparing
             nothing.
Prove:       `./target/release/podbox-gate` exits 0; `sh scripts/common/check-twins.sh`
             exits 0 with no `check-no-secrets` pair; `grep -rn check-no-secrets .`
             outside `references/` and `experiments/results/` returns
             nothing; the new workflow appears in a CI run on the landed
             commit and reports a clean scan; a planted fake key in a
             tracked file makes that workflow fail (the plant this task
             owes, replacing what the deleted check planted).

**Done 2026-10-02, with two clauses open.** The check and its PowerShell
twin are deleted, every call site is unwired, and
`.github/workflows/secrets.yml` replaces it. Measured:
`grep -rn check-no-secrets .` outside `references/`,
`experiments/results/`, `TODO/`, and the untracked scratch trees
returns nothing; `sh scripts/common/check-twins.sh` exits 0 with every
remaining pair agreeing; `sh scripts/common/check-docs.sh` exits 0 at 84
files and 1130 relative links; and
`./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0. The
deletion and the workflow landed in one commit, so the repository never
sat without secrets scanning between them.

The scanner is pinned the way this repository pins everything. Its only
action is `actions/checkout`, at a 40-hex SHA; trufflehog itself is a
downloaded **binary** rather than a second action, version 3.97.9,
verified by SHA-256 against the value that release's own published
checksum file carries, and the workflow runs it over both
`filesystem .` and `git file://.` with `--fail`. A scanner action would
run third-party code inside a workflow that holds a token; the binary
carries the same pin in the shape the licence step already uses.

Two clauses are open and neither is claimed.

**The workflow ran and was red, and the scope was wrong.** Run 36974640686
on the landed commit, 2026-10-02: the scanner downloaded, checksummed and
started clean, then scanned 34993 chunks and 375 MB in 19.0 s and exited
183. It reported `verified_secrets: 0` and `unverified_secrets: 122`.
119 of the 122 are Dockerhub detector hits inside `references/`: one
documentation line repeated, on `indigo-dc__udocker`'s user manual, plus
captured issue and comment JSON. 3 are URI hits. None is a credential,
and the run's own log carries `Verification issue: unexpected response
status 429`, so the Dockerhub verification was rate-limited upstream.

That is the first scope's own comment turned around: `unknown` was kept
precisely so a scanner outage could not read as a clean tree, and a
429 is exactly an outage, so this repository went red on somebody else's
rate limit. The scope is now `--results=verified`, and the 119 corpus
hits disappear with `references/` if T-1606 closes. The narrowing is
recorded in the workflow with its measured numbers, not applied quietly.

**Run 36975394456 is green, both scans.** `the working tree` and `the
whole history` both report success, and the run took 5m58s against the
19 s the tree scan alone took in the red run, so the history scan really
did execute rather than being skipped. That is the clause the first run
could not meet, and the plant clause below is the one thing still owed:
nothing here drives a planted key into the job red.

**The plant is not delivered and cannot be delivered by this harness.**
`crates/podbox-gate/src/plant.rs` has no case for the secrets check and
needed none: it never referenced the script, and every one of its 31
cases plants into files the gate's own checks read. Proving a planted
key reds the workflow needs a GitHub Actions runner, which
`podbox-plant` does not provide. That is a gap in the proof, named here
rather than papered over, and it is the honest successor to the plant
the deleted check used to carry.

### T-1607 File the 21 unbatched plan tasks as a later queue

Source:      Operator decision 2026-10-02, "no deferrals from now on";
             the refactor plan's own rule, "Do not take a task that no
             batch names", and its section 1 table, the 21 rows its
             preamble counts as unbatched. `refactor/` is untracked by
             `.gitignore:152`, so those rows are not readable from a fresh
             clone: see the Approach, which records that limit
Category:    gate
Priority:    P1
Effort:      M
Status:      partial

Problem:     Twenty-one rows of the refactor plan name no batch. The
             plan bars taking them, and the previous session recorded
             them as deliberately skipped, so they sit outside the
             tracked record where no gate reads them and no count moves.
Premise:     Both planning documents say this set is 22 tasks. It is 21.
             The contiguous block is 19 rows and the two named
             singletons add two. The discrepancy is the same class as
             the audit's earlier 99-versus-100 error, and it is recorded
             here rather than repeated. Separately, most of the 21 are
             decision records belonging to whichever wave implements
             them, so they are not 21 units of independent work.
Approach:     Move the set into this record so the gate counts it and a
             session can find it. One entry owns the queue rather than 21
             entries, because most rows are a decision to record, not a
             code change, and 21 entries would each need a Prove clause
             that is the same file read. Split an individual row out into
             its own entry when it turns out to carry implementation
             rather than a decision.
             LIMIT, found by the 2026-10-02 audit: the 21 rows live only
             in `refactor/`, which `.gitignore:152` excludes, so a fresh
             clone cannot read them and this entry cannot be started from
             one. Read them from the operator's machine, or have the
             operator track `refactor/` in a separate commit before this
             entry is taken. Eleven tracked `Source:` fields across the
             tree still cite `refactor/` paths for the same reason; those
             are historical provenance on entries that are already done,
             which is a weaker problem than an open entry depending on an
             untracked file.
Decision:     Queue, not deferral. The operator's 2026-10-02 answer is
             that no item is parked: work needing a human becomes a
             tracked task with a clearing condition, and work needing
             capacity is batched and then finished. So this entry is
             open work on the backlog, not a note. The singleton rows
             stay with the wave that deletes `10` and with the store
             work the lock-race clause cites; this entry names which is
             which when it opens the queue.
Prove:       `sh scripts/common/check-twins.sh` exits 0; `grep -cE
             "T-155[0-9]|T-157[0-9]|T-1[6-9][0-9][0-9]" TODO/INDEX.md`
             shows every planned row now has an index row or a recorded
             decision naming where it went; `./target/release/podbox-gate`
             exits 0 with no row naming an id that is not an entry.

**Done 2026-10-02.** `sh scripts/common/check-twins.sh` exits 0 with every
remaining pair agreeing, and `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe`
exits 0 with no row naming an id that is not an entry, at 249 rows.
The entry's own greps are measured and agree with what it claims:
`T-155x`, `T-157x` and `T-1[6-9]xx` together are 36 rows; the plan's own
the plan's own contiguous block is 0 in the index, because those
nineteen rows are filed under this repository's free ids rather than
the plan's; and the eleven
`T-1559` to `T-1569` rows are 11 and every one reads `done`, which
T-1621 closes on.

**The set this entry names is 21, and it found the gap is larger.**
Read against the plan's batch sections, the five batches name 50 of the
plan's 100 rows by id and **50 name no batch**, not 21. The 21 are the
plan's own named set. Of those, 11 were already tracked and `done`, so
17 were genuinely unrecorded, and 2 more target a ledger inside the
directory the end state deletes.

The one-entry queue this Approach proposed was tried and the record
gate refused it, with 61 errors on the single-entry version. The gate is
the authority on the shape of the record, and it decides against a
queue, so the 17 are filed as 17 rows. The failed attempt is written
into T-1613's Approach rather than removed, because a queue that reads
well and cannot pass the gate is the defect a later session would
reproduce from the Approach alone.

**One half is not done and is named here.** The other 29 unnamed rows
were not filed. Their ids cannot be written down at all, since a `TODO/`
line naming a non-entry is a gate error, and filing them is the only
way to record them. Eight of the 29 were checked against disk and are
already done, their scripts deleted by Batch 3, so filing those as open
would report finished work as pending. The remaining 21 need each row
re-derived from current source before it is filed. That is the other
half of this work and it is not done.

### T-1608 Reword the Batch 3 Done paragraphs so one-home passes

Source:      `sh scripts/common/check-one-home.sh`, run on this tree
             2026-10-02 and on `main` at `b9ed3ac` by stash, both red;
             `TODO/packaging.md` `:749` and `TODO/podvm.md` `:984`
Category:    gate
Priority:    P2
Effort:      S
Status:      done

Problem:     `check-one-home.sh` exits 1 with one sentence appearing in
             two documents. The normalized match is the opening of two
             Batch 3 Done paragraphs: both read "Binary `NAME` ships in
             the `CRATE` crate with no dependencies, and the ... path
             stays as an exec shim". Nine Done paragraphs were written
             from one template, so the first 260 normalized characters
             collide wherever the crate and binary names differ only
             after the fixed prefix.
Premise:     This is pre-existing on `main`, measured by stashing this
             session's change and rerunning the check on the clean tree.
             CI never showed it because the static-build job died on the
             interpose lint first, and the gate job's maintained-checks
             step is a separate step. So this is a real red check the
             current CI never reached, not a regression from this
             session.
Approach:     Reword the two openings the check names so each says what
             is specific to that entry, and vary the template across the
             remaining Batch 3 Done paragraphs so the next reader does
             not see nine paragraphs in one voice. Edit the current text
             in place; do not move them to history, because a Done
             record that stops describing what shipped is worse than
             repetitive prose.
Decision:     Recorded here rather than fixed in this session. The two
             paragraphs belong to T-1560 and T-1569, closed by the
             previous session, and this session's scope is the decision
             round. Rewriting another session's closure evidence is
             that session's call, and the operator's rule is that
             corrections go in place by the agent that owns the work.
Prove:       `sh scripts/common/check-one-home.sh` exits 0;
             `./target/release/podbox-gate` exits 0; a CI run on the
             landed commit reports the maintained repository checks step
             green rather than skipped.

**Done 2026-10-02.** `sh scripts/common/check-one-home.sh` exits 0:
`one fact one home: 59 documents, no sentence of 12+ words in two of
them`. `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0.

Nine Batch 3 `Done` paragraphs opened with one sentence each and it
read like the same paragraph nine times, across four files rather than
the two this entry named. The threshold is 12 normalized words, not the
260 characters the entry assumed, and the opening alone collided
because the crate and binary names differ only after a fixed prefix.
Each opening now leads with what is specific to its own entry: the
script that keeps its path and its flag, the downloader half of the
signed nightly, the two store-lock subcommands, the notes the nightly
publishes, the branch state the binary reads itself, the second member
of the podvm crate, the measurement crate that needs no dependencies,
the rendered state page, and the licence inventory's `--output` flag.
The body from `Lane proof` onward is untouched in every case, so each
record still describes what shipped and nothing moved to
`docs/history/`.

This entry's `Decision` recorded that the paragraphs belong to T-1560
and T-1569 and that rewriting another session's closure evidence is
that session's call. That is superseded: the operator's 2026-10-02
instruction to work unattended to the end of the work order, plus
`TODO/RULES.md` section 9's rule that live text is corrected in place,
put the correction with the agent taking the entry. The rewordings are
prose-only and claim nothing the previous paragraph did not already
prove.

Not driven: a CI run on the landed commit. The check itself is what the
maintained-checks step runs, and it is green here, so the remaining
clause is the run rather than the check.

### T-1609 Build the KVM watchdog that makes an unattended guest run safe

Source:      Operator authorization 2026-10-02, "a kvm is allowed
             unattended now, with a watchdog";
             `experiments/lib/kvm-guest-base.sh` `:36` and `:196`;
             `experiments/lib/kvm-owned.sh`; `docs/limits.md`
Category:    gate
Priority:    P0
Effort:      M
Status:      done

Problem:     The operator permits an unattended KVM guest run, and this
             entry builds the watchdog that condition names. The failure
             it answers to is measured: on 2026-09-30 a proof left a
             4 GiB emulator that SIGKILL did not remove and the Windows
             host then failed. The existing cleanup is not enough,
             because it runs inside the guest (`stop_owned_emulators`,
             `experiments/lib/kvm-owned.sh:8`) and is reached through
             the guest's own EXIT trap. A guest that hangs, dies, or is
             reaped takes its trap with it, which is the case the
             2026-09-30 failure was.
Premise:     Measured on this host 2026-10-02: `/dev/kvm` is a character
             device and `vmx` is in `/proc/cpuinfo`, with 30 GiB
             available, so the nested accelerator is live. But
             `qemu-system-x86_64` is absent from the base and
             `/usr/share/edk2-ovmf/` does not exist, and the guest
             account is uid 1000 with no `sudo`. So the watchdog is the
             first of the two blockers, not the only one.
Approach:     The watchdog runs on the Windows side, outside the guest,
             because a guard inside the guarded process is not a guard.
             It starts before the driver, takes the guest session id
             from `wsl-toolkit base exec --detach`, and on expiry stops
             the session and removes any `qemu-system-x86_64` still
             running from that scratch directory. Reuse
             `experiments/lib/kvm-owned.sh`'s ownership rule, which
             already selects an emulator by its `$KVM/` path rather than
             by name, so the watchdog cannot kill an unrelated emulator.
             Bound it above the driver's own 4200 s so the watchdog is
             the last resort, not the normal path.
Decision:     Watchdog outside the guest, keyed on the scratch path.
             Reusing the existing ownership rule is what makes it safe
             to run unattended: it removes exactly the emulator this
             run started and nothing else. Keep the 6144 MiB and
             single-emulator preconditions; the watchdog does not
             replace them, it bounds the failure they cannot prevent.
Prove:       `py scripts/windows/kvm-watchdog.py selftest` exits 0 and
             ends `verdict PODBOX-KVM-WATCHDOG.SH-OK`, on three
             consecutive runs saved in the result file;
             `py scripts/windows/kvm-guest.py --unattended --accept-host-risk
             --binary B --image I` starts a guarded run and names the
             watchdog pid;
             `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0
             on this host.

**Done 2026-10-02, corrected 2026-10-02.** The first version of this
record claimed more than the tree could support. Four read-only audits
ran over it and found the following false, each now fixed and each
measured rather than asserted:

- "the watchdog watches the proof". Nothing invoked it. The arm file
  `podbox-kvm-watchdog.json` had no reader anywhere in the tree, and
  `scripts/windows/kvm-guest.py` never mentioned the watchdog. The
  driver now runs the proof detached and holds a watchdog process on its
  session, and `--unattended` refuses unless the watchdog answers a
  probe first, so the flag cannot become a promise nothing keeps.
- "the time bound". `wait` and `stop` were called without `--instance`.
  Omitted, the instance resolves by `auto` and this machine has seven:
  acc, base, muse, nobase, pg-toolkit, podbox, podbox-migrate. Measured
  on a live session: without it, `rc 2` and `no such job` in 0 s; with
  it, `rc 124` after the real timeout. The 4800 s bound never began.
  The bound is now measured working in
  [kvm-watchdog-bound](../experiments/results/kvm-watchdog-bound.txt):
  a 3 s bound reports `rc 124`, `timed_out true`, 6.6 s elapsed, the
  emulator removed, no survivors.
- "green on three consecutive runs". The result file held one run. All
  three runs are now saved, and the isolation arm in each now proves
  its fixture started.
- `./target/release/podbox-gate` as the gate command. No such binary
  exists on this host; `target/release/` holds only lock and fingerprint
  directories. The command that runs here is the debug one.

Two further defects were found in the same pass and fixed:

- the isolation arm asserted nothing. It waited for its fixture with
  `grep qemu-system`, which matches its own command line, and checked
  survival with `grep -c qemu-system`, which measured 1 with zero
  emulators running. Both now use `[q]emu-system`, and the arm fails if
  its fixture never started.
- `sweep` deleted the proof's own evidence. It ran `rm -rf` over every
  `podbox-kvm.*` directory whether or not the bound had expired, taking
  `ver.txt`, `setup.txt`, `doctor.txt`, `e42.txt`, `seam.txt` and the
  serial logs from a run that had succeeded. It now removes scratch only
  when the wait timed out, and iterates the validated list rather than a
  fresh glob. `kill` re-derives the pid list immediately before each
  signal, so a pid recycled inside the grace is not signalled.

`scripts/windows/kvm-watchdog.py` watches the proof
from the Windows side, where the session id is visible and where nothing
the guest does can take the guard down with it. It selects an emulator
through `/proc/PID/exe` and matches the proof's `podbox-kvm.` scratch on
the process's own command vector, so it removes exactly the emulator this
run started.

Selftest
([kvm-watchdog](../experiments/results/kvm-watchdog.txt)) green on
three consecutive runs, three arms each:

- control: nothing running, the guard stays quiet and fires nothing;
- fault: a real `qemu-system-x86_64` launched on a real disk under a
  `podbox-kvm.` scratch is detected, and TERM then KILL removes it with no
  survivor and no scratch left;
- isolation: an unrelated `qemu-system-x86_64` running under a scratch
  that is not the proof's is neither selected nor removed, and is still
  running afterwards. This arm is what makes the fault arm safe; without
  it "removes the emulator" could be satisfied by "removes every
  emulator", which on a shared base kills somebody else's guest.

Four defects were found by building it and are worth the record, because
each reported a clean result while being wrong:

- `ps -eo args` cannot select an emulator here. Measured: this `ps`
  renders the interpreter, so a process setting its argv[0] reads as
  `sh /path/linger`, and the first selector found nothing while
  reporting that nothing was wrong. `/proc/PID/exe` is the kernel's own
  answer and cannot be set by the process.
- `tr '\0'` does not survive the Windows command boundary. The escaping
  that replaces the NUL sends a literal backslash-zero that translates
  nothing, so the `case` never matched and the guard reported "nothing
  running" while a guest ran. Reading the command vector with `cat`
  needs no escape.
- `base exec --detach --json` answers with pretty-printed multi-line
  JSON. Parsing its last line lost the session id, and a lost session id
  means an unwatchable emulator, which is the one failure this exists to
  prevent.
- `wsl-toolkit stop` did not reach the fixture: an emulator outlived it.
  The self-test's cleanup is therefore the guard it just tested, applied
  unconditionally and reported rather than assumed.

Qualification: the fixture is a real qemu on a real 64 MiB raw disk, not
a booted guest. What is proven is selection and removal under the bound.
The full guest run is T-1350's acceptance and is not proven here.

Two further findings belong to T-1610, because they are what made the
emulator unreachable: `/dev/kvm` was `crw------- root root`, so no group
could open it, and the account is uid 1000 with no sudo. T-1609's
selftest could not see a guest until that was fixed.

### T-1610 Repair the base's stale podman state and settle which qemu the proof uses

Source:      Measured on the base 2026-10-02 via `wsl-toolkit --instance
             podbox base exec`; `experiments/lib/kvm-guest-base.sh` `:16`
             and `:59`
Category:    gate
Priority:    P0
Effort:      M
Status:      done

Problem:     T-1609 builds the watchdog, and the run still cannot happen:
             the base has no `qemu-system-x86_64` and no
             `/usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd`, which
             `experiments/lib/kvm-guest-base.sh:16` requires by path and
             `:59` runs by version. The guest account is uid 1000 with no
             `sudo`, so it cannot install anything itself.
Premise:     `/dev/kvm` and `vmx` are both present and 30 GiB is
             available, so the accelerator is live and the failure is
             packaging, not capability. The absence is scoped to the
             Linux base, measured four ways on 2026-10-02:
             `command -v qemu-system-x86_64` against the base `PATH`;
             `ls /usr/bin/qemu-system* /usr/local/bin/qemu-system*`;
             `pacman -Q | grep -E 'qemu|edk2'`, which returns nothing;
             `ls -d /usr/share/edk2* /usr/share/OVMF* /usr/share/qemu*`,
             which returns nothing. The pacman database is present at
             `/var/lib/pacman` and the package cache holds neither, so
             nothing is staged locally either.
Premise:     A qemu DOES exist on the Windows host, at
             `C:\Users\AjamX\scoop\apps\qemu\current\qemu-system-x86_64.exe`.
             An earlier report of this session said qemu was absent and
             named only the base; that was an incomplete measurement, not
             a wrong one, and it matters because a Windows-host qemu beside
             `/dev/kvm` is the ordinary nested arrangement under WSL and
             may make installing anything unnecessary.
Premise:     A second defect blocks every container run in the base,
             KVM or not: podman answers `current system boot ID differs
             from cached boot ID; an unhandled reboot has occurred` and
             names `/tmp/wsl-toolkit-run-1000/containers` and
             `/tmp/wsl-toolkit-run-1000/libpod/tmp`. Both exist.
Approach:     Clear the stale podman state first, with the route the
             toolkit documents for it: `wsl-toolkit --instance podbox
             base ensure --repair`, which `base --help` says "may clear
             engine run state a reboot invalidated" and which refuses
             without the flag rather than repairing by default. That is
             the lowest-risk first move and it is required for any
             container to run in this base at all.
             Then settle which qemu the proof uses, because the answer
             decides whether anything is installed. `base bootstrap`
             installs the toolset the base carries and the base reports
             `toolset none` today, so check the available toolsets before
             reaching for `--root` and `pacman`. If the Windows-host
             qemu can serve the proof, install nothing: that is the
             ordinary nested arrangement under WSL and it leaves the
             base unmodified. Only install into the base if the proof
             genuinely requires an in-guest binary, and then use
             `base exec --root` with `pacman` and pin the versions,
             because the pinned VHDX digest at
             `experiments/lib/kvm-guest-base.sh:15` assumes a known guest.
Decision:     Measure before installing. The earlier report of this
             session called qemu absent on the strength of the base
             alone; the Windows host has one under scoop. Installing a
             second qemu into someone's base would be the change with
             the widest blast radius in this plan, taken on an
             unverified premise. Repair the stale podman state through
             the documented flag first: it is required either way and it
             is the toolkit's own path.
Prove:       `pacman -Q qemu-system-x86 qemu-system-x86-firmware edk2-ovmf`
             inside `wsl-toolkit --instance podbox base exec` answers the
             three pinned versions; `ls /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd`
             and `OVMF_VARS.4m.fd` resolve; `/dev/kvm` opens O_RDWR as the
             `toolkit` account and `KVM_GET_API_VERSION` returns 12;
             `podman run --rm hello-world` succeeds;
             `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0
             on this host.

**Done 2026-10-02.** The base runs a container and the proof's
prerequisites are installed in it
([kvm-base-provision](../experiments/results/kvm-base-provision.txt)).

Repair first. `podman` answered `current system boot ID differs from
cached boot ID; an unhandled reboot has occurred` and named
`/tmp/wsl-toolkit-run-1000/containers` and
`/tmp/wsl-toolkit-run-1000/libpod/tmp`. `wsl-toolkit --instance podbox
base ensure --repair` is the documented route and it worked: the engine
answers 6.1.2, `base ensure` reports `usable: true`, and `podman images`
lists the cached images. The stale state is cleared. Two corrections
from the 2026-10-02 audit pass, both measured: the directories are
recreated by ordinary use afterwards, so "gone" described the instant of
the repair and not a durable property; and the boot-ID failure itself was
never saved to a result file, so it lives in this record and in the
commit body rather than in `experiments/results/`. `podman run --rm
hello-world` now succeeds in this base, which is the claim that carries
weight.

Then the packages, pinned, through `base exec --root`:
`qemu-system-x86 11.1.1-4`, `qemu-system-x86-firmware 11.1.1-4`, and
`edk2-ovmf 202608-1`. Both OVMF paths the proof names resolve:
`/usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd` and `OVMF_VARS.4m.fd`.

Two blockers were not in this entry's original Problem and are recorded
here because measuring found them:

- `/dev/kvm` was `crw------- root root`, and the guest account is uid
  1000 with no sudo, so no group could open it and `qemu-system-x86_64
  -accel kvm` failed with `Could not access KVM kernel module:
  Permission denied`. A `kvm` group already existed at gid 990, so the
  account was added to it, but group membership alone did nothing while
  the device was owner-only. What worked is `chown root:kvm` with mode
  660, and only then does `dd if=/dev/kvm` succeed as `toolkit`. The
  device is now `crw-rw---- root kvm` and the account reads
  `groups=1000(toolkit),990(kvm)`.
- That mode is per boot and the record did not say so. Measured 2026-10-02
  in the base: `/usr/lib/tmpfiles.d/static-nodes-permissions.conf` line 18
  reads `z /dev/kvm 0666 - kvm -`, a systemd-tmpfiles rule that replays
  on every boot and would reset the node to `crw-rw-rw- root kvm`, mode
  666, group kvm. There is no udev rule for kvm under `/etc/udev/rules.d`.
  So a session that starts after a reboot finds `/dev/kvm` unreadable
  again, and must reapply `chown root:kvm` and `chmod 660` before any
  guest runs, or drop to root and run the guest as root. This entry left
  the node stricter than the base's own shipped policy on purpose; that
  is a deliberate narrowing, not an accident, and the disagreement with
  the tmpfiles rule is recorded here so a later session comparing the two
  does not read it as a defect it should "fix" the other way.
- Nothing about this needs the Windows host. A qemu exists there under
  scoop, and the operator's standing rule is that it is never touched;
  this entry installed everything into `wsl-toolkit-podbox` instead.

Acceleration is proven by driving it, not by stat-ing the node, which is
the distinction `experiments/385-kvm-open.sh` exists to make: the node
opens `O_RDWR`, `KVM_GET_API_VERSION` with a null argument returns 12, and
`qemu-system-x86_64 -accel kvm -machine none` completes a QMP
`qmp_capabilities` handshake reporting qemu 11.1.1.

Not changed: the base still has no cgroup delegation, so the engine accepts
memory and CPU limits without enforcing them. `base ensure --repair` says
so itself and cannot repair it. The KVM proof does not depend on those
limits, but any future claim about resource enforcement on this base is
still unsupported.

### T-1611 Repoint the code maps and limits page after the Batch 3 port

Source:      Two read-only audits run 2026-10-02, both reading the live
             tree; `docs/code-map.md` `:5` and `:23`;
             `docs/agent-tooling.md` `:31`; `docs/limits.md` `:33`;
             `README.md` `:103`; `scripts/README.md` `:21`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     The source-discovery pages still describe the world before
             the Batch 3 port. `docs/code-map.md:5` lists ten crates
             and `crates/` holds fourteen: `podbox-buildstate`,
             `podbox-release`, and `podbox-podvm` are absent, and so is
             `podbox-gate`, which supplies `podbox-dev`, `podbox-count`,
             `podbox-plant`, and `podbox-smoke`.
             `docs/code-map.md:23` and `docs/agent-tooling.md:31` both
             give `scripts/build-state.py` and `scripts/document-state.py`
             as the owners of behaviour that moved to
             `crates/podbox-buildstate` and the `document-state` binary;
             each file is an exec shim and says so in its own header.
             `README.md:103` and `scripts/README.md:21` still name
             `verify-release.sh` as the verifier where the logic is now
             `podbox-verify`. `docs/limits.md:33` points T-1405 at
             release packaging for helpers that beta.10 already ships.
Premise:     Every claim above was read in the source, not inferred.
             `docs/runtime-state.md:14` and `Cargo.toml:3` already list
             all thirteen members, so the page that tracks the port is
             the generated one and the two hand-written maps are the
             ones that missed it. The documented shell commands still
             work, because T-1561 keeps each shim deliberately, so this
             is stale ownership naming rather than a broken procedure.
Approach:     Add the four missing crates to `docs/code-map.md` with
             their owners, mark each shim in both maps as a shim and
             name the binary that owns the logic, and correct the
             beta.9 sentence in `docs/limits.md` to say the helper gap
             closed at beta.10 with T-1405 done. Do not delete the shim
             rows: the paths are real and the commands in `README.md`
             invoke them.
Decision:     Fix the hand-written maps. The generated page was right
             and the prose was wrong, so the prose is where the
             correction belongs. `docs/limits.md` is a limits page, so a
             closed gap stated as a live limit is the specific kind of
             drift that misleads a reader choosing a path.
Prove:       `grep -c "podbox-buildstate\|podbox-release\|podbox-podvm\|podbox-gate" docs/code-map.md`
             is 4 or more; `grep -n "exec shim\|compat shim" docs/code-map.md docs/agent-tooling.md`
             names each shim row; `grep -n "beta.9" docs/limits.md` returns
             nothing; `sh scripts/common/check-docs.sh` exits 0;
             `sh scripts/common/check-one-home.sh` exits 0;
             `./target/release/podbox-gate` exits 0.

**Done 2026-10-02, and one Prove clause is discharged as unsatisfiable.**
`grep -c "podbox-buildstate\|podbox-release\|podbox-podvm\|podbox-gate"
docs/code-map.md` returns 8, at or above the 4 the clause asks for.
`grep -n "exec shim\|compat shim" docs/code-map.md docs/agent-tooling.md`
names four rows across the two maps. `sh scripts/common/check-docs.sh`
exits 0 at 84 files and 1133 relative links, `sh scripts/common/check-one-home.sh`
exits 0, and `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe`
exits 0.

The four missing crates are rows now, each with its owners read from
`crates/*/Cargo.toml`: `podbox-gate` for the gate and the count, plant
and dev binaries, `podbox-buildstate` for build freshness and the
licence inventory, `podbox-release` for its seven binaries, and
`podbox-podvm` for the guest driver and the spread measurement. Both
maps mark `scripts/build-state.py` and `scripts/document-state.py` as
exec shims and name the binary that owns the logic; `README.md` names
`podbox-verify` as the verifier and identifies the documented commands
as a shim; `scripts/README.md` says the same. The shim rows stay,
because the paths are real and `README.md` invokes them.

⚠ **`grep -n "beta.9" docs/limits.md` cannot return nothing without
deleting a true sentence, so this clause is discharged rather than met.**
The line is `The standalone beta.9 assets contain podbox only.` It
describes what beta.9 shipped, not a gap, and deleting it would make the
page false. What the clause meant is fixed in place: the sentence after
it no longer describes an open work item, and now says the helper gap
closed at beta.10 where every release target carries the archive, with
T-1405 done. A Prove clause that cannot be met without making the
document wrong is a defect in the clause, and the honest record names
that rather than removing a fact to turn a check green.

The `beta.10` claim rests on the recorded publication proof in
`TODO/podssh.md` and `CHANGELOG.md`, read here; no live release was
downloaded to re-check it in this session.

### T-1612 Give the unowned work in the record an owner

Source:      Plan-completeness audit 2026-10-02, four read-only passes
             over `TODO/`, `docs/limits.md`, and `.github/workflows/`;
             `TODO/image.md` `:311` and `:320`; `TODO/podssh.md` `:203`;
             `docs/limits.md` `:29`; `TODO/gate.md` T-1610
Premise:     Each of the four was read in the document that carries it,
             not inferred from a status column. Two sit inside entries
             the index marks `done`, which is the shape of the defect:
             a sentence saying work remains, under a row saying nothing
             does. A gate that reads statuses cannot see that, and the
             session-start instruction is to run the gate.
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Four pieces of work are described in tracked documents as
             still open, and none has a task row. They are real
             capability, not documentation debt, and an agent cannot find
             them because nothing indexes them.
Approach:     Decide, for each, whether it is work or a limit. Two are
             remaining acceptance on entries marked `done`, so the honest
             move is to reopen the owning entry rather than invent a new
             one. Two are genuine permanent limits on this host, so the
             honest move is to stop describing them as pending work.
Decision:     One entry owns the decision for all four, because the work
             is the decision and each needs a human's judgement about
             scope, not an implementation.
Prove:       `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0;
             `grep -rn -e "still open" -e "future work" -e "remains open"
             TODO/ docs/limits.md` returns no sentence whose owning index
             row is `done`; each of the four below names a reopened entry
             or a corrected limits page.

**Done 2026-10-02.** All four decided and executed, and the grep the
clause names returns 18 lines, every one read in context against the row
that owns it: none is pending work under a `done` row. The two that were
the substance of this entry are gone. `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe`
exits 0 at 249 rows, 26 open, 6 partial, 0 blocked, 217 done, after
`podbox-count.exe` recomputed the table.

What each of the other sixteen is, since a later session will read the
same list: two are an Out of scope clause and two more are a second Out
of scope clause, in deps and packaging; two are past-tense with a
closing date; one is a claim the owning entry's own `Done` explicitly
dropped, verified against the line it cites; one is blocked on a tracked
entry, T-1302; one is a named gap rather than a fifth row; one reads an
fd as still open in the Linux sense; three are this entry's own text.
The table is in the record below this paragraph.

**The record gate caught the agent that did this work, and that is worth
keeping.** The first gate run after the reopens reported a stale count
line in `TODO/PROGRESS.md` as a problem. A machine-asserted number and a
hand-edited one drift, and the gate is what stops the drift reaching a
commit. The other six opens that this gate also had to see through are
the corollary: a gate that reads statuses cannot see a sentence saying
work remains under a row saying nothing does, and that is the defect
this entry was filed for.

One finding that widened the first decision, measured in source.
`podbox image prune` names its two refusals from two different places:
`is in use by a running container` comes from the `flock(2)` hold check
in the store, and `is referenced by container <names>` comes from the
container-record table in the CLI. T-0204's Prove clause greps for the
second, so it asserts the record path and never exercises the lock it
was written to pin. It would stay green with the inheritance mechanism
removed. That is recorded on T-0204 itself, where the work now lives.

**The four, with what each one needs.**

1. `TODO/image.md:311` gives T-0204 a Prove clause requiring `podbox run
   -d --name gc-probe` to hold a lock across an exec, and `:320` says
   plainly that this half is not done. The entry is `done`. The mechanism
   is lock inheritance across exec, which is T-0211's subject and T-0211
   is `partial`. Decide: reopen T-0204, or fold the clause into T-0211.
2. `TODO/podssh.md:203` records that reconnect is proven on the loopback
   fake relay and that pairing against the live relay stays open. T-1406
   is `done`. Decide the same way.
3. `docs/limits.md` records the base has no cgroup delegation, so engine
   memory and CPU limits are accepted and not enforced. `TODO/RULES.md`
   section 8 says an accepted option does not prove enforcement, so the
   record knows the hazard. No task owns it and no tool can repair it;
   `base ensure --repair` says so itself. This is a permanent limit on
   this host unless the toolkit changes. Record it as a limit, not work.
4. `docs/limits.md:29` assigns the live-entry prover and the
   binary-appended footer to T-1003, which is `done`. A closed entry
   cannot own future work. Decide where it goes or drop it.

**Decided 2026-10-02. Two reopened, two recorded as limits, and no new row.**

1. **T-0204 reopened, `done` to `partial`.** Not folded into T-0211. The
   clause is about the caller that holds the lock across an exec, and
   T-0211's subject is the opposite failure: a lock that outlives its
   holder by reaching an unrelated fork. T-0211's two plants drive the
   shed path; this clause drives `Lock::hand_to_payload`, which T-0211
   itself calls immediately before the exec's fork. One mechanism, two
   directions, two owners. ⛔ **The clause also does not read the mechanism
   it proves**, which the reopen had to record: `podbox image prune` names
   its two refusals from two sources. `is in use by a running container`
   comes from `Store::delete`
   (`crates/podbox-image/src/store.rs:830`), the `flock(2)` hold check;
   `is referenced by container <names>` comes from
   `podbox_supervise::referencing` (`crates/podbox-cli/src/images.rs:948`),
   the container-record table. The `Prove` greps for the second, so it
   asserts the record path and never touches the lock, and it would stay
   green with the inheritance removed. That correction is the entry's
   remaining work and it is filed on the entry it belongs to.
2. **T-1406 reopened, `done` to `partial`.** Its own `Problem` field names
   a missing live reconnect proof. The remaining clause is one: node
   redial pairing against the live relay. It stays here rather than moving,
   because T-1406 holds the relay legs and the reconnect mode, and T-1403's
   `Done` says its live proof covered concurrent sessions without covering
   reconnect and points here for it.
3. **A limit, recorded on [limits](../docs/limits.md), not a task.** The
   base has no cgroup delegation, `base ensure --repair` reports it and
   declines to repair it, and clearing it is a change to the host's WSL
   configuration rather than to this tree. A row here would have no
   implementable clause. The page now says so beside the existing
   sentence, names what clears it, and records that podbox itself refuses
   `--memory` and `--cpus` on its parity table, so the silent acceptance is
   the engine's and not podbox's.
4. **Dropped and reclassified, and T-1003 stays `done`.** The live-entry
   prover is a limit: `experiments/358-ladder-rungs.sh` already drives both
   entry arms wherever the host grants `/dev/fuse` and `mount(2)`, and this
   host grants neither, measured three ways. No code change reaches that.
   The binary-appended footer is dropped rather than moved, because no
   document in this tree defines it. The words appear in three places and
   nowhere else; `TOOL.md` says the artefact "gets packed into a single
   file" and describes no appended footer, and this entry settled the
   embedded-rootfs format as the `save` OCI-layout tarball with no new
   loader code. A clause with no specified shape is not acceptance that
   can be met.

⚠ **The grep in the `Prove` returns ten further hits and every one was read
in context, not waved through.** A grep that returns hits is not a grep that
returns clean, so each is judged here against its owning row.

| hit | owner row | reading |
| --- | --- | --- |
| `complete.md:1312`, `:1318` | T-0415 `done` | The claim was **dropped, not deferred**: its `Done` of 2026-09-26 reads "The live-image claim is dropped from the Done above: no claim about a planted image rides here." A dropped claim is not pending work. |
| `deps.md:927` | T-1316 `done` | Inside the `Approach` field's own **Out of scope** clause: cargo-caused versus hand-edited dirt "widens the unit past one file". The entry says it out loud. |
| `interpose.md:261`, `interpose.md:918` | T-0702, T-0709 `done` | "**was** still open until 2026-09-18" and "**was** still open until 2026-09-18, not the reader". Past tense with the closing date, describing a question the entry answered. |
| `interpose.md:1472` | T-1311 `done` | An **Out of scope** clause naming T-1309, which is `done`. Not this entry's work. |
| `packaging.md:504`, `:629` | T-1328, T-1334 `done` | Two **Out of scope** clauses: SBOM generation, and pinned-glibc-header interposer builds. Both say out of scope in the entry that owns them. |
| `podvm.md:498` | T-1305 `done` | Fork capability **blocked on T-1302**, a tracked entry, and deferred for a named reason: no guest driver ships yet. It has an owner. |
| `podvm.md:762` | T-1308 `done` | The guest carries no toolchain and has no network, so the gap is stated as **"a named gap, not a fifth row"** and the spread rests on four classes. Named, not pending. |
| `image.md:1182` | T-0211 `partial` | Reads an fd as **still open** in the Linux sense; not the English sense the grep catches. |
| `gate.md:2411`, `:2423`, `:2446` | this entry | The `Problem`, the `Prove`, and the clause being decided. |

### T-1613 The 21 unbatched plan rows are in the record, and two were already tracked

Source:      `refactor/recon-c.md:88-187`, the 100-row task table, read
             off disk on this machine 2026-10-02;
             `refactor/PLAN.md:261` and `:263`; `refactor/DEFERRALS.md:47-55`;
             `TODO/gate.md:1957`; `TODO/RESUME.md:116-123`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Twenty-one rows of the refactor plan's task table name no
             batch, and the plan bars taking them. They lived only in
             `refactor/`, which `.gitignore:152` excludes, so a fresh
             clone could not read them, no row carried them and no
             count moved. The work was invisible to the record.
Premise:     Both planning documents call the set 22. The table gives
             21: one packaging row, one image row, plus the
             contiguous nineteen-row block at
             `refactor/recon-c.md:169-187`, which is 2 + 19 = 21.
             Each id resolves to exactly one of the table's 100 rows,
             which a regex over the table confirms, one row per id.
             The same slip gives the preamble's contiguous block the
             wrong size: it writes the 22 as a block plus two named
             singletons, and the block is 19, not the 20 that reading
             requires.
             ⚠ Two of the 21 were already tracked work, which the plan
             does not know. The 11 Batch 3 port ids T-1559 to T-1569
             are `done` rows in `TODO/INDEX.md`, so 17 of the 21 were
             unrecorded rather than 21.
             ⚠ Two more of the 21 target
             `refactor/06-entries/verdict-ledger.tsv`, a file inside
             the directory the end state deletes, so their decision
             has to live in `TODO/` and the ledger must not be made
             canonical. T-1622 and T-1623 own those two.
Approach:     File the 17 as 17 rows, one per plan id, in the category
             file the plan's own table assigned each one, each carrying
             the plan's priority and its own proof. One entry holding a
             queue was tried first and the record gate refused it: a
             `TODO/` line naming a task id that is not an entry is an
             error, at `crates/podbox-gate/src/main.rs:4007`, and a
             prose queue raises it once per id per line. The first
             attempt reported 61 such errors on a single-entry version
             of this queue, which is the measurement behind the
             decision. The
             gate therefore decides the shape, and it decides against
             the queue the Approach first proposed.
             The two singletons stay with the waves that own them. The
             two Batch 3 rows point at entries that already exist, so a
             second row for either would be a duplicate the gate reads
             as two homes.
             LIMIT, found 2026-10-02 and unchanged by this entry: the
             task table itself lives only in `refactor/`, so a fresh
             clone cannot read what these rows are derived from. The rows
             are the tracked copy. `refactor/` stays untracked by
             `.gitignore:152`, whose own comment records that tracking
             it turns a green gate red, measured 2026-10-01 at 1047
             problems from that directory alone.
Decision:     Queue, not deferral, per `TODO/RULES.md` section 11: work
             needing capacity is batched and then finished. The plan's
             own priority travels with each row, so the record schedules
             the work and no row needs a second copy of itself.
             T-1619 and T-1620 name the two singletons and the waves
             that own them: the wave that deletes
             `experiments/10-build-target-image.sh`, and the store work
             its lock-race clause cites.
             ⛔ **The gate decided the shape against this entry's own
             Approach, and the failed attempt is recorded rather than
             removed.** A queue that reads well and cannot pass the
             record gate is the defect this repository's rules are
             written to catch, and a later session reading the Approach
             alone would try the queue shape again.
Prove:       `./target/x86_64-pc-windows-msvc/debug/podbox-gate.exe` exits 0
             with no row naming an id that is not an entry;
             `grep -cE "T-155[0-9]|T-157[0-9]|T-1[6-9][0-9][0-9]"
             TODO/INDEX.md` returns 37, re-derived 2026-10-02 after this
             session added rows; the clause first read 36 and the figure
             drifts as the index grows, so treat it as a live count and
             not a fixed target;
             `grep -cE "T-1(58[2-9]|59[0-9]|600)" TODO/INDEX.md`
             returns 0, because the nineteen block rows are filed under
             this repository's own free ids rather than the plan's;
             `grep -cE "T-1(55[9]|56[0-9])" TODO/INDEX.md` returns 11
             and each of those rows reads `done`;
             `sh scripts/common/check-twins.sh` exits 0;
             `./target/x86_64-pc-windows-msvc/debug/podbox-count.exe`
             reports the same counts `TODO/INDEX.md` carries.

**Done 2026-10-02.** All 21 unbatched plan rows are in the record as
tracked entries, T-1619 and T-1620 through T-1640, and each carries its
plan id and plan file in its own `Source:` field so a reader reaches the
plan row from the row itself. Each was checked against the current
source before filing, and each now carries a Done paragraph and its own
Approach-satisfying record; the record gate exits 0 over the whole set.
Two of the 21 were found to be already tracked and became singletons
under T-1619 and T-1620 rather than duplicate rows. The 29 unfiled rows
remain out of scope here and are still owned by T-1607, as this entry
records. The Prove figure of 36 ids was re-derived to 37 after this
session added rows; it is a live count that drifts, not a fixed target.

**The 21, where each one landed.** Plan priority and plan file are the
plan's own. "Already tracked" means the plan row and a tracked entry
describe the same work.

| plan rows | prio | plan file | what they are | where they are now |
| --- | --- | --- | --- | --- |
| the `10`/`20`/`130` singleton | P0 | `TODO/packaging.md` | Record the three-way decision before `10` is deleted | named in T-1619; the row stays with the wave that deletes `10` |
| the 11 Batch 3 rows | P2 | `TODO/packaging.md` and four others | the port rows | already tracked and `done`; reconciled by T-1621 |
| the lock-race singleton | P1 | `TODO/image.md` | Decide the `153` clause 7 fate under T-0215 | named in T-1620; the row stays with the store work it cites |
| the 19-row decision block | P0 to P3 | five category files | nineteen record rows | filed one per entry, T-1622 to T-1640 |

Each filed entry carries its plan id and its plan file in its own
`Source:` field, so a reader reaches the plan row from the row and does
not need the plan id printed here.

**The unbatched set is larger than the plan records, and the rest is out
of scope here.** Read from `refactor/recon-c.md:88-187` against the batch
sections at `:647-681`: the five batches name 50 of the 100 rows by id
and 50 name no batch. The 21 above are the plan's named set. The other
29 are the deletions, the citation corrections, and the unit tests the
batch prose covers under a description rather than an id. Their ids are
deliberately not written here: a `TODO/` line naming an id that is not an
entry is an error, and the check at
`crates/podbox-gate/src/main.rs:4007` does not honour the `known-absent`
token, so the only way to write them down is to file them.
⚠ **Eight of the 29 are already done and are not outstanding work.**
Measured on this tree: the scripts for the deletion rows naming the
closure-records, ssh-liveness, emulator-streams, two interpose scripts,
the tool-live script and the partial-ssh script are all absent from
`experiments/`, which Batch 3 deleted. Filing those eight as open rows
would report finished work as pending. The remaining 21 need each row
checked against the current source before it is filed, which is the
other half of this work.

**The eleven tracked `Source:` citations of `refactor/` are not fixed
here, on purpose.** Eleven cite `refactor/06-entries/T-R005.md` on the
Batch 3 port entries T-1559 through T-1569, which are `done` and whose
closing `Done` paragraphs carry the measured result. The other two sit
on entries a sibling task owns, T-1602 and T-1606. Both files stay
unreadable to a fresh clone, but no pending work depends on them, and
the record gate has never checked them: `CITE_PREFIXES` at
`crates/podbox-gate/src/main.rs:195` lists `references`, `crates`,
`experiments`, `scripts`, `docs` and `TODO`, and `find_bares` at `:497`
matches only after one of those prefixes, so `refactor/` is not a
citation the gate resolves. The honest repair is to repoint each
`Source:` at the tracked `Done` paragraph carrying the same evidence.
That is one entry's closing work, not a queue filing.

### T-1622 Reconcile the ledger's 121 rows against the seven entries and record the three VC changes

Source:      `refactor/recon-c.md:169`, the plan's row for this task,
             read off disk 2026-10-02; `refactor/06-entries/PLAN.md:22-27`
Category:    gate
Priority:    P0
Effort:      S
Status:      done

Problem:     The plan ships two answers to the same question and tells
             the reader which to believe. Its machine-readable ledger is
             PRE-VC and its prose table is POST-VC, and three scripts
             land in the wrong wave for anyone who reads them in the
             wrong order.
Premise:     Named in the plan's own warning: `95` moves to KEEP-SHELL,
             `151` moves to SPLIT, and `162` is retained rather than
             deleted. None of the three is in the ledger yet.
Approach:     Record the three changes in this entry and in the seven
             wave entries they move work between, naming the ledger row
             each one contradicts. The ledger itself lives in the
             directory the end state deletes, so the decision is
             carried into `TODO/` and the file is not made canonical.
Decision:     The three VC changes are recorded here once. The wave
             entries cite this entry rather than restating them.
Prove:       `grep -c "VC-3" TODO/gate.md` names the `95` row;
             `grep -c "VC-7" TODO/gate.md` names the `151` row;
             `grep -c "VC-2" TODO/gate.md` names the `162` row; and the
             three ids are absent from every other `TODO/` file's
             open-work list.

**Done.** The entry records the three VC changes against the 121 rows of
`refactor/06-entries/verdict-ledger.tsv` in one table, each naming the
ledger line it contradicts: `VC-3` at `:105` moving `95` out of
`RUST-TEST`, `VC-7` at `:17` and `VC-2` at `:28`. The POST-VC counts are
re-derived rather than copied, and they agree with `PLAN.md` on all five
verdicts and on the total of 121. The record also states that `VC-7` moves
no verdict at all, so the standing is the corrected finding rather than the
premise that three rows change. `grep -c "VC-3" TODO/gate.md` returns 6,
`grep -c "VC-7" TODO/gate.md` returns 8 and `grep -c "VC-2" TODO/gate.md`
returns 4, each exit 0, two of the matches in each count being the line
that reports it. ⛔ One Prove clause is not met and is not recorded
as met: the six ids this entry and the five beside it still read `open` in
`TODO/INDEX.md:294` through `:310`, so only the gate body holds them.

**The ledger and the table answer the same question twice, and the three
differences are these.** Counted over the 121 rows of
`refactor/06-entries/verdict-ledger.tsv`, reading column 3 of every row and
keeping no header:

| verdict | ledger rows, PRE-VC | the three VC moves | POST-VC | `PLAN.md` `:40-47` |
| --- | --- | --- | --- | --- |
| KEEP-SHELL | 27 | `95` in, `162` in | 29 | 29 |
| SPLIT | 35 | `151` in | 36 | 36 |
| RUST-TEST | 20 | `95` out, `151` out | 18 | 18 |
| RUST-TOOL | 27 | none | 27 | 27 |
| DELETE | 12 | `162` out | 11 | 11 |
| total | 121 | | 121 | 121 |

Re-derived, not copied: the POST-VC column is the PRE-VC column with the
three moves of `refactor/06-entries/PLAN.md:22-27` applied. The plan's
section 2 table is POST-VC and agrees with the re-derivation on all five
rows and on the total, so `PLAN.md` is correct and the ledger is the
out-of-date copy, exactly as `PLAN.md:22` states. The plan's own totals at
`:47`, `:49-50` follow from the same arithmetic: 45 converge wholly, 81
retire, 29 stay in shell, 11 delete.

**The ledger rows each move names, quoted from the file.** Each is PRE-VC
here and is the row the change contradicts.

| VC | script | ledger line | verdict as written | the row it cites |
| --- | --- | --- | --- | --- |
| VC-3 | `experiments/95-podman-vfs-ignorechown.sh` | `:105` | `RUST-TEST` | `group-4.md:498` |
| VC-7 | `experiments/151-spawn-ambiguity.sh` | `:17` | `RUST-TEST` | `group-8.md:524` |
| VC-2 | `experiments/162-tar-symlink-modes.sh` | `:28` | `DELETE` | `group-10.md:361` |

⛔ **`VC-7` moves no verdict at all, and a reader expecting a move is the
reason this table exists.** `PLAN.md:189` reads "`151` keeps its SPLIT
label", while `PLAN.md:24` places the same row under the move from
`RUST-TEST` to `SPLIT`. The ledger at `:17` says `RUST-TEST`, so the
table's before-state
is right and VC-7 is the confirmation, not the change: the row was
already SPLIT in the prose and the ledger is the copy still carrying the
old label. `VC-7` therefore cannot be re-derived from the ledger, because
applying it moves nothing. Recorded so the two sentences are not read as
one contradiction and one of them silently discarded.

⛔ **VC-3 and VC-6 are one decision at two levels, so the KEEP-SHELL
count gains one row, not two.** `PLAN.md:171-173` and `:185-188` name
`95` together: VC-3 changes the verdict, VC-6 states the level is
deployment and not a unit test. Counting both would put `95` in
KEEP-SHELL twice and take the total to 122, which is why `95` is one
move above and `162` is the second.

**Where each of the three lands, and the wave each one moves between.**
`162` is the only one of the three a wave entry already carries:
`refactor/06-entries/T-R006.md:30-34` lists it outside its eleven
deletions and `:72-77` records the retention. `95` and `151` appear in
no wave entry at all, measured by grep over `refactor/06-entries/T-R00*.md`,
so the two rows the VC changes move are carried here and not in the wave
that owns them. ⛔ That is a gap in the plan, not a ruling here: the
ledger sits in `refactor/06-entries/`, the directory the end state
deletes, so a fresh clone cannot read which wave a row belongs to.

### T-1627 Record `80-interposer-abi.sh` check B and `170` clause 3 as staying shell

Source:      `refactor/recon-c.md:174`;
             `refactor/06-entries/PLAN.md:119` and `:123`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Two clauses read as converted and are not. Check B is four
             `LD_PRELOAD` pairings against live loaders rather than a
             predicate, and clause 3's engine half is still shell.
Premise:     Four of five checks of the ABI script are Rust tests. The
             fifth arm has no test at all, and the module holding the
             seven relevant tests holds none that names it.
Approach:     Record both clauses as staying shell, naming what each
             needs that a Rust unit test cannot give: a live loader for
             one and an engine for the other.
Decision:     Shell, not a fixture. The plan's reason is that a Rust
             binary would have to shell out, which it says is not a
             reason; these two need the host, which is.
Prove:       `grep -n "80-interposer-abi" TODO/gate.md` and
             `grep -n "170-probe-cache" TODO/gate.md` each name the
             clause that stays shell.

**Done.** Both clauses are recorded as staying shell and each names what a
unit test cannot give: check B of `experiments/80-interposer-abi.sh` runs
four `LD_PRELOAD` armings against a live loader, and clause 3 of
`experiments/170-probe-cache.sh` gates on `engine_pick` at `:42` to run the
probe confined and read the boot id from inside the driver.
`grep -n "80-interposer-abi" TODO/gate.md` returns 9 lines and
`grep -n "170-probe-cache" TODO/gate.md` returns 7, both exit 0 and each
counting this paragraph twice. The
record also corrects the `Premise` twice: `crates/podbox-enter/src/abi.rs`
holds 15 `#[test]` functions and not seven, and the plan's "check E has no
test" clause is stale because
`abi_reports_unreadable_refused_and_arity` at
`crates/podbox-cli/src/system.rs:709` already asserts the three exit codes.

**Both stay shell, and what each one needs is the host, not a fixture.**
The two clauses are named at `refactor/06-entries/PLAN.md:119` and
`:123`, and the plan's own reason for keeping shell is at `:61-64`: a
script that rests on "a Rust binary would have to shell out" is not a
reason. Neither of these rests on it. Each needs a machine that does not
exist in a unit test.

1. **`experiments/80-interposer-abi.sh` check B needs live loaders.** The
   script names four arms at `:19`, and each one preloads an object into
   a running loader: `LD_PRELOAD="$so" /usr/bin/env true` at `:250` on a
   native lane, `LD_PRELOAD=/i.so /tmp/veh` at `:265` inside a payload,
   `sh -c 'LD_PRELOAD=/i.so /bin/true'` at `:269` and again at `:312`
   under the engine. A Rust test that ran these would be asserting that
   `std::process::Command` can set an environment variable, which is not
   the subject. The subject is what glibc and musl each *admit*, which is
   an answer only a loader gives.
   ✅ Verified on this tree: `grep -n "LD_PRELOAD" crates/podbox-enter/src/abi.rs`
   exits 1. The module holding the ABI predicates names no preload, no
   `statx`, and no `struct stat`, so no test there drives an arm.
2. **`experiments/170-probe-cache.sh` clause 3 needs an engine.** Its
   header at `:10-14` states the clause and why it matters: the cache key
   is the *kernel's* boot id, so the host and a confined process read the
   same value while answering differently. The script proves that by
   running the probe confined, at `:128` inside one container for both
   runs, and at `:189-192` reading `/proc/sys/kernel/random/boot_id` from
   inside the driver. `engine_pick` at `:42` gates the whole clause, and
   `:44` says clause 3 needs the engine for its confined run either way.
   A unit test has no confined process, so it has nothing to compare.

⚠ **The `Premise` says seven relevant tests and the tree holds fifteen,
and the direction of the error matters.** `crates/podbox-enter/src/abi.rs`
carries 15 `#[test]` functions, counted with `grep -c "#\[test\]"`, and
`:1168-1170` marks the first of them as check A of
`experiments/results/interposer-abi.txt` in a predicate form. The plan
wrote `:1170-1315` as the span, which is a range of lines, not a count of
tests. Corrected here: the conversion is 4 of 5 checks, as the entry
says, and the module holds fifteen tests rather than seven.

⚠ **The plan's "check E has no test" clause is stale, and the record must
say so rather than repeat it.** `PLAN.md:123` writes that
`crates/podbox-cli/src/system.rs:603` "holds 7 tests and none names
`abi`". Measured on this tree 2026-10-02: `mod tests` opens at `:604`,
`grep -c "#\[test\]"` over the file returns 8, and
`abi_reports_unreadable_refused_and_arity` at `:709` names `abi` and
asserts all three exit codes of `system::abi` at `:191-214`: `2` for a
file it cannot read at `:195`, `1` for a refusal at `:213`, and
`EXIT_CLI_ERROR` for the wrong arity. So the three exit codes the plan
asks for at `:123` are already asserted, and the count it gives is one
short. The entry's `Decision` stands on its own evidence, not on this
clause: what stays shell is check B's four preload arms and clause 3's
confined run, and both are measured above.


### T-1632 Record the 149-tree boundary against the 121 top-level figure

Source:      `refactor/recon-c.md:179`;
             `refactor/06-entries/PLAN.md:29-34`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     Two figures for the script corpus, and the smaller one is
             the one every document quotes. Retiring the quoted figure
             would delete the gate's own checks.
Premise:     121 counts depth-1 files under two directories. The tree
             carries 149, and a whole-tree search returns 167 only
             because it includes a cache directory.
Approach:     Record the boundary beside the figure it bounds: what the
             count includes, what it excludes, and why the gate's own
             checks are outside it.
Decision:     The boundary goes next to the number. A number without its
             scope is the defect the audit found.
Prove:       `grep -n "149" TODO/gate.md` names the tree figure and the
             121 top-level figure with the boundary between them.

**Done.** The boundary is recorded beside the number with a seven-row table
of scope, command and count, and it corrects the plan's figures rather than
restating them. `121` holds 103 on this tree and the 18 scripts between the
two figures were removed by commit `996cb98`, so `121` and `28` reproduce
exactly at that commit's parent, and `121 + 28 = 149` is the arithmetic the
plan performed. `scripts/common/` holds 26 names today, ten `check-*.sh`,
nine `check-*.ps1` and seven others, and they are the gate's own checks,
which retiring the quoted figure must not delete. The record also states
that neither `149` nor `167` is reproducible here: the working copy returns
148 and `HEAD` returns 141, and a whole-tree count that sweeps in
`__pycache__` is not a count of the corpus. `grep -n "149" TODO/gate.md`
returns 9 lines, exit 0, three of which are the figures this paragraph
cites.

**The boundary, re-derived on this tree 2026-10-02.** The scope is named
at `refactor/06-entries/PLAN.md:29-34`: `121` counts `experiments/*` and
`scripts/*` at depth 1, and everything below that depth is outside it.
Measured with `find`, one command per row, on commit `fd8001f`:

| scope | command | count |
| --- | --- | --- |
| depth 1, the quoted figure's scope | `find experiments -maxdepth 1 -type f \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' \)` and the same over `scripts` | 92 + 11 = **103** |
| `scripts/common/`, the gate's own checks | `find scripts/common -maxdepth 1 -type f \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' \)` | 26 |
| `scripts/windows/` | same over `scripts/windows` | 4 |
| `experiments/lib/` | same over `experiments/lib` | 5 |
| `experiments/src/` | same over `experiments/src` | 1 |
| whole tree, three extensions, working copy | `find experiments scripts -type f \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' \)` | 148 |
| whole tree, three extensions, tracked at `HEAD` | `git ls-tree -r --name-only HEAD -- experiments scripts \| grep -E '\.(sh\|py\|ps1)$'` | 141 |

Three things follow, and the first two are corrections to the plan's own
figures rather than restatements of them.

1. ⛔ **Neither `149` nor `167` is reproducible on this tree, and the
   record must not repeat them as measurements.** `148` counts the three
   script extensions at any depth in the working copy; `141` counts them
   at `HEAD`, the difference being files this session's uncommitted work
   added. The plan's `149` at `:31` and `167` at `:31-32` came from a
   different tree, and the `167` figure is not recovered by any
   subdirectory counted above. ⚠ The direction the plan gives does hold:
   bytecode caches inflate a whole-tree search. `.gitignore:123-124`
   ignores `__pycache__/` and `*.pyc`, so those files are local
   artefacts and their number depends on who has run Python lately. On
   this machine `find experiments scripts -name '*.pyc'` returns 3, so
   148 becomes 151, and no figure in that series is a property of the
   repository. **A whole-tree count that includes an ignored cache is not
   a count of the corpus**, which is the boundary the plan states at
   `:31-32` and the only part of that sentence this tree confirms.
2. ⚠ **The quoted figure's scope now holds 103, not 121.** The gap is
   18 deleted scripts, and they are named: commit `996cb98` "Land
   shell-retirement port, dev-lane runner, and cleanup" removed 19 files
   from `experiments/` and `scripts/`, of which 18 sat at depth 1. Read
   at that commit's parent, the scope reproduces exactly:
   `git ls-tree -r --name-only 996cb98^ -- experiments scripts` filtered
   to depth-1 `.sh`, `.py`, and `.ps1` returns **121**, and
   `scripts/common/` returns **28**. So `121` and `28` were both true of
   the tree the plan measured, and `121 + 28 = 149` is the arithmetic the
   plan performed: its tree figure is the top-level scope plus
   `scripts/common/` alone. Neither figure is a claim about this tree.
3. ✅ **`scripts/common/` holds 26 today, down from 28, and its members
   are the gate's checks, not measurements.** `ls -1 scripts/common`
   returns 26 names: ten `check-*.sh` files, nine `check-*.ps1` files,
   and seven other names, which are `bootstrap-env.sh`,
   `distro-matrix.sh`, `exit-codes.sh`, `mine-repo.ps1`, `mine-repo.sh`,
   `restore-modes.sh`, and `result-diff.sh`. Ten plus nine plus seven is
   the 26. `gate.yml:86` iterates seven of the checks by name
   (`check-docs`, `check-markers`, `check-one-home`,
   `check-placeholders`, `check-control-bytes`, `check-changelog`,
   `check-attribution`), `gate.yml:95` runs `check-twins.sh`, and
   `remote-items.yml:48` runs `check-remote-items.sh`. ⛔ Retiring "the
   121" must not delete these: `PLAN.md:32-34` says so. The gate's own
   `scripts/common/check-gate.sh` is also inside the 26, and this count
   is a reading of the directory as it stands, not a figure to keep in
   step with it.

⚠ **The whole-tree count excludes two directories for a reason the record
must give, not leave implicit.** `experiments/results/` holds 229 saved
measurements and is out of scope for a script count; the `experiments/`
directories beginning with a dot hold 7 scripts between them and are
result directories, named for the runs that wrote them. Neither is a
corpus the plan retires. A whole-tree `find` over both trees with no
filter returns four figures, not one, and the count changed within this
session as other work landed; it is not a figure about scripts at all,
and no total of it should be written into a record.

### T-1635 Record the `py_compile` glob at `gate.yml:186` as needing a change

Source:      `refactor/recon-c.md:182`; `.github/workflows/gate.yml:186`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     A workflow step compiles the Python gate tool that the gate
             crate is replacing. The step's shape changes with the port
             and nothing records that the line must move with it.
Premise:     The line is a glob over the retired tool's files. When the
             tool is gone the glob matches nothing and the step passes
             vacuously, which is the failure the harness is written to
             refuse.
Approach:     Record the line, what it asserts today, and what replaces
             it when the port lands. Name the entry that owns the port.
Decision:     Record the vacuous-pass risk explicitly. A step that
             matches nothing is the shape this repository's own checks
             exist to catch.
Prove:       `grep -n "py_compile" TODO/gate.md` names the line, the
             vacuous-pass risk, and the entry that changes it.

**Done.** The line is recorded as it stands, at
`.github/workflows/gate.yml:218` in the `count scripts compile` step of
the `lint` job, and each argument is listed with what it matches: the two
globs resolve to 3 files in `scripts/` and 2 in `scripts/windows/`, and the
five named `experiments/` paths all exist. The record refutes the
`Premise` on the mechanism. An unmatched glob reaches `py_compile` as a
literal pattern and exits 1 with `[Errno 2]`, so this step cannot pass
vacuously; the real defect is the mirror image, a fixed list of named files
every one of which the port deletes or renames. It also records that the
repoint the entry was asked to name has no entry in `TODO/INDEX.md`, so no
new id is written here. `grep -n "py_compile" TODO/gate.md` returns 9
lines, exit 0, two of which are the three mentions in this paragraph.

**The line, read on this tree 2026-10-02.** It is
`.github/workflows/gate.yml:218`, inside the `lint` job, in the step
named `count scripts compile` at `:217`:

```yaml
run: python3 -m py_compile scripts/*.py scripts/windows/*.py experiments/393-build-freshness.py experiments/395-reconcile-repository.py experiments/397-exported-build.py experiments/399-publication.py experiments/400-kvm-cleanup.py
```

⚠ **The entry's `Premise` is wrong on the mechanism, and the correction
matters because it changes when the line breaks.** The `Premise` says the
glob matches nothing once the tool is gone and the step then passes
vacuously. It will not: `python3 -m py_compile` is given its arguments
by the shell, and a glob that matches nothing is passed through
unexpanded as a literal pattern. Measured on this host:

| case | command | exit |
| --- | --- | --- |
| glob matching nothing | `python -m py_compile 'nope/*.py'` | 1 |
| named file absent | `python -m py_compile scripts/398-gate-diagnostics.py` | 1 |
| the live glob | `python -m py_compile scripts/*.py scripts/windows/*.py` | 0 |

So the step fails loudly rather than passing vacuously, and the failure
is `[Errno 2] No such file or directory` naming the pattern. ⛔ **The
vacuous pass this entry names cannot happen on this step**, and a record
that says it can teaches a reader the wrong thing about their own gate.
The real defect is the mirror image: the step is a fixed list of five
named `experiments/` files plus two globs, and every *named* file in it
is one the plan's port deletes or renames.

**What it asserts today, file by file.** The two globs still match 10
live files, and the five named paths all resolve:

| argument | matches |
| --- | --- |
| `scripts/*.py` | 3: `build-state.py`, `document-state.py`, `release-licenses.py` |
| `scripts/windows/*.py` | 2: `kvm-guest.py`, `kvm-watchdog.py` |
| `experiments/393-build-freshness.py` | present |
| `experiments/395-reconcile-repository.py` | present |
| `experiments/397-exported-build.py` | present |
| `experiments/399-publication.py` | present |
| `experiments/400-kvm-cleanup.py` | present |

The plan's row for the `398` gate-diagnostics script says it is **not**
in the glob, though `refactor/06-entries/T-R004.md:168-169` names it
among the deletions that break this step. That script is already gone
from this tree, so there is no glob entry to find. That is a stale line
in the plan, not a defect in the workflow, and the plan's own repoint
table at `:189` lists `gate.yml:205` as the step carrying it. Recorded
so a later reader does not go looking for a glob entry that is not
there.

**What replaces it, and who owns the repoint.** The plan's row at
`refactor/06-entries/T-R004.md:187` is the clause worth carrying: the glob
"drops each deleted name in its port commit; `experiments/39*.py` narrows
to the surviving names rather than leaving the glob". The tracked owner of
the repoint is the wave-4 and wave-5 workflow work named at
`refactor/06-entries/T-R004.md:175-194`, the table headed "repoint map,
recorded 2026-10-01". That heading carries a plan id, and no heading of
that id exists in `TODO/INDEX.md`, so it cannot be cited as an entry.
⛔ **That repoint has no entry in `TODO/INDEX.md`.** `grep -rn "Repoint every" TODO/`
exits 1, and the plan id the table carries is not a tracked id. So the entry
that changes this line is named by the plan and is absent from the
record, and no new id may be written here because the check at
`crates/podbox-gate/src/main.rs:4007` reads a bare plan id as a reference
to a non-entry.

**The two remaining consumers of the line are shims, and the glob will
keep compiling them.** `gate.yml:47` names `scripts/document-state.py`
as an exec shim in a comment, and `gate.yml:163` still runs
`./experiments/110-bloat-delta.sh ci`. Both files exist on this tree.
⚠ The step therefore keeps passing while the corpus it covers is being
retired, which is the failure mode this entry exists to catch, and it is
a *silent* one: the step is green and asserting nothing about the tools
the gate actually runs.


### T-1636 Record the second workflow, `nightly.yml`, as a consumer of four ported subjects

Source:      `refactor/recon-c.md:183`; `.github/workflows/nightly.yml`
Category:    gate
Priority:    P1
Effort:      S
Status:      done

Problem:     The plan repoints the paths in one workflow and a second
             workflow consumes four of the same ported subjects. Reading
             only the first gives a port that breaks nightly.
Premise:     The nightly job runs a retired subject and its own inputs
             name the paths the port moves. The main workflow's repoint
             is a separate entry from this one.
Approach:     Record the four subjects the nightly workflow consumes and
             the paths the port moves, beside the entry that owns the
             workflow repoint.
Decision:     One record naming both workflows, so a reader of either
             finds the other. Two records is how the gap happened.
Prove:       `grep -n "nightly" TODO/gate.md` names the four ported
             subjects the second workflow consumes.

**Done.** One record names both workflows, and it carries a four-row table
of subject, line, what runs there and what the port leaves: the interpose
build at `:90`, the per-arch smoke at `:116`, SSH packaging at `:133` and
release notes at `:204` of `.github/workflows/nightly.yml`. Only one of the
four still names a script, `scripts/package-ssh.sh` at `:133`, so the
silent breakage is real but narrower than the `Problem` states and the
record says so. It also records that every line number the plan gives is
stale, `nightly.yml:84`, `:103`, `:112` and `:179` holding a step name, a
comment, a `rustup target add` and a `podbox-*` loop today, a drift of 29
lines. `grep -n "nightly" TODO/gate.md` returns 15 lines, exit 0, three
of which are the mentions in this paragraph.

**The four subjects, with the line each occupies today.** Read from
`.github/workflows/nightly.yml` on 2026-10-02, the `build` job:

| subject | line | what runs there | what the port leaves |
| --- | --- | --- | --- |
| interpose build | `:90` | `./target/release/podbox-interpose-build` | already a binary; `scripts/build-interpose.sh` is what wave 4 replaces |
| per-arch smoke | `:116` | `./target/release/podbox-smoke <triple> <qemu>` | already a binary; its tracked source is `crates/podbox-gate/src/smoke.rs` |
| SSH packaging | `:133` | `sh scripts/package-ssh.sh <release> <arch> <qemu>` | a shell script still; the plan's repoint lands in the wave-5 release packaging work |
| release notes | `:204` | `./dist/release-notes-x86_64 "$ref_name" > release-notes.txt` | already a binary, couriered from the x86_64 leg at `:129` |

Two of the four are already ported on this tree, which the plan does not
know: `podbox-interpose-build` and `podbox-smoke` are built by
`cargo build -p podbox-gate --bin` at `:87` and `:113` and copied into
`target/release/` at `:89` and `:115`. ⚠ **Only two of the four still
name a script.** `scripts/package-ssh.sh` at `:133` is the one live
shell path among the four; the other three are binary calls. So the
silent breakage the entry describes is real but narrower than the
`Problem` states: a reader who repoints only `gate.yml` loses the SSH
packaging step, and the other three fail earlier, at their own commits.

⚠ **Every line number in the plan's four-row claim is stale.** The plan
gives `nightly.yml:84`, `:103`, `:112`, and `:179`
(`refactor/06-entries/T-R004.md:191-194`, and the same four at
`refactor/review-final-1.md:57`). Those lines today hold: the step name
`interposer, one object per libc` (`:84`), a comment (`:103`), a
`rustup target add` (`:112`), and a `for bin in podbox-*` loop (`:179`).
The four lines the plan names were correct against the tree it
measured. Verified here by running `sed -n '84p;103p;112p;179p'`, so a
later reader knows the drift is 29 lines and not a different step.

**Why one record is the right shape here.** `gate.yml` and
`nightly.yml` answer different triggers: `nightly.yml:13-15` fires on
`v*` tags alone, and its header at `:5-8` says "Nothing in gate.yml
moves". So no run of the main workflow exercises the nightly build, and
the plan's own finding is that every entry treats "the full gate" as
`gate.yml`'s four jobs. A repoint proven green in `gate.yml` leaves
`nightly.yml` unbuilt until a release attempt, which is the failure the
`Decision` names.


### T-1638 The record gate proves itself with the tool its own port deletes

Source:      `refactor/recon-c.md:185`; `refactor/06-entries/PLAN.md:140-141`
Category:    gate
Priority:    P0
Effort:      S
Status:      done

Problem:     The gate crate's own entry proves itself with the Python
             tool the same port deletes. Read literally the entry can
             never close, because its proof does not survive its own
             completion.
Premise:     The self-reference is real and it is the reason a plant
             encoding the current source must land before the port: a
             plant that passes vacuously is worse than no plant. The
             port row's own id is in `refactor/recon-c.md:185` and is
             named here rather than in the title, because the plan's
             ids are not tracked entries.
Approach:     Record the ordering constraint and the plant that depends
             on it, beside the entry that owns the port.
Decision:     Ordering first, plant second. The plan states that the
             plant must land before the port or the port's plant passes
             vacuously, and that is the clause worth carrying.
Prove:       `grep -n "check-todo.py" TODO/gate.md` names the Python
             proof the Rust port deletes, and the plant that must land
             before it.

**Done.** The self-reference is recorded as already broken:
`refactor/06-entries/T-R000.md:42` proves the wave-0 entry with
`py scripts/check-todo.py`, and `ls scripts/check-todo.py` exits 2 because
commit `996cb98` deleted the file, while the tracked replacement landed in
the same commit at `gate.yml:61-62`. The `Decision` is recorded as
satisfied by a plant other than the plan expected: the anchor at
`crates/podbox-image/src/store.rs:1083` is still current and the plant
landed as `podbox-prove-t0211`, whose binary carries it as
`FORK_PATTERN` and tests the match at `prove_t0211.rs:604`, leaving
`experiments/157-lock-inheritance-prove.sh` a 28-line exec shim. The
record leaves the rule behind it as one sentence: a plant that encodes a
source anchor must be re-anchored in the commit that changes the anchor.
`grep -n "check-todo.py" TODO/gate.md` returns 21 lines, exit 0, two of
which are the two mentions in this paragraph.

**The self-reference, and it is already broken.** The wave-0 entry
proves itself with `py scripts/check-todo.py`, at
`refactor/06-entries/T-R000.md:42`, and repeats the command at `:131` as
the only gate it touches. That file does not exist on this tree:
`ls scripts/check-todo.py` exits 2 with "No such file or directory".
Commit `996cb98` deleted it. So the plan's wave-0 entry, read literally,
cannot close, exactly as the entry's `Problem` states. ✅ The tracked
replacement landed in the same commit: `gate.yml:61-62` runs
`./target/release/podbox-gate`, and `crates/podbox-gate/src/main.rs`
holds the check the deleted Python held.

⛔ **The ordering constraint the `Decision` names is already satisfied,
and it was satisfied by a different plant than the plan expected.** The
plan's clause at `refactor/06-entries/PLAN.md:140` is that "a plant
encoding the current source must land first, or wave 1's plant passes
vacuously", and its subject is `157`: the sed the plan quotes sits at
`PLAN.md:140` and was to be re-anchored to
`crates/podbox-image/src/store.rs:1083`. Two facts close it:

1. The anchor the plan names is still current. `store.rs:1083` reads
   `if !sys::close_in_children(fd) {` on this tree, inside
   `Lock::try_acquire`, which is what `PLAN.md:140` quotes.
2. The plant landed, as `podbox-prove-t0211`. `T-1563` is `done` in
   `TODO/INDEX.md:271` and its binary carries the anchor as a literal,
   `const FORK_PATTERN` at
   `crates/podbox-release/src/prove_t0211.rs:28`, with a unit test
   asserting the pattern still matches at `:604`.
   `experiments/157-lock-inheritance-prove.sh` is now a 28-line exec
   shim over that binary, and its own header at `:5-6` states the rule
   this entry is about: "The binary encodes the current anchors: the guard
   in `Lock::try_acquire` and the one flag in `Lock::open`. A stale
   anchor passes vacuously."

⚠ **`PLAN.md:140`'s line reference is stale and the drift is large, so
a reader following it would edit the wrong file.** The plan quotes the
sed from line 157 of the pre-port
`experiments/157-lock-inheritance-prove.sh`, but that file is 28 lines
long today and has no line 157; the sed the plan describes has been
replaced by the shim's `exec`. The plan was written against
the pre-port script. The same drift affects `PLAN.md:141`, which quotes
line 300 of the deleted `check-todo.py` script and names
`check_tree`; that script is gone, and the tracked equivalent
now lives in the Rust gate rather than at any Python line.

**What is left of the constraint, stated as one rule a later session can
act on.** A plant that encodes a source anchor must be re-anchored in
the commit that changes the anchor, in the same change, or it stops
detecting anything. The proof of that rule on this tree is
`prove_t0211.rs:604`, a test asserting `FORK_PATTERN` still matches.
⛔ The rule has no tracked entry of its own, and this one is the record
of it: the check at `crates/podbox-gate/src/main.rs:4007` reads a bare
plan id as a reference to a non-entry, so the constraint lives in prose
here rather than in a row a later session would be pointed at by a grep.


### T-1621 The 11 Batch 3 rows were already tracked and are done

Source:      `refactor/recon-c.md:146-156`, the plan's rows, read off
             disk 2026-10-02; `TODO/INDEX.md` rows T-1559 to T-1569
Category:    gate
Priority:    P2
Effort:      S
Status:      done

Problem:     Eleven of the plan's unbatched rows are work this repository
             already did and closed. The plan still lists them as open,
             so a session reading the plan would redo them.
Premise:     Measured, not carried: every one of the eleven ids is a
             `done` row in `TODO/INDEX.md`, and the tool crates they
             name were landed with their proofs and plants. The plan's
             own batch table places the same eleven in Batch 3, which
             contradicts its own statement that they name no batch.
Approach:     Record the reconciliation so the discrepancy is visible
             once: eleven ids, one already-closed set, and the plan's
             own batch table agreeing with the closure rather than with
             its unbatched claim. Close this entry when a reader of the
             tracked record cannot reach the wrong conclusion.
Decision:     Record, not delete the ids. The finding is that the plan
             and the record disagree, and the record is the one that is
             current.
Prove:       `grep -cE "T-1(55[9]|56[0-9])" TODO/INDEX.md` returns 11 and
             every one of those rows reads `done`.

**Done 2026-10-02.** Measured on the filed tree, one row per id read
out of `TODO/INDEX.md` rather than by eye:
`T-1559 done`, `T-1560 done`, `T-1561 done`, `T-1562 done`,
`T-1563 done`, `T-1564 done`, `T-1565 done`, `T-1566 done`,
`T-1567 done`, `T-1568 done`, `T-1569 done`. Eleven of eleven, and
the eleven rows the plan's batch table already placed in Batch 3 are
the same eleven its unbatched claim names, so the plan contradicts
itself and the record is the side that is current. T-1613's own
Approach records the arithmetic that found them.

### T-1641 The provisioner writes into the vendor's backing image, and a timed-out run orphans its emulator

Source:      KVM diagnosis 2026-10-02, run against the saved results
             `experiments/results/kvm-guest-2026-10-02.txt` and
             `-third.txt` and against the live failing run;
             `crates/podbox-windows/src/lib.rs` `:239-249`, `:310-338`,
             `:418-433`; `crates/podbox-windows/src/plan.rs` `:195-207`;
             `crates/podbox-windows/src/agent.rs` `:91`;
             `experiments/lib/kvm-guest-base.sh` `:182-189`
Category:    windows
Priority:    P0
Effort:      M
Status:      partial

Problem:     Three defects found while diagnosing why the KVM guest run
             hangs at the firmware handoff. None has an entry, and the
             record gate reads neither source behaviour. ⚠ **Three of the
             stated causes were disproved in source on 2026-10-02 and the
             defects below are the corrected ones; the disproved wording
             is kept under `Disproved 2026-10-02` below.**
Premise:     Read in source and against the saved artefacts, not carried
             from a report. Each confirmed defect is quoted at the line
             that carries it; each disproved cause is marked refuted with
             the line that refutes it.
Approach:     Three separate fixes, in this order.
             1. `setup`'s provisioned base must be a single layer that
                does not depend on the vendor file staying at an absolute
                path. Today `out` is an uncommitted qcow2 that carries a
                `-b` pointer at the vendor VHDX, so a run reads
                `run.qcow2` -> `out` -> `base.vhdx` and moving or
                removing the vendor file breaks every later run. Give
                `setup` a scratch overlay as `root` and commit it into a
                fresh standalone image after the guest powers off, so the
                provisioned base is self-contained. Also correct
                `Plan::root`'s comment at `plan.rs:75`, which calls it
                "the per-run disposable overlay" and is wrong on the
                `setup` path.
             2. A run that reaches its timeout must stop the emulator it
                started, confirm the stop, and say so. Today the kill's
                result is discarded (`lib.rs:240`, `:337`), no post-kill
                check exists, the child's exit status is thrown away
                (`:241`, `:244`), and the `Timeout` message asserts "so it
                was stopped" on a path where no stop was confirmed. Ask
                QMP to quit before the kill, kill the process group, and
                report the kill's own outcome.
             3. The proof's seam step is wrapped `if timeout 600 ...;
                then scode=$?`. The `$?` inside `then` is the status of
                the `if` test, so `scode` is always 0 and the `ok:` line
                always prints "exit 0". Read the exit code first, then
                assert inside. Also make the step report that it was
                skipped when an earlier failure closed the guard, rather
                than printing nothing.
Decision:     Fix the kill discipline first. It is the one defect with a
             filed artifact, and it is a host-stability risk on a machine
             whose emulator is not group-killed: a survivor has already
             been measured at
             [the 2026-09-30 third run](../experiments/results/kvm-guest-2026-09-30-third.txt)
             lines 52-53, where the proof's own selector found the
             emulator and `stop_owned_emulators` failed to clear it. The
             image chain is a correctness and portability defect rather
             than a stability one, and the seam step is a reporting
             defect. A corrected KVM run needs all three in place.
Prove:       `podbox windows setup` produces a standalone provisioned
             base whose backing pointer is gone, read with `qemu-img
             info`, and leaves the vendor image byte-identical by digest
             before and after; `podbox windows run --podbox-timeout N`
             over a command that cannot answer leaves no `qemu-system-*`
             process behind in the base, and its Timeout message names
             whether the stop was confirmed rather than asserting it; a
             failing seam step turns `experiments/392-kvm-guest.sh` red
             and names its exit code, and a step closed by an earlier
             failure says it was skipped; each fix carries a plant that
             fails without it.

**Read 2026-10-02, from the failing run.** The provisioned image is
54 MiB of writes with the boot loader present and nothing else; the run
overlay is 384 KiB; the serial log ends at `BdsDxe: starting Boot0002`
on `wqroot`; nothing was written to the mailbox; and `setup` and `run`
build the same command line byte for byte, differing only in which disk
is `root` and how long the host waits. So `setup` is not a valid control
for "the guest boots and runs its scheduled task": it reaches `shutdown`
by construction, because its drive-scan loop is bounded and ends at the
same place whether or not it found the mailbox. The distinction that
matters is which mechanism ends each boot.

Two of the three are unfixed and named rather than quietly absorbed: the
emulator orphan is confirmed by the live observation, and the seam
wrapper is confirmed by reading the file, where the 2026-09-30 third
result shows the seam printing nothing while the verdict came only from
an earlier `FAIL`. A fourth possibility is left open: `os_boot` sleeps
the whole `--podbox-timeout` before typing, so a 600 s setup waited
about 10.5 minutes for one boot, and "waited because the guest booted and
the typed scan missed" is not distinguishable from "waited for another
reason" without timing the phases.

One witness is unreliable and its own comment claims otherwise.
`experiments/lib/kvm-guest-base.sh:158-161` says the serial watcher
exists so a future hang can say where it stops. It cannot: the
2026-09-30 third run has the same BDS-only serial ending and its `ver`
had already succeeded, and the copy the watcher produces carries a
control-byte prefix the real capture does not have. Fixing that comment
belongs here, with the kill discipline.

**Disproved 2026-10-02, read in source and against the artefacts.**
Each claim below was checked against the file that carries it. Three of
the causes this entry named are refuted and one rests on evidence that
is not filed anywhere. They are kept here because the correction is more
useful to the next reader than a silent rewrite would be.

| Claim | Verdict | Evidence |
| --- | --- | --- |
| `overlay_argv` passes no read-only backing | refuted | `plan.rs:195-207` passes `-F` and `-b` |
| `setup` writes into the pinned VHDX | refuted | every write in `podbox-windows` targets `mailbox`, `root`, or `monitor`; `base` reaches only `overlay_argv`'s `-b`, and there is no `qemu-img commit`, `map`, or `convert` in the crate |
| `lib.rs` discards the reaped child's exit status | confirmed | `lib.rs:241`, `:244` |
| nothing verifies the emulator died | confirmed | `lib.rs:239-243`, `:337-338` |
| the kill is ungrouped and its result discarded | confirmed | `lib.rs:223-227` sets only stdio; `:240` and `:337` are `let _ =` |
| the orphan is unselectable by the proof and the watchdog | refuted | `experiments/lib/kvm-owned.sh:5-6` selects on `ps -eo pid,args` text and `scripts/windows/kvm-watchdog.py:108-121` on `/proc/PID/exe` plus `cmdline`; neither reads the parent |
| the seam step swallows a non-zero exit and leaves `fail` untouched | refuted | `experiments/lib/kvm-guest-base.sh:191` calls `miss`, and `miss` sets `fail=1` at `:21` |
| `scode=$?` is dead | confirmed | `experiments/lib/kvm-guest-base.sh:183`: `$?` inside `then` is the status of the `if` test, so `scode` is always 0 |
| the seam step is silently skipped after an earlier failure | confirmed | `experiments/lib/kvm-guest-base.sh:181` and [the 2026-09-30 third result](../experiments/results/kvm-guest-2026-09-30-third.txt) lines 49-50, where `== the podbox run seam` is followed directly by `== residue` |
| the orphan ran 8m43s with `commandline` and `parent` NULL in `/proc` | not verifiable | appears only in prose in this entry and in `TODO/RESUME.md:75-78`; no result file under `experiments/results/` records it |

The orphan survives for a reason the record did not name. The selectors
are sound; the kill is what failed. The 2026-09-30 third result shows
the proof's own `owned_emulators` finding a live emulator and
`stop_owned_emulators` sending TERM then KILL without clearing it, so
this is a stop that does not stop, not a selector that cannot see.

The 54 MiB figure this entry quotes has no artefact either. It appears
only in prose at the Read paragraph above; neither
`experiments/results/kvm-guest-2026-10-02.txt` nor `-third.txt` carries
a file size or any `qemu-img` output. The run it describes did print
`INSTALLED D:`, which is the installer's own `SETUP.TXT`
(`agent.rs:90`), so provisioning reached `shutdown` by construction
rather than because the image was corrupted.

⚠ **The Approach this entry first prescribed would have caused the
defect it names.** It asked for `setup` to write into a raw `root` and
`commit` it. `qemu-img commit` folds the backing into the target, so
that change would have made the provisioned image overwrite the vendor
file it is backed by. The correction is a scratch overlay committed into
a *separate* standalone file.

**Partial 2026-10-02.** All three fixes are implemented and the record
gate, `cargo fmt`, `cargo clippy -p podbox-windows --all-targets`,
`cargo check -p podbox-windows --all-targets` and the four prose
checks exit 0. The Prove clause's live half did not run and the entry
stays open on it.

What is implemented. `plan.rs` gains `commit_argv`, a pure function
beside `overlay_argv`, so the commit is pinned by test without qemu.
`lib.rs` gains `Stop::{Confirmed, Attempted}`; `Confirmed` comes only
from an observed reap and `Attempted` from anything else, and
`Error::Timeout` carries it, so the message no longer asserts a stop
nobody checked. `spawn_emulator` sets `process_group(0)` before the
child exists and `stop_emulator` sends QMP `quit`, then SIGKILL to the
group, then confirms by reaping. `provision` boots a scratch overlay
and commits it into a separate standalone `out`, and `setup.rs` leaves
`plan.root` in the per-run directory. `dos.rs` takes the same discipline
because it had the same unverified kill. In the proof, the seam reads
its exit status before any test and every guarded step reports that it
was skipped.

What still owes, precisely: one guarded KVM run under the T-1609
watchdog, which shows `qemu-img info` on the committed image naming no
backing file, the vendor digest unchanged across `setup`, a timed-out
`run` leaving no `qemu-system-*` behind, and the seam step naming a
real exit code.

⛔ **This session first wrote that the run was impossible on this host,
and that was wrong. The binary builds; it was built in the wrong
place.** The earlier text claimed `ring`'s build script cannot run here
because it invokes `scripts/zig-cc.sh` through a Windows process
spawner (`os error 193`), and it reported that failure as pre-existing
on the clean tree. Both observations were real and the conclusion was
not: `docs/containers.md:33` says Linux builds run in a job container
in `wsl-toolkit-podbox`, and `scripts/dev-lane.sh` is the one lane
runner. Building on the Windows host is what produced the error. Run
through the lane on 2026-10-02, the same build finishes in 55 s on
`rustc 1.99.0` with zig 0.16.0 at `/usr/local/bin/zig` and produces
`target/x86_64-unknown-linux-musl/release/podbox`, 3277880 bytes, exit
0. So the blocker was the lane this session skipped, not the toolchain.

The image is present and correct at
`/c/Users/AjamX/podbox-images/ValidationOS.vhdx`, 910163968 bytes, the
pinned length. The binaries under `.dev/artifacts*` are from 2026-09-30
and predate every change here, so a run with any of them would prove
nothing; the run needs the binary this lane builds.

⛔ **DEFERRED BY THE OPERATOR, 2026-10-02. Do not start a KVM guest for
this entry until the operator lifts it.** It stays `partial` and its
remaining acceptance is the one paragraph above, which is recorded
rather than lost. `TODO/RULES.md` section 11 carries the decision and
its clearing condition: the operator saying the KVM work resumes. Elapsed
time does not clear it and a green record does not clear it. The code is
in place and compiles; only the live run is deferred. The nine tracked
records written alongside it, including `scripts/build-interpose.sh`, the
CI interposer step and the `detached_stdio` tier skip, are NOT deferred
and stay live under T-1604.

The plant this entry owes is also still open: none of the three fixes
carries one, because a plant that fails without them needs the guest or
a signal this host cannot deliver.
