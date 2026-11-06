use super::*;

fn p(xs: &[&str]) -> Result<Option<Args>, i32> {
    parse(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn help_prints_and_exits_zero() {
    assert!(matches!(p(&["-h"]), Ok(None)));
    assert!(matches!(p(&["--help"]), Ok(None)));
    assert!(
        matches!(p(&["--kernel", "k", "-h"]), Ok(None)),
        "help wins anywhere, like the windows parser"
    );
}

#[test]
fn missing_required_inputs_are_flag_errors() {
    use podvm_image::error::EXIT_FLAG_ERROR;
    assert_eq!(p(&[]).unwrap_err(), EXIT_FLAG_ERROR);
    assert_eq!(p(&["--kernel", "k"]).unwrap_err(), EXIT_FLAG_ERROR);
    assert_eq!(
        p(&["--kernel", "k", "--initramfs", "i"]).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(p(&["--kernel"]).unwrap_err(), EXIT_FLAG_ERROR);
    assert_eq!(p(&["--user-key"]).unwrap_err(), EXIT_FLAG_ERROR);
}

#[test]
fn bad_sizes_and_timeouts_are_flag_errors() {
    use podvm_image::error::EXIT_FLAG_ERROR;
    assert_eq!(
        p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--podvm-mem",
            "huge",
        ])
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--podvm-mem",
            "0",
        ])
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--podvm-timeout",
            "0",
        ])
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--podvm-timeout",
            "soon",
        ])
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        p(&[
            "--kernel",
            "k",
            "--initramfs",
            "i",
            "--user-key",
            "uk",
            "--bogus",
        ])
        .unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

#[test]
fn minimal_flags_carry_the_documented_defaults() {
    let a = p(&["--kernel", "k", "--initramfs", "i", "--user-key", "uk"])
        .unwrap()
        .unwrap();
    assert_eq!(a.user, "root");
    assert_eq!(a.mem, 256 << 20);
    assert_eq!(a.timeout, Duration::from_secs(600));
    assert!(a.command.is_empty());
    assert!(a.emu_args.is_empty());
}

