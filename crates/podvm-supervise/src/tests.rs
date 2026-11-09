//! podvm's glue over `pod-supervise`. The tests of the mechanism (the id,
//! the follow, the tail, the wait after issue 215) moved with it to
//! `pod_supervise` of PodUtils/podcrates.

use super::*;

#[test]
fn a_generated_name_is_short_and_derived_from_the_id() {
    let id = new_id();
    let n = generated_name(&id);
    assert!(n.starts_with("podvm_"));
    assert!(id.starts_with(&n["podvm_".len()..]));
}

/// ⛔ The settings keep what podvm wrote before the crate: the store's own
/// lock for the table, the `podvm_containers` key, the `podvm_` names, and
/// `podvm ps -a` in the miss message.
#[test]
fn the_settings_keep_the_store_lock_and_podvm_s_names() {
    let s = settings_at(std::path::Path::new("/store"));
    assert_eq!(s.root, std::path::Path::new("/store"));
    assert_eq!(s.table_lock, std::path::Path::new("/store/lock"));
    assert_eq!(s.table_key, "podvm_containers");
    assert_eq!(s.name_prefix, "podvm");
    assert_eq!(s.list_command.as_deref(), Some("podvm ps -a"));
    assert_eq!(s.failed_code, EXIT_RUNTIME_ERROR);
}

/// T-0810: creating a container makes its directory, or the memo rename
/// beside the record fails with ENOENT and no record can be inspected.
#[test]
fn creating_a_container_makes_its_directory() {
    let d = std::env::temp_dir().join(format!("podvm-mkdir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let s = Store::open(&d).unwrap();
    let c = create(
        &s,
        Some("e1"),
        "img",
        "sha256:0",
        "/tmp",
        vec!["true".into()],
        Vec::new(),
        "/".into(),
        "chroot",
        Vec::new(),
        0,
        Vec::new(),
    )
    .unwrap();
    assert!(table::dir(&s, &c.id).is_dir());
    // The shape the cli rename needs: a file moves beside the record.
    let src = d.join("ephemeral.tmp");
    std::fs::write(&src, b"memo").unwrap();
    let dest = table::memo_path(&s, &c.id);
    std::fs::rename(&src, &dest).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"memo");
    // ⭐ The table on disk keeps podvm's key and podvm's fields at the top
    // level of each record.
    let text = std::fs::read_to_string(d.join(table::TABLE_FILE)).unwrap();
    assert!(text.contains("\"podvm_containers\": 1"), "{text}");
    assert!(text.contains("\"rung\": \"chroot\""), "{text}");
    let _ = std::fs::remove_dir_all(&d);
}

/// T-1322: the reference query sees created containers, which hold no
/// lock, and only the digest it is asked about.
#[test]
fn referencing_finds_records_by_manifest_digest() {
    let d = std::env::temp_dir().join(format!("podvm-referrers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let s = Store::open(&d).unwrap();
    let mk = |name: &str, digest: &str| {
        create(
            &s,
            Some(name),
            "img",
            digest,
            "/tmp",
            vec!["true".into()],
            Vec::new(),
            "/".into(),
            "chroot",
            Vec::new(),
            0,
            Vec::new(),
        )
        .unwrap()
    };
    mk("k1", "sha256:aaa");
    mk("k2", "sha256:aaa");
    mk("k3", "sha256:bbb");
    let mut names: Vec<String> = referencing(&s, "sha256:aaa")
        .unwrap()
        .iter()
        .map(|c| c.name.clone())
        .collect();
    names.sort();
    assert_eq!(names, vec!["k1".to_string(), "k2".to_string()]);
    assert!(referencing(&s, "sha256:zzz").unwrap().is_empty());
    // A miss names podvm's command.
    let e = get(&s, "nope").unwrap_err();
    assert!(e.0.contains("`podvm ps -a` lists what there is"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}
