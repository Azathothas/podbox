use super::*;

fn job<'a>(
    rootfs: &'a Path,
    volumes: &'a [podvm_enter::userland::Volume],
    ro: &'a [String],
) -> Job<'a> {
    Job {
        verb: "run",
        store_root: Path::new("/store"),
        rootfs,
        argv: vec!["uname".into(), "-r".into()],
        env: vec!["PATH=/bin".into()],
        workdir: "/w".into(),
        stdin: None,
        volumes,
        ro,
        read_only: true,
        user: (1000, 100),
    }
}

/// The bundle names the container's own tree, the command, the user and
/// each volume as a bind mount, `:ro` as a read-only one.
#[test]
fn the_spec_carries_the_tree_the_command_and_each_volume() {
    let vols = vec![
        podvm_enter::userland::Volume {
            host: "/h/ws".into(),
            guest: "/workspace".into(),
        },
        podvm_enter::userland::Volume {
            host: "/h/cfg".into(),
            guest: "/cfg".into(),
        },
    ];
    let ro = vec!["/cfg".to_string()];
    let s = spec(&job(Path::new("/store/trees/x/rootfs"), &vols, &ro));
    assert_eq!(s["root"]["path"], "/store/trees/x/rootfs");
    assert_eq!(s["root"]["readonly"], true);
    assert_eq!(s["process"]["args"], serde_json::json!(["uname", "-r"]));
    assert_eq!(s["process"]["cwd"], "/w");
    assert_eq!(s["process"]["terminal"], false);
    assert_eq!(s["process"]["user"]["uid"], 1000);
    assert_eq!(s["process"]["user"]["gid"], 100);
    let m = s["mounts"].as_array().unwrap();
    let ws = m.iter().find(|m| m["destination"] == "/workspace").unwrap();
    assert_eq!(ws["type"], "bind");
    assert_eq!(ws["source"], "/h/ws");
    assert_eq!(ws["options"], serde_json::json!(["rbind", "rw"]));
    let cfg = m.iter().find(|m| m["destination"] == "/cfg").unwrap();
    assert_eq!(cfg["options"], serde_json::json!(["rbind", "ro"]));
}

/// rootless, the host's network, systrap, no overlay, the state and the
/// bundle of the run.
#[test]
fn the_arguments_are_rootless_systrap_with_the_state_of_the_run() {
    let a = args(Path::new("/s"), Path::new("/b"), "podvm-1");
    assert_eq!(
        a,
        [
            "--rootless",
            "--network=host",
            "--ignore-cgroups",
            "--platform=systrap",
            "--overlay2=none",
            "--root=/s",
            "run",
            "--bundle=/b",
            "podvm-1"
        ]
    );
}

#[test]
fn the_user_is_numeric_or_refused_by_name() {
    assert_eq!(numeric_user(None), Ok((0, 0)));
    assert_eq!(numeric_user(Some("1000")), Ok((1000, 1000)));
    assert_eq!(numeric_user(Some("1000:100")), Ok((1000, 100)));
    let e = numeric_user(Some("nobody")).unwrap_err();
    assert!(e.contains("numeric"), "{e}");
}

/// A `PODVM_RUNSC` that names no file is refused by name.
#[test]
fn a_runsc_variable_that_names_no_file_is_refused() {
    // SAFETY: the tests of this crate read the variable only here.
    unsafe { std::env::set_var(VAR, "/nonexistent/podvm-runsc") };
    let e = binary().unwrap_err();
    unsafe { std::env::remove_var(VAR) };
    assert!(
        e.contains("PODVM_RUNSC=/nonexistent/podvm-runsc names no file"),
        "{e}"
    );
}
