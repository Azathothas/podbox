Area: cli.

Record semantics: [task rules](RULES.md#5-entry-closure).


`crates/podvm`. `TOOL.md` section 6.8 and section 6.9.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ **An agent that knows docker must need zero new knowledge.** That is the
product requirement the whole specification exists for. Same verbs, same flags,
same exit codes; where a flag cannot be honoured it is accepted and reported in
the banner, and where a flag **requests isolation** it fails loudly with a
named reason.

# T-0801: The verb and flag parity table

**Source:** `TOOL.md` section 6.8
**Category:** cli
**Milestone:** none
**Priority:** P0
**Effort:** L
**Status:** done

## Problem

A tool that needs its user to learn its differences has not
replaced anything. Thousands of agents reach for `docker` because
it is the only container language they know.

## Premise

Read. `TOOL.md` section 6.8 is the table, in full, with a status per verb
and per flag. Four statuses: **Native** (real semantics),
**Degraded** (works, documented difference, stated in the banner),
**Stub** (accepted, no-op, listed in the banner), **None** (fails
with a named reason).

## Approach

Implement the table as data, not as a match arm per flag, so that
`podvm system info` can print it and a test can assert every row
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

## Decision

A table plus a generated parser, over a hand-written one. The table
is what section 6.8 already is, and generating from it makes "the flag
exists and is unlisted" impossible.

## Prove

`./experiments/320-cli-contract.sh` clauses 1 to 3, whose first clause is `podvm system info --format '{{json .Parity}}' | jq -e 'length >= 60 and all(.status | IN("Native","Degraded","Stub","None"))'`

## Correction

**Amended, 2026-09-10, by the door sweep after T-0412.** ⛔ **One verb was
served by another verb's parser and nothing said so, in either direction.**
`create` is parsed by `run::parse`, which called `admit("run", …)` with the verb
written in rather than passed. Two consequences, and each is the class this
entry exists to make impossible:

- ⛔ `podvm create --no-steps` was **accepted and acted on** while
  `podvm system info` listed `--no-steps` under `run` and `exec` only. The
  table under-described the binary, which is "the flag exists and is unlisted"
  arriving through the door the entry did not enumerate.
- ⛔ `podvm create --no-such-flag` answered **`podvm run: unknown option`**
  and printed `usage: podvm run`, naming a verb the caller had not typed, and
  then sent them to "every flag this verb takes" -- a row set that does not
  exist.

`parity::rows_of` is the one place that says where a verb's rows live, so the
28 rows stay a single copy; `run::usage(verb)` builds the first line from the
verb the caller typed; and the `create` row and the refusal message both name
the borrowing out loud. ⚠ Threading the verb was not cosmetic: `admit` looks the
flag up, so naming the caller's verb without `rows_of` would have made `create`
refuse every flag it accepts. Six functions in `run.rs` took the verb --
`parse`, `prepare`, `acquire`, `extract_now` and `config_of` among them --
because each wrote `podvm run:` into a message `create` and `exec` can reach.

Prove: `a_verb_served_by_another_parser_resolves_that_verbs_rows_and_says_so`
asserts both halves, and driven: `podvm create --no-such-flag` now names
`create`, `podvm create --no-steps` is still accepted, and the table is 141
rows as before, because the fix added none.

## Done

**Done, 2026-09-09.** `crates/podvm/src/parity.rs` is the table,
`crates/podvm/src/system.rs` is `podvm system info`, and
`experiments/320-cli-contract.sh` drives all three of the contracts below
against the shipped binary.

| | |
| --- | --- |
| rows | 287 on 2026-11-11 (`awk '/^pub const TABLE/,/^\];/' crates/podvm/src/parity.rs \| grep -c '^\s*Row {'`). ⚠ 131 when this closed and 265 on 2026-10-01: the table grows with the flags, so a count here is a reading with its date |
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
`podvm run --name c1` exits 2 with "`--name` is in the parity table with status
None: a name identifies a container, and podvm has none until M4", and
`podvm ps` exits 125 with that verb's row. Clause 3 asserts the message a verb
prints is byte-for-byte the note the table published.

⛔ **`--format` grew exactly one function, `json`.** `{{json .Parity}}` is
docker's own spelling and it is T-0801's `Prove`, so refusing it would have made
the acceptance unwritable. It is not a general function call: a verb declares
which of its fields are already documents, `{{json .X}}` on a plain string
quotes and escapes it, and every other function is still refused by name. ⚠ The
alternative, guessing from the value's first byte, makes a string that happens
to begin with `[` come out unquoted, which is a wrong answer that parses.

⚠ **A `None` row is not a placeholder.** It is the sentence a caller gets
instead of a flag being ignored, so deleting one makes podvm quieter and less
honest rather than smaller. A test asserts every one of them carries a reason
longer than a milestone number, which caught eleven rows whose whole note was
"the lifecycle is M4".

**Partial, 2026-09-25.** Issue 60 reopens this entry on its own
invariant: a flag with no row is a bug in the table, and 46 `run`
flags (`--read-only`, `--mount`, `--env-file`, `--label`,
`--init` among them) plus 10 verbs (`manifest`, `service`,
`secret`, `trust`, `config`, `node`, `plugin`, `stack`,
`checkpoint`, `scan`) have no row at all. Measured on the lane
2026-09-25 (`experiments/results/triage-353.txt` clauses
`60-read-only`, `60-mount`, `60-env-file`, `60-manifest`,
`60-service`): each flag answers `unknown option` with no reason
at 125, each verb exits 1 on the neither-podvm-nor-docker arm
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
`crates/podvm/src/parity.rs` with the `run` and `exec`
parsers for promoted flags. Risk if wrong is an operator
trusting an enumerable table that omits the docker surface they
script against. Prove is the curated flag list from the issue
driven row by row through the shipped binary with every refusal
naming its reason, beside the T-1325 check extension below.

**Done, 2026-09-25.** Every one of the issue's 46 flags and 10
verbs has a row, in `crates/podvm/src/parity.rs` (220 rows,
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
clause-1 40/40 refused with status None at 125, clause-2 10/10
verbs refused at 125 with their notes, `--env-file` end to end
(`from-file` on payload stdout, a later `-e` winning, the `=`
form equal, a bad line and a missing file at 125), the stub
trio admitted at 0 with `--strict` refusing at 125 naming Stub,
`--log-driver=json-file` accepted in both forms with `syslog`
refused at 125 naming the value, `create` inheriting all of it.
Report in `experiments/results/parity-curated.txt`. In-suite:
`issue_60_curated_surface_stays_covered`,
`env_file_loads_and_stubs_parse`,
`log_driver_takes_json_file_and_nothing_else`, podvm 139
passed, 0 failed. Guards are check 27's curated arm and the
in-suite test beside it.

# T-0802: docker's exit codes, unaltered

**Source:** `TOOL.md` section 6.8
**Category:** cli
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

An automated caller reads the exit code first. A runtime that
returns its own codes breaks every script that branches on
docker's, and a runtime that improves a payload's code lies in the
field read first.

## Premise

Read. docker's convention: the payload's own status for `run`, 125
when the runtime itself could not run the command, 126 when the
command was found and could not be invoked, 127 when it was not
found.

## Approach

Return the payload's status verbatim from `run`, `start` in the
attached case, and `wait`. Use 125, 126 and 127 for the three
runtime-side cases and nothing else.
⛔ Never translate a payload's non-zero status into zero because a
fixup succeeded, and never into non-zero because a fixup warned.
That rule is what T-0409 depends on.

## Decision

Match docker rather than define a clearer scheme. Parity is the
product.

## Prove

`podvm run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'exit 42'; test $? -eq 42 && podvm run --rm public.ecr.aws/docker/library/alpine:3.20 /nonexistent; test $? -eq 127`

## Done

**Done 2026-09-09, and the entry's own description of docker's convention was
incomplete.** `experiments/330-exit-codes.sh` runs the same case through both
binaries on one host and compares the numbers, which is the only way this can be
asserted: docker's contract is not written anywhere podvm can read, it is what
the binary does.

⭐ **THE DISCRIMINATOR IS NOT THE ONE THE ENTRY DESCRIBES, AND IT IS THE
FINDING.** docker exits **125** for anything its FLAG PARSER refuses and **1**
for anything the verb refuses afterwards. Measured on docker 29.3.1:

| case | docker | what podvm did before |
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
`podvm run alpine /etc/passwd` said 127 where docker says 126, and a script
branching on 127 retries with another path. The split is on the **errno**:
`EPERM`, `ENOEXEC`, `EACCES`, `EISDIR` and `ETXTBSY` are 126; `ENOENT` and
anything about resolving the path are 127.

**Prune and rmi on a held image, decided 2026-10-01.** No source defect:
`store.delete` with `Held::Skip` returns `Ok` with the image in `skipped`,
which `prune` reports as exit 0 naming `skipped:`; `store.remove`
(`Held::Refuse`) errors `in use`, which `rmi` reports as 125. The script
(`experiments/160-store-gc.sh:138-139`), the source
(`crates/podvm/src/images/login_extract.rs:222-247`), and
`experiments/results/store-gc.txt` (one `skipped:` line) agree. Pinned by
`rmi_refuses_a_held_image` and `prune_skips_a_held_image` in
`crates/podvm/src/images.rs`.

⭐ **The codes now live in ONE file**, `crates/podvm-probe/src/exit.rs`, and
are served as data by `podvm system info --format '{{json .ExitCodes}}'`.
⛔ They had been written out in **three**: `podvm-image::error`,
`podvm::main` and `podvm-enter`, and two of the copies had already
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
  15 cases, podvm and docker agree on every one
  exit 0
```

# T-0803: Answer to `docker` and `podman` on PATH

**Source:** `TOOL.md` section 2.0, section 6.8
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

An agent runs `docker`. If podvm is only reachable as `podvm`,
it has replaced nothing on the machines it is for.

## Premise

Read. On the studied runtime `docker` on PATH is already a podman
alias with no daemon, so the name is available and currently
resolves to something that does not work.

## Approach

Multicall on `argv[0]`: `docker`, `podman` and `podvm` all enter
the same parser, and the banner names which. Install the two extra
names as symlinks, never as copies, so one binary is one artefact.
⚠ Check what is already on PATH before replacing it, and refuse to
overwrite a real docker client without an explicit flag. A machine
with a working daemon is a machine where podvm is the wrong tool.

## Decision

Symlinks over a wrapper script. A shell wrapper is a second
artefact, it breaks the memfd rung (T-1001), and it is the exact
thing `references/qaidvoid__onelf` records as skipping that rung.
⭐ **RULED by the operator on 2026-09-08, and no longer open:**
podvm **refuses** to take the `docker` name where a working
docker daemon is reachable, unless an explicit flag says otherwise,
and says why in one line. A machine with a working daemon is a
machine where podvm is the wrong tool.
The alternative considered and rejected is deferring to the real
daemon transparently by exec'ing it: it costs an exec on every
call and it hides which tool ran, which is the class of thing
`TOOL.md` section 4.1 exists to forbid.
⚠ The check is on a **reachable daemon**, not on a `docker` binary
being present: on the target runtime `docker` on PATH is a podman
alias with no daemon behind it, so a binary check would refuse on
exactly the machines podvm is for.

## Prove

`./experiments/320-cli-contract.sh` clauses 4 and 5: both names run the payload and say which tool ran, and the `docker` name is refused where a daemon answers unless `--force`

## Done

**Done, 2026-09-09.** `crates/podvm/src/names.rs`, the banner line in `run`
and `exec`, `podvm system install-names`, and clauses 4 and 5 of
`experiments/320-cli-contract.sh`.

⭐ **Multicall on `argv[0]`, and the banner names which name was used.** Taking
the name is the product requirement; taking it silently is what `TOOL.md`
section 4.1 forbids, so every `run` and `exec` reached as `docker` or `podman`
carries one extra line saying this is podvm and where the differences are
listed.

⛔ **The daemon check is on a REACHABLE DAEMON and it is a real request.** The
socket named by `$DOCKER_HOST`, or `/var/run/docker.sock`, is connected to and
asked `/_ping` under a two-second timeout, so a stale socket file with nothing
behind it reads as absent, which is the state the machines podvm is for are
actually in. ⚠ A `$DOCKER_HOST` that is not a `unix://` socket is a THIRD state:
podvm says the guard itself is degraded and continues, which is the half of
`dockless`'s posture worth keeping.

⭐ **The ruling is about the `docker` name only.** On this machine, where a
daemon answers, `podvm system install-names` installs `podman`, refuses
`docker` with the reason, and exits 125; `--force` installs it. ⚠ Clause 5 reads
which state the machine is in first and asserts the other half where it can, so
a machine with no daemon reports the refusal half as unmeasured rather than as a
pass.

⛔ **Symlinks, never copies and never a wrapper script**, and the clause asserts
it on the filesystem rather than on an exit code. A file that is already a link
to this binary is left alone; anything else needs `--force`.

# T-0804: The honesty rules, and one switch that makes every degradation fatal

**Source:** `TOOL.md` section 4.1, section 6.8; `paper_final.md` section 10.1
**Category:** cli
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

The audience is automated. An agent cannot notice that its
"container" was a `chroot` the way a human skimming a log might, so
the reporting is the product and not a disclaimer.

## Premise

Read, and both halves are in the corpus.
`references/RuriOSS__ruri/tree/src/include/ruri.h:237-248` is the
degradation macro that becomes fatal under one flag.
`references/multikernel__sandlock` is the counter-example: a
`--dry-run` that reports no changes while changing files, traced in
T-0606.

## Approach

Four rules, enforced in the output path rather than in review:

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

## Decision

Suppressible by config, never by default. `ruri` allows
`--disable-warnings` to silence its degradation notices, which is
the `sandlock` failure mode with a flag in front of it; podvm's
equivalent is a config file a machine's operator sets once, and it
cannot be set from the command line of a single run.

## Prove

`! podvm run --rm --network=none public.ecr.aws/docker/library/alpine:3.20 true && ! podvm run --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true && ! podvm run --strict --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true`

## Done

**Done 2026-09-09.** All four rules, and the third and fourth needed a defect
fixed rather than a switch added.

1. **The banner prints on every `run` and `exec`**, suppressible by
   `<store>/config` with `banner = quiet` and never from the command line. ⭐ A
   file a machine's operator sets once is the Decision: `ruri`'s
   `--disable-warnings` is the `sandlock` failure with a flag in front of it.
   ⚠ Whether a machine has suppressed it is itself reported, by
   `podvm system info --format '{{.Banner}}'`, because a silent banner nobody
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
   printed `mode=` **the rung the PROBE selected**, and `podvm_enter` performs
   a plain `chroot` on every machine. On this host, where the probe selects
   `namespace`, every `podvm run` printed

   ```
   podvm 0.1.0: mode=namespace (namespaces: as configured; mounts: full; ...)
   ```

   for a payload that had no namespace and no mounts at all. It now reads the
   rung the entry sequence IMPLEMENTS, from one constant
   (`podvm_enter::ENTERED_RUNG`), and prints the machine's own answer beside
   it rather than dropping it:

   ```
   podvm 0.1.0: mode=chroot (namespaces: uts-only; mounts: none; devices: real;
   ownership: real; network: host-shared; pids: host-shared)
   this mode does NOT provide: process, network, IPC or mount isolation
   ⚠ this machine would permit `namespace`; podvm's entry sequence is `chroot`
     and creates no namespace and mounts nothing
   ```

⭐ **`--strict` reads three inputs and names every reason at once**, rather than
the first one it finds:

- **the flags this invocation passed**, against the parity table's own status.
  A `Degraded` or `Stub` row is a difference this run actually incurs;
- **the rung podvm entered with**, against `Selection::STRICT_FLOOR` -- the same
  constant `podvm probe --strict` gates on, not a second spelling of it;
- **the completion layer's report**, one row per fixup marked degraded.

⛔ A `None` row was already fatal without `--strict`, and has been since T-0801
made the table binding, so the switch is about the two statuses that otherwise
let a run proceed.

⚠ **THE `Prove` ABOVE CANNOT PASS AND ITS SECOND CLAUSE IS THE REASON.** It
expects `podvm run --rm --memory=1g ... true` to exit **0** and the same run
under `--strict` to fail. T-0801 landed after this entry was written and made
every `None` flag a refusal in the parser, so `--memory=1g` exits 125 with or
without `--strict` -- the entry's own rule 2, enforced earlier than it expected.
The rewrite, run 2026-09-09:

```
$ podvm run --rm public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  0, and the banner carries `mode=chroot` and the two lines above
$ podvm run --strict --rm public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  podvm run: --strict, and this run is degraded in 2 way(s). podvm refuses
  rather than running and letting the payload discover them:
    - the selected rung is `chroot` and not `namespace`, so the payload shares
      this machine's process table, network, IPC and mount namespaces
    - rewrote etc/mtab (T-0405): ...
  125
$ podvm run --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true; echo $?
  podvm run: -m, --memory is in the parity table with status None: resource
  limits need a cgroup this runtime does not grant
  125
```

# T-0805: Diagnostics that name the operation, the errno, the mechanism and the remedy

**Source:** `TOOL.md` section 6.9; `paper_final.md` section 10.8
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

In this environment the errno alone actively misleads. `EINVAL`
from `chown` reads as a bad argument and means an id that does not
exist. `fork/exec <path>: operation not permitted` reads as a
missing binary and is a denied `setgroups`.

## Premise

Read, and `TOOL.md` section 8 is a whole table of them. The target shape:

```
cannot restore ownership of etc/shadow (uid 0, gid 42): EINVAL
  gid 42 is not mapped in this user namespace (/proc/self/gid_map: 0 1000 1)
  extracting without ownership; intended metadata recorded in .meta.jsonl
```

## Approach

Every failure carries four parts: the operation, the errno, the
mechanism that produced it, and the remedy. The mechanism comes
from T-0102's discriminator and the maps from T-0105, so the
diagnostic quotes measured state rather than a guess.
Encode `TOOL.md` section 8's table as the mapping from a raw failure to
those four parts, so a new failure mode is a table row.

## Decision

Quote the actual `uid_map` contents rather than saying "an unmapped
id". The map is one read (T-0105) and it is what turns a confusing
errno into an explanation a reader can act on.

## Prove

`podvm pull public.ecr.aws/docker/library/alpine:3.20 && podvm extract public.ecr.aws/docker/library/alpine:3.20 2>&1 | grep -A2 'gid 42' | grep -q 'gid_map'`

## Done

**Done 2026-09-21.** `crates/podvm/src/diagnose.rs` carries TOOL.md
section 8 as data: 22 rows from observation to operation, errno, mechanism
and remedy. `ownership_note` renders the chown row with live measurements:
the first dropped entry from the sidecar, the map contents T-0105 reads,
and the sidecar path as the remedy. `report_dropped` prints the counts line
and the note. Both `extract` and `run` call it, so one extraction reads one
way on both verbs. The sidecar remembers the first dropped entry and
`Extracted` carries it (`src/sidecar.rs` and `src/lib.rs` of `pod-extract` (PodUtils/podcrates)).

The entry first named `pull` alone in Prove. Pull performs no extraction
(`crates/podvm-image/src/pull.rs:17`), so no pull output can name a dropped
id. Three routes were checked: a pull path that extracts somewhere unseen
(refuted by the pull handler, which prints the probe source and returns), a
predictive diagnostic inside pull (refuted, since pull cannot know dropped
ownership without extracting), and the pull-then-extract flow (adopted).

Run 2026-09-21 in a disposable container on host podman 6.1.2 with guest
maps `0 1000 1; 1 100000 65536`: pull exits 0, extract exits 0 over 517
entries with 1 dropped, and the note names etc/shadow (uid 0, gid 42) with
EINVAL beside the `/proc/self/gid_map` contents. The unit job exits 0: 71
passed in podvm with 5 new diagnose tests, 46 passed in
podvm-extract. Clippy passes with `-D warnings`. The spawn row in the table
stays unwired until T-0809.

# T-0806: Never prompt, never wait unbounded, and check space before every large write

**Source:** `TOOL.md` section 11.3; [RULES.md](RULES.md) section 8
**Category:** cli
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

A prompt with no terminal behind it is a hang, and a hang is total.
The three ways a long autonomous run dies are the three ways podvm
can make its caller die.

## Premise

Read, and one instance is in the corpus at file and line.
`references/89luca89__lilipod/tree/pkg/utils/utils.go:194-214`
checks `getsubids`, `newuidmap` and `newgidmap` with `exec.LookPath`
and returns an unrecoverable error when any is missing, and it runs
before **every** subcommand including `pull`. A dependency check
ahead of every syscall wall is the same class of problem as a
prompt: the tool refuses before it has done anything the user can
use.

## Approach

Three properties, tested rather than reviewed:

1. no code path reads stdin unless the user asked for it with `-i`;
2. every wait has an upper bound and a distinct outcome for reaching
   it (T-0602);
3. `statvfs` for blocks and inodes before every large write, with
   the destination named in the error (T-0203).

⚠ Where podvm needs a tool it does not have, it says so **at the
point of use** and not at startup, so every verb that does not need
it still works.

## Decision

Check at the point of use rather than up front. lilipod's shape
makes `pull` fail on a machine where `pull` would have worked, and
`pull` is the verb an agent reaches for first.

## Prove

`podvm run --rm public.ecr.aws/docker/library/alpine:3.20 true </dev/null && timeout 30 podvm pull public.ecr.aws/docker/library/alpine:3.20 </dev/null; test $? -ne 124`

## Done

**Done 2026-09-09.** All three properties, and each is asserted rather than
reviewed.

1. **No code path reads stdin.** podvm never opens or reads fd 0: the payload
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

⚠ **Where podvm needs a tool it does not have, it says so at the point of use.**
The keyring half of [T-0406](complete.md) is the instance: podvm cannot run
`pacman-key` from outside the chroot, so the fixup reports the state and names
the two commands, rather than refusing the run. That is the Decision's own
shape, against `lilipod`'s at
`references/89luca89__lilipod/tree/pkg/utils/utils.go:194-214`, which refuses
before every subcommand including `pull`.

Prove, run 2026-09-09:

```
$ podvm run --rm public.ecr.aws/docker/library/alpine:3.20 true </dev/null; echo $?
  0
$ timeout 30 podvm pull public.ecr.aws/docker/library/alpine:3.20 </dev/null; echo $?
  0, and never 124
```

# T-0807: `podvm images --format` refuses a template no verb can answer, before it looks at the store

**Source:** Found by driving the CLI cold against an empty store
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

A `--format` template was validated inside the loop over records.
An empty store means zero iterations, so an invalid template was
never seen: `podvm images --format '{{.Nope}}'` printed nothing
and exited 0, and a caller's typo read as an empty result set.

## Premise

⭐ **Measured by running it**, not by reading it, on 2026-09-08.
Three templates exited 0 against an empty store where all three are
refusals: an unknown field, an unterminated opening delimiter,
and docker's
`table` prefix, which rendered the word `table` beside the values.

## Approach

One walk over the template, shared by a `check` that has no record
in hand and a `render` that does, in
`crates/podvm/src/format.rs`. Both verbs call `check` before
the store is opened, against a named list of their own fields.
⛔ The field-name list and the field builder are two places holding
one value, so a test asserts they agree, which is the remedy
`docs/conventions/forbidden-patterns.md` names for exactly that row.

## Decision

Refuse `table` rather than rendering it as literal text. It selects
a column layout in docker and podvm does not have one, so printing
it is output shaped like something podvm does not do.

## Prove

`podvm images --format '{{.Nope}}' >/dev/null 2>&1; test $? -eq 2`

## Done

**Done 2026-09-08.** Driven against an empty store: the unknown field, the
unterminated brace, the `table` prefix and a pipeline all exit 2 and name what
is wrong; `{{.Digest}}` exits 0. `inspect` reports a bad template before it
reports a missing image, because the template is the caller's own input.

# T-0808: Drive every row of the parity table through the shipped binary

**Source:** T-0801; the door sweep of 2026-09-10
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

⛔ **The table is 141 rows and nothing asserts the binary agrees
with all of them.** `experiments/320-cli-contract.sh` drives a
handful: the JSON parses, one `None` flag refuses with its own
note, one unlisted flag is refused. The other rows are checked by
a unit test that walks the table and calls the PARSER, which is
the same code that reads the table, so the two cannot disagree
even when the binary is wrong.

## Premise

Found by hand on 2026-09-10 and not by any check: `podvm create
--no-steps` was accepted and acted on while `podvm system info`
listed `--no-steps` under `run` and `exec` only. ⚠ The unit test
could not have caught it, because it asks the table which flags a
verb has and then asks the parser about those; a flag the parser
accepts and the table does not mention is invisible from that
direction. ⛔ **The check has to run the BINARY and start from the
flag, not from the row.**

## Approach

One driver, over the table read out of the binary itself:
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

## Decision

Admit-first everywhere, in the same change as the driver. Every
dash-arg passes `parity::admit_all` before any match arm runs, so
an arm for a flag with no row is unreachable and a row with no arm
ends at `parity::no_arm`, which the driver reports as a mismatch.
Group subverbs carry the path the caller typed (`image prune`,
`system abi`) and resolve their rows through `parity::rows_of`.
Approach clause 3 closed structurally rather than by enumeration:
with the pre-pass there is no second list to compare.

## Prove

`./experiments/325-parity-drive.sh`, which exits 0 only when every
row of `podvm system info --format '{{json .Parity}}'` was driven
or reported as unreachable here, and prints the count of each.

## Done

**Done, 2026-09-21.** The driver is green against the shipped binary: 160
rows, 199 driven, 0 mismatches, 0 unreachable here
(`experiments/results/parity-drive.txt`, with the conditions at its head).
Both figures are historical readings; the table holds 265 rows
(`crates/podvm/src/parity.rs:119`).
The 153-row figure this record carried went stale at T-1302, which added
5 rows (the two tier rows, the two qemu-arg rows and `podvm`) without
re-driving, and T-1305 added 2 more (the two `--podvm-mem` rows) with the
re-drive above: 153 plus 5 plus 2 is the 160 the binary publishes.
`run -i` and `exec -i` both ran a self-pulled `alpine:3.20` payload with the
banner naming `-i`. Release build, `clippy --workspace --all-targets` with
`-D warnings`, and `cargo test -p podvm -p podvm-probe -p podvm-extract`
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

# T-0809: The spawn that fails with the wrong reason, and the one field that fixes it

**Source:** `https://github.com/talaria0101/sandbox-insights`, its walls document, wall 2; T-0805 above
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

A payload inside podvm spawns a child, the spawn fails, and the
message says the operation is not permitted. ⛔ **That one string
covers at least two different refused calls**, and reading it as the
wrong one has cost whole architectures.

## Premise

⭐ **Measured, with the matrix committed, by somebody else.** Go's
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

## Approach

podvm does not spawn this way: it is Rust and it declares its own
syscalls. ⛔ **The payload does, and podvm is what the payload's
operator reads.** So this is a diagnostic row and not a code change:
T-0805's four-part message gains an entry mapping the observed
string plus a denied `setgroups` leg to the mechanism and the
one-field remedy.
⚠ The discriminator is already probed: [probe.md](probe.md)
separates a refused clone from a refused `setgroups`, so podvm can
say which wall the payload met rather than repeating the payload's
own ambiguous string.

## Decision

Diagnose, never patch. ⛔ podvm does not rewrite a payload's
process attributes: that is a behaviour change inside somebody
else's program, which is a larger thing than
[complete.md](complete.md) T-0407's trust change, and it is not this
tool's to make. It names the wall and the remedy.

## Prove

`./experiments/151-spawn-ambiguity.sh` asserts podvm distinguishes a refused clone from a refused `setgroups` behind one identical payload message, and that the diagnostic names the field

## Done

**Done 2026-09-21.** The probe evidence names the wall per context
(`crates/podvm-probe/src/report.rs`): both legs with what this machine
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

# T-0810: `create` fails storing the memo: the container directory is never made

**Source:** T-0708's drive, host podman 6.1.2
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`podvm create --name e1 <image> ...` fails with `the ownership
memo could not be stored: No such file or directory`, so no
container with a record can be created and `inspect` has nothing
to read. `run --detach` shares the rename and fails the same way.

## Premise

Read at file and line. `crates/podvm/src/lifecycle.rs`
`create` renames the ephemeral memo into
`supervise::table::memo_path` beside the new record, and
`crates/podvm/src/run.rs` (detach) does the same. Neither
the rename's parent (`table::dir`, a pure join) nor
`supervise::create` (a table-JSON write only) makes the
directory. Foreground `run` never renames, which is why every
drive to date stayed green past it.

## Approach

Make the directory where the container is created, not where
the memo is renamed: `supervise::create` owns the container's
directories (`lock_path`, `log_path` and `control_path` all
live under `dir`), so it ensures `dir` after the table write.
Both rename call sites ride it with no change.

## Decision

A failed directory fails the creation loudly, beside the
rename's own failure. A container with a record and no memo
answers `stat` with the real uid and contradicts every
foreground run, which is the inconsistency T-0710 refuses.

## Prove

`podvm create --name e1 public.ecr.aws/docker/library/alpine:3.20 true && podvm rm e1`

## Done

**Done 2026-09-22.** `supervise::create` ensures `containers/<id>/` after the
table write (`crates/podvm-supervise/src/lib.rs`). Both memo renames ride it
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

# T-1319: `inspect` prints exactly one document for any reference

**Source:** issue 21, client beta testing 2026-09-22 (container
  inspect emits the record plus a spurious `[]`);
  `crates/podvm/src/images.rs` (the `find_one`, then
  `inspect_container`, then unconditional array print)
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

`podvm inspect <container>` prints two JSON documents: the
container record (printed inside `inspect_container`) and a
trailing `[]` (the empty image `records` vec, printed
unconditionally after). An automated caller parsing stdout
fails; `jq` cannot consume it. Image references print one
array correctly, and `--format` is unaffected, so only the
container path is broken.

## Premise

Measured by the reporter (`json.load` fails with "Extra
data", `raw_decode` finds two documents) and confirmed
against the loop above: the container branch prints inside
the call and returns `Some(0)`, leaving `records` empty for
the final print.

## Approach

Do not print the trailing image array where the reference
resolved to a container (or collect the rendered container
document into `records`, one path, one print). Out of
scope: changing either document's fields, touching image
inspect.

## Decision

Skip the second print on the container path: the smaller
diff, and the image path keeps its exact bytes.

## Prove

`podvm inspect <container>` piped to `python3 -c
'json.load(sys.stdin)'` and to `jq` both succeed with one
document; image inspect output is byte-identical before and
after. Close issue 21 with a comment showing the parse and
the single-print path as the guard that stops recurrence.

## Done

**Done 2026-09-23.** Skip the second print on the container path, as
decided. `inspect` (`crates/podvm/src/images.rs`) tracks whether any
reference resolved to a container; where the image `records` vec is empty
and a container document is already on stdout, the trailing array print is
skipped and the verb returns the container's code. The image path is byte
for byte what it was: the array prints wherever a record resolved,
including the empty store (`[]` with no container involved).

Driven on the shipped binary in the lane (`rust:1.98.1-bookworm` through
host podman 6.1.2), against `public.ecr.aws/docker/library/alpine:3.20`:

```
$ podvm create --name green1 ... true && podvm inspect green1
[{"Command":"true",...,"Name":"green1",...}]
$ podvm inspect green1 | python3 -c 'json.load(sys.stdin)' && echo PARSES
ONE-DOCUMENT len=1
$ podvm inspect public.ecr.aws/docker/library/alpine:3.20
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

# T-1323: `cp` reaches an image's extracted rootfs, and copies directories with the same checks

**Source:** issues 14 and 16, client beta testing 2026-09-22 (`cp`
  gated on container records; no recursive copy);
  `crates/podvm/src/parity.rs` (the `cp` row: one file
  at a time, gated through the containment check)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

`cp` only addresses `container:path`, and `run` leaves no
record, so on a host where the payload cannot launch there
is no checked way to stage or retrieve files: the only
interface with containment checks is gated on a started
container, which restricted hosts cannot always provide.
Separately, `cp` copies one file at a time; a recursive
option with the same checks has no entry.

## Premise

Measured by the reporter: `cp alpine:latest:/etc/os-release`
refuses ("no such container"), `create`+`cp` works only
where `start` works, and raw store writes omit every
check. The containment machinery (`podvm-extract` safety
module) and the extract walker already exist to make both
halves safe.

## Approach

Accept `image:path` (the extracted rootfs in the store) as
well as `container:path`, reusing the containment-checked
path unchanged; add a recursive option over the same walker
with the same checks. Out of scope: a new stage/unstage
verb pair (rejected: `image:path` covers the workflow with
no new verb and no parity debt), documenting raw store
surgery (never an answer).

## Decision

Extend `cp`, do not add verbs. One verb, one checker, two
new addressings.

## Prove

`podvm cp image:/etc/os-release` on a host where
payloads cannot launch retrieves bytes, and `podvm cp` of
a directory tree round-trips with containment intact (a
hostile symlink inside the tree refuses rather than
escapes); `podvm cp --help` documents both. Close issues 14
and 16 (cp half) with comments showing the commands and the
shared containment check as the guard that stops recurrence.

## Done

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
Lane prove [`pb-w23c-prove`](../docs/history/lane-proofs-2026-09-23/pb-w23c-prove.txt), verdict `fail=0`:
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

# T-1330: Bundled short flags parse as docker reads them

**Source:** issue 27, client beta testing 2026-09-22 (`ps -aq`,
  `run -it` refused); `crates/podvm/src/parity.rs`
  (the `-a`/`-q` and `-i`/`-t` rows, never mentioning
  bundling)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

Combined short flags are refused: `ps -aq` and `run -it`
answer `unknown option` with 125. Docker callers write
clusters constantly; the parity table lists the member flags
separately and never mentions bundling, so the refusal reads
as a missing flag rather than a missing spelling.

## Premise

Measured by the reporter on the beta.3 asset, both forms,
both 125. The parser has no cluster path (each flag is
matched whole).

## Approach

Parse a cluster of value-less short flags by docker's own
rule (a cluster containing a flag that takes a value
consumes the rest as its value; an unknown member refuses
naming the member). One parser change, every verb inherits
it. Out of scope: new flags, changing any refusal code.

## Decision

Docker's cluster rule exactly, not a subset. A subset would
move the surprise rather than remove it.

## Prove

`ps -aq`, `run -it` (and a cluster with an unknown member,
refusing by name) behave as docker's rule says across three
verbs; the existing flag tests still pass unchanged. Close
issue 27 (bundling third) with a comment showing the runs
and the single parser path as the guard that stops
recurrence.

## Done

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
spellings now. Lane prove [`pb-w30-prove`](../docs/history/lane-proofs-2026-09-23/pb-w30-prove.txt), verdict `fail=0`:
cluster units by exact name, the pre-existing flag tests unchanged,
clippy clean, `ps -aq`, `run -it`, `inspect -f` and `ls -la` through
a payload green, `-aZ` refusing 125 naming `-Z`. The guard is the
single expansion path plus check 26's mirror.

# T-1331: `--filter`, `restart` and `pull -a/-q` answer the docker idiom

**Source:** issue 27, client beta testing 2026-09-22 (three refused
  idioms with no entry);
  `crates/podvm/src/parity.rs` (the `None` rows for
  `--filter`, `restart`, `pull -a`)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

Three docker idioms break with 125 and have no work-order
entry: `--filter` on `images` and `ps` (every `docker ps
--filter name=x` one-liner), `restart` ("`stop` then `start`
is the same thing" is not the same verb), `pull
-a/--all-tags` and `pull -q`. The table is honest; the work
order does not know these are missing.

## Premise

Measured by the reporter on the beta.3 asset, each form
with its refusal text. Bundled flags are the sibling entry;
these are per-verb gaps, not parser gaps.

## Approach

Implement the three behind the table rows that already name
them: `--filter` as a caller-side predicate over listed
records (no new query language), `restart` as stop-then-
start in one verb reporting which half failed, `pull -a` as
all offered tags and `-q` as quiet output. Out of scope:
server-side filter syntax beyond `name=` and label
equality (named if dropped), restart policies (a different
gap).

## Decision

Caller-side filter, composite restart, per-tag pull. Each
reuses a path that already exists rather than forking one. A
tag with no manifest for this platform is named and skipped
(typed `NoPlatform`, not a string match); any other failing
tag stops the run, and an offer with nothing for this
platform at all is an error.

## Prove

`ps --filter name=x` selects, `restart` stops and starts
with both halves named, `pull -a` fetches every offered tag
and `-q` stays quiet; the three parity rows move status with
the behavior. Close issue 27 (idiom thirds) with a comment
showing the runs and the row-status moves as the guard that
stops recurrence.

## Done

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
fails with the same text and code. Lane prove [`pb-w31-prove`](../docs/history/lane-proofs-2026-09-23/pb-w31-prove.txt),
verdict `fail=0`: filter units by exact name, suites, clippy clean,
`ps --filter` selects/empty/refused-key, `images --filter` selects,
`restart` composites with the stop half named, `pull -a` rc 0 with four
`nanoserver` skip lines and `hello-world:latest` plus `:linux` in the
store, explicit-tag `-a` refused, `-q` byte-quiet, the three parity rows
moved. The guard is the three moved table rows plus the prove script.

# T-1332: `podvm man` generates the manual from the binary, pager-aware

**Source:** operator order 2026-09-23 (drift-free human/AI manual;
  docs need not carry help text); `crates/podvm/src`
  (per-verb usage strings, the parity table as data)
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Help text lives in docs or not at all, so every new flag is
a chance for drift: the manual says what the binary said on
the day somebody copied it. Nothing generates a readable
manual from the binary for humans (`less`) and agents
(plain text) alike.

## Premise

Audited: no `man` surface exists in `crates/podvm/src`
(only shell completion in `complete.rs`); per-verb usage
strings and the parity table already carry the content a
generator needs.

## Approach

A `man` verb rendering every verb, flag and parity note
from the binary's own data (usage strings plus
`parity::TABLE`), plain text to stdout, shaped like
`wsl-toolkit man --no-pager`: version header, command list
with one-line summaries, a Global section, then a COMMAND
REFERENCE with one section per command carrying its
synopsis and flags. `podvm man [verb]` pages through
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

## Decision

Generated plain text, pager-aware, no troff. The audience
is a tired reader and an agent: both read text, neither
needs typesetting.

## Prove

A script diffs `podvm man` output against every `--help`
output plus the parity rows and fails on any verb or flag
the manual lacks; `podvm man --no-pager` and piped `podvm
man` print identical bytes with no pager spawned. The
completeness script is the drift guard that stops
recurrence.

## Done

**Done, 2026-09-23.** New `man` verb in `crates/podvm/src/man.rs`:
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
prove [`pb-w32-prove`](../docs/history/lane-proofs-2026-09-23/pb-w32-prove.txt), verdict `fail=0`: seven man units by
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
prove script with `crates/podvm/src/man.rs`. Risk if wrong
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
is loud on the terminal (`podvm man:
/bogus-that-does-not-exist could not start (No such file or
directory (os error 2)), printing without a pager`,
`TTY-FALLBACK-LOUD`), which is what makes the empty stderr above
meaningful rather than vacuous. A regression that spawns where it
should not now fails the stderr assertion; a regression that
silences the fallback now fails the pty assertion.

# T-1336: CLI output is plain ASCII: no emoji, no markers, on every path

**Source:** operator order 2026-09-23 (no place for either in a CLI
  tool, let alone without a terminal or pty);
  `crates/podvm/src` (usage strings, error messages and
  parity notes carry marker glyphs on dozens of paths:
  `complete.rs:103`, `exec.rs:252`, `main.rs:372`,
  `names.rs:259`, `parity.rs:103`, `run.rs:26`, and the
  per-verb usage blocks behind every `--help`)
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

The binary's user-facing text carries ⛔/⚠/⭐ glyphs on
dozens of paths: per-verb usage blocks (printed by every
`--help`), error messages (`complete`, `exec`, `probe`
cache, `install-names`), and parity notes (the
machine-readable contract). On a constrained host with no
terminal or pty these are unrenderable bytes in an
automated caller's stream, and they force every downstream
parser to handle codepoints that carry no meaning.

## Premise

Measured on this tree: non-ASCII bytes outside comments
across `crates/podvm/src` (usage blocks, emitted
errors, table notes); comments and doc-comments that are
never printed are not in scope and stay.

## Approach

Scrub every printed string to ASCII, replacing each glyph
with the word it stands for (`refused:`, `note:`), usage
blocks, errors, banner lines and parity notes alike; keep
source comments as they are. Add the guard that stops
recurrence: a test (or gate check) failing any non-ASCII
byte in `--help` output per verb, the `man` output, and the
error/catalog paths. Out of scope: docs and comments (the
markers check owns those), changing any message's meaning
(glyphs become words, nothing is reworded).

## Decision

Words, not glyphs, everywhere printed. The manual entry
(T-1332) inherits ASCII output from this one.

## Prove

`./experiments/352-ascii-output.sh` exits 0, with the
existing suite green; the guard runs in the gate so a new
glyph fails before it ships.

## Done

**Done 2026-09-23.** Words, not glyphs, as decided: every printed
string across `crates/podvm/src`, `crates/podvm-probe/src`,
`crates/podvm-image/src/tls.rs`, `src/abi.rs` of `pod-enter` (PodUtils/podcrates)
and `crates/podvm-complete/src` carries bytes `0x00`-`0x7F` only.
U+26D4 reads `refused:`, U+26A0 reads `note:`, U+2B50 is deleted;
source comments stay as they are. Twenty-two files, 84 insertions
and 82 deletions, nothing reworded.

`experiments/352-ascii-output.sh` drives the binary rather than
grepping the source: every implemented verb's `--help` (verbs read
out of the binary's own parity table, never listed), `man` under
both pagings with stdout and stderr separated, and a catalog of
five error paths. `experiments/results/ascii-output.txt` carries
the run: 69 `ASCII-OK` lines, zero failures, verdict `fail=0`.

Ten unit tests hold the constants the script drives (usage per
module, every parity note, the whole manual through the stub):
`cargo test -p podvm ascii`, 10 passed. Clippy `-D warnings`
clean in the same lane run. The gate guard is check 28 in
`crates/podvm-gate/src/main.rs` (any non-ASCII byte in a printed string
fails, comments exempt) with its plant case in `podvm-plant`:
a glyph planted in `run.rs` goes red naming `U+26D4` and T-1336.
The guard is the check plus the 352 experiment: a new glyph fails
the gate before it ships and fails the binary drive beside it.

# T-1337: Doctor, disk usage, and log tail for operators

**Source:** issue 38, beta.7 drive 2026-09-25 (no `doctor`, no `df`,
  `logs` without `--tail`); `references/carlbomsdata__winquick`
  `tree/src/main.rs:50` with `tree/src/facts.rs:125-180` (the shape
  to copy, not the content)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

Three operator gaps found while driving beta.7. `podvm
probe` prints the measurement half (legs, verdicts,
refusals) and never the setup half: a QEMU too old for
migration, firmware present or absent, helper tools
present, image installed, disk free against the ceiling.
No `system df` sums stored, extracted and reclaimable
bytes, which operators on small stores need beside the
prune path. `logs -f` replays from the start on every
reconnect, because there is no `--tail N` (and later no
`--since`).

## Premise

Read on the lane-built binary 2026-09-25
(`experiments/results/triage-353.txt` clauses
`38-system-help` and `38-logs-help`): `system --help`
lists no doctor and no df, `logs --help` lists only
`-f|--follow`, and `probe --json` already carries every
leg a doctor would consume. `TODO/supervise.md`
already documents that `logs` has no `--tail`.

## Approach

Three small parity additions, each with its row and its
Prove through the shipped binary, and no spec change
beyond one row each. `podvm doctor` (or `probe --fix`)
reuses the machine legs with the T-1301 checks and prints
one fix line per missing piece, with the third-state
exits (0 ok, 1 problem, 2 could not run). `podvm system
df` (with image du rows) reports stored, extracted and
reclaimable bytes with the container gate applied, and
never exits non-zero for accounting alone. `logs --tail
N` (with `--since` as a second step) reads the same
captured file T-0605 owns, bounded like `-f`, with
byte-identical default output unchanged.

## Decision

One entry, three verbs in order doctor, df, tail, each
shippable alone. The doctor copies the reference's shape
(check lines with fix lines) and none of its content.
Out of scope: a log stream split (T-0605), a network check needing
a registry, any new exit-code meaning.

## Prove

`podvm doctor` on a kvm-less host exits 1 naming the
missing legs with one fix line each; `podvm system df`
sums to the store's own accounting; `podvm logs --tail 5`
prints the last five lines and `podvm logs` prints all
lines byte-identical to before.

## Done

**Done, 2026-09-26.** Three verbs in commit order tail, df,
doctor. Tail landed first: `logs --tail N`
(`--tail=N`, both in any position) prints the last N lines of the
same captured file, byte-identical default untouched, and `-f
--tail N` prints the tail then follows from the chained offset
instead of replaying the file. `podvm-supervise::tail_offset`
(pure, unit-pinned) with `follow_from` chaining the read length
past the printed slice, so no line is lost or repeated. A
non-count is a flag error at 125 naming the value. Prove through
the shipped binary (`experiments/364-qol.sh` tail clauses, exit
0, `experiments/results/qol.txt`): last five of ten exact,
plain logs byte-identical to a wide tail, `--tail 0` empty at
exit 0, non-count refused, `-f --tail 3` following to the end.
Doctor and df stay open below.

**Df, 2026-09-26.** The second verb landed: `podvm system df`
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
`podvm-gate` mirror maps it too, `man` routes its help.
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
explained away. CI went red on the df commit itself (plant
27b MISS: the shipped note moved out from under its sed) and
green on the plant retarget one commit later; the MISS line is
what T-1202 exists for. Doctor stays open
below.

⚠ Citation correction, read at file and line 2026-09-26: the
doctor shape lives at `tree/src/main.rs` (help text naming
`winquick doctor`) and `tree/src/facts.rs:125-180` (`Status`,
`Check`, `Doctor` with the ordered fix list, builder with
ok/note/fail, health agreeing with the problem list) - there is
no `tree/src/bin/` directory. The tree was read at one pass
([history/2026-09-11-reference-sweep.md](../docs/history/2026-09-11-reference-sweep.md)),
so only the shape travels, none of the content.

**Doctor, 2026-09-26.** The first verb landed last:
`podvm doctor` prints every machine leg as a check line, one
fix line per failed required leg, and the profile. Required
legs fail with fixes (install QEMU where the emulator or its
accelerator list does not answer; the failing call named with
its errno for the fsize and space legs); kvm and tun missing
alone are notes (TCG without hardware acceleration,
user-mode networking); a required leg never measured is
unmeasured and exits 2 rather than guessing a fix. A final
check compares the disk free at the store (or `.` with no
store) against the RLIMIT_FSIZE ceiling read by syscall:
below it fails with room-freeing fixes, infinity notes, an
unreadable limit or room notes where the legs already refuse.
Exits 0, 1, 2 in that precedence. No firmware, helper-tool or
image-presence checks: the tier boots `-kernel` directly and
the runtime consumes no helpers, so each would check what
nothing reads. The leg names and the leg renderer are public
from `podvm-probe::machine` rather than copied, so the
wording cannot drift. Top-level parity row with its `-h` row.
Prove through the shipped binary (`experiments/364-qol.sh`
doctor clauses, exit 0, `experiments/results/qol.txt`): exit
0 with profile tcg on the kvm-less lane, the kvm line and the
ceiling line present with no fix; PATH stripped of the
emulator exits 1 naming the emulator leg with its install-QEMU
fix; `--bogus` at 125, `--help` at 0. Unit-pinned: health
agrees with the problem list (fail carries exactly one fix,
unmeasured outranks failed), help and bad flags need no probe.
With all three verbs driven and 364 fully green, issue 38
closes below.

# T-1414: The no-chroot path returns 127 and 126

**Source:** issue 75; `crates/podvm/src/lifecycle/entry.rs:186`;
  `crates/podvm-probe/src/exit.rs:86`;
  `experiments/330-exit-codes.sh`
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

A missing command returns 125 on the userland rung. The
contract reserves 125 for podvm failing and 127 for not
found. Scripts branching on 127 misread the result.

## Premise

`decide_entry` (`lifecycle.rs:1746`) returns 125
unconditionally, including the unresolvable-payload arms
(`lifecycle.rs:1774`, `lifecycle.rs:1831`). `EXIT_NOT_FOUND`
exists with its case row (`exit.rs:86`) and the chroot path
emits it from the child. Nothing in
`crates/podvm/src` references `EXIT_NOT_FOUND` or
`EXIT_CANNOT_INVOKE` (measured by grep: zero hits), so the
userland path cannot produce 127 or 126 by construction.
`330` covers not-found only through the absolute-path
chroot case.

## Approach

Map `ResolveKind::Absent` to 127 and found-but-not-invocable
to 126 in the `no_chroot` arms. Add the bare-name trio to
`330` per rung.

## Decision

The exit code follows the payload status on every rung. The
rung never changes the code's meaning.

## Prove

`cargo test -p podvm` exits 0.
`sh experiments/330-exit-codes.sh` exits 0 with the userland
missing, not-invocable, and podvm-failure clauses.

## Correction

**Open 2026-09-30 (history).** Filed from issue 75. Extends [T-0802](cli.md),
whose proof ran on the chroot rung only.

## Done

**Done 2026-09-30.** The `no_chroot` arms map `ResolveKind::Absent`
to 127 and found-but-not-invocable to 126 (the T-0802 errno split);
genuine podvm failures stay 125. `cargo test -p podvm` exits 0
with the remapped arms pinned. The trio is driven two ways: the new
330 section 7 runs the bare-name userland missing, not-invocable, and
podvm-failure clauses (shape-probed green in-lane), and 356 clauses
7, 10, and 13 witness 127, 126, and 125 through the same exit-code
table on the fixture ([the drive](../experiments/results/no-chroot-rung.txt)).
Amendment in writing: full `330` exits 2 SKIP in this lane (no engine;
the drive gates on one at its head), so the userland trio is proven
through 356 plus unit until an engine lane re-drives section 7 whole.
The 330 drive itself gained a portability fix beside this entry: its
`set -o pipefail` line killed dash outright, so the option is now
gated on the shell name and `sh 330-exit-codes.sh` reaches the engine
check ([the SKIP run](../experiments/results/exit-codes-2026-09-30.txt)).

# T-1415: Strict classifies degradations on the userland rung

**Source:** issue 76; `crates/podvm/src/complete.rs:122`;
  [T-0804](cli.md)
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

`--strict` refuses every run where chroot is denied. The
five unavoidable dev-shim substitutions each count as a
degradation. The callers who want the gate most get
nothing.

## Premise

`strict_refusal` (`complete.rs:122`) pushes every
completion degradation and every step as a refusal reason
and refuses on the count. That is exactly what T-0804
specified before the userland rung existed. Its proof never
exercised `--strict` there.

## Approach

Classify degradations. Safety-relevant ones (no path
virtualization, host and libc mixing, escape) still refuse.
Substitution ones (dev shims) warn. Add `--strict=all`
preserving the current behavior. The refusal line names the
class.

## Decision

Kind decides, count no longer does. The current behavior
stays available under `--strict=all`.

## Prove

`cargo test -p podvm` exits 0. A tracked drive runs
`--strict public.ecr.aws/docker/library/alpine:3.20 echo hi`
clean on a chroot-denying fixture, still refuses a
path-virtualization defect, and `--strict=all` still refuses
the dev-shim run.

## Correction

**Open 2026-09-30 (history).** Filed from issue 76. Extends [T-0804](cli.md).

## Done

**Done 2026-09-30.** Strict refusal classifies by kind, not by count:
safety-relevant degradations (no path virtualization, host and libc
mixing, escape) still refuse, dev-shim substitutions warn, and
`--strict=all` preserves the old count-based behavior with the class
named on the refusal line. `cargo test -p podvm` exits 0 with
the class-aware tests. `sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): `--strict`
runs the alpine payload clean on the fixture, refuses the
path-virtualization defect, and `--strict=all` refuses the dev-shim
run. This extends T-0804, whose proof never exercised `--strict` on
the userland rung.

# T-1416: Quiet runs and a verbose banner

**Source:** issues 77 and 79;
  `crates/podvm/src/complete.rs:57`;
  `crates/podvm/src/run/prepare.rs:210`; [T-0804](cli.md)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

Every successful run prints the rung narration to stderr
with no per-run opt-out. Harnesses running hundreds of
commands cannot silence it. Issue 79 asks for the stronger
shape: quiet by default, full narration under `-v`, `-q` as
force-off.

## Premise

`banner_quiet` reads only the store config, never argv or
env (`complete.rs:57`). `run` and `exec` parse no `-q`,
`--quiet`, `-v`, or `--verbose` flag (measured by grep:
zero hits). The never-quiet-by-default test pins the
current shape (`complete.rs:515`). T-0804 decided the banner
cannot be set from a single run's command line.

## Approach

Implement the issue 79 shape: empty stderr on success at
default level, banner under `-v`, `-q` forces off, refusals
and payload stderr untouched. Record the reversed T-0804
decision in writing.

## Decision

Ruled 2026-09-30 by the operator: the issue 79 shape.
Quiet by default, banner under `-v`, `-q` forces off. The
default output change touches every user; refusals and
payload stderr stay untouched, so no safety information is
lost. Issues 77 and 79 both close here. This reverses the
T-0804 command-line rule in writing.

## Prove

`cargo test -p podvm` exits 0. A tracked drive shows
empty stderr on success, the banner under `-v`, silence
under `-q`, and a refused run printing at default level.

## Correction

**Open 2026-09-30 (history).** Filed from issues 77 and 79. Read the current
source before implementation.

## Done

**Done 2026-09-30.** The operator-ruled issue-79 shape: runs are
quiet by default (empty stderr on success), the banner prints under
`-v`/`--verbose`, `-q`/`--quiet` forces it off, and refusals and
payload stderr are untouched. `--verbose` is long-only: `-v` stays
docker's `--volume` refusal, and reclaiming it would turn a volume
refusal into a banner switch, so the parity rows say so in writing.
`cargo test -p podvm` exits 0 with the rewritten banner tests.
`sh experiments/356-no-chroot-rung.sh` exits 0
([the drive](../experiments/results/no-chroot-rung.txt)): clause 14
shows empty payload stdout with no banner line on either stream at
default, the banner under `--verbose`, silence under `-q`, and a
refused run printing at default. Limit stated beside the proof:
extraction progress still rides stderr through the extract side, so
the clause asserts banner-silence per stream, not total silence.
This reverses the T-0804 command-line rule in writing; issues 77 and
79 both close here.

# T-1417: Inspect carries the OCI config

**Source:** issue 82; `crates/podvm/src/images/inspect.rs:145`;
  `src/spec.rs` of `pod-oci` (PodUtils/podcrates);
  `crates/podvm-image/src/store.rs:127`; [T-1319](cli.md)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

`inspect` returns only podvm's record. Image defaults
already applied at runtime, such as `Env`, are unreadable.
The docker `inspect` contract breaks for `Config.Env`,
`Cmd`, `Entrypoint`, `WorkingDir`, and `User`.

## Premise

`INSPECT_FIELDS` is a closed enum with no `Config` branch
(`images.rs:1728`). `inspect_fields` and `inspect_json`
have no `Config` arm (`images.rs:1789`, `images.rs:1824`).
The config is parsed as `RunConfig` (`oci.rs:141`), its
digest is stored on the record (`store.rs:127`), and the
blob is re-readable (`store.rs:479`).

## Approach

Read the config blob through the stored digest. Emit
`Config` with docker key names, null-safe where the image
omits a field. Keep the one-document contract from T-1319.

## Decision

`inspect` reports what the image declares and what podvm
applied, side by side. It never merges the two silently.

## Prove

`cargo test -p podvm` exits 0. A tracked drive matches
`.Config.Env` against the manifest and shows `Cmd`,
`Entrypoint`, `WorkingDir`, and `User` present.

## Correction

**Open 2026-09-30 (history).** Filed from issue 82. Read the current source
before implementation.

## Done

**Done 2026-09-30.** `inspect` reads the config blob through the
stored digest and emits a `Config` object with docker key names,
null-safe where the image omits a field, in both `--format` and the
JSON document, keeping the T-1319 one-document contract: declared and
applied side by side, never merged silently. `cargo test -p podvm`
exits 0 with the field-agreement and template tests.
`sh experiments/373-store-gates.sh` exits 0 with verdict
`STORE GATES HOLD` ([the drive](../experiments/results/store-gates.txt)):
clause 2 matches `.Config.Env` (PATH=) and `.Config.Cmd` (/bin/sh)
against the manifest and shows the `Config` object in the document.

# T-1630: The symlink-escape script is converted, and its deletion has a condition

**Source:** `refactor/recon-c.md:177`, the plan's row, read off disk
  2026-10-02
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

The experiment 85-completion-symlink-escape.sh (retired by T-1735) is converted
and its deletion is blocked on another task, so the tree
reads as an unfinished port with no owner and no
condition.

## Premise

The conversion is the two library tests the plan names in
the completion write module. The deletion condition is the
entry that writes those tests, named in the plan's row and
not yet a tracked entry, so it is recorded here rather than
cited as an id a reader cannot follow.

## Approach

Record the converted state and the deletion condition, and
name what clears it. Do not delete the script here: this
row is a record, and deleting it widens the unit past the
record.

## Decision

Record, not delete. The condition is another task's and a
deletion taken from under it loses its plant.

## Prove

`grep -n "85-completion-symlink-escape" TODO/cli.md` names
the converted tests and the work whose landing clears the
deletion.

## Correction

**Recorded 2026-10-02.** The experiment 85-completion-symlink-escape.sh (retired by T-1735)
stays on disk and is converted in the library. The script plants six
doors, five out of the rootfs and one legitimate internal link
(`85-completion-symlink-escape.sh:23`, `:105`): A `/etc/mtab` (`:25`), B
`/etc/resolv.conf` (`:26`), C `/etc/hosts` (`:27`), E `/etc/passwd`
(`:28`), D `/etc/ssl/certs` (`:29`), F `/etc/apt -> /var/aptreal` (`:30`).

The converted tests are the two in
`src/write.rs` of `pod-rootfs` (PodUtils/podcrates) `mod tests` that name the script's
clauses. `a_write_replaces_an_out_link_with_a_regular_file`
(`write.rs:752`) carries clauses A through E: the out-link is replaced by
a regular file inside the rootfs, so a later reader does not follow it
out, and the outside canary stays byte-identical (`write.rs:748`,
`:754`, `:766`). `a_write_follows_a_directory_link_that_stays_inside`
(`write.rs:779`) carries clause F, openSUSE's own shape: a directory link
that stays inside is followed and the drop-in lands at its target
(`write.rs:775`, `:781`, `:787`).

**The deletion condition, and what clears it.** The script is deletable
when the plan's library-test row lands. That row sits at
`refactor/recon-c.md:110`, and it reads
`Write the two library tests into src/write.rs of pod-rootfs (PodUtils/podcrates) mod
tests (:640)`. Its id is a plan id, not a tracked entry: no `TODO/*.md`
carries a heading for it. The only place that id appears in the tracked
tree is the two doc comments in `write.rs` (`:748`, `:775`) that name the
tests it owns. The plan's own row for this record is
`refactor/recon-c.md:177`, and it makes that row the clearing condition.

⛔ **The condition has not landed, and the tree disagrees with the plan's
wording.** The two tests are present and committed, so the library half of
the plan's row is done. The plan's conversion for this script is wider.
`refactor/01-audit/group-4.md:290` asks for a reachability
integration test in `crates/podvm/tests/`, to drive the four doors
through the shipped binary. The plan names a target file for it at
`group-4.md:294`. That file was never written: the directory git tracks
holds `curated_refusals.rs`, `detached_stdio.rs`, `qol.rs`, and
`store_gates.rs`, and no file of that name exists under `crates/`.
Two further library tests the plan names at `group-4.md:286` and `:288`,
`the_link_is_replaced_and_the_file_outside_is_untouched` and
`a_dangling_directory_link_is_refused_and_reported`, are also absent.

So the script is **partly converted**, and the deletion is blocked by
more than the plan's row states. The library doors are answered at
`write.rs`; the shipped-binary reach that the script's own header at
`:10-15` calls necessary is not, and that header's reason is that a
containment check right in the library and unreached by `podvm run` is
a check nobody has (`group-4.md:272-278`). Deleting the script now drops
the only drive of the reach path. The deletion clears when the plan's
library-test row and its reachability half land as tracked entries. Until
then this row is the record, and no entry deletes the script.

**Retired 2026-11-11.** T-1735 met the deletion condition: the reachability test `crates/podvm/tests/completion_reach.rs` drives the six doors through the shipped binary, and `pod-rootfs` at `v0.22.0` holds the dangling directory link (`a_write_through_a_symlinked_directory_is_refused`, `reach_tells_absent_from_a_link_inside_from_a_link_out`). The script is deleted.

## Done

**Done 2026-10-02.** Recorded the converted state of
the experiment 85-completion-symlink-escape.sh (retired by T-1735) and the condition that
clears its deletion. The six doors the script plants are carried by two
tests in `src/write.rs` of `pod-rootfs` (PodUtils/podcrates):
`a_write_replaces_an_out_link_with_a_regular_file` (`write.rs:752`) for
clauses A to E, and
`a_write_follows_a_directory_link_that_stays_inside` (`write.rs:779`)
for clause F. The premise that the plan's row at
`refactor/recon-c.md:110` is the whole conversion does not hold: the two
library tests named at `refactor/01-audit/group-4.md:286` and line 288 at `c36e0db672d2` are
absent, and the reachability integration test asked for at
`group-4.md:290` was never written. The record stands on that correction.
The script is only partly converted, and the deletion stays blocked
until the plan's library-test row and its reachability half land as
tracked entries. `grep -n "85-completion-symlink-escape" TODO/cli.md`
exits 0 and reports 6 matches.

# T-1649: A `#!` entrypoint needs an undocumented `--entrypoint`, and the refusal must name the way out

**Source:** issue 92; issue 114 section 3.3 and section 6 item 7;
  `crates/podvm/src/lifecycle/script.rs:105`;
  `src/memfd.rs` of `pod-enter` (PodUtils/podcrates);
  `crates/podvm/src/run/prepare.rs:237`;
  A-V-04 of PodUtils/podcrates#39
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

On a chroot-denied host this run exits 126:
`run node:alpine node -e 'console.log(1)'`. The entrypoint
`docker-entrypoint.sh` is a `#!` script, and its interpreter
resolves on the host root. The caller gave a command. The
way out, `--entrypoint node`, exists, and the refusal does
not name it, so a docker user cannot find it.

## Premise

`run.rs:1236` builds argv from the image Entrypoint and Cmd,
the caller's command and `--entrypoint`, by docker's rule.
The image script stays the program and the caller's words
become its arguments. `decide_entry` refuses a script at
`lifecycle.rs:1879`. The message (`memfd.rs:199`, T-1413)
names the path and the `#!` line but no remedy.

## Approach

Where the refused script is the image's own Entrypoint and
the caller gave a command, the refusal adds: retry with
`--entrypoint` followed by the caller's first word. For a
top-level `#!` payload, resolve the interpreter inside the
rootfs and run it on the loader family with the script path
as its argument. T-1646 does the same for a child.

## Decision

docker's argv rule stays: podvm does not drop the image
Entrypoint on its own. The refusal names the remedy. Where
the interpreter resolves inside the rootfs, the script runs
and no refusal prints.

## Prove

`cargo test -p podvm` exits 0 with a test: the script
refusal for an image Entrypoint plus a caller command
contains `--entrypoint` and the caller's first word.
`sh experiments/356-no-chroot-rung.sh` exits 0 with clause 13
changed: a `#!/bin/sh` script whose interpreter is in the
image runs at 0, and the node image run with a caller
command either reaches the payload or refuses naming
`--entrypoint`.

## Correction

**Filed 2026-11-06.** Quoted from issue 92 and its comments, measured on
release 0.1.0-beta.12 on the errand host:

```text
$ podvm run node:alpine node -e 'console.log(1)'
podvm run: ... the payload .../rootfs/usr/local/bin/docker-entrypoint.sh
is a #! script (#!/bin/sh), whose interpreter path resolves on the host
root without a chroot (TODO/enter.md T-1413)                  [exit 126]
$ podvm run --entrypoint node node:alpine -e 'console.log(1)'
... node is 149234536 bytes, over the 134217728-byte ceiling  [exit 125]
on a noexec mount, a #! script run from a memfd fd:
$ /state/home/noexec-test/t.sh
bash: /state/home/noexec-test/t.sh: /bin/sh: bad interpreter: Permission denied
$ /tmp/sandhome-memexec /state/home/noexec-test/t.sh
HI
```

**Amended 2026-11-09.** For T-1649, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is one step of that chain: the entrypoint of the
node image is a `#!` script (T-1661). Its clauses do not change. T-1646
holds the end-to-end clause.

## Done

**Done 2026-11-10.** For T-1649, the last clause of the partial record below is run. Experiment 356 exits 0 in full on 2026-11-10 ([the run](../experiments/results/no-chroot-rung.txt); T-1648 in `TODO/enter.md` records its inputs).

**Partial, 2026-11-10.** On the no-chroot path `decide_entry` follows
a `#!` payload through `script::follow` (`lifecycle/script.rs`): it reads
the `#!` line as the kernel does, resolves the interpreter inside the
rootfs (four levels), and runs the kernel's argv `interp [arg] script
args` on the family of the interpreter. An interpreter outside the image
refuses at 126, never on the host. Where the image Entrypoint is the
refused script and the caller gave a command, the refusal adds "retry
with `--entrypoint <first word>`". Both `exec` paths apply the
interposer after the decision and run the same argv. `cargo test
--workspace` exits 0 (989 passed) with
`an_entrypoint_script_refusal_names_the_entrypoint_flag`. [The drive](../experiments/results/script-entry-1649.txt):
`run node:22-alpine node -e 'console.log(1)'` prints 1 at 0 on the
chroot-denying fixture; an image without `/bin/sh` refuses at 126 with
the retry line. Clause 13 of experiment 356 holds it. Remaining: one full run of the committed `sh experiments/356-no-chroot-rung.sh` (its pulls from `public.ecr.aws` answered HTTP 429 on 2026-11-09 and 2026-11-10; a copy with `mirror.gcr.io` images passed each step of the clause).

# T-1650: `--strict` does not refuse the userland grandchild limit, while `--strict=all` refuses every run

**Source:** issue 93; issue 114 section 3.7 and section 6 item 8;
  `crates/podvm/src/complete/strict.rs:129`;
  `crates/podvm/src/complete/strict.rs:174`;
  `crates/podvm/src/lifecycle/entry.rs:451`; [T-1415](cli.md)
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

No strictness level means "refuse a rung that cannot run
this command". `run --strict` of `sh -c '/bin/ls /'` warns,
then the shell fails `not found` at 127. `run --strict=all`
of `/bin/echo hi` refuses at 125 over five substitutions: the
namespace share and `dev/null`, `zero`, `random`, `urandom`.

## Premise

`classify_strict` (`complete.rs:251`) has two kinds, safety
and substitution. Where chroot is denied the userland rung
is a substitution (`complete.rs:296`), by T-1415's decision.
No reason names a capability gap of the selected family. The
exec gap appears only in the banner (`lifecycle.rs:1984`).

## Approach

Add a third kind, capability, for the gaps of the selected
family: a dynamic grandchild cannot exec (T-1646), a guest
object cannot load (T-1645), a relative path cannot resolve
(T-1643), a static payload cannot be reached (T-1648). A gap
leaves the list in the change that fixes it. `--strict`
refuses on a capability gap the payload will hit: a shell
given `-c` or a script runs children. Where podvm cannot
tell, it warns. The refusal names the gap and a rung that
runs the command, once T-1651 or T-1652 provides one.
`--strict=all` keeps its rule for substitutions.

## Decision

T-1415 stays: the userland rung alone is a warning where no
better rung exists. A gap that decides whether the command
can run at all is safety-relevant, and `--strict` refuses
it.

## Prove

`cargo test -p podvm` exits 0 with `classify_strict`
tests for the capability kind. `sh experiments/356-no-chroot-rung.sh`
exits 0 with a strict clause on the fixture: `run --strict`
of `sh -c '/bin/ls /'` exits 125 naming the exec gap, or 0
with the listing once T-1646 is done, and `run --strict` of
`/bin/echo hi` exits 0.

## Correction

**Filed 2026-11-06.** Quoted from issue 93, measured on release
0.1.0-beta.12 on the errand host with the exit codes read unpiped:

```text
$ podvm run --strict alpine:3.20 /bin/sh -c '/bin/ls /'
podvm run: --strict warning [substitution]: unchanged dev/random ...
sh: /bin/ls: not found
[exit 127]                       # strict did not refuse the run
$ podvm run --strict=all alpine:3.20 /bin/echo hi
... --strict=all refused ... [substitution] unchanged dev/zero ...
[exit 125]                       # refuses a run that would have worked
```

## Done

**Done 2026-11-10.** For T-1650, the last clause of the partial record below is run. Experiment 356 exits 0 in full on 2026-11-10 ([the run](../experiments/results/no-chroot-rung.txt); T-1648 in `TODO/enter.md` records its inputs).

**Partial, 2026-11-10.** `--strict` knows a third kind, a capability gap
of the loader family (`complete/capability.rs`). The list holds what is
true now, each gap with its entry: the routes T-1646 does not take
(`fexecve`, `execveat` with flags, `system()` and `popen()`, a foreign
child ELF; podvm cannot tell, so a warning), a static child on the host
root (T-1652; refused where a shell `-c` string or an entrypoint
argument names a static ELF of the image), and the raw `statx` of
libuv (T-1697; refused for npm, npx, yarn, pnpm, corepack and node given
a script). A test fails while a listed entry is `done`. `--strict=all`
keeps its rule. `cargo test --workspace` exits 0 with
`strict_refuses_a_capability_gap_that_the_payload_hits`. [The drive](../experiments/results/strict-capability-1650.txt):
issue 93's `--strict sh -c '/bin/ls /'` now runs at 0 (T-1646 routes the
grandchild), `--strict npm --version` refuses at 125 naming T-1697, and
`--strict=all /bin/echo hi` refuses at 125. Clause 24 of experiment 356
holds it. Remaining: one full run of the committed `sh experiments/356-no-chroot-rung.sh` (its pulls from `public.ecr.aws` answered HTTP 429 on 2026-11-09 and 2026-11-10; a copy with `mirror.gcr.io` images passed each step of the clause).

# T-1654: Profiles: strict TOML recipes with `extends` and `-s` overrides

**Source:** issue 166; #193; the owner's ruling of 2026-11-06
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

podvm has the docker and podman command line and a tier flag,
but no recipe layer. A caller cannot name a sandbox shape once
and run it again. sand-net ran 43 shapes from TOML recipes, and
each command line ran unchanged in every shape.

## Premise

The parity table is the only description of what `run`
accepts (`crates/podvm/src/parity.rs:209-277`). `--podvm-tier`
takes `machine` or `chroot` (`crates/podvm/src/tier.rs:22`).
Resource flags refuse by name (`parity.rs:341`, T-0804). The
identity calls are virtualised (T-0711, `parity.rs:338`). No
source reads a profile file, and no verb lists one.

## Approach

Add `podvm profile list` and `podvm profile show NAME`, and
`--profile NAME` on `run`. A profile is TOML with the tables
`fs`, `limits`, `net` and `env.set`. `extends` merges the
child's tables over the parent's. `-s key=value` sets one leaf
and repeats; the value parses as TOML and falls back to a
string. Built-in profiles live in the binary; user profiles
live under the XDG config directory. Map `fs.backend` to the
tier, `fs.user` to the identity of T-0711, `fs.wrap` to an
entrypoint prefix, and `limits.file_max` to `RLIMIT_FSIZE`.
`limits.memory` and `limits.cpus` keep the T-0804 refusal on a
rootfs tier and become guest RAM and vCPUs on the machine tier.

## Decision

An unknown key and a wrong type are errors that name the key:
"a misspelt key that is silently ignored leaves a cage looser
than the recipe says" (issue 166). A profile that names a
mechanism the probe reports absent refuses with the probe's
reason, at load time where the host is known, else at run
time. `--strict` keeps its meaning. On a host with no mount,
`/workspace` and `/state` are path-map entries (T-1653,
T-1652), not bind mounts.

## Prove

`cargo test -p podvm` exits 0 with tests: an unknown key and a
wrong type each refuse and name the key, a planted misspelt
key is refused, and `extends` plus `-s` give the expected
effective profile. `sh experiments/410-profiles.sh` exits 0:
`podvm profile show NAME -s k=v` prints the effective profile,
a profile that asks for an absent tier refuses with the
probe's reason, `sh -c 'pwd; ls /workspace'` answers the same
in every built-in profile that places `/workspace`, and
`limits.memory` refuses by name on a rootfs tier.

## Correction

**Filed 2026-11-06.** The facts below are quoted from issue 166, so the
entry stands if GitHub hides the issue. sand-net is the 0BSD tool that
issue 193 describes; its source is deleted.

```text
tables:   fs (backend, profile, read, write, execute, path_extra, disk_tmp,
          as_root, image, runtime, user, wrap, persist, rw_root),
          limits (file_max, tmp_size, shm_size, memory, cpus, pids),
          net (mode, ports, allow, auth, resolv, netns), env.set
defaults: backend bailey, net.mode broker, ports [443], tmp 256m, shm 250m,
          file_max 1g
-s 'net.ports=[80,443]'   -> a list (parsed as TOML)
-s net.mode=open          -> a string (TOML parse failed)
validation: broker and proxy-net need bailey; gvisor, bwrap, landrun take
          none, open, netns; microvm none and netns; ports 1 to 65535
F30: landrun has no mount namespace, so /workspace did not exist and
          `sand run landrun-none -- sh /workspace/probe.sh` failed
F24: a 0700 project made a rootless keep-id container fail to stat it
```

## Done

**Done 2026-11-11.** `cargo test -p podvm` exits 0 with the tests of
`crates/podvm/src/profile/tests.rs`: `an_unknown_key_refuses_and_names_the_key`,
`a_wrong_type_refuses_and_names_the_key`, `a_planted_misspelt_key_is_refused`
and `extends_and_s_give_the_effective_profile`, beside the cycle of
`extends`, each built-in profile, and `limits.memory` and `limits.cpus` on a
rootfs tier. `PODVM_410_MIRROR=1 sh experiments/410-profiles.sh` exits 0
(`experiments/results/profiles.txt`), each of its eight clauses:

| clause | reading |
| --- | --- |
| `profile list` | exit 0, the five built-in profiles |
| `profile show workspace -s fs.user=1000 -s limits.file_max="1g"` | `user = 1000`, `file_max = "1g"`, and the flags `--user=1000` and `--ulimit=fsize=1073741824` |
| an absent tier | `--profile incus`: exit 125, "no incusd answers"; `--profile machine` with `PODVM_MACHINE_ACCEL=kvm`: exit 125, "/dev/kvm does not open" |
| `sh -c 'pwd; ls /workspace'` | `/workspace` and `probe.txt` in workspace, readonly (on the chroot-denying fixture) and machine (a TCG guest) |
| `limits.memory` on a rootfs tier | exit 125, "`limits.memory` needs fs.backend = \"machine\"" |
| a misspelt key of a user profile | exit 125, "unknown key `fs.read_onyl`", before the pull |
| a user profile that extends `workspace` | its `env.set` reaches the command; `-s env.set.GREETING=...` overrides it |
| `-s` without `--profile` | exit 125 |

What podvm takes of issue 166, and what it leaves:

* The tables are podvm's own: `fs` (`backend`, `user`, `wrap`,
  `read_only`, `workspace`, `workdir`, `volumes`, `tmpfs`), `limits`
  (`file_max`, `memory`, `cpus`, `timeout`), `net` (`mode`) and
  `env.set`. A key of sand-net that has no mechanism in podvm is not
  accepted: it is an unknown key that names the keys of its table, which
  is the rule of the Decision. `net.mode` takes `host` only until the
  network entries T-1662 to T-1665 add the other modes.
* A profile becomes `run` flags that the one parser of `run` reads
  (`podvm profile show` prints them), so a profile can do nothing that a
  flag cannot, and a flag of the command line wins over the profile.
  `fs.backend` is `--podvm-tier`; `fs.workspace` is `-v HOST:/workspace`,
  a path-map entry of the loader family (T-1653) and a copy on the machine
  tier; `limits.file_max` is `--ulimit=fsize`; `limits.memory` and
  `limits.cpus` are `--podvm-mem` and the new `--podvm-cpus` on the
  machine tier and refuse on a rootfs tier.
* `--podvm-cpus` gives `-smp`; a guest of more than one CPU keeps ACPI,
  because with `acpi=off` the kernel saw one CPU (measured: `nproc` 1,
  then 2).
* User profiles are `$XDG_CONFIG_HOME/podvm/profiles/NAME.toml` (else
  `~/.config/podvm/profiles`); a user file takes the place of a built-in
  profile of the same name. A name takes letters, digits, `-`, `_` and
  `.` only.
* T-1728 came from clause 4: the machine tier gave the command a terminal,
  and `ls` coloured its output.

# T-1655: The sandbox hardening line: each `run` flag works on the tier or refuses by name

**Source:** issue 167; #193; the owner's ruling of 2026-11-06;
  A-V-07, G-12, G-06 of PodUtils/podcrates#39
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

sand-net ran podman and docker with one fixed hardening line.
podvm refuses most of that line with status None, so the same
command cannot run under podvm. Some of the refused flags have
an unprivileged equivalent that podvm does not use.

## Premise

The parity table refuses `--read-only`
(`crates/podvm/src/parity.rs:257`), `--tmpfs`
(`crates/podvm/src/parity.rs:259`), `--ulimit`
(`crates/podvm/src/parity.rs:274`), `--cap-add, --cap-drop`
(`crates/podvm/src/parity.rs:227`), `--security-opt`
(`crates/podvm/src/parity.rs:278`), `--runtime`
(`crates/podvm/src/parity.rs:277`) and `--userns`
(`crates/podvm/src/parity.rs:275`). `-u, --user` is Degraded
through the interposer identity
(`crates/podvm/src/parity.rs:225`). `setrlimit`,
`PR_SET_NO_NEW_PRIVS` and a seccomp filter need no privilege;
the probe already sets `PR_SET_NO_NEW_PRIVS` in a child
(`pod_hostprobe::probes`).

## Approach

Write an experiment that runs the sand-net line through
`podvm run` and records each flag as passed or refused by
name. Then implement the unprivileged equivalents. `--ulimit
fsize` calls `setrlimit` before the exec. `--read-only` makes
a write outside the named writable paths fail with `EROFS` in
the interposer and in the T-1652 path layer. `--cap-drop ALL`
and `--security-opt no-new-privileges` set
`PR_SET_NO_NEW_PRIVS`. `--tmpfs` with a size becomes a store
directory under a size check. `--memory`, `--cpus` and
`--pids-limit` keep the T-0804 refusal on a rootfs tier and
set guest resources on the machine tier. Each parity row moves
with its change.

## Decision

A flag works on the selected tier or refuses with a reason;
nothing in between. A flag that a path layer enforces is
Degraded, and its note says which layer, because the
interposer is not a boundary. `--userns keep-id` stays None:
there is no user namespace, and identity is virtualised, not
mapped. An image record carries its input digest, not a
timestamp (F33).

## Prove

`cargo test -p podvm` exits 0 with parity tests for each
changed row. `sh experiments/411-hardening-line.sh` exits 0 on
a chroot-denied fixture: every flag of the line passes or
refuses by name, `--read-only` gives `EROFS` for a write to `/`
while `/workspace` and `/tmp` stay writable, `--user 993:990`
reports 993 from `id -u` and writes the project, a write past
`--ulimit fsize` fails with `EFBIG`, and `--memory 128m`
refuses by name on a rootfs tier, `--ulimit nofile=64` shows in
`ulimit -n` in the payload, and `--timeout 2` on `sh -c 'sleep 100 &
wait'` ends the whole tree, with each child gone, within 60 s.

## Correction

**Filed 2026-11-06.** Quoted from issue 167. The host was Debian 13 in an
unprivileged Incus container, podman 5.4.2, docker 26.1.5, with egress
only through a loopback HTTP proxy:

```text
run --rm --cap-drop ALL --security-opt no-new-privileges --read-only
  --tmpfs /tmp:rw,nosuid,nodev,size=N --tmpfs /dev/shm:rw,nosuid,nodev,size=N
  --ulimit fsize=N -v PROJECT:/workspace -v STATE:/state
  -v RESOLV:/etc/resolv.conf:ro -w /workspace --network none|host|pasta|slirp4netns
  --http-proxy=false --userns keep-id[:uid=U,gid=G] --entrypoint CMD IMAGE ARGS
F13: rootless podman refused every cgroup limit ("rootless needs no limits +
     no cgrouppath when no permission is granted for cgroups")
F28: --user U:G without a mapping made the project read-only for 993, 1000, 0
F26: pasta printed --version and then failed "Couldn't set IPv4 route(s) in
     guest: Invalid argument" beside a linkdown route
F33: a cache-hit rebuild keeps Created, so a stale image looked current
probe matrix vs the bailey baseline: podman-none 307 same / 92 differ,
     docker-none 305 / 86, most in fs (54)
```

**Amended 2026-11-09.** The asks A-V-07, G-12 and G-06 of
PodUtils/podcrates#39 (COORD-015, a message from a peer session) add four
clauses. First, `--ulimit` takes `nofile`, `nproc`, `cpu`, `as`, `core` and
`fsize`, each set with `setrlimit` before the exec. A name that the host
denies refuses by name. Second, `--timeout SECONDS` (podman's spelling) ends
the whole process tree of the payload, with SIGTERM and then SIGKILL, and it
reaches a child that left the process group. Third, the run measures the
peak resident set and the CPU time of the tree from `wait4` and gives them
to the report of T-1695. Fourth, a run that ended by timeout or by `stop` is
gone within 60 s: its record and its store directory are removed (G-12). The
limits come from one source, the profile of T-1654, and the flags amend it.
The row of `--ulimit` in the parity table moves with the change.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC section 10.4, its measurements M1 and M2) adds four clauses.
First, the mechanism of the `--timeout` clause and of the 60 s clause is
T-1708 (the launcher as a subreaper); this entry takes the flags. Second,
`--ulimit fsize` below 2 TiB kills a .NET payload that runs with the
default `DOTNET_EnableWriteXorExecute`: the runtime calls `ftruncate` on a
memfd with 2 TiB, then gets SIGXFSZ and exits 153 (M1: under 20 GiB, 1
GiB, 128 MiB and 1 MiB). The parity note of `--ulimit` and the profiles of
T-1654 say so, and the experiment adds a .NET case (T-1706). Third, the
read-only layer serves a `:ro` volume beside `:rw` volumes and `:rw`
volumes nested in a `:rw` volume, as the runner's job-container table
needs (`/__e` read-only beside `/__w` and `/__w/_temp`, T-1711); the Prove
adds that table. Fourth, the Premise cited parity lines that moved; they
are corrected in place.

**Amended 2026-11-11.** `Plan` of `pod-enter` has no field for a resource
limit or for `no_new_privs` at `v0.15.0`, and podvm cannot set either in
its own process before the fork (an `fsize` limit would bind podvm's own
writes, and `PR_SET_NO_NEW_PRIVS` cannot be undone). The child-side part
(`Plan::limits`, `Plan::no_new_privs`, set after the `chdir` and before
the exec in each entry, a refused call named by its step) is asked of
podcrates by COORD-027 (issue 245). The `--timeout` clause waits for
T-1708 (podcrates podcrates@T-162). The parts in podvm alone: experiment 411, the
parse and the parity rows of each flag, the `--read-only` layer of the
interposer with the `:ro` volumes, and `--tmpfs` as a store directory
under a size check.

## Done

**Done 2026-11-11.** Each flag of the line works on the rung that podvm
enters or refuses by name; the partial records below give each part:
`--timeout` (the wall bound), `--ulimit` and `--security-opt
no-new-privileges` (podcrates `v0.17.0`), `--cap-drop ALL`, the read-only
layer of `--read-only` and `:ro`, and `--tmpfs` (a directory of a
foreground run; a detached run refuses it by name). `--network`,
`--userns`, `--memory`, `--cpus` and `--pids-limit` refuse by name. The
Prove, on the cloud host, 2026-11-11: `cargo test --workspace` exits 0
with the parse and parity tests of each changed row, and the interposer
tests (75) with the read-only layer and the job-container table of the
runner (`/__e` read-only beside `/__w` and `/__w/_temp`).
`PODVM_411_MIRROR=1 sh experiments/411-hardening-line.sh` exits 0 on the
chroot-denied fixture ([the run](../experiments/results/hardening-line.txt)):
each flag alone works or refuses by name; the line gives `EROFS` for a
write to `/` while `/workspace` and `/tmp` take writes, `EFBIG` past
`--ulimit fsize`, and `NoNewPrivs: 1` in the payload; `--user 993:990`
prints 993 and writes the project; `--memory 128m` refuses by name;
`--ulimit nofile=64` shows in `ulimit -n`; `--timeout 2` ends `sh -c
'sleep N & wait'` in 2 s with no child left. Two clauses of the
amendments ride with their own entries: the .NET case with SIGXFSZ under
a small `fsize` is part of T-1706 (the help and the row of `--ulimit`
name it), and the container-hook table of the runner is T-1711. The
measures of the end (peak resident set, CPU) are in `.End` (T-1708) for
the report of T-1695.

**Partial, 2026-11-11.** `--timeout SECONDS` works (podman's spelling,
whole seconds over 0): a detached run sets the wall bound of its launcher
(`set_wall` of `pod-supervise` v0.16.0), and a foreground run arms a timer
thread that kills each descendant of podvm with SIGKILL at the bound
(`crates/podvm/src/run/wall.rs`); the end reads `wall`, and the parity row
is Native. Drive on the cloud host, 2026-11-11, the pinned alpine digest:
a foreground `run --rm --timeout 2` of `sh -c 'sleep N & sleep M & wait'`
ends at 2 s with 137 and leaves no process; the same under `run -d` reads
`exited wall 2001 0` in `inspect --format '{{.State}} {{.End.Reason}}
{{.End.RunMs}} {{.End.Residual}}'`; `--timeout 0` exits 125.
`cargo test -p podvm` exits 0 with `the_timeout_takes_whole_seconds_over_zero`.
`--ulimit` and `--security-opt no-new-privileges` work since podvm takes
podcrates `v0.17.0` (`4da8fb3`, its podcrates@T-176, PodUtils/podcrates#58):
`podvm_enter::ulimit` reads docker's `NAME=SOFT[:HARD]` (`-1` for no
limit), the run puts the limits and the bit in `Plan::limits` and
`Plan::no_new_privs`, and a detached run keeps them in its record
(`Detail::hardening`) for the launcher. Each other `--security-opt` is
refused by name. Drive on the cloud host, 2026-11-11, the pinned alpine
digest: `--ulimit nofile=64` reads 64 from `ulimit -n` in a foreground run
on the namespace rung and on the chroot-denied fixture (userland), and 77
in a detached run; with `--ulimit fsize=1024` and SIGXFSZ ignored, `dd` of
4096 bytes fails with "File too large" on both rungs; the payload of a
detached run with `--security-opt no-new-privileges` has `NoNewPrivs: 1`
and 0 without it (namespace rung; the fixture sets the bit for its own
filter); `--ulimit nofile=99999999:99999999` refuses with
`setrlimit(RLIMIT_NOFILE): EPERM`. `cargo test -p podvm -p podvm-enter`
exits 0 with `a_ulimit_reads_as_docker_writes_it` and
`the_limits_of_the_hardening_line_parse`; experiment 355 and
`curated_refusals` refuse 38 flags now. `--cap-drop ALL` sets the same
bit (row Degraded: podvm does not change the capability sets), a named
capability and `--cap-add` are refused by name (`cap_drop` in
`crates/podvm/src/run/parse/values.rs`). `--memory`, `--cpus` and
`--pids-limit` stay refused by name on a rootfs tier. The read-only layer
works: `PODVM_RO` (`ro:PREFIX`/`rw:PREFIX`, the longest prefix decides)
makes the interposer answer `EROFS` for a write-class call, an `open` that
writes and an `fopen` mode that writes under a read-only guest path
(`crates/podvm-interpose/src/map/readonly.rs`); `--read-only` gives
`ro:/` with `rw:/dev` and an `rw:` entry for each writable volume, and a
`-v HOST:GUEST:ro` volume gives `ro:GUEST` (`read_only_env` in
`crates/podvm/src/lifecycle/volume.rs`). Each entry with no interposer
table refuses both by name; the rows are Degraded, because a raw system
call passes. Interposer tests: 75 passed, with
`a_write_under_a_read_only_prefix_answers_erofs`; experiment 356 clauses
26 and 30 drive both. Remaining:
`--tmpfs`, the guest resources of the machine tier (T-1656), the report of
T-1695, and experiment 411.

# T-1659: Provider shapes and bare userlands as built-in profiles

**Source:** issue 172; #193; the owner's ruling of 2026-11-06
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

A caller cannot run a tool in the shape of its production
host: the provider's user, limits and variables. sand-net had
seven such shapes. podvm pulls each image, and has no profile
to put the shape around it.

## Premise

Profiles arrive with T-1654. The identity calls are virtualised
(T-0711), so `id -u` can answer 993 while the real owner of the
project stays the caller. Resource flags refuse by name on a
rootfs tier (`crates/podvm/src/parity.rs:228`, T-0804). The
Lambda and Vercel images are glibc, which the rootfs tier
declines today (T-1644).

## Approach

Ship built-in profiles: `lambda-al2023`, `cloudflare-sandbox`,
`vercel-sandbox`, `busybox-bare`, `distroless-static` and
`node-shell`, each with the user, environment and limits of
issue 172. Memory and pids limits run on the machine tier or
refuse by name. A read-only root and a sized `/tmp` use the
T-1655 path layer. Each profile prints what it does not
model. A profile that needs a missing tool, curl for example,
detects it and says so.

## Decision

A shape is a profile, not a tier. podvm never infers success
from a zero exit of a pipeline: `curl ... | sh` with no curl
exits 0 and installs nothing. `RLIMIT_NPROC` and
`RLIMIT_FSIZE` are partial stand-ins and the profile labels
them so. The Alpine and busybox shapes come first; the glibc
shapes follow T-1644.

## Prove

`cargo test -p podvm` exits 0 with a test that every built-in
profile loads and lists what it leaves out.
`sh experiments/415-provider-shapes.sh` exits 0: the
`lambda-al2023` profile prints 993 for `id -u`, prints
`AWS_LAMBDA_FUNCTION_NAME`, gives a read-only error for
`touch /x` and leaves the project writable; `busybox-bare`
runs `wget`; `distroless-static` runs a static binary and
refuses `sh -c true` by name; the memory limit refuses by name
on a rootfs tier.

## Correction

**Filed 2026-11-06.** Quoted from issue 172:

```text
lambda-al2023  public.ecr.aws/lambda/provided:al2023  user 993:990, memory 128m,
               /tmp 512m, pids 1024, read-only root, AWS_LAMBDA_FUNCTION_NAME,
               AWS_REGION=us-east-1, LAMBDA_TASK_ROOT=/var/task, TZ=:UTC
vercel-sandbox amazonlinux:2023, user 1000:1000, USER=LOGNAME=vercel-sandbox
busybox-bare   busybox:musl; wget prints "TLS certificate validation not
               implemented", and expired, self-signed and wrong-host
               certificates all exit 0
distroless     gcr.io/distroless/static-debian12; `sh -c true` fails to start,
               a static binary placed in the project runs
install by `curl -fsSL URL | sh`: node-shell, cloudflare-sandbox and
               busybox-bare had no curl, and the pipeline exited 0 with
               nothing installed
```

## Done

**Done 2026-11-11.** The last clause, `lambda-al2023`, runs. podcrates
`v0.24.0` applies a layer whose root entry has an empty name (`pod-extract`,
`safety::is_root_entry`, the answer to COORD-042, issue 261), and podvm
takes the tag (T-1679). The run then met the image's entrypoint,
`/lambda-entrypoint.sh`, which wants a handler name: that script is
Lambda's invocation protocol, which the profile leaves out. A new key
`fs.entrypoint` gives `run --entrypoint=VALUE`, and `lambda-al2023` sets it
to `""`, so the command runs as the program
(`crates/podvm/src/profile/model.rs`). `PODVM_415_MIRROR=1 sh
experiments/415-provider-shapes.sh` exits 0 on the cloud host: clause 1
prints 993 for `id -u` and `fn=podvm`, `touch /x` fails with "Read-only
file system", and the project is writable; clauses 2 to 7 hold.
`cargo test -p podvm` exits 0 with
`a_provider_shape_names_its_image_and_its_identity`, which now asserts the
cleared entrypoint.

**Partial 2026-11-11.** `cargo test -p podvm` exits 0 with
`each_built_in_profile_checks_and_gives_its_flags` (each built-in profile
loads and lists what it leaves out), `a_provider_shape_names_its_image_and_its_identity`
and `a_profile_with_an_image_takes_the_words_as_the_command`.
`PODVM_415_MIRROR=1 sh experiments/415-provider-shapes.sh` exits 0 on the
chroot-denying fixture (`experiments/results/provider-shapes.txt`):
`busybox-bare` has `wget`; `distroless-static` refuses `sh -c true` by
name (exit 127) and runs a static program of the project; `vercel-sandbox`
(the glibc amazonlinux:2023) prints 1000 and `USER=vercel-sandbox`;
`cloudflare-sandbox` answers `touch /x` with "Read-only file system" and
leaves the project writable; `-s limits.memory=128m` and `-s
'needs=["curl"]'` refuse by name (exit 125); each of the eleven built-in
profiles lists what it leaves out.

The parts of the build: a profile names its image (`fs.image`), and each
positional word of the command line is then the command; `--` ends
podvm's flags; `leaves_out` and `needs` are top-level keys; `needs`
refuses a run whose image lacks a program on its PATH, so `curl ... | sh`
is never trusted to an exit code. A static program in a volume now takes
the native-static rung (its guest path maps back through the volume).

Remaining clause: `lambda-al2023`. `pod-extract` refuses a layer of
`public.ecr.aws/lambda/provided:al2023` whose root directory entry has an
empty name; COORD-042 (issue 261) asks podcrates for the fix of that
frozen code. The clause runs again with the tag that has it.

# T-1660: `podvm notebook`: Jupyter and marimo cycles that keep one directory

**Source:** issue 174; #193; the owner's ruling of 2026-11-06
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The owner names notebooks in the scope. podvm has no verb that
runs a notebook, and no model of a hosted session that wipes
its root on restart and keeps one directory.

## Premise

`run --rm` starts from the stored rootfs, so a fresh root for
each cycle costs nothing. A kept directory needs a volume,
which a host with no mount cannot give (T-1653); `cp` copies
in and out (T-1323, `crates/podvm/src/parity.rs:155`). The
supervisor holds one pidfd for the direct child
(`pod_supervise::launcher` of PodUtils/podcrates, T-0601) and
starts a session (`setsid` in the same launcher). A Jupyter kernel is a
grandchild, which T-1646 must reach. python:3.13-slim is
glibc (T-1644).

## Approach

Add `podvm notebook NB --persist DIR [--cycles N] [--kill-after
S] [--restart-kernel-after CELL]`. Each cycle is a new
container with a read-only root, a `/workspace` that holds
only the notebook, a new kernel and `DIR` at `/persist`. Print
each output line as `[N] text` and exit 1 when a cell raised.
`--kill-after` ends the cycle with 143 and kills the whole
process tree through the session. A `.py` notebook runs
through `marimo export html`. Use an Alpine Python image
until T-1644 admits glibc.

## Decision

The kept directory is the only state that crosses a cycle.
Two concurrent notebooks share the host network on a rootfs
tier, so each picks its own loopback ports; the banner says
the ports are not private. A failed `!` line does not raise,
so the verb does not report it as a failure.

## Prove

`cargo test -p podvm` exits 0 with tests for the argument
parse and the `[N]` line format.
`sh experiments/416-notebook-cycles.sh` exits 0: three cycles
keep only the `--persist` content, `--restart-kernel-after 1`
loses variables and keeps `/persist`, `--kill-after 5` on a
sleeping cell exits 143 and leaves no process, and a marimo
file exports to HTML.

## Correction

**Filed 2026-11-06.** Quoted from issue 174:

```text
jupyter cage:  jupyter nbconvert --to notebook --execute --stdout --log-level=ERROR NB
marimo cage:   marimo export html --no-include-code NB.py
sand notebook NB --persist DIR [--cycles N] [--lab LAB] [--kill-after S]
               [--restart-kernel-after CELL]... [--cage C]
NotebookCycles: only /persist survives a cycle and a kernel restart;
               variables do not survive the restart; /tmp does not survive
               the cycle; /workspace holds only the notebook
killing the client left the container running; leftover containers were
               found by their project volume and killed; exit 143
`curl -fsSL URL | sh` without curl: "curl: not found", exit 0, nothing installed
```

## Done

**Done 2026-11-11.** `cargo test -p podvm` exits 0 with
`the_arguments_parse_and_a_wrong_one_is_named`,
`each_output_is_a_line_of_its_cycle_and_an_error_raises` (the `[N]` line
format, and a failed `!` line that does not raise),
`a_restart_cuts_the_notebook_after_its_cell` and
`each_kernel_runs_with_a_read_only_root_and_only_persist_kept`. `sh
experiments/416-notebook-cycles.sh` exits 0 on the chroot-denying fixture
(`experiments/results/notebook-cycles.txt`): `--cycles 3
--restart-kernel-after 1` prints `[N] persist N tmp 1` and `[N] workspace
['c.ipynb']` for each cycle, `x kept False` after the restart with
`/persist` kept, and exits 1 for the raise; the control with no restart
keeps `x`; `--kill-after 5` on a sleeping cell exits 143 with no process
and no container tree left; `--kill-after 45` exits 143 after a kernel ran;
a marimo file exports to HTML that holds the output of its cell.

The verb: each cycle, and each part after a restart, is a new `podvm run
--rm` of the same binary with a read-only root, `--tmpfs /tmp`, a
`/workspace` with only the notebook and `--persist` at `/persist`; a
`.ipynb` runs through `jupyter nbconvert --execute --allow-errors
--stdout` of quay.io/jupyter/base-notebook, a `.py` through `marimo export
html` of ghcr.io/marimo-team/marimo. The images are glibc: the userland
rung runs them since T-1644, so no Alpine image is needed.

Two defects were found and fixed. The first exec of the loader family gave
the payload the host path of the loader's file as `argv[0]`, so the
`#!/opt/venv/bin/python3` of the marimo script lost its virtual
environment; the loader now gets `--argv0` with the invoked name, as the
interposer's exec route gives a child (`crates/podvm/src/lifecycle/argv0.rs`).
A run killed during the copy of its container tree left the partial tree;
a tree is now filled under `trees/.fill-PID-ID` and renamed when whole,
and each fill removes a partial tree whose process is gone.

# T-1661: Language runtime profiles: Node, Deno and Bun permissions, wasmtime and workerd

**Source:** issue 176; #193; the owner's ruling of 2026-11-06;
  A-V-05 of PodUtils/podcrates#39
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** partial

## Problem

For a language runtime with its own permission model, the
runtime is the boundary. podvm cannot start such a runtime
with its flags from one statement of the paths, and cannot say
which part the runtime enforces and which part podvm enforces.

## Premise

None of these runtimes needs a privilege. Today they hit filed
limits: the Debian images are glibc (T-1644), `node` in
node:alpine is over the read ceiling (T-1647), its entrypoint
is a `#!` script (T-1649), and `npm` spawns children (T-1646).
An `fs.wrap` prefix arrives with T-1654.

## Approach

Ship built-in profiles `node-permission`, `deno-permissions`,
`bun-runtime`, `wasmtime-wasi` and `workerd-test`. Generate the
runtime's flags (`--allow-fs-read`, `--allow-write`, `--dir`)
from the profile's `fs.read` and `fs.write`, so a profile
states each path once. Check the linkage of a host-style
binary before the rung is chosen. `profile show` prints the
runtime flags and podvm's path map side by side.

## Decision

The WASI profile needs no interposer and no tier: the module
sees only its preopened directories. Bun has no permission
model, and its profile says so. A runtime check never stands
in for podvm's own path map.

## Prove

`cargo test -p podvm` exits 0 with a test that one profile's
paths produce both the runtime flags and the path map.
`sh experiments/417-runtime-profiles.sh` exits 0 with the
fixtures of issue 176: `node-permission` prints `permission
on`, DENY for `/etc/hostname` and `/tmp`, ALLOW for
`/workspace`; `deno-permissions` prints `NotCapable` for the
outside read and for `Deno.env`; `wasmtime-wasi` prints `hello
from wasi`; `workerd-test` reports `typeof process =
undefined` and makes no connection: a `fetch` of a public URL from the
worker fails.

## Correction

**Filed 2026-11-06.** Quoted from issue 176:

```text
node-permission (Node 26): permission on; read /etc/hostname ERR_ACCESS_DENIED;
                 write /workspace/out.txt allowed; write /tmp/out.txt
                 ERR_ACCESS_DENIED
deno-permissions (Deno 2.9): NotCapable for reads outside /workspace, for
                 /tmp writes and for Deno.env (HOME)
bun-runtime (Bun 1.4): no permission model
wasmtime-wasi (wasmtime 50): "hello from wasi" with only /workspace
                 preopened and no sockets
workerd-test: typeof process = undefined; node:fs and node:child_process
                 cannot be imported; [ PASS ] main
F25: wasmtime and workerd publish no checksum; recorded by hash on first use
```

**Amended 2026-11-09.** The ask A-V-05 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a workerd profile with no network.
`workerd-test` gets no outbound service in its generated workerd config, and
`net.ports = []` for the confine layer of T-1658. Where Landlock is absent,
the profile refuses by name, as T-1658 says. The Chromium half of the ask is
T-1694.

## Done

**Partial 2026-11-11.** `cargo test -p podvm` exits 0 with
`one_profile_gives_the_runtime_flags_and_the_path_map` (the paths of
`node-permission` give both `node --permission --allow-fs-read=/workspace
--allow-fs-write=/workspace` and the `-v` of the path map),
`each_runtime_gets_its_flags_from_the_paths` and
`a_repeated_program_is_not_given_twice`. `PODVM_417_MIRROR=1 sh
experiments/417-runtime-profiles.sh` exits 0
(`experiments/results/runtime-profiles.txt`): on the chroot-denying
fixture, `node-permission` prints `permission on`, ERR_ACCESS_DENIED for a
read of `/etc/hostname` and a write of `/tmp/out.txt`, allows a write of
`/workspace/out.txt`, and `lstat` of `/workspace` answers a directory;
`deno-permissions` prints `NotCapable` for the read of `/etc/hostname`, the
write of `/tmp/out.txt` and `Deno.env`, and reads `/workspace`;
`bun-runtime` runs a script of `/workspace` in a TCG guest, and no emulator
stays; `profile show` prints the runtime's flags beside the run flags.

The parts of the build: the top-level key `runtime` (node, deno, bun,
wasmtime) and the keys `fs.read` and `fs.write`; the runtime and its flags
prefix the command, and a command that repeats the program (`node a.js`)
does not give it twice; a runtime profile clears the image's entrypoint
(`--entrypoint=`, which `run` now takes as docker does). Two defects were
found and fixed: the interposer answered `lstat("/workspace")` from the
rootfs, not from the volume (a mount point is never a link, so a call that
does not follow takes the volume's host path); and the guest's base archive
copied each hard link in full (busybox:musl took 500 MB).

Measured: on the userland rung, Bun read `/workspace/b.js` of the host's
own `/workspace`. Bun makes its own system calls, past the interposer, so
`bun-runtime` runs on the machine tier with 1 GiB.

**Partial 2026-11-12.** `workerd-test` is a built-in profile: `podvm run
--profile workerd-test` in a directory with `worker.js` runs `workerd test`
of that worker. workerd has no OCI image of its party, so podvm pins the
npm package `@cloudflare/workerd-linux-64` 1.20261029.1: the SHA-256 of
the archive and of `package/bin/workerd` (the registry's sha512 of the
archive agrees), fetched once into `<store>/tools/` and mounted read-only
at `/opt/podvm-workerd` (`crates/podvm/src/profile/workerd.rs`). The
config that podvm writes beside it has one worker, `globalOutbound =
"none"` and a network service that allows nothing, and the compatibility
date 2025-09-15, where `process` is undefined. The image is
`gcr.io/distroless/cc-debian12`, pinned by digest: glibc and a CA store
(workerd stops where it loads no CA certificate, also with no network).
The profile sets `confine = true` and `net.ports = []`: an empty list now
holds each TCP connect (`connect=` with no port), and no key leaves TCP
free. A profile that names its image now runs with no command; the
runtime workerd gives its own. `registry.npmjs.org` is in `WAITING` of
`crates/podvm/src/hosts_tests.rs` until `pod-services` lists it (COORD-053,
issue 274).

`cargo test -p podvm` exits 0 with `workerd_profile_holds_each_tcp_connect`
and `the_config_has_no_outbound_service`. `PODVM_417_MIRROR=1 sh
experiments/417-runtime-profiles.sh` exits 0 (clause 5, a fresh store):
the test of the worker passes, `typeof process = undefined`, `node:fs`
and `node:child_process` do not import, and a `fetch` to a host listener
fails with no connection.

Remaining: `wasmtime-wasi` (wasmtime needs a download, recorded by hash on
first use, F25 of issue 176).

# T-1694: Headless Chromium of Playwright runs as a proven payload with its flags in one profile

**Source:** A-V-05 of PodUtils/podcrates#39 (COORD-015); T-1661 (the
  runtime profiles); T-1655 (the hardening line);
  T-1658 (the confine layer)
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

A browser test job needs headless Chromium, as Playwright
starts it. podvm has no profile that says how to start it on
a host that denies namespaces, and no proof that it runs.
Chromium starts many children, wants a shared-memory
directory, and is a large glibc binary. The table refuses a
`--tmpfs` and a `--shm-size`, so the usual answer to the
shared-memory need does not work.

## Premise

Chromium's own sandbox needs user namespaces, which the
target host denies. Playwright turns it off with
`--no-sandbox`. Then the boundary is the confine layer of
T-1658 and not the browser. The parity table refuses
`--tmpfs` (`crates/podvm/src/parity.rs:259`) and
`--shm-size` (`crates/podvm/src/parity.rs:266`). The chain
of entries for a large dynamic payload is T-1643 to T-1646,
T-1647 and T-1649. No run of Chromium under podvm was
measured when this entry was filed.

## Approach

Add the built-in profile `playwright-chromium` to the T-1654
profile table. It sets `--headless`, `--no-sandbox` and
`--disable-dev-shm-usage`, which moves the shared memory to
a store directory under a size check. It sets `fs.read` and
`fs.write` to the paths that the browser needs, and
`net.ports = []` unless the profile asks for a proxy. Pick
one image with Playwright's Chromium and record its digest
after the first run. Measure the first run, name each
failure by its filed entry, and refuse by name until the
chain works.

## Decision

`--no-sandbox` is explicit in the profile and in the banner.
The profile refuses where the T-1658 layer is absent: it
never runs Chromium unconfined. Chromium under podvm is not
a boundary against a hostile page. The profile says so.

## Prove

`cargo test -p podvm` exits 0 with tests
`chromium_profile_flags` and `chromium_profile_refuses_without_confine`.
`sh experiments/425-chromium.sh` exits 0 on a chroot-denied
fixture with the chain of entries done: headless Chromium
from the pinned image prints the DOM of a page in
`/workspace` with `--dump-dom`, writes a screenshot of more
than zero bytes, makes no network connection, and the run
leaves no child process.

## Correction

**Filed 2026-11-09.** From the ask A-V-05 of PodUtils/podcrates#39
(COORD-015), a message from a peer session, priority P2: "headless
Chromium of Playwright; a workerd profile with no network". The workerd
half is in T-1661. The ask is a new capability and nothing documented is
broken, so the priority is P2.

## Done

**Done 2026-11-12.** The built-in profile `playwright-chromium` runs
`chrome-headless-shell` of `mcr.microsoft.com/playwright:v1.65.0-noble`,
pinned by digest (`sha256:9beb5460...`), with `--headless`,
`--no-sandbox`, `--no-zygote`, `--single-process`,
`--disable-dev-shm-usage` and `--disable-gpu` in `fs.wrap`, so the
arguments after `--` go to the browser. It sets `confine = true` with
read of the font, CA and identity files, write of `/workspace` and
`/tmp`, and execute of `/ms-playwright`, through the new key `fs.exec`
(Landlock gives a library mapped executable only with the EXECUTE right).
`--no-sandbox` is in the profile and its `leaves_out` says that the
browser is no boundary against a hostile page.

The chain of the first run: the namespace rung mounts no `/proc`, and
Chromium finds its ICU data through `/proc/self/exe`, so the profile runs
on the loader family (named in `leaves_out`); there, Chromium aborted with
"free(): invalid pointer" because the interposer gave back libc's
`malloc` buffers to a program with its own allocator (T-1738, fixed); the
children of the default mode did not start (T-1739), so the profile used
`--single-process` until T-1739 was done; it now runs the default mode,
with a zygote and a GPU process.

`cargo test -p podvm` exits 0 with `chromium_profile_flags` and
`chromium_profile_refuses_without_confine`. `sh
experiments/425-chromium.sh` exits 0 on the chroot-denying fixture: the
DOM of a page of `/workspace` is printed (12 s), a screenshot of 3252
bytes is written, a page that loads an image from a host listener makes
no connection under `-s net.ports=[9]` (the control with no port rule
connects once), and no process of the browser stays. The host
`mcr.microsoft.com` waits in the reported list of
`crates/podvm/src/hosts_tests.rs` for `pod-services` (COORD-050).


# T-1695: One JSON report for each run, and real labels on records

**Source:** A-V-09, A-V-07 and G-06 of PodUtils/podcrates#39 (COORD-015);
  T-1655 (the peak resident set and the CPU time);
  T-1319 (one `inspect` document);
  `crates/podvm/src/parity.rs` line 278 at `9962ba77`
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** L
**Status:** done

## Problem

A caller that runs a job under podvm has no machine-readable
record of the run. The mechanism that podvm entered is a
banner on stderr. The caller cannot read the exit status,
the times and the peak memory from one place. `--label` also
does nothing: the container record carries no labels, so
`ps --filter label=` matches nothing, and a runner cannot
find its own jobs.

## Premise

The `--label` flag is accepted and dropped
(`crates/podvm/src/run/parse.rs` line 106 at `9962ba77`). The parity table
marks it Stub (`crates/podvm/src/parity.rs` line 278 at `9962ba77`). The help
of `ps` says that a label filter matches nothing
(`crates/podvm/src/lifecycle.rs` line 41 at `9962ba77`), and so does the help of
`images` (`crates/podvm/src/images.rs` line 106 at `9962ba77`). The only JSON
that podvm writes for a caller today is `probe --json`
(`crates/podvm/src/main.rs:352`). The foreground run knows
the exit code at its end (`crates/podvm/src/run/foreground.rs:204`)
and its start is `crates/podvm/src/run/foreground.rs:233`.
The record that `run` prepares is
`crates/podvm/src/lifecycle/entry.rs:24`.

## Approach

Keep each `--label KEY=VALUE` in the container record. Make
`ps --filter label=KEY[=VALUE]` and `inspect` use it. Take
the labels of an image from its OCI config for `images
--filter label=`. Add `run --report PATH`. The run writes
exactly one JSON document to PATH, with a rename, also when
it refuses before the start. The document holds: a schema
version, the podvm version, the image digest, the rung or
tier that the run entered and how it was measured, the
argument vector, the exit status or the signal, the start
and end times, the wall time, the CPU time and the peak
resident set of the tree (T-1655), whether a timeout ended
the run, the limits applied, the labels, and each
degradation. An unknown value is `null`.

## Decision

The report goes to a file, never to standard output: that
belongs to the payload. The report holds no value of an
environment variable and no secret. One run writes one
report, and a runner that wants one report for each job
gives each job its own path (G-06). The `--strict` rule that
refuses a Stub no longer applies to `--label`, because the
flag works. The parity row moves from Stub to Native.

## Prove

`cargo test -p podvm` exits 0 with tests
`label_reaches_ps_filter`, `label_reaches_inspect`,
`report_has_every_field` and `report_written_on_refusal`.
`sh experiments/426-run-report.sh` exits 0: a run with
`--label job=7 --report R.json` leaves one valid JSON file
that names the tier, exit 3 for `sh -c 'exit 3'`, a peak
resident set above zero, and the label; `ps -a --filter
label=job=7` lists the run.

## Correction

**Filed 2026-11-09.** From the asks A-V-09 (priority P1), A-V-07 and G-06
of PodUtils/podcrates#39 (COORD-015), a message from a peer session: "a
JSON report of each run, and real labels", with peak RSS and CPU in the
report, and one JSON report for each job. The priority is P1 because
`--label` is a flag that does nothing (the priority rules of
`TODO/INDEX.md`). The report half alone would be P2.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC section 11.4 and the `wait` answer of its unit contract) reads
this report as the podvm half of its run report. Four clauses are new.
First, a detached run (`run -d`, `start`) writes its report too: the
launcher writes it with a rename when it reaps the payload, and `podvm
wait` returns after the rename. Second, the report names the entered rung
with the word of `.EnteredRung` (T-1713) and lists the volume table with
the mode of each pair. Third, the report gives the count of processes of
the tree that the end killed (T-1708, the subreaper), or `null` where
podvm cannot count. Fourth, an end by a signal names the signal (for
example `SIGXFSZ`) beside the number. Bytes in and out are not podvm's to
measure; the report does not carry them.

## Done

**Done 2026-11-11.** `run --report PATH` writes one JSON document of the
run with a rename, also when podvm refuses before the start
(`crates/podvm/src/run/report.rs`): the schema, the podvm version, the
image and its digest, the rung and how podvm knows it, argv, the
container, the exit code and the signal (128 + N), the refusal, the start,
the end and the wall time, the CPU time and the peak resident set of the
reaped children of the run (`getrusage(RUSAGE_CHILDREN)`), the timeout,
the limits, no_new_privs, the confine SPEC, the labels and the fixups. An
unknown value is null; no value of an environment variable is in it.
`--report` is a flag of `run` only: `create` refuses it by name. `images
--filter label=KEY[=VALUE]` reads the `config.Labels` of the image's OCI
config blob; by hand, `public.ecr.aws/lambda/provided:al2023` matches
`label=com.amazonaws.lambda.platform.kernel=k510ga` and not another value.
`cargo test -p podvm` exits 0 with `report_has_every_field`,
`report_written_on_refusal` and the image-label test.
`sh experiments/426-run-report.sh` exits 0 on the cloud host: a run with
`--label job=7 --report R.json` of `sh -c 'exit 3'` exits 3 and leaves one
valid document with rung `namespace`, exit_code 3, signal null,
max_rss_kib 26972 and the label; `ps -a --filter label=job=7` lists the
run and `label=job=8` does not; a refused run leaves its report (refused
125, exit_code null, no temporary file); the value of `-e SECRET=` is not
in the report.

**Partial 2026-11-11.** Labels are real for containers: each `--label
KEY[=VALUE]` of `run` and `create` joins the record (`Hardening.labels` of
`crates/podvm-supervise/src/table.rs`; the foreground record now stores
the options of its run, as `run -d` does), `ps --filter label=KEY` and
`label=KEY=VALUE` match them with docker's rule, and `inspect` prints
`.Labels` (`KEY=VALUE`, a comma between two). A label with no key is a
flag error (125). The parity row of `--label` moves from Stub to Native.
`cargo test -p podvm --test labels` exits 0 with `label_reaches_ps_filter`
and `label_reaches_inspect`, on an image that `podvm import` records (a
static program) and the record of a foreground run, with no registry. The
first form used `create` of a `#!` script: CI denies chroot, and there
`create` found no entry for it (gate run 43135351382); the test was
corrected on 2026-11-12.



# T-1696: `--secret` places a file with no argv and no env, and a read-once mode

**Source:** A-V-11 and G-03 of PodUtils/podcrates#39 (COORD-015);
  T-1653 (the volume path map); the ruling of 2026-11-09 on
  test accounts; `crates/podvm/src/run/parse.rs:108`
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

A job needs a credential. Today a caller passes it with `-e`,
which puts the value in the environment of the payload and
of each child, or in a file that a `-v` volume would place.
podvm has no flag that places one file, with a mode, for
the first process only.

## Premise

`--env-file` reads a file at parse time
(`crates/podvm/src/run/parse.rs:108`, used at
`crates/podvm/src/run/parse.rs:224`) and the values go into
the environment (`crates/podvm/src/run/prepare.rs:242`). The
userland path map is one pair for the root
(`src/userland.rs` of `pod-enter` (PodUtils/podcrates)); T-1653 adds pairs.
A foreground run removes its files at the end
(`crates/podvm/src/run/foreground.rs:204`). The parity table
has no `--secret` row. The interposer is not a boundary, so
a claim about "only the runner process" has a limit that the
banner must state.

## Approach

Add `run --secret src=PATH,target=GUEST,mode=0400[,once]`.
podvm reads `src` itself. The value never enters argv or the
environment. It stages the bytes under the run directory
(mode 0700 for the directory, the given mode for the file)
and adds one path-map pair from `target` (T-1653). Refuse a
`src` that group or other can read, and name its mode. With
`once`, the interposer serves the first `open` of `target`
from the first process of the payload (the runner) and
removes the staged file at once. A child, and any later
`open`, gets `ENOENT`. At the end of the run, also after a
failure, podvm removes the staged file. `inspect`, `ps`, the
banner, the logs and the report (T-1695) show the target and
the mode, never the value. The machine tier refuses
`--secret` by name until T-1656 carries a per-run file into
the guest.

## Decision

The credential reaches the runner process through a file and
nothing else (G-03). A payload that bypasses the libc
interposer with raw system calls can read the staged file if
it learns its host path. The host path is in no variable, and
the banner names this limit. podvm stores no credential
between runs. No test account is a CI secret now (the ruling
of 2026-11-09), so a live step that needs one waits.

## Prove

`cargo test -p podvm` exits 0 with tests
`secret_spec_parses`, `secret_refuses_group_readable_src`,
`secret_value_absent_from_argv_and_env` and
`secret_once_second_open_fails`.
`sh experiments/427-secret-flag.sh` exits 0 on a
chroot-denied fixture: the runner reads the target with mode
0400, `/proc/PID/cmdline` and `/proc/PID/environ` of the
payload hold no byte of the value, with `once` a second read
and a read from a child fail, and the staged file is gone
after the run, also after a payload that exits 1.

## Correction

**Filed 2026-11-09.** From the asks A-V-11 (priority P2) and G-03 of
PodUtils/podcrates#39 (COORD-015), a message from a peer session:
"--secret src=,target=,mode=0400 with no argv and no env", with a
read-once mode, and "the one-job runner credential reaches only the
runner process". The peer asks for P0 for G-03. The priority stays P2: no
behaviour of podvm is wrong today, and the flag is new.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC section 8.2, its measurement M1 with runner v2.338.2) measured
the official runner: its credential is three files (`.runner`,
`.credentials`, `.credentials_rsaparams`) in its root; it keeps them for
its life and does not write them again; its Worker child reads `.runner`
from disk again. Four clauses are new. First, `--secret` is repeatable,
and several targets can be in one guest directory that the payload also
writes. Second, `once` breaks a payload whose child reads the file again;
the help of `once` says so, and the experiment adds a child that reads a
target a second time without `once`. Third, `--secret` works with `run -d`
and `start`: the staged files stay until the launcher reaps the payload,
and `rm` removes them also when the launcher is gone. Fourth, the staging
directory is mode 0700 whatever the umask (T-1715, the store modes), and
the tests use mode 0600 as well as 0400.

