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

The [secret check](../../scripts/common/check-no-secrets.sh) detects known shapes.
Use its public mode before publication.
Read the matches. A green scan does not inspect all possible meanings.
[Public rules](../public/README.md) define the wider publication scope.

## Incident

If a credential enters the tree or a log, tell the operator.
Remove the value from current files.
The operator must rotate it.
A history rewrite does not invalidate the exposed value.
Do not rewrite published history without the operator's direction.
