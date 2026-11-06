use super::*;

#[test]
fn release_tags_match_the_script_expression() {
    assert!(valid_release("v0.1.0-beta.12"));
    assert!(valid_release("v10.20.30-beta.1"));
    assert!(!valid_release("v0.1.0"));
    assert!(!valid_release("v0.1.0-beta."));
    assert!(!valid_release("beta.12"));
    assert!(!valid_release("v0.1.0-beta.12 "));
    assert!(!valid_release("v0.1.0-beta.1.2"));
    assert!(!valid_release("not-a-tag"));
}

#[test]
fn recipe_arches_read_one_per_line_and_unique() {
    let recipe = "jobs:\n  build:\n    strategy:\n      matrix:\n        include:\n          - arch: x86_64\n          - arch: aarch64\n";
    assert_eq!(
        recipe_arches(recipe).unwrap(),
        vec!["x86_64".to_string(), "aarch64".to_string()]
    );
    let dup = "          - arch: x86_64\n          - arch: x86_64\n";
    assert!(recipe_arches(dup).is_none());
    assert!(recipe_arches("no matrix here").is_none());
}

#[test]
fn reader_and_shapes_cover_gh_answers() {
    let parsed = parse_json("{\"tagName\": \"v1\", \"isPrerelease\": true, \"assets\": [{\"name\": \"a\", \"size\": 3}]}").unwrap();
    assert_eq!(parsed.get("tagName").and_then(Json::as_str), Some("v1"));
    assert_eq!(
        parsed.get("isPrerelease").and_then(Json::as_bool),
        Some(true)
    );
    assert_eq!(parsed.describe(), "object of 3 pairs");
}
