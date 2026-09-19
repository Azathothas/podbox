# gate

`docs/methodology/gate.md` and `docs/methodology/experiments.md`. What makes a
check an assertion rather than a decoration, and what makes a number a
measurement rather than a property of one host.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ A check lands with the plant that proves it can fail, in the same change.

---

### T-1201 The gate reaches every file this project wrote

Source:      `docs/methodology/gate.md`; `docs/conventions/docs.md:80-92`
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
             link. `./scripts/check-todo.py` prints the reading on every run,
             and no document copies it.
             What is fixed rather than measured, and safe to state: bare path
             citations were an **unchecked class** before this entry, and
             `tree_citations` counted **1**.
Approach:    Checks 11 to 15 of `scripts/check-todo.py`. Citations and links are
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
Prove:       `./scripts/check-todo.py && ./scripts/plant.sh`

**Done.** `./scripts/check-todo.py` exits 0 and prints a coverage line; the two
plants for these checks, cases 11 and 14 in `scripts/plant.sh`, both go red.

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
             `scripts/plant.sh` plants a defect per case and asserts each makes
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
             `./scripts/plant.sh` prints the current pair on every run, and it
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
Prove:       `./scripts/plant.sh`

**Done.** Exit 0: 20 plants caught, 0 missed; 3 controls quiet, 0 fired.
⚠ The pair moves whenever a check lands. `./scripts/plant.sh` prints it, and
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
             it. `scripts/check-todo.py` went green throughout, because the file
             it reads did not change. ⚠ The ceiling is still enforced at BUILD
             time by `experiments/110-bloat-delta.sh`, which exits 1 over it, so
             nothing is currently unprotected: what is missing is the half that
             runs on a clone with no toolchain, which is the half check 17 was
             written to be.
Approach:    Read every `experiments/results/bloat-*.txt` rather than the
             baseline alone, and assert each `total_bytes` is under the ceiling.
             ⛔ Still no build: the check reads committed evidence, so it keeps
             working on a fresh clone with no cargo, which is
             `scripts/check-todo.py`'s own constraint.
             ⚠ The baseline keeps its own separate assertion, because a missing
             baseline means there is no "before" and `TODO/deps.md` T-0910 rules
             that a dependency without one has not landed.
Decision:    Every reading, not "the newest by date". A date inside a file is a
             string this script would have to parse and rank, and a stale file
             somebody forgot to delete would then silently outrank a real one.
             Asserting all of them needs no ordering and fails on the same file
             either way.
Prove:       `./scripts/plant.sh` reports case 17d caught, planting a `total_bytes` over the ceiling into `experiments/results/bloat-image.txt`

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
one too, and its home is `scripts/plant.sh`'s own verdict line.

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
Approach:    A check in `scripts/check-todo.py` that collects the leading number
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
Prove:       `./scripts/plant.sh` exits 0 with cases 18a and 18b caught, each planting a duplicate experiment number

**Done, 2026-09-08.** Check 18 of `scripts/check-todo.py`, and cases 18a and
18b of `scripts/plant.sh`, in this change. `./scripts/plant.sh` exits 0 with
**20 caught, 0 missed, 3 controls quiet**.

⭐ **The check was written before the renumbering and run against the tree that
still carried all four collisions.** It reported exactly the four this entry
records and named the same `file:line` on each. A hand sweep and a check that
agree independently is the check having been tested against a real defect as
well as a planted one, which is the half `scripts/plant.sh` cannot supply.

⚠ **Two cases, not one, because the halves fail apart.** A number can be
claimed by a document promising a script (all four real collisions were this
shape) or by a second file arriving on disk. A case for one leaves the other
unseen, which is this harness's own founding defect.

⛔ **The planted number is read out of the listing at run time and never
written into `scripts/plant.sh`.** The gate reads that file like any other, so
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
             | what a local `./scripts/dev.sh check` said | green, on every one of them |

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
             value. Check 19 of `scripts/check-todo.py` derives the required
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
Prove:       `./scripts/plant.sh` exits 0 with cases 19a and 19b caught, and the gate workflow is green on `main`

**Done, 2026-09-09.** Check 19 of `scripts/check-todo.py`, cases 19a and 19b of
`scripts/plant.sh`, and `zig` in both cargo-running jobs of
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
time and never written into `scripts/plant.sh`.** Naming it there would be a
third declaration of the value this check exists to keep in one place, which is
the trap `CEILING_NUM` and `TAKEN_EXP` were each written to avoid.

---

### T-1207 The one crate that runs inside other people's processes is the one the gate does not check

Source:      `Cargo.toml:13-18`; `scripts/dev.sh:199-202`
Category:    gate
Priority:    P1
Effort:      L
Status:      open

