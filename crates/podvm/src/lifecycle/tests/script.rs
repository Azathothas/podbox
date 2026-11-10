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
/// the line as ONE argument, the outer blanks off.
#[test]
fn a_shebang_line_parses_as_the_kernel_parses_it() {
    use crate::lifecycle::script::shebang;
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
    let f =
        follow(&root, &words(&["tool.sh", "a", "b"]), &path).unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(f.argv, words(&["/bin/sh", "-e", "/bin/tool.sh", "a", "b"]));
    assert_eq!(f.resolved, format!("{root}/bin/sh"));
    assert_eq!(f.scripts.len(), 1);
    assert_eq!(f.scripts[0].1, "#!/bin/sh -e");
    // Two levels, as the kernel nests them.
    let f =
        follow(&root, &words(&["/bin/outer.sh", "x"]), &path).unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        f.argv,
        words(&["/bin/sh", "-e", "/bin/tool.sh", "/bin/outer.sh", "x"])
    );
    assert_eq!(f.scripts.len(), 2);
    // Not a script: the argv of the caller, unchanged.
    let f = follow(&root, &words(&["sh", "-c", "x"]), &path).unwrap_or_else(|e| panic!("{}", e.1));
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
        let Err((code, note, routed)) = follow(&root, &words(&[name]), &path) else {
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
    let Err((code, note, routed)) = follow(&root, &argv, &path) else {
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
    let Err((_, note, routed)) = follow(&root, &words(&["nothing"]), &path) else {
        panic!("nothing resolves");
    };
    assert!(!routed);
    assert!(!no_chroot_message("run", &note, routed, Some("node")).contains("--entrypoint"));
    let _ = std::fs::remove_dir_all(&d);
}
