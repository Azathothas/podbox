# cli

`crates/podbox-cli`. `TOOL.md` section 6.8 and section 6.9.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ **An agent that knows docker must need zero new knowledge.** That is the
product requirement the whole specification exists for. Same verbs, same flags,
same exit codes; where a flag cannot be honoured it is accepted and reported in
the banner, and where a flag **requests isolation** it fails loudly with a
named reason.

---

### T-0801 The verb and flag parity table

Source:      `TOOL.md` section 6.8
Category:    cli
Priority:    P0
Effort:      L
Status:      done 2026-09-09

Problem:     A tool that needs its user to learn its differences has not
             replaced anything. Thousands of agents reach for `docker` because
             it is the only container language they know.
Premise:     Read. `TOOL.md` section 6.8 is the table, in full, with a status per verb
             and per flag. Four statuses: **Native** (real semantics),
             **Degraded** (works, documented difference, stated in the banner),
             **Stub** (accepted, no-op, listed in the banner), **None** (fails
             with a named reason).
Approach:    Implement the table as data, not as a match arm per flag, so that
             `podbox system info` can print it and a test can assert every row
             is reachable. A flag with no row is a bug in the table, and the
             parser says so rather than ignoring it.
             ⭐ The posture to copy is `dockless`'s, at
             `references/ylang-ylang__dockless/tree/dockless/run.py:129-160`.
             `guard_run` refuses `run --name` up front because the engine
             beneath it cannot honour it, and warns hard when `--rm` names an
             existing container, citing the incident that produced the guard.
             ⛔ dockless has **no licence statement of any kind**
             ([reference-map.md](reference-map.md)), so the posture is described
             and adopted as a design and no line is copied.
             ⚠ Its degradation is worth noting from the other side: when it
             cannot determine the target it prints that the guardrail itself is
             degraded and continues. Naming the degradation of the guard is the
             part to keep.
Decision:    A table plus a generated parser, over a hand-written one. The table
             is what section 6.8 already is, and generating from it makes "the flag
             exists and is unlisted" impossible.
Prove:       `./experiments/320-cli-contract.sh` clauses 1 to 3, whose first clause is `podbox system info --format '{{json .Parity}}' | jq -e 'length >= 60 and all(.status | IN("Native","Degraded","Stub","None"))'`

**Done, 2026-09-09.** `crates/podbox-cli/src/parity.rs` is the table,
`crates/podbox-cli/src/system.rs` is `podbox system info`, and
`experiments/320-cli-contract.sh` drives all three of the contracts below
against the shipped binary.

| | |
| --- | --- |
| rows | **220**. ⚠ 131 when this closed; the table grows with the flags, and this row is the CURRENT count so the two cannot drift apart |
| of which verbs | **69** |
| statuses used | Native, Degraded, Stub, None, and no fifth |
| rows with no reason | **0** |

⭐ **THE TABLE DECIDES, and that is the whole difference from a table that
merely describes.** Every argument beginning with `-` goes through
`parity::admit` before any match arm, so the two failures a hand-written parser
has are both structural rather than discouraged: a flag with no row cannot be
quietly accepted, because it never reaches an arm; and an arm for a flag with no
row is unreachable, because `admit` refused it first. A unit test walks the
table and asserts every row the table admits reaches an arm, ⚠ **by the exit
code rather than by a shape parsing**: `--pull` takes a closed set of values and
rejects anything a test could invent, so what the test asserts is that no shape
returns the fallback arm's own `EXIT_RUNTIME_ERROR`.

