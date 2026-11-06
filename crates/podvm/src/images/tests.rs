use super::*;

#[test]
fn every_image_usage_is_plain_ascii() {
    // TODO/cli.md T-1336: every image verb's `--help` prints bytes
    // 0x00-0x7F only; the usages are constants, so one test holds
    // all thirteen.
    let text = [
        PULL_USAGE,
        EXTRACT_USAGE,
        IMAGES_USAGE,
        RMI_USAGE,
        TAG_USAGE,
        PRUNE_USAGE,
        INSPECT_USAGE,
        LOGIN_USAGE,
        LOGOUT_USAGE,
        SAVE_USAGE,
        LOAD_USAGE,
        IMPORT_USAGE,
        VERIFY_USAGE,
    ]
    .concat();
    let bad = text.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "image usage carries {bad} non-ASCII bytes");
}

/// TODO/image.md T-1320. save/load/import parse their own surface and
/// nothing else's.
#[test]
fn save_load_import_parse_their_flags() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_save(&v(&["-o", "f.tar", "img:tag"])).unwrap();
    assert_eq!(o.output.as_deref(), Some("f.tar"));
    assert_eq!(o.want, "img:tag");
    let o = parse_save(&v(&["--output=f.tar", "img"])).unwrap();
    assert_eq!(o.output.as_deref(), Some("f.tar"));
    assert!(parse_save(&v(&[])).is_err());
    assert!(parse_save(&v(&["a", "b"])).is_err());
    let o = parse_load(&v(&["-i", "f.tar"])).unwrap();
    assert_eq!(o.input.as_deref(), Some("f.tar"));
    let o = parse_load(&v(&[])).unwrap();
    assert_eq!(o.input, None);
    let o = parse_import(&v(&["root.tar", "me:v1"])).unwrap();
    assert_eq!(o.tar, "root.tar");
    assert_eq!(o.reference.as_deref(), Some("me:v1"));
    let o = parse_import(&v(&["root.tar"])).unwrap();
    assert_eq!(o.reference, None);
    assert!(parse_import(&v(&[])).is_err());
}

/// TODO/image.md T-1321. `verify` takes one image, `all`, or nothing.
#[test]
fn verify_parses_one_image_all_or_nothing() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_verify(&v(&["img:tag"])).unwrap();
    assert_eq!(o.want.as_deref(), Some("img:tag"));
    let o = parse_verify(&v(&["all"])).unwrap();
    assert_eq!(o.want.as_deref(), Some("all"));
    let o = parse_verify(&v(&[])).unwrap();
    assert_eq!(o.want, None);
    assert!(parse_verify(&v(&["a", "b"])).is_err());
}

/// TODO/cli.md T-1417. The Config half renders docker's key names
/// with null where the image omits a field, flat strings space-joined
/// with a dash where absent, and a missing blob renders null rather
/// than failing the document.
#[test]
fn config_renders_docker_names_null_safe() {
    let full = Some(podvm_image::oci::RunConfig {
        entrypoint: Some(vec!["/entry".into()]),
        cmd: Some(vec!["a".into(), "b".into()]),
        env: vec!["A=1".into()],
        working_dir: "/w".into(),
        user: "u".into(),
    });
    let doc = config_json(&full);
    assert_eq!(doc["Env"], serde_json::json!(["A=1"]));
    assert_eq!(doc["Cmd"], serde_json::json!(["a", "b"]));
    assert_eq!(doc["Entrypoint"], serde_json::json!(["/entry"]));
    assert_eq!(doc["WorkingDir"], serde_json::json!("/w"));
    assert_eq!(doc["User"], serde_json::json!("u"));
    let flat = config_flat(&full);
    assert_eq!(flat.0, "A=1");
    assert_eq!(flat.1, "a b");
    // Omitted fields are null in JSON and a dash flat. An explicitly
    // cleared entrypoint (`Some(vec![])`) is an empty array, not null:
    // None and Some([]) stay apart, as the parser keeps them.
    let thin = Some(podvm_image::oci::RunConfig {
        entrypoint: Some(vec![]),
        ..Default::default()
    });
    let doc = config_json(&thin);
    assert!(doc["Env"].is_null(), "{doc}");
    assert!(doc["Cmd"].is_null(), "{doc}");
    assert_eq!(doc["Entrypoint"], serde_json::json!([]));
    assert!(doc["WorkingDir"].is_null(), "{doc}");
    assert!(doc["User"].is_null(), "{doc}");
    let flat = config_flat(&thin);
    assert_eq!(
        flat,
        ("-".into(), "-".into(), "".into(), "-".into(), "-".into())
    );
    let doc = config_json(&None);
    assert!(doc["Env"].is_null() && doc["User"].is_null(), "{doc}");
    // A record whose config blob is absent renders the same nulls.
    let store = Store::open(std::env::temp_dir().join(format!("podvm-cfg-{}", std::process::id())))
        .unwrap();
    assert!(run_config_of(&store, &a_record()).is_none());
    let fields = inspect_fields(&a_record(), &store);
    assert_eq!(pick(&fields, "Config.Env"), "-");
    assert_eq!(pick(&fields, "Config.User"), "-");
    let _ = std::fs::remove_dir_all(store.root());
}

