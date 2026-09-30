# Code rules

Use one parser, reader, writer, and permission check for each shared action.
A fix must reach every caller of that action.
Keep the interface small and typed. Convert errors at the process boundary.

Validate shape, length, version, and integrity before use.
Count bytes received instead of trusting a declared length.
Reject a mismatch. Do not pad or silently truncate data.
Persisted formats need an explicit version.

Keep the changing mechanism behind an existing interface.
Do not add unused frameworks, duplicate implementations, or dead code.
A compatibility path needs a current caller and a stated condition.

Bound buffers, retries, network operations, and child waits.
Check free blocks and inodes before large writes.
Use a cryptographic source for tokens.
Do not print credential values in errors or logs.

Comments state a constraint or invariant.
Use UTC for stored timestamps and correct units for measurements.
Structured output is read by field name.

## Verification

Use pure tests for deterministic logic.
Use fault tests for conditions a real service cannot produce on demand.
Use integration and deployment proof for the actual default path.
State what each kind proves.

A test must reject the defect it claims to detect.
A mock does not prove a live service or deployment.
A skipped test proves nothing about its subject.
Add a regression check for a material defect.
Repeat a check only when a change or unresolved fault justifies it.
