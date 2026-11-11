//! TODO/gate.md T-1672: pod-todo's reader finds no problem of form in
//! podvm's record. A problem of form is one of an entry's shape: its
//! heading, its fields and their values, its sections, a row and its entry,
//! and the counts. The problems of citations and words (a path of the
//! history, an id of another repository) are T-1680's: there the reader of
//! pod-todo takes the record half of `podvm-gate`.

use std::path::Path;

/// The words of each problem of form that pod-todo reports (`check.rs` and
/// `parse.rs` of pod-todo at the tag of `Cargo.toml`).
const FORM: &[&str] = &[
    "an H1 that is not an entry",
    "is not a field of an entry",
    "text before the first field",
    "the field ",
    "is not one of",
    "the source is empty",
    "is not a section of an entry",
    "is out of order or repeated",
    "` is empty",
    "the section `## ",
    "a Prove with no command",
    "with no `## Blocker`",
    "a `## Blocker` on an entry",
    "with no `## Done` evidence",
    "`## Done` gives no date",
    "a `## Done` on an entry",
    "has a row and no entry",
    "has an entry and no row",
    "a row must be",
    "the counts line",
    "row of the counts table",
    "the roadmap is missing",
];

#[test]
fn the_record_has_no_problem_of_form_for_pod_todo() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // No base: the check of citations left behind by an edit needs git
    // and a change, and this test reads the tree as it is.
    let report = pod_todo::check::check_with(&root, Some(Vec::new()), &|_: &str| Ok(None), "HEAD");
    let form: Vec<String> = report
        .problems
        .iter()
        .map(|p| p.to_string())
        .filter(|p| FORM.iter().any(|w| p.contains(w)))
        .collect();
    assert!(
        form.is_empty(),
        "{} problems of form:\n{}",
        form.len(),
        form.join("\n")
    );
    assert!(
        report.counts.total() > 300,
        "the reader found {} entries",
        report.counts.total()
    );
}