/// TODO/image.md T-1419. A record naming blobs the store does not
/// hold lists every one of them as missing: presence is what `images`
/// marks and what `run` refuses.
#[test]
fn missing_blobs_lists_blobs_with_no_file() {
    let store =
        Store::open(std::env::temp_dir().join(format!("podvm-miss-{}", std::process::id())))
            .unwrap();
    let missing = missing_blobs(&store, &a_record());
    assert_eq!(missing.len(), 3, "{missing:?}");
    assert!(
        missing.iter().all(|b| b.starts_with("sha256:")),
        "{missing:?}"
    );
    let _ = std::fs::remove_dir_all(store.root());
}

#[test]
fn the_table_pads_every_column_to_its_widest_cell() {
    let rows = vec![
        vec!["REPOSITORY".to_string(), "TAG".to_string()],
        vec!["alpine".to_string(), "latest".to_string()],
    ];
    let got = table(&rows);
    let lines: Vec<&str> = got.lines().collect();
    assert!(lines[0].starts_with("REPOSITORY   TAG"), "{got}");
    assert!(lines[1].starts_with("alpine       latest"), "{got}");
    // ⚠ No trailing padding on the last column: a trailing run of spaces
    // is invisible in review and breaks an exact-match acceptance.
    assert!(!lines[1].ends_with(' '), "{got:?}");
}

#[test]
fn the_field_names_are_the_names_the_builder_produces() {
    // ⛔ The remedy docs/conventions/forbidden-patterns.md names for a value
    // in two places: a check that they agree. The list is what validates a
    // template with no record in hand; the builder is what fills one in.
    let store =
        Store::open(std::env::temp_dir().join(format!("podvm-names-{}", std::process::id())))
            .unwrap();
    let r = a_record();
    let built: Vec<&str> = image_fields(&r, &store, false)
        .iter()
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(built, IMAGE_FIELDS);
    let built: Vec<&str> = inspect_fields(&r, &store).iter().map(|(k, _)| *k).collect();
    assert_eq!(built, INSPECT_FIELDS);
    let _ = std::fs::remove_dir_all(store.root());
}

#[test]
fn a_template_naming_a_field_no_verb_has_is_refused_without_any_record() {
    // ⭐ The defect this pass found: with the check inside the loop, an
    // empty store made `--format '{{.Nope}}'` print nothing and exit 0.
    assert!(format::check("{{.Nope}}", IMAGE_FIELDS).is_err());
    assert!(format::check("{{.Tag", IMAGE_FIELDS).is_err());
    assert!(format::check("table {{.Tag}}", IMAGE_FIELDS).is_err());
    assert!(format::check("{{.Digest}}", IMAGE_FIELDS).is_ok());
    // ⚠ The two verbs do not have the same fields, and each is checked
    // against its own list.
    assert!(format::check("{{.RepoTags}}", IMAGE_FIELDS).is_err());
    assert!(format::check("{{.RepoTags}}", INSPECT_FIELDS).is_ok());
}

fn a_record() -> Record {
    Record {
        repository: "docker.io/library/alpine".into(),
        tag: Some("latest".into()),
        digest: format!("sha256:{}", "1".repeat(64)),
        digest_media_type: podvm_image::oci::MEDIA_OCI_INDEX.into(),
        manifest_digest: format!("sha256:{}", "2".repeat(64)),
        config_digest: format!("sha256:{}", "3".repeat(64)),
        platform: "linux/amd64".into(),
        layers: vec![],
        stored_bytes: 0,
        architecture: "amd64".into(),
        os: "linux".into(),
        created: None,
        pulled_at: clock::now(),
    }
}

