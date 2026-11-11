use super::builtin;
use super::model::*;
use std::path::Path;

fn table(text: &str) -> toml::Table {
    text.parse().unwrap()
}

/// A directory of user profiles for one test.
fn dir(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-profiles-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (n, t) in files {
        std::fs::write(d.join(format!("{n}.toml")), t).unwrap();
    }
    d
}

#[test]
fn an_unknown_key_refuses_and_names_the_key() {
    let e = check(&table("[fs]\nbackend = \"auto\"\nusr = \"1000\"\n")).unwrap_err();
    assert!(e.contains("unknown key `fs.usr`"), "{e}");
    assert!(e.contains("backend, user"), "{e}");
    let e = check(&table("color = \"red\"\n")).unwrap_err();
    assert!(e.contains("unknown key `color`"), "{e}");
    let e = check(&table("[nett]\nmode = \"host\"\n")).unwrap_err();
    assert!(e.contains("unknown key `nett`"), "{e}");
    let e = check(&table("[env.set]\nA = \"1\"\n[env.get]\nB = \"2\"\n")).unwrap_err();
    assert!(e.contains("unknown key `env.get`"), "{e}");
}

#[test]
fn a_wrong_type_refuses_and_names_the_key() {
    for (text, key) in [
        (
            "[fs]\nread_only = \"yes\"\n",
            "`fs.read_only` must be a boolean",
        ),
        (
            "[limits]\ncpus = \"two\"\n",
            "`limits.cpus` must be an integer",
        ),
        ("[limits]\ncpus = 0\n", "`limits.cpus` must be from 1 to 64"),
        (
            "[fs]\nvolumes = \"/a:/b\"\n",
            "`fs.volumes` must be an array of strings",
        ),
        (
            "[fs]\nwrap = [\"env\", 1]\n",
            "`fs.wrap` must be an array of strings",
        ),
        (
            "[limits]\nmemory = \"lots\"\n",
            "`limits.memory` takes bytes",
        ),
        ("[env.set]\nN = 3\n", "`env.set.N` must be a string"),
        ("fs = 3\n", "`fs` must be a table"),
        (
            "[fs]\nbackend = \"bailey\"\n",
            "`fs.backend` takes auto, machine, incus, emulate",
        ),
    ] {
        let e = check(&table(text)).unwrap_err();
        assert!(e.contains(key), "{text}: {e}");
    }
}

