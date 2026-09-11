# Local evidence model

This repository is self-contained. Publishable claims resolve to local Markdown,
CSV, JSON, code, or committed experiment logs. The ledger verifies that the exact
supporting statement still exists and that its conditions remain attached.

## Evidence classes

| Class | Meaning | Valid use |
|---|---|---|
| `OBSERVED` | a recorded operation, result, and condition block | describe that observation under those conditions |
| `CODE` | repository code directly implements or checks a rule | state what the current code does |
| `DERIVED` | arithmetic or architecture follows from identified rows | state the derivation, never a new measurement |
| `DESIGN` | a proposed component, invariant, or acceptance criterion | describe intent, not completed implementation |
| `LIMIT` | a bounded negative statement or threat to validity | prevent stronger interpretation |
| `OPEN` | the required experiment has not run or a question remains unresolved | create an obligation, not a positive claim |

## Claim publication rule

Every row in `experiments/evidence-ledger.csv` contains:

1. a stable identifier;
2. an evidence class;
3. a repository-relative path;
4. a literal anchor present in that file;
5. the condition limiting the claim.

The literal anchor is a drift alarm, not proof of truth. The evidence audit can
show that the repository remains internally traceable. The claim review must still
ask whether the wording exceeds the observation or derivation.

Paths must resolve inside the repository. Parent traversal and absolute paths are
rejected by `scripts/check-evidence.ps1`.

## Observation record

A useful observation is a tuple:

```text
operation + exact arguments + result + discriminator + mechanism hypothesis
+ policy epoch + host conditions + reproduction status
```

Do not collapse this tuple into `supported` or `blocked`. In particular:

- an errno is not a mechanism;
- a successful creation step does not prove attachment or use;
- a host driver does not prove a tenant can obtain its device descriptor;
- a stored result is not a rerun on the current machine;
- agreement between two documents is not independent experimental replication.

## Reproduction status

Use these labels in `docs/observations.md`:

- `CAPTURED`: the full result and conditions are stored here, but the operation is
  not runnable on the current authoring host;
- `LOCAL-DERIVATION`: all inputs are in this repository and the calculation is
  rerun by the gate;
- `DESIGN-ONLY`: an architecture or acceptance criterion not yet implemented;
- `OPEN`: a named closure experiment remains.

## Triangulation order

Prefer, in descending order:

1. implementation plus self-asserting experiment plus log;
2. two distinct probes that isolate the same mechanism;
3. one probe plus a control-flow explanation;
4. implementation without a run;
5. prose stating intended behavior;
6. a one-time report retained with an explicit limitation.

An exit code or self-report is never sufficient by itself. Validate the artifact,
process state, network effect, descriptor, or other outcome the operation promised.

## Policy epochs

Policy observations can expire. Cache by boot identity, kernel identity, and a
small canary vector. Invalidate the cache whenever any canary changes. The local
record includes both syscall and network verdict transitions; they are evidence
that time belongs in the claim, not evidence of a universal schedule.