#[test]
fn an_image_with_no_creation_timestamp_prints_a_dash_and_not_a_guess() {
    // ⛔ AGENTS.md absolute 3.
    let store =
        Store::open(std::env::temp_dir().join(format!("podvm-fields-{}", std::process::id())))
            .unwrap();
    let f = image_fields(&a_record(), &store, false);
    assert_eq!(pick(&f, "CreatedSince"), "-");
    assert_eq!(pick(&f, "CreatedAt"), "-");
    assert_eq!(pick(&f, "ID"), "333333333333");
    assert_eq!(pick(&f, "Repository"), "alpine");
    let _ = std::fs::remove_dir_all(store.root());
}

fn login_args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test]
fn login_takes_a_user_a_stdin_flag_and_a_server() {
    let got = parse_login_args(
        "login",
        &login_args(&["-u", "bob", "--password-stdin", "example.com:5000"]),
    )
    .expect("parses");
    assert_eq!(got.username, "bob");
    assert_eq!(got.server.as_deref(), Some("example.com:5000"));
    assert!(!got.help);
}

#[test]
fn login_names_no_server_where_none_is_given() {
    // The caller defaults it to docker's canonical key.
    let got = parse_login_args(
        "login",
        &login_args(&["--username", "bob", "--password-stdin"]),
    )
    .expect("parses");
    assert!(got.server.is_none());
}

#[test]
fn login_accepts_the_equals_forms() {
    // ⛔ `admit` cuts at `=` before the table lookup, so these already
    // passed `admit_all`: without an arm they would read as a defect in
    // podvm rather than a value it accepts.
    let got = parse_login_args(
        "login",
        &login_args(&["--username=bob", "--password-stdin"]),
    )
    .expect("parses");
    assert_eq!(got.username, "bob");
    let got =
        parse_login_args("login", &login_args(&["-u=bob", "--password-stdin"])).expect("parses");
    assert_eq!(got.username, "bob");
}

#[test]
fn login_help_needs_nothing_else() {
    let got = parse_login_args("login", &login_args(&["--help"])).expect("parses");
    assert!(got.help);
}

#[test]
fn login_without_a_user_is_a_flag_refusal() {
    // TODO/cli.md T-0802: a flag-shape refusal is 125.
    assert_eq!(
        parse_login_args("login", &login_args(&["--password-stdin"])).expect_err("no user"),
        EXIT_FLAG_ERROR
    );
}

#[test]
fn login_without_password_stdin_is_a_flag_refusal() {
    // Podvm never prompts, so the flag is required, not defaulted.
    assert_eq!(
        parse_login_args("login", &login_args(&["-u", "bob"])).expect_err("no stdin flag"),
        EXIT_FLAG_ERROR
    );
}

/// T-0209: a terminal stdin refuses before any read. The reader
/// panics where touched, so passing proves the refusal precedes it:
/// a hang would never return to assert against.
#[test]
fn a_terminal_stdin_refuses_before_any_read() {
    struct PanicOnRead;
    impl std::io::Read for PanicOnRead {
        fn read(&mut self, _b: &mut [u8]) -> std::io::Result<usize> {
            panic!("the refusal must precede the read");
        }
    }
    assert_eq!(
        read_password("login", true, &mut PanicOnRead).unwrap_err(),
        EXIT_CLI_ERROR
    );
}

/// T-0209: a pipe reads to the trimmed password, and an empty one
/// refuses like any post-parse verb refusal.
#[test]
fn a_pipe_stdin_reads_to_the_trimmed_password() {
    let mut full: &[u8] = b"s3cret\r\n";
    assert_eq!(read_password("login", false, &mut full).unwrap(), "s3cret");
    let mut empty: &[u8] = b"\n";
    assert_eq!(
        read_password("login", false, &mut empty).unwrap_err(),
        EXIT_CLI_ERROR
    );
}

#[test]
fn login_with_a_dangling_user_flag_is_a_flag_refusal() {
    assert_eq!(
        parse_login_args("login", &login_args(&["-u"])).expect_err("dangling"),
        EXIT_FLAG_ERROR
    );
}

#[test]
fn login_with_an_empty_user_is_a_flag_refusal() {
    // The read path skips entries with no user, so storing one would
    // write a login that never reads back.
    assert_eq!(
        parse_login_args("login", &login_args(&["-u", "", "--password-stdin"]))
            .expect_err("empty user"),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse_login_args("login", &login_args(&["--username=", "--password-stdin"]))
            .expect_err("empty equals user"),
        EXIT_FLAG_ERROR
    );
}