⚠ Written as universal on 2026-09-09 and measured as partial on 2026-09-21:
only `run`, `exec` (and `create` through `run`'s parser) admitted first;
`images`, `system`, `lifecycle`, `probe`, `version` and the `image` group
refused from hand-written arms. T-0808 threaded `parity::admit_all` through
every one of those parsers, so the sentence above holds since 2026-09-21 and
`experiments/325-parity-drive.sh` asserts it per row, starting from the flag.

⭐ **The refusal carries the row's own reason**, so a caller that reads the
table and then runs the verb gets the same sentence back from the same place:
`podbox run --name c1` exits 2 with "`--name` is in the parity table with status
None: a name identifies a container, and podbox has none until M4", and
`podbox ps` exits 125 with that verb's row. Clause 3 asserts the message a verb
prints is byte-for-byte the note the table published.

⛔ **`--format` grew exactly one function, `json`.** `{{json .Parity}}` is
docker's own spelling and it is T-0801's `Prove`, so refusing it would have made
the acceptance unwritable. It is not a general function call: a verb declares
which of its fields are already documents, `{{json .X}}` on a plain string
quotes and escapes it, and every other function is still refused by name. ⚠ The
alternative, guessing from the value's first byte, makes a string that happens
to begin with `[` come out unquoted, which is a wrong answer that parses.

⚠ **A `None` row is not a placeholder.** It is the sentence a caller gets
instead of a flag being ignored, so deleting one makes podbox quieter and less
honest rather than smaller. A test asserts every one of them carries a reason
longer than a milestone number, which caught eleven rows whose whole note was
"the lifecycle is M4".

**Amended, 2026-09-10, by the door sweep after T-0412.** ⛔ **One verb was
served by another verb's parser and nothing said so, in either direction.**
`create` is parsed by `run::parse`, which called `admit("run", …)` with the verb
written in rather than passed. Two consequences, and each is the class this
entry exists to make impossible:

- ⛔ `podbox create --no-steps` was **accepted and acted on** while
  `podbox system info` listed `--no-steps` under `run` and `exec` only. The
  table under-described the binary, which is "the flag exists and is unlisted"
  arriving through the door the entry did not enumerate.
- ⛔ `podbox create --no-such-flag` answered **`podbox run: unknown option`**
  and printed `usage: podbox run`, naming a verb the caller had not typed, and
  then sent them to "every flag this verb takes" -- a row set that does not
  exist.

`parity::rows_of` is the one place that says where a verb's rows live, so the
28 rows stay a single copy; `run::usage(verb)` builds the first line from the
verb the caller typed; and the `create` row and the refusal message both name
the borrowing out loud. ⚠ Threading the verb was not cosmetic: `admit` looks the
flag up, so naming the caller's verb without `rows_of` would have made `create`
refuse every flag it accepts. Six functions in `run.rs` took the verb --
`parse`, `prepare`, `acquire`, `extract_now` and `config_of` among them --
because each wrote `podbox run:` into a message `create` and `exec` can reach.

Prove: `a_verb_served_by_another_parser_resolves_that_verbs_rows_and_says_so`
asserts both halves, and driven: `podbox create --no-such-flag` now names
`create`, `podbox create --no-steps` is still accepted, and the table is 141
rows as before, because the fix added none.

**Partial, 2026-09-25.** Issue 60 reopens this entry on its own
invariant: a flag with no row is a bug in the table, and 46 `run`
flags (`--read-only`, `--mount`, `--env-file`, `--label`,
`--init` among them) plus 10 verbs (`manifest`, `service`,
`secret`, `trust`, `config`, `node`, `plugin`, `stack`,
`checkpoint`, `scan`) have no row at all. Measured on the lane
2026-09-25 (`experiments/results/triage-353.txt` clauses
`60-read-only`, `60-mount`, `60-env-file`, `60-manifest`,
`60-service`): each flag answers `unknown option` with no reason
at 125, each verb exits 1 on the neither-podbox-nor-docker arm
with the usage text claiming every other docker verb is named in
the table, which is not true while these are missing. Add a
`None` row with the fitting reason for each refused flag (the
peers already name the reasons: no mount, no namespace, no
cgroup), a `Native` row with a parser arm for any flag promoted
to real behavior (`--env-file`, `--label`, `--init`,
`--read-only`), and rows for the ten verbs on the existing
no-daemon and out-of-shape reasons. Correct the usage sentence
beside the rows. Keep the parser answering `unknown option` only
as the backstop, never as the documented answer. The `--device`
and `--log-driver` rows ride here too (issues 55 and 56, filed
apart because entries name them). Fix area is
`crates/podbox-cli/src/parity.rs` with the `run` and `exec`
parsers for promoted flags. Risk if wrong is an operator
trusting an enumerable table that omits the docker surface they
script against. Prove is the curated flag list from the issue
driven row by row through the shipped binary with every refusal
naming its reason, beside the T-1325 check extension below.

**Done, 2026-09-25.** Every one of the issue's 46 flags and 10
verbs has a row, in `crates/podbox-cli/src/parity.rs` (220 rows,
69 verbs). `--env-file` is Native with a parser arm and a reader
(`run.rs read_env_file`: `KEY=VALUE` lines loaded at the flag's
position, `#` comments and blanks ignored, one matching quote pair
stripped, 1 MiB ceiling, UTF-8, a line without `=` refused naming
its number). `--label`, `--attach` and `--expose` are Stub.
`--log-driver` is Stub for `json-file`, accepted and changing
nothing (one raw interleaved file either way), with any other
driver refused naming it: the shape T-0605 prescribes. The
remaining 41 flags are None with the fitting reason, and the 10
verbs are None on the existing no-daemon and out-of-shape
reasons. The usage sentence in `main.rs` stands unchanged: with
the rows added it is true again, and check 27 holds it.
Prove: `experiments/355-parity-curated.sh` exit 0 on the lane
(kernel 7.2.0-WSL2-STABLE, lane-built musl debug binary
0.1.0-beta.7, pinned alpine:3.20 digest `d9e853e8`):
clause-1 41/41 refused with status None at 125, clause-2 10/10
verbs refused at 125 with their notes, `--env-file` end to end
(`from-file` on payload stdout, a later `-e` winning, the `=`
form equal, a bad line and a missing file at 125), the stub
trio admitted at 0 with `--strict` refusing at 125 naming Stub,
`--log-driver=json-file` accepted in both forms with `syslog`
refused at 125 naming the value, `create` inheriting all of it.
Report in `experiments/results/parity-curated.txt`. In-suite:
`issue_60_curated_surface_stays_covered`,
`env_file_loads_and_stubs_parse`,
`log_driver_takes_json_file_and_nothing_else`, podbox-cli 139
passed, 0 failed. Guards are check 27's curated arm and the
in-suite test beside it.

---

### T-0802 docker's exit codes, unaltered

Source:      `TOOL.md` section 6.8
Category:    cli
Priority:    P0
Effort:      S
Status:      done

Problem:     An automated caller reads the exit code first. A runtime that
             returns its own codes breaks every script that branches on
             docker's, and a runtime that improves a payload's code lies in the
             field read first.
Premise:     Read. docker's convention: the payload's own status for `run`, 125
             when the runtime itself could not run the command, 126 when the
             command was found and could not be invoked, 127 when it was not
             found.
Approach:    Return the payload's status verbatim from `run`, `start` in the
             attached case, and `wait`. Use 125, 126 and 127 for the three
             runtime-side cases and nothing else.
             ⛔ Never translate a payload's non-zero status into zero because a
             fixup succeeded, and never into non-zero because a fixup warned.
             That rule is what T-0409 depends on.
Decision:    Match docker rather than define a clearer scheme. Parity is the
             product.
Prove:       `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'exit 42'; test $? -eq 42 && podbox run --rm public.ecr.aws/docker/library/alpine:3.20 /nonexistent; test $? -eq 127`


**Done 2026-09-09, and the entry's own description of docker's convention was
incomplete.** `experiments/330-exit-codes.sh` runs the same case through both
binaries on one host and compares the numbers, which is the only way this can be
asserted: docker's contract is not written anywhere podbox can read, it is what
the binary does.

⭐ **THE DISCRIMINATOR IS NOT THE ONE THE ENTRY DESCRIBES, AND IT IS THE
FINDING.** docker exits **125** for anything its FLAG PARSER refuses and **1**
for anything the verb refuses afterwards. Measured on docker 29.3.1:

| case | docker | what podbox did before |
| --- | --- | --- |
| `run --badflag`, `--pull=bogus`, `--memory=notasize` | **125** | 2 |
| `images --format '{{.Nope}}'`, `run` with no image | **1** | 2 |
| `rmi no-such-image` | **1** | 125 |
| a verb neither tool has | **1** | 125 |
| the bare name, no arguments | **0**, help on stdout | 125 |
| the command found and not invocable | **126** | **127** |
| the command not found | 127 | 127 |
| the payload's own status, `128+N` for a signal | as docker | as docker |

Four of those eight were wrong. ⛔ The 126/127 one is the one that costs a
caller most: every `execve` failure was folded into "not found", so
`podbox run alpine /etc/passwd` said 127 where docker says 126, and a script
branching on 127 retries with another path. The split is on the **errno**:
`EPERM`, `ENOEXEC`, `EACCES`, `EISDIR` and `ETXTBSY` are 126; `ENOENT` and
anything about resolving the path are 127.

⭐ **The codes now live in ONE file**, `crates/podbox-probe/src/exit.rs`, and
are served as data by `podbox system info --format '{{json .ExitCodes}}'`.
⛔ They had been written out in **three**: `podbox-image::error`,
`podbox-cli::main` and `podbox-enter`, and two of the copies had already
diverged -- `main.rs` still carried the old usage code while `error.rs` carried
the corrected one, so two verbs of one binary disagreed about what a flag error
is. `scripts/common/exit-codes.sh` is how a shell script reads the table, and
six clauses across four experiments read it there instead of carrying a `2`.

⛔ **A fixup never moves the payload's code**, which is what
[T-0409](complete.md) depends on: clause 5 of the measurement runs the same
payload with and without the completion layer's source rewrite and asserts both
report the payload's own 7.

Prove, run 2026-09-09:

```
$ ./experiments/330-exit-codes.sh
  15 cases, podbox and docker agree on every one
  exit 0
```

---

### T-0803 Answer to `docker` and `podman` on PATH

Source:      `TOOL.md` section 2.0, section 6.8
Category:    cli
Priority:    P1
Effort:      S
Status:      done 2026-09-09

Problem:     An agent runs `docker`. If podbox is only reachable as `podbox`,
             it has replaced nothing on the machines it is for.
Premise:     Read. On the studied runtime `docker` on PATH is already a podman
             alias with no daemon, so the name is available and currently
             resolves to something that does not work.
Approach:    Multicall on `argv[0]`: `docker`, `podman` and `podbox` all enter
             the same parser, and the banner names which. Install the two extra
             names as symlinks, never as copies, so one binary is one artefact.
             ⚠ Check what is already on PATH before replacing it, and refuse to
             overwrite a real docker client without an explicit flag. A machine
             with a working daemon is a machine where podbox is the wrong tool.
Decision:    Symlinks over a wrapper script. A shell wrapper is a second
             artefact, it breaks the memfd rung (T-1001), and it is the exact
             thing `references/qaidvoid__onelf` records as skipping that rung.
             ⭐ **RULED by the operator on 2026-09-08, and no longer open:**
             podbox **refuses** to take the `docker` name where a working
             docker daemon is reachable, unless an explicit flag says otherwise,
             and says why in one line. A machine with a working daemon is a
             machine where podbox is the wrong tool.
             The alternative considered and rejected is deferring to the real
             daemon transparently by exec'ing it: it costs an exec on every
             call and it hides which tool ran, which is the class of thing
             `TOOL.md` section 4.1 exists to forbid.
             ⚠ The check is on a **reachable daemon**, not on a `docker` binary
             being present: on the target runtime `docker` on PATH is a podman
             alias with no daemon behind it, so a binary check would refuse on
             exactly the machines podbox is for.
Prove:       `./experiments/320-cli-contract.sh` clauses 4 and 5: both names run the payload and say which tool ran, and the `docker` name is refused where a daemon answers unless `--force`

**Done, 2026-09-09.** `crates/podbox-cli/src/names.rs`, the banner line in `run`
and `exec`, `podbox system install-names`, and clauses 4 and 5 of
`experiments/320-cli-contract.sh`.

⭐ **Multicall on `argv[0]`, and the banner names which name was used.** Taking
the name is the product requirement; taking it silently is what `TOOL.md`
section 4.1 forbids, so every `run` and `exec` reached as `docker` or `podman`
carries one extra line saying this is podbox and where the differences are
listed.

⛔ **The daemon check is on a REACHABLE DAEMON and it is a real request.** The
socket named by `$DOCKER_HOST`, or `/var/run/docker.sock`, is connected to and
asked `/_ping` under a two-second timeout, so a stale socket file with nothing
behind it reads as absent, which is the state the machines podbox is for are
actually in. ⚠ A `$DOCKER_HOST` that is not a `unix://` socket is a THIRD state:
podbox says the guard itself is degraded and continues, which is the half of
`dockless`'s posture worth keeping.

⭐ **The ruling is about the `docker` name only.** On this machine, where a
daemon answers, `podbox system install-names` installs `podman`, refuses
`docker` with the reason, and exits 125; `--force` installs it. ⚠ Clause 5 reads
which state the machine is in first and asserts the other half where it can, so
a machine with no daemon reports the refusal half as unmeasured rather than as a
pass.

⛔ **Symlinks, never copies and never a wrapper script**, and the clause asserts
it on the filesystem rather than on an exit code. A file that is already a link
to this binary is left alone; anything else needs `--force`.

---

### T-0804 The honesty rules, and one switch that makes every degradation fatal

Source:      `TOOL.md` section 4.1, section 6.8; `paper_final.md` section 10.1
Category:    cli
Priority:    P0
Effort:      M
Status:      done

Problem:     The audience is automated. An agent cannot notice that its
             "container" was a `chroot` the way a human skimming a log might, so
             the reporting is the product and not a disclaimer.
Premise:     Read, and both halves are in the corpus.
             `references/RuriOSS__ruri/tree/src/include/ruri.h:237-248` is the
             degradation macro that becomes fatal under one flag.
             `references/multikernel__sandlock` is the counter-example: a
             `--dry-run` that reports no changes while changing files, traced in
             T-0606.
Approach:    Four rules, enforced in the output path rather than in review:

             1. the banner prints on every `run` and `exec`, suppressible by
                config and never by default;
             2. a request that **requires** isolation fails with a named reason
                instead of silently using the host;
             3. `inspect` reports the true mode per container;
             4. no output may imply namespaces, cgroups or devices exist when
                they do not.

             ⭐ Add `--strict`, which turns every Degraded and every Stub into a
             refusal, so a caller that needs real semantics can demand them in
             one flag rather than checking the banner.
Decision:    Suppressible by config, never by default. `ruri` allows
             `--disable-warnings` to silence its degradation notices, which is
             the `sandlock` failure mode with a flag in front of it; podbox's
             equivalent is a config file a machine's operator sets once, and it
             cannot be set from the command line of a single run.
Prove:       `! podbox run --rm --network=none public.ecr.aws/docker/library/alpine:3.20 true && ! podbox run --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true && ! podbox run --strict --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true`


**Done 2026-09-09.** All four rules, and the third and fourth needed a defect
fixed rather than a switch added.

1. **The banner prints on every `run` and `exec`**, suppressible by
   `<store>/config` with `banner = quiet` and never from the command line. ⭐ A
   file a machine's operator sets once is the Decision: `ruri`'s
   `--disable-warnings` is the `sandlock` failure with a flag in front of it.
   ⚠ Whether a machine has suppressed it is itself reported, by
   `podbox system info --format '{{.Banner}}'`, because a silent banner nobody
   can tell from an absent one is the same shape again. ⛔ A `--strict` refusal
   prints whether or not the banner is quiet: the switch silences a notice,
   never a refusal.
2. **A request that requires isolation fails with a named reason.** Every
   isolation flag -- `--network`, `--privileged`, `--cap-add`, `-m`, `--cpus`,
   `--hostname`, `-v`, `-p`, `-u` -- is a `None` row and
   [T-0801](#t-0801-the-verb-and-flag-parity-table-as-data)'s `parity::admit`
   refuses it up front with the row's own reason.
3. **`inspect` reports the true mode per container**, and the container record
   now carries `Complete.Fixups` and `Complete.Degraded` beside `Rung`. A
   `/dev/null` that is a regular file is part of the mode, and a report that
   left it out would say devices exist when they do not.
4. ⛔ **THE FOURTH RULE WAS BEING BROKEN, AND BY THE BANNER ITSELF.** The banner
   printed `mode=` **the rung the PROBE selected**, and `podbox_enter` performs
   a plain `chroot` on every machine. On this host, where the probe selects
   `namespace`, every `podbox run` printed

   ```
   podbox 0.1.0: mode=namespace (namespaces: as configured; mounts: full; ...)
   ```

   for a payload that had no namespace and no mounts at all. It now reads the
   rung the entry sequence IMPLEMENTS, from one constant
   (`podbox_enter::ENTERED_RUNG`), and prints the machine's own answer beside
   it rather than dropping it:

   ```
   podbox 0.1.0: mode=chroot (namespaces: uts-only; mounts: none; devices: real;
   ownership: real; network: host-shared; pids: host-shared)
   this mode does NOT provide: process, network, IPC or mount isolation
   ⚠ this machine would permit `namespace`; podbox's entry sequence is `chroot`
     and creates no namespace and mounts nothing
   ```

⭐ **`--strict` reads three inputs and names every reason at once**, rather than
the first one it finds:

- **the flags this invocation passed**, against the parity table's own status.
  A `Degraded` or `Stub` row is a difference this run actually incurs;
- **the rung podbox entered with**, against `Selection::STRICT_FLOOR` -- the same
  constant `podbox probe --strict` gates on, not a second spelling of it;
- **the completion layer's report**, one row per fixup marked degraded.

⛔ A `None` row was already fatal without `--strict`, and has been since T-0801
made the table binding, so the switch is about the two statuses that otherwise
let a run proceed.

⚠ **THE `Prove` ABOVE CANNOT PASS AND ITS SECOND CLAUSE IS THE REASON.** It
expects `podbox run --rm --memory=1g ... true` to exit **0** and the same run
under `--strict` to fail. T-0801 landed after this entry was written and made
every `None` flag a refusal in the parser, so `--memory=1g` exits 125 with or
without `--strict` -- the entry's own rule 2, enforced earlier than it expected.
The rewrite, run 2026-09-09:

```
$ podbox run --rm public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  0, and the banner carries `mode=chroot` and the two lines above
$ podbox run --strict --rm public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  podbox run: --strict, and this run is degraded in 2 way(s). podbox refuses
  rather than running and letting the payload discover them:
    - the selected rung is `chroot` and not `namespace`, so the payload shares
      this machine's process table, network, IPC and mount namespaces
    - rewrote etc/mtab (T-0405): ...
  125
$ podbox run --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  podbox run: -m, --memory is in the parity table with status None: resource
  limits need a cgroup this runtime does not grant
  125
```

---

### T-0805 Diagnostics that name the operation, the errno, the mechanism and the remedy

Source:      `TOOL.md` section 6.9; `paper_final.md` section 10.8
Category:    cli
Priority:    P1
Effort:      M
Status:      done 2026-09-21

Problem:     In this environment the errno alone actively misleads. `EINVAL`
             from `chown` reads as a bad argument and means an id that does not
             exist. `fork/exec <path>: operation not permitted` reads as a
             missing binary and is a denied `setgroups`.
Premise:     Read, and `TOOL.md` section 8 is a whole table of them. The target shape:

             ```
             cannot restore ownership of etc/shadow (uid 0, gid 42): EINVAL
               gid 42 is not mapped in this user namespace (/proc/self/gid_map: 0 1000 1)
               extracting without ownership; intended metadata recorded in .meta.jsonl
             ```

Approach:    Every failure carries four parts: the operation, the errno, the
             mechanism that produced it, and the remedy. The mechanism comes
             from T-0102's discriminator and the maps from T-0105, so the
             diagnostic quotes measured state rather than a guess.
             Encode `TOOL.md` section 8's table as the mapping from a raw failure to
             those four parts, so a new failure mode is a table row.
Decision:    Quote the actual `uid_map` contents rather than saying "an unmapped
             id". The map is one read (T-0105) and it is what turns a confusing
             errno into an explanation a reader can act on.
Prove:       `podbox pull public.ecr.aws/docker/library/alpine:3.20 && podbox extract public.ecr.aws/docker/library/alpine:3.20 2>&1 | grep -A2 'gid 42' | grep -q 'gid_map'`

**Done 2026-09-21.** `crates/podbox-cli/src/diagnose.rs` carries TOOL.md
section 8 as data: 22 rows from observation to operation, errno, mechanism
and remedy. `ownership_note` renders the chown row with live measurements:
the first dropped entry from the sidecar, the map contents T-0105 reads,
and the sidecar path as the remedy. `report_dropped` prints the counts line
and the note. Both `extract` and `run` call it, so one extraction reads one
way on both verbs. The sidecar remembers the first dropped entry and
`Extracted` carries it (`crates/podbox-extract/src/sidecar.rs`,
`crates/podbox-extract/src/lib.rs`).

The entry first named `pull` alone in Prove. Pull performs no extraction
(`crates/podbox-image/src/pull.rs:17`), so no pull output can name a dropped
id. Three routes were checked: a pull path that extracts somewhere unseen
(refuted by the pull handler, which prints the probe source and returns), a
predictive diagnostic inside pull (refuted, since pull cannot know dropped
ownership without extracting), and the pull-then-extract flow (adopted).

Run 2026-09-21 in a disposable container on host podman 6.1.2 with guest
maps `0 1000 1; 1 100000 65536`: pull exits 0, extract exits 0 over 517
entries with 1 dropped, and the note names etc/shadow (uid 0, gid 42) with
EINVAL beside the `/proc/self/gid_map` contents. The unit job exits 0: 71
passed in podbox-cli with 5 new diagnose tests, 46 passed in
podbox-extract. Clippy passes with `-D warnings`. The spawn row in the table
stays unwired until T-0809.

---

### T-0806 Never prompt, never wait unbounded, and check space before every large write

Source:      `TOOL.md` section 11.3; [RULES.md](RULES.md) section 8
Category:    cli
Priority:    P0
Effort:      S
Status:      done

Problem:     A prompt with no terminal behind it is a hang, and a hang is total.
             The three ways a long autonomous run dies are the three ways podbox
             can make its caller die.
Premise:     Read, and one instance is in the corpus at file and line.
             `references/89luca89__lilipod/tree/pkg/utils/utils.go:194-214`
             checks `getsubids`, `newuidmap` and `newgidmap` with `exec.LookPath`
             and returns an unrecoverable error when any is missing, and it runs
             before **every** subcommand including `pull`. A dependency check
             ahead of every syscall wall is the same class of problem as a
             prompt: the tool refuses before it has done anything the user can
             use.
Approach:    Three properties, tested rather than reviewed:

             1. no code path reads stdin unless the user asked for it with `-i`;
             2. every wait has an upper bound and a distinct outcome for reaching
                it (T-0602);
             3. `statvfs` for blocks and inodes before every large write, with
                the destination named in the error (T-0203).

             ⚠ Where podbox needs a tool it does not have, it says so **at the
             point of use** and not at startup, so every verb that does not need
             it still works.
Decision:    Check at the point of use rather than up front. lilipod's shape
             makes `pull` fail on a machine where `pull` would have worked, and
             `pull` is the verb an agent reaches for first.
Prove:       `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 true </dev/null && timeout 30 podbox pull public.ecr.aws/docker/library/alpine:3.20 </dev/null; test $? -ne 124`


**Done 2026-09-09.** All three properties, and each is asserted rather than
reviewed.

1. **No code path reads stdin.** podbox never opens or reads fd 0: the payload
   inherits the caller's, which is what makes `-i` a `Stub` rather than a
   feature. ⚠ `experiments/330-exit-codes.sh` runs every case with stdin
   redirected from `/dev/null` under a `timeout`, so a read that blocked would
   arrive as exit 124 rather than as a hang nobody attributed.
2. **Every wait has an upper bound and a distinct outcome for reaching it.**
   The launcher's are [T-0602](supervise.md)'s. M5 added one more and it is
   bounded too: the HTTPS reachability probe [T-0411](complete.md) makes before
   rewriting a package source gets two seconds to connect and two to read, and
   a host that does not answer keeps its `http://` and is named on the banner.
3. **`statvfs` for blocks and inodes before every large write.** [T-0203](image.md)
   covers the download and the extraction; M5 added the third caller, and it is
   the one a caller with NOTHING can reach: the `/dev` shims are three megabytes
   into a rootfs that may be on a small tmpfs, so `space::require` runs before
   the first of them with the destination named in the error.

⚠ **Where podbox needs a tool it does not have, it says so at the point of use.**
The keyring half of [T-0406](complete.md) is the instance: podbox cannot run
`pacman-key` from outside the chroot, so the fixup reports the state and names
the two commands, rather than refusing the run. That is the Decision's own
shape, against `lilipod`'s at
`references/89luca89__lilipod/tree/pkg/utils/utils.go:194-214`, which refuses
before every subcommand including `pull`.

Prove, run 2026-09-09:

```
$ podbox run --rm public.ecr.aws/docker/library/alpine:3.20 true </dev/null; echo $?
  0
$ timeout 30 podbox pull public.ecr.aws/docker/library/alpine:3.20 </dev/null; echo $?
  0, and never 124
```

---

### T-0807 `podbox images --format` refuses a template no verb can answer, before it looks at the store

Source:      Found by driving the CLI cold against an empty store
Category:    cli
Priority:    P2
Effort:      S
Status:      done 2026-09-08

Problem:     A `--format` template was validated inside the loop over records.
             An empty store means zero iterations, so an invalid template was
             never seen: `podbox images --format '{{.Nope}}'` printed nothing
             and exited 0, and a caller's typo read as an empty result set.
Premise:     ⭐ **Measured by running it**, not by reading it, on 2026-09-08.
             Three templates exited 0 against an empty store where all three are
             refusals: an unknown field, an unterminated opening delimiter,
             and docker's
             `table` prefix, which rendered the word `table` beside the values.
Approach:    One walk over the template, shared by a `check` that has no record
             in hand and a `render` that does, in
             `crates/podbox-cli/src/format.rs`. Both verbs call `check` before
             the store is opened, against a named list of their own fields.
             ⛔ The field-name list and the field builder are two places holding
             one value, so a test asserts they agree, which is the remedy
             `docs/conventions/forbidden-patterns.md` names for exactly that row.
Decision:    Refuse `table` rather than rendering it as literal text. It selects
             a column layout in docker and podbox does not have one, so printing
             it is output shaped like something podbox does not do.
Prove:       `podbox images --format '{{.Nope}}' >/dev/null 2>&1; test $? -eq 2`

**Done 2026-09-08.** Driven against an empty store: the unknown field, the
unterminated brace, the `table` prefix and a pipeline all exit 2 and name what
is wrong; `{{.Digest}}` exits 0. `inspect` reports a bad template before it
reports a missing image, because the template is the caller's own input.

---

### T-0808 Drive every row of the parity table through the shipped binary

Source:      T-0801; the door sweep of 2026-09-10
Category:    cli
Priority:    P1
Effort:      L
Status:      done 2026-09-21

Problem:     ⛔ **The table is 141 rows and nothing asserts the binary agrees
             with all of them.** `experiments/320-cli-contract.sh` drives a
             handful: the JSON parses, one `None` flag refuses with its own
             note, one unlisted flag is refused. The other rows are checked by
             a unit test that walks the table and calls the PARSER, which is
             the same code that reads the table, so the two cannot disagree
             even when the binary is wrong.
Premise:     Found by hand on 2026-09-10 and not by any check: `podbox create
             --no-steps` was accepted and acted on while `podbox system info`
             listed `--no-steps` under `run` and `exec` only. ⚠ The unit test
             could not have caught it, because it asks the table which flags a
             verb has and then asks the parser about those; a flag the parser
             accepts and the table does not mention is invisible from that
             direction. ⛔ **The check has to run the BINARY and start from the
             flag, not from the row.**
Approach:    One driver, over the table read out of the binary itself:
             1. for every row with a flag, invoke the verb with it and assert
                the outcome its status predicts -- `None` refuses with the
                row's own note and the flag-error code, `Native` and
                `Degraded` do not refuse for the flag's own sake, `Stub` is
                accepted and named in the banner;
             2. for every verb, invoke it with a flag no row names and assert
                the refusal names **the verb the caller typed**;
             3. the direction the unit test cannot go: for every flag the
                parser has an arm for, assert a row exists. ⚠ This one needs
                the arms enumerated from the source rather than from the
                table, which is the only way the two lists can be compared at
                all.
             ⚠ Rows that need an image or a container are driven against a
             store that has neither, so the assertion is about the FLAG's
             refusal and not the verb's; a row whose flag cannot be reached
             without state reports the third state rather than passing.
Decision:    Admit-first everywhere, in the same change as the driver. Every
             dash-arg passes `parity::admit_all` before any match arm runs, so
             an arm for a flag with no row is unreachable and a row with no arm
             ends at `parity::no_arm`, which the driver reports as a mismatch.
             Group subverbs carry the path the caller typed (`image prune`,
             `system abi`) and resolve their rows through `parity::rows_of`.
             Approach clause 3 closed structurally rather than by enumeration:
             with the pre-pass there is no second list to compare.
Prove:       `./experiments/325-parity-drive.sh`, which exits 0 only when every
             row of `podbox system info --format '{{json .Parity}}'` was driven
             or reported as unreachable here, and prints the count of each.

**Done, 2026-09-21.** The driver is green against the shipped binary: 160
rows, 199 driven, 0 mismatches, 0 unreachable here
(`experiments/results/parity-drive.txt`, with the conditions at its head).
The 153-row figure this record carried went stale at T-1302, which added
5 rows (the two tier rows, the two qemu-arg rows and `podvm`) without
re-driving, and T-1305 added 2 more (the two `--podbox-mem` rows) with the
re-drive above: 153 plus 5 plus 2 is the 160 the binary publishes.
`run -i` and `exec -i` both ran a self-pulled `alpine:3.20` payload with the
banner naming `-i`. Release build, `clippy --workspace --all-targets` with
`-D warnings`, and `cargo test -p podbox-cli -p podbox-probe -p podbox-extract`
(73, 65 and 46 passed) were all green in the same run. The first drive found
4 mismatches and all 4 were the driver's: group subverbs probed as top-level
verbs, and `exec` driven with the run-only `--pull`. Both fixed in the
script; the second run was green, and a third stayed green after the
`admit_all` pre-pass landed.

**Re-driven 2026-09-22 under [T-0209](image.md), which added the 3 login
flag rows.** 164 rows, 202 driven, 0 mismatches, 2 unreachable here.
The committed 160-row reading had gone stale by one row meanwhile:
[T-1004](packaging.md) added `version --verbose` without re-driving
(160 plus 1 is the 161 the tree carried; plus 3 is the 164 the binary
publishes). The 2 unreachable are `run -i` and `exec -i`, and they
disagree with the sentence above: in the re-drive container both exit
0 and the banner names no `-i`, with and without a pty, so the driver
reports them unreachable here rather than passed. No line of the
`run`/`exec` parsers prints `-i` on a plain run; the TTY theory was
probed and refuted (0 mentions both ways). Whether the 2026-09-21
reading matched a lane-only line or an older banner is unestablished,
and neither the driver criterion nor the banner moves in the T-0209
change that exposed it.

---

### T-0809 The spawn that fails with the wrong reason, and the one field that fixes it

Source:      `https://github.com/talaria0101/sandbox-insights`, its walls document, wall 2; T-0805 above
Category:    cli
Priority:    P1
Effort:      M
Status:      done 2026-09-21

Problem:     A payload inside podbox spawns a child, the spawn fails, and the
             message says the operation is not permitted. ⛔ **That one string
             covers at least two different refused calls**, and reading it as the
             wrong one has cost whole architectures.
Premise:     ⭐ **Measured, with the matrix committed, by somebody else.** Go's
             `os/exec` issues `setgroups` in the child whenever a credential is
             set, and one guard meant to suppress it does not take when its
             enabling field is set with no mappings beside it. On a runtime that
             denies `setgroups`, the spawn fails.
             ⚠ **The ambiguity is what makes it expensive.** A spawn that asks
             for namespace clone flags AND a credential fails with the identical
             string whichever call was refused: the clone runs first, the
             `setgroups` runs in the child before the `execve`, and both surface
             the same way. Reading it as "namespaces are denied" is the recorded
             mistake.
             ⭐ The remedy is one field and it changes nothing else. Dropping the
             credential entirely is not required and has other effects.
Approach:    podbox does not spawn this way: it is Rust and it declares its own
             syscalls. ⛔ **The payload does, and podbox is what the payload's
             operator reads.** So this is a diagnostic row and not a code change:
             T-0805's four-part message gains an entry mapping the observed
             string plus a denied `setgroups` leg to the mechanism and the
             one-field remedy.
             ⚠ The discriminator is already probed: [probe.md](probe.md)
             separates a refused clone from a refused `setgroups`, so podbox can
             say which wall the payload met rather than repeating the payload's
             own ambiguous string.
Decision:    Diagnose, never patch. ⛔ podbox does not rewrite a payload's
             process attributes: that is a behaviour change inside somebody
             else's program, which is a larger thing than
             [complete.md](complete.md) T-0407's trust change, and it is not this
             tool's to make. It names the wall and the remedy.
Prove:       `./experiments/151-spawn-ambiguity.sh` asserts podbox distinguishes a refused clone from a refused `setgroups` behind one identical payload message, and that the diagnostic names the field

**Done 2026-09-21.** The probe evidence names the wall per context
(`crates/podbox-probe/src/report.rs`): both legs with what this machine
measured, the payload shape that tells them apart, and
`Credential{NoSetGroups: true}` where setgroups met the denial. It prints
only where a leg is denied. Five unit tests pin the leg matrix, including
the unmeasured leg, which no spawn is attributed to.

No generic string matcher ships. The mapping keys on the probe legs, not
on substrings of payload output, so a matcher that guesses from text has
no caller. The table row from T-0805 stays as the static mapping.

Run 2026-09-21 in a disposable container (go 1.19.8, host podman 6.1.2):
`./experiments/151-spawn-ambiguity.sh` exits 0. Both payloads fail with
the byte-identical `fork/exec /bin/true: operation not permitted`, the
clone half attributed in the plain context and the setgroups half under
`unshare -Ur`, and the control payload exits 0 where setgroups is
allowed. `experiments/results/spawn-ambiguity.txt` carries the run.

---

### T-0810 `create` fails storing the memo: the container directory is never made

Source:      T-0708's drive, host podman 6.1.2
Category:    cli
Priority:    P1
Effort:      S
Status:      done 2026-09-22

Problem:     `podbox create --name e1 <image> ...` fails with `the ownership
             memo could not be stored: No such file or directory`, so no
             container with a record can be created and `inspect` has nothing
             to read. `run --detach` shares the rename and fails the same way.
Premise:     Read at file and line. `crates/podbox-cli/src/lifecycle.rs`
             `create` renames the ephemeral memo into
             `supervise::table::memo_path` beside the new record, and
             `crates/podbox-cli/src/run.rs` (detach) does the same. Neither
             the rename's parent (`table::dir`, a pure join) nor
             `supervise::create` (a table-JSON write only) makes the
             directory. Foreground `run` never renames, which is why every
             drive to date stayed green past it.
Approach:    Make the directory where the container is created, not where
             the memo is renamed: `supervise::create` owns the container's
             directories (`lock_path`, `log_path` and `control_path` all
             live under `dir`), so it ensures `dir` after the table write.
             Both rename call sites ride it with no change.
Decision:    A failed directory fails the creation loudly, beside the
             rename's own failure. A container with a record and no memo
             answers `stat` with the real uid and contradicts every
             foreground run, which is the inconsistency T-0710 refuses.
Prove:       `podbox create --name e1 public.ecr.aws/docker/library/alpine:3.20 true && podbox rm e1`

**Done 2026-09-22.** `supervise::create` ensures `containers/<id>/` after the
table write (`crates/podbox-supervise/src/lib.rs`). Both memo renames ride it
with no change (`lifecycle.rs` `create`, `run.rs` detach). A directory the
kernel refuses fails the creation loudly. `creating_a_container_makes_its_directory`
creates a container, asserts the directory, and renames a file beside the
record. The entry Prove ran verbatim on host podman 6.1.2 with the shipped
binary (`create --name e1 ... true` then `rm e1`, both green). Row E1 of
the T-0708 drive covers the same create with a payload that writes the
memo: the `mknod` reads back through `inspect` at 1, and `rm -f` removes
the container.
A read-only audit over every other beside-the-record write site (log, lock,
control socket, memo opens in launcher and exec) names an independent
guarantee for each (the launcher's own directory creation, the lock's parent
creation, the memo helper's parent creation). No site writes beside a record
without a directory behind it.

---

### T-1319 `inspect` prints exactly one document for any reference

Source:      issue 21, client beta testing 2026-09-22 (container
             inspect emits the record plus a spurious `[]`);
             `crates/podbox-cli/src/images.rs` (the `find_one`, then
             `inspect_container`, then unconditional array print)
Category:    cli
Priority:    P1
Effort:      S
Status:      done 2026-09-23

Problem:     `podbox inspect <container>` prints two JSON documents: the
             container record (printed inside `inspect_container`) and a
             trailing `[]` (the empty image `records` vec, printed
             unconditionally after). An automated caller parsing stdout
             fails; `jq` cannot consume it. Image references print one
             array correctly, and `--format` is unaffected, so only the
             container path is broken.
Premise:     Measured by the reporter (`json.load` fails with "Extra
             data", `raw_decode` finds two documents) and confirmed
             against the loop above: the container branch prints inside
             the call and returns `Some(0)`, leaving `records` empty for
             the final print.
Approach:    Do not print the trailing image array where the reference
             resolved to a container (or collect the rendered container
             document into `records`, one path, one print). Out of
             scope: changing either document's fields, touching image
             inspect.
Decision:    Skip the second print on the container path: the smaller
             diff, and the image path keeps its exact bytes.
Prove:       `podbox inspect <container>` piped to `python3 -c
             'json.load(sys.stdin)'` and to `jq` both succeed with one
             document; image inspect output is byte-identical before and
             after. Close issue 21 with a comment showing the parse and
             the single-print path as the guard that stops recurrence.

**Done 2026-09-23.** Skip the second print on the container path, as
decided. `inspect` (`crates/podbox-cli/src/images.rs`) tracks whether any
reference resolved to a container; where the image `records` vec is empty
and a container document is already on stdout, the trailing array print is
skipped and the verb returns the container's code. The image path is byte
for byte what it was: the array prints wherever a record resolved,
including the empty store (`[]` with no container involved).

Driven on the shipped binary in the lane (`rust:1.98.1-bookworm` through
host podman 6.1.2), against `public.ecr.aws/docker/library/alpine:3.20`:

```
$ podbox create --name green1 ... true && podbox inspect green1
[{"Command":"true",...,"Name":"green1",...}]
$ podbox inspect green1 | python3 -c 'json.load(sys.stdin)' && echo PARSES
ONE-DOCUMENT len=1
$ podbox inspect public.ecr.aws/docker/library/alpine:3.20
[{"Architecture":"amd64",...}]      # one array, 14 keys, as before
```

Before the fix the same container printed the record plus a trailing
`[]`, and `json.load` failed with `Extra data: line 2 column 1`. The
guard that stops recurrence is the single-print path: there is now one
place that decides what `inspect` prints without `--format`, and a second
document cannot be added without passing it. `--format` is untouched: it
returns before the array either way.

The `jq` half ran beside it on a clean store in the same lane
(`rust:1.98.1-bookworm` through host podman 6.1.2):
`create --name jq1`, then `inspect jq1 | jq .` (JQ-CONTAINER-OK) and
`inspect <image> | jq .` (JQ-IMAGE-OK), one line each, both rc=0.
(The first attempt reused the T-1315 job's store after its byte flip,
so `create` failed there for the tampered-store reason; the rerun used a
fresh store.)

---

### T-1323 `cp` reaches an image's extracted rootfs, and copies directories with the same checks

Source:      issues 14 and 16, client beta testing 2026-09-22 (`cp`
             gated on container records; no recursive copy);
             `crates/podbox-cli/src/parity.rs` (the `cp` row: one file
             at a time, gated through the containment check)
Category:    cli
Priority:    P2
Effort:      M
Status:      done 2026-09-23

Problem:     `cp` only addresses `container:path`, and `run` leaves no
             record, so on a host where the payload cannot launch there
             is no checked way to stage or retrieve files: the only
             interface with containment checks is gated on a started
             container, which restricted hosts cannot always provide.
             Separately, `cp` copies one file at a time; a recursive
             option with the same checks has no entry.
Premise:     Measured by the reporter: `cp alpine:latest:/etc/os-release`
             refuses ("no such container"), `create`+`cp` works only
             where `start` works, and raw store writes bypass every
             check. The containment machinery (`podbox-extract` safety
             module) and the extract walker already exist to make both
             halves safe.
Approach:    Accept `image:path` (the extracted rootfs in the store) as
             well as `container:path`, reusing the containment-checked
             path unchanged; add a recursive option over the same walker
             with the same checks. Out of scope: a new stage/unstage
             verb pair (rejected: `image:path` covers the workflow with
             no new verb and no parity debt), documenting raw store
             surgery (never an answer).
Decision:    Extend `cp`, do not add verbs. One verb, one checker, two
             new addressings.
Prove:       `podbox cp image:/etc/os-release` on a host where
             payloads cannot launch retrieves bytes, and `podbox cp` of
             a directory tree round-trips with containment intact (a
             hostile symlink inside the tree refuses rather than
             escapes); `podbox cp --help` documents both. Close issues 14
             and 16 (cp half) with comments showing the commands and the
             shared containment check as the guard that stops recurrence.

**Done, 2026-09-23.** `cp` addresses `container:path` first and
`image:path` second (last-colon split, so a tag survives), extracting
on demand under the image hold; `-r` copies a tree over the same
`contain::within` gate in both directions. One deliberate deviation
from the Prove letter: a symlink inside the tree replicates verbatim,
it does not refuse. The first lane run proved the letter unworkable:
`cp -r alpine:3.20:/etc` refused on `/etc/mtab -> /proc/mounts`, and
every real rootfs carries such absolute links (TODO/extract.md
T-0305 settled this for extraction: absolute targets replicate, never
resolve). What refuses instead is a destination through a
pre-existing symlink, which would land where the link points.
Lane prove `.tmp/pb-w23c-prove.sh`, verdict `fail=0`:
`cp IMAGE:/etc/os-release` retrieves Alpine bytes with pull only (no
extract, no container); `cp -r IMAGE:/etc` lands 45 files plus 4
symlinks with `mtab` a link to `/proc/mounts` verbatim against the
rootfs; an imported absolute link replicates with its target intact
and stays a link; a planted destination link refuses naming the
symlink with nothing through it; the tree round-trips into a created
container byte-identical (`cmp` clean); a directory without `-r`
names `-r`; `--help` documents both addressings. Six unit tests by
exact name, `cargo clippy --workspace --all-targets -- -D warnings`
clean, `cargo test --workspace` exit 0. The guards that stop
recurrence are the shared `within` gate on every rootfs-side path,
the no-follow walk with verbatim link replication, and the
write-through-symlink refusal on every destination.

---

### T-1330 Bundled short flags parse as docker reads them

Source:      issue 27, client beta testing 2026-09-22 (`ps -aq`,
             `run -it` refused); `crates/podbox-cli/src/parity.rs`
             (the `-a`/`-q` and `-i`/`-t` rows, never mentioning
             bundling)
Category:    cli
Priority:    P2
Effort:      S
Status:      done 2026-09-23

Problem:     Combined short flags are refused: `ps -aq` and `run -it`
             answer `unknown option` with 125. Docker callers write
             clusters constantly; the parity table lists the member flags
             separately and never mentions bundling, so the refusal reads
             as a missing flag rather than a missing spelling.
Premise:     Measured by the reporter on the beta.3 asset, both forms,
             both 125. The parser has no cluster path (each flag is
             matched whole).
Approach:    Parse a cluster of value-less short flags by docker's own
             rule (a cluster containing a flag that takes a value
             consumes the rest as its value; an unknown member refuses
             naming the member). One parser change, every verb inherits
             it. Out of scope: new flags, changing any refusal code.
Decision:    Docker's cluster rule exactly, not a subset. A subset would
             move the surprise rather than remove it.
Prove:       `ps -aq`, `run -it` (and a cluster with an unknown member,
             refusing by name) behave as docker's rule says across three
             verbs; the existing flag tests still pass unchanged. Close
             issue 27 (bundling third) with a comment showing the runs
             and the single parser path as the guard that stops
             recurrence.

**Done, 2026-09-23.** One rule, `parity::expand`, called at the top of
every flag-taking verb before admission: value-less shorts split
(`ps -aq`, `run -it`), a value-taking member consumes the rest
(`stop -t5`, `-eFOO=bar`, `-f{{.Id}}`), an unknown member refuses
naming the member. The value-taking shorts live once in
`CLUSTER_VALUES`, held to the table by unit test; verbs that stop at
the image (`run`, `exec`, in `CLUSTER_BOUNDARY`) expand only before
it, so `run IMG -la` still reaches the payload. The first lane run
caught the boundary missing (payload clusters refused) and the
opaque value rest (`-eFOO=bar` passed through); both fixed and
pinned by unit test. Prune's local `a`/`f` splitter is gone, replaced
by the shared rule, and check 26 expands the same way from the same
lists, with its own plant case. The work also found one dead arm:
`system info` accepted `-f` with no table row; the row carries both
spellings now. Lane prove `.tmp/pb-w30-prove.sh`, verdict `fail=0`:
cluster units by exact name, the pre-existing flag tests unchanged,
clippy clean, `ps -aq`, `run -it`, `inspect -f` and `ls -la` through
a payload green, `-aZ` refusing 125 naming `-Z`. The guard is the
single expansion path plus check 26's mirror.

---

### T-1331 `--filter`, `restart` and `pull -a/-q` answer the docker idiom

Source:      issue 27, client beta testing 2026-09-22 (three refused
             idioms with no entry);
             `crates/podbox-cli/src/parity.rs` (the `None` rows for
             `--filter`, `restart`, `pull -a`)
Category:    cli
Priority:    P2
Effort:      M
Status:      done 2026-09-23

Problem:     Three docker idioms break with 125 and have no work-order
             entry: `--filter` on `images` and `ps` (every `docker ps
             --filter name=x` one-liner), `restart` ("`stop` then `start`
             is the same thing" is not the same verb), `pull
             -a/--all-tags` and `pull -q`. The table is honest; the work
             order does not know these are missing.
Premise:     Measured by the reporter on the beta.3 asset, each form
             with its refusal text. Bundled flags are the sibling entry;
             these are per-verb gaps, not parser gaps.
Approach:    Implement the three behind the table rows that already name
             them: `--filter` as a caller-side predicate over listed
             records (no new query language), `restart` as stop-then-
             start in one verb reporting which half failed, `pull -a` as
             all offered tags and `-q` as quiet output. Out of scope:
             server-side filter syntax beyond `name=` and label
             equality (named if dropped), restart policies (a different
             gap).
Decision:    Caller-side filter, composite restart, per-tag pull. Each
             reuses a path that already exists rather than forking one. A
             tag with no manifest for this platform is named and skipped
             (typed `NoPlatform`, not a string match); any other failing
             tag stops the run, and an offer with nothing for this
             platform at all is an error.
Prove:       `ps --filter name=x` selects, `restart` stops and starts
             with both halves named, `pull -a` fetches every offered tag
             and `-q` stays quiet; the three parity rows move status with
             the behavior. Close issue 27 (idiom thirds) with a comment
             showing the runs and the row-status moves as the guard that
             stops recurrence.

**Done, 2026-09-23.** `--filter` is a caller-side predicate over listed
records (`name=` substring, `label=` equality, any other key refused
naming `name=` and `label=`), `restart` is stop-then-start in one verb
naming which half failed, `pull -a` pulls every offered tag of a bare
repository (a tag on it is refused) and `-q` sinks progress output. The
first lane run caught the real case the entry had not named:
`hello-world` offers windows-only `nanoserver` tags, so stop-on-first-
failure made `pull -a` unusable against any multi-OS repository. A tag
with no manifest for this platform is now named and skipped on a typed
`NoPlatform` variant (not a string match), any other failing tag still
stops the run, and an offer with nothing for this platform is an error
rather than an empty success. A single-tag pull of such a tag still
fails with the same text and code. Lane prove `.tmp/pb-w31-prove.sh`,
verdict `fail=0`: filter units by exact name, suites, clippy clean,
`ps --filter` selects/empty/refused-key, `images --filter` selects,
`restart` composites with the stop half named, `pull -a` rc 0 with four
`nanoserver` skip lines and `hello-world:latest` plus `:linux` in the
store, explicit-tag `-a` refused, `-q` byte-quiet, the three parity rows
moved. The guard is the three moved table rows plus the prove script.

---

### T-1332 `podbox man` generates the manual from the binary, pager-aware

Source:      operator order 2026-09-23 (drift-free human/AI manual;
             docs need not carry help text); `crates/podbox-cli/src`
             (per-verb usage strings, the parity table as data)
Category:    cli
Priority:    P1
Effort:      M
Status:      done 2026-09-23

Problem:     Help text lives in docs or not at all, so every new flag is
             a chance for drift: the manual says what the binary said on
             the day somebody copied it. Nothing generates a readable
             manual from the binary for humans (`less`) and agents
             (plain text) alike.
Premise:     Audited: no `man` surface exists in `crates/podbox-cli/src`
             (only shell completion in `complete.rs`); per-verb usage
             strings and the parity table already carry the content a
             generator needs.
Approach:    A `man` verb rendering every verb, flag and parity note
             from the binary's own data (usage strings plus
             `parity::TABLE`), plain text to stdout, shaped like
             `wsl-toolkit man --no-pager`: version header, command list
             with one-line summaries, a Global section, then a COMMAND
             REFERENCE with one section per command carrying its
             synopsis and flags. `podbox man [verb]` pages through
             `$PAGER` (`less` where present), `--no-pager` (and non-tty
             stdout) prints without paging. Progress and diagnostics
             stay on stderr; stdout carries the answer alone. New verbs
             and flags appear by construction, never by edit. The
             generated output carries no emoji and no markers (the
             binary-wide scrub is the sibling entry below). Out of
             scope: troff/`man(1)` integration (no new dependency, no
             roff to drift either), shell completions (unchanged),
             `--json` (verbs that have structured answers keep it; the
             manual is text).
Decision:    Generated plain text, pager-aware, no troff. The audience
             is a tired reader and an agent: both read text, neither
             needs typesetting.
Prove:       A script diffs `podbox man` output against every `--help`
             output plus the parity rows and fails on any verb or flag
             the manual lacks; `podbox man --no-pager` and piped `podbox
             man` print identical bytes with no pager spawned. The
             completeness script is the drift guard that stops
             recurrence.

**Done, 2026-09-23.** New `man` verb in `crates/podbox-cli/src/man.rs`:
version header, command list with one-line summaries (the first
sentence of each implemented verb row's note), a Global section whose
exit-code numbers render from the exit-code constants, then a command
reference with one section per implemented verb carrying that verb's
`--help` bytes captured by re-executing the binary, so a changed usage
string arrives with no edit here. The verb set is the table's own
implemented rows minus the argv0 aliases; group subverbs (`prune`,
`install-names`, `abi`) render through their group path, pinned
against `rows_of` by unit test. `man [verb]` renders one section;
`--no-pager` and a non-terminal stdout print the same bytes with no
pager spawned, and an unstartable pager falls back to stdout. Lane
prove `.tmp/pb-w32-prove.sh`, verdict `fail=0`: seven man units by
exact name, suites, clippy, every implemented verb's `--help` exiting
0 with non-empty bytes contained in the manual, one-verb rendering,
byte-identity between `--no-pager` and piped runs, the refusal codes,
the moved `man` rows. One isolation gap is recorded, not hidden: the
bogus-PAGER run asserts identical bytes and rc 0, which the stdout
fallback also produces, so a follow-up prove should capture that run's
stderr separately and assert it empty to isolate non-spawning. The
prove caught two defects on the way: `restart --help` exited 125
because T-1331's verb lacked its `-h, --help` row (added here), and a
first pass of this change forgot the `--no-pager` row the same way
(caught in review before the green run). The manual carries the usage
strings verbatim, glyphs included: the ASCII scrub is the sibling
entry T-1336, whose guard covers this output. The guard is the
completeness script plus the pinned verb set.

**Partial, 2026-09-25.** Issue 50 reopens this entry on the
recorded isolation gap: the bogus-PAGER run asserts identical
bytes and rc 0, which the stdout fallback also produces, so the
test cannot tell non-spawning from a silent fallback. Capture the
bogus-PAGER run's stderr separately and assert it empty, keeping
the byte-identity assertion beside it. Fix area is the T-1332
prove script with `crates/podbox-cli/src/man.rs`. Risk if wrong
is a regression that silently falls back to stdout and still
passes. Prove is the extended bogus-PAGER run with stderr
asserted empty.

**Done 2026-09-26.** No product change: the behavior was already
right, the isolation was what was missing. Lane drive
(`rust:1.98.1-bookworm` job, musl debug binary) with
`PAGER=/bogus-that-does-not-exist`: piped stdout exits 0 with
empty stderr (`PIPED-STDERR-EMPTY`, so no spawn was attempted)
beside byte-identical stdout to `--no-pager` (both 39,238 bytes,
`BYTES-IDENTICAL`); under a pty (`script`) the same bogus pager
is loud on the terminal (`podbox man:
/bogus-that-does-not-exist could not start (No such file or
directory (os error 2)), printing without a pager`,
`TTY-FALLBACK-LOUD`), which is what makes the empty stderr above
meaningful rather than vacuous. A regression that spawns where it
should not now fails the stderr assertion; a regression that
silences the fallback now fails the pty assertion.

---

### T-1336 CLI output is plain ASCII: no emoji, no markers, on every path

Source:      operator order 2026-09-23 (no place for either in a CLI
             tool, let alone without a terminal or pty);
             `crates/podbox-cli/src` (usage strings, error messages and
             parity notes carry marker glyphs on dozens of paths:
             `complete.rs:103`, `exec.rs:252`, `main.rs:372`,
             `names.rs:259`, `parity.rs:103`, `run.rs:26`, and the
             per-verb usage blocks behind every `--help`)
Category:    cli
Priority:    P1
Effort:      M
Status:      done 2026-09-23

Problem:     The binary's user-facing text carries ⛔/⚠/⭐ glyphs on
             dozens of paths: per-verb usage blocks (printed by every
             `--help`), error messages (`complete`, `exec`, `probe`
             cache, `install-names`), and parity notes (the
             machine-readable contract). On a constrained host with no
             terminal or pty these are unrenderable bytes in an
             automated caller's stream, and they force every downstream
             parser to handle codepoints that carry no meaning.
Premise:     Measured on this tree: non-ASCII bytes outside comments
             across `crates/podbox-cli/src` (usage blocks, emitted
             errors, table notes); comments and doc-comments that are
             never printed are not in scope and stay.
Approach:    Scrub every printed string to ASCII, replacing each glyph
             with the word it stands for (`refused:`, `note:`), usage
             blocks, errors, banner lines and parity notes alike; keep
             source comments as they are. Add the guard that stops
             recurrence: a test (or gate check) failing any non-ASCII
             byte in `--help` output per verb, the `man` output, and the
             error/catalog paths. Out of scope: docs and comments (the
             markers check owns those), changing any message's meaning
             (glyphs become words, nothing is reworded).
Decision:    Words, not glyphs, everywhere printed. The manual entry
             (T-1332) inherits ASCII output from this one.
Prove:       `./experiments/352-ascii-output.sh` exits 0, with the
             existing suite green; the guard runs in the gate so a new
             glyph fails before it ships.

**Done 2026-09-23.** Words, not glyphs, as decided: every printed
string across `crates/podbox-cli/src`, `crates/podbox-probe/src`,
`crates/podbox-image/src/tls.rs`, `crates/podbox-enter/src/abi.rs`
and `crates/podbox-complete/src` carries bytes `0x00`-`0x7F` only.
U+26D4 reads `refused:`, U+26A0 reads `note:`, U+2B50 is deleted;
source comments stay as they are. Twenty-two files, 84 insertions
and 82 deletions, nothing reworded.

`experiments/352-ascii-output.sh` drives the binary rather than
grepping the source: every implemented verb's `--help` (verbs read
out of the binary's own parity table, never listed), `man` under
both pagings with stdout and stderr separated, and a catalog of
five error paths. `experiments/results/ascii-output.txt` carries
the run: 69 `ASCII-OK` lines, zero failures, verdict `fail=0`.

Nine unit tests hold the constants the script drives (usage per
module, every parity note, the whole manual through the stub):
`cargo test -p podbox-cli ascii`, 9 passed. Clippy `-D warnings`
clean in the same lane run. The gate guard is check 28 in
`scripts/check-todo.py` (any non-ASCII byte in a printed string
fails, comments exempt) with its plant case in `scripts/plant.sh`:
a glyph planted in `run.rs` goes red naming `U+26D4` and T-1336.
The guard is the check plus the 352 experiment: a new glyph fails
the gate before it ships and fails the binary drive beside it.

---

### T-1337 Doctor, disk usage, and log tail for operators

Source:      issue 38, beta.7 drive 2026-09-25 (no `doctor`, no `df`,
             `logs` without `--tail`); `references/carlbomsdata__winquick`
             `tree/src/main.rs:50` with `tree/src/facts.rs:125-180` (the shape
             to copy, not the content)
Category:    cli
Priority:    P2
Effort:      M
Status:      open

Problem:     Three operator gaps found while driving beta.7. `podbox
             probe` prints the measurement half (legs, verdicts,
             refusals) and never the setup half: a QEMU too old for
             migration, firmware present or absent, helper tools
             present, image installed, disk free against the ceiling.
             No `system df` sums stored, extracted and reclaimable
             bytes, which operators on small stores need beside the
             prune path. `logs -f` replays from the start on every
             reconnect, because there is no `--tail N` (and later no
             `--since`).
Premise:     Read on the lane-built binary 2026-09-25
             (`experiments/results/triage-353.txt` clauses
             `38-system-help` and `38-logs-help`): `system --help`
             lists no doctor and no df, `logs --help` lists only
             `-f|--follow`, and `probe --json` already carries every
             leg a doctor would consume. `TODO/supervise.md:272`
             already documents that `logs` has no `--tail`.
Approach:    Three small parity additions, each with its row and its
             Prove through the shipped binary, and no spec change
             beyond one row each. `podbox doctor` (or `probe --fix`)
             reuses the machine legs with the T-1301 checks and prints
             one fix line per missing piece, with the third-state
             exits (0 ok, 1 problem, 2 could not run). `podbox system
             df` (with image du rows) reports stored, extracted and
             reclaimable bytes with the container gate applied, and
             never exits non-zero for accounting alone. `logs --tail
             N` (with `--since` as a second step) reads the same
             captured file T-0605 owns, bounded like `-f`, with
             byte-identical default output unchanged.
Decision:    One entry, three verbs in order doctor, df, tail, each
             shippable alone. The doctor copies the reference's shape
             (check lines with fix lines) and none of its content.
Out of scope: a log stream split (T-0605), a network check needing
             a registry, any new exit-code meaning.
Prove:       `podbox doctor` on a kvm-less host exits 1 naming the
             missing legs with one fix line each; `podbox system df`
             sums to the store's own accounting; `podbox logs --tail 5`
             prints the last five lines and `podbox logs` prints all
             lines byte-identical to before.

**Tail, 2026-09-26.** The third verb landed first: `logs --tail N`
(`--tail=N`, both in any position) prints the last N lines of the
same captured file, byte-identical default untouched, and `-f
--tail N` prints the tail then follows from the chained offset
instead of replaying the file. `podbox-supervise::tail_offset`
(pure, unit-pinned) with `follow_from` chaining the read length
past the printed slice, so no line is lost or repeated. A
non-count is a flag error at 125 naming the value. Prove through
the shipped binary (`experiments/364-qol.sh` tail clauses, exit
0, `experiments/results/qol.txt`): last five of ten exact,
plain logs byte-identical to a wide tail, `--tail 0` empty at
exit 0, non-count refused, `-f --tail 3` following to the end.
Doctor and df stay open below.

**Df, 2026-09-26.** The second verb landed: `podbox system df`
prints one row per image (repository, tag with `<none>` where
none, twelve-digit IMAGE ID, stored blob bytes beside extracted
rootfs bytes), the image and container counts, stored and
extracted totals with each blob billed once, and Reclaimable:
what `image prune` would free. The number is prune's own gate
chain read-only (`prune_candidates(false)`, the `referencing`
record gate, the `in_use` hold check) with
`Store::reclaimable_bytes` standing in for `delete`, so on a
quiet store it is the exact string prune prints. Accounting
never fails the verb: `Store::blob_bytes` counts a missing blob
as 0, `space::dir_bytes` walks without following symlinks,
dedupes hardlinked inodes and counts an unreadable entry as 0.
Only a gate failure (unopenable store, unanswerable lock, failed
listing) exits non-zero. Parity: `df` Native row with its `-h`
row, the `system` note no longer claims df missing (check 27
would refuse the stale claim), `rows_of` maps `system df`, the
`check-todo.py` mirror maps it too, `man` routes its help.
Prove through the shipped binary (`experiments/364-qol.sh`
df clauses, exit 0, `experiments/results/qol.txt`): debian rows
26.9 MiB stored beside 74.6 MiB rootfs, totals present with the
image count identical before and after, a digest-pulled alpine
dangling (`<none>`) reclaimable at 3.7 MiB with prune freeing
the exact same string and df reading 0 B after, `--bogus` at
125, `--help` at 0. Unit-pinned: `reclaimable_bytes` counts an
unshared blob once and only where no survivor needs it,
`dir_bytes` counts once and never follows, df help and bad
flags need no store, short IDs are twelve hex digits.
Two findings ride with it. First, the drive refuted this
entry's own assumption that an unnamed `import` dangles: it
records `imported:latest` through `Reference::parse`'s central
default (`layout.rs:378-384`, T-1320 Done), so the IMPORT_USAGE
sentence claiming "no tag" was corrected in place; the honest
dangling path is a digest pull (`reference.rs`: a digest with
no tag keeps tag None). Second, lane notes: bootstrap's docker
daemon does not come up in job containers (364 needs no
docker, so the job proceeds past it with zig present), one
release build failed after printing Finished with no error
captured, and `build-interpose.sh` failed its musl half once
with no error captured; each went green on the identical tree
on retry, and all three are recorded here rather than
explained away. Doctor stays open
below.

⚠ Citation correction, read at file and line 2026-09-26: the
doctor shape lives at `tree/src/main.rs` (help text naming
`winquick doctor`) and `tree/src/facts.rs:125-180` (`Status`,
`Check`, `Doctor` with the ordered fix list, builder with
ok/note/fail, health agreeing with the problem list) - there is
no `tree/src/bin/` directory. The tree was read at one pass
([history/2026-09-11-reference-sweep.md](../docs/history/2026-09-11-reference-sweep.md)),
so only the shape travels, none of the content.
