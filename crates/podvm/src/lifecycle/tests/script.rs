//! TODO/cli.md T-1649. The `#!` route of the no-chroot entry and the
//! remedy that its refusal names.

use super::*;

/// A rootfs under the temporary directory, with `files` written under it
/// and made executable. Removed by the caller.
fn rootfs(tag: &str, files: &[(&str, &[u8])]) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let d = std::env::temp_dir().join(format!("podvm-script-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for (path, bytes) in files {
        let p = d.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, bytes).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    d
}

fn words(w: &[&str]) -> Vec<String> {
    w.iter().map(|s| s.to_string()).collect()
}

/// The `#!` line parses as `fs/binfmt_script.c` parses it, the cases of
/// `head::shebang` in the interposer: one interpreter, the remainder of
/// the line as ONE argument, the outer blanks off. The parser is
/// `pod-enter`'s `memfd::shebang_parts` (T-137 of PodUtils/podcrates);
/// these cases pin it beside the copy of the interposer.
#[test]
fn a_shebang_line_parses_as_the_kernel_parses_it() {
    use podvm_enter::memfd::shebang_parts as shebang;
    let s = |i: &str, a: Option<&str>| Some((i.to_string(), a.map(str::to_string)));
    assert_eq!(shebang(b"#!/bin/sh\necho\n"), s("/bin/sh", None));
    assert_eq!(shebang(b"#! /bin/sh -e \necho\n"), s("/bin/sh", Some("-e")));
    assert_eq!(
        shebang(b"#!/usr/bin/env node  --x y\n"),
        s("/usr/bin/env", Some("node  --x y"))
    );
    assert_eq!(shebang(b"#!\t/bin/sh\t\r\n"), s("/bin/sh", Some("\r")));
    // A whole short file with no newline still names its interpreter.
    assert_eq!(shebang(b"#!/bin/sh"), s("/bin/sh", None));
    // No name, and a name that runs past the head, are the ENOEXEC of the
    // kernel.
    assert_eq!(shebang(b"#!\n"), None);
    assert_eq!(shebang(b"#!   \n"), None);
    let mut long = b"#!/".to_vec();
    long.extend(std::iter::repeat_n(b'a', 400));
    assert_eq!(shebang(&long), None);
    // A long argument is cut at the head, as the kernel cuts it.
    let mut cut = b"#!/bin/sh ".to_vec();
    cut.extend(std::iter::repeat_n(b'b', 400));
    let (i, a) = shebang(&cut).unwrap();
    assert_eq!(i, "/bin/sh");
    assert_eq!(a.unwrap().len(), 256 - 10);
    assert_eq!(shebang(b"\x7fELF"), None);
}

