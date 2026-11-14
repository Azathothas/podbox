//! TODO/supervise.md T-1805: a name that resolves to nothing exits 1 on
//! each lifecycle verb, with one line on stderr, as docker 29.9.2 does
//! (measured 2026-11-14) and as podvm's own table of exit codes says.

use std::process::{Command, Stdio};

fn podvm(args: &[&str], store: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::null())
        .output()
        .expect("podvm runs")
}

#[test]
fn lifecycle_exit_of_a_missing_name_is_1_with_one_line() {
    let store = std::env::temp_dir().join(format!("podvm-lcexit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    for args in [
        &["start", "nosuch"][..],
        &["stop", "nosuch"],
        &["restart", "nosuch"],
        &["rm", "nosuch"],
        &["logs", "nosuch"],
        &["wait", "nosuch"],
        &["kill", "nosuch"],
        &["cp", "nosuch:/x", "/tmp/podvm-lcexit-y"],
    ] {
        let o = podvm(args, &store);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{args:?}: {err}");
        assert_eq!(err.lines().count(), 1, "{args:?}: {err}");
        assert!(err.contains("nosuch"), "{args:?}: {err}");
    }
    let _ = std::fs::remove_dir_all(&store);
}
