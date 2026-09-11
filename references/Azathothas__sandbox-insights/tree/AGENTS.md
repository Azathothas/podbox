# AGENTS.md

This repository is written for agents first. Markdown is the source of truth;
generated LaTeX and PDF files are secondary exports.

## Start every task

1. Read `README.md`.
2. Read `docs/evidence-model.md` before changing a claim.
3. Use the routing table below.
4. Run `scripts/run-gate.ps1` before declaring completion.

## Task router

| Task | Read in this order |
|---|---|
| understand the model | `docs/capability-model.md`, `docs/observations.md` |
| diagnose a failure | `docs/failure-field-guide.md`, then the relevant model section |
| change a claim | `docs/evidence-model.md`, `experiments/evidence-ledger.csv`, cited local file |
| add an observation | `docs/research-method.md`, `experiments/README.md`, `docs/observations.md` |
| change the design | `docs/architecture.md`, `docs/open-questions.md` |
| change the paper | `paper/paper.md`, then `scripts/build-paper.py` |
| review a release | all Markdown, all experiment scripts, `reviews/`, then the full gate |

## Non-negotiable rules

1. **Measure operations, not nominal privilege.** UID 0 and capability bits do
   not predict which namespace, syscall, path, device, address, or resource check
   will win.
2. **Keep the seven planes separate.** Identity, syscall, path, network, device,
   resource, and virtualization/observation failures can share an errno while
   requiring different remedies.
3. **Probe stages separately.** Creation, attachment, use, result injection, and
   lifecycle completion are separate claims.
4. **Keep captured and rerun evidence distinct.** A stored observation is not a
   fresh execution. Never rewrite its status merely because a local audit passes.
5. **Negative results are first-class.** Preserve arguments, errno, conditions,
   timeout, and exit status.
6. **Use three experiment outcomes.** `0` means ran and matched; `1` means ran and
   contradicted; `2` means the prerequisite was unavailable. A skip is not a pass.
7. **Do not pipe away the producing exit code.** Capture it before formatting or
   logging output.
8. **Fail closed on requested semantics.** Compatibility may degrade explicitly;
   isolation may not.
9. **Never hand-edit generated paper files.** Edit `paper/paper.md` or the builder,
   rebuild, render every page, and inspect the latest images.
10. **Expire live-policy claims.** Re-probe after boot, kernel, policy canary, or
    execution-host changes.
11. **Prefer Markdown additions.** Put operational knowledge in a discoverable
    Markdown document before adding a renderer-specific representation.

## Experiment contract

Every numbered experiment must:

- state one question;
- use only repository-local inputs unless its purpose explicitly tests the host;
- print relevant conditions;
- write a deterministic log path;
- enforce a timeout for potentially blocking work;
- exit `0`, `1`, or `2` according to the rule above;
- have a ledger entry or explain why it is purely structural.

## Completion gate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-gate.ps1
```

Completion requires all checks to pass and the latest rendered PDF pages to have
no clipping, overlap, broken tables, missing headers, or unreadable glyphs.
