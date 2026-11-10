use super::*;

#[test]
fn docker_keeps_0_to_124_and_podvm_keeps_125_up() {
    assert_eq!(cap_status(0), Some(0));
    assert_eq!(cap_status(42), Some(42));
    assert_eq!(cap_status(124), Some(124));
    assert_eq!(cap_status(125), None);
    assert_eq!(cap_status(255), None);
    assert_eq!(cap_status(-1), None);
}

/// podvm's names reach `pod-guest`: the per-run directory keeps its
/// `podvm-windows-` name, so `experiments/lib/kvm-guest-base.sh` still finds
/// a survivor, and the mailbox keeps its `podvm` marker and OEM name.
#[test]
fn podvm_gives_its_names_to_the_guest() {
    let s = settings();
    assert_eq!(s.scratch_prefix, "podvm-windows-");
    assert_eq!(s.marker, b"podvm");
    let dir = scratch_dir("names").expect("a scratch directory");
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(name, format!("podvm-windows-{}-names", std::process::id()));
    let m = mailbox(
        agent::AGENT_CMD,
        &Request {
            command: "ver".into(),
            token: "tok".into(),
            timeout: Duration::from_secs(1),
        },
    )
    .expect("a mailbox");
    assert_eq!(m.get(agent::MARKER).as_deref(), Some(&b"podvm"[..]));
    let b = fat16::PART_LBA * fat16::SECTOR;
    assert_eq!(&m.image()[b + 3..b + 11], b"PODVM   ");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `PODVM_WINDOWS_KEEP` keeps the per-run directory.
#[test]
fn podvm_windows_keep_keeps_the_run() {
    let dir = scratch_dir("keep").expect("a scratch directory");
    let plan = pod_guest::Plan {
        emulator: "qemu-system-x86_64".into(),
        accel: Accel::Tcg,
        share: "/usr/share/qemu".into(),
        firmware_code: "/code.fd".into(),
        firmware_vars: dir.join("vars.fd"),
        root: dir.join("run.qcow2"),
        mailbox: dir.join("mailbox.img"),
        serial: dir.join("serial.log"),
        monitor: dir.join("qmp.sock"),
        memory_mib: 512,
        cpus: 1,
        emu_args: Vec::new(),
    };
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::set_var("PODVM_WINDOWS_KEEP", "1") };
    cleanup(&plan);
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODVM_WINDOWS_KEEP") };
    assert!(dir.is_dir(), "PODVM_WINDOWS_KEEP keeps the run");
    cleanup(&plan);
    assert!(!dir.exists(), "without it the run goes");
}
