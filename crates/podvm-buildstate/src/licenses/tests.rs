use super::*;

#[test]
fn identifier_matches_the_script_character_class() {
    assert!(valid_identifier("ring0.17.14"));
    assert!(valid_identifier("libc-1.0_0+qux"));
    assert!(!valid_identifier(""));
    assert!(!valid_identifier("has space"));
    assert!(!valid_identifier("uni\u{e9}code"));
}

#[test]
fn parser_reads_metadata_shapes() {
    let parsed = parse_json("{\"packages\": [{\"name\": \"a\", \"version\": \"1\", \"source\": \"x\", \"manifest_path\": \"/m/C.toml\", \"license\": null}]}").unwrap();
    let list = match parsed.get("packages") {
        Some(Json::Array(items)) => items,
        _ => panic!("packages is not an array"),
    };
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].get("name").and_then(Json::as_str), Some("a"));
    assert!(list[0].get("license").and_then(Json::as_str).is_none());
    assert!(parse_json("[1, {\"a\": true}, null]").is_some());
    assert!(parse_json("{\"a\": }").is_none());
    assert!(parse_json("{\"a\": 1} trailing").is_none());
    assert_eq!(
        parse_json("\"a\\u00e9\"").unwrap().as_str(),
        Some("a\u{e9}")
    );
    match parse_json("{\"n\": 3, \"b\": false}").unwrap() {
        Json::Object(pairs) => {
            assert!(matches!(pairs[0].1, Json::Number(ref n) if n == "3"));
            assert!(matches!(pairs[1].1, Json::Bool(false)));
        }
        _ => panic!("object expected"),
    }
}

#[test]
fn inventory_renders_like_cpython_indent_two() {
    let records = vec![Record {
        name: "a".to_string(),
        version: "1".to_string(),
        licence: None,
        files: vec![FileEntry {
            path: "LICENSE".to_string(),
            sha256: "aa".to_string(),
        }],
    }];
    assert_eq!(
            inventory_json(&records),
            "[\n  {\n    \"name\": \"a\",\n    \"version\": \"1\",\n    \"licence\": null,\n    \"files\": [\n      {\n        \"path\": \"LICENSE\",\n        \"sha256\": \"aa\"\n      }\n    ]\n  }\n]\n"
        );
    let empty = vec![Record {
        name: "a".to_string(),
        version: "1".to_string(),
        licence: Some("MIT".to_string()),
        files: vec![],
    }];
    assert!(inventory_json(&empty).contains("\"licence\": \"MIT\""));
    assert!(inventory_json(&empty).contains("\"files\": []"));
}

#[test]
fn licence_stems_match_case_insensitively() {
    assert!(licence_hit("LICENSE-MIT"));
    assert!(licence_hit("licence.txt"));
    assert!(licence_hit("COPYING"));
    assert!(licence_hit("Notice.md"));
    assert!(!licence_hit("README.md"));
}
