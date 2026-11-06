use super::*;

/// ⭐ **The assertion that would have caught two glibc objects**, which is
/// the failure `scripts/build-interpose.sh` records as its own reason for
/// existing: before it asserted `DT_NEEDED`, the build exited 0 having
/// produced the wrong object twice.
///
/// ⚠ It reads the bytes rather than running `readelf`, so it holds inside
/// `cargo test` with no external tool. The dynamic string table carries the
/// `DT_NEEDED` names verbatim, and the two spellings are the discriminator:
/// musl's libc declares no SONAME, so its object names `libc.so`, and a
/// glibc one names `libc.so.6`.
///
/// ⛔ **It SKIPS rather than fails where nothing is embedded.** A machine
/// with no zig builds podvm and must still pass its tests; T-0706's
/// channel is what reports the absence to a caller at run time.
#[test]
fn each_embedded_object_names_its_own_libc() {
    fn has(bytes: &[u8], needle: &str) -> bool {
        bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
    }
    if let Some(gnu) = object(Libc::Gnu) {
        assert!(
            has(gnu, "libc.so.6"),
            "the glibc object does not name libc.so.6, so it is not glibc-linked"
        );
    }
    if let Some(musl) = object(Libc::Musl) {
        assert!(has(musl, "libc.so"), "the musl object names no libc at all");
        // ⚠ An exact spelling, because `libc.so.6` CONTAINS `libc.so`. The
        // same trap `scripts/build-interpose.sh` names, one layer up.
        assert!(
            !has(musl, "libc.so.6"),
            "the musl object names libc.so.6, so it is a SECOND GLIBC \
                 OBJECT and the whole one-object-per-libc requirement is \
                 inverted (TODO/interpose.md T-0702)"
        );
    }
}

/// ⚠ The two objects are different artefacts. Embedding the same bytes
/// twice would pass the check above for the glibc half and read as success.
#[test]
fn the_two_embedded_objects_are_not_the_same_bytes() {
    if let (Some(gnu), Some(musl)) = (object(Libc::Gnu), object(Libc::Musl)) {
        assert_ne!(gnu, musl, "one object is embedded under both names");
    }
}

/// ⛔ The line a caller reads has to say which state this binary is in,
/// including the state where it carries nothing.
#[test]
fn the_carried_line_names_the_state_it_is_in() {
    let line = carried();
    assert!(line.starts_with("interposer: "), "{line}");
    match (object(Libc::Gnu).is_some(), object(Libc::Musl).is_some()) {
        (true, true) => assert!(line.contains("glibc and musl"), "{line}"),
        (false, false) => assert!(line.contains("declined"), "{line}"),
        _ => assert!(line.contains("declined"), "{line}"),
    }
}

// ------------------------------------------------- T-0706 classification

use podvm_enter::abi::Elf;

const X86_64: u16 = 62;
const AARCH64: u16 = 183;

fn elf(interp: Option<&str>, sections: &[&str], machine: u16) -> Elf {
    Elf {
        path: "/payload".to_string(),
        machine,
        interp: interp.map(str::to_string),
        needed: Vec::new(),
        soname: None,
        sections: sections.iter().map(|s| (*s).to_string()).collect(),
        defined: Vec::new(),
        imported: Vec::new(),
        declares: Vec::new(),
    }
}

fn no_objects() -> Parsed {
    let empty = || Slot {
        bytes: None,
        elf: None,
    };
    Parsed {
        gnu: empty(),
        musl: empty(),
    }
}

fn declined_name(r: Reach) -> String {
    let Reach::Declined(why) = r else {
        panic!("a decline was expected and the payload was admitted");
    };
    why
}

/// ⭐ Row 2 of T-0706's table: no `PT_INTERP` means no loader, and nothing
/// reads the variable. The absence of an object cannot matter here, so no
/// embedded bytes are needed and none are read.
#[test]
fn a_static_payload_is_declined_by_name() {
    let why = declined_name(classify_elf(
        "/podvm",
        &elf(None, &[], X86_64),
        "/rootfs",
        &no_objects(),
    ));
    assert!(why.contains("statically linked"), "{why}");
    assert!(why.contains("PT_INTERP"), "{why}");
}

/// ⭐ Row 3: Go markers outrank a present interpreter, because a Go
/// payload is unreachable regardless of linkage. T-1209 refuses an
/// invented image reference, so this arm is proved here and not by a run.
#[test]
fn go_markers_outrank_a_present_interpreter() {
    for marker in [
        ".note.go.buildid",
        ".gosymtab",
        ".gopclntab",
        ".go.buildinfo",
    ] {
        let why = declined_name(classify_elf(
            "/server",
            &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[marker], X86_64),
            "/rootfs",
            &no_objects(),
        ));
        assert!(why.contains("Go"), "{marker}: {why}");
    }
}

