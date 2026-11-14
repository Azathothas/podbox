//! PodUtils/podvm#334 (TODO/cli.md T-1783): a usage error goes to stderr and
//! leaves stdout empty, so `podvm save > image.tar` never writes the usage
//! into a file; `--help` writes the usage to stdout and exits 0.

use std::process::{Command, Stdio};

const VERBS: [&str; 9] = [
    "save", "import", "start", "logs", "stop", "kill", "restart", "wait", "rm",
];

fn podvm(args: &[&str], store: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(args)
        .env("PODVM_STORE", store)
        .stdin(Stdio::null())
        .output()
        .expect("podvm runs")
}

#[test]
fn a_usage_error_leaves_stdout_empty_and_help_writes_it() {
    let store = std::env::temp_dir().join(format!("podvm-usage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    for verb in VERBS {
        let o = podvm(&[verb], &store);
        assert_ne!(o.status.code(), Some(0), "{verb} with no argument exits 0");
        assert!(
            o.stdout.is_empty(),
            "{verb}: the usage error wrote {} bytes to stdout",
            o.stdout.len()
        );
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("usage:"),
            "{verb}: no usage on stderr"
        );
        let h = podvm(&[verb, "--help"], &store);
        assert_eq!(h.status.code(), Some(0), "{verb} --help");
        assert!(
            String::from_utf8_lossy(&h.stdout).contains("usage:"),
            "{verb} --help: no usage on stdout"
        );
    }
    let _ = std::fs::remove_dir_all(&store);
}