## Done

**Done 2026-11-11 by withdrawal: the owner dropped this entry (the ruling
"T-1696 dropped", the answer to Q5).** No code was built and the Prove did
not run. podvm has no `--secret` flag and no read-once mode. podrunner
stages the three credential files of the official runner itself, at mode
0600, with no argv and no environment, and removes them at `destroy` (G-03
as DR-6 restates it, PodUtils/podrunner#6). The clauses of other entries
that named `--secret` are corrected in place: T-1695 (the report),
T-1715 (the store modes) and T-1656 (the guest's tmpfs).

# T-1667: Doctor and installers: one check per scope capability, verified downloads, a space floor

**Source:** issue 188; #193; the owner's ruling of 2026-11-06
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** partial

## Problem

`podvm doctor` reports the rootfs and machine legs. It does
not cover the widened scope: Landlock, Syscall User Dispatch,
`openat2(RESOLVE_IN_ROOT)`, user-mode networking, labs in a
guest, proxy reach. Assets that the new scope downloads
(guest kernels, runtimes) have no stated verification rule.

## Premise

The doctor is `crates/podvm/src/doctor.rs:149`, and it reads
the machine assessment (`doctor.rs:190`). The probe already
has a Landlock leg (`pod_hostprobe::probes`).
Space is checked before large writes
(`pod_hostprobe::space`, T-0806). Release
verification uses cosign and the workflow identity (T-1328).

## Approach

Add one doctor group for each scope capability. Each check
prints `ok`, `MISSING` with a `fix:` line, or "cannot be fixed
on this host" with the reason. `--strict` exits 1 on a
MISSING. Each check proves the thing runs: a guest boots, a
payload enters. For each downloaded asset, verify a checksum
and a signature where upstream publishes them, else record a
first-use hash in the store and refuse a change. Fetch a
release through `gh`, and fall back to the public API when
the call fails, not when `gh auth status` fails.

## Decision

A present file is not a working one (T-1301). An auth check is
not a working credential: a suspended token passes `gh auth
status` and then gets 403 (F34). A checksum proves integrity,
not who built the file, and the doctor says which of the two
it has.

## Prove

`cargo test -p podvm` exits 0 with tests: a planted changed
asset is refused, a planted rejected `gh` token falls back to
the API, and a run below the space floor refuses and names the
path. `sh experiments/423-doctor-scope.sh` exits 0 on a
chroot-denied fixture: `podvm doctor` lists every scope
capability with one of the three answers, and `podvm doctor
--strict` exits 1 where one is MISSING.

## Correction

**Filed 2026-11-06.** Quoted from issue 188:

```text
F34: `gh auth status` exits 0 for a suspended token while every
     `gh release` call returns 403; the unauthenticated API allows 60
     requests an hour per address, and an install script hit the limit
F25: landrun, wasmtime and workerd publish no checksum, and
     `gh release view --json assets` gives no digest for them
F12: `apt-get install tayga` pulled apparmor; its pasta profile in enforce
     mode made `pasta --version` exit 139
F26: `pasta --version` succeeding does not mean pasta works
F31: a run failed with "no space left on device" at 188 MiB free; the
     disk guard warns below 1 GiB and refuses below 64 MiB
```

## Done

**Partial 2026-11-11.** Done: `cargo test -p podvm` exits 0 with
`a_planted_changed_asset_is_refused` and
`a_run_below_the_space_floor_refuses_and_names_the_path`
(`crates/podvm/src/assets.rs`), and
`a_missing_scope_capability_fails_only_under_strict`.
`sh experiments/423-doctor-scope.sh` exits 0 on the chroot-denying
fixture (`experiments/results/doctor-scope.txt`): each of the ten scope
capabilities (`native-static`, `syscall-tier`, `openat2`, `landlock`,
`user-net`, `emulate`, `incus`, `proxy`, `space`, `assets`) has one
line with ok, MISSING or cannot; `scope.incus` is MISSING with its fix;
`doctor` exits 0 and `doctor --strict` exits 1; with
`--deny-dispatch`, the native-static rung and the syscall tier read
"cannot be fixed on this host". The native-static check proves the rung by
a run: podvm enters itself in a root of its own and prints its version.

Downloads: `crates/podvm/src/assets.rs` checks an asset against podvm's
pin, or records its SHA-256 at the first use in `<store>/assets/` and
refuses a change, naming the record to remove. `PODVM_MACHINE_KERNEL`, a
kernel of the caller with no pin, takes the first-use rule; the pinned
guest kernel keeps its pin. The fetch of the guest kernel checks a space
floor of 64 MiB first and names the path. `scope.assets` says what each
check proves: integrity, not who built the file.

Remaining clauses:
* The `gh` fetch with the fallback to the public API, and its planted
  rejected token: podvm fetches no GitHub release today. The first
  installer that does (a runtime of T-1661 or T-1660) takes the rule and
  the test.
* "Labs in a guest": a check for it comes with the network entries
  T-1662 to T-1665.

# T-1690: The doctor disk-free row judges the soft `RLIMIT_FSIZE`

**Source:** issue 214; T-1301 leg 3; T-1305
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

`podvm doctor` reads the hard `RLIMIT_FSIZE` in the row "disk
free against the file-size ceiling". The kernel raises SIGXFSZ
at the soft limit, and each other reader of the limit reads
the soft one. With a soft limit of 1 GB and a hard limit of
infinity, doctor prints a note and exits 0, and a machine-tier
run refuses on the same host.

## Premise

`crates/podvm/src/doctor.rs:317` keeps the hard limit. The
infinity test is at `crates/podvm/src/doctor.rs:329` and the
comparison at `crates/podvm/src/doctor.rs:343`. The soft
readers are `crates/podvm/src/tier.rs:114`,
`crates/podvm/src/machine/ssh/qemu.rs:260`,
`crates/podvm/src/windows/plan.rs:88`,
`crates/podvm-probe/src/nongoals.rs:190` and
`pod_hostprobe::space`. The function reads `prlimit` itself, so
a test cannot drive it.

## Approach

Keep the soft limit and use it for the infinity test and the
comparison. Print the hard limit as well when it differs. Move
the judgement into a pure function over the soft limit, the
hard limit and the free bytes.

## Decision

The enforced bound is the soft limit. Doctor and the machine
tier print the same number on one machine.

## Prove

`cargo test -p podvm` exits 0 with tests: a soft 1 GB and a
hard infinity give the 1 GB ceiling; the same limits with less
free space fail with a fix line; both infinite stay a note.

## Correction

**Filed 2026-11-09.** From issue 214 (an outside account). The triage of
2026-11-09 confirmed each claim at `2be31b2`.

## Done

**Done 2026-11-09.** `examine_ceiling` keeps the soft limit and judges
it; the comparison is the pure function `judge_ceiling`, which names the
hard limit when it differs. `cargo test -p podvm` exits 0 with
`the_ceiling_is_the_soft_limit` (a soft 1 GiB and a hard infinity give
the 1 GiB ceiling; less free space fails; equal limits print no hard
limit). [The drive](../experiments/results/doctor-soft-fsize.txt): with
`ulimit -S -f` at 2 GiB and the hard limit at infinity, `podvm doctor`
judges the 2 GiB ceiling, where it printed "no file-size ceiling"
before; at 64 GiB, above the free space, the row fails with its fix line.

# T-1706: The unmodified official `actions/runner` starts on the userland rung

**Source:** issue 231; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), SPEC sections 10.4, 10.5, WP-V1
  and milestone M-B, its measurements M1 and M2; A-V-04 of
  PodUtils/podcrates#39; T-1704 (`realpath` of
  `/proc/self/exe`); T-1705 (the doubled prefix); T-1707 (a
  first program relative to `-w`); T-1655 (`--ulimit`);
  `crates/podvm/src/tier.rs:114`
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

