use super::*;

#[test]
fn mutation_anchors_match_the_current_source_shape() {
    // The ports encode today's text; a drifted anchor passes vacuously,
    // which is the defect wave 0 exists to prevent. The anchor strings
    // are asserted non-empty here, and the lane proof runs the mutation
    // that proves they land.
    assert!(FORK_PATTERN.contains("close_in_children"));
    assert!(EXEC_PATTERN.contains("O_CLOEXEC"));
    assert_ne!(FORK_PATTERN, FORK_REPLACE);
    assert_ne!(EXEC_PATTERN, EXEC_REPLACE);
}

#[test]
fn subjects_name_the_two_t0211_tests() {
    assert!(FORK_TEST.contains("fork"));
    assert!(EXEC_TEST.contains("inherit"));
    assert_eq!(SUBJECT, "crates/podvm-image/src/store.rs");
}