/// An interpreter podvm does not recognise gets a decline, never a
/// guess at an object.
#[test]
fn an_unknown_interpreter_is_declined() {
    let why = declined_name(classify_elf(
        "/p",
        &elf(Some("/opt/weird/loader"), &[], X86_64),
        "/rootfs",
        &no_objects(),
    ));
    assert!(why.contains("no libc"), "{why}");
}

/// A binary built without the objects declines naming what is missing,
/// rather than preloading nothing.
#[test]
fn a_binary_without_objects_declines_naming_the_libc() {
    let why = declined_name(classify_elf(
        "/bin/sh",
        &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
        "/rootfs",
        &no_objects(),
    ));
    assert!(why.contains("glibc"), "{why}");
    assert!(why.contains("build-interpose.sh"), "{why}");
}

/// ⛔ A foreign payload under binfmt gets a decline, never an object its
/// loader cannot read.
#[test]
fn another_architecture_is_declined() {
    let objs = Parsed {
        gnu: Slot {
            bytes: Some(b"present but unparsed"),
            elf: Some(elf(None, &[], AARCH64)),
        },
        musl: Slot {
            bytes: None,
            elf: None,
        },
    };
    let why = declined_name(classify_elf(
        "/bin/sh",
        &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
        "/rootfs",
        &objs,
    ));
    assert!(why.contains("0x"), "{why}");
}

/// A present object that does not parse is refused with that reason,
/// rather than preloaded unchecked or reported as absent.
#[test]
fn an_unparsable_object_is_refused_as_unparsable() {
    let objs = Parsed {
        gnu: Slot {
            bytes: Some(b"present but unparsed"),
            elf: None,
        },
        musl: Slot {
            bytes: None,
            elf: None,
        },
    };
    let why = declined_name(classify_elf(
        "/bin/sh",
        &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
        "/rootfs",
        &objs,
    ));
    assert!(why.contains("does not parse"), "{why}");
}