The official runner (linux-x64, glibc, .NET) does not start
under podvm on a chroot-denied host: exit 154, then 137 after
a workaround. It runs only under `--unsafe-host-paths` with
the runner at its real host path, which is not isolation. A
.NET payload also dies at start with SIGXFSZ (exit 153) under
each soft `RLIMIT_FSIZE` below 2 TiB, and podvm says only "the
payload died on signal 25".

## Premise

M2 at `36b15ec`, one host, runner v2.338.2, 3 runs for each
case. Blocker 1 is the `realpath` answer (T-1704), blocker 2
the doubled prefix (T-1705). A third layer after `--help` is
possible: no job ran. M1: the runtime calls `ftruncate` on a
memfd with 2199023255552 bytes; under a smaller limit that
is `EFBIG` and SIGXFSZ, in the listener, in `config.sh` and in
the Worker; with `DOTNET_EnableWriteXorExecute=0` it starts.
M2: a `prlimit` on the podvm process reaches the payload. M1:
the runner needs ICU or
`DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1`, `libssl.so.3`, and
a glibc loader with symbols up to `GLIBC_2.28`. podvm reads
the soft limit for guests (`crates/podvm/src/tier.rs:114`)
and not for a payload. No podvm record names .NET.

## Approach

Build a test image:
`public.ecr.aws/debian/debian:bookworm-slim`, the runner's
libraries, and the runner tarball checked against a pinned
SHA-256; build it where chroot is permitted and `podvm
import` it. Close T-1704, T-1705 and T-1707. Then drive, on
the chroot-denying fixture: `Runner.Listener --version`,
`config.sh --help`, and `run.sh` with the three credential
files of a synthetic blob pre-staged at mode 0600 against a
loopback stand-in that serves `POST /oauth/token`, `POST
/session` and `GET /message` (the shape of M1). File each new
blocker as an entry. When a payload dies on SIGXFSZ, print
the soft `RLIMIT_FSIZE`; for a payload with a `.dll` beside
it or a `DOTNET_` variable, also name
`DOTNET_EnableWriteXorExecute=0`.

