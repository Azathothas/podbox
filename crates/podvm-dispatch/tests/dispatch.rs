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
    let out = Command::new(env!("CARGO_BIN_EXE_podvm-dispatch-run"))
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
