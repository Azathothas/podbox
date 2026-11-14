use super::*;

fn with_argv(argv: &[&str]) -> Container {
    Container {
        id: "a".repeat(64),
        name: "n".into(),
        argv: argv.iter().map(|s| s.to_string()).collect(),
        env: Vec::new(),
        working_dir: "/".into(),
        created_at: "now".into(),
        started_at: None,
        finished_at: None,
        state: State::Created,
        pid: None,
        launcher_pid: None,
        exit_code: None,
        noticed: None,
        wall_ms: None,
        end: None,
        of: podvm_supervise::table::Detail {
            image: "img".into(),
            manifest_digest: "sha256:0".into(),
            rootfs: "/tmp".into(),
            rung: "chroot".into(),
            userland_exec: Vec::new(),
            completion: Vec::new(),
            completion_degraded: 0,
            hardening: Default::default(),
        },
    }
}

/// TODO/supervise.md T-1800: the COMMAND column of `ps` cuts a long
/// command between two characters. The two commands of the QA report put
/// a multibyte character across byte 17, and a character of two, three or
/// four bytes at each byte from 15 to 20 must not panic either.
#[test]
fn command_of_cuts_between_characters() {
    let cases = [
        vec!["echo", "\u{65e5}\u{65e5}", "\u{65e5}\u{65e5}\u{65e5}"],
        vec!["echo", "AAAAAAAAAAA\u{e9}BCDEF"],
    ];
    for argv in &cases {
        let out = command_of(&with_argv(argv));
        assert!(out.chars().count() <= 20, "{out}");
    }
    for ch in ['\u{e9}', '\u{65e5}', '\u{1f600}'] {
        for at in 15..=20 {
            let arg = format!("{}{ch}{}", "A".repeat(at), "B".repeat(10));
            let out = command_of(&with_argv(&[&arg]));
            assert_eq!(out.chars().count(), 20, "{arg} gave {out}");
            assert!(out.ends_with("..."), "{out}");
        }
    }
    assert_eq!(
        command_of(&with_argv(&["echo", "\u{65e5}"])),
        "echo \u{65e5}"
    );
}