## Decision

The runner is a payload, not a special case: podvm adds no
runner flag and ships no runner. `--unsafe-host-paths` is not
a pass. podvm does not raise or drop the caller's
`RLIMIT_FSIZE`; it names it. The stand-in server is test code.

## Prove

`sh experiments/428-actions-runner.sh` exits 0 on the
chroot-denying fixture with no `--unsafe-host-paths`:
`--version` prints the pinned version, `config.sh --help`
exits 0, `run.sh` with the pre-staged files reaches the long
poll of the stand-in and ends within 30 s of SIGTERM, and
under `prlimit --fsize=1073741824` a run with W^X on exits
153 with a message that names `RLIMIT_FSIZE`, while the same
run with `-e DOTNET_EnableWriteXorExecute=0` exits 0.

## Correction

**Filed 2026-11-10.** From issue 231 and the podrunner design review
(the ruling "podrunner", 2026-11-10), WP-V1: "Official runner starts on
the userland rung without `--unsafe-host-paths` (exit 154, then 137)".
The entry is a new proven payload, so it is P2; its defects T-1704,
T-1705 and T-1707 are P1 and are podvm's own.

## Done

**Done 2026-11-11.** `experiments/428-actions-runner.sh` builds the test
image on the capable rung (debian bookworm-slim, `ca-certificates`,
`libicu72`, `libssl3`, `libkrb5-3`, `zlib1g`, `liblttng-ust1`, and the
runner v2.338.2 in `/opt/runner`, checked against SHA-256 `baebf204…`,
with the three credential files of a synthetic blob at mode 0600), and
`PODVM_428_MIRROR=1 sh experiments/428-actions-runner.sh` exits 0 on the
chroot-denying fixture with no `--unsafe-host-paths`
([the run](../experiments/results/actions-runner.txt)):

