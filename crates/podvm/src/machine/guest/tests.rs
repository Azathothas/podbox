//! TODO/machine.md T-1656: the archive cache key, the archive bytes, the
//! accelerator rule and the exit marker parse.

use super::init::{Console, Line};
use super::*;

#[test]
fn the_base_archive_is_keyed_by_the_digest_and_the_format() {
    let root = Path::new("/store");
    let p = base_path(root, "sha256:abc123");
    assert_eq!(
        p,
        Path::new(&format!("/store/machine/base-abc123-v{BASE_FORMAT}.cpio"))
    );
    assert_ne!(p, base_path(root, "sha256:abc124"));
    let d = std::env::temp_dir().join(format!("podvm-t1656-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("root/bin")).unwrap();
    std::fs::write(d.join("root/bin/sh"), b"#!sh").unwrap();
    std::os::unix::fs::symlink("sh", d.join("root/bin/ash")).unwrap();
    let (a, cached) = base_archive(&d, "sha256:ff", &d.join("root")).unwrap();
    assert!(!cached);
    let bytes = std::fs::read(&a).unwrap();
    let (b, cached) = base_archive(&d, "sha256:ff", &d.join("root")).unwrap();
    assert!(cached, "a second run reuses the archive");
    assert_eq!(a, b);
    // newc magic, each name, the link target, and the trailer.
    assert!(bytes.starts_with(b"070701"));
    let text = String::from_utf8_lossy(&bytes);
    for want in ["bin\0", "bin/ash\0", "bin/sh\0", "TRAILER!!!\0"] {
        assert!(text.contains(want), "{want:?}");
    }
    assert_eq!(bytes.len() % 4, 0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_run_archive_carries_init_and_the_console_nodes() {
    let a = run_archive(
        "#!/bin/sh\necho hi\n",
        Some(&Tools {
            podvm: b"ELF".to_vec(),
            modules: vec![("virtio_net", b"KO".to_vec())],
        }),
    )
    .unwrap();
    assert_eq!(cpio::member(&a, "podvm/podvm"), Some(&b"ELF"[..]));
    assert_eq!(cpio::member(&a, "/virtio_net.ko"), Some(&b"KO"[..]));
    let text = String::from_utf8_lossy(&a);
    assert!(text.contains("dev/console\0") && text.contains("dev/ttyS0\0"));
    assert!(text.contains("podvm-init\0") && text.contains("echo hi"));
    // The console node: mode 020600, rdev 5:1.
    let at = text.find("dev/console").unwrap() - 110;
    let head = &text[at..at + 110];
    assert_eq!(&head[14..22], "00002180");
    assert_eq!(&head[78..86], "00000005");
    assert_eq!(&head[86..94], "00000001");
}

#[test]
fn kvm_asked_and_closed_refuses_and_never_falls_back() {
    assert!(matches!(accel(None, true), Ok(Accel::Kvm)));
    assert!(matches!(accel(None, false), Ok(Accel::Tcg)));
    assert!(matches!(accel(Some("kvm"), true), Ok(Accel::Kvm)));
    let e = accel(Some("kvm"), false).unwrap_err();
    assert!(e.contains("does not fall back to TCG"), "{e}");
    assert!(matches!(accel(Some("tcg"), true), Ok(Accel::Tcg)));
    assert!(accel(Some("xen"), true).is_err());
}

#[test]
fn the_exit_marker_needs_the_nonce_of_the_run() {
    let mut c = Console::new("n1");
    assert_eq!(c.line("[    0.1] Linux version"), Line::Before);
    // A marker of another run before the ready marker is noise.
    assert_eq!(c.line("PODVM-EXIT-n1 3"), Line::Before);
    assert_eq!(c.line("PODVM-READY-n1\r"), Line::Ready);
    assert_eq!(c.line("hello\r"), Line::Out("hello"));
    // A forged marker without the nonce is output.
    assert_eq!(c.line("PODVM-EXIT-xx 9"), Line::Out("PODVM-EXIT-xx 9"));
    assert_eq!(c.line("tailPODVM-EXIT-n1 7\r"), Line::Exit(7, "tail"));
}

#[test]
fn the_init_script_quotes_each_word() {
    let s = init::script(
        "n",
        &["sh".into(), "-c".into(), "echo 'a b'; exit 7".into()],
        &["A=x y".into(), "1BAD-=z".into()],
        "/w d",
        true,
    );
    assert!(s.contains("/podvm/podvm guest-net"));
    assert!(
        s.contains("set -- 'sh' '-c' 'echo '\\''a b'\\''; exit 7'"),
        "{s}"
    );
    assert!(s.contains("export A='x y'"));
    assert!(!s.contains("1BAD"));
    assert!(s.contains("cd '/w d' || exit 125"));
    assert!(s.contains("echo PODVM-EXIT-n $rc"));
}

#[test]
fn the_tar_reader_finds_a_member() {
    let mut t = Vec::new();
    for (name, data) in [
        ("./boot/other", &b"x"[..]),
        ("./boot/vmlinuz-virt", &b"KERNEL"[..]),
    ] {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        let size = format!("{:011o}\0", data.len());
        h[124..136].copy_from_slice(size.as_bytes());
        h[156] = b'0';
        t.extend_from_slice(&h);
        let mut d = data.to_vec();
        d.resize(512, 0);
        t.extend_from_slice(&d);
    }
    t.extend_from_slice(&[0u8; 1024]);
    let got = kernel::tar_members(&t[..], &["boot/vmlinuz-virt", "boot/absent"]).unwrap();
    assert_eq!(got[0].as_deref(), Some(&b"KERNEL"[..]));
    assert_eq!(got[1], None);
}

#[test]
fn the_emulator_line_names_the_accelerator_and_the_init() {
    let v = qemu_argv(
        Accel::Tcg,
        512 << 20,
        Path::new("/k"),
        Path::new("/i"),
        &["-x".into()],
    );
    let line = v.join(" ");
    assert!(line.contains("-accel tcg,thread=multi"));
    assert!(line.contains("-m 512M"));
    assert!(line.contains("rdinit=/podvm-init"));
    assert!(line.contains("-no-reboot"));
    assert!(line.ends_with("-x"));
}
