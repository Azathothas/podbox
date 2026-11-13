//! TODO/machine.md T-1656: the archive cache key, the archive bytes, the
//! accelerator rule and the exit marker parse.

use super::archive::Tools;
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
        &[],
        None,
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
        &[],
        None,
        None,
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

/// `-v HOST:GUEST` on the machine tier: the host tree is in the run
/// archive under GUEST, with each directory of GUEST before it.
#[test]
fn a_volume_is_a_copy_at_its_guest_path() {
    let d = std::env::temp_dir().join(format!("podvm-t1656-vol-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("proj/sub")).unwrap();
    std::fs::write(d.join("proj/sub/f.txt"), b"in-volume").unwrap();
    let a = run_archive(
        "#!/bin/sh\n",
        None,
        &[(d.join("proj"), "/srv/workspace".to_string())],
        None,
    )
    .unwrap();
    let text = String::from_utf8_lossy(&a);
    for want in [
        "srv\0",
        "srv/workspace\0",
        "srv/workspace/sub\0",
        "srv/workspace/sub/f.txt\0",
    ] {
        assert!(text.contains(want), "{want:?}");
    }
    assert_eq!(
        cpio::member(&a, "workspace/sub/f.txt"),
        Some(&b"in-volume"[..])
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The second serial port: a socket that QEMU holds the guest for, and the
/// tty's `\r` taken out of the bytes.
#[test]
fn stderr_has_its_own_port() {
    let sock = stderr::socket_path("0123456789abcdef0123");
    assert!(sock
        .to_string_lossy()
        .ends_with("podvm-guest-0123456789abcdef.err"));
    let a = stderr::argv(&sock).join(" ");
    assert!(a.contains("server=on,wait=on") && a.ends_with("-serial chardev:podvmerr"));
    assert_eq!(stderr::strip_cr(b"e1\r\ne2\r\n"), b"e1\ne2\n");
    let s = init::script("n", &["true".into()], &[], "/", false, &[], None, None);
    assert!(s.contains("cat /podvm-err >/dev/ttyS1"));
}

/// TODO/machine.md T-1728: the command's stdout and stderr are FIFOs, not
/// the serial terminals, and the script waits for both copies before it
/// writes the exit marker.
#[test]
fn the_command_writes_to_fifos_not_to_a_terminal() {
    let s = init::script("n", &["true".into()], &[], "/", false, &[], None, None);
    assert!(s.contains("mkfifo /podvm-out /podvm-err"), "{s}");
    assert!(s.contains("\"$@\" >/podvm-out 2>/podvm-err"), "{s}");
    let wait = s
        .find("wait $o $e")
        .expect("the script waits for the copies");
    assert!(wait < s.find("PODVM-EXIT-n").unwrap(), "{s}");
}

/// stdin of the guest: the bytes ride in the run archive, and the script
/// reads them, or `/dev/null` where there are none.
#[test]
fn stdin_rides_in_the_run_archive() {
    let a = run_archive("#!/bin/sh\n", None, &[], Some(b"in-bytes")).unwrap();
    assert_eq!(cpio::member(&a, "podvm/stdin"), Some(&b"in-bytes"[..]));
    let s = init::script("n", &["cat".into()], &[], "/", false, &[], None, None);
    assert!(s.contains("exec 0</podvm/stdin") && s.contains("exec 0</dev/null"));
}

/// A hard link of the tree is one inode of the archive with the data once:
/// busybox:musl (one binary under about 400 names) filled 500 MB before,
/// and the guest ran out of memory while it unpacked them.
#[test]
fn a_hard_link_of_the_tree_carries_its_data_once() {
    let root = std::env::temp_dir().join(format!("podvm-cpio-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::write(root.join("bin/busybox"), vec![7u8; 100_000]).unwrap();
    for n in ["ls", "sh", "wget"] {
        std::fs::hard_link(root.join("bin/busybox"), root.join("bin").join(n)).unwrap();
    }
    let mut w = cpio::Newc::new(Vec::new());
    w.tree(&root).unwrap();
    let a = w.finish().unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert!(a.len() < 110_000, "the data went in {} bytes", a.len());
    // Each name of the link has the inode of the first, and a link count
    // above one, so the kernel links it.
    let heads: Vec<&[u8]> = a
        .windows(6)
        .enumerate()
        .filter(|(_, w)| *w == b"070701")
        .map(|(i, _)| &a[i..i + 110])
        .collect();
    let field = |h: &[u8], n: usize| {
        u32::from_str_radix(std::str::from_utf8(&h[6 + 8 * n..14 + 8 * n]).unwrap(), 16).unwrap()
    };
    let regs: Vec<&&[u8]> = heads
        .iter()
        .filter(|h| field(h, 1) & 0o170_000 == cpio::REG)
        .collect();
    assert_eq!(regs.len(), 4);
    assert!(regs
        .iter()
        .all(|h| field(h, 0) == field(regs[0], 0) && field(h, 4) == 2));
    assert_eq!(regs.iter().filter(|h| field(h, 6) > 0).count(), 1);
}

/// TODO/machine.md T-1723: each volume is a tag of the emulator, mounted
/// by `podvm guest-9p` before the `cd`; a failed mount ends the run with
/// 125 before the command.
#[test]
fn each_volume_is_a_9p_tag_mounted_before_the_working_directory() {
    let vols = super::vol::vols(&[("/data".into(), false), ("/cfg dir".into(), true)]);
    assert_eq!(vols[0].tag, "podvm0");
    assert_eq!(
        super::vol::virtfs(std::path::Path::new("/h/d"), &vols[0]),
        "local,path=/h/d,mount_tag=podvm0,security_model=none"
    );
    assert_eq!(
        super::vol::virtfs(std::path::Path::new("/h/c"), &vols[1]),
        "local,path=/h/c,mount_tag=podvm1,security_model=none,readonly=on"
    );
    let s = init::script("n", &["true".into()], &[], "/data", true, &vols, None, None);
    let mount = s
        .find("/podvm/podvm guest-9p 'podvm0=/data' 'podvm1=/cfg dir:ro'")
        .expect(&s);
    assert!(mount < s.find("cd '/data'").unwrap(), "{s}");
    assert!(s.contains("echo PODVM-EXIT-n 125"), "{s}");
    assert_eq!(
        super::vol::parse("podvm1=/cfg dir:ro"),
        Some(vols[1].clone())
    );
    assert_eq!(super::vol::parse("podvm0=relative"), None);
}

/// TODO/machine.md T-1723: with `-t` the command runs through
/// `podvm guest-tty` with the size of the host terminal, and no FIFO.
#[test]
fn a_terminal_runs_the_command_through_guest_tty() {
    let s = init::script(
        "n",
        &["sh".into()],
        &[],
        "/",
        true,
        &[],
        Some("40x120"),
        None,
    );
    assert!(
        s.contains("/podvm/podvm guest-tty '40x120' -- \"$@\""),
        "{s}"
    );
    assert!(s.contains("echo PODVM-EXIT-n $rc"), "{s}");
    assert!(!s.contains("mkfifo"), "{s}");
    let a = super::tty::argv(std::path::Path::new("/r/tty.sock"));
    assert!(a.contains(&"virtconsole,chardev=podvmtty,name=podvm.tty".to_string()));
    assert!(a
        .iter()
        .any(|x| x.contains("path=/r/tty.sock,server=on,wait=on")));
}

/// TODO/machine.md T-1776 (PodUtils/podrunner#81): each entry of the image
/// has the owner and the group of the sidecar; an entry it does not name is
/// root's.
#[test]
fn the_base_archive_keeps_the_owners_of_the_image() {
    let d = std::env::temp_dir().join(format!("podvm-t1776-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let root = d.join("rootfs");
    std::fs::create_dir_all(root.join("home/runner")).unwrap();
    std::fs::write(root.join("home/runner/run.sh"), b"#!/bin/sh\n").unwrap();
    std::fs::write(
        d.join(".meta.jsonl"),
        "{\"path\":\"home/runner\",\"uid\":1001,\"gid\":1001,\"mode\":\"0750\"}\n\
         {\"path\":\"home/runner/run.sh\",\"uid\":1001,\"gid\":121,\"mode\":\"0755\"}\nnot json\n",
    )
    .unwrap();
    let mut w = cpio::Newc::new(Vec::new());
    w.owners_from(&d.join(".meta.jsonl")).unwrap();
    w.tree(&root).unwrap();
    let a = w.finish().unwrap();
    // The ids of the header that precedes `name`: the header is 110 bytes,
    // and the uid and the gid are its fields 3 and 4 (8 hex digits each).
    let ids = |name: &str| {
        let n = format!("{name}\0");
        let at = a.windows(n.len()).position(|w| w == n.as_bytes()).unwrap();
        let h = &a[at - 110..at];
        let hex = |r: std::ops::Range<usize>| {
            u32::from_str_radix(std::str::from_utf8(&h[r]).unwrap(), 16).unwrap()
        };
        (hex(22..30), hex(30..38))
    };
    assert_eq!(ids("home/runner"), (1001, 1001));
    assert_eq!(ids("home/runner/run.sh"), (1001, 121));
    assert_eq!(ids("home"), (0, 0), "a name with no row is root's");
    let _ = std::fs::remove_dir_all(&d);
}