Problem:     ⛔ **`crates/podbox-interpose` is excluded from the workspace, and
             every step of `./scripts/dev.sh check` is scoped to the
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
Approach:    A second scope, not a second gate:
             1. `dev.sh check` runs the same four steps against
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
Decision:    Not taken on the shape. ⚠ Whether this belongs in `dev.sh check`
             or in a `dev.sh check --all` matters: check is what a change
             passes before it is committed and it is currently about 30 s, and
             a second toolchain invocation on every commit is a cost the
             operator should rule on rather than inherit.
Prove:       `./scripts/plant.sh` gains a case that introduces a clippy failure
             and an unformatted line in `crates/podbox-interpose` and asserts
             `./scripts/dev.sh check` goes red naming that crate, which it does
             not today.


---

### T-1208 A closed entry carries its recorded run, and the gate can see it

Source:      [RULES.md](RULES.md) section 5; the reconciliation of 2026-09-11
Category:    gate
Priority:    P1
Effort:      M
Status:      open

Problem:     [RULES.md](RULES.md) section 5 says an entry closes in place with
             its `Prove` command actually run and the output recorded underneath.
             **Nothing asserts it.** `scripts/check-todo.py` checks the counts,
             the rows, the fields and every citation, and it does not check the
             one thing that makes a closed entry mean anything.
Premise:     **Measured by `experiments/156-closure-records.sh`, which is the
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
             3. the plant, in `scripts/plant.sh`: delete the record from one
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
Prove:       `./scripts/check-todo.py` reports a `closure_records` coverage count equal to the number of closed entries, and the plant for it goes red naming the entry whose record was removed

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
             3. the check, in `scripts/check-todo.py`: a `Prove` line may name
                no unqualified image reference and no `docker.io/` reference;
                ⚠ **and `README.md`'s quick start names `alpine:latest` twice**,
                which is the same reference and not a `Prove` line. It is a
                reader's own quota rather than the acceptance's, so it is not
                this entry's defect; it is written down here so the next sweep
                does not find it and think nobody looked;
             4. the plant, in `scripts/plant.sh`: put `alpine:latest` into one
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
Prove:       `./scripts/check-todo.py` exits 0 with the new check in place, and `./scripts/plant.sh` reddens it by name

**Done 2026-09-19.** The mapping, the sweep and the check, in two changes.
The sweep moved 41 violating `Prove` lines to their `DISTRO_ROWS_M5` rows in
the same ten files the Premise names, aligned the [T-0408](complete.md) tag
to its row, reworded the two history notes in [T-0702 and
T-0706](interpose.md) to name the defect without the literal so their
`Prove` lines stay fully checked, and stated the mapping and the corrected
count in this entry. The check and its plant landed together, per
[T-1202](gate.md): check 21 in `scripts/check-todo.py` with cases 21a and
21b in `scripts/plant.sh`, one per arm because the two spellings fail apart.

```
$ py scripts/check-todo.py; echo EXIT:$?
check-todo: 136 rows, 136 entries, 38 open, 3 partial, 0 blocked, 95 done
check-todo: coverage bare_citations=1327 ci_components=3 corpus=41 counts=5 crossrefs=478 entries=136 exit_codes=5 experiment_numbers=471 fields=1360 prove_registry=136 rows=136 size_ceiling=232 todo_citations=101 todo_links=559 tree_citations=64 tree_links=342
check-todo: ok
EXIT:0
$ sh scripts/plant.sh; echo EXIT:$?
  plants   26 caught, 0 missed
  controls 3 quiet, 0 fired
  every check that was planted against went red with its own message.
EXIT:0
```

⚠ `plant.sh` ran under a `python3` shim on PATH pointing at the real
interpreter, because this host's `python3` is a Microsoft Store stub that
exits without running anything. The shim lived for the one command and was
removed afterwards.

---

### T-1210 Convert the interpose engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      open

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

---

### T-1211 Convert the distribution and probe engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      open

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

---

### T-1212 Convert the image, registry and CLI engine scripts to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/105-interpose-ownership.sh`
Category:    gate
Priority:    P1
Effort:      M
Status:      open

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
             `./experiments/280-insecure-registry.sh`, `./experiments/300-run.sh`,
             `./experiments/320-cli-contract.sh` and `./experiments/330-exit-codes.sh`
             each exit 0 on host podman with the conditions block naming the
             driver, and `experiments/results/` carries the runs.

---

### T-1213 Convert the target-image pair and its probe consumer to `experiments/lib/engine.sh`

Source:      `experiments/lib/engine.sh`; `experiments/130-probe-parity.sh:1-40`
Category:    gate
Priority:    P1
Effort:      M
Status:      open

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