/// T-0209: logout takes one optional server and help, and a second
/// server is a post-parse refusal rather than a silent pick.
#[test]
fn logout_takes_one_server_or_help() {
    let got = parse_logout_args("logout", &login_args(&[])).expect("bare");
    assert!(got.server.is_none() && !got.help);
    let got = parse_logout_args("logout", &login_args(&["example.com:5000"])).expect("server");
    assert_eq!(got.server.as_deref(), Some("example.com:5000"));
    let got = parse_logout_args("logout", &login_args(&["--help"])).expect("help");
    assert!(got.help);
    assert_eq!(
        parse_logout_args("logout", &login_args(&["a.example", "b.example"])).expect_err("two"),
        EXIT_CLI_ERROR
    );
}

#[test]
fn login_with_two_servers_is_a_verb_refusal() {
    // TODO/cli.md T-0802: the verb refusing afterwards is 1.
    assert_eq!(
        parse_login_args(
            "login",
            &login_args(&["-u", "bob", "--password-stdin", "a.example", "b.example"])
        )
        .expect_err("two servers"),
        EXIT_CLI_ERROR
    );
}

#[test]
fn login_with_an_unlisted_flag_is_a_flag_refusal() {
    assert_eq!(
        parse_login_args(
            "login",
            &login_args(&["-u", "bob", "--password-stdin", "--quiet"])
        )
        .expect_err("unlisted"),
        EXIT_FLAG_ERROR
    );
}

/// A scratch store for the hold/refusal tests below. `std` only: this
/// crate has no `tempfile` dependency and the store tests' `scratch`
/// helper is `#[cfg(test)]`-private to `podvm-image`.
fn scratch_store(name: &str) -> std::path::PathBuf {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root =
        std::env::temp_dir().join(format!("podvm-test-{}-{}-{}", std::process::id(), n, name));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// A minimal held-image record. Same shape as the store suite's own
/// `record` helper: canonical repository, tag, three digests, one
/// platform, two layers.
fn held_record() -> podvm_image::Record {
    let d = |b: u8| podvm_image::digest::Digest::of(&[b, 9]).to_string();
    podvm_image::Record {
        repository: "docker.io/library/alpine".into(),
        tag: Some("latest".into()),
        digest: d(1),
        digest_media_type: "application/vnd.oci.image.index.v1+json".into(),
        manifest_digest: d(2),
        config_digest: d(3),
        platform: "linux/amd64".into(),
        layers: vec![d(4), d(5)],
        stored_bytes: 100,
        architecture: "amd64".into(),
        os: "linux".into(),
        created: Some("2024-01-01T00:00:00Z".into()),
        pulled_at: "2024-01-01T00:00:00Z".into(),
    }
}

/// T-1517 (`160-store-gc.sh` clause 1). `rmi` on a held image exits
/// non-zero naming `in use`: `store.remove` takes `Held::Refuse` and
/// `rmi` reports the resulting error's own exit code.
#[test]
fn rmi_refuses_a_held_image() {
    let root = scratch_store("rmi-held");
    let store = podvm_image::store::Store::open(&root).unwrap();
    let record = held_record();
    store.put_record(record.clone()).unwrap();
    let held = store.hold(&record).unwrap();
    let error = store.remove("alpine:latest").unwrap_err();
    assert!(
        format!("{error}").contains("in use"),
        "held removal did not say in use: {error}"
    );
    assert_eq!(
        error.exit_code(),
        podvm_image::error::EXIT_RUNTIME_ERROR,
        "held removal must exit non-zero the way rmi reports it"
    );
    drop(held);
    assert!(
        store.remove("alpine:latest").is_ok(),
        "rmi refused an image nothing is using"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// T-1518 (`160-store-gc.sh` clause 2, defect decided in TODO/cli.md
/// T-0802). No source defect: `prune` on a held image exits 0, prints
/// `skipped:`, and leaves the image listed. `Store::prune` is the
/// CLI's data path minus the supervise record gate: candidates, then
/// `delete` with `Held::Skip`, whose `skipped` list is exactly what
/// `prune` prints after `skipped: `.
#[test]
fn prune_skips_a_held_image() {
    let root = scratch_store("prune-held");
    let store = podvm_image::store::Store::open(&root).unwrap();
    let record = held_record();
    store.put_record(record.clone()).unwrap();
    let held = store.hold(&record).unwrap();
    let pruned = store.prune(true).unwrap();
    assert_eq!(pruned.skipped, vec!["alpine:latest".to_string()]);
    assert!(pruned.deleted.is_empty());
    assert!(
        store.find("alpine:latest").unwrap().len() == 1,
        "prune removed an image it only skipped"
    );
    drop(held);
    let _ = std::fs::remove_dir_all(&root);
}
