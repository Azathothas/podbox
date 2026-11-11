//! TODO/cli.md T-1650. The capability kind of `--strict`.

use super::*;
use crate::complete::{classify_strict, strict_refusal, Ask, StrictCtx, StrictKind};

fn words(w: &[&str]) -> Vec<String> {
    w.iter().map(|s| s.to_string()).collect()
}

/// A 64-bit little-endian ET_DYN head with one program header: `PT_LOAD`
/// for a static program, `PT_INTERP` for a dynamic one.
fn elf(ptype: u32) -> Vec<u8> {
    let mut b = vec![0u8; 64 + 56];
    b[0..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[16..18].copy_from_slice(&3u16.to_le_bytes());
    b[32..40].copy_from_slice(&64u64.to_le_bytes());
    b[54..56].copy_from_slice(&56u16.to_le_bytes());
    b[56..58].copy_from_slice(&1u16.to_le_bytes());
    b[64..68].copy_from_slice(&ptype.to_le_bytes());
    b
}

/// The commands of a payload: the argv, each simple command of a `-c`
/// string, and the arguments of a followed script.
#[test]
fn the_commands_of_a_payload_are_read_from_its_argv() {
    let c = commands(
        &words(&["sh", "-c", "/bin/ls / && echo 'a b' | cat; FOO=1 x"]),
        false,
    );
    assert_eq!(
        c,
        vec![
            words(&["sh", "-c", "/bin/ls / && echo 'a b' | cat; FOO=1 x"]),
            words(&["/bin/ls", "/"]),
            words(&["echo", "a b"]),
            words(&["cat"]),
            words(&["FOO=1", "x"]),
        ]
    );
    // Options before -c, and busybox as the shell.
    let c = commands(&words(&["/bin/busybox", "sh", "-ec", "npm ci"]), false);
    assert_eq!(c.last().unwrap(), &words(&["npm", "ci"]));
    // A shell with no -c runs a script file: nothing more is read.
    assert_eq!(commands(&words(&["sh", "run.sh"]), false).len(), 1);
    // An entrypoint script runs its arguments.
    let c = commands(
        &words(&["docker-entrypoint.sh", "sh", "-c", "npm test"]),
        true,
    );
    assert_eq!(c.last().unwrap(), &words(&["npm", "test"]));
    assert_eq!(
        commands(&words(&["docker-entrypoint.sh", "x"]), false).len(),
        1
    );
}

/// A head tells static only where the whole program header table is in
/// it; a truncated table or another format cannot tell.
#[test]
fn a_static_head_is_told_from_its_program_headers() {
    assert_eq!(static_head(&elf(1)), Some(true));
    assert_eq!(static_head(&elf(3)), Some(false));
    assert_eq!(static_head(&elf(1)[..100]), None);
    assert_eq!(static_head(b"#!/bin/sh\n"), None);
    let mut exec_type = elf(1);
    exec_type[16] = 4; // ET_CORE
    assert_eq!(static_head(&exec_type), None);
}

/// The gaps of the loader family for one payload: a static program that
/// a `-c` string runs hits T-1724, and a dynamic one does not.
#[test]
fn the_loader_gaps_name_their_entries_and_their_hits() {
    let d = std::env::temp_dir().join(format!("podvm-gaps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/stat"), elf(1)).unwrap();
    std::fs::write(d.join("bin/dyn"), elf(3)).unwrap();
    let root = d.to_string_lossy().to_string();
    let path = vec!["/bin".to_string()];
    let gaps = loader_gaps(
        &root,
        &words(&["sh", "-c", "/bin/dyn; stat x"]),
        false,
        &path,
    );
    let entries: Vec<_> = gaps.iter().map(|g| g.entry).collect();
    assert_eq!(entries, vec!["T-1724"]);
    assert!(
        gaps[0].hit.as_deref().unwrap().contains("`stat`"),
        "{gaps:?}"
    );
    let gaps = loader_gaps(&root, &words(&["sh", "-c", "/bin/dyn"]), false, &path);
    assert!(gaps.iter().all(|g| g.hit.is_none()), "{gaps:?}");
    // The payload itself is decided by the entry: no hit from argv[0].
    let gaps = loader_gaps(&root, &words(&["/bin/stat"]), false, &path);
    assert!(gaps[0].hit.is_none());
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/cli.md T-1650, the Prove clause: `classify_strict` gives each gap
/// the capability kind; `--strict` refuses a gap that the payload hits
/// and warns on the others; `--strict=all` refuses each.
#[test]
fn strict_refuses_a_capability_gap_that_the_payload_hits() {
    let userland = podvm_probe::select::Rung::Userland.word();
    let report = podvm_complete::Report::default();
    let gap = |entry, hit: Option<&str>| Gap {
        entry,
        text: "a gap (TODO/x.md T-0000)",
        hit: hit.map(str::to_string),
    };
    let warned = StrictCtx {
        gaps: vec![gap("T-1652", None)],
        ..StrictCtx::default()
    };
    let strict = Ask {
        strict: true,
        ..Ask::default()
    };
    let reasons = classify_strict("run", &strict, userland, &report, &warned);
    assert!(reasons.contains(&(
        StrictKind::Capability { hit: false },
        "a gap (TODO/x.md T-0000)".to_string()
    )));
    let mut out = Vec::new();
    assert!(strict_refusal("run", &strict, userland, &report, &mut out, &warned).is_ok());
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("--strict warning [capability]: a gap"), "{s}");
    // A hit refuses at 125, naming the gap, why, and a rung.
    let hit = StrictCtx {
        gaps: vec![gap(
            "T-1652",
            Some("`stat` is a statically linked program of the image"),
        )],
        ..StrictCtx::default()
    };
    let mut out = Vec::new();
    let e = strict_refusal("run", &strict, userland, &report, &mut out, &hit).unwrap_err();
    assert_eq!(e, podvm_image::error::EXIT_RUNTIME_ERROR);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("  - [capability] a gap"), "{s}");
    assert!(s.contains("`stat` is a statically linked program"), "{s}");
    assert!(s.contains("chroot rung runs this command"), "{s}");
    assert!(s.contains("T-1651, T-1724"), "{s}");
    // `--strict=all` refuses the warned gap too.
    let all = Ask {
        strict: true,
        strict_all: true,
        ..Ask::default()
    };
    let mut out = Vec::new();
    assert!(strict_refusal("run", &all, userland, &report, &mut out, &warned).is_err());
    assert!(String::from_utf8(out).unwrap().contains("[capability]"));
}

/// A gap leaves the list in the change that closes its entry: each listed
/// entry is open, partial or blocked in `TODO/INDEX.md`, and the entries
/// that closed their gaps (T-1643, T-1645, T-1697) are not listed.
#[test]
fn each_listed_gap_names_an_entry_that_is_not_done() {
    let index = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../TODO/INDEX.md");
    let text = std::fs::read_to_string(&index).unwrap();
    for g in LOADER_GAPS {
        let row = text
            .lines()
            .find(|l| l.starts_with(&format!("| [{}](", g.entry)))
            .unwrap_or_else(|| panic!("{} has no row in TODO/INDEX.md", g.entry));
        let status = row.split('|').nth(4).unwrap_or("").trim();
        assert!(
            matches!(status, "open" | "partial" | "blocked"),
            "{} is {status}: remove its gap from LOADER_GAPS",
            g.entry
        );
        assert!(g.text.contains(g.entry), "{}: {}", g.entry, g.text);
    }
    for closed in ["T-1643", "T-1645", "T-1697"] {
        assert!(LOADER_GAPS.iter().all(|g| g.entry != closed), "{closed}");
    }
}
