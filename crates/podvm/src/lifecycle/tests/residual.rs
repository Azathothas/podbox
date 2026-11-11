//! TODO/supervise.md T-1708. `rm` of a stopped record whose tree left
//! processes removes it and names the count; with no residual it removes it
//! quietly. The `rm -f` of a running container is driven by experiment 429.

use super::*;

/// A stopped record with `end` written into the table file, as the launcher
/// writes it.
fn stopped_with_residual(d: &std::path::Path, residual: serde_json::Value) -> String {
    let s = podvm_image::Store::open(d).unwrap();
    let c = podvm_supervise::create(
        &s,
        Some("r1"),
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
    let path = d.join(podvm_supervise::table::TABLE_FILE);
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let rec = &mut doc["containers"][0];
    rec["state"] = "exited".into();
    rec["exit_code"] = 0.into();
    rec["end"] = serde_json::json!({
        "reason": "exit", "run_ms": 5, "user_ms": 1, "system_ms": 1,
        "max_rss_kib": 900, "residual_processes": residual,
    });
    std::fs::write(&path, doc.to_string()).unwrap();
    c.id
}

#[test]
fn rm_of_a_stopped_record_with_a_residual_removes_it() {
    let d = std::env::temp_dir().join(format!("podvm-residual-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let id = stopped_with_residual(&d, 2.into());
    let s = podvm_image::Store::open(&d).unwrap();
    let c = podvm_supervise::get(&s, &id).unwrap();
    assert_eq!(
        c.end.unwrap().residual_processes,
        podvm_supervise::Residual::Count(2)
    );
    let fields = container_fields(&s, &c);
    let pick = |k: &str| fields.iter().find(|(n, _)| *n == k).unwrap().1.clone();
    assert_eq!(pick("End.Residual"), "2");
    assert_eq!(pick("End.Reason"), "exit");
    assert_eq!(pick("End.MaxRssKiB"), "900");
    assert_eq!(
        super::super::wait_rm_cp::remove_tree(&s, &id, false).unwrap(),
        None
    );
    assert!(podvm_supervise::get(&s, &id).is_err(), "the record stayed");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_unknown_residual_reads_unknown_and_a_record_with_no_end_reads_dashes() {
    let d = std::env::temp_dir().join(format!("podvm-residual-u-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let id = stopped_with_residual(&d, "unknown".into());
    let s = podvm_image::Store::open(&d).unwrap();
    let c = podvm_supervise::get(&s, &id).unwrap();
    let fields = container_fields(&s, &c);
    let pick = |k: &str| fields.iter().find(|(n, _)| *n == k).unwrap().1.clone();
    assert_eq!(pick("End.Residual"), "unknown");
    let mut fresh = c.clone();
    fresh.end = None;
    let fields = container_fields(&s, &fresh);
    let pick = |k: &str| fields.iter().find(|(n, _)| *n == k).unwrap().1.clone();
    assert_eq!(pick("End.Residual"), "-");
    assert_eq!(pick("End.RunMs"), "-");
    let _ = std::fs::remove_dir_all(&d);
}