* `-w /opt/runner ./bin/Runner.Listener --version` prints `2.338.2`;
* `./config.sh --help` exits 0;
* under `prlimit --fsize=1073741824` the same run exits 153, and podvm
  names the soft `RLIMIT_FSIZE` of 1073741824 bytes and
  `DOTNET_EnableWriteXorExecute=0`; with that variable it exits 0;
* `./run.sh` against `experiments/runner-standin.py` (the shape of
  podrunner's `podrunner-broker-fake`, PodUtils/podrunner#50) takes a
  token, makes a session, prints "Listening for Jobs" at the long poll,
  and ends 1 s after SIGTERM to podvm (143), with no `Runner.Listener`
  left.

The SIGXFSZ lines are `crates/podvm/src/run/exit_note.rs` (a foreground
run; a .NET payload is a `DOTNET_` variable or a `.dll` beside the
program). Found on the way: with a relative first program the interposer
was not loaded (fixed under T-1707); and the runtime asks `realpath("")`
and `access("")` at each start, which printed "podvm cannot resolve the
path": an empty path now passes to the kernel, which answers `ENOENT`
(`prepare_mode` in `crates/podvm-interpose/src/map/rewrite.rs`, the test
`an_empty_path_passes_unchanged`). The runner sent no `DELETE /session`
before it ended on SIGTERM; an ephemeral runner needs none.

# T-1711: The calls of the four container-hook commands run through podvm verbs on the userland rung

**Source:** issue 236; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), SPEC sections 7.2, 10.1, its
  unknown U19 and its hook contract; its R4:2.6 (the official
  docker hook); G-09 of PodUtils/podcrates#39; T-1653
  (volumes); T-1655 (`:ro`); T-1707; T-1709; T-1710;
  `crates/podvm/src/parity.rs:222`;
  `crates/podvm/src/exec/enter.rs:203`
