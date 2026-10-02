# Secrets

## Storage

Keep credentials out of source, output, messages, and history.
This includes expired values and examples that grant access.

The operator identifies the required credential type.
Give the operator an ignored file or secret-store location.
Do not ask for the credential value in chat.
Do not replace a working secret without specific authority.

List the ignore rule before a credential file exists.
An ignore rule does not remove an already tracked file.

## Output

Log the operation and result, without the credential.
Use the shared redaction path where a field can contain a credential.
Check each output path, including errors and subprocess logs.

Do not publish private host names, account paths, or internal identifiers.
Use technical placeholders for examples.
A known checksum is not a credential.
State its purpose so the public check can distinguish it.

## Review

The [secrets workflow](../../.github/workflows/secrets.yml) runs TruffleHog
over the working tree and over the whole history.
It is the only credential scan this repository runs.
A verified result is a credential the vendor confirmed against its own API.
Read every result before publishing.
A green scan does not inspect all possible meanings.
A scan runs on every push to main, on every pull request, and on request.
[Public rules](../public/README.md) define the wider publication scope.

⛔ A SHAPE-REGEX SCANNER WAS RETIRED FROM THIS REPOSITORY ON 2026-10-02,
by operator decision, and must not come back as a second opinion.
TODO/gate.md T-1603 holds the record.
It matched credential SHAPES, and it read `scripts/dev-lane.sh`'s default
`HOME` as a fingerprint of a private machine and stayed red on it.
A regex cannot tell a checksum from a key, and this tree carries both on
purpose. A scan that cries wolf is a scan somebody turns off.

## Incident

If a credential enters the tree or a log, tell the operator.
Remove the value from current files.
The operator must rotate it.
A history rewrite does not invalidate the exposed value.
Do not rewrite published history without the operator's direction.
