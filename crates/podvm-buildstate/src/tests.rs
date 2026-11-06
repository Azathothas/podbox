use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn scratch() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = env::temp_dir().join(format!(
        "podvm-buildstate-test-{}-{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn objects_sort_keys_with_python_separators() {
    let mut map = BTreeMap::new();
    map.insert("b".to_string(), "2".to_string());
    map.insert("a".to_string(), "1".to_string());
    let mut out = String::new();
    json_object_sorted(&map, &mut out);
    assert_eq!(out, "{\"a\": \"1\", \"b\": \"2\"}");
}

#[test]
fn record_json_matches_python_key_order() {
    let mut outputs = BTreeMap::new();
    outputs.insert("podvm".to_string(), "aa".to_string());
    outputs.insert("node".to_string(), "bb".to_string());
    assert_eq!(
            record_json("in", &outputs),
            "{\"inputs\": \"in\", \"outputs\": {\"node\": \"bb\", \"podvm\": \"aa\"}, \"schema\": \"podvm-build/1\"}"
        );
}

#[test]
fn status_names_every_state_on_a_fixture() {
    let root = scratch();
    let binary = root.join("podvm");
    // Absent binary.
    assert_eq!(status(&root, "t", &binary), ("absent".to_string(), 2));
    fs::write(&binary, b"fixture-output\n").unwrap();
    for helper in ["node", "operator", "proxy", "shell"] {
        fs::write(root.join(helper), b"fixture-output\n").unwrap();
    }
    // No record.
    assert_eq!(status(&root, "t", &binary).1, 1);
    // Unreadable record.
    fs::create_dir_all(root.join(".dev")).unwrap();
    fs::write(root.join(".dev/build-state.json"), b"not json").unwrap();
    assert_eq!(
        status(&root, "t", &binary),
        ("stale: unreadable build record".to_string(), 1)
    );
    // Ready record through the writer itself. status() digests with
    // tools=true, so the fixture record must too, or the comparison
    // is against a different digest.
    let inputs = inputs_digest(&root, "t", true).unwrap();
    let outputs = outputs_digest(&binary).unwrap();
    fs::write(
        root.join(".dev/build-state.json"),
        record_json(&inputs, &outputs) + "\n",
    )
    .unwrap();
    assert_eq!(status(&root, "t", &binary), ("ready".to_string(), 0));
    // Changed output bytes.
    fs::write(&binary, b"different-output\n").unwrap();
    assert_eq!(status(&root, "t", &binary).1, 1);
    // Missing output.
    fs::remove_file(&binary).unwrap();
    assert_eq!(status(&root, "t", &binary).1, 2);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn walk_skips_target_and_pycache_parts() {
    let root = scratch();
    fs::create_dir_all(root.join("crates/a/target")).unwrap();
    fs::create_dir_all(root.join("scripts/__pycache__")).unwrap();
    fs::write(root.join("crates/a/keep.rs"), b"x").unwrap();
    fs::write(root.join("crates/a/target/drop.rs"), b"x").unwrap();
    fs::write(root.join("scripts/__pycache__/drop.pyc"), b"x").unwrap();
    let mut paths = BTreeSet::new();
    walk(&root.join("crates"), &root, &mut paths).unwrap();
    walk(&root.join("scripts"), &root, &mut paths).unwrap();
    assert_eq!(paths.len(), 1);
    assert!(paths.contains("crates/a/keep.rs"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn record_round_trips_through_the_reader() {
    let mut outputs = BTreeMap::new();
    outputs.insert("podvm".to_string(), "aa".to_string());
    let text = record_json("in", &outputs);
    assert_eq!(field_string(&text, "\"schema\"").as_deref(), Some(SCHEMA));
    assert_eq!(field_string(&text, "\"inputs\"").as_deref(), Some("in"));
    let raw = field_raw(&text, "\"outputs\"").unwrap();
    assert_eq!(parse_strmap(&raw).unwrap(), outputs);
    assert!(field_string("not json", "\"inputs\"").is_none());
    assert!(field_string("{\"inputs\": 1}", "\"inputs\"").is_none());
}