**Category:** cli
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

A container job of the official runner calls its container
hook. podrunner's hook asks the host to start a job container,
run each step in it, run a container action, read logs and
exit codes, and remove the containers. On a podvm unit those
are podvm verbs. No test drives that sequence, so nobody knows
which call fails first.

## Premise

`run -d`, `--entrypoint`, `-e`, `-w`, `--name` and
`--add-host` are Native (`crates/podvm/src/parity.rs:210` to
`crates/podvm/src/parity.rs:235`); `-v` is Degraded and
refuses `:ro` (`crates/podvm/src/parity.rs:222`); `exec -e`
and `-w` are Native, `exec -d` is None
(`crates/podvm/src/parity.rs:291` to
`crates/podvm/src/parity.rs:297`); `logs` is one interleaved
file (`crates/podvm/src/parity.rs:153`); `network` and `-p`
are None. `exec` checks its workdir through the container's
volume table (`crates/podvm/src/exec/enter.rs:203`), not yet
driven. The official docker hook calls `network create`,
`port` and `exec -i`, so it cannot run over podvm (Inferred).
Each `exec` runs the probe and completion again; the cost is
not measured.

## Approach

Write an experiment that replays the podvm calls of the
hook for a job with `container:` and one container action:
a job container with `run -d --entrypoint tail` and the seven
mounts of the job-container table (`/__w`, `/__w/_temp`,
`/__w/_actions`, `/__w/_tool`, `/__e` read-only,
`/github/home`, `/github/workflow`) from directories under a
runner root; a step with `exec -e K=V -w /__w/repo J sh -e
/__w/_temp/step.sh`; a container action with `run --rm`, the
five mounts of the container-action table and `-w
/github/workspace`; `logs`; `wait`; `rm -f`. Use a job image
that differs from the runner's image. Record each call as
passed or refused by name, and file each defect. Measure the
time of 20 `exec` steps.

