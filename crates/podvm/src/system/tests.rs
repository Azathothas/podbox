use super::*;

#[test]
fn system_usage_is_plain_ascii() {
    // TODO/cli.md T-1336: `system --help` and its subverb usages
    // print on a constrained host; a glyph there is unrenderable.
    let bad = SYSTEM_USAGE.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "system usage carries {bad} non-ASCII bytes");
}

/// ⛔ Every declared field is built, and every built field is declared. A
/// name in one list and not the other is a field that `--format` accepts
/// and renders blank, which is the wrong answer that looks right.
#[test]
fn the_declared_fields_are_the_built_ones() {
    let built: Vec<&str> = fields_from(Rung::Chroot, false, "/nowhere".into())
        .iter()
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(built, FIELDS);
    for d in DOCUMENTS {
        assert!(FIELDS.contains(d), "{d} is a document and not a field");
    }
}

/// ⭐ T-0804 rule 4, as an assertion, for the namespace rung
/// (TODO/enter.md T-1339). Where the probe selects `namespace`,
/// podvm enters it: `.EnteredRung` reads `namespace`, equal to
/// `.Rung`, because "what podvm did" and "what this machine
/// would permit" agree here. The divergence the rule guards is
/// covered the other way by the userland test below.
#[test]
fn the_entered_rung_is_namespace_where_selected() {
    let fields = fields_from(Rung::Namespace, true, "/nowhere".into());
    assert_eq!(pick(&fields, "Rung"), "namespace");
    assert_eq!(pick(&fields, "EnteredRung"), "namespace");
}

/// ⭐ TODO/enter.md T-1317. Where the probe selects `interpose`
/// (chroot denied), the entered sequence is `userland`, not the
/// selection: the field names what `run` enters with.
#[test]
fn the_entered_rung_is_userland_where_chroot_is_denied() {
    let fields = fields_from(Rung::Interpose, false, "/nowhere".into());
    assert_eq!(pick(&fields, "Rung"), "interpose");
    assert_eq!(pick(&fields, "EnteredRung"), "userland");
    let fields = fields_from(Rung::Chroot, false, "/nowhere".into());
    assert_eq!(pick(&fields, "EnteredRung"), "chroot");
}

/// ⭐ T-0801's `Prove`, without a shell: the template it names renders the
/// table, and what comes out parses as an array of the right shape.
#[test]
fn the_acceptance_template_renders_the_whole_table() {
    let fields = vec![("Parity", parity::json())];
    let out = format::render_with_documents("{{json .Parity}}", &fields, DOCUMENTS).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = doc.as_array().unwrap();
    assert!(arr.len() >= 60, "{} rows", arr.len());
    assert!(arr.iter().all(|r| {
        matches!(
            r["status"].as_str(),
            Some("Native") | Some("Degraded") | Some("Stub") | Some("None")
        )
    }));
}

/// TODO/cli.md T-1337: `system df` decides its arguments before it
/// touches the store, so `--help` prints without one and a bad flag
/// is a flag error without one. Both arms run hermetic here.
#[test]
fn df_help_and_bad_flag_need_no_store() {
    assert_eq!(df("system df", &["--help".to_string()]), 0);
    assert_eq!(
        df("system df", &["--bogus".to_string()]),
        podvm_image::error::EXIT_FLAG_ERROR
    );
    assert_eq!(
        df("system df", &["some-image".to_string()]),
        podvm_image::error::EXIT_FLAG_ERROR
    );
}

/// TODO/cli.md T-1337: IMAGE ID is docker's twelve hex digits, and
/// a digest that does not parse still prints its row.
#[test]
fn short_id_is_twelve_hex() {
    assert_eq!(
        short_id("sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"),
        "833d7afe7d42"
    );
    assert_eq!(short_id("bogus"), "bogus");
}

/// T-1519 (`80-interposer-abi.sh` check E). `system abi` reports three
/// exit codes: 0 admitted, 1 refused with the reason on stderr, 2 a file
/// podvm could not read. The unreadable and refused arms are pinned
/// here with hermetic inputs: two bogus paths for 2, and the test
/// binary against itself for 1 (same machine, but no libc SONAME or
/// libc name, so gate 2 of `admits` refuses on every host). The
/// admitted arm needs a real admitted pair and stays with the live
/// `80` pairings; only the return codes are asserted, never the
/// streams.
#[test]
fn abi_reports_unreadable_refused_and_arity() {
    let bogus = |n: &str| {
        std::env::temp_dir()
            .join(format!("podvm-abi-{n}-{}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!(
        abi(
            "system abi",
            &[bogus("object"), bogus("libc")]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        ),
        2
    );
    let me = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        abi("system abi", &[me.clone(), me]),
        1,
        "the binary against itself must refuse, not admit"
    );
    assert_eq!(
        abi("system abi", &["only-one-path".to_string()]),
        EXIT_CLI_ERROR
    );
}
