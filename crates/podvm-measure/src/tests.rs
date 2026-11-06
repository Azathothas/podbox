use super::*;

#[test]
fn table_rows_match_verb_and_flag_prefix() {
    let rows = vec![
        Json::Object(vec![
            ("verb".to_string(), Json::Str("run".to_string())),
            ("flag".to_string(), Json::Str("--podvm-tier".to_string())),
        ]),
        Json::Object(vec![("verb".to_string(), Json::Str("docker".to_string()))]),
    ];
    assert!(table_carries(&rows, "run", "--podvm-tier"));
    assert!(table_carries(&rows, "docker", "-"));
    assert!(!table_carries(&rows, "podman", "-"));
    assert!(!table_carries(&rows, "run", "--podvm-qemu-arg"));
    assert!(!table_carries(&rows, "exec", "--podvm-tier"));
}

#[test]
fn help_words_split_like_the_script_greps() {
    let words = help_words("run exec docker --podvm-tier\n  spaced words");
    for word in ["run", "exec", "docker", "--podvm-tier", "spaced", "words"] {
        assert!(words.contains(word));
    }
}

#[test]
fn exit_codes_and_parity_parse() {
    let codes = parse_json("[{\"case\": \"runtime-error\", \"code\": 1}]").unwrap();
    match &codes {
        Json::Array(rows) => {
            assert_eq!(
                field(&rows[0], "case"),
                Some(&Json::Str("runtime-error".to_string()))
            );
            assert_eq!(
                field(&rows[0], "code"),
                Some(&Json::Number("1".to_string()))
            );
        }
        _ => panic!("array expected"),
    }
    assert!(parse_json("[{\"a\": }]").is_none());
    assert!(parse_json("{\"verb\": \"run\"} trailing").is_none());
}
