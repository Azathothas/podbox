//! TODO/enter.md T-1652: the native-static rung on a root of its own. The
//! payload is `podvm-dispatch-probe`, a static program; the host is
//! `podvm-dispatch-run`. The host has no `/etc/podvm-marker`, so a payload
//! that reads it read the root's.

#![cfg(target_arch = "x86_64")]

use std::path::PathBuf;
use std::process::Command;

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(name: &str) -> Option<Root> {
    if let Err(e) = podvm_dispatch::available() {
        eprintln!("SKIP: {e}");
        return None;
    }
    let d = std::env::temp_dir().join(format!("podvm-t1652-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["etc", "bin", "tmp", "proc", "dev"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    std::fs::write(d.join("etc/podvm-marker"), "inroot\n").unwrap();
    std::fs::copy(
        env!("CARGO_BIN_EXE_podvm-dispatch-probe"),
        d.join("bin/probe"),
    )
    .unwrap();
    std::os::unix::fs::symlink("/etc/podvm-marker", d.join("tmp/abs")).unwrap();
    std::fs::write(d.join("bin/script"), "#!/bin/probe cat\n").unwrap();
    std::os::unix::fs::PermissionsExt::set_mode(
        &mut std::fs::metadata(d.join("bin/script"))
            .unwrap()
            .permissions(),
        0o755,
    );
    Command::new("chmod")
        .arg("755")
        .arg(d.join("bin/script"))
        .status()
        .unwrap();
    Some(Root(d))
}

/// Run `/bin/probe VERB ARGS` in the root: its stdout and exit code.
fn probe(r: &Root, args: &[&str]) -> (String, i32) {
    probe_with(r, args, None)
}

/// As [`probe`], on the syscall tier where `tier` is `Some(CAP)` (T-1657).
fn probe_with(r: &Root, args: &[&str], tier: Option<&str>) -> (String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_podvm-dispatch-run"));
    match tier {
        Some(t) => cmd.env(podvm_dispatch::TIER_VAR, t),
        None => cmd.env_remove(podvm_dispatch::TIER_VAR),
    };
    let out = cmd
        .arg(&r.0)
        .arg("/bin/probe")
        .arg("probe")
        .args(args)
        .current_dir(&r.0)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
    eprintln!(
        "{args:?} -> {text:?} {:?} {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.code().unwrap_or(-1))
}

#[test]
fn a_path_resolves_in_the_root_and_dotdot_stays_there() {
    let Some(r) = root("open") else { return };
    assert!(
        !std::path::Path::new("/etc/podvm-marker").exists(),
        "the host has no marker"
    );
    assert_eq!(
        probe(&r, &["cat", "/etc/podvm-marker"]),
        ("inroot".into(), 0)
    );
    assert_eq!(
        probe(&r, &["cat", "/../../../../etc/podvm-marker"]).0,
        "inroot"
    );
    assert_eq!(
        probe(&r, &["cat", "/tmp/abs"]).0,
        "inroot",
        "an absolute link resolves in the root"
    );
    assert_eq!(probe(&r, &["ls", "/"]).0, "bin dev etc proc tmp");
    assert_eq!(probe(&r, &["cd", "/etc", "podvm-marker"]).0, "/etc inroot");
    assert_eq!(probe(&r, &["cwd"]).0, "/");
    assert_eq!(
        probe(&r, &["write", "/tmp/a", "hello", "/tmp/b"]).0,
        "hello"
    );
    assert!(r.0.join("tmp/b").exists(), "the write landed in the root");
}

#[test]
fn each_call_traps_proc_self_exe_names_the_guest_program() {
    let Some(r) = root("trap") else { return };
    assert_eq!(probe(&r, &["exe"]).0, "/bin/probe");
    assert_eq!(
        probe(&r, &["signal"]).0,
        "usr1 true",
        "a handler of the payload runs"
    );
    assert_eq!(
        probe(&r, &["alloc"]).0,
        "alloc 469762048",
        "the break grows"
    );
}

#[test]
fn dispatch_is_armed_again_in_each_child_and_thread() {
    let Some(r) = root("child") else { return };
    assert_eq!(
        probe(&r, &["fork-cat", "/etc/podvm-marker"]).0,
        "child inroot\nparent status 7"
    );
    assert_eq!(
        probe(&r, &["thread-cat", "/etc/podvm-marker"]).0,
        "thread inroot"
    );
    assert_eq!(
        probe(&r, &["spawn", "/bin/probe", "cat", "/etc/podvm-marker"]).0,
        "spawn 0 inroot"
    );
    assert_eq!(
        probe(&r, &["spawn", "/bin/probe", "exe"]).0,
        "spawn 0 /bin/probe"
    );
}

#[test]
fn exec_loads_the_next_program_in_the_root() {
    let Some(r) = root("exec") else { return };
    assert_eq!(
        probe(&r, &["exec", "/bin/probe", "cat", "/etc/podvm-marker"]),
        ("inroot".into(), 0)
    );
    assert_eq!(
        probe(&r, &["spawn", "/bin/script"]).0,
        "spawn 0 #!/bin/probe cat"
    );
    assert_eq!(
        probe(&r, &["spawn", "/bin/nothere"]).0,
        "ERR No such file or directory (os error 2)"
    );
    assert_eq!(
        probe(&r, &["spawn", "/etc/podvm-marker"]).0,
        "ERR Permission denied (os error 13)"
    );
}

/// TODO/enter.md T-1651: the emulator serves each ELF (machine 0) where
/// `qemu-x86_64-static` is on the host; the program, an exec and a spawn
/// run under it, in the root.
#[test]
fn the_emulator_runs_the_program_and_its_children_in_the_root() {
    let Some(r) = root("emu") else { return };
    let Some(qemu) = [
        "/usr/bin/qemu-x86_64-static",
        "/usr/local/bin/qemu-x86_64-static",
    ]
    .into_iter()
    .find(|p| std::path::Path::new(p).exists()) else {
        eprintln!("SKIP: no qemu-x86_64-static on this host");
        return;
    };
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_podvm-dispatch-run"))
            .arg(&r.0)
            .arg("/bin/probe")
            .arg("probe")
            .args(args)
            .env("PODVM_DISPATCH_EMU", format!("0:{qemu}"))
            .current_dir(&r.0)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim_end().to_string()
    };
    assert_eq!(run(&["cat", "/etc/podvm-marker"]), "inroot");
    assert_eq!(
        run(&["exec", "/bin/probe", "cat", "/etc/podvm-marker"]),
        "inroot"
    );
    assert_eq!(
        run(&["spawn", "/bin/probe", "cat", "/etc/podvm-marker"]),
        "spawn 0 inroot"
    );
}

/// T-1657: on the syscall tier, `uname` names the tier; off it, the host's
/// release.
#[test]
fn the_syscall_tier_answers_uname_with_its_release() {
    let Some(r) = root("uname") else { return };
    let (on, code) = probe_with(&r, &["uname"], Some("1048576"));
    assert_eq!(code, 0);
    assert_eq!(on, "0 6.1.0-podvm-syscall");
    let (off, _) = probe(&r, &["uname"]);
    assert!(off.starts_with("0 ") && !off.contains("podvm"), "{off}");
}

/// T-1657: the cap of `/tmp` holds across a fork, an unlink gives the
/// space back, and a write over the cap fails with ENOSPC (28).
#[test]
fn the_syscall_tier_caps_tmp_across_a_fork() {
    let Some(r) = root("tmpcap") else { return };
    let (got, code) = probe_with(&r, &["tmpcap", "800", "400"], Some("1000"));
    assert_eq!(code, 0);
    assert_eq!(got, "a=ok b=28 rm=true b2=ok");
    // With no tier, no cap.
    let (got, _) = probe(&r, &["tmpcap", "800", "400"]);
    assert_eq!(got, "a=ok b=0 rm=true b2=ok");
}

/// T-1657: resolution fails closed: a missing file of the root is ENOENT
/// even where the host has one.
#[test]
fn a_missing_file_of_the_root_is_enoent_and_not_the_hosts() {
    let Some(r) = root("missing") else { return };
    assert!(
        std::path::Path::new("/etc/passwd").exists(),
        "the host has one"
    );
    let (got, _) = probe_with(&r, &["cat", "/etc/passwd"], Some("1048576"));
    assert!(got.starts_with("ERR No such file"), "{got}");
    let (got, _) = probe(&r, &["cat", "/bin/../../etc/passwd"]);
    assert!(got.starts_with("ERR No such file"), "{got}");
}
