# Prose

## Scope

Use ASD-STE100 for documents, comments, help text, tasks, and chat.
Use exact technical names where a general word would be incorrect.

## Sentences

Use active voice and name the actor.
Use one instruction per sentence.
Use simple present, past, or future tense.
Keep procedure sentences within 20 words and descriptions within 25 words.
Keep paragraphs within six sentences.
Keep articles and avoid long noun groups.

State an operation, condition, result, or constraint.
Remove marketing adjectives, personal judgments, and session narrative.
Use plain technical terms for errors and remedies.

## Claims

Verify a claim against source or a repeatable run before you write it.
Cite the file, captured revision, or result.
State the measurement conditions.
Write a dash for an unknown value.
Label an estimate each time you use it.

A source declaration proves that the declaration exists.
It does not prove the runtime path works.
A historical run proves its captured revision and environment.

## One fact, one home

Keep each changing fact in one source.
Link to that source elsewhere.
Generate repeated facts or check that the copies agree.
[Document roles](docs.md) define the homes.
[The source-state generator](../../scripts/document-state.py) owns its generated page.

## Correct text in place

Replace incorrect live text.
Do not add a correction box below an obsolete instruction.
Keep required superseded wording under [history](../methodology/history.md).
A historical result stays historical.
Do not replace its measured values with values from another host.

## Characters and markers

Use ASCII prose where possible. Do not use an em dash.
These markers have fixed meanings:

| Marker | Meaning |
| --- | --- |
| ⛔ | A required constraint |
| ⭐ | A first choice |
| ⚠ | A known error condition |
| ✅ | A passed machine result |
| ❌ | A failed machine result |

Use markers only when they improve a procedure.
The [marker check](../../scripts/common/check-markers.sh) owns the character
allowlist and density limit. Status symbols do not define rules.

## Review

A linter checks links, characters, placeholders, and repeated text.
A reader checks meaning and evidence.
[Reviews](../methodology/reviews.md) define the required passes.
