use super::*;

/// Sentences that the owner's rulings retired. Each stays out of the
/// product source: a message that says it misleads the user.
///
/// TODO/machine.md T-1879: the ruling "Windows media" (`docs/decisions.md`,
/// 2026-11-14) makes podvm fetch the FreeDOS base and the Windows media,
/// so no message may say that podvm fetches no base.
const RETIRED: [(&str, &str); 1] = [(
    "No base is fetched",
    "podvm fetches the base of each catalog name (`docs/decisions.md`, \
     \"Windows media\"; TODO/machine.md T-1879)",
)];

/// TODO/machine.md T-1879: no Rust source of `crates/` holds a retired
/// sentence.
pub(super) fn check_retired_sentences(gate: &mut Gate, root: &Path, files: &BTreeSet<String>) {
    for rel in files {
        if !rel.starts_with("crates/")
            || !rel.ends_with(".rs")
            || rel.starts_with("crates/podvm-gate/")
        {
            continue;
        }
        gate.bump("retired_sentences");
        let Ok(text) = read_text(&root.join(rel)) else {
            continue;
        };
        for (sentence, why) in RETIRED {
            if text.contains(sentence) {
                gate.err(
                    rel,
                    format!("says \"{sentence}\", a retired sentence: {why}"),
                );
            }
        }
    }
}
