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
    assert_eq!(SUBJECT, "crates/pod-supervise/src/lock.rs");
}

#[test]
fn the_podcrates_packages_come_from_the_lock_file_and_each_one_moves() {
    let lock = "[[package]]\nname = \"pod-core\"\nversion = \"0.21.0\"\nsource = \"git+https://github.com/PodUtils/podcrates?tag=v0.21.0#ada3699\"\n\n[[package]]\nname = \"podvm\"\nversion = \"0.2.0\"\n\n[[package]]\nname = \"pod-supervise\"\nversion = \"0.21.0\"\nsource = \"git+https://github.com/PodUtils/podcrates?tag=v0.21.0#ada3699\"\n";
    let names = podcrates_packages(lock);
    assert_eq!(names, ["pod-core", "pod-supervise"]);
    let copy = std::env::temp_dir().join(format!("podvm-t0211-plant-{}", std::process::id()));
    assert!(
        patch_args(&copy, &names).is_none(),
        "a missing crate must refuse"
    );
    for name in &names {
        fs::create_dir_all(copy.join("crates").join(name)).unwrap();
        fs::write(copy.join("crates").join(name).join("Cargo.toml"), "").unwrap();
    }
    let args = patch_args(&copy, &names).unwrap();
    assert_eq!(args.len(), 4);
    assert!(args[1].starts_with("patch.\"https://github.com/PodUtils/podcrates\".pod-core.path="));
    fs::remove_dir_all(&copy).ok();
}