## Decision

podvm stays a command line. It adds no hook, no socket
server and no word of podrunner (the ruling "New projects",
2026-11-09). The hook and its control socket belong to
podrunner.

## Prove

`sh experiments/431-hook-verbs.sh` exits 0 on the
chroot-denying fixture: each call of the sequence exits as
the experiment records, a step sees its `-e` value and its
workdir, a write to `/__e` fails with `EROFS`, a write to
`/__w` lands in the runner root, the container action reads
`/github/workspace`, `rm -f` leaves no process, and the time
of 20 steps is printed.

## Correction

**Filed 2026-11-10.** From issue 236 and the podrunner design review
(the ruling "podrunner", 2026-11-10), section 7.2: the hook relays to a host
socket, and the host-side server calls podvm. A new use with nothing
wrong without it is P2.

## Done

**Done 2026-11-11.** `sh experiments/431-hook-verbs.sh` exits 0 on the
chroot-denying fixture (the userland rung, alpine 3.20 as the job image):
`run -d --name job` with the seven mounts of the job-container table,
`/__w/_temp` nested in `/__w`, exits 0; a step `exec -e K=V -w /__w/repo
job sh -e /__w/_temp/step.sh` prints `K=V` and `pwd=/__w/repo`, and its
write lands in the runner root; a write to `/__e` (`:ro`) fails with "Read-only
file system" and leaves no file; the container action `run --rm` with the
five mounts of the container-action table and `-w /github/workspace`
reads the checkout and exits 7; `logs job` holds the output of the
entrypoint; `wait` of a detached `sh -c 'exit 5'` prints 5; 20 `exec`
steps take 1812 ms (about 90 ms each, the probe and the completion of each
`exec` included); `rm -f job` exits 0 and leaves no process of the job.
No defect was found: T-1653, T-1655, T-1707, T-1709, T-1710, T-1725 and
T-1726 carry the sequence. The official docker hook also calls `network
create`, `port` and `exec -i`, which podvm does not serve; podrunner's own
hook makes only the calls above.


