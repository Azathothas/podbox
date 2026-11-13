//! TODO/cli.md T-1769: an unknown verb or flag gets a short refusal.

/// An unknown flag prints the synopsis and where the
/// rest is, not the whole usage.
#[test]
fn an_unknown_flag_prints_a_short_refusal() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_podvm"))
        .args(["run", "--no-such-flag", "img"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(125));
    assert!(err.lines().count() < 5, "{err}");
    assert!(err.contains("--help"), "{err}");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_podvm"))
        .arg("frobnicate")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).lines().count() < 3);
}