/// The plant of the Prove: a user profile with one misspelt key, through
/// the whole path that `run --profile` takes.
#[test]
fn a_planted_misspelt_key_is_refused() {
    let d = dir(
        "plant",
        &[("cage", "extends = \"workspace\"\n[fs]\nread_onyl = true\n")],
    );
    let (t, _) = resolve("cage", Some(&d)).unwrap();
    let e = check(&t).unwrap_err();
    assert!(e.contains("unknown key `fs.read_onyl`"), "{e}");
    assert!(e.contains("looser than the recipe says"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn extends_and_s_give_the_effective_profile() {
    let d = dir(
        "extends",
        &[
            (
                "base",
                "[fs]\nuser = \"1000\"\nworkdir = \"/w\"\n[env.set]\nA = \"base\"\nB = \"base\"\n",
            ),
            (
                "child",
                "extends = \"base\"\n[fs]\nworkdir = \"/c\"\n[env.set]\nB = \"child\"\n",
            ),
        ],
    );
    let (mut t, source) = resolve("child", Some(&d)).unwrap();
    assert_eq!(source, Source::File(d.join("child.toml")));
    assert!(t.get("extends").is_none());
    set(&mut t, "limits.file_max=1048576").unwrap();
    set(&mut t, "env.set.C=plain words").unwrap();
    set(&mut t, "fs.volumes=[\"/a:/b:ro\"]").unwrap();
    let c = check(&t).unwrap();
    assert_eq!(c.user.as_deref(), Some("1000"), "the parent's key stays");
    assert_eq!(c.workdir.as_deref(), Some("/c"), "the child's key wins");
    assert_eq!(c.file_max, Some(1_048_576), "-s parses TOML");
    assert_eq!(c.volumes, vec!["/a:/b:ro".to_string()]);
    let env: Vec<_> = c.env.iter().map(|(k, v)| format!("{k}={v}")).collect();
    assert_eq!(
        env,
        ["A=base", "B=child", "C=plain words"],
        "a table merges key by key; -s falls back to a string"
    );
    let args = run_args(&c, Path::new("/")).unwrap();
    assert_eq!(
        args,
        [
            "--user=1000",
            "--workdir=/c",
            "-v",
            "/a:/b:ro",
            "--ulimit=fsize=1048576",
            "-e",
            "A=base",
            "-e",
            "B=child",
            "-e",
            "C=plain words"
        ]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_cycle_of_extends_refuses() {
    let d = dir(
        "cycle",
        &[("a", "extends = \"b\"\n"), ("b", "extends = \"a\"\n")],
    );
    let e = resolve("a", Some(&d)).unwrap_err();
    assert!(e.contains("cycle: a -> b -> a"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn each_built_in_profile_checks_and_gives_its_flags() {
    let cwd = std::env::temp_dir();
    for n in builtin::NAMES {
        let (t, source) = resolve(n, None).unwrap();
        assert_eq!(source, Source::BuiltIn);
        let c = check(&t).unwrap_or_else(|e| panic!("{n}: {e}"));
        assert!(!c.description.is_empty(), "{n} has no description");
        // T-1659: each built-in profile lists what it leaves out.
        assert!(
            !c.leaves_out.is_empty(),
            "{n} lists nothing that it leaves out"
        );
        run_args(&c, &cwd).unwrap_or_else(|e| panic!("{n}: {e}"));
    }
    let (t, _) = resolve("machine", None).unwrap();
    let a = run_args(&check(&t).unwrap(), &cwd).unwrap();
    assert!(a.contains(&"--podvm-tier=machine".to_string()), "{a:?}");
    assert!(a.contains(&format!("--podvm-mem={}", 1u64 << 30)), "{a:?}");
    assert!(a.contains(&"--podvm-cpus=2".to_string()), "{a:?}");
}

#[test]
fn memory_and_cpus_refuse_by_name_on_a_rootfs_tier() {
    let (mut t, _) = resolve("workspace", None).unwrap();
    set(&mut t, "limits.memory=\"512m\"").unwrap();
    let e = run_args(&check(&t).unwrap(), &std::env::temp_dir()).unwrap_err();
    assert!(
        e.contains("`limits.memory` needs fs.backend = \"machine\""),
        "{e}"
    );
    let (mut t, _) = resolve("default", None).unwrap();
    set(&mut t, "limits.cpus=2").unwrap();
    let e = run_args(&check(&t).unwrap(), &std::env::temp_dir()).unwrap_err();
    assert!(e.contains("`limits.cpus` needs"), "{e}");
    let (mut t, _) = resolve("default", None).unwrap();
    set(&mut t, "net.mode=none").unwrap();
    let e = run_args(&check(&t).unwrap(), &std::env::temp_dir()).unwrap_err();
    assert!(
        e.contains("`net.mode` \"none\" has no mechanism yet"),
        "{e}"
    );
}

#[test]
fn a_name_outside_the_alphabet_or_absent_refuses() {
    assert!(raw("../etc/passwd", None)
        .unwrap_err()
        .contains("a name takes"));
    assert!(raw("nosuch", None).unwrap_err().contains("no such profile"));
    assert!(set(&mut toml::Table::new(), "novalue")
        .unwrap_err()
        .contains("KEY=VALUE"));
}

/// T-1659: a provider shape names its image, user and variables.
#[test]
fn a_provider_shape_names_its_image_and_its_identity() {
    let (t, _) = resolve("lambda-al2023", None).unwrap();
    let c = check(&t).unwrap();
    assert_eq!(
        c.image.as_deref(),
        Some("public.ecr.aws/lambda/provided:al2023")
    );
    assert_eq!(c.user.as_deref(), Some("993:990"));
    assert!(c.read_only);
    assert!(c.env.iter().any(|(k, _)| k == "AWS_LAMBDA_FUNCTION_NAME"));
    assert!(c.leaves_out.iter().any(|l| l.contains("memory 128m")));
    let a = run_args(&c, &std::env::temp_dir()).unwrap();
    assert!(a.contains(&"--user=993:990".to_string()), "{a:?}");
    assert!(a.contains(&"--tmpfs=/tmp:size=512m".to_string()), "{a:?}");
    // The image's entrypoint is Lambda's invocation protocol, which the
    // profile leaves out: the command runs as the program.
    assert_eq!(c.entrypoint.as_deref(), Some(""));
    assert!(a.contains(&"--entrypoint=".to_string()), "{a:?}");
    let (mut t, _) = resolve("busybox-bare", None).unwrap();
    set(&mut t, "needs=[\"curl\"]").unwrap();
    assert_eq!(check(&t).unwrap().needs, vec!["curl".to_string()]);
}

/// TODO/cli.md T-1661: one profile's paths give both the runtime's flags
/// and podvm's path map.
#[test]
fn one_profile_gives_the_runtime_flags_and_the_path_map() {
    let (_, c, _) = super::effective("node-permission", &[]).unwrap();
    let cwd = std::env::current_dir().unwrap();
    let flags = run_args(&c, &cwd).unwrap();
    assert!(
        flags.iter().any(|f| f.ends_with(":/workspace")),
        "{flags:?}"
    );
    assert!(flags.contains(&"--entrypoint=".to_string()), "{flags:?}");
    let (p, d) = super::runtime::prefix(c.runtime.as_deref().unwrap(), &c.read, &c.write).unwrap();
    assert_eq!(p[..2], ["node".to_string(), "--permission".to_string()]);
    assert!(p.contains(&"--allow-fs-read=/workspace".to_string()));
    assert!(p.contains(&"--allow-fs-write=/workspace".to_string()));
    assert_eq!(d, ["node".to_string()]);
    // A path without a runtime, and a runtime beside fs.wrap, refuse.
    let e = super::effective("workspace", &["fs.read=[\"/x\"]".into()]).unwrap_err();
    assert!(e.contains("runtime"), "{e}");
    let e = super::effective("node-permission", &["fs.wrap=[\"env\"]".into()]).unwrap_err();
    assert!(e.contains("fs.wrap"), "{e}");
}