# T-1712: `--podvm-tier=chroot` does nothing: it runs the ladder

**Source:** issue 237; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), SPEC section 10.1, its measurement
  M2 (`01-probe-rung.log`); T-1302; call C2 of
  `docs/decisions.md`; `crates/podvm/src/tier.rs:86`;
  `crates/podvm/src/parity.rs:245`
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

The parity row says that `--podvm-tier=chroot` selects the
chroot tier. It runs the ladder. On a chroot-denied host it
enters the userland rung with exit 0; where the probe selects
the namespace rung it enters that. A caller that asks for one
mechanism gets another, and only the banner says so.

## Premise

`resolve` maps the value to `Tier::Chroot`
(`crates/podvm/src/tier.rs:86`). Each caller compares the tier
with `Tier::Machine` only
(`crates/podvm/src/run/prepare.rs:25`,
`crates/podvm/src/exec/command.rs:42`). T-1302 made `chroot`
the override of the argv0 default `podvm` = machine. Call C2
(2026-11-06) removed that default, so the value has no effect
now. M2 observed exit 0 and `mode=userland`.

## Approach

Take one of two forms and record it. (a) Remove the value
`chroot`; `--podvm-tier` takes `machine` only, and the ladder
is the default. (b) Make `chroot` enter the chroot sequence
and nothing else: where chroot is denied, refuse at 125 with
the T-1317 text; where the probe selects the namespace rung,
enter chroot without the namespace. The recommendation is
(a): a caller reads the entered rung from the report
(T-1695) and from `.EnteredRung` (T-1713), and needs no force.
Correct the parity row in the same change.

## Decision

A flag that names a tier selects it or refuses by name. No
compatibility is kept (the ruling "Compatibility",
2026-11-05).

## Prove

`cargo test -p podvm` exits 0 with a test of the chosen form.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with a new clause: on the fixture,
`--podvm-tier=chroot public.ecr.aws/docker/library/alpine:3.20
true` is refused by name (form a: an unknown value; form b:
chroot denied).

## Correction

**Filed 2026-11-10.** From issue 237 and the podrunner design review
(the ruling "podrunner", 2026-11-10), M2 surprise 7: "`--podvm-tier=chroot`
on a chroot-denied host silently enters userland". The cause is wider:
the value has no effect on any host. A flag that does nothing is P1.

## Done

**Done 2026-11-11, form (a)** (call C14 of `docs/decisions.md`).
`--podvm-tier` takes `machine` only: `tier::want` knows no `chroot`, and
`Want::Chroot` and `Tier::Chroot` are gone (`crates/podvm/src/tier.rs`).
`tier::refusal` gives the one flag error of `run`, `create` and `exec`:
`--podvm-tier takes machine, not "chroot": with no flag the ladder selects
the rung, and podvm system info --format '{{.EnteredRung}}' names the rung
that a run enters`. The parity rows of `run` and `exec`, both help texts,
`docs/architecture.md` and call C2 say the same. The Prove, on the cloud
host, 2026-11-11: `cargo test -p podvm -p podvm-measure` exits 0 (306
tests), with `the_flag_spells_machine_and_nothing_else` and
`the_tier_flag_takes_machine_only_and_qemu_args_collect_whole` of `run`
and of `exec` (both spellings of `chroot` exit 125).
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 with
the new clause 27 ([the run](../experiments/results/no-chroot-rung.txt)):
on the chroot-denied fixture `run --rm --podvm-tier=chroot` of the pinned
alpine digest exits 125 with that message. `cargo clippy --workspace
--all-targets -- -D warnings` exits 0.

# T-1713: `.EnteredRung` reads `chroot` where the probe selects `supervise` and chroot is denied

**Source:** issue 238; the podrunner design review (the owner's design
  review of PodUtils/podcrates#47), SPEC section 10.1, its measurement
  M2 (`01-probe-rung.log`); T-0804 rule 4; T-1317;
  `crates/podvm/src/system/df_info.rs:307`;
  `crates/podvm/src/run/acquire.rs:23`
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

On a host where the probe selects `supervise` (the listener,
the argument channel and ptrace work) and chroot is denied,
`system info` gives `.EnteredRung` = `chroot`. `run` enters
the userland rung there. A caller that reads the field before
a run reads the wrong mechanism.

## Premise

`crates/podvm/src/system/df_info.rs:307` takes the word from
`entered_word` of `pod-enter`, which maps each selection but
`interpose` and `namespace` to `chroot` (`src/lib.rs` of
`pod-enter` v0.12.0). `run` adds its own userland decision
(`crates/podvm/src/run/acquire.rs:23`). The help of `system
info` says that the field reads `userland` where chroot is
denied. The tests cover the `interpose` and `namespace`
selections only. M2 observed `.Rung`=`supervise`,
`.EnteredRung`=`chroot` and the banner `mode=userland`. The
errand host denies ptrace, so it selects `interpose` and reads
correctly.

## Approach

Derive `.EnteredRung` from the same facts that `run` uses:
where the `chroot(/tmp)` finding is denied, the word is
`userland`, whatever the selection. If the rule belongs in
`pod-enter`, send it to podcrates as a COORD issue
(`AGENTS.md`, section 9, item 4); else keep it in
`df_info.rs`.

## Decision

The field names what `run` enters, never what the probe
selected (T-0804 rule 4).

## Prove

`cargo test -p podvm` exits 0 with a test that a `supervise`
selection with a denied chroot reads `userland`.

## Correction

**Filed 2026-11-10.** From issue 238 and the podrunner design review
(the ruling "podrunner", 2026-11-10), M2 question Q1: "`system info`
gives `.Rung`=`supervise`, `.EnteredRung`=`chroot`; the first `run`
banner says `mode=userland`". A wrong answer is P1.

## Done

**Done 2026-11-11.** The rule stays in podvm (`df_info.rs`), because the
decision that it mirrors is podvm's: `decide_entry` asks `chroot_usable`
before the selection, and `pod-enter`'s `entered_rung` knows only the
selection. `system::df_info::entered_rung(selection, chroot_usable)` gives
`userland` wherever chroot is denied, else `pod_enter::entered_rung`;
`collect` gives it `chroot_usable` of the same findings. The help of
`system info` says so. The premise was wider than written: under the
chroot-denied fixture of experiment 356 the cloud host selects
`namespace` (user namespaces work), and the field read `namespace` while
`run` entered the userland rung. The Prove, on the cloud host,
2026-11-11: `cargo test -p podvm` exits 0 with
`a_denied_chroot_reads_userland_whatever_the_selection` (`supervise`,
`namespace`, `chroot` and `interpose` with a denied chroot read
`userland`; `supervise` with a usable chroot reads `chroot`). Clause 27
of experiment 356 exits 0 ([the run](../experiments/results/no-chroot-rung.txt)):
`system info --format '{{.Rung}} {{.EnteredRung}}'` under the fixture
prints `namespace userland`.

# T-1726: `exec` into a container ignores its `:ro` volumes and `--read-only`

**Source:** PodUtils/podrunner#62 (COORD-008 of podrunner, a message from
  a peer session); measured on the cloud host, 2026-11-11
**Category:** cli
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

On the loader family, `podvm run --rm -v DIR:/e:ro IMAGE sh -c
'echo x > /e/r.txt'` fails the write (EROFS), but after `podvm
run -d --name c -v DIR:/e:ro IMAGE sleep 30`, `podvm exec c sh
-c 'echo x > /e/w.txt'` exits 0 and `DIR/w.txt` exists
(measured, 2026-11-11). podrunner mounts the externals of the
runner as `/__e:ro`, so one step can change the code of the
next.

## Premise

`run` sets the read-only table `PODVM_RO` of the interposer
(`lifecycle::volume::read_only_env`, T-1655). An `exec` enters
the container's rootfs with the container's table of volumes.

## Approach

Keep the read-only part of the run in the container record and
set `PODVM_RO` for each `exec` of the container as `run` does;
an entry that cannot serve it refuses by name, as `run` does.

## Decision

No entry of a container serves a write under a `:ro` volume or
under `--read-only`.

## Prove

`cargo test -p podvm` exits 0 with a test of the exec
environment of a container with a `:ro` volume, and
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
exits 0 with a clause where `exec` into a container with a
`:ro` volume fails a write there with EROFS and leaves no file
in the host directory.

## Correction

**Filed 2026-11-11.** From PodUtils/podrunner#62, reproduced the same day.

## Done

**Done 2026-11-11.** `cargo test -p podvm` exits 0 with
`an_exec_takes_the_read_only_table_of_its_container`, and
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 ([the run](../experiments/results/no-chroot-rung.txt)): in clause 35 `exec` into a container with
`-v DIR:/e:ro` fails its write with exit 1, and DIR has no file.

**Partial, 2026-11-11.** The record of a detached run keeps `PODVM_RO` in its
environment; `exec` now takes it (`lifecycle::volume::container_read_only`,
called in `crates/podvm/src/exec/enter.rs`), drops a caller's own value, and
refuses by name on an entry with no read-only layer. `cargo test -p podvm`
exits 0 with `an_exec_takes_the_read_only_table_of_its_container`. On the
fixture, `exec` of a container with `-v DIR:/e:ro` answers "Read-only file
system" and DIR stays empty. Remaining: the run of experiment 356 with
clause 35.