/// The route follows each level inside the rootfs, in the argument order
/// of the kernel, and keeps a payload that is no script unchanged.
#[test]
fn a_script_runs_its_interpreter_from_the_image() {
    use crate::lifecycle::script::follow;
    let d = rootfs(
        "follow",
        &[
            ("bin/sh", b"\x7fELF-not-read-here"),
            ("bin/tool.sh", b"#!/bin/sh -e\necho hi\n"),
            ("bin/outer.sh", b"#!/bin/tool.sh\n"),
        ],
    );
    let root = d.to_string_lossy().to_string();
    let path = vec!["/bin".to_string()];
    let f = follow(&root, &words(&["tool.sh", "a", "b"]), &path, "/")
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["/bin/sh", "-e", "/bin/tool.sh", "a", "b"]));
    assert_eq!(f.resolved, format!("{root}/bin/sh"));
    assert_eq!(f.scripts.len(), 1);
    assert_eq!(f.scripts[0].1, "#!/bin/sh -e");
    // Two levels, as the kernel nests them.
    let f = follow(&root, &words(&["/bin/outer.sh", "x"]), &path, "/")
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        f.argv,
        words(&["/bin/sh", "-e", "/bin/tool.sh", "/bin/outer.sh", "x"])
    );
    assert_eq!(f.scripts.len(), 2);
    // Not a script: the argv of the caller, unchanged.
    let f =
        follow(&root, &words(&["sh", "-c", "x"]), &path, "/").unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["sh", "-c", "x"]));
    assert!(f.scripts.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

/// An interpreter outside the image, a relative one and a chain past the
/// bound of the kernel refuse at 126, naming the script; podvm never
/// looks on the host.
#[test]
fn a_script_whose_interpreter_is_not_in_the_image_refuses() {
    use crate::lifecycle::script::follow;
    let d = rootfs(
        "refuse",
        &[
            ("bin/bash.sh", b"#!/bin/bash\necho hi\n"),
            ("bin/rel.sh", b"#!sh\n"),
            ("bin/loop.sh", b"#!/bin/loop.sh\n"),
        ],
    );
    let root = d.to_string_lossy().to_string();
    let path = vec!["/bin".to_string()];
    for (name, want) in [
        ("bash.sh", "does not resolve inside the image"),
        ("rel.sh", "relative path"),
        ("loop.sh", "more than 4"),
    ] {
        let Err((code, note, routed)) = follow(&root, &words(&[name]), &path, "/") else {
            panic!("{name} was followed");
        };
        assert_eq!(code, EXIT_CANNOT_INVOKE, "{name}: {note}");
        assert!(note.contains(want), "{name}: {note}");
        assert!(note.contains("#!"), "{name}: {note}");
        assert!(routed || name != "bash.sh", "{name}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/cli.md T-1649, the Prove clause: the refusal of a script that is
/// the Entrypoint of the image, where the caller gave a command, names
/// `--entrypoint` followed by the first word of the caller. Without the
/// command, or for a refusal that followed no script, it does not.
#[test]
fn an_entrypoint_script_refusal_names_the_entrypoint_flag() {
    use crate::lifecycle::script::{follow, no_chroot_message};
    let d = rootfs(
        "retry",
        &[(
            "usr/local/bin/docker-entrypoint.sh",
            b"#!/bin/sh\nexec \"$@\"\n",
        )],
    );
    let root = d.to_string_lossy().to_string();
    let path = vec!["/usr/local/bin".to_string()];
    let argv = words(&["docker-entrypoint.sh", "node", "-e", "console.log(1)"]);
    let Err((code, note, routed)) = follow(&root, &argv, &path, "/") else {
        panic!("the image holds no /bin/sh, so the route refuses");
    };
    assert_eq!(code, EXIT_CANNOT_INVOKE);
    assert!(routed);
    let m = no_chroot_message("run", &note, routed, Some("node"));
    assert!(m.contains("--entrypoint node"), "{m}");
    assert!(m.contains("docker-entrypoint.sh"), "{m}");
    assert!(m.contains("#!/bin/sh"), "{m}");
    assert!(!no_chroot_message("run", &note, routed, None).contains("--entrypoint"));
    // A payload that resolves to nothing followed no script: no remedy.
    let Err((_, note, routed)) = follow(&root, &words(&["nothing"]), &path, "/") else {
        panic!("nothing resolves");
    };
    assert!(!routed);
    assert!(!no_chroot_message("run", &note, routed, Some("node")).contains("--entrypoint"));
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1653. A first program in a volume is the file of the
/// volume: the route reads the script from the host directory of the
/// volume, runs its interpreter from the image, and gives the script its
/// guest path. A bare name on a `PATH` directory in the volume gives the
/// unresolved join, as the kernel gives it.
#[test]
fn a_script_in_a_volume_runs_from_the_volume() {
    use crate::lifecycle::script::follow;
    let d = rootfs(
        "volume",
        &[
            ("rootfs/bin/sh", b"\x7fELF-not-read-here"),
            ("host/tool.sh", b"#!/bin/sh -e\necho hi\n"),
        ],
    );
    let root = d.join("rootfs").to_string_lossy().to_string();
    let host = d.join("host").to_string_lossy().to_string();
    let vols = [podvm_enter::userland::Volume {
        guest: "/data".to_string(),
        host: host.clone(),
    }];
    let maps = podvm_enter::userland::Maps::new(&podvm_enter::NAMES, &root, &vols).unwrap();
    let path = vec!["/bin".to_string(), "/data".to_string()];
    // The image has no /data/tool.sh: without the table it is absent.
    let Err((code, _, _)) = follow(&root, &words(&["/data/tool.sh"]), &path, "/") else {
        panic!("the rootfs alone resolved a file of the volume");
    };
    assert_eq!(code, EXIT_NOT_FOUND);
    let f = follow(&maps, &words(&["/data/tool.sh", "a"]), &path, "/")
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["/bin/sh", "-e", "/data/tool.sh", "a"]));
    assert_eq!(f.resolved, format!("{root}/bin/sh"));
    assert_eq!(f.scripts[0].0, format!("{host}/tool.sh"));
    let f = follow(&maps, &words(&["tool.sh"]), &path, "/").unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["/bin/sh", "-e", "/data/tool.sh"]));
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1707: `-w /opt/x ./prog` resolves to `/opt/x/prog`, as
/// docker's does; the argv keeps `./prog`. A relative `#!` script hands
/// its interpreter the name as invoked, as the kernel does. A bare name
/// still searches `PATH`, and `./prog` from `/` misses.
#[test]
fn a_relative_program_resolves_against_the_working_directory() {
    use crate::lifecycle::script::follow;
    assert_eq!(podvm_enter::program_in("./prog", "/opt/x"), "/opt/x/./prog");
    assert_eq!(podvm_enter::program_in("bin/p", "/opt/x/"), "/opt/x/bin/p");
    assert_eq!(podvm_enter::program_in("./p", ""), "/./p");
    assert_eq!(podvm_enter::program_in("/bin/p", "/opt/x"), "/bin/p");
    assert_eq!(podvm_enter::program_in("p", "/opt/x"), "p");
    let d = rootfs(
        "workdir",
        &[
            ("bin/sh", b"\x7fELF-not-read-here"),
            ("opt/x/prog", b"\x7fELF-not-read-here"),
            ("opt/x/run.sh", b"#!/bin/sh\n"),
        ],
    );
    let root = d.to_string_lossy().to_string();
    let path = vec!["/bin".to_string()];
    let f = follow(&root, &words(&["./prog", "a"]), &path, "/opt/x")
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.resolved, format!("{root}/opt/x/prog"));
    assert_eq!(f.argv, words(&["./prog", "a"]));
    let f =
        follow(&root, &words(&["./run.sh"]), &path, "/opt/x").unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["/bin/sh", "./run.sh"]));
    assert_eq!(f.resolved, format!("{root}/bin/sh"));
    let Err((code, _, _)) = follow(&root, &words(&["./prog"]), &path, "/") else {
        panic!("./prog resolved from /");
    };
    assert_eq!(code, 127);
    let _ = std::fs::remove_dir_all(&d);
}
