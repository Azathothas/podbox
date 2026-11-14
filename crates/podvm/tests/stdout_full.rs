//! TODO/cli.md T-1816: with stdout on `/dev/full`, a verb exits 125 and
//! names the error on stderr: no SIGABRT (134), and no exit 0 with its
//! answer lost.

use std::process::{Command, Stdio};

fn full(args: &[&str], store: &std::path::Path) -> std::process::Output {
    let dev_full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full opens");
    Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::null())
        .stdout(Stdio::from(dev_full))
        .stderr(Stdio::piped())
        .output()
        .expect("podvm runs")
}

#[test]
fn stdout_full_is_125_with_the_errno_named() {
    let store = std::env::temp_dir().join(format!("podvm-full-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    for args in [
        &["version"][..],
        &["ps", "-a"],
        // An empty store: the table has its header, a template no line.
        &["images"],
        &["probe"],
    ] {
        let o = full(args, &store);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(125), "{args:?}: {err}");
        assert_eq!(
            err.matches("stdout: No space left on device").count(),
            1,
            "{args:?}: {err}"
        );
    }
    let _ = std::fs::remove_dir_all(&store);
}
