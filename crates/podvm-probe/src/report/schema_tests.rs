//! TODO/probe.md T-1714: `probe --json` holds the report of `pod-hostprobe`
//! (the key `hostprobe`, T-1746), and that object validates against the
//! published schema of the taken tag, `report::schema()`, with the
//! validator of podcrates' own test (boon, JSON Schema 2020-12).

use super::*;
use pod_hostprobe::report::{schema, SCHEMA_ID};

/// The schema of the taken tag, compiled.
fn validator() -> (boon::Schemas, boon::SchemaIndex) {
    let mut compiler = boon::Compiler::new();
    let mut schemas = boon::Schemas::new();
    compiler
        .add_resource(SCHEMA_ID, schema())
        .expect("a resource");
    let at = compiler
        .compile(SCHEMA_ID, &mut schemas)
        .expect("the schema compiles");
    (schemas, at)
}

#[test]
fn the_report_of_probe_json_validates_against_the_published_schema() {
    let f = crate::run();
    let sel = Selection::choose(&f);
    let doc: serde_json::Value =
        serde_json::from_str(&document(&f, &sel)).expect("probe --json is JSON");
    let report = doc.get("hostprobe").expect("probe --json holds the report");
    let (schemas, at) = validator();
    if let Err(e) = schemas.validate(report, at) {
        panic!("the report does not validate against {SCHEMA_ID}: {e:#}");
    }
}

/// The schema refuses what a reader must refuse: the validation is not a
/// pass of each object.
#[test]
fn the_schema_refuses_a_report_of_another_version_or_rung() {
    let f = crate::run();
    let sel = Selection::choose(&f);
    let doc: serde_json::Value = serde_json::from_str(&document(&f, &sel)).unwrap();
    let (schemas, at) = validator();
    let mut other = doc["hostprobe"].clone();
    other["schema"] = serde_json::json!(999);
    assert!(
        schemas.validate(&other, at).is_err(),
        "a report of version 999 passed"
    );
    let mut rung = doc["hostprobe"].clone();
    rung["selection"]["rung"] = serde_json::json!("no-such-rung");
    assert!(
        schemas.validate(&rung, at).is_err(),
        "a rung that is no word passed"
    );
}
