//! TODO/cli.md T-1745: the flags of `podvm-api` against the parity table.

use super::*;

/// TODO/cli.md T-1745 (V-1 of PodUtils/podcrates#124): each flag that
/// `podvm-api` writes for `run` has a row that podvm serves. A rename of a
/// flag fails here, in podvm's CI, and not in a program that links the API.
#[test]
fn each_flag_of_podvm_api_has_a_served_run_row() {
    use podvm_api::{Pull, Run, Tier, Ulimit, Volume};
    let req = Run {
        image: "lab/one:1".into(),
        command: vec!["/hello".into()],
        env: vec![("A".into(), "1".into())],
        volumes: vec![Volume {
            host: "/srv".into(),
            guest: "/srv".into(),
            read_only: true,
        }],
        workdir: Some("/srv".into()),
        labels: vec![("job".into(), "7".into())],
        ulimits: vec![Ulimit {
            name: "nofile".into(),
            soft: 64,
            hard: None,
        }],
        timeout_s: Some(5),
        tier: Some(Tier::Machine),
        confine: Some("read=/srv".into()),
        pull: Pull::Never,
    };
    let argv = req
        .argv(
            std::path::Path::new("/r.json"),
            Some(std::path::Path::new("/env")),
        )
        .unwrap();
    assert_eq!(argv[0], "run");
    let flags: Vec<&String> = argv.iter().filter(|a| a.starts_with('-')).collect();
    assert!(flags.len() >= 11, "{flags:?}");
    for f in flags {
        let row = flag("run", f).unwrap_or_else(|| panic!("{f} has no row of run"));
        assert!(
            matches!(row.status, Status::Native | Status::Degraded),
            "{f} is not served: {:?}",
            row.status
        );
    }
    for t in [
        Tier::Machine,
        Tier::Runsc,
        Tier::Syscall,
        Tier::Incus,
        Tier::Emulate,
    ] {
        let note = flag("run", "--podvm-tier").unwrap().note;
        assert!(
            note.contains(t.word()),
            "--podvm-tier does not name {}",
            t.word()
        );
    }
}