#[test]
fn the_parsed_arm_carries_everything() {
    let a = p(&[
        "--kernel",
        "k",
        "--initramfs",
        "i",
        "--user-key",
        "uk",
        "--user",
        "guest",
        "--podvm-mem",
        "1G",
        "--podvm-timeout",
        "90",
        "--podvm-qemu-arg",
        "-smp",
        "--podvm-qemu-arg",
        "2",
        "--",
        "echo",
        "hi there",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(a.kernel, PathBuf::from("k"));
    assert_eq!(a.initramfs, PathBuf::from("i"));
    assert_eq!(a.user_key, PathBuf::from("uk"));
    assert_eq!(a.user, "guest");
    assert_eq!(a.mem, 1 << 30);
    assert_eq!(a.timeout, Duration::from_secs(90));
    assert_eq!(a.emu_args, vec!["-smp".to_string(), "2".to_string()]);
    assert_eq!(a.command, vec!["echo".to_string(), "hi there".to_string()]);
}

#[test]
fn bare_words_without_a_separator_are_the_command() {
    let a = p(&[
        "--kernel",
        "k",
        "--initramfs",
        "i",
        "--user-key",
        "uk",
        "echo",
        "hi",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(a.command, vec!["echo".to_string(), "hi".to_string()]);
}

#[test]
fn bytes_round_up_to_whole_mib_without_overflow() {
    assert_eq!(mem_mib(0), 0);
    assert_eq!(mem_mib(1), 1);
    assert_eq!(mem_mib(1 << 20), 1);
    assert_eq!(mem_mib((1 << 20) + 1), 2);
    assert_eq!(mem_mib(256 << 20), 256);
    assert_eq!(mem_mib(u64::MAX), (u64::MAX >> 20) + 1);
}

#[test]
fn the_dir_guard_removes_the_directory_on_drop() {
    let dir = std::env::temp_dir().join(format!("podvm-machine-ssh-guard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    {
        let _guard = DirGuard::arm(dir.clone());
    }
    assert!(!dir.exists());
}

#[test]
fn a_named_input_that_is_not_a_file_is_refused_never_replaced() {
    let dir = std::env::temp_dir();
    let missing = dir.join("podvm-machine-ssh-no-such-file-xyz");
    assert_eq!(resolve_input(&missing, "--kernel"), Err(EXIT_RUNTIME_ERROR));
}

#[test]
fn the_accelerator_follows_the_node_openability() {
    assert!(!kvm_present("/podvm-machine-ssh-no-such-node-xyz"));
    // Any read-write openable path answers true: the check probes
    // openability, not identity, which is what the argv needs.
    let file = std::env::temp_dir().join("podvm-machine-ssh-kvm-probe-xyz");
    std::fs::write(&file, b"no").unwrap();
    assert!(kvm_present(file.to_str().unwrap()));
    let _ = std::fs::remove_file(&file);
}

#[test]
fn the_qemu_argv_carries_the_proved_machine_line() {
    let argv = qemu_argv(
        Accel::Tcg,
        256,
        Path::new("/work/vmlinuz"),
        Path::new("/work/init.cpio"),
        Path::new("/run/serial.sock"),
        &[],
    );
    let line = argv.join(" ");
    for token in [
        "-M pc,acpi=off",
        "-m 256",
        "-display none",
        "-monitor none",
        "-no-reboot",
        "-accel tcg,thread=multi",
        "-cpu max",
        "-kernel /work/vmlinuz",
        "-initrd /work/init.cpio",
        "-append console=ttyS0 panic=-1 quiet loglevel=0",
        "-serial unix:/run/serial.sock,server=on,wait=off",
    ] {
        assert!(line.contains(token), "missing {token} in {line}");
    }
}

#[test]
fn the_kvm_pair_is_kvm_with_host_cpu() {
    let argv = qemu_argv(
        Accel::Kvm,
        512,
        Path::new("k"),
        Path::new("i"),
        Path::new("s"),
        &["-smp".to_string(), "2".to_string()],
    );
    let line = argv.join(" ");
    assert!(line.contains("-accel kvm"), "{line}");
    assert!(line.contains("-cpu host"), "{line}");
    assert!(line.ends_with("-smp 2"), "caller args append last: {line}");
}

#[test]
fn the_ssh_argv_is_batchmode_with_the_proxy_command() {
    let args = Args {
        kernel: PathBuf::from("k"),
        initramfs: PathBuf::from("i"),
        user_key: PathBuf::from("/keys/u ser"),
        user: "root".to_string(),
        mem: DEFAULT_MEM,
        timeout: Duration::from_secs(600),
        emu_args: Vec::new(),
        command: vec!["echo".to_string(), "hi".to_string()],
    };
    let argv = ssh_argv(
        Path::new("/opt/my bins/proxy"),
        Path::new("/run/serial.sock"),
        &args,
    );
    let line = argv.join(" ");
    for token in [
        "BatchMode=yes",
        "StrictHostKeyChecking=no",
        "UserKnownHostsFile=/dev/null",
        "ConnectTimeout=10",
        "-i /keys/u ser",
        "-l root",
    ] {
        assert!(line.contains(token), "missing {token} in {line}");
    }
    assert!(
        line.contains(
            "ProxyCommand='/opt/my bins/proxy' unix '/run/serial.sock' --hold-for-banner 60"
        ),
        "proxy command quoting: {line}"
    );
    assert!(line.contains(" guest "), "the inert hostname: {line}");
    assert!(
        line.ends_with("guest echo hi"),
        "command rides last with no separator: {line}"
    );
}

#[test]
fn an_empty_command_leaves_the_argv_at_the_hostname() {
    let args = Args {
        kernel: PathBuf::from("k"),
        initramfs: PathBuf::from("i"),
        user_key: PathBuf::from("uk"),
        user: "root".to_string(),
        mem: DEFAULT_MEM,
        timeout: Duration::from_secs(600),
        emu_args: Vec::new(),
        command: Vec::new(),
    };
    let argv = ssh_argv(Path::new("proxy"), Path::new("s"), &args);
    assert_eq!(argv.last().map(String::as_str), Some("guest"));
}

#[test]
fn shell_quoting_survives_spaces_and_quotes() {
    assert_eq!(sh_quote("plain"), "'plain'");
    assert_eq!(sh_quote(""), "''");
    assert_eq!(sh_quote("/opt/my bins/proxy"), "'/opt/my bins/proxy'");
    assert_eq!(sh_quote("o'clock"), "'o'\"'\"'clock'");
    assert_eq!(sh_quote("$(evil)"), "'$(evil)'");
    assert_eq!(sh_quote("back\\slash"), "'back\\slash'");
    assert_eq!(sh_quote("-n"), "'-n'");
}

#[test]
fn the_serial_wait_deadline_is_a_runtime_error() {
    // Deleting the deadline below hangs this test rather than failing
    // it: the arm it guards is the loop's only exit past the emulator
    // check, so this test is the deadline's proof by construction.
    // The message it prints is pinned by
    // run_phase_refusals_say_different_things below.
    let dir = std::env::temp_dir().join(format!("podvm-machine-ssh-wait-{}", std::process::id()));
    let sock = dir.join("serial.sock");
    let log = dir.join("qemu.log");
    // No listener, no emulator behind it: use a live-but-idle child
    // as the polled process so the wait exercises the deadline arm.
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let end = Instant::now() + Duration::from_secs(1);
    let code = wait_serial(&sock, end, &mut child, &log).unwrap_err();
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(code, EXIT_RUNTIME_ERROR);
}

#[test]
fn run_phase_refusals_say_different_things() {
    // All three run-phase failures share EXIT_RUNTIME_ERROR, so the
    // sentence is the only attribution the bounded drive can grep.
    // Merging any two sentences would leave that drive owning nothing.
    assert!(SERIAL_DEADLINE_MSG.contains("before the deadline"));
    assert!(SESSION_DEADLINE_MSG.contains("did not answer in time"));
    assert!(EMULATOR_EXITED_MSG.contains("exited at startup"));
    assert_ne!(SERIAL_DEADLINE_MSG, SESSION_DEADLINE_MSG);
    assert_ne!(SERIAL_DEADLINE_MSG, EMULATOR_EXITED_MSG);
    assert_ne!(SESSION_DEADLINE_MSG, EMULATOR_EXITED_MSG);
}

#[test]
fn the_serial_wait_fails_fast_where_the_emulator_died() {
    let dir = std::env::temp_dir().join(format!("podvm-machine-ssh-dead-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("serial.sock");
    let log = dir.join("qemu.log");
    std::fs::write(&log, "qemu said no\n").unwrap();
    // An already-exited child with no listener: only the fast-failure
    // arm can answer, and it answers at once rather than at the end.
    let mut child = std::process::Command::new("/bin/true").spawn().unwrap();
    let _ = child.wait();
    let start = Instant::now();
    let code = wait_serial(&sock, start + Duration::from_secs(60), &mut child, &log).unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_eq!(code, EXIT_RUNTIME_ERROR);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_serial_wait_accepts_a_bound_socket() {
    let dir = std::env::temp_dir().join(format!("podvm-machine-ssh-accept-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("serial.sock");
    let log = dir.join("qemu.log");
    let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    assert!(wait_serial(&sock, end, &mut child, &log).is_ok());
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