/// A payload that is not an ELF file is declined with the reader's
/// reason. The tier selects on `PT_INTERP`, which only ELF carries, and
/// declining is the safe direction: a wrong preload fails the payload.
#[test]
fn a_script_is_declined_with_the_readers_reason() {
    let d = std::env::temp_dir().join(format!("podvm-cls-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/tool.sh"), b"#!/bin/sh\necho hi\n").unwrap();
    let root = d.to_string_lossy().to_string();
    let why = declined_name(classify(&root, "tool.sh", &["/bin".to_string()]));
    assert!(why.contains("not an ELF file"), "{why}");
    // ⛔ And the decline wrote nothing into the image.
    assert!(!d.join(".podvm").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// A real ELF file is resolved along `path_dirs` exactly as production
/// passes them: with leading slashes, several entries, the hit not
/// first. This mirrors a `run` payload of a bare name.
#[test]
fn resolve_finds_a_real_elf_along_path_dirs() {
    let me = std::env::current_exe().unwrap();
    let d = std::env::temp_dir().join(format!("podvm-rsv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::copy(&me, d.join("bin/prog")).unwrap();
    let root = d.to_string_lossy().to_string();
    let dirs = ["/usr/local/sbin", "/usr/bin", "/bin"]
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    let got = resolve(&root, "prog", &dirs).unwrap();
    assert!(got.ends_with("bin/prog"), "{got}");
    // ⚠ And the absolute branch against the same tree.
    let got = resolve(&root, "/bin/prog", &dirs).unwrap();
    assert!(got.ends_with("bin/prog"), "{got}");
    let _ = std::fs::remove_dir_all(&d);
}

/// A payload the image does not hold is declined, and the later `execve`
/// still reports it as not found: the decline does not swallow the 127.
#[test]
fn a_missing_payload_is_declined() {
    let d = std::env::temp_dir().join(format!("podvm-clm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let root = d.to_string_lossy().to_string();
    let why = declined_name(classify(&root, "absent", &["/bin".to_string()]));
    assert!(why.contains("names no file"), "{why}");
    let _ = std::fs::remove_dir_all(&d);
}

/// ⭐ The alpine shape, and the defect that declined every dynamic
/// payload: `/bin/sh` points at the absolute `/bin/busybox`, which names
/// the host's file from outside and the image's from inside. The link
/// resolves under the rootfs, to the file the guest kernel would run.
#[test]
fn an_absolute_link_resolves_under_the_rootfs() {
    let me = std::env::current_exe().unwrap();
    let d = std::env::temp_dir().join(format!("podvm-cla-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::copy(&me, d.join("bin/busybox")).unwrap();
    std::os::unix::fs::symlink("/bin/busybox", d.join("bin/sh")).unwrap();
    let root = d.to_string_lossy().to_string();
    // ⚠ The answer is the target, not the link: it is the file the guest
    // kernel would run, and reading it answers about that file.
    let got = resolve(&root, "sh", &["/sbin".to_string(), "/bin".to_string()]).unwrap();
    assert!(got.ends_with("bin/busybox"), "{got}");
    let _ = std::fs::remove_dir_all(&d);
}

/// ⛔ `..` that escapes the root is refused rather than followed: the
/// path it names is the host's, and classifying it answers about the
/// wrong machine.
#[test]
fn a_payload_escaping_the_image_is_refused() {
    let d = std::env::temp_dir().join(format!("podvm-clx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::os::unix::fs::symlink("../../outside", d.join("bin/evil")).unwrap();
    let root = d.to_string_lossy().to_string();
    let why = declined_name(classify(&root, "evil", &["/bin".to_string()]));
    assert!(why.contains("escapes the image"), "{why}");
    let _ = std::fs::remove_dir_all(&d);
}

/// A link loop is refused with that reason, rather than followed until
/// the process runs out of anything.
#[test]
fn a_link_loop_is_refused() {
    let d = std::env::temp_dir().join(format!("podvm-cll-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::os::unix::fs::symlink("b", d.join("bin/a")).unwrap();
    std::os::unix::fs::symlink("a", d.join("bin/b")).unwrap();
    let root = d.to_string_lossy().to_string();
    let why = declined_name(classify(&root, "a", &["/bin".to_string()]));
    assert!(why.contains("too many levels of symlinks"), "{why}");
    let _ = std::fs::remove_dir_all(&d);
}

/// ⭐ The channel T-0706's `Prove` greps for. A decline that does not
/// carry this line passes a tree where nothing is declined.
#[test]
fn the_decline_line_carries_the_channel() {
    let mut note = String::new();
    decline(&mut note, "run", "/podvm", "static".to_string());
    assert!(note.contains("interpose: declined"), "{note}");
    assert!(note.contains("/podvm"), "{note}");
}

// ---------------------------------------------------- T-0702 placement

/// The object lands at the guest path, whole, and the staging file is
/// gone. A second placement overwrites rather than stacking.
#[test]
fn placement_writes_the_object_where_the_payload_sees_it() {
    let d = std::env::temp_dir().join(format!("podvm-plc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let root = d.to_string_lossy().to_string();
    place(&root, b"first").unwrap();
    assert_eq!(
        std::fs::read(d.join(".podvm/interpose.so")).unwrap(),
        b"first"
    );
    assert!(!d.join(".podvm/.interpose.so.tmp").exists());
    place(&root, b"second").unwrap();
    assert_eq!(
        std::fs::read(d.join(".podvm/interpose.so")).unwrap(),
        b"second"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Podvm first, so the ownership wall stays cleared whatever else the
/// payload preloads. An absent caller value leaves the path bare.
#[test]
fn a_caller_preload_survives_behind_podvms() {
    assert_eq!(merge_preload(&[]), "/.podvm/interpose.so");
    assert_eq!(
        merge_preload(&["LD_PRELOAD=/mine.so".to_string()]),
        "/.podvm/interpose.so /mine.so"
    );
    assert_eq!(
        merge_preload(&["LD_PRELOAD=".to_string()]),
        "/.podvm/interpose.so"
    );
}

/// ⭐ T-0710: the descriptor grammar the host and the interposer share.
/// Decimal digits only; anything else is no descriptor rather than a guess.
#[test]
fn the_memo_descriptor_parses_as_decimal_or_not_at_all() {
    assert_eq!(memo_fd_of(&["PODVM_MEMO_FD=17".to_string()]), Some(17));
    assert_eq!(memo_fd_of(&[]), None);
    assert_eq!(memo_fd_of(&["PODVM_MEMO_FD=".to_string()]), None);
    assert_eq!(memo_fd_of(&["PODVM_MEMO_FD=nobody".to_string()]), None);
    assert_eq!(memo_fd_of(&["PODVM_MEMO_FD=-1".to_string()]), None);
    // ⚠ The last wins, as every other `*_for` in this tree resolves it:
    // the caller wins over the image.
    assert_eq!(
        memo_fd_of(&[
            "PODVM_MEMO_FD=17".to_string(),
            "PODVM_MEMO_FD=18".to_string()
        ]),
        Some(18)
    );
}

/// ⛔ An entry that would load the tier without a memo refuses rather than
/// starting with no record. A declined tier needs none.
#[test]
fn a_reachable_payload_without_a_memo_is_refused_not_declined() {
    // A missing payload is declined (no memo needed); the refusal arm
    // needs a real rootfs with a real ELF, which this unit does not build.
    // What this pins is the grammar above, which `apply` reads: without it
    // the refusal cannot fire. The end-to-end refusal is asserted by
    // `experiments/105-interpose-ownership.sh` check G's harness, which
    // hands fd 17 where the tier loads.
    assert!(memo_fd_of(&[]).is_none());
    assert_eq!(memo_fd_env(), "PODVM_MEMO_FD=17");
    assert_eq!(MEMO_CHILD_FD, 17);
}
