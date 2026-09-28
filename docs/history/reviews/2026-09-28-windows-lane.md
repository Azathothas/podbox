# Windows lane and work record review, 2026-09-28

Scope: the Windows wrapper, cleanup check and plant, experiment callers,
the live lane procedure, the work record, the SSH task intake, and the
two captured reference trees. The captured trees were checked as evidence
for provenance, licence, prohibited instruction files, and the cited
source paths. This was not a full source audit of either SSH tree.

## Pass 1: every door into the changed path

The caller scan covered `scripts/`, `experiments/`, `TODO/`, and
`docs/`. It found fourteen old experiment comments about the shared
job path. Twelve scripts already used the working directory; two
resolved the repository from `$0`. The two now select the working
directory for a Windows input and retain file-relative discovery for a
native run. The host-podman fallback uses a path inside its own
container and does not share the Windows wrapper's checkout path.

The pull request source check found a second difference between task
text and code: PR 67 dispatches `remote ssh` and has its parity row,
but names `machine ssh` in help without a dispatch arm. T-1404 now
states that exact delta.

The full plant call found two more doors into the Windows wrapper.
It started a Bash script with `sh`, then made that script resolve its
checkout from `/in/job.sh`. T-1345 made the interpreter follow an
explicit Bash first line; T-1346 made the plant input use `/work`.

## Pass 2: can the guards fail?

A live ended base session made check 29 exit 1 and name its id. Cleanup
by that id left the report empty. The plant fixture for case 29b gives
check 29 a kept session from the JSON report. The first full plant run
found case 27a green with a wrong parity note. The former PROGRESS
sentence that check 27 read was gone. T-1347 now reads the done
milestone entries. The repeat caught 43 plants, missed none, and kept
four controls quiet. The full result is recorded in
[`TODO/PROGRESS.md`](../../../TODO/PROGRESS.md).

The Windows drive started two jobs from one checkout at the same time.
Both read their own input, found executable modes, and exited 0. A
third input, an existing experiment script, named the expected
`/work` binary path before returning 2 for a binary absent from the
small test image. Job-specific collection removed all three records.
The script and raw result are in
[`experiments/384-windows-lane-v6.sh`](../../../experiments/384-windows-lane-v6.sh)
and [its result](../../../experiments/results/windows-lane-v6.txt).

## Pass 3: claims against source and live state

The old progress state and index argument were stale. Their original
text is in `docs/history/`; the live work order and counts were
rewritten. The reference map said no tree had Discussions data, while
the saved provenance files showed some gh captures did. That sentence
was corrected in place. The SSH source pass found a stale one-session
sentence in the captured dropssh README and a shell path in PR 67's
task text that the captured sandssh tree did not contain. Those
differences now appear in the SSH entries.

The publication check marked full commit identifiers and a local home
path in new prose. Short identifiers and a general location name keep
the evidence checkable through the captured provenance and pull request
records. The two upstream snapshots retain their source bytes except
for removal of agent instruction files. The whitespace check for
project-owned changes is clean; whitespace inside the captured source
is left as upstream evidence.

## Result

The owned source and document changes have no open review finding.
The SSH implementation is still open in T-1401 to T-1404. The remote
CI state for this session is recorded in the
[session summary](../../../TODO/SESSION-SUMMARY-2026-09-28.md).
