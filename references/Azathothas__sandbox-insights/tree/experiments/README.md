# Experiments

The numbered experiments audit the repository's own evidence and calculations.
They are non-interactive and use only local inputs.

| # | Script | Question | Success condition |
|---|---|---|---|
| 10 | `10-artifact-inventory.ps1` | is the agent-first artifact complete and free of embedded revision IDs? | required files and canonical 0BSD text exist; no 40-character revision is embedded |
| 20 | `20-evidence-audit.ps1` | does every evidence row resolve inside this repository? | all IDs, classes, paths, anchors, and conditions validate |
| 30 | `30-mechanism-consistency.ps1` | do the core claims agree across local Markdown and data? | all cross-document invariants hold |
| 40 | `40-tradeoff-model.ps1` | do local benchmark samples derive the reported interval? | sample counts and checksum match; derived interval remains 20-25x |

## Exit codes

- `0`: ran and matched the written expectation;
- `1`: ran and contradicted it;
- `2`: could not run because a tool or prerequisite was unavailable.

## Data

- `data/benchmark-samples.csv` contains the six comparison samples used by
  experiment 40.
- `data/policy-transitions.csv` records the two time-varying policy examples used
  to justify epoch-aware probing.
- `evidence-ledger.csv` maps publishable claims to literal anchors and conditions
  in local files.

Logs are written under `logs/`. A rerun may replace a log only when the question
and input schema are unchanged. The Linux-specific observations in
`docs/observations.md` are captured records, not fresh executions by these Windows
audit scripts.
